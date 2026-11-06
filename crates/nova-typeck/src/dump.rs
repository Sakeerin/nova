//! The HIR dump: what the type checker built for one program, as text, one
//! entry per line, which the self-hosted type checker (`selfhost/`) prints
//! too, so that the two can be compared byte for byte (spec
//! `docs/superpowers/specs/2026-11-06-phase-5-3-the-type-checker-and-hir-design.md`
//! §3.3; ADR 0055 writes the grammar out, with an example of every form).
//! It carries no stability promise: it changes whenever the type checker
//! does, on both sides in one pull request. `nova check --dump-hir FILE…`
//! prints a program's part after a `== FILE` header, and `nova check
//! --dump-hir --std` std's part; `--target wasm` and `--test` as the
//! resolution dump takes them (ADR 0054).
//!
//! - It follows the resolution dump's conventions (ADR 0053 §1): a line is
//!   its indentation, two spaces per depth, `(` and its kind, its values
//!   and its fields `key=value`; its children follow, one level deeper,
//!   and its `)` closes it at the end of its last line. Names and messages
//!   are quoted as the AST dump quotes strings; a place is a file's label
//!   quoted, `:` and a span; diagnostics print as the resolution dump's.
//! - **5.3a's part is the items** (passes 2 to 5): records, sums, traits,
//!   impls, every signature by `DefId`, externs, then every diagnostic so
//!   far, the earlier stages' and the item passes'. 5.3b adds every
//!   function, after the externs and before the diagnostics.
//! - **Two parts.** std's tables are the same in every program for a
//!   target and mode, so they print once, by [`dump_std`]; a program's
//!   part, by [`dump_program`], prints only the program's own entries, and
//!   `(StdStart module= def= item=)` as the resolution dump does.
//! - **Ids** are the resolution dump's: `u<n>` for the program's own
//!   definition, `s<n>` for std's, its `DefId` less the first std
//!   definition's. An impl, which has no `DefId`, is numbered from 0 within
//!   its part, in the table's order.
//! - **Types** print by structure, with ids, not names: `Int`, `()`,
//!   `[T0]`, `(Int, String)`, `(Int,)`, `fn(Int) -> Bool`, `Future<Int>`,
//!   `T0` for a type parameter, a record or a sum as its id and its
//!   arguments (`s3<Int>`), a projection as its type, `::` and the
//!   associated type's id (`T0::s20`), `!`, `{error}`, and an inference
//!   variable as `?N`, as S0's messages print it.
//! - Tables print in the order the type checker collected them; a map's
//!   entries only sorted, by `DefId`.

use std::fmt::Write;

use nova_ast::dump::quote;
use nova_diagnostics::Diagnostic;
use nova_hir::{Ty, TyHead};
use nova_resolver::dump::{std_start, write_diagnostics, write_place, Labels};
use nova_resolver::{CompileTarget, DefId, DefKind, Definitions, MethodOwner, ProgramResolution};

use crate::ItemTables;

/// Every form the dump prints, as [`nova_resolver::dump::forms_on`] reads
/// a line, beyond those whose values are numbers or ids: the harness
/// checks that std, the corpus and the cases print each of them and print
/// nothing unlisted (`crates/nova-cli/tests/selfhost.rs`).
pub const KINDS: &[&str] = &[
    "(Hir",
    "(Hir checked=true",
    "(Hir checked=false",
    "(HirStd",
    "target=native",
    "target=wasm",
    "mode=test",
    "(StdStart",
    "(Record",
    "(Field",
    "(Sum",
    "(Variant",
    "(Trait",
    "(Supertrait",
    "(AssocType",
    "(Method",
    "(Method has_self=true",
    "(Method has_self=false",
    "(Method mut_self=true",
    "(Method mut_self=false",
    "default=None",
    "(Bound",
    "(Param",
    "(Ret",
    "(Impl",
    "trait=None",
    "head=Int",
    "head=Float",
    "head=Bool",
    "head=String",
    "head=Array",
    "(SelfType",
    "(AssocBinding",
    "(Sig",
    "(Sig selfless=true",
    "(Sig selfless=false",
    "(Sig mut_self=true",
    "(Sig mut_self=false",
    "(Extern",
    "abi=C",
    "abi=Js",
    "(Diagnostics",
    "(Error",
    "(Warning",
    "(Primary",
    "(Secondary",
    "(Note",
    "(Fix",
    "(Edit",
];

