//! End-to-end `nova run` tests — the Phase 1 gate criteria
//! (00-MASTER-SPEC.md §3, Phase 1).

use assert_cmd::Command;
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("crates/nova-cli has a repo root two levels up")
        .to_path_buf()
}

fn nova() -> Command {
    Command::cargo_bin("nova").expect("nova binary builds")
}

#[test]
fn gate_1_hello_world_runs() {
    nova()
        .arg("run")
        .arg(repo_root().join("examples/01-hello-world/src/main.nova"))
        .assert()
        .success()
        .stdout("Hello, World!\n");
}

#[test]
fn gate_2_fibonacci_runs() {
    nova()
        .arg("run")
        .arg(repo_root().join("examples/02-fibonacci/src/main.nova"))
        .assert()
        .success()
        .stdout("fibonacci(10) = 55\n");
}

#[test]
fn gate_3_match_on_enum_runs() {
    let expected = std::fs::read_to_string(repo_root().join("tests/runtime/match_enum.stdout"))
        .expect("expected-output fixture exists")
        .replace("\r\n", "\n");
    nova()
        .arg("run")
        .arg(repo_root().join("tests/runtime/match_enum.nova"))
        .assert()
        .success()
        .stdout(expected);
}

#[test]
fn gate_4_generic_functions_run() {
    let expected = std::fs::read_to_string(repo_root().join("tests/runtime/generics.stdout"))
        .expect("expected-output fixture exists")
        .replace("\r\n", "\n");
    nova()
        .arg("run")
        .arg(repo_root().join("tests/runtime/generics.nova"))
        .assert()
        .success()
        .stdout(expected);
}

#[test]
fn records_run() {
    let expected = std::fs::read_to_string(repo_root().join("tests/runtime/records.stdout"))
        .expect("expected-output fixture exists")
        .replace("\r\n", "\n");
    nova()
        .arg("run")
        .arg(repo_root().join("tests/runtime/records.nova"))
        .assert()
        .success()
        .stdout(expected);
}

#[test]
fn for_loops_run() {
    let expected = std::fs::read_to_string(repo_root().join("tests/runtime/for_loops.stdout"))
        .expect("expected-output fixture exists")
        .replace("\r\n", "\n");
    nova()
        .arg("run")
        .arg(repo_root().join("tests/runtime/for_loops.nova"))
        .assert()
        .success()
        .stdout(expected);
}

#[test]
fn arrays_run() {
    let expected = std::fs::read_to_string(repo_root().join("tests/runtime/arrays.stdout"))
        .expect("expected-output fixture exists")
        .replace("\r\n", "\n");
    nova()
        .arg("run")
        .arg(repo_root().join("tests/runtime/arrays.nova"))
        .assert()
        .success()
        .stdout(expected);
}

#[test]
fn array_out_of_bounds_aborts() {
    let dir = std::env::temp_dir().join("nova-oob");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let file = dir.join("oob.nova");
    std::fs::write(
        &file,
        "fn main() { let a = [1, 2, 3]\n println(\"${a[5]}\") }",
    )
    .expect("write");
    let exe = dir.join(format!("oob{}", std::env::consts::EXE_SUFFIX));
    nova()
        .arg("build")
        .arg(&file)
        .arg("-o")
        .arg(&exe)
        .assert()
        .success();
    let out = Command::new(&exe).assert().failure();
    let stderr = String::from_utf8_lossy(&out.get_output().stderr).to_string();
    assert!(stderr.contains("out of bounds"), "stderr: {stderr}");
}

#[test]
fn constants_run() {
    let expected = std::fs::read_to_string(repo_root().join("tests/runtime/constants.stdout"))
        .expect("expected-output fixture exists")
        .replace("\r\n", "\n");
    nova()
        .arg("run")
        .arg(repo_root().join("tests/runtime/constants.nova"))
        .assert()
        .success()
        .stdout(expected);
}

#[test]
fn break_continue_run() {
    let expected = std::fs::read_to_string(repo_root().join("tests/runtime/break_continue.stdout"))
        .expect("expected-output fixture exists")
        .replace("\r\n", "\n");
    nova()
        .arg("run")
        .arg(repo_root().join("tests/runtime/break_continue.nova"))
        .assert()
        .success()
        .stdout(expected);
}

#[test]
fn closures_run() {
    let expected = std::fs::read_to_string(repo_root().join("tests/runtime/closures.stdout"))
        .expect("expected-output fixture exists")
        .replace("\r\n", "\n");
    nova()
        .arg("run")
        .arg(repo_root().join("tests/runtime/closures.nova"))
        .assert()
        .success()
        .stdout(expected);
}

#[test]
fn closures_build_standalone() {
    let out = build_and_run("tests/runtime/closures.nova", "closures");
    let expected = std::fs::read_to_string(repo_root().join("tests/runtime/closures.stdout"))
        .expect("fixture")
        .replace("\r\n", "\n");
    assert_eq!(out, expected);
}

#[test]
fn generic_impls_run() {
    let expected = std::fs::read_to_string(repo_root().join("tests/runtime/generic_impls.stdout"))
        .expect("expected-output fixture exists")
        .replace("\r\n", "\n");
    nova()
        .arg("run")
        .arg(repo_root().join("tests/runtime/generic_impls.nova"))
        .assert()
        .success()
        .stdout(expected);
}

#[test]
fn generic_impls_build_standalone() {
    let out = build_and_run("tests/runtime/generic_impls.nova", "generic_impls");
    let expected = std::fs::read_to_string(repo_root().join("tests/runtime/generic_impls.stdout"))
        .expect("fixture")
        .replace("\r\n", "\n");
    assert_eq!(out, expected);
}

