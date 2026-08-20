# `std/sync`'s `Mutex` Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Give Nova an async mutex that protects an invariant across an `.await`, closing the buildable part of Phase 2's position 8.

**Architecture:** `Mutex<T>` is an ordinary generic record holding a `Bool` and the value. `lock` takes the flag if free and otherwise yields and retries, so the whole module is Nova code: no intrinsic, no runtime change, no compiler seam. Release is explicit because `Drop` does not exist, following `File::close`'s established pattern.

**Tech Stack:** Nova only (`std/sync`), plus `assert_cmd` fixtures registered in `crates/nova-cli/tests/run_tests.rs`. **No Rust is added by this increment.**

**Spec:** `docs/superpowers/specs/2026-08-20-std-sync-mutex-design.md`

## Global Constraints

- `cargo build --locked --workspace` **before** `cargo test --locked --workspace --all-features --no-fail-fast`. Baseline: **1029 passed / 0 failed / 8 ignored across 44 targets.**
- Sum **every** `test result:` line; never pipe cargo output through `head`/`tail` first. **Filter out lines containing `trapped`** — that is Nova's own harness schema echoed inside a *failing* fixture's captured stdout, and counting it inflates the target count from 44 to 46.
- Clippy `--all-targets --all-features -- -D warnings` clean, on **both** ubuntu and windows. MSRV **1.78**: **no `reason = "…"` in any lint attribute.**
- `cargo fmt --all -- --check` clean.
- The ADR-0010 GC tests stay ignored and untouched: **8 live `#[ignore]` attributes**, 17 textual hits. Both counts must stay put, and **cross-check any load-bearing count with a second method** — a plain recursive `grep` has intermittently under-counted this pattern.
- **CRLF** on `std/**/*.nova`, the fixtures, and markdown under `docs/`/`nova-spec/`. Check `tr -cd '\r' < f | wc -c` against `wc -l < f` after any formatting run and repair by raw byte copy. Three traps: **`cargo fmt` and `sed -i` both rewrite line endings**; **the Write tool does not turn a literal `\r` into a carriage-return byte** (a previous increment committed then fixed a file for exactly this); and **`grep` here strips CR from its output**, so `cat -A` shows no `^M` even when the bytes are present — **`od -c` or a byte count is authoritative**. **Never `git checkout --`.**
- **`core.autocrlf=true` with no `.gitattributes`**, so `git show HEAD:<path> | sha256sum` legitimately differs from `sha256sum <path>`. Verify restores with **`git diff`**, never by hashing.
- **Prefer the Edit tool for byte-exact edits** over shell-embedded scripting — a Python heredoc mangled `\r\n` escapes on a previous increment.
- Every fixture path unique per process.
- Commit messages: write to a UTF-8 file and apply with `git commit -F`, **never a heredoc**. Every body ends with exactly `Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>`.
- **Cite no SHA** that is not already an ancestor of `main` (`8a72243` and `0fe3d94` are). A commit last increment had to be amended for citing branch-local SHAs.
- **Do not push, merge or tag.** Linear history, zero merge commits.
- A pre-existing intermittent failure has seven known names, all of which build a subprocess and read its output: `nova_test_runs_an_async_body_via_block_on_and_pins_a_wrong_answer`, `spawning_two_distinct_futures_from_the_same_call_site_still_works`, `closures_build_standalone`, `method_generics_build_standalone`, `joining_a_handle_twice_returns_the_same_value_twice_with_no_abort`, `a_filter_selects_a_strict_subset_and_the_others_do_not_run`, and separately `net::tests::connecting_to_a_closed_port_is_connection_refused`. Signature: `nova-test-bin\<hash>\main.exe's inventory did not start with a test count (exit code: 0xc0000005): stdout "", stderr ""`. **Zero CI sightings ever** across four increments; all pass in isolation. Name it, confirm the isolation pass, re-run, move on. **A failing `sync_mutex_*` name is NOT the flake and must be examined.**

---

## File Structure

