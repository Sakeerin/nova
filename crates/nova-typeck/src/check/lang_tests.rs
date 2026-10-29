//! The checker's tests of 5.0a's language (spec
//! `docs/superpowers/specs/2026-10-26-phase-5-0-the-language-and-std-a-compiler-needs-design.md`
//! §3.3 to §3.6): `?` and its codes E0098 and E0099, `+=` on fields and
//! elements, `String + String`, and E0103.

use super::*;
use nova_diagnostics::{FileId, Severity};
use nova_lexer::lex;
use nova_parser::parse;
use nova_resolver::resolve;

/// Every diagnostic of `src`, the resolver's then the checker's.
fn diags(src: &str) -> Vec<Diagnostic> {
    let file_id = FileId::DUMMY;
    let (tokens, lex_errors) = lex(src, file_id);
    assert!(lex_errors.is_empty(), "lex errors: {lex_errors:?}");
    let (ast, parse_errors) = parse(&tokens, file_id);
    assert!(parse_errors.is_empty(), "parse errors: {parse_errors:?}");
    let resolved = resolve(&ast.expect("an AST"));
    let checked = check(&resolved.file, &resolved.definitions);
    let mut all = resolved.diagnostics;
    all.extend(checked.diagnostics);
    all
}

fn codes(src: &str) -> Vec<String> {
    diags(src).into_iter().map(|d| d.code).collect()
}

/// The one diagnostic of `src` with `code`.
fn only(src: &str, code: &str) -> Diagnostic {
    let all = diags(src);
    let hits: Vec<&Diagnostic> = all.iter().filter(|d| d.code == code).collect();
    assert_eq!(hits.len(), 1, "one {code} in {all:#?}");
    hits[0].clone()
}

fn clean(src: &str) {
    let all = diags(src);
    assert!(all.is_empty(), "{all:#?}");
}

// --- `?` ---

#[test]
fn try_on_option_in_an_option_function_checks() {
    clean(
        "fn first(s: String) -> Option<String> {\n    let i = s.index_of(\" \")?\n    Some(s.slice(0, i))\n}\nfn main() {}\n",
    );
}

#[test]
fn try_on_result_rewraps_a_different_ok_type() {
    clean(
        "fn p(s: String) -> Result<Int, String> { Ok(s.len()) }\n\
         fn q(s: String) -> Result<Bool, String> {\n    let n = p(s)?\n    Ok(n > 1)\n}\nfn main() {}\n",
    );
}

#[test]
fn try_gives_the_payload_type() {
    // `?` on `Option<Int>` is an `Int`: adding a `String` to it is E0010.
    let d = only(
        "fn f(o: Option<Int>) -> Option<Int> {\n    let s: String = o?\n    Some(1)\n}\nfn main() {}\n",
        "E0010",
    );
    assert!(d.message.contains("String"), "{d:?}");
}

#[test]
fn try_on_another_type_is_e0098_naming_it() {
    let d = only(
        "fn f(x: Int) -> Option<Int> {\n    let y = x?\n    Some(y)\n}\nfn main() {}\n",
        "E0098",
    );
    assert_eq!(d.message, "`?` needs an `Option` or a `Result`");
    assert!(d.notes.iter().any(|n| n.contains("`Int`")), "{d:?}");
}

#[test]
fn try_on_the_programs_own_option_is_e0098() {
    let d = only(
        "type Option<T> =\n    | Some(T)\n    | None\n\
         fn f(o: Option<Int>) -> Option<Int> {\n    let v = o?\n    Some(v)\n}\nfn main() {}\n",
        "E0098",
    );
    assert!(d.notes.iter().any(|n| n.contains("program's own")), "{d:?}");
}

#[test]
fn try_in_a_function_that_returns_neither_is_e0099() {
    let d = only(
        "fn f(o: Option<Int>) -> Int {\n    let v = o?\n    v\n}\nfn main() {}\n",
        "E0099",
    );
    assert_eq!(d.message, "`?` cannot return from this function");
    assert!(d.notes.iter().any(|n| n.contains("returns `Int`")), "{d:?}");
}

