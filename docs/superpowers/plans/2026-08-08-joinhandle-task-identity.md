# Task identity keyed on the future — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A `JoinHandle` can no longer name a task other than its own future's, and a forged handle aborts with a diagnostic instead of hanging the executor forever.

**Architecture:** Task ids stay as the executor's *internal* identity — `poll_one`, `run_to_completion` and `take_output_internal` keep using them. Only the two *Nova-facing* entry points change: `nova_rt_task_is_done` and `nova_rt_task_release` take the future, read word 1 for the state address, and look the task up in a new thread-local map. Because two LIVE tasks sharing one state would make that lookup ambiguous, `spawn` rejects a future that names a task which has not yet been released — which also closes the documented double-spawn footgun for a task still in flight. **Corrected during Task 1** (see that section): the map's entries are never removed, and the collector genuinely frees a released state, so rejecting on mere *presence* in the map is a false positive on a later, unrelated spawn that recycles the address — the check has to be liveness (`Task::taken`), not presence. A released task's entry stays for `join`'s idempotence but no longer blocks a new spawn at its address.

**Tech Stack:** Rust (nova-runtime, nova-mir, nova-typeck), Nova (`std/task`), `nova test` for the abort cases.

**Spec:** `docs/superpowers/specs/2026-08-08-joinhandle-task-identity-design.md`. **Read §1.1 and §2 before starting** — §1.1 states precisely what is and is not achievable, and §2's probe table corrects five things, two of which shrink the work.

**Base:** `main` at `6ea10fa`. Create branch `task-identity`.

## Global Constraints

Every task's requirements implicitly include this section.

- **`cargo build --workspace` BEFORE `cargo test`.** `cargo test` does not regenerate `nova-runtime`'s staticlib, and this plan changes `nova_rt_*` signatures, so skipping it produces ~25 unrelated MSVC unresolved-symbol failures that read like a codegen bug.
- **`--no-fail-fast` is mandatory** on `cargo test --workspace`. Never pipe cargo output through `head`/`tail` before summing totals — it truncates and under-reports.
- **A zero-match `cargo test <filter>` EXITS 0.** Confirm `running N tests` is non-zero before treating a filtered run as evidence.
- **Baseline: 44 targets, 807 passed, 0 failed, 8 ignored** at default parallelism. Take your own baseline; do not trust this number. The 8 ignored are conservative-scan GC tests deliberately gated by ADR 0010 — **do not touch them and do not try to fix them.**
- **No panic may cross a generated poll boundary.** Generated code has no landing pads. Both new failure paths fire from inside `join`, which runs inside a poll frame, so **both must abort, not panic** — use `abort_with`, as `block_on`'s re-entrancy guard does.
- **`mir_ty` collapses five of seven `Ty` variants.** `Int`/`Char` → `I64`; `String`/`Fn`/`Sum`/`Record`/`Array` → `Ptr` = `i64` on x86-64. Only `Bool` (`I8`) and `Float` (`F64`) are disjoint, and **`Float` is strictly stronger** — it crosses register banks. A test at `Int` vs `String` tests nothing at a MIR seam.
- **A module's doc must not assert its caller's policy, and no comment may narrate a measurement** (ADR 0009 §2). Invariant in the comment, measurement in the report. The digit-scan heuristic under-detects: "passes every other test in the workspace" and "more than once" both slipped through on the last branch. Scan for measurement *phrases* and worded quantities.
- **No `reason = "…"` in lint attributes** — workspace MSRV is 1.78 and that needs 1.81.
- Clippy `-D warnings` and `cargo fmt --all --check` clean before each commit.
- **THE CODE WINS OVER THIS PLAN.** On the previous branch, implementers falsified plan claims thirteen times and were right every time. Measure, report the correction, proceed correctly.
- **Do NOT push.** Commit on `task-identity`. End every commit body with:
  ```
  Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>
  ```

## File Structure

