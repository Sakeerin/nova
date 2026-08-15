# I/O Poller and `std/net` Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Give Nova's executor a third wake source — I/O readiness — and a TCP client to exercise it, so that a task waiting on a socket lets its siblings run.

**Architecture:** A new `poll.rs` owns one operation: wait until a socket in this set is ready or an instant passes. The executor's drained-queue branch calls it instead of sleeping, so `std::thread::sleep` disappears. A new `net.rs` holds a socket table (the `file.rs` model) and Rust-built futures that stage `Wait::Io` parks. `std/net` reaches Nova through the `Future<T>` spelling.

**Tech Stack:** Rust (nova-runtime, nova-resolver, nova-typeck, nova-mir, nova-cli), Nova (`std/net`), `std::net` + `libc`/`winsock` readiness via a thin platform shim.

**Spec:** `docs/superpowers/specs/2026-08-15-io-poller-and-std-net-design.md` (branch `io-poller-std-net`, 2026-08-15). Read it before Task 1 — its §3 and §4 are this plan's argument, and §8 separates what was measured from what is only reasoned.

**Base:** `main` at `3973dfd`, 404 commits, 0 merge commits, 921 tests (8 deliberately ignored), clean tree.

## Global Constraints

- **No panic may cross a generated poll boundary.** No `unwrap`, `expect`, slice indexing, or fallible `format!` in any poll function or anything it calls. Use `try_borrow_mut` with an `abort_with` backstop. `abort_with` is acceptable — it terminates without unwinding.
- **The poll ABI is frozen:** `PollFn = unsafe extern "C-unwind" fn(state: *mut u8, task_ctx: *mut u8) -> i64`, exactly two statuses (`POLL_PENDING = 0`, `POLL_READY = 1`), `task_ctx` always null.
- **MSRV is 1.78** — no `reason = "…"` in any lint attribute. The MSRV CI job is known-vacuous, so such an attribute ships as a user build failure.
- `cargo build --workspace` **before** `cargo test`. `--no-fail-fast` mandatory. **Never pipe cargo output through `head`/`tail` before summing** — sum the `test result:` lines mechanically.
- The **8 `#[ignore]`d ADR-0010** conservative-scan GC tests stay ignored and untouched. Read them if useful; never run or modify them.
- **Every fixture path unique per process.** Copy `unique_temp_dir(label)` in `run_tests.rs`; do not copy `write_test_project`'s run-invariant path.
- Commit with `git commit -F <utf8 file>`, **never a heredoc**. Body ends exactly `Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>`.
- **Never push. Never touch `main`. Never amend a commit** — amending has orphaned a cited SHA on this repo already.
- **Never cite a commit SHA that is not yet on `main`** (`agent.md`, Commits). A cited SHA is durable only if `git merge-base --is-ancestor <sha> main` already succeeds.
- `RESERVED_TYPE_NAMES` stays **7**. `STD_MODULES` goes **7 → 8**.

---

## File Structure

| File | Responsibility |
|---|---|
| `crates/nova-runtime/src/poll.rs` | **Create.** One operation: `wait(sockets, deadline) -> Vec<RawSocket>`. Knows nothing about tasks. |
| `crates/nova-runtime/src/net.rs` | **Create.** Socket table (the `file.rs` model), the `nova_rt_net_*` intrinsics, and the Rust-built futures that stage `Wait::Io`. |
| `crates/nova-runtime/src/task.rs` | **Modify.** `Wait::Io`, `Staged`, the restructured drive loop, `wake_due`'s new arm; delete `std::thread::sleep`. |
| `crates/nova-runtime/src/lib.rs` | **Modify.** `mod poll; mod net;` and the `symbols()` entries. |
| `crates/nova-runtime/src/fs.rs` | **Modify.** The pinned-kinds comment, four → six (Task 7 only). |
| `crates/nova-resolver/src/lib.rs` | **Modify.** `Builtin` variants, Nova-visible names, `STD_ONLY`, `STD_MODULES`. |
| `crates/nova-typeck/src/check.rs` | **Modify.** `builtin_signature`, `check_builtin_call`, and the coverage test's arms. |
| `crates/nova-mir/src/lib.rs`, `lower.rs` | **Modify.** `RtFunc` variant, symbol, `signature()`, `lower_call`. |
| `std/net/lib.nova` | **Create.** `TcpStream`, `connect`, `close`, `read_timeout`, `impl Read`, `impl Write`. |
| `tests/runtime/net_*.nova` + `.stdout` | **Create.** Round-trip, interleaving, timeout, refused, lifetime. |
| `crates/nova-cli/tests/run_tests.rs` | **Modify.** The loopback echo harness and the fixture runners. |

