# `std/net` listener: `bind`, `accept`, `local_port`

**Status:** approved 2026-08-23. Design only.

**Goal.** Give `std/net` a server side — `TcpListener`, `bind`, `accept`, `local_port` — so that
Phase 2 position 10 (`std/http`) has a transport to build a server on.

**Base.** `main` == `origin/main` == `3901db5`, 539 commits, 0 merge commits, 1057 tests (8 ignored),
clean tree, tagged `v0.2.0-alpha.1`.

---

## 1. Why this increment exists, and why it is not `std/http`

`00-MASTER-SPEC.md:240` puts `std/http` at Phase 2 position 10 and says "server first". **There is
no server transport beneath it.** `std/net/lib.nova` is 147 lines and outbound only: `TcpStream`
(`:47`), `connect` (`:52`), `close` (`:66`), `read_timeout` (`:83`), `impl Read` (`:106`),
`impl Write` (`:144`). Five intrinsics, all client. Every `TcpListener` in the tree is inside
`#[cfg(test)]` or the CLI harness.

`20-STDLIB.md:1509-1512` — §16, the `std/net` section — says so itself:

> `bind`/`accept`/`TcpListener` (a server side), UDP, and Unix sockets are all named in §1's
> module-index line for `std/net`, but none of the three is built by this section; each remains a
> future increment's to add.

**Three records assert position 10 is "unstarted and unblocked"** — `13-RUNTIME.md:442`,
`20-STDLIB.md:677`, `ADR 0018:73` — and all three omit that its transport does not exist. This
increment closes that gap rather than papering over it, and it belongs to **position 9
(`std/net`)**, which the sentence above already assigns it to. So this is finishing a position in
order, not a build-order deviation: **no new ADR under ADR 0014's out-of-order index.**

One imprecision in that same sentence, found while reading it: **§1's index line reads
`std/net          TCP/UDP/Unix sockets`** (`20-STDLIB.md:16`). It names UDP and Unix sockets; it
does **not** name `bind`, `accept` or `TcpListener`, which are only implied by "TCP". The amendment
this increment writes should correct that as it goes.

---

## 2. Scope

**In.** `net_listen`, `net_accept`, `net_local_port`; `TcpListener` and its four Nova methods; one
terminating fixture that exercises a real connection end to end; Rust unit tests for the two
non-suspending intrinsics; the records in §8.

**Out, deliberately.** Concurrency is *not* proven by this increment. No fixture will park two
sockets at once, no `select`/`race`/`join_all` is added, the `FD_SETSIZE` skip path is not fixed,
UDP and Unix sockets stay unbuilt, and `IoErrorKind` gains no variant. Each exclusion is recorded
in §6 or §8 as a known limitation, not left silent.

---

## 3. The three intrinsics

All three live in `crates/nova-runtime/src/net.rs` and follow `connect`/`close`'s established
contracts exactly. Nothing new is needed in the poller: **accept-readiness is read-readiness** on
both `select` and `WSAPoll`, so `Interest::Read` (`poll.rs:50-53`) covers it and `Interest` gains no
variant.

### 3.1 `net_listen(addr) -> i64` — non-suspending

Parse `addr`, `TcpListener::bind`, `set_nonblocking(true)`, insert into the handle table, stash the
new fd via `Slot::Buffer`, return status `0`.

Non-suspending, so **no poll function** — the shape of `net_close`, not of `connect`. The fd rides
back exactly as `connect`'s does: `stash(Slot::Buffer, gc_bytes(&n.to_le_bytes()))`
(`net.rs:162-174`), eight little-endian bytes read on the Nova side by `std/io`'s `decode_count`
(`std/io/lib.nova:256`). **No new slot and no new encoding.**

### 3.2 `net_local_port(fd) -> i64` — non-suspending

Returns status `0` with the kernel-assigned port stashed as eight little-endian bytes in
`Slot::Buffer`, reusing `decode_count`.

The port travels in the buffer rather than in the return value **because the status word is already
spoken for**: the established contract is that the status word *is* the error kind, mapped by
`io_error_kind_of` (`std/io/lib.nova:83`) against `fs::fail` (`fs.rs:413`). A port returned in the
status word would make `0` ambiguous and every non-zero port indistinguishable from an error.

### 3.3 `net_accept(fd) -> Future<Int>` — suspending

The only one that suspends, so it gets **one** `extern "C-unwind"` poll function, written
level-triggered and tag-free like `poll_read`:

1. Look up the listener. Wrong kind or absent is an error (§5).
2. `accept()`. On success, insert the new `TcpStream` in the table, stash its fd as eight
   little-endian bytes, return `POLL_READY` with status `0`.
