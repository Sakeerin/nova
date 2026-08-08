//! The async state-machine transform, part 1: `async fn`s with no `.await`.
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
//! # Why every value goes to the state object
//!
//! Nothing in an await-free body needs to be spilled: with no suspend point,
//! no value can be live across one. The spill is unconditional anyway because
//! the resumable form of this transform splits a body at its await points into
//! blocks that run in *different* invocations of the poll function, and a value
//! left in a temp across such a split is read on a path where nothing defined
//! it — which the Cranelift frontend resolves as a block parameter fed from a
//! predecessor that never executes, i.e. garbage, with no diagnostic. Spilling
//! everything removes the liveness question instead of answering it, and does
//! so for a shape this half of the transform can already be tested against.
//!
//! # A panic must not cross a poll function's boundary
//!
//! The `PollFn` ABI is `"C-unwind"`, which permits an unwind to pass through
//! without aborting, but a Cranelift- or LLVM-emitted frame has no landing pads
//! and no drop glue, so an unwind *through* a poll function would skip whatever
//! the executor's bookkeeping needs. What this pass contributes to that
//! requirement is narrow and checkable: **it introduces no call at all into a
//! poll function.** Every statement it emits is a field load, a field store or
//! an integer constant; the only calls in a poll body are the ones the
//! `async fn` already contained, under the same discipline as any other Nova
//! function (`nova_rt_panic_str` and `nova_rt_check_bounds` abort rather than
//! unwind, and a `Terminator::Trap` becomes a trap instruction). The one call
//! this pass adds — the state object's allocation — is in the wrapper, which is
//! an ordinary Nova function and not a poll function.
//!
//! # The layout this pass builds
//!
//! Declared here **and** in `nova-runtime/src/task.rs`: `nova-mir` must not
//! depend on `nova-runtime`, and `nova-runtime` must not depend on `nova-mir`,
//! so the two ends of this ABI are two independent declarations of one layout.
//! `nova-codegen-cranelift` depends on both and is where they are pinned
//! together (`the_state_layout_matches_nova_runtimes`).

use crate::{Block, Function, MirTy, Module, RtFunc, Stmt, Temp, Terminator};
use nova_hir as hir;

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
/// A poll function has not produced a value yet. Unreachable from this pass —
/// an await-free body cannot suspend — and declared here because the executor
/// rejects any status that is neither this nor [`POLL_READY`], so both halves
/// of that pair belong in one place.
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

/// Rewrite every `async fn` in `module` into a poll function plus a wrapper.
///
/// Idempotent: the flag it keys on is cleared on both halves, so a second run
/// finds nothing to do. `lower_module` asserts the postcondition — no function
/// left flagged — because a missed one reaches codegen with its body's return
/// class instead of a future's.
///
/// # Preconditions
///
/// Every flagged function's body is free of suspend points ([`contains_await`]
/// is the predicate, and `lower_module` rejects the rest with `E0088`).
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
    f.blocks = blocks.into_iter().map(|b| sp.block(b)).collect();
    f.temps = sp.temps;

    f.name.push_str("$poll");
    f.takes_env = true;
    f.params = 1;
    f.capture_count = 0;
    f.ret = MirTy::I64;
    f.is_async = false;
    wrapper
}

/// Build the function that keeps the original symbol: allocate the state,
/// seed it, and return the `{ poll_code, state }` future.
///
/// Reads `f` while it is still the pre-transform body, so it must run before
/// [`split_into_poll_and_wrapper`] mutates it.
fn build_wrapper(f: &Function) -> Function {
    // The temps the ABI hands this function: the environment pointer, if it
    // has one, then the real parameters — numbered exactly as `lower_function`
    // numbered them, which is what makes seeding them into slot `i` line up
    // with what the rewritten body reads back out of slot `i`.
    let abi_count = f.params as usize + usize::from(f.takes_env);
    let mut temps: Vec<MirTy> = f.temps[..abi_count].to_vec();
    let mut stmts: Vec<Stmt> = Vec::new();

    // `(STATE_SLOT_TEMPS + n_temps) * 8` bytes, through `nova_rt_alloc` —
    // which is `gc::alloc(.., true)`, i.e. SCANNED. That is load-bearing, not
    // incidental: a heap-valued output written to `STATE_SLOT_OUTPUT` is kept
    // alive only by the collector tracing through this object, and an
    // unscanned one is marked but never traced.
    let bytes = (STATE_SLOT_TEMPS as usize + f.temps.len()) as i64 * 8;
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

    Function {
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
    }
}

