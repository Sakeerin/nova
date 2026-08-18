# `timeout<T>` Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Deliver `timeout<T>(d: Duration, fut: Future<T>) -> Result<T, TimeoutError>` — the half of `nova-spec/20-STDLIB.md` §9 that `std/time` deliberately left out — together with the executor widening it requires.

**Architecture:** Widen the park set so a deadline may accompany any wait and two deadlines merge to the earlier; make `poll_sleep` level-triggered so a task woken for another reason cannot fabricate a completion; then add one status-carrying combinator whose inner value never moves, read from the inner future's own output slot by the existing `task_output`.

**Tech Stack:** Rust (nova-runtime, nova-resolver, nova-typeck, nova-mir), Nova (`std/time/lib.nova`), fixtures under `tests/runtime/`.

**Spec:** `docs/superpowers/specs/2026-08-18-timeout-combinator-design.md` (committed at `58a01a1`). Read it alongside this plan; every decision here is argued there.

## Global Constraints

- Run `cargo build --locked --workspace` **before** `cargo test`. Skipping it costs ~60 failures citing a missing `libnova_runtime.a` that look like real breakage and are not.
- `cargo test --locked --workspace --all-features --no-fail-fast`. `--no-fail-fast` is mandatory.
- **Never pipe cargo output through `head`/`tail` before summing.** Sum every `test result:` line — 44 targets. Baseline entering this plan: **982 passed, 0 failed, 8 ignored**.
- Pass `--locked` everywhere it is accepted.
- **No `reason = "…"` in any lint attribute** — MSRV is 1.78 and that syntax is newer; the MSRV leg runs `-D warnings`.
- `cargo clippy --locked --all-targets --all-features -- -D warnings` must pass on **both** ubuntu and windows.
- The **8 `#[ignore]`d ADR-0010 GC tests stay ignored and untouched.**
- The **poll ABI is frozen**: `PollFn = unsafe extern "C-unwind" fn(state: *mut u8, task_ctx: *mut u8) -> i64`, `POLL_PENDING = 0`, `POLL_READY = 1`, `task_ctx` always null.
- **No panic may cross a generated poll boundary.** `poll_sleep` and `poll_timeout` are hand-written `PollFn`s: no `unwrap`, `expect`, indexing, `panic!`, and **no `Instant + Duration`** (it panics on overflow — use `checked_add`). An out-of-range status from an inner poll goes to `abort_with`.
- Every fixture path unique per process.
- Commit messages go in a **UTF-8 file** applied with `git commit -F` — **never a heredoc**. Every body ends with exactly:
  `Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>`
- **Cite no SHA that is not already an ancestor of `main`.** `b1343aa` and `58a01a1` are. Nothing created on the work branch is — `gh pr merge --rebase` rewrites every branch SHA.
- `crates/nova-runtime/src/*.rs` are **CRLF** in the worktree, and `cargo fmt` flattened two of them to LF during the last increment. Check after any fmt run: `tr -cd '\r' < file | wc -c` must equal `wc -l < file`. To undo an edit use a raw byte copy verified by raw SHA-256 — **never** `git checkout --` or `git cat-file -p HEAD:<path> > <path>`.
- **Locate every edit site by content, not by the line numbers cited here.** They were correct at `58a01a1`; each task shifts the ones below it.
- **This plan touches `task.rs`, not `poll.rs`**, and adds no `#[cfg(unix)]` code — so unlike the last increment, local green is meaningful.

---

## File Structure

**Modified**

| File | Change |
|---|---|
| `crates/nova-runtime/src/task.rs` | `Wait::Task` grows a deadline; `try_stage` merges deadlines; `staged_to_wait`, `earliest_deadline`, `wake_due`, `deadlock_report` handle it; `poll_sleep` becomes level-triggered and tag-free; `poll_timeout` and `nova_rt_task_timeout_future` added |
| `crates/nova-runtime/src/time.rs` | extract `pub(crate) fn now_nanos() -> i64` so the deadline encoding has one source |
| `crates/nova-runtime/src/lib.rs` | register `nova_rt_task_timeout_future` in `symbols()` |
| `crates/nova-resolver/src/lib.rs` | `Builtin::TaskTimeoutFuture`; `STD_ONLY` 59→60 |
| `crates/nova-typeck/src/check.rs` | the builtin's signature and description |
| `crates/nova-mir/src/lib.rs`, `src/lower.rs` | `RtFunc::TaskTimeoutFuture`, its symbol and signature, the lowering map |
| `std/time/lib.nova` | `TimeoutError`, `timeout<T>`, `TIMEOUT_COMPLETED` |
| `crates/nova-cli/tests/run_tests.rs` | one `#[test]` per new fixture |
| `CHANGELOG.md`, `nova-spec/20-STDLIB.md`, `docs/adr/0009-async-execution-model.md`, `docs/superpowers/specs/2026-08-17-std-time-design.md` | the records |

**Created**

`tests/runtime/timeout_ok.nova`, `timeout_elapsed.nova`, `timeout_value.nova`, `timeout_join_ok.nova`, `timeout_join_elapsed.nova`, each with a `.stdout` golden.

**Task order is forced.** Task 1's widening is what makes a task wake for a reason that is not its own, so Task 2's level-triggering answers it. Task 3's correctness argument depends on Task 2. Task 4's Nova surface needs Task 3's symbol. Task 5 records what the first four did.

**Expected test totals:** 982 → 984 (T1) → 986 (T2) → 990 (T3) → 995 (T4) → 995 (T5). Report actuals; a mismatch is a signal, not a formality.

---

## Task 1: Widen the staging rule

**Files:**
- Modify: `crates/nova-runtime/src/task.rs` — `enum Wait` (~162), `staged_to_wait` (~469), `try_stage` (~508), `earliest_deadline` (~977), `wake_due` (~1079), `deadlock_report` (~1120), `poll_join`'s `stage_park` call (~1556), and the tests at ~2738, ~2753, ~2984

**Interfaces:**
- Consumes: nothing from earlier tasks.
- Produces: `Wait::Task { id: i64, deadline: Option<Instant> }`; `try_stage` merging two deadlines by `min`; `wake_due` waking a timed task wait.

**The hazard in this task, stated before the steps:** `earliest_deadline` and `deadlock_report` match `Wait` exhaustively with no wildcard, so the compiler *forces* you to handle the new field. **`wake_due` does not** — its `retain` ends in `_ => true`, so if you omit its arm, a `Wait::Task` whose deadline elapsed falls through, stays parked forever, and produces **no compile error, no panic and no diagnostic**. Step 1's third test is the only thing that fails instead.

- [ ] **Step 1: Write the failing tests**

Add to `task.rs`'s test module. Note `try_stage` is pure and non-aborting, which is what lets these run without going through `abort_with`'s `std::process::abort()`:

```rust
    /// Two deadlines in one poll merge to the earlier, **in either order**.
    ///
    /// Both orders on purpose: order-independence is the property, and a
    /// single-order test passes against a `min` written backwards.
    #[test]
    fn two_deadlines_in_one_poll_merge_to_the_earlier() {
        let base = Instant::now();
        let soon = base + Duration::from_secs(1);
        let later = base + Duration::from_secs(30);

        let staged = try_stage(Staged::default(), Wait::Deadline(later))
            .expect("first deadline stages");
        let staged = try_stage(staged, Wait::Deadline(soon))
            .expect("a second deadline must merge, not collide");
        assert_eq!(staged.deadline, Some(soon), "later-then-sooner must keep the earlier");

        let staged = try_stage(Staged::default(), Wait::Deadline(soon))
            .expect("first deadline stages");
        let staged = try_stage(staged, Wait::Deadline(later))
            .expect("a second deadline must merge, not collide");
        assert_eq!(staged.deadline, Some(soon), "sooner-then-later must keep the earlier");
    }

    /// A task wait and a deadline co-stage, in either order -- this is the
    /// pair `timeout(d, handle.join())` needs and that aborted the process
    /// before this increment.
    #[test]
    fn a_task_wait_and_a_deadline_co_stage_in_either_order() {
        let at = Instant::now() + Duration::from_secs(5);

        let staged = try_stage(Staged::default(), Wait::Task { id: 7, deadline: None })
            .expect("task stages");
        let staged = try_stage(staged, Wait::Deadline(at))
            .expect("a deadline must join a task wait, not collide");
        assert_eq!((staged.task, staged.deadline), (Some(7), Some(at)));

        let staged = try_stage(Staged::default(), Wait::Deadline(at))
            .expect("deadline stages");
        let staged = try_stage(staged, Wait::Task { id: 7, deadline: None })
            .expect("a task wait must join a deadline, not collide");
        assert_eq!((staged.task, staged.deadline), (Some(7), Some(at)));
    }

    /// `wake_due` must wake a task wait whose deadline elapsed.
    ///
    /// **This is the only test that fails if `wake_due`'s timed-task arm is
    /// missing.** Its `retain` ends in `_ => true`, so an unhandled timed
    /// `Wait::Task` is not a compile error, not a panic and not a diagnostic
    /// -- the task simply stays parked for the rest of the process.
    #[test]
    fn wake_due_wakes_a_task_wait_whose_deadline_elapsed() {
        let past = Instant::now();
        PARKED.with(|p| {
            p.borrow_mut()
                .push((41, Wait::Task { id: 999, deadline: Some(past) }));
        });
        wake_due(past + Duration::from_millis(1));
        let queued = QUEUE.with(|q| q.borrow().contains(&41));
        assert!(queued, "a timed task wait whose deadline passed must be re-queued");
        assert!(
            PARKED.with(|p| p.borrow().is_empty()),
            "and it must be removed from PARKED, not woken twice"
        );
    }

    /// A **timed** task wait contributes its deadline; a **bare** one still
    /// contributes nothing. This is the split of the old
    /// `earliest_deadline_and_wake_due_ignore_task_waits`, whose name asserted
    /// the half that this increment reverses.
    #[test]
    fn earliest_deadline_counts_a_timed_task_wait_and_ignores_a_bare_one() {
        let base = Instant::now();
        let soon = base + Duration::from_secs(1);
        let later = base + Duration::from_secs(30);
        PARKED.with(|p| {
            let mut p = p.borrow_mut();
            p.push((1, Wait::Task { id: 999, deadline: None }));
            p.push((2, Wait::Task { id: 998, deadline: Some(later) }));
            p.push((3, Wait::Deadline(soon)));
        });
        assert_eq!(earliest_deadline(), Some(soon));

        PARKED.with(|p| p.borrow_mut().clear());
        PARKED.with(|p| {
            let mut p = p.borrow_mut();
            p.push((1, Wait::Task { id: 999, deadline: None }));
            p.push((2, Wait::Task { id: 998, deadline: Some(later) }));
        });
        assert_eq!(
            earliest_deadline(),
            Some(later),
            "a timed task wait is the only deadline here and must be found"
        );
    }
```

These tests share `PARKED`/`QUEUE` thread-locals with their neighbours. Follow whatever isolation the existing `wake_due`/`earliest_deadline` tests already use — clear the thread-locals at the start and end rather than inventing a new mechanism.

- [ ] **Step 2: Run them and confirm they fail**

Run: `cargo test --locked -p nova-runtime task::`
Expected: FAIL — `Wait::Task` does not take named fields, so these do not compile.

- [ ] **Step 3: Grow `Wait::Task`**

Change the variant to carry a deadline, mirroring `Wait::Io`'s existing shape:

```rust
    /// Wake once the task with this id completes, or once `deadline` passes.
    ///
    /// The deadline rides inside this variant for the same reason it rides
    /// inside [`Wait::Io`]: one task must have exactly one `PARKED` entry, or
    /// every wake path has to remember to remove two.
    Task { id: i64, deadline: Option<Instant> },
```

Update `poll_join`'s call to `stage_park(Wait::Task { id: target, deadline: None })`. The compiler will point at every other construction and match site except `wake_due`'s.

- [ ] **Step 4: Merge deadlines in `try_stage`**

In the `Wait::Deadline(at)` arm, replace the deadline-already-set collision and the task-already-set rejection with a merge:

```rust
        Wait::Deadline(at) => {
            next.deadline = Some(match next.deadline {
                Some(prev) => prev.min(at),
                None => at,
            });
        }
```

In the `Wait::Io { deadline: Some(at), .. }` inner branch, replace its collision with the same `prev.min(at)` merge. In the `Wait::Task` arm, drop the deadline rejection and keep the task-already-set and io-already-set collisions, folding any incoming deadline in with the same merge.

Rewrite `try_stage`'s doc comment: it currently explains that a second deadline collides, which becomes false here.

- [ ] **Step 5: Fold the deadline in `staged_to_wait`**

```rust
    if let Some(id) = staged.task {
        return Some(Wait::Task { id, deadline: staged.deadline });
    }
```

Its doc comment says `task` "wins first only because [`try_stage`] never lets it" co-exist with a deadline. That is now false — rewrite it to say `task` still wins first but now *carries* the deadline, exactly as the `io` branch does.

- [ ] **Step 6: Handle the timed task wait in all three consumers**

`earliest_deadline` (compiler-forced): `Wait::Task { deadline, .. } => deadline`.

`deadlock_report` (compiler-forced): its `Wait::Task(target)` arm becomes `Wait::Task { id, deadline }` and names the deadline when present, following how its `Wait::Io { deadline: Some(..), .. }` arm already words the timed case.