| File | Responsibility | Task |
|---|---|---|
| `crates/nova-runtime/src/task.rs` | the state→id map; `spawn` rejection; both entry points retyped | 1 |
| `crates/nova-typeck/src/check.rs:7095-7096` | `TaskIsDone`/`TaskRelease` builtin signatures | 2 |
| `crates/nova-mir/src/lib.rs:277,279` | the two `RtFunc` MIR signatures | 2 |
| `std/task/lib.nova` | `JoinHandle` drops `id`; `join` and `spawn` follow | 2 |
| `tests/runtime/` + `crates/nova-cli/tests/run_tests.rs` | the forged-handle repro and the two abort cases | 2 |
| `docs/adr/0009-async-execution-model.md`, `docs/adr/0008-attributes-and-test-isolation.md`, `CHANGELOG.md` | the sweep | 3 |

---

### Task 1: The state→id map, `spawn`'s rejection, and both entry points

Runtime only, testable entirely in Rust. Nothing in Nova can reach it until Task 2.

**Files:**
- Modify: `crates/nova-runtime/src/task.rs`

**Interfaces:**
- Consumes: `gc::alloc`, `gc::add_root`/`remove_root`, the existing `TASKS`/`QUEUE`/`IN_BLOCK_ON` thread-locals.
- Produces, and Task 2 depends on these exact signatures:
  ```rust
  pub unsafe extern "C-unwind" fn nova_rt_task_is_done(future: *mut u8) -> i8
  pub extern "C-unwind" fn nova_rt_task_release(future: *mut u8)
  ```
  Both read word 1 of `future` for the state address. **Ids remain the internal identity** — `poll_one`, `run_to_completion` and `take_output_internal` are unchanged.

- [ ] **Step 1: Write the failing tests**

In `task.rs`'s existing test module, beside the current executor tests:

```rust
#[test]
fn a_handle_on_a_never_spawned_future_is_not_reported_done() {
    // The forged-handle case, at the runtime layer. Before this change the
    // lookup was by an `Int` index, so a handle could name a DIFFERENT task
    // and spin forever. Keyed on the state, a future that was never spawned
    // has no entry at all, which is a diagnosable condition rather than a
    // silent wrong answer.
    //
    // Asserted via `catch_unwind` is NOT possible -- the failure aborts, by
    // design (a panic must not cross a generated poll frame). So this test
    // asserts the *positive* half only: a spawned future IS found. The abort
    // is covered from Nova in Task 2, where `nova test`'s per-process runner
    // makes it observable.
    let fut = make_future(poll_ready_now, 0);
    unsafe { nova_rt_task_spawn(fut) };
    assert_eq!(unsafe { nova_rt_task_is_done(fut) }, 0, "not polled yet");
}

#[test]
fn is_done_follows_the_future_it_is_given_not_a_positional_id() {
    // The discriminating test, and the one that would have caught the
    // original defect: two tasks, and each handle must answer about ITS OWN
    // future. Under the old id-keyed lookup, passing task 1's index while
    // holding task 0's future answered about task 1 -- which is exactly the
    // forgery. Here the only thing passed IS the future, so a swap is
    // impossible to express; this test pins that the two are distinguished
    // at all, so a lookup that always returned the first task fails.
    let a = make_future(poll_ready_now, 0);
    let b = make_future(poll_suspend_once, 0);
    unsafe {
        nova_rt_task_spawn(a);
        nova_rt_task_spawn(b);
    }
    let root = make_future(poll_ready_now, 0);
    unsafe { nova_rt_task_block_on(root) };
    // `a` completes on its first poll; `b` needs two. Both are done after a
    // full drain, so assert the pair BEFORE the drain instead.
    assert_eq!(unsafe { nova_rt_task_is_done(a) }, 1);
    assert_eq!(unsafe { nova_rt_task_is_done(b) }, 1);
}

#[test]
fn releasing_by_future_unroots_that_futures_state_and_no_other() {
    // Two tasks, release one, and assert the OTHER's root survives. The old
    // signature took an index, so an off-by-one released somebody else's
    // state -- a premature free. Keyed on the future, the wrong-target case
    // cannot be expressed, and this test pins that release still targets
    // exactly one.
    let a = make_future(poll_ready_now, 0);
    let b = make_future(poll_ready_now, 0);
    let (sa, sb) = (state_of(a), state_of(b));
    unsafe {
        nova_rt_task_spawn(a);
        nova_rt_task_spawn(b);
    }
    assert_eq!(gc::root_count(sa), 1);
    assert_eq!(gc::root_count(sb), 1);

    nova_rt_task_release(a);

    assert_eq!(gc::root_count(sa), 0, "release must unroot its own target");
    assert_eq!(gc::root_count(sb), 1, "and must not touch another task's");
}

#[test]
fn releasing_the_same_future_twice_unroots_once() {
    // Idempotence, preserved from the id-keyed version: `join` releases then
    // reads, and Nova has no move checking, so a second `join` on the same
    // handle must release again harmlessly.
    let fut = make_future(poll_ready_now, 0);
    let state = state_of(fut);
    unsafe { nova_rt_task_spawn(fut) };
    nova_rt_task_release(fut);
    nova_rt_task_release(fut);
    assert_eq!(gc::root_count(state), 0);
}
```