#[test]
fn try_on_option_in_a_result_function_suggests_ok_or() {
    let d = only(
        "fn f(o: Option<Int>) -> Result<Int, String> {\n    let v = o?\n    Ok(v)\n}\nfn main() {}\n",
        "E0099",
    );
    assert!(d.notes.iter().any(|n| n.contains(".ok_or(err)?")), "{d:?}");
}

#[test]
fn try_on_result_in_an_option_function_is_e0099_without_ok_or() {
    let d = only(
        "fn f(r: Result<Int, String>) -> Option<Int> {\n    let v = r?\n    Some(v)\n}\nfn main() {}\n",
        "E0099",
    );
    assert!(!d.notes.iter().any(|n| n.contains("ok_or")), "{d:?}");
}

#[test]
fn try_in_main_notes_that_main_returns_unit() {
    let d = only(
        "fn main() {\n    let o: Option<Int> = Some(1)\n    let v = o?\n    println(\"${v}\")\n}\n",
        "E0099",
    );
    assert!(
        d.notes.iter().any(|n| n.contains("`main` and `@test`")),
        "{d:?}"
    );
}

#[test]
fn try_in_a_test_function_notes_that_it_returns_unit() {
    let d = only(
        "@test\nfn t() {\n    let o: Option<Int> = Some(1)\n    let v = o?\n    assert(v == 1)\n}\nfn main() {}\n",
        "E0099",
    );
    assert!(
        d.notes.iter().any(|n| n.contains("`main` and `@test`")),
        "{d:?}"
    );
}

#[test]
fn try_in_a_closure_inside_main_has_no_entry_note() {
    let d = only(
        "fn main() {\n    let f = |o: Option<Int>| -> Int {\n        let v = o?\n        v\n    }\n    println(\"${f(Some(1))}\")\n}\n",
        "E0099",
    );
    assert!(!d.notes.iter().any(|n| n.contains("`main`")), "{d:?}");
}

#[test]
fn try_with_different_error_types_is_e0010_pointing_at_map_err() {
    let d = only(
        "fn p() -> Result<Int, Int> { Ok(1) }\n\
         fn f() -> Result<Int, String> {\n    let v = p()?\n    Ok(v)\n}\nfn main() {}\n",
        "E0010",
    );
    assert!(d.notes.iter().any(|n| n.contains("map_err")), "{d:?}");
}

#[test]
fn map_err_then_try_checks() {
    clean(
        "fn p() -> Result<Int, Int> { Ok(1) }\n\
         fn f() -> Result<Int, String> {\n    let v = p().map_err(|e: Int| \"code ${e}\")?\n    Ok(v)\n}\nfn main() {}\n",
    );
}

#[test]
fn try_infers_a_closures_return_type() {
    // The closure's return type is unknown until `?` makes it an `Option`;
    // `Some(w.len())` then fixes its argument.
    clean(
        "fn first(s: String) -> Option<String> { Some(s) }\n\
         fn main() {\n    let f = |s: String| {\n        let w = first(s)?\n        Some(w.len())\n    }\n    \
         let n: Option<Int> = f(\"a\")\n    println(\"${n.is_some()}\")\n}\n",
    );
}

#[test]
fn try_in_a_closure_returns_from_the_closure_not_the_function() {
    // The enclosing function returns `()`; the closure returns an `Option`.
    clean(
        "fn main() {\n    let f = |o: Option<Int>| -> Option<Int> {\n        let v = o?\n        Some(v + 1)\n    }\n    \
         println(\"${f(Some(1)).is_some()}\")\n}\n",
    );
}

#[test]
fn return_in_a_closure_checks_against_the_closures_type() {
    clean(
        "fn main() {\n    let f = |x: Int| -> Int {\n        if x > 0 {\n            return 1\n        }\n        2\n    }\n    \
         println(\"${f(1)}\")\n}\n",
    );
}

