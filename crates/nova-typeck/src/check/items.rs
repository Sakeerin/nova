//! The item stage: passes 1 to 5 of [`check_with`](super::check_with)
//! (spec
//! `docs/superpowers/specs/2026-11-06-phase-5-3-the-type-checker-and-hir-design.md`
//! §3.1, ADR 0055), as [`check_items`], and std's item tables built once
//! and shared by many programs ([`SharedItems`], §3.4), for the hidden HIR
//! dump alone.
//!
//! Pass 1 is the language server's index (`note_item_declarations`),
//! which records nothing unless asked; passes 2 to 5 collect the generic
//! parameters' checks, the arities, supertraits, records, sums, traits,
//! impls with their checks, signatures and externs. [`check_items`] runs all
//! five, so that [`check_with`](super::check_with) calls it first and its
//! output is as it was; the self-hosted type checker ports passes 2 to 5.

use super::*;

/// What passes 1 to 5 build: the item tables, the item diagnostics in
/// the order the passes emitted them, and the state the bodies start from.
pub struct ItemTables {
    pub(crate) sigs: FxHashMap<DefId, FnSig>,
    pub(crate) method_locs: FxHashMap<DefId, MethodLoc>,
    pub(crate) selfless: FxHashSet<DefId>,
    pub(crate) mut_self: FxHashSet<DefId>,
    pub(crate) sums: Vec<hir::SumType>,
    pub(crate) records: Vec<hir::RecordType>,
    pub(crate) supertraits: FxHashMap<DefId, Vec<DefId>>,
    pub(crate) traits: Vec<hir::TraitDef>,
    pub(crate) impls: Vec<hir::ImplInfo>,
    pub(crate) impl_param_names: Vec<Vec<String>>,
    pub(crate) impl_selves: FxHashMap<usize, ImplSelf>,
    pub(crate) type_arity: FxHashMap<DefId, u32>,
    pub(crate) externs: Vec<hir::ExternFn>,
    /// Each impl's self type's span, whether it is std's, and its methods'
    /// names' spans, by the index of `impls`: what the impl checks read.
    pub(crate) impl_spans: Vec<Span>,
    pub(crate) impl_in_std: Vec<bool>,
    pub(crate) impl_method_spans: Vec<Vec<Span>>,
    /// The item diagnostics, in the order the passes emitted them.
    pub diagnostics: Vec<Diagnostic>,
    index: Option<Index>,
    cur_module: ModuleId,
    type_params: Vec<(String, Span)>,
    impl_self: Option<ImplSelf>,
}

/// Passes 1 to 5 of [`check_with`](super::check_with) over `file`: the
/// item tables and the item diagnostics, which are the prefix of
/// `check_with`'s diagnostics (5.3a).
pub fn check_items(file: &ast::File, defs: &Definitions, options: &CheckOptions) -> ItemTables {
    let mut checker = Checker::new(file, defs, options, None);
    checker.collect_items();
    checker.into_tables()
}

/// std's item tables, built once from the check of a program of no module
/// of its own, which each program's [`check_items_shared`] reads with std's
/// ids rebased past its own (spec 5.3 §3.4): std's items are not collected
/// again. For the hidden HIR dump alone.
pub struct SharedItems {
    tables: ItemTables,
    /// std's definitions and items, numbered from 0.
    defs: u32,
    /// Whether std's items gave no diagnostic: one would come between a
    /// program's, which the shared path drops, so a program is then
    /// checked in full.
    shareable: bool,
}

impl SharedItems {
    /// std's item tables, from `file` and `defs`, the resolution of a
    /// program of no module of its own. std's bodies are checked once
    /// too (5.3b, spec 5.3 §4.5): should they give a diagnostic, as its
    /// items, nothing is shared.
    pub fn new(file: &ast::File, defs: &Definitions, options: &CheckOptions) -> SharedItems {
        let tables = check_items(file, defs, options);
        let bodies = check_program(file, defs, options);
        SharedItems {
            shareable: tables.diagnostics.is_empty() && bodies.diagnostics.is_empty(),
            defs: defs.defs().len() as u32,
            tables,
        }
    }

    /// Whether a program may read these tables ([`SharedItems::shareable`]'s
    /// field): std's items and bodies gave no diagnostic.
    pub fn shareable(&self) -> bool {
        self.shareable
    }

    /// std's tables, as std's part of the dump prints them.
    pub fn tables(&self) -> &ItemTables {
        &self.tables
    }
}

