# `File`, `open` and `OpenOptions` Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship `nova-spec/20-STDLIB.md` §5's `open` and `File`, define the `OpenOptions` no document defines, and close ADR 0011's last remaining deviation.

**Architecture:** `File { fd: Int }` keys a thread-local table of open `std::fs::File`s in a new `crates/nova-runtime/src/file.rs`. Explicit `close` is the only release mechanism; a forgotten `File` leaks its descriptor until process exit. `close` is idempotent, and any other operation on a closed — or forged — `File` returns `IoError { kind: Other }`, because the table lookup simply misses. Payloads ride the per-task slot table `fs.rs` has owned since increment 3a.

**Tech Stack:** Rust 2021 (MSRV 1.78) across `nova-resolver`, `nova-typeck`, `nova-mir`, `nova-runtime`; Nova source in `std/fs`; fixtures under `tests/runtime` driven by `crates/nova-cli/tests/run_tests.rs`.

**Spec:** `docs/superpowers/specs/2026-08-14-file-open-and-openoptions-design.md`

## Global Constraints

- `cargo build --workspace` **before** `cargo test`. Not optional — tests link the built runtime.
- `cargo test --workspace --no-fail-fast` at default parallelism. Sum **every** `test result:` line. **Never pipe cargo output through `head` or `tail`** before summing.
- Baseline **907 passed / 0 failed / 8 ignored across 44 targets.** State the arithmetic for every task.
- **A zero-match test filter exits 0 here**, so a filter typo reads as a pass. `cargo test` rejects multiple positional filters — one name per invocation. Name the tests you ran and confirm they ran.
- **No panic may cross a generated poll boundary.** Generated code has no landing pads. `abort_with` is acceptable — it terminates without unwinding. `unwrap`, `expect`, `RefCell` borrow panics, slice indexing and fallible `format!` are **not**, in production code. Panics inside `#[cfg(test)]` are fine.
- **Every Nova wrapper is straight-line: no `.await` between an intrinsic call and its slot read.** The per-task table makes a suspension there survivable *across* tasks; it does not make it correct *within* one, because the task's own next call overwrites its own slot.
- **`RESERVED_TYPE_NAMES` and `STD_MODULES` must both stay at 7.** `File` and `OpenOptions` are `std/fs` definitions a user definition shadows (ADR 0004), not `Ty` variants. **If you find yourself editing either constant, stop and report it** — the design has been misunderstood.
- The **8 `#[ignore]`d ADR-0010 GC tests** stay ignored and untouched.
- `cargo clippy --all-targets --all-features -- -D warnings` and `cargo fmt --all --check` clean. **No `reason = "…"` in any lint attribute** (MSRV 1.78; that CI job is known-vacuous, so it would ship as a user build failure).
- **No line-number citation in a doc comment that points into its own file, and none into another file this branch edits.** Name the symbol. This project shipped the same-file form twice and the cross-file form three times, every instance stale on arrival or falsified by its own branch.
- Every fixture path unique per process.
- Commit messages to a **UTF-8 file**, applied with `git commit -F` — **never a heredoc.** Body ends `Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>`. **Never push.**
- Branch: `file-open-openoptions`, already created; spec committed at `5ce065d`.

## The ten seams a new intrinsic touches

Traced through `IoStdoutFlush`, which increment 3b added. These are **edit sites**; a grep returns more hits because several files also mention the variant in doc comments. Miss one and it fails at a different stage each time, so work the list:

| # | File | What |
|---|---|---|
| 1 | `nova-resolver/src/lib.rs` | `Builtin` enum variant |
| 2 | `nova-resolver/src/lib.rs` | `Builtin::X => "file_open"` — the **Nova-visible** name |
| 3 | `nova-resolver/src/lib.rs` | `Builtin::STD_ONLY` array entry |
| 4 | `nova-typeck/src/check.rs` | the `\|`-chain in `check_builtin_call` |
| 5 | `nova-typeck/src/check.rs` | `builtin_signature` — arity and types |
| 6 | `nova-mir/src/lib.rs` | `RtFunc` enum variant |
| 7 | `nova-mir/src/lib.rs` | `RtFunc::X => "nova_rt_file_open"` — the **Rust symbol** |
| 8 | `nova-mir/src/lib.rs` | `signature()` → `MirTy`s |
| 9 | `nova-mir/src/lower.rs` | `Builtin::X => Lowering::Runtime(RtFunc::X)` |
| 10 | `nova-runtime/src/lib.rs` | **`symbols()`** — keyed by the lowercase symbol string, therefore **invisible to a PascalCase grep**. A missing entry compiles, links, and fails only when compiled code actually calls it. The `nova_rt_task_*` entries shipped in exactly that state once, which is why `every_rt_func_symbol_is_registered_with_the_jit` exists. |

**Locate each by the neighbouring `IoStdoutFlush`/`FsRead` entry, never by line number** — the numbers drift as you insert.

## File Structure

| File | Responsibility | Change |
|---|---|---|
| `crates/nova-runtime/src/file.rs` | **New.** The open-file table and five intrinsics | Create |
| `crates/nova-runtime/src/lib.rs` | Runtime root; `symbols()` | Modify: `mod file;`, five `symbols()` entries |
| `crates/nova-resolver/src/lib.rs` | Builtin names, `STD_ONLY` | Modify: seams 1–3 |
| `crates/nova-typeck/src/check.rs` | Builtin typing | Modify: seams 4–5 |
| `crates/nova-mir/src/lib.rs`, `lower.rs` | `RtFunc`, symbols, lowering | Modify: seams 6–9 |
| `std/fs/lib.nova` | `OpenOptions`, `File`, `open`, `close`, the two impls | Modify |
| `tests/runtime/file_roundtrip.nova` + `.stdout` | Round-trip fixture | Create |
| `tests/runtime/file_lifetime.nova` + `.stdout` | Close semantics fixture | Create |
| `tests/runtime/file_errors.nova` + `.stdout` | Error-path fixture | Create |
| `crates/nova-cli/tests/run_tests.rs` | Fixture harness | Modify |
| `nova-spec/20-STDLIB.md`, `docs/adr/0011-*`, `docs/adr/0009-*`, `CHANGELOG.md` | Records | Modify: Task 4 |

**`file.rs` needs no new visibility from `fs.rs`.** Verified: `io.rs` uses exactly `use crate::fs::{fail, stash, Slot, OK}`, and `file.rs` needs the same four. `take` stays private — only the Nova wrapper takes, via the `fs_take_bytes` builtin.

---

### Task 1: The open-file table and five intrinsics

**Files:**
- Create: `crates/nova-runtime/src/file.rs`
- Modify: `crates/nova-runtime/src/lib.rs` (`mod file;`, five `symbols()` entries)
- Test: `crates/nova-runtime/src/file.rs`'s own `#[cfg(test)] mod tests`

**Interfaces:**
- Consumes: `crate::fs::{fail, stash, Slot, OK}`; `crate::task::abort_with`; `crate::NovaStr`; `crate::bytes::{gc_bytes, as_bytes}`; `crate::as_str`
- Produces, for Task 2: `nova_rt_file_open(path: *const NovaStr, read: i8, write: i8, append: i8, truncate: i8, create: i8, create_new: i8) -> i64`; `nova_rt_file_close(fd: i64) -> i64`; `nova_rt_file_read(fd: i64, max: i64) -> i64`; `nova_rt_file_write(fd: i64, buf: *const NovaStr) -> i64`; `nova_rt_file_flush(fd: i64) -> i64`

- [ ] **Step 1: Write the failing round-trip test**