| File | Responsibility | Task |
|---|---|---|
| `std/sync/lib.nova` | **new** — `Mutex<T>`, `MutexGuard<T>`, and nothing else | 1 |
| `crates/nova-resolver/src/lib.rs` | one `STD_MODULES` entry, 11 → 12 | 1 |
| `tests/runtime/sync_mutex_uncontended.{nova,stdout}` | the single-task lifecycle | 1 |
| `tests/runtime/sync_mutex_try_lock_fails_when_held.{nova,stdout}` | contention without suspension | 1 |
| `tests/runtime/sync_mutex_release_is_idempotent.{nova,stdout}` | double release | 1 |
| `tests/runtime/sync_mutex_two_tasks_serialise.{nova,stdout}` | **the discriminating test** | 2 |
| `crates/nova-cli/tests/run_tests.rs` | four registrations — **not automatic** | 1, 2 |
| `CHANGELOG.md`, `nova-spec/20-STDLIB.md`, `docs/adr/0016-*` | the records | 3 |

**No compiler seam and no intrinsic.** Unlike the last three increments, nothing here passes through `nova-mir`, `nova-typeck` or `nova-runtime`. `STD_ONLY` stays at **65** and `RESERVED_TYPE_NAMES` at **7**. The only non-Nova edit in the whole increment is a single line added to `STD_MODULES`.

**`STD_MODULES` order does not matter** beyond `$std.core` staying first. That list is neither a dependency order nor a visibility order — method resolution is order-independent (`collect_impls` builds from a global table) and the glob import is omnidirectional, as the constant's own doc comment says: *"Order is significant only in that it fixes module indices."* A previous increment asserted an ordering constraint here and had it disproved by measurement. Put `$std.sync` after `$std.task` for readability; nothing depends on it.

---

## Task 1: The module, and the three single-task fixtures

**Files:**
- Create: `std/sync/lib.nova`
- Modify: `crates/nova-resolver/src/lib.rs` (`STD_MODULES` 11 → 12)
- Create: `tests/runtime/sync_mutex_uncontended.nova` + `.stdout`, `tests/runtime/sync_mutex_try_lock_fails_when_held.nova` + `.stdout`, `tests/runtime/sync_mutex_release_is_idempotent.nova` + `.stdout`
- Modify: `crates/nova-cli/tests/run_tests.rs` (three registrations)

**Interfaces:**
- Consumes: `yield_now()` from `std/task` (`std/task/lib.nova:114`), `Option<T>` with `is_some` from `std/core`.
- Produces: `pub record Mutex<T> { locked: Bool, value: T }`; `pub record MutexGuard<T> { owner: Mutex<T> }`; `Mutex::new(value: T) -> Mutex<T>`; `Mutex::try_lock(mut self) -> Option<MutexGuard<T>>`; `Mutex::lock(mut self) -> MutexGuard<T>` (async); `MutexGuard::get(self) -> T`; `MutexGuard::release(mut self)`; private `Mutex::take(mut self) -> Bool`.

- [ ] **Step 1: Register the module**

In `crates/nova-resolver/src/lib.rs`, add to `STD_MODULES` after `$std.task` and bump the array length **11 → 12**:

```rust
    ("$std.sync", include_str!("../../../std/sync/lib.nova")),
```

The length is a `const`, so a wrong count will not compile.

- [ ] **Step 2: Write the three failing fixtures**

`tests/runtime/sync_mutex_uncontended.nova` — the whole lifecycle with no contention. **Note `let mut` on both bindings**: `lock` and `try_lock` mutate the receiver, and `release` mutates the guard, so an immutable binding gives `error[E0060]: 'Mutex_T.lock' mutates its receiver, but 'm' is immutable`.

```nova
// One task, no contention: take the lock, read through the guard, release,
// and confirm the mutex is free again. `let mut` is required on both
// bindings -- `try_lock`/`lock` mutate the mutex and `release` mutates the
// guard, so an immutable binding is an E0060 at the call site.
async fn main() {
    let mut m = Mutex::new(41)
    match m.try_lock() {
        Some(g) => {
            let mut held = g
            println("value=${held.get() + 1}")
            held.release()
        }
        None => println("unexpected: a fresh mutex refused try_lock")
    }
    println("free_again=${m.try_lock().is_some()}")
}
```

`tests/runtime/sync_mutex_uncontended.stdout`:

```
value=42
free_again=true
```

`tests/runtime/sync_mutex_try_lock_fails_when_held.nova`:

```nova
// A second `try_lock` must fail while the first guard is live, and succeed
// once it is released. This is contention without suspension -- the
// two-task fixture covers contention across an `.await`.
async fn main() {
    let mut m = Mutex::new(7)
    let first = m.try_lock()
    println("first=${first.is_some()}")
    println("second=${m.try_lock().is_some()}")
    match first {
        Some(g) => {
            let mut held = g
            held.release()
        }
        None => println("unexpected: a fresh mutex refused try_lock")
    }
    println("after_release=${m.try_lock().is_some()}")
}
```

`.stdout`:

```
first=true
second=false
after_release=true
```

`tests/runtime/sync_mutex_release_is_idempotent.nova`:

```nova
// Releasing twice is a no-op, matching `File::close`. There is no `Drop` in
// this language, so release is explicit, and an explicit operation that
// punishes being called twice is a trap rather than a safeguard.
async fn main() {
    let mut m = Mutex::new(1)
    match m.try_lock() {
        Some(g) => {
            let mut held = g
            held.release()
            held.release()
        }
        None => println("unexpected: a fresh mutex refused try_lock")
    }
    println("free=${m.try_lock().is_some()}")
}
```

`.stdout`:

```
free=true
```

- [ ] **Step 3: Register all three — they run zero tests otherwise**

In `crates/nova-cli/tests/run_tests.rs`, following the shape of the existing `fmt_int_pad_run`. **Registration is not automatic**, and an unregistered fixture runs zero tests while looking green — that has bitten four increments.

```rust
/// One task, no contention: lock, read through the guard, release, and the
/// mutex is free again.
#[test]
fn sync_mutex_uncontended_run() {
    let expected =
        std::fs::read_to_string(repo_root().join("tests/runtime/sync_mutex_uncontended.stdout"))
            .expect("expected-output fixture exists")
            .replace("\r\n", "\n");
    nova()
        .arg("run")
        .arg(repo_root().join("tests/runtime/sync_mutex_uncontended.nova"))
        .assert()
        .success()
        .stdout(expected);
}

/// A second `try_lock` fails while the first guard is live and succeeds after
/// it is released.
#[test]
fn sync_mutex_try_lock_fails_when_held_run() {
    let expected = std::fs::read_to_string(
        repo_root().join("tests/runtime/sync_mutex_try_lock_fails_when_held.stdout"),
    )
    .expect("expected-output fixture exists")
    .replace("\r\n", "\n");
    nova()
        .arg("run")
        .arg(repo_root().join("tests/runtime/sync_mutex_try_lock_fails_when_held.nova"))
        .assert()
        .success()
        .stdout(expected);
}

/// `release` twice is a no-op, matching `File::close`.
#[test]
fn sync_mutex_release_is_idempotent_run() {
    let expected = std::fs::read_to_string(
        repo_root().join("tests/runtime/sync_mutex_release_is_idempotent.stdout"),
    )
    .expect("expected-output fixture exists")
    .replace("\r\n", "\n");
    nova()
        .arg("run")
        .arg(repo_root().join("tests/runtime/sync_mutex_release_is_idempotent.nova"))
        .assert()
        .success()
        .stdout(expected);
}
```

- [ ] **Step 4: Run them and watch them fail**

Run: `cargo build --locked --workspace` then `cargo test --locked -p nova-cli --test run_tests sync_mutex_`
Expected: all three FAIL — `std/sync/lib.nova` does not exist, so building the embedded module fails.

- [ ] **Step 5: Write the module**

Create `std/sync/lib.nova`. **No `module` header** — no std module has one, in any of the twelve `lib.nova` files, despite `nova-spec` writing one for every section.

