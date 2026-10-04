# `examples/03-http-server` Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship `examples/03-http-server`, the Phase 2 gate in `nova-spec/60-EXAMPLES.md` §3. It serves `Hello from Nova!` on port 3000 and exits cleanly on SIGTERM: stop accepting, finish in-flight requests, exit 0.

**Architecture:**
- **Runtime.** A process-wide `AtomicBool`, set by a SIGTERM/SIGINT handler (Unix) or a console-control handler (Windows), is read through one new `STD_ONLY` builtin, `shutdown_requested()`. Its first call installs the handler. The executor is not changed.
- **`std/http`.** It gains a `Server` written in Nova. `listen` polls the flag every 100 ms between bounded accepts, and each connection bounds every read and write by a 10 s per-request deadline.
- **The example.** It is the spec listing rewritten in today's Nova.

**Tech Stack:** Rust (the `nova-runtime`, `nova-resolver`, `nova-typeck` and `nova-mir` crates; `libc` on Unix; `windows-sys` 0.61 on Windows), Nova (`std/http/lib.nova`, the example), libtest end-to-end tests in `crates/nova-cli/tests/run_tests.rs`.

**Spec:** `docs/superpowers/specs/2026-10-04-examples-03-http-server-design.md`, approved by the user on 2026-10-04. Read it before Task 1. It is the authority this plan argues from.

## Global Constraints

- **"Cleanly" means graceful:** stop accepting, finish requests already in flight, exit 0 (spec §1).
- **Inputs:** the first SIGTERM or SIGINT (Unix), or CTRL_BREAK or CTRL_C (Windows), sets the flag, unless that input was inherited as ignored. Any later one takes the default action (spec §4.2).
- **Opt-in and permanent.** Only `Server::listen` calls `shutdown_requested()`, once before `bind`. The handler is never removed, and the flag is never cleared (spec §4.2).
- **The handler touches only the atomic.** No executor, socket-table or heap state (spec §4.1).
- **No executor change.** `task.rs`, `poll.rs` and `net.rs` keep their behaviour (spec §1).
- **No public signal API.** `shutdown_requested` is `STD_ONLY` (spec §1).
- **Timing.** `TICK` is 100 ms and not configurable. A request's deadline is 10 s from when `serve` takes it up, and it bounds the reads and the response's write (spec §4.3).
- **Routes and JSON.** GET routes only. `Response::json` returns 200 with `content-type: application/json` and `content-length` (spec §4.3).
- **Pipelining.** `Server` answers pipelined requests in order, and answers requests already buffered even after the stop (spec §4.3).
- **The example.** `examples/03-http-server`, beside `examples/03-producer-consumer`, binding `0.0.0.0:3000`. The spec listing stays unchanged, as aspiration (spec §2, §5).
- **End-to-end tests use `nova run` only.** Every test that runs the example holds one `PORT_3000` mutex (spec §7.3).
- **Dependencies.** The only change is `windows-sys`'s `Win32_System_Console` feature, and `Cargo.lock` must not change: check it with `git diff --exit-code Cargo.lock`. The test helpers declare `kill` and `GenerateConsoleCtrlEvent` by hand.
- **Toolchain and CI.** Edition 2021, MSRV 1.78. CI runs `cargo test --locked --workspace --all-features --no-fail-fast` and `cargo clippy --locked --all-targets --all-features -- -D warnings` on ubuntu, windows and macOS. Both must pass.
- **Dated notes** carry the date they are written. Every note text below says `2026-10-04`. If you execute on a later day, change that date in every note you write, and leave the dates that refer to past decisions as they are.

## Review Focus

The five failure modes the spec implies that are most likely to bite a user, each with the test that pins it:

1. **A client that stalls mid-request across the stop.** It must not hold the process past its 10 s deadline, and the process must still exit 0 → `http_server_example_drops_a_stalled_request_at_its_deadline` (Task 4).
2. **A program that never calls `Server::listen`.** It must keep the default signal action → `a_server_that_never_calls_listen_keeps_the_default_signal_action` (Task 4).
3. **A request with a body on a keep-alive connection.** The body must be consumed exactly, and the next request on that connection served → step in `http_server_example_serves_and_exits_cleanly` (Task 4).
4. **A malformed request.** That connection is closed without a response, and the server keeps serving others → step in `http_server_example_serves_and_exits_cleanly` (Task 4).
5. **Port 3000 already taken.** The example fails fast with a nonzero exit and the unwrap message, rather than hanging → `http_server_example_fails_fast_when_the_port_is_taken` (Task 4).

## File Structure

| File | Task | Responsibility |
|---|---|---|
| `crates/nova-runtime/src/signal.rs` (new) | 1 | The flag, the one-time install, the Unix and Windows handlers, and the decision logic with its unit tests |
| `crates/nova-runtime/src/lib.rs` | 1 | `pub mod signal;` and the `symbols()` entry |
| `crates/nova-runtime/Cargo.toml` | 1 | `windows-sys` feature `Win32_System_Console` |
| `crates/nova-runtime/tests/signal_sigterm.rs`, `signal_sigint.rs`, `signal_console.rs` (new) | 1 | One process-global scenario per test binary |
| `docs/adr/0022-process-shutdown-signals.md` (new) | 1 | The decision record |
| `crates/nova-resolver/src/lib.rs`, `crates/nova-typeck/src/check.rs`, `crates/nova-mir/src/lib.rs`, `crates/nova-mir/src/lower.rs` | 2 | The `shutdown_requested` builtin's registration |
| `std/http/lib.nova` | 3 | `Response::json`, `Server`, `serve`, `read_in_flight`, `read_more`, `head_window`, and the comment rewrites |
| `std/net/lib.nova` | 3 | `accept`'s "No timeout" comment |
| `tests/runtime/http_server_dispatch.nova` and `.stdout` (new) | 3 | Socket-free dispatch and `Response::json` fixture |
| `examples/03-http-server/src/main.nova`, `README.md` (new) | 4 | The example and its §9 README |
| `crates/nova-cli/tests/run_tests.rs` | 3, 4 | Fixture registration; the end-to-end tests and their helpers |
| Records (Task 5 lists them) | 5 | Dated notes, the CHANGELOG, the sweep |

## Conventions for every task

- **Working directory.** `D:\Projects\nona\nova`, which is Git Bash `/d/Projects/nona/nova`. The Bash tool resets its directory after each call, so write `cd /d/Projects/nona/nova && …` in one command.
- **Line endings.** The working tree is CRLF (`core.autocrlf=true`). Any edit tool is fine. A script that matches multi-line text must convert its anchors to the file's own newline. The scripts below do that, and abort before writing anything if an anchor does not match exactly once.
- **Write scripts with the Write tool, never a Bash heredoc.** The Bash tool here turns `\\` into `\`.
  - Put them in `C:/Users/SAKEER~1/AppData/Local/Temp/gcm/_once/`.
  - Run them with `python -X utf8 <path>`.
  - Rename each to `<name>.applied` once it has run.
- **Stale runtime library.** The `*_build_standalone` tests in `run_tests.rs` link `target/debug/nova_runtime.lib`. Run `cargo build -p nova-runtime` before any `nova-cli` test run that follows a runtime change.
- **Long output** goes to a file in the scratchpad. Read its tail.

---

### Task 1: The runtime shutdown flag (`signal.rs`) and ADR 0022

**Files:**
- Create: `crates/nova-runtime/src/signal.rs`
- Create: `crates/nova-runtime/tests/signal_sigterm.rs`, `crates/nova-runtime/tests/signal_sigint.rs`, `crates/nova-runtime/tests/signal_console.rs`
- Create: `docs/adr/0022-process-shutdown-signals.md`
- Modify: `crates/nova-runtime/src/lib.rs`, at the module declarations (the `mod poll;` / `pub mod task;` region) and in `symbols()` after the `nova_rt_log_set_config` entry
- Modify: `crates/nova-runtime/Cargo.toml`, the `[target.'cfg(windows)'.dependencies]` line

**Interfaces:**
- Produces:
  - `nova_runtime::signal::nova_rt_shutdown_requested() -> i8`: `#[no_mangle] pub extern "C"`, 1 if set, 0 if not; its first call installs the handler.
  - On Windows only, `nova_runtime::signal::console_handler(event: u32) -> windows_sys::core::BOOL`: `pub extern "system"`.
  - The `symbols()` entry `("nova_rt_shutdown_requested", …)`.

- [ ] **Step 1: Write the three integration tests (one scenario per binary)**

Create `crates/nova-runtime/tests/signal_sigterm.rs`:

```rust
//! One scenario, one process: SIGTERM sets the shutdown flag without ending
//! the process, and a SIGINT the process already ignored stays ignored.
//!
//! Its own test binary because the handler's install and the flag are
//! process-global and permanent (see `nova_runtime::signal`), so no other
//! scenario may share this process.
#![cfg(unix)]

use nova_runtime::signal::nova_rt_shutdown_requested;

#[test]
fn sigterm_sets_the_flag_and_an_ignored_sigint_stays_ignored() {
    // SAFETY: setting a disposition to `SIG_IGN` touches no memory.
    unsafe { libc::signal(libc::SIGINT, libc::SIG_IGN) };
    assert_eq!(
        nova_rt_shutdown_requested(),
        0,
        "the first read installs the handler and finds the flag clear"
    );

    // SAFETY: `old` is a valid, writable `sigaction` for the call's duration.
    let mut old: libc::sigaction = unsafe { std::mem::zeroed() };
    unsafe { libc::sigaction(libc::SIGINT, std::ptr::null(), &mut old) };
    assert_eq!(
        old.sa_sigaction,
        libc::SIG_IGN,
        "an inherited SIGINT ignore must survive the install"
    );

    // SAFETY: SIGTERM now has a handler that only stores an atomic.
    unsafe { libc::raise(libc::SIGTERM) };
    assert_eq!(
        nova_rt_shutdown_requested(),
        1,
        "SIGTERM sets the flag, and the process survives it"
    );
}
```

Create `crates/nova-runtime/tests/signal_sigint.rs`:

```rust
//! One scenario, one process: a SIGINT that was not ignored sets the shutdown
//! flag without ending the process. Its own test binary for the reason
//! `signal_sigterm.rs` gives.
#![cfg(unix)]

use nova_runtime::signal::nova_rt_shutdown_requested;

#[test]
fn sigint_sets_the_flag_when_it_was_not_ignored() {
    // SAFETY: setting a disposition to `SIG_DFL` touches no memory.
    unsafe { libc::signal(libc::SIGINT, libc::SIG_DFL) };
    assert_eq!(
        nova_rt_shutdown_requested(),
        0,
        "the first read installs the handler and finds the flag clear"
    );

    // SAFETY: SIGINT now has a handler that only stores an atomic.
    unsafe { libc::raise(libc::SIGINT) };
    assert_eq!(
        nova_rt_shutdown_requested(),
        1,
        "SIGINT sets the flag, and the process survives it"
    );
}
```

Create `crates/nova-runtime/tests/signal_console.rs`:

```rust
//! One scenario, one process: the Windows console handler sets the shutdown
//! flag on the first CTRL_C, and lets every later event fall through to the
//! default action. Its own test binary for the reason `signal_sigterm.rs`
//! gives. The routine is called directly: delivering a real console event to
//! this process would also reach cargo.
#![cfg(windows)]

use nova_runtime::signal::{console_handler, nova_rt_shutdown_requested};

#[test]
fn ctrl_c_sets_the_flag_and_a_later_event_falls_through() {
    assert_eq!(
        nova_rt_shutdown_requested(),
        0,
        "the first read installs the handler and finds the flag clear"
    );
    assert_eq!(console_handler(0), 1, "CTRL_C_EVENT is handled the first time");
    assert_eq!(nova_rt_shutdown_requested(), 1, "and it set the flag");
    assert_eq!(
        console_handler(1),
        0,
        "CTRL_BREAK_EVENT with the flag already set falls through to the default action"
    );
    assert_eq!(console_handler(2), 0, "CTRL_CLOSE_EVENT always falls through");
}
```

- [ ] **Step 2: Run them to see them fail**

Run: `cd /d/Projects/nona/nova && cargo test -p nova-runtime --test signal_console 2>&1 | tail -20`
Expected: a compile error, `error[E0432]: unresolved import nova_runtime::signal` (or `E0433`), because the module does not exist yet. That is the RED.

- [ ] **Step 3: Write `crates/nova-runtime/src/signal.rs`**

