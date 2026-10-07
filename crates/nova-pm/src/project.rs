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