/// [`check_items`] reading std's tables from `shared`: `file` holds the
/// program's own items only, and `defs` its definitions followed by std's,
/// rebased (`nova_resolver::resolve_program_with_shared_std_for_check`).
/// Each pass runs over the program's items, and std's part of each table is
/// added where the full check would have collected it, after the program's;
/// the impl checks then pair the program's impls with std's as the full
/// check does. The caller checks in full when `shared` is not shareable.
pub fn check_items_shared(
    file: &ast::File,
    defs: &Definitions,
    options: &CheckOptions,
    shared: &SharedItems,
) -> ItemTables {
    let mut checker = shared_checker(file, defs, options, shared);
    checker.collect_items();
    checker.into_tables()
}

/// A checker over the program `file` and `defs` that reads std's tables
/// from `shared` ([`check_items_shared`]).
fn shared_checker<'a>(
    file: &'a ast::File,
    defs: &'a Definitions,
    options: &CheckOptions<'a>,
    shared: &'a SharedItems,
) -> Checker<'a> {
    let own_defs = defs.defs().len() as u32 - shared.defs;
    let rebase = Rebase {
        def: own_defs,
        item: file.items.len(),
    };
    let mut checker = Checker::new(file, defs, options, Some((shared, rebase)));
    checker.own_defs = own_defs;
    checker
}

/// What the whole type checker built for a program, for the HIR dump
/// (5.3b, spec 5.3 §4.3, ADR 0056): the item tables, the functions, and
/// every diagnostic it gave, the item passes' then the bodies'.
pub struct Checked {
    /// The item tables, whose `diagnostics` are the item passes' alone.
    pub tables: ItemTables,
    /// Every diagnostic the type checker gave, in order.
    pub diagnostics: Vec<Diagnostic>,
    /// Every function as `hir::Module` holds them: the program's, then
    /// std's when they were checked, then the closures and wrappers in the
    /// order they were lifted, the program's before std's; `None` when
    /// only the items were checked ([`Checked::items`]).
    pub(crate) functions: Option<Vec<hir::Function>>,
    /// The first closure's or wrapper's id, past every definition
    /// (`defs.len()`), and the first of std's.
    pub(crate) closures: u32,
    pub(crate) std_closures: u32,
}

impl Checked {
    /// The item passes' output alone ([`check_items`],
    /// [`check_items_shared`]), as 5.3a's dump prints it.
    pub fn items(tables: ItemTables) -> Checked {
        Checked {
            diagnostics: tables.diagnostics.clone(),
            tables,
            functions: None,
            closures: u32::MAX,
            std_closures: u32::MAX,
        }
    }

    /// The functions, when the bodies were checked.
    pub fn functions(&self) -> Option<&[hir::Function]> {
        self.functions.as_deref()
    }
}

/// [`check_with`](super::check_with) over `file`, with the item tables kept
/// beside the functions, for the HIR dump (5.3b): the program's bodies,
/// then std's, each in `DefId` order (spec 5.3 §4.1).
pub fn check_program(file: &ast::File, defs: &Definitions, options: &CheckOptions) -> Checked {
    let tables = check_items(file, defs, options);
    let checker = Checker::new(file, defs, options, None);
    checker.checked(tables)
}

/// [`check_program`] reading std's tables from `shared`
/// ([`check_items_shared`]): the program's bodies alone, since std's are
/// std's part of the dump, checked once by [`SharedItems::new`] (5.3b,
/// spec 5.3 §4.5). `None` when the program implements anything on a type
/// it does not own ([`implements_on_a_type_it_does_not_own`]): method
/// lookup scans every impl, so std's bodies could then check otherwise,
/// and the caller checks the program in full. The caller checks in full
/// when `shared` is not shareable, too.
pub fn check_program_shared(
    file: &ast::File,
    defs: &Definitions,
    options: &CheckOptions,
    shared: &SharedItems,
) -> Option<Checked> {
    let tables = check_items_shared(file, defs, options, shared);
    let own_defs = defs.defs().len() as u32 - shared.defs;
    if implements_on_a_type_it_does_not_own(file, &tables, own_defs) {
        return None;
    }
    Some(shared_checker(file, defs, options, shared).checked(tables))
}

