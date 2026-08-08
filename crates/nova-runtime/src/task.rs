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
/// `"C-unwind"`, not plain `"C"`: a poll function is ordinary (panicking)
/// Rust code under this ABI, same as every `nova_rt_task_*` entry point
/// below, and a panic that cannot cross a plain `"C"` function's boundary
/// does not unwind -- it aborts the whole process on the spot, regardless of
/// the workspace's unwind panic strategy. `nova_rt_task_block_on`'s
/// re-entrancy guard depends on a panic starting inside a poll function
/// being able to reach a `catch_unwind` several call frames up, through both
/// this indirect call and `nova_rt_task_block_on`'s own boundary.
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

/// One task known to this thread's executor.
struct Task {
    poll: PollFn,
    state: *mut u8,
    done: bool,
    output: i64,
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
    /// `nova_rt_task_take_output` can be called an arbitrary time later.
    static TASKS: RefCell<Vec<Option<Task>>> = const { RefCell::new(Vec::new()) };
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
/// `future` must point to a live, readable 16-byte block: word 0 a
/// [`PollFn`] (as a bit pattern), word 1 the state object's address.
unsafe fn read_future(future: *mut u8) -> (PollFn, *mut u8) {
    let words = future as *mut usize;
    // SAFETY: `future` is a valid 16-byte block per this function's contract.
    let poll_code = unsafe { words.read() };
    // SAFETY: same block, second word.
    let state = unsafe { words.add(1).read() } as *mut u8;
    // SAFETY: `poll_code` is a `PollFn` bit pattern by `future`'s own
    // contract above; a function pointer and a `usize` are both
    // pointer-width, so the transmute cannot change size.
    let poll: PollFn = unsafe { std::mem::transmute::<usize, PollFn>(poll_code) };
    (poll, state)
}

/// Register `future` as a new task: read its fat pointer, root its state
/// object (see the module doc comment for why `gc::add_root`/`remove_root`
/// pair with spawn/completion), enqueue it, and return its id.
///
/// # Safety
/// `future` must be a valid future fat pointer (see [`read_future`]).
unsafe fn spawn_internal(future: *mut u8) -> i64 {
    // SAFETY: forwarding this function's own contract.
    let (poll, state) = unsafe { read_future(future) };
    // The state object sits on no Nova stack and in no register while its
    // task is queued or parked -- the executor is its only reference, and
    // those are the collector's only other root sources (see `gc.rs`'s
    // module doc comment). Paired with `gc::remove_root` in `poll_one` on
    // completion.
    gc::add_root(state);
    let id = TASKS.with(|tasks| {
        let mut tasks = tasks.borrow_mut();
        let id = tasks.len() as i64;
        tasks.push(Some(Task {
            poll,
            state,
            done: false,
            output: 0,
        }));
        id
    });
    QUEUE.with(|queue| queue.borrow_mut().push_back(id));
    id
}

/// Poll task `id` once. On [`POLL_PENDING`], re-queue it for another turn.
/// On completion, copy its output out of the state object into the `Task`
/// record *before* calling `gc::remove_root` -- reversed, a collection
/// between the two calls could sweep a heap-valued output that nothing else
/// roots yet.
///
/// # Safety
/// `id` must be a currently-registered task id.
unsafe fn poll_one(id: i64) {
    let (poll, state) = TASKS.with(|tasks| {
        let tasks = tasks.borrow();
        let task = tasks
            .get(id as usize)
            .and_then(|slot| slot.as_ref())
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
    // SAFETY: `state` is the same live state object `spawn_internal` rooted;
    // `STATE_SLOT_OUTPUT` is within its bounds regardless of how many temp
    // slots follow it.
    let output = unsafe { (state as *mut i64).add(STATE_SLOT_OUTPUT).read() };
    TASKS.with(|tasks| {
        let mut tasks = tasks.borrow_mut();
        let task = tasks
            .get_mut(id as usize)
            .and_then(|slot| slot.as_mut())
            .expect("poll_one: task id is not registered");
        task.output = output;
        task.done = true;
    });
    gc::remove_root(state);
}

fn is_done_internal(id: i64) -> bool {
    TASKS.with(|tasks| {
        tasks
            .borrow()
            .get(id as usize)
            .and_then(|slot| slot.as_ref())
            .expect("nova_rt_task_is_done: unknown task id")
            .done
    })
}

fn take_output_internal(id: i64) -> i64 {
    TASKS.with(|tasks| {
        tasks
            .borrow()
            .get(id as usize)
            .and_then(|slot| slot.as_ref())
            .expect("nova_rt_task_take_output: unknown task id")
            .output
    })
}

/// Spawn `future` as this thread's newest task, then run this thread's
/// executor until every task registered so far -- `future`'s own and any
/// other still-pending task -- has completed, and return `future`'s output.
///
/// This drains the whole shared queue rather than stopping as soon as
/// `future` itself is done: with no waker and no background driver, a
/// `block_on` call is the only thing that ever advances any task on this
/// thread, so a task spawned earlier and left pending has no other way to
/// ever reach `nova_rt_task_take_output`'s promised state (`done`, with an
/// output) except by being polled to completion inside some `block_on` call.
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
    take_output_internal(root_id)
}

/// Queue a `{ poll_code, state }` future as a new task and return its id.
///
/// `"C-unwind"`: see [`PollFn`]'s doc comment for why every entry point in
/// this module shares that ABI rather than plain `"C"`.
///
/// # Safety
/// `future` must point to a live 16-byte `{ poll_code, state }` block.
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
/// `future` must point to a live 16-byte `{ poll_code, state }` block.
///
/// # Panics
/// If called while this thread is already inside a `nova_rt_task_block_on`
/// call -- a poll function must not call `block_on` itself, which would run
/// a second executor loop from inside the first one's frame and corrupt the
/// shared queue rather than being diagnosed.
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

/// A completed task's output slot.
///
/// `"C-unwind"`: see [`nova_rt_task_is_done`]'s doc comment.
///
/// # Safety
/// `id` must be an id previously returned by `nova_rt_task_spawn` on this
/// same thread.
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
        let state = gc::alloc((STATE_SLOT_TEMPS + temps) * 8, true);
        let fat = gc::alloc(16, true);
        unsafe {
            (fat as *mut usize).write(f as usize);
            (fat as *mut usize).add(1).write(state as usize);
        }
        fat
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

    #[test]
    fn two_pending_tasks_interleave_round_robin() {
        // The determinism the sub-phase gate depends on. Records the order
        // poll calls actually happen in, and asserts an ALTERNATING order --
        // not merely that both ran, which a run-to-completion scheduler would
        // also satisfy while producing different output in the gate fixture.
        static ORDER: std::sync::Mutex<Vec<i64>> = std::sync::Mutex::new(Vec::new());
        ORDER.lock().expect("lock").clear();

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
            ORDER.lock().expect("lock").push(who);
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
        let order = ORDER.lock().expect("lock").clone();
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
            let mut state = unsafe { (fut as *mut usize).add(1).read() };
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
        fn a_spawned_tasks_state_is_registered_as_a_gc_root() {
            // The positive half: a task's state object, reachable only
            // through the executor's root registration (no Nova stack slot,
            // no register -- see `setup_task_state`), must survive a real
            // collection.
            //
            // `lock_scan_test`: serializes with every other test that calls
            // the real `collect()` (this module's sibling below, and
            // `gc.rs`'s own `mod registry`) -- see its doc comment in `gc.rs`.
            let _guard = gc::lock_scan_test();
            let hidden = setup_task_state(true);

            gc::collect_for_test();

            let state = reveal(hidden);
            assert!(
                gc::object_info(state).is_some(),
                "a parked task's state object was swept; add_root is not wired"
            );
        }

        #[test]
        fn an_unspawned_tasks_state_is_swept() {
            // The required negative control (task brief STOP box): the
            // positive test above is worthless alone, since it would also
            // pass against an executor that registers nothing at all AND
            // against a collector that frees nothing at all. Identical
            // shape to the positive test except `spawn = false`, so the
            // state object is never registered and has no other root.
            let _guard = gc::lock_scan_test();
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
