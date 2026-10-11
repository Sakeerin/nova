//! `nova doc` end to end (spec
//! `docs/superpowers/specs/2026-10-11-phase-3-5a-the-documentation-site-design.md`
//! §4, §16.3).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use assert_cmd::assert::Assert;
use assert_cmd::Command;

fn nova() -> Command {
    Command::cargo_bin("nova").expect("nova binary builds")
}

/// A fresh, empty directory with a fixed name, so each run replaces the
/// last.
fn fresh(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("nova-doc-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create the test directory");
    dir
}

fn write(dir: &Path, files: &[(&str, &str)]) {
    for (path, text) in files {
        let path = dir.join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }
}

fn manifest(name: &str, extra: &str) -> String {
    format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2026\"\n{extra}")
}

/// `nova doc ARGS` in `dir`, with a fresh `NOVA_HOME` beside it. Colour
/// is off: `nova` colours its diagnostics even into a pipe.
fn doc(dir: &Path, args: &[&str]) -> Assert {
    let home = dir.with_file_name(format!(
        "{}-home",
        dir.file_name().unwrap().to_string_lossy()
    ));
    let _ = std::fs::remove_dir_all(&home);
    nova()
        .current_dir(dir)
        .env("NOVA_HOME", &home)
        .env("NO_COLOR", "1")
        .arg("doc")
        .args(args)
        .assert()
}

fn stdout(a: &Assert) -> String {
    String::from_utf8_lossy(&a.get_output().stdout)
        .trim()
        .replace('\\', "/")
}

fn stderr(a: &Assert) -> String {
    String::from_utf8_lossy(&a.get_output().stderr).into_owned()
}

/// A page of the site under `dir/target/doc`.
fn page(dir: &Path, path: &str) -> String {
    std::fs::read_to_string(dir.join("target").join("doc").join(path))
        .unwrap_or_else(|e| panic!("{path}: {e}"))
}

const GEOM_LIB: &str = "import utils\n\n/// A point.\npub record Point {\n    /// Across.\n    pub x: Int,\n    y: Int,\n}\n\nimpl Point {\n    /// Its x.\n    pub fn across(self) -> Int { self.x }\n\n    fn down(self) -> Int { self.y }\n}\n\n/// The area of a square of side `width()`.\npub fn area() -> Int { width() * width() }\n\nfn secret() -> Int { 1 }\n";
const GEOM_UTILS: &str = "/// Three.\npub fn width() -> Int { 3 }\n";
const APP_MAIN: &str = "import geom\n\n/// Shows `p`.\nfn show(p: Point) -> String { \"${p.x}\" }\n\n/// A record of the app's own.\nrecord Local {\n    hidden: Int,\n}\n\nfn main() {\n    println(show(Point { x: area(), y: 0 }))\n}\n";

/// `dir/geom`, a library, and `dir/app`, a program that uses it, with
/// `extra` appended to the app's manifest. Returns the app's directory.
fn geom_and_app(dir: &Path, extra: &str) -> PathBuf {
    write(
        &dir.join("geom"),
        &[
            ("nova.toml", &manifest("geom", "")),
            ("src/lib.nova", GEOM_LIB),
            ("src/utils.nova", GEOM_UTILS),
        ],
    );
    let deps = format!("\n[dependencies]\ngeom = {{ path = \"../geom\" }}\n{extra}");
    write(
        &dir.join("app"),
        &[
            ("nova.toml", &manifest("app", &deps)),
            ("src/main.nova", APP_MAIN),
        ],
    );
    dir.join("app")
}

#[test]
fn a_program_and_its_path_library_are_documented() {
    let app = geom_and_app(&fresh("program"), "");
    let a = doc(&app, &[]).success();
    assert_eq!(stdout(&a), "target/doc/app/index.html");
    for file in [
        ".nova-doc",
        "index.html",
        "nova.css",
        "nova.js",
        "search-index.js",
        "app/index.html",
        "geom/index.html",
        "geom/utils/index.html",
        "std/index.html",
        "std/core/index.html",
        "std/primitive.String.html",
    ] {
        assert!(app.join("target/doc").join(file).is_file(), "{file}");
    }
    let main = page(&app, "app/index.html");
    for id in [
        "fn.show",
        "fn.main",
        "record.Local",
        "record.Local.field.hidden",
    ] {
        assert!(main.contains(&format!(" id=\"{id}\"")), "{id}");
    }
    assert!(
        main.contains("href=\"../geom/index.html#record.Point\""),
        "{main}"
    );
    assert!(
        main.contains("href=\"../std/primitive.String.html\""),
        "{main}"
    );
}

