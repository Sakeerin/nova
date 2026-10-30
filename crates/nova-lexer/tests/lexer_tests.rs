use nova_diagnostics::FileDb;
use nova_lexer::{dump_tokens, lex, lex_with_comments, CommentKind, Token};
use std::path::{Path, PathBuf};

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

// === Comments, for the formatter (spec
// docs/superpowers/specs/2026-10-07-phase-3-1-formatter-design.md §3) ===

/// Each comment `lex_with_comments` reports, as its kind and source text.
fn comments(source: &str) -> Vec<(CommentKind, String)> {
    let mut db = FileDb::new();
    let file = db.add("<test>", source);
    let (_, comments, errors) = lex_with_comments(source, file);
    assert!(errors.is_empty(), "unexpected lex errors: {errors:?}");
    comments
        .iter()
        .map(|c| {
            (
                c.kind,
                source[c.span.start as usize..c.span.end as usize].to_owned(),
            )
        })
        .collect()
}

#[test]
fn lex_with_comments_returns_each_comment_in_order() {
    // The third sits just before a string, and the last ends the file with
    // no newline after it.
    assert_eq!(
        comments("// one\nfn /* two */ main() { f(/* three */\"s\") } // four"),
        vec![
            (CommentKind::Line, "// one".to_owned()),
            (CommentKind::Block, "/* two */".to_owned()),
            (CommentKind::Block, "/* three */".to_owned()),
            (CommentKind::Line, "// four".to_owned()),
        ]
    );
}

#[test]
fn a_line_comments_span_stops_before_a_crlf() {
    assert_eq!(
        comments("// a\r\nfn main() {}\r\n"),
        vec![(CommentKind::Line, "// a".to_owned())]
    );
}

#[test]
fn a_comment_inside_an_interpolation_hole_is_captured() {
    assert_eq!(
        comments("let s = \"${x /* c */}\""),
        vec![(CommentKind::Block, "/* c */".to_owned())]
    );
}

#[test]
fn a_comment_marker_inside_a_string_is_not_a_comment() {
    assert_eq!(
        comments("let u = \"http://example.com/*x*/\" // real"),
        vec![(CommentKind::Line, "// real".to_owned())]
    );
}

#[test]
fn four_slashes_make_a_plain_comment() {
    let toks = tokens("//// ----\nfn f() {}");
    assert!(matches!(toks[0], Token::Fn), "toks: {toks:?}");
    assert_eq!(
        comments("//// ----\nfn f() {}"),
        vec![(CommentKind::Line, "//// ----".to_owned())]
    );
}

#[test]
fn doc_text_keeps_its_indentation_and_drops_trailing_whitespace() {
    let toks = tokens("///   indented  \r\nfn f() {}");
    assert_eq!(toks[0], Token::DocComment("   indented".to_owned()));
}

#[test]
fn an_unterminated_block_comment_is_an_error() {
    let (_, errors) = lex_all("fn main() {} /* never closed");
    assert_eq!(errors, vec!["unterminated block comment".to_owned()]);
}

/// Every `.nova` file under `dir`, skipping `target/` and dot-directories.
fn nova_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let mut entries: Vec<_> = std::fs::read_dir(dir)
        .expect("read a directory")
        .map(|e| e.expect("read an entry"))
        .collect();
    entries.sort_by_key(|e| e.file_name());
    for entry in entries {
        let name = entry.file_name().to_string_lossy().into_owned();
        let kind = entry.file_type().expect("read an entry's type");
        if kind.is_dir() {
            if name != "target" && !name.starts_with('.') {
                nova_files(&entry.path(), out);
            }
        } else if kind.is_file() && name.ends_with(".nova") {
            out.push(entry.path());
        }
    }
}

#[test]
fn lex_and_lex_with_comments_agree_on_every_nova_file() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut files = Vec::new();
    nova_files(&root, &mut files);
    assert!(files.len() >= 168, "found only {} .nova files", files.len());
    for path in files {
        let text = std::fs::read_to_string(&path).expect("read a .nova file");
        let mut db = FileDb::new();
        let file = db.add("<corpus>", text.as_str());
        let (plain, _) = lex(&text, file);
        let (with, _, _) = lex_with_comments(&text, file);
        let pairs = |toks: nova_lexer::Tokens| {
            toks.into_iter()
                .map(|t| (t.value, t.span))
                .collect::<Vec<_>>()
        };
        assert_eq!(pairs(plain), pairs(with), "{}", path.display());
    }
}

// --- The line-break mark (5.0 spec §3.5) ---