3. On `WouldBlock`, `stage_io_park(listener_fd, Interest::Read, None)` and return `POLL_PENDING`.
   `stage_io_park`'s signature is `(socket: RawSocket, interest: Interest, deadline: Option<Instant>)`
   (`task.rs:661`); the deadline is `None` because this increment adds no accept timeout.
4. Any other error settles the future with that error's status.

**Level-triggered and tag-free is mandatory, not stylistic.** The poll ABI is frozen —
`PollFn = unsafe extern "C-unwind" fn(*mut u8, *mut u8) -> i64`, `POLL_PENDING = 0`,
`POLL_READY = 1`, `task_ctx` always null — and no wake path records *why* a task woke, so every
future must re-derive its own state on each poll. `poll_read_timeout` (`net.rs:942-958`) is the
worked example.

**No panic may cross the poll boundary**, and this is enforced mechanically rather than by review:
`no_net_intrinsic_can_panic` (`net.rs:2683`) fails the build on `.borrow_mut()`, `.borrow()`,
`unwrap()`, `.expect(`, `panic!` or `format!` anywhere in the production half of `net.rs` —
including inside a doc comment. New code must use the existing non-panicking accessors.

`STD_ONLY` moves **66 → 69** (`crates/nova-resolver/src/lib.rs:718`). `STD_MODULES` stays at **13**
(`:1291`) — `$std.net` is already registered. Each intrinsic pays the 12-site tax whose counting
rule ADR 0018 §3 states.

---

## 4. How listeners live in the runtime

Today: `SOCKETS: RefCell<HashMap<i64, TcpStream>>` (`net.rs:107`), `NEXT_FD: Cell<i64>` starting at
1 (`:111`), and `with_fd<R>(fd: i64, f: impl FnOnce(&mut TcpStream) -> R) -> Option<R>` (`:120`).
**Absence from the table is closedness** — the model ADR 0012 argued for.

**Decision: one table holding a two-variant enum.** `Sock::Stream(TcpStream)` and
`Sock::Listener(TcpListener)`, with `with_fd` splitting into kind-checked accessors.

Why: it keeps **one** `NEXT_FD` space, **one** closedness invariant and **one** `close`, and it
makes type confusion an explicit error. `read` on a listener fd *fails as wrong-kind* rather than
succeeding oddly. The cost is a handful of match arms at existing `with_fd` sites, and those arms
are where the wrong-kind error is produced.

**Rejected: a second table** (`LISTENERS: HashMap<i64, TcpListener>`). Smaller diff and no existing
call site touched, but closedness becomes two lookups, `close` must consult both, and `read` on a
listener fd would report **closed** — wrong and plausible, which is the worse failure mode of the
two.

**Rejected: raw fds with `from_raw_fd`.** No table entry at all, but it fights the model ADR 0012
established and invites double-close.

---

## 5. Nova surface

```nova
pub record TcpListener { fd: Int }

pub fn bind(addr: String) -> Result<TcpListener, IoError>
pub fn local_port(self) -> Result<Int, IoError>        // on TcpListener
pub async fn accept(self) -> Result<TcpStream, IoError>
pub async fn close(self) -> Result<(), IoError>        // on TcpListener
```

`accept` returns a `TcpStream` carrying the accepted fd, indistinguishable from a connected one, so
every existing `Read`/`Write`/`read_timeout`/`close` method works on it unchanged.

**`bind` and `local_port` are plain `fn`, not `async fn`.** Neither can suspend, and forcing
`.await` would misstate the cost. This **diverges from `close`**, which is declared `async fn` while
§16 itself says close "does not suspend: `close` calls a plain status-returning intrinsic with no
`.await`" (`20-STDLIB.md:1507`). That is a wart, and this increment declines to propagate it. The
divergence must be documented at both new signatures, naming `close` as the inconsistent neighbour,
so a later reader finds the reason rather than the discrepancy.

**`TcpListener::close` is nevertheless `async fn`, and this is deliberate rather than an oversight
of the rule just stated.** It cannot suspend either, so the rule above would make it a plain `fn`.
It is not, because a server closes two handles in sequence — `stream.close().await` then
`listener.close()` — and a reader should not have to remember which of two identically named methods
on two socket handles needs `.await`. Matching the shipped sibling wins over matching the new rule,
and changing `TcpStream::close` to match instead is out of scope: it is a breaking change to an API
that has already shipped. So the module ends up with a stated principle and one stated exception to
it, and **both must be written down at the signatures** — a principle with a silent exception is
worse than either.

