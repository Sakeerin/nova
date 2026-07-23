//! AST → typed HIR checking: signature collection, body inference,
//! desugaring, and minimal exhaustiveness analysis.

use nova_ast as ast;
use nova_ast::item::{TraitItem, TypeDef};
use nova_diagnostics::{Diagnostic, Span, Spanned};
use nova_hir as hir;
use nova_hir::{LocalId, Ty, TyHead};
use nova_resolver::{Builtin, DefId, DefKind, Definitions, MethodOwner, Res};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::infer::InferCtx;
use crate::{display_ty, CheckResult};

/// A collected function (or method) signature.
#[derive(Debug, Clone)]
struct FnSig {
    generics: u32,
    /// Trait bounds per generic parameter.
    bounds: Vec<Vec<DefId>>,
    /// Parameter types. For methods, `params[0]` is the `self` receiver.
    params: Vec<Ty>,
    ret: Ty,
}

/// Outcome of resolving a method name against a receiver type.
enum MethodRes {
    /// An inherent method (compiled function `DefId`).
    Inherent(DefId),
    /// A trait method: `(trait_id, method_index)`.
    Trait(DefId, u32),
    /// No method of that name applies.
    None,
    /// More than one trait provides the method.
    Ambiguous,
}

/// The AST location and flavor of a method to compile.
#[derive(Debug, Clone, Copy)]
struct MethodLoc {
    item_index: usize,
    method_index: usize,
    owner: MethodOwner,
}

/// Type-check a parsed file against its resolved definitions.
pub fn check(file: &ast::File, defs: &Definitions) -> CheckResult {
    let mut checker = Checker {
        file,
        defs,
        sigs: FxHashMap::default(),
        method_locs: FxHashMap::default(),
        sums: Vec::new(),
        records: Vec::new(),
        traits: Vec::new(),
        impls: Vec::new(),
        extra_functions: Vec::new(),
        next_closure_def: defs.defs().len() as u32,
        diagnostics: Vec::new(),
    };
    checker.collect_records();
    checker.collect_sums();
    checker.collect_traits();
    checker.collect_impls();
    checker.collect_signatures();

    let mut functions = Vec::new();
    let fn_ids: Vec<(DefId, usize)> = defs.functions().collect();
    for (def_id, item_index) in fn_ids {
        if let Some(f) = checker.check_function(def_id, item_index) {
            functions.push(f);
        }
    }
    let method_ids: Vec<DefId> = checker.method_locs.keys().copied().collect();
    for def_id in method_ids {
        if let Some(f) = checker.check_method(def_id) {
            functions.push(f);
        }
    }
    for (def_id, item_index) in checker.const_ids() {
        if let Some(f) = checker.check_const(def_id, item_index) {
            functions.push(f);
        }
    }
    checker.check_const_cycles(&functions);
    // Closure and bare-fn-wrapper functions synthesized while checking.
    functions.append(&mut checker.extra_functions);

    CheckResult {
        module: hir::Module {
            sums: checker.sums,
            records: checker.records,
            traits: checker.traits,
            impls: checker.impls,
            functions,
        },
        diagnostics: checker.diagnostics,
    }
}

struct Checker<'a> {
    file: &'a ast::File,
    defs: &'a Definitions,
    sigs: FxHashMap<DefId, FnSig>,
    /// AST location of each method `DefId`, for the compile pass.
    method_locs: FxHashMap<DefId, MethodLoc>,
    sums: Vec<hir::SumType>,
    records: Vec<hir::RecordType>,
    traits: Vec<hir::TraitDef>,
    impls: Vec<hir::ImplInfo>,
    /// Lifted closure / fn-wrapper functions, appended to the module.
    extra_functions: Vec<hir::Function>,
    /// Next synthetic `DefId` for a closure/wrapper (starts past all
    /// resolver-assigned defs so it never collides).
    next_closure_def: u32,
    diagnostics: Vec<Diagnostic>,
}

/// Per-function checking state.
struct FnCtx {
    icx: InferCtx,
    locals: Vec<hir::Local>,
    scopes: Vec<FxHashMap<String, LocalId>>,
    /// Generic parameter names of the enclosing function.
    generics: FxHashMap<String, u32>,
    /// Trait bounds per generic parameter, indexed like `generics` values.
    param_bounds: Vec<Vec<DefId>>,
    ret_ty: Ty,
    /// Nesting depth of enclosing loops in the *current* function body.
    /// Reset to 0 inside a closure body (a `break`/`continue` there cannot
    /// target an outer loop).
    loop_depth: usize,
    /// Closure / wrapper functions lifted out while checking this function's
    /// body; finalized with this context's `icx` and then emitted.
    pending_closures: Vec<hir::Function>,
}

impl FnCtx {
    fn lookup(&self, name: &str) -> Option<LocalId> {
        self.scopes
            .iter()
            .rev()
            .find_map(|scope| scope.get(name).copied())
    }

    fn new_local(&mut self, name: String, ty: Ty, is_mut: bool, span: Span) -> LocalId {
        let id = self.new_local_unscoped(name.clone(), ty, is_mut, span);
        if name != "_" {
            if let Some(scope) = self.scopes.last_mut() {
                scope.insert(name, id);
            }
        }
        id
    }

    /// Allocate a local that is *not* inserted into the name-resolution
    /// scope — used for compiler-synthesized temporaries (e.g. a for-loop
    /// counter) so they cannot be captured by, or collide with, source
    /// identifiers.
    fn new_local_unscoped(&mut self, name: String, ty: Ty, is_mut: bool, span: Span) -> LocalId {
        let id = LocalId(self.locals.len() as u32);
        self.locals.push(hir::Local {
            name,
            ty,
            is_mut,
            span,
        });
        id
    }
}

impl<'a> Checker<'a> {
    // === Collection ===

    fn collect_records(&mut self) {
        for (i, def) in self.defs.defs().iter().enumerate() {
            let DefKind::Record { item_index } = &def.kind else {
                continue;
            };
            let ast::Item::Record(decl) = &self.file.items[*item_index].value else {
                continue;
            };
            let generics = generic_scope(&decl.generics);
            let fields = decl
                .fields
                .iter()
                .map(|f| hir::RecordField {
                    name: f.name.value.clone(),
                    ty: self.convert_ty(&f.ty, &generics),
                })
                .collect();
            self.records.push(hir::RecordType {
                def_id: DefId(i as u32),
                name: def.name.clone(),
                generics: decl.generics.len() as u32,
                fields,
            });
        }
    }

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

    /// Collect trait declarations and their method signatures. `Self` is
    /// encoded as `Ty::Param(0)` in stored method signatures.
    fn collect_traits(&mut self) {
        // Map (trait item_index, method_index) → default-method DefId.
        let default_defs: FxHashMap<(usize, usize), DefId> = self
            .defs
            .methods()
            .filter(|(_, _, _, owner)| *owner == MethodOwner::TraitDefault)
            .map(|(id, ii, mi, _)| ((ii, mi), id))
            .collect();

        for (i, def) in self.defs.defs().iter().enumerate() {
            let DefKind::Trait { item_index } = def.kind else {
                continue;
            };
            let ast::Item::Trait(decl) = &self.file.items[item_index].value else {
                continue;
            };
            if !decl.generics.is_empty() {
                self.unsupported(decl.name.span, "generic traits");
            }
            let self_scope = self_generic_scope();
            let mut methods = Vec::new();
            for (mi, item) in decl.items.iter().enumerate() {
                let (name, params, ret, span, is_default) = match item {
                    TraitItem::Required(sig) => {
                        (&sig.name, &sig.params, &sig.return_ty, sig.name.span, false)
                    }
                    TraitItem::Provided(f) => (&f.name, &f.params, &f.return_ty, f.name.span, true),
                };
                let _ = span;
                let (m_params, m_ret) = self.method_sig_parts(params, ret, &self_scope);
                let default_def = if is_default {
                    default_defs.get(&(item_index, mi)).copied()
                } else {
                    None
                };
                methods.push(hir::TraitMethod {
                    name: name.value.clone(),
                    params: m_params,
                    ret: m_ret,
                    default_def,
                });
            }
            self.traits.push(hir::TraitDef {
                def_id: DefId(i as u32),
                name: def.name.clone(),
                methods,
            });
        }

        // Signatures for default-method function bodies: generic over Self
        // (`Param(0)`), bounded by the enclosing trait.
        let trait_items: Vec<(DefId, usize)> = self
            .defs
            .defs()
            .iter()
            .enumerate()
            .filter_map(|(i, d)| match d.kind {
                DefKind::Trait { item_index } => Some((DefId(i as u32), item_index)),
                _ => None,
            })
            .collect();
        for (trait_id, item_index) in trait_items {
            let ast::Item::Trait(decl) = &self.file.items[item_index].value else {
                continue;
            };
            let self_scope = self_generic_scope();
            for (mi, item) in decl.items.iter().enumerate() {
                let TraitItem::Provided(f) = item else {
                    continue;
                };
                let Some(def_id) = default_defs.get(&(item_index, mi)).copied() else {
                    continue;
                };
                self.reject_method_generics(f);
                let (mut params, ret) = self.method_sig_parts(&f.params, &f.return_ty, &self_scope);
                // Prepend the `self` receiver typed as `Self` (`Param(0)`).
                params.insert(0, Ty::Param(0));
                self.sigs.insert(
                    def_id,
                    FnSig {
                        generics: 1,
                        bounds: vec![vec![trait_id]],
                        params,
                        ret,
                    },
                );
                self.method_locs.insert(
                    def_id,
                    MethodLoc {
                        item_index,
                        method_index: mi,
                        owner: MethodOwner::TraitDefault,
                    },
                );
            }
        }
    }

    /// Collect impl blocks into the impl table and method signatures.
    fn collect_impls(&mut self) {
        let impl_methods: FxHashMap<(usize, usize), DefId> = self
            .defs
            .methods()
            .filter(|(_, _, _, owner)| *owner == MethodOwner::Impl)
            .map(|(id, ii, mi, _)| ((ii, mi), id))
            .collect();

        // Source span of each collected impl (aligned with `self.impls`), for
        // the post-collection coherence check.
        let mut impl_spans: Vec<Span> = Vec::new();

        for (item_index, item) in self.file.items.iter().enumerate() {
            let ast::Item::Impl(block) = &item.value else {
                continue;
            };
            // `where` clauses on impls (conditional impls beyond inline
            // `<T: Bound>`) are not supported yet — consistent with functions.
            if !block.where_clause.is_empty() {
                self.unsupported(block.ty.span, "`where` clauses on impl blocks");
                continue;
            }
            // The impl's generic parameters (`impl<T> …`) are in scope in the
            // self type and every method signature/body.
            let impl_generics = generic_scope(&block.generics);
            let impl_bounds = self.resolve_bounds(&block.generics);
            let self_ty = self.convert_ty(&block.ty, &impl_generics);
            let Some(self_head) = self_ty.head() else {
                self.error(
                    "E0010",
                    "impl blocks are only supported on named types",
                    block.ty.span,
                );
                continue;
            };
            // Every impl generic parameter must appear in the self type, or its
            // type argument could never be recovered at a call site (and, for an
            // inherent method that ignores it, the parameter would leak an
            // unconstrained inference variable). Cf. Rust's E0207.
            let mut has_unused = false;
            for (i, g) in block.generics.iter().enumerate() {
                if !self_ty.mentions_param(i as u32) {
                    has_unused = true;
                    let tystr = display_ty(&self_ty, self.defs);
                    self.error(
                        "E0073",
                        format!(
                            "impl type parameter `{}` is not used in the self type `{tystr}`",
                            g.name.value
                        ),
                        g.name.span,
                    );
                }
            }
            if has_unused {
                continue;
            }
            let trait_id = match &block.trait_ {
                Some(tr) => {
                    let name = tr
                        .value
                        .segments
                        .last()
                        .map(|s| s.value.as_str())
                        .unwrap_or("");
                    match self.defs.resolve_trait(name) {
                        Some(id) => Some(id),
                        None => {
                            self.error("E0001", format!("cannot find trait `{name}`"), tr.span);
                            continue;
                        }
                    }
                }
                None => None,
            };

            let mut methods = Vec::new();
            for (mi, f) in block.functions.iter().enumerate() {
                let Some(def_id) = impl_methods.get(&(item_index, mi)).copied() else {
                    continue;
                };
                self.reject_method_generics(f);
                // Non-self params + ret in terms of the self type, resolving
                // the impl's generic parameters.
                let (mut params, ret) =
                    self.method_sig_parts(&f.params, &f.return_ty, &impl_generics);
                params.insert(0, self_ty.clone());
                self.sigs.insert(
                    def_id,
                    FnSig {
                        generics: block.generics.len() as u32,
                        bounds: impl_bounds.clone(),
                        params,
                        ret,
                    },
                );
                self.method_locs.insert(
                    def_id,
                    MethodLoc {
                        item_index,
                        method_index: mi,
                        owner: MethodOwner::Impl,
                    },
                );
                methods.push((f.name.value.clone(), def_id));
            }

            // Conformance: a trait impl must define exactly the trait's
            // methods that lack defaults, and nothing foreign.
            if let Some(tid) = trait_id {
                self.check_impl_conformance(tid, &methods, &self_ty, block.ty.span);
            }

            self.impls.push(hir::ImplInfo {
                trait_id,
                self_head,
                self_ty,
                generics: block.generics.len() as u32,
                bounds: impl_bounds,
                methods,
            });
            impl_spans.push(block.ty.span);
        }

        self.check_impl_coherence(&impl_spans);
    }

