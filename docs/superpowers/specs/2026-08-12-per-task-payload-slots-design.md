# Per-task payload slots

**Status:** accepted, not yet implemented. Increment 3a of the `std/fmt` + `std/io` decomposition,
inserted ahead of increment 3 (`Read`/`Write`, `stdin`/`stdout`/`stderr`, `File`).

**Base:** `main` == `origin/main` == `52e6733`. 883 tests (8 deliberately ignored) across 44 targets;
CI green on ubuntu, macos and windows.

## 1. The problem

`std/fs`'s boundary returns one status word from each intrinsic and leaves the payload in a
thread-local slot for a second intrinsic to collect. There are three
(`crates/nova-runtime/src/fs.rs:88`, `:95`, `:97`):

| Slot | Holds | Stashed at | Taken at |
|---|---|---|---|
| `BUFFER_SLOT` | a `*mut NovaStr` — `String` *or* `Bytes` | `:238`, `:286` | `:266`, `:315` |
| `ARRAY_SLOT` | a GC array block of `DirEntry` names | `:220` | `:456` |
| `MESSAGE_SLOT` | the OS error message | `:165` | `:324` |

All three are `thread_local! { static … : Cell<usize> }`. **Their correctness rests on there being no
`.await` between an intrinsic call and its matching read**, which is why the wrappers in
`std/fs/lib.nova` that read a slot are deliberately straight-line. (`exists` reads none, and
`temp_dir` is a plain `fn` that touches no slot at all, so they have nothing to keep straight.)

**Note (2026-08-13, Task 3 of this plan): the line numbers throughout this section describe the
pre-change file.** By the time this increment landed, the three `thread_local!` slots above were
replaced by the per-task table §3 describes (`Slot`/`Slots`/`SLOTS` in
`crates/nova-runtime/src/fs.rs`), and none of the lines cited above — the intro sentence's `:88`,
`:95`, `:97`, or the table's stash/take sites — still hold the text they cite. Left as originally
written rather than renumbered: this section is the historical record of the problem this design
solves, not a live index into the post-change file. **This note's scope is the whole document, not
only this section** (final review, M2): most other `fs.rs`/`task.rs` line citations below, in §3 and
§4 alike, were written against a file the branch's later commits went on to move code around in and
were never updated to match — this document flags the rare exception where one still is — so
re-derive any citation in this design before relying on it rather than assuming it survived, the
same as this section's own.

That invariant holds today because `std/fs`'s `async fn`s never suspend — the filesystem call runs
synchronously inside the first poll. Increment 4's I/O poller removes exactly that property. Once a
read can return `POLL_PENDING` between the stash and the take, a second task polled in the interval
stashes into the same thread-global slot and the first task reads the second task's payload. Today
that is latent; under the poller it is a live clobber, and the displaced pointer's GC root goes with
it.

**Why now rather than with the poller.** The surface is 43 intrinsics. Increment 3 adds `open`, `File`,
and `Read`/`Write` for three more handle types, which add payload traffic of their own. Migrating the
boundary
before that work is written is cheaper than migrating it after, and it means increment 3's new
intrinsics are written against the correct boundary rather than converted later.

## 2. Non-goals

- **No new API surface.** Not one Nova-visible signature changes. `std/fs/lib.nova` is untouched.
- **No poller.** `Wait::Io`, timed waits and the drive loop's default arm remain increment 4's.
- **No `task_ctx`.** `PollFn`'s reserved `task_ctx` parameter (`task.rs:70`) stays null and unused;
  routing per-task context through it would require changes in both codegen backends and contradicts
  ADR 0009's documented contract. §6 records why it was considered and declined.
- **Not the `Bytes` debt.** `Hash`/`Display`/`Clone`/`Ord`, and the `Bytes::slice`-clamps versus
  `String::slice`-panics inconsistency, stay filed for increment 3. The value of this increment is
  that nothing else moves.

## 3. Storage and keying

The three thread-locals collapse into one per-task table inside `fs.rs`:

```rust
struct Slots { buffer: usize, array: usize, message: usize }

thread_local! {
    static SLOTS: RefCell<Vec<Slots>> = const { RefCell::new(Vec::new()) };
}
```

Indexed by **`id + 1`**, so **index 0 is the reserved no-task key**:

```rust
fn slot_index() -> usize {
    CURRENT.get().map_or(0, |id| id as usize + 1)
}
```

Task ids are dense from zero — `poll_one` reads `tasks.get(id as usize)` (`task.rs:414-419`) — so a
`Vec` gives O(1) access with no hashing and no per-call allocation, growing on demand. The table stays
`thread_local!` for the reason `task.rs`'s module doc gives for `TASKS` and `QUEUE`: the GC's roots are
per-thread, so a second thread running Nova code would free objects the first still holds.

