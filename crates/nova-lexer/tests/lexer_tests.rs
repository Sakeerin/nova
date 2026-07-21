use nova_diagnostics::FileDb;
use nova_lexer::{lex, Token};

fn tokens(source: &str) -> Vec<Token> {
    let mut db = FileDb::new();
    let file = db.add("<test>", source);
    let (toks, errs) = lex(source, file);
    assert!(errs.is_empty(), "unexpected lex errors: {:?}", errs);
    toks.into_iter().map(|s| s.value).collect()
}

fn token_kinds(source: &str) -> Vec<String> {
    tokens(source)
        .into_iter()
        .map(|t| format!("{:?}", t))
        .collect()
}

#[test]
fn keywords() {
    let src = "let mut const fn return if else while for in break continue \
               match type record trait impl import module pub async await \
               extern unsafe true false as is where with self Self";
    insta::assert_debug_snapshot!(token_kinds(src));
}

#[test]
fn integer_literals() {
    insta::assert_debug_snapshot!(token_kinds("0 42 1_000_000 0xff 0b1010 0o755"));
}

#[test]
fn float_literals() {
    insta::assert_debug_snapshot!(token_kinds("1.5 1.5e3 1.5e-3 1_000.5"));
}

#[test]
fn operators() {
    insta::assert_debug_snapshot!(token_kinds(
        "( ) [ ] { } , ; : :: -> => . .. ..= @ ? = == != < <= > >= && || ! \
         + - * / % += -= *= /= %= & | ^ ~ << >> &= |= ^= <<= >>="
    ));
}

#[test]
fn string_simple() {
    let src = r#""hello world""#;
    insta::assert_debug_snapshot!(token_kinds(src));
}

#[test]
fn string_escapes() {
    let src = r#""tab:\t newline:\n quote:\" backslash:\\""#;
    let toks = tokens(src);
    insta::assert_debug_snapshot!(toks);
}

#[test]
fn string_interpolation_simple() {
    let src = r#""Hello, ${name}!""#;
    insta::assert_debug_snapshot!(token_kinds(src));
}

#[test]
fn raw_string() {
    let src = r###"r#"raw "quoted" string"#"###;
    insta::assert_debug_snapshot!(token_kinds(src));
}

#[test]
fn char_literal() {
    insta::assert_debug_snapshot!(token_kinds(r"'a' '\n' '\t'"));
}

#[test]
fn doc_comment() {
    let src = "/// This is a doc comment\nfn foo() {}";
    insta::assert_debug_snapshot!(token_kinds(src));
}

#[test]
fn line_comment_skipped() {
    let src = "let x // this is a comment\n= 1";
    let toks = tokens(src);
    // Comments should not appear in token stream
    assert!(!toks.iter().any(|t| matches!(t, Token::DocComment(_))));
}

#[test]
fn identifier() {
    let toks = tokens("hello_world _private __init camelCase PascalCase");
    for tok in &toks[..toks.len() - 1] {
        assert!(
            matches!(tok, Token::Ident(_)),
            "expected Ident, got {:?}",
            tok
        );
    }
}

#[test]
fn eof_always_last() {
    let toks = tokens("let x = 1");
    assert!(matches!(toks.last(), Some(Token::Eof)));
}

// Property test: lex never panics on arbitrary input
#[cfg(test)]
mod prop {
    use nova_diagnostics::FileDb;
    use nova_lexer::lex;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn lex_never_panics(s in ".*") {
            let mut db = FileDb::new();
            let file = db.add("<prop>", s.as_str());
            let _ = lex(&s, file);
        }
    }
}

#[test]
fn interpolation_preserves_leading_whitespace_after_close() {
    // Regression: whitespace right after `${expr}` is part of the string
    // and must not be skipped when the lexer resumes string mode.
    let toks = tokens(r#""${a} b ${c}""#);
    let parts: Vec<&str> = toks
        .iter()
        .filter_map(|t| match t {
            Token::StrPart(s) => Some(s.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(parts, vec![" b "]);
}

#[test]
fn line_and_block_comments_are_skipped() {
    // Regression: logos skips comments but the wrapper must not drift into
    // the comment text when advancing.
    let toks = tokens("// leading\nfn /* mid */ main() {}\n// trailing");
    // Expect: Fn Ident("main") LParen RParen LBrace RBrace Eof
    assert!(matches!(toks[0], Token::Fn), "toks: {toks:?}");
    assert!(
        matches!(&toks[1], Token::Ident(n) if n == "main"),
        "toks: {toks:?}"
    );
}

#[test]
fn comment_before_string_literal() {
    // Regression: a comment right before a string must not misdirect the
    // string/raw-string dispatch.
    let toks = tokens("let m = /* c */ \"hi\"");
    assert!(
        toks.iter()
            .any(|t| matches!(t, Token::StrPart(s) if s == "hi")),
        "toks: {toks:?}"
    );
    // Raw string after a comment stays a raw string.
    let raw = tokens("/* c */ r\"a\n\"");
    assert!(
        matches!(&raw[0], Token::RawStr(s) if s == "a\n"),
        "raw: {raw:?}"
    );
}

#[test]
fn doc_comment_is_not_skipped() {
    let toks = tokens("/// docs\nfn f() {}");
    assert!(
        matches!(&toks[0], Token::DocComment(s) if s.contains("docs")),
        "toks: {toks:?}"
    );
}
