//! A file's line endings and final newline (spec
//! `docs/superpowers/specs/2026-10-07-phase-3-1-formatter-design.md` §5.1,
//! §7.3).

use std::path::{Path, PathBuf};

use crate::{editorconfig, FormatError};

/// A file's line ending.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LineEnding {
    /// `\n`.
    Lf,
    /// `\r\n`.
    Crlf,
}

impl LineEnding {
    /// The line ending of `text`'s first line break, or LF if it has none
    /// (spec §7.3).
    pub fn of(text: &str) -> LineEnding {
        match text.find('\n') {
            Some(i) if text[..i].ends_with('\r') => LineEnding::Crlf,
            _ => LineEnding::Lf,
        }
    }

    /// `lf`, a text with `\n` line endings, in this line ending.
    fn apply(self, lf: &str) -> String {
        match self {
            LineEnding::Lf => lf.to_owned(),
            LineEnding::Crlf => lf.replace('\n', "\r\n"),
        }
    }
}

/// A file, formatted: what it holds, and what it should hold.
#[derive(Debug)]
pub struct Formatted {
    /// The file as it is.
    pub original: String,
    /// The file as `nova fmt` would write it.
    pub formatted: String,
}

impl Formatted {
    /// Whether formatting would change the file.
    pub fn changed(&self) -> bool {
        self.original != self.formatted
    }
}

/// Why a file was not formatted (spec §7.2).
#[derive(Debug, thiserror::Error)]
pub enum FileError {
    /// It could not be read.
    #[error("{0}")]
    Io(std::io::Error),
    /// It is not UTF-8.
    #[error("the file is not UTF-8")]
    NotUtf8,
    /// It does not lex or parse, or the self-check refused the output.
    #[error(transparent)]
    Format(FormatError),
}

/// Format the file at `path` (spec §5.1): [`crate::format`]'s output, with
/// the line ending and the final newline §7.3 gives it. Nothing is written.
pub fn format_file(path: &Path) -> Result<Formatted, FileError> {
    let bytes = std::fs::read(path).map_err(FileError::Io)?;
    let original = String::from_utf8(bytes).map_err(|_| FileError::NotUtf8)?;
    let settings = editorconfig::settings_for(&absolute(path));
    let ending = match settings.crlf {
        Some(true) => LineEnding::Crlf,
        Some(false) => LineEnding::Lf,
        None => LineEnding::of(&original),
    };
    let mut lf =
        crate::format_named(&original, &path.display().to_string()).map_err(FileError::Format)?;
    // With `insert_final_newline = false`, the file ends in a newline only
    // if it did (the 3.1 plan's decision 13).
    if settings.insert_final_newline == Some(false) && !original.ends_with('\n') {
        lf.pop();
    }
    Ok(Formatted {
        formatted: ending.apply(&lf),
        original,
    })
}

/// Format `source` from standard input (spec §7.1): [`crate::format`]'s
/// output, in the input's own line ending.
pub fn format_text(source: &str) -> Result<String, FormatError> {
    let lf = crate::format(source)?;
    Ok(LineEnding::of(source).apply(&lf))
}

/// `path`, made absolute against the current directory, so that the
/// `.editorconfig` search can walk all the way up.
fn absolute(path: &Path) -> PathBuf {
    if path.is_absolute() {
        return path.to_path_buf();
    }
    match std::env::current_dir() {
        Ok(cwd) => cwd.join(path),
        Err(_) => path.to_path_buf(),
    }
}