/// Whether the program implements anything on a type it does not own
/// (5.3b, spec 5.3 §4.5; the user's ruling of 2026-11-12): an impl, a
/// trait's or an inherent one, whose self type is a primitive, an array
/// or one of std's records or sums, rather than one of the program's own,
/// those below `own_defs`. std's bodies look methods up in every impl,
/// with no filter by the traits in scope, so such an impl can give std's
/// bodies a diagnostic or bind one of their calls to the program's
/// method. To err on the safe side, an impl of the program's that the item
/// passes did not collect counts too: `file` holds the program's items
/// alone.
pub fn implements_on_a_type_it_does_not_own(
    file: &ast::File,
    tables: &ItemTables,
    own_defs: u32,
) -> bool {
    let written = file
        .items
        .iter()
        .filter(|item| matches!(item.value, ast::Item::Impl(_)))
        .count();
    let mut collected = 0;
    for (imp, &std) in tables.impls.iter().zip(&tables.impl_in_std) {
        if std {
            continue;
        }
        collected += 1;
        match imp.self_head {
            TyHead::Sum(id) | TyHead::Record(id) if id.0 < own_defs => {}
            _ => return true,
        }
    }
    collected != written
}

/// How std's ids move in a program: its definitions come after the
/// program's `def`, its items after the program's `item`.
#[derive(Clone, Copy)]
pub(crate) struct Rebase {
    def: u32,
    item: usize,
}

impl Rebase {
    fn id(self, id: DefId) -> DefId {
        DefId(id.0 + self.def)
    }

    fn ids(self, ids: &[DefId]) -> Vec<DefId> {
        ids.iter().map(|&id| self.id(id)).collect()
    }

    fn bounds(self, bounds: &[Vec<DefId>]) -> Vec<Vec<DefId>> {
        bounds.iter().map(|b| self.ids(b)).collect()
    }

    fn tys(self, tys: &[Ty]) -> Vec<Ty> {
        tys.iter().map(|t| self.ty(t)).collect()
    }

    fn ty(self, ty: &Ty) -> Ty {
        match ty {
            Ty::Fn { params, ret } => Ty::Fn {
                params: self.tys(params),
                ret: Box::new(self.ty(ret)),
            },
            Ty::Future(out) => Ty::Future(Box::new(self.ty(out))),
            Ty::Sum { def_id, args } => Ty::Sum {
                def_id: self.id(*def_id),
                args: self.tys(args),
            },
            Ty::Record { def_id, args } => Ty::Record {
                def_id: self.id(*def_id),
                args: self.tys(args),
            },
            Ty::Array(elem) => Ty::Array(Box::new(self.ty(elem))),
            Ty::Tuple(elems) => Ty::Tuple(self.tys(elems)),
            Ty::Assoc { on, assoc } => Ty::Assoc {
                on: Box::new(self.ty(on)),
                assoc: self.id(*assoc),
            },
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
            | Ty::Error => ty.clone(),
        }
    }

    fn head(self, head: TyHead) -> TyHead {
        match head {
            TyHead::Sum(id) => TyHead::Sum(self.id(id)),
            TyHead::Record(id) => TyHead::Record(self.id(id)),
            other => other,
        }
    }

    fn sig(self, sig: &FnSig) -> FnSig {
        FnSig {
            generics: sig.generics,
            bounds: self.bounds(&sig.bounds),
            params: self.tys(&sig.params),
            ret: self.ty(&sig.ret),
        }
    }

    fn imp(self, imp: &hir::ImplInfo) -> hir::ImplInfo {
        hir::ImplInfo {
            trait_id: imp.trait_id.map(|t| self.id(t)),
            self_head: self.head(imp.self_head),
            self_ty: self.ty(&imp.self_ty),
            generics: imp.generics,
            bounds: self.bounds(&imp.bounds),
            methods: imp
                .methods
                .iter()
                .map(|(n, d)| (n.clone(), self.id(*d)))
                .collect(),
            assoc_bindings: imp
                .assoc_bindings
                .iter()
                .map(|(d, t)| (self.id(*d), self.ty(t)))
                .collect(),
        }
    }

    fn trait_def(self, t: &hir::TraitDef) -> hir::TraitDef {
        hir::TraitDef {
            def_id: self.id(t.def_id),
            name: t.name.clone(),
            supertraits: self.ids(&t.supertraits),
            methods: t
                .methods
                .iter()
                .map(|m| hir::TraitMethod {
                    name: m.name.clone(),
                    params: self.tys(&m.params),
                    ret: self.ty(&m.ret),
                    has_self: m.has_self,
                    mut_self: m.mut_self,
                    generics: m.generics,
                    bounds: self.bounds(&m.bounds),
                    default_def: m.default_def.map(|d| self.id(d)),
                })
                .collect(),
            assoc_types: t
                .assoc_types
                .iter()
                .map(|(n, d)| (n.clone(), self.id(*d)))
                .collect(),
        }
    }

