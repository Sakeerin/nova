//! The package index end to end through `nova` (spec
//! `docs/superpowers/specs/2026-10-09-phase-3-3b-index-and-publishing-design.md`
//! §6, §10.5). Every command runs with `NOVA_HOME` and `NOVA_INDEX` set to
//! the test's own directories, so nothing reaches the internet.

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

#[test]
fn add_from_the_index_writes_the_version_chosen_and_the_lock() {
    let f = Fixture::new("add");
    f.publish("geom", "0.1.0", "", AREA);
    f.publish("geom", "0.2.0", "", AREA);
    let app = f.app("", MAIN_AREA);
    let out = stdout(&f.nova(&app).args(["add", "geom"]).assert().success());
    assert!(
        out.contains("added geom = \"0.2.0\" to [dependencies]"),
        "{out}"
    );
    assert!(read(&app.join("nova.toml")).contains("geom = \"0.2.0\""));
    let lock = read(&app.join("nova.lock"));
    assert!(
        lock.contains("name = \"geom\"\nversion = \"0.2.0\""),
        "{lock}"
    );
}

#[test]
fn add_with_a_requirement_writes_it_as_given() {
    let f = Fixture::new("add-requirement");
    f.publish("geom", "0.1.0", "", AREA);
    f.publish("geom", "0.2.0", "", AREA);
    let app = f.app("", MAIN_AREA);
    f.nova(&app).args(["add", "geom@0.1"]).assert().success();
    assert!(read(&app.join("nova.toml")).contains("geom = \"0.1\""));
    assert!(read(&app.join("nova.lock")).contains("version = \"0.1.0\""));
}

#[test]
fn add_from_the_index_keeps_a_crlf_manifests_line_endings() {
    // Review Focus 2.
    let f = Fixture::new("add-crlf");
    f.publish("geom", "0.1.0", "", AREA);
    let app = f.app("", MAIN_AREA);
    let crlf = manifest("app", "0.1.0", "\n[dependencies]\n").replace('\n', "\r\n");
    std::fs::write(app.join("nova.toml"), &crlf).unwrap();
    f.nova(&app).args(["add", "geom"]).assert().success();
    let text = read(&app.join("nova.toml"));
    assert!(text.contains("geom = \"0.1.0\"\r\n"), "{text:?}");
    assert!(!text.replace("\r\n", "").contains('\n'), "{text:?}");
}

#[test]
fn a_refused_add_leaves_both_files_untouched() {
    let f = Fixture::new("add-refused");
    f.publish("geom", "0.1.0", "", AREA);
    f.publish("json", "1.0.0", "", AREA);
    let app = f.app("", MAIN_AREA);
    f.nova(&app).args(["add", "geom"]).assert().success();
    let manifest_before = read(&app.join("nova.toml"));
    let lock_before = read(&app.join("nova.lock"));
    for (args, expected) in [
        (&["add", "nope"][..], "M0014"),
        (&["add", "json@9"][..], "M0015"),
        (&["add", "x@1", "--path", "../x"][..], "never both"),
    ] {
        let out = f.nova(&app).args(args).assert().failure();
        assert!(
            stderr(&out).contains(expected),
            "{args:?}: {}",
            stderr(&out)
        );
        assert_eq!(read(&app.join("nova.toml")), manifest_before, "{args:?}");
        assert_eq!(read(&app.join("nova.lock")), lock_before, "{args:?}");
    }
}

#[test]
fn an_app_runs_builds_and_tests_with_a_published_library() {
    let f = Fixture::new("end-to-end");
    f.publish("geom", "0.1.0", "", AREA);
    let app = f.app("", MAIN_AREA);
    write(
        &app,
        &[(
            "tests/area.nova",
            "import geom\n\n@test\nfn area_is_nine() {\n    assert_eq(area(), 9)\n}\n",
        )],
    );
    f.nova(&app).args(["add", "geom"]).assert().success();
    let out = f.nova(&app).arg("run").assert().success();
    assert_eq!(stdout(&out).trim(), "9");
    f.nova(&app).arg("build").assert().success();
    let exe = app
        .join("target")
        .join("debug")
        .join(format!("app{EXE_SUFFIX}"));
    let ran = std::process::Command::new(&exe).output().unwrap();
    assert_eq!(String::from_utf8_lossy(&ran.stdout).trim(), "9");
    f.nova(&app).arg("test").assert().success();
}

