# `Read`/`Write` and the Standard Streams Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship `nova-spec/20-STDLIB.md` §4's `Read` and `Write` traits plus concrete `Stdin`/`Stdout`/`Stderr`, using the `Future<T>` trait spelling, over increment 3a's per-task slot boundary.

**Architecture:** Both traits are declared with `fn … -> Future<T>` rather than `async fn`, because `async fn` in a trait declaration is a hard `E0900`. `read_to_end` is a default body that *returns* the future of a private generic `async fn read_all<T: Read>`, since a default body returning `Future<T>` is not itself `async` and cannot `await`. Five new intrinsics — one per stream operation — return one status word and leave payloads in the per-task slot table `fs.rs` has owned since increment 3a; `std/io`'s wrappers collect them with the existing `fs_take_bytes` and `fs_last_error_message` builtins.

**Tech Stack:** Rust 2021 (MSRV 1.78) across `nova-resolver`, `nova-typeck`, `nova-mir`, `nova-runtime`; Nova source in `std/io`; fixtures under `tests/runtime` driven by `crates/nova-cli/tests/run_tests.rs`.

**Spec:** `docs/superpowers/specs/2026-08-13-read-write-and-stdio-design.md`

## Global Constraints

- `cargo build --workspace` **before** `cargo test`. Not optional — tests link the built runtime.
- `cargo test --workspace --no-fail-fast` at default parallelism. Sum **every** `test result:` line. **Never pipe cargo output through `head` or `tail`** before summing.
- Baseline **891 passed / 0 failed / 8 ignored across 44 targets.** State the arithmetic for every task.
- **A zero-match test filter exits 0 here**, so a filter typo reads as a pass. `cargo test` rejects multiple positional filters — one name per invocation. Name the tests you ran and confirm they ran.
- **No panic may cross a generated poll boundary.** Generated code has no landing pads. `abort_with` is acceptable — it terminates without unwinding. `RefCell` borrow panics, `unwrap`, `expect`, slice indexing and fallible `format!` are **not**, in production code. Panics inside `#[cfg(test)]` are fine.
- **Every Nova wrapper is straight-line: no `.await` between an intrinsic call and its slot read.** 3a made a suspension there survivable *across* tasks; it did not make it correct *within* one.
- The **8 `#[ignore]`d ADR-0010 GC tests** stay ignored and untouched.
- `cargo clippy --all-targets --all-features -- -D warnings` and `cargo fmt --all --check` clean. **No `reason = "…"` in any lint attribute** (MSRV 1.78; that CI job is known-vacuous, so it would ship as a user build failure).
- **No line-number citation in a doc comment that points into its own file.** The previous branch shipped that twice, both stale on arrival. Name the symbol.
- Every fixture path unique per process.
- Commit messages to a **UTF-8 file**, applied with `git commit -F` — **never a heredoc.** Body ends `Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>`. **Never push.**
- Branch: `read-write-stdio`, already created; spec committed at `5815a3d`.

## The ten seams a new intrinsic touches

Verified by tracing `FsTakeBytes` end to end. Miss one and it fails at a different stage each time, so work the list:

| # | File | What |
|---|---|---|
| 1 | `nova-resolver/src/lib.rs:313` area | `Builtin` enum variant |
| 2 | `nova-resolver/src/lib.rs:444` area | `Builtin::X => "io_stdin_read"` — the **Nova-visible** name |
| 3 | `nova-resolver/src/lib.rs:521` area | `Builtin::STD_ONLY` array entry (43 → 48) |
| 4 | `nova-typeck/src/check.rs:3940` area | the `\|`-chain in `check_builtin_call` listing every fs/bytes builtin |
| 5 | `nova-typeck/src/check.rs:7163` area | `builtin_signature` — arity and types |
| 6 | `nova-mir/src/lib.rs:278` area | `RtFunc` enum variant |
| 7 | `nova-mir/src/lib.rs:357` area | `RtFunc::X => "nova_rt_io_stdin_read"` — the **Rust symbol** |
| 8 | `nova-mir/src/lib.rs:415` area | `signature()` → `MirTy`s |
| 9 | `nova-mir/src/lower.rs:709` area | `Builtin::X => Lowering::Runtime(RtFunc::X)` |
| 10 | `nova-runtime/src/lib.rs:471` area | **`symbols()`** — keyed by the lowercase symbol string, so **invisible to a PascalCase grep**. This is a documented blind spot; a fully wired intrinsic that is missing here links but does not resolve at JIT time. |

