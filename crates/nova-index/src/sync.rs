//! The sync step (spec §4.4, §4.6, §5.3): resolve when the lock needs it,
//! download what the cache lacks, and write `nova.lock` last, so a lock
//! never names a package that is not unpacked.

use std::collections::{BTreeMap, VecDeque};
use std::path::Path;

use nova_diagnostics::{Diagnostic, FileDb, Severity};
use nova_pm::{parse_lock, Lock, LockedPackage, Requirement, ResolveError, Unlock, LOCKFILE};

use crate::download::{fetch_package, write_atomically};
use crate::http::Http;
use crate::location::Index;
use crate::read::{Reader, View};

/// What to sync.
pub struct SyncRequest<'a> {
    /// The root package's directory, empty for the current directory.
    pub root: &'a Path,
    /// The root's `nova.toml` when it is not the file on disk: `nova add`
    /// syncs the manifest it would write.
    pub manifest: Option<&'a str>,
    /// Whether the root's dev-dependencies are resolved. Publishing's
    /// verification resolves none.
    pub dev: bool,
    pub unlock: Unlock,
    /// Whether to write `nova.lock`. `nova add` writes it itself, with
    /// `nova.toml`, once both pass.
    pub write: bool,
    pub index: &'a Index,
    /// How index files are read: for a GitHub index while publishing,
    /// through the API; otherwise [`crate::reader_for`]'s.
    pub reader: &'a mut dyn Reader,
    /// `$NOVA_HOME/registry`. `None` when `$NOVA_HOME` cannot be found,
    /// which is an error only when something must be downloaded (spec
    /// §5.1).
    pub registry: Option<&'a Path>,
}

/// What a sync did.
#[derive(Debug)]
pub struct Synced {
    /// The lock as it now stands. `None` when the project has no registry
    /// dependency and never had a lock.
    pub lock: Option<Lock>,
    /// The lock before the sync.
    pub before: Option<Lock>,
    /// Each package downloaded and unpacked, as `<name> <version>`.
    pub unpacked: Vec<String>,
    /// Notes about index lines that do not parse (spec §3.1).
    pub notes: Vec<String>,
}

/// Why a sync stopped.
#[derive(Debug)]
pub enum SyncError {
    /// Manifest, graph or resolution diagnostics, to render with the
    /// `FileDb` the sync was given.
    Diagnostics(Vec<Diagnostic>),
    /// An index that cannot be reached, a cache that cannot be written.
    Other(String),
}

