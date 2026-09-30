# Release a spawned task's GC root at completion — design

**Date:** 2026-09-30
**Branch:** `release-spawned-task-roots`
**Base:** `main` == `origin/main` == `a598d18`, 724 commits. The last
recorded suite result is 1134 passed / 0 failed / 8 ignored across 45
targets, on `d029d71`. The two commits since, PRs #55 and #56, are
documentation only.

---

## 1. What this is, in one paragraph

Every `spawn` registers the task's state object as a GC root. For a task
created by `spawn`, the only thing that removes that root is
`JoinHandle::join`. A handle nobody joins therefore keeps its task's state
object, and everything that state reaches, alive for the rest of the
process. `examples/05-json-api` spawns one task per connection and never
joins any of them. `BENCHMARK.md`'s "FURTHER AMENDMENT 2026-09-30" measured
the cost: one retained root and about 58 objects per closed connection.
**This change makes the executor release a spawned task's root when the
task completes. The `block_on` root task keeps today's release-at-take.**
It fixes the leak for every Nova program that spawns without joining, with
no change to Nova source.

---

## 2. What is known, and from where

### 2.1 The mechanism (read from source on `a598d18`)

- `spawn_internal` in `crates/nova-runtime/src/task.rs` calls
  `gc::add_root(state)`. The only matching `gc::remove_root` calls are in
  `take_output_internal` and `release_internal`. **Neither runs when a task
  completes.** `poll_one`'s doc comment says the root is "deliberately
  *not* released here".
- The reason is stated in `poll_one`'s doc comment. On completion the
  output's bits are copied into `Task::output`, a field of a Rust `Vec` the
  collector does not scan. A later `take_output` hands those bits back, and
  if they name a heap object, the state object's root is what has kept that
  object alive.
- **Only one production path reads `Task::output`.** `run_to_completion`
  calls `take_output_internal(root_id)` on the root task that `block_on`
  spawns itself. The lowering never emits `RtFunc::TaskTakeOutput`: a test
  in `crates/nova-mir/tests/lower_tests.rs` asserts `block_on` does not.
  The program entry point generated in `mono.rs` discards
  `nova_rt_task_block_on`'s return value.
- **`JoinHandle::join` does not read `Task::output`.** It calls
  `task_release(self.fut)` and then `task_output(self.fut)`. `task_output`
  lowers to a field read through the future's state object
  (`Lowering::FutureOutput` in `crates/nova-mir/src/lower.rs`), so the value
  stays reachable through the handle by ordinary tracing.
- `timeout` polls its inner future in place and does not spawn it, so it is
  not affected.
- A task's payload slots (`fs::release_task_slots`) hold results only that
  task's own code takes back. A completed task is never polled again, so
  releasing its slots at completion loses nothing.
