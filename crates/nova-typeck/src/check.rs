//! AST → typed HIR checking: signature collection, body inference,
//! desugaring, and minimal exhaustiveness analysis.

use nova_ast as ast;
use nova_ast::item::TypeDef;
use nova_diagnostics::{Diagnostic, Span, Spanned};
use nova_hir as hir;
use nova_hir::{LocalId, Ty};
use nova_resolver::{Builtin, DefId, DefKind, Definitions, Res};
use rustc_hash::FxHashMap;

use crate::infer::InferCtx;
use crate::{display_ty, CheckResult};

/// A collected function signature.
#[derive(Debug, Clone)]
struct FnSig {
    generics: u32,
    params: Vec<Ty>,
    ret: Ty,
}

/// Type-check a parsed file against its resolved definitions.
pub fn check(file: &ast::File, defs: &Definitions) -> CheckResult {
    let mut checker = Checker {
        file,
        defs,
        sigs: FxHashMap::default(),
        sums: Vec::new(),
        diagnostics: Vec::new(),
    };
    checker.collect_sums();
    checker.collect_signatures();

    let mut functions = Vec::new();
    let fn_ids: Vec<(DefId, usize)> = defs.functions().collect();
    for (def_id, item_index) in fn_ids {
        if let Some(f) = checker.check_function(def_id, item_index) {
            functions.push(f);
        }
    }

    CheckResult {
        module: hir::Module {
            sums: checker.sums,
            functions,
        },
        diagnostics: checker.diagnostics,
    }
}

struct Checker<'a> {
    file: &'a ast::File,
    defs: &'a Definitions,
    sigs: FxHashMap<DefId, FnSig>,
    sums: Vec<hir::SumType>,
    diagnostics: Vec<Diagnostic>,
}

/// Per-function checking state.
struct FnCtx {
    icx: InferCtx,
    locals: Vec<hir::Local>,
    scopes: Vec<FxHashMap<String, LocalId>>,
    /// Generic parameter names of the enclosing function.
    generics: FxHashMap<String, u32>,
    ret_ty: Ty,
}

impl FnCtx {
    fn lookup(&self, name: &str) -> Option<LocalId> {
        self.scopes
            .iter()
            .rev()
            .find_map(|scope| scope.get(name).copied())
    }

    fn new_local(&mut self, name: String, ty: Ty, is_mut: bool, span: Span) -> LocalId {
        let id = LocalId(self.locals.len() as u32);
        self.locals.push(hir::Local {
            name: name.clone(),
            ty,
            is_mut,
            span,
        });
        if name != "_" {
            if let Some(scope) = self.scopes.last_mut() {
                scope.insert(name, id);
            }
        }
        id
    }
}

impl<'a> Checker<'a> {
    // === Collection ===

    fn collect_sums(&mut self) {
        for (i, def) in self.defs.defs().iter().enumerate() {
            let DefKind::Sum { item_index, .. } = &def.kind else {
                continue;
            };
            let ast::Item::Type(decl) = &self.file.items[*item_index].value else {
                continue;
            };
            let TypeDef::Sum(variants) = &decl.def else {
                continue;
            };
            let generics = generic_scope(&decl.generics);
            let variants = variants
                .iter()
                .map(|v| hir::Variant {
                    name: v.name.value.clone(),
                    fields: v
                        .fields
                        .iter()
                        .map(|t| self.convert_ty(t, &generics))
                        .collect(),
                })
                .collect();
            self.sums.push(hir::SumType {
                def_id: DefId(i as u32),
                name: def.name.clone(),
                generics: decl.generics.len() as u32,
                variants,
            });
        }
    }

    fn collect_signatures(&mut self) {
        let fn_ids: Vec<(DefId, usize)> = self.defs.functions().collect();
        for (def_id, item_index) in fn_ids {
            let ast::Item::Function(f) = &self.file.items[item_index].value else {
                continue;
            };
            if f.is_async {
                self.unsupported(f.name.span, "async functions");
            }
            if !f.where_clause.is_empty() || f.generics.iter().any(|g| !g.bounds.is_empty()) {
                self.unsupported(f.name.span, "trait bounds on generics");
            }
            let generics = generic_scope(&f.generics);
            let params = f
                .params
                .iter()
                .map(|p| self.convert_ty(&p.ty, &generics))
                .collect();
            let ret = f
                .return_ty
                .as_ref()
                .map(|t| self.convert_ty(t, &generics))
                .unwrap_or(Ty::Unit);
            self.sigs.insert(
                def_id,
                FnSig {
                    generics: f.generics.len() as u32,
                    params,
                    ret,
                },
            );
        }
        // Constants are a later Phase 1 step.
        for def in self.defs.defs() {
            if let DefKind::Const { .. } = def.kind {
                self.unsupported(def.span, "constants");
            }
        }
    }

    /// Convert an AST type annotation to a `Ty`, resolving names.
    fn convert_ty(&mut self, ty: &Spanned<ast::Type>, generics: &FxHashMap<String, u32>) -> Ty {
        match &ty.value {
            ast::Type::Path { path, args } => {
                if path.segments.len() != 1 {
                    self.unsupported(ty.span, "module-qualified type paths");
                    return Ty::Error;
                }
                let name = path.segments[0].value.as_str();
                if let Some(&idx) = generics.get(name) {
                    if !args.is_empty() {
                        self.error(
                            "E0012",
                            format!("generic parameter `{name}` takes no type arguments"),
                            ty.span,
                        );
                    }
                    return Ty::Param(idx);
                }
                let prim = match name {
                    "Int" => Some(Ty::Int),
                    "Float" => Some(Ty::Float),
                    "Bool" => Some(Ty::Bool),
                    "Char" => Some(Ty::Char),
                    "String" => Some(Ty::String),
                    _ => None,
                };
                if let Some(p) = prim {
                    if !args.is_empty() {
                        self.error(
                            "E0012",
                            format!("`{name}` takes no type arguments"),
                            ty.span,
                        );
                    }
                    return p;
                }
                if let Some(def_id) = self.defs.resolve_type(name) {
                    let expected = self
                        .sums
                        .iter()
                        .find(|s| s.def_id == def_id)
                        .map(|s| s.generics)
                        .unwrap_or(0);
                    let converted: Vec<Ty> = args
                        .iter()
                        .map(|a| self.convert_ty(a, generics))
                        .collect();
                    if converted.len() != expected as usize {
                        self.error(
                            "E0012",
                            format!(
                                "type `{name}` expects {expected} type argument(s), found {}",
                                converted.len()
                            ),
                            ty.span,
                        );
                        return Ty::Error;
                    }
                    return Ty::Sum {
                        def_id,
                        args: converted,
                    };
                }
                self.error("E0001", format!("cannot find type `{name}`"), ty.span);
                Ty::Error
            }
            ast::Type::Tuple(items) if items.is_empty() => Ty::Unit,
            ast::Type::Fn { params, ret } => Ty::Fn {
                params: params
                    .iter()
                    .map(|p| self.convert_ty(p, generics))
                    .collect(),
                ret: Box::new(self.convert_ty(ret, generics)),
            },
            ast::Type::Tuple(_) => {
                self.unsupported(ty.span, "tuple types");
                Ty::Error
            }
            ast::Type::Ref { .. } | ast::Type::Ptr { .. } => {
                self.unsupported(ty.span, "reference and pointer types");
                Ty::Error
            }
            ast::Type::Array(_) => {
                self.unsupported(ty.span, "array types");
                Ty::Error
            }
            ast::Type::Optional(_) => {
                self.unsupported(ty.span, "the `T?` optional sugar");
                Ty::Error
            }
            ast::Type::Infer => {
                self.unsupported(ty.span, "`_` type placeholders");
                Ty::Error
            }
        }
    }

