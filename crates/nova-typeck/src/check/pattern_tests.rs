//! The checker's tests of 5.0b's patterns (spec
//! `docs/superpowers/specs/2026-10-26-phase-5-0-the-language-and-std-a-compiler-needs-design.md`
//! §4.3, §4.4): nested, tuple, or-, `Char` and range patterns, guards,
//! `let` and `for` destructuring, and the codes they meet: E0001, E0010,
//! E0016, E0020, E0021, E0022, E0100, E0101 and E0104.

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

fn of(src: &str, severity: Severity) -> Vec<Diagnostic> {
    checked(src)
        .diagnostics
        .into_iter()
        .filter(|d| d.severity == severity)
        .collect()
}

/// The one error of `src`, which has `code`.
fn only(src: &str, code: &str) -> Diagnostic {
    let all = of(src, Severity::Error);
    assert_eq!(all.len(), 1, "one error in {all:#?}");
    assert_eq!(all[0].code, code, "{all:#?}");
    all[0].clone()
}

/// The one warning of `src`, which has no error; it has `code`.
fn only_warning(src: &str, code: &str) -> Diagnostic {
    let errors = of(src, Severity::Error);
    assert!(errors.is_empty(), "{errors:#?}");
    let all = of(src, Severity::Warning);
    assert_eq!(all.len(), 1, "one warning in {all:#?}");
    assert_eq!(all[0].code, code, "{all:#?}");
    all[0].clone()
}

fn clean(src: &str) {
    let all = checked(src).diagnostics;
    assert!(all.is_empty(), "{all:#?}");
}

/// The text of `src` under the primary label of `d`.
fn labelled<'s>(src: &'s str, d: &Diagnostic) -> &'s str {
    let label = d
        .labels
        .iter()
        .find(|l| l.primary)
        .expect("a primary label");
    &src[label.span.start as usize..label.span.end as usize]
}

fn label_text(d: &Diagnostic) -> String {
    d.labels
        .iter()
        .find(|l| l.primary)
        .expect("a primary label")
        .message
        .clone()
}

/// The checked arms' patterns of the first `match` in `name`.
fn patterns_of(r: &CheckResult, name: &str) -> Vec<hir::Pattern> {
    fn walk<'e>(e: &'e hir::Expr, out: &mut Vec<&'e hir::Expr>) {
        out.push(e);
        for c in child_exprs(e) {
            walk(c, out);
        }
    }
    let f = r
        .module
        .functions
        .iter()
        .find(|f| f.name == name)
        .expect("the function");
    let mut all = Vec::new();
    walk(&f.body, &mut all);
    all.into_iter()
        .find_map(|e| match &e.kind {
            hir::ExprKind::Match { arms, .. } => {
                Some(arms.iter().map(|a| a.pattern.clone()).collect())
            }
            _ => None,
        })
        .expect("a match")
}

const SUMS: &str = "type Opt<T> =\n  | Yes(T)\n  | No\n\
                    type Res<T, E> =\n  | Win(T)\n  | Fail(E)\n\
                    type Tok =\n  | Ident(String)\n  | Num(Int)\n  | Sym(Char)\n  | Eof\n\
                    type Wrap =\n  | Wrap(Int, String)\n";

fn with_sums(body: &str) -> String {
    format!("{SUMS}{body}\nfn main() {{}}\n")
}

// --- The HIR pattern ---

