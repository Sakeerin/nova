# `std/sync`'s `Mutex` — design

**Status:** approved 2026-08-20. Base: `main` == `origin/main` == `8a72243`, 496 commits, 0 merge commits, 1029 tests (8 deliberately ignored), clean tree, seven CI checks green, tagged `v0.2.0-alpha.1`.

**Every `file:line` citation below is a hint about where to grep, not a promise.** Citations in this project have drifted twice within a single increment — once because the increment's own edits moved them. Locate by content.

**Goal.** Close the buildable part of Phase 2's position 8: an async mutex that protects an invariant across an `.await`, in pure Nova, with no executor change.

**Approach in one line.** `Mutex<T>` is an ordinary generic record whose `lock` takes the mutex if free and otherwise yields and retries — so the whole thing is Nova code and the runtime is untouched.

---

## 1. Scope

### In

- A new embedded module `std/sync`, `STD_MODULES` **11 → 12**.
- `Mutex<T>` with `new`, `try_lock`, `lock`; `MutexGuard<T>` with `get` and `release`.
- **Zero new intrinsics.** `STD_ONLY` stays at **65**.
- A dated amendment to `nova-spec/20-STDLIB.md` §13, and **ADR 0016**, because three of the four things §13 specifies are not built.

### Out, deliberately, each because it cannot be built as specified

- **`channel<T>(buffer: Int) -> (Sender<T>, Receiver<T>)`.** The signature returns a **tuple**, and tuples do not exist: `error[E0900]: tuple types are not supported yet`, measured at this baseline. A channel is desirable and this is not an argument against it — but its API has to be redesigned around Nova's actual type system (a record holding both ends, or two constructors), and that is a design conversation rather than a transcription. It gets its own increment.
- **`spawn_blocking<T>(f: fn() -> T)`.** Requires a thread pool. `crates/nova-runtime/src/task.rs`'s own first line is *"A single-threaded cooperative executor"*, and the only `std::thread::spawn` in the runtime is a test helper in `poll.rs`. Building this means introducing threads to a runtime that has none, which is a far larger decision than a stdlib function.
- **`JoinHandle::cancel(self)`.** Contradicts a settled contract. ADR 0009 and the `timeout<T>` increment established **abandonment, not cancellation**: an abandoned future is simply never polled again, nothing unwinds, and there is no mechanism to interrupt a task mid-flight. `cancel` as §13 writes it would require exactly that mechanism.
- **Fairness.** Yield-and-retry gives no ordering guarantee: among several waiters, whichever is polled first when the lock frees takes it. Stated rather than implied, because a reader who assumes FIFO will write code that depends on it.
- **Poisoning.** Rust's `Mutex` marks itself poisoned if a holder panics. Nova has no unwinding across a poll boundary — a panic on that path aborts — so there is no state in which a lock is held by a dead task. Nothing to poison.

---

## 2. Why position 8 is well-founded, correcting this project's own framing

When this increment was proposed, the argument against it was that *"the executor is single-threaded, so a `Mutex` has almost nothing to contend with"*. **That is wrong, and §13 already says why:** it declares

```nova
pub async fn lock(self) -> MutexGuard<T>
```

an **async** lock. A thread mutex would indeed be pointless here. An async mutex is not, because a task on a cooperative executor can be **suspended in the middle of a critical section** at any `.await`, and while it is suspended another task runs. Anything whose invariant spans a suspension point needs protecting, and nothing in the language currently offers that.

So "single-threaded" makes a *thread* mutex meaningless and an *async* mutex necessary. The original framing read the executor's threading model and stopped, which is worth recording because the same mistake is available to anyone reading §13 quickly.

---

## 3. No `Drop`, so release is explicit

`MutexGuard<T>` **cannot be RAII**, and this is not a shortcut.

`Drop` is specified in three places — `nova-spec/12-TYPESYSTEM.md:192` ("destructor (custom cleanup, GC interaction)"), `13-RUNTIME.md:96` ("`Drop` trait → finalizer registered at allocation"), `14-CODEGEN.md:24` ("Drop points inserted") — and is **implemented nowhere**: no `Drop` handling in `nova-typeck`, `nova-resolver` or `nova-mir`, and no `trait Drop` in `std/core`. Measured, not assumed.

