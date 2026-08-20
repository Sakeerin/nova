# ADR 0016 — `std/sync`'s `Mutex`: closing position 8, partially

**Numbering:** confirmed against `docs/adr/`'s actual contents rather than
trusted from the plan — `0001` through `0015` all exist with no gap, so
`0016` is next.

## Status

Accepted (2026-08-20). The `std/sync` `Mutex` increment, branch
`std-sync-mutex` (`docs/superpowers/specs/2026-08-20-std-sync-mutex-design.md`).

## Context

`00-MASTER-SPEC.md` §3 lists Phase 2's standard-library build order.
`nova-spec/20-STDLIB.md` §13 covers `std/sync` and `std/task` in one
section and specifies four things across the two: `Mutex<T>` (`new`,
`async fn lock`) and a `channel<T>` returning `(Sender<T>, Receiver<T>)`
under `std/sync` — §3's position 8 — and `spawn_blocking<T>` and
`JoinHandle::cancel` under `std/task` — position 7, otherwise already
shipped (`spawn`, `join`). This ADR's decision covers all four, since §13
specifies them together and this increment is what finally audited that
whole section against the tree.

Position 8 is reached only after positions 2 and 6 were each taken out of
§3's strict order, on two separate occasions recorded in
`docs/adr/0014-stdlib-build-order-deviations.md` and
`docs/adr/0015-std-fmt-scope.md`. ADR 0014 records position 6 (`std/log`,
alongside `std/time`) built ahead of position 2 — the second of two
deferrals of position 2, the first having deferred it entirely behind
async back in Phase 2.1. ADR 0015 records position 2 (`std/fmt`) finally
closed — itself out of the list's sequential order, since positions 3–5
(`std/collections`, `std/strings`, `std/fs`) and position 7 (`std/task`)
had already shipped by then. Position 8 has not been skipped before now;
ADR 0014's own Consequences section names it, alongside position 10, as
unbuilt but not yet passed over by name in any design doc. This increment
is the first to reach it at all.

**The framing under which this increment was nearly not started was
wrong, and it is worth recording precisely because the mistake is
available to anyone reading §13 quickly.** The argument against building
`std/sync` now was that the executor is single-threaded, so a `Mutex` has
almost nothing to contend with. §13's own specification refutes this:

```nova
pub async fn lock(self) -> MutexGuard<T>
```

is an **async** lock, not a thread lock. A thread mutex would indeed be
pointless on a single-threaded cooperative executor — there is no second
OS thread to race with. But a task on such an executor can be **suspended
in the middle of a critical section** at any `.await`, and while it is
suspended, another task runs. Any invariant that spans a suspension point
needs protecting from exactly that, which a thread mutex never addressed
and an async mutex does. "Single-threaded" makes a thread mutex meaningless
and an async mutex necessary — it does not make mutual exclusion
unnecessary, and the original framing stopped one inference short.

## Decision

**Ship `Mutex<T>` and `MutexGuard<T>`; defer `channel<T>`,
`spawn_blocking`, and `JoinHandle::cancel`, each for a measured, named
reason rather than as an oversight.**

What shipped, as part of `std/sync` (`STD_MODULES` 11 → 12, zero new
runtime intrinsics, `Builtin::STD_ONLY` unchanged at 65): `Mutex<T> {
locked: Bool, value: T }` with `new(value: T) -> Mutex<T>` and a private
`take`; `try_lock(mut self) -> Option<MutexGuard<T>>` for the
non-suspending case; `async fn lock(mut self) -> MutexGuard<T>`, which
retries by yielding rather than parking; and `MutexGuard<T> { owner:
Mutex<T> }` with `get(self) -> T` and an explicit, idempotent
`release(mut self)`. Release is explicit rather than RAII for one
measured reason: `Drop` is described in three spec files
(`12-TYPESYSTEM.md:192`, `13-RUNTIME.md:96`, `14-CODEGEN.md:24`) and
implemented in none of them — no handling in `nova-typeck`, `nova-resolver`
or `nova-mir`, and no `trait Drop` in `std/core`. A guard's release
(`self.owner.locked = false`) is pure Nova, so hooking it to scope exit
needs the *language* feature, and no runtime mechanism substitutes for it.
`docs/adr/0012-file-descriptor-lifecycle.md` is the precedent for what to
do in that situation, and **only** the precedent: it chose explicit,
idempotent `close` for `File` over a collector-based backstop, accepting a
uniform documented leak, and this module makes the identical choice for
the identical reason — not because ADR 0012 forecloses some other
mechanism for a `MutexGuard`. It does not: ADR 0012's own argument is that
the collector's per-object notification hook reports a freed object's
*own address*, never a field value read out of it, which tells an
`fd`-keyed handle table nothing — an argument specific to a
runtime-managed handle. A `MutexGuard`'s release never reaches the
collector at all, so that argument does not transfer here; only the shape
of the resulting trade-off does.

What did not ship, and why each is a deferral rather than an oversight:

- **`channel<T>(buffer: Int) -> (Sender<T>, Receiver<T>)`.** Its signature
  returns a tuple type, and Nova does not have tuples yet — measured,
  `error[E0900]: tuple types are not supported yet`. Of the three deferred
  items, this is the one this project actually wants: a bounded channel is
  broadly useful, and the blocker is that Nova's type system cannot yet
  express the signature §13 wrote, not any objection to the feature. It
  needs an API redesigned around what Nova can express — a record holding
  both ends, or two separate constructors — which is a design decision, not
  a transcription, and belongs to its own increment.