#[test]
fn try_in_an_async_fn_uses_its_declared_return_type() {
    clean(
        "async fn get() -> Option<Int> { Some(1) }\n\
         async fn f() -> Option<Int> {\n    let v = get().await?\n    Some(v)\n}\nfn main() {}\n",
    );
}

#[test]
fn try_in_a_function_returning_its_own_option_says_so() {
    let d = only(
        "type Option<T> =\n    | Some(T)\n    | None\n\
         fn f(s: String) -> Option<Int> {\n    let i = s.index_of(\"b\")?\n    Some(i)\n}\nfn main() {}\n",
        "E0099",
    );
    assert!(d.notes.iter().any(|n| n.contains("program's own")), "{d:?}");
}

#[test]
fn try_in_a_constant_is_e0099() {
    let d = only(
        "fn n() -> Option<Int> { None }\nconst A: Option<Int> = Some(n()? + 1)\nfn main() {}\n",
        "E0099",
    );
    assert!(
        d.notes
            .iter()
            .any(|n| n.contains("a constant is not a function")),
        "{d:?}"
    );
}

#[test]
fn try_in_a_closure_inside_a_constant_checks() {
    clean(
        "const F: fn(Option<Int>) -> Option<Int> = |o: Option<Int>| -> Option<Int> { Some(o? + 1) }\nfn main() {}\n",
    );
}

#[test]
fn try_desugars_to_a_match_in_hir() {
    let file_id = FileId::DUMMY;
    let src =
        "fn f(o: Option<Int>) -> Option<Int> {\n    let v = o?\n    Some(v)\n}\nfn main() {}\n";
    let (tokens, _) = lex(src, file_id);
    let (ast, _) = parse(&tokens, file_id);
    let resolved = resolve(&ast.expect("an AST"));
    let checked = check(&resolved.file, &resolved.definitions);
    assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
    let f = checked
        .module
        .functions
        .iter()
        .find(|f| f.name == "f")
        .expect("f");
    let hir::ExprKind::Block { stmts, .. } = &f.body.kind else {
        panic!("a block")
    };
    let hir::ExprKind::Let { init, .. } = &stmts[0].kind else {
        panic!("a let")
    };
    let hir::ExprKind::Match { arms, .. } = &init.kind else {
        panic!("`?` is a match: {:?}", init.kind)
    };
    assert_eq!(arms.len(), 2);
    assert!(matches!(arms[1].body.kind, hir::ExprKind::Return(Some(_))));
    assert_eq!(init.ty, Ty::Int);
}

// --- `+=` on fields and elements ---

#[test]
fn every_compound_operator_on_a_field_and_an_element_checks() {
    for op in ["+=", "-=", "*=", "/=", "%=", "&=", "|=", "^=", "<<=", ">>="] {
        clean(&format!(
            "record R {{ n: Int, a: [Int] }}\n\
             fn main() {{\n    let mut r = R {{ n: 1, a: [1, 2] }}\n    r.n {op} 3\n    r.a[1] {op} 2\n    println(\"${{r.n}}\")\n}}\n"
        ));
    }
}

#[test]
fn compound_assignment_on_nested_places_checks() {
    clean(
        "record G { cells: [[Int]] }\nrecord W { g: G }\n\
         fn main() {\n    let mut w = W { g: G { cells: [[1]] } }\n    w.g.cells[0][0] += 1\n    let mut a = [[1, 2], [3, 4]]\n    a[1][0] -= 1\n}\n",
    );
}

#[test]
fn compound_assignment_on_an_immutable_root_is_e0060() {
    let d = only(
        "record R { n: Int }\nfn main() {\n    let r = R { n: 1 }\n    r.n += 1\n}\n",
        "E0060",
    );
    assert!(d.message.contains("`r`"), "{d:?}");
    let d = only(
        "fn main() {\n    let a = [1, 2]\n    a[0] *= 2\n}\n",
        "E0060",
    );
    assert!(d.message.contains("`a`"), "{d:?}");
}

