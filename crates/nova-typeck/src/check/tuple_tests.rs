//! The checker's tests of 5.0b's tuples (spec
//! `docs/superpowers/specs/2026-10-26-phase-5-0-the-language-and-std-a-compiler-needs-design.md`
//! §4.1, §4.2): tuple types and literals, `t.N`, inference through
//! generics, and the codes a tuple meets: E0010, E0011, E0013 and E0014.

use super::*;
use nova_diagnostics::{FileId, Severity};
use nova_lexer::lex;
use nova_parser::parse;
use nova_resolver::resolve;

/// `src` checked, the resolver's diagnostics first.
fn checked(src: &str) -> CheckResult {
    let file_id = FileId::DUMMY;
    let (tokens, lex_errors) = lex(src, file_id);
    assert!(lex_errors.is_empty(), "lex errors: {lex_errors:?}");
    let (ast, parse_errors) = parse(&tokens, file_id);
    assert!(parse_errors.is_empty(), "parse errors: {parse_errors:?}");
    let resolved = resolve(&ast.expect("an AST"));
    let mut result = check(&resolved.file, &resolved.definitions);
    let mut all = resolved.diagnostics;
    all.append(&mut result.diagnostics);
    result.diagnostics = all;
    result
}

fn errors(src: &str) -> Vec<Diagnostic> {
    checked(src)
        .diagnostics
        .into_iter()
        .filter(|d| d.severity == Severity::Error)
        .collect()
}

/// The one error of `src`, which has `code`.
fn only(src: &str, code: &str) -> Diagnostic {
    let all = errors(src);
    assert_eq!(all.len(), 1, "one error in {all:#?}");
    assert_eq!(all[0].code, code, "{all:#?}");
    all[0].clone()
}

fn clean(src: &str) {
    let all = checked(src).diagnostics;
    assert!(all.is_empty(), "{all:#?}");
}

/// Every expression of `name`'s body, outermost first.
fn exprs<'m>(module: &'m hir::Module, name: &str) -> Vec<&'m hir::Expr> {
    fn walk<'e>(e: &'e hir::Expr, out: &mut Vec<&'e hir::Expr>) {
        out.push(e);
        for c in child_exprs(e) {
            walk(c, out);
        }
    }
    let f = module
        .functions
        .iter()
        .find(|f| f.name == name)
        .expect("the function");
    let mut out = Vec::new();
    walk(&f.body, &mut out);
    out
}

// --- Types and literals ---

#[test]
fn tuple_types_and_literals_check() {
    clean(
        "fn div_mod(a: Int, b: Int) -> (Int, Int) { (a / b, a % b) }\n\
         fn main() {\n    let p: (String, Int) = (\"x\", 1)\n    let one: (Int,) = (5,)\n    \
         let q = div_mod(7, 2)\n    let n: ((Int, Int), (Bool, (Char, Float))) = ((1, 2), (true, ('c', 1.5)))\n    \
         let v: Vec<(Int, String)> = Vec::new()\n    let o: Option<(Int, Bool)> = Some((1, true))\n    \
         let u: ((), Int) = ((), 1)\n}\n",
    );
}

#[test]
fn a_tuple_literal_builds_a_tuple_and_a_field_reads_its_index() {
    let r = checked(
        "fn f() -> Int {\n    let t = (1, \"s\", true)\n    t.2\n    t.0\n}\nfn main() {}\n",
    );
    let all = exprs(&r.module, "f");
    let made: Vec<&Ty> = all
        .iter()
        .filter(|e| matches!(e.kind, hir::ExprKind::MakeTuple { .. }))
        .map(|e| &e.ty)
        .collect();
    assert_eq!(made, [&Ty::Tuple(vec![Ty::Int, Ty::String, Ty::Bool])]);
    let reads: Vec<(u32, &Ty)> = all
        .iter()
        .filter_map(|e| match &e.kind {
            hir::ExprKind::FieldGet { index, .. } => Some((*index, &e.ty)),
            _ => None,
        })
        .collect();
    assert_eq!(reads, [(2, &Ty::Bool), (0, &Ty::Int)]);
}

#[test]
fn a_parenthesised_type_and_expression_are_not_tuples() {
    clean("fn f(x: (Int)) -> Int { (x) }\nfn main() {}\n");
    let r = checked("fn f() -> Int { ((5)) }\nfn main() {}\n");
    assert!(exprs(&r.module, "f")
        .iter()
        .all(|e| !matches!(e.kind, hir::ExprKind::MakeTuple { .. })));
}

