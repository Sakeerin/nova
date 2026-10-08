//! Comments (spec
//! `docs/superpowers/specs/2026-10-07-phase-3-1-formatter-design.md` §5.4).

mod common;
use common::{assert_formats, assert_stable};

#[test]
fn file_level_comments_keep_their_places() {
    assert_formats(
        "// header\nfn a() {}\n// between\nfn b() {} // after b\n// end\n",
        "// header\nfn a() {}\n\n// between\nfn b() {} // after b\n// end\n",
    );
}

#[test]
fn a_file_holding_only_comments() {
    assert_stable("// one\n\n/* two */\n");
}

#[test]
fn a_comment_at_the_end_of_a_file_without_a_newline() {
    assert_formats("fn main() {} // done", "fn main() {} // done\n");
    assert_formats("fn main() {}\n// done", "fn main() {}\n// done\n");
}

#[test]
fn comments_in_a_block_keep_their_lines() {
    assert_stable(
        "fn main() {\n    // first\n    let a = 1 // one\n\n    /* block */ let b = 2\n    // last\n}\n",
    );
}

#[test]
fn a_comment_between_a_closing_brace_and_else_survives() {
    assert_formats(
        "fn f(x: Bool) {\n    if x {\n        a()\n    } // not y\n    else {\n        b()\n    }\n}\n",
        "fn f(x: Bool) {\n    if x {\n        a()\n    } else { // not y\n        b()\n    }\n}\n",
    );
}

#[test]
fn a_comment_marker_inside_a_string_is_not_a_comment_either() {
    assert_formats(
        "fn main() { let url = \"http://example.com/*x*/\" // real\n}\n",
        "fn main() {\n    let url = \"http://example.com/*x*/\" // real\n}\n",
    );
}

#[test]
fn a_trailing_comment_in_a_list_goes_after_its_comma() {
    assert_formats(
        "fn main() {\n    f(a // first\n    , b)\n}\n",
        "fn main() {\n    f(\n        a, // first\n        b,\n    )\n}\n",
    );
}

#[test]
fn own_line_and_dangling_comments_in_lists() {
    assert_stable(
        "fn main() {\n    f(\n        // why a\n        a,\n        b,\n        // nothing after b\n    )\n    g(\n        // no arguments yet\n    )\n}\n",
    );
}

#[test]
fn comments_survive_in_declaration_lists() {
    assert_stable(
        "record P {\n    x: Int, // the x\n    // the y\n    y: Int,\n}\n\n\
         type T =\n    | A // the a\n    // the b\n    | B(Int)\n\n\
         trait Tr {\n    // a method\n    fn m(self)\n}\n\n\
         import m::{\n    a, // the a\n    b,\n}\n\n\
         fn g<\n    T, // the t\n    U,\n>() {}\n",
    );
}

#[test]
fn comments_survive_in_expression_and_pattern_lists() {
    assert_stable(
        "fn f(\n    a: Int, // the a\n    b: Int,\n) -> [Int] {\n\
         \x20   let (\n        x, // the x\n        y,\n    ) = (a, b)\n\
         \x20   let q = Q {\n        x: 1, // one\n        ..p\n    }\n\
         \x20   match a {\n        // zero\n        0 => [b] // one\n        _ => [\n            a, // first\n            b,\n        ]\n    }\n}\n",
    );
}

#[test]
fn a_comment_in_a_where_clause_breaks_the_parameters_first() {
    // The parameters break before the `where` clause does (spec §6), so a
    // forced break in the clause breaks them too (the 3.1 plan's decision 6).
    assert_formats(
        "fn w<T>(x: T) where T: A, // the bound\n{}\n",
        "fn w<T>(\n    x: T,\n)\nwhere\n    T: A, // the bound\n{}\n",
    );
}

#[test]
fn an_end_of_line_comment_inside_an_expression_stays_where_it_was() {
    assert_stable("fn main() {\n    let x = a\n        + // why\n        b\n    f(x)\n}\n");
}

#[test]
fn comments_move_with_their_imports() {
    // The header is separated from the first import by a blank line, so it
    // stays at the top (the 3.1 plan's decision 3).
    assert_formats(
        "// header\n\nimport zeta // last\n// about alpha\nimport alpha\n\nfn main() {}\n",
        "// header\n\n// about alpha\nimport alpha\nimport zeta // last\n\nfn main() {}\n",
    );
}

#[test]
fn a_comment_after_an_opening_brace_stays_on_its_line() {
    assert_stable(
        "impl Shape for Square { // squares only\n    fn area(self) -> Float { 1.0 }\n}\n",
    );
    assert_stable("fn main() { // entry\n    run()\n}\n");
}

#[test]
fn a_comment_between_doc_lines_moves_after_them() {
    assert_formats(
        "/// One.\n// note\n/// Two.\nfn f() {}\n",
        "/// One.\n/// Two.\n// note\nfn f() {}\n",
    );
}

#[test]
fn a_comment_after_a_function_type_stays_with_it() {
    // A `fn(T)` type with no `->` is spanned over the token after it (spec
    // §2); the comment must still trail the variant it follows.
    assert_stable("type Event =\n    | Click(fn(Int)) // a handler\n    | Close(Int)\n");
    assert_stable(
        "record Button {\n    on_click: fn(Int), // the handler\n    label: String,\n}\n",
    );
}

#[test]
fn a_block_comment_in_mid_line_keeps_its_place() {
    // A comment in a block keeps the block on several lines (spec §6).
    assert_formats(
        "fn main() { f(a, /* b */ c) }\n",
        "fn main() {\n    f(a, /* b */ c)\n}\n",
    );
}

#[test]
fn a_multi_line_block_comment_keeps_its_inner_lines() {
    assert_formats(
        "fn main() {\n        /* one\n           two */\n        f()\n}\n",
        "fn main() {\n    /* one\n           two */\n    f()\n}\n",
    );
}

#[test]
fn sorting_an_import_list_keeps_each_comment_with_its_name() {
    // A name takes its comments with it when the list is sorted, as an
    // import does in a sorted run (spec §6).
    assert_formats(
        "import m::{\n    b, // the b\n    a, // the a\n}\n",
        "import m::{\n    a, // the a\n    b, // the b\n}\n",
    );
    assert_formats(
        "import m::{\n    // about b\n    b,\n    a,\n}\n",
        "import m::{\n    a,\n    // about b\n    b,\n}\n",
    );
}
