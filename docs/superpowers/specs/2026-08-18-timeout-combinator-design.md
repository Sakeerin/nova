# `timeout<T>` and the staging widening — design

**Status:** approved 2026-08-18. Base: `main` == `origin/main` == `b1343aa`, 452 commits, 0 merge commits, 982 tests (8 deliberately ignored), clean tree, seven CI checks green.

**Goal.** Deliver the half of `nova-spec/20-STDLIB.md` §9 that `std/time` deliberately left out: `timeout<T>(d: Duration, fut: Future<T>) -> Result<T, TimeoutError>`. Answering the three blockers recorded in `docs/superpowers/specs/2026-08-17-std-time-design.md` §1 is the substance of this increment.

**Approach in one line.** Widen the executor so a deadline may accompany any wait and two deadlines merge to the earlier; make `poll_sleep` level-triggered so a task woken for another reason cannot fabricate a completion; then add one status-carrying combinator whose inner value never moves.

---

## 1. Scope

### In

- `try_stage`: two deadlines merge by `min`; `Wait::Task` gains `deadline: Option<Instant>`.
- `wake_due`, `earliest_deadline`, `deadlock_report`: handle a timed `Wait::Task`.
- `poll_sleep` becomes level-triggered and **tag-free**, storing a deadline rather than a duration.
- One new builtin, `task_timeout_future`, and one new hand-written `PollFn`, `poll_timeout`.
- `std/time`: `TimeoutError` and `timeout<T>`.

### Out, deliberately

- **A cancellation hook on the future ABI.** A second function pointer per future would let an abandoned `connect` release its socket. It is the correct long-term fix and it is not this increment: it touches the frozen poll ABI's neighbourhood, grows every `build_future` call site, and deserves its own design. §5 records the one leak it would close.
- **Unifying `net.rs::deadline_epoch`.** A second consumer of "encode a deadline as a scannable `i64`" now exists, which weakens the reason the `std/time` increment declined to unify — but `deadline_epoch` carries `read_timeout`'s shipped arithmetic and its own tests, and rewriting it inside an increment about `timeout` is unsanctioned scope. `task.rs` encodes against `crate::time::epoch()` instead, so both encoders are at least epoch-relative.
- **`select` / `race` / `join_all`.** Not in §9. YAGNI.

---

## 2. Why the widening's shape is forced

`PENDING_PARK` is **one slot per task poll, not per future**. `poll_one` polls the task's future and then takes the slot unconditionally. So a combinator and the future it polls stage into the *same* `Staged`, which is why `timeout(d, sleep(..))` collides today.

That is not a limitation to route around. It is the invariant `Wait::Io`'s own doc comment states: *"one task must have exactly one `PARKED` entry, or every wake path has to remember to remove two."*

`Wait::Io` already carries `deadline: Option<Instant>`, so a deadline riding **inside** another wait is the established pattern rather than a new idea. Extending it to `Wait::Task` keeps one `PARKED` entry per task and needs no new bookkeeping anywhere.

**The rule, stated once:**

> A deadline may accompany any wait. Two deadlines merge to the earlier. Every other pair still collides and aborts.

---

## 3. The executor change

### 3.1 `try_stage`

- `Wait::Deadline(at)`: replace the deadline-already-set collision with `next.deadline = Some(min(prev, at))`. Drop the `next.task.is_some()` rejection.
- `Wait::Io { deadline: Some(at), .. }`: the same `min` merge instead of a collision.
- `Wait::Task(id)`: drop the deadline rejection; keep the task-already-set and io-already-set collisions.
- Everything else unchanged. Task+Task and Io+Io still abort.

`try_stage` is pure and non-aborting on purpose, which is what makes §6.1's tests possible without going through `abort_with`'s `std::process::abort()`.

### 3.2 `Wait::Task` grows a deadline

```rust
Task { id: i64, deadline: Option<Instant> },
```

`staged_to_wait`'s `task` branch folds `staged.deadline` in, exactly as its `io` branch already does. Its comment that "`task` wins first only because `try_stage` never lets it" co-exist with a deadline stops being true and must be rewritten: `task` still wins first, but now *carrying* the deadline rather than excluding it.

### 3.3 The three consumers, two of which the compiler forces

| Site | Forced? | Change |
|---|---|---|
| `earliest_deadline` | **Yes** — exhaustive match, no wildcard | return the deadline instead of `None` |
| `deadlock_report` | **Yes** — exhaustive match | name the deadline in a timed task wait's line |
| `wake_due` | **NO** — `retain` ends in `_ => true` | must gain an explicit timed-task arm |