---

## 6. Error model, and two collapses to record

Every failure goes through `fail`'s existing contract, where the **status word is the error kind**.
`IoErrorKind` has exactly eight variants — `NotFound`, `PermissionDenied`, `AlreadyExists`,
`InvalidData`, `Interrupted`, `TimedOut`, `ConnectionRefused`, `Other` (`std/io/lib.nova:41-49`) —
and `fail`'s match ends `_ => OTHER`.

**`AddrInUse` is not among them.** So "address already in use" — the single most likely `bind`
failure, and the one a user hits first — collapses to `IoError { kind: Other }`, discriminable only
by the operating system's own message text, which tests must normalise
(`std/io/lib.nova:54-56`). Adding a variant is a wire-contract change touching `fs.rs`'s `fail` and
`std/io`'s `io_error_kind_of` together; it is **out of scope**, and the collapse is to be recorded
at `bind`'s doc comment and in §16's amendment rather than left for a user to discover.

**Wrong-kind access collapses too.** `read` on a listener fd, or `accept` on a stream fd, produces
`Other` for the same reason. Record it at the same two places.

---

## 7. Concurrency constraints this increment inherits and does not fix

These are properties of the existing runtime that a listener makes reachable. Each must be recorded
in code or records; none is fixed here.

1. **One socket wait per task, enforced by process abort.** `try_stage` reports a same-kind clash
   (`task.rs:511-530`) and `stage_park` turns it into `abort_with` (`task.rs:642`); the behaviour is
   pinned by "a second I/O wait in the same poll must collide" (`task.rs:3676`) and again with a
   deadline present (`:3693`). A listener plus a live connection is therefore **two tasks minimum**,
   and there is no `select`/`race`/`join_all` to avoid that.
2. **The `FD_SETSIZE` skip path becomes reachable for the first time.** On Unix, `poll::wait` uses
   `select`, and an fd at or above the ceiling is skipped and never watched again
   (`poll.rs:302-313`); because only `read_timeout` stages a deadline, that task is **never woken
   again and never surfaces an error**. Windows `WSAPoll` has no such ceiling. Neither this path nor
   the poller's non-`EINTR` error path has ever executed on any platform (`poll.rs:254-257`). A
   many-connection server reaches both first. **This is the single most important thing this
   increment records and does not solve.**
3. **A hung connection is silent.** An untimed `Wait::Io` is never reported as a deadlock
   (`task.rs:1006-1009`), and `block_on` cannot return while any task is parked (`task.rs:992-994`),
   so there is no graceful-shutdown path.
4. **Inter-task channels are the deadlock landmine, not mutexes.** `std/sync` waits by spinning
   `yield_now().await`; a `Mutex` never held across an `.await` can never freeze anything, but
   `recv` on an empty channel spins unconditionally, so a consumer waiting on a socket-parked
   producer can never be woken. **The fixture in §8 must therefore use no channel** — and it does
   not need one, because `bind` is synchronous and the port is known before anything parks.

---

## 8. Testing

**One fixture, self-contained and terminating.** `tests/runtime/net_listener_accept.nova`, with a
`.stdout` golden, plus an explicit `#[test]` in `crates/nova-cli/tests/run_tests.rs` —
**registration is not automatic**, and a fixture registered nowhere runs zero tests.

Shape: `bind("127.0.0.1:0")`, read the assigned port with `local_port`, `spawn` a client task with
that port **as a plain argument**, server `accept`s, reads one message, writes it back, both sides
`close`, the process exits with deterministic stdout. No channel, no sleep, and **no assertion on
elapsed time**.

That last point is a convention with evidence on both sides, so state it accurately rather than as
a prohibition: there is **no blanket ban** on timing assertions in this suite. The established
practice is to prove parking by **wake order rather than elapsed time** — `sleep(ms)`, "the first
primitive that parks rather than spins, proved by wake *order* rather than by elapsed time"
(`run_tests.rs:1196-1197`). The one test that does assert wall-clock time (`elapsed < 15s`,
`run_tests.rs:5151-5161`) is **precisely the test observed flaking twice during this project's last
increment**, with its spawned binary dying at `0xc0000005`. That is the argument for order over
duration here, and it is stronger than a rule that does not exist.

**Port 0 is required, not preferred.** A fixed port collides when test binaries run concurrently.
The existing `dead_addr()` flake (`net.rs:1571-1576`) is that failure: it binds `127.0.0.1:0` and
then **drops the listener**, so a concurrent bind can steal the port. This fixture keeps its
listener alive for the listener's whole life, so it does not reproduce that shape — and the design
must not introduce a second instance of it.

