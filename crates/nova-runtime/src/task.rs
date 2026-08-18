//! A single-threaded cooperative executor.
//!
//! **Single-threaded is a correctness requirement, not a simplification.**
//! The collector keeps its entire heap in a `thread_local!` (`gc`'s `HEAP`),
//! so an object allocated on one thread lives in that thread's heap map and
//! only that thread's collector can see or free it -- `gc::add_root` and
//! `gc::remove_root` are `thread_local!` for the identical reason (see their
//! doc comments in `gc.rs`). A second thread running Nova code would free
//! objects the first still holds. This executor's own state (`QUEUE`,
//! `TASKS`, `IN_BLOCK_ON` below) is therefore `thread_local!` too, by
//! construction rather than by convention: there is no API here for a task
//! to migrate between threads. See ADR 0009.
//!
//! No wakers: a task that returns [`POLL_PENDING`] is re-queued for another
//! turn unless it staged a park (see [`Wait`]), in which case it waits in the
//! park set instead until a deadline or another task's completion -- the
//! executor's only two wake sources -- moves it back. Both are scheduled by
//! the executor itself, not registered as an arbitrary callback the awaited
//! resource invokes, which is what "no wakers" still means once parking
//! exists. That makes interleaving between ready tasks deterministic by
//! construction, which is what lets [`nova_rt_task_block_on`]'s round-robin
//! order be pinned by a test rather than merely observed.

use crate::gc;
use crate::poll::{Interest, RawSocket};
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant};

/// The signature every compiled `async fn`'s poll function has -- both the
/// hand-written ones this module's tests drive it with, and the ones
/// `async_lower.rs` (Task 5) generates. Reads and writes `state`'s slots (see
/// [`STATE_SLOT_TAG`] and friends) and reports [`POLL_PENDING`] or
/// [`POLL_READY`].
///
/// `task_ctx` is unused in this phase, and every call site in this module
/// passes null. The parameter exists now, rather than being added later,
/// because a future phase (channels, timers) needs a task to be able to park
/// against an external event instead of being busy-repolled, and adding a
/// parameter afterward would mean changing every already-compiled poll
/// function's signature.
///
/// `"C-unwind"`, not plain `"C"`: a panic cannot cross a plain `"C"`
/// function's boundary at all -- it aborts the whole process on the spot,
/// regardless of the workspace's unwind panic strategy. Some of this module's
/// diagnostics are panics that a caller is meant to be able to observe --
/// [`nova_rt_task_take_output`]'s take-once check, and [`poll_one`]'s
/// rejection of a poll status the ABI does not define -- and each has to leave
/// the `#[no_mangle]` entry point it starts under. Declaring the ABI
/// `"C-unwind"` permits that, and every entry point here (this indirect call
/// included) shares it so this module has one ABI rather than two. It changes
/// nothing else: `"C-unwind"` was stabilized in Rust 1.71 (under this
/// workspace's 1.78 MSRV) and marshals parameters and return values
/// identically to `"C"`, so a generated call site is byte-identical either way.
///
/// **Constraint on generated poll functions: a panic must not cross their
/// boundary.** `"C-unwind"` says an unwind *may* pass through without
/// aborting; it does not make a frame able to survive one. A poll function
/// emitted by Cranelift or LLVM has no landing pads and no drop glue, so an
/// unwind through it would skip whatever cleanup its frame ought to do and
/// leave the executor's `QUEUE`/`TASKS` describing a task that is neither
/// running nor finished. Anything in a generated poll function that could
/// panic must therefore either be provably unable to (`nova_rt_check_bounds`
/// and `nova_rt_panic_str` both `abort`, not unwind) or catch it before
/// returning. Every diagnostic in this module that a *well-formed compiled
/// Nova program* can reach therefore goes through [`abort_with`] instead of
/// panicking; the panics named above are reachable only from a Rust caller or
/// from a compiler bug (see each one's own doc comment). The
/// permission this ABI grants exists for the Rust-side entry points below, not
/// for compiled Nova frames.
pub type PollFn = unsafe extern "C-unwind" fn(state: *mut u8, task_ctx: *mut u8) -> i64;

/// Report a violated contract and end the process, in the same style and with
/// the same `nova: panic:` prefix as `nova_rt_panic_str` and
/// `nova_rt_check_bounds`.
///
/// Aborting rather than panicking is what makes a check callable from compiled
/// Nova code: a generated frame has no landing pads and no drop glue, and no
/// unwind table describing it, so an unwind that started under one and passed
/// through it would have to be resolved by an unwinder that has no description
/// of that frame (see [`PollFn`]'s doc comment).
///
/// `pub(crate)`, not private: `bytes`'s `nova_rt_bytes_at` and
/// `nova_rt_bytes_from_ints` (Task 3) call this too, for the same reason this
/// module does -- both are reachable from inside a generated poll frame, so a
/// rejected caller error must abort rather than unwind. This is still the
/// *only* place that prints the `nova: panic:` marker and aborts on this path
/// (ADR 0008's emitter inventory); a second helper doing the same thing would
/// be a fifth emitter to track there, whereas a second module merely calling
/// this one is not.
pub(crate) fn abort_with(msg: &str) -> ! {
    eprintln!("nova: panic: {msg}");
    std::process::abort();
}

/// A [`PollFn`] returns this when it has not yet produced a value; the
/// executor re-queues the task for another turn.
pub const POLL_PENDING: i64 = 0;
/// A [`PollFn`] returns this once it has written the final value to the
/// state object's output slot ([`STATE_SLOT_OUTPUT`]).
pub const POLL_READY: i64 = 1;

/// The state object's layout, word-indexed: slot `n` is at byte offset
/// `8 * n`. `async_lower.rs` (Task 5) must build exactly this layout -- this
/// executor and every generated poll function address it by raw offset, with
/// no shared Rust type to keep the two ends in sync if one of them drifts.
///
/// - [`STATE_SLOT_TAG`] (offset 0): the resume tag. A poll function's entry
///   block switches on this to pick up wherever it last suspended.
/// - [`STATE_SLOT_OUTPUT`] (offset 8): the output value, written exactly
///   once, by the call that returns [`POLL_READY`].
/// - [`STATE_SLOT_TEMPS`] (offset 16) and up: one slot per MIR temp in the
///   `async fn` body, unconditionally -- every local is spilled here whether
///   or not it is live across a suspend point, so no local is ever live in a
///   register or on the stack across one.
pub const STATE_SLOT_TAG: usize = 0;
pub const STATE_SLOT_OUTPUT: usize = 1;
pub const STATE_SLOT_TEMPS: usize = 2;

/// The future value's own layout -- the `{ poll_code, state }` two-word fat
/// pointer every `nova_rt_task_spawn` / `nova_rt_task_block_on` argument
/// points at, the same shape `MakeClosure` builds for a closure. Named for
/// the same reason [`STATE_SLOT_TAG`] and friends are: `async_lower.rs`
/// (Task 5) builds this layout independently and the two ends are kept in
/// step by nothing but these declared offsets.
///
/// - [`FUTURE_SLOT_POLL`] (offset 0): the [`PollFn`]'s address.
/// - [`FUTURE_SLOT_STATE`] (offset 8): the state object's address.
pub const FUTURE_SLOT_POLL: usize = 0;
pub const FUTURE_SLOT_STATE: usize = 1;
/// The whole future value's size in bytes, for the allocation that holds it.
pub const FUTURE_SIZE: usize = 16;

/// The smallest state object any future may have: the tag and output slots
/// are read and written unconditionally, for every future, including one
/// whose `async fn` returns unit and has no temps at all. A state object
/// with `n` MIR temps is `(STATE_SLOT_TEMPS + n) * 8` bytes, which is never
/// smaller than this.
pub const STATE_MIN_SIZE: usize = (STATE_SLOT_TEMPS) * 8;

/// One task known to this thread's executor.
struct Task {
    poll: PollFn,
    state: *mut u8,
    done: bool,
    output: i64,
    /// Whether this task's GC root has already been released, by either
    /// [`take_output_internal`] or [`release_internal`]. The root is one
    /// `gc::add_root`, so it must be cancelled at most once; this flag is what
    /// both paths check, which is also what makes `release_internal`
    /// idempotent.
    taken: bool,
}

/// Why a parked task is waiting, and therefore what wakes it.
///
/// `Copy`, and staged through a `Cell` rather than a `RefCell`, deliberately:
/// it is written from inside a poll frame, and a `RefCell` borrow panic there
/// would unwind through generated code that has no landing pads (see
/// [`PollFn`]'s doc comment).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum Wait {
    /// Wake once the clock reaches this instant.
    Deadline(Instant),
    /// Wake once the task with this id completes, or once `deadline` passes.
    ///
    /// The deadline rides inside this variant for the same reason it rides
    /// inside [`Wait::Io`]: one task must have exactly one `PARKED` entry, or
    /// every wake path has to remember to remove two.
    Task { id: i64, deadline: Option<Instant> },
    /// Wake once `socket` is ready for `interest`, or `deadline` passes.
    ///
    /// The deadline rides inside this variant rather than being parked as a
    /// second entry: one task must have exactly one `PARKED` entry, or every
    /// wake path has to remember to remove two.
    Io {
        socket: RawSocket,
        interest: Interest,
        deadline: Option<Instant>,
    },
}

thread_local! {
    /// Ids ready for another turn, FIFO: pushed at the back, popped from the
    /// front, so a re-queued task's next poll always waits behind every
    /// other task that was already waiting -- round-robin, not
    /// last-in-first-out.
    static QUEUE: RefCell<VecDeque<i64>> = const { RefCell::new(VecDeque::new()) };
    /// Every task this thread has ever spawned, indexed by id (its index at
    /// spawn time). Entries are never removed: a finished task stays here,
    /// `done` and holding its output, since `nova_rt_task_is_done` /
    /// `nova_rt_task_take_output` can be called an arbitrary time later. A
    /// plain `Vec<Task>`, not `Vec<Option<Task>>`: nothing ever vacates a
    /// slot, so an `Option` here would add an arm no code path can reach.
    /// An out-of-range id is still rejected, by `Vec::get` returning `None`.
    static TASKS: RefCell<Vec<Task>> = const { RefCell::new(Vec::new()) };
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
    /// **An entry lasts exactly as long as the object its key names.** An
    /// address is not a durable identity for a heap object (`gc.rs`'s module
    /// doc comment states that property and owns it), so a key that outlived
    /// its state object would go on naming the old task for whatever
    /// unrelated object the collector next placed at that address, and the
    /// two reads below would answer about that dead task instead of rejecting
    /// a future the executor never saw. `spawn_internal` inserts;
    /// [`forget_freed_state`] removes, at the moment the address stops
    /// meaning anything.
    ///
    /// Two properties follow, and both are load-bearing elsewhere in this
    /// module. A key is present only for a state object that is still the
    /// allocation it was spawned with, so the reads need no staleness test of
    /// their own (see [`task_id_of`]) and `spawn_internal`'s check is about
    /// something else entirely (see its own comment). And a state a caller
    /// can still reach is never swept, so it is never pruned -- which is what
    /// makes a second `join` on a handle that still holds its future resolve
    /// to the same task as the first.
    static BY_STATE: RefCell<HashMap<usize, i64>> = RefCell::new(HashMap::new());
    /// Set for the duration of one `nova_rt_task_block_on` call, so a nested
    /// call (a poll function calling `block_on` again) can be diagnosed
    /// instead of running a second executor loop from inside the first
    /// one's frame and corrupting the shared queue.
    static IN_BLOCK_ON: Cell<bool> = const { Cell::new(false) };
    /// Tasks that are waiting on something, and what for. **Disjoint from
    /// `QUEUE` by construction:** an entry arrives only by `poll_one` declining
    /// to re-queue, and leaves only by being pushed back onto `QUEUE`. Keyed on
    /// the task id -- this executor's own private identity -- and deliberately
    /// not on a state address: `BY_STATE` needs pruning at the GC sweep only
    /// because its keys are heap addresses a Nova value names, and a second
    /// address-keyed map would inherit that hazard for nothing.
    static PARKED: RefCell<Vec<(i64, Wait)>> = const { RefCell::new(Vec::new()) };
    /// The task `poll_one` is polling right now, so a park staged from inside
    /// that poll knows whose it is without `task_ctx` having to carry it.
    static CURRENT: Cell<Option<i64>> = const { Cell::new(None) };
    /// What the poll in progress has staged so far -- at most one deadline
    /// and at most one I/O wait (see [`Staged`]), folded by
    /// [`staged_to_wait`] into the single [`Wait`] `poll_one` reads once the
    /// status is known: committed on `POLL_PENDING`, discarded on
    /// `POLL_READY`.
    static PENDING_PARK: Cell<Staged> = const {
        Cell::new(Staged {
            deadline: None,
            io: None,
            task: None,
        })
    };
}

/// The task currently being polled, or `None` outside a poll.
///
/// `poll_one` sets this to `Some(id)` immediately before calling that task's
/// poll function, and back to `None` immediately after it returns -- scoped
/// to exactly the duration of that one `poll` call -- so a runtime intrinsic
/// reached from generated code can key per-task storage on it. `fs.rs` is the
/// first consumer.
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

/// Test-only: the I/O half of whatever this thread's current poll has staged
/// so far in [`PENDING_PARK`] -- the socket, its interest, and the deadline
/// (if any) riding alongside it -- without exposing [`Wait`] or [`Staged`]
/// themselves. `None` if nothing with a socket is staged (nothing at all, or
/// only a bare `Wait::Task`/`Wait::Deadline`, neither of which has one).
///
/// `net.rs`'s own tests need this to assert that a future's would-block
/// branch genuinely staged a park with the *expected* socket and interest --
/// not merely that `POLL_PENDING` came back, which a future that stages
/// nothing at all and busy-spins through `QUEUE` would also return. Reading
/// [`PENDING_PARK`] here does not widen what `net.rs` can construct or match:
/// `Wait` and `stage_park` stay private to this module, and this hands back
/// plain data extracted from `Staged`, which is a different, already-`Copy`
/// type nothing outside this module can build a fake instance of.
#[cfg(test)]
pub(crate) fn staged_io_for_test() -> Option<(RawSocket, Interest, Option<Instant>)> {
    PENDING_PARK.with(|cell| {
        let staged = cell.get();
        staged
            .io
            .map(|(socket, interest)| (socket, interest, staged.deadline))
    })
}

/// The deadline currently staged by the poll in progress, if any.
///
/// The deadline counterpart of [`staged_io_for_test`], and it exists for the
/// same reason: an outcome test cannot tell a park from a busy spin, because
/// both complete. Before this, nothing could observe a staged deadline at all
/// -- so a `sleep` whose unit conversion was wrong by a factor of a million
/// still passed `tests/runtime/task_sleep_order.nova`, which only pins the
/// order of a 200 and a 20.
#[cfg(test)]
pub(crate) fn staged_deadline_for_test() -> Option<Instant> {
    PENDING_PARK.with(|cell| cell.get().deadline)
}

/// The task wait staged by the poll in progress, if any -- the third and last
/// of [`Staged`]'s fields, completing the set with [`staged_io_for_test`] and
/// [`staged_deadline_for_test`].
///
/// Added because with only two of the three readable, no test could assert
/// that the slot is **empty**, and "empty" is exactly the postcondition
/// [`poll_timeout`]'s abandonment path has to establish: an abandoned inner's
/// leftover [`Wait::Task`] was invisible to every accessor that existed, which
/// is why a whole class of two-parks-in-one-poll aborts shipped green. An
/// existence check on the other two fields cannot substitute -- the leftover
/// this closes is a `task`, and a `Deadline` merges rather than colliding, so
/// `task` is the one field whose staleness is both invisible and fatal.
#[cfg(test)]
pub(crate) fn staged_task_for_test() -> Option<i64> {
    PENDING_PARK.with(|cell| cell.get().task)
}

/// Forget the [`BY_STATE`] entry for the state object at `addr`, whose memory
/// the collector has just returned to the allocator.
///
/// The removal half of that map's own invariant, and the reason `gc.rs` calls
/// in here rather than this module watching for it: a freed address can be
/// reissued for an unrelated object (`gc.rs`'s module doc comment), and the
/// sweep is where that transition happens, so it is the only place a key can
/// be dropped before it starts naming the wrong thing.
///
/// **Removal only.** Nothing here inserts a key, and nothing here touches
/// `TASKS`, so this cannot introduce a key whose id `TASKS` does not have --
/// `spawn_internal` is still the only thing that puts a key in this map or an
/// entry in `TASKS`, and it takes the id from `TASKS` itself.
///
/// An address that never was a task's state simply misses: this map is keyed
/// on state objects, and a sweep frees every kind of heap object.
pub(crate) fn forget_freed_state(addr: usize) {
    BY_STATE.with(|m| {
        m.borrow_mut().remove(&addr);
    });
}

/// Read the `{ poll_code, state }` fat pointer the compiler builds for a
/// `Future` value (the same two-word shape `MakeClosure` builds for a
/// closure).
///
/// # Safety
/// `future` must point to a live, readable [`FUTURE_SIZE`]-byte block:
/// [`FUTURE_SLOT_POLL`] a [`PollFn`] (as a bit pattern), [`FUTURE_SLOT_STATE`]
/// the address of a state object of at least [`STATE_MIN_SIZE`] bytes (see
/// [`nova_rt_task_spawn`]'s contract for the full requirement).
unsafe fn read_future(future: *mut u8) -> (PollFn, *mut u8) {
    let words = future as *mut usize;
    // SAFETY: `future` is a valid `FUTURE_SIZE`-byte block per this
    // function's contract.
    let poll_code = unsafe { words.add(FUTURE_SLOT_POLL).read() };
    // SAFETY: same block, second word.
    let state = unsafe { words.add(FUTURE_SLOT_STATE).read() } as *mut u8;
    // SAFETY: `poll_code` is a `PollFn` bit pattern by `future`'s own
    // contract above; a function pointer and a `usize` are both
    // pointer-width, so the transmute cannot change size. The load-bearing
    // half of that contract is *non-nullability*: `PollFn` is a bare `fn`
    // pointer, not `Option<fn>`, so it has a niche at zero and transmuting a
    // zero word here is instantly undefined behaviour rather than a pointer
    // that merely faults when called. A future value whose first word is
    // still zero is exactly what a Task 5/6 bug that allocates the fat
    // pointer but never writes `poll_code` produces, and `gc::alloc` hands
    // back zeroed memory -- so this precondition is on the caller, and there
    // is no check here that could recover from its violation.
    let poll: PollFn = unsafe { std::mem::transmute::<usize, PollFn>(poll_code) };
    (poll, state)
}

/// Register `future` as a new task: read its fat pointer, root its state
/// object, enqueue it, and return its id.
///
/// The `gc::add_root` here is paired with exactly one `gc::remove_root` in
/// [`take_output_internal`] -- **not** at completion. See [`poll_one`] for why
/// the root has to outlive completion, and [`take_output_internal`] for what
/// that ordering costs. This module owns that policy; `gc.rs` states only the
/// registry's own multiset contract and deliberately not who pairs it or when.
///
/// # Safety
/// `future` must be a valid future fat pointer (see [`read_future`]).
unsafe fn spawn_internal(future: *mut u8) -> i64 {
    // SAFETY: forwarding this function's own contract.
    let (poll, state) = unsafe { read_future(future) };
    // Two LIVE tasks sharing one state object would make this map ambiguous,
    // so the lookup forces a decision here. A key exists only for a state
    // object that is still the allocation it was spawned with (`BY_STATE`'s
    // doc comment), so the question this hit asks is not "has this address
    // been used before" but "is this the same future again".
    //
    // Answered on liveness rather than on presence, because re-spawning a
    // future whose task has been *released* is deliberately allowed. It could
    // be refused: "released but still held" and "freed and recycled" are
    // distinguishable here, since the second no longer reaches this branch at
    // all. But a released future its caller still holds is the
    // `spawn(h.fut)`-after-`join` shape, and Nova has no move checking, so
    // `join` cannot consume the handle that makes it expressible. It is
    // accepted for the same reason the double-await footgun is (ADR 0009):
    // the second spawn re-polls a completed state machine from its last
    // suspend point, re-running the body after its final await, which is a
    // wrong answer confined to the caller that asked for it rather than
    // corruption of a task that did nothing. What must stay rejected is two
    // live tasks driving one state object, and `Task::taken` is exactly that
    // distinction: it is set only where the executor gives up its GC root, so
    // an unset flag means this state is still a task the executor is driving.
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
    // The state object sits on no Nova stack and in no register while its
    // task is queued or parked -- the executor is its only reference, and
    // those are the collector's only other root sources (see `gc.rs`'s
    // module doc comment). Paired with exactly one `gc::remove_root`, in
    // `take_output_internal`; see its doc comment for why completion is not
    // where the root is released.
    gc::add_root(state);
    let id = TASKS.with(|tasks| {
        let mut tasks = tasks.borrow_mut();
        let id = tasks.len() as i64;
        tasks.push(Task {
            poll,
            state,
            done: false,
            output: 0,
            taken: false,
        });
        id
    });
    BY_STATE.with(|m| m.borrow_mut().insert(state as usize, id));
    QUEUE.with(|queue| queue.borrow_mut().push_back(id));
    id
}

/// A poll may stage at most one deadline and at most one I/O wait.
///
/// Two of the *same* kind still aborts: that abort is what catches an inner
/// future's `POLL_PENDING` failing to propagate, and it must keep doing so.
/// Only the one legitimate combination -- a deadline and an I/O wait, from a
/// single `read_timeout` -- becomes newly legal, and [`staged_to_wait`] is
/// where the two are folded into the single [`Wait::Io`] entry [`PARKED`]
/// actually holds.
///
/// `task` is not in the brief's own sketch of this struct, which only names
/// the two kinds that newly compose. It is here because [`Wait::Task`]
/// (`poll_join`'s wait) still has to be exclusive with a second task wait or
/// with an I/O wait -- nothing about waiting on a sibling task composes with
/// a socket the way a deadline and an I/O wait do, so a `Task` park stacked
/// against another `Task` or an `Io` one still aborts exactly as any two
/// same-kind parks do. A `Deadline`, though, composes with a `Task` wait the
/// same way it composes with an `Io` one: `timeout(d, handle.join())` stages
/// exactly that pair, and [`try_stage`] merges rather than collides.
#[derive(Default, Clone, Copy, Debug)]
struct Staged {
    deadline: Option<Instant>,
    io: Option<(RawSocket, Interest)>,
    task: Option<i64>,
}

/// Fold everything staged so far back into the single [`Wait`] [`PARKED`]
/// holds, or `None` if nothing was staged this poll.
///
/// One task must have exactly one `PARKED` entry ([`Wait::Io`]'s own doc
/// comment), so this is where a deadline staged alongside an I/O wait is
/// folded into that `Wait::Io`'s own `deadline` field instead of becoming a
/// second entry. `task` still wins first, but it no longer wins by exclusion:
/// [`try_stage`] now lets a deadline coexist with it, so this folds
/// `staged.deadline` into the returned [`Wait::Task`]'s own `deadline` field,
/// exactly as the `io` branch below already does for `Wait::Io`.
fn staged_to_wait(staged: Staged) -> Option<Wait> {
    if let Some(id) = staged.task {
        return Some(Wait::Task {
            id,
            deadline: staged.deadline,
        });
    }
    if let Some((socket, interest)) = staged.io {
        return Some(Wait::Io {
            socket,
            interest,
            deadline: staged.deadline,
        });
    }
    staged.deadline.map(Wait::Deadline)
}

/// Try to add `wait` to `staged`, or report what it collided with.
///
/// A deadline never collides here: staging a second [`Wait::Deadline`], or
/// one alongside a [`Wait::Task`] or inside a [`Wait::Io`] that already
/// carries one, merges to the earlier of the two by [`Instant::min`] instead
/// of erroring -- see [`Staged`]'s own doc comment for why `task` and `io`
/// still exclude each other and themselves even though a deadline now
/// composes with either. What still collides is a same-kind clash (two
/// `Wait::Io`s, two `Wait::Task`s) or a `Wait::Task` crossing a `Wait::Io`.
///
/// Pure and non-aborting on purpose, unlike [`stage_park`] itself: a test
/// exercising the actual collision would have to go through
/// [`abort_with`]'s `std::process::abort()`, which `#[should_panic]`'s
/// `catch_unwind` cannot intercept -- an aborting test would take the whole
/// test binary down with it, not just fail. This is the half a test can call
/// directly to check the collision is detected, the same way
/// [`deadlock_report`] is the half a test can call to check
/// [`report_deadlock`]'s text without ending the process.
///
/// Every collision below extracts the specific `Wait` it collided with
/// directly out of whichever `Staged` field an `if let Some(_)` just proved
/// is populated, and passes that straight to [`collision_msg`] -- never
/// through [`staged_to_wait`], which would need to be asked to resolve a
/// `Staged` that might have nothing in it. `stage_park` is called from
/// inside a poll function, so nothing in this call chain may panic (see its
/// own doc comment): an `.expect()` provably unreachable *today* is still a
/// panic that could cross a poll boundary the day this function's logic
/// changed under it without that assumption being re-checked, so the
/// unreachable case is written out of existence here rather than asserted
/// away.
///
/// Exhaustive on `wait` with no wildcard, like every other match on `Wait`
/// in this module: a fourth variant must be considered here too.
fn try_stage(staged: Staged, wait: Wait) -> Result<Staged, String> {
    let mut next = staged;
    match wait {
        Wait::Deadline(at) => {
            next.deadline = Some(match next.deadline {
                Some(prev) => prev.min(at),
                None => at,
            });
        }
        Wait::Io {
            socket,
            interest,
            deadline,
        } => {
            if let Some((prev_socket, prev_interest)) = next.io {
                return Err(collision_msg(
                    Wait::Io {
                        socket: prev_socket,
                        interest: prev_interest,
                        deadline: next.deadline,
                    },
                    wait,
                ));
            }
            if let Some(prev_id) = next.task {
                return Err(collision_msg(
                    Wait::Task {
                        id: prev_id,
                        deadline: next.deadline,
                    },
                    wait,
                ));
            }
            if let Some(at) = deadline {
                next.deadline = Some(match next.deadline {
                    Some(prev) => prev.min(at),
                    None => at,
                });
            }
            next.io = Some((socket, interest));
        }
        Wait::Task { id, deadline } => {
            if let Some(prev_id) = next.task {
                return Err(collision_msg(
                    Wait::Task {
                        id: prev_id,
                        deadline: next.deadline,
                    },
                    wait,
                ));
            }
            if let Some((prev_socket, prev_interest)) = next.io {
                return Err(collision_msg(
                    Wait::Io {
                        socket: prev_socket,
                        interest: prev_interest,
                        deadline: next.deadline,
                    },
                    wait,
                ));
            }
            if let Some(at) = deadline {
                next.deadline = Some(match next.deadline {
                    Some(prev) => prev.min(at),
                    None => at,
                });
            }
            next.task = Some(id);
        }
    }
    Ok(next)
}

/// The "two parks staged" message [`stage_park`] aborts with.
///
/// Takes the colliding `Wait` values directly, not a [`Staged`] that might
/// resolve to nothing: see [`try_stage`]'s doc comment for why that
/// distinction is the fix, not a stylistic preference.
fn collision_msg(previous: Wait, wait: Wait) -> String {
    format!(
        "nova_rt: two parks staged in one poll ({previous:?} then {wait:?}); \
         an inner future's POLL_PENDING did not propagate"
    )
}

