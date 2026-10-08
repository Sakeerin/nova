use nova_ast::item::{ExternItem, TraitItem, TypeDef};
use nova_ast::{File, Item, Stmt, Visibility};
use nova_diagnostics::FileDb;
use nova_diagnostics::Spanned;
use nova_lexer::lex;
use nova_parser::{parse, ParseError};

/// Parse a source string and return the AST plus parse errors.
///
/// `parse_file`'s `(bool, usize)` and `parse_fixture`'s `(bool, Vec<String>)`
/// only summarize the result; neither returns the `File` itself, so neither
/// can inspect fields like `f.attrs`. This helper does, and is used by all
/// attribute tests below.
fn parse_str(source: &str) -> (File, Vec<ParseError>) {
    let mut db = FileDb::new();
    let file_id = db.add("attr_test", source);
    let (tokens, lex_errs) = lex(source, file_id);
    assert!(lex_errs.is_empty(), "lex errors: {:?}", lex_errs);
    let (ast, parse_errs) = parse(&tokens, file_id);
    (
        ast.expect("parse() should always return Some(File)"),
        parse_errs,
    )
}

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
fn a_bare_attribute_parses_onto_a_function() {
    let (file, errs) = parse_str("@test\nfn t() { }\n");
    assert!(errs.is_empty(), "{errs:?}");
    let Item::Function(f) = &file.items[0].value else {
        panic!("not a fn")
    };
    assert_eq!(f.attrs.len(), 1);
    assert_eq!(f.attrs[0].name.value, "test");
    assert!(f.attrs[0].args.is_empty());
    // `Attribute.span` is what lets Task 2 report a misplaced attribute with a
    // location; pin that it is real (non-empty) and points at the `@`, not a
    // zero-width placeholder like `Span::point(0, file)`.
    assert_eq!(f.attrs[0].span.start, 0, "span should start at the `@`");
    assert!(
        f.attrs[0].span.end > f.attrs[0].span.start,
        "span should be non-empty"
    );
}

#[test]
fn an_attribute_with_arguments_parses() {
    let (file, errs) = parse_str("@test(should_panic)\nfn t() { }\n");
    assert!(errs.is_empty(), "{errs:?}");
    let Item::Function(f) = &file.items[0].value else {
        panic!("not a fn")
    };
    assert_eq!(f.attrs[0].args.len(), 1);
    assert_eq!(f.attrs[0].args[0].value, "should_panic");
}

#[test]
fn multiple_arguments_and_multiple_attributes_parse() {
    // `@derive(Copy, Clone)` is the spec's other attribute form
    // (nova-spec/12-TYPESYSTEM.md:199). It must PARSE here even though Task 2
    // rejects it as unknown — separating syntax from the known set is what lets
    // Task 2's E0082 name the attribute instead of the parser failing first.
    let (file, errs) = parse_str("@derive(Copy, Clone)\n@test\nfn t() { }\n");
    assert!(errs.is_empty(), "{errs:?}");
    let Item::Function(f) = &file.items[0].value else {
        panic!("not a fn")
    };
    assert_eq!(f.attrs.len(), 2);
    assert_eq!(f.attrs[0].name.value, "derive");
    assert_eq!(f.attrs[0].args.len(), 2);
    // Order matters, not just count: a transposition bug (e.g. pushing an
    // argument onto the front instead of the back) would silently reverse
    // `@derive(Copy, Clone)` into `args = [Clone, Copy]` and nothing above
    // would catch it.
    assert_eq!(f.attrs[0].args[0].value, "Copy");
    assert_eq!(f.attrs[0].args[1].value, "Clone");
    assert_eq!(f.attrs[1].name.value, "test");
}

#[test]
fn an_attribute_precedes_pub() {
    let (file, errs) = parse_str("@test\npub fn t() { }\n");
    assert!(errs.is_empty(), "{errs:?}");
    let Item::Function(f) = &file.items[0].value else {
        panic!("not a fn")
    };
    assert_eq!(f.attrs.len(), 1);
    assert_eq!(f.vis, Visibility::Pub);
}