**ADR 0012 already foreclosed the mechanism it would need.** It records that close-on-collect is impossible because the collector's sweep names only the dying object's own GC address, giving no per-object hook — the same reason a finalizer cannot exist. So this is not "Drop is not built yet"; it is "the runtime has no place to put one".

**Therefore release follows `File`'s established pattern**: explicit, idempotent, and forgetting it is a documented consequence rather than a silent one. `std/fs`'s `File` requires an explicit `close` for exactly this reason, and ADR 0012 chose a uniform documented leak over a platform-dependent backstop. A `MutexGuard` that is never released leaves the mutex locked for the rest of the process — the direct analogue of a leaked descriptor, and the same trade already accepted once.

---

## 4. Surface

`std/sync/lib.nova`, complete. No `module` header — no std module has one.

```nova
pub record Mutex<T> { locked: Bool, value: T }
pub record MutexGuard<T> { owner: Mutex<T> }

impl<T> Mutex<T> {
    pub fn new(value: T) -> Mutex<T>
    fn take(mut self) -> Bool                          // private
    pub fn try_lock(mut self) -> Option<MutexGuard<T>>
    pub async fn lock(mut self) -> MutexGuard<T>
}

impl<T> MutexGuard<T> {
    pub fn get(self) -> T
    pub fn release(mut self)
}
```

**A private `take(mut self) -> Bool` sits under both entry points**, and it is not decoration — it is what lets the retry loop in §5 avoid an `Option` it would then have to unwrap, and `Option` has no `unwrap`, only `unwrap_or` (`std/core/lib.nova:30`). `try_lock` is `if self.take() { Some(…) } else { None }`; `lock` retries on the `Bool`.

**Every holder must bind the mutex `mut`, and callers will see this.** Both `lock` and `try_lock` mutate the receiver, so `let m = Mutex::new(0)` followed by `m.lock().await` is `error[E0060]: 'Mutex_T.lock' mutates its receiver, but 'm' is immutable`, with the compiler suggesting `let mut m`. That is a genuine ergonomic cost of holding the flag in a record field rather than in a runtime table, and it is accepted deliberately: the alternative buys nicer bindings with three new intrinsics and a second table to keep coherent, and this module's whole point is that it needs none.

**The design above was compiled and run before this spec was written**, not sketched: it prints the guarded value, refuses a second lock while held, and admits one after release.

**`lock` and `release` pair on the guard**, rather than `lock` on the mutex and `unlock` also on the mutex, so a critical section has one name and the thing you must release is the thing `lock` handed you.

**Generic records and `mut self` mutators are both established, measured at this baseline.** `Vec<T>`, `Map<K, V>`, `Set<T>` and `VecIter<T>` are all generic records in `std/collections`; `mut self` receivers are ADR 0005 §1's subject with ten uses in that same file. A probe of a generic record with a `mut self` mutator, aliased through a second binding, behaved exactly as this design needs — the mutation was visible through the alias, so two holders of the same `Mutex` contend over one flag rather than over copies.

**`try_lock` returns `Option<MutexGuard<T>>`**, not `Bool`, so a caller that succeeds receives the guard it needs and a caller that fails cannot accidentally proceed as though it holds the lock. `Option` already carries `is_some`, `map` and `unwrap_or` (`std/core/lib.nova:12`, `:18`, `:30`).

---

## 5. Waiting: yield-and-retry, and what it costs

```nova
    pub async fn lock(mut self) -> MutexGuard<T> {
        // Take it if free; otherwise let every other runnable task have a
        // turn and try again.
        while !self.take() {
            yield_now().await
        }
        MutexGuard { owner: self }
    }
```

**`while`, not `loop` — because Nova has no `loop`.** The first draft of this section used `loop { match self.try_lock() { … } }` and it does not parse: `loop` is not a keyword, so `loop {` is read as a **record literal** with `loop` as the type name, giving `error[P0001]: expected identifier (in field name), found 'match'`. Measured. `while` and `for` are the only loop forms — eighteen and twelve uses respectively across `std`.

The forced rewrite is better than what it replaced. Retrying on `take`'s `Bool` rather than on `try_lock`'s `Option` means the loop body needs no unwrapping, the guard is constructed once after the loop, and there is no unreachable tail expression to satisfy — all three of which the `loop`-plus-`match` version would have needed.