#[test]
fn compound_assignment_with_a_bad_operand_type_is_e0013_or_e0010() {
    assert!(codes(
        "record R { b: Bool }\nfn main() {\n    let mut r = R { b: true }\n    r.b += true\n}\n"
    )
    .contains(&"E0013".to_string()));
    assert!(codes(
        "record R { n: Int }\nfn main() {\n    let mut r = R { n: 1 }\n    r.n += \"x\"\n}\n"
    )
    .contains(&"E0010".to_string()));
}

#[test]
fn compound_assignment_evaluates_the_index_once() {
    // `a[f()] += 1` binds the index to a local: one call in the HIR.
    let file_id = FileId::DUMMY;
    let src = "fn f() -> Int { 0 }\nfn main() {\n    let mut a = [1]\n    a[f()] += 1\n}\n";
    let (tokens, _) = lex(src, file_id);
    let (ast, _) = parse(&tokens, file_id);
    let resolved = resolve(&ast.expect("an AST"));
    let checked = check(&resolved.file, &resolved.definitions);
    assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
    let main = checked
        .module
        .functions
        .iter()
        .find(|f| f.name == "main")
        .expect("main");
    let text = format!("{:?}", main.body);
    assert_eq!(text.matches("Call {").count(), 1, "{text}");
}

#[test]
fn compound_assignment_on_a_field_evaluates_the_receiver_once() {
    // `rs[f()].n += 1`: the receiver, `rs[f()]`, is bound to a local, so
    // its call is in the HIR once.
    let file_id = FileId::DUMMY;
    let src = "record R { n: Int }\nfn f() -> Int { 0 }\n\
               fn main() {\n    let mut rs = [R { n: 1 }]\n    rs[f()].n += 1\n}\n";
    let (tokens, _) = lex(src, file_id);
    let (ast, _) = parse(&tokens, file_id);
    let resolved = resolve(&ast.expect("an AST"));
    let checked = check(&resolved.file, &resolved.definitions);
    assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
    let main = checked
        .module
        .functions
        .iter()
        .find(|f| f.name == "main")
        .expect("main");
    let text = format!("{:?}", main.body);
    assert_eq!(text.matches("Call {").count(), 1, "{text}");
}

#[test]
fn string_compound_assignment_on_a_field_checks() {
    clean(
        "record R { s: String }\nfn main() {\n    let mut r = R { s: \"a\" }\n    r.s += \"b\"\n    let mut v = [\"x\"]\n    v[0] += \"y\"\n}\n",
    );
}

// --- `String + String` ---

#[test]
fn string_plus_string_is_a_string() {
    clean("fn main() {\n    let s: String = \"a\" + \"b\"\n    let mut t = s\n    t += \"c\"\n    println(t)\n}\n");
}

#[test]
fn string_plus_char_or_int_is_refused_with_a_note() {
    for rhs in ["'c'", "1"] {
        let d = only(
            &format!("fn main() {{\n    let s = \"a\" + {rhs}\n    println(s)\n}}\n"),
            "E0010",
        );
        assert!(d.notes.iter().any(|n| n.contains("interpolate")), "{d:?}");
    }
}

#[test]
fn string_minus_string_is_still_e0013() {
    assert!(codes("fn main() {\n    let s = \"a\" - \"b\"\n}\n").contains(&"E0013".to_string()));
}

// --- E0103 ---

fn e0103(body: &str) -> usize {
    diags(&format!(
        "fn g() -> Int {{ 1 }}\nfn main() {{\n    let x = 1\n    let y = 2\n{body}\n    println(\"${{x}}\")\n}}\n"
    ))
    .iter()
    .filter(|d| d.code == "E0103")
    .inspect(|d| assert_eq!(d.severity, Severity::Warning))
    .count()
}

