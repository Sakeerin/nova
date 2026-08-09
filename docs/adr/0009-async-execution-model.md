# ADR 0009 — Async execution model, and the documentation rule the branch that built it needed

Two decisions taken to ship `async fn`, `.await`, `Future<T>` and `std/task`
(Phase 2.3a, branch `async-core`; the
`.superpowers/sdd/2026-08-07-phase-2-3a-async-core/` plans).

They share a file because the second is what the first cost. Section 1 is the
execution model: single-threaded cooperative state machines, and why the
alternative this project's own plan recommended is unsound *here* specifically.
Section 2 is a documentation rule — a module's doc must not assert its caller's
policy — which is recorded as a decision rather than a note because the branch
shipped ten documentation-accuracy defects of one shape, and this rule is the
only mitigation that demonstrably worked on it.

Both are accepted, and each names a real loss rather than arguing it away.

**Numbering:** `docs/adr/0010-conservative-scan-root-test-gating.md` was written
first and deliberately took 0010 rather than claiming the number this document
was already reserved for (it says so in its own numbering note). 0009 was never
used and never deleted — this document fills it.

---

## 1. Async is single-threaded cooperative state machines, and this reverses `docs/phase-2-plan.md` decision 1

### Status

Accepted (2026-08-08). Phase 2.3a, branch `async-core`: the MIR state-machine
transform (`crates/nova-mir/src/async_lower.rs`), the executor
(`crates/nova-runtime/src/task.rs`), and `std/task`.

**Reverses `docs/phase-2-plan.md` decision 1**, which recommended option (b),
"stackful coroutines / thread-per-task over Tokio", as the pragmatic choice for
Phase 2.0 and deferred option (a), full state-machine lowering, to "Phase 2.x
once collections/net are proven". That plan text is edited in place alongside
this document rather than left standing as current advice.

### Context

`nova-spec/13-RUNTIME.md` specifies two things about async, and they are
separable:

- **§4.2 Future Type** — "A Nova `Future<T>` compiles to a state machine struct
  (like Rust's async)", polled to `Pending`/`Ready`.
- **§4.1 Executor** — wrap Tokio, with a work-stealing thread pool sized to CPU
  count.

`docs/phase-2-plan.md` decision 1 judged the state-machine half too large to
build early and recommended thread-per-task instead: each task gets an OS
thread, `await` blocks a runtime worker, and the compiler needs almost no new
machinery. Read on its own that is a fair trade — it moves work out of the
compiler, which is where the risk in a young language usually is.

### Decision

**An `async fn` lowers to a resumable state machine, and every task on a thread
is driven by one cooperative single-threaded executor.** No Tokio, no thread
pool, no work stealing. So §4.2 is honoured as written, and **§4.1 is a
deliberate deviation** recorded here.

Concretely: the transform gives each `async fn` a heap state object (resume tag,
output slot, then one slot per temp) and a poll function whose every temp access
is a load or store against that object; a call to an `async fn` builds the fat
pointer `{ poll_code, state }`; `.await` splits its block, stores a resume tag,
and returns `POLL_PENDING`; the entry block becomes a `Switch` on the tag.
`std/task` exposes `spawn`, `JoinHandle`, `join`, `yield_now` and `block_on` over
the executor, and `async fn main` is driven by `block_on`.

### Why thread-per-task is not the cheaper option in this codebase

This is the load-bearing part of the decision, and it is a property of Nova's
collector rather than a general argument about async.

**The GC heap is thread-local.** `crates/nova-runtime/src/gc.rs` holds the entire
heap in a `thread_local!` — `HEAP`, a `RefCell<Heap>` with the object map inside
it — `alloc` routes every allocation through `HEAP.with`, and `collect()` walks
only the calling thread's own stack, from its own `stack_base()`. The root
registry `PINNED` (`gc::add_root` / `gc::remove_root`) is thread-local for the
same reason and says so in its own contract.

So an object allocated on task A's thread lives in A's heap map and **only A's
collector can see or free it**. Hand that object to task B — through a channel,
a shared handle, anything — and A's next collection frees it while B holds a
pointer to it. B's collector cannot save it either: the object was never in B's
map, so B has nothing to mark. That is a use-after-free, in the one subsystem
this project has been most deliberate about, reachable from ordinary source with
no diagnostic anywhere.

Making thread-per-task **sound** therefore is not "almost no compiler work". It
needs a global heap behind a lock, stop-the-world coordination across every live
thread, stack scanning for all of them, and safepoints where a thread can be
stopped in a scannable state. That is a larger and considerably subtler body of
work than the lowering it was supposed to avoid — and it moves the risk from the
compiler into the collector, where this project's tooling is weakest: mutation
testing, `NOVA_GC_STRESS` and adversarial review all bite hardest on the
compiler, and the collector's failures are the silent kind.

Single-threaded state machines leave the thread-local invariant **completely
untouched**. The only collector change this slice needed is the additive
`PINNED` registry, and a suspended task's state object has a real root rather
than an accidental one.

### What is given up

- **Real parallelism.** Nova gets concurrency, not multi-core execution. Tasks
  interleave at suspension points on one thread.
- **`spawn_blocking` cannot be honoured** (`nova-spec/20-STDLIB.md` §13's
  `spawn_blocking<T>(f: fn() -> T) -> JoinHandle<T>`). Its whole purpose is to
  move a blocking call off the async thread onto another one, and there is no
  other thread. It is not provided, rather than provided as a synonym for
  `spawn` that would silently block the executor.
- **The spec's §4.1 executor.** Revisiting either of the two above means
  revisiting the collector first. That ordering is the point of this decision,
  not an accident of it.

### Residual gaps, stated plainly rather than left to be inferred

Every item here was found and disclosed during implementation, not discovered
afterwards.

- **No parking and no waking.** There are no wakers and no driver thread. A task
  that returns `POLL_PENDING` is pushed back onto the ready queue and re-polled
  on the next turn, round-robin. A task waiting on something therefore *spins* at
  one poll per turn rather than sleeping — `JoinHandle::join` is literally a
  `while !task_is_done(self.fut) { yield_now().await }` loop.
- **`block_on` drains the whole queue, and does not terminate if any queued task
  never reports ready.** Two consequences follow from the same loop. It
  **implicitly joins everything** spawned on the thread, unlike tokio's
  `block_on`, which returns as soon as its own future resolves — here nothing
  else would ever advance a task left pending, so returning early would strand
  it. And a task that reports `POLL_PENDING` forever is re-queued forever, so the
  loop **hangs**. Every *suspension* in 2.3a is `yield_now`-shaped and resumes on
  the next turn, so no ordinary async program reaches the hang that way — **but
  a second route to it, through a forged `JoinHandle`, was reachable and is now
  closed** (branch `task-identity`). `JoinHandle` had an `id` field next to
  `fut`, and both were public — Nova has no field privacy — so a hand-built
  value could set `id` to anything. Task ids are `0, 1, 2, …` in spawn order and
  `run_to_completion` spawns the `block_on` root *first*, so
  `JoinHandle { id: 0, fut: … }` — **`id` is no longer a field of
  `JoinHandle<T>` (`3bbe2d7`); this shape does not compile and is kept here
  only as the record of the hazard** — written inside that root named the root
  itself. `join`'s `while !task_is_done(id) { yield_now().await }` then waited
  for the task that was doing the waiting: a valid id, so nothing aborted, and
  it never became done. Measured on the pre-fix branch, 2026-08-09: `nova check`
  reported `ok` and `nova run` hung indefinitely with empty stdout and stderr
  (`timeout 8` → exit 124). The earlier disclosure of this shape said only that
  a hand-built handle with a *bogus* id aborts, which was true of an id no task
  ever had, and was the wrong half of the hazard: a valid-but-wrong id did not
  abort, it deadlocked. **What closed it:** `nova_rt_task_is_done` and
  `nova_rt_task_release` (`7bbd78d`) now take the future itself rather than an
  `Int`, read word 1 for its state address, and resolve the task through the
  executor's `BY_STATE` map keyed on that address
  (`crates/nova-runtime/src/task.rs`) — there is no id space left to forge,
  since the only way to obtain a state address is to call an `async fn`. The
  one case still reachable, a handle on a future that was never spawned at all,
  now aborts (`nova_rt_task_is_done: this future was never spawned, so there is
  no task to ask about`) rather than hanging, pinned end to end by
  `forged_join_handle_aborts_instead_of_hanging`
  (`crates/nova-cli/tests/run_tests.rs`, `tests/runtime/forged_join_handle.nova`).
  The park-set gap itself is unrelated and unchanged: it is still owed by
  whatever adds the first primitive that can park on an external event (an
  `await` on a channel nothing sends to) — a park set plus a deadlock diagnostic
  when the ready queue is empty and the park set is not. A busy re-poll loop
  cannot tell "not ready yet" from "never will be".
- **`spawn` used to accept the same future twice, with no check at all** —
  registering it as two independent tasks that both drove one shared state
  object, corrupting whichever one polled second. **Closed on branch
  `task-identity` (`51b68b4`), narrower than the ideal fix.** `spawn_internal`
  now checks `Task::taken` and aborts (`nova_rt_task_spawn: this future is
  already a live task; spawn it again only after its task has been released`)
  when the future's state already names a task that has not been released.
  This is a *liveness* check rather than a presence check, deliberately: the
  executor's `BY_STATE` map never removes an entry, and the collector genuinely
  frees a released, unreachable state and can hand its address to a later,
  unrelated allocation, so rejecting on bare presence would abort a spawn that
  did nothing wrong. The liveness check closes the case above, but **reopens a
  narrower one**: `Task::taken` distinguishes *released* from *live*, not
  *dead* from *alive*, so a released future a caller still holds — Nova has no
  move checking to stop that — now passes the check too. `let h = spawn(f());
  h.join().await; spawn(h.fut)` is therefore legal, and the second `spawn`
  re-polls the same completed state machine from its last suspend point. This
  is accepted, in the same family as this list's own "awaiting the same future
  twice" footgun below: both trade a rare, contrived re-poll of a finished
  future for removing a non-deterministic abort of ordinary code, and closing
  this narrower case would need the executor to distinguish "released but
  still held" from "freed and recycled", which needs to know whether the
  collector has actually freed the object — a sweep-integration change well
  beyond this fix.
- **No cancellation.** `nova-spec/13-RUNTIME.md` §4.4 specifies structured
  cancellation — dropping a handle cancels the child, plus an explicit
  `task.cancel()`. None of it exists. A `JoinHandle` is a plain value-semantics
  record; dropping it does nothing.
- **Async trait methods are still rejected, but by two different checks.** In a
  trait *declaration* and in a *default body* it is `E0900` ("async methods"),
  as it is for async `extern` functions ("async extern functions"). In an
  **impl** it is `E0072`: the `E0900` check is not applied there, and what
  refuses it is trait conformance — an `async fn` returns `Future<T>` while the
  trait declared `T`, rendered as ``method `m` returns `Future<Int>` but trait
  `T` declares `Int` ``. Both positions are refused and both diagnostics are
  legible, so this is one gap and not two, but the codes differ and only the
  declaration's is `E0900`
  (`async_trait_method_still_reports_e0900` and
  `async_trait_impl_method_is_refused_as_a_return_type_mismatch`, `nova-typeck`).
  Async *inherent* methods are accepted. Async closures are not in the grammar
  at all.
- **Every temp is spilled into the state object, not only those live across a
  suspend.** This is what buys the transform out of liveness analysis: no local
  lives in a register or on the stack across a suspension *by construction*. The
  cost is over-retention — a temp dead at every suspend point still occupies a
  slot and still roots whatever it points at. The collector is already
  conservative and over-retains by design, so this adds no new failure mode, only
  more of an accepted one; a liveness pass later narrows the state record and
  changes no observable semantics.
- **Awaiting the same future twice re-polls a completed future.** `let fut = g()`
  then `fut.await + fut.await` compiles, and re-enters `g`'s poll function at its
  stored resume tag, re-running the body's statements after its last suspend — so
  side effects after the final await run twice. Rust forbids this by move
  checking; **Nova has no move checking**, which is also why `JoinHandle::join`
  caches its value instead of consuming the handle (a `fn take(self)` can be
  called twice here). Measured on this branch, 2026-08-08: for `async fn g() ->
  Int { 1 }`, that expression yields `2`. Not unsoundness and not a miscompile,
  but this slice is what made the shape reachable.
- **A spawned task whose output is never taken leaks its state object.** The
  executor's `PINNED` root is released by `take_output` or by `task_release`,
  whichever a caller reaches; a task nobody ever joins keeps its state rooted for
  the rest of the process. This is the deliberate half of a trade: releasing the
  root at *completion* instead would free a heap-valued output while the task's
  own record still names it. A leak, not unsoundness. The natural fix point is a
  future `JoinHandle` drop or cancellation, i.e. the gap above.
- **Each `yield_now()` costs four allocations** — `yield_now` is itself an `async
  fn` wrapping a builtin, so a state object and a fat pointer for each of the two
  layers. Nothing caches or pools them.
- **`poll_one`'s compiler-bug panic can still cross a generated poll frame.**
  This is the one residual hole in the constraint the whole transform rests on:
  no unwind may cross a poll function's boundary, because a Cranelift- or
  LLVM-emitted frame has no landing pads, no drop glue and no unwind
  description. Every other reachable diagnostic was moved to `abort_with` for
  exactly this reason — including `block_on`'s re-entrancy check, which
  `std/task` made reachable from ordinary Nova source. `poll_one`'s
  out-of-range-status check is deliberately still a `panic!`, because its
  observability is what a test asserts on and its precondition is already a
  broken compiler. Honestly disclosed rather than closed.
- **The eight tests that assert on a real `gc::collect()`'s outcome are
  `#[ignore]`d**, including the two that pin the executor registering and
  releasing a task's root through the registry. That gating is a separate
  decision with its own evidence: **ADR 0010**. It is not about the registry —
  the failure direction is over-retention, from a stale stack word in an
  already-returned frame — but it does mean this model's rooting behaviour under
  a real collection has **no ungated coverage at all**. What runs in an ordinary
  test step is `a_completed_tasks_state_stays_rooted_until_its_output_is_taken`,
  which asserts on `gc::root_count` and never collects, so it covers the registry
  being *populated*, not honoured. The end-to-end
  `gate_async_tasks_under_gc_stress` configuration does not close that: it
  collects on every allocation and proves no premature free in a live async
  chain, but every state object in its fixture is also reachable from the stack
  independently of the registry — `nova_rt_task_block_on` holds the root future's
  fat pointer as a live parameter for the whole drive, and the rest of the chain
  hangs off it — so removing the registry would not change its result. Measured
  on this branch, 2026-08-09: with `gc::add_root` neutered to a no-op, all three
  `gate_async_tasks_*` tests still pass while the four `root_count` unit tests
  fail. That corrects an earlier statement of this residual, which listed the
  gc-stress configuration as part of the coverage; `run_tests.rs`'s comment on
  that test carries the same correction.

### Consequences

- **Concurrency is cooperative and visible.** A task that never awaits never
  yields the thread. There is no preemption, so a long computation inside one
  task starves every other task on that thread, and the fix is always an
  `.await`.
- **`spawn` starts nothing.** A task is queued, and the queue is drained only
  while a `block_on` call is running — so a `spawn` with no `block_on` anywhere
  above it never runs at all, silently. `async fn main` is driven by `block_on`,
  which is what makes the ordinary case work.
- **The poll ABI is a frozen contract that generated code and the runtime both
  reproduce from independent declarations.** `PollFn`, `POLL_PENDING`,
  `POLL_READY`, the slot indices and `STATE_MIN_SIZE` are declared in
  `nova-runtime` and mirrored in `nova-mir`; `nova-codegen-cranelift` depends on
  both crates and is where the cross-crate equality test lives, because a test
  pinning one crate's copy against literals would let the two drift
  together-but-wrong.
- **A state object must be allocated scanned, and never smaller than
  `STATE_MIN_SIZE`.** The output slot is read unconditionally on completion, even
  for a unit-returning `async fn` with no temps; and an unscanned state object
  would be marked but never traced, freeing a heap-valued output with its header
  still alive.
- **A generated poll function must return exactly `POLL_PENDING` or
  `POLL_READY`.** Any other value is a diagnostic, not a completion, so a codegen
  bug cannot become a wrong answer.
- **Nothing in this model is thread-safe, and none of it pretends to be.** The
  queue, the task table, the re-entrancy flag and the root registry are all
  thread-local, deliberately and for the collector's reason.

### Alternatives considered

- **Thread-per-task over Tokio** — `docs/phase-2-plan.md` decision 1(b), the
  recommendation this document reverses. Rejected on the thread-local-heap
  argument above: it is cheaper only if the collector is ignored, and it puts the
  risk in the collector.
- **A global locked heap with stop-the-world collection first, then
  thread-per-task.** Not rejected on the merits — it is the honest prerequisite
  for real parallelism, and it is what "revisit the collector first" above means.
  Rejected as *this* slice's work: it is a larger project than the async feature
  it would be enabling, and sequencing it after a working single-threaded model
  means the collector change can be judged on its own evidence.
- **Liveness-based state minimization**, spilling only temps live across a
  suspend. Deferred rather than rejected. Its bugs are the silent kind — a local
  wrongly judged dead is a use-after-free with no diagnostic — and it is a pure
  optimization that can land later on its own evidence without changing
  semantics.
- **A synchronous shim: make `async fn` ordinary functions and `.await` a no-op**,
  which would let `std/io` and `std/fmt` proceed on their spec signatures
  immediately. Rejected: it would make every `async` annotation in the standard
  library a lie, and there would be no point at which the lie got found, since
  every program would behave correctly right up until something actually needed
  to suspend.

---

## 2. A module's doc must not assert its caller's policy

### Status

Accepted (2026-08-08), during Task 4's second fix round, as the structural
response to a defect class that ordinary corrections had not stopped. Recorded
here because it currently lives nowhere durable — it is enforced only inside the
text of the one function it was first applied to, so the next person writing a
runtime primitive would not meet it.

### Context

This branch shipped a long run of documentation-accuracy defects, and they were
all one shape: **a comment narrating something owned somewhere else, which then
drifted.** Not sloppiness in the usual sense — in most instances the author had
reasoned correctly and then written a comment asserting more than they had
traced, or restated a neighbouring module's behaviour accurately at the time and
left it to rot.

The branch ledger's running tally reached **ten** instances by the round that
fixed the fourth batch of them (2026-08-08). That figure is a prose tally and
should be read as one; three specific instances are checkable and are what the
rule actually rests on:

1. **A collector primitive describing what the executor does.** `gc::add_root`'s
   own contract doc said the call "pairs with exactly one `remove_root` on that
   task's completion". The executor's rooting lifetime was changed in the same
   review round — the root is now released when a task's *output is taken*, not
   at completion — and the collector-side statement of that invariant went stale
   in the same commit. The worst possible place for it to be wrong, and it
   happened because the fix's own report treated "no change to `add_root`" as a
   virtue.
2. **The same falsified claim in two documents at once.** A CI comment and ADR
   0010 both asserted that eight `#[ignore]`d tests were the only coverage of
   `gc::add_root` being wired into the executor's spawn path. The very commit
   carrying that sentence added an ungated test that kills exactly that mutation
   on all three CI legs.
3. **A doc that contradicted its own inline comment three lines below it**, plus
   a cross-reference to a discussion that had never existed.

### Decision

**A module's documentation states its own contract and says nothing about who
calls it, or when.** Cross-module invariants live in exactly one place and are
referenced from the other, with the reference pointing in the direction that
cannot rot.

So `gc::add_root` now states multiset semantics, same-thread-only, and
**explicitly that pairing is the caller's duty and deliberately unspecified
there** — the last clause included so the removed text cannot creep back as a
missing detail. `remove_root` gained the contract half that had only been
inferable from code: over-removal is a no-op, and *which* duplicate a removal
cancels is unobservable, so the `rposition` + `swap_remove` implementation is
implementation rather than contract. Neither carries a back-reference to
`task.rs`.

Two companion rules, from the same evidence:

- **Invariant in the comment, measurement in the document.** A comment saying
  "measured: tests X and Y fail without this" drifts every time the test set
  changes. An ADR is the right home for a dated measurement; a source comment is
  not. (This project had already learned this once, when a `STD_ONLY` doc comment
  went stale across three consecutive review rounds.)
- **Saying what a feature is *for* survives the rule; saying when a caller uses
  it does not.** `gc.rs`'s module doc and `PINNED`'s field doc still name
  suspended async task state as the motivating case, with no verb tied to a call
  site or a moment. Purpose does not rot; sequence does.

### Consequences

- **The rule paid for itself in the round that adopted it.** Applying it, rather
  than patching only the four named sites, caught **two further instances the
  previous commit had itself introduced**: `gc::root_count`'s doc restated the
  executor's rooting policy verbatim, and a test comment justified a guard via
  "Task 4's executor". Both were added by the commit that fixed the class
  elsewhere.
- **A reader of a runtime primitive learns less about the system from that one
  file**, on purpose. Finding out when `add_root` is called now means reading
  `task.rs`. That is the trade: one authoritative place, at the cost of local
  convenience.
- **Reviews on this branch adopted a mechanical detector for the class** — scan
  added comment lines for narrated measurements. It began as a digit scan and
  **under-detected three times**, every time on a quantity written as words
  ("passes every other test in the workspace", "more than once", "both mistakes
  are pinned here"). Scan for measurement *phrases* and worded quantities, not
  for digits.

### Alternatives considered

- **Keep correcting instances as reviews find them.** This is what the branch did
  for its first several rounds, and it is why this is an ADR: three consecutive
  rounds each fixed an instance and introduced a fresh one, and one fix round
  introduced four.
- **Ban cross-module references in comments entirely.** Rejected as too strong: a
  reference is exactly how the one authoritative statement gets reached from the
  other side. The defect is *restating* the invariant, not pointing at it.
- **Rely on a lint or a doc test.** Nothing available checks whether English
  prose about another module is still true. The mechanical detector above is a
  review aid, not an enforcement mechanism, and is recorded as such.

---

## References

- Plan and ledger: `.superpowers/sdd/2026-08-07-phase-2-3a-async-core/`
- Design: `docs/superpowers/specs/2026-08-07-phase-2-3a-async-core-design.md`
  (§2 and §2.1 argue the reversal; §4.1 the all-temps trade)
- `docs/phase-2-plan.md` decision 1 — edited in place to record that option (b)
  was assessed and chosen against, and to point here
- `crates/nova-runtime/src/gc.rs`: `HEAP`, `PINNED`, `alloc`, `collect`,
  `stack_base`, `add_root`, `remove_root`, `root_count` (§1's thread-local
  argument; §2's worked example)
- `crates/nova-runtime/src/task.rs`: `PollFn`, `POLL_PENDING`, `POLL_READY`,
  `STATE_SLOT_*`, `STATE_MIN_SIZE`, `poll_one`, `run_to_completion`,
  `spawn_internal`, `take_output_internal`, `nova_rt_task_block_on`,
  `nova_rt_task_yield_future`
- `crates/nova-mir/src/async_lower.rs`: the state-machine transform, the
  `Spiller`, and the state-size guard
- `std/task/lib.nova`: `spawn`, `JoinHandle`, `join`, `yield_now`, `block_on`
- Gate: `tests/runtime/async_tasks.{nova,stdout}`, registered as
  `gate_async_tasks_run`, `gate_async_tasks_build_standalone` and
  `gate_async_tasks_under_gc_stress` (`crates/nova-cli/tests/run_tests.rs`)
- Spec: `nova-spec/13-RUNTIME.md` §4.2 (honoured), §4.1 and §4.4 (deviations
  recorded in §1); `nova-spec/20-STDLIB.md` §13 (`spawn_blocking`, not provided)
- `docs/adr/0010-conservative-scan-root-test-gating.md` — the `#[ignore]`d
  collection tests, including the two that cover the executor's rooting
- `docs/adr/0002-phase1-leaking-allocator.md` — why the collector is
  conservative, and why there is no unwinding to catch a panic with
