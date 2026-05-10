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
}

impl ParseError {
    pub fn span(&self) -> Span {
        match self {
            ParseError::Expected { span, .. } => *span,
            ParseError::UnexpectedEof { span } => *span,
            ParseError::ChainedComparison { span } => *span,
            ParseError::Custom { span, .. } => *span,
        }
    }
}
