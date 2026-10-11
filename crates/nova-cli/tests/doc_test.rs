//! `nova test`'s doc tests, end to end (spec
//! `docs/superpowers/specs/2026-10-11-phase-3-5b-doc-tests-design.md`
//! §6, §7, §12.3).

use std::path::{Path, PathBuf};

use assert_cmd::assert::Assert;
use assert_cmd::Command;

fn nova() -> Command {
    Command::cargo_bin("nova").expect("nova binary builds")
}

/// A fresh, empty directory with a fixed name, so each run replaces the
/// last.
fn fresh(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("nova-doc-test-{name}"));
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

/// `nova test ARGS` in `dir`, with a fresh `NOVA_HOME` beside it. Colour
/// is off, as `nova` colours its diagnostics even into a pipe, and so is
/// anyhow's backtrace, which CI's `RUST_BACKTRACE=1` adds to the closing
/// error.
fn nova_test(dir: &Path, args: &[&str]) -> Assert {
    let home = dir.with_file_name(format!(
        "{}-home",
        dir.file_name().unwrap().to_string_lossy()
    ));
    let _ = std::fs::remove_dir_all(&home);
    nova()
        .current_dir(dir)
        .env("NOVA_HOME", &home)
        .env("NO_COLOR", "1")
        .env("RUST_BACKTRACE", "0")
        .arg("test")
        .args(args)
        .assert()
}

fn stdout(a: &Assert) -> String {
    String::from_utf8_lossy(&a.get_output().stdout).replace("\r\n", "\n")
}

fn stderr(a: &Assert) -> String {
    String::from_utf8_lossy(&a.get_output().stderr).replace("\r\n", "\n")
}

const MANIFEST: &str = "[package]\nname = \"calc\"\nversion = \"0.1.0\"\nedition = \"2026\"\n";

/// The library's root module: an example in its `//!` docs, one on
/// `twice`, and a test of its own.
const LIB: &str = "//! Arithmetic, checked.\n//!\n//! ```\n//! assert_eq(twice(2) + 1, 5)\n//! ```\n\n\
                   import units\n\n/// Twice `x`.\n///\n/// ```\n/// assert_eq(twice(3), 6)\n/// ```\n\
                   pub fn twice(x: Int) -> Int { x * 2 + zero() }\n\n\
                   @test\nfn twice_doubles() { assert_eq(twice(1), 2) }\n";

/// Its second module, with an example in its `//!` docs.
const UNITS: &str = "//! Units of nothing.\n//!\n//! ```\n//! assert_eq(twice(0), 0)\n//! ```\n\n\
                     /// Zero.\npub fn zero() -> Int { 0 }\n";

/// A library whose docs hold no block that runs: a `text` block, and an
/// `ignore`d one that would fail.
const NO_DOC_TESTS: &str = "/// Twice `x`:\n///\n/// ```text\n/// twice(3) is 6\n/// ```\n///\n\
                            /// ```ignore\n/// assert_eq(twice(3), 7)\n/// ```\n\
                            pub fn twice(x: Int) -> Int { x * 2 }\n\n\
                            @test\nfn twice_doubles() { assert_eq(twice(1), 2) }\n";

/// The unit section of a run of `calc`, or of `NO_DOC_TESTS`.
const UNIT_SECTION: &str = "running 1 test\n\
                            test twice_doubles ... ok\n\
                            \n\
                            test result: ok. 1 passed; 0 failed; 0 trapped; 1 total\n";

/// The doc section of a run of `calc` (spec §5.3, §6.3).
const DOC_SECTION: &str = "running 3 doc tests\n\
                           test calc (src/lib.nova:3) ... ok\n\
                           test calc::twice (src/lib.nova:11) ... ok\n\
                           test calc::units (src/units.nova:3) ... ok\n\
                           \n\
                           test result: ok. 3 passed; 0 failed; 0 trapped; 3 total\n";

/// A package `calc` in a fresh directory named for `name`, with `files`.
fn package(name: &str, files: &[(&str, &str)]) -> PathBuf {
    let dir = fresh(name).join("calc");
    write(&dir, &[("nova.toml", MANIFEST)]);
    write(&dir, files);
    dir
}

/// The library `calc`, its root module `lib`.
fn calc(name: &str, lib: &str) -> PathBuf {
    package(name, &[("src/lib.nova", lib), ("src/units.nova", UNITS)])
}

#[test]
fn a_library_runs_its_unit_tests_then_its_doc_tests() {
    let dir = calc("both", LIB);
    let a = nova_test(&dir, &[]).success();
    assert_eq!(stdout(&a), format!("{UNIT_SECTION}\n{DOC_SECTION}"));
    assert_eq!(stderr(&a), "");
}

