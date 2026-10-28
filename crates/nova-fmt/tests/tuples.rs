//! Tuples in fmt (5.0 spec
//! `docs/superpowers/specs/2026-10-26-phase-5-0-the-language-and-std-a-compiler-needs-design.md`
//! §4.1, §4.2): tuple types and literals print as they did, and `t.N`, a
//! field named by its index, prints as written, so `t.1.0`, which the lexer
//! splits after a `.`, reads back as the same tree.

mod common;
use common::{assert_formats, assert_stable};

#[test]
fn tuple_fields_print_as_written() {
    assert_stable("fn f(t: ((Int, Int), Int)) -> Int { t.0.1 + t.1 }\n");
    assert_stable(
        "fn main() {\n    let n = t.0.1.2.3\n    let m = t.1.10\n    println(\"${n.0}\")\n}\n",
    );
}

#[test]
fn a_space_around_the_dot_of_a_tuple_field_goes() {
    assert_formats(
        "fn main() { let x = t . 0\n println(\"${x}\") }\n",
        "fn main() {\n    let x = t.0\n    println(\"${x}\")\n}\n",
    );
}

#[test]
fn a_tuple_of_one_keeps_its_comma() {
    assert_formats(
        "fn f(p: (Int ,)) -> (Int ,) { (p.0 ,) }\n",
        "fn f(p: (Int,)) -> (Int,) { (p.0,) }\n",
    );
}

#[test]
fn a_tuple_of_two_loses_a_trailing_comma_when_it_fits() {
    assert_formats(
        "fn f() -> (Int, Int,) { (1, 2,) }\n",
        "fn f() -> (Int, Int) { (1, 2) }\n",
    );
}

#[test]
fn a_tuple_field_write_and_compound_assignment_print_as_written() {
    assert_stable("fn main() {\n    let mut t = (1, 2)\n    t.0 = 3\n    t.1 += t.0\n}\n");
}

#[test]
fn a_long_tuple_breaks_one_element_a_line() {
    assert_formats(
        "fn main() { let t = (\"a long string here\", \"another long string\", \"and a third, much, much longer one\", 12345) }\n",
        "fn main() {\n    let t = (\n        \"a long string here\",\n        \"another long string\",\n        \"and a third, much, much longer one\",\n        12345,\n    )\n}\n",
    );
}

#[test]
fn a_tail_tuple_after_a_statement_keeps_its_semicolon() {
    // The `;` ends the statement before the `(` without E0103 (5.0 spec
    // §3.5), so fmt keeps it.
    assert_stable("fn f() -> (Int, Int) {\n    g();\n    (1, 2)\n}\n");
}
