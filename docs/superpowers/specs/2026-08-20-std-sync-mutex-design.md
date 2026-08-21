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
- **`JoinHandle::cancel(self)`.** Needs a mechanism the runtime does not have: nothing unwinds, and the poll ABI has no interrupt hook, so there is no way to stop a task mid-flight. `cancel` as §13 writes it would require exactly that mechanism, and building it is its own increment. **CORRECTED 2026-08-21 (second fix wave):** this bullet originally opened *"Contradicts a settled contract. ADR 0009 and the `timeout<T>` increment established abandonment, not cancellation"*. **ADR 0009 settles no such thing about `cancel`.** It files "No cancellation" under *Residual gaps* (`docs/adr/0009-async-execution-model.md:365`) and names "a future `JoinHandle` drop or cancellation" as the natural fix point for the task-leak footgun (`:405`) — an open gap, the opposite of a foreclosure — and `nova-spec/13-RUNTIME.md` §4.4 *specifies* structured cancellation, so the project's own spec asks for it. What ADR 0009 settles is narrower and is about `timeout<T>`: because the poll ABI has no cancellation hook, `timeout` **abandons** its inner future rather than cancelling it (`:328-329`). This is a deferral with a named blocker, which is what the other two bullets already are.
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

**CORRECTED 2026-08-20, after Task 1's review checked the citation.** This paragraph originally read: *"ADR 0012 already foreclosed the mechanism it would need. It records that close-on-collect is impossible because the collector's sweep names only the dying object's own GC address, giving no per-object hook."* **ADR 0012 says the opposite.** Its own words: *"**The collector has a per-object notification hook.** `gc.rs`'s sweep calls `crate::task::forget_freed_state(o.addr)` unconditionally for every object it frees"* — and it goes on to note the hook fits the shape a closer would need, and that *"a reader who finds this hook will reasonably ask why `File` does not register with it."* **The ADR anticipated exactly the mistake this spec made, named the hook, and flagged that a reader would find it. The first draft cited that passage for its negation.**

**ADR 0012's real argument, and why it does not transfer here.** The hook reports *the freed object's own address*, never a field value read out of it — so when a `File` record dies, `forget_freed_state` receives that record's address, which names nothing in a handle table keyed by `fd`, an arbitrary integer bearing no relation to any GC address. That is an argument about a **runtime-managed** resource reached through an integer handle. A `MutexGuard`'s release is pure Nova (`self.owner.locked = false`), so RAII here would need the *language* feature, `Drop`, and no runtime hook substitutes for it.

**So the honest reasoning is shorter than the first draft's.** `Drop` is unimplemented; that alone is why there is no RAII. **ADR 0012 is the precedent for the response** — explicit, idempotent release over a collector-based backstop, with a uniform documented leak accepted deliberately — and it is cited here for that and nothing else.

**Therefore release follows `File`'s established pattern**: explicit, idempotent, and forgetting it is a documented consequence rather than a silent one. `std/fs`'s `File` requires an explicit `close` for exactly this reason, and ADR 0012 chose a uniform documented leak over a platform-dependent backstop. A `MutexGuard` that is never released leaves the mutex locked for the rest of the process — the direct analogue of a leaked descriptor, and the same trade already accepted once.

---

## 4. Surface

`std/sync/lib.nova`, complete. No `module` header — no std module has one.

```nova
pub record Mutex<T> { locked: Bool, value: T }
pub record MutexGuard<T> { owner: Mutex<T>, released: Bool }

impl<T> Mutex<T> {
    pub fn new(value: T) -> Mutex<T>
    fn take(mut self) -> Bool                          // private
    pub fn try_lock(mut self) -> Option<MutexGuard<T>>
    pub async fn lock(mut self) -> MutexGuard<T>
}

impl<T> MutexGuard<T> {
    pub fn get(self) -> T
    pub fn set(mut self, v: T)
    pub fn release(mut self)
}
```

