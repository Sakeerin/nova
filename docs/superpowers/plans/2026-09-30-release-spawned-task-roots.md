# Release Spawned Task Roots Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the executor release a spawned task's GC root when the task completes, so a `spawn` whose handle is never joined stops keeping its state alive for the life of the process.

**Architecture:** A `Task` gains a `keep_root_until_taken` flag. Only `block_on`'s root task sets it, through a new `spawn_root_internal`. `poll_one` calls the existing `release_internal` at completion for every other task. `join` is unaffected because it reads a task's output through the handle's own future, not from the executor. A spawned task's output can no longer be *taken*, so the one test harness that did so moves to a new Rust-only `output_bits` read.

**Tech Stack:** Rust (the `nova-runtime`, `nova-driver` and `nova-cli` crates), Nova test fixtures, and the `nova-bench-http` load generator.

**Spec:** `docs/superpowers/specs/2026-09-30-release-spawned-task-roots-design.md`

## Global Constraints

- Branch `release-spawned-task-roots`, based on `a598d18`. Every commit ends with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`. The branch merges only through a PR, by rebase; no merge commits.
- CI's gates, which must all pass locally before the branch is offered:
  - `cargo test --locked --workspace --all-features --no-fail-fast`
  - `cargo clippy --locked --all-targets --all-features -- -D warnings`
  - `cargo fmt --all -- --check`
  - MSRV is 1.78.
- **No cargo inside subagents.** A subagent that builds is killed by its watchdog: 180 s for workflow agents, 600 s for background ones. The controller runs every cargo command; implementers edit only.
- **The collector frees memory only on Windows.** Elsewhere `gc::stack_base` returns `None`. Registry assertions (`gc::root_count`) work on every platform; anything that needs a collection to free something discriminates only on Windows.
- **Measurement rules:**
  - Record the byte size of every binary measured.
  - Delete `NAME` and `NAME.exe` before every `nova build -o NAME.exe`.
  - One fresh server process per reading.
  - State ranges, not points.
  - Claim a difference only when the ranges do not overlap.
- **Records:** amend with a dated note; never silently rewrite a merged record.
- **Scratch instrumentation is never committed.** It is saved as `C:\Users\SAKEER~1\AppData\Local\Temp\claude\D--Projects-nona\e50b9960-25c7-4b79-9a9f-e8700f4332fd\scratchpad\gc-instrumentation-and-switch.patch`, called **THE PATCH** below.

## Review Focus

1. **Joining long after completion, with collections in between.** A handle joined well after its task finished, with allocations in between, must still return the task's value. It is pinned by Task 3's end-to-end fixture under `NOVA_GC_STRESS=1`, which discriminates on Windows only.
2. **A spawned task that spawns and joins its own child.** When both tasks release at completion, in whatever order, both values must still come back. It is pinned by the `outer` task in Task 3's fixture.
3. **Re-spawning a completed but unjoined future.** It must be allowed (a new task id), not abort. This is a behaviour change (spec §3.4) and is pinned by `spawning_a_completed_unjoined_future_again_succeeds` in Task 2.
4. **A completed but unjoined task's stashed `fs` payload.** For example, an undrained error message must be released with the task, not kept for the process lifetime. It is pinned by `a_spawned_tasks_stashed_fs_payload_is_released_at_completion` in Task 2.
5. **`is_done` and the output copy after release, before any collection.** A handle holder asking `is_done` after the root is gone must still get `1`, and the `TASKS` entry keeps its output copy. It is pinned by `a_spawned_tasks_root_is_released_at_completion` in Task 2.

---

### Task 0: Clear the scratch edits

**Files:**
- Modify: `crates/nova-runtime/src/gc.rs`, `crates/nova-runtime/src/task.rs` (restore both to `HEAD`)

The spec's corrections (§3.4 on re-spawning, and §4's failing-first sentence) were committed together with this plan.

- [ ] **Step 1: Confirm THE PATCH exists and holds both scratch edits**

Run: `grep -c "NOVA_EXPERIMENT_RELEASE_AT_DONE\|pinned=\|NOVA_GC_THRESHOLD" "<THE PATCH>"`
Expected: a count of at least 3.

- [ ] **Step 2: Restore the two runtime files, discarding the scratch edits**

Run: `git checkout -- crates/nova-runtime/src/gc.rs crates/nova-runtime/src/task.rs && git status -s`
Expected: no output. The tree is clean, and there is nothing to commit.

---

### Task 1: `output_bits`, and move the test harnesses off spawned-task takes

This lands **before** the fix, so the workspace stays green. `output_bits` works on any completed task, and after Task 2 it becomes the only way to read a spawned task's output copy from Rust.

**Files:**
- Modify: `crates/nova-runtime/src/task.rs`: add `output_bits` beside `nova_rt_task_take_output`, and change `a_spawned_task_runs_to_completion_and_reports_done`.
- Modify: `crates/nova-driver/src/lib.rs`, the end of `run_async_fn_with` in `#[cfg(test)] mod async_end_to_end`.

**Interfaces:**
- Produces: `pub fn output_bits(id: i64) -> i64` in `nova_runtime::task`. It is safe: it panics if `id` is unknown or the task has not completed, and touches no GC root. It is **not** added to `symbols()`.

- [ ] **Step 1: Write the failing test**

In `task.rs`'s `mod tests`, change the last line of `a_spawned_task_runs_to_completion_and_reports_done`:

```rust
        assert_eq!(unsafe { nova_rt_task_is_done(fut) }, 1);
        assert_eq!(output_bits(id), 42);
    }
```

Add, beside it:

```rust
    /// `output_bits` refuses a task that has not completed: its `output`
    /// field still holds the `0` initializer, which is indistinguishable from
    /// a genuine `0`.
    #[test]
    fn output_bits_of_an_unfinished_task_panics() {
        let fut = make_future(poll_suspend_once, 0);
        let id = unsafe { nova_rt_task_spawn(fut) };
        let r = std::panic::catch_unwind(|| output_bits(id));
        assert!(r.is_err(), "output_bits must refuse an unfinished task");
    }
```

- [ ] **Step 2: Run it and watch it fail to compile**

Run: `cargo test --locked -p nova-runtime --lib output_bits`
Expected: FAIL with `cannot find function `output_bits` in this scope`.

- [ ] **Step 3: Add `output_bits`**

In `task.rs`, directly after `nova_rt_task_take_output`:

```rust
/// The raw bits a completed task's output was copied out as, **without**
/// taking it and without touching any GC root.
///
/// For Rust harnesses that read *scalar* outputs (`Int`, `Float`, `Bool`)
/// of a spawned task. After completion a spawned task's GC root is released,
/// so if these bits name a heap object, that object may already have been
/// freed; this function cannot tell and does not try. Nova code reads a
/// task's output through its future instead (`JoinHandle::join`), which
/// keeps any heap value reachable by ordinary tracing. Not registered in
/// [`crate::symbols`], so compiled Nova code cannot reach it.
///
/// # Panics
/// If `id` is unknown, or if task `id` has not completed.
pub fn output_bits(id: i64) -> i64 {
    TASKS.with(|tasks| {
        let tasks = tasks.borrow();
        let task = tasks
            .get(id as usize)
            .expect("output_bits: unknown task id");
        assert!(
            task.done,
            "output_bits: task {id} has not completed, so its output field \
             still holds its 0 initializer"
        );
        task.output
    })
}
```

- [ ] **Step 4: Switch the driver probe**

In `crates/nova-driver/src/lib.rs`, replace the tail of `run_async_fn_with`, from the comment `// A task left queued rather than polled -- the other failure shape --` through `unsafe { nova_runtime::task::nova_rt_task_take_output(PROBE_TASK_ID) }`, with:

```rust
        // A task left queued rather than polled -- the other failure shape --
        // is caught by `output_bits` itself: it asserts the task is done
        // before handing back its output, so that case panics here with its
        // own message rather than silently returning the `output: 0`
        // initializer. `output_bits` rather than `nova_rt_task_take_output`:
        // the probe is a *spawned* task, and a spawned task's GC root is
        // released at completion, so its output can no longer be taken. Every
        // probe in this module returns a scalar (`Int`, `Float`, `Bool` or
        // unit), which is what `output_bits` is for -- its bits are read, not
        // dereferenced.
        nova_runtime::task::output_bits(PROBE_TASK_ID)
```

Check every `run_async_fn(` and `run_async_fn_with(` call site in that module. Each probed `async fn` must return `Int`, `Float`, `Bool` or unit. At `a598d18` they are at lines 940, 958, 1027, 1062, 1083, 1102, 1121, 1150, 1275, 1308, 1337, 1358, 1383, 1409 and 1432. If any returns a heap type, stop and report it.

- [ ] **Step 5: Run the affected tests**

Run: `cargo test --locked -p nova-runtime --lib -- output_bits a_spawned_task_runs_to_completion_and_reports_done` and then `cargo test --locked -p nova-driver async_end_to_end`
Expected: all PASS. The driver run reports the same number of tests passed as on `main`.

- [ ] **Step 6: Commit**

```bash
git add crates/nova-runtime/src/task.rs crates/nova-driver/src/lib.rs
git commit -m "feat(runtime): add output_bits, and read probe outputs without taking them

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 2: Release a spawned task's root at completion

**Files:**
- Modify: `crates/nova-runtime/src/task.rs`: the `Task` struct, `spawn_internal`, a new `spawn_root_internal`, `run_to_completion`, `poll_one`, `take_output_internal`, `release_internal`'s and `nova_rt_task_take_output`'s doc comments, and `mod tests`.

**Interfaces:**
- Consumes: `output_bits(id: i64) -> i64` (Task 1).
- Produces: `Task::keep_root_until_taken: bool`, and `unsafe fn spawn_root_internal(future: *mut u8) -> i64`, which is private to `task.rs` and has the same safety contract as `spawn_internal`.

- [ ] **Step 1: Add the flag and `spawn_root_internal`, with no behaviour change yet**

In `struct Task`, after `taken`:

```rust
    /// Whether this task keeps its GC root past completion, until
    /// [`take_output_internal`] hands its output back. Set only by
    /// [`spawn_root_internal`], for `block_on`'s root task -- the one task
    /// whose output is read out of `Task::output` rather than through its
    /// future. Every other task has its root released in [`poll_one`] the
    /// moment it completes.
    keep_root_until_taken: bool,
