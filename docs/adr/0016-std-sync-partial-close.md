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
Mutex<T>, released: Bool }` with `get(self) -> T`, `set(mut self, v: T)`
and an explicit, idempotent `release(mut self)`.

**`released` and `set` were added by this increment's fix wave
(2026-08-21), after review; the paragraph above records the shipped
shape, not the first-drafted one.** `release` was an unconditional
`self.owner.locked = false` on a guard that carried no released-state, so
a second `release` on a guard whose mutex had since been reacquired freed
a lock another task was holding — the documented idempotence was false in
exactly the case that matters. `released` supplies the state that claim
needs, and it is precisely the precondition the `File::close` precedent
below does **not** transfer: `File`'s second `close` is safe because an
`fd` is a key into a runtime table where absence *is* closedness, and a
guard has no table to be absent from. `set` was added because `get`
returns the value, so for a `T` with no assignable interior (`Int`,
`Bool`, `Float`, `Char`, `String`) the guard was a read-only view while
`Mutex<T>`'s generic signature promised otherwise; `std/collections`
pairs `Vec::get` (`std/collections/lib.nova:49`) with `Vec::set` (`:53`)
for the same reason. `set` is a no-op on a released guard for the same
reason `release` is — a stale `set` would write into the protected value
while another task was inside its critical section. Neither addition
touches the runtime: still zero new intrinsics, `STD_ONLY` still 65.

What neither flag can reach is **forgery**. Nova enforces no field
privacy, so any code holding a `Mutex<T>` can write
`MutexGuard { owner: m, released: false }` and `release` it without ever
acquiring the lock — measured, and it frees a lock a real guard is
holding. That is the same unenforceable shape ADR 0012 records for
`File { fd: 9999 }`, with one difference worth stating: a forged `fd`
safely misses a table lookup, whereas a forged guard writes straight to
the live mutex. Documented, not enforced, exactly as `get`-after-release
is.

Release is explicit rather than RAII for one
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
  function, and not one this increment makes. This argument was already
  made and settled: `docs/adr/0009-async-execution-model.md:107`, under
  its "What is given up" heading, records that **`spawn_blocking` cannot
  be honoured** and that it is not provided, rather than provided as a
  synonym for `spawn`. This increment restates that conclusion, it does
  not reach it.
- **`JoinHandle::cancel(self)`.** ADR 0009 leaves cancellation an **open
  residual gap, not a settled rejection**. It lists "No cancellation"
  among its residual gaps (`docs/adr/0009-async-execution-model.md:365`)
  and names "a future `JoinHandle` drop or cancellation" as the natural
  fix point for the task-leak footgun it documents (`:405`) — the opposite
  of a foreclosure. What *is* settled is narrower: the poll ABI has no
  interrupt hook, which is why the `timeout<T>` combinator abandons rather
  than cancels (`:328-329`). So `cancel` as §13 writes it needs a
  mechanism that does not exist yet, and building that hook is a decision
  this increment does not make. It is deferred for want of the primitive,
  not because ADR 0009 rules it out.

## Consequences

**Position 8 is partially closed — the buildable part of it, not all of
it — which is what distinguishes this ADR from both of its
predecessors.** ADR 0014's subject is position 2 skipped twice with
nothing shipped at that position either time; ADR 0015's subject is
position 2 fully closed. This ADR's position is neither. Of position 8's
own items (`00-MASTER-SPEC.md:238`: `Mutex`, `channel`, `atomic`), one
ships — `Mutex` — and two remain open: `channel`, blocked on tuple types
(above), and `atomic`, which this increment does not touch at all and
which §13 never specifies with a signature. Separately, of the two
`std/task`/position-7 items §13 still lists after `spawn` and `join`
shipped — `spawn_blocking` and `JoinHandle::cancel` — both also remain
open, for the reasons above. The two positions are kept apart here
deliberately: an earlier draft of this paragraph counted all four
deferrals against position 8, which contradicts this ADR's own Context
section.

**The yield-and-retry trade is accepted explicitly, including its cost.**
`lock` retries by yielding (`while !self.take() { yield_now().await }`)
rather than by parking on a dedicated wait condition. That choice adds
**no executor surface at all**: no fourth `Wait` variant alongside the
existing `Deadline`/`Task`/`Io` (`crates/nova-runtime/src/task.rs:162`),
and no new arm in `wake_due`'s `retain` match, whose existing arms already
end in a wildcard rather than an exhaustive list over `Wait`'s variants —
the same non-exhaustive match whose omitted arm this project has
previously found to park a task forever with no compile error, panic or
diagnostic. The cost is
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

**The two fixtures that turn on the mutex holding a value across a
suspension point each derive their entire power from where one line sits,
and that is a fragility worth naming here rather than only in the
fixtures' own comments.**
`tests/runtime/sync_mutex_two_tasks_serialise.nova` spawns two tasks that
each read a shared counter, suspend at a `yield_now().await` placed
**inside** the locked critical section, then write back. Task 2 reaching
`lock()` and spinning while task 1 is still suspended mid-section was
confirmed by instrumented trace, not inferred, in this increment's review.
With the mutex present the tasks serialise and the result is `n=2`;
verified by mutation, a never-excluding `take` produces `n=1` here — the
lost update the mutex exists to prevent. **Four** of this increment's six
fixtures detect that mutation, all four measured rather than predicted:
this one and `sync_mutex_int_set_serialises` (both `n=2` becomes `n=1`),
`sync_mutex_try_lock_fails_when_held` (`second=false` becomes
`second=true`), and `sync_mutex_stale_guard_cannot_steal`
(`still_held=true` becomes `still_held=false`). The other two —
`sync_mutex_uncontended` and `sync_mutex_release_is_idempotent` — pass
unchanged, because a critical section containing no suspension point is
already serialised by cooperative scheduling. This fixture and
`sync_mutex_int_set_serialises` are the two that detect a *lost update*
across a suspension point, and each only because of that single line's
placement. A future edit that moves or removes one without noticing would
silently turn that fixture into a shape test that no longer proves mutual
exclusion at all.

Two numbers here were wrong before this increment's fix wave and are
recorded so the correction is not silent. The count was "three of four
would pass with `Mutex` deleted"; measured, two of the then-four passed.
And the parenthetical claimed that *moving* the `yield_now` outside the
critical section also produces `n=1`; measured, it produces `n=2` — which
is the point the paragraph's closing warning depends on, so the
parenthetical inverted it.

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
- `tests/runtime/sync_mutex_two_tasks_serialise.nova` and
  `tests/runtime/sync_mutex_int_set_serialises.nova`: the two fixtures that
  detect a lost update across a suspension point.
  `sync_mutex_try_lock_fails_when_held` and
  `sync_mutex_stale_guard_cannot_steal` also detect a never-excluding
  `take`, without needing one — four detectors of six fixtures, measured
- `tests/runtime/sync_mutex_stale_guard_cannot_steal.nova`: the only
  fixture that fails if `MutexGuard`'s `released` flag is removed, which is
  why the defect it pins shipped in the first place
