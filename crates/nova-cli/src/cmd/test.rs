//! `nova test [filter]` — compile the program once to a standalone test
//! binary (`nova_driver::build_test_binary`, Task 3) and run each collected
//! `@test` function in its own process, one `NOVA_TEST_INDEX` per test
//! (`nova-spec/40-TOOLING.md:20`).
//!
//! Process isolation is the entire reason this runs a subprocess per test
//! rather than calling each test function in-process: a Nova panic calls
//! `std::process::abort()` with no unwinding anywhere in the runtime
//! (`std/test/lib.nova`), so an in-process runner could not survive one
//! failing test to report on the next, and — more importantly — it could
//! never tell a checked panic apart from a hard trap, because after an abort
//! there is no process left to inspect. Running one process per test and
//! classifying it from the outside, by exit status and stderr, is what makes
//! both possible (design doc `2026-08-05-nova-test-design.md` §5).
//!
//! Phase 3.5b adds the doc tests, the code blocks of a library's docs (spec
//! `docs/superpowers/specs/2026-10-11-phase-3-5b-doc-tests-design.md` §6):
//! each is a module of a second binary, built after the unit tests' from
//! memory, whose tests run the same way and report by the doc tests' names.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use anyhow::{anyhow, bail, Context, Result};
use clap::Args;
use nova_diagnostics::{render, Diagnostic, FileDb, FileId, Severity, Span};
use nova_doc::{DocTest, Failure};
use nova_driver::{DiskSources, Program, Sources};

use crate::project::{self, Mode};

#[derive(Args)]
pub struct TestCmd {
    /// Only run tests whose name contains this substring.
    filter: Option<String>,

    /// Run only the doc tests: the code examples in the library's docs.
    #[arg(long)]
    doc: bool,

    /// With --doc: run std's doc tests instead, inside a project or not.
    #[arg(long, requires = "doc")]
    std: bool,
}

/// The three ways a test's process can end, kept distinct end to end rather
/// than collapsed to a bool. Distinguishing the last two from each other is
/// the entire justification for running each test in its own process
/// (design doc §5, §9 risk 4): a checked panic and a genuine miscompile both
/// exit nonzero, but only one of them is what `@test(should_panic)` means,
/// and conflating them would let a hard crash masquerade as an expected
/// panic.
///
/// Every non-`Passed` variant carries the process's full captured `stdout`
/// (and, for `Trapped`, `stderr` too — `Panicked` already carries the
/// relevant part of stderr as `line`) so [`print_captured`] can show it.
/// Whole-branch review, finding 3: before this, `Trapped` kept only the exit
/// code — a trapping test reported `TRAPPED (exit code N)` and nothing else,
/// no matter what its own `println` calls or the runtime had written to
/// either stream — and a *failing* test's own `println` output was silently
/// dropped too, since only the one matched panic line survived. Given this
/// branch's central claim is that traps deserve their own outcome,
/// surfacing them with *less* information than a failure was backwards.
/// Cheap to carry: `Command::output()` already buffered both streams in
/// full before `classify` ever runs.
enum Outcome {
    /// Exit 0. Carries `stdout` only so a should_panic test that passed
    /// unexpectedly — `(Outcome::Passed, true)`, reported as `FAILED` below
    /// — can still show what it printed; an ordinary pass never reads it.
    Passed { stdout: String },
    /// Nonzero exit, and stderr said why: a line containing
    /// [`PANIC_MARKER`]. Three call sites in the runtime independently
    /// `eprintln!` a line with that exact substring, rather than one shared
    /// function being the only emitter: `nova_rt_panic_str`
    /// (`nova-runtime/src/lib.rs`, user `panic(...)` and every `std/test`
    /// assertion), `nova_rt_check_bounds` (same file, array index out of
    /// bounds), and `gc::alloc`'s oversized-allocation guard
    /// (`nova-runtime/src/gc.rs`). [`classify`]'s plain substring search
    /// catches all three uniformly, which is not incidental:
    /// `should_panic_passes_on_a_checked_panic` reaches this marker through
    /// the bounds-check path and never calls `nova_rt_panic_str` at all, so
    /// the substring search — not a check against one specific emitter — is
    /// what makes that test pass. `line` carries the matched line verbatim,
    /// so the report shows exactly what the runtime said rather than a
    /// reconstruction of it; `stdout` carries anything the test itself
    /// printed before panicking, previously discarded entirely.
    Panicked { line: String, stdout: String },
    /// Nonzero exit with no such line: an illegal instruction, a segfault,
    /// anything the runtime did not choose to do. Carries the raw exit code
    /// for the report, but classification never depends on its *value* —
    /// the mapping from a hard trap to an exit code is platform- and
    /// shell-dependent (`1 / 0` measured as `132` via Git Bash and
    /// `-1073741795` / `0xC000001D` via Rust's own process API for the
    /// identical program — see ADR 0008 §2 — and neither figure is portable).
    /// This is a genuine trap, not an `abort()`: a checked panic reaches
    /// `nova_rt_panic_str`'s `std::process::abort()` and is `Panicked`
    /// above; a trap like `1 / 0` is an illegal-instruction fault the CPU
    /// raises directly and never reaches `abort()` at all, which is exactly
    /// why it needs its own stderr (it has none of `Panicked`'s marker) as
    /// well as its own stdout.
    Trapped {
        code: i32,
        stdout: String,
        stderr: String,
    },
}

