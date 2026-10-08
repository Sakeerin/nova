//! Which program a command works on, and where `build` writes (spec
//! `docs/superpowers/specs/2026-10-07-phase-3-0-foundations-design.md` §6,
//! and for packages
//! `docs/superpowers/specs/2026-10-08-phase-3-3a-local-packages-design.md`
//! §5.1).

use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use nova_driver::{Program, Roots};

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