Plus the Rust body itself. **Line numbers above are approximate and will drift as you insert — locate each by the `FsTakeBytes`/`FsRead` entry beside it, not by line number.**

## File Structure

| File | Responsibility | Change |
|---|---|---|
| `crates/nova-runtime/src/io.rs` | **New.** The five stream intrinsics: acquire the process stream, act, drop it inside the call | Create |
| `crates/nova-runtime/src/lib.rs` | Runtime root; `symbols()` | Modify: `mod io;`, five `symbols()` entries |
| `crates/nova-runtime/src/fs.rs` | Owns the per-task slot table and its takers | Modify: make `stash`/`take` reachable from `io.rs`, and note the `fs_` prefix is historical |
| `crates/nova-resolver/src/lib.rs` | Builtin names, `STD_ONLY` | Modify: seams 1–3 |
| `crates/nova-typeck/src/check.rs` | Builtin typing | Modify: seams 4–5 |
| `crates/nova-mir/src/lib.rs`, `lower.rs` | `RtFunc`, symbols, lowering | Modify: seams 6–9 |
| `std/io/lib.nova` | The traits, the three stream types, the wrappers | Modify: the whole new surface |
| `std/fs/lib.nova` | Filesystem wrappers | Modify: **one stale claim, see Task 1 Step 1** |
| `tests/runtime/io_streams.nova` + `.stdout` | Stream fixture | Create |
| `tests/runtime/io_read_to_end.nova` + `.stdout` | The default-body fixture | Create |
| `crates/nova-cli/tests/run_tests.rs` | Fixture harness | Modify: the new runners |
| `nova-spec/20-STDLIB.md`, `docs/adr/0009-*`, `docs/adr/0011-*`, `CHANGELOG.md` | Records | Modify: Task 4 |

---

### Task 1: The runtime boundary — five intrinsics

**Files:**
- Create: `crates/nova-runtime/src/io.rs`
- Modify: `crates/nova-runtime/src/lib.rs` (add `mod io;`, five `symbols()` entries), `crates/nova-runtime/src/fs.rs` (visibility + prefix note), `std/fs/lib.nova` (one stale claim)
- Test: `crates/nova-runtime/src/io.rs`'s own `#[cfg(test)] mod tests`

**Interfaces:**
- Consumes: `fs.rs`'s `Slot`, `stash`, `take`, `fail`, and the status constants `OK`..`OTHER`; `crate::task::abort_with`; `crate::NovaStr`; `crate::bytes::{gc_bytes, as_bytes}`
- Produces, for Task 2: five `pub unsafe extern "C" fn`s — `nova_rt_io_stdin_read(max: i64) -> i64`, `nova_rt_io_stdout_write(buf: *const NovaStr) -> i64`, `nova_rt_io_stderr_write(buf: *const NovaStr) -> i64`, `nova_rt_io_stdout_flush() -> i64`, `nova_rt_io_stderr_flush() -> i64`

- [ ] **Step 1: Fix the stale claim increment 3a left in `std/fs/lib.nova`**

Do this first, because Task 3 writes the same paragraph for `std/io` and must not copy a false one.

`std/fs/lib.nova`'s "The shape of these wrappers is load-bearing" section currently reads: *"The runtime's payload and message slots are **per-thread** and are overwritten by the next operation; only an `.await` can let another task run, so a straight-line wrapper cannot have its slot **clobbered by a sibling**."*