/// The one stable, portable signal a Nova panic (or panic-shaped runtime
/// abort — see [`Outcome::Panicked`] for its three independent emitters)
/// leaves behind. Its presence on stderr — not the process's exit code — is
/// what [`classify`] keys on.
const PANIC_MARKER: &str = "nova: panic:";

/// Classify a finished test process. The exit code decides clean vs.
/// unclean; stderr decides *how* it was unclean. Never the other way around:
/// an aborted process's exit code is not portable (see [`Outcome::Trapped`]),
/// so it must never be asked "was this a panic?", only "was this clean?".
fn classify(output: &Output) -> Outcome {
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    if output.status.success() {
        return Outcome::Passed { stdout };
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    match stderr.lines().find(|line| line.contains(PANIC_MARKER)) {
        Some(line) => {
            let line = line.trim().to_string();
            Outcome::Panicked { line, stdout }
        }
        None => Outcome::Trapped {
            code: output.status.code().unwrap_or(-1),
            stdout,
            stderr: stderr.into_owned(),
        },
    }
}

/// Print a non-passing test's captured output, the way `cargo test` prints
/// captured output for a failure — but only ever for a stream that actually
/// has something in it. An ordinary `assert_eq` failure or a `1 / 0` trap
/// writes nothing to stdout and (for the trap) nothing to stderr either, so
/// this prints nothing extra for either and every existing test's exact
/// rendered output is unaffected; it only ever *adds* lines, for a test that
/// actually produced output beyond what the summary line already shows.
fn print_captured(label: &str, output: &str) {
    if output.is_empty() {
        return;
    }
    println!("    ---- {label} ----");
    for line in output.lines() {
        println!("    {line}");
    }
}

/// Render a child process's exit code in both decimal and unsigned hex.
///
/// Load-bearing for one specific open question (ADR 0008): an intermittent,
/// never-reproduced failure in which a freshly linked test binary produces
/// completely empty output. The evidence that separates its candidate causes is
/// the raw code — on Windows a fault arrives as an NTSTATUS reinterpreted as a
/// signed `i32`, so `0xC0000005` (`STATUS_ACCESS_VIOLATION`, the process never
/// reached its entry point) shows up as `-1073741819`, and `0xC000001D`
/// (`STATUS_ILLEGAL_INSTRUCTION`, the deterministic fault a trapping test
/// raises) as `-1073741795`. Decimal alone is unrecognizable.
///
/// The `as u32` cast is measured to be a no-op at this type, not a fix for
/// sign extension: Rust's `UpperHex` for a *signed* integer already formats
/// its two's-complement bit pattern at the type's own width, so
/// `format!("{:X}", code)` and `format!("{:X}", code as u32)` are
/// byte-identical for every `i32` value (verified directly against rustc
/// 1.95.0, debug and `-O`). Sign-extending to something like
/// `0xFFFFFFFFC0000005` would require `code` to first be widened to a wider
/// signed type (e.g. `i64`), which nothing here does. The cast stays as
/// documentation of intent and as a guard should `code` ever become that
/// wider type; see `mod tests` below for why no test can pin its presence.
fn format_exit_code(code: i32) -> String {
    format!("code {code} (0x{:08X})", code as u32)
}

pub fn run(cmd: TestCmd) -> Result<()> {
    // `nova test`'s only positional argument is the filter
    // (`nova-spec/40-TOOLING.md:20`: `nova test [filter]`, no `[file]`,
    // unlike `run`/`build`/`check`), so it always works on the project
    // around the current directory, or on `src/main.nova` outside any.
    let filter = cmd.filter.as_deref();
    if cmd.std {
        return run_std_doc_tests(filter);
    }
    let mode = project::mode(None)?;
    if cmd.doc {
        return run_doc_tests(&mode, filter);
    }
    project::sync(&mode)?;
    let program = mode.program(nova_driver::Roots::Test);
    let (exe, tests) = nova_driver::build_test_program(program)?;
    let compiled: Vec<String> = tests.iter().map(|t| t.name.clone()).collect();
    let names = inventory(&exe, &compiled)?;
    let should_panic: Vec<bool> = tests.iter().map(|t| t.should_panic).collect();

    // Spec 3.5b §6.1: the doc tests follow when the root package has a
    // library whose docs hold at least one. Otherwise nothing changes.
    let library = match &mode {
        Mode::Project { root, .. } if has_library(root) => Some(root.as_path()),
        _ => None,
    };
    let docs = match library {
        Some(root) => library_doc_tests(root)?,
        None => Vec::new(),
    };
    let doc_names: Vec<String> = docs.iter().map(|d| d.name.clone()).collect();

    // A filter matches across both kinds (spec 3.5b §6.1).
    let selected = select(&names, filter);
    let doc_selected = select(&doc_names, filter);
    let mut available = vec![(names.len(), "test")];
    if !docs.is_empty() {
        available.push((docs.len(), "doc test"));
    }
    ensure_matched(filter, selected.len() + doc_selected.len(), &available)?;

    let tally = run_tests(&exe, "test", &names, &should_panic, &selected)?;
    let mut sections = vec![("test", tally)];
    // Both sections run when the unit tests compile, even if some failed
    // (spec 3.5b §6.3), so one run reports everything.
    if let (Some(root), false) = (library, docs.is_empty()) {
        let tally = run_doc_section(Some(root), &docs, &doc_selected, true)?;
        sections.push(("doc test", tally));
    }
    finish(&sections)
}

/// `nova test --doc [filter]` (spec 3.5b §6.1): the doc tests alone, of
/// the library of the package around the current directory.
fn run_doc_tests(mode: &Mode, filter: Option<&str>) -> Result<()> {
    let Mode::Project { root, .. } = mode else {
        bail!(
            "nova test --doc tests a package's library; run it inside one, or use \
             `nova test --doc --std`"
        );
    };
    if !has_library(root) {
        bail!("nova test --doc tests a library's docs, and this package has no src/lib.nova");
    }
    project::sync(mode)?;
    doc_tests_alone(Some(root), &library_doc_tests(root)?, filter)
}

/// `nova test --doc --std [filter]` (spec 3.5b §6.1): std's doc tests,
/// inside a project or not.
fn run_std_doc_tests(filter: Option<&str>) -> Result<()> {
    let docs = nova_doc::std_doc_tests().map_err(failure)?;
    doc_tests_alone(None, &docs, filter)
}

/// The doc section alone, of `docs`: the doc tests of the package at
/// `package`, or std's.
fn doc_tests_alone(package: Option<&Path>, docs: &[DocTest], filter: Option<&str>) -> Result<()> {
    let names: Vec<String> = docs.iter().map(|d| d.name.clone()).collect();
    let selected = select(&names, filter);
    ensure_matched(filter, selected.len(), &[(docs.len(), "doc test")])?;
    let tally = run_doc_section(package, docs, &selected, false)?;
    finish(&[("doc test", tally)])
}

/// Whether the package at `root` has a library (spec 3.5b §4.1).
fn has_library(root: &Path) -> bool {
    root.join("src").join("lib.nova").is_file()
}

/// The doc tests of the library of the package at `root`, in the site's
/// order. `root` is spelled as the doc tests' program spells it, so a
/// doc test's file is named alike in both (`nova_doc::DocTest::file`).
fn library_doc_tests(root: &Path) -> Result<Vec<DocTest>> {
    Ok(nova_doc::package_doc_tests(root)
        .map_err(failure)?
        .unwrap_or_default())
}

/// A failure to read the doc tests, as the command's error, its
/// diagnostics printed first.
fn failure(f: Failure) -> anyhow::Error {
    match f {
        Failure::Diagnostics { db, diagnostics } => {
            render::emit_all(&db, &diagnostics);
            let errors = diagnostics
                .iter()
                .filter(|d| d.severity == Severity::Error)
                .count();
            anyhow!(
                "could not compile due to {errors} previous error{}",
                if errors == 1 { "" } else { "s" }
            )
        }
        Failure::Other(why) => anyhow!(why),
    }
}

/// The directory, under a package's root, that its doc tests' modules are
/// named in: no file is there (spec 3.5b §11.2), and `<` and `>` keep one
/// from being made on Windows. Each module is `doc<N>.nova`, and the
/// first names the binary.
const DOC_TESTS: &str = "<doc tests>";

/// The directory, under the current directory, that std's doc tests'
/// modules are named in, as [`DOC_TESTS`] is for a package's.
const STD_DOC_TESTS: &str = "<std doc tests>";

/// The doc section (spec 3.5b §6.2, §6.3): the `selected` of `docs`, the
/// doc tests of the package at `package`, or std's, compiled into a binary
/// of their own and each run in its own process. `after_units` says the
/// unit section ran: a blank line comes first, and the build shows only
/// what is in doc tests' code, since the unit build showed the rest.
fn run_doc_section(
    package: Option<&Path>,
    docs: &[DocTest],
    selected: &[usize],
    after_units: bool,
) -> Result<Tally> {
    for &i in selected {
        warn_unknown_words(&docs[i]);
    }
    // With none selected there is nothing to build: the driver is never
    // given no doc test (spec 3.5b §6.1).
    let (exe, should_panic) = if selected.is_empty() {
        (PathBuf::new(), Vec::new())
    } else {
        build_doc_tests(package, docs, after_units)?
    };
    if after_units {
        println!();
    }
    let names: Vec<String> = docs.iter().map(|d| d.name.clone()).collect();
    run_tests(&exe, "doc test", &names, &should_panic, selected)
}

/// The binary of `docs`, the doc tests of the package at `package`, or
/// std's, and whether each of its tests should panic. Its modules are
/// served from memory, and its diagnostics point at the docs (spec 3.5b
/// §7).
fn build_doc_tests(
    package: Option<&Path>,
    docs: &[DocTest],
    after_units: bool,
) -> Result<(PathBuf, Vec<bool>)> {
    let dir = match package {
        Some(root) => root.join(DOC_TESTS),
        None => PathBuf::from(STD_DOC_TESTS),
    };
    let modules: Vec<(PathBuf, String)> = docs
        .iter()
        .enumerate()
        .map(|(i, d)| (dir.join(format!("doc{i}.nova")), d.text.clone()))
        .collect();
    let program = Program::doc_tests(package, modules.iter().map(|(p, _)| p.clone()).collect());
    // A module's name in the program's `FileDb`, and its doc test.
    let by_name: HashMap<String, usize> = modules
        .iter()
        .enumerate()
        .map(|(i, (path, _))| (path.display().to_string(), i))
        .collect();
    // No warning twice (spec 3.5b §6.3): after the unit section, only what
    // is in doc tests' code.
    let map =
        |db: &mut FileDb, d: &mut Diagnostic| to_the_docs(db, d, &by_name, docs) || !after_units;
    let (exe, tests) =
        nova_driver::build_test_program_with(program, &Overlay(&modules), Some(&map))?;
    // Each doc test module declares one test, and the driver collects only
    // that one, in the modules' order: test `i` is doc test `i`.
    anyhow::ensure!(
        tests.len() == docs.len(),
        "the doc tests' binary holds {} tests for {} doc tests; this is a compiler bug",
        tests.len(),
        docs.len()
    );
    let compiled: Vec<String> = tests.iter().map(|t| t.name.clone()).collect();
    inventory(&exe, &compiled)?;
    Ok((exe, tests.iter().map(|t| t.should_panic).collect()))
}

/// The doc tests' modules from memory, and every other file from the disk.
struct Overlay<'a>(&'a [(PathBuf, String)]);

