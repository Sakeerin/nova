//! End-to-end tests of nova's project model (spec
//! `docs/superpowers/specs/2026-10-07-phase-3-0-foundations-design.md`):
//! `nova version`, project discovery, and `nova new` and `nova init`.
//!
//! Every test works in its own fresh directory under the system temp
//! directory, and writes a `nova.toml` only inside it. Project discovery
//! walks up from the current directory, so a stray `nova.toml` in a shared
//! directory would capture every test run below it.

use assert_cmd::Command;
use std::env::consts::EXE_SUFFIX;
use std::path::{Path, PathBuf};

fn nova() -> Command {
    Command::cargo_bin("nova").expect("nova binary builds")
}

#[test]
fn version_reports_the_build_and_whether_the_runtime_is_embedded() {
    // build.rs exports the payload's size to every target of the package,
    // so this holds whether or not NOVA_EMBED_RUNTIME was set.
    let embedded = env!("NOVA_EMBEDDED_RUNTIME_SIZE") != "0";
    let expected = format!(
        "nova {}\ntarget: {}\nruntime: {}\n",
        env!("CARGO_PKG_VERSION"),
        env!("NOVA_TARGET"),
        if embedded { "embedded" } else { "not embedded" }
    );
    nova().arg("version").assert().success().stdout(expected);
}

/// A fresh, empty directory under the system temp dir, unique to this test.
fn fresh_dir(name: &str) -> PathBuf {
    // A fixed name, so each run replaces the last run's directory: a name with
    // the process id in it left one more behind on every run.
    let dir = std::env::temp_dir().join(format!("nova-project-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create the test directory");
    dir
}

/// A `nova.toml` for a package called `name`.
fn manifest(name: &str) -> String {
    format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2026\"\n")
}

/// Write a project called `name` into `dir`: its `nova.toml`, with `main`
/// as its `src/main.nova`.
fn write_project(dir: &Path, name: &str, main: &str) {
    std::fs::create_dir_all(dir.join("src")).expect("create src");
    std::fs::write(dir.join("nova.toml"), manifest(name)).expect("write nova.toml");
    std::fs::write(dir.join("src").join("main.nova"), main).expect("write src/main.nova");
}

fn stdout(assert: &assert_cmd::assert::Assert) -> String {
    String::from_utf8_lossy(&assert.get_output().stdout).into_owned()
}

fn stderr(assert: &assert_cmd::assert::Assert) -> String {
    String::from_utf8_lossy(&assert.get_output().stderr).into_owned()
}

/// `<dir>/target/<profile>/<name>`, with the platform's executable suffix.
fn built(dir: &Path, profile: &str, name: &str) -> PathBuf {
    dir.join("target")
        .join(profile)
        .join(format!("{name}{EXE_SUFFIX}"))
}

const HELLO: &str = "fn main() {\n    println(\"hello from the project\")\n}\n";

const WITH_TEST: &str =
    "fn main() {\n    println(\"main\")\n}\n\n@test\nfn passes() {\n    assert_eq(1 + 1, 2)\n}\n";

const PRINT_FIRST_ARG: &str = "fn main() {\n    let argv = args()\n    match argv.get(0) {\n        Some(s) => println(s)\n        None => println(\"no arguments\")\n    }\n}\n";

#[test]
fn every_command_finds_the_project_from_a_subdirectory() {
    let dir = fresh_dir("subdirectory");
    write_project(&dir, "demo", WITH_TEST);
    let sub = dir.join("src");
    nova()
        .current_dir(&sub)
        .arg("run")
        .assert()
        .success()
        .stdout("main\n");
    nova().current_dir(&sub).arg("check").assert().success();
    let tested = nova().current_dir(&sub).arg("test").assert().success();
    assert!(
        stdout(&tested).contains("1 passed; 0 failed"),
        "{}",
        stdout(&tested)
    );
    nova().current_dir(&sub).arg("build").assert().success();
    Command::new(built(&dir, "debug", "demo"))
        .assert()
        .success()
        .stdout("main\n");
}

