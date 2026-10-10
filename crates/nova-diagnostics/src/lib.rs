//! Error reporting infrastructure for the Nova compiler.
//!
//! Provides `Span`, `FileId`, `Spanned<T>`, and diagnostic rendering via
//! `codespan-reporting`. Every compiler crate that produces user-facing errors
//! depends on this crate.

pub mod files;
pub mod line_index;
pub mod render;

pub use files::{FileDb, FileId};
pub use line_index::LineIndex;

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
    /// Suggested changes, shown as `help:` lines and offered as quick
    /// fixes (spec 3.4b §3).
    pub fixes: Vec<Fix>,
}

/// A labelled source location within a diagnostic.
#[derive(Debug, Clone)]
pub struct Label {
    pub span: Span,
    pub message: String,
    pub primary: bool,
}
/// A suggested change: a title, and the text edits that make it (spec
/// `docs/superpowers/specs/2026-10-10-phase-3-4b-fixes-and-colour-design.md`
/// §3.1). A span carries its file, so one fix may edit several files.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fix {
    pub title: String,
    pub edits: Vec<Edit>,
}

/// Replace `span`'s text with `text`. An empty span inserts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Edit {
    pub span: Span,
    pub text: String,
}

impl Edit {
    /// Insert `text` at byte `at` of `file`.
    pub fn insert(at: u32, file: FileId, text: impl Into<String>) -> Edit {
        Edit {
            span: Span::point(at, file),
            text: text.into(),
        }
    }

    /// Replace `span` with `text`.
    pub fn replace(span: Span, text: impl Into<String>) -> Edit {
        Edit {
            span,
            text: text.into(),
        }
    }
}

impl Fix {
    pub fn new(title: impl Into<String>, edits: Vec<Edit>) -> Fix {
        Fix {
            title: title.into(),
            edits,
        }
    }

    /// Each file the fix edits, in the order its edits first name them,
    /// with the text its edits make. `None` when an edit lies outside its
    /// file, splits a character, or overlaps another edit.
    pub fn apply(&self, db: &FileDb) -> Option<Vec<(FileId, String)>> {
        let mut files: Vec<FileId> = Vec::new();
        for e in &self.edits {
            if !files.contains(&e.span.file) {
                files.push(e.span.file);
            }
        }
        let mut out = Vec::new();
        for file in files {
            let text = db.get_source(file)?;
            let mut edits: Vec<&Edit> = self.edits.iter().filter(|e| e.span.file == file).collect();
            edits.sort_by_key(|e| (e.span.start, e.span.end));
            if edits.windows(2).any(|w| w[0].span.end > w[1].span.start) {
                return None;
            }
            let mut new = text.to_string();
            for e in edits.iter().rev() {
                let (start, end) = (e.span.start as usize, e.span.end as usize);
                if start > end
                    || end > text.len()
                    || !text.is_char_boundary(start)
                    || !text.is_char_boundary(end)
                {
                    return None;
                }
                new.replace_range(start..end, &e.text);
            }
            out.push((file, new));
        }
        Some(out)
    }
}

impl Diagnostic {
    pub fn error(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Error,
            code: code.into(),
            message: message.into(),
            labels: Vec::new(),
            notes: Vec::new(),
            fixes: Vec::new(),
        }
    }

    pub fn warning(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Warning,
            code: code.into(),
            message: message.into(),
            labels: Vec::new(),
            notes: Vec::new(),
            fixes: Vec::new(),
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

    pub fn with_fix(mut self, fix: Fix) -> Self {
        self.fixes.push(fix);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn two_files() -> (FileDb, FileId, FileId) {
        let mut db = FileDb::new();
        let a = db.add("a.nova", "let x = 1\n");
        let b = db.add("b.nova", "fn f() {}\n");
        (db, a, b)
    }

    #[test]
    fn a_fix_rides_on_its_diagnostic() {
        let (_, a, _) = two_files();
        let fix = Fix::new("make `x` mutable", vec![Edit::insert(4, a, "mut ")]);
        let d = Diagnostic::error("E0060", "cannot assign").with_fix(fix.clone());
        assert_eq!(d.fixes, vec![fix]);
        assert!(Diagnostic::warning("E0021", "unreachable").fixes.is_empty());
    }

    #[test]
    fn a_fix_applies_to_each_file_it_edits() {
        let (db, a, b) = two_files();
        let fix = Fix::new(
            "two files",
            vec![
                Edit::replace(Span::new(3, 4, b), "g"),
                Edit::insert(4, a, "mut "),
                Edit::insert(0, b, "pub "),
            ],
        );
        assert_eq!(
            fix.apply(&db),
            Some(vec![
                (b, "pub fn g() {}\n".to_string()),
                (a, "let mut x = 1\n".to_string()),
            ])
        );
    }

    #[test]
    fn a_fix_outside_its_file_splitting_a_character_or_overlapping_does_not_apply() {
        let mut db = FileDb::new();
        // "ก" is three bytes.
        let t = db.add("t.nova", "ก = 1\n");
        let outside = Fix::new("outside", vec![Edit::insert(99, t, "x")]);
        let split = Fix::new("split", vec![Edit::insert(1, t, "x")]);
        let overlap = Fix::new(
            "overlap",
            vec![
                Edit::replace(Span::new(0, 3, t), "a"),
                Edit::replace(Span::new(2, 5, t), "b"),
            ],
        );
        assert_eq!(outside.apply(&db), None);
        assert_eq!(split.apply(&db), None);
        assert_eq!(overlap.apply(&db), None);
    }

    #[test]
    fn the_renderer_prints_each_fix_as_a_help_line_after_the_notes() {
        let (db, a, _) = two_files();
        let d = Diagnostic::error("E0060", "cannot assign to immutable variable `x`")
            .with_primary_label(Span::new(4, 5, a), "here")
            .with_note("a note")
            .with_fix(Fix::new(
                "make `x` mutable",
                vec![Edit::insert(4, a, "mut ")],
            ));
        let out = render::render_to_string(&db, &[d]);
        let note = out.find("= a note").unwrap_or_else(|| panic!("{out}"));
        let help = out
            .find("= help: make `x` mutable")
            .unwrap_or_else(|| panic!("{out}"));
        assert!(note < help, "{out}");
    }
}
