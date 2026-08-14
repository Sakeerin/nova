//! HIR → MIR lowering for one (already monomorphized) function.

use nova_diagnostics::Diagnostic;
use nova_hir as hir;
use nova_hir::Ty;
use nova_resolver::{Builtin, DefId};

use crate::{
    mangle, mir_ty, Block, BlockId, Function, MirTy, OperandClass, RtFunc, Stmt, Temp, Terminator,
    MAX_ARRAY_LEN,
};

/// How one [`Builtin`] reaches its implementation, decided by the exhaustive
/// match in [`Lowerer::lower_call`].
///
/// A named enum rather than `Option<RtFunc>` because there is more than one
/// reason a builtin has no runtime symbol, and the two are not
/// interchangeable: mixing them up emits a register move where two loads
/// belong, which has no diagnostic anywhere downstream.
enum Lowering {
    /// One `CallRuntime` to this runtime function.
    Runtime(RtFunc),
    /// The result *is* the argument, in a different `Ty` of the same `MirTy`.
    Reinterpret,
    /// Two typed field loads reaching a finished future's output slot.
    FutureOutput,
}

/// Lower a specialized (generic-free) HIR function to MIR.
///
/// `request` is called for every direct call / function reference so the
/// monomorphization driver can enqueue the callee instance.
pub(crate) fn lower_function(
    func: &hir::Function,
    mangled: &str,
    module: &hir::Module,
    entry: DefId,
    request: &mut dyn FnMut(DefId, Vec<Ty>),
) -> Result<Function, Vec<Diagnostic>> {
    let mut lo = Lowerer {
        module,
        entry,
        request,
        temps: Vec::new(),
        blocks: vec![BlockState::default()],
        current: BlockId(0),
        local_map: vec![Temp(0); func.locals.len()],
        loop_targets: Vec::new(),
        diagnostics: Vec::new(),
    };

    let capture_count = func.capture_count as usize;
    if func.takes_env {
        // ABI: (env_ptr, real_params...). Temp 0 is the env pointer, then
        // one temp per real parameter; captured locals and body locals get
        // temps afterwards. Captured locals are loaded from the env at entry.
        let env = lo.new_temp(MirTy::Ptr);
        // Real parameters (HIR locals capture_count..capture_count+params).
        for i in 0..func.params as usize {
            let local = &func.locals[capture_count + i];
            lo.local_map[capture_count + i] = lo.new_temp(mir_ty(&local.ty));
        }
        // Captured locals: fresh temps, loaded from the env record.
        for (i, local) in func.locals.iter().take(capture_count).enumerate() {
            lo.local_map[i] = lo.new_temp(mir_ty(&local.ty));
        }
        // Remaining body locals.
        for i in (capture_count + func.params as usize)..func.locals.len() {
            let local = &func.locals[i];
            lo.local_map[i] = lo.new_temp(mir_ty(&local.ty));
        }
        // Emit capture loads at entry.
        for i in 0..capture_count {
            let ty = mir_ty(&func.locals[i].ty);
            if ty != MirTy::Unit {
                let dst = lo.local_map[i];
                lo.push(Stmt::RecordField {
                    dst,
                    record: env,
                    index: i as u32,
                    ty,
                });
            }
        }
    } else {
        // Normal ABI: parameters occupy the first temps in local order.
        for (i, local) in func.locals.iter().enumerate() {
            let t = lo.new_temp(mir_ty(&local.ty));
            debug_assert_eq!(t.0 as usize, i, "params must map to leading temps");
            lo.local_map[i] = t;
        }
    }

    let body_ret = body_return_ty(func);
    let result = lo.lower_expr(&func.body);
    if !diverges(&func.body) {
        match mir_ty(body_ret) {
            MirTy::Unit => lo.terminate(Terminator::Return(None)),
            _ => lo.terminate(Terminator::Return(Some(result))),
        }
    }

    if !lo.diagnostics.is_empty() {
        return Err(lo.diagnostics);
    }

    let blocks = lo
        .blocks
        .into_iter()
        .map(|b| Block {
            stmts: b.stmts,
            // Dead blocks (opened after divergence, never jumped to) may
            // lack a terminator; they are unreachable, so trap.
            term: b.term.unwrap_or(Terminator::Trap),
        })
        .collect();

    Ok(Function {
        name: mangled.to_string(),
        params: func.params,
        takes_env: func.takes_env,
        capture_count: func.capture_count,
        temps: lo.temps,
        ret: mir_ty(body_ret),
        is_async: func.is_async,
        blocks,
    })
}

/// The type this function's BODY produces, which is not always the type its
/// signature returns.
///
/// An `async fn`'s `ret_ty` is `Future<T>` (always `MirTy::Ptr`) while its body
/// has type `T`, so the two disagree wherever `T` is not itself pointer-class —
/// `Float` (`MirTy::F64`) being the case that crosses register banks and so the
/// only one a backend can catch. Lowering the body against the wrapped type
/// produced a Cranelift verifier error there and, at `Int`, silently compiled.
/// `async_lower::transform` then rewrites this function into a poll function
/// (returning a status) plus a wrapper (returning the `Future<T>` pointer), so
/// the class here is only ever the intermediate one.
///
/// Keyed on `is_async`, not on "does `ret_ty` start with `Future`": a plain
/// function may itself declare `-> Future<T>` and forward an async call's
/// result unchanged (`nova-typeck`'s `wrap_fn_value`), and its body really does
/// produce the future.
fn body_return_ty(func: &hir::Function) -> &Ty {
    match &func.ret_ty {
        Ty::Future(out) if func.is_async => out,
        other => other,
    }
}

fn diverges(e: &hir::Expr) -> bool {
    matches!(e.ty, Ty::Never)
}

#[derive(Default)]
struct BlockState {
    stmts: Vec<Stmt>,
    term: Option<Terminator>,
}

struct Lowerer<'a> {
    module: &'a hir::Module,
    /// DefId of the program entry point, named `main` rather than mangled.
    entry: DefId,
    request: &'a mut dyn FnMut(DefId, Vec<Ty>),
    temps: Vec<MirTy>,
    blocks: Vec<BlockState>,
    current: BlockId,
    /// LocalId index → temp.
    local_map: Vec<Temp>,
    /// Stack of `(continue_target, break_target)` for enclosing loops.
    loop_targets: Vec<(BlockId, BlockId)>,
    diagnostics: Vec<Diagnostic>,
}