**Rust unit tests** in `net.rs` for the two non-suspending intrinsics: `net_listen` on a bindable
address and on an unbindable one; `net_local_port` on a bound listener, on a stream fd (wrong kind),
and on an absent fd.

**Mutations, run and reported by name.** Re-reading cannot establish coverage. At minimum: invert
the `WouldBlock` branch in the accept poll function so it settles instead of parking; drop the
`set_nonblocking(true)` call; and return the wrong fd from `net_listen`. Each must name the tests
that fail. A mutation that leaves a loop unsatisfiable **hangs** rather than fails — verify the
mutant's behaviour, not the shipped code's.

---

## 9. Records

- **`20-STDLIB.md` §16** — a dated amendment in house style
  (`**AMENDED <date> (branch \`<branch>\`):**`). The sentence at `:1509-1512` becomes two-thirds
  false and must say so in place. The amendment also carries: §1's index line names UDP and Unix
  sockets but not the server side; the four new signatures; the `bind`/`local_port` non-`async`
  divergence from `close` and why; and both error collapses from §6.
- **ADR 0013 (the I/O poller)** — a dated amendment, **not a new ADR**, because what is being
  accepted is a property of the poller's own design: this increment knowingly makes the
  never-executed `FD_SETSIZE` skip path reachable for the first time.
- **`CHANGELOG.md`** — `[Unreleased]`, `### Added`, lines at most 78 columns.
- **No new ADR.** This finishes position 9 in order.
- **Phase 2 stays incomplete** and the tag stays `v0.2.0-alpha.1`: `00-MASTER-SPEC.md` §7 makes
  `v0.{phase}.0` assert a phase is done, and positions 10 and 12, `examples/05-json-api` and
  `docs/benchmarks/` are all still absent.

---

## 10. Global constraints

- `cargo build --locked --workspace` **before** `cargo test`; `--no-fail-fast`; sum every
  `test result:` line across all 44 targets and **never** pipe cargo output through `head`/`tail`.
  Baseline **1057 passed / 0 failed / 8 ignored**.
- Clippy `-D warnings` on **both** ubuntu and windows; `cargo fmt --all -- --check`. **MSRV 1.78:
  no `reason = "..."` in any lint attribute.**
- The ignored GC tests stay ignored and untouched. Note the ignored count is **8 unconditional
  attributes** (six in `gc.rs`, two in `task.rs`) plus **one conditional** —
  `#[cfg_attr(target_os = "linux", ignore = ...)]` on `extern_ffi_run` — so the runtime count is 8
  on Windows and macOS and 9 on Linux, and CI's advisory `--ignored` step is red on Linux by design.
- The poll ABI is **frozen**. No panic may cross a generated poll boundary.
- `std/net/lib.nova` is `include_str!`'d, so editing it forces a full workspace rebuild.
- Commit messages to a UTF-8 file applied with `git commit -F`, **never a heredoc**; each body ends
  exactly `Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>`.
- **Cite no SHA that is not already an ancestor of `main`.** `3901db5` is.
- **Byte-scan every file written**, including this spec and the plan: no byte below 0x20 outside
  tab/CR/LF, no `0x7f`, valid UTF-8, and zero occurrences of a backslash-`u` escape followed by four
  hex digits in tracked markdown. Write code points as `U+XXXX`. The plan and the spec are the one
  class of file the review loop never scans, because every byte scan covers the files its own commit
  touched and these are written before the dispatch loop begins — which is how the only such byte
  ever to reach a commit got in.

---

## 11. Verification debt

Stated plainly so the plan can close it rather than inherit it.

- Nothing in the context map behind this design was compile-verified. "Unwritable" throughout means
  "rejected on a code path that was read", not "produced error E-nnnn". The plan's first task should
  compile the new signatures before depending on them.
- **`break` and `continue` do work** — fixture-pinned at `tests/runtime/break_continue.nova`,
  registered at `run_tests.rs:318`; only `break` *with a value* is unsupported, and `break` outside
  a loop is `E0080`. An earlier draft of this increment's framing asserted otherwise, generalising
  "no `loop` keyword" into a second claim that is false. An accept loop may use `break`.
- Whether Nova has **trait objects or dynamic dispatch** is unchecked. It does not affect this
  increment, but it decides whether `std/http`'s `Handler` can be a trait, so position 10's design
  must resolve it first.
- The `dead_addr()` port-reuse flake is unfixed at `3901db5` and is tracked separately.