```

In `spawn_internal`'s `tasks.push(Task { ... })`, add `keep_root_until_taken: false,` after `taken: false,`.

Directly after `spawn_internal`, add:

```rust
/// [`spawn_internal`], for `block_on`'s root task: the task keeps its GC root
/// past completion, until [`take_output_internal`] hands its output back.
///
/// `run_to_completion` needs this and nothing else does. It returns the
/// root's output out of `Task::output`, a copy in a Rust-heap `Vec` that
/// roots nothing, and it keeps polling other tasks -- which allocate --
/// between the root's completion and that take. The state object's root is
/// what keeps a heap-valued output alive across that gap.
///
/// # Safety
/// As [`spawn_internal`].
unsafe fn spawn_root_internal(future: *mut u8) -> i64 {
    // SAFETY: forwarding this function's own contract.
    let id = unsafe { spawn_internal(future) };
    TASKS.with(|tasks| {
        tasks
            .borrow_mut()
            .get_mut(id as usize)
            .expect("spawn_internal just registered this id")
            .keep_root_until_taken = true;
    });
    id
}
```

In `run_to_completion`, change `let root_id = unsafe { spawn_internal(future) };` to `let root_id = unsafe { spawn_root_internal(future) };`.

Run: `cargo test --locked -p nova-runtime --lib`
Expected: PASS, with the same count as after Task 1. Every task still keeps its root, so nothing observable has changed yet.

- [ ] **Step 2: Write the failing tests**

In `mod tests`, **replace** the body of `a_completed_tasks_state_stays_rooted_until_its_output_is_taken`, keeping its name because seven tracked files cite it. Change its one `nova_rt_task_spawn(fut)` to `spawn_root_internal(fut)`. Then replace the first paragraph of its doc comment with:

```rust
    /// The lifetime of a *kept* task's GC root -- `block_on`'s root, spawned
    /// through `spawn_root_internal` -- asserted on the registry directly
    /// rather than through a collection. An ordinary spawned task's root is
    /// released at completion instead; that is
    /// `a_spawned_tasks_root_is_released_at_completion`, below.
```

Keep the rest of its doc comment, with two edits:
- Change "releasing the root in `poll_one` on completion fails the middle assertion" to "releasing a kept task's root in `poll_one` on completion fails the middle assertion".
- Change the sentence "`JoinHandle::join` returns exactly that value." to "`run_to_completion` returns exactly that value, for `block_on`'s root." `join` no longer reads `Task::output`, so the old sentence is false. On `a598d18` it wraps across two comment lines: it starts at the end of `/// trip the threshold and free it.` and ends on the next line, `/// value.`

In `taking_a_tasks_output_also_releases_its_stashed_fs_payload` and `taking_an_output_twice_panics_rather_than_returning_stale_bits`, change `nova_rt_task_spawn(fut)` to `spawn_root_internal(fut)`. Add one sentence to each doc comment: "Spawned through `spawn_root_internal`, because only a kept task's output can be taken; an ordinary spawned task's root is released at completion."

Add these new tests to `mod tests`:

```rust
    /// An ordinary spawned task's GC root is released the moment it
    /// completes, not when its output is taken -- which, for a `spawn`
    /// whose handle is dropped, is never. Asserted on the registry, for the
    /// reason `a_completed_tasks_state_stays_rooted_until_its_output_is_taken`
    /// documents. The `TASKS` entry survives the release: `is_done` still
    /// answers, and the output copy is still there.
    #[test]
    fn a_spawned_tasks_root_is_released_at_completion() {
        let fut = make_future(poll_ready_now, 0);
        let state = state_of(fut);
        let id = unsafe { nova_rt_task_spawn(fut) };
        assert_eq!(
            gc::root_count(state),
            1,
            "spawn must register the state object exactly once"
        );

        unsafe { nova_rt_task_block_on(make_future(poll_ready_now, 0)) };
        assert_eq!(unsafe { nova_rt_task_is_done(fut) }, 1);
        assert_eq!(
            gc::root_count(state),
            0,
            "a spawned task's root must be released at completion, so a \
             handle nobody joins does not keep its state alive"
        );
        assert_eq!(output_bits(id), 7, "the TASKS entry keeps its output copy");
    }

    /// Releasing at completion goes through `release_internal`, so it
    /// releases the task's stashed `fs` payloads too -- an undrained error
    /// message must not outlive a task nobody joins.
    #[test]
    fn a_spawned_tasks_stashed_fs_payload_is_released_at_completion() {
        let fut = make_future(poll_ready_now, 0);
        let id = unsafe { nova_rt_task_spawn(fut) };
        let payload = crate::gc_str("release-at-completion-payload");
        let addr = payload as usize;
        crate::fs::stash_for_test(id, crate::fs::Slot::Buffer, payload);
        assert_eq!(gc::root_count(addr), 1, "stash_for_test must root its pointer");

        unsafe { nova_rt_task_block_on(make_future(poll_ready_now, 0)) };
        assert_eq!(unsafe { nova_rt_task_is_done(fut) }, 1);
        assert_eq!(
            gc::root_count(addr),
            0,
            "completion must release the task's stashed fs payload, not only \
             its state root"
        );
    }

    /// A spawned task's output cannot be taken once it has completed: its
    /// root is already gone, so the bits could name a freed object. The
    /// panic must say so, rather than claim a second take happened.
    #[test]
    fn taking_a_spawned_tasks_output_after_completion_panics_naming_the_release() {
        let fut = make_future(poll_ready_now, 0);
        let id = unsafe { nova_rt_task_spawn(fut) };
        unsafe { nova_rt_task_block_on(make_future(poll_ready_now, 0)) };

        let r = std::panic::catch_unwind(|| unsafe { nova_rt_task_take_output(id) });
        let msg = match r {
            Ok(v) => panic!("take must panic for a released spawned task, got {v}"),
            Err(e) => e.downcast_ref::<String>().cloned().unwrap_or_default(),
        };
        assert!(
            msg.contains("released when it completes"),
            "the panic must name the cause: {msg}"
        );
    }

    /// The behaviour change the spec records (§3.4): a completed spawned task
    /// has been released, so spawning its future again is allowed, as it
    /// already was after `join`. It re-polls the completed state machine --
    /// the footgun ADR 0009 accepts for that shape.
    #[test]
    fn spawning_a_completed_unjoined_future_again_succeeds() {
        let fut = make_future(poll_ready_now, 0);
        let first = unsafe { nova_rt_task_spawn(fut) };
        unsafe { nova_rt_task_block_on(make_future(poll_ready_now, 0)) };
        assert_eq!(unsafe { nova_rt_task_is_done(fut) }, 1);

        let second = unsafe { nova_rt_task_spawn(fut) };
        assert_ne!(first, second, "a re-spawn registers a new task");
        unsafe { nova_rt_task_block_on(make_future(poll_ready_now, 0)) };
        assert_eq!(unsafe { nova_rt_task_is_done(fut) }, 1);
    }

    std::thread_local! {
        static OBSERVED_ROOT_STATE: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
        static OBSERVED_ROOT_COUNT: std::cell::Cell<i64> = const { std::cell::Cell::new(-1) };
    }

    /// Records how many times the `block_on` root's state is registered, at
    /// the moment this task is polled -- which is after the root completed.
    unsafe extern "C-unwind" fn poll_observe_root(_state: *mut u8, _ctx: *mut u8) -> i64 {
        let root_state = OBSERVED_ROOT_STATE.with(|c| c.get());
        OBSERVED_ROOT_COUNT.with(|c| c.set(gc::root_count(root_state) as i64));
        POLL_READY
    }

    /// A `block_on` root that spawns an observer and completes on its first
    /// poll, with output 7.
    unsafe extern "C-unwind" fn poll_spawn_observer_then_ready(
        state: *mut u8,
        _ctx: *mut u8,
    ) -> i64 {
        OBSERVED_ROOT_STATE.with(|c| c.set(state as usize));
        let observer = make_future(poll_observe_root, 0);
        // SAFETY: `observer` is a fresh `make_future` result.
        unsafe { nova_rt_task_spawn(observer) };
        // SAFETY: `state` is a live state object (`make_future`'s contract).
        unsafe { *(state as *mut i64).add(STATE_SLOT_OUTPUT) = 7 };
        POLL_READY
    }

    /// `block_on`'s root keeps its root after completing, until
    /// `run_to_completion` takes its output. An observer task, spawned by the
    /// root and polled after it (`run_to_completion` drains the whole queue),
    /// sees the count while the root is complete but not yet taken. A guard,
    /// not a failing-first test: it passes before and after the fix, and
    /// fails under the mutation that releases regardless of the flag.
    #[test]
    fn a_block_on_roots_state_stays_rooted_until_taken() {
        let root = make_future(poll_spawn_observer_then_ready, 0);
        let root_state = state_of(root);
        assert_eq!(unsafe { nova_rt_task_block_on(root) }, 7);
        assert_eq!(
            OBSERVED_ROOT_COUNT.with(|c| c.get()),
            1,
            "the block_on root's state must stay rooted between its \
             completion and run_to_completion's take"
        );
        assert_eq!(
            gc::root_count(root_state),
            0,
            "run_to_completion's take must release it exactly once"
        );
    }
```