    // === Function bodies ===

    fn check_function(&mut self, def_id: DefId, item_index: usize) -> Option<hir::Function> {
        let ast::Item::Function(f) = &self.file.items[item_index].value else {
            return None;
        };
        let sig = self.sigs.get(&def_id)?.clone();
        let mut fcx = FnCtx {
            icx: InferCtx::default(),
            locals: Vec::new(),
            scopes: vec![FxHashMap::default()],
            generics: generic_scope(&f.generics),
            ret_ty: sig.ret.clone(),
        };
        for (p, ty) in f.params.iter().zip(sig.params.iter()) {
            fcx.new_local(p.name.value.clone(), ty.clone(), p.is_mut, p.name.span);
        }

        let body = self.check_block(&mut fcx, &f.body.value, f.body.span);
        if !fcx.icx.unify(&body.ty, &sig.ret) {
            let span = body_result_span(&f.body);
            self.error(
                "E0010",
                format!(
                    "function `{}` should return `{}` but its body has type `{}`",
                    f.name.value,
                    self.show(&sig.ret, &fcx),
                    self.show(&body.ty, &fcx),
                ),
                span,
            );
        }

        let mut func = hir::Function {
            def_id,
            name: f.name.value.clone(),
            generics: sig.generics,
            params: f.params.len() as u32,
            locals: fcx.locals,
            ret_ty: sig.ret,
            body,
            span: f.name.span,
        };
        self.finalize_function(&mut func, &fcx.icx);
        Some(func)
    }

    /// Apply the final substitution everywhere and report residual
    /// inference variables as E0011.
    fn finalize_function(&mut self, func: &mut hir::Function, icx: &InferCtx) {
        let mut residual: Vec<Span> = Vec::new();
        for local in &mut func.locals {
            local.ty = icx.apply(&local.ty);
            if local.ty.has_vars() {
                residual.push(local.span);
            }
        }
        finalize_expr(&mut func.body, icx, &mut residual);
        for span in residual {
            self.error(
                "E0011",
                "cannot infer the type here; add a type annotation",
                span,
            );
        }
    }

    fn check_block(&mut self, fcx: &mut FnCtx, block: &ast::Block, span: Span) -> hir::Expr {
        fcx.scopes.push(FxHashMap::default());
        let mut stmts = Vec::new();
        for stmt in &block.stmts {
            match &stmt.value {
                ast::Stmt::Let {
                    is_mut,
                    pattern,
                    ty,
                    init,
                } => {
                    let Some(init) = init else {
                        self.unsupported(stmt.span, "`let` without an initializer");
                        continue;
                    };
                    let mut value = self.check_expr(fcx, init);
                    if let Some(annot) = ty {
                        let annot_ty = self.convert_ty(annot, &fcx.generics.clone());
                        if !fcx.icx.unify(&value.ty, &annot_ty) {
                            self.error(
                                "E0010",
                                format!(
                                    "type mismatch: expected `{}`, found `{}`",
                                    self.show(&annot_ty, fcx),
                                    self.show(&value.ty, fcx),
                                ),
                                init.span,
                            );
                        }
                        value.ty = annot_ty;
                    }
                    let (name, pat_mut, name_span) = match &pattern.value {
                        ast::Pattern::Ident { is_mut, name } => {
                            (name.value.clone(), *is_mut, name.span)
                        }
                        ast::Pattern::Wildcard => ("_".to_string(), false, pattern.span),
                        _ => {
                            self.error(
                                "E0022",
                                "only irrefutable patterns (a name or `_`) are allowed in `let`",
                                pattern.span,
                            );
                            ("_".to_string(), false, pattern.span)
                        }
                    };
                    let local =
                        fcx.new_local(name, value.ty.clone(), *is_mut || pat_mut, name_span);
                    stmts.push(hir::Expr {
                        kind: hir::ExprKind::Let {
                            local,
                            init: Box::new(value),
                        },
                        ty: Ty::Unit,
                        span: stmt.span,
                    });
                }
                ast::Stmt::Expr(e) => {
                    stmts.push(self.check_expr(fcx, e));
                }
                ast::Stmt::Item(_) => {
                    self.unsupported(stmt.span, "items nested inside function bodies");
                }
            }
        }
        let trailing = block
            .trailing
            .as_ref()
            .map(|e| Box::new(self.check_expr(fcx, e)));
        fcx.scopes.pop();

        let ty = trailing.as_ref().map(|e| e.ty.clone()).unwrap_or(Ty::Unit);
        hir::Expr {
            kind: hir::ExprKind::Block { stmts, trailing },
            ty,
            span,
        }
    }

    fn check_expr(&mut self, fcx: &mut FnCtx, expr: &Spanned<ast::Expr>) -> hir::Expr {
        let span = expr.span;
        match &expr.value {
            ast::Expr::Lit(lit) => lit_expr(lit, span),
            ast::Expr::StringInterp(parts) => self.check_interp(fcx, parts, span),
            ast::Expr::Path(path) => self.check_path(fcx, path, span),
            ast::Expr::Call { callee, args } => self.check_call(fcx, callee, args, span),
            ast::Expr::Binary { op, lhs, rhs } => self.check_binary(fcx, *op, lhs, rhs, span),
            ast::Expr::Unary { op, expr: inner } => self.check_unary(fcx, *op, inner, span),
            ast::Expr::Block(block) => self.check_block(fcx, block, span),
            ast::Expr::If { cond, then, else_ } => {
                let cond = self.check_expr(fcx, cond);
                self.expect_ty(fcx, &cond, &Ty::Bool, "an `if` condition");
                let then_expr = self.check_block(fcx, &then.value, then.span);
                match else_ {
                    Some(else_branch) => {
                        let else_expr = self.check_expr(fcx, else_branch);
                        if !fcx.icx.unify(&then_expr.ty, &else_expr.ty) {
                            self.error(
                                "E0010",
                                format!(
                                    "`if` and `else` have incompatible types: `{}` vs `{}`",
                                    self.show(&then_expr.ty, fcx),
                                    self.show(&else_expr.ty, fcx),
                                ),
                                else_expr.span,
                            );
                        }
                        let ty = then_expr.ty.clone();
                        hir::Expr {
                            kind: hir::ExprKind::If {
                                cond: Box::new(cond),
                                then: Box::new(then_expr),
                                else_: Some(Box::new(else_expr)),
                            },
                            ty,
                            span,
                        }
                    }
                    None => {
                        if !fcx.icx.unify(&then_expr.ty, &Ty::Unit) {
                            self.error(
                                "E0010",
                                format!(
                                    "an `if` without `else` must have type `()`, found `{}`",
                                    self.show(&then_expr.ty, fcx),
                                ),
                                then_expr.span,
                            );
                        }
                        hir::Expr {
                            kind: hir::ExprKind::If {
                                cond: Box::new(cond),
                                then: Box::new(then_expr),
                                else_: None,
                            },
                            ty: Ty::Unit,
                            span,
                        }
                    }
                }
            }
            ast::Expr::While { cond, body } => {
                let cond = self.check_expr(fcx, cond);
                self.expect_ty(fcx, &cond, &Ty::Bool, "a `while` condition");
                let body = self.check_block(fcx, &body.value, body.span);
                hir::Expr {
                    kind: hir::ExprKind::While {
                        cond: Box::new(cond),
                        body: Box::new(body),
                    },
                    ty: Ty::Unit,
                    span,
                }
            }
            ast::Expr::Match { scrutinee, arms } => self.check_match(fcx, scrutinee, arms, span),
            ast::Expr::Return(value) => {
                let value = value.as_ref().map(|v| self.check_expr(fcx, v));
                let value_ty = value.as_ref().map(|v| v.ty.clone()).unwrap_or(Ty::Unit);
                let ret_ty = fcx.ret_ty.clone();
                if !fcx.icx.unify(&value_ty, &ret_ty) {
                    self.error(
                        "E0010",
                        format!(
                            "`return` value has type `{}` but the function returns `{}`",
                            self.show(&value_ty, fcx),
                            self.show(&ret_ty, fcx),
                        ),
                        span,
                    );
                }
                hir::Expr {
                    kind: hir::ExprKind::Return(value.map(Box::new)),
                    ty: Ty::Never,
                    span,
                }
            }
            ast::Expr::Assign { op, lhs, rhs } => self.check_assign(fcx, *op, lhs, rhs, span),
            ast::Expr::Tuple(items) if items.is_empty() => hir::Expr {
                kind: hir::ExprKind::Unit,
                ty: Ty::Unit,
                span,
            },
            // --- deferred constructs ---
            ast::Expr::Tuple(_) => self.unsupported_expr(span, "tuple expressions"),
            ast::Expr::Array(_) => self.unsupported_expr(span, "array literals"),
            ast::Expr::For { .. } => self.unsupported_expr(span, "`for` loops"),
            ast::Expr::Break(_) => self.unsupported_expr(span, "`break`"),
            ast::Expr::Continue => self.unsupported_expr(span, "`continue`"),
            ast::Expr::Closure { .. } => self.unsupported_expr(span, "closures"),
            ast::Expr::Record { .. } => self.unsupported_expr(span, "record literals"),
            ast::Expr::Index { .. } => self.unsupported_expr(span, "indexing"),
            ast::Expr::Field { .. } => self.unsupported_expr(span, "field access"),
            ast::Expr::Try(_) => self.unsupported_expr(span, "the `?` operator"),
            ast::Expr::Await(_) => self.unsupported_expr(span, "`.await`"),
            ast::Expr::Cast { .. } => self.unsupported_expr(span, "`as` casts"),
        }
    }