impl Sources for Overlay<'_> {
    fn read(&self, path: &Path) -> std::io::Result<String> {
        match self.0.iter().find(|(p, _)| p == path) {
            Some((_, text)) => Ok(text.clone()),
            None => DiskSources.read(path),
        }
    }
}

/// Moves each label of `d` that is in a doc test's module, which `by_name`
/// finds by the module's name, to the doc comment its code comes from
/// (spec 3.5b §7), and says whether there was one. A moved diagnostic's
/// fixes are dropped: their edits name the module.
fn to_the_docs(
    db: &mut FileDb,
    d: &mut Diagnostic,
    by_name: &HashMap<String, usize>,
    docs: &[DocTest],
) -> bool {
    let mut in_docs = false;
    for label in &mut d.labels {
        let Some(i) = db
            .get_name(label.span.file)
            .and_then(|name| by_name.get(name))
            .copied()
        else {
            continue;
        };
        in_docs = true;
        let test = &docs[i];
        let Some(file) = doc_file(db, test) else {
            continue;
        };
        let text = db.get_source(file).unwrap_or_default();
        let start = on_a_char(text, test.map(label.span.start));
        let end = on_a_char(text, test.map(label.span.end)).max(start);
        label.span = Span::new(start, end, file);
    }
    if in_docs {
        d.fixes.clear();
    }
    in_docs
}