**Both halves are now false, and this is shipped on `main`.** Increment 3a made the slots **per-task**, keyed on the polling task — a sibling clobbering your slot is exactly what it eliminated. Verify by reading `fs.rs`'s `SLOTS`/`slot_index` before editing.

The discipline it argues for is still correct, for a different reason: **within one task**, a second operation overwrites that task's own slot, so a wrapper that awaited mid-sequence could still read a payload its own later call replaced. Rewrite the paragraph to say that, and do not keep the sibling argument.

- [ ] **Step 2: Write the failing test for `stdout_write`**

In `io.rs`'s `mod tests`. This asserts the boundary contract, not the OS write:

```rust
/// A successful write reports the byte count through the per-task slot.
///
/// The count travels as a payload rather than in the status word because the
/// status word carries the `IoErrorKind`, and a byte count and an error kind
/// cannot share one `i64` without a sentinel this boundary does not have.
#[test]
fn a_successful_stdout_write_reports_its_byte_count() {
    let payload = crate::gc_bytes_for_test(b"hi");
    let status = unsafe { nova_rt_io_stdout_write(payload) };
    assert_eq!(status, crate::fs::OK, "writing to stdout must succeed");
    assert_eq!(
        take_count(),
        2,
        "the byte count must be the two bytes written, read back from the slot"
    );
}
```

If `gc_bytes_for_test` and `take_count` do not exist, write the smallest `#[cfg(test)]` helpers that make this test express its intent, and say in your report what you added and why. **Gate them `#[cfg(test)]` with no platform gate** — a `pub(crate)` test helper in this crate previously had to be `#[cfg(windows)]`-gated because its only callers were, and that mismatch shipped as a Linux-only clippy failure.

- [ ] **Step 3: Run it and confirm it fails**

```bash
cargo build --workspace
```

```bash
cargo test -p nova-runtime a_successful_stdout_write_reports_its_byte_count
```

Expected: FAIL to compile — `nova_rt_io_stdout_write` does not exist.

- [ ] **Step 4: Write `io.rs`**

Model every function on `fs.rs`'s existing intrinsics: acquire, act, map errors through `fail`, stash payloads, return a status word. Concretely:

```rust
//! The three standard streams' boundary.
//!
//! Each intrinsic acquires the process-global stream, acts, and drops it inside
//! the one call -- the same shape `nova_rt_print`/`nova_rt_eprint` have used
//! since phase 1. Nothing here holds an OS handle between calls, which is why
//! these three types need no lifetime management and `File` (increment 3c) will.
//!
//! Payloads travel in the per-task slot table `fs` owns. No panic may cross a
//! generated poll boundary, so nothing here unwraps, expects, indexes a slice,
//! or formats fallibly.

use crate::fs::{fail, stash, Slot, OK};
use crate::NovaStr;
use std::io::{Read as _, Write as _};

/// Read up to `max` bytes from stdin. An **empty** payload means end of stream.
///
/// A short read is not EOF: a terminal or pipe returns what is available. The
/// Nova-level contract is stated in `std/io`'s `Read::read`.
#[no_mangle]
pub unsafe extern "C" fn nova_rt_io_stdin_read(max: i64) -> i64 {
    let Ok(cap) = usize::try_from(max) else {
        crate::task::abort_with("nova_rt_io_stdin_read: negative maximum")
    };
    let mut buf = vec![0u8; cap];
    let stdin = std::io::stdin();
    let mut lock = stdin.lock();
    match lock.read(&mut buf) {
        Ok(n) => {
            buf.truncate(n);
            stash(Slot::Buffer, crate::bytes::gc_bytes(&buf));
            OK
        }
        Err(e) => fail(&e),
    }
}
```

Write the other four in the same shape. `stdout_write`/`stderr_write` take a `*const NovaStr`, read it with `crate::bytes::as_bytes`, write all of it, and stash the count; `stdout_flush`/`stderr_flush` flush and return a status only.