#[test]
fn e0103_flags_each_form_whose_value_is_discarded() {
    for stmt in [
        "    -y",
        "    !true",
        "    x + y",
        "    x == y",
        "    1",
        "    \"text\"",
        "    y",
        "    (x)",
        "    (x, y)",
        "    [x, y]",
        "    (g())",
    ] {
        assert_eq!(e0103(stmt), 1, "{stmt}");
    }
}

#[test]
fn e0103_flags_what_the_newline_rule_leaves() {
    // `let z = x` then `-y`: two statements, the second discarded.
    assert_eq!(e0103("    let z = x\n    -y\n    println(\"${z}\")"), 1);
}

#[test]
fn e0103_is_not_on_calls_assignments_conditions_or_a_tail() {
    for stmt in [
        "    g()",
        "    let mut m = 1\n    m = 2\n    m += 1",
        "    x == 1 || y == 2",
        "    x == 1 && y == 2",
        "    if x > 0 { g() }",
        "    match x { _ => g() }",
        "    while false {}",
        "    { g() }",
    ] {
        assert_eq!(e0103(stmt), 0, "{stmt}");
    }
    // A block's tail is not a statement.
    let all = diags("fn f(x: Int) -> Int {\n    x\n}\nfn main() {}\n");
    assert!(all.is_empty(), "{all:?}");
}

#[test]
fn e0103_reports_a_parenthesised_callee_after_a_line_break() {
    // `x` then `(g)()`: `x` is an unused name. The line break that ends it
    // before `(g)()` is not reported: `x` is an `Int`, so the old reading,
    // `x((g))()`, could not have been well typed (the user's 5.0b ruling).
    assert_eq!(e0103("    x\n    (g)()"), 1);
    // After a function it could, so the split is reported too.
    assert_eq!(e0103("    let h = g\n    h\n    (g)()"), 2);
}

// --- E0103 at the newline rule's splits (the user's ruling of 2026-10-27) ---

/// Each E0103's message, for `src`.
fn e0103_messages(src: &str) -> Vec<String> {
    diags(src)
        .into_iter()
        .filter(|d| d.code == "E0103")
        .map(|d| d.message)
        .collect()
}

#[test]
fn a_split_after_a_return_value_warns() {
    let all = e0103_messages(
        "fn net(a: Int, b: Int) -> Int {\n    return a\n        - b\n}\nfn main() {}\n",
    );
    assert_eq!(all, ["a line break ends the statement before this `-`"]);
}

#[test]
fn a_split_before_a_tail_warns() {
    let all = e0103_messages(
        "record R { a: Int, b: Int }\nfn w(r: R) -> Int {\n    r.a\n    - r.b\n}\nfn main() {}\n",
    );
    assert_eq!(all, ["a line break ends the statement before this `-`"]);
    let all = e0103_messages(
        "fn k(x: Int) -> fn(Int) -> Int { |y: Int| x + y }\nfn main() {\n    let v = k(1)\n    (5)\n    println(\"${v(2)}\")\n}\n",
    );
    // `(5)` is also an unused value: one warning, with the line break's note.
    assert_eq!(all, ["this value is not used"]);
}

#[test]
fn a_split_after_a_bare_return_warns() {
    let all = e0103_messages("fn g() {}\nfn f() {\n    return\n    g()\n}\nfn main() {}\n");
    assert_eq!(all, ["a line break ends this `return` or `break`"]);
}

#[test]
fn a_closures_bare_return_before_a_parenthesis_or_bracket_warns_once() {
    // One split at the `(` or `[`, so one warning with one note, not the
    // `return`'s note and the call's or index's beside it.
    for (line, message) in [
        ("(5)", "this value is not used"),
        ("[1, 2].len()", "a line break ends this `return` or `break`"),
    ] {
        let all: Vec<Diagnostic> = diags(&format!(
            "fn main() {{\n    let h = || return\n    {line}\n    println(\"x\")\n}}\n"
        ))
        .into_iter()
        .filter(|d| d.code == "E0103")
        .collect();
        assert_eq!(all.len(), 1, "{line}: {all:?}");
        assert_eq!(all[0].message, message, "{line}");
        assert_eq!(all[0].notes.len(), 1, "{line}: {all:?}");
        assert!(all[0].notes[0].contains("takes no value"), "{all:?}");
    }
}

