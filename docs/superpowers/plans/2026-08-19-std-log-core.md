# `std/log` core Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Give Nova a working logger — five levels, level filtering, ISO-8601 UTC timestamps, stderr or stdout — plus the wall clock it needs.

**Architecture:** The runtime contributes a clock reading and a three-field configuration cell, nothing more. Every formatting, padding, calendar and filtering decision is Nova code in `std/time` and `std/log`, so all of it is reachable from a Nova fixture. Writing reuses the existing `println`/`eprintln` builtins, so the logger stays synchronous and introduces no second sync-write path.

**Tech Stack:** Rust (`nova-runtime`, `nova-mir`, `nova-resolver`, `nova-typeck`), Nova (`std/time`, `std/log`), `cargo test` + `assert_cmd` fixtures.

**Spec:** `docs/superpowers/specs/2026-08-19-std-log-core-design.md`

## Global Constraints

- `cargo build --locked --workspace` **before** `cargo test --locked --workspace --all-features --no-fail-fast`. Baseline: **1009 passed / 0 failed / 8 ignored across 44 targets.**
- Sum **every** `test result:` line; never pipe cargo output through `head`/`tail` first. **Filter out lines containing `trapped`** — a *failing* gate fixture's captured stdout is echoed by cargo and contains `test result: FAILED. 0 passed; 1 failed; 0 trapped; 1 total`, which is Nova's own harness schema and inflates a naive target count from 44 to 46.
- Clippy `--all-targets --all-features -- -D warnings` clean, on **both** ubuntu and windows. MSRV is **1.78**: **no `reason = "…"` in any lint attribute.**
- `cargo fmt --all -- --check` clean.
- The 8 ignored ADR-0010 GC tests stay ignored and untouched. There are **17 `#[ignore]` attributes**; the count must not change.
- `crates/**/*.rs` are **CRLF** in the worktree, as is markdown under `docs/` and `nova-spec/`. `cargo fmt` has flattened them to LF **four times across three increments** — after every fmt run check `tr -cd '\r' < f | wc -c` against `wc -l < f`, and repair by raw byte copy. **Never** `git checkout --`.
- `core.autocrlf=true` with no `.gitattributes`, so `git show HEAD:<path> | sha256sum` legitimately differs from `sha256sum <path>`. Compare against committed state with **`git diff`**, never by hashing a blob.
- Every fixture path unique per process.
- Commit messages: write to a UTF-8 file and apply with `git commit -F`, **never a heredoc**. Every body ends with exactly `Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>`.
- **Cite no SHA** that is not already an ancestor of `main` (`64e2be6` and `e0464ec` are).
- **Do not push, merge or tag.** Linear history, zero merge commits.
- A known intermittent failure exists in `nova-cli::run_tests`: a different test each run, always green in isolation, at least once with empty captured stdout and exit `0xC0000005`. Pre-existing and unrelated. If it fires, report the test name and confirm the isolation pass; do not chase it and do not attribute it to your changes.

---

## File Structure

| File | Responsibility | Task |
|---|---|---|
| `std/time/lib.nova` | `SystemTime`, ISO-8601 rendering, `pad2`/`pad3`, civil-from-days | 1, 2 |
| `crates/nova-runtime/src/time.rs` | the wall-clock reading, separate from the monotonic one | 2 |
| `crates/nova-runtime/src/log.rs` | **new** — the configuration cell and its three accessors | 3 |
| `crates/nova-runtime/src/lib.rs` | `mod log;` and four `symbols()` entries | 2, 3 |
| `crates/nova-mir/src/lib.rs` | `rt_funcs!` variants, `symbol()`, `signature()` | 2, 3 |
| `crates/nova-mir/src/lower.rs` | `Builtin` → `Lowering::Runtime` | 2, 3 |
| `crates/nova-resolver/src/lib.rs` | `Builtin` variants, names, `STD_ONLY`, `STD_MODULES` | 2, 3, 4 |
| `crates/nova-typeck/src/check.rs` | three exhaustive tables per builtin | 2, 3 |
| `std/log/lib.nova` | **new** — the whole logger surface | 4 |
| `tests/runtime/*.nova` + `.stdout` | fixtures | 1, 2, 4 |
| `crates/nova-cli/tests/run_tests.rs` | fixture registrations — **not automatic** | 1, 2, 4 |

### The twelve seams a runtime-backed builtin passes through

Measured against `Builtin::TimeNowNanos` at `64e2be6`. **Eleven are compiler-forced** — exhaustive `match`es, plus two const array lengths that fail to compile if an element is added without bumping the count. Exactly **one is not**, and it is the one that matters:

| # | Site | Forced? |
|---|---|---|
| 1 | `nova-resolver/src/lib.rs` `enum Builtin` variant (with doc comment) | yes |
| 2 | `nova-resolver/src/lib.rs:641` `Builtin::name()` → the Nova-visible name | yes (match) |
| 3 | `nova-resolver/src/lib.rs:729` membership in `STD_ONLY` | yes (`[Builtin; 60]` length) |
| 4 | `nova-typeck/src/check.rs:3968` the interpolation-hint `""` arm | yes (match) |
| 5 | `nova-typeck/src/check.rs:7230` `builtin_signature` | yes (match) |
| 6 | `nova-typeck/src/check.rs:15312` the description table | yes (match) |
| 7 | `nova-mir/src/lower.rs:741` `Builtin` → `Lowering::Runtime(RtFunc::…)` | yes (match) |
| 8 | `nova-mir/src/lib.rs` `rt_funcs!` invocation — variant + doc | yes |
| 9 | `nova-mir/src/lib.rs:478` `RtFunc::symbol()` | yes (match) |
| 10 | `nova-mir/src/lib.rs:567` `RtFunc::signature()` | yes (match) |
| 11 | the `#[no_mangle] pub extern "C-unwind"` function itself | yes (symbol() names it) |
| 12 | **`crates/nova-runtime/src/lib.rs` `symbols()`** | **NO** |