#[test]
fn nested_patterns_check_to_a_recursive_hir_pattern() {
    let src = with_sums(
        "fn f(o: Opt<Res<(Int, Tok), String>>) -> Int {\n    match o {\n        \
         Yes(Win((n, Tok::Num(0 | 1)))) => n\n        Yes(Win((_, Tok::Sym('a'..='z')))) => 1\n        \
         Yes(Win((5..10, _))) => 2\n        Yes(Win(_)) => 3\n        Yes(Fail(\"x\")) => 4\n        \
         Yes(Fail(_)) | No => 5\n    }\n}",
    );
    let r = checked(&src);
    assert!(r.diagnostics.is_empty(), "{:#?}", r.diagnostics);
    let pats = patterns_of(&r, "f");
    let hir::Pattern::Variant { fields, .. } = &pats[0] else {
        panic!("{:?}", pats[0])
    };
    let hir::Pattern::Variant { fields: inner, .. } = &fields[0] else {
        panic!("{fields:?}")
    };
    let hir::Pattern::Tuple(elems) = &inner[0] else {
        panic!("{inner:?}")
    };
    assert!(matches!(elems[0], hir::Pattern::Bind(_)), "{elems:?}");
    let hir::Pattern::Variant { fields: num, .. } = &elems[1] else {
        panic!("{elems:?}")
    };
    assert_eq!(
        num[0],
        hir::Pattern::Or(vec![hir::Pattern::LitInt(0), hir::Pattern::LitInt(1)])
    );
    // A `Char` range is stored by code point, an exclusive `Int` range with
    // its end included.
    let range = |p: &hir::Pattern| -> hir::Pattern {
        let hir::Pattern::Variant { fields, .. } = p else {
            panic!("{p:?}")
        };
        let hir::Pattern::Variant { fields, .. } = &fields[0] else {
            panic!("{fields:?}")
        };
        let hir::Pattern::Tuple(elems) = &fields[0] else {
            panic!("{fields:?}")
        };
        elems
            .iter()
            .find(|e| !matches!(e, hir::Pattern::Wildcard))
            .cloned()
            .expect("an element")
    };
    let hir::Pattern::Variant { fields: sym, .. } = range(&pats[1]) else {
        panic!()
    };
    assert_eq!(
        sym[0],
        hir::Pattern::Range {
            lo: 'a' as i64,
            hi: 'z' as i64
        }
    );
    assert_eq!(range(&pats[2]), hir::Pattern::Range { lo: 5, hi: 9 });
    assert!(matches!(pats[5], hir::Pattern::Or(_)), "{:?}", pats[5]);
}

#[test]
fn an_or_pattern_binds_one_local_in_every_alternative() {
    let src = with_sums("fn f(p: (Int, Int)) -> Int {\n    match p {\n        (a, 0) | (0, a) => a\n        _ => 0\n    }\n}");
    let r = checked(&src);
    assert!(r.diagnostics.is_empty(), "{:#?}", r.diagnostics);
    let pats = patterns_of(&r, "f");
    let hir::Pattern::Or(alts) = &pats[0] else {
        panic!("{:?}", pats[0])
    };
    assert_eq!(alts[0].bindings(), alts[1].bindings());
    assert_eq!(alts[0].bindings().len(), 1);
}

#[test]
fn a_char_literal_pattern_checks() {
    clean(&with_sums(
        "fn f(c: Char) -> Int {\n    match c {\n        'a' => 1\n        '\\n' | '\\t' => 2\n        _ => 3\n    }\n}",
    ));
}

#[test]
fn guards_see_the_arms_bindings_and_must_be_bool() {
    clean(&with_sums(
        "fn f(t: Tok) -> Int {\n    match t {\n        Num(n) if n > 0 => n\n        \
         Ident(s) if s == \"x\" => 1\n        _ => 0\n    }\n}",
    ));
    let src = with_sums(
        "fn f(n: Int) -> Int {\n    match n {\n        x if x => 1\n        _ => 0\n    }\n}",
    );
    let d = only(&src, "E0010");
    assert!(d.message.contains("a match guard"), "{d:?}");
    assert_eq!(labelled(&src, &d), "x");
}

// --- E0100 ---

#[test]
fn an_alternative_missing_a_binding_is_e0100_naming_it() {
    let src = with_sums(
        "fn f(t: Res<Int, Int>) -> Int {\n    match t {\n        Win(x) | Fail(_) => 1\n    }\n}",
    );
    let d = only(&src, "E0100");
    assert_eq!(d.message, "the alternatives bind different names");
    assert!(label_text(&d).contains("`x`"), "{d:?}");
    assert_eq!(labelled(&src, &d), "Fail(_)");
}

