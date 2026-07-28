use nova_diagnostics::FileDb;
use nova_lexer::lex;
use nova_parser::parse;

fn parse_file(name: &str, source: &str) -> (bool, usize) {
    let mut db = FileDb::new();
    let file_id = db.add(name, source);
    let (tokens, lex_errs) = lex(source, file_id);
    let (ast, parse_errs) = parse(&tokens, file_id);
    assert!(
        lex_errs.is_empty(),
        "lex errors in {}: {:?}",
        name,
        lex_errs
    );
    (ast.is_some(), parse_errs.len())
}

fn parse_fixture(fixture: &str) -> (bool, Vec<String>) {
    let source = std::fs::read_to_string(format!(
        "{}/tests/fixtures/{}.nova",
        env!("CARGO_MANIFEST_DIR"),
        fixture
    ))
    .unwrap_or_else(|_| panic!("fixture {}.nova not found", fixture));

    let mut db = FileDb::new();
    let file_id = db.add(fixture, source.as_str());
    let (tokens, lex_errs) = lex(&source, file_id);
    let (ast, parse_errs) = parse(&tokens, file_id);

    let errors: Vec<String> = parse_errs.iter().map(|e| e.to_string()).collect();
    let lex_error_strs: Vec<String> = lex_errs.iter().map(|e| e.to_string()).collect();
    assert!(
        lex_error_strs.is_empty(),
        "lex errors in {}: {:?}",
        fixture,
        lex_error_strs
    );

    (ast.is_some(), errors)
}

#[test]
fn nested_generics_with_glued_gt_parse() {
    // Closing angle brackets that abut (`>>`) lex as one token; the parser must
    // still close nested generic argument lists (regression: Option<Option<T>>).
    let (_, errs) = parse_file("nested", "fn f() -> Option<Option<Int>> { None }\n");
    assert_eq!(errs, 0, "Option<Option<Int>> should parse");
    let (_, e2) = parse_file(
        "triple",
        "record Box<T> { value: T }\nfn f(b: Box<Box<Box<Int>>>) -> Int { 0 }\n",
    );
    assert_eq!(e2, 0, "Box<Box<Box<Int>>> should parse");
    let (_, e3) = parse_file("res", "fn f() -> Result<Int, Option<Int>> { Ok(1) }\n");
    assert_eq!(e3, 0, "Result<Int, Option<Int>> should parse");
}

#[test]
fn shift_right_operator_still_parses() {
    // The nested-generics fix must not break the `>>` right-shift operator.
    let (_, errs) = parse_file("shr", "fn main() { let x = 256 >> 2 }\n");
    assert_eq!(errs, 0, "`256 >> 2` should parse");
}

#[test]
fn fixture_hello() {
    let (ok, errs) = parse_fixture("hello");
    assert!(ok, "failed to parse hello.nova");
    assert!(errs.is_empty(), "parse errors in hello.nova: {:?}", errs);
}

#[test]
fn fixture_generics() {
    let (ok, errs) = parse_fixture("generics");
    assert!(ok, "failed to parse generics.nova");
    assert!(errs.is_empty(), "parse errors in generics.nova: {:?}", errs);
}

#[test]
fn fixture_sum_type() {
    let (ok, errs) = parse_fixture("sum_type");
    assert!(ok, "failed to parse sum_type.nova");
    assert!(errs.is_empty(), "parse errors in sum_type.nova: {:?}", errs);
}

#[test]
fn fixture_match() {
    let (ok, errs) = parse_fixture("match");
    assert!(ok, "failed to parse match.nova");
    assert!(errs.is_empty(), "parse errors in match.nova: {:?}", errs);
}

#[test]
fn fixture_trait_impl() {
    let (ok, errs) = parse_fixture("trait_impl");
    assert!(ok, "failed to parse trait_impl.nova");
    assert!(
        errs.is_empty(),
        "parse errors in trait_impl.nova: {:?}",
        errs
    );
}

