//! Unpacking a downloaded package into the cache (spec §5.1, §5.2).

use std::collections::HashSet;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use semver::Version;

/// How much unpacking may write.
#[derive(Debug, Clone, Copy)]
pub struct Limits {
    /// Bytes of file contents.
    pub bytes: u64,
    /// Entries in the tarball.
    pub entries: usize,
}

/// 100 MiB and 10,000 entries (spec §5.2).
pub const LIMITS: Limits = Limits {
    bytes: 100 * 1024 * 1024,
    entries: 10_000,
};

/// [`unpack_limited`] within [`LIMITS`].
pub fn unpack(
    tarball: &[u8],
    name: &str,
    version: &Version,
    deps: &[String],
    dest: &Path,
) -> Result<(), String> {
    unpack_limited(tarball, name, version, deps, dest, &LIMITS)
}

/// Unpack `tarball`, the package `name` `version` whose index line names
/// `deps` (sorted), into `dest` (spec §5.2). Nothing is written to `dest`
/// unless every check passes:
/// - each entry is a file or a directory under `<name>-<version>/`, with
///   portable names, no two differing only in case;
/// - within `limits`;
/// - its `nova.toml` names `name` and `version`, has no path entry, and its
///   `[dependencies]` are `deps`.
///
/// The package is unpacked beside `dest` under a name made unique by the
/// process id and a counter, then renamed into place. If another process
/// got there first, its copy is used. When `dest` already holds a
/// `nova.toml`, nothing is done.
pub fn unpack_limited(
    tarball: &[u8],
    name: &str,
    version: &Version,
    deps: &[String],
    dest: &Path,
    limits: &Limits,
) -> Result<(), String> {
    if dest.join(nova_pm::MANIFEST).is_file() {
        return Ok(());
    }
    let parent = dest
        .parent()
        .ok_or_else(|| format!("{} has no parent directory", dest.display()))?;
    std::fs::create_dir_all(parent).map_err(|e| cannot_write(parent, &e))?;
    let temp = new_temp_dir(parent, &format!("{name}-{version}"))?;
    let result = extract(tarball, name, version, &temp, limits)
        .and_then(|()| check_manifest(&temp, name, version, deps));
    if let Err(error) = result {
        let _ = std::fs::remove_dir_all(&temp);
        return Err(error);
    }
    match std::fs::rename(&temp, dest) {
        Ok(()) => Ok(()),
        Err(error) => {
            let _ = std::fs::remove_dir_all(&temp);
            // Another process may have put its copy there first.
            if dest.join(nova_pm::MANIFEST).is_file() {
                Ok(())
            } else {
                Err(cannot_write(dest, &error))
            }
        }
    }
}

fn cannot_write(path: &Path, error: &std::io::Error) -> String {
    format!(
        "cannot write the package cache at {}: {error}; set NOVA_HOME to a writable directory",
        path.display()
    )
}

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

/// A new, empty directory in `parent` for unpacking `stem`. A name a killed
/// run left behind is never read: `create_dir` refuses it, and the next
/// number is tried.
fn new_temp_dir(parent: &Path, stem: &str) -> Result<PathBuf, String> {
    loop {
        let n = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let temp = parent.join(format!(".{stem}.{}-{n}.tmp", std::process::id()));
        match std::fs::create_dir(&temp) {
            Ok(()) => return Ok(temp),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(cannot_write(&temp, &error)),
        }
    }
}

/// Write the tarball's entries under `root`, checking each.
fn extract(
    tarball: &[u8],
    name: &str,
    version: &Version,
    root: &Path,
    limits: &Limits,
) -> Result<(), String> {
    let prefix = format!("{name}-{version}");
    let bad = |why: String| format!("the tarball of {name} {version} is refused: {why}");
    let mut archive = tar::Archive::new(flate2::read::GzDecoder::new(tarball));
    let entries = archive.entries().map_err(|e| bad(e.to_string()))?;
    let mut seen: HashSet<String> = HashSet::new();
    let mut count = 0usize;
    let mut written = 0u64;
    for entry in entries {
        let mut entry = entry.map_err(|e| bad(e.to_string()))?;
        count += 1;
        if count > limits.entries {
            return Err(bad(format!("it has more than {} entries", limits.entries)));
        }
        let path = String::from_utf8(entry.path_bytes().into_owned())
            .map_err(|_| bad("an entry's name is not UTF-8".to_string()))?;
        let kind = entry.header().entry_type();
        if kind != tar::EntryType::Regular && kind != tar::EntryType::Directory {
            return Err(bad(format!("`{path}` is not a file or a directory")));
        }
        let mut parts: Vec<&str> = path.split('/').collect();
        if kind == tar::EntryType::Directory && parts.last() == Some(&"") {
            parts.pop();
        }
        if parts.first() != Some(&prefix.as_str()) {
            return Err(bad(format!("`{path}` is not under {prefix}/")));
        }
        let rest = &parts[1..];
        if let Some(part) = rest.iter().find(|part| !nova_pm::is_portable(part)) {
            return Err(bad(format!("`{path}`: `{part}` is not a portable name")));
        }
        if rest.is_empty() {
            continue;
        }
        let rel = rest.join("/");
        if !seen.insert(rel.to_lowercase()) {
            return Err(bad(format!(
                "`{path}` differs only in case from another entry"
            )));
        }
        let target = rest.iter().fold(root.to_path_buf(), |p, part| p.join(part));
        if kind == tar::EntryType::Directory {
            std::fs::create_dir_all(&target).map_err(|e| cannot_write(&target, &e))?;
            continue;
        }
        if let Some(dir) = target.parent() {
            std::fs::create_dir_all(dir).map_err(|e| cannot_write(dir, &e))?;
        }
        let room = limits.bytes - written;
        let mut data = Vec::new();
        (&mut entry)
            .take(room + 1)
            .read_to_end(&mut data)
            .map_err(|e| bad(e.to_string()))?;
        if data.len() as u64 > room {
            return Err(bad(format!(
                "its contents pass the limit of {} bytes",
                limits.bytes
            )));
        }
        written += data.len() as u64;
        std::fs::write(&target, &data).map_err(|e| cannot_write(&target, &e))?;
    }
    Ok(())
}

/// The unpacked `nova.toml` must be the index line's (spec §5.2).
fn check_manifest(
    root: &Path,
    name: &str,
    version: &Version,
    deps: &[String],
) -> Result<(), String> {
    let bad = |why: String| format!("the downloaded {name} {version} is refused: {why}");
    let path = root.join(nova_pm::MANIFEST);
    let text = std::fs::read_to_string(&path).map_err(|_| bad("it has no nova.toml".into()))?;
    let (manifest, _) = nova_pm::parse(&text, nova_diagnostics::FileId::DUMMY);
    let manifest = manifest.ok_or_else(|| bad("its nova.toml has errors".into()))?;
    if manifest.package.name != name || manifest.package.version != *version {
        return Err(bad(format!(
            "its nova.toml is {} {}",
            manifest.package.name, manifest.package.version
        )));
    }
    if let Some(entry) = manifest.dependencies.iter().find(|d| d.path.is_some()) {
        return Err(bad(format!("it has a path dependency `{}`", entry.name)));
    }
    let mut names: Vec<String> = manifest
        .dependencies
        .iter()
        .map(|d| d.name.clone())
        .collect();
    names.sort();
    if names != deps {
        return Err(bad(format!(
            "its dependencies are [{}], and its index line's are [{}]",
            names.join(", "),
            deps.join(", ")
        )));
    }
    Ok(())
}