- [ ] **Step 3: Run the new tests and confirm which fail before the fix**

Run: `cargo test --locked -p nova-runtime --lib -- a_spawned_tasks_root_is_released_at_completion a_spawned_tasks_stashed_fs_payload_is_released_at_completion taking_a_spawned_tasks_output_after_completion_panics_naming_the_release a_block_on_roots_state_stays_rooted_until_taken a_completed_tasks_state_stays_rooted_until_its_output_is_taken`

(Several names go after `--`, where libtest accepts them as alternative filters. Before `--`, cargo accepts only one.)

Expected:
- FAIL: `a_spawned_tasks_root_is_released_at_completion` (left 1, right 0), `a_spawned_tasks_stashed_fs_payload_is_released_at_completion` (left 1, right 0), and `taking_a_spawned_tasks_output_after_completion_panics_naming_the_release` ("take must panic ... got 7").
- PASS: `a_block_on_roots_state_stays_rooted_until_taken` and `a_completed_tasks_state_stays_rooted_until_its_output_is_taken`.

Run separately: `cargo test --locked -p nova-runtime --lib spawning_a_completed_unjoined_future_again_succeeds`
Expected: the test binary exits abnormally, and stderr contains `this future is already a live task`. Before the fix this is an `abort_with`, not a clean failure.

- [ ] **Step 4: Release at completion in `poll_one`**

Replace the end of `poll_one`, from `TASKS.with(|tasks| {` through the closing brace of the function:

```rust
    let release_now = TASKS.with(|tasks| {
        let mut tasks = tasks.borrow_mut();
        let task = tasks
            .get_mut(id as usize)
            .expect("poll_one: task id is not registered");
        task.output = output;
        task.done = true;
        !task.keep_root_until_taken
    });
    wake_tasks_waiting_on(id);
    // Outside the borrow above: `release_internal` borrows `TASKS` itself.
    // After the wake, which only moves ids between Rust-heap queues and
    // allocates nothing on the GC heap, so no collection can fall between
    // completion and release.
    if release_now {
        release_internal(id);
    }
}
```

- [ ] **Step 5: Name the new cause in `take_output_internal`'s panic**

In `take_output_internal`, directly after the `assert!(task.done, ...)` and before `assert!(!task.taken, ...)`, add:

```rust
        assert!(
            !(task.taken && !task.keep_root_until_taken),
            "nova_rt_task_take_output: task {id}'s GC root has already been \
             released -- a spawned task's is released when it completes -- so \
             its output can no longer be taken; read it through its future, as \
             JoinHandle::join does"
        );
```

- [ ] **Step 6: Run the runtime suite**

Run: `cargo test --locked -p nova-runtime --lib`
Expected: PASS, including all six tests named in Step 3.