/// Append a temp of class `ty` and return its id. The temp list's length *is*
/// the next id, in both halves of this pass.
fn push_temp(temps: &mut Vec<MirTy>, ty: MirTy) -> Temp {
    let t = Temp(temps.len() as u32);
    temps.push(ty);
    t
}

/// Rewrites a pre-transform body so that every value it names lives in the
/// state object instead of a temp.
struct Spiller<'a> {
    /// The poll function's temp list, grown one entry per load and per result.
    temps: Vec<MirTy>,
    /// The pre-transform temp classes, indexed by original temp id — which is
    /// also the index into the state object's temp slots.
    orig: &'a [MirTy],
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

    /// Load original temp `t` out of its slot into a fresh scratch temp.
    ///
    /// A fresh temp per load site, never one reused across uses: a reused
    /// scratch would be a value living outside the state object across
    /// statements, which is exactly what this pass exists to prevent.
    fn reload(&mut self, out: &mut Vec<Stmt>, t: Temp) -> Temp {
        let ty = self.orig_ty(t);
        let dst = self.scratch(ty);
        out.push(Stmt::RecordField {
            dst,
            record: STATE,
            index: Self::slot(t),
            ty,
        });
        dst
    }

    fn block(&mut self, b: Block) -> Block {
        let mut stmts: Vec<Stmt> = Vec::with_capacity(b.stmts.len() * 3);
        for mut s in b.stmts {
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
            stmts.append(&mut loads);
            stmts.push(s);
            stmts.append(&mut stores);
        }

        let term = match b.term {
            Terminator::Goto(target) => Terminator::Goto(target),
            Terminator::Branch { cond, then_, else_ } => Terminator::Branch {
                cond: self.reload(&mut stmts, cond),
                then_,
                else_,
            },
            Terminator::Switch {
                disc,
                arms,
                default,
            } => Terminator::Switch {
                disc: self.reload(&mut stmts, disc),
                arms,
                default,
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
                        let v = self.reload(&mut stmts, t);
                        stmts.push(Stmt::SetField {
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
                        let z = self.scratch(MirTy::I64);
                        stmts.push(Stmt::ConstInt(z, 0));
                        stmts.push(Stmt::SetField {
                            record: STATE,
                            index: STATE_SLOT_OUTPUT,
                            value: z,
                            ty: MirTy::I64,
                        });
                    }
                }
                let ready = self.scratch(MirTy::I64);
                stmts.push(Stmt::ConstInt(ready, POLL_READY));
                Terminator::Return(Some(ready))
            }
            // Left as a trap, which aborts rather than unwinding — a panic
            // must not cross a generated poll function's boundary, and this
            // pass introduces no call that could raise one (see the module
            // doc comment).
            Terminator::Trap => Terminator::Trap,
        };
        Block { stmts, term }
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

/// Whether `body` contains a suspend point anywhere inside it.
///
/// **The boundary this pass can express.** `lower_module` rejects a reachable
/// `async fn` with `E0088` exactly when this is true, and transforms it
/// exactly when it is false, so "handled" and "rejected" are decided by one
/// predicate rather than by two conditions that could drift apart. Whatever
/// makes the true case expressible owns removing both together.
///
/// Recurses through every expression form rather than scanning the body's
/// top-level statement list: a suspend point buried in a loop condition or a
/// string interpolation suspends just as much as one in tail position.
///
/// A closure written inside an `async fn` is a separate `hir::Function` and so
/// is not in this tree — correctly, since `.await` inside a closure body is
/// already rejected by `nova-typeck` (a closure is never `is_async`).
pub(crate) fn contains_await(body: &hir::Expr) -> bool {
    use hir::ExprKind as K;
    let any = |es: &[hir::Expr]| es.iter().any(contains_await);
    let opt = |e: &Option<Box<hir::Expr>>| e.as_deref().is_some_and(contains_await);
    match &body.kind {
        K::Await(_) => true,
        K::IntLit(_)
        | K::FloatLit(_)
        | K::BoolLit(_)
        | K::StrLit(_)
        | K::CharLit(_)
        | K::Unit
        | K::Break
        | K::Continue
        | K::Local(_) => false,
        K::MakeClosure { captures, .. } => any(captures),
        K::Call { args, .. }
        | K::MakeVariant { args, .. }
        | K::MakeArray { elems: args }
        | K::StrConcat(args) => any(args),
        K::MakeRecord { fields, .. } => any(fields),
        K::FieldGet { target, .. } | K::ArrayLen { target } => contains_await(target),
        K::FieldSet { target, value, .. } => contains_await(target) || contains_await(value),
        K::ArrayRepeat { init, len } => contains_await(init) || contains_await(len),
        K::Index { target, index } => contains_await(target) || contains_await(index),
        K::IndexSet {
            target,
            index,
            value,
        } => contains_await(target) || contains_await(index) || contains_await(value),
        K::TraitCall { receiver, args, .. } => {
            receiver.as_deref().is_some_and(contains_await) || any(args)
        }
        K::Binary { lhs, rhs, .. } | K::LogicalAnd { lhs, rhs } | K::LogicalOr { lhs, rhs } => {
            contains_await(lhs) || contains_await(rhs)
        }
        K::Unary { expr, .. } | K::ToStr(expr) => contains_await(expr),
        K::Let { init, .. } => contains_await(init),
        K::Assign { value, .. } => contains_await(value),
        K::Block { stmts, trailing } => any(stmts) || opt(trailing),
        K::If { cond, then, else_ } => contains_await(cond) || contains_await(then) || opt(else_),
        K::While { cond, body } => contains_await(cond) || contains_await(body),
        K::Match { scrutinee, arms } => {
            contains_await(scrutinee) || arms.iter().any(|a| contains_await(&a.body))
        }
        K::Return(value) => opt(value),
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
    fn the_wrapper_writes_the_entry_resume_tag() {
        // Deleting this store passes every other test in the workspace, because
        // `gc::alloc` hands back zeroed memory and nothing in an await-free poll
        // function reads the tag at all -- measured by mutation. So the store is
        // an intent that only an assertion can hold. The invariant is that a
        // fresh future starts in its first state, and this store is the only
        // thing that makes that a property of the generated code rather than of
        // the allocator's zeroing — which stops being enough as soon as
        // anything reads the tag.
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

    // === the suspension boundary ===

    fn body_of(src: &str) -> nova_hir::Expr {
        use nova_diagnostics::FileId;
        let (tokens, lex_errors) = nova_lexer::lex(src, FileId::DUMMY);
        assert!(lex_errors.is_empty(), "lex: {lex_errors:?}");
        let (ast, parse_errors) = nova_parser::parse(&tokens, FileId::DUMMY);
        assert!(parse_errors.is_empty(), "parse: {parse_errors:?}");
        let resolved = nova_resolver::resolve(&ast.expect("no AST"));
        let checked = nova_typeck::check(&resolved.file, &resolved.definitions);
        assert!(
            checked.diagnostics.is_empty(),
            "typeck: {:?}",
            checked.diagnostics
        );
        let f = checked
            .module
            .functions
            .iter()
            .find(|f| f.name == "f")
            .expect("`f` was checked");
        assert!(f.is_async, "the fixture's `f` must be async");
        f.body.clone()
    }

    #[test]
    fn contains_await_is_the_boundary_between_transformed_and_rejected() {
        // The single predicate `lower_module` keys its `E0088` rejection on
        // and this pass keys its coverage on, so the two can never disagree
        // about which functions the transform handles.
        assert!(!contains_await(&body_of("async fn f() -> Int { 1 }")));
        assert!(contains_await(&body_of(
            "async fn g() -> Int { 1 }\nasync fn f() -> Int { g().await }"
        )));
    }

    #[test]
    fn contains_await_looks_inside_nested_expressions() {
        // A predicate that only inspected the body's top-level statement list
        // would pass the test above (the await IS the trailing expression
        // there) and still let a suspend point through from anywhere real code
        // puts one. Each of these buries the `.await` under a different
        // expression form.
        for src in [
            "async fn g() -> Int { 1 }\n\
             async fn f() -> Int { if true { g().await } else { 0 } }",
            "async fn g() -> Int { 1 }\n\
             async fn f() -> Int { let mut i = 0\n while i < 1 { i = g().await }\n i }",
            "async fn g() -> Int { 1 }\n\
             async fn f() -> Int { g().await + 1 }",
            "async fn g() -> Int { 1 }\n\
             async fn f() -> Int { match g().await { _ => 0 } }",
            "async fn g() -> Int { 1 }\n\
             async fn f() -> Int { return g().await }",
            "async fn g() -> Int { 1 }\n\
             async fn f() -> String { \"${g().await}\" }",
            "async fn g() -> Int { 1 }\n\
             async fn f() -> Int { let a = [g().await, 2]\n a[0] }",
        ] {
            assert!(
                contains_await(&body_of(src)),
                "a buried `.await` must still be found: {src}"
            );
        }
    }
}
