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

#[test]
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
///
/// This is design doc §7's gate item 3, and it is the reason `Item = T` rather
/// than a primitive: `Int` and `String` are different machine classes, so a
/// substitution that dropped the impl's argument, or a normalization that
/// cached its first answer, cannot give both the right result.
///
/// The receiver is `mut it`, not `it`. §7 originally wrote
/// `fn first<I: Iterator>(it: I)`; Task 8 made `mut self` on a trait method
/// enforced, so that spelling is now `E0060` and the `mut` is the rule working
/// rather than a workaround. `mut` on a *parameter* is what carries it — there
/// is no `let mut` to reach for when the iterator arrives as an argument.
#[test]
fn a_generic_function_over_iterator_resolves_item_per_instantiation() {
    let src = "fn first<I: Iterator>(mut it: I) -> Option<I::Item> { it.next() }\n\
               fn main() {\n\
                   let mut ns: Vec<Int> = Vec::new()\n\
                   ns.push(7)\n\
                   let mut ss: Vec<String> = Vec::new()\n\
                   ss.push(\"hi\")\n\
                   match first(ns.iter()) { Some(n) => println(\"int=${n}\"), None => println(\"int=none\") }\n\
                   match first(ss.iter()) { Some(s) => println(\"str=${s}\"), None => println(\"str=none\") }\n\
                   let e: Vec<Bool> = Vec::new()\n\
                   match first(e.iter()) { Some(b) => println(\"bool=${b}\"), None => println(\"bool=none\") }\n\
               }";
    let dir = std::env::temp_dir().join("nova-assoc-iterator-generic");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("main.nova");
    std::fs::write(&path, src).expect("write");
    nova()
        .arg("run")
        .arg(&path)
        .assert()
        .success()
        .stdout("int=7\nstr=hi\nbool=none\n");
}

/// `Iterator::next` takes `mut self`, so calling it through an immutable
/// binding is `E0060` — on `std/core`'s own trait, not just a test-local one.
///
/// Task 8 pinned the rule on a synthetic `trait Bump`. This pins it on the
/// shipped trait, which is what a user actually meets, and is the reason
/// `let mut it` appears in every test above rather than being incidental
/// style. Without it a caller could silently advance an iterator someone else
/// believes is unread.
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
