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

/// The async transform runs on the finished MIR module, so both backends get
/// it from one place -- but "both" is a claim, and this is the half of it the
/// Cranelift tests cannot make. It asserts the two signatures the runtime's
/// `PollFn` and future value require, in the release backend's own output.
///
/// At `Float`: the wrapper must return `ptr` (a future) and the poll function
/// `i64` (a status), so the body's `double` crosses neither. Those three types
/// appearing on the right three definitions is exactly what a transform that
/// emitted one function under the original symbol would get wrong.
#[test]
fn an_await_free_async_fn_emits_a_poll_fn_and_a_wrapper() {
    let ir = ir_for("async fn f() -> Float { 1.5 }\nfn main() { let x = f() }");
    assert_well_formed(&ir);
    let poll = ir
        .lines()
        .find(|l| l.starts_with("define ") && l.contains("$poll\""))
        .unwrap_or_else(|| panic!("no `$poll` definition:\n{ir}"));
    assert!(
        poll.starts_with("define i64 @\"") && poll.ends_with("(ptr %p0, ptr %p1) {"),
        "the poll fn must be `(state, task_ctx) -> i64`, got `{poll}`"
    );
    let symbol = poll
        .trim_start_matches("define i64 @\"")
        .split('"')
        .next()
        .expect("a quoted symbol");
    let wrapper_symbol = symbol.trim_end_matches("$poll");
    assert!(
        ir.contains(&format!("define ptr @\"{wrapper_symbol}\"() {{")),
        "the original symbol must survive as a `() -> ptr` wrapper:\n{ir}"
    );
    // The wrapper's own future construction: the poll function's address in
    // word 0, then the state pointer patched over the null in word 1.
    assert!(
        ir.contains(&format!("store ptr @\"{symbol}\", ptr ")),
        "word 0 of the future must be the poll fn's address:\n{ir}"
    );
    // The body really did move, and the output store really is an f64 at the
    // output slot's byte offset. Both are asserted inside the poll function's
    // own text, so a wrapper that kept the body would fail the first and a
    // transform that stored the output through the i64 status class or at the
    // wrong offset would fail the second.
    let poll_body = ir
        .split(poll)
        .nth(1)
        .expect("text after the poll definition")
        .split("\n}")
        .next()
        .expect("the poll fn's body")
        .to_string();
    let bits = format!("0x{:016X}", 1.5f64.to_bits());
    assert!(
        poll_body.contains(&bits),
        "the body's constant ({bits}) must be inside the poll fn:{poll_body}"
    );
    // The output store: an f64, at the output slot's byte offset. A `double`
    // leaving through the i64 status class, or landing on a temp slot
    // (offset >= 16) instead, is exactly the miscompile this design avoids.
    // Found by pairing the `getelementptr` that computes the offset with the
    // instruction that stores through its result, so the assertion is about
    // one address rather than about two independent substrings.
    let out_offset = nova_mir::STATE_SLOT_OUTPUT as i64 * 8;
    let lines: Vec<&str> = poll_body.lines().map(str::trim).collect();
    let out_store = lines
        .windows(2)
        .find(|w| w[0].ends_with(&format!("i64 {out_offset}")) && w[1].starts_with("store "))
        .map(|w| w[1])
        .unwrap_or_else(|| panic!("no store at byte offset {out_offset}:{poll_body}"));
    assert!(
        out_store.starts_with("store double %"),
        "the output store must carry the f64 class, not the i64 status class: \
         `{out_store}`"
    );
    assert!(
        poll_body.contains(&format!("store i64 {}, ptr ", nova_mir::POLL_READY)),
        "the poll fn must set its status to the POLL_READY constant:{poll_body}"
    );
    assert!(
        lines.last().is_some_and(|l| l.starts_with("ret i64 ")),
        "the poll fn must return an i64 status:{poll_body}"
    );
}

/// The body text of `<name>`'s `$poll` definition. Needed once a fixture has more
/// than one `async fn`, where searching for "the `$poll`" is ambiguous.
fn poll_body_of(ir: &str, name: &str) -> String {
    let prefix = format!("define i64 @\"{name}.");
    let header = ir
        .lines()
        .find(|l| l.starts_with(&prefix) && l.contains("$poll\""))
        .unwrap_or_else(|| panic!("no `$poll` definition for `{name}`:\n{ir}"));
    ir.split(header)
        .nth(1)
        .expect("text after the definition")
        .split("\n}")
        .next()
        .expect("the poll fn's body")
        .to_string()
}

/// A function body's instruction lines, keyed by block label.
fn blocks_of(body: &str) -> std::collections::HashMap<String, Vec<String>> {
    let mut out: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
    let mut current = String::new();
    for line in body.lines() {
        let t = line.trim();
        if let Some(label) = t.strip_suffix(':') {
            if !label.is_empty() && !label.contains(' ') {
                current = label.to_string();
                out.entry(current.clone()).or_default();
                continue;
            }
        }
        if !current.is_empty() && !t.is_empty() {
            out.entry(current.clone()).or_default().push(t.to_string());
        }
    }
    out
}

/// `(default_label, [(value, label)])` from a `switch i64 %r, label %d [ … ]`
/// line.
fn parse_switch(line: &str) -> (String, Vec<(i64, String)>) {
    let (head, arms) = line.split_once('[').expect("a switch's arm list");
    let default = head
        .rsplit_once("label %")
        .expect("a switch default label")
        .1
        .trim()
        .to_string();
    let arms = arms
        .trim_end()
        .trim_end_matches(']')
        .split("i64 ")
        .filter_map(|a| {
            let (v, l) = a.split_once(", label %")?;
            Some((v.trim().parse::<i64>().ok()?, l.trim().to_string()))
        })
        .collect();
    (default, arms)
}