impl<'a> Lowerer<'a> {
    fn new_temp(&mut self, ty: MirTy) -> Temp {
        let t = Temp(self.temps.len() as u32);
        self.temps.push(ty);
        t
    }

    fn new_block(&mut self) -> BlockId {
        let id = BlockId(self.blocks.len() as u32);
        self.blocks.push(BlockState::default());
        id
    }

    fn switch_to(&mut self, id: BlockId) {
        self.current = id;
    }

    fn push(&mut self, stmt: Stmt) {
        let b = &mut self.blocks[self.current.0 as usize];
        if b.term.is_none() {
            b.stmts.push(stmt);
        }
        // Statements after a terminator are unreachable; drop them.
    }

    /// Set the current block's terminator (first one wins).
    fn terminate(&mut self, term: Terminator) {
        let b = &mut self.blocks[self.current.0 as usize];
        if b.term.is_none() {
            b.term = Some(term);
        }
    }

    fn unit_temp(&mut self) -> Temp {
        let t = self.new_temp(MirTy::Unit);
        self.push(Stmt::ConstUnit(t));
        t
    }

    /// Emit a runtime bounds check `check_bounds(index, len(arr))` that
    /// aborts if `index` is out of range, before an array access.
    fn bounds_check(&mut self, arr: Temp, index: Temp) {
        let len = self.new_temp(MirTy::I64);
        self.push(Stmt::ArrayLen { dst: len, arr });
        self.push(Stmt::CallRuntime {
            dst: None,
            func: RtFunc::CheckBounds,
            args: vec![index, len],
        });
    }

    /// Copy `src` into `dst` unless the value class is `Unit` (no data).
    fn copy(&mut self, dst: Temp, src: Temp) {
        if self.temps[dst.0 as usize] != MirTy::Unit {
            self.push(Stmt::Copy { dst, src });
        }
    }

    fn callee_name(&mut self, def: DefId, type_args: &[Ty]) -> String {
        (self.request)(def, type_args.to_vec());
        // Mirror `lower_module`'s entry-naming: a call to the entry point targets
        // the bare `main` symbol, while every other callee uses its unique
        // DefId-mangled name so cross-module same-name items never collide.
        if def == self.entry {
            return "main".to_string();
        }
        // An `extern` (FFI) callee is emitted under its raw C symbol, unmangled.
        if let Some(ext) = self.module.extern_fn(def) {
            return ext.symbol.clone();
        }
        let name = self
            .module
            .function(def)
            .map(|f| f.name.clone())
            .unwrap_or_else(|| format!("__unknown_{}", def.0));
        mangle(def, &name, type_args)
    }

