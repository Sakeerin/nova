//! What the printer takes from the source, and the separator rule (spec
//! `docs/superpowers/specs/2026-10-07-phase-3-1-formatter-design.md` §5.3,
//! §5.6).

mod common;
use common::{assert_formats, assert_stable};

#[test]
fn literals_keep_their_spelling() {
    assert_formats(
        "fn main() { let a = 0xFF\n let b = 1_000_000\n let c = 1.5e3\n let d = 'x'\n let e = r\"raw\" }\n",
        "fn main() {\n    let a = 0xFF\n    let b = 1_000_000\n    let c = 1.5e3\n    let d = 'x'\n    let e = r\"raw\"\n}\n",
    );
}

#[test]
fn strings_are_printed_as_written() {
    assert_stable("fn main() { println(\"a ${ x  +  1 } b\") }\n");
}

#[test]
fn a_multi_line_string_is_never_reindented() {
    assert_formats(
        "fn main() {\n        let s = \"line one\n   line two\"\n        println(s)\n}\n",
        "fn main() {\n    let s = \"line one\n   line two\"\n    println(s)\n}\n",
    );
}

#[test]
fn crlf_input_comes_out_lf() {
    assert_formats(
        "fn main() {\r\n    let s = \"a\r\nb\"\r\n    println(s)\r\n}\r\n",
        "fn main() {\n    let s = \"a\nb\"\n    println(s)\n}\n",
    );
}

#[test]
fn the_authors_parentheses_are_kept() {
    assert_formats(
        "fn main() { let a = (b * c) + d\n let e = (p).hash()\n let f = ((g)) }\n",
        "fn main() {\n    let a = (b * c) + d\n    let e = (p).hash()\n    let f = (g)\n}\n",
    );
}

#[test]
fn a_parenthesised_literal_keeps_one_pair() {
    // A literal prints from its source text, which must not include the
    // parentheses around it.
    assert_formats(
        "fn main() { let a = ((1))\n let b = (\"s\")\n match a { (1) => 0, _ => 1 } }\n",
        "fn main() {\n    let a = (1)\n    let b = (\"s\")\n    match a {\n        (1) => 0\n        _ => 1\n    }\n}\n",
    );
}

#[test]
fn parentheses_the_parser_needs_are_kept() {
    assert_stable("fn main() { while (if m > 3 { false } else { true }) { m = m + 1 } }\n");
}

#[test]
fn a_parenthesised_tuple_loses_its_extra_pair() {
    assert_formats(
        "fn main() { let t = ((1, 2)) }\n",
        "fn main() { let t = (1, 2) }\n",
    );
}

#[test]
fn a_parenthesised_pattern_keeps_its_parentheses() {
    assert_formats(
        "fn main() { let (x) = 1\n println(\"${x}\") }\n",
        "fn main() {\n    let (x) = 1\n    println(\"${x}\")\n}\n",
    );
}

#[test]
fn tokens_that_would_merge_keep_a_space() {
    // In an expression `&` takes a postfix operand, so a reference to a
    // reference is written `&(&x)` and keeps its parentheses. A type has no
    // such rule, so `& &T` keeps its space.
    assert_formats(
        "fn main() { let f = | | 0\n let r = &(&x) }\n",
        "fn main() {\n    let f = | | 0\n    let r = &(&x)\n}\n",
    );
    assert_stable("fn f(x: & &Int, y: &mut &Int) {}\n");
}

#[test]
fn impl_members_keep_their_source_order() {
    assert_stable(
        "impl Shape for Square {\n    fn area(self) -> Float { 1.0 }\n    type Unit = Float\n    const SIDES: Int = 4\n    fn name(self) -> String { \"square\" }\n}\n",
    );
}

#[test]
fn a_function_type_prints_no_unit_return() {
    assert_formats(
        "fn f(g: fn(Int) -> ()) {}\nfn h(g: fn(Int)) {}\n",
        "fn f(g: fn(Int)) {}\n\nfn h(g: fn(Int)) {}\n",
    );
}

#[test]
fn a_record_pattern_ending_in_rest_takes_no_trailing_comma() {
    assert_formats(
        "fn f(p: Point) -> Int { match p { Point { first_coordinate, second_coordinate, third_coordinate, fourth_coordinate, .. } => first_coordinate } }\n",
        "fn f(p: Point) -> Int {\n    match p {\n        Point {\n            first_coordinate,\n            second_coordinate,\n            third_coordinate,\n            fourth_coordinate,\n            ..\n        } => first_coordinate\n    }\n}\n",
    );
}

#[test]
fn a_statement_starting_with_a_parenthesis_keeps_its_semicolon() {
    assert_formats(
        "fn main() { f(); (a, b).show() }\n",
        "fn main() {\n    f();\n    (a, b).show()\n}\n",
    );
}

#[test]
fn each_token_that_could_continue_a_statement_keeps_its_semicolon() {
    for (input, expected) in [
        (
            "fn main() { f(); [1, 2].len() }\n",
            "fn main() {\n    f();\n    [1, 2].len()\n}\n",
        ),
        (
            "fn main() { let x = y; { z } }\n",
            "fn main() {\n    let x = y;\n    { z }\n}\n",
        ),
        (
            "fn main() { let x = y; -z }\n",
            "fn main() {\n    let x = y;\n    -z\n}\n",
        ),
        (
            "fn main() { let x = y; *p = 1 }\n",
            "fn main() {\n    let x = y;\n    *p = 1\n}\n",
        ),
        (
            "fn main() { f(); &x }\n",
            "fn main() {\n    f();\n    &x\n}\n",
        ),
        (
            "fn main() { f(); |x| x }\n",
            "fn main() {\n    f();\n    |x| x\n}\n",
        ),
    ] {
        assert_formats(input, expected);
    }
}

#[test]
fn a_match_arm_starting_with_a_parenthesis_keeps_its_comma() {
    assert_formats(
        "fn main() { match p { a => 1, (b, c) => 2 } }\n",
        "fn main() {\n    match p {\n        a => 1,\n        (b, c) => 2\n    }\n}\n",
    );
}

#[test]
fn a_value_less_return_keeps_its_semicolon() {
    assert_formats(
        "fn f() { return; x = 1 }\n",
        "fn f() {\n    return;\n    x = 1\n}\n",
    );
    assert_formats(
        "fn main() { while true { break; x = 1 } }\n",
        "fn main() {\n    while true {\n        break;\n        x = 1\n    }\n}\n",
    );
}

#[test]
fn width_counts_characters_not_bytes() {
    // 4 + `report("` + 86 Thai letters + `")` is exactly 100 characters, and
    // 272 bytes.
    let thai = "ก".repeat(86);
    assert_stable(&format!(
        "fn main() {{\n    a()\n    report(\"{thai}\")\n}}\n"
    ));
}

#[test]
fn tab_indentation_becomes_spaces() {
    assert_formats(
        "fn main() {\n\tprintln(\"a\")\n\tprintln(\"b\")\n}\n",
        "fn main() {\n    println(\"a\")\n    println(\"b\")\n}\n",
    );
}

#[test]
fn an_empty_or_blank_input_formats_to_nothing() {
    assert_eq!(nova_fmt::format("").unwrap(), "");
    assert_eq!(nova_fmt::format("\n  \n\t\n").unwrap(), "");
}

#[test]
fn a_syntax_error_is_reported() {
    match nova_fmt::format("fn main( {\n") {
        Err(nova_fmt::FormatError::Syntax { rendered, .. }) => {
            assert!(rendered.contains("P0001"), "{rendered}")
        }
        other => panic!("{other:?}"),
    }
}