- ADR 0009 records this leak ("A spawned task whose output is never taken
  leaks its state object") and names "a future `JoinHandle` drop or
  cancellation" as the natural fix point. Nova has neither `Drop` nor
  cancellation.

### 2.2 Measured (`examples/05-json-api/BENCHMARK.md`, "FURTHER AMENDMENT 2026-09-30")

One run per arm, 200 connections, `--warmup 5`. The registered-root count
rose from 212 to 412 at the warmup boundary, where the generator closes its
200 warmup connections and opens 200 more. At 50 connections it rose from
62 to 112. Live objects went from 10,705 to 22,300. A scratch switch that
released every root at completion kept the count at 201 and the live set
at about 9,887, and the old tasks' state was freed.

That switch also released `block_on`'s root, so it is not this design. It
would make `run_to_completion`'s take panic.

### 2.3 Platform scope of any collection-based evidence

The collector only frees memory on Windows. Elsewhere `gc::stack_base`
returns `None` and a collection gives up. Registry assertions
(`gc::root_count`) are platform-independent. Anything that depends on a
collection actually freeing an object discriminates only on Windows.

---

## 3. The change

### 3.1 Runtime (`crates/nova-runtime/src/task.rs`)

- `Task` gains `keep_root_until_taken: bool`.
- `spawn_internal` leaves the flag `false`. Its signature is unchanged.
  Of its 13 callers, all in `task.rs`, only `run_to_completion` changes, to
  the function below.
- A new `spawn_root_internal(future)` calls `spawn_internal` and sets the
  flag to `true`. `run_to_completion` uses it for the `block_on` root, and
  nothing else does.
- In `poll_one`, after `task.done = true` and `wake_tasks_waiting_on(id)`,
  a task whose flag is `false` goes through the existing
  `release_internal(id)`. That call sets `taken`, removes the single root,
  and releases the task's payload slots. The `TASKS` entry stays, with
  `done` and the `output` copy, so `is_done`, `Wait::Task` and `join`'s
  wait keep working.
- `take_output_internal` on a task whose root was released at completion
  already panics on its `!task.taken` assert. The message changes to name
  that cause, and says to read the value through the task's future as
  `join` does. It stays a panic, not a wrong answer.

### 3.2 What happens to a completed spawned task afterwards

- **Something still holds its handle or future.** The state object is
  reachable by ordinary tracing. `join`'s `task_release` becomes a no-op,
  because `taken` is already set, and `task_output` reads the output slot
  through the future.
- **Nothing holds it.** A later collection can free the state object, on
  Windows (§2.3), unless the conservative scan finds a stale word that
  looks like a pointer to it. The sweep's existing `forget_freed_state` hook
  removes its `BY_STATE` entry. A later future that lands on a recycled
  address is rejected as never spawned, which is already the behaviour
  (`a_swept_states_key_is_dropped_so_a_recycled_address_cannot_misresolve`).

### 3.3 Callers and interfaces

- **`nova_rt_task_take_output`** (C/JIT export, `symbols()`) stays. It is
  still correct for a task that keeps its root until taken. After this
  change the only such task is `block_on`'s root, which `run_to_completion`
  takes itself. Its doc comment says so.
- **The driver probe** (`crates/nova-driver/src/lib.rs`,
  `#[cfg(test)] mod async_end_to_end`) spawns the async function under test
  as an ordinary task, uses `block_on` only to drain the queue, and then
  calls `nova_rt_task_take_output(PROBE_TASK_ID)`.
  - That now panics, so the probe switches to a new Rust-only function,
    `nova_runtime::task::output_bits(id) -> i64`.
  - `output_bits` asserts the task is done and returns the `Task::output`
    copy without touching any root. It is not in `symbols()`, so compiled
    Nova code cannot reach it.
  - Its doc comment states that bits naming a heap object may already have
    been freed, so it is for harnesses that read scalar outputs.
  - The plan checks every probe call site: each must compare `Int` or
    `Float` bits.
  - The probe keeps testing what it tests today: a spawned task, polled by
    `block_on`'s drain.
- **`JoinHandle::join`** in `std/task/lib.nova` is unchanged in code. Its
  `task_release` call is now a no-op for every task it reaches, because
  `join` awaits completion first. The call stays, since it is idempotent,
  and its comment is updated so it no longer claims to end the executor's
  claim.

### 3.4 One behaviour change for re-spawning

`spawn_internal` aborts when a future's task has not been released ("this
future is already a live task"). Today a completed but unjoined task is
unreleased, so spawning its future again aborts. After this change it is
released at completion, so the same spawn is allowed and re-polls the
completed state machine from its last suspend point. That is the footgun
ADR 0009 already accepts for re-spawning after `join`, and it is accepted
here for the same reason. The abort exists to stop two *live* tasks
driving one state object, and a completed task is no longer driven.
`spawning_the_same_future_twice_aborts` (`nova-cli`) spawns twice with no
executor run in between, so it still aborts. The plan adds a runtime test
pinning the new case, and the ADR 0009 amendment records it.

### 3.5 Doc comments rewritten because they state the old contract

In `task.rs`: `poll_one`, `spawn_internal`, `take_output_internal` (its
"The cost of releasing the root here rather than at completion" paragraph),
`release_internal`, `Task::taken`, and `nova_rt_task_take_output`. In
`std/task/lib.nova`: `JoinHandle::join`'s comment on `task_release`.

---

## 4. Testing

Tests that pin the fix's *effect* (1, the reworked tests in 3, and the
payload and re-spawn tests the plan adds) are shown failing against the
unfixed code before the fix goes in. Tests 2 and 4 guard the fix's
*safety*: they pass before and after it by design, and they fail only
under the mutations in 5, which is how their power is shown.
**[2026-09-30, final review: true of test 2 only. No mutation run on the
branch is caught by test 4 alone, and its doc comment in
`crates/nova-cli/tests/run_tests.rs` now says so.]**

1. **`a_spawned_tasks_root_is_released_at_completion`** (runtime, new).
   - Spawns a task, drains it to completion, and asserts
     `gc::root_count(state) == 0`.
   - Reads the registry and never collects, so it is deterministic and
     platform-independent (ADR 0010).
   - Fails today with a count of 1.
2. **`a_block_on_roots_state_stays_rooted_until_taken`** (runtime, new).
   - An observer task, spawned from inside the `block_on` root, runs after
     the root completes and asserts the root's `root_count` is still 1.
   - `run_to_completion` drains the whole queue after the root finishes, so
     the observer is polled.
   - Guards against the flag being ignored.
3. **Existing runtime tests that take a spawned task's output** are
   reworked to the new contract:
   - `a_spawned_task_runs_to_completion_and_reports_done`
   - `a_completed_tasks_state_stays_rooted_until_its_output_is_taken`,
     whose "stays until taken" half moves to test 2
   - `taking_a_tasks_output_also_releases_its_stashed_fs_payload`, which
     now asserts the payload is released at completion
   - `taking_an_output_twice_panics_rather_than_returning_stale_bits`,
     which now asserts the new panic cause

   `taking_the_output_of_an_unfinished_task_panics` is expected to pass
   unchanged, because the not-done assert comes first. The plan confirms
   that by running it.