    fn check_interp(
        &mut self,
        fcx: &mut FnCtx,
        parts: &[ast::StringPart],
        span: Span,
    ) -> hir::Expr {
        let mut pieces = Vec::new();
        for part in parts {
            match part {
                ast::StringPart::Lit(s) => pieces.push(hir::Expr {
                    kind: hir::ExprKind::StrLit(s.clone()),
                    ty: Ty::String,
                    span,
                }),
                ast::StringPart::Expr(e) => {
                    let value = self.check_expr(fcx, e);
                    let resolved = fcx.icx.apply(&value.ty);
                    match resolved {
                        Ty::String => pieces.push(value),
                        Ty::Int | Ty::Float | Ty::Bool | Ty::Char | Ty::Error => {
                            let part_span = value.span;
                            pieces.push(hir::Expr {
                                kind: hir::ExprKind::ToStr(Box::new(value)),
                                ty: Ty::String,
                                span: part_span,
                            });
                        }
                        other => {
                            self.error(
                                "E0013",
                                format!(
                                    "`{}` cannot be interpolated into a string \
                                     (Display trait support arrives later in Phase 1)",
                                    self.show(&other, fcx),
                                ),
                                value.span,
                            );
                        }
                    }
                }
            }
        }
        hir::Expr {
            kind: hir::ExprKind::StrConcat(pieces),
            ty: Ty::String,
            span,
        }
    }

    fn check_path(&mut self, fcx: &mut FnCtx, path: &ast::Path, span: Span) -> hir::Expr {
        if path.segments.len() == 2 {
            // `Type::Variant` — qualified variant reference.
            let ty_name = path.segments[0].value.as_str();
            let v_name = path.segments[1].value.as_str();
            if let Some(def_id) = self.defs.resolve_type(ty_name) {
                if let Some(vi) = self.variant_index(def_id, v_name) {
                    return self.make_variant(fcx, def_id, vi, Vec::new(), span);
                }
                self.error(
                    "E0001",
                    format!("no variant `{v_name}` on type `{ty_name}`"),
                    span,
                );
                return error_expr(span);
            }
            self.unsupported(span, "module-qualified paths");
            return error_expr(span);
        }
        if path.segments.len() != 1 {
            self.unsupported(span, "module-qualified paths");
            return error_expr(span);
        }
        let name = path.segments[0].value.as_str();
        if let Some(local) = fcx.lookup(name) {
            let ty = fcx.locals[local.0 as usize].ty.clone();
            return hir::Expr {
                kind: hir::ExprKind::Local(local),
                ty,
                span,
            };
        }
        match self.defs.resolve_value(name) {
            Some(Res::Def(def_id)) => match &self.defs.def(def_id).kind {
                DefKind::Fn { .. } => {
                    let Some(sig) = self.sigs.get(&def_id).cloned() else {
                        return error_expr(span);
                    };
                    let type_args: Vec<Ty> =
                        (0..sig.generics).map(|_| fcx.icx.fresh()).collect();
                    let ty = Ty::Fn {
                        params: sig.params.iter().map(|p| p.subst(&type_args)).collect(),
                        ret: Box::new(sig.ret.subst(&type_args)),
                    };
                    hir::Expr {
                        kind: hir::ExprKind::FnRef {
                            def: def_id,
                            type_args,
                        },
                        ty,
                        span,
                    }
                }
                _ => {
                    self.unsupported(span, "referencing this kind of definition as a value");
                    error_expr(span)
                }
            },
            Some(Res::Variant(sum_id, vi)) => self.make_variant(fcx, sum_id, vi, Vec::new(), span),
            Some(Res::Builtin(_)) => {
                self.unsupported(span, "using builtins as values");
                error_expr(span)
            }
            None => {
                self.error("E0001", format!("cannot find `{name}` in this scope"), span);
                error_expr(span)
            }
        }
    }

    fn check_call(
        &mut self,
        fcx: &mut FnCtx,
        callee: &Spanned<ast::Expr>,
        args: &[Spanned<ast::Expr>],
        span: Span,
    ) -> hir::Expr {
        // Direct-call forms: a path naming a function, variant, or builtin.
        if let ast::Expr::Path(path) = &callee.value {
            if path.segments.len() == 1 {
                let name = path.segments[0].value.as_str();
                if fcx.lookup(name).is_none() {
                    match self.defs.resolve_value(name) {
                        Some(Res::Def(def_id)) => {
                            if let DefKind::Fn { .. } = self.defs.def(def_id).kind {
                                return self.check_direct_call(fcx, def_id, args, span);
                            }
                        }
                        Some(Res::Variant(sum_id, vi)) => {
                            let checked: Vec<hir::Expr> =
                                args.iter().map(|a| self.check_expr(fcx, a)).collect();
                            return self.make_variant(fcx, sum_id, vi, checked, span);
                        }
                        Some(Res::Builtin(b)) => {
                            return self.check_builtin_call(fcx, b, args, span);
                        }
                        None => {
                            self.error(
                                "E0001",
                                format!("cannot find function `{name}` in this scope"),
                                callee.span,
                            );
                            return error_expr(span);
                        }
                    }
                }
            } else if path.segments.len() == 2 {
                // `Type::Variant(args)`
                let ty_name = path.segments[0].value.as_str();
                let v_name = path.segments[1].value.as_str();
                if let Some(def_id) = self.defs.resolve_type(ty_name) {
                    if let Some(vi) = self.variant_index(def_id, v_name) {
                        let checked: Vec<hir::Expr> =
                            args.iter().map(|a| self.check_expr(fcx, a)).collect();
                        return self.make_variant(fcx, def_id, vi, checked, span);
                    }
                }
            }
        }

        // Indirect call through an fn-typed value (a local).
        let callee_expr = self.check_expr(fcx, callee);
        let hir::ExprKind::Local(local) = callee_expr.kind else {
            self.unsupported(span, "calling arbitrary expressions");
            return error_expr(span);
        };
        let checked: Vec<hir::Expr> = args.iter().map(|a| self.check_expr(fcx, a)).collect();
        let ret = fcx.icx.fresh();
        let expected = Ty::Fn {
            params: checked.iter().map(|a| a.ty.clone()).collect(),
            ret: Box::new(ret.clone()),
        };
        if !fcx.icx.unify(&callee_expr.ty, &expected) {
            self.error(
                "E0010",
                format!(
                    "`{}` has type `{}` and cannot be called with these arguments",
                    fcx.locals[local.0 as usize].name,
                    self.show(&callee_expr.ty, fcx),
                ),
                callee_expr.span,
            );
            return error_expr(span);
        }
        hir::Expr {
            kind: hir::ExprKind::Call {
                func: hir::Callee::Local(local),
                type_args: Vec::new(),
                args: checked,
            },
            ty: ret,
            span,
        }
    }

