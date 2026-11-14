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
//!   far, the earlier stages' and the item passes'. **5.3b's adds every
//!   function** (ADR 0056), after the externs and before the diagnostics,
//!   in `hir::Module`'s order: its line (id, name, generics, `takes_env`,
//!   captures, parameters, `async`, place), its bounds, its return type,
//!   its locals (`l<n>`) and its body, each node `(Kind values… span type`
//!   with its type arguments, a trait call's self type and its children a
//!   level deeper; a node's span is `start..end` in its function's file;
//!   a pattern carries no span or type. The type checker's diagnostics
//!   are then the bodies' too.
//! - **Two parts.** std's tables are the same in every program for a
//!   target and mode, so they print once, by [`dump_std`]; a program's
//!   part, by [`dump_program`], prints only the program's own entries, and
//!   `(StdStart module= def= item=)` as the resolution dump does.
//! - **Ids** are the resolution dump's: `u<n>` for the program's own
//!   definition, `s<n>` for std's, its `DefId` less the first std
//!   definition's. An impl, which has no `DefId`, is numbered from 0 within
//!   its part, in the table's order. A closure or wrapper, whose `DefId` is
//!   past every definition, prints as though its part's own closures came
//!   right after that part's definitions (5.3b), its name's number too.
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
use nova_diagnostics::Span;
use nova_hir::{BinOp, Callee, Expr, ExprKind, Function, Pattern, Ty, TyHead, UnOp};
use nova_resolver::dump::{std_start, write_diagnostics, write_place, Labels};
use nova_resolver::{CompileTarget, DefId, DefKind, Definitions, MethodOwner, ProgramResolution};

use crate::{Checked, ItemTables};

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
    "head=Char",
    "head=String",
    "head=Bytes",
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

/// Every form the full dump's functions print (5.3b, ADR 0056), beyond
/// [`KINDS`]: the full dumps of std, the corpus and the cases print each
/// of them, and 5.3a's item dumps none.
pub const BODY_KINDS: &[&str] = &[
    "(Function",
    "(Function takes_env=true",
    "(Function takes_env=false",
    "(Function async=true",
    "(Function async=false",
    "(LocalDecl",
    "(LocalDecl mut=true",
    "(LocalDecl mut=false",
    "(TypeArgs",
    "(IntLit",
    "(FloatLit",
    "(BoolLit",
    "(StrLit",
    "(CharLit",
    "(Unit",
    "(Local",
    "(MakeClosure",
    "(Call",
    "(MakeVariant",
    "(MakeRecord",
    "(MakeTuple",
    "(FieldGet",
    "(FieldSet",
    "(MakeArray",
    "(ArrayRepeat",
    "(Index",
    "(IndexSet",
    "(ArrayLen",
    "(TraitCall",
    "(TraitCall receiver=true",
    "(TraitCall receiver=false",
    "(Binary",
    "(LogicalAnd",
    "(LogicalOr",
    "(Unary",
    "(Let",
    "(Assign",
    "(Block",
    "(Block tail=true",
    "(Block tail=false",
    "(If",
    "(If else=true",
    "(If else=false",
    "(While",
    "(Break",
    "(Continue",
    "(Match",
    "(Arm",
    "(Arm guard=true",
    "(Arm guard=false",
    "(Return",
    "(Return value=true",
    "(Return value=false",
    "(ToStr",
    "(StrConcat",
    "(Await",
    "(PatWildcard",
    "(PatBind",
    "(PatInt",
    "(PatBool",
    "(PatStr",
    "(PatChar",
    "(PatRange",
    "(PatVariant",
    "(PatTuple",
    "(PatOr",
];

/// The forms no input prints: the two severities no stage gives.
pub const LATER_KINDS: &[&str] = &["(Help", "(NoteDiagnostic"];