`RtFunc::ALL` needs no edit: the `rt_funcs!` macro (`nova-mir/src/lib.rs:140`) generates the enum and `ALL` from one identifier list, and its own doc records why — a variant added to a second hand-maintained list was once forgotten and "silently emitted a call to an undeclared symbol."

**Seam 12 is the whole risk.** A missing `symbols()` entry compiles clean and makes `cranelift-jit` panic in `finalize_definitions` at run time. It is guarded by `every_rt_func_symbol_is_registered_with_the_jit` (`nova-codegen-cranelift/src/lib.rs:958`), whose failure message names the fix. Every task that adds a builtin must run that test by name.

---

## Task 1: `SystemTime` and ISO-8601, in Nova only

Pure Nova. No runtime change, no compiler seam, no intrinsic — `to_iso8601` is tested against literal nanosecond values, so the whole calendar computation lands and is verified before any plumbing exists.

**Files:**
- Modify: `std/time/lib.nova` (append after the existing `Duration` impl)
- Create: `tests/runtime/system_time_iso8601.nova`, `tests/runtime/system_time_iso8601.stdout`
- Modify: `crates/nova-cli/tests/run_tests.rs` (one registration)

**Interfaces:**
- Consumes: nothing.
- Produces: `pub record SystemTime { nanos: Int }`; `SystemTime::to_iso8601(self) -> String`; private `pad2(n: Int) -> String`, `pad3(n: Int) -> String`, `civil_from_days(days: Int) -> [Int]` returning `[year, month, day]`.

- [ ] **Step 1: Write the failing fixture**

Create `tests/runtime/system_time_iso8601.nova`. Every input is a literal, so nothing here reads a clock:

```nova
// ISO-8601 rendering, against fixed nanosecond values rather than the clock.
// Each row pins one way the formatter can silently break; see the plan's
// table for what each is for. The 2100 row fails any leap rule that only
// tests divisibility by 4, and the last row fails an unpadded formatter.
fn main() {
    println(SystemTime { nanos: 0 }.to_iso8601())
    println(SystemTime { nanos: 951_782_400_000_000_000 }.to_iso8601())
    println(SystemTime { nanos: 1_709_164_800_000_000_000 }.to_iso8601())
    println(SystemTime { nanos: 4_107_542_400_000_000_000 }.to_iso8601())
    println(SystemTime { nanos: 1_767_225_599_999_000_000 }.to_iso8601())
    println(SystemTime { nanos: 1_756_598_463_007_000_000 }.to_iso8601())
}
```

Create `tests/runtime/system_time_iso8601.stdout` with exactly these six lines:

```
1970-01-01T00:00:00.000Z
2000-02-29T00:00:00.000Z
2024-02-29T00:00:00.000Z
2100-03-01T00:00:00.000Z
2025-12-31T23:59:59.999Z
2025-08-31T00:01:03.007Z
```

These six values are **verified correct against an independent implementation** — do not recompute or adjust them. What each pins: the epoch itself; a leap day in a leap century; an ordinary leap day; that 2100 is **not** a leap year; a year boundary with every field at maximum; and single-digit minute and second with a sub-100 millisecond value.

- [ ] **Step 2: Register the fixture — it does not run otherwise**

In `crates/nova-cli/tests/run_tests.rs`, following the shape of the existing `time_elapsed_run`:

```rust
/// ISO-8601 rendering over fixed nanosecond inputs, so the assertion is on
/// known values rather than on a live clock. The six rows pin the epoch, two
/// leap days, the 2100 non-leap-year, a year boundary, and zero-padding.
#[test]
fn system_time_iso8601_run() {
    let expected =
        std::fs::read_to_string(repo_root().join("tests/runtime/system_time_iso8601.stdout"))
            .expect("expected-output fixture exists")
            .replace("\r\n", "\n");
    nova()
        .arg("run")
        .arg(repo_root().join("tests/runtime/system_time_iso8601.nova"))
        .assert()
        .success()
        .stdout(expected);
}
```

- [ ] **Step 3: Run it and watch it fail**

Run: `cargo test --locked -p nova-cli --test run_tests system_time_iso8601_run`
Expected: FAIL — `SystemTime` does not resolve.

- [ ] **Step 4: Add the padding helpers**

In `std/time/lib.nova`. **These are private top-level `fn`s with no `pub`**, so `import_std_module` never binds them into another module's scope and they take no name from user code:

```nova
// Two-digit zero pad. Private: a top-level `pub fn` is glob-imported into
// every module (see `std/strings`' `join` for the same reasoning).
fn pad2(n: Int) -> String {
    if n < 10 { "0${n}" } else { "${n}" }
}

// Three-digit zero pad, for the millisecond field.
fn pad3(n: Int) -> String {
    if n < 10 { "00${n}" } else if n < 100 { "0${n}" } else { "${n}" }
}
```

**Deviation from spec §5, recorded deliberately:** the spec says padding uses `String::repeat` over the digit count. Interpolation with a literal prefix is simpler, has no dependency on `std/strings`, and was probed working (`pad2(7)` → `07`, `pad2(42)` → `42`). The spec's actual requirement — two helpers named `pad2` and `pad3` — is met. If a reviewer prefers the `repeat` form, it is a two-line change.