/// Record that the task currently being polled wants to park on `wait`.
///
/// Called from inside a poll function -- [`poll_sleep`], [`poll_join`], and
/// this module's own tests -- so it must not panic: [`Staged`] is `Copy` and
/// held in a `Cell`, so neither borrow can fail. The two aborts below are
/// compiler-or-runtime bugs rather than user error, which is why they end the
/// process rather than returning a status.
fn stage_park(wait: Wait) {
    if CURRENT.with(Cell::get).is_none() {
        abort_with("nova_rt: a park was staged outside a poll");
    }
    PENDING_PARK.with(|cell| match try_stage(cell.get(), wait) {
        Ok(next) => cell.set(next),
        Err(msg) => abort_with(&msg),
    });
}

/// Stage an I/O park: wake the current task once `socket` is ready for
/// `interest`, or once `deadline` passes, whichever comes first.
///
/// The narrow seam `net.rs` stages an I/O wait through, so it never needs to
/// construct a [`Wait`] itself. `Wait` and [`stage_park`] stay private --
/// every exhaustive match on `Wait` lives in this module, so a fifth variant
/// is a compile error here rather than a silent miss in a sibling module that
/// matched on it.
///
/// `net.rs` calls this from four production sites: `poll_connect`
/// (`Interest::Write`), `poll_read` (`Interest::Read`), `poll_write`
/// (`Interest::Write`) and `poll_read_timeout` (`Interest::Read`, and the only
/// one of the four that passes a `deadline`). An earlier `#[allow(dead_code)]`
/// here recorded that no caller existed yet; it has been removed with its
/// reason rather than left to mask a real dead-code finding.
pub(crate) fn stage_io_park(socket: RawSocket, interest: Interest, deadline: Option<Instant>) {
    stage_park(Wait::Io {
        socket,
        interest,
        deadline,
    });
}

/// Poll task `id` once. On [`POLL_PENDING`], re-queue it for another turn --
/// or, if the poll staged a park via [`stage_park`], move it to [`PARKED`]
/// instead. On [`POLL_READY`], copy its output out of the state object into
/// the `Task` record, mark it done, and wake anything parked on it (see
/// [`wake_tasks_waiting_on`]).
///
/// **The state object's GC root is deliberately *not* released here.** The
/// output value has been copied into `Task::output`, which lives in `TASKS` --
/// a Rust-heap `Vec`, which is none of the collector's root sources (stack,
/// callee-saved registers, `PINNED`, scanned GC objects; see `gc.rs`'s module
/// doc comment). A copy there therefore roots nothing. Since the runtime
/// cannot tell a scalar output from a pointer output -- both are `i64` --
/// the root has to be conservative, and the state object already is one: the
/// output also still sits in the state object's own [`STATE_SLOT_OUTPUT`],
/// the state object is allocated scanned, and a `PINNED` root is traced
/// transitively (`gc.rs`'s `a_registered_root_keeps_its_transitive_children_alive`).
/// So keeping the state rooted until the output is taken keeps a heap-valued
/// output alive with no new mechanism and no scalar/pointer discrimination.
/// [`take_output_internal`] is where the single matching `gc::remove_root`
/// lives.
///
/// Any status that is neither [`POLL_PENDING`] nor [`POLL_READY`] panics
/// rather than being treated as completion. This module exists to pin the
/// poll ABI before codegen depends on it; a generated poll function returning
/// `2`, or garbage, is a Task 5 codegen bug, and completing the task with
/// whatever happens to be in the output slot would turn it into a wrong
/// answer instead of a diagnostic.
///
/// A poll function that panics leaves its task `done == false` and its state
/// permanently rooted -- a leak, on a path that is already unwinding out
/// through [`nova_rt_task_block_on`]. Not resolved here: cleaning up would
/// mean a `catch_unwind` per poll, which would swallow exactly the panic
/// `a_re_entrant_block_on_panics` needs to observe.
///
/// # Safety
/// `id` must be a currently-registered task id.
unsafe fn poll_one(id: i64) {
    let (poll, state) = TASKS.with(|tasks| {
        let tasks = tasks.borrow();
        let task = tasks
            .get(id as usize)
            .expect("poll_one: task id is not registered");
        (task.poll, task.state)
    });
    CURRENT.with(|c| c.set(Some(id)));
    // SAFETY: unchanged from before -- `poll`/`state` came from a `Task` this
    // module built in `spawn_internal`; `task_ctx` is still always null.
    let status = unsafe { poll(state, std::ptr::null_mut()) };
    CURRENT.with(|c| c.set(None));
    // Taken unconditionally, which is what discards a park staged by a poll
    // that then returned `POLL_READY`. Leaving it staged would park the next
    // task polled; committing it would leave a finished task in `PARKED`,
    // faking a deadlock for the rest of the process.
    let staged = staged_to_wait(PENDING_PARK.with(|p| p.take()));
    if status == POLL_PENDING {
        match staged {
            Some(wait) => PARKED.with(|parked| parked.borrow_mut().push((id, wait))),
            None => QUEUE.with(|queue| queue.borrow_mut().push_back(id)),
        }
        return;
    }
    // Panics rather than aborting, unlike every other diagnostic here that a
    // Nova program can reach: this one cannot be reached by a well-formed
    // compiled program at all. A generated poll function's every exit returns
    // `POLL_PENDING` or `POLL_READY` (`async_lower.rs` emits them as
    // `ConstInt`s), so an out-of-range status means the compiler is broken,
    // and the panic's observability is what lets
    // `an_out_of_range_poll_status_panics_rather_than_completing_the_task`
    // assert on the value. The unwind leaves through
    // `nova_rt_task_block_on`'s `resume_unwind`, so it can cross a generated
    // frame -- on a path whose precondition is already a compiler bug.
    if status != POLL_READY {
        panic!(
            "poll function for task {id} returned {status}, which is neither \
             POLL_PENDING ({POLL_PENDING}) nor POLL_READY ({POLL_READY})"
        );
    }
    // SAFETY: `state` is the same live state object `spawn_internal` rooted;
    // `STATE_SLOT_OUTPUT` is within its bounds regardless of how many temp
    // slots follow it (see `nova_rt_task_spawn`'s contract, which requires at
    // least `STATE_MIN_SIZE` bytes for exactly this read).
    let output = unsafe { (state as *mut i64).add(STATE_SLOT_OUTPUT).read() };
    TASKS.with(|tasks| {
        let mut tasks = tasks.borrow_mut();
        let task = tasks
            .get_mut(id as usize)
            .expect("poll_one: task id is not registered");
        task.output = output;
        task.done = true;
    });
    wake_tasks_waiting_on(id);
}

/// Move every task parked on `done_id` back onto the ready queue.
///
/// Called from `poll_one` the moment a task is marked done, which is what
/// makes `Wait::Task` a complete wake source: a task cannot finish by any
/// other route.
fn wake_tasks_waiting_on(done_id: i64) {
    let woken = PARKED.with(|parked| {
        let mut parked = parked.borrow_mut();
        let mut woken = Vec::new();
        parked.retain(|&(id, wait)| match wait {
            Wait::Task { id: target, .. } if target == done_id => {
                woken.push(id);
                false
            }
            _ => true,
        });
        woken
    });
    QUEUE.with(|queue| {
        let mut queue = queue.borrow_mut();
        for id in woken {
            queue.push_back(id);
        }
    });
}

/// Whether task `id` has completed. Aborts on an id no `spawn_internal` ever
/// handed out.
///
/// `id` is this executor's own internal identity, not something a Nova
/// program can name directly: [`nova_rt_task_is_done`] resolves a future to
/// one through `BY_STATE` before reaching here, so this abort now guards a
/// bug in this module rather than a forged caller value.
fn is_done_internal(id: i64) -> bool {
    TASKS.with(|tasks| match tasks.borrow().get(id as usize) {
        Some(task) => task.done,
        None => abort_with(&format!("nova_rt_task_is_done: unknown task id {id}")),
    })
}

/// Release task `id`'s state-object root, handing nothing back. Idempotent.
///
/// This is [`take_output_internal`] with the take removed, and the two are not
/// interchangeable in either direction. That one's second call is diagnosed
/// precisely because it *would* hand back bits that may name a freed object;
/// nothing leaves here, so nothing here can go stale, and a second call has
/// nothing to get wrong. The two share `Task::taken`, so whichever runs first
/// is the one that cancels the single `gc::add_root`.
///
/// The reason a caller can want the root released without the value is that
/// the value does not only live in `Task::output`: it is also still in
/// [`STATE_SLOT_OUTPUT`] of the state object, which is reachable from word
/// [`FUTURE_SLOT_STATE`] of the future -- so a caller that holds the future
/// holds the value, through the collector's ordinary tracing, and needs nothing
/// from here but the end of the executor's own claim. `JoinHandle::join` is the
/// intended caller; what it does with the value is its own business.
fn release_internal(id: i64) {
    let state = TASKS.with(|tasks| {
        let mut tasks = tasks.borrow_mut();
        let Some(task) = tasks.get_mut(id as usize) else {
            abort_with(&format!("nova_rt_task_release: unknown task id {id}"));
        };
        if task.taken {
            return None;
        }
        task.taken = true;
        Some(task.state)
    });
    // Outside the borrow above, for the same reason `take_output_internal`
    // orders its own `gc::remove_root` after releasing it.
    if let Some(state) = state {
        gc::remove_root(state);
    }
    // A task's payload slots die with the task. `fs.rs` owns the storage and the
    // root discipline; this is the whole of `task.rs`'s knowledge of it. The
    // matching call is in `take_output_internal` / `release_internal` -- both
    // release points need it, because either can be the last thing to touch a
    // task.
    crate::fs::release_task_slots(id);
}

/// Hand out task `id`'s output and release the GC root `spawn_internal`
/// registered for its state object -- the single `gc::remove_root` matching
/// that single `gc::add_root`, so the registry's multiset stays balanced.
///
/// This is the *take*, not a peek, and that is what makes a heap-valued
/// output sound: until this call the value is kept alive by the state object's
/// own root (see [`poll_one`]), and after it the value is in the caller's
/// hands, conservatively rooted by the caller's own frame the same way any
/// other runtime function's returned pointer is. Both failure modes are
/// therefore diagnosed rather than answered:
///
/// - **Not done.** The `output` field still holds its `0` initializer, which
///   is indistinguishable from a task that genuinely completed with `0`.
///   `JoinHandle::join` is specified to poll [`nova_rt_task_is_done`] until
///   it reports true before reading the output, so reaching here early is a
///   caller bug.
/// - **Already taken.** The root is gone, so `output`'s bits may name a freed
///   object; returning them a second time would be exactly the dangling
///   pointer this ordering exists to prevent. A caller that needs the value
///   twice must keep its own copy.
///
/// The cost of releasing the root here rather than at completion: a spawned
/// task whose root is *never* released keeps its state object rooted for the
/// rest of the process. That is a leak, not unsoundness, and it is the
/// deliberate trade -- the alternative (unroot at completion) frees a
/// heap-valued output while `Task::output` still names it. Either this function
/// or [`release_internal`] ends that claim; both check the same
/// `Task::taken` flag, so a task's single root is cancelled at most once
/// whichever of them a caller reaches.
fn take_output_internal(id: i64) -> i64 {
    let (output, state) = TASKS.with(|tasks| {
        let mut tasks = tasks.borrow_mut();
        let task = tasks
            .get_mut(id as usize)
            .expect("nova_rt_task_take_output: unknown task id");
        assert!(
            task.done,
            "nova_rt_task_take_output: task {id} has not completed; \
             poll nova_rt_task_is_done until it reports true first"
        );
        assert!(
            !task.taken,
            "nova_rt_task_take_output: task {id}'s output was already taken; \
             taking released the state object's GC root, so a heap-valued \
             output may already have been collected"
        );
        task.taken = true;
        (task.output, task.state)
    });
    // Strictly after the borrow above is released and after `output` has been
    // copied out: `gc::remove_root` allocates nothing and so cannot trigger a
    // collection, but ordering the read first keeps that independent of
    // `remove_root`'s implementation.
    gc::remove_root(state);
    // A task's payload slots die with the task. `fs.rs` owns the storage and the
    // root discipline; this is the whole of `task.rs`'s knowledge of it. The
    // matching call is in `take_output_internal` / `release_internal` -- both
    // release points need it, because either can be the last thing to touch a
    // task.
    crate::fs::release_task_slots(id);
    output
}

/// Spawn `future` as this thread's newest task, then run this thread's
/// executor until every task registered so far -- `future`'s own and any
/// other still-pending task -- has completed, and return `future`'s output.
///
/// This drains the whole shared queue rather than stopping as soon as
/// `future` itself is done. Two consequences, both decisions rather than
/// accidents:
///
/// 1. **A spawned task cannot outlive the root**, so `block_on` implicitly
///    joins everything queued on this thread. That diverges from tokio's
///    `block_on`, which returns as soon as its own future resolves and leaves
///    other tasks to the runtime's background workers. There are no
///    background workers here: with no waker and no driver thread, a
///    `block_on` call is the only thing that ever advances any task on this
///    thread, so a task spawned earlier and left pending would otherwise have
///    no way to reach `nova_rt_task_take_output`'s promised state (`done`,
///    with an output) at all.
/// 2. **A task that stages a park no longer spins this loop forever, and a
///    task that instead keeps re-queueing itself can no longer starve
///    anything else's deadline while it does.** (It still re-queues itself
///    forever, by design; what changed is only that doing so no longer costs
///    a sibling its wake-up. A task joining *that* kind of task is a
///    livelock, not a deadlock, and this loop still cannot see it -- ADR
///    0009 §1's 2026-08-10 amendment records it as accepted, not fixed.)
///    Before the park set, a task returning [`POLL_PENDING`] was
///    unconditionally re-queued, so one that never became ready re-queued
///    forever and the loop hung with no diagnostic. A poll that stages a
///    [`Wait`] (see [`stage_park`]) moves its task to [`PARKED`] instead of
///    back onto the queue -- but a task that does not park (`yield_now`)
///    still re-queues itself every turn by design, and that alone must not
///    be able to starve a sibling's deadline. [`wake_due`] is therefore
///    called after **every** poll below, not only once this loop's inner
///    pass finds the queue empty: if a deadline were checked only there, a
///    task that keeps re-queueing itself would keep that pass from ever
///    finding the queue empty, and anything parked on a deadline would never
///    be examined at all -- not delayed, starved permanently, for as long as
///    anything kept re-queueing. Task 2's own end-to-end fixture is what
///    surfaced this: `sleep`, awaited underneath what was then a spinning
///    `join`, hung indefinitely until this per-poll check was added.
///    `poll::wait`, which is where the real sleeping (or, from Task 2 on,
///    socket waiting) now happens, is reached only from the
///    drained-queue branch below, where blocking this thread is correct
///    because there is genuinely nothing left to run; a park set holding
///    nothing but *bare* `Wait::Task` entries and no I/O wait there is a
///    genuine deadlock, since nothing still running can ever finish and wake
///    one, and [`report_deadlock`] ends the process naming each one instead
///    of hanging silently. A `Wait::Io` park changes what "nothing left to
///    run" means: waiting on a socket that some *other* process must make
///    ready is legitimate waiting, not a deadlock. A **timed** `Wait::Task`
///    changes it the same way: its own clock, not some other task's
///    progress, is what eventually wakes it, so it is legitimate waiting too
///    -- [`earliest_deadline`] reports its deadline exactly as it would a
///    bare `Wait::Deadline` or a timed `Wait::Io`'s (see that function's own
///    doc comment), which is what routes the drained-queue match below to a
///    sleep instead of to `report_deadlock`. So `report_deadlock` is
///    reachable only when the deadline dimension -- which now includes any
///    timed `Wait::Task` -- and the I/O dimension are *both* empty, i.e.
///    every remaining park is a bare `Wait::Task` (see the drained-queue
///    match below). `sleep` (Task 2) and a parking `join`
///    (Task 3) are what let Nova source reach any of this at all: before
///    `sleep`, `yield_now` was the only suspension an `async fn` body could
///    await, and it never stages a `Wait`, so nothing compiled from Nova
///    source could ever populate [`PARKED`] in the first place.
///
/// # Safety
/// `future` must be a valid future fat pointer (see [`read_future`]).
unsafe fn run_to_completion(future: *mut u8) -> i64 {
    // SAFETY: forwarding this function's own contract.
    let root_id = unsafe { spawn_internal(future) };
    loop {
        while let Some(id) = QUEUE.with(|queue| queue.borrow_mut().pop_front()) {
            // SAFETY: `id` was just popped from `QUEUE`, so it is a registered
            // task id (pushed by `spawn_internal` once, re-pushed only by
            // `poll_one` and `wake_tasks_waiting_on` for a live task).
            unsafe { poll_one(id) };
            // Checked after every poll, not only once this inner loop finds
            // the queue empty -- see this function's own doc comment for why
            // a self-requeuing task would otherwise starve every deadline in
            // `PARKED` forever rather than merely delay it. Guarded on
            // `PARKED` being non-empty so the overwhelmingly common case
            // (nothing parked) costs one `Vec::is_empty` check and never
            // reads the clock.
            if !PARKED.with(|parked| parked.borrow().is_empty()) {
                wake_due(Instant::now());
            }
        }
        if PARKED.with(|parked| parked.borrow().is_empty()) {
            break;
        }
        // The ready queue is empty and something is still parked. A remaining
        // deadline (bare, or riding inside an I/O wait or a task wait) or a
        // live I/O wait can each refill the queue on their own; a park set
        // holding nothing but *bare* `Wait::Task` entries -- neither
        // dimension populated -- cannot, because nothing is running to
        // finish. This match is the executor's only remaining wait: sleeping
        // *is* waiting on an empty socket set (the `(Some(at), true)` arm),
        // so there is exactly one place this thread ever blocks, and
        // `task.rs` itself no longer knows how.
        let io: Vec<(RawSocket, Interest)> = io_parks();
        match (earliest_deadline(), io.is_empty()) {
            // Nothing can ever wake anything: the only true deadlock.
            (None, true) => report_deadlock(),
            // Waiting on a peer is legitimate waiting, not a deadlock.
            (None, false) => wake_ready(crate::poll::wait(&io, None)),
            // No sockets and a deadline IS a sleep -- there is no second
            // timing path; `poll::wait` does the actual blocking.
            (Some(at), true) => {
                crate::poll::wait(&[], Some(at));
            }
            (Some(at), false) => wake_ready(crate::poll::wait(&io, Some(at))),
        }
        wake_due(Instant::now());
    }
    // The queue is empty, and the root task was in it, so it must have
    // finished -- the only way out of the queue is completion. Asserted
    // rather than assumed because `take_output_internal` panics on a task
    // that is not done, and this call site is the one place that must never
    // be able to trigger that panic.
    debug_assert!(
        is_done_internal(root_id),
        "run_to_completion: the queue drained but the root task is not done"
    );
    take_output_internal(root_id)
}

/// The soonest instant any parked task is waiting for, if any is waiting on
/// the clock at all -- a bare [`Wait::Deadline`], or the deadline riding
/// inside a [`Wait::Io`] or a [`Wait::Task`]. A `Wait::Task` with no deadline
/// contributes nothing, the same as an untimed `Wait::Io`.
fn earliest_deadline() -> Option<Instant> {
    PARKED.with(|parked| {
        parked
            .borrow()
            .iter()
            .filter_map(|&(_, wait)| match wait {
                Wait::Deadline(at) => Some(at),
                Wait::Io { deadline, .. } => deadline,
                Wait::Task { deadline, .. } => deadline,
            })
            .min()
    })
}

/// Every socket a parked task is waiting on, paired with the interest it is
/// waiting for.
///
/// This returns a non-empty vector in production now: `net.rs` stages a
/// [`Wait::Io`] from each of its four poll functions (see [`stage_io_park`]),
/// so an ordinary Nova program awaiting a `connect`, `read`, `write` or
/// `read_timeout` puts a real socket here, and `run_to_completion`'s
/// drained-queue match hands it to `poll::wait`. An earlier version of this
/// comment said the opposite -- that nothing outside this module's own tests
/// ever staged one, so this always returned empty -- which was true only
/// before `net.rs` existed.
fn io_parks() -> Vec<(RawSocket, Interest)> {
    PARKED.with(|parked| {
        parked
            .borrow()
            .iter()
            .filter_map(|&(_, wait)| match wait {
                Wait::Io {
                    socket, interest, ..
                } => Some((socket, interest)),
                Wait::Deadline(_) | Wait::Task { .. } => None,
            })
            .collect()
    })
}

/// Move every task parked on a [`Wait::Io`] whose socket is in `ready` back
/// onto the ready queue.
///
/// The I/O counterpart of [`wake_tasks_waiting_on`] and [`wake_due`], and the
/// increment's central new wake source: called with whatever `poll::wait`
/// reports ready, which is now a genuinely non-empty set whenever a `net.rs`
/// operation's socket becomes ready (see [`io_parks`]). An earlier version of
/// this comment called it "always a no-op today", true only while nothing
/// production ever staged a `Wait::Io`.
///
/// The `ready.contains(&socket)` guard below is what makes this wake *only*
/// the tasks whose own socket is ready; the `retain`'s trailing `_ => true`
/// would swallow the loss of that guard silently, so
/// `wake_ready_wakes_only_the_task_whose_socket_is_ready` pins it
/// behaviourally.
fn wake_ready(ready: Vec<RawSocket>) {
    if ready.is_empty() {
        return;
    }
    let woken = PARKED.with(|parked| {
        let mut parked = parked.borrow_mut();
        let mut woken = Vec::new();
        parked.retain(|&(id, wait)| match wait {
            Wait::Io { socket, .. } if ready.contains(&socket) => {
                woken.push(id);
                false
            }
            _ => true,
        });
        woken
    });
    QUEUE.with(|queue| {
        let mut queue = queue.borrow_mut();
        for id in woken {
            queue.push_back(id);
        }
    });
}

/// Move every parked task whose deadline is `<= now` back onto the ready
/// queue, leaving every other entry -- an untimed [`Wait::Io`] or
/// [`Wait::Task`], or a [`Wait::Deadline`] (bare, or riding inside a
/// [`Wait::Io`] or a [`Wait::Task`]) still in the future -- exactly where it
/// was.
///
/// Once an I/O wait carries a deadline, a passed deadline on that wait *is*
/// its timeout firing, so it must wake the task exactly as a bare
/// `Wait::Deadline` does -- it just also happens to leave that task's
/// socket, if `poll::wait` ever reported one ready at the same moment,
/// unconsumed. That is reachable in production now that `net.rs`'s
/// `read_timeout` stages a timed `Wait::Io`, and it is harmless for the reason
/// `poll_read_timeout`'s own doc comment gives: every poll retries the read
/// before consulting the deadline, so a task woken by its timeout still
/// observes data that arrived at the same instant and reports success rather
/// than a spurious `TimedOut`. An earlier version of this comment said nothing
/// here ever populated a `Wait::Io` outside a test, which stopped being true
/// when `net.rs` landed.
///
/// A timed `Wait::Task` wakes on the same rule as a timed `Wait::Io` above:
/// its deadline passing is its own timeout firing, which this function is
/// what notices -- [`wake_tasks_waiting_on`] is the only other thing that
/// ever removes a `Wait::Task` entry, and it reacts to the target completing,
/// never to the clock. **This arm is not compiler-forced the way
/// [`earliest_deadline`]'s and [`deadlock_report`]'s matches are**: this
/// function's `retain` ends in a wildcard, so a timed `Wait::Task` with no
/// arm here would fall through it, park forever, and fail nothing at compile
/// time -- see `wake_due_wakes_a_task_wait_whose_deadline_elapsed`, the test
/// that exists because nothing else would catch it.
///
/// Called by [`run_to_completion`] after every single poll (cheap: one pass
/// over [`PARKED`], no blocking) so a self-requeuing task can never starve a
/// deadline -- see its doc comment -- and again after every `poll::wait` call
/// in the drained-queue branch, which is the one place this thread actually
/// blocks.
fn wake_due(now: Instant) {
    let woken = PARKED.with(|parked| {
        let mut parked = parked.borrow_mut();
        let mut woken = Vec::new();
        parked.retain(|&(id, wait)| match wait {
            Wait::Deadline(deadline) if deadline <= now => {
                woken.push(id);
                false
            }
            Wait::Io {
                deadline: Some(deadline),
                ..
            } if deadline <= now => {
                woken.push(id);
                false
            }
            Wait::Task {
                deadline: Some(deadline),
                ..
            } if deadline <= now => {
                woken.push(id);
                false
            }
            _ => true,
        });
        woken
    });
    QUEUE.with(|queue| {
        let mut queue = queue.borrow_mut();
        for id in woken {
            queue.push_back(id);
        }
    });
}

/// The deadlock message: a headline plus one line per parked task.
///
/// Separate from [`report_deadlock`] so a test can assert the text without
/// aborting. `run_to_completion` reaches `report_deadlock` -- and so this --
/// only from its drive loop's `(None, true)` arm: no deadline anywhere in
/// [`PARKED`] *and* no I/O wait anywhere in it either. A bare `Wait::Deadline`
/// or a timed `Wait::Io` therefore cannot actually reach here through that
/// path, and neither can an untimed `Wait::Io`, since `io.is_empty()` would
/// have been `false`. Every arm below is still printed rather than treated as
/// unreachable, because a panic on an abort path would replace a clear
/// diagnostic with a confusing one, and this function is also called
/// directly by this module's own tests with whatever `PARKED` contents they
/// choose.
fn deadlock_report() -> String {
    let entries = PARKED.with(|parked| parked.borrow().clone());
    let plural = if entries.len() == 1 {
        "task is"
    } else {
        "tasks are"
    };
    let mut report = format!(
        "nova: deadlock: {} {plural} parked and none can wake\n",
        entries.len()
    );
    for (id, wait) in entries {
        match wait {
            Wait::Task {
                id: target,
                deadline: None,
            } => {
                report.push_str(&format!(
                    "  task {id} is waiting for task {target} to finish\n"
                ));
            }
            Wait::Task {
                id: target,
                deadline: Some(_),
            } => {
                report.push_str(&format!(
                    "  task {id} is waiting for task {target} to finish, with a deadline\n"
                ));
            }
            Wait::Deadline(_) => {
                report.push_str(&format!("  task {id} is waiting on a deadline\n"));
            }
            Wait::Io { deadline: None, .. } => {
                report.push_str(&format!("  task {id} is waiting on i/o\n"));
            }
            Wait::Io {
                deadline: Some(_), ..
            } => {
                report.push_str(&format!("  task {id} is waiting on i/o with a deadline\n"));
            }
        }
    }
    report
}

/// Print the deadlock report and end the process.
fn report_deadlock() -> ! {
    eprint!("{}", deadlock_report());
    std::process::abort();
}

