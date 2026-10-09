//! Downloading a package into the cache (spec §5.1, §5.2).

use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use nova_pm::LockedPackage;

use crate::cache::unpack;
use crate::http::Http;
use crate::line::Config;
use crate::location::{Index, Location};
use crate::pack::{sha256_hex, MAX_TARBALL};

/// Make `locked` available in `registry` (`$NOVA_HOME/registry`), and
/// return its unpacked directory, `src/<idx>/<name>-<version>` (spec
/// §5.1). Nothing is done when it is already unpacked. The tarball comes
/// from `cache/<idx>/` when one there has the locked checksum, and is
/// otherwise downloaded from where `config`'s `dl` says. A tarball is
/// written to the cache, and unpacked, only after its SHA-256 matches the
/// lock (spec §5.2).
pub fn fetch_package(
    index: &Index,
    config: &Config,
    http: &Http,
    locked: &LockedPackage,
    registry: &Path,
) -> Result<PathBuf, String> {
    let idx = nova_pm::index_dir_name(&index.canonical);
    let stem = format!("{}-{}", locked.name, locked.version);
    let dest = registry.join("src").join(&idx).join(&stem);
    if dest.join(nova_pm::MANIFEST).is_file() {
        return Ok(dest);
    }
    let cached = registry
        .join("cache")
        .join(&idx)
        .join(format!("{stem}.nova-pkg"));
    let bytes = match std::fs::read(&cached) {
        Ok(bytes) if sha256_hex(&bytes) == locked.checksum => bytes,
        _ => {
            let bytes = download(index, config, http, locked)?;
            let actual = sha256_hex(&bytes);
            if actual != locked.checksum {
                return Err(format!(
                    "the tarball of {} {} has SHA-256 {actual}, but nova.lock says {}; it was \
                     discarded",
                    locked.name, locked.version, locked.checksum
                ));
            }
            write_atomically(&cached, &bytes).map_err(|e| {
                format!(
                    "cannot write the package cache at {}: {e}; set NOVA_HOME to a writable \
                     directory",
                    cached.display()
                )
            })?;
            bytes
        }
    };
    unpack(
        &bytes,
        &locked.name,
        &locked.version,
        &locked.dependencies,
        &dest,
    )?;
    Ok(dest)
}

fn download(
    index: &Index,
    config: &Config,
    http: &Http,
    locked: &LockedPackage,
) -> Result<Vec<u8>, String> {
    let version = locked.version.to_string();
    match index.tarball(&config.dl, &locked.name, &version)? {
        Location::Url(url) => http.get(&url, MAX_TARBALL)?.ok_or_else(|| {
            format!(
                "the index has no tarball for {} {version} at {}",
                locked.name,
                crate::http::shown(&url)
            )
        }),
        Location::File(path) => {
            let size = std::fs::metadata(&path)
                .map_err(|e| format!("cannot read {}: {e}", path.display()))?
                .len();
            if size > MAX_TARBALL {
                return Err(format!(
                    "{} is {size} bytes; the limit is 10 MiB",
                    path.display()
                ));
            }
            std::fs::read(&path).map_err(|e| format!("cannot read {}: {e}", path.display()))
        }
    }
}

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Write `bytes` to `path` through a temporary file beside it, unique to
/// this process, renamed into place. `path`'s directory is created.
pub(crate) fn write_atomically(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    write_new(path, bytes, false)
}

/// [`write_atomically`], the temporary file created readable by its owner
/// only (mode 0600) on Unix before anything is written to it (spec §6.7).
/// On Windows it takes its directory's permissions.
pub(crate) fn write_private(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    write_new(path, bytes, true)
}

fn write_new(path: &Path, bytes: &[u8], private: bool) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        if !dir.as_os_str().is_empty() {
            std::fs::create_dir_all(dir)?;
        }
    }
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let (mut file, temp) = loop {
        let n = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let temp = path.with_file_name(format!(".{name}.{}-{n}.tmp", std::process::id()));
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            if private {
                options.mode(0o600);
            }
        }
        #[cfg(not(unix))]
        let _ = private;
        match options.open(&temp) {
            Ok(file) => break (file, temp),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    };
    let written = file.write_all(bytes).and_then(|()| file.flush());
    drop(file);
    let result = written.and_then(|()| std::fs::rename(&temp, path));
    if result.is_err() {
        let _ = std::fs::remove_file(&temp);
    }
    result
}