#[test]
fn a_library_documents_its_exports_only() {
    let dir = fresh("library");
    geom_and_app(&dir, "");
    let geom = dir.join("geom");
    let a = doc(&geom, &[]).success();
    assert_eq!(stdout(&a), "target/doc/geom/index.html");
    let lib = page(&geom, "geom/index.html");
    for id in [
        "record.Point",
        "record.Point.field.x",
        "record.Point.method.across",
        "fn.area",
    ] {
        assert!(lib.contains(&format!(" id=\"{id}\"")), "{id}");
    }
    for id in [
        "record.Point.field.y",
        "record.Point.method.down",
        "fn.secret",
    ] {
        assert!(!lib.contains(&format!(" id=\"{id}\"")), "{id}");
    }
    assert!(lib.contains("private fields"), "{lib}");
    assert!(page(&geom, "geom/utils/index.html").contains(" id=\"fn.width\""));
}

#[test]
fn an_unused_dependency_has_no_pages() {
    let dir = fresh("unused");
    write(
        &dir.join("extra"),
        &[
            ("nova.toml", &manifest("extra", "")),
            ("src/lib.nova", "pub fn one() -> Int { 1 }\n"),
        ],
    );
    let app = geom_and_app(&dir, "extra = { path = \"../extra\" }\n");
    doc(&app, &[]).success();
    assert!(app.join("target/doc/geom/index.html").is_file());
    assert!(!app.join("target/doc/extra").exists());
}

#[test]
fn an_error_stops_it_and_a_warning_does_not() {
    let dir = fresh("error");
    write(
        &dir,
        &[
            ("nova.toml", &manifest("app", "")),
            ("src/main.nova", "fn main() {\n    let x: Int = \"s\"\n}\n"),
        ],
    );
    let a = doc(&dir, &[]).failure();
    let err = stderr(&a);
    assert!(err.contains("error["), "{err}");
    assert!(
        err.contains("could not document due to 1 previous error"),
        "{err}"
    );
    assert!(!dir.join("target").join("doc").exists());

    let dir = fresh("warning");
    write(
        &dir,
        &[
            ("nova.toml", &manifest("app", "")),
            (
                "src/main.nova",
                "fn main() {\n    let x = 1\n    match x {\n        _ => println(\"a\")\n        1 => println(\"b\")\n    }\n}\n",
            ),
        ],
    );
    let a = doc(&dir, &[]).success();
    assert!(stderr(&a).contains("warning[E0021]"), "{}", stderr(&a));
}

#[test]
fn outside_a_project_it_names_std() {
    let dir = fresh("outside");
    let a = doc(&dir, &[]).failure();
    assert!(stderr(&a).contains("`nova doc --std`"), "{}", stderr(&a));
}

#[test]
fn std_alone() {
    let dir = fresh("std");
    let a = doc(&dir, &["--std"]).success();
    assert_eq!(stdout(&a), "target/doc/std/index.html");
    let mut top: Vec<String> = std::fs::read_dir(dir.join("target/doc"))
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    top.sort();
    assert_eq!(
        top,
        [
            ".nova-doc",
            "index.html",
            "nova.css",
            "nova.js",
            "search-index.js",
            "std"
        ]
    );
    assert!(dir.join("target/doc/std/test/index.html").is_file());
}

#[test]
fn a_second_run_replaces_the_site() {
    // Review Focus 5.
    let dir = fresh("replace");
    write(
        &dir,
        &[
            ("nova.toml", &manifest("app", "")),
            (
                "src/main.nova",
                "import util\n\nfn main() {\n    println(\"${one()}\")\n}\n",
            ),
            ("src/util.nova", "pub fn one() -> Int { 1 }\n"),
        ],
    );
    doc(&dir, &[]).success();
    assert!(dir.join("target/doc/app/util/index.html").is_file());
    std::fs::remove_file(dir.join("src/util.nova")).unwrap();
    std::fs::write(dir.join("src/main.nova"), "fn main() {}\n").unwrap();
    doc(&dir, &[]).success();
    assert!(!dir.join("target/doc/app/util").exists());
}

#[test]
fn a_target_doc_it_did_not_write_is_refused() {
    let dir = fresh("not-ours");
    write(
        &dir,
        &[
            ("nova.toml", &manifest("app", "")),
            ("src/main.nova", "fn main() {}\n"),
            ("target/doc/keep.txt", "keep"),
        ],
    );
    let a = doc(&dir, &[]).failure();
    assert!(
        stderr(&a).contains("was not written by nova doc"),
        "{}",
        stderr(&a)
    );
    assert!(dir.join("target/doc/keep.txt").is_file());
}

