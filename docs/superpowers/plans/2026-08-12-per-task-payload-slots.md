# Per-Task Payload Slots Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Move `std/fs`'s three thread-local payload slots into per-task storage keyed on the current task, so increment 4's I/O poller cannot make one task read another's payload.

**Architecture:** The three `thread_local! { Cell<usize> }` slots in `crates/nova-runtime/src/fs.rs` collapse into one `thread_local! { RefCell<Vec<Slots>> }` owned by that same module, indexed by `current_task() + 1` with index 0 reserved for "no task". `stash`/`take` swap their `&'static LocalKey<Cell<usize>>` parameter for a three-variant `Slot` enum. `task.rs` gains one accessor (`current_task`) and calls one new `fs` function (`release_task_slots`) at the two places it already releases a task's state root. No Nova-visible signature changes; `std/fs/lib.nova` is not touched.

**Tech Stack:** Rust 2021 (MSRV 1.78), `nova-runtime` crate, `thread_local!`/`RefCell`/`Cell`, the crate's own `gc` module.

**Spec:** `docs/superpowers/specs/2026-08-12-per-task-payload-slots-design.md`

## Global Constraints

- `cargo build --workspace` **before** `cargo test`. Not optional — many tests link against the built runtime.
- `cargo test --workspace --no-fail-fast` at default parallelism. Sum **every** `test result:` line. **Never pipe cargo output through `head` or `tail`** before summing.
- Baseline is **883 passed / 0 failed / 8 ignored across 44 targets.** The passed count must rise by exactly the number of tests added; state the arithmetic.
- **No panic may cross a generated poll boundary.** Generated code has no landing pads. `abort_with` is acceptable — it terminates without unwinding. `RefCell` borrow panics, `unwrap`, `expect`, slice indexing and fallible `format!` are **not**.
- The **8 `#[ignore]`d ADR-0010 conservative-scan GC tests** stay ignored and untouched.
- `cargo clippy --all-targets --all-features -- -D warnings` and `cargo fmt --all --check` clean.
- **No `reason = "…"` in any lint attribute** (MSRV 1.78; the MSRV CI job is known-vacuous, so this ships as a user build failure).
- Every fixture path unique per process.
- Commit messages written to a **UTF-8 file** and applied with `git commit -F` — **never a heredoc.** Every commit body ends with `Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>`.
- **Never push.**
- Branch: `per-task-slots`, already created, spec committed at `0bebc66`.

## File Structure

| File | Responsibility | Change |
|---|---|---|
| `crates/nova-runtime/src/fs.rs` | Owns the `std/fs` status/payload boundary end to end, including all slot storage and GC root discipline for payloads | Modify: replace three thread-locals with one per-task table; retype `stash`/`take`; add `release_task_slots` |
| `crates/nova-runtime/src/task.rs` | Owns the executor, task lifetime, and the state-root release policy | Modify: expose `current_task()`; add a `#[cfg(test)]` setter; call `fs::release_task_slots` at two existing release points |
| `CHANGELOG.md` | User-facing history | Modify: one entry under the heading whose stated scope actually admits an internal runtime change |

`fs.rs` keeps owning the boundary — `task.rs` learns nothing about payloads beyond one function name. That is the layering the spec chose deliberately (spec §6 records why payload fields on `struct Task` were declined).

---

### Task 1: The per-task table, keyed on the current task

**Files:**
- Modify: `crates/nova-runtime/src/fs.rs:72-98` (the three thread-locals), `:135-157` (`stash`/`take`), and every call site: `:165`, `:220`, `:238`, `:266`, `:286`, `:315`, `:324`, `:456`
- Modify: `crates/nova-runtime/src/task.rs` — add `current_task()` and a test-only `set_current_for_test`
- Test: `crates/nova-runtime/src/fs.rs`'s own `#[cfg(test)] mod tests`

