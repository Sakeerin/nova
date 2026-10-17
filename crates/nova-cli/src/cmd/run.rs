//! `nova run [FILE]` — compile and execute a Nova program,
//! `nova build [FILE]` — compile to a standalone executable, and
//! `nova check [FILE]` — type-check without running.
//!
//! With no FILE, each works on the project around the current directory, or
//! on `src/main.nova` outside any project (`crate::project`).

use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Args;
use nova_driver::{Outcome, Program, Roots};

use crate::bundle::Bundle;
use crate::project::{self, Mode};

#[derive(Args)]
pub struct RunCmd {
    /// The Nova source file to run (default: the project's src/main.nova,
    /// found through the nearest nova.toml; or src/main.nova).
    file: Option<PathBuf>,

    /// Arguments for the program, after `--`: `nova run [FILE] -- ARGS...`.
    /// The program's `args()` is FILE followed by these.
    #[arg(last = true)]
    args: Vec<std::ffi::OsString>,
}

#[derive(Args)]
pub struct CheckCmd {
    /// The Nova source file to check (default: as for `nova run`).
    file: Option<PathBuf>,

    /// What to check for: `native`, or `wasm`, with WASM's std and rules.
    #[arg(long, value_enum, default_value_t = TargetArg::Native)]
    target: TargetArg,
}

/// What `--target` names (spec
/// `docs/superpowers/specs/2026-10-14-phase-4-0-foundations-design.md`
/// §3.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum TargetArg {
    /// A native executable.
    Native,
    /// A WASM module and its JS shim.
    Wasm,
}

impl From<TargetArg> for nova_driver::Target {
    fn from(target: TargetArg) -> Self {
        match target {
            TargetArg::Native => nova_driver::Target::Native,
            TargetArg::Wasm => nova_driver::Target::Wasm,
        }
    }
}

#[derive(Args)]
pub struct BuildCmd {
    /// The Nova source file to build (default: as for `nova run`).
    file: Option<PathBuf>,

    /// Output executable path (default: target/debug/<name> in a project,
    /// or target/release/<name> with --release; otherwise `<file stem>` in
    /// the current directory; each with the platform executable suffix).
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Optimizing build via the LLVM backend (emits LLVM IR and compiles it
    /// with a discovered `clang`/`llc`); the default is the fast Cranelift
    /// backend. With `--target wasm`, runs `wasm-opt -Oz` on app.wasm when
    /// `wasm-opt` is on PATH.
    #[arg(long)]
    release: bool,

    /// What to build for: `native`, an executable; or `wasm`, a page in a
    /// directory (default: dist/ at the project's root, or in the current
    /// directory), which `-o` names.
    #[arg(long, value_enum, default_value_t = TargetArg::Native)]
    target: TargetArg,
}

#[derive(Args)]
pub struct BundleCmd {
    /// The Nova source file to build (default: as for `nova run`).
    file: Option<PathBuf>,

    /// The directory to write (default: dist/ at the project's root, or in
    /// the current directory).
    #[arg(short, long)]
    output: Option<PathBuf>,
}

/// The program's entry, which `nova run` passes as its first argument and
/// `nova check` names.
fn entry(program: &Program) -> PathBuf {
    program
        .roots
        .first()
        .map(|root| root.path.clone())
        .unwrap_or_default()
}

pub fn run(cmd: RunCmd) -> Result<()> {
    let mode = project::mode(cmd.file)?;
    project::refuse_library(&mode)?;
    project::sync(&mode)?;
    let program = project::program_to_run(&mode)?;
    let mut args = vec![entry(&program).to_string_lossy().into_owned()];
    args.extend(cmd.args.iter().map(|a| a.to_string_lossy().into_owned()));
    match nova_driver::run_program(program, args)? {
        Outcome::Ok(()) => Ok(()),
        Outcome::Failed { errors } => anyhow::bail!(
            "could not compile due to {errors} previous error{}",
            if errors == 1 { "" } else { "s" }
        ),
    }
}

/// `nova bundle`: `nova build --target wasm` (spec
/// `docs/superpowers/specs/2026-10-14-phase-4-0-foundations-design.md`
/// §3.1).
pub fn bundle(cmd: BundleCmd) -> Result<()> {
    build(BuildCmd {
        file: cmd.file,
        output: cmd.output,
        release: false,
        target: TargetArg::Wasm,
    })
}

/// `nova build --target wasm`: `dist/` at the project's root, or in the
/// current directory, unless `-o` names it (spec 4.0 §7), with what the
/// 4.4 spec's §7 adds: the project's page, `assets/`, `manifest.json`, and
/// `wasm-opt` under `--release`.
fn build_wasm(cmd: BuildCmd) -> Result<()> {
    let mode = project::mode(cmd.file)?;
    project::refuse_library(&mode)?;
    project::sync(&mode)?;
    let program = project::program_to_run(&mode)?;
    let output = match (cmd.output, &mode) {
        (Some(output), _) => output,
        (None, Mode::Project { root, .. }) => root.join("dist"),
        (None, Mode::File(_)) => PathBuf::from("dist"),
    };
    let bundle = Bundle::of(&mode, &program, cmd.release);
    bundle.check_output(&output)?;
    bundle.prepare(&output)?;
    match nova_driver::build_program_wasm(program, &output)? {
        Outcome::Ok(dir) => {
            bundle.finish(&dir)?;
            println!("built {}", dir.display());
            Ok(())
        }
        Outcome::Failed { errors } => anyhow::bail!(
            "could not compile due to {errors} previous error{}",
            if errors == 1 { "" } else { "s" }
        ),
    }
}

pub fn build(cmd: BuildCmd) -> Result<()> {
    if cmd.target == TargetArg::Wasm {
        return build_wasm(cmd);
    }
    let mode = project::mode(cmd.file)?;
    project::refuse_library(&mode)?;
    project::sync(&mode)?;
    let program = project::program_to_run(&mode)?;
    let output = match (cmd.output, &mode) {
        (Some(output), _) => output,
        (None, Mode::Project { target_dir, .. }) => {
            let dir = target_dir.join(if cmd.release { "release" } else { "debug" });
            if !program.has_errors() {
                std::fs::create_dir_all(&dir)
                    .with_context(|| format!("creating {}", dir.display()))?;
            }
            dir.join(format!(
                "{}{}",
                project::package_name(&program),
                std::env::consts::EXE_SUFFIX
            ))
        }
        (None, Mode::File(file)) => {
            let stem = file
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_else(|| "out".to_string());
            PathBuf::from(format!("{stem}{}", std::env::consts::EXE_SUFFIX))
        }
    };
    let built = if cmd.release {
        nova_driver::build_program_release(program, &output)?
    } else {
        nova_driver::build_program(program, &output)?
    };
    match built {
        Outcome::Ok(path) => {
            println!("built {}", path.display());
            Ok(())
        }
        Outcome::Failed { errors } => anyhow::bail!(
            "could not compile due to {errors} previous error{}",
            if errors == 1 { "" } else { "s" }
        ),
    }
}

pub fn check(cmd: CheckCmd) -> Result<()> {
    let mode = project::mode(cmd.file)?;
    project::sync(&mode)?;
    let program = mode.program(Roots::Check);
    let entry = entry(&program);
    match nova_driver::check_program_for(program, cmd.target.into())? {
        Outcome::Ok(()) => {
            println!("ok: {}", entry.display());
            Ok(())
        }
        Outcome::Failed { errors } => {
            anyhow::bail!("found {errors} error{}", if errors == 1 { "" } else { "s" })
        }
    }
}
