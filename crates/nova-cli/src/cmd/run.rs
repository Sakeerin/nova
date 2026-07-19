//! `nova run <file>` — compile and execute a Nova program, and
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