**Interfaces:**
- Consumes: `crate::task::abort_with(msg: &str) -> !` (`task.rs:90`, already `pub(crate)`, already used by `bytes.rs`); `gc::add_root(*mut u8)`, `gc::remove_root(*mut u8)`, `gc::root_count(addr: usize) -> usize` (`gc.rs:216`, `:230`, `:278`)
- Produces, for Task 2: `enum Slot { Buffer, Array, Message }` (private to `fs.rs`); `fn stash(slot: Slot, ptr: *mut NovaStr)`; `fn take(slot: Slot) -> usize`; `pub(crate) fn crate::task::current_task() -> Option<i64>`; `#[cfg(test)] pub(crate) fn crate::task::set_current_for_test(id: Option<i64>)`

- [ ] **Step 1: Add the two `task.rs` accessors**

`CURRENT` is a private `thread_local!` at `task.rs:226`. Add beside it:

```rust
/// The task currently being polled, or `None` outside a poll.
///
/// `poll_one` sets this for exactly the duration of the `poll` call
/// (`task.rs:413-425`), so a runtime intrinsic reached from generated code can
/// key per-task storage on it. `fs.rs` is the first consumer.
pub(crate) fn current_task() -> Option<i64> {
    CURRENT.with(|c| c.get())
}

/// Set [`CURRENT`] directly, for tests that need a task context without an
/// executor.
///
/// Test-only because nothing in production may set `CURRENT` outside
/// `poll_one`. Gated `#[cfg(test)]` rather than `#[cfg(windows)]`: its callers
/// are `fs.rs`'s tests, which run on every platform, so unlike
/// `gc::collect_for_test` this cannot read as dead code off Windows.
#[cfg(test)]
pub(crate) fn set_current_for_test(id: Option<i64>) {
    CURRENT.with(|c| c.set(id));
}
```

- [ ] **Step 2: Write the failing interleaving test**

Add to `fs.rs`'s `mod tests`. This is the load-bearing test of the whole increment.

```rust
/// Two tasks stashing between one task's stash and its take must not collide.
///
/// This is the defect the per-task table exists to prevent, and it cannot be
/// reached from Nova today: `std/fs`'s wrappers are straight-line, so nothing
/// runs between a stash and its take. Increment 4's poller inserts exactly
/// that gap, which is why the interleaving is built here by hand instead.
///
/// **Fails against the pre-change thread-local slots**, where task B's stash
/// releases task A's root and overwrites the one shared slot, so A's take
/// returns B's pointer. That is what earns this test.
#[test]
fn a_stash_is_private_to_the_task_that_made_it() {
    let a = crate::gc_str("task-a-payload");
    let b = crate::gc_str("task-b-payload");

    crate::task::set_current_for_test(Some(0));
    stash(Slot::Buffer, a);

    crate::task::set_current_for_test(Some(1));
    stash(Slot::Buffer, b);

    crate::task::set_current_for_test(Some(0));
    let got = take(Slot::Buffer);
    assert_eq!(
        got, a as usize,
        "task 0 must read back its own payload, not task 1's"
    );

    crate::task::set_current_for_test(Some(1));
    assert_eq!(
        take(Slot::Buffer),
        b as usize,
        "task 1's payload must still be there, undisturbed"
    );
    crate::task::set_current_for_test(None);
}
```

- [ ] **Step 3: Write the failing no-task-key test**

```rust
/// "No task" is a key, not a special case, and it does not collide with task 0.
///
/// Kills the plausible mistake of indexing by `id as usize` instead of
/// `id as usize + 1`, which would map task 0 onto the no-task slot. `fs.rs`'s
/// other unit tests all run with `CURRENT == None`, so this is the path they
/// take.
#[test]
fn the_no_task_key_does_not_collide_with_task_zero() {
    let none_payload = crate::gc_str("no-task");
    let zero_payload = crate::gc_str("task-zero");

    crate::task::set_current_for_test(None);
    stash(Slot::Buffer, none_payload);

    crate::task::set_current_for_test(Some(0));
    stash(Slot::Buffer, zero_payload);

    crate::task::set_current_for_test(None);
    assert_eq!(
        take(Slot::Buffer),
        none_payload as usize,
        "the no-task slot must not be task 0's slot"
    );

    crate::task::set_current_for_test(Some(0));
    assert_eq!(take(Slot::Buffer), zero_payload as usize);
    crate::task::set_current_for_test(None);
}
```

- [ ] **Step 4: Run both tests and confirm they fail**

```bash
cargo build --workspace
```

```bash
cargo test -p nova-runtime a_stash_is_private_to_the_task_that_made_it the_no_task_key_does_not_collide_with_task_zero
```

Expected: both FAIL to compile, because `Slot`, `crate::task::set_current_for_test` and the retyped `stash`/`take` do not exist yet. **A compile failure is a legitimate red here** — but before implementing, temporarily add only `set_current_for_test` and a `Slot` enum whose `stash`/`take` still write the old thread-locals, and re-run to see `a_stash_is_private_to_the_task_that_made_it` fail on the *assertion* rather than on compilation. Record that assertion failure in your report: it is the evidence that the test discriminates. Then revert the scaffold and implement properly.

- [ ] **Step 5: Replace the three thread-locals with one per-task table**

Delete the `thread_local!` block at `fs.rs:72-98` and put this in its place, carrying over the existing doc comments' substance — in particular why `Buffer` serves `String` and `Bytes` both:

```rust
/// Which payload a `stash`/`take` pair is addressing.
///
/// Replaces the three separate `thread_local!` slots this module used before
/// per-task storage. `Buffer` serves `String` *and* `Bytes` payloads, not one
/// variant each: the two have the identical `{len, ptr}` representation (see
/// `crate::bytes`'s module doc comment), and which one a payload is gets
/// carried entirely by which builtin stashed it and which one reads it back
/// (`nova_rt_fs_read_to_string`/`nova_rt_fs_take_string` treat it as UTF-8;
/// `nova_rt_fs_read`/`nova_rt_fs_take_bytes` do not interpret it at all).
#[derive(Copy, Clone)]
enum Slot {
    Buffer,
    Array,
    Message,
}

