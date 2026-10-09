//! Publishing to a local index (spec §7, "A local index"). GitHub
//! publishing is in `github.rs`.

use crate::download::write_atomically;
use crate::line::{parse_lines, Line};
use crate::location::{index_path, Index, Location, Source};
use crate::read::{LocalReader, Reader};

/// Spec §7, step 1: `line` must be a version not yet in the index, under a
/// name spelled as any already in its file (§3.1). `existing` is the
/// file's text, and `file` its path in the index.
pub fn check_new(existing: &str, file: &str, line: &Line) -> Result<(), String> {
    let (lines, _) = parse_lines(existing, file);
    if let Some(other) = lines.iter().find(|l| l.name != line.name) {
        return Err(format!(
            "the index has `{}`, which differs from `{}` only in case; a name keeps the \
             spelling it was first published with",
            other.name, line.name
        ));
    }
    if lines.iter().any(|l| l.vers == line.vers) {
        return Err(format!(
            "{} {} is already in the index; a published version is never replaced",
            line.name, line.vers
        ));
    }
    Ok(())
}

/// Publish `tarball` and `line` to `index`, a local directory: the tarball
/// where `config.json`'s `dl` says, then the line appended to the
/// package's file, each through a temporary file renamed into place. The
/// line is the commit point: until it is written, nothing points at the
/// tarball.
pub fn publish_local(index: &Index, line: &Line, tarball: &[u8]) -> Result<(), String> {
    let Source::Local(dir) = &index.source else {
        return Err(format!("{} is not a local index", index.canonical));
    };
    let mut reader = LocalReader { dir: dir.clone() };
    let config = reader.config()?;
    let file = index_path(&line.name);
    let existing = reader.file(&file)?.unwrap_or_default();
    check_new(&existing, &file, line)?;
    let Location::File(target) = index.tarball(&config.dl, &line.name, &line.vers)? else {
        return Err(
            "a local index's `dl` in config.json must be a path under the index, not a URL"
                .to_string(),
        );
    };
    write_atomically(&target, tarball)
        .map_err(|e| format!("cannot write {}: {e}", target.display()))?;
    let mut text = existing;
    if !text.is_empty() && !text.ends_with('\n') {
        text.push('\n');
    }
    text.push_str(&line.to_json());
    text.push('\n');
    let path = file
        .split('/')
        .fold(dir.clone(), |path, part| path.join(part));
    write_atomically(&path, text.as_bytes())
        .map_err(|e| format!("cannot write {}: {e}", path.display()))
}
