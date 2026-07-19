//! The `nova` command-line tool.
//!
//! Dispatches to subcommands: parse, build, run, fmt, lsp, test, doc, bundle.
//! Phase 0 implements `nova parse`; Phase 1 adds `nova run` (Cranelift JIT)
//! and `nova check`.

mod cmd;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "nova",
    about = "The Nova programming language toolchain",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Parse a Nova source file and print the AST (for debugging).
    Parse(cmd::parse::ParseCmd),
    /// Compile and run a Nova program.
    Run(cmd::run::RunCmd),
    /// Type-check a Nova program without running it.
    Check(cmd::run::CheckCmd),
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::WARN.into()),
        )
        .init();

    let cli = Cli::parse();
    match cli.command {
        Command::Parse(cmd) => cmd::parse::run(cmd),
        Command::Run(cmd) => cmd::run::run(cmd),
        Command::Check(cmd) => cmd::run::check(cmd),
    }
}