**Two things to decide and record in your report rather than guess:** how the byte count reaches Nova (a `Bytes` payload the wrapper converts, or a second `usize` slot — prefer whichever needs no new slot kind), and whether `write` must loop on partial writes (`write_all` versus `write`). Say which you chose and why.

`stash` and `take` are private to `fs.rs` today. Make them `pub(crate)` — **the narrowest change** — and add a note at each that `io.rs` is now a second consumer.

- [ ] **Step 5: Register all five in `symbols()`**

`crates/nova-runtime/src/lib.rs`, beside the `nova_rt_fs_take_bytes` entry. **This table is keyed by the lowercase symbol string and is invisible to a PascalCase grep** — an intrinsic missing here compiles and links but fails to resolve at JIT time. Add `mod io;` too.

- [ ] **Step 6: Run the tests and confirm they pass**

```bash
cargo build --workspace
```

```bash
cargo test -p nova-runtime io::
```

Expected: your new tests pass, and the count is non-zero (not a zero-match pass).

```bash
cargo test --workspace --no-fail-fast
```

Expected: **891 + (tests you added)**. State the arithmetic.

- [ ] **Step 7: Mutation-test the boundary**

Per intrinsic, apply the plausible wrong implementation and confirm a test dies. At minimum: make `stdout_write` write to **stderr**; make `stdin_read` skip the `truncate(n)` so a short read returns `max` bytes of zero padding; make a `fail` path return `OK`. **Report each mutation and whether it died.** A survivor is a real result — say what a test that killed it would have to assert, and do not add a test that only appears to.

- [ ] **Step 8: Lint and commit**

```bash
cargo clippy --all-targets --all-features -- -D warnings
```

```bash
cargo fmt --all --check
```

```bash
git add -A && git commit -F <message-file>
```

---

### Task 2: Wire the five builtins through the compiler

**Files:**
- Modify: `crates/nova-resolver/src/lib.rs` (seams 1–3), `crates/nova-typeck/src/check.rs` (seams 4–5), `crates/nova-mir/src/lib.rs` (seams 6–8), `crates/nova-mir/src/lower.rs` (seam 9)
- Test: `crates/nova-typeck/src/check.rs`'s and `crates/nova-mir/src/lib.rs`'s own test modules

**Interfaces:**
- Consumes from Task 1: the five `nova_rt_io_*` Rust symbols
- Produces, for Task 3: five Nova-visible builtins — `io_stdin_read(Int) -> Int`, `io_stdout_write(Bytes) -> Int`, `io_stderr_write(Bytes) -> Int`, `io_stdout_flush() -> Int`, `io_stderr_flush() -> Int`. All return a **status word**; payloads come back through `fs_take_bytes`.

- [ ] **Step 1: Write the failing typeck test**

```rust
/// The five stream builtins are visible in an std module and typed as status
/// words.
///
/// `STD_ONLY` is seeded into every std module's scope, not per-module (see
/// `Builtin::STD_ONLY`'s own doc), so `std/io` reaches these without new
/// plumbing -- and so does any other std module, which is why their names carry
/// the `io_` prefix rather than relying on scope to disambiguate.
#[test]
fn the_stream_builtins_are_std_only_and_return_status_words() {
    for name in [
        "io_stdin_read",
        "io_stdout_write",
        "io_stderr_write",
        "io_stdout_flush",
        "io_stderr_flush",
    ] {
        assert!(
            nova_resolver::Builtin::STD_ONLY
                .iter()
                .any(|b| b.name() == name),
            "{name} must be an STD_ONLY builtin"
        );
    }
}
```

Adapt the accessor to whatever `Builtin` actually exposes — read the enum before writing this, and if the real accessor is not `name()`, use the real one and say so.

- [ ] **Step 2: Run it and confirm it fails**

```bash
cargo test -p nova-typeck the_stream_builtins_are_std_only_and_return_status_words
```

Expected: FAIL — the builtins do not exist.

- [ ] **Step 3: Work the ten seams**

