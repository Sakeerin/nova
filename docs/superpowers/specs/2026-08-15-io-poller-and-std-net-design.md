# I/O poller and `std/net`: design

**Increment 4 of 4** in the std/fmt + std/io decomposition, and the one the
first three were sequenced to make possible. Increments 1–3c gave Nova files,
bytes, per-task payload slots, `Read`/`Write`, the standard streams, and a
handle-lifetime model. None of them made anything suspend. This one does.

**Base:** `main` at `3973dfd`, 404 commits, 0 merge commits, 921 tests
(8 deliberately ignored), clean tree. Branch `io-poller-std-net`.

---

## 1. What this builds, and why it is the hard one

**Every `async fn` in Nova's standard library today is a lie of omission.**
`std/fs`'s eleven and `std/io`'s stream operations are spelled `async` and
never suspend: a call blocks the whole executor for its duration, so a sibling
task makes no progress during a large read. ADR 0009 §1 documents this as a
footgun. The executor has exactly two wake sources — a deadline passing and a
task completing — and neither of them is I/O.

This increment adds the third, and gives it something real to wake on.

Two halves, deliberately coupled:

- **A poller**, `crates/nova-runtime/src/poll.rs`, owning one operation: wait
  until a socket in this set is ready, or this instant passes, whichever comes
  first. Nothing else in the runtime learns what a socket is.
- **`std/net`**, a TCP *client* — `connect`, `close`, `read_timeout`, and
  `Read`/`Write` impls — because a poller with nothing to poll is machinery
  nobody can exercise, and this project has learned what unexercised machinery
  costs.

**Why not files.** Regular files are not readiness-pollable on any operating
system; that is what completion-based interfaces (IOCP, io_uring) exist for.
Making `std/fs` suspend is a subsystem in its own right and is out of scope.
After this increment `std/fs` suspends nowhere and `std/net` suspends
everywhere, which is an honest asymmetry and is documented where a reader meets
it rather than hidden.

**Why not stdin.** Windows does not report readiness on a redirected handle,
and this project's test suite always redirects it, so the suspending path would
be the untested one. *Reasoned, not measured* — no probe was run.

---

## 2. Surface

```nova
// std/net
pub record TcpStream { fd: Int }

pub async fn connect(addr: String) -> Result<TcpStream, IoError>
pub async fn close(s: TcpStream) -> Result<(), IoError>
pub async fn read_timeout(s: TcpStream, max: Int, ms: Int) -> Result<Bytes, IoError>

impl Read for TcpStream
impl Write for TcpStream
```

**The handle model is `File`'s, reused rather than reinvented.** A thread-local
table keyed by a never-reused `Int`; **absence from the table is closedness**,
so a closed fd, a stale fd, and an fd a Nova program forged by hand
(`TcpStream { fd: 9999 }` — record fields are not privacy-enforced) all miss the
lookup and become one `IoError { kind: Other }`. `close` is idempotent, because
Nova has no move checking and `close(s)` then `s.read(..)` compiles. The
descriptor leaks if `close` is never called; ADR 0012 already argues why
close-on-collect is foreclosed, and `std/net` inherits that argument by
reference rather than restating it.

**No new error kind is needed, and that is a real finding rather than a
convenience.** `IoErrorKind` already declares `TimedOut` and
`ConnectionRefused`, and `fs.rs` already maps them (`TIMED_OUT`,
`CONNECTION_REFUSED`) — both shipped with `std/io`'s error types and **neither
has ever had a producer**, because no filesystem operation can produce either.
`read_timeout` and a refused `connect` are their missing producers. ADR 0011's
table does not change.

This also improves a documented gap. `fs.rs`'s comment enumerating which kinds
are pinned by a fixture provoking the real OS condition currently names four;
this increment takes it to six, and that comment is amended as part of the work.

---

## 3. The execution model change

### 3.1 One wait, at the drained-queue point