/// The file a doc test's block is in, in `db`. A package's is there, by
/// the name its analysis gave it, which the doc tests' program reads it
/// under. A std module's is named `std/<module>/lib.nova`, as the doc
/// test's name shows it (spec 3.5b §7), and is added from std's sources
/// when missing: the build adds std's modules, as `<std/time>` and so on,
/// only after loading.
fn doc_file(db: &mut FileDb, test: &DocTest) -> Option<FileId> {
    let Some(text) = nova_driver::std_source(&test.file) else {
        return db.id_of(&test.file);
    };
    let (name, _) = test.place.rsplit_once(':')?;
    Some(db.id_of(name).unwrap_or_else(|| db.add(name, text)))
}

/// `offset`, at most `text`'s length, and moved back to a character
/// boundary.
fn on_a_char(text: &str, offset: u32) -> u32 {
    let mut at = (offset as usize).min(text.len());
    while !text.is_char_boundary(at) {
        at -= 1;
    }
    at as u32
}

/// Spec 3.5b §4.2: an unknown word after `nova` warns, naming the block's
/// line, and the block runs as a plain test.
fn warn_unknown_words(test: &DocTest) {
    if test.unknown.is_empty() {
        return;
    }
    let words: Vec<String> = test.unknown.iter().map(|w| format!("`{w}`")).collect();
    eprintln!(
        "warning: {}: unknown word{} {} in a code block's info string; the block runs as a \
         plain test",
        test.place,
        if words.len() == 1 { "" } else { "s" },
        words.join(", ")
    );
}