Follow the table in this plan's header. For each of the five, copy the shape of the nearest existing analogue: `FsRead` for `io_stdin_read` (takes a value, returns a status, payload in a slot), `FsWrite` for the two writes, `FsExists` for the two flushes if it is the closest no-payload shape.

**`STD_ONLY` goes from 43 to 48.** `STD_MODULES` stays at **7** and `RESERVED_TYPE_NAMES` stays at **7** — these are builtins and std definitions, not `Ty` variants, so **nothing is reserved and no program breaks.** If you find yourself editing either count, stop and report it.

- [ ] **Step 4: Run the seam tests**

```bash
cargo build --workspace
```

```bash
cargo test -p nova-typeck the_stream_builtins_are_std_only_and_return_status_words
```

Then the existing count assertions, which will fail if `STD_ONLY`'s declared length and its entries disagree:

```bash
cargo test --workspace --no-fail-fast
```

Expected: **the prior total + your new tests**, with no pre-existing failures. State the arithmetic.

- [ ] **Step 5: Prove seam 10 is load-bearing**

Delete one of your five `symbols()` entries, rebuild, and run a fixture that reaches that intrinsic under `nova run`. Record what the failure looks like. This is the documented blind spot and the one seam a PascalCase grep cannot find; knowing its failure mode is worth more than assuming it. Restore it afterwards and confirm `git diff` is clean.

- [ ] **Step 6: Lint and commit** — same four commands as Task 1 Step 8.

---

### Task 3: `std/io`'s surface — the traits, the streams, the wrappers

**Files:**
- Modify: `std/io/lib.nova`
- Create: `tests/runtime/io_streams.nova` + `.stdout`, `tests/runtime/io_read_to_end.nova` + `.stdout`
- Modify: `crates/nova-cli/tests/run_tests.rs`

**Interfaces:**
- Consumes from Task 2: the five Nova-visible builtins; plus the existing `fs_take_bytes()` and `fs_last_error_message()` and `io_error_kind_of(Int)`
- Produces, for Task 4: `Read`, `Write`, `Stdin`, `Stdout`, `Stderr`, `stdin()`, `stdout()`, `stderr()`

- [ ] **Step 1: Write the failing fixtures**

`tests/runtime/io_streams.nova` — writes to both streams and proves they are distinct:

```nova
async fn main() {
    let n = stdout().write(bytes_from_string("to-stdout\n")).await
    match n {
        Ok(count) => println("wrote ${count}")
        Err(e) => println("stdout err ${e.message}")
    }
    let m = stderr().write(bytes_from_string("to-stderr\n")).await
    match m {
        Ok(_) => println("stderr ok")
        Err(e) => println("stderr err ${e.message}")
    }
    match stdout().flush().await {
        Ok(_) => println("flushed")
        Err(e) => println("flush err ${e.message}")
    }
}
```

`tests/runtime/io_read_to_end.nova` — pins the default body against a fake reader, because the real streams cannot exercise it deterministically:

```nova
record Chunks { first: Bytes, second: Bytes, step: Int }

async fn chunk_read(c: Chunks, max: Int) -> Result<Bytes, IoError> {
    if c.step == 0 { return Ok(c.first) }
    if c.step == 1 { return Ok(c.second) }
    Ok(bytes_from_ints([]))
}

impl Read for Chunks {
    fn read(self, max: Int) -> Future<Result<Bytes, IoError>> { chunk_read(self, max) }
}

async fn main() { /* drive read_to_end and print the concatenation */ }
```

**`Chunks` needs mutable step state across calls and Nova records are values** — so work out how the fixture advances between chunks (a counter the fixture owns and threads through, or a different shape entirely) and **say in your report what you did.** If it turns out the default body cannot be exercised from Nova without mutation Nova does not have, **stop and report that** — it would mean `read_to_end`'s default needs pinning from Rust instead, which is a design consequence, not an implementation detail.

**A record literal in match-scrutinee position is `P0001`** — bind to a local before matching.

- [ ] **Step 2: Run them and confirm they fail**

```bash
cargo build --workspace
```

