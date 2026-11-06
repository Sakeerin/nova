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
    /// program of no module of its own.
    pub fn new(file: &ast::File, defs: &Definitions, options: &CheckOptions) -> SharedItems {
        let tables = check_items(file, defs, options);
        SharedItems {
            shareable: tables.diagnostics.is_empty(),
            defs: defs.defs().len() as u32,
            tables,
        }
    }

    /// Whether a program may read these tables ([`SharedItems::shareable`]'s
    /// field): std's items gave no diagnostic.
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
    let own_defs = defs.defs().len() as u32 - shared.defs;
    let rebase = Rebase {
        def: own_defs,
        item: file.items.len(),
    };
    let mut checker = Checker::new(file, defs, options, Some((shared, rebase)));
    checker.own_defs = own_defs;
    checker.collect_items();
    checker.into_tables()
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
                for (id, n) in sorted(&std.type_arity) {
                    self.type_arity.insert(r.id(id), n);
                }
            }
            Splice::Supertraits => {
                for (id, supers) in sorted(&std.supertraits) {
                    self.supertraits.insert(r.id(id), r.ids(&supers));
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
                for (item, s) in sorted(&std.impl_selves) {
                    self.impl_selves.insert(item + r.item, r.impl_self(&s));
                }
            }
            Splice::Signatures => {
                for (id, sig) in sorted(&std.sigs) {
                    if matches!(
                        self.defs.def(r.id(id)).kind,
                        DefKind::Fn { .. } | DefKind::Const { .. }
                    ) {
                        self.sigs.insert(r.id(id), r.sig(&sig));
                    }
                }
            }
            Splice::Externs => {
                for (id, sig) in sorted(&std.sigs) {
                    if matches!(self.defs.def(r.id(id)).kind, DefKind::ExternFn { .. }) {
                        self.sigs.insert(r.id(id), r.sig(&sig));
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
        for (id, loc) in sorted(&std.method_locs) {
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

/// A map's entries sorted by key, so that no map's order reaches a table.
fn sorted<K: Ord + Copy, V: Clone>(map: &FxHashMap<K, V>) -> Vec<(K, V)> {
    let mut entries: Vec<(K, V)> = map.iter().map(|(k, v)| (*k, v.clone())).collect();
    entries.sort_by_key(|(k, _)| *k);
    entries
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

    /// The text of a program's part, from its resolution and tables.
    fn shown(r: &ProgramResolution, t: &ItemTables) -> String {
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
             fn z(self) -> Int { 1 }\n}\nimpl Bool {\n    pub fn z(self) -> Int { 2 }\n}\n",
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
        let full_tables = check_items(&full.file, &full.definitions, &options);
        let shared =
            nova_resolver::resolve_program_with_shared_std_for_check(&sources, &shared_std);
        let shared_tables = check_items_shared(&shared.file, &shared.definitions, &options, &items);
        let text = shown(&full, &full_tables);
        assert!(text.contains("E0074"), "{text}");
        assert!(text.contains("E0105"), "{text}");
        assert_eq!(text, shown(&shared, &shared_tables));

        let broken = ("$std.zz", "pub record Bad { x: Missing }\n");
        let alone = nova_resolver::resolve_program_with_std(
            CompileTarget::Native,
            &[],
            &std_with(Some(broken)),
        );
        let items = SharedItems::new(&alone.file, &alone.definitions, &options);
        assert!(!items.shareable());
    }
}