    /// Reject overlapping implementations (Phase 1 has no specialization): two
    /// trait impls of the same trait whose self types share a ground instance
    /// conflict outright, and two inherent impls that overlap conflict on any
    /// method they both define. Without this, dispatch would depend on impl
    /// declaration order.
    fn check_impl_coherence(&mut self, spans: &[Span]) {
        let mut conflicts: Vec<(String, Span)> = Vec::new();
        for (i, a) in self.impls.iter().enumerate() {
            for (b, &b_span) in self.impls.iter().zip(spans).skip(i + 1) {
                if a.self_head != b.self_head || a.trait_id != b.trait_id {
                    continue;
                }
                if !hir::self_types_overlap(&a.self_ty, a.generics, &b.self_ty, b.generics) {
                    continue;
                }
                match a.trait_id {
                    Some(tid) => {
                        let tname = self
                            .traits
                            .iter()
                            .find(|t| t.def_id == tid)
                            .map(|t| t.name.clone())
                            .unwrap_or_else(|| "?".to_string());
                        conflicts.push((
                            format!(
                                "conflicting implementations of trait `{tname}` for \
                                 overlapping types"
                            ),
                            b_span,
                        ));
                    }
                    None => {
                        let dup = a
                            .methods
                            .iter()
                            .map(|(n, _)| n)
                            .find(|n| b.methods.iter().any(|(m, _)| m == *n))
                            .cloned();
                        if let Some(m) = dup {
                            conflicts.push((
                                format!(
                                    "method `{m}` is defined by multiple overlapping \
                                     inherent impls"
                                ),
                                b_span,
                            ));
                        }
                    }
                }
            }
        }
        for (msg, span) in conflicts {
            self.error("E0074", msg, span);
        }
    }

    /// Verify a trait impl provides all required methods, no unknown ones,
    /// and that each provided method's signature matches the trait's
    /// declaration (with `Self` bound to the impl's self type). Without the
    /// signature check the call site uses the trait signature while codegen
    /// dispatches to the impl's method — a mismatch miscompiles or is
    /// memory-unsafe.
    fn check_impl_conformance(
        &mut self,
        trait_id: DefId,
        provided: &[(String, DefId)],
        self_ty: &Ty,
        span: Span,
    ) {
        let Some(tr) = self.traits.iter().find(|t| t.def_id == trait_id).cloned() else {
            return;
        };
        let subst = [self_ty.clone()];
        for (name, def_id) in provided {
            let Some(trait_method) = tr.methods.iter().find(|m| &m.name == name) else {
                self.error(
                    "E0071",
                    format!("method `{name}` is not a member of trait `{}`", tr.name),
                    span,
                );
                continue;
            };
            let Some(impl_sig) = self.sigs.get(def_id).cloned() else {
                continue;
            };
            // impl_sig.params[0] is `self`; compare the rest and the return
            // type against the trait method (with `Self` substituted).
            let impl_params = &impl_sig.params[1..];
            let expected: Vec<Ty> = trait_method
                .params
                .iter()
                .map(|p| p.subst(&subst))
                .collect();
            if impl_params.len() != expected.len() {
                self.error(
                    "E0072",
                    format!(
                        "method `{name}` has {} parameter(s) but trait `{}` declares {}",
                        impl_params.len(),
                        tr.name,
                        expected.len()
                    ),
                    span,
                );
                continue;
            }
            for (i, (got, want)) in impl_params.iter().zip(expected.iter()).enumerate() {
                if got != want {
                    self.error(
                        "E0072",
                        format!(
                            "parameter {} of method `{name}` has type `{}` but trait `{}` \
                             declares `{}`",
                            i + 1,
                            display_ty(got, self.defs),
                            tr.name,
                            display_ty(want, self.defs),
                        ),
                        span,
                    );
                }
            }
            let expected_ret = trait_method.ret.subst(&subst);
            if impl_sig.ret != expected_ret {
                self.error(
                    "E0072",
                    format!(
                        "method `{name}` returns `{}` but trait `{}` declares `{}`",
                        display_ty(&impl_sig.ret, self.defs),
                        tr.name,
                        display_ty(&expected_ret, self.defs),
                    ),
                    span,
                );
            }
        }
        let missing: Vec<String> = tr
            .methods
            .iter()
            .filter(|m| m.default_def.is_none() && !provided.iter().any(|(n, _)| n == &m.name))
            .map(|m| format!("`{}`", m.name))
            .collect();
        if !missing.is_empty() {
            self.error(
                "E0070",
                format!(
                    "impl of trait `{}` is missing method(s): {}",
                    tr.name,
                    missing.join(", ")
                ),
                span,
            );
        }
    }

    /// Reject method-level generics/async (deferred in Phase 1).
    fn reject_method_generics(&mut self, f: &ast::Function) {
        if f.is_async {
            self.unsupported(f.name.span, "async methods");
        }
        if !f.generics.is_empty() {
            self.unsupported(f.name.span, "generic methods");
        }
    }