`run_to_completion`'s drained branch stops matching `Option<Instant>` and starts
matching both dimensions at once:

| earliest deadline | I/O parks | action |
|---|---|---|
| none | none | `report_deadlock()` — unchanged |
| none | some | wait on sockets, no timeout |
| some | none | wait on no sockets, with timeout — **this is the sleep** |
| some | some | wait on sockets, with timeout |

**`std::thread::sleep` is deleted, not replaced.** A wait over an empty socket
set with a timeout *is* a sleep, so there is no second timing path to keep in
sync.

**`report_deadlock()` becomes structurally unreachable except where it belongs.**
Today it is the `None` arm of a match on `Option<Instant>`, so any park lacking
a deadline falls into it — a task merely blocked on a socket would be told it
deadlocked. In the shape above it is reachable only when both dimensions are
empty, which is exactly the condition it was always meant to express.

### 3.2 `Wait` gains a variant

```rust
enum Wait {
    Deadline(Instant),
    Task(i64),
    Io { socket: RawSocket, interest: Interest, deadline: Option<Instant> },
}
```

The deadline is folded into the variant rather than parked as a second entry,
so no wake path has to remove two entries for one task.

**Three sites must be updated; two of them are forced and one is not.**
`earliest_deadline` and `deadlock_report` match `Wait` exhaustively with no
wildcard, so a new variant is a compile error at both. **`wake_due`'s `retain`
uses `_ => true` and is not forced** — and it *does* need a real arm, because a
`Wait::Io` whose deadline has passed is the timeout firing and must wake. An
earlier draft of this design claimed the wildcard could stand; that was only
true while I/O waits were untimed.

### 3.3 Staging widens; the abort does not loosen

`stage_park` currently holds `Cell<Option<Wait>>` and aborts on a second stage
in one poll. That abort catches an inner future's `POLL_PENDING` failing to
propagate and must keep doing so. So the staged value becomes two slots rather
than one wider one:

```rust
struct Staged { deadline: Option<Instant>, io: Option<IoWait> }
```

Staging a deadline when `deadline.is_some()` still aborts; staging an I/O wait
when `io.is_some()` still aborts. Exactly one new combination — a deadline and
an I/O wait, from one `read_timeout` — becomes legal. The abort's purpose is
preserved; its scope narrows by one case.

### 3.4 An I/O wait is never a deadlock

When every task is parked and at least one waits on I/O, the executor blocks in
the poller rather than reporting. This follows ADR 0009's own livelock
precedent: distinguishing "waiting for a peer that will send" from "waiting for
a peer that never will" is the halting problem, and this project already
declines to report the analogous case.

**The cost is stated rather than hidden:** a program waiting on a peer that
never sends hangs silently, and `nova test` has no per-test timeout.

### 3.5 How a task learns why it woke

The poll ABI is frozen — two statuses, and `task_ctx` always null:

```rust
pub type PollFn = unsafe extern "C-unwind" fn(state: *mut u8, task_ctx: *mut u8) -> i64;
```

So the wake reason cannot ride the call. It travels through the per-task slot table
increment 3a built, the same channel payloads already use: `read_timeout` stages
the park, and on re-poll takes a slot saying ready or timed out.

**Corrected 2026-08-16 (branch `io-poller-std-net`, Task 7 review): the
paragraph above is factually wrong, and no such mechanism was ever built.**
`crates/nova-runtime/src/task.rs` has no `SLOTS` and no `stash` call anywhere
in it — `grep -n "SLOTS\|stash(" crates/nova-runtime/src/task.rs` returns
zero matches. The "per-task slot table" this paragraph names is `fs.rs`'s
`SLOTS`, built for carrying a *payload* (a byte buffer, a count) from one
poll to a later one; no wake path in `task.rs` ever writes a wake reason into
it, or into anything else, because `task.rs`'s wake paths (`wake_ready`,
`wake_due`) only ever move a parked task back onto the ready queue.