#[test]
fn a_tuple_of_the_wrong_arity_is_e0010() {
    let d = only(
        "fn main() {\n    let p: (Int, Int) = (1, 2, 3)\n}\n",
        "E0010",
    );
    assert!(d.message.contains("`(Int, Int, Int)`"), "{d:?}");
    assert!(d.message.contains("`(Int, Int)`"), "{d:?}");
}

#[test]
fn a_tuple_of_the_wrong_element_type_is_e0010() {
    let d = only(
        "fn want(p: (Int, Int)) -> Int { p.0 }\nfn main() {\n    let x = want((\"s\", 2))\n}\n",
        "E0010",
    );
    assert!(d.message.contains("`(String, Int)`"), "{d:?}");
}

#[test]
fn a_tuple_of_one_is_not_its_element() {
    let d = only("fn main() {\n    let p: Int = (5,)\n}\n", "E0010");
    assert!(d.message.contains("`(Int,)`"), "{d:?}");
}

#[test]
fn an_element_whose_type_cannot_be_inferred_is_e0011() {
    let all = errors("fn main() {\n    let p = ([], 1)\n}\n");
    assert!(
        !all.is_empty() && all.iter().all(|d| d.code == "E0011"),
        "{all:#?}"
    );
}

// --- Generics, closures, and other types' arguments ---

#[test]
fn tuples_infer_through_generic_functions() {
    clean(
        "fn swap<A, B>(p: (A, B)) -> (B, A) { (p.1, p.0) }\n\
         fn first<A, B>(p: (A, B)) -> A { p.0 }\n\
         fn main() {\n    let s: (Bool, Int) = swap((1, true))\n    let n: Int = first(swap((\"x\", 3)))\n}\n",
    );
    let d = only(
        "fn swap<A, B>(p: (A, B)) -> (B, A) { (p.1, p.0) }\n\
         fn main() {\n    let s: (Int, Bool) = swap((1, true))\n}\n",
        "E0010",
    );
    assert!(d.message.contains("`(Bool, Int)`"), "{d:?}");
}

#[test]
fn tuples_flow_through_closures_and_function_types() {
    clean(
        "fn apply(f: fn((Int, Int)) -> Int, p: (Int, Int)) -> Int { f(p) }\n\
         fn main() {\n    let mul = |t: (Int, Int)| t.0 * t.1\n    let n = apply(mul, (2, 3))\n    \
         let base = (1, 2)\n    let add = |x: Int| (base.0 + x, base.1 + x)\n    let r: (Int, Int) = add(1)\n}\n",
    );
}

#[test]
fn a_record_field_may_hold_a_tuple() {
    clean(
        "record Holder { pair: (String, Int) }\n\
         fn main() {\n    let mut h = Holder { pair: (\"h\", 1) }\n    let n: Int = h.pair.1\n    h.pair = (\"g\", 2)\n}\n",
    );
}

// --- `t.N` ---

#[test]
fn an_index_past_the_end_is_e0014() {
    let d = only(
        "fn main() {\n    let t = (1, 2)\n    let x = t.2\n}\n",
        "E0014",
    );
    assert_eq!(d.message, "no field `2` on `(Int, Int)`");
}

#[test]
fn a_name_on_a_tuple_is_e0014() {
    let d = only(
        "fn main() {\n    let t = (1, 2)\n    let x = t.x\n}\n",
        "E0014",
    );
    assert_eq!(d.message, "no field `x` on `(Int, Int)`");
}

#[test]
fn an_index_on_a_type_that_is_not_a_tuple_is_e0014() {
    let d = only("fn main() {\n    let k = 5\n    let x = k.0\n}\n", "E0014");
    assert_eq!(d.message, "cannot access field `0` on `Int`");
    let d = only(
        "record P { x: Int }\nfn main() {\n    let p = P { x: 1 }\n    let y = p.0\n}\n",
        "E0014",
    );
    assert_eq!(d.message, "no field `0` on record `P`");
}

#[test]
fn an_index_on_a_value_of_unknown_type_is_one_e0011() {
    let d = only("fn main() {\n    let f = |p| p.0\n}\n", "E0011");
    assert!(d.message.contains("`.0`"), "{d:?}");
    let d = only(
        "fn main() {\n    let f = |mut p| {\n        p.1 = 2\n    }\n}\n",
        "E0011",
    );
    assert!(d.message.contains("`.1`"), "{d:?}");
}