```rust
/// A file opened for writing, written, closed, reopened and read gives back
/// what was written.
///
/// The whole point of this module is that a handle survives between intrinsic
/// calls, which nothing else in this runtime does — so the first test drives
/// the full open/write/close/open/read/close sequence rather than one call.
#[test]
fn a_file_round_trips_through_the_handle_table() {
    let path = unique_temp_path("round-trip");
    let p = crate::gc_str(&path);

    let status = unsafe { nova_rt_file_open(p, 0, 1, 0, 1, 1, 0) };
    assert_eq!(status, OK, "open for writing must succeed");
    let fd = take_fd();

    let payload = crate::bytes::gc_bytes(b"handle-table");
    assert_eq!(unsafe { nova_rt_file_write(fd, payload) }, OK);
    assert_eq!(unsafe { nova_rt_file_close(fd) }, OK);

    let status = unsafe { nova_rt_file_open(p, 1, 0, 0, 0, 0, 0) };
    assert_eq!(status, OK, "open for reading must succeed");
    let fd = take_fd();
    assert_eq!(unsafe { nova_rt_file_read(fd, 64) }, OK);
    assert_eq!(take_bytes(), b"handle-table", "the bytes must survive the round trip");
    assert_eq!(unsafe { nova_rt_file_close(fd) }, OK);

    let _ = std::fs::remove_file(&path);
}
```

Write the smallest `#[cfg(test)]` helpers that make this express its intent — `unique_temp_path`, `take_fd`, `take_bytes`. **Gate them `#[cfg(test)]` with no platform gate**: a `pub(crate)` test helper in this crate previously needed a `#[cfg(windows)]` gate because its only callers had one, and that mismatch shipped as a Linux-only clippy failure.

- [ ] **Step 2: Run it and confirm it fails**

```bash
cargo build --workspace
```

```bash
cargo test -p nova-runtime a_file_round_trips_through_the_handle_table
```

Expected: FAIL to compile — nothing exists yet.

- [ ] **Step 3: Write the table**

```rust
//! Open files, keyed by descriptor.
//!
//! **This is the first thing in this runtime that holds an OS resource across
//! more than one intrinsic call.** `fs`'s functions open, act and close inside
//! a single call, and `io`'s streams are process-global and never closed — so
//! neither needed a table. A `File` does.
//!
//! Nova has no destructors, so `std/fs`'s `close` is the only release
//! mechanism and a forgotten `File` leaks its descriptor until the process
//! exits. The design spec records why close-on-collect is foreclosed rather
//! than merely unbuilt.
//!
//! Absence from this table *is* closedness: a `read` on a closed fd, a stale
//! fd, or an fd a Nova program forged by hand all miss the lookup and become
//! one `IoError`. That is deliberate — record fields are not privacy-enforced,
//! so a forged `File { fd: 99 }` is constructible and must be safe.

use crate::fs::{fail, stash, Slot, OK};
use crate::NovaStr;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::io::{Read as _, Write as _};

thread_local! {
    /// Open files by descriptor. `thread_local!` for the reason `task.rs`'s
    /// module doc gives for `TASKS`: the GC's roots are per-thread, so a
    /// second thread running Nova code would free objects the first holds.
    static FILES: RefCell<HashMap<i64, std::fs::File>> = RefCell::new(HashMap::new());
    /// Never reused, so a stale fd stays stale rather than aliasing a
    /// different file later. Starts at 1 so 0 is available as an obviously
    /// invalid value in diagnostics.
    static NEXT_FD: Cell<i64> = const { Cell::new(1) };
}
```

Add `mod file;` to `lib.rs`.

- [ ] **Step 4: Write the access helper, panic-free**

Every table access must be fallible-borrow with an `abort_with` backstop, exactly as `fs.rs`'s `with_slot` is, because this runs inside a generated poll boundary with no landing pads:

```rust
/// Run `f` against the file behind `fd`, or report a closed-file error.
///
/// `try_borrow_mut` rather than `borrow_mut`: a `RefCell` panic here would
/// cross a generated poll boundary. The `None` arm is the closed/stale/forged
/// case and is an ordinary error, not an abort — `std/fs`'s `close` cannot
/// consume its receiver, because Nova has no move checking, so use-after-close
/// is a mistake the language invites rather than an exotic one.
fn with_fd<R>(fd: i64, f: impl FnOnce(&mut std::fs::File) -> R) -> Option<R> {
    FILES.with(|files| {
        let Ok(mut files) = files.try_borrow_mut() else {
            crate::task::abort_with("nova_rt_file: handle table is already borrowed")
        };
        files.get_mut(&fd).map(f)
    })
}
```

**Named `with_fd`, not `with_file`, on purpose.** The design spec's §6 declines a scope-based Nova API
called `with_file(path, options, body)`; reusing that name for an unrelated Rust helper would leave a
reader who has read both documents unsure which is which.

**Decide and report:** how the closed-file case reaches Nova as `IoError { kind: Other }`. The status word carries an `IoErrorKind` code, so `OTHER` is available — but `fs.rs`'s `fail` also stashes a message from a `std::io::Error`, and there is no `std::io::Error` here. Say what you did about the message.

- [ ] **Step 5: Write the five intrinsics**

`open` builds a `std::fs::OpenOptions` from the six flags, inserts on success, and **stashes the new fd as a payload** — the status word is already carrying the error kind, so the fd cannot also travel in it. **Reuse increment 3b's 8-little-endian-byte encoding in `Slot::Buffer`**, the same one `nova_rt_io_stdout_write` uses for its byte count; read that function and match it rather than inventing a second encoding.

`read` reads up to `max` into a truncated buffer and stashes the bytes; **an empty result means EOF and a short read does not**, per `std/io`'s contract. `write` writes once, possibly partially, and stashes the count. `close` removes from the table and returns `OK`; **removing an fd that is absent is also `OK`**, which is what makes `close` idempotent. `flush` flushes.

**Guard `max`** the way `nova_rt_io_stdin_read` does — a negative value aborts — and note in your report that a large `max` still allocates eagerly, which the design spec §7 records as a known asymmetry rather than a defect.

- [ ] **Step 6: Register all five in `symbols()`**

Beside the `nova_rt_io_*` entries in `crates/nova-runtime/src/lib.rs`. **This table is keyed by the lowercase symbol string and is invisible to a PascalCase grep.** Count them yourself after writing them.

- [ ] **Step 7: Run the tests, then the whole suite**

```bash
cargo build --workspace
```

```bash
cargo test -p nova-runtime file::
```

Expected: your tests pass with a non-zero match count.

```bash
cargo test --workspace --no-fail-fast
```

Expected **907 + (tests you added)**. State the arithmetic.

- [ ] **Step 8: Mutation-test the table**

At minimum: make `close` leave the entry in place; make `NEXT_FD` return a constant so two opens collide; make `with_fd`'s `None` arm return `OK`; make `read` skip the truncate so a short read pads with zeroes. **Report each and whether it died.** A survivor reported plainly is a useful result — say what a test that killed it would have to assert, and do not add a test that only appears to pin something.

- [ ] **Step 9: Lint and commit**

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

**Files:** `crates/nova-resolver/src/lib.rs` (seams 1–3), `crates/nova-typeck/src/check.rs` (seams 4–5), `crates/nova-mir/src/lib.rs` (seams 6–8), `crates/nova-mir/src/lower.rs` (seam 9)

**Interfaces:**
- Consumes from Task 1: the five `nova_rt_file_*` Rust symbols
- Produces, for Task 3, the Nova-visible builtins:

| Nova name | Signature |
|---|---|
| `file_open` | `(String, Bool, Bool, Bool, Bool, Bool, Bool) -> Int` |
| `file_close` | `(Int) -> Int` |
| `file_read` | `(Int, Int) -> Int` |
| `file_write` | `(Int, Bytes) -> Int` |
| `file_flush` | `(Int) -> Int` |