What shipped instead, in `crates/nova-runtime/src/net.rs`'s
`poll_read_timeout`, needs no channel at all: every poll retries the read
first, unconditionally — if data (or EOF) is there, that settles it
regardless of what woke the task. Only a `WouldBlock` result asks the second
question, and it asks the future's *own* state object, not a shared table:
has the absolute deadline stored in `RT_SLOT_DEADLINE` already passed? This
re-derives "ready vs. timed out" from retried, current state on every poll
instead of being told the answer — the identical pattern `finish_connect`
already uses to re-derive a connection refusal from `SO_ERROR` rather than
being told one occurred, and `poll_read_timeout`'s own doc comment in
`net.rs` states both the mechanism and that analogy directly.

---

## 4. The runtime boundary

**A parker is a Rust-built future with its own `PollFn`, not a Nova `async fn`
over a blocking intrinsic.** Two parkers already exist — `sleep`, which stages a
`Wait::Deadline`, and `join`, which stages a `Wait::Task` — and they share one
shape: a constructor intrinsic allocates a state object, and its poll function
reads a tag, so the first poll stages the park and returns `POLL_PENDING` while
the next writes the output and returns `POLL_READY`. `sleep` is the closer
model for `std/net`, because a deadline is a wake source the executor must
later notice, which is structurally what an I/O readiness wait is.

`std/fs`'s shape (an `async fn` calling a blocking intrinsic) cannot suspend and
is exactly what this increment exists to move past. So each suspending `net`
operation is a Rust future:

```
first poll:  try non-blocking → would block?  stage_park(Wait::Io{..}); POLL_PENDING
next poll:   retry → ready?     write output; POLL_READY
             deadline passed?   write TimedOut; POLL_READY
```

The Nova side hands it back with **the `Future<T>` spelling increment 3b already
proved** — `fn read(self, max: Int) -> Future<Bytes>`, returning the
intrinsic-built future unawaited. That spelling exists because `async fn` in a
trait declaration is `E0900`; it turns out to be the same mechanism that lets a
Rust-built future reach Nova. Both need a future named in a signature and not
awaited there.

**`connect` is a two-phase parker.** Set non-blocking, call `connect`, expect
would-block, park on *write* readiness; the second poll checks the socket error
and yields either the stream or `ConnectionRefused`. A blocking `connect` would
pass every loopback test while defeating the increment — this project's
"passes for the wrong reason" hazard — so it is specified out.

**This Rust is the poll boundary, not merely near it.** No `unwrap`, no
`expect`, no slice indexing, no fallible `format!`; `try_borrow_mut` with an
`abort_with` backstop on the socket table. `task.rs` already holds hand-written
poll functions — `poll_yield_once` and `poll_sleep` among them — so `std/net`'s
join an existing family and should be read alongside them rather than invented
fresh. What is new is that these are the first outside `task.rs`.

**Seams.** Each new intrinsic crosses the ten edit sites increments 2 and 3c
mapped — three in nova-resolver (`Builtin` variant, Nova-visible name,
`STD_ONLY`), two in nova-typeck, three in nova-mir, one in `lower.rs`, and one
in `symbols()`. The last is keyed by the **lowercase symbol string** and is
invisible to a PascalCase grep; a missing entry fails only for programs that
actually call the intrinsic.

**`STD_MODULES` goes 7 → 8**, its first change since increment 2. Note that a
directory under `std/` is not automatically a module: `std/test` is a directory
and is *not* an `STD_MODULES` entry, so `std/net` needs both, spelled
`"$std.net"`. `RESERVED_TYPE_NAMES` stays 7 — `TcpStream` is an ordinary
`std/net` definition, so no name is reserved and no existing program breaks.
Because `STD_ONLY` builtins are seeded into each std module's own scope before
its items are collected, **`std/net` must not define anything sharing a
builtin's name**, or it hits `E0002` against the pre-seeded entry.

