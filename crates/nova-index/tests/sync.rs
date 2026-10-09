//! The sync step (spec
//! `docs/superpowers/specs/2026-10-09-phase-3-3b-index-and-publishing-design.md`
//! §4.4, §4.6, §5.3), against a local index.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use nova_diagnostics::FileDb;
use nova_index::{
    index_path, pack, reader_for, sync, Index, Line, LineDep, SyncError, SyncRequest, Synced,
};
use nova_pm::Unlock;
use semver::{Version, VersionReq};

/// A fresh, empty directory with a fixed name, so each run replaces the last.
fn fresh(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("nova-index-sync-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn write(path: &Path, bytes: &[u8]) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, bytes).unwrap();
}

fn manifest(name: &str, version: &str, extra: &str) -> String {
    format!("[package]\nname = \"{name}\"\nversion = \"{version}\"\nedition = \"2026\"\n{extra}")
}

/// A local index in `dir/index`, and a registry in `dir/home/registry`.
struct Fixture {
    dir: PathBuf,
    index_dir: PathBuf,
    registry: PathBuf,
}

impl Fixture {
    fn new(name: &str) -> Fixture {
        let dir = fresh(name);
        let index_dir = dir.join("index");
        write(
            &index_dir.join("config.json"),
            br#"{"dl":"dl/{name}-{version}.nova-pkg"}"#,
        );
        Fixture {
            registry: dir.join("home").join("registry"),
            index_dir,
            dir,
        }
    }

    /// Publish the library `name` `version` with `deps` by hand: its
    /// tarball where `dl` says, and its line appended.
    fn publish(&self, name: &str, version: &str, deps: &[(&str, &str)]) {
        let source = self.dir.join("sources").join(format!("{name}-{version}"));
        let mut extra = String::new();
        if !deps.is_empty() {
            extra.push_str("\n[dependencies]\n");
            for (dep, req) in deps {
                extra.push_str(&format!("{dep} = \"{req}\"\n"));
            }
        }
        write(
            &source.join("nova.toml"),
            manifest(name, version, &extra).as_bytes(),
        );
        write(
            &source.join("src").join("lib.nova"),
            b"pub fn one() -> Int {\n    1\n}\n",
        );
        let packed = pack(&source, name, &Version::parse(version).unwrap()).unwrap();
        write(
            &self
                .index_dir
                .join("dl")
                .join(format!("{name}-{version}.nova-pkg")),
            &packed.bytes,
        );
        let line = Line {
            name: name.into(),
            vers: version.into(),
            deps: deps
                .iter()
                .map(|(dep, req)| LineDep {
                    name: dep.to_string(),
                    req: VersionReq::parse(req).unwrap().to_string(),
                })
                .collect(),
            cksum: packed.checksum,
            v: 1,
        };
        let file = self.index_dir.join(index_path(name));
        let mut text = std::fs::read_to_string(&file).unwrap_or_default();
        text.push_str(&line.to_json());
        text.push('\n');
        write(&file, text.as_bytes());
    }

    fn index(&self) -> Index {
        Index::new(self.index_dir.to_str().unwrap()).unwrap()
    }

    /// The program `app` in `dir/app`, with `extra` appended to its manifest.
    fn app(&self, extra: &str) -> PathBuf {
        let app = self.dir.join("app");
        write(
            &app.join("nova.toml"),
            manifest("app", "0.1.0", extra).as_bytes(),
        );
        write(&app.join("src").join("main.nova"), b"fn main() {}\n");
        app
    }

    fn sync_with(
        &self,
        root: &Path,
        manifest: Option<&str>,
        dev: bool,
        unlock: Unlock,
        write: bool,
    ) -> Result<Synced, SyncError> {
        let index = self.index();
        let mut reader = reader_for(&index);
        let mut db = FileDb::new();
        sync(
            SyncRequest {
                root,
                manifest,
                dev,
                unlock,
                write,
                index: &index,
                reader: reader.as_mut(),
                registry: Some(&self.registry),
            },
            &mut db,
        )
    }

    fn sync(&self, root: &Path, unlock: Unlock) -> Result<Synced, SyncError> {
        self.sync_with(root, None, true, unlock, true)
    }

    fn unpacked(&self, name: &str, version: &str) -> PathBuf {
        let idx = nova_pm::index_dir_name(&self.index().canonical);
        self.registry
            .join("src")
            .join(idx)
            .join(format!("{name}-{version}"))
    }
}

/// `name version` for each locked package.
fn locked(synced: &Synced) -> Vec<String> {
    synced
        .lock
        .as_ref()
        .map(|lock| {
            lock.packages
                .iter()
                .map(|p| format!("{} {}", p.name, p.version))
                .collect()
        })
        .unwrap_or_default()
}

fn codes(error: SyncError) -> Vec<String> {
    match error {
        SyncError::Diagnostics(list) => list.into_iter().map(|d| d.code).collect(),
        SyncError::Other(message) => panic!("not a diagnostic: {message}"),
    }
}

#[test]
fn a_first_sync_resolves_downloads_and_writes_the_lock() {
    let f = Fixture::new("first");
    f.publish("json", "1.0.0", &[]);
    f.publish("json", "1.2.0", &[]);
    let app = f.app("\n[dependencies]\njson = \"1\"\n");
    let synced = f.sync(&app, Unlock::Nothing).unwrap();
    assert_eq!(locked(&synced), ["json 1.2.0"]);
    assert_eq!(synced.unpacked, ["json 1.2.0"]);
    assert!(f.unpacked("json", "1.2.0").join("src/lib.nova").is_file());
    let text = std::fs::read_to_string(app.join("nova.lock")).unwrap();
    assert!(
        text.contains("name = \"json\"\nversion = \"1.2.0\""),
        "{text}"
    );
    assert!(!text.contains('\r'));
    assert!(text.contains(&format!("index = \"{}\"", f.index().canonical)));
}

#[test]
fn a_complete_lock_needs_no_index() {
    let f = Fixture::new("offline");
    f.publish("json", "1.0.0", &[]);
    let app = f.app("\n[dependencies]\njson = \"1\"\n");
    f.sync(&app, Unlock::Nothing).unwrap();
    let before = std::fs::read_to_string(app.join("nova.lock")).unwrap();
    std::fs::rename(&f.index_dir, f.dir.join("gone")).unwrap();
    let synced = f.sync(&app, Unlock::Nothing).unwrap();
    assert!(synced.unpacked.is_empty());
    assert_eq!(
        std::fs::read_to_string(app.join("nova.lock")).unwrap(),
        before
    );
}

#[test]
fn a_new_entry_keeps_the_other_locked_versions() {
    let f = Fixture::new("keep");
    f.publish("json", "1.0.0", &[]);
    let app = f.app("\n[dependencies]\njson = \"1\"\n");
    f.sync(&app, Unlock::Nothing).unwrap();
    f.publish("json", "1.1.0", &[]);
    f.publish("http", "0.3.0", &[]);
    f.app("\n[dependencies]\njson = \"1\"\nhttp = \"0.3\"\n");
    let synced = f.sync(&app, Unlock::Nothing).unwrap();
    assert_eq!(locked(&synced), ["http 0.3.0", "json 1.0.0"]);
}

#[test]
fn update_moves_every_version_and_update_one_only_that() {
    let f = Fixture::new("update");
    f.publish("json", "1.0.0", &[]);
    f.publish("http", "0.3.0", &[]);
    let app = f.app("\n[dependencies]\njson = \"1\"\nhttp = \"0.3\"\n");
    f.sync(&app, Unlock::Nothing).unwrap();
    f.publish("json", "1.1.0", &[]);
    f.publish("http", "0.3.1", &[]);
    let synced = f.sync(&app, Unlock::One("http".into())).unwrap();
    assert_eq!(locked(&synced), ["http 0.3.1", "json 1.0.0"]);
    assert_eq!(synced.before.unwrap().packages.len(), 2);
    let synced = f.sync(&app, Unlock::All).unwrap();
    assert_eq!(locked(&synced), ["http 0.3.1", "json 1.1.0"]);
}

#[test]
fn the_lock_is_pruned_when_an_entry_goes_without_reading_the_index() {
    let f = Fixture::new("prune");
    f.publish("json", "1.0.0", &[]);
    f.publish("http", "0.3.0", &[]);
    let app = f.app("\n[dependencies]\njson = \"1\"\nhttp = \"0.3\"\n");
    f.sync(&app, Unlock::Nothing).unwrap();
    f.app("\n[dependencies]\njson = \"1\"\n");
    std::fs::rename(&f.index_dir, f.dir.join("gone")).unwrap();
    let synced = f.sync(&app, Unlock::Nothing).unwrap();
    assert_eq!(locked(&synced), ["json 1.0.0"]);
    let text = std::fs::read_to_string(app.join("nova.lock")).unwrap();
    assert!(!text.contains("http"), "{text}");
}

#[test]
fn no_registry_dependency_makes_no_lock_and_an_emptied_one_is_kept() {
    let f = Fixture::new("no-lock");
    let app = f.app("");
    let synced = f.sync(&app, Unlock::Nothing).unwrap();
    assert!(synced.lock.is_none());
    assert!(!app.join("nova.lock").exists());
    f.publish("json", "1.0.0", &[]);
    f.app("\n[dependencies]\njson = \"1\"\n");
    f.sync(&app, Unlock::Nothing).unwrap();
    f.app("");
    let synced = f.sync(&app, Unlock::Nothing).unwrap();
    assert!(synced.lock.unwrap().packages.is_empty());
    let text = std::fs::read_to_string(app.join("nova.lock")).unwrap();
    assert!(!text.contains("[[package]]"), "{text}");
}

#[test]
fn a_conflict_is_m0015_and_writes_nothing() {
    let f = Fixture::new("conflict");
    f.publish("json", "1.0.0", &[]);
    let app = f.app("\n[dependencies]\njson = \"9\"\n");
    assert_eq!(codes(f.sync(&app, Unlock::Nothing).unwrap_err()), ["M0015"]);
    assert!(!app.join("nova.lock").exists());
}

#[test]
fn an_unknown_package_is_m0014() {
    let f = Fixture::new("unknown");
    let app = f.app("\n[dependencies]\nnope = \"1\"\n");
    assert_eq!(codes(f.sync(&app, Unlock::Nothing).unwrap_err()), ["M0014"]);
}

#[test]
fn a_failed_download_writes_no_lock() {
    // The lock is written last (spec §4.6): a lock never names a package
    // that is not unpacked.
    let f = Fixture::new("failed-download");
    f.publish("json", "1.0.0", &[]);
    std::fs::remove_file(f.index_dir.join("dl").join("json-1.0.0.nova-pkg")).unwrap();
    let app = f.app("\n[dependencies]\njson = \"1\"\n");
    assert!(matches!(
        f.sync(&app, Unlock::Nothing),
        Err(SyncError::Other(_))
    ));
    assert!(!app.join("nova.lock").exists());
}

#[test]
fn a_proposed_manifest_is_synced_without_writing_the_lock() {
    let f = Fixture::new("proposed");
    f.publish("json", "1.0.0", &[]);
    let app = f.app("");
    let proposed = manifest("app", "0.1.0", "\n[dependencies]\njson = \"1\"\n");
    let synced = f
        .sync_with(&app, Some(&proposed), true, Unlock::Nothing, false)
        .unwrap();
    assert_eq!(locked(&synced), ["json 1.0.0"]);
    assert!(!app.join("nova.lock").exists());
    assert!(f.unpacked("json", "1.0.0").is_dir());
}

#[test]
fn the_lock_is_rewritten_after_unpacking_even_unchanged() {
    let f = Fixture::new("rewrite");
    f.publish("json", "1.0.0", &[]);
    let app = f.app("\n[dependencies]\njson = \"1\"\n");
    f.sync(&app, Unlock::Nothing).unwrap();
    let lock = app.join("nova.lock");
    let old = SystemTime::now() - Duration::from_secs(3600);
    std::fs::File::options()
        .write(true)
        .open(&lock)
        .unwrap()
        .set_modified(old)
        .unwrap();
    std::fs::remove_dir_all(f.unpacked("json", "1.0.0")).unwrap();
    let synced = f.sync(&app, Unlock::Nothing).unwrap();
    assert_eq!(synced.unpacked, ["json 1.0.0"]);
    let modified = std::fs::metadata(&lock).unwrap().modified().unwrap();
    assert!(modified > old + Duration::from_secs(60));
}

#[test]
fn an_unreadable_lock_is_m0016_except_under_update() {
    let f = Fixture::new("unreadable");
    f.publish("json", "1.0.0", &[]);
    let app = f.app("\n[dependencies]\njson = \"1\"\n");
    std::fs::write(app.join("nova.lock"), "not a lock [").unwrap();
    assert_eq!(codes(f.sync(&app, Unlock::Nothing).unwrap_err()), ["M0016"]);
    let synced = f.sync(&app, Unlock::All).unwrap();
    assert_eq!(locked(&synced), ["json 1.0.0"]);
}

#[test]
fn a_lock_from_another_index_pins_nothing() {
    let f = Fixture::new("other-index");
    f.publish("json", "1.0.0", &[]);
    f.publish("json", "1.1.0", &[]);
    let app = f.app("\n[dependencies]\njson = \"1\"\n");
    std::fs::write(
        app.join("nova.lock"),
        "version = 1\nindex = \"https://elsewhere.test/\"\n\n[[package]]\nname = \"json\"\n\
         version = \"1.0.0\"\nchecksum = \"00\"\ndependencies = []\n",
    )
    .unwrap();
    let synced = f.sync(&app, Unlock::Nothing).unwrap();
    assert_eq!(locked(&synced), ["json 1.1.0"]);
    assert_eq!(synced.lock.unwrap().index, f.index().canonical);
}

#[test]
fn a_path_packages_registry_entries_are_resolved_and_dev_ones_only_with_dev() {
    let f = Fixture::new("path-and-dev");
    f.publish("json", "1.0.0", &[]);
    f.publish("kit", "0.1.0", &[]);
    let util = f.dir.join("util");
    write(
        &util.join("nova.toml"),
        manifest("util", "0.1.0", "\n[dependencies]\njson = \"1\"\n").as_bytes(),
    );
    write(&util.join("src").join("lib.nova"), b"");
    let app = f.app(
        "\n[dependencies]\nutil = { path = \"../util\" }\n\n[dev-dependencies]\nkit = \"0.1\"\n",
    );
    let synced = f.sync(&app, Unlock::Nothing).unwrap();
    assert_eq!(locked(&synced), ["json 1.0.0", "kit 0.1.0"]);
    let synced = f.sync_with(&app, None, false, Unlock::All, false).unwrap();
    assert_eq!(locked(&synced), ["json 1.0.0"]);
}

#[test]
fn a_registry_packages_own_dependencies_are_locked_and_downloaded() {
    let f = Fixture::new("transitive");
    f.publish("json", "1.0.0", &[]);
    f.publish("geom", "0.2.0", &[("json", "1")]);
    let app = f.app("\n[dependencies]\ngeom = \"0.2\"\n");
    let synced = f.sync(&app, Unlock::Nothing).unwrap();
    assert_eq!(locked(&synced), ["geom 0.2.0", "json 1.0.0"]);
    assert_eq!(synced.lock.unwrap().packages[0].dependencies, ["json"]);
    assert!(f.unpacked("json", "1.0.0").is_dir());
}

#[test]
fn notes_about_bad_index_lines_are_returned() {
    let f = Fixture::new("notes");
    f.publish("json", "1.0.0", &[]);
    let file = f.index_dir.join(index_path("json"));
    let mut text = std::fs::read_to_string(&file).unwrap();
    text.push_str("not json\n");
    std::fs::write(&file, text).unwrap();
    let app = f.app("\n[dependencies]\njson = \"1\"\n");
    let synced = f.sync(&app, Unlock::Nothing).unwrap();
    assert_eq!(synced.notes.len(), 1, "{:?}", synced.notes);
    assert!(
        synced.notes[0].starts_with("js/on/json:2:"),
        "{:?}",
        synced.notes
    );
}