#[test]
fn an_alternative_with_an_extra_binding_is_e0100_naming_it() {
    let src = with_sums(
        "fn f(t: Res<Int, Int>) -> Int {\n    match t {\n        Win(_) | Fail(y) => 1\n    }\n}",
    );
    let d = only(&src, "E0100");
    assert!(label_text(&d).contains("`y`"), "{d:?}");
    assert_eq!(labelled(&src, &d), "y");
}

#[test]
fn a_binding_at_two_types_is_e0100() {
    let src = with_sums("fn f(t: Res<Int, String>) -> Int {\n    match t {\n        Win(x) | Fail(x) => 1\n    }\n}");
    let d = only(&src, "E0100");
    assert!(d.message.contains("different types"), "{d:?}");
    assert!(label_text(&d).contains("`String`"), "{d:?}");
    assert!(label_text(&d).contains("`Int`"), "{d:?}");
}

#[test]
fn nested_or_patterns_bind_the_outer_first_alternatives_names() {
    clean(&with_sums(
        "fn f(t: (Opt<Res<Int, Int>>, Int)) -> Int {\n    match t {\n        \
         (Yes(Win(x) | Fail(x)), _) | (No, x) => x\n    }\n}",
    ));
    let src = with_sums(
        "fn f(t: Opt<Res<Int, Int>>) -> Int {\n    match t {\n        \
         Yes(Win(x) | Fail(x)) | No => 1\n    }\n}",
    );
    let d = only(&src, "E0100");
    assert_eq!(labelled(&src, &d), "No");
}

// --- E0101 ---

#[test]
fn an_empty_range_is_e0101() {
    for (pat, why) in [
        ("5..=4", "`5` is above `4`"),
        ("5..5", "`5..5` stops before its start"),
        ("9..2", "`9` is above `2`"),
    ] {
        let src = with_sums(&format!(
            "fn f(n: Int) -> Int {{\n    match n {{\n        {pat} => 1\n        _ => 0\n    }}\n}}"
        ));
        let d = only(&src, "E0101");
        assert_eq!(d.message, "this range matches nothing");
        assert_eq!(label_text(&d), why);
        assert_eq!(labelled(&src, &d), pat);
    }
    let src = with_sums(
        "fn f(c: Char) -> Int {\n    match c {\n        'z'..='a' => 1\n        _ => 0\n    }\n}",
    );
    let d = only(&src, "E0101");
    assert!(label_text(&d).contains("'z'"), "{d:?}");
}

#[test]
fn an_empty_range_does_not_make_the_arms_after_it_unreachable() {
    let src = with_sums(
        "fn f(n: Int) -> Int {\n    match n {\n        3..=1 => 1\n        2 => 2\n        _ => 0\n    }\n}",
    );
    only(&src, "E0101");
    assert!(of(&src, Severity::Warning).is_empty());
}

#[test]
fn a_one_value_range_is_not_empty() {
    clean(&with_sums(
        "fn f(n: Int) -> Int {\n    match n {\n        4..=4 => 1\n        5..6 => 2\n        _ => 0\n    }\n}",
    ));
}

// --- E0010 ---

#[test]
fn a_range_over_another_type_is_e0010() {
    let src = with_sums(
        "fn f(s: String) -> Int {\n    match s {\n        1..=3 => 1\n        _ => 0\n    }\n}",
    );
    let d = only(&src, "E0010");
    assert!(d.message.contains("`Int`"), "{d:?}");
    assert!(d.message.contains("`String`"), "{d:?}");
    let src = with_sums(
        "fn f(n: Int) -> Int {\n    match n {\n        'a'..='z' => 1\n        _ => 0\n    }\n}",
    );
    only(&src, "E0010");
}