    fn impl_self(self, s: &ImplSelf) -> ImplSelf {
        ImplSelf {
            ty: self.ty(&s.ty),
            trait_id: s.trait_id.map(|t| self.id(t)),
        }
    }
}

/// Where in the passes std's part of a table is added: where the full
/// check would have collected std's items, at the end of a pass's loop.
#[derive(Clone, Copy)]
pub(super) enum Splice {
    Arities,
    Supertraits,
    Records,
    Sums,
    Traits,
    TraitDefaults,
    Impls,
    Signatures,
    Externs,
}

impl<'a> Checker<'a> {
    /// A checker over `file` and `defs` with empty tables, reading std's
    /// from `shared` when given.
    pub(super) fn new(
        file: &'a ast::File,
        defs: &'a Definitions,
        options: &CheckOptions<'a>,
        shared: Option<(&'a SharedItems, Rebase)>,
    ) -> Checker<'a> {
        Checker {
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
            impl_param_names: Vec::new(),
            impl_self: None,
            impl_selves: FxHashMap::default(),
            extra_functions: Vec::new(),
            next_closure_def: defs.defs().len() as u32,
            type_arity: FxHashMap::default(),
            externs: Vec::new(),
            diagnostics: Vec::new(),
            probe: options.probe,
            probe_result: ProbeResult::default(),
            probe_pending: None,
            probe_locals_pending: None,
            index: options.index.then(Index::default),
            type_params: Vec::new(),
            sources: options.sources,
            bindings: FxHashMap::default(),
            qualified_callee: None,
            pats: PatState::default(),
            quiet_splits: FxHashSet::default(),
            impl_spans: Vec::new(),
            impl_in_std: Vec::new(),
            impl_method_spans: Vec::new(),
            own_defs: defs.defs().len() as u32,
            shared,
        }
    }

    /// Passes 1 to 5.
    fn collect_items(&mut self) {
        // Before any collection pass: every later pass builds a generic scope, and
        // one containing `Self` would give the name two meanings at once.
        self.note_item_declarations();
        self.reject_self_type_params();
        self.collect_type_arities();
        self.splice(Splice::Arities);
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
        self.collect_supertraits();
        self.splice(Splice::Supertraits);
        self.collect_records();
        self.splice(Splice::Records);
        self.collect_sums();
        self.splice(Splice::Sums);
        self.collect_traits();
        self.collect_impls();
        self.collect_signatures();
        self.splice(Splice::Signatures);
        self.collect_externs();
        self.splice(Splice::Externs);
    }

    /// The tables, taken out of the checker.
    fn into_tables(self) -> ItemTables {
        ItemTables {
            sigs: self.sigs,
            method_locs: self.method_locs,
            selfless: self.selfless,
            mut_self: self.mut_self,
            sums: self.sums,
            records: self.records,
            supertraits: self.supertraits,
            traits: self.traits,
            impls: self.impls,
            impl_param_names: self.impl_param_names,
            impl_selves: self.impl_selves,
            type_arity: self.type_arity,
            externs: self.externs,
            impl_spans: self.impl_spans,
            impl_in_std: self.impl_in_std,
            impl_method_spans: self.impl_method_spans,
            diagnostics: self.diagnostics,
            index: self.index,
            cur_module: self.cur_module,
            type_params: self.type_params,
            impl_self: self.impl_self,
        }
    }

    /// The checker as [`Checker::into_tables`] left it, for the bodies.
    pub(super) fn restore(&mut self, tables: ItemTables) {
        self.sigs = tables.sigs;
        self.method_locs = tables.method_locs;
        self.selfless = tables.selfless;
        self.mut_self = tables.mut_self;
        self.sums = tables.sums;
        self.records = tables.records;
        self.supertraits = tables.supertraits;
        self.traits = tables.traits;
        self.impls = tables.impls;
        self.impl_param_names = tables.impl_param_names;
        self.impl_selves = tables.impl_selves;
        self.type_arity = tables.type_arity;
        self.externs = tables.externs;
        self.impl_spans = tables.impl_spans;
        self.impl_in_std = tables.impl_in_std;
        self.impl_method_spans = tables.impl_method_spans;
        self.diagnostics = tables.diagnostics;
        self.index = tables.index;
        self.cur_module = tables.cur_module;
        self.type_params = tables.type_params;
        self.impl_self = tables.impl_self;
    }