**`wake_due` is the hazard of this increment.** Omit its arm and a `Wait::Task` whose deadline elapsed falls through `_ => true`, stays parked forever, and produces **no compile error, no panic, and no diagnostic** — just a hang. §6.1 pins it with the only test that can fail when the arm is absent.

### 3.4 `poll_sleep` becomes level-triggered and tag-free

Under `min` merging, a task can be woken for a reason that is not its own, so every parker must re-check its own condition on re-poll. `poll_join` already does. `poll_sleep` does not: it keys off `STATE_SLOT_TAG` and its second poll returns `POLL_READY` unconditionally.

So `poll_sleep` re-checks `now >= deadline`, returning `POLL_READY` if so, and re-staging its deadline and returning `POLL_PENDING` if not. **The tag disappears**, which makes `poll_sleep` structurally identical to `poll_join` — already tag-free and level-triggered — so this is a convergence onto an existing shape rather than a new one.

The deadline is stored as **nanoseconds since `crate::time::epoch()`**, computed at construction: the builtin receives a *duration* in nanoseconds and stores `now_since_epoch + nanos`, so the slot holds a deadline for its whole life and never a duration. The same applies to `timeout`'s own deadline slot. `std::time::Instant` has no documented byte layout an `i64` slot can hold, which is the same problem `net.rs::deadline_epoch` solves; this uses the epoch `time.rs` and `poll.rs` already share.

**The slot is renamed: `SLEEP_SLOT_NANOS` → `SLEEP_SLOT_DEADLINE_NANOS`.** Its type is `i64` before and after, so changing what the integer *means* while keeping its name is invisible to the compiler. This is the second instance of that hazard in two increments — the `std/time` increment renamed the whole parker for the same reason — which suggests it is a property of the state-slot design rather than coincidence.

Consequence worth recording: `d` runs from construction rather than first poll. For `sleep(d).await` and `timeout(d, f).await` the two are indistinguishable; for a future built and held they are not.

---

## 4. The combinator

### 4.1 Runtime

```rust
#[no_mangle]
pub extern "C-unwind" fn nova_rt_task_timeout_future(nanos: i64, fut: *mut u8) -> *mut u8
```

Built through the existing `build_future`. State layout, with `STATE_MIN_SIZE` = 16:

| Slot | Holds |
|---|---|
| `STATE_SLOT_TAG` (0) | unused — this parker is level-triggered, like `poll_join` |
| `STATE_SLOT_OUTPUT` (1) | the status: `0` = inner completed, non-zero = elapsed |
| `TIMEOUT_SLOT_INNER` (= `STATE_SLOT_TEMPS`, 2) | the inner future's fat pointer |
| `TIMEOUT_SLOT_DEADLINE_NANOS` (3) | the deadline, nanoseconds since `crate::time::epoch()` |

`TIMEOUT_STATE_SIZE = STATE_MIN_SIZE + 16` = 32.

**GC needs no new mechanism.** `build_future` allocates state with `gc::alloc(state_size, true)` — scanned — so the stored inner fat pointer keeps the inner future and, transitively, its state reachable for exactly as long as the timeout future itself.

### 4.2 `poll_timeout`, and why it polls the inner first

```
read the inner fat pointer and the deadline
call the inner's poll(state, null)
  POLL_READY    -> write status 0 (completed); return POLL_READY
  POLL_PENDING  -> if now >= deadline: write status 1 (elapsed); return POLL_READY
                   else: stage Wait::Deadline(deadline); return POLL_PENDING
  anything else -> abort_with, naming the status
```

**Polling the inner before checking the deadline** means work that completed is never reported as timed out, and it makes `timeout(Duration::from_secs(0), ready_future)` return `Ok` — the least surprising answer. That order is only *available* because §3.4 made `poll_sleep` level-triggered; with edge triggering, deadline-first would have been forced as a defence against a woken sleep fabricating a completion.

**Abandonment needs no code.** When the inner returns `POLL_PENDING` and this poll then returns `POLL_READY`, the inner has already staged a park — and `poll_one` takes `PENDING_PARK` unconditionally, discarding it. Its own comment says so: *"Taken unconditionally, which is what discards a park staged by a poll that then returned `POLL_READY`."* The mechanism that stops a finished task faking a deadlock cleans up the abandoned park for free.