#[test]
fn build_writes_target_debug_and_the_output_flag_overrides_it() {
    let dir = fresh_dir("build-output");
    write_project(&dir, "demo", HELLO);
    let out = nova().current_dir(&dir).arg("build").assert().success();
    let relative = Path::new("target")
        .join("debug")
        .join(format!("demo{EXE_SUFFIX}"));
    assert_eq!(stdout(&out), format!("built {}\n", relative.display()));
    Command::new(built(&dir, "debug", "demo"))
        .assert()
        .success()
        .stdout("hello from the project\n");
    let custom = dir.join(format!("custom{EXE_SUFFIX}"));
    nova()
        .current_dir(&dir)
        .arg("build")
        .arg("-o")
        .arg(&custom)
        .assert()
        .success();
    Command::new(&custom)
        .assert()
        .success()
        .stdout("hello from the project\n");
}

/// Like `release_builds_and_runs_when_clang_available` in run_tests.rs, this
/// skips when no `clang` is on PATH.
#[test]
fn build_release_writes_target_release() {
    let clang = std::process::Command::new("clang")
        .arg("--version")
        .output()
        .is_ok();
    if !clang {
        eprintln!("skipping: no clang on PATH");
        return;
    }
    let dir = fresh_dir("build-release");
    write_project(&dir, "demo", HELLO);
    nova()
        .current_dir(&dir)
        .args(["build", "--release"])
        .assert()
        .success();
    Command::new(built(&dir, "release", "demo"))
        .assert()
        .success()
        .stdout("hello from the project\n");
}

#[test]
fn paths_are_relative_at_the_root_and_absolute_from_a_subdirectory() {
    let dir = fresh_dir("paths");
    write_project(&dir, "demo", PRINT_FIRST_ARG);
    nova()
        .current_dir(&dir)
        .arg("run")
        .assert()
        .success()
        .stdout("src/main.nova\n");

    let sub = dir.join("src");
    let ran = nova().current_dir(&sub).arg("run").assert().success();
    let entry = PathBuf::from(stdout(&ran).trim_end());
    assert!(entry.is_absolute(), "{}", entry.display());
    assert_eq!(
        entry.canonicalize().unwrap(),
        dir.join("src").join("main.nova").canonicalize().unwrap()
    );

    let out = nova().current_dir(&sub).arg("build").assert().success();
    let printed = stdout(&out);
    let output = PathBuf::from(
        printed
            .trim_end()
            .strip_prefix("built ")
            .expect("`built <path>`"),
    );
    assert!(output.is_absolute(), "{}", output.display());
    assert_eq!(
        output.canonicalize().unwrap(),
        built(&dir, "debug", "demo").canonicalize().unwrap()
    );
}

/// Phase 3.3a: a package with neither target is M0013 (spec
/// `docs/superpowers/specs/2026-10-08-phase-3-3a-local-packages-design.md`
/// §3.1).
#[test]
fn a_project_without_src_main_nova_names_the_missing_entry() {
    let dir = fresh_dir("no-entry");
    std::fs::write(dir.join("nova.toml"), manifest("demo")).unwrap();
    let out = nova().current_dir(&dir).arg("run").assert().failure();
    let err = stderr(&out);
    assert!(
        err.contains("M0013") && err.contains("has neither src/lib.nova nor src/main.nova"),
        "{err}"
    );
}

/// Passes before project mode exists, and must still pass after it: a file
/// argument never reads the manifest.
#[test]
fn a_file_argument_inside_a_project_ignores_even_a_broken_manifest() {
    let dir = fresh_dir("file-argument");
    std::fs::write(dir.join("nova.toml"), "this is not [ valid toml").unwrap();
    std::fs::write(dir.join("hello.nova"), HELLO).unwrap();
    nova()
        .current_dir(&dir)
        .args(["run", "hello.nova"])
        .assert()
        .success()
        .stdout("hello from the project\n");
    nova()
        .current_dir(&dir)
        .args(["build", "hello.nova"])
        .assert()
        .success();
    assert!(dir.join(format!("hello{EXE_SUFFIX}")).is_file());
}

#[test]
fn a_declared_dependency_is_m0005() {
    let dir = fresh_dir("dependency");
    write_project(&dir, "demo", HELLO);
    let with_dependency = format!("{}\n[dependencies]\nhttp = \"1.0\"\n", manifest("demo"));
    std::fs::write(dir.join("nova.toml"), with_dependency).unwrap();
    let out = nova().current_dir(&dir).arg("run").assert().failure();
    let err = stderr(&out);
    assert!(err.contains("M0005") && err.contains("`http`"), "{err}");
}