    fn check_direct_call(
        &mut self,
        fcx: &mut FnCtx,
        def_id: DefId,
        args: &[Spanned<ast::Expr>],
        span: Span,
    ) -> hir::Expr {
        let Some(sig) = self.sigs.get(&def_id).cloned() else {
            return error_expr(span);
        };
        let name = self.defs.def(def_id).name.clone();
        if args.len() != sig.params.len() {
            self.error(
                "E0016",
                format!(
                    "`{name}` takes {} argument(s) but {} were supplied",
                    sig.params.len(),
                    args.len()
                ),
                span,
            );
            return error_expr(span);
        }
        let type_args: Vec<Ty> = (0..sig.generics).map(|_| fcx.icx.fresh()).collect();
        let mut checked = Vec::new();
        for (arg, param) in args.iter().zip(sig.params.iter()) {
            let a = self.check_expr(fcx, arg);
            let expected = param.subst(&type_args);
            if !fcx.icx.unify(&a.ty, &expected) {
                self.error(
                    "E0010",
                    format!(
                        "argument to `{name}` has type `{}` but `{}` was expected",
                        self.show(&a.ty, fcx),
                        self.show(&expected, fcx),
                    ),
                    a.span,
                );
            }
            checked.push(a);
        }
        let ret = sig.ret.subst(&type_args);
        hir::Expr {
            kind: hir::ExprKind::Call {
                func: hir::Callee::Def(def_id),
                type_args,
                args: checked,
            },
            ty: ret,
            span,
        }
    }

    fn check_builtin_call(
        &mut self,
        fcx: &mut FnCtx,
        builtin: Builtin,
        args: &[Spanned<ast::Expr>],
        span: Span,
    ) -> hir::Expr {
        match builtin {
            Builtin::Println | Builtin::Print => {
                if args.len() != 1 {
                    self.error(
                        "E0016",
                        format!(
                            "`{}` takes 1 argument but {} were supplied",
                            builtin.name(),
                            args.len()
                        ),
                        span,
                    );
                    return error_expr(span);
                }
                let arg = self.check_expr(fcx, &args[0]);
                if !fcx.icx.unify(&arg.ty, &Ty::String) {
                    self.error(
                        "E0010",
                        format!(
                            "`{}` expects a `String`, found `{}` \
                             (use string interpolation: \"${{value}}\")",
                            builtin.name(),
                            self.show(&arg.ty, fcx),
                        ),
                        arg.span,
                    );
                }
                hir::Expr {
                    kind: hir::ExprKind::Call {
                        func: hir::Callee::Builtin(builtin),
                        type_args: Vec::new(),
                        args: vec![arg],
                    },
                    ty: Ty::Unit,
                    span,
                }
            }
        }
    }

    fn make_variant(
        &mut self,
        fcx: &mut FnCtx,
        sum_id: DefId,
        variant: usize,
        args: Vec<hir::Expr>,
        span: Span,
    ) -> hir::Expr {
        let Some(sum) = self.sums.iter().find(|s| s.def_id == sum_id).cloned() else {
            return error_expr(span);
        };
        let v = &sum.variants[variant];
        if args.len() != v.fields.len() {
            self.error(
                "E0016",
                format!(
                    "variant `{}` has {} field(s) but {} were supplied",
                    v.name,
                    v.fields.len(),
                    args.len()
                ),
                span,
            );
            return error_expr(span);
        }
        let type_args: Vec<Ty> = (0..sum.generics).map(|_| fcx.icx.fresh()).collect();
        for (arg, field) in args.iter().zip(v.fields.iter()) {
            let expected = field.subst(&type_args);
            if !fcx.icx.unify(&arg.ty, &expected) {
                self.error(
                    "E0010",
                    format!(
                        "field of `{}` has type `{}` but `{}` was supplied",
                        v.name,
                        self.show(&expected, fcx),
                        self.show(&arg.ty, fcx),
                    ),
                    arg.span,
                );
            }
        }
        hir::Expr {
            kind: hir::ExprKind::MakeVariant {
                sum: sum_id,
                variant: variant as u32,
                args,
            },
            ty: Ty::Sum {
                def_id: sum_id,
                args: type_args,
            },
            span,
        }
    }

    fn check_binary(
        &mut self,
        fcx: &mut FnCtx,
        op: ast::BinOp,
        lhs: &Spanned<ast::Expr>,
        rhs: &Spanned<ast::Expr>,
        span: Span,
    ) -> hir::Expr {
        // Short-circuit operators get dedicated control-flow nodes.
        if matches!(op, ast::BinOp::And | ast::BinOp::Or) {
            let l = self.check_expr(fcx, lhs);
            let r = self.check_expr(fcx, rhs);
            self.expect_ty(fcx, &l, &Ty::Bool, "a logical operand");
            self.expect_ty(fcx, &r, &Ty::Bool, "a logical operand");
            let kind = if matches!(op, ast::BinOp::And) {
                hir::ExprKind::LogicalAnd {
                    lhs: Box::new(l),
                    rhs: Box::new(r),
                }
            } else {
                hir::ExprKind::LogicalOr {
                    lhs: Box::new(l),
                    rhs: Box::new(r),
                }
            };
            return hir::Expr {
                kind,
                ty: Ty::Bool,
                span,
            };
        }

        let l = self.check_expr(fcx, lhs);
        let r = self.check_expr(fcx, rhs);
        let hir_op = convert_binop(op);
        let ty = self.binary_result_ty(fcx, hir_op, &l, &r, span);
        hir::Expr {
            kind: hir::ExprKind::Binary {
                op: hir_op,
                lhs: Box::new(l),
                rhs: Box::new(r),
            },
            ty,
            span,
        }
    }