#[test]
fn a_tuple_pattern_of_the_wrong_arity_or_type_is_e0010() {
    let src =
        with_sums("fn f(p: (Int, Int)) -> Int {\n    match p {\n        (a, b, c) => a\n    }\n}");
    let d = only(&src, "E0010");
    assert!(d.message.contains("3 elements"), "{d:?}");
    assert!(d.message.contains("`(Int, Int)`"), "{d:?}");
    let src = with_sums("fn f(n: Int) -> Int {\n    match n {\n        (a, b) => a\n    }\n}");
    let d = only(&src, "E0010");
    assert!(d.message.contains("`Int`"), "{d:?}");
}

#[test]
fn a_char_pattern_against_an_int_is_e0010() {
    let src = with_sums(
        "fn f(n: Int) -> Int {\n    match n {\n        'a' => 1\n        _ => 0\n    }\n}",
    );
    only(&src, "E0010");
}

// --- A bare name inside a variant (5.0b's one change of meaning) ---

#[test]
fn a_variant_name_inside_a_variant_matches_that_variant() {
    // Before 5.0b, `Yes(No)` bound a local named `No`; now it is the variant.
    let src = with_sums(
        "fn f(o: Opt<Opt<Int>>) -> Int {\n    match o {\n        Yes(No) => 0\n        \
         Yes(Yes(n)) => n\n        No => 1\n    }\n}",
    );
    let r = checked(&src);
    assert!(r.diagnostics.is_empty(), "{:#?}", r.diagnostics);
    let pats = patterns_of(&r, "f");
    let hir::Pattern::Variant { fields, .. } = &pats[0] else {
        panic!("{:?}", pats[0])
    };
    assert!(
        matches!(&fields[0], hir::Pattern::Variant { fields, .. } if fields.is_empty()),
        "{fields:?}"
    );
    // Without `Yes(Yes(n))` the match misses it: `No` inside covers no more.
    let src = with_sums(
        "fn f(o: Opt<Opt<Int>>) -> Int {\n    match o {\n        Yes(No) => 0\n        No => 1\n    }\n}",
    );
    let e = only(&src, "E0020");
    assert!(e.message.contains("Yes(Yes(_))"), "{e:#?}");
}

#[test]
fn another_types_variant_name_inside_a_variant_is_e0001_with_a_note() {
    let src = with_sums(
        "fn f(o: Opt<Int>) -> Int {\n    match o {\n        Yes(Eof) => 0\n        _ => 1\n    }\n}",
    );
    let e = only(&src, "E0001");
    assert_eq!(e.message, "`Eof` is not a variant of the matched type");
    assert_eq!(labelled(&src, &e), "Eof");
    assert_eq!(
        e.notes,
        vec![
            "a name inside a pattern that names a variant matches that variant; \
             to bind a value, use a lower-case name"
                .to_string()
        ]
    );
}

#[test]
fn a_lower_case_name_inside_a_variant_still_binds() {
    let src = with_sums(
        "fn f(o: Opt<Int>) -> Int {\n    match o {\n        Yes(x) => x\n        No => 0\n    }\n}",
    );
    let r = checked(&src);
    assert!(r.diagnostics.is_empty(), "{:#?}", r.diagnostics);
    let pats = patterns_of(&r, "f");
    let hir::Pattern::Variant { fields, .. } = &pats[0] else {
        panic!("{:?}", pats[0])
    };
    assert!(matches!(fields[0], hir::Pattern::Bind(_)), "{fields:?}");
}

#[test]
fn a_variant_with_fields_named_bare_inside_a_variant_is_e0016() {
    // `Yes(Num)` is the variant `Num`, which has a field: E0016, where before
    // 5.0b it bound a local named `Num`.
    let src = with_sums(
        "fn f(o: Opt<Tok>) -> Int {\n    match o {\n        Yes(Num) => 0\n        _ => 1\n    }\n}",
    );
    only(&src, "E0016");
}