#[test]
fn a_manifest_error_shows_its_line_and_column() {
    let dir = fresh_dir("manifest-error");
    write_project(&dir, "demo", HELLO);
    std::fs::write(
        dir.join("nova.toml"),
        manifest("demo").replace("2026", "2021"),
    )
    .unwrap();
    let out = nova().current_dir(&dir).arg("build").assert().failure();
    let err = stderr(&out);
    assert!(
        err.contains("M0003") && err.contains("nova.toml:4:11"),
        "{err}"
    );
}

#[test]
fn an_unknown_key_warns_and_the_command_proceeds() {
    let dir = fresh_dir("unknown-key");
    write_project(&dir, "demo", HELLO);
    let with_features = format!("{}\n[features]\ndefault = []\n", manifest("demo"));
    std::fs::write(dir.join("nova.toml"), with_features).unwrap();
    let out = nova()
        .current_dir(&dir)
        .arg("run")
        .assert()
        .success()
        .stdout("hello from the project\n");
    let err = stderr(&out);
    assert!(err.contains("M0006") && err.contains("`features`"), "{err}");
}

/// Passes before project mode exists, and must still pass after it.
#[test]
fn outside_any_project_src_main_nova_stays_the_default() {
    let dir = fresh_dir("no-project");
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::write(dir.join("src").join("main.nova"), HELLO).unwrap();
    nova()
        .current_dir(&dir)
        .arg("run")
        .assert()
        .success()
        .stdout("hello from the project\n");
    nova().current_dir(&dir).arg("build").assert().success();
    assert!(dir.join(format!("main{EXE_SUFFIX}")).is_file());
}

/// Review Focus 1.
#[test]
fn a_project_under_a_path_with_spaces_and_thai_letters_works() {
    let dir = fresh_dir("spaces").join("โปรเจกต์ ของ ฉัน");
    write_project(&dir, "demo", WITH_TEST);
    nova()
        .current_dir(&dir)
        .arg("run")
        .assert()
        .success()
        .stdout("main\n");
    nova().current_dir(&dir).arg("build").assert().success();
    Command::new(built(&dir, "debug", "demo"))
        .assert()
        .success()
        .stdout("main\n");
    let tested = nova().current_dir(&dir).arg("test").assert().success();
    assert!(
        stdout(&tested).contains("1 passed; 0 failed"),
        "{}",
        stdout(&tested)
    );
}

/// Review Focus 3.
#[test]
fn an_import_resolves_when_run_from_a_subdirectory() {
    let dir = fresh_dir("import");
    write_project(
        &dir,
        "demo",
        "import util::{greeting}\n\nfn main() {\n    println(greeting())\n}\n",
    );
    std::fs::write(
        dir.join("src").join("util.nova"),
        "pub fn greeting() -> String {\n    \"from util\"\n}\n",
    )
    .unwrap();
    let docs = dir.join("docs");
    std::fs::create_dir_all(&docs).unwrap();
    nova()
        .current_dir(&docs)
        .arg("run")
        .assert()
        .success()
        .stdout("from util\n");
    nova().current_dir(&docs).arg("build").assert().success();
    Command::new(built(&dir, "debug", "demo"))
        .assert()
        .success()
        .stdout("from util\n");
}

const TEMPLATE_MAIN: &str = "fn greeting() -> String {\n    \"Hello, Nova!\"\n}\n\nfn main() {\n    println(greeting())\n}\n\n@test\nfn greeting_says_hello() {\n    assert_eq(greeting(), \"Hello, Nova!\")\n}\n";

fn read(path: impl AsRef<Path>) -> String {
    let path = path.as_ref();
    std::fs::read_to_string(path).unwrap_or_else(|e| panic!("reading {}: {e}", path.display()))
}

#[test]
fn new_writes_exactly_the_template_and_it_runs_and_tests() {
    let dir = fresh_dir("new");
    let out = nova()
        .current_dir(&dir)
        .args(["new", "demo"])
        .assert()
        .success();
    assert_eq!(
        stdout(&out),
        "created `demo`: nova.toml, .gitignore, README.md, src/main.nova\n"
    );
    let project = dir.join("demo");
    assert_eq!(
        read(project.join("nova.toml")),
        "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2026\"\n\n[dependencies]\n"
    );
    assert_eq!(read(project.join(".gitignore")), "/target\n");
    assert_eq!(
        read(project.join("README.md")),
        "# demo\n\n`nova run` builds and runs `src/main.nova`; `nova test` runs its tests.\n"
    );
    assert_eq!(read(project.join("src").join("main.nova")), TEMPLATE_MAIN);
    nova()
        .current_dir(&project)
        .arg("run")
        .assert()
        .success()
        .stdout("Hello, Nova!\n");
    let tested = nova().current_dir(&project).arg("test").assert().success();
    assert!(
        stdout(&tested).contains("1 passed; 0 failed"),
        "{}",
        stdout(&tested)
    );
}

