# `std/fs` on Strings Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Give Nova eight `async` filesystem functions plus `DirEntry`, an `IoError` surface, and the missing `eprint`/`eprintln`, using only types the language already has.

**Architecture:** Twelve new `STD_ONLY` intrinsics cross the boundary carrying only `Int`, `Bool`, `String` and one `[String]`. **The status code returned by each operation *is* the error kind**, so there is no separate kind fetch, and **no Nova aggregate layout enters Rust** — `std/fs`'s Nova wrappers build every `Result` and `IoError` themselves. Payload and message travel in GC-rooted thread-local slots.

**Tech Stack:** Rust (crates `nova-runtime`, `nova-mir`, `nova-resolver`, `nova-typeck`), Nova (`std/io/lib.nova`, `std/fs/lib.nova`), Cranelift JIT + textual LLVM backends.

**Spec:** `docs/superpowers/specs/2026-08-11-std-fs-strings-design.md` (`4b78aa3`).

## Global Constraints

- **Base:** `main` at `4b78aa3`. Branch for this work; do **not** push. Merges are `--ff-only`; history is strictly linear with 0 merge commits.
- **Baseline:** 847 passed, 0 failed, 8 ignored, 44 targets.
- **`cargo build --workspace` before `cargo test`** — `cargo test` does not regenerate `nova-runtime`'s staticlib, which `nova build` links against. Skipping it produces ~25 MSVC "unresolved external symbol" failures that read like codegen bugs. **A stale test binary has produced false passes in this project** — if a result surprises you, check you are running freshly built code.
- **`cargo test --workspace --no-fail-fast`** at default parallelism. Sum by listing every `test result:` line; **never pipe cargo output through `head` or `tail` before summing.**
- The **8 `#[ignore]`d ADR-0010 conservative-scan GC tests** stay ignored and untouched.
- `cargo clippy --workspace --all-targets -- -D warnings` and `cargo fmt --all --check` clean. **No `reason = "…"` in any lint attribute** — MSRV is 1.78 and the MSRV CI job is known vacuous.
- **NO PANIC MAY CROSS A GENERATED POLL BOUNDARY.** Every `std/fs` intrinsic is called from inside an `async fn`'s generated `$poll`, which has no landing pads. So: **no `RefCell` in any slot these intrinsics touch, no `unwrap`, no `expect`, no indexing that can go out of range, no `format!` on a fallible path.** This is why the slots in Task 2 are `Cell<usize>` holding GC-rooted pointers.
- **Assert ordering and completion, never elapsed duration.** Eight tests are already ignored for flakiness.
- **Every fixture uses a unique temp path.** This repo carries a latent race from `write_test_project` building a run-invariant temp path — do not copy that pattern.
- Every commit body ends with `Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>`. **Write commit messages to a UTF-8 file and use `git commit -F`** — a heredoc produced `Ā§` mojibake on an earlier branch.

---

## File Structure

| File | Responsibility |
|---|---|
| `std/io/lib.nova` | Create. `IoError`, `IoErrorKind`, and the `Int` → `IoErrorKind` mapping every `std/fs` wrapper calls. Nothing else — §4's traits arrive here in a later increment. |
| `std/fs/lib.nova` | Create. The eight public `async fn`s and `DirEntry`. Each is a thin wrapper: call the intrinsic, branch on the status, build the `Result`. |
| `crates/nova-runtime/src/fs.rs` | Create. All twelve intrinsics, the status mapping, and the three GC-rooted slots. A new module rather than more of `lib.rs`, which is already large and unrelated. |
| `crates/nova-runtime/src/lib.rs` | Modify. `pub mod fs;`, `nova_rt_eprint`/`nova_rt_eprintln`, and JIT symbol registration for all fourteen new symbols. |
| `crates/nova-resolver/src/lib.rs` | Modify. `Builtin` variants, names, `GLOBAL` 3 → 5, `STD_ONLY` 17 → 29, `STD_MODULES` 4 → 6. |
| `crates/nova-typeck/src/check.rs` | Modify. Builtin signatures and the arms that pin them. |
| `crates/nova-mir/src/lib.rs` | Modify. `RtFunc` variants, symbol names, signatures. |
| `crates/nova-mir/src/lower.rs` | Modify. `Builtin` → `Lowering::Runtime(RtFunc)` mappings. |
| `tests/runtime/fs_*.nova` + `.stdout` | Create. End-to-end fixtures, one per behaviour and one per error kind. |
| `docs/adr/0011-io-error-kinds.md` | Create. The two spec deviations. |
| `nova-spec/20-STDLIB.md` | Modify. §4's `IoErrorKind` amended. |
| `CHANGELOG.md` | Modify. Under `### Added`. |

**Adding a builtin touches about eleven sites, and the list above is not the authority.** Before writing seam code, run the grep named in each task and add a parallel arm at **every** hit. A list in a plan goes stale; this project has shipped a miscompile from two lookup sites drifting apart.

---

### Task 1: `std/io` error types, and `eprint`/`eprintln`

No intrinsics for filesystem work yet — this task is the two registrations everything else needs, and it is independently testable.

**Files:**
- Create: `std/io/lib.nova`
- Modify: `crates/nova-runtime/src/lib.rs`, `crates/nova-resolver/src/lib.rs`, `crates/nova-typeck/src/check.rs`, `crates/nova-mir/src/lib.rs`, `crates/nova-mir/src/lower.rs`
- Test: `tests/runtime/fs_io_types.nova` + `.stdout`, `tests/runtime/eprint_family.nova` + `.stdout`, and `crates/nova-cli/tests/run_tests.rs`

**Interfaces:**
- Consumes: nothing from earlier tasks.
- Produces: Nova `IoError { kind: IoErrorKind, message: String }`; `IoErrorKind` with variants `NotFound`, `PermissionDenied`, `AlreadyExists`, `InvalidData`, `Interrupted`, `TimedOut`, `ConnectionRefused`, `Other`; `pub fn io_error_kind_of(code: Int) -> IoErrorKind`. Tasks 2–4 call `io_error_kind_of` and construct `IoError`. Also `eprint`/`eprintln` builtins.

- [ ] **Step 1: Write the failing end-to-end test for the error types**

`tests/runtime/fs_io_types.nova`:

```nova
fn describe(e: IoError) -> String {
    match e.kind {
        NotFound => "not-found"
        PermissionDenied => "denied"
        AlreadyExists => "exists"
        InvalidData => "invalid"
        Interrupted => "interrupted"
        TimedOut => "timeout"
        ConnectionRefused => "refused"
        Other => "other"
    }
}

fn main() {
    let e = IoError { kind: io_error_kind_of(1), message: "m" }
    println(describe(e))
    println(describe(IoError { kind: io_error_kind_of(4), message: "m" }))
    println(describe(IoError { kind: io_error_kind_of(99), message: "m" }))
}
```

`tests/runtime/fs_io_types.stdout`:

```
not-found
invalid
other
```

**Why `99`:** an unmapped code must fall to `Other` rather than doing something undefined, and that arm is otherwise unreachable from Rust.

- [ ] **Step 2: Write the failing end-to-end test for `eprint`/`eprintln`**

`tests/runtime/eprint_family.nova`:

```nova
fn main() {
    println("to-stdout")
    eprint("a")
    eprintln("b")
}
```

`tests/runtime/eprint_family.stdout`:

```
to-stdout
```

Add a case in `crates/nova-cli/tests/run_tests.rs` asserting **stdout is exactly `to-stdout`** and **stderr is exactly `ab`** — the split is the whole point, so a version that wrote to stdout must fail. Follow how the existing `tests/runtime/*.nova` gates are registered in that file.

- [ ] **Step 3: Run both to confirm they fail**

```bash
cargo build --workspace && cargo test -p nova-cli fs_io_types -- --nocapture
```

Expected: FAIL — `E0001 cannot find type 'IoError'`. Confirm the run reports a non-zero test count; **a zero-match filter exits 0 in this project and proves nothing.**

- [ ] **Step 4: Create `std/io/lib.nova`**

```nova
// Nova standard library — I/O error types.
//
// Compiled as an implicit module and glob-imported into every user module, so
// these names need no `import`. A user definition of the same name shadows the
// one here (see docs/adr/0004-stdlib-compile-model.md).
//
// This module holds only the error surface. `nova-spec/20-STDLIB.md` §4 also
// specifies `Read`/`Write` traits and `stdin`/`stdout`/`stderr`; those need a
// byte type and a settled buffer signature, and arrive in a later increment.

// Why an operation failed. `ConnectionRefused` is unused by `std/fs` and is
// carried so §4's network I/O needs no further addition later.
//
// `AlreadyExists` and `InvalidData` are additions to the list in
// `nova-spec/20-STDLIB.md` §4; see docs/adr/0011-io-error-kinds.md.
pub type IoErrorKind =
    | NotFound
    | PermissionDenied
    | AlreadyExists
    | InvalidData
    | Interrupted
    | TimedOut
    | ConnectionRefused
    | Other

// A failed I/O operation: a kind to branch on, and the operating system's own
// message for diagnosis.
//
// `message` is platform-specific text, so a test asserting output should match
// on `kind` and normalise `message` -- the same treatment
// `tests/runtime/nova_test.stdout` already applies to a Windows NTSTATUS.
pub record IoError {
    pub kind: IoErrorKind
    pub message: String
}

// Map a runtime status code to its kind.
//
// **This function is one half of a wire contract.** The other half is
// `status_of` in `crates/nova-runtime/src/fs.rs`, which produces these codes
// from `std::io::ErrorKind`. The two are independent copies of one numbering,
// which is the shape that has already produced a miscompile in this project, so
// they are pinned together by a fixture per kind rather than by a comment
// asking a reader to keep them in step.
//
// An unrecognised code is `Other` rather than a panic: a status this function
// does not know is a runtime bug, and mapping it to `Other` keeps a diagnosable
// error flowing instead of ending the process.
pub fn io_error_kind_of(code: Int) -> IoErrorKind {
    if code == 1 { return NotFound }
    if code == 2 { return PermissionDenied }
    if code == 3 { return AlreadyExists }
    if code == 4 { return InvalidData }
    if code == 5 { return Interrupted }
    if code == 6 { return TimedOut }
    if code == 7 { return ConnectionRefused }
    Other
}
```

- [ ] **Step 5: Register `std/io` as a `STD_MODULES` entry**

In `crates/nova-resolver/src/lib.rs`, add `("$std.io", include_str!("../../../std/io/lib.nova"))` and bump the array's length annotation from `4` to `5`. Add it **after** `$std.core`, since `IoError` uses no other std module and `std/fs` (Task 2) will need `std/io`'s names.

- [ ] **Step 6: Add the `eprint`/`eprintln` builtins**

Run the grep that is the authority for the seams:

```bash
grep -rn 'Println' --include=*.rs crates/
```

`Println` is a `Builtin::GLOBAL` member — the same class `eprint`/`eprintln` belong in, because their siblings are global. Add a parallel `EPrint` / `EPrintln` arm at every hit, and bump `GLOBAL` from `[Builtin; 3]` to `[Builtin; 5]`. Both take `(vec![Ty::String], Ty::Unit)` in typeck and `(vec![MirTy::Ptr], MirTy::Unit)` in MIR, matching `Println`.

- [ ] **Step 7: Implement the two runtime functions**

In `crates/nova-runtime/src/lib.rs`, beside `nova_rt_println`:

```rust
/// Write `s` to stderr with no trailing newline.
///
/// Mirrors [`nova_rt_print`]'s explicit lock-and-flush rather than
/// `nova_rt_println`'s `println!`: without a newline there is nothing to
/// trigger a line-buffer flush, so the write must be flushed here or output can
/// arrive out of order relative to stdout.
///
/// # Safety
/// `s` must point to a live `NovaStr`.
#[no_mangle]
pub unsafe extern "C" fn nova_rt_eprint(s: *const NovaStr) {
    use std::io::Write;
    let err = std::io::stderr();
    let mut lock = err.lock();
    // SAFETY: forwarding this function's own contract.
    let _ = lock.write_all(unsafe { as_str(s) }.as_bytes());
    let _ = lock.flush();
}

/// Write `s` to stderr followed by a newline.
///
/// # Safety
/// `s` must point to a live `NovaStr`.
#[no_mangle]
pub unsafe extern "C" fn nova_rt_eprintln(s: *const NovaStr) {
    // SAFETY: forwarding this function's own contract.
    eprintln!("{}", unsafe { as_str(s) });
}
```

Register both in the JIT symbol table alongside `nova_rt_println`.

- [ ] **Step 8: Run both tests to confirm they pass**

```bash
cargo build --workspace && cargo test -p nova-cli fs_io_types -- --nocapture && cargo test -p nova-cli eprint_family -- --nocapture
```

Expected: PASS, each reporting a non-zero test count.

- [ ] **Step 9: Prove the `eprint` test discriminates**

Change `nova_rt_eprint` to write to `std::io::stdout()`, rebuild, re-run `eprint_family`. Expected: FAIL, because stdout now contains `to-stdout` plus `a`. **Then revert and rebuild** — a `--workspace` run with a mutation applied poisons `nova.exe` for later probes. Paste the failure into your report.

- [ ] **Step 10: Full verification**

```bash
cargo build --workspace && cargo test --workspace --no-fail-fast > /tmp/t.log 2>&1; echo "exit $?"; grep 'test result:' /tmp/t.log
```

Expected: every target `ok`; 847 + the new tests, 0 failed, 8 ignored. Then `cargo clippy --workspace --all-targets -- -D warnings` and `cargo fmt --all --check`.

- [ ] **Step 11: Commit**

Subject: `feat(std): IoError types and the eprint family`

---

### Task 2: The boundary, `read_to_string`, and `write_string`

This task establishes the protocol every later operation reuses. Get it right here and Tasks 3–4 are mechanical.

**Files:**
- Create: `crates/nova-runtime/src/fs.rs`, `std/fs/lib.nova`
- Modify: `crates/nova-runtime/src/lib.rs`, `crates/nova-resolver/src/lib.rs`, `crates/nova-typeck/src/check.rs`, `crates/nova-mir/src/lib.rs`, `crates/nova-mir/src/lower.rs`
- Test: `crates/nova-runtime/src/fs.rs` (its own `mod tests`), `tests/runtime/fs_roundtrip.nova` + `.stdout`, `tests/runtime/fs_not_found.nova` + `.stdout`, `tests/runtime/fs_invalid_data.nova` + `.stdout`

