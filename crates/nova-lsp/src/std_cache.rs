//! Std's sources on disk, so that a definition can open them (spec
//! `docs/superpowers/specs/2026-10-10-phase-3-4a-navigation-design.md`
//! §6).
//!
//! `$NOVA_HOME/std/<version>-<crc32>/`, one read-only file per module. A
//! file is compared byte for byte with the embedded text and rewritten
//! through a unique temporary name when it differs.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use crate::workspace::PathKey;

/// Each std module's short name, as in `<std/core>`, and its text, in
/// `STD_MODULES` order then `std/test`.
fn sources() -> Vec<(&'static str, &'static str)> {
    nova_resolver::STD_MODULES
        .iter()
        .chain(std::iter::once(&nova_resolver::STD_TEST_MODULE))
        .map(|&(name, text)| (name.strip_prefix("$std.").unwrap_or(name), text))
        .collect()
}

fn crc() -> u32 {
    let mut hasher = crc32fast::Hasher::new();
    for (_, text) in sources() {
        hasher.update(text.as_bytes());
    }
    hasher.finalize()
}

/// The cache's directory under `home`. The version is the server's, the
/// one `serverInfo` reports; the CRC-32 keeps two builds apart.
pub fn dir(home: &Path) -> PathBuf {
    home.join("std")
        .join(format!("{}-{:08x}", env!("CARGO_PKG_VERSION"), crc()))
}

/// The cache, written where it is not already right. `None`, after a
/// warning logged once per server, when there is no `$NOVA_HOME` or a
/// write fails.
pub fn ensure() -> Option<PathBuf> {
    let Some(home) = nova_pm::nova_home_from_env() else {
        warn_once("there is no NOVA_HOME and no home directory");
        return None;
    };
    let dir = dir(&home);
    match write_all(&dir) {
        Ok(()) => Some(dir),
        Err(e) => {
            warn_once(&format!("cannot write {}: {e}", dir.display()));
            None
        }
    }
}

/// The std module a path in the cache holds, by its file name.
pub fn module_of(path: &Path) -> Option<String> {
    if !in_std_cache(path) {
        return None;
    }
    let stem = path.file_stem()?.to_str()?;
    sources()
        .iter()
        .any(|(short, _)| *short == stem)
        .then(|| stem.to_string())
}

/// Whether `path` is under `$NOVA_HOME/std/`.
pub fn in_std_cache(path: &Path) -> bool {
    nova_pm::nova_home_from_env()
        .is_some_and(|home| PathKey::of(path).is_under(&PathKey::of(&home.join("std"))))
}

fn write_all(dir: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    for (short, text) in sources() {
        write_one(&dir.join(format!("{short}.nova")), text)?;
    }
    Ok(())
}

static TEMPS: AtomicU64 = AtomicU64::new(0);

/// Make `path` hold `text`, read-only.
fn write_one(path: &Path, text: &str) -> std::io::Result<()> {
    let right = |p: &Path| std::fs::read(p).is_ok_and(|bytes| bytes == text.as_bytes());
    if !right(path) {
        let temp = path.with_extension(format!(
            "nova.tmp-{}-{}",
            std::process::id(),
            TEMPS.fetch_add(1, Ordering::Relaxed)
        ));
        {
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temp)?;
            file.write_all(text.as_bytes())?;
        }
        // Windows refuses to rename over a read-only file.
        #[cfg(windows)]
        if path.exists() {
            let _ = set_read_only(path, false);
        }
        if let Err(e) = std::fs::rename(&temp, path) {
            let _ = std::fs::remove_file(&temp);
            // Another server may have written it first.
            if !right(path) {
                return Err(e);
            }
        }
    }
    set_read_only(path, true)
}

fn set_read_only(path: &Path, on: bool) -> std::io::Result<()> {
    let mut perms = std::fs::metadata(path)?.permissions();
    if perms.readonly() != on {
        perms.set_readonly(on);
        std::fs::set_permissions(path, perms)?;
    }
    Ok(())
}

static WARNED: AtomicBool = AtomicBool::new(false);

fn warn_once(why: &str) {
    if !WARNED.swap(true, Ordering::Relaxed) {
        tracing::warn!("nova lsp: std's sources are not on disk, so no location in std: {why}");
    }
}
