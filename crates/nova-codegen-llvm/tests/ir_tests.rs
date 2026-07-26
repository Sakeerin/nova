//! Structural tests for the LLVM IR emitter.
//!
//! Without an LLVM toolchain on the machine the IR cannot be assembled/run
//! here, so these assert on the shape of the emitted module: the entry
//! wrapper, runtime declarations, and the instruction patterns each MIR
//! construct lowers to. End-to-end execution of `--release` binaries is a
//! separate, toolchain-gated integration test.

use nova_codegen_llvm::compile_ir;
use nova_diagnostics::FileId;

fn ir_for(src: &str) -> String {
    let file_id = FileId::DUMMY;
    let (tokens, lex_errors) = nova_lexer::lex(src, file_id);
    assert!(lex_errors.is_empty(), "lex: {lex_errors:?}");
    let (ast, parse_errors) = nova_parser::parse(&tokens, file_id);
    assert!(parse_errors.is_empty(), "parse: {parse_errors:?}");
    let ast = ast.expect("ast");
    let resolved = nova_resolver::resolve(&ast);
    assert!(
        resolved.diagnostics.is_empty(),
        "resolve: {:?}",
        resolved.diagnostics
    );
    let checked = nova_typeck::check(&resolved.file, &resolved.definitions);
    assert!(
        checked.diagnostics.is_empty(),
        "typeck: {:?}",
        checked.diagnostics
    );
    let mir = nova_mir::lower_module(&checked.module).expect("mir lowering");
    compile_ir(&mir).expect("ir emission")
}

/// Basic well-formedness: braces balance, and the block just before every
/// closing `}` ends in a terminator instruction (so no function body falls
/// through the end).
fn assert_well_formed(ir: &str) {
    assert_eq!(
        ir.matches('{').count(),
        ir.matches('}').count(),
        "unbalanced braces:\n{ir}"
    );
    let terms = ["br ", "ret ", "switch ", "unreachable"];
    let lines: Vec<&str> = ir.lines().map(|l| l.trim()).collect();
    for (i, line) in lines.iter().enumerate() {
        if *line != "}" {
            continue;
        }
        // The last non-empty line before this `}` must be a terminator.
        let prev = lines[..i].iter().rev().find(|l| !l.is_empty());
        if let Some(prev) = prev {
            assert!(
                terms.iter().any(|k| prev.starts_with(k)),
                "block does not end in a terminator (`{prev}`):\n{ir}"
            );
        }
    }
}

#[test]
fn hello_emits_entry_and_c_wrapper() {
    let ir = ir_for("fn main() { println(\"hi\") }");
    assert!(ir.contains("define void @\"nova_main\"()"), "{ir}");
    assert!(
        ir.contains("define i32 @main(i32 %argc, ptr %argv)"),
        "{ir}"
    );
    assert!(ir.contains("call void @nova_main()"), "{ir}");
    assert!(ir.contains("declare void @nova_rt_println(ptr)"), "{ir}");
    assert!(ir.contains("call void @nova_rt_println"), "{ir}");
    assert_well_formed(&ir);
}

#[test]
fn string_literal_becomes_a_global_and_str_new() {
    let ir = ir_for("fn main() { println(\"hi\") }");
    assert!(
        ir.contains("private unnamed_addr constant [2 x i8] c\"hi\""),
        "{ir}"
    );
    assert!(ir.contains("call ptr @nova_rt_str_new(ptr"), "{ir}");
}

#[test]
fn recursion_and_branch_lower() {
    let ir = ir_for(
        "fn fib(n: Int) -> Int { if n <= 1 { n } else { fib(n - 1) + fib(n - 2) } }\n\
         fn main() { println(\"${fib(5)}\") }",
    );
    // `fib` is mangled with its DefId (e.g. `fib.0`) so cross-module
    // same-named functions can't collide; match the prefix, not the exact id.
    assert!(ir.contains("define i64 @\"fib."), "{ir}");
    assert!(ir.contains("call i64 @\"fib."), "{ir}");
    assert!(ir.contains("icmp sle i64"), "{ir}");
    assert!(ir.contains("br i1"), "{ir}");
    assert_well_formed(&ir);
}