**`wake_due` (NOT compiler-forced — this is the one to get right):** add an explicit arm before the `_ => true` catch-all:

```rust
            Wait::Task {
                deadline: Some(deadline),
                ..
            } if deadline <= now => {
                woken.push(id);
                false
            }
```

- [ ] **Step 7: Retire the two test names that assert the old behaviour**

Delete `two_deadlines_in_one_poll_still_abort` (~2738) — Step 1's `..._merge_to_the_earlier` replaces it. Delete `earliest_deadline_and_wake_due_ignore_task_waits` (~2984) — Step 1's split replaces it. **Keep `two_io_waits_in_one_poll_still_abort` (~2753) exactly as it is**: Io+Io still aborts, and that test is now load-bearing evidence the widening did not widen too far.

- [ ] **Step 8: Verify**

Run: `cargo build --locked --workspace` then `cargo test --locked --workspace --all-features --no-fail-fast`
Expected: **984 passed, 0 failed, 8 ignored** across 44 targets — 982 plus the four new tests from Step 1, minus the two deleted in Step 7. Note the fourth new test *is* the replacement for the deleted split, so it is counted once, not twice. Sum every `test result:` line.
Then `cargo clippy --locked --all-targets --all-features -- -D warnings` and `cargo fmt --all -- --check`, both clean. Re-check `task.rs` is still fully CRLF.

- [ ] **Step 9: Prove `wake_due`'s arm is load-bearing**

Delete the arm you added in Step 6, rebuild, and confirm `wake_due_wakes_a_task_wait_whose_deadline_elapsed` **fails** while everything else still passes — that contrast is the whole reason the test exists, because nothing else in the toolchain would notice. Restore with a raw byte copy verified by raw SHA-256, **not** `git checkout --`, and confirm the suite is green again.

- [ ] **Step 10: Commit**

```bash
git add crates/nova-runtime/src/task.rs
git commit -F <path-to-utf8-message-file>
```

Subject: `feat(runtime)!: let a deadline accompany any wait, merging by min`. The body must record that `wake_due` is not compiler-forced, the mutation result from Step 9, and why the two test names had to change with the behaviour.

---

## Task 2: Level-triggered, tag-free sleep

**Files:**
- Modify: `crates/nova-runtime/src/time.rs` — extract `now_nanos`
- Modify: `crates/nova-runtime/src/task.rs` — `poll_sleep` (~1477), `deadline_from_nanos` (~1501), `SLEEP_SLOT_NANOS` (~1509), `SLEEP_STATE_SIZE` (~1513), `nova_rt_task_sleep_future_nanos` (~1524)

**Interfaces:**
- Consumes: Task 1's merged staging — a task can now wake for a reason that is not its own, which is *why* this task exists.
- Produces: `pub(crate) fn now_nanos() -> i64` in `time.rs`; `SLEEP_SLOT_DEADLINE_NANOS`; a `poll_sleep` that re-checks its own deadline and needs no tag.

**Why this task is not optional:** under Task 1's merge, `poll_sleep`'s second poll can happen because *someone else's* deadline fired. Today it returns `POLL_READY` unconditionally at that point, reporting a completion it has not earned.

- [ ] **Step 1: Write the failing tests**

```rust
    /// A sleep polled before its deadline must re-stage **the same deadline**
    /// and stay pending.
    ///
    /// Asserted by identity, not existence: a mutant that re-stages
    /// `Instant::now()` satisfies "some deadline is staged" and then spins
    /// forever, which an existence check cannot distinguish from correct.
    #[test]
    fn a_sleep_polled_early_re_stages_the_same_deadline() {
        let fut = nova_rt_task_sleep_future_nanos(60 * 1_000_000_000);
        let poll_addr = unsafe { (fut as *mut usize).add(FUTURE_SLOT_POLL).read() };
        // SAFETY: word 0 is a `PollFn` bit pattern by `fut`'s own construction.
        let poll: PollFn = unsafe { std::mem::transmute(poll_addr) };
        let state = state_of(fut) as *mut u8;

        let previous = current_task();
        set_current_for_test(Some(0));
        // SAFETY: `poll`/`state` are the pair inside `fut`; `task_ctx` is null.
        let first = unsafe { poll(state, std::ptr::null_mut()) };
        let staged_first = staged_deadline_for_test();
        PENDING_PARK.with(|cell| {
            cell.take();
        });
        // SAFETY: same pair, polled a second time before the deadline.
        let second = unsafe { poll(state, std::ptr::null_mut()) };
        let staged_second = staged_deadline_for_test();
        PENDING_PARK.with(|cell| {
            cell.take();
        });
        set_current_for_test(previous);

        assert_eq!(first, POLL_PENDING, "a 60s sleep must park on its first poll");
        assert_eq!(
            second, POLL_PENDING,
            "and must stay pending when polled again before its deadline"
        );
        assert_eq!(
            staged_second, staged_first,
            "the re-staged deadline must be the original, not a fresh one"
        );
    }

    /// A sleep whose deadline has passed completes.
    #[test]
    fn a_sleep_polled_after_its_deadline_completes() {
        let fut = nova_rt_task_sleep_future_nanos(0);
        let poll_addr = unsafe { (fut as *mut usize).add(FUTURE_SLOT_POLL).read() };
        // SAFETY: as above.
        let poll: PollFn = unsafe { std::mem::transmute(poll_addr) };
        let state = state_of(fut) as *mut u8;

        let previous = current_task();
        set_current_for_test(Some(0));
        // SAFETY: as above.
        let status = unsafe { poll(state, std::ptr::null_mut()) };
        PENDING_PARK.with(|cell| {
            cell.take();
        });
        set_current_for_test(previous);

        assert_eq!(status, POLL_READY, "a zero-nanosecond sleep is already due");
    }
```

Drain `PENDING_PARK` after reading it, for the reason the last increment's structural test documents: a leftover staged park aborts the process when a later test stages a second deadline under `--test-threads=1`.

- [ ] **Step 2: Run and confirm the first test fails**

Run: `cargo test --locked -p nova-runtime a_sleep_polled`
Expected: `a_sleep_polled_early_re_stages_the_same_deadline` FAILS — the current `poll_sleep` returns `POLL_READY` on its second poll, so `second` is `POLL_READY` rather than `POLL_PENDING`. That failure **is** the bug this task fixes.

- [ ] **Step 3: Extract `now_nanos` in `time.rs`**

`nova_rt_time_now_nanos` currently computes the reading inline. Split it so Rust callers share one encoder:

```rust
/// Nanoseconds since [`epoch`], saturating at `i64::MAX`.
///
/// The one place a clock reading becomes an `i64`. `task.rs` encodes deadlines
/// against this so a deadline fits an `i64` state slot, which a
/// `std::time::Instant` cannot: it has no documented byte layout.
pub(crate) fn now_nanos() -> i64 {
    i64::try_from(epoch().elapsed().as_nanos()).unwrap_or(i64::MAX)
}

#[no_mangle]
pub extern "C-unwind" fn nova_rt_time_now_nanos() -> i64 {
    now_nanos()
}
```

- [ ] **Step 4: Store a deadline instead of a duration**

Rename the slot and rewrite the construction path in `task.rs`:

```rust
/// Where `nova_rt_task_sleep_future_nanos` stores its **deadline**, as
/// nanoseconds since `crate::time::epoch()`.
///
/// Renamed from `SLEEP_SLOT_NANOS`, which held a *duration*. The slot is an
/// `i64` either way, so changing what the integer means while keeping its
/// name would be invisible to the compiler -- the same hazard that made the
/// previous increment rename this parker rather than only retype it.
const SLEEP_SLOT_DEADLINE_NANOS: usize = STATE_SLOT_TEMPS;

/// State size for a sleep future: the ABI minimum plus the one temp slot
/// holding the deadline.
const SLEEP_STATE_SIZE: usize = STATE_MIN_SIZE + 8;

const _: () = assert!(SLEEP_STATE_SIZE >= (SLEEP_SLOT_DEADLINE_NANOS + 1) * 8);
```

Replace `deadline_from_nanos` with two helpers, and note the panic this fixes:

```rust
/// A deadline `nanos` nanoseconds from now, as nanoseconds since
/// `crate::time::epoch()`, clamping a non-positive argument to "now".
///
/// Nova's `Int` is signed and nothing stops `sleep(-1)`; treating it as an
/// immediate wake keeps the executor's invariants intact without inventing a
/// new failure mode for an argument that is merely useless. Saturating rather
/// than wrapping, for the same reason the reading itself saturates.
fn deadline_nanos_from_now(nanos: i64) -> i64 {
    crate::time::now_nanos().saturating_add(nanos.max(0))
}

/// A deadline in epoch-nanoseconds as an `Instant`, for staging.
///
/// **`checked_add`, not `+`.** `Instant + Duration` panics on overflow, and
/// this is reached from [`poll_sleep`], a hand-written `PollFn` across which
/// no panic may pass. The previous `Instant::now() + Duration::from_nanos(..)`
/// carried that panic on a path nothing had ruled out.
fn instant_from_deadline_nanos(deadline: i64) -> Instant {
    let ns = u64::try_from(deadline).unwrap_or(0);
    crate::time::epoch()
        .checked_add(Duration::from_nanos(ns))
        .unwrap_or_else(Instant::now)
}
```

And in the constructor:

```rust
    build_future(poll, SLEEP_STATE_SIZE, |slots| {
        // SAFETY: `slots` addresses a live `SLEEP_STATE_SIZE` block, and
        // `SLEEP_SLOT_DEADLINE_NANOS` is in bounds by the assertion above.
        unsafe { slots.add(SLEEP_SLOT_DEADLINE_NANOS).write(deadline_nanos_from_now(nanos)) };
    })
```

- [ ] **Step 5: Make `poll_sleep` level-triggered and tag-free**

```rust
/// Report [`POLL_READY`] once the stored deadline has passed, and
/// [`POLL_PENDING`] with the deadline re-staged until then.
///
/// **Level-triggered and tag-free**, which makes this structurally identical
/// to [`poll_join`]. It has to be: since a deadline may accompany any wait and
/// two deadlines merge to the earlier, this future can be polled because
/// *another* wait's deadline fired. An edge-triggered version -- returning
/// ready on its second poll regardless of the clock -- would report a
/// completion it had not earned.
///
/// **Must not unwind**, for the reason [`poll_yield_once`] states. Every
/// helper it calls is panic-free by construction; see
/// [`instant_from_deadline_nanos`] on why the old `Instant + Duration` was not.
unsafe extern "C-unwind" fn poll_sleep(state: *mut u8, _task_ctx: *mut u8) -> i64 {
    let slots = state as *mut i64;
    // SAFETY: `state` is the state object `nova_rt_task_sleep_future_nanos`
    // built, of at least `SLEEP_STATE_SIZE` bytes, so both slots are in bounds.
    let deadline = unsafe { slots.add(SLEEP_SLOT_DEADLINE_NANOS).read() };
    if crate::time::now_nanos() >= deadline {
        // SAFETY: same object, output slot.
        unsafe { slots.add(STATE_SLOT_OUTPUT).write(0) };
        return POLL_READY;
    }
    stage_park(Wait::Deadline(instant_from_deadline_nanos(deadline)));
    POLL_PENDING
}
```

The old SAFETY comment reads *"the duration (in nanoseconds) was stored here at construction — the deadline itself is computed below, now"*. That sentence was corrected in the previous increment and this task falsifies it again; the block above replaces it.

- [ ] **Step 6: Verify**

Run: `cargo build --locked --workspace` then `cargo test --locked --workspace --all-features --no-fail-fast`
Expected: **986 passed, 0 failed, 8 ignored**. Note `the_sleep_futures_layout_is_the_one_the_abi_declares` still passes — the layout is unchanged, only the slot's meaning and name. If it fails, the state size moved and something is wrong.
Then clippy `-D warnings`, `cargo fmt --all -- --check`, and a CRLF re-check on both touched files.

Also run the JIT path, since `sleep`'s behaviour changed under real driving:

```bash
cargo run --quiet -p nova-cli -- run tests/runtime/task_sleep_order.nova
```

Expected: its untouched golden output.

- [ ] **Step 7: Prove level-triggering is load-bearing**

Restore edge-triggering — make `poll_sleep` return `POLL_READY` unconditionally on any poll after the first — and confirm `a_sleep_polled_early_re_stages_the_same_deadline` **fails**. Restore by raw byte copy verified by SHA-256, and confirm green.

- [ ] **Step 8: Commit**

```bash
git add crates/nova-runtime/src/time.rs crates/nova-runtime/src/task.rs
git commit -F <path-to-utf8-message-file>
```

Subject: `fix(runtime): make sleep level-triggered, storing a deadline not a duration`. The body must record why level-triggering follows from Task 1, that the tag is gone and this converges on `poll_join`, the slot rename and why the compiler could not have caught it, and that `checked_add` removes a panic that could previously cross a poll boundary.

---

## Task 3: The combinator in the runtime

**Files:**
- Modify: `crates/nova-runtime/src/task.rs` — add `poll_timeout`, `nova_rt_task_timeout_future` and their slot constants beside the sleep and join parkers
- Modify: `crates/nova-runtime/src/lib.rs` — `symbols()`