- [ ] **Step 7: Mutation 1. Remove the release and confirm the effect tests fail**

Delete the `if release_now { release_internal(id); }` block and run Step 3's first command.
Expected: the three effect tests fail as in Step 3, and the re-spawn test aborts again. Record the observed output. Restore the block.

- [ ] **Step 8: Mutation 2. Release regardless of the flag and confirm the guards fail**

Change `if release_now {` to `if true {` and run: `cargo test --locked -p nova-runtime --lib block_on`
Expected: `a_block_on_roots_state_stays_rooted_until_taken` fails with observed count 0, and `block_on_runs_a_ready_future_and_returns_its_output` panics on the *existing* `output was already taken` assert. The root's flag is still set, so the new assert's condition holds and the old one fires. Record the observed output. If the run hangs instead of failing, record that and kill it. Restore `if release_now {`.

- [ ] **Step 9: Rewrite the doc comments that state the old contract**

Each replacement is given in full.

`poll_one`: replace the paragraph starting `**The state object's GC root is deliberately *not* released here.**` through `lives.` with:

```rust
/// **A spawned task's GC root is released here, at completion.** The output
/// value stays in the state object's own [`STATE_SLOT_OUTPUT`]; a caller that
/// wants it holds the future (`JoinHandle::join` does), and the future
/// reaches the state object by ordinary tracing, so a heap-valued output
/// stays alive exactly as long as someone can still read it. A task whose
/// handle was dropped is then reachable from nothing, and the collector can
/// free it -- the leak this used to be. The one exception is `block_on`'s
/// root ([`spawn_root_internal`]): its output is read out of `Task::output`,
/// a Rust-heap copy that roots nothing, while other tasks keep running and
/// allocating, so it keeps its root until [`take_output_internal`].
```

`spawn_internal`: replace its first doc paragraph, the one starting `The \`gc::add_root\` here is paired`, with:

```rust
/// The `gc::add_root` here is paired with exactly one `gc::remove_root`,
/// through [`release_internal`] or [`take_output_internal`], whichever runs
/// first. For an ordinary task that is [`poll_one`] releasing it at
/// completion; for `block_on`'s root ([`spawn_root_internal`]) it is
/// `run_to_completion`'s take. This module owns that policy; `gc.rs` states
/// only the registry's own multiset contract and deliberately not who pairs
/// it or when.
```

And replace the inline comment that ends `see its doc comment for why completion is not where the root is released.` with:

```rust
    // module doc comment). Paired with exactly one `gc::remove_root`: at
    // completion in `poll_one`, or at take for `block_on`'s root -- see
    // `poll_one`'s doc comment.
```

`take_output_internal`: replace the paragraph starting `The cost of releasing the root here rather than at completion:` through `whichever of them a caller reaches.` with:

```rust
/// Only a task spawned through [`spawn_root_internal`] -- `block_on`'s root --
/// still has its root when it completes; every other task's is released in
/// [`poll_one`] at completion, so taking such a task's output is refused with
/// a panic that says why. This function or [`release_internal`] ends a task's
/// claim; both check the same `Task::taken` flag, so a task's single root is
/// cancelled at most once.
```

`release_internal`: replace the sentence `\`JoinHandle::join\` is the intended caller; what it does with the value is its own business.` with:

```rust
/// [`poll_one`] is the main caller, at completion. `JoinHandle::join` also
/// calls it (through `nova_rt_task_release`), and for a completed task that
/// call finds `taken` already set and does nothing.
```

`Task::taken`: replace `by either [\`take_output_internal\`] or [\`release_internal\`].` with `by either [\`take_output_internal\`] or [\`release_internal\`] -- the latter from [\`poll_one\`] at completion, for every task but \`block_on\`'s root.`

`nova_rt_task_take_output`: replace its doc paragraph starting `Taking releases the GC root` with:

```rust
/// Taking releases the GC root that was keeping the output alive, so this must
/// be called once per task and only after [`nova_rt_task_is_done`] reports
/// true. **Only `block_on`'s root still has that root when it completes**, and
/// `run_to_completion` takes it itself; every other task's root is released at
/// completion, so taking its output panics, naming that cause. A second take,
/// or a take before completion, panics too -- see [`take_output_internal`].
```

- [ ] **Step 10: Lint, format and re-run**

Run: `cargo fmt --all` then `cargo clippy --locked -p nova-runtime --all-targets --all-features -- -D warnings` then `cargo test --locked -p nova-runtime --lib`
Expected: clean, and PASS.

- [ ] **Step 11: Commit**

```bash
git add crates/nova-runtime/src/task.rs
git commit -m "fix(runtime): release a spawned task's GC root at completion

A spawn whose handle is never joined kept its task's state object rooted for
the life of the process. Only block_on's root, whose output is read out of
Task::output, now keeps its root until taken.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 3: End-to-end guard, and `JoinHandle::join`'s comment

**Files:**
- Create: `tests/runtime/spawned_output_survives_release.nova`, `tests/runtime/spawned_output_survives_release.stdout`
- Modify: `crates/nova-cli/tests/run_tests.rs`, adding a test after `gate_async_tasks_under_gc_stress`
- Modify: `std/task/lib.nova`, `JoinHandle::join`'s comment

**Interfaces:**
- Consumes: the Task 2 behaviour.

- [ ] **Step 1: Write the fixture**

`tests/runtime/spawned_output_survives_release.nova`:

```
// Spawned tasks whose `String` outputs must survive the executor releasing
// each task's GC root at completion: `join` reads the value through the
// handle's own future, which keeps the state object reachable. `outer`
// spawns and joins a child of its own, so two release-at-completion tasks
// nest. The loop allocates after the tasks can have completed and before
// they are joined, so under NOVA_GC_STRESS=1 collections run in that gap.
// See `a_spawned_tasks_string_output_survives_release_at_completion` in
// `crates/nova-cli/tests/run_tests.rs` for what this does and does not
// prove.