- **`spawn_blocking<T>(f: fn() -> T) -> JoinHandle<T>`.** Requires a thread
  pool to run `f` on. `crates/nova-runtime/src/task.rs`'s own first line
  describes the runtime as "a single-threaded cooperative executor," and
  the only `std::thread::spawn` anywhere in the runtime is a test helper in
  `poll.rs`, not infrastructure a std module could dispatch blocking work
  onto. Building this means introducing real OS threads to a runtime that
  currently has none — a decision considerably larger than a stdlib
  function, and not one this increment makes.
- **`JoinHandle::cancel(self)`.** Contradicts a contract this project has
  already settled twice: `docs/adr/0009-async-execution-model.md` and the
  `timeout<T>` increment both established **abandonment, not
  cancellation** — an abandoned future is simply never polled again,
  nothing unwinds, and the poll ABI has no interrupt hook. `cancel` as §13
  writes it would need exactly the mechanism that contract rejects, so
  shipping it now would either be a silent no-op or a reversal of a
  settled decision, and this increment does neither.

## Consequences

**Position 8 is partially closed — the buildable quarter of it, not all
of it — which is what distinguishes this ADR from both of its
predecessors.** ADR 0014's subject is position 2 skipped twice with
nothing shipped at that position either time; ADR 0015's subject is
position 2 fully closed. This ADR's position is neither: one of its four
specified items ships, three remain open with named blockers, none of
which this increment removes.

**The yield-and-retry trade is accepted explicitly, including its cost.**
`lock` retries by yielding (`while !self.take() { yield_now().await }`)
rather than by parking on a dedicated wait condition. That choice adds
**no executor surface at all**: no fourth `Wait` variant alongside the
existing `Deadline`/`Task`/`Io` (`crates/nova-runtime/src/task.rs:162`),
and no new arm in `wake_due`'s `retain` match, whose existing arms already
end in a wildcard rather than an exhaustive list over `Wait`'s variants —
the same non-exhaustive match this project has previously found to
conceal a reachable process abort for an entire increment. The cost is
diagnosability: a task waiting on a lock stays **runnable**, not parked,
so `report_deadlock` — which only ever inspects parked tasks — cannot see
it. A lock that is taken and never released does not produce a deadlock
report; it produces a busy spin through the run queue instead, forever.
Two tasks each holding one lock and awaiting the other spin rather than
being detected. **The condition that would justify revisiting this
trade**: contention frequent enough that the spinning itself costs
measurable time, or a real deadlock going undiagnosed in practice. At that
point the fourth `Wait` variant is the answer, with getting `wake_due`'s
match right as the thing to be careful of.

**Two further limits, stated rather than left implicit.** This `Mutex` is
**not re-entrant** — a task that calls `lock` while it already holds the
same mutex spins against itself forever, since `take` has no notion of
"already mine." And it makes **no fairness guarantee** — `yield_now`
re-queues the caller behind everything currently runnable, which
guarantees the current holder always gets a turn and can always release,
but says nothing about which of several waiters acquires the lock next.

**This increment's only fixture that actually depends on the mutex derives
its entire power from where one line sits, and that is a fragility worth
naming here rather than only in the fixture's own comment.**
`tests/runtime/sync_mutex_two_tasks_serialise.nova` spawns two tasks that
each read a shared counter, suspend at a `yield_now().await` placed
**inside** the locked critical section, then write back. Task 2 reaching
`lock()` and spinning while task 1 is still suspended mid-section was
confirmed by instrumented trace, not inferred, in this increment's review.
With the mutex present the tasks serialise and the result is `n=2`;
verified by mutation, removing the mutex (or moving that `yield_now` back
outside the critical section) still passes every other fixture in this
increment and produces `n=1` here — the lost update the mutex exists to
prevent. Three of this increment's four fixtures would pass unchanged with
`Mutex` deleted entirely; this is the one that would not, and only because
of that single line's placement. A future edit that moves or removes it
without noticing would silently turn this fixture into a shape test that
no longer proves mutual exclusion at all.

## References

- Design: `docs/superpowers/specs/2026-08-20-std-sync-mutex-design.md`
- `docs/adr/0014-stdlib-build-order-deviations.md`: position 2's two skips
- `docs/adr/0015-std-fmt-scope.md`: position 2 closed; this ADR's
  structural precedent
- `docs/adr/0009-async-execution-model.md`: the abandonment-not-cancellation
  contract `JoinHandle::cancel` would violate
- `docs/adr/0012-file-descriptor-lifecycle.md`: the explicit-release
  precedent this decision follows for the same reason, not the mechanism
  it forecloses
- `nova-spec/00-MASTER-SPEC.md` §3: the strict build order; §7, item 5:
  "ADR written for any decision deviating from this spec"
- `nova-spec/20-STDLIB.md` §13: `std/sync`'s specification and this
  increment's own 2026-08-20 amendment
- `std/sync/lib.nova`: the shipped module
- `crates/nova-runtime/src/task.rs:162`: `enum Wait`, three variants,
  unchanged by this increment
- `crates/nova-resolver/src/lib.rs:1267`: `STD_MODULES`, 12 entries after
  this increment; `:1293`: `STD_TEST_MODULE`, `std/test`'s separate constant
- `tests/runtime/sync_mutex_two_tasks_serialise.nova`: the one fixture this
  increment's `Mutex` cannot be deleted without failing