#[test]
fn match_lowers_to_switch_with_trap_default() {
    let ir = ir_for(
        "type Shape = | Circle(Int) | Rect(Int, Int) | Empty\n\
         fn area(s: Shape) -> Int { match s { Circle(r) => r, Rect(w, h) => w, Empty => 0 } }\n\
         fn main() { println(\"${area(Empty)}\") }",
    );
    assert!(ir.contains("switch i64"), "{ir}");
    // Sum construction: alloc + tag store.
    assert!(ir.contains("call ptr @nova_rt_alloc"), "{ir}");
    // Exhaustive-match default is a trap.
    assert!(ir.contains("call void @llvm.trap()"), "{ir}");
    assert!(ir.contains("unreachable"), "{ir}");
    assert_well_formed(&ir);
}

#[test]
fn bool_match_switches_on_i8_not_i64() {
    // Regression (adversarial review): a `Bool` scrutinee is an `i8`, so the
    // `switch` condition and case constants must be `i8`. Spelling it `i64`
    // (as sum/Int switches are) produced a type-mismatched module LLVM rejects.
    let ir = ir_for(
        "fn classify(b: Bool) -> Int { match b { true => 1, false => 0 } }\n\
         fn main() { println(\"${classify(true)}\") }",
    );
    assert!(ir.contains("switch i8 "), "expected i8 switch:\n{ir}");
    assert!(
        !ir.contains("switch i64 "),
        "a bool match must not switch on i64:\n{ir}"
    );
    assert_well_formed(&ir);
}

#[test]
fn sum_match_still_switches_on_i64() {
    // The tag discriminant of a sum match is `i64` — this must be unaffected.
    let ir = ir_for(
        "type E = | A | B | C\n\
         fn f(e: E) -> Int { match e { A => 0, B => 1, C => 2 } }\n\
         fn main() { println(\"${f(B)}\") }",
    );
    assert!(ir.contains("switch i64 "), "expected i64 sum switch:\n{ir}");
    assert_well_formed(&ir);
}

#[test]
fn floats_use_hex_constants_and_fp_ops() {
    let ir = ir_for(
        "fn main() {\n\
             let a = 3.5\n let b = 2.0\n let c = a * b - 1.0\n\
             println(\"${c > 5.0}\")\n\
         }",
    );
    // 3.5 == 0x400C000000000000 exactly.
    assert!(ir.contains("double 0x400C000000000000"), "{ir}");
    assert!(ir.contains("fmul double"), "{ir}");
    assert!(ir.contains("fsub double"), "{ir}");
    assert!(ir.contains("fcmp ogt double"), "{ir}");
    assert_well_formed(&ir);
}

#[test]
fn arrays_emit_bounds_check_and_element_addressing() {
    let ir = ir_for(
        "fn main() {\n\
             let mut xs = [1, 2, 3]\n xs[0] = xs[1]\n\
             println(\"${xs.len()} ${xs[2]}\")\n\
         }",
    );
    assert!(ir.contains("call void @nova_rt_check_bounds"), "{ir}");
    // element address = base + index*8 + 8
    assert!(ir.contains("mul i64"), "{ir}");
    assert!(ir.contains("getelementptr inbounds i8, ptr"), "{ir}");
    assert_well_formed(&ir);
}

#[test]
fn closures_emit_fat_pointer_and_indirect_call() {
    let ir = ir_for(
        "fn main() {\n\
             let base = 10\n let f = |n| n + base\n\
             println(\"${f(5)}\")\n\
         }",
    );
    // Fat pointer is a 16-byte allocation storing a code pointer.
    assert!(ir.contains("call ptr @nova_rt_alloc(i64 16)"), "{ir}");
    assert!(ir.contains("store ptr @\""), "{ir}");
    // Indirect call is env-first through a loaded code pointer.
    assert!(ir.contains("(ptr "), "{ir}");
    assert_well_formed(&ir);
}

#[test]
fn records_emit_field_addressing() {
    let ir = ir_for(
        "record Point { x: Int, y: Int }\n\
         fn main() { let p = Point { x: 3, y: 4 }\n println(\"${p.x}\") }",
    );
    assert!(ir.contains("call ptr @nova_rt_alloc"), "{ir}");
    assert!(ir.contains("getelementptr inbounds i8, ptr"), "{ir}");
    assert_well_formed(&ir);
}