/// The test names `exe` prints when `NOVA_TEST_INDEX` is unset, checked
/// against `compiled`, the compiler's own list, by count and by position.
fn inventory(exe: &Path, compiled: &[String]) -> Result<Vec<String>> {
    // Enumerate by asking the compiled binary itself — exactly what a plain,
    // `NOVA_TEST_INDEX`-unset run of it prints to a human — rather than
    // trusting `tests` for the name/order pairing directly
    // (`nova_resolver::TestFn`'s doc comment: Task 5 reads names back out of
    // the binary so a future drift between what the compiler *built* and
    // what the binary *dispatches* would surface here instead of being
    // silently trusted away). One extra process, no second compilation.
    let inventory = Command::new(exe)
        .env_remove("NOVA_TEST_INDEX")
        .output()
        .with_context(|| format!("failed to run {}", exe.display()))?;
    // Captured eagerly, not just on the error paths below: every one of the
    // four checks between here and the loop can fail on a malformed
    // inventory, and each failure message needs the same evidence — the
    // exit status and stderr the child actually produced — which is exactly
    // what was missing the last four times a freshly linked test binary
    // produced empty output for no diagnosed reason (ADR 0008).
    let status = inventory.status;
    let stdout = String::from_utf8_lossy(&inventory.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&inventory.stderr).into_owned();
    let mut lines = stdout.lines();
    let count: usize = lines
        .next()
        .unwrap_or_default()
        .trim()
        .parse()
        .with_context(|| {
            format!(
                "{}'s inventory did not start with a test count ({status}): stdout {stdout:?}, stderr {stderr:?}",
                exe.display()
            )
        })?;
    let names: Vec<String> = lines.take(count).map(str::to_string).collect();
    anyhow::ensure!(
        names.len() == count,
        "{} reported {count} test{} but printed only {} name{} ({status}; stderr {stderr:?})",
        exe.display(),
        if count == 1 { "" } else { "s" },
        names.len(),
        if names.len() == 1 { "" } else { "s" }
    );
    anyhow::ensure!(
        names.len() == compiled.len(),
        "the compiled binary's inventory ({} tests) disagrees with the compiler's own count \
         ({}); this is a compiler bug (inventory process {status})",
        names.len(),
        compiled.len()
    );
    // Whole-branch review, finding 5 (minor): the two `ensure!`s above only
    // ever compared *lengths* — `names.len() == count` and
    // `names.len() == tests.len()` — never that the two lists actually agree
    // *position by position*. The `should_panic`-by-index comment in
    // [`run_tests`] used to claim this was already covered ("see the
    // `ensure!` above, which would have already failed"), which was not
    // true: two same-length lists in different orders would satisfy both
    // length checks and then silently misassociate `should_panic` with the
    // wrong test. This is the check that makes that comment's claim
    // actually hold.
    anyhow::ensure!(
        (0..count).all(|i| names[i] == compiled[i]),
        "the compiled binary's inventory names disagree with the compiler's own test names at \
         the same positions; this is a compiler bug (inventory process {status})"
    );
    Ok(names)
}

