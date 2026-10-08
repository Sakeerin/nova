//! End-to-end tests of `nova fmt` (spec
//! `docs/superpowers/specs/2026-10-07-phase-3-1-formatter-design.md` §7 and
//! §9.5).
//!
//! Every test works in its own directory under the system temp directory.
//! Each holds an `.editorconfig` that says `root = true`, so that one above
//! the temp directory cannot change a test's line endings.

use assert_cmd::Command;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

const UNFORMATTED: &str = "fn main() {\nprintln(\"hi\")\n}\n";
const FORMATTED: &str = "fn main() { println(\"hi\") }\n";

fn nova() -> Command {
    Command::cargo_bin("nova").expect("nova binary builds")
}

/// A fresh directory for one test. Its name is fixed, so each run replaces
/// the last run's.
fn fresh_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("nova-fmt-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create the test directory");
    std::fs::write(dir.join(".editorconfig"), "root = true\n").expect("write .editorconfig");
    dir
}

/// Write `text` at `rel` under `dir`, creating its directories.
fn write(dir: &Path, rel: &str, text: impl AsRef<[u8]>) -> PathBuf {
    let path = dir.join(rel);
    std::fs::create_dir_all(path.parent().unwrap()).expect("create the parent");
    std::fs::write(&path, text).expect("write the file");
    path
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).expect("read the file")
}

fn stdout(assert: &assert_cmd::assert::Assert) -> String {
    String::from_utf8_lossy(&assert.get_output().stdout).into_owned()
}

fn stderr(assert: &assert_cmd::assert::Assert) -> String {
    String::from_utf8_lossy(&assert.get_output().stderr).into_owned()
}

#[test]
fn check_fails_on_an_unformatted_file_and_passes_once_it_is_formatted() {
    let dir = fresh_dir("gate");
    let file = write(&dir, "main.nova", UNFORMATTED);
    let out = nova().arg("fmt").arg("--check").arg(&file).assert().code(1);
    assert_eq!(
        stdout(&out),
        format!("would reformat: {}\n", file.display())
    );
    assert_eq!(read(&file), UNFORMATTED, "--check writes nothing");
    nova().arg("fmt").arg(&file).assert().success().stdout("");
    assert_eq!(read(&file), FORMATTED);
    assert!(
        !dir.join(".main.nova.nova-fmt.tmp").exists(),
        "a temporary file was left"
    );
    nova()
        .arg("fmt")
        .arg("--check")
        .arg(&file)
        .assert()
        .success()
        .stdout("");
}

#[test]
fn stdin_formats_to_stdout_and_check_sets_only_the_exit_code() {
    nova()
        .args(["fmt", "--stdin"])
        .write_stdin(UNFORMATTED)
        .assert()
        .success()
        .stdout(FORMATTED);
    nova()
        .args(["fmt", "--stdin", "--check"])
        .write_stdin(UNFORMATTED)
        .assert()
        .code(1)
        .stdout("");
    nova()
        .args(["fmt", "--stdin", "--check"])
        .write_stdin(FORMATTED)
        .assert()
        .success()
        .stdout("");
}

#[test]
fn a_directory_is_searched_skipping_target_and_dot_directories() {
    let dir = fresh_dir("directories");
    let formatted = [
        write(&dir, "a.nova", UNFORMATTED),
        write(&dir, "sub/b.nova", UNFORMATTED),
    ];
    let skipped = [
        write(&dir, "target/c.nova", UNFORMATTED),
        write(&dir, ".hidden/d.nova", UNFORMATTED),
        write(&dir, "notes.txt", UNFORMATTED),
    ];
    nova().arg("fmt").arg(&dir).assert().success();
    for file in &formatted {
        assert_eq!(read(file), FORMATTED, "{}", file.display());
    }
    for file in &skipped {
        assert_eq!(read(file), UNFORMATTED, "{}", file.display());
    }
}