```rust
//! The process-wide shutdown flag behind `std/http`'s graceful
//! `Server::listen`.
//!
//! One `AtomicBool`, set by the first termination request the process
//! receives: SIGTERM or SIGINT on Unix, CTRL_BREAK or CTRL_C on Windows. A
//! request that arrives once the flag is already set takes the signal's
//! default action instead, so a stalled shutdown can always be ended.
//!
//! **Opt-in, and permanent once in.** Nothing is installed until the first
//! call to [`nova_rt_shutdown_requested`], which only `std/http`'s `Server`
//! makes: the builtin `shutdown_requested` is `STD_ONLY`. A program that never
//! calls it keeps every signal's default action. Once installed, the handler
//! stays for the life of the process, and the flag is never cleared.
//!
//! **The handler touches nothing but [`SHUTDOWN`].** On Unix it runs in signal
//! context, where only async-signal-safe calls are allowed. On Windows the OS
//! runs it on a thread of its own, where this crate's `thread_local!` state
//! (the executor, the socket table, the heap) is a different, empty copy. So
//! it never wakes the executor: `poll.rs` retries an interrupted wait, and Nova
//! code sees the flag at its next deadline. `Server::listen` provides one
//! every 100 ms. `docs/adr/0022-process-shutdown-signals.md` records the
//! decision and the alternatives it rejected.
//!
//! **An inherited ignore stays ignored.** A process started with SIGINT
//! ignored (a job backgrounded by a non-interactive shell) keeps ignoring it.
//! Windows' inherited Ctrl-C ignore flag keeps CTRL_C from reaching any
//! handler at all. SIGTERM and CTRL_BREAK are not affected.
//!
//! `pub`, unlike [`crate::time`], so the integration tests under
//! `crates/nova-runtime/tests/` can drive it. Each process-global scenario
//! there is its own test binary, because neither the install nor the flag can
//! be undone within one process.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Once;

/// Whether the process has been asked to stop.
static SHUTDOWN: AtomicBool = AtomicBool::new(false);

/// What a handler does with one termination request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Action {
    /// The first request: set [`SHUTDOWN`] and carry on.
    SetFlag,
    /// A later one: take the signal's default action.
    Default,
}

/// The first request sets the flag; any later one takes the default action.
fn action(already_set: bool) -> Action {
    if already_set {
        Action::Default
    } else {
        Action::SetFlag
    }
}

/// Windows console control events, numbered as `windows-sys` numbers them
/// (checked against it at compile time below). Plain constants, so the
/// decision logic's unit tests run on every operating system.
#[cfg(any(windows, test))]
const CTRL_C: u32 = 0;
#[cfg(any(windows, test))]
const CTRL_BREAK: u32 = 1;

#[cfg(windows)]
const _: () = {
    use windows_sys::Win32::System::Console::{CTRL_BREAK_EVENT, CTRL_C_EVENT};
    assert!(CTRL_C == CTRL_C_EVENT && CTRL_BREAK == CTRL_BREAK_EVENT);
};

/// Whether a console control event asks for a graceful shutdown. Close,
/// logoff and shutdown events keep the default action.
#[cfg(any(windows, test))]
fn is_termination_event(event: u32) -> bool {
    event == CTRL_C || event == CTRL_BREAK
}

/// `shutdown_requested() -> Bool`: installs the handler on the first call,
/// then reports whether the flag is set, as `1` or `0`.
#[no_mangle]
pub extern "C" fn nova_rt_shutdown_requested() -> i8 {
    install();
    i8::from(SHUTDOWN.load(Ordering::SeqCst))
}

/// Installs the handler, once per process.
fn install() {
    static INSTALL: Once = Once::new();
    INSTALL.call_once(install_handler);
}

#[cfg(unix)]
fn install_handler() {
    // SAFETY: an all-zero `sigaction` is a valid value (no flags, empty mask,
    // `SIG_DFL`). Every field set below is a plain integer, or the address of
    // `unix_handler`, whose signature is the one `sa_sigaction` expects when
    // `SA_SIGINFO` is not set. `sigaction` reads `sa` and writes `old` only
    // for the duration of each call.
    unsafe {
        let mut sa: libc::sigaction = std::mem::zeroed();
        sa.sa_sigaction = unix_handler as extern "C" fn(libc::c_int) as libc::sighandler_t;
        sa.sa_flags = libc::SA_RESTART;
        libc::sigemptyset(&mut sa.sa_mask);
        libc::sigaction(libc::SIGTERM, &sa, std::ptr::null_mut());
        let mut old: libc::sigaction = std::mem::zeroed();
        libc::sigaction(libc::SIGINT, std::ptr::null(), &mut old);
        if old.sa_sigaction != libc::SIG_IGN {
            libc::sigaction(libc::SIGINT, &sa, std::ptr::null_mut());
        }
    }
}

/// The SIGTERM and SIGINT handler. Only async-signal-safe work: an atomic
/// swap and, on a second signal, `signal` and `raise`, both on POSIX's list.
#[cfg(unix)]
extern "C" fn unix_handler(sig: libc::c_int) {
    if action(SHUTDOWN.swap(true, Ordering::SeqCst)) == Action::Default {
        // SAFETY: restores `sig`'s default disposition and raises it again.
        // `sig` is blocked while this handler runs, so the raised signal is
        // delivered as the handler returns, with the default action.
        unsafe {
            libc::signal(sig, libc::SIG_DFL);
            libc::raise(sig);
        }
    }
}

#[cfg(windows)]
fn install_handler() {
    use windows_sys::Win32::System::Console::SetConsoleCtrlHandler;
    // SAFETY: `console_handler` has the `PHANDLER_ROUTINE` signature, and as a
    // plain function it outlives the registration.
    let _ = unsafe {
        SetConsoleCtrlHandler(
            Some(console_handler as unsafe extern "system" fn(u32) -> windows_sys::core::BOOL),
            1,
        )
    };
}

/// The console control routine [`install`] registers on Windows. It returns
/// TRUE (handled) for the first CTRL_C or CTRL_BREAK and FALSE for anything
/// else, so the next routine runs instead: the default one, which ends the
/// process with `0xC000013A`.
///
/// The OS calls it on a thread of its own. It touches only [`SHUTDOWN`], an
/// atomic, so it is safe on any thread. `pub` so `tests/signal_console.rs` can
/// call it directly.
#[cfg(windows)]
pub extern "system" fn console_handler(event: u32) -> windows_sys::core::BOOL {
    if !is_termination_event(event) {
        return 0;
    }
    i32::from(action(SHUTDOWN.swap(true, Ordering::SeqCst)) == Action::SetFlag)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_first_request_sets_the_flag_and_a_later_one_takes_the_default_action() {
        assert_eq!(action(false), Action::SetFlag);
        assert_eq!(action(true), Action::Default);
    }

    #[test]
    fn only_ctrl_c_and_ctrl_break_ask_for_a_graceful_shutdown() {
        // CTRL_CLOSE_EVENT, CTRL_LOGOFF_EVENT and CTRL_SHUTDOWN_EVENT are 2, 5
        // and 6 in `windows-sys`.
        for (event, expected) in [
            (CTRL_C, true),
            (CTRL_BREAK, true),
            (2, false),
            (5, false),
            (6, false),
        ] {
            assert_eq!(is_termination_event(event), expected, "event {event}");
        }
    }
}
```

- [ ] **Step 4: Declare the module and add the Windows feature**

In `crates/nova-runtime/src/lib.rs`, insert between the `mod poll;` declaration (and its doc comment) and the `pub mod task;` doc comment:

```rust
/// The process-wide shutdown flag `std/http`'s `Server::listen` polls (see
/// its module doc and `docs/adr/0022-process-shutdown-signals.md`). `pub`,
/// unlike [`time`], so `tests/signal_*.rs` can drive it.
pub mod signal;
```

In `crates/nova-runtime/Cargo.toml`, change:

```toml
windows-sys = { version = "0.61", features = ["Win32_Networking_WinSock"] }
```

to:

```toml
windows-sys = { version = "0.61", features = ["Win32_Networking_WinSock", "Win32_System_Console"] }
```

Also add one comment line above it:

```toml
# `Win32_System_Console` is `signal.rs`'s `SetConsoleCtrlHandler`. Until now it
# was reachable only through another dependency's feature unification.
```

- [ ] **Step 5: Run the tests to see them pass**

Run: `cd /d/Projects/nona/nova && cargo test -p nova-runtime --lib signal 2>&1 | tail -8 && cargo test -p nova-runtime --test signal_console --test signal_sigterm --test signal_sigint 2>&1 | tail -15`
Expected:
- Both unit tests pass.
- On Windows, `signal_console`'s one test passes.
- `signal_sigterm` and `signal_sigint` build as empty test binaries, `0 passed`, because they are `#![cfg(unix)]`.

- [ ] **Step 6: Register the symbol with the JIT**

In `crates/nova-runtime/src/lib.rs`, in `symbols()`, after the entry

```rust
        (
            "nova_rt_log_set_config",
            log::nova_rt_log_set_config as *const u8,
        ),
```

add:

```rust
        (
            "nova_rt_shutdown_requested",
            signal::nova_rt_shutdown_requested as *const u8,
        ),
```

- [ ] **Step 7: Lint, try the Unix arm, check the lockfile**

Run: `cd /d/Projects/nona/nova && cargo clippy -p nova-runtime --all-targets --all-features -- -D warnings 2>&1 | tail -5 && git diff --exit-code Cargo.lock && echo LOCK-UNCHANGED`
Expected: no warnings, then `LOCK-UNCHANGED`.

Run: `cd /d/Projects/nona/nova && cargo check -p nova-runtime --tests --target x86_64-unknown-linux-gnu 2>&1 | tail -8`
Expected: either `Finished`, which type-checks the Unix arm here, or a `cc` failure compiling `src/gc_stack.c` for want of a Linux C compiler. If it is the `cc` failure, record in your report that the Unix arm is first compiled by CI's ubuntu and macOS legs. Do not install a toolchain for it.

- [ ] **Step 8: Write ADR 0022**

Create `docs/adr/0022-process-shutdown-signals.md`:

```markdown
# ADR 0022 — Process shutdown signals: an opt-in flag the handler sets and Nova code polls

## Status

Accepted (2026-10-04). Branch `examples-03-http-server`
(`docs/superpowers/specs/2026-10-04-examples-03-http-server-design.md`).

## Context

`nova-spec/60-EXAMPLES.md` §3's gate says the example "exits cleanly on
SIGTERM". On 2026-10-04 the user defined "cleanly" as graceful: stop
accepting, finish requests in flight, exit 0. Until then the runtime installed
no signal or console-control handler
(`docs/superpowers/specs/2026-10-04-examples-03-04-inventory.md` §3.3).

Four facts constrain any handler:

- **All executor, socket-table and heap state is `thread_local!`** (ADR 0009
  §1). A Unix handler runs in signal context, and a Windows console handler on
  a thread the OS creates. So a handler may touch only process-global atomics.
- **A built executable's entry point is a C `main`** that calls `nova_main()`
  and returns 0, so no Rust runtime initialisation runs. A handler has to be
  installed by a runtime intrinsic.
- **The executor retries an interrupted wait** (`poll.rs`'s EINTR
  `continue`), and a console handler cannot wake `WSAPoll`. A flag is
  therefore seen only at the executor's next deadline.
- **One task can wait on only one socket per poll** (`task.rs`'s
  `try_stage`). Closing a listener does not wake a task parked in `accept` on
  it. Measured on Windows: a 10 ms `WSAENOTSOCK` retry loop, or a block
  forever.

## Decision

1. **A process-wide flag.** `crates/nova-runtime/src/signal.rs` holds one
   `static AtomicBool`. One `STD_ONLY` builtin, `shutdown_requested() -> Bool`,
   reads it. Its first call installs the handler, through a `Once`.
2. **Opt-in and permanent.** Only `std/http`'s `Server::listen` calls it, once
   before `bind`.
   - A program that never does keeps every signal's default action.
   - Once installed, the handler stays. A program that recovers from
     `listen`'s `Err` and keeps running swallows its next signal.
3. **Which inputs.**
   - Unix: SIGTERM, and SIGINT unless it was `SIG_IGN` at install.
   - Windows: CTRL_BREAK and CTRL_C. An inherited Ctrl-C ignore keeps CTRL_C
     from reaching any handler, so it stays ignored with no code.
   - Close, logoff and shutdown events keep the default action.
4. **A second request takes the default action.**
   - Unix: the handler calls `signal(sig, SIG_DFL)` and `raise(sig)`, both
     async-signal-safe.
   - Windows: the handler returns FALSE, so the default routine ends the
     process with `0xC000013A`.

   A stalled drain can always be ended.
5. **The handler never wakes the executor.** It stores the atomic and returns.
   Nova code reads the flag at its own deadlines: `Server::listen` polls
   `accept` under `timeout` every 100 ms, and every connection bounds its
   waits. Shutdown therefore takes at most one tick, plus the requests already
   in flight, each capped at 10 s.

## Alternatives considered

- **A watcher task on a wake socket.** A self-pipe on Unix or a loopback pair
  on Windows; on wake, the watcher connects to the server's own port to
  unblock `accept`. Rejected:
  - it only lowers accept latency;
  - if `listen` ends any other way, the watcher's untimed wait keeps the
    process alive, turning an error exit into a hang;
  - connections still need the ticks.
- **A wake built into the executor.** A runtime-owned socket in
  `run_to_completion`'s wait set, with `accept` returning `Interrupted` once
  the flag is set. Rejected:
  - it changes the scheduler every program runs on;
  - it cannot tell an idle keep-alive read from a mid-request one, so
    `std/http` would still need the ticks;
  - it has the largest record cost of the three.
- **Installing the handler for every program,** from `block_on` or the
  async-main shim. Rejected: a handler that only sets a flag swallows the
  signal. Every async program that never reads the flag would stop responding
  to SIGTERM, including `examples/05-json-api` and the benchmark server.

## Consequences

- **`std/http` gains a graceful `Server::listen`** (`nova-spec/20-STDLIB.md`
  §6's 2026-10-04 note). Its first user is `examples/03-http-server`, whose
  end-to-end tests run on all three CI operating systems.
- **`STD_ONLY` grows from 77 to 78,** and the twelve registration sites ADR
  0018 lists all change.
- **`crates/nova-runtime/Cargo.toml` declares `windows-sys`'s
  `Win32_System_Console`.** Until now it was reachable only through another
  dependency's feature unification. `Cargo.lock` does not change.
- **The executor is unchanged.** `task.rs`, `poll.rs` and `net.rs` keep their
  behaviour, and their counts of wake sources stay true.
- **Not covered:**
  - a public signal API, or `std/process`;
  - the Unix handler on a Unix host, whose first measurement is CI's ubuntu
    and macOS legs;
  - an LLVM-built executable, never run on this host.

## References

- Design: `docs/superpowers/specs/2026-10-04-examples-03-http-server-design.md`
  §4.2 and §4.4
- Inventory: `docs/superpowers/specs/2026-10-04-examples-03-04-inventory.md`
  §3.3
- Code: `crates/nova-runtime/src/signal.rs`, and
  `crates/nova-runtime/tests/signal_sigterm.rs`, `signal_sigint.rs` and
  `signal_console.rs`
- `std/http/lib.nova`: `Server::listen`, `serve`
- `docs/adr/0009-async-execution-model.md` §1: thread-local state, and the
  implicit join
- `docs/adr/0013-io-poller.md`: its rejected "A poller thread signalling the
  executor", and the EINTR retry
- The CI delivery spike: draft PR #95, closed unmerged
```

- [ ] **Step 9: Refresh the debug runtime library, then commit**