```bash
cargo test -p nova-cli --test run_tests io_streams_run
```

Expected: FAIL — the runner and the surface do not exist.

- [ ] **Step 3: Write the surface in `std/io/lib.nova`**

The traits, the three records, the three constructors, the three impls, and the private `read_all`. Use §5 of the spec verbatim for signatures. Each wrapper is straight-line:

```nova
async fn write_stdout(s: Stdout, buf: Bytes) -> Result<Int, IoError> {
    let status = io_stdout_write(buf)
    if status == 0 {
        return Ok(fs_take_bytes().byte_at(0).unwrap_or(0))
    }
    Err(IoError { kind: io_error_kind_of(status), message: fs_last_error_message() })
}
```

That count-extraction line is a **placeholder shape, not a prescription** — it depends on Task 1 Step 4's decision about how the byte count crosses the boundary. Use whatever Task 1 actually built, and if it differs from this sketch, follow Task 1 and note the difference.

Add the module doc: the `Future<T>` spelling and why; that the wrappers are straight-line and why (the corrected *within-one-task* reason from Task 1 Step 1, **not** the sibling argument); that an empty read means EOF while **a short read does not**; and that the `fs_` prefix on the two takers is historical, the slot table having been a general per-task facility since increment 3a.

Update the module's opening comment, which currently says the traits "still arrive in a later increment."

- [ ] **Step 4: Add the fixture runners**

In `run_tests.rs`, following the existing `fs_bytes_roundtrip_run` shape. The stream fixture must assert **which** stream each write landed on — a `Write for Stdout` that wrote to stderr has to fail. Feed stdin from the harness, never a terminal.

- [ ] **Step 5: Run and confirm green**

```bash
cargo test -p nova-cli --test run_tests io_streams_run
```

```bash
cargo test -p nova-cli --test run_tests io_read_to_end_run
```

Both pass, both confirmed non-zero-match. Then:

```bash
cargo test --workspace --no-fail-fast
```

State the arithmetic.

- [ ] **Step 6: Mutation-test the wrappers and the default**

At minimum: delete a wrapper's status check so it always returns `Ok` — the gap that shipped twice before, for `write_string` and again for `fs::read`/`fs::write`; make `read_all` stop on the **first** result rather than on empty; make `read_all` concatenate in reverse order; and make `read_to_end`'s EOF test `len() < max` instead of `len() == 0`, which is §6's named hazard. **Report each and whether it died**, honestly.

- [ ] **Step 7: Lint and commit** — same four commands.

---

### Task 4: The records

**Files:**
- Modify: `nova-spec/20-STDLIB.md` (§4), `docs/adr/0009-async-execution-model.md` (§1), `docs/adr/0011-io-error-kinds.md`, `CHANGELOG.md`

- [ ] **Step 1: Amend `nova-spec/20-STDLIB.md` §4 in place**

A dated note recording that `async fn` in a trait declaration is `E0900`, that the traits therefore ship as `fn … -> Future<T>`, and that `stdin`/`stdout`/`stderr` return concrete `Stdin`/`Stdout`/`Stderr` because `impl Trait` in return position does not parse (`P0001`). **Preserve the original text** — it is the record of what was specified. Follow the convention the file's existing `AMENDED` notes use; read one first.

- [ ] **Step 2: Add the executor hazard to ADR 0009 §1**

`stdin` blocks the whole executor until increment 4's poller. `read_to_end` on an interactive terminal blocks until the user sends EOF, so **a program that spawns tasks and then reads stdin stalls every other task indefinitely.** Note that this is the same class as `std/fs`'s never-suspending `async fn`s and worse in degree, because a filesystem read finishes on its own and a terminal read waits on a human. §1 is where a reader looks for this; a doc comment alone is not enough.

- [ ] **Step 3: Narrow ADR 0011**

Its deviation is now `open`/`File` **only** — the traits exist. Amend in place with a dated note in that file's own convention. **Restrict by property, not by count**, and do not write a numeral you have not measured.

- [ ] **Step 4: `CHANGELOG.md`**