**This is chosen for what it does not touch.** The alternative — a fourth `Wait` variant so a blocked task genuinely parks — would widen the staging rule a second time and require an arm in `wake_due`, whose `retain` closure ends in `_ => true`. That non-exhaustive match is precisely the hazard that concealed a reachable process abort for an entire increment: eleven sites were compiler-forced to handle a new `Wait` shape and that one was not. Yield-and-retry adds **no runtime surface at all** — no `Wait` variant, no staging change, no wake plumbing, no intrinsic.

**The cost, stated plainly because it is a real regression in diagnosability.** A task waiting on a lock stays **runnable**, not parked. So:

- `report_deadlock` cannot see it. A never-released lock produces a **busy spin through the run queue**, not a deadlock report — where a genuinely parked waiter would be diagnosed.
- Two tasks each holding one lock and awaiting the other spin forever rather than being detected.

That is the honest trade: the version that diagnoses better is the version that touches the executor's most dangerous non-exhaustive match. **If lock contention later becomes common enough that spinning is a performance problem, or a deadlock goes undiagnosed in real use, that is the trigger to revisit** — and the fourth `Wait` variant is the answer at that point, with `wake_due` as the thing to get right.

**Progress is guaranteed against starvation of the holder, not among waiters.** `yield_now` re-queues the caller behind everything currently runnable, so the holder always gets a turn and always can release. Which waiter wins afterwards is unspecified (§1).

---

## 6. Edge cases, each with a stated answer

| Case | Answer |
|---|---|
| `release` called twice | Idempotent — the second is a no-op, matching `File::close`. |
| Guard never released | The mutex stays locked for the process's life. A documented leak, exactly as ADR 0012 accepted for descriptors. |
| `lock` while already holding it (same task) | **Deadlocks by spinning.** Not re-entrant, and re-entrancy is not added: it would require task identity in the mutex and it hides bugs. |
| `try_lock` on a free mutex | `Some(guard)`, and the mutex is now locked. |
| `get` after `release` | Returns the value regardless — the guard holds a reference to a heap object and the language cannot prevent this. Documented, not enforced. |
| A panic inside a critical section | Aborts the process; no unwinding crosses a poll boundary, so there is no poisoned state to observe (§1). |

---

## 7. Testing

**Nova fixtures**, each needing an explicit `#[test]` in `crates/nova-cli/tests/run_tests.rs` — **registration is not automatic**, and an unregistered fixture runs zero tests while looking green. That has bitten four increments.

| Fixture | Pins |
|---|---|
| `sync_mutex_uncontended` | `new`, `try_lock` succeeding, `get`, `release`, then `try_lock` succeeding again |
| `sync_mutex_try_lock_fails_when_held` | a second `try_lock` returns `None` while the first guard is live |
| `sync_mutex_two_tasks_serialise` | two spawned tasks each `lock`, mutate a shared counter across a `yield_now`, and `release`; the final value proves the critical section held |
| `sync_mutex_release_is_idempotent` | `release` twice, then `try_lock` still succeeds |

**The third fixture is the one that matters**, and it must be written so it *can* fail: the shared counter has to be mutated **across a suspension point**, because a critical section that contains no `.await` would be protected by the executor's cooperative scheduling alone and the test would pass with the mutex removed entirely.

**Mutations to run and report**, each with the test that must fail:

| Mutation | Caught by |
|---|---|
| `try_lock` never sets `locked` | `sync_mutex_try_lock_fails_when_held` |
| `try_lock` returns `Some` unconditionally | `sync_mutex_try_lock_fails_when_held`, and `sync_mutex_two_tasks_serialise` should show interleaving |
| `release` does not clear `locked` | `sync_mutex_uncontended`'s second `try_lock` |
| `lock`'s loop returns without acquiring | `sync_mutex_two_tasks_serialise` |
| the whole `Mutex` removed from the third fixture | that fixture — **and if it still passes, the fixture is wrong**, not the mutex |

**No uniqueness claim appears in that table.** Four such claims were measured false across the last three increments — a count reported as 18 that was 7, one row predicted where five failed, one fixture named where all five caught it. The table says which test *must* fail; if uniqueness matters, run the mutation against the whole suite and count, in a clean tree.