/// The indexes of `names` that contain `filter`: all of them without one.
fn select(names: &[String], filter: Option<&str>) -> Vec<usize> {
    (0..names.len())
        .filter(|&i| match filter {
            Some(f) => names[i].contains(f),
            None => true,
        })
        .collect()
}

/// Whole-branch review, finding 4: an *explicit* filter that matches
/// nothing is a failure, not a quiet no-op. Before this, `nova test
/// <typo>` printed `running 0 tests` / `test result: ok` and exited 0 —
/// indistinguishable from an unfiltered run of a project with no tests at
/// all, so a typo'd CI filter reported green having run nothing (ADR 0008
/// §2's own residual-gaps list named this). An *unfiltered* run of a file
/// with no tests is deliberately left alone: `count == 0` with no filter
/// still exits 0, since nothing was mistyped there. `matched` counts what
/// the filter selected across every section, and `available` how many
/// tests of each kind there are (spec 3.5b §6.1).
fn ensure_matched(filter: Option<&str>, matched: usize, available: &[(usize, &str)]) -> Result<()> {
    let Some(f) = filter else {
        return Ok(());
    };
    if matched > 0 {
        return Ok(());
    }
    let available: Vec<String> = available
        .iter()
        .map(|&(n, kind)| format!("{n} {kind}{}", if n == 1 { "" } else { "s" }))
        .collect();
    bail!(
        "no test name contains the filter `{f}` ({} available)",
        available.join(" and ")
    )
}

/// A section's failures and traps, which the closing error counts.
struct Tally {
    failed: usize,
    trapped: usize,
}

