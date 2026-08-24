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
fn field_assign_run() {
    let expected = std::fs::read_to_string(repo_root().join("tests/runtime/field_assign.stdout"))
        .expect("expected-output fixture exists")
        .replace("\r\n", "\n");
    nova()
        .arg("run")
        .arg(repo_root().join("tests/runtime/field_assign.nova"))
        .assert()
        .success()
        .stdout(expected);
}

#[test]
fn field_assign_build_standalone() {
    let expected = std::fs::read_to_string(repo_root().join("tests/runtime/field_assign.stdout"))
        .expect("expected-output fixture exists")
        .replace("\r\n", "\n");
    let out = build_and_run("tests/runtime/field_assign.nova", "field_assign");
    assert_eq!(out.replace("\r\n", "\n"), expected);
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
fn array_repeat_run() {
    let expected = std::fs::read_to_string(repo_root().join("tests/runtime/array_repeat.stdout"))
        .expect("expected-output fixture exists")
        .replace("\r\n", "\n");
    nova()
        .arg("run")
        .arg(repo_root().join("tests/runtime/array_repeat.nova"))
        .assert()
        .success()
        .stdout(expected);
}

#[test]
fn array_repeat_build_standalone() {
    let expected = std::fs::read_to_string(repo_root().join("tests/runtime/array_repeat.stdout"))
        .expect("expected-output fixture exists")
        .replace("\r\n", "\n");
    let out = build_and_run("tests/runtime/array_repeat.nova", "array_repeat");
    assert_eq!(out.replace("\r\n", "\n"), expected);
}

/// A negative repeat length is guarded, not clamped: `[x; -1]` aborts at the
/// allocation with a message naming the problem, rather than silently yielding
/// an empty array whose first index then fails somewhere that looks fine.
/// Mirrors `array_out_of_bounds_aborts`, the same abort-on-bad-input policy.
#[test]
fn repeat_array_negative_length_aborts() {
    let dir = std::env::temp_dir().join("nova-neg-len");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let file = dir.join("neg_len.nova");
    std::fs::write(
        &file,
        "fn main() { let n = 0 - 1\n let a = [7; n]\n println(\"${a.len()}\") }",
    )
    .expect("write");
    let exe = dir.join(format!("neg_len{}", std::env::consts::EXE_SUFFIX));
    nova()
        .arg("build")
        .arg(&file)
        .arg("-o")
        .arg(&exe)
        .assert()
        .success();
    let out = Command::new(&exe).assert().failure();
    let stderr = String::from_utf8_lossy(&out.get_output().stderr).to_string();
    assert!(
        stderr.contains("array length must not be negative"),
        "stderr: {stderr}"
    );
}

/// An *overlong* repeat length is guarded too — and unlike the negative case
/// this one was memory-unsafe, not merely confusing. Both backends compute the
/// allocation size as `8 * len + 8` with wrapping arithmetic, so at
/// `len = 2^60` the size wraps to `i64::MIN + 8`, `gc::alloc`'s `size.max(8)`
/// clamps it to an **8-byte** block, the huge length is written into that
/// block's header, and the fill loop (deliberately unchecked) then writes far
/// past the end. This program used to exit 139 (SIGSEGV) with no output.
///
/// `[init; n]` is the only way to allocate a runtime-length array, so `n` is by
/// design a computed value and this is reachable from ordinary code.
#[test]
fn repeat_array_overlong_length_aborts_instead_of_segfaulting() {
    let dir = std::env::temp_dir().join("nova-huge-len");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let file = dir.join("huge_len.nova");
    // `1 << 60`, written as a shift so no literal-overflow question arises.
    std::fs::write(
        &file,
        "fn main() { let n = 1 << 60\n let a = [7; n]\n println(\"${a.len()}\") }",
    )
    .expect("write");
    let exe = dir.join(format!("huge_len{}", std::env::consts::EXE_SUFFIX));
    nova()
        .arg("build")
        .arg(&file)
        .arg("-o")
        .arg(&exe)
        .assert()
        .success();
    let out = Command::new(&exe).assert().failure();
    let stderr = String::from_utf8_lossy(&out.get_output().stderr).to_string();
    assert!(
        stderr.contains("array length is too large"),
        "stderr: {stderr}"
    );
    // The JIT path shares the lowering, but assert it too: a segfault there
    // would take the compiler process down rather than a child.
    let assert = nova().arg("run").arg(&file).assert().failure();
    let jit_stderr = String::from_utf8_lossy(&assert.get_output().stderr).to_string();
    assert!(
        jit_stderr.contains("array length is too large"),
        "stderr: {jit_stderr}"
    );
}

/// The very top of the *legal* length range is a clean Nova abort, not a Rust
/// one. `MAX_ARRAY_LEN` is the largest length whose `8 * len + 8` size still fits
/// in an `i64`, so the lowering's guards both pass — but at `ALIGN = 16` that
/// size rounds up 8 bytes past `isize::MAX`, so `Layout::from_size_align` cannot
/// describe it. That used to reach an `expect` in `gc::alloc` and end the
/// process with "thread caused non-unwinding panic" plus a Rust backtrace.
///
/// The check happens before any allocation is attempted, so this test costs
/// nothing in memory. `gc::alloc` is shared by every allocation site in the
/// language, so this covers records, strings, closures and collection growth
/// too — arrays are just the one path that can name such a size in one line.
#[test]
fn undescribable_allocation_size_aborts_cleanly() {
    let dir = std::env::temp_dir().join("nova-max-len");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let file = dir.join("max_len.nova");
    // `(1 << 60) - 2` is exactly `MAX_ARRAY_LEN`, written as a shift so no
    // literal-overflow question arises.
    std::fs::write(
        &file,
        "fn main() { let n = (1 << 60) - 2\n let a = [7; n]\n println(\"${a.len()}\") }",
    )
    .expect("write");
    let exe = dir.join(format!("max_len{}", std::env::consts::EXE_SUFFIX));
    nova()
        .arg("build")
        .arg(&file)
        .arg("-o")
        .arg(&exe)
        .assert()
        .success();
    let out = Command::new(&exe).assert().failure();
    let stderr = String::from_utf8_lossy(&out.get_output().stderr).to_string();
    assert!(
        stderr.contains("nova: panic: allocation of 9223372036854775800 bytes exceeds the maximum object size of 9223372036854775792 bytes"),
        "stderr: {stderr}"
    );
    // The point of the fix: a Nova diagnostic instead of a Rust panic. Also
    // distinct from the genuine out-of-memory wording ("memory allocation of N
    // bytes failed"), which a merely-too-big length still produces.
    assert!(
        !stderr.contains("panicked at") && !stderr.contains("non-unwinding"),
        "expected no Rust panic, stderr: {stderr}"
    );
    // The JIT shares `gc::alloc`, but assert it too: the abort happens inside
    // the compiler process there.
    let assert = nova().arg("run").arg(&file).assert().failure();
    let jit_stderr = String::from_utf8_lossy(&assert.get_output().stderr).to_string();
    assert!(
        jit_stderr.contains("exceeds the maximum object size"),
        "stderr: {jit_stderr}"
    );
    assert!(
        !jit_stderr.contains("panicked at"),
        "expected no Rust panic, stderr: {jit_stderr}"
    );
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
fn method_generics_run() {
    let expected =
        std::fs::read_to_string(repo_root().join("tests/runtime/method_generics.stdout"))
            .expect("expected-output fixture exists")
            .replace("\r\n", "\n");
    nova()
        .arg("run")
        .arg(repo_root().join("tests/runtime/method_generics.nova"))
        .assert()
        .success()
        .stdout(expected);
}

#[test]
fn method_generics_build_standalone() {
    let out = build_and_run("tests/runtime/method_generics.nova", "method_generics");
    let expected =
        std::fs::read_to_string(repo_root().join("tests/runtime/method_generics.stdout"))
            .expect("fixture")
            .replace("\r\n", "\n");
    assert_eq!(out, expected);
}

#[test]
fn where_clauses_run() {
    let expected = std::fs::read_to_string(repo_root().join("tests/runtime/where_clauses.stdout"))
        .expect("expected-output fixture exists")
        .replace("\r\n", "\n");
    nova()
        .arg("run")
        .arg(repo_root().join("tests/runtime/where_clauses.nova"))
        .assert()
        .success()
        .stdout(expected);
}

#[test]
fn where_clauses_build_standalone() {
    let out = build_and_run("tests/runtime/where_clauses.nova", "where_clauses");
    let expected = std::fs::read_to_string(repo_root().join("tests/runtime/where_clauses.stdout"))
        .expect("fixture")
        .replace("\r\n", "\n");
    assert_eq!(out, expected);
}

#[test]
fn prelude_option_result_run() {
    let expected =
        std::fs::read_to_string(repo_root().join("tests/runtime/prelude_option_result.stdout"))
            .expect("expected-output fixture exists")
            .replace("\r\n", "\n");
    nova()
        .arg("run")
        .arg(repo_root().join("tests/runtime/prelude_option_result.nova"))
        .assert()
        .success()
        .stdout(expected);
}

#[test]
fn prelude_option_result_build_standalone() {
    let out = build_and_run(
        "tests/runtime/prelude_option_result.nova",
        "prelude_option_result",
    );
    let expected =
        std::fs::read_to_string(repo_root().join("tests/runtime/prelude_option_result.stdout"))
            .expect("fixture")
            .replace("\r\n", "\n");
    assert_eq!(out, expected);
}

/// `nova run`'s Cranelift JIT cannot resolve the libm symbol `sqrt` at run
/// time on Linux only (`E0902`) -- `nova build`'s ahead-of-time link step
/// resolves the same symbol fine on the same platform, and this exact `nova
/// run` invocation passes on macOS and Windows. Filed as
/// <https://github.com/Sakeerin/nova/issues/3> rather than fixed here: the
/// right remedy is a design decision (explicitly load `libm` at JIT setup,
/// or document `extern` libm-family functions as build-only on Linux), not
/// a mechanical patch.
///
/// `#[ignore]`, not `#[cfg(not(target_os = "linux"))]`, and only on Linux:
/// an ignored test still shows up in every platform's test counts (`cargo
/// test`'s summary line, and `-- --ignored` on the advisory CI step), while
/// a `cfg`-ed-out one vanishes with no trace anywhere -- and invisibility is
/// exactly how the defects this PR exists to fix survived undetected.
/// Matches the precedent of ADR 0010's eight documented ignores. Unaffected
/// on macOS and Windows, where it keeps running (and passing) normally.
#[test]
#[cfg_attr(
    target_os = "linux",
    ignore = "E0902 at JIT time on Linux; see issue #3"
)]
fn extern_ffi_run() {
    let expected = std::fs::read_to_string(repo_root().join("tests/runtime/extern_ffi.stdout"))
        .expect("expected-output fixture exists")
        .replace("\r\n", "\n");
    nova()
        .arg("run")
        .arg(repo_root().join("tests/runtime/extern_ffi.nova"))
        .assert()
        .success()
        .stdout(expected);
}

#[test]
fn extern_ffi_build_standalone() {
    let out = build_and_run("tests/runtime/extern_ffi.nova", "extern_ffi");
    let expected = std::fs::read_to_string(repo_root().join("tests/runtime/extern_ffi.stdout"))
        .expect("fixture")
        .replace("\r\n", "\n");
    assert_eq!(out, expected);
}

#[test]
fn extern_unresolvable_symbol_is_a_clean_error_not_a_panic() {
    let dir = std::env::temp_dir().join("nova-extern-unresolvable");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let file = dir.join("bad.nova");
    std::fs::write(
        &file,
        "extern \"C\" { fn totally_bogus_symbol_xyz(x: Int) -> Int }\n\
         fn main() { println(\"${totally_bogus_symbol_xyz(3)}\") }\n",
    )
    .expect("write");
    let assert = nova().arg("run").arg(&file).assert().failure();
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr).to_string();
    assert!(stderr.contains("E0902"), "stderr: {stderr}");
    // Must be a clean diagnostic, not a Rust panic nor a "compiler bug" label.
    assert!(!stderr.contains("panicked"), "should not panic: {stderr}");
    assert!(
        !stderr.contains("compiler bug"),
        "not a compiler bug: {stderr}"
    );
}

/// Regression for the `collect_impls` self-prepend bug (commit c4269ec):
/// `nova check` used to accept a self-less inherent method called on an
/// instance, and codegen then died with a Cranelift verifier error and
/// "internal codegen error (this is a compiler bug)". It must now be
/// rejected cleanly at type-check time with `E0014`, with no ICE.
#[test]
fn selfless_inherent_method_called_on_instance_is_e0014_not_an_ice() {
    let dir = std::env::temp_dir().join("nova-selfless-inherent-call");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let file = dir.join("selfless_call.nova");
    std::fs::write(
        &file,
        "record P { v: Int }\n\
         impl P { fn make() -> P { P { v: 7 } } }\n\
         fn main() {\n\
             let p = P { v: 0 }\n\
             let q = p.make()\n\
             println(\"${q.v}\")\n\
         }\n",
    )
    .expect("write");
    let assert = nova().arg("run").arg(&file).assert().failure();
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr).to_string();
    assert!(stderr.contains("E0014"), "stderr: {stderr}");
    // Must be a clean diagnostic, never the Cranelift verifier dump or the
    // "compiler bug" label the pre-fix codegen ICE produced.
    assert!(!stderr.contains("panicked"), "should not panic: {stderr}");
    assert!(
        !stderr.contains("compiler bug"),
        "not a compiler bug: {stderr}"
    );
}

/// Associated-function call syntax (`Type::f(args)`, commit 865bff4):
/// `P::new()` must produce the same value under both backends, mirroring
/// how every other feature in this file is checked for backend parity.
#[test]
fn assoc_fn_run() {
    let expected = std::fs::read_to_string(repo_root().join("tests/runtime/assoc_fn.stdout"))
        .expect("expected-output fixture exists")
        .replace("\r\n", "\n");
    nova()
        .arg("run")
        .arg(repo_root().join("tests/runtime/assoc_fn.nova"))
        .assert()
        .success()
        .stdout(expected);
}

#[test]
fn assoc_fn_build_standalone() {
    let out = build_and_run("tests/runtime/assoc_fn.nova", "assoc_fn");
    let expected = std::fs::read_to_string(repo_root().join("tests/runtime/assoc_fn.stdout"))
        .expect("fixture")
        .replace("\r\n", "\n");
    assert_eq!(out, expected);
}

/// Trait associated functions (`Type::name(…)` for a trait method with no
/// `self`), including dispatch through a generic parameter's bound. Checked
/// end-to-end under both backends because the bug class here is a *codegen*
/// one: a receiver lowered into a callee that has no `self` parameter type-checks
/// fine and is only caught by the backend's arity verifier, so no typeck- or
/// MIR-level test can stand in for actually running the program.
#[test]
fn trait_assoc_fn_run() {
    let expected = std::fs::read_to_string(repo_root().join("tests/runtime/trait_assoc_fn.stdout"))
        .expect("expected-output fixture exists")
        .replace("\r\n", "\n");
    nova()
        .arg("run")
        .arg(repo_root().join("tests/runtime/trait_assoc_fn.nova"))
        .assert()
        .success()
        .stdout(expected);
}

#[test]
fn trait_assoc_fn_build_standalone() {
    let out = build_and_run("tests/runtime/trait_assoc_fn.nova", "trait_assoc_fn");
    let expected = std::fs::read_to_string(repo_root().join("tests/runtime/trait_assoc_fn.stdout"))
        .expect("fixture")
        .replace("\r\n", "\n");
    assert_eq!(out, expected);
}

/// Supertraits (`trait Ranked: Named`): a bound `T: Ranked` calls the
/// supertrait's methods, a default body reaches them through `Self`, and
/// conditional impls (`impl<T: Ranked> Named for Boxed<T>`) discharge the
/// supertrait-derived bounds at monomorphization. Run end-to-end because the
/// interesting failure is a *dispatch* one: the type checker resolves the
/// supertrait call against the trait's signature while monomorphization picks
/// the impl function, so only executing the program proves they agree.
#[test]
fn supertraits_run() {
    let expected = std::fs::read_to_string(repo_root().join("tests/runtime/supertraits.stdout"))
        .expect("expected-output fixture exists")
        .replace("\r\n", "\n");
    nova()
        .arg("run")
        .arg(repo_root().join("tests/runtime/supertraits.nova"))
        .assert()
        .success()
        .stdout(expected);
}

#[test]
fn supertraits_build_standalone() {
    let out = build_and_run("tests/runtime/supertraits.nova", "supertraits");
    let expected = std::fs::read_to_string(repo_root().join("tests/runtime/supertraits.stdout"))
        .expect("fixture")
        .replace("\r\n", "\n");
    assert_eq!(out, expected);
}

#[test]
fn generic_trait_methods_run() {
    let expected =
        std::fs::read_to_string(repo_root().join("tests/runtime/generic_trait_methods.stdout"))
            .expect("expected-output fixture exists")
            .replace("\r\n", "\n");
    nova()
        .arg("run")
        .arg(repo_root().join("tests/runtime/generic_trait_methods.nova"))
        .assert()
        .success()
        .stdout(expected);
}

#[test]
fn generic_trait_methods_build_standalone() {
    let out = build_and_run(
        "tests/runtime/generic_trait_methods.nova",
        "generic_trait_methods",
    );
    let expected =
        std::fs::read_to_string(repo_root().join("tests/runtime/generic_trait_methods.stdout"))
            .expect("fixture")
            .replace("\r\n", "\n");
    assert_eq!(out, expected);
}

#[test]
fn conflicting_cross_module_extern_signatures_report_e0075() {
    let dir = std::env::temp_dir().join("nova-extern-conflict");
    std::fs::create_dir_all(&dir).expect("temp dir");
    std::fs::write(
        dir.join("mod_a.nova"),
        "extern \"C\" { fn sqrt(x: Float) -> Float }\npub fn ca() -> Float { sqrt(16.0) }\n",
    )
    .expect("write a");
    std::fs::write(
        dir.join("mod_b.nova"),
        "extern \"C\" { fn sqrt(x: Int) -> Int }\npub fn cb() -> Int { sqrt(16) }\n",
    )
    .expect("write b");
    let main = dir.join("main.nova");
    std::fs::write(
        &main,
        "import mod_a::{ca}\nimport mod_b::{cb}\nfn main() { println(\"${ca()} ${cb()}\") }\n",
    )
    .expect("write main");
    let assert = nova().arg("check").arg(&main).assert().failure();
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr).to_string();
    assert!(stderr.contains("E0075"), "stderr: {stderr}");
    assert!(!stderr.contains("panicked"), "should not panic: {stderr}");
}

#[test]
fn modules_run() {
    let expected = std::fs::read_to_string(repo_root().join("tests/runtime/modules/main.stdout"))
        .expect("expected-output fixture exists")
        .replace("\r\n", "\n");
    nova()
        .arg("run")
        .arg(repo_root().join("tests/runtime/modules/main.nova"))
        .assert()
        .success()
        .stdout(expected);
}

#[test]
fn modules_build_standalone() {
    let out = build_and_run("tests/runtime/modules/main.nova", "modules_main");
    let expected = std::fs::read_to_string(repo_root().join("tests/runtime/modules/main.stdout"))
        .expect("fixture")
        .replace("\r\n", "\n");
    assert_eq!(out, expected);
}

#[test]
fn import_of_private_item_is_rejected() {
    let dir = std::env::temp_dir().join("nova-mod-priv");
    std::fs::create_dir_all(&dir).expect("temp dir");
    std::fs::write(dir.join("lib.nova"), "fn hidden() -> Int { 1 }\n").expect("write lib");
    let main = dir.join("app.nova");
    std::fs::write(
        &main,
        "import lib::{hidden}\nfn main() { let x = hidden() }\n",
    )
    .expect("write");
    let assert = nova().arg("check").arg(&main).assert().failure();
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr).to_string();
    assert!(stderr.contains("E0001"), "stderr: {stderr}");
}

/// Two modules each defining a same-named function must lower to distinct
/// symbols and dispatch to their own definition — not collapse to one at
/// monomorphization. Regression for the module-merge mangling collision.
#[test]
fn modules_same_name_functions_dispatch_correctly() {
    let dir = std::env::temp_dir().join("nova-mod-collide-fn");
    std::fs::create_dir_all(&dir).expect("temp dir");
    std::fs::write(
        dir.join("lib.nova"),
        "fn helper() -> Int { 200 }\npub fn lib_val() -> Int { helper() }\n",
    )
    .expect("write lib");
    let main = dir.join("main.nova");
    std::fs::write(
        &main,
        "import lib::{lib_val}\n\
         fn helper() -> Int { 100 }\n\
         fn main() {\n\
             println(\"main=${helper()}\")\n\
             println(\"lib=${lib_val()}\")\n\
         }\n",
    )
    .expect("write main");
    // main's own helper() is 100; lib_val() calls lib's helper() = 200.
    nova()
        .arg("run")
        .arg(&main)
        .assert()
        .success()
        .stdout("main=100\nlib=200\n");
}

/// Two modules each defining a same-named record with a same-named inherent
/// method must dispatch each call to its own method body. Regression for the
/// impl-method symbol collision across modules.
#[test]
fn modules_same_name_methods_dispatch_correctly() {
    let dir = std::env::temp_dir().join("nova-mod-collide-method");
    std::fs::create_dir_all(&dir).expect("temp dir");
    std::fs::write(
        dir.join("lib.nova"),
        "record Box { v: Int }\n\
         impl Box { fn get(self) -> Int { self.v + 1000 } }\n\
         pub fn make_b() -> Int { Box { v: 7 }.get() }\n",
    )
    .expect("write lib");
    let main = dir.join("main.nova");
    std::fs::write(
        &main,
        "import lib::{make_b}\n\
         record Box { v: Int }\n\
         impl Box { fn get(self) -> Int { self.v } }\n\
         fn main() {\n\
             let a = Box { v: 1 }\n\
             println(\"main=${a.get()}\")\n\
             println(\"lib=${make_b()}\")\n\
         }\n",
    )
    .expect("write main");
    // main's Box.get is `self.v` = 1; lib's Box.get is `self.v + 1000` = 1007.
    nova()
        .arg("run")
        .arg(&main)
        .assert()
        .success()
        .stdout("main=1\nlib=1007\n");
}

/// A qualified/nested import path (`a::b`) is not supported and must be rejected
/// explicitly rather than silently binding the last segment's module.
#[test]
fn multi_segment_import_is_rejected() {
    let dir = std::env::temp_dir().join("nova-mod-qualified");
    std::fs::create_dir_all(&dir).expect("temp dir");
    std::fs::write(dir.join("foo.nova"), "pub fn marker() -> Int { 1 }\n").expect("write foo");
    std::fs::write(dir.join("bar.nova"), "pub fn marker() -> Int { 2 }\n").expect("write bar");
    let main = dir.join("main.nova");
    std::fs::write(
        &main,
        "import foo::bar\nfn main() { println(\"${marker()}\") }\n",
    )
    .expect("write main");
    let assert = nova().arg("check").arg(&main).assert().failure();
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr).to_string();
    assert!(stderr.contains("E0900"), "stderr: {stderr}");
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

/// `nova check` must reject an `impl` of a subtrait whose supertrait is not
/// implemented for the same type — the whole point of `trait B: A` is that the
/// bound `T: B` may rely on `A`'s methods existing.
#[test]
fn check_rejects_impl_missing_a_supertrait() {
    let dir = std::env::temp_dir().join("nova-check-supertrait");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let file = dir.join("supertrait.nova");
    std::fs::write(
        &file,
        "trait Named { fn name(self) -> String }\n\
         trait Ranked: Named { fn rank(self) -> Int }\n\
         record P { v: Int }\n\
         impl Ranked for P { fn rank(self) -> Int { self.v } }\n\
         fn main() { }\n",
    )
    .expect("write");
    let assert = nova().arg("check").arg(&file).assert().failure();
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr).to_string();
    assert!(stderr.contains("E0072"), "stderr: {stderr}");
    assert!(
        stderr.contains("requires `Named`"),
        "the diagnostic should name the missing supertrait: {stderr}"
    );
}

/// An `async fn` with NO `.await` anywhere in it, merely called from `main`
/// and its result discarded, now COMPILES and RUNS -- Phase 2.3a Task 5
/// replaced the `E0088` rejection of this shape with the real state-machine
/// transform.
///
/// At `Float`, which is where the rejection came from in the first place: an
/// async fn's declared MIR return class comes from `ret_ty` (`Future<T>`,
/// always `MirTy::Ptr`) while its body produces `T` directly, and at
/// `T = Float` (`MirTy::F64`) those are different Cranelift register classes,
/// so the verifier rejected the function with "result 0 has type f64, must
/// match function signature of i64", surfaced as `internal codegen error
/// (this is a compiler bug)`. Both commands are still asserted, because
/// `nova check` runs the same lowering stage and used to reject here too.
///
/// **What this does and does not prove.** It proves the whole program
/// compiles, links through the JIT, and runs to a clean exit -- so the
/// wrapper's state allocation and fat-pointer construction are real machine
/// code that executes. It does NOT prove the async fn's BODY ran: at the time
/// this test was written nothing in Nova could poll a future — `Future` is a
/// nameable type, but naming one is not driving one, there is no await outside
/// an `async fn`, and an `extern` signature accepts only Int/Float/Bool — so
/// `f()`'s future here is built and dropped. `std/task`'s `block_on` is what
/// later made driving a future reachable from source; this fixture deliberately
/// does not use it, because what it pins is the await-free wrapper compiling and
/// running at `Float`. `nova-driver`'s `async_end_to_end` module is where the
/// body is actually driven to completion and its `Float` value checked, in
/// process, because that needs MIR-level calling code no `.nova` file can
/// express until Task 7's `std/task`.
///
/// Asserting the ABSENCE of the old crash text, not just `.success()`, is
/// deliberate: `nova run` prints diagnostics to stderr and still could have
/// exited zero on some paths.
#[test]
fn run_and_check_accept_and_run_an_await_free_async_fn_at_float() {
    let dir = std::env::temp_dir().join("nova-async-no-await-float");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let file = dir.join("float_no_await.nova");
    std::fs::write(
        &file,
        "async fn f() -> Float { 1.5 }\n\
         fn main() { let x = f()\n  println(\"done\") }\n",
    )
    .expect("write test file");

    nova().arg("check").arg(&file).assert().success();

    let run_assert = nova()
        .arg("run")
        .arg(&file)
        .assert()
        .success()
        .stdout("done\n");
    let run_stderr = String::from_utf8_lossy(&run_assert.get_output().stderr).to_string();
    assert!(
        !run_stderr.contains("E0088"),
        "an await-free async fn must no longer be rejected: {run_stderr}"
    );
    assert!(
        !run_stderr.contains("internal codegen error"),
        "must not reach the codegen ICE path: {run_stderr}"
    );
    assert!(
        !run_stderr.contains("this is a compiler bug"),
        "must not reach the codegen ICE path: {run_stderr}"
    );
}

/// The same shape at `Int` -- kept as the record of why the original defect
/// went unmeasured. `Future<Int>` and the body's actual `Int` both map to
/// `MirTy::I64`/a general-purpose register on x86-64 (unlike `Float`,
/// `MirTy::F64`), so the return-class mismatch was invisible to Cranelift's
/// verifier: this exact program ran to completion silently even while it was
/// miscompiled. It stays in the suite as the reason this branch mandates
/// instantiating at `Float`, not `Int`, for anything tied to register class.
#[test]
fn run_accepts_an_await_free_async_fn_at_int() {
    let dir = std::env::temp_dir().join("nova-async-no-await-int");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let file = dir.join("int_no_await.nova");
    std::fs::write(
        &file,
        "async fn f() -> Int { 1 }\n\
         fn main() { let x = f()\n  println(\"done\") }\n",
    )
    .expect("write test file");

    nova()
        .arg("run")
        .arg(&file)
        .assert()
        .success()
        .stdout("done\n");
}

/// An `async fn` whose body contains `.await` now COMPILES and RUNS -- Phase
/// 2.3a Task 6 replaced the last `.await`-shaped `E0088` rejection with the
/// resumable transform. Both commands are asserted because `nova check` runs the
/// same lowering stage and used to reject here too.
///
/// **What this does and does not prove.** It proves a body with a suspend point
/// in it survives the whole pipeline into machine code that links and runs: the
/// resume dispatch, the indirect poll of the inner future and the trap arms are
/// all real instructions Cranelift accepted. It does NOT prove `g`'s body ran,
/// for the same reason the await-free case above does not -- nothing in Nova can
/// poll a future until Task 7's `std/task`, so `g()`'s future here is built and
/// dropped. `nova-driver`'s `async_end_to_end` module is where a suspend and a
/// resume are actually executed and the awaited `Float` checked.
///
/// At `Float`: `mir_ty` collapses `Int` onto the same class as every pointer, so
/// an awaited value moved through the wrong one of them is invisible at `Int`.
#[test]
fn run_and_check_accept_and_run_an_async_fn_containing_await() {
    let dir = std::env::temp_dir().join("nova-async-with-await");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let file = dir.join("with_await.nova");
    std::fs::write(
        &file,
        "async fn f() -> Float { 1.5 }\n\
         async fn g() -> Float { f().await }\n\
         fn main() { let x = g()\n  println(\"done\") }\n",
    )
    .expect("write test file");

    nova().arg("check").arg(&file).assert().success();

    let run_assert = nova()
        .arg("run")
        .arg(&file)
        .assert()
        .success()
        .stdout("done\n");
    let run_stderr = String::from_utf8_lossy(&run_assert.get_output().stderr).to_string();
    assert!(
        !run_stderr.contains("E0088"),
        "an `async fn` containing `.await` must no longer be rejected: {run_stderr}"
    );
    assert!(
        !run_stderr.contains("internal codegen error"),
        "must not reach the codegen ICE path: {run_stderr}"
    );
    assert!(
        !run_stderr.contains("this is a compiler bug"),
        "must not reach the codegen ICE path: {run_stderr}"
    );
}

/// An `.await` in a loop, end to end through the CLI: the same suspend point
/// resumed on every iteration, and a back edge into a block the split
/// renumbered. A back edge left pointing at its pre-split id reaches the
/// resumable poll function's trap block, so this exits abnormally rather than
/// printing -- which asserting on stdout catches and `.success()` alone might
/// not, since a trap's exit status is platform-shaped.
#[test]
fn run_accepts_an_await_inside_a_loop() {
    let dir = std::env::temp_dir().join("nova-async-await-loop");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let file = dir.join("await_loop.nova");
    std::fs::write(
        &file,
        "async fn one() -> Float { 1.0 }\n\
         async fn total(n: Int) -> Float {\n\
        \x20 let mut i = 0\n\
        \x20 let mut t = 0.0\n\
        \x20 while i < n { t = t + one().await\n i = i + 1 }\n\
        \x20 t\n\
         }\n\
         fn main() { let x = total(3)\n  println(\"done\") }\n",
    )
    .expect("write test file");

    nova().arg("check").arg(&file).assert().success();
    nova()
        .arg("run")
        .arg(&file)
        .assert()
        .success()
        .stdout("done\n");
}

/// An `async fn main` runs, and runs its whole body.
///
/// The failure this asserts against is not a diagnostic but silence: the
/// transform gives the future-building wrapper the symbol its callers were
/// compiled against, and a wrapper allocates a state object and returns without
/// polling it, so an entry point left named that way exits 0 with empty stdout
/// and no diagnostic anywhere. Asserting the *stdout*, not just the exit code,
/// is what distinguishes "drove the future" from "built one and dropped it" --
/// and the `.await` in the body means the shim also has to survive a suspension
/// rather than only a body that happens to finish on its first poll.
#[test]
fn run_drives_an_async_main() {
    let dir = std::env::temp_dir().join("nova-async-main");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let file = dir.join("async_main.nova");
    std::fs::write(
        &file,
        "async fn step(n: Int) -> Int {\n\
         \x20 yield_now().await\n\
         \x20 n + 1\n\
         }\n\
         async fn main() {\n\
         \x20 println(\"start\")\n\
         \x20 println(\"got ${step(41).await}\")\n\
         }\n",
    )
    .expect("write test file");

    nova().arg("check").arg(&file).assert().success();
    nova()
        .arg("run")
        .arg(&file)
        .assert()
        .success()
        .stdout("start\ngot 42\n");
}

/// The `std/task` gate fixture: two tasks spawned, joined, and interleaved by
/// the executor, plus a `Float` result.
///
/// The order the counters' lines appear in belongs to the executor's queue
/// discipline (round-robin, `nova-runtime`'s `task.rs`), not to this test: the
/// `.stdout` fixture is **regenerated by running the program**, never reasoned
/// out and written by hand. The interleaving rather than the totals is what it
/// exists to pin -- a run-to-completion scheduler would produce the same
/// `total 6` with all of `a`'s lines before all of `b`'s.
///
/// The `Float` line is not decoration. `mir_ty` collapses `Int`, `Char` and
/// every pointer-like type onto one 64-bit integer class, so an `Int`-only
/// fixture cannot tell a value that travelled in its own machine class from one
/// reinterpreted out of the executor's `i64` -- `F64` is the only class that
/// crosses register banks, and `half 3.5` is what fails if `block_on`'s result
/// comes back as raw bits.
#[test]
fn gate_async_tasks_run() {
    let expected = std::fs::read_to_string(repo_root().join("tests/runtime/async_tasks.stdout"))
        .expect("expected-output fixture exists")
        .replace("\r\n", "\n");
    nova()
        .arg("run")
        .arg(repo_root().join("tests/runtime/async_tasks.nova"))
        .assert()
        .success()
        .stdout(expected);
}

/// The same fixture through `nova build`, i.e. a standalone linked executable
/// rather than the JIT: object emission, the real linker, and a process whose
/// entry point is the compiled program rather than the compiler.
///
/// **This does not cross a backend boundary.** `build_and_run` invokes
/// `nova build` with no `--release`, and `--release` is what selects the LLVM
/// backend (`cmd/run.rs`'s `BuildCmd`); without it `nova build` uses the same
/// Cranelift backend `nova run` JITs with. So the state object's layout and the
/// poll ABI — which `nova-mir` and `nova-runtime` declare independently and both
/// backends reproduce — are exercised here from the Cranelift side only. **No
/// test executes async machine code emitted by LLVM**; that backend's async
/// coverage is IR-string assertions, with no run. Adding the release-backend
/// async run is follow-up work, deliberately not done in the fix wave that
/// corrected this comment.
#[test]
fn gate_async_tasks_build_standalone() {
    let expected = std::fs::read_to_string(repo_root().join("tests/runtime/async_tasks.stdout"))
        .expect("expected-output fixture exists")
        .replace("\r\n", "\n");
    let out = build_and_run("tests/runtime/async_tasks.nova", "async_tasks");
    assert_eq!(out.replace("\r\n", "\n"), expected);
}

/// The same fixture again with `NOVA_GC_STRESS=1` (collect on every
/// allocation) — the established third member of every gate trio.
///
/// **What this proves:** no premature free anywhere in a live async chain while
/// the collector runs at every allocation, end to end on generated code. The
/// fixture yields inside both spawned tasks, so state objects, their spilled
/// temps and their heap-valued contents all survive across real suspensions and
/// real collections, and the program still produces its exact expected output.
///
/// **What it does not prove: that the `PINNED` registry is honoured.** That
/// would need a suspended state object reachable from *nothing but* the
/// registry, and no state object in this fixture is. `nova_rt_task_block_on`
/// holds the root future's fat pointer as a live parameter for the whole drive,
/// so a conservative stack scan finds it; word 1 of it is the root state, whose
/// temp slots hold both `JoinHandle` records, each holding a `fut` whose word 1
/// is a spawned state. Every state in the chain therefore has a traced path from
/// the stack independent of the registry. The `#[ignore]`d collection tests
/// (ADR 0010) and not this configuration are the coverage of `PINNED` being
/// honoured, which is what `.github/workflows/ci.yml` and ADR 0010 already say.
/// The registry being *populated* is covered ungated, by `nova-runtime`'s
/// `gc::root_count` assertions.
#[test]
fn gate_async_tasks_under_gc_stress() {
    let expected = std::fs::read_to_string(repo_root().join("tests/runtime/async_tasks.stdout"))
        .expect("expected-output fixture exists")
        .replace("\r\n", "\n");
    nova()
        .env("NOVA_GC_STRESS", "1")
        .arg("run")
        .arg(repo_root().join("tests/runtime/async_tasks.nova"))
        .assert()
        .success()
        .stdout(expected);
}

/// `sleep(ms)`, the first primitive that parks rather than spins, proved by
/// wake *order* rather than by elapsed time.
///
/// `slow` is spawned before `quick`, so spawn order alone would print `slow`
/// then `quick`. The fixture's expected output is the reverse: only a
/// scheduler that actually honours each task's deadline (Task 1's park set,
/// driven here through `sleep`'s `Wait::Deadline`) wakes `quick` first. A
/// broken implementation that ignores the deadline and simply re-queues a
/// parked task -- indistinguishable from `yield_now` -- produces `slow` then
/// `quick`, which is exactly the mutation
/// `crates/nova-runtime/src/task.rs`'s `run_to_completion` was hand-mutated to
/// produce while verifying this fixture (see the task 2 report). No duration
/// is asserted anywhere, only the two lines' order, per this suite's
/// standing rule against timing-flaky assertions.
#[test]
fn gate_task_sleep_order_runs() {
    let expected =
        std::fs::read_to_string(repo_root().join("tests/runtime/task_sleep_order.stdout"))
            .expect("expected-output fixture exists")
            .replace("\r\n", "\n");
    nova()
        .arg("run")
        .arg(repo_root().join("tests/runtime/task_sleep_order.nova"))
        .assert()
        .success()
        .stdout(expected);
}

/// Phase 2 gate 2.3: a concurrent producer/consumer over a bounded channel,
/// with timers, producing deterministic output.
///
/// Only the consumer prints, and that is what makes the output deterministic
/// while a timer is genuinely in play: the channel is FIFO, so the order of
/// the `got` lines is fixed by the order of sends however the two tasks
/// interleave, and the `sleep` changes only *when* each value moves. No
/// duration is asserted anywhere, per this suite's standing rule against
/// timing-flaky assertions -- the same rule `gate_task_sleep_order_runs`
/// above follows. The sleep demonstrates that channels and timers compose;
/// it is not what the assertion rests on.
///
/// Capacity 2 against 5 sends is deliberate: the producer fills the buffer
/// and blocks in `send` twice, so this covers the bounded path rather than
/// the case where a channel never fills. `tx.close()` is what ends the
/// consumer loop, since `recv` yields `None` only once the channel is closed
/// *and* drained -- without it the consumer waits forever.
#[test]
fn gate_producer_consumer_channel_runs() {
    nova()
        .arg("run")
        .arg(repo_root().join("examples/03-producer-consumer/src/main.nova"))
        .assert()
        .success()
        .stdout("got 1\ngot 2\ngot 3\ngot 4\ngot 5\ndone\n");
}

/// `block_on` called from inside an `async fn` ends the process with a
/// diagnostic, rather than unwinding out of the runtime through a generated
/// frame.
///
/// The whole async transform rests on no unwind crossing a poll function's
/// boundary: a Cranelift- or LLVM-emitted frame has no landing pads, no drop
/// glue and no unwind description, so an unwinder handed one has nothing to work
/// from. `std/task`'s `block_on` is the first thing that made a *runtime*
/// re-entrancy check reachable from compiled Nova code, which is why that check
/// aborts (`nova_rt_task_block_on`) instead of panicking. This is the test that
/// the reachable path really is the aborting one -- it is the only place the
/// abort is observable, since a `catch_unwind` cannot see one.
#[test]
fn run_aborts_when_an_async_fn_calls_block_on() {
    let dir = std::env::temp_dir().join("nova-nested-block-on");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let file = dir.join("nested_block_on.nova");
    std::fs::write(
        &file,
        "async fn inner() -> Int { 1 }\n\
         async fn outer() -> Int { block_on(inner()) }\n\
         fn main() { println(\"total ${block_on(outer())}\") }\n",
    )
    .expect("write test file");

    // Accepted by every static stage: the hazard is a run-time property of the
    // executor, and nothing in the language forbids the call.
    nova().arg("check").arg(&file).assert().success();
    let assert = nova().arg("run").arg(&file).assert().failure().stdout("");
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr).to_string();
    assert!(
        stderr.contains("re-entrantly"),
        "the abort must say what was violated: {stderr}"
    );
    // The exit code and the message alone do not distinguish an abort from a
    // panic -- a panic unwinding out of the runtime also exits nonzero and also
    // prints this text. The `nova: panic:` prefix is the runtime's own abort
    // path (`abort_with`, the same prefix `nova_rt_panic_str` uses), and Rust's
    // panic handler's `panicked at` is what appears instead if the diagnostic
    // is ever turned back into a `panic!`. Both halves are needed: the first
    // alone passes for a panic whose payload happens to contain the prefix.
    assert!(
        stderr.contains("nova: panic:"),
        "the diagnostic must come from the runtime's abort path: {stderr}"
    );
    assert!(
        !stderr.contains("panicked at"),
        "the diagnostic must not unwind: an unwind here would have to pass \
         through a generated poll frame, which has no unwind description at \
         all: {stderr}"
    );
}

/// The headline case of `2026-08-08-joinhandle-task-identity`: the review's
/// exact forged-handle program, which used to pass `nova check` and then
/// hang forever with empty stdout and stderr (see ADR 0009). `JoinHandle`'s
/// field is public, so nothing stops building one directly
/// with a future that was never handed to `spawn` -- see the fixture's own
/// doc comment for why that used to name `block_on`'s own root task rather
/// than aborting.
///
/// Keyed on the future's own state address instead of a forgeable `Int` id,
/// there is no task left to misresolve to, so this must now abort with a
/// diagnostic instead of hanging -- asserted on the message, not merely on
/// failure, and the test completing at all is itself the proof it no longer
/// hangs.
///
/// The message names `nova_rt_task_join_future`, not `nova_rt_task_is_done`:
/// Task 3 rewrote `join` to call `task_join_future(self.fut).await` as its
/// first statement, so that is the first thing to resolve `self.fut` through
/// `task_id_of` and the first thing this program's forged handle can reach.
/// The contract violated is identical either way (`task_id_of`'s own "this
/// future was never spawned" message), only the caller's name changes.
#[test]
fn forged_join_handle_aborts_instead_of_hanging() {
    let file = repo_root().join("tests/runtime/forged_join_handle.nova");

    // Accepted by every static stage: the hazard is a run-time property of
    // the executor, and nothing in the language forbids constructing a
    // `JoinHandle` directly -- record fields are public (ADR 0007).
    nova().arg("check").arg(&file).assert().success();
    let assert = nova().arg("run").arg(&file).assert().failure().stdout("");
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr).to_string();
    assert!(
        stderr.contains(
            "nova_rt_task_join_future: this future was never spawned, so there is no task to ask about"
        ),
        "the abort must name the unspawned-future contract it violated: {stderr}"
    );
    assert!(
        stderr.contains("nova: panic:"),
        "the diagnostic must come from the runtime's abort path: {stderr}"
    );
    assert!(
        !stderr.contains("panicked at"),
        "the diagnostic must not unwind -- an unwind here would have to pass \
         through a generated poll frame, which has no unwind description at \
         all: {stderr}"
    );
}

/// The forged handle again, on the case the state-address lookup did not close
/// on its own: a never-spawned future whose fresh state object lands on the
/// address of one the collector has already freed.
///
/// `forged_join_handle_aborts_instead_of_hanging` above cannot reach this. Its
/// program allocates almost nothing, so no collection happens, no address is
/// recycled, and the lookup misses for the ordinary reason. This fixture spawns
/// and joins a run of short tasks first, keeping none of them, so the map holds
/// entries whose state objects have since been freed, and only then builds the
/// forged handle -- see the fixture for why each part of that shape is
/// load-bearing. It aborts iff a freed state's entry went with it.
///
/// `NOVA_GC_STRESS=1` is what makes this discriminate at all (collect on every
/// allocation), and it discriminates only where the collector runs -- see the
/// fixture's own comment, and `nova-runtime`'s
/// `a_swept_states_key_is_dropped_so_a_recycled_address_cannot_misresolve` for
/// the same property asserted deterministically and on every platform.
///
/// `.stdout("")` is half the assertion, not tidiness: the fixture prints only
/// on the path where the join returned a value instead of aborting, so a
/// misresolution shows up as stdout content and not merely as a missing
/// diagnostic.
///
/// The message names `nova_rt_task_join_future`, not `nova_rt_task_is_done`,
/// for the same reason `forged_join_handle_aborts_instead_of_hanging`'s does:
/// Task 3's `join` resolves `self.fut` through `task_join_future` first.
#[test]
fn a_recycled_state_address_does_not_resolve_a_never_spawned_future() {
    let file = repo_root().join("tests/runtime/recycled_task_state.nova");

    nova().arg("check").arg(&file).assert().success();
    let assert = nova()
        .arg("run")
        .arg(&file)
        .env("NOVA_GC_STRESS", "1")
        .assert()
        .failure()
        .stdout("");
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr).to_string();
    assert!(
        stderr.contains(
            "nova_rt_task_join_future: this future was never spawned, so there is no task to ask about"
        ),
        "a future the executor never saw must be rejected however its state \
         address was obtained: {stderr}"
    );
    assert!(
        stderr.contains("nova: panic:"),
        "the diagnostic must come from the runtime's abort path: {stderr}"
    );
    assert!(
        !stderr.contains("panicked at"),
        "the diagnostic must not unwind -- an unwind here would have to pass \
         through a generated poll frame, which has no unwind description at \
         all: {stderr}"
    );
}

/// The centrepiece of `2026-08-10-park-set` Task 3: two tasks that join each
/// other is what makes the deadlock diagnostic reachable from Nova source at
/// all. With the old spinning `join`, both tasks would re-check
/// `task_is_done` and `yield_now().await` forever, so the ready queue would
/// never empty and `run_to_completion` could never reach its deadlock arm --
/// a hang, not a diagnostic. Only a parking `join` lets both sides actually
/// leave the ready queue once each has staged its `Wait::Task`, which is
/// what lets the queue drain for real and the executor's exact (non-
/// heuristic) deadlock check fire. So this test passing -- completing at
/// all, with the right diagnostic -- is itself the proof that `join` parks.
///
/// The fixture's own header explains why this is a two-task mutual-join
/// cycle rather than "join a task that spins forever via `yield_now`": the
/// latter is a livelock indistinguishable from legitimate slow progress by
/// any sound check, and was confirmed by hand to hang regardless of whether
/// `join` spins or parks -- see task-3-report.md. See the fixture's own
/// header for why a regression here shows up as a hang rather than a failed
/// assertion.
#[test]
fn task_deadlock_reports_and_aborts_instead_of_hanging_forever() {
    let file = repo_root().join("tests/runtime/task_deadlock.nova");

    // Accepted by every static stage: the deadlock is a run-time property of
    // the executor's park set, and nothing in the language forbids joining a
    // task that never finishes.
    nova().arg("check").arg(&file).assert().success();
    let assert = nova().arg("run").arg(&file).assert().failure().stdout("");
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr).to_string();
    assert!(
        stderr.contains("nova: deadlock:"),
        "the diagnostic must be the deadlock report, not a different abort: {stderr}"
    );
    assert!(
        stderr.contains("is waiting for task"),
        "the report must name what the parked task is waiting for: {stderr}"
    );
}

/// Pins reachability semantics end-to-end (mirrors
/// `nova-mir`'s `unreached_async_fn_compiles_cleanly`, one layer up): an
/// `async fn` that is declared but never called from `main` must still run
/// cleanly. Guards against an over-broad fix that rejects every `is_async`
/// function in the module regardless of whether `main` ever reaches it.
#[test]
fn run_succeeds_when_async_fn_is_never_called() {
    let dir = std::env::temp_dir().join("nova-async-unreached");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let file = dir.join("unreached.nova");
    std::fs::write(
        &file,
        "async fn f() -> Float { 1.5 }\nfn main() { println(\"fine\") }\n",
    )
    .expect("write test file");
    nova()
        .arg("run")
        .arg(&file)
        .assert()
        .success()
        .stdout("fine\n");
}

// === nova build: standalone executables ===

/// Build `source` to a standalone executable named `exe_name`, run it, and
/// return its stdout.
///
/// **The directory carries this process's id.** The 26 `exe_name`s are unique
/// *within* this binary, so there was never an intra-run collision — but
/// without a per-process component every `run_tests` invocation on the machine
/// wrote, executed and then deleted the *same absolute paths*. Two runs
/// overlapping at all (two checkouts, a worktree, a developer alongside CI, two
/// `cargo test` invocations) had one process deleting or overwriting an image
/// another was executing. That is a named candidate mechanism for this
/// binary's known flake, which hits a different test name each run and always
/// passes in isolation: one such failure was observed as exit code
/// `0xC0000005` (ACCESS_VIOLATION) with *empty* stdout and stderr, which is
/// what running a half-written or swapped-out image looks like and not what a
/// Nova-level bug looks like. It also broke this branch's own constraint that
/// every fixture path be unique per process — these were unique only per
/// machine. Not a proven diagnosis: Windows Defender scanning a freshly
/// written `.exe` explains the same symptom equally well, and a genuine
/// intermittent codegen bug is not excluded. It is cheap to eliminate, and
/// eliminating it either fixes the flake or narrows it.
///
/// **The per-test `remove_file` stays, and the directory itself is never
/// removed -- not recursively, and not even best-effort.** All 26 `exe_name`s
/// share this one directory inside a single process, and these tests run on
/// parallel threads. A per-test `remove_dir_all` would have tests deleting each
/// other's executables mid-execution, manufacturing the exact flake this change
/// exists to remove.
///
/// A best-effort *non*-recursive `remove_dir` looks safe by comparison: it can
/// only succeed when the directory is already empty, so it can never delete a
/// sibling's executable. **MEASURED: it is not safe.** A full-workspace run with
/// one added here failed three `*_build_standalone` tests at once, each with
/// `failed to write ...\<name>.exe.nova.obj: The system cannot find the path
/// specified. (os error 3)`. One test's `remove_dir` won a race against a
/// sibling that had already passed `create_dir_all` and was about to write into
/// the directory. Emptiness is not the invariant that matters; "no sibling is
/// between `create_dir_all` and `nova build`" is, and nothing available here can
/// establish it. So the directory is left behind: one empty per-process
/// directory in the system temp folder is a far cheaper residue than a
/// self-inflicted parallel-execution flake.
fn build_and_run(source: &str, exe_name: &str) -> String {
    let dir = std::env::temp_dir().join(format!("nova-build-tests-{}", std::process::id()));
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

/// The GC reclaims garbage: a loop allocating far more than the heap threshold
/// keeps a bounded live set (rather than accumulating, as the old leaking
/// allocator did). Verified through the `NOVA_GC_DEBUG` collection log.
/// Windows-only: precise stack bounds (and thus collection) are currently
/// implemented there.
#[cfg(windows)]
#[test]
fn gc_reclaims_garbage() {
    let dir = std::env::temp_dir().join("nova-gc-reclaim");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let file = dir.join("garbage.nova");
    std::fs::write(
        &file,
        "record Node { v: Int }\n\
         fn main() {\n\
             let mut i = 0\n\
             while i < 300000 {\n\
                 let n = Node { v: i }\n\
                 i = i + 1\n\
             }\n\
             println(\"done ${i}\")\n\
         }",
    )
    .expect("write");
    let assert = nova()
        .arg("run")
        .arg(&file)
        .env("NOVA_GC_DEBUG", "1")
        .assert()
        .success()
        .stdout("done 300000\n");
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr).to_string();
    assert!(
        stderr.contains("nova-gc: collection"),
        "expected at least one collection: {stderr}"
    );
    // The live object count reported by every collection must stay small —
    // proof that garbage is reclaimed rather than accumulated.
    let mut max_live = 0u64;
    for line in stderr.lines() {
        if let Some(i) = line.find(" bytes, ") {
            let rest = &line[i + " bytes, ".len()..];
            if let Some(n) = rest.split(' ').next().and_then(|s| s.parse::<u64>().ok()) {
                max_live = max_live.max(n);
            }
        }
    }
    assert!(
        max_live > 0 && max_live < 1000,
        "live set should stay bounded, saw {max_live} live objects:\n{stderr}"
    );
}

/// `nova build --release` with no LLVM toolchain must fail cleanly and leave
/// the generated IR behind (forcing the no-toolchain path deterministically by
/// pointing `NOVA_CLANG`/`NOVA_LLC` at a nonexistent program).
#[test]
fn release_without_toolchain_emits_ir_and_errors() {
    let dir = std::env::temp_dir().join("nova-release-notool");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let exe = dir.join(format!("hello{}", std::env::consts::EXE_SUFFIX));
    // The intermediate IR is named `<output filename>.nova.ll` so it can never
    // alias the output path.
    let ll = exe.with_file_name(format!(
        "{}.nova.ll",
        exe.file_name().unwrap().to_string_lossy()
    ));
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

/// Regression: intermediate files must never alias the output path, even when
/// `-o` itself ends in `.ll` (which would otherwise make the IR intermediate
/// the output file and later delete the built binary).
#[test]
fn release_intermediate_never_aliases_output() {
    let dir = std::env::temp_dir().join("nova-release-alias");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let out = dir.join("prog.ll"); // output deliberately named `.ll`
    let ir = dir.join("prog.ll.nova.ll"); // the non-aliasing intermediate
    let _ = std::fs::remove_file(&out);
    let _ = std::fs::remove_file(&ir);
    nova()
        .arg("build")
        .arg("--release")
        .arg(repo_root().join("examples/01-hello-world/src/main.nova"))
        .arg("-o")
        .arg(&out)
        .env("NOVA_CLANG", "nova_no_such_tool_xyz")
        .env("NOVA_LLC", "nova_no_such_tool_xyz")
        .assert()
        .failure();
    // The IR went to the distinct intermediate, not to the output path.
    assert!(ir.exists(), "expected IR at {}", ir.display());
    let _ = std::fs::remove_file(&ir);
    let _ = std::fs::remove_file(&out);
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

/// `std/core` end-to-end gate (Phase 2.1, Task 10): `Option`/`Result`'s full
/// method sets, a custom `Display` (direct, through a `T: Display` bound, and
/// via interpolation), `Debug`, `Eq`/`ne`, `Ord` for every primitive that has
/// it (including the non-uniform `Bool` and `String` impls), `Clone`, and
/// `Default` (including `Default for Char`) — round-tripped under `nova run`.
#[test]
fn std_core_run() {
    let expected = std::fs::read_to_string(repo_root().join("tests/runtime/std_core.stdout"))
        .expect("expected-output fixture exists")
        .replace("\r\n", "\n");
    nova()
        .arg("run")
        .arg(repo_root().join("tests/runtime/std_core.nova"))
        .assert()
        .success()
        .stdout(expected);
}

/// Same fixture, compiled to a standalone executable (Cranelift object
/// backend) rather than JIT-run — the other of the two backends this task's
/// gate must cover.
#[test]
fn std_core_build_standalone() {
    let out = build_and_run("tests/runtime/std_core.nova", "std_core");
    let expected = std::fs::read_to_string(repo_root().join("tests/runtime/std_core.stdout"))
        .expect("expected-output fixture exists")
        .replace("\r\n", "\n");
    assert_eq!(out, expected);
}

/// Same fixture again, this time with `NOVA_GC_STRESS=1` (collect on every
/// allocation) — the established convention for proving root-scanning stays
/// correct under heavy `std/core` generic/trait allocation traffic.
#[test]
fn std_core_under_gc_stress() {
    let expected = std::fs::read_to_string(repo_root().join("tests/runtime/std_core.stdout"))
        .expect("expected-output fixture exists")
        .replace("\r\n", "\n");
    nova()
        .env("NOVA_GC_STRESS", "1")
        .arg("run")
        .arg(repo_root().join("tests/runtime/std_core.nova"))
        .assert()
        .success()
        .stdout(expected);
}

/// `Hash` end-to-end (Phase 2.2a Task 6, ADR 0005 §2). Run rather than merely
/// type-checked because the payoffs are all runtime ones: the mixer's large
/// two's-complement constants must actually wrap rather than trap, the
/// `char_to_int` builtin lowers to a register move that no type could catch
/// being wrong, and the **low** bits of an `Int` hash must be spread because
/// `Map` will select buckets with `hash & (cap - 1)`. The expected bucket
/// histograms in the fixture were computed independently from splitmix64's
/// finalizer, so they are an oracle rather than a recording of whatever the
/// implementation happened to print.
///
/// The fixture also carries the complement-pair regression. Because Nova's
/// `>>` is arithmetic, an unmasked `x ^ (x >> k)` yields the same value for
/// `x` and `-1 - x`, which made `mix64` fold 2-to-1 with *identical* hashes —
/// pairs that share a bucket at every capacity, so no resize separates them.
/// Every single-sign sample (consecutive keys, multiples of 8, all-negative)
/// looked textbook-uniform through it, which is exactly why
/// `h(0) != h(-1)` and the both-signs bucket count are in there.
#[test]
fn hash_run() {
    let expected = std::fs::read_to_string(repo_root().join("tests/runtime/hash.stdout"))
        .expect("expected-output fixture exists")
        .replace("\r\n", "\n");
    nova()
        .arg("run")
        .arg(repo_root().join("tests/runtime/hash.nova"))
        .assert()
        .success()
        .stdout(expected);
}

/// The same fixture through the object-file backend: a hash must not depend on
/// which backend compiled it, or a `Map` built by one and read by the other
/// would disagree about buckets.
#[test]
fn hash_build_standalone() {
    let out = build_and_run("tests/runtime/hash.nova", "hash");
    let expected = std::fs::read_to_string(repo_root().join("tests/runtime/hash.stdout"))
        .expect("expected-output fixture exists")
        .replace("\r\n", "\n");
    assert_eq!(out.replace("\r\n", "\n"), expected);
}

/// `std/collections` end-to-end gate (Phase 2.2a, Task 9). The behaviour this
/// pins is *runtime* behaviour that no type-level or MIR-level test can reach:
/// probe chains, tombstones and the load-factor arithmetic are all values
/// computed while the program runs.
///
/// What the fixture covers, and why each item is in there rather than left to
/// a throwaway program:
///
/// - **Removal from the middle of a probe chain**, then a successful lookup
///   *past* the hole — including a chain that **wraps** off the end of the
///   table and continues at index 0. This is the single most likely bug in the
///   whole increment and it is invisible without a targeted test: a `remove`
///   that marks the slot empty instead of tombstoned passes every other
///   assertion here while silently losing every key inserted past it.
/// - **Tombstone reuse**: an insert after a removal must land in the freed slot
///   with `used` (occupied *plus* tombstones) unchanged, so a churn workload
///   cannot grow the table without bound.
/// - **A growth driven by tombstones alone**, which is the only thing that
///   tests `used` at all. Everywhere else in the fixture `len == used` at the
///   growth points, so the load threshold would behave identically read off
///   either field. The churn block drives `used` to 6 of 8 while `len` stays at
///   1 — six inserts on six distinct home buckets, five of them removed, so no
///   tombstone is ever on a later probe path and none is reused — and then
///   asserts that the rehash drops `used` back to `len`, leaves no `2` in the
///   state array, keeps both live keys, and **resurrects none of the five
///   removed ones**. A `len`-keyed threshold never fires there and the table
///   fills until `insert` hits the "found no free slot" panic its own comment
///   calls unreachable; a `grow` that reinserted every not-empty slot instead
///   of every occupied one brings all five deleted keys back (a tombstoned slot
///   still holds its old key and value). Both single-token mutations left the
///   pre-churn fixture byte-identical.
/// - **No shadowed duplicate**: re-inserting a key that lives *behind* one or
///   more tombstones replaces it in place — verified by removing it once
///   afterwards and confirming it is then absent, which a second buried copy
///   would fail.
/// - The **load-factor arithmetic** and two rehashes, with the live count and
///   every lookup correct across them. The printed capacity sequence also
///   shows no doubling skips a power of two, which is what re-entrant growth
///   would look like.
/// - A **user record as a `Map` key and a `Set` element**, with its own `Hash`
///   and `Eq` impls — the case that proves a user type can flow through the
///   `K: Hash + Eq` bound at all, where only `Int` and `String` had been used.
/// - `Map<String, Int>` for the runtime hash path (`str_hash`/FNV-1a) and
///   **negative `Int` keys**, where `% cap` instead of `& (cap - 1)` would
///   produce a negative bucket index.
/// - `Vec` across three growths (0 → 4 → 8 → 16) with `pop`, `set`, `clear`
///   and `get` both in and out of range, plus `is_empty`'s `false` branch for
///   `Vec`, `Map` and `Set`.
///
/// Nothing in here panics: `panic` aborts the process, which would truncate
/// the remaining output. `Vec::set` out of range and `unwrap` on the wrong
/// variant have their own committed tests.
#[test]
fn collections_run() {
    let expected = std::fs::read_to_string(repo_root().join("tests/runtime/collections.stdout"))
        .expect("expected-output fixture exists")
        .replace("\r\n", "\n");
    nova()
        .arg("run")
        .arg(repo_root().join("tests/runtime/collections.nova"))
        .assert()
        .success()
        .stdout(expected);
}

/// The same fixture through the object-file backend. `Map` bucket indices are
/// derived from `Hash`, so a backend that computed a hash differently would
/// build different probe chains — the state-array assertions in the fixture
/// would then diverge here but not under the JIT.
#[test]
fn collections_build_standalone() {
    let expected = std::fs::read_to_string(repo_root().join("tests/runtime/collections.stdout"))
        .expect("expected-output fixture exists")
        .replace("\r\n", "\n");
    let out = build_and_run("tests/runtime/collections.nova", "collections");
    assert_eq!(out.replace("\r\n", "\n"), expected);
}

/// The same fixture again with `NOVA_GC_STRESS=1` (collect on every
/// allocation) — the reason this gate exists. `Vec::push` and `Map::grow`
/// allocate a larger buffer and copy into it, and during `grow`'s window the
/// old key/value/state arrays are reachable *only* through its locals while
/// two further allocations happen. That is exactly where a conservative
/// non-moving collector fails if its root scan misses a stack slot, and the
/// symptom would be silent data loss rather than a crash.
#[test]
fn collections_under_gc_stress() {
    let expected = std::fs::read_to_string(repo_root().join("tests/runtime/collections.stdout"))
        .expect("expected-output fixture exists")
        .replace("\r\n", "\n");
    nova()
        .env("NOVA_GC_STRESS", "1")
        .arg("run")
        .arg(repo_root().join("tests/runtime/collections.nova"))
        .assert()
        .success()
        .stdout(expected);
}

/// A record literal *inside* a `"${…}"` interpolation, end-to-end. The lexer
/// balances a hole's braces, so the literal's `}` no longer terminates the hole
/// (which used to fail with "expected `}` (in record literal), found `}`").
/// Run rather than merely checked because the payoff is that the value is
/// actually constructed and formatted: through interpolation's native
/// primitive conversion, through a `Display` impl, nested one level deeper,
/// and with a hole in a `no_struct_literal` position (an `if` condition).
#[test]
fn record_literal_inside_an_interpolation_runs() {
    let dir = std::env::temp_dir().join("nova-interp-record");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let file = dir.join("interp_record.nova");
    std::fs::write(
        &file,
        "record R { v: Int }\n\
         record G { w: R }\n\
         trait Display { fn fmt(self) -> String }\n\
         impl Display for R { fn fmt(self) -> String { \"R(${self.v})\" } }\n\
         fn f(r: R) -> Int { r.v }\n\
         fn main() {\n\
             println(\"${f(R { v: 1 })}\")\n\
             println(\"${R { v: 2 }}\")\n\
             println(\"${G { w: R { v: 3 } }.w.v}\")\n\
             println(\"${if true { R { v: 4 }.v } else { 0 }}\")\n\
             if \"${f(R { v: 5 })}\" == \"5\" { println(\"cond\") }\n\
         }\n",
    )
    .expect("write");
    let expected = "1\nR(2)\n3\n4\ncond\n";
    nova()
        .arg("run")
        .arg(&file)
        .assert()
        .success()
        .stdout(expected);

    // Same program through the object-file backend, the other of the two.
    let exe = dir.join(format!("interp_record{}", std::env::consts::EXE_SUFFIX));
    nova()
        .arg("build")
        .arg(&file)
        .arg("-o")
        .arg(&exe)
        .assert()
        .success();
    Command::new(&exe).assert().success().stdout(expected);
    let _ = std::fs::remove_file(&exe);
}

/// An interpolation hole that never closes must be a clean diagnostic naming
/// the unterminated interpolation — not a panic, and not a hang.
#[test]
fn unterminated_interpolation_hole_is_a_clean_error() {
    let dir = std::env::temp_dir().join("nova-interp-unterminated");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let file = dir.join("unterminated.nova");
    std::fs::write(
        &file,
        "record R { v: Int }\n\
         fn f(r: R) -> Int { r.v }\n\
         fn main() { println(\"${f(R { v: 1 }\") }\n",
    )
    .expect("write");
    let assert = nova().arg("check").arg(&file).assert().failure();
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr).to_string();
    assert!(
        stderr.contains("unterminated string interpolation"),
        "stderr: {stderr}"
    );
    assert!(!stderr.contains("panicked"), "should not panic: {stderr}");
    assert!(
        !stderr.contains("compiler bug"),
        "not a compiler bug: {stderr}"
    );
}

#[test]
fn panic_aborts_with_message() {
    let assert = nova()
        .arg("run")
        .arg(repo_root().join("tests/runtime/panic_unwrap.nova"))
        .assert()
        .failure();
    let out = assert.get_output();
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stdout.contains('7'), "stdout was {stdout:?}");
    assert!(
        stderr.contains("nova: panic: called get on None"),
        "stderr was {stderr:?}"
    );
    assert!(!stdout.contains("unreachable"), "stdout was {stdout:?}");
}

/// `std/strings` is the third embedded std module (Phase 2.2b). `is_empty` is
/// the one method in it that needs no new intrinsic, so this pins that the
/// module is loaded and its inherent `impl String` resolves — independently of
/// the five intrinsics the rest of the surface needs.
#[test]
fn std_strings_module_is_loaded() {
    let dir = std::env::temp_dir().join("nova-strings-scaffold");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("main.nova");
    std::fs::write(
        &path,
        "fn main() { println(\"${\"\".is_empty()} ${\"x\".is_empty()}\") }",
    )
    .expect("write test file");
    nova()
        .arg("run")
        .arg(&path)
        .assert()
        .success()
        .stdout("true false\n");
}

/// `String::len` counts CODEPOINTS, not bytes — the whole point of Phase 2.2b.
/// `café` is 5 UTF-8 bytes but 4 codepoints; each CJK character here is 3
/// bytes. A byte-based implementation prints `5` and `9`.
#[test]
fn string_len_counts_codepoints_not_bytes() {
    let src = "fn main() { println(\"${\"café\".len()} ${\"日本語\".len()} ${\"\".len()}\") }";
    // House idiom for a temp Nova source in this file — see
    // `check_reports_type_errors_with_code`. No `tempfile` dependency.
    let dir = std::env::temp_dir().join("nova-strings-len");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("main.nova");
    std::fs::write(&path, src).expect("write");
    nova()
        .arg("run")
        .arg(&path)
        .assert()
        .success()
        .stdout("4 3 0\n");
}

/// `str_chars` is the first intrinsic to build a Nova array in the runtime, so
/// it must reproduce codegen's `{ len, elems at 8 + 8*i }` layout exactly. A
/// wrong offset or a wrong length header is a SILENT MISCOMPILE, not a crash —
/// so this reads `.len()` back and indexes elements from Nova, which is the
/// only thing that actually exercises the layout the compiler assumes.
///
/// `char_at`'s whole contract is its two boundaries — `i == 0` (the `i < 0`
/// guard) and `i == len` (the `i >= cs.len()` guard) — so both are exercised
/// directly here, not just interior (`1`) and clearly-out-of-range (`9`, `-1`)
/// indices: `"héllo"` is 5 codepoints, so `char_at(0)` must be `Some('h')` and
/// `char_at(5)` (`== len`) must be `None`, never a bounds-check abort.
#[test]
fn str_chars_array_matches_codegen_layout() {
    let src = "fn main() {\n\
               let cs = \"a→🦀\".chars()\n\
               println(\"${cs.len()} ${cs[0]} ${cs[1]} ${cs[2]}\")\n\
               let e = \"\".chars()\n\
               println(\"${e.len()}\")\n\
               println(\"${\"héllo\".char_at(0).unwrap_or('?')} \
               ${\"héllo\".char_at(1).unwrap_or('?')} \
               ${\"héllo\".char_at(5).unwrap_or('?')} \
               ${\"héllo\".char_at(9).unwrap_or('?')} \
               ${\"héllo\".char_at(0 - 1).unwrap_or('?')}\")\n\
               }";
    // House idiom for a temp Nova source in this file — see
    // `check_reports_type_errors_with_code`. No `tempfile` dependency.
    let dir = std::env::temp_dir().join("nova-strings-chars");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("main.nova");
    std::fs::write(&path, src).expect("write");
    nova()
        .arg("run")
        .arg(&path)
        .assert()
        .success()
        .stdout("3 a → 🦀\n0\nh é ? ? ?\n");
}

/// Round-trip and the half-open slice boundary. Per spec §4.2, `slice` is
/// `start` inclusive / `end` exclusive, `start == end` is valid and yields
/// "", and `reverse` reverses codepoints.
///
/// `"héllo wörld".slice(6, 11)` (`"wörld"`) is not decorative: every other
/// `slice` call anywhere in this test uses `start == 0`, so without a
/// nonzero-`start` case, `chars_to_string`'s `cs[start + i]` could regress to
/// `cs[i]` (dropping the offset) and every test here would still pass — see
/// the mutation-testing note in the Task 4 fix report. The multi-byte prefix
/// (`é` inside `"héllo"`, both in the source before offset 6) also proves the
/// offset is counted in codepoints, not bytes: a byte-based offset would land
/// mid-character and either panic or produce garbage.
#[test]
fn str_from_chars_round_trips_and_slice_is_half_open() {
    let src = "fn main() {\n\
               println(\"${\"a→🦀é\".chars().len()}\")\n\
               println(\"${\"héllo wörld\".slice(0, 5)}|${\"héllo\".slice(0, 0)}|\
               ${\"héllo\".slice(5, 5)}|${\"héllo\".slice(0, 5)}|\
               ${\"héllo wörld\".slice(6, 11)}\")\n\
               println(\"${\"a→🦀\".reverse()} ${\"\".reverse()}\")\n\
               }";
    // House idiom for a temp Nova source in this file — see
    // `check_reports_type_errors_with_code`. No `tempfile` dependency.
    let dir = std::env::temp_dir().join("nova-strings-slice");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("main.nova");
    std::fs::write(&path, src).expect("write");
    nova()
        .arg("run")
        .arg(&path)
        .assert()
        .success()
        .stdout("4\nhéllo|||héllo|wörld\n🦀→a \n");
}

/// `String::slice`'s first panic: a negative `start`. Modelled on
/// `panic_aborts_with_message` — `panic` aborts the process, so this cannot
/// share a fixture with anything that must keep running afterward.
#[test]
fn string_slice_negative_start_panics() {
    let dir = std::env::temp_dir().join("nova-strings-slice-neg-start");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("main.nova");
    std::fs::write(
        &path,
        "fn main() { println(\"${\"abc\".slice(0 - 1, 2)}\") }\n",
    )
    .expect("write");
    let assert = nova().arg("run").arg(&path).assert().failure();
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr).to_string();
    assert!(
        stderr.contains("nova: panic: String::slice start is negative"),
        "stderr: {stderr}"
    );
}

/// `String::slice`'s second panic: `end` past the string's length (`"abc"` has
/// 3 codepoints, so `end = 4` is out of range). `start == end` is deliberately
/// NOT tested here — that is the valid empty-slice case covered by
/// `str_from_chars_round_trips_and_slice_is_half_open`.
#[test]
fn string_slice_end_past_len_panics() {
    let dir = std::env::temp_dir().join("nova-strings-slice-end-oob");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("main.nova");
    std::fs::write(&path, "fn main() { println(\"${\"abc\".slice(0, 4)}\") }\n").expect("write");
    let assert = nova().arg("run").arg(&path).assert().failure();
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr).to_string();
    assert!(
        stderr.contains("nova: panic: String::slice end is past the end of the string"),
        "stderr: {stderr}"
    );
}

/// `String::slice`'s third panic: `start > end` (here `start == end + 1`, one
/// past the boundary where `start == end` would be the valid empty slice).
#[test]
fn string_slice_start_after_end_panics() {
    let dir = std::env::temp_dir().join("nova-strings-slice-start-after-end");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("main.nova");
    std::fs::write(&path, "fn main() { println(\"${\"abc\".slice(2, 1)}\") }\n").expect("write");
    let assert = nova().arg("run").arg(&path).assert().failure();
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr).to_string();
    assert!(
        stderr.contains("nova: panic: String::slice start is after end"),
        "stderr: {stderr}"
    );
}

/// Search is codepoint-indexed, and an empty needle matches at position 0
/// (spec §4.2). `index_of` on "héllo wörld" must report 6 for "wörld", not a
/// byte offset — a byte-based implementation reports 7.
#[test]
fn string_search_is_codepoint_indexed_and_empty_needle_matches() {
    let src = "fn main() {\n\
               let s = \"héllo wörld\"\n\
               println(\"${s.index_of(\"wörld\").unwrap_or(0 - 1)} \
               ${s.index_of(\"zzz\").unwrap_or(0 - 1)} \
               ${s.index_of(\"\").unwrap_or(0 - 1)}\")\n\
               println(\"${s.starts_with(\"hé\")} ${s.starts_with(\"x\")} \
               ${s.ends_with(\"rld\")} ${s.ends_with(\"x\")}\")\n\
               println(\"${s.contains(\"ö\")} ${s.contains(\"q\")} \
               ${s.contains(\"\")} ${s.starts_with(\"\")} ${s.ends_with(\"\")}\")\n\
               println(\"${\"\".index_of(\"a\").unwrap_or(0 - 1)} \
               ${\"aaa\".index_of(\"aa\").unwrap_or(0 - 1)} \
               ${\"abc\".starts_with(\"abcd\")}\")\n\
               }";
    // House idiom for a temp Nova source in this file — see
    // `check_reports_type_errors_with_code`. No `tempfile` dependency.
    let dir = std::env::temp_dir().join("nova-strings-search");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("main.nova");
    std::fs::write(&path, src).expect("write");
    nova()
        .arg("run")
        .arg(&path)
        .assert()
        .success()
        .stdout("6 -1 0\ntrue false true false\ntrue false true true true\n-1 0 false\n");
}

/// Mutation-coverage gap the brief's own test above leaves open (same shape
/// as the Task 3 `char_at` and Task 4 `slice` lessons): every `ends_with` and
/// `index_of` call above compares a haystack STRICTLY LONGER than the needle,
/// so the pre-guards `n.len() > h.len()` (in both `ends_with` and
/// `index_of`) survive being mutated to `>=` — that mutant would wrongly
/// reject the same-length case (`n.len() == h.len()`) as "too long" — because
/// no assertion above ever has `n.len() == h.len()`. `"abc".ends_with("abc")`
/// and `"abc".index_of("abc")` (a full-string, same-length match) close that
/// gap; `"ab".ends_with("abc")` and `"ab".index_of("abc")` (needle strictly
/// LONGER than a nonempty haystack) pin the guard from the other side.
/// `"abcde".index_of("de")` is a plain-ASCII match at the very last valid
/// position (`last == h.len() - n.len() == 3`), independent of the Unicode
/// case above, so it kills both an `at <= last` → `at < last`/`>= last` typo
/// in the search loop's bound and an `at = at + 1` → `at = at + 2` typo in
/// its step (an every-other-position skip would jump straight over index 3
/// and miss the match).
#[test]
fn string_search_same_length_and_last_position_boundaries() {
    let src = "fn main() {\n\
               println(\"${\"abc\".ends_with(\"abc\")} ${\"ab\".ends_with(\"abc\")}\")\n\
               println(\"${\"abc\".index_of(\"abc\").unwrap_or(0 - 1)} \
               ${\"ab\".index_of(\"abc\").unwrap_or(0 - 1)} \
               ${\"abcde\".index_of(\"de\").unwrap_or(0 - 1)}\")\n\
               }";
    // House idiom for a temp Nova source in this file — see
    // `check_reports_type_errors_with_code`. No `tempfile` dependency.
    let dir = std::env::temp_dir().join("nova-strings-search-boundary");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("main.nova");
    std::fs::write(&path, src).expect("write");
    nova()
        .arg("run")
        .arg(&path)
        .assert()
        .success()
        .stdout("true false\n0 -1 3\n");
}

/// `split`'s pinned semantics (spec §4.2): a missing separator yields a
/// one-element array and NEVER an empty one; adjacent, leading and trailing
/// separators produce empty strings with no collapsing; and an EMPTY
/// separator splits into single codepoints — the JavaScript behaviour, chosen
/// because Rust adds boundary empties and Python raises, so there is no
/// consensus to inherit. `join` hangs off the separator, not the parts.
#[test]
fn string_split_and_join_match_the_pinned_semantics() {
    let src = "fn main() {\n\
               let a = \"a,b,c\".split(\",\")\n\
               println(\"${a.len()} ${a[0]}${a[1]}${a[2]}\")\n\
               let b = \"abc\".split(\",\")\n\
               println(\"${b.len()} ${b[0]}\")\n\
               let c = \",a,\".split(\",\")\n\
               println(\"${c.len()} [${c[0]}][${c[1]}][${c[2]}]\")\n\
               let d = \"a,,b\".split(\",\")\n\
               println(\"${d.len()} [${d[0]}][${d[1]}][${d[2]}]\")\n\
               let e = \"a→b\".split(\"→\")\n\
               println(\"${e.len()} ${e[0]}${e[1]}\")\n\
               let f = \"abc\".split(\"\")\n\
               println(\"${f.len()} ${f[0]}|${f[1]}|${f[2]}\")\n\
               println(\"${\"\".split(\"\").len()} ${\"\".split(\",\").len()}\")\n\
               let g = \"xx\".split(\"xx\")\n\
               println(\"${g.len()} [${g[0]}][${g[1]}]\")\n\
               println(\"[${\",\".join(a)}] [${\"\".join(f)}] [${\"-\".join([])}]\")\n\
               }";
    // House idiom for a temp Nova source in this file — see
    // `check_reports_type_errors_with_code`. No `tempfile` dependency.
    let dir = std::env::temp_dir().join("nova-strings-split");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("main.nova");
    std::fs::write(&path, src).expect("write");
    nova().arg("run").arg(&path).assert().success().stdout(
        "3 abc\n1 abc\n3 [][a][]\n3 [a][][b]\n2 ab\n3 a|b|c\n0 1\n2 [][]\n\
             [a,b,c] [abc] []\n",
    );
}

/// Mutation-coverage gap the brief's own test above leaves open (same shape
/// as the Task 3/4/5 lessons): every `join` call above uses a separator that
/// is either a single codepoint (`","`) or empty (`""`), and every `split`
/// separator is either one codepoint (`","`/`"→"`) or spans the WHOLE
/// haystack (`"xx"` in `"xx"`). That leaves gaps a one-character mutation
/// could hide in:
///
/// 1. `join`'s inner copy loop (`for k in 0..sep.len() { out[w] = sep[k] ...
///    }`) mutated to hardcode `sep[0]` instead of indexing by `k` is
///    invisible with `","` (length 1, so `sep[0]` and `sep[k]` never
///    differ) or `""` (the loop never runs zero times either way).
///    `"->".join(["a", "b", "c"])` uses a two-codepoint separator with
///    DIFFERENT characters at each position, so the hardcoded version writes
///    `"--"` where the real one writes `"->"`.
/// 2. `join`'s guard `if parts.len() == 0 { return "" }` mutated to
///    `parts.len() == 1` still gets caught by `"-".join([])` in the brief's
///    test above — but only by accident, via a negative-length array crash
///    on the UNRELATED zero-part call. The mutant's actual, intended effect
///    (silently returning `""` for a genuine one-part call) is never pinned
///    on its own terms. `",".join(["solo"])` does that directly: the answer
///    must be `"solo"`, unchanged, with no separator on either side.
/// 3. `split`'s count/fill arithmetic is only ever exercised with a
///    one-codepoint separator repeated inside a longer haystack, or a
///    multi-codepoint separator spanning the ENTIRE haystack (`"xx"`) with
///    no room for a second match. `"a::b::c".split("::")` is a
///    two-codepoint separator occurring twice INSIDE a seven-codepoint
///    haystack (non-overlapping, neither match at either boundary), a
///    combination none of the brief's cases cover.
/// 4. Every separator tried so far is either non-repeating (`","`, `"→"`,
///    `"::"`) or spans the WHOLE haystack with no room to overlap (`"xx"`
///    in `"xx"`), so the two-pass invariant (pass 1 counts, pass 2 fills,
///    and they MUST walk identically) is only half pinned: a separator
///    that can self-overlap when stepped one codepoint at a time is never
///    tried. Mutating pass 1's step alone — `i = i + s.len()` to
///    `i = i + 1` at `lib.nova:179`, leaving pass 2's `lib.nova:193`
///    untouched — makes pass 1 find every overlapping occurrence of `"aa"`
///    in `"aaaa"` (3, at positions 0/1/2) instead of the 2 correct
///    non-overlapping ones, so `pieces` comes out at 4 instead of 3. Since
///    every piece of `"aaaa".split("aa")` is `""` regardless, content alone
///    cannot catch this — only the LENGTH can, which is why `${u.len()}` is
///    checked explicitly rather than just concatenating pieces.
///    `"ababab".split("abab")` (expect `["", "ab"]`, length 2) adds a case
///    with a nonempty trailing piece after an overlap-capable separator, so
///    the same mutation is pinned by both an all-empty and a
///    not-all-empty result.
/// 5. No test above uses a non-ASCII separator occurring more than once
///    (`"a→b".split("→")` is one occurrence; `"a::b::c".split("::")` occurs
///    twice but is ASCII). `"a→b→c".split("→")` covers both at once.
#[test]
fn string_split_and_join_cover_multi_codepoint_separators_and_one_part_join() {
    let src = "fn main() {\n\
               let s = \"a::b::c\".split(\"::\")\n\
               println(\"${s.len()} ${s[0]}|${s[1]}|${s[2]}\")\n\
               let t = \"a→b→c\".split(\"→\")\n\
               println(\"${t.len()} ${t[0]}|${t[1]}|${t[2]}\")\n\
               let u = \"aaaa\".split(\"aa\")\n\
               println(\"${u.len()} [${u[0]}][${u[1]}][${u[2]}]\")\n\
               let v = \"ababab\".split(\"abab\")\n\
               println(\"${v.len()} [${v[0]}][${v[1]}]\")\n\
               let parts = [\"a\", \"b\", \"c\"]\n\
               println(\"${\"->\".join(parts)}\")\n\
               let one = [\"solo\"]\n\
               println(\"${\",\".join(one)}\")\n\
               }";
    // House idiom for a temp Nova source in this file — see
    // `check_reports_type_errors_with_code`. No `tempfile` dependency.
    let dir = std::env::temp_dir().join("nova-strings-split-join-boundary");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("main.nova");
    std::fs::write(&path, src).expect("write");
    nova()
        .arg("run")
        .arg(&path)
        .assert()
        .success()
        .stdout("3 a|b|c\n3 a|b|c\n3 [][][]\n2 [][ab]\na->b->c\nsolo\n");
}

/// The trim family and `repeat`. `repeat(0)` is "" and a negative count
/// panics (spec §4.2). Trimming an all-whitespace string yields "".
#[test]
fn string_trim_family_and_repeat() {
    let src = "fn main() {\n\
               let s = \"  héllo\\t\\n\"\n\
               println(\"[${s.trim()}][${s.trim_start()}][${s.trim_end()}]\")\n\
               println(\"[${\"   \".trim()}][${\"\".trim()}][${\"x\".trim()}]\")\n\
               println(\"[${\"ab\".repeat(3)}][${\"ab\".repeat(0)}][${\"\".repeat(5)}]\")\n\
               println(\"[${\"→\".repeat(2)}]\")\n\
               }";
    // House idiom for a temp Nova source in this file — see
    // `check_reports_type_errors_with_code`. No `tempfile` dependency.
    let dir = std::env::temp_dir().join("nova-strings-trim");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("main.nova");
    std::fs::write(&path, src).expect("write");
    nova()
        .arg("run")
        .arg(&path)
        .assert()
        .success()
        .stdout("[héllo][héllo\t\n][  héllo]\n[][][x]\n[ababab][][]\n[→→]\n");
}

/// `String::repeat`'s panic: a negative count. Modelled on
/// `panic_aborts_with_message` — `panic` aborts the process, so this cannot
/// share a fixture with anything that must keep running afterward. (An
/// earlier draft of this task pointed at a "`Vec::set` out-of-range test" as
/// the model instead; no such test exists in this file — the comment beside
/// `collections_run` claiming one does is stale — so `panic_aborts_with_message`
/// is the real, committed idiom for a process-aborting panic test.)
#[test]
fn string_repeat_negative_count_panics() {
    let dir = std::env::temp_dir().join("nova-strings-repeat-negative");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("main.nova");
    std::fs::write(&path, "fn main() { println(\"${\"x\".repeat(0 - 1)}\") }\n").expect("write");
    let assert = nova().arg("run").arg(&path).assert().failure();
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr).to_string();
    assert!(
        stderr.contains("nova: panic: String::repeat count must not be negative"),
        "stderr: {stderr}"
    );
}

/// Mutation-coverage gap the brief's own required test leaves open (same
/// shape as the Task 3–6 lessons): `string_trim_family_and_repeat` above only
/// ever trims ASCII whitespace (space, `\t`, `\n`) — none of the four
/// non-ASCII codepoints `char_is_whitespace` compares by scalar value
/// (U+00A0, U+2002, U+2003, U+3000) is exercised anywhere, so deleting or
/// mistyping any one of those four `if v == ...` lines is invisible to the
/// required test. All four are stacked on both sides of "mid" here (rather
/// than tested one at a time) because `trim_start_index`/`trim_end_index`
/// stop scanning at the FIRST non-whitespace codepoint they see: breaking any
/// single line makes the scan halt right there, leaving that codepoint and
/// everything after it (on that side) unstrimmed — so a wrong result on
/// EITHER side already proves at least one of the four is broken, without
/// needing four separate assertions. `\u{...}` is used rather than the raw
/// bytes because these are literally invisible/near-invisible characters in
/// a source file, and the brief's note that `\u{...}` lexes in a Nova STRING
/// literal (unlike a char literal) makes the escaped form both correct and
/// far more reviewable than an invisible byte would be.
#[test]
fn string_trim_covers_non_ascii_whitespace() {
    let src = "fn main() {\n\
               println(\"[${\"\\u{00A0}\\u{2002}\\u{2003}\\u{3000}mid\\u{3000}\\u{2003}\\u{2002}\\u{00A0}\".trim()}]\")\n\
               }";
    // House idiom for a temp Nova source in this file — see
    // `check_reports_type_errors_with_code`. No `tempfile` dependency.
    let dir = std::env::temp_dir().join("nova-strings-trim-unicode-ws");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("main.nova");
    std::fs::write(&path, src).expect("write");
    nova()
        .arg("run")
        .arg(&path)
        .assert()
        .success()
        .stdout("[mid]\n");
}

/// Mutation-coverage gap found in review, distinct from the non-ASCII gap
/// above: `trim_start` and `trim_end` are each called exactly once in
/// `string_trim_family_and_repeat`, both on `"  héllo\t\n"` — never
/// all-whitespace — so neither method's own all-whitespace fallback
/// (`trim_start_index`'s `cs.len()` fallback at `lib.nova:100`;
/// `trim_end_index`'s `floor` fallback at `lib.nova:110`) is ever reached
/// through them directly. Every all-whitespace input in the suite goes
/// through `trim()` instead, and `trim()`'s composition SELF-HEALS a wrong
/// `trim_start_index` result: whatever `a` it returns is fed straight back
/// in as `trim_end_index(cs, a)`'s `floor`, and for an all-whitespace string
/// `trim_end_index` always bottoms out at exactly `floor` (the whole range
/// is whitespace, so the scan never finds a reason to stop early) — so
/// `chars_to_string(cs, a, a)` always hits the `n <= 0` guard and yields ""
/// regardless of what `a` was. That makes "`trim`, `trim_start` and
/// `trim_end` are mutually distinguishable" (true, via the one asymmetric
/// string above) a DIFFERENT, weaker property than "each is independently
/// correct at the boundary where the whole string is whitespace" — which is
/// exactly where `trim_start`/`trim_end` diverge from `trim`'s
/// self-correcting composition. This test pins that boundary directly, on
/// each method's own call, bypassing `trim()` entirely. A tab-only case is
/// included since it costs nothing and confirms the fallback bug is not
/// specifically an ASCII-space artifact.
#[test]
fn string_trim_start_and_trim_end_pinned_on_all_whitespace() {
    let src = "fn main() {\n\
               println(\"[${\"   \".trim_start()}][${\"   \".trim_end()}]\")\n\
               println(\"[${\"\\t\\t\".trim_start()}][${\"\\t\\t\".trim_end()}]\")\n\
               }";
    // House idiom for a temp Nova source in this file — see
    // `check_reports_type_errors_with_code`. No `tempfile` dependency.
    let dir = std::env::temp_dir().join("nova-strings-trim-start-end-all-ws");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("main.nova");
    std::fs::write(&path, src).expect("write");
    nova()
        .arg("run")
        .arg(&path)
        .assert()
        .success()
        .stdout("[][]\n[][]\n");
}

/// Case mapping is WHOLE-STRING, not `Char -> Char`, because `ß` uppercases
/// to the two characters `SS`. A `Char -> Char` implementation cannot express
/// that and would silently corrupt such input — so `"ß".to_upper().len()`
/// being 2 is the assertion that proves the signature choice.
#[test]
fn string_case_mapping_is_whole_string_not_per_char() {
    let src = "fn main() {\n\
               println(\"${\"Straße\".to_upper()} ${\"ß\".to_upper().len()}\")\n\
               println(\"${\"HÉLLO WÖRLD\".to_lower()} ${\"\".to_upper()}|\")\n\
               println(\"${\"İ\".to_lower().len()} ${\"abc123\".to_upper()}\")\n\
               }";
    // House idiom for a temp Nova source in this file — see
    // `check_reports_type_errors_with_code`. No `tempfile` dependency.
    let dir = std::env::temp_dir().join("nova-strings-case");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("main.nova");
    std::fs::write(&path, src).expect("write");
    nova()
        .arg("run")
        .arg(&path)
        .assert()
        .success()
        .stdout("STRASSE 2\nhéllo wörld |\n2 ABC123\n");
}

/// `("a\"b").dbg()` used to produce `"a"b"`, which is not a valid Nova
/// literal — the defect that motivated Phase 2.2b. Escaping needs to inspect
/// the string's contents, which `str_chars` now allows.
#[test]
fn debug_for_string_escapes_into_a_valid_literal() {
    let src = "fn main() {\n\
               println(\"${(\"a\\\"b\").dbg()}\")\n\
               println(\"${(\"back\\\\slash\").dbg()}\")\n\
               println(\"${(\"tab\\there\").dbg()}\")\n\
               println(\"${(\"\").dbg()} ${(\"é→\").dbg()}\")\n\
               }";
    // House idiom for a temp Nova source in this file — see
    // `check_reports_type_errors_with_code`. No `tempfile` dependency.
    let dir = std::env::temp_dir().join("nova-strings-dbg");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("main.nova");
    std::fs::write(&path, src).expect("write");
    nova()
        .arg("run")
        .arg(&path)
        .assert()
        .success()
        .stdout("\"a\\\"b\"\n\"back\\\\slash\"\n\"tab\\there\"\n\"\" \"é→\"\n");
}

/// Mutation-coverage gap the test above leaves open: it exercises `"`, `\\`
/// and `\t`, but never in isolation (always beside plain letters), and never
/// `\n`, `\r` or `\0` at all — four of `escape_common`'s six arms have no
/// dedicated case anywhere else in this file. `\r` and `\0` are the likeliest
/// to be typo'd and go unnoticed: unlike `\n`, a stray CR or NUL byte does not
/// visibly break a terminal line, so a wrong comparison or a wrong returned
/// escape string (e.g. swapping `\r`'s output with `\n`'s, or writing the
/// letter `O` for `\0`'s digit `0`) would be invisible in casual output and
/// invisible to every other test in this file. Each solo case is exactly the
/// one-codepoint string for that escape, so any such mutation surfaces as a
/// wrong line here with nothing else to mask it.
///
/// The last line puts all six escapes adjacent with no plain character
/// between them, which no per-arm-in-isolation case can: an off-by-one in the
/// character loop, or an `out` accumulation step that drops a piece, would
/// only show up when consecutive iterations each append a multi-character
/// escape back to back. It also carries the round-trip property directly:
/// `escape_common`'s escape letters are exactly Nova's own (`n`, `t`, `r`,
/// `0`), so the printed line is byte-for-byte the same text as the source
/// literal used to build the input string — confirmed here by tracing the
/// algorithm byte-by-byte by hand *before* running it (see the task report),
/// not by pasting whatever the program happened to print.
///
/// Written with raw strings (`r#"..."#`) rather than this file's usual
/// backslash-escaped `&str` literals: both the Nova source and the expected
/// stdout are dense with `"` and `\`, and a raw string passes every one of
/// them through unchanged instead of adding a third level of escaping on top
/// of Nova's own and the shell's.
#[test]
fn debug_for_string_escapes_every_control_arm_in_isolation() {
    let src = r#"fn main() {
    println("${("\\").dbg()}")
    println("${("\n").dbg()}")
    println("${("\t").dbg()}")
    println("${("\r").dbg()}")
    println("${("\0").dbg()}")
    println("${("\"").dbg()}")
    println("${("\\\n\t\r\0\"").dbg()}")
}
"#;
    let dir = std::env::temp_dir().join("nova-strings-dbg-arms");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("main.nova");
    std::fs::write(&path, src).expect("write");
    nova().arg("run").arg(&path).assert().success().stdout(
        r#""\\"
"\n"
"\t"
"\r"
"\0"
"\""
"\\\n\t\r\0\""
"#,
    );
}

/// The refactor shares one `escape_common` helper between `Debug for Char`
/// and `Debug for String`, and each keeps only the one quote its own literal
/// syntax needs escaped: `Char` escapes `'` but not `"` (a `"` needs no
/// escaping inside `'…'`), `String` escapes `"` but not `'` (symmetrically).
/// A one-character slip that pointed either impl's quote check at the other
/// quote character would still compile, and would still pass every other
/// test in this file — neither `'"'` nor `"'"` appears in any of them — so
/// each direction gets its own case here.
#[test]
fn debug_for_char_and_string_escape_only_their_own_quote() {
    let src = r#"fn main() {
    println("${('"').dbg()}")
    println("${("'").dbg()}")
}
"#;
    let dir = std::env::temp_dir().join("nova-strings-dbg-quote-cross");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("main.nova");
    std::fs::write(&path, src).expect("write");
    nova().arg("run").arg(&path).assert().success().stdout(
        r#"'"'
"'"
"#,
    );
}

/// `std/strings` end-to-end gate (Phase 2.2b, Task 10). Every index in the
/// module is a codepoint, so a byte-based regression shows up as a wrong
/// number here. Covers every numbered item of the design doc's §7 and every
/// row of its §4.2 table **that does not panic** — nothing in the fixture
/// may panic (a panic aborts the process and truncates the remaining
/// output), so §7 item 8's three `slice` panics and §4.2's `slice`/`repeat`
/// panic rows are deliberately excluded here and live in their own
/// `#[test]`s instead (`string_slice_negative_start_panics`,
/// `string_slice_end_past_len_panics`, `string_slice_start_after_end_panics`,
/// `string_repeat_negative_count_panics`) — across all 18 methods:
/// byte-vs-codepoint length, `chars()`'s array layout read back from Nova,
/// both `char_at` boundaries, `slice`'s half-open boundary plus a
/// nonzero-`start` offset with a multi-byte prefix, a round-trip through
/// `slice`+`join` for ASCII/accented/CJK/emoji input (the only way to
/// exercise `str_from_chars(str_chars(s)) == s` from a user module, since
/// neither builtin is itself callable outside an std module), every pinned
/// `split` row including a self-overlapping separator, `join`'s
/// separator-hangs-off-the-receiver shape, search boundaries (an anchored
/// vs. merely-occurring-somewhere needle, an odd-index mismatch inside the
/// shared `chars_match_at` primitive, empty needle, same-length
/// haystack/needle), the trim family including each of `trim_start`/
/// `trim_end`'s own all-whitespace fallback, an odd-length whitespace run,
/// non-ASCII whitespace and `\r`, `repeat`, `reverse`, whole-string case
/// mapping (`ß` -> `SS`, both directions on `""`), and `Debug for String`'s
/// escaping fix. See `tests/runtime/strings.nova`'s header for the full
/// item-by-item map.
#[test]
fn strings_run() {
    let expected = std::fs::read_to_string(repo_root().join("tests/runtime/strings.stdout"))
        .expect("expected-output fixture exists")
        .replace("\r\n", "\n");
    nova()
        .arg("run")
        .arg(repo_root().join("tests/runtime/strings.nova"))
        .assert()
        .success()
        .stdout(expected);
}

/// The same fixture through the object-file backend. Not redundant with
/// `strings_run`: Task 8's review traced `nova_runtime::symbols()` to a
/// single caller, `compile_jit`, used only by `nova run` — `nova build`
/// links `nova_runtime.lib` at the OS linker level by the real `#[no_mangle]`
/// export name and never consults that table. An error confined to
/// `symbols()` would make the two backends disagree about the same program,
/// and only running the fixture through both can see it.
#[test]
fn strings_build_standalone() {
    let expected = std::fs::read_to_string(repo_root().join("tests/runtime/strings.stdout"))
        .expect("expected-output fixture exists")
        .replace("\r\n", "\n");
    let out = build_and_run("tests/runtime/strings.nova", "strings");
    assert_eq!(out.replace("\r\n", "\n"), expected);
}

/// The same fixture with `NOVA_GC_STRESS=1` (collect on every allocation) —
/// the reason this gate exists. `str_chars` and `str_from_chars` introduce
/// two new allocation shapes reachable from a builtin: a scanned array of
/// scalars, and a leaf byte buffer plus a scanned header. Every method built
/// on them (`slice`, `split`, `join`, the trim family, `repeat`, `reverse`,
/// `to_upper`/`to_lower`) decodes to an intermediate `[Char]` and then
/// allocates again to build the result string, and that intermediate array
/// must stay live across the second allocation. A missed root here is
/// silently wrong text, not a crash.
#[test]
fn strings_under_gc_stress() {
    let expected = std::fs::read_to_string(repo_root().join("tests/runtime/strings.stdout"))
        .expect("expected-output fixture exists")
        .replace("\r\n", "\n");
    nova()
        .env("NOVA_GC_STRESS", "1")
        .arg("run")
        .arg(repo_root().join("tests/runtime/strings.nova"))
        .assert()
        .success()
        .stdout(expected);
}

/// `Vec::set` past the end aborts with its own message rather than
/// corrupting memory. The collections gate's doc comment (`collections_run`,
/// above) has claimed since Phase 2.2a that this test exists; it did not —
/// `git grep` finds only the comment — so the path was uncovered. Modelled on
/// `panic_aborts_with_message`, the file's actual idiom for a
/// process-aborting program. The message is read verbatim out of
/// `std/collections/lib.nova`'s `Vec::set` (both its `i < 0` and `i >=
/// self.len` guards share one message), not guessed.
#[test]
fn vec_set_out_of_range_aborts_with_message() {
    let dir = std::env::temp_dir().join("nova-collections-setoob");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("main.nova");
    std::fs::write(
        &path,
        "fn main() {\n\
         let mut v: Vec<Int> = Vec::new()\n\
         v.push(1)\n\
         v.set(5, 9)\n\
         }",
    )
    .expect("write");
    let assert = nova().arg("run").arg(&path).assert().failure();
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr).to_string();
    assert!(
        stderr.contains("nova: panic: Vec::set index out of range"),
        "stderr: {stderr}"
    );
}

/// `Vec::set`'s OTHER guard: a negative index. The test above only trips
/// `std/collections/lib.nova:55`'s `i >= self.len` guard — deleting line 54's
/// separate `i < 0` guard entirely still passes it, since a negative index
/// then falls straight through to `self.data[i] = v`'s own array-bounds
/// check, which aborts with a *different*, generic message ("array index -1
/// out of bounds for length ...") rather than `Vec::set`'s named one. This
/// pins the negative-index guard on its own terms, in its own temp dir so
/// this and the test above (which run in parallel threads) never share a
/// `main.nova`.
#[test]
fn vec_set_negative_index_aborts_with_message() {
    let dir = std::env::temp_dir().join("nova-collections-setoob-neg");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("main.nova");
    std::fs::write(
        &path,
        "fn main() {\n\
         let mut v: Vec<Int> = Vec::new()\n\
         v.push(1)\n\
         v.set(0 - 1, 9)\n\
         }",
    )
    .expect("write");
    let assert = nova().arg("run").arg(&path).assert().failure();
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr).to_string();
    assert!(
        stderr.contains("nova: panic: Vec::set index out of range"),
        "stderr: {stderr}"
    );
}

/// `unwrap` on a `None` aborts with its own message. Same provenance as
/// `vec_set_out_of_range_aborts_with_message` above — claimed by
/// `collections_run`'s doc comment since Phase 2.2a, never written. The
/// message is read verbatim out of `std/core/lib.nova`'s `Option::unwrap`.
#[test]
fn unwrap_on_the_wrong_variant_aborts_with_message() {
    let dir = std::env::temp_dir().join("nova-collections-unwrap");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("main.nova");
    std::fs::write(
        &path,
        "fn main() {\n\
         let o: Option<Int> = None\n\
         println(\"${o.unwrap()}\")\n\
         }",
    )
    .expect("write");
    let assert = nova().arg("run").arg(&path).assert().failure();
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr).to_string();
    assert!(
        stderr.contains("nova: panic: called `unwrap` on a `None` value"),
        "stderr: {stderr}"
    );
}

// === Normalization seam 3: monomorphization (design doc §4.1) ===
//
// Trait bounds are discharged at monomorphization, not in `check_src`, so this
// seam cannot be exercised from `nova-typeck`'s unit suite at all — it needs the
// whole pipeline, which is why these live here.

/// A generic function whose signature mentions a projection, instantiated at
/// TWO different types. One instantiation would pass even if `subst` dropped
/// the binding and every projection resolved to the same thing; two cannot.
#[test]
fn a_projection_resolves_per_instantiation_at_monomorphization() {
    let src = "trait It { type Item\n fn get_item(self) -> Self::Item }\n\
               record W<T> { v: T }\n\
               impl<T> It for W<T> { type Item = T\n fn get_item(self) -> T { self.v } }\n\
               fn unwrap_item<I: It>(x: I) -> I::Item { x.get_item() }\n\
               fn main() {\n\
                   let a = unwrap_item(W { v: 7 })\n\
                   let b = unwrap_item(W { v: true })\n\
                   println(\"${a} ${b}\")\n\
               }";
    let dir = std::env::temp_dir().join("nova-assoc-mono");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("main.nova");
    std::fs::write(&path, src).expect("write");
    nova()
        .arg("run")
        .arg(&path)
        .assert()
        .success()
        .stdout("7 true\n");
}

/// The invalid-codegen path spec §4.2 and §9's risk 1 describe, closed.
///
/// A *parameter* declared `I::Item` is checked as `Assoc { on: Var(k) }`, because
/// a call site instantiates the callee's generic parameters as fresh inference
/// variables. No receiver is involved, so the `E0011` guard §4.2 relied on never
/// fires. With the determining argument first, `I` is pinned and the program
/// type-checks — and before this seam existed the projection survived to
/// lowering, where `mir_ty`'s defensive arm mapped it to `MirTy::Unit`.
///
/// Measured on the tree before this seam, which is sharper than the plan's
/// "codegen emitted garbage": `MirTy::Unit` parameters are *dropped* from the
/// Cranelift signature, so `f` was emitted taking ONE argument while `main`
/// called it with two, and the run died with
/// `WARN cranelift_codegen::verifier: Found verifier errors in function` /
/// `mismatched argument count for v4 = call fn1(v2, v3): got 2, expected 1`.
///
/// `y` is deliberately unused by the body: the point is the *signature* reaching
/// codegen, and using it would risk some other seam normalizing it first and
/// hiding what is under test.
#[test]
fn a_projection_parameter_on_a_pinned_generic_reaches_codegen_resolved() {
    let src = "trait It { type Item\n fn g(self) -> Int }\n\
               record W { v: Int }\n\
               impl It for W { type Item = Int\n fn g(self) -> Int { 1 } }\n\
               fn f<I: It>(x: I, y: I::Item) -> Int { 7 }\n\
               fn main() { println(\"${f(W { v: 1 }, 5)}\") }";
    let dir = std::env::temp_dir().join("nova-assoc-mono-param");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("main.nova");
    std::fs::write(&path, src).expect("write");
    let assert = nova()
        .arg("run")
        .arg(&path)
        .assert()
        .success()
        .stdout("7\n");
    // The old failure was a *backend* crash, so assert stderr is clean too: a
    // regression that emitted invalid IR but still printed `7` by luck would
    // otherwise pass.
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr).to_string();
    assert!(
        !stderr.contains("verifier"),
        "the projection must not reach codegen: {stderr}"
    );
}

/// The type arguments recorded at a *nested* call must be normalized too, not
/// only the instance's own signature and locals.
///
/// Inside `f`, `id`'s inferred type argument is `Param(0)::Item`, so each
/// instance of `f` records a projection there. Those arguments pick which
/// instance of `id` to emit and are fed to `mangle` — and `mangle_ty` maps
/// **every** `Ty::Assoc` to the single string `"X"`. So substituting without
/// normalizing gives `f::<W<Int>>` and `f::<W<Bool>>` the same callee symbol
/// `id.N$X`; mono's `done` set skips the second, and both calls dispatch to the
/// first one's code. That is a silent wrong answer of exactly the kind head-only
/// impl selection produced once before in this project.
///
/// Found by mutating `type_args: self.tys(type_args)` back to a bare `subst`,
/// which the rest of the suite survived. Measured under that mutation: Cranelift
/// aborts with `declared type of variable var2 doesn't match type of value v2`,
/// because `id`'s `Int` instance is asked to carry a `Bool`. Two instantiations
/// are load-bearing — with one, there is no second symbol to collide with.
#[test]
fn a_projection_in_a_nested_calls_type_arguments_is_normalized_too() {
    let src = "trait It { type Item\n fn get_item(self) -> Self::Item }\n\
               record W<T> { v: T }\n\
               impl<T> It for W<T> { type Item = T\n fn get_item(self) -> T { self.v } }\n\
               fn id<T>(x: T) -> T { x }\n\
               fn f<I: It>(x: I) -> I::Item { id(x.get_item()) }\n\
               fn main() {\n\
                   let a = f(W { v: 7 })\n\
                   let b = f(W { v: true })\n\
                   println(\"${a} ${b}\")\n\
               }";
    let dir = std::env::temp_dir().join("nova-assoc-mono-typeargs");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("main.nova");
    std::fs::write(&path, src).expect("write");
    nova()
        .arg("run")
        .arg(&path)
        .assert()
        .success()
        .stdout("7 true\n");
}

// === The `Iterator` trait and `Vec::iter` (design doc §5) ===
//
// The first real consumer of associated types: `std/core`'s `Iterator` with
// `type Item`, and `std/collections`' `VecIter<T>` binding `Item = T`. Every
// test here compiles a *user* program with no `import`, so it also pins that
// both names arrive by the std glob (ADR 0004).
//
// These are `run_tests` rather than `nova-typeck` unit tests on purpose: the
// projection is resolved at three separate seams and only the whole pipeline
// visits all three. They are also deliberately not folded into
// `tests/runtime/`'s fixtures — the fixture gate is a single program whose
// failure mode is one diff, while these isolate one property each.

/// Iterating a `Vec` by hand through the `Iterator` trait: no `for x in it`
/// desugar and no default methods exist yet (design doc §8), so this is what
/// iteration looks like today — a `while` plus a `match` on the `Option`.
///
/// The loop runs a **fixed** six `next()` calls over a three-element vector
/// rather than stopping at the first `None`, and that shape is load-bearing
/// three ways:
///
/// - It pins that `next` keeps answering `None` after exhaustion, not only
///   that it says `None` once. `nones=3` is the assertion.
/// - It makes a non-advancing `next` *observable instead of a hang*. Deleting
///   `self.i = self.i + 1` from `VecIter::next` yields element 0 six times:
///   `total=60 nones=0`, which fails here. A loop that exited on the first
///   `None` would spin forever and report nothing.
/// - The element values are 10/20/40, not 10/20/30, so that `total` alone
///   distinguishes the mutations. With 10/20/30 a non-advancing iterator sums
///   to the same 60 as the correct one, and only `nones` would have caught it.
///
/// `break` is avoided in the `match` arms: `break` followed by a newline and
/// an expression parses that expression as the break value (see the plan's
/// Global Constraints), and a counter is clearer than working around it.
#[test]
fn a_vec_iterates_to_exhaustion_through_the_iterator_trait() {
    let src = "fn main() {\n\
                   let mut v: Vec<Int> = Vec::new()\n\
                   v.push(10)\n\
                   v.push(20)\n\
                   v.push(40)\n\
                   let mut it = v.iter()\n\
                   let mut total = 0\n\
                   let mut nones = 0\n\
                   let mut n = 0\n\
                   while n < 6 {\n\
                       match it.next() {\n\
                           Some(x) => total = total + x,\n\
                           None => nones = nones + 1,\n\
                       }\n\
                       n = n + 1\n\
                   }\n\
                   println(\"total=${total} nones=${nones}\")\n\
                   let mut it2 = v.iter()\n\
                   let o: Option<Int> = it2.next()\n\
                   println(\"first=${o.unwrap_or(0)}\")\n\
                   let e: Vec<Int> = Vec::new()\n\
                   let mut ei = e.iter()\n\
                   match ei.next() {\n\
                       Some(x) => println(\"unexpected ${x}\"),\n\
                       None => println(\"empty ok\"),\n\
                   }\n\
               }";
    let dir = std::env::temp_dir().join("nova-assoc-veciter");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("main.nova");
    std::fs::write(&path, src).expect("write");
    nova()
        .arg("run")
        .arg(&path)
        .assert()
        .success()
        .stdout("total=70 nones=3\nfirst=10\nempty ok\n");
}

/// `next` at exhaustion must **not** advance the cursor, and nothing above can
/// see the difference.
///
/// `VecIter::next`'s `if self.i >= self.v.len() { return None }` guard is
/// redundant for *safety* — `Vec::get` bounds-checks and would return `None`
/// anyway — so relaxing it to `>` still returns `None` at every index past the
/// end, and `a_vec_iterates_to_exhaustion_through_the_iterator_trait` above
/// passes byte-identically under that mutation. What the guard actually buys is
/// that `i` stops at `len`, and the only way to observe `i` from Nova is to
/// give the vector a new element for the cursor to find.
///
/// That works because `VecIter` holds the `Vec` record by pointer, so `it.v`
/// and `v` are the same object and a `push` during iteration is visible to the
/// iterator (documented at `VecIter`'s declaration). So this test doubles as
/// the pin on that alias-visibility, which is otherwise only asserted in a
/// comment.
#[test]
fn an_exhausted_vec_iterator_does_not_advance_its_cursor() {
    let src = "fn main() {\n\
                   let mut v: Vec<Int> = Vec::new()\n\
                   v.push(10)\n\
                   let mut it = v.iter()\n\
                   match it.next() { Some(x) => println(\"a=${x}\"), None => println(\"a=none\") }\n\
                   match it.next() { Some(x) => println(\"b=${x}\"), None => println(\"b=none\") }\n\
                   v.push(20)\n\
                   match it.next() { Some(x) => println(\"c=${x}\"), None => println(\"c=none\") }\n\
               }";
    let dir = std::env::temp_dir().join("nova-assoc-veciter-cursor");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("main.nova");
    std::fs::write(&path, src).expect("write");
    nova()
        .arg("run")
        .arg(&path)
        .assert()
        .success()
        .stdout("a=10\nb=none\nc=20\n");
}

/// A generic function bounded by `Iterator` whose signature names the
/// projection, called at **two different** instantiations plus an empty one.
/// Design doc §7's gate item 3.
///
/// The receiver is `mut it`, not `it`. §7 originally wrote
/// `fn first<I: Iterator>(it: I)`; Task 8 made `mut self` on a trait method
/// enforced, so that spelling is now `E0060` and the `mut` is the rule working
/// rather than a workaround. `mut` on a *parameter* is what carries it — there
/// is no `let mut` to reach for when the iterator arrives as an argument.
///
/// Two functions, because `first` alone does **not** reach the monomorphization
/// seam and I measured that rather than assuming it. Made mono's normalization
/// cache its first answer and reuse it for every projection (the plan's
/// mutation 3), and `first`'s three instantiations still printed
/// `int=7 / str=hi / bool=none` byte-identically: `Option<Int>` and
/// `Option<String>` lower to the *same* `MirTy` — a pointer to a heap sum — so a
/// corrupted return type on `first`'s String instance changes nothing a backend
/// can see, and `main` is not generic, so typeck's own seam had already fixed
/// the types the `${…}` interpolations dispatch on.
///
/// `first_or` fixes that by naming `I::Item` **bare**, in both a parameter and
/// the return type, so the projection's resolution decides a machine class
/// (`I64` vs a pointer) instead of hiding inside a sum. Under the same mutation
/// it dies in Cranelift with `declared type of variable ... doesn't match type
/// of value`. The wrapped and unwrapped forms are both worth keeping: the
/// wrapped one is the spec's literal signature and the shape a user writes; the
/// unwrapped one is the one with teeth.
///
/// `it: I` deliberately precedes `dflt: I::Item`, so `I` is pinned by the first
/// argument before the projection parameter is checked — the ordering
/// `a_projection_parameter_on_a_pinned_generic_reaches_codegen_resolved` above
/// depends on for the same reason.
#[test]
fn a_generic_function_over_iterator_resolves_item_per_instantiation() {
    let src = "fn first<I: Iterator>(mut it: I) -> Option<I::Item> { it.next() }\n\
               fn first_or<I: Iterator>(mut it: I, dflt: I::Item) -> I::Item {\n\
                   match it.next() { Some(x) => x, None => dflt }\n\
               }\n\
               fn main() {\n\
                   let mut ns: Vec<Int> = Vec::new()\n\
                   ns.push(7)\n\
                   let mut ss: Vec<String> = Vec::new()\n\
                   ss.push(\"hi\")\n\
                   match first(ns.iter()) { Some(n) => println(\"int=${n}\"), None => println(\"int=none\") }\n\
                   match first(ss.iter()) { Some(s) => println(\"str=${s}\"), None => println(\"str=none\") }\n\
                   let e: Vec<Bool> = Vec::new()\n\
                   match first(e.iter()) { Some(b) => println(\"bool=${b}\"), None => println(\"bool=none\") }\n\
                   println(\"or_int=${first_or(ns.iter(), 0)}\")\n\
                   println(\"or_str=${first_or(ss.iter(), \"?\")}\")\n\
                   println(\"or_empty=${first_or(e.iter(), true)}\")\n\
               }";
    let dir = std::env::temp_dir().join("nova-assoc-iterator-generic");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("main.nova");
    std::fs::write(&path, src).expect("write");
    let assert = nova()
        .arg("run")
        .arg(&path)
        .assert()
        .success()
        .stdout("int=7\nstr=hi\nbool=none\nor_int=7\nor_str=hi\nor_empty=true\n");
    // The mutation this test exists to catch fails in the *backend*, so a clean
    // stderr is part of the assertion: a regression that emitted invalid IR and
    // still printed the right bytes by luck would otherwise pass.
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr).to_string();
    assert!(
        !stderr.contains("verifier") && !stderr.contains("panicked"),
        "no projection may reach codegen unresolved: {stderr}"
    );
}

/// The rooting chain `VecIter` introduces, under `NOVA_GC_STRESS=1` (collect on
/// every allocation).
///
/// This is the one risk in `VecIter` that is not a typing question. `next`
/// allocates on every call — `Vec::get` returns `Some(x)`, which is a heap
/// variant — and under stress every one of those is a collection point. The
/// backing `[String]` is reachable only as `it` -> `VecIter` -> `Vec` ->
/// `data`: three levels of heap indirection from one stack slot. Nothing in the
/// existing `collections` gate has that shape, because a `Vec` there is always
/// itself a live local.
///
/// `make()` is what makes the chain load-bearing: the vector is built in a frame
/// that has already returned, so `it` is the **only** root. Holding the `Vec` in
/// `main` as well would keep the array alive independently and the test would
/// prove nothing about `VecIter` holding it.
///
/// `Vec<String>` rather than `Vec<Int>` so the elements are heap objects too,
/// and the accumulator is built by interpolation so each loop turn allocates
/// again while the iterator's storage must stay live across it.
///
/// `VecIter<String>` is written out as `make`'s return type, which also pins
/// that the type is nameable from user code rather than only inferable.
#[test]
fn a_vec_iterator_keeps_its_backing_storage_alive_under_gc_stress() {
    let src = "fn make() -> VecIter<String> {\n\
                   let mut v: Vec<String> = Vec::new()\n\
                   v.push(\"a\")\n\
                   v.push(\"b\")\n\
                   v.push(\"c\")\n\
                   v.push(\"d\")\n\
                   v.push(\"e\")\n\
                   v.iter()\n\
               }\n\
               fn main() {\n\
                   let mut it = make()\n\
                   let mut out = \"\"\n\
                   let mut n = 0\n\
                   while n < 7 {\n\
                       match it.next() {\n\
                           Some(s) => out = \"${out}${s}\",\n\
                           None => out = \"${out}.\",\n\
                       }\n\
                       n = n + 1\n\
                   }\n\
                   println(\"out=${out}\")\n\
               }";
    let dir = std::env::temp_dir().join("nova-assoc-veciter-gc");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("main.nova");
    std::fs::write(&path, src).expect("write");
    // Both modes, from one program: a missed root here is silently wrong *text*,
    // not a crash, so the un-stressed run is the control that says the expected
    // string is right in the first place.
    nova()
        .arg("run")
        .arg(&path)
        .assert()
        .success()
        .stdout("out=abcde..\n");
    nova()
        .env("NOVA_GC_STRESS", "1")
        .arg("run")
        .arg(&path)
        .assert()
        .success()
        .stdout("out=abcde..\n");
}

/// `std/core`'s `Iterator` is implementable by **user** code, and an impl may
/// echo the trait's projection (`-> Option<Self::Item>`) instead of writing the
/// concrete type — design doc §5.1's "either spelling is accepted" row, checked
/// against the shipped trait rather than a test-local one.
///
/// Also a third instantiation of `first<I: Iterator>` that is not a `VecIter`
/// at all, which is what distinguishes "the trait works" from "the trait works
/// for the one implementor std happens to ship".
///
/// `Counter { n: 9 }` is bound to a local before being passed: a record literal
/// written directly inside a `match` scrutinee (`match first(Counter { n: 9 })`)
/// does not parse — the `{` is taken as the end of the call arguments. That is a
/// pre-existing parser limitation unrelated to iterators, worked around here
/// rather than papered over silently.
#[test]
fn a_user_record_can_implement_the_std_iterator_trait() {
    let src = "record Counter { n: Int }\n\
               impl Iterator for Counter {\n\
                   type Item = Int\n\
                   fn next(mut self) -> Option<Self::Item> {\n\
                       if self.n >= 3 { return None }\n\
                       let x = self.n\n\
                       self.n = self.n + 1\n\
                       Some(x)\n\
                   }\n\
               }\n\
               fn first<I: Iterator>(mut it: I) -> Option<I::Item> { it.next() }\n\
               fn main() {\n\
                   let mut c = Counter { n: 0 }\n\
                   match c.next() { Some(x) => println(\"a=${x}\"), None => println(\"a=none\") }\n\
                   let c0 = Counter { n: 0 }\n\
                   match first(c0) { Some(x) => println(\"b=${x}\"), None => println(\"b=none\") }\n\
                   let c9 = Counter { n: 9 }\n\
                   match first(c9) { Some(x) => println(\"c=${x}\"), None => println(\"c=none\") }\n\
               }";
    let dir = std::env::temp_dir().join("nova-assoc-iterator-user-impl");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("main.nova");
    std::fs::write(&path, src).expect("write");
    nova()
        .arg("run")
        .arg(&path)
        .assert()
        .success()
        .stdout("a=0\nb=0\nc=none\n");
}

/// `Iterator::next` takes `mut self`, so calling it through an immutable
/// binding is `E0060` — on `std/core`'s own trait, not just a test-local one.
///
/// Task 8 pinned the rule on a synthetic `trait Bump`. This pins it on the
/// shipped trait, which is what a user actually meets, and is the reason
/// `let mut it` appears in every test above rather than being incidental
/// style. Without it, a caller could silently advance an iterator someone
/// else believes is unread through `next` itself — which is still exactly
/// what this test pins.
///
/// It is *not* what every method on this trait pins any longer.
/// `fold`/`count`/`any`/`collect` (added later, see `std/core/lib.nova`'s
/// comment above `fold`) deliberately dropped this same requirement so they
/// could run on a temporary receiver, and a side effect of that is a
/// non-`mut` binding being silently advanced through any of the four just the
/// same — `iterator_any_short_circuits_and_does_not_scan_past_the_first_
/// match` below pins that as accepted, current behavior, not an oversight.
#[test]
fn calling_next_on_an_immutable_vec_iterator_reports_e0060() {
    let src = "fn main() {\n\
                   let mut v: Vec<Int> = Vec::new()\n\
                   v.push(1)\n\
                   let it = v.iter()\n\
                   match it.next() { Some(x) => println(\"${x}\"), None => println(\"none\") }\n\
               }";
    let dir = std::env::temp_dir().join("nova-assoc-iterator-immutable");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("main.nova");
    std::fs::write(&path, src).expect("write");
    let assert = nova().arg("check").arg(&path).assert().failure();
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr).to_string();
    assert!(
        stderr.contains("error[E0060]") && stderr.contains("`next` mutates its receiver"),
        "stderr: {stderr}"
    );
}

/// Associated types end-to-end gate (Phase 2.2c, Task 10). One program covering
/// every item of the design doc's §7 gate list that a fixture *can* cover —
/// items 1-5; its items 6-10 each fail to compile or abort, so they are
/// `#[test]`s in `crates/nova-typeck/src/check.rs` instead
/// (`a_bound_on_an_associated_type_reports_e0900`,
/// `an_impl_missing_an_associated_type_reports_e0070`,
/// `an_impl_binding_an_undeclared_associated_type_reports_e0071`,
/// `mut_self_trait_method_on_an_immutable_receiver_reports_e0060`,
/// `mut_self_trait_method_conformance_disagreement_is_e0072`).
///
/// Covers: an associated type bound to the impl's **own** generic parameter at
/// three instantiations (so every projection goes through `subst`); both
/// accepted spellings of an impl's signature (`-> T` and the echoed
/// `-> Self::Out`) plus `Self::Out` in a `let` annotation inside an impl body;
/// a trait with **two** associated types, instantiated once as
/// `Both<Int, String>` and once as `Both<String, Int>` so the two cannot be
/// confused; `it.next()` on a concrete `VecIter<Int>` typing as `Option<Int>`
/// with no annotation; a `Vec` iterated past exhaustion; a `Vec::new()` whose
/// first `next` is already `None`; and generic functions naming the projection
/// **bare** (`-> S::Out`, `-> P::Left`, `(dflt: I::Item) -> I::Item`) as well as
/// wrapped (`-> Option<I::Item>`).
///
/// The bare spelling is the load-bearing one and the reason this fixture is not
/// just `first`. Task 9 measured that `-> Option<I::Item>` at three
/// instantiations **survives** a mutation making monomorphization's
/// normalization cache its first answer, byte-identically: `Option<Int>` and
/// `Option<String>` lower to the same `MirTy`, so a wrong `Item` is invisible
/// after lowering, and `main` is not generic, so typeck's seam had already
/// fixed the dispatch types. Bare, a wrong `Item` is a wrong machine class and
/// dies in Cranelift. See the fixture's header for the mutation-to-line map.
#[test]
fn assoc_types_run() {
    let expected = std::fs::read_to_string(repo_root().join("tests/runtime/assoc_types.stdout"))
        .expect("expected-output fixture exists")
        .replace("\r\n", "\n");
    nova()
        .arg("run")
        .arg(repo_root().join("tests/runtime/assoc_types.nova"))
        .assert()
        .success()
        .stdout(expected);
}

/// The same fixture through the object-file backend. Not redundant with
/// `assoc_types_run` for the reason `strings_build_standalone`'s comment gives
/// — `nova run` resolves runtime symbols through `nova_runtime::symbols()`
/// while `nova build` links `nova_runtime.lib` by export name and never
/// consults that table — and for one specific to this gate: the mutation this
/// fixture exists to catch fails in **code generation**, not in the type
/// checker, so the two backends are the two places it can fail differently.
#[test]
fn assoc_types_build_standalone() {
    let expected = std::fs::read_to_string(repo_root().join("tests/runtime/assoc_types.stdout"))
        .expect("expected-output fixture exists")
        .replace("\r\n", "\n");
    let out = build_and_run("tests/runtime/assoc_types.nova", "assoc_types");
    assert_eq!(out.replace("\r\n", "\n"), expected);
}

/// The same fixture with `NOVA_GC_STRESS=1` (collect on every allocation) — a
/// gate criterion, not belt-and-braces. `VecIter` introduces a three-level
/// rooting chain: the backing `[T]` is reachable only as
/// `it` -> `VecIter` -> `Vec` -> `data`, and `next` allocates a fresh `Option`
/// on every call, so the storage must survive a collection triggered by the
/// very call that reads it. The collector scans the stack plus callee-saved
/// registers, and that is precisely what makes an iterator held across a call a
/// root; a missed root here loses elements silently rather than crashing. The
/// `String` and `Char` iterators in the fixture matter for the same reason —
/// their elements are themselves heap objects, so a lost root shows up as wrong
/// text rather than a wrong number.
#[test]
fn assoc_types_under_gc_stress() {
    let expected = std::fs::read_to_string(repo_root().join("tests/runtime/assoc_types.stdout"))
        .expect("expected-output fixture exists")
        .replace("\r\n", "\n");
    nova()
        .env("NOVA_GC_STRESS", "1")
        .arg("run")
        .arg(repo_root().join("tests/runtime/assoc_types.nova"))
        .assert()
        .success()
        .stdout(expected);
}

// === Task 11 Step 6b: two source-reachable branches in `mono.rs` that had no
// test at all. Both survived mutation with the whole suite green.

/// A trait with `n+1` associated types `A0..An` whose impl chains each `Ak` to
/// `link(k)` and bottoms out at `An = Int`, plus a generic function that names
/// `I::A0` **only inside its own body**.
///
/// That last part is what makes monomorphization the *first* seam to resolve the
/// chain concretely, and it is the whole difficulty of reaching mono's `E0078`
/// from source. The typeck-side twins of these programs
/// (`a_binding_chain_that_resolves_to_an_enormous_type_is_a_diagnostic` and
/// `a_long_but_terminating_binding_chain_is_a_diagnostic` in
/// `crates/nova-typeck/src/check.rs`) give the trait a method whose *signature*
/// mentions `Self::A0`, so `check_impl_conformance` normalizes it with a
/// concrete `Self` and reports there — three times — and compilation stops
/// before mono runs. Here nothing in any signature mentions the projection:
/// inside `use_it` the type is `[Assoc { on: Param(0) }]`, which has no head, so
/// seam 1 correctly leaves it alone, and `main`'s call instantiates only
/// `(I) -> Int`.
///
/// `[I::A0]` rather than `Vec<I::A0>` for the diagnostic's sake, not the
/// compiler's: both reach the branch, but mono's `type_name` drops a record's
/// type arguments, so the `Vec` spelling reports "the associated types in
/// `Vec`" — naming a type that does not mention a projection at all. The array
/// spelling reports `[W::A0]`. (`type_name` dropping generic arguments is a
/// queued defect; this test is written so it cannot hide behind it.)
///
/// An empty array literal is the initializer because the element type is
/// genuinely uninhabited here: no value of `I::A0` can be produced without a
/// trait method returning it, and adding one would put the projection back into
/// a signature and move the diagnostic to typeck.
fn assoc_chain_in_a_generic_body(n: u32, link: impl Fn(u32) -> String) -> String {
    let decls: String = (0..=n).map(|k| format!(" type A{k}\n")).collect();
    let binds: String = (0..n)
        .map(|k| format!(" type A{} = {}\n", k, link(k)))
        .collect();
    format!(
        "record Pair<A, B> {{ a: A\n b: B }}\n\
         trait Chain {{\n{decls} fn g(self) -> Int }}\n\
         record W {{ v: Int }}\n\
         impl Chain for W {{\n{binds} type A{n} = Int\n\
         fn g(self) -> Int {{ 1 }} }}\n\
         fn use_it<I: Chain>(x: I) -> Int {{\n\
         let a: [I::A0] = []\n\
         a.len() }}\n\
         fn main() {{ println(\"${{use_it(W {{ v: 1 }})}}\") }}\n"
    )
}

/// Monomorphization's `E0078` branch, and **which of the two limits it names**.
///
/// Mono has its own normalization budget failure path, distinct from
/// `Checker::normalize`'s, and before this test it had no coverage of any kind:
/// making the branch unreachable left the whole suite green. It is not a
/// backstop — a 45-line program reaches it (the wide half below), which is why
/// this is a `nova check` test and not a hand-built `hir::Module`.
///
/// Both halves are here because the `detail` match in `mono.rs` has two arms and
/// a test that asserted only the code would accept either arm for either input.
/// The wide chain is 16 links, comfortably inside the depth limit of 64, so
/// blaming depth would tell the user to shorten a chain that is already short;
/// the deep chain resolves to 66 nodes, so blaming size would tell them to
/// simplify a type that is tiny. The same mutation on the typeck side
/// (`619f453`) survived a test that checked only the code.
///
/// Exactly one diagnostic each, asserted by counting: the typeck twins report
/// three for one root cause because conformance and `check_fn_body` each
/// normalize the same signature, and mono records only the *first* failure per
/// instance. A regression that moved the report back to typeck would show up
/// here as three.
#[test]
fn mono_reports_which_normalization_limit_a_projection_overflowed() {
    let dir = std::env::temp_dir().join("nova-mono-normalize-limits");
    std::fs::create_dir_all(&dir).expect("temp dir");
    // Wide: `type A(k) = Pair<Self::A(k+1), Self::A(k+1)>`, so `A0` resolves to
    // 2^17-1 nodes. The step allowance, not the depth one.
    let wide = dir.join("wide.nova");
    std::fs::write(
        &wide,
        assoc_chain_in_a_generic_body(16, |k| format!("Pair<Self::A{}, Self::A{}>", k + 1, k + 1)),
    )
    .expect("write");
    let assert = nova().arg("check").arg(&wide).assert().failure();
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr).to_string();
    assert_eq!(
        stderr.matches("error[E0078]").count(),
        1,
        "one overflow, one report: {stderr}"
    );
    assert!(
        stderr.contains("resolves to more than 10000 type nodes"),
        "a wide chain is a size report, not a depth report: {stderr}"
    );
    // The projection has to be nameable in the message; `[W::A0]` is what
    // `type_name` produces for the array of it.
    assert!(
        stderr.contains("`[W::A0]`") && stderr.contains("when instantiating `use_it`"),
        "the message must name both the type and the instance: {stderr}"
    );
    // Deep: `type A(k) = Self::A(k+1)`, 65 links, so the chain exceeds the depth
    // limit long before the step allowance (66 nodes total).
    let deep = dir.join("deep.nova");
    std::fs::write(
        &deep,
        assoc_chain_in_a_generic_body(65, |k| format!("Self::A{}", k + 1)),
    )
    .expect("write");
    let assert = nova().arg("check").arg(&deep).assert().failure();
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr).to_string();
    assert_eq!(
        stderr.matches("error[E0078]").count(),
        1,
        "one overflow, one report: {stderr}"
    );
    assert!(
        stderr.contains("the chain of bindings is more than 64 deep"),
        "a deep chain is a depth report, not a size report: {stderr}"
    );
}

/// A trait bound is satisfied only by an impl whose self type **fits the
/// argument structurally**, not merely one that shares its head.
///
/// `impl_satisfies` filters `module.impls` by `self_head`, then requires
/// `imp.match_args(arg)` to succeed. Dropping that second requirement — so any
/// impl on the same head satisfies the bound — left the entire suite green, and
/// it is not an equivalent mutant: `f(P { a: 1, b: true })` then **prints `7`
/// and exits 0** with a declared bound silently unenforced. That is the
/// head-only-selection class this project has already shipped once as a
/// miscompile (`d49f896`), and it is also the gate `E0079`'s unreachability
/// rests on — `match_args` on a non-fitting impl is what would otherwise hand
/// mono a `Ty::Error` self type with a projection on it.
///
/// The positive half is not optional. `P<Int, Bool>` alone would also pass if
/// `impl It for P<Int, Int>` were never selectable at all, which is the failure
/// mode the fix for a head-only bug most plausibly introduces. Both halves are
/// the same declarations with one literal changed.
#[test]
fn a_trait_bound_needs_an_impl_that_fits_structurally_not_just_by_head() {
    let program = |b: &str| {
        format!(
            "trait It {{ type Item\n\
             fn g(self) -> Int }}\n\
             record P<A, B> {{ a: A\n\
             b: B }}\n\
             impl It for P<Int, Int> {{ type Item = Int\n\
             fn g(self) -> Int {{ 1 }} }}\n\
             fn f<I: It>(x: I) -> Int {{ 7 }}\n\
             fn main() {{ println(\"${{f(P {{ a: 1, b: {b} }})}}\") }}\n"
        )
    };
    let dir = std::env::temp_dir().join("nova-impl-structural-fit");
    std::fs::create_dir_all(&dir).expect("temp dir");
    // `P<Int, Bool>` shares the head `Record(P)` with the impl and does not fit
    // it, so the bound is unsatisfied.
    let bad = dir.join("bad.nova");
    std::fs::write(&bad, program("true")).expect("write");
    let assert = nova().arg("run").arg(&bad).assert().failure();
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr).to_string();
    assert!(
        stderr.contains("error[E0013]") && stderr.contains("`P: It`"),
        "stderr: {stderr}"
    );
    // The control: `P<Int, Int>` does fit, so the same program compiles and runs.
    let good = dir.join("good.nova");
    std::fs::write(&good, program("2")).expect("write");
    nova()
        .arg("run")
        .arg(&good)
        .assert()
        .success()
        .stdout("7\n");
}

/// `MapIter` and `FilterIter` (iterator-finishing plan, Task 3): the two lazy
/// adapters `std/core` provides, each a record plus an `Iterator` impl,
/// constructed directly here rather than through `.map()`/`.filter()` — Task 4
/// adds those — so this task is verifiable on its own. Covers three things at
/// once: `MapIter` alone, `FilterIter` alone, and the two chained (`MapIter`
/// wrapping a `FilterIter` wrapping a `VecIter`).
///
/// The chain's output is `Bool`, deliberately: `mir_ty` collapses `Int`,
/// `Char`, `String` and every heap type to one machine class (`MirTy::I64`/
/// `Ptr`), so an `Int`-only pipeline cannot distinguish "yields the right
/// value" from "yields *some* value of the same machine shape". `Bool` is
/// `MirTy::I8` — a different machine class from the vector's `Int` — so a
/// `MapIter` that forwarded the unmapped inner item, or a `FilterIter` whose
/// `Item` bound to a concrete `Int` instead of chasing `I::Item`, both have
/// somewhere to go visibly wrong instead of coincidentally still working.
///
/// The chain is also the first real-source appearance of the `Assoc { on:
/// Assoc }` shape: the outer `MapIter<I, U>`'s field `f: fn(I::Item) -> U`
/// needs `I::Item` where `I = FilterIter<VecIter<Int>>`, and `FilterIter`'s
/// own `type Item = I::Item` is itself a projection rather than a concrete
/// type — so resolving it chases through a second impl (`VecIter<Int>`'s)
/// before landing on `Int`.
#[test]
fn iterator_adapters_chain_and_are_lazy() {
    let src = "fn main() {\n\
                   let mut v = Vec::new()\n\
                   v.push(1)\n\
                   v.push(2)\n\
                   v.push(3)\n\
                   let mut m = MapIter { it: v.iter(), f: |n| n * 10 }\n\
                   for x in m { println(\"${x}\") }\n\
                   let mut f = FilterIter { it: v.iter(), keep: |n| n > 1 }\n\
                   for x in f { println(\"keep ${x}\") }\n\
                   let mut c = MapIter { it: FilterIter { it: v.iter(), keep: |n| n > 1 }, f: |n| n > 2 }\n\
                   for b in c { println(\"chain ${b}\") }\n\
               }";
    let dir = std::env::temp_dir().join("nova-iterator-adapters");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("main.nova");
    std::fs::write(&path, src).expect("write");
    // chain yields Bool: filter keeps 2,3 then map gives false,true
    nova()
        .arg("run")
        .arg(&path)
        .assert()
        .success()
        .stdout("10\n20\n30\nkeep 2\nkeep 3\nchain false\nchain true\n");
}

/// `record_field_index_and_ty` (`crates/nova-typeck/src/check.rs`) had the
/// same raw-`subst`-instead-of-`instantiate` gap `check_record_literal` did
/// above, on the field *read* path rather than construction: reading `m.f`
/// or `c.hit` back out of an already-built, now-concretely-typed record
/// substituted a projection on the record's own bounded parameter without
/// normalizing it, so `g(5)` and `x + 1` below failed structurally even
/// though `m`/`c` themselves built without complaint. Two shapes in one
/// test, since the bug needed a concrete instance of each to show up: a
/// function-typed field (`f: fn(I::Item) -> U`) read out and called, and a
/// plain field (`hit: I::Item`) read out and used directly.
#[test]
fn a_record_field_read_resolves_a_projection_through_a_now_concrete_parameter() {
    let src = "trait It { type Item\n\
                   fn next(mut self) -> Option<Self::Item> }\n\
               record M<I: It, U> { it: I, f: fn(I::Item) -> U }\n\
               record Cache<I: It> { it: I, hit: I::Item }\n\
               record Counter { n: Int }\n\
               impl It for Counter {\n\
                   type Item = Int\n\
                   fn next(mut self) -> Option<Int> { None }\n\
               }\n\
               fn main() {\n\
                   let m = M { it: Counter { n: 0 }, f: |x| x + 1 }\n\
                   let g = m.f\n\
                   println(\"${g(5)}\")\n\
                   let c = Cache { it: Counter { n: 0 }, hit: 7 }\n\
                   let x = c.hit\n\
                   println(\"${x + 1}\")\n\
               }";
    let dir = std::env::temp_dir().join("nova-record-field-read-projection");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("main.nova");
    std::fs::write(&path, src).expect("write");
    nova()
        .arg("run")
        .arg(&path)
        .assert()
        .success()
        .stdout("6\n8\n");
}

/// `MapIter::next` maps exactly one element per call, on demand — it is not
/// merely correct about *which* values come out (the tests above), it is lazy
/// about *when* `f` runs. `f` here is side-effecting (`println("call")`) so
/// that is directly observable: the `for` loop's desugar calls `next` once,
/// prints the mapped result, then `break`s. A lazy `MapIter` therefore prints
/// exactly one `call` before `got 10`; an eager one — say, a `MapIter` that
/// drained and mapped its whole source at construction, buffering the
/// results — would print `call` three times (once per element of `v`)
/// before the loop ever ran at all, since building `m` would need to finish
/// that up front.
#[test]
fn a_map_iter_calls_f_lazily_not_all_up_front() {
    let src = "fn main() {\n\
                   let mut v = Vec::new()\n\
                   v.push(1)\n\
                   v.push(2)\n\
                   v.push(3)\n\
                   let mut m = MapIter { it: v.iter(), f: |n| { println(\"call\") n * 10 } }\n\
                   for x in m { println(\"got ${x}\") break }\n\
                   println(\"done\")\n\
               }";
    let dir = std::env::temp_dir().join("nova-map-iter-laziness");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("main.nova");
    std::fs::write(&path, src).expect("write");
    nova()
        .arg("run")
        .arg(&path)
        .assert()
        .success()
        .stdout("call\ngot 10\ndone\n");
}

/// Task 4 (iterator-finishing plan): the six default methods `Iterator`
/// provides so callers reach `MapIter`/`FilterIter` without ever naming them —
/// `map`, `filter`, `fold`, `count`, `any`, `collect`. Chains `filter` into
/// `map` into `collect` (a `Vec`), then separately drives `fold`/`count`/`any`
/// each over their own fresh `.iter()`, since every consumer here exhausts
/// its receiver and a second call needs a fresh cursor.
///
/// The last line is deliberate, not incidental: it chains `map` to `Bool` (so
/// the item type actually changes partway down the pipeline) and only then
/// consumes with `any`, so that consumer sees a **`Bool`** item rather than
/// the `Int` every earlier line uses — the one assertion here that could
/// catch a wrong item type surviving to the monomorphization seam instead of
/// merely a wrong count or a coincidentally-right value.
///
/// `assert_runs_with` does not exist in this file; this follows the same
/// inline `nova run` pattern every neighboring test in this module uses
/// (e.g. `a_map_iter_calls_f_lazily_not_all_up_front` above).
///
/// **Matches the plan's literal source exactly, unlike an earlier round of
/// this test.** The plan's Step 1 chains `fold`/`count`/`any`/`collect`
/// straight onto a call result — e.g. `v.iter().fold(0, |a, x| a + x)` — which
/// first failed with `E0060` (`place_root`,
/// `crates/nova-typeck/src/check.rs`, classifies a call result
/// `PlaceRoot::NotAPlace`, and a `mut self` receiver requires
/// `PlaceRoot::Mutable`). That was fixed at the design level, not by
/// rewriting this test to dodge it: all four consumers now take plain `self`
/// and rebind internally (`let mut it = self`, `trait Iterator`'s own doc
/// comment above `fold` explains why this aliases rather than copies), so the
/// receiver no longer has to already be a mutable place the caller named.
/// With that landed, the plan's original source needs no `let mut` workaround
/// at all and is reproduced here verbatim.
#[test]
fn iterator_default_methods_work_and_chain() {
    let src = "fn main() {\n\
      let mut v = Vec::new()\n\
      v.push(1)\n\
      v.push(2)\n\
      v.push(3)\n\
      let got = v.iter().filter(|n| n > 1).map(|n| n * 10).collect()\n\
      println(\"${got.len()}\")\n\
      println(\"${got.get(0).unwrap()}\")\n\
      println(\"${v.iter().fold(0, |a, x| a + x)}\")\n\
      println(\"${v.iter().count()}\")\n\
      println(\"${v.iter().any(|n| n > 2)}\")\n\
      println(\"${v.iter().any(|n| n > 9)}\")\n\
      println(\"${v.iter().map(|n| n > 2).any(|b| b)}\")\n\
    }";
    let dir = std::env::temp_dir().join("nova-iterator-default-methods");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("main.nova");
    std::fs::write(&path, src).expect("write");
    nova()
        .arg("run")
        .arg(&path)
        .assert()
        .success()
        .stdout("2\n20\n6\n3\ntrue\nfalse\ntrue\n");
}

/// Task 4 Step 5: laziness is the property the `Iterator::map` *default
/// method* exists to preserve, and nothing above can see it — a lazy and an
/// eager `.map()` produce identical values as long as nothing observes *when*
/// the source is pulled from. `a_map_iter_calls_f_lazily_not_all_up_front`
/// (above, from Task 3) already puts the side effect in the mapping closure
/// `f` and already goes through a directly-built `MapIter { .. }` literal, not
/// `.map()`. This test puts the side effect in the *source* iterator's own
/// `next` instead, and reaches the adapter through `.map()` — the one new
/// piece Task 4 adds — so it is `map`'s default-method body
/// (`MapIter { it: self, f: f }`, nothing more) that is on trial, not
/// `MapIter::next` a second time.
///
/// `PrintSrc` is a hand-written `Iterator` whose `next` prints `pull N` before
/// yielding `N`. `.map(|x| x * 10)` is built and then a marker (`built`)
/// prints *before* the result is touched at all. An eager `.map()` would have
/// had to drain `PrintSrc` to build its result, so every `pull` would print
/// before `built` does. A lazy one prints `built` with no `pull` at all yet,
/// and then exactly one `pull` per `.next()` call, interleaved with that
/// call's mapped result — not three `pull`s up front followed by three
/// results. That interleaving, not merely the final values, is what "lazy"
/// means operationally.
#[test]
fn iterator_map_default_method_is_lazy_over_a_side_effecting_source() {
    let src = "record PrintSrc { n: Int, max: Int }\n\
               impl Iterator for PrintSrc {\n\
                   type Item = Int\n\
                   fn next(mut self) -> Option<Int> {\n\
                       if self.n >= self.max { return None }\n\
                       println(\"pull ${self.n}\")\n\
                       let x = self.n\n\
                       self.n = self.n + 1\n\
                       Some(x)\n\
                   }\n\
               }\n\
               fn main() {\n\
                   let src = PrintSrc { n: 0, max: 3 }\n\
                   let mut m = src.map(|x| x * 10)\n\
                   println(\"built\")\n\
                   println(\"${m.next().unwrap()}\")\n\
                   println(\"${m.next().unwrap()}\")\n\
                   println(\"${m.next().unwrap()}\")\n\
               }";
    let dir = std::env::temp_dir().join("nova-iterator-map-default-is-lazy");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("main.nova");
    std::fs::write(&path, src).expect("write");
    nova()
        .arg("run")
        .arg(&path)
        .assert()
        .success()
        .stdout("built\npull 0\n0\npull 1\n10\npull 2\n20\n");
}

/// Needed to make Step 6 mutation (b) mean anything, and missing from the
/// plan: `any`'s own doc comment says it "short-circuits, which is why it is
/// not written over `fold`", but neither the plan's Step 1 test nor
/// `iterator_map_default_method_is_lazy_over_a_side_effecting_source` above
/// observes that. Step 1's two `any` calls match on the *last* element (`3`)
/// or not at all, so a full scan and a short-circuiting scan return the same
/// `Bool` either way — the mutation the plan itself prescribes (rewriting
/// `any` over `fold`, which visits every element) is invisible to every test
/// this task otherwise has. Without this, mutation (b) would be applied,
/// "pass" against the existing suite, and be reported as caught when it was
/// not — exactly the false confidence the quality bar warns about.
///
/// `CountingSrc` prints on every `next`, same technique as the `map` laziness
/// test above but with the match in the *middle* (`n > 1` first matches at
/// `2` of `0..5`) rather than at an edge, so a short-circuiting `any` prints
/// exactly `pull 0`, `pull 1`, `pull 2` and stops, where a full scan (e.g.
/// `any` rewritten over `fold`) would also print `pull 3` and `pull 4` before
/// the same `true`.
///
/// `src` is deliberately **not** `mut` (unlike an earlier round of this
/// test), and that is not merely "no longer necessary" — it is the point of
/// keeping it this way. Dropping `mut self` from `any` relaxed *two* of
/// `place_root`'s classifications, not only the temporary-receiver one this
/// whole design targeted: a caller's own binding no longer has to be declared
/// `mut` either. So this test doubles as the pin for that: a non-`mut` `src`
/// is silently advanced by `.any()` below with no diagnostic, which is
/// accepted, current behavior — not an oversight, and not something a future
/// change should tighten back to `E0060` without a deliberate decision to do
/// so. Also re-run after the `mut self` -> `self` design change specifically
/// to confirm the short-circuit itself survived the receiver rebinding
/// (`let mut it = self`, which sits directly in the path this test
/// exercises); the expected stdout is unchanged from before that change.
#[test]
fn iterator_any_short_circuits_and_does_not_scan_past_the_first_match() {
    let src = "record CountingSrc { n: Int, max: Int }\n\
               impl Iterator for CountingSrc {\n\
                   type Item = Int\n\
                   fn next(mut self) -> Option<Int> {\n\
                       if self.n >= self.max { return None }\n\
                       println(\"pull ${self.n}\")\n\
                       let x = self.n\n\
                       self.n = self.n + 1\n\
                       Some(x)\n\
                   }\n\
               }\n\
               fn main() {\n\
                   let src = CountingSrc { n: 0, max: 5 }\n\
                   println(\"${src.any(|n| n > 1)}\")\n\
               }";
    let dir = std::env::temp_dir().join("nova-iterator-any-short-circuits");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("main.nova");
    std::fs::write(&path, src).expect("write");
    nova()
        .arg("run")
        .arg(&path)
        .assert()
        .success()
        .stdout("pull 0\npull 1\npull 2\ntrue\n");
}

/// A design question carried from Task 2's review, not merely assumed:
/// `fold`/`any`/`collect` are default methods on `Iterator` itself, so their
/// bodies see `Self` at `Param(0)` (`crates/nova-typeck/src/check.rs`, the
/// `MethodOwner::TraitDefault` arm) exactly as a free function's own bounded
/// type parameter does. Task 3's mutation testing already observed the
/// diagnostic name for that placeholder directly — `` `Option<T0::Item>` `` in
/// Finding 8 of `task-3-report.md` — and Task 2 documented that a generic
/// `fn f<T: Iterator>(mut it: T)` binds its loop variable at that same
/// unnormalized projection, so an operation requiring `T::Item` concretely
/// (`n + x`, string interpolation) is `E0010`/`E0013` there — not a
/// regression, since the hand-written `while`+`match` equivalent behaves
/// identically.
///
/// So: do `fold`, `any` and `collect` actually work when called on a
/// *generic* `I: Iterator`, not only on a concrete `VecIter`, given that
/// their own signatures name `Self::Item` in a parameter or return type the
/// same way `first_or`'s `dflt: I::Item` does in
/// `a_generic_function_over_iterator_resolves_item_per_instantiation`? Answer,
/// measured directly rather than inferred from the above: **yes, as long as
/// the item is used opaquely.** `count_via_fold` never inspects `x` (`|a, x|
/// a + 1`), `any_via_generic` never inspects it either (`|x| true`), and
/// `len_via_collect` only calls `.len()` on the `Vec<I::Item>` `collect`
/// hands back — none of that requires `I::Item` to resolve to anything
/// concrete, so all three type-check, monomorphize and run correctly. Trying
/// to use the item concretely inside such a generic helper — `|a, x| a + x`,
/// or interpolating `x` — reproduces the exact `T0::Item`/E0010 and
/// `?0`/E0013 shapes Task 2 and Task 3 already documented, confirmed by
/// hand-testing both during this task. That does mean `v.iter().map(f)
/// .fold(...)` works while a generic
/// `fn sums<I: Iterator>(mut it: I) -> Int { it.fold(0, |a, x| a + x) }`
/// does not — a real, pre-existing gap, worth stating plainly rather than
/// leaving implicit.
///
/// **Correction, caught in review:** an earlier round of this comment claimed
/// the negative case above was "not re-pinned here since Task 2's own test
/// already owns that limitation." It does not, and nothing anywhere pinned
/// it: `a_generic_function_over_iterator_resolves_item_per_instantiation`
/// (Task 2) is purely positive — `I::Item` flows through `first`/`first_or`'s
/// signatures and is returned or matched, never operated on concretely — so
/// no existing test asserted the negative case failed, only that this task's
/// own hand-testing observed it once and moved on.
/// `a_generic_iterator_bound_cannot_use_its_item_concretely_in_fold`, below,
/// closes that.
///
/// None of the three helpers below declare `mut it: I` any more (an earlier
/// round of this test did): `fold`/`any`/`collect` take plain `self` now, so
/// nothing here requires the caller's own parameter to already be a mutable
/// place either.
#[test]
fn iterator_default_methods_work_through_a_generic_iterator_bound() {
    let src = "fn count_via_fold<I: Iterator>(it: I) -> Int {\n\
                   it.fold(0, |a, x| a + 1)\n\
               }\n\
               fn any_via_generic<I: Iterator>(it: I) -> Bool {\n\
                   it.any(|x| true)\n\
               }\n\
               fn len_via_collect<I: Iterator>(it: I) -> Int {\n\
                   it.collect().len()\n\
               }\n\
               fn main() {\n\
                   let mut v = Vec::new()\n\
                   v.push(1)\n\
                   v.push(2)\n\
                   v.push(3)\n\
                   println(\"${count_via_fold(v.iter())}\")\n\
                   println(\"${any_via_generic(v.iter())}\")\n\
                   println(\"${len_via_collect(v.iter())}\")\n\
               }";
    let dir = std::env::temp_dir().join("nova-iterator-default-methods-generic-bound");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("main.nova");
    std::fs::write(&path, src).expect("write");
    nova()
        .arg("run")
        .arg(&path)
        .assert()
        .success()
        .stdout("3\ntrue\n3\n");
}

/// The negative half of the design question above, pinned rather than only
/// described. Task 2 documented (and this task's own hand-testing
/// re-confirmed) that a generic `fn f<T: Iterator>(mut it: T)` binds a value
/// of type `T::Item` at the unnormalized projection, so an operation
/// requiring it concretely — here, `+` inside `fold`'s own closure — is
/// `E0010` naming that projection by its internal placeholder (`T0::Item`,
/// the same name Task 3's mutation testing observed independently in a
/// different diagnostic). Not a regression: the hand-written `while`+`match`
/// equivalent behaves identically, and this was true before Task 4 added a
/// single default method. Pinned so a future change to how generic default
/// methods normalize `Self::Item` either keeps failing this exact way on
/// purpose, or flips this assertion deliberately — not silently, in either
/// direction.
#[test]
fn a_generic_iterator_bound_cannot_use_its_item_concretely_in_fold() {
    let src = "fn sum_via_fold<I: Iterator>(it: I) -> Int {\n\
                   it.fold(0, |a, x| a + x)\n\
               }\n\
               fn main() {}";
    let dir = std::env::temp_dir().join("nova-iterator-generic-item-not-concrete");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("main.nova");
    std::fs::write(&path, src).expect("write");
    let assert = nova().arg("check").arg(&path).assert().failure();
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr).to_string();
    assert!(
        stderr.contains("error[E0010]") && stderr.contains("T0::Item"),
        "stderr: {stderr}"
    );
}

/// The design this task was chosen on, pinned at exactly the shape a user
/// will write: `v.iter().filter(…).map(…).collect()` as one expression, no
/// intermediate binding anywhere in the chain. This was `E0060` (`collect`
/// mutates its receiver, which cannot be a temporary) until `fold`/`count`/
/// `any`/`collect` moved from `mut self` to plain `self`, each rebinding
/// internally (`let mut it = self`) instead of requiring the caller to have
/// already named a mutable place — `trait Iterator`'s own doc comment above
/// `fold` (`std/core/lib.nova`) explains why. Deliberately narrower than
/// `iterator_default_methods_work_and_chain` above (which exercises all six
/// methods and several values at once): this test's only job is to fail
/// loudly and specifically if the advertised chained form ever stops
/// compiling, rather than being one assertion among many in a larger test.
///
/// The second chain closes a coverage gap review found: `collect` is the one
/// new method whose own *return type* names the projection
/// (`Vec<Self::Item>`), which makes it the most exposed of the six to the
/// monomorphization seam — `mir_ty` collapses `Int`/`Char` into `MirTy::I64`
/// and `String`/`Fn`/`Sum`/`Record`/`Array` into `MirTy::Ptr`, and since `Ptr`
/// is `i64` on x86-64 those two classes are the same machine width (they do
/// stay distinct at codegen). `Bool` is `I8` and `Float` is `F64`, disjoint
/// from both, so only those two actually discriminate
/// a wrong item type from a coincidentally-right one (the same reasoning
/// Task 3 applied to its own chain, and the brief applied to `any` above but
/// never to `collect`). Every `collect` call elsewhere in this task's tests
/// collects `Int`; this one collects `Bool` instead
/// (`v.iter().map(|n| n > 2)`), so a `collect` that silently forwarded the
/// source's `Int` items instead of the mapped `Bool` ones has something to go
/// visibly wrong.
#[test]
fn iterator_adapter_chain_compiles_and_runs_as_a_single_expression() {
    let src = "fn main() {\n\
      let mut v = Vec::new()\n\
      v.push(1)\n\
      v.push(2)\n\
      v.push(3)\n\
      let got = v.iter().filter(|n| n > 1).map(|n| n * 10).collect()\n\
      println(\"${got.len()}\")\n\
      println(\"${got.get(0).unwrap()}\")\n\
      let bools = v.iter().map(|n| n > 2).collect()\n\
      println(\"${bools.len()}\")\n\
      println(\"${bools.get(2).unwrap()}\")\n\
    }";
    let dir = std::env::temp_dir().join("nova-iterator-chain-single-expression");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("main.nova");
    std::fs::write(&path, src).expect("write");
    nova()
        .arg("run")
        .arg(&path)
        .assert()
        .success()
        .stdout("2\n20\n3\ntrue\n");
}

/// The go/no-go check for the `mut self` -> `self` design change: `fold`,
/// `count`, `any` and `collect` now rebind their receiver internally
/// (`let mut it = self`) instead of requiring the caller's own binding to
/// already be mutable. Records are heap objects with no copy semantics
/// anywhere in this compiler (grepped for "moved value"/"use after move"/
/// "linear type"/"affine type" across every crate during Task 4's original
/// work; zero hits), so `it` is expected to alias exactly the storage `self`
/// pointed to rather than copy it — but "expected" is not "verified," and
/// this is precisely the kind of change a silent value-copy would make look
/// identical for every test that only checks a consumer's *return* value.
///
/// `Cursor` exposes its own cursor field (`n`) directly, so this reads the
/// iterator's storage back out through the *original* binding after a
/// consumer has run on it, rather than inferring aliasing indirectly through
/// a second `.next()` call. Two consumers, deliberately different: `any`
/// short-circuits (stops at the first match, `x > 1` first true at `2`), so
/// `a.n` reading `3` afterward (not `0`, a copy; not `5`, a full scan) proves
/// aliasing *and* confirms the short-circuit itself survived the receiver
/// rebinding, in one assertion. `count` runs to exhaustion, so `b.n` reading
/// `3` afterward confirms the same aliasing on the other kind of consumer —
/// one that always visits every element rather than stopping early.
#[test]
fn an_iterators_own_storage_still_advances_when_a_consumer_takes_plain_self() {
    let src = "record Cursor { n: Int, max: Int }\n\
               impl Iterator for Cursor {\n\
                   type Item = Int\n\
                   fn next(mut self) -> Option<Int> {\n\
                       if self.n >= self.max { return None }\n\
                       let x = self.n\n\
                       self.n = self.n + 1\n\
                       Some(x)\n\
                   }\n\
               }\n\
               fn main() {\n\
                   let mut a = Cursor { n: 0, max: 5 }\n\
                   println(\"${a.any(|x| x > 1)}\")\n\
                   println(\"${a.n}\")\n\
                   let mut b = Cursor { n: 0, max: 3 }\n\
                   println(\"${b.count()}\")\n\
                   println(\"${b.n}\")\n\
               }";
    let dir = std::env::temp_dir().join("nova-iterator-consumer-aliases-storage");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("main.nova");
    std::fs::write(&path, src).expect("write");
    nova()
        .arg("run")
        .arg(&path)
        .assert()
        .success()
        .stdout("true\n3\n3\n3\n");
}

/// `continue` inside `for x in <an iterator>`. Nothing covered it anywhere:
/// `tests/runtime/break_continue.nova` is range-only and `tests/runtime/
/// iterator.nova` has no `continue` at all. Written here rather than added to
/// that fixture so the gate's checked-in `.stdout` stays untouched.
///
/// Worth its own test because the two `for` desugars get this right by
/// *opposite* mechanisms, so a test of one says nothing about the other. The
/// range form has to deliberately hoist its increment above the body, precisely
/// because a `continue` would otherwise jump past it (its own comments in
/// `check_for_range`, `crates/nova-typeck/src/check.rs`, say so). The iterator
/// form gets it for free: `next()` is the `match` scrutinee, so it sits *in* the
/// body and a `continue` re-enters through it. That freeness is fragile in a
/// specific, plausible way: hoisting `next()` out of the body so the `while true`
/// placeholder can carry a real condition forces the advance to the *end* of the
/// body, which is exactly where a `continue` jumps past it — the range desugar's
/// hazard, imported. That restructure is a refactor rather than a one-line
/// mutation, so it was **not** run here; what was measured is narrower and
/// enough to justify the test. Removing `check_for_iterator`'s
/// `fcx.loop_depth` bracket fails this test and
/// `a_map_iter_calls_f_lazily_not_all_up_front` and nothing else in the
/// workspace — so `break` in this position already had exactly one pin, and
/// `continue` had none anywhere: before this test the only `continue` in any
/// fixture or suite was over an integer *range*.
///
/// `break` is exercised in the same loop, and the totals discriminate: skipping
/// `2` and stopping before `4` gives `1 + 3 = 4`, which no other combination of
/// "continue ignored" (`1+2+3 = 6`), "break ignored" (`1+3+4+5 = 13`) or "both
/// ignored" (`15`) produces. The element counter separates a `continue` that
/// skipped the rest of the body from one that skipped the *iteration*.
#[test]
fn continue_and_break_work_inside_a_for_loop_over_an_iterator() {
    let src = "fn main() {\n\
                   let mut v = Vec::new()\n\
                   v.push(1)\n\
                   v.push(2)\n\
                   v.push(3)\n\
                   v.push(4)\n\
                   v.push(5)\n\
                   let mut total = 0\n\
                   let mut seen = 0\n\
                   for x in v.iter() {\n\
                       seen = seen + 1\n\
                       if x == 2 { continue }\n\
                       if x == 4 { break }\n\
                       total = total + x\n\
                   }\n\
                   println(\"total=${total} seen=${seen}\")\n\
               }";
    let dir = std::env::temp_dir().join("nova-iterator-for-continue");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("main.nova");
    std::fs::write(&path, src).expect("write");
    nova()
        .arg("run")
        .arg(&path)
        .assert()
        .success()
        .stdout("total=4 seen=4\n");
}

/// `for x in it` where `it: I` is a **generic parameter bounded by `Iterator`**.
/// Nothing covered it: `a_generic_function_over_iterator_resolves_item_per_
/// instantiation` above drives `next()` by hand, and every `for`-over-iterator
/// test binds a concrete receiver.
///
/// It is the one `for` path whose loop variable binds at an **unnormalized**
/// projection. `check_for_iterator` reads the item type off `next`'s result with
/// a raw `payload.subst(&args)` — correct by design, since inside a generic
/// function there is no impl to normalize against yet — so `x`'s type stays
/// `Assoc { on: Param(i) }` all the way to monomorphization, and seam 3 is the
/// only thing that ever resolves it. That is exactly the configuration whose
/// failure mode was this area's worst historical bug: a projection reaching
/// `mir_ty` as `MirTy::Unit`, which drops the parameter from the Cranelift
/// signature rather than reporting anything.
///
/// **Instantiated at `Bool` and at `Float`, not just `Int`.** `mir_ty` maps
/// `Int` and `Char` to `I64` and every heap type to `Ptr` (also `i64` on
/// x86-64), so an `Int` item type is the weakest possible choice — a wrong
/// answer hides inside its own machine class. `Bool` is `I8` and `Float` is
/// `F64`; `Float` is the strictly stronger of the two, because it crosses
/// register banks, where `Bool`'s only values (0 and 1) survive an `I64`
/// confusion intact.
///
/// Two shapes, because counting alone would not notice a wrong item type.
/// `count_via_for` only increments, so it pins that the loop *runs* the right
/// number of times over a bounded generic. `first_via_for` carries the loop
/// variable back out at `Option<I::Item>` and lets `main` consume it
/// concretely — `if b.unwrap()` needs a real `Bool` and `${f.unwrap()}`
/// dispatches `Display` on a real `Float`. The `Bool` source yields `true`
/// first on purpose: a payload dropped or zeroed by a `MirTy::Unit` item type
/// would read back as `false`, so `true` is the value that cannot be faked.
#[test]
fn a_for_loop_over_a_generic_iterator_bound_binds_its_item_at_each_instantiation() {
    let src = "record BoolSrc { n: Int }\n\
               impl Iterator for BoolSrc {\n\
                   type Item = Bool\n\
                   fn next(mut self) -> Option<Bool> {\n\
                       if self.n >= 3 { return None }\n\
                       self.n = self.n + 1\n\
                       Some(self.n == 1)\n\
                   }\n\
               }\n\
               record FloatSrc { n: Int }\n\
               impl Iterator for FloatSrc {\n\
                   type Item = Float\n\
                   fn next(mut self) -> Option<Float> {\n\
                       if self.n >= 2 { return None }\n\
                       self.n = self.n + 1\n\
                       Some(2.5)\n\
                   }\n\
               }\n\
               fn first_via_for<I: Iterator>(it: I) -> Option<I::Item> {\n\
                   for x in it { return Some(x) }\n\
                   None\n\
               }\n\
               fn count_via_for<I: Iterator>(it: I) -> Int {\n\
                   let mut n = 0\n\
                   for x in it { n = n + 1 }\n\
                   n\n\
               }\n\
               fn main() {\n\
                   let b = first_via_for(BoolSrc { n: 0 })\n\
                   if b.unwrap() { println(\"bool=true\") } else { println(\"bool=false\") }\n\
                   let f = first_via_for(FloatSrc { n: 0 })\n\
                   println(\"float=${f.unwrap()}\")\n\
                   println(\"bools=${count_via_for(BoolSrc { n: 0 })}\")\n\
                   println(\"floats=${count_via_for(FloatSrc { n: 0 })}\")\n\
               }";
    let dir = std::env::temp_dir().join("nova-iterator-for-generic-bound");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("main.nova");
    std::fs::write(&path, src).expect("write");
    nova()
        .arg("run")
        .arg(&path)
        .assert()
        .success()
        .stdout("bool=true\nfloat=2.5\nbools=3\nfloats=2\n");
}

/// Iteration end-to-end gate (Phase 2.2d, Task 5). One program covering every
/// item of the increment's gate list — `for` over a `Vec` via `.iter()`, `for`
/// over an integer range (a regression guard, since Task 2 rewrote the function
/// that dispatches a `for` head), exhaustion plus one further `next()`, an
/// empty source whose *first* `next()` is already `None`, a two-stage
/// `.filter().map()` chain, all six default methods, and every generic block
/// instantiated at `Bool` **and** at `Float`.
///
/// That last item is the load-bearing one and the reason this fixture is not
/// simply the `Int` pipeline a reader would write first. `mir_ty` maps `Int`
/// *and* `Char` to `MirTy::I64` and `String`/`Fn`/`Record`/`Sum`/`Array` to
/// `MirTy::Ptr` — `pointer_type()`, `types::I64` on x86-64 — so at the level a
/// backend can see, `Int`, `Char`, `String` and every heap type are one type,
/// and an `Item` resolved to the wrong one of them is invisible after
/// lowering. Only `Bool` (`I8`) and `Float` (`F64`) have machine classes a
/// wrong answer cannot hide in. `tests/runtime/assoc_types.nova`'s header
/// records the measurement that established this; this fixture's own header
/// carries its mutation-to-line map.
#[test]
fn iterator_run() {
    let expected = std::fs::read_to_string(repo_root().join("tests/runtime/iterator.stdout"))
        .expect("expected-output fixture exists")
        .replace("\r\n", "\n");
    nova()
        .arg("run")
        .arg(repo_root().join("tests/runtime/iterator.nova"))
        .assert()
        .success()
        .stdout(expected);
}

/// The same fixture through the object-file backend. Not redundant with
/// `iterator_run` for the reason `strings_build_standalone`'s comment gives —
/// `nova run` resolves runtime symbols through `nova_runtime::symbols()` while
/// `nova build` links `nova_runtime.lib` by export name and never consults that
/// table — and for one specific to this gate: an `Item` resolved to the wrong
/// machine class fails in **code generation**, not in the type checker, so the
/// two backends are the two places it can fail differently.
#[test]
fn iterator_build_standalone() {
    let expected = std::fs::read_to_string(repo_root().join("tests/runtime/iterator.stdout"))
        .expect("expected-output fixture exists")
        .replace("\r\n", "\n");
    let out = build_and_run("tests/runtime/iterator.nova", "iterator");
    assert_eq!(out.replace("\r\n", "\n"), expected);
}

/// The same fixture with `NOVA_GC_STRESS=1` (collect on every allocation) — a
/// gate criterion, not belt-and-braces. A `.filter().map()` chain is a *tower*
/// of records, so the backing array is reachable only as
/// `MapIter` -> `FilterIter` -> `VecIter` -> `Vec` -> `data` while every `next`
/// allocates a fresh `Option` and `collect` grows a `Vec` — a strictly deeper
/// rooting chain than `assoc_types_under_gc_stress`'s three levels. The
/// collector scans the stack plus callee-saved registers, which is what makes
/// an adapter held across a call a root; a missed root at any link loses
/// elements silently rather than crashing.
#[test]
fn iterator_under_gc_stress() {
    let expected = std::fs::read_to_string(repo_root().join("tests/runtime/iterator.stdout"))
        .expect("expected-output fixture exists")
        .replace("\r\n", "\n");
    nova()
        .env("NOVA_GC_STRESS", "1")
        .arg("run")
        .arg(repo_root().join("tests/runtime/iterator.nova"))
        .assert()
        .success()
        .stdout(expected);
}

// === Task 5 Step 6a: the three cases ADR 0007 section 1's safety argument
// rests on, none of which had a test.
//
// A bound on a record's type parameter is a **resolution scope, not a
// constraint** (ADR 0007): it makes a projection nameable in a field type and
// is not itself checked when the record is built. That decision is only
// defensible because of what happens instead — and "what happens instead" was
// written down twice before it was measured, wrong both times (first as a
// uniform `E0014`, then as a uniform `E0013`). It is not uniform. It is three
// different answers depending on whether the bound reaches a field type, and
// the third of them is a hole.
//
// All three need monomorphization, so none can be a `check_src` unit test in
// `nova-typeck`: `E0079` and `E0013` are both raised in `nova-mir`'s `mono`,
// after typeck has finished, and case 3 has to run to completion.

/// **Case 1 — the shape the stdlib actually ships, and the one that was
/// entirely unpinned.** `MapIter`/`FilterIter` carry a bound *because a field
/// type names a projection on it* (`f: fn(I::Item) -> U`). Substituting
/// `I := Int` leaves `Int::Item` in that field's declared type, nothing binds
/// `Item` for `Int`, and monomorphization rejects it — `E0079`, the
/// surviving-projection check built in Phase 2.2c Task 7.
///
/// Asserted **without driving the iterator**, because firing at *construction*
/// is the property. That makes the real behaviour earlier and stronger than
/// "the bound is unenforced" suggests, and it is why the ADR does not have to
/// argue that a bogus `MapIter` is merely inert: for this shape it cannot be
/// built at all.
///
/// Checked by the `Int::Item` spelling, not only by the code: `E0079` is also
/// the backstop for a projection surviving mono by any other route, so the code
/// alone would not distinguish this cause from that one.
#[test]
fn a_wrong_instantiation_of_a_projection_shaped_record_is_e0079_at_construction() {
    // No `.next()`, no `for`, no consumer — the value is built and dropped.
    let src = "fn main() {\n\
                   let m = MapIter { it: 5, f: |x| x }\n\
                   println(\"built\")\n\
               }";
    let dir = std::env::temp_dir().join("nova-record-bound-e0079");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("main.nova");
    std::fs::write(&path, src).expect("write");
    let assert = nova().arg("run").arg(&path).assert().failure();
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr).to_string();
    assert!(
        stderr.contains("error[E0079]") && stderr.contains("`Int::Item`"),
        "stderr: {stderr}"
    );
    // And it really is at construction: nothing ran.
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout).to_string();
    assert!(
        !stdout.contains("built"),
        "rejected before `main` ran: {stdout}"
    );
}

/// **Case 2 — the bound reaches no field type, but is exercised through a
/// bounded impl method.** `Boxed<K: Hash + Eq, V>`'s bound is inert on the
/// record itself (neither `k: K` nor `v: V` names a projection), so nothing
/// stops `Boxed { k: NoHash { .. }, v: 7 }` being built. The
/// `impl<K: Hash + Eq, V>` block's bound is real, and instantiating `key` with
/// a `K` implementing neither trait is `E0013` — one diagnostic per unsatisfied
/// bound.
///
/// This is the shape an earlier round of the plan transcribed onto `MapIter`
/// and got wrong. `E0013` *was* measured, but on a record whose field types
/// name no projection, so case 1's earlier and stricter `E0079` never fires
/// here. Different shape, different diagnostic — which is why both are pinned
/// rather than one standing in for the other.
///
/// Asserted on the **bound spelling** (NoHash: Hash), not just the code:
/// `E0013` is the code for every unsatisfied bound in the language, so the code
/// alone would still pass if the diagnostic named the wrong trait or the wrong
/// type. Both bounds are checked, since reporting only the first would be a
/// silent regression in a diagnostic whose whole value is completeness.
#[test]
fn an_unused_record_bound_is_still_enforced_through_a_bounded_impl_method() {
    let src = "record Boxed<K: Hash + Eq, V> { k: K\n\
               v: V }\n\
               impl<K: Hash + Eq, V> Boxed<K, V> {\n\
                   fn key(self) -> K { self.k }\n\
               }\n\
               record NoHash { z: Int }\n\
               fn main() {\n\
                   let b = Boxed { k: NoHash { z: 1 }, v: 7 }\n\
                   let k = b.key()\n\
                   println(\"${k.z}\")\n\
               }";
    let dir = std::env::temp_dir().join("nova-record-bound-e0013");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("main.nova");
    std::fs::write(&path, src).expect("write");
    let assert = nova().arg("run").arg(&path).assert().failure();
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr).to_string();
    assert!(
        stderr.contains("error[E0013]") && stderr.contains("`NoHash: Hash`"),
        "stderr: {stderr}"
    );
    assert!(
        stderr.contains("`NoHash: Eq`"),
        "one diagnostic per unsatisfied bound, not just the first: {stderr}"
    );
}

/// **Case 3 — the residual hole, pinned as accepted.** The same `Boxed`, the
/// same non-conforming `K`, but only the *unbounded* field is read, so no
/// bounded method is ever instantiated. It compiles, runs and prints. No
/// diagnostic anywhere.
///
/// So the claim "a bogus instantiation is never silently useless" — which an
/// earlier round of this plan asserted — is **false**, and this test is what
/// stops that being rediscovered as a bug. It also stops a future enforcement
/// change landing silently: threading type arguments through `MakeRecord` would
/// turn this program into an error, which should be a deliberate act with a
/// test to change rather than a quiet tightening.
///
/// Success is asserted **explicitly**, on the exact stdout. A test that merely
/// omitted an error assertion would also pass if the program failed to compile
/// for some unrelated reason.
#[test]
fn a_record_bound_no_field_type_uses_is_silently_accepted_when_never_exercised() {
    let src = "record Boxed<K: Hash + Eq, V> { k: K\n\
               v: V }\n\
               impl<K: Hash + Eq, V> Boxed<K, V> {\n\
                   fn key(self) -> K { self.k }\n\
               }\n\
               record NoHash { z: Int }\n\
               fn main() {\n\
                   let b = Boxed { k: NoHash { z: 1 }, v: 7 }\n\
                   println(\"${b.v}\")\n\
               }";
    let dir = std::env::temp_dir().join("nova-record-bound-accepted");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("main.nova");
    std::fs::write(&path, src).expect("write");
    nova()
        .arg("run")
        .arg(&path)
        .assert()
        .success()
        .stdout("7\n");
}

// === nova test: the synthesized dispatching `main` (Task 3) ===

/// The synthesized `main` with no `NOVA_TEST_INDEX` set prints the
/// inventory: a count line, then one name per line. This is how `nova test`
/// (Task 5) will enumerate tests without a second compilation, and it is
/// also what a human gets by running the binary directly with no variable
/// set.
///
/// Asserts **exact** stdout, not `contains`. The mutation table this task
/// carries includes `unwrap_or(0)` in place of `unwrap_or(-1)` inside
/// `nova_rt_test_selector`, which makes a bare run silently execute test 0
/// (`alpha`) instead of printing the inventory. Verified directly: under
/// that mutation, stdout goes from `"2\nalpha\nbeta\n"` to `""` (`alpha`'s
/// own body is empty), which this exact-equality assertion correctly fails
/// on.
#[test]
fn a_test_binary_with_no_index_prints_its_inventory() {
    let dir = std::env::temp_dir().join("nova-test-inventory");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let file = dir.join("inventory.nova");
    std::fs::write(&file, "@test\nfn alpha() { }\n@test\nfn beta() { }\n").expect("write");

    let (exe, tests) =
        nova_driver::build_test_binary(&file).expect("test binary compiles and links");
    assert_eq!(
        tests.iter().map(|t| t.name.as_str()).collect::<Vec<_>>(),
        vec!["alpha", "beta"],
        "collection order feeds the printed inventory directly"
    );

    let assert = Command::new(&exe)
        .env_remove("NOVA_TEST_INDEX")
        .assert()
        .success();
    let out = String::from_utf8(assert.get_output().stdout.clone())
        .expect("stdout is UTF-8")
        .replace("\r\n", "\n");
    assert_eq!(out, "2\nalpha\nbeta\n");
    let _ = std::fs::remove_file(&exe);
}

/// `NOVA_TEST_INDEX=1` runs the second collected test and nothing else. The
/// two tests print distinguishable markers, so running the wrong one — or
/// both — is visible rather than merely a wrong count.
///
/// Asserts stdout is **exactly** `"B\n"`, not merely that `"B"` appears.
/// Asserting presence alone would still pass if the dispatch ran *both*
/// `alpha` and `beta` (stdout `"A\nB\n"` contains `"B"`), which is exactly
/// the failure mode a mutated dispatch condition (e.g. comparing `sel + 1`
/// against each index instead of `sel`) must be caught producing. Exact
/// equality both asserts the selected test's output and stands in for a
/// separate "and `A` is absent" check: no string equal to `"B\n"` also
/// contains `"A"`.
#[test]
fn a_test_binary_runs_exactly_the_selected_test() {
    let dir = std::env::temp_dir().join("nova-test-selected");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let file = dir.join("selected.nova");
    std::fs::write(
        &file,
        "@test\nfn alpha() { println(\"A\") }\n\
         @test\nfn beta() { println(\"B\") }\n",
    )
    .expect("write");

    let (exe, tests) =
        nova_driver::build_test_binary(&file).expect("test binary compiles and links");
    assert_eq!(
        tests.iter().map(|t| t.name.as_str()).collect::<Vec<_>>(),
        vec!["alpha", "beta"],
        "index 1 below selects beta only if collection order is [alpha, beta]"
    );

    let assert = Command::new(&exe)
        .env("NOVA_TEST_INDEX", "1")
        .assert()
        .success();
    let out = String::from_utf8(assert.get_output().stdout.clone())
        .expect("stdout is UTF-8")
        .replace("\r\n", "\n");
    assert_eq!(out, "B\n");
    let _ = std::fs::remove_file(&exe);
}

/// A source file with both `@test` functions and its own `fn main()` must
/// still run the selected test, not the user's `main`. Before this fix,
/// `build_test_binary` appended the synthesized dispatcher to the *end* of
/// `module.functions`, but `nova-mir/src/mono.rs`'s entry-point search
/// (`.iter().find(|f| f.name == "main")`) takes the *first* match — so an
/// earlier user `main` won, compiled and linked cleanly with no diagnostic
/// anywhere, and the dispatcher became dead code mono never even emitted.
/// `nova build`/`nova run` *require* a `main` (`E0601`), so a program with
/// both tests and an entry point is the unremarkable case, not an edge one.
#[test]
fn a_test_binary_runs_the_selected_test_not_the_users_own_main() {
    let dir = std::env::temp_dir().join("nova-test-user-main");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let file = dir.join("user_main.nova");
    std::fs::write(
        &file,
        "@test\nfn alpha() { println(\"TEST-RAN\") }\n\
         fn main() { println(\"USER-MAIN-RAN\") }\n",
    )
    .expect("write");

    let (exe, tests) =
        nova_driver::build_test_binary(&file).expect("test binary compiles and links");
    assert_eq!(
        tests.iter().map(|t| t.name.as_str()).collect::<Vec<_>>(),
        vec!["alpha"]
    );

    let assert = Command::new(&exe)
        .env("NOVA_TEST_INDEX", "0")
        .assert()
        .success();
    let out = String::from_utf8(assert.get_output().stdout.clone())
        .expect("stdout is UTF-8")
        .replace("\r\n", "\n");
    assert_eq!(out, "TEST-RAN\n");
    let _ = std::fs::remove_file(&exe);
}

/// Two different source files that happen to share a file stem — `main.nova`
/// is overwhelmingly the common fixture name, including 50-plus occurrences
/// in this very file — must not collide on `build_test_binary`'s output
/// path. Before this fix the output path was derived from the stem alone
/// inside one shared directory, so two different `main.nova` files (in two
/// different directories, as they always are among real fixtures) resolved
/// to the identical path — a real race under `cargo test`'s default
/// parallelism, and one Task 5's runner (which builds several fixtures)
/// would hit directly.
#[test]
fn test_binary_output_paths_do_not_collide_across_same_stem_sources() {
    let dir_a = std::env::temp_dir().join("nova-test-stem-collision-a");
    let dir_b = std::env::temp_dir().join("nova-test-stem-collision-b");
    std::fs::create_dir_all(&dir_a).expect("temp dir a");
    std::fs::create_dir_all(&dir_b).expect("temp dir b");
    let file_a = dir_a.join("main.nova");
    let file_b = dir_b.join("main.nova");
    std::fs::write(&file_a, "@test\nfn from_a() { }\n").expect("write a");
    std::fs::write(&file_b, "@test\nfn from_b() { }\n").expect("write b");

    let (exe_a, _) = nova_driver::build_test_binary(&file_a).expect("a compiles and links");
    let (exe_b, _) = nova_driver::build_test_binary(&file_b).expect("b compiles and links");

    assert_ne!(
        exe_a, exe_b,
        "two different main.nova files must not share an output path"
    );
    assert!(exe_a.exists(), "a's binary exists at {}", exe_a.display());
    assert!(exe_b.exists(), "b's binary exists at {}", exe_b.display());
    let _ = std::fs::remove_file(&exe_a);
    let _ = std::fs::remove_file(&exe_b);
}

// === std/test: assert, assert_eq, assert_ne (Task 4) ===

/// `assert_eq` must report BOTH values. A message that dropped one would
/// still look like a failure report, and every other assertion in std/test
/// would still "work" — so this asserts on the rendered text, not just that
/// the process panicked.
///
/// Asserts the message is **exactly** `"nova: panic: assertion failed: 1 !=
/// 3"`, not `contains("1") && contains("3")`. Those are single characters and
/// would match a line number, a byte offset, a file path, or the test index
/// too, so a two-fragment `contains` check passes against a completely
/// garbled message. This is the defect that has produced every Important
/// finding on this branch so far.
#[test]
fn assert_eq_failure_names_both_values() {
    let dir = std::env::temp_dir().join("nova-test-assert-eq-both-values");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let file = dir.join("both_values.nova");
    std::fs::write(&file, "@test\nfn t() { assert_eq(1, 3) }\n").expect("write");

    let (exe, tests) =
        nova_driver::build_test_binary(&file).expect("test binary compiles and links");
    assert_eq!(
        tests.iter().map(|t| t.name.as_str()).collect::<Vec<_>>(),
        vec!["t"]
    );

    let assert = Command::new(&exe)
        .env("NOVA_TEST_INDEX", "0")
        .assert()
        .failure();
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr).replace("\r\n", "\n");
    assert_eq!(
        stderr.trim_end(),
        "nova: panic: assertion failed: 1 != 3",
        "stderr: {stderr}"
    );
    let _ = std::fs::remove_file(&exe);
}

/// `assert_eq<T: Eq + Debug>` instantiated at `Bool` and `Float`, not only
/// `Int`. `mir_ty` maps `Int` and `Char` to `MirTy::I64` and every heap type
/// to `MirTy::Ptr`, so only `Bool` (`I8`) and `Float` (`F64`) are disjoint
/// from both — and `Float` is the stronger of the two because it crosses
/// register banks while `Bool`'s 0/1 would survive an `I64` mix-up intact.
/// `String` is included too, since it is the one heap type whose `Debug`
/// output (quoted) differs textually from its `Display` output.
///
/// Each type gets a PASSING case and a FAILING one. Passing cases alone prove
/// nothing: `assert_eq(1, 1)`, `(true, true)`, `(0.5, 0.5)` and `("a", "a")`
/// all succeed even if `assert_eq` never compares its arguments — implement
/// `Eq::ne` as `{ false }` and every one of them still passes. The failing
/// case is what proves the comparison actually happened; the passing case
/// only proves it does not fire spuriously.
#[test]
fn assert_eq_works_at_bool_and_float_not_only_int() {
    let dir = std::env::temp_dir().join("nova-test-assert-eq-types");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let file = dir.join("types.nova");
    std::fs::write(
        &file,
        "@test\nfn int_pass() { assert_eq(1, 1) }\n\
         @test\nfn int_fail() { assert_eq(1, 2) }\n\
         @test\nfn bool_pass() { assert_eq(true, true) }\n\
         @test\nfn bool_fail() { assert_eq(true, false) }\n\
         @test\nfn float_pass() { assert_eq(0.5, 0.5) }\n\
         @test\nfn float_fail() { assert_eq(0.5, 1.5) }\n\
         @test\nfn string_pass() { assert_eq(\"a\", \"a\") }\n\
         @test\nfn string_fail() { assert_eq(\"a\", \"b\") }\n",
    )
    .expect("write");

    let (exe, tests) =
        nova_driver::build_test_binary(&file).expect("test binary compiles and links");
    assert_eq!(
        tests.iter().map(|t| t.name.as_str()).collect::<Vec<_>>(),
        vec![
            "int_pass",
            "int_fail",
            "bool_pass",
            "bool_fail",
            "float_pass",
            "float_fail",
            "string_pass",
            "string_fail",
        ],
        "collection order fixes which index below selects which case"
    );

    // (index, expected panic message; `None` means this index must pass with
    // no output at all, since a passing `assert_eq` prints nothing).
    let cases: [(usize, Option<&str>); 8] = [
        (0, None),
        (1, Some("nova: panic: assertion failed: 1 != 2")),
        (2, None),
        (3, Some("nova: panic: assertion failed: true != false")),
        (4, None),
        (5, Some("nova: panic: assertion failed: 0.5 != 1.5")),
        (6, None),
        (7, Some("nova: panic: assertion failed: \"a\" != \"b\"")),
    ];

    for (index, expected_panic) in cases {
        let assert = Command::new(&exe)
            .env("NOVA_TEST_INDEX", index.to_string())
            .assert();
        match expected_panic {
            None => {
                let assert = assert.success();
                let out = String::from_utf8_lossy(&assert.get_output().stdout).to_string();
                assert_eq!(
                    out, "",
                    "a passing assert_eq must print nothing (index {index})"
                );
            }
            Some(msg) => {
                let assert = assert.failure();
                let stderr =
                    String::from_utf8_lossy(&assert.get_output().stderr).replace("\r\n", "\n");
                assert_eq!(stderr.trim_end(), msg, "index {index}");
            }
        }
    }
    let _ = std::fs::remove_file(&exe);
}

/// `assert`/`assert_eq`/`assert_ne` must NOT leak into an ordinary program:
/// `std/test` is seeded only under `nova test` (`build_test_binary`), not
/// into `nova run`/`nova check`/`nova build`. A top-level `pub fn assert` in
/// an always-embedded module would be glob-imported into every module of
/// every program and take the name away from user code permanently.
///
/// Asserts the diagnostic message NAMES `assert`, not just that the compile
/// failed with code `E0001` — a bare code check would pass against an
/// `E0001` raised for any unrelated reason (e.g. a typo the fixture didn't
/// intend to introduce).
#[test]
fn assert_is_not_available_outside_nova_test() {
    let dir = std::env::temp_dir().join("nova-test-assert-not-available");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("main.nova");
    std::fs::write(&path, "fn main() { assert(true, \"x\") }\n").expect("write");

    let assert = nova().arg("run").arg(&path).assert().failure();
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr).to_string();
    assert!(stderr.contains("E0001"), "stderr: {stderr}");
    assert!(
        stderr.contains("cannot find function `assert` in this scope"),
        "stderr: {stderr}"
    );
}

/// `assert(cond, msg)` panics with exactly `msg` when `cond` is false, and is
/// silent when `cond` is true. Both cases are needed: a passing-only test
/// would not catch `assert`'s condition being inverted (`if cond` instead of
/// `if !cond`) — under that mutation `assert(true, ..)` panics and
/// `assert(false, ..)` does not, and neither case alone tells the two apart
/// from the correct behavior in a way an all-passing suite would notice.
#[test]
fn assert_panics_with_its_message_and_is_silent_when_true() {
    let dir = std::env::temp_dir().join("nova-test-assert-basic");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let file = dir.join("basic.nova");
    std::fs::write(
        &file,
        "@test\nfn assert_pass() { assert(true, \"boom\") }\n\
         @test\nfn assert_fail() { assert(false, \"boom\") }\n",
    )
    .expect("write");

    let (exe, tests) =
        nova_driver::build_test_binary(&file).expect("test binary compiles and links");
    assert_eq!(
        tests.iter().map(|t| t.name.as_str()).collect::<Vec<_>>(),
        vec!["assert_pass", "assert_fail"]
    );

    let pass = Command::new(&exe)
        .env("NOVA_TEST_INDEX", "0")
        .assert()
        .success();
    let out = String::from_utf8_lossy(&pass.get_output().stdout).to_string();
    assert_eq!(out, "", "assert(true, ..) must print nothing");

    let fail = Command::new(&exe)
        .env("NOVA_TEST_INDEX", "1")
        .assert()
        .failure();
    let stderr = String::from_utf8_lossy(&fail.get_output().stderr).replace("\r\n", "\n");
    assert_eq!(
        stderr.trim_end(),
        "nova: panic: assertion failed: boom",
        "stderr: {stderr}"
    );
    let _ = std::fs::remove_file(&exe);
}

/// `assert_ne(a, b)` panics with a message naming both (rendered with `==`,
/// since they are equal at the point it fires) when `a == b`, and is silent
/// when they differ. Both cases matter for the same reason `assert`'s do: a
/// swapped condition (`if a.eq(b)` written as `if a.ne(b)`, plausible as a
/// copy-paste from `assert_eq`'s body) would still compile and would still
/// pass an all-passing-inputs test.
#[test]
fn assert_ne_panics_when_equal_and_is_silent_when_different() {
    let dir = std::env::temp_dir().join("nova-test-assert-ne-basic");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let file = dir.join("basic.nova");
    std::fs::write(
        &file,
        "@test\nfn ne_pass() { assert_ne(1, 2) }\n\
         @test\nfn ne_fail() { assert_ne(1, 1) }\n",
    )
    .expect("write");

    let (exe, tests) =
        nova_driver::build_test_binary(&file).expect("test binary compiles and links");
    assert_eq!(
        tests.iter().map(|t| t.name.as_str()).collect::<Vec<_>>(),
        vec!["ne_pass", "ne_fail"]
    );

    let pass = Command::new(&exe)
        .env("NOVA_TEST_INDEX", "0")
        .assert()
        .success();
    let out = String::from_utf8_lossy(&pass.get_output().stdout).to_string();
    assert_eq!(out, "", "assert_ne(1, 2) must print nothing");

    let fail = Command::new(&exe)
        .env("NOVA_TEST_INDEX", "1")
        .assert()
        .failure();
    let stderr = String::from_utf8_lossy(&fail.get_output().stderr).replace("\r\n", "\n");
    assert_eq!(
        stderr.trim_end(),
        "nova: panic: assertion failed: 1 == 1",
        "stderr: {stderr}"
    );
    let _ = std::fs::remove_file(&exe);
}

// === nova test: the `nova test [filter]` CLI subcommand (Task 5) ===
//
// Unlike `run`/`build`/`check`, `nova test` takes no `[file]` argument
// (`nova-spec/40-TOOLING.md:20`: `nova test [filter]`) — it always compiles
// `src/main.nova` relative to the current directory, so every test below
// writes its fixture there and points `nova()` at that directory with
// `.current_dir`.

/// A project directory with `src/main.nova` containing `source`, under a
/// fresh, uniquely-named temp directory so parallel tests (and parallel runs
/// of this same suite) never share one `nova test` project.
fn write_test_project(unique_name: &str, source: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(unique_name);
    std::fs::create_dir_all(dir.join("src")).expect("temp project dir");
    std::fs::write(dir.join("src/main.nova"), source).expect("write fixture");
    dir
}

/// A passing test and a failing one, reported distinctly in one run: the
/// pass line says "ok", the failure line says "FAILED" and is followed by
/// the exact panic message, and the summary counts each into its own
/// bucket. Asserts the **entire** rendered stdout, not a substring of it —
/// this project's own retrospective (`2026-08-05-nova-test-design.md` §1)
/// found seven cases of a diagnostic checked by a single fragment that
/// would have matched a completely wrong message just as well.
#[test]
fn nova_test_reports_a_pass_and_a_failure_distinctly() {
    let dir = write_test_project(
        "nova-test-cli-pass-fail",
        "@test\nfn addition_works() { assert_eq(1 + 1, 2) }\n\
         @test\nfn addition_is_broken() { assert_eq(1 + 1, 3) }\n",
    );

    let assert = nova().current_dir(&dir).arg("test").assert().failure();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout).replace("\r\n", "\n");
    assert_eq!(
        stdout,
        "running 2 tests\n\
         test addition_works ... ok\n\
         test addition_is_broken ... FAILED\n\
         \x20\x20\x20\x20nova: panic: assertion failed: 2 != 3\n\
         \n\
         test result: FAILED. 1 passed; 1 failed; 0 trapped; 2 total\n"
    );
}

/// The `(code, hex)` pair `nova test`'s own report will show in its
/// `TRAPPED (exit code ...)` line, measured from a direct run of the same
/// compiled test binary. Shared by
/// `a_hard_trap_is_reported_as_a_trap_and_does_not_satisfy_should_panic` and
/// `a_trapping_tests_captured_output_is_shown_not_discarded` — both trap the
/// process the same way (a division by zero) and both need this pair to
/// build their expected string, so a divergence between them would only
/// mean one had bit-rotted.
///
/// On Windows a fault arrives as a concrete NTSTATUS, so `ExitStatus::code()`
/// is always `Some` and is returned verbatim — a real measurement, exactly
/// as both tests' own doc comments already describe (design doc §9 risk 4:
/// not portable, so measured rather than hard-coded).
///
/// On Unix, POSIX makes "exited normally" and "killed by a signal" mutually
/// exclusive, and a hard trap (a CPU-raised fault — SIGFPE, SIGILL, ...) is
/// the latter, so `.code()` is unconditionally `None` here: there is no code
/// to measure. `classify` (`crates/nova-cli/src/cmd/test.rs:112`) already
/// handles exactly this via `output.status.code().unwrap_or(-1)`, so `-1`
/// (`0xFFFFFFFF`) asserted below deliberately duplicates that production
/// constant rather than measuring anything — asserted because that is
/// genuinely what `nova test` will print, not because anything was
/// measured. That duplication still catches real regressions: `classify`
/// changing its fallback value, or the hex form being misrendered. What it
/// can no longer do is cross-check against a real reported status, because
/// a signal-terminated process on Unix never hands one back — this half is
/// strictly weaker than the Windows measurement above, not an equivalent
/// to it.
///
/// What IS asserted, and is why this does not degrade into a tautology that
/// passes for any nonzero exit: `.signal().is_some()` demands the process
/// was actually killed by a fatal signal. If a future change made this
/// program exit gracefully with some nonzero code instead of trapping,
/// `code()` would be `Some` and `signal()` `None`, and this assertion would
/// fail here — before either caller ever reaches its own comparison against
/// `nova test`'s reported output.
fn expect_trap_exit_code(status: &std::process::ExitStatus) -> (i32, u32) {
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        assert!(
            status.signal().is_some(),
            "a hard trap must be killed by a fatal signal on Unix, not merely exit nonzero: {status:?}"
        );
        let code = -1i32;
        (code, code as u32)
    }
    #[cfg(windows)]
    {
        let code = status
            .code()
            .expect("a concrete exit code on this platform");
        (code, code as u32)
    }
}

/// The load-bearing test on this branch (design doc §5, §9 risk 4; brief
/// step 3). A division by zero exits nonzero with **no** `nova: panic:` line
/// on stderr — a hard trap, not a checked panic — measured directly below
/// rather than assumed. `@test(should_panic)` must NOT be satisfied by it:
/// the report must call it "TRAPPED", never "ok" or plain "FAILED", and the
/// summary must count it under `trapped`, not `passed` or `failed`.
///
/// Treating any nonzero exit as "panicked as expected" would let a
/// miscompile masquerade as a passing test. That is not hypothetical on this
/// project: `nova-typeck`'s
/// `a_next_returning_a_three_variant_sum_is_not_an_iterator` test documents a
/// real guard whose removal turned a clean type-check into exactly this
/// exit-132-on-legal-source shape.
///
/// The expected exit code is **measured directly on Windows**, not
/// hard-coded: the code an aborted process reports is platform- and
/// shell-dependent (design doc §9 risk 4 measured 127 and 132 for two
/// different aborts through Git Bash on this same project), so pinning a
/// literal number here would test one shell's translation of the exit code
/// rather than what `nova test` itself observed through Rust's own process
/// API. Measured once, independently, while building this test: the raw
/// code Windows reports for this exact program is `-1073741795`
/// (`0xC000001D`, `STATUS_ILLEGAL_INSTRUCTION` — Cranelift lowers a checked
/// `sdiv` with a `ud2`-style trap rather than letting the hardware `#DE`
/// fault propagate, which is also consistent with Git Bash's "Illegal
/// instruction" wording for the same program) — a different value from both
/// this project's own 127/132 Git Bash figures and this task's 0xC0000409 /
/// 0xC0000005 figures, because none of the three measured the same thing:
/// different program, different platform-vs-shell layer. That mismatch is
/// why Windows measures its own expectation rather than hard-coding one of
/// those unrelated figures.
///
/// Unix has no figure to measure in the first place: a signal-terminated
/// process reports no exit code at all. Below asserts a fatal signal was
/// delivered instead (confirming this really is a hard trap, not a graceful
/// nonzero exit) and pins the same `-1` (`0xFFFFFFFF`) `classify`
/// (`crates/nova-cli/src/cmd/test.rs:112`) itself falls back to via
/// `code().unwrap_or(-1)` — a deliberate duplication of that production
/// constant, not a second measurement, and correspondingly weaker: it
/// catches `classify`'s fallback changing or the hex form being
/// misrendered, but it cannot cross-check against a real reported status
/// the way the Windows half can, because a signal-terminated process on
/// Unix never hands one back. See `expect_trap_exit_code`'s own doc
/// comment for the full mechanism.
#[test]
fn a_hard_trap_is_reported_as_a_trap_and_does_not_satisfy_should_panic() {
    let dir = write_test_project(
        "nova-test-cli-hard-trap",
        "@test(should_panic)\nfn divides_by_zero() { let _ = 1 / 0 }\n",
    );

    // On Windows, measure the real exit code directly, the same way `nova
    // test` itself will see it, rather than assuming a portable number. On
    // Unix there is no code to measure, so `expect_trap_exit_code` asserts a
    // fatal signal instead and pins `classify`'s own fallback constant --
    // see its doc comment (and this test's, above) for why that is weaker
    // than a measurement but still a real check, not a tautology.
    let (exe, tests) = nova_driver::build_test_binary(&dir.join("src/main.nova"))
        .expect("test binary compiles and links");
    assert_eq!(
        tests.iter().map(|t| t.name.as_str()).collect::<Vec<_>>(),
        vec!["divides_by_zero"]
    );
    let direct = Command::new(&exe)
        .env("NOVA_TEST_INDEX", "0")
        .output()
        .expect("run the trap directly");
    assert!(
        !direct.status.success(),
        "a division by zero must exit nonzero"
    );
    let direct_stderr = String::from_utf8_lossy(&direct.stderr);
    assert!(
        !direct_stderr.contains("nova: panic:"),
        "must be a hard trap with no panic message, not a checked panic: {direct_stderr}"
    );
    let (code, hex) = expect_trap_exit_code(&direct.status);
    let _ = std::fs::remove_file(&exe);

    let assert = nova().current_dir(&dir).arg("test").assert().failure();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout).replace("\r\n", "\n");
    assert_eq!(
        stdout,
        format!(
            "running 1 test\n\
             test divides_by_zero ... TRAPPED (exit code {code} (0x{hex:08X}))\n\
             \n\
             test result: FAILED. 0 passed; 0 failed; 1 trapped; 1 total\n"
        )
    );
}

/// `@test(should_panic)` passes when the panic is a *checked* one — array
/// out of bounds, which panics via `nova_rt_check_bounds`
/// (`nova-runtime/src/lib.rs`) — reported as an ordinary "ok", not called out
/// as anything special. Pairs with the hard-trap test above: together they
/// are the two halves of "should_panic passes on the middle row only, and
/// only that row" (design doc §5).
#[test]
fn should_panic_passes_on_a_checked_panic() {
    let dir = write_test_project(
        "nova-test-cli-should-panic-checked",
        "@test(should_panic)\nfn out_of_bounds_panics() { let xs = [1, 2, 3]\n let _ = xs[7] }\n",
    );

    let assert = nova().current_dir(&dir).arg("test").assert().success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout).replace("\r\n", "\n");
    assert_eq!(
        stdout,
        "running 1 test\n\
         test out_of_bounds_panics ... ok\n\
         \n\
         test result: ok. 1 passed; 0 failed; 0 trapped; 1 total\n"
    );
}

/// Not one of the four required tests, but the same "should_panic passes on
/// the middle row only" rule (design doc §5) has a second edge this project's
/// own gate list never separately exercises: a `should_panic` test that
/// simply completes without panicking. `should_panic` inverting *only* the
/// `Panicked` row (brief step 2) does not mean the `Passed` row is exempt
/// from `should_panic`'s requirement — "passes on the middle row only" reads
/// both ways. Left unhandled, a bug fix that accidentally removed the code
/// path a `should_panic` test exists to guard would make that test keep
/// "passing" forever, which is precisely the silent-regression failure mode
/// `@test(should_panic)` exists to prevent.
#[test]
fn should_panic_fails_distinctly_when_the_test_does_not_panic() {
    let dir = write_test_project(
        "nova-test-cli-should-panic-not-panicking",
        "@test(should_panic)\nfn does_not_actually_panic() { assert_eq(1, 1) }\n",
    );

    let assert = nova().current_dir(&dir).arg("test").assert().failure();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout).replace("\r\n", "\n");
    assert_eq!(
        stdout,
        "running 1 test\n\
         test does_not_actually_panic ... FAILED (expected a panic, but the test passed)\n\
         \n\
         test result: FAILED. 0 passed; 1 failed; 0 trapped; 1 total\n"
    );
}

// === `2026-08-08-joinhandle-task-identity`: `JoinHandle` keyed on its future,
// not a forgeable id ===
//
// The forged-handle hang itself is `forged_join_handle_aborts_instead_of_hanging`
// above (a `nova run` gate, since the whole point is that it must *complete*).
// The two cases below are its `@test(should_panic)` siblings -- observable
// this way because each test runs in its own process (`cmd/test.rs`'s design),
// so one `abort_with` cannot take any other test down with it -- plus the
// false-positive guard neither would catch on its own: an over-broad
// duplicate-spawn rejection (keying on mere `BY_STATE` presence rather than
// `Task::taken` liveness) would make ordinary double-spawning abort too, and
// nothing above would say so.

/// `join` on a handle built directly from a future that was never `spawn`ed
/// aborts, and spawning one future value twice aborts -- the two ways
/// `nova_rt_task_is_done`/`nova_rt_task_release`/`nova_rt_task_spawn` reject a
/// future they cannot (or must not) resolve. Both land on `abort_with`, which
/// `eprintln!`s the `nova: panic:` marker `cmd/test.rs`'s `classify` looks
/// for, so a `should_panic` test is satisfied by either -- the same
/// classification `should_panic_passes_on_a_checked_panic` exercises via
/// `nova_rt_check_bounds` instead.
#[test]
fn join_handle_rejects_a_never_spawned_future_and_a_live_duplicate() {
    let dir = write_test_project(
        "nova-test-join-handle-forgery-and-duplicate",
        "async fn spin() -> Int { 1 }\n\
         \n\
         @test(should_panic)\n\
         fn joining_a_never_spawned_handle_aborts() {\n\
         \x20\x20let h = JoinHandle { fut: spin() }\n\
         \x20\x20let _ = block_on(h.join())\n\
         }\n\
         \n\
         @test(should_panic)\n\
         fn spawning_the_same_future_twice_aborts() {\n\
         \x20\x20let f = spin()\n\
         \x20\x20spawn(f)\n\
         \x20\x20let _ = spawn(f)\n\
         }\n",
    );

    let assert = nova().current_dir(&dir).arg("test").assert().success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout).replace("\r\n", "\n");
    assert_eq!(
        stdout,
        "running 2 tests\n\
         test joining_a_never_spawned_handle_aborts ... ok\n\
         test spawning_the_same_future_twice_aborts ... ok\n\
         \n\
         test result: ok. 2 passed; 0 failed; 0 trapped; 2 total\n"
    );
}

/// The false-positive guard the design's own argument needs and the two
/// `should_panic` tests above cannot provide: two distinct calls to `spin()`
/// are two distinct futures (each allocates its own state object), so
/// spawning both must succeed and run to completion, not merely "not abort".
/// Without this, an over-broad duplicate-spawn rejection would have nothing
/// in this suite to catch it: neither `should_panic` test above spawns two
/// live, legitimately distinct futures, so neither would notice a rejection
/// that fired on that shape too.
#[test]
fn spawning_two_distinct_futures_from_the_same_call_site_still_works() {
    let dir = write_test_project(
        "nova-test-join-handle-double-spawn-guard",
        "async fn spin() -> Int { 1 }\n\
         async fn run_two_spawns() -> Int {\n\
         \x20\x20let a = spawn(spin())\n\
         \x20\x20let b = spawn(spin())\n\
         \x20\x20let ra = a.join().await\n\
         \x20\x20let rb = b.join().await\n\
         \x20\x20ra + rb\n\
         }\n\
         \n\
         @test\n\
         fn spawning_two_distinct_futures_still_works() {\n\
         \x20\x20assert_eq(block_on(run_two_spawns()), 2)\n\
         }\n",
    );

    let assert = nova().current_dir(&dir).arg("test").assert().success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout).replace("\r\n", "\n");
    assert_eq!(
        stdout,
        "running 1 test\n\
         test spawning_two_distinct_futures_still_works ... ok\n\
         \n\
         test result: ok. 1 passed; 0 failed; 0 trapped; 1 total\n"
    );
}

/// `join` is specified to be idempotent (`std/task/lib.nova`'s own doc
/// comment on `JoinHandle::join`): `task_release` is a no-op on its second
/// call, and the value is read back out of the state object's output slot
/// rather than taken, so it survives being read twice. Keying identity on
/// the future rather than an id does not disturb this -- both calls resolve
/// through the same `self.fut`, so there is only ever one task to ask.
#[test]
fn joining_a_handle_twice_returns_the_same_value_twice_with_no_abort() {
    let dir = write_test_project(
        "nova-test-join-handle-join-twice",
        "async fn spin() -> Int { 1 }\n\
         async fn run_join_twice() -> Int {\n\
         \x20\x20let h = spawn(spin())\n\
         \x20\x20let a = h.join().await\n\
         \x20\x20let b = h.join().await\n\
         \x20\x20a + b\n\
         }\n\
         \n\
         @test\n\
         fn joining_a_handle_twice_returns_the_same_value_twice() {\n\
         \x20\x20assert_eq(block_on(run_join_twice()), 2)\n\
         }\n",
    );

    let assert = nova().current_dir(&dir).arg("test").assert().success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout).replace("\r\n", "\n");
    assert_eq!(
        stdout,
        "running 1 test\n\
         test joining_a_handle_twice_returns_the_same_value_twice ... ok\n\
         \n\
         test result: ok. 1 passed; 0 failed; 0 trapped; 1 total\n"
    );
}

/// `nova test <filter>` selects a strict substring-matched subset
/// (`nova-spec/40-TOOLING.md:20`) and — this is the part a weaker test would
/// miss — the unselected test genuinely does not run, not merely go
/// unreported.
///
/// A filter bug that runs every test and only *prints* the selected ones
/// would still pass a test that checks nothing but the two "kept" lines
/// (task brief, step 1). To catch that shape too, `skip_c` (excluded by the
/// filter `"keep"`) spends real wall-clock time if — and only if — it
/// actually runs: a 40-billion-iteration loop, calibrated by direct
/// measurement while writing this test at roughly 1.6 billion iterations per
/// second on this machine (2 billion iterations measured at 1.257s via a
/// throwaway `nova build` + run), so it costs about 25–30 seconds if
/// executed and costs nothing at all if it is correctly skipped. The
/// filtered run below is asserted to finish in well under that — measured at
/// ~0.25s for the correctly-filtered path during development, so the 15
/// second bound leaves a wide, non-flaky margin without ever hanging: the
/// loop is large but finite, so even a buggy runner that does execute it
/// completes (slowly) rather than never returning. `assert_eq(i, ...)` at
/// the end (rather than discarding `i`) exists so the fast Cranelift
/// backend cannot prove the loop dead and remove it.
#[test]
fn a_filter_selects_a_strict_subset_and_the_others_do_not_run() {
    let dir = write_test_project(
        "nova-test-cli-filter",
        "@test\nfn keep_a() { assert_eq(1, 1) }\n\
         @test\nfn keep_b() { assert_eq(2, 2) }\n\
         @test\nfn skip_c() {\n\
         \x20\x20\x20\x20let mut i = 0\n\
         \x20\x20\x20\x20while i < 40000000000 { i = i + 1 }\n\
         \x20\x20\x20\x20assert_eq(i, 40000000000)\n\
         }\n",
    );

    let start = std::time::Instant::now();
    let assert = nova()
        .current_dir(&dir)
        .arg("test")
        .arg("keep")
        .assert()
        .success();
    let elapsed = start.elapsed();
    assert!(
        elapsed < std::time::Duration::from_secs(15),
        "the excluded test's ~25s loop must not have run; took {elapsed:?}"
    );
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout).replace("\r\n", "\n");
    assert_eq!(
        stdout,
        "running 2 tests\n\
         test keep_a ... ok\n\
         test keep_b ... ok\n\
         \n\
         test result: ok. 2 passed; 0 failed; 0 trapped; 2 total\n"
    );
}

// === Fix round 1 (review): two "correct but unpinned" gaps ===

/// `should_panic` must be looked up **by the same index the test itself just
/// ran at**, not always at position 0. Every should_panic fixture above has
/// exactly one `@test` function, always at index 0 — `tests[i].should_panic`
/// and the bug `tests[0].should_panic` are indistinguishable at index 0, so
/// none of them could have caught a mis-association. This fixture puts
/// `should_panic` in the middle, at index 1, flanked by two ordinary
/// (`should_panic = false`) tests, so three different plausible
/// index-mistakes are all independently visible:
///
/// - a constant `tests[0]`: `beta_panics_checked` (index 1) would be
///   evaluated against index 0's `false`, turning its "ok" into "FAILED" —
///   a graceful wrong-verdict content mismatch. Measured: `nova test`
///   completes normally and prints a full, well-formed report with exactly
///   this one line wrong.
/// - `tests[i - 1]`: **not** a graceful content mismatch — measured to be a
///   genuine Rust panic that crashes the `nova` process outright, before
///   evaluating anything. `i - 1` is computed on a `usize`, and the very
///   first iteration is `i = 0` (`alpha_ok`), so it underflows immediately:
///   `thread 'main' panicked at ...: attempt to subtract with overflow`,
///   exit code 101. Measured stdout is only the `running 3 tests` header —
///   `beta_panics_checked` and `gamma_ok` never get a chance to print
///   anything, let alone the wrong thing.
/// - `tests[i + 1]`: also a genuine Rust panic, measured one iteration
///   later. `alpha_ok` (index 0) is evaluated against index 1's `true`
///   ("FAILED (expected a panic, but the test passed)") and
///   `beta_panics_checked` (index 1) against index 2's `false` ("FAILED"
///   with its own bounds-panic message) — both print exactly as a
///   content-mismatch framing would predict — but then `gamma_ok` (index 2)
///   reads `tests[3]`, past the end of this 3-element array, and the `nova`
///   process panics with `index out of bounds: the len is 3 but the index
///   is 3` before any `test result:` summary line prints.
///
/// So this single fixture's exact-output assertion fails under any of the
/// three shapes — the first because the content is wrong, the other two
/// because the `nova` process crashes and never reaches a `test result:`
/// line at all — not just the literal one the review reproduced.
#[test]
fn should_panic_is_matched_to_its_own_test_not_to_index_zero() {
    let dir = write_test_project(
        "nova-test-cli-should-panic-by-index",
        "@test\nfn alpha_ok() { assert_eq(1, 1) }\n\
         @test(should_panic)\nfn beta_panics_checked() { let xs = [1, 2, 3]\n let _ = xs[9] }\n\
         @test\nfn gamma_ok() { assert_eq(2, 2) }\n",
    );

    let assert = nova().current_dir(&dir).arg("test").assert().success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout).replace("\r\n", "\n");
    assert_eq!(
        stdout,
        "running 3 tests\n\
         test alpha_ok ... ok\n\
         test beta_panics_checked ... ok\n\
         test gamma_ok ... ok\n\
         \n\
         test result: ok. 3 passed; 0 failed; 0 trapped; 3 total\n"
    );
}

/// The filter matches a substring **anywhere** in the name, not only as a
/// prefix. `a_filter_selects_a_strict_subset_and_the_others_do_not_run`'s
/// filter `"keep"` happens to be a literal prefix of both names it selects,
/// so mutating `.contains` to `.starts_with` there still passes. Here
/// neither matching name *starts with* `"alpha"` — it sits in the middle of
/// one and at the end of the other — so `.starts_with("alpha")` would select
/// neither, shrinking `running 2 tests` to `running 0 tests` with no per-test
/// lines at all: an unmistakable difference from the exact output asserted
/// below.
#[test]
fn a_filter_matches_a_substring_anywhere_not_only_as_a_prefix() {
    let dir = write_test_project(
        "nova-test-cli-filter-substring-not-prefix",
        "@test\nfn zzz_alpha_zzz() { assert_eq(1, 1) }\n\
         @test\nfn keep_alpha() { assert_eq(2, 2) }\n\
         @test\nfn unrelated_beta() { assert_eq(3, 3) }\n",
    );

    let assert = nova()
        .current_dir(&dir)
        .arg("test")
        .arg("alpha")
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout).replace("\r\n", "\n");
    assert_eq!(
        stdout,
        "running 2 tests\n\
         test zzz_alpha_zzz ... ok\n\
         test keep_alpha ... ok\n\
         \n\
         test result: ok. 2 passed; 0 failed; 0 trapped; 2 total\n"
    );
}

// === Task 6: `nova test` end-to-end gate ===
//
// `tests/runtime/nova_test.{nova,stdout}`, run three ways like every other
// `tests/runtime` fixture (`nova_test_run`, `nova_test_build_standalone`,
// `nova_test_under_gc_stress` — the registrations that take this workspace's
// gate-configuration count from 15 to 18). Unlike `iterator`/`assoc_types`/
// `strings`/`collections`/`std_core`, there is no JIT-vs-object-backend split
// available to give the trio's middle member a distinct meaning:
// `nova_driver::build_test_binary` always compiles through the object
// backend (the same one `nova build` uses; see its doc comment), so
// `nova run`'s JIT path is never exercised by `nova test` at all.
// `nova_test_build_standalone` therefore tests the compiled binary directly —
// bypassing the `nova` CLI subprocess for the compile-and-orchestrate step,
// the same way `a_hard_trap_is_reported_as_a_trap_and_does_not_satisfy_
// should_panic` above does for a single test, generalized here to all four.
//
// Two of the task brief's seven gate items are deliberately NOT inside
// `nova_test.nova` itself:
//   - A filtered run over this exact fixture is `nova_test_filter_run`
//     below — a filter selects a *subset* of the file, so it is not a fourth
//     way to run the whole thing, and Task 5's own tests already pin the
//     filter mechanism (substring, anywhere, not a prefix check); this test's
//     job is only to confirm filtering composes with the real fixture.
//   - An unknown attribute rejected as `E0082` is
//     `nova_test_reports_e0082_for_an_unknown_attribute` below, against its
//     own single-line fixture. It cannot live in `nova_test.nova`: `E0082` is
//     a compile error, and that file must compile cleanly for its other four
//     tests to run at all.

/// Copies the checked-in gate fixture's source into a temp project's
/// `src/main.nova`, since `nova test`'s only positional argument is a filter
/// (`nova-spec/40-TOOLING.md:20`: `nova test [filter]`, no `[file]`, unlike
/// `run`/`build`/`check`) — the entry file is always `src/main.nova` relative
/// to the current directory (`cmd/test.rs`'s `test_entry_file`). `unique_name`
/// keeps concurrent `cargo test` threads from colliding on one directory,
/// same convention as `write_test_project`.
fn write_gate_fixture_project(unique_name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(unique_name);
    std::fs::create_dir_all(dir.join("src")).expect("temp project dir");
    let source = std::fs::read_to_string(repo_root().join("tests/runtime/nova_test.nova"))
        .expect("gate fixture exists");
    std::fs::write(dir.join("src/main.nova"), source).expect("write fixture");
    dir
}

fn read_gate_expected() -> String {
    std::fs::read_to_string(repo_root().join("tests/runtime/nova_test.stdout"))
        .expect("expected-output fixture exists")
        .replace("\r\n", "\n")
}

/// Replace the two platform-specific numbers in a `TRAPPED (exit code N (0xH))`
/// line with fixed placeholders, leaving every other line byte-identical.
///
/// The gate fixture (`tests/runtime/nova_test.stdout`) asserts the runner's
/// full literal stdout, which is what makes it able to catch a collapsed
/// panicked/trapped distinction. But one token in it cannot be portable:
/// Windows translates a hardware fault into an NTSTATUS exit code
/// (`0xC000001D`, `STATUS_ILLEGAL_INSTRUCTION`), while on Linux and macOS the
/// same program dies from `SIGFPE` with no exit code at all, which
/// `classify` (`cmd/test.rs:112`'s `unwrap_or(-1)`) renders as `-1`. CI's
/// matrix (`.github/workflows/ci.yml`) runs ubuntu, windows and macos, so a
/// fixture pinning either raw value cannot hold on the other two platforms.
///
/// Only the numbers are elided; the surrounding structure — the test name,
/// the word `TRAPPED`, and the presence of both the decimal and the
/// parenthesized hex — is preserved, so a change in the report's *shape*
/// still fails the comparison. This is the same rule this file already
/// follows for its two trap tests,
/// `a_hard_trap_is_reported_as_a_trap_and_does_not_satisfy_should_panic` and
/// `a_trapping_tests_captured_output_is_shown_not_discarded`: never write a
/// literal exit code into an expectation, because the code an aborted
/// process reports is platform-dependent. Those two tests get their
/// `(code, hex)` pair from `expect_trap_exit_code` — a real measurement on
/// Windows, but on Unix (where a signal-terminated process reports no code
/// at all) a deliberate pin of `classify`'s own documented `-1`
/// (`0xFFFFFFFF`) fallback rather than a second measurement (see that
/// helper's doc comment). Either way the value is platform-appropriate and
/// asserted rather than written as a literal here, which is exactly why
/// normalizing this fixture loses no coverage — the real, or on Unix the
/// correctly-predicted, value is still pinned, just not in this file.
///
/// Splits on plain `'\n'` rather than [`str::lines`]: `lines()` also
/// swallows a trailing line terminator (`"a\n".lines().collect::<Vec<_>>()`
/// is just `["a"]`, indistinguishable from splitting `"a"` with no newline
/// at all), so `s.lines().collect::<Vec<_>>().join("\n")` silently drops a
/// trailing newline the input had. Measured directly by temporarily making
/// that swap: the test below's very first `assert_eq!` then fails with
/// `left: "...<HEX>))"` vs. `right: "...<HEX>))\n"` — the trailing newline
/// gone from the normalized side. `split`/`join` on the same literal
/// delimiter are exact inverses of one another (unlike `lines`/`join`), so
/// every byte that isn't inside a rewritten line — including a trailing
/// newline, or the lack of one — survives untouched. Every actual call site
/// (below, and in `nova_test_run`, `nova_test_build_standalone`,
/// `nova_test_under_gc_stress`) already has `"\r\n"` collapsed to `"\n"`
/// before calling this function: the unit test passes only `\n`-terminated
/// literals, and each of the three gate tests calls
/// `.replace("\r\n", "\n")` on the text it passes in — captured stdout
/// for `nova_test_run` and `nova_test_under_gc_stress`, but
/// `nova_test_build_standalone`'s own synthesized report string (built
/// from each subprocess's `result.stderr` and `result.status`; that
/// function never reads `result.stdout`) for the third. (The
/// fixture's own text, loaded by `read_gate_expected`, is independently
/// CRLF-normalized when read from disk — but it is never itself passed to
/// this function; it is only the literal right-hand side `assert_eq!`
/// compares this function's output against.) So plain `'\n'` splitting is
/// exact here; no `'\r'` ever reaches this function.
fn normalize_trap_codes(s: &str) -> String {
    const MARKER: &str = "TRAPPED (exit code ";
    s.split('\n')
        .map(|line| match line.find(MARKER) {
            // Rewrite only from the marker to the end of the parenthesized
            // payload, so anything before `TRAPPED` (the test name) is kept.
            // The `ends_with("))")` guard is what keeps a decoy line — one
            // that merely mentions "exit code", or a TRAPPED line missing
            // its hex form — from being rewritten. Verified directly against
            // `tests/runtime/nova_test.stdout` (post `\r\n` -> `\n`
            // normalization, as every caller applies): its one TRAPPED line
            // is the only line in that fixture ending in `"))"`.
            Some(i) if line.ends_with("))") => {
                format!("{}{}<CODE> (<HEX>))", &line[..i], MARKER)
            }
            _ => line.to_string(),
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn normalize_trap_codes_elides_only_the_platform_specific_numbers() {
    // The gate fixture cannot hold a real exit code: Windows reports an
    // NTSTATUS (-1073741795 / 0xC000001D for an illegal instruction) while a
    // signal-killed process on Linux/macOS has no code at all and renders as
    // -1. Both must normalize to the same text, or the fixture can only ever
    // match one platform.
    let win = "test t ... TRAPPED (exit code -1073741795 (0xC000001D))\n";
    let unix = "test t ... TRAPPED (exit code -1 (0xFFFFFFFF))\n";
    assert_eq!(normalize_trap_codes(win), normalize_trap_codes(unix));
    assert_eq!(
        normalize_trap_codes(win),
        "test t ... TRAPPED (exit code <CODE> (<HEX>))\n"
    );

    // STRUCTURE MUST SURVIVE. If the runner ever stopped printing the hex
    // form, or dropped the "TRAPPED" word, or lost the test name, the
    // normalized text must change so the fixture comparison fails. A
    // normalizer that collapsed the whole line to a constant would hide
    // exactly the regressions this fixture exists to catch.
    assert_ne!(
        normalize_trap_codes(win),
        normalize_trap_codes("test t ... TRAPPED (exit code -1073741795)\n")
    );
    assert_ne!(
        normalize_trap_codes(win),
        normalize_trap_codes("test OTHER ... TRAPPED (exit code -1073741795 (0xC000001D))\n")
    );
    assert_ne!(
        normalize_trap_codes(win),
        normalize_trap_codes("test t ... FAILED (exit code -1073741795 (0xC000001D))\n")
    );

    // Lines with no trap must pass through byte-identically -- the fixture's
    // other five lines (the header, two `ok`s, a FAILED with its panic
    // message, and the summary) are fully portable and must stay asserted
    // exactly.
    let untouched = "running 4 tests\ntest a ... ok\n\ntest result: FAILED. 2 passed; 1 failed; 1 trapped; 4 total\n";
    assert_eq!(normalize_trap_codes(untouched), untouched);

    // A panic message that merely mentions the words must not be rewritten.
    let decoy = "    nova: panic: exit code -5 is wrong\n";
    assert_eq!(normalize_trap_codes(decoy), decoy);
}

/// Through the full `nova` CLI, exactly as a user would invoke it.
/// `.failure()` is correct, not a mistake: the fixture deliberately contains
/// a failing assertion and a hard trap (see its own header comment), so
/// `nova test` itself must exit nonzero even though two of its four tests
/// pass. The full stdout is asserted — not a pass/fail count and not a
/// substring — because a runner that collapsed the panicked/trapped
/// distinction would still exit nonzero here (the unrelated failing
/// `assert_eq` guarantees that), so only the exact per-test lines catch the
/// regression this fixture exists to protect against (task brief, item 4).
/// Captured and run through `normalize_trap_codes` rather than asserted with
/// `assert_cmd`'s `.stdout(expected)` predicate, which cannot normalize:
/// the trapped test's exit code is platform-specific (see
/// `normalize_trap_codes`'s doc comment), so it is the one part of stdout
/// not compared completely literally.
#[test]
fn nova_test_run() {
    let expected = read_gate_expected();
    let dir = write_gate_fixture_project("nova-test-gate-run");
    let assert = nova().current_dir(&dir).arg("test").assert().failure();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout).replace("\r\n", "\n");
    // stderr is attached below too, alongside stdout, even though only stdout
    // is compared: `.assert()` above has already captured it at no extra
    // cost (no re-run), and an ADR-0008 occurrence on this freshly built
    // binary could in principle leave stdout empty with its only diagnostic
    // on stderr — in which case stdout alone would print nothing useful.
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert_eq!(
        normalize_trap_codes(&stdout),
        expected,
        "gate output mismatch. Raw, un-normalized stdout follows — the exit \
         codes in it are the evidence ADR 0008's open question needs, and \
         normalization would otherwise replace them with placeholders. \
         stderr follows too, in case stdout above is empty:\nstdout:\n{stdout}\n\
         stderr:\n{stderr}"
    );
}

/// The same fixture through `nova_driver::build_test_binary` called
/// directly, running the produced binary itself rather than going through
/// the `nova` CLI's subprocess at all. `crates/nova-cli` has no `[lib]`
/// target (only a `[[bin]]`), so an integration test cannot call
/// `cmd::test::run` directly — calling the driver crate and reconstructing
/// the report is the closest equivalent to `iterator_build_standalone`'s
/// "exercise a different code path than `_run`" intent that is actually
/// available here. Reconstructs the exact report `cmd/test.rs::run` would
/// print (mirroring its `classify` + reporting loop) and compares the
/// captured `String` with `assert_eq!`, rather than asserting on an
/// `assert_cmd::Command` the way `nova_test_run` does — matching the brief's
/// note that "the build variant compares captured stdout rather than
/// asserting on the command." `out` itself is still built from the real,
/// live exit code the child process reported (see the `TRAPPED` arm below)
/// exactly as `cmd/test.rs::run` would print it; only the final `assert_eq!`
/// runs it through `normalize_trap_codes`, for the same cross-platform
/// reason `nova_test_run` does.
#[test]
fn nova_test_build_standalone() {
    let expected = read_gate_expected();
    let dir = write_gate_fixture_project("nova-test-gate-build-standalone");
    let (exe, tests) = nova_driver::build_test_binary(&dir.join("src/main.nova"))
        .expect("test binary compiles and links");

    let mut out = format!(
        "running {} test{}\n",
        tests.len(),
        if tests.len() == 1 { "" } else { "s" }
    );
    let (mut passed, mut failed, mut trapped) = (0usize, 0usize, 0usize);
    // `NOVA_TEST_INDEX` addresses a test by its position in `tests` — source
    // order, which is also dispatch order in the synthesized `main`
    // (`nova_driver::synthesize_test_main`) — so the enumeration index `i`,
    // not `t.def_id`, is what selects it.
    for (i, t) in tests.iter().enumerate() {
        let result = Command::new(&exe)
            .env("NOVA_TEST_INDEX", i.to_string())
            .output()
            .unwrap_or_else(|e| panic!("failed to run {} for `{}`: {e}", exe.display(), t.name));
        let stderr = String::from_utf8_lossy(&result.stderr);
        let panic_line = stderr.lines().find(|line| line.contains("nova: panic:"));
        if result.status.success() {
            if t.should_panic {
                failed += 1;
                out.push_str(&format!(
                    "test {} ... FAILED (expected a panic, but the test passed)\n",
                    t.name
                ));
            } else {
                passed += 1;
                out.push_str(&format!("test {} ... ok\n", t.name));
            }
        } else if let Some(line) = panic_line {
            if t.should_panic {
                passed += 1;
                out.push_str(&format!("test {} ... ok\n", t.name));
            } else {
                failed += 1;
                out.push_str(&format!(
                    "test {} ... FAILED\n    {}\n",
                    t.name,
                    line.trim()
                ));
            }
        } else {
            trapped += 1;
            let code = result.status.code().unwrap_or(-1);
            let hex = code as u32;
            out.push_str(&format!(
                "test {} ... TRAPPED (exit code {code} (0x{hex:08X}))\n",
                t.name
            ));
        }
    }
    out.push('\n');
    let total = tests.len();
    let all_ok = failed == 0 && trapped == 0;
    out.push_str(&format!(
        "test result: {}. {passed} passed; {failed} failed; {trapped} trapped; {total} total\n",
        if all_ok { "ok" } else { "FAILED" }
    ));
    let _ = std::fs::remove_file(&exe);

    // No separate stderr is attached here the way the other two gate tests'
    // failure messages attach one: unlike those two, which run one top-level
    // `nova` subprocess and could in principle see it produce empty stdout
    // with a diagnostic only on stderr, this test's `out` is itself already
    // assembled per-subprocess from each test binary's own `result.stderr`
    // and `result.status` (see the loop above) — a TRAPPED line's real exit
    // code is already inlined into `out` directly from that `result.status`,
    // so `out` alone carries the evidence an ADR-0008 occurrence would leave.
    assert_eq!(
        normalize_trap_codes(&out.replace("\r\n", "\n")),
        expected,
        "gate output mismatch. Raw, un-normalized report follows — the exit \
         codes in it are the evidence ADR 0008's open question needs, and \
         normalization would otherwise replace them with placeholders:\n{out}"
    );
}

/// The same fixture with `NOVA_GC_STRESS=1` (collect on every allocation).
/// Each `@test` runs in its own process (`cmd/test.rs`'s whole design), and
/// `std::process::Command::env` only ever ADDS to an inherited environment
/// rather than replacing it, so setting the variable on the outer
/// `nova test` invocation reaches every process it spawns too — the same
/// propagation Task 4 independently confirmed for this exact mechanism
/// (`std::process::Command::env`, not a proxy for it). Compared through
/// `normalize_trap_codes` the same way `nova_test_run` is, and for the same
/// reason — see its comment.
#[test]
fn nova_test_under_gc_stress() {
    let expected = read_gate_expected();
    let dir = write_gate_fixture_project("nova-test-gate-gc-stress");
    let assert = nova()
        .env("NOVA_GC_STRESS", "1")
        .current_dir(&dir)
        .arg("test")
        .assert()
        .failure();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout).replace("\r\n", "\n");
    // Same reasoning as `nova_test_run`'s comment above its own `stderr`
    // capture: already captured by `.assert()` at no extra cost, and
    // attached below in case an ADR-0008 occurrence leaves stdout empty with
    // its only diagnostic on stderr.
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert_eq!(
        normalize_trap_codes(&stdout),
        expected,
        "gate output mismatch. Raw, un-normalized stdout follows — the exit \
         codes in it are the evidence ADR 0008's open question needs, and \
         normalization would otherwise replace them with placeholders. \
         stderr follows too, in case stdout above is empty:\nstdout:\n{stdout}\n\
         stderr:\n{stderr}"
    );
}

/// Gate item 5 ("a filter run"), composed with the real multi-item fixture
/// rather than a fresh throwaway one. Task 5's own
/// `a_filter_selects_a_strict_subset_and_the_others_do_not_run` and
/// `a_filter_matches_a_substring_anywhere_not_only_as_a_prefix` already pin
/// the filter *mechanism* (substring match, anywhere in the name, proven not
/// to be merely a prefix check); this test's job is narrower — confirm
/// filtering still composes correctly against this specific fixture.
/// `"is_correct"` is a substring of exactly one of the four names, so a
/// selected count other than 1 is immediately visible.
#[test]
fn nova_test_filter_run() {
    let dir = write_gate_fixture_project("nova-test-gate-filter");
    let assert = nova()
        .current_dir(&dir)
        .arg("test")
        .arg("is_correct")
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout).replace("\r\n", "\n");
    assert_eq!(
        stdout,
        "running 1 test\n\
         test addition_is_correct ... ok\n\
         \n\
         test result: ok. 1 passed; 0 failed; 0 trapped; 1 total\n"
    );
}

/// Gate item 6 ("an unknown attribute rejected as `E0082`"), exercised
/// through `nova test` specifically rather than only `nova check` / the
/// resolver's own unit tests (which already cover `E0082` exhaustively —
/// Task 2). It cannot live inside `nova_test.nova`: `E0082` is a compile
/// error, and that file must compile cleanly for its other four tests to run
/// at all. Asserts the diagnostic's actual text, not only its code, matching
/// this branch's standing rule that a bare code check would still pass if
/// the message body were replaced with something unrelated.
#[test]
fn nova_test_reports_e0082_for_an_unknown_attribute() {
    let dir = write_test_project("nova-test-gate-e0082", "@tset\nfn foo() { }\n");
    let assert = nova().current_dir(&dir).arg("test").assert().failure();
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(stderr.contains("E0082"), "stderr: {stderr}");
    assert!(
        stderr.contains("unknown attribute `@tset`; known attributes are: test"),
        "stderr: {stderr}"
    );
}

// === Final whole-branch review, condition 1 (CRITICAL): `@test async fn` must
// be refused, and the supported shape must still work ===
//
// `@test async fn` used to compile and report `ok` while running none of its
// body: the runner's dispatcher calls the collected symbol directly, and for an
// `async fn` that symbol is the future-building wrapper, so the call allocated a
// state object and returned a future nothing polled. A guaranteed-failing
// assertion inside such a test reported green.
//
// The fix refuses the shape (`E0084`) rather than shimming the dispatcher,
// because `@test fn t() { block_on(f()) }` is the shape the `nova test` design
// and Phase 2.3a's own design (§10) both specify for testing async code — the
// defect was acceptance, not a missing feature. The two tests below are the
// halves of that claim and only mean something together: the first that the
// refusal fires with an actionable diagnostic, the second that async code is
// still testable *and* that a wrong answer inside the supported shape really
// does fail. Without the second, the first would be indistinguishable from
// closing off async testing entirely.

/// The refusal fires through `nova test`, with a diagnostic that names the
/// working alternative.
///
/// End to end rather than only in `nova-typeck`
/// (`test_on_an_async_function_is_e0084`) because the dispatcher this protects
/// lives in `nova-driver` and is reached only on this path: a refusal that held
/// in the resolver's own tests but not here would leave the original defect
/// intact for every actual user of `nova test`.
#[test]
fn nova_test_rejects_an_async_test_function() {
    let dir = write_test_project(
        "nova-test-async-rejected",
        "async fn helper() -> Int { 41 }\n\
         @test\nasync fn t() { assert_eq(helper().await, 999) }\n",
    );
    let assert = nova().current_dir(&dir).arg("test").assert().failure();
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(stderr.contains("E0084"), "stderr: {stderr}");
    assert!(
        stderr.contains("a `@test` function may not have an `async` body"),
        "the message must name `async`, not a generic bad signature: {stderr}"
    );
    // The note is the load-bearing half for a user: without it the refusal
    // reads as "async cannot be tested". Both fragments are asserted because
    // naming `block_on` without showing where it goes is not actionable.
    assert!(
        stderr.contains("block_on"),
        "the diagnostic must name the working alternative: {stderr}"
    );
    assert!(
        stderr.contains("@test fn t()"),
        "the diagnostic must spell the replacement out as source: {stderr}"
    );
    // The refusal must be a compile error, so nothing can be reported as a
    // test result at all -- a diagnostic printed beside a green `ok` line would
    // be the original defect with extra noise.
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert!(
        !stdout.contains("... ok"),
        "no test may be reported as passing: {stdout}"
    );
}

/// The supported shape runs the async body, and a wrong answer inside it FAILS.
///
/// `helper` suspends at a real `yield_now().await` before producing its value,
/// so passing here cannot be satisfied by a future that was built and dropped:
/// the state machine has to be resumed after a suspension for `41` to exist at
/// all. The failing twin pins the same thing from the other side — the rendered
/// `41 != 999` is the value that came back out of the future, so a runner that
/// silently skipped the body could not produce it.
///
/// Asserts the entire rendered stdout rather than a fragment, the same reason
/// `nova_test_reports_a_pass_and_a_failure_distinctly` does.
#[test]
fn nova_test_runs_an_async_body_via_block_on_and_pins_a_wrong_answer() {
    let dir = write_test_project(
        "nova-test-async-block-on",
        "async fn helper() -> Int {\n  yield_now().await\n  41\n}\n\
         @test\nfn an_async_body_runs_under_block_on() { assert_eq(block_on(helper()), 41) }\n\
         @test\nfn a_wrong_answer_inside_block_on_fails() { assert_eq(block_on(helper()), 999) }\n",
    );
    let assert = nova().current_dir(&dir).arg("test").assert().failure();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout).replace("\r\n", "\n");
    assert_eq!(
        stdout,
        "running 2 tests\n\
         test an_async_body_runs_under_block_on ... ok\n\
         test a_wrong_answer_inside_block_on_fails ... FAILED\n\
         \x20\x20\x20\x20nova: panic: assertion failed: 41 != 999\n\
         \n\
         test result: FAILED. 1 passed; 1 failed; 0 trapped; 2 total\n"
    );
}

// === Whole-branch review, finding 1 (CRITICAL): `@test` functions must not
// break `nova check` / `nova build` / `nova run` ===
//
// Before this fix, a `@test` function was an ordinary item in every
// compilation unit regardless of `with_test_module` — `nova_resolver::
// collect_item` collected it, gave it a `DefId`, and `nova_typeck::check`
// type-checked its body exactly like any other function. That body is
// written assuming `std/test`'s `assert`/`assert_eq`/`assert_ne` are in
// scope; they are seeded only when compiling for `nova test`
// (`nova_driver::FrontendContext::check`'s own doc comment). So a source
// file containing so much as one `@test` function that called `assert_eq`
// could not be `nova check`ed, `nova build`t, or `nova run` at all —
// `error[E0001]: cannot find function `assert_eq`` on every call — even
// though `nova test` itself compiled and ran the identical file correctly.
// `nova check` is the editor path, so this reproduced on every keystroke in
// any project with tests, and `nova build && nova test` could never both
// succeed on one tree. Fixed by `nova_driver::strip_test_functions`, which
// removes every top-level `@test` function from the compilation unit before
// `nova_resolver::resolve_program` ever sees it, whenever `with_test_module`
// is `false`.

/// The reviewer's exact repro, reduced to a standalone fixture: a file with
/// `@test` functions calling `assert_eq` must compile clean under `nova
/// check`, `nova build` and `nova run` (previously: two `E0001`s under each,
/// one per `assert_eq` call), and the identical, unmodified file must still
/// run correctly under `nova test` — proving the fix removes `@test`
/// functions from non-test compilation without disturbing their real
/// execution. Asserts both halves, per the finding.
#[test]
fn test_functions_calling_assert_eq_do_not_break_check_build_or_run() {
    let dir = write_test_project(
        "nova-test-cli-critical-nontest-compile",
        "@test\nfn addition_is_correct() { assert_eq(2 + 2, 4) }\n\
         @test\nfn addition_is_wrong() { assert_eq(2 + 2, 5) }\n\
         fn main() { println(\"hello\") }\n",
    );
    let file = dir.join("src/main.nova");

    // `nova check`: previously two `E0001`s (one per `assert_eq` call).
    nova().arg("check").arg(&file).assert().success();

    // `nova build`: the same program, through the same broken path
    // (`build_file` -> `lower_to_mir` -> `FrontendContext::check(false)`).
    // The built binary runs the user's ordinary `main` — `@test` functions
    // are absent from this compilation entirely, not merely unreachable.
    let out_exe = dir.join(format!("built{}", std::env::consts::EXE_SUFFIX));
    nova()
        .arg("build")
        .arg(&file)
        .arg("-o")
        .arg(&out_exe)
        .assert()
        .success();
    let run = Command::new(&out_exe).output().expect("run built binary");
    assert!(run.status.success());
    assert_eq!(
        String::from_utf8_lossy(&run.stdout).replace("\r\n", "\n"),
        "hello\n"
    );
    let _ = std::fs::remove_file(&out_exe);

    // `nova run`: same broken path via the JIT (`run_file` -> `compile_file`
    // -> `FrontendContext::check(false)`).
    nova()
        .arg("run")
        .arg(&file)
        .assert()
        .success()
        .stdout("hello\n");

    // The identical file, completely unmodified, still compiles and runs
    // correctly under `nova test` — one pass, one fail, exact message.
    let assert = nova().current_dir(&dir).arg("test").assert().failure();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout).replace("\r\n", "\n");
    assert_eq!(
        stdout,
        "running 2 tests\n\
         test addition_is_correct ... ok\n\
         test addition_is_wrong ... FAILED\n\
         \x20\x20\x20\x20nova: panic: assertion failed: 4 != 5\n\
         \n\
         test result: FAILED. 1 passed; 1 failed; 0 trapped; 2 total\n"
    );
}

/// The same fix, but for a `@test` function that lives in an *imported*
/// module rather than the entry file itself. `nova_driver::
/// strip_test_functions` is applied to every module `load_modules` returns,
/// not only the entry file's — a `@test` function reached only through
/// `import` is just as capable of referencing `assert_eq` and hitting the
/// identical `E0001` if it is not also stripped there.
#[test]
fn test_functions_in_an_imported_module_do_not_break_check_or_build() {
    let dir = write_test_project(
        "nova-test-cli-critical-nontest-compile-imported",
        "import helper\nfn main() { println(\"hello\") }\n",
    );
    std::fs::write(
        dir.join("src/helper.nova"),
        "@test\nfn helper_test() { assert_eq(1 + 1, 2) }\n",
    )
    .expect("write imported module");
    let file = dir.join("src/main.nova");

    nova().arg("check").arg(&file).assert().success();

    let out_exe = dir.join(format!("built-imported{}", std::env::consts::EXE_SUFFIX));
    nova()
        .arg("build")
        .arg(&file)
        .arg("-o")
        .arg(&out_exe)
        .assert()
        .success();
    let _ = std::fs::remove_file(&out_exe);
}

// === Whole-branch review, finding 3 (Important): captured output must not
// be discarded for a non-passing test ===
//
// Before this fix, `Outcome::Trapped` carried only the raw exit code — a
// trapping test rendered `TRAPPED (exit code N)` and nothing else, no
// matter what it (or the runtime) had written to either stream — and a
// FAILED test's own `println` output was silently dropped too, since only
// the one matched panic-marker line from stderr survived. `classify`
// (`cmd/test.rs`) always had the full `Output` in hand; the fix carries it
// through to the report via `print_captured`, printed only for a stream
// that actually has something in it.

/// A test that `println`s before its `assert_eq` fails: the failure summary
/// still shows the panic message exactly as before, and now also shows what
/// the test itself printed, which used to be discarded entirely.
#[test]
fn a_failing_tests_own_stdout_is_no_longer_discarded() {
    let dir = write_test_project(
        "nova-test-cli-captured-stdout-on-failure",
        "@test\nfn prints_then_fails() {\n\
         \x20\x20\x20\x20println(\"about to fail\")\n\
         \x20\x20\x20\x20assert_eq(1, 2)\n\
         }\n",
    );
    let assert = nova().current_dir(&dir).arg("test").assert().failure();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout).replace("\r\n", "\n");
    assert_eq!(
        stdout,
        "running 1 test\n\
         test prints_then_fails ... FAILED\n\
         \x20\x20\x20\x20nova: panic: assertion failed: 1 != 2\n\
         \x20\x20\x20\x20---- stdout ----\n\
         \x20\x20\x20\x20about to fail\n\
         \n\
         test result: FAILED. 0 passed; 1 failed; 0 trapped; 1 total\n"
    );
}

/// A trap must show captured output too, not just the bare exit code — the
/// entire point of this finding: reporting a trap with *less* information
/// than an ordinary failure is backwards given this branch's central claim
/// that traps deserve their own outcome. The `(code, hex)` pair below comes
/// from `expect_trap_exit_code`, exactly as
/// `a_hard_trap_is_reported_as_a_trap_and_does_not_satisfy_should_panic`
/// uses it: measured directly on Windows, pinned to `classify`'s own
/// documented fallback on Unix rather than assumed either way (design doc
/// §9 risk 4: not portable) — see that test's doc comment, or the helper's,
/// for why the Unix half is weaker than a measurement but not a tautology.
#[test]
fn a_trapping_tests_captured_output_is_shown_not_discarded() {
    let dir = write_test_project(
        "nova-test-cli-captured-output-on-trap",
        "@test(should_panic)\nfn prints_then_traps() {\n\
         \x20\x20\x20\x20println(\"about to trap\")\n\
         \x20\x20\x20\x20let _ = 1 / 0\n\
         }\n",
    );

    let (exe, tests) = nova_driver::build_test_binary(&dir.join("src/main.nova"))
        .expect("test binary compiles and links");
    assert_eq!(
        tests.iter().map(|t| t.name.as_str()).collect::<Vec<_>>(),
        vec!["prints_then_traps"]
    );
    let direct = Command::new(&exe)
        .env("NOVA_TEST_INDEX", "0")
        .output()
        .expect("run the trap directly");
    let (code, hex) = expect_trap_exit_code(&direct.status);
    let _ = std::fs::remove_file(&exe);

    let assert = nova().current_dir(&dir).arg("test").assert().failure();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout).replace("\r\n", "\n");
    assert_eq!(
        stdout,
        format!(
            "running 1 test\n\
             test prints_then_traps ... TRAPPED (exit code {code} (0x{hex:08X}))\n\
             \x20\x20\x20\x20---- stdout ----\n\
             \x20\x20\x20\x20about to trap\n\
             \n\
             test result: FAILED. 0 passed; 0 failed; 1 trapped; 1 total\n"
        )
    );
}

// === Whole-branch review, finding 4 (Important): an explicit filter
// matching zero tests must fail, not silently report success ===
//
// Before this fix, `nova test <typo>` printed `running 0 tests` / `test
// result: ok` and exited 0 — indistinguishable from an unfiltered run of a
// project with no tests at all, so a typo'd CI filter reported green having
// run nothing (ADR 0008 §2's own residual-gaps list named this).

/// `nova test <typo>` must exit nonzero and name the filter, not report
/// success having run nothing.
#[test]
fn an_explicit_filter_matching_nothing_is_a_failure() {
    let dir = write_test_project(
        "nova-test-cli-filter-matches-nothing",
        "@test\nfn addition_is_correct() { assert_eq(1 + 1, 2) }\n",
    );
    let assert = nova()
        .current_dir(&dir)
        .arg("test")
        .arg("nosuchtest")
        .assert()
        .failure();
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(
        stderr.contains("no test name contains the filter `nosuchtest`"),
        "stderr: {stderr}"
    );
    // Nothing ran: no `running N tests`, no per-test line, no summary.
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert_eq!(stdout, "", "stdout: {stdout}");
}

/// The one case finding 4 must NOT touch: an *unfiltered* run of a file with
/// no tests at all is not an error — nothing was mistyped; there is simply
/// nothing to run, same as before this fix.
#[test]
fn an_unfiltered_run_with_no_tests_still_succeeds() {
    let dir = write_test_project("nova-test-cli-no-tests-at-all", "fn main() { }\n");
    let assert = nova().current_dir(&dir).arg("test").assert().success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout).replace("\r\n", "\n");
    assert_eq!(
        stdout,
        "running 0 tests\n\
         \n\
         test result: ok. 0 passed; 0 failed; 0 trapped; 0 total\n"
    );
}

// === Task 1 (std/fs on Strings increment): `std/io` error types, and the
// `eprint`/`eprintln` builtins ===

/// `IoError`/`IoErrorKind` resolve with no `import`, and `io_error_kind_of`
/// maps a runtime status code to the right variant — including an unmapped
/// code (`99`), which must fall to `Other` rather than doing something
/// undefined; that arm is otherwise unreachable from Rust.
#[test]
fn fs_io_types_run() {
    let expected = std::fs::read_to_string(repo_root().join("tests/runtime/fs_io_types.stdout"))
        .expect("expected-output fixture exists")
        .replace("\r\n", "\n");
    nova()
        .arg("run")
        .arg(repo_root().join("tests/runtime/fs_io_types.nova"))
        .assert()
        .success()
        .stdout(expected);
}

/// `eprint`/`eprintln` write to stderr, not stdout — the split is the whole
/// point, so this pins both streams exactly rather than only checking stdout.
/// A version that wrote either to stdout instead must fail this.
#[test]
fn eprint_family_run() {
    let expected = std::fs::read_to_string(repo_root().join("tests/runtime/eprint_family.stdout"))
        .expect("expected-output fixture exists")
        .replace("\r\n", "\n");
    nova()
        .arg("run")
        .arg(repo_root().join("tests/runtime/eprint_family.nova"))
        .assert()
        .success()
        .stdout(expected)
        .stderr("ab\n");
}

// === Task 2 (std/fs on Strings increment): the boundary, `read_to_string`,
// and `write_string` ===

/// Reading a path that does not exist reports `NotFound`, round-tripped
/// through the status-code boundary in `crates/nova-runtime/src/fs.rs` and
/// back out through `io_error_kind_of` in `std/io`. Asserts on `kind` only,
/// never `message`, per the spec's diagnostics rule -- the OS's own wording
/// for "no such file" is platform-specific.
#[test]
fn fs_not_found_run() {
    let expected = std::fs::read_to_string(repo_root().join("tests/runtime/fs_not_found.stdout"))
        .expect("expected-output fixture exists")
        .replace("\r\n", "\n");
    nova()
        .arg("run")
        .arg(repo_root().join("tests/runtime/fs_not_found.nova"))
        .assert()
        .success()
        .stdout(expected);
}

/// The same fixture again through `nova build` -- a standalone linked
/// executable rather than the JIT (see `gate_async_tasks_build_standalone`'s
/// doc comment for why this does not itself cross the Cranelift/LLVM
/// boundary). Closes half of I4(b) from the final review: before this, no
/// `std/fs` fixture ran under anything but `nova run`, even though the
/// reviewer checked by hand that `nova build` works. This particular
/// fixture needs no temp directory or environment override -- it only reads
/// a path that must not exist -- so it is the cheapest fs fixture to route
/// through `build_and_run` as it stands.
#[test]
fn fs_not_found_build_standalone() {
    let expected = std::fs::read_to_string(repo_root().join("tests/runtime/fs_not_found.stdout"))
        .expect("expected-output fixture exists")
        .replace("\r\n", "\n");
    let out = build_and_run("tests/runtime/fs_not_found.nova", "fs_not_found");
    assert_eq!(out.replace("\r\n", "\n"), expected);
}

/// **MEASURED, not assumed -- on Windows only (final review, I1).** Pins the
/// one arm of the Rust-side status↔kind mapping (`crates/nova-runtime/src/fs.rs`'s
/// `fail`) that is both reachable from `std/fs` and was previously unpinned:
/// `read_to_string` on a directory reports `PermissionDenied` here. Swapping
/// `PERMISSION_DENIED` with another status in `fail` (the review's own
/// mutation) now fails this fixture; before this test existed, that swap
/// survived the entire suite. `#[cfg(windows)]` because Windows is the only
/// platform this was measured on -- the same call reports a different kind
/// (`Other`, via `Uncategorized`) on POSIX, per `docs/adr/0011-io-error-kinds.md`,
/// so asserting `PermissionDenied` unconditionally would be wrong there, not
/// merely untested.
#[cfg(windows)]
#[test]
fn fs_permission_denied_run() {
    let tmp = unique_temp_dir("nova-fs-permission-denied");
    let expected =
        std::fs::read_to_string(repo_root().join("tests/runtime/fs_permission_denied.stdout"))
            .expect("expected-output fixture exists")
            .replace("\r\n", "\n");
    nova()
        .arg("run")
        .arg(repo_root().join("tests/runtime/fs_permission_denied.nova"))
        .env("TMPDIR", &tmp)
        .env("TMP", &tmp)
        .env("TEMP", &tmp)
        .assert()
        .success()
        .stdout(expected);
    let _ = std::fs::remove_dir_all(&tmp);
}

/// Non-UTF-8 file contents report `InvalidData`, not a panic and not lossy
/// replacement.
///
/// **Before the byte-type increment's Task 4, `write_string` only accepted a
/// `String`, so the invalid bytes could only be placed on disk from Rust** --
/// this harness wrote them directly before invoking the fixture, with its own
/// process-id-namespaced directory and `.current_dir` (since the fixture then
/// read a bare relative filename). Task 4's `write` accepts `Bytes`, so the
/// fixture now writes its own non-UTF-8 payload via
/// `write(p, bytes_from_ints([0xFF, 0xFE])).await`, and this harness shrinks
/// to the same `unique_temp_dir` + `TMPDIR`/`TMP`/`TEMP` shape every other
/// `fs_` fixture that touches a real path already uses -- nothing
/// filesystem-specific to this one test remains here. See the task report
/// for the exact before/after.
#[test]
fn fs_invalid_data_run() {
    let tmp = unique_temp_dir("nova-fs-invalid-data");
    let expected =
        std::fs::read_to_string(repo_root().join("tests/runtime/fs_invalid_data.stdout"))
            .expect("expected-output fixture exists")
            .replace("\r\n", "\n");
    nova()
        .arg("run")
        .arg(repo_root().join("tests/runtime/fs_invalid_data.nova"))
        .env("TMPDIR", &tmp)
        .env("TMP", &tmp)
        .env("TEMP", &tmp)
        .assert()
        .success()
        .stdout(expected);
    let _ = std::fs::remove_dir_all(&tmp);
}

/// Closes review findings I1/I2 on this task: `fs_not_found` and
/// `fs_invalid_data` above exercise only the error branch of
/// `read_to_string`/`write_string`, so neither `write_string` actually
/// touching disk nor `read_to_string`'s success arm delivering the right
/// payload was ever exercised by anything. Proven by mutation, not argued:
/// replacing `nova_rt_fs_write_string`'s body with a no-op returning success
/// left every target in the workspace passing, and discarding the payload in
/// `nova_rt_fs_read_to_string`'s `Ok` arm (returning `NOT_FOUND` instead)
/// left every test in this binary and in `fs.rs`'s own suite passing too.
/// See the task report for both transcripts.
///
/// A real write-then-read round trip needs a writable path, which is what
/// `temp_dir` (pulled forward from Task 3 for exactly this reason) supplies.
/// This is deliberately not named or shaped like Task 3's own
/// `fs_roundtrip.nova` (which additionally exercises `remove_file`):
/// `write_string`/`read_to_string` accept only a `String` and `std/fs` had no
/// delete operation when this fixture was written, so *this* harness deletes
/// the file directly, both before (a stale file from an interrupted prior
/// run must not produce a false pass) and after (leave nothing behind).
///
/// **The literal filename below used to be joined onto the OS-shared temp
/// directory directly** (Task 3 review): two concurrent `cargo test`
/// invocations of this same binary would then race on one path. The
/// before/after `remove_file` calls only ever protected against a *stale*
/// file left by a non-overlapping prior run; they cannot protect against a
/// second process deleting or overwriting the file out from under this one
/// mid-test. `unique_temp_dir` fixes the actual collision by redirecting
/// *this process's* view of `temp_dir()` (via `TMP`/`TEMP`/`TMPDIR`, scoped
/// to the child `nova` process only) to a directory namespaced by this test
/// binary's own process id, so the path this fixture computes at runtime can
/// no longer collide with another process's. See `unique_temp_dir`'s doc
/// comment for why this is an environment override rather than a Nova-level
/// change, and the task report for how this was verified.
#[test]
fn fs_write_then_read_run() {
    let tmp = unique_temp_dir("nova-fs-write-then-read");
    let path = tmp.join("nova_fs_write_then_read_8a41.txt");
    let _ = std::fs::remove_file(&path);

    let expected =
        std::fs::read_to_string(repo_root().join("tests/runtime/fs_write_then_read.stdout"))
            .expect("expected-output fixture exists")
            .replace("\r\n", "\n");
    nova()
        .arg("run")
        .arg(repo_root().join("tests/runtime/fs_write_then_read.nova"))
        .env("TMPDIR", &tmp)
        .env("TMP", &tmp)
        .env("TEMP", &tmp)
        .assert()
        .success()
        .stdout(expected);

    // M4 (final review). MEASURED: temporarily hardcoding
    // `nova_rt_fs_temp_dir` to always create and return
    // `C:/nova-mutant-tempdir-probe`, ignoring `TMPDIR`/`TMP`/`TEMP` entirely,
    // still produced byte-identical stdout here -- `write_string` and
    // `read_to_string` both resolve `temp_dir()` themselves, so the mutant
    // stayed internally self-consistent. The assertion below is what caught
    // it: `fs_write_then_read.nova` never deletes what it writes, so
    // asserting the file exists at exactly the path this harness computed
    // from its own override -- before this harness's own cleanup removes it
    // -- pins that `temp_dir()` returned what those env vars said, not merely
    // *a* writable directory. Reverted after measuring; not one of this
    // fix wave's three required mutation transcripts, but cheap to check.
    assert!(
        path.exists(),
        "the write should have landed at {} (the path this harness computed \
         from its own TMPDIR/TMP/TEMP override), not merely somewhere writable",
        path.display()
    );

    let _ = std::fs::remove_dir_all(&tmp);
}

// === Task 3 (std/fs on Strings increment): exists, create_dir,
// create_dir_all, remove_file, remove_dir_all ===

/// A fresh directory under the OS temp root, namespaced by `label` and this
/// test binary's own process id, for a fixture that calls Nova's `temp_dir()`.
///
/// Every fixture below (and `fs_write_then_read_run` above) builds its path
/// as `"${temp_dir()}/<literal>"` *inside the compiled Nova program*, per
/// this task's brief -- so, unlike `fs_invalid_data_run`'s bare relative
/// filename plus `.current_dir`, pointing the spawned `nova` process's
/// working directory elsewhere would not change the path the fixture
/// actually touches: `temp_dir()` resolves the OS-wide shared temp
/// directory regardless of the current directory. What Nova's `temp_dir()`
/// *does* respond to is the same thing Rust's `std::env::temp_dir()` (which
/// `nova_rt_fs_temp_dir` calls directly) responds to: the `TMP`/`TEMP`
/// (Windows) and `TMPDIR` (POSIX) environment variables, read fresh every
/// call rather than cached at process start. Setting them only on the
/// spawned `nova` `Command` -- never via `std::env::set_var` on this test
/// binary's own process -- keeps the override scoped to that one child, so
/// it cannot bleed into a sibling test running on another thread of this
/// same binary (`cargo test`'s default parallelism runs many `#[test]`s
/// concurrently within one process, all sharing one real environment).
///
/// The process id, folded in after `label`, is what keeps two *separate*
/// invocations of this test binary (a developer re-running the suite while
/// a previous run is still finishing; two overlapping CI jobs on one
/// machine) from resolving `temp_dir()` to the same directory. `label`
/// alone already keeps sibling fixtures inside a single run from colliding
/// with each other. See `fs_write_then_read_run`'s doc comment for the
/// fixed, run-invariant path this replaces.
fn unique_temp_dir(label: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("{label}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("unique temp dir for a std/fs fixture");
    dir
}

/// `write_string`, `read_to_string`, `exists`, and `remove_file` in one pass
/// -- the operations Task 2 and this task between them add a status-boundary
/// wrapper for, chained through one real file.
///
/// **The stdout check alone does not exercise `remove_file`'s success arm**:
/// the fixture prints "removed" whether or not anything was actually
/// deleted. Proven, not assumed: replacing `nova_rt_fs_remove_file`'s body
/// with a no-op returning `OK` left this test passing against the stdout
/// check alone (see the task report for the transcript) -- the same shape
/// of gap Task 2's review found in `write_string`. The explicit `exists`
/// assertion below, checking the raw filesystem directly rather than
/// through the builtin, is what actually exercises the success arm.
///
/// **The `exists(p)` calls inside the fixture itself, bracketing
/// `remove_file`, are a review addition, not brief text.** Task 3's review
/// (finding I1) found that every `exists()` call anywhere in this suite ran
/// on a directory (`fs_dirs.nova`'s `d`, always freshly made by
/// `create_dir_all`), so changing `nova_rt_fs_exists` to call `.is_dir()`
/// instead of `.exists()` still passed every fixture and every
/// `nova-runtime` test. `p` here is a plain file, so `.is_dir()` and
/// `.exists()` disagree on it before removal (`false` vs. the expected
/// `true`) -- proven by mutation, transcript in the task report. This is the
/// one deliberate departure from "the fixture's text is the brief,
/// transcribed verbatim" in this task, made after review rather than
/// pre-emptively, and `fs_roundtrip.stdout` was updated to match (`true`
/// after the write, `false` after the remove).
#[test]
fn fs_roundtrip_run() {
    let tmp = unique_temp_dir("nova-fs-roundtrip");
    let written_path = tmp.join("nova_fs_roundtrip_5d2c.txt");
    let expected = std::fs::read_to_string(repo_root().join("tests/runtime/fs_roundtrip.stdout"))
        .expect("expected-output fixture exists")
        .replace("\r\n", "\n");
    nova()
        .arg("run")
        .arg(repo_root().join("tests/runtime/fs_roundtrip.nova"))
        .env("TMPDIR", &tmp)
        .env("TMP", &tmp)
        .env("TEMP", &tmp)
        .assert()
        .success()
        .stdout(expected);
    let removed = !written_path.exists();
    let _ = std::fs::remove_dir_all(&tmp);
    assert!(
        removed,
        "remove_file should have actually deleted {}, not merely reported success",
        written_path.display()
    );
}

/// A second `create_dir` on an existing directory reports `AlreadyExists`.
/// The fixture's own leading `remove_dir_all` (discarding the result: absent
/// is fine, present is cleaned up) makes this independent of a previous
/// run's leftovers; `unique_temp_dir` makes it independent of a *concurrent*
/// one. Together they cover both halves of the race this task's brief
/// called out -- see `fs_write_then_read_run`'s doc comment.
#[test]
fn fs_already_exists_run() {
    let tmp = unique_temp_dir("nova-fs-already-exists");
    let expected =
        std::fs::read_to_string(repo_root().join("tests/runtime/fs_already_exists.stdout"))
            .expect("expected-output fixture exists")
            .replace("\r\n", "\n");
    nova()
        .arg("run")
        .arg(repo_root().join("tests/runtime/fs_already_exists.nova"))
        .env("TMPDIR", &tmp)
        .env("TMP", &tmp)
        .env("TEMP", &tmp)
        .assert()
        .success()
        .stdout(expected);
    let _ = std::fs::remove_dir_all(&tmp);
}

/// `exists` bracketing a nested `create_dir_all`: absent, then present, then
/// absent again. Three calls rather than one so that a `nova_rt_fs_exists`
/// hard-coded to either `true` or `false` fails this fixture -- proven by
/// mutation in the task report, not just argued here.
#[test]
fn fs_dirs_run() {
    let tmp = unique_temp_dir("nova-fs-dirs");
    let expected = std::fs::read_to_string(repo_root().join("tests/runtime/fs_dirs.stdout"))
        .expect("expected-output fixture exists")
        .replace("\r\n", "\n");
    nova()
        .arg("run")
        .arg(repo_root().join("tests/runtime/fs_dirs.nova"))
        .env("TMPDIR", &tmp)
        .env("TMP", &tmp)
        .env("TEMP", &tmp)
        .assert()
        .success()
        .stdout(expected);
    let _ = std::fs::remove_dir_all(&tmp);
}

// === Task 4 (std/fs on Strings increment): read_dir, DirEntry ===

/// `read_dir` returns entries sorted by name, with `DirEntry.is_file`/`is_dir`
/// coming from one `fs_kind` call per entry. One entry (`Zebra.txt`) is
/// capitalized deliberately, so that byte-lexicographic sort (what the
/// runtime's `names.sort()` actually does: capitals order before lowercase)
/// disagrees with every case-insensitive or creation-order enumeration this
/// project has observed -- a missing `names.sort()` was measured to survive
/// an earlier, all-lowercase version of this fixture on this host, because
/// this host's raw directory order already happened to be alphabetical for
/// same-case names. See `tests/runtime/fs_read_dir.nova`'s own header and the
/// task report for the measurement.
///
/// This one fixture is written to discriminate three separate mutations:
/// dropping the sort (order changes), swapping `fs_kind`'s file/dir status
/// codes (the `mid` line inverts), and a wrong array length (a line goes
/// missing). See the task report for all three transcripts.
#[test]
fn fs_read_dir_run() {
    let tmp = unique_temp_dir("nova-fs-read-dir");
    let expected = std::fs::read_to_string(repo_root().join("tests/runtime/fs_read_dir.stdout"))
        .expect("expected-output fixture exists")
        .replace("\r\n", "\n");
    nova()
        .arg("run")
        .arg(repo_root().join("tests/runtime/fs_read_dir.nova"))
        .env("TMPDIR", &tmp)
        .env("TMP", &tmp)
        .env("TEMP", &tmp)
        .assert()
        .success()
        .stdout(expected);
    let _ = std::fs::remove_dir_all(&tmp);
}

/// The same fixture again with `NOVA_GC_STRESS=1` (collect on every
/// allocation) -- closing the other half of I4(b) from the final review.
///
/// **Why this fixture is the one that matters most, among the fs fixtures.**
/// This branch's own GC-rooting unit tests
/// (`a_stashed_string_is_rooted_until_it_is_taken`,
/// `stash_overwriting_an_occupied_slot_does_not_leak_the_displaced_root`) use
/// `gc::root_count`, which reads the root registry directly and never runs a
/// collection at all -- deliberately, per their own doc comments, because a
/// real collection inherits the conservative scanner's documented
/// intermittent over-retention (`docs/adr/0010-conservative-scan-root-test-gating.md`).
/// That leaves the actual end-to-end property -- a stashed payload survives a
/// collection that happens between the intrinsic call and the take --
/// guarded by nothing in this suite before this test. `read_dir` is the
/// richest fs fixture for exercising it: `stash_array` allocates one
/// `NovaStr` per entry inside a freshly rooted block, so with a collection
/// forced on every allocation, each element's string must survive both its
/// own allocation and every later one before `read_dir`'s Nova wrapper reads
/// the array back out.
///
/// Uses its own `unique_temp_dir` label so this directory never collides with
/// `fs_read_dir_run`'s, even though `cargo test`'s default parallelism can
/// run both concurrently in the same process (same pid, different label).
#[test]
fn fs_read_dir_under_gc_stress() {
    let tmp = unique_temp_dir("nova-fs-read-dir-stress");
    let expected = std::fs::read_to_string(repo_root().join("tests/runtime/fs_read_dir.stdout"))
        .expect("expected-output fixture exists")
        .replace("\r\n", "\n");
    nova()
        .env("NOVA_GC_STRESS", "1")
        .arg("run")
        .arg(repo_root().join("tests/runtime/fs_read_dir.nova"))
        .env("TMPDIR", &tmp)
        .env("TMP", &tmp)
        .env("TEMP", &tmp)
        .assert()
        .success()
        .stdout(expected);
    let _ = std::fs::remove_dir_all(&tmp);
}

// === byte-type increment, Task 1: `Bytes` joins `RESERVED_TYPE_NAMES` ===

/// `Bytes` must join `nova_resolver::RESERVED_TYPE_NAMES`, so a user's own
/// `record Bytes { .. }` is rejected with `E0089` rather than becoming a
/// silently-unusable declaration -- shadowed by the built-in in every
/// signature or `impl`, yet still constructible and matchable, which is
/// exactly the confusing split `RESERVED_TYPE_NAMES`'s own doc explains
/// reservation exists to prevent.
///
/// A checked-in fixture rather than this file's usual temp-dir string for an
/// `E00xx` test: every earlier pin of the reserved-name mechanism lives in
/// `nova-typeck`'s own `mod tests` (`check_src` against an inline string), so
/// this is the first end-to-end, CLI-level confirmation that the declaration
/// is rejected the same way through the real binary.
///
/// `check`, not `run`: the rejection happens at name resolution, long before
/// anything would execute, and `main` here is empty besides.
#[test]
fn bytes_reserved_declaration_is_rejected() {
    let file = repo_root().join("tests/runtime/bytes_reserved.nova");
    let assert = nova().arg("check").arg(&file).assert().failure();
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr).to_string();
    assert!(stderr.contains("E0089"), "stderr: {stderr}");
}

// === byte-type increment, Task 2: Bytes representation, len and to_string ===

/// End-to-end: `bytes_from_string` builds a `Bytes` value, `Bytes::len` reads
/// its byte length, and `Bytes::to_string` round-trips valid UTF-8 back to a
/// `String` through the `Some` arm of the `Option<String>` it returns.
#[test]
fn bytes_basics_run() {
    let expected = std::fs::read_to_string(repo_root().join("tests/runtime/bytes_basics.stdout"))
        .expect("expected-output fixture exists")
        .replace("\r\n", "\n");
    nova()
        .arg("run")
        .arg(repo_root().join("tests/runtime/bytes_basics.nova"))
        .assert()
        .success()
        .stdout(expected);
}

// === byte-type increment, Task 3: the rest of the byte surface ===

/// End-to-end: `bytes_from_ints` builds a non-UTF-8 buffer (byte `255`), and
/// every new method round-trips against it -- `byte_at` (both the in-range
/// and out-of-range arms), `slice`, `concat`, `to_ints` fed back through
/// `bytes_from_ints` and compared with `eq`, `index_of`, and `contains`. Byte
/// `255` is deliberate: it makes the buffer invalid UTF-8 (closing Task 2's
/// surviving `bytes_is_utf8` mutation, since Task 2's own fixtures never
/// exercised the "not UTF-8" arm end-to-end) and it proves `byte_at` returns
/// an unsigned value rather than a sign-extended `-1`.
///
/// **Two `eq` calls, deliberately not one.** `bytes_from_ints(b.to_ints()).eq(b)`
/// reconstructs a buffer that is byte-for-byte identical to `b`, so a
/// length-only `eq` mutation (comparing `len` alone, never the bytes) cannot
/// be told apart from a real one there -- both answer `true`. The second call
/// compares two three-byte buffers differing only in their last byte, which a
/// length-only comparison gets wrong (`true`) where a real one answers
/// `false`. Measured, not assumed: see the task report's mutation transcripts
/// for the run where the first call alone let that mutation survive.
///
/// **Review findings P1 and P3, folded in here rather than as separate
/// fixtures.** `b.slice(1, 100)` and `b.slice(100, 200)` both push a bound
/// past the buffer's 3-byte length -- the first past `end`, the second past
/// `start` as well -- which the original fixture never did (its only
/// `slice(0, 2)` call is in range on both bounds), so a missing
/// `.min(bytes.len())` on `nova_rt_bytes_slice`'s `hi` survived undetected;
/// see the task report for the mutation transcript. `bytes_from_ints([97,
/// 97, 97]).index_of(bytes_from_ints([97, 97]))` is the byte-domain mirror of
/// `tests/runtime/strings.nova`'s `"aaa".index_of("aa")`: the needle matches
/// at both index 0 and index 1, so this is what would catch `index_of`
/// returning the last match instead of the first -- the original fixture's
/// only search (`"i"` in `"hi\xff"`) matches exactly once and cannot tell the
/// two apart.
///
/// **Final-review findings D6 and D7, folded in here too.** `b.byte_at(0 - 1)`
/// is the byte-domain mirror of `tests/runtime/strings.nova`'s
/// `"héllo".char_at(0 - 1)`: `byte_at`'s negative-index guard had no fixture
/// coverage, so a deleted `if i < 0 { return None }` reached the
/// `bytes_at` intrinsic with a negative index and aborted the process
/// instead of returning `None`. `b.index_of(b)`/`b.contains(b)` cover the
/// equal-length needle/haystack case `tests/runtime/strings.nova` already
/// covers for `String`: changing `index_of`'s `n.len() > h.len()` guard to
/// `>=` survived the whole suite before this, since no prior call had a
/// needle exactly as long as its haystack.
#[test]
fn bytes_api_run() {
    let expected = std::fs::read_to_string(repo_root().join("tests/runtime/bytes_api.stdout"))
        .expect("expected-output fixture exists")
        .replace("\r\n", "\n");
    nova()
        .arg("run")
        .arg(repo_root().join("tests/runtime/bytes_api.nova"))
        .assert()
        .success()
        .stdout(expected);
}

/// The same fixture through `nova build` -- a standalone linked executable
/// rather than the JIT. Closes the `nova build` half of the byte-type plan's
/// definition of done (§9): unlike `std/fs`'s fixtures, which inherit `nova
/// build`/`NOVA_GC_STRESS=1` coverage from that increment's existing gate
/// wiring, no `bytes_*` fixture ran under either configuration before this --
/// a gap the plan's own self-review caught before implementation, rather than
/// after. This fixture needs no temp directory or environment override, so it
/// is as cheap to route through `build_and_run` as `fs_not_found` was for the
/// `std/fs` increment.
#[test]
fn bytes_api_build_standalone() {
    let expected = std::fs::read_to_string(repo_root().join("tests/runtime/bytes_api.stdout"))
        .expect("expected-output fixture exists")
        .replace("\r\n", "\n");
    let out = build_and_run("tests/runtime/bytes_api.nova", "bytes_api");
    assert_eq!(out.replace("\r\n", "\n"), expected);
}

/// The same fixture again with `NOVA_GC_STRESS=1` (collect on every
/// allocation) -- the other half of the byte-type plan's `nova build` /
/// `NOVA_GC_STRESS=1` definition-of-done gap. `to_ints`/`from_ints` allocate a
/// fresh scanned array block, `slice`/`concat`/`bytes_from_ints` each allocate
/// a fresh header and leaf buffer, and `index_of`/`contains` hold both
/// operands' arrays live across a Nova-level loop -- so a collection forced on
/// every allocation exercises every new intrinsic's rooting, not only the
/// four already covered by `bytes_basics`/`bytes_len`.
#[test]
fn bytes_api_under_gc_stress() {
    let expected = std::fs::read_to_string(repo_root().join("tests/runtime/bytes_api.stdout"))
        .expect("expected-output fixture exists")
        .replace("\r\n", "\n");
    nova()
        .env("NOVA_GC_STRESS", "1")
        .arg("run")
        .arg(repo_root().join("tests/runtime/bytes_api.nova"))
        .assert()
        .success()
        .stdout(expected);
}

/// Review finding P2 (Important): `bytes_from_ints`'s `0..=255` contract is
/// documented in three places (the plan, `bytes.rs`'s doc comment, and
/// `std/bytes/lib.nova`'s), and nothing exercised it -- replacing the
/// `u8::try_from`/`abort_with` pair with a wrapping `v as u8` passed the
/// whole suite, since nothing anywhere called it with a value outside the
/// range. An abort cannot be observed as a stdout diff (the process ends
/// before producing any more output), so this follows the same shape as
/// `panic_aborts_with_message`/`run_aborts_when_an_async_fn_calls_block_on`:
/// `nova run` on a fixture that aborts, asserting failure and the exact
/// `nova: panic:` message on stderr, exactly as `nova_rt_bytes_from_ints`'s
/// own `abort_with` call emits it.
///
/// This is the "too high" direction (`256`, one past the range). See
/// `bytes_from_ints_rejects_a_value_below_the_range` for "too low" (`-1`) --
/// covered separately because `u8::try_from` rejects each for a different
/// reason (`-1` fails on sign, `256` fails on magnitude), and a mutation
/// could plausibly get one direction right and the other wrong (e.g. `if v >
/// 255 { abort } else { v as u8 }` accepts `256`'s rejection but wraps `-1`
/// silently).
#[test]
fn bytes_from_ints_rejects_a_value_above_the_range() {
    let assert = nova()
        .arg("run")
        .arg(repo_root().join("tests/runtime/bytes_from_ints_too_high.nova"))
        .assert()
        .failure();
    let out = assert.get_output();
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("nova: panic: nova_rt_bytes_from_ints: element out of range 0..=255"),
        "stderr was {stderr:?}"
    );
    // Nothing after the aborting call ever runs.
    assert!(!stdout.contains('1'), "stdout was {stdout:?}");
}

/// The "too low" direction (`-1`) of P2 -- see the sibling test above.
#[test]
fn bytes_from_ints_rejects_a_value_below_the_range() {
    let assert = nova()
        .arg("run")
        .arg(repo_root().join("tests/runtime/bytes_from_ints_too_low.nova"))
        .assert()
        .failure();
    let out = assert.get_output();
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("nova: panic: nova_rt_bytes_from_ints: element out of range 0..=255"),
        "stderr was {stderr:?}"
    );
    assert!(!stdout.contains('1'), "stdout was {stdout:?}");
}

// === byte-type increment, Task 4: byte-based fs::read and fs::write ===

/// `write`/`read` round-trip a non-UTF-8 payload (bytes `0`, `255`, `10`,
/// `254`, `65`) through a real file: `len`, `eq` against the original
/// payload, and `to_string().is_none()` (this buffer is not valid UTF-8), so
/// a version that routed through `String` anywhere in the path -- write or
/// read -- cannot pass. See the task report's mutation transcripts for the
/// specific implementations this fixture was checked against.
#[test]
fn fs_bytes_roundtrip_run() {
    let tmp = unique_temp_dir("nova-fs-bytes-roundtrip");
    let expected =
        std::fs::read_to_string(repo_root().join("tests/runtime/fs_bytes_roundtrip.stdout"))
            .expect("expected-output fixture exists")
            .replace("\r\n", "\n");
    nova()
        .arg("run")
        .arg(repo_root().join("tests/runtime/fs_bytes_roundtrip.nova"))
        .env("TMPDIR", &tmp)
        .env("TMP", &tmp)
        .env("TEMP", &tmp)
        .assert()
        .success()
        .stdout(expected);
    let _ = std::fs::remove_dir_all(&tmp);
}

/// The same fixture again with `NOVA_GC_STRESS=1` (collect on every
/// allocation). `nova_rt_fs_read` stashes a freshly allocated `Bytes` header
/// and leaf buffer into the current task's `Slot::Buffer` entry, GC-rooting it
/// for exactly the span between the intrinsic call and the wrapper's own read
/// of that slot (`crates/nova-runtime/src/fs.rs`) -- the same rooting
/// contract `fs_read_dir_under_gc_stress` exists to exercise for
/// `stash_array`, applied here to the payload slot's newest producer. Its own
/// label keeps this directory from colliding with `fs_bytes_roundtrip_run`'s,
/// even though `cargo test`'s default parallelism can run both concurrently
/// in the same process.
#[test]
fn fs_bytes_roundtrip_under_gc_stress() {
    let tmp = unique_temp_dir("nova-fs-bytes-roundtrip-stress");
    let expected =
        std::fs::read_to_string(repo_root().join("tests/runtime/fs_bytes_roundtrip.stdout"))
            .expect("expected-output fixture exists")
            .replace("\r\n", "\n");
    nova()
        .env("NOVA_GC_STRESS", "1")
        .arg("run")
        .arg(repo_root().join("tests/runtime/fs_bytes_roundtrip.nova"))
        .env("TMPDIR", &tmp)
        .env("TMP", &tmp)
        .env("TEMP", &tmp)
        .assert()
        .success()
        .stdout(expected);
    let _ = std::fs::remove_dir_all(&tmp);
}

/// `read`'s and `write`'s `Err` arms, neither exercised by
/// `fs_bytes_roundtrip_run` (which only ever takes the success path for
/// both). **Proven by mutation, not argued**: deleting the `if status == 0`
/// check from either wrapper in `std/fs/lib.nova` -- so it unconditionally
/// returns `Ok`, discarding a real failure status -- left every target in
/// this workspace passing, `fs_bytes_roundtrip_run` included, the identical
/// shape of gap `fs_write_then_read_run`'s own doc comment records for
/// `write_string`. See the task report for both transcripts.
///
/// Needs no `unique_temp_dir`: like `fs_not_found_run`, neither branch ever
/// creates anything on disk (the whole point is that both fail before
/// touching the filesystem), so there is nothing to namespace or clean up and
/// a bare relative path cannot collide with a concurrent run the way a
/// created file or directory could.
#[test]
fn fs_bytes_errors_run() {
    let expected =
        std::fs::read_to_string(repo_root().join("tests/runtime/fs_bytes_errors.stdout"))
            .expect("expected-output fixture exists")
            .replace("\r\n", "\n");
    nova()
        .arg("run")
        .arg(repo_root().join("tests/runtime/fs_bytes_errors.nova"))
        .assert()
        .success()
        .stdout(expected);
}

// === Task 3 (read-write-and-stdio increment): `std/io`'s `Read`/`Write`
// surface over the three standard streams, and `read_to_end`'s default ===

/// The stdout marker `tests/runtime/io_streams.nova` writes: `"Q".repeat(300)`
/// plus a trailing newline. Built the same way here as there, rather than
/// copied out as a 301-character literal, so a future change to the repeat
/// count is a one-word diff to compare instead of a wall of `Q`s to eyeball.
/// Deliberately 256 bytes or longer: a write's byte count crosses the
/// runtime boundary as 8 little-endian bytes
/// (`crates/nova-runtime/src/io.rs`'s `stash_count`), and a Nova-level decode
/// that read back only the first of them would under-report any count at or
/// above 256 without a marker this long catching it.
fn io_streams_stdout_marker() -> String {
    format!("{}\n", "Q".repeat(300))
}

/// The exact stderr content `tests/runtime/io_streams.nova` writes -- nothing
/// else in that fixture ever writes to stderr.
const IO_STREAMS_STDERR_MARKER: &str = "NOVA_STDERR_ONLY_MARKER\n";

/// `Read`/`Write` against the three real standard streams: `nova run` as a
/// child process with stdout and stderr captured as two independent
/// `std::process::Output` buffers, stdin fed a fixed string from this
/// harness rather than a terminal (so this test cannot hang waiting on one).
///
/// **The cross-stream check is the point of this test, not a bonus.** A
/// `Write for Stdout` wired to the wrong OS stream (or the reverse) still
/// produces a program that runs, exits `0`, and prints the right status
/// lines -- only capturing both streams separately and checking that each
/// contains *only* its own marker catches that. This is not hypothetical:
/// Task 1's own report records that exactly this mutation
/// (`nova_rt_io_stdout_write` writing to stderr) survived that task's entire
/// test suite, because nothing at that layer observes which real OS stream
/// received the bytes -- closing that gap is what this test exists for.
#[test]
fn io_streams_run() {
    let expected = std::fs::read_to_string(repo_root().join("tests/runtime/io_streams.stdout"))
        .expect("expected-output fixture exists")
        .replace("\r\n", "\n");
    let assert = nova()
        .arg("run")
        .arg(repo_root().join("tests/runtime/io_streams.nova"))
        .write_stdin("hello-stdin")
        .assert()
        .success();
    let out = assert.get_output();
    let stdout = String::from_utf8_lossy(&out.stdout).replace("\r\n", "\n");
    let stderr = String::from_utf8_lossy(&out.stderr).replace("\r\n", "\n");

    assert_eq!(stdout, expected, "stdout did not match the golden fixture");
    assert_eq!(
        stderr, IO_STREAMS_STDERR_MARKER,
        "stderr must hold exactly the stderr marker and nothing else"
    );

    let stdout_marker = io_streams_stdout_marker();
    assert!(
        stdout.contains(&stdout_marker),
        "stdout must contain its own marker: {stdout:?}"
    );
    assert!(
        stderr.contains("NOVA_STDERR_ONLY_MARKER"),
        "stderr must contain its own marker: {stderr:?}"
    );
    // The negative half. Without this pair, a mutation that leaks one
    // stream's bytes into the other (rather than swapping them outright)
    // could still pass the two `.contains()` checks just above -- each of
    // those only checks "did my own marker appear somewhere," which stays
    // true even when the wrong stream *also* picked it up. This is what
    // actually catches bytes landing on the wrong stream in that case.
    assert!(
        !stdout.contains("NOVA_STDERR_ONLY_MARKER"),
        "stdout must not contain stderr's marker: {stdout:?}"
    );
    assert!(
        !stderr.contains(&stdout_marker),
        "stderr must not contain stdout's marker: {stderr:?}"
    );
}

/// `io_streams_run` again through `nova build` -- a standalone linked
/// executable rather than the JIT -- exercising the same cross-stream
/// separation on the other backend. Written inline rather than through the
/// shared `build_and_run` helper: that helper keeps only stdout, and the
/// entire point here is comparing both streams independently.
#[test]
fn io_streams_build_standalone() {
    let expected = std::fs::read_to_string(repo_root().join("tests/runtime/io_streams.stdout"))
        .expect("expected-output fixture exists")
        .replace("\r\n", "\n");
    let dir = std::env::temp_dir().join("nova-io-streams-build");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let exe = dir.join(format!("io_streams{}", std::env::consts::EXE_SUFFIX));
    nova()
        .arg("build")
        .arg(repo_root().join("tests/runtime/io_streams.nova"))
        .arg("-o")
        .arg(&exe)
        .assert()
        .success();

    let assert = Command::new(&exe)
        .write_stdin("hello-stdin")
        .assert()
        .success();
    let out = assert.get_output();
    let stdout = String::from_utf8_lossy(&out.stdout).replace("\r\n", "\n");
    let stderr = String::from_utf8_lossy(&out.stderr).replace("\r\n", "\n");
    let _ = std::fs::remove_file(&exe);

    assert_eq!(stdout, expected, "stdout did not match the golden fixture");
    assert_eq!(
        stderr, IO_STREAMS_STDERR_MARKER,
        "stderr must hold exactly the stderr marker and nothing else"
    );
    assert!(
        !stdout.contains("NOVA_STDERR_ONLY_MARKER"),
        "stdout must not contain stderr's marker: {stdout:?}"
    );
    assert!(
        !stderr.contains(&io_streams_stdout_marker()),
        "stderr must not contain stdout's marker: {stderr:?}"
    );
}

/// `Read::read_to_end`'s default body (`read_all` in `std/io`), pinned
/// against fake readers with fully deterministic results. See `tests/runtime/
/// io_read_to_end.nova`'s own doc comment for how its `Chunks` reader
/// advances state between calls despite Nova records being values with no
/// `&mut`, and for why a separate `Failing` reader is there too.
#[test]
fn io_read_to_end_run() {
    let expected = std::fs::read_to_string(repo_root().join("tests/runtime/io_read_to_end.stdout"))
        .expect("expected-output fixture exists")
        .replace("\r\n", "\n");
    nova()
        .arg("run")
        .arg(repo_root().join("tests/runtime/io_read_to_end.nova"))
        .assert()
        .success()
        .stdout(expected);
}

/// Runs `path` (`tests/runtime/io_broken_stdout.nova` or
/// `io_broken_stderr.nova`) as a child process, deterministically breaking
/// the pipe behind whichever of its stdout/stderr the fixture writes to,
/// then returns the exit status and the full text of the *other* (healthy)
/// stream.
///
/// **MEASURED on Windows only (fix round 1: deleting `write_stdout`'s,
/// `write_stderr`'s, `flush_stdout`'s or `flush_stderr`'s status check was
/// reachable-but-untested).** The mechanism: `std::process::Command` with
/// all three standard streams piped, `spawn`, then this immediately drops
/// the parent's read end of `break_stdout`'s stream (`child.stdout`/
/// `child.stderr`) -- *before* touching `child.stdin` at all. Both fixtures
/// block on `stdin().read(1).await` as their very first action, so at the
/// moment this function drops that handle the child provably has not
/// written anything to either stream yet; only after the drop does this
/// function close `child.stdin`, releasing the child to run past its stdin
/// read. This ordering is what makes the break race-free rather than
/// merely likely: the child cannot reach its own writes until this
/// function's close-then-release sequence has already happened.
///
/// Once the target stream's only reader is gone, an OS write to it fails
/// (`BrokenPipe`, mapped by `fs.rs`'s `fail` to `IoErrorKind::Other`) --
/// confirmed by reading the healthy stream's own reported results, not
/// assumed from the exit status alone. Rust's `Stdout` is line-buffered, so
/// a payload ending in `"\n"` fails at the `write()` call itself, while one
/// without a trailing newline only fails later, at the next `flush()` --
/// `io_broken_stdout_run` exploits that split to reach `write_stdout` and
/// `flush_stdout` independently. `Stderr` turned out not to line-buffer at
/// all (measured in `io_broken_stderr_run`, not assumed), so the same split
/// reaches only `write_stderr`; see that test's own doc comment for what
/// this means for `flush_stderr`. See `io_broken_stdout.nova`/
/// `io_broken_stderr.nova` for why the healthy stream's reporting must go
/// through `print`/`eprint` and never `println`/`eprintln`.
///
/// Not attempted on non-Windows CI: the mechanism (drop a pipe's read end,
/// observe the writer's next operation fail) is standard POSIX behaviour
/// too, and grepping this workspace found no `SIGPIPE`/`signal`/`sigaction`
/// handling anywhere in `nova-runtime` that would turn a broken pipe into a
/// killed process instead of an `Err` -- Rust's own default of ignoring
/// `SIGPIPE` should apply unmodified. That is a reason to expect it works,
/// not a measurement that it does; this environment has no Linux or macOS
/// host to run it on, so the gate below is what was actually verified
/// against, not a guess about what would also pass.
#[cfg(windows)]
fn run_with_a_broken_pipe(
    path: &std::path::Path,
    break_stdout: bool,
) -> (std::process::ExitStatus, String) {
    use std::io::Read;
    use std::process::Stdio;

    let mut child = std::process::Command::new(assert_cmd::cargo::cargo_bin("nova"))
        .arg("run")
        .arg(path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn nova run");

    // Drop the parent's read end of the stream under test first, while the
    // child is still provably blocked on its own stdin read -- see this
    // function's own doc comment for why that ordering is what removes the
    // race rather than merely narrowing it.
    let mut healthy: Box<dyn Read> = if break_stdout {
        drop(child.stdout.take().expect("stdout was piped"));
        Box::new(child.stderr.take().expect("stderr was piped"))
    } else {
        drop(child.stderr.take().expect("stderr was piped"));
        Box::new(child.stdout.take().expect("stdout was piped"))
    };

    // Release the child from `stdin().read(1)` by closing its stdin.
    drop(child.stdin.take().expect("stdin was piped"));

    let mut out = String::new();
    healthy
        .read_to_string(&mut out)
        .expect("read the healthy stream to completion");
    let status = child.wait().expect("wait for the child");
    (status, out)
}

/// `write_stdout`'s and `flush_stdout`'s `Err` branches, reached through a
/// genuinely broken stdout pipe rather than argued to be unreachable. See
/// `run_with_a_broken_pipe`'s own doc comment for the mechanism and its
/// Windows-only scope, and `io_broken_stdout.nova`'s for why it reports on
/// stderr through `eprint` alone.
#[cfg(windows)]
#[test]
fn io_broken_stdout_run() {
    let (status, stderr) = run_with_a_broken_pipe(
        &repo_root().join("tests/runtime/io_broken_stdout.nova"),
        true,
    );
    assert!(status.success(), "the child process itself must exit 0");
    assert!(
        stderr.contains("write_stdout: err"),
        "write_stdout must report Err against a broken pipe: {stderr:?}"
    );
    assert!(
        stderr.contains("write_stdout_buffered: ok"),
        "a write with no trailing newline must still buffer successfully: {stderr:?}"
    );
    assert!(
        stderr.contains("flush_stdout: err"),
        "flush_stdout must report Err when the deferred write it flushes fails: {stderr:?}"
    );
}

/// `write_stderr`'s `Err` branch, reached the same way as
/// `io_broken_stdout_run`, against stderr instead.
///
/// **`flush_stderr`'s `Err` branch is NOT reached here, measured rather
/// than assumed.** `io_broken_stdout_run`'s deferred-write trick relies on
/// `Stdout` being line-buffered; `Stderr` is not buffered at all --
/// confirmed directly below, not inferred, because the *second* write (the
/// one with no trailing newline, which buffers and reports `Ok` for
/// stdout) already fails immediately here too. With nothing ever buffered,
/// the following `flush()` has nothing to flush and reports `Ok`
/// unconditionally, even though the pipe is still broken. So of the four
/// wrappers a mutated status check could hide behind, this fixture closes
/// three (`write_stdout`, `flush_stdout` above, `write_stderr` here);
/// `flush_stderr`'s status check has no fixture-reachable failure -- **settled, not open.**
/// Rust 1.95's own source closes it: `StderrLock::flush` reaches the shared `RefCell<StderrRaw>`
/// and calls `handle_ebadf(sys::stdio::Stderr::flush, || Ok(()))`, and the Windows platform
/// module's `impl io::Write for Stderr` hardcodes `fn flush(&mut self) -> io::Result<()> { Ok(())
/// }` -- there is no staging layer for a write to fail through, so this call cannot report `Err`
/// on this platform, by construction, regardless of the pipe's state. No second fixture technique
/// was needed: a Nova program has no way to invalidate its own file descriptors any more severely
/// than closing their readers, which this technique already does, and that would not matter here
/// regardless. **Measured by reading the standard library's own source on Windows; the identical
/// claim for Unix is reasoned from the same shape, not run** -- no Unix host was available to
/// check it. The `flush_stderr: ok` assertion below stays; it correctly pins today's platform
/// behaviour.
#[cfg(windows)]
#[test]
fn io_broken_stderr_run() {
    let (status, stdout) = run_with_a_broken_pipe(
        &repo_root().join("tests/runtime/io_broken_stderr.nova"),
        false,
    );
    assert!(status.success(), "the child process itself must exit 0");
    assert!(
        stdout.contains("write_stderr: err"),
        "write_stderr must report Err against a broken pipe: {stdout:?}"
    );
    assert!(
        stdout.contains("write_stderr_buffered: err"),
        "unlike stdout's, a write to stderr with no trailing newline must \
         still fail immediately against a broken pipe -- stderr does not \
         line-buffer: {stdout:?}"
    );
    assert!(
        stdout.contains("flush_stderr: ok"),
        "with nothing buffered (stderr never buffered either of the writes \
         above), flushing stderr must report Ok even though the pipe is \
         broken -- there is nothing pending for it to fail on: {stdout:?}"
    );
}

/// Runs `path` (`tests/runtime/io_read_stdin_write_only.nova`) as a child
/// process whose **stdin is a write-only file handle**, and returns the exit
/// status and the full text of its stdout.
///
/// **Reuses `run_with_a_broken_pipe`'s spawn shape but needs none of its
/// ordering (final review, I2).** That function must drop a pipe's read end
/// before releasing the child from its own stdin block, because the break
/// and the child's read race each other, and getting the order wrong would
/// let the child observe a still-healthy stream. There is no such race
/// here: `Stdio::from(File::create(..))` hands the child a handle that was
/// never readable from the moment the process was created, so the child's
/// very first `stdin().read(..)` fails at spawn-adjacent OS call time
/// rather than blocking on anything this harness must later release. Both
/// the child's stdout and stderr stay healthy -- only stdin is broken --
/// so this can capture output the ordinary way with `Command::output`,
/// unlike `run_with_a_broken_pipe`'s manual drop-then-read sequencing.
///
/// **`File::create` is deliberately write-only, not read-write.** On
/// Windows this opens with `GENERIC_WRITE` alone, no `GENERIC_READ`, and a
/// spawned child inherits that same restricted handle rather than a
/// broader one -- confirmed by `io_read_stdin_write_only_run`'s own
/// measured result, not assumed from the API name.
#[cfg(windows)]
fn run_with_a_write_only_stdin(path: &std::path::Path) -> (std::process::ExitStatus, String) {
    use std::process::Stdio;

    let stdin_path = std::env::temp_dir().join("nova_io_read_stdin_write_only_probe.txt");
    let stdin_file = std::fs::File::create(&stdin_path)
        .expect("create a write-only file to hand the child as its stdin");

    let output = std::process::Command::new(assert_cmd::cargo::cargo_bin("nova"))
        .arg("run")
        .arg(path)
        .stdin(Stdio::from(stdin_file))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("run nova with a write-only stdin");

    // Best-effort: a leaked probe file does not affect correctness, and this
    // harness has no child-process failure path that would leave the file
    // locked on Windows the way a still-open handle would.
    let _ = std::fs::remove_file(&stdin_path);

    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    (output.status, stdout)
}

/// `read_stdin`'s `Err` branch (`std/io/lib.nova`), reached deterministically
/// through a write-only stdin handle rather than argued to be hard to reach.
///
/// **Closes the gap the branch's own records called "plausibly genuinely
/// hard" (final review, I2).** That characterisation was true only of the
/// broken-pipe *technique*: closing a stdin pipe yields ordinary EOF
/// (`Ok(0)`), a path `read_stdin` already handles as success. A write-only
/// handle is a different failure shape entirely -- the read never reaches
/// EOF because it never reaches "empty"; it is refused before any bytes are
/// considered, and this fixture is what proves that mechanically rather
/// than by argument. This is not merely as deterministic as the
/// broken-pipe fixtures above: it is *more* so, because there is no
/// ordering to get right and no race window at all -- the handle is
/// write-only from the instant the child exists, not made so by an action
/// this harness takes after spawning.
///
/// **Measured on this host, not assumed:** deleting `read_stdin`'s status
/// check (`std/io/lib.nova`, replacing its `Err(IoError { .. })` arm with
/// `Ok(fs_take_bytes())`, the exact mutation the design named) turned this
/// test's output from `read_stdin: err PermissionDenied` into
/// `read_stdin: unexpectedly ok, len 0`, failing the assertion below with
/// that exact string in the panic message. Reverting the mutation restored
/// a pass. This doc comment describes what was actually run against this
/// fixture, in that order, not a plan for what the mutation ought to do.
///
/// `#[cfg(windows)]` in the same style as `run_with_a_broken_pipe`'s two
/// fixtures -- measured on Windows, reasoned rather than measured on Unix,
/// because no Unix host is available here -- but for a different mechanism,
/// not the same one: those two gate on `SIGPIPE`/`EPIPE` for a *write* to a
/// broken pipe, while this technique is a *read*. Rust's own `unix.rs`
/// shows `StdinRaw::read` wrapped in `handle_ebadf(.., || Ok(0))`, and a
/// file opened `O_WRONLY` is textbook-POSIX `EBADF` on a `read()` call --
/// so on Unix this exact technique would fold to `Ok(0)`, the identical
/// value correct code already returns for real EOF. **That is a reason to
/// expect this specific technique gives no signal there, not merely an
/// unmeasured one**: unlike `run_with_a_broken_pipe`'s mechanism, which is
/// reasoned to also *work* on Unix, this one is reasoned to *fold away* on
/// Unix, which is why this fixture does not attempt a non-Windows variant
/// rather than shipping one untested.
#[cfg(windows)]
#[test]
fn io_read_stdin_write_only_run() {
    let (status, stdout) = run_with_a_write_only_stdin(
        &repo_root().join("tests/runtime/io_read_stdin_write_only.nova"),
    );
    assert!(status.success(), "the child process itself must exit 0");
    assert!(
        stdout.contains("read_stdin: err PermissionDenied"),
        "a write-only stdin handle must make read_stdin's intrinsic report \
         PermissionDenied, not silently succeed on an empty read: {stdout:?}"
    );
}

// === Task 3 (file-open-openoptions increment): `std/fs`'s `File`,
// `OpenOptions`, `open`, and `impl Read`/`Write for File` ===

/// Open for writing, write, flush, close, reopen for appending, write, reopen
/// for reading, `read_to_end`, close — then reopen for writing once more to
/// prove that leg truncates what the append left. Exercises `impl Write for
/// File` (`write` and `flush`), `impl Read for File` (through `read_to_end`'s
/// default body in `std/io`, over `File`'s own `read`), the inherent
/// `File::close`, and all three `OpenOptions` constructors, following
/// `fs_bytes_roundtrip_run`'s shape. See the fixture's own doc comment for why
/// the append, re-truncate and flush legs are there — each kills a mutation
/// that survived the whole suite before them.
#[test]
fn file_roundtrip_run() {
    let tmp = unique_temp_dir("nova-file-roundtrip");
    let expected = std::fs::read_to_string(repo_root().join("tests/runtime/file_roundtrip.stdout"))
        .expect("expected-output fixture exists")
        .replace("\r\n", "\n");
    nova()
        .arg("run")
        .arg(repo_root().join("tests/runtime/file_roundtrip.nova"))
        .env("TMPDIR", &tmp)
        .env("TMP", &tmp)
        .env("TEMP", &tmp)
        .assert()
        .success()
        .stdout(expected);
    let _ = std::fs::remove_dir_all(&tmp);
}

/// The increment's core resource-lifetime behaviour: `close` is idempotent,
/// reading, writing or flushing a handle this fixture itself closed is an
/// ordinary `IoError` of kind `Other` rather than a panic, and a Nova program
/// can forge a `File` naming no file this module ever opened (`fd` is not
/// privacy-enforced) and get the exact same safe treatment. See
/// `tests/runtime/file_lifetime.nova`'s own doc comment for why `open_or_die`
/// is fixture-local glue rather than `std/fs` surface, and why it opens
/// read+write rather than through `OpenOptions::writing()`.
#[test]
fn file_lifetime_run() {
    let tmp = unique_temp_dir("nova-file-lifetime");
    let expected = std::fs::read_to_string(repo_root().join("tests/runtime/file_lifetime.stdout"))
        .expect("expected-output fixture exists")
        .replace("\r\n", "\n");
    nova()
        .arg("run")
        .arg(repo_root().join("tests/runtime/file_lifetime.nova"))
        .env("TMPDIR", &tmp)
        .env("TMP", &tmp)
        .env("TEMP", &tmp)
        .assert()
        .success()
        .stdout(expected);
    let _ = std::fs::remove_dir_all(&tmp);
}

/// `create_new` against an existing path, and a path under a missing parent
/// directory, measured `AlreadyExists`/`NotFound` on this host -- consistent
/// with `std/io/lib.nova`'s own doc comment, which already lists
/// `AlreadyExists` and `NotFound` among the kinds a real OS condition pins,
/// with no platform qualifier attached to either the way it attaches one to
/// `PermissionDenied` (see `file_open_dir_run` below).
///
/// **Not platform-gated, unlike `file_open_dir_run`.** This fixture used to
/// bundle a third, genuinely platform-divergent check (opening a directory
/// for reading) into the same golden `.stdout`, which meant the whole
/// fixture -- these two portable checks included -- ran on Windows only.
/// Review (fix round 1, Important 1) found that was also the *only* thing
/// backing up two of the five mandated mutations (`open`'s status check
/// deleted; `create_new` hardcoded false), since `open`/`OpenOptions` are
/// new this task and no pre-existing test can catch a regression in them
/// the way `io_streams_run` backs up the shared decoder. Moving the
/// directory check to its own fixture (`file_open_dir.nova`,
/// `file_open_dir_run` below) is what lets these two run, and those two
/// mutations be caught, on every platform this project targets.
#[test]
fn file_errors_run() {
    let tmp = unique_temp_dir("nova-file-errors");
    let expected = std::fs::read_to_string(repo_root().join("tests/runtime/file_errors.stdout"))
        .expect("expected-output fixture exists")
        .replace("\r\n", "\n");
    nova()
        .arg("run")
        .arg(repo_root().join("tests/runtime/file_errors.nova"))
        .env("TMPDIR", &tmp)
        .env("TMP", &tmp)
        .env("TEMP", &tmp)
        .assert()
        .success()
        .stdout(expected);
    let _ = std::fs::remove_dir_all(&tmp);
}

/// Opening a directory for reading, split out of `file_errors_run` above
/// (fix round 1, Important 1) because it is the one genuinely
/// platform-divergent check among that fixture's original three --
/// measured `PermissionDenied` on this host, consistent with
/// `std/io/lib.nova`'s own doc comment's "(Windows only) `PermissionDenied`"
/// note. **Reasoned, not measured, since no POSIX host is available in this
/// environment:** POSIX `open(2)` permits read-only access to a directory
/// (needed for e.g. reading it back by fd), so this exact call would
/// plausibly *succeed* on Linux/macOS rather than fail, with a failure only
/// reachable from an actual `read()` afterward -- reporting `IsADirectory`,
/// a kind `crates/nova-runtime/src/fs.rs`'s `fail` has no arm for and
/// therefore maps to `Other`, not `PermissionDenied`. This fixture now
/// holds only this one check, so gating it -- rather than a fixture with
/// portable checks bundled in -- costs nothing on non-Windows platforms:
/// the same shape `fs_permission_denied_run` already uses for its own
/// directory-flavoured, Windows-only check.
#[cfg(windows)]
#[test]
fn file_open_dir_run() {
    let tmp = unique_temp_dir("nova-file-open-dir");
    let expected = std::fs::read_to_string(repo_root().join("tests/runtime/file_open_dir.stdout"))
        .expect("expected-output fixture exists")
        .replace("\r\n", "\n");
    nova()
        .arg("run")
        .arg(repo_root().join("tests/runtime/file_open_dir.nova"))
        .env("TMPDIR", &tmp)
        .env("TMP", &tmp)
        .env("TEMP", &tmp)
        .assert()
        .success()
        .stdout(expected);
    let _ = std::fs::remove_dir_all(&tmp);
}

// === Task 6 (I/O poller and std/net increment): std/net fixtures ===
//
// Every fixture below *except* `net_listener_accept_run` needs a real TCP
// peer at a port it cannot know in advance (the OS hands out an ephemeral
// one), so each of those tests binds one itself and hands the Nova program
// the port number through a file: `EchoServer` (and
// `write_refused_port_file`, for the one fixture that wants nothing
// listening) writes it, and the paired `.nova` fixture reads it back with
// `fs::read_to_string`.
//
// `net_listener_accept_run` is the exception and needs none of that
// machinery: both ends of its connection are Nova, in two tasks of one
// process, so it binds `127.0.0.1:0` itself and passes the kernel's choice
// to a spawned client as an ordinary argument. The paragraph below therefore
// does not describe it either -- with no port file, it has no port-file path
// to collide on.
//
// **That file's path is derived from the fixture's own name alone, not this
// process** — unlike `unique_temp_dir`, which folds in `std::process::id()`
// specifically to keep concurrent runs of this same binary from colliding.
// Two concurrent `cargo test` invocations racing the *same* test would
// collide on this path — the identical latent hazard `write_test_project`'s
// fixed `unique_name`-only directory already carries (see that function's
// own comment, above). A fixed port would be flakier (a stale process, or a
// genuinely concurrent suite run, could already hold it) and generating the
// `.nova` source itself would break the static-file-plus-golden convention
// every other runtime fixture in this file follows, so this hazard is
// accepted rather than engineered around, the same call this file already
// makes for `write_test_project`. None of these fixtures touch `TMPDIR`/
// `TMP`/`TEMP`: Nova's `temp_dir()` and this harness's own
// `std::env::temp_dir()` calls therefore resolve to the identical, real OS
// temp directory on both sides with no override needed.

/// A one-shot loopback TCP server for a `std/net` fixture: binds
/// `127.0.0.1:0`, writes the ephemeral port to a well-known path the paired
/// `.nova` fixture reads with `fs::read_to_string`, then accepts exactly one
/// connection and echoes back whatever it reads, byte for byte, until the
/// peer's own half closes.
///
/// `delay_ms`, when non-zero, sleeps once right after `accept` and before
/// the first read — long enough that a fixture's `read`/`read_timeout`
/// against this connection is guaranteed to find nothing yet and genuinely
/// suspend, rather than racing real loopback latency (which is normally fast
/// enough that a plain round trip could occasionally resolve before a
/// sibling task ever got a turn). `net_interleave_run` and `net_timeout_run`
/// both depend on this; `net_roundtrip_run` and `net_lifetime_run` pass `0`.
///
/// The accepting thread polls a non-blocking listener against `shutdown`
/// rather than blocking in `accept` forever, so a fixture that never
/// connects at all (a failing test, before it ever reaches `connect`) still
/// lets `Drop` join the thread and end the process cleanly instead of
/// leaking one parked in a blocking `accept` for the rest of this test
/// binary's life.
struct EchoServer {
    port_path: std::path::PathBuf,
    shutdown: std::sync::Arc<std::sync::atomic::AtomicBool>,
    handle: Option<std::thread::JoinHandle<()>>,
}

impl EchoServer {
    fn start(label: &str, delay_ms: u64) -> EchoServer {
        let listener =
            std::net::TcpListener::bind("127.0.0.1:0").expect("bind an ephemeral loopback port");
        listener
            .set_nonblocking(true)
            .expect("listener supports non-blocking accept");
        let port = listener
            .local_addr()
            .expect("bound listener has a local address")
            .port();
        let port_path = std::env::temp_dir().join(format!("nova_{label}_port.txt"));
        std::fs::write(&port_path, port.to_string())
            .expect("write the port file the paired fixture reads");

        let shutdown = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let shutdown_for_thread = std::sync::Arc::clone(&shutdown);
        let handle = std::thread::spawn(move || {
            let mut stream = loop {
                match listener.accept() {
                    Ok((stream, _)) => break stream,
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        if shutdown_for_thread.load(std::sync::atomic::Ordering::SeqCst) {
                            return;
                        }
                        std::thread::sleep(std::time::Duration::from_millis(5));
                    }
                    Err(_) => return,
                }
            };
            // Reverts to blocking for this thread's own reads/writes below —
            // this thread has nothing else to do while serving one
            // connection, so there is no reason to poll it too.
            if stream.set_nonblocking(false).is_err() {
                return;
            }
            if delay_ms > 0 {
                std::thread::sleep(std::time::Duration::from_millis(delay_ms));
            }
            let mut buf = [0u8; 4096];
            loop {
                match std::io::Read::read(&mut stream, &mut buf) {
                    Ok(0) | Err(_) => return,
                    Ok(n) => {
                        if std::io::Write::write_all(&mut stream, &buf[..n]).is_err() {
                            return;
                        }
                    }
                }
            }
        });

        EchoServer {
            port_path,
            shutdown,
            handle: Some(handle),
        }
    }
}

impl Drop for EchoServer {
    fn drop(&mut self) {
        self.shutdown
            .store(true, std::sync::atomic::Ordering::SeqCst);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
        let _ = std::fs::remove_file(&self.port_path);
    }
}

/// A port on `127.0.0.1` with nothing listening: reserve an ephemeral one,
/// then immediately drop the listener so the OS releases it again before any
/// fixture connects. `net_refused_run`'s loopback `connect` to this port
/// gets an OS-level refusal — an immediate RST, needing no network round
/// trip (`crates/nova-runtime/src/net.rs`'s own module doc comment) — rather
/// than a hardcoded high port number some other process on the host might
/// genuinely be listening on.
///
/// **Accepted, unengineered race:** between the `drop` below and the fixture's
/// own `connect`, nothing stops the OS handing that just-released port to some
/// other process, which would turn the expected refusal into a connection and
/// fail `net_refused_run`. Measured at 0 occurrences in 40 consecutive runs on
/// this project's own hosts, and closing it would mean holding a port open to
/// prove nothing is listening on it — so it is recorded here rather than
/// engineered around.
fn write_refused_port_file(label: &str) -> std::path::PathBuf {
    let listener =
        std::net::TcpListener::bind("127.0.0.1:0").expect("bind an ephemeral loopback port");
    let port = listener
        .local_addr()
        .expect("bound listener has a local address")
        .port();
    drop(listener);
    let port_path = std::env::temp_dir().join(format!("nova_{label}_port.txt"));
    std::fs::write(&port_path, port.to_string())
        .expect("write the port file the paired fixture reads");
    port_path
}

/// `connect`, `impl Write for TcpStream`, `impl Read for TcpStream`, and
/// `TcpStream::close` against a real loopback echo server. See the
/// fixture's own header for why this alone cannot tell a real poller from a
/// blocking one — `net_interleave_run`, below, is the test that can.
#[test]
fn net_roundtrip_run() {
    let _server = EchoServer::start("net_roundtrip", 0);
    let expected = std::fs::read_to_string(repo_root().join("tests/runtime/net_roundtrip.stdout"))
        .expect("expected-output fixture exists")
        .replace("\r\n", "\n");
    nova()
        .arg("run")
        .arg(repo_root().join("tests/runtime/net_roundtrip.nova"))
        .assert()
        .success()
        .stdout(expected);
}

/// The fixture that decides the increment: asserts that a sibling task's own
/// output lands between a socket task's "wrote" and "read" lines, which only
/// a real, non-blocking poller can produce. See
/// `tests/runtime/net_interleave.nova`'s own header for the full reasoning,
/// including why `connect` runs before either task is spawned.
///
/// The paired server sleeps 150ms after accepting before echoing anything —
/// long enough for the counter task's three `yield_now` steps (each costing
/// no real wall-clock time at all) to run to completion while the reader
/// waits, on any machine this suite runs on, and short enough not to slow
/// the suite down.
#[test]
fn net_interleave_run() {
    let _server = EchoServer::start("net_interleave", 150);
    let expected = std::fs::read_to_string(repo_root().join("tests/runtime/net_interleave.stdout"))
        .expect("expected-output fixture exists")
        .replace("\r\n", "\n");
    nova()
        .arg("run")
        .arg(repo_root().join("tests/runtime/net_interleave.nova"))
        .assert()
        .success()
        .stdout(expected);
}

/// `TcpStream::read_timeout` in both directions: too-short and comfortably
/// long, against one real connection. See `tests/runtime/net_timeout.nova`'s
/// own header for why both directions matter.
///
/// The paired server sleeps 250ms after accepting before echoing anything —
/// comfortably longer than the fixture's own 80ms short deadline and
/// comfortably shorter than its 3000ms long one.
#[test]
fn net_timeout_run() {
    let _server = EchoServer::start("net_timeout", 250);
    let expected = std::fs::read_to_string(repo_root().join("tests/runtime/net_timeout.stdout"))
        .expect("expected-output fixture exists")
        .replace("\r\n", "\n");
    nova()
        .arg("run")
        .arg(repo_root().join("tests/runtime/net_timeout.nova"))
        .assert()
        .success()
        .stdout(expected);
}

/// `connect` against a loopback port nothing is listening on —
/// `ConnectionRefused`'s first producer end to end. See
/// `tests/runtime/net_refused.nova`'s own header.
#[test]
fn net_refused_run() {
    let port_path = write_refused_port_file("net_refused");
    let expected = std::fs::read_to_string(repo_root().join("tests/runtime/net_refused.stdout"))
        .expect("expected-output fixture exists")
        .replace("\r\n", "\n");
    nova()
        .arg("run")
        .arg(repo_root().join("tests/runtime/net_refused.nova"))
        .assert()
        .success()
        .stdout(expected);
    let _ = std::fs::remove_file(&port_path);
}

/// The increment's core resource-lifetime behaviour for `TcpStream`: `close`
/// is idempotent, reading, writing or timing out a handle this fixture
/// itself closed is an ordinary `IoError` of kind `Other` rather than a
/// panic, `flush` stays a no-op even then, and a Nova program can forge a
/// `TcpStream` naming no connection this module ever opened (`fd` is not
/// privacy-enforced) and get the exact same safe treatment. See
/// `tests/runtime/net_lifetime.nova`'s own header, which follows
/// `file_lifetime.nova`'s shape exactly.
#[test]
fn net_lifetime_run() {
    let _server = EchoServer::start("net_lifetime", 0);
    let expected = std::fs::read_to_string(repo_root().join("tests/runtime/net_lifetime.stdout"))
        .expect("expected-output fixture exists")
        .replace("\r\n", "\n");
    nova()
        .arg("run")
        .arg(repo_root().join("tests/runtime/net_lifetime.nova"))
        .assert()
        .success()
        .stdout(expected);
}

/// `bind`, `local_port`, `accept` and both `close`s, end to end, with a real
/// connection crossing the listener. **The only `std/net` fixture with no
/// port file and no Rust peer**: both ends are Nova, in two tasks of one
/// process, so the listener binds `127.0.0.1:0` itself and hands the
/// kernel's choice to a spawned client task as a plain `spawn` argument.
///
/// This said "**the only `std/net` fixture that needs no `EchoServer`**",
/// which `net_refused_run` falsifies — its entire point is that nothing is
/// listening, so it touches no `EchoServer` either, and the section header
/// above this group already names it for exactly that reason. The property
/// stated above is the one that is genuinely exclusive, and it survives the
/// group growing: `net_refused_run` does write a port file, so this stays
/// the one `net_*` fixture needing neither a port file nor a Rust peer.
///
/// Also `local_port`'s only runtime fixture, which is why it exists as much
/// as the round trip is: `RtFunc::NetClose` and `RtFunc::NetLocalPort` are
/// indistinguishable in `RtFunc::signature`, and this pins the one direction
/// of that swap nothing else could. See the fixture's own header.
#[test]
fn net_listener_accept_run() {
    let expected =
        std::fs::read_to_string(repo_root().join("tests/runtime/net_listener_accept.stdout"))
            .expect("expected-output fixture exists")
            .replace("\r\n", "\n");
    nova()
        .arg("run")
        .arg(repo_root().join("tests/runtime/net_listener_accept.nova"))
        .assert()
        .success()
        .stdout(expected);
}

/// `Duration`'s three constructors and two accessors, over fixed inputs with
/// no real clock involved. See `tests/runtime/time_conversions.nova`'s own
/// header for why the fourth line (`from_micros(1).as_millis()` is `0`) is
/// pinned deliberately rather than accidental.
#[test]
fn time_conversions_run() {
    let expected =
        std::fs::read_to_string(repo_root().join("tests/runtime/time_conversions.stdout"))
            .expect("expected-output fixture exists")
            .replace("\r\n", "\n");
    nova()
        .arg("run")
        .arg(repo_root().join("tests/runtime/time_conversions.nova"))
        .assert()
        .success()
        .stdout(expected);
}

/// Four printed lines: `Duration`'s three constructors -- `from_secs`,
/// `from_millis`, `from_micros` -- each given one argument past its upper
/// cap, which an unclamped constructor would wrap into a negative `Int`,
/// plus `from_secs` given one argument past its lower cap, on the negative
/// side, which an unclamped constructor would wrap the other way -- into a
/// positive `Int` byte-identical to the correctly-clamped result on the
/// first line. That second direction is the more insidious failure: an
/// unclamped negative wraps to roughly +292 years, so `sleep` of it would
/// park for centuries where it should wake immediately. A fifth line covers
/// `Instant::duration_since` saturating at zero when the arguments are the
/// wrong way round. See `tests/runtime/time_saturation.nova`'s own header.
#[test]
fn time_saturation_run() {
    let expected =
        std::fs::read_to_string(repo_root().join("tests/runtime/time_saturation.stdout"))
            .expect("expected-output fixture exists")
            .replace("\r\n", "\n");
    nova()
        .arg("run")
        .arg(repo_root().join("tests/runtime/time_saturation.nova"))
        .assert()
        .success()
        .stdout(expected);
}

/// `Instant::now`/`elapsed`/`duration_since` against the real monotonic
/// clock. A real clock varies, so the fixture asserts a range rather than a
/// value; see `tests/runtime/time_elapsed.nova`'s own header. The fixture
/// sleeps 5ms and requires the measured elapsed time to cover it -- that
/// sleep is the only thing in the whole test suite that would fail against
/// a frozen or mis-wired clock, so it is load-bearing, not incidental, and
/// must survive any future trim of the suite's running time.
#[test]
fn time_elapsed_run() {
    let expected = std::fs::read_to_string(repo_root().join("tests/runtime/time_elapsed.stdout"))
        .expect("expected-output fixture exists")
        .replace("\r\n", "\n");
    nova()
        .arg("run")
        .arg(repo_root().join("tests/runtime/time_elapsed.nova"))
        .assert()
        .success()
        .stdout(expected);
}

/// ISO-8601 rendering over fixed nanosecond inputs, so the assertion is on
/// known values rather than on a live clock. The six rows pin the epoch, two
/// leap days, the 2100 non-leap-year, a year boundary, and zero-padding.
#[test]
fn system_time_iso8601_run() {
    let expected =
        std::fs::read_to_string(repo_root().join("tests/runtime/system_time_iso8601.stdout"))
            .expect("expected-output fixture exists")
            .replace("\r\n", "\n");
    nova()
        .arg("run")
        .arg(repo_root().join("tests/runtime/system_time_iso8601.nova"))
        .assert()
        .success()
        .stdout(expected);
}

/// The live clock, asserted on shape rather than value: the year must be a
/// plausible four digits and the rendering must be exactly 24 characters
/// ending in `Z`. A process-relative reading would render as 1970 and fail
/// the first check.
#[test]
fn system_time_now_run() {
    let expected =
        std::fs::read_to_string(repo_root().join("tests/runtime/system_time_now.stdout"))
            .expect("expected-output fixture exists")
            .replace("\r\n", "\n");
    nova()
        .arg("run")
        .arg(repo_root().join("tests/runtime/system_time_now.nova"))
        .assert()
        .success()
        .stdout(expected);
}

/// `timeout` when the inner future completes well inside the deadline:
/// `Ok`'s branch runs, not `Err`'s. Prints which branch ran, not an
/// ordering, so a unit error in the comparison cannot pass by accident.
#[test]
fn timeout_ok_run() {
    let expected = std::fs::read_to_string(repo_root().join("tests/runtime/timeout_ok.stdout"))
        .expect("expected-output fixture exists")
        .replace("\r\n", "\n");
    nova()
        .arg("run")
        .arg(repo_root().join("tests/runtime/timeout_ok.nova"))
        .assert()
        .success()
        .stdout(expected);
}

/// `timeout` when the deadline elapses well before the inner future would
/// finish: `Err`'s branch runs, not `Ok`'s. The inner future is never
/// spawned, so abandoning it costs nothing further once the deadline fires.
#[test]
fn timeout_elapsed_run() {
    let expected =
        std::fs::read_to_string(repo_root().join("tests/runtime/timeout_elapsed.stdout"))
            .expect("expected-output fixture exists")
            .replace("\r\n", "\n");
    nova()
        .arg("run")
        .arg(repo_root().join("tests/runtime/timeout_elapsed.nova"))
        .assert()
        .success()
        .stdout(expected);
}

/// `timeout` returning the inner future's value, not the combinator's status.
///
/// The one fixture that distinguishes those -- see `tests/runtime/
/// timeout_value.nova`'s own header for why a "wrong slot" mutation can't be
/// demonstrated at the Nova-source level (the type checker rejects it) and
/// what this fixture actually guards instead. Measured, not reasoned: a
/// slot-layout bug in `poll_timeout` (writing the combinator's status into the
/// *inner's* output slot too) fails **this test and nothing else in the
/// workspace**. A `nova-mir` `Builtin::TaskOutput` lowering bug also makes this
/// fixture print `0` instead of `42`, but so does every other test that reads a
/// value through that lowering -- eight failures in this file when measured, and
/// not a stable number to quote (three measurements gave 18, 7 and 8) -- so this
/// fixture is not what guards that; an earlier version of the header claimed
/// otherwise for both.
#[test]
fn timeout_value_run() {
    let expected = std::fs::read_to_string(repo_root().join("tests/runtime/timeout_value.stdout"))
        .expect("expected-output fixture exists")
        .replace("\r\n", "\n");
    nova()
        .arg("run")
        .arg(repo_root().join("tests/runtime/timeout_value.nova"))
        .assert()
        .success()
        .stdout(expected);
}

/// `timeout` wrapping a `JoinHandle::join()` future on a task that finishes
/// well inside the deadline: one of the Task+Deadline pair that aborted the
/// process before Task 1 widened the executor's park set.
#[test]
fn timeout_join_ok_run() {
    let expected =
        std::fs::read_to_string(repo_root().join("tests/runtime/timeout_join_ok.stdout"))
            .expect("expected-output fixture exists")
            .replace("\r\n", "\n");
    nova()
        .arg("run")
        .arg(repo_root().join("tests/runtime/timeout_join_ok.nova"))
        .assert()
        .success()
        .stdout(expected);
}

/// `timeout` wrapping a `JoinHandle::join()` future on a task that sleeps
/// past the deadline: the other half of the Task+Deadline pair that aborted
/// the process before Task 1 widened the executor's park set. The spawned
/// task keeps running after the join is abandoned, so this fixture's own
/// sleep is kept short rather than dramatic.
#[test]
fn timeout_join_elapsed_run() {
    let expected =
        std::fs::read_to_string(repo_root().join("tests/runtime/timeout_join_elapsed.stdout"))
            .expect("expected-output fixture exists")
            .replace("\r\n", "\n");
    nova()
        .arg("run")
        .arg(repo_root().join("tests/runtime/timeout_join_elapsed.nova"))
        .assert()
        .success()
        .stdout(expected);
}

/// An elapsed `timeout` over a join, followed by a second `join().await` in
/// the same task: the abandoned inner's `Wait::Task` must not survive into the
/// next suspension of that same task poll.
///
/// Before `poll_timeout` restored `PENDING_PARK`, this aborted the process
/// with "two parks staged in one poll" and an exit code of 127 -- from
/// ordinary Nova source. `.success()` plus the golden is therefore the whole
/// assertion: the abort produced neither.
#[test]
fn timeout_elapsed_then_join_run() {
    let expected =
        std::fs::read_to_string(repo_root().join("tests/runtime/timeout_elapsed_then_join.stdout"))
            .expect("expected-output fixture exists")
            .replace("\r\n", "\n");
    nova()
        .arg("run")
        .arg(repo_root().join("tests/runtime/timeout_elapsed_then_join.nova"))
        .assert()
        .success()
        .stdout(expected);
}

/// Two sequential elapsing `timeout`s over joins -- the same abandonment
/// leftover, crossed by a second combinator rather than by a bare `join`, and
/// the idiomatic shape (a retry loop, or two timed joins in sequence).
#[test]
fn timeout_elapsed_twice_run() {
    let expected =
        std::fs::read_to_string(repo_root().join("tests/runtime/timeout_elapsed_twice.stdout"))
            .expect("expected-output fixture exists")
            .replace("\r\n", "\n");
    nova()
        .arg("run")
        .arg(repo_root().join("tests/runtime/timeout_elapsed_twice.nova"))
        .assert()
        .success()
        .stdout(expected);
}

/// No `init` call anywhere: the default threshold is `Info`, so `Trace` and
/// `Debug` are dropped while `Info`, `Warn` and `Error` reach stderr.
/// `Info` at threshold `Info` is what fails if `<` becomes `<=` -- though not
/// uniquely. Every fixture here logs a message at its own threshold, so all
/// five catch that mutation; this one is simply the cheapest to read. An
/// earlier version of this comment claimed it was the only one, which was
/// measured false by running the isolated mutation against all five.
///
/// The stdout golden is asserted as well, and pins what the stderr assertion
/// cannot: that the program ran to *completion*. Without it, a logger that
/// emitted three correct lines and then died would still pass.
#[test]
fn log_default_level_run() {
    let out = nova()
        .arg("run")
        .arg(repo_root().join("tests/runtime/log_default_level.nova"))
        .assert()
        .success();
    let stderr = String::from_utf8_lossy(&out.get_output().stderr).replace("\r\n", "\n");
    let levels: Vec<&str> = stderr
        .lines()
        .map(|l| l.split_once(' ').expect("line starts with a timestamp").1)
        .collect();
    assert_eq!(
        levels,
        vec!["INFO yes-info", "WARN yes-warn", "ERROR yes-error"]
    );
    let expected =
        std::fs::read_to_string(repo_root().join("tests/runtime/log_default_level.stdout"))
            .expect("expected-output fixture exists")
            .replace("\r\n", "\n");
    let stdout = String::from_utf8_lossy(&out.get_output().stdout).replace("\r\n", "\n");
    assert_eq!(stdout, expected);
}

/// `init_with` at `Warn` raises the threshold above the default `Info`:
/// `info` is silenced while `warn` and `error` still reach stderr.
#[test]
fn log_init_with_threshold_run() {
    let expected =
        std::fs::read_to_string(repo_root().join("tests/runtime/log_init_with_threshold.stdout"))
            .expect("expected-output fixture exists")
            .replace("\r\n", "\n");
    let out = nova()
        .arg("run")
        .arg(repo_root().join("tests/runtime/log_init_with_threshold.nova"))
        .assert()
        .success()
        .stdout(expected);
    let stderr = String::from_utf8_lossy(&out.get_output().stderr).replace("\r\n", "\n");
    let levels: Vec<&str> = stderr
        .lines()
        .map(|l| l.split_once(' ').expect("line starts with a timestamp").1)
        .collect();
    assert_eq!(levels, vec!["WARN kept-warn", "ERROR kept-error"]);
}

/// Logging once at the default threshold and then calling `init_with` to
/// raise it: the last write wins, so the first stderr line is at `Info`
/// (before the reconfigure) and the second is at `Error` (after). This is
/// the fixture that fails if a getter reads a hardcoded default instead of
/// the runtime's cell.
#[test]
fn log_reconfigure_after_logging_run() {
    let expected = std::fs::read_to_string(
        repo_root().join("tests/runtime/log_reconfigure_after_logging.stdout"),
    )
    .expect("expected-output fixture exists")
    .replace("\r\n", "\n");
    let out = nova()
        .arg("run")
        .arg(repo_root().join("tests/runtime/log_reconfigure_after_logging.nova"))
        .assert()
        .success()
        .stdout(expected);
    let stderr = String::from_utf8_lossy(&out.get_output().stderr).replace("\r\n", "\n");
    let levels: Vec<&str> = stderr
        .lines()
        .map(|l| l.split_once(' ').expect("line starts with a timestamp").1)
        .collect();
    assert_eq!(levels, vec!["INFO before", "ERROR after-kept"]);
}

/// `output: Stdout` moves the line off stderr entirely: this is the fixture
/// that fails if `eprintln` and `println` are swapped inside `emit`.
#[test]
fn log_stdout_output_run() {
    let out = nova()
        .arg("run")
        .arg(repo_root().join("tests/runtime/log_stdout_output.nova"))
        .assert()
        .success();
    let output = out.get_output();
    let stderr = String::from_utf8_lossy(&output.stderr).replace("\r\n", "\n");
    assert_eq!(stderr, "", "stderr should be empty: {stderr}");
    let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines.len(), 1, "stdout: {stdout}");
    let (_, rest) = lines[0]
        .split_once(' ')
        .expect("line starts with a timestamp");
    assert_eq!(rest, "INFO on-stdout");
}

/// All five labels, at threshold `Trace` so every one emits, routed to
/// stdout so a live clock doesn't need stderr capture too. This pins the
/// five label strings and their emit order at a threshold that admits every
/// level. It does *not* catch `Warn` and `Error` swapping in
/// `LogLevel::to_int` -- measured: at threshold `Trace` every level's
/// `to_int()` clears the filter regardless of numbering, and `label()`
/// (`std/log/lib.nova`) never consults `to_int()`, so the output is
/// byte-identical under that swap. `log_init_with_threshold_run` is the
/// fixture that catches it: its threshold is `Warn`, so the swap pushes
/// `Error` below it and `"ERROR kept-error"` disappears.
#[test]
fn log_level_labels_run() {
    let out = nova()
        .arg("run")
        .arg(repo_root().join("tests/runtime/log_level_labels.nova"))
        .assert()
        .success();
    let output = out.get_output();
    let stderr = String::from_utf8_lossy(&output.stderr).replace("\r\n", "\n");
    assert_eq!(stderr, "", "stderr should be empty: {stderr}");
    let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
    let levels: Vec<&str> = stdout
        .lines()
        .map(|l| l.split_once(' ').expect("line starts with a timestamp").1)
        .collect();
    assert_eq!(
        levels,
        vec!["TRACE a", "DEBUG b", "INFO c", "WARN d", "ERROR e"]
    );
}

/// `Log::init()` must install exactly the runtime's default -- `Info`
/// threshold, stderr -- not whatever `init_with` left behind. The fixture
/// first installs a configuration wrong in *both* fields (`Error` threshold,
/// `Stdout` destination); a correct `init()` overwrites it, so `debug` stays
/// silenced and `info` reaches stderr. This is the fixture that pins
/// `std/log/lib.nova`'s `init()` literal against the runtime's `DEFAULT`
/// (`crates/nova-runtime/src/log.rs`): a wrong level drops the `info` line
/// entirely, and a wrong destination moves it to stdout instead.
#[test]
fn log_init_resets_to_default_run() {
    let out = nova()
        .arg("run")
        .arg(repo_root().join("tests/runtime/log_init_resets_to_default.nova"))
        .assert()
        .success();
    let output = out.get_output();
    let stderr = String::from_utf8_lossy(&output.stderr).replace("\r\n", "\n");
    let levels: Vec<&str> = stderr
        .lines()
        .map(|l| l.split_once(' ').expect("line starts with a timestamp").1)
        .collect();
    // Stdout is asserted *first*, deliberately, so a failure says which field
    // of `init()`'s default drifted rather than only that one did. The two
    // mutations this test exists to catch are otherwise indistinguishable: a
    // wrong level drops the line, a wrong destination moves it, and both leave
    // stderr empty. Checking stdout first separates them -- a misrouted line
    // shows up here as an unexpected extra line, while a dropped one leaves
    // stdout correct and fails the stderr assertion below.
    let expected = std::fs::read_to_string(
        repo_root().join("tests/runtime/log_init_resets_to_default.stdout"),
    )
    .expect("expected-output fixture exists")
    .replace("\r\n", "\n");
    let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
    assert_eq!(stdout, expected);
    assert_eq!(levels, vec!["INFO yes"]);
}

/// Zero-padding an Int: single digit, zero, a negative where the sign counts
/// toward the width, an over-wide value returned unpadded, and a negative
/// width clamped by the early return.
#[test]
fn fmt_int_pad_run() {
    let expected = std::fs::read_to_string(repo_root().join("tests/runtime/fmt_int_pad.stdout"))
        .expect("expected-output fixture exists")
        .replace("\r\n", "\n");
    nova()
        .arg("run")
        .arg(repo_root().join("tests/runtime/fmt_int_pad.nova"))
        .assert()
        .success()
        .stdout(expected);
}

/// Space-padding a String at a width above, equal to and below its length.
/// The brackets in the fixture make trailing padding visible in the golden.
#[test]
fn fmt_string_pad_run() {
    let expected = std::fs::read_to_string(repo_root().join("tests/runtime/fmt_string_pad.stdout"))
        .expect("expected-output fixture exists")
        .replace("\r\n", "\n");
    nova()
        .arg("run")
        .arg(repo_root().join("tests/runtime/fmt_string_pad.nova"))
        .assert()
        .success()
        .stdout(expected);
}

/// Fixed-place decimal rendering: two values whose default rendering is long
/// and imprecise, a zero-places truncation, and a trailing zero the value
/// does not itself carry.
#[test]
fn fmt_float_fixed_run() {
    let expected =
        std::fs::read_to_string(repo_root().join("tests/runtime/fmt_float_fixed.stdout"))
            .expect("expected-output fixture exists")
            .replace("\r\n", "\n");
    nova()
        .arg("run")
        .arg(repo_root().join("tests/runtime/fmt_float_fixed.nova"))
        .assert()
        .success()
        .stdout(expected);
}

/// Non-finite `Float` values (NaN, +/- infinity) rendering as Rust renders
/// them, and a negative `places` clamped to zero rather than panicking.
#[test]
fn fmt_float_edge_run() {
    let expected = std::fs::read_to_string(repo_root().join("tests/runtime/fmt_float_edge.stdout"))
        .expect("expected-output fixture exists")
        .replace("\r\n", "\n");
    nova()
        .arg("run")
        .arg(repo_root().join("tests/runtime/fmt_float_edge.nova"))
        .assert()
        .success()
        .stdout(expected);
}

/// One task, no contention: lock, read through the guard, release, and the
/// mutex is free again.
#[test]
fn sync_mutex_uncontended_run() {
    let expected =
        std::fs::read_to_string(repo_root().join("tests/runtime/sync_mutex_uncontended.stdout"))
            .expect("expected-output fixture exists")
            .replace("\r\n", "\n");
    nova()
        .arg("run")
        .arg(repo_root().join("tests/runtime/sync_mutex_uncontended.nova"))
        .assert()
        .success()
        .stdout(expected);
}

/// A second `try_lock` fails while the first guard is live and succeeds after
/// it is released.
#[test]
fn sync_mutex_try_lock_fails_when_held_run() {
    let expected = std::fs::read_to_string(
        repo_root().join("tests/runtime/sync_mutex_try_lock_fails_when_held.stdout"),
    )
    .expect("expected-output fixture exists")
    .replace("\r\n", "\n");
    nova()
        .arg("run")
        .arg(repo_root().join("tests/runtime/sync_mutex_try_lock_fails_when_held.nova"))
        .assert()
        .success()
        .stdout(expected);
}

/// `release` twice on the same guard is a no-op. Note what this fixture
/// cannot see: it releases twice with no intervening acquire, so it passes
/// even with `MutexGuard`'s `released` flag removed.
/// `sync_mutex_stale_guard_cannot_steal_run` is the one that covers that.
#[test]
fn sync_mutex_release_is_idempotent_run() {
    let expected = std::fs::read_to_string(
        repo_root().join("tests/runtime/sync_mutex_release_is_idempotent.stdout"),
    )
    .expect("expected-output fixture exists")
    .replace("\r\n", "\n");
    nova()
        .arg("run")
        .arg(repo_root().join("tests/runtime/sync_mutex_release_is_idempotent.nova"))
        .assert()
        .success()
        .stdout(expected);
}

/// Two tasks each read a shared counter, suspend, then write it back, under
/// the mutex. Serialised the answer is 2; interleaved it is 1, because both
/// read 0 and the second write clobbers the first. The `yield_now` inside the
/// critical section is what makes this test able to fail -- a section with no
/// suspension point is already protected by cooperative scheduling.
#[test]
fn sync_mutex_two_tasks_serialise_run() {
    let expected = std::fs::read_to_string(
        repo_root().join("tests/runtime/sync_mutex_two_tasks_serialise.stdout"),
    )
    .expect("expected-output fixture exists")
    .replace("\r\n", "\n");
    nova()
        .arg("run")
        .arg(repo_root().join("tests/runtime/sync_mutex_two_tasks_serialise.nova"))
        .assert()
        .success()
        .stdout(expected);
}

/// The same lost-update test over a bare `Mutex<Int>` rather than a record
/// wrapper, which is what `MutexGuard::set` is for: `get` copies an `Int` out,
/// so mutating the copy cannot reach the mutex. This fixture does not compile
/// without `set`.
#[test]
fn sync_mutex_int_set_serialises_run() {
    let expected = std::fs::read_to_string(
        repo_root().join("tests/runtime/sync_mutex_int_set_serialises.stdout"),
    )
    .expect("expected-output fixture exists")
    .replace("\r\n", "\n");
    nova()
        .arg("run")
        .arg(repo_root().join("tests/runtime/sync_mutex_int_set_serialises.nova"))
        .assert()
        .success()
        .stdout(expected);
}

/// A guard released once, then released again after another task has taken the
/// lock, must not free it. This is the case `sync_mutex_release_is_idempotent`
/// cannot see: it releases twice with no intervening acquire, so an
/// unconditional `self.owner.locked = false` passes it. `MutexGuard`'s
/// `released` flag is what makes the documented idempotence true here. Pins
/// `release`'s use of that flag only -- `set`'s use is pinned by
/// `sync_mutex_stale_guard_cannot_write_run` below. Depends on the executor's
/// FIFO ready queue to place the stale release inside the holder's critical
/// section; the fixture's own comment says so and why it matters.
#[test]
fn sync_mutex_stale_guard_cannot_steal_run() {
    let expected = std::fs::read_to_string(
        repo_root().join("tests/runtime/sync_mutex_stale_guard_cannot_steal.stdout"),
    )
    .expect("expected-output fixture exists")
    .replace("\r\n", "\n");
    nova()
        .arg("run")
        .arg(repo_root().join("tests/runtime/sync_mutex_stale_guard_cannot_steal.nova"))
        .assert()
        .success()
        .stdout(expected);
}

/// A stale guard's `set` must not reach the protected value while another task
/// is inside its critical section. This is `set`'s half of what `released`
/// closes, and the half that shipped uncovered: deleting `set`'s early return
/// failed no test in the suite until this fixture existed. Same FIFO ready-queue
/// dependency as the fixture above, and the same warning in its comment.
#[test]
fn sync_mutex_stale_guard_cannot_write_run() {
    let expected = std::fs::read_to_string(
        repo_root().join("tests/runtime/sync_mutex_stale_guard_cannot_write.stdout"),
    )
    .expect("expected-output fixture exists")
    .replace("\r\n", "\n");
    nova()
        .arg("run")
        .arg(repo_root().join("tests/runtime/sync_mutex_stale_guard_cannot_write.nova"))
        .assert()
        .success()
        .stdout(expected);
}

#[test]
fn channel_uncontended_run() {
    let expected =
        std::fs::read_to_string(repo_root().join("tests/runtime/channel_uncontended.stdout"))
            .expect("expected-output fixture exists")
            .replace("\r\n", "\n");
    nova()
        .arg("run")
        .arg(repo_root().join("tests/runtime/channel_uncontended.nova"))
        .assert()
        .success()
        .stdout(expected);
}

#[test]
fn channel_full_refuses_run() {
    let expected =
        std::fs::read_to_string(repo_root().join("tests/runtime/channel_full_refuses.stdout"))
            .expect("expected-output fixture exists")
            .replace("\r\n", "\n");
    nova()
        .arg("run")
        .arg(repo_root().join("tests/runtime/channel_full_refuses.nova"))
        .assert()
        .success()
        .stdout(expected);
}

#[test]
fn channel_fifo_order_run() {
    let expected =
        std::fs::read_to_string(repo_root().join("tests/runtime/channel_fifo_order.stdout"))
            .expect("expected-output fixture exists")
            .replace("\r\n", "\n");
    nova()
        .arg("run")
        .arg(repo_root().join("tests/runtime/channel_fifo_order.nova"))
        .assert()
        .success()
        .stdout(expected);
}

#[test]
fn channel_close_refuses_send_run() {
    let expected = std::fs::read_to_string(
        repo_root().join("tests/runtime/channel_close_refuses_send.stdout"),
    )
    .expect("expected-output fixture exists")
    .replace("\r\n", "\n");
    nova()
        .arg("run")
        .arg(repo_root().join("tests/runtime/channel_close_refuses_send.nova"))
        .assert()
        .success()
        .stdout(expected);
}

#[test]
fn channel_close_then_drain_run() {
    let expected =
        std::fs::read_to_string(repo_root().join("tests/runtime/channel_close_then_drain.stdout"))
            .expect("expected-output fixture exists")
            .replace("\r\n", "\n");
    nova()
        .arg("run")
        .arg(repo_root().join("tests/runtime/channel_close_then_drain.nova"))
        .assert()
        .success()
        .stdout(expected);
}

/// A producer sending 5 values through a channel of capacity 2, and a consumer
/// draining it, each in its own task. The only fixture in which `send` and
/// `recv` actually suspend and resume. Every other `channel_*` fixture either
/// never calls the async pair at all -- the synchronous ones, which use
/// `try_send`/`try_recv` and so cannot suspend -- or calls it and answers
/// without ever reaching a yield: the two `*_suspends_only_to_retry` fixtures
/// deliberately never enter either retry path, and
/// `channel_send_refuses_when_closed` calls the async `send` twice, the first
/// succeeding on an empty capacity-2 channel and the second refused by
/// `send`'s own `closed` check, which precedes the yield. That partition is
/// stated as a property over the whole population rather than as a count on
/// purpose: a count over that population falsified this sentence twice
/// before, and the fixture registered below would have falsified it a third
/// time. Capacity
/// below the send count is what forces `send` to suspend, and it also makes
/// `head` wrap twice. Depends on the executor's FIFO ready queue
/// (`crates/nova-runtime/src/task.rs:184`) for a deterministic interleaving;
/// the fixture's own comment says so and why.
#[test]
fn channel_two_tasks_blocking_run() {
    let expected = std::fs::read_to_string(
        repo_root().join("tests/runtime/channel_two_tasks_blocking.stdout"),
    )
    .expect("expected-output fixture exists")
    .replace("\r\n", "\n");
    nova()
        .arg("run")
        .arg(repo_root().join("tests/runtime/channel_two_tasks_blocking.nova"))
        .assert()
        .success()
        .stdout(expected);
}

/// Pins that `recv` does not suspend when a value is already buffered, i.e.
/// that its `yield_now().await` lives on the retry path and nowhere else --
/// measured against that yield moved to either side of the loop, before the
/// first `pop` and after the loop, both of which this catches. Loading the
/// channel before either task is spawned is what makes `consume`'s first `pop`
/// succeed so the retry loop is never entered; the fixture's `close` is a
/// termination guard rather than part of that, and its comment records why
/// deleting it would turn this failure into a hang.
///
/// `channel_two_tasks_blocking` is blind to the placement -- moving that
/// `yield_now` out of its retry loop leaves it and every other test in the
/// suite green, so this is the only test that fails for it, because its
/// interleaving never runs the retry-loop body twice in a row. Asserting the
/// invariant directly is not possible: a retry loop with no suspension point
/// livelocks rather than answering wrongly, so a fixture built that way would
/// hang CI instead of reporting. Same FIFO ready-queue dependency as the
/// fixtures above, recorded in the fixture's own comment.
#[test]
fn channel_recv_suspends_only_to_retry_run() {
    let expected = std::fs::read_to_string(
        repo_root().join("tests/runtime/channel_recv_suspends_only_to_retry.stdout"),
    )
    .expect("expected-output fixture exists")
    .replace("\r\n", "\n");
    nova()
        .arg("run")
        .arg(repo_root().join("tests/runtime/channel_recv_suspends_only_to_retry.nova"))
        .assert()
        .success()
        .stdout(expected);
}

/// `send`'s half of what the fixture above pins for `recv`: a `send` with room
/// in the channel must enqueue without yielding. Moving `send`'s
/// `yield_now().await` out of the retry loop to before the first `push` leaves
/// both `channel_two_tasks_blocking` and the `recv` fixture green, so this is
/// the only test that fails for it. Unlike the `recv` twin this fixture has no
/// termination guard and so can HANG rather than fail if `push` regresses; its
/// own comment says why that is documented instead of fixed. Same FIFO
/// ready-queue dependency.
#[test]
fn channel_send_suspends_only_to_retry_run() {
    let expected = std::fs::read_to_string(
        repo_root().join("tests/runtime/channel_send_suspends_only_to_retry.stdout"),
    )
    .expect("expected-output fixture exists")
    .replace("\r\n", "\n");
    nova()
        .arg("run")
        .arg(repo_root().join("tests/runtime/channel_send_suspends_only_to_retry.nova"))
        .assert()
        .success()
        .stdout(expected);
}

/// Pins `channel`'s `buffer < 1` clamp, which until this fixture existed was
/// executed by no test in the suite: every other channel fixture passes 1, 2
/// or 3, and no `channel(0)` or negative argument appeared anywhere in the
/// repo. Synchronous, so no regression of the clamp can hang. Both the zero
/// and the negative case are asserted because they fail differently without
/// the clamp -- capacity 0 refuses the first send, capacity -4 reaches
/// `[v; cap]` with a negative length -- and the clamp is what keeps a
/// negative modulus away from the ring's `%` arithmetic entirely.
#[test]
fn channel_clamps_buffer_below_one_run() {
    let expected = std::fs::read_to_string(
        repo_root().join("tests/runtime/channel_clamps_buffer_below_one.stdout"),
    )
    .expect("expected-output fixture exists")
    .replace("\r\n", "\n");
    nova()
        .arg("run")
        .arg(repo_root().join("tests/runtime/channel_clamps_buffer_below_one.nova"))
        .assert()
        .success()
        .stdout(expected);
}

/// Pins the ENQUEUE modulo in `Channel::push`, `(head + len) % cap`, which
/// until this fixture was executed by no test in a state where it wraps: no
/// other fixture pushes while `head != 0`, so deleting `% self.cap` from that
/// line left all 44 targets green. Only the dequeue modulo in `pop` was
/// pinned. Found by the whole-branch review rather than by a record -- no
/// record claimed the line was covered, and none recorded it as a gap either,
/// which is the worse of the two.
///
/// Capacity 2, two sends, one receive so `head` advances to 1 with one value
/// still live, then a third send that must land at `(1 + 1) % 2 == 0`. The
/// drain asserts the order, `2 3`, not merely the count, so a wrapped write
/// that landed in the wrong slot fails rather than passes. Synchronous, so no
/// regression of the ring arithmetic can turn it into a hang.
#[test]
fn channel_enqueue_wraps_after_dequeue_run() {
    let expected = std::fs::read_to_string(
        repo_root().join("tests/runtime/channel_enqueue_wraps_after_dequeue.stdout"),
    )
    .expect("expected-output fixture exists")
    .replace("\r\n", "\n");
    nova()
        .arg("run")
        .arg(repo_root().join("tests/runtime/channel_enqueue_wraps_after_dequeue.nova"))
        .assert()
        .success()
        .stdout(expected);
}

/// Pins that the async `send` returns false on a closed channel -- the one
/// place `send` and `try_send` are specified to differ, and, until this
/// fixture, pinned by neither. No other test calls `send` on a closed
/// channel: `channel_close_refuses_send` uses `try_send`, and
/// `channel_two_tasks_blocking`'s producer closes only after every send has
/// returned. Asserts the open case on the same channel first, so a `false`
/// is attributable to the `close` rather than to a broken `send`.
///
/// Bounded on the shipped code because the `closed` check precedes the
/// `yield_now().await`. It is NOT bounded under every regression: deleting
/// that check makes this fixture hang rather than fail, since `push` refuses
/// every value on a closed channel and `main` is the only task, so the retry
/// loop spins unpreemptably. Its own comment records why no fixture can fix
/// that. What it catches as a bounded diff is `push` losing its `closed`
/// guard, which flips the second line to true.
#[test]
fn channel_send_refuses_when_closed_run() {
    let expected = std::fs::read_to_string(
        repo_root().join("tests/runtime/channel_send_refuses_when_closed.stdout"),
    )
    .expect("expected-output fixture exists")
    .replace("\r\n", "\n");
    nova()
        .arg("run")
        .arg(repo_root().join("tests/runtime/channel_send_refuses_when_closed.nova"))
        .assert()
        .success()
        .stdout(expected);
}

#[test]
fn map_keys_run() {
    let expected = std::fs::read_to_string(repo_root().join("tests/runtime/map_keys.stdout"))
        .expect("expected-output fixture exists")
        .replace("\r\n", "\n");
    nova()
        .arg("run")
        .arg(repo_root().join("tests/runtime/map_keys.nova"))
        .assert()
        .success()
        .stdout(expected);
}

#[test]
fn json_stringify_run() {
    let expected = std::fs::read_to_string(repo_root().join("tests/runtime/json_stringify.stdout"))
        .expect("expected-output fixture exists")
        .replace("\r\n", "\n");
    nova()
        .arg("run")
        .arg(repo_root().join("tests/runtime/json_stringify.nova"))
        .assert()
        .success()
        .stdout(expected);
}

#[test]
fn json_stringify_escapes_run() {
    let expected =
        std::fs::read_to_string(repo_root().join("tests/runtime/json_stringify_escapes.stdout"))
            .expect("expected-output fixture exists")
            .replace("\r\n", "\n");
    nova()
        .arg("run")
        .arg(repo_root().join("tests/runtime/json_stringify_escapes.nova"))
        .assert()
        .success()
        .stdout(expected);
}

#[test]
fn json_parse_numbers_run() {
    let expected =
        std::fs::read_to_string(repo_root().join("tests/runtime/json_parse_numbers.stdout"))
            .expect("expected-output fixture exists")
            .replace("\r\n", "\n");
    nova()
        .arg("run")
        .arg(repo_root().join("tests/runtime/json_parse_numbers.nova"))
        .assert()
        .success()
        .stdout(expected);
}

#[test]
fn json_parse_strings_run() {
    let expected =
        std::fs::read_to_string(repo_root().join("tests/runtime/json_parse_strings.stdout"))
            .expect("expected-output fixture exists")
            .replace("\r\n", "\n");
    nova()
        .arg("run")
        .arg(repo_root().join("tests/runtime/json_parse_strings.nova"))
        .assert()
        .success()
        .stdout(expected);
}

#[test]
fn json_parse_values_run() {
    let expected =
        std::fs::read_to_string(repo_root().join("tests/runtime/json_parse_values.stdout"))
            .expect("expected-output fixture exists")
            .replace("\r\n", "\n");
    nova()
        .arg("run")
        .arg(repo_root().join("tests/runtime/json_parse_values.nova"))
        .assert()
        .success()
        .stdout(expected);
}

#[test]
fn json_round_trip_run() {
    let expected =
        std::fs::read_to_string(repo_root().join("tests/runtime/json_round_trip.stdout"))
            .expect("expected-output fixture exists")
            .replace("\r\n", "\n");
    nova()
        .arg("run")
        .arg(repo_root().join("tests/runtime/json_round_trip.nova"))
        .assert()
        .success()
        .stdout(expected);
}

#[test]
fn json_traits_run() {
    let expected = std::fs::read_to_string(repo_root().join("tests/runtime/json_traits.stdout"))
        .expect("expected-output fixture exists")
        .replace("\r\n", "\n");
    nova()
        .arg("run")
        .arg(repo_root().join("tests/runtime/json_traits.nova"))
        .assert()
        .success()
        .stdout(expected);
}