/// A program's part (module docs) for `target`, under test mode when
/// `test`: when the program reached the type checker, its resolution, its
/// item tables and its functions, each with its locals and its typed body
/// (5.3b), and every diagnostic, the package graph's, the loader's, the
/// resolver's, the item passes' and the bodies', in order. A program that
/// stopped before the type checker prints `checked=false` and the
/// diagnostics so far.
pub fn dump_program(
    target: CompileTarget,
    test: bool,
    labels: &Labels,
    checked: Option<(&ProgramResolution, &Checked)>,
    diagnostics: &[Diagnostic],
) -> String {
    let mut d = Dump::new(labels, u32::MAX);
    d.open(0, "Hir");
    d.field("target", target.name());
    d.mode(test);
    d.bool_field("checked", checked.is_some());
    if let Some((resolution, checked)) = checked {
        let defs = &resolution.definitions;
        let (module, def, item) = std_start(defs);
        d.std_def = def;
        d.closures = checked.closures;
        d.std_closures = checked.std_closures;
        d.open(1, "StdStart");
        d.field("module", &module.to_string());
        d.field("def", &def.to_string());
        d.field("item", &item.to_string());
        d.close();
        d.tables(defs, &checked.tables, false);
        d.functions(checked, false);
    }
    d.close();
    d.out.push('\n');
    write_diagnostics(&mut d.out, labels, diagnostics);
    d.out
}

/// std's part (module docs) for `target`, under test mode when `test`, from
/// the check of a program of no module of its own: its resolution, its
/// item tables and its functions (5.3b), and every diagnostic, the
/// resolver's, the item passes' and the bodies'.
pub fn dump_std(
    target: CompileTarget,
    test: bool,
    labels: &Labels,
    resolution: &ProgramResolution,
    checked: &Checked,
    diagnostics: &[Diagnostic],
) -> String {
    let mut out = std_part_of(target, test, labels, resolution, checked);
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
    checked: &Checked,
) -> String {
    let defs = &resolution.definitions;
    let tables = &checked.tables;
    let (_, std_def, _) = std_start(defs);
    let mut d = Dump::new(labels, std_def);
    d.closures = checked.closures;
    d.std_closures = checked.std_closures;
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
    let functions = checked.functions();
    let n = functions.iter().filter(|f| d.ours(f.def_id, true)).count();
    d.field("functions", &count(n));
    d.tables(defs, tables, true);
    d.functions(checked, true);
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
    /// The first closure's or wrapper's id, `defs.len()`, and the first of
    /// std's ([`Checked`]); `u32::MAX` when no body was checked.
    closures: u32,
    std_closures: u32,
}