/// The forms no input prints: impls on `Char` and `Bytes`, which neither
/// std nor any input writes, and the two severities no stage gives.
pub const LATER_KINDS: &[&str] = &["head=Char", "head=Bytes", "(Help", "(NoteDiagnostic"];

/// A program's part (module docs) for `target`, under test mode when
/// `test`: when the program reached the type checker, its resolution and
/// item tables, and every diagnostic so far, the package graph's, the
/// loader's, the resolver's and the item passes', in order. A program that
/// stopped before the type checker prints `checked=false` and the
/// diagnostics.
pub fn dump_program(
    target: CompileTarget,
    test: bool,
    labels: &Labels,
    checked: Option<(&ProgramResolution, &ItemTables)>,
    diagnostics: &[Diagnostic],
) -> String {
    let mut d = Dump::new(labels, u32::MAX);
    d.open(0, "Hir");
    d.field("target", target.name());
    d.mode(test);
    d.bool_field("checked", checked.is_some());
    if let Some((resolution, tables)) = checked {
        let defs = &resolution.definitions;
        let (module, def, item) = std_start(defs);
        d.std_def = def;
        d.open(1, "StdStart");
        d.field("module", &module.to_string());
        d.field("def", &def.to_string());
        d.field("item", &item.to_string());
        d.close();
        d.tables(defs, tables, false);
    }
    d.close();
    d.out.push('\n');
    write_diagnostics(&mut d.out, labels, diagnostics);
    d.out
}

/// std's part (module docs) for `target`, under test mode when `test`, from
/// the check of a program of no module of its own: its resolution, its
/// item tables, and every diagnostic, the resolver's and the item passes'.
pub fn dump_std(
    target: CompileTarget,
    test: bool,
    labels: &Labels,
    resolution: &ProgramResolution,
    tables: &ItemTables,
    diagnostics: &[Diagnostic],
) -> String {
    let mut out = std_part_of(target, test, labels, resolution, tables);
    write_diagnostics(&mut out, labels, diagnostics);
    out
}

/// std's part of any program's check, without its diagnostics: the same
/// text as [`dump_std`]'s for a program of no module of its own, in every
/// program for one target and mode, which a test checks.
pub fn std_part_of(
    target: CompileTarget,
    test: bool,
    labels: &Labels,
    resolution: &ProgramResolution,
    tables: &ItemTables,
) -> String {
    let defs = &resolution.definitions;
    let (_, std_def, _) = std_start(defs);
    let mut d = Dump::new(labels, std_def);
    d.open(0, "HirStd");
    d.field("target", target.name());
    d.mode(test);
    let std = |id: DefId| id.0 >= std_def;
    let count = |n: usize| n.to_string();
    d.field(
        "records",
        &count(tables.records.iter().filter(|r| std(r.def_id)).count()),
    );
    d.field(
        "sums",
        &count(tables.sums.iter().filter(|s| std(s.def_id)).count()),
    );
    d.field(
        "traits",
        &count(tables.traits.iter().filter(|t| std(t.def_id)).count()),
    );
    d.field(
        "impls",
        &count(tables.impl_in_std.iter().filter(|s| **s).count()),
    );
    d.field(
        "sigs",
        &count(tables.sigs.keys().filter(|id| std(**id)).count()),
    );
    d.field(
        "externs",
        &count(tables.externs.iter().filter(|e| std(e.def_id)).count()),
    );
    d.tables(defs, tables, true);
    d.close();
    d.out.push('\n');
    d.out
}

/// The text being written, the files' labels, and the first std
/// definition's id, from which ids print as `s<n>`.
struct Dump<'a> {
    out: String,
    labels: &'a Labels,
    std_def: u32,
}

impl<'a> Dump<'a> {
    fn new(labels: &'a Labels, std_def: u32) -> Self {
        Dump {
            out: String::new(),
            labels,
            std_def,
        }
    }

    // --- Lines and values ---

    /// A line, up to its kind.
    fn open(&mut self, depth: usize, kind: &str) {
        if !self.out.is_empty() && !self.out.ends_with('\n') {
            self.out.push('\n');
        }
        for _ in 0..depth {
            self.out.push_str("  ");
        }
        self.out.push('(');
        self.out.push_str(kind);
    }

    fn close(&mut self) {
        self.out.push(')');
    }