/// Run the `selected` tests of `exe`, each in its own process, and print
/// their section: `running N {kind}s`, a line per test by its entry in
/// `names`, a blank line, and the result line (spec 3.5b §6.3).
fn run_tests(
    exe: &Path,
    kind: &str,
    names: &[String],
    should_panic: &[bool],
    selected: &[usize],
) -> Result<Tally> {
    println!(
        "running {} {kind}{}",
        selected.len(),
        if selected.len() == 1 { "" } else { "s" }
    );

    let mut passed = 0usize;
    let mut failed = 0usize;
    let mut trapped = 0usize;

    for &i in selected {
        let name = &names[i];
        let result = Command::new(exe)
            .env("NOVA_TEST_INDEX", i.to_string())
            .output()
            .with_context(|| format!("failed to run {} for test `{name}`", exe.display()))?;
        // should_panic, by position: the binary's inventory and the
        // compiler's list are both derived from the identical
        // source-ordered list at compile time, so index `i` names the same
        // test in both — see [`inventory`]'s `ensure!`s, which would have
        // already failed if that invariant did not hold.
        match (classify(&result), should_panic[i]) {
            (Outcome::Passed { .. }, false) => {
                passed += 1;
                println!("test {name} ... ok");
            }
            // should_panic means the test is only correct if it panics, so a
            // clean exit is a failure — reported distinctly from both an
            // ordinary failed assertion and a trap, since neither happened.
            // This is the one `Passed` case that is not an overall pass, so
            // (finding 3) its captured stdout is shown like any other
            // failure's, rather than silently discarded.
            (Outcome::Passed { stdout }, true) => {
                failed += 1;
                println!("test {name} ... FAILED (expected a panic, but the test passed)");
                print_captured("stdout", &stdout);
            }
            (Outcome::Panicked { .. }, true) => {
                passed += 1;
                println!("test {name} ... ok");
            }
            (Outcome::Panicked { line, stdout }, false) => {
                failed += 1;
                println!("test {name} ... FAILED");
                println!("    {line}");
                print_captured("stdout", &stdout);
            }
            // A trap is a failure whether or not `should_panic` is set: it
            // means the process executed an illegal instruction or crashed,
            // not that it panicked. `should_panic` inverts only the
            // `Panicked` row above; reporting a trap as "FAILED" like an
            // ordinary assertion would erase that distinction for whoever
            // reads the output, which is the entire reason process isolation
            // is used here rather than an in-process runner that could not
            // tell the two apart. Finding 3: prints whatever the process
            // captured on either stream, so a trap is never reported with
            // less information than a failure — and so an anomaly like the
            // intermittent 0xC0000005 seen elsewhere on this branch would
            // self-document here if it recurs, instead of producing a bare
            // number.
            (
                Outcome::Trapped {
                    code,
                    stdout,
                    stderr,
                },
                _,
            ) => {
                trapped += 1;
                println!("test {name} ... TRAPPED (exit {})", format_exit_code(code));
                print_captured("stdout", &stdout);
                print_captured("stderr", &stderr);
            }
        }
    }

    // Deliberately the length of `selected`, not the inventory's count:
    // this line must enumerate what actually ran, not repeat a number that
    // was never measured when a filter narrowed the run.
    let total = selected.len();
    let all_ok = failed == 0 && trapped == 0;
    println!();
    println!(
        "test result: {}. {passed} passed; {failed} failed; {trapped} trapped; {total} total",
        if all_ok { "ok" } else { "FAILED" }
    );
    Ok(Tally { failed, trapped })
}

/// The run's end (spec 3.5b §6.3): it fails when a test of any section
/// failed or trapped, and the error counts each section's.
fn finish(sections: &[(&str, Tally)]) -> Result<()> {
    if sections
        .iter()
        .all(|(_, t)| t.failed == 0 && t.trapped == 0)
    {
        return Ok(());
    }
    let counts: Vec<String> = sections
        .iter()
        .map(|(kind, t)| {
            format!(
                "{} {kind}{} failed, {} trapped",
                t.failed,
                if t.failed == 1 { "" } else { "s" },
                t.trapped
            )
        })
        .collect();
    bail!("{}", counts.join("; "))
}

#[cfg(test)]
mod tests {
    use super::*;
    use nova_doc::Origin;

