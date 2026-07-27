//! AST → typed HIR checking: signature collection, body inference,
//! desugaring, and Maranget-based exhaustiveness/reachability analysis.

use nova_ast as ast;
use nova_ast::item::{ExternItem, TraitItem, TypeDef};
use nova_diagnostics::{Diagnostic, Span, Spanned};
use nova_hir as hir;
use nova_hir::{LocalId, Ty, TyHead};
use nova_resolver::{Builtin, DefId, DefKind, Definitions, MethodOwner, ModuleId, Res};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::infer::InferCtx;
use crate::usefulness;
use crate::{display_ty, CheckResult};

/// A collected function (or method) signature.
#[derive(Debug, Clone)]
struct FnSig {
    generics: u32,
    /// Trait bounds per generic parameter.
    bounds: Vec<Vec<DefId>>,
    /// Parameter types. For a method that declares a `self` receiver,
    /// `params[0]` is that receiver. An associated function (an impl method with
    /// no `self`, tracked in `Checker::selfless`) holds only its declared
    /// parameters, so `params` stays positionally aligned with the AST's.
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

/// How a trait call names its `Self` type — the *only* difference between a
/// method call (`x.cmp(y)`) and an associated-function call (`Int::default()`).
///
/// Bundling the receiver together with the `Self` type it determines is what
/// lets [`Checker::emit_trait_call`] be the single emitter for both: there is
/// no way to pass a receiver without `Self` being derived from it, no way to
/// pass both a receiver and an unrelated `Self`, and no way to pass neither.
/// Two sibling emitters instead had to keep the flat `Param` substitution
/// layout (`[Self] ++ method generics`) in step by hand, and a divergence there
/// silently dispatches to a wrongly specialized function.
enum TraitCallSelf {
    /// `receiver.name(args)`: `Self` is the receiver's resolved type and the
    /// receiver becomes the callee's `self` argument. Valid only for a method
    /// the trait declares *with* a `self` receiver ([`hir::TraitMethod::has_self`]).
    Receiver(hir::Expr),
    /// `Type::name(args)` or `T::name(args)`: `Self` comes from the path
    /// qualifier and there is no receiver. Valid only for a receiver-less
    /// method (a trait associated function).
    Qualifier(Ty),
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
        cur_module: ModuleId(0),
        sigs: FxHashMap::default(),
        method_locs: FxHashMap::default(),
        selfless: FxHashSet::default(),
        mut_self: FxHashSet::default(),
        sums: Vec::new(),
        records: Vec::new(),
        supertraits: FxHashMap::default(),
        traits: Vec::new(),
        impls: Vec::new(),
        extra_functions: Vec::new(),
        next_closure_def: defs.defs().len() as u32,
        type_arity: FxHashMap::default(),
        externs: Vec::new(),
        diagnostics: Vec::new(),
    };
    checker.collect_type_arities();
    checker.collect_records();
    checker.collect_sums();
    checker.collect_supertraits();
    checker.collect_traits();
    checker.collect_impls();
    checker.collect_signatures();
    checker.collect_externs();

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
            externs: checker.externs,
        },
        diagnostics: checker.diagnostics,
    }
}

