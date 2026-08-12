//! Monomorphization: instantiate generic functions per concrete
//! type-argument list, reachable from `main`.

use nova_diagnostics::Diagnostic;
use nova_hir as hir;
use nova_hir::Ty;
use nova_resolver::DefId;
use rustc_hash::FxHashSet;

use crate::lower::lower_function;
use crate::{mangle, mir_ty, ExternDecl, Function, MirTy, Module, Temp};

/// Lower a typed HIR module to monomorphized MIR.
///
/// Instances are discovered from `main` outward, so unreferenced functions
/// (and never-instantiated generic functions) are not emitted.
pub fn lower_module(module: &hir::Module) -> Result<Module, Vec<Diagnostic>> {
    let mut diagnostics = Vec::new();
    let Some(main) = module.functions.iter().find(|f| f.name == "main") else {
        diagnostics.push(Diagnostic::error(
            "E0601",
            "no `main` function found; `nova run` needs a `fn main()` entry point",
        ));
        return Err(diagnostics);
    };
    if main.generics != 0 || main.params != 0 {
        diagnostics.push(
            Diagnostic::error("E0601", "`main` cannot be generic or take parameters")
                .with_primary_label(main.span, "invalid `main` signature"),
        );
        return Err(diagnostics);
    }

    let entry = main.def_id;
    let mut mir = Module::default();
    let mut done: FxHashSet<String> = FxHashSet::default();
    let mut worklist: Vec<(DefId, Vec<Ty>)> = vec![(entry, Vec::new())];
    // The symbol an `async fn main` was lowered under, once it has been
    // lowered — the future-building wrapper `async_main_shim` calls. `None`
    // for the ordinary case of a non-async entry point.
    let mut async_entry: Option<String> = None;

    while let Some((def_id, type_args)) = worklist.pop() {
        let Some(func) = module.function(def_id) else {
            // Not a Nova function: if it is an `extern` declaration, record it
            // (a leaf — no body to lower) so codegen imports its raw C symbol.
            // Deduped by symbol via the shared `done` set (raw symbols never
            // collide with `name.<defid>`-mangled function names). Two extern
            // declarations that share a symbol but disagree on signature would
            // otherwise collapse to one import and miscompile (or crash codegen)
            // the mismatched caller, so that is rejected here.
            if let Some(ext) = module.extern_fn(def_id) {
                let params: Vec<MirTy> = ext.params.iter().map(mir_ty).collect();
                let ret = mir_ty(&ext.ret);
                if done.insert(ext.symbol.clone()) {
                    mir.externs.push(ExternDecl {
                        symbol: ext.symbol.clone(),
                        params,
                        ret,
                    });
                } else if let Some(prev) = mir.externs.iter().find(|e| e.symbol == ext.symbol) {
                    if prev.params != params || prev.ret != ret {
                        diagnostics.push(
                            Diagnostic::error(
                                "E0075",
                                format!(
                                    "extern symbol `{}` is declared with conflicting signatures",
                                    ext.symbol
                                ),
                            )
                            .with_primary_label(ext.span, "conflicting declaration"),
                        );
                    }
                }
            }
            continue;
        };
        // The entry point keeps the bare symbol `main` (the backends look it up
        // by that name); every other function is mangled with its DefId so that
        // same-named items from different modules never collapse to one symbol.
        //
        // An `async fn main` is the one exception, and it is mangled like any
        // other function: the async transform hands the entry symbol to the
        // future-building *wrapper*, which allocates a state object, returns a
        // future and never polls it, while the backends call `main` for its
        // effects and discard what it returns. So `main` goes to
        // `async_main_shim` below, which is what actually drives the future.
        //
        // Keyed on `is_async`, never on "does `ret_ty` start with `Future`": a
        // non-async function may itself declare `-> Future<T>` and forward an
        // async call's result unchanged (see `wrap_fn_value` in `nova-typeck`),
        // and such a `main` needs no shim — it already returns without
        // suspending.
        let name = if def_id == entry && !func.is_async {
            "main".to_string()
        } else {
            mangle(def_id, &func.name, &type_args)
        };
        if def_id == entry && func.is_async {
            async_entry = Some(name.clone());
        }
        if !done.insert(name.clone()) {
            continue;
        }
        if type_args.iter().any(Ty::has_params) || type_args.iter().any(Ty::has_vars) {
            diagnostics.push(
                Diagnostic::error(
                    "E0011",
                    format!(
                        "cannot monomorphize `{}` with non-concrete type arguments",
                        func.name
                    ),
                )
                .with_primary_label(func.span, "instantiated here"),
            );
            continue;
        }

        // Check that each concrete type argument satisfies its generic
        // parameter's trait bounds (spec 12-TYPESYSTEM §5.4: bounds are
        // verified during monomorphization).
        let mut bounds_ok = true;
        for (i, bounds) in func.bounds.iter().enumerate() {
            let Some(arg) = type_args.get(i) else {
                continue;
            };
            for &trait_id in bounds {
                let satisfied = impl_satisfies(module, arg, trait_id);
                if !satisfied {
                    bounds_ok = false;
                    let trait_name = module
                        .trait_def(trait_id)
                        .map(|t| t.name.clone())
                        .unwrap_or_else(|| format!("trait#{}", trait_id.0));
                    diagnostics.push(
                        Diagnostic::error(
                            "E0013",
                            format!(
                                "trait bound `{}: {}` is not satisfied when instantiating `{}`",
                                type_name(arg, module),
                                trait_name,
                                func.name
                            ),
                        )
                        .with_primary_label(func.span, "required by this generic parameter"),
                    );
                }
            }
        }
        // Skip lowering an instance with unsatisfied bounds: its body would
        // fail to resolve the very trait methods the bound guarantees,
        // producing a duplicate diagnostic for one root cause.
        if !bounds_ok {
            continue;
        }

        // Specialize the function body for these type arguments, resolving every
        // associated-type projection the now-concrete arguments make resolvable
        // (normalization seam 3 — see `Specializer`).
        let mut spec = Specializer {
            args: &type_args,
            impls: &module.impls,
            unresolved: None,
            overflow: None,
        };
        let specialized = spec.function(func);
        // A projection that survives to `mir_ty` is mapped to `MirTy::Unit` by
        // its defensive arm, so the failure is a *unit-typed value where another
        // type was meant* — not a crash. Measured before this seam existed:
        // `unwrap_item(W { v: 7 })` printed `0` and `unwrap_item(W { v: true })`
        // printed `false`, exit code 0, no diagnostic at all; and a projection in
        // a parameter position dropped the parameter from the Cranelift
        // signature, so the caller passed two arguments to a one-argument
        // function and the backend aborted with a verifier error. Both are why
        // this has to be reported here, where a diagnostic is still possible.
        let mut resolved_ok = true;
        if let Some((at, limit)) = spec.overflow {
            resolved_ok = false;
            // The same condition and the same remedy as `Checker::normalize`'s
            // `E0078`, so the same code: it names the error class, not the phase
            // that noticed. A second code for one condition would be worse.
            let detail = match limit {
                hir::NormalizeLimit::Depth => format!(
                    "the chain of bindings is more than {} deep, or does not terminate",
                    hir::NORMALIZE_DEPTH_LIMIT,
                ),
                hir::NormalizeLimit::Steps => format!(
                    "it resolves to more than {} type nodes",
                    hir::NORMALIZE_STEP_LIMIT,
                ),
            };
            diagnostics.push(
                Diagnostic::error(
                    "E0078",
                    format!(
                        "could not resolve the associated types in `{}` when instantiating \
                         `{}`: {}",
                        type_name(&at, module),
                        func.name,
                        detail,
                    ),
                )
                .with_primary_label(func.span, "instantiated here"),
            );
        }
        if let Some(at) = spec.unresolved {
            resolved_ok = false;
            diagnostics.push(
                Diagnostic::error(
                    "E0079",
                    format!(
                        "`{}` is still an unresolved associated type after instantiating `{}`; \
                         no impl in scope binds it for this Self type",
                        type_name(&at, module),
                        func.name,
                    ),
                )
                .with_primary_label(func.span, "instantiated here"),
            );
        }
        // Skip lowering, for the same reason an instance with unsatisfied bounds
        // is skipped: its types are not the types they claim to be, so lowering
        // would add its own diagnostics — or emit the invalid code this check
        // exists to prevent — for one root cause.
        if !resolved_ok {
            continue;
        }
        let mut request = |def: DefId, args: Vec<Ty>| worklist.push((def, args));
        match lower_function(&specialized, &name, module, entry, &mut request) {
            Ok(f) => mir.functions.push(f),
            Err(d) => diagnostics.extend(d),
        }
    }

    if diagnostics.is_empty() {
        // Before the transform, so the shim is in place whichever order the two
        // are read in: it is not `is_async`, so `transform` walks past it.
        if let Some(wrapper) = async_entry {
            mir.functions.push(async_main_shim(&wrapper));
        }
        // Run on the finished module rather than per function inside the loop:
        // it needs no generics resolved and lands once for both codegen
        // backends.
        crate::async_lower::transform(&mut mir);
        debug_assert!(
            mir.functions.iter().all(|f| !f.is_async),
            "the async transform must leave no function flagged: one that reaches \
             codegen still flagged is emitted under its own symbol with its BODY's \
             return class instead of a future's pointer (see `Function::is_async`)"
        );
        debug_assert!(
            !mir.functions
                .iter()
                .flat_map(|f| &f.blocks)
                .flat_map(|b| &b.stmts)
                .any(|s| matches!(s, crate::Stmt::Await { .. })),
            "the async transform must consume every await marker: no codegen \
             backend can emit one, and a function that is not `is_async` cannot \
             have had its awaits split (see `Stmt::Await`)"
        );
        Ok(mir)
    } else {
        Err(diagnostics)
    }
}