#[test]
fn snapshot_hello() {
    let source = r#"fn main() { println("Hello, World!") }"#;
    let mut db = FileDb::new();
    let file_id = db.add("test.nova", source);
    let (tokens, _) = lex(source, file_id);
    let (ast, errs) = parse(&tokens, file_id);
    assert!(errs.is_empty(), "parse errors: {:?}", errs);
    insta::assert_debug_snapshot!(ast);
}

#[test]
fn snapshot_function_with_generics() {
    let source = r#"fn identity<T>(x: T) -> T { x }"#;
    let mut db = FileDb::new();
    let file_id = db.add("test.nova", source);
    let (tokens, _) = lex(source, file_id);
    let (ast, errs) = parse(&tokens, file_id);
    assert!(errs.is_empty(), "parse errors: {:?}", errs);
    insta::assert_debug_snapshot!(ast);
}

#[test]
fn snapshot_record_decl() {
    let source = r#"record Point { x: Float, y: Float, }"#;
    let mut db = FileDb::new();
    let file_id = db.add("test.nova", source);
    let (tokens, _) = lex(source, file_id);
    let (ast, errs) = parse(&tokens, file_id);
    assert!(errs.is_empty(), "parse errors: {:?}", errs);
    insta::assert_debug_snapshot!(ast);
}

#[test]
fn recovery_missing_closing_brace() {
    let source = "fn broken() { let x = 1";
    let (ok, n_errs) = parse_file("broken.nova", source);
    // Should still produce an AST (partial) and collect errors
    assert!(ok, "expected partial AST even with errors");
    assert!(n_errs > 0, "expected at least one parse error");
}

#[test]
fn if_else_expr() {
    let source = r#"fn f(x: Int) -> Int { if x > 0 { x } else { 0 } }"#;
    let (ok, errs) = parse_file("f.nova", source);
    assert!(ok);
    assert_eq!(errs, 0, "expected no parse errors, got {}", errs);
}

#[test]
fn match_with_guard() {
    let source = r#"
fn describe(n: Int) -> String {
    match n {
        0 => "zero",
        n if n < 0 => "negative",
        _ => "positive",
    }
}
"#;
    let (ok, errs) = parse_file("describe.nova", source);
    assert!(ok);
    assert_eq!(errs, 0, "expected no parse errors, got {}", errs);
}

#[test]
fn record_literal_inside_a_string_interpolation_parses() {
    // The lexer balances braces inside a `${…}` hole, so the record literal's
    // `}` no longer ends the hole. Before that, this reported two nonsense
    // errors, the first being "expected `}` (in record literal), found `}`".
    let (ok, errs) = parse_file(
        "interp_record.nova",
        "record R { v: Int }\nfn f(r: R) -> Int { r.v }\nfn main() { println(\"${f(R { v: 1 })}\") }\n",
    );
    assert!(ok);
    assert_eq!(errs, 0, "expected no parse errors, got {}", errs);

    // Nested a further level, and a block expression in a hole.
    let (ok, errs) = parse_file(
        "interp_nested.nova",
        "record R { v: G }\nrecord G { w: Int }\n\
         fn main() { println(\"${R { v: G { w: 1 } }.v.w}\") }\n",
    );
    assert!(ok);
    assert_eq!(errs, 0, "expected no parse errors, got {}", errs);

    let (ok, errs) = parse_file(
        "interp_block.nova",
        "fn main() { let a = true\n println(\"${if a { 1 } else { 2 }}\") }\n",
    );
    assert!(ok);
    assert_eq!(errs, 0, "expected no parse errors, got {}", errs);
}

