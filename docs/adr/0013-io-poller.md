# ADR 0013 — The I/O poller: a third wake source, polled only at the drained-queue point

**Numbering:** confirmed against `docs/adr/`'s actual contents rather than
trusted from the plan — `0001` through `0012` all exist with no gap, so
`0013` is next.

## Status

Accepted (2026-08-16). The I/O poller and `std/net` increment, branch
`io-poller-std-net` (`docs/superpowers/specs/2026-08-15-io-poller-and-std-net-design.md`).

## Context

Before this increment, `crates/nova-runtime/src/task.rs`'s executor had
exactly two wake sources: a deadline passing (`Wait::Deadline`, `sleep`) and a
task completing (`Wait::Task`, `join`). `run_to_completion`'s drained-queue
branch matched `Option<Instant>` and, on `Some`, called `std::thread::sleep`
directly — the only place this thread ever blocked. Nothing let a task
suspend on socket readiness, and `std/net` did not exist: `docs/adr/0009-async-execution-model.md`
§1 recorded "parking on a genuine external event (I/O readiness) is still
owed to `std/io`" as an open residual gap through two prior amendments.

A TCP client needs to suspend a task while a `connect`, `read`, or `write`
would block, run a sibling task in the meantime, and resume when the socket
is ready or a deadline (for `read_timeout`) passes first. That needs a third
wake source the executor can wait on alongside the existing two, without
touching the collector's thread-locality invariant ADR 0009 §1 already spent
its argument establishing.

## Decision

**A new module, `crates/nova-runtime/src/poll.rs`, gives the executor a third
wake source: socket readiness.** `Wait` gains a third variant:

```rust
enum Wait {
    Deadline(Instant),
    Task(i64),
    Io { socket: RawSocket, interest: Interest, deadline: Option<Instant> },
}
```

The deadline rides inside `Wait::Io` rather than being staged as a second
`PARKED` entry, because one task has exactly one `PARKED` entry and a second
entry would mean every wake path had to remember to remove two. This is what
lets `read_timeout` — the one operation that needs both an I/O wait and a
deadline at once — stage a single combined park (`task.rs`'s `Staged` struct,
which now holds one deadline slot and one I/O slot instead of one wider
`Wait`, so a deadline and an I/O wait from the same poll compose instead of
colliding).

**The poller itself is `select` on Unix and `WSAPoll` on Windows, behind one
`#[cfg]` seam in `poll.rs`, both driven through one platform-independent
entry point:**

```rust
pub fn wait(sockets: &[(RawSocket, Interest)], deadline: Option<Instant>) -> Vec<RawSocket>
```

An empty `sockets` slice with a deadline is definitionally a sleep — nothing
distinguishes it from the old timer-only path — so `wait`'s empty-set branch
calls `std::thread::sleep` itself, and `task.rs`'s own direct call to it is
deleted rather than kept as a second timing path to keep in sync. `poll::wait`
is now the one place this thread ever blocks, for any reason.

**The wait happens only where `run_to_completion`'s ready queue drains, not
once per task turn.** `run_to_completion` already calls the cheap per-turn
check `wake_due` after every single poll — a `Vec` scan over `PARKED`, no
blocking — so a self-requeuing task can never starve a due deadline (or,
after this increment, a due I/O timeout) even though it keeps the queue from
ever looking drained. Polling the socket set itself on every turn the same
way was considered and declined on cost: `wake_due`'s per-turn check is a
`Vec` scan, while a socket poll is a syscall (`select`/`WSAPoll`), and paying
that on every single task turn — most of which involve no I/O wait at all —
is a materially different cost than a scan. So the real wait, `poll::wait`,
is reached only from the drained-queue branch, exactly where `std::thread::sleep`
used to be reached: there is genuinely nothing else to run, so blocking this
thread is correct.

That branch now matches both dimensions at once instead of `Option<Instant>`
alone:

| earliest deadline | I/O parks | action |
|---|---|---|
| none | none | `report_deadlock()` |
| none | some | `poll::wait(&io, None)` — wait on sockets, no timeout |
| some | none | `poll::wait(&[], Some(at))` — this **is** the sleep |
| some | some | `poll::wait(&io, Some(at))` |

`report_deadlock()` is therefore reachable only when both dimensions are
empty — a task parked on I/O with no deadline is legitimate waiting on a peer
that may still answer, not a deadlock. The consequences of that — a
permanently-runnable task starving I/O the identical way it already starves a
deadline, and an I/O wait never being reported as a deadlock at all — are two
new footguns of ADR 0009 §1's existing shape and are recorded there, not
duplicated here (`docs/adr/0009-async-execution-model.md` §1, 2026-08-16
amendment).

## Alternatives considered