/// The `main` an `async fn main` gets: call the future-building wrapper
/// `wrapper`, then hand the future to the executor.
///
/// This is what makes `async fn main` mean anything. An `async fn`'s body is
/// rewritten into a poll function, and the symbol its callers were compiled
/// against goes to a wrapper that returns a `{ poll_code, state }` future
/// without polling it — so an entry point named that way would allocate a state
/// object and exit, running none of the user's code, with exit 0 and no
/// diagnostic. The wrapper is therefore mangled like any other function (see
/// `lower_module`) and the entry symbol goes to this shim instead.
///
/// Built here rather than in the driver, and as MIR rather than as a
/// synthesized `hir::Function`, so that every caller of `lower_module` gets it:
/// `nova check`, `nova run`, `nova build` and this crate's own tests all reach
/// the same shim, and none of them has to know that an entry point might need
/// wrapping.
///
/// `Stmt::CallRuntime`'s `dst` is `None`, so the executor's `i64` return —
/// which is the future's output slot — is discarded. That matches what happens
/// to a non-async `fn main`'s return value, and it is why this shim does not
/// need to know the output's machine class.
fn async_main_shim(wrapper: &str) -> Function {
    let future = Temp(0);
    Function {
        name: "main".to_string(),
        params: 0,
        takes_env: false,
        capture_count: 0,
        temps: vec![MirTy::Ptr],
        ret: MirTy::Unit,
        is_async: false,
        blocks: vec![crate::Block {
            stmts: vec![
                crate::Stmt::Call {
                    dst: Some(future),
                    callee: wrapper.to_string(),
                    args: Vec::new(),
                },
                crate::Stmt::CallRuntime {
                    dst: None,
                    func: crate::RtFunc::TaskBlockOn,
                    args: vec![future],
                },
            ],
            term: crate::Terminator::Return(None),
        }],
    }
}