async fn make(n: Int) -> String {
    "value-${n}"
}

async fn outer() -> String {
    let inner = spawn(make(2))
    yield_now().await
    let v = inner.join().await
    "outer(${v})"
}

async fn run() -> String {
    let a = spawn(make(1))
    let b = spawn(outer())
    yield_now().await
    yield_now().await
    let mut acc = ""
    let mut i = 0
    while i < 50 {
        acc = "${acc}${i}"
        i = i + 1
    }
    let ra = a.join().await
    let rb = b.join().await
    "${ra} ${rb} ${i}"
}

fn main() {
    println(block_on(run()))
}
```

`tests/runtime/spawned_output_survives_release.stdout`, one line with a trailing newline:

```
value-1 outer(value-2) 50
```

- [ ] **Step 2: Check the fixture compiles and prints the expected line**

Run: `./target/release/nova check tests/runtime/spawned_output_survives_release.nova` then `NOVA_GC_STRESS=1 ./target/release/nova run tests/runtime/spawned_output_survives_release.nova`. Build first with `cargo build --release --locked --workspace` if needed.
Expected: `check` exits 0, and `run` prints exactly `value-1 outer(value-2) 50`. If `check` reports an error in the fixture's syntax, adjust the fixture, never the expectation, and record what changed.

- [ ] **Step 3: Add the test**

In `crates/nova-cli/tests/run_tests.rs`, after `gate_async_tasks_under_gc_stress`:

```rust
/// Spawned tasks' `String` outputs survive the executor releasing each task's
/// GC root at completion, under `NOVA_GC_STRESS=1` (collect on every
/// allocation), including a spawned task that spawns and joins its own child.
///
/// **What this guards:** that `join` keeps reading a completed task's output
/// through the handle's future. If it went back to reading the executor's
/// `Task::output` copy, a collection between completion and join could free
/// the string, and this would print garbage or crash.
///
/// **What it does not prove: soundness.** It discriminates only where the
/// collector frees memory, which is Windows (`gc::stack_base` is `None`
/// elsewhere). Even there, conservative over-retention can let it pass with
/// the rooting wrong, for the reason `gate_async_tasks_under_gc_stress`
/// documents. The release itself is pinned deterministically on every
/// platform by `nova-runtime`'s `a_spawned_tasks_root_is_released_at_completion`.
#[test]
fn a_spawned_tasks_string_output_survives_release_at_completion() {
    let expected = std::fs::read_to_string(
        repo_root().join("tests/runtime/spawned_output_survives_release.stdout"),
    )
    .expect("expected-output fixture exists")
    .replace("\r\n", "\n");
    nova()
        .env("NOVA_GC_STRESS", "1")
        .arg("run")
        .arg(repo_root().join("tests/runtime/spawned_output_survives_release.nova"))
        .assert()
        .success()
        .stdout(expected);
}
```

- [ ] **Step 4: Run it**

Run: `cargo test --locked -p nova-cli --test run_tests -- a_spawned_tasks_string_output_survives_release_at_completion gate_async_tasks join_handle_rejects a_recycled_state_address json_api_example_serves_its_routes`
Expected: all PASS.

- [ ] **Step 5: Update `JoinHandle::join`'s comment**

In `std/task/lib.nova`, replace the comment paragraph that starts `// \`task_release\` before the read, not after:` and ends `cannot stop a second call.` with:

```
    // `task_release` before the read. The executor already released a
    // spawned task's claim on its state object when the task completed, so
    // for every task `join` reaches -- it awaits completion first -- this
    // call finds the claim gone and does nothing. It stays because it is
    // idempotent and costs nothing, and because it keeps `join` correct if
    // the executor's release point ever moves. The read below depends only
    // on the claim this handle has through `self.fut`. Calling `join` twice
    // on one handle reads the same value twice -- deliberately, because Nova
    // has no move checking, so `self` by value cannot stop a second call.
```

Then read the module's top-of-file comment block about `JoinHandle`, which starts `// A handle on a spawned task`. It says `task_release` "has ended the executor's own claim". Change `That is what makes the value still readable after \`task_release\` has ended the executor's own claim on it.` to `That is what makes the value still readable after the executor has released its own claim on it, which for a spawned task happens when it completes.`

- [ ] **Step 6: Re-run the Nova-level suites that load `std/task`**

Run: `cargo test --locked -p nova-cli --test run_tests async` and `cargo test --locked -p nova-cli --test run_tests join`
Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add tests/runtime/spawned_output_survives_release.nova tests/runtime/spawned_output_survives_release.stdout crates/nova-cli/tests/run_tests.rs std/task/lib.nova
git commit -m "test(cli): guard spawned outputs across release-at-completion under GC stress

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 4: Records that state the leak

**Files:**
- Modify: `docs/adr/0009-async-execution-model.md` (the bullet at line 399 on `a598d18`, and the re-spawn bullet around line 336)
- Modify: `docs/adr/0012-file-descriptor-lifecycle.md` (the paragraph at line 56)
- Modify: `CHANGELOG.md` (`[Unreleased]`)
- Modify: every further file the set difference in Step 4 returns, or record it as deliberately left

- [ ] **Step 1: Amend ADR 0009's leak bullet**

Directly after the paragraph ending `The natural fix point is a future \`JoinHandle\` drop or cancellation, i.e. the gap above.`, insert:

```
  **Amended 2026-09-30 (branch `release-spawned-task-roots`): fixed for
  spawned tasks, without drop or cancellation.** `poll_one` now releases a
  spawned task's root at completion. The trade above assumed the output was
  read out of `Task::output`, and for a spawned task it is not: `join` reads
  it through the handle's own future, which keeps the state object
  reachable by ordinary tracing. Only `block_on`'s root still reads
  `Task::output`, so only it keeps release-at-take (`spawn_root_internal`).
  Measured on `examples/05-json-api` before the change: one retained root
  and about 58 objects per closed connection
  (`examples/05-json-api/BENCHMARK.md`, "FURTHER AMENDMENT 2026-09-30").
  Pinned by `a_spawned_tasks_root_is_released_at_completion` (`nova-runtime`).
```

- [ ] **Step 2: Amend ADR 0009's re-spawn bullet**

After the sentence ending `is what pins it as one.` in the bullet starting `**\`spawn\` used to accept the same future twice`, insert:

```
  **Amended 2026-09-30:** a *completed* spawned task is now released at
  completion, so its future may be spawned again without a `join` first --
  the same accepted re-poll, reached one step earlier. Two spawns with no
  executor run between them still abort, because the first task is still
  live (`spawning_the_same_future_twice_aborts`, `nova-cli`). Pinned by
  `spawning_a_completed_unjoined_future_again_succeeds` (`nova-runtime`).
```

- [ ] **Step 3: Amend ADR 0012's reference**

After the paragraph ending `for reasons specific to what would have to change to close them.`, insert:

```
**Amended 2026-09-30:** the task-state leak cited above is fixed for
spawned tasks (ADR 0009 §1's 2026-09-30 amendment). The descriptor leak
this decision is about is unchanged: a `File` still needs an explicit
`close`.
```

- [ ] **Step 4: Find every other tracked file that states the leak, by set difference**

Run:

```bash
comm -23 \
  <(git grep -l -i -E "never (taken|joined)|output is never taken|leaks its state|nobody ever joins|stays rooted until|release-at-take|a_completed_tasks_state_stays_rooted" | sort) \
  <(git diff --name-only a598d18..HEAD | sort)
```

For each file listed, open every hit, including the lines around it, because grep matches single lines and a statement can wrap. Decide per hit:
- **Amend with a dated note** if it states, as current, that a spawned task's state leaks until joined. Examples: the `[Unreleased]` CHANGELOG bullets and the `BENCHMARK.md` amendments dated 2026-09-30.
- **Leave** if it is a dated historical record that stays true as history: a released CHANGELOG section, or a dated spec or plan under `docs/superpowers/`. Leave a test-name citation that is still accurate for the kept-root test, such as `ci.yml` line 50 and ADR 0010 line 189. Check that each such citation's surrounding sentence doesn't claim the test covers ordinary spawned tasks; if it does, amend it.

Record every file and its decision in the commit message.

- [ ] **Step 5: CHANGELOG**

Under `## [Unreleased]`, add a `### Fixed` section if none exists, placed after `### Changed`, with:

```
- **A spawned task whose handle is never joined no longer keeps its state
  alive for the life of the process.** The executor now releases a spawned
  task's GC root when the task completes; only `block_on`'s own root keeps
  it until its output is taken. `join` is unaffected, because it reads the
  output through the handle's future. One behaviour follows: a completed
  spawned task's future may be spawned again without a `join` first, as it
  already could after one. `nova_rt_task_take_output` on a spawned task now
  panics, naming the release; the test harness that used it reads
  `nova_runtime::task::output_bits` instead. Measured effect: see
  `examples/05-json-api/BENCHMARK.md`'s amendment for this change.
```

- [ ] **Step 6: Commit**

```bash
git add -A docs CHANGELOG.md examples .github
git commit -m "docs: record that spawned tasks release their roots at completion

<list each file from Step 4 and whether it was amended or left, and why>

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

Before running it, replace the `<list ...>` line with the actual per-file decisions from Step 4. Check `git status` first: nothing under `crates/nova-runtime` may be staged.

---

### Task 5: Measure before against after, and record it

**Files:**
- Modify: `examples/05-json-api/BENCHMARK.md`: a new dated amendment after "FURTHER AMENDMENT 2026-09-30", and a dated pointer to it inside that section.
- Scratch, not committed: `/tmp/gcm/` builds and logs, and a `git worktree` at `a598d18`.

- [ ] **Step 1: Build the four binaries**

```bash
cd /d/Projects/nona/nova
git worktree add ../nova-before a598d18
# before, plain
(cd ../nova-before && cargo build --release --locked --workspace && rm -f /tmp/gcm/before && rm -f /tmp/gcm/before.exe && ./target/release/nova build examples/05-json-api/src/main.nova -o /tmp/gcm/before.exe)
# after, plain
cargo build --release --locked --workspace && rm -f /tmp/gcm/after && rm -f /tmp/gcm/after.exe && ./target/release/nova build examples/05-json-api/src/main.nova -o /tmp/gcm/after.exe
# instrumented: gc.rs part of THE PATCH only, never the task.rs switch
(cd ../nova-before && git apply --include=crates/nova-runtime/src/gc.rs "<THE PATCH>" && cargo build --release --locked --workspace && rm -f /tmp/gcm/before-i && rm -f /tmp/gcm/before-i.exe && ./target/release/nova build examples/05-json-api/src/main.nova -o /tmp/gcm/before-i.exe && git checkout -- crates/nova-runtime/src/gc.rs)
git apply --include=crates/nova-runtime/src/gc.rs "<THE PATCH>" && cargo build --release --locked --workspace && rm -f /tmp/gcm/after-i && rm -f /tmp/gcm/after-i.exe && ./target/release/nova build examples/05-json-api/src/main.nova -o /tmp/gcm/after-i.exe && git checkout -- crates/nova-runtime/src/gc.rs && cargo build --release --locked --workspace
ls -l /tmp/gcm/before.exe /tmp/gcm/after.exe /tmp/gcm/before-i.exe /tmp/gcm/after-i.exe
git status -s crates/
```

Expected: four binaries, each within a few kilobytes of the recorded release sizes (690,688 plain and 693,760–694,272 instrumented), and none near the 965,632 of a debug-runtime build. `git status -s crates/` prints nothing. Record the four sizes.

- [ ] **Step 2: Run the arms, alternated**

Use `/tmp/gcm/run_x.sh`, which takes `BIN`, `CONN`, `DUR` and `WARM` environment variables and a label, and seeds ten users before loading. Run:

```bash
cd /tmp/gcm
for r in 1 2 3; do
  for b in before after; do
    BIN=/tmp/gcm/$b.exe ./run_x.sh p_${b}_$r 2>&1 | grep -E "RESULT|mem:" | sed "s/^/p_${b}_$r /"
    BIN=/tmp/gcm/$b-i.exe ./run_x.sh i_${b}_$r NOVA_GC_DEBUG=1 2>&1 | grep -E "RESULT" | sed "s/^/i_${b}_$r /"
  done
