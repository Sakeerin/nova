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
        names.len() == tests.len(),
        "the compiled binary's inventory ({} tests) disagrees with the compiler's own count \
         ({}); this is a compiler bug (inventory process {status})",
        names.len(),
        tests.len()
    );
    // Whole-branch review, finding 5 (minor): the two `ensure!`s above only
    // ever compared *lengths* — `names.len() == count` and
    // `names.len() == tests.len()` — never that the two lists actually agree
    // *position by position*. The `should_panic`-by-index comment just below
    // used to claim this was already covered ("see the `ensure!` above,
    // which would have already failed"), which was not true: two
    // same-length lists in different orders would satisfy both length checks
    // and then silently misassociate `should_panic` with the wrong test.
    // This is the check that makes that comment's claim actually hold.
    anyhow::ensure!(
        (0..count).all(|i| names[i] == tests[i].name),
        "the compiled binary's inventory names disagree with the compiler's own test names at \
         the same positions; this is a compiler bug (inventory process {status})"
    );

    // should_panic, by position: `names` (read from the binary, above) and
    // `tests` (returned by `build_test_binary`) are both derived from the
    // identical source-ordered list at compile time, so index `i` names the
    // same test in both — see the `ensure!`s above, which would have already
    // failed if that invariant did not hold.
    let selected: Vec<usize> = (0..count)
        .filter(|&i| match &cmd.filter {
            Some(f) => names[i].contains(f.as_str()),
            None => true,
        })
        .collect();

    // Whole-branch review, finding 4: an *explicit* filter that matches
    // nothing is a failure, not a quiet no-op. Before this, `nova test
    // <typo>` printed `running 0 tests` / `test result: ok` and exited 0 —
    // indistinguishable from an unfiltered run of a project with no tests at
    // all, so a typo'd CI filter reported green having run nothing (ADR 0008
    // §2's own residual-gaps list named this). An *unfiltered* run of a file
    // with no tests is deliberately left alone: `count == 0` with
    // `cmd.filter` absent still exits 0, since nothing was mistyped there.
    if let Some(f) = &cmd.filter {
        anyhow::ensure!(
            !selected.is_empty(),
            "no test name contains the filter `{f}` ({count} test{} available)",
            if count == 1 { "" } else { "s" }
        );
    }

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

#[cfg(test)]
mod tests {
    use super::*;

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