**`CURRENT` is readable from an intrinsic, and this is the fact the design rests on.** `poll_one`
scopes its `TASKS` borrow to a closure and drops it *before* calling `poll` (`task.rs:414-424`), and
sets `CURRENT` for the duration of the call. So generated code calling an `fs` intrinsic mid-poll can
read `CURRENT` and borrow a side table without a borrow conflict.

**`BUFFER_SLOT` stays one field for `String` and `Bytes` both**, unchanged from its current rationale:
the two share the `{len, ptr}` representation, and which type it holds is carried entirely by which
builtin stashed it and which one reads it back.

**The no-task key is not dead weight.** `fs.rs`'s own Rust unit tests call the intrinsics directly with
no executor at all (`fs.rs:637`, `:677`, `:699`, `:723`), so index 0 is the path they exercise. Keying
absence rather than special-casing it means those tests survive the migration unchanged, and there is
one storage path rather than two.

## 4. Lifecycle and root discipline

`stash` and `take` keep their present contract, moved from a `&'static Cell` argument to a field of the
current task's `Slots`:

- **Root before publish.** `gc::add_root` precedes the store, so a scan between the two statements
  still reaches the object through the root table.
- **Release the displaced occupant.** Stashing over an occupied field releases what it evicts, via the
  same `take`. This is already the behaviour, added by the byte-type branch's final review.
- **Release on take.** `take` clears the field and calls `gc::remove_root`.

**What is new is a third release point.** A task's payload must not outlive the task, so `fs.rs`
exposes one function — `release_task_slots(id: i64)` — called from the two places `task.rs` already
releases a task's *state* root: `release_internal` (`task.rs:526`) and `take_output_internal`
(`task.rs:574`).

Those two are the correct hooks specifically because completion is **not** where a task's root is
released. `task.rs:288-291` records that deliberately: a spawned task's output has to outlive its
completion so a later `join` can take it. Hanging payload release on the same pair means payload
lifetime follows the policy `task.rs` already owns instead of introducing a second one, and it means
`struct Task` does not grow I/O fields — `fs.rs` keeps owning its boundary end to end.

**What this does not fix.** A task whose output is never taken and never released keeps its last
unread payload until the process exits. That is exactly the already-documented leak for such a task's
state (ADR 0009 §1: "a spawned task whose output is never taken leaks its state"), inherited rather
than added — this increment does not close it, and an earlier draft of this design claimed otherwise.
Today's leak is one payload per thread per slot kind; afterwards it is one per *leaked task* per kind.

**Panic safety.** No panic may cross a generated poll boundary — generated code has no landing pads.
Today's slots are `Cell`s and cannot panic; a `RefCell` can. Every access therefore uses
`try_borrow_mut`, falling back to `abort_with` (`task.rs:90`, `pub(crate)`, already used by
`bytes.rs`), which terminates without unwinding and is permitted where a panic is not. The abort is a
backstop for a state believed unreachable, not a routine path: `poll_one` holds no `TASKS` borrow
across `poll` — verified at `task.rs:414-424`, where the borrow is scoped to a closure that returns
before the call — and the migration must hold no `SLOTS` borrow across one either.

**One premise here is a requirement, not a measurement.** The backstop is only unreachable if no `fs`
intrinsic calls back into the executor while holding the borrow. That has not been checked across all
thirteen, so the implementation must confirm it rather than inherit it from this spec, and say which
intrinsics it read to do so.

**Confirmed (2026-08-13, Task 3): the premise holds.** All sixteen `nova_rt_fs_*` functions in
`crates/nova-runtime/src/fs.rs` were read: `nova_rt_fs_read_to_string`, `nova_rt_fs_write_string`,
`nova_rt_fs_take_string`, `nova_rt_fs_read`, `nova_rt_fs_write`, `nova_rt_fs_take_bytes`,
`nova_rt_fs_last_error_message`, `nova_rt_fs_temp_dir`, `nova_rt_fs_exists`, `nova_rt_fs_create_dir`,
`nova_rt_fs_create_dir_all`, `nova_rt_fs_remove_file`, `nova_rt_fs_remove_dir_all`,
`nova_rt_fs_read_dir`, `nova_rt_fs_take_string_array`, and `nova_rt_fs_kind`. Of these, thirteen touch
`SLOTS` at all (directly, or through `fail`/`stash_array`) — `temp_dir`, `exists` and `kind` never
touch a slot — which is exactly the count this paragraph's "thirteen" already named, read as
"intrinsics that touch the slot table" rather than "every `nova_rt_fs_*` function." None of the
sixteen calls into `task.rs` directly beyond `current_task`/`abort_with`.