#[test]
fn no_path_formats_the_projects_src_even_from_a_subdirectory() {
    let dir = fresh_dir("project");
    write(
        &dir,
        "nova.toml",
        "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2026\"\n",
    );
    let main = write(&dir, "src/main.nova", UNFORMATTED);
    let more = write(&dir, "src/util/more.nova", UNFORMATTED);
    let outside = write(&dir, "scratch.nova", UNFORMATTED);
    nova()
        .arg("fmt")
        .current_dir(dir.join("src").join("util"))
        .assert()
        .success();
    assert_eq!(read(&main), FORMATTED);
    assert_eq!(read(&more), FORMATTED);
    assert_eq!(read(&outside), UNFORMATTED, "only src/ is the project's");
}

#[test]
fn no_path_outside_a_project_uses_src_or_asks_for_paths() {
    let dir = fresh_dir("no-project");
    let out = nova().arg("fmt").current_dir(&dir).assert().code(2);
    assert!(
        stderr(&out).contains("name the files or directories"),
        "{}",
        stderr(&out)
    );
    let main = write(&dir, "src/main.nova", UNFORMATTED);
    nova().arg("fmt").current_dir(&dir).assert().success();
    assert_eq!(read(&main), FORMATTED);
}

#[test]
fn a_file_with_a_syntax_error_is_left_untouched_and_the_rest_are_formatted() {
    let dir = fresh_dir("syntax-error");
    let bad = "fn main( {\n";
    let broken = write(&dir, "a.nova", bad);
    let good = write(&dir, "b.nova", UNFORMATTED);
    let out = nova().arg("fmt").arg(&dir).assert().code(2);
    assert!(stderr(&out).contains("P0001"), "{}", stderr(&out));
    assert_eq!(read(&broken), bad);
    assert_eq!(read(&good), FORMATTED);
    // With `--check`, the error outranks a file that would change.
    write(&dir, "b.nova", UNFORMATTED);
    nova().arg("fmt").arg("--check").arg(&dir).assert().code(2);
}

#[test]
fn an_already_formatted_file_is_not_rewritten() {
    let dir = fresh_dir("unchanged");
    let file = write(&dir, "main.nova", FORMATTED);
    let old = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000_000_000);
    std::fs::File::options()
        .write(true)
        .open(&file)
        .unwrap()
        .set_modified(old)
        .unwrap();
    nova().arg("fmt").arg(&file).assert().success();
    assert_eq!(std::fs::metadata(&file).unwrap().modified().unwrap(), old);
}

#[test]
fn crlf_in_gives_crlf_out() {
    let dir = fresh_dir("crlf");
    let file = write(&dir, "main.nova", UNFORMATTED.replace('\n', "\r\n"));
    nova().arg("fmt").arg(&file).assert().success();
    assert_eq!(read(&file), FORMATTED.replace('\n', "\r\n"));
}

#[test]
fn editorconfig_can_force_lf_and_keep_a_missing_final_newline() {
    let dir = fresh_dir("editorconfig");
    write(
        &dir,
        ".editorconfig",
        "root = true\n\n[*.nova]\nend_of_line = lf\n\n[keep.nova]\ninsert_final_newline = false\n",
    );
    let crlf = write(&dir, "crlf.nova", UNFORMATTED.replace('\n', "\r\n"));
    let keep = write(&dir, "keep.nova", "fn main() {\nprintln(\"hi\")\n}");
    nova().arg("fmt").arg(&dir).assert().success();
    assert_eq!(read(&crlf), FORMATTED);
    assert_eq!(read(&keep), "fn main() { println(\"hi\") }");
}

#[test]
fn nova_check_accepts_a_doc_comment_before_an_item() {
    let dir = fresh_dir("doc-comment");
    let file = write(
        &dir,
        "main.nova",
        "/// The entry point.\nfn main() {\n    println(\"hi\")\n}\n",
    );
    nova().arg("check").arg(&file).assert().success();
}

