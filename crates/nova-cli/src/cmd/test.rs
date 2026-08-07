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

use std::path::PathBuf;
use std::process::{Command, Output};

use anyhow::{Context, Result};
use clap::Args;

#[derive(Args)]
pub struct TestCmd {
    /// Only run tests whose name contains this substring.
    filter: Option<String>,
}

/// The three ways a test's process can end, kept distinct end to end rather
/// than collapsed to a bool. Distinguishing the last two from each other is
/// the entire justification for running each test in its own process
/// (design doc §5, §9 risk 4): a checked panic and a genuine miscompile both
/// exit nonzero, but only one of them is what `@test(should_panic)` means,
/// and conflating them would let a hard crash masquerade as an expected
/// panic.
enum Outcome {
    /// Exit 0.
    Passed,
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
    /// what makes that test pass. Carries the matched line verbatim, so the
    /// report shows exactly what the runtime said rather than a
    /// reconstruction of it.
    Panicked(String),
    /// Nonzero exit with no such line: an illegal instruction, a segfault,
    /// anything the runtime did not choose to do. Carries the raw exit code
    /// for the report, but classification never depends on its *value* —
    /// the mapping from `abort()` to an exit code is platform- and
    /// shell-dependent (measured 127 and 132 for two different aborts on
    /// this project, on Windows, through Git Bash; neither is portable).
    Trapped(i32),
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
    if output.status.success() {
        return Outcome::Passed;
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    match stderr.lines().find(|line| line.contains(PANIC_MARKER)) {
        Some(line) => Outcome::Panicked(line.trim().to_string()),
        None => Outcome::Trapped(output.status.code().unwrap_or(-1)),
    }
}

/// `nova test`'s only positional argument is the filter
/// (`nova-spec/40-TOOLING.md:20`: `nova test [filter]`, no `[file]`, unlike
/// `run`/`build`/`check`), so the entry file is always the project default.
fn test_entry_file() -> PathBuf {
    PathBuf::from("src/main.nova")
}

pub fn run(cmd: TestCmd) -> Result<()> {
    let file = test_entry_file();
    let (exe, tests) = nova_driver::build_test_binary(&file)?;

    // Enumerate by asking the compiled binary itself — exactly what a plain,
    // `NOVA_TEST_INDEX`-unset run of it prints to a human — rather than
    // trusting `tests` for the name/order pairing directly
    // (`nova_resolver::TestFn`'s doc comment: Task 5 reads names back out of
    // the binary so a future drift between what the compiler *built* and
    // what the binary *dispatches* would surface here instead of being
    // silently trusted away). One extra process, no second compilation.
    let inventory = Command::new(&exe)
        .env_remove("NOVA_TEST_INDEX")
        .output()
        .with_context(|| format!("failed to run {}", exe.display()))?;
    let stdout = String::from_utf8_lossy(&inventory.stdout).into_owned();
    let mut lines = stdout.lines();
    let count: usize = lines
        .next()
        .unwrap_or_default()
        .trim()
        .parse()
        .with_context(|| {
            format!(
                "{}'s inventory did not start with a test count: {stdout:?}",
                exe.display()
            )
        })?;
    let names: Vec<String> = lines.take(count).map(str::to_string).collect();
    anyhow::ensure!(
        names.len() == count,
        "{} reported {count} test{} but printed only {} name{}",
        exe.display(),
        if count == 1 { "" } else { "s" },
        names.len(),
        if names.len() == 1 { "" } else { "s" }
    );
    anyhow::ensure!(
        names.len() == tests.len(),
        "the compiled binary's inventory ({} tests) disagrees with the compiler's own count \
         ({}); this is a compiler bug",
        names.len(),
        tests.len()
    );

    // should_panic, by position: `names` (read from the binary, above) and
    // `tests` (returned by `build_test_binary`) are both derived from the
    // identical source-ordered list at compile time, so index `i` names the
    // same test in both — see the `ensure!` above, which would have already
    // failed if that invariant did not hold.
    let selected: Vec<usize> = (0..count)
        .filter(|&i| match &cmd.filter {
            Some(f) => names[i].contains(f.as_str()),
            None => true,
        })
        .collect();

    println!(
        "running {} test{}",
        selected.len(),
        if selected.len() == 1 { "" } else { "s" }
    );

    let mut passed = 0usize;
    let mut failed = 0usize;
    let mut trapped = 0usize;

    for &i in &selected {
        let name = &names[i];
        let result = Command::new(&exe)
            .env("NOVA_TEST_INDEX", i.to_string())
            .output()
            .with_context(|| format!("failed to run {} for test `{name}`", exe.display()))?;
        let should_panic = tests[i].should_panic;
        match (classify(&result), should_panic) {
            (Outcome::Passed, false) => {
                passed += 1;
                println!("test {name} ... ok");
            }
            // should_panic means the test is only correct if it panics, so a
            // clean exit is a failure — reported distinctly from both an
            // ordinary failed assertion and a trap, since neither happened.
            (Outcome::Passed, true) => {
                failed += 1;
                println!("test {name} ... FAILED (expected a panic, but the test passed)");
            }
            (Outcome::Panicked(_), true) => {
                passed += 1;
                println!("test {name} ... ok");
            }
            (Outcome::Panicked(line), false) => {
                failed += 1;
                println!("test {name} ... FAILED");
                println!("    {line}");
            }
            // A trap is a failure whether or not `should_panic` is set: it
            // means the process executed an illegal instruction or crashed,
            // not that it panicked. `should_panic` inverts only the
            // `Panicked` row above; reporting a trap as "FAILED" like an
            // ordinary assertion would erase that distinction for whoever
            // reads the output, which is the entire reason process isolation
            // is used here rather than an in-process runner that could not
            // tell the two apart.
            (Outcome::Trapped(code), _) => {
                trapped += 1;
                println!("test {name} ... TRAPPED (exit code {code})");
            }
        }
    }

    // Deliberately the length of `selected`, not `count` or `tests.len()`:
    // this line must enumerate what actually ran, not repeat a number that
    // was never measured when a filter narrowed the run.
    let total = selected.len();
    let all_ok = failed == 0 && trapped == 0;
    println!();
    println!(
        "test result: {}. {passed} passed; {failed} failed; {trapped} trapped; {total} total",
        if all_ok { "ok" } else { "FAILED" }
    );

    if all_ok {
        Ok(())
    } else {
        anyhow::bail!(
            "{failed} test{} failed, {trapped} trapped",
            if failed == 1 { "" } else { "s" }
        );
    }
}
