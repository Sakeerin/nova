//! The `nova` command-line tool.
//!
//! Dispatches to subcommands: parse, build, run, fmt, lsp, test, doc, bundle.
//! Phase 0 implements `nova parse <file>` for parser testing.

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
    }
}