- [ ] **Step 5: Add civil-from-days**

Hinnant's algorithm, transcribed from spec §5. Returns `[year, month, day]` because Nova rejects tuples (`E0900`):

```nova
// Days since 1970-01-01 to [year, month, day], by Howard Hinnant's
// civil_from_days. Shifting the era to start in March makes leap-day
// handling fall out of integer arithmetic with no conditionals and no
// table: February becomes the last month of the previous shifted year, so
// its variable length never lands mid-sequence.
fn civil_from_days(days: Int) -> [Int] {
    let z = days + 719_468
    let era = z / 146_097
    let doe = z - era * 146_097
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365
    let y = yoe + era * 400
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100)
    let mp = (5 * doy + 2) / 153
    let d = doy - (153 * mp + 2) / 5 + 1
    let m = if mp < 10 { mp + 3 } else { mp - 9 }
    let year = if m <= 2 { y + 1 } else { y }
    [year, m, d]
}
```

- [ ] **Step 6: Add the record and the renderer**

```nova
// A wall-clock reading: nanoseconds since the Unix epoch.
//
// Deliberately a separate type from `Instant`, not a method on it.
// `Instant`'s whole contract is that it is monotonic and comparable by
// subtraction within one process; a wall clock is neither, and can jump
// backwards when NTP corrects it. Two types that cannot be mistaken for
// one another is the point.
pub record SystemTime { nanos: Int }

impl SystemTime {
    // ISO-8601 UTC to milliseconds: `2026-08-19T02:40:13.123Z`. Fixed
    // width, so log lines align with no padding logic in the caller.
    //
    // UTC only, permanently: `00-MASTER-SPEC.md` §6's crate list is FINAL
    // and has no date/time crate, so there is no timezone database to
    // consult and a local-time offset would be a guess that is wrong twice
    // a year in every DST zone.
    pub fn to_iso8601(self) -> String {
        let nanos_per_day = 86_400_000_000_000
        let days = self.nanos / nanos_per_day
        let rem = self.nanos % nanos_per_day
        let ymd = civil_from_days(days)
        let hour = rem / 3_600_000_000_000
        let minute = rem / 60_000_000_000 % 60
        let second = rem / 1_000_000_000 % 60
        let milli = rem / 1_000_000 % 1_000
        "${ymd[0]}-${pad2(ymd[1])}-${pad2(ymd[2])}T${pad2(hour)}:${pad2(minute)}:${pad2(second)}.${pad3(milli)}Z"
    }
}
```

- [ ] **Step 7: Run the fixture and the whole suite**

Run: `cargo build --locked --workspace` then `cargo test --locked -p nova-cli --test run_tests system_time_iso8601_run`
Expected: PASS, all six lines matching.

Then the full suite: `cargo build --locked --workspace` followed by `cargo test --locked --workspace --all-features --no-fail-fast`, summing every `test result:` line except those containing `trapped`.
Expected: **1010 passed / 0 failed / 8 ignored across 44 targets** (+1).

- [ ] **Step 8: Mutation — prove the padding test can fail**

Change `pad2` to `fn pad2(n: Int) -> String { "${n}" }`. Run `system_time_iso8601_run`.
Expected: FAIL on the last row, `2025-8-31T0:1:3.007Z` against `2025-08-31T00:01:03.007Z`.

Then change `to_iso8601` to compute the year as `1970 + days / 365` and the month as `1`, leaving the rest. Run again.
Expected: FAIL on **every row except the epoch**.

Restore both. Verify the restore with `git diff` — **not** by hashing, since `core.autocrlf=true` makes a blob hash legitimately differ from the worktree file. Re-run the fixture and confirm PASS.

- [ ] **Step 9: Check line endings, then commit**

`tr -cd '\r' < std/time/lib.nova | wc -c` must equal `wc -l < std/time/lib.nova`; same for the two fixtures and `run_tests.rs`.

```bash
git add std/time/lib.nova tests/runtime/system_time_iso8601.nova tests/runtime/system_time_iso8601.stdout crates/nova-cli/tests/run_tests.rs
git commit -F <path-to-utf8-message-file>
```

Message subject: `feat(std): render a wall-clock instant as ISO-8601 UTC`. Body must state that the calendar math is in Nova so it is fixture-reachable, that the six goldens were verified against an independent implementation, and end with the `Co-Authored-By` trailer.

---

## Task 2: The wall-clock intrinsic through all twelve seams

**Files:**
- Modify: `crates/nova-runtime/src/time.rs`, `crates/nova-runtime/src/lib.rs`
- Modify: `crates/nova-mir/src/lib.rs`, `crates/nova-mir/src/lower.rs`
- Modify: `crates/nova-resolver/src/lib.rs`, `crates/nova-typeck/src/check.rs`
- Modify: `std/time/lib.nova` (add `SystemTime::now`)
- Create: `tests/runtime/system_time_now.nova`, `tests/runtime/system_time_now.stdout`
- Modify: `crates/nova-cli/tests/run_tests.rs`

**Interfaces:**
- Consumes: `SystemTime { nanos: Int }` and `to_iso8601` from Task 1.
- Produces: Nova builtin `time_now_epoch_nanos() -> Int`; `SystemTime::now() -> SystemTime`; `RtFunc::TimeNowEpochNanos`; runtime symbol `nova_rt_time_now_epoch_nanos`.

- [ ] **Step 1: Write the failing runtime unit test**

In `crates/nova-runtime/src/time.rs`'s test module:

```rust
/// The wall clock reads from the Unix epoch, not from this module's
/// process epoch. A reading taken now must sit after 2026-01-01 and
/// before 2100-01-01 — a window wide enough to never be flaky and narrow
/// enough to fail if the reading is actually a process-relative value,
/// which would be a handful of milliseconds.
#[test]
fn the_wall_clock_reads_from_the_unix_epoch_not_the_process_epoch() {
    let n = nova_rt_time_now_epoch_nanos();
    assert!(
        n > 1_767_225_600_000_000_000,
        "wall clock returned {n}, which is before 2026-01-01; a process-relative reading looks like this"
    );
    assert!(n < 4_102_444_800_000_000_000, "wall clock returned {n}, after 2100-01-01");
}
```

- [ ] **Step 2: Run it and watch it fail**

Run: `cargo test --locked -p nova-runtime the_wall_clock_reads_from_the_unix_epoch`
Expected: FAIL to compile — the function does not exist.

- [ ] **Step 3: Add the runtime function**

In `crates/nova-runtime/src/time.rs`:

```rust
/// Nanoseconds since the Unix epoch, saturating at `i64::MAX`.
///
/// **Deliberately does not reuse [`now_nanos`] or [`epoch`].** Those read
/// from a process-start `OnceLock` and answer "how long has this process
/// been running"; this answers "what time is it". Same unit, same width,
/// different quantity — which is exactly the shape of mistake that forced
/// the `SLEEP_SLOT_MS` → `SLEEP_SLOT_NANOS` → `SLEEP_SLOT_DEADLINE_NANOS`
/// renames, twice, because the type never changed while the meaning did.
///
/// A clock set before 1970 makes `duration_since` fail; that returns `0`
/// rather than a negative value, so the calendar math downstream never has
/// to interpret one.
#[no_mangle]
pub extern "C-unwind" fn nova_rt_time_now_epoch_nanos() -> i64 {
    match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
        Ok(d) => i64::try_from(d.as_nanos()).unwrap_or(i64::MAX),
        Err(_) => 0,
    }
}
```

- [ ] **Step 4: Run the unit test to green**

Run: `cargo test --locked -p nova-runtime the_wall_clock_reads_from_the_unix_epoch`
Expected: PASS.

- [ ] **Step 5: Wire seams 8, 9, 10 — `nova-mir`**

In the `rt_funcs!` invocation, after `TimeNowNanos`:

```rust
    /// `() -> i64` — nanoseconds since the **Unix** epoch. Distinct from
    /// `TimeNowNanos`, which is process-relative.
    TimeNowEpochNanos,
```

In `RtFunc::symbol()`: `RtFunc::TimeNowEpochNanos => "nova_rt_time_now_epoch_nanos",`
In `RtFunc::signature()`: `RtFunc::TimeNowEpochNanos => (vec![], MirTy::I64),`

- [ ] **Step 6: Wire seam 7 — `nova-mir/src/lower.rs`**

`Builtin::TimeNowEpochNanos => Lowering::Runtime(RtFunc::TimeNowEpochNanos),`

- [ ] **Step 7: Wire seams 1, 2, 3 — `nova-resolver`**

Add the `Builtin` variant with a doc comment noting it is std-only and distinct from `TimeNowNanos`; add `Builtin::TimeNowEpochNanos => "time_now_epoch_nanos",` to `name()`; add the variant to `STD_ONLY` and **bump its length from 60 to 61**.

- [ ] **Step 8: Wire seams 4, 5, 6 — `nova-typeck`**

Add `Builtin::TimeNowEpochNanos` to the `""` interpolation-hint arm; add `Builtin::TimeNowEpochNanos => (vec![], Ty::Int),` to `builtin_signature`; add a description-table entry. **Spell any type out explicitly** rather than reaching for a helper — a previous increment's brief used `future_of_param0` in a context where it was out of scope.

- [ ] **Step 9: Wire seam 12 — the one nothing forces**

In `crates/nova-runtime/src/lib.rs`'s `symbols()`, beside the existing `nova_rt_time_now_nanos` entry:

```rust
        (
            "nova_rt_time_now_epoch_nanos",
            time::nova_rt_time_now_epoch_nanos as *const u8,
        ),
```

Then run the guard by name: `cargo test --locked -p nova-codegen-cranelift every_rt_func_symbol_is_registered_with_the_jit`
Expected: PASS. **Confirm it fails without the entry** — comment the entry out, run it, see `RtFunc variants with no nova_runtime::symbols() entry: ["nova_rt_time_now_epoch_nanos"]`, then restore.

- [ ] **Step 10: Add `SystemTime::now` and its fixture**

In `std/time/lib.nova`, inside `impl SystemTime`:

```nova
    pub fn now() -> SystemTime { SystemTime { nanos: time_now_epoch_nanos() } }
```

Create `tests/runtime/system_time_now.nova`:

```nova
// The live clock, asserted on shape rather than value: the year must be a
// plausible four digits and the rendering must be exactly 24 characters
// ending in `Z`. A process-relative reading would render as 1970 and fail
// the first check.
fn main() {
    let s = SystemTime::now().to_iso8601()
    println("${s.len()}")
    println("${s.ends_with("Z")}")
    println("${s.starts_with("20")}")
}
```

`tests/runtime/system_time_now.stdout`:

```
24
true
true
```

Register it in `run_tests.rs` as `system_time_now_run`, same shape as Task 1's registration.

- [ ] **Step 11: Full suite**