#[test]
fn a_written_semicolon_silences_the_split() {
    assert!(
        e0103_messages("fn net(a: Int, b: Int) -> Int {\n    a;\n    -b\n}\nfn main() {}\n")
            .iter()
            .all(|m| m != "a line break ends the statement before this `-`")
    );
    assert!(
        e0103_messages("fn g() {}\nfn f() {\n    return;\n    g()\n}\nfn main() {}\n").is_empty()
    );
}

#[test]
fn an_unused_value_at_a_split_gets_one_warning_with_the_line_breaks_note() {
    let all: Vec<Diagnostic> = diags(
        "fn main() {\n    let x = 1\n    let y = 2\n    let z = x\n    -y\n    println(\"${z}\")\n}\n",
    )
    .into_iter()
    .filter(|d| d.code == "E0103")
    .collect();
    assert_eq!(all.len(), 1, "{all:?}");
    assert_eq!(all[0].message, "this value is not used");
    assert!(
        all[0]
            .notes
            .iter()
            .any(|n| n.contains("a line break ended the statement")),
        "{all:?}"
    );
    // Without a line break, no such note.
    let all: Vec<Diagnostic> =
        diags("fn main() {\n    let y = 2\n    let z = 1; -y\n    println(\"${z}\")\n}\n")
            .into_iter()
            .filter(|d| d.code == "E0103")
            .collect();
    assert_eq!(all.len(), 1, "{all:?}");
    assert!(all[0].notes.is_empty(), "{all:?}");
}

// --- E0103 only where the old reading could have been well typed (the
// user's 5.0b ruling of 2026-10-27) ---

#[test]
fn a_tuple_tail_after_a_call_that_returns_no_function_is_quiet() {
    let all = e0103_messages(
        "fn g() -> Int { 1 }\nfn f() -> (Int, Int) {\n    g()\n    (1, 2)\n}\nfn main() {}\n",
    );
    assert!(all.is_empty(), "{all:?}");
    // A unit statement, as an `.await` or a `println` is.
    let all =
        e0103_messages("fn f() -> (Int, Int) {\n    println(\"x\")\n    (1, 2)\n}\nfn main() {}\n");
    assert!(all.is_empty(), "{all:?}");
}

#[test]
fn a_split_warns_only_after_a_value_the_old_reading_could_extend() {
    let split = |prev: &str, line: &str| -> Vec<String> {
        e0103_messages(&format!(
            "fn k() -> fn(Int) -> Int {{ |y: Int| y }}\n\
             fn f(n: Int, s: String, a: [Int], x: Float) -> Int {{\n    {prev}\n    {line}\n}}\nfn main() {{}}\n"
        ))
    };
    let call = "a line break ends the statement before this `(`";
    let index = "a line break ends the statement before this `[`";
    let minus = "a line break ends the statement before this `-`";
    // `(` after a function, and not after an `Int` or a `String`.
    assert_eq!(split("let h = k()", "(n)"), [call]);
    assert!(split("let m = n", "(n)").is_empty());
    assert!(split("let t = s", "(n)").is_empty());
    // `[` after an array, and not after an `Int`.
    assert_eq!(split("let b = a", "[n][0]"), [index]);
    assert!(split("let m = n", "[n][0]").is_empty());
    // `-` after an `Int` or a `Float`, and not after a `String`; the
    // operand is the rightmost one of a binary operator.
    assert_eq!(split("let m = n", "-n"), [minus]);
    assert_eq!(split("let y = x * 2.0", "-n"), [minus]);
    assert_eq!(split("let b = n == 1", "-n"), [minus]);
    assert!(split("let t = s", "-n").is_empty());
}