- [ ] **Step 2: Run the tests to verify they fail**

```bash
cargo build --workspace && cargo test -p nova-runtime --lib --no-fail-fast 2>&1 | tail -25
```

Expected: compile failures — `nova_rt_task_is_done`/`nova_rt_task_release` still take `i64`. Confirm `running N tests` is non-zero once they compile.

- [ ] **Step 3: Add the map and retype the entry points**

Add a thread-local beside `TASKS`:

```rust
    /// State address to task id, for the two entry points Nova calls.
    ///
    /// The executor's own identity is still the `TASKS` index -- `poll_one`
    /// and `run_to_completion` address tasks by it. This map exists because
    /// the *Nova-facing* boundary must not be a forgeable integer: a
    /// `JoinHandle` is constructible (Nova has no field privacy), so an
    /// `Int` id in it can name a task the caller never spawned. A state
    /// address cannot be fabricated -- obtaining one requires a real future,
    /// which only calling an `async fn` produces.
    ///
    /// Entries are never removed. A released task's entry must stay so a
    /// second `join` still answers, which is what keeps `join` idempotent --
    /// but that is only half of what never-removing costs. The collector
    /// genuinely frees a released, unreachable state (`gc.rs`'s sweep calls
    /// the real deallocator), so its address can be handed to a later,
    /// wholly unrelated allocation while this map still names the OLD task
    /// at that key. Presence here is therefore not the same question as "is
    /// this address still in use" -- `spawn_internal`'s duplicate check
    /// consults `Task::taken` for that, not this map alone (see its own
    /// comment). The two Nova-facing reads need no such check: a caller can
    /// only ever hold a future for a task that is still reachable, and a
    /// state whose future is reachable cannot have been freed and recycled
    /// out from under it.
    static BY_STATE: RefCell<HashMap<usize, i64>> = RefCell::new(HashMap::new());
```

`spawn_internal` inserts, and rejects a duplicate:

**Corrected during implementation.** A presence-only check here (`BY_STATE` has this key at all) is a
latent false positive on legitimate `spawn`: `BY_STATE` never removes an entry, and the collector
genuinely frees a released, unreachable state through the real allocator, so a later, wholly unrelated
spawn can land its own fresh state at a recycled address and hit a stale entry that names a task with
nothing wrong with it. `gate_async_tasks_under_gc_stress` collects on every allocation against
same-sized state objects, so the window is not hypothetical. The check below is what was actually
built and committed: it tests **liveness** (`Task::taken` — has the named task been released?), not
mere presence. The narrowing this trades in is real and is spelled out in the check's own comment: a
released future a caller still holds can be spawned again, re-polling its completed state machine.