#[test]
fn an_upper_case_binding_is_e0104() {
    // `Rde` names no variant, so it binds, and matches every value.
    let src = with_sums(
        "fn f(o: Opt<Int>) -> Int {\n    match o {\n        Yes(Rde) => Rde\n        No => 0\n    }\n}",
    );
    let d = only_warning(&src, "E0104");
    assert_eq!(
        d.message,
        "`Rde` binds a value: no variant of that name is in scope"
    );
    assert_eq!(labelled(&src, &d), "Rde");
    assert!(
        d.notes[0].contains("to bind a value, use a lower-case name"),
        "{d:?}"
    );
    // At the top of an arm, and in a destructuring `let`, too.
    let src = with_sums(
        "fn f(n: Int) -> Int {\n    let (A, b) = (n, 1)\n    match n {\n        0 => b\n        Whatever => Whatever + A\n    }\n}",
    );
    assert!(of(&src, Severity::Error).is_empty());
    let all = of(&src, Severity::Warning);
    let named: Vec<&str> = all.iter().map(|d| labelled(&src, d)).collect();
    assert_eq!(named, ["A", "Whatever"], "{all:#?}");
}

#[test]
fn a_lower_case_or_underscored_binding_or_a_real_variant_is_not_e0104() {
    clean(&with_sums(
        "fn f(o: Opt<Opt<Int>>) -> Int {\n    match o {\n        Yes(No) => 0\n        \
         Yes(Yes(x)) => x\n        No => 1\n    }\n}\n\
         fn g(o: Opt<Int>) -> Int {\n    match o {\n        Yes(_Upper) => 1\n        _ => 0\n    }\n}",
    ));
}

#[test]
fn an_upper_case_binding_in_an_or_pattern_warns_once() {
    // The second alternative binds the first one's local: one E0104.
    let src = with_sums(
        "fn f(r: Res<Int, Int>) -> Int {\n    match r {\n        Win(Q) | Fail(Q) => Q\n    }\n}",
    );
    assert!(of(&src, Severity::Error).is_empty());
    let all = of(&src, Severity::Warning);
    assert_eq!(all.len(), 1, "{all:#?}");
    assert_eq!(all[0].code, "E0104");
}

#[test]
fn e0104_on_a_constant_says_to_use_a_guard() {
    let src = with_sums(
        "const MAX: Int = 9\nfn f(o: Opt<Int>) -> Int {\n    match o {\n        Yes(MAX) => 1\n        _ => 0\n    }\n}",
    );
    let d = only_warning(&src, "E0104");
    assert_eq!(
        d.notes,
        ["a pattern cannot match a constant by name; use a guard: `n if n == MAX`"]
    );
    assert!(d.fixes.is_empty(), "{d:?}");
}

#[test]
fn e0104_near_a_variant_says_which() {
    let src = with_sums("fn f(t: Tok) -> Int {\n    match t {\n        Eot => 1\n    }\n}");
    let d = only_warning(&src, "E0104");
    assert!(d.notes.iter().any(|n| n == "did you mean `Eof`?"), "{d:?}");
    assert_eq!(d.fixes.len(), 1, "{d:?}");
    assert_eq!(d.fixes[0].title, "change `Eot` to `Eof`");
    // A name near no variant gets no guess.
    let src = with_sums(
        "fn f(o: Opt<Int>) -> Int {\n    match o {\n        Yes(Zebra) => Zebra\n        No => 0\n    }\n}",
    );
    let d = only_warning(&src, "E0104");
    assert_eq!(d.notes.len(), 1, "{d:?}");
    assert!(d.fixes.is_empty(), "{d:?}");
}

#[test]
fn an_erroneous_pattern_does_not_make_the_arms_after_it_unreachable() {
    // `Eof` is `Tok`'s, so the first arm is E0001 and stands in as `_`; the
    // arms after it are not reported unreachable for that.
    let src = with_sums(
        "fn f(o: Opt<Int>) -> Int {\n    match o {\n        Eof => 0\n        Yes(n) => n\n        No => 1\n    }\n}",
    );
    only(&src, "E0001");
    let warnings = of(&src, Severity::Warning);
    assert!(warnings.is_empty(), "{warnings:#?}");
}