All return a **status word**; payloads come back through the existing `fs_take_bytes`, messages through `fs_last_error_message`.

- [ ] **Step 1: Write the failing typeck test**

```rust
/// The five file builtins are `STD_ONLY` and return status words.
///
/// `STD_ONLY` is seeded into every std module's scope rather than per-module
/// (see `Builtin::STD_ONLY`'s own doc), which is why these carry a `file_`
/// prefix rather than relying on scope to disambiguate them.
#[test]
fn the_file_builtins_are_std_only_and_return_status_words() {
    for name in [
        "file_open", "file_close", "file_read", "file_write", "file_flush",
    ] {
        assert!(
            nova_resolver::Builtin::STD_ONLY.iter().any(|b| b.name() == name),
            "{name} must be an STD_ONLY builtin"
        );
    }
}
```

**Read `Builtin`'s real accessor before writing this** — if it is not `name()`, use the real one and say so.

- [ ] **Step 2: Run it and confirm it fails**

```bash
cargo test -p nova-typeck the_file_builtins_are_std_only_and_return_status_words
```

Expected: FAIL — the builtins do not exist.

- [ ] **Step 3: Work seams 1–9**

Copy the shape of the nearest analogue per seam: `IoStdoutWrite` for `file_write` (value in, status out, payload in a slot), `IoStdinRead` for `file_read`, `IoStdoutFlush` for `file_close`/`file_flush`. `file_open` has no close analogue — **seven parameters is the widest builtin in the table**, so check `builtin_signature`'s and `RtFunc::signature`'s shapes tolerate it and report if anything caps arity.

**`STD_ONLY` goes 48 → 53.** `STD_MODULES` and `RESERVED_TYPE_NAMES` **both stay at 7.**

- [ ] **Step 4: Verify the seams**

```bash
cargo build --workspace
```

```bash
cargo test -p nova-typeck the_file_builtins_are_std_only_and_return_status_words
```

Then the whole suite; state the arithmetic.

- [ ] **Step 5: Confirm seam 10 is load-bearing**

Delete one of Task 1's `symbols()` entries, rebuild, and run a fixture that reaches that intrinsic under `nova run`. **Record the exact failure text.** Note what increment 3b measured: a program that never *calls* the missing intrinsic runs fine, because `finalize_definitions` resolves only the symbols an actual call references — so the failure is invisible to any fixture that does not exercise it. Restore and confirm `git diff` is clean.

- [ ] **Step 6: Lint and commit** — same four commands as Task 1 Step 9.

---

### Task 3: `std/fs`'s surface and the fixtures

**Files:**
- Modify: `std/fs/lib.nova`
- Create: `tests/runtime/file_roundtrip.nova` + `.stdout`, `tests/runtime/file_lifetime.nova` + `.stdout`, `tests/runtime/file_errors.nova` + `.stdout`
- Modify: `crates/nova-cli/tests/run_tests.rs`

**Interfaces:**
- Consumes from Task 2: the five Nova-visible builtins; plus existing `fs_take_bytes()`, `fs_last_error_message()`, `io_error_kind_of(Int)`, and `Read`/`Write` from `std/io`
- Produces, for Task 4: `OpenOptions`, `File`, `open`, `File::close`, `impl Read for File`, `impl Write for File`

- [ ] **Step 1: Write the surface**

Use spec §2 verbatim for signatures. Three things to get right, all measured:

- **`close` is inherent, so it is a plain `async fn`** — the `Future<T>` spelling is forced only on *trait* methods, because `async fn` in a trait declaration is `E0900`.
- **No chainable builder.** A receiver-mutating method cannot be called on a temporary (`E0060`), so `OpenOptions::reading().with_write()` does not compile. Ship `impl Default`, three named constructors, and let exotic combinations use `let mut o = OpenOptions::default()` plus field assignment.
- **Wrappers stay straight-line.** Decode the fd from the 8-byte payload the same way `std/io`'s `write_stdout` decodes its count — read that function and reuse its decoder rather than writing a second one.