`cargo build --locked --workspace` then `cargo test --locked --workspace --all-features --no-fail-fast`, summing every non-`trapped` `test result:` line.
Expected: **1012 passed / 0 failed / 8 ignored across 44 targets** (+2 from Task 1's total: one runtime unit test, one fixture).

Then clippy and fmt. **After fmt, check CRLF on every touched `.rs` file** — this is the fifth increment in which `cargo fmt` may flatten them.

- [ ] **Step 12: Commit**

```bash
git add crates/ std/time/lib.nova tests/runtime/system_time_now.nova tests/runtime/system_time_now.stdout
git commit -F <path-to-utf8-message-file>
```

Subject: `feat(runtime): add a wall clock, separate from the process epoch`. The body must say why it does not reuse `now_nanos`, and that seam 12 was verified to fail without its entry.

---

## Task 3: The configuration cell and its three intrinsics

**Files:**
- Create: `crates/nova-runtime/src/log.rs`
- Modify: `crates/nova-runtime/src/lib.rs` (`mod log;` plus three `symbols()` entries)
- Modify: `crates/nova-mir/src/lib.rs`, `crates/nova-mir/src/lower.rs`, `crates/nova-resolver/src/lib.rs`, `crates/nova-typeck/src/check.rs`

**Interfaces:**
- Consumes: nothing from Tasks 1–2.
- Produces: Nova builtins `log_config_level() -> Int`, `log_config_to_stderr() -> Int`, `log_set_config(level: Int, to_stderr: Int) -> unit`; `RtFunc::LogConfigLevel`, `LogConfigToStderr`, `LogSetConfig`.

- [ ] **Step 1: Write the three failing unit tests**

Create `crates/nova-runtime/src/log.rs` with only its test module first:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    /// An unset cell resolves to the spec's default rather than to zero:
    /// level `Info` (2), output stderr. This *is* the auto-initialize rule
    /// — there is no separate init path, the getter's `None` arm is it.
    #[test]
    fn an_unset_config_reads_as_the_default() {
        reset_for_test();
        assert_eq!(nova_rt_log_config_level(), 2);
        assert_eq!(nova_rt_log_config_to_stderr(), 1);
    }

    #[test]
    fn set_then_get_round_trips() {
        reset_for_test();
        nova_rt_log_set_config(4, 0);
        assert_eq!(nova_rt_log_config_level(), 4);
        assert_eq!(nova_rt_log_config_to_stderr(), 0);
    }

    /// Last writer wins, which is what makes `init_with` after a log call
    /// reconfigure rather than being ignored.
    #[test]
    fn the_second_set_wins() {
        reset_for_test();
        nova_rt_log_set_config(0, 1);
        nova_rt_log_set_config(3, 0);
        assert_eq!(nova_rt_log_config_level(), 3);
        assert_eq!(nova_rt_log_config_to_stderr(), 0);
    }
}
```

- [ ] **Step 2: Run them and watch them fail**

Run: `cargo test --locked -p nova-runtime log::tests`
Expected: FAIL to compile.

- [ ] **Step 3: Implement the cell**

```rust
//! The logger's configuration, and nothing else.
//!
//! Nova has no mutable global state — top-level bindings are `const` — so
//! the logger's level and destination live here, in the shape `file.rs`'s
//! open-file table and `task.rs`'s `CURRENT` already establish.
//!
//! **Thread-local, not global**, because the executor is single-threaded
//! and every other piece of runtime state here already is. If Nova grows
//! real threads, a per-thread logger configuration is the wrong answer and
//! changing it is ADR-worthy — recorded now so that it is a decision then
//! rather than a discovery.

use std::cell::Cell;

#[derive(Clone, Copy)]
struct Config {
    level: i64,
    to_stderr: bool,
}

/// `None` means "never configured", which the getters resolve to the
/// default. That resolution is the entire auto-initialize rule: a program
/// that never calls `Log::init()` still logs, because the first *read*
/// installs the default.
const DEFAULT: Config = Config {
    level: 2, // Info
    to_stderr: true,
};

thread_local! {
    static CONFIG: Cell<Option<Config>> = const { Cell::new(None) };
}

fn get() -> Config {
    CONFIG.with(|c| match c.get() {
        Some(cfg) => cfg,
        None => {
            c.set(Some(DEFAULT));
            DEFAULT
        }
    })
}

/// The threshold, as `LogLevel::to_int` numbers it.
#[no_mangle]
pub extern "C-unwind" fn nova_rt_log_config_level() -> i64 {
    get().level
}

/// `1` for stderr, `0` for stdout. An `i64` rather than a Rust `bool`
/// because every other intrinsic in this crate crosses the boundary as one.
#[no_mangle]
pub extern "C-unwind" fn nova_rt_log_config_to_stderr() -> i64 {
    if get().to_stderr { 1 } else { 0 }
}

/// Install a configuration, overwriting any previous one.
///
/// Two separate getters above rather than one packed integer
/// (`level * 2 + to_stderr`), deliberately: packing would save one builtin
/// and reintroduce an `i64` whose *meaning* the compiler cannot check,
/// which this project has already had to rename its way out of twice.
#[no_mangle]
pub extern "C-unwind" fn nova_rt_log_set_config(level: i64, to_stderr: i64) {
    CONFIG.with(|c| {
        c.set(Some(Config {
            level,
            to_stderr: to_stderr != 0,
        }))
    });
}