/// Whether concrete type `arg` implements `trait_id`. The matching impl's self
/// type must fit `arg` structurally (so `impl Foo for Pair<Int, Int>` does not
/// cover `Pair<Int, Bool>`); a generic impl (`impl<T: Bound> Trait for Box<T>`)
/// additionally requires its own generic bounds to hold for the type arguments
/// recovered from `arg`.
///
/// This recursion is well-founded and needs no depth cap: each recursive call
/// checks a bound against a type argument that is a strict sub-term of `arg`
/// (impl self types are always named constructors, so recovered arguments are
/// proper components), and a finite type has finite nesting.
fn impl_satisfies(module: &hir::Module, arg: &Ty, trait_id: DefId) -> bool {
    let Some(head) = arg.head() else {
        return false;
    };
    // Any impl of the trait for this head that both fits `arg` structurally
    // and whose own generic bounds hold satisfies the requirement. Selecting
    // by head alone (and committing to the first) would miss a later impl that
    // actually fits, and coherence guarantees at most one fits anyway.
    module
        .impls
        .iter()
        .filter(|im| im.trait_id == Some(trait_id) && im.self_head == head)
        .any(|imp| match imp.match_args(arg) {
            // For a non-generic or unconstrained impl `bounds` is empty, so
            // the fit alone suffices; otherwise every parameter's bounds hold.
            Some(impl_args) => imp.bounds.iter().enumerate().all(|(i, param_bounds)| {
                param_bounds.iter().all(|&bound| {
                    impl_args
                        .get(i)
                        .map(|a| impl_satisfies(module, a, bound))
                        .unwrap_or(false)
                })
            }),
            None => false,
        })
}

