//! `nova package`'s tarball (spec §6.5).

use std::io::Write;
use std::path::{Path, PathBuf};

use flate2::{Compression, GzBuilder};
use semver::Version;

/// The largest tarball nova packs or downloads (spec §5.2, §6.5).
pub const MAX_TARBALL: u64 = 10 * 1024 * 1024;

/// A packed package.
#[derive(Debug, Clone)]
pub struct Packed {
    pub bytes: Vec<u8>,
    /// How many files it holds.
    pub files: usize,
    /// The SHA-256 of `bytes`.
    pub checksum: String,
}

/// `bytes`'s SHA-256, as 64 lower-case hex digits.
pub fn sha256_hex(bytes: &[u8]) -> String {
    let digest = ring::digest::digest(&ring::digest::SHA256, bytes);
    digest
        .as_ref()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// Pack the package at `root` as `name` `version` (spec §6.5): its
/// `nova.toml`, `src/`, `tests/`, and top-level `README*` and `LICENSE*`
/// files, leaving out any name that starts with `.`. Every entry is under
/// `<name>-<version>/`, sorted, with fixed times, modes and owners, so the
/// same source always packs to the same bytes.
///
/// A symbolic link or a name that is not portable, anywhere in what would
/// be packed, refuses the whole package, as does a tarball over 10 MiB.
pub fn pack(root: &Path, name: &str, version: &Version) -> Result<Packed, String> {
    let mut files: Vec<(String, PathBuf)> = Vec::new();
    for entry in sorted_names(root)? {
        let path = root.join(&entry);
        let included = entry == "nova.toml"
            || entry == "src"
            || entry == "tests"
            || entry.starts_with("README")
            || entry.starts_with("LICENSE");
        if !included {
            continue;
        }
        let kind = kind_of(&path, &entry)?;
        match kind {
            Kind::Dir if entry == "src" || entry == "tests" => walk(&path, &entry, &mut files)?,
            Kind::File => files.push((entry.clone(), path)),
            Kind::Dir => {}
        }
    }
    let mut lower: Vec<(String, &str)> = files
        .iter()
        .map(|(rel, _)| (rel.to_lowercase(), rel.as_str()))
        .collect();
    lower.sort();
    if let Some(pair) = lower.windows(2).find(|pair| pair[0].0 == pair[1].0) {
        return Err(format!(
            "`{}` and `{}` differ only in case, which Windows and macOS cannot hold apart",
            pair[0].1, pair[1].1
        ));
    }
    files.sort();
    let prefix = format!("{name}-{version}");
    let mut tar = tar::Builder::new(Vec::new());
    for (rel, path) in &files {
        let data =
            std::fs::read(path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
        let mut header = tar::Header::new_gnu();
        header.set_entry_type(tar::EntryType::Regular);
        header.set_size(data.len() as u64);
        header.set_mode(0o644);
        header.set_mtime(0);
        header.set_uid(0);
        header.set_gid(0);
        tar.append_data(&mut header, format!("{prefix}/{rel}"), data.as_slice())
            .map_err(|e| format!("cannot pack {rel}: {e}"))?;
    }
    let tar = tar.into_inner().map_err(|e| format!("cannot pack: {e}"))?;
    let mut gz = GzBuilder::new()
        .mtime(0)
        .operating_system(255)
        .write(Vec::new(), Compression::default());
    gz.write_all(&tar)
        .map_err(|e| format!("cannot compress: {e}"))?;
    let bytes = gz.finish().map_err(|e| format!("cannot compress: {e}"))?;
    if bytes.len() as u64 > MAX_TARBALL {
        return Err(format!(
            "the package is {} bytes packed; the limit is 10 MiB",
            bytes.len()
        ));
    }
    let checksum = sha256_hex(&bytes);
    Ok(Packed {
        bytes,
        files: files.len(),
        checksum,
    })
}

enum Kind {
    File,
    Dir,
}

/// What `path` is, refusing a link (packing never follows one), anything
/// but a file or a directory, and a name that is not portable.
fn kind_of(path: &Path, rel: &str) -> Result<Kind, String> {
    let metadata = std::fs::symlink_metadata(path)
        .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    if metadata.file_type().is_symlink() {
        return Err(format!(
            "{rel} is a symbolic link; nova package never follows links"
        ));
    }
    let name = rel.rsplit('/').next().unwrap_or(rel);
    if !nova_pm::is_portable(name) {
        return Err(format!(
            "{rel}: `{name}` is not a name every system can hold (spec §6.5)"
        ));
    }
    if metadata.is_dir() {
        Ok(Kind::Dir)
    } else if metadata.is_file() {
        Ok(Kind::File)
    } else {
        Err(format!("{rel} is neither a file nor a directory"))
    }
}

/// Every file under `dir`, whose path in the package is `rel`, leaving out
/// names that start with `.`.
fn walk(dir: &Path, rel: &str, files: &mut Vec<(String, PathBuf)>) -> Result<(), String> {
    for entry in sorted_names(dir)? {
        if entry.starts_with('.') {
            continue;
        }
        let path = dir.join(&entry);
        let child = format!("{rel}/{entry}");
        match kind_of(&path, &child)? {
            Kind::Dir => walk(&path, &child, files)?,
            Kind::File => files.push((child, path)),
        }
    }
    Ok(())
}

/// The names in `dir`, sorted. A name that is not UTF-8 is refused.
fn sorted_names(dir: &Path) -> Result<Vec<String>, String> {
    let entries =
        std::fs::read_dir(dir).map_err(|e| format!("cannot read {}: {e}", dir.display()))?;
    let mut names = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| format!("cannot read {}: {e}", dir.display()))?;
        let name = entry.file_name().into_string().map_err(|name| {
            format!(
                "{}: `{}` is not UTF-8",
                dir.display(),
                name.to_string_lossy()
            )
        })?;
        names.push(name);
    }
    names.sort();
    Ok(names)
}