#[test]
fn a_split_after_a_bare_return_warns_whatever_comes_before() {
    let all =
        e0103_messages("fn g() -> Int { 1 }\nfn f() {\n    return\n    (g())\n}\nfn main() {}\n");
    assert!(
        all.iter()
            .any(|m| m == "this value is not used"
                || m == "a line break ends this `return` or `break`"),
        "{all:?}"
    );
}

#[test]
fn a_split_after_a_value_of_unknown_type_warns() {
    // `m`'s type is not known when the split is judged, so the old reading,
    // `make()(1, 2)`, could have been well typed.
    let all = e0103_messages(
        "fn make<T>() -> T {\n    panic(\"no\")\n}\n\
         fn f() -> (Int, Int) {\n    let m = make()\n    (1, 2)\n}\nfn main() {}\n",
    );
    assert_eq!(all, ["a line break ends the statement before this `(`"]);
}

#[test]
fn a_split_after_a_bare_return_before_a_tail_warns() {
    // No "this value is not used" here: `(g())` is the block's tail.
    let all = e0103_messages(
        "fn g() {}\nfn f(c: Bool) {\n    if c {\n        return\n        (g())\n    }\n}\nfn main() {}\n",
    );
    assert_eq!(all, ["a line break ends this `return` or `break`"]);
}

#[test]
fn the_split_operand_is_read_through_an_assignment() {
    // `q = k()` then the tail `(n)`: the old reading, `q = k()(n)`, calls a
    // function, so the split warns; after `m = n`, an `Int`, it does not.
    let src = |prev: &str| {
        format!(
            "fn k() -> fn(Int) -> Int {{\n    |y: Int| y\n}}\n\
             fn f(n: Int) -> Int {{\n    let mut q = k()\n    let mut m = 0\n    {prev}\n    (n)\n}}\nfn main() {{}}\n"
        )
    };
    assert_eq!(
        e0103_messages(&src("q = k()")),
        ["a line break ends the statement before this `(`"]
    );
    assert!(e0103_messages(&src("m = n")).is_empty());
}

#[test]
fn a_split_after_a_compound_assignment_to_an_element_or_a_field_warns() {
    // `a[0] -= 2` then `- 1`: the old reading subtracted 1 from the 2.
    let minus = "a line break ends the statement before this `-`";
    let src = |stmt: &str| {
        format!(
            "record R {{ x: Int }}\nfn f(mut a: [Int], mut r: R, mut t: (Int, (Int, Int))) -> Int {{\n    \
             {stmt}\n    - 1\n}}\nfn main() {{}}\n"
        )
    };
    for stmt in ["a[0] -= 2", "r.x += 2", "t.1.0 += 2"] {
        assert_eq!(e0103_messages(&src(stmt)), [minus], "{stmt}");
    }
}

#[test]
fn a_split_after_a_closure_reads_its_body() {
    // The old reading extended the closure's body: `|x: Int| x - 1`,
    // `|i: Int| arr[1]`, `|x: Int| g(3)`.
    let src = |prev: &str, line: &str| {
        format!(
            "fn g(x: Int) -> Int {{ x }}\nfn f(arr: [Int]) -> Int {{\n    {prev}\n    {line}\n}}\nfn main() {{}}\n"
        )
    };
    let warned = |prev: &str, line: &str, what: &str| {
        assert_eq!(
            e0103_messages(&src(prev, line)),
            [format!(
                "a line break ends the statement before this `{what}`"
            )],
            "{prev} / {line}"
        );
    };
    warned("let f = |x: Int| x", "- 1", "-");
    warned("let f = |i: Int| arr", "[1][0]", "[");
    warned("let f = |x: Int| g", "(3)", "(");
    // A body the old reading could not extend is quiet.
    assert!(e0103_messages(&src("let f = |x: Int| \"s\"", "- 1")).is_empty());
    // A bare function as a value is a function, not a body to read.
    warned("let h = g", "(3)", "(");
}