**AMENDED 2026-08-21 (fix wave):** `released` and `set` are not in the surface this section originally approved; both were added after review, and the block above is the shipped shape. `released` makes `release`'s documented idempotence true — without it `release` was an unconditional `self.owner.locked = false`, so a second `release` on a guard whose mutex had since been reacquired freed a lock another task held (§6's first row has the detail). `set` exists because `get` returns the value, so for a `T` with no assignable interior — `Int`, `Bool`, `Float`, `Char`, `String` — the guard was read-only while `Mutex<T>`'s generic signature promised otherwise; `std/collections` pairs `Vec::get` (`std/collections/lib.nova:49`) with `Vec::set` (`:53`) for the same reason, and this spec shipped only half that pair. Both are pure Nova: `STD_ONLY` is still 65 and no intrinsic was added.

**A private `take(mut self) -> Bool` sits under both entry points**, and it is not decoration — returning a `Bool` is what lets §5's retry loop read the result straight into its `while` condition (`!self.take()`) rather than inspect and discard an `Option` on every turn. `try_lock` is `if self.take() { Some(…) } else { None }`; `lock` retries on the `Bool`.

**Corrected 2026-08-21.** This paragraph originally justified the `Bool` by asserting *"`Option` has no `unwrap`, only `unwrap_or` (`std/core/lib.nova:30`)"*. **That is false.** `Option::unwrap` is defined at `std/core/lib.nova:26` — four lines *above* the `unwrap_or` this spec cited as evidence for its absence, inside the same `impl<T> Option<T>` block — and std/core calls it on `Option`s itself at lines 255, 280, 297, 354 and 386; `Result::unwrap` is at `:58`. The claim was asserted as a measured fact across four increments and reached `std/sync/lib.nova`'s shipped doc comment as the stated reason `take` returns `Bool`. The `Bool` is still the right choice, for the reason now given above.

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
        MutexGuard { owner: self, released: false }
    }