/// Each token's mark, by its source text, for `src`, `Eof` left out.
fn marks(src: &str) -> Vec<(String, bool)> {
    let mut db = FileDb::new();
    let file = db.add("<marks>", src);
    let (tokens, errors) = lex(src, file);
    assert!(errors.is_empty(), "{errors:?}");
    tokens
        .iter()
        .enumerate()
        .filter(|(_, t)| t.value != Token::Eof)
        .map(|(i, t)| {
            let text = src[t.span.start as usize..t.span.end as usize].to_string();
            (text, tokens.line_break_before(i))
        })
        .collect()
}

#[test]
fn a_token_after_a_line_break_is_marked() {
    let m = marks("a\n(b)\r\n  - c");
    let want: Vec<(String, bool)> = [
        ("a", false),
        ("(", true),
        ("b", false),
        (")", false),
        ("-", true),
        ("c", false),
    ]
    .iter()
    .map(|(t, b)| (t.to_string(), *b))
    .collect();
    assert_eq!(m, want);
}

#[test]
fn the_first_token_is_marked_only_after_a_line_break() {
    assert!(!marks("x")[0].1);
    assert!(marks("\n\nx")[0].1);
}

#[test]
fn a_line_comment_marks_the_token_after_it() {
    let m = marks("a // note\nb");
    assert_eq!(m[1], ("b".to_string(), true));
}

#[test]
fn a_block_comment_marks_the_token_after_it_only_when_it_holds_a_line_break() {
    assert_eq!(marks("a /* x\n y */ b")[1], ("b".to_string(), true));
    assert_eq!(marks("a /* x */ b")[1], ("b".to_string(), false));
}

#[test]
fn a_line_break_inside_a_string_marks_nothing() {
    let m = marks("f(\"a\nb\") (c)");
    assert!(m.iter().all(|(_, b)| !b), "{m:?}");
}

#[test]
fn the_marks_leave_the_tokens_as_they_were() {
    let src = "fn main() {\n    let x = 1\n    -x\n}\n";
    let mut db = FileDb::new();
    let file = db.add("<marks>", src);
    let (tokens, _) = lex(src, file);
    let (with, _, _) = lex_with_comments(src, file);
    assert_eq!(tokens.len(), with.len());
    let plain: Vec<_> = tokens.clone().into_vec();
    assert_eq!(plain.len(), tokens.len());
    assert!((0..tokens.len()).all(|i| tokens.line_break_before(i) == with.line_break_before(i)));
    assert!(!tokens.line_break_before(tokens.len() + 5));
}

#[test]
fn a_lone_carriage_return_is_a_line_break() {
    assert_eq!(marks("b\r(d)")[1], ("(".to_string(), true));
}

// ── Tuple indexes: the split after a `.` (5.0 spec §4.2) ───────────────────

/// Each token with its source text, for the split's spans.
fn texts(src: &str) -> Vec<(Token, String)> {
    let mut db = FileDb::new();
    let file = db.add("<split>", src);
    let (tokens, errors) = lex(src, file);
    assert!(errors.is_empty(), "{errors:?}");
    tokens
        .iter()
        .filter(|t| t.value != Token::Eof)
        .map(|t| {
            let text = src[t.span.start as usize..t.span.end as usize].to_string();
            (t.value.clone(), text)
        })
        .collect()
}

#[test]
fn a_tuple_index_is_an_integer_after_a_dot() {
    assert_eq!(
        tokens("t.0"),
        vec![ident("t"), Token::Dot, Token::Int(0), Token::Eof]
    );
}

#[test]
fn a_float_after_a_dot_splits_into_two_indexes() {
    assert_eq!(
        texts("t.1.0"),
        vec![
            (ident("t"), "t".to_string()),
            (Token::Dot, ".".to_string()),
            (Token::Int(1), "1".to_string()),
            (Token::Dot, ".".to_string()),
            (Token::Int(0), "0".to_string()),
        ]
    );
}

#[test]
fn the_split_chains() {
    assert_eq!(
        tokens("t.0.1.2"),
        vec![
            ident("t"),
            Token::Dot,
            Token::Int(0),
            Token::Dot,
            Token::Int(1),
            Token::Dot,
            Token::Int(2),
            Token::Eof
        ]
    );
}

#[test]
fn the_split_follows_the_source_text_not_the_value() {
    // `1.10` is the float 1.1, but the index after the second `.` is 10.
    let t = texts("t.1.10");
    assert_eq!(t[4], (Token::Int(10), "10".to_string()));
    assert_eq!(t.len(), 5);
}

