//! `KEYWORDS` (spec
//! `docs/superpowers/specs/2026-10-08-phase-3-2-lsp-core-design.md` §4.3).

use nova_diagnostics::FileId;
use nova_lexer::{lex, Token, KEYWORDS};

#[test]
fn every_keyword_lexes_as_itself_not_as_an_identifier() {
    for word in KEYWORDS {
        let (tokens, errors) = lex(word, FileId::DUMMY);
        assert!(errors.is_empty(), "{word}: {errors:?}");
        // The keyword, then `Eof`.
        assert_eq!(tokens.len(), 2, "{word}: {tokens:?}");
        assert!(
            !matches!(tokens[0].value, Token::Ident(_)),
            "`{word}` lexed as an identifier"
        );
    }
}

#[test]
fn keywords_lists_every_alphabetic_token_the_lexer_declares() {
    let source = include_str!("../src/lib.rs");
    let mut declared: Vec<String> = source
        .split("#[token(\"")
        .skip(1)
        .map(|rest| rest.chars().take_while(|c| *c != '"').collect::<String>())
        .filter(|w| !w.is_empty() && w.chars().all(|c| c.is_ascii_alphabetic() || c == '_'))
        .collect();
    let mut listed: Vec<String> = KEYWORDS.iter().map(|w| w.to_string()).collect();
    declared.sort();
    listed.sort();
    assert_eq!(declared, listed);
}
