//! Writing the site to `target/doc/` (spec §4.3, §6.1).

use std::path::{Path, PathBuf};

use crate::model::Site;
use crate::{render, search, CSS, JS};

/// The file every site holds, so that `nova doc` replaces only a
/// directory it wrote (spec §4.3).
pub const MARKER: &str = ".nova-doc";

const MARKER_TEXT: &str =
    "This directory was written by nova doc, which replaces it on its next run.\n";

/// Why a site was not written.
#[derive(Debug, thiserror::Error)]
pub enum WriteError {
    /// The directory exists, holds files, and has no marker.
    #[error("{} exists and was not written by nova doc; remove it, and nova doc will write its site there", .0.display())]
    NotOurs(PathBuf),
    /// A file or directory could not be removed, read or written.
    #[error("cannot write {}: {}", .0.display(), .1)]
    Io(PathBuf, std::io::Error),
}

/// Write `site` to `dir`, replacing a site written there before (spec
/// §4.3). Returns the start page's path.
pub fn write_site(site: &Site, dir: &Path) -> Result<PathBuf, WriteError> {
    let io = |path: &Path| {
        let path = path.to_path_buf();
        move |e| WriteError::Io(path, e)
    };
    if dir.exists() {
        if dir.join(MARKER).is_file() {
            std::fs::remove_dir_all(dir).map_err(io(dir))?;
        } else if std::fs::read_dir(dir).map_err(io(dir))?.next().is_some() {
            return Err(WriteError::NotOurs(dir.to_path_buf()));
        }
    }
    let mut files = render::pages(site);
    files.push(("nova.css".to_string(), CSS.to_string()));
    files.push(("nova.js".to_string(), JS.to_string()));
    files.push(("search-index.js".to_string(), search::index(site)));
    files.push((MARKER.to_string(), MARKER_TEXT.to_string()));
    for (path, text) in &files {
        let full = under(dir, path);
        if let Some(parent) = full.parent() {
            std::fs::create_dir_all(parent).map_err(io(parent))?;
        }
        std::fs::write(&full, text.replace("\r\n", "\n")).map_err(io(&full))?;
    }
    Ok(under(dir, &site.start))
}

/// `relative`, a `/`-separated path, under `dir`.
fn under(dir: &Path, relative: &str) -> PathBuf {
    relative
        .split('/')
        .fold(dir.to_path_buf(), |path, part| path.join(part))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::sample_site;

    fn fresh(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("nova-doc-write-{name}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn the_sample_site_is_written_whole_and_its_links_hold() {
        let dir = fresh("whole").join("doc");
        let start = write_site(&sample_site(), &dir).unwrap();
        assert_eq!(start, dir.join("app").join("index.html"));
        for file in [
            MARKER,
            "index.html",
            "nova.css",
            "nova.js",
            "search-index.js",
            "app/index.html",
            "app/util/index.html",
            "std/index.html",
            "std/core/index.html",
            "std/primitive.Int.html",
        ] {
            assert!(dir.join(file).is_file(), "{file}");
        }
        assert_eq!(crate::check_links(&dir), Vec::new());
    }

    #[test]
    fn a_site_it_wrote_is_replaced_and_another_directory_is_refused() {
        let base = fresh("replace");
        let dir = base.join("doc");
        write_site(&sample_site(), &dir).unwrap();
        std::fs::write(dir.join("stale.html"), "old").unwrap();
        write_site(&sample_site(), &dir).unwrap();
        assert!(!dir.join("stale.html").exists());
        let other = base.join("other");
        std::fs::create_dir_all(&other).unwrap();
        std::fs::write(other.join("keep.txt"), "keep").unwrap();
        assert!(matches!(
            write_site(&sample_site(), &other),
            Err(WriteError::NotOurs(_))
        ));
        assert!(other.join("keep.txt").is_file());
        let empty = base.join("empty");
        std::fs::create_dir_all(&empty).unwrap();
        write_site(&sample_site(), &empty).unwrap();
    }

    #[test]
    fn every_file_ends_its_lines_with_a_line_feed() {
        let dir = fresh("endings").join("doc");
        write_site(&sample_site(), &dir).unwrap();
        for file in [
            "index.html",
            "nova.css",
            "nova.js",
            "search-index.js",
            "app/index.html",
        ] {
            let text = std::fs::read_to_string(dir.join(file)).unwrap();
            assert!(!text.contains('\r'), "{file}");
        }
    }
}