    fn lower_expr(&mut self, e: &hir::Expr) -> Temp {
        use hir::ExprKind as K;
        match &e.kind {
            K::IntLit(v) => {
                let t = self.new_temp(MirTy::I64);
                self.push(Stmt::ConstInt(t, *v));
                t
            }
            K::FloatLit(v) => {
                let t = self.new_temp(MirTy::F64);
                self.push(Stmt::ConstFloat(t, *v));
                t
            }
            K::BoolLit(v) => {
                let t = self.new_temp(MirTy::I8);
                self.push(Stmt::ConstBool(t, *v));
                t
            }
            K::StrLit(s) => {
                let t = self.new_temp(MirTy::Ptr);
                self.push(Stmt::ConstStr(t, s.clone()));
                t
            }
            K::CharLit(c) => {
                let t = self.new_temp(MirTy::I64);
                self.push(Stmt::ConstInt(t, *c as i64));
                t
            }
            K::Unit => self.unit_temp(),
            K::Local(l) => self.local_map[l.0 as usize],
            K::MakeClosure {
                func,
                type_args,
                captures,
            } => {
                let code = self.callee_name(*func, type_args);
                let cap_temps: Vec<(Temp, MirTy)> = captures
                    .iter()
                    .map(|c| {
                        let t = self.lower_expr(c);
                        (t, mir_ty(&c.ty))
                    })
                    .collect();
                let t = self.new_temp(MirTy::Ptr);
                self.push(Stmt::MakeClosure {
                    dst: t,
                    code,
                    captures: cap_temps,
                });
                t
            }
            K::Call {
                func,
                type_args,
                args,
            } => self.lower_call(e, func, type_args, args),
            K::MakeVariant { variant, args, .. } => {
                let fields: Vec<(Temp, MirTy)> = args
                    .iter()
                    .map(|a| {
                        let t = self.lower_expr(a);
                        (t, mir_ty(&a.ty))
                    })
                    .collect();
                let t = self.new_temp(MirTy::Ptr);
                self.push(Stmt::MakeSum {
                    dst: t,
                    tag: *variant,
                    fields,
                });
                t
            }
            K::MakeRecord { fields, .. } => {
                let lowered: Vec<(Temp, MirTy)> = fields
                    .iter()
                    .map(|f| {
                        let t = self.lower_expr(f);
                        (t, mir_ty(&f.ty))
                    })
                    .collect();
                let t = self.new_temp(MirTy::Ptr);
                self.push(Stmt::MakeRecord {
                    dst: t,
                    fields: lowered,
                });
                t
            }
            K::FieldGet { target, index } => {
                let record = self.lower_expr(target);
                let ty = mir_ty(&e.ty);
                if ty == MirTy::Unit {
                    return self.unit_temp();
                }
                let t = self.new_temp(ty);
                self.push(Stmt::RecordField {
                    dst: t,
                    record,
                    index: *index,
                    ty,
                });
                t
            }
            K::FieldSet {
                target,
                index,
                value,
            } => {
                // Left-to-right, as `IndexSet` does: the record expression is
                // evaluated before the value expression.
                let record = self.lower_expr(target);
                let v = self.lower_expr(value);
                let ty = mir_ty(&value.ty);
                // A unit-typed field has no runtime representation, so there is
                // nothing to store — but the value still had to be evaluated
                // for its side effects. Same rule as `IndexSet`/`FieldGet`.
                if ty != MirTy::Unit {
                    self.push(Stmt::SetField {
                        record,
                        index: *index,
                        value: v,
                        ty,
                    });
                }
                self.unit_temp()
            }
            K::MakeArray { elems } => {
                let lowered: Vec<(Temp, MirTy)> = elems
                    .iter()
                    .map(|el| {
                        let t = self.lower_expr(el);
                        (t, mir_ty(&el.ty))
                    })
                    .collect();
                let t = self.new_temp(MirTy::Ptr);
                self.push(Stmt::MakeArray {
                    dst: t,
                    elems: lowered,
                });
                t
            }
            K::ArrayRepeat { init, len } => self.lower_array_repeat(init, len),
            K::ArrayLen { target } => {
                let arr = self.lower_expr(target);
                let t = self.new_temp(MirTy::I64);
                self.push(Stmt::ArrayLen { dst: t, arr });
                t
            }
            K::Index { target, index } => {
                let arr = self.lower_expr(target);
                let idx = self.lower_expr(index);
                self.bounds_check(arr, idx);
                let ty = mir_ty(&e.ty);
                if ty == MirTy::Unit {
                    return self.unit_temp();
                }
                let t = self.new_temp(ty);
                self.push(Stmt::ArrayGet {
                    dst: t,
                    arr,
                    index: idx,
                    ty,
                });
                t
            }
            K::IndexSet {
                target,
                index,
                value,
            } => {
                let arr = self.lower_expr(target);
                let idx = self.lower_expr(index);
                let v = self.lower_expr(value);
                self.bounds_check(arr, idx);
                let ty = mir_ty(&value.ty);
                if ty != MirTy::Unit {
                    self.push(Stmt::ArraySet {
                        arr,
                        index: idx,
                        value: v,
                        ty,
                    });
                }
                self.unit_temp()
            }
            K::TraitCall {
                trait_id,
                method,
                self_ty,
                type_args,
                receiver,
                args,
            } => self.lower_trait_call(
                e,
                *trait_id,
                *method,
                self_ty,
                type_args,
                receiver.as_deref(),
                args,
            ),
            K::Binary { op, lhs, rhs } => self.lower_binary(*op, lhs, rhs),
            K::LogicalAnd { lhs, rhs } => self.lower_logical(lhs, rhs, true),
            K::LogicalOr { lhs, rhs } => self.lower_logical(lhs, rhs, false),
            K::Unary { op, expr } => {
                let src = self.lower_expr(expr);
                match op {
                    hir::UnOp::Neg => {
                        let class = operand_class(&expr.ty);
                        let t = self.new_temp(self.temps[src.0 as usize]);
                        self.push(Stmt::Neg { dst: t, class, src });
                        t
                    }
                    hir::UnOp::Not => {
                        let t = self.new_temp(MirTy::I8);
                        self.push(Stmt::Not { dst: t, src });
                        t
                    }
                    hir::UnOp::BitNot => {
                        let t = self.new_temp(MirTy::I64);
                        self.push(Stmt::BitNot { dst: t, src });
                        t
                    }
                }
            }
            K::Let { local, init } => {
                let v = self.lower_expr(init);
                if !diverges(init) {
                    let dst = self.local_map[local.0 as usize];
                    self.copy(dst, v);
                }
                self.unit_temp()
            }
            K::Assign { local, value } => {
                let v = self.lower_expr(value);
                if !diverges(value) {
                    let dst = self.local_map[local.0 as usize];
                    self.copy(dst, v);
                }
                self.unit_temp()
            }
            K::Block { stmts, trailing } => {
                for s in stmts {
                    self.lower_expr(s);
                }
                match trailing {
                    Some(t) => self.lower_expr(t),
                    None => self.unit_temp(),
                }
            }
            K::If { cond, then, else_ } => self.lower_if(e, cond, then, else_.as_deref()),
            K::While { cond, body } => {
                let header = self.new_block();
                let body_b = self.new_block();
                let exit = self.new_block();
                self.terminate(Terminator::Goto(header));
                self.switch_to(header);
                // `continue` → header (re-test), `break` → exit. Push before
                // lowering the condition so break/continue *in the condition*
                // target this loop.
                self.loop_targets.push((header, exit));
                let c = self.lower_expr(cond);
                if diverges(cond) {
                    // The condition never yields a bool (it breaks/returns);
                    // control has already left the header. Resume at the exit.
                    self.loop_targets.pop();
                    self.switch_to(exit);
                    return self.unit_temp();
                }
                self.terminate(Terminator::Branch {
                    cond: c,
                    then_: body_b,
                    else_: exit,
                });
                self.switch_to(body_b);
                self.lower_expr(body);
                self.loop_targets.pop();
                self.terminate(Terminator::Goto(header));
                self.switch_to(exit);
                self.unit_temp()
            }
            K::Break | K::Continue => {
                if let Some(&(cont, brk)) = self.loop_targets.last() {
                    let target = if matches!(e.kind, K::Break) {
                        brk
                    } else {
                        cont
                    };
                    self.terminate(Terminator::Goto(target));
                }
                // Code after break/continue is unreachable; continue lowering
                // into a fresh dead block.
                let dead = self.new_block();
                self.switch_to(dead);
                self.unit_temp()
            }
            K::Match { scrutinee, arms } => self.lower_match(e, scrutinee, arms),
            K::Return(value) => {
                match value {
                    Some(v) => {
                        let t = self.lower_expr(v);
                        if !diverges(v) {
                            match self.temps[t.0 as usize] {
                                MirTy::Unit => self.terminate(Terminator::Return(None)),
                                _ => self.terminate(Terminator::Return(Some(t))),
                            }
                        }
                    }
                    None => self.terminate(Terminator::Return(None)),
                }
                // Continue lowering into a fresh (unreachable) block.
                let dead = self.new_block();
                self.switch_to(dead);
                self.unit_temp()
            }
            K::ToStr(inner) => {
                let v = self.lower_expr(inner);
                let func = match &inner.ty {
                    Ty::Int => RtFunc::IntToStr,
                    Ty::Float => RtFunc::FloatToStr,
                    Ty::Bool => RtFunc::BoolToStr,
                    Ty::Char => RtFunc::CharToStr,
                    // String / error recovery: pass through.
                    _ => return v,
                };
                let t = self.new_temp(MirTy::Ptr);
                self.push(Stmt::CallRuntime {
                    dst: Some(t),
                    func,
                    args: vec![v],
                });
                t
            }
            K::StrConcat(parts) => {
                if parts.is_empty() {
                    let t = self.new_temp(MirTy::Ptr);
                    self.push(Stmt::ConstStr(t, String::new()));
                    return t;
                }
                let mut acc = self.lower_expr(&parts[0]);
                for part in &parts[1..] {
                    let rhs = self.lower_expr(part);
                    let t = self.new_temp(MirTy::Ptr);
                    self.push(Stmt::CallRuntime {
                        dst: Some(t),
                        func: RtFunc::StrConcat,
                        args: vec![acc, rhs],
                    });
                    acc = t;
                }
                acc
            }
            K::Await(inner) => {
                // A marker statement, not control flow. Performing the await —
                // polling the awaited future, suspending while it is pending,
                // resuming where it stopped — is `async_lower`'s block split,
                // and every part of that sequence addresses the state object
                // that split builds. There is no state object here, so lowering
                // the suspend into control flow at this point would mean
                // inventing that layout a second time, in a pass that runs
                // before monomorphization and so before the awaited output's
                // machine class is even known.
                //
                // No guard against `.await` outside an `async fn`: `nova-typeck`
                // makes that unrepresentable (`fcx.in_async`), so there is
                // nothing for this arm to reject.
                let future = self.lower_expr(inner);
                let ty = mir_ty(&e.ty);
                let dst = if ty == MirTy::Unit {
                    None
                } else {
                    Some(self.new_temp(ty))
                };
                self.push(Stmt::Await { dst, future });
                match dst {
                    Some(d) => d,
                    // A unit-output await produces no value, matching how a
                    // unit-returning call is lowered: `Stmt::Await`'s `dst` is
                    // `None` and the expression's own result is a fresh unit.
                    None => self.unit_temp(),
                }
            }
        }
    }

