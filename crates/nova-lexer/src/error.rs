//! Lexer error types.

use nova_diagnostics::Span;
use thiserror::Error;

/// A non-fatal error produced by the lexer.
///
/// Lexing continues after any of these; all errors are collected and returned
/// alongside the token stream.
#[derive(Debug, Clone, Error)]
pub enum LexError {
    #[error("unterminated string literal")]
    UnterminatedString(Span),

    #[error("unterminated string interpolation: this `${{` has no matching `}}`")]
    UnterminatedInterpolation(Span),

    #[error("unterminated block comment")]
    UnterminatedBlockComment(Span),

    #[error("invalid escape sequence `\\{0}`")]
    InvalidEscapeSequence(char, Span),

    #[error("invalid unicode escape sequence")]
    InvalidUnicodeEscape(Span),

    #[error("invalid numeric literal")]
    InvalidNumberLiteral(Span),

    #[error("unexpected character `{0}`")]
    UnexpectedCharacter(char, Span),
}

impl LexError {
    pub fn span(&self) -> Span {
        match self {
            LexError::UnterminatedString(s) => *s,
            LexError::UnterminatedInterpolation(s) => *s,
            LexError::UnterminatedBlockComment(s) => *s,
            LexError::InvalidEscapeSequence(_, s) => *s,
            LexError::InvalidUnicodeEscape(s) => *s,
            LexError::InvalidNumberLiteral(s) => *s,
            LexError::UnexpectedCharacter(_, s) => *s,
        }
    }
}
