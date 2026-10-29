//! `match` lowering by test chains (5.0 spec
//! `docs/superpowers/specs/2026-10-26-phase-5-0-the-language-and-std-a-compiler-needs-design.md`
//! §4.6): a flat match keeps its one `Switch`; any other match lowers wholly
//! by chains, which evaluate the scrutinee and each projection once, send a
//! false guard to the next arm's chain, and fill an or-pattern's bindings
//! on every path.

use nova_diagnostics::FileId;
use nova_mir::{Function, Stmt, Terminator};

fn mir_for(src: &str) -> nova_mir::Module {
    let file_id = FileId::DUMMY;
    let (tokens, lex_errors) = nova_lexer::lex(src, file_id);
    assert!(lex_errors.is_empty(), "lex errors: {lex_errors:?}");
    let (ast, parse_errors) = nova_parser::parse(&tokens, file_id);
    assert!(parse_errors.is_empty(), "parse errors: {parse_errors:?}");
    let resolved = nova_resolver::resolve(&ast.expect("no AST"));
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
    nova_mir::lower_module(&checked.module).expect("MIR lowering failed")
}

/// The function named `name`, or its one instance.
fn func<'m>(mir: &'m nova_mir::Module, name: &str) -> &'m Function {
    let instance = format!("{name}.");
    mir.functions
        .iter()
        .find(|f| f.name == name || f.name.starts_with(&instance))
        .unwrap_or_else(|| panic!("no function `{name}`"))
}

fn stmts(f: &Function) -> Vec<&Stmt> {
    f.blocks.iter().flat_map(|b| &b.stmts).collect()
}

fn count(f: &Function, pred: impl Fn(&Stmt) -> bool) -> usize {
    stmts(f).into_iter().filter(|s| pred(s)).count()
}

fn switches(f: &Function) -> usize {
    f.blocks
        .iter()
        .filter(|b| matches!(b.term, Terminator::Switch { .. }))
        .count()
}

fn branches(f: &Function) -> usize {
    f.blocks
        .iter()
        .filter(|b| matches!(b.term, Terminator::Branch { .. }))
        .count()
}

const SUMS: &str = "type Opt<T> =\n  | Yes(T)\n  | No\n\
                    type Wrap =\n  | Wrap(Int, String)\n";

fn program(body: &str, call: &str) -> String {
    format!("{SUMS}{body}\nfn main() {{ println(\"${{{call}}}\") }}\n")
}

#[test]
fn a_flat_match_keeps_its_one_switch() {
    let mir = mir_for(&program(
        "fn f(o: Opt<Int>) -> Int { match o { Yes(x) => x, No => 0 } }",
        "f(No)",
    ));
    let f = func(&mir, "f");
    assert_eq!(switches(f), 1);
    assert_eq!(branches(f), 0);
    // A `Char` literal match is flat too: one switch on code points.
    let mir = mir_for(&program(
        "fn f(c: Char) -> Int { match c { 'a' => 1, 'b' => 2, _ => 3 } }",
        "f('a')",
    ));
    let f = func(&mir, "f");
    let arms: Vec<i64> = f
        .blocks
        .iter()
        .find_map(|b| match &b.term {
            Terminator::Switch { arms, .. } => Some(arms.iter().map(|(v, _)| *v).collect()),
            _ => None,
        })
        .expect("a switch");
    assert_eq!(arms, ['a' as i64, 'b' as i64]);
    assert_eq!(branches(f), 0);
}

#[test]
fn a_nested_match_lowers_by_chains_with_no_switch() {
    let mir = mir_for(&program(
        "fn f(o: Opt<Opt<Int>>) -> Int { match o { Yes(Yes(1)) => 1, Yes(_) => 2, No => 3 } }",
        "f(No)",
    ));
    let f = func(&mir, "f");
    assert_eq!(switches(f), 0);
    // One tag test per variant each arm names: two, one and one.
    assert_eq!(count(f, |s| matches!(s, Stmt::SumTag { .. })), 4);
}

#[test]
fn each_projection_is_evaluated_once_per_chain() {
    let mir = mir_for(&program(
        "fn f(o: Opt<(Int, Int)>) -> Int { match o { Yes((a, b)) => a + b, No => 0 } }",
        "f(No)",
    ));
    let f = func(&mir, "f");
    assert_eq!(count(f, |s| matches!(s, Stmt::SumField { .. })), 1);
    assert_eq!(count(f, |s| matches!(s, Stmt::RecordField { .. })), 2);
}

#[test]
fn the_scrutinee_is_evaluated_once() {
    let mir = mir_for(&program(
        "fn g() -> (Int, Int) { (1, 2) }\n\
         fn f() -> Int { match g() { (1, _) => 1, (_, 2) => 2, _ => 3 } }",
        "f()",
    ));
    let f = func(&mir, "f");
    let calls = count(
        f,
        |s| matches!(s, Stmt::Call { callee, .. } if callee.starts_with("g")),
    );
    assert_eq!(calls, 1);
}