#[test]
fn the_forms_still_out_are_e0900() {
    for (pat, what) in [
        ("x @ 1", "`@` patterns"),
        ("1.5", "`Float` literal patterns"),
        ("[a, b]", "array patterns"),
    ] {
        let src = with_sums(&format!(
            "fn f(n: Int) -> Int {{\n    match n {{\n        {pat} => 1\n        _ => 0\n    }}\n}}"
        ));
        let errors = of(&src, Severity::Error);
        assert!(
            errors
                .iter()
                .any(|d| d.code == "E0900" && d.message.starts_with(what)),
            "{pat}: {errors:#?}"
        );
    }
}

// --- Exhaustiveness and reachability (spec §4.4) ---

#[test]
fn a_missing_nested_variant_is_e0020_with_a_nested_witness() {
    let src = with_sums("fn f(o: Opt<Res<Int, String>>) -> Int {\n    match o {\n        Yes(Win(_)) => 1\n        No => 2\n    }\n}");
    let d = only(&src, "E0020");
    assert!(d.message.contains("`Yes(Fail(_))`"), "{d:?}");
}

#[test]
fn a_tuple_of_sums_missing_a_combination_is_e0020() {
    let src = with_sums(
        "fn f(p: (Opt<Int>, Bool)) -> Int {\n    match p {\n        (Yes(_), _) => 1\n        (No, true) => 2\n    }\n}",
    );
    let d = only(&src, "E0020");
    assert!(d.message.contains("`(No, false)`"), "{d:?}");
}

#[test]
fn a_tuple_of_one_witness_keeps_its_comma() {
    let src = with_sums("fn f(p: (Bool,)) -> Int {\n    match p {\n        (true,) => 1\n    }\n}");
    let d = only(&src, "E0020");
    assert!(d.message.contains("`(false,)`"), "{d:?}");
}

#[test]
fn an_or_pattern_covers_each_alternative() {
    clean(&with_sums(
        "fn f(o: Opt<Res<Int, Int>>) -> Int {\n    match o {\n        Yes(Win(_) | Fail(_)) => 1\n        No => 2\n    }\n}",
    ));
}

#[test]
fn a_guarded_arm_covers_nothing() {
    let src = with_sums(
        "fn f(o: Opt<Int>) -> Int {\n    match o {\n        Yes(n) if n > 0 => n\n        No => 0\n    }\n}",
    );
    let d = only(&src, "E0020");
    assert!(d.message.contains("`Yes(_)`"), "{d:?}");
}

#[test]
fn literals_and_ranges_need_a_catch_all() {
    let src = with_sums("fn f(c: Char) -> Int {\n    match c {\n        'a'..='z' => 1\n        'A'..='Z' => 2\n    }\n}");
    let d = only(&src, "E0020");
    assert!(d.message.contains("`_`"), "{d:?}");
    let src = with_sums(
        "fn f(t: Tok) -> Int {\n    match t {\n        Num(0..=9) => 1\n        Ident(\"x\") => 2\n        Sym(_) | Eof => 3\n    }\n}",
    );
    let d = only(&src, "E0020");
    assert!(d.message.contains("`Ident(_)`"), "{d:?}");
    assert!(d.message.contains("`Num(_)`"), "{d:?}");
}

#[test]
fn a_nested_arm_an_earlier_arm_covers_is_unreachable() {
    let src = with_sums(
        "fn f(o: Opt<Opt<Int>>) -> Int {\n    match o {\n        Yes(_) => 1\n        Yes(Yes(1)) => 2\n        No => 3\n    }\n}",
    );
    let d = only_warning(&src, "E0021");
    assert_eq!(labelled(&src, &d), "Yes(Yes(1))");
}