    /// Determine the result type of a binary operation and validate the
    /// operand types (E0013 when an operator isn't defined for a type).
    fn binary_result_ty(
        &mut self,
        fcx: &mut FnCtx,
        op: hir::BinOp,
        lhs: &hir::Expr,
        rhs: &hir::Expr,
        span: Span,
    ) -> Ty {
        use hir::BinOp::*;
        if !fcx.icx.unify(&lhs.ty, &rhs.ty) {
            self.error(
                "E0010",
                format!(
                    "mismatched operand types: `{}` vs `{}`",
                    self.show(&lhs.ty, fcx),
                    self.show(&rhs.ty, fcx),
                ),
                span,
            );
            return Ty::Error;
        }
        let mut operand = fcx.icx.apply(&lhs.ty);
        if matches!(operand, Ty::Var(_)) {
            // Unconstrained operand (e.g. two fresh generic results):
            // default to Int, matching the spec's literal-defaulting rule.
            fcx.icx.unify(&operand, &Ty::Int);
            operand = Ty::Int;
        }
        match op {
            Add | Sub | Mul | Div | Rem => match operand {
                Ty::Int | Ty::Float | Ty::Never | Ty::Error => operand,
                other => {
                    self.op_not_defined(fcx, "arithmetic", &other, span);
                    Ty::Error
                }
            },
            Lt | Le | Gt | Ge => match operand {
                Ty::Int | Ty::Float | Ty::Char | Ty::Never | Ty::Error => Ty::Bool,
                other => {
                    self.op_not_defined(fcx, "comparison", &other, span);
                    Ty::Bool
                }
            },
            Eq | Ne => match operand {
                Ty::Int
                | Ty::Float
                | Ty::Bool
                | Ty::Char
                | Ty::String
                | Ty::Unit
                | Ty::Never
                | Ty::Error => Ty::Bool,
                other => {
                    self.op_not_defined(fcx, "equality", &other, span);
                    Ty::Bool
                }
            },
            BitAnd | BitOr | BitXor | Shl | Shr => match operand {
                Ty::Int | Ty::Never | Ty::Error => Ty::Int,
                other => {
                    self.op_not_defined(fcx, "bitwise", &other, span);
                    Ty::Error
                }
            },
        }
    }

    fn op_not_defined(&mut self, fcx: &FnCtx, kind: &str, ty: &Ty, span: Span) {
        self.error(
            "E0013",
            format!(
                "{kind} operators are not defined for `{}` \
                 (operator traits arrive later in Phase 1)",
                self.show(ty, fcx),
            ),
            span,
        );
    }

    fn check_unary(
        &mut self,
        fcx: &mut FnCtx,
        op: ast::UnOp,
        inner: &Spanned<ast::Expr>,
        span: Span,
    ) -> hir::Expr {
        let value = self.check_expr(fcx, inner);
        let (hir_op, ty) = match op {
            ast::UnOp::Neg => {
                let t = fcx.icx.apply(&value.ty);
                let t = match t {
                    Ty::Var(_) => {
                        fcx.icx.unify(&t, &Ty::Int);
                        Ty::Int
                    }
                    Ty::Int | Ty::Float | Ty::Never | Ty::Error => t,
                    other => {
                        self.op_not_defined(fcx, "negation", &other, span);
                        Ty::Error
                    }
                };
                (hir::UnOp::Neg, t)
            }
            ast::UnOp::Not => {
                self.expect_ty(fcx, &value, &Ty::Bool, "the `!` operator");
                (hir::UnOp::Not, Ty::Bool)
            }
            ast::UnOp::BitNot => {
                self.expect_ty(fcx, &value, &Ty::Int, "the `~` operator");
                (hir::UnOp::BitNot, Ty::Int)
            }
            ast::UnOp::Ref | ast::UnOp::RefMut | ast::UnOp::Deref => {
                self.unsupported(span, "reference operators");
                return error_expr(span);
            }
        };
        hir::Expr {
            kind: hir::ExprKind::Unary {
                op: hir_op,
                expr: Box::new(value),
            },
            ty,
            span,
        }
    }

    fn check_assign(
        &mut self,
        fcx: &mut FnCtx,
        op: ast::AssignOp,
        lhs: &Spanned<ast::Expr>,
        rhs: &Spanned<ast::Expr>,
        span: Span,
    ) -> hir::Expr {
        let ast::Expr::Path(path) = &lhs.value else {
            self.unsupported(lhs.span, "assignment to anything but a local variable");
            return error_expr(span);
        };
        let name = if path.segments.len() == 1 {
            path.segments[0].value.as_str()
        } else {
            self.unsupported(lhs.span, "assignment to paths");
            return error_expr(span);
        };
        let Some(local) = fcx.lookup(name) else {
            self.error("E0001", format!("cannot find `{name}` in this scope"), lhs.span);
            return error_expr(span);
        };
        let info = fcx.locals[local.0 as usize].clone();
        if !info.is_mut {
            self.error(
                "E0060",
                format!("cannot assign to immutable variable `{name}`"),
                span,
            );
            self.diagnostics
                .last_mut()
                .expect("just pushed")
                .notes
                .push(format!("declare it as `let mut {name}` to allow assignment"));
        }
        let value = self.check_expr(fcx, rhs);

        // Desugar compound assignment: `x += e` → `x = x + e`.
        let final_value = match assign_binop(op) {
            None => value,
            Some(bin) => {
                let lhs_read = hir::Expr {
                    kind: hir::ExprKind::Local(local),
                    ty: info.ty.clone(),
                    span: lhs.span,
                };
                let ty = self.binary_result_ty(fcx, bin, &lhs_read, &value, span);
                hir::Expr {
                    kind: hir::ExprKind::Binary {
                        op: bin,
                        lhs: Box::new(lhs_read),
                        rhs: Box::new(value),
                    },
                    ty,
                    span,
                }
            }
        };
        if !fcx.icx.unify(&final_value.ty, &info.ty) {
            self.error(
                "E0010",
                format!(
                    "cannot assign `{}` to `{name}` which has type `{}`",
                    self.show(&final_value.ty, fcx),
                    self.show(&info.ty, fcx),
                ),
                span,
            );
        }
        hir::Expr {
            kind: hir::ExprKind::Assign {
                local,
                value: Box::new(final_value),
            },
            ty: Ty::Unit,
            span,
        }
    }

    fn check_match(
        &mut self,
        fcx: &mut FnCtx,
        scrutinee: &Spanned<ast::Expr>,
        arms: &[ast::MatchArm],
        span: Span,
    ) -> hir::Expr {
        let scrut = self.check_expr(fcx, scrutinee);
        let result_ty = fcx.icx.fresh();
        let mut hir_arms = Vec::new();
        let mut saw_catch_all = false;
        let mut covered_variants: Vec<u32> = Vec::new();

        for arm in arms {
            if arm.guard.is_some() {
                self.unsupported(arm.pattern.span, "match guards");
            }
            if saw_catch_all {
                self.diagnostics.push(
                    Diagnostic::warning("E0021", "unreachable match arm")
                        .with_primary_label(arm.pattern.span, "this arm is never reached")
                        .with_note("a previous arm matches all values".to_string()),
                );
            }
            fcx.scopes.push(FxHashMap::default());
            let pattern = self.check_pattern(fcx, &arm.pattern, &scrut.ty);
            match &pattern {
                hir::Pattern::Wildcard | hir::Pattern::Bind(_) => saw_catch_all = true,
                hir::Pattern::Variant { variant, .. } => covered_variants.push(*variant),
                _ => {}
            }
            let body = self.check_expr(fcx, &arm.body);
            fcx.scopes.pop();
            if !fcx.icx.unify(&body.ty, &result_ty) {
                self.error(
                    "E0010",
                    format!(
                        "match arms have incompatible types: expected `{}`, found `{}`",
                        self.show(&result_ty, fcx),
                        self.show(&body.ty, fcx),
                    ),
                    body.span,
                );
            }
            hir_arms.push(hir::Arm {
                pattern,
                body,
                span: arm.pattern.span,
            });
        }

        self.check_exhaustiveness(fcx, &scrut, &covered_variants, saw_catch_all, span);

        hir::Expr {
            kind: hir::ExprKind::Match {
                scrutinee: Box::new(scrut),
                arms: hir_arms,
            },
            ty: result_ty,
            span,
        }
    }