#[test]
fn record_literal_in_an_interpolation_parses_in_a_no_struct_literal_position() {
    // `if`/`while`/`for`/`match` scrutinees suppress `Ident {` record literals
    // to keep the following `{ block }` unambiguous. A `${…}` hole is delimited
    // by its own `InterpClose`, so the suppression must not reach inside it.
    let source = "record R { v: Int }\n\
                  fn f(r: R) -> Int { r.v }\n\
                  fn main() {\n\
                      if \"${f(R { v: 1 })}\" == \"1\" { println(\"y\") }\n\
                      while \"${f(R { v: 2 })}\" == \"\" { println(\"n\") }\n\
                      match \"${f(R { v: 3 })}\" { _ => println(\"m\") }\n\
                  }\n";
    let (ok, errs) = parse_file("interp_no_struct.nova", source);
    assert!(ok);
    assert_eq!(errs, 0, "expected no parse errors, got {}", errs);

    // The suppression still applies to the scrutinee itself, outside any hole:
    // `if r == R { v: 1 } { }` must not swallow the block as a record literal.
    let (ok, errs) = parse_file(
        "no_struct_still_on.nova",
        "record R { v: Int }\nfn main() { let r = R { v: 1 }\n if r == r { println(\"y\") } }\n",
    );
    assert!(ok);
    assert_eq!(errs, 0, "expected no parse errors, got {}", errs);
}

#[test]
fn async_function() {
    let source = r#"async fn f() -> Int { 42 }"#;
    let (ok, errs) = parse_file("f.nova", source);
    assert!(ok);
    assert_eq!(errs, 0, "expected no parse errors, got {}", errs);
}

#[test]
fn trait_declaration() {
    let source = r#"
trait Greet {
    fn hello(self) -> String;
}
"#;
    let (ok, errs) = parse_file("greet.nova", source);
    assert!(ok);
    assert_eq!(errs, 0, "expected no parse errors, got {}", errs);
}

#[test]
fn impl_for_type() {
    let source = r#"
impl Display for Point {
    fn fmt(self) -> String {
        "point"
    }
}
"#;
    let (ok, errs) = parse_file("impl.nova", source);
    assert!(ok);
    assert_eq!(errs, 0, "expected no parse errors, got {}", errs);
}

#[test]
fn parses_repeat_array_literal() {
    // `[init; n]` — an `n`-slot array whose every slot is `init`, both evaluated
    // once at runtime.
    let (ok, errs) = parse_file("repeat", "fn main() { let n = 3\n let a = [0; n] }\n");
    assert!(ok, "`[0; n]` should parse");
    assert_eq!(errs, 0, "expected no parse errors, got {}", errs);
}

#[test]
fn plain_and_empty_array_literals_still_parse() {
    // The repeat form must not disturb the comma-separated and empty forms.
    let (ok, errs) = parse_file(
        "arrays",
        "fn main() { let a = [1, 2, 3]\n let e = []\n let t = [1,] }\n",
    );
    assert!(ok, "plain array literals should parse");
    assert_eq!(errs, 0, "expected no parse errors, got {}", errs);
}

#[test]
fn snapshot_trait_with_associated_type() {
    // No separator is required between an associated-type declaration and the
    // next trait item — same convention as a required method signature.
    let source = "trait It { type Item  fn next(self) -> Int }";
    let mut db = FileDb::new();
    let file_id = db.add("test.nova", source);
    let (tokens, _) = lex(source, file_id);
    let (ast, errs) = parse(&tokens, file_id);
    assert!(errs.is_empty(), "parse errors: {:?}", errs);
    insta::assert_debug_snapshot!(ast);
}

#[test]
fn snapshot_trait_with_bounded_associated_type() {
    // `bounds` is parsed here and rejected later (`nova-typeck` reports
    // E0900) — this snapshot only pins that parsing keeps the bound.
    let source = "trait It { type Item: Display }";
    let mut db = FileDb::new();
    let file_id = db.add("test.nova", source);
    let (tokens, _) = lex(source, file_id);
    let (ast, errs) = parse(&tokens, file_id);
    assert!(errs.is_empty(), "parse errors: {:?}", errs);
    insta::assert_debug_snapshot!(ast);
}