    #[test]
    fn a_std_doc_tests_diagnostic_points_at_its_modules_file() {
        // Spec 3.5b §7: std's diagnostics name `std/<module>/lib.nova`, as
        // the doc test's name does, before the build adds `<std/time>`.
        let time = nova_driver::std_source("<std/time>").expect("std/time");
        let third_line = time.match_indices('\n').nth(1).expect("three lines").0 + 1;
        let test = DocTest {
            name: "std::time (std/time/lib.nova:2)".to_string(),
            place: "std/time/lib.nova:2".to_string(),
            text: "@test fn nova_doctest() {\nlet d = nope\n}\n".to_string(),
            file: "<std/time>".to_string(),
            start: 0,
            lines: vec![
                Origin::Added,
                Origin::Code {
                    at: third_line as u32,
                    cut: None,
                },
                Origin::Added,
            ],
            unknown: Vec::new(),
        };
        let docs = [test];
        let mut db = FileDb::new();
        let module = db.add("<std doc tests>/doc0.nova", docs[0].text.clone());
        let by_name = HashMap::from([("<std doc tests>/doc0.nova".to_string(), 0)]);
        let nope = docs[0].text.find("nope").unwrap() as u32;
        let mut files = Vec::new();
        for _ in 0..2 {
            let mut d = Diagnostic::error("E0001", "cannot find `nope` in this scope")
                .with_primary_label(Span::new(nope, nope + 4, module), "here")
                .with_fix(nova_diagnostics::Fix::new("a fix", Vec::new()));
            assert!(to_the_docs(&mut db, &mut d, &by_name, &docs));
            let span = d.labels[0].span;
            assert_eq!(db.get_name(span.file), Some("std/time/lib.nova"));
            assert_eq!(db.get_source(span.file), Some(time));
            assert_eq!(db.location(span.file, span.start), Some((3, 9)));
            assert_eq!(span.end - span.start, 4);
            assert!(d.fixes.is_empty());
            files.push(span.file);
        }
        assert_eq!(
            files[0], files[1],
            "the second found the file the first added"
        );
        // A label in no doc test's module stays, and says so.
        let lib = db.add("src/lib.nova", "pub fn f() {}\n");
        let mut d = Diagnostic::warning("W0000", "a warning")
            .with_primary_label(Span::new(0, 3, lib), "here");
        assert!(!to_the_docs(&mut db, &mut d, &by_name, &docs));
        assert_eq!(d.labels[0].span, Span::new(0, 3, lib));
    }

    #[test]
    fn an_exit_code_is_rendered_in_decimal_and_as_an_unsigned_hex_ntstatus() {
        // `.code()` hands back a Windows NTSTATUS reinterpreted as a signed
        // i32, so 0xC0000005 arrives as -1073741819. Printing only decimal
        // loses the recognizable form; dropping the `(0x{:08X})` suffix
        // entirely is the mistake this test pins.
        //
        // The `as u32` cast in `format_exit_code` is NOT pinned by this
        // test, and cannot be by any test: measured directly (rustc 1.95.0,
        // debug and `-O`) that `format!("{:X}", code)` and
        // `format!("{:X}", code as u32)` are byte-identical for every `i32`
        // value, because Rust's `UpperHex` for a signed integer already
        // formats the two's-complement bit pattern at the type's own width
        // rather than sign-extending to a wider one — there is no
        // `0xFFFFFFFFC0000005` to observe here. The cast is kept as
        // documentation and as a guard should `code`'s type ever widen, not
        // because dropping it changes this function's output today.
        //
        // STATUS_ACCESS_VIOLATION -- the anomaly's signature: the process
        // never reached its entry point.
        assert_eq!(
            format_exit_code(-1073741819),
            "code -1073741819 (0xC0000005)"
        );
        // STATUS_ILLEGAL_INSTRUCTION -- the deterministic trap fault this must
        // be told apart from. If these two rendered the same, ADR 0008's whole
        // argument would be unverifiable.
        assert_ne!(format_exit_code(-1073741819), format_exit_code(-1073741795));
        assert_eq!(
            format_exit_code(-1073741795),
            "code -1073741795 (0xC000001D)"
        );
        // An ordinary small failure code must stay readable rather than being
        // rendered as a giant unsigned value.
        assert_eq!(format_exit_code(1), "code 1 (0x00000001)");
    }
}