`writing()` is write + create + truncate; `appending()` is append + create; `reading()` is read only.

- [ ] **Step 2: Write the three failing fixtures**

`file_roundtrip.nova` — open for writing, write, close, reopen for reading, `read_to_end`, close, print the text. Exercises both trait impls and `read_to_end`'s default from increment 3b.

`file_lifetime.nova` — the increment's core behaviour:

```nova
async fn main() {
    let f = open_or_die("...")
    match f.close().await { Ok(_) => println("close: ok") Err(e) => println("close: err") }
    match f.close().await { Ok(_) => println("close again: ok") Err(e) => println("close again: err") }
    let r = f.read(16).await
    match r { Ok(_) => println("read after close: unexpectedly ok") Err(e) => println("read after close: ${e.message}") }
    let forged = File { fd: 9999 }
    let g = forged.read(16).await
    match g { Ok(_) => println("forged: unexpectedly ok") Err(e) => println("forged: err") }
}
```

**A record literal in match-scrutinee position is `P0001`** — bind to a local before matching, as this sketch does.

`file_errors.nova` — the portable failure modes: `create_new` on an existing path is `AlreadyExists`; opening a **directory** for reading fails; a path under a **missing parent** fails. Assert on `kind`, and **normalise or omit `message`** — it is platform text, the treatment `fs_io_types.nova` already applies.

- [ ] **Step 3: Run them and confirm they fail**

```bash
cargo build --workspace
```

```bash
cargo test -p nova-cli --test run_tests file_lifetime_run
```

Expected: FAIL — the runner and surface do not exist.

- [ ] **Step 4: Add the fixture runners**

Following `fs_bytes_roundtrip_run`'s shape, each with a `unique_temp_dir` label. Use all three of `TMPDIR`/`TMP`/`TEMP` — every `fs_*` fixture that touches a real path already does.

- [ ] **Step 5: Run and confirm green, then the suite**

Each fixture by name, confirming a non-zero match, then the full suite with the arithmetic stated.

- [ ] **Step 6: Mutation-test the wrappers**

At minimum: delete a wrapper's status check so it always returns `Ok` — **that exact gap shipped twice before**, for `write_string` and again for `fs::read`/`fs::write`; make `close` non-idempotent so a second call errors; make the fd decoder read one byte instead of eight, which is invisible until the 256th file opened in a process; ignore `create_new` when building the `std::fs::OpenOptions`. **Report each honestly.**

- [ ] **Step 7: Lint and commit.**

---

### Task 4: The records

**Files:** `nova-spec/20-STDLIB.md`, `docs/adr/0011-io-error-kinds.md`, `docs/adr/0009-async-execution-model.md`, `CHANGELOG.md`

- [ ] **Step 1: Amend `nova-spec/20-STDLIB.md` §5 in place**

A dated note **defining `OpenOptions`** — the document references it and has never defined it — and recording that `File` carries an `Int` descriptor with explicit `close`. **Preserve the original text.** Read an existing `AMENDED` note in that file first and match its shape.

- [ ] **Step 2: Close ADR 0011's deviation**

`open` and `File` were its last two items, so the deviation is now **closed, not narrowed**. Amend in place with a dated note in that file's own convention. **Restrict by property, not by count**, and write no numeral you have not measured.

- [ ] **Step 3: Record the foreclosed backstop**

This is the step most likely to be skipped and the one that matters most in a year. The collector's sweep calls `task::forget_freed_state` at the moment an address dies — a hook whose shape would fit closing a file. **`File`'s `Int` representation makes using it impossible, not merely unused**, and collection does not run at all off Windows (`stack_base()` returns `None` there). **Write that down** as an ADR 0009 amendment or a short new ADR — your call, but say which you chose and why — so the next person who finds that hook does not read its absence as an oversight.