**No panic may cross this boundary.** Every slot access carries a SAFETY comment; the out-of-range inner status goes to `abort_with`, the same route a staging collision takes, never `panic!`.

### 4.3 Nova surface

```nova
pub record TimeoutError {}

pub async fn timeout<T>(d: Duration, fut: Future<T>) -> Result<T, TimeoutError> {
    if task_timeout_future(d.nanos, fut).await == TIMEOUT_COMPLETED {
        Ok(task_output(fut))
    } else {
        Err(TimeoutError {})
    }
}
```

Three facts this rests on, each measured rather than assumed:

- **A future value is reusable after being passed to a builtin.** `std/task`'s `block_on` is the precedent: `task_drive(fut)` then `task_output(fut)`.
- **`task_output(fut: Future<T>) -> T` already exists**, typed `(vec![future_of_param0()], Ty::Param(0))`, and reads "the value a finished future wrote to its state object's output slot". So the inner's value is read **from the inner's own slot** and never moves. **Correction: this was originally justified as avoiding "any interaction with the output slot's scalar/pointer non-discrimination" — that hazard never applied at this boundary in the first place.** `task_output`'s type parameter is rigid inside `timeout<T>`'s own body, and `unify` has no arm joining a rigid `Ty::Param` to a concrete type (measured via Task 4's `E0010`, §6.4 above) — so a call site that read the wrong future's slot, timeout's own `Future<Int>` instead of the inner's `Future<T>`, is rejected at compile time regardless of the output slot's representation. The reason `task_output(fut)` reading the inner's own slot is still the right design is simpler: it is the only slot that ever holds the value `T` names.
- **An empty record parses.** `record Marker {}` and `Marker {}` both type-check, so `TimeoutError` can carry nothing — which §9 specifies, and which the caller does not need since it passed `d` in. It is the first record in `std` needing no field-privacy disclosure, because it has no field.

`Result<T, E> = | Ok(T) | Err(E)` is a sum type in `std/core`.

### 4.4 Counts

`STD_ONLY` 59 → **60**. `STD_MODULES` stays **9**. `RESERVED_TYPE_NAMES` stays **7** — `TimeoutError` is a record.

---

## 5. Errors and edge cases

There is no status-code boundary here in the `std/fs` sense: `timeout` cannot fail for an OS reason. The status word is a two-valued discriminant, not an error kind.

- **Non-positive `d`.** The deadline is already past, but the inner is polled first, so a ready future returns `Ok` and a pending one returns `Err` on that same poll. No park, no spin, no special case.
- **Nested timeouts.** `timeout(a, timeout(b, f))` stages three deadlines that merge by `min`. Correct by construction.
- **`timeout` over `read_timeout`.** Two deadlines merge to the tighter, which is the right answer and needs no code.
- **The one leak.** `timeout` **abandons** the inner future; it does not cancel it. Measured against what each inner future owns mid-park: `sleep` owns only its state, which GC reclaims; `join` owns nothing and its target task runs on independently; `read` and `write` own nothing, since the caller still holds the `TcpStream` and can close it. **`connect` is the exception**: `start_connect` calls `register_new_socket` and only `finish_connect` removes or hands back that entry, so a timed-out `connect` leaves a socket nothing can reach or close. It leaks until process exit — which is exactly ADR 0012's existing stance for any unclosed descriptor, so this deviates from nothing. Documented at `timeout`'s doc comment **and** at `std/net`'s `connect`, because that is where someone wrapping a connect will be reading.
- **`d.nanos` overflow.** Impossible through the API: `Duration`'s constructors saturate at both ends. A hand-built `Duration` is covered by `std/time`'s existing disclosure that no record field in this language is privacy-enforced.

---

## 6. Testing

### 6.1 The staging widening

`try_stage` is pure and non-aborting, so test it directly:

- Two deadlines merge to the earlier **in both orders** — order-independence is the property, and a single-order test would pass against a `min` written backwards.
- Task+Deadline no longer collides, **in both orders**.
- Task+Task and Io+Io **still abort**, so the widening did not widen too far.
- `two_deadlines_in_one_poll_still_abort` asserts the behaviour this increment reverses. It becomes `two_deadlines_in_one_poll_merge_to_the_earlier`; the still-colliding pairs keep their abort tests. Leaving the old name over new behaviour would be a falsified claim of exactly the kind this project keeps finding.

**Then the test nothing else substitutes for:** `wake_due` must wake a `Wait::Task` whose deadline elapsed. Park one with a past deadline, call `wake_due(now)`, assert the id reached `QUEUE`. Because of `retain`'s `_ => true`, omitting the arm hangs silently — this test is the only thing that fails instead.

`earliest_deadline_and_wake_due_ignore_task_waits` splits: a **timed** `Wait::Task` contributes its deadline, a **bare** one still contributes nothing.

### 6.2 Level-triggered sleep

Poll once to stage, then poll again **before** the deadline: it must return `POLL_PENDING` and re-stage **the same deadline**, asserted by identity rather than existence — a mutant re-staging `now` satisfies "some deadline is staged" and then spins. A poll after the deadline returns `POLL_READY`.

### 6.3 The combinator

- **Inner-first ordering:** a zero-duration timeout over an immediately-ready inner reports *completed*, not elapsed. Reversing the order in `poll_timeout` fails this.
- **Elapsed:** a past deadline over a never-ready inner reports elapsed.
- **Pending:** a future deadline over a pending inner returns `POLL_PENDING`, and the staged park holds the inner's wait **and** the merged deadline — structural, via `staged_deadline_for_test` and `staged_io_for_test`.
- **GC:** the inner future survives a collection while only the timeout's state references it, following `a_completed_tasks_state_stays_rooted_until_its_output_is_taken`.

### 6.4 Nova fixtures

- `timeout` over a short sleep returns `Ok`; over a long sleep returns `Err`. Each prints **which branch ran**, so the golden distinguishes `Ok` from `Err` rather than pinning an ordering — an order-only assertion is scale-invariant and could not fail on a unit error.
- **The discriminating one:** `timeout(d, f)` where `f` produces `42` must print `42`. **Correction, made after Task 4 measured it: the mutation this bullet originally described — reading `task_output` on the timeout's own future instead of the inner's — does not compile.** `task_output`'s builtin signature is `(vec![Future<Param(0)>], Param(0))`; inside `timeout<T>`'s own body, `Param(0)` is `T` held rigid, checked once, and `unify` has no arm joining a rigid `Ty::Param` to a concrete type. The timeout combinator's own future is `Future<Int>` (`task_timeout_future`'s builtin return type is concrete, never `Future<Param(0))`), so binding it to `task_output`'s `Future<T>` parameter fails typeck with `E0010` before the mutant ever runs — this class of mistake is rejected at compile time for every instantiation of `T`, not merely caught at runtime for this fixture. This fixture's real discriminating power sits **below** the typechecker, at the runtime/lowering boundary that produced the concrete `Future<Int>` in the first place: a `nova-mir` lowering bug in `Builtin::TaskOutput`, a wrong runtime slot layout, or `task_timeout_future`'s Rust-side typeck signature mistakenly widened to return `Future<Param(0))` would each compile cleanly and still print the status `0` for this fixture instead of `42`. That is what this fixture actually pins.
- `timeout` over `handle.join()` in both directions — the Task+Deadline pair that aborts the process today.