#[test]
fn traits_run() {
    let expected = std::fs::read_to_string(repo_root().join("tests/runtime/traits.stdout"))
        .expect("expected-output fixture exists")
        .replace("\r\n", "\n");
    nova()
        .arg("run")
        .arg(repo_root().join("tests/runtime/traits.nova"))
        .assert()
        .success()
        .stdout(expected);
}

#[test]
fn traits_build_standalone() {
    let out = build_and_run("tests/runtime/traits.nova", "traits");
    let expected = std::fs::read_to_string(repo_root().join("tests/runtime/traits.stdout"))
        .expect("fixture")
        .replace("\r\n", "\n");
    assert_eq!(out, expected);
}

#[test]
fn check_reports_type_errors_with_code() {
    let dir = std::env::temp_dir().join("nova-check-test");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let file = dir.join("bad.nova");
    std::fs::write(&file, "fn main() { let x: Int = \"hello\" }").expect("write test file");
    let assert = nova().arg("check").arg(&file).assert().failure();
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr).to_string();
    assert!(stderr.contains("E0010"), "stderr was: {stderr}");
}

#[test]
fn check_passes_on_valid_program() {
    nova()
        .arg("check")
        .arg(repo_root().join("examples/02-fibonacci/src/main.nova"))
        .assert()
        .success();
}

/// `nova check` must reject an unsatisfied trait bound — it runs
/// monomorphization so its "ok" verdict matches what `nova run` accepts.
#[test]
fn check_rejects_unsatisfied_trait_bound() {
    let dir = std::env::temp_dir().join("nova-check-bound");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let file = dir.join("bound.nova");
    std::fs::write(
        &file,
        "record P { v: Int }\n\
         trait Show { fn name(self) -> String }\n\
         impl Show for P { fn name(self) -> String { \"p\" } }\n\
         fn label<T: Show>(x: T) -> String { x.name() }\n\
         fn main() { println(label(5)) }\n",
    )
    .expect("write");
    let assert = nova().arg("check").arg(&file).assert().failure();
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr).to_string();
    assert!(stderr.contains("E0013"), "stderr: {stderr}");
}

// === nova build: standalone executables ===

fn build_and_run(source: &str, exe_name: &str) -> String {
    let dir = std::env::temp_dir().join("nova-build-tests");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let exe = dir.join(format!("{exe_name}{}", std::env::consts::EXE_SUFFIX));
    nova()
        .arg("build")
        .arg(repo_root().join(source))
        .arg("-o")
        .arg(&exe)
        .assert()
        .success();
    let out = Command::new(&exe)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let _ = std::fs::remove_file(&exe);
    String::from_utf8(out).expect("program output is UTF-8")
}

/// `nova build --release` with no LLVM toolchain must fail cleanly and leave
/// the generated IR behind (forcing the no-toolchain path deterministically by
/// pointing `NOVA_CLANG`/`NOVA_LLC` at a nonexistent program).
#[test]
fn release_without_toolchain_emits_ir_and_errors() {
    let dir = std::env::temp_dir().join("nova-release-notool");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let exe = dir.join(format!("hello{}", std::env::consts::EXE_SUFFIX));
    let ll = exe.with_extension("ll");
    let _ = std::fs::remove_file(&ll);
    let assert = nova()
        .arg("build")
        .arg("--release")
        .arg(repo_root().join("examples/01-hello-world/src/main.nova"))
        .arg("-o")
        .arg(&exe)
        .env("NOVA_CLANG", "nova_no_such_tool_xyz")
        .env("NOVA_LLC", "nova_no_such_tool_xyz")
        .assert()
        .failure();
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr).to_string();
    assert!(stderr.contains("LLVM toolchain"), "stderr: {stderr}");
    // The IR is left for the user to compile / inspect.
    assert!(ll.exists(), "expected generated IR at {}", ll.display());
    let ir = std::fs::read_to_string(&ll).expect("read ir");
    assert!(ir.contains("define i32 @main"), "ir:\n{ir}");
    let _ = std::fs::remove_file(&ll);
}

/// When a real LLVM toolchain is available, `--release` builds and runs an
/// optimized binary with identical output to the debug build. Skipped where
/// no `clang` is installed.
#[test]
fn release_builds_and_runs_when_clang_available() {
    let clang_ok = std::process::Command::new("clang")
        .arg("--version")
        .output()
        .is_ok();
    if !clang_ok {
        eprintln!("skipping: no clang on PATH");
        return;
    }
    let dir = std::env::temp_dir().join("nova-release-run");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let exe = dir.join(format!("hello_rel{}", std::env::consts::EXE_SUFFIX));
    nova()
        .arg("build")
        .arg("--release")
        .arg(repo_root().join("examples/01-hello-world/src/main.nova"))
        .arg("-o")
        .arg(&exe)
        .assert()
        .success();
    let out = Command::new(&exe)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    assert_eq!(String::from_utf8(out).expect("utf8"), "Hello, World!\n");
    let _ = std::fs::remove_file(&exe);
}

#[test]
fn build_hello_world_standalone() {
    let out = build_and_run("examples/01-hello-world/src/main.nova", "hello");
    assert_eq!(out, "Hello, World!\n");
}

#[test]
fn build_match_enum_standalone() {
    let out = build_and_run("tests/runtime/match_enum.nova", "match_enum");
    let expected = std::fs::read_to_string(repo_root().join("tests/runtime/match_enum.stdout"))
        .expect("fixture")
        .replace("\r\n", "\n");
    assert_eq!(out, expected);
}

#[test]
fn build_generics_standalone() {
    let out = build_and_run("tests/runtime/generics.nova", "generics");
    let expected = std::fs::read_to_string(repo_root().join("tests/runtime/generics.stdout"))
        .expect("fixture")
        .replace("\r\n", "\n");
    assert_eq!(out, expected);
}