/// One task's three payload slots. Each field is a GC-rooted pointer, or 0.
#[derive(Clone, Copy, Default)]
struct Slots {
    buffer: usize,
    array: usize,
    message: usize,
}

thread_local! {
    /// Payload slots, per task, indexed by `slot_index`.
    ///
    /// `thread_local!` for the reason `task.rs`'s module doc gives for `TASKS`
    /// and `QUEUE`: the GC's root table is per-thread, so a second thread
    /// running Nova code would free objects the first still holds.
    static SLOTS: RefCell<Vec<Slots>> = const { RefCell::new(Vec::new()) };
}
```

`fs.rs`'s imports are currently exactly three lines (`:30-32`):

```rust
use crate::{gc, NovaStr};
use std::cell::Cell;
use std::thread::LocalKey;
```

The three deleted thread-locals were the only users of **both** `Cell` and `LocalKey`, so both imports go and `use std::cell::RefCell;` replaces them. **Verify that by grepping for `Cell` and `LocalKey` in the file after the change rather than assuming it** — an unused import is `-D warnings`, and an unused-import defect in this crate shipped undetected until CI first ran on Linux, because it only fires off Windows.

`abort_with` is reached fully qualified as `crate::task::abort_with`, matching `bytes.rs:120`; `fs.rs` does not import `task` today and does not need to.

- [ ] **Step 6: Add the keying and access helper**

```rust
/// The `SLOTS` index for the current task.
///
/// `id + 1`, so index **0 is the reserved no-task key**. Task ids are dense
/// from zero (`poll_one` reads `tasks.get(id as usize)`), so a `Vec` gives O(1)
/// access with no hashing and no per-call allocation.
fn slot_index() -> usize {
    crate::task::current_task().map_or(0, |id| id as usize + 1)
}

