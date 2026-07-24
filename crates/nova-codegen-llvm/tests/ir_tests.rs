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
    let checked = nova_typeck::check(&ast, &resolved.definitions);
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