One *indirect* path exists and was traced rather than assumed absent: `gc::alloc` (reached via
`gc_str`/`gc_message`/`crate::bytes::gc_bytes`/`stash_array`'s own block allocation) can trigger a
collection (`maybe_collect` → `collect` → `collect_with_roots`), which calls
`crate::task::forget_freed_state` — a call into `task.rs` beyond `current_task`/`abort_with`. It does
not violate the premise: every `gc::alloc` call reachable from this module's production code happens
either as an argument evaluated *before* `stash` is invoked, or (in `nova_rt_fs_take_string_array`'s
and `nova_rt_fs_take_bytes`'s empty-slot arms) strictly *after* `take` has already returned and
dropped its `with_slot` borrow. `with_slot`'s own closures — in `stash`, `take`, and
`release_task_slots`'s inline equivalent — never call `gc::alloc` themselves; they only read and write
a `Vec<Slots>` field, backed by the ordinary Rust allocator rather than the GC heap. So nothing that
can reach `collect_with_roots` ever runs while a `SLOTS` borrow is held, and the backstop stays a
backstop. `fs.rs`'s own `no_slot_access_can_panic_on_a_borrow` now pins the narrower, mechanical half
of this — every `SLOTS` access uses a fallible borrow — at its source, so this claim does not rely on
this reading staying accurate by itself as the file changes.

## 5. Testing

The defect this prevents needs a poller that does not exist, so the interleaving is constructed
directly in Rust — two tasks, a switch between stash and take:

1. Register two tasks. Set `CURRENT` to A; stash a payload.
2. Set `CURRENT` to B; stash a different payload.
3. Set `CURRENT` back to A; take.
4. Assert A reads **A's** payload.

**That test fails against today's thread-locals**, where step 2 overwrites step 1 and releases its
root. Being a real discriminator against the pre-change code is what earns it.

Also pinned:

- **The no-task key.** An intrinsic with `CURRENT == None` stashes and takes correctly, and does not
  collide with task 0's slots.
- **Root accounting**, via `gc::root_count(addr)` (`gc.rs:278`) — the pattern `fs.rs`'s tests already
  use at `:639`–`:694`. Per ADR 0010, a churn-loop test asserting an object *survives* a real
  collection cannot discriminate here; `root_count` proves bookkeeping, not survival, and the spec
  claims only that.
- **Release on the task hooks.** After `take_output_internal` or `release_internal`, an unread
  payload's root count returns to zero.
- **Overwrite release**, carried forward from the existing tests: stashing twice into one task's field
  leaves exactly one root.

Every existing `fs` test must pass unchanged. The suite is 883/0/8 across 44 targets and the new tests
add to it; the count must rise by exactly the number added.

## 6. Alternatives considered

**Payload fields on `struct Task`.** Automatic per-task lifetime and one source of truth, but it puts
I/O payload GC roots into `task.rs`, inverting the layering in which `fs.rs` owns its protocol, and
every later payload kind — `File` handles, sockets — widens that struct. Declined.

**Thread `task_ctx` through the ABI.** The reserved parameter's evident purpose, and it would remove
the `CURRENT` lookup entirely. Declined on cost, not merit: ADR 0009 documents `task_ctx` as always
null, generated code does not pass it to intrinsic calls, and making it do so is a change in both
codegen backends. If a later increment needs per-task context in generated code for other reasons,
this becomes the right answer and §3's table becomes its first consumer.

**Keep thread-locals, add per-task storage alongside.** Nothing existing changes, at the cost of two
mechanisms permanently and the clobber-prone path left in the tree for someone to reach for. Declined.

## 7. Risks

| Risk | Mitigation |
|---|---|
| A `RefCell` borrow conflict aborts a running program | `try_borrow_mut` + `abort_with`, and no access holds a borrow across a call |
| An unbounded `Vec` grows with task ids | Ids are dense and per-thread; three `usize` per task |
| The migration silently keeps thread-global behaviour | The step-1-to-4 interleaving test fails against the pre-change code |
| A release point is missed, leaking roots | `root_count` assertions on both hooks, plus the existing overwrite test |
| `CURRENT` is `None` in a path believed to be inside a poll | Keyed, not special-cased, so it is correct rather than fatal |

## 8. Definition of done

- Three thread-local slots replaced by one per-task table in `fs.rs`, keyed on `CURRENT` with index 0
  reserved for no task.
- `release_task_slots` called from `release_internal` and `take_output_internal`.
- Every table access panic-free by construction, with an `abort_with` backstop.
- The cross-task interleaving test present and demonstrated to fail against pre-change code.
- No Nova-visible signature changed; `std/fs/lib.nova` untouched.
- `cargo build --workspace` before `cargo test --workspace --no-fail-fast`; suite green with the count
  risen by exactly the tests added; clippy `-D warnings` and `cargo fmt --all --check` clean; the 8
  ADR-0010 ignored tests still ignored and untouched.
- `CHANGELOG.md` updated. No user-visible behaviour changes, so nothing belongs under `### Added` —
  **verify that against the heading's own stated scope rather than assuming it.**
