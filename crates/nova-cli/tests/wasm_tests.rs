//! The WASM target, end to end (spec
//! `docs/superpowers/specs/2026-10-14-phase-4-0-foundations-design.md`).

use std::path::{Path, PathBuf};

use assert_cmd::assert::Assert;
use assert_cmd::Command;

fn nova() -> Command {
    Command::cargo_bin("nova").expect("nova binary builds")
}

/// A fresh, empty directory with a fixed name, so each run replaces the
/// last.
fn fresh(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("nova-wasm-test-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create the test directory");
    dir
}

fn write(dir: &Path, files: &[(&str, &str)]) {
    for (path, text) in files {
        let path = dir.join(path);
        std::fs::create_dir_all(path.parent().expect("a parent")).expect("create a directory");
        std::fs::write(path, text).expect("write a file");
    }
}

/// `nova ARGS` in `dir`, with a fresh `NOVA_HOME` beside it, no colour and
/// no backtrace.
fn nova_in(dir: &Path, args: &[&str]) -> Assert {
    let home = dir.with_file_name(format!(
        "{}-home",
        dir.file_name().expect("a name").to_string_lossy()
    ));
    let _ = std::fs::remove_dir_all(&home);
    nova()
        .current_dir(dir)
        .env("NOVA_HOME", &home)
        .env("NO_COLOR", "1")
        .env("RUST_BACKTRACE", "0")
        .args(args)
        .assert()
}

fn stdout(a: &Assert) -> String {
    String::from_utf8_lossy(&a.get_output().stdout).replace("\r\n", "\n")
}

fn stderr(a: &Assert) -> String {
    String::from_utf8_lossy(&a.get_output().stderr).replace("\r\n", "\n")
}

/// A program WASM can build (spec §3.1).
#[test]
fn check_for_wasm_accepts_a_program() {
    let dir = fresh("check-ok");
    write(
        &dir,
        &[(
            "main.nova",
            "fn main() {\n    let n = 6 * 7\n    println(\"${n}\")\n}\n",
        )],
    );
    let a = nova_in(&dir, &["check", "--target", "wasm", "main.nova"]).success();
    assert_eq!(stdout(&a), "ok: main.nova\n");
}

/// `extern` is E0090 on WASM, and fine natively (spec §3.3).
#[test]
fn check_for_wasm_refuses_an_extern_block() {
    let dir = fresh("check-extern");
    write(
        &dir,
        &[(
            "main.nova",
            "extern \"C\" {\n    fn abs(x: Int) -> Int\n}\nfn main() { println(\"${abs(-3)}\") }\n",
        )],
    );
    let a = nova_in(&dir, &["check", "--target", "wasm", "main.nova"]).failure();
    assert!(
        stderr(&a).contains("error[E0090]: extern functions are not available on target wasm"),
        "{}",
        stderr(&a)
    );
    nova_in(&dir, &["check", "--target", "native", "main.nova"]).success();
    nova_in(&dir, &["check", "main.nova"]).success();
}

/// A native-only module is not compiled for WASM, so its names do not
/// resolve (spec §3.2).
#[test]
fn check_for_wasm_has_no_native_only_module() {
    let dir = fresh("check-process");
    write(
        &dir,
        &[("main.nova", "fn main() { println(\"${args().len()}\") }\n")],
    );
    let a = nova_in(&dir, &["check", "--target", "wasm", "main.nova"]).failure();
    assert!(
        stderr(&a).contains("error[E0001]: cannot find function `args` in this scope"),
        "{}",
        stderr(&a)
    );
    nova_in(&dir, &["check", "main.nova"]).success();
}

/// The memory builtins are `std/rt`'s: a user module cannot call one, even
/// on WASM (spec §3.3).
#[test]
fn a_user_module_cannot_call_a_memory_builtin() {
    let dir = fresh("check-mem");
    write(
        &dir,
        &[("main.nova", "fn main() { println(\"${mem_load64(8)}\") }\n")],
    );
    let a = nova_in(&dir, &["check", "--target", "wasm", "main.nova"]).failure();
    assert!(
        stderr(&a).contains("error[E0001]: cannot find function `mem_load64` in this scope"),
        "{}",
        stderr(&a)
    );
}

/// A program that needs a runtime function WASM lacks is E0091, from
/// `check` as from `build` (spec §3.3).
#[test]
fn check_for_wasm_reports_a_runtime_function_wasm_lacks() {
    let dir = fresh("check-async");
    write(
        &dir,
        &[("main.nova", "async fn main() { println(\"a\") }\n")],
    );
    let a = nova_in(&dir, &["check", "--target", "wasm", "main.nova"]).failure();
    assert!(
        stderr(&a).contains(
            "error[E0091]: this program needs `task_block_on`, which target wasm does not have"
        ),
        "{}",
        stderr(&a)
    );
    nova_in(&dir, &["check", "main.nova"]).success();
}