#[test]
fn the_same_literal_or_range_again_is_unreachable_but_an_overlap_is_not() {
    let src = with_sums("fn f(n: Int) -> Int {\n    match n {\n        1..=5 => 1\n        1..=5 => 2\n        _ => 3\n    }\n}");
    let d = only_warning(&src, "E0021");
    assert_eq!(labelled(&src, &d), "1..=5");
    // Ranges are not split (5.0 spec §4.4): a value an earlier range
    // covers is not reported.
    clean(&with_sums(
        "fn f(n: Int) -> Int {\n    match n {\n        1..=5 => 1\n        3 => 2\n        2..=4 => 3\n        _ => 4\n    }\n}",
    ));
}

#[test]
fn an_unreachable_alternative_is_e0021_on_it_alone() {
    let src = with_sums(
        "fn f(n: Int) -> Int {\n    match n {\n        0 | 1 | 0 => 1\n        _ => 2\n    }\n}",
    );
    let d = only_warning(&src, "E0021");
    assert_eq!(d.message, "unreachable alternative");
    let label = d.labels.iter().find(|l| l.primary).unwrap();
    assert_eq!(
        label.span.start as usize,
        src.find("0 | 1 | 0").unwrap() + 8
    );
    // One an earlier arm covers, at depth.
    let src = with_sums(
        "fn f(o: Opt<Int>) -> Int {\n    match o {\n        Yes(1) => 1\n        Yes(2 | 1) => 2\n        _ => 3\n    }\n}",
    );
    let d = only_warning(&src, "E0021");
    assert_eq!(labelled(&src, &d), "1");
    // An alternative after a catch-all one.
    let src = with_sums("fn f(o: Opt<Int>) -> Int {\n    match o {\n        Yes(_) | Yes(3) => 1\n        No => 2\n    }\n}");
    let d = only_warning(&src, "E0021");
    assert_eq!(labelled(&src, &d), "Yes(3)");
}

#[test]
fn an_alternative_inside_one_reported_is_not_reported_again() {
    let src = with_sums(
        "fn f(o: Opt<Int>) -> Int {\n    match o {\n        Yes(_) | Yes(1 | 2) => 1\n        No => 2\n    }\n}",
    );
    let d = only_warning(&src, "E0021");
    assert_eq!(labelled(&src, &d), "Yes(1 | 2)");
}

#[test]
fn a_whole_unreachable_arm_reports_the_arm_not_its_alternatives() {
    let src = with_sums(
        "fn f(n: Int) -> Int {\n    match n {\n        _ => 1\n        2 | 3 => 2\n    }\n}",
    );
    let d = only_warning(&src, "E0021");
    assert_eq!(d.message, "unreachable match arm");
}

#[test]
fn a_generic_sum_matched_at_depth_is_exhaustive() {
    clean(
        "type Tree<T> =\n  | Leaf(T)\n  | Node(Tree<T>, Tree<T>)\n\
         fn f<T>(t: Tree<T>) -> Int {\n    match t {\n        Leaf(_) => 1\n        \
         Node(Leaf(_), _) => 2\n        Node(Node(_, _), _) => 3\n    }\n}\nfn main() {}\n",
    );
}

// --- `let` and `for` ---

#[test]
fn let_destructures_a_tuple_and_a_one_variant_sum() {
    let src = with_sums(
        "fn f() -> Int {\n    let (a, (b, _)) = (1, (2, \"x\"))\n    let Wrap(n, s) = Wrap(3, \"y\")\n    \
         let mut (c, d) = (4, 5)\n    c += d\n    a + b + n + c + s.len()\n}",
    );
    clean(&src);
}

#[test]
fn a_destructured_let_binding_is_immutable_without_mut() {
    let src = with_sums("fn f() -> Int {\n    let (a, b) = (1, 2)\n    a = 3\n    a + b\n}");
    let d = only(&src, "E0060");
    // The note says how a pattern's binding is made mutable.
    assert!(
        d.notes.iter().any(|n| n.contains("`let mut (…) = …`")),
        "{d:?}"
    );
}