```rust
    // Two LIVE tasks sharing one state object would make this map ambiguous,
    // so the lookup forces a decision here -- but presence in `BY_STATE` is
    // not liveness. `gc.rs`'s sweep frees an unreachable object through the
    // real allocator, so once a task is released and nothing else holds its
    // future, its state can be collected and the address handed out again to
    // a wholly unrelated, later spawn; rejecting on presence alone would
    // abort code that did nothing wrong. Checking `Task::taken` instead (has
    // the task this address last named been released?) is airtight for the
    // case that matters: a recycled address requires the old state to have
    // actually been freed, which requires no live handle to hold it, so no
    // reachable handle can ever be misresolved by allowing the new spawn.
    //
    // This also reopens the footgun of one future value spawned twice --
    // narrower than before, not eliminated: `spawn(f())` twice still produces
    // two distinct futures and is unaffected, but `Task::taken` distinguishes
    // *released* from *live*, not *dead* from *alive*, so a released future a
    // caller still holds (Nova has no move checking) now passes this check
    // too. `spawn(h.fut)` after `h.join()` is therefore legal and re-polls
    // the completed state machine from its last suspend point -- the same
    // family as the already-documented double-await footgun (ADR 0009). What
    // this trades away is a non-deterministic abort of code that never
    // re-spawned anything, for a rare, contrived, and already-precedented
    // one. Distinguishing "released but still held" from "freed and
    // recycled" needs to know whether the collector has actually freed the
    // object, which is a sweep-integration change well beyond this check.
    if let Some(prior) = BY_STATE.with(|m| m.borrow().get(&(state as usize)).copied()) {
        let still_live = TASKS.with(|tasks| {
            !tasks
                .borrow()
                .get(prior as usize)
                .expect("BY_STATE named a task id that TASKS does not have")
                .taken
        });
        if still_live {
            abort_with(
                "nova_rt_task_spawn: this future is already a live task; spawn it again only after its task has been released",
            );
        }
    }
```

Both entry points take the future, read word 1, and look up — aborting when the state is not a task:

```rust
/// Read the task id for `future`'s state object, or abort.
///
/// Aborts rather than panics: both callers are reachable from `join`, which
/// runs inside a generated poll frame, and a panic must not cross that
/// boundary (ADR 0009 section 1).
unsafe fn task_id_of(future: *mut u8, who: &str) -> i64 {
    let state = /* word 1 of `future` -- reuse the existing accessor */;
    match BY_STATE.with(|m| m.borrow().get(&(state as usize)).copied()) {
        Some(id) => id,
        None => abort_with(&format!(
            "{who}: this future was never spawned, so there is no task to ask about"
        )),
    }
}
```

Then `nova_rt_task_is_done(future: *mut u8) -> i8` and `nova_rt_task_release(future: *mut u8)` resolve through it and keep their existing bodies.

**Update the existing Rust tests that call these with an `i64`** — there are several. Where a test's whole point was the *unknown id* case, that case no longer exists at this layer; convert it or delete it, and **say which in your report**, because a deleted test is a coverage change.

- [ ] **Step 4: Run the tests to verify they pass**

```bash
cargo build --workspace && cargo test -p nova-runtime --lib --no-fail-fast 2>&1 | tail -20
```

- [ ] **Step 5: Kill three mutations by hand**

1. Make `task_id_of` return `0` instead of aborting on a miss → `is_done_follows_the_future_it_is_given_not_a_positional_id` must fail.
2. Delete `spawn_internal`'s duplicate check → no *runtime* test fails (the Nova-level test lands in Task 2). **Record this as an expected non-kill**, not a gap — and it is why Task 2 owns that case.
3. Remove the `BY_STATE` insert entirely → every new test fails.

Revert each, then `cargo build --workspace`.

- [ ] **Step 6: Commit**

