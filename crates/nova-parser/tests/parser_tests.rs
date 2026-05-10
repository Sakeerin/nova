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