struct Checker<'a> {
    file: &'a ast::File,
    defs: &'a Definitions,
    /// Module owning the item currently being processed; name resolution is
    /// performed relative to it. Set at each per-item entry point.
    cur_module: ModuleId,
    sigs: FxHashMap<DefId, FnSig>,
    /// AST location of each method `DefId`, for the compile pass.
    method_locs: FxHashMap<DefId, MethodLoc>,
    /// Impl methods that declare no `self` receiver (associated functions).
    /// Their `sigs` entry holds only the declared parameters — no prepended
    /// self type — so `check_fn_body`'s params/sig zip stays aligned.
    selfless: FxHashSet<DefId>,
    /// Methods whose `self` receiver is declared `mut`. Calling one requires a
    /// mutable receiver place at the call site, so `mut` keeps the meaning it
    /// already has for `arr[i] = v` and `rec.f = v` (see ADR 0005). Populated
    /// for **inherent** impl methods only — see [`Checker::check_method_call`]
    /// for why the trait-dispatch path is not covered.
    mut_self: FxHashSet<DefId>,
    sums: Vec<hir::SumType>,
    records: Vec<hir::RecordType>,
    /// Direct supertraits of every trait (`trait Ord: Eq` → `Ord ↦ [Eq]`),
    /// resolved by [`Checker::collect_supertraits`] *before* any bound list is
    /// built. Expansion cannot read them off the incrementally-populated
    /// `traits` table: a bound may name a trait declared later in the file, so
    /// some bound lists would be expanded and others not — and
    /// `check_impl_conformance` compares a trait method's bound set against the
    /// impl method's for equality, which a half-expanded table turns into a
    /// bogus `E0072`.
    supertraits: FxHashMap<DefId, Vec<DefId>>,
    traits: Vec<hir::TraitDef>,
    impls: Vec<hir::ImplInfo>,
    /// Lifted closure / fn-wrapper functions, appended to the module.
    extra_functions: Vec<hir::Function>,
    /// Next synthetic `DefId` for a closure/wrapper (starts past all
    /// resolver-assigned defs so it never collides).
    next_closure_def: u32,
    /// Generic-parameter arity of every record/sum type, precomputed so a
    /// type's arity never depends on collection order (a field or variant may
    /// reference a type collected later, including the implicit prelude).
    type_arity: FxHashMap<DefId, u32>,
    /// `extern` (FFI) function declarations, collected into the HIR module.
    externs: Vec<hir::ExternFn>,
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

    /// Report `E0403` for any generic parameter name that repeats within a
    /// single declaration — `fn f<U, U>`, `record R<T, T>`, `impl<T, T> …`, a
    /// trait method `fn m<U, U>`, etc. A silent duplicate leaves the type
    /// checker's name→index map keeping only the last binding, so the earlier
    /// parameter becomes a phantom the program can never name. `owner` names the
    /// declaration kind for the message (e.g. "function", "type", "method").
    fn check_duplicate_generics(&mut self, generics: &[ast::TypeParam], owner: &str) {
        let mut seen: Vec<&str> = Vec::new();
        for g in generics {
            let name = g.name.value.as_str();
            if seen.contains(&name) {
                self.error(
                    "E0403",
                    format!(
                        "the name `{name}` is already used for a generic parameter of this {owner}"
                    ),
                    g.name.span,
                );
            } else {
                seen.push(name);
            }
        }
    }

    /// Reject a trait bound written on a record's or sum type's *own* generic
    /// parameter (`record Keyed<K: Hash, V>`, `type Wrap<T: Hash> = …`).
    ///
    /// Such a bound parses but nothing honours it: neither [`hir::RecordType`]
    /// nor [`hir::SumType`] carries a `bounds` field, and monomorphization
    /// discharges only *function* and *impl* bounds (`nova-mir`'s `mono.rs`
    /// walks a worklist of function instances). Enforcing it would need a notion
    /// of "record instantiation site" that no pass has — a record's type
    /// arguments survive only inside the enclosing expression's `Ty`,
    /// `ExprKind::MakeRecord` does not record them, and MIR erases them to
    /// `Ptr` — so, exactly as for `trait B where Self: A`, the construct is
    /// rejected loudly rather than left reading as meaningful. Put the bound on
    /// an `impl` block instead, which *is* enforced (this is what
    /// `std/collections`' `Map<K, V>` does).
    ///
    /// One diagnostic per bounded parameter, so a second offender is not hidden
    /// behind the first. The bound names are deliberately **not** resolved: an
    /// unknown trait here would stack an `E0001` cascade on top of the real
    /// error. `owner` is the plural noun phrase for the message, e.g.
    /// "record type parameters".
    fn reject_type_param_bounds(&mut self, generics: &[ast::TypeParam], owner: &str) {
        for g in generics {
            if g.bounds.is_empty() {
                continue;
            }
            // Cover `K: Hash + Eq`, not just `K`. Guarded rather than a bare
            // `merge`, which debug-asserts that both spans share a file.
            let mut span = g.name.span;
            for b in &g.bounds {
                if b.span.file == span.file {
                    span = span.merge(b.span);
                }
            }
            self.diagnostics.push(
                Diagnostic::error(
                    "E0900",
                    format!("trait bounds on {owner} are not supported yet"),
                )
                .with_primary_label(
                    span,
                    format!("bound on the type parameter `{}`", g.name.value),
                )
                .with_note(
                    "this bound is not enforced, so it is rejected rather than \
                     silently ignored; write it on an `impl` block instead, \
                     where it is enforced",
                ),
            );
        }
    }

    fn collect_records(&mut self) {
        for (i, def) in self.defs.defs().iter().enumerate() {
            let DefKind::Record { item_index } = &def.kind else {
                continue;
            };
            let ast::Item::Record(decl) = &self.file.items[*item_index].value else {
                continue;
            };
            self.cur_module = self.defs.module_of(*item_index);
            self.check_duplicate_generics(&decl.generics, "type");
            self.reject_type_param_bounds(&decl.generics, "record type parameters");
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

    /// Precompute the generic arity of every record and sum type from the AST,
    /// before any type is converted. A record field or variant payload may
    /// mention a type whose `RecordType`/`SumType` entry has not been built yet
    /// (collection order, or the implicit prelude — `std/core` — registered
    /// last), so arity must not be read from the incrementally-populated tables.
    fn collect_type_arities(&mut self) {
        let mut arities: Vec<(DefId, u32)> = Vec::new();
        for (i, def) in self.defs.defs().iter().enumerate() {
            let n = match &def.kind {
                DefKind::Record { item_index } => match &self.file.items[*item_index].value {
                    ast::Item::Record(r) => r.generics.len() as u32,
                    _ => continue,
                },
                DefKind::Sum { item_index, .. } => match &self.file.items[*item_index].value {
                    ast::Item::Type(t) => t.generics.len() as u32,
                    _ => continue,
                },
                _ => continue,
            };
            arities.push((DefId(i as u32), n));
        }
        for (id, n) in arities {
            self.type_arity.insert(id, n);
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
            self.cur_module = self.defs.module_of(*item_index);
            self.check_duplicate_generics(&decl.generics, "type");
            self.reject_type_param_bounds(&decl.generics, "sum type parameters");
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

    /// Resolve every trait's declared supertraits (`trait B: A`) into
    /// [`Checker::supertraits`]. Runs before [`Checker::collect_traits`] so the
    /// whole supertrait graph is known by the time the first bound list is
    /// expanded, whatever order the traits are declared in.
    ///
    /// Deduplicated per trait, mirroring [`Checker::resolve_bounds`]: a repeated
    /// trait id reads as two providers of the same method and yields a false
    /// `E0015` "ambiguous method call" at the call site.
    fn collect_supertraits(&mut self) {
        // Copy the `&'a File` reference so the item borrow outlives `&mut self`.
        let file: &'a ast::File = self.file;
        let traits: Vec<(DefId, usize)> = self
            .defs
            .defs()
            .iter()
            .enumerate()
            .filter_map(|(i, d)| match d.kind {
                DefKind::Trait { item_index } => Some((DefId(i as u32), item_index)),
                _ => None,
            })
            .collect();
        for (trait_id, item_index) in traits {
            let Some(item) = file.items.get(item_index) else {
                continue;
            };
            let ast::Item::Trait(decl) = &item.value else {
                continue;
            };
            self.cur_module = self.defs.module_of(item_index);
            let mut ids: Vec<DefId> = Vec::new();
            for path in &decl.supertraits {
                let name = path
                    .value
                    .segments
                    .last()
                    .map(|s| s.value.as_str())
                    .unwrap_or("");
                match self.defs.resolve_trait(self.cur_module, name) {
                    Some(id) => {
                        if !ids.contains(&id) {
                            ids.push(id);
                        }
                    }
                    None => {
                        self.error("E0001", format!("cannot find trait `{name}`"), path.span);
                    }
                }
            }
            self.supertraits.insert(trait_id, ids);
        }
    }

    /// Expand a bound list with the transitive supertraits of each trait, so a
    /// bound `T: B` also provides `B`'s supertrait `A`. Deduplicated, because a
    /// repeated trait id would read as two method providers (a false `E0015`) —
    /// which a diamond (`C: A + B` with `A: X` and `B: X`) reaches easily.
    ///
    /// The walk appends to `out` and never revisits an id already in it, so it
    /// terminates even on a cyclic `trait A: B` / `trait B: A` declaration. An
    /// infinite loop here would hang the compiler, which is far worse than the
    /// missing diagnostic for the cycle itself.
    fn with_supertraits(&self, bounds: &[DefId]) -> Vec<DefId> {
        let mut out: Vec<DefId> = Vec::with_capacity(bounds.len());
        for &id in bounds {
            if !out.contains(&id) {
                out.push(id);
            }
        }
        // Breadth-first over `out` itself: the declared bounds keep their source
        // order and each trait's supertraits follow the traits already queued.
        let mut i = 0;
        while let Some(&id) = out.get(i) {
            i += 1;
            let Some(supers) = self.supertraits.get(&id) else {
                continue;
            };
            for &s in supers {
                if !out.contains(&s) {
                    out.push(s);
                }
            }
        }
        out
    }

    /// Apply [`Checker::with_supertraits`] to every generic parameter's bound
    /// list, in place. Called once per declaration *after* all of its bound
    /// sources (inline bounds and any `where` clause) have been folded in, so the
    /// expansion sees the complete set. Only the contents of each entry change —
    /// never its index, which `FnSig.bounds`, `ImplInfo.bounds` and
    /// `TraitMethod.bounds` all address positionally.
    fn expand_bounds(&self, bounds: &mut [Vec<DefId>]) {
        for slot in bounds.iter_mut() {
            *slot = self.with_supertraits(slot);
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
            self.cur_module = self.defs.module_of(item_index);
            if !decl.generics.is_empty() {
                self.unsupported(decl.name.span, "generic traits");
            }
            // `trait B where Self: A` is a second spelling of a supertrait
            // requirement, distinct from the `trait B: A` shorthand that
            // `collect_supertraits` resolves. Routing it into the same graph
            // is a feature addition beyond parsing it, so — like `where`
            // clauses on trait methods just below — it is rejected outright
            // rather than silently discarded.
            if !decl.where_clause.is_empty() {
                self.unsupported(decl.name.span, "`where` clauses on trait declarations");
            }
            let self_scope = self_generic_scope();
            let mut methods = Vec::new();
            for (mi, item) in decl.items.iter().enumerate() {
                let (name, generics, where_clause, params, ret, is_default, is_async) = match item {
                    TraitItem::Required(sig) => (
                        &sig.name,
                        &sig.generics,
                        &sig.where_clause,
                        &sig.params,
                        &sig.return_ty,
                        false,
                        sig.is_async,
                    ),
                    TraitItem::Provided(f) => (
                        &f.name,
                        &f.generics,
                        &f.where_clause,
                        &f.params,
                        &f.return_ty,
                        true,
                        f.is_async,
                    ),
                };
                // `async` is unsupported at every method site; check it here so
                // that declaration-only (`Required`) trait methods are covered
                // too — the default-body pass below only visits `Provided` ones.
                if is_async {
                    self.unsupported(name.span, "async methods");
                }
                if !where_clause.is_empty() {
                    self.error(
                        "E0900",
                        "`where` clauses on trait methods are not supported yet",
                        name.span,
                    );
                }
                self.check_duplicate_generics(generics, "method");
                // A generic trait method (`fn map<U>(self, …)`) binds `Self` at
                // Param(0) and its own generic parameters at Param(1..).
                let mut m_scope = self_scope.clone();
                for (j, g) in generics.iter().enumerate() {
                    m_scope.insert(g.name.value.clone(), 1 + j as u32);
                }
                let (m_params, m_ret) = self.method_sig_parts(params, ret, &m_scope);
                let mut m_bounds = self.resolve_bounds(generics);
                // A `where` clause on a trait method is rejected above, so the
                // inline bounds are the complete set.
                self.expand_bounds(&mut m_bounds);
                let default_def = if is_default {
                    default_defs.get(&(item_index, mi)).copied()
                } else {
                    None
                };
                methods.push(hir::TraitMethod {
                    name: name.value.clone(),
                    params: m_params,
                    ret: m_ret,
                    // `any`, not `first`: `method_sig_parts` strips *every*
                    // parameter named `self`, and the parser accepts a misplaced
                    // receiver (`fn m(x: Int, self)`), so the same predicate must
                    // decide both or `params` and `has_self` disagree. Mirrors
                    // `collect_impls`.
                    has_self: params.iter().any(|p| p.name.value == "self"),
                    generics: generics.len() as u32,
                    bounds: m_bounds,
                    default_def,
                });
            }
            let def_id = DefId(i as u32);
            self.traits.push(hir::TraitDef {
                def_id,
                name: def.name.clone(),
                supertraits: self.supertraits.get(&def_id).cloned().unwrap_or_default(),
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
            self.cur_module = self.defs.module_of(item_index);
            let self_scope = self_generic_scope();
            for (mi, item) in decl.items.iter().enumerate() {
                let TraitItem::Provided(f) = item else {
                    continue;
                };
                let Some(def_id) = default_defs.get(&(item_index, mi)).copied() else {
                    continue;
                };
                // `async` and `where` clauses on trait methods are both rejected
                // in the table loop above; don't build a body signature for one.
                if f.is_async {
                    continue;
                }
                if !f.where_clause.is_empty() {
                    continue;
                }
                // `Self` at Param(0), the method's own generics at Param(1..).
                let mut scope = self_scope.clone();
                for (j, g) in f.generics.iter().enumerate() {
                    scope.insert(g.name.value.clone(), 1 + j as u32);
                }
                let (mut params, ret) = self.method_sig_parts(&f.params, &f.return_ty, &scope);
                // Prepend the `self` receiver typed as `Self` (`Param(0)`) — but
                // only for a method that declares one. A default-bodied
                // associated function (`fn zero() -> Self { … }`) has no
                // receiver, and prepending one would desynchronise `sig.params`
                // from the AST parameter list that `check_fn_body` zips against
                // it. Mirrors the same conditional in `collect_impls`.
                if f.params.iter().any(|p| p.name.value == "self") {
                    params.insert(0, Ty::Param(0));
                }
                // One generic for `Self` (bounded by the trait) plus the method's
                // own generics, at the same flat Param indices.
                let mut bounds = vec![vec![trait_id]];
                bounds.extend(self.resolve_bounds(&f.generics));
                // `Self`'s bound is the enclosing trait, so expanding it is what
                // lets a `trait B: A` default body call `self.a()`.
                self.expand_bounds(&mut bounds);
                self.sigs.insert(
                    def_id,
                    FnSig {
                        generics: 1 + f.generics.len() as u32,
                        bounds,
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
            self.cur_module = self.defs.module_of(item_index);
            self.check_duplicate_generics(&block.generics, "impl");
            // The impl's generic parameters (`impl<T> …`) are in scope in the
            // self type and every method signature/body.
            let impl_generics = generic_scope(&block.generics);
            let mut impl_bounds = self.resolve_bounds(&block.generics);
            self.apply_where(&mut impl_bounds, &block.where_clause, &impl_generics);
            self.expand_bounds(&mut impl_bounds);
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
                    match self.defs.resolve_trait(self.cur_module, name) {
                        Some(id) => Some(id),
                        None => {
                            self.error("E0001", format!("cannot find trait `{name}`"), tr.span);
                            continue;
                        }
                    }
                }
                None => None,
            };

            let impl_count = block.generics.len() as u32;
            let mut methods = Vec::new();
            for (mi, f) in block.functions.iter().enumerate() {
                let Some(def_id) = impl_methods.get(&(item_index, mi)).copied() else {
                    continue;
                };
                if f.is_async {
                    self.unsupported(f.name.span, "async methods");
                }
                // The method's generic scope: the impl's parameters (`impl<T> …`)
                // at indices [0, impl_count), then the method's own parameters
                // (`fn map<U>`) at [impl_count, …). A single flat `type_args`
                // vector — impl args recovered from the receiver, method args
                // inferred from the call — then drives substitution, bound
                // checking, and monomorphization uniformly.
                let mut scope = impl_generics.clone();
                let mut bounds = impl_bounds.clone();
                let mut seen: Vec<&str> = Vec::new();
                for (j, g) in f.generics.iter().enumerate() {
                    let gname = g.name.value.as_str();
                    if impl_generics.contains_key(gname) {
                        self.error(
                            "E0403",
                            format!("the name `{gname}` shadows the impl's generic parameter"),
                            g.name.span,
                        );
                    } else if seen.contains(&gname) {
                        self.error(
                            "E0403",
                            format!(
                                "the name `{gname}` is already used for a generic \
                                 parameter of this method"
                            ),
                            g.name.span,
                        );
                    }
                    seen.push(gname);
                    scope.insert(g.name.value.clone(), impl_count + j as u32);
                }
                bounds.extend(self.resolve_bounds(&f.generics));
                let method_generic_count = f.generics.len() as u32;
                // A method's `where` clause may constrain the impl's or its own
                // type parameters; fold it into the combined bounds.
                self.apply_where(&mut bounds, &f.where_clause, &scope);
                // The leading `impl_count` entries are already expanded (they are
                // a clone of `impl_bounds`); re-expanding them is a no-op.
                self.expand_bounds(&mut bounds);
                // Non-self params + ret in terms of the self type, resolving the
                // impl's and this method's generic parameters.
                let (mut params, ret) = self.method_sig_parts(&f.params, &f.return_ty, &scope);
                // `self` is stripped by `method_sig_parts` and re-inserted here as
                // the receiver — but only for methods that actually declare one.
                // For an associated function (`fn new() -> P`) inserting it would
                // shift every parameter by one against `f.params`, which
                // `check_fn_body` zips positionally. The predicate mirrors
                // `method_sig_parts`, which strips *every* param named `self`, so
                // `sig.params.len()` equals `f.params.len()` for a method with
                // zero or exactly one `self` parameter — the only shapes this
                // insert re-aligns. Nothing enforces that shape: a duplicated
                // receiver (`impl P { fn g(self, self) -> Int { 1 } }`) still
                // checks `ok`, because `method_sig_parts` strips both `self`s but
                // only one is re-inserted here, leaving `sig.params` one short of
                // `f.params` and desynchronising the positional zip again —
                // calling `p.g()` ICEs in codegen. Pre-existing, not fixed here.
                let has_self = f.params.iter().any(|p| p.name.value == "self");
                if has_self {
                    params.insert(0, self_ty.clone());
                } else {
                    self.selfless.insert(def_id);
                }
                // `mut self` makes the method a mutator, which its callers must
                // opt into with a mutable receiver (ADR 0005). `any`, for the
                // same reason `has_self` uses it: `method_sig_parts` strips a
                // `self` at *any* position, so both predicates must scan the
                // whole list or they can disagree about the same parameter.
                if f.params.iter().any(|p| p.name.value == "self" && p.is_mut) {
                    self.mut_self.insert(def_id);
                }
                self.sigs.insert(
                    def_id,
                    FnSig {
                        generics: impl_count + method_generic_count,
                        bounds,
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
                self.check_impl_conformance(tid, &methods, &self_ty, impl_count, block.ty.span);
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
        self.check_supertrait_impls(&impl_spans);
    }

    /// Every trait impl must be accompanied by an impl of each of the trait's
    /// supertraits for the same self type: `trait B: A` says a `B` *is* an `A`,
    /// so `impl B for R` without `impl A for R` lets the bound `T: B` promise
    /// methods no impl provides — the call resolves in the type checker and then
    /// finds no impl at monomorphization.
    ///
    /// Only *direct* supertraits are required here. That is enough transitively:
    /// the `impl A for R` this pass demands is itself an impl in the table, so it
    /// is visited too and made to supply `A`'s own supertraits. Checking the
    /// transitive closure instead would report the same missing impl once per
    /// subtrait in the chain.
    ///
    /// Runs *after* the whole impl table is built, beside
    /// [`Checker::check_impl_coherence`], rather than from
    /// [`Checker::check_impl_conformance`]: conformance is called from the middle
    /// of `collect_impls`, before the impl being checked has even been pushed, so
    /// it sees only impls from *earlier* items and would reject an `impl B for R`
    /// written above its `impl A for R`. Nova has no declaration-order rule for
    /// impls, and `resolve_method_on` does not impose one either.
    fn check_supertrait_impls(&mut self, spans: &[Span]) {
        let mut errors: Vec<(String, Span)> = Vec::new();
        for (imp, &span) in self.impls.iter().zip(spans) {
            let Some(trait_id) = imp.trait_id else {
                continue;
            };
            let Some(tr) = self.traits.iter().find(|t| t.def_id == trait_id) else {
                continue;
            };
            for &super_id in &tr.supertraits {
                // A generic impl's self type is a pattern, so the supertrait impl
                // must cover the whole family: `match_args` treats the supertrait
                // impl's self type as the pattern and this impl's as the ground
                // term, which an `impl A for W<Int>` fails against
                // `impl<T> B for W<T>` — as it should, since it leaves every other
                // `W<T>` without an `A`.
                let satisfied = self.impls.iter().any(|other| {
                    other.trait_id == Some(super_id) && other.match_args(&imp.self_ty).is_some()
                });
                if satisfied {
                    continue;
                }
                let sname = self
                    .traits
                    .iter()
                    .find(|t| t.def_id == super_id)
                    .map(|t| t.name.as_str())
                    .unwrap_or("?");
                errors.push((
                    format!(
                        "the trait `{}` requires `{sname}`, which `{}` does not implement",
                        tr.name,
                        display_ty(&imp.self_ty, self.defs),
                    ),
                    span,
                ));
            }
        }
        for (msg, span) in errors {
            self.error("E0072", msg, span);
        }
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

    /// Render a set of trait bounds for a diagnostic, e.g. `` `Show + Ord` ``
    /// or `(none)` when empty.
    fn render_bound_set(&self, bounds: &[DefId]) -> String {
        if bounds.is_empty() {
            return "(none)".to_string();
        }
        let names: Vec<String> = bounds
            .iter()
            .map(|d| {
                self.traits
                    .iter()
                    .find(|t| t.def_id == *d)
                    .map(|t| t.name.clone())
                    .unwrap_or_else(|| format!("{d:?}"))
            })
            .collect();
        format!("`{}`", names.join(" + "))
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
        impl_count: u32,
        span: Span,
    ) {
        let Some(tr) = self.traits.iter().find(|t| t.def_id == trait_id).cloned() else {
            return;
        };
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
            // The receiver must agree. Nothing below can catch a disagreement:
            // neither `params` list stores `self`, so an impl that adds or drops
            // the receiver still compares equal parameter-for-parameter and
            // return-type-wise. Yet a call site programs against the trait's
            // signature while codegen dispatches to the impl's function, so the
            // two differ by exactly one leading argument — Cranelift rejects the
            // module ("mismatched argument count") and the compiler ICEs on
            // source that `nova check` accepted.
            let impl_has_self = !self.selfless.contains(def_id);
            if impl_has_self != trait_method.has_self {
                let (want, got) = if trait_method.has_self {
                    ("a `self` receiver", "none")
                } else {
                    ("no `self` receiver", "one")
                };
                self.error(
                    "E0072",
                    format!(
                        "method `{name}` has {got} but trait `{}` declares {want}",
                        tr.name
                    ),
                    span,
                );
                continue;
            }
            // The impl method's own generics = its total minus the impl's, and
            // must match the trait method's generic count.
            let impl_method_generics = impl_sig.generics.saturating_sub(impl_count);
            if impl_method_generics != trait_method.generics {
                self.error(
                    "E0072",
                    format!(
                        "method `{name}` has {impl_method_generics} generic parameter(s) but \
                         trait `{}` declares {}",
                        tr.name, trait_method.generics
                    ),
                    span,
                );
                continue;
            }
            // Each method generic must carry exactly the trait's declared bounds
            // — neither dropped nor added. The impl method's own generics live at
            // `impl_sig.bounds[impl_count + k]`, aligned with the trait method's
            // `bounds[k]`. Without this the trait signature the call site programs
            // against is not the contract the impl honors: an impl that drops a
            // bound accepts calls the trait forbids (unsound), and one that adds a
            // bound rejects trait-valid calls only later, at monomorphization.
            for k in 0..trait_method.generics as usize {
                let want = trait_method.bounds[k].as_slice();
                let got = impl_sig
                    .bounds
                    .get(impl_count as usize + k)
                    .map(Vec::as_slice)
                    .unwrap_or(&[]);
                if !same_bound_set(want, got) {
                    let want_str = self.render_bound_set(want);
                    let got_str = self.render_bound_set(got);
                    self.error(
                        "E0072",
                        format!(
                            "generic parameter {} of method `{name}` has bound(s) {got_str} \
                             but trait `{}` declares {want_str}",
                            k + 1,
                            tr.name,
                        ),
                        span,
                    );
                }
            }
            // Map the trait method's Param space into the impl's: `Self`
            // (Param(0)) -> the impl self type; method generic k (Param(1+k)) ->
            // the impl method's own generic at Param(impl_count + k).
            let mut subst = Vec::with_capacity(1 + trait_method.generics as usize);
            subst.push(self_ty.clone());
            for k in 0..trait_method.generics {
                subst.push(Ty::Param(impl_count + k));
            }
            // `impl_sig.params[0]` is the `self` receiver — skip it and compare
            // the declared parameters (and the return type) against the trait
            // method's, which `method_sig_parts` also stores without `self`. An
            // associated function has no receiver stored, so there is nothing to
            // skip (and `[1..]` would panic on its empty parameter list).
            let impl_params: &[Ty] = if self.selfless.contains(def_id) {
                &impl_sig.params
            } else {
                impl_sig.params.get(1..).unwrap_or_default()
            };
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
            self.cur_module = self.defs.module_of(item_index);
            if f.is_async {
                self.unsupported(f.name.span, "async functions");
            }
            self.check_duplicate_generics(&f.generics, "function");
            let generics = generic_scope(&f.generics);
            let mut bounds = self.resolve_bounds(&f.generics);
            self.apply_where(&mut bounds, &f.where_clause, &generics);
            self.expand_bounds(&mut bounds);
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
            self.cur_module = self.defs.module_of(item_index);
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

    /// Collect and validate `extern` (FFI) function declarations. Each becomes a
    /// zero-generic `FnSig` (so calls type-check through the normal path) plus a
    /// `hir::ExternFn` carrying its raw C symbol for codegen. Only the C ABI and
    /// FFI-safe scalar types are supported; anything else is a diagnostic.
    fn collect_externs(&mut self) {
        let externs: Vec<(DefId, usize, usize)> = self.defs.extern_functions().collect();
        for (def_id, item_index, fn_index) in externs {
            let ast::Item::Extern(block) = &self.file.items[item_index].value else {
                continue;
            };
            let ExternItem::Fn(sig) = &block.items[fn_index];
            self.cur_module = self.defs.module_of(item_index);

            // The symbol is emitted raw (unmangled), so it shares a namespace
            // with the compiler's own symbols. Reserve the `nova_` prefix (the
            // runtime's `nova_rt_*` and the `nova_main` entry) and `main` so an
            // extern can't shadow or alias an internal symbol.
            if sig.name.value == "main" || sig.name.value.starts_with("nova_") {
                self.error(
                    "E0900",
                    format!(
                        "extern symbol `{}` is reserved by the compiler",
                        sig.name.value
                    ),
                    sig.name.span,
                );
                continue;
            }

            // Only the C ABI (explicit `"C"` or omitted) is supported.
            if !matches!(block.abi.as_deref(), None | Some("C")) {
                let abi = block.abi.clone().unwrap_or_default();
                self.error(
                    "E0900",
                    format!("extern ABI `\"{abi}\"` is not supported; only the C ABI is supported"),
                    sig.name.span,
                );
            }
            if sig.is_async {
                self.unsupported(sig.name.span, "async extern functions");
            }
            if !sig.where_clause.is_empty() {
                self.unsupported(sig.name.span, "`where` clauses on extern functions");
            }
            if !sig.generics.is_empty() {
                // Skip type conversion: the generic params aren't in scope, which
                // would cascade a false "cannot find type" on top of this error.
                self.unsupported(sig.name.span, "generic extern functions");
                continue;
            }

            let empty = FxHashMap::default();
            let params: Vec<Ty> = sig
                .params
                .iter()
                .map(|p| {
                    let ty = self.convert_ty(&p.ty, &empty);
                    self.require_ffi_safe(&ty, p.ty.span, false);
                    ty
                })
                .collect();
            let ret = match &sig.return_ty {
                Some(t) => {
                    let ty = self.convert_ty(t, &empty);
                    self.require_ffi_safe(&ty, t.span, true);
                    ty
                }
                None => Ty::Unit,
            };

            self.sigs.insert(
                def_id,
                FnSig {
                    generics: 0,
                    bounds: Vec::new(),
                    params: params.clone(),
                    ret: ret.clone(),
                },
            );
            self.externs.push(hir::ExternFn {
                def_id,
                symbol: sig.name.value.clone(),
                params,
                ret,
                span: sig.name.span,
            });
        }
    }

    /// Reject a non-FFI-safe type in an `extern` signature. The FFI-safe types
    /// are the scalars `Int`, `Float`, `Bool` (and, for a return, unit / `void`).
    fn require_ffi_safe(&mut self, ty: &Ty, span: Span, is_return: bool) {
        let ok = match ty {
            Ty::Int | Ty::Float | Ty::Bool => true,
            Ty::Unit => is_return,
            // `convert_ty` already reported why this type is unusable.
            Ty::Error => true,
            _ => false,
        };
        if !ok {
            self.error(
                "E0900",
                format!(
                    "`{}` is not FFI-safe in an extern signature yet; only Int, Float, and Bool \
                     (and a unit return) are supported",
                    display_ty(ty, self.defs)
                ),
                span,
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
                let mut ids: Vec<DefId> = Vec::new();
                for b in &g.bounds {
                    let name = b
                        .value
                        .segments
                        .last()
                        .map(|s| s.value.as_str())
                        .unwrap_or("");
                    match self.defs.resolve_trait(self.cur_module, name) {
                        // Deduplicate: `T: Show + Show` means the same as
                        // `T: Show`. A repeated trait must not later read as two
                        // distinct method providers (a false E0015 ambiguity).
                        Some(id) => {
                            if !ids.contains(&id) {
                                ids.push(id);
                            }
                        }
                        None => {
                            self.error("E0001", format!("cannot find trait `{name}`"), b.span);
                        }
                    }
                }
                ids
            })
            .collect()
    }

    /// Fold a `where` clause into a per-parameter bound list (as produced by
    /// [`Self::resolve_bounds`]). A `where` bound is just an out-of-line spelling
    /// of an inline `<T: Trait>`: each `T: Trait` must constrain one of the
    /// item's own type parameters (resolved through `scope`), and its traits
    /// accumulate on top of any inline bounds. Constraints on concrete or
    /// compound types (`where Box<T>: Trait`) are not supported yet.
    fn apply_where(
        &mut self,
        bounds: &mut [Vec<DefId>],
        where_clause: &[ast::WhereBound],
        scope: &FxHashMap<String, u32>,
    ) {
        for wb in where_clause {
            let idx = match self.convert_ty(&wb.ty, scope) {
                Ty::Param(i) => i as usize,
                // `convert_ty` already reported an unknown/invalid type.
                Ty::Error => continue,
                _ => {
                    self.error(
                        "E0900",
                        "a `where` clause may only constrain a type parameter",
                        wb.ty.span,
                    );
                    continue;
                }
            };
            let Some(slot) = bounds.get_mut(idx) else {
                continue;
            };
            for b in &wb.bounds {
                let name = b
                    .value
                    .segments
                    .last()
                    .map(|s| s.value.as_str())
                    .unwrap_or("");
                match self.defs.resolve_trait(self.cur_module, name) {
                    // Deduplicate against inline and earlier `where` bounds, so a
                    // trait named twice is not read as two method providers.
                    Some(id) => {
                        if !slot.contains(&id) {
                            slot.push(id);
                        }
                    }
                    None => self.error("E0001", format!("cannot find trait `{name}`"), b.span),
                }
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
                if let Some(def_id) = self.defs.resolve_type(self.cur_module, name) {
                    let is_record = matches!(self.defs.def(def_id).kind, DefKind::Record { .. });
                    // Arity is precomputed (see `collect_type_arities`) so it is
                    // independent of whether this type has been collected yet.
                    let expected = self.type_arity.get(&def_id).copied().unwrap_or(0);
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
        self.cur_module = self.defs.module_of(item_index);
        let generics = generic_scope(&f.generics);
        self.check_fn_body(def_id, f, generics)
    }

    /// Compile an impl or trait-default method body.
    fn check_method(&mut self, def_id: DefId) -> Option<hir::Function> {
        let loc = *self.method_locs.get(&def_id)?;
        self.cur_module = self.defs.module_of(loc.item_index);
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
            // An impl method sees the impl's generic parameters (`impl<T> …`) at
            // [0, impl_count), then its own generic parameters after them. Must
            // mirror `collect_impls`.
            MethodOwner::Impl => match &file.items[loc.item_index].value {
                ast::Item::Impl(block) => {
                    let mut scope = generic_scope(&block.generics);
                    let impl_count = block.generics.len() as u32;
                    for (j, g) in f.generics.iter().enumerate() {
                        scope.insert(g.name.value.clone(), impl_count + j as u32);
                    }
                    scope
                }
                _ => FxHashMap::default(),
            },
            // A trait default body sees `Self` at Param(0), then the method's own
            // generic parameters at Param(1..). Mirrors `collect_traits`.
            MethodOwner::TraitDefault => {
                let mut scope = self_generic_scope();
                for (j, g) in f.generics.iter().enumerate() {
                    scope.insert(g.name.value.clone(), 1 + j as u32);
                }
                scope
            }
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
        self.cur_module = self.defs.module_of(item_index);
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
            ast::Expr::ArrayRepeat { init, len } => self.check_array_repeat(fcx, init, len, span),
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
            if let Some(def_id) = self.defs.resolve_type(self.cur_module, ty_name) {
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
        match self.defs.resolve_value(self.cur_module, name) {
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
            // `target` (the receiver's *AST*) rides along because the mutable-
            // receiver rule needs `place_root`, which walks AST projections; the
            // checked `hir::Expr` has already lost that shape.
            return self.check_method_call(fcx, receiver, target, field, checked, span);
        }

        // Direct-call forms: a path naming a function, variant, or builtin.
        if let ast::Expr::Path(path) = &callee.value {
            if path.segments.len() == 1 {
                let name = path.segments[0].value.as_str();
                if fcx.lookup(name).is_none() {
                    match self.defs.resolve_value(self.cur_module, name) {
                        Some(Res::Def(def_id)) => {
                            if matches!(
                                self.defs.def(def_id).kind,
                                DefKind::Fn { .. } | DefKind::ExternFn { .. }
                            ) {
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
                // `Type::Variant(args)`, `Type::assoc_fn(args)`, or `T::assoc_fn(args)`.
                let ty_name = path.segments[0].value.as_str();
                let name = path.segments[1].value.as_str();
                // `T::zero()` where `T` is a generic parameter: dispatch through
                // its bounds, exactly as a bounded instance method call does.
                // Checked before the type namespace so a generic parameter
                // shadows a same-named type, matching `convert_ty`.
                if let Some(&k) = fcx.generics.get(ty_name) {
                    let matches: Vec<(DefId, u32)> = fcx
                        .param_bounds
                        .get(k as usize)
                        .into_iter()
                        .flatten()
                        .filter_map(|&tid| self.trait_assoc_fn_index(tid, name).map(|i| (tid, i)))
                        .collect();
                    let checked: Vec<hir::Expr> =
                        args.iter().map(|a| self.check_expr(fcx, a)).collect();
                    return match matches.as_slice() {
                        [(tid, idx)] => self.emit_trait_call(
                            fcx,
                            *tid,
                            *idx,
                            TraitCallSelf::Qualifier(Ty::Param(k)),
                            checked,
                            span,
                        ),
                        [] => {
                            // A bound may still declare `name` — just as a
                            // method with a `self` receiver rather than an
                            // associated function. "none of its bounds
                            // declares one" would be false in that case, so
                            // check for it and name the real reason.
                            let declared_as_method = fcx
                                .param_bounds
                                .get(k as usize)
                                .into_iter()
                                .flatten()
                                .any(|&tid| self.trait_method_index(tid, name).is_some());
                            if declared_as_method {
                                self.error(
                                    "E0001",
                                    format!(
                                        "no associated function `{name}` on generic parameter \
                                         `{ty_name}`; its bound declares `{name}` as a method \
                                         with a `self` receiver, so it must be called on a value"
                                    ),
                                    callee.span,
                                );
                            } else {
                                self.error(
                                    "E0001",
                                    format!(
                                        "no associated function `{name}` on generic parameter \
                                         `{ty_name}`; none of its bounds declares one"
                                    ),
                                    callee.span,
                                );
                            }
                            error_expr(span)
                        }
                        _ => {
                            self.error(
                                "E0015",
                                format!(
                                    "ambiguous associated function `{ty_name}::{name}`: more \
                                     than one bound of `{ty_name}` provides it"
                                ),
                                callee.span,
                            );
                            error_expr(span)
                        }
                    };
                }
                // `Type::Variant(args)` keeps its existing meaning. Only a
                // nominal type has variants, so this stays on the `resolve_type`
                // path and is tried before any associated function.
                if let Some(def_id) = self.defs.resolve_type(self.cur_module, ty_name) {
                    if let Some(vi) = self.variant_index(def_id, name) {
                        let checked: Vec<hir::Expr> =
                            args.iter().map(|a| self.check_expr(fcx, a)).collect();
                        return self.make_variant(fcx, def_id, vi, checked, span);
                    }
                }
                // Then an associated function: on an inherent impl first (they
                // take priority over trait methods, as for instance calls), then
                // through a trait impl for the qualifier's type. `Int::zero()`
                // reaches both only via `qualifier_self_ty` — a primitive name is
                // absent from the resolver's type namespace entirely.
                if let Some(self_ty) = self.qualifier_self_ty(fcx, ty_name) {
                    let inherent = self_ty
                        .head()
                        .map(|head| self.find_assoc_fns(head, name))
                        .unwrap_or_default();
                    match inherent.as_slice() {
                        [assoc_id] => {
                            let assoc_id = *assoc_id;
                            let checked: Vec<hir::Expr> =
                                args.iter().map(|a| self.check_expr(fcx, a)).collect();
                            return self.emit_assoc_call(fcx, assoc_id, checked, span);
                        }
                        [] => {}
                        // Two inherent impls sharing the head both declare it.
                        // `check_impl_coherence` lets a *disjoint concrete*
                        // pair (`impl Box<Int>` / `impl Box<Bool>`) through, so
                        // this is the only guard against declaration-order
                        // dispatch here.
                        _ => {
                            self.error(
                                "E0015",
                                format!(
                                    "ambiguous associated function `{ty_name}::{name}`: more \
                                     than one inherent impl of `{ty_name}` provides it"
                                ),
                                callee.span,
                            );
                            return error_expr(span);
                        }
                    }
                    let matches = self.find_trait_assoc_fns(&self_ty, name);
                    match matches.as_slice() {
                        [(tid, idx)] => {
                            let (tid, idx) = (*tid, *idx);
                            let checked: Vec<hir::Expr> =
                                args.iter().map(|a| self.check_expr(fcx, a)).collect();
                            return self.emit_trait_call(
                                fcx,
                                tid,
                                idx,
                                TraitCallSelf::Qualifier(self_ty),
                                checked,
                                span,
                            );
                        }
                        [] => {
                            // A primitive is invisible to `resolve_type`, so the
                            // fall-through below would blame "module-qualified
                            // paths are not supported" for what is really a
                            // missing associated function.
                            if self.defs.resolve_type(self.cur_module, ty_name).is_none() {
                                self.error(
                                    "E0001",
                                    format!("no associated function `{name}` on type `{ty_name}`"),
                                    callee.span,
                                );
                                return error_expr(span);
                            }
                            // A nominal qualifier whose type arguments are the
                            // reason nothing matched: some impl sharing this
                            // type's head does declare `name` through a trait,
                            // but `find_trait_assoc_fns` could not select it
                            // because `self_ty`'s arguments are still unresolved
                            // inference variables (see that function's doc
                            // comment) — `match_args` cannot line a concrete
                            // impl's arguments up with a variable. Name that
                            // reason here rather than let this fall through to
                            // `check_path`, which would report a missing
                            // variant and blame the wrong feature entirely.
                            if let Some(head) = self_ty.head() {
                                let head_declares_it = self
                                    .impls
                                    .iter()
                                    .filter(|i| i.self_head == head)
                                    .filter_map(|i| i.trait_id)
                                    .any(|tid| self.trait_assoc_fn_index(tid, name).is_some());
                                if head_declares_it {
                                    self.error(
                                        "E0011",
                                        format!(
                                            "cannot call `{ty_name}::{name}()`: an impl provides \
                                             it, but `{ty_name}`'s type arguments could not be \
                                             determined from the qualifier alone to select it"
                                        ),
                                        callee.span,
                                    );
                                    return error_expr(span);
                                }
                            }
                            // Otherwise a nominal qualifier keeps falling
                            // through to `check_path`, which reports the
                            // pre-existing "no variant" diagnostic.
                        }
                        _ => {
                            self.error(
                                "E0015",
                                format!(
                                    "ambiguous associated function `{ty_name}::{name}`: more \
                                     than one trait in scope provides it"
                                ),
                                callee.span,
                            );
                            return error_expr(span);
                        }
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
        let (params, ret) = builtin_signature(builtin);
        if args.len() != params.len() {
            self.error(
                "E0016",
                format!(
                    "`{}` takes {} argument{} but {} were supplied",
                    builtin.name(),
                    params.len(),
                    if params.len() == 1 { "" } else { "s" },
                    args.len()
                ),
                span,
            );
            return error_expr(span);
        }
        // The print family's hint is specific to it: passing a non-`String`
        // there is nearly always a missing interpolation, whereas the
        // std-only builtins are called from several sites across std/core and
        // std/strings, with no single wrong-argument pattern common to all of
        // them.
        let hint = match builtin {
            Builtin::Println | Builtin::Print | Builtin::Panic => {
                " (use string interpolation: \"${value}\")"
            }
            Builtin::StrCmp
            | Builtin::StrHash
            | Builtin::CharToInt
            | Builtin::StrLenChars
            | Builtin::StrChars
            | Builtin::StrFromChars
            | Builtin::StrToUpper
            | Builtin::StrToLower => "",
        };
        let mut checked = Vec::with_capacity(args.len());
        for (arg, param) in args.iter().zip(&params) {
            let arg = self.check_expr(fcx, arg);
            if !fcx.icx.unify(&arg.ty, param) {
                self.error(
                    "E0010",
                    format!(
                        "`{}` expects a `{}`, found `{}`{hint}",
                        builtin.name(),
                        self.show(param, fcx),
                        self.show(&arg.ty, fcx),
                    ),
                    arg.span,
                );
            }
            checked.push(arg);
        }
        hir::Expr {
            kind: hir::ExprKind::Call {
                func: hir::Callee::Builtin(builtin),
                type_args: Vec::new(),
                args: checked,
            },
            ty: ret,
            span,
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

    /// `[init; n]` — an `n`-slot array whose every slot is `init`. The element
    /// type comes from `init` (so no `Default` bound is needed and a fresh array
    /// is never uninitialized), and `n` must be an `Int` evaluated at runtime.
    ///
    /// `init` is evaluated once and the same value goes into every slot, so for
    /// a heap element type the slots are one object, not `n` of them — see
    /// `hir::ExprKind::ArrayRepeat`.
    fn check_array_repeat(
        &mut self,
        fcx: &mut FnCtx,
        init: &Spanned<ast::Expr>,
        len: &Spanned<ast::Expr>,
        span: Span,
    ) -> hir::Expr {
        let init_hir = self.check_expr(fcx, init);
        let len_hir = self.check_expr(fcx, len);
        self.expect_ty(fcx, &len_hir, &Ty::Int, "an array length");
        let elem_ty = fcx.icx.apply(&init_hir.ty);
        hir::Expr {
            kind: hir::ExprKind::ArrayRepeat {
                init: Box::new(init_hir),
                len: Box::new(len_hir),
            },
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
        let Some(def_id) = self.defs.resolve_type(self.cur_module, name) else {
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
        let recv_ty = fcx.icx.apply(&recv.ty);
        if let Some((index, field_ty)) = self.record_field_index_and_ty(fcx, &recv_ty, &field.value)
        {
            return hir::Expr {
                kind: hir::ExprKind::FieldGet {
                    target: Box::new(recv),
                    index,
                },
                ty: field_ty,
                span,
            };
        }
        // A broken receiver already reported its own error; another one here
        // would just cascade.
        if matches!(recv_ty, Ty::Error) {
            return error_expr(span);
        }
        self.error(
            "E0014",
            self.no_field_message(fcx, &recv_ty, &field.value),
            field.span,
        );
        error_expr(span)
    }

    /// The "no such field" message for a resolved, non-`Error` receiver type
    /// that failed `record_field_index_and_ty`: names the record when the
    /// receiver is one, otherwise describes the type. Shared by the field
    /// read and write paths (`check_field`, `check_field_set`) for the same
    /// reason `record_field_index_and_ty` itself is shared — so the two
    /// cannot independently drift on how the same mistake is phrased, which
    /// is exactly what had happened before this was pulled out (the write
    /// path said "no field `x` on type `P`" and had no separate wording for a
    /// non-record receiver at all).
    fn no_field_message(&self, fcx: &FnCtx, recv_ty: &Ty, field_name: &str) -> String {
        let record_name = match recv_ty {
            Ty::Record { def_id, .. } => self
                .records
                .iter()
                .find(|r| r.def_id == *def_id)
                .map(|r| r.name.clone()),
            _ => None,
        };
        match record_name {
            Some(record_name) => format!("no field `{field_name}` on record `{record_name}`"),
            None => format!(
                "cannot access field `{field_name}` on `{}`",
                self.show(recv_ty, fcx)
            ),
        }
    }

    /// Resolve a field name on a record type to its `(index, substituted type)`.
    /// Shared by the field read and field write paths so they cannot disagree
    /// about layout or generic substitution.
    ///
    /// Emits no diagnostics: a `None` means "no such field on this type", and
    /// each caller phrases that in its own terms.
    fn record_field_index_and_ty(
        &mut self,
        fcx: &mut FnCtx,
        recv_ty: &Ty,
        field: &str,
    ) -> Option<(u32, Ty)> {
        let Ty::Record { def_id, args } = fcx.icx.apply(recv_ty) else {
            return None;
        };
        let record = self.records.iter().find(|r| r.def_id == def_id)?;
        let index = record.fields.iter().position(|f| f.name == field)?;
        // The field's declared type is written in terms of the record's own
        // type parameters, so it must be substituted with this instantiation's
        // arguments before it means anything to the caller.
        let field_ty = record.fields.get(index)?.ty.subst(&args);
        Some((index as u32, field_ty))
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

    /// The index of the trait method `name` that can be *called on a receiver*.
    /// A receiver-less method is skipped, exactly as `find_inherent_method` skips
    /// a `selfless` inherent method: there is no receiver slot to bind `x` into,
    /// so `x.zero()` must report the existing `E0014: no method` rather than
    /// dispatch a receiver into a signature that has no place for it — which
    /// lowered to a call with one argument too many and ICEd in codegen.
    fn trait_method_index(&self, trait_id: DefId, name: &str) -> Option<u32> {
        self.traits
            .iter()
            .find(|t| t.def_id == trait_id)?
            .methods
            .iter()
            .position(|m| m.name == name && m.has_self)
            .map(|i| i as u32)
    }

    /// The index of the trait *associated function* (no `self` receiver) named
    /// `name` — the `Type::name(…)` counterpart of `trait_method_index`.
    fn trait_assoc_fn_index(&self, trait_id: DefId, name: &str) -> Option<u32> {
        self.traits
            .iter()
            .find(|t| t.def_id == trait_id)?
            .methods
            .iter()
            .position(|m| m.name == name && !m.has_self)
            .map(|i| i as u32)
    }

    fn find_inherent_method(&self, recv_ty: &Ty, head: TyHead, name: &str) -> Option<DefId> {
        self.impls
            .iter()
            .filter(|i| i.trait_id.is_none() && i.self_head == head)
            // The receiver must fit the impl's self-type pattern, not just its
            // head, so `impl<T> Pair<T, T>` is skipped for `Pair<Int, String>`.
            .filter(|i| i.match_args(recv_ty).is_some())
            .find_map(|i| {
                // An associated function has no receiver to bind, so it is not a
                // candidate for `x.m()`; skipping it here reports the existing
                // `E0014: no method` instead of dispatching a receiver into a
                // signature that has no slot for it.
                i.methods
                    .iter()
                    .find(|(n, d)| n == name && !self.selfless.contains(d))
                    .map(|(_, d)| *d)
            })
    }

    /// Find every self-less method named `name` on an inherent impl whose self
    /// type has the head `head`. Associated functions have no receiver, so
    /// selection is by the impl's nominal head only — there is no receiver type
    /// to run `match_args` against, unlike `find_inherent_method`. That is
    /// deliberately permissive: `Box::make(5)` must reach
    /// `impl Box<Int> { fn make(…) }` even though the qualifier's type argument
    /// is still an inference variable at this point, so filtering by
    /// `match_args` (which compares structurally, and so cannot line `Int` up
    /// with a variable) would reject the single-candidate case users want.
    ///
    /// The price of that permissiveness is that two *disjoint concrete* impls
    /// of a generic type — `impl Box<Int>` and `impl Box<Bool>` — are both
    /// candidates. `check_impl_coherence` does not catch that pair either
    /// (their self types do not overlap, so there is no `E0074`), so returning
    /// the first would make dispatch depend on impl declaration order, the
    /// exact invariant that check exists to protect. Hence every candidate is
    /// returned and the caller reports `E0015`, mirroring
    /// `find_trait_assoc_fns`.
    ///
    /// Keyed on the head rather than a type `DefId` so that `impl Int { fn … }`
    /// is reachable: a primitive has no `DefId` at all (it never enters the
    /// resolver's type namespace), yet `collect_impls` records an impl on one
    /// under its primitive head just like any other.
    fn find_assoc_fns(&self, head: TyHead, name: &str) -> Vec<DefId> {
        self.impls
            .iter()
            .filter(|i| i.trait_id.is_none() && i.self_head == head)
            .filter_map(|i| {
                i.methods
                    .iter()
                    .find(|(n, d)| n == name && self.selfless.contains(d))
                    .map(|(_, d)| *d)
            })
            .collect()
    }

    /// The self type named by a two-segment path's qualifier (`Int::zero()`,
    /// `P::new()`, `Box::make(…)`), for associated-function lookup.
    ///
    /// A primitive type name never enters the resolver's *type* namespace —
    /// `insert_type` is only called for `record`/`type` items, so
    /// `Definitions::resolve_type(module, "Int")` is always `None` — and is
    /// instead a separate arm of `convert_ty`. Matching those names here produces
    /// exactly the `Ty` `convert_ty` produces, hence exactly the `TyHead` that
    /// `collect_impls` recorded as an impl's `self_head`, so `impl Zero for Int`
    /// is reachable from `Int::zero()`.
    ///
    /// A generic nominal type's arguments become fresh inference variables,
    /// recovered from the call's context the way `emit_assoc_call` recovers an
    /// impl's generics; an unresolvable one is reported by the residual
    /// inference-variable check.
    fn qualifier_self_ty(&self, fcx: &mut FnCtx, name: &str) -> Option<Ty> {
        match name {
            "Int" => return Some(Ty::Int),
            "Float" => return Some(Ty::Float),
            "Bool" => return Some(Ty::Bool),
            "Char" => return Some(Ty::Char),
            "String" => return Some(Ty::String),
            _ => {}
        }
        let def_id = self.defs.resolve_type(self.cur_module, name)?;
        let arity = self.type_arity.get(&def_id).copied().unwrap_or(0);
        let args: Vec<Ty> = (0..arity).map(|_| fcx.icx.fresh()).collect();
        match self.defs.def(def_id).kind {
            DefKind::Record { .. } => Some(Ty::Record { def_id, args }),
            DefKind::Sum { .. } => Some(Ty::Sum { def_id, args }),
            _ => None,
        }
    }

    /// Find a receiver-less trait method `name` reachable through a trait impl
    /// for `self_ty`. Selection mirrors `resolve_method_on`'s trait branch — an
    /// impl must share the head *and* fit the self type — restricted to methods
    /// the trait declares without a `self` receiver, since a `Type::name(…)` call
    /// site has no receiver to pass. Returns every candidate so the caller can
    /// report ambiguity rather than silently picking one.
    ///
    /// A qualifier whose generic arguments are still inference variables
    /// (`Box::zero()`) only matches an impl whose self type is generic in the
    /// same positions (`impl<T> Zero for Box<T>`), because `match_args` compares
    /// structurally: a concrete `impl Zero for Box<Int>` is not found until the
    /// qualifier's argument is already known, which at a two-segment path it
    /// never is. Concrete and fully-generic self types both work; the caller
    /// reports the unresolved-argument case as E0011, naming the type
    /// arguments as the reason rather than reporting a missing associated
    /// function that in fact exists.
    fn find_trait_assoc_fns(&self, self_ty: &Ty, name: &str) -> Vec<(DefId, u32)> {
        let Some(head) = self_ty.head() else {
            return Vec::new();
        };
        self.impls
            .iter()
            .filter(|i| i.self_head == head && i.match_args(self_ty).is_some())
            .filter_map(|i| i.trait_id)
            .filter_map(|tid| self.trait_assoc_fn_index(tid, name).map(|idx| (tid, idx)))
            .collect()
    }

    /// Check `receiver.method(args)`. `receiver` is the already-checked HIR;
    /// `receiver_ast` is the same expression before checking, needed because the
    /// mutable-receiver rule below classifies it with `place_root`.
    ///
    /// **The rule covers inherent methods only.** A trait method's `mut self`
    /// is declared on the *trait*, not on the impl, and trait dispatch here
    /// resolves to `(trait_id, method_index)` — for a generic receiver
    /// (`fn f<T: Tr>(x: T) { x.m() }`) there is no single impl to read the
    /// receiver's mutability off. Enforcing it there needs a `mut_self` flag on
    /// `hir::TraitMethod` beside `has_self`, plus a conformance rule keeping the
    /// impl's receiver in step with the trait's. Deliberately deferred; ADR 0005
    /// records it as a known gap.
    fn check_method_call(
        &mut self,
        fcx: &mut FnCtx,
        receiver: hir::Expr,
        receiver_ast: &Spanned<ast::Expr>,
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
                self.check_mutable_receiver(fcx, def_id, receiver_ast, span);
                self.emit_inherent_call(fcx, def_id, receiver, args, span)
            }
            MethodRes::Trait(trait_id, method_idx) => self.emit_trait_call(
                fcx,
                trait_id,
                method_idx,
                TraitCallSelf::Receiver(receiver),
                args,
                span,
            ),
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

    /// A method declaring `mut self` mutates its receiver, so the receiver must
    /// be reachable through a mutable binding — the same requirement, and the
    /// same `place_root` walk, as `arr[i] = v` and `rec.f = v`. Without it
    /// `v.push(x)` would mutate `v` after `let v = …` while `v.field = x` on the
    /// same binding was rejected: one operation, two answers (ADR 0005).
    ///
    /// A no-op for any method that does not declare `mut self`, so a plain
    /// `self` reader still works on an immutable binding.
    fn check_mutable_receiver(
        &mut self,
        fcx: &FnCtx,
        def_id: DefId,
        receiver_ast: &Spanned<ast::Expr>,
        span: Span,
    ) {
        if !self.mut_self.contains(&def_id) {
            return;
        }
        let mname = self.defs.def(def_id).name.clone();
        self.require_mutable_place(fcx, receiver_ast, span, MutTarget::Receiver(mname));
    }

    fn emit_inherent_call(
        &mut self,
        fcx: &mut FnCtx,
        def_id: DefId,
        receiver: hir::Expr,
        args: Vec<hir::Expr>,
        span: Span,
    ) -> hir::Expr {
        // Everything below assumes a receiver at `sig.params[0]`; an associated
        // function has none, so the arity arithmetic and the receiver unification
        // would both be off by one. `find_inherent_method` already skips these —
        // this guard keeps any future caller from reintroducing the codegen ICE.
        if self.selfless.contains(&def_id) {
            let mname = self.defs.def(def_id).name.clone();
            self.error(
                "E0014",
                format!(
                    "`{mname}` is an associated function with no `self` receiver; \
                     call it as `Type::{mname}(…)`"
                ),
                span,
            );
            return error_expr(span);
        }
        let Some(sig) = self.sigs.get(&def_id).cloned() else {
            return error_expr(span);
        };
        // sig.params[0] is `self`; the rest are the declared parameters.
        let expected_args = sig.params.len().saturating_sub(1);
        if args.len() != expected_args {
            // One wording for every arity error in this family (see also
            // `emit_assoc_call`, `emit_trait_call`, and the free-function call in
            // `check_call`), so the same mistake always reads the same way.
            let mname = self.defs.def(def_id).name.clone();
            self.error(
                "E0016",
                format!(
                    "`{mname}` takes {expected_args} argument(s) but {} were supplied",
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

    /// Emit a call to an associated function (`Type::f(args)`). Unlike
    /// `emit_inherent_call`, there is no receiver: `sig.params` holds exactly
    /// the declared parameters (see `Checker::selfless`), so arity is compared
    /// directly rather than via `saturating_sub(1)`. The impl's generic
    /// parameters cannot be recovered from a receiver either, so they become
    /// fresh inference variables resolved by the surrounding context (e.g. a
    /// `let` annotation); an unresolved one is reported by the existing
    /// residual-inference-variable check.
    fn emit_assoc_call(
        &mut self,
        fcx: &mut FnCtx,
        def_id: DefId,
        args: Vec<hir::Expr>,
        span: Span,
    ) -> hir::Expr {
        let Some(sig) = self.sigs.get(&def_id).cloned() else {
            return error_expr(span);
        };
        if args.len() != sig.params.len() {
            let fname = self.defs.def(def_id).name.clone();
            self.error(
                "E0016",
                format!(
                    "`{fname}` takes {} argument(s) but {} were supplied",
                    sig.params.len(),
                    args.len()
                ),
                span,
            );
            return error_expr(span);
        }
        let type_args: Vec<Ty> = (0..sig.generics).map(|_| fcx.icx.fresh()).collect();
        for (arg, param) in args.iter().zip(sig.params.iter()) {
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
        hir::Expr {
            kind: hir::ExprKind::Call {
                func: hir::Callee::Def(def_id),
                type_args,
                args,
            },
            ty: ret,
            span,
        }
    }

    /// Emit a trait method call — the sole constructor of
    /// [`hir::ExprKind::TraitCall`]. `dispatch` says where `Self` comes from and
    /// so distinguishes a receiver call (`x.cmp(y)`) from an associated-function
    /// call (`Int::default()`, or `T::default()` inside a generic function);
    /// everything else — the `has_self`/receiver agreement check, the flat
    /// substitution, arity, argument unification, the result type — is shared,
    /// which is the point. For [`TraitCallSelf::Qualifier`], `Self` is a concrete
    /// type for `Int::zero()` or `Param(k)` when dispatching through a generic
    /// parameter's bound (`T::zero()` inside `fn f<T: Zero>()`), which
    /// monomorphization resolves once `T` is known, exactly as it resolves a
    /// bounded instance method call.
    fn emit_trait_call(
        &mut self,
        fcx: &mut FnCtx,
        trait_id: DefId,
        method_idx: u32,
        dispatch: TraitCallSelf,
        args: Vec<hir::Expr>,
        span: Span,
    ) -> hir::Expr {
        let Some(tm) = self
            .traits
            .iter()
            .find(|t| t.def_id == trait_id)
            .and_then(|t| t.methods.get(method_idx as usize))
            .cloned()
        else {
            return error_expr(span);
        };
        // The receiver's presence must agree with what the trait declares.
        // `tm.params` never stores `self`, so it cannot reveal a mismatch: the
        // arity check below would pass and MIR would lower one argument too many
        // (or too few), which is the Cranelift "mismatched argument count" ICE in
        // one direction or the other. `trait_method_index` and
        // `trait_assoc_fn_index` already partition candidates by `has_self`, so
        // neither arm is reachable today — this keeps any future caller from
        // reintroducing the ICE, mirroring `emit_inherent_call`, and being one
        // check covering both directions it cannot rot on only one side.
        let (self_ty, receiver) = match dispatch {
            TraitCallSelf::Receiver(_) if !tm.has_self => {
                self.error(
                    "E0014",
                    format!(
                        "`{}` is an associated function with no `self` receiver; \
                         call it as `Type::{}(…)`",
                        tm.name, tm.name
                    ),
                    span,
                );
                return error_expr(span);
            }
            // `Self` is the receiver's type; deriving it here rather than taking
            // it from the caller is what makes the two impossible to disagree.
            TraitCallSelf::Receiver(recv) => (fcx.icx.apply(&recv.ty), Some(Box::new(recv))),
            TraitCallSelf::Qualifier(_) if tm.has_self => {
                self.error(
                    "E0014",
                    format!(
                        "`{}` is a method with a `self` receiver; \
                         call it on a value as `value.{}(…)`",
                        tm.name, tm.name
                    ),
                    span,
                );
                return error_expr(span);
            }
            TraitCallSelf::Qualifier(ty) => (ty, None),
        };
        // Substitution over the trait method's flat Param space: `Self` is
        // Param(0); the method's own generics are Param(1..) and become fresh
        // inference vars, recovered from the argument types like a generic fn.
        // `hir::TraitMethod::bounds` is indexed by that same flat position, so
        // this order is not a convention but a requirement.
        let type_args: Vec<Ty> = (0..tm.generics).map(|_| fcx.icx.fresh()).collect();
        let mut subst = Vec::with_capacity(1 + type_args.len());
        subst.push(self_ty.clone());
        subst.extend(type_args.iter().cloned());
        if args.len() != tm.params.len() {
            self.error(
                "E0016",
                format!(
                    "`{}` takes {} argument(s) but {} were supplied",
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
                self_ty,
                type_args,
                receiver,
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
        let tm = self
            .traits
            .iter()
            .find(|t| t.def_id == trait_id)?
            .methods
            .get(method_idx as usize)?;
        if !tm.params.is_empty() || tm.ret != Ty::String {
            return None;
        }
        let span = value.span;
        Some(self.emit_trait_call(
            fcx,
            trait_id,
            method_idx,
            TraitCallSelf::Receiver(value),
            Vec::new(),
            span,
        ))
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
        // Field assignment `rec.field = v`.
        if let ast::Expr::Field { target, field } = &lhs.value {
            return self.check_field_set(fcx, op, target, field, rhs, span);
        }
        let ast::Expr::Path(path) = &lhs.value else {
            // `arr[i] = v` and `rec.f = v` are handled above, so what is left is
            // a target with no assignable place at all (a call result, a
            // literal, a parenthesized expression, …).
            self.unsupported(
                lhs.span,
                "assignments to anything but a local variable, array element, or record field",
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

    /// Require that `target`'s storage be reachable through a mutable binding,
    /// reporting `E0060` if it is not.
    ///
    /// The one place all three mutation forms share: `arr[i] = v`,
    /// `rec.f = v`, and a call to a method declaring `mut self`. They differ
    /// only in how the diagnostic names what is being mutated (`what`), so the
    /// `place_root` classification, the error code, and — crucially — the
    /// actionable note live here rather than in three hand-maintained copies.
    fn require_mutable_place(
        &mut self,
        fcx: &FnCtx,
        target: &Spanned<ast::Expr>,
        span: Span,
        what: MutTarget,
    ) {
        match self.place_root(fcx, target) {
            PlaceRoot::Mutable => {}
            PlaceRoot::ImmutableLocal(name) => {
                self.error("E0060", what.immutable_message(&name), span);
                // `let mut self` is not Nova syntax — a receiver's mutability is
                // declared in the signature, so a `self` root needs the other
                // advice entirely. `place_root` hands back the root's *name*,
                // which is all it takes to tell the two apart; `self` is only
                // ever bound as a method receiver.
                let note = if name == "self" {
                    "declare the enclosing method's receiver as `mut self` to allow mutation"
                        .to_string()
                } else {
                    format!("declare it as `let mut {name}` to allow mutation")
                };
                self.diagnostics
                    .last_mut()
                    .expect("just pushed")
                    .notes
                    .push(note);
            }
            PlaceRoot::NotAPlace => {
                self.error("E0060", what.not_a_place_message(), span);
            }
        }
    }

    /// Check `arr[index] = value`.
    ///
    /// Invariant shared with [`Checker::check_field_set`]: `rhs` is always
    /// type-checked, whatever early return this function takes. A mistake in
    /// the right-hand side (an unresolved call, say) is independent of
    /// whatever is wrong with the assignment target — an unsupported
    /// compound-assignment shape, a non-array receiver, an unknown field —
    /// so it is reported exactly as it already is on the path where the
    /// target is fine. The one thing deliberately *not* repeated is a second
    /// diagnostic about the target itself once one has already been reported
    /// for it (`check_field_set`'s `Ty::Error`-receiver case): that would be
    /// the same mistake twice, not an independent one.
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
            self.check_expr(fcx, rhs);
            return error_expr(span);
        }
        // The array's storage must be reachable through a mutable binding.
        // Walk the whole index/field chain to its root local — `grid[0][1]`,
        // `rec.data[0]`, and `make()[0]` all bypass a single-segment check.
        self.require_mutable_place(fcx, target, span, MutTarget::Element);
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
            self.check_expr(fcx, rhs);
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

    /// Check `target.field = value`. Shares `check_index_set`'s invariant
    /// that `rhs` is type-checked on every early return, not only the path
    /// where the target resolves cleanly.
    fn check_field_set(
        &mut self,
        fcx: &mut FnCtx,
        op: ast::AssignOp,
        target: &Spanned<ast::Expr>,
        field: &Spanned<String>,
        rhs: &Spanned<ast::Expr>,
        span: Span,
    ) -> hir::Expr {
        if !matches!(op, ast::AssignOp::Assign) {
            self.unsupported(span, "compound assignment to a record field");
            self.check_expr(fcx, rhs);
            return error_expr(span);
        }
        // The record's storage must be reachable through a mutable binding.
        // Walk the whole field/index chain to its root local — `rec.inner.f`
        // and `make().f` both bypass a single-segment check. Records have no
        // per-field `mut`, so mutability is a property of the binding.
        self.require_mutable_place(fcx, target, span, MutTarget::Field);
        let rec = self.check_expr(fcx, target);
        let recv_ty = fcx.icx.apply(&rec.ty);
        // A broken receiver already reported its own error; another one here
        // would just cascade, as it would on the read path.
        if matches!(recv_ty, Ty::Error) {
            self.check_expr(fcx, rhs);
            return error_expr(span);
        }
        // Resolve the field to its index and declared type through the same
        // lookup `FieldGet` uses for reads, so reads and writes cannot disagree
        // about layout.
        let Some((index, field_ty)) = self.record_field_index_and_ty(fcx, &recv_ty, &field.value)
        else {
            // Same wording as the read path (`no_field_message`), including
            // its separate not-a-record case.
            self.error(
                "E0014",
                self.no_field_message(fcx, &recv_ty, &field.value),
                field.span,
            );
            self.check_expr(fcx, rhs);
            return error_expr(span);
        };
        let value = self.check_expr(fcx, rhs);
        if !fcx.icx.unify(&value.ty, &field_ty) {
            self.error(
                "E0010",
                format!(
                    "field `{}` has type `{}` but `{}` was assigned",
                    field.value,
                    self.show(&field_ty, fcx),
                    self.show(&value.ty, fcx),
                ),
                rhs.span,
            );
        }
        hir::Expr {
            kind: hir::ExprKind::FieldSet {
                target: Box::new(rec),
                index,
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
        // Normalized (pattern, has-guard, span) per arm, for the exhaustiveness
        // and reachability analysis after all arms are checked.
        let mut arm_pats: Vec<(usefulness::Pat, bool, Span)> = Vec::new();

        for arm in arms {
            let guarded = arm.guard.is_some();
            if guarded {
                self.unsupported(arm.pattern.span, "match guards");
            }
            fcx.scopes.push(FxHashMap::default());
            let pattern = self.check_pattern(fcx, &arm.pattern, &scrut.ty);
            let upat = to_useful_pat(&pattern);
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
            arm_pats.push((upat, guarded, arm.pattern.span));
            hir_arms.push(hir::Arm {
                pattern,
                body,
                span: arm.pattern.span,
            });
        }

        self.check_match_usefulness(fcx, &scrut, &arm_pats, span);

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

    /// Exhaustiveness and reachability via Maranget's usefulness algorithm
    /// (see `usefulness.rs`): an unguarded arm is unreachable when it is not
    /// useful against the earlier arms (E0021), and the match is non-exhaustive
    /// when a wildcard row is still useful against all arms — the witnesses
    /// name the uncovered values (E0020).
    fn check_match_usefulness(
        &mut self,
        fcx: &mut FnCtx,
        scrut: &hir::Expr,
        arm_pats: &[(usefulness::Pat, bool, Span)],
        span: Span,
    ) {
        let scrut_ty = fcx.icx.apply(&scrut.ty);
        // A type we cannot yet resolve to constructors (an unresolved inference
        // variable or generic parameter) carries no enumerable signature, so we
        // cannot analyze its arms — with one exception: a match with *no* arms
        // still leaves every value of an inhabited type uncovered. `Never` is
        // uninhabited (an empty match is fine) and `Error` already reported a
        // type error, so neither should pile on here.
        if matches!(scrut_ty, Ty::Var(_) | Ty::Param(_) | Ty::Error | Ty::Never) {
            if arm_pats.is_empty() && matches!(scrut_ty, Ty::Var(_) | Ty::Param(_)) {
                self.error(
                    "E0020",
                    "non-exhaustive match: this match has no arms",
                    span,
                );
                self.diagnostics
                    .last_mut()
                    .expect("just pushed")
                    .notes
                    .push("a `match` on an inhabited type needs at least one arm".to_string());
            }
            return;
        }
        // Clone the sum table so the analysis context does not hold a borrow of
        // `self` across the diagnostic calls below.
        let sums = self.sums.clone();
        let cx = usefulness::MatchCx::new(&sums);
        let col = [scrut_ty];

        // Reachability: each unguarded arm must cover a value no earlier
        // (unguarded) arm does. A guarded arm's match is conditional, so it
        // neither is reported nor counts toward coverage.
        let mut prior: Vec<Vec<usefulness::Pat>> = Vec::new();
        for (pat, guarded, arm_span) in arm_pats {
            if *guarded {
                continue;
            }
            if cx
                .usefulness(&prior, std::slice::from_ref(pat), &col)
                .is_empty()
            {
                self.diagnostics.push(
                    Diagnostic::warning("E0021", "unreachable match arm")
                        .with_primary_label(*arm_span, "this arm is never reached")
                        .with_note(
                            "an earlier arm already matches every value this one would".to_string(),
                        ),
                );
            }
            prior.push(vec![pat.clone()]);
        }

        // Exhaustiveness: a wildcard must not be useful against all arms.
        let witnesses = cx.usefulness(&prior, &[usefulness::Pat::Wild], &col);
        if !witnesses.is_empty() {
            let mut rendered: Vec<String> = witnesses
                .iter()
                .filter_map(|w| w.first())
                .map(|p| self.render_witness(p))
                .collect();
            rendered.dedup();
            self.error(
                "E0020",
                format!(
                    "non-exhaustive match: pattern{} {} not covered",
                    if rendered.len() == 1 { "" } else { "s" },
                    rendered
                        .iter()
                        .map(|w| format!("`{w}`"))
                        .collect::<Vec<_>>()
                        .join(", "),
                ),
                span,
            );
            self.diagnostics
                .last_mut()
                .expect("just pushed")
                .notes
                .push("add the missing arms or a `_ => ...` catch-all".to_string());
        }
    }

    /// Render a witness pattern for a non-exhaustiveness diagnostic.
    fn render_witness(&self, pat: &usefulness::Pat) -> String {
        match pat {
            usefulness::Pat::Wild => "_".to_string(),
            usefulness::Pat::Ctor(ctor, args) => match ctor {
                usefulness::Ctor::Bool(b) => b.to_string(),
                usefulness::Ctor::Int(v) => v.to_string(),
                usefulness::Ctor::Str(s) => format!("{s:?}"),
                usefulness::Ctor::Variant(sum, vi) => {
                    let name = self
                        .sums
                        .iter()
                        .find(|s| s.def_id == *sum)
                        .and_then(|s| s.variants.get(*vi as usize))
                        .map(|v| v.name.clone())
                        .unwrap_or_else(|| "?".to_string());
                    if args.is_empty() {
                        name
                    } else {
                        let inner: Vec<String> =
                            args.iter().map(|a| self.render_witness(a)).collect();
                        format!("{name}({})", inner.join(", "))
                    }
                }
            },
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
                if let Some(Res::Variant(sum_id, vi)) =
                    self.defs.resolve_value(self.cur_module, &name.value)
                {
                    if self.variant_matches_scrutinee(fcx, sum_id, scrut_ty) {
                        return self.variant_pattern(fcx, sum_id, vi, &[], scrut_ty, pattern.span);
                    }
                    // The name is a known constructor, but of a different type:
                    // reject it rather than silently binding a catch-all (which
                    // would mask uncovered cases), mirroring the `Path` and
                    // `TupleStruct` arms.
                    self.error(
                        "E0001",
                        format!("`{}` is not a variant of the matched type", name.value),
                        pattern.span,
                    );
                    return hir::Pattern::Wildcard;
                }
                let local = fcx.new_local(name.value.clone(), scrut_ty.clone(), *is_mut, name.span);
                hir::Pattern::Bind(local)
            }
            ast::Pattern::Path(path) if path.segments.len() == 1 => {
                let name = &path.segments[0].value;
                if let Some(Res::Variant(sum_id, vi)) =
                    self.defs.resolve_value(self.cur_module, name)
                {
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
                if let Some(sum_id) = self.defs.resolve_type(self.cur_module, ty_name) {
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
                    match self
                        .defs
                        .resolve_value(self.cur_module, &path.segments[0].value)
                    {
                        Some(Res::Variant(sum_id, vi)) => Some((sum_id, vi)),
                        _ => None,
                    }
                } else if path.segments.len() == 2 {
                    self.defs
                        .resolve_type(self.cur_module, &path.segments[0].value)
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
        K::FieldSet { target, value, .. } => {
            out.push(target);
            out.push(value);
        }
        K::ArrayRepeat { init, len } => {
            out.push(init);
            out.push(len);
        }
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
            // `None` for a trait associated function — no receiver to visit.
            out.extend(receiver.iter().map(|r| r.as_ref()));
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
        K::FieldSet { target, value, .. } => {
            out.push(target);
            out.push(value);
        }
        K::ArrayRepeat { init, len } => {
            out.push(init);
            out.push(len);
        }
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
            out.extend(receiver.iter_mut().map(|r| r.as_mut()));
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

/// Set equality on two trait-bound lists, ignoring order and duplicates.
fn same_bound_set(a: &[DefId], b: &[DefId]) -> bool {
    a.iter().all(|d| b.contains(d)) && b.iter().all(|d| a.contains(d))
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

/// Every builtin's parameter types and result type — the whole of what
/// `check_builtin_call` needs to know about one, so that arity and argument
/// checking is a single shared code path rather than an arm per builtin.
///
/// It has to be a table rather than per-builtin code because most of these
/// are *not callable from a user program*: `Builtin::STD_ONLY` members
/// (`str_cmp`, `str_hash`, `char_to_int`, `str_len_chars`, `str_chars`) are
/// seeded only into std modules' scopes, so their arity/type diagnostics are
/// unreachable from any Nova source and cannot be tested through it. Sharing
/// one checking path means the reachable builtins (`println`/`print`/`panic`)
/// exercise it, and this function's own table is directly unit-testable —
/// see `builtin_signatures_are_what_the_std_call_sites_use`. Its `match` is
/// exhaustive over the *type* `Builtin` (the compiler rejects a missing
/// variant there), and it drives that match with every entry of
/// `Builtin::ALL` — itself generated by the `builtins!` macro from the same
/// list that declares the enum, so *that* list cannot omit or duplicate a
/// variant either (see its doc comment). Neither piece alone would close the
/// gap; together they do.
fn builtin_signature(builtin: Builtin) -> (Vec<Ty>, Ty) {
    match builtin {
        Builtin::Println | Builtin::Print => (vec![Ty::String], Ty::Unit),
        Builtin::Panic => (vec![Ty::String], Ty::Never),
        Builtin::StrCmp => (vec![Ty::String, Ty::String], Ty::Int),
        Builtin::StrHash => (vec![Ty::String], Ty::Int),
        Builtin::CharToInt => (vec![Ty::Char], Ty::Int),
        Builtin::StrLenChars => (vec![Ty::String], Ty::Int),
        Builtin::StrChars => (vec![Ty::String], Ty::Array(Box::new(Ty::Char))),
        Builtin::StrFromChars => (vec![Ty::Array(Box::new(Ty::Char))], Ty::String),
        Builtin::StrToUpper | Builtin::StrToLower => (vec![Ty::String], Ty::String),
    }
}

/// Normalize a checked HIR pattern into the usefulness-algorithm form.
/// Bindings and wildcards are both "match anything"; variant payloads are
/// irrefutable in Phase 1, so each contributes a wildcard sub-pattern.
fn to_useful_pat(p: &hir::Pattern) -> usefulness::Pat {
    use usefulness::{Ctor, Pat};
    match p {
        hir::Pattern::Wildcard | hir::Pattern::Bind(_) => Pat::Wild,
        hir::Pattern::LitInt(v) => Pat::Ctor(Ctor::Int(*v), Vec::new()),
        hir::Pattern::LitBool(v) => Pat::Ctor(Ctor::Bool(*v), Vec::new()),
        hir::Pattern::LitStr(v) => Pat::Ctor(Ctor::Str(v.clone()), Vec::new()),
        hir::Pattern::Variant {
            sum,
            variant,
            binders,
        } => Pat::Ctor(
            Ctor::Variant(*sum, *variant),
            vec![Pat::Wild; binders.len()],
        ),
    }
}

/// What `require_mutable_place` is being asked to mutate — the only thing that
/// differs between the three callers, and so the only thing that varies in the
/// `E0060` they report.
enum MutTarget {
    /// `arr[i] = v`.
    Element,
    /// `rec.f = v`.
    Field,
    /// A call to a method declaring `mut self`; carries the callee's
    /// impl-qualified `Def` name, the spelling every other inherent-method
    /// diagnostic uses.
    Receiver(String),
}

impl MutTarget {
    /// The message when the place is rooted at an immutable local named `name`.
    fn immutable_message(&self, name: &str) -> String {
        match self {
            MutTarget::Element => format!("cannot assign to an element of immutable `{name}`"),
            MutTarget::Field => format!("cannot assign to a field of immutable `{name}`"),
            MutTarget::Receiver(m) => {
                format!("`{m}` mutates its receiver, but `{name}` is immutable")
            }
        }
    }

    /// The message when the place has no assignable root at all.
    fn not_a_place_message(&self) -> String {
        match self {
            MutTarget::Element => {
                "cannot assign to an element of a temporary or non-assignable value".to_string()
            }
            MutTarget::Field => {
                "cannot assign to a field of a temporary or non-assignable value".to_string()
            }
            MutTarget::Receiver(m) => {
                format!("`{m}` mutates its receiver, which cannot be a temporary")
            }
        }
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
        hir::ExprKind::FieldSet { target, value, .. } => {
            finalize_expr(target, icx, residual);
            finalize_expr(value, icx, residual);
        }
        hir::ExprKind::ArrayRepeat { init, len } => {
            finalize_expr(init, icx, residual);
            finalize_expr(len, icx, residual);
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
            type_args,
            receiver,
            args,
            ..
        } => {
            *self_ty = icx.apply(self_ty);
            if self_ty.has_vars() {
                residual.push(expr.span);
            }
            // Resolve the method's own generic args; an uninferable one is E0011.
            for t in type_args.iter_mut() {
                *t = icx.apply(t);
                if t.has_vars() {
                    residual.push(expr.span);
                }
            }
            if let Some(receiver) = receiver {
                finalize_expr(receiver, icx, residual);
            }
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
        // Type-check against the merged file (input + the implicit prelude,
        // which is `std/core` — see ADR 0004), whose `item_index`es the
        // definitions refer to.
        check(&resolved.file, &resolved.definitions)
    }

    fn error_codes(result: &CheckResult) -> Vec<&str> {
        result
            .diagnostics
            .iter()
            .filter(|d| d.severity == nova_diagnostics::Severity::Error)
            .map(|d| d.code.as_str())
            .collect()
    }

    /// Every expression in the named function's body, outermost first, for
    /// tests that need to inspect what the checker actually built rather than
    /// trust the absence of diagnostics.
    fn exprs_in<'m>(module: &'m hir::Module, fn_name: &str) -> Vec<&'m hir::Expr> {
        fn walk<'e>(e: &'e hir::Expr, out: &mut Vec<&'e hir::Expr>) {
            out.push(e);
            for c in child_exprs(e) {
                walk(c, out);
            }
        }
        let f = module
            .functions
            .iter()
            .find(|f| f.name == fn_name)
            .expect("the named function exists");
        let mut exprs = Vec::new();
        walk(&f.body, &mut exprs);
        exprs
    }

    /// Every `FieldSet` in the named function, for tests that need to inspect
    /// the store the checker actually built rather than trust the absence of
    /// diagnostics.
    fn field_sets_in<'m>(module: &'m hir::Module, fn_name: &str) -> Vec<&'m hir::Expr> {
        exprs_in(module, fn_name)
            .into_iter()
            .filter(|e| matches!(e.kind, hir::ExprKind::FieldSet { .. }))
            .collect()
    }

    #[test]
    fn hello_world_checks() {
        let r = check_src("fn main() { println(\"hi\") }");
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
        // `functions` is whole-program — it also carries every method
        // std/core defines on Option/Result — so a total count shifts every
        // time std/core grows. Assert the relationship this test is actually
        // about instead: checking produced exactly one `main`, taking no
        // parameters.
        let mains: Vec<_> = r
            .module
            .functions
            .iter()
            .filter(|f| f.name == "main")
            .collect();
        assert_eq!(
            mains.len(),
            1,
            "expected exactly one `main`, got {:?}",
            r.module
                .functions
                .iter()
                .map(|f| f.name.as_str())
                .collect::<Vec<_>>()
        );
        assert_eq!(mains[0].params, 0, "main should take no parameters");
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
    fn inherent_method_generic_infers_and_checks() {
        // `map<U>` introduces its own `U`, inferred from the mapper's return.
        let r = check_src(
            "record Box<T> { value: T }\n\
             impl<T> Box<T> {\n\
                 fn map<U>(self, f: fn(T) -> U) -> Box<U> { Box { value: f(self.value) } }\n\
             }\n\
             fn twice(n: Int) -> Int { n * 2 }\n\
             fn main() {\n\
                 let a = Box { value: 5 }\n\
                 let b = a.map(twice)\n\
                 println(\"${b.value}\")\n\
             }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    }

    #[test]
    fn generic_trait_method_typechecks() {
        // A trait method with its own generic parameter, implemented and called.
        let r = check_src(
            "trait Mapper { fn remap<U>(self, f: fn(Int) -> U) -> U }\n\
             record Thing { v: Int }\n\
             impl Mapper for Thing { fn remap<U>(self, f: fn(Int) -> U) -> U { f(self.v) } }\n\
             fn dbl(n: Int) -> Int { n * 2 }\n\
             fn main() { let t = Thing { v: 21 }\n println(\"${t.remap(dbl)}\") }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    }

    #[test]
    fn generic_trait_method_arity_mismatch_reports_e0072() {
        // An impl method whose generic arity disagrees with the trait's.
        let r = check_src(
            "trait Mapper { fn remap<U>(self, f: fn(Int) -> U) -> U }\n\
             record Thing { v: Int }\n\
             impl Mapper for Thing { fn remap(self, f: fn(Int) -> Int) -> Int { f(self.v) } }\n\
             fn main() { }",
        );
        assert!(error_codes(&r).contains(&"E0072"), "{:?}", r.diagnostics);
    }

    #[test]
    fn method_generic_shadowing_impl_generic_reports_e0403() {
        let r = check_src(
            "record Box<T> { value: T }\n\
             impl<T> Box<T> { fn weird<T>(self, x: T) -> T { x } }\n\
             fn main() { let a = Box { value: 1 }\n println(\"${a.weird(2)}\") }",
        );
        assert!(error_codes(&r).contains(&"E0403"), "{:?}", r.diagnostics);
    }

    #[test]
    fn duplicate_method_generic_name_reports_e0403() {
        // Two method generics with the same name — rejected at the declaration,
        // not left as a silently-uncallable method.
        let r = check_src(
            "record Box<T> { value: T }\n\
             impl<T> Box<T> { fn weird<U, U>(self, a: U, b: U) -> U { a } }\n\
             fn main() { let a = Box { value: 1 }\n println(\"${a.value}\") }",
        );
        assert!(error_codes(&r).contains(&"E0403"), "{:?}", r.diagnostics);
    }

    #[test]
    fn duplicate_generic_on_free_function_reports_e0403() {
        // `fn f<U, U>` — the same name twice in a free function's generic list.
        let r = check_src(
            "fn f<U, U>(a: U, b: U) -> U { a }\n\
             fn main() { }",
        );
        assert!(error_codes(&r).contains(&"E0403"), "{:?}", r.diagnostics);
    }

    #[test]
    fn duplicate_generic_on_record_reports_e0403() {
        let r = check_src(
            "record R<T, T> { a: T }\n\
             fn main() { }",
        );
        assert!(error_codes(&r).contains(&"E0403"), "{:?}", r.diagnostics);
    }

    #[test]
    fn duplicate_generic_on_sum_reports_e0403() {
        let r = check_src(
            "type E<T, T> = | A(T) | B\n\
             fn main() { }",
        );
        assert!(error_codes(&r).contains(&"E0403"), "{:?}", r.diagnostics);
    }

    #[test]
    fn duplicate_generic_on_trait_method_reports_e0403() {
        let r = check_src(
            "trait Tr { fn m<U, U>(self, a: U) -> U }\n\
             fn main() { }",
        );
        assert!(error_codes(&r).contains(&"E0403"), "{:?}", r.diagnostics);
    }

    #[test]
    fn duplicate_generic_on_impl_block_reports_e0403() {
        let r = check_src(
            "record Box<T> { value: T }\n\
             impl<T, T> Box<T> { fn get(self) -> Int { 0 } }\n\
             fn main() { }",
        );
        assert!(error_codes(&r).contains(&"E0403"), "{:?}", r.diagnostics);
    }

    #[test]
    fn distinct_generics_are_accepted() {
        // Guard: distinct names on a function must not trip the duplicate check.
        let r = check_src(
            "fn f<T, U>(a: T, b: U) -> T { a }\n\
             fn main() { let x = f(1, \"s\")\n println(\"${x}\") }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    }

    #[test]
    fn generic_impl_method_for_nongeneric_trait_method_reports_e0072() {
        // A generic method in a trait impl whose trait method is non-generic is
        // an arity mismatch (the impl declares generics the trait does not), and
        // must not cascade a false E0001 for the method's own generic.
        let r = check_src(
            "trait Mapper { fn remap(self) -> Int }\n\
             record Box<T> { value: T }\n\
             impl<T> Mapper for Box<T> { fn remap<U>(self, x: U) -> Int { 0 } }\n\
             fn main() { }",
        );
        let codes = error_codes(&r);
        assert!(codes.contains(&"E0072"), "{:?}", r.diagnostics);
        assert!(
            !codes.contains(&"E0001"),
            "spurious E0001: {:?}",
            r.diagnostics
        );
    }

    #[test]
    fn impl_dropping_trait_method_bound_reports_e0072() {
        // The impl drops the trait method's declared generic bound
        // (`<U: Show>` -> `<U>`). Conformance must reject it (E0072); otherwise
        // the trait's bound is not a contract the call site can rely on and an
        // invalid call slips through to run unsoundly.
        let r = check_src(
            "trait Show { fn show(self) -> String }\n\
             trait Tagger { fn tag<U: Show>(self, x: U) -> String }\n\
             record Thing { v: Int }\n\
             impl Tagger for Thing { fn tag<U>(self, x: U) -> String { \"no-show\" } }\n\
             fn main() { }",
        );
        assert!(error_codes(&r).contains(&"E0072"), "{:?}", r.diagnostics);
    }

    #[test]
    fn impl_adding_undeclared_method_bound_reports_e0072() {
        // The impl adds a bound the trait does not declare (`<U>` -> `<U: Show>`).
        // Conformance must reject it up front, rather than letting a valid-per-
        // trait program fail later at monomorphization with a misattributed span.
        let r = check_src(
            "trait Show { fn show(self) -> String }\n\
             trait Tagger { fn tag<U>(self, x: U) -> String }\n\
             record Thing { v: Int }\n\
             impl Tagger for Thing { fn tag<U: Show>(self, x: U) -> String { \"tagged\" } }\n\
             fn main() { }",
        );
        assert!(error_codes(&r).contains(&"E0072"), "{:?}", r.diagnostics);
    }

    #[test]
    fn faithful_method_bound_impl_typechecks() {
        // An impl that repeats the trait method's bound exactly must NOT trip the
        // bound-conformance check.
        let r = check_src(
            "trait Show { fn show(self) -> String }\n\
             trait Tagger { fn tag<U: Show>(self, x: U) -> String }\n\
             record Thing { v: Int }\n\
             impl Tagger for Thing { fn tag<U: Show>(self, x: U) -> String { \"ok\" } }\n\
             fn main() { }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    }

    #[test]
    fn async_trait_method_declaration_reports_e0900() {
        // `async` on a trait-method *declaration* (no body) must be rejected the
        // same as on impl methods, default bodies, free fns, and externs.
        let r = check_src(
            "trait Foo { async fn bar(self) -> Int }\n\
             record Thing { v: Int }\n\
             impl Foo for Thing { fn bar(self) -> Int { self.v } }\n\
             fn main() { let t = Thing { v: 1 } }",
        );
        assert!(error_codes(&r).contains(&"E0900"), "{:?}", r.diagnostics);
    }

    #[test]
    fn panic_typechecks_as_never_in_match_arm() {
        // `panic` diverges, so the match's type comes from the other arm.
        let r = check_src(
            "fn get(o: Option<Int>) -> Int {\n\
                 match o { Some(v) => v, None => panic(\"none\") }\n\
             }\n\
             fn main() { println(\"${get(Some(3))}\") }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    }

    #[test]
    fn panic_rejects_non_string_argument() {
        let r = check_src("fn main() { panic(7) }");
        assert!(error_codes(&r).contains(&"E0010"), "{:?}", r.diagnostics);
    }

    #[test]
    fn async_default_trait_method_reports_e0900_once() {
        // A bodied (default) async trait method is still rejected — exactly once,
        // not duplicated by both the table and default-body passes.
        let r = check_src(
            "trait Foo { async fn bar(self) -> Int { 7 } }\n\
             record Thing { v: Int }\n\
             impl Foo for Thing { fn bar(self) -> Int { self.v } }\n\
             fn main() { let t = Thing { v: 1 } }",
        );
        let n = error_codes(&r).iter().filter(|c| **c == "E0900").count();
        assert_eq!(n, 1, "{:?}", r.diagnostics);
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

    // === Maranget exhaustiveness / reachability ===

    #[test]
    fn bool_match_true_and_false_is_exhaustive() {
        // Regression: `true | false` covers `Bool` with no wildcard needed.
        let r = check_src(
            "fn f(b: Bool) -> Int { match b { true => 1, false => 0 } }\n\
             fn main() { }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    }

    #[test]
    fn bool_match_missing_false_reports_e0020() {
        let r = check_src("fn f(b: Bool) -> Int { match b { true => 1 } }\nfn main() { }");
        assert!(error_codes(&r).contains(&"E0020"), "{:?}", r.diagnostics);
        // The witness names the uncovered value.
        assert!(
            r.diagnostics.iter().any(|d| d.message.contains("false")),
            "expected a `false` witness: {:?}",
            r.diagnostics
        );
    }

    #[test]
    fn int_match_without_catch_all_reports_e0020() {
        let r = check_src("fn f(n: Int) -> Int { match n { 0 => 1, 1 => 2 } }\nfn main() { }");
        assert!(error_codes(&r).contains(&"E0020"), "{:?}", r.diagnostics);
    }

    #[test]
    fn int_match_with_catch_all_ok() {
        let r = check_src("fn f(n: Int) -> Int { match n { 0 => 1, _ => 2 } }\nfn main() { }");
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    }

    #[test]
    fn non_exhaustive_witness_names_missing_variant() {
        let r = check_src(
            "type Opt = | Some(Int) | None\n\
             fn f(o: Opt) -> Int { match o { Some(x) => x } }\n\
             fn main() { }",
        );
        assert!(error_codes(&r).contains(&"E0020"), "{:?}", r.diagnostics);
        assert!(
            r.diagnostics.iter().any(|d| d.message.contains("None")),
            "expected a `None` witness: {:?}",
            r.diagnostics
        );
    }

    #[test]
    fn duplicate_variant_arm_is_unreachable_e0021() {
        // A repeated variant arm is now flagged even without a preceding
        // catch-all — usefulness detects it directly.
        let r = check_src(
            "type Opt = | Some(Int) | None\n\
             fn f(o: Opt) -> Int { match o { None => 0, Some(x) => x, None => 9 } }\n\
             fn main() { }",
        );
        assert!(
            r.diagnostics.iter().any(|d| d.code == "E0021"),
            "expected E0021 unreachable: {:?}",
            r.diagnostics
        );
    }

    #[test]
    fn arm_after_catch_all_is_unreachable_e0021() {
        let r = check_src("fn f(n: Int) -> Int { match n { _ => 0, 1 => 2 } }\nfn main() { }");
        assert!(
            r.diagnostics.iter().any(|d| d.code == "E0021"),
            "expected E0021 unreachable: {:?}",
            r.diagnostics
        );
    }

    #[test]
    fn empty_match_on_generic_param_reports_e0020() {
        // Regression (adversarial review): an empty match on a type parameter
        // was silently accepted (skipped as unanalyzable) and trapped at
        // runtime; it must be reported non-exhaustive.
        let r = check_src(
            "fn oops<T>(x: T) -> Int { match x { } }\n\
             fn main() { let r = oops(7)\n println(\"${r}\") }",
        );
        assert!(error_codes(&r).contains(&"E0020"), "{:?}", r.diagnostics);
    }

    #[test]
    fn foreign_variant_in_ident_pattern_reports_e0001() {
        // Regression (adversarial review): a bare identifier naming a variant
        // of a *different* sum type must be rejected, not silently bound as a
        // catch-all (which masked uncovered cases).
        let r = check_src(
            "type A = | Foo | Bar\n\
             type B = | Baz\n\
             fn f(a: A) -> Int { match a { Baz => 1, Foo => 2, Bar => 3 } }\n\
             fn main() { }",
        );
        assert!(error_codes(&r).contains(&"E0001"), "{:?}", r.diagnostics);
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
        // Exact wording, not just the code: the read and write paths share
        // `no_field_message` precisely so they cannot independently drift,
        // but until this assertion existed only the write path's
        // `assignment_to_unknown_field_reports_e0014` pinned any wording at
        // all — reverting the read path to inline its own message would have
        // passed every test in this file.
        let d = r
            .diagnostics
            .iter()
            .find(|d| d.code == "E0014")
            .expect("an E0014 was reported");
        assert_eq!(d.message, "no field `z` on record `Point`");
    }

    #[test]
    fn field_access_on_non_record_reports_e0014() {
        // The read-path mirror of `field_assignment_on_non_record_reports_e0014`:
        // a receiver with no fields at all gets `no_field_message`'s other
        // wording, not the "no field `x` on record `P`" phrasing that implies
        // a record exists. No read-path test exercised this case before.
        let r = check_src("fn main() { let n = 1\n let z = n.v }");
        let d = r
            .diagnostics
            .iter()
            .find(|d| d.code == "E0014")
            .expect("an E0014 was reported");
        assert_eq!(d.message, "cannot access field `v` on `Int`");
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
    fn repeat_array_typechecks_and_has_array_type() {
        let r =
            check_src("fn main() { let n = 3\n let a = [7; n]\n println(\"${a.len()} ${a[0]}\") }");
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
        // An empty diagnostic list alone is a weak assertion: `Ty::Error`
        // unifies with anything, so a broken arm could report nothing and
        // still have inferred a useless type. Pin what was actually built.
        let repeat = exprs_in(&r.module, "main")
            .into_iter()
            .find(|e| matches!(e.kind, hir::ExprKind::ArrayRepeat { .. }))
            .expect("the checker built an `ArrayRepeat`");
        assert!(
            matches!(&repeat.ty, Ty::Array(elem) if **elem == Ty::Int),
            "`[7; n]` should have type `[Int]`, got {:?}",
            repeat.ty
        );
    }

    #[test]
    fn repeat_array_elem_type_follows_the_init_expression() {
        // The element type comes from `init`, so a heap filler yields
        // `[String]` — and it must satisfy a `[String]` annotation.
        let r = check_src(
            "fn main() { let n = 2\n let a: [String] = [\"hi\"; n]\n println(\"${a[0]}\") }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
        let repeat = exprs_in(&r.module, "main")
            .into_iter()
            .find(|e| matches!(e.kind, hir::ExprKind::ArrayRepeat { .. }))
            .expect("the checker built an `ArrayRepeat`");
        assert!(
            matches!(&repeat.ty, Ty::Array(elem) if **elem == Ty::String),
            "`[\"hi\"; n]` should have type `[String]`, got {:?}",
            repeat.ty
        );
    }

    #[test]
    fn repeat_array_non_int_length_reports_e0010() {
        let r = check_src("fn main() { let a = [7; \"three\"]\n println(\"${a[0]}\") }");
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
    fn index_set_on_non_array_still_checks_the_rhs() {
        // The mirror of `field_assignment_with_unknown_field_still_checks_the_rhs`
        // on `check_index_set`'s "not an array" branch: it used to return
        // before checking the RHS, dropping an independent mistake there
        // alongside (correctly) rejecting the index-assign itself.
        let r = check_src("fn main() { let mut n = 1\n n[0] = undefined_fn() }");
        let codes = error_codes(&r);
        assert!(
            codes.contains(&"E0014"),
            "expected the not-an-array error: {:?}",
            r.diagnostics
        );
        assert!(
            codes.contains(&"E0001"),
            "expected `undefined_fn`'s own error to still surface: {:?}",
            r.diagnostics
        );
    }

    #[test]
    fn field_assignment_typechecks() {
        let r = check_src(
            "record P { v: Int }\n\
             fn main() { let mut p = P { v: 1 }\n p.v = 7\n println(\"${p.v}\") }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
        // Absence of diagnostics alone is not enough: `error_expr` is silent and
        // `Ty::Error` unifies with anything, so a store that never got built
        // would still look clean here. Pin the node, its field index, and its
        // unit type.
        let sets = field_sets_in(&r.module, "main");
        assert_eq!(sets.len(), 1, "exactly one FieldSet in `main`");
        let hir::ExprKind::FieldSet { index, value, .. } = &sets[0].kind else {
            unreachable!("filtered to FieldSet")
        };
        assert_eq!(*index, 0, "`v` is field 0, so the store offset is 8*0");
        assert_eq!(value.ty, Ty::Int, "the stored value keeps its `Int` type");
        assert_eq!(sets[0].ty, Ty::Unit, "a field store is unit-typed");
    }

    #[test]
    fn field_assignment_picks_the_declared_field_index() {
        // The store offset is `8 * index`, so writing the *second* field must
        // resolve to index 1. A helper that always returned 0 would still pass
        // a single-field test.
        let r = check_src(
            "record P { a: Int, b: String }\n\
             fn main() { let mut p = P { a: 1, b: \"x\" }\n p.b = \"y\"\n println(\"${p.b}\") }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
        let sets = field_sets_in(&r.module, "main");
        assert_eq!(sets.len(), 1, "exactly one FieldSet in `main`");
        let hir::ExprKind::FieldSet { index, value, .. } = &sets[0].kind else {
            unreachable!("filtered to FieldSet")
        };
        assert_eq!(*index, 1, "`b` is the second field");
        assert_eq!(value.ty, Ty::String);
    }

    #[test]
    fn field_assignment_on_generic_record_substitutes_the_field_type() {
        // The shared lookup substitutes the record's type arguments into the
        // field's declared type. This is the path `Vec<T>`/`Map<K, V>` depend
        // on, so assert the *resolved* type rather than just the absence of
        // errors — `Ty::Error` unifies with anything and would hide a failure.
        let r = check_src(
            "record Box<T> { value: T }\n\
             fn main() { let mut b = Box { value: 1 }\n b.value = 7\n println(\"${b.value}\") }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
        let sets = field_sets_in(&r.module, "main");
        assert_eq!(sets.len(), 1, "exactly one FieldSet in `main`");
        let hir::ExprKind::FieldSet { index, value, .. } = &sets[0].kind else {
            unreachable!("filtered to FieldSet")
        };
        assert_eq!(*index, 0);
        assert_eq!(
            value.ty,
            Ty::Int,
            "`T` was instantiated to `Int`, so the stored value is an `Int`"
        );
    }

    #[test]
    fn field_assignment_on_generic_record_mismatch_reports_e0010() {
        // The mirror of the above: substitution must also *reject* the wrong
        // type, rather than leaving the field type as an unconstrained `T`
        // that unifies with anything.
        let r = check_src(
            "record Box<T> { value: T }\n\
             fn main() { let mut b = Box { value: 1 }\n b.value = \"s\"\n println(\"${b.value}\") }",
        );
        assert!(error_codes(&r).contains(&"E0010"), "{:?}", r.diagnostics);
    }

    #[test]
    fn field_assignment_to_immutable_reports_e0060() {
        let r = check_src(
            "record P { v: Int }\n\
             fn main() { let p = P { v: 1 }\n p.v = 7\n println(\"${p.v}\") }",
        );
        assert!(error_codes(&r).contains(&"E0060"), "{:?}", r.diagnostics);
    }

    #[test]
    fn field_assignment_type_mismatch_reports_e0010() {
        let r = check_src(
            "record P { v: Int }\n\
             fn main() { let mut p = P { v: 1 }\n p.v = \"s\"\n println(\"${p.v}\") }",
        );
        assert!(error_codes(&r).contains(&"E0010"), "{:?}", r.diagnostics);
    }

    #[test]
    fn assignment_to_unknown_field_reports_e0014() {
        let r = check_src(
            "record P { v: Int }\n\
             fn main() { let mut p = P { v: 1 }\n p.nope = 7\n println(\"${p.v}\") }",
        );
        // Same wording as the read path's `unknown_field_access_reports_e0014`
        // shape: named as a record, not "on type `P`" — the two used to say
        // different things for the identical mistake.
        let d = r
            .diagnostics
            .iter()
            .find(|d| d.code == "E0014")
            .expect("an E0014 was reported");
        assert_eq!(d.message, "no field `nope` on record `P`");
    }

    #[test]
    fn field_assignment_on_non_record_reports_e0014() {
        // A mutable binding is not enough — the receiver has to *have* fields.
        let r = check_src("fn main() { let mut n = 1\n n.v = 2\n println(\"${n}\") }");
        // The write path used to collapse this into the same "no field `v` on
        // type `Int`" wording as the wrong-field-name case; it must now match
        // the read path's separate not-a-record message.
        let d = r
            .diagnostics
            .iter()
            .find(|d| d.code == "E0014")
            .expect("an E0014 was reported");
        assert_eq!(d.message, "cannot access field `v` on `Int`");
    }

    #[test]
    fn field_assignment_with_unknown_field_still_checks_the_rhs() {
        // `check_field_set`'s "no field" branch used to return before checking
        // the RHS at all, so an independent mistake there (`undefined_fn` is
        // not defined) was silently dropped — only the wrong-field-name E0014
        // was ever reported. The array path (`a[i] = undefined_fn()`) does not
        // drop it, and the field path must not either.
        let r = check_src(
            "record P { v: Int }\n\
             fn main() { let mut p = P { v: 1 }\n p.nope = undefined_fn() }",
        );
        let codes = error_codes(&r);
        assert!(
            codes.contains(&"E0014"),
            "expected the unknown-field error: {:?}",
            r.diagnostics
        );
        assert!(
            codes.contains(&"E0001"),
            "expected `undefined_fn`'s own error to still surface: {:?}",
            r.diagnostics
        );
    }

    #[test]
    fn field_assignment_through_broken_receiver_does_not_cascade() {
        // `p.nope` is already an error on the read path; the write path must not
        // add a second E0014 for the unknown field of an error-typed receiver.
        // This is the `Ty::Error` early return: the *target* diagnostic is
        // deliberately not repeated, since that would be the same mistake
        // reported twice. But the RHS is unrelated to the receiver and must
        // still be checked, per the invariant on `check_index_set` — so the
        // RHS below is `undefined_fn()`, not a literal: a literal produces no
        // diagnostic either way and cannot tell "checked but clean" apart
        // from "never checked".
        let r = check_src(
            "record P { v: Int }\n\
             fn main() { let mut p = P { v: 1 }\n p.nope.deeper = undefined_fn() }",
        );
        let codes = error_codes(&r);
        let n = codes.iter().filter(|c| **c == "E0014").count();
        assert_eq!(n, 1, "expected exactly one E0014, got {:?}", r.diagnostics);
        assert!(
            codes.contains(&"E0001"),
            "expected `undefined_fn`'s own error to still surface: {:?}",
            r.diagnostics
        );
    }

    #[test]
    fn field_assignment_on_temporary_reports_e0060() {
        // A call result has no assignable root, so mutating its field is
        // meaningless — rejected at the root exactly as `make()[0] = v` is.
        let r = check_src(
            "record P { v: Int }\n\
             fn make() -> P { P { v: 1 } }\n\
             fn main() { make().v = 7 }",
        );
        assert!(error_codes(&r).contains(&"E0060"), "{:?}", r.diagnostics);
    }

    #[test]
    fn field_assignment_through_immutable_nested_record_reports_e0060() {
        // `outer.inner.v = 7` roots at an immutable local through two `Field`
        // projections; `place_root` walks the whole chain.
        let r = check_src(
            "record Inner { v: Int }\n\
             record Outer { inner: Inner }\n\
             fn main() { let o = Outer { inner: Inner { v: 1 } }\n o.inner.v = 7 }",
        );
        assert!(error_codes(&r).contains(&"E0060"), "{:?}", r.diagnostics);
    }

    #[test]
    fn field_assignment_through_mutable_nested_record_ok() {
        // The mirror of the above: a `mut` root makes the reachable record
        // storage assignable, however deep the chain.
        let r = check_src(
            "record Inner { v: Int }\n\
             record Outer { inner: Inner }\n\
             fn main() {\n\
                 let mut o = Outer { inner: Inner { v: 1 } }\n\
                 o.inner.v = 7\n\
                 println(\"${o.inner.v}\")\n\
             }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
        assert_eq!(field_sets_in(&r.module, "main").len(), 1);
    }

    #[test]
    fn compound_assignment_to_field_reports_e0900() {
        // `+=` on a field would have to read-modify-write; not supported yet,
        // mirroring compound assignment to an array element.
        let r = check_src(
            "record P { v: Int }\n\
             fn main() { let mut p = P { v: 1 }\n p.v += 1\n println(\"${p.v}\") }",
        );
        assert!(error_codes(&r).contains(&"E0900"), "{:?}", r.diagnostics);
    }

    #[test]
    fn compound_assignment_to_field_still_checks_the_rhs() {
        // This branch returns before doing anything else at all — no
        // mutability check, no receiver check — so it used to drop an
        // independent RHS mistake along with (correctly) rejecting the
        // compound form itself.
        let r = check_src(
            "record P { v: Int }\n\
             fn main() { let mut p = P { v: 1 }\n p.v += undefined_fn() }",
        );
        let codes = error_codes(&r);
        assert!(codes.contains(&"E0900"), "{:?}", r.diagnostics);
        assert!(
            codes.contains(&"E0001"),
            "expected `undefined_fn`'s own error to still surface: {:?}",
            r.diagnostics
        );
    }

    #[test]
    fn compound_assignment_to_array_element_still_checks_the_rhs() {
        // The array-element analogue of
        // `compound_assignment_to_field_still_checks_the_rhs`.
        let r = check_src("fn main() { let mut xs = [1, 2]\n xs[0] += undefined_fn() }");
        let codes = error_codes(&r);
        assert!(codes.contains(&"E0900"), "{:?}", r.diagnostics);
        assert!(
            codes.contains(&"E0001"),
            "expected `undefined_fn`'s own error to still surface: {:?}",
            r.diagnostics
        );
    }

    #[test]
    fn mut_self_method_on_immutable_receiver_reports_e0060() {
        let r = check_src(
            "record P { v: Int }\n\
             impl P { fn bump(mut self) { self.v = self.v + 1 } }\n\
             fn main() { let p = P { v: 1 }\n p.bump()\n println(\"${p.v}\") }",
        );
        assert!(error_codes(&r).contains(&"E0060"), "{:?}", r.diagnostics);
    }

    #[test]
    fn mut_self_method_on_mutable_receiver_typechecks() {
        let r = check_src(
            "record P { v: Int }\n\
             impl P { fn bump(mut self) { self.v = self.v + 1 } }\n\
             fn main() { let mut p = P { v: 1 }\n p.bump()\n println(\"${p.v}\") }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    }

    #[test]
    fn plain_self_method_on_immutable_receiver_still_typechecks() {
        // Guard: only `mut self` demands a mutable receiver.
        let r = check_src(
            "record P { v: Int }\n\
             impl P { fn get(self) -> Int { self.v } }\n\
             fn main() { let p = P { v: 1 }\n println(\"${p.get()}\") }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    }

    #[test]
    fn mut_self_is_a_mutable_root_inside_the_method_body() {
        // The receiver rule is only useful if `mut self` also *permits* the
        // mutation it advertises: `place_root` must classify `self` as
        // `Mutable` inside the body. Asserting the compiled `FieldSet` rather
        // than the absence of diagnostics, because an empty diagnostic list
        // would also hold if the body had silently not been compiled at all.
        let r = check_src(
            "record P { v: Int }\n\
             impl P { fn bump(mut self) { self.v = self.v + 1 } }\n\
             fn main() { let mut p = P { v: 1 }\n p.bump()\n println(\"${p.v}\") }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
        assert_eq!(
            // An inherent method's `hir::Function` carries the resolver's
            // mangled `Type.method` name.
            field_sets_in(&r.module, "P.bump").len(),
            1,
            "`mut self` body compiled its store"
        );
    }

    #[test]
    fn plain_self_method_assigning_a_field_reports_e0060() {
        // The mirror of the above, and what makes the `mut` in `mut self` load-
        // bearing rather than decorative: a plain `self` receiver is an
        // immutable binding like any other, so its fields are read-only.
        let r = check_src(
            "record P { v: Int }\n\
             impl P { fn bump(self) { self.v = self.v + 1 } }\n\
             fn main() { let p = P { v: 1 }\n p.bump()\n println(\"${p.v}\") }",
        );
        assert!(error_codes(&r).contains(&"E0060"), "{:?}", r.diagnostics);
    }

    #[test]
    fn mut_self_method_called_on_a_field_of_mut_self_typechecks() {
        // The shape `std/collections`' `Set` uses: `self.map.insert(…)` from a
        // `mut self` method. `place_root` walks the field chain to `self`, so a
        // `mut self` root makes the nested receiver mutable too.
        let r = check_src(
            "record Inner { v: Int }\n\
             record Outer { inner: Inner }\n\
             impl Inner { fn bump(mut self) { self.v = self.v + 1 } }\n\
             impl Outer { fn bump_inner(mut self) { self.inner.bump() } }\n\
             fn main() {\n\
                 let mut o = Outer { inner: Inner { v: 1 } }\n\
                 o.bump_inner()\n\
                 println(\"${o.inner.v}\")\n\
             }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    }

    #[test]
    fn mut_self_method_called_on_field_of_immutable_root_reports_e0060() {
        // The mirror: the receiver is a field projection, so the whole chain
        // must be walked to the root binding — a single-segment check would
        // accept `o.inner.bump()` on an immutable `o`.
        let r = check_src(
            "record Inner { v: Int }\n\
             record Outer { inner: Inner }\n\
             impl Inner { fn bump(mut self) { self.v = self.v + 1 } }\n\
             fn main() {\n\
                 let o = Outer { inner: Inner { v: 1 } }\n\
                 o.inner.bump()\n\
             }",
        );
        assert!(error_codes(&r).contains(&"E0060"), "{:?}", r.diagnostics);
    }

    #[test]
    fn mut_self_method_on_temporary_reports_e0060() {
        // A call result has no assignable root, so mutating it is meaningless —
        // rejected exactly as `make().v = 7` is.
        let r = check_src(
            "record P { v: Int }\n\
             impl P { fn bump(mut self) { self.v = self.v + 1 } }\n\
             fn make() -> P { P { v: 1 } }\n\
             fn main() { make().bump() }",
        );
        assert!(error_codes(&r).contains(&"E0060"), "{:?}", r.diagnostics);
    }

    #[test]
    fn mut_self_method_called_from_a_plain_self_method_reports_e0060() {
        // The mistake a std author actually makes: an outer method forgets its
        // own `mut` and then delegates to a mutator. `self` is a parameter like
        // any other, so a plain `self` receiver is an immutable root and the
        // rule catches it at the inner call — not later, at the store.
        let r = check_src(
            "record Inner { v: Int }\n\
             record Outer { inner: Inner }\n\
             impl Inner { fn bump(mut self) { self.v = self.v + 1 } }\n\
             impl Outer { fn bump_inner(self) { self.inner.bump() } }\n\
             fn main() {\n\
                 let mut o = Outer { inner: Inner { v: 1 } }\n\
                 o.bump_inner()\n\
             }",
        );
        assert!(error_codes(&r).contains(&"E0060"), "{:?}", r.diagnostics);
    }

    #[test]
    fn misplaced_mut_self_receiver_is_still_a_mutator() {
        // `mut_self` must use the same `.any(…)` scan as `has_self`, because
        // `method_sig_parts` strips a `self` at *any* position and the parser
        // accepts a misplaced receiver. A `params[0]`-shaped predicate would
        // classify this method as a non-mutator while the signature machinery
        // still treated it as having a receiver — the two disagreeing about the
        // same parameter. Everything else about this shape is pre-existing
        // garbage (`self` ends up typed `Int`), so only the E0060 is asserted.
        let r = check_src(
            "record P { v: Int }\n\
             impl P { fn odd(x: Int, mut self) -> Int { x } }\n\
             fn main() { let p = P { v: 1 }\n println(\"${p.odd(1)}\") }",
        );
        assert!(error_codes(&r).contains(&"E0060"), "{:?}", r.diagnostics);
    }

    #[test]
    fn mut_self_method_on_immutable_receiver_suggests_let_mut() {
        // The note is the actionable half of the diagnostic; pin it so a
        // refactor cannot quietly drop it (`check_field_set` carries the same).
        // The message names the callee by its impl-qualified `Def` name, the
        // spelling `arity_errors_name_the_callee_uniformly` already pins for the
        // other inherent-method diagnostics.
        let r = check_src(
            "record P { v: Int }\n\
             impl P { fn bump(mut self) { self.v = self.v + 1 } }\n\
             fn main() { let p = P { v: 1 }\n p.bump()\n println(\"${p.v}\") }",
        );
        let d = r
            .diagnostics
            .iter()
            .find(|d| d.code == "E0060")
            .expect("an E0060 was reported");
        assert_eq!(
            d.message,
            "`P.bump` mutates its receiver, but `p` is immutable"
        );
        assert!(
            d.notes.iter().any(|n| n.contains("let mut p")),
            "{:?}",
            d.notes
        );
    }

    #[test]
    fn immutable_self_root_is_advised_to_use_mut_self_not_let_mut_self() {
        // All three mutation forms route through `require_mutable_place`, and
        // when the immutable root is a method's own receiver the uniform
        // ``declare it as `let mut {name}` `` note would read `let mut self` —
        // which Nova cannot parse, so the advice would be unfollowable. A
        // receiver's mutability is declared in the signature instead.
        //
        // `Set::insert` delegating to `Map::insert` is exactly the receiver
        // shape here, so this is the note a std author is most likely to be
        // handed.
        let r = check_src(
            "record Inner { v: Int }\n\
             impl Inner { fn bump(mut self) { self.v = self.v + 1 } }\n\
             record Outer { inner: Inner, data: [Int] }\n\
             impl Outer {\n\
                 fn call_mutator(self) { self.inner.bump() }\n\
                 fn set_elem(self) { self.data[0] = 1 }\n\
                 fn set_field(self) { self.inner.v = 1 }\n\
             }\n\
             fn main() { }",
        );
        let notes: Vec<&str> = r
            .diagnostics
            .iter()
            .filter(|d| d.code == "E0060")
            .flat_map(|d| d.notes.iter().map(|n| n.as_str()))
            .collect();
        // One per mutation form: the `mut self` call, the element store, and the
        // field store.
        assert_eq!(notes.len(), 3, "{:?}", r.diagnostics);
        for n in &notes {
            assert_eq!(
                *n, "declare the enclosing method's receiver as `mut self` to allow mutation",
                "{notes:?}"
            );
        }
        // The primary messages keep their per-form wording; only the note
        // changes for a `self` root.
        let messages: Vec<&str> = r
            .diagnostics
            .iter()
            .filter(|d| d.code == "E0060")
            .map(|d| d.message.as_str())
            .collect();
        assert!(
            messages.contains(&"`Inner.bump` mutates its receiver, but `self` is immutable"),
            "{messages:?}"
        );
        assert!(
            messages.contains(&"cannot assign to an element of immutable `self`"),
            "{messages:?}"
        );
        assert!(
            messages.contains(&"cannot assign to a field of immutable `self`"),
            "{messages:?}"
        );
    }

    /// **Documents a known gap, not a desired behaviour.** ADR 0005 §1
    /// ("Consequences" / "Migration path") records that the mutable-receiver
    /// rule covers inherent methods only: `check_method_call`'s
    /// `MethodRes::Trait` arm dispatches straight to `emit_trait_call` with no
    /// call to `check_mutable_receiver` at all, because a generic receiver's
    /// trait dispatch has no single impl method whose `mut self` could be read
    /// off — closing it needs a `mut_self` flag on `hir::TraitMethod` plus an
    /// impl/trait conformance check, neither of which exists yet.
    ///
    /// So a trait method declaring `mut self`, called through an *immutable*
    /// receiver, typechecks clean today — the same operation on an *inherent*
    /// method is `mut_self_method_on_immutable_receiver_reports_e0060` a few
    /// tests up, and reports E0060. This test pins today's accepted behaviour
    /// the same way `float_has_no_hash_impl` pins a deliberate absence: this
    /// exact six-line program's clean acceptance is now a recorded fact, so
    /// half-closing the gap (at least for this shape of program) makes this
    /// test fail and forces a deliberate decision, with reference to ADR
    /// 0005 §1's Migration path, rather than a silent pass. It does not, and
    /// cannot, reach further than the program it compiles — a `mut self`
    /// trait method added elsewhere, e.g. to `std/`, is not covered by this
    /// test and would typecheck clean with no diagnostic anywhere, exactly
    /// because this is the gap it documents.
    #[test]
    fn trait_method_mut_self_is_not_enforced_on_immutable_receiver_known_gap() {
        let r = check_src(
            "trait Bump { fn bump(mut self) }\n\
             record P { v: Int }\n\
             impl Bump for P { fn bump(mut self) { self.v = self.v + 1 } }\n\
             fn main() { let p = P { v: 1 }\n p.bump()\n println(\"${p.v}\") }",
        );
        assert!(
            r.diagnostics.is_empty(),
            "known gap (ADR 0005 §1): trait-dispatched `mut self` methods are \
             not yet checked against the receiver's mutability, so this is \
             expected to compile clean today: {:?}",
            r.diagnostics
        );
    }

    #[test]
    fn assignment_to_a_non_place_target_reports_e0900() {
        // `arr[i] = v` and `rec.f = v` are both handled before this fallback, so
        // its message must name record fields among the supported forms — this
        // is the wording that went stale when field assignment landed.
        let r = check_src("fn f() -> Int { 1 }\nfn main() { f() = 2 }");
        let d = r
            .diagnostics
            .iter()
            .find(|d| d.code == "E0900")
            .expect("an E0900 was reported");
        assert_eq!(
            d.message,
            "assignments to anything but a local variable, array element, or record field \
             are not supported yet"
        );
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
    fn where_clause_on_trait_impl_is_accepted() {
        // A `where` clause on a trait impl is an out-of-line bound: this is the
        // conditional impl `impl<T: Tag> Tag for Box<T>` and must type-check.
        let r = check_src(
            "record Box<T> { value: T }\n\
             trait Tag { fn tag(self) -> String }\n\
             impl<T> Tag for Box<T> where T: Tag { fn tag(self) -> String { \"b\" } }\n\
             fn main() { }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    }

    #[test]
    fn where_clause_on_function_enforces_bound() {
        // `where T: Show` is accepted here (Int: Show) and equivalent to the
        // inline bound `<T: Show>`.
        let r = check_src(
            "trait Show { fn show(self) -> String }\n\
             impl Show for Int { fn show(self) -> String { \"i\" } }\n\
             fn label<T>(x: T) -> String where T: Show { x.show() }\n\
             fn main() { println(label(1)) }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    }

    #[test]
    fn where_clause_on_concrete_type_reports_e0900() {
        // A `where` clause may only constrain a type parameter.
        let r = check_src(
            "trait Show { fn show(self) -> String }\n\
             fn f<T>(x: T) -> Int where Int: Show { 0 }\n\
             fn main() { }",
        );
        assert!(error_codes(&r).contains(&"E0900"), "{:?}", r.diagnostics);
    }

    #[test]
    fn where_clause_on_trait_method_reports_e0900() {
        let r = check_src(
            "trait Foo { fn f(self) -> Int where Self: Foo }\n\
             fn main() { }",
        );
        assert!(error_codes(&r).contains(&"E0900"), "{:?}", r.diagnostics);
    }

    #[test]
    fn record_type_param_bound_reports_e0900() {
        // `record Keyed<K: Hash2, V>` parses, but nothing honours the bound:
        // `hir::RecordType` has no `bounds` field and monomorphization only
        // discharges *function* bounds, so this used to compile and run with
        // `NoHash` — a bound that means nothing. Rejected outright, exactly as
        // `trait B where Self: A` is, rather than left reading as meaningful.
        let r = check_src(
            "trait Hash2 { fn h(self) -> Int }\n\
             record Keyed<K: Hash2, V> { k: K, v: V }\n\
             record NoHash { n: Int }\n\
             fn main() { let x = Keyed { k: NoHash { n: 1 }, v: 2 }\n println(\"${x.v}\") }",
        );
        assert!(error_codes(&r).contains(&"E0900"), "{:?}", r.diagnostics);
        // Exactly one, on the one bounded parameter — not one per parameter.
        assert_eq!(
            error_codes(&r).iter().filter(|c| **c == "E0900").count(),
            1,
            "{:?}",
            r.diagnostics
        );
    }

    #[test]
    fn sum_type_param_bound_reports_e0900() {
        // The same hole in the sum-type spelling: `hir::SumType` has no
        // `bounds` field either, so `type Wrap<T: Hash2>` used to accept a
        // payload with no `Hash2` impl.
        let r = check_src(
            "trait Hash2 { fn h(self) -> Int }\n\
             type Wrap<T: Hash2> = | Wrapped(T) | Empty\n\
             record NoHash { n: Int }\n\
             fn main() { let x = Wrapped(NoHash { n: 1 })\n match x { Wrapped(_) => println(\"w\"), Empty => println(\"e\") } }",
        );
        assert!(error_codes(&r).contains(&"E0900"), "{:?}", r.diagnostics);
    }

    #[test]
    fn record_type_param_bound_e0900_reports_every_bounded_param() {
        // One diagnostic per bounded parameter, so a multi-parameter record
        // does not hide the second offender behind the first.
        let r = check_src(
            "trait A { fn a(self) -> Int }\n\
             trait B { fn b(self) -> Int }\n\
             record Two<K: A, V: B> { k: K, v: V }\n\
             fn main() { }",
        );
        assert_eq!(
            error_codes(&r).iter().filter(|c| **c == "E0900").count(),
            2,
            "{:?}",
            r.diagnostics
        );
    }

    #[test]
    fn bounds_on_supported_positions_do_not_report_e0900() {
        // The E0900 above must fire *only* for a bound on a record's or sum
        // type's own parameter. Bounds on functions, impl blocks, generic trait
        // methods and `where` clauses are all supported and are used throughout
        // `std/`, which every program compiles — a false positive here would
        // break the whole stdlib. An unbounded generic record/sum (how `std`
        // actually writes `Vec<T>` / `Map<K, V>`) must stay clean too.
        let r = check_src(
            "trait Show { fn show(self) -> String }\n\
             impl Show for Int { fn show(self) -> String { \"i\" } }\n\
             record Vec2<T> { a: T, b: T }\n\
             type Maybe<T> = | Just(T) | Nothing\n\
             impl<T: Show> Vec2<T> { fn first(self) -> String { self.a.show() } }\n\
             trait Conv { fn conv<U: Show>(self, u: U) -> String }\n\
             impl Conv for Int { fn conv<U: Show>(self, u: U) -> String { u.show() } }\n\
             fn label<T: Show>(x: T) -> String { x.show() }\n\
             fn label2<T>(x: T) -> String where T: Show { x.show() }\n\
             fn main() {\n\
                 println(label(1))\n\
                 println(label2(2))\n\
                 println(Vec2 { a: 1, b: 2 }.first())\n\
                 println(3.conv(4))\n\
                 let m = Just(5)\n\
                 match m { Just(v) => println(\"${v}\"), Nothing => println(\"n\") }\n\
             }",
        );
        assert!(
            !error_codes(&r).contains(&"E0900"),
            "false positive: {:?}",
            r.diagnostics
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    }

    #[test]
    fn redundant_same_trait_bounds_are_not_ambiguous() {
        // Naming a trait more than once is redundant, not ambiguous: it must not
        // read as two providers of the method (a false E0015). Covers inline+where
        // accumulation, a duplicated `where` bound, and a duplicated inline bound.
        for sig in [
            "fn label<T: Show>(x: T) -> String where T: Show { x.show() }",
            "fn label<T>(x: T) -> String where T: Show, T: Show { x.show() }",
            "fn label<T: Show + Show>(x: T) -> String { x.show() }",
        ] {
            let src = format!(
                "trait Show {{ fn show(self) -> String }}\n\
                 impl Show for Int {{ fn show(self) -> String {{ \"i\" }} }}\n\
                 {sig}\n\
                 fn main() {{ println(label(1)) }}"
            );
            let r = check_src(&src);
            assert!(r.diagnostics.is_empty(), "sig `{sig}`: {:?}", r.diagnostics);
        }
    }

    #[test]
    fn generic_sum_in_field_or_payload_typechecks() {
        // A generic sum used as a record field or a sum-variant payload must not
        // be mis-read as arity 0 (regression: spurious E0012). Covers the prelude
        // `Option`/`Result` (collected last) and a forward-referenced record.
        let r = check_src(
            "record Slot { tag: Option<Int> }\n\
             type Wrapper = | W(Result<Int, String>) | Empty\n\
             fn main() {\n\
                 let s = Slot { tag: Some(1) }\n\
                 match s.tag { Some(v) => println(\"${v}\"), None => println(\"n\") }\n\
             }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
        let r2 = check_src(
            "record Slot { b: Box<Int> }\n\
             record Box<T> { value: T }\n\
             fn main() { }",
        );
        assert!(r2.diagnostics.is_empty(), "{:?}", r2.diagnostics);
    }

    #[test]
    fn extern_scalar_signature_is_accepted() {
        let r = check_src(
            "extern \"C\" { fn sqrt(x: Float) -> Float\n fn llabs(x: Int) -> Int }\n\
             fn main() { let r = sqrt(2.0)\n println(\"${r}\") }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
        assert!(r.module.externs.iter().any(|e| e.symbol == "sqrt"));
    }

    #[test]
    fn extern_non_scalar_param_reports_e0900() {
        let r = check_src("extern \"C\" { fn puts(s: String) -> Int }\nfn main() { }");
        assert!(error_codes(&r).contains(&"E0900"), "{:?}", r.diagnostics);
    }

    #[test]
    fn extern_unsupported_abi_reports_e0900() {
        let r = check_src("extern \"stdcall\" { fn foo() -> Int }\nfn main() { }");
        assert!(error_codes(&r).contains(&"E0900"), "{:?}", r.diagnostics);
    }

    #[test]
    fn generic_extern_reports_e0900_without_cascade() {
        let r = check_src("extern \"C\" { fn id<T>(x: T) -> T }\nfn main() { }");
        let codes = error_codes(&r);
        assert!(codes.contains(&"E0900"), "{:?}", r.diagnostics);
        // The generic extern short-circuits, so no false "cannot find type `T`".
        assert!(!codes.contains(&"E0001"), "cascade: {:?}", r.diagnostics);
    }

    #[test]
    fn extern_reserved_symbol_reports_e0900() {
        // An extern may not shadow/alias a compiler-internal symbol (`nova_*`).
        let r = check_src("extern \"C\" { fn nova_rt_alloc(x: Int) -> Int }\nfn main() { }");
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

    #[test]
    fn record_literal_in_an_interpolation_reaches_both_interp_paths() {
        // Now that a hole's braces balance, a record literal can sit directly
        // in one. `check_interp` has two arms and this exercises both from that
        // position: `${R { … }.v}` is an `Int`, converted natively without
        // consulting any `fmt`, while `${R { … }}` is a nominal type and goes
        // through the `Display` bridge.
        let r = check_src(
            "record R { v: Int }\n\
             trait Display { fn fmt(self) -> String }\n\
             impl Display for R { fn fmt(self) -> String { \"r=${self.v}\" } }\n\
             fn main() { println(\"${R { v: 1 }.v} ${R { v: 2 }}\") }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
        // Both pieces really are the two different lowerings, not one path
        // twice: the native `ToStr` for the `Int` field, and a call for the
        // record. (Plus the `StrLit` for the literal space between them.)
        let main = r
            .module
            .functions
            .iter()
            .find(|f| f.name == "main")
            .expect("`main` compiled to a hir::Function");
        let concat = find_str_concat(&main.body).expect("main interpolates");
        let kinds: Vec<&str> = concat
            .iter()
            .map(|p| match &p.kind {
                hir::ExprKind::ToStr(_) => "ToStr",
                hir::ExprKind::StrLit(_) => "StrLit",
                hir::ExprKind::TraitCall { .. } => "TraitCall",
                _ => "other",
            })
            .collect();
        assert_eq!(
            kinds,
            vec!["ToStr", "StrLit", "TraitCall"],
            "the Int field converts natively and the record goes through the \
             Display bridge"
        );
        // The record really is built inside the hole, not read from a local.
        let record_arg = match &concat[2].kind {
            hir::ExprKind::TraitCall {
                receiver: Some(r), ..
            } => r,
            other => panic!("expected a Display TraitCall with a receiver, got {other:?}"),
        };
        // A record literal lowers to a block that binds its field values to
        // temporaries and then builds the record.
        let builds_record = match &record_arg.kind {
            hir::ExprKind::MakeRecord { .. } => true,
            hir::ExprKind::Block {
                trailing: Some(t), ..
            } => matches!(t.kind, hir::ExprKind::MakeRecord { .. }),
            _ => false,
        };
        assert!(
            builds_record,
            "receiver should be the record literal itself, got {:?}",
            record_arg.kind
        );
    }

    /// The first `StrConcat` inside `e`, searched through the expression forms
    /// an interpolation can be nested in here (a block, and `println`'s args).
    fn find_str_concat(e: &hir::Expr) -> Option<&Vec<hir::Expr>> {
        match &e.kind {
            hir::ExprKind::StrConcat(pieces) => Some(pieces),
            hir::ExprKind::Call { args, .. } => args.iter().find_map(find_str_concat),
            hir::ExprKind::Block { stmts, trailing } => stmts
                .iter()
                .chain(trailing.as_deref())
                .find_map(find_str_concat),
            _ => None,
        }
    }

    #[test]
    fn selfless_method_params_are_not_shifted() {
        // `x: Int` must stay Int; a wrongly prepended `self` shifted it to `P`.
        let r = check_src(
            "record P { v: Int }\n\
             impl P { fn make(x: Int) -> Int { x + 1 } }\n\
             fn main() { println(\"ok\") }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    }

    #[test]
    fn selfless_method_param_types_are_declared_types() {
        // `selfless_method_params_are_not_shifted` above only proves there is
        // no *diagnostic* for a self-less method's parameters — that covers
        // the false-error symptom, not the silent-miscompile one. Pre-fix,
        // `impl P { fn f(a: Int, b: Int) -> Int { b } }` type-checked as `ok`
        // with `a` silently bound to the self type `P` instead of `Int`: the
        // positional zip in `check_fn_body` paired `a` with the wrongly
        // prepended self type and `b` with the first declared type (`Int`),
        // which happened to be the correct type for `b` — no diagnostic ever
        // compared `a` against anything, so nothing caught it. This pins the
        // actual bound *types*, not just the absence of errors.
        let r = check_src(
            "record P { v: Int }\n\
             impl P { fn f(a: Int, b: Int) -> Int { b } }\n\
             fn main() { println(\"ok\") }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);

        let f = r
            .module
            .functions
            .iter()
            .find(|func| func.name == "P.f")
            .expect("impl method `f` compiled to a hir::Function");
        // No phantom leading `self` parameter: exactly the two declared params.
        assert_eq!(f.params, 2, "expected 2 declared params, none prepended");
        assert_eq!(f.locals.len(), 2, "no extra locals beyond the two params");
        // And each keeps its *declared* type rather than being shifted by one.
        assert_eq!(f.locals[0].ty, Ty::Int, "`a` should be Int, not `P`");
        assert_eq!(f.locals[1].ty, Ty::Int, "`b` should be Int");
    }

    #[test]
    fn selfless_trait_impl_method_checks_conformance_without_panicking() {
        // A trait method declared without `self` leaves the impl signature with
        // no receiver to skip; conformance must compare the declared parameters
        // as-is rather than slicing past the end of an empty list.
        let r = check_src(
            "record P { v: Int }\n\
             trait Zero { fn zero() -> Int }\n\
             impl Zero for P { fn zero() -> Int { 0 } }\n\
             fn main() { println(\"ok\") }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    }

    #[test]
    fn selfless_method_called_on_instance_reports_e0014() {
        // Must be a clean diagnostic, never a codegen ICE.
        let r = check_src(
            "record P { v: Int }\n\
             impl P { fn make() -> P { P { v: 7 } } }\n\
             fn main() { let p = P { v: 0 }\n let q = p.make()\n println(\"${q.v}\") }",
        );
        assert!(error_codes(&r).contains(&"E0014"), "{:?}", r.diagnostics);
    }

    #[test]
    fn associated_function_call_on_concrete_type() {
        let r = check_src(
            "record P { v: Int }\n\
             impl P { fn new() -> P { P { v: 7 } } }\n\
             fn main() { let p = P::new()\n println(\"${p.v}\") }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    }

    #[test]
    fn associated_function_with_args() {
        let r = check_src(
            "record P { v: Int }\n\
             impl P { fn of(x: Int) -> P { P { v: x } } }\n\
             fn main() { let p = P::of(5)\n println(\"${p.v}\") }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    }

    #[test]
    fn associated_function_wrong_arity_reports_e0016() {
        let r = check_src(
            "record P { v: Int }\n\
             impl P { fn of(x: Int) -> P { P { v: x } } }\n\
             fn main() { let p = P::of()\n println(\"${p.v}\") }",
        );
        assert!(error_codes(&r).contains(&"E0016"), "{:?}", r.diagnostics);
    }

    #[test]
    fn unknown_associated_function_still_reports_e0001() {
        let r = check_src(
            "record P { v: Int }\n\
             fn main() { let p = P::nope()\n println(\"x\") }",
        );
        assert!(error_codes(&r).contains(&"E0001"), "{:?}", r.diagnostics);
    }

    #[test]
    fn generic_type_qualifier_with_unresolved_args_reports_e0011() {
        // `Box::zero()` where only a concrete `impl Zero for Box<Int>` exists:
        // `find_trait_assoc_fns` cannot select it because the qualifier's type
        // argument (from `qualifier_self_ty`) is a fresh, still-unresolved
        // inference variable, and `ImplInfo::match_args` compares structurally
        // — a documented, deliberate limitation (see `find_trait_assoc_fns`).
        // Before this fix this fell through silently to `check_path`, which
        // reported "no variant `zero` on type `Box`" — a diagnostic that blames
        // the wrong feature, since `zero` is a real associated function on a
        // real impl, just not one this call can select.
        let r = check_src(
            "record Box<T> { value: T }\n\
             trait Zero { fn zero() -> Self }\n\
             impl Zero for Box<Int> { fn zero() -> Box<Int> { Box { value: 0 } } }\n\
             fn main() { let b: Box<Int> = Box::zero()\n println(\"${b.value}\") }",
        );
        assert!(error_codes(&r).contains(&"E0011"), "{:?}", r.diagnostics);
        assert!(
            r.diagnostics
                .iter()
                .any(|d| d.message.contains("type arguments could not be determined")),
            "expected the message to name the real reason: {:?}",
            r.diagnostics
        );
        assert!(
            !r.diagnostics.iter().any(|d| d.message.contains("variant")),
            "should not blame a missing variant: {:?}",
            r.diagnostics
        );
    }

    #[test]
    fn qualified_variant_call_with_args_still_typechecks() {
        // The single largest regression risk of the associated-function-call
        // feature: `Type::Variant(args)` and `Type::assoc_fn(args)` share the
        // same two-segment branch in `check_call`, and the variant lookup must
        // keep running — and returning early — before the associated-function
        // lookup is ever tried. No test anywhere exercised the *qualified call*
        // form with arguments before this: the existing exhaustiveness /
        // reachability tests only ever construct variants via their bare,
        // unqualified names (`Circle(r)`, `Some(x)`, ...), never `Type::Variant(args)`.
        let r = check_src(
            "type Shape = | Circle(Int) | Square(Int)\n\
             fn area(s: Shape) -> Int { match s { Circle(r) => r * r, Square(w) => w * w } }\n\
             fn main() { println(\"${area(Shape::Circle(5))}\") }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    }

    #[test]
    fn associated_function_on_generic_impl_resolves_type_arg() {
        // `impl<T> Box<T>` gives an associated function no receiver, so `T`
        // cannot be recovered the way `emit_inherent_call` recovers it (by
        // unifying the impl's generics against the receiver's type);
        // `emit_assoc_call` must instead recover `T` purely from the call's
        // own argument. Deliberately no `let` annotation on the call site:
        // `check_block`'s `let` handling overwrites the initializer's
        // inferred type with the annotation's (`value.ty = annot_ty`)
        // whenever unification succeeds — and `Ty::Error` unifies with
        // anything — so an annotated call site would silently paper over a
        // broken `type_args` substitution rather than exercise it. For the
        // same reason, asserting only `diagnostics.is_empty()` would not
        // catch that class of bug either, so this also pins the actual
        // resolved type of `b`.
        let r = check_src(
            "record Box<T> { value: T }\n\
             impl<T> Box<T> { fn make(v: T) -> Box<T> { Box { value: v } } }\n\
             fn main() {\n\
                 let b = Box::make(5)\n\
                 println(\"${b.value}\")\n\
             }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
        let box_def = r
            .module
            .records
            .iter()
            .find(|rt| rt.name == "Box")
            .expect("record Box exists")
            .def_id;
        let main_fn = r
            .module
            .functions
            .iter()
            .find(|f| f.name == "main")
            .expect("main compiled to a hir::Function");
        let b_ty = &main_fn
            .locals
            .iter()
            .find(|l| l.name == "b")
            .expect("`b` is a local in main")
            .ty;
        assert_eq!(
            *b_ty,
            Ty::Record {
                def_id: box_def,
                args: vec![Ty::Int]
            },
            "expected `b: Box<Int>`, got {b_ty:?}"
        );
    }

    #[test]
    fn ambiguous_inherent_associated_function_reports_e0015() {
        // Two *disjoint concrete* inherent impls of one generic type both
        // declare `tag`. `check_impl_coherence` deliberately allows the pair
        // (`self_types_overlap(Box<Int>, Box<Bool>)` is false, so no E0074),
        // and `find_assoc_fns` cannot tell them apart either — the qualifier
        // `Box` carries no type argument, so neither impl can be selected.
        // Before this guard, `Box::tag()` silently took whichever impl was
        // declared first: swapping the two `impl` lines below changed the
        // program's output from `1` to `2` with no diagnostic at all. That is
        // exactly the "dispatch would depend on impl declaration order"
        // failure `check_impl_coherence`'s doc comment names.
        let r = check_src(
            "record Box<T> { value: T }\n\
             impl Box<Int> { fn tag() -> Int { 1 } }\n\
             impl Box<Bool> { fn tag() -> Int { 2 } }\n\
             fn main() { println(\"${Box::tag()}\") }",
        );
        assert!(error_codes(&r).contains(&"E0015"), "{:?}", r.diagnostics);
        // And the pair really is coherent, so E0015 is the *only* thing
        // standing between this program and order-dependent dispatch.
        assert!(
            !error_codes(&r).contains(&"E0074"),
            "the two impls do not overlap, so coherence must not fire: {:?}",
            r.diagnostics
        );
        // The reverse declaration order must be rejected identically — the
        // whole point is that order stops mattering.
        let flipped = check_src(
            "record Box<T> { value: T }\n\
             impl Box<Bool> { fn tag() -> Int { 2 } }\n\
             impl Box<Int> { fn tag() -> Int { 1 } }\n\
             fn main() { println(\"${Box::tag()}\") }",
        );
        assert!(
            error_codes(&flipped).contains(&"E0015"),
            "{:?}",
            flipped.diagnostics
        );
    }

    #[test]
    fn single_concrete_inherent_associated_function_still_resolves() {
        // The permissive single-candidate case the E0015 guard above must not
        // regress: one `impl Box<Int>` and a bare `Box::tag()` qualifier whose
        // type argument is still an inference variable. Selection is by
        // nominal head alone, so this resolves; asserting the *resolved type*
        // of `t` rather than only `diagnostics.is_empty()` because `Ty::Error`
        // unifies with anything, which would make an emptiness-only assertion
        // silently vacuous.
        let r = check_src(
            "record Box<T> { value: T }\n\
             impl Box<Int> { fn tag() -> Int { 7 } }\n\
             fn main() {\n\
                 let t = Box::tag()\n\
                 println(\"${t}\")\n\
             }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
        let main_fn = r
            .module
            .functions
            .iter()
            .find(|f| f.name == "main")
            .expect("main compiled to a hir::Function");
        let t_ty = &main_fn
            .locals
            .iter()
            .find(|l| l.name == "t")
            .expect("`t` is a local in main")
            .ty;
        assert_eq!(*t_ty, Ty::Int, "expected `t: Int`, got {t_ty:?}");
    }

    #[test]
    fn trait_associated_function_on_concrete_type() {
        let r = check_src(
            "trait Zero { fn zero() -> Self }\n\
             impl Zero for Int { fn zero() -> Int { 0 } }\n\
             fn main() { println(\"${Int::zero()}\") }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    }

    #[test]
    fn trait_associated_function_through_bound() {
        let r = check_src(
            "trait Zero { fn zero() -> Self }\n\
             impl Zero for Int { fn zero() -> Int { 0 } }\n\
             fn make<T: Zero>() -> T { T::zero() }\n\
             fn main() { let n: Int = make()\n println(\"${n}\") }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    }

    #[test]
    fn impl_self_receiver_disagreeing_with_trait_reports_e0072() {
        // The trait declares an associated function; the impl gives it a
        // receiver. That is a signature mismatch, not a silent difference.
        let r = check_src(
            "trait Zero { fn zero() -> Self }\n\
             record R { v: Int }\n\
             impl Zero for R { fn zero(self) -> R { self } }\n\
             fn main() { }",
        );
        assert!(error_codes(&r).contains(&"E0072"), "{:?}", r.diagnostics);
    }

    #[test]
    fn impl_omitting_a_self_receiver_the_trait_declares_reports_e0072() {
        // The mirror of `impl_self_receiver_disagreeing_with_trait_reports_e0072`
        // and an independent bug: the trait declares `fn zero(self)`, the impl
        // omits the receiver. Before `has_self`, conformance compared the impl's
        // *declared* parameters (`selfless` ⇒ nothing to skip) against the
        // trait's (also none, `self` being implicit) and the return types, so
        // both agreed and the impl was accepted. A call then type-checked
        // against the trait's signature and lowered with a receiver argument
        // into an impl function of arity 0 — Cranelift "mismatched argument
        // count: got 1, expected 0", i.e. an ICE from `nova check`-clean source.
        let r = check_src(
            "record R { v: Int }\n\
             trait Zero { fn zero(self) -> Int }\n\
             impl Zero for R { fn zero() -> Int { 0 } }\n\
             fn main() { }",
        );
        assert!(error_codes(&r).contains(&"E0072"), "{:?}", r.diagnostics);
    }

    #[test]
    fn trait_associated_function_result_type_is_the_qualifier() {
        // `trait_associated_function_on_concrete_type` above asserts only that
        // no diagnostic is produced, which is a weak claim: `Ty::Error` unifies
        // with anything, so a botched `Self` substitution can be silent. Pin the
        // resolved type instead. Deliberately *no* `let` annotation — an
        // annotation would have `check_block` overwrite the initializer's
        // inferred type with the annotation's, hiding exactly the bug this tests.
        let r = check_src(
            "trait Zero { fn zero() -> Self }\n\
             impl Zero for Int { fn zero() -> Int { 0 } }\n\
             fn main() { let n = Int::zero()\n println(\"${n}\") }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
        let main_fn = r
            .module
            .functions
            .iter()
            .find(|f| f.name == "main")
            .expect("main compiled to a hir::Function");
        let n_ty = &main_fn
            .locals
            .iter()
            .find(|l| l.name == "n")
            .expect("`n` is a local in main")
            .ty;
        assert_eq!(*n_ty, Ty::Int, "`Int::zero()` should have type `Int`");
    }

    #[test]
    fn trait_associated_function_through_bound_has_param_self_ty() {
        // `trait_associated_function_through_bound` above needs its `let n: Int`
        // annotation (nothing else can determine `T`), which makes "no
        // diagnostics" a weak signal. Pin the shape of the emitted call instead:
        // inside `make`, `T::zero()` must be a receiver-less `TraitCall` whose
        // `Self` is the *generic parameter* `Param(0)` — that is what lets
        // monomorphization resolve it per instantiation — and whose result type
        // is that same parameter, not a stray inference variable or `Error`.
        let r = check_src(
            "trait Zero { fn zero() -> Self }\n\
             impl Zero for Int { fn zero() -> Int { 0 } }\n\
             fn make<T: Zero>() -> T { T::zero() }\n\
             fn main() { let n: Int = make()\n println(\"${n}\") }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
        let make_fn = r
            .module
            .functions
            .iter()
            .find(|f| f.name == "make")
            .expect("`make` compiled to a hir::Function");
        let call = child_exprs(&make_fn.body)
            .into_iter()
            .chain(std::iter::once(&make_fn.body))
            .find(|e| matches!(e.kind, hir::ExprKind::TraitCall { .. }))
            .expect("`make`'s body contains a TraitCall");
        assert_eq!(call.ty, Ty::Param(0), "`T::zero()` should have type `T`");
        let hir::ExprKind::TraitCall {
            self_ty, receiver, ..
        } = &call.kind
        else {
            unreachable!("filtered above")
        };
        assert_eq!(*self_ty, Ty::Param(0), "`Self` should be the parameter `T`");
        assert!(receiver.is_none(), "an associated function has no receiver");
    }

    #[test]
    fn trait_associated_function_with_default_body_on_concrete_type() {
        // A receiver-less trait method may carry a default body, which becomes a
        // `Self`-generic function. `collect_traits` must not prepend the `self`
        // receiver to that function's signature — the only site exercising the
        // `has_self` conditional in the default-body signature loop.
        let r = check_src(
            "trait Zero { fn zero() -> Int { 5 } }\n\
             impl Zero for Int { }\n\
             fn main() { println(\"${Int::zero()}\") }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    }

    #[test]
    fn trait_associated_function_substitutes_self_in_parameters() {
        // `Self` in a *parameter* must be substituted with the qualifier too,
        // not just in the return type: the expected type of `true` is `Int`.
        let r = check_src(
            "trait Merge { fn combine(a: Self, b: Self) -> Self }\n\
             impl Merge for Int { fn combine(a: Int, b: Int) -> Int { a + b } }\n\
             fn main() { println(\"${Int::combine(2, true)}\") }",
        );
        assert!(error_codes(&r).contains(&"E0010"), "{:?}", r.diagnostics);
    }

    #[test]
    fn ambiguous_trait_associated_function_reports_e0015() {
        // Two traits in scope provide `Int::zero`; picking one would make
        // dispatch depend on impl declaration order.
        let r = check_src(
            "trait A { fn zero() -> Int }\n\
             trait B { fn zero() -> Int }\n\
             impl A for Int { fn zero() -> Int { 1 } }\n\
             impl B for Int { fn zero() -> Int { 2 } }\n\
             fn main() { println(\"${Int::zero()}\") }",
        );
        assert!(error_codes(&r).contains(&"E0015"), "{:?}", r.diagnostics);
    }

    #[test]
    fn trait_associated_function_wrong_arity_reports_e0016() {
        let r = check_src(
            "trait Zero { fn zero() -> Int }\n\
             impl Zero for Int { fn zero() -> Int { 1 } }\n\
             fn main() { println(\"${Int::zero(9)}\") }",
        );
        assert!(error_codes(&r).contains(&"E0016"), "{:?}", r.diagnostics);
    }

    #[test]
    fn trait_call_substitution_puts_self_before_the_methods_own_generics() {
        // The binding invariant of `emit_trait_call`: the substitution is
        // `[Self] ++ method_type_args`, because a trait method's Param space is
        // flat — `Self` is Param(0) and the method's own generics follow at
        // Param(1..), which is also how `hir::TraitMethod::bounds` is indexed.
        // Get the order wrong and a call silently dispatches to a wrongly
        // specialized function.
        //
        // Both shapes are checked because they used to be two functions that had
        // to build this vector identically by hand. A swapped order is only
        // observable when the method has generics of its own (with `generics ==
        // 0` the two orders coincide), so it needs a `<U>` method and a `Self`
        // return: under a swap, `u`'s expected type becomes `Self` and the
        // `Bool` argument stops type-checking.
        for (src, shape) in [
            (
                "trait Tag { fn tag<U>(self, u: U) -> Self }\n\
                 impl Tag for Int { fn tag<U>(self, u: U) -> Int { 1 } }\n\
                 fn main() { let n = 5\n let m: Int = n.tag(true)\n println(\"${m}\") }",
                "receiver",
            ),
            (
                "trait Make { fn make<U>(u: U) -> Self }\n\
                 impl Make for Int { fn make<U>(u: U) -> Int { 1 } }\n\
                 fn main() { let m: Int = Int::make(true)\n println(\"${m}\") }",
                "qualifier",
            ),
        ] {
            let r = check_src(src);
            assert!(
                r.diagnostics.is_empty(),
                "{shape} dispatch: {:?}",
                r.diagnostics
            );
            let main_fn = r
                .module
                .functions
                .iter()
                .find(|f| f.name == "main")
                .expect("main compiled to a hir::Function");
            let m_ty = &main_fn
                .locals
                .iter()
                .find(|l| l.name == "m")
                .expect("`m` is a local in main")
                .ty;
            // `-> Self` must resolve through Param(0), i.e. to the qualifier /
            // receiver type, not to the method's own generic.
            assert_eq!(*m_ty, Ty::Int, "{shape} dispatch: `Self` return");
        }
    }

    #[test]
    fn every_trait_call_agrees_with_its_callees_has_self() {
        // `emit_trait_call` is the sole emitter of `TraitCall`, and it refuses to
        // build one whose receiver-ness disagrees with the trait method's
        // `has_self` — a mismatch in either direction lowers a `self` argument
        // into a callee with no slot for it, or omits one from a callee that has
        // it, both of which Cranelift rejects as "mismatched argument count".
        // The two guard arms are unreachable from source today (see the emitter's
        // comment), so instead of a test that cannot fail, assert the invariant
        // they protect over every call a real program produces: any future
        // emitter path that got this wrong would break here even if the guard
        // itself were deleted.
        //
        // `check_src` merges the implicit prelude, so `module.functions` also
        // carries all of std/core — every `Ord`/`Eq`/`Display` call in it is
        // covered for free, on top of the receiver and receiver-less trait calls
        // the source below adds.
        let r = check_src(
            "trait Zero { fn zero() -> Self }\n\
             trait Widen { fn widen(self, k: Int) -> Int }\n\
             impl Zero for Int { fn zero() -> Int { 0 } }\n\
             impl Widen for Int { fn widen(self, k: Int) -> Int { k } }\n\
             fn make<T: Zero>() -> T { T::zero() }\n\
             fn main() {\n\
                 let n: Int = make()\n\
                 let m: Int = Int::zero()\n\
                 println(\"${n.widen(1)}${m}\")\n\
             }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);

        fn walk<'e>(e: &'e hir::Expr, out: &mut Vec<&'e hir::Expr>) {
            out.push(e);
            for c in child_exprs(e) {
                walk(c, out);
            }
        }
        let mut seen_with_receiver = 0;
        let mut seen_without_receiver = 0;
        for f in &r.module.functions {
            let mut exprs = Vec::new();
            walk(&f.body, &mut exprs);
            for e in exprs {
                let hir::ExprKind::TraitCall {
                    trait_id,
                    method,
                    receiver,
                    ..
                } = &e.kind
                else {
                    continue;
                };
                let tm = r
                    .module
                    .traits
                    .iter()
                    .find(|t| t.def_id == *trait_id)
                    .and_then(|t| t.methods.get(*method as usize))
                    .expect("a TraitCall names an existing trait method");
                assert_eq!(
                    receiver.is_some(),
                    tm.has_self,
                    "`{}` (has_self = {}) got receiver.is_some() = {} in `{}`",
                    tm.name,
                    tm.has_self,
                    receiver.is_some(),
                    f.name
                );
                if receiver.is_some() {
                    seen_with_receiver += 1;
                } else {
                    seen_without_receiver += 1;
                }
            }
        }
        // Both flavors must actually be present, or the loop above proves nothing.
        assert!(seen_with_receiver > 0, "no receiver trait call was emitted");
        assert!(
            seen_without_receiver > 0,
            "no receiver-less trait call was emitted"
        );
    }

    #[test]
    fn arity_errors_name_the_callee_uniformly() {
        // E0016 is raised from five places — a free function (`check_call`), an
        // inherent method and an inherent associated function
        // (`emit_inherent_call` / `emit_assoc_call`), and a trait method called
        // both ways (`emit_trait_call`) — and the wording had drifted three ways
        // across them: one said "method `f` takes …", one "`f` takes …", and the
        // inherent-method one "method takes …" with no name at all. The same
        // mistake must read the same way whichever dispatch path found the
        // callee, so pin the shared phrasing rather than only the code. Two of
        // these sites had no arity test at all before this, which is how the
        // drift went unnoticed.
        //
        // What is unified is the phrasing, not the spelling of the name: an impl
        // method's `Def` name is impl-qualified (`P.of`), while a trait dispatch
        // site only ever has the trait's declared method name — `Self` may still
        // be a generic parameter there, so there is no impl to qualify with.
        // Each path therefore names the callee with the only name it has.
        let cases: [(&str, &str); 5] = [
            // Free function.
            (
                "fn f(a: Int) -> Int { a }\nfn main() { let x = f(1, 2) }",
                "`f` takes 1 argument(s) but 2 were supplied",
            ),
            // Inherent associated function (no receiver).
            (
                "record P { v: Int }\n\
                 impl P { fn of(x: Int) -> P { P { v: x } } }\n\
                 fn main() { let p = P::of()\n println(\"${p.v}\") }",
                "`P.of` takes 1 argument(s) but 0 were supplied",
            ),
            // Inherent method (receiver at sig.params[0], so the expected count
            // is one less than the signature's length).
            (
                "record P { v: Int }\n\
                 impl P { fn bump(self, k: Int) -> Int { k } }\n\
                 fn main() { let p = P { v: 1 }\n println(\"${p.bump()}\") }",
                "`P.bump` takes 1 argument(s) but 0 were supplied",
            ),
            // Trait method through a receiver.
            (
                "trait Widen { fn widen(self, k: Int) -> Int }\n\
                 impl Widen for Int { fn widen(self, k: Int) -> Int { k } }\n\
                 fn main() { let n = 5\n println(\"${n.widen()}\") }",
                "`widen` takes 1 argument(s) but 0 were supplied",
            ),
            // Trait associated function through a qualifier — the same emitter as
            // the case above, reached with no receiver.
            (
                "trait Zero { fn zero() -> Int }\n\
                 impl Zero for Int { fn zero() -> Int { 1 } }\n\
                 fn main() { println(\"${Int::zero(9)}\") }",
                "`zero` takes 0 argument(s) but 1 were supplied",
            ),
        ];
        for (src, expected) in cases {
            let r = check_src(src);
            let msgs: Vec<&str> = r
                .diagnostics
                .iter()
                .filter(|d| d.code == "E0016")
                .map(|d| d.message.as_str())
                .collect();
            assert_eq!(msgs, vec![expected], "for source:\n{src}");
        }
    }

    #[test]
    fn generic_param_associated_function_without_a_bound_reports_e0001() {
        // `T::zero()` where none of `T`'s bounds declares `zero`. Must be a
        // targeted diagnostic, not the misleading "module-qualified paths are
        // not supported yet" the pre-existing fall-through produced.
        let r = check_src(
            "trait Show { fn fmt(self) -> String }\n\
             fn make<T: Show>() -> T { T::zero() }\n\
             fn main() { println(\"hi\") }",
        );
        assert!(error_codes(&r).contains(&"E0001"), "{:?}", r.diagnostics);
        assert!(
            !error_codes(&r).contains(&"E0900"),
            "should not blame unsupported module paths: {:?}",
            r.diagnostics
        );
    }

    #[test]
    fn generic_param_bound_declaring_it_as_a_method_reports_that_reason() {
        // `T::fmt(x)` where `T`'s bound *does* declare `fmt` — just not as an
        // associated function: as a method with a `self` receiver. The
        // sibling test above's final clause ("none of its bounds declares
        // one") would be false here, since a bound does declare `fmt`.
        let r = check_src(
            "trait Show { fn fmt(self) -> String }\n\
             fn make<T: Show>(x: T) -> String { T::fmt(x) }\n\
             fn main() { }",
        );
        assert!(error_codes(&r).contains(&"E0001"), "{:?}", r.diagnostics);
        assert!(
            r.diagnostics
                .iter()
                .any(|d| d.message.contains("as a method with a `self` receiver")),
            "expected the message to name the real reason: {:?}",
            r.diagnostics
        );
        assert!(
            !r.diagnostics
                .iter()
                .any(|d| d.message.contains("none of its bounds declares one")),
            "should not claim the bound declares nothing: {:?}",
            r.diagnostics
        );
    }

    #[test]
    fn inherent_associated_function_on_a_primitive_resolves() {
        // The other half of teaching the two-segment path gate about primitive
        // type names: `impl Int { … }` records an inherent impl under the `Int`
        // head (and an *instance* method on it already dispatched fine), but
        // `Int::zero()` could not resolve while lookup was keyed on a type
        // `DefId`, which no primitive has. Keeping only the trait half working
        // would leave the two kinds of associated function inconsistent.
        let r = check_src(
            "impl Int { fn three() -> Int { 3 } }\n\
             fn main() { let n = Int::three()\n println(\"${n}\") }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
        let main_fn = r
            .module
            .functions
            .iter()
            .find(|f| f.name == "main")
            .expect("main compiled to a hir::Function");
        let n_ty = &main_fn
            .locals
            .iter()
            .find(|l| l.name == "n")
            .expect("`n` is a local in main")
            .ty;
        assert_eq!(*n_ty, Ty::Int);
    }

    #[test]
    fn unknown_associated_function_on_a_primitive_reports_e0001() {
        // A primitive type name is invisible to `Definitions::resolve_type`, so
        // before this task every `Primitive::f()` call — resolvable or not — fell
        // through to `check_path` and reported E0900.
        let r = check_src("fn main() { println(\"${Int::nope()}\") }");
        assert!(error_codes(&r).contains(&"E0001"), "{:?}", r.diagnostics);
        assert!(
            !error_codes(&r).contains(&"E0900"),
            "should not blame unsupported module paths: {:?}",
            r.diagnostics
        );
    }

    #[test]
    fn receiverless_trait_method_called_on_instance_reports_e0014() {
        // The `selfless_method_called_on_instance_reports_e0014` case for a
        // *trait* method rather than an inherent one. This impl conforms — the
        // trait declares no receiver and neither does the impl — so conformance
        // cannot catch it; resolution must. Before `has_self`, `resolve_method_on`
        // matched the trait method by name alone, `emit_trait_call` compared the
        // argument list against `tm.params` (which never holds `self`) and so saw
        // no arity error, and MIR prepended the receiver to a callee of arity 0:
        // the same Cranelift ICE, again from source `nova check` accepted.
        let r = check_src(
            "record P { v: Int }\n\
             trait Zero { fn zero() -> Int }\n\
             impl Zero for P { fn zero() -> Int { 42 } }\n\
             fn main() { let p = P { v: 0 }\n println(\"${p.zero()}\") }",
        );
        assert!(error_codes(&r).contains(&"E0014"), "{:?}", r.diagnostics);
    }

    // === Supertraits (`trait B: A`) ===

    #[test]
    fn impl_of_subtrait_without_supertrait_reports_e0072() {
        let r = check_src(
            "trait A { fn a(self) -> Int }\n\
             trait B: A { fn b(self) -> Int }\n\
             record R { v: Int }\n\
             impl B for R { fn b(self) -> Int { 2 } }\n\
             fn main() { }",
        );
        assert!(error_codes(&r).contains(&"E0072"), "{:?}", r.diagnostics);
    }

    #[test]
    fn impl_of_subtrait_with_supertrait_typechecks() {
        let r = check_src(
            "trait A { fn a(self) -> Int }\n\
             trait B: A { fn b(self) -> Int }\n\
             record R { v: Int }\n\
             impl A for R { fn a(self) -> Int { 1 } }\n\
             impl B for R { fn b(self) -> Int { 2 } }\n\
             fn main() { }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    }

    #[test]
    fn supertrait_method_callable_through_subtrait_bound() {
        // `T: B` implies `T: A`, so `a()` is callable.
        let r = check_src(
            "trait A { fn a(self) -> Int }\n\
             trait B: A { fn b(self) -> Int }\n\
             record R { v: Int }\n\
             impl A for R { fn a(self) -> Int { 1 } }\n\
             impl B for R { fn b(self) -> Int { 2 } }\n\
             fn sum<T: B>(x: T) -> Int { x.a() + x.b() }\n\
             fn main() { println(\"${sum(R { v: 0 })}\") }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    }

    #[test]
    fn supertrait_impl_may_be_declared_after_the_subtrait_impl() {
        // The declaration-order guard that decides *where* the
        // supertrait-satisfaction check may live. Running it inside
        // `check_impl_conformance` (called from `collect_impls` *before* the impl
        // being checked is pushed) only sees impls from earlier items, so this
        // source — `impl B` first, `impl A` second — would report a bogus E0072.
        // Nova has no forward-declaration rule for impls; the check therefore has
        // to be a post-collection pass, like `check_impl_coherence`.
        let r = check_src(
            "trait A { fn a(self) -> Int }\n\
             trait B: A { fn b(self) -> Int }\n\
             record R { v: Int }\n\
             impl B for R { fn b(self) -> Int { 2 } }\n\
             impl A for R { fn a(self) -> Int { 1 } }\n\
             fn main() { }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    }

    #[test]
    fn subtrait_bound_resolves_the_supertrait_method_to_the_supertrait() {
        // `supertrait_method_callable_through_subtrait_bound` only asserts the
        // absence of diagnostics, which is weak: `Ty::Error` unifies with
        // anything, so a mis-resolved call can be silent. Pin the emitted HIR
        // instead — `x.a()` inside `sum` must be a `TraitCall` naming trait `A`
        // (not `B`) with `Self` bound to the generic parameter `Param(0)`.
        let r = check_src(
            "trait A { fn a(self) -> Int }\n\
             trait B: A { fn b(self) -> Int }\n\
             record R { v: Int }\n\
             impl A for R { fn a(self) -> Int { 1 } }\n\
             impl B for R { fn b(self) -> Int { 2 } }\n\
             fn only_a<T: B>(x: T) -> Int { x.a() }\n\
             fn main() { println(\"${only_a(R { v: 0 })}\") }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
        let a_id = r
            .module
            .traits
            .iter()
            .find(|t| t.name == "A")
            .expect("trait A collected")
            .def_id;
        let only_a = r
            .module
            .functions
            .iter()
            .find(|f| f.name == "only_a")
            .expect("`only_a` compiled to a hir::Function");
        assert_eq!(
            only_a.bounds,
            vec![vec![
                r.module
                    .traits
                    .iter()
                    .find(|t| t.name == "B")
                    .expect("trait B collected")
                    .def_id,
                a_id
            ]],
            "the bound `T: B` should expand to `[B, A]`"
        );
        let call = child_exprs(&only_a.body)
            .into_iter()
            .chain(std::iter::once(&only_a.body))
            .find(|e| matches!(e.kind, hir::ExprKind::TraitCall { .. }))
            .expect("`only_a`'s body contains a TraitCall");
        let hir::ExprKind::TraitCall {
            trait_id, self_ty, ..
        } = &call.kind
        else {
            unreachable!("filtered above")
        };
        assert_eq!(*trait_id, a_id, "`x.a()` should dispatch through trait `A`");
        assert_eq!(*self_ty, Ty::Param(0), "`Self` should be the parameter `T`");
        assert_eq!(call.ty, Ty::Int);
    }

    #[test]
    fn subtrait_redeclaring_supertrait_method_name_is_ambiguous() {
        // `B: A` re-declaring a method name `A` already declares makes a call
        // through a `T: B` bound ambiguous: the bound expands to `[B, A]`,
        // and both traits now provide `same`, so `resolve_method_on` finds
        // two candidate providers. This is a newly reachable shape of the
        // pre-existing "ambiguous method call" diagnostic (Rust reports the
        // analogous conflict too, as E0034 "multiple applicable items in
        // scope"). Pin the behavior, don't try to make it resolve to `B`.
        let r = check_src(
            "trait A { fn same(self) -> Int }\n\
             trait B: A { fn same(self) -> Int }\n\
             record R { v: Int }\n\
             impl A for R { fn same(self) -> Int { 1 } }\n\
             impl B for R { fn same(self) -> Int { 2 } }\n\
             fn call_same<T: B>(x: T) -> Int { x.same() }\n\
             fn main() { println(\"${call_same(R { v: 0 })}\") }",
        );
        assert!(error_codes(&r).contains(&"E0015"), "{:?}", r.diagnostics);
    }

    #[test]
    fn supertrait_method_callable_from_a_default_body() {
        // A default body's `Self` is bounded by the enclosing trait alone; if that
        // bound is not expanded, `self.a()` in `B`'s default body cannot resolve.
        let r = check_src(
            "trait A { fn a(self) -> Int }\n\
             trait B: A { fn b(self) -> Int { self.a() + 1 } }\n\
             record R { v: Int }\n\
             impl A for R { fn a(self) -> Int { 1 } }\n\
             impl B for R { }\n\
             fn main() { let x = R { v: 0 }\n println(\"${x.b()}\") }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
        let b_body = r
            .module
            .functions
            .iter()
            .find(|f| f.name == "B::b$default")
            .expect("`B::b`'s default body compiled to a hir::Function");
        let a_id = r
            .module
            .traits
            .iter()
            .find(|t| t.name == "A")
            .expect("trait A collected")
            .def_id;
        // `child_exprs` is one level deep, and the call sits under a `Binary`
        // under the block, so walk the whole tree.
        fn calls_trait(e: &hir::Expr, tid: DefId) -> bool {
            if matches!(&e.kind, hir::ExprKind::TraitCall { trait_id, .. } if *trait_id == tid) {
                return true;
            }
            child_exprs(e).into_iter().any(|c| calls_trait(c, tid))
        }
        assert!(
            calls_trait(&b_body.body, a_id),
            "`self.a()` should resolve to a TraitCall on `A`"
        );
        assert_eq!(
            b_body.bounds,
            vec![vec![
                r.module
                    .traits
                    .iter()
                    .find(|t| t.name == "B")
                    .expect("trait B collected")
                    .def_id,
                a_id
            ]],
            "the default body's `Self` should be bounded by `B` and its supertrait `A`"
        );
    }

    #[test]
    fn unknown_supertrait_reports_e0001() {
        let r = check_src(
            "trait B: Nope { fn b(self) -> Int }\n\
             fn main() { }",
        );
        assert!(error_codes(&r).contains(&"E0001"), "{:?}", r.diagnostics);
    }

    #[test]
    fn where_clause_on_trait_declaration_reports_e0900() {
        // `trait B where Self: A` is a second spelling of a supertrait
        // requirement, distinct from the `trait B: A` shorthand that
        // `collect_supertraits` resolves into the graph. Wiring this spelling
        // in too is a feature addition, not a fix, so it must be rejected
        // outright — otherwise declaring a supertrait this way would once
        // again mean nothing (silently accepted, no `A` impl required),
        // exactly the bug this feature exists to close for `trait B: A`.
        let r = check_src(
            "trait A { fn a(self) -> Int }\n\
             trait B where Self: A { fn b(self) -> Int }\n\
             record R { v: Int }\n\
             impl B for R { fn b(self) -> Int { 2 } }\n\
             fn main() { }",
        );
        assert!(error_codes(&r).contains(&"E0900"), "{:?}", r.diagnostics);
    }

    #[test]
    fn cyclic_supertraits_do_not_hang() {
        // `trait A: B` / `trait B: A` is nonsense, but the compiler must
        // terminate on it rather than loop forever expanding bounds. No
        // diagnostic is required — only that this test returns.
        let r = check_src(
            "trait A: B { fn a(self) -> Int }\n\
             trait B: A { fn b(self) -> Int }\n\
             record R { v: Int }\n\
             impl A for R { fn a(self) -> Int { 1 } }\n\
             impl B for R { fn b(self) -> Int { 2 } }\n\
             fn both<T: A>(x: T) -> Int { x.a() + x.b() }\n\
             fn main() { println(\"${both(R { v: 0 })}\") }",
        );
        // The cycle is satisfiable (each impl exists), so nothing is reported;
        // the point of the test is that we get here at all.
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    }

    #[test]
    fn diamond_supertraits_are_not_ambiguous() {
        // `C: A + B` with `A: X` and `B: X` reaches `X` twice. A duplicated trait
        // id in the expanded bound set reads as two method providers, which
        // `resolve_method_on` reports as a false E0015 "ambiguous method call" —
        // the same trap `resolve_bounds`' dedup exists for.
        let r = check_src(
            "trait X { fn x(self) -> Int }\n\
             trait A: X { fn a(self) -> Int }\n\
             trait B: X { fn b(self) -> Int }\n\
             trait C: A + B { fn c(self) -> Int }\n\
             record R { v: Int }\n\
             impl X for R { fn x(self) -> Int { 1 } }\n\
             impl A for R { fn a(self) -> Int { 2 } }\n\
             impl B for R { fn b(self) -> Int { 3 } }\n\
             impl C for R { fn c(self) -> Int { 4 } }\n\
             fn only_x<T: C>(v: T) -> Int { v.x() }\n\
             fn main() { println(\"${only_x(R { v: 0 })}\") }",
        );
        // A diamond must not read as two providers of `x` (a false E0015);
        // `diagnostics.is_empty()` below already covers that and more, so
        // there is no separate `E0015`-specific assertion here.
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
        let only_x = r
            .module
            .functions
            .iter()
            .find(|f| f.name == "only_x")
            .expect("`only_x` compiled to a hir::Function");
        let bounds = only_x.bounds.first().expect("`T` has bounds");
        assert_eq!(
            bounds.len(),
            4,
            "expected exactly `[C, A, B, X]` with no repeat, got {bounds:?}"
        );
    }

    #[test]
    fn subtrait_bound_expansion_does_not_break_conformance_bound_sets() {
        // `check_impl_conformance` compares each method generic's bound set
        // against the trait's. Both sides must be expanded consistently, and
        // `M` is declared *before* `B` on purpose: an expansion that reads a
        // partially-built trait table would leave `M::m`'s `U: B` unexpanded
        // while the impl's is expanded, producing a bogus E0072.
        let r = check_src(
            "trait M { fn m<U: B>(self, u: U) -> Int }\n\
             trait A { fn a(self) -> Int }\n\
             trait B: A { fn b(self) -> Int }\n\
             record R { v: Int }\n\
             record S { w: Int }\n\
             impl A for S { fn a(self) -> Int { 1 } }\n\
             impl B for S { fn b(self) -> Int { 2 } }\n\
             impl M for R { fn m<U: B>(self, u: U) -> Int { u.a() + u.b() } }\n\
             fn main() {\n\
                 let r = R { v: 0 }\n\
                 let s = S { w: 0 }\n\
                 println(\"${r.m(s)}\")\n\
             }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    }

    #[test]
    fn impl_method_bound_redundantly_spelling_supertrait_typechecks() {
        // Deliberate behavior change: the trait declares `U: B`, which expands
        // to `[B, A]`. An impl is now free to spell that same set out in full
        // (`U: B + A`) instead of just `U: B` — before supertrait expansion
        // existed, `same_bound_set` would have compared the trait's
        // unexpanded `[B]` against the impl's `[B, A]` and reported E0072 for
        // a "mismatched" bound that is in fact redundant (`B + A` is exactly
        // `B` when `B: A`). Nothing pinned this previously, so it could
        // silently regress.
        let r = check_src(
            "trait A { fn a(self) -> Int }\n\
             trait B: A { fn b(self) -> Int }\n\
             trait M { fn m<U: B>(self, u: U) -> Int }\n\
             record R { v: Int }\n\
             record S { w: Int }\n\
             impl A for S { fn a(self) -> Int { 1 } }\n\
             impl B for S { fn b(self) -> Int { 2 } }\n\
             impl M for R { fn m<U: B + A>(self, u: U) -> Int { u.a() + u.b() } }\n\
             fn main() {\n\
                 let r = R { v: 0 }\n\
                 let s = S { w: 0 }\n\
                 println(\"${r.m(s)}\")\n\
             }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    }

    #[test]
    fn supertrait_method_callable_through_an_impl_generic_bound() {
        // The impl's own generic bound (`impl<T: B> …`) must be expanded too, so
        // a method body can call the supertrait method on `T`.
        let r = check_src(
            "trait A { fn a(self) -> Int }\n\
             trait B: A { fn b(self) -> Int }\n\
             record R { v: Int }\n\
             record W<T> { inner: T }\n\
             impl A for R { fn a(self) -> Int { 1 } }\n\
             impl B for R { fn b(self) -> Int { 2 } }\n\
             impl<T: B> W<T> { fn total(self) -> Int { self.inner.a() + self.inner.b() } }\n\
             fn main() {\n\
                 let w = W { inner: R { v: 0 } }\n\
                 println(\"${w.total()}\")\n\
             }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    }

    #[test]
    fn generic_impl_of_subtrait_without_a_covering_supertrait_impl_reports_e0072() {
        // `impl<T: B> B for W<T>` needs an `A` impl covering *every* `W<T>`;
        // an `impl A for W<R>` covers only one instance, so the requirement is
        // unmet and must be reported at the impl, not silently at some later
        // instantiation.
        let r = check_src(
            "trait A { fn a(self) -> Int }\n\
             trait B: A { fn b(self) -> Int }\n\
             record R { v: Int }\n\
             record W<T> { inner: T }\n\
             impl A for R { fn a(self) -> Int { 1 } }\n\
             impl B for R { fn b(self) -> Int { 2 } }\n\
             impl A for W<R> { fn a(self) -> Int { 3 } }\n\
             impl<T: B> B for W<T> { fn b(self) -> Int { 4 } }\n\
             fn main() { }",
        );
        assert!(error_codes(&r).contains(&"E0072"), "{:?}", r.diagnostics);
    }

    #[test]
    fn generic_impl_of_subtrait_with_a_generic_supertrait_impl_typechecks() {
        // The companion of the test above: an `impl<T: B> A for W<T>` covers the
        // whole family, so `impl<T: B> B for W<T>` is satisfied.
        let r = check_src(
            "trait A { fn a(self) -> Int }\n\
             trait B: A { fn b(self) -> Int }\n\
             record R { v: Int }\n\
             record W<T> { inner: T }\n\
             impl A for R { fn a(self) -> Int { 1 } }\n\
             impl B for R { fn b(self) -> Int { 2 } }\n\
             impl<T: B> A for W<T> { fn a(self) -> Int { self.inner.a() } }\n\
             impl<T: B> B for W<T> { fn b(self) -> Int { self.inner.b() } }\n\
             fn main() {\n\
                 let w = W { inner: R { v: 0 } }\n\
                 println(\"${w.b()}\")\n\
             }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    }

    #[test]
    fn hir_trait_def_records_resolved_supertraits() {
        let r = check_src(
            "trait A { fn a(self) -> Int }\n\
             trait B: A { fn b(self) -> Int }\n\
             record R { v: Int }\n\
             impl A for R { fn a(self) -> Int { 1 } }\n\
             impl B for R { fn b(self) -> Int { 2 } }\n\
             fn main() { }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
        let a = r
            .module
            .traits
            .iter()
            .find(|t| t.name == "A")
            .expect("trait A collected");
        let b = r
            .module
            .traits
            .iter()
            .find(|t| t.name == "B")
            .expect("trait B collected");
        assert_eq!(b.supertraits, vec![a.def_id]);
        assert!(a.supertraits.is_empty());
    }

    #[test]
    fn duplicate_supertrait_is_deduplicated() {
        // `trait B: A + A` must not record `A` twice — a repeat reads as two
        // providers of `a` and yields a false E0015 at the call site.
        let r = check_src(
            "trait A { fn a(self) -> Int }\n\
             trait B: A + A { fn b(self) -> Int }\n\
             record R { v: Int }\n\
             impl A for R { fn a(self) -> Int { 1 } }\n\
             impl B for R { fn b(self) -> Int { 2 } }\n\
             fn only_a<T: B>(x: T) -> Int { x.a() }\n\
             fn main() { println(\"${only_a(R { v: 0 })}\") }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
        let b = r
            .module
            .traits
            .iter()
            .find(|t| t.name == "B")
            .expect("trait B collected");
        assert_eq!(b.supertraits.len(), 1, "supertraits: {:?}", b.supertraits);
    }

    #[test]
    fn std_core_parses_and_typechecks_clean() {
        // The implicit std/core module must itself be error-free; a program that
        // uses nothing from it must produce no diagnostics.
        let r = check_src("fn main() { println(\"hi\") }");
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    }

    #[test]
    fn std_core_option_is_available_without_import() {
        let r = check_src(
            "fn main() {\n\
                 let o = Some(3)\n\
                 match o { Some(v) => println(\"${v}\"), None => println(\"none\") }\n\
             }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    }

    #[test]
    fn std_core_option_result_methods_typecheck() {
        let r = check_src(
            "fn dbl(n: Int) -> Int { n * 2 }\n\
             fn main() {\n\
                 let a = Some(21).map(dbl).unwrap_or(0)\n\
                 let b = Some(1).is_some()\n\
                 let c: Result<Int, String> = Some(2).ok_or(\"none\")\n\
                 let d = c.map(dbl).unwrap_or(0)\n\
                 println(\"${a} ${b} ${d}\")\n\
             }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    }

    #[test]
    fn std_core_traits_and_primitive_impls_typecheck() {
        let r = check_src(
            "fn show_all<T: Display>(x: T) -> String { x.fmt() }\n\
             fn main() {\n\
                 println(show_all(3))\n\
                 println(\"${(1).eq(1)}\")\n\
                 let o = (\"a\").cmp(\"b\")\n\
                 match o { Less => println(\"less\"), Equal => println(\"eq\"), \
                           Greater => println(\"gt\") }\n\
             }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    }

    #[test]
    fn vec_methods_typecheck() {
        let r = check_src(
            "fn main() {\n\
                 let mut v = Vec::new()\n\
                 v.push(1)\n\
                 v.push(2)\n\
                 println(\"${v.len()}\")\n\
                 match v.get(0) { Some(x) => println(\"${x}\"), None => println(\"none\") }\n\
                 match v.pop() { Some(x) => println(\"${x}\"), None => println(\"none\") }\n\
                 v.set(0, 9)\n\
                 v.clear()\n\
                 println(\"${v.len()}\")\n\
             }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    }

    #[test]
    fn map_methods_typecheck() {
        let r = check_src(
            "fn main() {\n\
                 let mut m = Map::new()\n\
                 let prev = m.insert(1, \"one\")\n\
                 println(\"${m.len()} ${m.contains_key(1)}\")\n\
                 match m.get(1) { Some(s) => println(s), None => println(\"none\") }\n\
                 match m.remove(1) { Some(s) => println(s), None => println(\"none\") }\n\
                 println(\"${m.len()}\")\n\
             }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
        // Not vacuous. `Ty::Error` unifies with anything, so an empty
        // diagnostic list alone would still hold if every one of these calls had
        // collapsed to an error expression — and `Map::new()`'s `K`/`V` are
        // fixed only by inference from the later calls, so a wrong signature
        // surfaces as an error type rather than as a diagnostic. Pin what the
        // checker actually built. `Ty::Unit` drops the `println`s, which are also
        // `Call`s; what remains is every `Map` method in the program, in source
        // order. `Option`'s and `Map`'s `DefId`s are matched structurally rather
        // than by number, but the type *arguments* are exact: `Map<Int, String>`
        // from `new`, and `Option<String>` — not `Option<_>` — from the three
        // that return one.
        let map_ty = |t: &Ty| matches!(t, Ty::Record { args, .. } if args.as_slice() == [Ty::Int, Ty::String]);
        let opt_str = |t: &Ty| matches!(t, Ty::Sum { args, .. } if args.as_slice() == [Ty::String]);
        let calls: Vec<Ty> = exprs_in(&r.module, "main")
            .into_iter()
            .filter(|e| matches!(e.kind, hir::ExprKind::Call { .. }) && e.ty != Ty::Unit)
            .map(|e| e.ty.clone())
            .collect();
        assert_eq!(calls.len(), 7, "seven non-unit `Map` calls: {calls:?}");
        assert!(
            map_ty(&calls[0]),
            "`Map::new()` is `Map<Int, String>`: {calls:?}"
        );
        assert!(
            opt_str(&calls[1]),
            "`insert` returns `Option<String>`: {calls:?}"
        );
        assert_eq!(calls[2], Ty::Int, "`len` returns `Int`: {calls:?}");
        assert_eq!(
            calls[3],
            Ty::Bool,
            "`contains_key` returns `Bool`: {calls:?}"
        );
        assert!(
            opt_str(&calls[4]),
            "`get` returns `Option<String>`: {calls:?}"
        );
        assert!(
            opt_str(&calls[5]),
            "`remove` returns `Option<String>`: {calls:?}"
        );
        assert_eq!(calls[6], Ty::Int, "`len` returns `Int`: {calls:?}");
    }

    // `Map`'s key contract is `K: Hash + Eq`, and a key satisfying neither must
    // be rejected. That test is *not* here: like every trait bound in Nova
    // (`12-TYPESYSTEM.md` §5.4) it is discharged at monomorphization, so `check`
    // alone reports nothing at all for `Map<Unhashable, Int>` — the same reason
    // the conditional-impl note above gives. It lives in `nova-mir`'s
    // `map_key_without_hash_reports_e0013`, whose helper runs
    // `check` + `lower_module`, which is exactly what `nova check` runs
    // (`nova_driver::check_file`).

    #[test]
    fn set_methods_typecheck() {
        let r = check_src(
            "fn main() {\n\
                 let mut s = Set::new()\n\
                 println(\"${s.insert(1)} ${s.insert(1)}\")\n\
                 println(\"${s.len()} ${s.contains(1)} ${s.contains(2)}\")\n\
                 println(\"${s.remove(1)} ${s.len()}\")\n\
             }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
        // Not vacuous, as `map_methods_typecheck` above notes: `Ty::Error`
        // unifies with anything, and `Set::new()`'s `T` is fixed only by
        // inference from the later calls, so a wrong signature would surface
        // as an error type rather than as a diagnostic. Pin what the checker
        // actually built. `Ty::Unit` drops the `println`s, which are also
        // `Call`s; what remains is every `Set` method call, in source order.
        // `Set`'s own `DefId` is looked up rather than assumed, because `Vec`
        // is also a single-type-argument record and `args == [Ty::Int]` alone
        // would not tell the two apart.
        let set_def_id = r
            .module
            .records
            .iter()
            .find(|rt| rt.name == "Set")
            .expect("Set is defined")
            .def_id;
        let set_int = |t: &Ty| matches!(t, Ty::Record { def_id, args } if *def_id == set_def_id && args.as_slice() == [Ty::Int]);
        let calls: Vec<Ty> = exprs_in(&r.module, "main")
            .into_iter()
            .filter(|e| matches!(e.kind, hir::ExprKind::Call { .. }) && e.ty != Ty::Unit)
            .map(|e| e.ty.clone())
            .collect();
        assert_eq!(calls.len(), 8, "eight non-unit `Set` calls: {calls:?}");
        assert!(set_int(&calls[0]), "`Set::new()` is `Set<Int>`: {calls:?}");
        assert_eq!(
            calls[1],
            Ty::Bool,
            "first `insert` returns `Bool`: {calls:?}"
        );
        assert_eq!(
            calls[2],
            Ty::Bool,
            "second `insert` returns `Bool`: {calls:?}"
        );
        assert_eq!(calls[3], Ty::Int, "`len` returns `Int`: {calls:?}");
        assert_eq!(
            calls[4],
            Ty::Bool,
            "first `contains` returns `Bool`: {calls:?}"
        );
        assert_eq!(
            calls[5],
            Ty::Bool,
            "second `contains` returns `Bool`: {calls:?}"
        );
        assert_eq!(calls[6], Ty::Bool, "`remove` returns `Bool`: {calls:?}");
        assert_eq!(calls[7], Ty::Int, "final `len` returns `Int`: {calls:?}");
    }

    /// Written as an exhaustive `match` rather than a list of `assert_eq!`s so
    /// that adding a `Builtin` without stating its expected signature does not
    /// compile. The previous hand-written list silently missed `StrLenChars`.
    #[test]
    fn builtin_signatures_are_what_the_std_call_sites_use() {
        // The site each signature has to satisfy, so a mismatch names the
        // caller it would break rather than only the types.
        fn expected(b: Builtin) -> ((Vec<Ty>, Ty), &'static str) {
            match b {
                Builtin::Println | Builtin::Print => {
                    ((vec![Ty::String], Ty::Unit), "`println(s)` / `print(s)`")
                }
                Builtin::Panic => ((vec![Ty::String], Ty::Never), "`panic(msg)` diverges"),
                Builtin::StrCmp => (
                    (vec![Ty::String, Ty::String], Ty::Int),
                    "`str_cmp(self, other)` in `impl Ord for String`",
                ),
                Builtin::StrHash => (
                    (vec![Ty::String], Ty::Int),
                    "`str_hash(self)` in `impl Hash for String`",
                ),
                Builtin::CharToInt => (
                    (vec![Ty::Char], Ty::Int),
                    "`char_to_int(self)` in `impl Hash for Char`",
                ),
                Builtin::StrLenChars => (
                    (vec![Ty::String], Ty::Int),
                    "`str_len_chars(self)` in `String::len`",
                ),
                Builtin::StrChars => (
                    (vec![Ty::String], Ty::Array(Box::new(Ty::Char))),
                    "`str_chars(self)` in `String::chars`",
                ),
                Builtin::StrFromChars => (
                    (vec![Ty::Array(Box::new(Ty::Char))], Ty::String),
                    "`str_from_chars(cs)` in `chars_to_string`",
                ),
                Builtin::StrToUpper => (
                    (vec![Ty::String], Ty::String),
                    "`str_to_upper(self)` in `String::to_upper`",
                ),
                Builtin::StrToLower => (
                    (vec![Ty::String], Ty::String),
                    "`str_to_lower(self)` in `String::to_lower`",
                ),
            }
        }
        for b in Builtin::ALL {
            let (sig, site) = expected(b);
            assert_eq!(builtin_signature(b), sig, "{}: {site}", b.name());
        }
    }

    /// The shared arity/argument path all builtins now go through, exercised
    /// via the one family a *user* program can call. Before these, nothing in
    /// the suite reached `check_builtin_call`'s error branches at all.
    #[test]
    fn builtin_call_with_wrong_arity_reports_e0016() {
        let r = check_src("fn main() { println(\"a\", \"b\") }");
        assert!(error_codes(&r).contains(&"E0016"), "{:?}", r.diagnostics);
    }

    #[test]
    fn builtin_call_with_a_non_string_argument_reports_e0010() {
        let r = check_src("fn main() { println(7) }");
        assert!(error_codes(&r).contains(&"E0010"), "{:?}", r.diagnostics);
        // The print family keeps its interpolation hint; the std-only builtins
        // deliberately have none.
        assert!(
            r.diagnostics
                .iter()
                .any(|d| d.message.contains("use string interpolation")),
            "{:?}",
            r.diagnostics
        );
    }

    #[test]
    fn hash_impls_typecheck_for_primitives() {
        let r = check_src(
            "fn h<T: Hash>(x: T) -> Int { x.hash() }\n\
             fn main() {\n\
                 println(\"${h(7)} ${h(true)} ${h('c')} ${h(\"s\")}\")\n\
             }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
        // Not vacuous: `Ty::Error` unifies with anything, so an empty
        // diagnostic list alone would still hold if `x.hash()` had collapsed
        // to an error expression. Pin what the checker actually built — a
        // `TraitCall` dispatching through `T`'s bound, typed `Int`.
        let call = exprs_in(&r.module, "h")
            .into_iter()
            .find(|e| matches!(e.kind, hir::ExprKind::TraitCall { .. }))
            .expect("`x.hash()` is a trait-method call");
        assert_eq!(
            call.ty,
            Ty::Int,
            "`hash()` must be typed `Int`, not an error type"
        );
    }

    /// The companion the generic test above cannot be: inside `h`, `self_ty` is
    /// `Param(0)` and the bound is only discharged at monomorphization, so
    /// `hash_impls_typecheck_for_primitives` would still pass with `trait Hash`
    /// declared and *no* impls at all. Calling `.hash()` on each concrete
    /// primitive resolves against the impl itself (`E0014 no method 'hash' on
    /// type 'Int'` otherwise), which is what pins all four impls existing —
    /// including `Hash for Char`, whose `Char`→`Int` conversion needed the
    /// `char_to_int` builtin.
    #[test]
    fn hash_dispatches_on_each_concrete_primitive() {
        let r = check_src(
            "fn main() {\n\
                 println(\"${(7).hash()} ${true.hash()} ${'c'.hash()} ${(\"s\").hash()}\")\n\
             }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
        let tys: Vec<&Ty> = exprs_in(&r.module, "main")
            .into_iter()
            .filter(|e| matches!(e.kind, hir::ExprKind::TraitCall { .. }))
            .map(|e| &e.ty)
            .collect();
        assert_eq!(tys.len(), 4, "four `.hash()` calls: {tys:?}");
        assert!(
            tys.iter().all(|t| **t == Ty::Int),
            "every `hash()` is typed `Int`, not an error type: {tys:?}"
        );
    }

    /// `Hash for Float` is deliberately absent (ADR 0005 §2): NaN never equals
    /// itself, so a NaN key would be unreachable in any hash map, and
    /// `0.0`/`-0.0` compare equal but would hash differently. Pinned as a
    /// diagnostic so re-adding it is a deliberate act with a test to change,
    /// not an accident.
    #[test]
    fn float_has_no_hash_impl() {
        let r = check_src("fn main() { println(\"${(1.5).hash()}\") }");
        assert!(
            error_codes(&r).contains(&"E0014"),
            "expected E0014 no method `hash` on `Float`: {:?}",
            r.diagnostics
        );
    }

    // `str_cmp_builtin_wrong_arity_reports_e0016` and
    // `str_cmp_builtin_rejects_non_string_argument` used to live here,
    // calling `str_cmp(...)` directly from a *user* module to exercise
    // `check_builtin_call`'s `Builtin::StrCmp` arity/type-error branches.
    // They were removed by the Fix 1 review pass (nova-resolver's
    // `Builtin::STD_ONLY`): `str_cmp` is no longer seeded into user
    // module scopes, so `str_cmp(...)` written in a user module no longer
    // resolves at all — it now fails name resolution with `E0001 cannot find
    // function 'str_cmp'`, not the `E0016`/`E0010` these tests asserted.
    // `std/core` is the only remaining caller, and its one call site
    // (`str_cmp(self, other)` in `impl Ord for String`, above) always passes
    // exactly two `String` arguments, so the arity/type-error branches are
    // now defensive-only — unreachable from any Nova program, user or
    // std/core. Exercising them would require bypassing name resolution
    // entirely to call `check_builtin_call` with hand-built `Checker`/`FnCtx`
    // state, which is disproportionate test-harness surgery for dead code
    // paths, so the tests were dropped rather than repointed. The success
    // path they didn't cover anyway (`("a").cmp("b")`, i.e. `Ord for String`
    // dispatching through the builtin) remains covered by
    // `std_core_traits_and_primitive_impls_typecheck` above, unaffected by
    // Fix 1 since it calls `.cmp()` as a method, never `str_cmp` by name.
    //
    // `Builtin::StrHash` and `Builtin::CharToInt` (ADR 0005 §2) joined the
    // same std-only list and are unreachable for the same reason. Rather than
    // add two more untestable copies of the arity/type logic,
    // `check_builtin_call` was collapsed onto one shared path driven by
    // `builtin_signature`: the two error branches are now exercised by
    // `println`, which a user program *can* misuse
    // (`builtin_call_with_wrong_arity_reports_e0016` /
    // `builtin_call_with_a_non_string_argument_reports_e0010`), and the part
    // that is genuinely per-builtin — the signature itself — is unit-tested
    // directly by `builtin_signatures_are_what_the_std_call_sites_use`.
}