#[test]
fn a_missing_path_or_a_file_that_is_not_utf8_is_an_error() {
    let dir = fresh_dir("errors");
    let out = nova()
        .arg("fmt")
        .arg(dir.join("nope.nova"))
        .assert()
        .code(2);
    assert!(stderr(&out).contains("does not exist"), "{}", stderr(&out));
    let bytes = write(&dir, "bytes.nova", [0x66u8, 0x6e, 0xff]);
    let out = nova().arg("fmt").arg(&bytes).assert().code(2);
    assert!(stderr(&out).contains("not UTF-8"), "{}", stderr(&out));
    assert_eq!(std::fs::read(&bytes).unwrap(), [0x66u8, 0x6e, 0xff]);
}

#[test]
fn a_read_only_file_is_refused_and_left_untouched() {
    let dir = fresh_dir("write-protected");
    let file = write(&dir, "main.nova", UNFORMATTED);
    let writable = std::fs::metadata(&file).unwrap().permissions();
    let mut read_only = writable.clone();
    read_only.set_readonly(true);
    std::fs::set_permissions(&file, read_only).unwrap();
    let out = nova().arg("fmt").arg(&file).output().unwrap();
    // Writable again before any assertion, so the next run can remove it.
    std::fs::set_permissions(&file, writable).unwrap();
    let err = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(2), "{err}");
    assert!(err.contains("read-only"), "{err}");
    assert_eq!(read(&file), UNFORMATTED);
    assert!(
        !dir.join(".main.nova.nova-fmt.tmp").exists(),
        "a temporary file was left"
    );
}

#[cfg(unix)]
#[test]
fn a_symbolic_link_is_written_through_to_its_file() {
    let dir = fresh_dir("symlink");
    let real = write(&dir, "real.nova", UNFORMATTED);
    let link = dir.join("link.nova");
    std::os::unix::fs::symlink("real.nova", &link).unwrap();
    nova().arg("fmt").arg(&link).assert().success();
    let kind = std::fs::symlink_metadata(&link).unwrap().file_type();
    assert!(kind.is_symlink(), "the link was replaced by a file");
    assert_eq!(read(&real), FORMATTED);
}

#[cfg(unix)]
#[test]
fn a_files_permissions_survive_formatting() {
    use std::os::unix::fs::PermissionsExt;
    let dir = fresh_dir("permissions");
    let file = write(&dir, "main.nova", UNFORMATTED);
    std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o600)).unwrap();
    nova().arg("fmt").arg(&file).assert().success();
    assert_eq!(read(&file), FORMATTED);
    let mode = std::fs::metadata(&file).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o600, "the mode is {mode:o}");
}

#[cfg(unix)]
#[test]
fn a_stale_temporary_file_is_replaced_not_written_through() {
    let dir = fresh_dir("stale-temp");
    let file = write(&dir, "main.nova", UNFORMATTED);
    let other = write(&dir, "other.txt", "keep me\n");
    std::os::unix::fs::symlink("other.txt", dir.join(".main.nova.nova-fmt.tmp")).unwrap();
    nova().arg("fmt").arg(&file).assert().success();
    assert_eq!(read(&file), FORMATTED);
    assert_eq!(read(&other), "keep me\n");
    let kind = std::fs::symlink_metadata(&file).unwrap().file_type();
    assert!(kind.is_file(), "main.nova is no longer a regular file");
}

#[test]
fn no_path_formats_tests_too_and_skips_a_nested_package() {
    // Phase 3.3a (spec
    // docs/superpowers/specs/2026-10-08-phase-3-3a-local-packages-design.md §5.4).
    let dir = fresh_dir("tests-dir");
    let manifest = "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2026\"\n";
    write(&dir, "nova.toml", manifest);
    let main = write(&dir, "src/main.nova", UNFORMATTED);
    let test = write(&dir, "tests/api.nova", UNFORMATTED);
    write(
        &dir,
        "src/vendor/geom/nova.toml",
        manifest.replace("demo", "geom"),
    );
    let nested = write(&dir, "src/vendor/geom/src/lib.nova", UNFORMATTED);
    nova().arg("fmt").current_dir(&dir).assert().success();
    assert_eq!(read(&main), FORMATTED);
    assert_eq!(read(&test), FORMATTED);
    assert_eq!(read(&nested), UNFORMATTED, "a nested package is its own");
}