#[test]
fn an_attribute_on_a_record_parses_and_is_kept() {
    // Not an error here — Task 2 decides placement. Kept so the resolver has a
    // span to report against.
    let (file, errs) = parse_str("@test\nrecord R { n: Int }\n");
    assert!(errs.is_empty(), "{errs:?}");
    let Item::Record(r) = &file.items[0].value else {
        panic!("not a record")
    };
    assert_eq!(r.attrs.len(), 1);
}

#[test]
fn a_bare_at_sign_is_a_parse_error_and_does_not_hang() {
    // `parse_file`'s loop guarantees progress (grammar.rs:194-232) but that
    // guarantee was added *after* a two-line file hung nova check for 15 s.
    // A new token that can appear at item position is exactly the shape that
    // broke it, so assert termination, not just the error.
    let (_file, errs) = parse_str("@\nfn main() { }\n");
    assert!(!errs.is_empty(), "expected a parse error for a bare `@`");
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

#[test]
fn snapshot_impl_with_associated_type_binding() {
    // `type Item = Int` inside an impl, mixed with a method and a const, to
    // pin that the three parallel vectors on `ImplBlock` stay independent and
    // that no separator is required between impl items.
    let source = "impl It for W { type Item = Int  const N: Int = 1  fn get(self) -> Int { 1 } }";
    let mut db = FileDb::new();
    let file_id = db.add("test.nova", source);
    let (tokens, _) = lex(source, file_id);
    let (ast, errs) = parse(&tokens, file_id);
    assert!(errs.is_empty(), "parse errors: {errs:?}");
    insta::assert_debug_snapshot!(ast);
}

#[test]
fn an_impl_associated_type_binding_may_not_be_pub() {
    // `parse_visibility()` runs before the impl body's item dispatch, so a
    // `pub` here would otherwise parse and then be silently dropped: the
    // binding has no visibility of its own to honour it with. Rejected
    // loudly, and the binding is still recorded so the rest of the impl
    // checks normally.
    let source = "impl It for W { pub type Item = Int }";
    let mut db = FileDb::new();
    let file_id = db.add("test.nova", source);
    let (tokens, _) = lex(source, file_id);
    let (ast, errs) = parse(&tokens, file_id);
    assert!(ast.is_some(), "still returns an AST");
    let msgs: Vec<String> = errs.iter().map(|e| e.to_string()).collect();
    assert_eq!(msgs.len(), 1, "exactly one error: {msgs:?}");
    assert!(
        msgs[0].contains("pub"),
        "names the offending modifier: {msgs:?}"
    );
    // The private spelling of the same item must stay clean — otherwise the
    // assertion above would pass for an implementation that rejects every
    // binding.
    let ok_src = "impl It for W { type Item = Int }";
    let ok_id = db.add("ok.nova", ok_src);
    let (ok_tokens, _) = lex(ok_src, ok_id);
    let (_, ok_errs) = parse(&ok_tokens, ok_id);
    assert!(ok_errs.is_empty(), "private binding is fine: {ok_errs:?}");
}

/// Every token that `sync_to_item_boundary` treats as an item boundary, as it
/// would appear at the start of an item inside an `impl` body. Each one used to
/// make the impl-body loop spin forever: the `_` arm pushed an error and then
/// called a sync that stops *at* an item-start token without consuming it, so
/// the next iteration re-peeked the identical token. `pub` is included because
/// `parse_visibility()` consumes it and leaves the loop looking at the
/// item-start token behind it.
///
/// `type` was an eighth entry here until an impl gained associated-type
/// bindings, which made `type Item = Int` a legal impl item rather than an
/// unexpected token — its malformed spellings are covered by
/// `a_malformed_associated_type_binding_terminates` instead.
const IMPL_BODY_ITEM_STARTS: &[(&str, &str)] = &[
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
fn a_malformed_associated_type_binding_terminates() {
    // The `Token::Type` arm consumes `type` before anything can fail, so each
    // of these bails out having already made progress. Same property as the
    // test above, for the arm that replaced `type` in that list.
    for (label, item) in [
        ("no name", "type"),
        ("no eq", "type Item"),
        ("no type", "type Item ="),
        ("bad name", "type = Int"),
    ] {
        let source = format!("record W {{ v: Int }}\nimpl W {{ {item} }}\nfn main() {{ }}\n");
        let (ok, errs) = parse_file(label, &source);
        assert!(ok, "{label}: parser must still return an AST");
        assert!(errs > 0, "{label}: an incomplete binding is an error");
        assert!(errs < 10, "{label}: bounded, got {errs}");
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

/// The names of the top-level items the parser actually produced, so a test can
/// assert that recovery from a bad item did not eat the *good* ones after it.
/// The error count alone cannot say that: an item swallowed inside a preceding
/// impl block is reported as an illegal impl item, which is still just "an
/// error".
fn item_names(name: &str, source: &str) -> Vec<String> {
    use nova_ast::Item;
    let mut db = FileDb::new();
    let file_id = db.add(name, source);
    let (tokens, lex_errs) = lex(source, file_id);
    assert!(lex_errs.is_empty(), "lex errors in {name}: {lex_errs:?}");
    let (ast, _) = parse(&tokens, file_id);
    ast.map(|f| {
        f.items
            .iter()
            .map(|i| match &i.value {
                Item::Function(f) => f.name.value.clone(),
                Item::Record(r) => r.name.value.clone(),
                Item::Trait(t) => t.name.value.clone(),
                Item::Type(t) => t.name.value.clone(),
                Item::Const(c) => c.name.value.clone(),
                Item::Impl(_) => "<impl>".to_string(),
                Item::Import(_) => "<import>".to_string(),
                Item::Module(_) => "<module>".to_string(),
                Item::Extern(_) => "<extern>".to_string(),
            })
            .collect()
    })
    .unwrap_or_default()
}

/// Task 11 Step 3. Recovery from a bad item inside an `impl` body must stop at
/// the impl's own closing brace instead of walking out of it.
///
/// `sync_to_item_boundary` listed only item-start keywords, so from `impl W
/// { 42 }` it skipped past the `}` and stopped at the following `record` —
/// which the impl-body loop then reported as an illegal impl item, and `fn main`
/// was parsed *into* the impl and discarded with it. Measured on `25db453`, on
/// the four-line file below: three errors, two of them about perfectly valid
/// items. One bad token inside an impl body cost every following item in the
/// file.
///
/// The error count is not the load-bearing assertion — `item_names` is. A fix
/// that merely stopped reporting the swallowed items while still swallowing them
/// would pass on the count alone.
#[test]
fn impl_body_recovery_stops_at_the_impls_closing_brace() {
    let source = "record W { v: Int }\n\
                  impl W { 42 }\n\
                  record R { a: Int }\n\
                  fn main() { }\n";
    let (ok, errs) = parse_file("impl_recovery", source);
    assert!(ok, "parser must still return an AST");
    assert_eq!(
        errs, 1,
        "one bad token, one error — not one per following item"
    );
    assert_eq!(
        item_names("impl_recovery", source),
        ["W", "<impl>", "R", "main"],
        "the items after the malformed impl must survive it"
    );
}

/// The other half of Step 3, and the reason it is not a one-line change.
///
/// `RBrace` is the first stop token that `try_parse_item` has no arm for, and
/// `try_parse_item`'s fallthrough reports without consuming. So a stray `}` at
/// top level left `parse_file` re-peeking the same token forever: measured with
/// the stop added and the progress guard absent, `nova check` on these two lines
/// produced no output and was killed at 15 seconds. That is the same hang class
/// Task 4 fixed inside the impl body, reintroduced one caller over.
///
/// Both orders, because the guard has to hold whether or not any valid item
/// follows, and `fn main` surviving is what says recovery consumed exactly the
/// stray brace.
#[test]
fn top_level_recovery_terminates_on_a_stray_closing_brace() {
    for (label, source, want) in [
        ("brace first", "}\nfn main() { }\n", vec!["main"]),
        ("brace last", "fn main() { }\n}\n", vec!["main"]),
        ("brace alone", "}\n", vec![]),
        ("two braces", "}\n}\nfn main() { }\n", vec!["main"]),
    ] {
        let (ok, errs) = parse_file(label, source);
        assert!(ok, "{label}: parser must still return an AST");
        assert!(errs > 0, "{label}: a stray `}}` is an error, not silence");
        assert!(
            errs < 10,
            "{label}: recovery must be bounded, got {errs} errors"
        );
        assert_eq!(
            item_names(label, source),
            want,
            "{label}: the stray brace is consumed and nothing else is"
        );
    }
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

// === Doc comments (spec
// docs/superpowers/specs/2026-10-07-phase-3-1-formatter-design.md §4) ===

const MISPLACED: &str = "a doc comment must come right before an item, a field or a variant; use `//` for a plain comment";

fn docs(d: &[Spanned<String>]) -> Vec<&str> {
    d.iter().map(|s| s.value.as_str()).collect()
}

#[test]
fn docs_attach_to_each_kind_of_item() {
    let src = "/// f\nfn f() {}\n/// r\nrecord R { x: Int }\n/// t\ntype T = Int\n\
               /// tr\ntrait Tr {}\n/// i\nimpl R {}\n/// c\nconst C: Int = 1\n\
               /// im\nimport m\n/// mo\nmodule n\n/// e\nextern \"C\" {}\n";
    let (file, errors) = parse_str(src);
    assert!(errors.is_empty(), "{errors:?}");
    let got: Vec<Vec<&str>> = file
        .items
        .iter()
        .map(|item| match &item.value {
            Item::Function(x) => docs(&x.docs),
            Item::Record(x) => docs(&x.docs),
            Item::Type(x) => docs(&x.docs),
            Item::Trait(x) => docs(&x.docs),
            Item::Impl(x) => docs(&x.docs),
            Item::Const(x) => docs(&x.docs),
            Item::Import(x) => docs(&x.docs),
            Item::Module(x) => docs(&x.docs),
            Item::Extern(x) => docs(&x.docs),
        })
        .collect();
    let want: Vec<Vec<&str>> = [" f", " r", " t", " tr", " i", " c", " im", " mo", " e"]
        .iter()
        .map(|d| vec![*d])
        .collect();
    assert_eq!(got, want);
    // A documented item's span starts at its first doc line.
    assert_eq!(file.items[0].span.start, 0);
}

#[test]
fn docs_attach_to_members_fields_and_variants() {
    let src = "trait Tr {\n    /// req\n    fn a(self)\n    /// prov\n    fn b(self) {}\n    \
               /// assoc\n    type Item\n}\n\
               impl Tr for R {\n    /// m\n    fn a(self) {}\n    /// k\n    const K: Int = 1\n    \
               /// bind\n    type Item = Int\n}\n\
               record R {\n    /// x\n    x: Int,\n}\n\
               type T =\n    /// a\n    | A\n    | B\n\
               extern \"C\" {\n    /// ext\n    fn ext(x: Int) -> Int\n}\n";
    let (file, errors) = parse_str(src);
    assert!(errors.is_empty(), "{errors:?}");
    let Item::Trait(tr) = &file.items[0].value else {
        panic!("{:?}", file.items[0])
    };
    let trait_docs: Vec<Vec<&str>> = tr
        .items
        .iter()
        .map(|it| match it {
            TraitItem::Required(sig) => docs(&sig.docs),
            TraitItem::Provided(f) => docs(&f.docs),
            TraitItem::AssocType { docs: d, .. } => docs(d),
        })
        .collect();
    assert_eq!(
        trait_docs,
        vec![vec![" req"], vec![" prov"], vec![" assoc"]]
    );
    let Item::Impl(im) = &file.items[1].value else {
        panic!("{:?}", file.items[1])
    };
    assert_eq!(docs(&im.functions[0].docs), vec![" m"]);
    assert_eq!(docs(&im.consts[0].docs), vec![" k"]);
    assert_eq!(docs(&im.assoc_types[0].docs), vec![" bind"]);
    let Item::Record(r) = &file.items[2].value else {
        panic!("{:?}", file.items[2])
    };
    assert_eq!(docs(&r.fields[0].docs), vec![" x"]);
    let Item::Type(t) = &file.items[3].value else {
        panic!("{:?}", file.items[3])
    };
    let TypeDef::Sum(variants) = &t.def else {
        panic!("{:?}", t.def)
    };
    assert_eq!(docs(&variants[0].docs), vec![" a"]);
    assert!(variants[1].docs.is_empty());
    let Item::Extern(e) = &file.items[4].value else {
        panic!("{:?}", file.items[4])
    };
    let ExternItem::Fn(sig) = &e.items[0];
    assert_eq!(docs(&sig.docs), vec![" ext"]);
}

#[test]
fn docs_may_come_between_attributes_on_a_top_level_item() {
    let (file, errors) = parse_str("/// one\n@test\n/// two\nfn t() {}\n");
    assert!(errors.is_empty(), "{errors:?}");
    let Item::Function(f) = &file.items[0].value else {
        panic!("{:?}", file.items)
    };
    assert_eq!(docs(&f.docs), vec![" one", " two"]);
    assert_eq!(f.attrs.len(), 1);
}

#[test]
fn docs_attach_to_a_nested_item() {
    let (file, errors) = parse_str("fn main() {\n    /// inner\n    fn helper() {}\n}\n");
    assert!(errors.is_empty(), "{errors:?}");
    let Item::Function(main) = &file.items[0].value else {
        panic!("{:?}", file.items)
    };
    let Stmt::Item(item) = &main.body.value.stmts[0].value else {
        panic!("{:?}", main.body.value.stmts)
    };
    let Item::Function(helper) = &**item else {
        panic!("{item:?}")
    };
    assert_eq!(docs(&helper.docs), vec![" inner"]);
}

#[test]
fn a_misplaced_doc_comment_is_an_error() {
    for (place, src) in [
        (
            "before a let",
            "fn main() {\n    /// no\n    let x = 1\n}\n",
        ),
        (
            "before an expression statement",
            "fn main() {\n    /// no\n    f()\n}\n",
        ),
        (
            "before a match arm",
            "fn main() {\n    match x {\n        /// no\n        _ => 1\n    }\n}\n",
        ),
        (
            "before a parameter",
            "fn f(\n    /// no\n    x: Int,\n) {}\n",
        ),
        (
            "before a closing brace",
            "fn main() {\n    f()\n    /// no\n}\n",
        ),
        (
            "before a body's closing brace",
            "trait T {\n    /// no\n}\n",
        ),
        ("at the end of the file", "fn main() {}\n/// no\n"),
    ] {
        let (_, errors) = parse_str(src);
        let messages: Vec<String> = errors.iter().map(|e| e.to_string()).collect();
        assert_eq!(messages, vec![MISPLACED.to_owned()], "{place}");
    }
}

#[test]
fn a_variant_list_does_not_take_the_next_items_doc() {
    let (file, errors) = parse_str("type T = | A | B\n/// next\nfn f() {}\n");
    assert!(errors.is_empty(), "{errors:?}");
    let Item::Function(f) = &file.items[1].value else {
        panic!("{:?}", file.items)
    };
    assert_eq!(docs(&f.docs), vec![" next"]);
}

#[test]
fn a_trailing_comma_ends_a_where_clause() {
    let (with, errors) = parse_str("fn f<T>(x: T) -> T where T: A, { x }\n");
    assert!(errors.is_empty(), "{errors:?}");
    let (without, errors) = parse_str("fn f<T>(x: T) -> T where T: A { x }\n");
    assert!(errors.is_empty(), "{errors:?}");
    let bounds = |file: &File| match &file.items[0].value {
        Item::Function(f) => f.where_clause.len(),
        other => panic!("{other:?}"),
    };
    assert_eq!((bounds(&with), bounds(&without)), (1, 1));
}

#[test]
fn a_parenthesised_pattern_spans_its_parentheses() {
    let src = "fn main() {\n    let (x) = 1\n    let () = ()\n}\n";
    let (file, errors) = parse_str(src);
    assert!(errors.is_empty(), "{errors:?}");
    let Item::Function(f) = &file.items[0].value else {
        panic!("{:?}", file.items)
    };
    let spans: Vec<&str> = f
        .body
        .value
        .stmts
        .iter()
        .map(|s| match &s.value {
            Stmt::Let { pattern, .. } => {
                &src[pattern.span.start as usize..pattern.span.end as usize]
            }
            other => panic!("{other:?}"),
        })
        .collect();
    assert_eq!(spans, vec!["(x)", "()"]);
}

// === Phase 3.2: recovery for the language server (spec
// docs/superpowers/specs/2026-10-08-phase-3-2-lsp-core-design.md §3.2, §3.4) ===

#[test]
fn an_unfinished_member_access_keeps_its_receiver() {
    let source = "fn main() {\n    v.\n}\n";
    let (file, errors) = parse_str(source);
    assert!(
        matches!(&errors[..], [ParseError::Expected { expected, .. }]
            if expected == "identifier (in field access)"),
        "{errors:?}"
    );
    let Item::Function(f) = &file.items[0].value else {
        panic!("{:?}", file.items);
    };
    let trailing = f.body.value.trailing.as_ref().expect("the body keeps `v.`");
    let nova_ast::expr::Expr::Field { target, field } = &trailing.value else {
        panic!("{:?}", trailing.value);
    };
    assert!(
        matches!(&target.value, nova_ast::expr::Expr::Path(p) if p.segments[0].value == "v"),
        "{:?}",
        target.value
    );
    assert_eq!(field.value, "");
    // The missing name is the empty span right after the `.`.
    let dot_end = "fn main() {\n    v.".len() as u32;
    assert_eq!((field.span.start, field.span.end), (dot_end, dot_end));
}

#[test]
fn an_item_dropped_after_its_name_is_reported_by_name() {
    let source = "fn f(x: Int {\n}\nfn g() {}\n";
    let mut db = FileDb::new();
    let id = db.add("dropped", source);
    let (tokens, lex_errors) = lex(source, id);
    assert!(lex_errors.is_empty(), "{lex_errors:?}");
    let parsed = nova_parser::parse_recovering(&tokens, id);
    assert!(!parsed.errors.is_empty());
    let dropped: Vec<&str> = parsed.dropped.iter().map(|n| n.value.as_str()).collect();
    assert_eq!(dropped, ["f"]);
    // The item after it still parses.
    assert!(parsed
        .file
        .items
        .iter()
        .any(|i| matches!(&i.value, Item::Function(f) if f.name.value == "g")));
}

#[test]
fn parse_recovering_agrees_with_parse_on_a_clean_file() {
    let source = "fn main() {\n    let v = 1\n    println(\"${v}\")\n}\n";
    let mut db = FileDb::new();
    let id = db.add("clean", source);
    let (tokens, _) = lex(source, id);
    let parsed = nova_parser::parse_recovering(&tokens, id);
    let (file, errors) = parse(&tokens, id);
    assert!(parsed.errors.is_empty() && errors.is_empty());
    assert!(parsed.dropped.is_empty());
    assert_eq!(
        format!("{:?}", parsed.file),
        format!("{:?}", file.expect("Some"))
    );
}