    fn lower_call(
        &mut self,
        e: &hir::Expr,
        func: &hir::Callee,
        type_args: &[Ty],
        args: &[hir::Expr],
    ) -> Temp {
        let arg_temps: Vec<Temp> = args.iter().map(|a| self.lower_expr(a)).collect();
        let ret_class = mir_ty(&e.ty);
        let dst = match ret_class {
            MirTy::Unit => None,
            other => Some(self.new_temp(other)),
        };
        match func {
            hir::Callee::Def(def) => {
                let callee = self.callee_name(*def, type_args);
                self.push(Stmt::Call {
                    dst,
                    callee,
                    args: arg_temps,
                });
            }
            hir::Callee::Builtin(b) => {
                // Exhaustive rather than a `_` fallback so a new builtin has to
                // decide here how it is implemented; every arm is a deliberate
                // choice and none of them means "unhandled".
                let how = match b {
                    Builtin::Println => Lowering::Runtime(RtFunc::Println),
                    Builtin::Print => Lowering::Runtime(RtFunc::Print),
                    Builtin::EPrint => Lowering::Runtime(RtFunc::EPrint),
                    Builtin::EPrintln => Lowering::Runtime(RtFunc::EPrintln),
                    Builtin::Panic => Lowering::Runtime(RtFunc::Panic),
                    Builtin::StrCmp => Lowering::Runtime(RtFunc::StrCmp),
                    Builtin::StrHash => Lowering::Runtime(RtFunc::StrHash),
                    Builtin::CharToInt => Lowering::Reinterpret,
                    Builtin::StrLenChars => Lowering::Runtime(RtFunc::StrLenChars),
                    Builtin::StrChars => Lowering::Runtime(RtFunc::StrChars),
                    Builtin::StrFromChars => Lowering::Runtime(RtFunc::StrFromChars),
                    Builtin::StrToUpper => Lowering::Runtime(RtFunc::StrToUpper),
                    Builtin::StrToLower => Lowering::Runtime(RtFunc::StrToLower),
                    Builtin::TestSelector => Lowering::Runtime(RtFunc::TestSelector),
                    Builtin::TaskSpawn => Lowering::Runtime(RtFunc::TaskSpawn),
                    Builtin::TaskIsDone => Lowering::Runtime(RtFunc::TaskIsDone),
                    Builtin::TaskRelease => Lowering::Runtime(RtFunc::TaskRelease),
                    // `task_drive` is typed `-> unit`, so `dst` is `None` and
                    // the executor's `i64` return is discarded. That is the
                    // point: the value it returns is the root future's output
                    // slot as an `i64`, and `block_on`'s result reaches Nova
                    // through `task_output` instead, in the output's own
                    // machine class.
                    Builtin::TaskDrive => Lowering::Runtime(RtFunc::TaskBlockOn),
                    Builtin::TaskYieldFuture => Lowering::Runtime(RtFunc::TaskYieldFuture),
                    Builtin::TaskSleepFuture => Lowering::Runtime(RtFunc::TaskSleepFuture),
                    Builtin::TaskJoinFuture => Lowering::Runtime(RtFunc::TaskJoinFuture),
                    Builtin::TaskOutput => Lowering::FutureOutput,
                    Builtin::FsReadToString => Lowering::Runtime(RtFunc::FsReadToString),
                    Builtin::FsWriteString => Lowering::Runtime(RtFunc::FsWriteString),
                    Builtin::FsTakeString => Lowering::Runtime(RtFunc::FsTakeString),
                    Builtin::FsLastErrorMessage => Lowering::Runtime(RtFunc::FsLastErrorMessage),
                    Builtin::FsTempDir => Lowering::Runtime(RtFunc::FsTempDir),
                    Builtin::FsExists => Lowering::Runtime(RtFunc::FsExists),
                    Builtin::FsCreateDir => Lowering::Runtime(RtFunc::FsCreateDir),
                    Builtin::FsCreateDirAll => Lowering::Runtime(RtFunc::FsCreateDirAll),
                    Builtin::FsRemoveFile => Lowering::Runtime(RtFunc::FsRemoveFile),
                    Builtin::FsRemoveDirAll => Lowering::Runtime(RtFunc::FsRemoveDirAll),
                    Builtin::FsReadDir => Lowering::Runtime(RtFunc::FsReadDir),
                    Builtin::FsTakeStringArray => Lowering::Runtime(RtFunc::FsTakeStringArray),
                    Builtin::FsKind => Lowering::Runtime(RtFunc::FsKind),
                    Builtin::FsRead => Lowering::Runtime(RtFunc::FsRead),
                    Builtin::FsTakeBytes => Lowering::Runtime(RtFunc::FsTakeBytes),
                    Builtin::FsWrite => Lowering::Runtime(RtFunc::FsWrite),
                    Builtin::FileOpen => Lowering::Runtime(RtFunc::FileOpen),
                    Builtin::FileClose => Lowering::Runtime(RtFunc::FileClose),
                    Builtin::FileRead => Lowering::Runtime(RtFunc::FileRead),
                    Builtin::FileWrite => Lowering::Runtime(RtFunc::FileWrite),
                    Builtin::FileFlush => Lowering::Runtime(RtFunc::FileFlush),
                    Builtin::IoStdinRead => Lowering::Runtime(RtFunc::IoStdinRead),
                    Builtin::IoStdoutWrite => Lowering::Runtime(RtFunc::IoStdoutWrite),
                    Builtin::IoStderrWrite => Lowering::Runtime(RtFunc::IoStderrWrite),
                    Builtin::IoStdoutFlush => Lowering::Runtime(RtFunc::IoStdoutFlush),
                    Builtin::IoStderrFlush => Lowering::Runtime(RtFunc::IoStderrFlush),
                    Builtin::BytesLen => Lowering::Runtime(RtFunc::BytesLen),
                    Builtin::BytesFromString => Lowering::Runtime(RtFunc::BytesFromString),
                    Builtin::BytesIsUtf8 => Lowering::Runtime(RtFunc::BytesIsUtf8),
                    Builtin::BytesToStringUnchecked => {
                        Lowering::Runtime(RtFunc::BytesToStringUnchecked)
                    }
                    Builtin::BytesAt => Lowering::Runtime(RtFunc::BytesAt),
                    Builtin::BytesSlice => Lowering::Runtime(RtFunc::BytesSlice),
                    Builtin::BytesConcat => Lowering::Runtime(RtFunc::BytesConcat),
                    Builtin::BytesToInts => Lowering::Runtime(RtFunc::BytesToInts),
                    Builtin::BytesFromInts => Lowering::Runtime(RtFunc::BytesFromInts),
                    Builtin::BytesEq => Lowering::Runtime(RtFunc::BytesEq),
                };
                match how {
                    Lowering::Runtime(func) => self.push(Stmt::CallRuntime {
                        dst,
                        func,
                        args: arg_temps,
                    }),
                    // `char_to_int` is a representation-level no-op: `Ty::Char`
                    // and `Ty::Int` are both `MirTy::I64` (a Char *is* its
                    // Unicode scalar value at runtime), so the conversion is a
                    // register move. Giving it a runtime symbol would mean
                    // adding an ABI function whose body is the identity —
                    // permanent surface area for nothing.
                    //
                    // The `Copy` is not optional and must not be skipped:
                    // `dst` was allocated above and is returned below, so
                    // emitting nothing would leave every later reader of the
                    // result reading an *unassigned* temp — a miscompile, not
                    // an error. `builtin_signature` types this builtin as one
                    // argument returning `Int`, so `dst` is always `Some` and
                    // `arg_temps` always has length 1; the assertions state
                    // that rather than letting an impossible shape pass
                    // quietly (the whole suite runs in debug, so a violation
                    // fails immediately and visibly).
                    Lowering::Reinterpret => {
                        debug_assert!(
                            dst.is_some() && arg_temps.len() == 1,
                            "`{}` must lower with a dst and exactly one argument \
                             (got dst={:?}, {} args), or its result temp is left \
                             unassigned",
                            b.name(),
                            dst,
                            arg_temps.len(),
                        );
                        if let (Some(dst), [src]) = (dst, arg_temps.as_slice()) {
                            self.push(Stmt::Copy { dst, src: *src });
                        }
                    }
                    // `task_output(fut)`: word `FUTURE_SLOT_STATE` of the future
                    // is its state object, and `STATE_SLOT_OUTPUT` of that is
                    // the value the poll function wrote when it completed — the
                    // same two loads `async_lower`'s await continuation emits,
                    // against the same declared offsets, which is why they are
                    // imported from there rather than restated.
                    //
                    // Two typed loads rather than a runtime call because this is
                    // the only form in which the value can arrive in its own
                    // machine class: the executor hands outputs back as `i64`,
                    // and a `Float` output is an `F64`. Loading it under
                    // `ret_class` is what keeps `block_on(half(7.0))` a float
                    // rather than a reinterpreted bit pattern.
                    //
                    // Emitted only when there is somewhere to put the result: a
                    // unit-valued output has no `dst` at all, and both backends
                    // drop a `ty: Unit` access anyway.
                    Lowering::FutureOutput => {
                        debug_assert_eq!(
                            arg_temps.len(),
                            1,
                            "`{}` takes exactly one future",
                            b.name(),
                        );
                        if let (Some(dst), [future]) = (dst, arg_temps.as_slice()) {
                            let state = self.new_temp(MirTy::Ptr);
                            self.push(Stmt::RecordField {
                                dst: state,
                                record: *future,
                                index: crate::async_lower::FUTURE_SLOT_STATE,
                                ty: MirTy::Ptr,
                            });
                            self.push(Stmt::RecordField {
                                dst,
                                record: state,
                                index: crate::async_lower::STATE_SLOT_OUTPUT,
                                ty: ret_class,
                            });
                        }
                    }
                }
            }
            hir::Callee::Local(l) => {
                let callee = self.local_map[l.0 as usize];
                let params: Vec<MirTy> = args.iter().map(|a| mir_ty(&a.ty)).collect();
                self.push(Stmt::CallIndirect {
                    dst,
                    callee,
                    params,
                    ret: ret_class,
                    args: arg_temps,
                });
            }
        }
        dst.unwrap_or_else(|| self.unit_temp())
    }