/// A short display name for a type in monomorphization diagnostics.
///
/// `nova-typeck`'s `display_ty` is the richer renderer but it needs
/// `&Definitions`, which mono does not have; this reads names out of the HIR
/// module instead. A projection is spelled `<on>::Name`, matching `display_ty`,
/// by finding the owning trait — the associated type's `DefId` belongs to
/// exactly one, so the scan is unambiguous.
fn type_name(ty: &Ty, module: &hir::Module) -> String {
    match ty {
        Ty::Int => "Int".to_string(),
        Ty::Float => "Float".to_string(),
        Ty::Bool => "Bool".to_string(),
        Ty::Char => "Char".to_string(),
        Ty::String => "String".to_string(),
        Ty::Bytes => "Bytes".to_string(),
        Ty::Unit => "()".to_string(),
        Ty::Sum { def_id, .. } => module
            .sum(*def_id)
            .map(|s| s.name.clone())
            .unwrap_or_else(|| "?".to_string()),
        Ty::Record { def_id, .. } => module
            .record(*def_id)
            .map(|r| r.name.clone())
            .unwrap_or_else(|| "?".to_string()),
        Ty::Array(elem) => format!("[{}]", type_name(elem, module)),
        Ty::Future(out) => format!("Future<{}>", type_name(out, module)),
        Ty::Assoc { on, assoc } => {
            let name = module
                .traits
                .iter()
                .flat_map(|t| t.assoc_types.iter())
                .find(|(_, d)| d == assoc)
                .map(|(n, _)| n.as_str())
                .unwrap_or("?");
            format!("{}::{}", type_name(on, module), name)
        }
        _ => "?".to_string(),
    }
}

/// `subst` followed by normalization — **normalization seam 3** (design doc
/// §4.1), and the reason it belongs here rather than in `nova-typeck`.
///
/// Inside a generic body a projection on a generic parameter is genuinely
/// abstract: `Assoc { on: Param(0) }` has no [`Ty::head`], so no impl can be
/// selected and seam 1 correctly leaves it alone. Monomorphization is the first
/// point where `Param(0)` is known, so `subst` turns it into
/// `Assoc { on: W<Int> }` — which does have a head — and normalization resolves
/// it to the impl's binding. Both halves have to happen together at every type
/// in the instance; `subst` alone is what let a projection reach `mir_ty`.
///
/// Carries the *first* failure of each kind rather than reporting per type: one
/// unresolvable projection appears in a parameter type, in the corresponding
/// local, and again on every expression node that mentions it, so reporting at
/// each site would give one root cause a dozen diagnostics.
struct Specializer<'a> {
    /// The instance's type arguments, indexed by `Param`.
    args: &'a [Ty],
    /// The whole impl table. `normalize_ty` takes a plain `&[hir::ImplInfo]`
    /// slice precisely so this crate can pass `&module.impls` and share the
    /// identical code with `Checker::normalize`'s `&self.impls` — two copies of
    /// projection resolution would be free to disagree silently about a *type*.
    impls: &'a [hir::ImplInfo],
    /// A projection that normalization could not resolve even with concrete
    /// type arguments in hand. Reaching `mir_ty` with one is a miscompile
    /// (§9 risk 1), so it must be a diagnostic.
    unresolved: Option<Ty>,
    /// Normalization ran out of budget: `(the type asked about, which limit)`.
    overflow: Option<(Ty, hir::NormalizeLimit)>,
}

