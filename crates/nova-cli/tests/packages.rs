//! Packages end to end through `nova` (spec
//! `docs/superpowers/specs/2026-10-08-phase-3-3a-local-packages-design.md`
//! §5, §7.3).

use std::path::{Path, PathBuf};

use assert_cmd::Command;

fn nova() -> Command {
    Command::cargo_bin("nova").expect("nova binary builds")
}

/// A fresh, empty directory with a fixed name, so each run replaces the last.
fn fresh(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("nova-packages-{name}"));
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

/// A `nova.toml` for `name`, with `extra` appended.
fn manifest(name: &str, extra: &str) -> String {
    format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2026\"\n{extra}")
}

fn stdout(assert: &assert_cmd::assert::Assert) -> String {
    String::from_utf8_lossy(&assert.get_output().stdout).into_owned()
}

fn stderr(assert: &assert_cmd::assert::Assert) -> String {
    String::from_utf8_lossy(&assert.get_output().stderr).into_owned()
}

const GEOM_DEPENDENCY: &str = "\n[dependencies]\ngeom = { path = \"../geom\" }\n";

/// `dir/geom`: a library whose `lib.nova` uses its own `utils.nova`.
fn geom(dir: &Path) {
    write(
        &dir.join("geom"),
        &[
            ("nova.toml", manifest("geom", "").as_str()),
            (
                "src/lib.nova",
                "import utils\n\npub fn area() -> Int {\n    width() * width()\n}\n",
            ),
            ("src/utils.nova", "pub fn width() -> Int {\n    3\n}\n"),
        ],
    );
}

/// `dir/app`, which depends on `geom` and has its own `utils.nova`.
fn app(dir: &Path) -> PathBuf {
    let app = dir.join("app");
    write(
        &app,
        &[
            ("nova.toml", manifest("app", GEOM_DEPENDENCY).as_str()),
            (
                "src/main.nova",
                "import geom\nimport utils\n\nfn main() {\n    println(\"area ${area()}\")\n    println(label())\n}\n",
            ),
            ("src/utils.nova", "pub fn label() -> String {\n    \"app utils\"\n}\n"),
        ],
    );
    app
}

#[test]
fn an_app_runs_and_builds_against_a_path_library() {
    let dir = fresh("run");
    geom(&dir);
    let app = app(&dir);
    nova()
        .current_dir(&app)
        .arg("run")
        .assert()
        .success()
        .stdout("area 9\napp utils\n");
    nova().current_dir(&app).arg("build").assert().success();
    let exe = app
        .join("target")
        .join("debug")
        .join(format!("app{}", std::env::consts::EXE_SUFFIX));
    let out = std::process::Command::new(&exe)
        .output()
        .expect("run the build");
    assert_eq!(String::from_utf8_lossy(&out.stdout), "area 9\napp utils\n");
}

#[test]
fn run_and_build_are_refused_in_a_library() {
    let dir = fresh("refused");
    geom(&dir);
    for command in ["run", "build"] {
        let out = nova()
            .current_dir(dir.join("geom"))
            .arg(command)
            .assert()
            .failure();
        let err = stderr(&out);
        assert!(
            err.contains(
                "`geom` is a library: it has no src/main.nova; `nova check` and `nova test` work on it"
            ),
            "{command}: {err}"
        );
    }
}

#[test]
fn check_checks_the_library_and_the_program() {
    let dir = fresh("check");
    write(
        &dir,
        &[
            ("nova.toml", manifest("both", "").as_str()),
            ("src/main.nova", "fn main() {}\n"),
            // `main.nova` does not import it.
            ("src/lib.nova", "pub fn area() -> Int {\n    \"nine\"\n}\n"),
        ],
    );
    let out = nova().current_dir(&dir).arg("check").assert().failure();
    let err = stderr(&out);
    assert!(err.contains("lib.nova:"), "{err}");
    geom(&dir);
    nova()
        .current_dir(dir.join("geom"))
        .arg("check")
        .assert()
        .success()
        .stdout("ok: src/lib.nova\n");
}