**Interfaces:**
- Consumes: `io_error_kind_of(code: Int) -> IoErrorKind` and `IoError` (Task 1).
- Produces: `fs::fail(&std::io::Error) -> i64` (the status mapping — **named `fail`, not `status_of`**; an earlier draft of this plan and `std/io/lib.nova`'s comment both used the latter, and nothing by that name exists) plus the `stash`/`take` slot helpers; builtins `fs_read_to_string(String) -> Int`, `fs_write_string(String, String) -> Int`, `fs_take_string() -> String`, `fs_last_error_message() -> String`, `fs_temp_dir() -> String`; Nova `read_to_string`, `write_string`, `temp_dir`. Tasks 3–4 reuse `fail`, the slots, `fs_last_error_message` and `temp_dir`.

**Amended after Task 2's review: `fs_temp_dir` moved here from Task 3, and a round-trip fixture added.** The review proved by mutation that `write_string` had **no coverage of any kind** — replacing its body with a no-op returning success left all 44 targets passing — and that `read_to_string`'s success path was equally untested, since both original fixtures exercise only the error branch. This plan's claim that those two fixtures were "sufficient" for Task 2 was wrong. A round-trip needs a writable path, so `temp_dir` comes forward with it. `STD_ONLY` therefore goes 17 → **22** here, and Task 3 adds five rather than six to reach 27.

**Decided before execution: the round-trip fixture belongs to Task 3, not here.** An earlier draft of this plan had Task 2 write `fs_roundtrip.nova` unregistered, to be gated only in Task 3 — but a fixture that cannot run is indistinguishable from a test that asserts nothing, and a reviewer would rightly flag it. `fs_roundtrip` is created, registered and passing entirely within Task 3, where `temp_dir` and `remove_file` exist. Task 2 gates its own deliverable with `fs_not_found` and `fs_invalid_data`, which is sufficient.

- [ ] **Step 1: Write Task 2's failing fixtures**

`tests/runtime/fs_not_found.nova`:

```nova
fn main() {
    block_on(go())
}

async fn go() {
    match read_to_string("nova_no_such_file_9f3a.txt").await {
        Ok(_) => println("unexpectedly read it")
        Err(e) => println(kind_name(e.kind))
    }
}

fn kind_name(k: IoErrorKind) -> String {
    match k {
        NotFound => "NotFound"
        PermissionDenied => "PermissionDenied"
        AlreadyExists => "AlreadyExists"
        InvalidData => "InvalidData"
        Interrupted => "Interrupted"
        TimedOut => "TimedOut"
        ConnectionRefused => "ConnectionRefused"
        Other => "Other"
    }
}
```

`tests/runtime/fs_not_found.stdout`:

```
NotFound
```

This asserts on `kind` and never on `message`, per the spec's diagnostics rule.

- [ ] **Step 2: Run it to confirm it fails**

```bash
cargo build --workspace && cargo test -p nova-cli fs_not_found -- --nocapture
```

Expected: FAIL — `E0001 cannot find function 'read_to_string'`. Confirm a non-zero test count.

- [ ] **Step 3: Create `crates/nova-runtime/src/fs.rs` with the slots and the status map**

```rust
//! Filesystem intrinsics for `std/fs`.
//!
//! # The boundary this module implements
//!
//! Nova has no out-parameters, so an intrinsic returns exactly one word — but
//! `read_to_string` must convey a `String` *and* a two-field `IoError`. Rather
//! than build a Nova sum and record here, which would put Nova value layout in
//! Rust, **the status code returned by each operation *is* the error kind**, and
//! payloads travel in the slots below. `std/fs`'s Nova wrappers map a status to
//! an `IoErrorKind` and construct every `Result` and `IoError` themselves.
//!
//! # Why these slots are `Cell<usize>` and not `RefCell<Option<String>>`
//!
//! **Every function here is called from inside an `async fn`'s generated
//! `$poll`, which has no landing pads**, so nothing here may panic (see
//! `nova_mir::async_lower`'s module doc). A `RefCell` borrow can panic; a `Cell`
//! of a `Copy` value cannot. The slots therefore hold a raw already-allocated
//! pointer as a `usize`, with `0` meaning empty.
//!
//! # Why the pointer is GC-rooted while it sits in a slot
//!
//! A slot is written by one intrinsic and read by the next, with Nova code in
//! between. A conservative stack scan cannot see a pointer that lives only in a
//! thread-local, so the object is registered with `gc::add_root` while stashed
//! and released on take — the same balance `task::spawn_internal` and
//! `take_output_internal` keep. Without it the payload could be collected
//! between the two calls, which is why this does not merely rely on "no
//! allocation happens in between".

use crate::{gc, NovaStr};
use std::cell::Cell;

/// Status codes. `0` is success; every other value is an `IoErrorKind`.
///
/// **This numbering is one half of a wire contract.** The other half is
/// `io_error_kind_of` in `std/io/lib.nova`. The two are independent copies, so
/// they are pinned together by a fixture per kind, not by this comment.
pub const OK: i64 = 0;
pub const NOT_FOUND: i64 = 1;
pub const PERMISSION_DENIED: i64 = 2;
pub const ALREADY_EXISTS: i64 = 3;
pub const INVALID_DATA: i64 = 4;
pub const INTERRUPTED: i64 = 5;
pub const TIMED_OUT: i64 = 6;
pub const CONNECTION_REFUSED: i64 = 7;
pub const OTHER: i64 = 8;

thread_local! {
    /// A GC-rooted `*mut NovaStr` awaiting `nova_rt_fs_take_string`, or 0.
    static STRING_SLOT: Cell<usize> = const { Cell::new(0) };
    /// A GC-rooted array block awaiting `nova_rt_fs_take_string_array`, or 0.
    static ARRAY_SLOT: Cell<usize> = const { Cell::new(0) };
    /// A GC-rooted `*mut NovaStr` awaiting `nova_rt_fs_last_error_message`, or 0.
    static MESSAGE_SLOT: Cell<usize> = const { Cell::new(0) };
}

/// Map an `std::io::Error` to its status code, and stash its message.
///
/// Called on every failure path, so the message is always available to the
/// wrapper that is about to build an `IoError`.
fn fail(e: &std::io::Error) -> i64 {
    use std::io::ErrorKind;
    stash(&MESSAGE_SLOT, gc_message(&e.to_string()));
    match e.kind() {
        ErrorKind::NotFound => NOT_FOUND,
        ErrorKind::PermissionDenied => PERMISSION_DENIED,
        ErrorKind::AlreadyExists => ALREADY_EXISTS,
        ErrorKind::InvalidData => INVALID_DATA,
        ErrorKind::Interrupted => INTERRUPTED,
        ErrorKind::TimedOut => TIMED_OUT,
        ErrorKind::ConnectionRefused => CONNECTION_REFUSED,
        _ => OTHER,
    }
}
```

You will need small helpers `stash(slot, ptr)` (register a root, then set) and `take(slot)` (read, clear, release the root, return), plus `gc_message(&str)` producing a `*mut NovaStr`. `gc_str` in `crates/nova-runtime/src/lib.rs` already does the string allocation — reuse it rather than reproducing `NovaStr { len, ptr }` here; a second copy of that layout is the drift class this module exists to avoid. Make it visible to this module rather than duplicating it.

**Order `take` exactly as `task::take_output_internal` does:** release the root *after* reading the pointer out of the slot and *before* returning, so the root's add/remove stay balanced and the returned pointer is then rooted by the caller's frame like any other runtime return value.

- [ ] **Step 4: Implement the two operations and the two takers**

```rust
/// Read `path` as UTF-8. Returns a status; on `OK` the contents are waiting in
/// `nova_rt_fs_take_string`.
///
/// Non-UTF-8 contents are `INVALID_DATA`: `std::fs::read_to_string` reports
/// `ErrorKind::InvalidData` for them, which is exactly the case that motivated
/// adding that kind (see docs/adr/0011-io-error-kinds.md).
///
/// # Safety
/// `path` must point to a live `NovaStr`.
#[no_mangle]
pub unsafe extern "C" fn nova_rt_fs_read_to_string(path: *const NovaStr) -> i64 {
    // SAFETY: forwarding this function's own contract.
    let p = unsafe { crate::as_str(path) };
    match std::fs::read_to_string(p) {
        Ok(s) => {
            stash(&STRING_SLOT, gc_message(&s));
            OK
        }
        Err(e) => fail(&e),
    }
}

/// Write `content` to `path`, truncating an existing file.
///
/// # Safety
/// Both arguments must point to live `NovaStr`s.
#[no_mangle]
pub unsafe extern "C" fn nova_rt_fs_write_string(
    path: *const NovaStr,
    content: *const NovaStr,
) -> i64 {
    // SAFETY: forwarding this function's own contract.
    let (p, c) = unsafe { (crate::as_str(path), crate::as_str(content)) };
    match std::fs::write(p, c) {
        Ok(()) => OK,
        Err(e) => fail(&e),
    }
}

/// Take the pending payload string. Returns an empty string if nothing is
/// pending, which a correct wrapper never asks for.
#[no_mangle]
pub extern "C" fn nova_rt_fs_take_string() -> *mut NovaStr {
    match take(&STRING_SLOT) {
        0 => gc_message(""),
        p => p as *mut NovaStr,
    }
}

/// Take the pending error message.
#[no_mangle]
pub extern "C" fn nova_rt_fs_last_error_message() -> *mut NovaStr {
    match take(&MESSAGE_SLOT) {
        0 => gc_message(""),
        p => p as *mut NovaStr,
    }
}
```

Add `pub mod fs;` to `crates/nova-runtime/src/lib.rs` and register all four symbols with the JIT.

- [ ] **Step 5: Wire the four builtins**

Run the seam grep — `StrToUpper` is the closest analogue, a `STD_ONLY` builtin taking and returning a `String`:

```bash
grep -rn 'StrToUpper' --include=*.rs crates/
```

Add a parallel arm at every hit for `FsReadToString`, `FsWriteString`, `FsTakeString`, `FsLastErrorMessage`, and bump `STD_ONLY` from `[Builtin; 17]` to `[Builtin; 21]`. Typeck signatures: `(vec![Ty::String], Ty::Int)`, `(vec![Ty::String, Ty::String], Ty::Int)`, `(vec![], Ty::String)`, `(vec![], Ty::String)`. MIR signatures: `(vec![MirTy::Ptr], MirTy::I64)`, `(vec![MirTy::Ptr, MirTy::Ptr], MirTy::I64)`, `(vec![], MirTy::Ptr)`, `(vec![], MirTy::Ptr)`.

- [ ] **Step 6: Create `std/fs/lib.nova` with the two wrappers**

```nova
// Nova standard library — filesystem.
//
// Compiled as an implicit module and glob-imported into every user module. Error
// types live in `std/io`.
//
// Every function here is an `async fn` per `nova-spec/20-STDLIB.md` §5, but
// **none of them suspends**: the operation runs synchronously inside the poll,
// because there is no I/O poller yet. The signature is what matters — it does
// not change when a poller lands, so no call site breaks then. The cost is that
// a call blocks the whole executor for its duration, so a sibling task makes no
// progress during a large read. See docs/adr/0011-io-error-kinds.md.
//
// # The shape of these wrappers is load-bearing
//
// Each one calls its intrinsic, then reads a slot, with **no `.await` between
// the two**. The runtime's payload and message slots are per-thread and are
// overwritten by the next operation; only an `.await` can let another task run,
// so a straight-line wrapper cannot have its slot clobbered by a sibling. A
// future function here that awaited mid-sequence would break that. Keep them
// straight-line.

// Read the whole file at `path` as a UTF-8 string.
//
// Non-UTF-8 contents are `InvalidData`, not a panic and not lossy replacement.
pub async fn read_to_string(path: String) -> Result<String, IoError> {
    let status = fs_read_to_string(path)
    if status == 0 {
        return Ok(fs_take_string())
    }
    Err(IoError { kind: io_error_kind_of(status), message: fs_last_error_message() })
}

// Write `content` to `path`, replacing any existing contents.
pub async fn write_string(path: String, content: String) -> Result<(), IoError> {
    let status = fs_write_string(path, content)
    if status == 0 {
        return Ok(())
    }
    Err(IoError { kind: io_error_kind_of(status), message: fs_last_error_message() })
}
```

Register `("$std.fs", include_str!("../../../std/fs/lib.nova"))` in `STD_MODULES`, bumping `5` to `6`. It must come **after** `$std.io`, since these wrappers use `IoError` and `io_error_kind_of`.

- [ ] **Step 7: Run the fixture to confirm it passes**

```bash
cargo build --workspace && cargo test -p nova-cli fs_not_found -- --nocapture
```

Expected: PASS, printing `NotFound`.

- [ ] **Step 8: Add the `InvalidData` fixture and a Rust-level slot test**

`tests/runtime/fs_invalid_data.nova` writes bytes that are not UTF-8 and reads them back. Since `write_string` only takes a `String`, create the file from Rust in the test harness instead, then have the fixture read it — register it in `run_tests.rs` with the harness writing `&[0xFF, 0xFE]` to a unique temp path first.

Then, in `crates/nova-runtime/src/fs.rs`'s own `mod tests`:

```rust
/// Pins the spec's "these `async fn`s never suspend" property.
///
/// **Decided before execution.** The spec asks for a test that `PARKED` is
/// empty after a `std/fs` call, but `PARKED` is private to `crate::task` and no
/// accessor exists, so the property is pinned at its source instead: nothing in
/// this module may register a park. That is strictly what matters — a `std/fs`
/// intrinsic can only reach the park set by calling `stage_park`.
///
/// Self-referential, like `every_rt_func_is_declared_with_its_real_signature`
/// elsewhere in this workspace, and carrying the same weakness: it checks the
/// text of this file, not the behaviour of a running program. It would miss
/// parking reached by some route other than a literal call. Accepted because the
/// alternative is test-only surface in a module this branch does not otherwise
/// touch.
#[test]
fn no_filesystem_intrinsic_registers_a_park() {
    let source = include_str!("fs.rs");
    // Skip this test's own body, which necessarily names the function.
    let code = source
        .split("fn no_filesystem_intrinsic_registers_a_park")
        .next()
        .unwrap_or(source);
    assert!(
        !code.contains("stage_park"),
        "a std/fs intrinsic must not park: these async fns run synchronously \
         inside the poll, and parking here without the poller work would \
         suspend a task nothing can wake"
    );
}

#[test]
fn a_stashed_string_survives_a_collection_before_it_is_taken() {
    let p = gc_message("payload");
    stash(&STRING_SLOT, p);
    // Allocate enough to trigger a collection while the only reference to the
    // payload is the thread-local slot. Without the `gc::add_root` in `stash`,
    // a conservative stack scan cannot see it and it is freed.
    for _ in 0..2000 {
        let _ = gc_message("churn");
    }
    let taken = nova_rt_fs_take_string();
    // SAFETY: `taken` is the payload `stash` rooted, still live.
    assert_eq!(unsafe { crate::as_str(taken) }, "payload");
}
```

- [ ] **Step 9: Prove both tests discriminate**

Two mutations, one at a time, each applied then reverted with a rebuild:

1. Delete the `gc::add_root` call in `stash`. Expected: `a_stashed_string_survives_a_collection_before_it_is_taken` fails or reads garbage. **If it still passes, the churn loop is not triggering a collection — raise the count or use `NOVA_GC_STRESS=1` rather than concluding the root is unnecessary.**
2. In `fail`, swap `NOT_FOUND` and `PERMISSION_DENIED`. Expected: `fs_not_found` fails, printing `PermissionDenied`. This is the round-trip guard for the wire contract.

Paste both transcripts into your report.

- [ ] **Step 10: Full verification and commit**

As Task 1 Step 10. Subject: `feat(std): read_to_string and write_string over a status boundary`

---

### Task 3: `exists`, `create_dir`, `create_dir_all`, `remove_file`, `remove_dir_all`

Mechanical, reusing Task 2's protocol whole. This task also **creates** the round-trip fixture, since only now do `temp_dir` and `remove_file` exist to make it pass.

**Files:**
- Modify: `crates/nova-runtime/src/fs.rs`, `crates/nova-runtime/src/lib.rs`, `crates/nova-resolver/src/lib.rs`, `crates/nova-typeck/src/check.rs`, `crates/nova-mir/src/lib.rs`, `crates/nova-mir/src/lower.rs`, `std/fs/lib.nova`
- Create: `tests/runtime/fs_roundtrip.nova` + `.stdout`, `tests/runtime/fs_already_exists.nova` + `.stdout`, `tests/runtime/fs_dirs.nova` + `.stdout`

**The round-trip fixture, created and registered here:**

`tests/runtime/fs_roundtrip.nova`:

```nova
fn main() {
    block_on(go())
}

async fn go() {
    let p = "${temp_dir()}/nova_fs_roundtrip_5d2c.txt"
    match write_string(p, "hello\nworld").await {
        Ok(_) => println("wrote")
        Err(e) => println("write failed: ${e.message}")
    }
    match read_to_string(p).await {
        Ok(s) => println(s)
        Err(e) => println("read failed: ${e.message}")
    }
    match remove_file(p).await {
        Ok(_) => println("removed")
        Err(e) => println("remove failed: ${e.message}")
    }
}
```

`tests/runtime/fs_roundtrip.stdout`:

```
wrote
hello
world
removed
```

It exercises Task 2's two operations and this task's `remove_file` and `temp_dir` in one pass, which is why it lives here rather than in Task 2.

**Interfaces:**
- Consumes: `fs::fail`, `fs::OK`, the slots, `fs_last_error_message`, `io_error_kind_of` (Task 2).
- Produces: builtins `fs_exists(String) -> Bool`, `fs_create_dir(String) -> Int`, `fs_create_dir_all(String) -> Int`, `fs_remove_file(String) -> Int`, `fs_remove_dir_all(String) -> Int`; Nova `exists`, `create_dir`, `create_dir_all`, `remove_file`, `remove_dir_all`. Task 4 uses `temp_dir` (from Task 2) and `create_dir` in its fixtures.

**`fs_temp_dir` is NOT this task's — it moved to Task 2** when that task's review showed a round-trip fixture was needed there and a round-trip needs a writable path. Do not add it again; `temp_dir()` already exists. `STD_ONLY` therefore starts at **22** here and reaches 27 by adding **five** builtins, not six.

`temp_dir` is a deviation from `nova-spec/20-STDLIB.md` §5, which does not list it, and belongs in Task 5's ADR regardless of which task shipped it.

- [ ] **Step 1: Write the failing `AlreadyExists` fixture**

`tests/runtime/fs_already_exists.nova`:

```nova
fn main() {
    block_on(go())
}

async fn go() {
    let d = "${temp_dir()}/nova_fs_exists_7c1b"
    let _ = remove_dir_all(d).await
    match create_dir(d).await {
        Ok(_) => println("created")
        Err(e) => println("first create failed: ${e.message}")
    }
    match create_dir(d).await {
        Ok(_) => println("created twice, which is wrong")
        Err(e) => println(kind_name(e.kind))
    }
    let _ = remove_dir_all(d).await
    println("cleaned")
}

fn kind_name(k: IoErrorKind) -> String {
    match k {
        NotFound => "NotFound"
        PermissionDenied => "PermissionDenied"
        AlreadyExists => "AlreadyExists"
        InvalidData => "InvalidData"
        Interrupted => "Interrupted"
        TimedOut => "TimedOut"
        ConnectionRefused => "ConnectionRefused"
        Other => "Other"
    }
}
```

`tests/runtime/fs_already_exists.stdout`:

```
created
AlreadyExists
cleaned
```

- [ ] **Step 2: Write the failing `exists` fixture**

`tests/runtime/fs_dirs.nova`:

```nova
fn main() {
    block_on(go())
}

async fn go() {
    let d = "${temp_dir()}/nova_fs_dirs_2e9f/a/b"
    let _ = remove_dir_all("${temp_dir()}/nova_fs_dirs_2e9f").await
    println("${exists(d).await}")
    match create_dir_all(d).await {
        Ok(_) => println("made")
        Err(e) => println("failed: ${e.message}")
    }
    println("${exists(d).await}")
    let _ = remove_dir_all("${temp_dir()}/nova_fs_dirs_2e9f").await
    println("${exists(d).await}")
}
```

`tests/runtime/fs_dirs.stdout`:

```
false
made
true
false
```

**Why three `exists` calls:** absent-then-present-then-absent means a hardcoded `true` or `false` fails, which a single call would not catch.

- [ ] **Step 3: Run both to confirm they fail**

```bash
cargo build --workspace && cargo test -p nova-cli fs_already_exists -- --nocapture
```

Expected: FAIL — `E0001 cannot find function 'temp_dir'`. Confirm a non-zero test count.

- [ ] **Step 4: Add the six runtime functions**

In `crates/nova-runtime/src/fs.rs`:

```rust
/// Whether `path` exists. Cannot distinguish absent from unreadable, because
/// `nova-spec/20-STDLIB.md` §5 gives `exists` a `Bool` return rather than a
/// `Result`; that is the spec's choice and is left alone.
///
/// # Safety
/// `path` must point to a live `NovaStr`.
#[no_mangle]
pub unsafe extern "C" fn nova_rt_fs_exists(path: *const NovaStr) -> i8 {
    // SAFETY: forwarding this function's own contract.
    let p = unsafe { crate::as_str(path) };
    i8::from(std::path::Path::new(p).exists())
}

/// Create the directory `path`. Its parent must exist; an existing `path` is
/// `ALREADY_EXISTS`.
///
/// # Safety
/// `path` must point to a live `NovaStr`.
#[no_mangle]
pub unsafe extern "C" fn nova_rt_fs_create_dir(path: *const NovaStr) -> i64 {
    // SAFETY: forwarding this function's own contract.
    let p = unsafe { crate::as_str(path) };
    match std::fs::create_dir(p) {
        Ok(()) => OK,
        Err(e) => fail(&e),
    }
}
```

Then `nova_rt_fs_create_dir_all` over `std::fs::create_dir_all`, `nova_rt_fs_remove_file` over `std::fs::remove_file`, and `nova_rt_fs_remove_dir_all` over `std::fs::remove_dir_all`, each in exactly the shape above. And:

```rust
/// The system temporary directory, as a path string.
///
/// Exists so fixtures need not hardcode a writable location. Not in
/// `nova-spec/20-STDLIB.md` §5; recorded as a deviation in
/// docs/adr/0011-io-error-kinds.md.
#[no_mangle]
pub extern "C" fn nova_rt_fs_temp_dir() -> *mut NovaStr {
    gc_message(&std::env::temp_dir().to_string_lossy())
}
```

**`to_string_lossy` is deliberate**: a Nova `String` is UTF-8 and a Windows path is UTF-16, so a path with unpaired surrogates cannot round-trip. Lossy conversion keeps a usable path rather than failing; note it in the doc comment.

**Measure `remove_dir_all`'s behaviour on a read-only entry and record what you observe** — the spec flags this as expectation, not measurement. Do not restate the guess as behaviour.

- [ ] **Step 5: Wire the six builtins**

Re-run `grep -rn 'StrToUpper' --include=*.rs crates/` and add parallel arms. `STD_ONLY` goes from `[Builtin; 21]` to `[Builtin; 27]`. Typeck: `fs_exists` is `(vec![Ty::String], Ty::Bool)`; the four status functions are `(vec![Ty::String], Ty::Int)`; `fs_temp_dir` is `(vec![], Ty::String)`. MIR: `MirTy::I8` for the `Bool` return, `MirTy::I64` for statuses, `MirTy::Ptr` for the string.

- [ ] **Step 6: Add the six Nova wrappers**

In `std/fs/lib.nova`, following `write_string`'s shape exactly:

```nova
// Whether `path` exists.
//
// Returns `Bool`, not `Result`, per `nova-spec/20-STDLIB.md` §5 — so a path that
// exists but cannot be examined is indistinguishable from one that is absent.
pub async fn exists(path: String) -> Bool {
    fs_exists(path)
}

// Create the directory `path`. The parent must already exist; use
// `create_dir_all` otherwise. An existing `path` is `AlreadyExists`.
pub async fn create_dir(path: String) -> Result<(), IoError> {
    let status = fs_create_dir(path)
    if status == 0 {
        return Ok(())
    }
    Err(IoError { kind: io_error_kind_of(status), message: fs_last_error_message() })
}

// The system temporary directory.
//
// Not `async`: it queries the environment and touches no filesystem.
pub fn temp_dir() -> String {
    fs_temp_dir()
}
```

Then `create_dir_all`, `remove_file` and `remove_dir_all` in `create_dir`'s exact shape.

- [ ] **Step 7: Register all three fixtures and run them**

```bash
cargo build --workspace && cargo test -p nova-cli fs_ -- --nocapture
```

Expected: `fs_roundtrip`, `fs_already_exists`, `fs_dirs`, `fs_not_found`, `fs_invalid_data` all pass.

- [ ] **Step 8: Prove `fs_dirs` discriminates**

Change `nova_rt_fs_exists` to `i8::from(true)`, rebuild, re-run. Expected: FAIL on the first `exists` line. Revert and rebuild. Paste the transcript.

- [ ] **Step 9: Full verification and commit**

Subject: `feat(std): exists, create_dir, create_dir_all, remove_file, remove_dir_all`

---

### Task 4: `read_dir` and `DirEntry`

The only task where the runtime builds a Nova **array**, which is `str_chars`'s shape one step further.

**Files:**
- Modify: `crates/nova-runtime/src/fs.rs`, `crates/nova-runtime/src/lib.rs`, `crates/nova-resolver/src/lib.rs`, `crates/nova-typeck/src/check.rs`, `crates/nova-mir/src/lib.rs`, `crates/nova-mir/src/lower.rs`, `std/fs/lib.nova`
- Test: `crates/nova-runtime/src/fs.rs` (`mod tests`), `tests/runtime/fs_read_dir.nova` + `.stdout`

**Interfaces:**
- Consumes: everything from Tasks 1–3, especially `temp_dir`, `create_dir_all`, `write_string`, `remove_dir_all`.
- Produces: builtins `fs_read_dir(String) -> Int`, `fs_take_string_array() -> [String]`, `fs_kind(String) -> Int`; Nova `read_dir(path) -> Result<[DirEntry], IoError>` and `record DirEntry`.

- [ ] **Step 1: Write the failing fixture, with entries created out of order**

`tests/runtime/fs_read_dir.nova`:

```nova
fn main() {
    block_on(go())
}

async fn go() {
    let d = "${temp_dir()}/nova_fs_read_dir_4b8e"
    let _ = remove_dir_all(d).await
    let _ = create_dir_all(d).await
    // Created deliberately out of alphabetical order: zebra first, then apple,
    // then a subdirectory. If the runtime does not sort, output follows
    // filesystem order and this fixture fails.
    let _ = write_string("${d}/zebra.txt", "z").await
    let _ = write_string("${d}/apple.txt", "a").await
    let _ = create_dir("${d}/mid").await
    match read_dir(d).await {
        Ok(entries) => {
            for e in entries.iter() {
                println("${e.name} file=${e.is_file} dir=${e.is_dir}")
            }
        }
        Err(e) => println("failed: ${e.message}")
    }
    let _ = remove_dir_all(d).await
}
```

`tests/runtime/fs_read_dir.stdout`:

```
apple.txt file=true dir=false
mid file=false dir=true
zebra.txt file=true dir=false
```

**This one fixture kills three separate mutations**: dropping the sort (order changes), swapping `fs_kind`'s file and dir codes (the `mid` line inverts), and returning a wrong array length (a line goes missing).

- [ ] **Step 2: Run it to confirm it fails**

```bash
cargo build --workspace && cargo test -p nova-cli fs_read_dir -- --nocapture
```

Expected: FAIL — `E0001 cannot find function 'read_dir'`. Confirm a non-zero test count.

- [ ] **Step 3: Add the three runtime functions**

```rust
/// List `path`'s entry names, sorted. On `OK` the names are waiting in
/// `nova_rt_fs_take_string_array`.
///
/// **Sorted in the runtime deliberately.** Directory order is unspecified by
/// every OS, so unsorted output would make each fixture platform-dependent.
///
/// # Safety
/// `path` must point to a live `NovaStr`.
#[no_mangle]
pub unsafe extern "C" fn nova_rt_fs_read_dir(path: *const NovaStr) -> i64 {
    // SAFETY: forwarding this function's own contract.
    let p = unsafe { crate::as_str(path) };
    let iter = match std::fs::read_dir(p) {
        Ok(it) => it,
        Err(e) => return fail(&e),
    };
    let mut names: Vec<String> = Vec::new();
    for entry in iter {
        match entry {
            Ok(e) => names.push(e.file_name().to_string_lossy().into_owned()),
            Err(e) => return fail(&e),
        }
    }
    names.sort();
    stash_array(&names);
    OK
}

/// What `path` is: 0 absent, 1 file, 2 directory.
///
/// One call rather than separate `is_file`/`is_dir` intrinsics, so a `DirEntry`
/// costs one syscall instead of two and the two answers cannot disagree.
///
/// # Safety
/// `path` must point to a live `NovaStr`.
#[no_mangle]
pub unsafe extern "C" fn nova_rt_fs_kind(path: *const NovaStr) -> i64 {
    // SAFETY: forwarding this function's own contract.
    let p = unsafe { crate::as_str(path) };
    let meta = match std::fs::metadata(p) {
        Ok(m) => m,
        Err(_) => return 0,
    };
    if meta.is_dir() {
        2
    } else {
        1
    }
}
```

`stash_array` builds the Nova array and roots it:

```rust
/// Build a Nova `[String]` from `names` and stash it, GC-rooted.
///
/// **Reproduces the layout codegen emits for an array**: word 0 is the element
/// count, elements follow at `8 + 8*i`, allocated scanned so the collector
/// traces the `NovaStr` pointers inside. Getting this wrong is a silent
/// miscompile rather than a failure, which is why `mod tests` asserts the
/// tracked `(size, scan)` through `gc::object_info` and not merely the values
/// read back — the same discipline `nova_rt_str_chars` carries.
///
/// Each element is rooted while it is being built, because the allocation of a
/// later element can collect and an earlier one is then named only by a block
/// that is itself not yet reachable from anywhere the collector scans.
fn stash_array(names: &[String]) {
    let n = names.len();
    let block = gc::alloc(8 + 8 * n, true);
    gc::add_root(block);
    let words = block as *mut i64;
    // SAFETY: `block` has `8 + 8*n` writable bytes, so word 0 and words
    // `1..=n` are all in bounds.
    unsafe { *words = n as i64 };
    for (i, name) in names.iter().enumerate() {
        let s = gc_message(name);
        // SAFETY: same block; `i < n`.
        unsafe { *words.add(1 + i) = s as i64 };
    }
    // The block is already rooted by `stash` below, so drop the build-time root
    // to keep `add_root`/`remove_root` balanced.
    gc::remove_root(block);
    stash(&ARRAY_SLOT, block as *mut NovaStr);
}
```

**Check that ordering against `gc.rs`'s actual contract before relying on it.** The claim is that rooting `block` across the element allocations keeps both the block and, through it, every already-written element reachable. If `gc::add_root` on a block does not cause its contents to be traced, this is wrong and each element needs its own root — probe it, do not assume.

Then `nova_rt_fs_take_string_array` mirroring `nova_rt_fs_take_string` but returning `*mut u8`, with `0` yielding a fresh empty array rather than a null pointer.

- [ ] **Step 4: Wire the three builtins**

Re-run the seam grep. `STD_ONLY` goes `[Builtin; 27]` → `[Builtin; 30]`. Typeck: `fs_read_dir` is `(vec![Ty::String], Ty::Int)`; `fs_take_string_array` is `(vec![], Ty::Array(Box::new(Ty::String)))`; `fs_kind` is `(vec![Ty::String], Ty::Int)`. MIR: `(vec![MirTy::Ptr], MirTy::I64)`, `(vec![], MirTy::Ptr)`, `(vec![MirTy::Ptr], MirTy::I64)`.

**Note the running total: the spec said 12 new intrinsics and 17 → 29. This plan adds `fs_temp_dir` in Task 3, so the real total is 13 and `[Builtin; 30]`.** Correct the spec's §3.4 count as part of Task 5 rather than leaving the two documents disagreeing.

- [ ] **Step 5: Add `DirEntry` and `read_dir` in Nova**

```nova
// One entry in a directory listing.
//
// `is_file` and `is_dir` come from a single `fs_kind` call, so they cannot
// disagree: exactly one is true for an entry that exists.
pub record DirEntry {
    pub name: String
    pub path: String
    pub is_file: Bool
    pub is_dir: Bool
}

// List the entries in `path`, sorted by name.
//
// Sorted because directory order is unspecified by the operating system; the
// sort happens in the runtime.
//
// Builds each `DirEntry` here rather than in the runtime: a record built in Rust
// would put Nova's field layout in two places, which is the drift this module's
// boundary exists to avoid. The cost is one extra `fs_kind` call per entry.
pub async fn read_dir(path: String) -> Result<[DirEntry], IoError> {
    let status = fs_read_dir(path)
    if status != 0 {
        return Err(IoError { kind: io_error_kind_of(status), message: fs_last_error_message() })
    }
    let names = fs_take_string_array()
    let n = names.len()
    // A repeat array, then every slot overwritten. `[init; n]` evaluates `init`
    // exactly ONCE, so all `n` slots start as the same heap object -- harmless
    // only because the loop below replaces each one. Do not read a slot before
    // assigning it.
    let mut out = [DirEntry { name: "", path: "", is_file: false, is_dir: false }; n]
    let mut i = 0
    while i < n {
        let name = names[i]
        let full = "${path}/${name}"
        let k = fs_kind(full)
        out[i] = DirEntry { name: name, path: full, is_file: k == 1, is_dir: k == 2 }
        i = i + 1
    }
    Ok(out)
}
```

**No `Vec` is involved, deliberately — `Vec` has no array conversion.** Measured: `std/collections/lib.nova`'s `Vec` offers `new`, `len`, `is_empty`, `push`, `pop`, `get`, `set`, `clear` and `iter`, and nothing that produces a `[T]`. Building the `[DirEntry]` directly avoids both an invented method and a signature deviation, so `read_dir` returns `[DirEntry]` exactly as `nova-spec/20-STDLIB.md` §5 specifies.

Two things to verify as you write this, because neither was measured: that `out[i] = …` is accepted on a `mut` local holding an array (the mutable-receiver rule requires the assignment be rooted at a `mut` binding), and that `names[i]` indexes a `[String]` returned from a builtin the same way any other array indexes. If either fails, report it rather than reaching for `Vec`.

- [ ] **Step 6: Add the layout test**

```rust
#[test]
fn a_stashed_array_has_the_layout_the_abi_declares() {
    let names = vec!["a".to_string(), "bb".to_string()];
    stash_array(&names);
    let block = take(&ARRAY_SLOT) as *mut u8;
    assert_eq!(
        gc::object_info(block),
        Some((8 + 8 * 2, true)),
        "an array block is a length word plus one word per element, scanned"
    );
    let words = block as *mut i64;
    // SAFETY: the block is `8 + 8*2` bytes, so these three words are in bounds.
    unsafe {
        assert_eq!(*words, 2);
        assert_eq!(crate::as_str(*words.add(1) as *const NovaStr), "a");
        assert_eq!(crate::as_str(*words.add(2) as *const NovaStr), "bb");
    }
}
```

**Reading the words back is not enough on its own** — an allocation that is too small but has slack after it passes a value check and fails the `gc::object_info` check. Both assertions are needed, which is exactly what the sleep and join state layouts established on the park-set branch.

- [ ] **Step 7: Prove the fixture discriminates, three ways**

Apply each mutation alone, confirm the failure, revert, rebuild:

1. Delete `names.sort()`. Expected: `fs_read_dir` fails on order — **but filesystem order may coincidentally be alphabetical on this host, in which case the fixture passes and the test is not discriminating.** If that happens, say so and rename the entries so filesystem order provably differs, rather than reporting the mutation as killed.
2. Swap `fs_kind`'s `2` and `1`. Expected: the `mid` line inverts.
3. Write `n as i64 - 1` into word 0. Expected: a line goes missing.

Paste all three transcripts.

- [ ] **Step 8: Full verification and commit**

Subject: `feat(std): read_dir and DirEntry`

---

### Task 5: ADR, spec amendment, CHANGELOG, and the sweep

**Files:**
- Create: `docs/adr/0011-io-error-kinds.md`
- Modify: `nova-spec/20-STDLIB.md`, `CHANGELOG.md`, `docs/superpowers/specs/2026-08-11-std-fs-strings-design.md`

**Interfaces:**
- Consumes: everything Tasks 1–4 shipped.
- Produces: nothing code depends on.

- [ ] **Step 1: Write ADR 0011**

Check the highest existing ADR number first — this plan says 0011, but confirm rather than trusting it. Record **four** deviations from `nova-spec/20-STDLIB.md`, with the reason for each:

1. `IoErrorKind` gains `AlreadyExists` and `InvalidData`. The specced list is network-flavoured; the two commonest filesystem failures would otherwise collapse into `Other`, forcing user code to string-match `message`, which is what a kind enum exists to prevent.
2. This increment ships a subset of §5 — `read`, `write`, `open`, `File` need a byte type.
3. `temp_dir()` is added and is not in the spec.
4. Only if Task 4 Step 5 could not produce `[DirEntry]` after all — the plan's approach avoids that, so this entry should normally be unnecessary. Do not add a deviation that did not happen.

Also record the two accepted limitations: **these `async fn`s never suspend**, so a call blocks the whole executor; and `exists` cannot distinguish absent from unreadable. Put the first beside ADR 0009's existing async footguns by cross-referencing it.

- [ ] **Step 2: Amend `nova-spec/20-STDLIB.md` §4's `IoErrorKind`**

Add the two variants, with a dated note pointing at ADR 0011. Amend in place; do not silently rewrite.

- [ ] **Step 3: `CHANGELOG.md` under `### Added`**

`std/fs`, `std/io`'s error types, and `eprint`/`eprintln` are all additions — nothing that previously compiled changes meaning, so **nothing here belongs under `### Changed`**. Check the heading's own stated scope before filing: on an earlier branch an entry was cross-filed under a heading scoped to a different phase, breaking that section's purpose.

- [ ] **Step 4: Correct the spec's intrinsic count**

`docs/superpowers/specs/2026-08-11-std-fs-strings-design.md` §3.4 says twelve intrinsics and 17 → 29. With `fs_temp_dir` it is thirteen and 17 → 30. Fix it with a dated note so the two documents agree.

- [ ] **Step 5: Run the quantifier sweep**

```bash
git diff main --stat
```

Then grep each changed file's **added** lines for `always`, `every`, `only`, `any`, `never`, `all`, `cannot`. Per hit: delete the quantifier, or state the measurement behind it. Scope to added lines — whole-file grepping is dominated by pre-existing code.

**Two things this sweep structurally cannot catch, so check them by reading:** a doc quoting a **literal diagnostic string** (there is no keyword in it to grep for), and **a sentence this branch falsified but did not touch**. Both have bitten this project. In particular, re-read `std/task/lib.nova`'s and `crates/nova-runtime/src/task.rs`'s module docs: they describe what suspends and what does not, and this branch adds `async fn`s that never suspend.

- [ ] **Step 6: Full verification and commit**

Subject: `docs: ADR 0011 for std/fs's spec deviations`

---

## Self-Review

**Spec coverage.** §3.1 `std/io` → Task 1. §3.2's eight functions → Tasks 2 (2), 3 (5, plus `temp_dir`), 4 (1). §3.3 `eprint`/`eprintln` → Task 1. §3.4's boundary → Task 2 Steps 4–7, extended in 3 and 4. §4's non-goals → nothing implements them, by construction; the drive loop is untouched in every task. §5's never-suspends property → documented in `std/fs`'s header (Task 2 Step 7) and ADR'd in Task 5. §6 diagnostics → every error fixture asserts `kind`, never `message`. §7's deviations → Task 5 Step 1, which lists **four** where the spec listed two. §8's risks: risk 1 → Task 2 Step 10 mutation 2; risk 2 → Task 4 Step 6; risk 3 → Task 2's separate slots and its `fs_take_string` mutation; risk 4 → unique temp paths in every fixture; risk 5 → Task 3 Step 4's measure-and-record instruction. §9's mutation table → every row has a step, except `fs_take_string` reading the error slot, which Task 2 Step 10 covers by a different route.

**§9's `PARKED`-stays-empty requirement is met at its source instead, decided before execution.** `PARKED` is private to `crates/nova-runtime/src/task.rs` with no accessor, so a test asserting it directly would need test-only surface in a module this branch does not otherwise touch. Task 2 Step 8's `no_filesystem_intrinsic_registers_a_park` pins the property where it actually lives: a `std/fs` intrinsic can only reach the park set by calling `stage_park`, so asserting that no such call exists in `fs.rs` is exactly the guarantee. It is self-referential and that weakness is recorded in the test's own doc comment.

**Type consistency.** `io_error_kind_of(code: Int) -> IoErrorKind` used identically in Tasks 1–4. `IoError { kind, message }` field order and names constant throughout. Status constants named once in Task 2 and reused. `fs_kind`'s 0/1/2 encoding stated identically in Task 3's absence of it, Task 4 Step 3, and Task 4 Step 5's `k == 1` / `k == 2`. The `STD_ONLY` length runs 17 → 21 → 27 → 30, and each task states the number it starts from.

**Placeholder scan.** No TBDs. The plan's first draft called `Vec::to_array`, which **does not exist** — measured against `std/collections/lib.nova`, whose `Vec` offers only `new`, `len`, `is_empty`, `push`, `pop`, `get`, `set`, `clear`, `iter`. Task 4 Step 5 now builds the array directly and no `Vec` is involved. That was the self-review's one real catch, and it is exactly the "references types or methods not defined anywhere" failure the rule against placeholders names.

Three places still deliberately say "verify before relying" rather than asserting — `gc::add_root`'s tracing of a block's contents, `remove_dir_all`'s Windows behaviour, and array index-assignment on a `mut` local — because each is a fact I did not measure. Inventing an answer for them is how a plan ships that cannot be followed.