Under `### Added`. **Nothing belongs under `### Changed`** — no existing behaviour moves and no name is reserved — but **verify that against the heading's own stated scope before filing rather than assuming it**, because an entry was once cross-filed under a heading scoped to a different phase and that was a review finding. Say plainly that `print`/`println`/`eprint`/`eprintln` are unchanged and why.

- [ ] **Step 5: The claim sweep**

```bash
git diff main --stat
```

Grep each changed file's **added** lines for `always`, `every`, `only`, `any`, `never`, `all`, `cannot` **and the counting words `once`, `exactly`, `unique`, `single`, `both`, `two`, `three`**. The counting half is not optional: a fix on the previous branch shipped "occurs exactly once" (it occurred four times, and the sentence making the claim was itself one of the occurrences), and no word in the shorter list appears in that sentence.

Two things the sweep structurally cannot catch, so check them by reading: a doc **quoting a literal diagnostic string**, and **a sentence this branch falsified but did not touch.** For the second, specifically re-read: `std/fs/lib.nova`'s wrapper paragraph (Task 1 fixed one instance — confirm no sibling survives); `fs.rs`'s module doc; ADR 0011's description of the boundary; ADR 0009 §1's existing footgun entries; and `std/io`'s own opening comment.

**Report the sweep's output even if it finds nothing.**

- [ ] **Step 6: Full verification and commit**

```bash
cargo build --workspace
```

```bash
cargo test --workspace --no-fail-fast
```

```bash
cargo clippy --all-targets --all-features -- -D warnings
```

```bash
cargo fmt --all --check
```

```bash
git add -A && git commit -F <message-file>
```

---

## Self-Review

**Spec coverage.** §4's spelling → Task 2 and Task 3 Step 3, recorded in Task 4 Step 1. §5's surface → Task 3 Step 3. §6's EOF rule and short-read hazard → Task 3 Steps 3 and 6. §7's five intrinsics and per-task slots → Tasks 1 and 2, with the `fs_` prefix note in Task 3 Step 3. §8's executor hazard → Task 4 Step 2. §9's tests → Task 1 Step 2, Task 3 Steps 1 and 4, and the mutation steps. §10's risks → the straight-line constraint in Global Constraints, the short-read fixture in Task 3, the prefix notes in Task 3. §11's definition of done → distributed across all four tasks' verification steps. §2 and §3's non-goals → nothing implements `File`, `OpenOptions`, a poller, the `Bytes` debt, or touches `print`.

**Placeholder scan.** No "TBD", no "add error handling", no "similar to Task N". Three steps deliberately ask for a decision plus a report rather than prescribing: how the byte count crosses the boundary (Task 1 Step 4), whether `write` loops on partial writes (same), and how the `read_to_end` fixture advances between chunks (Task 3 Step 1). Each is a real design question I could not settle without writing the code, and each says what to do if the answer is "this cannot work" — that is a reporting contract, not a placeholder.

**Type consistency.** Nova-visible builtin names (`io_stdin_read`, `io_stdout_write`, `io_stderr_write`, `io_stdout_flush`, `io_stderr_flush`) and Rust symbols (`nova_rt_io_*`) are spelled identically in every task and never confused for one another — the Nova level appears in `std/io` code, the `nova_rt_` level in `symbols()` and `RtFunc`. `Read`/`Write`/`Stdin`/`Stdout`/`Stderr`/`stdin`/`stdout`/`stderr`/`read_all` keep one spelling throughout. Counts chain: `STD_ONLY` 43 → 48; `STD_MODULES` and `RESERVED_TYPE_NAMES` both stay 7.

**One risk this plan carries deliberately.** Task 3 Step 1 may find that `read_to_end`'s default body cannot be exercised from Nova at all, because the fake reader needs state that advances across calls and Nova records are values. The plan asks for that to be reported as a design consequence rather than worked around, because the alternative — pinning the default from Rust instead — changes what §9 of the spec claims is testable, and that is the human's call.