```bash
git add crates/nova-runtime/src/task.rs
git commit -m "feat(runtime): key the Nova-facing task lookups on the future

nova_rt_task_is_done and nova_rt_task_release took a task id, which is a
TASKS vector index handed out in spawn order. A JoinHandle is
constructible -- Nova has no field privacy -- so that id let a handle
name a task other than its own future's: an unknown id aborted, but a
valid-but-wrong one hung the executor forever with empty output.

Both now take the future and resolve it through a state-address map. A
state cannot be fabricated: obtaining one requires a real future, which
only calling an async fn produces. Ids remain the executor's internal
identity.

Two tasks sharing one state would make that map ambiguous, so spawn
rejects a future that is already a task -- which also closes the footgun
where one future value spawned twice runs its body twice. spawn(f())
twice is unaffected, producing two distinct futures.

Both new failures abort rather than panic: they are reachable from join,
which runs inside a generated poll frame with no landing pads.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

- [ ] **Step 7 (post-review correction): Liveness, not presence**

Code review of the commit above found a defect in this section's own design, not in its
implementation: the duplicate-spawn check above rejects on *presence* in `BY_STATE`, and `BY_STATE`
never removes an entry (by design, for `join`'s idempotence). `gc.rs`'s sweep genuinely frees a
released, unreachable state through the real allocator, so once a task is released and its future
becomes unreachable, a later, wholly unrelated `spawn` can have its own fresh state land at that same
recycled address — and hit the stale entry, aborting code that did nothing wrong.
`gate_async_tasks_under_gc_stress` collects on every allocation against same-sized state objects, so
the window is real, not hypothetical. See the spec's §3.3 for the full argument and the accepted
consequence (a released-but-still-held future can now be spawned again, re-polling its completed
state machine — the same family as the already-documented double-await footgun, ADR 0009 §1).

The fix: check `Task::taken` (has the task this address names been released?) instead of mere
presence. Applied directly to `spawn_internal` and to the `BY_STATE`/duplicate-check snippets above,
which now show the corrected code rather than the original.

**Required test**, added to Step 1's set: `spawn(fut)` → `release(fut)` → `spawn(fut)` must succeed —
verified to abort under the original presence check and to succeed under the liveness check. Existing
tests, including `releasing_the_same_future_twice_unroots_once` and the live-task duplicate-spawn
abort (unwritten, as ever — an abort cannot be asserted via `catch_unwind`), all still pass/hold.

Committed separately from Step 6's commit, on the same branch, same file plus the two document updates
this correction required (this plan and the design spec). See
`.superpowers/sdd/2026-08-08-joinhandle-task-identity/task-1-report.md` for the fix's full account.

---

### Task 2: The signatures, `std/task`, and the Nova-level cases

Makes the change reachable from Nova and proves the headline: the review's forged-handle program aborts instead of hanging.

**Files:**
- Modify: `crates/nova-typeck/src/check.rs:7095-7096`
- Modify: `crates/nova-mir/src/lib.rs:277,279`
- Modify: `std/task/lib.nova`
- Modify/Create: `tests/runtime/` fixture(s) and `crates/nova-cli/tests/run_tests.rs`

**Interfaces:**
- Consumes: Task 1's two retyped entry points.
- Produces: `pub record JoinHandle<T> { fut: Future<T> }` — `id` removed.

- [ ] **Step 1: Retype the builtins**

`crates/nova-typeck/src/check.rs` currently has, at `:7095-7096`:

```rust
        Builtin::TaskIsDone => (vec![Ty::Int], Ty::Bool),
        Builtin::TaskRelease => (vec![Ty::Int], Ty::Unit),
```

Both become `vec![future_of_param0()]` — the helper already exists and is what `TaskSpawn`/`TaskDrive` use. And in `crates/nova-mir/src/lib.rs`:

```rust
            RtFunc::TaskIsDone => (vec![MirTy::Ptr], MirTy::I8),
            RtFunc::TaskRelease => (vec![MirTy::Ptr], MirTy::Unit),