#[test]
fn a_failing_example_fails_the_run_by_its_name() {
    // Mutant 8 (spec §12.5): a failing doc test reported as passing.
    let dir = calc("failing", &LIB.replace("twice(3), 6", "twice(3), 7"));
    let a = nova_test(&dir, &[]).failure();
    assert_eq!(
        stdout(&a),
        format!(
            "{UNIT_SECTION}\n\
             running 3 doc tests\n\
             test calc (src/lib.nova:3) ... ok\n\
             test calc::twice (src/lib.nova:11) ... FAILED\n\
             \x20   nova: panic: assertion failed: 6 != 7\n\
             test calc::units (src/units.nova:3) ... ok\n\
             \n\
             test result: FAILED. 2 passed; 1 failed; 0 trapped; 3 total\n"
        )
    );
    assert_eq!(
        stderr(&a),
        "Error: 0 tests failed, 0 trapped; 1 doc test failed, 0 trapped\n"
    );
}

#[test]
fn a_compile_error_in_an_example_points_at_its_doc_line() {
    // Spec §7, and mutant 6 (§12.5): the line map off by one line. The
    // unit section runs; the doc build's error stops the run there.
    let dir = calc(
        "compile-error",
        &LIB.replace("twice(3), 6", "twice(3), nope"),
    );
    let a = nova_test(&dir, &[]).failure();
    assert_eq!(stdout(&a), UNIT_SECTION);
    assert_eq!(
        stderr(&a),
        "error[E0001]: cannot find `nope` in this scope\n   \
         ┌─ src/lib.nova:12:25\n   \
         │\n\
         12 │ /// assert_eq(twice(3), nope)\n   \
         │                         ^^^^ here\n\
         \n\
         Error: could not compile due to 1 previous error\n"
    );
}

#[test]
fn an_unknown_word_warns_and_its_block_runs() {
    let lib = LIB.replace(
        "/// ```\n/// assert_eq(twice(3), 6)",
        "/// ```nova,should_panic\n/// assert_eq(twice(3), 6)",
    );
    let dir = calc("unknown-word", &lib);
    let a = nova_test(&dir, &[]).success();
    assert_eq!(
        stderr(&a),
        "warning: src/lib.nova:11: unknown word `should_panic` in a code block's info \
         string; the block runs as a plain test\n"
    );
    assert_eq!(stdout(&a), format!("{UNIT_SECTION}\n{DOC_SECTION}"));
}

#[test]
fn doc_alone_runs_only_the_doc_tests() {
    let dir = calc("doc-alone", LIB);
    let a = nova_test(&dir, &["--doc"]).success();
    assert_eq!(stdout(&a), DOC_SECTION);
    assert_eq!(stderr(&a), "");
}

#[test]
fn doc_outside_a_project_names_std() {
    let dir = fresh("outside");
    let a = nova_test(&dir, &["--doc"]).failure();
    assert_eq!(
        stderr(&a),
        "Error: nova test --doc tests a package's library; run it inside one, or use \
         `nova test --doc --std`\n"
    );
}

#[test]
fn a_programs_blocks_are_not_doc_tests() {
    // Spec §4.1: a package with no library has no doc tests.
    let dir = package(
        "program",
        &[(
            "src/main.nova",
            "/// Starts.\n///\n/// ```\n/// assert(false, \"never run\")\n/// ```\nfn main() {}\n",
        )],
    );
    let a = nova_test(&dir, &["--doc"]).failure();
    assert_eq!(
        stderr(&a),
        "Error: nova test --doc tests a library's docs, and this package has no \
         src/lib.nova\n"
    );
    let a = nova_test(&dir, &[]).success();
    assert_eq!(
        stdout(&a),
        "running 0 tests\n\ntest result: ok. 0 passed; 0 failed; 0 trapped; 0 total\n"
    );
}

#[test]
fn doc_with_no_doc_tests_runs_none_and_passes() {
    let dir = package("doc-none", &[("src/lib.nova", NO_DOC_TESTS)]);
    let a = nova_test(&dir, &["--doc"]).success();
    assert_eq!(
        stdout(&a),
        "running 0 doc tests\n\ntest result: ok. 0 passed; 0 failed; 0 trapped; 0 total\n"
    );
    assert_eq!(stderr(&a), "");
}