/// Clear the cell so a test can observe the unset behaviour. `#[cfg(test)]`
/// only — nothing in a running program un-configures a logger.
#[cfg(test)]
fn reset_for_test() {
    CONFIG.with(|c| c.set(None));
}
```

Add `mod log;` to `crates/nova-runtime/src/lib.rs` beside `mod time;` — **private**, matching `file`, `net`, `poll` and `time`, since nothing outside the crate needs it.

- [ ] **Step 4: Run the three tests to green**

Run: `cargo test --locked -p nova-runtime log::tests`
Expected: 3 passed.

- [ ] **Step 5: Wire all three builtins through seams 1–11**

Exactly as Task 2's steps 5–8, three times. Signatures:

| Builtin | Nova name | typeck | MIR |
|---|---|---|---|
| `LogConfigLevel` | `log_config_level` | `(vec![], Ty::Int)` | `(vec![], MirTy::I64)` |
| `LogConfigToStderr` | `log_config_to_stderr` | `(vec![], Ty::Int)` | `(vec![], MirTy::I64)` |
| `LogSetConfig` | `log_set_config` | `(vec![Ty::Int, Ty::Int], Ty::Unit)` | `(vec![MirTy::I64, MirTy::I64], MirTy::Unit)` |

`STD_ONLY` goes **61 → 64**.

**Both getters return `Int`, not `Bool`,** even though `to_stderr` is a boolean idea: the boundary carries `i64`, and the Nova side compares against `1`. Introducing a `Bool`-returning intrinsic here would be the only one in the crate.

- [ ] **Step 6: Wire seam 12 for all three, and prove the guard bites**

Add three `symbols()` entries. Then comment out **one** of them, run `cargo test --locked -p nova-codegen-cranelift every_rt_func_symbol_is_registered_with_the_jit`, confirm it names exactly that symbol, and restore.

- [ ] **Step 7: Full suite, clippy, fmt, CRLF**

Expected: **1015 passed / 0 failed / 8 ignored across 44 targets** (+3).

- [ ] **Step 8: Commit**

Subject: `feat(runtime): hold the logger's level and destination in a cell`. Body must record why two getters rather than a packed integer, and why thread-local.

---

## Task 4: The `std/log` module

**Files:**
- Create: `std/log/lib.nova`
- Modify: `crates/nova-resolver/src/lib.rs` (`STD_MODULES`, 9 → 10)
- Create: five fixtures under `tests/runtime/` with `.stdout` pairs
- Modify: `crates/nova-cli/tests/run_tests.rs` (five registrations)

**Interfaces:**
- Consumes: `SystemTime::now()`/`to_iso8601()` (Tasks 1–2); `log_config_level`, `log_config_to_stderr`, `log_set_config` (Task 3).
- Produces: `Log`, `LogLevel`, `LogFormat`, `LogOutput`, `LogConfig`.

- [ ] **Step 1: Register the module**

In `crates/nova-resolver/src/lib.rs`, add to `STD_MODULES` and bump its length **9 → 10**:

```rust
    ("$std.log", include_str!("../../../std/log/lib.nova")),
```

Place it **after** `$std.time`: `import_std_module` runs std-into-std in this order, and `std/log` uses `SystemTime` from `std/time`.

- [ ] **Step 2: Write the module**

Create `std/log/lib.nova`:

```nova
module std.log

// Levels, ordered by `to_int` below. The order is load-bearing: filtering
// compares those integers, because Nova has no `==` on sum types
// (`E0013`), so `to_int` is the only mechanism available rather than a
// convenience.
pub type LogLevel = | Trace | Debug | Info | Warn | Error

// One variant today. `Json` arrives with the increment that brings string
// escaping; declaring the field now means adding that variant later breaks
// only exhaustive matches, where adding a *field* to `LogConfig` would
// break every construction site.
pub type LogFormat = | Human

// `File(String)` arrives with the increment that adds a synchronous
// file-write path; every `std/fs` write is `async fn` today.
pub type LogOutput = | Stderr | Stdout

pub record LogConfig {
    pub level: LogLevel
    pub format: LogFormat
    pub output: LogOutput
}

// The logger's namespace. An empty record because Nova has no free-standing
// namespaces, and associated functions rather than top-level ones because a
// top-level `pub fn` is glob-imported into every module: `error`, `info` and
// `debug` at top level would take those names from all user code, and
// `import_std_module` resolves such a collision silently in the *user's*
// favour — making this module's `error` unreachable with no diagnostic.
// `std/strings` refused the same trade for `join`.
pub record Log {}

impl LogLevel {
    pub fn to_int(self) -> Int {
        match self { Trace => 0, Debug => 1, Info => 2, Warn => 3, Error => 4 }
    }

    pub fn label(self) -> String {
        match self {
            Trace => "TRACE"
            Debug => "DEBUG"
            Info => "INFO"
            Warn => "WARN"
            Error => "ERROR"
        }
    }
}

// `match` rather than `out == Stderr`: there is no equality on sum types.
fn to_stderr_flag(out: LogOutput) -> Int {
    match out { Stderr => 1, Stdout => 0 }
}

// Emit one line if `level` clears the configured threshold.
//
// Private, so the glob import never binds it and it takes no name from user
// code. The threshold is checked *before* the clock is read and the line is
// built, so a filtered-out call costs one intrinsic and no allocation.
//
// The comparison is `<`, which makes the threshold inclusive: at threshold
// `Info`, an `Info` message is emitted.
fn emit(level: LogLevel, msg: String) {
    if level.to_int() < log_config_level() { return }
    let line = "${SystemTime::now().to_iso8601()} ${level.label()} ${msg}"
    if log_config_to_stderr() == 1 { eprintln(line) } else { println(line) }
}

impl Log {
    // Install the default: `Info`, `Human`, `Stderr`. Not a prerequisite —
    // the runtime's getter installs the same default on first read, so a
    // program that never calls this still logs. This is the override spelled
    // explicitly.
    pub fn init() {
        Log::init_with(LogConfig { level: Info, format: Human, output: Stderr })
    }

    // `config.format` is deliberately unread: `LogFormat` has one variant,
    // so there is nothing to branch on yet.
    pub fn init_with(config: LogConfig) {
        log_set_config(config.level.to_int(), to_stderr_flag(config.output))
    }

    pub fn trace(msg: String) { emit(Trace, msg) }
    pub fn debug(msg: String) { emit(Debug, msg) }
    pub fn info(msg: String)  { emit(Info, msg) }
    pub fn warn(msg: String)  { emit(Warn, msg) }
    pub fn error(msg: String) { emit(Error, msg) }
}
```