4. **A joined heap-valued output survives release-at-completion**
   (`nova-cli` end to end, new).
   - A Nova fixture spawns a task that returns a `String`, lets it
     complete, allocates, then joins it and prints the value. The test
     asserts on stdout.
   - It runs under `NOVA_GC_STRESS=1`, so every allocation collects.
   - It discriminates only on Windows (§2.3), and over-retention can make
     it pass trivially. It guards against regressions and does not prove
     soundness, and its doc comment says so.
5. **Mutations, each run and its outcome recorded:**
   - Removing the new `release_internal` call from `poll_one` must fail
     test 1.
   - Releasing regardless of the flag must fail test 2 and make the
     `block_on` path panic.
   - Each mutant's behaviour is observed, not assumed, because a mutant
     that leaves a loop unsatisfiable hangs rather than fails.
6. **Must stay green:**
   - The full workspace, including
     `a_swept_states_key_is_dropped_so_a_recycled_address_cannot_misresolve`,
     `a_reachable_futures_key_survives_a_collection_so_a_second_read_resolves`,
     `spawning_the_same_future_again_after_release_succeeds`,
     `a_recycled_state_address_does_not_resolve_a_never_spawned_future` and
     `json_api_example_serves_its_routes`.
   - clippy `--all-targets --all-features -- -D warnings`, and fmt.
   - CI on ubuntu, windows and macOS.

**Deliberately not tested: that an unjoined task's state object is freed.**
A collection-based assertion on that is flaky under the conservative scan
(ADR 0010) and Windows-only. §5 shows it at the level that matters.

---

## 5. Measurement

**Workload.** `examples/05-json-api`, ten seeded users, 200 connections,
`--warmup 5 --duration 15`, one fresh process per reading.

**Builds.** Before (`main` at the branch point) and after (this change),
measured side by side in one script. Each is also built with the scratch
instrumentation re-applied: the per-cycle timer, `NOVA_GC_THRESHOLD`, and
the `pinned=` root count, **without** the experiment switch. The
instrumentation stays uncommitted and the amendment quotes its patch.
Every figure carries its binary's byte size, and sibling filenames are
deleted before each build.

**Arms, alternated, three readings each:**

- **Throughput, uninstrumented builds.** Before against after, as ranges.
  A difference is claimed only if the ranges do not overlap.
- **Timed, instrumented builds.** The root count and live objects before
  and after the warmup boundary, and total collector time.

**Success criteria.** Each is reported as met or not met. If any is not
met, the fix is not described as working.

- After the boundary, the root count equals the `main` task plus the open
  connections, 201, against 412 today.
- The live set after the boundary stays near the pre-boundary plateau,
  about 9,900 objects, against about 22,300 today.
- The 11 seeding connections no longer appear in the root count.

---

## 6. Records

- **`examples/05-json-api/BENCHMARK.md`:** a dated amendment with the §5
  figures, and a dated pointer to it from "FURTHER AMENDMENT 2026-09-30".
  Whether the gate's ten-user figure moves is reported as measured, not
  assumed.
- **`docs/adr/0009-async-execution-model.md`:** a dated amendment on the
  bullet "A spawned task whose output is never taken leaks its state
  object". It becomes fixed for spawned tasks, with `block_on`'s root
  keeping release-at-take. The bullet is amended, not rewritten.
- **`nova-spec/13-RUNTIME.md`:** the same, wherever it repeats the leak.
- **Every other tracked file that states the leak** is found by a set
  difference, not a list written in advance: every tracked file naming it
  (`git grep -l`), minus every file the branch touched. Each hit is
  amended or recorded as deliberately left.
- **`CHANGELOG.md`:** a `### Fixed` entry under `[Unreleased]`.

---

## 7. Out of scope

- **`TASKS` growth.** The `Vec` keeps one `Task`, 32 bytes of Rust memory
  on a 64-bit target (three 8-byte fields and the flags, padded), per
  spawn, joined or not, because ids are indices and nothing
  removes an entry. The payload-slot table's length grows the same way.
- **Removing `nova_rt_task_take_output` and `RtFunc::TaskTakeOutput`.**
  After this change nothing outside `run_to_completion` can use either
  correctly. Removing them is a separate cleanup.
- **`Drop`-based or explicit cancellation.** Neither exists, and this
  change does not need them.
- **`JoinHandle::detach()`.** Not needed once completion releases the root.
- **The generator's two-stage opening** of its second connection set,
  observed in `BENCHMARK.md` and unexamined.
- **The panicking-poll leak** that `poll_one`'s doc comment records. A
  poll that panics leaves its task undone and rooted, and that is
  unchanged.
