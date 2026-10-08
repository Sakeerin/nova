//! Finding a project (spec §6.1).

use std::path::{Path, PathBuf};

/// A project's manifest file name.
pub const MANIFEST: &str = "nova.toml";

/// The nearest directory at or above `start` that holds a `nova.toml`
/// file.
pub fn find_root(start: &Path) -> Option<PathBuf> {
    start
        .ancestors()
        .find(|dir| dir.join(MANIFEST).is_file())
        .map(Path::to_path_buf)
}

/// `path`, canonicalised when it exists, without Windows' `\\?\` prefix, so
/// that it is spelled as the file system spells it. A package is identified
/// by its directory's real path (spec 3.3a §3.3), and the language server
/// builds unopened files' URIs on it.
pub fn real_path(path: &Path) -> PathBuf {
    let real = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    match real.to_str().and_then(|s| s.strip_prefix(r"\\?\")) {
        Some(rest) => PathBuf::from(rest),
        None => real,
    }
}