done | tee /tmp/gcm/fix-series.log
```

Expected: 12 result lines, every one with `errors=0`. The whole series runs about 5 minutes.

- [ ] **Step 3: Extract the root and live-set plateaus**

```bash
cd /tmp/gcm
for f in i_before_1 i_before_2 i_before_3 i_after_1 i_after_2 i_after_3; do
  grep nova-gc-time $f.err | awk -v f=$f '{match($0,/objects=[0-9]+/);o=substr($0,RSTART+8,RLENGTH-8)+0; match($0,/pinned=[0-9]+/);p=substr($0,RSTART+7,RLENGTH-7)+0; match($0,/ns=[0-9]+/);ns+=substr($0,RSTART+3,RLENGTH-3); n++; if(n==100){o1=o;p1=p} lo=o; lp=p} END{printf "%s collections=%d pinned@100=%d objects@100=%d pinned_last=%d objects_last=%d gc_total_s=%.2f\n",f,n,p1,o1,lp,lo,ns/1e9}'
done | tee /tmp/gcm/fix-plateaus.log
```

Expected before: `pinned@100=212` and `pinned_last` of about 412. Expected after: `pinned@100=201` and `pinned_last` of about 201. The last collections can dip by 1–3 as the measured window's connections close, so if `pinned_last` is below 201, read the plateau from the middle of the log instead. Check each success criterion in spec §5 against these numbers and write down met or not met.

- [ ] **Step 4: Write the amendment**

In `examples/05-json-api/BENCHMARK.md`, insert a new section immediately before `## What was measured, and with what`, titled `## AMENDMENT 2026-09-30 (release-spawned-task-roots): the retained roots are released`. It holds:
- which builds were measured, their four byte sizes, the procedure (Step 2's command), and which readings were saved;
- a table of throughput ranges before against after, with a difference claimed only if the ranges do not overlap;
- a table of `pinned` and `objects` plateaus before and after the warmup boundary, plus total collector time;
- each spec §5 success criterion, marked met or not met;
- whether the gate's ten-user figure moved, as measured;
- a "What this does not settle" list: one host, Windows, three readings per arm, and `TASKS` growth still present.

Inside "FURTHER AMENDMENT 2026-09-30", at the end of its "What this does not settle" bullet on **What a fix looks like**, add: `**[Later on 2026-09-30: fixed on branch \`release-spawned-task-roots\` -- see the amendment below.]**`

- [ ] **Step 5: Commit, and clean up the worktree**

```bash
git add examples/05-json-api/BENCHMARK.md
git commit -m "docs(json-api): measure the retained-root fix before and after

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
git worktree remove ../nova-before
```

---

### Task 6: Whole-branch verification

- [ ] **Step 1: Run CI's gates locally**

Run: `cargo fmt --all -- --check`, then `cargo clippy --locked --all-targets --all-features -- -D warnings`, then `cargo test --locked --workspace --all-features --no-fail-fast 2>&1 | tee /tmp/gcm/full-test.log`
Expected: fmt and clippy exit 0. Sum every `test result:` line in the log for passed, failed and ignored, and report the three totals and the number of targets. Expected: 0 failed, and passed = 1141. That is 1134, the last recorded count, from `d029d71`, whose two successors are documentation only, plus the 7 tests this branch adds: 1 runtime test in Task 1, 5 in Task 2, and 1 CLI test in Task 3. Any other count is investigated, not explained away.

- [ ] **Step 2: Confirm no scratch code is on the branch**

Run: `git diff a598d18..HEAD | grep -c "NOVA_EXPERIMENT_RELEASE_AT_DONE\|nova-gc-time\|NOVA_GC_THRESHOLD\|pinned="`
Expected: the only hits are inside `BENCHMARK.md`'s quoted patches. Run `git diff a598d18..HEAD --stat` and confirm `crates/nova-runtime/src/gc.rs` is not listed.

- [ ] **Step 3: Whole-branch review**

A fresh reviewer, on the most capable model and forbidden to run cargo, reads the spec, this plan, and `git diff a598d18..HEAD`, and checks every claim in the records against the source and `/tmp/gcm` logs. Fix what it finds before anything is pushed.

- [ ] **Step 4: Stop and ask the user before pushing**

Pushing, opening the PR and merging are outward-facing, so ask first.