```bash
cd /d/Projects/nona/nova && cargo build -p nova-runtime 2>&1 | tail -2 && git add crates/nova-runtime/src/signal.rs crates/nova-runtime/src/lib.rs crates/nova-runtime/Cargo.toml crates/nova-runtime/tests docs/adr/0022-process-shutdown-signals.md && git commit -m "feat(runtime): an opt-in process shutdown flag (ADR 0022)

The first call to nova_rt_shutdown_requested installs a SIGTERM/SIGINT
handler (Unix) or a console-control handler for CTRL_BREAK/CTRL_C
(Windows). The handler only stores an atomic. A second request takes the
default action, and an inherited SIGINT or Ctrl-C ignore stays ignored.
The executor is unchanged. One integration-test binary per
process-global scenario.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 2: Register the `shutdown_requested` builtin

**Files:**
- Modify: `crates/nova-resolver/src/lib.rs`: the `builtins!` list (after `LogSetConfig`), `Builtin::name` (after `Builtin::LogSetConfig => "log_set_config",`), and `STD_ONLY` (`[Builtin; 77]` becomes `78`; append after `Builtin::LogSetConfig,`)
- Modify: `crates/nova-typeck/src/check.rs`: the hint arm (`| Builtin::LogSetConfig => "",`), `builtin_signature` (after `Builtin::LogSetConfig => (vec![Ty::Int, Ty::Int], Ty::Unit),`), and the test table `expected()` (after its `Builtin::LogSetConfig` entry); plus a new test
- Modify: `crates/nova-mir/src/lib.rs`: the `rt_funcs!` list (after `LogSetConfig,`), `symbol()` and `signature()`
- Modify: `crates/nova-mir/src/lower.rs`: the lowering table (after `Builtin::LogSetConfig => Lowering::Runtime(RtFunc::LogSetConfig),`)

**Interfaces:**
- Consumes: `nova_rt_shutdown_requested` and its `symbols()` entry, from Task 1.
- Produces: a std module may call `shutdown_requested()`, typed `() -> Bool` and lowered to `RtFunc::ShutdownRequested`, which is `() -> i8` and symbol `nova_rt_shutdown_requested`.

- [ ] **Step 1: Write the failing test**

In `crates/nova-typeck/src/check.rs`'s test module, after `the_file_builtins_are_std_only_and_return_status_words`, add:

```rust
    /// `shutdown_requested` is `STD_ONLY` and typed `() -> Bool`: only
    /// `std/http`'s `Server` may read the shutdown flag, and reading it is
    /// what installs the signal handler (docs/adr/0022).
    #[test]
    fn shutdown_requested_is_std_only_and_returns_bool() {
        let b = nova_resolver::Builtin::STD_ONLY
            .iter()
            .copied()
            .find(|b| b.name() == "shutdown_requested")
            .expect("shutdown_requested must be an STD_ONLY builtin");
        assert_eq!(builtin_signature(b), (vec![], Ty::Bool));
    }
```

- [ ] **Step 2: Run it to see it fail**

Run: `cd /d/Projects/nona/nova && cargo test -p nova-typeck shutdown_requested_is_std_only 2>&1 | tail -8`
Expected: FAIL with `shutdown_requested must be an STD_ONLY builtin`.

- [ ] **Step 3: Add the builtin at every site**

`crates/nova-resolver/src/lib.rs`, in the `builtins!` list, after `LogSetConfig,`:

```rust
    /// `shutdown_requested() -> Bool` — whether the process has been asked to
    /// stop: the first SIGTERM or SIGINT on Unix, CTRL_BREAK or CTRL_C on
    /// Windows. The first call installs the handler that sets it, so a
    /// program that never calls this keeps every signal's default action.
    /// Read by `std/http`'s `Server::listen` and its connections, once per
    /// tick. Runtime symbol `nova_rt_shutdown_requested`
    /// (`crates/nova-runtime/src/signal.rs`, docs/adr/0022). Std-only.
    ShutdownRequested,
```

In `Builtin::name`, after `Builtin::LogSetConfig => "log_set_config",`:

```rust
            Builtin::ShutdownRequested => "shutdown_requested",