#[test]
fn a_false_guard_jumps_to_the_next_arms_chain() {
    let mir = mir_for(&program(
        "fn f(n: Int) -> Int { match n { x if x > 5 => 1, 7 => 2, _ => 3 } }",
        "f(7)",
    ));
    let f = func(&mir, "f");
    // The guard's compare is the one `>` in the function; the block it
    // ends in branches on it, and its false edge enters the next arm,
    // whose first test loads the literal 7.
    let (guard_block, else_) = f
        .blocks
        .iter()
        .find_map(|b| {
            let has_gt = b.stmts.iter().any(|s| {
                matches!(
                    s,
                    Stmt::Bin {
                        op: nova_hir::BinOp::Gt,
                        ..
                    }
                )
            });
            match (&b.term, has_gt) {
                (Terminator::Branch { else_, .. }, true) => Some((b, *else_)),
                _ => None,
            }
        })
        .expect("the guard's branch");
    assert!(
        matches!(
            f.blocks[else_.0 as usize].stmts.first(),
            Some(Stmt::ConstInt(_, 7))
        ),
        "{:?} then {:?}",
        guard_block,
        f.blocks[else_.0 as usize]
    );
}

#[test]
fn an_or_pattern_fills_its_bindings_on_every_path_and_shares_one_body() {
    let mir = mir_for(&program(
        "fn f(p: (Int, Int)) -> Int { match p { (a, 0) | (0, a) => a, _ => 0 } }",
        "f((0, 4))",
    ));
    let f = func(&mir, "f");
    // `a` is written once in each alternative, into the same temporary.
    let mut dsts: Vec<u32> = stmts(f)
        .into_iter()
        .filter_map(|s| match s {
            Stmt::Copy { dst, .. } => Some(dst.0),
            _ => None,
        })
        .collect();
    dsts.sort();
    let a = dsts
        .windows(2)
        .find(|w| w[0] == w[1])
        .map(|w| w[0])
        .expect("one local copied into twice");
    assert_eq!(dsts.iter().filter(|d| **d == a).count(), 2, "{dsts:?}");
    // Both alternatives' successes jump to one block.
    let mut gotos: Vec<u32> = f
        .blocks
        .iter()
        .filter_map(|b| match b.term {
            Terminator::Goto(t) => Some(t.0),
            _ => None,
        })
        .collect();
    gotos.sort();
    assert!(gotos.windows(2).any(|w| w[0] == w[1]), "{gotos:?}");
}

#[test]
fn a_range_is_two_compares_with_its_ends_included() {
    let mir = mir_for(&program(
        "fn f(n: Int) -> Int { match n { 1..5 => 1, 10..=20 => 2, _ => 0 } }",
        "f(3)",
    ));
    let f = func(&mir, "f");
    let consts: Vec<i64> = stmts(f)
        .into_iter()
        .filter_map(|s| match s {
            Stmt::ConstInt(_, v) => Some(*v),
            _ => None,
        })
        .collect();
    // `1..5` is tested as `1..=4`.
    for end in [1, 4, 10, 20] {
        assert!(consts.contains(&end), "{consts:?}");
    }
    assert!(!consts.contains(&5), "{consts:?}");
    let op = |want: nova_hir::BinOp| count(f, |s| matches!(s, Stmt::Bin { op, .. } if *op == want));
    assert_eq!(op(nova_hir::BinOp::Ge), 2);
    assert_eq!(op(nova_hir::BinOp::Le), 2);
}

#[test]
fn let_destructuring_lowers_to_projections_alone() {
    let mir = mir_for(&program(
        "fn f() -> Int {\n    let (Wrap(n, _), (m, k)) = (Wrap(1, \"s\"), (2, 3))\n    n + m + k\n}",
        "f()",
    ));
    let f = func(&mir, "f");
    assert_eq!(branches(f), 0);
    assert_eq!(switches(f), 0);
    // A sum of one variant needs no tag test.
    assert_eq!(count(f, |s| matches!(s, Stmt::SumTag { .. })), 0);
    assert_eq!(count(f, |s| matches!(s, Stmt::SumField { .. })), 1);
}

#[test]
fn a_bool_literal_branches_on_the_value() {
    let mir = mir_for(&program(
        "fn f(p: (Bool, Int)) -> Int { match p { (true, n) => n, (false, _) => 0 } }",
        "f((true, 1))",
    ));
    let f = func(&mir, "f");
    assert_eq!(count(f, |s| matches!(s, Stmt::Bin { .. })), 0);
    assert_eq!(branches(f), 2);
}

#[test]
fn a_string_literal_at_depth_is_one_string_compare() {
    let mir = mir_for(&program(
        "fn f(o: Opt<String>) -> Int { match o { Yes(\"a\") => 1, Yes(_) => 2, No => 3 } }",
        "f(No)",
    ));
    let f = func(&mir, "f");
    assert_eq!(
        count(
            f,
            |s| matches!(s, Stmt::CallRuntime { func, .. } if *func == nova_mir::RtFunc::StrEq)
        ),
        1
    );
}
