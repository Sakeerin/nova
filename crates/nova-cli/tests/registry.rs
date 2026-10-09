//! The package index end to end through `nova` (spec
//! `docs/superpowers/specs/2026-10-09-phase-3-3b-index-and-publishing-design.md`
//! §6, §10.5). Every command runs with `NOVA_HOME` and `NOVA_INDEX` set to
//! the test's own directories, so nothing reaches the internet.

#[allow(unused_imports)]
use std::env::consts::EXE_SUFFIX;
use std::path::{Path, PathBuf};

use assert_cmd::Command;
use nova_index::{Index, Line};
use nova_pm::IndexView;

/// A fresh, empty directory with a fixed name, so each run replaces the last.
fn fresh(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("nova-registry-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create the test directory");
    dir
}

/// Write each `(path, text)` under `dir`.
fn write(dir: &Path, files: &[(&str, &str)]) {
    for (path, text) in files {
        let path = dir.join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|e| panic!("reading {}: {e}", path.display()))
}

fn stdout(assert: &assert_cmd::assert::Assert) -> String {
    String::from_utf8_lossy(&assert.get_output().stdout).into_owned()
}

fn stderr(assert: &assert_cmd::assert::Assert) -> String {
    String::from_utf8_lossy(&assert.get_output().stderr).into_owned()
}

fn manifest(name: &str, version: &str, extra: &str) -> String {
    format!("[package]\nname = \"{name}\"\nversion = \"{version}\"\nedition = \"2026\"\n{extra}")
}

const AREA: &str = "pub fn area() -> Int {\n    9\n}\n";
#[allow(dead_code)]
const MAIN_AREA: &str = "import geom\n\nfn main() {\n    println(\"${area()}\")\n}\n";
/// A match whose second arm is unreachable: E0021, a warning.
const UNREACHABLE: &str = "pub fn pick(n: Int) -> Int {\n    match n { _ => 1, 0 => 2 }\n}\n";

/// A local index in `dir/index` whose `dl` is relative, and a `NOVA_HOME`
/// in `dir/home`.
struct Fixture {
    dir: PathBuf,
    index: PathBuf,
    home: PathBuf,
}

