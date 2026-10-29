//! 5.0b's patterns in fmt (5.0 spec
//! `docs/superpowers/specs/2026-10-26-phase-5-0-the-language-and-std-a-compiler-needs-design.md`
//! §4.3 to §4.5): each form the checker now takes prints as it reads, and
//! a messy spelling of each reformats to it.

mod common;
use common::{assert_formats, assert_stable};

#[test]
fn tuple_patterns_print_as_written() {
    assert_stable(
        "fn f(p: (Int, (Bool, Char))) -> Int {\n    match p {\n        (0, (true, _)) => 1\n        (n, _) => n\n    }\n}\n",
    );
    assert_formats(
        "fn f(p: (Int, Int)) -> Int { match p { ( a , b , ) => a } }\n",
        "fn f(p: (Int, Int)) -> Int {\n    match p {\n        (a, b) => a\n    }\n}\n",
    );
}

#[test]
fn a_tuple_pattern_of_one_keeps_its_comma() {
    assert_formats(
        "fn main() {\n    let (a ,) = (1,)\n}\n",
        "fn main() { let (a,) = (1,) }\n",
    );
}

#[test]
fn nested_patterns_print_as_written() {
    assert_stable(
        "fn f(o: Option<Result<Int, String>>) -> Int {\n    match o {\n        Some(Ok(n)) => n\n        Some(Err(_)) | None => 0\n    }\n}\n",
    );
}

#[test]
fn a_guard_prints_after_its_pattern() {
    assert_stable(
        "fn f(o: Option<Int>) -> Int {\n    match o {\n        Some(n) if n > 0 => n\n        _ => 0\n    }\n}\n",
    );
    assert_formats(
        "fn f(n: Int) -> Int { match n { x   if   x > 0   => x, _ => 0 } }\n",
        "fn f(n: Int) -> Int {\n    match n {\n        x if x > 0 => x\n        _ => 0\n    }\n}\n",
    );
}

#[test]
fn or_patterns_print_with_spaces_around_each_bar() {
    assert_formats(
        "fn f(n: Int) -> Int { match n { 0|1  |2 => 1, _ => 0 } }\n",
        "fn f(n: Int) -> Int {\n    match n {\n        0 | 1 | 2 => 1\n        _ => 0\n    }\n}\n",
    );
    assert_stable(
        "fn f(o: Option<Int>) -> Int {\n    match o {\n        Some(1 | 2) => 1\n        _ => 0\n    }\n}\n",
    );
}

#[test]
fn a_long_or_pattern_breaks_before_each_bar() {
    assert_formats(
        "fn f(t: Token) -> Int { match t { Token::Identifier(name) | Token::Keyword(name) | Token::Operator(name) | Token::Punctuation(name) => 1, _ => 0 } }\n",
        "fn f(t: Token) -> Int {\n    match t {\n        Token::Identifier(name)\n            | Token::Keyword(name)\n            | Token::Operator(name)\n            | Token::Punctuation(name) => 1\n        _ => 0\n    }\n}\n",
    );
}

#[test]
fn char_literals_and_ranges_print_as_written() {
    assert_stable(
        "fn f(c: Char) -> Int {\n    match c {\n        'a'..='z' | 'A'..='Z' => 1\n        '\\n' => 2\n        '0' => 3\n        _ => 0\n    }\n}\n",
    );
    assert_formats(
        "fn f(c: Char) -> Int { match c { 'a' ..= 'z' => 1, _ => 0 } }\n",
        "fn f(c: Char) -> Int {\n    match c {\n        'a'..='z' => 1\n        _ => 0\n    }\n}\n",
    );
}

#[test]
fn int_ranges_print_as_written() {
    assert_stable(
        "fn f(n: Int) -> Int {\n    match n {\n        0..10 => 1\n        10..=99 => 2\n        _ => 0\n    }\n}\n",
    );
    assert_formats(
        "fn f(n: Int) -> Int { match n { 0 .. 10 => 1, _ => 0 } }\n",
        "fn f(n: Int) -> Int {\n    match n {\n        0..10 => 1\n        _ => 0\n    }\n}\n",
    );
}

#[test]
fn let_and_for_destructuring_print_as_written() {
    assert_stable(
        "fn main() {\n    let (q, r) = div_mod(7, 2)\n    let mut (a, b) = (1, 2)\n    for (name, n) in pairs { println(name) }\n}\n",
    );
    assert_formats(
        "fn main() { let (q,r)=div_mod(7, 2)\n for (name,n) in pairs { println(name) } }\n",
        "fn main() {\n    let (q, r) = div_mod(7, 2)\n    for (name, n) in pairs { println(name) }\n}\n",
    );
}

#[test]
fn a_tuple_pattern_arm_after_a_bare_return_arm_prints_without_a_semicolon() {
    // The newline rule ends the bare `return` at its line break, so the
    // next arm's `(` is not read as a call (5.0 spec §3.5).
    assert_stable(
        "fn f(p: (Int, Int)) -> Int {\n    match p {\n        (0, 0) => return 0\n        (a, b) => a + b\n    }\n}\n\nfn g(p: (Int, Int)) {\n    match p {\n        (0, _) => return\n        (a, _) => println(a)\n    }\n}\n",
    );
}
