//! Parse error types.

use nova_diagnostics::Span;
use thiserror::Error;

/// A parse error with source location.
#[derive(Debug, Clone, Error)]
pub enum ParseError {
    #[error("expected {expected}, found {found}")]
    Expected {
        expected: String,
        found: String,
        span: Span,
    },

    #[error("unexpected end of file")]
    UnexpectedEof { span: Span },

    #[error("chained comparison operators are not allowed (use parentheses)")]
    ChainedComparison { span: Span },

    #[error("{message}")]
    Custom { message: String, span: Span },

    /// More than [`MAX_DEPTH`](crate::MAX_DEPTH) levels of nesting, at the
    /// token where the limit was crossed: P0002 (spec
    /// `docs/superpowers/specs/2026-10-30-phase-5-1-the-lexer-the-parser-and-the-ast-design.md`
    /// §4.4). The parser stops there, so it is the file's last error.
    #[error("this is nested too deeply (more than {} levels)", crate::MAX_DEPTH)]
    TooDeep { span: Span },

    /// `error`, with a note that says why it may have come about.
    #[error("{error}")]
    Noted {
        error: Box<ParseError>,
        note: String,
    },
}

impl ParseError {
    pub fn span(&self) -> Span {
        match self {
            ParseError::Expected { span, .. } => *span,
            ParseError::UnexpectedEof { span } => *span,
            ParseError::ChainedComparison { span } => *span,
            ParseError::Custom { span, .. } => *span,
            ParseError::TooDeep { span } => *span,
            ParseError::Noted { error, .. } => error.span(),
        }
    }

    /// The code a caller gives the error's diagnostic: P0002 for
    /// [`ParseError::TooDeep`], P0001 for every other.
    pub fn code(&self) -> &'static str {
        match self {
            ParseError::TooDeep { .. } => "P0002",
            ParseError::Noted { error, .. } => error.code(),
            _ => "P0001",
        }
    }

    /// The notes that go with the error's diagnostic.
    pub fn notes(&self) -> Vec<String> {
        match self {
            ParseError::Noted { error, note } => {
                let mut notes = error.notes();
                notes.push(note.clone());
                notes
            }
            _ => Vec::new(),
        }
    }
}