impl Fixture {
    fn new(name: &str) -> Fixture {
        let dir = fresh(name);
        let index = dir.join("index");
        write(
            &index,
            &[("config.json", r#"{"dl":"dl/{name}-{version}.nova-pkg"}"#)],
        );
        Fixture {
            home: dir.join("home"),
            index,
            dir,
        }
    }

    /// `nova`, in `cwd`, with this fixture's index and home.
    fn nova(&self, cwd: &Path) -> Command {
        let mut command = Command::cargo_bin("nova").expect("nova binary builds");
        command
            .current_dir(cwd)
            .env("NOVA_INDEX", &self.index)
            .env("NOVA_HOME", &self.home);
        command
    }

    /// The library `name` `version` in `dir/sources/<name>-<version>`.
    fn library(&self, name: &str, version: &str, extra: &str, lib: &str) -> PathBuf {
        let dir = self.dir.join("sources").join(format!("{name}-{version}"));
        write(
            &dir,
            &[
                ("nova.toml", manifest(name, version, extra).as_str()),
                ("src/lib.nova", lib),
            ],
        );
        dir
    }

    /// Publish it with `nova publish`.
    fn publish(&self, name: &str, version: &str, extra: &str, lib: &str) {
        let source = self.library(name, version, extra, lib);
        self.nova(&source).arg("publish").assert().success();
    }

    /// Publish it without `nova publish`'s verification, as a package
    /// published by an older or careless nova could be.
    fn publish_by_hand(&self, name: &str, version: &str, lib: &str) {
        let source = self.library(name, version, "", lib);
        let packed =
            nova_index::pack(&source, name, &semver::Version::parse(version).unwrap()).unwrap();
        let line = Line {
            name: name.into(),
            vers: version.into(),
            deps: Vec::new(),
            cksum: packed.checksum,
            v: 1,
        };
        nova_index::publish_local(&self.index(), &line, &packed.bytes).unwrap();
    }

    fn index(&self) -> Index {
        Index::new(self.index.to_str().unwrap()).unwrap()
    }

    /// The program `app` in `dir/app`, with `extra` appended to its
    /// manifest and `main` as its `src/main.nova`.
    fn app(&self, extra: &str, main: &str) -> PathBuf {
        let app = self.dir.join("app");
        write(
            &app,
            &[
                ("nova.toml", manifest("app", "0.1.0", extra).as_str()),
                ("src/main.nova", main),
            ],
        );
        app
    }

    /// Where `name` `version` is unpacked.
    fn cached(&self, name: &str, version: &str) -> PathBuf {
        self.home
            .join("registry")
            .join("src")
            .join(nova_pm::index_dir_name(&self.index().canonical))
            .join(format!("{name}-{version}"))
    }
}

#[test]
fn package_writes_a_reproducible_tarball_and_reports_it() {
    let f = Fixture::new("package");
    let geom = f.library("geom", "0.1.0", "", AREA);
    let out = stdout(&f.nova(&geom).arg("package").assert().success());
    let tarball = geom
        .join("target")
        .join("package")
        .join("geom-0.1.0.nova-pkg");
    let bytes = std::fs::read(&tarball).unwrap();
    assert!(out.contains("packed geom 0.1.0: "), "{out}");
    assert!(out.contains("geom-0.1.0.nova-pkg"), "{out}");
    assert!(out.contains("2 files"), "{out}");
    assert!(out.contains(&nova_index::sha256_hex(&bytes)), "{out}");
    f.nova(&geom).arg("package").assert().success();
    assert_eq!(std::fs::read(&tarball).unwrap(), bytes);
}

#[test]
fn publish_writes_the_tarball_where_dl_says_and_appends_a_line() {
    let f = Fixture::new("publish");
    f.publish("geom", "0.1.0", "", AREA);
    let text = read(&f.index.join("ge").join("om").join("geom"));
    assert!(
        text.starts_with("{\"name\":\"geom\",\"vers\":\"0.1.0\",\"deps\":[],\"cksum\":\""),
        "{text}"
    );
    assert!(text.ends_with("\"v\":1}\n"), "{text}");
    assert!(f.index.join("dl").join("geom-0.1.0.nova-pkg").is_file());
}

#[test]
fn a_version_already_published_is_refused() {
    let f = Fixture::new("again");
    f.publish("geom", "0.1.0", "", AREA);
    let file = f.index.join("ge").join("om").join("geom");
    let before = read(&file);
    let source = f.library("geom", "0.1.0", "", AREA);
    let out = f.nova(&source).arg("publish").assert().failure();
    assert!(
        stderr(&out).contains("geom 0.1.0 is already in the index"),
        "{}",
        stderr(&out)
    );
    assert_eq!(read(&file), before);
}

#[test]
fn a_capitalised_name_lives_at_the_lower_case_path() {
    // Review Focus 4.
    let f = Fixture::new("capital");
    f.publish("Geom", "0.1.0", "", AREA);
    let text = read(&f.index.join("ge").join("om").join("geom"));
    assert!(text.contains("\"name\":\"Geom\""), "{text}");
    let source = f.library("geom", "0.2.0", "", AREA);
    let out = f.nova(&source).arg("publish").assert().failure();
    assert!(
        stderr(&out).contains("differs from `geom` only in case"),
        "{}",
        stderr(&out)
    );
    // Resolved by its exact name only.
    let mut reader = nova_index::LocalReader {
        dir: f.index.clone(),
    };
    let mut view = nova_index::View::new(&mut reader);
    assert!(view.versions("Geom").unwrap().is_some());
    assert_eq!(view.versions("geom").unwrap(), None);
}

#[test]
fn a_path_dependency_is_m0017_and_a_path_dev_dependency_is_allowed() {
    let f = Fixture::new("m0017");
    f.library("util", "0.1.0", "", AREA);
    let geom = f.library(
        "geom",
        "0.1.0",
        "\n[dependencies]\nutil = { path = \"../util-0.1.0\" }\n",
        AREA,
    );
    let out = f.nova(&geom).arg("package").assert().failure();
    let err = stderr(&out);
    assert!(err.contains("M0017"), "{err}");
    assert!(
        err.contains("publish `util` first and depend on its version"),
        "{err}"
    );
    assert!(!geom.join("target").exists());
    let geom = f.library(
        "geom",
        "0.1.0",
        "\n[dev-dependencies]\nutil = { path = \"../util-0.1.0\" }\n",
        AREA,
    );
    f.nova(&geom).arg("package").assert().success();
}

#[test]
fn package_refuses_a_program_and_a_broken_manifest() {
    let f = Fixture::new("refusals");
    let app = f.app("", "fn main() {}\n");
    let out = f.nova(&app).arg("package").assert().failure();
    assert!(
        stderr(&out).contains("only a library can be published"),
        "{}",
        stderr(&out)
    );
    let broken = f.library("geom", "0.1.0", "colour = \n", AREA);
    let out = f.nova(&broken).arg("package").assert().failure();
    assert!(stderr(&out).contains("M0001"), "{}", stderr(&out));
    assert!(!broken.join("target").exists());
}

#[test]
fn verification_fails_on_a_warning_in_the_package() {
    let f = Fixture::new("own-warning");
    let geom = f.library("geom", "0.1.0", "", UNREACHABLE);
    let out = f.nova(&geom).arg("package").assert().failure();
    let err = stderr(&out);
    assert!(err.contains("E0021"), "{err}");
    assert!(err.contains("0 errors and 1 warning"), "{err}");
    assert!(!geom.join("target").exists());
}

#[test]
fn verification_fails_on_a_dependency_not_published() {
    let f = Fixture::new("unpublished");
    let geom = f.library("geom", "0.1.0", "\n[dependencies]\njson = \"1\"\n", AREA);
    let out = f.nova(&geom).arg("package").assert().failure();
    assert!(stderr(&out).contains("M0014"), "{}", stderr(&out));
}

#[test]
fn verification_passes_despite_a_warning_in_a_dependency() {
    let f = Fixture::new("dependency-warning");
    f.publish_by_hand("json", "1.0.0", UNREACHABLE);
    let geom = f.library(
        "geom",
        "0.1.0",
        "\n[dependencies]\njson = \"1\"\n",
        "import json\n\npub fn area() -> Int {\n    pick(3)\n}\n",
    );
    f.nova(&geom).arg("package").assert().success();
    // The verification downloaded `json`; nova refuses to work inside its
    // cache.
    let out = f
        .nova(&f.cached("json", "1.0.0"))
        .arg("package")
        .assert()
        .failure();
    assert!(
        stderr(&out).contains("is a downloaded package in nova's cache; it is read only"),
        "{}",
        stderr(&out)
    );
}

#[test]
fn publishing_to_a_local_index_needs_a_relative_dl() {
    let f = Fixture::new("absolute-dl");
    write(
        &f.index,
        &[("config.json", r#"{"dl":"https://example.test/{name}"}"#)],
    );
    let geom = f.library("geom", "0.1.0", "", AREA);
    let out = f.nova(&geom).arg("publish").assert().failure();
    assert!(
        stderr(&out).contains("must be a path under the index"),
        "{}",
        stderr(&out)
    );
}