    /// This new checker restored from `tables`, as
    /// [`check_with`](super::check_with) restores it, after the bodies
    /// ([`Checker::check_bodies`]): the tables, the functions and every
    /// diagnostic.
    fn checked(mut self, tables: ItemTables) -> Checked {
        let items = tables.diagnostics.len();
        let closures = self.next_closure_def;
        self.restore(tables);
        let (functions, std_closures) = self.check_bodies();
        let mut tables = self.into_tables();
        let diagnostics = std::mem::take(&mut tables.diagnostics);
        tables.diagnostics = diagnostics[..items].to_vec();
        Checked {
            tables,
            diagnostics,
            functions: Some(functions),
            closures,
            std_closures,
        }
    }

    /// Whether the definition `id` is the program's own, which the passes
    /// collect: every one, unless std's tables are shared.
    pub(super) fn own(&self, id: DefId) -> bool {
        id.0 < self.own_defs
    }

    /// Add std's part of the tables `at` fills, rebased, when they are
    /// shared; nothing otherwise.
    pub(super) fn splice(&mut self, at: Splice) {
        let Some((shared, r)) = self.shared else {
            return;
        };
        let std = &shared.tables;
        match at {
            Splice::Arities => {
                for id in sorted(&std.type_arity) {
                    self.type_arity.insert(r.id(id), std.type_arity[&id]);
                }
            }
            Splice::Supertraits => {
                for id in sorted(&std.supertraits) {
                    self.supertraits
                        .insert(r.id(id), r.ids(&std.supertraits[&id]));
                }
            }
            Splice::Records => {
                self.records.extend(std.records.iter().map(|rec| {
                    hir::RecordType {
                        def_id: r.id(rec.def_id),
                        name: rec.name.clone(),
                        generics: rec.generics,
                        fields: rec
                            .fields
                            .iter()
                            .map(|f| hir::RecordField {
                                name: f.name.clone(),
                                ty: r.ty(&f.ty),
                            })
                            .collect(),
                    }
                }));
            }
            Splice::Sums => {
                self.sums.extend(std.sums.iter().map(|s| {
                    hir::SumType {
                        def_id: r.id(s.def_id),
                        name: s.name.clone(),
                        generics: s.generics,
                        variants: s
                            .variants
                            .iter()
                            .map(|v| hir::Variant {
                                name: v.name.clone(),
                                fields: r.tys(&v.fields),
                            })
                            .collect(),
                    }
                }));
            }
            Splice::Traits => {
                self.traits
                    .extend(std.traits.iter().map(|t| r.trait_def(t)));
            }
            Splice::TraitDefaults => self.splice_methods(MethodOwner::TraitDefault),
            Splice::Impls => {
                self.splice_methods(MethodOwner::Impl);
                self.impls.extend(std.impls.iter().map(|i| r.imp(i)));
                self.impl_spans.extend(std.impl_spans.iter().copied());
                self.impl_in_std.extend(std.impl_in_std.iter().copied());
                self.impl_method_spans
                    .extend(std.impl_method_spans.iter().cloned());
                self.impl_param_names
                    .extend(std.impl_param_names.iter().cloned());
                for item in sorted(&std.impl_selves) {
                    let s = r.impl_self(&std.impl_selves[&item]);
                    self.impl_selves.insert(item + r.item, s);
                }
            }
            Splice::Signatures => {
                for id in sorted(&std.sigs) {
                    if matches!(
                        self.defs.def(r.id(id)).kind,
                        DefKind::Fn { .. } | DefKind::Const { .. }
                    ) {
                        self.sigs.insert(r.id(id), r.sig(&std.sigs[&id]));
                    }
                }
            }
            Splice::Externs => {
                for id in sorted(&std.sigs) {
                    if matches!(self.defs.def(r.id(id)).kind, DefKind::ExternFn { .. }) {
                        self.sigs.insert(r.id(id), r.sig(&std.sigs[&id]));
                    }
                }
                self.externs
                    .extend(std.externs.iter().map(|e| hir::ExternFn {
                        def_id: r.id(e.def_id),
                        symbol: e.symbol.clone(),
                        abi: e.abi,
                        params: r.tys(&e.params),
                        ret: r.ty(&e.ret),
                        span: e.span,
                    }));
            }
        }
    }