```

**`while`, not `loop` — because Nova has no `loop`.** The first draft of this section used `loop { match self.try_lock() { … } }` and it does not parse: `loop` is not a keyword, so `loop {` is read as a **record literal** with `loop` as the type name, giving `error[P0001]: expected identifier (in field name), found 'match'`. Measured. `while` and `for` are the only loop forms — eighteen and twelve uses respectively across `std`.

The forced rewrite is better than what it replaced. Retrying on `take`'s `Bool` rather than on `try_lock`'s `Option` means the loop body needs no unwrapping, the guard is constructed once after the loop, and there is no unreachable tail expression to satisfy — all three of which the `loop`-plus-`match` version would have needed.

**This is chosen for what it does not touch.** The alternative — a fourth `Wait` variant so a blocked task genuinely parks — would widen the staging rule a second time and require an arm in `wake_due`, whose `retain` closure ends in `_ => true`. That non-exhaustive match is the hazard the `timeout` increment had to handle by hand. Counted at this baseline: of the **seven** matches on `Wait` in non-test `crates/nova-runtime/src/task.rs`, **four** are exhaustive with no wildcard and would be compiler-forced onto a fourth variant — `try_stage` (`:546`), `earliest_deadline` (`:1040`), `io_parks` (`:1065`) and `deadlock_report` (`:1209`) — while **three** end in `_ => true`: `wake_tasks_waiting_on` (`:771`), `wake_ready` (`:1097`) and `wake_due` (`:1153`). Omitting `wake_due`'s arm produces no compile error, no panic and no diagnostic — only a task parked forever, as its own doc comment at `:1137-1142` says in as many words ("park forever, and fail nothing at compile time"), and as ADR 0009's 2026-08-18 amendment records. Yield-and-retry adds **no runtime surface at all** — no `Wait` variant, no staging change, no wake plumbing, no intrinsic.

**The cost, stated plainly because it is a real regression in diagnosability.** A task waiting on a lock stays **runnable**, not parked. So:

- `report_deadlock` cannot see it. A never-released lock produces a **busy spin through the run queue**, not a deadlock report — where a genuinely parked waiter would be diagnosed.
- Two tasks each holding one lock and awaiting the other spin forever rather than being detected.

That is the honest trade: the version that diagnoses better is the version that touches the executor's most dangerous non-exhaustive match. **If lock contention later becomes common enough that spinning is a performance problem, or a deadlock goes undiagnosed in real use, that is the trigger to revisit** — and the fourth `Wait` variant is the answer at that point, with `wake_due` as the thing to get right.

**Progress is guaranteed against starvation of the holder, not among waiters.** `yield_now` re-queues the caller behind everything currently runnable, so the holder always gets a turn and always can release. Which waiter wins afterwards is unspecified (§1).

---

## 6. Edge cases, each with a stated answer

| Case | Answer |
|---|---|
| `release` called twice | A no-op — and **enforced** as of 2026-08-21 rather than only asserted: `MutexGuard` carries a `released: Bool` and `release` returns early on it. `File::close` was cited here as the precedent, and it is one, but it does **not** supply the precondition that makes it safe: `File`'s idempotence rests on an `fd` being a key into a runtime table where absence *is* closedness (ADR 0012), and a guard has no table and no key — `owner` is a direct reference to the live mutex. Without the flag, a second `release` on a guard whose mutex had since been reacquired cleared `locked` while another task held it. What the flag cannot cover is **tampering, and the root of that is one fact: Nova enforces no field privacy** — `released` is an ordinary readable and writable field, so it closes mistakes rather than attacks. Two measured routes past it, of unequal reach. *Forgery:* any code holding a `Mutex<T>` can write `MutexGuard { owner: m, released: false }` and `release` it without ever acquiring the lock — measured, and it does free a lock a real guard is holding. *Resurrection:* writing `g.released = false` on a guard that was **genuinely obtained and genuinely released** makes its next `release` free a lock a different task now holds — also measured, and its precondition is **strictly weaker**, since it needs nothing but the stale guard, not a `Mutex<T>` to name. **AMENDED 2026-08-21 (second fix wave):** this row named forgery alone and so understated the reach; the resurrection route is the one that matters, because handing a stale guard to another task is exactly what `sync_mutex_stale_guard_cannot_steal` does. Both are unenforceable in this language, exactly as `File { fd: 9999 }` is, with one difference worth stating: a forged `fd` safely misses a table lookup, whereas a forged or resurrected guard writes straight to the live mutex. |
| `set` called after `release` | A no-op — enforced on the same flag, and pinned by its own fixture, `sync_mutex_stale_guard_cannot_write`. **Added 2026-08-21 (second fix wave):** the wave that added `released` guarded `set` with it but tested only `release`'s use — deleting `set`'s early return failed no test in the suite, measured, while the records described the flag as pinned. The gap was in the worse hazard: a stale `release` frees a flag, a stale `set` corrupts the protected value while another task is inside its critical section. |
| Guard never released | The mutex stays locked for the process's life. A documented leak, exactly as ADR 0012 accepted for descriptors. |
| `lock` while already holding it (same task) | **Livelocks by spinning — it never parks, so `report_deadlock` cannot see it.** This project reserves the two words: `crates/nova-runtime/src/task.rs:927-929` calls a task that keeps re-queueing itself a livelock, "not a deadlock, and this loop still cannot see it", citing ADR 0009 §1's 2026-08-10 amendment. `lock`'s `while !take() { yield_now().await }` is exactly that shape. Not re-entrant, and re-entrancy is not added: it would require task identity in the mutex and it hides bugs. |
| `try_lock` on a free mutex | `Some(guard)`, and the mutex is now locked. |
| `get` after `release` | Returns the value regardless, and `get` deliberately does **not** return early on `released` where `release` and `set` do. **CORRECTED 2026-08-21 (second fix wave):** the reason given here was *"the language cannot prevent this"*, and the 2026-08-21 wave that added `released` falsified that premise while leaving it standing — the guard now carries exactly the state needed to detect the case, so the library *can* prevent it and chooses not to. The real reason is that `get` returns a `T` and therefore has no failure channel: an early return has no value to hand back, and `T` is generic so there is no default to invent. `Option<T>` would change the signature every caller unwraps, and a panic would abort the process rather than unwind, since nothing unwinds across a poll boundary. A stale read is also the harmless one of the three — it cannot corrupt the mutex, where a stale `set` or `release` writes to it. Documented, not enforced. |
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

**AMENDED 2026-08-21 (fix wave): six fixtures, not four.** Two were added with the `released`/`set` fixes above: `sync_mutex_stale_guard_cannot_steal` pins that a guard released once, then released again after another task has taken the lock, does not free it — and it is the **only** one of the six that fails when `released` is removed, measured, which is why the defect shipped past the original four. `sync_mutex_int_set_serialises` is the two-task lost-update test over a bare `Mutex<Int>`, which does not compile without `set`; golden `n=2`, and with the mutex stripped out of the fixture it gives `n=1`, measured. Against a never-excluding `take`, four of the six fail and two pass — see ADR 0016's Consequences for the per-fixture breakdown.

**AMENDED 2026-08-21 (second fix wave): seven fixtures, not six.** The wave above guarded `set` on `released` and tested only `release`'s use of the flag: deleting `set`'s `if self.released { return }` failed **zero** of the six, measured across the whole suite, while the records read as though the flag were pinned. `sync_mutex_stale_guard_cannot_write` closes that — a stale guard's `set` landing inside another task's critical section must not reach the protected value. One fixture per use of the flag now, which also **supersedes the note above's uniqueness claim**: "the only one of the six that fails when `released` is removed" was true of the six, and removing the *field* now fails both stale-guard fixtures. Re-measured against a never-excluding `take` with seven fixtures: still **four fail, three pass** — the four are unchanged and `sync_mutex_stale_guard_cannot_write` joins the passing set, because a stale `set` is inert regardless of what `take` does. Deleting `set`'s early return fails **exactly one** test out of 1036, suite-wide: this fixture. Both stale-guard fixtures depend on the executor's **FIFO** ready queue (`crates/nova-runtime/src/task.rs:184`) to place the stale operation inside the holder's critical section; that dependency is stated in each fixture's own comment and in ADR 0016's Consequences, because it is invisible in the fixture bodies and a future edit that adds or removes a `yield_now` would silently defeat it.

**No uniqueness claim appears in that table.** Four such claims were measured false across the last three increments — a count reported as 18 that was 7, one row predicted where five failed, one fixture named where all five caught it. The table says which test *must* fail; if uniqueness matters, run the mutation against the whole suite and count, in a clean tree.

---

## 8. Records

- **CHANGELOG** `[Unreleased]`: Added for the module, the two records and their methods, and `STD_MODULES` 11 → 12 with `STD_ONLY` unchanged at 65.
- **`nova-spec/20-STDLIB.md` §13**: a dated amendment in the file's house style (`**AMENDED <date> (branch \`<branch>\`):**`). **Corrected 2026-08-21:** the line list originally given here — 31, 36, 169, 184, 199, 214 — was inherited verbatim from `2026-08-19-std-fmt-design.md:181` and four of the six numbers do not open a marker in the current file (169 and 184 land mid-paragraph, 199 is blank, 214 is a `---` divider). Regrep rather than trusting a copied list: `grep -n AMENDED nova-spec/20-STDLIB.md` gives the live set. **Re-measured 2026-08-21 (second fix wave), because "currently including 31, 36, 162 and 578" replaced a list presented as exhaustive with a truncated one and so invited the same copy-forward this correction exists to stop.** The bold-prose form `**AMENDED` occurs at **31, 36, 162, 578, 609, 631, 683 and 900** — eight, the complete set — plus seven `// AMENDED` comments inside code blocks (221, 236, 251, 266, 306, 352, 373), fifteen hits in all. Line 900 is this increment's own. It must record what shipped, that release is explicit because `Drop` is unimplemented — **full stop**, per §3 above; ADR 0012 is cited only as precedent for explicit idempotent release, not as a mechanism that forecloses anything — and **why each of the three unbuilt items is unbuilt** — a tuple-returning signature the language cannot express, a thread pool the runtime does not have, and an interrupt hook the poll ABI does not have. **CORRECTED 2026-08-21 (second fix wave):** the third read *"a cancellation model the project already rejected"*. It was not rejected — `nova-spec/13-RUNTIME.md` §4.4 specifies it and ADR 0009 `:365` files it as an open residual gap. See §1's `JoinHandle::cancel` bullet above, corrected the same day. Also that §13's `module std.sync` header line is implemented in **no** std module, which §3's and §10's amendments already record for their own sections.
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
- **`wake_due`'s `retain` closure ends in `_ => true`** — the non-exhaustive match whose omitted arm parks a task forever with no compile error, panic or diagnostic, and the reason §5 avoids adding a `Wait` variant. Three of the seven `Wait` matches in non-test `task.rs` end in a wildcard (`wake_tasks_waiting_on`, `wake_ready`, `wake_due`); the other four are exhaustive and would be compiler-forced. Counted, per §5.
- **No std module has a `module` header**, in any of the twelve `lib.nova` files, despite `nova-spec` writing one for every section.