    /// Lower a trait method call. `self_ty` is concrete here (the enclosing
    /// function was already monomorphized), so we resolve to the impl's
    /// method and emit a direct, statically-dispatched call.
    #[allow(clippy::too_many_arguments)]
    fn lower_trait_call(
        &mut self,
        e: &hir::Expr,
        trait_id: DefId,
        method: u32,
        self_ty: &Ty,
        method_type_args: &[Ty],
        receiver: Option<&hir::Expr>,
        args: &[hir::Expr],
    ) -> Temp {
        // `None` for a trait associated function (`Type::zero()`): the callee has
        // no `self` parameter, so passing one would make the call's arity
        // disagree with the target's signature and codegen would reject the
        // module. Evaluate and pass the receiver only when there is one.
        let mut arg_temps: Vec<Temp> = receiver.map(|r| self.lower_expr(r)).into_iter().collect();
        for a in args {
            arg_temps.push(self.lower_expr(a));
        }

        let ret_class = mir_ty(&e.ty);
        let dst = match ret_class {
            MirTy::Unit => None,
            other => Some(self.new_temp(other)),
        };

        // Resolve to the target function and the type arguments it must be
        // instantiated with — the impl's own generics recovered from the
        // concrete self type for an impl method, or `[self_ty]` for a
        // `Self`-generic trait-default body.
        let Some((target, mut type_args)) =
            self.module.resolve_method_full(trait_id, method, self_ty)
        else {
            let trait_name = self
                .module
                .trait_def(trait_id)
                .map(|t| t.name.clone())
                .unwrap_or_else(|| format!("trait#{}", trait_id.0));
            self.diagnostics.push(
                Diagnostic::error(
                    "E0013",
                    format!("no implementation of trait `{trait_name}` for the receiver type"),
                )
                .with_primary_label(e.span, "no matching trait impl"),
            );
            return dst.unwrap_or_else(|| self.unit_temp());
        };

        // Append the trait method's own generic args after the impl/Self args,
        // so the target is instantiated at the full flat type-arg list that its
        // signature (impl params then method params, or Self then method params)
        // expects.
        type_args.extend(method_type_args.iter().cloned());
        let callee = self.callee_name(target, &type_args);
        self.push(Stmt::Call {
            dst,
            callee,
            args: arg_temps,
        });
        dst.unwrap_or_else(|| self.unit_temp())
    }

