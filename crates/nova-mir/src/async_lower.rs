//! The async state-machine transform: an `async fn` becomes a resumable poll
//! function.
//!
//! A post-monomorphization pass over the finished [`Module`]. Each function
//! [`crate::Function::is_async`] flags is split into two:
//!
//! - **`<mangled>$poll`** — the original body, rewritten to hold every value
//!   in a heap *state object* rather than in a temp, with the signature
//!   `(state, task_ctx) -> i64` the runtime's `PollFn` declares. Its
//!   environment pointer **is** the state object, so the existing closure
//!   calling convention (`takes_env`: temp 0 is the env) carries it with no
//!   new ABI.
//! - **`<mangled>`** — a wrapper keeping the original symbol and parameter
//!   list, returning the `{ poll_code, state }` two-word future value its
//!   callers were already compiled to expect for a `Future<T>`.
//!
//! Running on the finished module rather than during lowering means the pass
//! sees no generics and lands once for both codegen backends.
//!
//! # How a body is split
//!
//! A body's suspend points arrive as [`Stmt::Await`] markers. Each block is cut
//! at every one of them, and each cut expands into three blocks: one that polls
//! the awaited future, one that suspends, and one that carries on. The poll
//! function's entry becomes a [`Terminator::Switch`] on the resume tag, sending
//! tag 0 to the body's original entry and tag *k* to await *k*'s poll block — an
//! existing terminator, so neither backend needs a new construct.
//!
//! Awaiting means polling: a future value *is* the `{ poll_code, state }` fat
//! pointer `Stmt::CallIndirect` already knows how to call, and calling it passes
//! the inner state object as the leading environment argument, which is exactly
//! `PollFn`'s first parameter. On `POLL_READY` the continuation copies the
//! awaited value out of the inner state object's own output slot. On
//! `POLL_PENDING` the suspend stores **this await's own tag**, not the next one:
//! an inner future may report pending any number of times, and a tag one higher
//! would resume the task past a suspend point that never finished — reading an
//! output slot nothing has written, which completes with a wrong value rather
//! than failing.
//!
//! ## Awaiting one future twice
//!
//! Nothing here stops a body from awaiting the same future value twice
//! (`let fut = g()` then `fut.await + fut.await`), and the result is that the
//! inner poll function is re-entered at whatever tag its state object holds, so
//! it re-runs everything after its own last suspend point — its side effects
//! included. Rust makes the shape unrepresentable instead, by move-checking the
//! future out of the awaiting expression.
//!
//! This is not a miscompile of the split: each await polls a future that says it
//! is ready and reads the slot that future wrote. It is a missing *front-end*
//! rule, and it belongs to whatever gives `Future` an ownership story. Named here
//! because the split is what made the shape reachable.
//!
//! # Why every value goes to the state object
//!
//! Nothing in an await-free body needs to be spilled: with no suspend point,
//! no value can be live across one. The spill is unconditional anyway because
//! the split puts blocks that run in *different* invocations of the poll
//! function either side of a suspend, and a value left in a temp across such a
//! split is read on a path where nothing defined it — which the Cranelift
//! frontend resolves as a block parameter fed from a predecessor that never
//! executes, i.e. garbage, with no diagnostic. Spilling everything removes the
//! liveness question instead of answering it, for every body rather than only
//! the ones a liveness analysis would get right.
//!
//! # A panic must not cross a poll function's boundary
//!
//! The `PollFn` ABI is `"C-unwind"`, which permits an unwind to pass through
//! without aborting, but a Cranelift- or LLVM-emitted frame has no landing pads
//! and no drop glue, so an unwind *through* a poll function would skip whatever
//! the executor's bookkeeping needs. What this pass contributes to that
//! requirement is narrow and checkable: **the only call it introduces into a
//! poll function is the indirect poll of an awaited future.** Everything else it
//! emits is a field load, a field store, an integer constant or a terminator,
//! and the other calls in a poll body are the ones the `async fn` already
//! contained, under the same discipline as any other Nova function
//! (`nova_rt_panic_str` and `nova_rt_check_bounds` abort rather than unwind, and
//! a `Terminator::Trap` becomes a trap instruction).
//!
//! That one call is sound by induction rather than by inspection, because its
//! callee is a value and not a symbol. What the value can be is narrow. `Future`
//! is a *nameable* type — `fn take(f: Future<Int>)` type-checks — but naming it
//! is not constructing one: the language has no future literal and no
//! constructor, and an `extern` signature cannot mention one either (`Future` is
//! not FFI-safe, `require_ffi_safe` in `nova-typeck`), so no foreign function
//! pointer can reach an await.
//!
//! Poll code therefore comes from exactly two *kinds* of source, and each
//! carries the obligation itself:
//!
//! - The `$poll` functions this pass generates, under this same paragraph's
//!   discipline — the induction step.
//! - Every hand-written `PollFn` in `nova-runtime`, each stating its own
//!   no-unwind justification at its own definition rather than here:
//!   `nova_rt_task_yield_future`'s `poll_yield_once` (what `yield_now`
//!   awaits, because an `async fn` body suspends only at an `.await` and
//!   nothing in the language is a future that is not already ready) and
//!   `nova_rt_task_sleep_future_nanos`'s `poll_sleep` (what `sleep` awaits) are two
//!   of them, not an exhaustive count — `nova_rt_task_join_future`'s
//!   `poll_join` (what `join` awaits) is a third, added after this paragraph
//!   was first written, and there may be more by the time this is read.
//!   Naming this as a *kind* rather than a roster is deliberate: this
//!   comment went stale once already when a second hand-written `PollFn`
//!   joined the first, and the fix is not a bigger number but a claim that
//!   does not depend on one -- the two *kinds* are closed, by the
//!   callee-is-a-value argument above; how many hand-written poll functions
//!   currently exist is not, and is not this comment's business to track.
//!
//! The other way to break the argument is to make a runtime function that *can*
//! unwind reachable from a poll body. `nova_rt_task_block_on` was that function:
//! it diagnosed re-entrancy with a `panic!`, and `std/task`'s `block_on` is
//! exactly what makes it reachable — `async fn f() { block_on(g()) }` compiles.
//! That diagnostic aborts instead, for this reason; see
//! `nova-runtime`'s `abort_with`.
//!
//! The state object's allocation is a call too. The outer async fn's own
//! state object is allocated in its wrapper, which is an ordinary Nova
//! function and not a poll function, and so sits outside this argument.
//!
//! **Corrected 2026-08-11: the inner future a body awaits is a different
//! allocation, and is not outside this argument.** `task_yield_future`,
//! `task_sleep_future_nanos` and `task_join_future` are called from the awaiting
//! body itself, so that call -- and the `build_future`/`gc::alloc` inside
//! it -- runs in the generated `$poll` frame that awaits the result, on
//! whichever poll first reaches it. Its own no-panic argument is not
//! restated here: it lives with `build_future`
//! (`nova-runtime/src/task.rs`), which only ever receives a
//! compile-time-constant size.
//!
//! # The layout this pass builds
//!
//! Declared here **and** in `nova-runtime/src/task.rs`: `nova-mir` must not
//! depend on `nova-runtime`, and `nova-runtime` must not depend on `nova-mir`,
//! so the two ends of this ABI are two independent declarations of one layout.
//! `nova-codegen-cranelift` depends on both and is where they are pinned
//! together (`the_state_layout_matches_nova_runtimes`).

use crate::{Block, BlockId, Function, MirTy, Module, RtFunc, Stmt, Temp, Terminator};

/// The resume tag, at byte offset 0: which of a poll function's states to
/// enter. Written by the wrapper; a resumable poll function dispatches on it.
pub const STATE_SLOT_TAG: u32 = 0;
/// The output value, at byte offset 8. Written by whichever poll call reports
/// [`POLL_READY`], and read by the executor **unconditionally** on completion.
pub const STATE_SLOT_OUTPUT: u32 = 1;
/// The first per-temp slot, at byte offset 16: original temp `i` lives at slot
/// `STATE_SLOT_TEMPS + i`.
pub const STATE_SLOT_TEMPS: u32 = 2;
/// The smallest state object this pass may emit. The tag and output slots are
/// read and written for every future, including one whose `async fn` returns
/// unit and has no temps.
pub const STATE_MIN_SIZE: i64 = STATE_SLOT_TEMPS as i64 * 8;
/// A poll function has not produced a value: it stopped at an await whose future
/// was not ready, and recorded a resume tag that returns to that same await.
pub const POLL_PENDING: i64 = 0;
/// A poll function has written its final value to [`STATE_SLOT_OUTPUT`].
pub const POLL_READY: i64 = 1;
/// Word 0 of a future value: the poll function's address.
pub const FUTURE_SLOT_POLL: u32 = 0;
/// Word 1 of a future value: the state object's address, not a record holding
/// it.
pub const FUTURE_SLOT_STATE: u32 = 1;

/// The state object pointer inside a generated poll function: its environment
/// parameter, which `takes_env` places at temp 0.
const STATE: Temp = Temp(0);
/// The task context pointer inside a generated poll function: its one real
/// parameter, which `takes_env` places at temp 1.
const TASK_CTX: Temp = Temp(1);

/// The block a resumable poll function enters at, where it switches on the
/// resume tag. MIR's entry is `BlockId(0)` by definition, so the dispatch has to
/// take that id and every block the body already had has to be renumbered past
/// it.
const DISPATCH: BlockId = BlockId(0);
/// The one block a resumable poll function sends every unrecognized resume tag
/// and every unrecognized inner poll status to. Holding no statements is what
/// lets all of them share it, and [`Terminator::Trap`] aborts rather than
/// unwinding.
const BAD_DISCRIMINANT: BlockId = BlockId(1);
/// How many blocks a resumable poll function reserves ahead of the body's own:
/// [`DISPATCH`] and [`BAD_DISCRIMINANT`].
const RESERVED_BLOCKS: u32 = 2;
/// How many blocks one await expands to, beyond the piece of its own block that
/// runs before it: the poll, the suspend, and the continuation.
const BLOCKS_PER_AWAIT: u32 = 3;
/// The lowest tag a suspend may store. Tag 0 is the entry state — what the
/// wrapper writes into a fresh state object — so the resume tags start above it.
const FIRST_RESUME_TAG: i64 = 1;

/// Rewrite every `async fn` in `module` into a poll function plus a wrapper.
///
/// Idempotent: the flag it keys on is cleared on both halves, so a second run
/// finds nothing to do. `lower_module` asserts the postcondition — no function
/// left flagged — because a missed one reaches codegen with its body's return
/// class instead of a future's.
///
/// # Preconditions
///
/// Every suspend point in a flagged function's body is a [`Stmt::Await`] marker,
/// which is what `lower::lower_expr` emits for `.await`. There is no body shape
/// this pass rejects.
pub(crate) fn transform(module: &mut Module) {
    let mut wrappers: Vec<Function> = Vec::new();
    for f in &mut module.functions {
        if !f.is_async {
            continue;
        }
        wrappers.push(split_into_poll_and_wrapper(f));
    }
    module.functions.append(&mut wrappers);
}