/// `rec.field = v` must emit a *store* through the field address, at the same
/// `8 * index` offset `RecordField` loads from. No LLVM toolchain is available
/// here to run the IR (the Cranelift `nova run`/`nova build` e2e tests cover
/// execution), so this is the only guard on the `--release` backend's store:
/// without it the arm could silently go missing or use the wrong offset and
/// every test would still pass.
#[test]
fn field_assignment_emits_store_at_field_offset() {
    let ir = ir_for(
        "record Point { x: Int, y: Int }\n\
         fn main() {\n\
             let mut p = Point { x: 3, y: 4 }\n\
             p.y = 5\n\
             println(\"${p.y}\")\n\
         }",
    );
    // Scope to the user's `main` (`@main` is a thin wrapper around it).
    let body: Vec<&str> = ir
        .lines()
        .map(str::trim)
        .skip_while(|l| !l.starts_with("define void @\"nova_main\"("))
        .take_while(|l| *l != "}")
        .collect();
    assert!(!body.is_empty(), "no @nova_main body in:\n{ir}");
    // The record literal *also* initializes field 1, through a store at the
    // very same offset — so "a store at base+8 exists" would pass even if the
    // `SetField` arm emitted nothing at all. The assignment is distinguished by
    // its base pointer: the literal writes through the fresh `nova_rt_alloc`
    // result, while the assignment reloads the record from its local slot.
    let alloc_reg = body
        .iter()
        .find_map(|l| l.strip_suffix(" = call ptr @nova_rt_alloc(i64 16)"))
        .unwrap_or_else(|| panic!("no 16-byte record allocation in:\n{ir}"));
    // `y` is field 1, so its address is the record pointer + 8.
    let reloaded_field1: Vec<&str> = body
        .iter()
        .filter_map(|l| {
            let (dst, rest) = l.split_once(" = getelementptr inbounds i8, ptr ")?;
            let base = rest.strip_suffix(", i64 8")?;
            (base != alloc_reg).then_some(dst)
        })
        .collect();
    assert!(
        !reloaded_field1.is_empty(),
        "no field-1 address off a reloaded record pointer in:\n{ir}"
    );
    // At least one of those addresses must be *stored* through, not just read
    // (the `println("${p.y}")` read computes one of these addresses too).
    assert!(
        reloaded_field1.iter().any(|addr| body
            .iter()
            .any(|l| l.starts_with("store i64 ") && l.ends_with(&format!("ptr {addr}")))),
        "expected an i64 store through a reloaded field-1 address in:\n{ir}"
    );
    assert_well_formed(&ir);
}

/// `[init; n]`'s `ArrayAlloc` under the LLVM `--release` backend. No toolchain
/// is available here to assemble and run the emitted IR (the Cranelift path
/// covers execution in `nova-cli`'s `array_repeat_*` e2e tests), so this pins
/// the IR's *shape*: the runtime-length size arithmetic (`8 + 8*n`), the
/// allocation, and the length store at offset 0 — the parts that would silently
/// go missing or go wrong if this arm broke.
#[test]
fn repeat_array_emits_runtime_length_alloc() {
    let ir = ir_for("fn main() { let n = 3\n let a = [7; n]\n println(\"${a[0]}\") }");
    let body: Vec<&str> = ir.lines().map(|l| l.trim()).collect();

    // Every assertion below chains backwards from the allocation call, so it
    // binds to `ArrayAlloc`'s own registers. Matching on shape alone would be
    // close to vacuous here: `let n = 3` emits `store i64 3, ptr %t2.slot`,
    // which looks exactly like a length store, and the fill loop's
    // element-address arithmetic emits its own `mul …, 8` / `add …, 8` pair.
    let defining = |reg: &str, op: &str| -> String {
        let prefix = format!("{reg} = {op} ");
        body.iter()
            .find(|l| l.starts_with(&prefix))
            .unwrap_or_else(|| panic!("expected `{prefix}…` in:\n{ir}"))
            .to_string()
    };
    // `<len>, 8` → `<len>`: the operand a `mul`/`add` scales by 8.
    let scaled_operand = |line: &str, op: &str| -> String {
        line.split_once(&format!(" = {op} "))
            .and_then(|(_, rhs)| rhs.strip_suffix(", 8"))
            .unwrap_or_else(|| panic!("expected `{op} <reg>, 8`, got `{line}` in:\n{ir}"))
            .to_string()
    };

    let call = body
        .iter()
        .find(|l| l.contains(" = call ptr @nova_rt_alloc(i64 "))
        .unwrap_or_else(|| panic!("expected an `@nova_rt_alloc(i64 …)` call in:\n{ir}"));
    let alloc_dst = call
        .split(" = ")
        .next()
        .expect("the alloc call line has a destination register");
    let size_reg = call
        .rsplit_once("(i64 ")
        .and_then(|(_, rest)| rest.strip_suffix(')'))
        .unwrap_or_else(|| panic!("expected a register size argument in `{call}`"));

    // The size passed to the allocator must be the computed `8 + 8*n`, not the
    // raw length: the `add` that defines it adds the 8-byte length header, and
    // the `mul` feeding that `add` scales the length by the 8-byte slot size.
    let add = defining(size_reg, "add i64");
    let bytes_reg = scaled_operand(&add, "add i64");
    let mul = defining(&bytes_reg, "mul i64");
    let len_reg = scaled_operand(&mul, "mul i64");

    // The length is stored at offset 0 — straight through the pointer *this*
    // allocation returned, with no getelementptr — and the value stored is the
    // same register the size arithmetic consumed.
    let len_store = format!("store i64 {len_reg}, ptr {alloc_dst}");
    assert!(
        body.contains(&len_store.as_str()),
        "expected `{len_store}` (the length stored at offset 0 through the \
         allocation's own pointer) in:\n{ir}"
    );

    // The fill loop is lowered in MIR, so it reaches the backend as ordinary
    // blocks and branches rather than anything array-specific.
    assert!(
        body.iter().any(|l| l.starts_with("br i1 ")),
        "expected the fill loop's conditional branch in:\n{ir}"
    );
    assert!(
        ir.contains("call void @nova_rt_panic_str("),
        "expected the negative-length guard's panic call in:\n{ir}"
    );
    assert_well_formed(&ir);
}