```

In `STD_ONLY`, change `pub const STD_ONLY: [Builtin; 77] = [` to `pub const STD_ONLY: [Builtin; 78] = [`, and after `Builtin::LogSetConfig,` add:

```rust
        Builtin::ShutdownRequested,
```

`crates/nova-typeck/src/check.rs`, in the hint arm, change:

```rust
            | Builtin::LogSetConfig => "",
```

to:

```rust
            | Builtin::LogSetConfig
            | Builtin::ShutdownRequested => "",
```

In `builtin_signature`, after `Builtin::LogSetConfig => (vec![Ty::Int, Ty::Int], Ty::Unit),`:

```rust
        Builtin::ShutdownRequested => (vec![], Ty::Bool),
```

In the test table `expected()`, after its `Builtin::LogSetConfig => ( … ),` entry:

```rust
                Builtin::ShutdownRequested => (
                    (vec![], Ty::Bool),
                    "`shutdown_requested()` in `std/http`'s `Server::listen` and `serve`",
                ),
```

`crates/nova-mir/src/lib.rs`, in the `rt_funcs!` list, after `LogSetConfig,`:

```rust
    /// `() -> i8` — whether a termination signal has been received; the first
    /// call installs the handler (`crates/nova-runtime/src/signal.rs`).
    ShutdownRequested,
```

In `symbol()`, after `RtFunc::LogSetConfig => "nova_rt_log_set_config",`:

```rust
            RtFunc::ShutdownRequested => "nova_rt_shutdown_requested",
```

In `signature()`, after `RtFunc::LogSetConfig => (vec![MirTy::I64, MirTy::I64], MirTy::Unit),`:

```rust
            RtFunc::ShutdownRequested => (vec![], MirTy::I8),
```

`crates/nova-mir/src/lower.rs`, after `Builtin::LogSetConfig => Lowering::Runtime(RtFunc::LogSetConfig),`:

```rust
                    Builtin::ShutdownRequested => Lowering::Runtime(RtFunc::ShutdownRequested),
```

- [ ] **Step 4: Run the new test and the guards that cover every builtin**

Run: `cd /d/Projects/nona/nova && cargo check --workspace --all-targets 2>&1 | tail -3 && cargo test -p nova-typeck -- shutdown_requested_is_std_only builtin_signatures_are_what_the_std_call_sites_use no_std_only_builtin_is_callable_from_user_code 2>&1 | grep -E "^test |test result" && cargo test -p nova-resolver no_std_only_builtin_is_a_reserved_word 2>&1 | grep -E "^test |test result" && cargo test -p nova-codegen-cranelift every_rt_func_symbol_is_registered_with_the_jit 2>&1 | grep -E "^test |test result"`
Expected:
- `cargo check` finishes clean. `--all-targets` is what reaches the typechecker's test-table site.
- Every named test passes: the new one, the signature table, the two `STD_ONLY` loops (which now cover the new entry), and the JIT symbol check (which finds Task 1's `symbols()` entry).

- [ ] **Step 5: Commit**

```bash
cd /d/Projects/nona/nova && cargo fmt --all -- --check && git add crates/nova-resolver/src/lib.rs crates/nova-typeck/src/check.rs crates/nova-mir/src/lib.rs crates/nova-mir/src/lower.rs && git commit -m "feat(builtins): shutdown_requested, STD_ONLY 77 -> 78

Typed () -> Bool, lowered to RtFunc::ShutdownRequested (() -> i8,
nova_rt_shutdown_requested). Std-only, so only std/http's Server can
read the flag, and reading it installs the handler.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

If `cargo fmt --all -- --check` reports a diff, run `cargo fmt --all`, re-run Step 4's `cargo check`, then commit.

---

### Task 3: `std/http`'s `Server` and `Response::json`

**Files:**
- Modify: `std/http/lib.nova`: header comment, lines 5-16 and 29-49; `Response::json` after `not_found`; the transport-half comment at lines 442-446; the `Server` section appended at the end
- Modify: `std/net/lib.nova`: `accept`'s "No timeout" comment, lines 278-282
- Create: `tests/runtime/http_server_dispatch.nova`, `tests/runtime/http_server_dispatch.stdout`
- Modify: `crates/nova-cli/tests/run_tests.rs`: append `http_server_dispatch_run`

**Interfaces:**
- Consumes: the std-only builtin `shutdown_requested() -> Bool`, from Task 2.
- Produces, in `std/http`:
  - `pub record Server`, with `Server::new() -> Server`;
  - `get(self, path: String, handler: fn(Request) -> Response) -> Server`;
  - `dispatch(self, req: Request) -> Response`;
  - `pub async fn listen(self, addr: String) -> Result<(), IoError>`;
  - `Response::json(v: JsonValue) -> Response`.

- [ ] **Step 1: Write the failing fixture and register it**

Create `tests/runtime/http_server_dispatch.nova`:

```nova
// `std/http`'s `Server::dispatch` and `Response::json`, with no socket.
// `examples/03-http-server` drives the same routes over TCP; this pins the
// routing and the JSON response's bytes on their own.

fn status_ok() -> JsonValue {
    let mut m: Map<String, JsonValue> = Map::new()
    m.insert("status", String("ok"))
    Object(m)
}

fn request(method: Method, path: String) -> Request {
    Request { method: method, path: path, headers: Map::new(), body: bytes_from_string("") }
}

fn body_text(r: Response) -> String {
    match r.body.to_string() {
        Some(s) => s
        None => "<not utf-8>"
    }
}

fn header(r: Response, name: String) -> String {
    match r.headers.get(name) {
        Some(v) => v
        None => "<absent>"
    }
}

fn main() {
    let app = Server::new()
        .get("/", |_| Response::text(200, "Hello from Nova!"))
        .get("/health", |_| Response::json(status_ok()))
        .get("/echo", |r: Request| Response::text(200, r.path))

    let a = app.dispatch(request(Get, "/"))
    println("GET / -> ${a.status} ${body_text(a)}")
    let b = app.dispatch(request(Get, "/health"))
    println("GET /health -> ${b.status} ${header(b, "content-type")} ${header(b, "content-length")} ${body_text(b)}")
    let c = app.dispatch(request(Get, "/echo"))
    println("GET /echo -> ${c.status} ${body_text(c)}")
    let d = app.dispatch(request(Get, "/nope"))
    println("GET /nope -> ${d.status}")
    let e = app.dispatch(request(Post, "/"))
    println("POST / -> ${e.status}")
}
```

Create `tests/runtime/http_server_dispatch.stdout`. Every line ends with a newline, the last included, because `println` writes one:

```
GET / -> 200 Hello from Nova!
GET /health -> 200 application/json 15 {"status":"ok"}
GET /echo -> 200 /echo
GET /nope -> 404
POST / -> 404
```

Append to `crates/nova-cli/tests/run_tests.rs`:

```rust

/// `std/http`'s `Server::dispatch` and `Response::json`, socket-free: exact
/// GET routes, a handler that reads its request, 404 for an unknown path and
/// for a non-GET method, and the JSON response's headers and bytes. Design:
/// docs/superpowers/specs/2026-10-04-examples-03-http-server-design.md §7.2.
#[test]
fn http_server_dispatch_run() {
    let expected = std::fs::read_to_string(repo_root().join("tests/runtime/http_server_dispatch.stdout"))
        .expect("expected-output fixture exists")
        .replace("\r\n", "\n");
    nova()
        .arg("run")
        .arg(repo_root().join("tests/runtime/http_server_dispatch.nova"))
        .assert()
        .success()
        .stdout(expected);
}
```

- [ ] **Step 2: Run it to see it fail**

Run: `cd /d/Projects/nona/nova && cargo test -p nova-cli --test run_tests http_server_dispatch_run 2>&1 | tail -15`
Expected: FAIL. `nova run` exits nonzero because `Server` does not resolve: `E0900` "module-qualified paths" on `Server::new()`, or `E0001`.

- [ ] **Step 3: Write the std changes with one script**

Write this script with the Write tool to `C:/Users/SAKEER~1/AppData/Local/Temp/gcm/_once/t3_std_http.py`, then run `python -X utf8 C:/Users/SAKEER~1/AppData/Local/Temp/gcm/_once/t3_std_http.py`, then rename it to `t3_std_http.py.applied`.

```python
import sys
ROOT = "D:/Projects/nona/nova/"

def run(edits):
    state = {}
    for f, anchor, text, mode in edits:
        if f not in state:
            raw = open(ROOT + f, encoding="utf-8", newline="").read()
            state[f] = [raw, "\r\n" if "\r\n" in raw else "\n"]
        raw, nl = state[f]
        a = anchor.replace("\n", nl)
        t = text.replace("\n", nl)
        n = raw.count(a)
        if n != 1:
            sys.exit("ABORT before any write: %s: anchor matched %d times: %r" % (f, n, anchor[:70]))
        if mode == "after":
            state[f][0] = raw.replace(a, a + t)
        elif mode == "before":
            state[f][0] = raw.replace(a, t + a)
        else:
            state[f][0] = raw.replace(a, t)
    for f, (raw, nl) in state.items():
        open(ROOT + f, "w", encoding="utf-8", newline="").write(raw)
        print("wrote", f)

EDITS = []

EDITS.append(("std/http/lib.nova",
"""// The transport is `std/net`: this module parses and serialises, and never
// opens a socket itself.
//
// **What v1 is, against `nova-spec/20-STDLIB.md` section 6.** That section
// specifies a router (`Server::new().get(path, handler)`) over
// `pub type Handler = async fn(Request) -> Response`. That alias does not
// parse -- measured, `P0001: expected type (in type alias), found async` --
// so v1 ships no router and no client. It loses less than it sounds: handler
// code sits inside the caller's own accept loop, which is already an async
// context, so it may `await` freely. A router would have *added* a constraint.
// See docs/adr/0019-offset-table-intrinsic-boundary.md and that section's own
// dated amendment.
""",
"""// The transport is `std/net`. Parsing and serialising never open a socket;
// `Server::listen`, at the end of this file, binds one and is the only code
// here that does.
//
// **What ships, against `nova-spec/20-STDLIB.md` section 6.** That section
// specifies a router over `pub type Handler = async fn(Request) -> Response`.
// That alias does not parse -- measured, `P0001: expected type (in type
// alias), found async` -- and a type alias of any kind is `E0900`, so
// `Server` at the end of this file types each handler inline as
// `fn(Request) -> Response`. A handler is therefore synchronous: it builds
// its response without awaiting. Only GET routes ship, and there is no
// client. `Server::listen` shuts down gracefully on SIGTERM or SIGINT
// (Ctrl+Break or Ctrl+C on Windows); see its own comment and
// docs/adr/0022-process-shutdown-signals.md. See also
// docs/adr/0019-offset-table-intrinsic-boundary.md and section 6's own dated
// amendments.
""", "replace"))

EDITS.append(("std/http/lib.nova",
"""// **Two limitations below share one cause: this module reads exactly one
// request's worth of bytes per connection turn and trusts `Content-Length`
// to say where that ends.**
//
// - `Content-Length` is the only framing v1 understands. Chunked
//   transfer-encoding is neither supported nor detected, so a chunked
//   request's body is left on the connection and is misread as the next
//   request.
// - Bytes already read past the current request's body are discarded, so a
//   pipelined request is lost and its connection deadlocks rather than
//   merely going unsupported. See `read_request`'s own doc comment for the
//   exact wording this needs.
//
// **A third limitation, with a different cause.** Neither `conn.read` call
// in `read_request` below carries a timeout, so a peer that connects and
// then sends nothing -- or sends a partial head or body and stops -- holds
// the task serving it parked forever. `std/net::TcpStream::read_timeout`
// already exists (`std/net/lib.nova`) and was not used here; see
// `read_request`'s own doc comment for the reasoning, and
// docs/adr/0019-offset-table-intrinsic-boundary.md's Consequences section
// for the same gap recorded against that ADR.
""",
"""// **Two limitations of `read_request` below share one cause: it reads
// exactly one request's worth of bytes per connection turn and trusts
// `Content-Length` to say where that ends.** `Server` shares the first and
// not the second: it keeps one buffer per connection, so it answers
// pipelined requests in order.
//
// - `Content-Length` is the only framing either understands. Chunked
//   transfer-encoding is neither supported nor detected, so a chunked
//   request's body is left on the connection and is misread as the next
//   request.
// - `read_request` discards bytes already read past the current request's
//   body, so a pipelined request is lost and its connection deadlocks rather
//   than merely going unsupported. See `read_request`'s own doc comment for
//   the exact wording this needs.
//
// **A third limitation, with a different cause.** Neither `conn.read` call
// in `read_request` below carries a timeout, so a peer that connects and
// then sends nothing -- or sends a partial head or body and stops -- holds
// the task serving it parked forever. `std/net::TcpStream::read_timeout`
// already exists (`std/net/lib.nova`) and was not used there; see
// `read_request`'s own doc comment for the reasoning, and
// docs/adr/0019-offset-table-intrinsic-boundary.md's Consequences section
// for the same gap recorded against that ADR. `Server` does not share this
// one either: every wait in its `serve` is bounded.
""", "replace"))

EDITS.append(("std/http/lib.nova",
"""    pub fn not_found() -> Response { Response::text(404, "not found") }
""",
"""
    // `200 OK` carrying `v` as JSON text, with `content-type` and
    // `content-length` set. It takes the name section 6 gives a client-side
    // decoder (`json<T: FromJson>(self)`): there is no client, and one type
    // cannot have both, so the constructor has it.
    pub fn json(v: JsonValue) -> Response {
        let body = bytes_from_string(stringify(v))
        let mut h: Map<String, String> = Map::new()
        h.insert("content-length", "${body.len()}")
        h.insert("content-type", "application/json")
        Response { status: 200, headers: h, body: body }
    }
""", "after"))

EDITS.append(("std/http/lib.nova",
"""// The transport half: reading a request off a connection and writing a
// response back onto one, honouring keep-alive. `std/net`'s `TcpStream` is
// the only socket type either function below touches, through the `Read` and
// `Write` impls that module defines for it -- this module still never opens
// one itself.
""",
"""// The transport half: reading a request off a connection and writing a
// response back onto one, honouring keep-alive. `std/net`'s `TcpStream` is
// the only socket type either function below touches, through the `Read` and
// `Write` impls that module defines for it -- neither function opens one.
// Only `Server::listen`, at the end of this file, binds a listener.
""", "replace"))

EDITS.append(("std/http/lib.nova",
"""        sent = sent + n
    }
    Ok(total)
}
""",
"""
// ---------------------------------------------------------------------------
// The router: `Server`.
//
// Written in Nova over the pieces above. Handlers are `fn(Request) ->
// Response` values -- this module's header says why not section 6's
// `Handler` alias -- so a handler is synchronous: it builds its response
// without awaiting.

// How often `Server::listen` and every connection it serves look at the
// shutdown flag, in milliseconds. A signal is noticed within one tick.
const SERVER_TICK_MS: Int = 100

// How long one request may take, in milliseconds: from the moment `serve`
// takes it up until its response has been written. A stalled client is
// dropped at this deadline, so a shutdown always finishes.
const SERVER_REQUEST_MS: Int = 10000

// A routing table and nothing else. Build one with `Server::new()` and `get`,
// then `listen`.
pub record Server {
    routes: Vec<Route>
}

// One GET route: an exact path, and the handler that answers it.
record Route {
    path: String
    handler: fn(Request) -> Response
}

// A request read off a connection, and the bytes that followed it: a
// pipelined next request, or the start of one.
record InFlight {
    req: Request
    rest: Bytes
}

impl Server {
    pub fn new() -> Server { Server { routes: Vec::new() } }

    // Adds a GET route. Takes `self` rather than `mut self`, and returns
    // `Server` rather than section 6's `Self`: a `mut self` call on the
    // temporary `Server::new()` returns is `E0060`, and `Self` as a type in an
    // impl block is `E0001`.
    pub fn get(self, path: String, handler: fn(Request) -> Response) -> Server {
        let mut rs = self.routes
        rs.push(Route { path: path, handler: handler })
        Server { routes: rs }
    }

    // The response to `req`: the handler of the GET route whose path equals
    // `req.path` exactly, or `Response::not_found()` for any other path or
    // method. Socket-free, so a test can call it directly.
    pub fn dispatch(self, req: Request) -> Response {
        match req.method {
            Get => {
                let mut i = 0
                while i < self.routes.len() {
                    match self.routes.get(i) {
                        Some(r) => {
                            if r.path == req.path {
                                // Bound to a local first: calling a fn-typed
                                // field directly, `r.handler(req)`, is `E0014`.
                                let h = r.handler
                                return h(req)
                            }
                        }
                        None => {}
                    }
                    i = i + 1
                }
                Response::not_found()
            }
            _ => Response::not_found()
        }
    }

    // Serves `addr` until the process is asked to stop: SIGTERM or SIGINT on
    // Unix, Ctrl+Break or Ctrl+C on Windows.
    //
    // **Graceful.** On the first such signal it stops accepting within one
    // tick, closes the listener, and returns `Ok(())`. Connections are not
    // touched: each finishes the request it has started -- pipelined ones
    // included -- within that request's deadline, and an idle one closes at
    // its next tick. A second signal ends the process at once, by the
    // signal's default action.
    //
    // **It returns before those connections finish.** Their tasks run on
    // alongside any code after `.await`, and the process exits only once
    // `block_on`'s implicit join has finished them
    // (docs/adr/0009-async-execution-model.md).
    //
    // **The signal handler is installed by the first `listen`, before `bind`,
    // and stays installed for the rest of the process.** So a client that can
    // connect can also stop the server gracefully. But a program that recovers
    // from this function's `Err` and keeps running swallows its next signal,
    // and only a second one ends it; docs/adr/0022-process-shutdown-signals.md
    // records why.
    //
    // **Errors.** A failed `bind` or `accept` returns `Err`. An accept error
    // closes the listener first and leaves the connections running; the flag
    // is not set, so they keep being served.
    pub async fn listen(self, addr: String) -> Result<(), IoError> {
        let _ = shutdown_requested()
        let l = match bind(addr) {
            Ok(l) => l
            Err(e) => return Err(e)
        }
        // The flag is checked on every turn, not only when an accept times
        // out: under steady traffic the timeout arm may never run. The accept
        // is polled under `timeout` because a bare one never returns without a
        // connection, and closing the listener under it does not wake it.
        while !shutdown_requested() {
            match timeout(Duration::from_millis(SERVER_TICK_MS), l.accept()).await {
                Ok(r) => {
                    match r {
                        Ok(conn) => {
                            let _ = spawn(serve(self, conn))
                        }
                        Err(e) => {
                            let _ = l.close().await
                            return Err(e)
                        }
                    }
                }
                Err(_) => {}
            }
        }
        let _ = l.close().await
        Ok(())
    }
}

// One connection, for its whole life, with one buffer it keeps throughout.
//
// - **Idle (buffer empty):** waits for a request's first bytes one tick at a
//   time. EOF closes; a timeout closes once the shutdown flag is set and
//   otherwise keeps waiting.
// - **In flight (buffer not empty):** a request is taken up -- its deadline
//   starts now -- read to completion by `read_in_flight`, and answered. The
//   response is written under the same deadline, so a peer that stops
//   reading cannot hold the connection forever either.
// - **After an answer:** bytes left in the buffer are the next request,
//   answered even after a stop, because they are in flight. With the buffer
//   empty, the connection closes if the flag is set and goes idle otherwise.
//
// Anything else that goes wrong -- EOF mid-request, a read or write error, a
// malformed head, an oversized body, a deadline passing -- closes this
// connection without a response, and touches no other.
async fn serve(s: Server, conn: TcpStream) {
    let mut buf = bytes_from_string("")
    let mut open = true
    while open {
        if buf.len() == 0 {
            match conn.read_timeout(4096, SERVER_TICK_MS).await {
                Ok(chunk) => {
                    if chunk.len() == 0 {
                        open = false
                    } else {
                        buf = chunk
                    }
                }
                Err(e) => {
                    match e.kind {
                        TimedOut => {
                            if shutdown_requested() { open = false }
                        }
                        _ => open = false
                    }
                }
            }
        } else {
            let started = Instant::now()
            match read_in_flight(conn, buf, started).await {
                Some(f) => {
                    buf = f.rest
                    let left = SERVER_REQUEST_MS - started.elapsed().as_millis()
                    match timeout(Duration::from_millis(left), write_response(conn, s.dispatch(f.req))).await {
                        Ok(w) => {
                            match w {
                                Ok(n) => {
                                    if buf.len() == 0 && shutdown_requested() { open = false }
                                }
                                Err(e) => open = false
                            }
                        }
                        Err(_) => open = false
                    }
                }
                None => open = false
            }
        }
    }
    let _ = conn.close().await
}

// The request at the front of `first`, reading more from `conn` as needed, or
// `None` to close the connection. The head is parsed from at most
// `max_head_bytes` of the buffer, so bytes buffered behind it -- a body, or a
// pipelined request -- never count against the head's own limit.
async fn read_in_flight(conn: TcpStream, first: Bytes, started: Instant) -> Option<InFlight> {
    let limits = Limits::default()
    let mut buf = first
    let mut head: Option<Request> = None
    while head.is_none() {
        let window = head_window(buf, limits.max_head_bytes)
        match parse_request_head(window, limits) {
            Ok(maybe) => head = maybe
            Err(_) => return None
        }
        if head.is_none() {
            // A head still incomplete at `max_head_bytes` never will be.
            if window.len() >= limits.max_head_bytes { return None }
            match read_more(conn, 4096, started).await {
                Some(chunk) => buf = buf.concat(chunk)
                None => return None
            }
        }
    }
    let req = match head {
        Some(r) => r
        None => return None
    }
    // `parse_request_head` returned `Ok(Some(_))` on this same window, so its
    // offset table has status 0 and its last element is where the body starts.
    let t = parse_offsets(head_window(buf, limits.max_head_bytes))
    let body_start = t[t.len() - 1]
    let want = match content_length_of(req, limits.max_body_bytes) {
        Ok(n) => n
        Err(_) => return None
    }
    if want > limits.max_body_bytes { return None }
    let end = body_start + want
    while buf.len() < end {
        match read_more(conn, end - buf.len(), started).await {
            Some(chunk) => buf = buf.concat(chunk)
            None => return None
        }
    }
    Some(InFlight {
        req: Request { method: req.method, path: req.path, headers: req.headers, body: buf.slice(body_start, end) },
        rest: buf.slice(end, buf.len())
    })
}

// Up to `max` more bytes for the request taken up at `started`, or `None` at
// EOF, on a read error, or once that request's deadline has passed.
async fn read_more(conn: TcpStream, max: Int, started: Instant) -> Option<Bytes> {
    let left = SERVER_REQUEST_MS - started.elapsed().as_millis()
    if left <= 0 { return None }
    match conn.read_timeout(max, left).await {
        Ok(chunk) => {
            if chunk.len() == 0 { return None }
            Some(chunk)
        }
        Err(_) => None
    }
}

// The first `max` bytes of `buf`, or all of it if it is shorter.
fn head_window(buf: Bytes, max: Int) -> Bytes {
    if buf.len() <= max { return buf }
    buf.slice(0, max)
}
""", "after"))

EDITS.append(("std/net/lib.nova",
"""    // **No timeout.** There is no `accept_timeout` counterpart to
    // `TcpStream::read_timeout`, so a task waiting here waits until a
    // connection arrives or the process ends; an untimed wait is never
    // reported as a deadlock, and `block_on` cannot return while any task is
    // parked, so there is no graceful-shutdown path through this.
""",
"""    // **No timeout.** There is no `accept_timeout` counterpart to
    // `TcpStream::read_timeout`, so a task waiting here waits until a
    // connection arrives or the process ends; an untimed wait is never
    // reported as a deadlock, and `block_on` cannot return while any task is
    // parked. A graceful shutdown therefore polls this under `std/time`'s
    // `timeout` instead of awaiting it bare, and closes the listener only
    // once the abandoned accept has returned: closing it under a parked
    // accept does not wake that accept. `std/http`'s `Server::listen` is that
    // shape.
""", "replace"))

run(EDITS)
```

Expected output: `wrote std/http/lib.nova` and `wrote std/net/lib.nova`, with no `ABORT`.

- [ ] **Step 4: Run the fixture, then every `std/http` test and 05's**

Run: `cd /d/Projects/nona/nova && cargo test -p nova-cli --test run_tests -- http_ json_api_example_serves_its_routes 2>&1 | grep -E "^test |test result"`
Expected:
- `http_server_dispatch_run` passes.
- Every other `http_*` test still passes: `http_serialise_run`, `http_keepalive_run`, `http_limits_run`, `http_malformed_run`, `http_offsets_run`, `http_partial_run`.
- `json_api_example_serves_its_routes` still passes.

If the fixture fails on a `nova` diagnostic, read it. The std code above follows shapes that compiled in the inventory probes (`docs/superpowers/specs/2026-10-04-examples-03-04-inventory.md` §3.2): nested `match` on `timeout`'s result, `TimedOut` as a bare pattern, `let h = r.handler` before the call, and no line starting with `(` or `[`. Fix the std code; never change the fixture to match a wrong output.

- [ ] **Step 5: Commit**

```bash
cd /d/Projects/nona/nova && git add std/http/lib.nova std/net/lib.nova tests/runtime/http_server_dispatch.nova tests/runtime/http_server_dispatch.stdout crates/nova-cli/tests/run_tests.rs && git commit -m "feat(std/http): Server with graceful listen, and Response::json

Server::new/get/dispatch/listen, written in Nova. listen installs the
shutdown handler before bind, polls the flag every 100 ms between
timeout-bounded accepts, and closes the listener from its own task.
Each connection keeps one buffer, answers pipelined requests in order,
and bounds every read and the response's write by a 10 s per-request
deadline. The module header and std/net's accept comment are rewritten
to match.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 4: `examples/03-http-server`, its README and its end-to-end tests

**Files:**
- Create: `examples/03-http-server/src/main.nova`, `examples/03-http-server/README.md`
- Modify: `crates/nova-cli/tests/run_tests.rs`: append helpers and five tests

**Interfaces:**
- Consumes: `Server`, `Response::json` and the graceful `listen`, from Task 3; the runtime flag, from Task 1.
- Produces: the gate artifact. Tests:
  - `http_server_example_serves_and_exits_cleanly`
  - `http_server_example_second_signal_forces_exit`
  - `http_server_example_drops_a_stalled_request_at_its_deadline`
  - `http_server_example_fails_fast_when_the_port_is_taken`
  - `a_server_that_never_calls_listen_keeps_the_default_signal_action`

- [ ] **Step 1: Write the end-to-end tests**

Append to `crates/nova-cli/tests/run_tests.rs`:

```rust

// ---------------------------------------------------------------------------
// `examples/03-http-server`: the Phase 2 gate in `nova-spec/60-EXAMPLES.md` §3.
// Design: docs/superpowers/specs/2026-10-04-examples-03-http-server-design.md
// §7.3, and the plan's Review Focus.

/// The example binds the fixed port 3000 the gate names, so every test that
/// runs it holds this lock: libtest runs tests on parallel threads, and two
/// servers cannot both have the port.
static PORT_3000: std::sync::Mutex<()> = std::sync::Mutex::new(());

const HTTP03_ADDR: &str = "127.0.0.1:3000";

fn lock_port_3000() -> std::sync::MutexGuard<'static, ()> {
    PORT_3000
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn http03_addr() -> std::net::SocketAddr {
    HTTP03_ADDR.parse().expect("a valid socket address")
}

/// Fails the calling test unless nothing is listening on port 3000.
fn assert_port_3000_free() {
    use std::time::Duration;
    if let Ok(s) = std::net::TcpStream::connect_timeout(&http03_addr(), Duration::from_millis(500))
    {
        drop(s);
        panic!("something is already listening on {HTTP03_ADDR}; free port 3000 and re-run");
    }
    if let Err(e) = std::net::TcpListener::bind("0.0.0.0:3000") {
        panic!("cannot bind 0.0.0.0:3000 ({e}); free port 3000 and re-run");
    }
}

/// An exit status in a form that names both a signal death (Unix) and a
/// Windows status code in hex.
fn describe_status(s: &std::process::ExitStatus) -> String {
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        if let Some(sig) = s.signal() {
            return format!("killed by signal {sig}");
        }
    }
    match s.code() {
        Some(c) => format!("exit code {c} ({:#x})", c as u32),
        None => format!("{s:?}"),
    }
}

/// SIGTERM to the child. Hand-declared, so no dev-dependency and no
/// `Cargo.lock` change. Measured on all three CI runners by the 2026-10-04
/// spike (draft PR #95).
#[cfg(unix)]
fn send_termination_request(pid: u32) -> Result<(), String> {
    extern "C" {
        fn kill(pid: i32, sig: i32) -> i32;
    }
    let pid = i32::try_from(pid).map_err(|e| format!("pid {pid}: {e}"))?;
    if pid <= 0 {
        return Err(format!("refusing to signal pid {pid}"));
    }
    // SAFETY: `kill` takes two plain integers and touches no memory.
    let rc = unsafe { kill(pid, 15) };
    if rc == 0 {
        Ok(())
    } else {
        Err(format!(
            "kill(pid, SIGTERM) failed: {}",
            std::io::Error::last_os_error()
        ))
    }
}

/// CTRL_BREAK to the child's own console process group, whose id is the
/// child's pid because it was spawned with CREATE_NEW_PROCESS_GROUP. Never
/// group 0, which would reach every process on the console, cargo and this
/// test included. Hand-declared, as on Unix.
#[cfg(windows)]
fn send_termination_request(pid: u32) -> Result<(), String> {
    #[link(name = "kernel32")]
    extern "system" {
        fn GenerateConsoleCtrlEvent(ctrl_event: u32, process_group_id: u32) -> i32;
    }
    if pid == 0 {
        return Err("refusing to signal process group 0".to_string());
    }
    // SAFETY: `GenerateConsoleCtrlEvent` takes two plain integers.
    let rc = unsafe { GenerateConsoleCtrlEvent(1, pid) };
    if rc != 0 {
        Ok(())
    } else {
        Err(format!(
            "GenerateConsoleCtrlEvent(CTRL_BREAK, {pid}) failed: {}",
            std::io::Error::last_os_error()
        ))
    }
}

/// A running `nova run examples/03-http-server/src/main.nova`. Both of its
/// output streams are drained on threads for its whole life: a closed stdout
/// pipe aborts a later `println` (measured), and an undrained full one can
/// block. Dropping it kills a server that is still running, so a test that
/// panics part-way never leaves one on port 3000.
struct Http03Server {
    child: std::process::Child,
    stdout: Option<std::thread::JoinHandle<String>>,
    stderr: Option<std::thread::JoinHandle<String>>,
}

impl Http03Server {
    fn spawn() -> Self {
        use std::io::Read;
        use std::process::Stdio;
        let mut cmd = std::process::Command::new(assert_cmd::cargo::cargo_bin("nova"));
        cmd.arg("run")
            .arg(repo_root().join("examples/03-http-server/src/main.nova"))
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            // CREATE_NEW_PROCESS_GROUP: the child leads its own console
            // group, so a CTRL_BREAK aimed at it never reaches this test.
            cmd.creation_flags(0x0000_0200);
        }
        let mut child = cmd.spawn().expect("spawn nova run on the 03 example");
        let mut out = child.stdout.take().expect("stdout was piped");
        let mut err = child.stderr.take().expect("stderr was piped");
        let stdout = std::thread::spawn(move || {
            let mut s = String::new();
            let _ = out.read_to_string(&mut s);
            s
        });
        let stderr = std::thread::spawn(move || {
            let mut s = String::new();
            let _ = err.read_to_string(&mut s);
            s
        });
        Http03Server {
            child,
            stdout: Some(stdout),
            stderr: Some(stderr),
        }
    }

    fn pid(&self) -> u32 {
        self.child.id()
    }

    /// Waits until the port accepts a connection. `listen` installs the
    /// signal handler before `bind`, so from then on a signal is handled
    /// gracefully. The probe connection is dropped at once; the server reads
    /// its end of stream and closes it.
    fn wait_until_accepting(&mut self) {
        use std::time::{Duration, Instant};
        let deadline = Instant::now() + Duration::from_secs(120);
        loop {
            if let Ok(s) =
                std::net::TcpStream::connect_timeout(&http03_addr(), Duration::from_millis(500))
            {
                drop(s);
                return;
            }
            if let Ok(Some(status)) = self.child.try_wait() {
                let (out, err) = self.streams();
                panic!(
                    "the example exited before listening: {}; stdout={out:?} stderr={err:?}",
                    describe_status(&status)
                );
            }
            if Instant::now() >= deadline {
                self.kill_and_panic("the example never accepted on 127.0.0.1:3000 within 120 s");
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }

    /// Waits up to `limit` for the process to exit.
    fn wait_for_exit(&mut self, limit: std::time::Duration) -> Option<std::process::ExitStatus> {
        let deadline = std::time::Instant::now() + limit;
        loop {
            match self.child.try_wait().expect("try_wait") {
                Some(s) => return Some(s),
                None if std::time::Instant::now() >= deadline => return None,
                None => std::thread::sleep(std::time::Duration::from_millis(50)),
            }
        }
    }

    /// Both streams. Call only once the process has exited.
    fn streams(&mut self) -> (String, String) {
        let out = self
            .stdout
            .take()
            .map(|h| h.join().unwrap_or_default())
            .unwrap_or_default();
        let err = self
            .stderr
            .take()
            .map(|h| h.join().unwrap_or_default())
            .unwrap_or_default();
        (out, err)
    }

    fn kill_and_panic(&mut self, why: &str) -> ! {
        let _ = self.child.kill();
        let status = self.child.wait();
        let (out, err) = self.streams();
        panic!("{why}; force-killed: {status:?}; stdout={out:?} stderr={err:?}");
    }
}

impl Drop for Http03Server {
    fn drop(&mut self) {
        if matches!(self.child.try_wait(), Ok(None)) {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}

/// One client connection to the example. It reads responses framed by
/// `content-length`, and keeps any bytes past one response for the next.
struct Http03Conn {
    sock: std::net::TcpStream,
    pending: Vec<u8>,
}

/// One response: status, header values by lower-cased name, and body.
struct Http03Response {
    status: u16,
    headers: std::collections::HashMap<String, String>,
    body: String,
}

impl Http03Conn {
    fn open() -> Self {
        let sock = std::net::TcpStream::connect(HTTP03_ADDR).expect("connect to 127.0.0.1:3000");
        let limit = Some(std::time::Duration::from_secs(10));
        sock.set_read_timeout(limit).expect("set_read_timeout");
        sock.set_write_timeout(limit).expect("set_write_timeout");
        Http03Conn {
            sock,
            pending: Vec::new(),
        }
    }

    fn send(&mut self, bytes: &str) {
        use std::io::Write;
        self.sock
            .write_all(bytes.as_bytes())
            .expect("write to the example");
    }

    /// The next whole response, or a description of what arrived instead.
    fn response(&mut self) -> Result<Http03Response, String> {
        use std::io::Read;
        let mut chunk = [0u8; 4096];
        loop {
            if let Some(head_end) = self
                .pending
                .windows(4)
                .position(|w| w == b"\r\n\r\n")
                .map(|i| i + 4)
            {
                let head = String::from_utf8_lossy(&self.pending[..head_end]).to_string();
                let mut lines = head.split("\r\n");
                let status = lines
                    .next()
                    .and_then(|l| l.split_whitespace().nth(1))
                    .and_then(|s| s.parse::<u16>().ok())
                    .ok_or_else(|| format!("no status line in {head:?}"))?;
                let mut headers = std::collections::HashMap::new();
                for line in lines {
                    if let Some((name, value)) = line.split_once(':') {
                        headers.insert(name.trim().to_ascii_lowercase(), value.trim().to_string());
                    }
                }
                let len: usize = headers
                    .get("content-length")
                    .and_then(|v| v.parse().ok())
                    .ok_or_else(|| format!("no content-length in {head:?}"))?;
                if self.pending.len() >= head_end + len {
                    let body =
                        String::from_utf8_lossy(&self.pending[head_end..head_end + len]).to_string();
                    self.pending.drain(..head_end + len);
                    return Ok(Http03Response {
                        status,
                        headers,
                        body,
                    });
                }
            }
            match self.sock.read(&mut chunk) {
                Ok(0) => {
                    return Err(format!(
                        "peer closed mid-response; had {:?}",
                        String::from_utf8_lossy(&self.pending)
                    ))
                }
                Ok(n) => self.pending.extend_from_slice(&chunk[..n]),
                Err(e) => {
                    return Err(format!(
                        "read failed: {e}; had {:?}",
                        String::from_utf8_lossy(&self.pending)
                    ))
                }
            }
        }
    }

    /// `Ok` once the server has closed this connection: a read returns end of
    /// stream, or a reset, with nothing else pending.
    fn closed_by_server(&mut self) -> Result<(), String> {
        use std::io::Read;
        if !self.pending.is_empty() {
            return Err(format!(
                "unexpected bytes: {:?}",
                String::from_utf8_lossy(&self.pending)
            ));
        }
        let mut chunk = [0u8; 64];
        match self.sock.read(&mut chunk) {
            Ok(0) => Ok(()),
            Ok(n) => Err(format!(
                "expected end of stream, got {:?}",
                String::from_utf8_lossy(&chunk[..n])
            )),
            Err(e)
                if matches!(
                    e.kind(),
                    std::io::ErrorKind::ConnectionReset | std::io::ErrorKind::ConnectionAborted
                ) =>
            {
                Ok(())
            }
            Err(e) => Err(format!("expected end of stream, got error {e}")),
        }
    }
}

/// The next response on `conn`, or the test fails with the server's streams.
fn http03_response(server: &mut Http03Server, conn: &mut Http03Conn, what: &str) -> Http03Response {
    match conn.response() {
        Ok(r) => r,
        Err(e) => server.kill_and_panic(&format!("{what}: {e}")),
    }
}

/// Waits until a fresh connection to the example is refused, which proves
/// `listen` saw the shutdown flag and closed its listener. A refusal takes
/// about 2 s on Windows (measured), so each attempt allows 3 s.
fn wait_until_refused() -> Result<(), String> {
    use std::time::{Duration, Instant};
    let deadline = Instant::now() + Duration::from_secs(15);
    loop {
        match std::net::TcpStream::connect_timeout(&http03_addr(), Duration::from_secs(3)) {
            Err(e) if e.kind() == std::io::ErrorKind::ConnectionRefused => return Ok(()),
            Ok(s) => drop(s),
            Err(_) => {}
        }
        if Instant::now() >= deadline {
            return Err("the listener was still accepting 15 s after the signal".to_string());
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

/// The gate: both routes; a body on a keep-alive connection; pipelining; a
/// malformed request; a request that straddles the signal; idle connections
/// closed after it; then exit 0.
#[test]
fn http_server_example_serves_and_exits_cleanly() {
    use std::time::Duration;
    let _port = lock_port_3000();
    assert_port_3000_free();
    let mut server = Http03Server::spawn();
    server.wait_until_accepting();

    // Routes, on one keep-alive connection K.
    let mut k = Http03Conn::open();
    k.send("GET / HTTP/1.1\r\nhost: x\r\n\r\n");
    let hello = http03_response(&mut server, &mut k, "GET /");
    assert_eq!(hello.status, 200);
    assert_eq!(hello.body, "Hello from Nova!");
    k.send("GET /health HTTP/1.1\r\nhost: x\r\n\r\n");
    let health = http03_response(&mut server, &mut k, "GET /health");
    assert_eq!(health.status, 200);
    assert_eq!(
        health.headers.get("content-type").map(String::as_str),
        Some("application/json")
    );
    assert_eq!(
        health.headers.get("content-length").map(String::as_str),
        Some("15")
    );
    assert_eq!(health.body, r#"{"status":"ok"}"#);

    // A body is consumed exactly, so the connection stays usable (Review Focus 3).
    k.send("POST / HTTP/1.1\r\nhost: x\r\ncontent-length: 3\r\n\r\nabc");
    let post = http03_response(&mut server, &mut k, "POST / with a body");
    assert_eq!(post.status, 404, "only GET routes ship");
    k.send("GET / HTTP/1.1\r\nhost: x\r\n\r\n");
    let again = http03_response(&mut server, &mut k, "GET / after a body");
    assert_eq!(again.body, "Hello from Nova!");
    // K now stays open and idle across the signal.

    // Pipelining: two requests in one write, answered in order.
    let mut p = Http03Conn::open();
    p.send("GET / HTTP/1.1\r\nhost: x\r\n\r\nGET /health HTTP/1.1\r\nhost: x\r\n\r\n");
    let first = http03_response(&mut server, &mut p, "pipelined GET /");
    let second = http03_response(&mut server, &mut p, "pipelined GET /health");
    assert_eq!(first.body, "Hello from Nova!");
    assert_eq!(second.body, r#"{"status":"ok"}"#);

    // A malformed request closes its connection with no response (Review Focus 4).
    let mut m = Http03Conn::open();
    m.send("this is not http\r\n\r\n");
    if let Err(e) = m.closed_by_server() {
        server.kill_and_panic(&format!("malformed request: {e}"));
    }

    // In flight: half a head, a pause past one tick so the server holds it,
    // the signal, proof the server saw it, and only then the rest.
    let mut f = Http03Conn::open();
    f.send("GET / HTTP/1.1\r\nho");
    std::thread::sleep(Duration::from_millis(300));
    if let Err(e) = send_termination_request(server.pid()) {
        server.kill_and_panic(&format!("could not deliver the signal: {e}"));
    }
    if let Err(e) = wait_until_refused() {
        server.kill_and_panic(&e);
    }
    f.send("st: x\r\n\r\n");
    let in_flight = http03_response(&mut server, &mut f, "the request straddling the signal");
    assert_eq!(in_flight.status, 200);
    assert_eq!(in_flight.body, "Hello from Nova!");
    if let Err(e) = f.closed_by_server() {
        server.kill_and_panic(&format!("after the in-flight answer: {e}"));
    }

    // Idle connections were closed after the signal.
    if let Err(e) = k.closed_by_server() {
        server.kill_and_panic(&format!("idle keep-alive K: {e}"));
    }
    if let Err(e) = p.closed_by_server() {
        server.kill_and_panic(&format!("idle pipelining P: {e}"));
    }

    let status = match server.wait_for_exit(Duration::from_secs(10)) {
        Some(s) => s,
        None => server.kill_and_panic("the example did not exit within 10 s of finishing"),
    };
    let (out, err) = server.streams();
    assert_eq!(
        status.code(),
        Some(0),
        "{}; stdout={out:?} stderr={err:?}",
        describe_status(&status)
    );
    assert!(err.contains("listening on :3000"), "stderr: {err:?}");
}

/// A second signal while a request is still in flight takes the default
/// action at once, well before that request's 10 s deadline.
#[test]
fn http_server_example_second_signal_forces_exit() {
    use std::time::Duration;
    let _port = lock_port_3000();
    assert_port_3000_free();
    let mut server = Http03Server::spawn();
    server.wait_until_accepting();

    let mut stalled = Http03Conn::open();
    stalled.send("GET / HTTP/1.1\r\nho");
    std::thread::sleep(Duration::from_millis(300));
    if let Err(e) = send_termination_request(server.pid()) {
        server.kill_and_panic(&format!("could not deliver the first signal: {e}"));
    }
    // Unix merges a second SIGTERM sent while the first is pending, so wait
    // for proof the first was handled before sending the second.
    if let Err(e) = wait_until_refused() {
        server.kill_and_panic(&e);
    }
    if let Err(e) = send_termination_request(server.pid()) {
        server.kill_and_panic(&format!("could not deliver the second signal: {e}"));
    }
    let status = match server.wait_for_exit(Duration::from_secs(5)) {
        Some(s) => s,
        None => server.kill_and_panic("a second signal did not end the process within 5 s"),
    };
    let (out, err) = server.streams();
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        assert_eq!(
            status.signal(),
            Some(15),
            "{}; stdout={out:?} stderr={err:?}",
            describe_status(&status)
        );
    }
    #[cfg(windows)]
    {
        assert_eq!(
            status.code().map(|c| c as u32),
            Some(0xC000_013A),
            "{}; stdout={out:?} stderr={err:?}",
            describe_status(&status)
        );
    }
    drop(stalled);
}

/// A request that stalls mid-head across the signal is held until its 10 s
/// deadline and then dropped, so the drain always finishes and the process
/// still exits 0 (Review Focus 1).
#[test]
fn http_server_example_drops_a_stalled_request_at_its_deadline() {
    use std::time::{Duration, Instant};
    let _port = lock_port_3000();
    assert_port_3000_free();
    let mut server = Http03Server::spawn();
    server.wait_until_accepting();

    let mut stalled = Http03Conn::open();
    stalled.send("GET / HTTP/1.1\r\nho");
    std::thread::sleep(Duration::from_millis(300));
    let signalled = Instant::now();
    if let Err(e) = send_termination_request(server.pid()) {
        server.kill_and_panic(&format!("could not deliver the signal: {e}"));
    }
    let status = match server.wait_for_exit(Duration::from_secs(20)) {
        Some(s) => s,
        None => server.kill_and_panic("the stalled request held the process past 20 s"),
    };
    let took = signalled.elapsed();
    let (out, err) = server.streams();
    assert_eq!(
        status.code(),
        Some(0),
        "{}; stdout={out:?} stderr={err:?}",
        describe_status(&status)
    );
    assert!(
        took >= Duration::from_secs(8),
        "exited {took:?} after the signal: the stalled request was not held to its deadline"
    );
    if let Err(e) = stalled.closed_by_server() {
        panic!("the stalled connection was not closed by the server: {e}");
    }
}

/// With port 3000 taken, the example fails at once with a nonzero exit and
/// the generic unwrap message, rather than hanging (design §6; Review Focus 5).
#[test]
fn http_server_example_fails_fast_when_the_port_is_taken() {
    use std::time::Duration;
    let _port = lock_port_3000();
    assert_port_3000_free();
    let _holder = std::net::TcpListener::bind("0.0.0.0:3000").expect("hold port 3000");
    let mut server = Http03Server::spawn();
    let status = match server.wait_for_exit(Duration::from_secs(120)) {
        Some(s) => s,
        None => server.kill_and_panic("the example did not exit with port 3000 already taken"),
    };
    let (out, err) = server.streams();
    assert!(
        !status.success(),
        "it must fail: {}; stdout={out:?} stderr={err:?}",
        describe_status(&status)
    );
    assert!(
        err.contains("called `unwrap` on an `Err` value"),
        "stderr: {err:?}"
    );
}

/// A server that never calls `Server::listen` installs no handler, so a
/// termination request still ends it by the default action (design §7.4;
/// Review Focus 2). `docs/benchmarks/server.nova` is such a server. This is
/// the 2026-10-04 spike's measurement (draft PR #95), kept. It passes before
/// this branch exists, by design: it pins behaviour that must not change.
#[test]
fn a_server_that_never_calls_listen_keeps_the_default_signal_action() {
    use std::io::{BufRead, BufReader, Read};
    use std::process::Stdio;
    use std::time::{Duration, Instant};

    let mut cmd = std::process::Command::new(assert_cmd::cargo::cargo_bin("nova"));
    cmd.arg("run")
        .arg(repo_root().join("docs/benchmarks/server.nova"))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0000_0200);
    }
    let mut child = cmd
        .spawn()
        .expect("spawn nova run docs/benchmarks/server.nova");
    let stdout = child.stdout.take().expect("stdout was piped");
    let (tx, rx) = std::sync::mpsc::channel::<String>();
    let out_thread = std::thread::spawn(move || {
        let mut lines = BufReader::new(stdout).lines().map_while(Result::ok);
        if let Some(first) = lines.next() {
            let _ = tx.send(first);
        }
        lines.collect::<Vec<_>>().join("\n")
    });
    let mut stderr = child.stderr.take().expect("stderr was piped");
    let err_thread = std::thread::spawn(move || {
        let mut s = String::new();
        let _ = stderr.read_to_string(&mut s);
        s
    });

    let first = rx.recv_timeout(Duration::from_secs(120)).unwrap_or_default();
    if !first.contains("listening on") {
        let _ = child.kill();
        let status = child.wait();
        let err = err_thread.join().unwrap_or_default();
        panic!("no ready line; got {first:?}; status {status:?}; stderr {err:?}");
    }
    // Let the server reach its parked accept before the request arrives.
    std::thread::sleep(Duration::from_millis(500));
    if let Err(e) = send_termination_request(child.id()) {
        let _ = child.kill();
        let _ = child.wait();
        panic!("could not deliver the request: {e}");
    }
    let deadline = Instant::now() + Duration::from_secs(10);
    let status = loop {
        if let Some(s) = child.try_wait().expect("try_wait") {
            break s;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let s = child.wait();
            panic!("the request did not end the server within 10 s; force-killed: {s:?}");
        }
        std::thread::sleep(Duration::from_millis(50));
    };
    let out = out_thread.join().unwrap_or_default();
    let err = err_thread.join().unwrap_or_default();
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        assert_eq!(
            status.signal(),
            Some(15),
            "{}; stdout={out:?} stderr={err:?}",
            describe_status(&status)
        );
    }
    #[cfg(windows)]
    {
        assert_eq!(
            status.code().map(|c| c as u32),
            Some(0xC000_013A),
            "{}; stdout={out:?} stderr={err:?}",
            describe_status(&status)
        );
    }
}
```

- [ ] **Step 2: Run them to see the gate tests fail**

Run: `cd /d/Projects/nona/nova && cargo test -p nova-cli --test run_tests -- http_server_example a_server_that_never_calls_listen 2>&1 | grep -E "^test |panicked|test result" | head -20`
Expected:
- The four `http_server_example_*` tests FAIL, because the example does not exist yet. The three that wait for readiness fail with "the example exited before listening". The port-taken test fails because stderr lacks the unwrap message.
- `a_server_that_never_calls_listen_keeps_the_default_signal_action` PASSES. That is intended: it pins behaviour that must not change, and the Review Focus line says so.

- [ ] **Step 3: Write the example**

Create `examples/03-http-server/src/main.nova`:

```nova
// Phase 2's gate example in `nova-spec/60-EXAMPLES.md` section 3: an HTTP
// server with two routes that exits cleanly on SIGTERM.
//
// **This is not that section's listing, deliberately.** The listing is kept
// as the aspiration it was, the precedent `examples/05-json-api` set, and
// this serves the same two routes in the Nova that exists. Every
// substitution, and the evidence for it, is in
// `docs/superpowers/specs/2026-10-04-examples-03-http-server-design.md`
// section 3.
//
// **"Exits cleanly" means graceful,** by the user's decision of 2026-10-04.
// On SIGTERM or SIGINT (Ctrl+Break or Ctrl+C on Windows) `listen` stops
// accepting, the requests already in flight are finished, and the process
// exits 0. A second signal ends it at once. See `std/http`'s
// `Server::listen` and docs/adr/0022-process-shutdown-signals.md.
//
// **The log line comes before the server is ready.** It is written before
// `listen` binds the port and installs the signal handler, so readiness
// means the port accepts a connection.

// `{"status":"ok"}`. Built from a `Map` because a map literal is `P0001`; it
// has one key, so `Map`'s per-process ordering cannot reorder it.
fn status_ok() -> JsonValue {
    let mut m: Map<String, JsonValue> = Map::new()
    m.insert("status", String("ok"))
    Object(m)
}

async fn main() {
    Log::init()

    let app = Server::new()
        .get("/", |_| Response::text(200, "Hello from Nova!"))
        .get("/health", |_| Response::json(status_ok()))

    Log::info("listening on :3000")
    app.listen("0.0.0.0:3000").await.unwrap()
}
```

Create `examples/03-http-server/README.md`:

````markdown
# 03-http-server

The smallest `std/http` server: two routes, and a clean exit on SIGTERM.

## What this demonstrates

- **`std/http`'s `Server`:** `Server::new()`, `get(path, handler)` and
  `listen(addr)`, with handlers written as closures.
- **`Response::text` and `Response::json`.**
- **`std/log`:** `Log::init()` and `Log::info(...)`, which write to stderr.
- **Graceful shutdown.** On SIGTERM or SIGINT (Ctrl+Break or Ctrl+C on Windows)
  the server stops accepting, finishes the requests already in flight, and
  exits 0. A second signal ends it at once.

## Run it

```bash
cd examples/03-http-server
nova run
```

Then, from another terminal:

```bash
curl http://localhost:3000/
curl http://localhost:3000/health
```

## Expected output

The server writes one log line to stderr and nothing to stdout. The timestamp
differs from run to run:

```
2026-10-04T10:27:52.126Z INFO listening on :3000
```

The two requests return:

```
Hello from Nova!
{"status":"ok"}
```

## Notes

- **This is not `nova-spec/60-EXAMPLES.md` §3's listing, on purpose.** That
  listing is written in a Nova that does not exist, starting with its first
  line, `import std/http`, which does not parse. It is kept as the aspiration,
  as §5's is. Every substitution this example makes, and the evidence for it,
  is in `docs/superpowers/specs/2026-10-04-examples-03-http-server-design.md`
  §3.
- **The log line is printed before the server is ready.** It comes before
  `listen` binds the port and installs the signal handler. Readiness, for a
  request or for a graceful signal, means the port accepts a connection.
- **`curl http://localhost:3000/` pays about 0.2 s on Windows.** curl tries
  `::1` first, and this server binds IPv4 only (`0.0.0.0`).
  `curl http://127.0.0.1:3000/` does not pay it.
- **Port 3000 must be free.** If it is not, `listen` returns an error and
  `.unwrap()` ends the process with ``nova: panic: called `unwrap` on an `Err`
  value``. The error itself is not printed.
- **Shutdown takes up to 100 ms with nothing in flight.** A request still
  arriving can hold it for up to 10 s, its deadline. Each idle connection
  costs a wake every 100 ms.
- **Windows has no SIGTERM.** Use Ctrl+Break or Ctrl+C in the server's
  console. A plain `taskkill` cannot stop a console program; `taskkill /F`
  ends it at once, without a graceful shutdown.
- **The tests** are in `crates/nova-cli/tests/run_tests.rs`, the
  `http_server_example_*` functions, not in a `tests/` folder here: Nova code
  cannot send a signal to another process.
````

- [ ] **Step 4: Run the tests to see them pass**

Run: `cd /d/Projects/nona/nova && cargo test -p nova-cli --test run_tests -- http_server_example a_server_that_never_calls_listen http_server_dispatch_run 2>&1 | grep -E "^test |panicked|test result"`
Expected: all six pass. The deadline test takes about 10 s by design.

If one fails, read its message: every failure prints the exit status and both streams.
- A Windows status of `0xC000013A` in the gate test means the console handler did not take the CTRL_BREAK. Check Task 1's `install_handler` ran, meaning `listen` reached its first `shutdown_requested()`.
- An exit 0 that took about 10 s in the escalation test means the second signal was merged into the first, or not escalated. Check `unix_handler` and `console_handler`.

- [ ] **Step 5: Check the literal gate command on this host (spec §11.1)**

Run: `cd /d/Projects/nona/nova/examples/03-http-server && ../../target/debug/nova.exe run > /tmp/gcm/e03.out 2> /tmp/gcm/e03.err & P=$!; sleep 8; curl -s http://localhost:3000/; echo; curl -s http://localhost:3000/health; echo; kill $P; sleep 1; cat /tmp/gcm/e03.err`
Expected:
- `Hello from Nova!`
- `{"status":"ok"}`
- the stderr log line `… INFO listening on :3000`

The `kill` here only cleans up: Git Bash's `kill` is a forced termination on Windows. The graceful exit is what the end-to-end tests prove. If `curl` gets nothing, the 8 s may not have covered compilation; read `/tmp/gcm/e03.err` and retry with a longer sleep.

- [ ] **Step 6: Commit**

```bash
cd /d/Projects/nona/nova && git add examples/03-http-server crates/nova-cli/tests/run_tests.rs && git commit -m "feat(examples): 03-http-server, the 60-EXAMPLES section 3 gate

Serves / and /health on port 3000 through std/http's Server, and exits
0 on SIGTERM or Ctrl+Break after finishing in-flight requests. The
end-to-end tests drive:
- both routes;
- a body on a keep-alive connection, pipelining, a malformed request;
- a request straddling the signal, and idle connections closing after it;
- exit 0;
- a second signal forcing exit;
- a stalled request dropped at its 10 s deadline;
- a taken port failing fast;
- a server without listen keeping the default signal action.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 5: Records: dated notes, the CHANGELOG, and the sweep

**Files:** (all modify)
- `nova-spec/60-EXAMPLES.md` (§3, §5, §9, §10)
- `nova-spec/20-STDLIB.md` (§6, and the gate-remeasure-7 paragraph)
- `nova-spec/13-RUNTIME.md` (§4.1, §4.4, and the gate-remeasure-7 paragraph)
- `nova-spec/00-MASTER-SPEC.md` (the gate-remeasure-7 paragraph, and §9's drift notes)
- `docs/phase-2-plan.md` (the gate-remeasure-7 paragraph)
- `docs/adr/0013-io-poller.md`, `docs/adr/0009-async-execution-model.md` (References)
- `docs/adr/0019-offset-table-intrinsic-boundary.md` (Consequences)
- `docs/superpowers/specs/2026-10-04-examples-03-04-inventory.md`
- `docs/superpowers/specs/2026-09-01-std-http-request-parsing-design.md` (§1, §3, §10)
- `docs/superpowers/specs/2026-08-23-std-net-listener-design.md` (§7 item 3)
- `examples/05-json-api/README.md`
- `docs/benchmarks/bun-server.js`
- `CHANGELOG.md` (`[Unreleased]` Added, and a forward marker on `[0.2.0-alpha.2]`)

**Interfaces:**
- Consumes: the names and behaviour from Tasks 1-4.
- Produces: no code.

- [ ] **Step 1: Apply every note with one script**

Write this script with the Write tool to `C:/Users/SAKEER~1/AppData/Local/Temp/gcm/_once/t5_records.py`, run it with `python -X utf8`, then rename it to `t5_records.py.applied`. Every anchor below was checked on 2026-10-04 to match exactly once. If one has moved, the script aborts before writing anything: re-read that file, fix the anchor, and re-run.

```python
import sys
ROOT = "D:/Projects/nona/nova/"

def run(edits):
    state = {}
    for f, anchor, text, mode in edits:
        if f not in state:
            raw = open(ROOT + f, encoding="utf-8", newline="").read()
            state[f] = [raw, "\r\n" if "\r\n" in raw else "\n"]
        raw, nl = state[f]
        a = anchor.replace("\n", nl)
        t = text.replace("\n", nl)
        n = raw.count(a)
        if n != 1:
            sys.exit("ABORT before any write: %s: anchor matched %d times: %r" % (f, n, anchor[:70]))
        if mode == "after":
            state[f][0] = raw.replace(a, a + t)
        elif mode == "before":
            state[f][0] = raw.replace(a, t + a)
        else:
            state[f][0] = raw.replace(a, t)
    for f, (raw, nl) in state.items():
        open(ROOT + f, "w", encoding="utf-8", newline="").write(raw)
        print("wrote", f)

GATE7 = '"AMENDMENT 2026-10-04 (gate-remeasure-7)".\n'
GATE7_NOTE = """
**Recorded 2026-10-04 (branch `examples-03-http-server`):** `03-http-server`
now exists under `examples/`, and end-to-end tests of both of its gate
clauses run on all three CI operating systems; see `nova-spec/60-EXAMPLES.md`
§3. `04-todo-cli` still does not exist, so Phase 2 is still not complete.
"""

EDITS = []

EDITS.append(("nova-spec/60-EXAMPLES.md",
'on SIGTERM" means, and the runtime observes no signal (§3.3 there). The listing\nabove is unchanged.\n',
"""
**Recorded 2026-10-04 (branch `examples-03-http-server`): the example now
exists, beside `examples/03-producer-consumer`, and "exits cleanly" now means
graceful.** `examples/03-http-server` serves `/` and `/health` through
`std/http`'s new `Server`. On SIGTERM or SIGINT (Ctrl+Break or Ctrl+C on
Windows) it stops accepting, finishes the requests already in flight, and
exits 0; a second signal ends it at once. That definition is the user's
decision of 2026-10-04, recorded in
`docs/superpowers/specs/2026-10-04-examples-03-http-server-design.md`, whose
§3 lists every substitution the example makes for the listing above. The
listing is unchanged, kept as the aspiration it was, as §5's is. This settles
the 2026-09-01 drift note above only in part: `03-http-server` now exists on
disk, and slot 03 holds two entries.
""", "after"))

EDITS.append(("nova-spec/60-EXAMPLES.md",
"`docs/superpowers/specs/2026-10-04-examples-03-04-inventory.md` §3.1 and §3.2.\n",
"""
**Recorded 2026-10-04 (branch `examples-03-http-server`): `std/http` now has
a `Server` with `get`.** The sentence above that there is "nothing for
`Server.get`/`.post` to be built on" is no longer true of `get`; `post` still
does not ship, and this listing's `async |..|` handlers still do not parse.
Separately, two citations in the 2026-09-03 amendment above were already stale
and are corrected here rather than edited in place. The `@derive` sentence it
cites as `nova-spec/20-STDLIB.md:548` is in that file's §7 ("`@derive` for
ToJson/FromJson is implemented as a compiler builtin (Phase 2)"), and the
`pub type Handler` line it cites as `:504` is in §6's code block. Both are
given by section here because that file's line numbers move whenever §6 gains
a note.
""", "after"))

EDITS.append(("nova-spec/60-EXAMPLES.md",
"inside the file it counts.\n",
"""
**Recorded 2026-10-04 (branch `examples-03-http-server`): a second example
follows this template.** `examples/03-http-server/README.md` does, beside
`examples/05-json-api/README.md`. `01-hello-world`, `02-fibonacci` and
`03-producer-consumer` still have no `README.md`. The durable check is still
`ls examples/*/README.md` against `ls -d examples/*/`.
""", "after"))

EDITS.append(("nova-spec/60-EXAMPLES.md",
"      working-directory: examples/${{ matrix.example }}\n```\n",
"""
**Recorded 2026-10-04 (branch `examples-03-http-server`): no example meets
this section, `03-http-server` included.** Its tests live in
`crates/nova-cli/tests/run_tests.rs`, not in an `examples/03-http-server/tests/`
folder, for two reasons. `nova test` collects only `@test` functions reachable
from `src/main.nova`, so a `tests/` folder is invisible to it. And Nova code
cannot send a signal to another process, so a `nova test` could not test the
gate's second clause. The CI job above does not exist in
`.github/workflows/ci.yml`; the end-to-end tests run in its ordinary
`cargo test` step on all three operating systems.
""", "after"))

EDITS.append(("nova-spec/20-STDLIB.md",
"closures (`async |..|`) still do not exist. See\n`docs/superpowers/specs/2026-10-04-examples-03-04-inventory.md` §3.1.\n",
"""
**Recorded 2026-10-04 (branch `examples-03-http-server`): the router ships,
narrower than the code block below.** `std/http` now has `Server` with
`new()`, `get(path, handler)`, `dispatch(req)` and `listen(addr)`, and
`Response::json(v: JsonValue)`, all written in Nova.
- `get` returns `Server`, not `Self`: `Self` as a type in an impl block is
  `E0001`. Handlers are typed `fn(Request) -> Response` inline, because the
  `Handler` alias below is still `P0001`, so a handler is synchronous.