    /// Minimal exhaustiveness: full decision-tree usefulness analysis
    /// (Maranget) is a later Phase 1 step; this covers the common cases.
    fn check_exhaustiveness(
        &mut self,
        fcx: &mut FnCtx,
        scrut: &hir::Expr,
        covered_variants: &[u32],
        saw_catch_all: bool,
        span: Span,
    ) {
        if saw_catch_all {
            return;
        }
        match fcx.icx.apply(&scrut.ty) {
            Ty::Sum { def_id, .. } => {
                let Some(sum) = self.sums.iter().find(|s| s.def_id == def_id) else {
                    return;
                };
                let missing: Vec<String> = sum
                    .variants
                    .iter()
                    .enumerate()
                    .filter(|(i, _)| !covered_variants.contains(&(*i as u32)))
                    .map(|(_, v)| format!("`{}`", v.name))
                    .collect();
                if !missing.is_empty() {
                    self.error(
                        "E0020",
                        format!("non-exhaustive match: {} not covered", missing.join(", ")),
                        span,
                    );
                    self.diagnostics
                        .last_mut()
                        .expect("just pushed")
                        .notes
                        .push("add the missing arms or a `_ => ...` catch-all".to_string());
                }
            }
            Ty::Error | Ty::Never | Ty::Var(_) | Ty::Param(_) => {}
            _ => {
                self.error(
                    "E0020",
                    "non-exhaustive match: add a `_ => ...` or binding arm",
                    span,
                );
            }
        }
    }

    fn check_pattern(
        &mut self,
        fcx: &mut FnCtx,
        pattern: &Spanned<ast::Pattern>,
        scrut_ty: &Ty,
    ) -> hir::Pattern {
        match &pattern.value {
            ast::Pattern::Wildcard => hir::Pattern::Wildcard,
            ast::Pattern::Lit(lit) => match lit {
                ast::Literal::Int(v) => {
                    self.unify_pattern_ty(fcx, &Ty::Int, scrut_ty, pattern.span);
                    hir::Pattern::LitInt(*v)
                }
                ast::Literal::Bool(v) => {
                    self.unify_pattern_ty(fcx, &Ty::Bool, scrut_ty, pattern.span);
                    hir::Pattern::LitBool(*v)
                }
                ast::Literal::Str(v) => {
                    self.unify_pattern_ty(fcx, &Ty::String, scrut_ty, pattern.span);
                    hir::Pattern::LitStr(v.clone())
                }
                ast::Literal::Float(_) | ast::Literal::Char(_) => {
                    self.unsupported(pattern.span, "float and char patterns");
                    hir::Pattern::Wildcard
                }
            },
            ast::Pattern::Ident { is_mut, name } => {
                // An identifier that names a payload-less variant of the
                // scrutinee's sum type is a variant pattern, not a binding.
                if let Some(Res::Variant(sum_id, vi)) = self.defs.resolve_value(&name.value) {
                    if self.variant_matches_scrutinee(fcx, sum_id, scrut_ty) {
                        return self.variant_pattern(fcx, sum_id, vi, &[], scrut_ty, pattern.span);
                    }
                }
                let local = fcx.new_local(
                    name.value.clone(),
                    scrut_ty.clone(),
                    *is_mut,
                    name.span,
                );
                hir::Pattern::Bind(local)
            }
            ast::Pattern::Path(path) if path.segments.len() == 1 => {
                let name = &path.segments[0].value;
                if let Some(Res::Variant(sum_id, vi)) = self.defs.resolve_value(name) {
                    if self.variant_matches_scrutinee(fcx, sum_id, scrut_ty) {
                        return self.variant_pattern(fcx, sum_id, vi, &[], scrut_ty, pattern.span);
                    }
                }
                self.error(
                    "E0001",
                    format!("`{name}` is not a variant of the matched type"),
                    pattern.span,
                );
                hir::Pattern::Wildcard
            }
            ast::Pattern::Path(path) if path.segments.len() == 2 => {
                let ty_name = path.segments[0].value.as_str();
                let v_name = path.segments[1].value.as_str();
                if let Some(sum_id) = self.defs.resolve_type(ty_name) {
                    if let Some(vi) = self.variant_index(sum_id, v_name) {
                        if self.variant_matches_scrutinee(fcx, sum_id, scrut_ty) {
                            return self.variant_pattern(fcx, sum_id, vi, &[], scrut_ty, pattern.span);
                        }
                    }
                }
                self.error(
                    "E0001",
                    format!("`{ty_name}::{v_name}` is not a variant of the matched type"),
                    pattern.span,
                );
                hir::Pattern::Wildcard
            }
            ast::Pattern::TupleStruct { path, fields } => {
                let resolved = if path.segments.len() == 1 {
                    match self.defs.resolve_value(&path.segments[0].value) {
                        Some(Res::Variant(sum_id, vi)) => Some((sum_id, vi)),
                        _ => None,
                    }
                } else if path.segments.len() == 2 {
                    self.defs
                        .resolve_type(&path.segments[0].value)
                        .and_then(|sum_id| {
                            self.variant_index(sum_id, &path.segments[1].value)
                                .map(|vi| (sum_id, vi))
                        })
                } else {
                    None
                };
                let Some((sum_id, vi)) = resolved else {
                    self.error(
                        "E0001",
                        "cannot resolve this pattern to a sum type variant",
                        pattern.span,
                    );
                    return hir::Pattern::Wildcard;
                };
                if !self.variant_matches_scrutinee(fcx, sum_id, scrut_ty) {
                    self.error(
                        "E0010",
                        "this variant does not belong to the matched type",
                        pattern.span,
                    );
                    return hir::Pattern::Wildcard;
                }
                self.variant_pattern(fcx, sum_id, vi, fields, scrut_ty, pattern.span)
            }
            ast::Pattern::Path(_)
            | ast::Pattern::Binding { .. }
            | ast::Pattern::Record { .. }
            | ast::Pattern::Tuple(_)
            | ast::Pattern::Array(_)
            | ast::Pattern::Or(_)
            | ast::Pattern::Range { .. } => {
                self.unsupported(pattern.span, "this pattern form");
                hir::Pattern::Wildcard
            }
        }
    }

    /// Unify the scrutinee with `Sum { def_id, fresh vars }`, so payload
    /// field types pick up the scrutinee's generic arguments.
    fn variant_matches_scrutinee(&mut self, fcx: &mut FnCtx, sum_id: DefId, scrut_ty: &Ty) -> bool {
        let generics = self
            .sums
            .iter()
            .find(|s| s.def_id == sum_id)
            .map(|s| s.generics)
            .unwrap_or(0);
        let args: Vec<Ty> = (0..generics).map(|_| fcx.icx.fresh()).collect();
        fcx.icx.unify(
            scrut_ty,
            &Ty::Sum {
                def_id: sum_id,
                args,
            },
        )
    }

