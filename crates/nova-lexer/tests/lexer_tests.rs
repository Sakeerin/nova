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

/// Lex without asserting success — for the error-path tests.
fn lex_all(source: &str) -> (Vec<Token>, Vec<String>) {
    let mut db = FileDb::new();
    let file = db.add("<test>", source);
    let (toks, errs) = lex(source, file);
    (
        toks.into_iter().map(|s| s.value).collect(),
        errs.iter().map(|e| e.to_string()).collect(),
    )
}

fn ident(name: &str) -> Token {
    Token::Ident(name.to_owned())
}

fn part(s: &str) -> Token {
    Token::StrPart(s.to_owned())
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

        /// `.*` above almost never produces a `${`, so it does not exercise the
        /// interpolation state machine. Draw from an alphabet of exactly the
        /// characters that drive it instead: arbitrarily unbalanced holes,
        /// braces, quotes, and escapes must still terminate, never panic, and
        /// always yield an `Eof`-terminated stream.
        #[test]
        fn lex_never_panics_on_interpolation_soup(s in r#"["$\{\}\\a'r ]{0,48}"#) {
            let mut db = FileDb::new();
            let file = db.add("<prop>", s.as_str());
            let (toks, _) = lex(&s, file);
            prop_assert!(matches!(
                toks.last().map(|t| &t.value),
                Some(nova_lexer::Token::Eof)
            ));
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

// === `${...}` interpolation holes balance braces ===
//
// A hole ends at the `}` that matches its opening `${`, not at the first `}`
// the lexer happens to meet. Before this, `"${f(R { v: 1 })}"` closed the hole
// on the record literal's `}` and produced two nonsense parse errors
// ("expected `}` (in record literal), found `}`").

#[test]
fn interpolation_hole_balances_braces_of_a_record_literal() {
    assert_eq!(
        tokens(r#""${f(R { v: 1 })}""#),
        vec![
            Token::StrStart,
            Token::InterpOpen,
            ident("f"),
            Token::LParen,
            ident("R"),
            Token::LBrace,
            ident("v"),
            Token::Colon,
            Token::Int(1),
            Token::RBrace,
            Token::RParen,
            Token::InterpClose,
            Token::StrEnd,
            Token::Eof,
        ]
    );
}

#[test]
fn interpolation_hole_balances_nested_braces() {
    // Two levels deep: the hole must survive both inner `}`s.
    assert_eq!(
        tokens(r#""${f(R { v: G { w: 1 } })}""#),
        vec![
            Token::StrStart,
            Token::InterpOpen,
            ident("f"),
            Token::LParen,
            ident("R"),
            Token::LBrace,
            ident("v"),
            Token::Colon,
            ident("G"),
            Token::LBrace,
            ident("w"),
            Token::Colon,
            Token::Int(1),
            Token::RBrace,
            Token::RBrace,
            Token::RParen,
            Token::InterpClose,
            Token::StrEnd,
            Token::Eof,
        ]
    );
}

#[test]
fn interpolation_hole_ignores_braces_inside_a_nested_string() {
    // A `}` inside a string *inside* the hole is text, not structure. This is
    // free here: the hole's contents are lexed inline, so a nested `"` puts the
    // lexer into string mode and logos never sees the `}` at all.
    assert_eq!(
        tokens(r#""${g("}")}""#),
        vec![
            Token::StrStart,
            Token::InterpOpen,
            ident("g"),
            Token::LParen,
            Token::StrStart,
            part("}"),
            Token::StrEnd,
            Token::RParen,
            Token::InterpClose,
            Token::StrEnd,
            Token::Eof,
        ]
    );

    // Escapes inside that nested string are still escapes: `\"` does not end
    // it, and `\\` does not escape the quote that follows.
    assert_eq!(
        tokens(r#""${g("a\"}\\")}""#),
        vec![
            Token::StrStart,
            Token::InterpOpen,
            ident("g"),
            Token::LParen,
            Token::StrStart,
            part("a\"}\\"),
            Token::StrEnd,
            Token::RParen,
            Token::InterpClose,
            Token::StrEnd,
            Token::Eof,
        ]
    );
}

#[test]
fn interpolation_hole_ignores_braces_inside_char_and_raw_literals() {
    assert_eq!(
        tokens(r#""${g('}')}""#),
        vec![
            Token::StrStart,
            Token::InterpOpen,
            ident("g"),
            Token::LParen,
            Token::Char('}'),
            Token::RParen,
            Token::InterpClose,
            Token::StrEnd,
            Token::Eof,
        ]
    );

    assert_eq!(
        tokens(r##""${g(r"}")}""##),
        vec![
            Token::StrStart,
            Token::InterpOpen,
            ident("g"),
            Token::LParen,
            Token::RawStr("}".to_owned()),
            Token::RParen,
            Token::InterpClose,
            Token::StrEnd,
            Token::Eof,
        ]
    );
}

#[test]
fn interpolation_hole_inside_a_nested_string_tracks_its_own_brace_depth() {
    // Brace depth is per hole, so an inner hole's record literal must not be
    // charged to the outer hole (which would leave the outer one unclosed).
    assert_eq!(
        tokens(r#""${f("${g(R { v: 1 })}")}""#),
        vec![
            Token::StrStart,
            Token::InterpOpen,
            ident("f"),
            Token::LParen,
            Token::StrStart,
            Token::InterpOpen,
            ident("g"),
            Token::LParen,
            ident("R"),
            Token::LBrace,
            ident("v"),
            Token::Colon,
            Token::Int(1),
            Token::RBrace,
            Token::RParen,
            Token::InterpClose,
            Token::StrEnd,
            Token::RParen,
            Token::InterpClose,
            Token::StrEnd,
            Token::Eof,
        ]
    );
}

#[test]
fn interpolation_hole_balances_a_block_expression() {
    assert_eq!(
        tokens(r#""${if a { 1 } else { 2 }}""#),
        vec![
            Token::StrStart,
            Token::InterpOpen,
            Token::If,
            ident("a"),
            Token::LBrace,
            Token::Int(1),
            Token::RBrace,
            Token::Else,
            Token::LBrace,
            Token::Int(2),
            Token::RBrace,
            Token::InterpClose,
            Token::StrEnd,
            Token::Eof,
        ]
    );
}

#[test]
fn unterminated_interpolation_hole_is_an_error_not_a_hang() {
    // The record literal's `}` no longer closes the hole, so this hole never
    // closes. That has to be reported, not silently accepted (and not hang).
    let (toks, errs) = lex_all(r#""${f(R { v: 1 }""#);
    assert!(
        errs.iter()
            .any(|e| e.contains("unterminated") && e.contains("interpolation")),
        "expected an unterminated-interpolation error, got {errs:?}"
    );
    assert!(matches!(toks.last(), Some(Token::Eof)), "toks: {toks:?}");

    // Nothing after the `${` at all is the same error.
    let (_, errs) = lex_all(r#""${"#);
    assert!(
        errs.iter()
            .any(|e| e.contains("unterminated") && e.contains("interpolation")),
        "expected an unterminated-interpolation error, got {errs:?}"
    );
}

#[test]
fn interpolation_basics_still_lex() {
    // Plain hole.
    assert_eq!(
        tokens(r#""${n}""#),
        vec![
            Token::StrStart,
            Token::InterpOpen,
            ident("n"),
            Token::InterpClose,
            Token::StrEnd,
            Token::Eof,
        ]
    );

    // Adjacent holes — no empty `StrPart` between them.
    assert_eq!(
        tokens(r#""${a}${b}""#),
        vec![
            Token::StrStart,
            Token::InterpOpen,
            ident("a"),
            Token::InterpClose,
            Token::InterpOpen,
            ident("b"),
            Token::InterpClose,
            Token::StrEnd,
            Token::Eof,
        ]
    );

    // Several holes with text around them.
    assert_eq!(
        tokens(r#""a${x}b${y}c""#),
        vec![
            Token::StrStart,
            part("a"),
            Token::InterpOpen,
            ident("x"),
            Token::InterpClose,
            part("b"),
            Token::InterpOpen,
            ident("y"),
            Token::InterpClose,
            part("c"),
            Token::StrEnd,
            Token::Eof,
        ]
    );

    // Empty string.
    assert_eq!(
        tokens(r#""""#),
        vec![Token::StrStart, Token::StrEnd, Token::Eof]
    );

    // A `$` not followed by `{`, and a bare `}` in string text, are literal.
    assert_eq!(
        tokens(r#""cost: $5, $ and }""#),
        vec![
            Token::StrStart,
            part("cost: $5, $ and }"),
            Token::StrEnd,
            Token::Eof,
        ]
    );

    // After the string closes, a `}` is an ordinary `RBrace` again — the hole's
    // frame must be popped, not left on the stack.
    assert_eq!(
        tokens(r#"fn f() { "${a}" }"#),
        vec![
            Token::Fn,
            ident("f"),
            Token::LParen,
            Token::RParen,
            Token::LBrace,
            Token::StrStart,
            Token::InterpOpen,
            ident("a"),
            Token::InterpClose,
            Token::StrEnd,
            Token::RBrace,
            Token::Eof,
        ]
    );
}