### 6.5 Mutations to prove each mechanism is load-bearing

Delete `wake_due`'s new arm; swap `min` for `max` in `try_stage`; check the deadline before polling the inner; restore `poll_sleep`'s unconditional completion. Each must fail a named test above.

### 6.6 One caveat that does not apply this time

This increment touches `task.rs`, not `poll.rs`, and adds no `#[cfg(unix)]` code. Unlike the `std/time` increment — where CI's ubuntu legs caught a dead `timeval` initializer that eleven commits and four review passes had missed — local green means something here. The Unix-arm caveat is genuinely absent rather than merely unmentioned.

---

## 7. Records to update

- **`CHANGELOG.md`** `[Unreleased]`: `Added` for `timeout<T>` and `TimeoutError`; `Changed` for the staging rule and `poll_sleep`'s level-triggering, both internal but both semantics changes worth naming.
- **`nova-spec/20-STDLIB.md` §9**: §9 is now fully implemented; remove the not-yet-delivered note the `std/time` increment added, and record the abandonment contract.
- **`docs/adr/0009-async-execution-model.md`**: a dated amendment. The park set's staging rule changes, `Wait` grows a field, and `sleep` stops being edge-triggered — all of which ADR 0009 describes today.
- **`docs/superpowers/specs/2026-08-17-std-time-design.md` §1**: its "Out, deliberately" entry for `timeout<T>` should point at this spec rather than reading as an open question.
- **No new ADR.** Nothing here deviates from `nova-spec`; §9 is implemented as written. Should the implementation force a deviation, that is the signal to write one.
