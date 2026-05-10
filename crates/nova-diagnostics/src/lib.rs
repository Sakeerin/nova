//! Error reporting infrastructure for the Nova compiler.
//!
//! Provides `Span`, `FileId`, `Spanned<T>`, and diagnostic rendering via
//! `codespan-reporting`. Every compiler crate that produces user-facing errors
//! depends on this crate.

pub mod files;
pub mod render;

pub use files::{FileDb, FileId};

/// A byte-range span inside a single source file.
///
/// Uses `u32` offsets to keep `Spanned<T>` structs compact.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Span {
    pub start: u32,
    pub end: u32,
    pub file: FileId,
}

impl Span {
    pub fn new(start: u32, end: u32, file: FileId) -> Self {
        Self { start, end, file }
    }

    /// A zero-width span at a single byte position.
    pub fn point(offset: u32, file: FileId) -> Self {
        Self {
            start: offset,
            end: offset,
            file,
        }
    }

    /// Merge two spans into one that covers both.
    pub fn merge(self, other: Span) -> Self {
        debug_assert_eq!(
            self.file, other.file,
            "cannot merge spans from different files"
        );
        Self {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
            file: self.file,
        }
    }

    pub fn as_range(self) -> std::ops::Range<usize> {
        self.start as usize..self.end as usize
    }
}

/// A value paired with the source span it came from.
#[derive(Debug, Clone)]
pub struct Spanned<T> {
    pub value: T,
    pub span: Span,
}

impl<T> Spanned<T> {
    pub fn new(value: T, span: Span) -> Self {
        Self { value, span }
    }

    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> Spanned<U> {
        Spanned {
            value: f(self.value),
            span: self.span,
        }
    }
}

/// Severity level of a diagnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
    Note,
    Help,
}

/// A single compiler diagnostic shown to the user.
///
/// Every diagnostic has an error code (e.g. `E0042`), a title, at least one
/// source span, and an optional suggestion.
#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub severity: Severity,
    pub code: String,
    pub message: String,
    pub labels: Vec<Label>,
    pub notes: Vec<String>,
}

/// A labelled source location within a diagnostic.
#[derive(Debug, Clone)]
pub struct Label {
    pub span: Span,
    pub message: String,
    pub primary: bool,
}

impl Diagnostic {
    pub fn error(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Error,
            code: code.into(),
            message: message.into(),
            labels: Vec::new(),
            notes: Vec::new(),
        }
    }

    pub fn warning(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Warning,
            code: code.into(),
            message: message.into(),
            labels: Vec::new(),
            notes: Vec::new(),
        }
    }

    pub fn with_label(mut self, span: Span, message: impl Into<String>, primary: bool) -> Self {
        self.labels.push(Label {
            span,
            message: message.into(),
            primary,
        });
        self
    }

    pub fn with_primary_label(self, span: Span, message: impl Into<String>) -> Self {
        self.with_label(span, message, true)
    }

    pub fn with_secondary_label(self, span: Span, message: impl Into<String>) -> Self {
        self.with_label(span, message, false)
    }

    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }
}