    fn lower_binary(&mut self, op: hir::BinOp, lhs: &hir::Expr, rhs: &hir::Expr) -> Temp {
        let l = self.lower_expr(lhs);
        let r = self.lower_expr(rhs);

        // String equality is a runtime call, not a machine instruction.
        //
        // **`Ty::Bytes` deliberately has no arm here.** `binary_result_ty`
        // (`nova-typeck/src/check.rs`) has no `Ty::Bytes` case in its
        // `Eq | Ne` arm either, so `b1 == b2` is `E0013` today, pinned by
        // `bytes_eq_bytes_is_e0013_not_a_silent_pointer_compare`
        // (`nova-typeck/src/check.rs`) — `Bytes` equality is reached only
        // through `impl Eq for Bytes`'s `eq` method
        // (`Builtin::BytesEq`'s doc comment, `nova-resolver`). If a
        // `Ty::Bytes` arm is ever added to that typeck match — the obvious
        // symmetry with `Ty::String` — this `matches!` must gain a matching
        // `Ty::Bytes` arm too, or `==` silently falls through to the generic
        // path below: `operand_class` maps `Ty::Bytes` to
        // `OperandClass::Int`, so `Stmt::Bin { op: Eq, class: Int, .. }` runs
        // on the two `MirTy::Ptr` temps directly — a pointer-identity
        // comparison, not a byte-for-byte one, and silently wrong rather
        // than a compile error.
        if matches!(lhs.ty, Ty::String) && matches!(op, hir::BinOp::Eq | hir::BinOp::Ne) {
            let eq = self.new_temp(MirTy::I8);
            self.push(Stmt::CallRuntime {
                dst: Some(eq),
                func: RtFunc::StrEq,
                args: vec![l, r],
            });
            if matches!(op, hir::BinOp::Eq) {
                return eq;
            }
            let ne = self.new_temp(MirTy::I8);
            self.push(Stmt::Not { dst: ne, src: eq });
            return ne;
        }
        // Unit comparison is a constant.
        if matches!(lhs.ty, Ty::Unit) {
            let t = self.new_temp(MirTy::I8);
            self.push(Stmt::ConstBool(t, matches!(op, hir::BinOp::Eq)));
            return t;
        }

        let class = operand_class(&lhs.ty);
        let result = match op {
            hir::BinOp::Eq
            | hir::BinOp::Ne
            | hir::BinOp::Lt
            | hir::BinOp::Le
            | hir::BinOp::Gt
            | hir::BinOp::Ge => MirTy::I8,
            _ => match class {
                OperandClass::Float => MirTy::F64,
                _ => MirTy::I64,
            },
        };
        let t = self.new_temp(result);
        self.push(Stmt::Bin {
            dst: t,
            op,
            class,
            lhs: l,
            rhs: r,
        });
        t
    }

    fn lower_logical(&mut self, lhs: &hir::Expr, rhs: &hir::Expr, is_and: bool) -> Temp {
        let result = self.new_temp(MirTy::I8);
        let l = self.lower_expr(lhs);
        if diverges(lhs) {
            return result;
        }
        let rhs_block = self.new_block();
        let short_block = self.new_block();
        let join = self.new_block();
        if is_and {
            self.terminate(Terminator::Branch {
                cond: l,
                then_: rhs_block,
                else_: short_block,
            });
        } else {
            self.terminate(Terminator::Branch {
                cond: l,
                then_: short_block,
                else_: rhs_block,
            });
        }
        self.switch_to(rhs_block);
        let r = self.lower_expr(rhs);
        if !diverges(rhs) {
            self.copy(result, r);
        }
        self.terminate(Terminator::Goto(join));
        self.switch_to(short_block);
        self.push(Stmt::ConstBool(result, !is_and));
        self.terminate(Terminator::Goto(join));
        self.switch_to(join);
        result
    }