---

## 8. Records

- **CHANGELOG** `[Unreleased]`: Added for the module, the two records and their methods, and `STD_MODULES` 11 → 12 with `STD_ONLY` unchanged at 65.
- **`nova-spec/20-STDLIB.md` §13**: a dated amendment in the file's house style (`**AMENDED <date> (branch \`<branch>\`):**`, as at lines 31, 36, 169, 184, 199, 214). It must record what shipped, that release is explicit because `Drop` is unimplemented and its mechanism foreclosed by ADR 0012, and **why each of the three unbuilt items is unbuilt** — a tuple-returning signature the language cannot express, a thread pool the runtime does not have, and a cancellation model the project already rejected. Also that §13's `module std.sync` header line is implemented in **no** std module, which §3's and §10's amendments already record for their own sections.
- **ADR 0016.** `00-MASTER-SPEC.md` **§7 item 5** requires an ADR for any decision deviating from the spec — **not §0**, which is "Project Identity"; an earlier spec on this project cited §0 and the correction had to be made twice. Verify `0016` is unused with `ls docs/adr/` before creating it; `0001`–`0015` are in use and a previous increment guessed a number that already existed.
  Its subject is the **partial close of position 8**: what shipped, the three deferrals with their reasons, and the yield-and-retry trade including the condition that would justify revisiting it. It should distinguish itself from ADR 0014 (position 2 skipped twice) and ADR 0015 (position 2 closed).

---

## 9. Measured facts this design rests on

Each checked against the tree at `8a72243` rather than recalled:

- **Tuples do not exist**: `fn pair() -> (Int, Int)` gives `error[E0900]: tuple types are not supported yet`. This is what makes §13's `channel` unbuildable as written.
- **The executor is single-threaded**, stated in `crates/nova-runtime/src/task.rs`'s first line; the only `std::thread::spawn` in the runtime is a test helper in `poll.rs`.
- **`Drop` is specified and unimplemented** — three spec files describe it, and there is no handling in `nova-typeck`, `nova-resolver` or `nova-mir` and no `trait Drop` in `std/core`.
- **Records are reference types.** A `mut self` mutator's effect is visible through a second binding to the same record — probed directly, and it is what makes a shared `Mutex` contend over one flag.
- **Field assignment requires a `mut` binding**: an immutable parameter gives `error[E0060]: cannot assign to a field of immutable c`.
- **`E0060` also fires at the *call site* of a receiver-mutating method**, which is the form this design meets: `let m = Mutex::new(0)` then `m.lock().await` gives `error[E0060]: 'Mutex_T.lock' mutates its receiver, but 'm' is immutable`, with the compiler suggesting `let mut m`. Two distinct manifestations of one rule, and the call-site one is the user-visible cost recorded in §4.
- **There is no `loop` keyword.** `loop {` parses as a **record literal** with `loop` as the type name — `error[P0001]: expected identifier (in field name), found 'match'` for `loop { match … }`. `while` and `for` are the only loop forms, with eighteen and twelve uses respectively across `std`. This invalidated §5's first draft.
- **The design in §4 and §5 was compiled and run before this spec was finalised**, printing the guarded value, refusing a second lock while held, and admitting one after release. Every signature here is transcribed from something that executed, not sketched.
- **Generic records are established**: `Vec<T>`, `VecIter<T>`, `Map<K, V>`, `Set<T>` in `std/collections`, and `MapIter<I: Iterator, U>` in `std/core`.
- **`mut self` receivers are established**: ADR 0005 §1's subject, ten uses in `std/collections/lib.nova`.
- **`impl<T>` on a generic record works**, probed together with the above.
- **`Option<T>` carries `is_some`, `map`, `unwrap_or`** (`std/core/lib.nova:12`, `:18`, `:30`), so `try_lock`'s return type needs nothing new.
- **`yield_now` is an existing glob-imported `pub async fn`**, so the retry loop needs no new surface.
- **`wake_due`'s `retain` closure ends in `_ => true`** — the non-exhaustive match that concealed a reachable abort for an entire increment, and the reason §5 avoids adding a `Wait` variant.
- **No std module has a `module` header**, in any of the twelve `lib.nova` files, despite `nova-spec` writing one for every section.