/// Run `f` against one field of the current task's slots, growing the table if
/// this task has not been seen before.
///
/// **Panic-free by construction, because this runs inside a generated poll
/// boundary with no landing pads.** `try_borrow_mut` rather than `borrow_mut`,
/// and `get_mut` rather than `[i]`; both fall back to `abort_with`, which
/// terminates without unwinding. The `get_mut` arm is unreachable — the resize
/// immediately above guarantees `i < len` — and exists so the impossible case
/// still cannot unwind.
fn with_slot<R>(slot: Slot, f: impl FnOnce(&mut usize) -> R) -> R {
    SLOTS.with(|cell| {
        let Ok(mut slots) = cell.try_borrow_mut() else {
            crate::task::abort_with("nova_rt_fs: payload slot table is already borrowed")
        };
        let i = slot_index();
        if slots.len() <= i {
            slots.resize(i + 1, Slots::default());
        }
        let Some(entry) = slots.get_mut(i) else {
            crate::task::abort_with("nova_rt_fs: payload slot index out of range after resize")
        };
        let field = match slot {
            Slot::Buffer => &mut entry.buffer,
            Slot::Array => &mut entry.array,
            Slot::Message => &mut entry.message,
        };
        f(field)
    })
}
```

- [ ] **Step 7: Retype `stash` and `take`**

Replace `fs.rs:135-157`. **Keep both doc comments' existing reasoning** — the root-before-publish ordering, the overwrite release added by the byte-type branch's final review, and `take`'s ordering rationale — and add the one new ordering fact:

```rust
fn stash(slot: Slot, ptr: *mut NovaStr) {
    take(slot);
    gc::add_root(ptr as *mut u8);
    with_slot(slot, |field| *field = ptr as usize);
}

/// Read and clear `slot`, releasing its root, and return what was there (`0`
/// if the slot was empty).
///
/// **The `gc::remove_root` deliberately happens after the borrow is dropped**,
/// not inside `with_slot`'s closure. Holding a `RefCell` borrow across a call
/// into the collector would be a re-entrancy hazard for no benefit; the
/// pointer is already out of the table and owned by this frame by then.
fn take(slot: Slot) -> usize {
    let ptr = with_slot(slot, |field| {
        let ptr = *field;
        *field = 0;
        ptr
    });
    if ptr != 0 {
        gc::remove_root(ptr as *mut u8);
    }
    ptr
}
```

- [ ] **Step 8: Update all eight call sites**

Mechanical: `&MESSAGE_SLOT` → `Slot::Message`, `&ARRAY_SLOT` → `Slot::Array`, `&BUFFER_SLOT` → `Slot::Buffer`, at `fs.rs:165`, `:220`, `:238`, `:266`, `:286`, `:315`, `:324`, `:456`. The existing tests at `:637`, `:677`, `:684`, `:699`, `:723` use the same names and change the same way.

**Grep for the three old names afterwards and confirm zero hits outside doc comments**, including in `lib.rs` and any test module — a missed one is a compile error, but a stale mention in prose is the defect class this project keeps producing.

- [ ] **Step 9: Run the two new tests, then the whole suite**

```bash
cargo build --workspace
```

```bash
cargo test -p nova-runtime a_stash_is_private_to_the_task_that_made_it the_no_task_key_does_not_collide_with_task_zero
```

Expected: both PASS. Confirm they actually ran — **a zero-match filter exits 0 here**, so a typo reads as a pass. Then:

```bash
cargo test --workspace --no-fail-fast
```

Expected: **885 passed / 0 failed / 8 ignored across 44 targets** — 883 + 2. State the arithmetic. Every pre-existing `fs` test must pass untouched.

- [ ] **Step 10: Lint and commit**

```bash
cargo clippy --all-targets --all-features -- -D warnings
```

```bash
cargo fmt --all --check
```

Write the message to a UTF-8 file, then:

```bash
git add -A && git commit -F <message-file>
```

---

### Task 2: Release a task's payloads where its state root is already released

**Files:**
- Modify: `crates/nova-runtime/src/fs.rs` — add `release_task_slots`
- Modify: `crates/nova-runtime/src/task.rs:526` (`release_internal`) and `:574` (`take_output_internal`)
- Test: `crates/nova-runtime/src/fs.rs`'s `mod tests`

**Interfaces:**
- Consumes from Task 1: `enum Slot`, `struct Slots`, `SLOTS`, `with_slot`, `stash`, `take`, `crate::task::current_task`, `crate::task::set_current_for_test`
- Produces: `pub(crate) fn release_task_slots(id: i64)`

- [ ] **Step 1: Write the failing release test**

```rust
/// A task's unread payload is released when its state root is.
///
/// `task.rs` releases a task's state root in `release_internal` and
/// `take_output_internal` — deliberately not at completion, because a spawned
/// task's output has to outlive it so a later `join` can take it
/// (`task.rs:288-291`). Payload release hangs off the same two points so
/// payload lifetime follows the policy `task.rs` already owns rather than a
/// second one.
///
/// Uses `gc::root_count` rather than asserting the object is collected: per
/// ADR 0010, a churn-loop test asserting survival cannot discriminate on this
/// platform. This proves bookkeeping, not survival, and claims only that.
#[test]
fn releasing_a_tasks_slots_drops_the_roots_it_held() {
    let payload = crate::gc_str("never-read");
    let addr = payload as usize;

    crate::task::set_current_for_test(Some(7));
    stash(Slot::Buffer, payload);
    assert_eq!(gc::root_count(addr), 1, "the stash roots its pointer");

    crate::task::set_current_for_test(None);
    release_task_slots(7);
    assert_eq!(
        gc::root_count(addr),
        0,
        "releasing task 7's slots must release the root it held"
    );

    // Idempotent: a second release must not double-remove or abort.
    release_task_slots(7);
    assert_eq!(gc::root_count(addr), 0);
}

