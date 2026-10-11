//! Nova code, coloured by the lexer (spec §9.4).

use nova_diagnostics::FileId;
use nova_lexer::Token;

use crate::{escape, span};

/// The class a token is coloured with: `kw` for a keyword, `lit` for a
/// literal, `cm` for a doc comment, or none.
pub(crate) fn class(token: &Token) -> Option<&'static str> {
    match token {
        Token::Let
        | Token::Mut
        | Token::Const
        | Token::Fn
        | Token::Return
        | Token::If
        | Token::Else
        | Token::While
        | Token::For
        | Token::In
        | Token::Break
        | Token::Continue
        | Token::Match
        | Token::Type
        | Token::Record
        | Token::Trait
        | Token::Impl
        | Token::Import
        | Token::Module
        | Token::Pub
        | Token::Async
        | Token::Await
        | Token::Extern
        | Token::Unsafe
        | Token::True
        | Token::False
        | Token::As
        | Token::Is
        | Token::Where
        | Token::With
        | Token::SelfLower
        | Token::SelfUpper => Some("kw"),
        Token::Int(_)
        | Token::Float(_)
        | Token::Char(_)
        | Token::StrStart
        | Token::StrPart(_)
        | Token::StrEnd
        | Token::InterpOpen
        | Token::InterpClose
        | Token::RawStr(_) => Some("lit"),
        Token::DocComment(_) => Some("cm"),
        _ => None,
    }
}

/// `code` as HTML, coloured (spec §9.4). Code the lexer rejects is shown
/// plain.
pub(crate) fn highlight(code: &str) -> String {
    let (tokens, comments, errors) = nova_lexer::lex_with_comments(code, FileId::DUMMY);
    if !errors.is_empty() {
        return escape(code);
    }
    let mut marks: Vec<(usize, usize, Option<&str>)> = tokens
        .iter()
        .filter(|t| t.value != Token::Eof)
        .map(|t| (t.span.start as usize, t.span.end as usize, class(&t.value)))
        .collect();
    marks.extend(
        comments
            .iter()
            .map(|c| (c.span.start as usize, c.span.end as usize, Some("cm"))),
    );
    marks.sort_by_key(|m| m.0);
    let mut out = String::new();
    let mut at = 0;
    for (start, end, class) in marks {
        if start < at || end > code.len() {
            continue;
        }
        out.push_str(&escape(&code[at..start]));
        match class {
            Some(c) => span(&mut out, c, &code[start..end]),
            None => out.push_str(&escape(&code[start..end])),
        }
        at = end;
    }
    out.push_str(&escape(&code[at..]));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keywords_literals_and_comments_are_coloured() {
        assert_eq!(
            highlight("fn f() -> Int { 1 } // one"),
            "<span class=\"kw\">fn</span> f() -&gt; Int { <span class=\"lit\">1</span> } \
             <span class=\"cm\">// one</span>"
        );
    }

    #[test]
    fn a_string_and_a_doc_comment() {
        assert_eq!(
            highlight("/// Doc.\nlet s = \"a <b>\""),
            "<span class=\"cm\">/// Doc.</span>\n<span class=\"kw\">let</span> s = \
             <span class=\"lit\">&quot;</span><span class=\"lit\">a &lt;b&gt;</span>\
             <span class=\"lit\">&quot;</span>"
        );
    }

    #[test]
    fn code_the_lexer_rejects_is_plain() {
        assert_eq!(highlight("let s = \"open"), "let s = &quot;open");
    }
}
