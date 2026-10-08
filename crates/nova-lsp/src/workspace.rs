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

    /// The projects the open documents belong to, each once.
    pub fn projects(&self) -> Vec<ProjectKey> {
        let mut out: Vec<ProjectKey> = Vec::new();
        for doc in self.docs.values() {
            let p = ProjectKey::of(&doc.path);
            if !out.contains(&p) {
                out.push(p);
            }
        }
        out
    }

    /// The open documents that belong to `project`.
    pub fn open_in(&self, project: &ProjectKey) -> Vec<Document> {
        self.docs
            .values()
            .filter(|d| project.holds(&d.path))
            .cloned()
            .collect()
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

/// A project, or a loose file (spec §6.2). Two keys are equal when their
/// paths' `PathKey`s are.
#[derive(Debug, Clone)]
pub enum ProjectKey {
    /// A directory holding `nova.toml`, in its real spelling; its entry is
    /// `src/main.nova`.
    Root(PathBuf),
    /// A file in no project: its own entry.
    Loose(PathBuf),
}

impl ProjectKey {
    /// The project `path` belongs to: the nearest directory above it that
    /// holds `nova.toml`, as `nova run` finds it, or none.
    pub fn of(path: &Path) -> ProjectKey {
        let dir = path.parent().unwrap_or(path);
        match nova_pm::find_root(dir) {
            Some(root) => ProjectKey::Root(real_path(&root)),
            None => ProjectKey::Loose(path.to_path_buf()),
        }
    }

    /// The file the project's analysis starts from.
    pub fn entry(&self) -> PathBuf {
        match self {
            ProjectKey::Root(dir) => dir.join("src").join("main.nova"),
            ProjectKey::Loose(file) => file.clone(),
        }
    }

    /// Whether `path` belongs to this project.
    pub fn holds(&self, path: &Path) -> bool {
        match self {
            ProjectKey::Root(dir) => PathKey::of(path).is_under(&PathKey::of(dir)),
            ProjectKey::Loose(file) => PathKey::of(path) == PathKey::of(file),
        }
    }

    fn id(&self) -> (bool, PathKey) {
        match self {
            ProjectKey::Root(dir) => (true, PathKey::of(dir)),
            ProjectKey::Loose(file) => (false, PathKey::of(file)),
        }
    }
}

impl PartialEq for ProjectKey {
    fn eq(&self, other: &Self) -> bool {
        self.id() == other.id()
    }
}

impl Eq for ProjectKey {}

impl std::hash::Hash for ProjectKey {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id().hash(state);
    }
}

/// `path`, canonicalised when it exists, without Windows' `\\?\` prefix, so
/// that it is spelled as the file system spells it. The URIs of unopened
/// files are made from paths built on it, and VS Code keeps an opened
/// file's real spelling.
pub fn real_path(path: &Path) -> PathBuf {
    let real = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    match real.to_str().and_then(|s| s.strip_prefix(r"\\?\")) {
        Some(rest) => PathBuf::from(rest),
        None => real,
    }
}
