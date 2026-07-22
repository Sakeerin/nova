//! Monomorphization: instantiate generic functions per concrete
//! type-argument list, reachable from `main`.

use nova_diagnostics::Diagnostic;
use nova_hir as hir;
use nova_hir::Ty;
use nova_resolver::DefId;
use rustc_hash::FxHashSet;

use crate::lower::lower_function;
use crate::{mangle, Module};

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

    let mut mir = Module::default();
    let mut done: FxHashSet<String> = FxHashSet::default();
    let mut worklist: Vec<(DefId, Vec<Ty>)> = vec![(main.def_id, Vec::new())];

    while let Some((def_id, type_args)) = worklist.pop() {
        let Some(func) = module.function(def_id) else {
            continue;
        };
        let name = mangle(&func.name, &type_args);
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

        // Specialize the function body for these type arguments.
        let specialized = specialize(func, &type_args);
        let mut request = |def: DefId, args: Vec<Ty>| worklist.push((def, args));
        match lower_function(&specialized, &name, module, &mut request) {
            Ok(f) => mir.functions.push(f),
            Err(d) => diagnostics.extend(d),
        }
    }

    if diagnostics.is_empty() {
        Ok(mir)
    } else {
        Err(diagnostics)
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
    let Some(imp) = module
        .impls
        .iter()
        .find(|im| im.trait_id == Some(trait_id) && im.self_head == head)
    else {
        return false;
    };
    // `arg` must actually fit the impl's self-type pattern, not merely its head.
    let Some(impl_args) = imp.match_args(arg) else {
        return false;
    };
    // For a non-generic or unconstrained impl `bounds` is empty, so this is
    // vacuously true; otherwise every generic parameter's bounds must hold.
    imp.bounds.iter().enumerate().all(|(i, param_bounds)| {
        param_bounds.iter().all(|&bound| {
            impl_args
                .get(i)
                .map(|a| impl_satisfies(module, a, bound))
                .unwrap_or(false)
        })
    })
}

/// A short display name for a type in monomorphization diagnostics.
fn type_name(ty: &Ty, module: &hir::Module) -> String {
    match ty {
        Ty::Int => "Int".to_string(),
        Ty::Float => "Float".to_string(),
        Ty::Bool => "Bool".to_string(),
        Ty::Char => "Char".to_string(),
        Ty::String => "String".to_string(),
        Ty::Unit => "()".to_string(),
        Ty::Sum { def_id, .. } => module
            .sum(*def_id)
            .map(|s| s.name.clone())
            .unwrap_or_else(|| "?".to_string()),
        Ty::Record { def_id, .. } => module
            .record(*def_id)
            .map(|r| r.name.clone())
            .unwrap_or_else(|| "?".to_string()),
        _ => "?".to_string(),
    }
}

/// Clone a function with `Param(i)` replaced by `type_args[i]` throughout
/// locals, signature, and body (including recorded call-site type args).
fn specialize(func: &hir::Function, type_args: &[Ty]) -> hir::Function {
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
                ty: l.ty.subst(type_args),
                is_mut: l.is_mut,
                span: l.span,
            })
            .collect(),
        ret_ty: func.ret_ty.subst(type_args),
        body: subst_expr(&func.body, type_args),
        span: func.span,
    }
}

