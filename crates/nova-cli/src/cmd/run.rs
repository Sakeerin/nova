//! `nova run <file>` — compile and execute a Nova program,
//! `nova build <file>` — compile to a standalone executable, and
//! `nova check <file>` — type-check without running.

use std::path::PathBuf;

use anyhow::Result;
use clap::Args;
use nova_driver::Outcome;

#[derive(Args)]
pub struct RunCmd {
    /// Path to the Nova source file to run (default: src/main.nova).
    file: Option<PathBuf>,
}

#[derive(Args)]
pub struct CheckCmd {
    /// Path to the Nova source file to check (default: src/main.nova).
    file: Option<PathBuf>,
}

#[derive(Args)]
pub struct BuildCmd {
    /// Path to the Nova source file to build (default: src/main.nova).
    file: Option<PathBuf>,

    /// Output executable path (default: `<file stem>` in the current
    /// directory, with the platform executable suffix).
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Optimizing build via the LLVM backend (emits LLVM IR and compiles it
    /// with a discovered `clang`/`llc`); the default is the fast Cranelift
    /// backend.
    #[arg(long)]
    release: bool,
}

fn default_file(file: Option<PathBuf>) -> PathBuf {
    file.unwrap_or_else(|| PathBuf::from("src/main.nova"))
}

pub fn run(cmd: RunCmd) -> Result<()> {
    let file = default_file(cmd.file);
    match nova_driver::run_file(&file)? {
        Outcome::Ok(()) => Ok(()),
        Outcome::Failed { errors } => anyhow::bail!(
            "could not compile due to {errors} previous error{}",
            if errors == 1 { "" } else { "s" }
        ),
    }
}

pub fn build(cmd: BuildCmd) -> Result<()> {
    let file = default_file(cmd.file);
    let output = cmd.output.unwrap_or_else(|| {
        let stem = file
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "out".to_string());
        PathBuf::from(format!("{stem}{}", std::env::consts::EXE_SUFFIX))
    });
    let built = if cmd.release {
        nova_driver::build_file_release(&file, &output)?
    } else {
        nova_driver::build_file(&file, &output)?
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
    let file = default_file(cmd.file);
    match nova_driver::check_file(&file)? {
        Outcome::Ok(()) => {
            println!("ok: {}", file.display());
            Ok(())
        }
        Outcome::Failed { errors } => {
            anyhow::bail!("found {errors} error{}", if errors == 1 { "" } else { "s" })
        }
    }
}