impl<'a> Dump<'a> {
    fn new(labels: &'a Labels, std_def: u32) -> Self {
        Dump {
            out: String::new(),
            labels,
            std_def,
            closures: u32::MAX,
            std_closures: u32::MAX,
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
    /// A closure or wrapper, past every definition, prints as though its
    /// part's own came right after that part's definitions (5.3b, ADR
    /// 0056): the program's from `u<std_def>` on, std's from `s<n>`, `n`
    /// std's definitions, on. So the program's closures print alike
    /// whatever std's size, and std's alike in every program, whose own
    /// closures come first (spec 5.3 §4.1).
    fn id(&self, id: DefId) -> String {
        if id.0 >= self.std_closures {
            format!(
                "s{}",
                self.closures - self.std_def + (id.0 - self.std_closures)
            )
        } else if id.0 >= self.closures {
            format!("u{}", self.std_def + (id.0 - self.closures))
        } else if id.0 >= self.std_def {
            format!("s{}", id.0 - self.std_def)
        } else {
            format!("u{}", id.0)
        }
    }

    /// Whether the function `id` is std's part's (`std`) or the program's.
    fn ours(&self, id: DefId, std: bool) -> bool {
        let in_std = if id.0 >= self.closures {
            id.0 >= self.std_closures
        } else {
            id.0 >= self.std_def
        };
        in_std == std
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

    // --- The functions (5.3b) ---

    /// Each function of std's part (`std`) or of the program's, in
    /// `hir::Module`'s order.
    fn functions(&mut self, checked: &Checked, std: bool) {
        for f in checked.functions() {
            if self.ours(f.def_id, std) {
                self.function(f);
            }
        }
    }

    /// A function: its line, its bounds, its return type, its locals and
    /// its body. A closure's or wrapper's name holds its id as printed.
    fn function(&mut self, f: &Function) {
        self.open(1, "Function");
        let id = self.id(f.def_id);
        self.value(&id);
        if f.def_id.0 >= self.closures {
            let stem = f.name.split('$').next().unwrap_or("");
            self.string(&format!("{stem}${id}"));
        } else {
            self.string(&f.name);
        }
        self.field("generics", &f.generics.to_string());
        self.bool_field("takes_env", f.takes_env);
        self.field("captures", &f.capture_count.to_string());
        self.field("params", &f.params.to_string());
        self.bool_field("async", f.is_async);
        write_place(&mut self.out, self.labels, f.span);
        self.bounds(2, &f.bounds);
        self.open(2, "Ret");
        let ret = self.ty(&f.ret_ty);
        self.value(&ret);
        self.close();
        for (k, local) in f.locals.iter().enumerate() {
            self.open(2, "LocalDecl");
            let _ = write!(self.out, " l{k}");
            self.string(&local.name);
            self.bool_field("mut", local.is_mut);
            self.span(local.span, f.span);
            let ty = self.ty(&local.ty);
            self.value(&ty);
            self.close();
        }
        self.expr(2, &f.body, f.span);
        self.close();
    }

    /// ` start..end`, or the whole place when `span` is in another file
    /// than `within`, the function's.
    fn span(&mut self, span: Span, within: Span) {
        if span.file == within.file {
            let _ = write!(self.out, " {}..{}", span.start, span.end);
        } else {
            write_place(&mut self.out, self.labels, span);
        }
    }

    /// An expression's line, up to its values: its kind, then, after the
    /// values the caller writes, [`Dump::node_end`] writes its span and
    /// type.
    fn node_end(&mut self, e: &Expr, within: Span) {
        self.span(e.span, within);
        let ty = self.ty(&e.ty);
        self.value(&ty);
    }

    /// `(TypeArgs ty…)` at `depth`, when there are any.
    fn type_args(&mut self, depth: usize, tys: &[Ty]) {
        if tys.is_empty() {
            return;
        }
        self.open(depth, "TypeArgs");
        for t in tys {
            let t = self.ty(t);
            self.value(&t);
        }
        self.close();
    }

    /// An expression at `depth`: `(Kind values… span type`, its type
    /// arguments, then its children a level deeper, then `)`.
    fn expr(&mut self, depth: usize, e: &Expr, within: Span) {
        let d = depth + 1;
        let children: Vec<&Expr> = match &e.kind {
            ExprKind::IntLit(n) => {
                self.open(depth, "IntLit");
                let _ = write!(self.out, " {n}");
                self.node_end(e, within);
                Vec::new()
            }
            ExprKind::FloatLit(f) => {
                self.open(depth, "FloatLit");
                let _ = write!(self.out, " {:016x}", f.to_bits());
                self.node_end(e, within);
                Vec::new()
            }
            ExprKind::BoolLit(b) => {
                self.open(depth, "BoolLit");
                let _ = write!(self.out, " {b}");
                self.node_end(e, within);
                Vec::new()
            }
            ExprKind::StrLit(text) => {
                self.open(depth, "StrLit");
                self.string(text);
                self.node_end(e, within);
                Vec::new()
            }
            ExprKind::CharLit(c) => {
                self.open(depth, "CharLit");
                let _ = write!(self.out, " {}", *c as u32);
                self.node_end(e, within);
                Vec::new()
            }
            ExprKind::Unit => {
                self.open(depth, "Unit");
                self.node_end(e, within);
                Vec::new()
            }
            ExprKind::Local(l) => {
                self.open(depth, "Local");
                let _ = write!(self.out, " l{}", l.0);
                self.node_end(e, within);
                Vec::new()
            }
            ExprKind::MakeClosure {
                func,
                type_args,
                captures,
            } => {
                self.open(depth, "MakeClosure");
                let id = self.id(*func);
                self.value(&id);
                self.node_end(e, within);
                self.type_args(d, type_args);
                captures.iter().collect()
            }
            ExprKind::Call {
                func,
                type_args,
                args,
            } => {
                self.open(depth, "Call");
                match func {
                    Callee::Def(id) => {
                        let id = self.id(*id);
                        let _ = write!(self.out, " Def {id}");
                    }
                    Callee::Builtin(b) => {
                        let _ = write!(self.out, " Builtin {b:?}");
                    }
                    Callee::Local(l) => {
                        let _ = write!(self.out, " Local l{}", l.0);
                    }
                }
                self.node_end(e, within);
                self.type_args(d, type_args);
                args.iter().collect()
            }
            ExprKind::MakeVariant { sum, variant, args } => {
                self.open(depth, "MakeVariant");
                let id = self.id(*sum);
                let _ = write!(self.out, " {id} {variant}");
                self.node_end(e, within);
                args.iter().collect()
            }
            ExprKind::MakeRecord { record, fields } => {
                self.open(depth, "MakeRecord");
                let id = self.id(*record);
                self.value(&id);
                self.node_end(e, within);
                fields.iter().collect()
            }
            ExprKind::MakeTuple { elems } => {
                self.open(depth, "MakeTuple");
                self.node_end(e, within);
                elems.iter().collect()
            }
            ExprKind::FieldGet { target, index } => {
                self.open(depth, "FieldGet");
                let _ = write!(self.out, " {index}");
                self.node_end(e, within);
                vec![&**target]
            }
            ExprKind::FieldSet {
                target,
                index,
                value,
            } => {
                self.open(depth, "FieldSet");
                let _ = write!(self.out, " {index}");
                self.node_end(e, within);
                vec![&**target, &**value]
            }
            ExprKind::MakeArray { elems } => {
                self.open(depth, "MakeArray");
                self.node_end(e, within);
                elems.iter().collect()
            }
            ExprKind::ArrayRepeat { init, len } => {
                self.open(depth, "ArrayRepeat");
                self.node_end(e, within);
                vec![&**init, &**len]
            }
            ExprKind::Index { target, index } => {
                self.open(depth, "Index");
                self.node_end(e, within);
                vec![&**target, &**index]
            }
            ExprKind::IndexSet {
                target,
                index,
                value,
            } => {
                self.open(depth, "IndexSet");
                self.node_end(e, within);
                vec![&**target, &**index, &**value]
            }
            ExprKind::ArrayLen { target } => {
                self.open(depth, "ArrayLen");
                self.node_end(e, within);
                vec![&**target]
            }
            ExprKind::TraitCall {
                trait_id,
                method,
                self_ty,
                type_args,
                receiver,
                args,
            } => {
                self.open(depth, "TraitCall");
                let id = self.id(*trait_id);
                let _ = write!(self.out, " {id} {method}");
                self.bool_field("receiver", receiver.is_some());
                self.node_end(e, within);
                self.open(d, "SelfType");
                let ty = self.ty(self_ty);
                self.value(&ty);
                self.close();
                self.type_args(d, type_args);
                receiver.iter().map(|r| &**r).chain(args.iter()).collect()
            }
            ExprKind::Binary { op, lhs, rhs } => {
                self.open(depth, "Binary");
                self.value(bin_op(*op));
                self.node_end(e, within);
                vec![&**lhs, &**rhs]
            }
            ExprKind::LogicalAnd { lhs, rhs } => {
                self.open(depth, "LogicalAnd");
                self.node_end(e, within);
                vec![&**lhs, &**rhs]
            }
            ExprKind::LogicalOr { lhs, rhs } => {
                self.open(depth, "LogicalOr");
                self.node_end(e, within);
                vec![&**lhs, &**rhs]
            }
            ExprKind::Unary { op, expr } => {
                self.open(depth, "Unary");
                self.value(match op {
                    UnOp::Neg => "Neg",
                    UnOp::Not => "Not",
                    UnOp::BitNot => "BitNot",
                });
                self.node_end(e, within);
                vec![&**expr]
            }
            ExprKind::Let { local, init } => {
                self.open(depth, "Let");
                let _ = write!(self.out, " l{}", local.0);
                self.node_end(e, within);
                vec![&**init]
            }
            ExprKind::Assign { local, value } => {
                self.open(depth, "Assign");
                let _ = write!(self.out, " l{}", local.0);
                self.node_end(e, within);
                vec![&**value]
            }
            ExprKind::Block { stmts, trailing } => {
                self.open(depth, "Block");
                self.bool_field("tail", trailing.is_some());
                self.node_end(e, within);
                stmts.iter().chain(trailing.iter().map(|t| &**t)).collect()
            }
            ExprKind::If { cond, then, else_ } => {
                self.open(depth, "If");
                self.bool_field("else", else_.is_some());
                self.node_end(e, within);
                [&**cond, &**then]
                    .into_iter()
                    .chain(else_.iter().map(|x| &**x))
                    .collect()
            }
            ExprKind::While { cond, body } => {
                self.open(depth, "While");
                self.node_end(e, within);
                vec![&**cond, &**body]
            }
            ExprKind::Break => {
                self.open(depth, "Break");
                self.node_end(e, within);
                Vec::new()
            }
            ExprKind::Continue => {
                self.open(depth, "Continue");
                self.node_end(e, within);
                Vec::new()
            }
            ExprKind::Match { scrutinee, arms } => {
                self.open(depth, "Match");
                self.node_end(e, within);
                self.expr(d, scrutinee, within);
                for arm in arms {
                    self.open(d, "Arm");
                    self.bool_field("guard", arm.guard.is_some());
                    self.span(arm.span, within);
                    self.pattern(d + 1, &arm.pattern);
                    if let Some(g) = &arm.guard {
                        self.expr(d + 1, g, within);
                    }
                    self.expr(d + 1, &arm.body, within);
                    self.close();
                }
                self.close();
                return;
            }
            ExprKind::Return(value) => {
                self.open(depth, "Return");
                self.bool_field("value", value.is_some());
                self.node_end(e, within);
                value.iter().map(|v| &**v).collect()
            }
            ExprKind::ToStr(inner) => {
                self.open(depth, "ToStr");
                self.node_end(e, within);
                vec![&**inner]
            }
            ExprKind::StrConcat(parts) => {
                self.open(depth, "StrConcat");
                self.node_end(e, within);
                parts.iter().collect()
            }
            ExprKind::Await(inner) => {
                self.open(depth, "Await");
                self.node_end(e, within);
                vec![&**inner]
            }
        };
        for c in children {
            self.expr(d, c, within);
        }
        self.close();
    }

    /// A pattern at `depth`, which carries no span or type: `(PatKind
    /// values…`, its sub-patterns a level deeper, then `)`.
    fn pattern(&mut self, depth: usize, p: &Pattern) {
        let subs: &[Pattern] = match p {
            Pattern::Wildcard => {
                self.open(depth, "PatWildcard");
                &[]
            }
            Pattern::Bind(l) => {
                self.open(depth, "PatBind");
                let _ = write!(self.out, " l{}", l.0);
                &[]
            }
            Pattern::LitInt(n) => {
                self.open(depth, "PatInt");
                let _ = write!(self.out, " {n}");
                &[]
            }
            Pattern::LitBool(b) => {
                self.open(depth, "PatBool");
                let _ = write!(self.out, " {b}");
                &[]
            }
            Pattern::LitStr(text) => {
                self.open(depth, "PatStr");
                self.string(text);
                &[]
            }
            Pattern::LitChar(c) => {
                self.open(depth, "PatChar");
                let _ = write!(self.out, " {}", *c as u32);
                &[]
            }
            Pattern::Range { lo, hi } => {
                self.open(depth, "PatRange");
                let _ = write!(self.out, " {lo} {hi}");
                &[]
            }
            Pattern::Variant {
                sum,
                variant,
                fields,
            } => {
                self.open(depth, "PatVariant");
                let id = self.id(*sum);
                let _ = write!(self.out, " {id} {variant}");
                fields
            }
            Pattern::Tuple(elems) => {
                self.open(depth, "PatTuple");
                elems
            }
            Pattern::Or(alts) => {
                self.open(depth, "PatOr");
                alts
            }
        };
        for q in subs {
            self.pattern(depth + 1, q);
        }
        self.close();
    }
}

/// A binary operator's name, as `hir::BinOp` names it.
fn bin_op(op: BinOp) -> &'static str {
    match op {
        BinOp::Add => "Add",
        BinOp::Sub => "Sub",
        BinOp::Mul => "Mul",
        BinOp::Div => "Div",
        BinOp::Rem => "Rem",
        BinOp::Eq => "Eq",
        BinOp::Ne => "Ne",
        BinOp::Lt => "Lt",
        BinOp::Le => "Le",
        BinOp::Gt => "Gt",
        BinOp::Ge => "Ge",
        BinOp::BitAnd => "BitAnd",
        BinOp::BitOr => "BitOr",
        BinOp::BitXor => "BitXor",
        BinOp::Shl => "Shl",
        BinOp::Shr => "Shr",
    }
}