/// Rewrite `f` in place into its `$poll` function and return the wrapper that
/// takes over its old symbol.
fn split_into_poll_and_wrapper(f: &mut Function) -> Function {
    // The precondition the output store depends on: a returned temp's class is
    // the class this function's body produces. It is `lower::body_return_ty`
    // that makes it hold for an `async fn`, whose `ret_ty` is `Future<T>` while
    // its body has type `T` — lowering the body against the wrapped type leaves
    // `ret` claiming `Ptr` for a `Float`-producing body, and the store below
    // would then be describing a value it is not. Asserted rather than assumed
    // because the two agree by coincidence at every class except `Float`, so a
    // regression here is invisible in most fixtures.
    debug_assert!(
        f.blocks.iter().all(|b| match &b.term {
            Terminator::Return(Some(t)) => f.temps[t.0 as usize] == f.ret,
            _ => true,
        }),
        "`{}` returns a temp whose class is not its declared return class {:?}; \
         see `lower::body_return_ty`",
        f.name,
        f.ret
    );
    let wrapper = build_wrapper(f);

    // Every original temp becomes a state slot, so the temp list the rewritten
    // body is *indexed against* is the old one, while the poll function's own
    // temps start fresh: the state pointer, task_ctx, then one scratch per
    // load and per result.
    let orig_temps = std::mem::take(&mut f.temps);
    let mut sp = Spiller {
        temps: vec![MirTy::Ptr, MirTy::Ptr],
        orig: &orig_temps,
    };
    let blocks = std::mem::take(&mut f.blocks);
    f.blocks = sp.body(blocks);
    f.temps = sp.temps;

    // Every await must have been consumed by the split. A surviving marker
    // reaches a codegen backend, which has no machine code for it — reported
    // there as a malformed-MIR error, but the pass that was supposed to remove
    // it is where the mistake is.
    debug_assert!(
        !f.blocks
            .iter()
            .flat_map(|b| &b.stmts)
            .any(|s| matches!(s, Stmt::Await { .. })),
        "`{}$poll` still holds an await marker after the split",
        f.name
    );

    // **The two halves of the state object's size must agree.** The allocation
    // was sized above, from the pre-transform temp count, *before* the rewrite
    // ran; the slot indices were just chosen by `Spiller::slot`. Nothing in the
    // types connects them, so a rewrite that addresses one slot more than the
    // wrapper paid for writes past the allocation — and that has no diagnostic,
    // no verifier error, and no test that can see it, because a test asserting
    // the expected size recomputes it from the same formula the bug changed.
    //
    // This compares emitted code against emitted code: the highest slot any
    // access actually names, against the exact byte count the wrapper actually
    // allocated. Adding a slot (a per-await resume tag, an awaited future's
    // handle) therefore has to grow `state_slot_count` in the same edit.
    debug_assert!(
        highest_state_slot(&f.blocks, STATE) < wrapper.state_slots,
        "`{}$poll` addresses state slot {}, which is past the {} slots ({} \
         bytes) its wrapper allocated: a slot added to the rewrite must be \
         added to `state_slot_count` in the same edit",
        f.name,
        highest_state_slot(&f.blocks, STATE),
        wrapper.state_slots,
        wrapper.state_slots as i64 * 8,
    );
    debug_assert!(
        highest_state_slot(&wrapper.function.blocks, wrapper.state) < wrapper.state_slots,
        "`{}` seeds state slot {}, which is past the {} slots it allocated",
        f.name,
        highest_state_slot(&wrapper.function.blocks, wrapper.state),
        wrapper.state_slots,
    );

    f.name.push_str("$poll");
    f.takes_env = true;
    f.params = 1;
    f.capture_count = 0;
    f.ret = MirTy::I64;
    f.is_async = false;
    wrapper.function
}

/// The highest state-object slot index any field access in `blocks` addresses
/// through `state`.
///
/// Filtering on the record temp is what makes this a *state* slot count rather
/// than a field-index count: a rewritten body's own record accesses (an
/// `async fn` reading `self.v`, say) are `RecordField`s too, against a scratch
/// temp, with an index that belongs to the user's record and not to this layout.
/// In a poll function no body statement can name [`STATE`] by accident — the
/// rewrite replaces every body operand with a scratch temp, and those start at
/// index 2.
///
/// Returns [`STATE_SLOT_TAG`] when there is no access at all, which is in bounds
/// for every state object by [`STATE_MIN_SIZE`].
fn highest_state_slot(blocks: &[Block], state: Temp) -> u32 {
    blocks
        .iter()
        .flat_map(|b| &b.stmts)
        .filter_map(|s| match s {
            Stmt::RecordField { record, index, .. } | Stmt::SetField { record, index, .. }
                if *record == state =>
            {
                Some(*index)
            }
            _ => None,
        })
        .max()
        .unwrap_or(STATE_SLOT_TAG)
}

/// How many 8-byte slots a state object needs for a body with `n_temps` temps.
///
/// The single place this is decided. A rewrite that needs another slot grows
/// this, and the `debug_assert!`s in [`split_into_poll_and_wrapper`] fail if it
/// does not.
fn state_slot_count(n_temps: usize) -> u32 {
    STATE_SLOT_TEMPS + n_temps as u32
}

/// A wrapper, plus the two facts the state-size cross-check needs from the
/// function that built it.
struct Wrapper {
    function: Function,
    /// The slot count baked into this wrapper's allocation — carried out rather
    /// than recomputed, so the check compares against the number the emitted
    /// code actually used.
    state_slots: u32,
    /// The temp holding the state pointer, so the wrapper's own seeding stores
    /// can be told apart from its stores to the future value.
    state: Temp,
}

/// Build the function that keeps the original symbol: allocate the state,
/// seed it, and return the `{ poll_code, state }` future.
///
/// Reads `f` while it is still the pre-transform body, so it must run before
/// [`split_into_poll_and_wrapper`] mutates it.
fn build_wrapper(f: &Function) -> Wrapper {
    // The temps the ABI hands this function: the environment pointer, if it
    // has one, then the real parameters — numbered exactly as `lower_function`
    // numbered them, which is what makes seeding them into slot `i` line up
    // with what the rewritten body reads back out of slot `i`.
    let abi_count = f.params as usize + usize::from(f.takes_env);
    let mut temps: Vec<MirTy> = f.temps[..abi_count].to_vec();
    let mut stmts: Vec<Stmt> = Vec::new();

    // `state_slot_count(n_temps) * 8` bytes, through `nova_rt_alloc` — which is
    // `gc::alloc(.., true)`, i.e. SCANNED. That is load-bearing, not incidental:
    // a heap-valued output written to `STATE_SLOT_OUTPUT` is kept alive only by
    // the collector tracing through this object, and an unscanned one is marked
    // but never traced.
    //
    // Sized here, from the pre-transform temp count, before the rewrite that
    // chooses the slot indices has run. `split_into_poll_and_wrapper` checks the
    // two against each other afterwards.
    let state_slots = state_slot_count(f.temps.len());
    let bytes = state_slots as i64 * 8;
    debug_assert!(
        bytes >= STATE_MIN_SIZE,
        "a state object must hold at least the tag and output slots, since the \
         executor reads the output slot on completion even for a future with \
         no temps at all"
    );
    let size = push_temp(&mut temps, MirTy::I64);
    stmts.push(Stmt::ConstInt(size, bytes));
    let state = push_temp(&mut temps, MirTy::Ptr);
    stmts.push(Stmt::CallRuntime {
        dst: Some(state),
        func: RtFunc::Alloc,
        args: vec![size],
    });

    // The resume tag. `gc::alloc` hands back zeroed memory, so this store is
    // redundant today; it is emitted anyway so the entry tag is a property of
    // the generated code rather than of the allocator's zeroing.
    let zero = push_temp(&mut temps, MirTy::I64);
    stmts.push(Stmt::ConstInt(zero, 0));
    stmts.push(Stmt::SetField {
        record: state,
        index: STATE_SLOT_TAG,
        value: zero,
        ty: MirTy::I64,
    });

    // Seed the incoming ABI temps into their slots. An `async fn`'s arguments
    // arrive HERE, not at poll, whose only parameters are the state and
    // task_ctx — so without this the body reads the allocator's zeroes.
    // Emitted for a unit-class temp too: both backends drop a `ty: Unit`
    // field access before touching the value, so it costs one dead statement
    // rather than a special case that could disagree with the reload side.
    for (i, &ty) in temps[..abi_count].iter().enumerate() {
        stmts.push(Stmt::SetField {
            record: state,
            index: STATE_SLOT_TEMPS + i as u32,
            value: Temp(i as u32),
            ty,
        });
    }

    // The future value. `MakeClosure` is the only MIR construct that
    // materializes a function's address, and with an empty capture list it
    // allocates exactly the two-word block this ABI wants — `{ poll_code,
    // null }`, with no environment record — so word 1 is then patched to the
    // state object itself. Capturing the state instead would put a record
    // *containing* it in word 1, one indirection too many, and the runtime
    // would read the output slot out of that record.
    let future = push_temp(&mut temps, MirTy::Ptr);
    stmts.push(Stmt::MakeClosure {
        dst: future,
        code: format!("{}$poll", f.name),
        captures: Vec::new(),
    });
    stmts.push(Stmt::SetField {
        record: future,
        index: FUTURE_SLOT_STATE,
        value: state,
        ty: MirTy::Ptr,
    });

    Wrapper {
        function: Function {
            name: f.name.clone(),
            params: f.params,
            takes_env: f.takes_env,
            // The wrapper loads nothing from an environment record: it only
            // forwards the pointer into a state slot, so the body's own capture
            // loads (now inside the poll function) still find them.
            capture_count: 0,
            temps,
            ret: MirTy::Ptr,
            is_async: false,
            blocks: vec![Block {
                stmts,
                term: Terminator::Return(Some(future)),
            }],
        },
        state_slots,
        state,
    }
}

/// Append a temp of class `ty` and return its id. The temp list's length *is*
/// the next id, in both halves of this pass.
fn push_temp(temps: &mut Vec<MirTy>, ty: MirTy) -> Temp {
    let t = Temp(temps.len() as u32);
    temps.push(ty);
    t
}

/// Rewrites a pre-transform body into the poll function's: every value it names
/// moves into the state object, and every suspend point becomes the control flow
/// that polls, suspends and resumes.
///
/// The split and the spill are one pass because each needs what the other
/// consumes. The split reads an await's *original* temp ids to name the state
/// slots its future and result live in — which is exactly what the spill
/// replaces with scratch temps — and the spill has to run over the pieces the
/// split produced, since the piece a statement lands in is where its reloads
/// have to go.
struct Spiller<'a> {
    /// The poll function's temp list, grown one entry per load and per result.
    temps: Vec<MirTy>,
    /// The pre-transform temp classes, indexed by original temp id — which is
    /// also the index into the state object's temp slots.
    orig: &'a [MirTy],
}

/// One suspend point lifted out of a block's statement list.
struct AwaitPoint {
    /// Where the awaited value goes, or `None` when the awaited output is unit.
    dst: Option<Temp>,
    /// The pre-transform temp holding the awaited `{ poll_code, state }` value.
    future: Temp,
}

/// Split one pre-transform block's statements at its suspend points.
///
/// Returns the straight-line runs between the awaits — always exactly one more
/// than there are awaits, so a block whose last statement is an await still has
/// a continuation to resume into — and the awaits themselves, in order.
///
/// Lifting the awaits out here, rather than substituting through them like every
/// other statement, is what lets the sequence each one expands to address state
/// slots by the *original* temp ids: [`visit_temps`] would already have replaced
/// them with scratch temps that name no slot at all.
fn split_at_awaits(stmts: Vec<Stmt>) -> (Vec<Vec<Stmt>>, Vec<AwaitPoint>) {
    let mut segments: Vec<Vec<Stmt>> = vec![Vec::new()];
    let mut awaits: Vec<AwaitPoint> = Vec::new();
    for s in stmts {
        match s {
            Stmt::Await { dst, future } => {
                awaits.push(AwaitPoint { dst, future });
                segments.push(Vec::new());
            }
            other => segments
                .last_mut()
                .expect("segments starts with one entry and only ever grows")
                .push(other),
        }
    }
    (segments, awaits)
}

/// How many suspend points `b` contains.
fn await_count(b: &Block) -> u32 {
    b.stmts
        .iter()
        .filter(|s| matches!(s, Stmt::Await { .. }))
        .count() as u32
}

/// Whether a statement's operand is consumed or produced. A load has to be
/// emitted before the statement and a store after it, so the two cannot share
/// one code path.
#[derive(Clone, Copy)]
enum Role {
    Read,
    Write,
}

