//! The input, lexed and parsed (spec
//! `docs/superpowers/specs/2026-10-07-phase-3-1-formatter-design.md` §5.1),
//! with the source positions the printer reads what the AST forgets from
//! (§5.3).

use nova_ast::File;
use nova_diagnostics::{render, Diagnostic, FileDb, Span, Spanned};
use nova_lexer::{Comment, Token};

use crate::FormatError;

/// An input, lexed and parsed: its text, its tokens, its comments and its
/// AST.
pub(crate) struct Source<'t> {
    pub text: &'t str,
    pub file: File,
    pub tokens: Vec<Spanned<Token>>,
    pub comments: Vec<Comment>,
}

impl<'t> Source<'t> {
    /// Lex and parse `text`, naming it `name` in any diagnostics. Any lex or
    /// parse error is [`FormatError::Syntax`], holding the diagnostics
    /// `nova check` would show.
    pub fn parse(text: &'t str, name: &str) -> Result<Self, FormatError> {
        let mut db = FileDb::new();
        let id = db.add(name, text);
        let (tokens, comments, lex_errors) = nova_lexer::lex_with_comments(text, id);
        let (file, parse_errors) = nova_parser::parse(&tokens, id);
        let mut diagnostics: Vec<Diagnostic> = lex_errors
            .iter()
            .map(|e| Diagnostic::error("L0001", e.to_string()).with_primary_label(e.span(), "here"))
            .collect();
        diagnostics.extend(parse_errors.iter().map(|e| {
            Diagnostic::error("P0001", e.to_string()).with_primary_label(e.span(), "here")
        }));
        match file {
            Some(file) if diagnostics.is_empty() => Ok(Self {
                text,
                file,
                tokens,
                comments,
            }),
            _ => {
                let rendered = render::render_to_string(&db, &diagnostics);
                Err(FormatError::Syntax {
                    diagnostics,
                    rendered,
                })
            }
        }
    }

    /// The source text of `span`.
    pub fn slice(&self, span: Span) -> &'t str {
        &self.text[span.start as usize..span.end as usize]
    }

    /// The index of the first token that starts at or after `offset`.
    fn index_from(&self, offset: u32) -> usize {
        self.tokens.partition_point(|t| t.span.start < offset)
    }

    /// The first token that starts at or after `offset`: `Eof` past the end.
    pub fn token_from(&self, offset: u32) -> &Token {
        self.tokens
            .get(self.index_from(offset))
            .map_or(&Token::Eof, |t| &t.value)
    }

    /// Where a token of `kind` ends, if one is the first token at or after
    /// `offset`: the `,` after a list element, the `;` after a statement.
    pub fn token_end(&self, offset: u32, kind: &Token) -> Option<u32> {
        let t = self.tokens.get(self.index_from(offset))?;
        (t.value == *kind).then_some(t.span.end)
    }

    /// The last token ending at or before `end` that is not a `;`.
    pub fn last_code_token(&self, end: u32) -> Option<&Token> {
        let i = self.tokens.partition_point(|t| t.span.end <= end);
        self.tokens[..i]
            .iter()
            .rev()
            .map(|t| &t.value)
            .find(|t| **t != Token::Semicolon)
    }

    /// Where the token at `offset` starts after walking back over any tokens
    /// of `kinds`: a member's `pub fn`, `const` or `type`, a field's `pub`,
    /// a variant's `|`.
    pub fn start_with(&self, offset: u32, kinds: &[Token]) -> u32 {
        let mut i = self.index_from(offset);
        while i > 0 && kinds.contains(&self.tokens[i - 1].value) {
            i -= 1;
        }
        self.tokens.get(i).map_or(offset, |t| t.span.start)
    }

    /// The first `(` at or after `offset`, and its matching `)`: where each
    /// starts.
    pub fn parens_from(&self, offset: u32) -> Option<(u32, u32)> {
        let from = self.index_from(offset);
        let open = from
            + self.tokens[from..]
                .iter()
                .position(|t| t.value == Token::LParen)?;
        let mut depth = 0usize;
        for t in &self.tokens[open..] {
            if t.value == Token::LParen {
                depth += 1;
            } else if t.value == Token::RParen {
                depth -= 1;
                if depth == 0 {
                    return Some((self.tokens[open].span.start, t.span.start));
                }
            }
        }
        None
    }

    /// Where the first token of `kind`'s variant at or after `offset` starts.
    pub fn find_from(&self, offset: u32, kind: &Token) -> Option<u32> {
        self.tokens[self.index_from(offset)..]
            .iter()
            .find(|t| std::mem::discriminant(&t.value) == std::mem::discriminant(kind))
            .map(|t| t.span.start)
    }

    /// Whether `span` is wrapped in parentheses: it begins with `(`, and
    /// that `(`'s matching `)` ends it (spec §5.3).
    pub fn is_parenthesized(&self, span: Span) -> bool {
        let i = self.index_from(span.start);
        match self.tokens.get(i) {
            Some(t) if t.value == Token::LParen && t.span.start == span.start => {}
            _ => return false,
        }
        let mut depth = 0usize;
        for t in &self.tokens[i..] {
            if t.value == Token::LParen {
                depth += 1;
            } else if t.value == Token::RParen {
                depth -= 1;
                if depth == 0 {
                    return t.span.end == span.end;
                }
            }
        }
        false
    }

    /// `span` without the parentheses wrapped around it, however many pairs:
    /// `((x))` gives `x` (spec §5.3). With `own`, the innermost pair is the
    /// node's own syntax and stays: a tuple's `(a, b)`, or `()`.
    pub fn unparen(&self, span: Span, own: bool) -> Span {
        let mut outer = span;
        let mut inner = span;
        while self.is_parenthesized(inner) {
            outer = inner;
            let first = self.index_from(inner.start) + 1;
            let last = self.tokens.partition_point(|t| t.span.end <= inner.end) - 2;
            if first > last {
                break;
            }
            inner = Span {
                start: self.tokens[first].span.start,
                end: self.tokens[last].span.end,
                ..inner
            };
        }
        if own {
            outer
        } else {
            inner
        }
    }

    /// Whether the text between `a` and `b` holds a blank line: two line
    /// breaks with only spaces or tabs between them.
    pub fn blank_line_in(&self, a: u32, b: u32) -> bool {
        if a >= b {
            return false;
        }
        let mut seen = false;
        for c in self.text[a as usize..b as usize].chars() {
            match c {
                '\n' if seen => return true,
                '\n' => seen = true,
                ' ' | '\t' | '\r' => {}
                _ => seen = false,
            }
        }
        false
    }
}