    /// Convert a method's non-`self` parameter types and return type using
    /// `scope` (which maps `Self`/generics to `Param` indices).
    fn method_sig_parts(
        &mut self,
        params: &[ast::Param],
        ret: &Option<Spanned<ast::Type>>,
        scope: &FxHashMap<String, u32>,
    ) -> (Vec<Ty>, Ty) {
        let converted = params
            .iter()
            .filter(|p| p.name.value != "self")
            .map(|p| self.convert_ty(&p.ty, scope))
            .collect();
        let ret_ty = ret
            .as_ref()
            .map(|t| self.convert_ty(t, scope))
            .unwrap_or(Ty::Unit);
        (converted, ret_ty)
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
            if !f.where_clause.is_empty() {
                self.unsupported(f.name.span, "`where` clauses");
            }
            let generics = generic_scope(&f.generics);
            let bounds = self.resolve_bounds(&f.generics);
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
                    bounds,
                    params,
                    ret,
                },
            );
        }
        // A constant compiles to a zero-argument function returning its
        // value; register its signature here.
        for (def_id, item_index) in self.const_ids() {
            let ast::Item::Const(c) = &self.file.items[item_index].value else {
                continue;
            };
            let ret = self.convert_ty(&c.ty, &FxHashMap::default());
            self.sigs.insert(
                def_id,
                FnSig {
                    generics: 0,
                    bounds: Vec::new(),
                    params: Vec::new(),
                    ret,
                },
            );
        }
    }

    /// All constant definitions as `(DefId, item_index)`.
    fn const_ids(&self) -> Vec<(DefId, usize)> {
        self.defs
            .defs()
            .iter()
            .enumerate()
            .filter_map(|(i, d)| match d.kind {
                DefKind::Const { item_index } => Some((DefId(i as u32), item_index)),
                _ => None,
            })
            .collect()
    }

    /// Resolve each generic parameter's trait bounds to trait `DefId`s.
    fn resolve_bounds(&mut self, generics: &[ast::TypeParam]) -> Vec<Vec<DefId>> {
        generics
            .iter()
            .map(|g| {
                g.bounds
                    .iter()
                    .filter_map(|b| {
                        let name = b
                            .value
                            .segments
                            .last()
                            .map(|s| s.value.as_str())
                            .unwrap_or("");
                        match self.defs.resolve_trait(name) {
                            Some(id) => Some(id),
                            None => {
                                self.error("E0001", format!("cannot find trait `{name}`"), b.span);
                                None
                            }
                        }
                    })
                    .collect()
            })
            .collect()
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
                    let is_record = matches!(self.defs.def(def_id).kind, DefKind::Record { .. });
                    let expected = if is_record {
                        self.records
                            .iter()
                            .find(|r| r.def_id == def_id)
                            .map(|r| r.generics)
                            .unwrap_or(0)
                    } else {
                        self.sums
                            .iter()
                            .find(|s| s.def_id == def_id)
                            .map(|s| s.generics)
                            .unwrap_or(0)
                    };
                    let converted: Vec<Ty> =
                        args.iter().map(|a| self.convert_ty(a, generics)).collect();
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
                    return if is_record {
                        Ty::Record {
                            def_id,
                            args: converted,
                        }
                    } else {
                        Ty::Sum {
                            def_id,
                            args: converted,
                        }
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
            ast::Type::Array(elem) => Ty::Array(Box::new(self.convert_ty(elem, generics))),
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
        let generics = generic_scope(&f.generics);
        self.check_fn_body(def_id, f, generics)
    }

    /// Compile an impl or trait-default method body.
    fn check_method(&mut self, def_id: DefId) -> Option<hir::Function> {
        let loc = *self.method_locs.get(&def_id)?;
        // `self.file` is a `&'a File`; copy the reference so the method
        // borrow is tied to `'a`, not to `self` (which we mutate below).
        let file: &'a ast::File = self.file;
        let f: &'a ast::Function = match loc.owner {
            MethodOwner::Impl => {
                let ast::Item::Impl(block) = &file.items[loc.item_index].value else {
                    return None;
                };
                &block.functions[loc.method_index]
            }
            MethodOwner::TraitDefault => {
                let ast::Item::Trait(decl) = &file.items[loc.item_index].value else {
                    return None;
                };
                match &decl.items[loc.method_index] {
                    TraitItem::Provided(f) => f,
                    TraitItem::Required(_) => return None,
                }
            }
        };
        let generics = match loc.owner {
            // An impl method sees the impl's generic parameters (`impl<T> …`).
            MethodOwner::Impl => match &file.items[loc.item_index].value {
                ast::Item::Impl(block) => generic_scope(&block.generics),
                _ => FxHashMap::default(),
            },
            MethodOwner::TraitDefault => self_generic_scope(),
        };
        self.check_fn_body(def_id, f, generics)
    }

    /// Shared body-checking for functions and methods.
    fn check_fn_body(
        &mut self,
        def_id: DefId,
        f: &ast::Function,
        generics: FxHashMap<String, u32>,
    ) -> Option<hir::Function> {
        let sig = self.sigs.get(&def_id)?.clone();
        let name = self.defs.def(def_id).name.clone();
        let mut fcx = FnCtx {
            icx: InferCtx::default(),
            locals: Vec::new(),
            scopes: vec![FxHashMap::default()],
            generics,
            param_bounds: sig.bounds.clone(),
            ret_ty: sig.ret.clone(),
            loop_depth: 0,
            pending_closures: Vec::new(),
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
                    "`{}` should return `{}` but its body has type `{}`",
                    name,
                    self.show(&sig.ret, &fcx),
                    self.show(&body.ty, &fcx),
                ),
                span,
            );
        }

        let mut func = hir::Function {
            def_id,
            name,
            generics: sig.generics,
            bounds: sig.bounds,
            takes_env: false,
            capture_count: 0,
            params: f.params.len() as u32,
            locals: fcx.locals,
            ret_ty: sig.ret,
            body,
            span: f.name.span,
        };
        self.finalize_function(&mut func, &fcx.icx);
        // Finalize and emit any closures/wrappers lifted from this body,
        // using the same inference context so their types resolve.
        let mut closures = std::mem::take(&mut fcx.pending_closures);
        for c in &mut closures {
            self.finalize_function(c, &fcx.icx);
        }
        self.extra_functions.append(&mut closures);
        Some(func)
    }

    /// Allocate a fresh synthetic `DefId` for a closure/wrapper function.
    fn fresh_closure_def(&mut self) -> DefId {
        let id = DefId(self.next_closure_def);
        self.next_closure_def += 1;
        id
    }

    /// Compile a constant as a zero-argument function returning its value.
    fn check_const(&mut self, def_id: DefId, item_index: usize) -> Option<hir::Function> {
        // Copy the `&'a File` reference so the value borrow outlives `&mut self`.
        let file: &'a ast::File = self.file;
        let ast::Item::Const(c) = &file.items[item_index].value else {
            return None;
        };
        let value_ast = &c.value;
        let sig = self.sigs.get(&def_id)?.clone();
        let name = self.defs.def(def_id).name.clone();
        let mut fcx = FnCtx {
            icx: InferCtx::default(),
            locals: Vec::new(),
            scopes: vec![FxHashMap::default()],
            generics: FxHashMap::default(),
            param_bounds: Vec::new(),
            ret_ty: sig.ret.clone(),
            loop_depth: 0,
            pending_closures: Vec::new(),
        };
        let value = self.check_expr(&mut fcx, value_ast);
        if !fcx.icx.unify(&value.ty, &sig.ret) {
            self.error(
                "E0010",
                format!(
                    "constant `{name}` has type `{}` but is declared `{}`",
                    self.show(&value.ty, &fcx),
                    self.show(&sig.ret, &fcx),
                ),
                value_ast.span,
            );
        }
        let mut func = hir::Function {
            def_id,
            name,
            generics: 0,
            bounds: Vec::new(),
            takes_env: false,
            capture_count: 0,
            params: 0,
            locals: fcx.locals,
            ret_ty: sig.ret,
            body: value,
            span: value_ast.span,
        };
        self.finalize_function(&mut func, &fcx.icx);
        let mut closures = std::mem::take(&mut fcx.pending_closures);
        for cl in &mut closures {
            self.finalize_function(cl, &fcx.icx);
        }
        self.extra_functions.append(&mut closures);
        Some(func)
    }

    /// Report `E0081` for a constant defined (transitively) in terms of
    /// itself — otherwise the generated zero-arg functions would recurse
    /// forever at run time.
    fn check_const_cycles(&mut self, functions: &[hir::Function]) {
        let const_ids: FxHashSet<DefId> = self.const_ids().into_iter().map(|(id, _)| id).collect();
        // Edges: const → the constants its value references.
        let mut edges: FxHashMap<DefId, Vec<DefId>> = FxHashMap::default();
        for f in functions {
            if !const_ids.contains(&f.def_id) {
                continue;
            }
            let mut callees = Vec::new();
            collect_const_calls(&f.body, &const_ids, &mut callees);
            edges.insert(f.def_id, callees);
        }
        // A constant is cyclic iff it can reach itself through the edges.
        let mut cyclic: Vec<DefId> = const_ids
            .iter()
            .copied()
            .filter(|&c| const_reaches_self(c, &edges))
            .collect();
        cyclic.sort();
        for c in cyclic {
            let name = &self.defs.def(c).name;
            self.diagnostics.push(
                Diagnostic::error(
                    "E0081",
                    format!("constant `{name}` is defined in terms of itself"),
                )
                .with_primary_label(self.defs.def(c).span, "cyclic constant")
                .with_note("a constant's value cannot depend on its own value".to_string()),
            );
        }
    }

    /// Apply the final substitution everywhere and report residual
    /// inference variables as E0011.
    fn finalize_function(&mut self, func: &mut hir::Function, icx: &InferCtx) {
        let mut residual: Vec<Span> = Vec::new();
        // Resolve the return type — for closures it may be an inference
        // variable (normal functions carry a concrete signature type).
        func.ret_ty = icx.apply(&func.ret_ty);
        if func.ret_ty.has_vars() {
            residual.push(func.span);
        }
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
                        // The branches' types unify only when neither
                        // diverges; the `if`'s type is the non-diverging
                        // branch (a diverging branch imposes no constraint —
                        // `Never` is the bottom type).
                        let then_d = matches!(fcx.icx.apply(&then_expr.ty), Ty::Never);
                        let else_d = matches!(fcx.icx.apply(&else_expr.ty), Ty::Never);
                        if !then_d && !else_d && !fcx.icx.unify(&then_expr.ty, &else_expr.ty) {
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
                        let ty = if then_d {
                            else_expr.ty.clone()
                        } else {
                            then_expr.ty.clone()
                        };
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
                // The condition is inside the loop for break/continue targeting
                // (e.g. `while (if done { break } else { c }) { ... }`).
                fcx.loop_depth += 1;
                let cond = self.check_expr(fcx, cond);
                self.expect_ty(fcx, &cond, &Ty::Bool, "a `while` condition");
                let body = self.check_block(fcx, &body.value, body.span);
                fcx.loop_depth -= 1;
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
            ast::Expr::For {
                pattern,
                iter,
                body,
            } => self.check_for(fcx, pattern, iter, body, span),
            ast::Expr::Range { .. } => {
                self.unsupported(span, "ranges outside a `for` loop");
                error_expr(span)
            }
            ast::Expr::Break(value) => {
                if value.is_some() {
                    self.unsupported(span, "`break` with a value");
                    return error_expr(span);
                }
                if fcx.loop_depth == 0 {
                    self.error("E0080", "`break` outside of a loop", span);
                    return error_expr(span);
                }
                hir::Expr {
                    kind: hir::ExprKind::Break,
                    ty: Ty::Never,
                    span,
                }
            }
            ast::Expr::Continue => {
                if fcx.loop_depth == 0 {
                    self.error("E0080", "`continue` outside of a loop", span);
                    return error_expr(span);
                }
                hir::Expr {
                    kind: hir::ExprKind::Continue,
                    ty: Ty::Never,
                    span,
                }
            }
            ast::Expr::Array(elems) => self.check_array_literal(fcx, elems, span),
            ast::Expr::Index { target, index } => self.check_index(fcx, target, index, span),
            // --- deferred constructs ---
            ast::Expr::Tuple(_) => self.unsupported_expr(span, "tuple expressions"),
            ast::Expr::Closure { params, ret, body } => {
                self.check_closure(fcx, params, ret, body, span)
            }
            ast::Expr::Record { path, fields, base } => {
                self.check_record_literal(fcx, path, fields, base.as_deref(), span)
            }
            ast::Expr::Field { target, field } => self.check_field(fcx, target, field, span),
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
                            // Bridge to a user-defined `Display` trait: if the
                            // value's type has a `fmt(self) -> String` in scope,
                            // interpolation calls it.
                            match self.try_display(fcx, value, &other) {
                                Some(fmt_call) => pieces.push(fmt_call),
                                None => {
                                    self.error(
                                        "E0013",
                                        format!(
                                            "`{}` cannot be interpolated into a string; \
                                             implement `Display` (fmt(self) -> String) for it",
                                            self.show(&other, fcx),
                                        ),
                                        // value was moved into try_display on the
                                        // Some path; use the interp span here.
                                        span,
                                    );
                                }
                            }
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
                    let type_args: Vec<Ty> = (0..sig.generics).map(|_| fcx.icx.fresh()).collect();
                    let param_types: Vec<Ty> =
                        sig.params.iter().map(|p| p.subst(&type_args)).collect();
                    let ret = sig.ret.subst(&type_args);
                    // A bare function used as a value becomes a fat pointer to
                    // a synthesized wrapper `(env, params) { def(params) }`.
                    self.make_fn_wrapper(fcx, def_id, type_args, param_types, ret, span)
                }
                DefKind::Const { .. } => {
                    // A constant reference is a call to its zero-arg function.
                    let ret = self
                        .sigs
                        .get(&def_id)
                        .map(|s| s.ret.clone())
                        .unwrap_or(Ty::Error);
                    hir::Expr {
                        kind: hir::ExprKind::Call {
                            func: hir::Callee::Def(def_id),
                            type_args: Vec::new(),
                            args: Vec::new(),
                        },
                        ty: ret,
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
        // Method call: `receiver.method(args)`.
        if let ast::Expr::Field { target, field } = &callee.value {
            let receiver = self.check_expr(fcx, target);
            let checked: Vec<hir::Expr> = args.iter().map(|a| self.check_expr(fcx, a)).collect();
            return self.check_method_call(fcx, receiver, field, checked, span);
        }

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

        // Indirect call through any fn-typed value expression (a local, a
        // constant of function type, a field, another call's result, …).
        let callee_expr = self.check_expr(fcx, callee);
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
                    "this value has type `{}` and cannot be called with these arguments",
                    self.show(&callee_expr.ty, fcx),
                ),
                callee_expr.span,
            );
            return error_expr(span);
        }
        // Indirect calls dispatch through a local. If the callee is already a
        // local, use it directly; otherwise bind it to a fresh local first
        // (evaluated before the arguments, preserving left-to-right order).
        if let hir::ExprKind::Local(local) = callee_expr.kind {
            return hir::Expr {
                kind: hir::ExprKind::Call {
                    func: hir::Callee::Local(local),
                    type_args: Vec::new(),
                    args: checked,
                },
                ty: ret,
                span,
            };
        }
        let callee_ty = callee_expr.ty.clone();
        let tmp =
            fcx.new_local_unscoped("__callee".to_string(), callee_ty, false, callee_expr.span);
        let bind = hir::Expr {
            kind: hir::ExprKind::Let {
                local: tmp,
                init: Box::new(callee_expr),
            },
            ty: Ty::Unit,
            span,
        };
        let call = hir::Expr {
            kind: hir::ExprKind::Call {
                func: hir::Callee::Local(tmp),
                type_args: Vec::new(),
                args: checked,
            },
            ty: ret.clone(),
            span,
        };
        hir::Expr {
            kind: hir::ExprKind::Block {
                stmts: vec![bind],
                trailing: Some(Box::new(call)),
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

    /// `for i in lo..hi { body }` desugars to a counter-driven `while`:
    /// `{ let i = lo; let end = hi; while i < end { body; i = i + 1 } }`
    fn check_array_literal(
        &mut self,
        fcx: &mut FnCtx,
        elems: &[Spanned<ast::Expr>],
        span: Span,
    ) -> hir::Expr {
        let elem_ty = fcx.icx.fresh();
        let mut checked = Vec::with_capacity(elems.len());
        for e in elems {
            let v = self.check_expr(fcx, e);
            if !fcx.icx.unify(&v.ty, &elem_ty) {
                self.error(
                    "E0010",
                    format!(
                        "array elements must share a type: expected `{}`, found `{}`",
                        self.show(&elem_ty, fcx),
                        self.show(&v.ty, fcx),
                    ),
                    e.span,
                );
            }
            checked.push(v);
        }
        hir::Expr {
            kind: hir::ExprKind::MakeArray { elems: checked },
            ty: Ty::Array(Box::new(elem_ty)),
            span,
        }
    }

    fn check_index(
        &mut self,
        fcx: &mut FnCtx,
        target: &Spanned<ast::Expr>,
        index: &Spanned<ast::Expr>,
        span: Span,
    ) -> hir::Expr {
        let arr = self.check_expr(fcx, target);
        let idx = self.check_expr(fcx, index);
        self.expect_ty(fcx, &idx, &Ty::Int, "an array index");
        let elem_ty = fcx.icx.fresh();
        if !fcx
            .icx
            .unify(&arr.ty, &Ty::Array(Box::new(elem_ty.clone())))
        {
            self.error(
                "E0014",
                format!("cannot index a value of type `{}`", self.show(&arr.ty, fcx)),
                target.span,
            );
            return error_expr(span);
        }
        hir::Expr {
            kind: hir::ExprKind::Index {
                target: Box::new(arr),
                index: Box::new(idx),
            },
            ty: elem_ty,
            span,
        }
    }

    /// (`<=` for an inclusive range). Phase 1 iterables are integer ranges.
    fn check_for(
        &mut self,
        fcx: &mut FnCtx,
        pattern: &Spanned<ast::Pattern>,
        iter: &Spanned<ast::Expr>,
        body: &Spanned<ast::Block>,
        span: Span,
    ) -> hir::Expr {
        let ast::Expr::Range { lo, hi, inclusive } = &iter.value else {
            self.unsupported(
                iter.span,
                "`for` loops over anything but an integer range (`a..b`)",
            );
            return error_expr(span);
        };

        let lo = self.check_expr(fcx, lo);
        self.expect_ty(fcx, &lo, &Ty::Int, "a range bound");
        let hi = self.check_expr(fcx, hi);
        self.expect_ty(fcx, &hi, &Ty::Int, "a range bound");

        // Hidden counter/bound locals are unscoped so they neither collide
        // with nor shadow source identifiers, and the counter is separate
        // from the user's (immutable) loop variable so assigning the loop
        // variable in the body is rejected (E0060) and cannot corrupt the
        // trip count.
        let counter = fcx.new_local_unscoped("__i".to_string(), Ty::Int, true, span);
        let end = fcx.new_local_unscoped("__end".to_string(), Ty::Int, false, span);

        fcx.scopes.push(FxHashMap::default());
        let (var_name, var_span) = match &pattern.value {
            ast::Pattern::Ident { name, .. } => (name.value.clone(), name.span),
            ast::Pattern::Wildcard => ("_".to_string(), pattern.span),
            _ => {
                self.error(
                    "E0022",
                    "a `for` loop variable must be a name or `_`",
                    pattern.span,
                );
                ("_".to_string(), pattern.span)
            }
        };
        let i = fcx.new_local(var_name, Ty::Int, false, var_span);
        fcx.loop_depth += 1;
        let body_hir = self.check_block(fcx, &body.value, body.span);
        fcx.loop_depth -= 1;
        fcx.scopes.pop();

        let int = |kind| hir::Expr {
            kind,
            ty: Ty::Int,
            span,
        };
        let read = |local| int(hir::ExprKind::Local(local));
        let assign = |local, value| hir::Expr {
            kind: hir::ExprKind::Assign {
                local,
                value: Box::new(value),
            },
            ty: Ty::Unit,
            span,
        };
        let let_stmt = |local, init| hir::Expr {
            kind: hir::ExprKind::Let {
                local,
                init: Box::new(init),
            },
            ty: Ty::Unit,
            span,
        };
        // Bind the immutable loop variable to the counter each iteration.
        let bind_i = let_stmt(i, read(counter));
        // increment: __i = __i + 1
        let incr = assign(
            counter,
            int(hir::ExprKind::Binary {
                op: hir::BinOp::Add,
                lhs: Box::new(read(counter)),
                rhs: Box::new(int(hir::ExprKind::IntLit(1))),
            }),
        );

        let mut outer_stmts = vec![let_stmt(counter, lo), let_stmt(end, hi)];

        let while_expr = if *inclusive {
            // Inclusive ranges use a run flag so that iterating up to
            // `Int::MAX` terminates instead of wrapping past the bound.
            let run = fcx.new_local_unscoped("__run".to_string(), Ty::Bool, true, span);
            let bool_expr = |kind| hir::Expr {
                kind,
                ty: Ty::Bool,
                span,
            };
            let cmp = |op| {
                bool_expr(hir::ExprKind::Binary {
                    op,
                    lhs: Box::new(read(counter)),
                    rhs: Box::new(read(end)),
                })
            };
            outer_stmts.push(let_stmt(run, cmp(hir::BinOp::Le)));
            // Advance the counter and run-flag *before* the body so that a
            // `continue` (which jumps to the loop header) still progresses.
            let update_run = assign(run, cmp(hir::BinOp::Lt));
            let while_body = hir::Expr {
                kind: hir::ExprKind::Block {
                    stmts: vec![bind_i, update_run, incr, body_hir],
                    trailing: None,
                },
                ty: Ty::Unit,
                span,
            };
            hir::Expr {
                kind: hir::ExprKind::While {
                    cond: Box::new(bool_expr(hir::ExprKind::Local(run))),
                    body: Box::new(while_body),
                },
                ty: Ty::Unit,
                span,
            }
        } else {
            let cond = hir::Expr {
                kind: hir::ExprKind::Binary {
                    op: hir::BinOp::Lt,
                    lhs: Box::new(read(counter)),
                    rhs: Box::new(read(end)),
                },
                ty: Ty::Bool,
                span,
            };
            // Increment before the body so `continue` still advances.
            let while_body = hir::Expr {
                kind: hir::ExprKind::Block {
                    stmts: vec![bind_i, incr, body_hir],
                    trailing: None,
                },
                ty: Ty::Unit,
                span,
            };
            hir::Expr {
                kind: hir::ExprKind::While {
                    cond: Box::new(cond),
                    body: Box::new(while_body),
                },
                ty: Ty::Unit,
                span,
            }
        };

        outer_stmts.push(while_expr);
        hir::Expr {
            kind: hir::ExprKind::Block {
                stmts: outer_stmts,
                trailing: None,
            },
            ty: Ty::Unit,
            span,
        }
    }

    /// Check a closure literal `|params| body`, lifting it into its own
    /// function. Free variables referring to enclosing locals are captured
    /// by value; the closure value is a fat pointer `{ code, env }`.
    fn check_closure(
        &mut self,
        fcx: &mut FnCtx,
        params: &[ast::Param],
        ret: &Option<Spanned<ast::Type>>,
        body: &Spanned<ast::Expr>,
        span: Span,
    ) -> hir::Expr {
        // Locals allocated below this index belong to enclosing scopes and
        // become captures if the body references them.
        let snapshot = fcx.locals.len() as u32;
        let generics_scope = fcx.generics.clone();

        fcx.scopes.push(FxHashMap::default());
        let mut param_locals = Vec::new();
        let mut param_types = Vec::new();
        for p in params {
            let ty = if matches!(p.ty.value, ast::Type::Infer) {
                fcx.icx.fresh()
            } else {
                self.convert_ty(&p.ty, &generics_scope)
            };
            let local = fcx.new_local(p.name.value.clone(), ty.clone(), p.is_mut, p.name.span);
            param_locals.push(local);
            param_types.push(ty);
        }
        let ret_ty = match ret {
            Some(rt) => self.convert_ty(rt, &generics_scope),
            None => fcx.icx.fresh(),
        };
        // A `break`/`continue` in the closure body cannot target a loop in
        // the enclosing function.
        let saved_loop_depth = fcx.loop_depth;
        fcx.loop_depth = 0;
        let body_hir = self.check_expr(fcx, body);
        fcx.loop_depth = saved_loop_depth;
        if !fcx.icx.unify(&body_hir.ty, &ret_ty) {
            self.error(
                "E0010",
                format!(
                    "closure body has type `{}` but its declared return type is `{}`",
                    self.show(&body_hir.ty, fcx),
                    self.show(&ret_ty, fcx),
                ),
                body.span,
            );
        }
        fcx.scopes.pop();

        let closure_ty = Ty::Fn {
            params: param_types,
            ret: Box::new(ret_ty.clone()),
        };
        self.lift_closure(
            fcx,
            snapshot,
            param_locals.len() as u32,
            ret_ty,
            body_hir,
            closure_ty,
            span,
        )
    }

    /// Extract a checked closure body into a standalone function, remapping
    /// captured locals to leading env slots and the closure's own locals to
    /// fresh indices. Returns the `MakeClosure` expression.
    #[allow(clippy::too_many_arguments)]
    fn lift_closure(
        &mut self,
        fcx: &mut FnCtx,
        snapshot: u32,
        param_count: u32,
        ret_ty: Ty,
        body: hir::Expr,
        closure_ty: Ty,
        span: Span,
    ) -> hir::Expr {
        // Captured locals: those referenced by the body with index < snapshot.
        let mut captures: Vec<LocalId> = Vec::new();
        collect_captures(&body, snapshot, &mut captures);
        let capture_count = captures.len() as u32;

        // Remap old local ids to the lifted function's local space:
        // captures → 0..C, then the closure's own locals (>= snapshot).
        let mut remap: FxHashMap<u32, u32> = FxHashMap::default();
        let mut new_locals: Vec<hir::Local> = Vec::new();
        for (i, cap) in captures.iter().enumerate() {
            remap.insert(cap.0, i as u32);
            new_locals.push(fcx.locals[cap.0 as usize].clone());
        }
        for old in (snapshot as usize)..fcx.locals.len() {
            let new_id = capture_count + (old as u32 - snapshot);
            remap.insert(old as u32, new_id);
            new_locals.push(fcx.locals[old].clone());
        }

        let mut lifted_body = body;
        remap_locals(&mut lifted_body, &remap);

        let generics = fcx.generics.len() as u32;
        let type_args: Vec<Ty> = (0..generics).map(Ty::Param).collect();
        let cdef = self.fresh_closure_def();
        let closure_fn = hir::Function {
            def_id: cdef,
            name: format!("closure${}", cdef.0),
            generics,
            bounds: fcx.param_bounds.clone(),
            takes_env: true,
            capture_count,
            params: param_count,
            locals: new_locals,
            ret_ty,
            body: lifted_body,
            span,
        };
        fcx.pending_closures.push(closure_fn);

        let capture_exprs: Vec<hir::Expr> = captures
            .iter()
            .map(|c| hir::Expr {
                kind: hir::ExprKind::Local(*c),
                ty: fcx.locals[c.0 as usize].ty.clone(),
                span,
            })
            .collect();
        hir::Expr {
            kind: hir::ExprKind::MakeClosure {
                func: cdef,
                type_args,
                captures: capture_exprs,
            },
            ty: closure_ty,
            span,
        }
    }

    /// Synthesize a fat-pointer wrapper for a bare function used as a value:
    /// `(env, params) { target(params) }`, captured environment empty.
    fn make_fn_wrapper(
        &mut self,
        fcx: &mut FnCtx,
        target: DefId,
        target_type_args: Vec<Ty>,
        param_types: Vec<Ty>,
        ret: Ty,
        span: Span,
    ) -> hir::Expr {
        let generics = fcx.generics.len() as u32;
        let type_args: Vec<Ty> = (0..generics).map(Ty::Param).collect();
        let cdef = self.fresh_closure_def();

        let mut locals = Vec::new();
        let mut call_args = Vec::new();
        for (i, pty) in param_types.iter().enumerate() {
            locals.push(hir::Local {
                name: format!("__a{i}"),
                ty: pty.clone(),
                is_mut: false,
                span,
            });
            call_args.push(hir::Expr {
                kind: hir::ExprKind::Local(LocalId(i as u32)),
                ty: pty.clone(),
                span,
            });
        }
        let body = hir::Expr {
            kind: hir::ExprKind::Call {
                func: hir::Callee::Def(target),
                type_args: target_type_args,
                args: call_args,
            },
            ty: ret.clone(),
            span,
        };
        let wrapper = hir::Function {
            def_id: cdef,
            name: format!("fnval${}", cdef.0),
            generics,
            bounds: fcx.param_bounds.clone(),
            takes_env: true,
            capture_count: 0,
            params: param_types.len() as u32,
            locals,
            ret_ty: ret.clone(),
            body,
            span,
        };
        fcx.pending_closures.push(wrapper);
        hir::Expr {
            kind: hir::ExprKind::MakeClosure {
                func: cdef,
                type_args,
                captures: Vec::new(),
            },
            ty: Ty::Fn {
                params: param_types,
                ret: Box::new(ret),
            },
            span,
        }
    }

    fn check_record_literal(
        &mut self,
        fcx: &mut FnCtx,
        path: &ast::Path,
        fields: &[ast::FieldInit],
        base: Option<&Spanned<ast::Expr>>,
        span: Span,
    ) -> hir::Expr {
        if path.segments.len() != 1 {
            self.unsupported(span, "module-qualified record paths");
            return error_expr(span);
        }
        let name = path.segments[0].value.as_str();
        let Some(def_id) = self.defs.resolve_type(name) else {
            self.error("E0001", format!("cannot find record `{name}`"), span);
            return error_expr(span);
        };
        let Some(record) = self.records.iter().find(|r| r.def_id == def_id).cloned() else {
            self.error("E0010", format!("`{name}` is not a record type"), span);
            return error_expr(span);
        };
        let type_args: Vec<Ty> = (0..record.generics).map(|_| fcx.icx.fresh()).collect();
        let record_ty = Ty::Record {
            def_id,
            args: type_args.clone(),
        };

        // Evaluate initializers into locals in *source order* (then the
        // base), so their side effects fire left-to-right, consistent with
        // how function arguments and `let` bindings evaluate. Field slots
        // are then assembled from these locals in declaration order.
        let mut stmts: Vec<hir::Expr> = Vec::new();
        let mut provided: FxHashMap<String, LocalId> = FxHashMap::default();
        for init in fields {
            let fname = init.name.value.clone();
            let Some(field) = record.fields.iter().find(|f| f.name == fname) else {
                self.error(
                    "E0014",
                    format!("record `{name}` has no field `{fname}`"),
                    init.name.span,
                );
                continue;
            };
            let expected = field.ty.subst(&type_args);
            let value = match &init.value {
                Some(v) => self.check_expr(fcx, v),
                // Shorthand `{ x }` binds the local named `x`.
                None => self.check_path(fcx, &ast::Path::single(init.name.clone()), init.name.span),
            };
            if !fcx.icx.unify(&value.ty, &expected) {
                self.error(
                    "E0010",
                    format!(
                        "field `{fname}` expects `{}` but found `{}`",
                        self.show(&expected, fcx),
                        self.show(&value.ty, fcx),
                    ),
                    init.name.span,
                );
            }
            let value_ty = value.ty.clone();
            let local = fcx.new_local(format!("__f_{fname}"), value_ty, false, init.name.span);
            stmts.push(hir::Expr {
                kind: hir::ExprKind::Let {
                    local,
                    init: Box::new(value),
                },
                ty: Ty::Unit,
                span,
            });
            if provided.insert(fname.clone(), local).is_some() {
                self.error(
                    "E0014",
                    format!("field `{fname}` specified more than once"),
                    init.name.span,
                );
            }
        }

        // A `..base` spread fills any fields not given explicitly. Bind the
        // base to a local so it is evaluated exactly once, after the fields.
        let base_local = match base {
            Some(base_expr) => {
                let checked = self.check_expr(fcx, base_expr);
                if !fcx.icx.unify(&checked.ty, &record_ty) {
                    self.error(
                        "E0010",
                        format!(
                            "the `..` base has type `{}` but `{name}` was expected",
                            self.show(&checked.ty, fcx),
                        ),
                        base_expr.span,
                    );
                }
                let local = fcx.new_local("__base".to_string(), record_ty.clone(), false, span);
                stmts.push(hir::Expr {
                    kind: hir::ExprKind::Let {
                        local,
                        init: Box::new(checked),
                    },
                    ty: Ty::Unit,
                    span,
                });
                Some(local)
            }
            None => None,
        };

        // Assemble field slots in declaration order from the evaluated locals.
        let mut ordered = Vec::with_capacity(record.fields.len());
        let mut missing = Vec::new();
        for (idx, field) in record.fields.iter().enumerate() {
            let field_ty = field.ty.subst(&type_args);
            if let Some(local) = provided.get(&field.name) {
                ordered.push(hir::Expr {
                    kind: hir::ExprKind::Local(*local),
                    ty: field_ty,
                    span,
                });
            } else if let Some(base_local) = base_local {
                ordered.push(hir::Expr {
                    kind: hir::ExprKind::FieldGet {
                        target: Box::new(hir::Expr {
                            kind: hir::ExprKind::Local(base_local),
                            ty: record_ty.clone(),
                            span,
                        }),
                        index: idx as u32,
                    },
                    ty: field_ty,
                    span,
                });
            } else {
                missing.push(format!("`{}`", field.name));
            }
        }
        if !missing.is_empty() {
            self.error(
                "E0014",
                format!("missing field(s) in `{name}`: {}", missing.join(", ")),
                span,
            );
            return error_expr(span);
        }

        let make = hir::Expr {
            kind: hir::ExprKind::MakeRecord {
                record: def_id,
                fields: ordered,
            },
            ty: record_ty.clone(),
            span,
        };
        if stmts.is_empty() {
            make
        } else {
            hir::Expr {
                kind: hir::ExprKind::Block {
                    stmts,
                    trailing: Some(Box::new(make)),
                },
                ty: record_ty,
                span,
            }
        }
    }

    fn check_field(
        &mut self,
        fcx: &mut FnCtx,
        target: &Spanned<ast::Expr>,
        field: &Spanned<String>,
        span: Span,
    ) -> hir::Expr {
        let recv = self.check_expr(fcx, target);
        match fcx.icx.apply(&recv.ty) {
            Ty::Record { def_id, args } => {
                let record = self
                    .records
                    .iter()
                    .find(|r| r.def_id == def_id)
                    .expect("record type resolves to a record def");
                match record.fields.iter().position(|f| f.name == field.value) {
                    Some(idx) => {
                        let field_ty = record.fields[idx].ty.subst(&args);
                        hir::Expr {
                            kind: hir::ExprKind::FieldGet {
                                target: Box::new(recv),
                                index: idx as u32,
                            },
                            ty: field_ty,
                            span,
                        }
                    }
                    None => {
                        self.error(
                            "E0014",
                            format!("no field `{}` on record `{}`", field.value, record.name),
                            field.span,
                        );
                        error_expr(span)
                    }
                }
            }
            Ty::Error => error_expr(span),
            other => {
                self.error(
                    "E0014",
                    format!(
                        "cannot access field `{}` on `{}`",
                        field.value,
                        self.show(&other, fcx)
                    ),
                    field.span,
                );
                error_expr(span)
            }
        }
    }

    /// Resolve a method `name` on a receiver of (resolved) type `recv_ty`,
    /// without emitting diagnostics.
    fn resolve_method_on(&self, recv_ty: &Ty, fcx: &FnCtx, name: &str) -> MethodRes {
        match recv_ty {
            Ty::Param(k) => {
                let bounds = fcx.param_bounds.get(*k as usize);
                let matches: Vec<(DefId, u32)> = bounds
                    .into_iter()
                    .flatten()
                    .filter_map(|&tid| self.trait_method_index(tid, name).map(|i| (tid, i)))
                    .collect();
                match matches.len() {
                    0 => MethodRes::None,
                    1 => MethodRes::Trait(matches[0].0, matches[0].1),
                    _ => MethodRes::Ambiguous,
                }
            }
            _ => {
                let Some(head) = recv_ty.head() else {
                    return MethodRes::None;
                };
                // Inherent methods take priority over trait methods — but only
                // an impl that actually fits the receiver counts. A restricted
                // inherent impl (`impl<T> Pair<T, T>`) must not shadow an
                // applicable trait method when the receiver (`Pair<Int, Str>`)
                // does not fit it.
                if let Some(def) = self.find_inherent_method(recv_ty, head, name) {
                    return MethodRes::Inherent(def);
                }
                let matches: Vec<(DefId, u32)> = self
                    .impls
                    .iter()
                    .filter(|i| i.self_head == head && i.match_args(recv_ty).is_some())
                    .filter_map(|i| i.trait_id)
                    .filter_map(|tid| self.trait_method_index(tid, name).map(|idx| (tid, idx)))
                    .collect();
                match matches.len() {
                    0 => MethodRes::None,
                    1 => MethodRes::Trait(matches[0].0, matches[0].1),
                    _ => MethodRes::Ambiguous,
                }
            }
        }
    }

    fn trait_method_index(&self, trait_id: DefId, name: &str) -> Option<u32> {
        self.traits
            .iter()
            .find(|t| t.def_id == trait_id)?
            .methods
            .iter()
            .position(|m| m.name == name)
            .map(|i| i as u32)
    }

    fn find_inherent_method(&self, recv_ty: &Ty, head: TyHead, name: &str) -> Option<DefId> {
        self.impls
            .iter()
            .filter(|i| i.trait_id.is_none() && i.self_head == head)
            // The receiver must fit the impl's self-type pattern, not just its
            // head, so `impl<T> Pair<T, T>` is skipped for `Pair<Int, String>`.
            .filter(|i| i.match_args(recv_ty).is_some())
            .find_map(|i| i.methods.iter().find(|(n, _)| n == name).map(|(_, d)| *d))
    }

    fn check_method_call(
        &mut self,
        fcx: &mut FnCtx,
        receiver: hir::Expr,
        method: &Spanned<String>,
        args: Vec<hir::Expr>,
        span: Span,
    ) -> hir::Expr {
        let recv_ty = fcx.icx.apply(&receiver.ty);
        if matches!(recv_ty, Ty::Error) {
            return error_expr(span);
        }
        if matches!(recv_ty, Ty::Var(_)) {
            self.error(
                "E0011",
                "cannot infer the receiver's type; add a type annotation",
                receiver.span,
            );
            return error_expr(span);
        }
        // Built-in array methods.
        if matches!(recv_ty, Ty::Array(_)) {
            if method.value == "len" && args.is_empty() {
                return hir::Expr {
                    kind: hir::ExprKind::ArrayLen {
                        target: Box::new(receiver),
                    },
                    ty: Ty::Int,
                    span,
                };
            }
            self.error(
                "E0014",
                format!(
                    "no method `{}` on array type `{}`",
                    method.value,
                    self.show(&recv_ty, fcx)
                ),
                method.span,
            );
            return error_expr(span);
        }
        match self.resolve_method_on(&recv_ty, fcx, &method.value) {
            MethodRes::Inherent(def_id) => {
                self.emit_inherent_call(fcx, def_id, receiver, args, span)
            }
            MethodRes::Trait(trait_id, method_idx) => {
                self.emit_trait_call(fcx, trait_id, method_idx, receiver, args, span)
            }
            MethodRes::Ambiguous => {
                self.error(
                    "E0015",
                    format!(
                        "ambiguous method call `{}`: more than one trait in scope provides it",
                        method.value
                    ),
                    method.span,
                );
                error_expr(span)
            }
            MethodRes::None => {
                self.error(
                    "E0014",
                    format!(
                        "no method `{}` on type `{}`",
                        method.value,
                        self.show(&recv_ty, fcx)
                    ),
                    method.span,
                );
                error_expr(span)
            }
        }
    }

    fn emit_inherent_call(
        &mut self,
        fcx: &mut FnCtx,
        def_id: DefId,
        receiver: hir::Expr,
        args: Vec<hir::Expr>,
        span: Span,
    ) -> hir::Expr {
        let Some(sig) = self.sigs.get(&def_id).cloned() else {
            return error_expr(span);
        };
        // sig.params[0] is `self`; the rest are the declared parameters.
        let expected_args = sig.params.len().saturating_sub(1);
        if args.len() != expected_args {
            self.error(
                "E0016",
                format!(
                    "method takes {expected_args} argument(s) but {} were supplied",
                    args.len()
                ),
                span,
            );
            return error_expr(span);
        }
        // A method on a generic impl (`impl<T> Box<T> { … }`) is generic over
        // the impl's parameters; instantiate them with fresh inference vars so
        // the receiver/args recover them (e.g. `T = Int` from a `Box<Int>`).
        let type_args: Vec<Ty> = (0..sig.generics).map(|_| fcx.icx.fresh()).collect();
        let self_param = sig.params[0].subst(&type_args);
        // The receiver must fit the impl's self-type pattern exactly — for a
        // repeated/partially-concrete self type (`impl<T> Pair<T, T>`) merely
        // sharing a head is not enough, and a silent mismatch would dispatch to
        // a wrongly specialized method.
        if !fcx.icx.unify(&receiver.ty, &self_param) {
            let mname = self.defs.def(def_id).name.clone();
            self.error(
                "E0014",
                format!(
                    "method `{mname}` does not apply to receiver of type `{}`",
                    self.show(&receiver.ty, fcx),
                ),
                span,
            );
            return error_expr(span);
        }
        for (arg, param) in args.iter().zip(sig.params[1..].iter()) {
            let expected = param.subst(&type_args);
            if !fcx.icx.unify(&arg.ty, &expected) {
                self.error(
                    "E0010",
                    format!(
                        "argument has type `{}` but `{}` was expected",
                        self.show(&arg.ty, fcx),
                        self.show(&expected, fcx),
                    ),
                    arg.span,
                );
            }
        }
        let ret = sig.ret.subst(&type_args);
        let mut call_args = Vec::with_capacity(args.len() + 1);
        call_args.push(receiver);
        call_args.extend(args);
        hir::Expr {
            kind: hir::ExprKind::Call {
                func: hir::Callee::Def(def_id),
                type_args,
                args: call_args,
            },
            ty: ret,
            span,
        }
    }

    fn emit_trait_call(
        &mut self,
        fcx: &mut FnCtx,
        trait_id: DefId,
        method_idx: u32,
        receiver: hir::Expr,
        args: Vec<hir::Expr>,
        span: Span,
    ) -> hir::Expr {
        let self_ty = fcx.icx.apply(&receiver.ty);
        let tm = self.traits[self
            .traits
            .iter()
            .position(|t| t.def_id == trait_id)
            .expect("trait exists")]
        .methods[method_idx as usize]
            .clone();
        // Substitute `Self` (`Param(0)`) with the receiver type.
        let subst = [self_ty.clone()];
        if args.len() != tm.params.len() {
            self.error(
                "E0016",
                format!(
                    "method `{}` takes {} argument(s) but {} were supplied",
                    tm.name,
                    tm.params.len(),
                    args.len()
                ),
                span,
            );
            return error_expr(span);
        }
        for (arg, param) in args.iter().zip(tm.params.iter()) {
            let expected = param.subst(&subst);
            if !fcx.icx.unify(&arg.ty, &expected) {
                self.error(
                    "E0010",
                    format!(
                        "argument has type `{}` but `{}` was expected",
                        self.show(&arg.ty, fcx),
                        self.show(&expected, fcx),
                    ),
                    arg.span,
                );
            }
        }
        hir::Expr {
            kind: hir::ExprKind::TraitCall {
                trait_id,
                method: method_idx,
                self_ty: self_ty.clone(),
                receiver: Box::new(receiver),
                args,
            },
            ty: tm.ret.subst(&subst),
            span,
        }
    }

    /// If `recv_ty` has a `Display`-style `fmt(self) -> String` method in
    /// scope, build the call that produces its string. Used to interpolate
    /// user types. Returns `None` (leaving `value` consumed) if no such
    /// method resolves.
    fn try_display(
        &mut self,
        fcx: &mut FnCtx,
        value: hir::Expr,
        recv_ty: &Ty,
    ) -> Option<hir::Expr> {
        let (trait_id, method_idx) = match self.resolve_method_on(recv_ty, fcx, "fmt") {
            MethodRes::Trait(t, i) => (t, i),
            _ => return None,
        };
        // The method must take no extra args and return `String`.
        let tm = &self.traits.iter().find(|t| t.def_id == trait_id)?.methods[method_idx as usize];
        if !tm.params.is_empty() || tm.ret != Ty::String {
            return None;
        }
        let span = value.span;
        Some(self.emit_trait_call(fcx, trait_id, method_idx, value, Vec::new(), span))
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
        // Element assignment `arr[i] = v`.
        if let ast::Expr::Index { target, index } = &lhs.value {
            return self.check_index_set(fcx, op, target, index, rhs, span);
        }
        let ast::Expr::Path(path) = &lhs.value else {
            self.unsupported(
                lhs.span,
                "assignment to anything but a local variable or array element",
            );
            return error_expr(span);
        };
        let name = if path.segments.len() == 1 {
            path.segments[0].value.as_str()
        } else {
            self.unsupported(lhs.span, "assignment to paths");
            return error_expr(span);
        };
        let Some(local) = fcx.lookup(name) else {
            self.error(
                "E0001",
                format!("cannot find `{name}` in this scope"),
                lhs.span,
            );
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
                .push(format!(
                    "declare it as `let mut {name}` to allow assignment"
                ));
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

    /// Classify the root binding of an lvalue place, walking through element
    /// (`arr[i]`) and field (`rec.f`) projections. The mutability of the base
    /// binding governs whether the reachable heap storage may be mutated.
    fn place_root(&self, fcx: &FnCtx, expr: &Spanned<ast::Expr>) -> PlaceRoot {
        match &expr.value {
            ast::Expr::Path(p) if p.segments.len() == 1 => {
                match fcx.lookup(&p.segments[0].value) {
                    Some(l) if fcx.locals[l.0 as usize].is_mut => PlaceRoot::Mutable,
                    Some(_) => PlaceRoot::ImmutableLocal(p.segments[0].value.clone()),
                    // A constant, a multi-segment path, or an unknown name is
                    // not a mutable place.
                    None => PlaceRoot::NotAPlace,
                }
            }
            ast::Expr::Index { target, .. } => self.place_root(fcx, target),
            ast::Expr::Field { target, .. } => self.place_root(fcx, target),
            // Any other base (call result, literal, block, …) is a temporary
            // with no assignable root.
            _ => PlaceRoot::NotAPlace,
        }
    }

    /// Check `arr[index] = value`.
    fn check_index_set(
        &mut self,
        fcx: &mut FnCtx,
        op: ast::AssignOp,
        target: &Spanned<ast::Expr>,
        index: &Spanned<ast::Expr>,
        rhs: &Spanned<ast::Expr>,
        span: Span,
    ) -> hir::Expr {
        if !matches!(op, ast::AssignOp::Assign) {
            self.unsupported(span, "compound assignment to an array element");
            return error_expr(span);
        }
        // The array's storage must be reachable through a mutable binding.
        // Walk the whole index/field chain to its root local — `grid[0][1]`,
        // `rec.data[0]`, and `make()[0]` all bypass a single-segment check.
        match self.place_root(fcx, target) {
            PlaceRoot::Mutable => {}
            PlaceRoot::ImmutableLocal(name) => {
                self.error(
                    "E0060",
                    format!("cannot assign to an element of immutable `{name}`"),
                    span,
                );
                self.diagnostics
                    .last_mut()
                    .expect("just pushed")
                    .notes
                    .push(format!("declare it as `let mut {name}` to allow mutation"));
            }
            PlaceRoot::NotAPlace => {
                self.error(
                    "E0060",
                    "cannot assign to an element of a temporary or non-assignable value"
                        .to_string(),
                    span,
                );
            }
        }
        let arr = self.check_expr(fcx, target);
        let idx = self.check_expr(fcx, index);
        self.expect_ty(fcx, &idx, &Ty::Int, "an array index");
        let elem_ty = fcx.icx.fresh();
        if !fcx
            .icx
            .unify(&arr.ty, &Ty::Array(Box::new(elem_ty.clone())))
        {
            self.error(
                "E0014",
                format!(
                    "cannot index-assign a value of type `{}`",
                    self.show(&arr.ty, fcx)
                ),
                target.span,
            );
            return error_expr(span);
        }
        let value = self.check_expr(fcx, rhs);
        if !fcx.icx.unify(&value.ty, &elem_ty) {
            self.error(
                "E0010",
                format!(
                    "array element has type `{}` but `{}` was assigned",
                    self.show(&elem_ty, fcx),
                    self.show(&value.ty, fcx),
                ),
                rhs.span,
            );
        }
        hir::Expr {
            kind: hir::ExprKind::IndexSet {
                target: Box::new(arr),
                index: Box::new(idx),
                value: Box::new(value),
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
            // A diverging arm (`Never`) imposes no constraint on the result
            // type; only unify arms that actually produce a value, so a
            // `break`/`return`/`continue` arm does not pin the match to
            // `Never`.
            if !matches!(fcx.icx.apply(&body.ty), Ty::Never) && !fcx.icx.unify(&body.ty, &result_ty)
            {
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

        // If every arm diverged the result was never constrained; it is then
        // itself `Never`.
        let result_ty = if matches!(fcx.icx.apply(&result_ty), Ty::Var(_)) {
            Ty::Never
        } else {
            result_ty
        };
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
                let local = fcx.new_local(name.value.clone(), scrut_ty.clone(), *is_mut, name.span);
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
                            return self.variant_pattern(
                                fcx,
                                sum_id,
                                vi,
                                &[],
                                scrut_ty,
                                pattern.span,
                            );
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
                    let local = fcx.new_local(name.value.clone(), bound_ty, *is_mut, name.span);
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

/// Collect the constants a constant's body references (calls to zero-arg
/// const functions), for cycle detection.
fn collect_const_calls(expr: &hir::Expr, consts: &FxHashSet<DefId>, out: &mut Vec<DefId>) {
    if let hir::ExprKind::Call {
        func: hir::Callee::Def(d),
        ..
    } = &expr.kind
    {
        if consts.contains(d) && !out.contains(d) {
            out.push(*d);
        }
    }
    for child in child_exprs(expr) {
        collect_const_calls(child, consts, out);
    }
}

/// Whether constant `start` can reach itself through the dependency edges
/// (i.e. participates in a cycle).
fn const_reaches_self(start: DefId, edges: &FxHashMap<DefId, Vec<DefId>>) -> bool {
    let mut stack: Vec<DefId> = edges.get(&start).cloned().unwrap_or_default();
    let mut seen: FxHashSet<DefId> = FxHashSet::default();
    while let Some(n) = stack.pop() {
        if n == start {
            return true;
        }
        if !seen.insert(n) {
            continue;
        }
        if let Some(succ) = edges.get(&n) {
            stack.extend(succ.iter().copied());
        }
    }
    false
}

/// Walk a closure body and collect the locals it captures — those with an
/// index below `snapshot` (declared in an enclosing scope) — in first-seen
/// order, without duplicates.
fn collect_captures(expr: &hir::Expr, snapshot: u32, out: &mut Vec<LocalId>) {
    use hir::ExprKind as K;
    let mut consider = |id: &LocalId| {
        if id.0 < snapshot && !out.contains(id) {
            out.push(*id);
        }
    };
    // Every position that *references* an existing local is a capture
    // candidate: reads, assignment targets, and indirect-call callees. (A
    // `Let` binder and match binders introduce fresh locals — never < the
    // snapshot — so they are not candidates.)
    match &expr.kind {
        K::Local(id) => consider(id),
        K::Assign { local, .. } => consider(local),
        K::Call {
            func: hir::Callee::Local(id),
            ..
        } => consider(id),
        _ => {}
    }
    for child in child_exprs(expr) {
        collect_captures(child, snapshot, out);
    }
}

/// Rewrite every local id in a lifted closure body through `map`.
fn remap_locals(expr: &mut hir::Expr, map: &FxHashMap<u32, u32>) {
    use hir::ExprKind as K;
    let remap = |id: &mut LocalId| {
        if let Some(&new) = map.get(&id.0) {
            id.0 = new;
        }
    };
    match &mut expr.kind {
        K::Local(id) => remap(id),
        K::Let { local, .. } | K::Assign { local, .. } => remap(local),
        K::Call {
            func: hir::Callee::Local(id),
            ..
        } => remap(id),
        K::Match { arms, .. } => {
            for arm in arms.iter_mut() {
                remap_pattern(&mut arm.pattern, &remap);
            }
        }
        _ => {}
    }
    for child in child_exprs_mut(&mut expr.kind) {
        remap_locals(child, map);
    }
}

fn remap_pattern(pat: &mut hir::Pattern, remap: &impl Fn(&mut LocalId)) {
    match pat {
        hir::Pattern::Bind(local) => remap(local),
        hir::Pattern::Variant { binders, .. } => {
            for b in binders.iter_mut().flatten() {
                remap(b);
            }
        }
        _ => {}
    }
}

/// Immutable iterator over an expression's direct sub-expressions.
fn child_exprs(expr: &hir::Expr) -> Vec<&hir::Expr> {
    use hir::ExprKind as K;
    let mut out: Vec<&hir::Expr> = Vec::new();
    match &expr.kind {
        K::MakeClosure { captures, .. } => out.extend(captures.iter()),
        K::Call { args, .. } => out.extend(args.iter()),
        K::MakeVariant { args, .. }
        | K::MakeRecord { fields: args, .. }
        | K::MakeArray { elems: args }
        | K::StrConcat(args) => out.extend(args.iter()),
        K::FieldGet { target, .. } | K::ArrayLen { target } => out.push(target),
        K::Index { target, index } => {
            out.push(target);
            out.push(index);
        }
        K::IndexSet {
            target,
            index,
            value,
        } => {
            out.push(target);
            out.push(index);
            out.push(value);
        }
        K::TraitCall { receiver, args, .. } => {
            out.push(receiver);
            out.extend(args.iter());
        }
        K::Binary { lhs, rhs, .. } | K::LogicalAnd { lhs, rhs } | K::LogicalOr { lhs, rhs } => {
            out.push(lhs);
            out.push(rhs);
        }
        K::Unary { expr: e, .. } | K::ToStr(e) => out.push(e),
        K::Let { init, .. } => out.push(init),
        K::Assign { value, .. } => out.push(value),
        K::Block { stmts, trailing } => {
            out.extend(stmts.iter());
            if let Some(t) = trailing {
                out.push(t);
            }
        }
        K::If { cond, then, else_ } => {
            out.push(cond);
            out.push(then);
            if let Some(e) = else_ {
                out.push(e);
            }
        }
        K::While { cond, body } => {
            out.push(cond);
            out.push(body);
        }
        K::Match { scrutinee, arms } => {
            out.push(scrutinee);
            out.extend(arms.iter().map(|a| &a.body));
        }
        K::Return(v) => {
            if let Some(v) = v {
                out.push(v);
            }
        }
        K::IntLit(_)
        | K::FloatLit(_)
        | K::BoolLit(_)
        | K::StrLit(_)
        | K::CharLit(_)
        | K::Unit
        | K::Break
        | K::Continue
        | K::Local(_) => {}
    }
    out
}

/// Mutable iterator over a kind's direct sub-expressions.
fn child_exprs_mut(kind: &mut hir::ExprKind) -> Vec<&mut hir::Expr> {
    use hir::ExprKind as K;
    let mut out: Vec<&mut hir::Expr> = Vec::new();
    match kind {
        K::MakeClosure { captures, .. } => out.extend(captures.iter_mut()),
        K::Call { args, .. } => out.extend(args.iter_mut()),
        K::MakeVariant { args, .. }
        | K::MakeRecord { fields: args, .. }
        | K::MakeArray { elems: args }
        | K::StrConcat(args) => out.extend(args.iter_mut()),
        K::FieldGet { target, .. } | K::ArrayLen { target } => out.push(target),
        K::Index { target, index } => {
            out.push(target);
            out.push(index);
        }
        K::IndexSet {
            target,
            index,
            value,
        } => {
            out.push(target);
            out.push(index);
            out.push(value);
        }
        K::TraitCall { receiver, args, .. } => {
            out.push(receiver);
            out.extend(args.iter_mut());
        }
        K::Binary { lhs, rhs, .. } | K::LogicalAnd { lhs, rhs } | K::LogicalOr { lhs, rhs } => {
            out.push(lhs);
            out.push(rhs);
        }
        K::Unary { expr: e, .. } | K::ToStr(e) => out.push(e),
        K::Let { init, .. } => out.push(init),
        K::Assign { value, .. } => out.push(value),
        K::Block { stmts, trailing } => {
            out.extend(stmts.iter_mut());
            if let Some(t) = trailing {
                out.push(t);
            }
        }
        K::If { cond, then, else_ } => {
            out.push(cond);
            out.push(then);
            if let Some(e) = else_ {
                out.push(e);
            }
        }
        K::While { cond, body } => {
            out.push(cond);
            out.push(body);
        }
        K::Match { scrutinee, arms } => {
            out.push(scrutinee);
            out.extend(arms.iter_mut().map(|a| &mut a.body));
        }
        K::Return(v) => {
            if let Some(v) = v {
                out.push(v);
            }
        }
        K::IntLit(_)
        | K::FloatLit(_)
        | K::BoolLit(_)
        | K::StrLit(_)
        | K::CharLit(_)
        | K::Unit
        | K::Break
        | K::Continue
        | K::Local(_) => {}
    }
    out
}

fn generic_scope(generics: &[ast::TypeParam]) -> FxHashMap<String, u32> {
    generics
        .iter()
        .enumerate()
        .map(|(i, g)| (g.name.value.clone(), i as u32))
        .collect()
}

/// The generic scope inside a trait definition / default method: `Self`
/// maps to `Param(0)`.
fn self_generic_scope() -> FxHashMap<String, u32> {
    let mut m = FxHashMap::default();
    m.insert("Self".to_string(), 0u32);
    m
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

/// The mutability classification of the root binding of an assignment place.
enum PlaceRoot {
    /// Rooted at a mutable local — mutation through it is allowed.
    Mutable,
    /// Rooted at an immutable local of the given name.
    ImmutableLocal(String),
    /// No assignable root: a temporary (call result, literal), a constant, or
    /// an unresolved/multi-segment path.
    NotAPlace,
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
        hir::ExprKind::Call {
            type_args, args, ..
        } => {
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
        hir::ExprKind::MakeClosure {
            type_args,
            captures,
            ..
        } => {
            for t in type_args.iter_mut() {
                *t = icx.apply(t);
                if t.has_vars() {
                    residual.push(expr.span);
                }
            }
            for c in captures {
                finalize_expr(c, icx, residual);
            }
        }
        hir::ExprKind::MakeVariant { args, .. }
        | hir::ExprKind::MakeRecord { fields: args, .. }
        | hir::ExprKind::MakeArray { elems: args }
        | hir::ExprKind::StrConcat(args) => {
            for a in args {
                finalize_expr(a, icx, residual);
            }
        }
        hir::ExprKind::FieldGet { target, .. } | hir::ExprKind::ArrayLen { target } => {
            finalize_expr(target, icx, residual);
        }
        hir::ExprKind::Index { target, index } => {
            finalize_expr(target, icx, residual);
            finalize_expr(index, icx, residual);
        }
        hir::ExprKind::IndexSet {
            target,
            index,
            value,
        } => {
            finalize_expr(target, icx, residual);
            finalize_expr(index, icx, residual);
            finalize_expr(value, icx, residual);
        }
        hir::ExprKind::TraitCall {
            self_ty,
            receiver,
            args,
            ..
        } => {
            *self_ty = icx.apply(self_ty);
            if self_ty.has_vars() {
                residual.push(expr.span);
            }
            finalize_expr(receiver, icx, residual);
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
        | hir::ExprKind::Break
        | hir::ExprKind::Continue
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
        if let hir::ExprKind::Call {
            type_args, args, ..
        } = &expr.kind
        {
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

    #[test]
    fn record_literal_and_field_access_ok() {
        let r = check_src(
            "record Point { x: Int, y: Int }\n\
             fn main() {\n\
                 let p = Point { x: 3, y: 4 }\n\
                 println(\"${p.x} ${p.y}\")\n\
             }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    }

    #[test]
    fn record_field_type_mismatch_reports_e0010() {
        let r = check_src(
            "record Point { x: Int, y: Int }\n\
             fn main() { let p = Point { x: 3, y: \"no\" } }",
        );
        assert!(error_codes(&r).contains(&"E0010"), "{:?}", r.diagnostics);
    }

    #[test]
    fn missing_field_reports_e0014() {
        let r = check_src(
            "record Point { x: Int, y: Int }\n\
             fn main() { let p = Point { x: 3 } }",
        );
        assert!(error_codes(&r).contains(&"E0014"), "{:?}", r.diagnostics);
    }

    #[test]
    fn unknown_field_access_reports_e0014() {
        let r = check_src(
            "record Point { x: Int, y: Int }\n\
             fn main() { let p = Point { x: 3, y: 4 }\n let z = p.z }",
        );
        assert!(error_codes(&r).contains(&"E0014"), "{:?}", r.diagnostics);
    }

    #[test]
    fn generic_record_instantiates() {
        let r = check_src(
            "record Pair<A, B> { first: A, second: B }\n\
             fn main() {\n\
                 let p = Pair { first: 1, second: \"two\" }\n\
                 println(\"${p.first} ${p.second}\")\n\
             }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    }

    #[test]
    fn record_spread_base_ok() {
        let r = check_src(
            "record Point { x: Int, y: Int }\n\
             fn main() {\n\
                 let p = Point { x: 1, y: 2 }\n\
                 let q = Point { x: 10, ..p }\n\
                 println(\"${q.x} ${q.y}\")\n\
             }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    }

    #[test]
    fn inherent_method_ok() {
        let r = check_src(
            "record Point { x: Int, y: Int }\n\
             impl Point { fn sum(self) -> Int { self.x + self.y } }\n\
             fn main() { let p = Point { x: 1, y: 2 }\n println(\"${p.sum()}\") }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    }

    #[test]
    fn trait_impl_and_default_method_ok() {
        let r = check_src(
            "record P { v: Int }\n\
             trait Show { fn name(self) -> String\n fn shout(self) -> String { self.name() } }\n\
             impl Show for P { fn name(self) -> String { \"p\" } }\n\
             fn main() { let p = P { v: 1 }\n println(p.shout()) }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    }

    #[test]
    fn generic_bound_ok() {
        let r = check_src(
            "record P { v: Int }\n\
             trait Show { fn name(self) -> String }\n\
             impl Show for P { fn name(self) -> String { \"p\" } }\n\
             fn label<T: Show>(x: T) -> String { x.name() }\n\
             fn main() { println(label(P { v: 1 })) }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    }

    #[test]
    fn no_method_reports_e0014() {
        let r = check_src(
            "record P { v: Int }\n\
             fn main() { let p = P { v: 1 }\n let s = p.missing() }",
        );
        assert!(error_codes(&r).contains(&"E0014"), "{:?}", r.diagnostics);
    }

    #[test]
    fn impl_missing_required_method_reports_e0070() {
        let r = check_src(
            "record P { v: Int }\n\
             trait Show { fn name(self) -> String }\n\
             impl Show for P { }\n\
             fn main() { }",
        );
        assert!(error_codes(&r).contains(&"E0070"), "{:?}", r.diagnostics);
    }

    #[test]
    fn method_not_in_trait_reports_e0071() {
        let r = check_src(
            "record P { v: Int }\n\
             trait Show { fn name(self) -> String }\n\
             impl Show for P { fn name(self) -> String { \"p\" }\n fn extra(self) -> Int { 0 } }\n\
             fn main() { }",
        );
        assert!(error_codes(&r).contains(&"E0071"), "{:?}", r.diagnostics);
    }

    #[test]
    fn impl_wrong_param_type_reports_e0072() {
        let r = check_src(
            "record P { v: Int }\n\
             trait T { fn m(self, x: Int) -> String }\n\
             impl T for P { fn m(self, x: String) -> String { x } }\n\
             fn main() { }",
        );
        assert!(error_codes(&r).contains(&"E0072"), "{:?}", r.diagnostics);
    }

    #[test]
    fn impl_wrong_return_type_reports_e0072() {
        let r = check_src(
            "record P { v: Int }\n\
             trait T { fn m(self) -> Int }\n\
             impl T for P { fn m(self) -> String { \"x\" } }\n\
             fn main() { }",
        );
        assert!(error_codes(&r).contains(&"E0072"), "{:?}", r.diagnostics);
    }

    #[test]
    fn impl_wrong_arity_reports_e0072() {
        let r = check_src(
            "record P { v: Int }\n\
             trait T { fn m(self, x: Int) -> Int }\n\
             impl T for P { fn m(self) -> Int { 0 } }\n\
             fn main() { }",
        );
        assert!(error_codes(&r).contains(&"E0072"), "{:?}", r.diagnostics);
    }

    #[test]
    fn for_loop_over_range_ok() {
        let r = check_src(
            "fn main() {\n\
                 let mut s = 0\n\
                 for i in 0..10 { s = s + i }\n\
                 for j in 1..=3 { s = s + j }\n\
                 println(\"${s}\")\n\
             }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    }

    #[test]
    fn closure_no_capture_ok() {
        let r = check_src("fn main() { let inc = |n| n + 1\n println(\"${inc(41)}\") }");
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    }

    #[test]
    fn closure_capturing_local_ok() {
        let r = check_src(
            "fn main() {\n\
                 let base = 10\n\
                 let f = |n| n + base\n\
                 println(\"${f(5)}\")\n\
             }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
        // A closure and a wrapper are lifted; at least one extra function.
        assert!(r.module.functions.len() >= 2);
    }

    #[test]
    fn closure_to_higher_order_ok() {
        let r = check_src(
            "fn apply(f: fn(Int) -> Int, x: Int) -> Int { f(x) }\n\
             fn main() { let k = 3\n println(\"${apply(|n| n * k, 4)}\") }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    }

    #[test]
    fn bare_fn_as_value_ok() {
        let r = check_src(
            "fn dbl(n: Int) -> Int { n * 2 }\n\
             fn apply(f: fn(Int) -> Int, x: Int) -> Int { f(x) }\n\
             fn main() { println(\"${apply(dbl, 5)}\") }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    }

    #[test]
    fn closure_assigns_captured_variable_ok() {
        // Regression: an assignment-only capture must still be captured
        // (previously miscompiled / ICE'd because collect_captures ignored
        // the Assign target).
        let r = check_src(
            "fn main() {\n\
                 let mut acc = 100\n\
                 let keep = 9\n\
                 let f = |n: Int| { acc = n\n keep + n }\n\
                 println(\"${f(5)}\")\n\
             }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    }

    #[test]
    fn closure_calls_captured_fn_value_ok() {
        // Regression: a captured fn-value used only as a call target must be
        // captured and its Callee::Local remapped.
        let r = check_src(
            "fn inc(x: Int) -> Int { x + 1 }\n\
             fn dbl(x: Int) -> Int { x * 2 }\n\
             fn main() {\n\
                 let f = inc\n let g = dbl\n\
                 let compose = |x: Int| f(g(x))\n\
                 println(\"${compose(5)}\")\n\
             }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    }

    #[test]
    fn nested_closure_calling_local_ok() {
        // Regression: a nested closure bound to a local and then called
        // (Callee::Local) must be remapped when the outer closure is lifted.
        let r = check_src(
            "fn main() {\n\
                 let z = 1\n\
                 let outer = |a: Int| { let inner = |b: Int| a + b\n inner(100) }\n\
                 println(\"${outer(7)}\")\n\
             }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    }

    #[test]
    fn assigning_for_loop_variable_reports_e0060() {
        let r = check_src("fn main() { for i in 0..5 { i = i + 1 } }");
        assert!(error_codes(&r).contains(&"E0060"), "{:?}", r.diagnostics);
    }

    #[test]
    fn for_loop_does_not_shadow_user_end() {
        // Hidden counter locals are unscoped, so a user `__end` is unaffected.
        let r = check_src(
            "fn main() {\n\
                 let __end = 100\n\
                 for i in 0..3 { println(\"${__end}\") }\n\
             }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    }

    #[test]
    fn break_and_continue_in_loops_ok() {
        let r = check_src(
            "fn main() {\n\
                 let mut s = 0\n\
                 while true { if s > 3 { break }\n s = s + 1 }\n\
                 for i in 0..10 { if i % 2 == 1 { continue }\n s = s + i }\n\
                 println(\"${s}\")\n\
             }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    }

    #[test]
    fn arrays_literal_index_len_ok() {
        let r = check_src(
            "fn main() {\n\
                 let xs = [10, 20, 30]\n\
                 let mut ys = [1, 2]\n\
                 ys[0] = xs[1]\n\
                 println(\"${xs.len()} ${ys[0]}\")\n\
             }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    }

    #[test]
    fn array_param_and_return_ok() {
        let r = check_src(
            "fn first(xs: [Int]) -> Int { xs[0] }\n\
             fn make() -> [Int] { [7, 8, 9] }\n\
             fn main() { println(\"${first(make())}\") }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    }

    #[test]
    fn heterogeneous_array_reports_e0010() {
        let r = check_src("fn main() { let xs = [1, \"two\"] }");
        assert!(error_codes(&r).contains(&"E0010"), "{:?}", r.diagnostics);
    }

    #[test]
    fn non_int_index_reports_e0010() {
        let r = check_src("fn main() { let xs = [1, 2]\n let y = xs[\"a\"] }");
        assert!(error_codes(&r).contains(&"E0010"), "{:?}", r.diagnostics);
    }

    #[test]
    fn indexing_non_array_reports_e0014() {
        let r = check_src("fn main() { let x = 5\n let y = x[0] }");
        assert!(error_codes(&r).contains(&"E0014"), "{:?}", r.diagnostics);
    }

    #[test]
    fn index_set_immutable_reports_e0060() {
        let r = check_src("fn main() { let xs = [1, 2]\n xs[0] = 9 }");
        assert!(error_codes(&r).contains(&"E0060"), "{:?}", r.diagnostics);
    }

    #[test]
    fn index_set_through_immutable_field_reports_e0060() {
        // Regression: mutating an element reached through a field of an
        // immutable binding must still be rejected — the base is `Field`, not
        // a bare `Path`, so the single-segment check used to miss it.
        let r = check_src(
            "record Box { data: [Int] }\n\
             fn main() { let b = Box { data: [1, 2, 3] }\n b.data[0] = 99 }",
        );
        assert!(error_codes(&r).contains(&"E0060"), "{:?}", r.diagnostics);
    }

    #[test]
    fn index_set_through_immutable_nested_array_reports_e0060() {
        // Regression: `grid[0][1] = v` roots at an immutable local through a
        // nested `Index`, which the single-segment check used to bypass.
        let r = check_src("fn main() { let grid = [[1, 2], [3, 4]]\n grid[0][1] = 99 }");
        assert!(error_codes(&r).contains(&"E0060"), "{:?}", r.diagnostics);
    }

    #[test]
    fn index_set_on_call_result_reports_e0060() {
        // A temporary (call result) has no assignable root — mutating an
        // element of it is meaningless and must be rejected.
        let r = check_src(
            "fn make() -> [Int] { [1, 2, 3] }\n\
             fn main() { make()[0] = 99 }",
        );
        assert!(error_codes(&r).contains(&"E0060"), "{:?}", r.diagnostics);
    }

    #[test]
    fn index_set_through_mutable_field_ok() {
        // The mirror of the immutable-field case: a `mut` base makes the
        // reachable array storage assignable.
        let r = check_src(
            "record Box { data: [Int] }\n\
             fn main() { let mut b = Box { data: [1, 2, 3] }\n b.data[0] = 99 }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    }

    #[test]
    fn index_set_nested_mutable_ok() {
        let r = check_src("fn main() { let mut grid = [[1, 2], [3, 4]]\n grid[0][1] = 99 }");
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    }

    #[test]
    fn constants_ok() {
        let r = check_src(
            "const MAX: Int = 100\n\
             const DOUBLE: Int = MAX * 2\n\
             const NAME: String = \"nova\"\n\
             fn main() { println(\"${MAX} ${DOUBLE} ${NAME}\") }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    }

    #[test]
    fn constant_type_mismatch_reports_e0010() {
        let r = check_src("const X: Int = \"not an int\"\nfn main() { println(\"${X}\") }");
        assert!(error_codes(&r).contains(&"E0010"), "{:?}", r.diagnostics);
    }

    #[test]
    fn direct_constant_cycle_reports_e0081() {
        let r = check_src("const A: Int = A + 1\nfn main() { println(\"${A}\") }");
        assert!(error_codes(&r).contains(&"E0081"), "{:?}", r.diagnostics);
    }

    #[test]
    fn indirect_constant_cycle_reports_e0081() {
        let r = check_src("const A: Int = B\nconst B: Int = A\nfn main() { }");
        assert!(error_codes(&r).contains(&"E0081"), "{:?}", r.diagnostics);
    }

    #[test]
    fn function_typed_constant_called_directly_ok() {
        // Regression: a fn-typed const must be callable with call syntax
        // `CONST(args)`, not only via a local (was a spurious E0900).
        let r = check_src(
            "const DOUBLE: fn(Int) -> Int = |n| n * 2\n\
             fn triple(n: Int) -> Int { n * 3 }\n\
             const TRIPLE: fn(Int) -> Int = triple\n\
             fn main() { println(\"${DOUBLE(3)} ${TRIPLE(4)}\") }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    }

    #[test]
    fn non_cyclic_constant_chain_ok() {
        let r = check_src(
            "const A: Int = 1\nconst B: Int = A + 1\nconst C: Int = B + 1\n\
             fn main() { println(\"${C}\") }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    }

    #[test]
    fn if_with_diverging_then_takes_else_type() {
        // Regression: `if c { return } else { v }` must type as `v`, not
        // `Never` — otherwise a diverging branch in a condition ICE'd codegen.
        let r = check_src(
            "fn f() -> Int {\n\
                 let x = if true { return 0 } else { 5 }\n\
                 x + 1\n\
             }\n\
             fn main() { println(\"${f()}\") }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    }

    #[test]
    fn break_in_while_condition_ok() {
        // Regression: break in a while's own condition is accepted and does
        // not crash later stages.
        let r = check_src(
            "fn main() {\n\
                 let mut n = 0\n\
                 while (if n > 5 { break } else { true }) { n = n + 1 }\n\
                 println(\"${n}\")\n\
             }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    }

    #[test]
    fn return_in_while_condition_ok() {
        // Regression: a `Never`-typed while condition (return) must not ICE.
        let r = check_src(
            "fn f() -> Int { while (if true { return 1 } else { false }) { }\n 0 }\n\
             fn main() { println(\"${f()}\") }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    }

    #[test]
    fn break_outside_loop_reports_e0080() {
        let r = check_src("fn main() { break }");
        assert!(error_codes(&r).contains(&"E0080"), "{:?}", r.diagnostics);
    }

    #[test]
    fn continue_outside_loop_reports_e0080() {
        let r = check_src("fn main() { continue }");
        assert!(error_codes(&r).contains(&"E0080"), "{:?}", r.diagnostics);
    }

    #[test]
    fn break_in_closure_does_not_see_outer_loop() {
        // A `break` inside a closure body cannot target the enclosing loop.
        let r = check_src(
            "fn apply(f: fn(Int) -> Int, x: Int) -> Int { f(x) }\n\
             fn main() { while true { let c = |n: Int| { if n > 0 { break }\n n }\n let _ = apply(c, 1) } }",
        );
        assert!(error_codes(&r).contains(&"E0080"), "{:?}", r.diagnostics);
    }

    #[test]
    fn for_over_non_range_reports_e0900() {
        let r = check_src("fn main() { for x in 5 { } }");
        assert!(error_codes(&r).contains(&"E0900"), "{:?}", r.diagnostics);
    }

    #[test]
    fn range_outside_for_reports_e0900() {
        let r = check_src("fn main() { let r = 0..5 }");
        assert!(error_codes(&r).contains(&"E0900"), "{:?}", r.diagnostics);
    }

    #[test]
    fn conforming_impl_with_args_ok() {
        let r = check_src(
            "record P { v: Int }\n\
             trait T { fn add(self, x: Int) -> Int }\n\
             impl T for P { fn add(self, x: Int) -> Int { self.v + x } }\n\
             fn main() { let p = P { v: 1 }\n println(\"${p.add(2)}\") }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    }

    // === Generic impls ===

    #[test]
    fn generic_inherent_impl_ok() {
        // A method on `impl<T> Box<T>` returning `T`, used at two element
        // types, type-checks.
        let r = check_src(
            "record Box<T> { value: T }\n\
             impl<T> Box<T> { fn get(self) -> T { self.value } }\n\
             fn main() {\n\
                 let a = Box { value: 1 }\n\
                 let b = Box { value: \"s\" }\n\
                 println(\"${a.get()} ${b.get()}\")\n\
             }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    }

    #[test]
    fn generic_inherent_impl_return_mismatch_reports_e0010() {
        // `get` claims to return `T` but returns an `Int`, so calling it on a
        // `Box<String>` and using the result as a String must fail.
        let r = check_src(
            "record Box<T> { value: T }\n\
             impl<T> Box<T> { fn get(self) -> T { 0 } }\n\
             fn main() { let b = Box { value: \"s\" }\n let x: String = b.get() }",
        );
        assert!(error_codes(&r).contains(&"E0010"), "{:?}", r.diagnostics);
    }

    #[test]
    fn generic_trait_impl_ok() {
        let r = check_src(
            "record Box<T> { value: T }\n\
             trait Tag { fn tag(self) -> String }\n\
             impl<T> Tag for Box<T> { fn tag(self) -> String { \"b\" } }\n\
             fn main() { let b = Box { value: 1 }\n println(b.tag()) }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    }

    #[test]
    fn conditional_generic_impl_ok() {
        // `Wrap<T>: Display` requires `T: Display`; `Int: Display` holds.
        let r = check_src(
            "trait Display { fn fmt(self) -> String }\n\
             record Wrap<T> { inner: T }\n\
             impl Display for Int { fn fmt(self) -> String { \"i\" } }\n\
             impl<T: Display> Display for Wrap<T> {\n\
                 fn fmt(self) -> String { self.inner.fmt() }\n\
             }\n\
             fn main() { let w = Wrap { inner: 1 }\n println(w.fmt()) }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    }

    // Note: an *unsatisfied* conditional-impl bound (`Wrap<Bool>: Display`
    // when `Bool` is not `Display`) is reported at monomorphization (E0013),
    // not by `check` alone — see the `nova-mir` lowering tests. All trait
    // bounds in Nova are verified during monomorphization.

    #[test]
    fn generic_impl_body_uses_undeclared_capability_reports_e0014() {
        // Without a `T: Display` bound the impl body cannot call `fmt` on `T`.
        let r = check_src(
            "trait Display { fn fmt(self) -> String }\n\
             record Wrap<T> { inner: T }\n\
             impl<T> Display for Wrap<T> {\n\
                 fn fmt(self) -> String { self.inner.fmt() }\n\
             }\n\
             fn main() { let w = Wrap { inner: 1 }\n println(w.fmt()) }",
        );
        assert!(error_codes(&r).contains(&"E0014"), "{:?}", r.diagnostics);
    }

    #[test]
    fn generic_impl_wrong_self_param_reports_e0072() {
        // The trait says the method returns `Self` (`Box<T>`), but the impl
        // declares it returns the bare parameter `T`.
        let r = check_src(
            "record Box<T> { value: T }\n\
             trait Dup { fn dup(self) -> Self }\n\
             impl<T> Dup for Box<T> { fn dup(self) -> T { self.value } }\n\
             fn main() { let b = Box { value: 1 }\n let c = b.dup() }",
        );
        assert!(error_codes(&r).contains(&"E0072"), "{:?}", r.diagnostics);
    }

    #[test]
    fn repeated_param_inherent_impl_matches_only_uniform_receiver() {
        // `impl<T> Pair<T, T>` must apply to `Pair<Int, Int>` but not to
        // `Pair<Int, String>` — sharing the `Pair` head is not enough.
        let ok = check_src(
            "record Pair<A, B> { first: A, second: B }\n\
             impl<T> Pair<T, T> { fn both(self) -> T { self.first } }\n\
             fn main() { let p = Pair { first: 1, second: 2 }\n println(\"${p.both()}\") }",
        );
        assert!(ok.diagnostics.is_empty(), "{:?}", ok.diagnostics);

        let bad = check_src(
            "record Pair<A, B> { first: A, second: B }\n\
             impl<T> Pair<T, T> { fn both(self) -> T { self.first } }\n\
             fn main() { let p = Pair { first: 1, second: \"x\" }\n println(\"${p.both()}\") }",
        );
        assert!(
            error_codes(&bad).contains(&"E0014"),
            "{:?}",
            bad.diagnostics
        );
    }

    #[test]
    fn where_clause_on_impl_is_unsupported() {
        let r = check_src(
            "record Box<T> { value: T }\n\
             trait Tag { fn tag(self) -> String }\n\
             impl<T> Tag for Box<T> where T: Tag { fn tag(self) -> String { \"b\" } }\n\
             fn main() { }",
        );
        assert!(error_codes(&r).contains(&"E0900"), "{:?}", r.diagnostics);
    }

    #[test]
    fn unused_impl_param_reports_e0073() {
        // `U` is declared but appears nowhere in the self type `Box<T>`.
        let r = check_src(
            "record Box<T> { value: T }\n\
             impl<T, U> Box<T> { fn get(self) -> T { self.value } }\n\
             fn main() { let b = Box { value: 1 }\n let x = b.get() }",
        );
        assert!(error_codes(&r).contains(&"E0073"), "{:?}", r.diagnostics);
    }

    #[test]
    fn overlapping_trait_impls_report_e0074() {
        // A generic impl and a specific impl of the same trait for the same
        // head both cover `Box<Int>` — a coherence conflict (no specialization).
        let r = check_src(
            "record Box<T> { value: T }\n\
             trait Kind { fn kind(self) -> String }\n\
             impl<T> Kind for Box<T> { fn kind(self) -> String { \"g\" } }\n\
             impl Kind for Box<Int> { fn kind(self) -> String { \"i\" } }\n\
             fn main() { }",
        );
        assert!(error_codes(&r).contains(&"E0074"), "{:?}", r.diagnostics);
    }

    #[test]
    fn overlapping_inherent_impls_sharing_method_report_e0074() {
        let r = check_src(
            "record Box<T> { value: T }\n\
             impl<T> Box<T> { fn tag(self) -> String { \"g\" } }\n\
             impl Box<Int> { fn tag(self) -> String { \"i\" } }\n\
             fn main() { }",
        );
        assert!(error_codes(&r).contains(&"E0074"), "{:?}", r.diagnostics);
    }

    #[test]
    fn non_overlapping_concrete_trait_impls_ok() {
        // Two concrete impls of one trait for the same head that share no
        // ground instance are allowed, and the call resolves to the fitting
        // one regardless of declaration order.
        let r = check_src(
            "record Pair<A, B> { first: A, second: B }\n\
             trait Foo { fn foo(self) -> String }\n\
             impl Foo for Pair<Int, Bool> { fn foo(self) -> String { \"b\" } }\n\
             impl Foo for Pair<Int, String> { fn foo(self) -> String { \"s\" } }\n\
             fn main() { let p = Pair { first: 1, second: \"x\" }\n println(p.foo()) }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    }

    #[test]
    fn non_overlapping_inherent_impls_ok() {
        // Distinct concrete inherent impls on the same head do not conflict.
        let r = check_src(
            "record Box<T> { value: T }\n\
             impl Box<Int> { fn a(self) -> Int { self.value } }\n\
             impl Box<Bool> { fn b(self) -> Bool { self.value } }\n\
             fn main() {\n\
                 let bi = Box { value: 1 }\n\
                 let bb = Box { value: true }\n\
                 let x = bi.a()\n\
                 let y = bb.b()\n\
             }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    }

    #[test]
    fn non_applicable_inherent_method_does_not_shadow_trait_method() {
        // A restricted inherent `impl<T> Pair<T, T>` must not block the
        // applicable trait method for a non-uniform `Pair<Int, String>`;
        // resolution falls through to the trait impl instead of rejecting.
        let r = check_src(
            "record Pair<A, B> { first: A, second: B }\n\
             trait Describe { fn describe(self) -> String }\n\
             impl<T> Pair<T, T> { fn describe(self) -> String { \"same\" } }\n\
             impl<A, B> Describe for Pair<A, B> { fn describe(self) -> String { \"pair\" } }\n\
             fn main() {\n\
                 let p = Pair { first: 1, second: \"x\" }\n\
                 println(p.describe())\n\
             }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    }

    #[test]
    fn applicable_inherent_method_still_takes_priority() {
        // When the inherent impl *does* fit the receiver, it wins over the
        // trait method (unchanged precedence).
        let r = check_src(
            "record Pair<A, B> { first: A, second: B }\n\
             trait Describe { fn describe(self) -> String }\n\
             impl<T> Pair<T, T> { fn describe(self) -> String { \"same\" } }\n\
             impl<A, B> Describe for Pair<A, B> { fn describe(self) -> String { \"pair\" } }\n\
             fn main() {\n\
                 let q = Pair { first: 1, second: 2 }\n\
                 println(q.describe())\n\
             }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    }

    #[test]
    fn non_applicable_trait_impl_is_rejected_on_direct_call() {
        // A trait impl on `Pair<T, T>` does not apply to `Pair<Int, String>`;
        // a direct call has no other candidate, so it is rejected (E0014).
        let r = check_src(
            "record Pair<A, B> { first: A, second: B }\n\
             trait Same { fn same(self) -> Int }\n\
             impl<T> Same for Pair<T, T> { fn same(self) -> Int { 1 } }\n\
             fn main() { let p = Pair { first: 1, second: \"x\" }\n let n = p.same() }",
        );
        assert!(error_codes(&r).contains(&"E0014"), "{:?}", r.diagnostics);
    }

    #[test]
    fn non_applicable_inherent_fmt_does_not_block_display_bridge() {
        // A non-applicable inherent `fmt` must not block string interpolation
        // from using the applicable `Display` trait impl.
        let r = check_src(
            "record Pair<A, B> { first: A, second: B }\n\
             trait Display { fn fmt(self) -> String }\n\
             impl<T> Pair<T, T> { fn fmt(self) -> String { \"same\" } }\n\
             impl<A, B> Display for Pair<A, B> { fn fmt(self) -> String { \"pair\" } }\n\
             fn main() { let p = Pair { first: 1, second: \"x\" }\n println(\"${p}\") }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    }
}