---

## 5. Tests

**One fixture decides whether this increment did anything.** A blocking
`connect`, a blocking read, or `POLL_READY` returned where `POLL_PENDING`
belongs all produce *correct output* against a loopback echo server — a
round-trip fixture passes under every one of them. So a fixture must assert
interleaving: task A connects and reads, task B increments a counter and
yields, and the golden shows **B ran while A was waiting**. Without it, nothing
in the suite distinguishes a real poller from a blocking one.

**The harness is the server.** `run_tests.rs` binds a loopback listener on an
ephemeral port per fixture and echoes.

**Port discovery.** The harness writes the port to a path derived from the test
name; the fixture reads it with `fs::read_to_string`. This needs no new surface,
but the path is run-invariant — the same latent race the debt queue records for
`write_test_project`. Two concurrent cargo runs of the same test would collide.
**Stated in the fixture's own comment**, not left to be discovered. The
alternatives were a generated fixture source (no race, but breaks the
static-file-plus-golden convention every other runtime fixture follows) and a
fixed port (flaky).

**Mutations that must die:**

1. `stage_park` deleted from the read poll — the task spins instead of parking.
2. Read/write interest transposed at registration.
3. `POLL_PENDING` → `POLL_READY` on would-block.
4. The `read_timeout` deadline not honoured.
5. A task parked on I/O with no deadline reporting a deadlock — the
   false-positive this restructuring exists to prevent.
6. `connect` made blocking — which must fail the interleaving fixture and
   nothing else.

Mutation 6 is the one that proves the interleaving fixture works. If it dies
only under some *other* test, that fixture is not doing its job.

---

## 6. Alternatives considered

- **A zero-timeout poll after every task turn**, mirroring the split between
  `wake_due` (every poll, cheap) and `wake_due_deadlines` (only when drained).
  That function's own doc comment explains why the drained-only check is
  insufficient for deadlines — a permanently-runnable task means the queue never
  drains — and the argument transfers to I/O. **Declined on cost**: `wake_due`
  is a `Vec` scan, a per-turn poll is a syscall. The consequence is that **a
  permanently-runnable task starves I/O**, which joins ADR 0009 §1's existing
  family ("a permanently-runnable task masks a deadlock among the others")
  rather than opening a new one.
- **A poller thread signalling the executor**, and **a thread pool offloading
  blocking file I/O**. Both declined for one reason: `PARKED`, `QUEUE`,
  `SLOTS`, `FILES` and the GC roots are all `thread_local`, and `stack_base()`
  returns `None` off Windows so collection does not run there at all. A pool
  thread would see none of it.
- **IOCP / completion-based file I/O.** The only route to making `std/fs`
  genuinely suspend, and a subsystem in its own right.
- **Requiring every `Wait::Io` to carry a deadline**, making the false-deadlock
  unrepresentable by construction. Declined because "block until data arrives"
  becomes inexpressible.
- **A server side** (`bind`, `accept`, `TcpListener`), **UDP**, and **Unix
  sockets** — all named in the spec's `std/net` line, none built here.

---

## 7. Definition of done

- A Nova program connects to a loopback server, writes, reads, and closes.
- **A second task demonstrably runs while the first waits**, shown in a golden.
- `read_timeout` returns `TimedOut` against a server that never replies.
- `connect` to a closed port returns `ConnectionRefused`.
- A task parked on I/O with no deadline does **not** report a deadlock.
- `std::thread::sleep` no longer appears in `task.rs`.
- All six mutations above fail at least one test, and mutation 6 fails the
  interleaving fixture specifically.
- `nova-spec/20-STDLIB.md` gains a `std/net` section and §4 gains the
  fs-vs-net asymmetry note; a new ADR records the poller and the third wake
  source; ADR 0009 §1 gains the two new footguns (I/O starvation under a
  permanently-runnable task; an I/O wait never reported as a deadlock);
  `fs.rs`'s pinned-kinds comment goes four → six; `CHANGELOG.md` records
  `STD_MODULES` 7 → 8.
