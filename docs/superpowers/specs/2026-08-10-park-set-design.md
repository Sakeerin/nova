# Own the park set — Design

**Status:** approved 2026-08-10. Follow-up 4 from Phase 2.3a's whole-branch review — the last open
one, and the prerequisite ADR 0009 named for whatever adds the first primitive that can block.

**Base:** `main` at `b27c942` (async core, task identity and reserved type names all merged and
pushed; 830 tests, 8 deliberately ignored).

---

## 1. Why this, and why now

The executor has no park set. `poll_one` pushes every `POLL_PENDING` task straight back onto the
ready queue (`crates/nova-runtime/src/task.rs:363-366`), so a task waiting on something is re-polled
once per turn forever. ADR 0009 §"Residual gaps" states the consequence in one line: **a busy
re-poll loop cannot tell "not ready yet" from "never will be".**

Today that costs only CPU, because the only future that *originates* a suspension is
`poll_yield_once`, which is ready on its very next poll. Generated state machines return
`POLL_PENDING` too, but only by propagating an inner future's — so the innermost pend is always a
`yield_now`, and every pending task is genuinely runnable on the next turn. `std/fmt` + `std/io` is the increment that changes it: the **blocking
operations** are `async fn` — `Read::read`, `Read::read_to_end`, `Write::write` and `Write::flush` in
`nova-spec/20-STDLIB.md` §4 (`:158-164`), and all eleven functions in §5 `std/fs` (`:192-201`, `:210`;
the section's other two declarations are records) — and a read
on a pipe with nothing to read is exactly the case the loop cannot distinguish. The stream
constructors `stdin`/`stdout`/`stderr` (`:180-182`) are **not** async and do not need to be; what
blocks is the operations, not obtaining the stream. ADR 0009 asked for the diagnostic to exist
*before* the primitive that can deadlock, which is why this lands first.

**Corrected 2026-08-15 (branch `file-open-openoptions`, fix round 4) — §5 no longer holds eleven
functions, and its other declarations are no longer two records.** Measured by extracting the section
(`awk '/^## 5\. /,/^## 6\. /'`) at three revisions: at this spec's own commit `d8dd5c6`, §5 held 11
functions, all of them `async fn`, plus 2 records and 2 impl blocks. At `509834e`: 11 functions, 3
records, 2 impls. At `b21e4a9`: **14 functions, 3 records and 4 impl blocks** — increment 3c declared
`OpenOptions` in that section and gave it an `impl Default` and three named constructors, `reading()`,
`writing()` and `appending()`.

**The blocking-operations claim survives, and its figure is still eleven — but as a count of
`async fn`s, not of functions.** §5's eleven `async fn`s are the same eleven today as at `d8dd5c6`:
the ten filesystem calls plus `open`. The three added functions are `pub fn`, not `pub async fn`, and
they build a value rather than touching the filesystem, so they are neither blocking operations nor
counter-examples to "the blocking operations are `async fn`." Read the sentence above as **all eleven
`async fn`s in §5 `std/fs`**, not as a statement of how many declarations that section contains.

The line references throughout that sentence (`:158-164`, `:180-182`, `:192-201`, `:210`) were exact at
`d8dd5c6` and had **already** drifted before this branch began — at `7207a41`, this branch's merge base
with `main`, §5's ten filesystem calls sat at `:232-241` and `open` at `:250`. They are positions in a
living file, not stable citations; resolve them by name.

There is already a reachable hang of this shape: `JoinHandle::join` is
`while !task_is_done(self.fut) { yield_now().await }` (`std/task/lib.nova:73`), so joining a task
that never finishes spins at one poll per turn with no output and no diagnostic.

## 2. What is established, and what is not

Read from the tree at `b27c942`. **The right-hand column marks how each was established**, because
this project has repeatedly shipped a claim measured on one shape and stated for all of them.

| Claim | How established |
|---|---|
| `poll_one` re-queues unconditionally on `POLL_PENDING` (`task.rs:363-366`) | read |
| `task_ctx` is a parameter of the poll ABI, and the executor passes **null** at its own call site (`task.rs:362`) | read — **one call site only** |
| The only non-generated `PollFn` is `poll_yield_once`, pending exactly once (`task.rs:760-777`) | read |
| `release_internal` sets `taken` and drops the GC root but **leaves the `Task` in `TASKS`**, so a task can be `taken` without being `done` (`task.rs:428-445`) | read |
| `poll_one` releases its `TASKS` borrow before calling `poll` (`task.rs:351-362`) | read |
| `run_to_completion` exits only when the queue drains (`task.rs:538-544`) | read |

**To probe before T1 writes any code**, each because the design leans on it:

- A poll function that calls a `nova_rt_*` function which borrows `BY_STATE` — confirm no borrow is
  live at that point in practice, not only by reading `poll_one`.
- Whether a released-but-incomplete task is still polled to completion, so nothing parked on it is
  stranded. The reading above says yes; confirm it.
- `Instant`/`thread::sleep` behaviour on this Windows host with a sub-millisecond deadline, since
  the drive loop sleeps on the earliest deadline.
- **What generated `await` sites pass for `task_ctx`.** The executor passes null at its own call
  site, but a generated state machine polling an inner future is a second class of call site. If it
  synthesises a value rather than forwarding, "`task_ctx` is null everywhere" is false — grep
  `async_lower.rs` for the inner-poll call rather than inferring it from `poll_one`. Nothing in this
  design depends on the answer, since parking goes through `CURRENT`; the claim is worth getting
  right because §3.5 asserts the ABI is unchanged.

## 3. The change

A **park set**: the authority on who is waiting and why.

```rust
#[derive(Copy, Clone)]
enum Wait { Deadline(Instant), Task(i64) }

thread_local! {
    static PARKED: RefCell<Vec<(i64, Wait)>>,   // the park set
    static CURRENT: Cell<Option<i64>>,          // task being polled right now
    static PENDING_PARK: Cell<Option<Wait>>,    // staged during a poll, read after
}
```

`Task` gains **no** field. A task is parked exactly when it appears in `PARKED`; `QUEUE` and
`PARKED` are disjoint and every transition is a move between them. One structure records who waits
and why, so there is no second place to disagree with it — and it is keyed on the executor's own
**task id**, never on a state address, so it needs no sweep hook and no pruning (see §4).

### 3.1 Staged, then committed

Registration happens *during* a poll, because a Nova-level `sleep`/`join` calls a builtin from
inside the poll; the decision belongs *after* it, because only then is the status known.

- a park builtin writes `PENDING_PARK` for `CURRENT`;
- `poll_one` reads it after `poll` returns: staged **and** `POLL_PENDING` → move to `PARKED`;
  nothing staged → re-queue, as today.

`yield_now` stages nothing, so its behaviour is unchanged and the deliberate spin stays available.

Two staging edge cases, both diagnosed rather than absorbed:

- **Staged, but the poll returned `POLL_READY`** → discard the registration. Keeping it would leave
  a finished task in the park set, faking a deadlock for the rest of the process.
- **A second stage while one is already staged** → abort. It means an inner future's `POLL_PENDING`
  did not propagate, which is a compiler or std bug, not user error.

### 3.2 Two new futures

Both mirror `nova_rt_task_yield_future`'s established shape — a scanned `FUTURE_SIZE` fat pointer
over a scanned state object of at least `STATE_MIN_SIZE`, with the resume tag in `STATE_SLOT_TAG`.

- **`nova_rt_task_sleep_future(ms: i64)`** — poll 1: stage `Deadline(now + ms)`, return
  `POLL_PENDING`. Poll 2: write the unit output, return `POLL_READY`.
- **`nova_rt_task_join_future(future: *mut u8)`** — poll 1: target already done → `POLL_READY`
  immediately; otherwise stage `Task(id)` and return `POLL_PENDING`. Poll 2: `POLL_READY`. The
  target id is resolved through the existing `BY_STATE` path (`task_id_of`), so a future no task was
  spawned for aborts there exactly as it does today, rather than becoming a deadlock.

### 3.3 Waking, in two places

- **On completion**, in `poll_one`'s existing completion path: after marking a task done, move every
  `Wait::Task(that id)` entry from `PARKED` to `QUEUE`.
- **On deadline**, in the drive loop, which gains an outer turn:

```
loop {
    while let Some(id) = QUEUE.pop_front() {
        poll_one(id)
        if !PARKED.is_empty() { wake every entry due by now }   // no sleeping here
    }
    if PARKED.is_empty() { break }
    match earliest Deadline in PARKED {
        Some(t) => { sleep until t; wake every entry now due }
        None    => deadlock_report_and_abort()
    }
}
```

**Corrected 2026-08-10, during Task 2 — the first version of this loop woke deadlines only at the
drained-queue branch, and that is a starvation bug.** A task that re-queues itself every turn (any
`yield_now` loop, including the spin-based `join` this increment replaces) means `pop_front` always
returns `Some`, so the inner loop never exits, `earliest_deadline` is never consulted, and a
deadline-parked task is **never woken**. It survives this increment, because `yield_now` keeps
re-queueing deliberately: `spawn(forever_yielding())` alongside `sleep(10).await` would hang for
good. Waking what is already due is cheap and belongs on every turn; only *sleeping* requires an
empty ready queue. The `PARKED.is_empty()` guard keeps the common case at one check with no clock
read. Found by Task 2's own end-to-end fixture being the first thing to exercise the combination.

**The deadlock condition is unchanged by that correction** — still queue-empty *and* park-set-non-empty
*and* no `Wait::Deadline` remaining.

**The deadlock condition is exact rather than heuristic.** An empty queue means nothing is running.
If every remaining wait is a `Task(_)`, then each target is itself parked — a target that was done
would already have woken its waiters, and a target in the queue contradicts the queue being empty —
so no task can complete and none can wake. If any `Deadline` remains, the loop sleeps instead, so a
sleeping program is never reported as deadlocked.

### 3.4 `std/task` surface

`join` becomes a single `await` rather than a loop, because a wake can only mean completion:

```nova
pub async fn join(self) -> T {
    task_join_future(self.fut).await
    task_release(self.fut)
    task_output(self.fut)
}

pub async fn sleep(ms: Int) { task_sleep_future(ms).await }
```

`sleep` takes whole milliseconds as an `Int`; Nova has no duration type and inventing one is not
this increment's job.

### 3.5 The poll ABI does not change

Still exactly two statuses. `task_ctx` stays null and reserved: the parking call comes from Nova, so
threading `task_ctx` to it would mean generated code forwarding that parameter into builtins, for
information the runtime already has — the executor is single-threaded and polls one task at a time,
so a `CURRENT` thread-local answers "which task is parking" without touching codegen. All five
frozen ABI constraints hold.

**One obligation is new, and it does not inherit.** `poll_yield_once` justifies "must not unwind" by
having no fallible operation at all; the two new poll functions touch runtime state, so that
argument does not transfer to them. Therefore `Wait` is `Copy` and staging uses `Cell`, not
`RefCell` — a `RefCell` borrow panic inside a poll frame would unwind through generated code that
has no landing pads. `nova_rt_task_join_future` does need a `BY_STATE` borrow; that it is safe rests
on `poll_one` holding no `TASKS`/`BY_STATE` borrow across the call, which §2 lists as a thing to
probe rather than assume.

## 4. Non-goals, each deliberate

- **No real I/O.** `std/io` is the next increment and adds a second wake source, not a new mechanism.
- **No Nova-visible wakers**, no multi-threading, no cancellation, no join timeout.
- **The park set is not keyed on state addresses.** `BY_STATE` exists only because a Nova-visible
  handle names a heap address, and it needs pruning at the GC sweep to stay honest — a gap that cost
  a Critical on the task-identity branch. Task ids are the executor's private identity, so keying on
  them needs no sweep hook and inherits no address-recycling hazard.
- **`yield_now` keeps spinning.** It is the primitive that means "let others run", and re-queueing is
  the correct implementation of that.
- **A permanently-runnable task masks a deadlock among the others, and that is accepted.** The check
  is gated on the ready queue draining, so one task looping on `yield_now` suppresses the diagnostic
  even when two *other* tasks are in a mutual-join cycle. Detecting that needs a
  no-task-made-progress-in-N-turns heuristic, which is exactly what §3.3 rejects: it cannot
  distinguish a slow program from a stuck one. Recorded rather than fixed, and it belongs in ADR 0009
  alongside the existing footguns.

## 5. Diagnostics

```
nova: deadlock: 2 tasks are parked and none can wake
  task 1 is waiting for task 2 to finish
  task 2 is waiting for task 1 to finish
```

Then abort. One line per parked task, naming its wait reason; the park set already stores the reason
for mechanical purposes, so reporting it adds bookkeeping only for the message itself.

**This adds an abort emitter, and the existing inventory must be re-derived rather than trusted.**
ADR 0008 inventories the `nova: panic:` emitters and its recorded count was already found wrong
once — `abort_with` was a fourth where the ADR said three. So: `grep` for the emitters, count what
is there, and correct ADR 0008 from the grep. Do not copy a number from this spec or from the ADR.

## 6. Testing

**The hard part: "join parks" is otherwise unobservable.** A spinning join and a parking join
produce identical output *and* identical completion order, so an ordering assertion passes under
both. That is the exact trap this project keeps falling into, so it is called out here rather than
left to review. **Superseded 2026-08-10 — the counter below was never built and is not needed;** the
deadlock fixture discriminates on its own, since a spinning `join` can only hang and never report.
The original reasoning, kept for the record: the executor exposes a **poll counter for tests**, and the assertion is that joining
a task which sleeps burns a bounded number of polls rather than one per turn.

- Runtime unit tests: park-then-wake on a deadline; wake-on-completion; deadlock with **two**
  mutually-joining tasks; a task parked on a deadline while another task runs; discard-on-`READY`;
  double-stage abort; a task that joins **itself** (parks on its own id → deadlock).
- Nova end-to-end: two tasks sleeping different durations complete in **deadline order, not spawn
  order** — spawn order is what a broken implementation produces, so this discriminates. Plus a
  deadlock fixture asserting the diagnostic text and a non-zero exit, mirroring
  `tests/runtime/forged_join_handle.nova`.
- **The deadlock fixture must be a mutual-join cycle** — two tasks each joining the other — which is
  the shape §5's own example shows. **Corrected 2026-08-10 during Task 3: the plan had specified
  "join a task that loops forever with `yield_now`", which contradicted §5 and cannot work.** That
  shape is a **livelock, not a deadlock**: the spinner re-queues itself every turn, so the ready
  queue is never empty, so the check is never reached — and it should not be, because telling "never
  completes" from "has not completed yet" is the halting problem. Reporting it would require a
  heuristic, which this design rejects. The mutual-join cycle still discriminates spinning from
  parking: with a spinning `join` the same fixture hangs instead of reporting.
- **Assert ordering and completion, never elapsed duration.** A timing assertion is the one thing
  certain to flake in CI, and eight tests are already `#[ignore]`d for flakiness (ADR 0010).
- The suite stays green at 830 + the new tests, with those 8 still ignored and untouched.

Mutation targets, named here rather than left to review:

| Mutation | Must be killed by |
|---|---|
| Delete the discard-on-`READY` branch | a future that stages a park then completes |
| Let the deadlock arm fire while a `Deadline` remains | `two_not_yet_due_deadlines_drain_the_queue_then_wake_in_deadline_order` |
| Wake on the wrong `Wait` reason | wake-on-completion test |
| Report only the first parked task | the **two**-task deadlock test |
| Re-queue instead of parking when a park is staged | `a_staged_park_moves_the_task_out_of_the_ready_queue` |

**Corrected 2026-08-11, during the final whole-branch review's fix pass
(finding I3).** The deadline-arm row named "the deadline-plus-running-task
test" —
`a_self_requeuing_task_does_not_starve_a_sibling_parked_on_a_deadline`. That
test parks its sleeper on `Wait::Deadline(Instant::now())`, already due the
instant it stages, so `run_to_completion`'s per-poll wake check always
wakes it before the drained-queue branch this mutation targets is ever
reached. MEASURED: replacing that arm's `wake_due_deadlines(at)` with
`report_deadlock()` left every pre-existing `nova-runtime` test green. The
row now names the Rust-level test written to close that gap, which parks
on a deadline still in the future so the drained-queue branch's real sleep
is what has to run. The last row named a test-only poll counter that this
section's own opening paragraph records as superseded and never built; it
now names the existing test that already covers the same mutation without
one.

## 7. Risks

1. **Timing flakiness.** Mitigated by asserting order, not duration — but the drive loop does sleep
   on a real clock, so a deadline test is closer to the flaky GC tests than anything else here.
   Prefer deadlines far enough apart that ordering cannot invert under load.
2. **ADR 0009 must be amended, not rewritten.** Its "no parking and no waking" residual gap is what
   this closes. Amend in place with a dated note, as ADR 0005 was amended, so the original reasoning
   stays legible.
3. **`block_on`'s observable behaviour changes**: a program that used to spin now idles, and a
   deadlocked one now aborts quickly instead of hanging. That is an improvement, but it *is* a
   behaviour change and belongs in `CHANGELOG.md` under `### Changed`, not `### Added`.
4. **The staged-park protocol is a two-step invariant across a foreign-function boundary.** The
   failure mode if it is got wrong is a task in the park set that should not be, i.e. a false
   deadlock report — loud rather than silent, which is the right direction for a mistake to fall.

## 8. Definition of done

- A task that parks is removed from the ready queue, and one woken by deadline or by completion is
  returned to it — each pinned by a test.
- `join` parks instead of spinning, demonstrated by the deadlock fixture rather than by output.
  **Superseded 2026-08-10: the poll counter §6 proposed was dropped as unnecessary.** A mutual-join
  cycle reports a deadlock only if `join` parks — with a spinning `join` the ready queue never empties
  and the same fixture hangs instead, verified under an external timeout — so the fixture discriminates
  on its own and the counter would have added executor surface for a property already pinned.
- The queue draining with a non-empty park set and no deadline reports every parked task and its
  wait reason, then aborts; a fixture asserts the text and the exit status.
- `yield_now`'s behaviour is unchanged, pinned by an existing test still passing.
- The poll ABI is unchanged: two statuses, `task_ctx` still null.
- ADR 0009 amended in place; ADR 0008's emitter inventory corrected **from a fresh grep**.
- Suite green, clippy `-D warnings` and `cargo fmt --all --check` clean.
- **Before committing, run the quantifier sweep over everything written**: `grep` for `always`,
  `every`, `only`, `any`, `never`, `all`, `cannot` and, per hit, delete the quantifier or state the
  measurement behind it. On the previous branch this caught an overclaim that two rounds of careful
  reading had missed.