**Interfaces:**
- Consumes: Task 1's merged staging; Task 2's `crate::time::now_nanos()`, `deadline_nanos_from_now`, `instant_from_deadline_nanos`.
- Produces: `#[no_mangle] pub extern "C-unwind" fn nova_rt_task_timeout_future(nanos: i64, fut: *mut u8) -> *mut u8`, registered in `symbols()` as `"nova_rt_task_timeout_future"`; status `0` = inner completed, `1` = elapsed.

- [ ] **Step 1: Write the failing tests**

```rust
    /// The inner future is polled **before** the deadline is checked, so work
    /// that completed is never reported as timed out -- and a zero-duration
    /// timeout over an already-ready future succeeds.
    ///
    /// Reversing the order in `poll_timeout` fails exactly this.
    #[test]
    fn a_zero_duration_timeout_over_a_ready_future_reports_completed() {
        let inner = nova_rt_task_sleep_future_nanos(0);
        let fut = nova_rt_task_timeout_future(0, inner);
        let status = poll_once_for_test(fut);
        assert_eq!(status, POLL_READY);
        assert_eq!(
            output_of_for_test(fut), TIMEOUT_STATUS_COMPLETED,
            "the inner future was ready, so this must not report elapsed"
        );
    }

    /// A past deadline over a future that will not complete reports elapsed.
    #[test]
    fn an_expired_timeout_over_a_pending_future_reports_elapsed() {
        let inner = nova_rt_task_sleep_future_nanos(60 * 1_000_000_000);
        let fut = nova_rt_task_timeout_future(0, inner);
        let status = poll_once_for_test(fut);
        assert_eq!(status, POLL_READY);
        assert_eq!(output_of_for_test(fut), TIMEOUT_STATUS_ELAPSED);
    }

    /// A live timeout over a pending inner parks, and the staged park carries
    /// **both** the inner's wait and this timeout's deadline, merged.
    #[test]
    fn a_live_timeout_over_a_pending_future_stages_both_deadlines_merged() {
        let inner = nova_rt_task_sleep_future_nanos(60 * 1_000_000_000);
        let fut = nova_rt_task_timeout_future(1_000_000, inner);
        let before = Instant::now();
        let status = poll_once_for_test(fut);
        let staged = staged_deadline_for_test();
        PENDING_PARK.with(|cell| {
            cell.take();
        });

        assert_eq!(status, POLL_PENDING, "neither the inner nor the deadline is done");
        let staged = staged.expect("a park must be staged");
        let delta = staged.duration_since(before);
        assert!(
            delta <= Duration::from_secs(1),
            "the merged deadline must be the timeout's 1ms, not the inner's 60s -- got {delta:?}"
        );
    }

    /// The inner future survives a collection while only the timeout's state
    /// references it, which is what makes storing its fat pointer sufficient.
    #[test]
    fn a_timeouts_inner_future_survives_a_collection() {
        let inner = nova_rt_task_sleep_future_nanos(60 * 1_000_000_000);
        let inner_state = state_of(inner);
        let fut = nova_rt_task_timeout_future(60 * 1_000_000_000, inner);
        gc::collect_for_test();
        assert!(
            gc::object_info(inner_state).is_some(),
            "the inner state must stay live: the timeout's scanned state holds its fat pointer"
        );
        let _ = fut;
    }
```

**Two of those helpers do not exist and one pair does.** `gc::collect_for_test()` and `gc::object_info(addr)` are real — use them as written. `poll_once_for_test` and `output_of_for_test` are **not**: `task.rs` has no poll-once helper, so add these two `#[cfg(test)]` helpers beside `staged_deadline_for_test` and use them in all four tests above:

```rust
    /// Poll a future once under a borrowed task context, draining any park it
    /// staged.
    ///
    /// The park must be drained: `stage_park` aborts the process when a second
    /// deadline is staged over a first, so a leftover entry kills a later test
    /// under `--test-threads=1`.
    #[cfg(test)]
    fn poll_once_for_test(fut: *mut u8) -> i64 {
        let poll_addr = unsafe { (fut as *mut usize).add(FUTURE_SLOT_POLL).read() };
        // SAFETY: word 0 is a `PollFn` bit pattern by `fut`'s own construction.
        let poll: PollFn = unsafe { std::mem::transmute(poll_addr) };
        let state = state_of(fut) as *mut u8;
        let previous = current_task();
        set_current_for_test(Some(0));
        // SAFETY: `poll`/`state` are the pair inside `fut`; `task_ctx` is null.
        let status = unsafe { poll(state, std::ptr::null_mut()) };
        PENDING_PARK.with(|cell| {
            cell.take();
        });
        set_current_for_test(previous);
        status
    }

    /// The value a future wrote to its own output slot.
    #[cfg(test)]
    fn output_of_for_test(fut: *mut u8) -> i64 {
        let slots = state_of(fut) as *mut i64;
        // SAFETY: every future this module builds is at least
        // `STATE_MIN_SIZE`, so the output slot is in bounds.
        unsafe { slots.add(STATE_SLOT_OUTPUT).read() }
    }
```

Note the third test above reads `staged_deadline_for_test()` **before** the drain, so it needs the poll inlined rather than going through `poll_once_for_test` — write that one with the raw idiom, as Task 2's tests do.

- [ ] **Step 2: Run and confirm failure**

Run: `cargo test --locked -p nova-runtime timeout`
Expected: FAIL — `nova_rt_task_timeout_future` does not exist.

- [ ] **Step 3: Add the slot constants and status values**

```rust
/// Where a timeout future stores the inner future's `{ poll_code, state }`
/// fat pointer.
///
/// The state object is scanned (`build_future` allocates with
/// `gc::alloc(size, true)`), so storing this pointer is the whole of the
/// rooting: it keeps the inner future and, transitively, its state reachable
/// for exactly as long as the timeout future itself.
const TIMEOUT_SLOT_INNER: usize = STATE_SLOT_TEMPS;

/// Where a timeout future stores its deadline, as nanoseconds since
/// `crate::time::epoch()` -- the same encoding `SLEEP_SLOT_DEADLINE_NANOS` uses.
const TIMEOUT_SLOT_DEADLINE_NANOS: usize = STATE_SLOT_TEMPS + 1;

/// State size for a timeout future: the ABI minimum plus two temp slots.
const TIMEOUT_STATE_SIZE: usize = STATE_MIN_SIZE + 16;

const _: () = assert!(TIMEOUT_STATE_SIZE >= (TIMEOUT_SLOT_DEADLINE_NANOS + 1) * 8);

/// The inner future completed before the deadline.
pub const TIMEOUT_STATUS_COMPLETED: i64 = 0;

/// The deadline passed with the inner future still pending.
pub const TIMEOUT_STATUS_ELAPSED: i64 = 1;
```

- [ ] **Step 4: Write `poll_timeout`**