/// `panic` (Phase 2.1) under the LLVM `--release` backend: no toolchain is
/// available in this environment to assemble/run the emitted IR (see the
/// clang-gated `release_builds_and_runs_when_clang_available` in
/// `nova-cli`'s e2e suite), so this pins the IR's *shape* instead — the
/// runtime declaration is always present (`DECLS` is unconditional), but the
/// call site only appears when a program actually calls `panic`, which is
/// the part that would silently go missing if the builtin's MIR lowering
/// (`Builtin::Panic => RtFunc::Panic`) ever broke.
#[test]
fn panic_emits_declaration_and_call() {
    let ir = ir_for("fn main() { panic(\"boom\") }");
    assert!(ir.contains("declare void @nova_rt_panic_str(ptr)"), "{ir}");
    assert!(ir.contains("call void @nova_rt_panic_str("), "{ir}");
    assert_well_formed(&ir);
}

/// Regression for the bug class this backend used to be exposed to: the
/// declaration list (`DECLS`) used to be a hand-written array of raw
/// strings with no compile-time tie to `RtFunc` at all, so a new variant
/// could be added and its declaration forgotten — the crate still compiled
/// clean, and `nova build --release` would silently emit a call to an
/// undeclared symbol (invalid LLVM IR). Declarations are unconditional
/// (emitted regardless of whether the tiny program below calls them), so
/// this asserts every `RtFunc` variant's exact `declare` line — spelled out
/// independently here via `signature()`, not by calling the backend's own
/// generator — is present in the emitted IR.
#[test]
fn every_rt_func_is_declared_with_its_real_signature() {
    fn llty(ty: nova_mir::MirTy) -> &'static str {
        match ty {
            nova_mir::MirTy::I64 => "i64",
            nova_mir::MirTy::F64 => "double",
            nova_mir::MirTy::I8 => "i8",
            nova_mir::MirTy::Ptr => "ptr",
            nova_mir::MirTy::Unit => "void",
        }
    }

    let ir = ir_for("fn main() {}");
    for rt in nova_mir::RtFunc::ALL {
        let (params, ret) = rt.signature();
        let params: Vec<&str> = params.iter().map(|&p| llty(p)).collect();
        let expected = format!(
            "declare {} @{}({})",
            llty(ret),
            rt.symbol(),
            params.join(", ")
        );
        assert!(
            ir.lines().any(|l| l == expected),
            "missing declaration for RtFunc::{rt:?}: expected line `{expected}` in IR:\n{ir}"
        );
    }
}
