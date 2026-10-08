//! The open documents, and the sources an analysis reads (spec
//! `docs/superpowers/specs/2026-10-08-phase-3-2-lsp-core-design.md` §6.2,
//! §6.9).

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use nova_driver::Sources;

use crate::uri;

/// A path as the server compares paths (decision 6):
/// - absolute, with its directory canonicalised when it exists;
/// - Windows' `\\?\` prefix stripped and `/` for `\`;
/// - on Windows, lower case.
///
/// A buffer and the driver's path for the same file always have the same
/// key.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PathKey(String);

impl PathKey {
    pub fn of(path: &Path) -> PathKey {
        let abs = if path.is_absolute() {
            path.to_path_buf()
        } else {
            std::env::current_dir()
                .map(|cwd| cwd.join(path))
                .unwrap_or_else(|_| path.to_path_buf())
        };
        let full = match (abs.parent(), abs.file_name()) {
            (Some(dir), Some(name)) => std::fs::canonicalize(dir)
                .unwrap_or_else(|_| dir.to_path_buf())
                .join(name),
            _ => abs,
        };
        let mut s = full.to_string_lossy().replace('\\', "/");
        if let Some(rest) = s.strip_prefix("//?/") {
            s = rest.to_string();
        }
        if cfg!(windows) {
            s = s.to_lowercase();
        }
        PathKey(s)
    }

    /// Whether this path is inside the directory `dir`'s key.
    pub fn is_under(&self, dir: &PathKey) -> bool {
        self.0
            .starts_with(&format!("{}/", dir.0.trim_end_matches('/')))
    }
}

/// An open document.
#[derive(Debug, Clone)]
pub struct Document {
    /// The client's URI for it, under which its diagnostics are published.
    pub uri: String,
    /// The path it is checked under.
    pub path: PathBuf,
    pub version: i32,
    pub text: String,
}

/// Every open document.
#[derive(Debug, Default)]
pub struct Workspace {
    docs: HashMap<PathKey, Document>,
}

impl Workspace {
    /// Open `uri`, and return its path, or `None` for a URI that names no
    /// file.
    pub fn open(&mut self, uri: &str, version: i32, text: String) -> Option<PathBuf> {
        let path = uri::document_path(uri)?;
        self.docs.insert(
            PathKey::of(&path),
            Document {
                uri: uri.to_string(),
                path: path.clone(),
                version,
                text,
            },
        );
        Some(path)
    }

    /// Replace an open document's text.
    pub fn change(&mut self, uri: &str, version: i32, text: String) -> Option<PathBuf> {
        let path = uri::document_path(uri)?;
        let doc = self.docs.get_mut(&PathKey::of(&path))?;
        doc.version = version;
        doc.text = text;
        Some(path)
    }

    /// Close a document, and return it.
    pub fn close(&mut self, uri: &str) -> Option<Document> {
        let path = uri::document_path(uri)?;
        self.docs.remove(&PathKey::of(&path))
    }

    pub fn get(&self, uri: &str) -> Option<&Document> {
        let path = uri::document_path(uri)?;
        self.docs.get(&PathKey::of(&path))
    }

    /// A copy of the open buffers, for an analysis on another thread.
    pub fn overlay(&self) -> Overlay {
        Overlay {
            buffers: self
                .docs
                .iter()
                .map(|(k, d)| (k.clone(), d.text.clone()))
                .collect(),
        }
    }
}

/// The open buffers, read in place of their files; every other file comes
/// from disk.
#[derive(Debug, Clone, Default)]
pub struct Overlay {
    buffers: HashMap<PathKey, String>,
}

impl Sources for Overlay {
    fn read(&self, path: &Path) -> std::io::Result<String> {
        match self.buffers.get(&PathKey::of(path)) {
            Some(text) => Ok(text.clone()),
            None => std::fs::read_to_string(path),
        }
    }

    fn same_file(&self, a: &Path, b: &Path) -> bool {
        PathKey::of(a) == PathKey::of(b)
    }
}

/// Whether `text` declares a top-level `fn main`. A loose file without one
/// is checked as a module (spec §6.2).
pub fn declares_main(text: &str) -> bool {
    let file = nova_diagnostics::FileId::DUMMY;
    let (tokens, _) = nova_lexer::lex(text, file);
    let parsed = nova_parser::parse_recovering(&tokens, file);
    parsed
        .file
        .items
        .iter()
        .any(|i| matches!(&i.value, nova_ast::Item::Function(f) if f.name.value == "main"))
}
