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
    /// backend.
    #[arg(long)]
    release: bool,
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

pub fn build(cmd: BuildCmd) -> Result<()> {
    let mode = project::mode(cmd.file)?;
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
    let program = mode.program(Roots::Check);
    let entry = entry(&program);
    match nova_driver::check_program(program)? {
        Outcome::Ok(()) => {
            println!("ok: {}", entry.display());
            Ok(())
        }
        Outcome::Failed { errors } => {
            anyhow::bail!("found {errors} error{}", if errors == 1 { "" } else { "s" })
        }
    }
}
