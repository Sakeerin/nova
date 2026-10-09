//! `nova package` and `nova publish` (spec
//! `docs/superpowers/specs/2026-10-09-phase-3-3b-index-and-publishing-design.md`
//! §6.5, §6.6, §6.8, §7).

use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail, Context, Result};
use nova_diagnostics::{render, Diagnostic, FileDb, Severity};
use nova_driver::{Program, Roots};
use nova_index::{
    Index, Line, LineDep, Packed, Reader, Source, SyncError, SyncRequest, LINE_VERSION,
};
use nova_pm::{Manifest, Offline, Unlock};

/// A package packed and verified.
struct Prepared {
    manifest: Manifest,
    packed: Packed,
    /// `target/package/<name>-<version>.nova-pkg`.
    path: PathBuf,
}

impl Prepared {
    /// Its index line (spec §3.1): its `[dependencies]` only, each
    /// requirement as `semver` prints it (plan decision 14).
    fn line(&self) -> Line {
        Line {
            name: self.manifest.package.name.clone(),
            vers: self.manifest.package.version.to_string(),
            deps: self
                .manifest
                .dependencies
                .iter()
                .filter_map(|d| {
                    d.version.as_ref().map(|req| LineDep {
                        name: d.name.clone(),
                        req: req.to_string(),
                    })
                })
                .collect(),
            cksum: self.packed.checksum.clone(),
            v: LINE_VERSION,
        }
    }
}

pub fn package() -> Result<()> {
    let index = Index::from_env().map_err(|e| anyhow!(e))?;
    let mut reader = nova_index::reader_for(&index);
    let prepared = prepare(&index, reader.as_mut())?;
    report(&prepared);
    Ok(())
}

pub fn publish() -> Result<()> {
    let index = Index::from_env().map_err(|e| anyhow!(e))?;
    match &index.source {
        Source::Local(_) => {
            let mut reader = nova_index::reader_for(&index);
            let prepared = prepare(&index, reader.as_mut())?;
            report(&prepared);
            nova_index::publish_local(&index, &prepared.line(), &prepared.packed.bytes)
                .map_err(|e| anyhow!(e))?;
            let package = &prepared.manifest.package;
            println!(
                "published {} {} to {}",
                package.name, package.version, index.canonical
            );
            Ok(())
        }
        Source::Http(_) => bail!(
            "`nova publish` writes only to a local index so far; {} is read over HTTP",
            index.canonical
        ),
    }
}

/// Pack the package around the current directory, then verify the
/// tarball (spec §6.5, §6.6). The tarball is written to `target/package/`
/// only once both pass.
fn prepare(index: &Index, reader: &mut dyn Reader) -> Result<Prepared> {
    let cwd = std::env::current_dir().context("reading the current directory")?;
    let root = nova_pm::find_root(&cwd).context("no nova.toml here or in any directory above")?;
    crate::project::refuse_cached(&root)?;
    let manifest_path = root.join(nova_pm::MANIFEST);
    let text = std::fs::read_to_string(&manifest_path)
        .with_context(|| format!("reading {}", manifest_path.display()))?;
    let mut db = FileDb::new();
    let file = db.add(manifest_path.display().to_string(), text.as_str());
    let (manifest, diagnostics) = nova_pm::parse(&text, file);
    let Some(manifest) =
        manifest.filter(|_| !diagnostics.iter().any(|d| d.severity == Severity::Error))
    else {
        render::emit_all(&db, &diagnostics);
        bail!("nova.toml has errors; nothing was packed");
    };
    let name = manifest.package.name.clone();
    if !root.join("src").join("lib.nova").is_file() {
        bail!("`{name}` has no src/lib.nova: only a library can be published");
    }
    let paths: Vec<Diagnostic> = manifest
        .dependencies
        .iter()
        .filter(|d| d.path.is_some())
        .map(|d| {
            Diagnostic::error(
                "M0017",
                format!(
                    "a published package cannot have a path dependency; publish `{}` first and \
                     depend on its version",
                    d.name
                ),
            )
            .with_primary_label(d.span, "a path dependency")
        })
        .collect();
    if !paths.is_empty() {
        render::emit_all(&db, &paths);
        bail!("`{name}` was not packed");
    }
    let version = manifest.package.version.clone();
    let packed = nova_index::pack(&root, &name, &version).map_err(|e| anyhow!(e))?;
    let mut deps: Vec<String> = manifest
        .dependencies
        .iter()
        .map(|d| d.name.clone())
        .collect();
    deps.sort();
    verify(&packed, &name, &version, &deps, index, reader)?;
    let path = root
        .join("target")
        .join("package")
        .join(format!("{name}-{version}.nova-pkg"));
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
    }
    std::fs::write(&path, &packed.bytes).with_context(|| format!("writing {}", path.display()))?;
    Ok(Prepared {
        manifest,
        packed,
        path,
    })
}

/// Spec §6.6: the tarball, unpacked in `<temp>/nova-verify-<pid>/` (plan
/// decision 7), must resolve with no lock and no dev-dependencies, then
/// check with no error and no warning. A dependency's warnings are never
/// shown, so every warning counted is the package's own (plan decision 15).
fn verify(
    packed: &Packed,
    name: &str,
    version: &semver::Version,
    deps: &[String],
    index: &Index,
    reader: &mut dyn Reader,
) -> Result<()> {
    let dir = std::env::temp_dir().join(format!("nova-verify-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let root = dir.join(format!("{name}-{version}"));
    let result = nova_index::unpack(&packed.bytes, name, version, deps, &root)
        .map_err(|e| anyhow!(e))
        .and_then(|()| check_unpacked(&root, index, reader));
    let _ = std::fs::remove_dir_all(&dir);
    result
}

fn check_unpacked(root: &Path, index: &Index, reader: &mut dyn Reader) -> Result<()> {
    let registry = nova_pm::registry_dir();
    let mut db = FileDb::new();
    let request = SyncRequest {
        root,
        manifest: None,
        dev: false,
        unlock: Unlock::All,
        write: false,
        index,
        reader,
        registry: registry.as_deref(),
    };
    let synced = match nova_index::sync(request, &mut db) {
        Ok(synced) => synced,
        Err(SyncError::Diagnostics(diagnostics)) => {
            render::emit_all(&db, &diagnostics);
            bail!("verification failed: the package's dependencies do not resolve; nothing was written");
        }
        Err(SyncError::Other(why)) => bail!("verification failed: {why}"),
    };
    for note in &synced.notes {
        eprintln!("note: {note}");
    }
    for package in &synced.unpacked {
        eprintln!("downloaded {package}");
    }
    let offline = Offline {
        registry,
        lock: synced.lock,
        dev: false,
    };
    let checked =
        nova_driver::check_program_counted(Program::for_package_in(root, Roots::Check, &offline))?;
    if checked.errors > 0 || checked.warnings > 0 {
        bail!(
            "verification failed: {} and {} in the packed package; nothing was written",
            count(checked.errors, "error"),
            count(checked.warnings, "warning")
        );
    }
    Ok(())
}

fn count(n: usize, word: &str) -> String {
    format!("{n} {word}{}", if n == 1 { "" } else { "s" })
}

fn report(prepared: &Prepared) {
    let package = &prepared.manifest.package;
    println!(
        "packed {} {}: {}",
        package.name,
        package.version,
        prepared.path.display()
    );
    println!(
        "  {}, {} bytes",
        count(prepared.packed.files, "file"),
        prepared.packed.bytes.len()
    );
    println!("  sha256 {}", prepared.packed.checksum);
}