    /// ` key=value`.
    fn field(&mut self, key: &str, value: &str) {
        let _ = write!(self.out, " {key}={value}");
    }

    /// ` mode=test` under test mode, and nothing under `check`'s.
    fn mode(&mut self, test: bool) {
        if test {
            self.field("mode", "test");
        }
    }

    fn bool_field(&mut self, key: &str, value: bool) {
        let _ = write!(self.out, " {key}={value}");
    }

    /// ` ` and `text` quoted.
    fn string(&mut self, text: &str) {
        self.out.push(' ');
        quote(&mut self.out, text);
    }

    /// ` ` and a value.
    fn value(&mut self, text: &str) {
        self.out.push(' ');
        self.out.push_str(text);
    }

    /// A definition's id: `u<n>` for the program's own, `s<n>` for std's.
    fn id(&self, id: DefId) -> String {
        if id.0 >= self.std_def {
            format!("s{}", id.0 - self.std_def)
        } else {
            format!("u{}", id.0)
        }
    }

    fn opt_id(&self, id: Option<DefId>) -> String {
        id.map_or_else(|| "None".to_string(), |id| self.id(id))
    }

    /// A type, by structure (module docs).
    fn ty(&self, ty: &Ty) -> String {
        let list = |tys: &[Ty]| tys.iter().map(|t| self.ty(t)).collect::<Vec<_>>();
        match ty {
            Ty::Int => "Int".to_string(),
            Ty::Float => "Float".to_string(),
            Ty::Bool => "Bool".to_string(),
            Ty::Char => "Char".to_string(),
            Ty::String => "String".to_string(),
            Ty::Bytes => "Bytes".to_string(),
            Ty::Unit => "()".to_string(),
            Ty::Fn { params, ret } => {
                format!("fn({}) -> {}", list(params).join(", "), self.ty(ret))
            }
            Ty::Future(out) => format!("Future<{}>", self.ty(out)),
            Ty::Sum { def_id, args } | Ty::Record { def_id, args } => {
                if args.is_empty() {
                    self.id(*def_id)
                } else {
                    format!("{}<{}>", self.id(*def_id), list(args).join(", "))
                }
            }
            Ty::Array(elem) => format!("[{}]", self.ty(elem)),
            Ty::Tuple(elems) => crate::tuple_text(list(elems)),
            Ty::Param(i) => format!("T{i}"),
            Ty::Assoc { on, assoc } => format!("{}::{}", self.ty(on), self.id(*assoc)),
            Ty::Var(v) => format!("?{v}"),
            Ty::Never => "!".to_string(),
            Ty::Error => "{error}".to_string(),
        }
    }

    fn head(&self, head: TyHead) -> String {
        match head {
            TyHead::Int => "Int".to_string(),
            TyHead::Float => "Float".to_string(),
            TyHead::Bool => "Bool".to_string(),
            TyHead::Char => "Char".to_string(),
            TyHead::String => "String".to_string(),
            TyHead::Bytes => "Bytes".to_string(),
            TyHead::Array => "Array".to_string(),
            TyHead::Sum(id) | TyHead::Record(id) => self.id(id),
        }
    }

    /// `(Bound k id…)` for each generic parameter's bounds, at `depth`.
    fn bounds(&mut self, depth: usize, bounds: &[Vec<DefId>]) {
        for (k, slot) in bounds.iter().enumerate() {
            self.open(depth, "Bound");
            let _ = write!(self.out, " {k}");
            for &id in slot {
                let id = self.id(id);
                self.value(&id);
            }
            self.close();
        }
    }

    /// `(Param ty)` for each parameter, then `(Ret ty)`, at `depth`.
    fn signature(&mut self, depth: usize, params: &[Ty], ret: &Ty) {
        for p in params {
            self.open(depth, "Param");
            let t = self.ty(p);
            self.value(&t);
            self.close();
        }
        self.open(depth, "Ret");
        let t = self.ty(ret);
        self.value(&t);
        self.close();
    }

    // --- The tables ---