#[test]
fn a_filter_can_match_one_doc_test() {
    // Spec §6.1: a filter matches across both kinds.
    let dir = calc("filter-one", LIB);
    let a = nova_test(&dir, &["units"]).success();
    assert_eq!(
        stdout(&a),
        "running 0 tests\n\
         \n\
         test result: ok. 0 passed; 0 failed; 0 trapped; 0 total\n\
         \n\
         running 1 doc test\n\
         test calc::units (src/units.nova:3) ... ok\n\
         \n\
         test result: ok. 1 passed; 0 failed; 0 trapped; 1 total\n"
    );
}

#[test]
fn a_filter_matching_nothing_is_an_error() {
    let dir = calc("filter-none", LIB);
    let a = nova_test(&dir, &["nosuch"]).failure();
    assert_eq!(stdout(&a), "");
    assert_eq!(
        stderr(&a),
        "Error: no test name contains the filter `nosuch` (1 test and 3 doc tests available)\n"
    );
    // Under `--doc`, a unit test's name matches nothing.
    let a = nova_test(&dir, &["--doc", "twice_doubles"]).failure();
    assert_eq!(stdout(&a), "");
    assert_eq!(
        stderr(&a),
        "Error: no test name contains the filter `twice_doubles` (3 doc tests available)\n"
    );
}

#[test]
fn a_library_with_no_doc_tests_prints_what_it_printed_before() {
    // Spec §6.1: no doc section, and today's output and messages.
    let dir = package("no-doc-tests", &[("src/lib.nova", NO_DOC_TESTS)]);
    let a = nova_test(&dir, &[]).success();
    assert_eq!(stdout(&a), UNIT_SECTION);
    assert_eq!(stderr(&a), "");
    let a = nova_test(&dir, &["nosuch"]).failure();
    assert_eq!(
        stderr(&a),
        "Error: no test name contains the filter `nosuch` (1 test available)\n"
    );
    let failing = NO_DOC_TESTS.replace("twice(1), 2", "twice(1), 3");
    write(&dir, &[("src/lib.nova", &failing)]);
    let a = nova_test(&dir, &[]).failure();
    assert_eq!(
        stdout(&a),
        "running 1 test\n\
         test twice_doubles ... FAILED\n\
         \x20   nova: panic: assertion failed: 2 != 3\n\
         \n\
         test result: FAILED. 0 passed; 1 failed; 0 trapped; 1 total\n"
    );
    assert_eq!(stderr(&a), "Error: 1 test failed, 0 trapped\n");
}

#[test]
fn a_helper_that_asserts_breaks_neither_section() {
    // Spec §6.1: the doc tests read the library as `nova test` does, with
    // `std/test` in scope, so a plain helper that calls `assert_eq` still
    // compiles, with or without an example in the docs.
    const HELPER: &str = "pub fn double(x: Int) -> Int { x + x }\n\n\
                          fn check_double(x: Int) { assert_eq(double(x), x + x) }\n\n\
                          @test\nfn doubles() { check_double(2) }\n";
    const UNITS: &str = "running 1 test\n\
                         test doubles ... ok\n\
                         \n\
                         test result: ok. 1 passed; 0 failed; 0 trapped; 1 total\n";
    let dir = package("helper", &[("src/lib.nova", HELPER)]);
    let a = nova_test(&dir, &[]).success();
    assert_eq!(stdout(&a), UNITS);
    assert_eq!(stderr(&a), "");
    let a = nova_test(&dir, &["--doc"]).success();
    assert_eq!(
        stdout(&a),
        "running 0 doc tests\n\ntest result: ok. 0 passed; 0 failed; 0 trapped; 0 total\n"
    );
    assert_eq!(stderr(&a), "");
    let documented =
        format!("/// Twice `x`.\n///\n/// ```\n/// assert_eq(double(3), 6)\n/// ```\n{HELPER}");
    write(&dir, &[("src/lib.nova", &documented)]);
    let docs = "running 1 doc test\n\
                test calc::double (src/lib.nova:3) ... ok\n\
                \n\
                test result: ok. 1 passed; 0 failed; 0 trapped; 1 total\n";
    let a = nova_test(&dir, &[]).success();
    assert_eq!(stdout(&a), format!("{UNITS}\n{docs}"));
    assert_eq!(stderr(&a), "");
    let a = nova_test(&dir, &["--doc"]).success();
    assert_eq!(stdout(&a), docs);
    assert_eq!(stderr(&a), "");
}

