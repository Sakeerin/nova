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

/// The `next` a `for` loop drives, as [`Checker::iterator_next`] resolved it.
///
/// The `Option` sum and its two variant indices travel with the method because
/// they are read off the *same* trait declaration: a caller that looked them up
/// separately could pair a `next` from one trait with the variant numbering of
/// another sum, which lowers to a `match` switching on the wrong tags.
struct IteratorNext {
    trait_id: DefId,
    method_idx: u32,
    /// The sum `next` returns — `Option` in std, but only its shape is required.
    option: DefId,
    /// Index of the one-field `Some` variant, and of the empty `None` variant.
    some: u32,
    none: u32,
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
enum TraitCallSelf<'a> {
    /// `receiver.name(args)`: `Self` is the receiver's resolved type and the
    /// receiver becomes the callee's `self` argument. Valid only for a method
    /// the trait declares *with* a `self` receiver ([`hir::TraitMethod::has_self`]).
    ///
    /// The receiver's **AST** travels alongside its checked form for the same
    /// reason [`Checker::check_mutable_receiver`] takes one: `place_root` walks
    /// the field/index projection shape, which the checked `hir::Expr` has
    /// already lost. Carrying it *in the variant* rather than as a separate
    /// parameter is what makes the mutable-receiver rule unbypassable by route —
    /// there is no way to hand `emit_trait_call` a receiver without also handing
    /// it the place to classify, so a future caller cannot reintroduce the gap
    /// ADR 0005 §1 recorded by simply forgetting a check.
    Receiver(hir::Expr, &'a Spanned<ast::Expr>),
    /// `Type::name(args)` or `T::name(args)`: `Self` comes from the path
    /// qualifier and there is no receiver. Valid only for a receiver-less
    /// method (a trait associated function).
    Qualifier(Ty),
}

/// The `Self` of an `impl` block, for resolving `Self::Item` written inside it.
///
/// An impl's `Self` cannot be a `generics` entry the way a trait body's is: a
/// trait body's `Self` is an implicit type *parameter* at `Param(0)`, whereas an
/// impl's is a concrete (possibly compound) type — `W<Param(0)>` for
/// `impl<T> Tr for W<T>` — with no parameter index and therefore no slot in the
/// by-index bound table either. Its candidate traits are not bounds at all but
/// the single trait the impl implements.
#[derive(Debug, Clone)]
struct ImplSelf {
    /// The impl's self type, with the impl's own `Param(k)` still in it.
    ty: Ty,
    /// The trait this impl implements, or `None` for an inherent impl — which
    /// implements no trait and so has no associated type to project onto.
    trait_id: Option<DefId>,
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
        impl_self: None,
        impl_selves: FxHashMap::default(),
        extra_functions: Vec::new(),
        next_closure_def: defs.defs().len() as u32,
        type_arity: FxHashMap::default(),
        externs: Vec::new(),
        diagnostics: Vec::new(),
    };
    // Before any collection pass: every later pass builds a generic scope, and
    // one containing `Self` would give the name two meanings at once.
    checker.reject_self_type_params();
    checker.collect_type_arities();
    // Before `collect_records`: since Task 1 of the iterator-finishing plan,
    // `collect_records` resolves each record's bounds and calls
    // `expand_bounds`, which folds transitive supertraits in from
    // `Checker::supertraits` — the very table `collect_supertraits` builds.
    // Moved ahead of `collect_records` (and `collect_sums`, which never reads
    // it but has no reason to run first either) so that table is populated
    // before the first `expand_bounds` call, not just before
    // `collect_traits`'s. `collect_supertraits` itself only reads trait
    // declarations via `self.defs` (already fully resolved before `check`
    // runs) and `decl.supertraits`, so it has no ordering dependency on
    // `collect_type_arities`, `collect_records`, or `collect_sums`.
    //
    // It did change the *order* diagnostics are rendered in, and nothing sorts
    // `self.diagnostics` before rendering, so pass order is emission order.
    // Measured on one program carrying an unresolvable supertrait, a duplicate
    // record generic and a sum-type bound: `E0001` -> `E0403` -> `E0900` now,
    // `E0403` -> `E0900` -> `E0001` with this call back in its old slot (the
    // move was re-run in both directions to confirm it, not inferred from the
    // pass list). Cosmetic — every diagnostic is still reported, with its own
    // span — and recorded rather than fixed: stabilizing order would mean
    // sorting by span, which is a separate decision affecting every diagnostic
    // in the compiler.
    checker.collect_supertraits();
    checker.collect_records();
    checker.collect_sums();
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
    /// Impl methods whose `self` receiver is declared `mut`. Calling one requires
    /// a mutable receiver place at the call site, so `mut` keeps the meaning it
    /// already has for `arr[i] = v` and `rec.f = v` (see ADR 0005 §1).
    ///
    /// Populated in `collect_impls` for **every** impl method, inherent or in a
    /// trait impl, and read for two different purposes. An *inherent* callee is
    /// looked up here at its call site ([`Checker::check_mutable_receiver`]). A
    /// *trait* callee is not: the call site consults
    /// [`hir::TraitMethod::mut_self`] instead, because trait dispatch resolves
    /// to `(trait_id, method_index)` with no impl to look up. The trait-impl
    /// entries are what
    /// [`Checker::check_impl_method_signatures`] compares against the trait's
    /// declaration, so the two flags cannot silently disagree.
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
    /// The `Self` of the `impl` block whose signatures or body are being
    /// converted right now, or `None` outside one. Read by [`Checker::convert_ty`]
    /// to resolve `Self::Item` inside an impl.
    ///
    /// Ambient rather than a `convert_ty` parameter because `convert_ty` has 18
    /// call sites and only two of them are inside an impl. The discipline is
    /// that every per-item entry point sets it: `collect_impls` per item,
    /// `check_method` per method (`None` for a trait default body, whose `Self`
    /// is the trait's own `Param(0)` in `generics` instead), and
    /// `check_function` / `check_const` clear it.
    impl_self: Option<ImplSelf>,
    /// Each impl's `Self`, keyed by AST item index, so [`Checker::check_method`]
    /// can restore it when compiling a body in a later pass without re-running
    /// (and re-diagnosing) the self-type conversion.
    impl_selves: FxHashMap<usize, ImplSelf>,
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
    /// Whether the function body currently being checked is `async`, which is
    /// what makes `.await` legal. Set from `f.is_async` on entry and reset to
    /// `false` inside a closure body — the same discipline `loop_depth` uses
    /// above, and for the same reason: a closure is its own non-async
    /// function even when written inside an `async fn`.
    in_async: bool,
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
    /// Reject a user-written type parameter named `Self` (`E0076`), at every
    /// place a generic parameter can be declared.
    ///
    /// `Self` already means something in two scopes, and in neither is it a
    /// parameter the user declared: inside a trait body it is the trait's own
    /// implicit type, bound at `Param(0)` by [`self_generic_scope`]; inside an
    /// impl it is that impl's self type, read from [`Checker::impl_self`] by
    /// [`Checker::convert_ty`]. But `Token::SelfUpper` parses to the plain
    /// string `"Self"`, so the parser admits it as an ordinary identifier and
    /// [`generic_scope`] would bind it like any other name — giving `Self` a
    /// second meaning in the very scope where it already has one, and making
    /// `Self::Item` mean two different things depending on whether the user
    /// happened to declare a parameter by that name. `E0076` and not `E0900`:
    /// this is a name that will never be legal, not a feature arriving later.
    ///
    /// One pass over every item rather than a call beside each
    /// [`Checker::check_duplicate_generics`] — those five sites cover neither a
    /// trait's own generics nor an impl method's, and a generic-carrying
    /// construct added later would silently opt out of a per-site check. This
    /// walks all six of the AST's `generics` fields.
    ///
    /// The rejected name is still bound by `generic_scope` afterwards, so the
    /// rest of the declaration resolves and the user gets this one error rather
    /// than a cascade of "cannot find type `Self`".
    fn reject_self_type_params(&mut self) {
        // Copy the `&'a File` so the item borrow is tied to `'a`, not to `self`.
        let file: &'a ast::File = self.file;
        for item in &file.items {
            match &item.value {
                ast::Item::Function(f) => self.reject_self_generic_name(&f.generics),
                ast::Item::Record(r) => self.reject_self_generic_name(&r.generics),
                ast::Item::Type(t) => self.reject_self_generic_name(&t.generics),
                ast::Item::Trait(t) => {
                    self.reject_self_generic_name(&t.generics);
                    for ti in &t.items {
                        match ti {
                            TraitItem::Required(sig) => {
                                self.reject_self_generic_name(&sig.generics)
                            }
                            TraitItem::Provided(f) => self.reject_self_generic_name(&f.generics),
                            // An associated type declares no generics of its
                            // own — Nova has no generic associated types.
                            TraitItem::AssocType { .. } => {}
                        }
                    }
                }
                ast::Item::Impl(b) => {
                    self.reject_self_generic_name(&b.generics);
                    for f in &b.functions {
                        self.reject_self_generic_name(&f.generics);
                    }
                }
                ast::Item::Extern(b) => {
                    for ei in &b.items {
                        match ei {
                            ExternItem::Fn(sig) => self.reject_self_generic_name(&sig.generics),
                        }
                    }
                }
                ast::Item::Const(_) | ast::Item::Import(_) | ast::Item::Module(_) => {}
            }
        }
    }

    /// The per-declaration half of [`Checker::reject_self_type_params`]. One
    /// diagnostic per offending parameter, so a second is not hidden behind the
    /// first.
    fn reject_self_generic_name(&mut self, generics: &[ast::TypeParam]) {
        for g in generics {
            if g.name.value == "Self" {
                self.error(
                    "E0076",
                    "a generic parameter may not be named `Self`: in a trait `Self` is the \
                     implementing type, and in an impl it is that impl's self type",
                    g.name.span,
                );
            }
        }
    }

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

