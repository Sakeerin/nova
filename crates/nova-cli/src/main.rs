//! The `nova` command-line tool.
//!
//! Dispatches to subcommands: parse, run, build, dev, check, test, fmt, doc,
//! lsp, new, init, fetch, update, package, publish, login and version. Phase 0
//! implemented `nova
//! parse`, Phase 1 `nova run` (Cranelift JIT) and `nova check`, Phase 3.0
//! the project commands (spec
//! `docs/superpowers/specs/2026-10-07-phase-3-0-foundations-design.md`),
//! Phase 3.1 `nova fmt` (spec
//! `docs/superpowers/specs/2026-10-07-phase-3-1-formatter-design.md`),
//! Phase 3.2 `nova lsp` (spec
//! `docs/superpowers/specs/2026-10-08-phase-3-2-lsp-core-design.md`),
//! Phase 3.3b the package index (spec
//! `docs/superpowers/specs/2026-10-09-phase-3-3b-index-and-publishing-design.md`),
//! Phase 3.5a `nova doc` (spec
//! `docs/superpowers/specs/2026-10-11-phase-3-5a-the-documentation-site-design.md`),
//! and Phase 4.5 `nova dev` (spec
//! `docs/superpowers/specs/2026-10-18-phase-4-5-router-and-nova-dev-design.md`).

mod bundle;
mod cmd;
mod dev;
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
    /// Compile a Nova program to a standalone executable, or with
    /// `--target wasm` to a page in dist/.
    Build(cmd::run::BuildCmd),
    /// Compile a Nova program to a page in dist/: `build --target wasm`.
    Bundle(cmd::run::BundleCmd),
    /// Build a page into dist/ and serve it on 127.0.0.1, building it again
    /// and reloading the page when a source changes.
    Dev(cmd::dev::DevCmd),
    /// Type-check a Nova program without running it.
    Check(cmd::run::CheckCmd),
    /// Compile and run `@test` functions, one process per test.
    Test(cmd::test::TestCmd),
    /// Format Nova source files: the project's `src/`, or the files and
    /// directories given.
    Fmt(cmd::fmt::FmtCmd),
    /// Write the project's documentation site to target/doc/, with its
    /// dependencies' and std's.
    Doc(cmd::doc::DocCmd),
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

/// What a command's thread hands the main thread: the command's result,
/// or, for `nova run`, the program it compiled, which runs on the main
/// thread.
enum Ran {
    Done(Result<()>),
    Program(Box<cmd::run::Compiled>),
}

/// What a command line asks for: a command [`Cli`] parses, or the hidden
/// `nova check --dump-resolution`, `nova check --dump-hir` or `nova check
/// --dump-hir-items`, which have a parser of their own, so that `check`'s
/// is as it was
/// ([`cmd::run::DumpCmd`]).
enum Job {
    Cli(Command),
    Dump(cmd::run::DumpCmd),
}

fn main() -> Result<()> {
    let job = match cmd::run::DumpCmd::from_args(std::env::args_os()) {
        Some(cmd) => Job::Dump(cmd),
        None => Job::Cli(Cli::parse().command),
    };
    let filter = tracing_subscriber::EnvFilter::from_default_env()
        .add_directive(tracing::Level::WARN.into());
    // Under `nova lsp`, stdout carries only protocol messages, so logs (and
    // the `log` records tracing forwards, lsp-server's included) go to
    // stderr (spec docs/superpowers/specs/2026-10-08-phase-3-2-lsp-core-design.md
    // §6.1). Every other command keeps its subscriber as it was.
    if matches!(job, Job::Cli(Command::Lsp)) {
        tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_writer(std::io::stderr)
            .init();
    } else {
        tracing_subscriber::fmt().with_env_filter(filter).init();
    }

    nova_driver::set_embedded_runtime(embedded::runtime());
    // The command runs on a thread with the pipeline's stack, so that a
    // deeply nested source meets the parser's limit, not the platform's
    // stack (spec
    // `docs/superpowers/specs/2026-10-30-phase-5-1-the-lexer-the-parser-and-the-ast-design.md`
    // §4.4, ADR 0052). The thread is named `main`, so a panic's message
    // reads as it did; the panic then resumes here, unwinding out of `main`
    // as it did, with the exit code 101, and is not printed again. A
    // stack overflow aborts the process from either thread, and
    // `std::process::exit` ends it from either.
    let thread = std::thread::Builder::new()
        .name("main".to_string())
        .stack_size(nova_driver::PIPELINE_STACK)
        .spawn(move || match job {
            Job::Cli(command) => dispatch(command),
            Job::Dump(cmd) => Ran::Done(cmd::run::dump(cmd)),
        });
    let ran = match thread {
        Ok(thread) => thread.join(),
        Err(e) => anyhow::bail!("could not start the compiler's thread: {e}"),
    };
    match ran {
        Ok(Ran::Done(result)) => result,
        Ok(Ran::Program(program)) => {
            program.run();
            Ok(())
        }
        Err(panic) => std::panic::resume_unwind(panic),
    }
}

/// Run `command`, on the thread `main` starts for it.
fn dispatch(command: Command) -> Ran {
    let result = match command {
        Command::Parse(cmd) => cmd::parse::run(cmd),
        Command::Run(cmd) => {
            return match cmd::run::compile(cmd) {
                Ok(program) => Ran::Program(Box::new(program)),
                Err(e) => Ran::Done(Err(e)),
            }
        }
        Command::Build(cmd) => cmd::run::build(cmd),
        Command::Bundle(cmd) => cmd::run::bundle(cmd),
        Command::Dev(cmd) => cmd::dev::run(cmd),
        Command::Check(cmd) => cmd::run::check(cmd),
        Command::Test(cmd) => cmd::test::run(cmd),
        // Its exit code says more than success or failure (spec §7.2).
        Command::Fmt(cmd) => std::process::exit(cmd::fmt::run(cmd)),
        Command::Doc(cmd) => cmd::doc::run(cmd),
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
    };
    Ran::Done(result)
}