    fn lower_if(
        &mut self,
        e: &hir::Expr,
        cond: &hir::Expr,
        then: &hir::Expr,
        else_: Option<&hir::Expr>,
    ) -> Temp {
        let result = self.new_temp(mir_ty(&e.ty));
        let c = self.lower_expr(cond);
        if diverges(cond) {
            return result;
        }
        let then_b = self.new_block();
        let join = self.new_block();
        let else_b = if else_.is_some() {
            self.new_block()
        } else {
            join
        };
        self.terminate(Terminator::Branch {
            cond: c,
            then_: then_b,
            else_: else_b,
        });

        self.switch_to(then_b);
        let tv = self.lower_expr(then);
        if !diverges(then) {
            self.copy(result, tv);
        }
        self.terminate(Terminator::Goto(join));

        if let Some(else_expr) = else_ {
            self.switch_to(else_b);
            let ev = self.lower_expr(else_expr);
            if !diverges(else_expr) {
                self.copy(result, ev);
            }
            self.terminate(Terminator::Goto(join));
        }

        self.switch_to(join);
        result
    }

    /// `[init; n]` → allocate `n` slots, then fill every one with `init`.
    ///
    /// The fill loop is built here rather than in codegen, reusing the same
    /// `new_block`/`terminate`/`switch_to` machinery as the `while` arm above,
    /// so both backends need only the new `ArrayAlloc` statement and neither
    /// grows a loop emitter:
    ///
    /// ```text
    ///   guard:  neg = len < 0;  branch neg -> panic, guard2
    ///   panic:  call nova_rt_panic_str("…");  trap
    ///   guard2: big = len > MAX_ARRAY_LEN;  branch big -> panic2, alloc
    ///   panic2: call nova_rt_panic_str("…");  trap
    ///   alloc:  ArrayAlloc { dst: arr, len };  i = 0;  goto header
    ///   header: more = i < len;  branch more -> body, exit
    ///   body:   arr[i] = init;  i = i + 1;  goto header
    ///   exit:   arr
    /// ```
    ///
    /// `init` is lowered **once**, into a single temp that is stored into every
    /// slot. For a heap element type every slot therefore holds the same
    /// pointer — see `hir::ExprKind::ArrayRepeat`.
    fn lower_array_repeat(&mut self, init: &hir::Expr, len: &hir::Expr) -> Temp {
        let init_t = self.lower_expr(init);
        let len_t = self.lower_expr(len);
        let elem_ty = mir_ty(&init.ty);
        let arr = self.new_temp(MirTy::Ptr);

        // Only a length in `0..=MAX_ARRAY_LEN` may reach `ArrayAlloc`. Abort with
        // a message rather than clamping into range: a clamp would hide the
        // mistake here and surface it later as a confusing out-of-bounds abort
        // somewhere that looks fine, and a clamped-to-zero capacity can make a
        // growable collection spin (grow → still full → grow) instead of
        // failing. Same abort-on-bad-input policy as the existing
        // `check_bounds`.
        //
        // Both ends are memory safety, not just hygiene, because the backends'
        // `8 * len + 8` size arithmetic *wraps*: a large-magnitude negative
        // length overflows the multiplication into a wild size, and a length
        // above `MAX_ARRAY_LEN` wraps the size back to negative, which
        // `gc::alloc`'s `size.max(8)` clamps to an 8-byte block that the
        // deliberately unchecked fill loop then runs off the end of.
        let zero = self.new_temp(MirTy::I64);
        self.push(Stmt::ConstInt(zero, 0));
        let negative = self.bin_i64(hir::BinOp::Lt, len_t, zero, MirTy::I8);
        self.panic_if(negative, "array length must not be negative");

        let max = self.new_temp(MirTy::I64);
        self.push(Stmt::ConstInt(max, MAX_ARRAY_LEN));
        let too_long = self.bin_i64(hir::BinOp::Gt, len_t, max, MirTy::I8);
        self.panic_if(too_long, "array length is too large");

        self.push(Stmt::ArrayAlloc {
            dst: arr,
            len: len_t,
        });
        // A unit element has no runtime representation, so there is nothing to
        // store and the loop would spin to no effect. The allocation still
        // carries the right length. Same rule as `MakeArray`/`IndexSet`.
        if elem_ty == MirTy::Unit {
            return arr;
        }
        let i = self.new_temp(MirTy::I64);
        self.push(Stmt::ConstInt(i, 0));

        let header = self.new_block();
        let body_b = self.new_block();
        let exit = self.new_block();
        self.terminate(Terminator::Goto(header));

        self.switch_to(header);
        let more = self.bin_i64(hir::BinOp::Lt, i, len_t, MirTy::I8);
        self.terminate(Terminator::Branch {
            cond: more,
            then_: body_b,
            else_: exit,
        });

        self.switch_to(body_b);
        self.push(Stmt::ArraySet {
            arr,
            index: i,
            value: init_t,
            ty: elem_ty,
        });
        let one = self.new_temp(MirTy::I64);
        self.push(Stmt::ConstInt(one, 1));
        let next = self.bin_i64(hir::BinOp::Add, i, one, MirTy::I64);
        self.push(Stmt::Copy { dst: i, src: next });
        self.terminate(Terminator::Goto(header));

        self.switch_to(exit);
        arr
    }

    /// `if bad { nova_rt_panic_str(msg) }`, continuing in a fresh block.
    ///
    /// Shared by `[init; n]`'s two length guards so both abort through exactly
    /// one code path, with only the message differing.
    fn panic_if(&mut self, bad: Temp, msg: &str) {
        let panic_b = self.new_block();
        let ok_b = self.new_block();
        self.terminate(Terminator::Branch {
            cond: bad,
            then_: panic_b,
            else_: ok_b,
        });

        self.switch_to(panic_b);
        let m = self.new_temp(MirTy::Ptr);
        self.push(Stmt::ConstStr(m, msg.to_string()));
        self.push(Stmt::CallRuntime {
            dst: None,
            func: RtFunc::Panic,
            args: vec![m],
        });
        // `nova_rt_panic_str` aborts, so control never returns here.
        self.terminate(Terminator::Trap);

        self.switch_to(ok_b);
    }