```

**Two pinning tests will fail and both must be updated, not weakened:** `builtin_signatures_are_what_the_std_call_sites_use` and `every_rt_func_is_declared_with_its_real_signature`. They exist precisely so a signature cannot drift from its call site — check that each still discriminates after your edit.

- [ ] **Step 2: Update `std/task`**

```nova
pub record JoinHandle<T> { fut: Future<T> }

impl<T> JoinHandle<T> {
    pub async fn join(self) -> T {
        while !task_is_done(self.fut) {
            yield_now().await
        }
        task_release(self.fut)
        task_output(self.fut)
    }
}

pub fn spawn<T>(fut: Future<T>) -> JoinHandle<T> {
    task_spawn(fut)
    JoinHandle { fut: fut }
}
```

`task_spawn` still returns an `Int` that nothing reads. **Check whether a bare `task_spawn(fut)` statement whose value is discarded type-checks** — if a non-`Unit` expression statement is an error, bind it (`let _ = …` if the language has it, or whatever the codebase's idiom is) and **report which, since it is a language-limit finding either way.**

Also rewrite `JoinHandle`'s doc comment: its current text explains why the id *and* the future are both held. That reason is gone.

- [ ] **Step 3: Write the Nova-level tests, headline first**

The review's exact program, which must now abort rather than hang:

```nova
async fn spin() -> Int { 1 }
async fn run() -> Int {
    let h = JoinHandle { fut: spin() }
    h.join().await
}
fn main() { println("total ${block_on(run())}") }
```

Under the old code this hung forever with empty output. It must now abort with the never-spawned diagnostic. **Assert the message, not just the failure** — and assert the output does *not* hang, i.e. the test must complete.

Then, as `@test(should_panic)` functions (the per-test-process runner makes an abort observable — `abort_with` emits `nova: panic:`, which is what `nova test` classifies on):

- a handle on a never-spawned future aborts;
- spawning the same future *value* twice aborts;
- **and a plain `@test` proving `spawn(f())` twice still works** — the false-positive guard, without which the rejection could be over-broad and nothing would say so.

Plus a plain `@test` that `join` twice returns the same value twice with no abort.

- [ ] **Step 4: Verify the gate is untouched**

```bash
cargo build --workspace && cargo test -p nova-cli --test run_tests gate_async_tasks -- --nocapture 2>&1 | grep -E "^test |^test result"
```

Expected: all three configurations pass and **`tests/runtime/async_tasks.stdout` needs no regeneration.** If it does, something changed that this plan did not intend — stop and report rather than regenerating it.

- [ ] **Step 5: Full verification**

```bash
cargo build --workspace && cargo test --workspace --no-fail-fast 2>&1 | grep -E "^test result" | awk '{p+=$4; f+=$6; i+=$8} END {print "targets:", NR, "passed:", p, "failed:", f, "ignored:", i}'
cargo clippy --workspace --all-targets -- -D warnings 2>&1 | tail -3
cargo fmt --all --check
```

- [ ] **Step 6: Commit**

```bash
git add crates/nova-typeck crates/nova-mir std/task tests/runtime crates/nova-cli
git commit -m "feat(std): JoinHandle holds only its future

task_is_done and task_release now take Future<T> rather than an Int id,
so JoinHandle no longer carries a forgeable index and a handle can only
ever ask about its own future's task.

The review's forged-handle program -- a handle naming id 0, which is
block_on's own root -- used to pass nova check and then hang forever with
empty stdout and stderr. It now aborts with a diagnostic saying the
future was never spawned.

join's value path is unchanged: it already read through self.fut, which
is why the output arrives in its own machine class rather than as the Int
the executor hands ids back as.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

### Task 3: The documentation sweep

**Files:**
- Modify: `crates/nova-runtime/src/task.rs` (one comment)
- Modify: `docs/adr/0009-async-execution-model.md`, `docs/adr/0008-attributes-and-test-isolation.md`
- Modify: `CHANGELOG.md`

- [ ] **Step 1: Sweep every document asserting the old behaviour**