    /// std's methods owned by `owner`: their signatures, places and
    /// receivers.
    fn splice_methods(&mut self, owner: MethodOwner) {
        let Some((shared, r)) = self.shared else {
            return;
        };
        let std = &shared.tables;
        for id in sorted(&std.method_locs) {
            let loc = std.method_locs[&id];
            if loc.owner != owner {
                continue;
            }
            let at = r.id(id);
            if let Some(sig) = std.sigs.get(&id) {
                self.sigs.insert(at, r.sig(sig));
            }
            self.method_locs.insert(
                at,
                MethodLoc {
                    item_index: loc.item_index + r.item,
                    ..loc
                },
            );
            if std.selfless.contains(&id) {
                self.selfless.insert(at);
            }
            if std.mut_self.contains(&id) {
                self.mut_self.insert(at);
            }
        }
    }
}

/// A map's keys sorted, so that no map's order reaches a table.
fn sorted<K: Ord + Copy, V>(map: &FxHashMap<K, V>) -> Vec<K> {
    let mut keys: Vec<K> = map.keys().copied().collect();
    keys.sort();
    keys
}

#[cfg(test)]
mod tests {
    use super::*;
    use nova_diagnostics::FileId;
    use nova_resolver::{ModuleSource, ParsedStd, ProgramResolution, SharedStd};

    fn parse(src: &str) -> ast::File {
        let (tokens, _) = nova_lexer::lex(src, FileId::DUMMY);
        let parsed = nova_parser::parse_recovering(&tokens, FileId::DUMMY);
        assert!(parsed.errors.is_empty(), "{:?}", parsed.errors);
        parsed.file
    }

    /// std for the native target, `extra` (a module's name and text) after
    /// its modules.
    fn std_with(extra: Option<(&'static str, &'static str)>) -> ParsedStd {
        let ids: Vec<FileId> = CompileTarget::Native
            .std_modules()
            .iter()
            .map(|_| FileId::DUMMY)
            .collect();
        nova_resolver::parse_std(
            CompileTarget::Native,
            &ids,
            extra.map(|e| (e, FileId::DUMMY)),
        )
    }

    /// The text of a program's part, from its resolution and what the
    /// type checker built.
    fn shown(r: &ProgramResolution, t: &Checked) -> String {
        let labels = [(FileId::DUMMY, "main".to_string())];
        let mut diagnostics = r.diagnostics.clone();
        diagnostics.extend(t.diagnostics.iter().cloned());
        crate::dump::dump_program(
            CompileTarget::Native,
            false,
            &labels,
            Some((r, t)),
            &diagnostics,
        )
    }

    /// Every table of `t`, a map's or a set's entries sorted by key.
    fn tables_text(t: &ItemTables) -> String {
        fn set(s: &FxHashSet<DefId>) -> Vec<DefId> {
            let mut v: Vec<DefId> = s.iter().copied().collect();
            v.sort();
            v
        }
        let sigs: Vec<_> = sorted(&t.sigs)
            .into_iter()
            .map(|k| (k, &t.sigs[&k]))
            .collect();
        let locs: Vec<_> = sorted(&t.method_locs)
            .into_iter()
            .map(|k| (k, t.method_locs[&k]))
            .collect();
        let supers: Vec<_> = sorted(&t.supertraits)
            .into_iter()
            .map(|k| (k, &t.supertraits[&k]))
            .collect();
        let selves: Vec<_> = sorted(&t.impl_selves)
            .into_iter()
            .map(|k| (k, &t.impl_selves[&k]))
            .collect();
        let arity: Vec<_> = sorted(&t.type_arity)
            .into_iter()
            .map(|k| (k, t.type_arity[&k]))
            .collect();
        format!(
            "{sigs:?}\n{locs:?}\n{:?}\n{:?}\n{:?}\n{:?}\n{supers:?}\n{:?}\n{:?}\n{:?}\n{selves:?}\n\
             {arity:?}\n{:?}\n{:?}\n{:?}\n{:?}",
            set(&t.selfless),
            set(&t.mut_self),
            t.sums,
            t.records,
            t.traits,
            t.impls,
            t.impl_param_names,
            t.externs,
            t.impl_spans,
            t.impl_in_std,
            t.impl_method_spans,
        )
    }