**Why two runtime files rather than one:** `poll.rs` is platform code with no notion of a task; `net.rs` is task-aware and holds the futures. Keeping the platform shim isolated means the readiness mechanism can be unit-tested against raw sockets with no executor involved, which Task 2 does.

---

## Task 1: The executor learns to wait instead of sleep

**Files:**
- Create: `crates/nova-runtime/src/poll.rs`
- Modify: `crates/nova-runtime/src/task.rs` (`Wait`, `stage_park`, `run_to_completion`, `wake_due`, `earliest_deadline`, `deadlock_report`)
- Modify: `crates/nova-runtime/src/lib.rs` (`mod poll;`)

**Interfaces:**
- Produces: `poll::wait(sockets: &[RawSocket], deadline: Option<Instant>) -> Vec<RawSocket>`; `Wait::Io { socket: RawSocket, interest: Interest, deadline: Option<Instant> }`; `poll::Interest::{Read, Write}`; `poll::RawSocket` (a transparent `i64` newtype so `net.rs` and `task.rs` agree without either depending on the other's socket type).

**This task ships no I/O.** It restructures the executor so that sleeping *is* waiting on an empty socket set, and so a future `Wait::Io` cannot reach `report_deadlock()`. Memory's instruction is to make the drive loop a forced match site as the first act of the increment; this is that act.

- [ ] **Step 1: Write the failing test — an I/O park must not report a deadlock**

Add to `task.rs`'s test module:

```rust
#[test]
fn a_park_on_io_with_no_deadline_is_not_a_deadlock() {
    // `deadlock_report` is the text-only half of `report_deadlock`, so a test
    // can assert what a program would be told without aborting the process.
    let report = with_parked(&[(7, Wait::Io {
        socket: RawSocket(-1),
        interest: Interest::Read,
        deadline: None,
    })], deadlock_report);
    assert!(
        report.contains("waiting on i/o"),
        "an I/O park must describe itself, got: {report}"
    );
}
```

- [ ] **Step 2: Run it to confirm it fails**

Run: `cargo test -p nova-runtime --lib a_park_on_io_with_no_deadline_is_not_a_deadlock`
Expected: FAIL to **compile** — `Wait` has no `Io` variant, `RawSocket` and `Interest` do not exist. A compile failure is the correct first failure here; do not stub the types to get a runtime failure instead.

- [ ] **Step 3: Add the types**

In `poll.rs`:

```rust
/// A socket as the poller sees it: the OS handle, widened to `i64` so
/// `task.rs` can hold one in `Wait` without depending on this module's
/// platform types.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct RawSocket(pub i64);

/// What a task is waiting for the socket to become.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Interest {
    Read,
    Write,
}
```

In `task.rs`, extend `Wait`:

```rust
enum Wait {
    Deadline(Instant),
    Task(i64),
    /// Wake once `socket` is ready for `interest`, or `deadline` passes.
    ///
    /// The deadline rides inside this variant rather than being parked as a
    /// second entry: one task must have exactly one `PARKED` entry, or every
    /// wake path has to remember to remove two.
    Io {
        socket: crate::poll::RawSocket,
        interest: crate::poll::Interest,
        deadline: Option<Instant>,
    },
}
```

- [ ] **Step 4: Fix the two forced sites the compiler now names**

`earliest_deadline` and `deadlock_report` match `Wait` exhaustively with no wildcard, so both fail to compile until updated. That is by design — it is the mechanism that guarantees a new variant is considered everywhere it matters.

```rust
// earliest_deadline
.filter_map(|&(_, wait)| match wait {
    Wait::Deadline(at) => Some(at),
    Wait::Io { deadline, .. } => deadline,
    Wait::Task(_) => None,
})

// deadlock_report
Wait::Io { deadline: None, .. } => {
    report.push_str(&format!("  task {id} is waiting on i/o\n"));
}
Wait::Io { deadline: Some(_), .. } => {
    report.push_str(&format!("  task {id} is waiting on i/o with a deadline\n"));
}
```

- [ ] **Step 5: Give `wake_due` a real arm — it is NOT forced**

`wake_due`'s `retain` uses `_ => true`, so it compiles unchanged and silently leaves every I/O park asleep. That is correct only while I/O waits are untimed. Once `Io` carries a deadline, **a passed deadline is the timeout firing and must wake the task**:

```rust
parked.retain(|&(id, wait)| match wait {
    Wait::Deadline(deadline) if deadline <= now => { woken.push(id); false }
    Wait::Io { deadline: Some(deadline), .. } if deadline <= now => {
        woken.push(id);
        false
    }
    _ => true,
});
```

Leave the trailing `_ => true` — it now covers `Wait::Task`, a future `Deadline`, and an untimed `Io`, all of which correctly stay parked.

- [ ] **Step 6: Restructure the drive loop, deleting the sleep**

Replace `match earliest_deadline() { Some(at) => wake_due_deadlines(at), None => report_deadlock() }` with a match on both dimensions:

```rust
let io: Vec<(RawSocket, Interest)> = io_parks();
match (earliest_deadline(), io.is_empty()) {
    // Nothing can ever wake anything: the only true deadlock.
    (None, true) => report_deadlock(),
    // Waiting on a peer is legitimate waiting, not a deadlock (spec 3.4).
    (None, false) => wake_ready(crate::poll::wait(&io, None)),
    // No sockets and a deadline IS a sleep -- there is no second timing path.
    (Some(at), true) => { crate::poll::wait(&[], Some(at)); }
    (Some(at), false) => wake_ready(crate::poll::wait(&io, Some(at))),
}
wake_due(Instant::now());
```

Then **delete `wake_due_deadlines` entirely**, and confirm `std::thread::sleep` no longer appears in `task.rs`.

- [ ] **Step 7: Implement `poll::wait`'s timer-only path**

Sockets come in Task 2. For now the empty-set case must behave exactly as the old sleep did:

```rust
pub fn wait(sockets: &[(RawSocket, Interest)], deadline: Option<Instant>) -> Vec<RawSocket> {
    if sockets.is_empty() {
        if let Some(at) = deadline {
            let now = Instant::now();
            if at > now {
                std::thread::sleep(at - now);
            }
        }
        return Vec::new();
    }
    // Task 2 replaces this with a real readiness wait.
    Vec::new()
}
```

The sleep moves here rather than vanishing: `task.rs` no longer knows how to wait, which is the point. Task 2 removes this one too.

- [ ] **Step 8: Run the tests**

Run: `cargo build --workspace && cargo test -p nova-runtime --lib --no-fail-fast`
Expected: PASS, including every pre-existing `sleep`, `join` and deadlock test — they exercise the timer path through its new home.

- [ ] **Step 9: Widen staging without loosening the abort**

```rust
/// A poll may stage at most one deadline and at most one I/O wait.
///
/// Two of the *same* kind still aborts: that abort is what catches an inner
/// future's `POLL_PENDING` failing to propagate, and it must keep doing so.
/// Only the one legitimate combination -- a deadline and an I/O wait, from a
/// single `read_timeout` -- becomes newly legal.
#[derive(Default, Clone, Copy, Debug)]
struct Staged {
    deadline: Option<Instant>,
    io: Option<(RawSocket, Interest)>,
}
```

`stage_park` keeps its "outside a poll" abort, and aborts when the slot it is about to fill is already full, naming both waits as it does today.

- [ ] **Step 10: Test the abort still fires, and the new combination does not**

```rust
#[test]
fn a_deadline_and_an_io_wait_stage_together() { /* both staged, no abort */ }

#[test]
#[should_panic(expected = "two parks staged in one poll")]
fn two_deadlines_in_one_poll_still_abort() { /* unchanged behaviour */ }

#[test]
#[should_panic(expected = "two parks staged in one poll")]
fn two_io_waits_in_one_poll_still_abort() { /* the new same-kind case */ }
```

- [ ] **Step 11: Full verification and commit**

Run: `cargo build --workspace`, then `cargo test --workspace --no-fail-fast`, summing the `test result:` lines mechanically. Then clippy `--all-targets --all-features -- -D warnings` and `cargo fmt --all --check`.
Expected: 921 + the new tests, 0 failed, 8 ignored, 44 targets.

Commit message must state that `std::thread::sleep` was deleted from `task.rs` and where the timer path now lives.

---

## Task 2: Real readiness in `poll.rs`

**Files:**
- Modify: `crates/nova-runtime/src/poll.rs`
- Test: `crates/nova-runtime/src/poll.rs` (`mod tests`)

**Interfaces:**
- Consumes: `RawSocket`, `Interest` from Task 1.
- Produces: `poll::wait` now genuinely waits on sockets; `poll::set_nonblocking(RawSocket) -> std::io::Result<()>`.

**This task involves no Nova and no tasks.** It is testable entirely against raw loopback sockets, which is why the platform shim is its own file.

- [ ] **Step 1: Write the failing test — a ready socket is reported**

```rust
#[test]
fn wait_reports_a_socket_with_data_waiting() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind loopback");
    let addr = listener.local_addr().expect("local addr");
    let mut client = std::net::TcpStream::connect(addr).expect("connect");
    let (mut server, _) = listener.accept().expect("accept");
    std::io::Write::write_all(&mut server, b"hi").expect("write");

    let sock = RawSocket(raw_of(&client));
    let ready = wait(&[(sock, Interest::Read)], None);
    assert_eq!(ready, vec![sock], "the client socket has data and must be ready");
    let _ = &mut client;
}
```

`expect` is acceptable **in tests** — the poll-boundary rule binds the poll functions and what they call, not the test module. Do not carry `expect` into `wait` itself.

- [ ] **Step 2: Run it to confirm it fails**

Run: `cargo test -p nova-runtime --lib wait_reports_a_socket_with_data_waiting`
Expected: FAIL — Task 1's `wait` returns an empty `Vec` for any non-empty socket set.

- [ ] **Step 3: Implement the readiness wait**

Use `select` on Unix and `WSAPoll` on Windows behind one `#[cfg]` seam, converting `deadline` to a relative timeout at the call. Return the subset that is ready. **Report, in the task report, which platform primitive each `#[cfg]` arm uses** — the spec records socket pollability off Windows as *reasoned, not measured*, and this task is where it becomes measured.

Rules for this function specifically:
- No `unwrap`/`expect`: it is called from the drive loop, which runs between polls.
- A negative or already-passed deadline means a zero timeout, never a negative one.
- `EINTR` retries rather than being reported as readiness.

- [ ] **Step 4: Run the test**

Run: `cargo test -p nova-runtime --lib wait_reports_a_socket_with_data_waiting`
Expected: PASS.

- [ ] **Step 5: Add the timeout and not-ready tests**

```rust
#[test]
fn wait_returns_empty_when_the_deadline_passes_first() { /* connected, no data, 50ms */ }

#[test]
fn wait_with_no_sockets_and_a_deadline_sleeps_until_it() { /* >= the interval */ }
```

The second pins that Task 1's timer path survived the rewrite — it is the only remaining test of the behaviour `wake_due_deadlines` used to own.

- [ ] **Step 6: Full verification and commit**

Same command sequence and expectations as Task 1 Step 11.

---

## Task 3: The socket table and a non-blocking `connect`

**Files:**
- Create: `crates/nova-runtime/src/net.rs`
- Modify: `crates/nova-runtime/src/lib.rs` (`mod net;` and five `symbols()` entries)

**Interfaces:**
- Consumes: `poll::{RawSocket, Interest, set_nonblocking}`; `fs::{fail, stash, Slot, OK}`; `task::{build_future, stage_park, Wait, PollFn, POLL_PENDING, POLL_READY, STATE_SLOT_TAG, STATE_SLOT_OUTPUT, STATE_SLOT_TEMPS, abort_with}`.
- Produces: `nova_rt_net_connect_future(addr: *const NovaStr) -> *mut u8`, `nova_rt_net_close(fd: i64) -> i64`, and (Task 4) `nova_rt_net_read_future`, `nova_rt_net_write_future`, `nova_rt_net_read_timeout_future`.

**The table is `file.rs`'s, deliberately.** Copy its shape — a `thread_local!` `RefCell<HashMap<i64, TcpStream>>`, a `Cell<i64>` next-fd starting at 1, `try_borrow_mut` with an `abort_with` backstop, absence from the table meaning closedness. Read `file.rs` before writing this; the point is one handle model, not two.

- [ ] **Step 1: Write the failing test — connect, then close, then use-after-close**

```rust
#[test]
fn a_connected_socket_closes_once_and_then_reports_not_open() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
    let addr = listener.local_addr().expect("addr").to_string();
    let fd = connect_blocking_for_test(&addr);
    assert_eq!(unsafe { nova_rt_net_close(fd) }, OK);
    // Idempotent: a second close finds nothing and still succeeds.
    assert_eq!(unsafe { nova_rt_net_close(fd) }, OK);
    // Absence from the table IS closedness.
    assert_ne!(read_status_for_test(fd), OK, "a read on a closed fd must fail");
}
```

- [ ] **Step 2: Run it to confirm it fails**

Run: `cargo test -p nova-runtime --lib a_connected_socket_closes_once_and_then_reports_not_open`
Expected: FAIL to compile — `net.rs` does not exist.

- [ ] **Step 3: Write the table and `close`**

Mirror `file.rs`'s `FILES`/`NEXT_FD`/`with_fd`/`register_new_file`/`closed_fd_error`, renamed for sockets. `closed_fd_error` fabricates `std::io::Error::other("socket is not open")` and routes it through `fs::fail`, which has no `ErrorKind::Other` arm and so falls to its catch-all — no new status constant, no second mapping table.

- [ ] **Step 4: Write the two-phase `connect` future**

```rust
/// Where the connect future keeps its socket between polls. One past the last
/// slot the ABI reserves, so the state object is one word larger than
/// `STATE_MIN_SIZE` -- the same arrangement `SLEEP_SLOT_MS` uses.
const CONNECT_SLOT_SOCK: usize = STATE_SLOT_TEMPS;

unsafe extern "C-unwind" fn poll_connect(state: *mut u8, _task_ctx: *mut u8) -> i64 {
    let slots = state as *mut i64;
    let tag = unsafe { slots.add(STATE_SLOT_TAG).read() };
    if tag == 0 {
        unsafe { slots.add(STATE_SLOT_TAG).write(1) };
        // Non-blocking connect: expect would-block, park on WRITE readiness.
        // A blocking connect would pass every loopback test while defeating
        // this increment -- see the spec's section 4.
        match start_connect(slots) {
            Started::WouldBlock(sock) => {
                stage_park(Wait::Io { socket: sock, interest: Interest::Write, deadline: None });
                return POLL_PENDING;
            }
            Started::Failed(status) => {
                unsafe { slots.add(STATE_SLOT_OUTPUT).write(status) };
                return POLL_READY;
            }
        }
    }
    // Second poll: the socket is write-ready; SO_ERROR says whether the
    // connection actually established or was refused.
    let status = finish_connect(slots);
    unsafe { slots.add(STATE_SLOT_OUTPUT).write(status) };
    POLL_READY
}
```

`finish_connect` maps a refused connection to `CONNECTION_REFUSED` — a status constant that already exists and has never had a producer.

- [ ] **Step 5: Run the tests**

Run: `cargo build --workspace && cargo test -p nova-runtime --lib net::`
Expected: PASS.

- [ ] **Step 6: Test a refused connection**

```rust
#[test]
fn connecting_to_a_closed_port_is_connection_refused() {
    // Bind, read the port, then drop the listener so nothing is listening.
    // The kernel refuses rather than hanging, on all three CI platforms.
    assert_eq!(connect_status_for_test(&dead_addr()), CONNECTION_REFUSED);
}
```

- [ ] **Step 7: Register the symbols**

Add all five `nova_rt_net_*` names to `symbols()` in `lib.rs`. **That map is keyed by the lowercase symbol string and is invisible to a PascalCase grep**; a missing entry fails only for programs that actually call the intrinsic, because `finalize_definitions` resolves only referenced symbols. `nova_rt_task_*` once shipped in exactly that state.

- [ ] **Step 8: Full verification and commit**

Same sequence and expectations as Task 1 Step 11.

---

## Task 4: `read`, `write`, and `read_timeout`

**Files:**
- Modify: `crates/nova-runtime/src/net.rs`

**Interfaces:**
- Produces: `nova_rt_net_read_future(fd: i64, max: i64) -> *mut u8`, `nova_rt_net_write_future(fd: i64, bytes: *const NovaStr) -> *mut u8`, `nova_rt_net_read_timeout_future(fd: i64, max: i64, ms: i64) -> *mut u8`. Payloads ride `Slot::Buffer`; counts use the 8-little-endian-byte encoding `io.rs`'s `stash_count` uses — **read that function rather than inventing a second encoding.**

- [ ] **Step 1: Write the failing test — a read parks and then yields the bytes**

```rust
#[test]
fn a_read_with_no_data_parks_and_completes_when_data_arrives() {
    // First poll: nothing to read, so the future must park rather than spin.
    assert_eq!(poll_once(&fut), POLL_PENDING);
    assert!(staged_io_park().is_some(), "a read with no data must stage a park");
    write_from_the_other_end(b"hello");
    assert_eq!(poll_once(&fut), POLL_READY);
    assert_eq!(taken_bytes(), b"hello");
}
```

- [ ] **Step 2: Run it to confirm it fails**

Run: `cargo test -p nova-runtime --lib a_read_with_no_data_parks_and_completes_when_data_arrives`
Expected: FAIL to compile — the future does not exist.

- [ ] **Step 3: Implement the read future**

The tag-based shape, with `poll_join`'s optimisation: if data is already waiting, complete on the first poll without parking at all.

**Two contracts inherited from `std/io` and not to be re-decided:** an **empty** result means EOF, and a **short read is not EOF** — the test is `len() == 0`, never `len() < max`. Getting it backwards truncates silently rather than hanging (measured twice on the 3b branch).

- [ ] **Step 4: Implement the write future** — same shape, parking on `Interest::Write`. `Write::write` may write **fewer bytes than given**; the count rides `Slot::Buffer`.

- [ ] **Step 5: Implement `read_timeout`**

The only operation that stages both: `Wait::Io { socket, interest: Interest::Read, deadline: Some(at) }`. On a wake it must distinguish ready from timed out. **The poll ABI is frozen and `task_ctx` is always null**, so the reason travels through the per-task slot table increment 3a built — the same channel payloads already use.

A passed deadline yields `TIMED_OUT`, a status constant that already exists and has never had a producer.

- [ ] **Step 6: Test the timeout**

```rust
#[test]
fn a_read_timeout_against_a_silent_peer_reports_timed_out() {
    // A connected socket the far end never writes to.
    assert_eq!(read_timeout_status_for_test(fd, 64, 50), TIMED_OUT);
}
```

- [ ] **Step 7: Full verification and commit** — same sequence as Task 1 Step 11.

---

## Task 5: Wire five builtins through the ten seams

**Files:**
- Modify: `crates/nova-resolver/src/lib.rs` (`Builtin` variant list, `name()`, `STD_ONLY`)
- Modify: `crates/nova-typeck/src/check.rs` (`builtin_signature`, `check_builtin_call`, the coverage test's arms)
- Modify: `crates/nova-mir/src/lib.rs` (`RtFunc` variant list, `symbol()`, `signature()`)
- Modify: `crates/nova-mir/src/lower.rs` (`lower_call`'s `Builtin` → `RtFunc` match)

**Interfaces:**
- Consumes: the five `nova_rt_net_*` symbols from Tasks 3–4.
- Produces: Nova-visible `net_connect`, `net_close`, `net_read`, `net_write`, `net_read_timeout`, all `STD_ONLY`.

**Seam 10 (`symbols()`) was closed in Task 3**, so this task has seams 1–9. Increment 3c's Task 2 is the worked example — read its commit before starting.

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn the_net_builtins_are_std_only_and_build_futures() {
    for b in [Builtin::NetConnect, Builtin::NetClose, Builtin::NetRead,
              Builtin::NetWrite, Builtin::NetReadTimeout] {
        assert!(STD_ONLY.contains(&b), "{b:?} must be invisible to user code");
    }
}
```

- [ ] **Step 2: Run it to confirm it fails** — `cargo test -p nova-resolver the_net_builtins_are_std_only`. Expected: FAIL to compile, no such variants.

- [ ] **Step 3: Close seams 1–3** (nova-resolver), **4–5** (nova-typeck), **6–8** (nova-mir/lib.rs), **9** (lower.rs).

The future-building intrinsics return `Ptr`, not a status word — they are constructors, unlike every `fs_*`/`io_*`/`file_*` intrinsic before them. `net_close` alone returns a status. **State that asymmetry in the doc comment**; a reader who has just read `file.rs` will expect the old shape.

- [ ] **Step 4: Extend the exhaustive coverage test**

`builtin_signatures_are_what_the_std_call_sites_use` matches every `Builtin`. Its second tuple element names the call site a mismatch would break — **point the five new arms at their real `std/net` call sites**, which Task 6 creates. Do not write "no call site yet"; five such strings went stale on the previous increment and had to be repaired.

- [ ] **Step 5: Run, verify, commit** — `STD_ONLY` 53 → 58; `STD_MODULES` and `RESERVED_TYPE_NAMES` both still 7 at this point. Confirm all three in source and state them in the commit body.

---

## Task 6: `std/net` and the fixtures that prove suspension

**Files:**
- Create: `std/net/lib.nova`
- Modify: `crates/nova-resolver/src/lib.rs` (`STD_MODULES` 7 → 8)
- Create: `tests/runtime/net_roundtrip.nova`, `net_interleave.nova`, `net_timeout.nova`, `net_refused.nova`, `net_lifetime.nova` and their `.stdout`
- Modify: `crates/nova-cli/tests/run_tests.rs` (the echo harness and five runners)

**Interfaces:**
- Consumes: the five builtins from Task 5.
- Produces: `pub record TcpStream { fd: Int }`, `pub async fn connect(addr: String) -> Result<TcpStream, IoError>`, `pub async fn close(s: TcpStream) -> Result<(), IoError>`, `pub async fn read_timeout(s: TcpStream, max: Int, ms: Int) -> Result<Bytes, IoError>`, `impl Read for TcpStream`, `impl Write for TcpStream`.

- [ ] **Step 1: Add `std/net` to `STD_MODULES`**

```rust
("$std.net", include_str!("../../../std/net/lib.nova")),
```

**A directory under `std/` is not automatically a module** — `std/test` is a directory and is *not* an `STD_MODULES` entry, so both the file and this line are needed. The array's length annotation goes 7 → 8.

Because `STD_ONLY` builtins are seeded into each std module's own scope *before* its items are collected, **`std/net` must not define anything sharing a builtin's name** or it hits `E0002` against the pre-seeded entry.

- [ ] **Step 2: Write `std/net/lib.nova`**

The trait methods use the `Future<T>` spelling — `async fn` in a trait declaration is a hard `E0900`:

```nova
impl Read for TcpStream {
    fn read(self, max: Int) -> Future<Bytes> { net_read_fut(self, max) }
}
```

where `net_read_fut` is a private `async fn` whose future is returned **unawaited**. `impl Trait` in return position does not parse (`P0001`), so `TcpStream` is a concrete record.

- [ ] **Step 3: Write the echo harness**

In `run_tests.rs`: bind `127.0.0.1:0`, take the ephemeral port, write it to a path derived from the test name, spawn a thread that accepts once and echoes, and shut down after the fixture exits.

**State the run-invariant race in the fixture's own comment.** The path is derived from the test name rather than the process, so two concurrent cargo runs of the same test would collide — the same latent hazard the debt queue records for `write_test_project`. A fixed port would be flakier and a generated fixture would break the static-file-plus-golden convention every other runtime fixture follows.

- [ ] **Step 4: The interleaving fixture — the one that decides the increment**

```nova
// A blocking `connect`, a blocking read, and POLL_READY-where-POLL_PENDING-
// belongs ALL produce correct output against an echo server. A round-trip
// fixture passes under every one of them. Only this fixture can tell a real
// poller from a blocking one: it asserts that the counter task ran WHILE the
// socket task was waiting.
async fn main() {
    let reader = spawn(read_after_connecting())
    let counter = spawn(count_to_three())
    ...
}
```

Its golden must show the counter's output **interleaved** with the socket task's, not merely present.

- [ ] **Step 5: The remaining four fixtures** — round-trip, timeout, refused, and lifetime (idempotent `close`, use-after-close, and a forged `TcpStream { fd: 9999 }`, which is constructible because Nova has no field privacy).

- [ ] **Step 6: Run all six mandated mutations**

Each must fail at least one test, and **mutation 6 must fail the interleaving fixture specifically** — that is what proves the fixture does its job:

1. `stage_park` deleted from the read poll.
2. Read/write interest transposed at registration.
3. `POLL_PENDING` → `POLL_READY` on would-block.
4. `read_timeout`'s deadline not honoured.
5. A task parked on I/O with no deadline reporting a deadlock.
6. `connect` made blocking.

**Stage before reverting** — a `git checkout` on an unstaged file destroyed an implementer's own work on the previous increment, caught only by `git hash-object`. Report a transcript per mutation.

- [ ] **Step 7: Full verification and commit** — `STD_MODULES` now 8, `RESERVED_TYPE_NAMES` still 7. Confirm both in source.

---

## Task 7: The records

**Files:** `nova-spec/20-STDLIB.md`, `docs/adr/0009-async-execution-model.md`, a new `docs/adr/0013-io-poller.md`, `CHANGELOG.md`, `crates/nova-runtime/src/fs.rs`

- [ ] **Step 1: A new ADR for the poller** — the third wake source; why threads and IOCP were both declined (`PARKED`, `QUEUE`, `SLOTS`, `FILES` and the GC roots are all `thread_local`, and `stack_base()` returns `None` off Windows); and why the wait happens only at the drained-queue point. **Check the next free ADR number rather than assuming 0013.**

- [ ] **Step 2: Amend ADR 0009 §1 with the two new footguns** — a permanently-runnable task starves I/O, and an I/O wait is never reported as a deadlock so a program waiting on a silent peer hangs. Both join the existing family; append a dated note, do not rewrite.

- [ ] **Step 3: `nova-spec/20-STDLIB.md`** — a `std/net` section, and a §4 note on the fs-vs-net asymmetry: after this increment `std/fs` suspends nowhere and `std/net` suspends everywhere, with two different wrapper shapes visible in the stdlib.

- [ ] **Step 4: `fs.rs`'s pinned-kinds comment, four → six.** `TimedOut` and `ConnectionRefused` now have producers. **Verify each of the six against its fixture** rather than trusting this line.

- [ ] **Step 5: `CHANGELOG.md`** under `### Added`. Read `### Changed`'s stated scope before filing anything there.

- [ ] **Step 6: The claim sweep**

Grep every changed file's **added** lines for `always`, `every`, `only`, `any`, `never`, `all`, `cannot`, `nothing`, `the first`, `impossible` **and the counting words `once`, `exactly`, `unique`, `single`, `both`, `two`…`ten`**. For each hit, **name the file that would falsify it and open that file** — a claim whose falsifier you cannot name is itself a finding. Check the forward half as well as the backward half: a sentence saying something is true "until now" asserts a present state.

Then **the opposite direction**: this branch's changes as predicate, the corpus as subject. When an edit adds declarations to a spec section, the right search key is **"who counts, enumerates, quotes, or cross-references that section"** — not the identifier. A sentence counting `nova-spec` §5's declarations was falsified by the previous increment and never names the type involved.

Run all three rendering instruments over every changed file: per-line **backtick parity**, `grep -nE '[A-Za-z]-$'`, and `grep -nE '/$'`. The first catches a code span broken across a newline; the third catches a path broken after a slash, which the second cannot see.

For any sentence dating or attributing a change, run `git log -S` on the changed string and check it agrees. **Cite no SHA that is not yet on `main`.**

- [ ] **Step 7: Full verification and commit.**

---

## Self-Review

**Spec coverage.** §1's poller → Tasks 1–2. §2's surface and handle model → Tasks 3, 6. §2's "no new error kind" → Tasks 3–4 (the two producers) and Task 7 Step 4 (the count). §3.1's unified wait → Task 1 Step 6. §3.2's variant and three sites → Task 1 Steps 3–5. §3.3's staging → Task 1 Steps 9–10. §3.4's deadlock ruling → Task 1 Steps 1 and 6. §3.5's slot channel → Task 4 Step 5. §4's parker shape → Tasks 3–4; its seams → Task 5. §5's tests → Task 6, mutations at Step 6. §6's alternatives → declined, not built; recorded in Task 7 Step 1. §7's definition of done → distributed.

**Placeholder scan.** No "TBD", no "add error handling", no "similar to Task N". Three steps ask for a decision plus a report rather than prescribing: which platform primitive each `#[cfg]` arm uses (Task 2 Step 3), the next free ADR number (Task 7 Step 1), and the six pinned kinds' verification (Task 7 Step 4). Each says what to report.

**Type consistency.** `RawSocket` and `Interest` are defined in `poll.rs` in Task 1 and used unchanged in `task.rs` and `net.rs`. The Nova-visible names (`net_connect`, `net_close`, `net_read`, `net_write`, `net_read_timeout`) and the Rust symbols (`nova_rt_net_*`) are spelled consistently and never interchanged — the Nova level appears only in `std/net` and the seam tables, the `nova_rt_` level only in `symbols()` and `RtFunc`. `STD_ONLY` 53 → 58; `STD_MODULES` 7 → 8 in Task 6 Step 1 only; `RESERVED_TYPE_NAMES` 7 throughout.

**One risk carried deliberately.** Task 2 makes a claim the spec records as *reasoned, not measured* — that sockets are cleanly readiness-pollable on all three CI platforms. If a platform disagrees, Task 2 is told to report it rather than work around it: a fallback changes the execution model and is a design decision, not an implementation detail.