#[test]
fn a_match_arm_or_a_loop_variable_is_rebound_to_be_mutable() {
    // `let mut (…)` is a `let`'s alone: an arm's binding is rebound.
    let src = with_sums(
        "fn f(o: Opt<Int>) -> Int {\n    match o {\n        Yes(z) => {\n            z = 1\n            z\n        }\n        No => 0\n    }\n}",
    );
    let d = only(&src, "E0060");
    assert_eq!(
        d.notes,
        [
            "`z` is bound by a pattern, whose bindings are immutable: rebind it as \
          `let mut z = z` to allow assignment"
        ]
    );
    // `for mut x` does not parse: the loop's variable is rebound in the body.
    let src = with_sums("fn f() {\n    for x in 0..3 {\n        x = 2\n    }\n}");
    let d = only(&src, "E0060");
    assert_eq!(
        d.notes,
        [
            "a `for` loop's variable is immutable: rebind it inside the loop's body as \
          `let mut x = x` to allow assignment"
        ]
    );
}

#[test]
fn a_let_pattern_with_an_error_is_not_also_e0022() {
    // `Eof`, another type's variant, is E0001 and becomes `_`, which would
    // make `Yes(_)` refutable.
    let src = with_sums("fn f(o: Opt<Int>) {\n    let Yes(Eof) = o\n}");
    let errors = of(&src, Severity::Error);
    let codes: Vec<&str> = errors.iter().map(|d| d.code.as_str()).collect();
    assert!(!codes.contains(&"E0022"), "{errors:#?}");
    assert!(!codes.is_empty(), "{errors:#?}");
}

#[test]
fn a_refutable_let_pattern_is_e0022_with_the_values_it_misses() {
    let src = with_sums("fn f(o: Opt<Int>) -> Int {\n    let Yes(x) = o\n    x\n}");
    let d = only(&src, "E0022");
    assert_eq!(d.message, "refutable pattern in `let`");
    assert!(label_text(&d).contains("`No`"), "{d:?}");
    let src = with_sums("fn f(p: (Int, Bool)) -> Int {\n    let (n, true) = p\n    n\n}");
    let d = only(&src, "E0022");
    assert!(label_text(&d).contains("`(_, false)`"), "{d:?}");
    // A literal's column names a value it misses, not `_`.
    let src = with_sums("fn f(p: (Int, Int)) -> Int {\n    let (a, 5) = p\n    a\n}");
    let d = only(&src, "E0022");
    assert!(label_text(&d).contains("`(_, 0)`"), "{d:?}");
}

#[test]
fn a_destructured_let_is_a_one_arm_match() {
    let r = checked(&with_sums(
        "fn f() -> Int {\n    let (a, b) = (1, 2)\n    a + b\n}",
    ));
    assert!(r.diagnostics.is_empty(), "{:#?}", r.diagnostics);
    let pats = patterns_of(&r, "f");
    assert_eq!(pats.len(), 1);
    assert!(matches!(&pats[0], hir::Pattern::Tuple(e) if e.len() == 2));
}

#[test]
fn a_tuple_pattern_over_a_range_is_e0010() {
    let src = with_sums("fn f() {\n    for (a, b) in 0..3 {\n        let c = a\n    }\n}");
    only(&src, "E0010");
}

// --- The newline rule and tuple patterns (5.0 spec §3.5) ---

#[test]
fn a_tuple_pattern_arm_after_a_bare_return_or_an_ordinary_arm_checks() {
    clean(&with_sums(
        "fn f(p: (Int, Int)) -> Int {\n    match p {\n        (0, 0) => return 0\n        (1, n) => n\n        \
         (a, b) => a + b\n    }\n}\nfn g(p: (Int, Int)) {\n    match p {\n        (0, _) => return\n        \
         (a, b) => {}\n    }\n}",
    ));
}
