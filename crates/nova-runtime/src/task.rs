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
//! No wakers: a task that returns [`POLL_PENDING`] is simply re-queued for
//! another turn, with no registered interest in any external event. That
//! makes interleaving between tasks deterministic by construction, which is
//! what lets [`nova_rt_task_block_on`]'s round-robin order be pinned by a
//! test rather than merely observed.

use crate::gc;
use std::cell::{Cell, RefCell};
use std::collections::VecDeque;

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
/// regardless of the workspace's unwind panic strategy. This module's own
/// hand-written test poll functions *are* ordinary panicking Rust, and
/// `nova_rt_task_block_on`'s re-entrancy guard needs a panic starting inside
/// one to reach a `catch_unwind` several frames up, through both this
/// indirect call and `nova_rt_task_block_on`'s own boundary. Declaring the
/// ABI `"C-unwind"` permits that. It changes nothing else: `"C-unwind"` was
/// stabilized in Rust 1.71 (under this workspace's 1.78 MSRV) and marshals
/// parameters and return values identically to `"C"`, so a generated call
/// site is byte-identical either way.
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
/// returning. The permission this ABI grants exists for the Rust-side entry
/// points below, not for compiled Nova frames.
pub type PollFn = unsafe extern "C-unwind" fn(state: *mut u8, task_ctx: *mut u8) -> i64;

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
    /// Whether [`nova_rt_task_take_output`] has already handed `output` out.
    /// Taking is what releases this task's GC root (see
    /// [`take_output_internal`]), so it must happen at most once.
    taken: bool,
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
    /// Set for the duration of one `nova_rt_task_block_on` call, so a nested
    /// call (a poll function calling `block_on` again) can be diagnosed
    /// instead of running a second executor loop from inside the first
    /// one's frame and corrupting the shared queue.
    static IN_BLOCK_ON: Cell<bool> = const { Cell::new(false) };
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
    QUEUE.with(|queue| queue.borrow_mut().push_back(id));
    id
}

/// Poll task `id` once. On [`POLL_PENDING`], re-queue it for another turn. On
/// [`POLL_READY`], copy its output out of the state object into the `Task`
/// record and mark it done.
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
    // SAFETY: `poll`/`state` came from a `Task` this module built in
    // `spawn_internal` from a caller-guaranteed-valid future fat pointer;
    // `task_ctx` is always null (see `PollFn`'s doc comment -- unused in
    // this phase).
    let status = unsafe { poll(state, std::ptr::null_mut()) };
    if status == POLL_PENDING {
        QUEUE.with(|queue| queue.borrow_mut().push_back(id));
        return;
    }
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
}