#[test]
fn new_refuses_a_non_empty_directory_and_bad_names() {
    let dir = fresh_dir("new-refuses");
    std::fs::create_dir_all(dir.join("taken")).unwrap();
    std::fs::write(dir.join("taken").join("file.txt"), "x").unwrap();
    let out = nova()
        .current_dir(&dir)
        .args(["new", "taken"])
        .assert()
        .failure();
    assert!(
        stderr(&out).contains("not an empty directory"),
        "{}",
        stderr(&out)
    );
    assert!(!dir.join("taken").join("nova.toml").exists());

    // An empty directory is fine.
    std::fs::create_dir_all(dir.join("empty")).unwrap();
    nova()
        .current_dir(&dir)
        .args(["new", "empty"])
        .assert()
        .success();
    assert!(dir.join("empty").join("nova.toml").is_file());

    // Review Focus 5: a bad name is refused before anything touches the
    // disk, names that look like paths included.
    for bad in ["1abc", "a b", "con", "../nova-escape-test", "a/b"] {
        let out = nova()
            .current_dir(&dir)
            .args(["new", bad])
            .assert()
            .failure();
        let err = stderr(&out);
        assert!(
            err.contains("package name") || err.contains("reserves"),
            "{bad}: {err}"
        );
    }
    let mut left: Vec<String> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    left.sort();
    assert_eq!(left, ["empty", "taken"]);
    assert!(!dir.parent().unwrap().join("nova-escape-test").exists());
}

#[test]
fn init_names_the_project_after_its_directory_and_reports_what_it_wrote() {
    let dir = fresh_dir("init").join("my-app");
    std::fs::create_dir_all(&dir).unwrap();
    let out = nova().current_dir(&dir).arg("init").assert().success();
    assert_eq!(
        stdout(&out),
        "wrote: nova.toml, .gitignore, README.md, src/main.nova\n"
    );
    assert!(read(dir.join("nova.toml")).contains("name = \"my-app\""));
    nova()
        .current_dir(&dir)
        .arg("run")
        .assert()
        .success()
        .stdout("Hello, Nova!\n");
}

#[test]
fn init_keeps_existing_files_and_says_so() {
    let dir = fresh_dir("init-keeps").join("keeper");
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::write(dir.join("src").join("main.nova"), HELLO).unwrap();
    std::fs::write(dir.join("README.md"), "mine\n").unwrap();
    let out = nova().current_dir(&dir).arg("init").assert().success();
    assert_eq!(
        stdout(&out),
        "wrote: nova.toml, .gitignore\nkept: README.md, src/main.nova\n"
    );
    assert_eq!(read(dir.join("src").join("main.nova")), HELLO);
    assert_eq!(read(dir.join("README.md")), "mine\n");
    nova()
        .current_dir(&dir)
        .arg("run")
        .assert()
        .success()
        .stdout("hello from the project\n");
}

#[test]
fn init_refuses_a_directory_that_already_has_a_manifest() {
    let dir = fresh_dir("init-refuses").join("existing");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("nova.toml"), "# mine\n").unwrap();
    let out = nova().current_dir(&dir).arg("init").assert().failure();
    assert!(
        stderr(&out).contains("already has a nova.toml"),
        "{}",
        stderr(&out)
    );
    assert_eq!(read(dir.join("nova.toml")), "# mine\n");
    assert!(!dir.join("src").exists());
}

#[test]
fn init_suggests_the_name_flag_for_a_bad_directory_name() {
    let dir = fresh_dir("init-bad-name").join("1 bad name");
    std::fs::create_dir_all(&dir).unwrap();
    let out = nova().current_dir(&dir).arg("init").assert().failure();
    assert!(
        stderr(&out).contains("nova init --name"),
        "{}",
        stderr(&out)
    );
    assert!(!dir.join("nova.toml").exists());
    nova()
        .current_dir(&dir)
        .args(["init", "--name", "good"])
        .assert()
        .success();
    assert!(read(dir.join("nova.toml")).contains("name = \"good\""));
}