```rust
/// Poll the inner future, then this timeout's own deadline.
///
/// **That order is deliberate.** Polling the inner first means work that
/// completed is never reported as timed out, and it makes a zero-duration
/// timeout over an already-ready future succeed -- the least surprising
/// answer. The order is only *available* because `poll_sleep` is
/// level-triggered: with an edge-triggered sleep, a woken sleep could report a
/// completion it had not earned, forcing a deadline-first check as a defence.
///
/// **Abandonment needs no code here.** When the inner returns
/// [`POLL_PENDING`] and this function then returns [`POLL_READY`], the inner
/// has already staged a park -- and [`poll_one`] takes `PENDING_PARK`
/// unconditionally, discarding it. The mechanism that stops a finished task
/// faking a deadlock cleans up the abandoned park for free.
///
/// **Must not unwind.** Every slot access is a raw read with the SAFETY note
/// below; an out-of-range status from the inner poll goes to [`abort_with`],
/// the same route a staging collision takes, never `panic!`.
unsafe extern "C-unwind" fn poll_timeout(state: *mut u8, _task_ctx: *mut u8) -> i64 {
    let slots = state as *mut i64;
    // SAFETY: `state` is the object `nova_rt_task_timeout_future` built, at
    // least `TIMEOUT_STATE_SIZE` bytes, so every slot below is in bounds.
    let inner = unsafe { slots.add(TIMEOUT_SLOT_INNER).read() } as *mut u8;
    // SAFETY: same object.
    let deadline = unsafe { slots.add(TIMEOUT_SLOT_DEADLINE_NANOS).read() };

    // SAFETY: `inner` is the fat pointer written at construction, so word 0 is
    // its poll function and word 1 its state -- the layout `build_future`
    // guarantees and `async_lower.rs` independently emits.
    let inner_poll_addr = unsafe { (inner as *mut usize).add(FUTURE_SLOT_POLL).read() };
    // SAFETY: that word is a `PollFn` bit pattern by the inner future's own
    // construction; a fn pointer and a `usize` are both pointer-width.
    let inner_poll: PollFn = unsafe { std::mem::transmute(inner_poll_addr) };
    // SAFETY: same fat pointer, state word.
    let inner_state = unsafe { (inner as *mut usize).add(FUTURE_SLOT_STATE).read() } as *mut u8;

    // SAFETY: `inner_poll`/`inner_state` are the pair inside `inner`;
    // `task_ctx` is always null, matching every other call site in this crate.
    let inner_status = unsafe { inner_poll(inner_state, std::ptr::null_mut()) };
    if inner_status == POLL_READY {
        // SAFETY: same object, output slot.
        unsafe { slots.add(STATE_SLOT_OUTPUT).write(TIMEOUT_STATUS_COMPLETED) };
        return POLL_READY;
    }
    if inner_status != POLL_PENDING {
        abort_with(&format!(
            "nova_rt: a future polled by a timeout returned {inner_status}, which is \
             neither POLL_PENDING ({POLL_PENDING}) nor POLL_READY ({POLL_READY})"
        ));
    }
    if crate::time::now_nanos() >= deadline {
        // SAFETY: same object, output slot.
        unsafe { slots.add(STATE_SLOT_OUTPUT).write(TIMEOUT_STATUS_ELAPSED) };
        return POLL_READY;
    }
    stage_park(Wait::Deadline(instant_from_deadline_nanos(deadline)));
    POLL_PENDING
}
```

`abort_with(msg: &str) -> !` takes a `&str`, and building a `String` on an abort path inside a poll is already established: `stage_park` does `Err(msg) => abort_with(&msg)` with a `String` from `collision_msg`. The textual no-panic guards (`no_net_intrinsic_can_panic` and friends) are **per-file** — each does `include_str!` on its own module and there is none for `task.rs` — so `format!` here trips nothing.

- [ ] **Step 5: Write the constructor and register the symbol**

```rust
/// A fresh `Future<Int>` that polls `fut` until it completes or `nanos`
/// nanoseconds pass, reporting which happened.
///
/// The value is **not** carried here: the inner future wrote its own output
/// slot, and Nova reads it with `task_output` on the inner future itself, so
/// nothing moves an `i64` that might be a scalar or a pointer.
#[no_mangle]
pub extern "C-unwind" fn nova_rt_task_timeout_future(nanos: i64, fut: *mut u8) -> *mut u8 {
    let poll: PollFn = poll_timeout;
    let deadline = deadline_nanos_from_now(nanos);
    build_future(poll, TIMEOUT_STATE_SIZE, |slots| {
        // SAFETY: `slots` addresses a live `TIMEOUT_STATE_SIZE` block, and both
        // slots are in bounds by the assertion above.
        unsafe { slots.add(TIMEOUT_SLOT_INNER).write(fut as i64) };
        unsafe { slots.add(TIMEOUT_SLOT_DEADLINE_NANOS).write(deadline) };
    })
}
```

Then add to `symbols()` in `crates/nova-runtime/src/lib.rs`, in the same shape as its neighbours:

```rust
        (
            "nova_rt_task_timeout_future",
            task::nova_rt_task_timeout_future as *const u8,
        ),
```

**Both halves matter.** The string is what the JIT resolves; a renamed function with a stale string compiles clean and fails at run time.

- [ ] **Step 6: Verify**

Run: `cargo build --locked --workspace` then `cargo test --locked --workspace --all-features --no-fail-fast`
Expected: **990 passed, 0 failed, 8 ignored**.
Then clippy `-D warnings`, fmt `--check`, and a CRLF re-check on both files.

- [ ] **Step 7: Prove the poll order is load-bearing**

Move the deadline check above the inner poll, rebuild, and confirm `a_zero_duration_timeout_over_a_ready_future_reports_completed` **fails** while the elapsed test still passes. Restore by raw byte copy verified by SHA-256.

- [ ] **Step 8: Commit**

```bash
git add crates/nova-runtime/src/task.rs crates/nova-runtime/src/lib.rs
git commit -F <path-to-utf8-message-file>
```

Subject: `feat(runtime): add a timeout combinator that polls its inner future first`. The body must record the poll order and why level-triggered sleep makes it available, that abandonment needs no code because `poll_one` discards the staged park, that the scanned state object is the whole of the rooting, and the Step 7 mutation result.

---

## Task 4: Wire the builtin, the Nova surface and the fixtures

