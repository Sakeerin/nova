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
2. **Opt-in and permanent.** Only `std/http`'s `Server` calls it. `listen`
   calls it first before `bind`, which installs the handler, and then on every
   turn of its accept loop. Each connection calls it at its idle ticks and
   after each answer.
   - A program that never calls `listen` keeps every signal's default action.
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
   waits. A connection reads nothing more for a request it takes up after the
   stop, so a client that keeps pipelining cannot extend the drain. Shutdown
   therefore takes at most one tick, plus the requests already in flight
   (those taken up before the stop, and those already received whole), each
   capped at 10 s.

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