#[test]
fn update_moves_to_a_newer_version_and_says_so() {
    let f = Fixture::new("update");
    f.publish("geom", "0.1.0", "", AREA);
    let app = f.app("", MAIN_AREA);
    f.nova(&app).args(["add", "geom@0.1"]).assert().success();
    f.publish("geom", "0.1.1", "", AREA);
    let out = stdout(&f.nova(&app).arg("update").assert().success());
    assert!(out.contains("geom 0.1.0 -> 0.1.1"), "{out}");
    let out = stdout(&f.nova(&app).arg("update").assert().success());
    assert!(out.contains("nothing to update"), "{out}");
}

#[test]
fn update_one_name_leaves_the_others() {
    let f = Fixture::new("update-one");
    f.publish("geom", "0.1.0", "", AREA);
    f.publish("json", "1.0.0", "", AREA);
    let app = f.app("", MAIN_AREA);
    f.nova(&app).args(["add", "geom@0.1"]).assert().success();
    f.nova(&app).args(["add", "json@1"]).assert().success();
    f.publish("geom", "0.1.1", "", AREA);
    f.publish("json", "1.0.1", "", AREA);
    let out = stdout(&f.nova(&app).args(["update", "geom"]).assert().success());
    assert!(out.contains("geom 0.1.0 -> 0.1.1"), "{out}");
    assert!(!out.contains("json"), "{out}");
    assert!(read(&app.join("nova.lock")).contains("version = \"1.0.0\""));
    let out = f.nova(&app).args(["update", "nope"]).assert().failure();
    assert!(
        stderr(&out).contains("`nope` is not in nova.lock"),
        "{}",
        stderr(&out)
    );
}

#[test]
fn fetch_fills_the_cache_and_a_build_needs_no_index() {
    let f = Fixture::new("fetch");
    f.publish("geom", "0.1.0", "", AREA);
    let app = f.app("", MAIN_AREA);
    f.nova(&app).args(["add", "geom"]).assert().success();
    std::fs::remove_dir_all(&f.home).unwrap();
    let out = stdout(&f.nova(&app).arg("fetch").assert().success());
    assert!(out.contains("fetched 1 package"), "{out}");
    let out = stdout(&f.nova(&app).arg("fetch").assert().success());
    assert!(out.contains("nothing to fetch"), "{out}");
    std::fs::rename(&f.index, f.dir.join("index-gone")).unwrap();
    f.nova(&app).arg("build").assert().success();
}

#[test]
fn a_command_inside_the_cache_is_refused() {
    let f = Fixture::new("inside-cache");
    f.publish("geom", "0.1.0", "", AREA);
    let app = f.app("", MAIN_AREA);
    f.nova(&app).args(["add", "geom"]).assert().success();
    let out = f
        .nova(&f.cached("geom", "0.1.0"))
        .arg("check")
        .assert()
        .failure();
    assert!(
        stderr(&out).contains("is a downloaded package in nova's cache; it is read only"),
        "{}",
        stderr(&out)
    );
}

#[test]
fn a_library_is_refused_by_run_before_anything_syncs() {
    let f = Fixture::new("library-run");
    let geom = f.library("geom", "0.1.0", "\n[dependencies]\nnope = \"1\"\n", AREA);
    let out = f.nova(&geom).arg("run").assert().failure();
    let err = stderr(&out);
    assert!(err.contains("`geom` is a library"), "{err}");
    assert!(!err.contains("M0014"), "{err}");
}
#[path = "../../nova-index/tests/support/mod.rs"]
mod support;