fn subst_expr(expr: &hir::Expr, args: &[Ty]) -> hir::Expr {
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
            type_args: type_args.iter().map(|t| t.subst(args)).collect(),
            captures: captures.iter().map(|c| subst_expr(c, args)).collect(),
        },
        K::Call {
            func,
            type_args,
            args: call_args,
        } => K::Call {
            func: *func,
            type_args: type_args.iter().map(|t| t.subst(args)).collect(),
            args: call_args.iter().map(|a| subst_expr(a, args)).collect(),
        },
        K::MakeVariant {
            sum,
            variant,
            args: v_args,
        } => K::MakeVariant {
            sum: *sum,
            variant: *variant,
            args: v_args.iter().map(|a| subst_expr(a, args)).collect(),
        },
        K::MakeRecord {
            record,
            fields: rec_fields,
        } => K::MakeRecord {
            record: *record,
            fields: rec_fields.iter().map(|f| subst_expr(f, args)).collect(),
        },
        K::FieldGet { target, index } => K::FieldGet {
            target: Box::new(subst_expr(target, args)),
            index: *index,
        },
        K::MakeArray { elems } => K::MakeArray {
            elems: elems.iter().map(|e| subst_expr(e, args)).collect(),
        },
        K::Index { target, index } => K::Index {
            target: Box::new(subst_expr(target, args)),
            index: Box::new(subst_expr(index, args)),
        },
        K::IndexSet {
            target,
            index,
            value,
        } => K::IndexSet {
            target: Box::new(subst_expr(target, args)),
            index: Box::new(subst_expr(index, args)),
            value: Box::new(subst_expr(value, args)),
        },
        K::ArrayLen { target } => K::ArrayLen {
            target: Box::new(subst_expr(target, args)),
        },
        K::TraitCall {
            trait_id,
            method,
            self_ty,
            receiver,
            args: call_args,
        } => K::TraitCall {
            trait_id: *trait_id,
            method: *method,
            self_ty: self_ty.subst(args),
            receiver: Box::new(subst_expr(receiver, args)),
            args: call_args.iter().map(|a| subst_expr(a, args)).collect(),
        },
        K::Binary { op, lhs, rhs } => K::Binary {
            op: *op,
            lhs: Box::new(subst_expr(lhs, args)),
            rhs: Box::new(subst_expr(rhs, args)),
        },
        K::LogicalAnd { lhs, rhs } => K::LogicalAnd {
            lhs: Box::new(subst_expr(lhs, args)),
            rhs: Box::new(subst_expr(rhs, args)),
        },
        K::LogicalOr { lhs, rhs } => K::LogicalOr {
            lhs: Box::new(subst_expr(lhs, args)),
            rhs: Box::new(subst_expr(rhs, args)),
        },
        K::Unary { op, expr: inner } => K::Unary {
            op: *op,
            expr: Box::new(subst_expr(inner, args)),
        },
        K::Let { local, init } => K::Let {
            local: *local,
            init: Box::new(subst_expr(init, args)),
        },
        K::Assign { local, value } => K::Assign {
            local: *local,
            value: Box::new(subst_expr(value, args)),
        },
        K::Block { stmts, trailing } => K::Block {
            stmts: stmts.iter().map(|s| subst_expr(s, args)).collect(),
            trailing: trailing.as_ref().map(|t| Box::new(subst_expr(t, args))),
        },
        K::If { cond, then, else_ } => K::If {
            cond: Box::new(subst_expr(cond, args)),
            then: Box::new(subst_expr(then, args)),
            else_: else_.as_ref().map(|e| Box::new(subst_expr(e, args))),
        },
        K::While { cond, body } => K::While {
            cond: Box::new(subst_expr(cond, args)),
            body: Box::new(subst_expr(body, args)),
        },
        K::Match { scrutinee, arms } => K::Match {
            scrutinee: Box::new(subst_expr(scrutinee, args)),
            arms: arms
                .iter()
                .map(|a| hir::Arm {
                    pattern: a.pattern.clone(),
                    body: subst_expr(&a.body, args),
                    span: a.span,
                })
                .collect(),
        },
        K::Return(v) => K::Return(v.as_ref().map(|e| Box::new(subst_expr(e, args)))),
        K::ToStr(inner) => K::ToStr(Box::new(subst_expr(inner, args))),
        K::StrConcat(parts) => K::StrConcat(parts.iter().map(|p| subst_expr(p, args)).collect()),
    };
    hir::Expr {
        kind,
        ty: expr.ty.subst(args),
        span: expr.span,
    }
}