/// Releasing one task's slots leaves another task's alone.
#[test]
fn releasing_one_tasks_slots_leaves_another_tasks_intact() {
    let keep = crate::gc_str("keep-me");
    let keep_addr = keep as usize;

    crate::task::set_current_for_test(Some(3));
    stash(Slot::Message, keep);
    release_task_slots(4);
    assert_eq!(
        gc::root_count(keep_addr),
        1,
        "task 4's release must not touch task 3's slots"
    );

    crate::task::set_current_for_test(Some(3));
    assert_eq!(take(Slot::Message), keep as usize);
    crate::task::set_current_for_test(None);
}
```

- [ ] **Step 2: Run them and confirm they fail**

```bash
cargo test -p nova-runtime releasing_a_tasks_slots_drops_the_roots_it_held releasing_one_tasks_slots_leaves_another_tasks_intact
```

Expected: FAIL — `release_task_slots` does not exist.

- [ ] **Step 3: Implement `release_task_slots`**

Note it keys on the **`id` argument**, not on `current_task()`: the two call sites in `task.rs` may run with `CURRENT` unset or set to a different task.

```rust
/// Release every payload root task `id` still holds, and clear its slots.
///
/// Called from `crate::task::release_internal` and `crate::task::take_output_internal` —
/// the two places that release a task's *state* root. Keyed on `id` rather
/// than [`slot_index`] precisely because those call sites do not run with
/// `CURRENT` set to the task being released.
///
/// Idempotent, and silent for a task that never stashed anything: a task id
/// past the end of the table simply has no slots.
///
/// Roots are released after the borrow is dropped, for the same reason
/// [`take`] does it that way.
pub(crate) fn release_task_slots(id: i64) {
    let held = SLOTS.with(|cell| {
        let Ok(mut slots) = cell.try_borrow_mut() else {
            crate::task::abort_with("nova_rt_fs: payload slot table is already borrowed")
        };
        let index = id as usize + 1;
        match slots.get_mut(index) {
            Some(entry) => {
                let held = [entry.buffer, entry.array, entry.message];
                *entry = Slots::default();
                held
            }
            None => [0, 0, 0],
        }
    });
    for ptr in held {
        if ptr != 0 {
            gc::remove_root(ptr as *mut u8);
        }
    }
}
```

- [ ] **Step 4: Call it from both release points in `task.rs`**

In `release_internal` (`task.rs:526`) and `take_output_internal` (`task.rs:574`), beside the existing state-root release. Add at each site a comment naming the other, so a future reader changing one finds the other:

```rust
// A task's payload slots die with the task. `fs.rs` owns the storage and the
// root discipline; this is the whole of `task.rs`'s knowledge of it. The
// matching call is in `take_output_internal` / `release_internal` -- both
// release points need it, because either can be the last thing to touch a
// task.
crate::fs::release_task_slots(id);
```

- [ ] **Step 5: Run the new tests, then the whole suite**

```bash
cargo build --workspace
```

```bash
cargo test -p nova-runtime releasing_a_tasks_slots_drops_the_roots_it_held releasing_one_tasks_slots_leaves_another_tasks_intact
```

Expected: both PASS, and confirm they ran.

```bash
cargo test --workspace --no-fail-fast
```

Expected: **887 passed / 0 failed / 8 ignored across 44 targets** — 885 + 2. State the arithmetic.

- [ ] **Step 6: Mutation-test both release points**

Delete the `release_task_slots` call from `release_internal` alone, run the two new tests, and record whether either dies. Then restore it and delete the one in `take_output_internal` alone, and do the same. **Report the result honestly, including if one survives** — the tests above call `release_task_slots` directly, so they may not discriminate between the two call sites at all. If a call site is unpinned, say so and say what a test that pinned it would have to drive, rather than adding a test that only appears to.

- [ ] **Step 7: Lint and commit**

```bash
cargo clippy --all-targets --all-features -- -D warnings
```

```bash
cargo fmt --all --check
```

```bash
git add -A && git commit -F <message-file>
```

---

### Task 3: The unverified premise, a mechanical guard, and the docs

**Files:**
- Test: `crates/nova-runtime/src/fs.rs`'s `mod tests`
- Modify: `CHANGELOG.md`
- Modify: `docs/superpowers/specs/2026-08-12-per-task-payload-slots-design.md` (record the §4 premise's outcome)

**Interfaces:**
- Consumes from Tasks 1 and 2: everything above; `with_slot` and `release_task_slots` are the only `try_borrow_mut` sites

- [ ] **Step 1: Discharge the spec's unverified premise**

Spec §4 says the `abort_with` backstop is only unreachable if no `fs` intrinsic re-enters the executor while holding the `SLOTS` borrow, and records that **this was not checked across all thirteen intrinsics.** Discharge it now: read every `nova_rt_fs_*` function in `fs.rs` and confirm none calls into `task.rs` (beyond `current_task`/`abort_with`) or into anything that could re-enter `with_slot`.

**List the intrinsics you read, by name, in your report.** If any does re-enter, stop and report it rather than adjusting the guard — that would change the design, which is not this task's call.

- [ ] **Step 2: Write the mechanical panic-freedom guard**

Model it on `no_filesystem_intrinsic_registers_a_park` (`fs.rs:580-596`), which reads its own source with `include_str!` and scans only the portion before `#[cfg(test)]`. Target the specific new hazard rather than a general panic sweep, so it cannot fail for unrelated reasons:

```rust
/// Every `SLOTS` access in production code is fallible-borrow, never
/// `borrow_mut`.
///
/// A `RefCell` borrow panic in this module would cross a generated poll
/// boundary, where there are no landing pads — the one hazard the per-task
/// table introduced that the three `Cell`s it replaced could not have. Pinned
/// at its source rather than by a fixture, so it covers every future access
/// without a recount.
///
/// Scans only the part of this file before its own `#[cfg(test)] mod tests`,
/// for the same reason `no_filesystem_intrinsic_registers_a_park` does: this
/// test's own doc comment names the forbidden string in prose, and scanning
/// the whole file would fail on that alone.
#[test]
fn no_slot_access_can_panic_on_a_borrow() {
    let source = include_str!("fs.rs");
    let production = source.split("#[cfg(test)]").next().unwrap_or(source);
    assert!(
        !production.contains(".borrow_mut()"),
        "production code in fs.rs must use try_borrow_mut, not borrow_mut"
    );
    assert!(
        !production.contains(".borrow()"),
        "production code in fs.rs must not take an infallible RefCell borrow"
    );
}
```

- [ ] **Step 3: Run it, and prove it discriminates**

```bash
cargo test -p nova-runtime no_slot_access_can_panic_on_a_borrow
```

Expected: PASS. Then temporarily change one `try_borrow_mut` in `with_slot` to `borrow_mut`, re-run, confirm it FAILS, and revert. Record both outcomes — a guard that passes but cannot fail is worthless.

- [ ] **Step 4: Update the spec with the premise's outcome**

Spec §4 asks the implementation to confirm the re-entrancy premise "and say which intrinsics it read to do so." Append a dated note to §4 recording the answer from Step 1. Follow the amendment convention already in the repo's specs: keep the original text, add a marked paragraph below it. Do **not** delete the original claim.

- [ ] **Step 5: Update `CHANGELOG.md`**

No Nova-visible behaviour changes, so this does not belong under `### Added`. **Read the `### Changed` and `### Fixed` headings' own stated scope before filing**, and put it under whichever actually admits an internal runtime change — on an earlier branch an entry was cross-filed under a heading scoped to a different phase, and that was a review finding. State plainly what moved and what it prevents, and **do not claim it fixes the untaken-payload root leak**: spec §4 records that a leaked task still keeps its last unread payload, which is the state leak ADR 0009 §1 already documents.