impl Specializer<'_> {
    fn ty(&mut self, ty: &Ty) -> Ty {
        let substituted = ty.subst(self.args);
        // Not observable — `normalize_ty` on a projection-free type returns an
        // equal value — but it is a real cost here in a way it is not at the
        // other two seams: this runs over *every* type of every local and every
        // expression node of every instance, where conformance runs over a
        // handful of signatures. Every Nova program compiles all of std, so the
        // saved tree clones are not hypothetical.
        if !substituted.has_assoc() {
            return substituted;
        }
        match hir::normalize_ty(&substituted, self.impls) {
            Ok(resolved) => {
                if resolved.has_assoc() && self.unresolved.is_none() {
                    self.unresolved = Some(resolved.clone());
                }
                resolved
            }
            Err(hir::NormalizeOverflow { at, limit }) => {
                if self.overflow.is_none() {
                    self.overflow = Some((at, limit));
                }
                // Returned unresolved rather than as `Ty::Error`: the caller
                // skips lowering this instance entirely because a diagnostic was
                // recorded, so nothing reads it, and `Ty::Error` would be a
                // second lie on top of the one being reported.
                substituted
            }
        }
    }

    /// Clone a function with every type substituted and normalized: locals,
    /// signature, and body, including the type arguments recorded at nested call
    /// sites — those pick the callee's instance, so a projection left in one
    /// would monomorphize the wrong function.
    fn function(&mut self, func: &hir::Function) -> hir::Function {
        hir::Function {
            def_id: func.def_id,
            name: func.name.clone(),
            generics: 0,
            bounds: Vec::new(),
            takes_env: func.takes_env,
            capture_count: func.capture_count,
            params: func.params,
            locals: func
                .locals
                .iter()
                .map(|l| hir::Local {
                    name: l.name.clone(),
                    ty: self.ty(&l.ty),
                    is_mut: l.is_mut,
                    span: l.span,
                })
                .collect(),
            ret_ty: self.ty(&func.ret_ty),
            is_async: func.is_async,
            body: self.expr(&func.body),
            span: func.span,
        }
    }
}