#[test]
fn an_example_whose_main_returns_a_value_passes() {
    // Spec §5.2: a block that declares `fn main` runs as a program would,
    // whatever `main` returns.
    let lib = "/// Twice `x`.\n///\n/// ```\n/// fn main() -> Int { 1 }\n/// ```\n\
               pub fn twice(x: Int) -> Int { x * 2 }\n";
    let dir = package("main-value", &[("src/lib.nova", lib)]);
    let a = nova_test(&dir, &["--doc"]).success();
    assert_eq!(
        stdout(&a),
        "running 1 doc test\n\
         test calc::twice (src/lib.nova:3) ... ok\n\
         \n\
         test result: ok. 1 passed; 0 failed; 0 trapped; 1 total\n"
    );
    assert_eq!(stderr(&a), "");
}

#[test]
fn a_warning_is_shown_once() {
    // Spec §6.3: after the unit section, the doc build shows only what is
    // in doc tests' code; under `--doc` alone it shows the rest too.
    let lib = format!(
        "{LIB}\npub fn sign(x: Int) -> Int {{\n    match x {{\n        _ => 1\n        0 => 0\n    }}\n}}\n"
    );
    let dir = calc("warning-once", &lib);
    write(
        &dir,
        &[("nova.toml", &format!("{MANIFEST}colour = \"blue\"\n"))],
    );
    for args in [&[][..], &["--doc"][..]] {
        let a = nova_test(&dir, args).success();
        let err = stderr(&a);
        assert_eq!(err.matches("warning[E0021]").count(), 1, "{args:?}: {err}");
        assert_eq!(err.matches("warning[M0006]").count(), 1, "{args:?}: {err}");
        assert!(stdout(&a).ends_with(DOC_SECTION), "{}", stdout(&a));
    }
}

#[test]
fn doc_std_runs_stds_doc_tests_inside_a_project_or_not() {
    // Spec §6.1, §9.2, §13: std's examples run, at least one per module,
    // inside a project or not, and every one passes.
    const MODULES: [&str; 17] = [
        "bytes",
        "collections",
        "core",
        "crypto",
        "fmt",
        "fs",
        "http",
        "io",
        "json",
        "log",
        "net",
        "process",
        "strings",
        "sync",
        "task",
        "test",
        "time",
    ];
    for dir in [fresh("std-outside"), calc("std-inside", LIB)] {
        let a = nova_test(&dir, &["--doc", "--std"]).success();
        let out = stdout(&a);
        let count: usize = out
            .strip_prefix("running ")
            .and_then(|rest| rest.split(' ').next())
            .and_then(|n| n.parse().ok())
            .unwrap_or_else(|| panic!("{out}"));
        assert!(count >= 32, "only {count} of std's doc tests ran:\n{out}");
        assert!(
            out.ends_with(&format!(
                "\ntest result: ok. {count} passed; 0 failed; 0 trapped; {count} total\n"
            )),
            "{out}"
        );
        for m in MODULES {
            assert!(
                out.contains(&format!("\ntest std::{m} (std/{m}/lib.nova:")),
                "no example of std::{m} ran:\n{out}"
            );
        }
        assert!(!out.contains("calc"), "{out}");
        assert_eq!(stderr(&a), "");
    }
}

#[test]
fn std_needs_doc() {
    let dir = fresh("std-without-doc");
    let a = nova_test(&dir, &["--std"]).failure();
    assert_eq!(a.get_output().status.code(), Some(2), "a usage error");
    assert!(
        stderr(&a)
            .starts_with("error: the following required arguments were not provided:\n  --doc\n"),
        "{}",
        stderr(&a)
    );
}

/// `from` copied to `to`, without any `target/`.
fn copy(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let name = entry.file_name();
        if name == "target" {
            continue;
        }
        let path = entry.path();
        if path.is_dir() {
            copy(&path, &to.join(&name));
        } else {
            std::fs::copy(&path, to.join(&name)).unwrap();
        }
    }
}

#[test]
fn the_example_library_runs_its_doc_tests() {
    // Spec §10, §12.4: `nova test` inside a copy of the example's library.
    let shapes = fresh("example").join("shapes");
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    copy(&repo.join("examples/05-geometry/shapes"), &shapes);
    let a = nova_test(&shapes, &[]).success();
    assert_eq!(
        stdout(&a),
        "running 0 tests\n\
         \n\
         test result: ok. 0 passed; 0 failed; 0 trapped; 0 total\n\
         \n\
         running 2 doc tests\n\
         test shapes (src/lib.nova:8) ... ok\n\
         test shapes::units::Length::plus (src/units.nova:17) ... ok\n\
         \n\
         test result: ok. 2 passed; 0 failed; 0 trapped; 2 total\n"
    );
    assert_eq!(stderr(&a), "");
}