    /// The entries of std's part (`std`) or of the program's.
    fn tables(&mut self, defs: &Definitions, t: &ItemTables, std: bool) {
        let std_def = self.std_def;
        let ours = move |id: DefId| (id.0 >= std_def) == std;
        for r in t.records.iter().filter(|r| ours(r.def_id)) {
            self.open(1, "Record");
            let id = self.id(r.def_id);
            self.value(&id);
            self.string(&r.name);
            self.field("generics", &r.generics.to_string());
            for f in &r.fields {
                self.open(2, "Field");
                self.string(&f.name);
                let ty = self.ty(&f.ty);
                self.value(&ty);
                self.close();
            }
            self.close();
        }
        for s in t.sums.iter().filter(|s| ours(s.def_id)) {
            self.open(1, "Sum");
            let id = self.id(s.def_id);
            self.value(&id);
            self.string(&s.name);
            self.field("generics", &s.generics.to_string());
            for v in &s.variants {
                self.open(2, "Variant");
                self.string(&v.name);
                for f in &v.fields {
                    let ty = self.ty(f);
                    self.value(&ty);
                }
                self.close();
            }
            self.close();
        }
        for tr in t.traits.iter().filter(|tr| ours(tr.def_id)) {
            self.open(1, "Trait");
            let id = self.id(tr.def_id);
            self.value(&id);
            self.string(&tr.name);
            for &s in &tr.supertraits {
                self.open(2, "Supertrait");
                let id = self.id(s);
                self.value(&id);
                self.close();
            }
            for (name, assoc) in &tr.assoc_types {
                self.open(2, "AssocType");
                self.string(name);
                let id = self.id(*assoc);
                self.value(&id);
                self.close();
            }
            for m in &tr.methods {
                self.open(2, "Method");
                self.string(&m.name);
                self.bool_field("has_self", m.has_self);
                self.bool_field("mut_self", m.mut_self);
                self.field("generics", &m.generics.to_string());
                let default = self.opt_id(m.default_def);
                self.field("default", &default);
                self.bounds(3, &m.bounds);
                self.signature(3, &m.params, &m.ret);
                self.close();
            }
            self.close();
        }
        let mut index = 0;
        for (k, imp) in t.impls.iter().enumerate() {
            if t.impl_in_std.get(k).copied().unwrap_or(false) != std {
                continue;
            }
            self.open(1, "Impl");
            let _ = write!(self.out, " {index}");
            index += 1;
            let tr = self.opt_id(imp.trait_id);
            self.field("trait", &tr);
            let head = self.head(imp.self_head);
            self.field("head", &head);
            self.field("generics", &imp.generics.to_string());
            if let Some(&span) = t.impl_spans.get(k) {
                write_place(&mut self.out, self.labels, span);
            }
            self.open(2, "SelfType");
            let ty = self.ty(&imp.self_ty);
            self.value(&ty);
            self.close();
            self.bounds(2, &imp.bounds);
            for (name, def) in &imp.methods {
                self.open(2, "Method");
                self.string(name);
                let id = self.id(*def);
                self.value(&id);
                self.close();
            }
            for (assoc, ty) in &imp.assoc_bindings {
                self.open(2, "AssocBinding");
                let id = self.id(*assoc);
                self.value(&id);
                let ty = self.ty(ty);
                self.value(&ty);
                self.close();
            }
            self.close();
        }
        let mut sigs: Vec<DefId> = t.sigs.keys().copied().filter(|id| ours(*id)).collect();
        sigs.sort_by_key(|id| id.0);
        for id in sigs {
            let sig = &t.sigs[&id];
            self.open(1, "Sig");
            let shown = self.id(id);
            self.value(&shown);
            self.field("generics", &sig.generics.to_string());
            let impl_method = matches!(
                defs.def(id).kind,
                DefKind::Method {
                    owner: MethodOwner::Impl,
                    ..
                }
            );
            if impl_method {
                self.bool_field("selfless", t.selfless.contains(&id));
                self.bool_field("mut_self", t.mut_self.contains(&id));
            }
            self.bounds(2, &sig.bounds);
            self.signature(2, &sig.params, &sig.ret);
            self.close();
        }
        for e in t.externs.iter().filter(|e| ours(e.def_id)) {
            self.open(1, "Extern");
            let id = self.id(e.def_id);
            self.value(&id);
            self.string(&e.symbol);
            let abi = match e.abi {
                nova_hir::ExternAbi::C => "C",
                nova_hir::ExternAbi::Js => "Js",
            };
            self.field("abi", abi);
            write_place(&mut self.out, self.labels, e.span);
            self.signature(2, &e.params, &e.ret);
            self.close();
        }
    }
}