    const PROGRAMS: &[&str] = &[
        "fn main() {}\n",
        "record P<T> { x: T }\ntype S = | A(Int) | B\ntrait Show {\n    type Item\n    \
         fn show(self) -> String\n}\nimpl<T> Show for P<T> {\n    type Item = T\n    \
         fn show(self) -> String { \"p\" }\n}\nfn f<T: Show>(x: T) -> T::Item { panic(\"\") }\n\
         fn main() {}\n",
        "fn f<T: Nope, T>(x: Missing) -> Int { 1 }\nimpl Eq for Int {\n    \
         fn eq(self, other: Int) -> Bool { true }\n}\nimpl [Int] {\n    fn len(self) -> Int { 0 }\n}\n\
         trait A: B {}\nimpl A for Bool {}\nextern \"C\" {\n    fn nova_x(a: String) -> [Int]\n}\n\
         fn main() {}\n",
        "impl<T> Ord for [T] {}\ntrait It {\n    type Item\n    type Item\n    fn g(self)\n    \
         fn g(self)\n}\nimpl It for Int {\n    type Item = Self::Item\n    fn h(self) {}\n}\n\
         fn main() {}\n",
    ];

    /// `check_items` is the item stage of `check_with`: its diagnostics are
    /// the prefix of `check_with`'s, and its tables the module's.
    #[test]
    fn check_items_is_the_prefix_of_check_with() {
        for src in PROGRAMS {
            let ast = parse(src);
            let r = nova_resolver::resolve(&ast);
            let options = CheckOptions::default();
            let items = check_items(&r.file, &r.definitions, &options);
            let all = check_with(&r.file, &r.definitions, &options);
            let n = items.diagnostics.len();
            assert!(all.diagnostics.len() >= n, "{src}");
            assert_eq!(
                format!("{:?}", &all.diagnostics[..n]),
                format!("{:?}", items.diagnostics),
                "{src}"
            );
            let m = &all.module;
            assert_eq!(format!("{:?}", m.sums), format!("{:?}", items.sums));
            assert_eq!(format!("{:?}", m.records), format!("{:?}", items.records));
            assert_eq!(format!("{:?}", m.traits), format!("{:?}", items.traits));
            assert_eq!(format!("{:?}", m.impls), format!("{:?}", items.impls));
            assert_eq!(format!("{:?}", m.externs), format!("{:?}", items.externs));
        }
    }

    /// The shared path gives a program what the full check gives it, with a
    /// std of one more module whose trait and impl a program's items pair
    /// with (E0074, E0105's kin and conformance), and std alone with an
    /// item diagnostic is not shareable, so the driver checks in full.
    #[test]
    fn shared_items_check_what_the_full_check_does() {
        let extra = (
            "$std.zz",
            "pub trait Zed {\n    fn z(self) -> Int\n}\nimpl Zed for Int {\n    \
             fn z(self) -> Int { 1 }\n}\nimpl Bool {\n    pub fn z(self) -> Int { 2 }\n}\n\
             extern \"C\" {\n    fn zz_abs(x: Int) -> Int\n}\n",
        );
        let program = "impl Zed for Int {\n    fn z(self) -> Int { 3 }\n}\n\
                       impl Zed for Bool {\n    fn z(self) -> Int { 4 }\n}\n\
                       record R { z: Option<Int> }\nfn main() {}\n";
        let std = std_with(Some(extra));
        let shared_std = SharedStd::new(CompileTarget::Native, std_with(Some(extra)));
        assert!(shared_std.shareable());
        let alone = shared_std.alone();
        let options = CheckOptions::default();
        let items = SharedItems::new(&alone.file, &alone.definitions, &options);
        assert!(items.shareable());
        let ast = parse(program);
        let sources = [ModuleSource::new("main", &ast)];
        let full = nova_resolver::resolve_program_with_std(CompileTarget::Native, &sources, &std);
        let full_tables = Checked::items(check_items(&full.file, &full.definitions, &options));
        let shared =
            nova_resolver::resolve_program_with_shared_std_for_check(&sources, &shared_std);
        let shared_tables = Checked::items(check_items_shared(
            &shared.file,
            &shared.definitions,
            &options,
            &items,
        ));
        let text = shown(&full, &full_tables);
        assert!(text.contains("E0074"), "{text}");
        assert!(text.contains("E0105"), "{text}");
        assert_eq!(text, shown(&shared, &shared_tables));
        // Every table, std's part included: the splices add, rebased, what
        // the full passes collect, though no item pass reads some of it
        // (std's records, sums, method places and impl selves are the
        // bodies', 5.3b's).
        assert_eq!(
            tables_text(&full_tables.tables),
            tables_text(&shared_tables.tables)
        );

        let broken = ("$std.zz", "pub record Bad { x: Missing }\n");
        let alone = nova_resolver::resolve_program_with_std(
            CompileTarget::Native,
            &[],
            &std_with(Some(broken)),
        );
        let items = SharedItems::new(&alone.file, &alone.definitions, &options);
        assert!(!items.shareable());
    }