- [ ] **Step 4: `CHANGELOG.md`** under `### Added`. Nothing should belong under `### Changed`, but **read that heading's own stated scope before filing** — an entry was once cross-filed under a heading scoped to a different phase, and that was a review finding.

- [ ] **Step 5: The two clauses increment 3b left here**

`nova-spec/20-STDLIB.md` §4 never states a short write is legal, though `-> Result<Int, IoError>` implies a count. And `nova_rt_io_stdin_read` allocates the caller's `max` eagerly, so a generous *ceiling* is charged in full — one sentence each, in the docs a caller reads.

- [ ] **Step 6: The claim sweep**

```bash
git diff main --stat
```

Grep each changed file's **added** lines for `always`, `every`, `only`, `any`, `never`, `all`, `cannot` **and the counting words `once`, `exactly`, `unique`, `single`, `both`, `two`, `three`, `four`, `five`, `six`, `eight`, `ten`**.

The counting half is not optional and this project has failed it repeatedly: a guard comment claimed a literal "recurs once more" when it occurred four times, *undercounting because the sentence stating the count was itself an occurrence*; and **two successive specs claimed a false occurrence count for `OpenOptions` specifically** — the identifier this increment is about. **Prefer a property to a figure**, and never write a numeral you have not just measured.

Two things the sweep cannot catch, so check them by reading: a doc **quoting a literal diagnostic string**, and **a sentence this branch falsified but did not touch**. For the second, re-read: `std/fs/lib.nova`'s module doc and its wrapper paragraph; `fs.rs`'s taker docs; `std/io/lib.nova`'s module doc; ADR 0011's Consequences; and ADR 0009 §1's footgun list. **Report the sweep's output even if it finds nothing.**

- [ ] **Step 7: Full verification and commit** — build, suite with arithmetic, clippy, fmt, `git commit -F`.

---

## Self-Review

**Spec coverage.** §2's surface → Task 3 Step 1. §3's table, thread-locality and panic-freedom → Task 1 Steps 3–4. §3's idempotent close and use-after-close → Task 1 Step 5, pinned by Task 3's `file_lifetime` fixture. §3's foreclosed backstop → Task 4 Step 3. §4's boundary and ten seams → Tasks 1–2. §5's tests → Task 1 Step 1, Task 3 Step 2, and both mutation steps. §6's alternatives → no task; they are declined, not built. §7's two clauses → Task 4 Step 5. §8's definition of done → distributed across all four tasks.

**Placeholder scan.** No "TBD", no "add error handling", no "similar to Task N". Two steps ask for a decision plus a report rather than prescribing: how the closed-file case produces a message with no `std::io::Error` to draw one from (Task 1 Step 4), and whether the foreclosed backstop is recorded as an ADR amendment or a new ADR (Task 4 Step 3). Both are judgment calls I cannot settle without the code in front of me, and both say what to report.

**Type consistency.** Nova-visible names (`file_open`, `file_close`, `file_read`, `file_write`, `file_flush`) and Rust symbols (`nova_rt_file_*`) are spelled identically throughout and never interchanged — the Nova level appears only in `std/fs` code, the `nova_rt_` level only in `symbols()` and `RtFunc`. `OpenOptions`/`File`/`open`/`close`/`with_fd`/`FILES`/`NEXT_FD` keep one spelling, and the Rust helper is `with_fd` precisely so it cannot be mistaken for spec §6's declined `with_file` API. `STD_ONLY` 48 → 53; `STD_MODULES` and `RESERVED_TYPE_NAMES` both 7 throughout.

**One risk carried deliberately.** Task 1's `open` takes seven parameters, wider than any existing builtin. If `builtin_signature` or `RtFunc::signature` turns out to cap arity, Task 2 Step 3 says to report it rather than work around it — because the fallback (packing the flags into one `Int` bitmask) changes the Nova surface, and that is a design decision, not an implementation detail.
