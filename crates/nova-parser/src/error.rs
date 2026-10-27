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
            ParseError::Noted { error, .. } => error.span(),
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
