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
}