    fn variant_pattern(
        &mut self,
        fcx: &mut FnCtx,
        sum_id: DefId,
        variant: usize,
        fields: &[Spanned<ast::Pattern>],
        scrut_ty: &Ty,
        span: Span,
    ) -> hir::Pattern {
        let Some(sum) = self.sums.iter().find(|s| s.def_id == sum_id).cloned() else {
            return hir::Pattern::Wildcard;
        };
        let v = &sum.variants[variant];
        if fields.len() != v.fields.len() {
            self.error(
                "E0016",
                format!(
                    "variant `{}` has {} field(s) but the pattern has {}",
                    v.name,
                    v.fields.len(),
                    fields.len()
                ),
                span,
            );
            return hir::Pattern::Wildcard;
        }
        // The scrutinee was already unified with `Sum { fresh args }` by
        // `variant_matches_scrutinee`; read its args to type the binders.
        let scrut_args = match fcx.icx.apply(scrut_ty) {
            Ty::Sum { args, .. } => args,
            _ => Vec::new(),
        };
        let mut binders = Vec::new();
        for (field_pat, field_ty) in fields.iter().zip(v.fields.iter()) {
            let bound_ty = field_ty.subst(&scrut_args);
            match &field_pat.value {
                ast::Pattern::Wildcard => binders.push(None),
                ast::Pattern::Ident { is_mut, name } => {
                    let local =
                        fcx.new_local(name.value.clone(), bound_ty, *is_mut, name.span);
                    binders.push(Some(local));
                }
                _ => {
                    self.unsupported(field_pat.span, "nested patterns inside variants");
                    binders.push(None);
                }
            }
        }
        hir::Pattern::Variant {
            sum: sum_id,
            variant: variant as u32,
            binders,
        }
    }

    // === Helpers ===

    fn variant_index(&self, sum_id: DefId, name: &str) -> Option<usize> {
        self.sums
            .iter()
            .find(|s| s.def_id == sum_id)?
            .variants
            .iter()
            .position(|v| v.name == name)
    }

    fn unify_pattern_ty(&mut self, fcx: &mut FnCtx, pat_ty: &Ty, scrut_ty: &Ty, span: Span) {
        if !fcx.icx.unify(pat_ty, scrut_ty) {
            self.error(
                "E0010",
                format!(
                    "pattern has type `{}` but the matched value has type `{}`",
                    self.show(pat_ty, fcx),
                    self.show(scrut_ty, fcx),
                ),
                span,
            );
        }
    }

    fn expect_ty(&mut self, fcx: &mut FnCtx, expr: &hir::Expr, expected: &Ty, what: &str) {
        if !fcx.icx.unify(&expr.ty, expected) {
            self.error(
                "E0010",
                format!(
                    "{what} must have type `{}`, found `{}`",
                    self.show(expected, fcx),
                    self.show(&expr.ty, fcx),
                ),
                expr.span,
            );
        }
    }

    fn show(&self, ty: &Ty, fcx: &FnCtx) -> String {
        display_ty(&fcx.icx.apply(ty), self.defs)
    }

    fn error(&mut self, code: &str, message: impl Into<String>, span: Span) {
        self.diagnostics
            .push(Diagnostic::error(code, message).with_primary_label(span, "here"));
    }

    fn unsupported(&mut self, span: Span, what: &str) {
        self.diagnostics.push(
            Diagnostic::error("E0900", format!("{what} are not supported yet"))
                .with_primary_label(span, "not supported yet")
                .with_note(
                    "the Phase 1 MVP compiler supports a subset of Nova; \
                     this feature arrives in a later milestone"
                        .to_string(),
                ),
        );
    }

    fn unsupported_expr(&mut self, span: Span, what: &str) -> hir::Expr {
        self.unsupported(span, what);
        error_expr(span)
    }
}

// === Free helpers ===

fn generic_scope(generics: &[ast::TypeParam]) -> FxHashMap<String, u32> {
    generics
        .iter()
        .enumerate()
        .map(|(i, g)| (g.name.value.clone(), i as u32))
        .collect()
}

fn lit_expr(lit: &ast::Literal, span: Span) -> hir::Expr {
    let (kind, ty) = match lit {
        ast::Literal::Int(v) => (hir::ExprKind::IntLit(*v), Ty::Int),
        ast::Literal::Float(v) => (hir::ExprKind::FloatLit(*v), Ty::Float),
        ast::Literal::Str(v) => (hir::ExprKind::StrLit(v.clone()), Ty::String),
        ast::Literal::Char(v) => (hir::ExprKind::CharLit(*v), Ty::Char),
        ast::Literal::Bool(v) => (hir::ExprKind::BoolLit(*v), Ty::Bool),
    };
    hir::Expr { kind, ty, span }
}

fn error_expr(span: Span) -> hir::Expr {
    hir::Expr {
        kind: hir::ExprKind::Unit,
        ty: Ty::Error,
        span,
    }
}

fn convert_binop(op: ast::BinOp) -> hir::BinOp {
    match op {
        ast::BinOp::Add => hir::BinOp::Add,
        ast::BinOp::Sub => hir::BinOp::Sub,
        ast::BinOp::Mul => hir::BinOp::Mul,
        ast::BinOp::Div => hir::BinOp::Div,
        ast::BinOp::Rem => hir::BinOp::Rem,
        ast::BinOp::Eq => hir::BinOp::Eq,
        ast::BinOp::Ne => hir::BinOp::Ne,
        ast::BinOp::Lt => hir::BinOp::Lt,
        ast::BinOp::Le => hir::BinOp::Le,
        ast::BinOp::Gt => hir::BinOp::Gt,
        ast::BinOp::Ge => hir::BinOp::Ge,
        ast::BinOp::BitAnd => hir::BinOp::BitAnd,
        ast::BinOp::BitOr => hir::BinOp::BitOr,
        ast::BinOp::BitXor => hir::BinOp::BitXor,
        ast::BinOp::Shl => hir::BinOp::Shl,
        ast::BinOp::Shr => hir::BinOp::Shr,
        // And/Or are handled before conversion.
        ast::BinOp::And | ast::BinOp::Or => hir::BinOp::Eq,
    }
}

fn assign_binop(op: ast::AssignOp) -> Option<hir::BinOp> {
    match op {
        ast::AssignOp::Assign => None,
        ast::AssignOp::AddAssign => Some(hir::BinOp::Add),
        ast::AssignOp::SubAssign => Some(hir::BinOp::Sub),
        ast::AssignOp::MulAssign => Some(hir::BinOp::Mul),
        ast::AssignOp::DivAssign => Some(hir::BinOp::Div),
        ast::AssignOp::RemAssign => Some(hir::BinOp::Rem),
        ast::AssignOp::BitOrAssign => Some(hir::BinOp::BitOr),
        ast::AssignOp::BitAndAssign => Some(hir::BinOp::BitAnd),
        ast::AssignOp::BitXorAssign => Some(hir::BinOp::BitXor),
        ast::AssignOp::ShlAssign => Some(hir::BinOp::Shl),
        ast::AssignOp::ShrAssign => Some(hir::BinOp::Shr),
    }
}

/// The span to point at when a function body's type doesn't match its
/// declared return type: the trailing expression if there is one.
fn body_result_span(body: &Spanned<ast::Block>) -> Span {
    body.value
        .trailing
        .as_ref()
        .map(|e| e.span)
        .unwrap_or(body.span)
}