```nova
// Nova standard library -- synchronisation.
//
// An *async* mutex, which is the primitive a cooperative executor needs. A
// thread mutex would be pointless here: `crates/nova-runtime/src/task.rs`'s
// first line is "A single-threaded cooperative executor". But a task can be
// suspended in the middle of a critical section at any `.await`, and while it
// is suspended another task runs -- so anything whose invariant spans a
// suspension point needs exactly this.
//
// Contention is handled by yielding and retrying rather than by parking. That
// keeps the whole module in Nova with no runtime change, and the cost is
// stated rather than hidden: a waiter stays *runnable*, so `report_deadlock`
// cannot see it and a never-released lock spins instead of being diagnosed.
// See the design doc's §5 for the condition that would justify revisiting.

pub record Mutex<T> { locked: Bool, value: T }

// The right to touch a `Mutex`'s value, and the obligation to release it.
//
// NOT RAII, and not by choice: `Drop` is described in three spec files and
// implemented in none, and ADR 0012 foreclosed the mechanism it would need,
// since the collector's sweep names only the dying object's own address and
// offers no per-object hook. So release is explicit and idempotent, exactly
// as `std/fs`'s `File` requires an explicit `close`, and a guard that is
// never released leaves the mutex locked for the life of the process -- a
// documented leak, the same trade ADR 0012 already accepted for descriptors.
pub record MutexGuard<T> { owner: Mutex<T> }

impl<T> Mutex<T> {
    pub fn new(value: T) -> Mutex<T> {
        Mutex { locked: false, value: value }
    }

    // Take the flag if it is free. Private: the two public entry points differ
    // only in what they do when it is not, and returning a `Bool` rather than
    // an `Option` is what lets `lock`'s loop retry without unwrapping --
    // `Option` has `is_some`, `map` and `unwrap_or` but no `unwrap`.
    fn take(mut self) -> Bool {
        if self.locked { return false }
        self.locked = true
        true
    }

    // Take the lock, or report that it is held. The caller that gets `Some`
    // receives the guard it needs; the caller that gets `None` has nothing it
    // could mistake for one.
    pub fn try_lock(mut self) -> Option<MutexGuard<T>> {
        if self.take() {
            Some(MutexGuard { owner: self })
        } else {
            None
        }
    }

    // Take the lock, yielding to every other runnable task until it is free.
    //
    // `while`, not `loop`: Nova has no `loop` keyword -- `loop {` parses as a
    // record literal with `loop` as the type name. `yield_now` re-queues this
    // task behind everything currently runnable, so the holder always gets a
    // turn and can always release; which *waiter* wins afterwards is
    // unspecified, and this mutex makes no fairness guarantee.
    pub async fn lock(mut self) -> MutexGuard<T> {
        while !self.take() {
            yield_now().await
        }
        MutexGuard { owner: self }
    }
}

impl<T> MutexGuard<T> {
    // The guarded value. Records are reference types, so mutating the returned
    // value's fields mutates what the mutex holds -- which is the point.
    //
    // Callable after `release`, and the language cannot prevent that: the guard
    // holds a reference to a heap object either way. Documented, not enforced.
    pub fn get(self) -> T {
        self.owner.value
    }

    // Release the lock. Idempotent -- a second call is a no-op.
    pub fn release(mut self) {
        self.owner.locked = false
    }
}
```

- [ ] **Step 6: Run the three fixtures to green, then the whole suite**

Run: `cargo build --locked --workspace` then `cargo test --locked -p nova-cli --test run_tests sync_mutex_`
Expected: 3 passed.

Confirm registration: `cargo test --locked -p nova-cli --test run_tests sync_mutex_ -- --list`
Expected: exactly three names.

Then the full suite: `cargo build --locked --workspace` followed by `cargo test --locked --workspace --all-features --no-fail-fast`, summing every `test result:` line except those containing `trapped`.
Expected: **1032 passed / 0 failed / 8 ignored across 44 targets** (+3).

- [ ] **Step 7: Three mutations, each restored and verified by `git diff`**

| Mutation | Must fail |
|---|---|
| `take`'s `self.locked = true` line deleted | `sync_mutex_try_lock_fails_when_held_run` — `second=false` becomes `second=true` |
| `take` replaced by `true` unconditionally | `sync_mutex_try_lock_fails_when_held_run` — same row |
| `release`'s `self.owner.locked = false` deleted | `sync_mutex_uncontended_run` — `free_again=true` becomes `free_again=false` |

Report the actual assertion output for each, not "it failed". **A mutation that does not fail the test named for it is the most valuable thing you could find, and you must say so rather than moving on.** Do not claim any test is the *only* one that catches its mutation unless you ran the whole suite and counted — four such claims were measured false across the last three increments.

- [ ] **Step 8: Line endings, then commit**

`tr -cd '\r' < f | wc -c` must equal `wc -l < f` for `std/sync/lib.nova`, all six fixture files, and the two modified `.rs` files. Use `od -c` if a count looks wrong — `grep` and `cat -A` strip CR here.

```bash
git add std/sync/lib.nova crates/nova-resolver/src/lib.rs tests/runtime/sync_mutex_uncontended.nova tests/runtime/sync_mutex_uncontended.stdout tests/runtime/sync_mutex_try_lock_fails_when_held.nova tests/runtime/sync_mutex_try_lock_fails_when_held.stdout tests/runtime/sync_mutex_release_is_idempotent.nova tests/runtime/sync_mutex_release_is_idempotent.stdout crates/nova-cli/tests/run_tests.rs
git commit -F <path-to-utf8-message-file>
```

Subject: `feat(std): add std/sync's Mutex over yield-and-retry`. The body must record why release is explicit (no `Drop`, mechanism foreclosed by ADR 0012) and why waiting yields rather than parks (avoids a fourth `Wait` variant and an arm in `wake_due`'s non-exhaustive `retain`).

---

## Task 2: The discriminating fixture

**This task exists because the other three fixtures would all pass with the `Mutex` deleted.** They exercise the API's shape, not its purpose. A mutex earns its place only by protecting an invariant **across a suspension point**, and this is the only fixture that tests that.

**Files:**
- Create: `tests/runtime/sync_mutex_two_tasks_serialise.nova` + `.stdout`
- Modify: `crates/nova-cli/tests/run_tests.rs` (one registration)

**Interfaces:**
- Consumes: everything Task 1 produced, plus `spawn<T>(fut: Future<T>) -> JoinHandle<T>` (`std/task/lib.nova:102`) and `JoinHandle::join` (`:65`).
- Produces: nothing.

- [ ] **Step 1: Write the fixture**

`tests/runtime/sync_mutex_two_tasks_serialise.nova`. **The read-suspend-write shape is mandatory and is the whole point** — a critical section containing no `.await` is already protected by cooperative scheduling, so a fixture without the `yield_now` inside the lock would pass with the mutex removed and would prove nothing.

```nova
// Two tasks incrementing a shared counter, each reading it, suspending, then
// writing back. Without a mutex both read 0 and the second write clobbers the
// first, giving 1 -- a lost update. Under the mutex the sections serialise and
// the answer is 2.
//
// The `yield_now` INSIDE the critical section is the entire test. Remove it
// and the fixture passes with no mutex at all, because a section containing
// no suspension point is already protected by cooperative scheduling.
pub record Counter { n: Int }

async fn bump(mut m: Mutex<Counter>) {
    let mut g = m.lock().await
    let mut c = g.get()
    let seen = c.n
    yield_now().await
    c.n = seen + 1
    g.release()
}

async fn main() {
    let mut m = Mutex::new(Counter { n: 0 })
    let a = spawn(bump(m))
    let b = spawn(bump(m))
    a.join().await
    b.join().await
    println("n=${m.value.n}")
}
```

`.stdout`:

```
n=2
```

**Both values are measured, not predicted.** This exact program prints `n=2`; the identical program with the mutex removed — `async fn bump(mut c: Counter)` doing read, `yield_now().await`, write — prints `n=1`. Verified before this plan was written.

- [ ] **Step 2: Register it**

```rust
/// Two tasks each read a shared counter, suspend, then write it back, under
/// the mutex. Serialised the answer is 2; interleaved it is 1, because both
/// read 0 and the second write clobbers the first. The `yield_now` inside the
/// critical section is what makes this test able to fail -- a section with no
/// suspension point is already protected by cooperative scheduling.
#[test]
fn sync_mutex_two_tasks_serialise_run() {
    let expected = std::fs::read_to_string(
        repo_root().join("tests/runtime/sync_mutex_two_tasks_serialise.stdout"),
    )
    .expect("expected-output fixture exists")
    .replace("\r\n", "\n");
    nova()
        .arg("run")
        .arg(repo_root().join("tests/runtime/sync_mutex_two_tasks_serialise.nova"))
        .assert()
        .success()
        .stdout(expected);
}
```

Confirm: `cargo test --locked -p nova-cli --test run_tests sync_mutex_ -- --list`
Expected: exactly **four** names now.

- [ ] **Step 3: Run it**

Run: `cargo build --locked --workspace` then `cargo test --locked -p nova-cli --test run_tests sync_mutex_two_tasks_serialise`
Expected: PASS, printing `n=2`.

- [ ] **Step 4: The full suite**

Expected: **1033 passed / 0 failed / 8 ignored across 44 targets** (+1).

- [ ] **Step 5: The two mutations that matter**

| Mutation | Must fail |
|---|---|
| `lock`'s `while !self.take() { … }` replaced by a single `self.take()` discarding the result, so it proceeds without acquiring | `sync_mutex_two_tasks_serialise_run` — `n=2` becomes `n=1` |
| **the mutex removed from the fixture entirely** — `bump` takes `mut c: Counter` and does read / `yield_now().await` / write with no lock | `sync_mutex_two_tasks_serialise_run` — `n=1`. **If this fixture still prints `n=2`, the fixture is wrong, not the mutex**, and you must stop and report that rather than adjusting the golden. |

The second is not a mutation of the implementation but of the **test**, and it is the more important of the two: it is the only check that this fixture is capable of failing. Restore both and verify each restore with `git diff`.

- [ ] **Step 6: Line endings, then commit**

Subject: `test(std): pin the Mutex against a lost update across a suspension`. The body must state both measured values — `n=2` locked, `n=1` unlocked — and that the `yield_now` inside the critical section is what gives the test its power.

---

## Task 3: The records

No code. This task makes the project's documents true.

**Files:**
- Modify: `CHANGELOG.md`, `nova-spec/20-STDLIB.md`
- Create: `docs/adr/0016-std-sync-partial-close.md`

- [ ] **Step 1: CHANGELOG `[Unreleased]`**

Append to the existing sections; `[Unreleased]` accumulates since `v0.2.0-alpha.1` and already holds the `std/time`, `timeout<T>`, `std/log` and `std/fmt` entries. **Do not create a new section.**

**Added**: `std/sync` as a twelfth `STD_MODULES` entry (`STD_MODULES` 11 → 12), `Mutex<T>` with `new`/`try_lock`/`lock`, and `MutexGuard<T>` with `get`/`release`. Say in one clause that release is explicit because there is no `Drop`, and that `STD_ONLY` is **unchanged at 65** — this increment adds no intrinsic. `RESERVED_TYPE_NAMES` unchanged at 7.

- [ ] **Step 2: `nova-spec/20-STDLIB.md` §13**

A dated amendment in the file's existing house style — `**AMENDED 2026-08-20 (branch \`std-sync-mutex\`):**`, matching the precedents at lines 31, 36, 169, 184, 199 and 214. **Read two of them before writing yours.**

It must record four things:

1. **What shipped**, and that position 8 is **partially** closed.
2. **That release is explicit, and why it is not a shortcut.** `Drop` appears in `12-TYPESYSTEM.md:192`, `13-RUNTIME.md:96` and `14-CODEGEN.md:24` and is implemented in none of them — no handling in `nova-typeck`, `nova-resolver` or `nova-mir`, no `trait Drop` in `std/core` — and **ADR 0012 already foreclosed the mechanism**, because the collector's sweep names only the dying object's own address and so offers no per-object hook. This is not "`Drop` is not built yet"; it is "there is nowhere to put one".
3. **Why each of the three unbuilt items is unbuilt**, each with its measured reason: `channel` returns a **tuple** and `error[E0900]: tuple types are not supported yet`; `spawn_blocking` needs a thread pool in a runtime whose own first line is *"A single-threaded cooperative executor"*; and `JoinHandle::cancel` contradicts the **abandonment-not-cancellation** contract that ADR 0009 and the `timeout<T>` increment settled. **State that these are deferrals with named blockers, not oversights** — `channel` in particular is desirable and merely needs an API that Nova's type system can express.
4. **That §13's `module std.sync` header line is not implemented, and that this is true of every section.** No std module has one, across all twelve `lib.nova` files. §3's and §10's amendments already record the same fact for their own sections; a reader checking only §13 would otherwise conclude `std/sync` alone is non-conforming.

- [ ] **Step 3: ADR 0016**

First run `ls docs/adr/` and confirm `0016` is unused — `0001`–`0015` are in use, and a previous increment guessed a number that already existed.

`docs/adr/0016-std-sync-partial-close.md`. The requirement comes from `00-MASTER-SPEC.md` **§7 item 5** — *"ADR written for any decision deviating from this spec"*. **Cite §7, not §0**: §0 is "Project Identity (FINAL)", a table of fields with no ADR text, and an earlier spec on this project cited it wrongly, requiring the correction to be made twice.

- **Context:** §13 specifies `Mutex`, `channel`, `spawn_blocking` and `JoinHandle::cancel`. Position 8 was reached after positions 2 and 6 had each been taken out of order (ADRs 0014 and 0015).
- **Decision:** ship `Mutex<T>` and `MutexGuard<T>`; defer the other three with the named blockers above. Record that the framing under which this increment was nearly skipped — *"a Mutex has nothing to contend with on a single-threaded executor"* — **was wrong**, and that §13's own `async fn lock` is what shows why: a thread mutex would be pointless, an async mutex is necessary, because a task can be suspended mid-critical-section.
- **Consequences:** position 8 is **partially** closed, which is what distinguishes this ADR from 0014 (position 2 skipped twice) and 0015 (position 2 closed). Record the **yield-and-retry trade explicitly**: no executor change, no fourth `Wait` variant, no arm in `wake_due`'s non-exhaustive `retain` — against a waiter that stays runnable, so `report_deadlock` cannot see it and a never-released lock spins rather than being diagnosed. **Name the condition that would justify revisiting**: contention frequent enough for spinning to cost measurable time, or a real deadlock going undiagnosed. And record that the mutex is **not re-entrant** and makes **no fairness guarantee**.

- [ ] **Step 4: Verify, then commit**

Full suite must be **unchanged at 1033 / 0 / 8 across 44 targets** — for a records-only task an unchanged total is the evidence, not a formality. Clippy and fmt clean. Check CRLF on every markdown file touched, **before and after** editing.

Subject: `docs: record std/sync's Mutex and position 8's partial close`.

---

## Plan Self-Review

**Spec coverage.** §1 scope in → Tasks 1–2; §1 out → Task 3's §13 amendment and ADR each carry the five exclusions with reasons. §2 (why position 8 is well-founded) → the module's header comment and ADR 0016's Decision. §3 (no `Drop`) → the `MutexGuard` doc comment and the §13 amendment. §4 (surface) → Task 1 Step 5. §5 (yield-and-retry and its cost) → `lock`'s doc comment and ADR 0016's Consequences. §6 (edge cases) → `sync_mutex_release_is_idempotent` covers double release; `get`-after-release and non-re-entrancy are **documented rather than tested**, as the spec states, because both are behaviours the language cannot prevent. §7 (testing) → Tasks 1–2. §8 (records) → Task 3. §9 (measured facts) → used throughout. **No gaps.**

**Type consistency.** `Mutex<T>`, `MutexGuard<T>`, `new`, `take`, `try_lock`, `lock`, `get`, `release` are spelled identically in Task 1's Interfaces block, Task 1 Step 5's code, and Task 2's fixture. `Counter` is defined inside Task 2's fixture and used only there. `m.value.n` in Task 2 reads the field directly rather than through a guard, which is deliberate — the assertion is about the final state after both guards are released.

**Test-count arithmetic.** 1029 baseline → 1032 after Task 1 (+3 fixtures) → 1033 after Task 2 (+1) → 1033 after Task 3. Each task states its expected total.

**Every value in Task 2 was measured before being written.** `n=2` under the mutex and `n=1` without it were both run. That matters more than usual here: three of this increment's four fixtures would pass with the mutex deleted, so Task 2 is the increment's only real test, and a plan that guessed its numbers would be guessing about the only thing that proves the feature works.

**One deliberate omission.** The spec's §7 lists a mutation "`take` never sets `locked`" *and* "`take` returns true unconditionally" as separate rows catching the same test. Both are kept in Task 1 Step 7 because they fail for different reasons — the first never acquires, the second always acquires — and a reader who sees only one might conclude the other is untested.