    /// The shared path checks a program's own bodies alone and dumps what
    /// the full check dumps, std's closures after the program's (5.3b, spec
    /// 5.3 §4.1, §4.5); a program that implements anything on a type it
    /// does not own, a std record or a primitive, takes the full check, and
    /// one whose impls are on its own types does not; std whose bodies give
    /// a diagnostic, with clean items, is not shareable.
    #[test]
    fn shared_bodies_check_what_the_full_check_does() {
        let extra = (
            "$std.zz",
            "pub record Zr { n: Int }
pub fn zz(x: Int) -> Int {
    \
             Some(x).map(|y| y + 1).unwrap_or(0)
}
",
        );
        let std = std_with(Some(extra));
        let shared_std = SharedStd::new(CompileTarget::Native, std_with(Some(extra)));
        let alone = shared_std.alone();
        let options = CheckOptions::default();
        let items = SharedItems::new(&alone.file, &alone.definitions, &options);
        assert!(items.shareable());
        for (program, takes_full) in [
            (
                "fn main() {\n    let f = |x: Int| x * 2\n    let g = zz\n    \
                 println(\"${f(2)} ${g(1)}\")\n}\n",
                false,
            ),
            (
                "record Mine { x: Int }\nimpl Mine {\n    fn m(self) -> Int { \"s\" }\n}\n\
                 trait T {\n    fn t(self) -> Int\n}\nimpl T for Mine {\n    \
                 fn t(self) -> Int { 1 }\n}\nfn main() {}\n",
                false,
            ),
            (
                "impl Zr {\n    fn m(self) -> Int { 1 }\n}\nfn main() {}\n",
                true,
            ),
            (
                "trait T {\n    fn t(self) -> Int\n}\nimpl T for Int {\n    \
                 fn t(self) -> Int { 1 }\n}\nfn main() {}\n",
                true,
            ),
            (
                "impl<T> [T] {\n    fn m(self) -> Int { 1 }\n}\nfn main() {}\n",
                true,
            ),
        ] {
            let ast = parse(program);
            let sources = [ModuleSource::new("main", &ast)];
            let full =
                nova_resolver::resolve_program_with_std(CompileTarget::Native, &sources, &std);
            let full_checked = check_program(&full.file, &full.definitions, &options);
            let shared =
                nova_resolver::resolve_program_with_shared_std_for_check(&sources, &shared_std);
            let shared_checked =
                check_program_shared(&shared.file, &shared.definitions, &options, &items);
            assert_eq!(shared_checked.is_none(), takes_full, "{program}");
            let text = shown(&full, &full_checked);
            assert!(text.contains("(Function u"), "{text}");
            if let Some(shared_checked) = shared_checked {
                assert_eq!(text, shown(&shared, &shared_checked), "{program}");
                let lifted = |c: &Checked| {
                    let defs = full.definitions.defs().len() as u32;
                    c.functions()
                        .expect("the bodies")
                        .iter()
                        .filter(|f| f.def_id.0 >= defs)
                        .map(|f| f.def_id.0)
                        .collect::<Vec<u32>>()
                };
                let (all, own) = (lifted(&full_checked), lifted(&shared_checked));
                assert!(all.len() > own.len(), "{all:?} {own:?}");
                assert_eq!(all[..own.len()], own[..]);
            }
        }

        // A program's closures print alike whatever std's size: std's one
        // more module moves their ids, not what the dump shows.
        let program = "fn main() {\n    let f = |x: Int| x * 2\n    println(\"${f(2)}\")\n}\n";
        let text_with = |std: &ParsedStd| {
            let ast = parse(program);
            let sources = [ModuleSource::new("main", &ast)];
            let r = nova_resolver::resolve_program_with_std(CompileTarget::Native, &sources, std);
            shown(&r, &check_program(&r.file, &r.definitions, &options))
        };
        let text = text_with(&std);
        assert!(text.contains("\"closure$u1\""), "{text}");
        assert_eq!(text, text_with(&std_with(None)));

        let broken = ("$std.zz", "pub fn bad() -> Int { \"s\" }\n");
        let alone = nova_resolver::resolve_program_with_std(
            CompileTarget::Native,
            &[],
            &std_with(Some(broken)),
        );
        assert!(check_items(&alone.file, &alone.definitions, &options)
            .diagnostics
            .is_empty());
        let items = SharedItems::new(&alone.file, &alone.definitions, &options);
        assert!(!items.shareable());
    }
}