/// Deeply apply the substitution to every type stored in an expression,
/// collecting spans whose types still contain inference variables.
fn finalize_expr(expr: &mut hir::Expr, icx: &InferCtx, residual: &mut Vec<Span>) {
    expr.ty = icx.apply(&expr.ty);
    if expr.ty.has_vars() {
        residual.push(expr.span);
    }
    match &mut expr.kind {
        hir::ExprKind::Call { type_args, args, .. } => {
            for t in type_args.iter_mut() {
                *t = icx.apply(t);
                if t.has_vars() {
                    residual.push(expr.span);
                }
            }
            for a in args {
                finalize_expr(a, icx, residual);
            }
        }
        hir::ExprKind::FnRef { type_args, .. } => {
            for t in type_args.iter_mut() {
                *t = icx.apply(t);
                if t.has_vars() {
                    residual.push(expr.span);
                }
            }
        }
        hir::ExprKind::MakeVariant { args, .. } | hir::ExprKind::StrConcat(args) => {
            for a in args {
                finalize_expr(a, icx, residual);
            }
        }
        hir::ExprKind::Binary { lhs, rhs, .. }
        | hir::ExprKind::LogicalAnd { lhs, rhs }
        | hir::ExprKind::LogicalOr { lhs, rhs } => {
            finalize_expr(lhs, icx, residual);
            finalize_expr(rhs, icx, residual);
        }
        hir::ExprKind::Unary { expr: inner, .. }
        | hir::ExprKind::ToStr(inner)
        | hir::ExprKind::Let { init: inner, .. }
        | hir::ExprKind::Assign { value: inner, .. } => {
            finalize_expr(inner, icx, residual);
        }
        hir::ExprKind::Block { stmts, trailing } => {
            for s in stmts {
                finalize_expr(s, icx, residual);
            }
            if let Some(t) = trailing {
                finalize_expr(t, icx, residual);
            }
        }
        hir::ExprKind::If { cond, then, else_ } => {
            finalize_expr(cond, icx, residual);
            finalize_expr(then, icx, residual);
            if let Some(e) = else_ {
                finalize_expr(e, icx, residual);
            }
        }
        hir::ExprKind::While { cond, body } => {
            finalize_expr(cond, icx, residual);
            finalize_expr(body, icx, residual);
        }
        hir::ExprKind::Match { scrutinee, arms } => {
            finalize_expr(scrutinee, icx, residual);
            for arm in arms {
                finalize_expr(&mut arm.body, icx, residual);
            }
        }
        hir::ExprKind::Return(value) => {
            if let Some(v) = value {
                finalize_expr(v, icx, residual);
            }
        }
        hir::ExprKind::IntLit(_)
        | hir::ExprKind::FloatLit(_)
        | hir::ExprKind::BoolLit(_)
        | hir::ExprKind::StrLit(_)
        | hir::ExprKind::CharLit(_)
        | hir::ExprKind::Unit
        | hir::ExprKind::Local(_) => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nova_diagnostics::FileId;
    use nova_lexer::lex;
    use nova_parser::parse;
    use nova_resolver::resolve;

    fn check_src(src: &str) -> CheckResult {
        let file_id = FileId::DUMMY;
        let (tokens, lex_errors) = lex(src, file_id);
        assert!(lex_errors.is_empty(), "lex errors: {lex_errors:?}");
        let (ast, parse_errors) = parse(&tokens, file_id);
        assert!(parse_errors.is_empty(), "parse errors: {parse_errors:?}");
        let ast = ast.expect("no AST");
        let resolved = resolve(&ast);
        assert!(
            resolved.diagnostics.is_empty(),
            "resolve errors: {:?}",
            resolved.diagnostics
        );
        check(&ast, &resolved.definitions)
    }

    fn error_codes(result: &CheckResult) -> Vec<&str> {
        result
            .diagnostics
            .iter()
            .filter(|d| d.severity == nova_diagnostics::Severity::Error)
            .map(|d| d.code.as_str())
            .collect()
    }

    #[test]
    fn hello_world_checks() {
        let r = check_src("fn main() { println(\"hi\") }");
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
        assert_eq!(r.module.functions.len(), 1);
    }

    #[test]
    fn fibonacci_checks() {
        let r = check_src(
            "fn fib(n: Int) -> Int {\n\
                 if n <= 1 { n } else { fib(n - 1) + fib(n - 2) }\n\
             }\n\
             fn main() { println(\"${fib(10)}\") }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    }

    #[test]
    fn type_mismatch_reports_e0010() {
        let r = check_src("fn main() { let x: Int = \"hello\" }");
        assert!(error_codes(&r).contains(&"E0010"), "{:?}", r.diagnostics);
    }

    #[test]
    fn generic_identity_instantiates() {
        let r = check_src(
            "fn identity<T>(x: T) -> T { x }\n\
             fn main() { let n = identity(1) + 1\n let s = identity(\"s\")\n println(\"${n}${s}\") }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
        let main = r
            .module
            .functions
            .iter()
            .find(|f| f.name == "main")
            .expect("main exists");
        // The two calls to identity must record Int and String type args.
        let mut type_args = Vec::new();
        collect_call_type_args(&main.body, &mut type_args);
        assert!(type_args.contains(&vec![Ty::Int]));
        assert!(type_args.contains(&vec![Ty::String]));
    }

    fn collect_call_type_args(expr: &hir::Expr, out: &mut Vec<Vec<Ty>>) {
        if let hir::ExprKind::Call { type_args, args, .. } = &expr.kind {
            if !type_args.is_empty() {
                out.push(type_args.clone());
            }
            for a in args {
                collect_call_type_args(a, out);
            }
        }
        if let hir::ExprKind::Block { stmts, trailing } = &expr.kind {
            for s in stmts {
                collect_call_type_args(s, out);
            }
            if let Some(t) = trailing {
                collect_call_type_args(t, out);
            }
        }
        if let hir::ExprKind::Let { init, .. } = &expr.kind {
            collect_call_type_args(init, out);
        }
        if let hir::ExprKind::Binary { lhs, rhs, .. } = &expr.kind {
            collect_call_type_args(lhs, out);
            collect_call_type_args(rhs, out);
        }
        if let hir::ExprKind::StrConcat(parts) = &expr.kind {
            for p in parts {
                collect_call_type_args(p, out);
            }
        }
        if let hir::ExprKind::ToStr(inner) = &expr.kind {
            collect_call_type_args(inner, out);
        }
    }

    #[test]
    fn non_exhaustive_match_reports_e0020() {
        let r = check_src(
            "type Shape = | Circle(Int) | Empty\n\
             fn f(s: Shape) -> Int { match s { Circle(r) => r, } }\n\
             fn main() { }",
        );
        assert!(error_codes(&r).contains(&"E0020"), "{:?}", r.diagnostics);
    }

    #[test]
    fn exhaustive_match_ok() {
        let r = check_src(
            "type Shape = | Circle(Int) | Empty\n\
             fn f(s: Shape) -> Int { match s { Circle(r) => r, Empty => 0, } }\n\
             fn main() { }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    }

    #[test]
    fn unknown_name_reports_e0001() {
        let r = check_src("fn main() { let x = nope() }");
        assert!(error_codes(&r).contains(&"E0001"), "{:?}", r.diagnostics);
    }

    #[test]
    fn assign_to_immutable_reports_e0060() {
        let r = check_src("fn main() { let x = 1\n x = 2 }");
        assert!(error_codes(&r).contains(&"E0060"), "{:?}", r.diagnostics);
    }

    #[test]
    fn mutable_assign_ok() {
        let r = check_src("fn main() { let mut x = 1\n x = 2\n x += 3\n println(\"${x}\") }");
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    }

    #[test]
    fn fn_as_value_ok() {
        let r = check_src(
            "fn double(n: Int) -> Int { n * 2 }\n\
             fn apply_twice<T>(f: fn(T) -> T, x: T) -> T { f(f(x)) }\n\
             fn main() { println(\"${apply_twice(double, 5)}\") }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    }

    #[test]
    fn wrong_arity_reports_e0016() {
        let r = check_src("fn f(a: Int) -> Int { a }\nfn main() { let x = f(1, 2) }");
        assert!(error_codes(&r).contains(&"E0016"), "{:?}", r.diagnostics);
    }
}