#[test]
fn a_package_named_std_is_refused() {
    let dir = fresh("named-std");
    write(
        &dir,
        &[
            ("nova.toml", &manifest("std", "")),
            ("src/main.nova", "fn main() {}\n"),
        ],
    );
    let a = doc(&dir, &[]).failure();
    assert!(stderr(&a).contains("`std`"), "{}", stderr(&a));
}

#[test]
fn a_broken_doc_link_is_a_warning() {
    // Review Focus 4.
    let dir = fresh("doc-link");
    write(
        &dir,
        &[
            ("nova.toml", &manifest("app", "")),
            (
                "src/main.nova",
                "/// See [the guide](guide.html).\nfn main() {}\n",
            ),
        ],
    );
    let a = doc(&dir, &[]).success();
    assert!(
        stderr(&a).contains("warning: app/index.html: broken link `guide.html`"),
        "{}",
        stderr(&a)
    );
}

#[test]
fn open_with_a_missing_browser_warns() {
    let dir = fresh("open-missing");
    write(
        &dir,
        &[
            ("nova.toml", &manifest("app", "")),
            ("src/main.nova", "fn main() {}\n"),
        ],
    );
    let home = dir.with_file_name("nova-doc-open-missing-home");
    let _ = std::fs::remove_dir_all(&home);
    let a = nova()
        .current_dir(&dir)
        .env("NOVA_HOME", &home)
        .env("BROWSER", dir.join("no-such-browser"))
        .args(["doc", "--open"])
        .assert()
        .success();
    let err = stderr(&a);
    assert!(err.contains("could not open a browser"), "{err}");
    assert!(
        err.replace('\\', "/").contains("target/doc/app/index.html"),
        "{err}"
    );
}

#[cfg(unix)]
#[test]
fn open_runs_the_browser_with_the_start_page() {
    use std::os::unix::fs::PermissionsExt;
    let dir = fresh("open-runs");
    write(
        &dir,
        &[
            ("nova.toml", &manifest("app", "")),
            ("src/main.nova", "fn main() {}\n"),
        ],
    );
    let opened = dir.join("opened.txt");
    let browser = dir.join("browser.sh");
    std::fs::write(
        &browser,
        format!("#!/bin/sh\necho \"$1\" > \"{}\"\n", opened.display()),
    )
    .unwrap();
    std::fs::set_permissions(&browser, std::fs::Permissions::from_mode(0o755)).unwrap();
    let home = dir.with_file_name("nova-doc-open-runs-home");
    let _ = std::fs::remove_dir_all(&home);
    nova()
        .current_dir(&dir)
        .env("NOVA_HOME", &home)
        .env("BROWSER", &browser)
        .args(["doc", "--open"])
        .assert()
        .success();
    // The browser runs on its own; give it up to ten seconds.
    let mut seen = String::new();
    for _ in 0..100 {
        seen = std::fs::read_to_string(&opened).unwrap_or_default();
        if !seen.is_empty() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    assert!(
        seen.trim().ends_with("target/doc/app/index.html"),
        "{seen:?}"
    );
    assert!(Path::new(seen.trim()).is_absolute(), "{seen:?}");
}

/// Every file under `dir`, by its path, with its bytes.
fn files(dir: &Path, prefix: &str, out: &mut BTreeMap<String, Vec<u8>>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let entry = entry.unwrap();
        let name = format!("{prefix}{}", entry.file_name().to_string_lossy());
        if entry.path().is_dir() {
            files(&entry.path(), &format!("{name}/"), out);
        } else {
            out.insert(name, std::fs::read(entry.path()).unwrap());
        }
    }
}

#[test]
fn two_runs_write_the_same_bytes() {
    let app = geom_and_app(&fresh("same-bytes"), "");
    doc(&app, &[]).success();
    let mut first = BTreeMap::new();
    files(&app.join("target/doc"), "", &mut first);
    doc(&app, &[]).success();
    let mut second = BTreeMap::new();
    files(&app.join("target/doc"), "", &mut second);
    assert_eq!(
        first.keys().collect::<Vec<_>>(),
        second.keys().collect::<Vec<_>>()
    );
    for (name, bytes) in &first {
        assert!(second[name] == *bytes, "{name} differs");
    }
}