/// Sync the package at `request.root` (spec §5.3).
pub fn sync(request: SyncRequest<'_>, db: &mut FileDb) -> Result<Synced, SyncError> {
    let SyncRequest {
        root,
        manifest,
        dev,
        unlock,
        write,
        index,
        reader,
        registry,
    } = request;
    let (requirements, diagnostics) = nova_pm::requirements(root, manifest, dev, db);
    if diagnostics.iter().any(|d| d.severity == Severity::Error) {
        return Err(SyncError::Diagnostics(diagnostics));
    }
    let before = match read_lock(root, db) {
        Ok(lock) => lock,
        // `nova update` ignores the lock, so it replaces an unreadable one.
        Err(_) if unlock == Unlock::All => None,
        Err(diagnostic) => return Err(SyncError::Diagnostics(vec![diagnostic])),
    };
    if requirements.is_empty() && before.is_none() {
        return Ok(Synced {
            lock: None,
            before,
            unpacked: Vec::new(),
            notes: Vec::new(),
        });
    }
    // A lock from another index pins nothing (spec §4.4).
    let usable = before.as_ref().filter(|lock| lock.index == index.canonical);
    let mut notes = Vec::new();
    let packages = match usable {
        _ if requirements.is_empty() => Vec::new(),
        Some(lock) if unlock == Unlock::Nothing && answers(&requirements, lock) => {
            reached(&requirements, lock)
        }
        _ => {
            let mut view = View::new(&mut *reader);
            let result = nova_pm::resolve(&requirements, usable, &unlock, &mut view);
            notes = std::mem::take(&mut view.notes);
            result.map_err(|error| match error {
                ResolveError::Diagnostic(diagnostic) => SyncError::Diagnostics(vec![diagnostic]),
                ResolveError::Index(why) => SyncError::Other(unreachable(index, &why)),
            })?
        }
    };
    let lock = Lock {
        index: index.canonical.clone(),
        packages,
    };
    // A published version never changes (spec §3.1, §8): a version the
    // lock already holds keeps its checksum, whatever the index now says.
    if let Some(old) = usable {
        for package in &lock.packages {
            let kept = old
                .find(&package.name)
                .filter(|locked| locked.version == package.version);
            if let Some(locked) = kept.filter(|locked| locked.checksum != package.checksum) {
                return Err(SyncError::Other(format!(
                    "the index now gives {} {} the SHA-256 {}, but nova.lock records {}; a \
                     published version never changes, so nothing was changed",
                    package.name, package.version, package.checksum, locked.checksum
                )));
            }
        }
    }
    let idx = nova_pm::index_dir_name(&index.canonical);
    let missing: Vec<&LockedPackage> = lock
        .packages
        .iter()
        .filter(|p| {
            !registry.is_some_and(|registry| {
                registry
                    .join("src")
                    .join(&idx)
                    .join(format!("{}-{}", p.name, p.version))
                    .join(nova_pm::MANIFEST)
                    .is_file()
            })
        })
        .collect();
    let mut unpacked = Vec::new();
    if !missing.is_empty() {
        // $NOVA_HOME is needed only when something must be downloaded.
        let registry = registry.ok_or_else(|| {
            SyncError::Other(
                "cannot place the package cache: set NOVA_HOME to a writable directory".into(),
            )
        })?;
        let config = reader
            .config()
            .map_err(|why| SyncError::Other(unreachable(index, &why)))?;
        let http = Http::new();
        for package in missing {
            fetch_package(index, &config, &http, package, registry).map_err(SyncError::Other)?;
            unpacked.push(format!("{} {}", package.name, package.version));
        }
    }
    if write && (before.as_ref() != Some(&lock) || !unpacked.is_empty()) {
        write_lock(root, &lock).map_err(SyncError::Other)?;
    }
    Ok(Synced {
        lock: Some(lock),
        before,
        unpacked,
        notes,
    })
}

fn unreachable(index: &Index, why: &str) -> String {
    format!("cannot reach the index at {}: {why}", index.canonical)
}

/// The root's `nova.lock`: `None` when there is none, M0016 when it cannot
/// be read.
fn read_lock(root: &Path, db: &mut FileDb) -> Result<Option<Lock>, Diagnostic> {
    let path = root.join(LOCKFILE);
    match std::fs::read_to_string(&path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(
            Diagnostic::error("M0016", format!("nova.lock cannot be read: {error}"))
                .with_note("delete nova.lock, or run `nova update`"),
        ),
        Ok(text) => {
            let file = db.add(path.display().to_string(), text.as_str());
            parse_lock(&text, file).map(Some)
        }
    }
}

/// Whether `lock` already answers `requirements` (spec §4.4): each one is
/// locked at a version that meets it, and every package reached has its
/// own dependencies locked.
fn answers(requirements: &[Requirement], lock: &Lock) -> bool {
    requirements.iter().all(|requirement| {
        lock.find(&requirement.name)
            .is_some_and(|locked| requirement.req.matches(&locked.version))
    }) && reached(requirements, lock).iter().all(|package| {
        package
            .dependencies
            .iter()
            .all(|name| lock.find(name).is_some())
    })
}

/// The locked packages `requirements` reach, sorted by name: the lock,
/// pruned (spec §4.6).
fn reached(requirements: &[Requirement], lock: &Lock) -> Vec<LockedPackage> {
    let mut found: BTreeMap<String, LockedPackage> = BTreeMap::new();
    let mut queue: VecDeque<String> = requirements.iter().map(|r| r.name.clone()).collect();
    while let Some(name) = queue.pop_front() {
        if found.contains_key(&name) {
            continue;
        }
        if let Some(package) = lock.find(&name) {
            queue.extend(package.dependencies.iter().cloned());
            found.insert(name, package.clone());
        }
    }
    found.into_values().collect()
}

/// Write `lock` as `root`'s `nova.lock`, through a temporary file renamed
/// into place.
pub fn write_lock(root: &Path, lock: &Lock) -> Result<(), String> {
    let path = root.join(LOCKFILE);
    write_atomically(&path, lock.to_text().as_bytes())
        .map_err(|e| format!("cannot write {}: {e}", path.display()))
}