- Suite green with the new tests; clippy clean under
  `--all-targets --all-features -- -D warnings`; `fmt --all --check` clean; and
  **no `reason = "…"`** in any lint attribute (MSRV 1.78).

**Corrected 2026-08-16 (branch `io-poller-std-net`, final whole-branch review):
the mutation-6 bullet above overstates what shipped, and the branch does not
meet it as written.** Mutation 6 in the form §5 specifies it — `connect` made
genuinely OS-blocking, by issuing it on a blocking socket with
`set_nonblocking` applied only on the success paths so the refusal path stays
byte-identical — **passes the whole suite** (970 passed / 0 failed / 8 ignored /
44 targets). Two independent causes, each sufficient on its own:

- **`tests/runtime/net_interleave.nova` calls `connect` in `main`, before
  either task is spawned.** That was deliberate and is the right call for what
  that fixture measures (its own header explains the confound it removes), but
  it takes `connect` out of the interleaving window the fixture asserts on. The
  variant the fixture does kill is the *wider* one that also drops
  `set_nonblocking`, which makes the `read` blocking too — so what is actually
  guarded end to end is **the read-blocking shape**, not a blocking `connect`.
- **`net.rs`'s `start_connect` folds `rc == 0` and would-block into one
  `Started::WouldBlock`.** Both `platform_connect` arms return `Ok(stream)` for
  `rc == 0` and for `WSAEWOULDBLOCK`/`EINPROGRESS` alike, so
  `connect_parks_on_its_first_poll_rather_than_completing_synchronously`
  returns `POLL_PENDING` either way and structurally cannot observe whether the
  syscall blocked.

**An isolated connect-only blocking syscall is therefore not guarded, and no
structural guard was added for it.** Asserting would-block at that seam is
platform-fragile in a way that would trade a documentation gap for a flaky
test: a loopback `connect` legitimately returns `rc == 0` on Linux, which is
precisely why the two outcomes were folded in the first place. The DoD bullet
above should be read as: mutations 1–5 each fail at least one test, and
mutation 6's read-blocking form fails the interleaving fixture specifically.

---

## 8. What is measured and what is reasoned

Measured in the tree at `3973dfd`, by reading the source:

- `Wait` has exactly two variants; `earliest_deadline` and `deadlock_report`
  match it exhaustively; `wake_due`'s `retain` uses `_ => true`.
- `wake_due_deadlines` calls `std::thread::sleep`.
- `stage_park` holds `Cell<Option<Wait>>` and aborts on a second stage.
- The drive loop matches `Option<Instant>`, so `report_deadlock()` is a default
  arm reachable by any park lacking a deadline.
- `IoErrorKind` declares eight kinds including `TimedOut` and
  `ConnectionRefused`; `fs.rs` maps both and neither has a producer.
- `sleep` is a Rust-built future with a tag-based state machine that stages a
  park and returns `POLL_PENDING`. It is **not** the only parker: `join` stages
  a `Wait::Task` the same way, and `task.rs` already contains hand-written poll
  functions including `poll_yield_once` and `poll_sleep`. An earlier draft of
  this spec called `sleep` the only parker and called `std/net`'s poll functions
  the first hand-written ones since the executor; both were false, and the
  quantifier sweep is what caught them.
- `STD_MODULES` has 7 entries; `std/test` is a directory that is not one of
  them.

Reasoned, not measured — each should be probed before the code that depends on
it is written:

- Windows will not report readiness on a redirected stdin handle.
- Sockets are cleanly readiness-pollable on all three CI platforms.
- A non-blocking `connect` reports its result through the socket error, and
  loopback connects complete fast enough that a blocking implementation would
  pass the round-trip fixture.