/// Queue a `{ poll_code, state }` future as a new task and return its id.
///
/// `"C-unwind"`: see [`PollFn`]'s doc comment for why every entry point in
/// this module shares that ABI rather than plain `"C"`.
///
/// # Safety
/// `future` must point to a live [`FUTURE_SIZE`]-byte
/// `{ poll_code, state }` block, and:
///
/// - `poll_code` ([`FUTURE_SLOT_POLL`]) must be a non-null [`PollFn`] address
///   (see [`read_future`] for why null is undefined behaviour here, not a
///   later fault);
/// - `state` ([`FUTURE_SLOT_STATE`]) must point to a live, writable block of
///   at least `(STATE_SLOT_TEMPS + n_temps) * 8` bytes, and never fewer than
///   [`STATE_MIN_SIZE`]. **[`STATE_SLOT_OUTPUT`] is read unconditionally on
///   completion** -- including for a future whose `async fn` returns unit and
///   has no temps, where nothing in the poll function itself ever touches
///   that slot. A tag-only 8-byte state object satisfies neither this
///   contract nor the executor's read, and would be read 8 bytes past its
///   allocation;
/// - `state` must be allocated **scanned** (`gc::alloc(.., true)`, which is
///   what `nova_rt_alloc` does): a heap-valued output written to
///   [`STATE_SLOT_OUTPUT`] is kept alive by tracing through the state object,
///   so an unscanned state object would let it be collected (see
///   [`poll_one`]).
#[no_mangle]
pub unsafe extern "C-unwind" fn nova_rt_task_spawn(future: *mut u8) -> i64 {
    // SAFETY: forwarding this function's own contract.
    unsafe { spawn_internal(future) }
}

/// Drive this thread's executor to quiescence and return `future`'s output.
/// See [`run_to_completion`] for exactly what "quiescence" means here.
///
/// `"C-unwind"`: see [`PollFn`]'s doc comment. [`poll_one`]'s status
/// diagnostic leaves through this function's `resume_unwind` below, so this is
/// the boundary that permission is needed at.
///
/// **Re-entrancy ends the process rather than unwinding out of it.** A nested
/// call means a poll function called `block_on`, and a poll function's frame is
/// generated code with no unwind description, so a panic raised here would have
/// to be unwound *through* that frame to reach any handler. `std/task`'s
/// `block_on` makes this reachable from ordinary Nova source (`async fn f() {
/// block_on(g()) }` compiles), which is exactly why the diagnostic is an
/// [`abort_with`] and not a `panic!`.
///
/// # Safety
/// `future` must satisfy [`nova_rt_task_spawn`]'s contract exactly -- same
/// [`FUTURE_SIZE`]-byte fat pointer, same non-null `poll_code`, and the same
/// minimum of `(STATE_SLOT_TEMPS + n_temps) * 8` scanned, writable bytes
/// (never fewer than [`STATE_MIN_SIZE`]) for the state object, because
/// [`STATE_SLOT_OUTPUT`] is read unconditionally on completion here too.
///
/// # Panics
/// If any polled task's poll function returns a status that is neither
/// [`POLL_PENDING`] nor [`POLL_READY`] (see [`poll_one`]).
#[no_mangle]
pub unsafe extern "C-unwind" fn nova_rt_task_block_on(future: *mut u8) -> i64 {
    if IN_BLOCK_ON.with(|in_block_on| in_block_on.get()) {
        abort_with(
            "nova_rt_task_block_on called re-entrantly: an async fn must not call block_on, \
             which would run a second executor loop from inside the first one's frame",
        );
    }
    IN_BLOCK_ON.with(|in_block_on| in_block_on.set(true));
    // `AssertUnwindSafe`: the closure only captures a `*mut u8`, which is
    // GC-owned data with no invariant that spans a panic here, so asserting
    // unwind-safety is correct rather than papering over a real hazard.
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        // SAFETY: forwarding this function's own contract.
        unsafe { run_to_completion(future) }
    }));
    // Cleared on every path out of this call, including `poll_one`'s status
    // diagnostic unwinding through the `catch_unwind` above -- otherwise one
    // such `block_on` would leave every later call on this thread permanently
    // (and incorrectly) diagnosed as re-entrant.
    IN_BLOCK_ON.with(|in_block_on| in_block_on.set(false));
    match result {
        Ok(output) => output,
        Err(payload) => std::panic::resume_unwind(payload),
    }
}

/// Read the task id for `future`'s state object, or abort.
///
/// **A miss means `future` was never spawned, and nothing else can miss.** The
/// key is dropped only when the collector frees the state object
/// ([`forget_freed_state`]), and that cannot have happened while a live future
/// still points at the state: the fat pointer is a scanned heap object holding
/// the state address in word [`FUTURE_SLOT_STATE`], so tracing the future marks
/// the state. So a hit is this future's own task -- including a released one,
/// which is what a second `join` needs -- and there is no staleness test to do
/// here.
///
/// Aborts rather than panics: both callers are reachable from `join`, which
/// runs inside a generated poll frame, and a panic must not cross that
/// boundary (ADR 0009 section 1).
///
/// # Safety
/// `future` must be a valid future fat pointer (see [`read_future`]).
unsafe fn task_id_of(future: *mut u8, who: &str) -> i64 {
    // SAFETY: forwarding this function's own contract.
    let (_, state) = unsafe { read_future(future) };
    match BY_STATE.with(|m| m.borrow().get(&(state as usize)).copied()) {
        Some(id) => id,
        None => abort_with(&format!(
            "{who}: this future was never spawned, so there is no task to ask about"
        )),
    }
}

/// Whether the task named by `future`'s state object has completed.
///
/// `"C-unwind"`: see [`PollFn`]'s doc comment. Sharing one ABI across every
/// entry point in this module means a caller never has to know which of
/// them can panic; `is_done`/`take_output`'s internal helpers do, on an
/// unknown task id, and so does resolving `future` itself (see
/// [`task_id_of`]).
///
/// # Safety
/// `future` must be a valid future fat pointer (see [`read_future`]). One
/// this thread never spawned ends the process rather than answering.
#[no_mangle]
pub unsafe extern "C-unwind" fn nova_rt_task_is_done(future: *mut u8) -> i8 {
    // SAFETY: forwarding this function's own contract.
    let id = unsafe { task_id_of(future, "nova_rt_task_is_done") };
    is_done_internal(id) as i8
}

/// A completed task's output slot, **taken exactly once**.
///
/// Taking releases the GC root that was keeping the output alive, so this must
/// be called once per task and only after [`nova_rt_task_is_done`] reports
/// true. `JoinHandle::join` is the intended caller and is specified to do
/// exactly that. A second take, or a take before completion, panics with a
/// diagnostic rather than returning a stale or uninitialised value -- see
/// [`take_output_internal`] for the full rationale and for the leak that is
/// the cost of this ordering.
///
/// `"C-unwind"`: see [`nova_rt_task_is_done`]'s doc comment.
///
/// # Safety
/// `id` must be an id previously returned by `nova_rt_task_spawn` on this
/// same thread.
///
/// # Panics
/// If `id` is unknown, if task `id` has not completed, or if its output has
/// already been taken.
#[no_mangle]
pub unsafe extern "C-unwind" fn nova_rt_task_take_output(id: i64) -> i64 {
    take_output_internal(id)
}

/// End the executor's claim on the task named by `future`'s state object.
/// See [`release_internal`] for why this exists next to
/// [`nova_rt_task_take_output`] rather than instead of it, and why calling it
/// twice is a no-op rather than a diagnostic.
///
/// `"C-unwind"`: see [`nova_rt_task_is_done`]'s doc comment.
///
/// # Safety
/// `future` must be a valid future fat pointer (see [`read_future`]). One
/// this thread never spawned ends the process rather than releasing the
/// wrong task (see [`task_id_of`]).
#[no_mangle]
pub unsafe extern "C-unwind" fn nova_rt_task_release(future: *mut u8) {
    // SAFETY: forwarding this function's own contract.
    let id = unsafe { task_id_of(future, "nova_rt_task_release") };
    release_internal(id);
}

/// The state object [`nova_rt_task_yield_future`] allocates: the tag and output
/// slots, and nothing else.
///
/// [`poll_yield_once`] holds no value across its one suspension, so it needs no
/// temp slots -- and [`STATE_MIN_SIZE`] is the floor regardless, because
/// [`STATE_SLOT_OUTPUT`] is read unconditionally on completion.
const YIELD_STATE_SIZE: usize = STATE_MIN_SIZE;

/// Both slots [`poll_yield_once`] and the executor address must be inside
/// [`YIELD_STATE_SIZE`]. A compile-time check rather than a test, because it is
/// a relation between two constants in this file and nothing at run time can
/// make it hold or fail differently: shrinking [`STATE_MIN_SIZE`], or moving
/// [`STATE_SLOT_OUTPUT`] past it, must fail the build here rather than write one
/// word past an allocation.
const _: () = assert!(YIELD_STATE_SIZE >= (STATE_SLOT_OUTPUT + 1) * 8);

/// Report [`POLL_PENDING`] on the first poll of a state object and
/// [`POLL_READY`] on every later one.
///
/// The only [`PollFn`] in the system that `async_lower.rs` did not generate,
/// and it exists because a Nova `async fn` body suspends *only* at an
/// `.await`: `yield_now` has to await something that is not ready yet, and
/// nothing expressible in Nova is. Its resumption is unconditional rather than
/// tied to any event -- see this module's own doc comment on the absence of
/// wakers.
///
/// **It must not unwind.** `async_lower.rs`'s argument that no unwind can
/// cross a generated poll frame rests on every awaited future's poll code
/// having been emitted by that pass; this is the one exception, so it carries
/// the obligation directly. Its body is two raw word reads and one raw word
/// write, with no allocation, no `TASKS`/`QUEUE` borrow and no fallible
/// operation, so there is no panic to suppress.
unsafe extern "C-unwind" fn poll_yield_once(state: *mut u8, _task_ctx: *mut u8) -> i64 {
    let slots = state as *mut i64;
    // SAFETY: `state` is a `YIELD_STATE_SIZE`-byte state object built by
    // `nova_rt_task_yield_future`, so both slots below are in bounds.
    let tag = unsafe { slots.add(STATE_SLOT_TAG).read() };
    if tag == 0 {
        // SAFETY: same object, tag slot.
        unsafe { slots.add(STATE_SLOT_TAG).write(1) };
        return POLL_PENDING;
    }
    // A unit-valued output, written explicitly for the same reason
    // `async_lower.rs` writes one: the executor reads this slot on completion
    // whether or not the future carries a value.
    //
    // SAFETY: same object, output slot -- in bounds by `YIELD_STATE_SIZE`.
    unsafe { slots.add(STATE_SLOT_OUTPUT).write(0) };
    POLL_READY
}

/// Build a `{ poll_code, state }` future: a scanned [`FUTURE_SIZE`] fat pointer
/// over a freshly allocated scanned state object of `state_size` bytes, with
/// `init` handed the state's slots to populate before the future is returned.
///
/// **The one place this layout is written.** It is the layout
/// `async_lower.rs` independently emits, so a second copy that drifts from it
/// is a silent miscompile rather than a failure -- this project has already
/// shipped one miscompile from two sites drifting apart.
///
/// `pub(crate)`, not private: `net.rs` (Task 3) builds sockets' futures the
/// same way every future in this file is built, so it needs this constructor
/// rather than a second copy of the layout it writes -- the same reason
/// [`abort_with`] is `pub(crate)` rather than private. `Wait` and
/// [`stage_park`] stay private regardless; a wider `build_future` does not
/// let `net.rs` construct a `Wait` or match on one, which is the property
/// this task exists to protect (see [`stage_io_park`]).
///
/// `state` is registered as a GC root across the second allocation, which can
/// collect while `state` is named only by this frame. That ordering is the
/// subtle part every caller previously had to reproduce.
///
/// `gc::alloc` returns zeroed memory, so a caller whose only state is a resume
/// tag starting at `0` passes an `init` that writes nothing.
///
/// `state_size` must be at least [`STATE_MIN_SIZE`]: [`STATE_SLOT_OUTPUT`] is
/// read unconditionally on completion (`poll_one`'s doc comment), regardless
/// of what the future's own poll function does with it, so a smaller state
/// object is an out-of-bounds read rather than a caught error. Checked with a
/// `debug_assert!` rather than an `abort_with`: every caller passes a
/// compile-time constant today, so this can only fire on a caller's own
/// mistake, not on Nova input, and release builds already pay for the bug via
/// the out-of-bounds read itself if the assert is compiled out.
pub(crate) fn build_future(
    poll: PollFn,
    state_size: usize,
    init: impl FnOnce(*mut i64),
) -> *mut u8 {
    debug_assert!(
        state_size >= STATE_MIN_SIZE,
        "build_future: state_size {state_size} is smaller than STATE_MIN_SIZE \
         ({STATE_MIN_SIZE}); STATE_SLOT_OUTPUT is read unconditionally on \
         completion and would be read out of bounds"
    );
    let state = gc::alloc(state_size, true);
    gc::add_root(state);
    init(state as *mut i64);
    let fat = gc::alloc(FUTURE_SIZE, true);
    // SAFETY: `fat` is a live, writable `FUTURE_SIZE` block, so both words
    // below are in bounds. Written before the root is released, so the state
    // object is reachable from the future by the time it is unrooted.
    unsafe {
        (fat as *mut i64)
            .add(FUTURE_SLOT_POLL)
            .write(poll as usize as i64);
        (fat as *mut i64).add(FUTURE_SLOT_STATE).write(state as i64);
    }
    gc::remove_root(state);
    fat
}

/// A fresh `Future<unit>` that pends once and then completes -- what
/// `std/task`'s `yield_now` awaits.
///
/// **The state object is fresh on every call**, not a shared static: the whole
/// value carried by one of these futures is its resume tag, so two suspensions
/// alive at once would otherwise be one suspension, and the second task to
/// poll would find the first task's tag already advanced and complete without
/// ever having yielded.
///
/// Builds exactly the layout [`nova_rt_task_spawn`] documents and
/// `async_lower.rs` independently emits, via [`build_future`] -- see its doc
/// comment for why that layout now has exactly one home. [`poll_yield_once`]
/// is the poll function and [`YIELD_STATE_SIZE`] the state size. Reproducing a
/// layout the compiler also emits is a silent miscompile when it is wrong,
/// which is why `the_yield_futures_layout_is_the_one_the_abi_declares` asserts
/// the tracked `(size, scan)` of both allocations rather than only the words
/// written into them.
#[no_mangle]
pub extern "C-unwind" fn nova_rt_task_yield_future() -> *mut u8 {
    // Bound as a `PollFn` before being passed, rather than cast from the
    // function item: the coercion is what checks that this function's
    // signature *is* the poll ABI.
    let poll: PollFn = poll_yield_once;
    build_future(poll, YIELD_STATE_SIZE, |_slots| {})
}

/// Report [`POLL_READY`] once the stored deadline has passed, and
/// [`POLL_PENDING`] with the deadline re-staged until then.
///
/// **Level-triggered and tag-free**, which makes this structurally identical
/// to [`poll_join`]. It has to be: since a deadline may accompany any wait and
/// two deadlines merge to the earlier, this future can be polled because
/// *another* wait's deadline fired. An edge-triggered version -- returning
/// ready on its second poll regardless of the clock -- would report a
/// completion it had not earned.
///
/// **Must not unwind**, for the reason [`poll_yield_once`] states. Every
/// helper it calls is panic-free by construction; see
/// [`instant_from_deadline_nanos`] on why the old `Instant + Duration` was not.
unsafe extern "C-unwind" fn poll_sleep(state: *mut u8, _task_ctx: *mut u8) -> i64 {
    let slots = state as *mut i64;
    // SAFETY: `state` is the state object `nova_rt_task_sleep_future_nanos`
    // built, of at least `SLEEP_STATE_SIZE` bytes, so both slots are in bounds.
    let deadline = unsafe { slots.add(SLEEP_SLOT_DEADLINE_NANOS).read() };
    if crate::time::now_nanos() >= deadline {
        // SAFETY: same object, output slot.
        unsafe { slots.add(STATE_SLOT_OUTPUT).write(0) };
        return POLL_READY;
    }
    stage_park(Wait::Deadline(instant_from_deadline_nanos(deadline)));
    POLL_PENDING
}

/// A deadline `nanos` nanoseconds from now, as nanoseconds since
/// `crate::time::epoch()`, clamping a non-positive argument to "now".
///
/// Nova's `Int` is signed and nothing stops `sleep(-1)`; treating it as an
/// immediate wake keeps the executor's invariants intact without inventing a
/// new failure mode for an argument that is merely useless. Saturating rather
/// than wrapping, for the same reason the reading itself saturates.
fn deadline_nanos_from_now(nanos: i64) -> i64 {
    crate::time::now_nanos().saturating_add(nanos.max(0))
}

/// A deadline in epoch-nanoseconds as an `Instant`, for staging.
///
/// **`checked_add`, not `+`.** `Instant + Duration` panics on overflow, and
/// this is reached from [`poll_sleep`], a hand-written `PollFn` across which
/// no panic may pass. The previous `Instant::now() + Duration::from_nanos(..)`
/// carried that panic on a path nothing had ruled out.
///
/// **Delegates the overflow case to [`furthest_representable_instant`]
/// rather than falling back to `Instant::now()`.** An earlier version of this
/// function did exactly that, and it traded the panic for a livelock: the
/// staged `Instant` would read as immediately due to [`wake_due`]'s
/// `deadline <= now` check, but [`poll_sleep`] compares the *stored integer*
/// deadline (still far in the future) against the clock on every re-poll, so
/// the task would wake, find itself not due by its own arithmetic, and
/// re-stage the same broken `Instant::now()` -- forever, burning CPU with no
/// progress. See [`furthest_representable_instant`] for why its answer does
/// not have that problem.
fn instant_from_deadline_nanos(deadline: i64) -> Instant {
    let ns = u64::try_from(deadline).unwrap_or(0);
    furthest_representable_instant(crate::time::epoch(), Duration::from_nanos(ns))
}

/// `base + duration`, or the furthest instant from `base` this platform's
/// `Instant` can represent, halving `duration` until `checked_add` succeeds.
///
/// **Never earlier than `base`, and `base` is a fixed point in the past**
/// (`crate::time::epoch()`, captured once at process start), so this never
/// answers with something [`wake_due`] would treat as already due while the
/// caller's own integer deadline is still ahead of the clock -- the property
/// [`instant_from_deadline_nanos`]'s old `Instant::now()` fallback broke.
/// Halving only ever *shrinks* an unrepresentable `duration` towards one that
/// fits, so the loop always terminates (worst case at `Duration::ZERO`,
/// which `checked_add` always accepts, since adding nothing cannot overflow)
/// and every candidate it tries on the way stays a duration measured
/// forward from `base`, never behind it.
///
/// In production this call always succeeds on its first attempt: the only
/// caller bounds `duration` to at most `i64::MAX` nanoseconds (~292 years),
/// via [`deadline_nanos_from_now`]'s own saturation, and no supported `Instant`
/// backend has under that much headroom from a process-start `base`. The
/// halving loop exists for the input this crate cannot actually construct
/// through its own API, not the one it can -- one of this module's own tests
/// forces it directly with `Duration::MAX`, a value no caller here can ever
/// pass in, since `Instant` itself offers no public way to sit near its own
/// overflow boundary the way `Duration::MAX` does.
fn furthest_representable_instant(base: Instant, mut duration: Duration) -> Instant {
    loop {
        if let Some(instant) = base.checked_add(duration) {
            return instant;
        }
        duration /= 2;
    }
}

/// Where `nova_rt_task_sleep_future_nanos` stores its **deadline**, as
/// nanoseconds since `crate::time::epoch()`.
///
/// Renamed from `SLEEP_SLOT_NANOS`, which held a *duration*. The slot is an
/// `i64` either way, so changing what the integer means while keeping its
/// name would be invisible to the compiler -- the same hazard that made the
/// previous increment rename this parker rather than only retype it.
const SLEEP_SLOT_DEADLINE_NANOS: usize = STATE_SLOT_TEMPS;

/// State size for a sleep future: the ABI minimum plus the one temp slot
/// holding the deadline.
const SLEEP_STATE_SIZE: usize = STATE_MIN_SIZE + 8;

const _: () = assert!(SLEEP_STATE_SIZE >= (SLEEP_SLOT_DEADLINE_NANOS + 1) * 8);

/// A fresh `Future<unit>` that parks for `nanos` nanoseconds, then completes.
///
/// Same layout obligation as [`nova_rt_task_yield_future`]: a scanned
/// [`FUTURE_SIZE`] fat pointer over a scanned state object, built to the
/// layout `async_lower.rs` independently emits. A fresh state object per call,
/// because the deadline is per-suspension.
#[no_mangle]
pub extern "C-unwind" fn nova_rt_task_sleep_future_nanos(nanos: i64) -> *mut u8 {
    let poll: PollFn = poll_sleep;
    build_future(poll, SLEEP_STATE_SIZE, |slots| {
        // SAFETY: `slots` addresses a live `SLEEP_STATE_SIZE` block, and
        // `SLEEP_SLOT_DEADLINE_NANOS` is in bounds by the assertion above.
        unsafe {
            slots
                .add(SLEEP_SLOT_DEADLINE_NANOS)
                .write(deadline_nanos_from_now(nanos))
        };
    })
}

/// Park until the task driving `target` completes, then complete.
///
/// Polls to `POLL_READY` immediately if the target is already done, so a join
/// on a finished task costs no suspension. Otherwise stages a
/// [`Wait::Task`] park; the only thing that clears it is
/// [`wake_tasks_waiting_on`] at the target's completion, so one suspension is
/// always enough and no loop is needed.
///
/// **Must not unwind**, as [`poll_sleep`] and [`poll_yield_once`] must not.
/// `is_done_internal` borrows `TASKS`, and `poll_one` holds no `TASKS` borrow
/// across the `poll` call, so that borrow cannot fail.
unsafe extern "C-unwind" fn poll_join(state: *mut u8, _task_ctx: *mut u8) -> i64 {
    let slots = state as *mut i64;
    // SAFETY: `state` is the object `nova_rt_task_join_future` built, at least
    // `JOIN_STATE_SIZE` bytes, so every slot below is in bounds.
    let target = unsafe { slots.add(JOIN_SLOT_TARGET).read() };
    if is_done_internal(target) {
        // SAFETY: same object, output slot.
        unsafe { slots.add(STATE_SLOT_OUTPUT).write(0) };
        return POLL_READY;
    }
    stage_park(Wait::Task {
        id: target,
        deadline: None,
    });
    POLL_PENDING
}

/// Where the target task id lives in a join future's state object.
const JOIN_SLOT_TARGET: usize = STATE_SLOT_TEMPS;

/// State size for a join future: the ABI minimum plus the target-id slot.
const JOIN_STATE_SIZE: usize = STATE_MIN_SIZE + 8;

const _: () = assert!(JOIN_STATE_SIZE >= (JOIN_SLOT_TARGET + 1) * 8);

/// A fresh `Future<unit>` that completes when `future`'s task does.
///
/// Resolves `future` to a task id through the same `BY_STATE` path
/// `nova_rt_task_is_done` uses, so a future no task was spawned for aborts
/// there rather than becoming a park nothing can wake. The id is resolved
/// *once*, here, rather than on every poll: the target cannot change, and a
/// stale-address abort belongs at the call the Nova program made.
///
/// # Safety
/// `future` must point to a live `FUTURE_SIZE` `{ poll_code, state }` block.
#[no_mangle]
pub unsafe extern "C-unwind" fn nova_rt_task_join_future(future: *mut u8) -> *mut u8 {
    // SAFETY: forwarding this function's own contract.
    let target = unsafe { task_id_of(future, "nova_rt_task_join_future") };
    let poll: PollFn = poll_join;
    build_future(poll, JOIN_STATE_SIZE, |slots| {
        // SAFETY: `slots` addresses a live `JOIN_STATE_SIZE` block, and
        // `JOIN_SLOT_TARGET` is in bounds by the assertion above.
        unsafe { slots.add(JOIN_SLOT_TARGET).write(target) };
    })
}

/// Where a timeout future stores the inner future's `{ poll_code, state }`
/// fat pointer.
///
/// The state object is scanned (`build_future` allocates with
/// `gc::alloc(size, true)`), so storing this pointer is the whole of the
/// rooting: it keeps the inner future and, transitively, its state reachable
/// for exactly as long as the timeout future itself.
const TIMEOUT_SLOT_INNER: usize = STATE_SLOT_TEMPS;

/// Where a timeout future stores its deadline, as nanoseconds since
/// `crate::time::epoch()` -- the same encoding `SLEEP_SLOT_DEADLINE_NANOS` uses.
const TIMEOUT_SLOT_DEADLINE_NANOS: usize = STATE_SLOT_TEMPS + 1;

/// State size for a timeout future: the ABI minimum plus two temp slots.
const TIMEOUT_STATE_SIZE: usize = STATE_MIN_SIZE + 16;

const _: () = assert!(TIMEOUT_STATE_SIZE >= (TIMEOUT_SLOT_DEADLINE_NANOS + 1) * 8);

/// The inner future completed before the deadline.
pub const TIMEOUT_STATUS_COMPLETED: i64 = 0;

/// The deadline passed with the inner future still pending.
pub const TIMEOUT_STATUS_ELAPSED: i64 = 1;

