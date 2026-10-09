//! Which program a command works on, and where `build` writes (spec
//! `docs/superpowers/specs/2026-10-07-phase-3-0-foundations-design.md` §6,
//! and for packages
//! `docs/superpowers/specs/2026-10-08-phase-3-3a-local-packages-design.md`
//! §5.1).

use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail, Context, Result};
use nova_diagnostics::{render, FileDb, Severity};
use nova_driver::{Program, Roots};
use nova_index::{Index, SyncError, SyncRequest, Synced};
use nova_pm::Unlock;

/// What a command works on.
pub enum Mode {
    /// One file: a file argument, or `src/main.nova` outside any project.
    File(PathBuf),
    /// A project, found by walking up to its `nova.toml`.
    Project {
        /// Its directory: empty when it is the current directory, so that
        /// paths stay relative there as they always have, and absolute from
        /// anywhere else.
        root: PathBuf,
        /// `target`, relative or absolute in the same way.
        target_dir: PathBuf,
    },
}

impl Mode {
    /// The program the command compiles: a file's (spec 3.3a §4.1), or the
    /// project's with the roots `roots` names. The driver renders the
    /// graph's diagnostics, so a manifest error stops the command there.
    pub fn program(&self, roots: Roots) -> Program {
        match self {
            Mode::File(file) => Program::for_file(file),
            Mode::Project { root, .. } => Program::for_package(root, roots),
        }
    }
}

/// The mode for a command given `file`, or none. A file argument always
/// means file mode; it reads its package's manifest only when it is directly
/// in a package's `src/` or `tests/` (spec 3.3a §4.1). Otherwise the
/// nearest `nova.toml` at or above the current directory makes a project;
/// with none, `src/main.nova` stays the default.
pub fn mode(file: Option<PathBuf>) -> Result<Mode> {
    if let Some(file) = file {
        return Ok(Mode::File(file));
    }
    let cwd = std::env::current_dir().context("reading the current directory")?;
    let Some(root) = nova_pm::find_root(&cwd) else {
        return Ok(Mode::File(PathBuf::from("src/main.nova")));
    };
    // At the root, paths stay relative, exactly as before projects existed.
    let root = if root == cwd { PathBuf::new() } else { root };
    Ok(Mode::Project {
        target_dir: root.join("target"),
        root,
    })
}

/// The program `nova run` and `nova build` compile. A project that is only
/// a library has none, and is refused (spec 3.3a §5.1).
pub fn program_to_run(mode: &Mode) -> Result<Program> {
    let program = mode.program(Roots::Program);
    if let (Mode::Project { .. }, Some(graph)) = (mode, &program.graph) {
        let root = graph.root();
        if !program.has_errors() && root.has_lib && !root.has_main {
            bail!(
                "`{}` is a library: it has no src/main.nova; `nova check` and `nova test` \
                 work on it",
                root.name
            );
        }
    }
    Ok(program)
}

/// The project's package name, which `nova build` names its output after.
pub fn package_name(program: &Program) -> String {
    program
        .graph
        .as_ref()
        .map_or_else(|| "out".to_string(), |graph| graph.root().name.clone())
}

/// Refuse to work inside nova's cache of downloaded packages, which is
/// never edited in place (spec 3.3b §5.1, §5.4).
pub fn refuse_cached(root: &Path) -> Result<()> {
    let Some(registry) = nova_pm::registry_dir() else {
        return Ok(());
    };
    let dir = nova_pm::real_path(if root.as_os_str().is_empty() {
        Path::new(".")
    } else {
        root
    });
    if dir.starts_with(nova_pm::real_path(&registry)) {
        bail!(
            "`{}` is a downloaded package in nova's cache; it is read only",
            dir.display()
        );
    }
    Ok(())
}

/// The package a command syncs (spec 3.3b §5.3): the project, or the
/// package a file argument is directly in. `None` for a loose file.
pub fn package_root(mode: &Mode) -> Option<PathBuf> {
    match mode {
        Mode::Project { root, .. } => Some(root.clone()),
        Mode::File(file) => nova_driver::package_of(file).map(|(root, _)| root),
    }
}

/// `nova run` and `nova build` refuse a project that is only a library
/// before anything is synced (spec 3.3a §5.1, 3.3b §5.3). A manifest with
/// errors is left for the graph to report.
pub fn refuse_library(mode: &Mode) -> Result<()> {
    let Mode::Project { root, .. } = mode else {
        return Ok(());
    };
    let src = root.join("src");
    if !src.join("lib.nova").is_file() || src.join("main.nova").is_file() {
        return Ok(());
    }
    let text = std::fs::read_to_string(root.join(nova_pm::MANIFEST)).unwrap_or_default();
    let (Some(manifest), _) = nova_pm::parse(&text, nova_diagnostics::FileId::DUMMY) else {
        return Ok(());
    };
    bail!(
        "`{}` is a library: it has no src/main.nova; `nova check` and `nova test` work on it",
        manifest.package.name
    )
}

/// Sync the package `mode` works on before a command compiles it, as
/// `nova fetch` does (spec 3.3b §5.3).
pub fn sync(mode: &Mode) -> Result<()> {
    sync_with(mode, Unlock::Nothing).map(|_| ())
}

/// The sync, with `unlock`. `Ok(None)` for a loose file, which has nothing
/// to sync.
pub fn sync_with(mode: &Mode, unlock: Unlock) -> Result<Option<Synced>> {
    let Some(root) = package_root(mode) else {
        return Ok(None);
    };
    refuse_cached(&root)?;
    let index = Index::from_env().map_err(|e| anyhow!(e))?;
    let mut reader = nova_index::reader_for(&index);
    let registry = nova_pm::registry_dir();
    let mut db = FileDb::new();
    let result = nova_index::sync(
        SyncRequest {
            root: &root,
            manifest: None,
            dev: true,
            unlock,
            write: true,
            index: &index,
            reader: reader.as_mut(),
            registry: registry.as_deref(),
        },
        &mut db,
    );
    finish(result, &db).map(Some)
}

/// A sync's outcome: its notes and downloads printed on standard error,
/// its diagnostics rendered.
pub fn finish(result: Result<Synced, SyncError>, db: &FileDb) -> Result<Synced> {
    match result {
        Ok(synced) => {
            for note in &synced.notes {
                eprintln!("note: {note}");
            }
            for package in &synced.unpacked {
                eprintln!("downloaded {package}");
            }
            Ok(synced)
        }
        Err(SyncError::Diagnostics(diagnostics)) => {
            render::emit_all(db, &diagnostics);
            let errors = diagnostics
                .iter()
                .filter(|d| d.severity == Severity::Error)
                .count();
            bail!(
                "could not resolve the dependencies due to {errors} previous error{}",
                if errors == 1 { "" } else { "s" }
            )
        }
        Err(SyncError::Other(why)) => Err(anyhow!(why)),
    }
}