#[test]
fn test_runs_the_library_and_the_tests_directory() {
    let dir = fresh("test");
    geom(&dir);
    let geom = dir.join("geom");
    write(
        &geom,
        &[
            (
                "src/lib.nova",
                "import utils\n\npub fn area() -> Int {\n    width() * width()\n}\n\n\
                 @test\nfn area_is_nine() {\n    assert_eq(area(), 9)\n}\n",
            ),
            (
                "tests/api.nova",
                "import geom\n\n@test\nfn parses() {\n    assert_eq(area(), 9)\n}\n",
            ),
        ],
    );
    let out = nova().current_dir(&geom).arg("test").assert().success();
    let printed = stdout(&out);
    assert!(printed.contains("running 2 tests"), "{printed}");
    assert!(printed.contains("area_is_nine ... ok"), "{printed}");
    assert!(printed.contains("parses ... ok"), "{printed}");
}

#[test]
fn each_graph_error_is_rendered_and_stops_the_command() {
    let dir = fresh("graph-errors");
    geom(&dir);
    write(
        &dir.join("program"),
        &[
            ("nova.toml", manifest("program", "").as_str()),
            ("src/main.nova", "fn main() {}\n"),
        ],
    );
    write(
        &dir.join("geometry"),
        &[
            ("nova.toml", manifest("geometry", "").as_str()),
            ("src/lib.nova", ""),
        ],
    );
    let cases = [
        ("M0005", "\n[dependencies]\nhttp = \"1.0\"\n"),
        (
            "M0007",
            "\n[dependencies]\ngeom = { path = \"../nowhere\" }\n",
        ),
        (
            "M0008",
            "\n[dependencies]\ngeom = { path = \"../geometry\" }\n",
        ),
        (
            "M0009",
            "\n[dependencies]\nprogram = { path = \"../program\" }\n",
        ),
        ("M0010", "\n[dependencies]\nme = { path = \".\" }\n"),
        (
            "M0012",
            "\n[dependencies]\nmatch = { path = \"../geom\" }\n",
        ),
    ];
    for (code, extra) in cases {
        let app = dir.join(format!("app-{code}"));
        write(
            &app,
            &[
                ("nova.toml", manifest("app", extra).as_str()),
                ("src/main.nova", "fn main() {}\n"),
            ],
        );
        let out = nova().current_dir(&app).arg("check").assert().failure();
        let err = stderr(&out);
        assert!(err.contains(code), "{code}: {err}");
    }
    // M0011: two packages named `c`.
    write(
        &dir.join("c1"),
        &[
            ("nova.toml", manifest("c", "").as_str()),
            ("src/lib.nova", ""),
        ],
    );
    write(
        &dir.join("c2"),
        &[
            ("nova.toml", manifest("c", "").as_str()),
            ("src/lib.nova", ""),
        ],
    );
    write(
        &dir.join("a"),
        &[
            (
                "nova.toml",
                manifest("a", "\n[dependencies]\nc = { path = \"../c1\" }\n").as_str(),
            ),
            ("src/lib.nova", ""),
        ],
    );
    write(
        &dir.join("b"),
        &[
            (
                "nova.toml",
                manifest("b", "\n[dependencies]\nc = { path = \"../c2\" }\n").as_str(),
            ),
            ("src/lib.nova", ""),
        ],
    );
    let both = "\n[dependencies]\na = { path = \"../a\" }\nb = { path = \"../b\" }\n";
    let app = dir.join("app-M0011");
    write(
        &app,
        &[
            ("nova.toml", manifest("app", both).as_str()),
            ("src/main.nova", "fn main() {}\n"),
        ],
    );
    let out = nova().current_dir(&app).arg("run").assert().failure();
    assert!(stderr(&out).contains("M0011"), "{}", stderr(&out));
    // M0013: neither target.
    let empty = dir.join("empty");
    write(&empty, &[("nova.toml", manifest("empty", "").as_str())]);
    let out = nova().current_dir(&empty).arg("test").assert().failure();
    assert!(stderr(&out).contains("M0013"), "{}", stderr(&out));
}

#[test]
fn a_file_argument_in_src_reads_its_packages_manifest() {
    // A guard: the driver's `Program::for_file` (Task 4) already does this.
    let dir = fresh("file-argument");
    geom(&dir);
    let app = app(&dir);
    nova()
        .current_dir(&app)
        .args(["run", "src/main.nova"])
        .assert()
        .success()
        .stdout("area 9\napp utils\n");
}