    /// Reject a trait bound written on a sum type's *own* generic parameter
    /// (`type Wrap<T: Hash> = …`).
    ///
    /// Record type parameters used to go through this too, but no longer do:
    /// since the iterator-finishing plan's Task 1, a bound on a record's
    /// parameter is instead resolved by `resolve_bounds` + `expand_bounds` in
    /// `collect_records` and passed to `convert_ty`, as a **resolution scope**
    /// for a projection in a field type (`f: fn(I::Item) -> U`) — not a
    /// constraint. See the comment at that call site for why enforcement is
    /// still out of scope. Sum types have no such use case yet (no variant
    /// payload needs to name a projection on the sum's own parameter), so the
    /// bound stays rejected here.
    ///
    /// Such a bound parses but nothing honours it: [`hir::SumType`] carries no
    /// `bounds` field, and monomorphization discharges only *function* and
    /// *impl* bounds (`nova-mir`'s `mono.rs` walks a worklist of function
    /// instances). Enforcing it would need a notion of "sum type instantiation
    /// site" that no pass has — a sum type's type arguments survive only
    /// inside the enclosing expression's `Ty`, `ExprKind::MakeVariant` does not
    /// record them, and MIR erases them to `Ptr` — so, exactly as for `trait B
    /// where Self: A`, the construct is rejected loudly rather than left
    /// reading as meaningful. Put the bound on an `impl` block instead, which
    /// *is* enforced (this is what `std/collections`' `Map<K, V>` does).
    ///
    /// One diagnostic per bounded parameter, so a second offender is not hidden
    /// behind the first. The bound names are deliberately **not** resolved: an
    /// unknown trait here would stack an `E0001` cascade on top of the real
    /// error. `owner` is the plural noun phrase for the message; the one
    /// caller left passes "sum type parameters".
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
            let generics = generic_scope(&decl.generics);
            // A bound on a record's type parameter is a RESOLUTION SCOPE, not a
            // constraint: it exists so a field type may name a projection on
            // that parameter (`f: fn(I::Item) -> U`), which is what makes a
            // lazy iterator adapter expressible. It is deliberately NOT checked
            // at construction — `MakeRecord` carries no type arguments, and
            // monomorphization visits only instances reachable from `main`, so
            // enforcement would fire *sometimes*, which is worse than not at
            // all (the Phase 2.2a assessment; ADR 0007 records it). Safety comes
            // from the impl: `impl<I: Iterator, U> Iterator for MapIter<I, U>`
            // requires the bound, so a `MapIter<Int, U>` simply has no
            // `Iterator` impl and is inert.
            let mut bounds = self.resolve_bounds(&decl.generics);
            self.expand_bounds(&mut bounds);
            let fields = decl
                .fields
                .iter()
                .map(|f| hir::RecordField {
                    name: f.name.value.clone(),
                    ty: self.convert_ty(&f.ty, &generics, &bounds),
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
                    // No bounds: `reject_type_param_bounds` above rejects any
                    // bound on a sum type's generic parameters outright.
                    fields: v
                        .fields
                        .iter()
                        .map(|t| self.convert_ty(t, &generics, &[]))
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
    /// [`Checker::supertraits`]. Runs before [`Checker::collect_traits`] *and*
    /// [`Checker::collect_records`] (the first two callers that expand a bound
    /// list) so the whole supertrait graph is known by the time the first
    /// `expand_bounds` call is made, whatever order the traits are declared
    /// in.
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
            // Needed before the items loop below: an associated type's `Def`
            // (`DefKind::AssocType { trait_def }`) points back to this trait's
            // own id, so matching them up requires knowing it up front rather
            // than only after `methods` is built.
            let def_id = DefId(i as u32);
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
            // The resolver pushes one `Def` per `TraitItem::AssocType` for
            // this trait, in the same order it walked `t.items` — the same
            // order `decl.items` is walked just below. Filtering preserves
            // that relative order, so draining this front-to-back as each
            // `AssocType` item is encountered lines each one up with its
            // `DefId` without needing a second, item-indexed lookup table.
            let mut assoc_type_ids =
                self.defs
                    .defs()
                    .iter()
                    .enumerate()
                    .filter_map(|(di, d)| match d.kind {
                        DefKind::AssocType { trait_def } if trait_def == def_id => {
                            Some(DefId(di as u32))
                        }
                        _ => None,
                    });
            let mut assoc_types = Vec::new();
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
                    TraitItem::AssocType { name, bounds } => {
                        if !bounds.is_empty() {
                            self.unsupported(name.span, "trait bounds on an associated type");
                        }
                        // Drained unconditionally, *before* the duplicate check
                        // can `continue`: this iterator is positional (see its
                        // comment above), so skipping a `next()` would hand the
                        // *next* associated type this one's `DefId` and every
                        // later one would be off by one. The whole point of the
                        // rejection is that a duplicate name is ambiguous, so
                        // misaligning the survivors while reporting it would be
                        // the same bug one seat over.
                        let assoc_def_id = assoc_type_ids.next();
                        // Two declarations of one name leave which `DefId` a
                        // projection resolves to up to `find_assoc_type`'s scan
                        // order — it takes the first, so the second is silently
                        // dead: nothing binds it, nothing can name it, and
                        // conformance matches bindings by *name* so the missing
                        // binding is not reported either. Keep the first and
                        // reject the rest, the same `E0403` a duplicate generic
                        // parameter and a duplicate associated-type *binding* in
                        // an impl already get.
                        if assoc_types.iter().any(|(n, _)| n == &name.value) {
                            self.error(
                                "E0403",
                                format!(
                                    "the name `{}` is already used for an associated type of \
                                     this trait",
                                    name.value
                                ),
                                name.span,
                            );
                            continue;
                        }
                        if let Some(assoc_def_id) = assoc_def_id {
                            assoc_types.push((name.value.clone(), assoc_def_id));
                        }
                        continue;
                    }
                };
                // The same hole as the duplicate associated type above, wearing
                // different clothes, and the plan's Step 2 asked for it to be
                // checked rather than assumed: `trait It { fn g(self) -> Int\n
                // fn g(self) -> Bool }` was accepted, and the second declaration
                // is dead in exactly the same way. `trait_method_index` and
                // `check_impl_method_signatures` both take the *first* match by
                // name, so an impl conforms to signature one while signature two
                // constrains nothing — a trait can promise two contradictory
                // things and the impl is checked against whichever is written
                // first.
                //
                // On the name alone, not on `(name, has_self)`. The two lookups
                // that consume this list partition it by receiver
                // (`trait_method_index` wants `has_self`,
                // `trait_assoc_fn_index` wants `!has_self`), so allowing
                // `fn g(self)` beside `fn g()` would make `g` mean two different
                // things depending on the *call syntax* — which is the very
                // ambiguity being rejected, not an exception to it.
                //
                // Reported before the signature work below rather than after, so
                // a rejected duplicate does not also emit its own `E0900`s and
                // `E0001`s: one mistake, one diagnostic.
                if methods
                    .iter()
                    .any(|m: &hir::TraitMethod| m.name == name.value)
                {
                    self.error(
                        "E0403",
                        format!(
                            "the name `{}` is already used for a method of this trait",
                            name.value
                        ),
                        name.span,
                    );
                    continue;
                }
                // Trait methods stay `E0900`-rejected even though free
                // functions and impl/inherent methods no longer are (Phase
                // 2.3a Task 2): trait async needs associated-type futures,
                // out of scope here. Checked on the *table* entry, not only
                // in the default-body pass below, so a declaration-only
                // (`Required`) trait method — which the default-body pass
                // never visits — is covered too.
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
                // `Self` (at index 0, matching `m_scope`) is bounded by the
                // enclosing trait plus its supertraits — needed so this
                // method's OWN signature can resolve `Self::Item` (e.g.
                // `Iterator::next`'s `Option<Self::Item>`) before this
                // trait's `hir::TraitDef` even exists (see `find_assoc_type`).
                // Computed and expanded *before* `method_sig_parts` so
                // `convert_ty` can see it; `TraitMethod.bounds` stays indexed
                // 0..generics with no Self slot, so it is sliced off below
                // rather than stored as-is.
                let mut sig_bounds = vec![vec![def_id]];
                sig_bounds.extend(self.resolve_bounds(generics));
                // A `where` clause on a trait method is rejected above, so the
                // inline bounds are the complete set.
                self.expand_bounds(&mut sig_bounds);
                let (m_params, m_ret) = self.method_sig_parts(params, ret, &m_scope, &sig_bounds);
                let m_bounds = sig_bounds[1..].to_vec();
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
                    // The same predicate `collect_impls` uses for the impl side
                    // of this flag, for the same reason `has_self` uses `any`:
                    // `method_sig_parts` strips a `self` at any position, so a
                    // `params[0]`-shaped predicate would classify a misplaced
                    // receiver as a non-mutator while the signature machinery
                    // still treated it as a receiver. Both sides of the
                    // conformance comparison below must be decided the same way
                    // or the comparison itself is the bug.
                    mut_self: params.iter().any(|p| p.name.value == "self" && p.is_mut),
                    generics: generics.len() as u32,
                    bounds: m_bounds,
                    default_def,
                });
            }
            self.traits.push(hir::TraitDef {
                def_id,
                name: def.name.clone(),
                supertraits: self.supertraits.get(&def_id).cloned().unwrap_or_default(),
                methods,
                assoc_types,
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
                // One generic for `Self` (bounded by the trait) plus the method's
                // own generics, at the same flat Param indices. Computed *before*
                // `method_sig_parts` (reordered from its previous position after
                // it) so `convert_ty` can resolve a `Self::Item` this default
                // body's own signature might mention.
                let mut bounds = vec![vec![trait_id]];
                bounds.extend(self.resolve_bounds(&f.generics));
                // `Self`'s bound is the enclosing trait, so expanding it is what
                // lets a `trait B: A` default body call `self.a()` — and lets its
                // signature resolve `Self::Item` when `Item` is declared by `A`.
                self.expand_bounds(&mut bounds);
                let (mut params, ret) =
                    self.method_sig_parts(&f.params, &f.return_ty, &scope, &bounds);
                // Prepend the `self` receiver typed as `Self` (`Param(0)`) — but
                // only for a method that declares one. A default-bodied
                // associated function (`fn zero() -> Self { … }`) has no
                // receiver, and prepending one would desynchronise `sig.params`
                // from the AST parameter list that `check_fn_body` zips against
                // it. Mirrors the same conditional in `collect_impls`.
                if f.params.iter().any(|p| p.name.value == "self") {
                    params.insert(0, Ty::Param(0));
                }
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
            // Cleared up front, not only on the way out: several paths below
            // `continue` past the end of the loop body, and a stale `Self` from
            // the previous impl would then resolve a `Self::Item` written in
            // this one against the wrong type.
            self.impl_self = None;
            self.check_duplicate_generics(&block.generics, "impl");
            // The impl's generic parameters (`impl<T> …`) are in scope in the
            // self type and every method signature/body.
            let impl_generics = generic_scope(&block.generics);
            let mut impl_bounds = self.resolve_bounds(&block.generics);
            self.apply_where(&mut impl_bounds, &block.where_clause, &impl_generics);
            self.expand_bounds(&mut impl_bounds);
            let self_ty = self.convert_ty(&block.ty, &impl_generics, &impl_bounds);
            // A projection anywhere in the self type is rejected outright, and
            // *before* the head check below so the message names the real
            // problem: a bare `impl<T: It> Tr for T::Item` otherwise reports
            // "impl blocks are only supported on named types", which is
            // misleading because `T::Item` may well resolve to a named type.
            //
            // Two independent defects compound here, which is why this is not a
            // position to leave accepted-and-broken. Measured on `25db453`, both
            // on the same five-line program:
            //
            //  * **The impl can never be selected.** `Ty::match_pattern`
            //    recovers an impl's type arguments by matching its self type
            //    structurally against a ground type, and it cannot invert
            //    `T::Item` to find `T`. So the impl is dead code.
            //  * **It is invisible to coherence.** `hir::self_types_overlap`'s
            //    helpers do not understand `Assoc`, so
            //    `impl<T: It> Tr for W<T::Item>` and `impl Tr for W<Int>` do not
            //    conflict — while the control, `impl<T> Tr for W<T>` in place of
            //    the first, correctly reports `E0074`. Any `T` with
            //    `Item = Int` makes both apply to `W<Int>`.
            //
            // Dead code that also silently defeats overlap checking is the worst
            // of the two, which is why Rust forbids the position outright.
            //
            // `E0900` and not a new code, deliberately. The construct is not
            // meaningless — it becomes implementable the moment impl selection
            // can invert a projection — and every other "the parser accepts it,
            // no machinery implements it" rejection in this checker is `E0900`
            // (generic traits, `where` on trait methods, bounds on an associated
            // type, module-qualified type paths). A dedicated code would assert a
            // *permanent* illegality this project has not decided, and the design
            // doc has already been falsified twice on exactly this subject.
            if self_ty.has_assoc() {
                let tystr = display_ty(&self_ty, self.defs);
                self.diagnostics.push(
                    Diagnostic::error(
                        "E0900",
                        format!(
                            "an associated-type projection in an impl's self type \
                             (`{tystr}`) is not supported yet"
                        ),
                    )
                    .with_primary_label(block.ty.span, "not supported yet")
                    .with_note(
                        "an impl's type arguments are recovered by matching its self type \
                         against a concrete one, which cannot invert a projection, so this \
                         impl could never be selected — and overlap checking cannot see \
                         through the projection either, so it would not conflict with an \
                         impl that does apply"
                            .to_string(),
                    )
                    .with_note(
                        "the Phase 1 MVP compiler supports a subset of Nova; \
                         this feature arrives in a later milestone"
                            .to_string(),
                    ),
                );
                continue;
            }
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

            // From here on, `Self` inside this impl means this impl's self type.
            // Recorded by item index too, so `check_method` can restore it when
            // it compiles a body in a later pass.
            let imp_self = ImplSelf {
                ty: self_ty.clone(),
                trait_id,
            };
            self.impl_selves.insert(item_index, imp_self.clone());
            self.impl_self = Some(imp_self);

            let impl_count = block.generics.len() as u32;
            // Associated-type bindings (`type Item = T`), keyed by the
            // associated type's own `DefId` — the same key
            // `TraitDef::assoc_types` uses, so a projection resolved to
            // `Ty::Assoc { assoc, .. }` looks its binding up directly.
            //
            // The bound type is converted in the impl's own generic scope, so
            // it may contain the impl's `Param(k)`; normalization substitutes
            // the impl's type arguments before using it. Converting it here
            // rather than lazily also means `type Item = Nope` reports its own
            // "cannot find type" once, at the binding, and not once per use.
            //
            // Whether the *set* of bindings matches the trait's is
            // `check_impl_conformance`'s job, beside the method set. All that
            // happens here is resolving each name to its `DefId`: a name the
            // trait does not declare has no `DefId` to be keyed under, so it is
            // dropped from this list and reported there.
            let mut assoc_bindings: Vec<(DefId, Ty)> = Vec::new();
            // Span of each kept binding's name, aligned with `assoc_bindings`,
            // so the cycle check below can point at the offending binding rather
            // than at the impl header.
            let mut assoc_spans: Vec<Span> = Vec::new();

            // Associated constants are parsed but not implemented. `ImplBlock`
            // carries them in a third parallel vector beside `functions` and
            // `assoc_types`, and until this check existed **nothing in the
            // workspace read it** — so `impl K { const LIMIT: Int = 99 }`
            // compiled, ran, and silently dropped the constant, after which
            // `K::LIMIT` reported `no variant \`LIMIT\` on type \`K\``: a
            // message about a construct the user did not write.
            //
            // Refusing it is the honest state. Accepting a declaration and
            // discarding it is strictly worse than rejecting it, because the
            // program looks correct and the constant simply is not there. This
            // is deliberately *not* wired into the top-level `const` path
            // (which compiles a constant as a zero-argument function): doing
            // that is a real feature with its own questions — visibility,
            // whether `Self` may appear in the type, cycle detection across
            // impls — and it belongs in an increment that answers them.
            for c in &block.consts {
                // `unsupported` appends "are not supported yet", so this reads
                // as a plural phrase.
                self.unsupported(
                    c.name.span,
                    &format!("associated constants like `{}`", c.name.value),
                );
            }
            for b in &block.assoc_types {
                let ty = self.convert_ty(&b.ty, &impl_generics, &impl_bounds);
                let resolved = trait_id.and_then(|tid| self.find_assoc_type(tid, &b.name.value));
                let Some(assoc) = resolved else {
                    // An inherent impl has no trait, so it can never bind an
                    // associated type — and `check_impl_conformance` does not
                    // run for it, so this is the only place that can say so.
                    if trait_id.is_none() {
                        self.error(
                            "E0071",
                            format!(
                                "associated type `{}` cannot be bound by an inherent impl, \
                                 which implements no trait",
                                b.name.value
                            ),
                            b.name.span,
                        );
                    }
                    continue;
                };
                // Two bindings for one associated type would leave which one
                // normalization picks up to list order. Keep the first and
                // reject the rest, the way a duplicate generic parameter is
                // handled (`check_duplicate_generics`, same code).
                if assoc_bindings.iter().any(|(d, _)| *d == assoc) {
                    self.error(
                        "E0403",
                        format!(
                            "the associated type `{}` is already bound by this impl",
                            b.name.value
                        ),
                        b.name.span,
                    );
                    continue;
                }
                assoc_bindings.push((assoc, ty));
                assoc_spans.push(b.name.span);
            }
            self.check_assoc_binding_cycles(&mut assoc_bindings, &assoc_spans, &self_ty);
            let mut methods = Vec::new();
            for (mi, f) in block.functions.iter().enumerate() {
                let Some(def_id) = impl_methods.get(&(item_index, mi)).copied() else {
                    continue;
                };
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
                let (mut params, ret) =
                    self.method_sig_parts(&f.params, &f.return_ty, &scope, &bounds);
                // An async method's declared return type is its future's
                // OUTPUT — the spec's own `pub async fn join(self) -> T`
                // reads `T`, not `Future<T>` — so the signature callers and
                // `check_impl_method_signatures` compare against is
                // `Future<ret>`, the same wrapping `collect_signatures` does
                // for a free async fn. `check_fn_body` unwraps it back to
                // `ret` to check the method's body.
                let ret = if f.is_async {
                    Ty::Future(Box::new(ret))
                } else {
                    ret
                };
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
            // methods that lack defaults, and nothing foreign. Only the *set* of
            // items is decided here — each method's signature is compared by
            // `check_impl_method_signatures` after the loop, because that
            // comparison normalizes associated-type projections and so needs the
            // finished impl table.
            if let Some(tid) = trait_id {
                self.check_impl_conformance(tid, &methods, &block.assoc_types, block.ty.span);
            }

            self.impls.push(hir::ImplInfo {
                trait_id,
                self_head,
                self_ty,
                generics: block.generics.len() as u32,
                bounds: impl_bounds,
                methods,
                assoc_bindings,
            });
            impl_spans.push(block.ty.span);
        }
        // No impl is in scope for `collect_signatures` / `collect_externs`,
        // which run next.
        self.impl_self = None;

        // Ordered first among the three post-collection passes so that an
        // `E0072` about a method signature still precedes `check_supertrait_impls`'s
        // own `E0072`, as it did when the signature check ran inside the loop.
        self.check_impl_method_signatures(&impl_spans);
        self.check_impl_coherence(&impl_spans);
        self.check_supertrait_impls(&impl_spans);
    }

    /// Report `E0077` for an associated type this impl binds in terms of
    /// itself, and poison the offending binding to `Ty::Error`.
    ///
    /// One binding may legitimately name another of the same impl's associated
    /// types — `type A = Self::B` with `type B = Int` is legal, and it is the
    /// reason [`hir::normalize_ty`] re-normalizes its own result. That is
    /// precisely the walk that never terminates on `type Item = Self::Item`, or
    /// on a mutual `A = Self::B` / `B = Self::A`, so the cycle has to be
    /// rejected where it is created. `normalize_ty`'s step guard is the
    /// backstop for a cycle built by some path this does not see, not a
    /// substitute for this check: a hang is worse than any diagnostic, and a
    /// compiler-limit message is the wrong thing to show a user who wrote a
    /// two-line cycle.
    ///
    /// **Poisoning to `Ty::Error` rather than dropping the binding** keeps the
    /// bound *set* complete, so `check_impl_conformance` does not add a spurious
    /// `E0070` for a binding that is present but bad. The cycle is already
    /// reported, so this is suppression of a *second* error, never silence.
    ///
    /// **Poisoning is not self-suppressing, and this comment used to claim it
    /// was.** It said `Ty::Error` "unifies with anything, so no use of the
    /// projection cascades either". That is true of `InferCtx::unify`, where
    /// `Ty::Error` absorbs — and false of every consumer that compares two `Ty`s
    /// with the derived `PartialEq`, where an `Error` on one side *forces* a
    /// mismatch. `check_impl_method_signatures` is such a consumer, and it
    /// reported `method `get` returns `Int` but trait `It` declares `{error}``
    /// after this very check had already explained the cycle. It now skips a
    /// comparison whose either side [`hir::Ty::has_error`]; the suppression has
    /// to live at each `PartialEq` consumer, because poisoning cannot do it from
    /// here.
    fn check_assoc_binding_cycles(
        &mut self,
        bindings: &mut [(DefId, Ty)],
        spans: &[Span],
        self_ty: &Ty,
    ) {
        // Edges: this impl's associated type `d` refers to `e` when its bound
        // type projects onto `Self::e` anywhere inside it.
        let mut edges: FxHashMap<DefId, Vec<DefId>> = FxHashMap::default();
        for (d, ty) in bindings.iter() {
            let mut refs = Vec::new();
            collect_self_projections(ty, self_ty, &mut refs);
            edges.insert(*d, refs);
        }
        // `zip` rather than an index: the two are pushed in the same step by
        // `collect_impls`, and zipping makes a misalignment inexpressible rather
        // than something a fallback span would paper over.
        for ((d, ty), &span) in bindings.iter_mut().zip(spans) {
            if !reaches_self(*d, &edges) {
                continue;
            }
            let name = self.defs.def(*d).name.clone();
            *ty = Ty::Error;
            self.diagnostics.push(
                Diagnostic::error(
                    "E0077",
                    format!("the associated type `{name}` is defined in terms of itself"),
                )
                .with_primary_label(span, "cyclic associated type")
                .with_note(
                    "an associated type's binding may name another of this impl's \
                     associated types, but the references must bottom out in a \
                     concrete type"
                        .to_string(),
                ),
            );
        }
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

    /// Verify a trait impl provides exactly the trait's *set* of items: every
    /// required method, every declared associated type, and nothing foreign in
    /// either category.
    ///
    /// **Set membership only.** Whether each provided method's *signature*
    /// agrees with the trait's declaration is
    /// [`Checker::check_impl_method_signatures`], which cannot run from here —
    /// see its doc comment for why. The split follows the diagnostic codes
    /// exactly: `E0070` (missing a required item) and `E0071` (not a member of
    /// the trait) are decided here, from names alone; `E0072` (the item exists
    /// on both sides but its shape disagrees) is decided there, from types.
    ///
    /// The *whole* per-method comparison moved, not only the two checks that
    /// compare a type. The receiver, generic-count and generic-bound checks need
    /// nothing from the impl table and could have stayed — but they are the early
    /// `continue`s that decide whether the type comparison runs at all, so
    /// splitting the loop would mean duplicating each of those guards on both
    /// sides of the split and keeping the two copies in agreement forever.
    ///
    /// `provided_assoc` is the impl's associated-type bindings as written, by
    /// name and span: this is the set-membership check in both directions, so
    /// it needs the names the trait *doesn't* declare, which have no `DefId`
    /// and are therefore absent from `ImplInfo::assoc_bindings`.
    fn check_impl_conformance(
        &mut self,
        trait_id: DefId,
        provided: &[(String, DefId)],
        provided_assoc: &[ast::AssocTypeBinding],
        span: Span,
    ) {
        let Some(tr) = self.traits.iter().find(|t| t.def_id == trait_id).cloned() else {
            return;
        };
        for (name, _) in provided {
            if !tr.methods.iter().any(|m| &m.name == name) {
                self.error(
                    "E0071",
                    format!("method `{name}` is not a member of trait `{}`", tr.name),
                    span,
                );
            }
        }
        // An associated type the trait never declared: the E0071 "not a member
        // of the trait" case, the same as a foreign method above, and reported
        // at the binding itself rather than at the impl's self type because
        // there is a precise span for it.
        for b in provided_assoc {
            if !tr.assoc_types.iter().any(|(n, _)| n == &b.name.value) {
                self.error(
                    "E0071",
                    format!(
                        "associated type `{}` is not a member of trait `{}`",
                        b.name.value, tr.name
                    ),
                    b.name.span,
                );
            }
        }
        let missing: Vec<String> = tr
            .methods
            .iter()
            .filter(|m| m.default_def.is_none() && !provided.iter().any(|(n, _)| n == &m.name))
            .map(|m| format!("`{}`", m.name))
            .collect();
        // Associated types have no defaults in this increment, so every
        // declared one is required: the filter is simply "not provided", with
        // no `default_def.is_none()` counterpart to exempt anything. If
        // defaults are ever added, this is the line that must learn about them.
        let missing_assoc: Vec<String> = tr
            .assoc_types
            .iter()
            .filter(|(n, _)| !provided_assoc.iter().any(|b| &b.name.value == n))
            .map(|(n, _)| format!("`{n}`"))
            .collect();
        // One diagnostic for both categories — but naming only the ones
        // actually missing, so a missing `Item` never reads as a missing
        // *method*. With only methods missing this is byte-identical to the
        // message this site emitted before associated types existed.
        let mut kinds: Vec<String> = Vec::new();
        if !missing.is_empty() {
            kinds.push(format!("method(s): {}", missing.join(", ")));
        }
        if !missing_assoc.is_empty() {
            kinds.push(format!("associated type(s): {}", missing_assoc.join(", ")));
        }
        if !kinds.is_empty() {
            self.error(
                "E0070",
                format!(
                    "impl of trait `{}` is missing {}",
                    tr.name,
                    kinds.join(" and ")
                ),
                span,
            );
        }
    }

    /// Verify each impl method's signature against the trait's declaration,
    /// with `Self` bound to the impl's self type and **both sides normalized**
    /// — normalization seam 2 of three (design doc §4.1, and its named risk 2).
    ///
    /// Without this check the call site programs against the trait's signature
    /// while codegen dispatches to the impl's method, and a mismatch
    /// miscompiles or is memory-unsafe. Every diagnostic here is `E0072`.
    ///
    /// **Both sides are normalized, not just the trait's.** The trait's
    /// declaration may spell a type as `Self::Item`; an impl may spell the same
    /// type either way (design doc §5.1 pins that both are accepted). Normalize
    /// only the trait side and the `-> Self::Item` spelling breaks, since the
    /// impl's stored signature still holds the projection; normalize only the
    /// impl side and the `-> T` spelling stays broken. Substituting is not
    /// enough on its own either: it turns `Self::Item` into
    /// `Assoc { on: W<Param(0)> }`, which is still a projection.
    ///
    /// **Runs after the whole impl table is built**, beside
    /// [`Checker::check_impl_coherence`] and
    /// [`Checker::check_supertrait_impls`], rather than from
    /// [`Checker::check_impl_conformance`] where it used to live — and for the
    /// same reason `check_supertrait_impls` was moved out of conformance.
    /// `collect_impls` calls conformance ten lines *before* it pushes the impl
    /// being checked, so a `normalize` there would see neither this impl's own
    /// bindings nor those of any impl written below it. Pushing the `ImplInfo`
    /// above the conformance call fixes the first half only: normalization
    /// consults the whole table, and a projection can need a *different*
    /// impl's binding. It is reachable from ordinary source, because
    /// `collect_traits` puts a trait's expanded supertraits in `sig_bounds[0]`,
    /// so `Self::Elem` inside `trait Ext: Base` is `Base`'s associated type and
    /// substituting `Self` yields a projection whose binding lives in
    /// `impl Base for W` — which may be written either side of
    /// `impl Ext for W`. Nova has no declaration-order rule for impls, and
    /// `conformance_resolves_a_projection_bound_by_a_later_declared_impl` pins
    /// that it has none here either. Measured under a hoisted push: that test's
    /// `impl Ext` first ordering still reports `E0072`, and it is the *only*
    /// test in the suite that separates the two designs.
    ///
    /// **What moving this changed with no test to pin it: diagnostic order.**
    /// Every `E0070`/`E0071` for every impl now precedes every `E0072` about a
    /// method signature, where before both came from one call per impl in source
    /// order. Nothing sorts diagnostics between here and the terminal, so the
    /// order is user-visible: an impl on line 5 with a bad signature and one on
    /// line 7 missing a method now report line 7 first. No test asserts a code
    /// *sequence* spanning both bands, so this is deliberate and unpinned rather
    /// than accidentally green — pinning it would freeze an ordering that is not
    /// itself a designed property.
    fn check_impl_method_signatures(&mut self, spans: &[Span]) {
        // The table is cloned rather than walked in place: `normalize` and
        // `error` both need `&mut self`, and `normalize` — reading `self.impls`
        // — is the entire point of this pass. `check_supertrait_impls` avoids the
        // clone by deferring its diagnostics to a second loop; that is not open
        // here, because `normalize` itself needs `&mut self` to report `E0078`.
        let impls = self.impls.clone();
        for (imp, &span) in impls.iter().zip(spans) {
            let Some(trait_id) = imp.trait_id else {
                continue;
            };
            let Some(tr) = self.traits.iter().find(|t| t.def_id == trait_id).cloned() else {
                continue;
            };
            for (name, def_id) in &imp.methods {
                let Some(trait_method) = tr.methods.iter().find(|m| &m.name == name) else {
                    // A method the trait does not declare: reported as `E0071`
                    // by `check_impl_conformance`, and there is no trait
                    // signature here to compare it against.
                    continue;
                };
                let Some(impl_sig) = self.sigs.get(def_id).cloned() else {
                    continue;
                };
                let impl_count = imp.generics;
                // The receiver must agree. Nothing below can catch a
                // disagreement: neither `params` list stores `self`, so an impl
                // that adds or drops the receiver still compares equal
                // parameter-for-parameter and return-type-wise. Yet a call site
                // programs against the trait's signature while codegen
                // dispatches to the impl's function, so the two differ by
                // exactly one leading argument — Cranelift rejects the module
                // ("mismatched argument count") and the compiler ICEs on source
                // that `nova check` accepted.
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
                // The receiver's *mutability* must agree too, and nothing else
                // catches it either: `method_sig_parts` strips a `self`
                // parameter whether or not it is `mut`, so neither `params` list
                // records it, and the parameter and return comparisons below
                // therefore pass a disagreeing pair. Yet the two halves are read
                // by *different* consumers — `emit_trait_call` gates the call
                // site on the trait's flag while `check_fn_body` gates the body
                // on the impl's own `mut` — so a disagreement means the
                // receiver's mutability requirement is decided by whichever
                // table the reader happened to consult. Trait `mut self` with an
                // impl `self` demands `let mut` at every call site for a method
                // that cannot mutate; the reverse lets an impl mutate through a
                // binding no caller ever granted the permission to.
                //
                // Deliberately **no `continue`**, unlike the receiver-*presence*
                // mismatch above. That one `continue`s because both parameter
                // lists are then misaligned by one and every later comparison is
                // noise; a `mut` disagreement misaligns nothing, so the rest of
                // the signature is still worth checking in the same pass.
                let impl_mut_self = self.mut_self.contains(def_id);
                if impl_mut_self != trait_method.mut_self {
                    let (want, got) = if trait_method.mut_self {
                        ("a `mut self` receiver", "a plain `self` receiver")
                    } else {
                        ("a plain `self` receiver", "a `mut self` receiver")
                    };
                    self.error(
                        "E0072",
                        format!(
                            "method `{name}` has {got} but trait `{}` declares {want}",
                            tr.name
                        ),
                        span,
                    );
                }
                // The impl method's own generics = its total minus the impl's,
                // and must match the trait method's generic count.
                let impl_method_generics = impl_sig.generics.saturating_sub(impl_count);
                if impl_method_generics != trait_method.generics {
                    self.error(
                        "E0072",
                        format!(
                            "method `{name}` has {impl_method_generics} generic parameter(s) \
                             but trait `{}` declares {}",
                            tr.name, trait_method.generics
                        ),
                        span,
                    );
                    continue;
                }
                // Each method generic must carry exactly the trait's declared
                // bounds — neither dropped nor added. The impl method's own
                // generics live at `impl_sig.bounds[impl_count + k]`, aligned
                // with the trait method's `bounds[k]`. Without this the trait
                // signature the call site programs against is not the contract
                // the impl honors: an impl that drops a bound accepts calls the
                // trait forbids (unsound), and one that adds a bound rejects
                // trait-valid calls only later, at monomorphization.
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
                                "generic parameter {} of method `{name}` has bound(s) \
                                 {got_str} but trait `{}` declares {want_str}",
                                k + 1,
                                tr.name,
                            ),
                            span,
                        );
                    }
                }
                // Map the trait method's Param space into the impl's: `Self`
                // (Param(0)) -> the impl self type; method generic k
                // (Param(1+k)) -> the impl method's own generic at
                // Param(impl_count + k).
                let mut subst = Vec::with_capacity(1 + trait_method.generics as usize);
                subst.push(imp.self_ty.clone());
                for k in 0..trait_method.generics {
                    subst.push(Ty::Param(impl_count + k));
                }
                // `impl_sig.params[0]` is the `self` receiver — skip it and
                // compare the declared parameters (and the return type) against
                // the trait method's, which `method_sig_parts` also stores
                // without `self`. An associated function has no receiver stored,
                // so there is nothing to skip (and `[1..]` would panic on its
                // empty parameter list).
                let impl_params: &[Ty] = if self.selfless.contains(def_id) {
                    &impl_sig.params
                } else {
                    impl_sig.params.get(1..).unwrap_or_default()
                };
                if impl_params.len() != trait_method.params.len() {
                    self.error(
                        "E0072",
                        format!(
                            "method `{name}` has {} parameter(s) but trait `{}` declares {}",
                            impl_params.len(),
                            tr.name,
                            trait_method.params.len()
                        ),
                        span,
                    );
                    continue;
                }
                // Normalized on both sides, and *after* the count check so a
                // wrong arity is still reported as an arity error rather than as
                // a per-parameter mismatch — `zip` truncates to the shorter list,
                // so with the count check gone a missing parameter reports
                // *nothing at all*.
                //
                // Normalized unconditionally, with no `Ty::has_assoc` guard of
                // the kind `Checker::instantiate` carries. There, the guard
                // preserves the exact pre-existing `subst`-only path for a
                // projection-free call; here `normalize_ty` on a projection-free
                // type is a structural clone and returns an equal value, so a
                // guard would be unobservable *and* unpinnable, and would only
                // add a second thing to keep true.
                let expected: Vec<Ty> = trait_method
                    .params
                    .iter()
                    .map(|p| {
                        let p = p.subst(&subst);
                        self.normalize(&p, span)
                    })
                    .collect();
                let got_params: Vec<Ty> = impl_params
                    .iter()
                    .map(|p| self.normalize(p, span))
                    .collect();
                for (i, (got, want)) in got_params.iter().zip(expected.iter()).enumerate() {
                    // A poisoned side is not a mismatch — see `signatures_are
                    // _not_compared_when_either_side_is_already_poisoned`.
                    if got.has_error() || want.has_error() {
                        continue;
                    }
                    if got != want {
                        self.error(
                            "E0072",
                            format!(
                                "parameter {} of method `{name}` has type `{}` but trait \
                                 `{}` declares `{}`",
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
                let expected_ret = self.normalize(&expected_ret, span);
                let impl_ret = self.normalize(&impl_sig.ret, span);
                // `Ty::Error` on either side means a diagnostic about *that* type
                // was already reported, so comparing is worse than useless here:
                // `Ty` derives `PartialEq` with no `Error` absorption, so a
                // poisoned side does not merely fail to help — it *forces* a
                // mismatch, and the follow-on `E0072` renders it as `{error}`,
                // which names nothing a user can act on. This is the exact
                // opposite of `Ty::Error`'s behaviour at `unify`, where it
                // absorbs; the two consumers of the same sentinel need opposite
                // handling, and only `unify` got it for free.
                //
                // Transitive (`has_error`) rather than `== Ty::Error`: an impl
                // binding an unresolvable type makes the trait's
                // `Option<Self::Item>` normalize to `Option<{error}>`, so a
                // top-level check would still leak `{error}` through a wrapper.
                //
                // Suppression of a *second* error only. Both routes to a poisoned
                // side — `E0077` for a cyclic binding and `E0001` for an
                // unresolvable one — have already reported the real problem.
                if impl_ret.has_error() || expected_ret.has_error() {
                    continue;
                }
                if impl_ret != expected_ret {
                    self.error(
                        "E0072",
                        format!(
                            "method `{name}` returns `{}` but trait `{}` declares `{}`",
                            display_ty(&impl_ret, self.defs),
                            tr.name,
                            display_ty(&expected_ret, self.defs),
                        ),
                        span,
                    );
                }
            }
        }
    }

    /// Convert a method's non-`self` parameter types and return type using
    /// `scope` (which maps `Self`/generics to `Param` indices) and `bounds`
    /// (indexed the same way — see `convert_ty`).
    fn method_sig_parts(
        &mut self,
        params: &[ast::Param],
        ret: &Option<Spanned<ast::Type>>,
        scope: &FxHashMap<String, u32>,
        bounds: &[Vec<DefId>],
    ) -> (Vec<Ty>, Ty) {
        let converted = params
            .iter()
            .filter(|p| p.name.value != "self")
            .map(|p| self.convert_ty(&p.ty, scope, bounds))
            .collect();
        let ret_ty = ret
            .as_ref()
            .map(|t| self.convert_ty(t, scope, bounds))
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
            self.check_duplicate_generics(&f.generics, "function");
            let generics = generic_scope(&f.generics);
            let mut bounds = self.resolve_bounds(&f.generics);
            self.apply_where(&mut bounds, &f.where_clause, &generics);
            self.expand_bounds(&mut bounds);
            let params = f
                .params
                .iter()
                .map(|p| self.convert_ty(&p.ty, &generics, &bounds))
                .collect();
            let ret = f
                .return_ty
                .as_ref()
                .map(|t| self.convert_ty(t, &generics, &bounds))
                .unwrap_or(Ty::Unit);
            // An async fn's declared return type is its future's OUTPUT, per
            // the spec's own signatures (`pub async fn join(self) -> T`), so
            // the signature every caller sees is `Future<ret>`. `check_fn_body`
            // unwraps it back to `ret` to check this fn's own body.
            let ret = if f.is_async {
                Ty::Future(Box::new(ret))
            } else {
                ret
            };
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
            let ret = self.convert_ty(&c.ty, &FxHashMap::default(), &[]);
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
                    let ty = self.convert_ty(&p.ty, &empty, &[]);
                    self.require_ffi_safe(&ty, p.ty.span, false);
                    ty
                })
                .collect();
            let ret = match &sig.return_ty {
                Some(t) => {
                    let ty = self.convert_ty(t, &empty, &[]);
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
            let idx = match self.convert_ty(&wb.ty, scope, bounds) {
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

    /// Convert an AST type annotation to a `Ty`, resolving names. `bounds` is
    /// the trait-bound list of each of `generics`' parameters, indexed the
    /// same way (`bounds[i]` bounds whichever name maps to `i` in `generics`,
    /// mirroring `FnCtx.param_bounds`) — needed to resolve a two-segment path
    /// like `I::Item` against the traits `I` is actually bounded by.
    fn convert_ty(
        &mut self,
        ty: &Spanned<ast::Type>,
        generics: &FxHashMap<String, u32>,
        bounds: &[Vec<DefId>],
    ) -> Ty {
        match &ty.value {
            ast::Type::Path { path, args } => {
                if path.segments.len() == 2 {
                    let base = path.segments[0].value.as_str();
                    let assoc_name = path.segments[1].value.as_str();
                    // A two-segment path is a **projection** when its first
                    // segment names something a projection can be *on*, and a
                    // module-qualified path otherwise. There are exactly two
                    // such things, and they are different mechanisms:
                    //
                    //  * an in-scope generic parameter, i.e. a `generics` key.
                    //    That includes a trait body's `Self`, which
                    //    `self_generic_scope` inserts as the trait's own
                    //    implicit parameter at `Param(0)`. It does *not*
                    //    include a user-written parameter spelled `Self`:
                    //    `E0076` rejects that name outright, so `Self` never
                    //    has two meanings in one scope.
                    //  * `Self` inside an `impl` block, which is not a
                    //    parameter at all but the impl's own self type — a
                    //    possibly compound `W<Param(0)>` with no parameter
                    //    index, and therefore no slot in the by-index `bounds`
                    //    table. Its candidates are not bounds but the single
                    //    trait the impl implements.
                    let by_index = generics.get(base).copied();
                    let in_impl = by_index.is_none() && base == "Self";
                    if by_index.is_some() || (in_impl && self.impl_self.is_some()) {
                        // Nova has no generic associated types: a non-empty
                        // `args` here (`I::Item<Int>`) has nowhere to go, so
                        // it must be flagged rather than silently dropped —
                        // mirroring the two single-segment branches below
                        // (generic parameter, primitive) that already guard
                        // this the same way. Deliberately says only
                        // "`{base}::{assoc_name}`", not "associated type
                        // `{base}::{assoc_name}`": `assoc_name` may not even
                        // be one (`I::Nope<Int>` reports this alongside
                        // resolve_projection's own E0001 for `Nope`, and the
                        // two must not read as contradicting each other).
                        if !args.is_empty() {
                            self.error(
                                "E0012",
                                format!("`{base}::{assoc_name}` takes no type arguments"),
                                ty.span,
                            );
                        }
                        let (on, candidates) = match by_index {
                            // Find the associated type among the traits
                            // bounding this parameter. Searching the bounds
                            // (rather than every trait) is what makes
                            // `I::Item` mean "the Item of the trait I is
                            // bounded by". `expand_bounds` has already folded
                            // supertraits into every entry here, so a bound of
                            // `Ord` also carries `Eq` — `I::Item` resolves
                            // against the transitive bound set as a
                            // consequence of that ordering, not a separate
                            // decision made here.
                            Some(idx) => (
                                Ty::Param(idx),
                                bounds.get(idx as usize).cloned().unwrap_or_default(),
                            ),
                            None => {
                                // `in_impl` and `impl_self` is `Some`, per the
                                // guard above.
                                let imp = match self.impl_self.clone() {
                                    Some(imp) => imp,
                                    None => return Ty::Error,
                                };
                                match imp.trait_id {
                                    // Supertrait-expanded, so `Self::Elem` in
                                    // `impl Ext for W` resolves against `Base`
                                    // when `trait Ext: Base` declares nothing
                                    // itself. This is not an extra feature: the
                                    // *trait* side already does it, because
                                    // `collect_traits` seeds `sig_bounds[0]` with
                                    // the trait and then calls `expand_bounds`.
                                    // Without it, `trait Ext: Base { fn peek
                                    // (self) -> Self::Elem }` compiled while
                                    // `impl Ext for W { fn peek(self) ->
                                    // Self::Elem }` — the same signature, echoed
                                    // — was `E0001`, so the one spelling §5.1
                                    // pins as accepted was rejected on the side
                                    // that has to write it.
                                    Some(tid) => (imp.ty, self.with_supertraits(&[tid])),
                                    // An inherent impl implements no trait, so
                                    // no trait declares an associated type for
                                    // this to name. Given its own wording
                                    // rather than `resolve_projection`'s
                                    // empty-candidate message, which says "on
                                    // any bound of `Self`" — an inherent
                                    // impl's `Self` has no bounds, so that
                                    // would describe a lookup that never
                                    // happened.
                                    None => {
                                        let on = display_ty(&imp.ty, self.defs);
                                        self.error(
                                            "E0001",
                                            format!(
                                                "no associated type `{assoc_name}` on `{on}`: \
                                                 an inherent impl implements no trait, so \
                                                 nothing declares one"
                                            ),
                                            ty.span,
                                        );
                                        return Ty::Error;
                                    }
                                }
                            }
                        };
                        return self.resolve_projection(on, base, assoc_name, &candidates, ty.span);
                    }
                    self.unsupported(ty.span, "module-qualified type paths");
                    return Ty::Error;
                }
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
                // `Future<T>` — the one built-in type name with an argument.
                // Handled ahead of the nullary `prim` table below, whose
                // `args.is_empty()` guard means the opposite here.
                if name == "Future" {
                    if args.len() != 1 {
                        self.error(
                            "E0012",
                            format!(
                                "`Future` takes exactly one type argument, found {}",
                                args.len()
                            ),
                            ty.span,
                        );
                        return Ty::Error;
                    }
                    let out = self.convert_ty(&args[0], generics, bounds);
                    return Ty::Future(Box::new(out));
                }
                // Every name matched here must also be in
                // `nova_resolver::RESERVED_TYPE_NAMES`: that list is what
                // turns a user type declared under one of these names into
                // `E0089` instead of silently unreachable. A new nullary
                // built-in added to this table without joining that list
                // would recreate the defect the list exists to close.
                let prim = match name {
                    "Int" => Some(Ty::Int),
                    "Float" => Some(Ty::Float),
                    "Bool" => Some(Ty::Bool),
                    "Char" => Some(Ty::Char),
                    "String" => Some(Ty::String),
                    "Bytes" => Some(Ty::Bytes),
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
                    let converted: Vec<Ty> = args
                        .iter()
                        .map(|a| self.convert_ty(a, generics, bounds))
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
                    .map(|p| self.convert_ty(p, generics, bounds))
                    .collect(),
                ret: Box::new(self.convert_ty(ret, generics, bounds)),
            },
            ast::Type::Tuple(_) => {
                self.unsupported(ty.span, "tuple types");
                Ty::Error
            }
            ast::Type::Ref { .. } | ast::Type::Ptr { .. } => {
                self.unsupported(ty.span, "reference and pointer types");
                Ty::Error
            }
            ast::Type::Array(elem) => Ty::Array(Box::new(self.convert_ty(elem, generics, bounds))),
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

    /// Resolve a two-segment type path `<base>::<assoc_name>` into a projection
    /// on `on`, searching `candidate_traits` for the one that declares
    /// `assoc_name`.
    ///
    /// `on` is the type the projection is *on*, chosen by the caller: a generic
    /// parameter's `Ty::Param(idx)`, or an impl's own (possibly compound) self
    /// type. It is never `Ty::Var` — see the `Ty::Assoc` doc comment on why that
    /// distinction matters to the unifier.
    ///
    /// `candidate_traits` is likewise the caller's: a parameter's
    /// already-supertrait-expanded bounds, or the single trait an impl
    /// implements. `base` is only for rendering the two error messages, and both
    /// of them read the list as bounds — a caller whose candidates are not
    /// bounds must handle its own empty case before calling (see the inherent
    /// impl in `convert_ty`).
    fn resolve_projection(
        &mut self,
        on: Ty,
        base: &str,
        assoc_name: &str,
        candidate_traits: &[DefId],
        span: Span,
    ) -> Ty {
        let candidates: Vec<DefId> = candidate_traits
            .iter()
            .filter_map(|&trait_id| self.find_assoc_type(trait_id, assoc_name))
            .collect();
        match candidates.as_slice() {
            [assoc] => Ty::Assoc {
                on: Box::new(on),
                assoc: *assoc,
            },
            [] => {
                self.error(
                    "E0001",
                    format!("no associated type `{assoc_name}` on any bound of `{base}`"),
                    span,
                );
                Ty::Error
            }
            _ => {
                self.error(
                    "E0015",
                    format!(
                        "ambiguous associated type `{base}::{assoc_name}`: more than one \
                         bound of `{base}` declares it"
                    ),
                    span,
                );
                Ty::Error
            }
        }
    }

    /// The `DefId` of the associated type named `name` declared directly by
    /// trait `trait_id`, if any.
    ///
    /// Searches `self.defs` (the resolver's output) rather than the
    /// incrementally-built `self.traits`: a trait's own method signature can
    /// name its own associated type (`Self::Item` in `Iterator::next`, which
    /// has no default body) before `collect_traits` has pushed *that very
    /// trait's* `hir::TraitDef`, and a bound can equally name a trait
    /// declared later in the file. That is the same ordering hazard
    /// `Checker::supertraits` exists to avoid for supertrait expansion (see
    /// its doc comment) — `self.defs` carries every `DefKind::AssocType` from
    /// resolution, before any of `collect_traits`'s incremental order exists.
    fn find_assoc_type(&self, trait_id: DefId, name: &str) -> Option<DefId> {
        self.defs
            .defs()
            .iter()
            .enumerate()
            .find_map(|(di, d)| match d.kind {
                DefKind::AssocType { trait_def } if trait_def == trait_id && d.name == name => {
                    Some(DefId(di as u32))
                }
                _ => None,
            })
    }

    /// Substitute a call's type arguments into a stored signature type and
    /// resolve any projection the result contains — the call-site half of
    /// normalization seam 1.
    ///
    /// `subst` alone is not enough at a call site. A call's type arguments start
    /// as fresh inference variables that the argument unification then solves, so
    /// a projection on a generic parameter substitutes to `Assoc { on: Var(k) }`,
    /// which has no [`Ty::head`] and cannot be resolved. Applying the current
    /// substitution first turns it into `Assoc { on: <concrete> }`.
    ///
    /// **The `has_assoc` guard is not an optimization, and no test can kill it.**
    /// Applying the substitution early is, as far as I can reason, unobservable —
    /// `unify` walks through variables anyway, `show` applies before rendering,
    /// and `finalize_function` applies again at the end — and the full suite is
    /// green with the guard removed, so nothing pins it. It is here because that
    /// reasoning, not the compiler, is what makes the unguarded version safe: with
    /// the guard, a call whose signature has no projection takes exactly the plain
    /// `subst` path it took before this task, and the reasoning only has to hold
    /// for the projection case. Deliberate, unpinned, and recorded as such rather
    /// than left to look like a mutation that got away.
    ///
    /// This is also where the design's one admitted hole shows: the variable has
    /// to be *already solved*, which for a call means the argument that
    /// determines it must come first. `fn f<I: It>(y: I::Item, x: I)` still fails
    /// — see the design doc §4.2, whose claim that `Assoc { on: Var(_) }` is
    /// unreachable holds for receivers but not for this shape.
    ///
    /// # One known site that should call this and does not
    ///
    /// [`Checker::emit_inherent_call`] substitutes an inherent method's
    /// parameter types (`let expected = param.subst(&type_args)`) and its return
    /// type (`let ret = sig.ret.subst(&type_args)`) with **no** normalization,
    /// although it has already unified the receiver against the impl's self type
    /// a few lines above — so the projection's root is concrete by then and this
    /// helper's precondition is met. [`Checker::emit_trait_call`] routes the same
    /// two positions through here. The asymmetry rejects a valid program:
    ///
    /// ```text
    /// record W<T> { v: T }
    /// impl<I: Iterator> W<I> { fn echo(self, d: I::Item) -> I::Item { d } }
    /// // with `Cur: Iterator, Item = Bool` and `w: W<Cur>`:
    /// //   w.echo(true)
    /// //   => error[E0010]: argument has type `Bool` but `Cur::Item` was expected
    /// ```
    ///
    /// Measured, both directions: routing those two lines through `instantiate`
    /// makes that program compile and run, with the whole workspace suite still
    /// green and every gate configuration passing; reverting brings the `E0010`
    /// straight back. **Left unfixed on purpose.** It is a behaviour
    /// change — programs that are rejected today would start compiling — so it
    /// needs its own test and its own review, and it is *not* a defect this
    /// increment introduced: it needs only an impl-block bound plus Phase 2.2c
    /// associated types, no feature from the record-parameter-bounds work.
    ///
    /// This is recorded here because the enumeration of raw-`subst` sites
    /// reachable by a projection — the thing the 2.2c design doc's own
    /// correction says to consult *instead of* a seam count — was believed
    /// complete at five and is not. `emit_assoc_call` has the same textual
    /// shape but is a different case: with no receiver there is nothing to pin
    /// the impl's parameters, so it fails as `?N::Item` (this helper's admitted
    /// hole above), not as a missing normalization.
    fn instantiate(&mut self, ty: &Ty, args: &[Ty], icx: &InferCtx, span: Span) -> Ty {
        let ty = ty.subst(args);
        if !ty.has_assoc() {
            return ty;
        }
        self.normalize(&icx.apply(&ty), span)
    }

    /// Resolve the associated-type projections in `ty` through the impl table —
    /// normalization seam 1 of three (design doc §4.1).
    ///
    /// A thin wrapper: [`hir::normalize_ty`] holds all of the logic, over a plain
    /// `&[hir::ImplInfo]` slice, so that monomorphization can call the identical
    /// code without reaching through a half-built `Checker`. All this adds is the
    /// diagnostic, which `nova-hir` cannot emit.
    ///
    /// **Precondition: `collect_impls` has returned.** It fills `self.impls`, and
    /// it runs before `collect_signatures` and before any body is checked, so every
    /// seam that exists today sees the complete table. Nothing enforces this, and
    /// it is easy to get wrong in one specific way: `collect_impls` pushes each
    /// `ImplInfo` at the *end* of its loop body, ten lines after it calls
    /// `check_impl_conformance`, so a `normalize` added inside conformance cannot
    /// see the bindings of the very impl it is checking. Measured — with both sides
    /// of the conformance return-type comparison normalized, `impl<T> It for W<T>
    /// { type Item = T  fn get_item(self) -> T }` still reports `E0072`.
    ///
    /// **This comment used to prescribe hoisting the push above the conformance
    /// call. Do not do that — it is measured to be wrong.** It fixes the case
    /// above and leaves a worse one: normalization consults the *whole* impl
    /// table, so a binding that resolves through a **later-declared** impl would
    /// still fail, which is a declaration-order dependency Nova deliberately does
    /// not have for impls. Exactly one test in the suite separates the two
    /// designs, and both of the obvious hand-written tests pass under the wrong
    /// one. The seam was instead added as a separate pass that runs once
    /// `collect_impls` has returned — see [`Checker::check_impl_method_signatures`],
    /// which follows [`Checker::check_supertrait_impls`]'s precedent for the same
    /// reason.
    ///
    /// **Never called from `unify`.** The whole design rests on projections being
    /// gone by the time the unifier sees a type; normalizing inside it would give
    /// the unifier the impl-table dependency it deliberately does not have.
    fn normalize(&mut self, ty: &Ty, span: Span) -> Ty {
        match hir::normalize_ty(ty, &self.impls) {
            Ok(t) => t,
            Err(hir::NormalizeOverflow { at, limit }) => {
                // Reported, not silently swallowed: `Ty::Error` unifies with
                // anything, so returning it without a diagnostic would turn a
                // hang into a wrong answer.
                //
                // **`E0078` is reachable from ordinary source**, and an earlier
                // version of this comment said the opposite ("reaching here means
                // some other path built a projection that does not converge — a
                // compiler defect"), which would send a maintainer hunting an ICE
                // that does not exist. `E0077` catches every cyclic *binding* and
                // poisons it, but a binding chain that is long or wide is not a
                // cycle, so `E0077` never sees it. Measured, both on programs
                // `nova check` otherwise accepts: a linear chain of 64 links
                // (`ok` at 63, `E0078` at 64), and `type A(k) =
                // Pair<Self::A(k+1), Self::A(k+1)>`, which resolves to a
                // `2^k`-node type.
                //
                // Those two are different failures with different fixes, so the
                // message distinguishes them rather than quoting one number for
                // both. Both name the type *this call* was asked about — which is
                // what the user wrote — not the intermediate the walk ran out on.
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
                self.error(
                    "E0078",
                    format!(
                        "could not resolve the associated types in `{}`: {}",
                        display_ty(&at, self.defs),
                        detail,
                    ),
                    span,
                );
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
        // A free function is not inside any impl, so `Self::Item` in its body
        // is a module-qualified path. Cleared rather than assumed, because
        // `check_method` may have run first and left an impl in scope.
        self.impl_self = None;
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
                    // Neither has a body to check: a required method has none,
                    // and `loc.method_index` never actually points at an
                    // associated type — the resolver only ever creates a
                    // `MethodOwner::TraitDefault` `Def` for a `Provided` item —
                    // but the match must stay exhaustive over `TraitItem`.
                    TraitItem::Required(_) | TraitItem::AssocType { .. } => return None,
                }
            }
        };
        // `Self` inside an impl method's body means the impl's self type, the
        // same as in its signature; inside a trait default body it is the
        // trait's own implicit `Param(0)`, which `self_generic_scope` puts in
        // `generics` below instead.
        self.impl_self = match loc.owner {
            MethodOwner::Impl => self.impl_selves.get(&loc.item_index).cloned(),
            MethodOwner::TraitDefault => None,
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
        let mut sig = self.sigs.get(&def_id)?.clone();
        // Normalization seam: the declared return type. `impl It for K { type
        // Item = Int  fn get(self) -> Self::Item { 1 } }` writes `K::Item` where
        // the body produces `Int`, and the unifier cannot decide a projection
        // against a concrete type.
        //
        // Done once, here, rather than at the `unify` below, because `sig.ret`
        // is read three times for the same contract — `fcx.ret_ty` (which every
        // `return e` in the body checks against), this unify, and
        // `hir::Function::ret_ty` — and normalizing only one of them would let
        // `return 1` and a trailing `1` disagree about the same signature.
        //
        // `sig.params` for the same reason: each becomes a local's type below, so
        // `fn put(self, x: Self::Item) -> Int { x + 1 }` would type `x` as an
        // unresolved projection and reject every operation on it.
        sig.ret = self.normalize(&sig.ret, f.name.span);
        let params: Vec<Ty> = sig
            .params
            .iter()
            .map(|p| self.normalize(p, f.name.span))
            .collect();
        sig.params = params;
        let name = self.defs.def(def_id).name.clone();
        // `sig.ret` is the fn's *signature* type — already `Future<output>`
        // for an async fn, wrapped by `collect_signatures`/`collect_impls`.
        // The BODY produces the output directly (`async fn f() -> Int { 1 }`
        // has a body of type `Int`, not `Future<Int>`), so of the three
        // same-contract reads the comment above describes, the two that face
        // the body — `fcx.ret_ty` and this function's own unify below — must
        // name `body_ret_ty`, not `sig.ret`; only `hir::Function::ret_ty`
        // (the external-facing signature) keeps the wrapped type.
        let body_ret_ty = if f.is_async {
            match &sig.ret {
                Ty::Future(out) => (**out).clone(),
                // Unreachable by construction: `collect_signatures` and
                // `collect_impls` wrap an async fn/method's `ret` in
                // `Ty::Future` before it ever reaches `self.sigs`, and
                // `normalize_within`'s `Future` arm recurses into the
                // payload without changing the top-level constructor. Matched
                // defensively rather than unwrapped, so a future change that
                // breaks the invariant degrades instead of panicking in this
                // library path — the same discipline `mir_ty` documents for
                // its own should-be-impossible arm.
                other => other.clone(),
            }
        } else {
            sig.ret.clone()
        };
        let mut fcx = FnCtx {
            icx: InferCtx::default(),
            locals: Vec::new(),
            scopes: vec![FxHashMap::default()],
            generics,
            param_bounds: sig.bounds.clone(),
            ret_ty: body_ret_ty.clone(),
            loop_depth: 0,
            in_async: f.is_async,
            pending_closures: Vec::new(),
        };
        for (p, ty) in f.params.iter().zip(sig.params.iter()) {
            fcx.new_local(p.name.value.clone(), ty.clone(), p.is_mut, p.name.span);
        }

        let body = self.check_block(&mut fcx, &f.body.value, f.body.span);
        if !fcx.icx.unify(&body.ty, &body_ret_ty) {
            let span = body_result_span(&f.body);
            self.error(
                "E0010",
                format!(
                    "`{}` should return `{}` but its body has type `{}`",
                    name,
                    self.show(&body_ret_ty, &fcx),
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
            is_async: f.is_async,
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
        // A top-level `const` is not inside any impl (an impl's own `const`
        // items are never collected at all — `ast::ImplBlock::consts` has no
        // consumer), so nothing here can see an impl's `Self`.
        self.impl_self = None;
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
            // A `const` value has no surface syntax for `async` (`ast::ConstDecl`
            // has no `is_async` field), so `.await` is always illegal in one.
            in_async: false,
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
            is_async: false,
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
            .filter(|&c| reaches_self(c, &edges))
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
                        let annot_ty =
                            self.convert_ty(annot, &fcx.generics.clone(), &fcx.param_bounds);
                        // Normalization seam: a projection written in a `let`
                        // annotation. Needed twice over — for the unify just
                        // below, and because `value.ty = annot_ty` *replaces* the
                        // initializer's type with the annotation, so an
                        // unnormalized projection would become the binding's type
                        // and propagate to every later use of the local.
                        let annot_ty = self.normalize(&annot_ty, annot.span);
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
            ast::Expr::Await(inner) => {
                let e = self.check_expr(fcx, inner);
                if !fcx.in_async {
                    self.diagnostics.push(
                        Diagnostic::error(
                            "E0086",
                            "`.await` is only allowed inside an `async fn`".to_string(),
                        )
                        .with_primary_label(span, "await outside an async function")
                        .with_note(
                            "make the enclosing function `async`, or drive the \
                             future to completion with `block_on`"
                                .to_string(),
                        ),
                    );
                    return error_expr(span);
                }
                let out = match fcx.icx.apply(&e.ty) {
                    Ty::Future(out) => (*out).clone(),
                    Ty::Error => return error_expr(span),
                    other => {
                        self.diagnostics.push(
                            Diagnostic::error(
                                "E0087",
                                format!(
                                    "`.await` expects a future, found `{}`",
                                    display_ty(&other, self.defs)
                                ),
                            )
                            .with_primary_label(inner.span, "not a future"),
                        );
                        return error_expr(span);
                    }
                };
                hir::Expr {
                    kind: hir::ExprKind::Await(Box::new(e)),
                    ty: out,
                    span,
                }
            }
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
                            match self.try_display(fcx, value, e, &other) {
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
            // Normalization seam: a generic function may declare a parameter or
            // its return type as a projection on one of its own type parameters
            // (`fn take<I: It>(x: I, y: I::Item)`), and `type_args` has just been
            // narrowed by the earlier arguments.
            let expected = self.instantiate(param, &type_args, &fcx.icx, a.span);
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
        let ret = self.instantiate(&sig.ret, &type_args, &fcx.icx, span);
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
            Builtin::Println
            | Builtin::Print
            | Builtin::EPrint
            | Builtin::EPrintln
            | Builtin::Panic => " (use string interpolation: \"${value}\")",
            Builtin::StrCmp
            | Builtin::StrHash
            | Builtin::CharToInt
            | Builtin::StrLenChars
            | Builtin::StrChars
            | Builtin::StrFromChars
            | Builtin::StrToUpper
            | Builtin::StrToLower
            | Builtin::TestSelector
            | Builtin::TaskSpawn
            | Builtin::TaskIsDone
            | Builtin::TaskRelease
            | Builtin::TaskDrive
            | Builtin::TaskOutput
            | Builtin::TaskYieldFuture
            | Builtin::TaskSleepFuture
            | Builtin::TaskJoinFuture
            | Builtin::FsReadToString
            | Builtin::FsWriteString
            | Builtin::FsTakeString
            | Builtin::FsLastErrorMessage
            | Builtin::FsTempDir
            | Builtin::FsExists
            | Builtin::FsCreateDir
            | Builtin::FsCreateDirAll
            | Builtin::FsRemoveFile
            | Builtin::FsRemoveDirAll
            | Builtin::FsReadDir
            | Builtin::FsTakeStringArray
            | Builtin::FsKind
            | Builtin::FsRead
            | Builtin::FsTakeBytes
            | Builtin::FsWrite
            | Builtin::BytesLen
            | Builtin::BytesFromString
            | Builtin::BytesIsUtf8
            | Builtin::BytesToStringUnchecked
            | Builtin::BytesAt
            | Builtin::BytesSlice
            | Builtin::BytesConcat
            | Builtin::BytesToInts
            | Builtin::BytesFromInts
            | Builtin::BytesEq => "",
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

    /// `for i in lo..hi { body }` desugars to a counter-driven `while`:
    /// `{ let i = lo; let end = hi; while i < end { body; i = i + 1 } }`
    /// (`<=` for an inclusive range). Any other iterable is an iterator —
    /// see [`Checker::check_for_iterator`].
    ///
    /// The first two lines were stranded on `check_array_literal`, which had been
    /// inserted into the middle of this comment; rejoined here.
    fn check_for(
        &mut self,
        fcx: &mut FnCtx,
        pattern: &Spanned<ast::Pattern>,
        iter: &Spanned<ast::Expr>,
        body: &Spanned<ast::Block>,
        span: Span,
    ) -> hir::Expr {
        let ast::Expr::Range { lo, hi, inclusive } = &iter.value else {
            return self.check_for_iterator(fcx, pattern, iter, body, span);
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
        let (var_name, var_span) = self.for_loop_var(pattern);
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

    /// The name and span a `for` loop binds its element to. Shared by the range
    /// and iterator desugars: they bind at different types but accept exactly
    /// the same pattern forms, so `E0022` — and the choice to fall back to `_`
    /// and keep checking the body rather than bail — is stated once here instead
    /// of in two copies that can drift.
    ///
    /// A `for` loop variable is immutable in both desugars, so assigning it is
    /// `E0060` — measured: `for i in 0..3 { i = i + 1 }` is
    /// `error[E0060]: cannot assign to immutable variable 'i'`.
    ///
    /// The `..` in the `Ident` arm below discards `ast::Pattern::Ident`'s
    /// `is_mut`, and that discard is **currently unreachable from source**: the
    /// parser rejects a `mut` in this position before any pattern exists, so
    /// there is no binding to make immutable and no `E0060` to compare against.
    /// Measured, both spellings: `for mut i in 0..3` and `for mut x in v.iter()`
    /// are each `error[P0001]: expected pattern (in statement), found 'mut'`.
    /// An earlier version of this comment said `for mut i in …` "gets the same
    /// immutable binding (and the same `E0060` on assignment) as `for i in …`",
    /// which describes a program that cannot be written. The discard stays as
    /// the right answer for if the parser ever accepts the form — not as a
    /// statement about today's behaviour.
    fn for_loop_var(&mut self, pattern: &Spanned<ast::Pattern>) -> (String, Span) {
        match &pattern.value {
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
        }
    }

    /// `for x in it { body }` desugars to
    /// `{ let mut __it = it
    ///    while true { match __it.next() { Some(x) => body, None => break } } }`.
    ///
    /// `while true`, not `loop`: Nova has no `loop` keyword — `loop { … }`
    /// parses as an identifier followed by a record literal.
    ///
    /// `__it` is unscoped (so it can neither collide with nor shadow a source
    /// identifier) and `mut` (so `next`'s `mut self` receiver is satisfied
    /// without the user writing `mut`, per ADR 0005 §1). The user's `x` stays
    /// immutable, exactly as in the range form, so assigning it is `E0060`.
    ///
    /// **The iterable is checked here, not at monomorphization.** An earlier
    /// revision of this comment claimed the opposite — "the `Iterator` bound is
    /// discharged at monomorphization (`E0013`) like every other bound, not
    /// here" — and that is measured false: `for x in 5` is
    /// `error[E0900]: 'for' loops over anything but an integer range ('a..b')
    /// or a value implementing 'Iterator' (try '.iter()') are not supported
    /// yet`, reported by the `iterator_next` lookup below, in typeck. There is
    /// no `Iterator` *bound* to discharge anywhere: the desugar needs an impl
    /// in hand to learn the item type at all, so failing to find one is
    /// necessarily an error at this point rather than a constraint deferred to
    /// mono.
    ///
    /// What it therefore keys on is `next`'s *shape*, not std's `Iterator`'s
    /// identity — the same duck-typing [`Checker::try_display`] uses for `fmt`,
    /// and forced by the same thing: the checker has no name-based handle on a
    /// std trait, and a user `trait It { type Item  fn next(…) }` is as good an
    /// iterator as std's. std/core already documents that `next` is *not*
    /// soft-reserved, so a user trait declaring one on a primitive makes
    /// `for x in 3` legal; that is the accepted consequence, not an oversight.
    fn check_for_iterator(
        &mut self,
        fcx: &mut FnCtx,
        pattern: &Spanned<ast::Pattern>,
        iter: &Spanned<ast::Expr>,
        body: &Spanned<ast::Block>,
        span: Span,
    ) -> hir::Expr {
        // One name for the hidden iterator. The local, the path `place_root`
        // resolves, and the scope entry it resolves through must all agree, so
        // they read it from here rather than repeating a literal three times.
        const IT: &str = "__it";

        let iterable = self.check_expr(fcx, iter);
        let iter_ty = fcx.icx.apply(&iterable.ty);
        // The iterable's own mistake has already been reported. `Ty::Error` has
        // no head, so resolution below would fail and add `E0900` on top —
        // two diagnostics for one mistake.
        if matches!(iter_ty, Ty::Error) {
            return error_expr(span);
        }
        let Some(next) = self.iterator_next(fcx, &iter_ty) else {
            self.unsupported(
                iter.span,
                "`for` loops over anything but an integer range (`a..b`) or a value \
                 implementing `Iterator` (try `.iter()`)",
            );
            return error_expr(span);
        };

        // The iterator is live for the whole loop and `next` takes `mut self`,
        // so `__it` is `mut`; it is unscoped so no source identifier can reach
        // it or be shadowed by it.
        let it_local = fcx.new_local_unscoped(IT.to_string(), iter_ty.clone(), true, span);
        let receiver = hir::Expr {
            kind: hir::ExprKind::Local(it_local),
            ty: iter_ty,
            span: iter.span,
        };
        // `emit_trait_call` classifies its receiver with `place_root`, which
        // resolves an *AST* name against the scopes — the checked `hir::Expr`
        // has lost the shape it needs. So the receiver has to be spelled as a
        // path and be findable for the one call emitted below. The scope is
        // pushed and popped around that single call: no source expression is
        // checked inside it, so `__it` stays unreachable from user code, while
        // `place_root` still reads the local's real `is_mut` — dropping the
        // `mut` above is `E0060`, not a silently accepted mutation.
        let it_ast = Spanned::new(
            ast::Expr::Path(ast::Path::single(Spanned::new(IT.to_string(), iter.span))),
            iter.span,
        );
        fcx.scopes.push(FxHashMap::default());
        fcx.scopes
            .last_mut()
            .expect("just pushed")
            .insert(IT.to_string(), it_local);
        let next_call = self.emit_trait_call(
            fcx,
            next.trait_id,
            next.method_idx,
            TraitCallSelf::Receiver(receiver, &it_ast),
            Vec::new(),
            span,
        );
        fcx.scopes.pop();

        // `next`'s declared `Option<Self::Item>` comes back from
        // `emit_trait_call` with `Self` substituted and the projection already
        // normalized against the impl that call selected, so this *result's*
        // argument list is the item type. Reading it off the call is what keeps
        // the desugar on one impl-selection path: resolving `Self::Item` again
        // here would be the second lookup that has twice drifted out of step
        // with `match_args` and shipped as a miscompile.
        let item_ty = match fcx.icx.apply(&next_call.ty) {
            Ty::Sum { def_id, args } if def_id == next.option => self
                .sums
                .iter()
                .find(|s| s.def_id == next.option)
                .and_then(|s| s.variants.get(next.some as usize))
                .and_then(|v| v.fields.first())
                .map(|payload| payload.subst(&args))
                .unwrap_or(Ty::Error),
            // `emit_trait_call` bailed (it reported why). `Ty::Error` unifies
            // with anything, so the body is still checked for its own mistakes
            // without the loop variable manufacturing new ones.
            _ => Ty::Error,
        };

        fcx.scopes.push(FxHashMap::default());
        let (var_name, var_span) = self.for_loop_var(pattern);
        let elem = fcx.new_local(var_name, item_ty, false, var_span);
        fcx.loop_depth += 1;
        let body_hir = self.check_block(fcx, &body.value, body.span);
        fcx.loop_depth -= 1;
        fcx.scopes.pop();

        let unit = |kind| hir::Expr {
            kind,
            ty: Ty::Unit,
            span,
        };
        let variant = |v, binders| hir::Pattern::Variant {
            sum: next.option,
            variant: v,
            binders,
        };
        let arms = vec![
            hir::Arm {
                pattern: variant(next.some, vec![Some(elem)]),
                // Wrapped in a block so the arm is `Unit` whatever the body
                // evaluates to — the range desugar discards the body's value
                // the same way, by making it a statement.
                body: unit(hir::ExprKind::Block {
                    stmts: vec![body_hir],
                    trailing: None,
                }),
                span: body.span,
            },
            hir::Arm {
                pattern: variant(next.none, Vec::new()),
                body: hir::Expr {
                    kind: hir::ExprKind::Break,
                    ty: Ty::Never,
                    span,
                },
                span,
            },
        ];
        let while_expr = unit(hir::ExprKind::While {
            cond: Box::new(hir::Expr {
                kind: hir::ExprKind::BoolLit(true),
                ty: Ty::Bool,
                span,
            }),
            body: Box::new(unit(hir::ExprKind::Match {
                scrutinee: Box::new(next_call),
                arms,
            })),
        });
        let bind_it = unit(hir::ExprKind::Let {
            local: it_local,
            init: Box::new(iterable),
        });
        unit(hir::ExprKind::Block {
            stmts: vec![bind_it, while_expr],
            trailing: None,
        })
    }

    /// Resolve the `next` a `for` loop will drive on a value of type `recv_ty`.
    /// `None` — which is what makes `for` reject the iterable — when the type
    /// has no method of that name, when more than one trait provides one, or
    /// when the one it finds is not shaped like `Iterator::next`.
    ///
    /// Resolution goes through [`Checker::resolve_method_on`], the checker's one
    /// method-lookup routine, which filters candidate impls by
    /// `ImplInfo::match_args` and not by head alone.
    fn iterator_next(&self, fcx: &FnCtx, recv_ty: &Ty) -> Option<IteratorNext> {
        let MethodRes::Trait(trait_id, method_idx) = self.resolve_method_on(recv_ty, fcx, "next")
        else {
            return None;
        };
        let tm = self
            .traits
            .iter()
            .find(|t| t.def_id == trait_id)?
            .methods
            .get(method_idx as usize)?;
        // `next(mut self) -> Option<…>`: no further arguments, and no generics
        // of its own, so the item type is ground the moment `Self` is known
        // rather than an inference variable the loop body has to pin down.
        if !tm.params.is_empty() || tm.generics != 0 {
            return None;
        }
        // An `Option`-shaped return: exactly the two variants `Some(x)` and
        // `None`. Read off the *declaration*, before any call is emitted, so a
        // wrongly shaped `next` reports only `E0900` and not also whatever
        // `emit_trait_call` would have said about its receiver or arity.
        let Ty::Sum { def_id, .. } = tm.ret else {
            return None;
        };
        let sum = self.sums.iter().find(|s| s.def_id == def_id)?;
        if sum.variants.len() != 2 {
            return None;
        }
        Some(IteratorNext {
            trait_id,
            method_idx,
            option: def_id,
            some: sum
                .variants
                .iter()
                .position(|v| v.name == "Some" && v.fields.len() == 1)? as u32,
            none: sum
                .variants
                .iter()
                .position(|v| v.name == "None" && v.fields.is_empty())? as u32,
        })
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
                self.convert_ty(&p.ty, &generics_scope, &fcx.param_bounds)
            };
            let local = fcx.new_local(p.name.value.clone(), ty.clone(), p.is_mut, p.name.span);
            param_locals.push(local);
            param_types.push(ty);
        }
        let ret_ty = match ret {
            Some(rt) => self.convert_ty(rt, &generics_scope, &fcx.param_bounds),
            None => fcx.icx.fresh(),
        };
        // A `break`/`continue` in the closure body cannot target a loop in
        // the enclosing function.
        let saved_loop_depth = fcx.loop_depth;
        fcx.loop_depth = 0;
        // Likewise `.await`: Nova has no `async` closure syntax, so a closure
        // is always its own non-async function, even one written inside an
        // `async fn`'s body.
        let saved_in_async = fcx.in_async;
        fcx.in_async = false;
        let body_hir = self.check_expr(fcx, body);
        fcx.loop_depth = saved_loop_depth;
        fcx.in_async = saved_in_async;
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
            // Nova has no `async` closure syntax (see the reset in
            // `check_closure`), so a lifted closure body is never async.
            is_async: false,
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
            // The wrapper's body is a synthetic forwarding `Call`, never a
            // `.await` — irrespective of whether `target` itself is async
            // (then `ret` is already its `Future<_>`, forwarded unchanged).
            is_async: false,
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
            // Normalization seam: a record's field may declare a projection on
            // the record's own bounded type parameter (`f: fn(I::Item) -> U`),
            // and `type_args` carries this literal's fresh per-field inference
            // vars — so once an earlier field (`it: I`) has pinned one of them
            // to a concrete type via `unify` below, a later field's `subst`
            // can leave a projection rooted in that now-concrete type. Plain
            // `subst` cannot see that; `instantiate` applies the current
            // bindings and, only if a projection remains, resolves it through
            // the impl table — the same seam `check_direct_call` and
            // `emit_trait_call` use for a generic function or trait method's
            // parameter and return types.
            //
            // Same admitted hole as `instantiate`'s own doc comment describes
            // for a call (above, `fn instantiate`): the pinning variable has to
            // already be solved. `for init in fields` walks the *literal's*
            // written order, not declaration order, so this only resolves
            // `I::Item` when the field that pins `I` (`it: I`) is written
            // before the field naming the projection (`f: fn(I::Item) -> U`)
            // in the literal. `M { f: |x| x + 1, it: Counter { n: 0 } }` — the
            // same fields, reversed — still fails with the pre-fix symptom
            // (an unresolved `Assoc` reaching `unify`), because `type_args`'s
            // slot for `I` is still a free variable when `f`'s turn comes.
            // Wider than the call case: a function's parameter order is fixed
            // by its signature, but a record literal's field order is free
            // syntax with no natural declaration-order requirement, so a
            // caller has no signature to read this constraint off of.
            // `a_record_field_initializer_is_order_sensitive_when_it_names_a_
            // projection` (this file's test module) pins the current
            // behavior so a future fix to field-order independence flips it
            // deliberately rather than silently.
            let expected = self.instantiate(&field.ty, &type_args, &fcx.icx, init.name.span);
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
            // Same normalization seam as above, but **defence in depth, not a
            // load-bearing seam — and no test pins it.** Deliberate, and
            // recorded as such rather than left to look like a mutation that
            // got away (the convention `instantiate`'s own doc comment sets for
            // its `has_assoc` guard).
            //
            // Measured: replacing this line with the pre-increment
            // `field.ty.subst(&type_args)` leaves the entire suite green — zero
            // failures, every gate configuration included — and the two probes
            // that exercise the shape are byte-identical either way
            // (`MapIter { it: 5, f: |x| x }` is one `E0079` from mono;
            // `f: |x: Int| x` is one `E0010` plus four `E0011`).
            //
            // The reason is structural, so it is not a coverage gap that a new
            // test would close. These types do flow into `MakeRecord`'s
            // `hir::Expr`s, but `Specializer::expr` (`crates/nova-mir/src/mono.rs`)
            // rewrites `ty: self.ty(&expr.ty)` on **every** expression node it
            // clones, and `MakeRecord`'s field exprs go through `self.exprs`
            // like any other operand. `Specializer::ty` substitutes *and*
            // normalizes, recording `E0079` when a projection cannot resolve.
            // So a raw `subst` here cannot reach `mir_ty`: normalization seam 3
            // re-normalizes it first, and the worst case is a diagnostic, never
            // the `MirTy::Unit` an earlier version of this comment claimed
            // ("would not be a diagnostic gap but a miscompile" — measured
            // false, by the mutation above).
            //
            // Contrast the sibling seam at `let expected = …` above, which is
            // the real one: there the type is unified against a field
            // initializer *inside typeck*, so failing to normalize is a
            // spurious `E0010` that mono never gets to see — a genuine
            // diagnostic gap, and pinned by
            // `a_record_field_initializer_normalizes_a_projection_once_its_
            // parameter_is_concrete`. Keep both routed through `instantiate`
            // anyway: one seam per read of `field.ty` is the invariant that
            // makes this function easy to reason about, and relying on a
            // downstream crate to clean up after this one is a coupling worth
            // paying a redundant call to avoid.
            let field_ty = self.instantiate(&field.ty, &type_args, &fcx.icx, span);
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
        if let Some((index, field_ty)) =
            self.record_field_index_and_ty(fcx, &recv_ty, &field.value, span)
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

    /// Resolve a field name on a record type to its `(index, instantiated type)`.
    /// Shared by the field read and field write paths so they cannot disagree
    /// about layout or about how a field's declared type is instantiated.
    ///
    /// "Instantiated", not merely "substituted": the type goes through
    /// `instantiate`, which substitutes the record's type arguments, applies the
    /// current inference bindings, and normalizes a projection that survives
    /// both. That matters because a field type may name a projection on one of
    /// the record's own bounded parameters (`f: fn(I::Item) -> U`), and `unify`
    /// never normalizes — see `instantiate`'s own doc comment.
    ///
    /// Emits no diagnostics: a `None` means "no such field on this type", and
    /// each caller phrases that in its own terms.
    fn record_field_index_and_ty(
        &mut self,
        fcx: &mut FnCtx,
        recv_ty: &Ty,
        field: &str,
        span: Span,
    ) -> Option<(u32, Ty)> {
        let Ty::Record { def_id, args } = fcx.icx.apply(recv_ty) else {
            return None;
        };
        let record = self.records.iter().find(|r| r.def_id == def_id)?;
        let index = record.fields.iter().position(|f| f.name == field)?;
        // The field's declared type is written in terms of the record's own
        // type parameters, so it must be substituted with this instantiation's
        // arguments before it means anything to the caller. Cloned out of
        // `record` (rather than substituted in place) so the borrow of
        // `self.records` ends here, before the `&mut self` call below.
        let declared_ty = record.fields[index].ty.clone();
        // Normalization seam, the read-side mirror of the one
        // `check_record_literal` needed for construction: by the time a field
        // is *read* (`m.f`, `c.hit`), `recv_ty`'s arguments may already be
        // concrete (`m`/`c` were already built), so a field type naming a
        // projection on the record's own bounded parameter (`f: fn(I::Item)
        // -> U`, `hit: I::Item`) can substitute to an unnormalized `Assoc`
        // just as a field *initializer*'s expected type could. Plain `subst`
        // cannot see that; `instantiate` applies the current bindings and, if
        // a projection remains, resolves it through the impl table.
        let field_ty = self.instantiate(&declared_ty, &args, &fcx.icx, span);
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
        // Every name matched here must also be in
        // `nova_resolver::RESERVED_TYPE_NAMES`, the same requirement
        // `convert_ty`'s own nullary table is already held to: a new
        // nullary built-in added to this match without joining that list
        // would leave a user type under its name declarable, which is
        // exactly the defect the list exists to close.
        match name {
            "Int" => return Some(Ty::Int),
            "Float" => return Some(Ty::Float),
            "Bool" => return Some(Ty::Bool),
            "Char" => return Some(Ty::Char),
            "String" => return Some(Ty::String),
            "Bytes" => return Some(Ty::Bytes),
            // `Future` is compiler-constructed and carries no associated
            // functions; `Future::f()` is not a qualifier. Returning None here
            // (rather than falling through to resolve_type) keeps the
            // diagnostic about the qualifier instead of about a missing type.
            "Future" => return None,
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
    /// mutable-receiver rule classifies it with `place_root`.
    ///
    /// **The rule covers both dispatch paths.** An inherent callee is checked
    /// here, keyed on the impl method's `DefId` in `Checker::mut_self`; a trait
    /// callee is checked inside [`Checker::emit_trait_call`], keyed on
    /// [`hir::TraitMethod::mut_self`], because that is the one point *every*
    /// receiver route converges on and this arm is not (see the note there).
    /// Either way the receiver AST is what gets classified, which is why it is
    /// threaded through [`TraitCallSelf::Receiver`] rather than left behind.
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
                TraitCallSelf::Receiver(receiver, receiver_ast),
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
        dispatch: TraitCallSelf<'_>,
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
            TraitCallSelf::Receiver(..) if !tm.has_self => {
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
            TraitCallSelf::Receiver(recv, recv_ast) => {
                // The mutable-receiver rule (ADR 0005 §1), on the trait path.
                // Keyed on the *trait method's* flag, not on any impl's: for a
                // generic receiver there is no single impl, and the trait's
                // declaration is what the call site programs against anyway —
                // `check_impl_method_signatures` is what keeps the impl in step.
                //
                // Here rather than in `check_method_call`'s `MethodRes::Trait`
                // arm, which is where ADR 0005's migration path put it, because
                // that arm is not the only receiver route: `try_display` reaches
                // a `fmt(mut self) -> String` straight from string interpolation
                // without passing through it at all. This is the one point every
                // receiver route converges on, so the rule cannot be dodged by
                // finding another way in. Runs *before* the arity and argument
                // checks below and does not return early, so a missing `let mut`
                // stays one diagnostic rather than cascading — the same order
                // and the same reasoning as `MethodRes::Inherent`.
                if tm.mut_self {
                    // The callee is named by the trait's declared method name,
                    // unqualified: `Self` may still be a generic parameter here,
                    // so there is no impl to qualify with. That is the same
                    // choice every other diagnostic at this dispatch site makes,
                    // and `arity_errors_name_the_callee_uniformly` pins it.
                    self.require_mutable_place(
                        fcx,
                        recv_ast,
                        span,
                        MutTarget::Receiver(tm.name.clone()),
                    );
                }
                (fcx.icx.apply(&recv.ty), Some(Box::new(recv)))
            }
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
            // Same seam as the return type below: a trait method may declare a
            // *parameter* as `Self::Item` (`fn put(mut self, x: Self::Item)`) and
            // the argument the caller passes is a concrete type.
            let expected = self.instantiate(param, &subst, &fcx.icx, arg.span);
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
        // Normalization seam: the method-call return type. A trait method
        // declaring `-> Self::Item` gives `Assoc { on: Param(0) }`, and `subst`
        // has just replaced `Param(0)` with the receiver's *concrete* type — so
        // this is the first point at which the impl table can answer it, and the
        // last point before the type escapes into the caller's unification.
        //
        // `self_ty` came from `fcx.icx.apply(&recv.ty)` above, so the `Self` half
        // of `subst` is already resolved; `instantiate` covers the rest, for a
        // generic trait method projecting onto one of its own parameters.
        let ret = self.instantiate(&tm.ret, &subst, &fcx.icx, span);
        hir::Expr {
            kind: hir::ExprKind::TraitCall {
                trait_id,
                method: method_idx,
                self_ty,
                type_args,
                receiver,
                args,
            },
            ty: ret,
            span,
        }
    }

    /// If `recv_ty` has a `Display`-style `fmt(self) -> String` method in
    /// scope, build the call that produces its string. Used to interpolate
    /// user types. Returns `None` (leaving `value` consumed) if no such
    /// method resolves.
    ///
    /// `value_ast` is `value` before checking, threaded through purely so
    /// [`Checker::emit_trait_call`] can apply the mutable-receiver rule: nothing
    /// stops a user trait from declaring `fn fmt(mut self) -> String`, and this
    /// is a receiver route that never passes through
    /// [`Checker::check_method_call`]. `std/core`'s own `Display` declares a
    /// plain `self`, so the flag is false for every interpolation in std and the
    /// check is a no-op there.
    fn try_display(
        &mut self,
        fcx: &mut FnCtx,
        value: hir::Expr,
        value_ast: &Spanned<ast::Expr>,
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
            TraitCallSelf::Receiver(value, value_ast),
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
        let Some((index, field_ty)) =
            self.record_field_index_and_ty(fcx, &recv_ty, &field.value, span)
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

/// Collect, without duplicates, the associated types that `ty` projects onto
/// `self_ty` — every `Assoc { on, assoc }` inside `ty` whose `on` is exactly
/// `self_ty`, which inside an impl block is what `Self::Name` converts to (see
/// `convert_ty`'s two-segment path case). The edge set for
/// `check_assoc_binding_cycles`.
///
/// **Recurses into compound types.** `type Item = [Self::Item]` names an
/// infinitely large type just as `type Item = Self::Item` does, and a walk that
/// only inspected the top level would accept it and leave `normalize_ty` growing
/// one `[…]` layer per step.
///
/// A projection onto anything *other* than the impl's own self type is not an
/// edge: `type Item = T::Item` for `impl<T: It> It for W<T>` names the
/// argument's associated type, and every normalization step strips a `W<…>`
/// layer, so it bottoms out. Flagging it would reject legitimate code.
fn collect_self_projections(ty: &Ty, self_ty: &Ty, out: &mut Vec<DefId>) {
    match ty {
        Ty::Assoc { on, assoc } => {
            if **on == *self_ty && !out.contains(assoc) {
                out.push(*assoc);
            }
            // A projection *on* a projection (`Self::A::B` is unwritable today,
            // but `subst` can build one) still has to be searched.
            collect_self_projections(on, self_ty, out);
        }
        Ty::Fn { params, ret } => {
            for p in params {
                collect_self_projections(p, self_ty, out);
            }
            collect_self_projections(ret, self_ty, out);
        }
        Ty::Sum { args, .. } | Ty::Record { args, .. } => {
            for a in args {
                collect_self_projections(a, self_ty, out);
            }
        }
        Ty::Array(elem) => collect_self_projections(elem, self_ty, out),
        Ty::Future(fut_out) => collect_self_projections(fut_out, self_ty, out),
        Ty::Int
        | Ty::Float
        | Ty::Bool
        | Ty::Char
        | Ty::String
        | Ty::Bytes
        | Ty::Unit
        | Ty::Param(_)
        | Ty::Var(_)
        | Ty::Never
        | Ty::Error => {}
    }
}

/// Whether `start` can reach itself through the dependency edges (i.e.
/// participates in a cycle). Shared by the two definition-cycle checks —
/// `check_const_cycles` (a constant defined in terms of its own value) and
/// `check_assoc_binding_cycles` (an associated type bound in terms of itself);
/// the graph is `DefId -> DefId` in both and the walk is identical, so keeping
/// one copy is what stops the two from drifting on cycle detection itself.
///
/// `seen` is what makes this terminate on the cyclic input it exists to find:
/// a node is expanded at most once.
fn reaches_self(start: DefId, edges: &FxHashMap<DefId, Vec<DefId>>) -> bool {
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
        K::Unary { expr: e, .. } | K::ToStr(e) | K::Await(e) => out.push(e),
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
        K::Unary { expr: e, .. } | K::ToStr(e) | K::Await(e) => out.push(e),
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
/// are *not callable from a user program*: `Builtin::STD_ONLY` members are
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
///
/// **`Ty::Param(0)` here means "the caller's first type parameter", not "any
/// type".** `unify` relates two `Ty::Param`s only when their indices are equal
/// (`infer.rs`), so a signature mentioning `Param(0)` type-checks against a
/// call site inside a generic function whose *first* type parameter is the one
/// being passed, and against nothing else. That is exactly the shape
/// `std/task`'s wrappers have (`fn spawn<T>(fut: Future<T>)` and friends), and
/// it is what lets a builtin be generic at all without a signature
/// instantiation step: monomorphization substitutes the call expression's own
/// type along with every other, so `nova-mir` sees the concrete one. A
/// `std/task` function that declared a second type parameter *before* `T`
/// would fail here with `E0010` rather than miscompiling.
fn builtin_signature(builtin: Builtin) -> (Vec<Ty>, Ty) {
    // `Future<T>` where `T` is the caller's first type parameter — see above.
    let future_of_param0 = || Ty::Future(Box::new(Ty::Param(0)));
    match builtin {
        Builtin::Println | Builtin::Print | Builtin::EPrint | Builtin::EPrintln => {
            (vec![Ty::String], Ty::Unit)
        }
        Builtin::Panic => (vec![Ty::String], Ty::Never),
        Builtin::StrCmp => (vec![Ty::String, Ty::String], Ty::Int),
        Builtin::StrHash => (vec![Ty::String], Ty::Int),
        Builtin::CharToInt => (vec![Ty::Char], Ty::Int),
        Builtin::StrLenChars => (vec![Ty::String], Ty::Int),
        Builtin::StrChars => (vec![Ty::String], Ty::Array(Box::new(Ty::Char))),
        Builtin::StrFromChars => (vec![Ty::Array(Box::new(Ty::Char))], Ty::String),
        Builtin::StrToUpper | Builtin::StrToLower => (vec![Ty::String], Ty::String),
        Builtin::TestSelector => (vec![], Ty::Int),
        Builtin::TaskSpawn => (vec![future_of_param0()], Ty::Int),
        Builtin::TaskIsDone => (vec![future_of_param0()], Ty::Bool),
        Builtin::TaskRelease => (vec![future_of_param0()], Ty::Unit),
        Builtin::TaskDrive => (vec![future_of_param0()], Ty::Unit),
        Builtin::TaskOutput => (vec![future_of_param0()], Ty::Param(0)),
        Builtin::TaskYieldFuture => (vec![], Ty::Future(Box::new(Ty::Unit))),
        Builtin::TaskSleepFuture => (vec![Ty::Int], Ty::Future(Box::new(Ty::Unit))),
        Builtin::TaskJoinFuture => (vec![future_of_param0()], Ty::Future(Box::new(Ty::Unit))),
        Builtin::FsReadToString => (vec![Ty::String], Ty::Int),
        Builtin::FsWriteString => (vec![Ty::String, Ty::String], Ty::Int),
        Builtin::FsTakeString => (vec![], Ty::String),
        Builtin::FsLastErrorMessage => (vec![], Ty::String),
        Builtin::FsTempDir => (vec![], Ty::String),
        Builtin::FsExists => (vec![Ty::String], Ty::Bool),
        Builtin::FsCreateDir => (vec![Ty::String], Ty::Int),
        Builtin::FsCreateDirAll => (vec![Ty::String], Ty::Int),
        Builtin::FsRemoveFile => (vec![Ty::String], Ty::Int),
        Builtin::FsRemoveDirAll => (vec![Ty::String], Ty::Int),
        Builtin::FsReadDir => (vec![Ty::String], Ty::Int),
        Builtin::FsTakeStringArray => (vec![], Ty::Array(Box::new(Ty::String))),
        Builtin::FsKind => (vec![Ty::String], Ty::Int),
        Builtin::FsRead => (vec![Ty::String], Ty::Int),
        Builtin::FsTakeBytes => (vec![], Ty::Bytes),
        Builtin::FsWrite => (vec![Ty::String, Ty::Bytes], Ty::Int),
        Builtin::BytesLen => (vec![Ty::Bytes], Ty::Int),
        Builtin::BytesFromString => (vec![Ty::String], Ty::Bytes),
        Builtin::BytesIsUtf8 => (vec![Ty::Bytes], Ty::Bool),
        Builtin::BytesToStringUnchecked => (vec![Ty::Bytes], Ty::String),
        Builtin::BytesAt => (vec![Ty::Bytes, Ty::Int], Ty::Int),
        Builtin::BytesSlice => (vec![Ty::Bytes, Ty::Int, Ty::Int], Ty::Bytes),
        Builtin::BytesConcat => (vec![Ty::Bytes, Ty::Bytes], Ty::Bytes),
        Builtin::BytesToInts => (vec![Ty::Bytes], Ty::Array(Box::new(Ty::Int))),
        Builtin::BytesFromInts => (vec![Ty::Array(Box::new(Ty::Int))], Ty::Bytes),
        Builtin::BytesEq => (vec![Ty::Bytes, Ty::Bytes], Ty::Bool),
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
        | hir::ExprKind::Assign { value: inner, .. }
        | hir::ExprKind::Await(inner) => {
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
    use nova_resolver::{resolve, TestFn};

    fn check_src(src: &str) -> CheckResult {
        let file_id = FileId::DUMMY;
        let (tokens, lex_errors) = lex(src, file_id);
        assert!(lex_errors.is_empty(), "lex errors: {lex_errors:?}");
        let (ast, parse_errors) = parse(&tokens, file_id);
        assert!(parse_errors.is_empty(), "parse errors: {parse_errors:?}");
        let ast = ast.expect("no AST");
        let resolved = resolve(&ast);
        // Type-check against the merged file (input + the implicit prelude,
        // which is `std/core` — see ADR 0004), whose `item_index`es the
        // definitions refer to. Resolver diagnostics (Task 2's `@test`
        // attribute validation among them: E0082-E0085) used to be asserted
        // empty here and dropped; they are prepended into the returned
        // `CheckResult` instead, in the same order the driver renders them
        // (resolve, then check), since callers now legitimately exercise
        // resolver-level errors through this helper. This is a no-op for
        // every caller that never triggered a resolver diagnostic in the
        // first place — which, before this change, was every caller, since
        // the old assert would otherwise have failed their test.
        let mut checked = check(&resolved.file, &resolved.definitions);
        let mut diagnostics = resolved.diagnostics;
        diagnostics.append(&mut checked.diagnostics);
        checked.diagnostics = diagnostics;
        checked
    }

    /// Resolve `src` and return its collected `@test` functions, in the order
    /// `nova_resolver::resolve`'s item walk visited them — source order. Kept
    /// separate from `check_src`/`CheckResult` (which has no field for this):
    /// `TestFn` lives on the resolver's output, and this test needs nothing
    /// from the checker.
    fn collect_tests_of(src: &str) -> Vec<TestFn> {
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
        resolved.tests
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
        // `async` on a trait-method *declaration* (no body) must still be
        // rejected, same as a default (bodied) trait method and an extern fn —
        // Phase 2.3a Task 2 lifts this same rejection for free functions and
        // impl/inherent methods (see `async_inherent_method_is_accepted`), but
        // trait methods are the half that stays rejected (see
        // `async_trait_method_still_reports_e0900`).
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
    fn async_fn_returns_a_future_of_its_declared_type() {
        // The declared return type is the future's OUTPUT, per the spec's own
        // signatures (`pub async fn join(self) -> T`). So passing the CALL of an
        // async fn where the output type is expected must be a mismatch.
        let r = check_src(
            "async fn f() -> Int { 1 }\n\
             fn g() -> Int { f() }\n\
             fn main() {}",
        );
        let d = r
            .diagnostics
            .iter()
            .find(|d| d.code == "E0010")
            .unwrap_or_else(|| {
                panic!(
                    "calling an async fn yields Future<Int>, not Int; got {:?}",
                    r.diagnostics.iter().map(|d| &d.code).collect::<Vec<_>>()
                )
            });
        // A code-only assertion here would survive a reversal (declared and
        // actual body types swapped in the message) or a wrong-layer bug
        // (e.g. the body type shown as bare `Int` instead of `Future<Int>`,
        // which would mean the E0010 fired for some other reason than the
        // one this test exists to pin). Tying each type name to its role via
        // the surrounding words, not just checking both names appear
        // somewhere, is what catches a reversal: after swapping, the message
        // would contain "return `Future<Int>`" and "type `Int`" instead.
        // Measured, not assumed: `` `g` should return `Int` but its body has
        // type `Future<Int>` ``.
        assert!(
            d.message.contains("return `Int`") && d.message.contains("has type `Future<Int>`"),
            "E0010 must name the declared return type (`Int`) and the actual \
             body type (`Future<Int>`) in their correct roles; got {:?}",
            d.message
        );
    }

    #[test]
    fn awaiting_a_future_yields_its_output_type() {
        // The positive case: inside an async fn, `.await` unwraps to the output
        // and type-checks against it. Assert CLEAN, and assert on the messages so
        // a spurious diagnostic is visible rather than counted.
        let r = check_src(
            "async fn f() -> Int { 1 }\n\
             async fn g() -> Int { f().await }\n\
             fn main() {}",
        );
        assert!(
            r.diagnostics.is_empty(),
            "expected no diagnostics, got {:?}",
            r.diagnostics
                .iter()
                .map(|d| d.message.clone())
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn awaiting_yields_the_output_not_the_future() {
        // Discriminates "await returns T" from "await returns Future<T>" by
        // giving `g` a DIFFERENT Output type (Bool) than `f` (Int): a wrong
        // `.await` result is named directly in an Int-vs-Bool mismatch here.
        // (Measured: in the current implementation a total forgot-to-unwrap
        // bug also happens to fail the clean test above, via its own
        // body-vs-declared-return unify — but this test does not depend on
        // that coincidence, and would still catch a subtler bug, e.g. one
        // that always produces `Int`, which would pass the clean test by luck.)
        let r = check_src(
            "async fn f() -> Int { 1 }\n\
             async fn g() -> Bool { f().await }\n\
             fn main() {}",
        );
        let msgs: Vec<String> = r.diagnostics.iter().map(|d| d.message.clone()).collect();
        assert!(
            msgs.iter().any(|m| m.contains("Int") && m.contains("Bool")),
            "expected an Int-vs-Bool mismatch naming both types, got {msgs:?}"
        );
    }

    #[test]
    fn await_outside_an_async_fn_reports_e0086() {
        let r = check_src(
            "async fn f() -> Int { 1 }\n\
             fn g() -> Int { f().await }\n\
             fn main() {}",
        );
        let d = r
            .diagnostics
            .iter()
            .find(|d| d.code == "E0086")
            .unwrap_or_else(|| {
                panic!(
                    "expected E0086, got {:?}",
                    r.diagnostics.iter().map(|d| &d.code).collect::<Vec<_>>()
                )
            });
        // Assert the message names the actual problem. A code-only assertion here
        // would survive swapping E0086's and E0087's message text.
        assert!(
            d.message.contains("async"),
            "E0086 must explain that await requires an async fn; got {:?}",
            d.message
        );
    }

    #[test]
    fn await_on_a_non_future_reports_e0087() {
        let r = check_src(
            "async fn g() -> Int { let x = 1\n x.await }\n\
             fn main() {}",
        );
        let d = r
            .diagnostics
            .iter()
            .find(|d| d.code == "E0087")
            .unwrap_or_else(|| {
                panic!(
                    "expected E0087, got {:?}",
                    r.diagnostics.iter().map(|d| &d.code).collect::<Vec<_>>()
                )
            });
        assert!(
            d.message.contains("Int"),
            "E0087 must name the type that is not a future; got {:?}",
            d.message
        );
    }

    /// `std/task`'s spawn-and-join surface, at `Float`.
    ///
    /// `Float` rather than `Int` deliberately: `mir_ty` maps `Int`, `Char`,
    /// `String`, `Fn`, `Sum`, `Record` and `Array` all onto the same 64-bit
    /// class, so an `Int` fixture proves nothing about a value that has to
    /// travel through the executor's `i64` output slot. `Float` is the one class
    /// that does not, and a `spawn`/`join` pair that lost the output's type
    /// would show up here as a `Float`-vs-something mismatch rather than as a
    /// clean compile with wrong bits at run time.
    #[test]
    fn std_task_spawn_and_join_typecheck() {
        let r = check_src(
            "async fn f() -> Float { 1.5 }\n\
             async fn g() -> Float { let h = spawn(f())\n h.join().await }\n\
             fn main() { }",
        );
        assert!(
            r.diagnostics.is_empty(),
            "expected no diagnostics, got {:?}",
            r.diagnostics
                .iter()
                .map(|d| d.message.clone())
                .collect::<Vec<_>>()
        );
        // Not vacuous on an empty diagnostic list alone: `Ty::Error` unifies
        // with anything, so a `spawn` that failed to resolve would also produce
        // no *further* diagnostics. Pin that `g`'s body really is typed `Float`.
        let awaited = exprs_in(&r.module, "g")
            .into_iter()
            .find(|e| matches!(e.kind, hir::ExprKind::Await(_)))
            .expect("`h.join().await` is an await");
        assert_eq!(
            awaited.ty,
            Ty::Float,
            "`join().await` on a `JoinHandle<Float>` must be typed `Float`"
        );
    }

    /// `block_on` is how async is entered *from* sync code, so it must not be
    /// mistaken for an await: `E0086` is the diagnostic for awaiting outside an
    /// `async fn`, and a `block_on` that tripped it would leave no way to run
    /// async code from `main` or from a `@test` function at all.
    #[test]
    fn block_on_outside_async_is_allowed() {
        let r = check_src("async fn f() -> Int { 1 }\nfn main() { let x = block_on(f()) }");
        assert!(
            r.diagnostics.is_empty(),
            "expected no diagnostics, got {:?}",
            r.diagnostics
                .iter()
                .map(|d| d.message.clone())
                .collect::<Vec<_>>()
        );
    }

    /// `yield_now` is an `async fn` returning unit, so it is only usable with
    /// `.await` and only inside another `async fn` — the shape the gate fixture
    /// depends on.
    #[test]
    fn yield_now_is_awaitable_inside_an_async_fn() {
        let r = check_src(
            "async fn f() -> Int { yield_now().await\n 1 }\n\
             fn main() { let x = block_on(f()) }",
        );
        assert!(
            r.diagnostics.is_empty(),
            "expected no diagnostics, got {:?}",
            r.diagnostics
                .iter()
                .map(|d| d.message.clone())
                .collect::<Vec<_>>()
        );
    }

    /// No std-only builtin is *callable* from a user module, which for the
    /// `Future`-taking ones is a type-safety property and not only a
    /// namespacing one.
    ///
    /// Their signatures mention `Ty::Param(0)`, meaning "the caller's first type
    /// parameter" — which only exists inside a generic function. Called from an
    /// ordinary `fn main`, such a builtin's result expression would be typed
    /// `Param(0)` with nothing to substitute it, and `mir_ty` maps a surviving
    /// `Param` to `MirTy::Unit`: a value silently dropped rather than a
    /// diagnostic. So the fact that these names do not resolve in user code is
    /// what keeps that unreachable, and it must not be quietly relaxed by moving
    /// one to `Builtin::GLOBAL`.
    ///
    /// Driven off `Builtin::STD_ONLY` and `b.name()` rather than a list of
    /// spellings, so a builtin added to that constant is covered the moment it
    /// joins — and so renaming one cannot leave this passing against a name
    /// nothing declares. `nova-resolver`'s `no_std_only_builtin_is_a_reserved_word`
    /// is the other direction: a user *definition* of one of these names is
    /// allowed.
    #[test]
    fn no_std_only_builtin_is_callable_from_user_code() {
        for b in Builtin::STD_ONLY {
            let name = b.name();
            let r = check_src(&format!("fn main() {{ {name}() }}"));
            assert!(
                error_codes(&r).contains(&"E0001"),
                "`{name}` must not resolve in a user module, got {:?}",
                r.diagnostics
                    .iter()
                    .map(|d| (d.code.clone(), d.message.clone()))
                    .collect::<Vec<_>>()
            );
        }
    }

    #[test]
    fn async_inherent_method_is_accepted() {
        // NOT optional: the spec's JoinHandle::join is `pub async fn join(self) -> T`,
        // an inherent async method. Task 7 cannot write std/task without this.
        let r = check_src(
            "record W { v: Int }\n\
             impl W { async fn get(self) -> Int { self.v } }\n\
             fn main() {}",
        );
        assert!(
            r.diagnostics.is_empty(),
            "expected no diagnostics, got {:?}",
            r.diagnostics
                .iter()
                .map(|d| d.message.clone())
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn async_trait_method_still_reports_e0900() {
        // Trait async needs associated-type futures; out of scope for 2.3a.
        // This pins the HALF-lift: :852 becomes conditional, it does not vanish.
        let r = check_src("trait T { async fn m(self) -> Int }\nfn main() {}");
        let d = r
            .diagnostics
            .iter()
            .find(|d| d.code == "E0900")
            .unwrap_or_else(|| panic!("expected E0900, got {:?}", r.diagnostics));
        // A code-only assertion here would survive the message changing to
        // name a different unsupported construct entirely (e.g. if this
        // E0900 came from the unrelated "where clauses on trait methods"
        // check instead, which fires on the same kind of source). Measured,
        // not assumed: "async methods are not supported yet".
        assert!(
            d.message.contains("async methods"),
            "E0900 must identify async methods as the unsupported construct; got {:?}",
            d.message
        );
    }

    /// An `async` method in a trait *impl* is refused as a return-type
    /// mismatch, not as `E0900`.
    ///
    /// The `E0900` check was removed from `collect_impls`, so what catches this
    /// is trait-conformance: an `async fn` returns `Future<T>`, the trait
    /// declared `T`, and those are different types. The refusal is what matters
    /// and it is legible on its own, but the *code* is `E0072` — pinned here
    /// because the sibling above covers only the declaration, which is why the
    /// claim that `E0900` applies "in a trait declaration, a default body, and
    /// an impl alike" was able to stand in two documents while being false for
    /// the third position. The trait's own `m` is deliberately **not** `async`:
    /// were it async, `E0900` would fire on the declaration and mask which
    /// check refused the impl.
    #[test]
    fn async_trait_impl_method_is_refused_as_a_return_type_mismatch() {
        let r = check_src(
            "trait T { fn m(self) -> Int }\n\
             record C { v: Int }\n\
             impl T for C { async fn m(self) -> Int { self.v } }\n\
             fn main() {}",
        );
        let d = r
            .diagnostics
            .iter()
            .find(|d| d.code == "E0072")
            .unwrap_or_else(|| panic!("expected E0072, got {:?}", r.diagnostics));
        // Naming both types, not just the code: the message is the whole reason
        // this refusal is acceptable without a dedicated diagnostic, so a
        // version of it that said only "signature mismatch" would not be.
        assert!(
            d.message.contains("Future<Int>") && d.message.contains("declares `Int`"),
            "the mismatch must name the future and what the trait declared; got {:?}",
            d.message
        );
        // And it must NOT be reported as the unsupported-construct code, which
        // is what the corrected documentation now says.
        assert!(
            !error_codes(&r).contains(&"E0900"),
            "an impl-side async method is not `E0900`; got {:?}",
            r.diagnostics
        );
    }

    #[test]
    fn async_extern_fn_still_reports_e0900() {
        let r = check_src("extern \"C\" { async fn c_thing() -> Int }\nfn main() {}");
        let d = r
            .diagnostics
            .iter()
            .find(|d| d.code == "E0900")
            .unwrap_or_else(|| panic!("expected E0900, got {:?}", r.diagnostics));
        // Measured, not assumed: "async extern functions are not supported
        // yet". Distinguishes this from the trait-method message above (and
        // from an extern block's OTHER E0900 sites, e.g. an unsupported ABI
        // or generic extern fn) rather than accepting any E0900 on the input.
        assert!(
            d.message.contains("async extern functions"),
            "E0900 must identify async extern functions as the unsupported construct; got {:?}",
            d.message
        );
    }

    #[test]
    fn generic_async_fn_instantiates_at_float() {
        // Float, not Int/Bool: mir_ty collapses Int/Char to I64 and five variants
        // to Ptr (= i64 on x86-64), so an Int-vs-String pair tests nothing at any
        // seam. Float is F64 and crosses register banks.
        let r = check_src(
            "async fn id<T>(x: T) -> T { x }\n\
             async fn g() -> Float { id(1.5).await }\n\
             fn main() {}",
        );
        assert!(
            r.diagnostics.is_empty(),
            "expected no diagnostics, got {:?}",
            r.diagnostics
                .iter()
                .map(|d| d.message.clone())
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn closure_inside_async_fn_resets_in_async_so_await_is_e0086() {
        // Nova has no `async` closure syntax, so a closure is its own non-async
        // function even when written inside an `async fn`'s body — the same
        // discipline `loop_depth` uses for `break`/`continue` (`check_closure`
        // saves/resets/restores `fcx.in_async` around the closure body). Without
        // that reset this `.await` would wrongly inherit `g`'s `in_async = true`
        // and typecheck clean instead of reporting E0086.
        // Written `|n: Int|`, not `||`: the lexer greedily tokenizes `||` as
        // one `PipePipe` (logical-or) token rather than two `Pipe`s, so a
        // truly zero-parameter closure is not spellable — an unrelated,
        // pre-existing lexer fact (measured: `|| { .. }` fails to parse with
        // "expected expression ... found `||`"), not anything this task
        // changes. One unused parameter sidesteps it without adding anything
        // relevant to what this test checks.
        let r = check_src(
            "async fn f() -> Int { 1 }\n\
             async fn g() -> Int {\n\
                 let h = |n: Int| { f().await }\n\
                 0\n\
             }\n\
             fn main() {}",
        );
        let d = r
            .diagnostics
            .iter()
            .find(|d| d.code == "E0086")
            .unwrap_or_else(|| {
                panic!(
                    "expected E0086, got {:?}",
                    r.diagnostics.iter().map(|d| &d.code).collect::<Vec<_>>()
                )
            });
        assert!(
            d.message.contains("async"),
            "E0086 must explain that await requires an async fn; got {:?}",
            d.message
        );
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

    /// **The enforced rule: `mut self` demands a mutable receiver whichever way
    /// the method was dispatched.** A trait method declaring `mut self`, called
    /// through an immutable binding, is `E0060` — the same answer the *inherent*
    /// path gives for the same operation
    /// (`mut_self_method_on_immutable_receiver_reports_e0060` a few tests up).
    /// One rule, one answer, which was the whole point: ADR 0005 §1 rejected
    /// Java/Python receiver semantics precisely because "a language where
    /// `v.push(x)` is allowed but `v.items[0] = x` is not has no rule a reader
    /// can hold in their head — it has two", and until this landed *trait*
    /// dispatch was a third answer to the same question.
    ///
    /// **This assertion was deliberately inverted, and that is the intended
    /// outcome, not a test bent to fit.** It previously read
    /// `assert!(r.diagnostics.is_empty())` under the name
    /// `…_is_not_enforced_on_immutable_receiver_known_gap`, pinning the
    /// permissive behaviour on purpose so that closing the gap could not happen
    /// silently. What authorised the flip: ADR 0005 §1's Migration path, whose
    /// three steps (the `hir::TraitMethod` flag, the conformance comparison, the
    /// call-site check) are now all executed and recorded there as done, and the
    /// associated-types plan and design doc §6, which made closing it a hard gate
    /// before the first `mut self` trait method — `Iterator::next` — could be
    /// declared at all.
    ///
    /// It keeps its narrow reach: it compiles one program and says nothing about
    /// any other. The routes into the same check that this shape does *not*
    /// exercise — a generic bound, a supertrait bound, a default body, string
    /// interpolation — are pinned individually by the `mut_self_trait_method_*`
    /// tests below, because one accepted program could never have proved the
    /// check was unbypassable.
    #[test]
    fn trait_method_mut_self_is_enforced_on_immutable_receiver() {
        let r = check_src(
            "trait Bump { fn bump(mut self) }\n\
             record P { v: Int }\n\
             impl Bump for P { fn bump(mut self) { self.v = self.v + 1 } }\n\
             fn main() { let p = P { v: 1 }\n p.bump()\n println(\"${p.v}\") }",
        );
        assert!(
            r.diagnostics.iter().any(|d| d.code == "E0060"),
            "the mutable-receiver rule is enforced on trait dispatch too \
             (ADR 0005 §1, Migration path): `p` is immutable, so `p.bump()` on a \
             `mut self` trait method must be E0060: {:?}",
            r.diagnostics
        );
    }

    #[test]
    fn mut_self_trait_method_on_an_immutable_receiver_reports_e0060() {
        let r = check_src(
            "trait Bump { fn bump(mut self) }\n\
             record C { n: Int }\n\
             impl Bump for C { fn bump(mut self) { self.n = self.n + 1 } }\n\
             fn main() { let c = C { n: 1 }\n c.bump() }",
        );
        assert!(
            r.diagnostics.iter().any(|d| d.code == "E0060"),
            "expected E0060 on a mut-self trait method through an immutable binding: {:?}",
            r.diagnostics
        );
    }

    #[test]
    fn mut_self_trait_method_on_a_mutable_receiver_is_accepted() {
        // The over-rejection guard for the common case: enforcing the rule on
        // trait dispatch must not make a legitimately `mut` binding unusable.
        let r = check_src(
            "trait Bump { fn bump(mut self) }\n\
             record C { n: Int }\n\
             impl Bump for C { fn bump(mut self) { self.n = self.n + 1 } }\n\
             fn main() { let mut c = C { n: 1 }\n c.bump()\n println(\"${c.n}\") }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    }

    #[test]
    fn a_plain_self_trait_method_is_unaffected_by_the_mut_self_trait_rule() {
        // The no-op half, and the mutation-killer for an unconditional check:
        // only `mut self` demands anything of the caller, so a reader still
        // works on an immutable binding. Mirrors
        // `plain_self_method_on_immutable_receiver_still_typechecks` on the
        // inherent path.
        let r = check_src(
            "trait Get { fn get(self) -> Int }\n\
             record C { n: Int }\n\
             impl Get for C { fn get(self) -> Int { self.n } }\n\
             fn main() { let c = C { n: 1 }\n println(\"${c.get()}\") }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    }

    #[test]
    fn mut_self_trait_method_conformance_disagreement_is_e0072() {
        // Both directions: the trait says mut, the impl does not, and vice
        // versa. Either way the receiver's mutability requirement would be
        // decided by whichever table the caller happened to consult — the call
        // site reads the trait's flag, `check_fn_body` reads the impl's `mut`.
        //
        // The two messages are pinned, not merely the code: a single `E0072`
        // assertion holds just as well if the want/got pair is swapped, and a
        // swapped pair tells the author to change the wrong side.
        for (t, i, got, want) in [
            (
                "mut self",
                "self",
                "a plain `self` receiver",
                "a `mut self` receiver",
            ),
            (
                "self",
                "mut self",
                "a `mut self` receiver",
                "a plain `self` receiver",
            ),
        ] {
            let src = format!(
                "trait Bump {{ fn bump({t}) }}\n\
                 record C {{ n: Int }}\n\
                 impl Bump for C {{ fn bump({i}) {{ }} }}\n\
                 fn main() {{ }}"
            );
            let r = check_src(&src);
            let msgs: Vec<&str> = r
                .diagnostics
                .iter()
                .filter(|d| d.code == "E0072")
                .map(|d| d.message.as_str())
                .collect();
            assert!(
                msgs.contains(
                    &format!("method `bump` has {got} but trait `Bump` declares {want}").as_str()
                ),
                "trait `{t}` vs impl `{i}` must be a conformance error naming both sides: {:?}",
                r.diagnostics
            );
        }
    }

    #[test]
    fn mut_self_trait_method_diagnostic_names_the_callee_and_advises_let_mut() {
        // The trait-dispatch spelling of the callee is the trait's declared
        // method name, unqualified — `arity_errors_name_the_callee_uniformly`
        // pins the same choice for `E0016` at this dispatch site, and for the
        // same reason: `Self` may still be a generic parameter, so there is no
        // impl to qualify with. The actionable note is shared with the other
        // three mutation forms via `require_mutable_place`.
        let r = check_src(
            "trait Bump { fn bump(mut self) }\n\
             record C { n: Int }\n\
             impl Bump for C { fn bump(mut self) { self.n = self.n + 1 } }\n\
             fn main() { let c = C { n: 1 }\n c.bump() }",
        );
        let d = r
            .diagnostics
            .iter()
            .find(|d| d.code == "E0060")
            .expect("an E0060 was reported");
        assert_eq!(
            d.message,
            "`bump` mutates its receiver, but `c` is immutable"
        );
        assert!(
            d.notes.iter().any(|n| n.contains("let mut c")),
            "{:?}",
            d.notes
        );
    }

    #[test]
    fn mut_self_trait_method_on_a_temporary_reports_e0060() {
        // ADR 0005 §1 Consequences: a temporary receiver is rejected, because
        // the mutation could not be observed by anyone. The trait path must
        // agree with the inherent one
        // (`mut_self_method_on_temporary_reports_e0060`) — one rule, one answer,
        // which is the whole point of closing this gap.
        let r = check_src(
            "trait Bump { fn bump(mut self) }\n\
             record C { n: Int }\n\
             impl Bump for C { fn bump(mut self) { self.n = self.n + 1 } }\n\
             fn make() -> C { C { n: 1 } }\n\
             fn main() { make().bump() }",
        );
        assert!(error_codes(&r).contains(&"E0060"), "{:?}", r.diagnostics);
    }

    #[test]
    fn mut_self_trait_method_on_a_field_of_a_mutable_root_is_accepted() {
        // `place_root` walks the whole projection chain on the trait path too,
        // so a nested receiver under a `mut` root is fine — the shape
        // `std/collections` uses, but reached through trait dispatch.
        let r = check_src(
            "trait Bump { fn bump(mut self) }\n\
             record Inner { n: Int }\n\
             record Outer { inner: Inner }\n\
             impl Bump for Inner { fn bump(mut self) { self.n = self.n + 1 } }\n\
             fn main() {\n\
                 let mut o = Outer { inner: Inner { n: 1 } }\n\
                 o.inner.bump()\n\
                 println(\"${o.inner.n}\")\n\
             }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    }

    #[test]
    fn mut_self_trait_method_through_a_generic_bound_reports_e0060() {
        // Route 2 of five. A generic receiver is the case ADR 0005 named as the
        // reason the gap existed: there is no single impl to read `mut self`
        // off, so the flag has to live on `hir::TraitMethod`. `x` is an
        // ordinary immutable parameter, so the rule must fire here exactly as
        // it does for a `let`.
        let r = check_src(
            "trait Bump { fn bump(mut self) }\n\
             record C { n: Int }\n\
             impl Bump for C { fn bump(mut self) { self.n = self.n + 1 } }\n\
             fn poke<T: Bump>(x: T) { x.bump() }\n\
             fn main() { let c = C { n: 1 }\n poke(c) }",
        );
        assert!(error_codes(&r).contains(&"E0060"), "{:?}", r.diagnostics);
    }

    #[test]
    fn mut_self_trait_method_through_a_mut_generic_parameter_is_accepted() {
        // The over-rejection mirror of the above, and a prerequisite for the
        // generic-projection gate (`fn first<I: Iterator>(…)` calls
        // `it.next()`): `mut` on a *parameter* parses and makes the parameter a
        // mutable root, so the generic route stays writable.
        let r = check_src(
            "trait Bump { fn bump(mut self) }\n\
             record C { n: Int }\n\
             impl Bump for C { fn bump(mut self) { self.n = self.n + 1 } }\n\
             fn poke<T: Bump>(mut x: T) { x.bump() }\n\
             fn main() { let c = C { n: 1 }\n poke(c) }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    }

    #[test]
    fn mut_self_trait_method_through_a_supertrait_bound_reports_e0060() {
        // Route 3 of five. `collect_traits` puts a trait's *expanded*
        // supertraits in the `Self` bound slot, so `x.bump()` under `T: Ext`
        // resolves to `Bump`'s method through a different trait's bound list.
        // The check must key on the trait the method was resolved *in*, not on
        // the bound that was written.
        let r = check_src(
            "trait Bump { fn bump(mut self) }\n\
             trait Ext: Bump { }\n\
             record C { n: Int }\n\
             impl Bump for C { fn bump(mut self) { self.n = self.n + 1 } }\n\
             impl Ext for C { }\n\
             fn poke<T: Ext>(x: T) { x.bump() }\n\
             fn main() { }",
        );
        assert!(error_codes(&r).contains(&"E0060"), "{:?}", r.diagnostics);
    }

    #[test]
    fn mut_self_trait_method_from_a_plain_self_default_body_reports_e0060() {
        // Route 4 of five, and the one a trait author hits: a default body
        // forgets its own `mut` and delegates to a mutator. Inside a default
        // body `self` is typed `Param(0)`, so the delegated call resolves
        // through `Self`'s own bound — a second trait-dispatch route into the
        // same check.
        let r = check_src(
            "trait Bump {\n\
                 fn bump(mut self)\n\
                 fn twice(self) { self.bump() }\n\
             }\n\
             record C { n: Int }\n\
             impl Bump for C { fn bump(mut self) { self.n = self.n + 1 } }\n\
             fn main() { }",
        );
        assert!(error_codes(&r).contains(&"E0060"), "{:?}", r.diagnostics);
    }

    #[test]
    fn mut_self_trait_method_from_a_mut_self_default_body_is_accepted() {
        // The over-rejection mirror: a `mut self` default body is a mutable
        // root, so delegating to another `mut self` method is exactly what the
        // declaration promises. Without this, closing the gap would make every
        // mutating default method unwritable.
        let r = check_src(
            "trait Bump {\n\
                 fn bump(mut self)\n\
                 fn twice(mut self) { self.bump() }\n\
             }\n\
             record C { n: Int }\n\
             impl Bump for C { fn bump(mut self) { self.n = self.n + 1 } }\n\
             fn main() { }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    }

    #[test]
    fn mut_self_trait_method_reached_by_string_interpolation_reports_e0060() {
        // Route 5 of five, and the one that is invisible from
        // `check_method_call`: `"${c}"` bridges to a user `fmt(self) -> String`
        // through `try_display`, which calls `emit_trait_call` directly. A check
        // installed only in `check_method_call`'s `MethodRes::Trait` arm leaves
        // this route accepting a mutation through an immutable binding — so the
        // check lives at `emit_trait_call`, the choke point every receiver
        // route passes through.
        let r = check_src(
            "trait Show { fn fmt(mut self) -> String }\n\
             record C { n: Int }\n\
             impl Show for C { fn fmt(mut self) -> String { \"c\" } }\n\
             fn main() { let c = C { n: 1 }\n println(\"${c}\") }",
        );
        assert!(error_codes(&r).contains(&"E0060"), "{:?}", r.diagnostics);
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

    // The iterable is `make_once()` and not the record literal `Once { … }`
    // the plan wrote, in this test and the two below: the parser parses the
    // `for` iterable in a `no_struct_literal` context (as Rust does), so
    // `for x in Once { v: 7 } { … }` takes the record literal's brace as the
    // loop body and cannot parse at all. Parenthesising does not help either —
    // Nova's paren-grouping inherits the flag rather than resetting it. A call
    // and a `let`-bound local are also the two receiver shapes real code uses
    // (`v.iter()` is a call), and they are the two `place_root` classifications
    // — `NotAPlace` and `ImmutableLocal` — that a desugar handing the *source*
    // iterable to the mutable-receiver check would wrongly reject.
    #[test]
    fn a_for_loop_iterates_an_iterator() {
        let r = check_src(
            "trait It { type Item\n fn next(mut self) -> Option<Self::Item> }\n\
             record Once { v: Int, done: Bool }\n\
             impl It for Once { type Item = Int\n\
              fn next(mut self) -> Option<Int> { if self.done { None } else { self.done = true\n Some(self.v) } } }\n\
             fn make_once() -> Once { Once { v: 7, done: false } }\n\
             fn main() { for x in make_once() { println(\"${x}\") } }",
        );
        assert_eq!(error_codes(&r), Vec::<&str>::new(), "{:?}", r.diagnostics);
    }

    /// Not in the plan; added because the plan's own mutation (c) — dropping the
    /// `None => break` arm — survives every one of its other assertions.
    /// Exhaustiveness analysis runs over *source* `match` expressions, not over
    /// one the checker synthesized, so a desugar that can never leave the loop
    /// type-checks completely clean and only fails at runtime: measured, the
    /// program prints its real elements and then dies on the switch's default
    /// `Terminator::Trap` (exit 132, `Illegal instruction`) — it does **not**
    /// hang. Swapping the `Some` and `None` variant indices likewise survives
    /// every diagnostic-based assertion. Both are structural faults, so this
    /// reads the structure the checker built rather than the diagnostics it
    /// didn't emit.
    #[test]
    fn a_for_loop_over_an_iterator_breaks_on_none() {
        let r = check_src(
            "trait It { type Item\n fn next(mut self) -> Option<Self::Item> }\n\
             record Once { v: Int, done: Bool }\n\
             impl It for Once { type Item = Int\n\
              fn next(mut self) -> Option<Int> { if self.done { None } else { self.done = true\n Some(self.v) } } }\n\
             fn make_once() -> Once { Once { v: 7, done: false } }\n\
             fn main() { for x in make_once() { println(\"${x}\") } }",
        );
        assert_eq!(error_codes(&r), Vec::<&str>::new(), "{:?}", r.diagnostics);
        let arms = exprs_in(&r.module, "main")
            .into_iter()
            .find_map(|e| match &e.kind {
                hir::ExprKind::Match { arms, .. } => Some(arms),
                _ => None,
            })
            .expect("the desugar built a `match` on `next()`");
        assert_eq!(arms.len(), 2, "{arms:?}");
        // The variant a pattern names, by *name* and binder count. Reading the
        // name and not just the binder count matters: `None` and a `Some` written
        // with no binders both have zero binders, so a binder-count assertion
        // alone lets the two indices be swapped — measured, that mutation
        // survives it.
        let names = |p: &hir::Pattern| match p {
            hir::Pattern::Variant {
                sum,
                variant,
                binders,
            } => {
                let s = r.module.sum(*sum).expect("the scrutinee's sum");
                (s.variants[*variant as usize].name.clone(), binders.len())
            }
            other => panic!("a variant pattern, not {other:?}"),
        };
        let (exit, elem): (Vec<&hir::Arm>, Vec<&hir::Arm>) = arms
            .iter()
            .partition(|a| matches!(a.body.kind, hir::ExprKind::Break));
        assert_eq!(exit.len(), 1, "exactly one arm leaves the loop: {arms:?}");
        assert_eq!(names(&exit[0].pattern), ("None".to_string(), 0));
        assert_eq!(names(&elem[0].pattern), ("Some".to_string(), 1));
    }

    #[test]
    fn a_for_loop_over_a_range_still_works() {
        // This task edits the function the range loop lives in, so the range
        // path needs its own assertion here rather than relying on the older
        // range tests being run.
        let r = check_src("fn main() { for i in 0..3 { println(\"${i}\") } }");
        assert_eq!(error_codes(&r), Vec::<&str>::new(), "{:?}", r.diagnostics);
    }

    #[test]
    fn a_for_loops_variable_binds_at_the_projected_item_type() {
        // `x` must be the normalized `Self::Item` (here `Bool`), not the
        // projection and not an inference variable. Bool rather than Int
        // deliberately: `mir_ty` collapses Int and Char to the same machine
        // type, so a wrong item type among them is invisible downstream.
        let r = check_src(
            "trait It { type Item\n fn next(mut self) -> Option<Self::Item> }\n\
             record OnceB { v: Bool, done: Bool }\n\
             impl It for OnceB { type Item = Bool\n\
              fn next(mut self) -> Option<Bool> { if self.done { None } else { self.done = true\n Some(self.v) } } }\n\
             fn main() { let it = OnceB { v: true, done: false }\n\
              for x in it { let y: Int = x\n let _ = y } }",
        );
        let d = r
            .diagnostics
            .iter()
            .find(|d| d.code == "E0010")
            .expect("E0010: the loop variable is Bool, not Int");
        assert!(d.message.contains("Bool"), "{}", d.message);
    }

    #[test]
    fn a_for_loops_variable_is_immutable() {
        // Same rule the range loop already enforces. Without it the desugar
        // could hand out a mutable binding by accident.
        let r = check_src(
            "trait It { type Item\n fn next(mut self) -> Option<Self::Item> }\n\
             record Once { v: Int, done: Bool }\n\
             impl It for Once { type Item = Int\n\
              fn next(mut self) -> Option<Int> { if self.done { None } else { self.done = true\n Some(self.v) } } }\n\
             fn main() { let it = Once { v: 7, done: false }\n\
              for x in it { x = 1 } }",
        );
        assert!(
            r.diagnostics.iter().any(|d| d.code == "E0060"),
            "{:?}",
            r.diagnostics
        );
    }

    #[test]
    fn a_for_loop_over_a_non_iterator_names_both_accepted_forms() {
        // The existing message says "anything but an integer range", which
        // becomes false the moment this task lands. `for x in v` is the mistake
        // people will actually make, so the text must mention `.iter()`.
        let r = check_src("fn main() { for x in 3 { println(\"${x}\") } }");
        let d = r
            .diagnostics
            .iter()
            .find(|d| d.code == "E0900")
            .expect("E0900 for a non-range non-iterator");
        assert!(
            d.message.contains("iter()"),
            "points at the fix: {}",
            d.message
        );
    }

    /// `check_for_iterator` pushes a scope holding its hidden `__it` local so
    /// that `emit_trait_call`'s `place_root` can resolve the receiver as a
    /// *path*, then pops it before the body is checked. That pop is the whole
    /// hygiene mechanism, and it had **no coverage**: with the single
    /// `fcx.scopes.pop()` line deleted, this test is the *only* failure in the
    /// whole workspace — measured, not inferred — and this compiles and prints
    /// `stole 2`:
    ///
    /// ```nova
    /// for x in v.iter() { match __it.next() { Some(y) => println("stole ${y}"), None => … } }
    /// ```
    ///
    /// Worth pinning specifically because the *range* desugar's version of the
    /// same property is structurally safer and yet is the one with a test
    /// (`for_loop_does_not_shadow_user_end`): its hidden locals are made with
    /// `new_local_unscoped` and never enter a scope at all, so there is nothing
    /// to leak. The iterator desugar cannot do that — `place_root` resolves an
    /// AST name against the scopes — so it inserts and then relies on a matching
    /// pop, which is exactly the shape that one deleted line breaks.
    ///
    /// Both directions are asserted, because they fail differently. `__it`
    /// unreachable from the body is `E0001`; a user's own `__it` staying visible
    /// and *unshadowed* is the same leak seen from the other side — under the
    /// mutation the body's `__it` resolves to the hidden `VecIter`, so the
    /// second half reports `E0013` (no `Display`) rather than compiling clean.
    #[test]
    fn a_for_loops_hidden_iterator_local_is_not_reachable_from_the_body() {
        let decls = "trait It { type Item\n fn next(mut self) -> Option<Self::Item> }\n\
                     record Once { v: Int, done: Bool }\n\
                     impl It for Once { type Item = Int\n\
                      fn next(mut self) -> Option<Int> { if self.done { None } else { self.done = true\n Some(self.v) } } }\n\
                     fn make_once() -> Once { Once { v: 7, done: false } }\n";
        let stolen = check_src(
            &(decls.to_string()
                + "fn main() { for x in make_once() {\n\
                    match __it.next() { Some(y) => println(\"stole ${y}\"), None => println(\"none\") } } }"),
        );
        let d = stolen
            .diagnostics
            .iter()
            .find(|d| d.code == "E0001")
            .expect("E0001: `__it` is not a name the body can see");
        assert!(d.message.contains("__it"), "names it: {}", d.message);

        // And it does not shadow a user's own binding of that name either: the
        // scope the desugar pushes is gone before the body is checked, so
        // `__it` here is still the `Int`.
        let shadowed = check_src(
            &(decls.to_string()
                + "fn main() { let __it = 99\n\
                    for x in make_once() { println(\"${x} ${__it}\") }\n\
                    println(\"${__it}\") }"),
        );
        assert_eq!(
            error_codes(&shadowed),
            Vec::<&str>::new(),
            "{:?}",
            shadowed.diagnostics
        );
    }

    /// `iterator_next`'s `if sum.variants.len() != 2` guard, which nothing
    /// pinned. It is reachable from ordinary source: `Some`/`None` are not
    /// reserved, so a user can declare a three-variant sum that still has an
    /// `Option`-shaped pair inside it and return that from `next`. Measured —
    /// `type Tri = | Some(Int) | None | Third` compiles and `Third` is
    /// constructible, so the shape below is writable, not hypothetical.
    ///
    /// The guard is load-bearing, not defensive. Without it the desugar reads
    /// `Some`/`None` off by name and builds a **two**-arm match over a
    /// three-variant sum, and exhaustiveness never runs on a synthesized match
    /// (see `a_for_loop_over_an_iterator_breaks_on_none`'s doc comment, which
    /// measured exactly that for a different mutation): a `Third` would reach
    /// the switch's default `Terminator::Trap` at runtime with no diagnostic
    /// anywhere. Measured, with the guard deleted: this test is the *only*
    /// failure in the whole workspace, and the program below then passes
    /// `nova check` with **zero** diagnostics and dies at run time with
    /// `Illegal instruction` (exit 132).
    ///
    /// So this asserts the rejection, and does it by code *and* by message,
    /// since `E0900` is this file's code for every unsupported form.
    #[test]
    fn a_next_returning_a_three_variant_sum_is_not_an_iterator() {
        let r = check_src(
            "type Tri = | Some(Int) | None | Third\n\
             record R { n: Int }\n\
             trait It3 { fn next(mut self) -> Tri }\n\
             impl It3 for R { fn next(mut self) -> Tri { Third } }\n\
             fn main() { let r = R { n: 0 }\n for x in r { println(\"${x}\") } }",
        );
        let d = r
            .diagnostics
            .iter()
            .find(|d| d.code == "E0900")
            .expect("E0900: a three-variant return is not `Option`-shaped");
        assert!(
            d.message.contains("Iterator"),
            "the not-an-iterator message, not some other E0900: {}",
            d.message
        );
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
    fn record_type_param_bound_no_longer_reports_e0900() {
        // Pre-Task-1 (iterator-finishing plan), this was rejected outright:
        // `record Keyed<K: Hash2, V>` parsed, but nothing honoured the bound —
        // `hir::RecordType` has no `bounds` field and monomorphization only
        // discharges *function* bounds — so it used to compile and run with
        // `NoHash` (which does not implement `Hash2`), a bound that meant
        // nothing. That silent acceptance was worse than an error, so it was
        // rejected, exactly as `trait B where Self: A` is.
        //
        // Since Task 1, the bound is a resolution scope rather than a
        // constraint (see the comment in `collect_records`): it exists so a
        // field type may name a projection on the bounded parameter, and is
        // deliberately not enforced at construction. So this now compiles
        // clean, `NoHash`'s missing `Hash2` impl notwithstanding.
        // `a_record_bound_is_not_enforced_at_construction` (below, in this
        // module) pins the same decision with a projection-shaped bound; this
        // test keeps the original two-parameter, real-trait-method shape as a
        // second, differently shaped guard on the same decision.
        let r = check_src(
            "trait Hash2 { fn h(self) -> Int }\n\
             record Keyed<K: Hash2, V> { k: K, v: V }\n\
             record NoHash { n: Int }\n\
             fn main() { let x = Keyed { k: NoHash { n: 1 }, v: 2 }\n println(\"${x.v}\") }",
        );
        assert_eq!(error_codes(&r), Vec::<&str>::new(), "{:?}", r.diagnostics);
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
    fn sum_type_param_bound_e0900_reports_every_bounded_param() {
        // `reject_type_param_bounds` is still live for sum types (Task 1 only
        // dropped the record caller) and its own doc comment promises "one
        // diagnostic per bounded parameter, so a second offender is not
        // hidden behind the first." Task 1 removed the record caller, so the
        // record-side version of this guard could not survive as a count: it
        // became *this* test, on the sum-type path that still has the
        // behavior, and the empty-diagnostic-list assertion that replaced it
        // for records lives in a separate test,
        // `multiple_record_type_param_bounds_no_longer_report_e0900` (just
        // below). Without this one, no test anywhere in this file would assert
        // an `E0900` *count* greater than one.
        //
        // An earlier version of this comment named a
        // `record_type_param_bound_e0900_reports_every_bounded_param` as
        // having been "rewritten by Task 1" — no test of that name exists, in
        // this file or anywhere else, so the citation pointed a reader at
        // nothing. Both surviving tests are named above instead.
        let r = check_src(
            "trait A { fn a(self) -> Int }\n\
             trait B { fn b(self) -> Int }\n\
             type Two<K: A, V: B> = | X(K) | Y(V)\n\
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
    fn multiple_record_type_param_bounds_no_longer_report_e0900() {
        // Pre-Task-1, one E0900 fired *per* bounded parameter, so a
        // multi-parameter record did not hide a second offender behind the
        // first. Since Task 1 (see `record_type_param_bound_no_longer_reports_e0900`
        // just above), the bound is a resolution scope rather than a rejected
        // constraint, for every bounded parameter — not just the first — so
        // this guards against a partial regression that reintroduced the
        // rejection for, say, only a record's first bounded parameter.
        let r = check_src(
            "trait A { fn a(self) -> Int }\n\
             trait B { fn b(self) -> Int }\n\
             record Two<K: A, V: B> { k: K, v: V }\n\
             fn main() { }",
        );
        assert_eq!(error_codes(&r), Vec::<&str>::new(), "{:?}", r.diagnostics);
    }

    #[test]
    fn bounds_on_supported_positions_do_not_report_e0900() {
        // The E0900 above must fire *only* for a bound on a sum type's own
        // parameter (since Task 1 of the iterator-finishing plan, a record's
        // own parameter no longer reports it at all — see
        // `record_type_param_bound_no_longer_reports_e0900`). Bounds on
        // functions, impl blocks, generic trait methods and `where` clauses
        // are all supported and are used throughout `std/`, which every
        // program compiles — a false positive here would break the whole
        // stdlib. An unbounded generic record/sum (how `std` actually writes
        // `Vec<T>` / `Map<K, V>`) must stay clean too.
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
    fn a_record_field_may_name_a_projection_on_a_bounded_parameter() {
        // The blocker this whole increment exists to remove. Without the bound
        // resolving here, a lazy `map` adapter cannot be written at all: its
        // field must be typed `fn(I::Item) -> U`.
        let r = check_src(
            "trait It { type Item\n fn next(mut self) -> Option<Self::Item> }\n\
             record M<I: It, U> { it: I, f: fn(I::Item) -> U }\n\
             fn main() { }",
        );
        assert_eq!(error_codes(&r), Vec::<&str>::new(), "{:?}", r.diagnostics);
    }

    #[test]
    fn a_record_bound_resolves_when_the_bounded_parameter_is_not_first() {
        // `resolve_bounds` returns one `Vec<DefId>` per generic parameter, in
        // declaration order, and `convert_ty` looks up a projection's bound
        // list by the same positional index that `generic_scope` assigned —
        // that free function is a three-line `generics.iter().enumerate()`, and
        // it is named rather than cited by line because the previous version of
        // this comment pointed at a line number that had since gone blank.
        // Every other test in
        // this file puts its bound on parameter index 0 (`M<I: It, U>`,
        // `M<I: It>`, `M<I: Sub>`), so a mutation that dropped unbounded
        // parameters' empty entries before indexing — shifting every later
        // index — would pass the whole suite. `U` (unbounded) is declared
        // first here specifically so `I`'s bound sits at index 1.
        let r = check_src(
            "trait It { type Item\n fn next(mut self) -> Option<Self::Item> }\n\
             record M<U, I: It> { it: I, f: fn(I::Item) -> U }\n\
             fn main() { }",
        );
        assert_eq!(error_codes(&r), Vec::<&str>::new(), "{:?}", r.diagnostics);
    }

    #[test]
    fn a_record_bound_naming_an_unknown_trait_is_e0001() {
        // Resolution must report, not skip. A silently-dropped bound would put
        // this increment straight back into the "accepted and quietly ignored"
        // family the spec's §3.2 warns about.
        let r = check_src(
            "record M<I: NoSuchTrait> { it: I }\n\
             fn main() { }",
        );
        let d = r
            .diagnostics
            .iter()
            .find(|d| d.code == "E0001")
            .expect("E0001 for an unresolvable record bound");
        assert!(
            d.message.contains("NoSuchTrait"),
            "names the trait: {}",
            d.message
        );
    }

    #[test]
    fn a_projection_on_an_unbounded_record_parameter_is_still_e0001() {
        // The bound is what makes the projection resolvable, so without one the
        // old error must remain. This is the guard against "resolve projections
        // against every trait in scope", which would accept nonsense.
        let r = check_src(
            "trait It { type Item\n fn next(mut self) -> Option<Self::Item> }\n\
             record M<I, U> { it: I, f: fn(I::Item) -> U }\n\
             fn main() { }",
        );
        assert!(
            r.diagnostics.iter().any(|d| d.code == "E0001"),
            "an unbounded parameter has no `Item`: {:?}",
            r.diagnostics
        );
    }

    #[test]
    fn a_bound_on_a_sum_type_parameter_is_still_e0900() {
        // Records only. Nothing in this increment needs a bound on a sum
        // parameter, and leaving the rejection in place halves the surface.
        let r = check_src(
            "trait It { type Item\n fn next(mut self) -> Option<Self::Item> }\n\
             type S<I: It> = | A(I) | B\n\
             fn main() { }",
        );
        let d = r
            .diagnostics
            .iter()
            .find(|d| d.code == "E0900")
            .expect("E0900 still rejects a bound on a sum type parameter");
        assert!(d.message.contains("sum type"), "{}", d.message);
    }

    #[test]
    fn a_record_bound_is_not_enforced_at_construction() {
        // The spec's §3.2 decision, pinned so it cannot drift silently in
        // either direction. `Int` is not an `It`, and building `M<Int, …>` is
        // accepted: the bound is a resolution scope, not a constraint.
        //
        // What makes that safe is NOT one uniform diagnostic. It is three
        // different answers depending on whether the bound reaches a field
        // type, and only the first two are diagnostics at all — see ADR 0007
        // §1, whose three cases are pinned in `crates/nova-cli/tests/
        // run_tests.rs` (they need monomorphization, so they cannot live
        // here):
        //   `a_wrong_instantiation_of_a_projection_shaped_record_is_e0079_at_
        //    construction`               — a field type NAMES the projection
        //                                  (std's `MapIter`/`FilterIter`):
        //                                  `E0079` at construction.
        //   `an_unused_record_bound_is_still_enforced_through_a_bounded_impl_
        //    method`                     — no field type does, but a bounded
        //                                  impl method is instantiated:
        //                                  `E0013`.
        //   `a_record_bound_no_field_type_uses_is_silently_accepted_when_
        //    never_exercised`            — neither: accepted, runs, prints.
        //                                  The residual hole, pinned as
        //                                  accepted.
        // `M` here is the third shape (`it: I` names no projection) and is
        // never used, so it is case 3. Earlier revisions of this comment
        // pointed at "the E0014 test below": wrong code, and no such test ever
        // existed.
        let r = check_src(
            "trait It { type Item\n fn next(mut self) -> Option<Self::Item> }\n\
             record M<I: It> { it: I }\n\
             fn main() { let m = M { it: 3 }\n let _ = m }",
        );
        assert_eq!(error_codes(&r), Vec::<&str>::new(), "{:?}", r.diagnostics);
    }

    #[test]
    fn a_record_field_initializer_normalizes_a_projection_once_its_parameter_is_concrete() {
        // Regression test for a bug found implementing the iterator-finishing
        // plan's Task 3 (`MapIter`/`FilterIter` in std/core/lib.nova): building
        // a concrete instance of a record whose field type names a projection
        // on the record's own bounded parameter. `a_record_field_may_name_a_
        // projection_on_a_bounded_parameter`, above, only ever checks the
        // *declaration* — its `fn main() {}` is empty and never constructs one.
        //
        // `check_record_literal` used to compute each field's expected type
        // with a raw `field.ty.subst(&type_args)`, with no normalization step.
        // Once `it: Counter { n: 0 }` pins `I` to the concrete `Counter`, `f`'s
        // declared type substitutes to `fn(Assoc { on: Counter, name: "Item" })
        // -> ?U` — an *unnormalized* projection — and unifying that against the
        // closure's real type `fn(Int) -> Int` failed structurally, since
        // `unify` never normalizes (see `Checker::normalize`'s doc comment,
        // "Never called from `unify`"). `|x| x + 1`, not the bare identity
        // `|x| x`, is deliberate: a fully generic closure would unify against
        // the unnormalized projection just as well as against `Int` (a fresh
        // variable binds to either), so nothing would fail and this test would
        // pass regardless of the bug. Forcing `x: Int` through `+` is what
        // makes the mismatch (`Int` against `Assoc { .. }`) structural, and so
        // visible.
        //
        // Fixed by routing both of `check_record_literal`'s field-type call
        // sites through `instantiate` (subst, then apply current bindings and,
        // if a projection remains, normalize through the impl table) instead of
        // a raw `subst` — the same seam `check_direct_call` and
        // `emit_trait_call` already used.
        let r = check_src(
            "trait It { type Item\n fn next(mut self) -> Option<Self::Item> }\n\
             record M<I: It, U> { it: I, f: fn(I::Item) -> U }\n\
             record Counter { n: Int }\n\
             impl It for Counter { type Item = Int\n fn next(mut self) -> Option<Int> { None } }\n\
             fn main() { let m = M { it: Counter { n: 0 }, f: |x| x + 1 }\n let _ = m }",
        );
        assert_eq!(error_codes(&r), Vec::<&str>::new(), "{:?}", r.diagnostics);
    }

    #[test]
    fn a_record_field_initializer_is_order_sensitive_when_it_names_a_projection() {
        // A known, documented limitation of the fix above (see the comment at
        // its call site in `check_record_literal`), not a regression: this
        // pins *current* behavior so a future fix to field-order independence
        // has to touch this test deliberately rather than leave it silently
        // wrong.
        //
        // `check_record_literal` walks `for init in fields` in the *literal's
        // written* order. The sibling test just above writes `it` before `f`,
        // so by the time `f`'s expected type is computed, `it: Counter { n: 0
        // }` has already pinned `I` to `Counter` via `unify`, and `instantiate`
        // has something concrete to normalize `I::Item` against. Here the two
        // fields are swapped — same record, same values, same closure — so
        // `I` is still a free inference variable when `f`'s turn comes:
        // `instantiate`'s own `has_assoc()` check finds the projection, but
        // `icx.apply` on an unbound variable is a no-op, so there is nothing
        // yet to normalize *through*. The result is the exact pre-fix
        // symptom: an `E0010` naming an unresolved `Assoc`, cascading into
        // `E0011`s for the variables it left unpinned. Measured on *this*
        // source: one `E0010` and six `E0011`s. The assertion below
        // deliberately checks only the `E0010` — the cascade is a consequence
        // of the failure, not the property under test, and pinning its exact
        // shape would make this test fail for uninteresting reasons.
        //
        // **The cascade is a property of this shape, not of the field swap,
        // so do not carry it to another one.** `E0011` fires only for the
        // variables *nothing later pins*, and here `m` is bound and never
        // used, so `I` and `U` both stay free. Swap the same two fields in a
        // literal that is subsequently driven and the cascade disappears
        // entirely: `tests/runtime/iterator.nova`'s CHAIN block measures
        // exactly one `E0010` and zero `E0011`s from this same swap, because
        // its `it:` initializer still pins `I` and its `next()` calls pin `U`.
        // That fixture's comment briefly claimed a cascade transcribed from
        // here; the count was never true there. Same over-generalization ADR
        // 0007 §1 exists to record.
        //
        // This is the same class of hole `instantiate`'s own doc comment
        // already admits for a call (`fn f<I: It>(y: I::Item, x: I)` still
        // fails) — but wider here, since a function's parameter order is
        // fixed by its signature while a record literal's field order is free
        // syntax with nothing to read an ordering requirement off of.
        let r = check_src(
            "trait It { type Item\n fn next(mut self) -> Option<Self::Item> }\n\
             record M<I: It, U> { it: I, f: fn(I::Item) -> U }\n\
             record Counter { n: Int }\n\
             impl It for Counter { type Item = Int\n fn next(mut self) -> Option<Int> { None } }\n\
             fn main() { let m = M { f: |x| x + 1, it: Counter { n: 0 } }\n let _ = m }",
        );
        assert!(
            error_codes(&r).contains(&"E0010"),
            "documents a known limitation (field order matters when a field \
             names a projection); if this now fails, the limitation is fixed \
             and this test should be updated to assert a clean compile \
             instead: {:?}",
            r.diagnostics
        );
    }

    #[test]
    fn a_record_bound_resolves_a_projection_through_a_supertrait() {
        // `collect_records` calls `expand_bounds` after `resolve_bounds`, so a
        // record's bound list carries transitive supertraits exactly like a
        // function's or an impl's does (see `convert_ty`'s two-segment path
        // case, the `Some(idx)` arm of its `match by_index`: "`expand_bounds`
        // has already folded supertraits into every entry here" — named by
        // function and arm rather than by line, because the line number this
        // comment used to quote was already two dozen lines stale). `Sub: It`
        // declares no `Item` itself — only its supertrait `It` does — so
        // resolving `I::Item` against a bound of just `Sub` requires that
        // fold-in. This is TDD-by-mutation Step 7's second case: skipping
        // `expand_bounds` while still passing `&bounds` (rather than `&[]`)
        // reproduces the exact failure this test pins.
        let r = check_src(
            "trait It { type Item\n fn next(mut self) -> Option<Self::Item> }\n\
             trait Sub: It { fn extra(self) -> Int }\n\
             record M<I: Sub> { f: fn(I::Item) -> Int }\n\
             fn main() { }",
        );
        assert_eq!(error_codes(&r), Vec::<&str>::new(), "{:?}", r.diagnostics);
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
    fn a_trait_records_its_associated_types_in_order() {
        let r = check_src("trait Pair { type A\n type B\n fn get(self) -> Int }\nfn main() { }");
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
        let t = r
            .module
            .traits
            .iter()
            .find(|t| t.name == "Pair")
            .expect("trait Pair collected");
        let names: Vec<&str> = t.assoc_types.iter().map(|(n, _)| n.as_str()).collect();
        // Order matters: it is declaration order, and two associated types is
        // the case that catches an implementation assuming there is only one.
        assert_eq!(names, ["A", "B"]);
        // Each gets its own DefId, so `display_ty` can name it.
        assert_ne!(t.assoc_types[0].1, t.assoc_types[1].1);
    }

    #[test]
    fn a_bound_on_an_associated_type_reports_e0900() {
        // Rejected rather than silently dropped — the same rule this project
        // applies to record and sum type-parameter bounds, because a bound
        // that enforces nothing is worse than no bound.
        let r = check_src("trait It { type Item: Display }\nfn main() { }");
        let d = r
            .diagnostics
            .iter()
            .find(|d| d.code == "E0900")
            .expect("E0900 for a bound on an associated type");
        assert!(
            d.message.contains("associated type"),
            "message should name the construct: {}",
            d.message
        );
    }

    #[test]
    fn a_projection_on_a_generic_parameter_resolves() {
        // `I::Item` where I is a generic parameter bounded by a trait that
        // declares `Item`. No impl is needed: the projection stays abstract.
        let r = check_src(
            "trait It { type Item\n fn get(self) -> Int }\n\
             fn first<I: It>(x: I) -> I::Item { panic(\"unreachable\") }\n\
             fn main() { }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    }

    #[test]
    fn a_module_qualified_type_path_still_reports_e0900() {
        // The two-segment path case now has a second meaning; the original
        // one must survive with its original message.
        let r = check_src("fn f(x: some_mod::Thing) -> Int { 1 }\nfn main() { }");
        let d = r
            .diagnostics
            .iter()
            .find(|d| d.code == "E0900")
            .expect("E0900 for a module-qualified type path");
        assert!(
            d.message.contains("module-qualified type paths"),
            "original message preserved: {}",
            d.message
        );
    }

    #[test]
    fn rejected_type_arguments_and_an_undeclared_name_do_not_contradict_each_other() {
        // `I::Nope<Int>` combines both problems the projection branch can
        // report: type arguments on a projection (rejected, E0012) and an
        // undeclared associated-type name (E0001). Both fire — the E0012
        // guard runs before resolve_projection, deliberately, so args are
        // rejected whether or not the name even resolves — and neither may
        // claim something the other disproves: E0012 must not call `Nope`
        // "an associated type" in the same breath E0001 says it is not one.
        let r = check_src(
            "trait It { type Item\n fn get(self) -> Int }\n\
             fn f<I: It>(x: I) -> I::Nope<Int> { panic(\"unreachable\") }\n\
             fn main() { }",
        );
        let e0012 = r
            .diagnostics
            .iter()
            .find(|d| d.code == "E0012")
            .expect("E0012 for the rejected type arguments");
        assert!(
            !e0012.message.contains("associated type"),
            "must not call `Nope` an associated type before E0001 says it is not one: {}",
            e0012.message
        );
        let e0001 = r
            .diagnostics
            .iter()
            .find(|d| d.code == "E0001")
            .expect("E0001 for the undeclared associated type name");
        assert!(
            e0001.message.contains("no associated type") && e0001.message.contains("Nope"),
            "{}",
            e0001.message
        );
    }

    #[test]
    fn a_projection_naming_an_undeclared_associated_type_is_an_error() {
        // Not just "some diagnostic fired": a lazy implementation that
        // reused `self.unsupported(span, "module-qualified type paths")` for
        // the not-found case (E0900, "... are not supported yet") would also
        // satisfy a bare `!diagnostics.is_empty()` — so this pins the actual
        // code and that the message names the real construct.
        let r = check_src(
            "trait It { type Item\n fn get(self) -> Int }\n\
             fn f<I: It>(x: I) -> I::Nope { panic(\"unreachable\") }\n\
             fn main() { }",
        );
        let d = r
            .diagnostics
            .iter()
            .find(|d| d.code == "E0001")
            .expect("E0001 for a projection naming an undeclared associated type");
        assert!(
            d.message.contains("associated type"),
            "message should name the construct: {}",
            d.message
        );
    }

    #[test]
    fn two_traits_with_the_same_associated_type_name_get_distinct_defids() {
        // The `trait_def == def_id` guard in `collect_traits` (where
        // `assoc_type_ids` is filtered) is exactly what a single-trait test
        // cannot exercise: two unrelated traits both declaring `Item` must
        // not be cross-assigned to each other's associated type.
        let r = check_src(
            "trait A { type Item\n fn a(self) -> Int }\n\
             trait B { type Item\n fn b(self) -> Int }\n\
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
        let a_item = a
            .assoc_types
            .iter()
            .find(|(n, _)| n.as_str() == "Item")
            .expect("A::Item")
            .1;
        let b_item = b
            .assoc_types
            .iter()
            .find(|(n, _)| n.as_str() == "Item")
            .expect("B::Item")
            .1;
        assert_ne!(a_item, b_item, "A::Item and B::Item must not share a DefId");
    }

    #[test]
    fn a_projection_ambiguous_between_two_bounds_is_an_error() {
        // `expand_bounds` folds supertraits in *before* `convert_ty` runs, so
        // a parameter can legitimately see associated types declared by more
        // than one of its bounds — this is the case that makes that
        // reachable, not hypothetical. Two unrelated traits, both declaring
        // `Item`, both bounding the same parameter: an implementation that
        // picked the first match (e.g. the first bounding trait, or always
        // `assoc_types[0]`) would silently resolve to the wrong one instead
        // of reporting the ambiguity.
        let r = check_src(
            "trait A { type Item\n fn a(self) -> Int }\n\
             trait B { type Item\n fn b(self) -> Int }\n\
             fn f<I: A + B>(x: I) -> I::Item { panic(\"unreachable\") }\n\
             fn main() { }",
        );
        assert!(
            r.diagnostics.iter().any(|d| d.code == "E0015"),
            "expected an ambiguity error: {:?}",
            r.diagnostics
        );
    }

    #[test]
    fn a_projection_resolves_through_a_transitive_supertrait_bound() {
        // `expand_bounds` folds supertraits into a bound list before
        // `convert_ty` runs, so a bound on `B` (which requires `A`) also
        // carries `A` — `I::Item` must resolve against `A`'s declaration even
        // though `I` is only written `I: B`, not `I: A`. Desirable, but a
        // consequence of ordering rather than a decision made in
        // `resolve_projection` itself (see its comment in `convert_ty`); this
        // is the test that would fail if that ordering ever regressed.
        let r = check_src(
            "trait A { type Item\n fn a(self) -> Int }\n\
             trait B: A { fn b(self) -> Int }\n\
             fn f<I: B>(x: I) -> I::Item { panic(\"unreachable\") }\n\
             fn main() { }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    }

    #[test]
    fn a_projection_resolves_to_the_bounding_traits_own_associated_type_not_an_unrelated_one() {
        // Two traits both declare `Item`, but only ONE (`Keeper`) bounds `I`
        // here. A lookup that ignored *which* trait `I` is actually bounded
        // by — matching the name alone, anywhere in the program — could
        // silently resolve to `Other::Item` instead of `Keeper::Item`, and a
        // test that only asserts "compiles clean" would not notice: both
        // outcomes typecheck. So this inspects the compiled `Ty::Assoc`
        // itself and pins both halves of it — the projection's `on` (must be
        // `Param(0)`, i.e. `I`, never some other parameter or a `Var`) and
        // its `assoc` (must be `Keeper`'s `Item`, not `Other`'s or a fresh
        // one).
        let r = check_src(
            "trait Other { type Item\n fn o(self) -> Int }\n\
             trait Keeper { type Item\n fn get(self) -> Int }\n\
             fn first<I: Keeper>(x: I) -> I::Item { panic(\"unreachable\") }\n\
             fn main() { }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
        let keeper = r
            .module
            .traits
            .iter()
            .find(|t| t.name == "Keeper")
            .expect("trait Keeper collected");
        let keeper_item = keeper
            .assoc_types
            .iter()
            .find(|(n, _)| n.as_str() == "Item")
            .expect("Keeper::Item")
            .1;
        let f = r
            .module
            .functions
            .iter()
            .find(|f| f.name == "first")
            .expect("fn first collected");
        match &f.ret_ty {
            Ty::Assoc { on, assoc } => {
                assert_eq!(
                    on.as_ref(),
                    &Ty::Param(0),
                    "projected on `I` (Param 0), not a Var or some other parameter"
                );
                assert_eq!(
                    *assoc, keeper_item,
                    "must resolve to Keeper::Item, not Other::Item"
                );
            }
            other => panic!("expected `first`'s return type to be a Ty::Assoc, got {other:?}"),
        }
    }

    #[test]
    fn a_projection_projects_onto_the_correct_parameter_when_there_are_several() {
        // `I` is the SECOND generic parameter here (`T` is first, unrelated
        // and unbounded). Every other projection test in this file happens
        // to project onto a lone parameter at index 0, so a resolver that
        // hardcoded `Ty::Param(0)` for the projection's `on` — instead of the
        // actual index `base` resolved to — would still pass all of them.
        let r = check_src(
            "trait It { type Item\n fn get(self) -> Int }\n\
             fn f<T, I: It>(x: T, y: I) -> I::Item { panic(\"unreachable\") }\n\
             fn main() { }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
        let it = r
            .module
            .traits
            .iter()
            .find(|t| t.name == "It")
            .expect("trait It collected");
        let item = it
            .assoc_types
            .iter()
            .find(|(n, _)| n.as_str() == "Item")
            .expect("It::Item")
            .1;
        let f = r
            .module
            .functions
            .iter()
            .find(|f| f.name == "f")
            .expect("fn f collected");
        assert_eq!(
            f.ret_ty,
            Ty::Assoc {
                on: Box::new(Ty::Param(1)),
                assoc: item,
            },
            "must project onto `I` at Param(1), not `T` at Param(0)"
        );
    }

    #[test]
    fn a_projection_resolves_against_its_own_parameters_bounds_not_every_parameters() {
        // Two generic parameters, each bounded by a DIFFERENT trait, and the
        // projection is onto the FIRST one — the shape the existing
        // two-parameter test above does not cover, because there `T` (index
        // 0) is unbounded and only `I` (index 1) has any bounds at all. A
        // resolver that read the wrong bound list for `idx` — e.g.
        // concatenating every parameter's bounds instead of indexing just
        // this one's — would still pass every projection test in this file,
        // because in every one of them there is only a single non-empty
        // bound list for such a mistake to fall into. Here `A: Foo` (declares
        // `Item`, not `Other`) and `B: It` (declares `Other`); projecting
        // `A::Other` must fail, because `Other` is not on any bound of `A`,
        // even though it *is* on a bound of some parameter in scope.
        let r = check_src(
            "trait Foo { type Item\n fn f(self) -> Int }\n\
             trait It { type Other\n fn get(self) -> Int }\n\
             fn f<A: Foo, B: It>(x: A, y: B) -> A::Other { panic(\"x\") }\n\
             fn main() { }",
        );
        let d = r
            .diagnostics
            .iter()
            .find(|d| d.code == "E0001")
            .expect("E0001: `Other` is not on any bound of `A`");
        assert!(
            d.message.contains("Other") && d.message.contains("bound of `A`"),
            "names both the missing associated type and the parameter: {}",
            d.message
        );
    }

    #[test]
    fn a_projection_with_type_arguments_reports_e0012() {
        // Nova has no generic associated types, so `I::Item<Int>` must not
        // silently drop the `<Int>` and resolve exactly as plain `I::Item`
        // would — that would let a program compile clean while meaning
        // something its source does not actually say.
        let r = check_src(
            "trait It { type Item\n fn get(self) -> Int }\n\
             fn first<I: It>(x: I) -> I::Item<Int> { panic(\"unreachable\") }\n\
             fn main() { }",
        );
        let d = r
            .diagnostics
            .iter()
            .find(|d| d.code == "E0012")
            .expect("E0012 for type arguments on a projection");
        assert!(
            d.message.contains("`I::Item` takes no type arguments"),
            "{}",
            d.message
        );
        // Deliberately NOT "associated type `I::Item` takes no type
        // arguments": `I::Nope<Int>` reports this same E0012 alongside
        // resolve_projection's own E0001 for the undeclared `Nope`, and
        // calling something "an associated type" that then turns out not to
        // be one would read as self-contradictory.
        assert!(
            !d.message.contains("associated type"),
            "must not call the base an \"associated type\" before it is known to resolve: {}",
            d.message
        );
    }

    #[test]
    fn rejected_type_arguments_on_a_projection_are_not_themselves_resolved() {
        // `args` is rejected wholesale (E0012) rather than converted, exactly
        // like the two single-segment branches (generic parameter,
        // primitive) that also reject type arguments without recursing into
        // them. That is intentional, not an oversight, so this pins it as a
        // positive fact rather than merely "some diagnostic fired": `Nope`
        // must never surface its own `cannot find type` (E0001) — if it did,
        // rejected arguments would secretly still be getting resolved, and a
        // future change making that happen should have to update this test
        // rather than slip in silently under a bare `!diagnostics.is_empty()`.
        let r = check_src(
            "trait It { type Item\n fn get(self) -> Int }\n\
             fn first<I: It>(x: I) -> I::Item<Nope> { panic(\"unreachable\") }\n\
             fn main() { }",
        );
        assert_eq!(
            r.diagnostics.len(),
            1,
            "expected exactly the one E0012 for the rejected argument: {:?}",
            r.diagnostics
        );
        assert_eq!(r.diagnostics[0].code, "E0012", "{:?}", r.diagnostics);
        assert!(
            !r.diagnostics.iter().any(|d| d.message.contains("Nope")),
            "`Nope` must not be individually resolved or diagnosed: {:?}",
            r.diagnostics
        );
    }

    #[test]
    fn future_displays_with_its_output_type_in_a_real_diagnostic() {
        // display_ty must render the output, not a bare "Future". Two different
        // futures printing the same string is the `T{i}` debt this project already
        // carries in diagnostics; do not add another instance of it.
        //
        // Asserted through a real mismatch message rather than by calling
        // display_ty directly: `CheckResult` is `{ module, diagnostics }` and
        // exposes no `Definitions`, so a direct call would need a second resolve
        // in the test. Going through the diagnostic is also the stronger test —
        // it is the path a user actually sees.
        let r = check_src(
            "fn take(x: Future<Int>) -> Int { 1 }\n\
             fn f(y: Future<Float>) -> Int { take(y) }\n\
             fn main() {}",
        );
        let msgs: Vec<String> = r.diagnostics.iter().map(|d| d.message.clone()).collect();
        assert!(
            msgs.iter()
                .any(|m| m.contains("Future<Int>") && m.contains("Future<Float>")),
            "expected a message naming both futures by their output types, got {msgs:?}"
        );
    }

    #[test]
    fn bare_future_without_a_type_argument_is_rejected() {
        // `Future` takes exactly one argument. Both the zero-argument and the
        // two-argument spellings must be diagnosed, not silently accepted --
        // this is the arity path that no existing built-in type name exercises,
        // because Int/Float/Bool/Char/String are all nullary.
        let r = check_src("fn f(x: Future) -> Int { 1 }\nfn main() {}");
        assert!(
            r.diagnostics.iter().any(|d| d.code == "E0012"),
            "expected E0012, got {:?}",
            r.diagnostics.iter().map(|d| &d.code).collect::<Vec<_>>()
        );
    }

    #[test]
    fn future_with_two_type_arguments_is_rejected() {
        let r = check_src("fn f(x: Future<Int, Bool>) -> Int { 1 }\nfn main() {}");
        assert!(r.diagnostics.iter().any(|d| d.code == "E0012"));
    }

    #[test]
    fn future_of_int_and_future_of_float_do_not_unify() {
        // The unifier must descend into the output type. An arm that unified any
        // two Futures would make `Future<Int>` and `Future<Float>` interchangeable.
        let r = check_src(
            "fn take(x: Future<Int>) -> Int { 1 }\n\
             fn f(y: Future<Float>) -> Int { take(y) }\n\
             fn main() {}",
        );
        assert!(
            r.diagnostics.iter().any(|d| d.code == "E0010"),
            "expected a type mismatch, got {:?}",
            r.diagnostics.iter().map(|d| &d.code).collect::<Vec<_>>()
        );
    }

    #[test]
    fn future_of_int_unifies_with_itself() {
        // The discriminating half of the test above, and NOT redundant with it:
        // an implementation whose `Future` unify arm always FAILED would satisfy
        // the mismatch test perfectly. Only this one rejects that.
        let r = check_src(
            "fn take(x: Future<Int>) -> Int { 1 }\n\
             fn f(y: Future<Int>) -> Int { take(y) }\n\
             fn main() {}",
        );
        assert!(
            r.diagnostics.is_empty(),
            "Future<Int> must unify with Future<Int>, got {:?}",
            r.diagnostics
                .iter()
                .map(|d| d.message.clone())
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn future_used_as_a_call_qualifier_is_rejected() {
        // Measured, not assumed (there is no reason to expect a specific code
        // here a priori): `Future::x()` reports E0900 "module-qualified paths
        // are not supported yet" -- the same generic fallback an ordinary
        // undeclared-type qualifier gets. It is neither a `resolve_type`-style
        // "unknown type" message nor a successful call.
        //
        // Kills a `"Future" => return Some(_)` mutant in `qualifier_self_ty`
        // (e.g. a stray `Some(Ty::Error)`, or miscopying a neighbouring
        // `"X" => return Some(Ty::X)` line): that would route into the
        // assoc-fn-lookup branch instead, which -- finding nothing for `x`,
        // and `resolve_type("Future")` still `None` since nothing declares it
        // -- reports the *different* E0001 "no associated function `x` on
        // type `Future`" and returns early, never reaching this fallback.
        //
        // Does NOT kill a mutant that deletes the arm outright: with nothing
        // named `Future` declared, `resolve_type` also answers `None`, so
        // deletion is a no-op here. Since the built-in type names were
        // reserved, that mutant is a no-op everywhere: declaring a type
        // named `Future` is itself `E0089`
        // (`declaring_a_type_named_for_a_builtin_is_rejected`), so no
        // `Definitions` reachable through `resolve()` ever answers `Some`
        // for `resolve_type(_, "Future")`.
        // `future_qualifier_short_circuits_before_resolve_type` below records
        // that the mutant it used to catch is no longer catchable at all.
        let r = check_src("fn f() { Future::x() }\nfn main() { }");
        assert_eq!(error_codes(&r), ["E0900"], "{:?}", r.diagnostics);
    }

    #[test]
    fn future_qualifier_short_circuits_before_resolve_type() {
        // White-box, not a `check_src` integration test: `qualifier_self_ty`'s
        // return only becomes *observable* through `check_call` when a
        // matching inherent/trait associated function is found for the
        // qualifier, which needs a real `impl` block on a type named
        // `Future` -- and such a block cannot be declared at all, since its
        // header fails `convert_ty`'s arity check first (see
        // `bare_future_without_a_type_argument_is_rejected`).
        //
        // This test used to drive the method past `check()` entirely by
        // `resolve()`-ing a program that declared `record Future { v: Int }`,
        // which compiled clean and gave `resolve_type(module, "Future")` a
        // record to find -- exactly the defect that reserving the built-in
        // type names closes. Declaring a type named `Future` is now `E0089`
        // (`declaring_a_type_named_for_a_builtin_is_rejected`), and
        // `Definitions` has no way to register one other than through that
        // now-rejecting path, so `resolve_type(_, "Future")` is `None` for
        // every `Definitions` reachable through `resolve()`. The arm-deleted
        // mutant this test used to kill is therefore no longer
        // distinguishable from correct behaviour by any input -- the same
        // conclusion `future_used_as_a_call_qualifier_is_rejected` above
        // already reaches for its own case. What remains below is a plain
        // pin of the arm's return value against an otherwise-empty
        // `Definitions`.
        let file_id = FileId::DUMMY;
        let src = "fn main() { }";
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
        // A `Checker` with otherwise-empty tables: `qualifier_self_ty`'s
        // `"Future"` arm returns before it would ever touch `fcx`,
        // `self.impls`, or any other field a full `check()` pass would
        // populate, so this needs only enough of a `Checker` to compile --
        // mirroring `check()`'s own construction above.
        let file = ast::File { items: Vec::new() };
        let checker = Checker {
            file: &file,
            defs: &resolved.definitions,
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
            impl_self: None,
            impl_selves: FxHashMap::default(),
            extra_functions: Vec::new(),
            next_closure_def: resolved.definitions.defs().len() as u32,
            type_arity: FxHashMap::default(),
            externs: Vec::new(),
            diagnostics: Vec::new(),
        };
        let mut fcx = FnCtx {
            icx: InferCtx::default(),
            locals: Vec::new(),
            scopes: Vec::new(),
            generics: FxHashMap::default(),
            param_bounds: Vec::new(),
            ret_ty: Ty::Unit,
            loop_depth: 0,
            in_async: false,
            pending_closures: Vec::new(),
        };
        assert_eq!(
            checker.qualifier_self_ty(&mut fcx, "Future"),
            None,
            "the reserved qualifier must not resolve to a same-named user type"
        );
    }

    #[test]
    fn every_reserved_nullary_names_qualifier_resolves_to_its_own_primitive() {
        // Pins `qualifier_self_ty`'s table, which
        // `every_reserved_name_really_is_a_builtin_type` does not reach: that
        // test only ever annotates a parameter (`fn f(x: {ann}) -> Int`),
        // never a qualifier (`Int::zero()`). White-box for the same reason
        // `future_qualifier_short_circuits_before_resolve_type` above is: the
        // qualifier only becomes observable through `check_call` when a
        // matching associated function is found afterward, which needs an
        // `impl` this test does not care about constructing.
        //
        // `Future` is excluded: it is not nullary, and its own arm returns
        // `None` rather than a `Ty`, unpinned.
        let file_id = FileId::DUMMY;
        let src = "fn main() { }";
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
        let file = ast::File { items: Vec::new() };
        let checker = Checker {
            file: &file,
            defs: &resolved.definitions,
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
            impl_self: None,
            impl_selves: FxHashMap::default(),
            extra_functions: Vec::new(),
            next_closure_def: resolved.definitions.defs().len() as u32,
            type_arity: FxHashMap::default(),
            externs: Vec::new(),
            diagnostics: Vec::new(),
        };
        let mut fcx = FnCtx {
            icx: InferCtx::default(),
            locals: Vec::new(),
            scopes: Vec::new(),
            generics: FxHashMap::default(),
            param_bounds: Vec::new(),
            ret_ty: Ty::Unit,
            loop_depth: 0,
            in_async: false,
            pending_closures: Vec::new(),
        };
        for name in nova_resolver::RESERVED_TYPE_NAMES {
            if name == "Future" {
                continue;
            }
            let expected = match name {
                "Int" => Ty::Int,
                "Float" => Ty::Float,
                "Bool" => Ty::Bool,
                "Char" => Ty::Char,
                "String" => Ty::String,
                "Bytes" => Ty::Bytes,
                _ => panic!("RESERVED_TYPE_NAMES grew a name this test does not know: {name}"),
            };
            assert_eq!(
                checker.qualifier_self_ty(&mut fcx, name),
                Some(expected),
                "`{name}::_()`'s qualifier must resolve to the primitive"
            );
        }
    }

    #[test]
    fn a_user_written_self_type_parameter_is_rejected_in_an_impl() {
        // Was `a_user_written_self_type_parameter_makes_self_item_resolve_in_
        // an_impl`, which pinned that this program compiled clean. It did, and
        // that was the problem: `Self` is an accepted identifier
        // (`Token::SelfUpper` parses to the plain string `"Self"`), so a
        // user-written `<Self>` landed an ordinary `generic_scope` entry and
        // `Self::Item` resolved to `Assoc { on: Param(0) }` — a *second*
        // meaning of `Self` in the very scope where `Self` also means the
        // impl's own self type. Rejecting the name is what makes that
        // unambiguous, so the same program is now an error. Same program on
        // purpose: it is the only pin on this shape.
        let r = check_src(
            "trait It { type Item\n fn get(self) -> Int }\n\
             record W<T> { v: T }\n\
             impl<Self: It> W<Self> { fn peek(self) -> Self::Item { panic(\"x\") } }\n\
             fn main() { }",
        );
        let d = r
            .diagnostics
            .iter()
            .find(|d| d.code == "E0076")
            .expect("E0076 for a type parameter named `Self`");
        assert!(
            d.message.contains("Self"),
            "names the parameter: {}",
            d.message
        );
        // Not E0900: this is not a feature that arrives later, it is a name
        // that will never be legal.
        assert!(!error_codes(&r).contains(&"E0900"), "{:?}", r.diagnostics);
    }

    #[test]
    fn a_type_parameter_named_self_is_rejected_at_every_generic_declaration() {
        // `record W<Self>` is exactly as confusing as `impl<Self>`, so the
        // check belongs at every place a generic parameter can be declared —
        // all six `generics` fields in the AST, including the two that only
        // ever hold a method's own parameters. One shared pass, so a new
        // generic-carrying construct cannot quietly opt out.
        //
        // Each program is checked in isolation, and each must produce at least
        // one E0076 that names the parameter.
        let cases: &[(&str, &str)] = &[
            ("free fn", "fn f<Self>(x: Self) -> Int { 1 }\nfn main() { }"),
            ("record", "record W<Self> { v: Self }\nfn main() { }"),
            // `ast::Item::Type` covers both a sum type and a plain alias, but
            // an alias cannot be tested here: the resolver rejects
            // `type A<T> = …` outright with "type aliases are not supported yet
            // in the Phase 1 compiler", so it never reaches this pass.
            ("sum type", "type S<Self> = | A(Self)\nfn main() { }"),
            (
                "trait",
                "trait Q<Self> { fn q(self) -> Int }\nfn main() { }",
            ),
            (
                "trait method",
                "trait Q { fn q<Self>(self, x: Self) -> Int }\nfn main() { }",
            ),
            (
                "impl",
                "record W<T> { v: T }\nimpl<Self> W<Self> { fn m(self) -> Int { 1 } }\n\
                 fn main() { }",
            ),
            (
                "impl method",
                "record W { v: Int }\nimpl W { fn m<Self>(self, x: Self) -> Int { 1 } }\n\
                 fn main() { }",
            ),
        ];
        for (label, src) in cases {
            let r = check_src(src);
            let d = r
                .diagnostics
                .iter()
                .find(|d| d.code == "E0076")
                .unwrap_or_else(|| panic!("{label}: expected E0076, got {:?}", r.diagnostics));
            assert!(d.message.contains("Self"), "{label}: {}", d.message);
        }
    }

    #[test]
    fn the_implicit_self_of_a_trait_body_is_not_a_user_written_parameter() {
        // The rejection must not catch the `Self` that `self_generic_scope`
        // inserts for a trait's own implicit type — that one is not a declared
        // parameter at all, and it is the whole mechanism `Self::Item` inside a
        // trait relies on. A control case, because a check that rejected every
        // occurrence of the *name* rather than every *declaration* of it would
        // still pass the test above.
        let r = check_src(
            "trait It { type Item\n \
             fn get(self) -> Self::Item\n \
             fn dup(self) -> Self::Item { self.get() } }\n\
             record W { v: Int }\n\
             impl It for W { type Item = Int\n fn get(self) -> Self::Item { panic(\"x\") } }\n\
             fn main() { }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
        assert!(!error_codes(&r).contains(&"E0076"), "{:?}", r.diagnostics);
    }

    #[test]
    fn a_parameter_named_self_is_rejected_but_still_scoped() {
        // The rejected name still enters the generic scope, so the *rest* of
        // the declaration resolves normally and the user gets one error rather
        // than an E0076 followed by a cascade of "cannot find type `Self`".
        let r = check_src("record W<Self> { v: Self }\nfn main() { }");
        assert_eq!(error_codes(&r), vec!["E0076"], "{:?}", r.diagnostics);
    }

    #[test]
    fn self_item_in_an_inherent_impl_reports_that_it_has_no_trait() {
        // Was `self_item_in_an_impl_with_no_explicit_self_parameter_still_
        // reports_e0900`, which pinned that this shape fell through to the
        // module-qualified-path branch. `Self` in an impl now means the impl's
        // own self type, so it no longer falls through — but an *inherent*
        // impl implements no trait, so nothing declares an associated type for
        // `Self::Item` to name. That is an E0001 "no such name", not an E0900
        // "not supported yet": there is no feature missing here.
        //
        // Deliberately not `resolve_projection`'s shared empty-candidate
        // wording ("on any bound of `Self`"): an inherent impl's `Self` has no
        // bounds at all, so that message would describe a lookup that never
        // happened.
        let r = check_src(
            "trait It { type Item\n fn get(self) -> Int }\n\
             record K { v: Int }\n\
             impl K { fn peek(self) -> Self::Item { panic(\"x\") } }\n\
             fn main() { }",
        );
        let d = r
            .diagnostics
            .iter()
            .find(|d| d.code == "E0001")
            .expect("E0001 for Self::Item in an inherent impl");
        assert!(d.message.contains("Item"), "names it: {}", d.message);
        assert!(d.message.contains("inherent"), "says why: {}", d.message);
        assert!(
            !d.message.contains("bound"),
            "an inherent impl's `Self` has no bounds: {}",
            d.message
        );
        assert!(
            !error_codes(&r).contains(&"E0900"),
            "no longer a module-qualified path: {:?}",
            r.diagnostics
        );
    }

    #[test]
    fn self_item_in_a_trait_impl_projects_onto_the_impls_own_self_type() {
        // The case Task 6's normalization rests on: `Self::Item` written
        // inside `impl<T> It for W<T>` must resolve, and it must project onto
        // the impl's SELF TYPE (`W<Param(0)>`) — not onto `Param(0)`, which is
        // the impl's first type parameter and a different type entirely. Every
        // projection before this task was on a flat `Param(idx)`, so a
        // by-index implementation would look right until this compound case.
        let r = check_src(
            "trait It { type Item\n fn get(self) -> Self::Item }\n\
             record W<T> { v: T }\n\
             impl<T> It for W<T> { type Item = T\n \
             fn get(self) -> Self::Item { panic(\"x\") } }\n\
             fn main() { }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
        let it = r
            .module
            .traits
            .iter()
            .find(|t| t.name == "It")
            .expect("trait It collected");
        let item = it.assoc_types[0].1;
        let w = r
            .module
            .records
            .iter()
            .find(|rec| rec.name == "W")
            .expect("record W collected")
            .def_id;
        // The trait's own declared return type still *is* the projection — this
        // is the side nothing normalizes, and it pins the `Param(0)`-is-`Self`
        // convention the impl side is checked against.
        assert_eq!(
            it.methods[0].ret,
            Ty::Assoc {
                on: Box::new(Ty::Param(0)),
                assoc: item
            },
            "the trait declares `Self::Item`"
        );
        // The impl method's compiled return type, not just "no diagnostics":
        // `Ty::Error` unifies with everything, so an empty diagnostic list
        // alone would also hold if the projection had collapsed to an error.
        // By the impl method's exact mangled name — `<self type>.<trait>.<method>`,
        // with the self type's own arguments folded in by `type_full_name`.
        // Matching on a suffix instead picks up `Option::get` from the implicit
        // prelude.
        let m = r
            .module
            .functions
            .iter()
            .find(|f| f.name == "W_T.It.get")
            .unwrap_or_else(|| {
                panic!(
                    "the impl's `get` was compiled; have {:?}",
                    r.module
                        .functions
                        .iter()
                        .map(|f| f.name.as_str())
                        .filter(|n| n.contains("get"))
                        .collect::<Vec<_>>()
                )
            });
        // **Deliberately flipped by Task 5**, which is what this test's own
        // header anticipated. It used to assert `ret_ty` was still
        // `Assoc { on: W<Param(0)>, assoc: Item }`; the function-return
        // normalization seam now resolves that through `type Item = T` to
        // `Param(0)`, i.e. `T`.
        //
        // The flip *strengthens* the original claim rather than weakening it. A
        // projection built onto `Param(0)` instead of onto the impl's self type
        // — the by-index implementation this test exists to catch, Task 4's
        // mutation F — has no `head()` and so cannot normalize at all: it would
        // still read `Assoc { .. }` here, and would additionally trip
        // conformance. Reaching `Param(0)` therefore proves both that the
        // projection was on `W<T>` and that it resolved through this impl's own
        // binding.
        assert_eq!(
            m.ret_ty,
            Ty::Param(0),
            "`W<T>::Item` normalizes through `type Item = T` to the impl's `T`"
        );
        // `w` is still the record this projects onto; asserted through the trait
        // impl's recorded self type, which normalization does not touch.
        let imp = r
            .module
            .impls
            .iter()
            .find(|i| i.trait_id.is_some())
            .expect("the trait impl was collected");
        assert_eq!(
            imp.self_ty,
            Ty::Record {
                def_id: w,
                args: vec![Ty::Param(0)]
            }
        );
    }

    #[test]
    fn self_naming_an_undeclared_associated_type_in_a_trait_impl_is_an_error() {
        // The impl's trait is the only candidate: `It` declares `Item`, not
        // `Nope`, so this must not silently resolve to something.
        let r = check_src(
            "trait It { type Item\n fn get(self) -> Int }\n\
             record W { v: Int }\n\
             impl It for W { type Item = Int\n \
             fn get(self) -> Int { 1 }\n \
             fn other(self) -> Self::Nope { panic(\"x\") } }\n\
             fn main() { }",
        );
        let d = r
            .diagnostics
            .iter()
            .find(|d| d.code == "E0001" && d.message.contains("Nope"))
            .expect("E0001 for Self::Nope in a trait impl");
        assert!(d.message.contains("associated type"), "{}", d.message);
    }

    #[test]
    fn self_item_outside_any_impl_or_trait_still_reports_e0900() {
        // The two-segment path branch now has a third meaning, and the
        // original one must survive it: outside a trait body and outside an
        // impl there is no `Self` at all, so `Self::Item` is just a
        // module-qualified path.
        let r = check_src(
            "trait It { type Item\n fn get(self) -> Int }\n\
             fn f() -> Self::Item { panic(\"x\") }\n\
             fn main() { }",
        );
        let d = r
            .diagnostics
            .iter()
            .find(|d| d.code == "E0900")
            .expect("E0900 for Self::Item with no enclosing impl or trait");
        assert!(
            d.message.contains("module-qualified type paths"),
            "{}",
            d.message
        );
    }

    #[test]
    fn self_item_resolves_in_an_impl_method_body_annotation_too() {
        // `impl_self` has to be in scope for body checking, not only signature
        // collection: a `let` annotation goes through the same `convert_ty`,
        // from a different pass (`check_method`, not `collect_impls`).
        //
        // **Deliberately flipped by Task 5**, exactly as this test's own header
        // said it would be. The initializer used to be `panic(...)` (type
        // `Never`, which unifies with anything), because nothing normalized a
        // projection and `let x: Self::Item = 1` was a genuine mismatch; it is
        // now a plain `1`, and `x` is typed `Int`. That closes concern 1 of the
        // Task 4 report.
        //
        // `1` is the stronger initializer, not merely the newly-legal one:
        // `Never` unified with an unnormalized projection just as happily, so the
        // old program could not tell resolving from failing to resolve. `Int`
        // against `W::Item` can only pass if the projection was resolved through
        // `type Item = Int` — and the mismatching spelling is pinned separately
        // by `a_projection_in_a_let_annotation_normalizes`.
        let r = check_src(
            "trait It { type Item\n fn get(self) -> Self::Item }\n\
             record W { v: Int }\n\
             impl It for W { type Item = Int\n \
             fn get(self) -> Self::Item { let x: Self::Item = 1\n x } }\n\
             fn main() { }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
        let m = r
            .module
            .functions
            .iter()
            .find(|f| f.name == "W.It.get")
            .expect("the impl's `get` was compiled");
        // The local's own recorded type, not just an empty diagnostic list:
        // `Ty::Error` unifies with everything, so a failed annotation would
        // also have produced no diagnostics here.
        let x = m
            .locals
            .iter()
            .find(|l| l.name == "x")
            .expect("local `x` compiled");
        assert_eq!(
            x.ty,
            Ty::Int,
            "the `let` annotation resolved to the projection and normalized to Int"
        );
    }

    #[test]
    fn a_required_methods_own_signature_resolves_its_own_traits_self_item() {
        // `Iterator::next`'s eventual shape (`fn next(mut self) ->
        // Option<Self::Item>`) is exactly this: a REQUIRED method whose own
        // return type projects, through `Self`, onto an associated type its
        // own enclosing trait declares. At the point this method's signature
        // is converted, `collect_traits` has not yet pushed *this trait's
        // own* `hir::TraitDef` (it is still being built) — this test is what
        // proves `find_assoc_type` must search `self.defs`, not the
        // incrementally-built `self.traits`.
        let r = check_src("trait It { type Item\n fn get(self) -> Self::Item }\nfn main() { }");
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    }

    #[test]
    fn a_default_methods_own_signature_resolves_its_own_traits_self_item() {
        // Mirrors the required-method case above, but for a method WITH a
        // default body: `collect_traits` builds this signature in a second,
        // separate pass (the default-method-body pass), with its own
        // `Self`-bound construction that is independently susceptible to the
        // same ordering hazard.
        let r = check_src(
            "trait It { type Item\n fn get(self) -> Self::Item { panic(\"unreachable\") } }\n\
             fn main() { }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    }

    #[test]
    fn an_impl_binds_its_associated_type() {
        let r = check_src(
            "trait It { type Item\n fn get(self) -> Int }\n\
             record W<T> { v: T }\n\
             impl<T> It for W<T> { type Item = T\n fn get(self) -> Int { 1 } }\n\
             fn main() { }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
        let i = r
            .module
            .impls
            .iter()
            .find(|i| i.trait_id.is_some())
            .expect("the trait impl was collected");
        // Bound to the impl's OWN parameter, which is what makes `subst` the
        // thing that carries it — a binding to a primitive would not.
        assert_eq!(i.assoc_bindings.len(), 1);
        assert_eq!(i.assoc_bindings[0].1, Ty::Param(0));
        // And the key is the trait's own associated-type `DefId`, not some
        // freshly minted or positional id: a binding keyed by anything else
        // could not be looked up by the normalization seams in Tasks 5-7.
        let t = r
            .module
            .traits
            .iter()
            .find(|t| t.name == "It")
            .expect("trait It collected");
        assert_eq!(i.assoc_bindings[0].0, t.assoc_types[0].1);
    }

    #[test]
    fn an_impl_missing_an_associated_type_reports_e0070() {
        let r = check_src(
            "trait It { type Item\n fn get(self) -> Int }\n\
             record W { v: Int }\n\
             impl It for W { fn get(self) -> Int { 1 } }\n\
             fn main() { }",
        );
        let d = r
            .diagnostics
            .iter()
            .find(|d| d.code == "E0070")
            .expect("E0070 for a missing associated type");
        assert!(
            d.message.contains("Item"),
            "names the missing type: {}",
            d.message
        );
        // The impl provides every method the trait requires, so the message
        // must not read as a missing *method* — the shared E0070 site says
        // "method(s)" for that case and has to say something else for this one.
        assert!(
            !d.message.contains("method"),
            "a missing associated type is not a missing method: {}",
            d.message
        );
    }

    #[test]
    fn an_impl_binding_an_undeclared_associated_type_reports_e0071() {
        let r = check_src(
            "trait It { type Item\n fn get(self) -> Int }\n\
             record W { v: Int }\n\
             impl It for W { type Item = Int\n type Extra = Bool\n fn get(self) -> Int { 1 } }\n\
             fn main() { }",
        );
        let d = r
            .diagnostics
            .iter()
            .find(|d| d.code == "E0071")
            .expect("E0071 for an undeclared associated type");
        assert!(d.message.contains("Extra"), "names it: {}", d.message);
        // `Item` IS declared, so only the one offender may be reported.
        assert!(
            !d.message.contains("Item"),
            "must not implicate the legitimate binding: {}",
            d.message
        );
        assert_eq!(
            r.diagnostics.iter().filter(|d| d.code == "E0071").count(),
            1,
            "exactly one E0071: {:?}",
            r.diagnostics
        );
    }

    #[test]
    fn binding_the_same_associated_type_twice_is_rejected() {
        // Both bindings resolve to the same `DefId`, so keeping both would
        // leave which one normalization reads up to list order.
        let r = check_src(
            "trait It { type Item\n fn get(self) -> Int }\n\
             record W { v: Int }\n\
             impl It for W { type Item = Int\n type Item = Bool\n fn get(self) -> Int { 1 } }\n\
             fn main() { }",
        );
        let d = r
            .diagnostics
            .iter()
            .find(|d| d.code == "E0403")
            .expect("E0403 for a repeated associated-type binding");
        assert!(d.message.contains("Item"), "{}", d.message);
        // The first binding survives, so the set is complete and no E0070 is
        // due — a duplicate is one error, not two.
        assert!(!error_codes(&r).contains(&"E0070"), "{:?}", r.diagnostics);
        let i = r
            .module
            .impls
            .iter()
            .find(|i| i.trait_id.is_some())
            .expect("the trait impl was collected");
        assert_eq!(i.assoc_bindings.len(), 1, "{:?}", i.assoc_bindings);
        assert_eq!(i.assoc_bindings[0].1, Ty::Int, "the FIRST binding is kept");
    }

    #[test]
    fn an_inherent_impl_cannot_bind_an_associated_type() {
        // `check_impl_conformance` never runs for an inherent impl, so without
        // its own check here the binding would be silently dropped.
        let r = check_src(
            "record W { v: Int }\n\
             impl W { type Item = Int\n fn get(self) -> Int { 1 } }\n\
             fn main() { }",
        );
        let d = r
            .diagnostics
            .iter()
            .find(|d| d.code == "E0071")
            .expect("E0071 for a binding on an inherent impl");
        assert!(d.message.contains("Item"), "{}", d.message);
        assert!(
            d.message.contains("inherent"),
            "says why, not just that: {}",
            d.message
        );
    }

    #[test]
    fn an_impl_level_const_is_rejected_rather_than_silently_dropped() {
        // `ast::ImplBlock::consts` had no reader anywhere in the workspace, so
        // an impl-level `const` parsed, ran, and vanished: this program printed
        // its method's result with no diagnostic, and `K::LIMIT` then reported
        // `no variant 'LIMIT' on type 'K'` — which is not what the user wrote.
        // Accepting and discarding a declaration is worse than refusing it, so
        // it is `E0900` until associated constants are actually implemented.
        let r = check_src(
            "record K { v: Int }\n\
             impl K { const LIMIT: Int = 99\n fn get(self) -> Int { self.v } }\n\
             fn main() { }",
        );
        let d = r
            .diagnostics
            .iter()
            .find(|d| d.code == "E0900")
            .expect("E0900 for an impl-level const");
        assert!(
            d.message.contains("LIMIT"),
            "names the constant: {}",
            d.message
        );
        assert!(
            d.message.contains("associated constant"),
            "names the construct, so the message is searchable: {}",
            d.message
        );
    }

    #[test]
    fn a_top_level_const_still_works_beside_an_impl() {
        // The rejection above must not reach ordinary `const`s. Without this,
        // narrowing it to impl bodies could not be distinguished from banning
        // constants outright — every existing const test uses a file with no
        // `impl` in it at all.
        let r = check_src(
            "const LIMIT: Int = 99\n\
             record K { v: Int }\n\
             impl K { fn get(self) -> Int { LIMIT } }\n\
             fn main() { }",
        );
        assert_eq!(error_codes(&r), Vec::<&str>::new(), "{:?}", r.diagnostics);
    }

    #[test]
    fn conformance_compares_the_associated_type_sets_not_their_sizes() {
        // Two declared, two bound, but one of each is wrong: `A` is missing
        // and `C` is undeclared. A count comparison sees 2 == 2 and reports
        // nothing; only a set comparison catches both. This is the case a
        // single-associated-type test cannot distinguish.
        let r = check_src(
            "trait Pair { type A\n type B\n fn get(self) -> Int }\n\
             record W { v: Int }\n\
             impl Pair for W { type B = Int\n type C = Bool\n fn get(self) -> Int { 1 } }\n\
             fn main() { }",
        );
        let missing = r
            .diagnostics
            .iter()
            .find(|d| d.code == "E0070")
            .expect("E0070 for the missing `A`");
        assert!(missing.message.contains('A'), "{}", missing.message);
        assert!(
            !missing.message.contains('B'),
            "`B` is bound, so it is not missing: {}",
            missing.message
        );
        let extra = r
            .diagnostics
            .iter()
            .find(|d| d.code == "E0071")
            .expect("E0071 for the undeclared `C`");
        assert!(extra.message.contains('C'), "{}", extra.message);
    }

    #[test]
    fn a_self_referential_associated_type_binding_is_rejected() {
        // `type Item = Self::Item` describes a type in terms of itself. It has
        // to be rejected *here*, at the declaration, because `normalize` must
        // re-normalize its own result for the legitimate `A = Self::B` /
        // `B = Int` chain to resolve — and that is exactly the walk that would
        // never terminate on this input.
        let r = check_src(
            "trait It { type Item\n fn get(self) -> Self::Item }\n\
             record W { v: Int }\n\
             impl It for W { type Item = Self::Item\n\
             fn get(self) -> Self::Item { 1 } }\n\
             fn main() { }",
        );
        let d = r
            .diagnostics
            .iter()
            .find(|d| d.code == "E0077")
            .expect("E0077 for a self-referential associated-type binding");
        assert!(
            d.message.contains("Item"),
            "names the offending type: {}",
            d.message
        );
        // A cycle is one mistake, so the binding is not also reported missing.
        assert!(!error_codes(&r).contains(&"E0070"), "{:?}", r.diagnostics);
    }

    #[test]
    fn a_mutually_recursive_pair_of_associated_type_bindings_is_rejected() {
        // Neither binding is self-referential on its own, so a check that only
        // looked at each binding in isolation would accept this. The cycle
        // closes only by following `A -> B -> A`.
        let r = check_src(
            "trait Two { type A\n type B\n fn get(self) -> Int }\n\
             record W { v: Int }\n\
             impl Two for W { type A = Self::B\n type B = Self::A\n\
             fn get(self) -> Int { 1 } }\n\
             fn main() { }",
        );
        let msgs: Vec<&str> = r
            .diagnostics
            .iter()
            .filter(|d| d.code == "E0077")
            .map(|d| d.message.as_str())
            .collect();
        assert_eq!(msgs.len(), 2, "one per offending type: {:?}", r.diagnostics);
        assert!(
            msgs.iter().any(|m| m.contains('A')) && msgs.iter().any(|m| m.contains('B')),
            "both members of the cycle are named: {msgs:?}"
        );
    }

    #[test]
    fn a_cyclic_associated_type_binding_nested_in_a_compound_type_is_rejected() {
        // `Item = [Item]` is an infinitely large type exactly as `Item = Item`
        // is. A cycle walk that only inspected the top level of each bound type
        // accepts this and hands `normalize` a projection that grows a layer per
        // step, so the compiler diverges on a five-line program.
        let r = check_src(
            "trait It { type Item\n fn get(self) -> Int }\n\
             record W { v: Int }\n\
             impl It for W { type Item = [Self::Item]\n fn get(self) -> Int { 1 } }\n\
             fn main() { }",
        );
        assert_eq!(error_codes(&r), ["E0077"], "{:?}", r.diagnostics);
    }

    #[test]
    fn a_cyclic_associated_type_binding_nested_in_a_future_is_rejected() {
        // The same shape as the test above, `Item = [Self::Item]`, but through
        // `collect_self_projections`'s `Ty::Future` arm instead of its
        // `Ty::Array` arm. Kills a mutant that drops the payload there (e.g.
        // matching `Ty::Future(_)` and doing nothing, the way the pre-existing
        // primitive arms correctly do for types with no self-projections to
        // find): with no edge recorded back to `Item`, `reaches_self` would
        // say there is no cycle, and the compiler would fall through to
        // `normalize_ty`'s re-normalization instead of reporting it here --
        // which is precisely the wrong-diagnostic failure mode this function's
        // own doc comment warns about ("a compiler-limit message is the wrong
        // thing to show a user who wrote a two-line cycle").
        let r = check_src(
            "trait It { type Item\n fn get(self) -> Int }\n\
             record W { v: Int }\n\
             impl It for W { type Item = Future<Self::Item>\n fn get(self) -> Int { 1 } }\n\
             fn main() { }",
        );
        assert_eq!(error_codes(&r), ["E0077"], "{:?}", r.diagnostics);
    }

    /// Task 11 Step 4. A signature comparison whose either side is already
    /// poisoned reports nothing, so `{error}` never reaches a user.
    ///
    /// `Ty` derives `PartialEq` with no `Ty::Error` absorption, so at this seam a
    /// poisoned side *forces* a mismatch — the opposite of `unify`, where
    /// `Ty::Error` unifies with anything and the poison quietly stops. Every one
    /// of the three programs below already has a diagnostic explaining the real
    /// problem, and each used to get a second `E0072` naming `{error}`, which is
    /// meaningless to a user and points at a method that is not the mistake.
    ///
    /// Three rows, not one, because the poison arrives by two different routes
    /// and at two different depths:
    ///  * a cyclic binding poisoned by `E0077`, on the **trait** side;
    ///  * an unresolvable binding poisoned by `E0001`, also on the trait side;
    ///  * the same, but the projection is **wrapped** in `Option`, so the
    ///    normalized type is `Option<{error}>` and not `Ty::Error` itself. This
    ///    row is why the guard is `has_error()` and not `== Ty::Error`: a
    ///    top-level check leaves this one leaking `Option<{error}>`. Measured, on
    ///    `25db453`.
    ///
    /// A fourth row puts the projection in a **parameter** rather than the return
    /// type, because the comparison has two guarded sites and they are separate
    /// code: measured, removing only the parameter-side guard left all 602 tests
    /// green when the first three rows were the whole test.
    ///
    /// The last row is the control that this is suppression of a *second* error
    /// and not of the check: two concrete types that genuinely disagree, with no
    /// poison anywhere, still report `E0072`. Without it, deleting the comparison
    /// outright would pass.
    #[test]
    fn signatures_are_not_compared_when_either_side_is_already_poisoned() {
        // (trait's `get` signature tail, impl body, the complete code set)
        let rows: [(&str, &str, &[&str]); 5] = [
            (
                ") -> Self::Item",
                "type Item = Self::Item\n fn get(self) -> Int { 1 }",
                &["E0077"],
            ),
            (
                ") -> Self::Item",
                "type Item = Nope\n fn get(self) -> Bool { true }",
                &["E0001"],
            ),
            (
                ") -> Option<Self::Item>",
                "type Item = Nope\n fn get(self) -> Option<Bool> { None }",
                &["E0001"],
            ),
            (
                ", x: Self::Item) -> Int",
                "type Item = Nope\n fn get(self, x: Bool) -> Int { 1 }",
                &["E0001"],
            ),
            (
                ") -> Int",
                "type Item = Int\n fn get(self) -> Bool { true }",
                &["E0072"],
            ),
        ];
        for (trait_tail, impl_body, want) in rows {
            let r = check_src(&format!(
                "trait It {{ type Item\n fn get(self{trait_tail} }}\n\
                 record W {{ v: Int }}\n\
                 impl It for W {{ {impl_body} }}\n\
                 fn main() {{ }}"
            ));
            assert_eq!(
                error_codes(&r),
                want,
                "impl `{impl_body}` against `fn get(self{trait_tail}`: {:?}",
                r.diagnostics
            );
            // Belt and braces on the whole row set: no message may render the
            // `Ty::Error` sentinel, whatever code carries it.
            for d in &r.diagnostics {
                assert!(
                    !d.message.contains("{error}"),
                    "no user-facing message may render `Ty::Error`: {}",
                    d.message
                );
            }
        }
    }

    /// Task 11 Step 1. A projection in an impl's **self type** is rejected.
    ///
    /// Two independent defects compound in that position, both measured on
    /// `25db453` where the whole file below reported `ok`. The impl can never be
    /// selected, because `Ty::match_pattern` recovers an impl's type arguments by
    /// matching its self type against a ground type and cannot invert `T::Item`
    /// to find `T`. And it is invisible to coherence, because
    /// `hir::self_types_overlap`'s helpers do not understand `Assoc` — so it does
    /// not conflict with an impl that *does* apply to the same type. Dead code
    /// that also defeats overlap checking is worse than either alone.
    ///
    /// The `E0074` control is what makes the coherence half of that claim
    /// falsifiable: the same file with `impl<T> Tr for W<T>` in place of the
    /// projection *does* conflict. Without it, "no `E0074`" would be consistent
    /// with these two impls simply not overlapping.
    #[test]
    fn a_projection_in_an_impls_self_type_is_rejected() {
        let prelude = "trait It { type Item\n fn g(self) -> Int }\n\
                       trait Tr { fn h(self) -> Int }\n\
                       record W<T> { v: T }\n";
        let overlapping = "impl<T: It> Tr for W<T::Item> { fn h(self) -> Int { 1 } }\n\
                           impl Tr for W<Int> { fn h(self) -> Int { 2 } }\n";
        let r = check_src(&format!("{prelude}{overlapping}fn main() {{ }}"));
        assert_eq!(error_codes(&r), ["E0900"], "{:?}", r.diagnostics);
        assert!(
            r.diagnostics[0].message.contains("impl's self type"),
            "the message must name the position: {}",
            r.diagnostics[0].message
        );
        // Alone, too: the impl is unselectable whether or not anything overlaps
        // it, so overlap is not the reason for the rejection and a test that only
        // used the pair would leave the single impl accepted.
        let alone = check_src(&format!(
            "{prelude}impl<T: It> Tr for W<T::Item> {{ fn h(self) -> Int {{ 1 }} }}\n\
             fn main() {{ }}"
        ));
        assert_eq!(error_codes(&alone), ["E0900"], "{:?}", alone.diagnostics);
        // A *bare* projection as the self type, which used to report
        // `E0010: impl blocks are only supported on named types` — misleading,
        // because `T::Item` may well resolve to a named type. Checked before the
        // head check for exactly that reason.
        let bare = check_src(&format!(
            "{prelude}impl<T: It> Tr for T::Item {{ fn h(self) -> Int {{ 1 }} }}\n\
             fn main() {{ }}"
        ));
        assert_eq!(error_codes(&bare), ["E0900"], "{:?}", bare.diagnostics);
        // The control. With a plain `W<T>` the same pair *is* an overlap, so the
        // projection was genuinely hiding one rather than the two impls being
        // disjoint.
        let control = check_src(&format!(
            "{prelude}impl<T> Tr for W<T> {{ fn h(self) -> Int {{ 1 }} }}\n\
             impl Tr for W<Int> {{ fn h(self) -> Int {{ 2 }} }}\n\
             fn main() {{ }}"
        ));
        assert_eq!(
            error_codes(&control),
            ["E0074"],
            "{:?}",
            control.diagnostics
        );
    }

    /// The other half of Step 1, and the assertion that keeps the rejection from
    /// quietly becoming "projections are banned from impls".
    ///
    /// Every position a projection *is* legal in, in one program: an impl's
    /// binding right-hand side both on the impl's own parameter (`type Item =
    /// T::Item`) and on `Self` (`type Other = Self::Item`); an impl method's
    /// return type, bare and wrapped in `Option`; a `let` annotation inside an
    /// impl method body; a trait method's own declaration; and a free generic
    /// function's return type. Only the self type is refused.
    #[test]
    fn a_projection_is_still_accepted_in_every_other_position_of_an_impl() {
        let r = check_src(
            "trait It { type Item\n type Other\n\
             fn get(self) -> Self::Item\n\
             fn wrapped(self) -> Option<Self::Item> }\n\
             record W<T> { v: T }\n\
             impl<T: It> It for W<T> { type Item = T::Item\n\
             type Other = Self::Item\n\
             fn get(self) -> Self::Item { self.v.get() }\n\
             fn wrapped(self) -> Option<Self::Item> { let x: Self::Item = self.get()\n\
             Some(x) } }\n\
             record C { n: Int }\n\
             impl It for C { type Item = Int\n type Other = Int\n\
             fn get(self) -> Int { self.n }\n\
             fn wrapped(self) -> Option<Int> { Some(self.n) } }\n\
             fn first<I: It>(x: I) -> I::Item { x.get() }\n\
             fn main() { let c = C { n: 4 }\n println(\"${first(c)}\")\n\
             let w = W { v: C { n: 7 } }\n println(\"${first(w)}\") }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    }

    /// Task 11 Step 2. A trait may not declare the same name twice — as an
    /// associated type or as a method.
    ///
    /// Both were accepted on `25db453`, and both are the same defect: the
    /// consumers take the **first** match by name, so the second declaration is
    /// silently dead. For an associated type, `find_assoc_type` scans and takes
    /// the first, so nothing binds the second and conformance does not notice
    /// because it matches bindings by *name*. For a method,
    /// `trait_method_index` and `check_impl_method_signatures` likewise take the
    /// first, so a trait can declare two contradictory signatures and the impl is
    /// checked against whichever happens to be written first.
    ///
    /// The plan asked for the associated-type half and said to *check* whether
    /// the method half had the same hole rather than assume it. It does; both are
    /// fixed here, with the same `E0403` a duplicate generic parameter and a
    /// duplicate associated-type binding in an impl already report.
    ///
    /// `fn g(self)` beside `fn g()` is rejected too, and that is the case worth
    /// naming: the two lookups partition the method list by receiver, so allowing
    /// it would make `g` resolve to a different declaration depending on the call
    /// syntax.
    ///
    /// One diagnostic each, asserted as the complete list: the duplicate is
    /// skipped before its signature is converted, so it does not also report its
    /// own errors for a mistake that is the duplication.
    #[test]
    fn a_trait_may_not_declare_one_name_twice() {
        let assoc = check_src(
            "trait It { type Item\n type Item\n fn g(self) -> Int }\n\
             record W { v: Int }\n\
             impl It for W { type Item = Int\n fn g(self) -> Int { 1 } }\n\
             fn main() { }",
        );
        assert_eq!(error_codes(&assoc), ["E0403"], "{:?}", assoc.diagnostics);
        assert!(
            assoc.diagnostics[0].message.contains("`Item`")
                && assoc.diagnostics[0].message.contains("associated type"),
            "the message must name the type: {}",
            assoc.diagnostics[0].message
        );
        let method = check_src(
            "trait It { fn g(self) -> Int\n fn g(self) -> Bool }\n\
             record W { v: Int }\n\
             impl It for W { fn g(self) -> Int { 1 } }\n\
             fn main() { }",
        );
        assert_eq!(error_codes(&method), ["E0403"], "{:?}", method.diagnostics);
        assert!(
            method.diagnostics[0].message.contains("`g`")
                && method.diagnostics[0].message.contains("method"),
            "the message must name the method: {}",
            method.diagnostics[0].message
        );
        // A receiver-ful method beside a receiver-less associated function of the
        // same name: still one name, still rejected.
        let mixed = check_src(
            "trait It { fn g(self) -> Int\n fn g() -> Int }\n\
             record W { v: Int }\n\
             impl It for W { fn g(self) -> Int { 1 } }\n\
             fn main() { }",
        );
        assert_eq!(error_codes(&mixed), ["E0403"], "{:?}", mixed.diagnostics);
        // Control: distinct names are untouched.
        let ok = check_src(
            "trait It { type A\n type B\n fn g(self) -> Self::A\n fn h(self) -> Self::B }\n\
             record W { v: Int }\n\
             impl It for W { type A = Int\n type B = Bool\n\
             fn g(self) -> Int { 1 }\n fn h(self) -> Bool { true } }\n\
             fn main() { }",
        );
        assert!(ok.diagnostics.is_empty(), "{:?}", ok.diagnostics);
        // And the survivors keep the right `DefId`s. `assoc_type_ids` is drained
        // *positionally* — one id per `TraitItem::AssocType`, in source order —
        // so a rejection that `continue`d without calling `next()` would hand `B`
        // the id minted for the discarded second `A`, and `TraitDef.assoc_types`
        // would carry a `(name, DefId)` pair whose two halves disagree.
        //
        // The observable consequence is a cross-table one, which is why this is
        // asserted here rather than left to the diagnostic list: `ImplInfo
        // .assoc_bindings` is keyed by `find_assoc_type`, which searches
        // `self.defs` by name and is therefore *immune* to the misalignment,
        // while `TraitDef.assoc_types` is not. So the two tables key the same
        // associated type under two different ids and nothing else notices —
        // `check_impl_conformance` compares by name, so it reports nothing, and
        // the only reader of the id half is `mono.rs`'s `type_name`, which would
        // render the projection as `W::?` in a diagnostic that needs a name.
        // Measured: with the `next()` moved inside the `if let`, the whole
        // 599-test suite stays green and only this assertion fails.
        let realigned = check_src(
            "trait It { type A\n type A\n type B\n\
             fn g(self) -> Self::B }\n\
             record W { v: Int }\n\
             impl It for W { type A = Int\n type B = Bool\n\
             fn g(self) -> Bool { true } }\n\
             fn main() { }",
        );
        assert_eq!(
            error_codes(&realigned),
            ["E0403"],
            "the duplicate is the only error — `B` must still resolve to `Bool`: {:?}",
            realigned.diagnostics
        );
        let t = realigned
            .module
            .traits
            .iter()
            .find(|t| t.name == "It")
            .expect("trait It collected");
        assert_eq!(
            t.assoc_types
                .iter()
                .map(|(n, _)| n.as_str())
                .collect::<Vec<_>>(),
            ["A", "B"],
            "the duplicate is dropped, the two survivors keep their order"
        );
        let i = realigned
            .module
            .impls
            .iter()
            .find(|i| i.trait_id.is_some())
            .expect("the trait impl was collected");
        for (name, id) in &t.assoc_types {
            assert!(
                i.assoc_bindings.iter().any(|(d, _)| d == id),
                "`{name}` in the trait table is keyed by an id no binding uses, so the \
                 positional drain fell out of step: {:?} vs {:?}",
                t.assoc_types,
                i.assoc_bindings
            );
        }
    }

    /// Task 11 Step 5. An impl may echo an associated type its trait inherits
    /// from a **supertrait**, exactly as the trait's own method signature may.
    ///
    /// The two sides used to disagree. `collect_traits` seeds `sig_bounds[0]`
    /// with the trait and then calls `expand_bounds`, so inside
    /// `trait Ext: Base { fn peek(self) -> Self::Elem }` the projection resolves
    /// against `Base`. `convert_ty`'s impl branch passed `vec![tid]`
    /// *unexpanded*, and `find_assoc_type` matches `trait_def == trait_id`
    /// exactly, so the echoed spelling in `impl Ext for W` was
    /// `E0001: no associated type `Elem` on any bound of `Self`` — measured on
    /// `25db453` — while the identical signature one declaration above was fine.
    /// Design doc §5.1 pins "either spelling is accepted", and this was the one
    /// place the echo was not.
    ///
    /// Two negative controls, because "resolve it against more traits" has two
    /// ways to over-reach. A name no bound declares must still be `E0001`, and a
    /// name **both** `Ext` and `Base` declare must still be `E0015` — for that
    /// second one the trait side already reported `E0015` before this change (so
    /// the program was already rejected, measured), and the impl now agrees
    /// instead of quietly picking `Ext`'s. Two reports for one root cause is the
    /// pre-existing lack of diagnostic dedup, asserted by count rather than
    /// hidden behind a `contains`.
    #[test]
    fn an_impl_may_echo_an_associated_type_inherited_from_a_supertrait() {
        let prelude = "trait Base { type Elem\n fn base(self) -> Int }\n";
        let w = "record W { v: Int }\n\
                 impl Base for W { type Elem = Int\n fn base(self) -> Int { 1 } }\n";
        // The echo: `Ext` declares no associated type of its own, so `Self::Elem`
        // can only come from `Base`.
        let r = check_src(&format!(
            "{prelude}trait Ext: Base {{ fn peek(self) -> Self::Elem }}\n\
             {w}impl Ext for W {{ fn peek(self) -> Self::Elem {{ 5 }} }}\n\
             fn main() {{ }}"
        ));
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
        // And it really resolved to `Int` rather than to `Ty::Error`, which would
        // unify with anything and make the assertion above vacuous.
        let wrong = check_src(&format!(
            "{prelude}trait Ext: Base {{ fn peek(self) -> Self::Elem }}\n\
             {w}impl Ext for W {{ fn peek(self) -> Bool {{ true }} }}\n\
             fn main() {{ }}"
        ));
        assert_eq!(error_codes(&wrong), ["E0072"], "{:?}", wrong.diagnostics);
        // Control 1: a name no bound of `Self` declares is still unresolved, on
        // both sides.
        let absent = check_src(&format!(
            "{prelude}trait Ext: Base {{ fn peek(self) -> Self::Nope }}\n\
             {w}impl Ext for W {{ fn peek(self) -> Self::Nope {{ 5 }} }}\n\
             fn main() {{ }}"
        ));
        assert_eq!(
            error_codes(&absent),
            ["E0001", "E0001"],
            "{:?}",
            absent.diagnostics
        );
        // Control 2: declared by the trait *and* its supertrait is ambiguous, and
        // now says so on both sides. Pre-fix this reported one `E0015` (the trait
        // side), so the program was already rejected; the impl no longer resolves
        // it silently to `Ext`'s.
        let ambiguous = check_src(&format!(
            "{prelude}trait Ext: Base {{ type Elem\n fn peek(self) -> Self::Elem }}\n\
             {w}impl Ext for W {{ type Elem = Int\n fn peek(self) -> Self::Elem {{ 5 }} }}\n\
             fn main() {{ }}"
        ));
        assert_eq!(
            error_codes(&ambiguous),
            ["E0015", "E0015"],
            "{:?}",
            ambiguous.diagnostics
        );
    }

    #[test]
    fn a_binding_projecting_onto_an_impl_parameter_is_not_a_cycle() {
        // The control case. `type Item = T::Item` is a projection in a binding
        // and must be accepted: it names the *argument's* associated type, and
        // each normalization step strips a `W<…>` layer, so it bottoms out.
        // A cycle check that flags any `Assoc` in a bound type kills this.
        let r = check_src(
            "trait It { type Item\n fn get(self) -> Int }\n\
             record W<T> { v: T }\n\
             impl<T: It> It for W<T> { type Item = T::Item\n fn get(self) -> Int { 1 } }\n\
             fn main() { }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    }

    #[test]
    fn a_chain_of_associated_type_bindings_is_accepted() {
        // `A = Self::B` with `B = Int` is legal and is the reason `normalize`
        // has to re-normalize its own result. The cycle check must not reject a
        // reference that bottoms out.
        let r = check_src(
            "trait Two { type A\n type B\n fn get(self) -> Int }\n\
             record W { v: Int }\n\
             impl Two for W { type A = Self::B\n type B = Int\n\
             fn get(self) -> Int { 1 } }\n\
             fn main() { }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    }

    /// A hang is not assertable: this test's value is that it *completes*.
    /// Every cyclic shape reachable from source goes through here, and the
    /// diagnostic count is bounded so a regression to merely-quadratic
    /// reporting is caught too (the model is Task 4's termination tests).
    #[test]
    fn the_compiler_terminates_on_every_cyclic_binding_shape() {
        let shapes = [
            "impl It for W { type Item = Self::Item\n fn get(self) -> Self::Item { 1 } }",
            "impl It for W { type Item = [Self::Item]\n fn get(self) -> Int { 1 } }",
            "impl It for W { type Item = [[Self::Item]]\n fn get(self) -> Int { 1 } }",
        ];
        for shape in shapes {
            let src = format!(
                "trait It {{ type Item\n fn get(self) -> Self::Item }}\n\
                 record W {{ v: Int }}\n\
                 {shape}\n\
                 fn use_it(w: W) -> Int {{ 1 }}\n\
                 fn main() {{ println(\"${{use_it(W {{ v: 1 }})}}\") }}"
            );
            let r = check_src(&src);
            assert!(
                r.diagnostics.len() < 10,
                "bounded diagnostics for {shape}: {:?}",
                r.diagnostics
            );
            assert!(
                error_codes(&r).contains(&"E0077"),
                "the cycle is reported for {shape}: {:?}",
                r.diagnostics
            );
        }
    }

    /// The plan's Step 1 test, with one change recorded here rather than only in
    /// the report: it writes the impl's return type as `Self::Item`, not as `T`.
    ///
    /// The `T` spelling reports an *extra* `E0072` ("method `get_item` returns
    /// `T0` but trait `It` declares `W<T0>::Item`"), because
    /// `check_impl_conformance` still compares the two signatures raw — that is
    /// the second normalization seam, and it is Task 6's, where both spellings
    /// are pinned by `an_impl_may_echo_the_projection_or_write_the_concrete_type`.
    /// Written the `T` way, this test could not pass at the end of Task 5 for a
    /// reason that has nothing to do with `normalize`.
    #[test]
    fn a_projection_on_a_concrete_type_normalizes_at_a_use_site() {
        // `w.get_item()` returns `Self::Item`; with Self = W<Int> that is Int,
        // so assigning it to an Int must typecheck with no annotation.
        let r = check_src(
            "trait It { type Item\n fn get_item(self) -> Self::Item }\n\
             record W<T> { v: T }\n\
             impl<T> It for W<T> { type Item = T\n\
             fn get_item(self) -> Self::Item { self.v } }\n\
             fn main() { let w = W { v: 7 }\n let n: Int = w.get_item()\n\
             println(\"${n} ${w.get_item()}\") }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
        // Not vacuous on the *value* either: the call must be typed `Int`, so a
        // `normalize` that returned `Ty::Error` (which unifies with anything)
        // cannot hide behind the empty diagnostic list above.
        //
        // Read off the **unannotated** second call, deliberately. `Stmt::Let`
        // does `value.ty = annot_ty` *after* the unify, so the initializer of
        // `let n: Int = …` carries the annotation's type by construction — an
        // assertion there equals `Int` whatever `normalize` returned. Measured:
        // this exact test passed against a `normalize` that answered `Ty::Error`
        // for the second impl when it was written against the annotated call.
        let call_tys: Vec<&Ty> = exprs_in(&r.module, "main")
            .into_iter()
            .filter(|e| matches!(e.kind, hir::ExprKind::TraitCall { .. }))
            .map(|e| &e.ty)
            .collect();
        assert_eq!(call_tys.len(), 2, "two calls: {call_tys:?}");
        assert_eq!(
            *call_tys[1],
            Ty::Int,
            "the unannotated call is typed Int, not an error type"
        );
    }

    #[test]
    fn a_projection_normalizes_to_the_wrong_type_is_an_error() {
        // The negative direction: Self::Item is Int here, so binding it to a
        // Bool must fail. Without this, a `normalize` that returned Ty::Error
        // or Ty::Never for everything would pass the positive test.
        let r = check_src(
            "trait It { type Item\n fn get_item(self) -> Self::Item }\n\
             record W<T> { v: T }\n\
             impl<T> It for W<T> { type Item = T\n\
             fn get_item(self) -> Self::Item { self.v } }\n\
             fn main() { let w = W { v: 7 }\n let b: Bool = w.get_item()\n println(\"${b}\") }",
        );
        assert!(
            r.diagnostics.iter().any(|d| d.code == "E0010"),
            "expected a type mismatch: {:?}",
            r.diagnostics
        );
    }

    #[test]
    fn normalization_picks_the_impl_that_matches_the_receiver() {
        // Two impls of one trait, on different self types, binding `Item` to
        // different types. A single-impl test cannot tell "looks the impl up"
        // apart from "takes the only impl in the table".
        let prelude = "trait It { type Item\n fn get_item(self) -> Self::Item }\n\
             record W<T> { v: T }\n\
             record K { k: Bool }\n\
             impl<T> It for W<T> { type Item = T\n\
             fn get_item(self) -> Self::Item { self.v } }\n\
             impl It for K { type Item = Bool\n\
             fn get_item(self) -> Self::Item { self.k } }\n";
        // The calls are left **unannotated**, on purpose: `Stmt::Let` overwrites
        // an annotated initializer's type with the annotation, so `let n: Int =
        // w.get_item()` records `Int` on the call node no matter what `normalize`
        // returned. The `let`s below only pin the mismatch direction; the types
        // are read off the interpolated calls.
        let r = check_src(&format!(
            "{prelude}fn main() {{ let w = W {{ v: 7 }}\n let k = K {{ k: true }}\n\
             println(\"${{w.get_item()}} ${{k.get_item()}}\") }}"
        ));
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
        // The two calls' own types, in source order. An empty diagnostic list is
        // not enough: an implementation that picks the wrong impl and fails to
        // recover its arguments yields `Ty::Error`, which unifies with both `Int`
        // and `Bool` and so satisfies the assertion above for the wrong reason.
        // Measured — this is what let a "take any impl that binds `Item`"
        // mutation survive before these two lines existed.
        let call_tys: Vec<&Ty> = exprs_in(&r.module, "main")
            .into_iter()
            .filter(|e| matches!(e.kind, hir::ExprKind::TraitCall { .. }))
            .map(|e| &e.ty)
            .collect();
        assert_eq!(call_tys, [&Ty::Int, &Ty::Bool], "one type each, in order");
        // And the crossed pairing must fail, or `Item` is resolving to something
        // that fits both.
        let bad = check_src(&format!(
            "{prelude}fn main() {{ let w = W {{ v: 7 }}\n\
             let b: Bool = w.get_item()\n println(\"${{b}}\") }}"
        ));
        assert!(
            bad.diagnostics.iter().any(|d| d.code == "E0010"),
            "W<Int>::Item is Int, not Bool: {:?}",
            bad.diagnostics
        );
    }

    /// A trait method may declare a *parameter* as `Self::Item`, and the caller
    /// passes a concrete value for it. The plan named only the method-call
    /// *return* type; the parameter is the mirror at the same seam, and without
    /// it `w.put(9)` reads "argument has type `Int` but `W<Int>::Item` was
    /// expected".
    #[test]
    fn a_projection_in_a_trait_method_parameter_normalizes_at_the_call() {
        let prelude = "trait It { type Item\n fn put(self, x: Self::Item) -> Int }\n\
             record W<T> { v: T }\n\
             impl<T> It for W<T> { type Item = T\n\
             fn put(self, x: Self::Item) -> Int { 1 } }\n";
        let r = check_src(&format!(
            "{prelude}fn main() {{ println(\"${{(W {{ v: 7 }}).put(9)}}\") }}"
        ));
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
        // And the parameter still constrains: `W<Int>::Item` is `Int`.
        let bad = check_src(&format!(
            "{prelude}fn main() {{ println(\"${{(W {{ v: 7 }}).put(true)}}\") }}"
        ));
        assert!(
            bad.diagnostics.iter().any(|d| d.code == "E0010"),
            "a Bool argument where Int was expected: {:?}",
            bad.diagnostics
        );
    }

    /// An *impl* method whose parameter is declared `Self::Item` binds a local of
    /// that type, so `check_fn_body` has to normalize `sig.params` as well as
    /// `sig.ret` — otherwise `x + 1` reports "mismatched operand types:
    /// `K::Item` vs `Int`" inside a body the signature seam already accepted.
    #[test]
    fn a_projection_in_an_impl_method_parameter_normalizes_in_the_body() {
        let r = check_src(
            "trait It { type Item\n fn put(self, x: Self::Item) -> Int }\n\
             record K { k: Int }\n\
             impl It for K { type Item = Int\n\
             fn put(self, x: Self::Item) -> Int { x + 1 } }\n\
             fn main() { println(\"${(K { k: 0 }).put(9)}\") }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
        // The local's recorded type, not just the absence of diagnostics: an
        // annotation that collapsed to `Ty::Error` would also add nothing here.
        let m = r
            .module
            .functions
            .iter()
            .find(|f| f.name == "K.It.put")
            .expect("the impl's `put` was compiled");
        let x = m
            .locals
            .iter()
            .find(|l| l.name == "x")
            .expect("local `x` compiled");
        assert_eq!(x.ty, Ty::Int, "`K::Item` normalized to Int");
    }

    /// A generic *free* function whose signature projects onto its own type
    /// parameter. Neither the plan's three seams nor Task 7's monomorphization
    /// covers this: the instantiation is fully concrete here in typeck, and it is
    /// the exact shape of Task 7's own Step 1 test
    /// (`fn unwrap_item<I: It>(x: I) -> I::Item`), which cannot even be called
    /// without it.
    #[test]
    fn a_projection_in_a_generic_functions_signature_normalizes_at_the_call() {
        let prelude = "trait It { type Item\n fn get_item(self) -> Self::Item }\n\
             record W<T> { v: T }\n\
             record K { k: Bool }\n\
             impl<T> It for W<T> { type Item = T\n\
             fn get_item(self) -> Self::Item { self.v } }\n\
             impl It for K { type Item = Bool\n\
             fn get_item(self) -> Self::Item { self.k } }\n\
             fn first<I: It>(x: I) -> I::Item { x.get_item() }\n\
             fn take<I: It>(x: I, y: I::Item) -> Int { 1 }\n";
        // Two instantiations of the same generic function, so "resolves per
        // instantiation" is distinguishable from "resolves once".
        let r = check_src(&format!(
            "{prelude}fn main() {{ let n: Int = first(W {{ v: 7 }})\n\
             let b: Bool = first(K {{ k: true }})\n\
             let t: Int = take(W {{ v: 7 }}, 9)\n\
             println(\"${{n}} ${{b}} ${{t}}\") }}"
        ));
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
        let bad = check_src(&format!(
            "{prelude}fn main() {{ let b: Bool = first(W {{ v: 7 }})\n println(\"${{b}}\") }}"
        ));
        assert!(
            bad.diagnostics.iter().any(|d| d.code == "E0010"),
            "`W<Int>::Item` is Int, not Bool: {:?}",
            bad.diagnostics
        );
        let bad_arg = check_src(&format!(
            "{prelude}fn main() {{ println(\"${{take(W {{ v: 7 }}, true)}}\") }}"
        ));
        assert!(
            bad_arg.diagnostics.iter().any(|d| d.code == "E0010"),
            "`I::Item` is Int for `W<Int>`, so a Bool argument is wrong: {:?}",
            bad_arg.diagnostics
        );
    }

    /// The design doc §4.2 says `Assoc { on: Var(_) }` cannot arise. It can, in
    /// one shape: a generic call whose projection-typed parameter comes *before*
    /// the parameter that determines the type, so nothing has solved the variable
    /// when the projection is instantiated.
    ///
    /// Pinned as a **known gap**, not as desired behaviour. Fixing it needs
    /// deferred obligations, which §4.1 rules out for this increment. The value
    /// of the test is that closing it later is a visible decision with a test to
    /// change, and that the failure mode stays an ordinary type mismatch.
    #[test]
    fn known_gap_a_projection_parameter_before_its_determining_parameter() {
        let r = check_src(
            "trait It { type Item\n fn get_item(self) -> Self::Item }\n\
             record W<T> { v: T }\n\
             impl<T> It for W<T> { type Item = T\n\
             fn get_item(self) -> Self::Item { self.v } }\n\
             fn take<I: It>(y: I::Item, x: I) -> Int { 1 }\n\
             fn main() { println(\"${take(9, W { v: 7 })}\") }",
        );
        let d = r
            .diagnostics
            .iter()
            .find(|d| d.code == "E0010")
            .expect("the known gap still reports a plain type mismatch");
        // Names the unresolved projection rather than claiming anything false.
        assert!(
            d.message.contains("::Item"),
            "the message names the projection: {}",
            d.message
        );
        // Reversing the parameters is the workaround, and it must work — the gap
        // is about argument order, not about the shape being unsupported.
        let reversed = check_src(
            "trait It { type Item\n fn get_item(self) -> Self::Item }\n\
             record W<T> { v: T }\n\
             impl<T> It for W<T> { type Item = T\n\
             fn get_item(self) -> Self::Item { self.v } }\n\
             fn take<I: It>(x: I, y: I::Item) -> Int { 1 }\n\
             fn main() { println(\"${take(W { v: 7 }, 9)}\") }",
        );
        assert!(
            reversed.diagnostics.is_empty(),
            "{:?}",
            reversed.diagnostics
        );
    }

    #[test]
    fn a_projection_nested_inside_a_compound_type_normalizes() {
        // `[Self::Item]`, not `Self::Item`. A `normalize` that handles a
        // top-level projection but does not recurse into `Array`/`Record`/
        // `Sum`/`Fn` passes every test whose projection is the whole type.
        let r = check_src(
            "trait It { type Item\n fn items(self) -> [Self::Item] }\n\
             record W<T> { v: T }\n\
             impl<T> It for W<T> { type Item = T\n\
             fn items(self) -> [Self::Item] { [self.v] } }\n\
             fn main() { let w = W { v: 7 }\n let a: [Int] = w.items()\n\
             println(\"${a.len()}\") }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
        let bad = check_src(
            "trait It { type Item\n fn items(self) -> [Self::Item] }\n\
             record W<T> { v: T }\n\
             impl<T> It for W<T> { type Item = T\n\
             fn items(self) -> [Self::Item] { [self.v] } }\n\
             fn main() { let a: [Bool] = W { v: 7 }.items()\n println(\"${a.len()}\") }",
        );
        assert!(
            bad.diagnostics.iter().any(|d| d.code == "E0010"),
            "`[W<Int>::Item]` is `[Int]`, not `[Bool]`: {:?}",
            bad.diagnostics
        );
    }

    #[test]
    fn a_projection_in_a_let_annotation_normalizes() {
        // The `let`-annotation seam. `value.ty = annot_ty` happens *after* the
        // unify, so an unnormalized projection written here becomes the
        // binding's type and propagates to every later use of it.
        let r = check_src(
            "trait It { type Item\n fn get_item(self) -> Self::Item }\n\
             record W<T> { v: T }\n\
             impl<T> It for W<T> { type Item = T\n\
             fn get_item(self) -> Self::Item { let n: Self::Item = self.v\n n } }\n\
             fn main() { let w = W { v: 7 }\n println(\"${w.get_item()}\") }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
        // And the annotation still constrains: `Self::Item` is `T` here, so
        // initializing it from a `Bool` must fail.
        let bad = check_src(
            "trait It { type Item\n fn get_item(self) -> Self::Item }\n\
             record W<T> { v: T }\n\
             impl<T> It for W<T> { type Item = T\n\
             fn get_item(self) -> Self::Item { let n: Self::Item = true\n self.v } }\n\
             fn main() { }",
        );
        assert!(
            bad.diagnostics.iter().any(|d| d.code == "E0010"),
            "expected a mismatch on the annotated let: {:?}",
            bad.diagnostics
        );
    }

    #[test]
    fn a_method_body_checks_against_its_normalized_return_type() {
        // The function-return seam, on a *non-generic* impl so the expected type
        // is a concrete `Int` rather than a parameter: the body must be accepted
        // when it matches and rejected when it does not. The negative half is
        // what distinguishes normalizing from discarding.
        let ok = check_src(
            "trait It { type Item\n fn get_item(self) -> Self::Item }\n\
             record K { k: Int }\n\
             impl It for K { type Item = Int\n fn get_item(self) -> Self::Item { 1 } }\n\
             fn main() { }",
        );
        assert!(ok.diagnostics.is_empty(), "{:?}", ok.diagnostics);
        let bad = check_src(
            "trait It { type Item\n fn get_item(self) -> Self::Item }\n\
             record K { k: Int }\n\
             impl It for K { type Item = Int\n fn get_item(self) -> Self::Item { true } }\n\
             fn main() { }",
        );
        assert!(
            bad.diagnostics.iter().any(|d| d.code == "E0010"),
            "`K::Item` is `Int`, so a `Bool` body is wrong: {:?}",
            bad.diagnostics
        );
    }

    #[test]
    fn a_chain_of_associated_type_bindings_normalizes_all_the_way() {
        // `A = Self::B` and `B = Int`: one resolution step yields `Self::B`, not
        // `Int`. A `normalize` that does not re-normalize its own result fails
        // here and passes every other positive test in this file.
        let prelude = "trait Two { type A\n type B\n fn get_a(self) -> Self::A }\n\
             record W { v: Int }\n\
             impl Two for W { type A = Self::B\n type B = Int\n\
             fn get_a(self) -> Self::A { self.v } }\n";
        let r = check_src(&format!(
            "{prelude}fn main() {{ let n: Int = W {{ v: 1 }}.get_a()\n println(\"${{n}}\") }}"
        ));
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
        let bad = check_src(&format!(
            "{prelude}fn main() {{ let b: Bool = W {{ v: 1 }}.get_a()\n println(\"${{b}}\") }}"
        ));
        assert!(
            bad.diagnostics.iter().any(|d| d.code == "E0010"),
            "the chain ends at Int, not Bool: {:?}",
            bad.diagnostics
        );
    }

    /// `E0077` poisons the binding to `Ty::Error`, so a *use* of the cyclic
    /// projection adds nothing: not a second `E0010`, and not `normalize`'s
    /// `E0078` depth-limit report. One mistake, one diagnostic.
    ///
    /// **The impl's return type is `Int`, deliberately, and this test was wrong
    /// before it was.** It used to spell `Self::Item` on the impl side too, so
    /// *both* sides of `check_impl_method_signatures`' comparison normalized to
    /// `Ty::Error` and compared **equal** — the test passed by accident of its
    /// own spelling, not because the property it is named for held. Changing that
    /// one token to a concrete type made it fail with
    /// `E0072: method `get` returns `Int` but trait `It` declares `{error}``,
    /// which is the cascade the name denies. Fixed at the comparison (see the
    /// `has_error` guard there and the poisoning comment on
    /// `check_assoc_binding_cycles`), and the spelling is now the one that can
    /// see it.
    #[test]
    fn a_reported_binding_cycle_is_poisoned_and_does_not_cascade() {
        let r = check_src(
            "trait It { type Item\n fn get(self) -> Self::Item }\n\
             record W { v: Int }\n\
             impl It for W { type Item = Self::Item\n\
             fn get(self) -> Int { 1 } }\n\
             fn main() { println(\"${W { v: 1 }.get()}\") }",
        );
        assert_eq!(error_codes(&r), ["E0077"], "{:?}", r.diagnostics);
    }

    /// A trait with `n+1` associated types `A0..An`, a method taking `Self::A0`,
    /// and an impl binding each `Ak` from `link(k)`. Terminates at `An = Int`, so
    /// there is no cycle and `E0077` never fires — this is the shape that makes
    /// `E0078` reachable from ordinary source.
    ///
    /// Accepted, when it is accepted: the method takes `Self::A0` as a
    /// *parameter* and returns `Int`, so conformance compares two spellings of the
    /// same projection and the body is just `1`. An `E0078` here is therefore the
    /// only diagnostic, never a knock-on of some other rejection.
    fn assoc_chain_src(n: u32, link: impl Fn(u32) -> String) -> String {
        let decls: String = (0..=n).map(|k| format!(" type A{k}\n")).collect();
        let binds: String = (0..n)
            .map(|k| format!(" type A{} = {}\n", k, link(k)))
            .collect();
        format!(
            "record Pair<A, B> {{ a: A\n b: B }}\n\
             trait Chain {{\n{decls} fn put(self, y: Self::A0) -> Int }}\n\
             record W {{ v: Int }}\n\
             impl Chain for W {{\n{binds} type A{n} = Int\n\
             fn put(self, y: Self::A0) -> Int {{ 1 }} }}\n\
             fn main() {{ }}"
        )
    }

    #[test]
    fn a_binding_chain_that_resolves_to_an_enormous_type_is_a_diagnostic() {
        // `type A(k) = Pair<Self::A(k+1), Self::A(k+1)>`. No cycle and only 16
        // links, so neither `E0077` nor the depth limit sees anything wrong — but
        // the resolved type is `2^16` nodes. Before the step allowance this
        // *compiled*, slowly: measured through `nova check`, 16 links took 716 ms,
        // 20 took 12.9 s, and 24 (a 60-line file) had not finished after two
        // minutes. Now 41 ms and an error.
        //
        // The failure has to be a diagnostic, and it has to be *this* code: a hang
        // is worse than any error, and `Ty::Error` without a diagnostic would be
        // worse still, because it unifies with anything and would turn the hang
        // into a silently wrong type.
        let branching =
            assoc_chain_src(16, |k| format!("Pair<Self::A{}, Self::A{}>", k + 1, k + 1));
        let r = check_src(&branching);
        // Three reports, all `E0078`, for one root cause. That is **pre-existing
        // duplication**, not a consequence of this fix: `normalize` has three call
        // sites this program reaches — conformance normalizes the trait side and
        // the impl side of the same signature (two, same span), and
        // `check_fn_body` normalizes `sig.params` (one, the body's span) — and
        // nothing in this codebase deduplicates diagnostics. Verified by rebuilding
        // the base commit and counting: `nova check` printed three `E0078`s there
        // too, and the number of `self.normalize(` call sites is unchanged at 8.
        //
        // Asserted rather than tolerated, because before this seam was reachable
        // nobody could see it. Asserting only "contains E0078" would hide both the
        // duplication and any *other* code creeping in.
        assert_eq!(
            error_codes(&r),
            ["E0078", "E0078", "E0078"],
            "{:?}",
            r.diagnostics
        );
        // And that it reports the *size* limit, not the depth one. The chain here
        // is 16 links, comfortably inside the depth limit of 64, so a message
        // blaming depth would be actively misleading — it would tell the user to
        // shorten a chain that is already short. Added after a mutation that
        // reported `NormalizeLimit::Depth` for every overflow survived this test
        // with only the code asserted.
        assert!(
            r.diagnostics[0]
                .message
                .contains("resolves to more than 10000 type nodes"),
            "a wide chain is a size report, not a depth report: {}",
            r.diagnostics[0].message
        );

        // The control: the same shape, small enough to resolve. Without this, the
        // test above would also pass if all branching were rejected outright.
        let small = assoc_chain_src(4, |k| format!("Pair<Self::A{}, Self::A{}>", k + 1, k + 1));
        let ok = check_src(&small);
        assert!(ok.diagnostics.is_empty(), "{:?}", ok.diagnostics);
    }

    #[test]
    fn a_binding_chain_longer_than_the_depth_limit_is_a_diagnostic() {
        // A plain linear chain, which is neither a cycle nor large — it resolves
        // to `Int`. It is only *long*. This is the case that proves `E0078` is
        // reachable from source and that the comment claiming otherwise ("a
        // compiler defect") was wrong: 63 links check clean, 64 report.
        //
        // Both boundaries are asserted because a limit test that only checks the
        // rejecting side cannot tell the limit from a blanket refusal.
        let ok = check_src(&assoc_chain_src(63, |k| format!("Self::A{}", k + 1)));
        assert!(ok.diagnostics.is_empty(), "{:?}", ok.diagnostics);
        let r = check_src(&assoc_chain_src(64, |k| format!("Self::A{}", k + 1)));
        // Three, for the pre-existing reason recorded on the test above.
        assert_eq!(
            error_codes(&r),
            ["E0078", "E0078", "E0078"],
            "{:?}",
            r.diagnostics
        );
        // And that the two limits are told apart in the message. A single
        // catch-all wording would make this chain read as if it were too big.
        assert!(
            r.diagnostics[0].message.contains("more than 64 deep"),
            "a long chain is a depth report, not a size report: {}",
            r.diagnostics[0].message
        );
    }

    // === Normalization seam 2: impl conformance (design doc §4.1, risk 2) ===
    //
    // The dangerous direction here is **over-acceptance**. This seam makes two
    // types that used to compare unequal compare equal, so a suite that only
    // checks "the good impl is accepted" cannot tell the real fix from deleting
    // the comparison. Every positive test below is therefore paired with the
    // negative the same code path must still reject.

    #[test]
    fn an_impl_may_echo_the_projection_or_write_the_concrete_type() {
        // Both spellings must be accepted (design doc §5.1).
        for ret in ["T", "Self::Item"] {
            let src = format!(
                "trait It {{ type Item\n fn get_item(self) -> Self::Item }}\n\
                 record W<T> {{ v: T }}\n\
                 impl<T> It for W<T> {{ type Item = T\n fn get_item(self) -> {ret} {{ self.v }} }}\n\
                 fn main() {{ }}"
            );
            let r = check_src(&src);
            assert!(r.diagnostics.is_empty(), "ret = {ret}: {:?}", r.diagnostics);
        }
    }

    #[test]
    fn a_genuinely_wrong_impl_signature_still_reports_e0072() {
        // The risk is that normalizing to make the two spellings agree also
        // makes everything agree. Self::Item is T here, so returning Bool is
        // wrong and must still be caught.
        let r = check_src(
            "trait It { type Item\n fn get_item(self) -> Self::Item }\n\
             record W<T> { v: T }\n\
             impl<T> It for W<T> { type Item = T\n fn get_item(self) -> Bool { true } }\n\
             fn main() { }",
        );
        // Exactly one, and exactly the conformance error — not a `contains` that
        // would also hold if the fix had added a second, cascading diagnostic.
        assert_eq!(error_codes(&r), ["E0072"], "{:?}", r.diagnostics);
        // And the message names the **normalized** trait type. Before this seam
        // it read "declares `W<T0>::Item`" — the projection the user never
        // wrote. A fix that normalized only the impl side would still report
        // E0072 here, so the code alone cannot tell the two apart.
        let msg = &r.diagnostics[0].message;
        assert!(
            msg.contains("returns `Bool`") && msg.contains("declares `T0`"),
            "the diagnostic reports the normalized trait type: {msg}"
        );
    }

    #[test]
    fn conformance_normalizes_parameter_types_not_only_the_return_type() {
        // Normalizing only the return type is the plausible half-fix: it passes
        // every return-position test in this file and leaves this one reporting a
        // bogus E0072 on parameter 1.
        let ok = check_src(
            "trait It { type Item\n fn put(self, x: Self::Item) -> Int }\n\
             record W<T> { v: T }\n\
             impl<T> It for W<T> { type Item = T\n fn put(self, x: T) -> Int { 1 } }\n\
             fn main() { }",
        );
        assert!(ok.diagnostics.is_empty(), "{:?}", ok.diagnostics);
        // The negative: `Self::Item` is `T`, so `x: Bool` is wrong and must still
        // be rejected. `x` is deliberately unused, which produces no diagnostic
        // of its own — measured, so the exact-sequence assertion is safe.
        let bad = check_src(
            "trait It { type Item\n fn put(self, x: Self::Item) -> Int }\n\
             record W<T> { v: T }\n\
             impl<T> It for W<T> { type Item = T\n fn put(self, x: Bool) -> Int { 1 } }\n\
             fn main() { }",
        );
        assert_eq!(error_codes(&bad), ["E0072"], "{:?}", bad.diagnostics);
        let msg = &bad.diagnostics[0].message;
        assert!(
            msg.contains("parameter 1") && msg.contains("declares `T0`"),
            "the parameter diagnostic reports the normalized trait type: {msg}"
        );
    }

    #[test]
    fn conformance_normalizes_inside_a_compound_type() {
        // `[Self::Item]` against `[T]`. A comparison that normalized only a
        // top-level projection passes every other positive test here and fails
        // this one — the same boundary
        // `a_projection_nested_inside_a_compound_type_normalizes` pins for seam
        // 1, at seam 2.
        let ok = check_src(
            "trait It { type Item\n fn items(self) -> [Self::Item] }\n\
             record W<T> { v: T }\n\
             impl<T> It for W<T> { type Item = T\n fn items(self) -> [T] { [self.v] } }\n\
             fn main() { }",
        );
        assert!(ok.diagnostics.is_empty(), "{:?}", ok.diagnostics);
        // `[Bool]` is still wrong, so the compound case is not waved through
        // wholesale.
        let bad = check_src(
            "trait It { type Item\n fn items(self) -> [Self::Item] }\n\
             record W<T> { v: T }\n\
             impl<T> It for W<T> { type Item = T\n fn items(self) -> [Bool] { [true] } }\n\
             fn main() { }",
        );
        assert_eq!(error_codes(&bad), ["E0072"], "{:?}", bad.diagnostics);
    }

    #[test]
    fn conformance_still_rejects_a_wrong_arity_and_a_wrong_receiver() {
        // Neither of these compares a projection at all — the arity check counts
        // parameters and the receiver check compares two bools — but both sit on
        // the code path this seam moved out of `check_impl_conformance`, and both
        // `continue` past the type comparison. They are the checks that can
        // silently stop running while every projection-shaped test stays green.
        let arity = check_src(
            "trait It { type Item\n fn put(self, x: Self::Item) -> Int }\n\
             record W<T> { v: T }\n\
             impl<T> It for W<T> { type Item = T\n fn put(self) -> Int { 1 } }\n\
             fn main() { }",
        );
        assert_eq!(error_codes(&arity), ["E0072"], "{:?}", arity.diagnostics);
        // Specifically the arity message, not a per-parameter mismatch: the
        // count is compared before substitution and normalization precisely so a
        // wrong arity reads as one.
        assert!(
            arity.diagnostics[0].message.contains("0 parameter(s)"),
            "{}",
            arity.diagnostics[0].message
        );
        // A trait method declared with `self`, implemented as an associated
        // function. The return types agree (`T` on both sides once normalized),
        // so the receiver is the only disagreement and nothing else can catch it.
        let recv = check_src(
            "trait It { type Item\n fn get_item(self) -> Self::Item }\n\
             record W<T> { v: T }\n\
             impl<T> It for W<T> { type Item = T\n\
             fn get_item() -> T { panic(\"no\") } }\n\
             fn main() { }",
        );
        assert_eq!(error_codes(&recv), ["E0072"], "{:?}", recv.diagnostics);
        assert!(
            recv.diagnostics[0].message.contains("`self` receiver"),
            "{}",
            recv.diagnostics[0].message
        );
    }

    #[test]
    fn conformance_normalizes_through_the_matching_impl_not_the_only_one() {
        // Two impls of one trait with **different** bindings, each writing its
        // own concrete type. A normalizer that takes the first impl, or the only
        // impl, or ignores the self type answers `T` where `[T]` is wanted and
        // reports E0072 on one of them — which a single-impl test cannot see.
        let prelude = "trait It { type Item\n fn peek(self) -> Self::Item }\n\
             record W<T> { v: T }\n\
             record K<T> { k: T }\n";
        let ok = check_src(&format!(
            "{prelude}\
             impl<T> It for W<T> {{ type Item = T\n fn peek(self) -> T {{ self.v }} }}\n\
             impl<T> It for K<T> {{ type Item = [T]\n fn peek(self) -> [T] {{ [self.k] }} }}\n\
             fn main() {{ }}"
        ));
        assert!(ok.diagnostics.is_empty(), "{:?}", ok.diagnostics);
        // Swap the two return types and *both* impls must be rejected. Without
        // this half, the positive above would also hold for a normalizer that
        // answered "whichever binding fits" rather than "this impl's binding".
        let swapped = check_src(&format!(
            "{prelude}\
             impl<T> It for W<T> {{ type Item = T\n fn peek(self) -> [T] {{ [self.v] }} }}\n\
             impl<T> It for K<T> {{ type Item = [T]\n fn peek(self) -> T {{ self.k }} }}\n\
             fn main() {{ }}"
        ));
        assert_eq!(
            error_codes(&swapped),
            ["E0072", "E0072"],
            "{:?}",
            swapped.diagnostics
        );
    }

    #[test]
    fn conformance_resolves_a_projection_bound_by_a_later_declared_impl() {
        // The discriminating test for why this seam is a post-collection pass
        // rather than a `normalize` inside `check_impl_conformance` with the
        // `ImplInfo` push hoisted above the call.
        //
        // A supertrait's associated type is reachable from a subtrait method's
        // signature: `collect_traits` puts the trait *plus its expanded
        // supertraits* in `sig_bounds[0]`, so `Self::Elem` inside
        // `trait Ext: Base` is `Base`'s `Elem`. Substituting `Self` at
        // conformance yields `Assoc { on: W }` — a projection with a head whose
        // binding lives in a **different** impl, `impl Base for W`, which the
        // user may write either side of `impl Ext for W`.
        //
        // Hoisting the push fixes only the impl's own bindings: the "sub first"
        // ordering below would still fail, because normalization consults the
        // whole table and would see only impls from earlier items. Nova has no
        // declaration-order rule for impls — the same reason
        // `check_supertrait_impls` was moved out of conformance — so both
        // orderings must pass.
        //
        // **Two shapes, added in Task 11 Step 6.** The original test only had
        // the primitive-on-a-non-generic-record row, which resolves the
        // projection through a **ground** self type: `Assoc { on: W }`, whose
        // binding is `Int`. Nothing there can tell whether the impl's own type
        // arguments reach the binding at all, because `Int.subst(_)` is `Int`
        // for every argument list.
        //
        // The second row is the one with teeth: the supertrait impl binds
        // `Elem = A` — its *first* parameter — and the subtrait impl is
        // partially concrete, `impl<T> Ext for W<Int, T>`. So conformance
        // resolves `Assoc { on: W<Int, Param(0)> }`, `match_args` recovers
        // `[Int, Param(0)]`, and `Elem` is `Int` **only if** those arguments are
        // substituted into the binding. Measured: dropping the `subst` in
        // `hir::normalize_ty`'s `Assoc` arm leaves the primitive row passing and
        // fails this one.
        //
        // The plan asked for `record W<T>` with `type Elem = T`. That is not
        // enough, and the reason is worth recording: `match_args` on
        // `W<Param(0)>` recovers `[Param(0)]`, so `Param(0).subst([Param(0)])` is
        // the *identity* — the substitution runs but cannot be observed, and the
        // subst-dropping mutation survives it. A parameter of the impl only
        // "survives substitution" observably when the argument it maps to is
        // something else.
        for (shape, decl, ext_ok, ext_bad, base) in [
            (
                "a primitive on a non-generic record",
                "record W { v: Int }\n",
                "impl Ext for W { fn peek(self) -> Int { self.v } }\n",
                "impl Ext for W { fn peek(self) -> Bool { true } }\n",
                "impl Base for W { type Elem = Int }\n",
            ),
            (
                "the impl's own arguments substituted into a supertrait's binding",
                "record W<A, B> { a: A\n b: B }\n",
                "impl<T> Ext for W<Int, T> { fn peek(self) -> Int { self.a } }\n",
                "impl<T> Ext for W<Int, T> { fn peek(self) -> Bool { true } }\n",
                "impl<A, B> Base for W<A, B> { type Elem = A }\n",
            ),
        ] {
            let src = |impls: String| {
                format!(
                    "trait Base {{ type Elem }}\n\
                     trait Ext: Base {{ fn peek(self) -> Self::Elem }}\n\
                     {decl}{impls}fn main() {{ }}"
                )
            };
            for (order, impls) in [
                ("sub first", format!("{ext_ok}{base}")),
                ("super first", format!("{base}{ext_ok}")),
            ] {
                let r = check_src(&src(impls));
                assert!(
                    r.diagnostics.is_empty(),
                    "{shape}, {order}: {:?}",
                    r.diagnostics
                );
            }
            // Not vacuous in either order: `Elem` is the element type, so a
            // `Bool` return is still wrong however the two impls are ordered.
            // Without this half, a pass that skipped the comparison whenever a
            // projection was involved would satisfy the loop above.
            for (order, impls) in [
                ("sub first", format!("{ext_bad}{base}")),
                ("super first", format!("{base}{ext_bad}")),
            ] {
                let r = check_src(&src(impls));
                assert_eq!(
                    error_codes(&r),
                    ["E0072"],
                    "{shape}, {order}: {:?}",
                    r.diagnostics
                );
            }
        }
    }

    /// Task 11 Step 6, second half. The **selfless** branch of the conformance
    /// comparison, with a projection in it.
    ///
    /// A trait method declared without a receiver leaves the impl signature with
    /// no `self` to skip, so `check_impl_method_signatures` compares
    /// `impl_sig.params` as-is instead of `params[1..]`. Its nearest existing
    /// test, `selfless_trait_impl_method_checks_conformance_without_panicking`,
    /// cannot see the branch at all: with `fn zero() -> Int` both arms produce an
    /// empty parameter list, so always slicing `[1..]` is an equivalent mutant
    /// there. A *parameter* is what makes the branch observable.
    ///
    /// Measured, so the claim is exact rather than "this branch was untested":
    /// under the always-slice mutation the whole workspace reports **two**
    /// failures — this test and
    /// `trait_call_substitution_puts_self_before_the_methods_own_generics`, whose
    /// `fn make<U>(u: U) -> Self` row also puts a parameter on a receiverless
    /// trait method. So the branch was partly pinned; what had no test is the
    /// branch **with a projection on it**, where the parameter list the arm
    /// selects is also the list normalization runs over.
    ///
    /// The impl echoes `Self::Out` in both positions rather than writing `Int`,
    /// so this is also the §5.1 "either spelling" row for an associated function
    /// — the one place the echo was never exercised.
    #[test]
    fn conformance_normalizes_a_selfless_methods_projection() {
        let prelude = "trait Zero { type Out\n\
                       fn zero() -> Self::Out\n\
                       fn of(x: Self::Out) -> Self::Out }\n\
                       record P { v: Int }\n";
        // Echoed on the impl side, in a parameter and in the return type.
        let echoed = check_src(&format!(
            "{prelude}impl Zero for P {{ type Out = Int\n\
             fn zero() -> Int {{ 0 }}\n\
             fn of(x: Self::Out) -> Self::Out {{ x }} }}\n\
             fn main() {{ println(\"${{P::of(P::zero())}}\") }}"
        ));
        assert!(echoed.diagnostics.is_empty(), "{:?}", echoed.diagnostics);
        // And the concrete spelling of the same signature.
        let concrete = check_src(&format!(
            "{prelude}impl Zero for P {{ type Out = Int\n\
             fn zero() -> Int {{ 0 }}\n\
             fn of(x: Int) -> Int {{ x }} }}\n\
             fn main() {{ println(\"${{P::of(P::zero())}}\") }}"
        ));
        assert!(
            concrete.diagnostics.is_empty(),
            "{:?}",
            concrete.diagnostics
        );
        // Not vacuous: the *parameter* is what the branch decides, so a wrong
        // parameter type is the assertion that matters. `Out` is `Int`, so a
        // `Bool` parameter is a mismatch and must be reported as one — not as an
        // arity error, which is what slicing an already-receiverless list
        // produces.
        let wrong = check_src(&format!(
            "{prelude}impl Zero for P {{ type Out = Int\n\
             fn zero() -> Int {{ 0 }}\n\
             fn of(x: Bool) -> Int {{ 1 }} }}\n\
             fn main() {{ }}"
        ));
        assert_eq!(error_codes(&wrong), ["E0072"], "{:?}", wrong.diagnostics);
        assert!(
            wrong.diagnostics[0].message.contains("parameter 1"),
            "the parameter is compared, not sliced away: {}",
            wrong.diagnostics[0].message
        );
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
                Builtin::Println | Builtin::Print | Builtin::EPrint | Builtin::EPrintln => (
                    (vec![Ty::String], Ty::Unit),
                    "`println(s)` / `print(s)` / `eprint(s)` / `eprintln(s)`",
                ),
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
                Builtin::TestSelector => (
                    (vec![], Ty::Int),
                    "the synthesized `main`'s dispatch, reading `NOVA_TEST_INDEX`",
                ),
                Builtin::TaskSpawn => (
                    (vec![Ty::Future(Box::new(Ty::Param(0)))], Ty::Int),
                    "`task_spawn(fut)` in `std/task`'s `spawn<T>`",
                ),
                Builtin::TaskIsDone => (
                    (vec![Ty::Future(Box::new(Ty::Param(0)))], Ty::Bool),
                    "no call site in `std/task` since Task 3's `join` -- kept \
                     as part of the executor's Nova-facing surface",
                ),
                Builtin::TaskRelease => (
                    (vec![Ty::Future(Box::new(Ty::Param(0)))], Ty::Unit),
                    "`task_release(self.fut)` in `JoinHandle<T>::join`",
                ),
                Builtin::TaskDrive => (
                    (vec![Ty::Future(Box::new(Ty::Param(0)))], Ty::Unit),
                    "`task_drive(fut)` in `std/task`'s `block_on<T>`",
                ),
                Builtin::TaskOutput => (
                    (vec![Ty::Future(Box::new(Ty::Param(0)))], Ty::Param(0)),
                    "`task_output(fut)` in `std/task`'s `block_on<T>` and `JoinHandle<T>::join`",
                ),
                Builtin::TaskYieldFuture => (
                    (vec![], Ty::Future(Box::new(Ty::Unit))),
                    "`task_yield_future().await` in `std/task`'s `yield_now`",
                ),
                Builtin::TaskSleepFuture => (
                    (vec![Ty::Int], Ty::Future(Box::new(Ty::Unit))),
                    "`task_sleep_future(ms).await` in `std/task`'s `sleep`",
                ),
                Builtin::TaskJoinFuture => (
                    (
                        vec![Ty::Future(Box::new(Ty::Param(0)))],
                        Ty::Future(Box::new(Ty::Unit)),
                    ),
                    "`task_join_future(self.fut).await` in `JoinHandle<T>::join`",
                ),
                Builtin::FsReadToString => (
                    (vec![Ty::String], Ty::Int),
                    "`fs_read_to_string(path)` in `std/fs`'s `read_to_string`",
                ),
                Builtin::FsWriteString => (
                    (vec![Ty::String, Ty::String], Ty::Int),
                    "`fs_write_string(path, content)` in `std/fs`'s `write_string`",
                ),
                Builtin::FsTakeString => (
                    (vec![], Ty::String),
                    "`fs_take_string()` in `std/fs`'s `read_to_string`",
                ),
                Builtin::FsLastErrorMessage => (
                    (vec![], Ty::String),
                    "`fs_last_error_message()` in every fallible `std/fs` wrapper: \
                     `read_to_string`, `write_string`, `create_dir`, \
                     `create_dir_all`, `remove_file`, `remove_dir_all`, `read_dir`",
                ),
                Builtin::FsTempDir => (
                    (vec![], Ty::String),
                    "`fs_temp_dir()` in `std/fs`'s `temp_dir`",
                ),
                Builtin::FsExists => (
                    (vec![Ty::String], Ty::Bool),
                    "`fs_exists(path)` in `std/fs`'s `exists`",
                ),
                Builtin::FsCreateDir => (
                    (vec![Ty::String], Ty::Int),
                    "`fs_create_dir(path)` in `std/fs`'s `create_dir`",
                ),
                Builtin::FsCreateDirAll => (
                    (vec![Ty::String], Ty::Int),
                    "`fs_create_dir_all(path)` in `std/fs`'s `create_dir_all`",
                ),
                Builtin::FsRemoveFile => (
                    (vec![Ty::String], Ty::Int),
                    "`fs_remove_file(path)` in `std/fs`'s `remove_file`",
                ),
                Builtin::FsRemoveDirAll => (
                    (vec![Ty::String], Ty::Int),
                    "`fs_remove_dir_all(path)` in `std/fs`'s `remove_dir_all`",
                ),
                Builtin::FsReadDir => (
                    (vec![Ty::String], Ty::Int),
                    "`fs_read_dir(path)` in `std/fs`'s `read_dir`",
                ),
                Builtin::FsTakeStringArray => (
                    (vec![], Ty::Array(Box::new(Ty::String))),
                    "`fs_take_string_array()` in `std/fs`'s `read_dir`",
                ),
                Builtin::FsKind => (
                    (vec![Ty::String], Ty::Int),
                    "`fs_kind(path)` in `std/fs`'s `read_dir`",
                ),
                Builtin::FsRead => (
                    (vec![Ty::String], Ty::Int),
                    "`fs_read(path)` in `std/fs`'s `read`",
                ),
                Builtin::FsTakeBytes => (
                    (vec![], Ty::Bytes),
                    "`fs_take_bytes()` in `std/fs`'s `read`",
                ),
                Builtin::FsWrite => (
                    (vec![Ty::String, Ty::Bytes], Ty::Int),
                    "`fs_write(path, content)` in `std/fs`'s `write`",
                ),
                Builtin::BytesLen => (
                    (vec![Ty::Bytes], Ty::Int),
                    "`bytes_len(self)` in `Bytes::len`",
                ),
                Builtin::BytesFromString => (
                    (vec![Ty::String], Ty::Bytes),
                    "`bytes_from_string_intrinsic(s)` in the free function `bytes_from_string`",
                ),
                Builtin::BytesIsUtf8 => (
                    (vec![Ty::Bytes], Ty::Bool),
                    "`bytes_is_utf8(self)` in `Bytes::to_string`",
                ),
                Builtin::BytesToStringUnchecked => (
                    (vec![Ty::Bytes], Ty::String),
                    "`bytes_to_string_unchecked(self)` in `Bytes::to_string`",
                ),
                Builtin::BytesAt => (
                    (vec![Ty::Bytes, Ty::Int], Ty::Int),
                    "`bytes_at(self, i)` in `Bytes::byte_at`",
                ),
                Builtin::BytesSlice => (
                    (vec![Ty::Bytes, Ty::Int, Ty::Int], Ty::Bytes),
                    "`bytes_slice(self, start, end)` in `Bytes::slice`",
                ),
                Builtin::BytesConcat => (
                    (vec![Ty::Bytes, Ty::Bytes], Ty::Bytes),
                    "`bytes_concat(self, other)` in `Bytes::concat`",
                ),
                Builtin::BytesToInts => (
                    (vec![Ty::Bytes], Ty::Array(Box::new(Ty::Int))),
                    "`bytes_to_ints(self)` in `Bytes::to_ints`",
                ),
                Builtin::BytesFromInts => (
                    (vec![Ty::Array(Box::new(Ty::Int))], Ty::Bytes),
                    "`bytes_from_ints_intrinsic(ints)` in the free function `bytes_from_ints`",
                ),
                Builtin::BytesEq => (
                    (vec![Ty::Bytes, Ty::Bytes], Ty::Bool),
                    "`bytes_eq(self, other)` in `impl Eq for Bytes`",
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

    // ---- `@test` attribute validation and collection (Task 2) --------------
    //
    // Task 1 made `@name(args)` parse and be stored on items, deliberately
    // without validating them. These tests are the first thing to ever read
    // `Attribute` at all: an unknown name is `E0082`, `@test` on anything but
    // a function is `E0083`, a `@test` function with parameters, generics, or
    // a non-`Unit` return is `E0084`, and an unknown `@test(...)` argument is
    // `E0085`. A misspelled `@tset` must be a hard error rather than silently
    // parsed and ignored — that would compile a function that looks like a
    // test and never runs as one, which is the worst instance of this
    // project's most-repeated defect ("parses, then enforces nothing").

    #[test]
    fn an_unknown_attribute_is_e0082_and_names_it() {
        let r = check_src("@tset\nfn t() { }\nfn main() { }");
        let d = r
            .diagnostics
            .iter()
            .find(|d| d.code == "E0082")
            .expect("E0082 for an unknown attribute");
        assert!(
            d.message.contains("tset"),
            "names the attribute: {}",
            d.message
        );
        assert!(
            d.message.contains("test"),
            "lists the known set: {}",
            d.message
        );
    }

    #[test]
    fn test_on_a_non_function_is_e0083() {
        let r = check_src("@test\nrecord R { n: Int }\nfn main() { }");
        assert!(error_codes(&r).contains(&"E0083"), "{:?}", r.diagnostics);
    }

    #[test]
    fn test_on_a_function_with_parameters_is_e0084() {
        let r = check_src("@test\nfn t(x: Int) { }\nfn main() { }");
        let d = r
            .diagnostics
            .iter()
            .find(|d| d.code == "E0084")
            .expect("E0084 for a @test function with parameters");
        // "have parameters" (not just "parameters") so this can't pass
        // against a message that named "generic parameters" instead —
        // "generic parameters" contains "parameters" as a substring but not
        // "have parameters", since "have" is followed by "generic " there,
        // not directly by "parameters".
        assert!(
            d.message.contains("have parameters"),
            "names parameters specifically, not a generic \"bad signature\": {}",
            d.message
        );
    }

    #[test]
    fn test_on_a_function_returning_a_value_is_e0084() {
        let r = check_src("@test\nfn t() -> Int { 1 }\nfn main() { }");
        let d = r
            .diagnostics
            .iter()
            .find(|d| d.code == "E0084")
            .expect("E0084 for a @test function returning a value");
        assert!(
            d.message.contains("non-Unit return type"),
            "names the return type specifically, not a generic \"bad signature\": {}",
            d.message
        );
    }

    #[test]
    fn test_on_a_generic_function_is_e0084() {
        // A test takes no arguments, so nothing could ever fix its type
        // parameter — monomorphization would have no instance to emit.
        let r = check_src("@test\nfn t<T>() { }\nfn main() { }");
        let d = r
            .diagnostics
            .iter()
            .find(|d| d.code == "E0084")
            .expect("E0084 for a generic @test function");
        assert!(
            d.message.contains("generic parameters"),
            "names generic parameters specifically, not a generic \"bad signature\": {}",
            d.message
        );
    }

    /// An `async` `@test` is rejected, not run.
    ///
    /// The runner calls a `@test` function directly and discards its result, so
    /// an `async` one hands back a future that nothing polls: were the shape
    /// accepted, the call would allocate a state object, the body would not run,
    /// and a guaranteed-failing assertion inside it would report `ok` — the same
    /// failure `nova-mir`'s `mono.rs` describes for an `async` entry point.
    ///
    /// Rejected rather than shimmed because
    /// `docs/superpowers/specs/2026-08-07-phase-2-3a-async-core-design.md` §10
    /// specifies `@test fn t() { block_on(f()) }` as the way async code is
    /// tested, with no change to the runner; the defect was that `@test async
    /// fn` was silently accepted instead of refused. `nova-cli`'s
    /// `nova_test_runs_an_async_body_via_block_on_and_pins_a_wrong_answer`
    /// is the other half — that the supported shape really does run and really
    /// does fail on a wrong answer.
    #[test]
    fn test_on_an_async_function_is_e0084() {
        let r = check_src("@test\nasync fn t() { }\nfn main() { }");
        let d = r
            .diagnostics
            .iter()
            .find(|d| d.code == "E0084")
            .expect("E0084 for an async @test function");
        assert!(
            d.message.contains("an `async` body"),
            "names async specifically, not a generic \"bad signature\": {}",
            d.message
        );
        // The note has to be actionable on its own: a user who reads it must be
        // able to write the working form without opening the docs, so it is
        // required to name `block_on` *and* to spell the replacement as source.
        let notes = d.notes.join(" ");
        assert!(
            notes.contains("block_on"),
            "the note names the working alternative: {notes}"
        );
        assert!(
            notes.contains("@test fn t()"),
            "the note spells the replacement out as source: {notes}"
        );
    }

    #[test]
    fn an_unknown_test_argument_is_e0085() {
        let r = check_src("@test(shuold_panic)\nfn t() { }\nfn main() { }");
        let d = r
            .diagnostics
            .iter()
            .find(|d| d.code == "E0085")
            .expect("E0085 for an unknown @test argument");
        assert!(
            d.message.contains("shuold_panic"),
            "names the offending value: {}",
            d.message
        );
        assert!(
            d.message.contains("should_panic"),
            "names the accepted arg: {}",
            d.message
        );
    }

    #[test]
    fn a_well_formed_test_is_accepted() {
        let r = check_src("@test\nfn t() { }\n@test(should_panic)\nfn u() { }\nfn main() { }");
        assert_eq!(error_codes(&r), Vec::<&str>::new(), "{:?}", r.diagnostics);
    }

    #[test]
    fn collected_tests_are_in_source_order_with_should_panic_attached() {
        // The runner addresses tests BY INDEX across separate processes
        // (spec §4, risk 2). If collection order is not source order, or is
        // not stable between the enumeration run and a test run, `nova test`
        // silently runs one test and reports another's name. That failure is
        // invisible to every other test in this file, so it is pinned here.
        //
        // Three tests, not two: with two, a reversed order and a swapped
        // should_panic flag are indistinguishable. The middle one carries the
        // flag so a reversal moves it.
        let r = collect_tests_of(
            "@test\nfn alpha() { }\n\
             @test(should_panic)\nfn beta() { }\n\
             @test\nfn gamma() { }\n\
             fn main() { }",
        );
        let names: Vec<&str> = r.iter().map(|t| t.name.as_str()).collect();
        assert_eq!(names, ["alpha", "beta", "gamma"]);
        assert_eq!(
            r.iter().map(|t| t.should_panic).collect::<Vec<_>>(),
            [false, true, false]
        );
    }

    // ---- fix round 1: import/module/extern carry attrs too -----------------
    //
    // Escalated after independent measurement: `@tset` before a function was
    // E0082, but before an `import` it was masked by an unrelated E0001
    // ("cannot find module"), and before an `extern` block it produced NO
    // diagnostic at all — an unknown attribute compiled clean. The root cause
    // was in `nova-ast`/`nova-parser`, not just `nova-resolver`: `Import`,
    // `Module` and `ExternBlock` had no `attrs` field, so `try_parse_item`'s
    // freshly-parsed `Vec<Attribute>` had nowhere to go for those three item
    // kinds and was silently dropped before the resolver ever ran. Fixed by
    // giving all three an `attrs` field (matching the other six exactly in
    // placement and type) and extending `validate_attrs_reject_test` to walk
    // them, rather than special-casing the parser to reject `@` there — every
    // item may carry attributes; validation decides what is legal, uniformly.

    #[test]
    fn an_unknown_attribute_before_an_import_is_e0082_and_names_it() {
        let r = check_src("@tset\nimport core\nfn main() { }");
        let d = r
            .diagnostics
            .iter()
            .find(|d| d.code == "E0082")
            .expect("E0082 for an unknown attribute before an import");
        assert!(
            d.message.contains("tset"),
            "names the attribute: {}",
            d.message
        );
        assert!(
            d.message.contains("test"),
            "lists the known set: {}",
            d.message
        );
    }

    #[test]
    fn an_unknown_attribute_before_a_module_is_e0082_and_names_it() {
        let r = check_src("@tset\nmodule foo;\nfn main() { }");
        let d = r
            .diagnostics
            .iter()
            .find(|d| d.code == "E0082")
            .expect("E0082 for an unknown attribute before a module declaration");
        assert!(
            d.message.contains("tset"),
            "names the attribute: {}",
            d.message
        );
        assert!(
            d.message.contains("test"),
            "lists the known set: {}",
            d.message
        );
    }

    #[test]
    fn an_unknown_attribute_before_an_extern_block_is_e0082_and_names_it() {
        let r = check_src("@tset\nextern { }\nfn main() { }");
        let d = r
            .diagnostics
            .iter()
            .find(|d| d.code == "E0082")
            .expect("E0082 for an unknown attribute before an extern block");
        assert!(
            d.message.contains("tset"),
            "names the attribute: {}",
            d.message
        );
        assert!(
            d.message.contains("test"),
            "lists the known set: {}",
            d.message
        );
    }

    #[test]
    fn test_before_an_import_is_e0083() {
        let r = check_src("@test\nimport core\nfn main() { }");
        let d = r
            .diagnostics
            .iter()
            .find(|d| d.code == "E0083")
            .expect("E0083 for @test before an import");
        assert!(
            d.message.contains("import"),
            "names the item kind: {}",
            d.message
        );
    }

    #[test]
    fn test_before_a_module_is_e0083() {
        let r = check_src("@test\nmodule foo;\nfn main() { }");
        let d = r
            .diagnostics
            .iter()
            .find(|d| d.code == "E0083")
            .expect("E0083 for @test before a module declaration");
        assert!(
            d.message.contains("module"),
            "names the item kind: {}",
            d.message
        );
    }

    #[test]
    fn test_before_an_extern_block_is_e0083() {
        let r = check_src("@test\nextern { }\nfn main() { }");
        let d = r
            .diagnostics
            .iter()
            .find(|d| d.code == "E0083")
            .expect("E0083 for @test before an extern block");
        assert!(
            d.message.contains("extern"),
            "names the item kind: {}",
            d.message
        );
    }

    // ---- fix round 2: a function with two @test attributes is one test ----
    //
    // `@test` twice on one function is syntactically legal (the parser puts
    // no uniqueness constraint on attribute names — see
    // `multiple_arguments_and_multiple_attributes_parse` in nova-parser for
    // proof a function can carry multiple attributes at all). Without
    // deduplication in `validate_test_function`, each well-formed `@test`
    // independently pushes a `TestFn`, so the same def_id/name would be
    // collected twice and `nova test` would list and run one function as two
    // different tests.

    #[test]
    fn a_function_with_two_test_attributes_is_collected_once() {
        let r = collect_tests_of("@test\n@test\nfn t() { }\nfn main() { }");
        assert_eq!(r.len(), 1, "{:?}", r);
        assert_eq!(r[0].name, "t");
    }

    // ---- reserved built-in type names ----

    #[test]
    fn reserved_type_names_is_exactly_the_seven_expected_names() {
        // Both tests below iterate `RESERVED_TYPE_NAMES` to build their own
        // cases, so neither one can notice a name silently missing from it --
        // they would just stop checking that name, not fail. This list is
        // written out independently of the constant it checks against, so a
        // name dropped (or swapped for a duplicate of another, keeping the
        // length unchanged) shows up as a content mismatch here, and a name
        // dropped without a length change fails to even compile, since two
        // differently-sized arrays are different types.
        let mut expected = ["Int", "Float", "Bool", "Char", "String", "Future", "Bytes"];
        let mut actual = nova_resolver::RESERVED_TYPE_NAMES;
        expected.sort_unstable();
        actual.sort_unstable();
        assert_eq!(actual, expected);
    }

    #[test]
    fn a_rejected_declarations_own_body_is_never_checked() {
        // Pins skip-not-register for BOTH declaration forms:
        // `reject_reserved_type_name` returns before `push_def`, so the
        // rejected item never enters `defs` and is never visited by
        // `collect_records`/`collect_sums`/`collect_type_arities` (which
        // iterate `defs()` directly, independent of the name lookup that
        // rejected it). An ordinary field or payload type (`v: Int`, or a
        // variant with no payload) can't discriminate this -- it would
        // convert cleanly whether or not the item were processed. One naming
        // an unresolvable type can: if a future change made rejection
        // register the def anyway, `collect_records`/`collect_sums` would
        // convert it and add a second diagnostic for `Nope` alongside
        // `E0089`. `check_src` runs the type checker unconditionally (unlike
        // the driver, which stops after a resolver error), so this is the
        // layer where such a regression would actually surface. The record
        // and sum arms are two separate call sites in `collect_item`
        // (`reject_reserved_type_name` is called independently from each),
        // so one passing is no evidence about the other -- both are checked.
        for src in [
            "record Bool { v: Nope }\nfn main() { }",
            "type Bool = | A(Nope) | B\nfn main() { }",
        ] {
            let r = check_src(src);
            assert_eq!(
                r.diagnostics.len(),
                1,
                "expected only E0089 for {src:?}, got {:?}",
                r.diagnostics
                    .iter()
                    .map(|d| (&d.code, &d.message))
                    .collect::<Vec<_>>()
            );
            assert_eq!(r.diagnostics[0].code, "E0089");
        }
    }

    #[test]
    fn two_declarations_of_the_same_reserved_name_each_raise_e0089_not_e0002() {
        // Pins a claim `reject_reserved_type_name`'s own doc makes but that
        // nothing exercised: skip-not-register means a second declaration
        // under an already-rejected reserved name is not a *duplicate* --
        // `insert_type`, the `E0002` source two same-file type declarations
        // under one name would otherwise reach, is never reached for either
        // one here, since both return from `reject_reserved_type_name`
        // before `push_def`. So this is two independent `E0089`s, not an
        // `E0089` followed by an `E0002`. A refactor that reordered the check
        // after `push_def`/`insert_type` would turn the second occurrence
        // into `E0002` silently; this fails if it does.
        let r = check_src("record Bool { v: Int }\nrecord Bool { w: Int }\nfn main() { }");
        let codes: Vec<&str> = r.diagnostics.iter().map(|d| d.code.as_str()).collect();
        assert_eq!(
            codes,
            ["E0089", "E0089"],
            "got {:?}",
            r.diagnostics
                .iter()
                .map(|d| d.message.clone())
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn declaring_a_type_named_for_a_builtin_is_rejected() {
        // All six names, both declaration forms. `convert_ty` resolves the name
        // to the built-in, or to a shadowing generic parameter, wherever it
        // runs -- never to a declaration under this name -- so a type declared
        // under one of these names could never be referred to in a type
        // annotation. Rejecting the declaration puts that fact where the user
        // can act on it, rather than only once it is named in a signature.
        //
        // This also forecloses construction and pattern matching, which used to
        // work (a record literal resolves through `resolve_type` directly; a
        // sum type's variants live in the value namespace, both independent of
        // `convert_ty`) -- a real, accepted breaking change for that narrower
        // usage. An `impl` header's self type is a `convert_ty` site too. See
        // CHANGELOG.md for exactly what breaks, and
        // `a_rejected_declarations_own_body_is_never_checked` for the
        // no-cascade guarantee this rejection still provides.
        for name in nova_resolver::RESERVED_TYPE_NAMES {
            for src in [
                format!("record {name} {{ v: Bool }}\nfn main() {{ }}"),
                format!("type {name} = | A | B\nfn main() {{ }}"),
            ] {
                let r = check_src(&src);
                // Exactly one diagnostic, not merely "E0089 is present somewhere":
                // skip-not-register means none of these twelve declarations should
                // ever cascade, and `.find()` alone would not notice if one did.
                assert_eq!(
                    r.diagnostics.len(),
                    1,
                    "expected only E0089 for `{name}` in {src:?}, got {:?}",
                    r.diagnostics
                        .iter()
                        .map(|d| (&d.code, &d.message))
                        .collect::<Vec<_>>()
                );
                let d = &r.diagnostics[0];
                assert_eq!(d.code, "E0089", "for `{name}` in {src:?}: {:?}", d.message);
                // Both halves of the message matter. The name identifies which
                // built-in was shadowed; the second half is the fact a user cannot
                // discover from the declaration alone, and a code-only assertion
                // would survive deleting it.
                assert!(
                    d.message.contains(name),
                    "E0089 must name the built-in it collides with; got {:?}",
                    d.message
                );
                assert!(
                    d.message.contains("built-in") || d.message.contains("builtin"),
                    "E0089 must say the name belongs to a built-in type; got {:?}",
                    d.message
                );
            }
        }
    }

    #[test]
    fn every_reserved_name_really_is_a_builtin_type() {
        // The drift guard, in the direction that can be caught. If a name is
        // removed from `convert_ty`'s table while staying in the reserved list,
        // this fails -- annotating with it would become `E0001 cannot find type`.
        //
        // The other direction (a seventh built-in added without reserving it) is
        // NOT caught here and cannot be from a fixed list; the mitigation is the
        // pointer comment at the table itself (Step 4).
        for name in nova_resolver::RESERVED_TYPE_NAMES {
            let ann = if name == "Future" {
                "Future<Int>"
            } else {
                name
            };
            let r = check_src(&format!("fn f(x: {ann}) -> Int {{ 1 }}\nfn main() {{ }}"));
            assert!(
                !r.diagnostics.iter().any(|d| d.code == "E0001"),
                "`{name}` is in RESERVED_TYPE_NAMES but is not a built-in type name: {:?}",
                r.diagnostics
                    .iter()
                    .map(|d| d.message.clone())
                    .collect::<Vec<_>>()
            );
        }
    }

    #[test]
    fn bytes_is_a_nameable_type_in_every_position() {
        // `Bytes` has no operations yet, so this only asserts it converts in a
        // signature, a `let` annotation, a field type and a generic argument --
        // every position `convert_ty` runs. A value cannot be constructed
        // until Task 2, which is why `main`'s `let` initializes with
        // `panic(...)` rather than a real `Bytes` value: `panic` diverges
        // (`Ty::Never`), and `Never` unifies with anything, so the annotation
        // still converts and unifies without needing a way to construct one.
        let r = check_src(
            "record Holder { b: Bytes }\n\
             fn ident(x: Bytes) -> Bytes { x }\n\
             fn takes_array(xs: [Bytes]) -> Int { xs.len() }\n\
             fn main() { let x: Bytes = panic(\"unreachable\") }",
        );
        assert!(
            r.diagnostics.is_empty(),
            "`Bytes` must convert in every type position, got {:?}",
            r.diagnostics
                .iter()
                .map(|d| (&d.code, &d.message))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn a_generic_parameter_named_for_a_builtin_still_works() {
        // NON-GOAL, pinned. `convert_ty` resolves generics BEFORE the built-in
        // table, so this shadowing is coherent rather than broken: the parameter
        // genuinely means the parameter. Compiling is not enough to show that --
        // an `Int` argument round-trips whether `x: Int` names the parameter or
        // the primitive it shadows, since both are the same value either way.
        // This crate has no evaluator, only the type checker, so the
        // discriminating half of the claim -- that the annotation names a
        // parameter free to become a *different* type, not just that it
        // compiles for one -- is pinned at the runtime layer instead: see
        // `shadow_builtin` in `tests/runtime/generics.nova`, called at both
        // `Int` and `String`. If `Int` had instead fallen through to the
        // primitive, the `String` call would be a compile-time type error,
        // not a different runtime value.
        let r = check_src("fn f<Int>(x: Int) -> Int { x }\nfn main() { }");
        assert!(
            r.diagnostics.is_empty(),
            "a generic parameter may shadow a built-in type name, got {:?}",
            r.diagnostics
                .iter()
                .map(|d| d.message.clone())
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn a_trait_named_for_a_builtin_still_works() {
        // NON-GOAL, pinned. Traits are a separate namespace: `trait Int` does not
        // shadow the type, and the return annotation below resolves to the
        // primitive.
        let r = check_src("trait Int { fn m(self) -> Int }\nfn main() { }");
        assert!(
            r.diagnostics.is_empty(),
            "a trait may be named for a built-in type, got {:?}",
            r.diagnostics
                .iter()
                .map(|d| d.message.clone())
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn a_value_named_for_a_builtin_still_works() {
        // NON-GOAL, pinned. `reject_reserved_type_name` runs only from the
        // sum-type and record arms of `collect_item`, never from `insert_value`
        // -- a function, a const, and a local binding all live in the value
        // namespace, which this check does not touch. Nothing before this
        // stopped `nova-resolver` from being extended to reject these too, and
        // this is the test that would fail if a later tidy-up quietly widened
        // the rule that way.
        let r = check_src(
            "fn Int() -> Int { 1 }\nconst String: Int = 2\n\
             fn main() { let Bool = 3\nprintln(\"${Int()} ${String} ${Bool}\") }",
        );
        assert!(
            r.diagnostics.is_empty(),
            "a function, const, or local binding may be named for a built-in type, got {:?}",
            r.diagnostics
                .iter()
                .map(|d| d.message.clone())
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn an_unknown_type_name_is_still_e0001_not_e0089() {
        // The new check must not swallow the not-found path.
        let r = check_src("fn f(x: Nope) -> Int { 1 }\nfn main() { }");
        assert!(r.diagnostics.iter().any(|d| d.code == "E0001"));
        assert!(!r.diagnostics.iter().any(|d| d.code == "E0089"));
    }

    #[test]
    fn an_ordinary_type_declaration_is_unaffected() {
        // Kills a check that fires on every type name rather than the reserved
        // ones. Weak on its own -- the whole suite would fail -- but it states the
        // boundary at the point the reader is looking at.
        let r = check_src("record Wrap { v: Int }\ntype Two = | A | B\nfn main() { }");
        assert!(!r.diagnostics.iter().any(|d| d.code == "E0089"));
    }
}