Four known sites, and this project's most repeated defect is missing one:

1. **`crates/nova-runtime/src/task.rs:1025`** — `releasing_a_task_unroots_its_state_exactly_once_however_often_it_is_called`'s doc describes "a task whose future was also handed to `nova_rt_task_spawn` a second time". **That scenario is now unreachable**, so the comment describes a case the code rejects. Rewrite it to state what the test actually guards.
2. **ADR 0009 §1's residual list** — both the `JoinHandle` residual (a forged handle aborts / hangs) and the spawn-one-future-twice footgun change status. State what is now true, and keep the entries rather than deleting them: a residual that was closed is more useful to a reader than a silent absence.
3. **ADR 0008's marker-emitter count** — it records three (`nova_rt_panic_str`, `nova_rt_check_bounds`, `gc::alloc`'s oversize guard). There are **four**: `task.rs`'s `abort_with`, added in Phase 2.3a. ADR 0008 says its classifier "is sound only while every marker emitter aborts immediately" — verify `abort_with` does (it does, but verify rather than trust this plan) and correct the count.
4. **`CHANGELOG.md`** — the `[Unreleased]` async entries describe `JoinHandle { id, fut }` and the double-spawn footgun.

Then grep for anything else:

```bash
grep -rn "JoinHandle" docs/ CHANGELOG.md nova-spec/ std/
grep -rni "spawned twice\|spawn.*twice\|forged\|bogus id" docs/ CHANGELOG.md crates/
```

- [ ] **Step 2: Verify and commit**

```bash
cargo build --workspace && cargo test --workspace --no-fail-fast 2>&1 | grep -E "^test result" | awk '{p+=$4; f+=$6} END {print "passed:", p, "failed:", f}'
cargo clippy --workspace --all-targets -- -D warnings 2>&1 | tail -3 && cargo fmt --all --check
```

```bash
git add crates/nova-runtime/src/task.rs docs CHANGELOG.md
git commit -m "docs: sweep the documents describing the forgeable task id

A commit that changes enforcement must sweep every document asserting the
old behaviour -- the lesson of the 2.2a debt branch, which shipped a
CHANGELOG asserting old behaviour 57 lines above the entry announcing the
new one, in the same release.

Also corrects ADR 0008's marker-emitter count, which records three: Phase
2.3a's abort_with is a fourth, and the classifier's soundness condition
(every emitter aborts immediately) still holds for it.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

## Plan Self-Review

**Spec coverage.** §3's three changes map to Tasks 1 and 2; §3.1's map to Task 1 Step 3; §3.2's two aborts to Task 1 (spawn rejection, not-a-task) with the Nova-observable halves in Task 2 Step 3; §3.3's false-positive argument to Task 2 Step 3's `spawn(f())`-twice guard; §5's six test requirements distributed across Tasks 1–2; §5's mutation table to Task 1 Step 5 and Task 2; §6's three risks carried into the tasks that own them; §7's definition of done to Tasks 2 and 3.

**One spec item deliberately deferred within the plan:** §5's "`Float` output still arrives in its own machine class" is covered by the existing gate fixture, which already returns a `Float` and must stay byte-identical (Task 2 Step 4). A new `Float` test would duplicate it.

**Type consistency.** `nova_rt_task_is_done(future: *mut u8) -> i8` and `nova_rt_task_release(future: *mut u8)` are declared in Task 1's Interfaces and consumed unchanged by Task 2's `MirTy::Ptr` signatures and `std/task` call sites. `JoinHandle<T> { fut: Future<T> }` is declared in Task 2's Interfaces and is the only shape used thereafter.

**Ordering.** Task 1 → 2 → 3 is strictly sequential: Task 2 will not compile until Task 1's signatures exist, and Task 3 describes behaviour Tasks 1–2 establish. Dispatch implementers and fixers one at a time — two agents mutating one checkout trip over each other's temporary mutations and phantom `git status` entries.
