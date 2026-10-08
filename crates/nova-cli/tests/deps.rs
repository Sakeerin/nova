//! `nova add --path` and `nova remove` (spec
//! `docs/superpowers/specs/2026-10-08-phase-3-3a-local-packages-design.md`
//! §5.3, §7.3).

use std::path::{Path, PathBuf};

use assert_cmd::Command;

fn nova() -> Command {
    Command::cargo_bin("nova").expect("nova binary builds")
}

/// A fresh, empty directory with a fixed name, so each run replaces the last.
fn fresh(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("nova-deps-{name}"));
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

fn stderr(assert: &assert_cmd::assert::Assert) -> String {
    String::from_utf8_lossy(&assert.get_output().stderr).into_owned()
}

/// A library called `name` at `dir/<name>`, whose `version()` says `name`.
fn library(dir: &Path, name: &str) {
    write(
        &dir.join(name),
        &[
            (
                "nova.toml",
                format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2026\"\n")
                    .as_str(),
            ),
            (
                "src/lib.nova",
                format!("pub fn version() -> String {{\n    \"{name}\"\n}}\n").as_str(),
            ),
        ],
    );
}

const APP_MANIFEST: &str = "[package]\nname = \"app\" # the app\nversion = \"0.1.0\"\nedition = \"2026\"\n\n# Libraries.\n[dependencies]\n";

/// `dir/app`, with `APP_MANIFEST` and a `main` printing `version()`.
fn app(dir: &Path, import: &str) -> PathBuf {
    let app = dir.join("app");
    write(
        &app,
        &[
            ("nova.toml", APP_MANIFEST),
            (
                "src/main.nova",
                format!("import {import}\n\nfn main() {{\n    println(version())\n}}\n").as_str(),
            ),
        ],
    );
    app
}

#[test]
fn add_writes_a_path_entry_and_keeps_the_rest_of_the_manifest() {
    let dir = fresh("add");
    library(&dir, "geom");
    library(&dir, "helper");
    let app = app(&dir, "geom");
    nova()
        .current_dir(&app)
        .args(["add", "geom", "--path", "../geom"])
        .assert()
        .success();
    assert_eq!(
        read(&app.join("nova.toml")),
        format!("{APP_MANIFEST}geom = {{ path = \"../geom\" }}\n")
    );
    nova()
        .current_dir(&app)
        .arg("run")
        .assert()
        .success()
        .stdout("geom\n");
    nova()
        .current_dir(&app)
        .args(["add", "helper", "--path", "../helper", "--dev"])
        .assert()
        .success();
    let text = read(&app.join("nova.toml"));
    assert!(
        text.starts_with(&format!("{APP_MANIFEST}geom = {{ path = \"../geom\" }}\n")),
        "{text}"
    );
    assert!(
        text.contains("[dev-dependencies]\nhelper = { path = \"../helper\" }\n"),
        "{text}"
    );
}

#[test]
fn add_edits_an_inline_dependency_table() {
    let dir = fresh("add-inline");
    library(&dir, "geom");
    let app = app(&dir, "geom");
    std::fs::write(
        app.join("nova.toml"),
        "dependencies = {}\n\n[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2026\"\n",
    )
    .unwrap();
    nova()
        .current_dir(&app)
        .args(["add", "geom", "--path", "../geom"])
        .assert()
        .success();
    let text = read(&app.join("nova.toml"));
    assert!(text.starts_with("dependencies = {"), "{text}");
    nova()
        .current_dir(&app)
        .arg("run")
        .assert()
        .success()
        .stdout("geom\n");
}

#[test]
fn remove_takes_the_entry_and_its_comment_lines() {
    let dir = fresh("remove");
    let before = "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2026\"\n\n\
                  [dependencies]\n# Shapes.\ngeom = { path = \"../geom\" }\n\n\
                  [dev-dependencies]\n# Kept.\nhelper = { path = \"../helper\" }\n";
    write(
        &dir,
        &[("nova.toml", before), ("src/main.nova", "fn main() {}\n")],
    );
    nova()
        .current_dir(&dir)
        .args(["remove", "geom"])
        .assert()
        .success();
    assert_eq!(
        read(&dir.join("nova.toml")),
        before.replace("# Shapes.\ngeom = { path = \"../geom\" }\n", "")
    );
    let out = nova()
        .current_dir(&dir)
        .args(["remove", "geom"])
        .assert()
        .failure();
    assert!(
        stderr(&out).contains("`geom` is not in [dependencies]"),
        "{}",
        stderr(&out)
    );
    nova()
        .current_dir(&dir)
        .args(["remove", "helper", "--dev"])
        .assert()
        .success();
    assert!(!read(&dir.join("nova.toml")).contains("helper ="));
}