- [ ] **Step 3: Write the five fixtures**

Every log line carries a live timestamp, so **each `.nova` fixture prints only the part after it**, using `String::slice` to drop the fixed 24-character prefix plus its space. That keeps the golden exact instead of pattern-matched.

`tests/runtime/log_default_level.nova` — logging with no `init` anywhere:

```nova
// No `init` call: the runtime getter installs the default on first read, so
// `Info` and above emit and `Debug`/`Trace` do not. `Info` at threshold
// `Info` is the case that catches `<` becoming `<=`.
//
// The fixture prints nothing but `done` on stdout; the level lines go to
// stderr, and the harness splits the live timestamp off each one.
fn main() {
    Log::trace("no")
    Log::debug("no")
    Log::info("yes-info")
    Log::warn("yes-warn")
    Log::error("yes-error")
    println("done")
}
```

Because the level lines go to **stderr** by default and only `done` reaches stdout, this fixture's `.stdout` is:

```
done
```

and the assertion must also check stderr. Register it with an explicit `.stderr(...)` expectation:

```rust
/// No `init` call anywhere: the default threshold is `Info`, so `Trace` and
/// `Debug` are dropped while `Info`, `Warn` and `Error` reach stderr.
/// `Info` at threshold `Info` is what fails if `<` becomes `<=`.
#[test]
fn log_default_level_run() {
    let out = nova()
        .arg("run")
        .arg(repo_root().join("tests/runtime/log_default_level.nova"))
        .assert()
        .success();
    let stderr = String::from_utf8_lossy(&out.get_output().stderr).replace("\r\n", "\n");
    let levels: Vec<&str> = stderr
        .lines()
        .map(|l| l.split_once(' ').expect("line starts with a timestamp").1)
        .collect();
    assert_eq!(levels, vec!["INFO yes-info", "WARN yes-warn", "ERROR yes-error"]);
}
```

Splitting on the first space and discarding the timestamp is what makes this assert the level and message **exactly** while tolerating a live clock — stronger than a substring match, and immune to the clock advancing between runs.

`tests/runtime/log_init_with_threshold.nova` — `init_with` at `Warn` silences `Info`:

```nova
fn main() {
    Log::init_with(LogConfig { level: Warn, format: Human, output: Stderr })
    Log::info("silenced")
    Log::warn("kept-warn")
    Log::error("kept-error")
    println("done")
}
```

Expect stderr levels `["WARN kept-warn", "ERROR kept-error"]`.

`tests/runtime/log_reconfigure_after_logging.nova` — log first, then reconfigure:

```nova
fn main() {
    Log::info("before")
    Log::init_with(LogConfig { level: Error, format: Human, output: Stderr })
    Log::info("after-silenced")
    Log::error("after-kept")
    println("done")
}
```

Expect stderr levels `["INFO before", "ERROR after-kept"]`. This is the fixture that fails if a getter returns a hardcoded default instead of reading the cell.

`tests/runtime/log_stdout_output.nova` — `output: Stdout` moves the line:

```nova
fn main() {
    Log::init_with(LogConfig { level: Info, format: Human, output: Stdout })
    Log::info("on-stdout")
}
```

Assert **stdout** has one line whose post-timestamp text is `INFO on-stdout`, and that stderr is empty. This is the fixture that fails if `eprintln` and `println` are swapped.

`tests/runtime/log_level_labels.nova` — all five labels at threshold `Trace`:

```nova
fn main() {
    Log::init_with(LogConfig { level: Trace, format: Human, output: Stdout })
    Log::trace("a")
    Log::debug("b")
    Log::info("c")
    Log::warn("d")
    Log::error("e")
}
```

Assert stdout's five post-timestamp texts are `TRACE a`, `DEBUG b`, `INFO c`, `WARN d`, `ERROR e` in order. This is the fixture that fails if `Warn` and `Error` swap in `to_int`, because the threshold ordering changes which lines appear — and it pins every label.

- [ ] **Step 4: Register all five — nothing is automatic**

Five `#[test]` functions in `crates/nova-cli/tests/run_tests.rs`. **A fixture without a registration runs zero tests and looks like a pass.** After adding them, confirm the count:

Run: `cargo test --locked -p nova-cli --test run_tests log_ -- --list`
Expected: exactly five test names beginning `log_`.

- [ ] **Step 5: Run them**

Run: `cargo build --locked --workspace` then `cargo test --locked -p nova-cli --test run_tests log_`
Expected: 5 passed.

- [ ] **Step 6: Full suite**

Expected: **1020 passed / 0 failed / 8 ignored across 44 targets** (+5).

- [ ] **Step 7: The four mutations, each with its named test**

Run each, confirm the named test fails, restore, confirm green. Verify every restore with `git diff`.