- `listen` shuts down gracefully on the first SIGTERM or SIGINT (Ctrl+Break or
  Ctrl+C on Windows). It stops accepting within 100 ms and returns `Ok(())` as
  soon as accepting stops; each open connection then finishes the request it
  has started, within a 10 s deadline that also bounds the response's write.
  A second signal ends the process. The first `listen` installs the signal
  handler, through the `STD_ONLY` builtin `shutdown_requested`, and it stays
  installed; see `docs/adr/0022-process-shutdown-signals.md`.
- `Server` answers pipelined requests in order. `read_request` still does not,
  so the "request pipelining" under "Not in v1" above now describes
  `read_request` only.
- `Response::json` is a constructor: status 200, `content-type:
  application/json`, `content-length`. It takes the name this section's client
  half gives a decoder, `json<T: FromJson>(self)`; one type cannot have both,
  and the client still does not ship.
- Not shipped: `post`, `put`, `delete`, `route`, `use_middleware`, path params,
  the `Handler` and `Middleware` aliases, and the client.

See `docs/superpowers/specs/2026-10-04-examples-03-http-server-design.md`.
""", "after"))

EDITS.append(("nova-spec/20-STDLIB.md", GATE7, GATE7_NOTE, "after"))
EDITS.append(("nova-spec/13-RUNTIME.md", GATE7, GATE7_NOTE, "after"))
EDITS.append(("nova-spec/00-MASTER-SPEC.md", GATE7, GATE7_NOTE, "after"))
EDITS.append(("docs/phase-2-plan.md", GATE7, GATE7_NOTE, "after"))

EDITS.append(("nova-spec/13-RUNTIME.md",
"  is unpreemptable, and no watchdog can fire while it spins.\n",
"""
**AMENDED 2026-10-04 (branch `examples-03-http-server`): the runtime can now
be asked to stop, and the executor still does not know it.** A process-wide
flag in `crates/nova-runtime/src/signal.rs` is set by the first SIGTERM or
SIGINT (Ctrl+Break or Ctrl+C on Windows), through a handler that the first
call to the `STD_ONLY` builtin `shutdown_requested()` installs. The handler
only stores an atomic. It never wakes the executor, which keeps retrying an
interrupted wait, so Nova code sees the flag at its next deadline;
`std/http`'s `Server::listen` provides one every 100 ms. A program that never
calls `shutdown_requested()` installs nothing and keeps every signal's default
action. See `docs/adr/0022-process-shutdown-signals.md`.
""", "after"))

EDITS.append(("nova-spec/13-RUNTIME.md",
"- **The frozen poll ABI has no interrupt hook** (§4.2). There is no way to stop a task\n  mid-flight, only to stop polling it.\n",
"""
**AMENDED 2026-10-04 (branch `examples-03-http-server`): a graceful shutdown
is not cancellation.** `std/http`'s `Server::listen` stops accepting once the
shutdown flag is set (§4.1's 2026-10-04 note), and it stops nothing else. Each
connection's task finishes the request it has started, bounded by that
request's own deadline, and `block_on` returns only once those tasks complete.
Nothing is interrupted mid-flight, so the gap this section records is
unchanged.
""", "after"))

EDITS.append(("nova-spec/00-MASTER-SPEC.md",
"neither. Section 2's tree does name `05-json-api/`, so that entry of it is no\nlonger ahead of the disk.]\n",
"""
[Amended 2026-10-04, branch `examples-03-http-server`: `examples/03-http-server/`
now exists, beside `examples/03-producer-consumer/`, so Section 2's
`03-http-server/` entry is no longer ahead of the disk either. Slot 03 now
holds two entries; nothing was renumbered, by the user's decision of
2026-10-04. A second example now has the §9 README,
`examples/03-http-server/README.md`; `01-hello-world`, `02-fibonacci` and
`03-producer-consumer` still have none. The durable checks are still
`ls -d examples/*/` and `ls examples/*/README.md`, not this note.]
""", "after"))

EDITS.append(("docs/adr/0013-io-poller.md",
"  the same property this decision's second alternative (IOCP) notes does not\n  itself decide the question, unlike the first\n",
"""- `docs/adr/0022-process-shutdown-signals.md` (2026-10-04) — a process-wide
  shutdown flag set by a signal handler. It leaves this ADR's first rejected
  alternative rejected: the handler stores an atomic and never signals the
  executor, which still learns of the flag only at its next deadline
""", "after"))

EDITS.append(("docs/adr/0009-async-execution-model.md",
"  livelock footguns this document already carried for the same reason\n",
"""- `docs/adr/0022-process-shutdown-signals.md` (2026-10-04) — the graceful
  shutdown `std/http`'s `Server` builds on this ADR's implicit join: `listen`
  stops accepting, and `block_on` returns once every connection's task has
  finished its own bounded request
""", "after"))

EDITS.append(("docs/adr/0019-offset-table-intrinsic-boundary.md",
'  silently consumed and the connection deadlocks, which is a sharper claim\n  than "unsupported").\n',
"""  [Amended 2026-10-04, branch `examples-03-http-server`: the router now ships,
  as `Server` with `get` and `listen`, written in Nova over this module's own
  pieces with no new intrinsic, and `Server` answers pipelined requests. The
  pipelining deadlock above remains true of `read_request`. See
  `nova-spec/20-STDLIB.md` §6's 2026-10-04 note.]
""", "after"))

EDITS.append(("docs/superpowers/specs/2026-10-04-examples-03-04-inventory.md",
"in the repository changed to produce it. No probe program is kept.\n",
"""
**Recorded 2026-10-04 (branch `examples-03-http-server`): parts of this record
are now history.**
- §2 and §3.3: the runtime now observes SIGTERM and SIGINT (Ctrl+Break and
  Ctrl+C on Windows), through an opt-in flag
  (`docs/adr/0022-process-shutdown-signals.md`), and `std/http`'s `Server`
  turns it into a graceful exit.
- §6: decision (i) is made, "exits cleanly" means graceful, and so are (ii),
  beside `03-producer-consumer`, and (iii), the listing kept as aspiration.
  Step 1 of the recommended order is done: `examples/03-http-server` exists.
- §3.2 and §7: the router compiles inside `std/http`, so "whether it compiles
  inside std is inferred" is settled.
- Line numbers this record cites in `nova-spec/20-STDLIB.md` hold for this
  record's own branch, as §1 says. The branch named above added a note under
  that file's §6, which moves every later line again, so read those citations
  by the text they quote.
""", "after"))

EDITS.append(("docs/superpowers/specs/2026-09-01-std-http-request-parsing-design.md",
"  `docs/superpowers/specs/2026-10-04-examples-03-04-inventory.md` §3.1 and\n  §3.2.]\n",
"""  [Amended 2026-10-04, branch `examples-03-http-server`: the router now ships
  in `std/http` as `Server`; see `nova-spec/20-STDLIB.md` §6's 2026-10-04
  note.]
""", "after"))

EDITS.append(("docs/superpowers/specs/2026-09-01-std-http-request-parsing-design.md",
"- **HTTPS, HTTP/2, chunked transfer-encoding, and request pipelining.**\n  Keep-alive *is* in scope: the gate is a throughput number and reconnecting per\n  request would dominate it.\n",
"""  [Amended 2026-10-04, branch `examples-03-http-server`: `Server` now answers
  pipelined requests; this item stays true of `read_request`.]
""", "after"))

EDITS.append(("docs/superpowers/specs/2026-09-01-std-http-request-parsing-design.md",
"exist, so a handler that awaits needs a named `async fn`. See\n`docs/superpowers/specs/2026-10-04-examples-03-04-inventory.md` §3.1.]\n",
"""
[Amended 2026-10-04, branch `examples-03-http-server`: the router shipped with
synchronous handlers only: `Server`'s handlers are typed
`fn(Request) -> Response`.]
""", "after"))

EDITS.append(("docs/superpowers/specs/2026-09-01-std-http-request-parsing-design.md",
"- The router, the client, HTTPS, HTTP/2, chunked encoding, pipelining — §1.\n",
"""  [Amended 2026-10-04, branch `examples-03-http-server`: the router now
  ships, and `Server` answers pipelined requests though `read_request` still
  does not; see §1's notes.]
""", "after"))

EDITS.append(("docs/superpowers/specs/2026-08-23-std-net-listener-design.md",
"   > `docs/superpowers/specs/2026-10-04-examples-03-04-inventory.md` §3.3.\n",
"""   >
   > **AMENDED 2026-10-04 (branch `examples-03-http-server`).** `std/http`'s
   > `Server::listen` is that shape, driven by a shutdown signal; see
   > `docs/adr/0022-process-shutdown-signals.md`.
""", "after"))

EDITS.append(("examples/05-json-api/README.md",
"  `req.path.split(\"/\")`. `std/http` ships no router type.\n  `nova-spec/20-STDLIB.md`'s own `pub type Handler = async fn(Request) -> Response`\n  still does not parse, but a router does not need it: one over\n  `fn(Request) -> Response` fields ran in today's Nova on this Windows host\n  (`docs/superpowers/specs/2026-10-04-examples-03-04-inventory.md` §3.2).\n",
"""  `req.path.split("/")`. `std/http`'s `Server` (see `examples/03-http-server`)
  routes exact paths and `GET` only, so this example's `POST /users` and its
  `/users/:id` pattern still route by hand.
""", "replace"))

EDITS.append(("docs/benchmarks/bun-server.js",
"// Nova's side is a raw `std/http` accept loop -- `std/http` has no router,\n// `pub type Handler = async fn(Request) -> Response` being P0001. Putting a\n// routing framework here would measure Nova against that framework.\n",
"""// Nova's side is a raw `std/http` accept loop -- `examples/05-json-api` routes
// by hand rather than through `std/http`'s `Server`. Putting a routing
// framework here would measure Nova against that framework.
""", "replace"))

EDITS.append(("CHANGELOG.md",
"\n### Measured\n",
"""- **`examples/03-http-server`, the Phase 2 gate in `nova-spec/60-EXAMPLES.md`
  §3, beside `examples/03-producer-consumer`.** It serves `/`
  (`Hello from Nova!`) and `/health` (`{"status":"ok"}`) on port 3000. On
  SIGTERM or SIGINT (Ctrl+Break or Ctrl+C on Windows) it stops accepting,
  finishes requests already in flight, and exits 0; a second signal ends it
  at once. End-to-end tests on all three CI operating systems drive both
  routes, a body on a keep-alive connection, pipelining, a malformed
  request, a request that straddles the signal, a stalled request dropped
  at its deadline, a taken port, and the second signal. The listing is kept
  as aspiration; the substitutions are in
  `docs/superpowers/specs/2026-10-04-examples-03-http-server-design.md` §3.
- **`std/http` gains a router: `Server` with `new`, `get`, `dispatch` and
  `listen`, and a `Response::json` constructor, all written in Nova.**
  Handlers are `fn(Request) -> Response`. `listen` polls the shutdown flag
  every 100 ms, bounds every read and the response's write by a 10 s
  per-request deadline, and answers pipelined requests in order.
- **The runtime can be asked to stop: a process-wide shutdown flag behind
  one new `STD_ONLY` builtin, `shutdown_requested()` (`STD_ONLY` 77 → 78).**
  Its first call installs a SIGTERM/SIGINT handler (Unix) or a
  console-control handler for Ctrl+Break/Ctrl+C (Windows); the handler only
  stores an atomic. A second signal takes the default action, and an
  inherited SIGINT or Ctrl+C ignore stays ignored. Programs that never call
  it keep every signal's default action. See
  `docs/adr/0022-process-shutdown-signals.md`.
""", "before"))

EDITS.append(("CHANGELOG.md",
"  delta to 14 → 15 would falsify a correct record of what this increment\n  did.]\n",
"""  [Forward marker, 2026-10-04, branch `examples-03-http-server`: "with no
  router" is true of this increment forever and stays as written. `std/http`
  gained its router, `Server`, on that later branch.]
""", "after"))

run(EDITS)
```

Expected: a `wrote …` line for each of the 15 files, and no `ABORT`.

- [ ] **Step 2: Check the edits are clean**

Run: `cd /d/Projects/nona/nova && git diff --check && git diff --stat && git ls-files --eol nova-spec/60-EXAMPLES.md CHANGELOG.md examples/05-json-api/README.md`
Expected:
- no whitespace errors;
- 15 files changed, insertions only except the two replaced passages;
- the files still `i/lf w/crlf`.

- [ ] **Step 3: Run the set-difference sweep**

Run:
```bash
cd /d/Projects/nona/nova && for t in "Server.get" "Server::" "no router" "Handler" "SIGTERM" "graceful" "pipelin" "03-http-server" "03-producer-consumer" "neither exists under"; do git grep -l -F "$t"; done | sort -u > /tmp/gcm/sweep_all.txt && git diff --name-only main...HEAD | sort -u > /tmp/gcm/sweep_touched.txt && comm -23 /tmp/gcm/sweep_all.txt /tmp/gcm/sweep_touched.txt
```
Expected: a list of tracked files that name one of those tokens and that this branch did not touch.

For each one:
- read the matching lines in context, with wrapped prose flattened, since a line-oriented `grep` misses a phrase split across lines;
- decide whether it is now **false**. Records of their own date's state are not false: historical plans, old CHANGELOG releases, and specs that describe what their increment did.
- give every file now false a dated note in its own amendment style, and list the files you judged still true in your report.

Then re-read the existing dated notes in every touched file (`git diff --name-only main...HEAD`) for a paragraph this branch made stale that the list above missed.

- [ ] **Step 4: Commit**

```bash
cd /d/Projects/nona/nova && git add -A nova-spec docs CHANGELOG.md examples/05-json-api/README.md && git status --short && git commit -m "docs: record 03-http-server, the router and the shutdown flag

Dated notes in every record this makes stale:
- 60-EXAMPLES sections 3, 5, 9 and 10;
- 20-STDLIB section 6, 13-RUNTIME sections 4.1 and 4.4;
- the gate-remeasure-7 paragraph in four files;
- MASTER-SPEC section 9's drift notes;
- ADRs 0009, 0013 and 0019;
- the inventory record, and the std/http and listener designs.

The 05 README and bun-server.js comment are rewritten in place. The
CHANGELOG gets three Added bullets and a forward marker on alpha.2's
'with no router'.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 6: Final verification

**Files:** none changed, unless a check fails. If one does, fix it in the task that owns the code and re-run this task.

- [ ] **Step 1: Full build, then the whole suite**

Run: `cd /d/Projects/nona/nova && cargo build -p nova-runtime 2>&1 | tail -1 && cargo build --locked --workspace 2>&1 | tail -1 && cargo test --locked --workspace --all-features --no-fail-fast > /tmp/gcm/t6_test.log 2>&1; grep -E "^test result|FAILED|panicked" /tmp/gcm/t6_test.log | sort | uniq -c | tail -30`
Expected:
- every `test result` line reads `ok`, with no `FAILED`;
- the new tests appear among the passes;
- the known `#[ignore]`d GC tests stay ignored.

If an `0xC0000005` crash appears in a `*_build_standalone` or `nova test` product, read `docs/adr/0008-attributes-and-test-isolation.md` §4 before re-running: it is a known flake family, and a re-run of that one test is the recorded response.

- [ ] **Step 2: Lint and format as CI does**

Run: `cd /d/Projects/nona/nova && cargo clippy --locked --all-targets --all-features -- -D warnings 2>&1 | tail -3 && cargo fmt --all -- --check && echo FMT-OK && git diff --exit-code Cargo.lock && echo LOCK-UNCHANGED`
Expected: clippy finishes with no warnings, then `FMT-OK`, then `LOCK-UNCHANGED`.

- [ ] **Step 3: Nothing stray**

Run: `cd /d/Projects/nona/nova && git status --short && ls examples/`
Expected:
- an empty status;
- `examples/` lists `01-hello-world`, `02-fibonacci`, `03-http-server`, `03-producer-consumer` and `05-json-api`.

- [ ] **Step 4: Hand off to the whole-branch review**

The execution skill's final review takes over from here, followed by `superpowers:finishing-a-development-branch`.

The branch is not done until CI's three operating systems pass. That CI run is the first measurement of the Unix handler (spec §9), and of Windows console delivery with a handler in place.
