# Task identity: key the executor on the future, not a forgeable `Int` — Design

**Status:** approved 2026-08-08. Follow-up 2 from Phase 2.3a's whole-branch review.

**Base:** `main` at `c0d8511` (Phase 2.3a merged and pushed; 807 tests, 8 deliberately ignored).

---

## 1. Why this, and why now

`std/task`'s handle is `pub record JoinHandle<T> { id: Int, fut: Future<T> }`. Nova has no field
privacy, so a user can construct one. The `id` is a `TASKS` vector index and ids are handed out
`0, 1, 2, …` in spawn order, so a handle can be made to name **a different task than its own
future**. That single fact produces both known failures:

- **An unknown id aborts.** `is_done_internal` reaches `abort_with` for an out-of-range index.
  Legible, and correct given the constraint that a panic must not cross a generated poll frame.
- **A valid-but-wrong id hangs, silently.** Measured during the whole-branch review:

  ```nova
  async fn run() -> Int {
      let h = JoinHandle { id: 0, fut: spin() }   // id 0 is `run` itself
      h.join().await
  }
  fn main() { println("total ${block_on(run())}") }
  ```

  `nova check` reports `ok`; `nova run` never terminates, with empty stdout **and** empty stderr.
  `join`'s `while !task_is_done(id) { yield_now().await }` spins forever and the queue never
  empties.

The review put this second on its follow-up list and gave the reason to do it before the next
increment rather than after: **`std/fmt`/`std/io` will model handle-shaped types on `JoinHandle`**,
so the shape propagates if it is not fixed first.

### 1.1 What is achievable, stated precisely

`JoinHandle` will remain **constructible** — that needs field privacy, which the language does not
have, and is not what this design attempts. What it closes is narrower and sufficient:

- a handle can no longer name **a different task** than its own future's, and
- the silent hang is replaced by an abort with a diagnostic.

## 2. Probe table

Measured against `c0d8511`.

| Claim | Measured | Consequence |
|---|---|---|
| `id` is used in more than two places | **False.** Only `is_done_internal` and `release_internal` index by it; `take_output_internal` is unreachable from compiled Nova (its `RtFunc` has no emitter) | The change is contained |
| `join` needs the id for its value | **False.** `task_output(self.fut)` already reads word 1 → the state's output slot | The value path needs no change at all |
| Some fixture or test spawns the same future value twice | **False.** `async_tasks.nova` spawns `counter("a", 3)` and `counter("b", 3)` — distinct futures. Every Rust test spawns each future once | Rejecting double-spawn breaks nothing |
| Abort-based failures need a new test harness | **False.** `nova test` runs one process per test and classifies on the `nova: panic:` marker (ADR 0008). `task.rs`'s `abort_with` emits exactly that prefix and then aborts, so an abort is observable as `should_panic` | The review's "untested because it needs a subprocess" no longer applies |
| ADR 0008's marker-emitter count is current | **False — there are four, not three.** `nova_rt_panic_str`, `nova_rt_check_bounds`, `gc::alloc`'s oversize guard, and `task.rs`'s `abort_with`, which Phase 2.3a added. ADR 0008 says the classifier "is sound only while every marker emitter aborts immediately"; `abort_with` does, so soundness holds | The count in ADR 0008 is stale and should be corrected while this change is already sweeping that ADR's neighbours |
| Nothing documents the double-spawn scenario | **False.** `crates/nova-runtime/src/task.rs:1025` describes "a task whose future was also handed to `nova_rt_task_spawn` a second time" | That scenario becomes unreachable; the comment must be swept in the same commit |

## 3. The change

**`std/task`.** `JoinHandle<T>` becomes `{ fut: Future<T> }` — `id` is removed, not retained as a
vestigial field. `join` becomes `task_is_done(self.fut)` / `task_release(self.fut)`. `spawn` stops
threading a returned id into the record.

**Runtime.** `nova_rt_task_is_done` and `nova_rt_task_release` take the future pointer, read word 1
for the state address, and look the task up in a new thread-local map from state address to task id,
populated at spawn and never removed. Removal is not needed and would be wrong: a released task's
entry must stay so a second `join` still answers, which is what keeps `join` idempotent.

**Builtin and `RtFunc` signatures.** `task_is_done` and `task_release` change from `(Int)` to
`(Future<T>)`. `task_spawn`'s returned id becomes unused by `std/task`; keep the return rather than
changing it to unit, so the executor's own Rust tests keep the handle they address tasks by.

