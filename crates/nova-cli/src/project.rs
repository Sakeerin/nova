//! Which file a command works on, and where `build` writes, now that a
//! directory can be a project (spec
//! `docs/superpowers/specs/2026-10-07-phase-3-0-foundations-design.md` §6).

use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use nova_diagnostics::{render, Diagnostic, FileDb, Severity};
use nova_pm::Manifest;

/// What a command works on.
pub enum Mode {
    /// One file: a file argument, or `src/main.nova` outside any project.
    File(PathBuf),
    /// A project, found by walking up to its `nova.toml`.
    Project {
        /// `src/main.nova`: relative when the current directory is the
        /// project's root, as it has always been there, and absolute from
        /// anywhere else.
        entry: PathBuf,
        /// `target`, relative or absolute in the same way.
        target_dir: PathBuf,
        /// The package's name, which `build` names its output after.
        name: String,
    },
}

impl Mode {
    /// The source file the command compiles.
    pub fn entry(&self) -> &Path {
        match self {
            Mode::File(file) => file,
            Mode::Project { entry, .. } => entry,
        }
    }
}

/// The mode for a command given `file`, or none. A file argument always
/// means file mode, and no manifest is read. Otherwise the nearest
/// `nova.toml` at or above the current directory makes a project; with
/// none, `src/main.nova` stays the default.
pub fn mode(file: Option<PathBuf>) -> Result<Mode> {
    if let Some(file) = file {
        return Ok(Mode::File(file));
    }
    let cwd = std::env::current_dir().context("reading the current directory")?;
    let Some(root) = nova_pm::find_root(&cwd) else {
        return Ok(Mode::File(PathBuf::from("src/main.nova")));
    };
    // At the root, paths stay relative, exactly as before projects existed.
    let base = if root == cwd {
        PathBuf::new()
    } else {
        root.clone()
    };
    let manifest = read_manifest(&root, &base)?;
    let entry = if base.as_os_str().is_empty() {
        PathBuf::from("src/main.nova")
    } else {
        root.join("src").join("main.nova")
    };
    if !root.join("src").join("main.nova").is_file() {
        bail!(
            "project `{}` has no {}",
            manifest.package.name,
            entry.display()
        );
    }
    Ok(Mode::Project {
        entry,
        target_dir: base.join("target"),
        name: manifest.package.name,
    })
}

/// Read and check `<root>/nova.toml`, rendering its diagnostics. Any error,
/// a declared dependency (M0005) included, stops the command.
fn read_manifest(root: &Path, base: &Path) -> Result<Manifest> {
    let path = root.join(nova_pm::MANIFEST);
    let source =
        std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    let mut db = FileDb::new();
    let file = db.add(
        base.join(nova_pm::MANIFEST).display().to_string(),
        source.as_str(),
    );
    let (manifest, mut diagnostics) = nova_pm::parse(&source, file);
    if let Some(manifest) = &manifest {
        diagnostics.extend(unresolved_dependencies(manifest));
    }
    if !diagnostics.is_empty() {
        render::emit_all(&db, &diagnostics);
    }
    let errors = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.severity == Severity::Error)
        .count();
    match manifest {
        Some(manifest) if errors == 0 => Ok(manifest),
        _ => bail!(
            "could not read nova.toml due to {errors} previous error{}",
            if errors == 1 { "" } else { "s" }
        ),
    }
}

/// M0005, one per declared dependency: nothing resolves them before 3.3,
/// and an `import` of one would otherwise fail later, and less clearly
/// (spec §5.2). This is the one place 3.3 removes.
fn unresolved_dependencies(manifest: &Manifest) -> Vec<Diagnostic> {
    manifest
        .dependencies
        .iter()
        .chain(&manifest.dev_dependencies)
        .map(|dependency| {
            Diagnostic::error(
                "M0005",
                format!("dependency `{}` cannot be used yet", dependency.name),
            )
            .with_primary_label(dependency.span, "declared here")
            .with_note("this nova does not resolve dependencies; remove the entry for now")
        })
        .collect()
}