/// Every token that `sync_to_item_boundary` treats as an item boundary, as it
/// would appear at the start of an item inside an `impl` body. Each one used to
/// make the impl-body loop spin forever: the `_` arm pushed an error and then
/// called a sync that stops *at* an item-start token without consuming it, so
/// the next iteration re-peeked the identical token. `pub` is included because
/// `parse_visibility()` consumes it and leaves the loop looking at the
/// item-start token behind it.
const IMPL_BODY_ITEM_STARTS: &[(&str, &str)] = &[
    ("type", "type Item = Int"),
    ("record", "record R { a: Int }"),
    ("trait", "trait Q { fn q(self) -> Int }"),
    ("impl", "impl W { }"),
    ("import", "import foo"),
    ("module", "module foo"),
    ("extern", "extern { fn e() -> Int }"),
    ("pub record", "pub record R { a: Int }"),
];

#[test]
fn an_item_start_token_inside_an_impl_body_terminates() {
    // The load-bearing property is that this test *finishes*: a
    // zero-progress recovery loop cannot be caught by an assertion, only by
    // the run never returning (or, in a test binary, by the allocator giving
    // up on the error vector — measured at 8 GiB before this was fixed).
    //
    // The bound on the error count is the assertable half of the same
    // property. The exact number is not the contract — being O(1) rather than
    // O(remaining tokens) is, so a regression that merely made recovery
    // quadratic instead of infinite would still be caught here.
    for (label, item) in IMPL_BODY_ITEM_STARTS {
        let source = format!("record W {{ v: Int }}\nimpl W {{ {item} }}\nfn main() {{ }}\n");
        let (ok, errs) = parse_file(label, &source);
        assert!(ok, "{label}: parser must still return an AST");
        assert!(
            errs > 0,
            "{label}: an item inside an impl body is an error, not silence"
        );
        assert!(
            errs < 10,
            "{label}: recovery must be bounded, got {errs} errors"
        );
    }
}

#[test]
fn a_non_item_token_inside_an_impl_body_still_recovers() {
    // The control case: `42` and `let` were never item-start tokens, so
    // `sync_to_item_boundary` always consumed them and these two always
    // terminated. They must keep doing so — the fix must not turn a working
    // recovery path into a worse one.
    for (label, item) in [("int", "42"), ("let", "let x = 1")] {
        let source = format!("record W {{ v: Int }}\nimpl W {{ {item} }}\nfn main() {{ }}\n");
        let (ok, errs) = parse_file(label, &source);
        assert!(ok, "{label}: parser must still return an AST");
        assert!(errs > 0, "{label}: still an error");
        assert!(errs < 10, "{label}: bounded, got {errs}");
    }
}

#[test]
fn a_keyword_method_name_followed_by_a_generic_trait_method_terminates() {
    // The separately queued parser hang from the `std/strings` phase
    // (`.superpowers/sdd/phase-2.2a-debt/parser-hang-repro.nova`), which needs
    // BOTH halves to reproduce: `with` is a keyword, so it fails in the
    // method-name position and `parse_function` returns `None`; the impl-body
    // loop then syncs forward, and `sync_to_item_boundary` walks straight past
    // the impl's own closing brace to stop at the following `trait` — which
    // the loop re-peeked forever. Same root cause, same fix.
    let source = "record B<T> { v: T }\n\
                  impl<T> B<T> { fn with(self) -> Int { 1 } }\n\
                  trait M { fn remap<U>(self, u: U) -> Int }\n\
                  fn main() { }\n";
    let (ok, errs) = parse_file("keyword_method_name", source);
    assert!(ok, "parser must still return an AST");
    assert!(errs > 0, "`fn with` is a parse error");
    assert!(errs < 10, "recovery must be bounded, got {errs} errors");
}

// Property test: parse never panics
#[cfg(test)]
mod prop {
    use nova_diagnostics::FileDb;
    use nova_lexer::lex;
    use nova_parser::parse;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn parse_never_panics(s in ".*") {
            let mut db = FileDb::new();
            let file_id = db.add("<prop>", s.as_str());
            let (tokens, _) = lex(&s, file_id);
            let _ = parse(&tokens, file_id);
        }
    }
}
