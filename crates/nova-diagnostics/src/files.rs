//! Source file database — maps `FileId` to source text.

use std::collections::HashMap;

/// An opaque identifier for a source file in the `FileDb`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct FileId(u32);

impl FileId {
    /// A sentinel `FileId` used in tests and synthesized spans.
    pub const DUMMY: Self = Self(u32::MAX);
}

/// Database of source files, keyed by `FileId`.
#[derive(Debug, Default)]
pub struct FileDb {
    files: Vec<SourceFile>,
    by_name: HashMap<String, FileId>,
}

#[derive(Debug)]
struct SourceFile {
    name: String,
    source: String,
    /// Byte offsets of the start of each line (for fast line/column lookup).
    line_starts: Vec<u32>,
}

impl SourceFile {
    fn new(name: String, source: String) -> Self {
        let line_starts = std::iter::once(0)
            .chain(source.match_indices('\n').map(|(i, _)| (i + 1) as u32))
            .collect();
        Self {
            name,
            source,
            line_starts,
        }
    }
}

impl FileDb {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a file and return its `FileId`.
    pub fn add(&mut self, name: impl Into<String>, source: impl Into<String>) -> FileId {
        let name = name.into();
        let source = source.into();
        let id = FileId(self.files.len() as u32);
        self.by_name.insert(name.clone(), id);
        self.files.push(SourceFile::new(name, source));
        id
    }

    pub fn get_source(&self, id: FileId) -> Option<&str> {
        self.files.get(id.0 as usize).map(|f| f.source.as_str())
    }

    pub fn get_name(&self, id: FileId) -> Option<&str> {
        self.files.get(id.0 as usize).map(|f| f.name.as_str())
    }

    /// Returns (1-based line, 1-based column) for a byte offset.
    pub fn location(&self, id: FileId, offset: u32) -> Option<(usize, usize)> {
        let file = self.files.get(id.0 as usize)?;
        let line_idx = file
            .line_starts
            .partition_point(|&start| start <= offset)
            .saturating_sub(1);
        let line_start = file.line_starts[line_idx] as usize;
        let col = offset as usize - line_start + 1;
        Some((line_idx + 1, col))
    }
}