use support::fake_github::FakeGitHub;

/// A stand-in GitHub index: its config.json read raw from the stand-in,
/// whose API it also is, with `dl` at its release downloads. The URL is
/// known only once the stand-in runs, so config.json is written then.
fn github_index() -> FakeGitHub {
    let fake = FakeGitHub::start("{}");
    let config = format!(
        r#"{{"dl":"{}/dl/{{name}}-{{version}}.nova-pkg","api":"owner/index"}}"#,
        fake.server.url
    );
    fake.state
        .lock()
        .unwrap()
        .files
        .insert("config.json".into(), (config, "sha-config".into()));
    fake
}

/// `nova` as `f.nova` does, but reading `fake`'s index.
fn nova_on(f: &Fixture, fake: &FakeGitHub, cwd: &Path) -> Command {
    let mut command = f.nova(cwd);
    command
        .env("NOVA_INDEX", format!("{}/raw/", fake.server.url))
        .env("NOVA_GITHUB_API", fake.api());
    command
}

#[test]
fn login_checks_the_token_and_stores_it_unprinted() {
    let f = Fixture::new("login");
    let fake = github_index();
    let app = f.app("", "fn main() {}\n");
    let out = nova_on(&f, &fake, &app)
        .arg("login")
        .write_stdin(format!("{}\n", FakeGitHub::TOKEN))
        .assert()
        .success();
    assert!(!stdout(&out).contains(FakeGitHub::TOKEN));
    assert!(!stderr(&out).contains(FakeGitHub::TOKEN));
    let stored = read(&f.home.join("credentials.toml"));
    assert!(stored.contains(FakeGitHub::TOKEN), "{stored}");
    fake.state.lock().unwrap().push = false;
    let out = nova_on(&f, &fake, &app)
        .arg("login")
        .write_stdin(format!("{}\n", FakeGitHub::TOKEN))
        .assert()
        .failure();
    assert!(
        stderr(&out).contains("cannot push to owner/index"),
        "{}",
        stderr(&out)
    );
    let out = nova_on(&f, &fake, &app)
        .arg("login")
        .write_stdin("ghp_wrong\n")
        .assert()
        .failure();
    assert!(!stderr(&out).contains("ghp_wrong"), "{}", stderr(&out));
}

#[test]
fn publish_to_a_github_index_goes_through_the_api() {
    let f = Fixture::new("publish-github");
    let fake = github_index();
    let geom = f.library("geom", "0.1.0", "", AREA);
    let out = nova_on(&f, &fake, &geom).arg("publish").assert().failure();
    assert!(
        stderr(&out).contains("not logged in: run `gh auth token | nova login` first"),
        "{}",
        stderr(&out)
    );
    nova_on(&f, &fake, &geom)
        .arg("login")
        .write_stdin(FakeGitHub::TOKEN)
        .assert()
        .success();
    let out = nova_on(&f, &fake, &geom).arg("publish").assert().success();
    let printed = format!("{}{}", stdout(&out), stderr(&out));
    assert!(printed.contains("up to five minutes"), "{printed}");
    assert!(!printed.contains(FakeGitHub::TOKEN), "{printed}");
    let text = fake.state.lock().unwrap().files["ge/om/geom"].0.clone();
    assert!(text.contains("\"vers\":\"0.1.0\""), "{text}");
    // The token went to the API only.
    for request in fake.server.requests() {
        if request.header("authorization").is_some() {
            assert!(request.path.starts_with("/repos/"), "{request:?}");
        }
    }
    // An app depends on it, reading the index raw and the tarball from
    // the release, with no token.
    let app = f.app("", MAIN_AREA);
    nova_on(&f, &fake, &app)
        .args(["add", "geom"])
        .assert()
        .success();
    let out = nova_on(&f, &fake, &app).arg("run").assert().success();
    assert_eq!(stdout(&out).trim(), "9");
}