**Files:**
- Modify: `crates/nova-resolver/src/lib.rs` — `Builtin::TaskTimeoutFuture`, its spelling, `STD_ONLY` 59→60 (~657)
- Modify: `crates/nova-typeck/src/check.rs` — signature (~7161 area) and description (~15089 area)
- Modify: `crates/nova-mir/src/lower.rs`, `crates/nova-mir/src/lib.rs` — `RtFunc::TaskTimeoutFuture`, symbol, MIR signature
- Modify: `std/time/lib.nova`
- Create: `tests/runtime/timeout_ok.{nova,stdout}`, `timeout_elapsed.{nova,stdout}`, `timeout_value.{nova,stdout}`, `timeout_join_ok.{nova,stdout}`, `timeout_join_elapsed.{nova,stdout}`
- Modify: `crates/nova-cli/tests/run_tests.rs` — one `#[test]` per fixture

**Interfaces:**
- Consumes: Task 3's `nova_rt_task_timeout_future`.
- Produces: Nova `timeout<T>` and `TimeoutError`.

**Fixture registration is not automatic.** Every `tests/runtime/*.nova` needs an explicit `#[test]` in `run_tests.rs` or it runs **zero tests** while looking fine. The previous increment's plan omitted this and three fixtures would have silently never executed.

- [ ] **Step 1: Write the failing fixture**

`tests/runtime/timeout_value.nova` — the discriminating one:

```nova
async fn answer() -> Int {
    42
}

async fn main() {
    match timeout(Duration::from_secs(30), answer()).await {
        Ok(v) => println("${v}")
        Err(_) => println("elapsed")
    }
}
```

`tests/runtime/timeout_value.stdout`:

```
42
```

**Why this is the fixture that matters:** a mutant reading the *timeout's* own output slot instead of the inner future's prints the status `0` here rather than `42`. Nothing else in the suite distinguishes those.

That `match` spelling is verified: a probe of `match pick(true) { Ok(v) => println("${v}") Err(_) => println("elapsed") }` prints `7`, so arms need no separators and `Err(_)` binds nothing.

- [ ] **Step 2: Run and confirm it fails**

Run: `cargo test --locked -p nova-cli timeout_value`
Expected: FAIL — the test does not exist yet, and `timeout` is not a known name.

- [ ] **Step 3: Add the builtin through its seams**

`crates/nova-resolver/src/lib.rs` — a variant beside the other task builtins:

```rust
    /// `task_timeout_future(nanos: Int, fut: Future<T>) -> Future<Int>` — a
    /// fresh future that polls `fut` until it completes or `nanos` nanoseconds
    /// pass, producing `0` if the inner future completed and `1` if the
    /// deadline elapsed.
    ///
    /// Carries a status rather than the value, which is what keeps it
    /// expressible: a builtin's signature is a fixed list of types, and the
    /// value is read separately with [`Builtin::TaskOutput`] on the inner
    /// future, from the slot the inner future itself wrote. Backs `std/time`'s
    /// `timeout`. Std-only.
    TaskTimeoutFuture,
```

Add `Builtin::TaskTimeoutFuture => "task_timeout_future",` to the spelling table, add the variant to `STD_ONLY`, and change its length annotation from `[Builtin; 59]` to `[Builtin; 60]`.

`crates/nova-typeck/src/check.rs` — add to the arm list that groups the task builtins, to the signature table:

```rust
            Builtin::TaskTimeoutFuture => (
                vec![Ty::Int, future_of_param0()],
                Ty::Future(Box::new(Ty::Int)),
            ),
```

and to the description table, in the same shape as its neighbours:

```rust
                Builtin::TaskTimeoutFuture => (
                    (vec![Ty::Int, future_of_param0()], Ty::Future(Box::new(Ty::Int))),
                    "`task_timeout_future(nanos, fut).await` in `std/time`'s `timeout`",
                ),
```

`crates/nova-mir/src/lower.rs`: `Builtin::TaskTimeoutFuture => Lowering::Runtime(RtFunc::TaskTimeoutFuture),`

`crates/nova-mir/src/lib.rs`: add the `RtFunc::TaskTimeoutFuture` variant with a doc comment reading `` `(i64, ptr) -> ptr to { poll_code, state }` — a future that polls its inner future under a deadline ``, add `RtFunc::TaskTimeoutFuture => "nova_rt_task_timeout_future",` to the symbol table, and `RtFunc::TaskTimeoutFuture => (vec![MirTy::I64, MirTy::Ptr], MirTy::Ptr),` to the signature table.

- [ ] **Step 4: Add the Nova surface**

Append to `std/time/lib.nova`:

```nova
// What `task_timeout_future` reports when the inner future finished first.
const TIMEOUT_COMPLETED: Int = 0

// The error `timeout` reports when `d` elapsed before the inner future
// completed.
//
// Carries nothing, and needs to: the caller passed `d` in, so it already has
// the only fact there is. It is the one record in this library with no field
// to disclose as un-enforced, because it has no field.
pub record TimeoutError {}

// Run `fut`, giving up after `d`.
//
// Returns `Ok` with the future's value if it completed within `d`, and
// `Err(TimeoutError {})` if `d` elapsed first. A non-positive `d` still polls
// `fut` once, so an already-complete future succeeds rather than reporting a
// timeout it never had a chance to beat.
//
// **On timeout the inner future is ABANDONED, not cancelled.** Nothing tells
// it that it was dropped. That is free for a `sleep` (its state is collected),
// for a `join` (the joined task runs on independently), and for a `read` or
// `write` (the caller still holds the `TcpStream` and can close it). It is
// **not** free for `std/net`'s `connect`: a timed-out connect leaves a socket
// registered that nothing can reach or close, and it leaks until the process
// exits -- which is what `docs/adr/0012-file-descriptor-lifecycle.md` already
// says of any descriptor nobody closes.
pub async fn timeout<T>(d: Duration, fut: Future<T>) -> Result<T, TimeoutError> {
    if task_timeout_future(d.nanos, fut).await == TIMEOUT_COMPLETED {
        Ok(task_output(fut))
    } else {
        Err(TimeoutError {})
    }
}
```

Reading `task_output(fut)` on the **inner** future after the builtin has taken it is the same shape `std/task`'s `block_on` uses: `task_drive(fut)` then `task_output(fut)`.

- [ ] **Step 5: Register the fixture and confirm it passes**

Add to `crates/nova-cli/tests/run_tests.rs`, modelled exactly on an existing `*_run` test:

```rust
/// `timeout` returning the inner future's value, not the combinator's status.
///
/// The one fixture that distinguishes those: a `task_output` reading the
/// timeout future's own output slot prints `0` here instead of `42`.
#[test]
fn timeout_value_run() {
    let expected = std::fs::read_to_string(repo_root().join("tests/runtime/timeout_value.stdout"))
        .expect("expected-output fixture exists")
        .replace("

", "
");
    nova()
        .arg("run")
        .arg(repo_root().join("tests/runtime/timeout_value.nova"))
        .assert()
        .success()
        .stdout(expected);
}
```

That is `time_elapsed_run`'s exact shape, including the `