impl Spiller<'_> {
    fn scratch(&mut self, ty: MirTy) -> Temp {
        push_temp(&mut self.temps, ty)
    }

    fn orig_ty(&self, t: Temp) -> MirTy {
        self.orig[t.0 as usize]
    }

    fn slot(t: Temp) -> u32 {
        STATE_SLOT_TEMPS + t.0
    }

    /// Load field `index` of `record` into a fresh scratch temp.
    ///
    /// A fresh temp per load site, never one reused across uses: a reused
    /// scratch would be a value living outside the state object across
    /// statements, which is exactly what this pass exists to prevent — and after
    /// a split it could be a value living across a *suspend*, read on a resume
    /// where nothing defined it.
    fn load(&mut self, out: &mut Vec<Stmt>, record: Temp, index: u32, ty: MirTy) -> Temp {
        let dst = self.scratch(ty);
        out.push(Stmt::RecordField {
            dst,
            record,
            index,
            ty,
        });
        dst
    }

    /// Load original temp `t` out of its state slot into a fresh scratch temp.
    fn reload(&mut self, out: &mut Vec<Stmt>, t: Temp) -> Temp {
        let ty = self.orig_ty(t);
        self.load(out, STATE, Self::slot(t), ty)
    }

    /// A fresh scratch temp holding the i64 constant `v`.
    fn const_i64(&mut self, out: &mut Vec<Stmt>, v: i64) -> Temp {
        let t = self.scratch(MirTy::I64);
        out.push(Stmt::ConstInt(t, v));
        t
    }

    /// Rewrite a whole pre-transform body: split it at its suspend points, spill
    /// every value it names, and give a resumable body its entry dispatch.
    fn body(&mut self, orig: Vec<Block>) -> Vec<Block> {
        let per_block: Vec<u32> = orig.iter().map(await_count).collect();
        let total: u32 = per_block.iter().sum();
        // An await-free body has exactly one state, so a dispatch on its tag
        // would be the identity and moving its entry block would renumber every
        // terminator target for no gain. Reserving nothing leaves such a body's
        // block list exactly as it arrived.
        let reserved = if total == 0 { 0 } else { RESERVED_BLOCKS };

        // Where each original block's leading piece lands, decided before
        // anything is emitted: a terminator can name a block that has not been
        // rewritten yet, and a loop's back edge always does.
        let mut next = reserved;
        let starts: Vec<BlockId> = per_block
            .iter()
            .map(|a| {
                let id = BlockId(next);
                next += 1 + BLOCKS_PER_AWAIT * a;
                id
            })
            .collect();

        let mut out: Vec<Block> = Vec::with_capacity(next as usize);
        for _ in 0..reserved {
            // Placeholders. `BAD_DISCRIMINANT` is already exactly what it needs
            // to be; `DISPATCH` is overwritten below, once every resume point
            // exists to switch to.
            out.push(Block {
                stmts: Vec::new(),
                term: Terminator::Trap,
            });
        }
        let mut resume: Vec<BlockId> = Vec::with_capacity(total as usize);
        for (i, b) in orig.into_iter().enumerate() {
            debug_assert_eq!(
                out.len() as u32,
                starts[i].0,
                "block {i}'s pieces must land at the id reserved for them, or \
                 every terminator naming it is off by however far the two drifted"
            );
            self.block_pieces(b, &starts, &mut out, &mut resume);
        }
        debug_assert_eq!(
            out.len() as u32,
            next,
            "the emitted block count must match the ids reserved for it"
        );

        if total > 0 {
            debug_assert_eq!(
                resume.len(),
                total as usize,
                "one resume point per await, in tag order"
            );
            let mut stmts = Vec::new();
            let tag = self.load(&mut stmts, STATE, STATE_SLOT_TAG, MirTy::I64);
            let arms = std::iter::once((0, starts[0]))
                .chain(
                    resume
                        .into_iter()
                        .enumerate()
                        .map(|(k, b)| (FIRST_RESUME_TAG + k as i64, b)),
                )
                .collect();
            out[DISPATCH.0 as usize] = Block {
                stmts,
                term: Terminator::Switch {
                    disc: tag,
                    arms,
                    default: BAD_DISCRIMINANT,
                },
            };
        }
        out
    }

    /// Emit one original block's pieces: the statements before its first await,
    /// then a poll / suspend / continuation triple per await.
    ///
    /// Appends each await's poll block to `resume`, in tag order.
    fn block_pieces(
        &mut self,
        b: Block,
        starts: &[BlockId],
        out: &mut Vec<Block>,
        resume: &mut Vec<BlockId>,
    ) {
        let base = out.len() as u32;
        let Block {
            stmts: body,
            term: orig_term,
        } = b;
        let (segments, awaits) = split_at_awaits(body);
        let mut segments = segments.into_iter();
        let n = awaits.len();
        // Whichever piece is last takes the block's original terminator, and
        // exactly one does.
        let mut orig_term = Some(orig_term);

        // The leading piece. A resume skips it, which is the reason for cutting
        // the block here rather than re-entering it from the top.
        let mut stmts = Vec::new();
        self.spill(
            segments.next().expect("one segment more than awaits"),
            &mut stmts,
        );
        if n == 0 {
            let term = self.finish(&mut orig_term, &mut stmts, starts);
            out.push(Block { stmts, term });
            return;
        }
        out.push(Block {
            stmts,
            term: Terminator::Goto(BlockId(base + 1)),
        });

        for (k, aw) in awaits.into_iter().enumerate() {
            let poll = BlockId(base + 1 + BLOCKS_PER_AWAIT * k as u32);
            let suspend = BlockId(poll.0 + 1);
            let cont = BlockId(poll.0 + 2);
            let tag = FIRST_RESUME_TAG + resume.len() as i64;
            resume.push(poll);

            // The poll. Entered both by falling out of the code before the await
            // and by a resume at `tag`, so it holds nothing but the poll itself:
            // anything else here would run again on every re-poll.
            debug_assert_eq!(out.len() as u32, poll.0, "the poll block's own id");
            let mut stmts = Vec::new();
            let future = self.reload(&mut stmts, aw.future);
            let status = self.scratch(MirTy::I64);
            stmts.push(Stmt::CallIndirect {
                dst: Some(status),
                callee: future,
                params: vec![MirTy::Ptr],
                ret: MirTy::I64,
                args: vec![TASK_CTX],
            });
            out.push(Block {
                stmts,
                // Exactly the two statuses the ABI defines, and a trap for the
                // rest: a generated poll function returns only these, so
                // anything else is a compiler bug, and treating an unrecognized
                // status as ready would complete the await out of an output slot
                // nothing wrote.
                term: Terminator::Switch {
                    disc: status,
                    arms: vec![(POLL_READY, cont), (POLL_PENDING, suspend)],
                    default: BAD_DISCRIMINANT,
                },
            });

            // The suspend. The tag stored is THIS await's own, so the next poll
            // re-enters the block above and polls the same future again. A tag
            // one higher would resume past a suspend point that never finished,
            // and the continuation would then read an inner output slot nothing
            // has written — a wrong value on a path that still completes.
            let mut stmts = Vec::new();
            let t = self.const_i64(&mut stmts, tag);
            stmts.push(Stmt::SetField {
                record: STATE,
                index: STATE_SLOT_TAG,
                value: t,
                ty: MirTy::I64,
            });
            let pending = self.const_i64(&mut stmts, POLL_PENDING);
            out.push(Block {
                stmts,
                term: Terminator::Return(Some(pending)),
            });

            // The continuation, reached only from the READY arm above — so the
            // awaited value exists: word `FUTURE_SLOT_STATE` of the future is the
            // inner state object, and `STATE_SLOT_OUTPUT` of *that* is the value.
            // Copied into the await result's own temp slot before the statements
            // that followed the await run.
            let mut stmts = Vec::new();
            match aw.dst {
                Some(dst) if self.orig_ty(dst) != MirTy::Unit => {
                    let ty = self.orig_ty(dst);
                    let future = self.reload(&mut stmts, aw.future);
                    let inner = self.load(&mut stmts, future, FUTURE_SLOT_STATE, MirTy::Ptr);
                    let value = self.load(&mut stmts, inner, STATE_SLOT_OUTPUT, ty);
                    stmts.push(Stmt::SetField {
                        record: STATE,
                        index: Self::slot(dst),
                        value,
                        ty,
                    });
                }
                // A unit-output future has nothing to copy out: it writes an
                // explicit zero into its own output slot, and the awaiting body
                // has no temp to put it in.
                _ => {}
            }
            self.spill(
                segments.next().expect("one segment more than awaits"),
                &mut stmts,
            );
            let term = if k + 1 < n {
                // The next await's poll block, which is the very next one
                // emitted.
                Terminator::Goto(BlockId(cont.0 + 1))
            } else {
                self.finish(&mut orig_term, &mut stmts, starts)
            };
            out.push(Block { stmts, term });
        }
    }

    /// Take the original terminator and rewrite it. Panics if a second piece
    /// asks for it, which would mean two pieces of one block both claimed to be
    /// its last.
    fn finish(
        &mut self,
        orig_term: &mut Option<Terminator>,
        out: &mut Vec<Stmt>,
        starts: &[BlockId],
    ) -> Terminator {
        let term = orig_term
            .take()
            .expect("exactly one piece of a block takes its original terminator");
        self.terminator(term, out, starts)
    }

    /// Move every value one straight-line run of statements names into the state
    /// object, appending the rewritten statements to `out`.
    fn spill(&mut self, stmts: Vec<Stmt>, out: &mut Vec<Stmt>) {
        for mut s in stmts {
            let mut loads: Vec<Stmt> = Vec::new();
            let mut stores: Vec<Stmt> = Vec::new();
            visit_temps(&mut s, &mut |t, role| match role {
                Role::Read => *t = self.reload(&mut loads, *t),
                Role::Write => {
                    let ty = self.orig_ty(*t);
                    let dst = self.scratch(ty);
                    stores.push(Stmt::SetField {
                        record: STATE,
                        index: Self::slot(*t),
                        value: dst,
                        ty,
                    });
                    *t = dst;
                }
            });
            out.append(&mut loads);
            out.push(s);
            out.append(&mut stores);
        }
    }

    /// Rewrite one terminator: reload whatever it reads, renumber the blocks it
    /// names through `starts`, and turn a `Return` into a completion.
    fn terminator(
        &mut self,
        term: Terminator,
        out: &mut Vec<Stmt>,
        starts: &[BlockId],
    ) -> Terminator {
        // Every original terminator names the *start* of a block, and a split
        // block's start is its leading piece — so this is the whole renumbering.
        // A target left un-remapped lands in whatever block now carries the old
        // id, which for a resumable poll function is a reserved one.
        let at = |b: BlockId| starts[b.0 as usize];
        match term {
            Terminator::Goto(target) => Terminator::Goto(at(target)),
            Terminator::Branch { cond, then_, else_ } => Terminator::Branch {
                cond: self.reload(out, cond),
                then_: at(then_),
                else_: at(else_),
            },
            Terminator::Switch {
                disc,
                arms,
                default,
            } => Terminator::Switch {
                disc: self.reload(out, disc),
                arms: arms.into_iter().map(|(v, b)| (v, at(b))).collect(),
                default: at(default),
            },
            // Completion. The value goes to the output slot and the *return*
            // becomes the status, rather than returning the value directly:
            // the status is an i64, and an output that is not also an i64 —
            // a `Float`, the one class `mir_ty` keeps disjoint from the rest —
            // would otherwise be forced through it.
            Terminator::Return(value) => {
                match value {
                    Some(t) if self.orig_ty(t) != MirTy::Unit => {
                        let ty = self.orig_ty(t);
                        let v = self.reload(out, t);
                        out.push(Stmt::SetField {
                            record: STATE,
                            index: STATE_SLOT_OUTPUT,
                            value: v,
                            ty,
                        });
                    }
                    // A unit output has no value to store, but the executor
                    // reads the output slot on completion regardless, so an
                    // explicit zero goes in rather than leaving the slot to
                    // whatever the allocator left there.
                    _ => {
                        let z = self.const_i64(out, 0);
                        out.push(Stmt::SetField {
                            record: STATE,
                            index: STATE_SLOT_OUTPUT,
                            value: z,
                            ty: MirTy::I64,
                        });
                    }
                }
                let ready = self.const_i64(out, POLL_READY);
                Terminator::Return(Some(ready))
            }
            // Left as a trap, which aborts rather than unwinding: a panic must
            // not cross a generated poll function's boundary (see the module
            // doc comment).
            Terminator::Trap => Terminator::Trap,
        }
    }
}

