//! The `nova` command-line tool.
//!
//! Dispatches to subcommands: parse, run, build, check, test, fmt, new,
//! init and version. Phase 0 implemented `nova parse`, Phase 1 `nova run`
//! (Cranelift JIT) and `nova check`, Phase 3.0 the project commands (spec
//! `docs/superpowers/specs/2026-10-07-phase-3-0-foundations-design.md`),
//! and Phase 3.1 `nova fmt` (spec
//! `docs/superpowers/specs/2026-10-07-phase-3-1-formatter-design.md`).

mod cmd;
mod embedded;
mod project;
mod template;

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
    /// Compile a Nova program to a standalone executable.
    Build(cmd::run::BuildCmd),
    /// Type-check a Nova program without running it.
    Check(cmd::run::CheckCmd),
    /// Compile and run `@test` functions, one process per test.
    Test(cmd::test::TestCmd),
    /// Format Nova source files: the project's `src/`, or the files and
    /// directories given.
    Fmt(cmd::fmt::FmtCmd),
    /// Create a new project in a new directory.
    New(cmd::new::NewCmd),
    /// Make the current directory a project.
    Init(cmd::new::InitCmd),
    /// Show the version, the target, and whether the runtime library is
    /// embedded.
    Version,
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::WARN.into()),
        )
        .init();

    nova_driver::set_embedded_runtime(embedded::runtime());
    let cli = Cli::parse();
    match cli.command {
        Command::Parse(cmd) => cmd::parse::run(cmd),
        Command::Run(cmd) => cmd::run::run(cmd),
        Command::Build(cmd) => cmd::run::build(cmd),
        Command::Check(cmd) => cmd::run::check(cmd),
        Command::Test(cmd) => cmd::test::run(cmd),
        // Its exit code says more than success or failure (spec §7.2).
        Command::Fmt(cmd) => std::process::exit(cmd::fmt::run(cmd)),
        Command::New(cmd) => cmd::new::new(cmd),
        Command::Init(cmd) => cmd::new::init(cmd),
        Command::Version => cmd::version::run(),
    }
}