- **A poller thread signalling the executor, and a thread pool offloading
  blocking file I/O.** Declined for the same reason ADR 0009 §1 already
  established for thread-per-task generally, applied here to a narrower
  case: `PARKED`, `QUEUE`, `SLOTS` (`fs.rs`), `FILES` (`file.rs`), and the
  collector's own roots (`HEAP`, `PINNED`, `gc.rs`) are all `thread_local!`.
  A second thread's collector cannot see an object allocated on this
  thread's heap, and this thread's collector cannot see anything the second
  thread allocated — the identical use-after-free argument ADR 0009 §1 makes
  against thread-per-task, not a new one. A poller thread or a worker pool
  would be invisible to every one of those tables and to the collector, so
  neither can safely touch Nova state; only a same-thread poll, consulted
  from `run_to_completion`'s own loop, can.
- **IOCP / completion-based file I/O.** Declined for a different reason than
  the thread-based alternatives above, not the same one: this is the only
  route that would make `std/fs` itself genuinely suspend (regular files are
  not readiness-pollable by `select`/`WSAPoll` the way sockets are — that is
  what a completion-based interface exists for), and building it is a
  subsystem in its own right, out of scope for an increment whose surface is
  `std/net`. Nothing about `stack_base()`'s Windows-only precise bounds
  (below) forces this choice the way it bears on the thread-based
  alternatives — a completion port can be polled from the same thread that
  already owns `PARKED`/`QUEUE` — so this is left declined on scope, not
  re-argued on thread-locality grounds it does not actually turn on.
- **A zero-timeout poll after every task turn**, matching `wake_due`'s own
  per-turn cadence exactly instead of confining the real wait to the
  drained-queue point. Declined on cost, per the Decision section above: a
  `Vec` scan on every turn is cheap, a syscall on every turn is not. The
  consequence — a permanently-runnable task starving I/O — is accepted and
  recorded in ADR 0009 §1 rather than engineered around here.
- **Requiring every `Wait::Io` to carry a deadline**, making an I/O wait with
  no deadline unrepresentable by construction and the "never a deadlock"
  question moot. Declined because "block until data arrives, however long
  that takes" becomes inexpressible — the ordinary shape of a `read` or
  `write` with no `read_timeout`-style bound.

## Consequences