` normalisation the goldens need on Windows. Repeat it per fixture in Step 6, changing only the name and the two paths.

Run: `cargo build --locked --workspace` then `cargo test --locked -p nova-cli timeout_value`
Expected: PASS, printing `42`.

- [ ] **Step 6: Add the remaining four fixtures**

`timeout_ok.nova` — a short sleep inside a long timeout, printing `ok`; golden `ok`.
`timeout_elapsed.nova` — a long sleep inside a short timeout, printing `elapsed`; golden `elapsed`.
`timeout_join_ok.nova` — `timeout(long, handle.join())` over a task that finishes, printing `ok`; golden `ok`.
`timeout_join_elapsed.nova` — `timeout(short, handle.join())` over a task that sleeps longer, printing `elapsed`; golden `elapsed`.

Each prints **which branch ran**, not an ordering: an order-only assertion is scale-invariant and could not fail on a unit error. The two `join` fixtures are the Task+Deadline pair that aborted the process before Task 1.

Register one `#[test]` per fixture in `run_tests.rs`.

- [ ] **Step 7: Verify**

Run: `cargo build --locked --workspace` then `cargo test --locked --workspace --all-features --no-fail-fast`
Expected: **995 passed, 0 failed, 8 ignored**.
Then clippy `-D warnings` and fmt `--check`, both clean. Confirm `std/time/lib.nova` is still CRLF.

Note the existing resolver test `std_only_builtins_are_visible_inside_std_modules` covers your new builtin automatically: it loops `1..=STD_MODULES.len()` × `Builtin::STD_ONLY`, so half-done wiring fails there rather than surfacing subtly.

- [ ] **Step 8: Prove the value comes from the inner future's slot**

Change `timeout`'s `Ok(task_output(fut))` to read the timeout future's own output instead, rebuild, and confirm `timeout_value_run` **fails** by printing `0`. Restore and confirm green.

- [ ] **Step 9: Commit**

```bash
git add crates/nova-resolver/src/lib.rs crates/nova-typeck/src/check.rs crates/nova-mir/src/lib.rs crates/nova-mir/src/lower.rs std/time/lib.nova tests/runtime/timeout_ok.nova tests/runtime/timeout_ok.stdout tests/runtime/timeout_elapsed.nova tests/runtime/timeout_elapsed.stdout tests/runtime/timeout_value.nova tests/runtime/timeout_value.stdout tests/runtime/timeout_join_ok.nova tests/runtime/timeout_join_ok.stdout tests/runtime/timeout_join_elapsed.nova tests/runtime/timeout_join_elapsed.stdout crates/nova-cli/tests/run_tests.rs
git commit -F <path-to-utf8-message-file>
```

Subject: `feat(std): add std/time's timeout over a status-carrying combinator`. The body must record `STD_ONLY` 59→60, that the value is read from the inner future's own slot, the abandonment contract with its one leak, and the Step 8 mutation result.

---

## Task 5: The records

**Files:**
- Modify: `CHANGELOG.md` (`[Unreleased]`)
- Modify: `nova-spec/20-STDLIB.md` (§9)
- Modify: `docs/adr/0009-async-execution-model.md`
- Modify: `docs/superpowers/specs/2026-08-17-std-time-design.md` (§1)

**Interfaces:**
- Consumes: everything from Tasks 1–4. Produces: no code.

- [ ] **Step 1: CHANGELOG**

Under `## [Unreleased]`, add `### Added` for `timeout<T>` and `TimeoutError` — naming the abandonment contract and its one leak — and `### Changed` for the two internal semantics changes: a deadline may now accompany any wait and two merge to the earlier, and `sleep` is level-triggered rather than edge-triggered. Both are internal, and both are the kind of change a reader tracking executor behaviour needs to see.

- [ ] **Step 2: Amend `nova-spec/20-STDLIB.md` §9**

§9 is now fully implemented. Remove the not-yet-delivered note the `std/time` increment added, and record the abandonment contract — that `timeout` abandons rather than cancels, and what that costs for a `connect`.

- [ ] **Step 3: Amend ADR 0009**

Three things ADR 0009 currently describes are now different: the park set's staging rule, `Wait`'s shape, and `sleep` being edge-triggered. Add a dated amendment in the form the file already uses (`2026-08-10, park-set` is the precedent), recording all three plus the new combinator. **No new ADR** — nothing here deviates from `nova-spec`.

- [ ] **Step 4: Point the std/time spec's deferral at this one**

`docs/superpowers/specs/2026-08-17-std-time-design.md` §1's "Out, deliberately" entry for `timeout<T>` ends *"belong with the widening that answers them"* and reads as an open question. Add a line saying which spec answered it and that it shipped, so a reader does not go looking for undone work.

- [ ] **Step 5: Sweep for claims this increment falsified**

```bash
grep -rn "SLEEP_SLOT_NANOS\|deadline_from_nanos\|edge-triggered\|Wait::Task(" --include=*.rs --include=*.md --include=*.nova . | grep -v "^./target/"
```

Every survivor must be either updated or genuinely still true. Apply the same history-versus-living rule the last increment established: `docs/superpowers/plans|specs/` documents describing **completed** increments are accurate history and stay; CHANGELOG sections for **released** versions stay; only living records — ADR 0009, `nova-spec/`, CHANGELOG `[Unreleased]`, and code comments — get amended. Report every survivor and why it is legitimate.

- [ ] **Step 6: Verify**

Run: `cargo build --locked --workspace` then `cargo test --locked --workspace --all-features --no-fail-fast`
Expected: **995 passed, 0 failed, 8 ignored** — unchanged, which for a records task is the evidence rather than a formality.
Then clippy and fmt clean.

- [ ] **Step 7: Commit**

```bash
git add CHANGELOG.md nova-spec/20-STDLIB.md docs/adr/0009-async-execution-model.md docs/superpowers/specs/2026-08-17-std-time-design.md
git commit -F <path-to-utf8-message-file>
```

Subject: `docs: record timeout, the widened staging rule and level-triggered sleep`.

---

## Expected final state

- **`main` + 5 commits**, linear, zero merge commits.
- **995 tests passing**, 0 failed, 8 ignored, 44 targets — from 982: +2 net (staging: four added, two retired) +2 (sleep) +4 (combinator) +5 (fixtures).
- `STD_ONLY` **60**, `STD_MODULES` **9**, `RESERVED_TYPE_NAMES` **7**.
- Seven CI checks green, including clippy on both ubuntu and windows and the MSRV 1.78 leg with `-D warnings`.
- `nova-spec/20-STDLIB.md` §9 fully implemented for the first time.
- Four mutations run and reported: `wake_due`'s arm deleted, sleep's edge-triggering restored, the timeout's poll order reversed, and `task_output` pointed at the wrong slot — each failing exactly one named test.