/// Helper methods so a struct literal below can call several of them in one
/// expression: each returns an owned value, so its `&mut self` borrow ends with
/// the call.
impl Specializer<'_> {
    fn tys(&mut self, tys: &[Ty]) -> Vec<Ty> {
        tys.iter().map(|t| self.ty(t)).collect()
    }

    fn exprs(&mut self, exprs: &[hir::Expr]) -> Vec<hir::Expr> {
        exprs.iter().map(|e| self.expr(e)).collect()
    }

    fn boxed(&mut self, expr: &hir::Expr) -> Box<hir::Expr> {
        Box::new(self.expr(expr))
    }

    fn opt(&mut self, expr: Option<&hir::Expr>) -> Option<Box<hir::Expr>> {
        expr.map(|e| self.boxed(e))
    }

    fn expr(&mut self, expr: &hir::Expr) -> hir::Expr {
        use hir::ExprKind as K;
        let kind = match &expr.kind {
            K::IntLit(v) => K::IntLit(*v),
            K::FloatLit(v) => K::FloatLit(*v),
            K::BoolLit(v) => K::BoolLit(*v),
            K::StrLit(v) => K::StrLit(v.clone()),
            K::CharLit(v) => K::CharLit(*v),
            K::Unit => K::Unit,
            K::Break => K::Break,
            K::Continue => K::Continue,
            K::Local(l) => K::Local(*l),
            K::MakeClosure {
                func,
                type_args,
                captures,
            } => K::MakeClosure {
                func: *func,
                type_args: self.tys(type_args),
                captures: self.exprs(captures),
            },
            K::Call {
                func,
                type_args,
                args: call_args,
            } => K::Call {
                func: *func,
                type_args: self.tys(type_args),
                args: self.exprs(call_args),
            },
            K::MakeVariant {
                sum,
                variant,
                args: v_args,
            } => K::MakeVariant {
                sum: *sum,
                variant: *variant,
                args: self.exprs(v_args),
            },
            K::MakeRecord {
                record,
                fields: rec_fields,
            } => K::MakeRecord {
                record: *record,
                fields: self.exprs(rec_fields),
            },
            K::FieldGet { target, index } => K::FieldGet {
                target: self.boxed(target),
                index: *index,
            },
            K::FieldSet {
                target,
                index,
                value,
            } => K::FieldSet {
                target: self.boxed(target),
                index: *index,
                value: self.boxed(value),
            },
            K::MakeArray { elems } => K::MakeArray {
                elems: self.exprs(elems),
            },
            K::ArrayRepeat { init, len } => K::ArrayRepeat {
                init: self.boxed(init),
                len: self.boxed(len),
            },
            K::Index { target, index } => K::Index {
                target: self.boxed(target),
                index: self.boxed(index),
            },
            K::IndexSet {
                target,
                index,
                value,
            } => K::IndexSet {
                target: self.boxed(target),
                index: self.boxed(index),
                value: self.boxed(value),
            },
            K::ArrayLen { target } => K::ArrayLen {
                target: self.boxed(target),
            },
            K::TraitCall {
                trait_id,
                method,
                self_ty,
                type_args,
                receiver,
                args: call_args,
            } => K::TraitCall {
                trait_id: *trait_id,
                method: *method,
                self_ty: self.ty(self_ty),
                type_args: self.tys(type_args),
                // `None` for a trait associated function; substitution must not
                // invent a receiver for one.
                receiver: self.opt(receiver.as_deref()),
                args: self.exprs(call_args),
            },
            K::Binary { op, lhs, rhs } => K::Binary {
                op: *op,
                lhs: self.boxed(lhs),
                rhs: self.boxed(rhs),
            },
            K::LogicalAnd { lhs, rhs } => K::LogicalAnd {
                lhs: self.boxed(lhs),
                rhs: self.boxed(rhs),
            },
            K::LogicalOr { lhs, rhs } => K::LogicalOr {
                lhs: self.boxed(lhs),
                rhs: self.boxed(rhs),
            },
            K::Unary { op, expr: inner } => K::Unary {
                op: *op,
                expr: self.boxed(inner),
            },
            K::Let { local, init } => K::Let {
                local: *local,
                init: self.boxed(init),
            },
            K::Assign { local, value } => K::Assign {
                local: *local,
                value: self.boxed(value),
            },
            K::Block { stmts, trailing } => K::Block {
                stmts: self.exprs(stmts),
                trailing: self.opt(trailing.as_deref()),
            },
            K::If { cond, then, else_ } => K::If {
                cond: self.boxed(cond),
                then: self.boxed(then),
                else_: self.opt(else_.as_deref()),
            },
            K::While { cond, body } => K::While {
                cond: self.boxed(cond),
                body: self.boxed(body),
            },
            K::Match { scrutinee, arms } => K::Match {
                scrutinee: self.boxed(scrutinee),
                arms: arms
                    .iter()
                    .map(|a| hir::Arm {
                        pattern: a.pattern.clone(),
                        body: self.expr(&a.body),
                        span: a.span,
                    })
                    .collect(),
            },
            K::Return(v) => K::Return(self.opt(v.as_deref())),
            K::ToStr(inner) => K::ToStr(self.boxed(inner)),
            K::StrConcat(parts) => K::StrConcat(self.exprs(parts)),
            K::Await(inner) => K::Await(self.boxed(inner)),
        };
        hir::Expr {
            kind,
            ty: self.ty(&expr.ty),
            span: expr.span,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nova_diagnostics::{FileId, Span};
    use nova_hir::{Callee, Expr, ExprKind as K, ImplInfo, Local, TraitDef, TyHead};

    const TRAIT: DefId = DefId(10);
    const ITEM: DefId = DefId(11);
    const OTHER: DefId = DefId(12);
    const REC: DefId = DefId(13);
    const MAIN: DefId = DefId(14);
    const F: DefId = DefId(15);

    fn dummy_span() -> Span {
        Span::point(0, FileId::DUMMY)
    }

    fn w() -> Ty {
        Ty::Record {
            def_id: REC,
            args: Vec::new(),
        }
    }

    fn expr(kind: K, ty: Ty) -> Expr {
        Expr {
            kind,
            ty,
            span: dummy_span(),
        }
    }

    /// A module where `main` calls `f::<W>`, and `f` holds one local whose
    /// declared type is the projection `Param(0)::Item`.
    ///
    /// `binding` is what `impl ? for W` binds, keyed by associated type — so
    /// passing `ITEM` gives an impl that answers the projection and passing
    /// `OTHER` gives one that shares the head but binds something else. That is
    /// the difference between a projection this seam resolves and one that
    /// survives it, with everything else held equal.
    fn module_with(binding: DefId) -> hir::Module {
        // `let y = 5` where `y: Param(0)::Item`. Nothing else, so the only thing
        // under test is what that type becomes.
        let body = expr(
            K::Block {
                stmts: vec![expr(
                    K::Let {
                        local: nova_hir::LocalId(0),
                        init: Box::new(expr(K::IntLit(5), Ty::Int)),
                    },
                    Ty::Unit,
                )],
                trailing: None,
            },
            Ty::Unit,
        );
        let f = hir::Function {
            def_id: F,
            name: "f".to_string(),
            generics: 1,
            // No bounds: `E0013` is a different check and would mask this one.
            bounds: vec![Vec::new()],
            takes_env: false,
            capture_count: 0,
            params: 0,
            locals: vec![Local {
                name: "y".to_string(),
                ty: Ty::Assoc {
                    on: Box::new(Ty::Param(0)),
                    assoc: ITEM,
                },
                is_mut: false,
                span: dummy_span(),
            }],
            ret_ty: Ty::Unit,
            is_async: false,
            body,
            span: dummy_span(),
        };
        let main = hir::Function {
            def_id: MAIN,
            name: "main".to_string(),
            generics: 0,
            bounds: Vec::new(),
            takes_env: false,
            capture_count: 0,
            params: 0,
            locals: Vec::new(),
            ret_ty: Ty::Unit,
            is_async: false,
            body: expr(
                K::Block {
                    stmts: vec![expr(
                        K::Call {
                            func: Callee::Def(F),
                            type_args: vec![w()],
                            args: Vec::new(),
                        },
                        Ty::Unit,
                    )],
                    trailing: None,
                },
                Ty::Unit,
            ),
            span: dummy_span(),
        };
        hir::Module {
            sums: Vec::new(),
            records: vec![hir::RecordType {
                def_id: REC,
                name: "W".to_string(),
                generics: 0,
                fields: Vec::new(),
            }],
            traits: vec![TraitDef {
                def_id: TRAIT,
                name: "It".to_string(),
                supertraits: Vec::new(),
                methods: Vec::new(),
                assoc_types: vec![("Item".to_string(), ITEM)],
            }],
            impls: vec![ImplInfo {
                trait_id: Some(TRAIT),
                self_head: TyHead::Record(REC),
                self_ty: w(),
                generics: 0,
                bounds: Vec::new(),
                methods: Vec::new(),
                assoc_bindings: vec![(binding, Ty::Int)],
            }],
            functions: vec![main, f],
            externs: Vec::new(),
        }
    }

    #[test]
    fn a_projection_on_a_generic_parameter_resolves_once_the_argument_is_known() {
        // The positive half of seam 3, at the level the diagnostic lives: `f`'s
        // local is declared `Param(0)::Item`, the instance is `f::<W>`, and the
        // impl binds `Item = Int`. So the lowered temp must be `I64`.
        //
        // `MirTy::Unit` is the discriminating value, not an arbitrary one: it is
        // exactly what `mir_ty`'s defensive arm returns for an unresolved
        // `Assoc`, so this assertion fails precisely when the projection reaches
        // codegen unresolved.
        let module = module_with(ITEM);
        let mir = lower_module(&module).expect("no diagnostics");
        let f = mir
            .functions
            .iter()
            .find(|f| f.name.starts_with("f."))
            .expect("f was instantiated");
        assert_eq!(
            f.temps.first(),
            Some(&MirTy::I64),
            "W::Item is Int, so the local is an i64, not a dropped unit"
        );
    }

    #[test]
    fn a_projection_that_survives_monomorphization_is_a_diagnostic() {
        // The same module with the impl binding a *different* associated type, so
        // no binding answers `W::Item` and normalization returns the projection
        // unchanged. Without the check this reaches `mir_ty`, which maps it to
        // `MirTy::Unit` — a unit-typed value where an `Int` was meant, with no
        // diagnostic at all. That is spec §9's risk 1, and it is the one failure
        // mode this seam exists to make impossible.
        //
        // Constructed here rather than in a `.nova` file because **it is not
        // reachable from source**. Seven probes were tried and each is closed by
        // an earlier diagnostic: an unresolvable projection at a concrete call
        // site is `E0010` from seam 1, a supertrait whose impl does not fit
        // structurally is `E0072`, and an impl generic that the self type does
        // not mention is `E0073` — that last one being the only route that would
        // otherwise reach here, via `match_args` resolving the unused parameter
        // to `Ty::Error`. Instrumenting this branch and running the whole suite
        // reached it zero times. So it is a backstop, and a backstop with no test
        // is exactly what Task 5's review found twice.
        let module = module_with(OTHER);
        let diagnostics = lower_module(&module).expect_err("a surviving projection");
        let codes: Vec<&str> = diagnostics.iter().map(|d| d.code.as_str()).collect();
        assert_eq!(codes, ["E0079"], "{diagnostics:?}");
        // The message has to name the projection. A diagnostic that says only
        // "could not resolve an associated type" leaves the user with nothing to
        // look at, and `type_name` returning `?` for `Assoc` (as it did before
        // this task) would still satisfy an assertion on the code alone.
        assert!(
            diagnostics[0].message.contains("`W::Item`"),
            "{}",
            diagnostics[0].message
        );
    }

    #[test]
    fn mangle_ty_distinguishes_futures_by_output_type() {
        // A Future's mangled name must depend on its output type. `Ty::Assoc`
        // mangling to a constant "X" already shipped as a miscompile on this
        // project: two instantiations collided on one symbol and both dispatched
        // to the first's code. A constant here reproduces that exactly.
        let a = crate::mangle_ty(&hir::Ty::Future(Box::new(hir::Ty::Int)));
        let b = crate::mangle_ty(&hir::Ty::Future(Box::new(hir::Ty::Float)));
        let c = crate::mangle_ty(&hir::Ty::Future(Box::new(hir::Ty::Bool)));
        assert_ne!(a, b);
        assert_ne!(b, c);
        assert_ne!(a, c);
        // And distinct from the array mangling of the same element type, so the
        // two single-argument constructors cannot collide either.
        assert_ne!(a, crate::mangle_ty(&hir::Ty::Array(Box::new(hir::Ty::Int))));
    }

    #[test]
    fn mir_ty_maps_future_to_ptr() {
        // A future value is the fat pointer { poll_code, state_ptr }.
        // MirTy::Unit would be catastrophic and silent: unit parameters are
        // DROPPED from the Cranelift signature, which is how the 2.2c projection
        // bug produced wrong values with exit 0 and no diagnostic.
        assert_eq!(
            crate::mir_ty(&hir::Ty::Future(Box::new(hir::Ty::Int))),
            crate::MirTy::Ptr
        );
    }
}
