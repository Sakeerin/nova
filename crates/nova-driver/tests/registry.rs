//! A registry package in the driver (spec
//! `docs/superpowers/specs/2026-10-09-phase-3-3b-index-and-publishing-design.md`
//! §5.4, §6.6): its warnings are neither shown nor counted, and its errors
//! are.

use std::path::{Path, PathBuf};

use nova_driver::{
    analyze_program, check_program_counted, Checked, DiskSources, Options, Program, Roots,
};
use nova_pm::Offline;

const INDEX: &str = "https://example.test/index/";

/// A fresh, empty directory with a fixed name, so each run replaces the last.
fn fresh(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("nova-driver-registry-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
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

fn manifest(name: &str, version: &str, extra: &str) -> String {
    format!("[package]\nname = \"{name}\"\nversion = \"{version}\"\nedition = \"2026\"\n{extra}")
}

/// A match whose second arm is unreachable: E0021, a warning.
const UNREACHABLE: &str = "pub fn pick(n: Int) -> Int {\n    match n { _ => 1, 0 => 2 }\n}\n";

/// `dir/app`, depending on `json = "1"` locked at 1.0.0, with `main`, and
/// `dir/registry` holding `json` 1.0.0 whose `lib.nova` is `json_lib`.
fn project(dir: &Path, main: &str, json_lib: &str) -> (PathBuf, Offline) {
    let app = dir.join("app");
    write(
        &app,
        &[
            (
                "nova.toml",
                manifest("app", "0.1.0", "\n[dependencies]\njson = \"1\"\n").as_str(),
            ),
            (
                "nova.lock",
                "version = 1\nindex = \"https://example.test/index/\"\n\n[[package]]\n\
                 name = \"json\"\nversion = \"1.0.0\"\nchecksum = \"00\"\ndependencies = []\n",
            ),
            ("src/main.nova", main),
        ],
    );
    let registry = dir.join("registry");
    let json = registry
        .join("src")
        .join(nova_pm::index_dir_name(INDEX))
        .join("json-1.0.0");
    write(
        &json,
        &[
            ("nova.toml", manifest("json", "1.0.0", "").as_str()),
            ("src/lib.nova", json_lib),
        ],
    );
    let offline = Offline {
        registry: Some(registry),
        lock: None,
        dev: true,
    };
    (app, offline)
}

const MAIN: &str = "import json\n\nfn main() {\n    println(\"${pick(3)}\")\n}\n";

fn codes(program: Program) -> Vec<String> {
    let analysis =
        analyze_program(program, &DiskSources, &Options::default()).expect("the entry reads");
    analysis
        .diagnostics
        .iter()
        .map(|d| d.code.clone())
        .collect()
}

#[test]
fn a_dependencys_warning_is_neither_shown_nor_counted() {
    let dir = fresh("dependency-warning");
    let (app, offline) = project(&dir, MAIN, UNREACHABLE);
    let checked =
        check_program_counted(Program::for_package_in(&app, Roots::Program, &offline)).unwrap();
    assert_eq!(
        checked,
        Checked {
            errors: 0,
            warnings: 0
        }
    );
    assert!(codes(Program::for_package_in(&app, Roots::Program, &offline)).is_empty());
}

#[test]
fn the_packages_own_warning_is_counted() {
    let dir = fresh("own-warning");
    let main = "import json\n\nfn own(n: Int) -> Int {\n    match n { _ => 1, 0 => 2 }\n}\n\n\
                fn main() {\n    println(\"${pick(own(3))}\")\n}\n";
    let (app, offline) = project(&dir, main, UNREACHABLE);
    let checked =
        check_program_counted(Program::for_package_in(&app, Roots::Program, &offline)).unwrap();
    assert_eq!(
        checked,
        Checked {
            errors: 0,
            warnings: 1
        }
    );
    assert_eq!(
        codes(Program::for_package_in(&app, Roots::Program, &offline)),
        ["E0021"]
    );
}

#[test]
fn a_dependencys_error_is_shown_and_counted() {
    let dir = fresh("dependency-error");
    let (app, offline) = project(
        &dir,
        MAIN,
        "pub fn pick(n: Int) -> Int {\n    \"nine\"\n}\n",
    );
    let checked =
        check_program_counted(Program::for_package_in(&app, Roots::Program, &offline)).unwrap();
    assert_eq!(checked.errors, 1);
    assert_eq!(
        codes(Program::for_package_in(&app, Roots::Program, &offline)).len(),
        1
    );
}

#[test]
fn a_path_dependencys_warning_is_still_shown() {
    let dir = fresh("path-warning");
    write(
        &dir.join("geom"),
        &[
            ("nova.toml", manifest("geom", "0.1.0", "").as_str()),
            ("src/lib.nova", UNREACHABLE),
        ],
    );
    let app = dir.join("app");
    write(
        &app,
        &[
            (
                "nova.toml",
                manifest(
                    "app",
                    "0.1.0",
                    "\n[dependencies]\ngeom = { path = \"../geom\" }\n",
                )
                .as_str(),
            ),
            (
                "src/main.nova",
                "import geom\n\nfn main() {\n    println(\"${pick(3)}\")\n}\n",
            ),
        ],
    );
    let offline = Offline {
        registry: Some(dir.join("registry")),
        lock: None,
        dev: true,
    };
    let checked =
        check_program_counted(Program::for_package_in(&app, Roots::Program, &offline)).unwrap();
    assert_eq!(checked.warnings, 1);
}

#[test]
fn for_package_in_reads_no_dev_dependencies_when_told() {
    let dir = fresh("no-dev");
    let app = dir.join("app");
    write(
        &app,
        &[
            (
                "nova.toml",
                manifest("app", "0.1.0", "\n[dev-dependencies]\nkit = \"1\"\n").as_str(),
            ),
            ("src/lib.nova", "pub fn one() -> Int {\n    1\n}\n"),
        ],
    );
    let with_dev = Offline {
        registry: Some(dir.join("registry")),
        lock: None,
        dev: true,
    };
    assert_eq!(
        codes(Program::for_package_in(&app, Roots::Check, &with_dev)),
        ["M0005"]
    );
    let without = Offline {
        dev: false,
        ..with_dev
    };
    let checked =
        check_program_counted(Program::for_package_in(&app, Roots::Check, &without)).unwrap();
    assert_eq!(
        checked,
        Checked {
            errors: 0,
            warnings: 0
        }
    );
}