#[test]
fn an_element_is_written_through_a_mutable_binding() {
    clean(
        "fn main() {\n    let mut t = (1, \"a\")\n    t.0 = 2\n    t.0 += 3\n    t.1 += \"b\"\n    \
         let mut n = ((1, 2), 3)\n    n.0.1 = 4\n}\n",
    );
    let d = only("fn main() {\n    let t = (1, 2)\n    t.0 = 3\n}\n", "E0060");
    assert!(d.message.contains("`t`"), "{d:?}");
    let d = only(
        "fn main() {\n    let mut t = (1, 2)\n    t.0 = \"s\"\n}\n",
        "E0010",
    );
    assert!(d.message.contains("field `0` has type `Int`"), "{d:?}");
    let d = only(
        "fn main() {\n    let mut t = (1, 2)\n    t.5 = 1\n}\n",
        "E0014",
    );
    assert_eq!(d.message, "no field `5` on `(Int, Int)`");
}

// --- No impls (decision 3) ---

#[test]
fn equality_on_tuples_is_e0013_with_a_note() {
    let d = only(
        "fn main() {\n    let a = (1, 2)\n    let b = a == (1, 2)\n}\n",
        "E0013",
    );
    assert!(d.message.contains("`(Int, Int)`"), "{d:?}");
    assert_eq!(d.notes, [hir::TUPLE_HAS_NO_IMPLS]);
}

#[test]
fn interpolating_a_tuple_is_e0013_with_a_note() {
    let d = only(
        "fn main() {\n    let a = (1, 2)\n    println(\"${a}\")\n}\n",
        "E0013",
    );
    assert!(d.message.contains("`(Int, Int)`"), "{d:?}");
    assert_eq!(d.notes, [hir::TUPLE_HAS_NO_IMPLS]);
}

#[test]
fn equality_on_a_record_has_no_tuple_note() {
    let d = only(
        "record P { x: Int }\nfn main() {\n    let b = P { x: 1 } == P { x: 1 }\n}\n",
        "E0013",
    );
    assert!(d.notes.is_empty(), "{d:?}");
}

#[test]
fn no_impl_can_name_a_tuple() {
    let all = errors(
        "trait Show { fn show(self) -> String }\n\
         impl Show for (Int, Int) { fn show(self) -> String { \"p\" } }\n\
         impl (Int, Bool) { fn f(self) -> Int { 1 } }\nfn main() {}\n",
    );
    let codes: Vec<&str> = all.iter().map(|d| d.code.as_str()).collect();
    assert_eq!(codes, ["E0010", "E0010"], "{all:#?}");
}

#[test]
fn a_method_on_a_tuple_is_e0014() {
    let d = only(
        "fn main() {\n    let t = (1, 2)\n    let n = t.len()\n}\n",
        "E0014",
    );
    assert_eq!(d.message, "no method `len` on type `(Int, Int)`");
}

#[test]
fn extern_c_refuses_a_tuple() {
    let all = errors("extern \"C\" {\n    fn f(p: (Int, Int)) -> Int\n}\nfn main() {}\n");
    assert_eq!(all.len(), 1, "{all:#?}");
    assert_eq!(all[0].code, "E0900");
    assert!(all[0].message.contains("`(Int, Int)`"), "{:?}", all[0]);
}

// --- Display ---

#[test]
fn a_tuple_displays_as_written() {
    let defs = nova_resolver::Definitions::default();
    let show = |t: &Ty| crate::display_ty(t, &defs);
    assert_eq!(show(&Ty::Tuple(vec![Ty::Int, Ty::String])), "(Int, String)");
    assert_eq!(show(&Ty::Tuple(vec![Ty::Int])), "(Int,)");
    assert_eq!(
        show(&Ty::Tuple(vec![
            Ty::Tuple(vec![Ty::Bool]),
            Ty::Array(Box::new(Ty::Char))
        ])),
        "((Bool,), [Char])"
    );
    let named = crate::display_ty_named(
        &Ty::Tuple(vec![Ty::Param(0), Ty::Var(3)]),
        &defs,
        &["K".to_string()],
    );
    assert_eq!(named, "(K, _)");
}
