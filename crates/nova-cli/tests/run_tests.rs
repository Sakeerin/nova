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