/// Visit every temp `stmt` mentions, telling the callback whether it is read
/// or written, and letting it substitute a replacement.
///
/// Exhaustive by construction rather than by a `_` arm: a new [`Stmt`] variant
/// must decide here which of its temps are operands and which are results.
/// Getting that wrong is not a compile error but a miscompile — a missed read
/// leaves a statement naming a temp that no longer exists in the poll
/// function, and a read misfiled as a write drops the value it was supposed to
/// consume.
///
/// Deliberately private to this module: the read/write classification is only
/// as trustworthy as the pass that exercises it, and nothing else here needs
/// it.
fn visit_temps(stmt: &mut Stmt, f: &mut impl FnMut(&mut Temp, Role)) {
    use Role::{Read, Write};
    match stmt {
        Stmt::ConstInt(dst, _)
        | Stmt::ConstFloat(dst, _)
        | Stmt::ConstBool(dst, _)
        | Stmt::ConstStr(dst, _)
        | Stmt::ConstUnit(dst) => f(dst, Write),
        Stmt::Copy { dst, src } => {
            f(src, Read);
            f(dst, Write);
        }
        Stmt::Bin { dst, lhs, rhs, .. } => {
            f(lhs, Read);
            f(rhs, Read);
            f(dst, Write);
        }
        Stmt::Neg { dst, src, .. } | Stmt::Not { dst, src } | Stmt::BitNot { dst, src } => {
            f(src, Read);
            f(dst, Write);
        }
        Stmt::Call { dst, args, .. } | Stmt::CallRuntime { dst, args, .. } => {
            for a in args.iter_mut() {
                f(a, Read);
            }
            if let Some(dst) = dst {
                f(dst, Write);
            }
        }
        Stmt::CallIndirect {
            dst, callee, args, ..
        } => {
            f(callee, Read);
            for a in args.iter_mut() {
                f(a, Read);
            }
            if let Some(dst) = dst {
                f(dst, Write);
            }
        }
        // Not reached: [`Spiller::block_pieces`] takes every await out of the
        // statement list before spilling what is left, because the await
        // sequence it emits addresses state slots by the *original* temp ids
        // this substitution would already have replaced. Classified honestly
        // anyway rather than left to a panic, since the roles are the same ones
        // `CallIndirect` above has — an await *is* an indirect call to a poll
        // function — and this match is the one place that decides them.
        Stmt::Await { dst, future } => {
            f(future, Read);
            if let Some(dst) = dst {
                f(dst, Write);
            }
        }
        Stmt::MakeClosure { dst, captures, .. } => {
            for (c, _) in captures.iter_mut() {
                f(c, Read);
            }
            f(dst, Write);
        }
        Stmt::MakeSum { dst, fields, .. } | Stmt::MakeRecord { dst, fields } => {
            for (v, _) in fields.iter_mut() {
                f(v, Read);
            }
            f(dst, Write);
        }
        Stmt::SumTag { dst, sum } | Stmt::SumField { dst, sum, .. } => {
            f(sum, Read);
            f(dst, Write);
        }
        Stmt::RecordField { dst, record, .. } => {
            f(record, Read);
            f(dst, Write);
        }
        Stmt::SetField { record, value, .. } => {
            f(record, Read);
            f(value, Read);
        }
        Stmt::MakeArray { dst, elems } => {
            for (e, _) in elems.iter_mut() {
                f(e, Read);
            }
            f(dst, Write);
        }
        Stmt::ArrayAlloc { dst, len } => {
            f(len, Read);
            f(dst, Write);
        }
        Stmt::ArrayLen { dst, arr } => {
            f(arr, Read);
            f(dst, Write);
        }
        Stmt::ArrayGet {
            dst, arr, index, ..
        } => {
            f(arr, Read);
            f(index, Read);
            f(dst, Write);
        }
        Stmt::ArraySet {
            arr, index, value, ..
        } => {
            f(arr, Read);
            f(index, Read);
            f(value, Read);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Function, MirTy, Module, RtFunc, Stmt, Temp, Terminator};

    /// A `Module` shaped exactly the way [`crate::lower::lower_function`]
    /// leaves an await-free `async fn f(params...) -> T { <literal> }`: one
    /// function, `is_async`, whose declared `ret` is the **body's** class
    /// (`T`) rather than `Future<T>`'s `Ptr`, with a single block that returns
    /// the literal.
    ///
    /// Built by hand rather than run through the real pipeline so the input to
    /// `transform` is fixed and visible; the pipeline-level tests below
    /// (`mir_for`) cover the wiring.
    fn module_with_async_const_fn(ret: MirTy, params: &[MirTy]) -> Module {
        let mut temps: Vec<MirTy> = params.to_vec();
        let result = Temp(temps.len() as u32);
        temps.push(ret);
        let (stmts, term) = match ret {
            MirTy::I64 => (
                vec![Stmt::ConstInt(result, 7)],
                Terminator::Return(Some(result)),
            ),
            MirTy::F64 => (
                vec![Stmt::ConstFloat(result, 1.5)],
                Terminator::Return(Some(result)),
            ),
            MirTy::I8 => (
                vec![Stmt::ConstBool(result, true)],
                Terminator::Return(Some(result)),
            ),
            MirTy::Ptr => (
                vec![Stmt::ConstStr(result, "hi".to_string())],
                Terminator::Return(Some(result)),
            ),
            MirTy::Unit => (vec![Stmt::ConstUnit(result)], Terminator::Return(None)),
        };
        Module {
            functions: vec![Function {
                name: "f.15".to_string(),
                params: params.len() as u32,
                takes_env: false,
                capture_count: 0,
                temps,
                ret,
                is_async: true,
                blocks: vec![crate::Block { stmts, term }],
            }],
            externs: Vec::new(),
        }
    }

    /// `async fn f(a: T, b: T) -> T { a + b }`: two parameters read, one result
    /// written, one value returned — the smallest body with an operand of each
    /// kind, for asserting on the exact slots the rewrite touches.
    fn module_with_async_bin_fn(ret: MirTy) -> Module {
        let mut m = module_with_async_const_fn(ret, &[ret, ret]);
        let f = &mut m.functions[0];
        f.blocks[0].stmts = vec![Stmt::Bin {
            dst: Temp(2),
            op: nova_hir::BinOp::Add,
            class: crate::OperandClass::Float,
            lhs: Temp(0),
            rhs: Temp(1),
        }];
        m
    }

    /// The same shape with `is_async: false` — the negative control for
    /// "the pass rewrites only what it was asked to".
    fn module_with_plain_fn() -> Module {
        let mut m = module_with_async_const_fn(MirTy::I64, &[]);
        m.functions[0].is_async = false;
        m
    }

    fn find<'a>(m: &'a Module, name: &str) -> &'a Function {
        m.functions
            .iter()
            .find(|f| f.name == name)
            .unwrap_or_else(|| {
                panic!(
                    "no function named `{name}`; have {:?}",
                    m.functions.iter().map(|f| &f.name).collect::<Vec<_>>()
                )
            })
    }

    fn stmts_of(f: &Function) -> impl Iterator<Item = &Stmt> {
        f.blocks.iter().flat_map(|b| &b.stmts)
    }

    #[test]
    fn an_await_free_async_fn_becomes_a_poll_fn_and_a_wrapper() {
        let mut m = module_with_async_const_fn(MirTy::I64, &[]);
        transform(&mut m);

        let poll = find(&m, "f.15$poll");
        assert!(poll.takes_env, "the poll fn's env IS the state object");
        assert_eq!(poll.params, 1, "poll takes task_ctx as its one real param");
        assert_eq!(poll.ret, MirTy::I64, "poll returns a status, not the value");
        // The ABI the runtime declares is `(state, task_ctx) -> i64`, and both
        // backends build the signature from `temps[..params + takes_env]`. So
        // the first two temps must be pointer-class, or the generated
        // signature is not `PollFn`'s.
        assert_eq!(
            &poll.temps[..2],
            &[MirTy::Ptr, MirTy::Ptr],
            "temp 0 is the state pointer, temp 1 is task_ctx"
        );

        let wrapper = find(&m, "f.15");
        assert_eq!(wrapper.ret, MirTy::Ptr, "the wrapper returns a future");
        assert!(
            !wrapper.takes_env,
            "the wrapper keeps the original ABI its callers already compiled against"
        );
        assert!(
            stmts_of(wrapper).any(|s| matches!(s, Stmt::MakeClosure { .. })),
            "the wrapper must build the {{ poll_code, state }} fat pointer"
        );
    }

    #[test]
    fn the_poll_fn_writes_its_result_to_the_output_slot_not_the_return() {
        // Instantiated at `Float`, not `Int`: `mir_ty` collapses `Int` and
        // every pointer-like type onto the same 64-bit integer class, so an
        // `Int` fixture cannot tell "stored the value" from "returned the
        // value" -- both are an i64 in a general-purpose register. `F64` is
        // the one class that crosses register banks, so it is the only one
        // where returning the value directly through the i64 status is a
        // visible type confusion rather than an invisible one.
        let mut m = module_with_async_const_fn(MirTy::F64, &[]);
        transform(&mut m);
        let poll = find(&m, "f.15$poll");
        assert!(
            stmts_of(poll).any(|s| matches!(
                s,
                Stmt::SetField { index, ty, .. }
                    if *index == STATE_SLOT_OUTPUT && *ty == MirTy::F64
            )),
            "the f64 result must be stored to the output slot AS an f64: {:?}",
            poll.blocks
        );
        assert_eq!(
            poll.ret,
            MirTy::I64,
            "the poll fn's own return stays the i64 status"
        );
    }

    #[test]
    fn the_poll_fn_returns_the_ready_status_and_nothing_else() {
        // Task 4's executor panics on any status that is neither
        // POLL_PENDING nor POLL_READY, naming the value. An await-free body
        // never suspends, so every exit must be exactly POLL_READY -- and the
        // returned temp has to be the one a `ConstInt` set to it, not
        // whatever the body last computed.
        let mut m = module_with_async_const_fn(MirTy::F64, &[]);
        transform(&mut m);
        let poll = find(&m, "f.15$poll");
        let returned: Vec<Temp> = poll
            .blocks
            .iter()
            .filter_map(|b| match &b.term {
                Terminator::Return(Some(t)) => Some(*t),
                _ => None,
            })
            .collect();
        assert!(!returned.is_empty(), "the poll fn must return a status");
        for t in returned {
            assert_eq!(
                poll.temps[t.0 as usize],
                MirTy::I64,
                "a poll status is an i64"
            );
            assert!(
                stmts_of(poll)
                    .any(|s| matches!(s, Stmt::ConstInt(d, v) if *d == t && *v == POLL_READY)),
                "the returned temp %{} must be a constant POLL_READY, not a body value: {:?}",
                t.0,
                poll.blocks
            );
        }
    }

    #[test]
    fn a_unit_returning_async_fn_still_writes_the_output_slot() {
        // Task 4's `poll_one` reads STATE_SLOT_OUTPUT *unconditionally* on
        // completion, including for a future whose `async fn` returns unit and
        // whose body never touches that slot. Leaving the slot untouched would
        // work only by accident (the allocator zeroes), so the store is
        // emitted explicitly.
        let mut m = module_with_async_const_fn(MirTy::Unit, &[]);
        transform(&mut m);
        let poll = find(&m, "f.15$poll");
        assert!(
            stmts_of(poll).any(|s| matches!(
                s,
                Stmt::SetField { index, ty, .. }
                    if *index == STATE_SLOT_OUTPUT && *ty != MirTy::Unit
            )),
            "a unit-returning poll fn must still write a real value to the \
             output slot (a `ty: Unit` store is a codegen no-op): {:?}",
            poll.blocks
        );
    }

    #[test]
    fn the_poll_fn_reloads_every_operand_and_stores_every_result_to_its_own_slot() {
        // The exact slot indices, in order, for a body that is one binary
        // operation on two parameters -- which is the smallest shape with both
        // operand kinds and a result. Two classes of defect this discriminates
        // and nothing else here does:
        //
        // - The temp slots starting anywhere other than `STATE_SLOT_TEMPS`.
        //   The wrapper computes its seeding index from that constant directly
        //   while the body's reloads go through `Spiller::slot`, so the two can
        //   disagree -- and then a parameter is written to one slot and read
        //   from another, with `SetField`/`RecordField` staying perfectly
        //   consistent about the offsets they each use.
        // - An operand misfiled as a result in `visit_temps`. That drops the
        //   reload of the value the statement was supposed to consume and adds
        //   a store of an undefined one, which is visible here as a missing
        //   index rather than as the wrong answer at run time.
        let mut m = module_with_async_bin_fn(MirTy::F64);
        transform(&mut m);
        let poll = find(&m, "f.15$poll");
        let loads: Vec<u32> = stmts_of(poll)
            .filter_map(|s| match s {
                Stmt::RecordField { index, .. } => Some(*index),
                _ => None,
            })
            .collect();
        let stores: Vec<u32> = stmts_of(poll)
            .filter_map(|s| match s {
                Stmt::SetField { index, .. } => Some(*index),
                _ => None,
            })
            .collect();
        let temps = STATE_SLOT_TEMPS;
        assert_eq!(
            loads,
            vec![temps, temps + 1, temps + 2],
            "both operands, then the returned value, each reloaded from its \
             own slot: {:?}",
            poll.blocks
        );
        assert_eq!(
            stores,
            vec![temps + 2, STATE_SLOT_OUTPUT],
            "the result to its own slot, then the output slot on completion: \
             {:?}",
            poll.blocks
        );
    }

    #[test]
    fn every_body_value_is_addressed_through_the_state_object() {
        // The property a resumable body depends on: no body value lives in a register
        // across a block boundary, because every one of them is a slot in the
        // state object. Asserted as "every field access in the poll fn is
        // against temp 0", which is the state pointer -- a body value left in
        // a temp would show up as a `RecordField`/`SetField` against some
        // other record, or as no access at all.
        let mut m = module_with_async_const_fn(MirTy::F64, &[MirTy::I64]);
        transform(&mut m);
        let poll = find(&m, "f.15$poll");
        let mut accesses = 0;
        for s in stmts_of(poll) {
            match s {
                Stmt::RecordField { record, .. } | Stmt::SetField { record, .. } => {
                    accesses += 1;
                    assert_eq!(
                        *record,
                        Temp(0),
                        "every state access must go through temp 0, the state pointer"
                    );
                }
                _ => {}
            }
        }
        assert!(accesses > 0, "the poll fn must touch the state object");
    }

    #[test]
    fn the_wrapper_seeds_the_state_with_its_parameters() {
        // An async fn's arguments arrive at the WRAPPER, not at poll -- poll's
        // only parameters are the state and task_ctx. If the wrapper does not
        // copy them into their temp slots, the body reads whatever the
        // allocator left there (zero), and `async fn add(a, b) { a + b }`
        // silently returns 0.
        let mut m = module_with_async_const_fn(MirTy::I64, &[MirTy::F64, MirTy::I64]);
        transform(&mut m);
        let wrapper = find(&m, "f.15");
        for (i, ty) in [(0u32, MirTy::F64), (1, MirTy::I64)] {
            assert!(
                stmts_of(wrapper).any(|s| matches!(
                    s,
                    Stmt::SetField { index, value, ty: t, .. }
                        if *index == STATE_SLOT_TEMPS + i && *value == Temp(i) && *t == ty
                )),
                "parameter %{i} must be stored into state slot {} as {ty:?}: {:?}",
                STATE_SLOT_TEMPS + i,
                wrapper.blocks
            );
        }
    }

    #[test]
    fn the_wrapper_allocates_a_scanned_state_no_smaller_than_the_minimum() {
        // Two Task 4 constraints in one place, because one allocation call
        // carries both:
        //
        // - `RtFunc::Alloc` is `nova_rt_alloc`, which is `gc::alloc(.., true)`
        //   -- SCANNED. An unscanned state object is marked but never traced
        //   (`gc.rs`'s `if !scan { continue; }`), so a heap-valued output in
        //   its output slot would be freed while the executor still names it.
        // - The size must never be below `STATE_MIN_SIZE`, because
        //   STATE_SLOT_OUTPUT is read on completion even for a future with no
        //   temps at all.
        let mut m = module_with_async_const_fn(MirTy::Unit, &[]);
        let n_temps = m.functions[0].temps.len();
        transform(&mut m);
        let wrapper = find(&m, "f.15");
        let expected = ((STATE_SLOT_TEMPS as usize + n_temps) * 8) as i64;
        assert!(expected >= STATE_MIN_SIZE);
        let sizes: Vec<i64> = stmts_of(wrapper)
            .filter_map(|s| match s {
                Stmt::CallRuntime {
                    func: RtFunc::Alloc,
                    args,
                    ..
                } => Some(args.clone()),
                _ => None,
            })
            .flatten()
            .filter_map(|t| {
                stmts_of(wrapper).find_map(|s| match s {
                    Stmt::ConstInt(d, v) if *d == t => Some(*v),
                    _ => None,
                })
            })
            .collect();
        assert_eq!(
            sizes,
            vec![expected],
            "the state object must be one `nova_rt_alloc` of \
             (STATE_SLOT_TEMPS + n_temps) * 8 bytes: {:?}",
            wrapper.blocks
        );
    }

    #[test]
    fn every_return_becomes_its_own_completion_and_a_trap_stays_a_trap() {
        // Two terminator shapes an `async fn` body reaches easily -- an early
        // `return` gives two `Return`s, and a diverging or exhaustive-match arm
        // gives a `Trap` -- and neither is exercised by a single-block fixture.
        //
        // Both need pinning for opposite reasons. Every `Return` must get its
        // OWN output store and status: rewriting only the first would leave the
        // second returning whatever the body last computed, which the executor
        // would then reject as an out-of-range status at run time. A `Trap` must
        // NOT be turned into a completion: it aborts, and rewriting it into
        // `POLL_READY` would complete the task with an unwritten output slot.
        let mut m = module_with_async_const_fn(MirTy::F64, &[]);
        {
            let f = &mut m.functions[0];
            let second = Temp(f.temps.len() as u32);
            f.temps.push(MirTy::F64);
            f.blocks.push(Block {
                stmts: vec![Stmt::ConstFloat(second, 2.5)],
                term: Terminator::Return(Some(second)),
            });
            f.blocks.push(Block {
                stmts: Vec::new(),
                term: Terminator::Trap,
            });
        }
        transform(&mut m);
        let poll = find(&m, "f.15$poll");

        let statuses: Vec<Temp> = poll
            .blocks
            .iter()
            .filter_map(|b| match &b.term {
                Terminator::Return(Some(t)) => Some(*t),
                _ => None,
            })
            .collect();
        assert_eq!(
            statuses.len(),
            2,
            "both returns must survive: {:?}",
            poll.blocks
        );
        for t in &statuses {
            assert!(
                stmts_of(poll)
                    .any(|s| matches!(s, Stmt::ConstInt(d, v) if d == t && *v == POLL_READY)),
                "each return must carry its own POLL_READY constant: {:?}",
                poll.blocks
            );
        }
        let output_stores = stmts_of(poll)
            .filter(|s| matches!(s, Stmt::SetField { index, .. } if *index == STATE_SLOT_OUTPUT))
            .count();
        assert_eq!(
            output_stores, 2,
            "each completion writes the output slot on its own path, since only \
             one of them runs: {:?}",
            poll.blocks
        );
        assert_eq!(
            poll.blocks
                .iter()
                .filter(|b| matches!(b.term, Terminator::Trap))
                .count(),
            1,
            "the trap must stay a trap, not become a completion: {:?}",
            poll.blocks
        );
    }

    #[test]
    fn the_wrapper_writes_the_entry_resume_tag() {
        // The invariant is that a fresh future starts in its first state. This
        // fixture is await-free, so its poll function never reads the tag, and
        // `gc::alloc` hands back zeroed memory -- nothing about *behaviour*
        // depends on this store here, which is exactly why it needs an assertion
        // rather than a test of an observable effect. It is the only thing making
        // the entry tag a property of the generated code instead of the
        // allocator's zeroing.
        let mut m = module_with_async_const_fn(MirTy::I64, &[]);
        transform(&mut m);
        let wrapper = find(&m, "f.15");
        let tag_value = stmts_of(wrapper)
            .find_map(|s| match s {
                Stmt::SetField {
                    index, value, ty, ..
                } if *index == STATE_SLOT_TAG => {
                    assert_ne!(*ty, MirTy::Unit, "a `ty: Unit` store is a no-op");
                    Some(*value)
                }
                _ => None,
            })
            .unwrap_or_else(|| panic!("no store to the tag slot: {:?}", wrapper.blocks));
        assert!(
            stmts_of(wrapper).any(|s| matches!(s, Stmt::ConstInt(d, 0) if *d == tag_value)),
            "the entry tag must be the constant 0, so a fresh future starts in \
             its first state: {:?}",
            wrapper.blocks
        );
    }

    #[test]
    fn the_future_carries_the_state_itself_not_a_record_holding_it() {
        // The ABI's word 1 IS the state object's address. `MakeClosure` with a
        // capture would allocate an environment RECORD containing the state
        // and put *that* in word 1 -- one indirection too many, which the
        // runtime would then transmute as a poll function's state and read the
        // output slot out of. So the captures list must be empty and word 1
        // patched to the state directly.
        let mut m = module_with_async_const_fn(MirTy::I64, &[]);
        transform(&mut m);
        let wrapper = find(&m, "f.15");
        let (fut, code) = stmts_of(wrapper)
            .find_map(|s| match s {
                Stmt::MakeClosure {
                    dst,
                    code,
                    captures,
                } => {
                    assert!(
                        captures.is_empty(),
                        "a captured state would sit behind an extra indirection"
                    );
                    Some((*dst, code.clone()))
                }
                _ => None,
            })
            .expect("the wrapper builds a fat pointer");
        assert_eq!(code, "f.15$poll", "word 0 must be the poll fn's address");
        let state = stmts_of(wrapper)
            .find_map(|s| match s {
                Stmt::CallRuntime {
                    dst: Some(d),
                    func: RtFunc::Alloc,
                    ..
                } => Some(*d),
                _ => None,
            })
            .expect("the wrapper allocates a state");
        assert!(
            stmts_of(wrapper).any(|s| matches!(
                s,
                Stmt::SetField { record, index, value, .. }
                    if *record == fut && *index == FUTURE_SLOT_STATE && *value == state
            )),
            "word {FUTURE_SLOT_STATE} of the future must be the state pointer \
             itself: {:?}",
            wrapper.blocks
        );
    }

    #[test]
    fn a_non_async_function_is_left_byte_identical() {
        // The pass must be a no-op for everything else. Without this, a greedy
        // pass that rewrote every function would still pass the tests above.
        let mut m = module_with_plain_fn();
        let before = format!("{:?}", m.functions);
        transform(&mut m);
        assert_eq!(before, format!("{:?}", m.functions));
    }

    #[test]
    fn the_transform_is_idempotent() {
        // Running the pass twice must not produce `f.15$poll$poll`. It clears
        // `is_async` on what it rewrites, which is also the invariant
        // `lower_module` asserts before handing the module to codegen: a
        // function still flagged async has a return class that came from its
        // BODY, not from `Future<T>`, and would miscompile.
        let mut m = module_with_async_const_fn(MirTy::F64, &[]);
        transform(&mut m);
        let once = format!("{:?}", m.functions);
        transform(&mut m);
        assert_eq!(once, format!("{:?}", m.functions));
        assert!(
            m.functions.iter().all(|f| !f.is_async),
            "no function may still be flagged async after the pass"
        );
    }

    #[test]
    fn the_state_layout_constants_match_the_runtime() {
        // `nova-mir` must not depend on `nova-runtime`, so the layout is
        // declared twice. This pins THIS copy to its literal values; the pin
        // that actually compares the two copies lives in
        // `nova-codegen-cranelift`, the one crate that depends on both.
        assert_eq!(STATE_SLOT_TAG, 0);
        assert_eq!(STATE_SLOT_OUTPUT, 1);
        assert_eq!(STATE_SLOT_TEMPS, 2);
        assert_eq!(STATE_MIN_SIZE, 16);
        assert_eq!(POLL_PENDING, 0);
        assert_eq!(POLL_READY, 1);
        assert_eq!(FUTURE_SLOT_POLL, 0);
        assert_eq!(FUTURE_SLOT_STATE, 1);
    }

    // === splitting at await points ===

    /// A one-function `Module` holding an `async fn` with the given declared
    /// body class, parameter count, temps and blocks — the shape
    /// [`crate::lower::lower_function`] leaves before `transform` runs, for a
    /// body built by hand here rather than compiled from source.
    ///
    /// Hand-built for the same reason [`module_with_async_const_fn`] is: the
    /// input to `transform` stays fixed and visible, and a block layout with a
    /// back edge into a block that gets split is expressible directly.
    /// `tests/lower_tests.rs` covers the same shapes from real source.
    fn async_module(ret: MirTy, params: u32, temps: Vec<MirTy>, blocks: Vec<Block>) -> Module {
        Module {
            functions: vec![Function {
                name: "f.15".to_string(),
                params,
                takes_env: false,
                capture_count: 0,
                temps,
                ret,
                is_async: true,
                blocks,
            }],
            externs: Vec::new(),
        }
    }

    /// `async fn f() -> T { g().await }`: one suspend point, in the entry
    /// block, whose result is what the body returns. Temp 0 holds the future
    /// `g()` produced, temp 1 the awaited value.
    fn module_with_async_fn_awaiting_once(out: MirTy) -> Module {
        async_module(
            out,
            0,
            vec![MirTy::Ptr, out],
            vec![Block {
                stmts: vec![
                    Stmt::Call {
                        dst: Some(Temp(0)),
                        callee: "g.16".to_string(),
                        args: Vec::new(),
                    },
                    Stmt::Await {
                        dst: Some(Temp(1)),
                        future: Temp(0),
                    },
                ],
                term: Terminator::Return(Some(Temp(1))),
            }],
        )
    }

    /// `async fn f() -> T { g().await + h().await }`: two suspend points in one
    /// block, so the second await's own leading piece is a piece of a piece —
    /// and the two tags can be told apart.
    fn module_with_async_fn_awaiting_twice(out: MirTy) -> Module {
        async_module(
            out,
            0,
            vec![MirTy::Ptr, out, MirTy::Ptr, out, out],
            vec![Block {
                stmts: vec![
                    Stmt::Call {
                        dst: Some(Temp(0)),
                        callee: "g.16".to_string(),
                        args: Vec::new(),
                    },
                    Stmt::Await {
                        dst: Some(Temp(1)),
                        future: Temp(0),
                    },
                    Stmt::Call {
                        dst: Some(Temp(2)),
                        callee: "h.17".to_string(),
                        args: Vec::new(),
                    },
                    Stmt::Await {
                        dst: Some(Temp(3)),
                        future: Temp(2),
                    },
                    Stmt::Bin {
                        dst: Temp(4),
                        op: nova_hir::BinOp::Add,
                        class: crate::OperandClass::Float,
                        lhs: Temp(1),
                        rhs: Temp(3),
                    },
                ],
                term: Terminator::Return(Some(Temp(4))),
            }],
        )
    }

    /// `async fn f() -> T { while c { v = g().await }\n v }`: the suspend point
    /// is inside a loop body, so one tag is resumed at repeatedly and the loop's
    /// back edge targets a block the split renumbered.
    fn module_with_async_fn_awaiting_in_a_loop(out: MirTy) -> Module {
        async_module(
            out,
            0,
            vec![MirTy::I8, MirTy::Ptr, out],
            vec![
                // 0: entry — falls into the header.
                Block {
                    stmts: Vec::new(),
                    term: Terminator::Goto(BlockId(1)),
                },
                // 1: header — re-tests the condition on every iteration.
                Block {
                    stmts: vec![Stmt::ConstBool(Temp(0), true)],
                    term: Terminator::Branch {
                        cond: Temp(0),
                        then_: BlockId(2),
                        else_: BlockId(3),
                    },
                },
                // 2: body — the await, then the back edge.
                Block {
                    stmts: vec![
                        Stmt::Call {
                            dst: Some(Temp(1)),
                            callee: "g.16".to_string(),
                            args: Vec::new(),
                        },
                        Stmt::Await {
                            dst: Some(Temp(2)),
                            future: Temp(1),
                        },
                    ],
                    term: Terminator::Goto(BlockId(1)),
                },
                // 3: exit.
                Block {
                    stmts: Vec::new(),
                    term: Terminator::Return(Some(Temp(2))),
                },
            ],
        )
    }

    /// `async fn f() -> T { match c { A => g().await, B => v } }`: the block
    /// holding the await is reached through a `Switch`, which is the only
    /// terminator shape whose targets live in a `Vec` rather than in named
    /// fields — so it is the one the renumbering can miss on its own.
    ///
    /// The `B` arm leaves temp 2 undefined, which on that path would read the
    /// allocator's zero. Never taken: the scrutinee is the constant 0. This
    /// fixture is about which blocks the terminators name, not about the value.
    fn module_with_async_fn_awaiting_in_a_match(out: MirTy) -> Module {
        async_module(
            out,
            0,
            vec![MirTy::I64, MirTy::Ptr, out],
            vec![
                // 0: the scrutinee and the dispatch over the arms.
                Block {
                    stmts: vec![Stmt::ConstInt(Temp(0), 0)],
                    term: Terminator::Switch {
                        disc: Temp(0),
                        arms: vec![(0, BlockId(1)), (1, BlockId(2))],
                        default: BlockId(3),
                    },
                },
                // 1: the `A` arm — awaits, then joins.
                Block {
                    stmts: vec![
                        Stmt::Call {
                            dst: Some(Temp(1)),
                            callee: "g.16".to_string(),
                            args: Vec::new(),
                        },
                        Stmt::Await {
                            dst: Some(Temp(2)),
                            future: Temp(1),
                        },
                    ],
                    term: Terminator::Goto(BlockId(4)),
                },
                // 2: the `B` arm.
                Block {
                    stmts: Vec::new(),
                    term: Terminator::Goto(BlockId(4)),
                },
                // 3: the exhaustive-match default.
                Block {
                    stmts: Vec::new(),
                    term: Terminator::Trap,
                },
                // 4: the join.
                Block {
                    stmts: Vec::new(),
                    term: Terminator::Return(Some(Temp(2))),
                },
            ],
        )
    }

    /// `async fn f() { g().await }` with `g` returning unit: an await with no
    /// value to copy out of the inner future's output slot.
    fn module_with_async_fn_awaiting_unit() -> Module {
        async_module(
            MirTy::Unit,
            0,
            vec![MirTy::Ptr],
            vec![Block {
                stmts: vec![
                    Stmt::Call {
                        dst: Some(Temp(0)),
                        callee: "g.16".to_string(),
                        args: Vec::new(),
                    },
                    Stmt::Await {
                        dst: None,
                        future: Temp(0),
                    },
                ],
                term: Terminator::Return(None),
            }],
        )
    }

    /// The arms of a poll function's entry dispatch, panicking if the entry
    /// block is not one.
    fn resume_arms(poll: &Function) -> Vec<(i64, BlockId)> {
        match &poll.blocks[0].term {
            Terminator::Switch { arms, .. } => arms.clone(),
            other => panic!("the entry block must dispatch on the resume tag, found {other:?}"),
        }
    }

    /// The block a resume point branches to for inner-poll status `status`.
    fn status_arm(poll: &Function, resume: BlockId, status: i64) -> BlockId {
        match &poll.blocks[resume.0 as usize].term {
            Terminator::Switch { arms, .. } => *arms
                .iter()
                .find_map(|(v, b)| (*v == status).then_some(b))
                .unwrap_or_else(|| {
                    panic!("resume block {resume:?} has no arm for status {status}: {arms:?}")
                }),
            other => {
                panic!("a resume point must branch on the inner poll's status, found {other:?}")
            }
        }
    }

    /// Every constant `block` stores into the resume-tag slot, resolved through
    /// the `ConstInt` that produced it.
    fn tag_stores(poll: &Function, block: BlockId) -> Vec<i64> {
        let b = &poll.blocks[block.0 as usize];
        b.stmts
            .iter()
            .filter_map(|s| match s {
                Stmt::SetField {
                    record,
                    index,
                    value,
                    ..
                } if *record == STATE && *index == STATE_SLOT_TAG => Some(*value),
                _ => None,
            })
            .map(|v| {
                b.stmts
                    .iter()
                    .find_map(|s| match s {
                        Stmt::ConstInt(d, k) if *d == v => Some(*k),
                        _ => None,
                    })
                    .unwrap_or_else(|| {
                        panic!(
                            "the tag stored must be a constant defined in the same block: {:?}",
                            b.stmts
                        )
                    })
            })
            .collect()
    }

    /// Every block id any terminator in `f` names.
    fn targets(f: &Function) -> Vec<BlockId> {
        f.blocks
            .iter()
            .flat_map(|b| match &b.term {
                Terminator::Goto(t) => vec![*t],
                Terminator::Branch { then_, else_, .. } => vec![*then_, *else_],
                Terminator::Switch { arms, default, .. } => arms
                    .iter()
                    .map(|(_, b)| *b)
                    .chain(std::iter::once(*default))
                    .collect(),
                Terminator::Return(_) | Terminator::Trap => Vec::new(),
            })
            .collect()
    }

    #[test]
    fn one_await_produces_two_resume_states_dispatched_by_a_switch() {
        let mut m = module_with_async_fn_awaiting_once(MirTy::F64);
        transform(&mut m);
        let poll = find(&m, "f.15$poll");
        let arms = resume_arms(poll);
        assert!(
            arms.len() >= 2,
            "one await means two resume states, got {arms:?}"
        );
        // The tags must be DISTINCT. A switch whose arms all target one block is
        // the mutation this test exists to kill.
        let distinct: std::collections::HashSet<_> = arms.iter().map(|(_, b)| *b).collect();
        assert_eq!(
            distinct.len(),
            arms.len(),
            "resume arms must target distinct blocks: {arms:?}"
        );
    }

    #[test]
    fn a_suspend_stores_a_resume_tag_before_returning_pending() {
        // Without the store, the task resumes at state 0 forever -- an infinite
        // loop, not a wrong value, which is why an output-only assertion misses
        // it.
        let mut m = module_with_async_fn_awaiting_once(MirTy::F64);
        transform(&mut m);
        let poll = find(&m, "f.15$poll");
        let stores_tag = poll.blocks.iter().any(|b| {
            b.stmts.iter().any(|s| {
                matches!(s, Stmt::SetField { record, index, .. }
                    if *record == STATE && *index == STATE_SLOT_TAG)
            })
        });
        assert!(stores_tag, "a suspend must record a resume tag");
    }

    #[test]
    fn a_pending_inner_future_suspends_at_the_tag_that_returns_to_the_same_await() {
        // **The same-tag rule.** When the inner future reports PENDING, the tag
        // stored must be the one whose resume arm comes BACK to this await --
        // not the one belonging to the await after it. Storing `tag + 1` resumes
        // the task past a suspend point it never finished: the continuation then
        // reads an output slot the inner future has not written, which is a
        // wrong value on a path that still completes normally, so neither the
        // poll statuses nor the block count nor the arm count can see it.
        //
        // Two awaits, so "this await's tag" and "the next await's tag" are
        // different numbers. With one await, storing `tag + 1` would land on the
        // switch's default and trap, which is a different (and louder) failure.
        let mut m = module_with_async_fn_awaiting_twice(MirTy::F64);
        transform(&mut m);
        let poll = find(&m, "f.15$poll");
        let resumes: Vec<(i64, BlockId)> = resume_arms(poll)
            .into_iter()
            .filter(|(tag, _)| *tag != 0)
            .collect();
        assert_eq!(resumes.len(), 2, "two awaits, two resume tags: {resumes:?}");
        for (tag, resume) in resumes {
            let pending = status_arm(poll, resume, POLL_PENDING);
            assert_eq!(
                tag_stores(poll, pending),
                vec![tag],
                "the suspend reached from tag {tag}'s resume point must store \
                 {tag} again, so the next poll retries the same await: {:?}",
                poll.blocks
            );
            // And it must report PENDING. Returning READY here would complete the
            // task out of an output slot the awaited future has not written, and
            // the tag assertion above cannot see that: the store and the status
            // are independent. Checked against the constant rather than against
            // "not READY", since the executor accepts only the two.
            let block = &poll.blocks[pending.0 as usize];
            let status = match &block.term {
                Terminator::Return(Some(t)) => *t,
                other => panic!("a suspend must return a status, found {other:?}"),
            };
            assert!(
                block.stmts.iter().any(|s| matches!(s, Stmt::ConstInt(d, v)
                        if *d == status && *v == POLL_PENDING)),
                "the suspend must return a constant POLL_PENDING defined in its \
                 own block: {block:?}"
            );
        }
    }

    #[test]
    fn the_resume_dispatch_switches_on_the_tag_slot_and_enters_the_body_at_tag_zero() {
        // Three things a resume dispatch can get wrong independently of each
        // other, all of them silent:
        //
        // - switching on something other than the tag slot (the output slot is
        //   the adjacent index and is also an i64);
        // - not reserving tag 0 for the body's own entry, which would make a
        //   fresh future -- whose tag the wrapper sets to 0 -- resume at an
        //   await whose future does not exist yet;
        // - collapsing the later tags onto one block.
        let mut m = module_with_async_fn_awaiting_twice(MirTy::F64);
        transform(&mut m);
        let poll = find(&m, "f.15$poll");
        let entry = &poll.blocks[0];
        let disc = match &entry.term {
            Terminator::Switch { disc, .. } => *disc,
            other => panic!("the entry must dispatch on the resume tag, found {other:?}"),
        };
        assert!(
            entry.stmts.iter().any(|s| matches!(
                s,
                Stmt::RecordField { dst, record, index, ty }
                    if *dst == disc && *record == STATE && *index == STATE_SLOT_TAG
                        && *ty == MirTy::I64
            )),
            "the discriminant must be loaded from the tag slot: {:?}",
            entry
        );
        let arms = resume_arms(poll);
        assert_eq!(
            arms.iter().map(|(t, _)| *t).collect::<Vec<_>>(),
            vec![0, 1, 2],
            "one arm for the entry state plus one per await, in order: {arms:?}"
        );
        let distinct: std::collections::HashSet<_> = arms.iter().map(|(_, b)| *b).collect();
        assert_eq!(distinct.len(), 3, "every state is its own block: {arms:?}");
    }

    #[test]
    fn awaiting_polls_the_inner_future_through_its_own_fat_pointer() {
        // A future value *is* a `{ poll_code, state }` fat pointer, and
        // `CallIndirect` already means `code_ptr(env_ptr, args...)` -- so an
        // indirect call through the awaited future is a `PollFn` call, with the
        // inner state object arriving as the env. The signature has to be
        // exactly `(Ptr, Ptr) -> I64`: `params: [Ptr]` is task_ctx, the leading
        // env is implicit, and `ret: I64` is the status. Forwarding this
        // function's own task_ctx (temp 1) rather than a fresh null keeps the
        // parameter meaningful once anything uses it.
        let mut m = module_with_async_fn_awaiting_once(MirTy::F64);
        transform(&mut m);
        let poll = find(&m, "f.15$poll");
        let (callee, block) = poll
            .blocks
            .iter()
            .find_map(|b| {
                b.stmts.iter().find_map(|s| match s {
                    Stmt::CallIndirect {
                        dst: Some(_),
                        callee,
                        params,
                        ret,
                        args,
                    } => {
                        assert_eq!(params, &[MirTy::Ptr], "the one real param is task_ctx");
                        assert_eq!(*ret, MirTy::I64, "an inner poll returns a status");
                        assert_eq!(args, &[TASK_CTX], "task_ctx must be forwarded");
                        Some((*callee, b))
                    }
                    _ => None,
                })
            })
            .unwrap_or_else(|| panic!("the await must poll the inner future: {:?}", poll.blocks));
        assert!(
            block.stmts.iter().any(|s| matches!(
                s,
                Stmt::RecordField { dst, record, index, ty }
                    if *dst == callee && *record == STATE
                        && *index == STATE_SLOT_TEMPS && *ty == MirTy::Ptr
            )),
            "the awaited future must be reloaded from its own temp slot in the \
             same block as the call: {:?}",
            block
        );
    }

    #[test]
    fn an_awaited_value_is_read_from_the_inner_futures_own_output_slot() {
        // The chain that has to be exactly right, at `Float` because it is the
        // one class that crosses register banks: word `FUTURE_SLOT_STATE` of the
        // awaited future is the inner state object, and `STATE_SLOT_OUTPUT` of
        // THAT object is the awaited value. Reading the future's word 1 *as* the
        // value, or reading this function's own output slot, are both mistakes
        // an `Int` fixture cannot distinguish, since every step is an i64-shaped
        // load of an 8-byte slot.
        let mut m = module_with_async_fn_awaiting_once(MirTy::F64);
        transform(&mut m);
        let poll = find(&m, "f.15$poll");
        let inner = stmts_of(poll)
            .find_map(|s| match s {
                Stmt::RecordField {
                    dst,
                    record,
                    index,
                    ty,
                } if *index == FUTURE_SLOT_STATE && *record != STATE => {
                    assert_eq!(*ty, MirTy::Ptr, "the inner state is a pointer");
                    Some(*dst)
                }
                _ => None,
            })
            .unwrap_or_else(|| {
                panic!(
                    "the await must load the inner state out of the future: {:?}",
                    poll.blocks
                )
            });
        let value = stmts_of(poll)
            .find_map(|s| match s {
                Stmt::RecordField {
                    dst,
                    record,
                    index,
                    ty,
                } if *record == inner && *index == STATE_SLOT_OUTPUT => {
                    assert_eq!(*ty, MirTy::F64, "the awaited value keeps its own class");
                    Some(*dst)
                }
                _ => None,
            })
            .unwrap_or_else(|| {
                panic!(
                    "the awaited value must come from the inner state's output \
                     slot: {:?}",
                    poll.blocks
                )
            });
        assert!(
            stmts_of(poll).any(|s| matches!(
                s,
                Stmt::SetField { record, index, value: v, ty }
                    if *record == STATE && *index == STATE_SLOT_TEMPS + 1
                        && *v == value && *ty == MirTy::F64
            )),
            "the awaited value must be stored into the await result's own temp \
             slot, as an f64: {:?}",
            poll.blocks
        );
    }

    #[test]
    fn an_awaited_value_reaches_its_slot_before_the_statements_that_read_it() {
        // Order, which a whole-function scan of the statements cannot see. In the
        // two-await fixture the second await's result is temp 3 and the very next
        // statement reads it, so that await's continuation both stores and reads
        // slot `STATE_SLOT_TEMPS + 3`.
        //
        // Emitted after the statements that followed the await, the copy out of
        // the inner state object still exists, still names the right slot and
        // still reads the right class, so nothing about its presence or its shape
        // distinguishes it -- position is the only thing that does. What breaks
        // is that the expression the await feeds reads whatever the slot held
        // beforehand, which on a first pass is the allocator's zero.
        let mut m = module_with_async_fn_awaiting_twice(MirTy::F64);
        transform(&mut m);
        let poll = find(&m, "f.15$poll");
        let second = resume_arms(poll)
            .into_iter()
            .find_map(|(tag, b)| (tag == FIRST_RESUME_TAG + 1).then_some(b))
            .expect("a second resume tag");
        let cont = status_arm(poll, second, POLL_READY);
        let stmts = &poll.blocks[cont.0 as usize].stmts;
        let slot = STATE_SLOT_TEMPS + 3;
        let store = stmts
            .iter()
            .position(|s| {
                matches!(s, Stmt::SetField { record, index, .. }
                    if *record == STATE && *index == slot)
            })
            .unwrap_or_else(|| panic!("no store of the awaited value: {stmts:?}"));
        let read = stmts
            .iter()
            .position(|s| {
                matches!(s, Stmt::RecordField { record, index, .. }
                    if *record == STATE && *index == slot)
            })
            .unwrap_or_else(|| {
                panic!("the fixture's next statement must read the awaited value: {stmts:?}")
            });
        assert!(
            store < read,
            "the awaited value must be in its slot before anything reads it: {stmts:?}"
        );
    }

    #[test]
    fn an_inner_poll_status_that_is_neither_pending_nor_ready_traps() {
        // The mirror of the executor's own strictness. A generated poll function
        // returns exactly PENDING or READY, so an awaited future reporting
        // anything else is a compiler bug -- and treating an unrecognized status
        // as ready would complete the await from an output slot nothing wrote.
        // A trap aborts, which is also what keeps a panic from having to cross
        // this frame's boundary.
        let mut m = module_with_async_fn_awaiting_once(MirTy::F64);
        transform(&mut m);
        let poll = find(&m, "f.15$poll");
        let resume = resume_arms(poll)
            .into_iter()
            .find_map(|(tag, b)| (tag != 0).then_some(b))
            .expect("a resume arm");
        let (arms, default) = match &poll.blocks[resume.0 as usize].term {
            Terminator::Switch { arms, default, .. } => (arms.clone(), *default),
            other => panic!("expected a status switch, found {other:?}"),
        };
        let mut values: Vec<i64> = arms.iter().map(|(v, _)| *v).collect();
        values.sort_unstable();
        assert_eq!(
            values,
            vec![POLL_PENDING, POLL_READY],
            "exactly the two statuses the ABI defines may be recognized: {arms:?}"
        );
        assert!(
            matches!(poll.blocks[default.0 as usize].term, Terminator::Trap),
            "an unrecognized status must reach a trap, not fall into a state: {:?}",
            poll.blocks
        );
    }

    #[test]
    fn a_unit_awaits_result_is_not_copied_out_of_the_inner_state() {
        // Nothing to copy: a unit-output future writes an explicit zero to its
        // output slot and the awaiting body has no temp to put it in. Asserted
        // as "the poll function never chases a pointer other than the state
        // object", which is what a spurious extraction would break.
        let mut m = module_with_async_fn_awaiting_unit();
        transform(&mut m);
        let poll = find(&m, "f.15$poll");
        for s in stmts_of(poll) {
            if let Stmt::RecordField { record, index, .. } = s {
                assert_eq!(
                    *record, STATE,
                    "a unit await must not read field {index} of the inner \
                     future: {:?}",
                    poll.blocks
                );
            }
        }
        // It must still suspend and resume properly.
        assert_eq!(resume_arms(poll).len(), 2, "{:?}", poll.blocks);
    }

    #[test]
    fn an_await_in_a_loop_resumes_at_the_await_and_keeps_its_back_edge() {
        // The same suspend point, resumed more than once. Two failures a
        // straight-line fixture cannot reach:
        //
        // - The loop's back edge has to follow the renumbering the split
        //   introduces. Left pointing at its pre-split id it lands in whatever
        //   block now carries that number, which for a resumable poll function
        //   is the shared trap block -- the loop would abort on its second
        //   iteration.
        // - The resume arm has to target the await's own poll block, not the
        //   loop header. Resuming at the header would re-test the condition and
        //   build a second future, abandoning the one already in flight.
        let mut m = module_with_async_fn_awaiting_in_a_loop(MirTy::F64);
        transform(&mut m);
        let poll = find(&m, "f.15$poll");

        let header = poll
            .blocks
            .iter()
            .position(|b| matches!(b.term, Terminator::Branch { .. }))
            .expect("the loop header survives as a conditional branch");
        assert!(
            poll.blocks[header]
                .stmts
                .iter()
                .any(|s| matches!(s, Stmt::ConstBool(..))),
            "the header must be the block that re-tests the condition: {:?}",
            poll.blocks
        );
        assert!(
            poll.blocks
                .iter()
                .any(|b| matches!(b.term, Terminator::Goto(t) if t.0 as usize == header)),
            "the back edge must reach the renumbered header (block {header}): {:?}",
            poll.blocks
        );

        let resume = resume_arms(poll)
            .into_iter()
            .find_map(|(tag, b)| (tag != 0).then_some(b))
            .expect("a resume arm");
        assert!(
            poll.blocks[resume.0 as usize]
                .stmts
                .iter()
                .any(|s| matches!(s, Stmt::CallIndirect { .. })),
            "resuming must re-poll the awaited future, not re-enter the loop \
             header: {:?}",
            poll.blocks
        );

        // The header's own two arms have to be renumbered as well, and nothing
        // above reaches them: `Branch`'s targets are named fields, so leaving
        // them alone is its own defect, and un-remapped they name the entry
        // piece and the header itself -- both past the reserved ids, so the
        // reserved-block property is blind to it, and both give a loop that
        // never leaves. Identified by what each target *does* rather than by its
        // id, so this also rejects the two being swapped.
        let (then_, else_) = match &poll.blocks[header].term {
            Terminator::Branch { then_, else_, .. } => (*then_, *else_),
            other => panic!("expected the header's branch, found {other:?}"),
        };
        assert!(
            poll.blocks[then_.0 as usize]
                .stmts
                .iter()
                .any(|s| matches!(s, Stmt::Call { .. })),
            "the taken arm must reach the loop body's leading piece, the block \
             that builds the awaited future: {:?}",
            poll.blocks
        );
        assert!(
            matches!(poll.blocks[else_.0 as usize].term, Terminator::Return(_)),
            "the untaken arm must reach the exit, which completes: {:?}",
            poll.blocks
        );
    }

    #[test]
    fn every_exit_from_a_resumable_poll_fn_returns_a_constant_pending_or_ready() {
        // The executor panics on any status that is neither, and that panic
        // cannot unwind out of a generated frame -- it kills the process. So
        // every `Return` must carry a constant one of the two, never the inner
        // poll's status forwarded (which is only PENDING by coincidence on that
        // path) and never a body value.
        for mut m in every_awaiting_module() {
            transform(&mut m);
            let poll = find(&m, "f.15$poll");
            let returned: Vec<Temp> = poll
                .blocks
                .iter()
                .filter_map(|b| match &b.term {
                    Terminator::Return(Some(t)) => Some(*t),
                    _ => None,
                })
                .collect();
            assert!(!returned.is_empty(), "{:?}", poll.blocks);
            for t in returned {
                assert_eq!(poll.temps[t.0 as usize], MirTy::I64, "a status is an i64");
                let v = stmts_of(poll)
                    .find_map(|s| match s {
                        Stmt::ConstInt(d, v) if *d == t => Some(*v),
                        _ => None,
                    })
                    .unwrap_or_else(|| {
                        panic!(
                            "the returned temp %{} must be a status constant: {:?}",
                            t.0, poll.blocks
                        )
                    });
                assert!(
                    v == POLL_PENDING || v == POLL_READY,
                    "a poll function may return only PENDING or READY, found {v}: {:?}",
                    poll.blocks
                );
            }
        }
    }

    /// Every await fixture in this module, so a whole-function invariant is
    /// checked against all of them rather than whichever one it was written for.
    fn every_awaiting_module() -> Vec<Module> {
        vec![
            module_with_async_fn_awaiting_once(MirTy::F64),
            module_with_async_fn_awaiting_twice(MirTy::F64),
            module_with_async_fn_awaiting_in_a_loop(MirTy::F64),
            module_with_async_fn_awaiting_in_a_match(MirTy::F64),
            module_with_async_fn_awaiting_unit(),
        ]
    }

    #[test]
    fn no_terminator_names_a_reserved_block_except_a_status_switchs_default() {
        // The renumbering, asserted as a property instead of by recomputing the
        // block ids the split chose -- which would only restate the arithmetic
        // that produced them.
        //
        // A resumable poll function keeps the two lowest ids for its dispatch and
        // its trap block, so any *other* terminator still naming one of them is
        // one the split failed to remap. Neither outcome is a crash at the seam:
        // reaching the trap aborts, and re-entering the dispatch re-reads the tag
        // and jumps somewhere plausible. `Switch` is the shape most exposed,
        // because its targets are a `Vec` the remap has to walk rather than named
        // fields the compiler would notice were unused.
        //
        // The one deliberate reference to a reserved block is `BAD_DISCRIMINANT`
        // as a switch `default`, so that is the only exemption.
        for mut m in every_awaiting_module() {
            transform(&mut m);
            let poll = find(&m, "f.15$poll");
            for (i, b) in poll.blocks.iter().enumerate() {
                let named: Vec<BlockId> = match &b.term {
                    Terminator::Goto(t) => vec![*t],
                    Terminator::Branch { then_, else_, .. } => vec![*then_, *else_],
                    Terminator::Switch { arms, default, .. } => arms
                        .iter()
                        .map(|(_, b)| *b)
                        .chain((*default != BAD_DISCRIMINANT).then_some(*default))
                        .collect(),
                    Terminator::Return(_) | Terminator::Trap => Vec::new(),
                };
                for t in named {
                    assert!(
                        t.0 >= RESERVED_BLOCKS,
                        "block {i} targets reserved block {t:?}, so the split did \
                         not renumber it: {:?}",
                        poll.blocks
                    );
                }
            }
        }
    }

    #[test]
    fn a_body_switchs_default_is_renumbered_along_with_its_arms() {
        // `Switch::default` is a separate field from `Switch::arms`, so leaving
        // only the default un-remapped is its own defect -- and the reserved-block
        // property above cannot see it. That test exempts any default equal to
        // `BAD_DISCRIMINANT` and otherwise only requires `>= RESERVED_BLOCKS`, and
        // the match fixture's original default is `BlockId(3)`, which clears that
        // bar while naming a completely different block.
        //
        // In the fixture the original default is the exhaustive-match trap, and
        // after the split it is still the only `Trap`-terminated block the body
        // has -- so "the default still reaches a trap" pins it exactly. Left
        // un-remapped, `BlockId(3)` is the awaiting arm's leading piece, so the
        // defect builds a second future and polls it instead of aborting. That is
        // also why no runtime test can catch this one: the default is an
        // unreachable exhaustive-match trap, so neither a source-level `match`
        // test nor a driver probe ever branches through it.
        let mut m = module_with_async_fn_awaiting_in_a_match(MirTy::F64);
        transform(&mut m);
        let poll = find(&m, "f.15$poll");
        let defaults: Vec<BlockId> = poll
            .blocks
            .iter()
            .filter_map(|b| match &b.term {
                Terminator::Switch { default, .. } if *default != BAD_DISCRIMINANT => {
                    Some(*default)
                }
                _ => None,
            })
            .collect();
        assert_eq!(
            defaults.len(),
            1,
            "the fixture has exactly one `Switch` of its own, and its default must \
             not have been collapsed onto the shared trap: {:?}",
            poll.blocks
        );
        assert!(
            matches!(poll.blocks[defaults[0].0 as usize].term, Terminator::Trap),
            "the body `Switch`'s default must still reach the trap block it named \
             before the split, at its new id: {:?}",
            poll.blocks
        );
    }

    #[test]
    fn the_split_leaves_no_await_marker_and_no_dangling_block_target() {
        // Two whole-function invariants, over every await shape here. A
        // surviving marker reaches a codegen backend, which has no machine code
        // for it; a block target past the end of the block list indexes out of
        // bounds inside the backend rather than being diagnosed.
        for mut m in every_awaiting_module() {
            transform(&mut m);
            for f in &m.functions {
                assert!(
                    !stmts_of(f).any(|s| matches!(s, Stmt::Await { .. })),
                    "`{}` still contains an await marker: {:?}",
                    f.name,
                    f.blocks
                );
                for t in targets(f) {
                    assert!(
                        (t.0 as usize) < f.blocks.len(),
                        "`{}` jumps to {t:?}, past its {} blocks: {:?}",
                        f.name,
                        f.blocks.len(),
                        f.blocks
                    );
                }
            }
        }
    }

    #[test]
    fn an_await_free_body_gains_no_dispatch_block() {
        // A body with one state needs no dispatch: switching on a tag that is
        // always 0 is the identity, and moving the entry block would renumber
        // every terminator target for nothing. This is also what keeps the
        // exact load- and store-sequence assertions above describing the code
        // that runs.
        let mut m = module_with_async_const_fn(MirTy::F64, &[]);
        let before = m.functions[0].blocks.len();
        transform(&mut m);
        let poll = find(&m, "f.15$poll");
        assert_eq!(
            poll.blocks.len(),
            before,
            "the block list must be untouched: {:?}",
            poll.blocks
        );
        assert!(
            matches!(poll.blocks[0].term, Terminator::Return(_)),
            "no tag dispatch may be introduced: {:?}",
            poll.blocks
        );
    }

    #[test]
    fn a_suspend_needs_no_state_slot_beyond_the_bodys_own_temps() {
        // The decision the state-size cross-check rests on: an await adds no
        // slot. The resume tag is `STATE_SLOT_TAG`, which every state object
        // already has, and the awaited future is an ordinary body temp with an
        // ordinary temp slot -- so the allocation the wrapper sizes from the
        // pre-transform temp count is still exactly right after the split. If a
        // later change does need a slot, `state_slot_count` is the one place it
        // goes, and the `debug_assert!`s in `split_into_poll_and_wrapper` fail
        // until it is added there.
        let mut m = module_with_async_fn_awaiting_twice(MirTy::F64);
        let n_temps = m.functions[0].temps.len();
        transform(&mut m);
        let wrapper = find(&m, "f.15");
        let expected = ((STATE_SLOT_TEMPS as usize + n_temps) * 8) as i64;
        let sizes: Vec<i64> = stmts_of(wrapper)
            .filter_map(|s| match s {
                Stmt::CallRuntime {
                    func: RtFunc::Alloc,
                    args,
                    ..
                } => Some(args.clone()),
                _ => None,
            })
            .flatten()
            .filter_map(|t| {
                stmts_of(wrapper).find_map(|s| match s {
                    Stmt::ConstInt(d, v) if *d == t => Some(*v),
                    _ => None,
                })
            })
            .collect();
        assert_eq!(
            sizes,
            vec![expected],
            "a resumable body's state object is still \
             (STATE_SLOT_TEMPS + n_temps) * 8 bytes: {:?}",
            wrapper.blocks
        );
    }
}