/// Poll the inner future, then this timeout's own deadline.
///
/// **That order is deliberate.** Polling the inner first means work that
/// completed is never reported as timed out, and it makes a zero-duration
/// timeout over an already-ready future succeed -- the least surprising
/// answer. The order is only *available* because `poll_sleep` is
/// level-triggered: with an edge-triggered sleep, a woken sleep could report a
/// completion it had not earned, forcing a deadline-first check as a defence.
///
/// **Abandonment takes two lines, not none.** This function was first written
/// on the claim that it needed none: when the inner returns [`POLL_PENDING`]
/// and this function then returns [`POLL_READY`], the inner has already staged
/// a park, and [`poll_one`] takes `PENDING_PARK` unconditionally. That
/// mechanism is real, but it fires **once per task poll, not once per
/// future**, which is not what abandonment needs. A generated state machine
/// advances through as many awaits as complete in one poll, so an abandoned
/// inner's park sits in `PENDING_PARK` and the *next* suspension in that same
/// task poll stages against it. A [`Wait::Deadline`] merges harmlessly; every
/// other pairing reaches [`abort_with`] through [`try_stage`], with a message
/// blaming an inner future's `POLL_PENDING` for something it did not do.
/// `timeout(d, h.join())` followed by any other parking suspension aborted the
/// process for exactly that reason.
///
/// So `PENDING_PARK` is snapshotted before the inner is polled and
/// **restored** -- not cleared to `Staged::default()` -- on both
/// [`POLL_READY`] exits. Restoring the snapshot keeps the contract *local*:
/// this function hands the slot back exactly as it found it, so it composes
/// with a nested timeout and with an earlier abandonment in the same task
/// poll, where clearing would silently discard a park this function never
/// staged. A local invariant is also the one that survives the next
/// combinator: `select`/`race` will have this same shape. `Staged` is `Copy`
/// in a `Cell`, so neither the read nor the write can fail -- keep it that
/// way, since no panic may cross this boundary.
///
/// The pending path deliberately leaves the slot alone. There the inner's park
/// must survive to merge with this timeout's own deadline, which is the entire
/// point of the staging widening.
///
/// The test-only `poll_once_for_test` drains the slot unconditionally for the
/// same hazard seen from the other side; its own doc comment records why.
///
/// The abandonment *contract* is unchanged by any of this: abandonment is
/// still not cancellation, still free for `sleep`/`join`/`read`/`write`, and
/// still leaks a socket for `connect` (`std/time`'s own doc comment, and §5 of
/// this increment's design). Only the claim that it needed no code was false.
///
/// **Must not unwind.** Every slot access is a raw read with the SAFETY note
/// below; an out-of-range status from the inner poll goes to [`abort_with`],
/// the same route a staging collision takes, never `panic!`.
unsafe extern "C-unwind" fn poll_timeout(state: *mut u8, _task_ctx: *mut u8) -> i64 {
    let slots = state as *mut i64;
    // SAFETY: `state` is the object `nova_rt_task_timeout_future` built, at
    // least `TIMEOUT_STATE_SIZE` bytes, so every slot below is in bounds.
    let inner = unsafe { slots.add(TIMEOUT_SLOT_INNER).read() } as *mut u8;
    // SAFETY: same object.
    let deadline = unsafe { slots.add(TIMEOUT_SLOT_DEADLINE_NANOS).read() };

    // SAFETY: `inner` is the fat pointer written at construction, so word 0 is
    // its poll function and word 1 its state -- the layout `build_future`
    // guarantees and `async_lower.rs` independently emits.
    let inner_poll_addr = unsafe { (inner as *mut usize).add(FUTURE_SLOT_POLL).read() };
    // SAFETY: that word is a `PollFn` bit pattern by the inner future's own
    // construction; a fn pointer and a `usize` are both pointer-width.
    let inner_poll: PollFn = unsafe { std::mem::transmute(inner_poll_addr) };
    // SAFETY: same fat pointer, state word.
    let inner_state = unsafe { (inner as *mut usize).add(FUTURE_SLOT_STATE).read() } as *mut u8;

    // Whatever this task poll had already staged before the inner ran, so both
    // completing exits below can hand the slot back exactly as they found it.
    // Cannot fail: `Staged` is `Copy` in a `Cell` (see the doc comment above).
    let park_before_inner = PENDING_PARK.with(Cell::get);

    // SAFETY: `inner_poll`/`inner_state` are the pair inside `inner`;
    // `task_ctx` is always null, matching every other call site in this crate.
    let inner_status = unsafe { inner_poll(inner_state, std::ptr::null_mut()) };
    if inner_status == POLL_READY {
        PENDING_PARK.with(|cell| cell.set(park_before_inner));
        // SAFETY: same object, output slot.
        unsafe { slots.add(STATE_SLOT_OUTPUT).write(TIMEOUT_STATUS_COMPLETED) };
        return POLL_READY;
    }
    if inner_status != POLL_PENDING {
        abort_with(&format!(
            "nova_rt: a future polled by a timeout returned {inner_status}, which is \
             neither POLL_PENDING ({POLL_PENDING}) nor POLL_READY ({POLL_READY})"
        ));
    }
    if crate::time::now_nanos() >= deadline {
        // The abandonment path: the inner just parked and is about to be
        // dropped on the floor, so its park goes with it.
        PENDING_PARK.with(|cell| cell.set(park_before_inner));
        // SAFETY: same object, output slot.
        unsafe { slots.add(STATE_SLOT_OUTPUT).write(TIMEOUT_STATUS_ELAPSED) };
        return POLL_READY;
    }
    stage_park(Wait::Deadline(instant_from_deadline_nanos(deadline)));
    POLL_PENDING
}