- [ ] **Step 6: Claim sweep over everything this branch changed**

```bash
git diff main --stat
```

Grep each changed file's **added** lines for `always`, `every`, `only`, `any`, `never`, `all`, `cannot`. Per hit: delete the quantifier, or state the measurement behind it. Scope to added lines — whole-file grepping is dominated by pre-existing code.

Two things the sweep structurally cannot catch, so check them by reading: **a doc quoting a literal diagnostic string**, and **a sentence this change falsified but did not touch**. For the second, specifically re-read: `fs.rs`'s module doc comment; the old three-slot doc comments' claims now carried into `Slot`; `a_stashed_string_is_rooted_until_it_is_taken`'s doc comment (referenced from the old `stash` comment at `:127`); ADR 0011's description of the `std/fs` boundary; and ADR 0009 §1's async-footgun list.

**Report the sweep's output even if it finds nothing.**

- [ ] **Step 7: Full verification and commit**

```bash
cargo build --workspace
```

```bash
cargo test --workspace --no-fail-fast
```

Expected: **888 passed / 0 failed / 8 ignored across 44 targets** — 887 + 1. State the arithmetic.

```bash
cargo clippy --all-targets --all-features -- -D warnings
```

```bash
cargo fmt --all --check
```

```bash
git add -A && git commit -F <message-file>
```

---

## Self-Review

**Spec coverage.** §3 storage and keying → Task 1 Steps 5–6. §3 no-task key → Task 1 Step 3. §4 root discipline preserved → Task 1 Step 7. §4 third release point → Task 2. §4 "what this does not fix" → Task 3 Step 5's explicit prohibition. §4 panic safety → Task 1 Step 6 plus Task 3 Step 2's guard. §4's unverified premise → Task 3 Step 1. §5 interleaving test → Task 1 Step 2, with its red state demanded in Step 4. §5 no-task key, root accounting, release on hooks, overwrite release → Tasks 1 and 2. §6 alternatives → no task; they are declined, not built. §8 definition of done → distributed across all three tasks' verification steps.

**Placeholder scan.** No "TBD", no "add error handling", no "similar to Task N". Every code step carries the actual code. The one deliberately open-ended step is Task 2 Step 6, which asks for an honest mutation result rather than a predetermined one — that is a reporting requirement, not a placeholder.

**Type consistency.** `Slot`/`Slots`/`SLOTS`/`slot_index`/`with_slot`/`stash`/`take`/`release_task_slots`/`current_task`/`set_current_for_test` are spelled identically in every task, and `stash(Slot, *mut NovaStr)` / `take(Slot) -> usize` / `release_task_slots(i64)` keep one signature throughout. Test counts chain: 883 → 885 → 887 → 888.

**One risk the plan carries deliberately.** Task 2 Step 6 may find that neither new test discriminates between the two `task.rs` call sites, because the tests call `release_task_slots` directly. The plan asks for that to be reported rather than papered over, because the alternative — a test that drives a real spawn/join cycle — is a larger piece of work whose value should be judged on the evidence, not assumed in advance.