    /// An `Int`-class binary op on two temps, yielding a fresh temp of `result`.
    fn bin_i64(&mut self, op: hir::BinOp, lhs: Temp, rhs: Temp, result: MirTy) -> Temp {
        let dst = self.new_temp(result);
        self.push(Stmt::Bin {
            dst,
            op,
            class: OperandClass::Int,
            lhs,
            rhs,
        });
        dst
    }

    fn lower_match(&mut self, e: &hir::Expr, scrutinee: &hir::Expr, arms: &[hir::Arm]) -> Temp {
        let result = self.new_temp(mir_ty(&e.ty));
        let s = self.lower_expr(scrutinee);
        if diverges(scrutinee) {
            return result;
        }
        let join = self.new_block();

        match &scrutinee.ty {
            Ty::Sum { .. } => {
                let disc = self.new_temp(MirTy::I64);
                self.push(Stmt::SumTag { dst: disc, sum: s });
                self.lower_switch_arms(s, disc, arms, result, join, |pat| match pat {
                    hir::Pattern::Variant { variant, .. } => Some(*variant as i64),
                    _ => None,
                });
            }
            Ty::Int | Ty::Char => {
                self.lower_switch_arms(s, s, arms, result, join, |pat| match pat {
                    hir::Pattern::LitInt(v) => Some(*v),
                    _ => None,
                });
            }
            Ty::Bool => {
                self.lower_switch_arms(s, s, arms, result, join, |pat| match pat {
                    hir::Pattern::LitBool(v) => Some(*v as i64),
                    _ => None,
                });
            }
            Ty::String => {
                self.lower_string_match(s, arms, result, join);
            }
            _ => {
                // Only catch-all arms are possible for other types.
                if let Some(arm) = arms.first() {
                    self.lower_arm_body(s, arm, result);
                    self.terminate(Terminator::Goto(join));
                } else {
                    self.terminate(Terminator::Trap);
                }
            }
        }

        self.switch_to(join);
        result
    }

    /// Lower arms dispatched by an integer discriminant (sum tag, Int,
    /// Char or Bool value). `key` extracts the switch value of a pattern;
    /// `None` marks a catch-all.
    fn lower_switch_arms(
        &mut self,
        scrut: Temp,
        disc: Temp,
        arms: &[hir::Arm],
        result: Temp,
        join: BlockId,
        key: impl Fn(&hir::Pattern) -> Option<i64>,
    ) {
        let mut switch_arms: Vec<(i64, BlockId)> = Vec::new();
        let mut default: Option<BlockId> = None;
        let mut pending: Vec<(BlockId, &hir::Arm)> = Vec::new();

        for arm in arms {
            match key(&arm.pattern) {
                Some(value) => {
                    if switch_arms.iter().any(|(v, _)| *v == value) {
                        continue; // duplicate arm — unreachable
                    }
                    let b = self.new_block();
                    switch_arms.push((value, b));
                    pending.push((b, arm));
                }
                None => {
                    // Catch-all (binding or wildcard) — becomes the default.
                    if default.is_none() {
                        let b = self.new_block();
                        default = Some(b);
                        pending.push((b, arm));
                    }
                    break; // later arms are unreachable
                }
            }
        }

        let default = default.unwrap_or_else(|| {
            let b = self.new_block();
            // Exhaustiveness was checked; this is unreachable.
            let saved = self.current;
            self.switch_to(b);
            self.terminate(Terminator::Trap);
            self.switch_to(saved);
            b
        });
        self.terminate(Terminator::Switch {
            disc,
            arms: switch_arms,
            default,
        });

        for (block, arm) in pending {
            self.switch_to(block);
            self.lower_arm_body(scrut, arm, result);
            self.terminate(Terminator::Goto(join));
        }
    }

    /// String matches lower to a chain of equality tests.
    fn lower_string_match(&mut self, scrut: Temp, arms: &[hir::Arm], result: Temp, join: BlockId) {
        for arm in arms {
            match &arm.pattern {
                hir::Pattern::LitStr(lit) => {
                    let lit_t = self.new_temp(MirTy::Ptr);
                    self.push(Stmt::ConstStr(lit_t, lit.clone()));
                    let eq = self.new_temp(MirTy::I8);
                    self.push(Stmt::CallRuntime {
                        dst: Some(eq),
                        func: RtFunc::StrEq,
                        args: vec![scrut, lit_t],
                    });
                    let body_b = self.new_block();
                    let next = self.new_block();
                    self.terminate(Terminator::Branch {
                        cond: eq,
                        then_: body_b,
                        else_: next,
                    });
                    self.switch_to(body_b);
                    self.lower_arm_body(scrut, arm, result);
                    self.terminate(Terminator::Goto(join));
                    self.switch_to(next);
                }
                _ => {
                    // Catch-all terminates the chain.
                    self.lower_arm_body(scrut, arm, result);
                    self.terminate(Terminator::Goto(join));
                    return;
                }
            }
        }
        // No catch-all: unreachable per exhaustiveness check.
        self.terminate(Terminator::Trap);
    }

    /// Emit binder initialization and the arm body into the current block.
    fn lower_arm_body(&mut self, scrut: Temp, arm: &hir::Arm, result: Temp) {
        match &arm.pattern {
            hir::Pattern::Bind(local) => {
                let dst = self.local_map[local.0 as usize];
                self.copy(dst, scrut);
            }
            hir::Pattern::Variant { binders, .. } => {
                for (i, binder) in binders.iter().enumerate() {
                    if let Some(local) = binder {
                        let dst = self.local_map[local.0 as usize];
                        let ty = self.temps[dst.0 as usize];
                        if ty != MirTy::Unit {
                            let v = self.new_temp(ty);
                            self.push(Stmt::SumField {
                                dst: v,
                                sum: scrut,
                                index: i as u32,
                                ty,
                            });
                            self.push(Stmt::Copy { dst, src: v });
                        }
                    }
                }
            }
            _ => {}
        }
        let v = self.lower_expr(&arm.body);
        if !diverges(&arm.body) {
            self.copy(result, v);
        }
    }
}

fn operand_class(ty: &Ty) -> OperandClass {
    match ty {
        Ty::Float => OperandClass::Float,
        Ty::Bool => OperandClass::Bool,
        _ => OperandClass::Int,
    }
}