/// A fresh `Future<Int>` that polls `fut` until it completes or `nanos`
/// nanoseconds pass, reporting which happened.
///
/// The value is **not** carried here: the inner future wrote its own output
/// slot, and Nova reads it with `task_output` on the inner future itself, so
/// nothing moves an `i64` that might be a scalar or a pointer.
#[no_mangle]
pub extern "C-unwind" fn nova_rt_task_timeout_future(nanos: i64, fut: *mut u8) -> *mut u8 {
    let poll: PollFn = poll_timeout;
    let deadline = deadline_nanos_from_now(nanos);
    build_future(poll, TIMEOUT_STATE_SIZE, |slots| {
        // SAFETY: `slots` addresses a live `TIMEOUT_STATE_SIZE` block, and both
        // slots are in bounds by the assertion above.
        unsafe { slots.add(TIMEOUT_SLOT_INNER).write(fut as i64) };
        unsafe { slots.add(TIMEOUT_SLOT_DEADLINE_NANOS).write(deadline) };
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    /// A hand-written poll function shaped exactly like the ones
    /// `async_lower.rs` will generate: it reads and writes the resume tag in
    /// slot 0, writes its result to slot 1, and returns PENDING/READY.
    ///
    /// Suspends once, then completes with 42.
    unsafe extern "C-unwind" fn poll_suspend_once(state: *mut u8, _ctx: *mut u8) -> i64 {
        let slots = state as *mut i64;
        // SAFETY: `state` is a live state object with at least
        // `STATE_SLOT_TEMPS` slots (`make_future`'s contract).
        let tag = unsafe { *slots.add(STATE_SLOT_TAG) };
        if tag == 0 {
            unsafe { *slots.add(STATE_SLOT_TAG) = 1 };
            POLL_PENDING
        } else {
            unsafe { *slots.add(STATE_SLOT_OUTPUT) = 42 };
            POLL_READY
        }
    }

    unsafe extern "C-unwind" fn poll_ready_now(state: *mut u8, _ctx: *mut u8) -> i64 {
        // SAFETY: `state` is a live state object (`make_future`'s contract).
        unsafe { *(state as *mut i64).add(STATE_SLOT_OUTPUT) = 7 };
        POLL_READY
    }

    /// Build the `{ poll_code, state_ptr }` fat pointer the compiler emits.
    ///
    /// `#[inline(never)]`: `root_registration`'s tests need this call's own
    /// `state`/`fat` locals confined to a call frame that returns (and can
    /// go dead) before the collection they provoke, matching the same
    /// requirement `gc.rs`'s `setup_parent_and_child` documents. Inlined,
    /// this function's locals would become anonymous temporaries inside the
    /// *caller's* frame, under names the caller has no way to null out.
    #[inline(never)]
    fn make_future(f: PollFn, temps: usize) -> *mut u8 {
        // Exactly `nova_rt_task_spawn`'s documented minimum: scanned, and
        // `(STATE_SLOT_TEMPS + temps) * 8` bytes so `STATE_SLOT_OUTPUT` is in
        // bounds even at `temps == 0`.
        let state = gc::alloc((STATE_SLOT_TEMPS + temps) * 8, true);
        let fat = gc::alloc(FUTURE_SIZE, true);
        unsafe {
            (fat as *mut usize).add(FUTURE_SLOT_POLL).write(f as usize);
            (fat as *mut usize)
                .add(FUTURE_SLOT_STATE)
                .write(state as usize);
        }
        fat
    }

    /// Read a future's state-object address back out of its fat pointer.
    fn state_of(future: *mut u8) -> usize {
        // SAFETY: `future` is a `make_future` result, so word
        // `FUTURE_SLOT_STATE` is the state object's address.
        unsafe { (future as *mut usize).add(FUTURE_SLOT_STATE).read() }
    }

    /// Poll a future once under a borrowed task context, draining any park it
    /// staged.
    ///
    /// The park must be drained: `stage_park` aborts the process when a second
    /// deadline is staged over a first, so a leftover entry kills a later test
    /// under `--test-threads=1`.
    ///
    /// That is the same hazard [`poll_timeout`] has to handle in production,
    /// seen from the other side -- and the reason this helper drains
    /// *unconditionally*, rather than only when the poll came back
    /// `POLL_PENDING`, is precisely that a poll returning `POLL_READY` can
    /// still have left something staged. This helper may therefore not be used
    /// by any test that needs to read the slot after the poll; those tests
    /// write the raw poll idiom out longhand instead.
    #[cfg(test)]
    fn poll_once_for_test(fut: *mut u8) -> i64 {
        let poll_addr = unsafe { (fut as *mut usize).add(FUTURE_SLOT_POLL).read() };
        // SAFETY: word 0 is a `PollFn` bit pattern by `fut`'s own construction.
        let poll: PollFn = unsafe { std::mem::transmute(poll_addr) };
        let state = state_of(fut) as *mut u8;
        let previous = current_task();
        set_current_for_test(Some(0));
        // SAFETY: `poll`/`state` are the pair inside `fut`; `task_ctx` is null.
        let status = unsafe { poll(state, std::ptr::null_mut()) };
        PENDING_PARK.with(|cell| {
            cell.take();
        });
        set_current_for_test(previous);
        status
    }

    /// The value a future wrote to its own output slot.
    #[cfg(test)]
    fn output_of_for_test(fut: *mut u8) -> i64 {
        let slots = state_of(fut) as *mut i64;
        // SAFETY: every future this module builds is at least
        // `STATE_MIN_SIZE`, so the output slot is in bounds.
        unsafe { slots.add(STATE_SLOT_OUTPUT).read() }
    }

    /// Build a future with a `STATE_MIN_SIZE` state and nothing in it but the
    /// tag and output slots, which `gc::alloc`'s zeroing already leaves at
    /// `0`. A one-liner over [`build_future`] rather than a hand-rolled
    /// layout, so the park-set tests below go through the same construction
    /// path every other future in the system does.
    fn test_future(poll: PollFn) -> *mut u8 {
        build_future(poll, STATE_MIN_SIZE, |_| {})
    }

    /// Run `f` with [`PARKED`] set to exactly `entries` for its duration,
    /// restoring whatever was there before once `f` returns.
    ///
    /// The precedent for splitting a diagnostic this way is
    /// [`deadlock_report`] itself: separating the text-only half of a
    /// diagnostic from the half that ends the process (`report_deadlock`) is
    /// what lets a test assert on it at all. This does the analogous job for
    /// the fixture a test needs `PARKED` to hold -- seeding it directly
    /// rather than driving a real `spawn_internal`/`poll_one` sequence just
    /// to get a specific `Wait` into the park set, and restoring the prior
    /// contents afterward rather than assuming `PARKED` started empty (the
    /// way this file's other `PARKED.with(|p| p.borrow_mut().clear())`
    /// tests do), so a test using this helper composes safely even nested
    /// inside one that does not.
    fn with_parked<T>(entries: &[(i64, Wait)], f: impl FnOnce() -> T) -> T {
        let previous = PARKED.with(|p| p.replace(entries.to_vec()));
        let result = f();
        PARKED.with(|p| *p.borrow_mut() = previous);
        result
    }

    #[test]
    fn block_on_runs_a_ready_future_and_returns_its_output() {
        let fut = make_future(poll_ready_now, 0);
        assert_eq!(unsafe { nova_rt_task_block_on(fut) }, 7);
    }

    #[test]
    fn block_on_re_polls_a_pending_future_until_it_completes() {
        // Discriminates a real re-queue from "poll once and return whatever
        // is in the output slot", which would return 0 here.
        let fut = make_future(poll_suspend_once, 0);
        assert_eq!(unsafe { nova_rt_task_block_on(fut) }, 42);
    }

    #[test]
    fn a_spawned_task_runs_to_completion_and_reports_done() {
        let fut = make_future(poll_suspend_once, 0);
        let id = unsafe { nova_rt_task_spawn(fut) };
        // Not done before the executor has run it at all.
        assert_eq!(unsafe { nova_rt_task_is_done(fut) }, 0);
        let root = make_future(poll_ready_now, 0);
        unsafe { nova_rt_task_block_on(root) };
        assert_eq!(unsafe { nova_rt_task_is_done(fut) }, 1);
        assert_eq!(unsafe { nova_rt_task_take_output(id) }, 42);
    }

    /// The lifetime of a task's GC root, asserted on the registry directly
    /// rather than through a collection.
    ///
    /// A heap-valued output (a `String`, record or sum returned by an
    /// `async fn`) is reachable from exactly one place once its task
    /// completes: [`STATE_SLOT_OUTPUT`] of the state object. `Task::output`'s
    /// copy roots nothing -- `TASKS` is a Rust-heap `Vec`, which is none of
    /// the collector's root sources. So unrooting the state at completion
    /// (which is what this executor originally did) leaves the output
    /// collectable from the moment the task finishes until
    /// `nova_rt_task_take_output` is called, and *anything* that allocates in
    /// between -- polling the next task in the queue, most obviously -- can
    /// trip the threshold and free it. `JoinHandle::join` returns exactly that
    /// value.
    ///
    /// Asserted here on `gc::root_count` rather than by allocating until a
    /// collection fires and checking the payload survived: the registry is
    /// where the invariant actually lives, and a `collect()`-based version
    /// would inherit the conservative scan's intermittent over-retention
    /// (`docs/adr/0010-conservative-scan-root-test-gating.md`) and could pass
    /// on an accidental stack root even with the rooting deleted. The other
    /// half of the argument -- that a `PINNED` root is traced *through*, so a
    /// pointer in the state object's output slot is marked too -- is
    /// `gc.rs`'s `a_registered_root_keeps_its_transitive_children_alive`.
    ///
    /// The counts are asserted exactly, not merely as non-zero, so this also
    /// discriminates a double `add_root` or a missing `remove_root` rather
    /// than only a missing root: releasing the root in `poll_one` on
    /// completion fails the middle assertion, never releasing it fails the
    /// last, and not registering it in `spawn_internal` at all fails the
    /// first.
    #[test]
    fn a_completed_tasks_state_stays_rooted_until_its_output_is_taken() {
        let fut = make_future(poll_ready_now, 0);
        let state = state_of(fut);
        let id = unsafe { nova_rt_task_spawn(fut) };
        assert_eq!(
            gc::root_count(state),
            1,
            "spawn must register the state object exactly once"
        );

        // Drive the queue to empty, which completes the spawned task.
        unsafe { nova_rt_task_block_on(make_future(poll_ready_now, 0)) };
        assert_eq!(unsafe { nova_rt_task_is_done(fut) }, 1);
        assert_eq!(
            gc::root_count(state),
            1,
            "a completed task's state must stay rooted until its output is \
             taken -- the output value lives in the state object's own output \
             slot, and Task::output's copy is in a Rust-heap Vec the \
             collector never scans"
        );

        assert_eq!(unsafe { nova_rt_task_take_output(id) }, 7);
        assert_eq!(
            gc::root_count(state),
            0,
            "take_output must release the state object's root exactly once, \
             so add_root/remove_root stay balanced"
        );
    }

    /// `release_internal` must release a task's stashed `fs` payload, not
    /// only its own state root.
    ///
    /// Calls `release_internal` directly -- it is private to this module,
    /// and this test lives in it -- rather than through the public
    /// `nova_rt_task_release`, and seeds the payload with
    /// `crate::fs::stash_for_test` rather than by driving a real poll:
    /// `release_internal` has no precondition on `Task::done` (only on
    /// `Task::taken`, which a freshly spawned task has not set), so a task
    /// that was spawned -- and therefore queued, by `spawn_internal` -- but
    /// never actually dequeued and polled already exercises it fully. This
    /// is what actually pins `release_internal`'s call into `fs.rs`, unlike
    /// `fs::tests::releasing_a_tasks_slots_drops_the_roots_it_held`, which
    /// calls `crate::fs::release_task_slots` directly and so cannot see
    /// whether `release_internal` still makes that call at all.
    #[test]
    fn releasing_a_task_also_releases_its_stashed_fs_payload() {
        let fut = make_future(poll_ready_now, 0);
        let id = unsafe { nova_rt_task_spawn(fut) };

        let payload = crate::gc_str("release-internal-payload");
        let addr = payload as usize;
        crate::fs::stash_for_test(id, crate::fs::Slot::Buffer, payload);
        assert_eq!(
            gc::root_count(addr),
            1,
            "stash_for_test must root its pointer"
        );

        release_internal(id);
        assert_eq!(
            gc::root_count(addr),
            0,
            "release_internal must release the task's stashed fs payload, \
             not only its own state root"
        );
    }

    /// `take_output_internal` must release a task's stashed `fs` payload,
    /// not only its own state root.
    ///
    /// Unlike `release_internal`'s sibling test above, `take_output_internal`
    /// asserts `Task::done`, so this drives one real `nova_rt_task_block_on`
    /// pass first to let `poll_one` mark the task done, and only stashes the
    /// payload afterward -- the ordering does not matter to `fs.rs`'s slot
    /// table, which knows nothing about task completion, but the task must
    /// be done before `take_output_internal` will accept it at all. Calls
    /// `take_output_internal` directly for the same reason the sibling test
    /// calls `release_internal` directly: only that pins the call into
    /// `fs.rs`, rather than `crate::fs::release_task_slots` itself, which
    /// `fs::tests::releasing_one_tasks_slots_leaves_another_tasks_intact`
    /// already covers.
    #[test]
    fn taking_a_tasks_output_also_releases_its_stashed_fs_payload() {
        let fut = make_future(poll_ready_now, 0);
        let id = unsafe { nova_rt_task_spawn(fut) };
        unsafe { nova_rt_task_block_on(make_future(poll_ready_now, 0)) };
        assert_eq!(unsafe { nova_rt_task_is_done(fut) }, 1);

        let payload = crate::gc_str("take-output-internal-payload");
        let addr = payload as usize;
        crate::fs::stash_for_test(id, crate::fs::Slot::Buffer, payload);
        assert_eq!(
            gc::root_count(addr),
            1,
            "stash_for_test must root its pointer"
        );

        assert_eq!(
            take_output_internal(id),
            7,
            "poll_ready_now's own output, unaffected by the stashed fs payload"
        );
        assert_eq!(
            gc::root_count(addr),
            0,
            "take_output_internal must release the task's stashed fs \
             payload, not only its own state root"
        );
    }

    /// `release_internal` must release all three of a task's stashed `fs`
    /// payload kinds, not only the one kind (`Buffer`) the two tests above
    /// happen to seed (final review, I2).
    ///
    /// **Mutation, confirmed to survive before this test existed** (measured
    /// directly against `3037908`, in an isolated worktree, apart from just
    /// re-running the review's own report): `release_task_slots` collecting
    /// `[entry.buffer, 0, 0]` instead of
    /// `[entry.buffer, entry.array, entry.message]` passed all 78
    /// `nova-runtime` lib tests. Every test that stashed a payload for the
    /// *same task* it then released seeded only `Slot::Buffer` -- both tests
    /// above do, and `stash_for_test` hardcoded `Buffer` until this test
    /// needed otherwise.
    /// `fs::tests::releasing_one_tasks_slots_leaves_another_tasks_intact`
    /// does stash into `Slot::Message`, but for a *different* task than the
    /// one it releases, so it asserts that unrelated root is left alone, not
    /// that a same-task `Message`/`Array` root is actually dropped. This is
    /// what an undrained OS error message (`fail` stashes one on every
    /// failure path) or an undrained `read_dir` array would ride on to keep
    /// its root registered for the process lifetime.
    #[test]
    fn releasing_a_task_releases_all_three_stashed_fs_slot_kinds() {
        let fut = make_future(poll_ready_now, 0);
        let id = unsafe { nova_rt_task_spawn(fut) };

        let buffer = crate::gc_str("all-three-buffer-payload");
        let array = crate::gc_str("all-three-array-payload");
        let message = crate::gc_str("all-three-message-payload");
        let addrs = [buffer as usize, array as usize, message as usize];

        crate::fs::stash_for_test(id, crate::fs::Slot::Buffer, buffer);
        crate::fs::stash_for_test(id, crate::fs::Slot::Array, array);
        crate::fs::stash_for_test(id, crate::fs::Slot::Message, message);
        for addr in addrs {
            assert_eq!(
                gc::root_count(addr),
                1,
                "stash_for_test must root each of the three payloads"
            );
        }

        release_internal(id);
        for addr in addrs {
            assert_eq!(
                gc::root_count(addr),
                0,
                "release_internal must release all three of the task's \
                 stashed fs payload roots -- buffer, array and message alike \
                 -- not only whichever kind a caller happened to seed"
            );
        }
    }

    /// `take_output` is a take, not a peek: the second call must be diagnosed,
    /// because the first one released the root that was keeping a heap-valued
    /// output alive, so the bits it would hand back a second time may name a
    /// freed object.
    #[test]
    fn taking_an_output_twice_panics_rather_than_returning_stale_bits() {
        let fut = make_future(poll_ready_now, 0);
        let id = unsafe { nova_rt_task_spawn(fut) };
        unsafe { nova_rt_task_block_on(make_future(poll_ready_now, 0)) };
        assert_eq!(unsafe { nova_rt_task_take_output(id) }, 7);

        let r = std::panic::catch_unwind(|| unsafe { nova_rt_task_take_output(id) });
        assert!(r.is_err(), "a second take must panic, not succeed");
    }

    /// Taking the output of a task that has not finished would return the
    /// `output: 0` initializer, indistinguishable from a task that genuinely
    /// completed with `0`. `JoinHandle::join` is specified to poll
    /// `nova_rt_task_is_done` first, so reaching this is a caller bug and is
    /// diagnosed as one.
    #[test]
    fn taking_the_output_of_an_unfinished_task_panics() {
        let fut = make_future(poll_suspend_once, 0);
        let id = unsafe { nova_rt_task_spawn(fut) };
        assert_eq!(unsafe { nova_rt_task_is_done(fut) }, 0);

        let r = std::panic::catch_unwind(|| unsafe { nova_rt_task_take_output(id) });
        assert!(
            r.is_err(),
            "taking an unfinished task's output must panic, not return 0"
        );
    }

    /// A poll function returning something that is neither [`POLL_PENDING`]
    /// nor [`POLL_READY`] is a codegen bug. This module exists to pin the poll
    /// ABI *before* codegen depends on it, so the status is checked against
    /// both constants rather than "anything not pending is ready" -- otherwise
    /// a generated poll returning `2` silently completes the task with
    /// whatever happens to be in the output slot.
    #[test]
    fn an_out_of_range_poll_status_panics_rather_than_completing_the_task() {
        unsafe extern "C-unwind" fn poll_bogus_status(_s: *mut u8, _c: *mut u8) -> i64 {
            2
        }
        let fut = make_future(poll_bogus_status, 0);
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| unsafe {
            nova_rt_task_block_on(fut)
        }));
        let payload = r.expect_err("an unknown poll status must panic");
        let msg = payload
            .downcast_ref::<String>()
            .map(String::as_str)
            .unwrap_or("");
        assert!(
            msg.contains("returned 2"),
            "the panic must name the offending status, got {msg:?}"
        );
    }

    #[test]
    fn two_pending_tasks_interleave_round_robin() {
        // The determinism the sub-phase gate depends on. Records the order
        // poll calls actually happen in, and asserts an ALTERNATING order --
        // not merely that both ran, which a run-to-completion scheduler would
        // also satisfy while producing different output in the gate fixture.
        // Thread-local, not a process-global `Mutex`: the executor whose order
        // this records is itself thread-local (`QUEUE`/`TASKS` above), so a
        // process-global recorder would let any concurrently running test that
        // drove these same poll functions interleave into the sequence being
        // asserted. Nothing else uses `poll_a`/`poll_b` today, which is why
        // the global version passed; the scope of the recorder should still
        // match the scope of the thing recorded.
        thread_local! {
            static ORDER: RefCell<Vec<i64>> = const { RefCell::new(Vec::new()) };
        }
        ORDER.with(|o| o.borrow_mut().clear());

        unsafe extern "C-unwind" fn poll_a(state: *mut u8, _c: *mut u8) -> i64 {
            unsafe { record(state, 1) }
        }
        unsafe extern "C-unwind" fn poll_b(state: *mut u8, _c: *mut u8) -> i64 {
            unsafe { record(state, 2) }
        }
        unsafe fn record(state: *mut u8, who: i64) -> i64 {
            let slots = state as *mut i64;
            // SAFETY: `state` is a live state object (`make_future`'s
            // contract).
            let tag = unsafe { *slots.add(STATE_SLOT_TAG) };
            ORDER.with(|o| o.borrow_mut().push(who));
            if tag < 2 {
                unsafe { *slots.add(STATE_SLOT_TAG) = tag + 1 };
                POLL_PENDING
            } else {
                unsafe { *slots.add(STATE_SLOT_OUTPUT) = who };
                POLL_READY
            }
        }

        let a = make_future(poll_a, 0);
        let b = make_future(poll_b, 0);
        unsafe {
            nova_rt_task_spawn(a);
            nova_rt_task_spawn(b);
            nova_rt_task_block_on(make_future(poll_suspend_once, 0));
        }
        let order = ORDER.with(|o| o.borrow().clone());
        let a_positions: Vec<usize> = order
            .iter()
            .enumerate()
            .filter(|(_, &w)| w == 1)
            .map(|(i, _)| i)
            .collect();
        let b_positions: Vec<usize> = order
            .iter()
            .enumerate()
            .filter(|(_, &w)| w == 2)
            .map(|(i, _)| i)
            .collect();
        assert!(
            a_positions.len() >= 3 && b_positions.len() >= 3,
            "order = {order:?}"
        );
        assert!(
            a_positions[0] < b_positions[0] && b_positions[0] < a_positions[1],
            "expected round-robin interleaving, got {order:?}"
        );
    }

    /// The re-entrancy guard is armed for exactly the span of one `block_on`
    /// call, and a poll function running inside one can see that it is.
    ///
    /// The guard's own diagnostic ends the process (`abort_with`, because a
    /// nested call means a *generated* frame is on the path an unwind would
    /// have to take), so what is observable in-process is the flag it reads
    /// rather than the abort. Both halves discriminate a real defect: a guard
    /// that never armed would let a nested `block_on` run a second executor
    /// loop from inside the first one's frame, and one that never disarmed
    /// would end the process on the second, unrelated, `block_on` in a
    /// program. The abort itself is asserted end to end, on a real compiled
    /// program, by `nova-cli`'s `run_aborts_when_an_async_fn_calls_block_on`.
    #[test]
    fn the_re_entrancy_guard_is_armed_only_inside_block_on() {
        thread_local! {
            static SEEN: Cell<bool> = const { Cell::new(false) };
        }
        unsafe extern "C-unwind" fn poll_observes_guard(state: *mut u8, _c: *mut u8) -> i64 {
            SEEN.with(|s| s.set(IN_BLOCK_ON.with(|g| g.get())));
            // SAFETY: `state` is a live state object (`make_future`'s contract).
            unsafe { *(state as *mut i64).add(STATE_SLOT_OUTPUT) = 3 };
            POLL_READY
        }
        assert!(
            !IN_BLOCK_ON.with(|g| g.get()),
            "the guard must not be armed before any block_on"
        );
        assert_eq!(
            unsafe { nova_rt_task_block_on(make_future(poll_observes_guard, 0)) },
            3
        );
        assert!(
            SEEN.with(|s| s.get()),
            "the guard must be armed while a poll function runs"
        );
        assert!(
            !IN_BLOCK_ON.with(|g| g.get()),
            "the guard must be disarmed once block_on returns"
        );
    }

    /// `nova_rt_task_release` ends the executor's claim on a task's state
    /// object and can be called any number of times.
    ///
    /// The exact root counts, not merely "eventually zero": releasing twice
    /// must not cancel a registration it does not own. Simulated here with a
    /// second, independent `gc::add_root` on the same state -- spawning the
    /// same future twice no longer reaches this (`spawn_internal` now rejects
    /// it), so this stands in for any other root on the object, which release
    /// must leave alone. Asserted on the registry rather than through a
    /// collection, for the reason
    /// `a_completed_tasks_state_stays_rooted_until_its_output_is_taken`
    /// documents.
    #[test]
    fn releasing_a_task_unroots_its_state_exactly_once_however_often_it_is_called() {
        let fut = make_future(poll_ready_now, 0);
        let state = state_of(fut);
        unsafe { nova_rt_task_spawn(fut) };
        // A second, independent registration of the same state: it must
        // survive the release below (see this test's doc comment).
        gc::add_root(state as *mut u8);
        assert_eq!(gc::root_count(state), 2);

        unsafe { nova_rt_task_release(fut) };
        assert_eq!(
            gc::root_count(state),
            1,
            "release must cancel the executor's own registration"
        );
        unsafe {
            nova_rt_task_release(fut);
            nova_rt_task_release(fut);
        }
        assert_eq!(
            gc::root_count(state),
            1,
            "a repeated release must be a no-op, not another remove_root"
        );
        gc::remove_root(state as *mut u8);
    }

    /// A released task's output is still readable out of the state object the
    /// future points at -- which is what `JoinHandle::join` does, so that a
    /// second `join` on one handle neither panics nor reads stale bits.
    #[test]
    fn a_released_tasks_output_slot_still_holds_its_value() {
        let fut = make_future(poll_ready_now, 0);
        unsafe { nova_rt_task_spawn(fut) };
        unsafe { nova_rt_task_block_on(make_future(poll_ready_now, 0)) };
        assert_eq!(unsafe { nova_rt_task_is_done(fut) }, 1);
        unsafe { nova_rt_task_release(fut) };
        // SAFETY: `fut` is a live future fat pointer; `state_of` reads its
        // state word, and `STATE_SLOT_OUTPUT` is in bounds by `make_future`'s
        // minimum size.
        let out = unsafe { (state_of(fut) as *mut i64).add(STATE_SLOT_OUTPUT).read() };
        assert_eq!(
            out, 7,
            "the value stays in the slot the poll fn wrote it to"
        );
    }

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
        //
        // One poll each via `poll_one`, not a full drain through `block_on`:
        // `block_on` drains the whole shared queue, and by the time it empties,
        // every task queued on this thread -- `a` AND `b` -- has reported
        // `POLL_READY`, so both handles would read back `1` even from a lookup
        // that resolved every future to the same task (any id in this test is
        // done by then). One poll each keeps `a` (`poll_ready_now`) done and
        // `b` (`poll_suspend_once`, which needs a second poll) not done, so the
        // two answers must disagree for this test to mean anything.
        let a = make_future(poll_ready_now, 0);
        let b = make_future(poll_suspend_once, 0);
        let (id_a, id_b) = unsafe { (nova_rt_task_spawn(a), nova_rt_task_spawn(b)) };
        unsafe {
            poll_one(id_a);
            poll_one(id_b);
        }
        assert_eq!(unsafe { nova_rt_task_is_done(a) }, 1);
        assert_eq!(
            unsafe { nova_rt_task_is_done(b) },
            0,
            "b needs a second poll; reporting it done means the lookup did not \
             distinguish the two futures"
        );
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

        unsafe { nova_rt_task_release(a) };

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
        unsafe {
            nova_rt_task_spawn(fut);
            nova_rt_task_release(fut);
            nova_rt_task_release(fut);
        }
        assert_eq!(gc::root_count(state), 0);
    }

    #[test]
    fn spawning_the_same_future_again_after_release_succeeds() {
        // Presence in `BY_STATE` is not what must be rejected. What has to
        // stay rejected is two LIVE tasks sharing one state object, which is
        // what would make the map ambiguous; a released task's entry must not
        // block re-spawning the future it belongs to. Re-spawning after
        // release is a deliberately accepted footgun rather than a case the
        // check cannot see -- see `spawn_internal`'s own comment for why it is
        // accepted -- and this test is what pins the acceptance, so a
        // "tightening" of the check to bare presence fails here.
        let fut = make_future(poll_ready_now, 0);
        let id1 = unsafe { nova_rt_task_spawn(fut) };
        unsafe { nova_rt_task_release(fut) };
        let id2 = unsafe { nova_rt_task_spawn(fut) };
        assert_ne!(
            id1, id2,
            "the second spawn must register a new task, not reuse the first's"
        );
        unsafe { nova_rt_task_block_on(make_future(poll_ready_now, 0)) };
        assert_eq!(
            unsafe { nova_rt_task_is_done(fut) },
            1,
            "the re-spawned task must actually run, not merely avoid aborting"
        );
    }

    /// A freed state object's `BY_STATE` key goes with it, so the address
    /// cannot resolve a later, unrelated object to the old task.
    ///
    /// The read path has no staleness test -- a hit is returned as the answer
    /// -- so the whole of its correctness is that a key cannot outlive the
    /// object it names. Asserted on the map directly, and on the deterministic
    /// sweep (`gc::sweep_with_roots_for_test`, an explicit root set and no
    /// stack scan) rather than on a real collection: what is being checked is
    /// that *sweeping an object* drops its key, and handing the root set in is
    /// what makes "this object was swept" a fact about the argument instead of
    /// a fact about whatever the conservative scanner happened to retain
    /// (`docs/adr/0010-conservative-scan-root-test-gating.md`, and the
    /// `#[ignore]`d tests below that pay for it). It also keeps this test on
    /// every platform, where a `collect()`-based version would assert nothing
    /// wherever `gc.rs`'s `stack_base` has no implementation.
    ///
    /// The key's presence is asserted *before* the sweep as well, so a
    /// `spawn_internal` that stopped inserting could not make this pass by
    /// having nothing to remove.
    #[test]
    fn a_swept_states_key_is_dropped_so_a_recycled_address_cannot_misresolve() {
        let fut = make_future(poll_ready_now, 0);
        let state = state_of(fut);
        unsafe { nova_rt_task_spawn(fut) };
        unsafe { nova_rt_task_block_on(make_future(poll_ready_now, 0)) };
        // Ends the executor's own claim, which is what lets the state be swept
        // at all -- while a task holds its root, the collector keeps it.
        unsafe { nova_rt_task_release(fut) };
        assert!(
            BY_STATE.with(|m| m.borrow().contains_key(&state)),
            "spawn must have registered this state, or the removal below \
             proves nothing"
        );

        // An empty root set: nothing is reachable, so this state really is
        // freed. Every other object this thread allocated is freed with it, so
        // nothing below dereferences any of them.
        gc::sweep_with_roots_for_test(&[]);

        assert!(
            !BY_STATE.with(|m| m.borrow().contains_key(&state)),
            "a freed state's key must not survive it: the address can be \
             handed to an unrelated object, and this map is what the two \
             Nova-facing reads resolve, so a surviving key answers about the \
             old task instead of rejecting a future that was never spawned"
        );
    }

    /// The other half, and what makes pruning compatible with `join`'s
    /// idempotence rather than a trade against it: a state object a caller can
    /// still reach keeps its key across a collection.
    ///
    /// `join` releases and then reads, and a second `join` on the same handle
    /// has to resolve the same task again (`std/task`'s `JoinHandle::join`).
    /// Released is not swept: the handle still holds the future, the future is
    /// a scanned two-word object holding the state address, so tracing the
    /// future marks the state. The root set here is the future alone --
    /// deliberately *not* the state -- so the state survives only if it is
    /// reached *through* the future, which is the actual mechanism.
    ///
    /// Without this, "prune at release" and "prune when freed" would be
    /// indistinguishable, and the first breaks idempotence.
    #[test]
    fn a_reachable_futures_key_survives_a_collection_so_a_second_read_resolves() {
        let fut = make_future(poll_ready_now, 0);
        let state = state_of(fut);
        unsafe { nova_rt_task_spawn(fut) };
        unsafe { nova_rt_task_block_on(make_future(poll_ready_now, 0)) };
        unsafe { nova_rt_task_release(fut) };

        gc::sweep_with_roots_for_test(&[fut as usize]);

        assert_eq!(
            gc::object_info(state).map(|(_, scan)| scan),
            Some(true),
            "the state must have been reached through the future's own words, \
             or this test is asserting nothing about tracing"
        );
        assert!(
            BY_STATE.with(|m| m.borrow().contains_key(&state)),
            "a state a handle can still reach must keep its key, or a second \
             join aborts on a handle that did nothing wrong"
        );
        assert_eq!(
            unsafe { nova_rt_task_is_done(fut) },
            1,
            "and the surviving key must still resolve to that task"
        );
    }

    /// The exact layout `nova_rt_task_yield_future` builds, read back from the
    /// collector's own records.
    ///
    /// This is the second place in the runtime that constructs a heap value the
    /// *compiler* also constructs (`nova_rt_str_chars` was the first), and a
    /// layout that disagrees with the compiler's is a silent miscompile rather
    /// than a failure. So both allocations are checked against the tracked
    /// `(size, scan)` and not only against the words stored in them: reading
    /// words back cannot see an allocation that is too small but has slop after
    /// it, and cannot see the `scan` flag at all -- and an unscanned state
    /// object is marked but never traced (`gc.rs`'s `if !scan { continue; }`).
    #[test]
    fn the_yield_futures_layout_is_the_one_the_abi_declares() {
        let fut = nova_rt_task_yield_future();
        assert_eq!(
            gc::object_info(fut as usize),
            Some((FUTURE_SIZE, true)),
            "the fat pointer must be exactly the two-word future, scanned"
        );
        let state = state_of(fut);
        assert_eq!(
            gc::object_info(state),
            Some((STATE_MIN_SIZE, true)),
            "the state object must be at least the tag and output slots, scanned"
        );
        // That `STATE_MIN_SIZE` really does cover the output slot is checked at
        // compile time, next to `YIELD_STATE_SIZE`.
        //
        // SAFETY: `fut` is this call's own `FUTURE_SIZE`-byte block.
        let poll = unsafe { (fut as *mut usize).add(FUTURE_SLOT_POLL).read() };
        let expected: PollFn = poll_yield_once;
        assert_eq!(
            poll, expected as usize,
            "word 0 must be the poll function's address, not the state's"
        );
    }

    /// The exact layout `nova_rt_task_sleep_future_nanos` builds, read back
    /// from the collector's own records -- the same discipline as
    /// `the_yield_futures_layout_is_the_one_the_abi_declares`, and for the
    /// identical reason, but this one is load-bearing in a way that test is
    /// not: a review of this function caught a `build_future` call site
    /// passing `STATE_MIN_SIZE` where `SLEEP_STATE_SIZE` belonged -- one word
    /// short of what `nova_rt_task_sleep_future_nanos`'s own `init` closure
    /// writes -- and neither existing guard caught it. The compile-time
    /// assertion beside `SLEEP_STATE_SIZE` is a relation between two
    /// constants and is blind to what a call site actually passes;
    /// `build_future`'s own `debug_assert!(state_size >= STATE_MIN_SIZE)` is
    /// a floor, and `STATE_MIN_SIZE` itself satisfies its own floor. Reading
    /// the words back out of the state object cannot catch it either, for
    /// the same reason the sibling test's doc comment gives: a too-small
    /// allocation with slop after it still reads back whatever was written,
    /// silently, until something else happens to occupy that word. Only the
    /// collector's own recorded size distinguishes "16 bytes, and `nanos`
    /// was written one word past the end" from "24 bytes, exactly as
    /// declared."
    #[test]
    fn the_sleep_futures_layout_is_the_one_the_abi_declares() {
        let fut = nova_rt_task_sleep_future_nanos(0);
        assert_eq!(
            gc::object_info(fut as usize),
            Some((FUTURE_SIZE, true)),
            "the fat pointer must be exactly the two-word future, scanned"
        );
        let state = state_of(fut);
        assert_eq!(
            gc::object_info(state),
            Some((SLEEP_STATE_SIZE, true)),
            "the state object must be the ABI minimum plus the one temp slot \
             holding `nanos`, scanned"
        );
        // SAFETY: `fut` is this call's own `FUTURE_SIZE`-byte block.
        let poll = unsafe { (fut as *mut usize).add(FUTURE_SLOT_POLL).read() };
        let expected: PollFn = poll_sleep;
        assert_eq!(
            poll, expected as usize,
            "word 0 must be the poll function's address, not the state's"
        );
    }

    /// A 50ms sleep must stage a deadline roughly 50ms out -- the magnitude,
    /// not merely the presence of some deadline.
    ///
    /// `before` is captured **outside** the poll on purpose. The staged
    /// deadline is computed at or after `before`, so by monotonicity the
    /// difference is at least the requested 50ms however long this thread is
    /// descheduled; measuring from after the poll could drift below any lower
    /// bound and flake. The window is loose against jitter and tight against
    /// unit errors: a millionfold overshoot stages ~50,000 seconds and fails
    /// the upper bound, a millionfold undershoot stages ~50ns and fails the
    /// lower one.
    #[test]
    fn a_sleep_stages_a_deadline_of_the_right_magnitude() {
        let before = Instant::now();
        let fut = nova_rt_task_sleep_future_nanos(50 * 1_000_000);
        // SAFETY: `fut` is this call's own `FUTURE_SIZE`-byte block, so word 0
        // is its poll function -- exactly how
        // `the_sleep_futures_layout_is_the_one_the_abi_declares` reads it.
        let poll_addr = unsafe { (fut as *mut usize).add(FUTURE_SLOT_POLL).read() };
        // SAFETY: that word is a `PollFn` bit pattern by `fut`'s own
        // construction; a fn pointer and a `usize` are both pointer-width.
        let poll: PollFn = unsafe { std::mem::transmute(poll_addr) };
        let state = state_of(fut) as *mut u8;

        // `stage_park` aborts outside a task context, so borrow one for this
        // single manual poll and put back whatever was there.
        let previous = current_task();
        set_current_for_test(Some(0));
        // SAFETY: `poll`/`state` are the pair inside `fut`; `task_ctx` is
        // always null, matching every other call site in this crate.
        let first = unsafe { poll(state, std::ptr::null_mut()) };
        let staged = staged_deadline_for_test();
        // This manual poll never goes through `poll_one`'s own cleanup, so
        // unlike a real poll, the `Wait::Deadline` it just staged is left
        // sitting in `PENDING_PARK` unless something here drains it. Left in
        // place, the next test to stage a park on this thread -- if one runs
        // before anything else clears it, which `--test-threads=1` makes
        // deterministic rather than merely possible -- collides with it, and
        // `stage_park` aborts the whole process over two parks it never
        // staged itself. Draining with the same `Cell::take` `poll_one` uses
        // (line ~674) resets `PENDING_PARK` to empty, exactly as if this
        // poll's park had been committed and later cleared.
        PENDING_PARK.with(|cell| {
            cell.take();
        });
        set_current_for_test(previous);

        assert_eq!(first, POLL_PENDING, "a sleep must park on its first poll");
        let staged = staged.expect("a sleep must stage a deadline");
        let delta = staged.duration_since(before);
        assert!(
            delta >= Duration::from_millis(40) && delta <= Duration::from_millis(500),
            "a 50ms sleep staged a deadline {delta:?} out -- a unit error, not jitter"
        );
    }

    /// A sleep polled before its deadline must re-stage **the same deadline**
    /// and stay pending.
    ///
    /// Asserted by identity, not existence: a mutant that re-stages
    /// `Instant::now()` satisfies "some deadline is staged" and then spins
    /// forever, which an existence check cannot distinguish from correct.
    #[test]
    fn a_sleep_polled_early_re_stages_the_same_deadline() {
        let fut = nova_rt_task_sleep_future_nanos(60 * 1_000_000_000);
        let poll_addr = unsafe { (fut as *mut usize).add(FUTURE_SLOT_POLL).read() };
        // SAFETY: word 0 is a `PollFn` bit pattern by `fut`'s own construction.
        let poll: PollFn = unsafe { std::mem::transmute(poll_addr) };
        let state = state_of(fut) as *mut u8;

        let previous = current_task();
        set_current_for_test(Some(0));
        // SAFETY: `poll`/`state` are the pair inside `fut`; `task_ctx` is null.
        let first = unsafe { poll(state, std::ptr::null_mut()) };
        let staged_first = staged_deadline_for_test();
        PENDING_PARK.with(|cell| {
            cell.take();
        });
        // SAFETY: same pair, polled a second time before the deadline.
        let second = unsafe { poll(state, std::ptr::null_mut()) };
        let staged_second = staged_deadline_for_test();
        PENDING_PARK.with(|cell| {
            cell.take();
        });
        set_current_for_test(previous);

        assert_eq!(
            first, POLL_PENDING,
            "a 60s sleep must park on its first poll"
        );
        assert_eq!(
            second, POLL_PENDING,
            "and must stay pending when polled again before its deadline"
        );
        assert_eq!(
            staged_second, staged_first,
            "the re-staged deadline must be the original, not a fresh one"
        );
    }

    /// A sleep whose deadline has passed completes.
    #[test]
    fn a_sleep_polled_after_its_deadline_completes() {
        let fut = nova_rt_task_sleep_future_nanos(0);
        let poll_addr = unsafe { (fut as *mut usize).add(FUTURE_SLOT_POLL).read() };
        // SAFETY: as above.
        let poll: PollFn = unsafe { std::mem::transmute(poll_addr) };
        let state = state_of(fut) as *mut u8;

        let previous = current_task();
        set_current_for_test(Some(0));
        // SAFETY: as above.
        let status = unsafe { poll(state, std::ptr::null_mut()) };
        PENDING_PARK.with(|cell| {
            cell.take();
        });
        set_current_for_test(previous);

        assert_eq!(status, POLL_READY, "a zero-nanosecond sleep is already due");
    }

    /// `instant_from_deadline_nanos`'s result must never be treated as
    /// already due while the deadline it encodes is still in the future --
    /// the property the old `Instant::now()` overflow fallback broke (see
    /// that function's own doc comment). `i64::MAX` is the largest deadline
    /// [`deadline_nanos_from_now`] can ever produce, so it is the input this
    /// crate's own API can actually construct that stresses the conversion
    /// hardest.
    #[test]
    fn instant_from_deadline_nanos_of_i64_max_is_not_already_due() {
        let before = Instant::now();
        let staged = instant_from_deadline_nanos(i64::MAX);
        assert!(
            staged >= before,
            "a deadline this far in the future must not stage an instant \
             that is already due"
        );
    }

    /// Forces [`furthest_representable_instant`]'s halving fallback directly,
    /// since nothing reachable through this crate's own sleep API can:
    /// `deadline_nanos_from_now` bounds every deadline to at most `i64::MAX`
    /// nanoseconds (~292 years), and `checked_add` of that many nanoseconds
    /// from a process-start `Instant` succeeds on every supported backend --
    /// confirmed below, not assumed. `Duration::MAX` (~584 billion years) is
    /// the one value guaranteed to overflow it, and `Instant` offers no
    /// public way to sit near its own overflow boundary the way `Duration`
    /// does, so this is the only lever available to actually exercise the
    /// loop rather than merely asserting its result type is right.
    #[test]
    fn furthest_representable_instant_forces_the_halving_fallback_and_is_not_already_due() {
        let base = Instant::now();
        assert!(
            base.checked_add(Duration::MAX).is_none(),
            "this test only exercises the halving loop if Duration::MAX \
             overflows Instant::checked_add on this platform; if that ever \
             changes, this assertion (not the one below) is what will fail"
        );

        let before = Instant::now();
        let staged = furthest_representable_instant(base, Duration::MAX);
        assert!(
            staged >= before,
            "the furthest representable instant must not be treated as \
             already due"
        );
    }

    /// The inner future is polled **before** the deadline is checked, so work
    /// that completed is never reported as timed out -- and a zero-duration
    /// timeout over an already-ready future succeeds.
    ///
    /// Reversing the order in `poll_timeout` fails exactly this.
    #[test]
    fn a_zero_duration_timeout_over_a_ready_future_reports_completed() {
        let inner = nova_rt_task_sleep_future_nanos(0);
        let fut = nova_rt_task_timeout_future(0, inner);
        let status = poll_once_for_test(fut);
        assert_eq!(status, POLL_READY);
        assert_eq!(
            output_of_for_test(fut),
            TIMEOUT_STATUS_COMPLETED,
            "the inner future was ready, so this must not report elapsed"
        );
    }

    /// A past deadline over a future that will not complete reports elapsed.
    #[test]
    fn an_expired_timeout_over_a_pending_future_reports_elapsed() {
        let inner = nova_rt_task_sleep_future_nanos(60 * 1_000_000_000);
        let fut = nova_rt_task_timeout_future(0, inner);
        let status = poll_once_for_test(fut);
        assert_eq!(status, POLL_READY);
        assert_eq!(output_of_for_test(fut), TIMEOUT_STATUS_ELAPSED);
    }

    /// A live timeout over a pending inner parks, and the staged park carries
    /// **both** the inner's wait and this timeout's deadline, merged.
    ///
    /// Written with the raw poll idiom rather than `poll_once_for_test`,
    /// because that helper drains `PENDING_PARK` before returning and this
    /// test must read `staged_deadline_for_test()` **before** the drain.
    #[test]
    fn a_live_timeout_over_a_pending_future_stages_both_deadlines_merged() {
        let inner = nova_rt_task_sleep_future_nanos(60 * 1_000_000_000);
        let fut = nova_rt_task_timeout_future(1_000_000, inner);
        let before = Instant::now();
        // SAFETY: `fut` is this call's own `FUTURE_SIZE`-byte block, so word 0
        // is its poll function.
        let poll_addr = unsafe { (fut as *mut usize).add(FUTURE_SLOT_POLL).read() };
        // SAFETY: that word is a `PollFn` bit pattern by `fut`'s own
        // construction; a fn pointer and a `usize` are both pointer-width.
        let poll: PollFn = unsafe { std::mem::transmute(poll_addr) };
        let state = state_of(fut) as *mut u8;

        let previous = current_task();
        set_current_for_test(Some(0));
        // SAFETY: `poll`/`state` are the pair inside `fut`; `task_ctx` is
        // always null, matching every other call site in this crate.
        let status = unsafe { poll(state, std::ptr::null_mut()) };
        let staged = staged_deadline_for_test();
        PENDING_PARK.with(|cell| {
            cell.take();
        });
        set_current_for_test(previous);

        assert_eq!(
            status, POLL_PENDING,
            "neither the inner nor the deadline is done"
        );
        let staged = staged.expect("a park must be staged");
        let delta = staged.duration_since(before);
        assert!(
            delta <= Duration::from_secs(1),
            "the merged deadline must be the timeout's 1ms, not the inner's \
             60s -- got {delta:?}"
        );
    }

    /// Everything one poll left staged in `PENDING_PARK`, read before the
    /// drain -- the three [`Staged`] fields as the three `*_for_test`
    /// accessors report them, `io` keeping the deadline that rides *inside*
    /// the I/O wait rather than folding it away.
    #[derive(Debug)]
    struct StagedForTest {
        deadline: Option<Instant>,
        io: Option<(RawSocket, Interest, Option<Instant>)>,
        task: Option<i64>,
    }

    /// Poll `fut` once under a borrowed task context, optionally with
    /// `pre_staged` already staged first, and hand back both the status and
    /// everything left staged.
    ///
    /// `poll_once_for_test` cannot serve these tests: it drains
    /// `PENDING_PARK` before returning and hands back only the status, so the
    /// leftover-park property is precisely what it destroys. The drain still
    /// has to happen -- a park left staged aborts the next test to stage one
    /// under `--test-threads=1` -- so it happens here, after the read.
    ///
    /// `pre_staged` is what makes "restores what it found" testable at all:
    /// with an empty slot going in, restoring a snapshot and clearing to
    /// `Staged::default()` are indistinguishable.
    fn poll_once_and_read_staged_for_test(
        fut: *mut u8,
        pre_staged: Option<Wait>,
    ) -> (i64, StagedForTest) {
        let poll_addr = unsafe { (fut as *mut usize).add(FUTURE_SLOT_POLL).read() };
        // SAFETY: word 0 is a `PollFn` bit pattern by `fut`'s own construction.
        let poll: PollFn = unsafe { std::mem::transmute(poll_addr) };
        let state = state_of(fut) as *mut u8;

        let previous = current_task();
        set_current_for_test(Some(0));
        if let Some(wait) = pre_staged {
            stage_park(wait);
        }
        // SAFETY: `poll`/`state` are the pair inside `fut`; `task_ctx` is
        // always null, matching every other call site in this crate.
        let status = unsafe { poll(state, std::ptr::null_mut()) };
        let staged = StagedForTest {
            deadline: staged_deadline_for_test(),
            io: staged_io_for_test(),
            task: staged_task_for_test(),
        };
        PENDING_PARK.with(|cell| {
            cell.take();
        });
        set_current_for_test(previous);
        (status, staged)
    }

    /// A poll function that parks on a **non-deadline** wait -- a bare
    /// `Wait::Task` -- and never completes.
    ///
    /// Non-deadline on purpose. A `Wait::Deadline` left behind by an abandoned
    /// inner merges with the next suspension's deadline and is then corrected
    /// by level-triggering, so it cannot expose a leftover park; a
    /// `Wait::Task` or a `Wait::Io` collides and aborts the process. Every
    /// shipped fixture and every unit test used a `sleep` inner, which stages
    /// only a deadline, which is the entire reason the leftover went unseen.
    ///
    /// `id: 0` names no task that has to exist: `stage_park` does not resolve
    /// ids, and nothing here reaches `staged_to_wait` or the wake paths.
    unsafe extern "C-unwind" fn poll_parks_on_a_task_forever(
        _state: *mut u8,
        _task_ctx: *mut u8,
    ) -> i64 {
        stage_park(Wait::Task {
            id: 0,
            deadline: None,
        });
        POLL_PENDING
    }

    /// A poll function that parks on a `Wait::Io` and never completes -- the
    /// other non-deadline wait, and the one `staged_io_for_test` reads.
    unsafe extern "C-unwind" fn poll_parks_on_an_io_forever(
        _state: *mut u8,
        _task_ctx: *mut u8,
    ) -> i64 {
        stage_park(Wait::Io {
            socket: RawSocket(9),
            interest: Interest::Read,
            deadline: None,
        });
        POLL_PENDING
    }

    /// A poll function that stages a park and *then* returns `POLL_READY` --
    /// exactly the shape `poll_one`'s "taken unconditionally" comment
    /// describes, reproduced here so `poll_timeout`'s completed exit can be
    /// held to the same postcondition as its elapsed one.
    unsafe extern "C-unwind" fn poll_parks_then_completes(
        state: *mut u8,
        _task_ctx: *mut u8,
    ) -> i64 {
        stage_park(Wait::Task {
            id: 0,
            deadline: None,
        });
        // SAFETY: `state` is a `STATE_MIN_SIZE` state object from
        // `make_future`, so the output slot is in bounds.
        unsafe { (state as *mut i64).add(STATE_SLOT_OUTPUT).write(0) };
        POLL_READY
    }

    /// The other half of spec §6.3's structural pending test: a live timeout
    /// over an inner that parks on something that is **not** a deadline stages
    /// the inner's own wait *and* the timeout's deadline, merged into it.
    ///
    /// `a_live_timeout_over_a_pending_future_stages_both_deadlines_merged`
    /// covers the deadline half with a `sleep` inner, which stages only a
    /// deadline, so `staged_io_for_test` went unexercised by any
    /// `poll_timeout` test and no unit test polled a timeout over a
    /// non-deadline inner at all. This is the assertion that was one line from
    /// finding the abandonment bug: it is the same `StagedForTest` read, on
    /// the same inner shape, differing only in whether the deadline has
    /// already passed.
    #[test]
    fn a_live_timeout_over_an_io_parking_inner_stages_the_io_wait_and_the_deadline() {
        let inner = make_future(poll_parks_on_an_io_forever, 0);
        let before = Instant::now();
        let fut = nova_rt_task_timeout_future(5 * 1_000_000_000, inner);

        let (status, staged) = poll_once_and_read_staged_for_test(fut, None);

        assert_eq!(
            status, POLL_PENDING,
            "neither the inner nor the 5s deadline is done"
        );
        let (socket, interest, io_deadline) = staged
            .io
            .expect("the inner's I/O wait must survive into the staged park");
        assert_eq!((socket, interest), (RawSocket(9), Interest::Read));
        let io_deadline = io_deadline.expect(
            "the timeout's own deadline must ride *inside* the inner's I/O \
             wait -- one task, one PARKED entry",
        );
        assert_eq!(
            staged.deadline,
            Some(io_deadline),
            "the deadline `staged_io_for_test` reports and the one \
             `staged_deadline_for_test` reports are the same single deadline"
        );
        // The inner stages no deadline of its own, so any deadline here is the
        // timeout's -- and its magnitude is what pins the unit conversion.
        let delta = io_deadline.duration_since(before);
        assert!(
            delta >= Duration::from_secs(4) && delta <= Duration::from_secs(30),
            "a 5s timeout staged a deadline {delta:?} out -- a unit error, \
             not jitter"
        );
        assert_eq!(
            staged.task, None,
            "nothing staged a task wait, so this must be empty"
        );
    }

    /// An **elapsed** timeout must leave nothing staged by the inner it just
    /// abandoned.
    ///
    /// The structural guard for the abandonment bug: `poll_one` takes
    /// `PENDING_PARK` once per *task poll*, not once per future, so a leftover
    /// `Wait::Task` here collides with the next suspension in the same task
    /// poll and aborts the process -- reachable from
    /// `timeout(d, h.join())` followed by any other parking suspension. See
    /// [`poll_timeout`]'s own doc comment.
    ///
    /// Deliberately a *unit* test as well as two fixtures: the fixtures cover
    /// the two shapes anyone happened to think of, this covers the property.
    #[test]
    fn an_elapsed_timeout_discards_the_park_its_abandoned_inner_staged() {
        let inner = make_future(poll_parks_on_a_task_forever, 0);
        let fut = nova_rt_task_timeout_future(0, inner);

        let (status, staged) = poll_once_and_read_staged_for_test(fut, None);

        assert_eq!(status, POLL_READY, "a zero-duration timeout is already due");
        assert_eq!(
            output_of_for_test(fut),
            TIMEOUT_STATUS_ELAPSED,
            "the inner never completes, so this is the elapsed exit"
        );
        assert_eq!(
            staged.task, None,
            "the abandoned inner's task wait must not survive this poll: the \
             next suspension in this same task poll would collide with it and \
             abort the process"
        );
        assert_eq!(staged.deadline, None, "nothing staged a deadline");
        assert!(staged.io.is_none(), "nothing staged an I/O wait");
    }

    /// The same postcondition over an inner that parked on an **`Io`** wait
    /// rather than a `Task` one.
    ///
    /// The review that found this bug measured the `Task` shape end-to-end and
    /// recorded the `Io` shapes as *inferred* -- the leftover mechanism looked
    /// kind-agnostic and `try_stage` was probed directly, but no `Io` case was
    /// run, because end-to-end it needs a socket fixture. This is that
    /// measurement, one layer down where no socket is needed: the abandoned
    /// park is discarded regardless of which field of `Staged` it occupies.
    /// "Probe one shape, generalise the conclusion" is how the bug got in;
    /// asserting the generalisation is cheaper than repeating the mistake.
    #[test]
    fn an_elapsed_timeout_discards_an_io_park_its_abandoned_inner_staged() {
        let inner = make_future(poll_parks_on_an_io_forever, 0);
        let fut = nova_rt_task_timeout_future(0, inner);

        let (status, staged) = poll_once_and_read_staged_for_test(fut, None);

        assert_eq!(status, POLL_READY, "a zero-duration timeout is already due");
        assert_eq!(output_of_for_test(fut), TIMEOUT_STATUS_ELAPSED);
        assert!(
            staged.io.is_none(),
            "the abandoned inner's I/O wait must not survive this poll either \
             -- got {:?}",
            staged.io
        );
        assert_eq!(staged.task, None, "and nothing staged a task wait");
        assert_eq!(staged.deadline, None, "and nothing staged a deadline");
    }

    /// The **completed** exit is held to the same postcondition, over an inner
    /// that stages a park and then returns `POLL_READY` in the same poll.
    ///
    /// Not a hypothetical shape: it is the one `poll_one`'s "taken
    /// unconditionally" comment exists for. Both of `poll_timeout`'s
    /// `POLL_READY` exits hand the slot back as they found it, so both are
    /// tested; covering only the elapsed one would leave half the contract
    /// resting on the belief that no inner ever does this.
    #[test]
    fn a_completed_timeout_discards_a_park_its_inner_staged_before_completing() {
        let inner = make_future(poll_parks_then_completes, 0);
        let fut = nova_rt_task_timeout_future(30 * 1_000_000_000, inner);

        let (status, staged) = poll_once_and_read_staged_for_test(fut, None);

        assert_eq!(status, POLL_READY, "the inner completed");
        assert_eq!(
            output_of_for_test(fut),
            TIMEOUT_STATUS_COMPLETED,
            "the inner beat the 30s deadline"
        );
        assert_eq!(
            staged.task, None,
            "a park staged by a poll that then completed must not survive \
             this poll either"
        );
    }

    /// `poll_timeout` **restores** what it found rather than clearing to
    /// `Staged::default()`.
    ///
    /// The distinction is the whole design of the fix, and it is invisible
    /// unless something was already staged: with an empty slot going in, a
    /// snapshot restore and a clear produce the same answer. Here a deadline
    /// is staged *before* the timeout runs -- an earlier suspension in the
    /// same task poll, or an enclosing timeout -- and it must still be there
    /// afterwards, while the abandoned inner's task wait must not.
    ///
    /// A deadline rather than an `Io` or `Task` pre-stage on purpose: those
    /// would *collide* with the inner's own `Wait::Task` inside
    /// `stage_park`, which aborts the process, and an aborting test takes the
    /// whole test binary down instead of failing (see `try_stage`'s doc
    /// comment). A deadline merges, so this test can still fail cleanly
    /// against every mutant it is aimed at.
    #[test]
    fn an_elapsed_timeout_restores_the_park_staged_before_it_rather_than_clearing() {
        let already_staged = Instant::now() + Duration::from_secs(30);
        let inner = make_future(poll_parks_on_a_task_forever, 0);
        let fut = nova_rt_task_timeout_future(0, inner);

        let (status, staged) =
            poll_once_and_read_staged_for_test(fut, Some(Wait::Deadline(already_staged)));

        assert_eq!(status, POLL_READY, "a zero-duration timeout is already due");
        assert_eq!(output_of_for_test(fut), TIMEOUT_STATUS_ELAPSED);
        assert_eq!(
            staged.task, None,
            "the abandoned inner's task wait must be gone"
        );
        assert_eq!(
            staged.deadline,
            Some(already_staged),
            "a park staged before this timeout ran must survive it -- \
             clearing to Staged::default() would discard a park poll_timeout \
             never staged, and this is the assertion that says so"
        );
    }

    /// The inner future's state survives a sweep whose **entire** root set is
    /// the timeout's own state, which is what proves the timeout's stored fat
    /// pointer -- not some other reachability path -- is what keeps it alive.
    ///
    /// **Deliberately `gc::sweep_with_roots_for_test`, not
    /// `gc::collect_for_test`.** An earlier version of this test used the
    /// real, stack-scanning collector and tried to defeat the conservative
    /// scan's false positives with the `hide`/`reveal` bit-complement
    /// technique `gc.rs`'s own `registry` tests use. That did not work: two
    /// throwaway probes (reproducing `gc.rs`'s own
    /// `an_unregistered_object_is_swept` verbatim, and stripping this test
    /// down to no timeout/combinator code at all) both showed the object
    /// surviving a real collection regardless, purely because it ran inside
    /// `task.rs`'s test binary rather than `gc.rs`'s -- the identical
    /// technique reliably sweeps in `gc.rs`'s own module and reliably does
    /// not here. That is a property of the stack-scanning path and this
    /// binary, not of anything this combinator does, and it made the
    /// stack-scanning version structurally unable to fail against a
    /// constructor that never wrote `TIMEOUT_SLOT_INNER` -- passing 20/20
    /// runs whether or not the bug was present.
    ///
    /// `sweep_with_roots_for_test` sidesteps that path entirely: no stack
    /// scan, and -- confirmed by reading `collect_with_roots`'s own body,
    /// not merely trusting its doc comment -- no consultation of `PINNED`
    /// (the `add_root` registry) either. Its marking loop iterates only the
    /// `roots` slice a caller passes in; `PINNED` is copied into that set by
    /// `collect()` alone, which this path never calls. So the only way
    /// `inner_state` can survive this sweep is through `fut_state`'s own
    /// scanned contents, which is exactly the property under test.
    ///
    /// The assertion just above the sweep checks the other way a root could
    /// leak: if `build_future`'s `add_root`/`remove_root` pairing ever left
    /// either state pinned, `PINNED` would be non-empty for it. Confirmed
    /// rather than assumed, per the same standard applied to `PINNED`
    /// itself above.
    ///
    /// No `#[cfg(windows)]`: `sweep_with_roots_for_test` is `#[cfg(test)]`
    /// only, so -- unlike the version this replaces -- this test compiles
    /// and runs on every platform.
    #[test]
    fn a_timeouts_inner_future_survives_a_root_set_sweep_of_only_its_own_state() {
        let inner = nova_rt_task_sleep_future_nanos(60 * 1_000_000_000);
        let inner_state = state_of(inner);
        let fut = nova_rt_task_timeout_future(60 * 1_000_000_000, inner);
        let fut_state = state_of(fut);

        assert_eq!(
            gc::root_count(inner_state) + gc::root_count(fut_state),
            0,
            "build_future's add_root/remove_root pairing left a root pinned \
             on one of these states; this test would pass regardless of \
             TIMEOUT_SLOT_INNER if PINNED were consulted here (it is not,\
             but this state should never arise regardless)"
        );

        gc::sweep_with_roots_for_test(&[fut_state]);

        assert!(
            gc::object_info(inner_state).is_some(),
            "the inner state must stay live: with only the timeout's own \
             state in the root set, the inner state is reachable through \
             nothing but the fat pointer TIMEOUT_SLOT_INNER stores"
        );
    }

    /// The exact layout `nova_rt_task_join_future` builds, read back from the
    /// collector's own records -- the same discipline as
    /// `the_sleep_futures_layout_is_the_one_the_abi_declares`, and for the
    /// identical reason: that test's own doc comment records a review that
    /// caught a `build_future` call site one word short of what its `init`
    /// closure writes, undetected by the const assert beside `JOIN_STATE_SIZE`
    /// (a relation between two constants, blind to what a call site actually
    /// passes) and by `build_future`'s own `debug_assert!` (a floor, and
    /// `STATE_MIN_SIZE` already satisfies its own floor). Only the collector's
    /// own recorded size distinguishes "16 bytes, and the target id was
    /// written one word past the end" from "24 bytes, exactly as declared."
    #[test]
    fn the_join_futures_layout_is_the_one_the_abi_declares() {
        let target = make_future(poll_ready_now, 0);
        // SAFETY: `target` is a well-formed future from `make_future`.
        unsafe { nova_rt_task_spawn(target) };
        // SAFETY: `target` was just spawned, so `task_id_of` resolves it.
        let fut = unsafe { nova_rt_task_join_future(target) };
        assert_eq!(
            gc::object_info(fut as usize),
            Some((FUTURE_SIZE, true)),
            "the fat pointer must be exactly the two-word future, scanned"
        );
        let state = state_of(fut);
        assert_eq!(
            gc::object_info(state),
            Some((JOIN_STATE_SIZE, true)),
            "the state object must be the ABI minimum plus the one temp slot \
             holding the target task id, scanned"
        );
        // SAFETY: `fut` is this call's own `FUTURE_SIZE`-byte block.
        let poll = unsafe { (fut as *mut usize).add(FUTURE_SLOT_POLL).read() };
        let expected: PollFn = poll_join;
        assert_eq!(
            poll, expected as usize,
            "word 0 must be the poll function's address, not the state's"
        );
    }

    /// The layout a bare [`build_future`] call produces, read back from the
    /// collector's own records -- the same check as
    /// `the_yield_futures_layout_is_the_one_the_abi_declares`, but aimed at
    /// the shared constructor directly rather than at one of its callers.
    /// After `nova_rt_task_yield_future` was routed through it, `build_future`
    /// is where every future's layout in this system actually comes from, so
    /// this is the one test that would catch that function's own layout
    /// drifting, independent of which caller happens to be tested elsewhere.
    #[test]
    fn a_build_future_result_has_the_layout_the_abi_declares() {
        let fut = test_future(poll_ready_now);
        assert_eq!(
            gc::object_info(fut as usize),
            Some((FUTURE_SIZE, true)),
            "the fat pointer must be exactly the two-word future, scanned"
        );
        let state = state_of(fut);
        assert_eq!(
            gc::object_info(state),
            Some((STATE_MIN_SIZE, true)),
            "the state object must be at least the tag and output slots, scanned"
        );
    }

    /// The suspension shape everything else depends on: pending exactly once.
    #[test]
    fn the_yield_future_pends_once_then_completes() {
        let fut = nova_rt_task_yield_future();
        // SAFETY: `fut` is a valid future fat pointer, built above.
        let (poll, state) = unsafe { read_future(fut) };
        // SAFETY: `state` is that future's own state object.
        assert_eq!(
            unsafe { poll(state, std::ptr::null_mut()) },
            POLL_PENDING,
            "the first poll must suspend, or `yield_now` never yields"
        );
        assert_eq!(
            unsafe { poll(state, std::ptr::null_mut()) },
            POLL_READY,
            "the second poll must complete, or `yield_now` never returns"
        );
    }

    /// Each call hands out its own state object, so two suspensions can be
    /// alive at once.
    ///
    /// A shared static state would make the second future start at the first
    /// one's advanced tag and complete without ever suspending -- which is the
    /// failure asserted here, rather than merely that the addresses differ:
    /// distinct addresses alone would also hold for two futures that shared
    /// their tag through some other route.
    #[test]
    fn two_yield_futures_do_not_share_a_resume_tag() {
        let a = nova_rt_task_yield_future();
        let b = nova_rt_task_yield_future();
        assert_ne!(
            state_of(a),
            state_of(b),
            "each call allocates its own state"
        );
        // SAFETY: both are valid future fat pointers, built above.
        let (poll_a, state_a) = unsafe { read_future(a) };
        let (poll_b, state_b) = unsafe { read_future(b) };
        // SAFETY: each `state` is its own future's state object.
        assert_eq!(
            unsafe { poll_a(state_a, std::ptr::null_mut()) },
            POLL_PENDING
        );
        assert_eq!(
            unsafe { poll_b(state_b, std::ptr::null_mut()) },
            POLL_PENDING,
            "polling one yield future must not advance another's tag"
        );
    }

    /// The whole `yield_now` shape end to end at the executor level: a task
    /// that awaits a yield future gets re-queued and finishes on its next turn.
    #[test]
    fn a_task_awaiting_a_yield_future_resumes_on_its_next_turn() {
        // Shaped like the poll function `async fn f() { yield_now().await }`
        // compiles to: the inner future lives in a temp slot, the entry poll
        // creates it, and each poll forwards to it.
        unsafe extern "C-unwind" fn poll_awaits_yield(state: *mut u8, ctx: *mut u8) -> i64 {
            let slots = state as *mut i64;
            // SAFETY: a `make_future(_, 1)` state object, so slot
            // `STATE_SLOT_TEMPS` is in bounds.
            let mut inner = unsafe { slots.add(STATE_SLOT_TEMPS).read() };
            if inner == 0 {
                inner = nova_rt_task_yield_future() as i64;
                // SAFETY: same object, same slot.
                unsafe { slots.add(STATE_SLOT_TEMPS).write(inner) };
            }
            // SAFETY: `inner` is a future this function just built or stored.
            let (poll, inner_state) = unsafe { read_future(inner as *mut u8) };
            // SAFETY: forwarding the poll ABI, `task_ctx` unchanged.
            let status = unsafe { poll(inner_state, ctx) };
            if status == POLL_PENDING {
                return POLL_PENDING;
            }
            // SAFETY: same object, output slot.
            unsafe { slots.add(STATE_SLOT_OUTPUT).write(11) };
            POLL_READY
        }
        let fut = make_future(poll_awaits_yield, 1);
        assert_eq!(
            unsafe { nova_rt_task_block_on(fut) },
            11,
            "the executor must re-poll a task that awaited a yield future"
        );
    }

    /// A poll function that stages a deadline park on its first poll and
    /// completes on its second, used to drive the park set without any Nova
    /// surface. Mirrors `poll_yield_once`'s shape, including its obligation not
    /// to unwind.
    unsafe extern "C-unwind" fn poll_park_once(state: *mut u8, _task_ctx: *mut u8) -> i64 {
        let slots = state as *mut i64;
        // SAFETY: `state` is a `STATE_MIN_SIZE` state object built by the test
        // helper below, so both slots are in bounds.
        let tag = unsafe { slots.add(STATE_SLOT_TAG).read() };
        if tag == 0 {
            unsafe { slots.add(STATE_SLOT_TAG).write(1) };
            stage_park(Wait::Deadline(Instant::now()));
            return POLL_PENDING;
        }
        unsafe { slots.add(STATE_SLOT_OUTPUT).write(0) };
        POLL_READY
    }

    #[test]
    fn a_staged_park_moves_the_task_out_of_the_ready_queue() {
        let fut = test_future(poll_park_once);
        // SAFETY: `fut` is a well-formed future from `test_future`.
        let _id = unsafe { spawn_internal(fut) };
        assert_eq!(QUEUE.with(|q| q.borrow().len()), 1, "spawn should queue it");

        let id = QUEUE.with(|q| q.borrow_mut().pop_front()).expect("queued");
        // SAFETY: `id` was just popped from `QUEUE`.
        unsafe { poll_one(id) };

        assert_eq!(
            QUEUE.with(|q| q.borrow().len()),
            0,
            "a parked task must NOT be re-queued -- that is the whole change"
        );
        assert_eq!(
            PARKED.with(|p| p.borrow().len()),
            1,
            "it must be in the park set instead"
        );
        let _ = id;
    }

    /// A poll function that stages a bare deadline and then an I/O wait, in
    /// two separate `stage_park` calls, on its first poll -- the one
    /// legitimate two-parks-in-one-poll combination this task makes legal --
    /// and completes on its second.
    unsafe extern "C-unwind" fn poll_deadline_then_io(state: *mut u8, _task_ctx: *mut u8) -> i64 {
        let slots = state as *mut i64;
        // SAFETY: `state` is a `STATE_MIN_SIZE` state object built by
        // `test_future`, so both slots are in bounds.
        let tag = unsafe { slots.add(STATE_SLOT_TAG).read() };
        if tag == 0 {
            unsafe { slots.add(STATE_SLOT_TAG).write(1) };
            stage_park(Wait::Deadline(Instant::now() + Duration::from_secs(30)));
            stage_park(Wait::Io {
                socket: RawSocket(9),
                interest: Interest::Read,
                deadline: None,
            });
            return POLL_PENDING;
        }
        unsafe { slots.add(STATE_SLOT_OUTPUT).write(0) };
        POLL_READY
    }

    /// Two separate `stage_park` calls in the same poll -- one bare
    /// `Wait::Deadline`, one untimed `Wait::Io` -- must not abort, and must
    /// fold into exactly one `PARKED` entry carrying both: `Wait::Io` with
    /// its own `deadline` field now populated. Exactly one entry, not two,
    /// is the property `Wait::Io`'s own doc comment states: one task has one
    /// `PARKED` entry, or every wake path has to remove two.
    #[test]
    fn a_deadline_and_an_io_wait_stage_together() {
        let fut = test_future(poll_deadline_then_io);
        // SAFETY: `fut` is a well-formed future from `test_future`.
        let id = unsafe { spawn_internal(fut) };
        let popped = QUEUE.with(|q| q.borrow_mut().pop_front());
        assert_eq!(popped, Some(id), "spawn should have queued it");
        // SAFETY: `id` was just popped from `QUEUE`.
        unsafe { poll_one(id) };

        let entries = PARKED.with(|p| p.borrow().clone());
        assert_eq!(
            entries.len(),
            1,
            "a deadline staged alongside an I/O wait must fold into ONE \
             PARKED entry, not two -- got: {entries:?}"
        );
        match entries[0].1 {
            Wait::Io {
                socket,
                interest,
                deadline,
            } => {
                assert_eq!(socket, RawSocket(9));
                assert_eq!(interest, Interest::Read);
                assert!(
                    deadline.is_some(),
                    "the separately-staged deadline must ride inside this \
                     Io wait"
                );
            }
            other => panic!("expected a single Wait::Io entry, got {other:?}"),
        }
        PARKED.with(|p| p.borrow_mut().clear());
    }

    /// The same-kind collision this task keeps: two `Wait::Io` stages in the
    /// same poll must still abort -- widening what one poll may stage (a
    /// deadline now composes with a task wait or an I/O wait) must not widen
    /// which same-kind collisions it tolerates. See `try_stage`'s own doc
    /// comment for why this checks `try_stage` directly rather than
    /// `stage_park`'s actual abort.
    ///
    /// `try_stage` has **four** `Err` branches -- the `Wait::Io` arm rejecting
    /// `next.io` and `next.task`, and the `Wait::Task` arm rejecting
    /// `next.task` and `next.io` -- and for a while this was the only test of
    /// any of them, while the widening added a `deadline:` field to all four
    /// reconstructed waits that nothing observed. This one covers the
    /// `Wait::Io` arm's `next.io` branch; the two tests below cover the other
    /// three. All four now check the `deadline:` field as well, because
    /// `collision_msg` receives a `Wait` this function *reconstructs* from
    /// `Staged` rather than the one that was originally staged, so a mutant
    /// hard-coding `deadline: None` there would lose the merged deadline from
    /// every collision diagnostic with no test noticing.
    #[test]
    fn two_io_waits_in_one_poll_still_abort() {
        let first = Wait::Io {
            socket: RawSocket(1),
            interest: Interest::Read,
            deadline: None,
        };
        let second = Wait::Io {
            socket: RawSocket(2),
            interest: Interest::Write,
            deadline: None,
        };
        let staged =
            try_stage(Staged::default(), first).expect("the first I/O wait stages cleanly");
        let err =
            try_stage(staged, second).expect_err("a second I/O wait in the same poll must collide");
        assert!(err.contains("two parks staged in one poll"), "got: {err}");
        assert!(
            err.contains("deadline: None"),
            "with no deadline staged, the reconstructed I/O wait must say so: {err}"
        );

        // The same collision with a deadline already merged in, so the
        // `deadline:` field the reconstruction fills from `next.deadline` is
        // actually observed.
        let staged = try_stage(
            Staged::default(),
            Wait::Deadline(Instant::now() + Duration::from_secs(5)),
        )
        .expect("a bare deadline stages");
        let staged = try_stage(staged, first).expect("an I/O wait joins a staged deadline");
        let err = try_stage(staged, second)
            .expect_err("a second I/O wait must still collide, deadline or not");
        assert!(
            err.contains("deadline: Some("),
            "the merged deadline must appear in the wait the diagnostic \
             reconstructs: {err}"
        );
    }

    /// Task+Task in one poll must still abort -- the collision spec §6.1 asked
    /// for by name ("Task+Task and Io+Io **still abort**, so the widening did
    /// not widen too far") and that no test covered.
    ///
    /// It is not merely hygiene. `poll_timeout`'s abandonment path makes these
    /// branches production-reachable: before it discarded the park its
    /// abandoned inner staged, this exact pairing is what aborted the process
    /// from `timeout(d, h1.join())` followed by `h2.join().await`. The
    /// behaviour under test is the one that *should* abort -- a genuine
    /// two-parks-in-one-poll bug -- so it has to keep aborting while the
    /// abandonment case stops reaching it.
    ///
    /// Both ids are named in the assertions, and in order: the diagnostic's
    /// value is that it says *which* two waits clashed, so a mutant reporting
    /// the incoming wait twice, or reversing the pair, would still contain
    /// "two parks staged in one poll".
    #[test]
    fn two_task_waits_in_one_poll_still_abort() {
        let first = Wait::Task {
            id: 1,
            deadline: None,
        };
        let second = Wait::Task {
            id: 2,
            deadline: None,
        };
        let staged =
            try_stage(Staged::default(), first).expect("the first task wait stages cleanly");
        let err = try_stage(staged, second)
            .expect_err("a second task wait in the same poll must collide");
        assert!(err.contains("two parks staged in one poll"), "got: {err}");
        assert!(
            err.contains("Task { id: 1, deadline: None } then Task { id: 2"),
            "the diagnostic must name the already-staged wait first and the \
             incoming one second: {err}"
        );

        // Again with a deadline already merged in -- the `Wait::Task` arm's own
        // reconstruction of the previous task wait, whose `deadline:` field is
        // otherwise unobserved by any test.
        let at = Instant::now() + Duration::from_secs(5);
        let staged =
            try_stage(Staged::default(), Wait::Deadline(at)).expect("a bare deadline stages");
        let staged = try_stage(staged, first).expect("a task wait joins a staged deadline");
        let err = try_stage(staged, second)
            .expect_err("a second task wait must still collide, deadline or not");
        assert!(
            err.contains("deadline: Some("),
            "the merged deadline must appear in the wait the diagnostic \
             reconstructs: {err}"
        );
    }

    /// A `Wait::Task` and a `Wait::Io` in one poll must still abort, **in
    /// either order** -- the last two of `try_stage`'s four `Err` branches
    /// (`Wait::Io`'s `next.task` rejection and `Wait::Task`'s `next.io` one).
    ///
    /// Both orders, because they are two *different* branches in two different
    /// match arms, not one property observed twice: deleting either rejection
    /// leaves the other passing. Nothing about waiting on a sibling task
    /// composes with waiting on a socket the way a deadline composes with
    /// both, which is the distinction `Staged`'s doc comment draws and this
    /// pins.
    #[test]
    fn a_task_wait_and_an_io_wait_in_one_poll_still_abort_in_either_order() {
        let task = Wait::Task {
            id: 3,
            deadline: None,
        };
        let io = Wait::Io {
            socket: RawSocket(4),
            interest: Interest::Write,
            deadline: None,
        };
        let at = Instant::now() + Duration::from_secs(5);

        // Task first, then Io -- `try_stage`'s `Wait::Io` arm, `next.task`
        // branch, which reconstructs the previous wait as a `Wait::Task`.
        let staged = try_stage(Staged::default(), task).expect("the task wait stages cleanly");
        let err = try_stage(staged, io).expect_err("an I/O wait must not join a staged task wait");
        assert!(err.contains("two parks staged in one poll"), "got: {err}");
        assert!(
            err.contains("Task { id: 3, deadline: None } then Io {"),
            "the diagnostic must reconstruct the staged task wait, then name \
             the incoming I/O wait: {err}"
        );
        let staged =
            try_stage(Staged::default(), Wait::Deadline(at)).expect("a bare deadline stages");
        let staged = try_stage(staged, task).expect("a task wait joins a staged deadline");
        let err = try_stage(staged, io)
            .expect_err("an I/O wait must still collide with a timed task wait");
        assert!(
            err.contains("deadline: Some("),
            "the merged deadline must ride in the reconstructed task wait: {err}"
        );

        // Io first, then Task -- `try_stage`'s `Wait::Task` arm, `next.io`
        // branch, which reconstructs the previous wait as a `Wait::Io`.
        let staged = try_stage(Staged::default(), io).expect("the I/O wait stages cleanly");
        let err = try_stage(staged, task).expect_err("a task wait must not join a staged I/O wait");
        assert!(err.contains("two parks staged in one poll"), "got: {err}");
        assert!(
            err.contains("Io { socket: RawSocket(4)"),
            "the diagnostic must reconstruct the staged I/O wait with its own \
             socket, not the incoming task wait: {err}"
        );
        assert!(
            err.contains("then Task { id: 3"),
            "and name the incoming task wait second: {err}"
        );
        let staged =
            try_stage(Staged::default(), Wait::Deadline(at)).expect("a bare deadline stages");
        let staged = try_stage(staged, io).expect("an I/O wait joins a staged deadline");
        let err = try_stage(staged, task)
            .expect_err("a task wait must still collide with a timed I/O wait");
        assert!(
            err.contains("deadline: Some("),
            "the merged deadline must ride in the reconstructed I/O wait: {err}"
        );
    }

    /// Two deadlines in one poll merge to the earlier, **in either order**.
    ///
    /// Both orders on purpose: order-independence is the property, and a
    /// single-order test passes against a `min` written backwards.
    #[test]
    fn two_deadlines_in_one_poll_merge_to_the_earlier() {
        let base = Instant::now();
        let soon = base + Duration::from_secs(1);
        let later = base + Duration::from_secs(30);

        let staged =
            try_stage(Staged::default(), Wait::Deadline(later)).expect("first deadline stages");
        let staged = try_stage(staged, Wait::Deadline(soon))
            .expect("a second deadline must merge, not collide");
        assert_eq!(
            staged.deadline,
            Some(soon),
            "later-then-sooner must keep the earlier"
        );

        let staged =
            try_stage(Staged::default(), Wait::Deadline(soon)).expect("first deadline stages");
        let staged = try_stage(staged, Wait::Deadline(later))
            .expect("a second deadline must merge, not collide");
        assert_eq!(
            staged.deadline,
            Some(soon),
            "sooner-then-later must keep the earlier"
        );
    }

    /// A task wait and a deadline co-stage, in either order -- this is the
    /// pair `timeout(d, handle.join())` needs and that aborted the process
    /// before this increment.
    #[test]
    fn a_task_wait_and_a_deadline_co_stage_in_either_order() {
        let at = Instant::now() + Duration::from_secs(5);

        let staged = try_stage(
            Staged::default(),
            Wait::Task {
                id: 7,
                deadline: None,
            },
        )
        .expect("task stages");
        let staged = try_stage(staged, Wait::Deadline(at))
            .expect("a deadline must join a task wait, not collide");
        assert_eq!((staged.task, staged.deadline), (Some(7), Some(at)));

        let staged = try_stage(Staged::default(), Wait::Deadline(at)).expect("deadline stages");
        let staged = try_stage(
            staged,
            Wait::Task {
                id: 7,
                deadline: None,
            },
        )
        .expect("a task wait must join a deadline, not collide");
        assert_eq!((staged.task, staged.deadline), (Some(7), Some(at)));
    }

    /// A **timed** `Wait::Io` and a bare `Wait::Deadline` merge to the
    /// earlier, regardless of arrival order **or** which side carries which
    /// magnitude.
    ///
    /// `two_deadlines_in_one_poll_merge_to_the_earlier` only ever stages a
    /// bare `Wait::Deadline` on both sides, so it never reaches `try_stage`'s
    /// `Wait::Io` arm's own inner `deadline` merge -- that site only fires
    /// when `next.deadline` is already `Some` from an earlier stage *and*
    /// the incoming `Wait::Io` itself carries `deadline: Some(_)`.
    ///
    /// That site is `match next.deadline { Some(prev) => prev.min(at), None
    /// => at }`, and a two-argument `min` breaks in two independent ways --
    /// `Some(prev) => at` (discard the already-staged deadline, keep only
    /// the incoming one) and `Some(prev) => prev` (the opposite: freeze on
    /// the already-staged one, discard the incoming one) -- each invisible
    /// whenever the surviving argument already happens to be the smaller
    /// one. Catching both takes both magnitude assignments, the same
    /// principle `two_deadlines_in_one_poll_merge_to_the_earlier` already
    /// establishes for the standalone arm ("a single-order test passes
    /// against a `min` written backwards"):
    ///
    /// 1. bare `Deadline(soon)` staged first, timed `Io` carrying `later`
    ///    second -- `prev` (`soon`) is already the smaller value, so a
    ///    `Some(prev) => prev` mutant would accidentally produce the right
    ///    answer here; only a `Some(prev) => at` mutant is caught.
    /// 2. bare `Deadline(later)` staged first, timed `Io` carrying `soon`
    ///    second -- the reverse: `at` (`soon`) is now the smaller value, so
    ///    a `Some(prev) => at` mutant would accidentally produce the right
    ///    answer here; only a `Some(prev) => prev` mutant is caught.
    ///
    /// Both are required and neither is redundant with the other -- do not
    /// delete one because the other "already covers the merge".
    ///
    /// A third case (timed `Io` staged first, bare deadline second) instead
    /// goes through the standalone `Wait::Deadline` arm, which is covered
    /// elsewhere -- it stays here anyway, because what it demonstrates is
    /// that the *end state* does not depend on arrival order, not which
    /// branch happened to run. Do not delete it as "redundant" either.
    #[test]
    fn a_timed_io_wait_and_a_bare_deadline_merge_to_the_earlier_in_either_order() {
        let base = Instant::now();
        let soon = base + Duration::from_secs(1);
        let later = base + Duration::from_secs(30);

        // Case 1: bare deadline (the earlier value) first, timed Io carrying
        // the later value second -- catches a `Some(prev) => at` mutant.
        let staged =
            try_stage(Staged::default(), Wait::Deadline(soon)).expect("bare deadline stages");
        let staged = try_stage(
            staged,
            Wait::Io {
                socket: RawSocket(1),
                interest: Interest::Read,
                deadline: Some(later),
            },
        )
        .expect("a timed Io wait must merge with a staged deadline, not collide");
        assert_eq!(
            staged.deadline,
            Some(soon),
            "deadline(soon)-then-timed-io(later) must keep the earlier"
        );

        // Case 2: bare deadline (the later value) first, timed Io carrying
        // the earlier value second -- catches a `Some(prev) => prev` mutant.
        let staged =
            try_stage(Staged::default(), Wait::Deadline(later)).expect("bare deadline stages");
        let staged = try_stage(
            staged,
            Wait::Io {
                socket: RawSocket(1),
                interest: Interest::Read,
                deadline: Some(soon),
            },
        )
        .expect("a timed Io wait must merge with a staged deadline, not collide");
        assert_eq!(
            staged.deadline,
            Some(soon),
            "deadline(later)-then-timed-io(soon) must keep the earlier"
        );

        // Case 3: timed Io (carrying the earlier value) first, bare deadline
        // (the later value) second -- routes through the standalone
        // `Wait::Deadline` arm instead, demonstrating order-independence.
        let staged = try_stage(
            Staged::default(),
            Wait::Io {
                socket: RawSocket(1),
                interest: Interest::Read,
                deadline: Some(soon),
            },
        )
        .expect("timed Io stages");
        let staged = try_stage(staged, Wait::Deadline(later))
            .expect("a bare deadline must merge with a staged timed Io, not collide");
        assert_eq!(
            staged.deadline,
            Some(soon),
            "timed-io-then-deadline must keep the earlier"
        );
    }

    /// A **timed** `Wait::Task` and a bare `Wait::Deadline` merge to the
    /// earlier, regardless of arrival order **or** which side carries which
    /// magnitude -- the `Wait::Task` counterpart of
    /// `a_timed_io_wait_and_a_bare_deadline_merge_to_the_earlier_in_either_order`,
    /// for the same reason: `a_task_wait_and_a_deadline_co_stage_in_either_order`
    /// only ever stages `Wait::Task { deadline: None, .. }`, so it never
    /// reaches the `Wait::Task` arm's own inner `deadline` merge either, and
    /// that site breaks in the same two independent ways a two-argument
    /// `min` always can: `Some(prev) => at` (discard the already-staged
    /// deadline) and `Some(prev) => prev` (discard the incoming one), each
    /// invisible whenever the surviving argument already happens to be the
    /// smaller one.
    ///
    /// 1. bare `Deadline(soon)` staged first, timed task carrying `later`
    ///    second -- catches a `Some(prev) => at` mutant.
    /// 2. bare `Deadline(later)` staged first, timed task carrying `soon`
    ///    second -- catches a `Some(prev) => prev` mutant.
    ///
    /// Both are required and neither is redundant with the other. The third
    /// case (timed task first, bare deadline second) goes back through the
    /// standalone `Wait::Deadline` arm and stays here anyway, for the same
    /// order-independence reason given in the `Io` version -- do not delete
    /// any of the three as "redundant".
    #[test]
    fn a_timed_task_wait_and_a_bare_deadline_merge_to_the_earlier_in_either_order() {
        let base = Instant::now();
        let soon = base + Duration::from_secs(1);
        let later = base + Duration::from_secs(30);

        // Case 1: bare deadline (the earlier value) first, timed task
        // carrying the later value second -- catches a `Some(prev) => at`
        // mutant.
        let staged =
            try_stage(Staged::default(), Wait::Deadline(soon)).expect("bare deadline stages");
        let staged = try_stage(
            staged,
            Wait::Task {
                id: 7,
                deadline: Some(later),
            },
        )
        .expect("a timed task wait must merge with a staged deadline, not collide");
        assert_eq!(
            staged.deadline,
            Some(soon),
            "deadline(soon)-then-timed-task(later) must keep the earlier"
        );

        // Case 2: bare deadline (the later value) first, timed task carrying
        // the earlier value second -- catches a `Some(prev) => prev`
        // mutant.
        let staged =
            try_stage(Staged::default(), Wait::Deadline(later)).expect("bare deadline stages");
        let staged = try_stage(
            staged,
            Wait::Task {
                id: 7,
                deadline: Some(soon),
            },
        )
        .expect("a timed task wait must merge with a staged deadline, not collide");
        assert_eq!(
            staged.deadline,
            Some(soon),
            "deadline(later)-then-timed-task(soon) must keep the earlier"
        );

        // Case 3: timed task (carrying the earlier value) first, bare
        // deadline (the later value) second -- routes through the
        // standalone `Wait::Deadline` arm instead, demonstrating
        // order-independence.
        let staged = try_stage(
            Staged::default(),
            Wait::Task {
                id: 7,
                deadline: Some(soon),
            },
        )
        .expect("timed task stages");
        let staged = try_stage(staged, Wait::Deadline(later))
            .expect("a bare deadline must merge with a staged timed task, not collide");
        assert_eq!(
            staged.deadline,
            Some(soon),
            "timed-task-then-deadline must keep the earlier"
        );
    }

    #[test]
    fn a_deadline_park_is_woken_and_the_task_completes() {
        let fut = test_future(poll_park_once);
        // SAFETY: well-formed future from `test_future`.
        let out = unsafe { run_to_completion(fut) };
        assert_eq!(
            out, 0,
            "the parked task must be woken and run to completion"
        );
        assert!(
            PARKED.with(|p| p.borrow().is_empty()),
            "the park set must be empty once the loop exits"
        );
    }

    /// Like `poll_park_once`, but the deadline is `STATE_SLOT_TEMPS`
    /// milliseconds in the future rather than already due, and the
    /// completing poll records -- in its own output slot -- whether the
    /// task named by the *second* temp slot was already done at that
    /// moment.
    ///
    /// Final whole-branch review of `park-set`, finding I3: every existing
    /// deadline test (`poll_park_once`,
    /// `poll_park_then_read_sibling_progress`) stages
    /// `Wait::Deadline(Instant::now())` -- already due the instant it is
    /// staged -- so `run_to_completion`'s per-poll `wake_due` check (run
    /// after every `poll_one`, before the ready queue can ever be observed
    /// as drained) always wakes it first. `run_to_completion`'s *other*
    /// wake path -- the drained-queue branch's
    /// `match earliest_deadline() { Some(at) => wake_due_deadlines(at), ... }`,
    /// the one place a real `thread::sleep` runs -- was reachable from no
    /// Rust-level test at all. MEASURED: replacing that arm with
    /// `Some(_at) => report_deadlock()` left all 59 pre-existing
    /// `nova-runtime` tests green; only the end-to-end `nova run` gate
    /// (`gate_task_sleep_order_runs`) caught it.
    ///
    /// **Corrected 2026-08-15 (branch `io-poller-std-net`, Task 1): that
    /// quoted match and `wake_due_deadlines` are both gone, and no
    /// `thread::sleep` runs in `task.rs` anywhere anymore.** The drained-queue
    /// branch now matches `(earliest_deadline(), io_parks().is_empty())`, and
    /// the real sleep this finding is about now runs in `poll::wait`'s
    /// empty-socket-set branch (`poll.rs`), reached from the `(Some(at),
    /// true)` arm of that match. That arm -- unlike this finding's original
    /// claim about its predecessor -- **is** covered, by the test just below:
    /// `two_not_yet_due_deadlines_drain_the_queue_then_wake_in_deadline_order`
    /// stages two not-yet-due deadlines specifically so the ready queue
    /// drains with both still parked, forcing exactly this branch to consult
    /// `earliest_deadline()` and actually sleep (see that test's own doc
    /// comment, which predates this correction and already says so).
    unsafe extern "C-unwind" fn poll_park_after_delay(state: *mut u8, _task_ctx: *mut u8) -> i64 {
        let slots = state as *mut i64;
        // SAFETY: `state` is a `make_future(_, 2)` object: TAG, OUTPUT, a
        // delay-in-milliseconds temp, and a sibling-task-id temp.
        let tag = unsafe { slots.add(STATE_SLOT_TAG).read() };
        if tag == 0 {
            unsafe { slots.add(STATE_SLOT_TAG).write(1) };
            // SAFETY: same object, the delay temp.
            let delay_ms = unsafe { slots.add(STATE_SLOT_TEMPS).read() };
            stage_park(Wait::Deadline(
                Instant::now() + Duration::from_millis(delay_ms as u64),
            ));
            return POLL_PENDING;
        }
        // SAFETY: same object, the sibling-id temp, written by the test
        // before either task in the pair is ever polled.
        let sibling_id = unsafe { slots.add(STATE_SLOT_TEMPS + 1).read() };
        let sibling_was_already_done = is_done_internal(sibling_id) as i64;
        // SAFETY: same object, output slot.
        unsafe { slots.add(STATE_SLOT_OUTPUT).write(sibling_was_already_done) };
        POLL_READY
    }

    /// I3 (final whole-branch review of `park-set`): the one Rust-level gap
    /// the review found. Two tasks park on deadlines `SHORT_DELAY_MS` and
    /// `LONG_DELAY_MS` in the future -- neither due when staged -- so the
    /// ready queue genuinely drains with both still parked, forcing
    /// `run_to_completion`'s drained-queue branch to consult
    /// `earliest_deadline()` and actually sleep, unlike every other
    /// deadline test in this module (see `poll_park_after_delay`'s doc
    /// comment). Spawned in *reverse* of deadline order -- the long one
    /// first -- so an implementation that woke parked tasks in spawn order
    /// rather than deadline order would also be caught here.
    ///
    /// Asserts completion and ordering only, never elapsed duration, per
    /// this project's rule against timing assertions
    /// (`docs/adr/0010-conservative-scan-root-test-gating.md` exists
    /// because eight tests already flake on real timing). Ordering is read
    /// off executor state -- which task's `is_done_internal` the other
    /// observes as true at the moment it completes -- never off the clock:
    /// nothing here reads elapsed time or makes an assertion about it. The
    /// 285ms gap between the two delays is not a timing assertion either;
    /// it is margin against ordinary host scheduling delay between staging
    /// the two deadlines and the wake that follows. Unlike this module's
    /// already-due-deadline tests, where monotonicity alone guarantees the
    /// order with no margin needed, this test's ordering claim depends on
    /// that margin holding -- an arbitrarily slow host could in principle
    /// exceed it. If that ever happens the two assertions on `.output`
    /// below fail loudly, which is the safe direction for this kind of
    /// dependency to fail in.
    #[test]
    fn two_not_yet_due_deadlines_drain_the_queue_then_wake_in_deadline_order() {
        const SHORT_DELAY_MS: i64 = 15;
        const LONG_DELAY_MS: i64 = 300;

        let long = make_future(poll_park_after_delay, 2);
        let short = make_future(poll_park_after_delay, 2);
        // SAFETY: both are `make_future(_, 2)` objects, so slot
        // `STATE_SLOT_TEMPS` (the delay) is in bounds.
        unsafe {
            (state_of(long) as *mut i64)
                .add(STATE_SLOT_TEMPS)
                .write(LONG_DELAY_MS);
            (state_of(short) as *mut i64)
                .add(STATE_SLOT_TEMPS)
                .write(SHORT_DELAY_MS);
        }

        // SAFETY: both are well-formed futures from `make_future`.
        let long_id = unsafe { spawn_internal(long) };
        let short_id = unsafe { spawn_internal(short) };

        // Neither id exists until both are spawned, and neither future is
        // polled before `run_to_completion` starts below -- `spawn_internal`
        // only enqueues -- so stashing each one's sibling id now is still
        // strictly before either poll ever reads it.
        // SAFETY: both are `make_future(_, 2)` objects, so slot
        // `STATE_SLOT_TEMPS + 1` (the sibling id) is in bounds.
        unsafe {
            (state_of(long) as *mut i64)
                .add(STATE_SLOT_TEMPS + 1)
                .write(short_id);
            (state_of(short) as *mut i64)
                .add(STATE_SLOT_TEMPS + 1)
                .write(long_id);
        }

        let root = test_future(poll_ready_now);
        // SAFETY: well-formed future from `test_future`.
        assert_eq!(
            unsafe { run_to_completion(root) },
            7,
            "the root task must still complete normally"
        );

        assert!(
            PARKED.with(|p| p.borrow().is_empty()),
            "both deadline-parked tasks must be woken, not left parked -- an \
             empty park set here is exactly what distinguishes this from the \
             deadlock this same code path would otherwise report"
        );
        assert!(
            is_done_internal(long_id) && is_done_internal(short_id),
            "both tasks must run to completion"
        );
        assert_eq!(
            TASKS.with(|t| t.borrow()[short_id as usize].output),
            0,
            "the short deadline must wake and complete before the long one, \
             so it must not see the long task already done"
        );
        assert_eq!(
            TASKS.with(|t| t.borrow()[long_id as usize].output),
            1,
            "the long deadline must wake after the short one -- deadline \
             order, not spawn order, since long was spawned first -- so it \
             must see the short task already done"
        );
    }

    /// A poll function that stages a *timed* task wait -- not yet due -- on
    /// its first poll and completes on its second. The target id (`999`)
    /// never names a real task, and is not meant to: this exists to drive a
    /// park set of nothing but one timed `Wait::Task` through the real
    /// drained-queue branch, so only the deadline can wake it, the same way
    /// `poll_park_after_delay` does for a bare `Wait::Deadline`.
    unsafe extern "C-unwind" fn poll_park_on_timed_task_wait(
        state: *mut u8,
        _task_ctx: *mut u8,
    ) -> i64 {
        let slots = state as *mut i64;
        // SAFETY: `state` is a `STATE_MIN_SIZE` object from `test_future`.
        let tag = unsafe { slots.add(STATE_SLOT_TAG).read() };
        if tag == 0 {
            unsafe { slots.add(STATE_SLOT_TAG).write(1) };
            stage_park(Wait::Task {
                id: 999,
                deadline: Some(Instant::now() + Duration::from_millis(15)),
            });
            return POLL_PENDING;
        }
        // SAFETY: same object, output slot.
        unsafe { slots.add(STATE_SLOT_OUTPUT).write(0) };
        POLL_READY
    }

    /// Corrects `run_to_completion`'s own doc comment: a park set holding
    /// nothing but a *timed* `Wait::Task`, and no I/O wait, is legitimate
    /// waiting, not a deadlock -- only a *bare* one is. Before this test, no
    /// test drove that exact park set through `run_to_completion` itself;
    /// `earliest_deadline_counts_a_timed_task_wait_and_ignores_a_bare_one`
    /// checks `earliest_deadline` in isolation, and
    /// `wake_due_wakes_a_task_wait_whose_deadline_elapsed` checks `wake_due`
    /// in isolation, but neither exercises the
    /// `(earliest_deadline(), io.is_empty())` match in the drained-queue
    /// branch, which is the one place `report_deadlock` -- and its
    /// `std::process::abort()` -- could fire on this exact park set. If
    /// `earliest_deadline` ever again stopped reporting a timed task wait's
    /// deadline, this test would not fail cleanly: it would abort the whole
    /// test binary, which is the loud failure this class of regression
    /// deserves.
    ///
    /// Uses a not-yet-due deadline, not an already-due one, for the same
    /// reason `two_not_yet_due_deadlines_drain_the_queue_then_wake_in_deadline_order`
    /// does: an already-due deadline would be caught by the per-poll
    /// `wake_due` check before the queue ever drains, never reaching the
    /// drained-queue match this test exists to exercise.
    #[test]
    fn a_lone_timed_task_wait_sleeps_instead_of_deadlocking() {
        let fut = test_future(poll_park_on_timed_task_wait);
        // SAFETY: well-formed future from `test_future`.
        let out = unsafe { run_to_completion(fut) };
        assert_eq!(
            out, 0,
            "a lone timed task wait must be woken by its own deadline and \
             complete, not deadlock"
        );
        assert!(
            PARKED.with(|p| p.borrow().is_empty()),
            "the park set must be empty once the loop exits"
        );
    }

    #[test]
    fn the_earliest_deadline_is_the_one_slept_on() {
        let far = Instant::now() + Duration::from_secs(30);
        let near = Instant::now();
        PARKED.with(|p| {
            let mut p = p.borrow_mut();
            p.push((7, Wait::Deadline(far)));
            p.push((8, Wait::Deadline(near)));
        });
        assert_eq!(
            earliest_deadline(),
            Some(near),
            "sleeping on the later deadline would strand the earlier task"
        );
        PARKED.with(|p| p.borrow_mut().clear());
    }

    /// `wake_due` must wake a task wait whose deadline elapsed.
    ///
    /// **This is the only test that fails if `wake_due`'s timed-task arm is
    /// missing.** Its `retain` ends in `_ => true`, so an unhandled timed
    /// `Wait::Task` is not a compile error, not a panic and not a diagnostic
    /// -- the task simply stays parked for the rest of the process.
    ///
    /// Clears `PARKED`/`QUEUE` at both ends, not just the end: the test this
    /// task retires (`earliest_deadline_and_wake_due_ignore_task_waits`) only
    /// cleared at the end of each phase, on the unstated assumption that
    /// whatever ran before it on the same libtest worker thread had already
    /// left both empty. `assert!(PARKED...is_empty())` below is exactly the
    /// kind of check that assumption can silently falsify, so this test does
    /// not carry it forward.
    #[test]
    fn wake_due_wakes_a_task_wait_whose_deadline_elapsed() {
        PARKED.with(|p| p.borrow_mut().clear());
        QUEUE.with(|q| q.borrow_mut().clear());
        let past = Instant::now();
        PARKED.with(|p| {
            p.borrow_mut().push((
                41,
                Wait::Task {
                    id: 999,
                    deadline: Some(past),
                },
            ));
        });
        wake_due(past + Duration::from_millis(1));
        let queued = QUEUE.with(|q| q.borrow().contains(&41));
        assert!(
            queued,
            "a timed task wait whose deadline passed must be re-queued"
        );
        assert!(
            PARKED.with(|p| p.borrow().is_empty()),
            "and it must be removed from PARKED, not woken twice"
        );
        PARKED.with(|p| p.borrow_mut().clear());
        QUEUE.with(|q| q.borrow_mut().clear());
    }

    /// A **timed** task wait contributes its deadline; a **bare** one still
    /// contributes nothing. This is the split of the old
    /// `earliest_deadline_and_wake_due_ignore_task_waits`, whose name asserted
    /// the half that this increment reverses.
    ///
    /// Cleared at the start as well as the end, for the same
    /// worker-thread-reuse reason given in
    /// `wake_due_wakes_a_task_wait_whose_deadline_elapsed`'s doc comment.
    #[test]
    fn earliest_deadline_counts_a_timed_task_wait_and_ignores_a_bare_one() {
        PARKED.with(|p| p.borrow_mut().clear());
        let base = Instant::now();
        let soon = base + Duration::from_secs(1);
        let later = base + Duration::from_secs(30);
        PARKED.with(|p| {
            let mut p = p.borrow_mut();
            p.push((
                1,
                Wait::Task {
                    id: 999,
                    deadline: None,
                },
            ));
            p.push((
                2,
                Wait::Task {
                    id: 998,
                    deadline: Some(later),
                },
            ));
            p.push((3, Wait::Deadline(soon)));
        });
        assert_eq!(earliest_deadline(), Some(soon));

        PARKED.with(|p| p.borrow_mut().clear());
        PARKED.with(|p| {
            let mut p = p.borrow_mut();
            p.push((
                1,
                Wait::Task {
                    id: 999,
                    deadline: None,
                },
            ));
            p.push((
                2,
                Wait::Task {
                    id: 998,
                    deadline: Some(later),
                },
            ));
        });
        assert_eq!(
            earliest_deadline(),
            Some(later),
            "a timed task wait is the only deadline here and must be found"
        );
        PARKED.with(|p| p.borrow_mut().clear());
    }

    /// `wake_due`'s `Wait::Io { deadline: Some(_), .. }` arm, behaviourally.
    ///
    /// Reverting that arm still compiles (the trailing `_ => true` swallows
    /// it silently), which is exactly what makes it *not* one of the two
    /// compiler-forced sites -- unlike `earliest_deadline`/`deadlock_report`,
    /// nothing here stops a backwards `<=`/`<` or a dropped `woken.push` from
    /// shipping green. Three entries, one of each shape a timed `Wait::Io`
    /// can be in relative to `now`, so a mutant that wakes too many or too
    /// few is caught either way: a due I/O wait (must wake), a not-yet-due
    /// one (must not), and an untimed one (must never be treated as due at
    /// all -- the same property a bare `Wait::Task` has, and
    /// `wake_due_wakes_a_task_wait_whose_deadline_elapsed` is the analogous
    /// check on `wake_due`'s `Wait::Task` arm).
    #[test]
    fn wake_due_wakes_a_due_io_wait_and_leaves_the_rest_parked() {
        let due = Instant::now();
        let not_due = due + Duration::from_secs(30);
        PARKED.with(|p| {
            let mut p = p.borrow_mut();
            p.push((
                1,
                Wait::Io {
                    socket: RawSocket(1),
                    interest: Interest::Read,
                    deadline: Some(due),
                },
            ));
            p.push((
                2,
                Wait::Io {
                    socket: RawSocket(2),
                    interest: Interest::Write,
                    deadline: Some(not_due),
                },
            ));
            p.push((
                3,
                Wait::Io {
                    socket: RawSocket(3),
                    interest: Interest::Read,
                    deadline: None,
                },
            ));
        });
        wake_due(Instant::now());
        assert_eq!(
            QUEUE.with(|q| q.borrow().clone()),
            std::collections::VecDeque::from([1]),
            "only the due I/O wait's task must be woken"
        );
        assert_eq!(
            PARKED.with(|p| p.borrow().clone()),
            vec![
                (
                    2,
                    Wait::Io {
                        socket: RawSocket(2),
                        interest: Interest::Write,
                        deadline: Some(not_due),
                    }
                ),
                (
                    3,
                    Wait::Io {
                        socket: RawSocket(3),
                        interest: Interest::Read,
                        deadline: None,
                    }
                ),
            ],
            "the not-yet-due and the untimed I/O waits must stay parked"
        );
        PARKED.with(|p| p.borrow_mut().clear());
        QUEUE.with(|q| q.borrow_mut().clear());
    }

    /// `wake_ready`'s `ready.contains(&socket)` guard, behaviourally -- the
    /// exact counterpart of
    /// `wake_due_wakes_a_due_io_wait_and_leaves_the_rest_parked`, for the
    /// other of this module's two I/O wake paths, and needed for the same
    /// reason it was: the `retain`'s trailing `_ => true` swallows any
    /// narrowing mistake silently. Widening that arm to a bare
    /// `Wait::Io { .. }` -- waking *every* I/O-parked task no matter which
    /// socket `poll::wait` actually reported ready -- compiles, and before
    /// this test it also passed the whole suite. This is the increment's
    /// central new wake source, and nothing else asserted it consults the
    /// ready set at all.
    ///
    /// Two entries on different sockets, one of them reported ready: a mutant
    /// that wakes too many fails the `PARKED` assertion, one that wakes too
    /// few fails the `QUEUE` assertion. `wake_ready` reads no clock and does
    /// no I/O, so nothing here depends on timing.
    #[test]
    fn wake_ready_wakes_only_the_task_whose_socket_is_ready() {
        let ready_sock = RawSocket(11);
        let idle_sock = RawSocket(22);
        PARKED.with(|p| {
            let mut p = p.borrow_mut();
            p.push((
                1,
                Wait::Io {
                    socket: ready_sock,
                    interest: Interest::Read,
                    deadline: None,
                },
            ));
            p.push((
                2,
                Wait::Io {
                    socket: idle_sock,
                    interest: Interest::Read,
                    deadline: None,
                },
            ));
        });
        wake_ready(vec![ready_sock]);
        assert_eq!(
            QUEUE.with(|q| q.borrow().clone()),
            std::collections::VecDeque::from([1]),
            "only the task parked on the socket poll::wait reported ready \
             must be woken"
        );
        assert_eq!(
            PARKED.with(|p| p.borrow().clone()),
            vec![(
                2,
                Wait::Io {
                    socket: idle_sock,
                    interest: Interest::Read,
                    deadline: None,
                }
            )],
            "a task parked on a socket that was not reported ready must stay \
             parked"
        );
        PARKED.with(|p| p.borrow_mut().clear());
        QUEUE.with(|q| q.borrow_mut().clear());
    }

    /// How many times `poll_spin_n_times` reports `POLL_PENDING`, without
    /// ever staging a park, before it completes. Large enough that "the
    /// spinner already finished" and "the spinner still has turns left" are
    /// unmistakably different states for
    /// `a_self_requeuing_task_does_not_starve_a_sibling_parked_on_a_deadline`
    /// to tell apart.
    const SPIN_TURNS: i64 = 5;

    /// A poll function shaped like `yield_now`, or a spinning `join` before
    /// Task 3: it re-queues itself by returning `POLL_PENDING` without ever
    /// calling `stage_park`, `SPIN_TURNS` times, then completes.
    /// `STATE_SLOT_TAG` doubles as the turn counter.
    unsafe extern "C-unwind" fn poll_spin_n_times(state: *mut u8, _task_ctx: *mut u8) -> i64 {
        let slots = state as *mut i64;
        // SAFETY: `state` is a `STATE_MIN_SIZE` object from `test_future`.
        let turns = unsafe { slots.add(STATE_SLOT_TAG).read() };
        if turns < SPIN_TURNS {
            unsafe { slots.add(STATE_SLOT_TAG).write(turns + 1) };
            return POLL_PENDING;
        }
        unsafe { slots.add(STATE_SLOT_OUTPUT).write(0) };
        POLL_READY
    }

    /// Like `poll_park_once`, except the completing poll writes -- instead of
    /// a fixed `0` -- the current `STATE_SLOT_TAG` of a *different* state
    /// object, whose address is stashed in this one's first temp slot. Lets a
    /// test read out how far a sibling task had progressed at the exact
    /// moment this task was woken and completed, with no shared test-global
    /// mutable state and no dependence on wall-clock timing.
    unsafe extern "C-unwind" fn poll_park_then_read_sibling_progress(
        state: *mut u8,
        _task_ctx: *mut u8,
    ) -> i64 {
        let slots = state as *mut i64;
        // SAFETY: `state` is a `make_future(_, 1)` object: TAG, OUTPUT, then
        // one temp slot holding the sibling's state address.
        let tag = unsafe { slots.add(STATE_SLOT_TAG).read() };
        if tag == 0 {
            unsafe { slots.add(STATE_SLOT_TAG).write(1) };
            stage_park(Wait::Deadline(Instant::now()));
            return POLL_PENDING;
        }
        let sibling = unsafe { slots.add(STATE_SLOT_TEMPS).read() } as *mut i64;
        // SAFETY: the sibling is a live state object of at least
        // `STATE_MIN_SIZE`, spawned by the test and still registered (hence
        // still rooted) for as long as this task is running.
        let sibling_progress = unsafe { sibling.add(STATE_SLOT_TAG).read() };
        unsafe { slots.add(STATE_SLOT_OUTPUT).write(sibling_progress) };
        POLL_READY
    }

    /// The starvation bug a controller review caught in Task 2: a task that
    /// keeps re-queueing itself (`yield_now`, or -- until Task 3 -- a
    /// spinning `join`) must not be able to starve a sibling parked on a
    /// deadline. Before this test existed, `run_to_completion` only ever
    /// checked `PARKED` for due deadlines once its inner queue-draining pass
    /// found the ready queue empty; a task that never let the queue go empty
    /// (by re-queueing itself every turn) meant that check was never reached
    /// at all, so a sibling's deadline -- however short -- was never
    /// examined until the spinner happened to finish on its own. Task 2's
    /// end-to-end `sleep` fixture hit exactly this, underneath a spinning
    /// `join`, and hung indefinitely.
    ///
    /// Deterministic despite using a real clock: the sleeper's deadline is
    /// `Instant::now()` at the moment it first parks, so it is already due
    /// the instant anything next checks the clock, on any monotonic clock,
    /// regardless of how fast or slow the machine is. Neither poll function
    /// ever reaches `thread::sleep`, and nothing here asserts on elapsed
    /// time -- only on which of two turn counts is smaller.
    #[test]
    fn a_self_requeuing_task_does_not_starve_a_sibling_parked_on_a_deadline() {
        let spinner = test_future(poll_spin_n_times);
        // SAFETY: `spinner` is a well-formed future from `test_future`.
        let (_, spinner_state) = unsafe { read_future(spinner) };

        let sleeper = make_future(poll_park_then_read_sibling_progress, 1);
        // SAFETY: `sleeper` has one temp slot (`STATE_SLOT_TEMPS`), per
        // `make_future(_, 1)`, to stash the spinner's state address into.
        let (_, sleeper_state) = unsafe { read_future(sleeper) };
        unsafe {
            (sleeper_state as *mut i64)
                .add(STATE_SLOT_TEMPS)
                .write(spinner_state as i64);
        }

        // SAFETY: both are well-formed futures built above.
        let spinner_id = unsafe { spawn_internal(spinner) };
        let sleeper_id = unsafe { spawn_internal(sleeper) };

        // A trivial always-ready root, spawned last so `run_to_completion`'s
        // own `spawn_internal` call queues it behind the spinner and the
        // sleeper -- matching the shape of Task 2's fixture, where `both()`
        // (the root) spawns two tasks and then awaits them.
        let root = test_future(poll_ready_now);
        // SAFETY: well-formed future from `test_future`.
        assert_eq!(
            unsafe { run_to_completion(root) },
            7,
            "the root task must still complete normally"
        );

        assert!(
            is_done_internal(sleeper_id),
            "the deadline-parked task must have been woken and completed"
        );
        let sibling_progress_at_wake =
            TASKS.with(|tasks| tasks.borrow()[sleeper_id as usize].output);
        assert!(
            sibling_progress_at_wake < SPIN_TURNS,
            "the parked task must be woken while the spinner still has \
             pending turns left ({sibling_progress_at_wake} of {SPIN_TURNS} \
             turns spent) -- {sibling_progress_at_wake} >= {SPIN_TURNS} means \
             it was only woken once the spinner had already finished on its \
             own, which is the starvation bug this test exists to catch"
        );
        assert!(
            is_done_internal(spinner_id),
            "run_to_completion must not return until the spinner also \
             finishes, regardless of when the sleeper was woken"
        );
    }

    /// The fast path `poll_join`'s own doc comment promises: joining a task
    /// that has already finished completes on its first poll, with no park
    /// staged at all -- "a join on a finished task costs no suspension".
    /// Without this branch, every join would park at least once even when
    /// nothing is left to wait for.
    #[test]
    fn joining_an_already_done_task_completes_without_parking() {
        let target = make_future(poll_ready_now, 0);
        // SAFETY: well-formed future from `make_future`.
        let target_id = unsafe { spawn_internal(target) };
        // `poll_one` does not pop from `QUEUE` itself -- `spawn_internal`
        // queued `target_id`, and that entry must be drained by hand, or the
        // later pop below sees this stale entry instead of the joiner's.
        let popped_target = QUEUE.with(|q| q.borrow_mut().pop_front());
        assert_eq!(
            popped_target,
            Some(target_id),
            "spawn should have queued the target"
        );
        // SAFETY: `target_id` is currently registered.
        unsafe { poll_one(target_id) };
        assert!(
            is_done_internal(target_id),
            "target must be done before the join below means anything"
        );

        // SAFETY: `target` was spawned above, so `task_id_of` resolves it.
        let join_fut = unsafe { nova_rt_task_join_future(target) };
        // SAFETY: `join_fut` is a well-formed future `nova_rt_task_join_future`
        // built, so spawning it directly as a task is the same as spawning
        // any other future -- `poll_one` needs only a `{ poll_code, state }`
        // pair, and that is exactly what it is.
        let joiner_id = unsafe { spawn_internal(join_fut) };
        let popped = QUEUE.with(|q| q.borrow_mut().pop_front());
        assert_eq!(
            popped,
            Some(joiner_id),
            "spawn should have queued the joiner"
        );
        // SAFETY: `joiner_id` is currently registered.
        unsafe { poll_one(joiner_id) };

        assert!(
            is_done_internal(joiner_id),
            "joining an already-finished task must complete on the first \
             poll, not park"
        );
        assert!(
            PARKED.with(|p| p.borrow().is_empty()),
            "no park may be staged on the immediate-ready path"
        );
    }

    /// The suspend path: joining a task that has not yet finished parks on
    /// `Wait::Task`, and is woken -- moved back onto the ready queue -- the
    /// moment the target completes, with no re-poll of the joiner in
    /// between. `poll_join` always stages its wait with `deadline: None`
    /// (only a wrapping combinator like a timeout merges one in), so this
    /// bare park has no involvement of `wake_due` either -- only a *timed*
    /// `Wait::Task`, like a timed `Wait::Io`, would ever reach it.
    ///
    /// The join future is spawned directly as the joiner task, the same way
    /// `joining_an_already_done_task_completes_without_parking` does: a
    /// `{ poll_code, state }` pair is all `poll_one` needs, and
    /// `nova_rt_task_join_future` hands back exactly that, so no synthetic
    /// wrapper poll function is needed to drive `poll_join` through the real
    /// stage/commit park protocol (`CURRENT`/`PENDING_PARK`).
    #[test]
    fn joining_a_pending_task_parks_and_is_woken_when_the_target_finishes() {
        let target = make_future(poll_ready_now, 0);
        // SAFETY: well-formed future from `make_future`.
        let target_id = unsafe { spawn_internal(target) };
        // SAFETY: `target` was just spawned, so `task_id_of` resolves it.
        let join_fut = unsafe { nova_rt_task_join_future(target) };
        // SAFETY: `join_fut` is a well-formed future `nova_rt_task_join_future`
        // built.
        let joiner_id = unsafe { spawn_internal(join_fut) };

        // Drain the queue `spawn_internal` filled for both, so the rest of
        // this test can poll in a deliberately chosen order instead of FIFO
        // order -- the joiner first, before its target has ever run.
        assert_eq!(
            QUEUE.with(|q| q.borrow_mut().drain(..).collect::<Vec<_>>()),
            vec![target_id, joiner_id],
            "spawn should have queued both, in order"
        );

        // SAFETY: `joiner_id` was just spawned, so it is currently registered.
        unsafe { poll_one(joiner_id) };
        assert!(
            !is_done_internal(joiner_id),
            "the joiner must not complete while its target is still pending"
        );
        assert_eq!(
            PARKED.with(|p| p.borrow().clone()),
            vec![(
                joiner_id,
                Wait::Task {
                    id: target_id,
                    deadline: None
                }
            )],
            "the joiner must be parked on the target's id, not re-queued"
        );
        assert!(
            QUEUE.with(|q| q.borrow().is_empty()),
            "a parked task must not also sit in the ready queue"
        );

        // SAFETY: `target_id` is currently registered.
        unsafe { poll_one(target_id) };
        assert!(
            is_done_internal(target_id),
            "poll_ready_now completes on its first poll"
        );
        assert!(
            PARKED.with(|p| p.borrow().is_empty()),
            "the target's completion must wake the joiner out of the park set"
        );
        let woken = QUEUE.with(|q| q.borrow_mut().pop_front());
        assert_eq!(
            woken,
            Some(joiner_id),
            "the woken joiner must be the one re-queued"
        );

        // SAFETY: `joiner_id` is currently registered.
        unsafe { poll_one(joiner_id) };
        assert!(
            is_done_internal(joiner_id),
            "once woken, the joiner must complete -- its target is now done"
        );
    }

    #[test]
    fn a_deadlock_names_every_parked_task_and_its_reason() {
        PARKED.with(|p| {
            let mut p = p.borrow_mut();
            p.push((
                1,
                Wait::Task {
                    id: 2,
                    deadline: None,
                },
            ));
            p.push((
                2,
                Wait::Task {
                    id: 1,
                    deadline: None,
                },
            ));
        });
        let report = deadlock_report();
        assert!(report.contains("2 tasks are parked"), "got: {report}");
        assert!(
            report.contains("task 1 is waiting for task 2 to finish"),
            "every parked task must be named, not just the first -- got: {report}"
        );
        assert!(
            report.contains("task 2 is waiting for task 1 to finish"),
            "the second parked task is missing -- got: {report}"
        );
        PARKED.with(|p| p.borrow_mut().clear());
    }

    /// `deadlock_report`'s singular-vs-plural headline, exercised with
    /// exactly one parked task -- the only other test that calls
    /// `deadlock_report` (`a_deadlock_names_every_parked_task_and_its_reason`,
    /// directly above) pushes two, so a hardcoded `"tasks are"` would pass
    /// unnoticed there. A task parked on its own id is also the smallest
    /// possible deadlock: nothing else needs to exist for it to be
    /// unwakeable.
    #[test]
    fn a_task_parked_on_itself_is_reported_as_a_deadlock() {
        PARKED.with(|p| {
            p.borrow_mut().push((
                3,
                Wait::Task {
                    id: 3,
                    deadline: None,
                },
            ))
        });
        let report = deadlock_report();
        assert!(
            report.contains("task 3 is waiting for task 3 to finish"),
            "got: {report}"
        );
        assert!(
            report.contains("1 task is"),
            "singular headline -- got: {report}"
        );
        PARKED.with(|p| p.borrow_mut().clear());
    }

    /// The whole point of this task: an I/O park must describe itself as
    /// waiting on i/o, not fall through to a deadline-shaped or task-shaped
    /// message -- and, more importantly, `run_to_completion`'s drive loop
    /// must never treat it as a deadlock in the first place (see the
    /// `(None, false)` arm there). This test covers only the message. The
    /// `(None, false)` arm itself still has no dedicated *unit* test, but no
    /// longer for the reason an earlier version of this comment gave -- that
    /// nothing outside a test could populate a real, unresolved `Wait::Io`
    /// through `run_to_completion`. `net.rs` now stages exactly that from
    /// three of its four operations (`connect`, `read` and `write` all pass
    /// `deadline: None`), so the arm is reached in production and end to end by
    /// every `std/net` runtime fixture: `tests/runtime/net_interleave.nova`'s
    /// `connect` in `main` parks the only live task on an untimed `Wait::Io`
    /// and drains the queue behind it, which is that arm exactly.
    #[test]
    fn a_park_on_io_with_no_deadline_is_not_a_deadlock() {
        let report = with_parked(
            &[(
                7,
                Wait::Io {
                    socket: RawSocket(-1),
                    interest: Interest::Read,
                    deadline: None,
                },
            )],
            deadlock_report,
        );
        assert!(
            report.contains("waiting on i/o"),
            "an I/O park must describe itself, got: {report}"
        );
    }

    /// A poll function that stages a park and then reports `POLL_READY` anyway --
    /// the shape that would strand a finished task in the park set.
    unsafe extern "C-unwind" fn poll_stage_then_ready(state: *mut u8, _task_ctx: *mut u8) -> i64 {
        let slots = state as *mut i64;
        stage_park(Wait::Task {
            id: 0,
            deadline: None,
        });
        // SAFETY: `state` is a `STATE_MIN_SIZE` object from `test_future`.
        unsafe { slots.add(STATE_SLOT_OUTPUT).write(0) };
        POLL_READY
    }

    #[test]
    fn a_park_staged_by_a_poll_that_completes_is_discarded() {
        let fut = test_future(poll_stage_then_ready);
        // SAFETY: well-formed future from `test_future`.
        let out = unsafe { run_to_completion(fut) };
        assert_eq!(out, 0);
        assert!(
            PARKED.with(|p| p.borrow().is_empty()),
            "a completed task must not be left parked -- it would fake a deadlock forever"
        );
    }

    /// The discard in `poll_one` (`PENDING_PARK.with(|p| p.take())`) must
    /// actually clear the cell, not just read it -- otherwise a park staged
    /// by one task's completing poll would still be sitting in `PENDING_PARK`
    /// when the *next* task is polled, and that unrelated task would be
    /// swept into the park set on the first task's stale wait instead of
    /// being re-queued. `a_park_staged_by_a_poll_that_completes_is_discarded`
    /// cannot catch this: it only ever polls the one task that staged the
    /// park, so it never observes what a `.take()` -> `.get()` regression
    /// does to whichever task is polled afterward.
    #[test]
    fn a_discarded_park_does_not_leak_onto_the_next_task_polled() {
        let a = test_future(poll_stage_then_ready);
        let b = test_future(poll_suspend_once);
        // SAFETY: both are well-formed futures from `test_future`.
        let (id_a, id_b) = unsafe { (spawn_internal(a), spawn_internal(b)) };
        // `spawn_internal` queues each id as it spawns it; drain that so the
        // final QUEUE assertion below reflects only what `poll_one` itself
        // does with each task, matching
        // `a_staged_park_moves_the_task_out_of_the_ready_queue`'s pattern of
        // popping before polling.
        assert_eq!(
            QUEUE.with(|q| q.borrow_mut().drain(..).collect::<Vec<_>>()),
            vec![id_a, id_b],
            "spawn should have queued both, in order"
        );
        // SAFETY: `id_a`/`id_b` were just returned by `spawn_internal`, so
        // both are currently-registered task ids.
        unsafe {
            poll_one(id_a);
            poll_one(id_b);
        }
        assert!(
            PARKED.with(|p| p.borrow().is_empty()),
            "b never called stage_park and returned POLL_PENDING on its own; \
             it must be re-queued, not parked on a's leftover wait -- got: {:?}",
            PARKED.with(|p| p.borrow().clone())
        );
        assert_eq!(
            QUEUE.with(|q| q.borrow().clone()),
            std::collections::VecDeque::from([id_b]),
            "b must be the one task left in the ready queue"
        );
    }

    /// Windows-only, matching `gc.rs`'s own `mod registry` precedent, and for
    /// the identical reason: both tests below call `gc::collect_for_test`,
    /// which runs the real, stack-scanning `collect()`. `stack_base()`
    /// (`gc.rs`) only has a real implementation on Windows; elsewhere it
    /// returns `None`, so `collect()` returns before marking anything and an
    /// `is_some()` assertion would pass vacuously while an `is_none()`
    /// assertion would fail outright. `.github/workflows/ci.yml` runs
    /// ubuntu, windows and macos, so leaving this ungated would land red on
    /// two of three jobs and green on the third for the wrong reason.
    ///
    /// **Both tests below are also unconditionally `#[ignore]`d**, for the
    /// identical mechanism as `gc.rs`'s `mod registry` (see that module's doc
    /// comment, and `docs/adr/0010-conservative-scan-root-test-gating.md` for
    /// the measurements): under default test parallelism, a stale stack word
    /// in an already-returned frame is intermittently read as a conservative
    /// root, in debug as well as release.
    /// `an_unspawned_tasks_state_is_swept` is the one that actually flakes --
    /// the ADR's diagnostic capture is of this exact test.
    /// `a_spawned_tasks_state_is_registered_as_a_gc_root` is gated alongside
    /// it rather than left green alone, for the same negative-control
    /// pairing reason `gc.rs`'s doc comment explains: otherwise it would keep
    /// passing even against a registry that never registered anything, which
    /// the sweep test exists to rule out. Reachable with
    /// `cargo test -- --ignored`, which CI runs as an advisory,
    /// `continue-on-error` step.
    ///
    /// Neither test is this module's coverage of the *pairing* of
    /// `gc::add_root` with `gc::remove_root`. That invariant is asserted
    /// deterministically, on the registry itself, by
    /// `a_completed_tasks_state_stays_rooted_until_its_output_is_taken`
    /// above -- which needs no collection, so it runs on every platform and
    /// in every CI job.
    #[cfg(windows)]
    mod root_registration {
        use super::*;

        /// A local copy of `gc.rs`'s `tests::registry::hide`/`reveal`
        /// technique: a bitwise-complement encoding that carries an address
        /// across a call boundary with no in-range bit pattern for it
        /// anywhere in the frame or a register, so the conservative scanner
        /// cannot mistake the carried value itself for a root.
        ///
        /// Duplicated here rather than reused from `gc.rs`: Task 3's
        /// `hide`/`reveal` are private to a module nested two levels inside
        /// `gc`'s own `#[cfg(test)]` tests (`tests::registry`), not
        /// reachable from a sibling module's tests, and the technique is two
        /// lines of pure bit-twiddling -- reimplementing it here is less
        /// invasive than restructuring `gc.rs`'s existing, already-reviewed
        /// test module.
        ///
        /// `#[inline(never)]` on both, for the same reason as `gc.rs`'s
        /// originals: without it, an optimizing build can inline both calls,
        /// prove `reveal(hide(x)) == x`, and keep the original `x` live
        /// across a collection instead of ever materializing the hidden
        /// form.
        #[inline(never)]
        fn hide(addr: usize) -> usize {
            !addr
        }

        #[inline(never)]
        fn reveal(hidden: usize) -> usize {
            !hidden
        }

        /// Build a `poll_suspend_once` future, optionally spawn it, and hand
        /// back its state object's address hidden (see [`hide`]).
        ///
        /// A dedicated `#[inline(never)]` function, not inlined into the
        /// tests themselves, so every raw value this scenario needs -- the
        /// future's fat pointer, the state address read back from it -- is
        /// local to this one call and provably dead once it returns, rather
        /// than potentially left live in the *caller's* frame or in a
        /// register the caller happens to still be holding it in. This
        /// mirrors `gc.rs`'s `tests::registry::setup_parent_and_child`,
        /// which documents the same call-boundary requirement in more
        /// detail.
        ///
        /// Both `fut` (itself a live GC allocation -- `make_future`'s fat
        /// pointer is `gc::alloc`-ed and scanned) and `state` are `mut` and
        /// overwritten by reassignment, never shadowed: shadowing binds a
        /// new name without clearing the original stack slot, which is
        /// exactly the hazard the task brief's STOP box warns about. `fut`
        /// is cleared for the same reason as `state`: left live, its own
        /// scanned contents (word 1 is `state`'s address) would transitively
        /// re-root `state` regardless of whether `gc::add_root` does
        /// anything at all, defeating the test independently of `state`'s
        /// own handling.
        #[inline(never)]
        fn setup_task_state(spawn: bool) -> usize {
            let mut fut = make_future(poll_suspend_once, 0);
            let mut state = state_of(fut);
            let hidden = hide(state);
            if spawn {
                unsafe {
                    nova_rt_task_spawn(fut);
                }
            }
            fut = std::ptr::null_mut::<u8>();
            state = 0;
            std::hint::black_box(fut);
            std::hint::black_box(state);
            hidden
        }

        #[test]
        #[ignore = "not flaky on its own -- never observed to fail by itself. \
                    Gated only to stay paired with the sweep-asserting control \
                    test that does flake: this one asserts an object SURVIVED, \
                    which also passes against a collector that frees nothing at \
                    all, so running it while its control is ignored would leave \
                    a green assertion proving nothing. See \
                    docs/adr/0010-conservative-scan-root-test-gating.md. \
                    Reachable with `cargo test -- --ignored`."]
        fn a_spawned_tasks_state_is_registered_as_a_gc_root() {
            // The positive half: a task's state object, reachable only
            // through the executor's root registration (no Nova stack slot,
            // no register -- see `setup_task_state`), must survive a real
            // collection.
            let hidden = setup_task_state(true);

            gc::collect_for_test();

            let state = reveal(hidden);
            assert!(
                gc::object_info(state).is_some(),
                "a parked task's state object was swept; add_root is not wired"
            );
        }

        #[test]
        #[ignore = "flaky: asserts an unreachable object was SWEPT, and the \
                    conservative stack scan intermittently retains it instead \
                    -- a stale stack word in an already-returned frame is read \
                    as a root, in debug as well as release, at default test \
                    parallelism. Mechanism identified, not fixed; two remedies \
                    were each measured to make it worse. See \
                    docs/adr/0010-conservative-scan-root-test-gating.md. \
                    Reachable with `cargo test -- --ignored`."]
        fn an_unspawned_tasks_state_is_swept() {
            // The required negative control (task brief STOP box): the
            // positive test above is worthless alone, since it would also
            // pass against an executor that registers nothing at all AND
            // against a collector that frees nothing at all. Identical
            // shape to the positive test except `spawn = false`, so the
            // state object is never registered and has no other root.
            let hidden = setup_task_state(false);

            gc::collect_for_test();

            let state = reveal(hidden);
            assert!(
                gc::object_info(state).is_none(),
                "an unregistered, unreachable task state survived a collection; \
                 this test cannot discriminate a working registry from a \
                 collector that frees nothing"
            );
        }
    }
}