/// The **resumable** shape in the release backend's own output.
///
/// Everything else here covers only the await-free poll function, so without this
/// an `async fn` containing `.await` reaches `nova build --release` unexercised —
/// and `--release` is the LLVM path, which the Cranelift tests and the driver
/// probes cannot speak for.
///
/// Asserts the three constructs the split introduces that no await-free body
/// emits, inside `f`'s own poll function: the resume dispatch on the tag slot,
/// the indirect poll of the awaited future at `PollFn`'s exact signature, and a
/// suspend that writes the tag slot and returns the pending status. Both switches
/// must share one trapping default, which is the shared bad-discriminant block.
#[test]
fn an_async_fn_containing_await_emits_a_resume_dispatch_and_an_indirect_poll() {
    let ir = ir_for(
        "async fn g() -> Float { 1.5 }\n\
         async fn f() -> Float { g().await + 1.0 }\n\
         fn main() { let x = f() }",
    );
    assert_well_formed(&ir);
    let body = poll_body_of(&ir, "f");
    let blocks = blocks_of(&body);
    let switches: Vec<&String> = body
        .lines()
        .map(str::trim)
        .filter(|l| l.starts_with("switch i64 "))
        .map(|l| {
            blocks
                .values()
                .flatten()
                .find(|s| s.as_str() == l)
                .expect("every switch line belongs to a block")
        })
        .collect();
    assert_eq!(
        switches.len(),
        2,
        "one resume dispatch plus one status switch per await:{body}"
    );

    // The resume dispatch: an arm for the entry state and one per await, on the
    // value loaded from the tag slot, with distinct targets.
    let (dispatch_default, dispatch_arms) = parse_switch(switches[0]);
    let mut values: Vec<i64> = dispatch_arms.iter().map(|(v, _)| *v).collect();
    values.sort_unstable();
    assert_eq!(
        values,
        vec![0, 1],
        "the entry state plus one resume tag: `{}`",
        switches[0]
    );
    let distinct: std::collections::HashSet<&String> =
        dispatch_arms.iter().map(|(_, l)| l).collect();
    assert_eq!(
        distinct.len(),
        2,
        "the resume states must be distinct blocks: `{}`",
        switches[0]
    );
    let tag_offset = nova_mir::STATE_SLOT_TAG as i64 * 8;
    let entry = blocks
        .get("bb0")
        .unwrap_or_else(|| panic!("no `bb0` (the dispatch):{body}"));
    assert!(
        entry
            .windows(2)
            .any(
                |w| w[0].ends_with(&format!("i64 {tag_offset}")) && w[1].starts_with("load i64")
                    || w[0].ends_with(&format!("i64 {tag_offset}")) && w[1].contains("= load i64")
            ),
        "the dispatch must switch on an i64 loaded from byte offset {tag_offset}, \
         the tag slot: {entry:?}"
    );

    // Both defaults are the same block, and it traps rather than falling into a
    // state or a continuation.
    let (status_default, status_arms) = parse_switch(switches[1]);
    assert_eq!(
        dispatch_default, status_default,
        "both switches send an unrecognized discriminant to one shared block: \
         `{}` vs `{}`",
        switches[0], switches[1]
    );
    let bad = blocks
        .get(&status_default)
        .unwrap_or_else(|| panic!("no block `{status_default}`:{body}"));
    assert!(
        bad.iter().any(|l| l.contains("@llvm.trap()")) && bad.iter().any(|l| l == "unreachable"),
        "the shared default must trap: {bad:?}"
    );

    // The indirect poll, at `PollFn`'s signature: an i64 returned from a call
    // through a *register* (not a symbol) taking exactly two pointers -- the inner
    // state object and task_ctx.
    assert!(
        body.lines()
            .map(str::trim)
            .any(|l| l.contains("= call i64 %") && l.matches("ptr %").count() == 2),
        "the await must be an indirect `(ptr, ptr) -> i64` call:{body}"
    );

    // The suspend: the status switch's PENDING arm writes the tag slot and returns
    // the pending status.
    let pending = status_arms
        .iter()
        .find_map(|(v, l)| (*v == nova_mir::POLL_PENDING).then_some(l))
        .unwrap_or_else(|| panic!("no PENDING arm: `{}`", switches[1]));
    let suspend = blocks
        .get(pending)
        .unwrap_or_else(|| panic!("no block `{pending}`:{body}"));
    assert!(
        suspend
            .windows(2)
            .any(|w| w[0].ends_with(&format!("i64 {tag_offset}")) && w[1].starts_with("store i64")),
        "the suspend must store a tag at byte offset {tag_offset}: {suspend:?}"
    );
    assert!(
        suspend.iter().any(
            |l| l == &format!("store i64 {}, ptr %t6.slot", nova_mir::POLL_PENDING)
                || l.starts_with(&format!("store i64 {}, ptr %t", nova_mir::POLL_PENDING))
        ),
        "the suspend must materialize the POLL_PENDING constant: {suspend:?}"
    );
    assert!(
        suspend.last().is_some_and(|l| l.starts_with("ret i64 ")),
        "the suspend must return a status: {suspend:?}"
    );
}