| Mutation | Must fail |
|---|---|
| `emit`'s `<` becomes `<=` | `log_default_level_run` (`INFO yes-info` disappears) |
| `to_int` swaps `Warn => 4, Error => 3` | `log_level_labels_run` |
| `nova_rt_log_config_level` returns a literal `2` instead of `get().level` | `log_reconfigure_after_logging_run` |
| `emit` swaps `eprintln` and `println` | `log_stdout_output_run` |

The first is the one to be careful about: `<` versus `<=` is one character, and a test at any level pair *other* than a message exactly at the threshold cannot catch it.

- [ ] **Step 8: Clippy, fmt, CRLF, commit**

Subject: `feat(std): add std/log over associated functions on Log`. The body must record why the levels are `Log::` associated functions rather than top-level, naming the silent-shadowing direction.

---

## Task 5: The records

No code. This task makes the project's documents true.

**Files:**
- Modify: `CHANGELOG.md`, `nova-spec/20-STDLIB.md`, `docs/superpowers/specs/2026-08-17-std-time-design.md`
- Create: `docs/adr/0014-stdlib-build-order-deviations.md`

- [ ] **Step 1: CHANGELOG `[Unreleased]`**

**Added only** — this increment renames and removes nothing. One entry for `std/log` (the five levels as `Log::` associated functions and *why*, level filtering, Stderr/Stdout, and what is deferred), one for `SystemTime` with ISO-8601 UTC. Name the four new builtins and the counts: `STD_ONLY` 60 → 64, `STD_MODULES` 9 → 10, `RESERVED_TYPE_NAMES` unchanged at 7.

- [ ] **Step 2: `nova-spec/20-STDLIB.md` §9**

A dated **AMENDED 2026-08-19** paragraph recording `SystemTime`, that it is a separate type from `Instant` and why, that it is UTC-only and why that is permanent, and that this **discharges the wall-clock deferral** §9 recorded — quoting the condition it set ("`std/log` will eventually want a timestamp… it waits for the increment that needs it") and naming this increment as the consumer.

- [ ] **Step 3: `nova-spec/20-STDLIB.md` §10**

A dated amendment recording: the `Log::` shape and its reason; that `Json`, `File(String)` and TTY detection are a named next increment rather than gaps; and that log calls return nothing and cannot fail because a logger has nowhere to report a stderr failure.

- [ ] **Step 4: ADR 0014 — the build-order deviation**

`docs/adr/0014-stdlib-build-order-deviations.md`. Check the number is unused first: `ls docs/adr/` — `0013-io-poller.md` exists, so 0014 is next.

Content: **Context** — §3 calls the build order strict; position 2 (`std/fmt`) is the earliest incomplete entry and has now been passed twice, first by Phase 2.1 (which deferred it behind async, recorded in `2026-07-25-phase-2-1-std-core-design.md`) and now by this increment. **Decision** — take fully-specified modules with existing dependencies ahead of position 2, and require an ADR entry each time it happens. **Consequences** — what `std/fmt` still actually needs (`Display` exists and interpolation already calls it, so two thirds would replace working mechanisms; `Formatter`'s body is elided in the spec, so that increment starts by designing what §3 left blank), and that a skip recorded twice is a decision while a skip recorded zero times is an oversight.

- [ ] **Step 5: Point the `std/time` spec's deferral here**

In `docs/superpowers/specs/2026-08-17-std-time-design.md` §1, append to the wall-clock deferral the same way its `timeout<T>` deferral was pointed at the combinator spec: name this spec, and say the deferral is discharged rather than still open.

- [ ] **Step 6: Verify, then commit**

Full suite must be **unchanged at 1020 / 0 / 8 across 44 targets** — for a records-only task an unchanged total is the evidence, not a formality. Clippy and fmt clean. Check CRLF on every markdown file touched, before and after.

Subject: `docs: record std/log, the wall clock, and the build-order deviation`.

---

## Plan Self-Review

**Spec coverage.** §1 scope → Tasks 1–4 (in) and Task 5 §10 amendment (out, named). §2 build order → Task 5 step 4. §3 `Log::` shape → Task 4 step 2. §4 wall clock → Task 2. §5 ISO-8601 → Task 1. §6 surface → Task 4. §7 config and auto-init → Task 3. §8 edge cases → covered by Task 3's default test, Task 4's fixtures, and the clock's `Err(_) => 0` arm; the 2262 saturation and the newline-in-message case are *recorded* rather than tested, as the spec states. §9 testing → Tasks 1–4. §10 records → Task 5. §11 measured facts → used throughout. **No gaps.**

**Type consistency.** `SystemTime { nanos: Int }` in Tasks 1 and 2; `to_iso8601` and `now` on the same impl; `log_config_level`/`log_config_to_stderr`/`log_set_config` named identically in Tasks 3 and 4; `LogLevel::to_int`/`label` defined in Task 4 step 2 and used by `emit` in the same file; `civil_from_days` returns `[Int]` and is indexed as `ymd[0..2]` in Task 1 step 6.

**Two deviations from the spec, both deliberate and flagged in place rather than silent:** padding uses interpolation instead of `String::repeat` (Task 1 step 4, probed working); and the fixtures assert by splitting the timestamp off in the *harness* rather than in Nova, which is simpler than the spec's "normalize the prefix" wording and strictly more exact.

**One thing the spec asks for that this plan deliberately does not do:** spec §9 lists a `log_stdout_output` fixture asserting stdout "which the golden distinguishes from stderr". This plan asserts stderr is *empty* as well, which is strictly stronger.