#[test]
fn add_refuses_without_writing() {
    let dir = fresh("refuses");
    library(&dir, "geom");
    // `geom2` depends on `app`, so adding it closes a cycle.
    write(
        &dir.join("geom2"),
        &[
            (
                "nova.toml",
                "[package]\nname = \"geom2\"\nversion = \"0.1.0\"\nedition = \"2026\"\n\n\
                 [dependencies]\napp = { path = \"../app\" }\n",
            ),
            ("src/lib.nova", ""),
        ],
    );
    let app = app(&dir, "geom");
    write(&app, &[("src/lib.nova", "")]);
    let manifest = app.join("nova.toml");
    // The package's own directory is M0010 before its name is compared; a
    // key equal to the package's own name would be M0012 first.
    let cases: [(&[&str], &str); 4] = [
        (&["add", "me", "--path", "."], "M0010"),
        (&["add", "geom2", "--path", "../geom2"], "M0010"),
        (&["add", "geom", "--path", "../nowhere"], "M0007"),
        (
            &["add", "geom"],
            "registry dependencies arrive with the package index",
        ),
    ];
    for (args, expected) in cases {
        let out = nova().current_dir(&app).args(args).assert().failure();
        assert!(
            stderr(&out).contains(expected),
            "{args:?}: {}",
            stderr(&out)
        );
        assert_eq!(read(&manifest), APP_MANIFEST, "{args:?} wrote");
    }
    nova()
        .current_dir(&app)
        .args(["add", "geom", "--path", "../geom"])
        .assert()
        .success();
    let added = read(&manifest);
    let again: [&[&str]; 2] = [
        &["add", "geom", "--path", "../geom"],
        &["add", "geom", "--path", "../geom", "--dev"],
    ];
    for args in again {
        let out = nova().current_dir(&app).args(args).assert().failure();
        assert!(
            stderr(&out).contains("`geom` is already in [dependencies]"),
            "{args:?}: {}",
            stderr(&out)
        );
        assert_eq!(read(&manifest), added);
    }
    // A manifest with errors is refused, its errors shown.
    std::fs::write(&manifest, APP_MANIFEST.replace("2026", "2021")).unwrap();
    let out = nova()
        .current_dir(&app)
        .args(["add", "geom", "--path", "../geom"])
        .assert()
        .failure();
    assert!(stderr(&out).contains("M0003"), "{}", stderr(&out));
    assert_eq!(read(&manifest), APP_MANIFEST.replace("2026", "2021"));
}

#[test]
fn add_works_in_a_package_with_no_source_yet() {
    // Spec §3.1: M0013 does not stop `nova add`, which compiles nothing.
    let dir = fresh("no-source");
    library(&dir, "geom");
    let app = dir.join("app");
    write(&app, &[("nova.toml", APP_MANIFEST)]);
    nova()
        .current_dir(&app)
        .args(["add", "geom", "--path", "../geom"])
        .assert()
        .success();
    assert_eq!(
        read(&app.join("nova.toml")),
        format!("{APP_MANIFEST}geom = {{ path = \"../geom\" }}\n")
    );
}

#[test]
fn add_from_a_subdirectory_writes_a_path_relative_to_the_manifest() {
    // Review Focus 5.
    let dir = fresh("subdirectory");
    library(&dir, "geom");
    let app = app(&dir, "geom");
    nova()
        .current_dir(app.join("src"))
        .args(["add", "geom", "--path", "../../geom"])
        .assert()
        .success();
    assert_eq!(
        read(&app.join("nova.toml")),
        format!("{APP_MANIFEST}geom = {{ path = \"../geom\" }}\n")
    );
}

#[test]
fn a_hyphenated_library_is_added_and_imported_with_an_underscore() {
    // Review Focus 3.
    let dir = fresh("hyphen");
    library(&dir, "json-api");
    let app = app(&dir, "json_api");
    nova()
        .current_dir(&app)
        .args(["add", "json-api", "--path", "../json-api"])
        .assert()
        .success();
    nova()
        .current_dir(&app)
        .arg("run")
        .assert()
        .success()
        .stdout("json-api\n");
}