fn is_done_internal(id: i64) -> bool {
    TASKS.with(|tasks| {
        tasks
            .borrow()
            .get(id as usize)
            .expect("nova_rt_task_is_done: unknown task id")
            .done
    })
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
/// task whose output is *never* taken keeps its state object rooted for the
/// rest of the process. That is a leak, not unsoundness, and it is the
/// deliberate trade -- the alternative (unroot at completion) frees a
/// heap-valued output while `Task::output` still names it.
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
/// 2. **This loop does not terminate if any queued task never reports
///    [`POLL_READY`].** A task that returns [`POLL_PENDING`] forever is
///    re-queued forever, and the queue never empties. In 2.3a that is
///    unreachable: every suspension is a `yield_now`-shaped one that resumes
///    on the next turn, so a task's tag always advances. It becomes reachable
///    the moment 2.3b adds a primitive that can park on an external event --
///    an `await` on a channel nothing ever sends to. Whatever introduces such
///    a primitive owns the fix (a park set holding tasks that are waiting on
///    something rather than ready, and a deadlock diagnostic when the queue
///    is empty of ready tasks but the park set is not); a busy re-poll loop
///    has no way to tell "not ready yet" from "never will be".
///
/// # Safety
/// `future` must be a valid future fat pointer (see [`read_future`]).
unsafe fn run_to_completion(future: *mut u8) -> i64 {
    // SAFETY: forwarding this function's own contract.
    let root_id = unsafe { spawn_internal(future) };
    while let Some(id) = QUEUE.with(|queue| queue.borrow_mut().pop_front()) {
        // SAFETY: `id` was just popped from `QUEUE`, so it is a currently
        // registered task id (every id is pushed by `spawn_internal` exactly
        // once and re-pushed by `poll_one` itself only for a task that is
        // not yet done).
        unsafe { poll_one(id) };
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
/// `"C-unwind"`: see [`PollFn`]'s doc comment. This function is the one that
/// actually panics on re-entrancy, so it is the one this ABI choice exists
/// for: under plain `"C"`, the panic below is unable to leave this
/// function's own frame at all, and the process aborts on the spot rather
/// than unwinding to any caller.
///
/// # Safety
/// `future` must satisfy [`nova_rt_task_spawn`]'s contract exactly -- same
/// [`FUTURE_SIZE`]-byte fat pointer, same non-null `poll_code`, and the same
/// minimum of `(STATE_SLOT_TEMPS + n_temps) * 8` scanned, writable bytes
/// (never fewer than [`STATE_MIN_SIZE`]) for the state object, because
/// [`STATE_SLOT_OUTPUT`] is read unconditionally on completion here too.
///
/// # Panics
/// If called while this thread is already inside a `nova_rt_task_block_on`
/// call -- a poll function must not call `block_on` itself, which would run
/// a second executor loop from inside the first one's frame and corrupt the
/// shared queue rather than being diagnosed. Also if any polled task's poll
/// function returns a status that is neither [`POLL_PENDING`] nor
/// [`POLL_READY`] (see [`poll_one`]).
#[no_mangle]
pub unsafe extern "C-unwind" fn nova_rt_task_block_on(future: *mut u8) -> i64 {
    if IN_BLOCK_ON.with(|in_block_on| in_block_on.get()) {
        panic!("nova_rt_task_block_on called re-entrantly: a poll function must not call block_on");
    }
    IN_BLOCK_ON.with(|in_block_on| in_block_on.set(true));
    // `AssertUnwindSafe`: the closure only captures a `*mut u8`, which is
    // GC-owned data with no invariant that spans a panic here, so asserting
    // unwind-safety is correct rather than papering over a real hazard.
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        // SAFETY: forwarding this function's own contract.
        unsafe { run_to_completion(future) }
    }));
    // Cleared on every path out of this call, including a panic (from a
    // nested re-entrant call, or from a poll function itself) unwinding
    // through the `catch_unwind` above -- otherwise a single panicking
    // `block_on` would leave every later call on this thread permanently
    // (and incorrectly) diagnosed as re-entrant.
    IN_BLOCK_ON.with(|in_block_on| in_block_on.set(false));
    match result {
        Ok(output) => output,
        Err(payload) => std::panic::resume_unwind(payload),
    }
}

/// Whether task `id` (as returned by [`nova_rt_task_spawn`]) has completed.
///
/// `"C-unwind"`: see [`PollFn`]'s doc comment. Sharing one ABI across every
/// entry point in this module means a caller never has to know which of
/// them can panic; both of `is_done`/`take_output`'s internal helpers do, on
/// an unknown task id.
///
/// # Safety
/// `id` must be an id previously returned by `nova_rt_task_spawn` on this
/// same thread.
#[no_mangle]
pub unsafe extern "C-unwind" fn nova_rt_task_is_done(id: i64) -> i8 {
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

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!(unsafe { nova_rt_task_is_done(id) }, 0);
        let root = make_future(poll_ready_now, 0);
        unsafe { nova_rt_task_block_on(root) };
        assert_eq!(unsafe { nova_rt_task_is_done(id) }, 1);
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
        assert_eq!(unsafe { nova_rt_task_is_done(id) }, 1);
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
        assert_eq!(unsafe { nova_rt_task_is_done(id) }, 0);

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

    #[test]
    fn a_re_entrant_block_on_panics() {
        // Nesting an executor inside a poll would run a task from inside
        // another task's frame. Diagnose it instead of corrupting the queue.
        //
        // `AssertUnwindSafe` is required: the closure captures a `*mut u8`,
        // and raw pointers are not `UnwindSafe`, so a bare `catch_unwind`
        // does not compile. Asserting unwind-safety is correct here -- the
        // pointer is GC-owned and no invariant spans the panic.
        //
        // This test assumes the unwind panic strategy, which is the
        // workspace default (`Cargo.toml` sets no `panic = "abort"` profile
        // override). If a profile ever adds one, this must become a
        // subprocess test instead.
        unsafe extern "C-unwind" fn poll_reenters(_s: *mut u8, _c: *mut u8) -> i64 {
            let inner = make_future(poll_ready_now, 0);
            unsafe { nova_rt_task_block_on(inner) }
        }
        let fut = make_future(poll_reenters, 0);
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| unsafe {
            nova_rt_task_block_on(fut)
        }));
        assert!(r.is_err(), "re-entrant block_on must panic, not nest");
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