#[test]
fn a_float_elsewhere_is_not_split() {
    assert_eq!(tokens("1.0"), vec![Token::Float(1.0), Token::Eof]);
    assert_eq!(
        tokens("x = 1.5"),
        vec![ident("x"), Token::Eq, Token::Float(1.5), Token::Eof]
    );
    // A `..` before it is a range, not a field access.
    assert_eq!(
        tokens("0..1.5"),
        vec![Token::Int(0), Token::DotDot, Token::Float(1.5), Token::Eof]
    );
    assert_eq!(
        tokens("0..1"),
        vec![Token::Int(0), Token::DotDot, Token::Int(1), Token::Eof]
    );
    // A float followed by a `.` stays a float: `1.0.0` is not after one.
    assert_eq!(
        tokens("1.0.0"),
        vec![Token::Float(1.0), Token::Dot, Token::Int(0), Token::Eof]
    );
}

#[test]
fn a_float_with_an_underscore_or_an_exponent_after_a_dot_is_left_whole() {
    assert_eq!(
        tokens("t.1_0.2"),
        vec![ident("t"), Token::Dot, Token::Float(10.2), Token::Eof]
    );
    assert_eq!(
        tokens("t.1.0e3"),
        vec![ident("t"), Token::Dot, Token::Float(1000.0), Token::Eof]
    );
    assert_eq!(
        tokens("t.1e3"),
        vec![ident("t"), Token::Dot, Token::Float(1000.0), Token::Eof]
    );
}

#[test]
fn only_plain_decimal_integers_are_marked_as_tuple_indexes() {
    let src = "t.0 t.7 t.12 t.01 t.1_0 t.0x1 t.00 t.0b1 t.0o7 x";
    let mut db = FileDb::new();
    let file = db.add("<plain>", src);
    let (tokens, errors) = lex(src, file);
    assert!(errors.is_empty(), "{errors:?}");
    let plain: Vec<(String, bool)> = tokens
        .iter()
        .enumerate()
        .filter(|(_, t)| matches!(t.value, Token::Int(_)))
        .map(|(i, t)| {
            let text = src[t.span.start as usize..t.span.end as usize].to_string();
            (text, tokens.plain_decimal(i))
        })
        .collect();
    let want: Vec<(String, bool)> = [
        ("0", true),
        ("7", true),
        ("12", true),
        ("01", false),
        ("1_0", false),
        ("0x1", false),
        ("00", false),
        ("0b1", false),
        ("0o7", false),
    ]
    .iter()
    .map(|(t, b)| (t.to_string(), *b))
    .collect();
    assert_eq!(plain, want);
    // An identifier is no integer, and past the end is `false`.
    assert!(!tokens.plain_decimal(tokens.len() - 2));
    assert!(!tokens.plain_decimal(tokens.len() + 3));
}

// --- The token dump (5.1 spec §3.3, ADR 0051) ---

/// Every field of the dump: the spans; the kinds' names; `Int` in decimal,
/// `Float` as its bits in lowercase hex, `Char` as its code point, text in
/// quotes with its control bytes, DEL, `"` and `\` escaped and non-ASCII
/// kept; the `NL` and `PD` marks; `Eof` last; then the errors in order, a
/// control byte in a message escaped and a `\` kept; then the comments.
#[test]
fn the_token_dump_writes_every_field() {
    let source = "let s = \"a\x01\x7f\\\"\\\\é\" r#\"q\\\"# // c\n\
                  /* b */ 07 1_0 0x1 12 2.5 'é' t.1.2 \x02 \"\\q\"\n/// doc \nfn 0.1";
    let want = [
        "0 3 Let",
        r#"4 5 Ident "s""#,
        "6 7 Eq",
        "8 9 StrStart",
        r#"9 18 StrPart "a\x01\x7f\x22\x5cé""#,
        "18 19 StrEnd",
        r#"20 27 RawStr "q\x5c""#,
        "41 43 Int 7 NL",
        "44 47 Int 10",
        "48 51 Int 1",
        "52 54 Int 12 PD",
        "55 58 Float 4004000000000000",
        "59 63 Char 233",
        r#"64 65 Ident "t""#,
        "65 66 Dot",
        "66 67 Int 1 PD",
        "67 68 Dot",
        "68 69 Int 2 PD",
        "72 73 StrStart",
        r#"74 75 StrPart "q""#,
        "75 76 StrEnd",
        r#"77 85 DocComment " doc" NL"#,
        "86 88 Fn NL",
        "89 92 Float 3fb999999999999a",
        "92 92 Eof",
        r"ERR 70 71 unexpected character `\x02`",
        r"ERR 73 74 invalid escape sequence `\q`",
        "COMMENT 28 32 Line",
        "COMMENT 33 40 Block",
    ];
    assert_eq!(dump_tokens(source), want.join("\n") + "\n");
}

/// An empty source is `Eof` alone, and every line ends with `\n`.
#[test]
fn the_token_dump_of_nothing_is_eof() {
    assert_eq!(dump_tokens(""), "0 0 Eof\n");
    assert_eq!(dump_tokens("\r\n1"), "2 3 Int 1 NL PD\n3 3 Eof\n");
}