- **`std::thread::sleep` no longer appears anywhere in `task.rs`.** That half
  holds: the three matches left in that file are doc-comment prose, not calls.
  **Corrected 2026-08-16 (final whole-branch review): this bullet went on to
  say "the one remaining call lives in `poll.rs`'s own empty-socket-set
  branch". `poll.rs` has four production calls, not one.** The empty-set sleep
  inside `wait`, reached from the drained-queue match's `(Some(at), true)` arm,
  is the only one that implements a *wait*. The other three are
  `ERROR_RETRY_BACKOFF` sleeps on paths that report "nothing ready" for a
  reason retrying cannot fix — the Unix arm's `!any_watched` branch, the Unix
  arm's non-`EINTR` `select` failure, and the Windows arm's non-`WSAEINTR`
  `WSAPoll` failure — and they bound the CPU cost of a stuck condition rather
  than waiting on anything (see `ERROR_RETRY_BACKOFF`'s own doc comment). All
  four are still inside `poll::wait`, so "this thread blocks in exactly one
  place" below is unaffected; only the count was wrong.
- **A parked task's `PARKED` entry is exactly one, always** — `Staged` folds
  a deadline staged alongside an I/O wait into that same `Wait::Io`'s own
  `deadline` field rather than creating a second entry, so every wake path
  (`wake_due`, `wake_ready`, `deadlock_report`) still only ever has to
  consider one entry per task id.
- **This thread blocks in exactly one place, `poll::wait`, whenever a
  deadline, an I/O wait, or both remain to wait on** — there is no second
  timing path and no second I/O path to keep in sync with it. When neither
  remains, the drained-queue match's **first** arm — `(None, true)`, in source
  order — never reaches `poll::wait` at all: `report_deadlock()` aborts the
  process immediately instead. (Corrected 2026-08-16, final whole-branch
  review: this said "fourth arm". It is the first of the four, matching the
  order of the Decision section's own table above.)
- **The two new footguns this decision's shape produces — I/O starvation
  under a permanently-runnable task, and an I/O wait never reported as a
  deadlock — are recorded in `docs/adr/0009-async-execution-model.md` §1**,
  which already carries the identically-shaped deadline-starvation and
  livelock footguns from the park-set amendment, rather than restated here.

- **AMENDED 2026-08-24 (branch `std-net-listener`): `accept` makes a
  many-descriptor process the first realistic shape for `std/net`, and this
  increment records this decision's one unexercised rejection path rather
  than solving it.** `std/net` gained `bind`/`accept`/`local_port`, so a Nova
  program can now hold a listening socket and any number of accepted ones at
  once.

  **This amendment first said `accept` makes that path "reachable for the
  first time", and that the failure mode is a task that stops "silently, with
  nothing to observe". Both are wrong, and how they are wrong matters more
  than the corrections.**

  The rejection is on a descriptor's **number**, not on how many sockets are
  in the wait set: the Unix arm skips a socket `if fd < 0 || fd as usize >=
  libc::FD_SETSIZE`, once per socket, independently of how long the slice is.
  **So one socket has always sufficed.** And a route to it predates this
  increment entirely: `std/fs`'s `open` hands back a `File { fd: Int }` whose
  OS descriptor stays open until an explicit `close`, so descriptors
  accumulated there push a later socket's number up — no listener and no
  `accept` involved anywhere. Whether a particular process can actually get
  there depends on its own `RLIMIT_NOFILE` relative to `FD_SETSIZE`, which
  this module does not control and which on many systems are the same number;
  the point is not that the route is easy but that nothing about `accept` is
  what opens it. **Read, not run**, like the path itself.

  What is true is that `accept` makes a many-descriptor process the first
  *realistic* shape for this module, which is what makes the ceiling worth a
  reader's attention now rather than in principle. The Unix arm selects with
  `select`, which rejects any descriptor at or above `FD_SETSIZE`; and because
  only `read_timeout` stages a deadline, a task whose descriptor is rejected
  is **never woken again and never surfaces an error**. Windows `WSAPoll` has
  no equivalent ceiling. So the failure mode a many-connection server reaches
  first is a task that simply stops.

  **Nothing in the language can observe that, but the process is not silent
  — both halves matter.** No `IoError` ever reaches the Nova program, so
  nothing written in this language can branch on it, retry, or report it;
  that is the real defect and it is unchanged. But the process does emit a
  rate-limited `tracing::warn!` line naming the offending socket and
  `FD_SETSIZE` (at most one line per second, gated because that loop runs
  once per socket per call), and `nova-cli` installs a subscriber at `WARN`
  by default, so an operator reading stderr sees it. The poller's own comment
  at that call site already says "`tracing::warn!` rather than silence" --
  cited by its wording rather than its line, because three line citations on
  this branch went stale or were wrong.

  That path was already labelled, and the label is still accurate. This
  file's own poller reads, verbatim at `crates/nova-runtime/src/poll.rs`:
  "**Still reasoned, not measured:** the `FD_SETSIZE` rejection path and the
  non-`EINTR` error/backoff path below. No test reaches either -- one needs a
  descriptor number above `FD_SETSIZE`, the other a real socket-level fault --
  so both remain read-not-run on every platform, Windows included."

  Nothing in this increment changes that. What changed is only which shapes
  of program make the ceiling plausible to reach — not whether it can be
  reached, which it always could, and not whether anything has reached it,
  which nothing has. The decision to leave it stands on the same ground the
  Decision section gives for choosing `select`/`WSAPoll` over IOCP, and a cap
  or a `poll(2)`-based Unix arm is a later increment's to make — but a reader
  deciding whether to point `std/net` at a socket that accepts should know
  that this is where it breaks and that nothing has ever executed the break.

## References

- Design: `docs/superpowers/specs/2026-08-15-io-poller-and-std-net-design.md`
  §3.1 (the drained-queue-only wait), §3.2 (`Wait` gains a variant), §3.3
  (`Staged` widens), §3.4 (an I/O wait is never a deadlock), §6 (alternatives,
  in the design's own words)
- `crates/nova-runtime/src/poll.rs`: `RawSocket`, `Interest`, `wait`,
  `set_nonblocking`, and the `#[cfg(unix)]`/`#[cfg(windows)]` `platform_wait`
  arms (`select`/`WSAPoll`)
- `crates/nova-runtime/src/task.rs`: `Wait`, `Staged`, `stage_park`,
  `stage_io_park`, `run_to_completion`'s drained-queue match, `earliest_deadline`,
  `io_parks`, `wake_ready`, `wake_due`, `deadlock_report`/`report_deadlock`
- `crates/nova-runtime/src/net.rs`: the first production source of a
  non-empty socket set for `poll::wait` to see — `connect`, `read`, `write`,
  `read_timeout` and (since 2026-08-24) `accept` each stage a `Wait::Io`
  through `stage_io_park`, `read_timeout` being the only one that passes a
  deadline; no count is given here because an earlier wording named four and
  `accept`'s arrival falsified it; `task.rs`'s `run_to_completion` remains
  `poll::wait`'s only caller
- `docs/adr/0009-async-execution-model.md` §1: the thread-local-heap argument
  this decision's first alternative reapplies, and the 2026-08-16 amendment
  recording this decision's two new footguns
- `docs/adr/0012-file-descriptor-lifecycle.md`: `stack_base()`'s Windows-only
  precise bounds and why collection does not run at all off Windows —
  the same property this decision's second alternative (IOCP) notes does not
  itself decide the question, unlike the first
