//! The `nova` command-line tool.
//!
//! Dispatches to subcommands: parse, run, build, check, test, fmt, lsp,
//! new, init, fetch, update, package, publish, login and version. Phase 0
//! implemented `nova
//! parse`, Phase 1 `nova run` (Cranelift JIT) and `nova check`, Phase 3.0
//! the project commands (spec
//! `docs/superpowers/specs/2026-10-07-phase-3-0-foundations-design.md`),
//! Phase 3.1 `nova fmt` (spec
//! `docs/superpowers/specs/2026-10-07-phase-3-1-formatter-design.md`),
//! Phase 3.2 `nova lsp` (spec
//! `docs/superpowers/specs/2026-10-08-phase-3-2-lsp-core-design.md`), and
//! Phase 3.3b the package index (spec
//! `docs/superpowers/specs/2026-10-09-phase-3-3b-index-and-publishing-design.md`).

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
    /// Run the language server over stdin and stdout (editors start it).
    Lsp,
    /// Create a new project in a new directory.
    New(cmd::new::NewCmd),
    /// Make the current directory a project.
    Init(cmd::new::InitCmd),
    /// Add a dependency to nova.toml.
    Add(cmd::deps::AddCmd),
    /// Remove a dependency from nova.toml.
    Remove(cmd::deps::RemoveCmd),
    /// Download what nova.lock names and the cache lacks, resolving first
    /// if the lock does not answer nova.toml.
    Fetch,
    /// Move locked versions to the newest that fit: every package, or one.
    Update(cmd::fetch::UpdateCmd),
    /// Store a GitHub token for `nova publish`, read from standard input:
    /// `gh auth token | nova login`.
    Login,
    /// Pack the library into target/package/, and verify the tarball.
    Package,
    /// Pack and verify the library, then publish it to the package
    /// index.
    Publish,
    /// Show the version, the target, and whether the runtime library is
    /// embedded.
    Version,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let filter = tracing_subscriber::EnvFilter::from_default_env()
        .add_directive(tracing::Level::WARN.into());
    // Under `nova lsp`, stdout carries only protocol messages, so logs (and
    // the `log` records tracing forwards, lsp-server's included) go to
    // stderr (spec docs/superpowers/specs/2026-10-08-phase-3-2-lsp-core-design.md
    // §6.1). Every other command keeps its subscriber as it was.
    if matches!(cli.command, Command::Lsp) {
        tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_writer(std::io::stderr)
            .init();
    } else {
        tracing_subscriber::fmt().with_env_filter(filter).init();
    }

    nova_driver::set_embedded_runtime(embedded::runtime());
    match cli.command {
        Command::Parse(cmd) => cmd::parse::run(cmd),
        Command::Run(cmd) => cmd::run::run(cmd),
        Command::Build(cmd) => cmd::run::build(cmd),
        Command::Check(cmd) => cmd::run::check(cmd),
        Command::Test(cmd) => cmd::test::run(cmd),
        // Its exit code says more than success or failure (spec §7.2).
        Command::Fmt(cmd) => std::process::exit(cmd::fmt::run(cmd)),
        // Its exit code is the protocol's: 0 after `shutdown` then `exit`.
        Command::Lsp => std::process::exit(cmd::lsp::run()),
        Command::New(cmd) => cmd::new::new(cmd),
        Command::Init(cmd) => cmd::new::init(cmd),
        Command::Add(cmd) => cmd::deps::add(cmd),
        Command::Remove(cmd) => cmd::deps::remove(cmd),
        Command::Fetch => cmd::fetch::fetch(),
        Command::Update(cmd) => cmd::fetch::update(cmd),
        Command::Login => cmd::login::login(),
        Command::Package => cmd::package::package(),
        Command::Publish => cmd::package::publish(),
        Command::Version => cmd::version::run(),
    }
}
