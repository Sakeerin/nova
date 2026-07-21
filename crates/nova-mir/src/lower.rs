//! HIR → MIR lowering for one (already monomorphized) function.

use nova_diagnostics::Diagnostic;
use nova_hir as hir;
use nova_hir::Ty;
use nova_resolver::{Builtin, DefId};

use crate::{
    mangle, mir_ty, Block, BlockId, Function, MirTy, OperandClass, RtFunc, Stmt, Temp, Terminator,
};

/// Lower a specialized (generic-free) HIR function to MIR.
///
/// `request` is called for every direct call / function reference so the
/// monomorphization driver can enqueue the callee instance.
pub(crate) fn lower_function(
    func: &hir::Function,
    mangled: &str,
    module: &hir::Module,
    request: &mut dyn FnMut(DefId, Vec<Ty>),
) -> Result<Function, Vec<Diagnostic>> {
    let mut lo = Lowerer {
        module,
        request,
        temps: Vec::new(),
        blocks: vec![BlockState::default()],
        current: BlockId(0),
        local_map: Vec::new(),
        diagnostics: Vec::new(),
    };

    // Parameters occupy the first temps; remaining locals get fresh temps.
    for (i, local) in func.locals.iter().enumerate() {
        let t = lo.new_temp(mir_ty(&local.ty));
        debug_assert_eq!(t.0 as usize, i, "params must map to leading temps");
        lo.local_map.push(t);
    }

    let result = lo.lower_expr(&func.body);
    if !diverges(&func.body) {
        match mir_ty(&func.ret_ty) {
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
        temps: lo.temps,
        ret: mir_ty(&func.ret_ty),
        blocks,
    })
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
    request: &'a mut dyn FnMut(DefId, Vec<Ty>),
    temps: Vec<MirTy>,
    blocks: Vec<BlockState>,
    current: BlockId,
    /// LocalId index → temp.
    local_map: Vec<Temp>,
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

    /// Copy `src` into `dst` unless the value class is `Unit` (no data).
    fn copy(&mut self, dst: Temp, src: Temp) {
        if self.temps[dst.0 as usize] != MirTy::Unit {
            self.push(Stmt::Copy { dst, src });
        }
    }

    fn callee_name(&mut self, def: DefId, type_args: &[Ty]) -> String {
        let name = self
            .module
            .function(def)
            .map(|f| f.name.clone())
            .unwrap_or_else(|| format!("__unknown_{}", def.0));
        (self.request)(def, type_args.to_vec());
        mangle(&name, type_args)
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
            K::FnRef { def, type_args } => {
                let callee = self.callee_name(*def, type_args);
                let t = self.new_temp(MirTy::Ptr);
                self.push(Stmt::FnAddr { dst: t, callee });
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
                let c = self.lower_expr(cond);
                self.terminate(Terminator::Branch {
                    cond: c,
                    then_: body_b,
                    else_: exit,
                });
                self.switch_to(body_b);
                self.lower_expr(body);
                self.terminate(Terminator::Goto(header));
                self.switch_to(exit);
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
                let rt = match b {
                    Builtin::Println => RtFunc::Println,
                    Builtin::Print => RtFunc::Print,
                };
                self.push(Stmt::CallRuntime {
                    dst,
                    func: rt,
                    args: arg_temps,
                });
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

    fn lower_binary(&mut self, op: hir::BinOp, lhs: &hir::Expr, rhs: &hir::Expr) -> Temp {
        let l = self.lower_expr(lhs);
        let r = self.lower_expr(rhs);

        // String equality is a runtime call, not a machine instruction.
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