### 3.1 Why a map rather than a scan

`join` busy-waits: one `task_is_done` per turn per joining task. A linear scan over `TASKS` makes a
queue drain O(N²) in the number of tasks, and `std/io` will spawn many. The map is O(1) and its
sync invariant is one insert at spawn with no removal.

### 3.2 Two new failures, both aborts

Both fire from inside `join`, which runs inside a generated poll frame, and **a panic must not cross
that boundary** — generated code has no landing pads (ADR 0009 §1, constraint 4). So both abort, the
same resolution `block_on`'s re-entrancy guard already uses:

- **The state is not a task** — "this future was never spawned". This replaces the hang.
- **The state is already a task** — `spawn` rejects it.

### 3.3 Rejecting double-spawn is forced, and closes a second footgun

Two tasks sharing one state object would make a state-keyed lookup **ambiguous**, so the design has
to decide. Rejecting is the answer that also fixes the documented footgun in which spawning one
future value twice runs its body twice and returns wrong answers (ADR 0009 §1).

**No false positives.** `spawn(f())` twice calls `f()` twice and produces two distinct futures with
distinct states. Only re-spawning the *same future value* — `let fut = f(); spawn(fut); spawn(fut)`
— is rejected, which is exactly the footgun.

## 4. What this does not do

Not attempted, and each has its own reason:

- **Making `JoinHandle` unconstructible.** Needs field privacy.
- **Diagnosing a handle whose future was never spawned at compile time.** Needs to know at
  type-check time whether a value reached `spawn`, which the language cannot express.
- **Removing the leak** in which a spawned task whose output is never taken keeps its state rooted.
  Unchanged, still recorded in ADR 0009 §1.
- **Reserving `Future` as a type name** (follow-up 3) or **owning the park set** (follow-up 4).
  Separate increments.

## 5. Testing

- **The review's exact forged-handle program aborts instead of hanging.** This is the headline; it
  must be the literal program from §1, not a paraphrase.
- **A handle on a never-spawned future aborts**, as a Nova `@test(should_panic)` — the existing
  per-test-process runner makes the abort observable.
- **Double-spawn aborts**, same shape.
- **`join` twice still returns the same value twice** with no abort. This is the regression guard on
  idempotence, which the release-then-read ordering exists to provide.
- **The gate fixture's output is byte-identical** under `nova run`, `nova build`, and
  `NOVA_GC_STRESS=1`. `tests/runtime/async_tasks.stdout` must not need regenerating; if it does,
  something changed that this design did not intend.
- **A `Float` output still arrives in its own machine class.** `mir_ty` collapses five of seven `Ty`
  variants, so an `Int`-only test cannot see a class confusion.

Mutation targets, named here rather than left to review:

| Mutation | Must be killed by |
|---|---|
| Look the task up by `id` again, ignoring the state | the forged-handle test — it would hang again |
| Drop the not-a-task check | the never-spawned test |
| Drop the already-a-task check | the double-spawn test |
| Remove the map entry on release | `join` twice |

## 6. Risks

1. **The signature change ripples.** `task_is_done`/`task_release` are `STD_ONLY` builtins with
   `RtFunc` entries and arms in both backends. `Builtin::ALL` is macro-counted and `lower.rs`'s
   builtin match is deliberately exhaustive, so a missed site fails to compile rather than silently
   misbehaving — the failure mode is loud.
2. **`Future<T>` in a builtin signature is positional.** `Ty::Param(0)` there means "the first type
   parameter of the calling generic function", which is why every `std/task` wrapper declares
   exactly one. A wrong arity fails as `E0010`, not as a miscompile.
3. **The doc sweep.** `task.rs:1025` describes a scenario this makes unreachable, and ADR 0009 §1's
   residual list changes for both the `JoinHandle` residual and the spawn-twice footgun. This
   project's most repeated defect is a comment describing behaviour a change has just altered, and
   ADR 0009 §2 records the rule; a commit that changes enforcement must sweep every document
   asserting the old behaviour.

## 7. Definition of done

- The §1 forged-handle program aborts with a diagnostic naming the cause; it does not hang.
- Double-spawn is rejected; `spawn(f())` twice still works.
- `join` remains idempotent.
- The gate fixture is byte-identical under all three configurations, with no `.stdout` regeneration.
- Suite green at 807 + the new tests, 0 failed; clippy `-D warnings` and `cargo fmt --check` clean.
- ADR 0009 §1's residual list and `task.rs:1025` both updated in the same commit as the behaviour.
