//! The resolution dump: what the loader and the resolver built for one
//! program, as text, one entry per line, which the self-hosted resolver
//! (`selfhost/`) prints too, so that the two can be compared byte for byte
//! (spec
//! `docs/superpowers/specs/2026-11-02-phase-5-2-the-loader-and-the-resolver-design.md`
//! §3.4; ADR 0053 writes the grammar out, with an example of every form).
//! It carries no stability promise: it changes whenever resolution does, on
//! both sides in one pull request. `nova check --dump-resolution FILE…`
//! prints a program's part after a `== FILE` header, and `nova check
//! --dump-resolution --std` std's part; `--target wasm` resolves for the
//! WASM target, and `--test` as `nova test` resolves (ADR 0054).
//!
//! - It follows the AST dump's conventions (ADR 0052 §1): a line is its
//!   indentation, two spaces per depth, `(` and its kind, then its values
//!   and its fields `key=value`, each after one space; its children follow,
//!   one level deeper, and its `)` closes it at the end of its last line.
//!   Strings are quoted as the AST dump quotes them ([`quote`]), messages
//!   included; a span is `start..end`; a place, a span in a file, is the
//!   file's label quoted, `:` and the span.
//! - **Two parts.** std's modules come after the user's, and std's
//!   resolution is the same in every program for a target, so it is
//!   printed once, by [`dump_std`], and a program's part, by
//!   [`dump_program`], prints only what is the program's own: its
//!   modules, its definitions, its modules' namespaces and exports, its
//!   items and splits, its tests and every diagnostic.
//! - **Ids.** A program's own definition is `u<n>`, its `DefId`; a std
//!   definition is `s<n>`, its `DefId` less the first std definition's, so
//!   std's part prints the same ids whatever program it came from. A
//!   program's part says where std begins, `(StdStart module= def=
//!   item=)`. Modules and items are numbered from 0 in each part: std's
//!   part numbers std's modules and items from its first.
//! - **Namespaces** are printed sorted by name, values, then types, then
//!   traits; **exports** in the order they were exported, which a glob
//!   import follows (spec §3.1); `missing`, a target's, in std's part,
//!   sorted by name.

use std::fmt::Write;

use nova_ast::dump::quote;
use nova_diagnostics::{Diagnostic, FileId, Severity, Span};

use crate::{
    CompileTarget, DefId, DefKind, Definitions, ImportTarget, MethodOwner, ModuleSource,
    ProgramResolution, Res,
};

/// Every form the dump prints, by which the differential checks that std,
/// the corpus and the cases hold each of them
/// (`crates/nova-cli/tests/selfhost.rs`): a line's kind as `(Kind`, and a
/// field's word values as `key=Value` or, for a `bool`, after its line's
/// kind as `(Kind key=value` ([`forms_on`]). 5.2a's loose native programs
/// print the first 43; a package's module key, the WASM target with its
/// `missing` names, and test mode with its tests are 5.2b's (ADR 0054). An
/// import the loader reported, `(Import Reported`, is a loose program's
/// too: E0093 reports one, as a package's E0004 and noted E0001s do.
pub const KINDS: &[&str] = &[
    "(Program",
    "(Program resolved=true",
    "(Program resolved=false",
    "target=native",
    "(Module",
    "package=None",
    "(Import",
    "(Import Reported",
    "(Importable",
    "(StdStart",
    "(Def.Fn",
    "(Def.Sum",
    "(Variant",
    "(Def.Record",
    "(Def.Const",
    "(Def.Trait",
    "(Def.Method",
    "owner=Impl",
    "owner=TraitDefault",
    "(Def.ExternFn",
    "(Def.AssocType",
    "(Scope",
    "(Value.Def",
    "(Value.Variant",
    "(Value.Builtin",
    "(Type",
    "(Trait",
    "(Exports",
    "(Items",
    "(Split",
    "kind=Call",
    "kind=Index",
    "kind=Minus",
    "kind=Value",
    "(Std",
    "(Diagnostics",
    "(Error",
    "(Warning",
    "(Primary",
    "(Secondary",
    "(Note",
    "(Fix",
    "(Edit",
    // 5.2b (ADR 0054).
    "package=<n>",
    "target=wasm",
    "(Missing",
    "mode=test",
    "(Test",
    "(Test should_panic=true",
    "(Test should_panic=false",
];

/// The forms no program prints: the two severities neither the package
/// graph, the loader nor the resolver gives.
pub const LATER_KINDS: &[&str] = &["(Help", "(NoteDiagnostic"];

/// The label each file of a program is printed under: a user module's
/// label, and std's modules' names in the file database, as `<std/core>`.
pub type Labels = [(FileId, String)];

/// A program's part (module docs) for `target`, under test mode when
/// `test`: its `modules` as the loader gave them, the files' `labels`, its
/// resolution once it got that far, and every diagnostic, the package
/// graph's, the loader's and the resolver's, in order. Without a
/// resolution the part holds the modules loaded and the diagnostics.
pub fn dump_program(
    target: CompileTarget,
    test: bool,
    modules: &[ModuleSource],
    labels: &Labels,
    resolution: Option<&ProgramResolution>,
    diagnostics: &[Diagnostic],
) -> String {
    let mut d = Dump::new(labels, u32::MAX);
    d.open(0, "Program");
    d.field("target", target.name());
    d.mode(test);
    d.bool_field("resolved", resolution.is_some());
    for (index, m) in modules.iter().enumerate() {
        d.open(1, "Module");
        let _ = write!(d.out, " {index}");
        d.string(&m.name);
        d.field("package", &package(m.package));
        let mut imports: Vec<(&String, &ImportTarget)> = m.imports.iter().collect();
        imports.sort_by(|a, b| a.0.cmp(b.0));
        for (name, target) in imports {
            d.open(2, "Import");
            d.string(name);
            match target {
                ImportTarget::Module(t) => {
                    let _ = write!(d.out, " {t}");
                }
                ImportTarget::Reported => d.out.push_str(" Reported"),
            }
            d.close();
        }
        for (name, t) in &m.importable {
            d.open(2, "Importable");
            d.string(name);
            let _ = write!(d.out, " {t}");
            d.close();
        }
        d.close();
    }
    if let Some(r) = resolution {
        let defs = &r.definitions;
        let std_module = defs.std_start;
        let std_item = defs
            .item_module
            .iter()
            .position(|&m| m as usize >= std_module)
            .unwrap_or(defs.item_module.len());
        let std_def = first_std_def(defs, std_item);
        d.std_def = std_def;
        d.open(1, "StdStart");
        d.field("module", &std_module.to_string());
        d.field("def", &std_def.to_string());
        d.field("item", &std_item.to_string());
        d.close();
        d.defs(defs, 0..std_def, 0);
        d.scopes(defs, 0..std_module, 0);
        d.exports(defs, 0..std_module, 0);
        d.items(defs, 0..std_item, 0, 0);
        for split in &r.file.newline_splits {
            d.open(1, "Split");
            d.place(split.at);
            d.field("kind", &format!("{:?}", split.kind));
            d.close();
        }
        for test in &r.tests {
            d.open(1, "Test");
            let id = d.id(test.def_id);
            let _ = write!(d.out, " {id}");
            d.string(&test.name);
            d.bool_field("should_panic", test.should_panic);
            d.close();
        }
    }
    d.close();
    d.out.push('\n');
    d.diagnostics(diagnostics);
    d.out
}

/// std's part (module docs) for `target`, under test mode when `test`,
/// from the resolution of a program of no module of its own, whose std
/// files have `labels`.
pub fn dump_std(
    target: CompileTarget,
    test: bool,
    labels: &Labels,
    resolution: &ProgramResolution,
) -> String {
    let mut d = Dump::new(labels, 0);
    d.std_part(target, test, &resolution.definitions, 0, 0, 0);
    d.out.push('\n');
    d.diagnostics(&resolution.diagnostics);
    d.out
}

/// std's part of any program's `resolution`, without its diagnostics: the
/// same text as [`dump_std`]'s for a program of no module of its own, in
/// every program for one target, which a test checks.
pub fn std_part_of(
    target: CompileTarget,
    test: bool,
    labels: &Labels,
    resolution: &ProgramResolution,
) -> String {
    let defs = &resolution.definitions;
    let std_module = defs.std_start;
    let std_item = defs
        .item_module
        .iter()
        .position(|&m| m as usize >= std_module)
        .unwrap_or(defs.item_module.len());
    let std_def = first_std_def(defs, std_item);
    let mut d = Dump::new(labels, std_def);
    d.std_part(target, test, defs, std_module, std_def, std_item);
    d.out.push('\n');
    d.out
}

/// Where std begins in `defs`, as `(StdStart …)` prints it: the first std
/// module, the first std definition and the first item of a std module.
/// The HIR dump repeats it (5.3a, ADR 0055).
pub fn std_start(defs: &Definitions) -> (usize, u32, usize) {
    let std_module = defs.std_start;
    let std_item = defs
        .item_module
        .iter()
        .position(|&m| m as usize >= std_module)
        .unwrap_or(defs.item_module.len());
    (std_module, first_std_def(defs, std_item), std_item)
}

/// `diagnostics` as `(Diagnostics …)` prints them, after `out`, each place
/// under its file's label in `labels`: the HIR dump prints them so too
/// (5.3a, ADR 0055).
pub fn write_diagnostics(out: &mut String, labels: &Labels, diagnostics: &[Diagnostic]) {
    let mut d = Dump::new(labels, u32::MAX);
    d.out = std::mem::take(out);
    d.diagnostics(diagnostics);
    *out = d.out;
}

/// ` ` and a place, as the dump prints one, after `out`.
pub fn write_place(out: &mut String, labels: &Labels, span: Span) {
    let mut d = Dump::new(labels, u32::MAX);
    d.out = std::mem::take(out);
    d.place(span);
    *out = d.out;
}

/// The first std definition's index: the first definition of an item of a
/// std module. The user's definitions all come before std's, since pass 1
/// collects the modules in order, and every kind of definition but an
/// associated type names its item; an associated type follows its trait.
fn first_std_def(defs: &Definitions, std_item: usize) -> u32 {
    let mut first = defs.defs.len();
    for (i, def) in defs.defs.iter().enumerate() {
        let item = match def.kind {
            DefKind::Fn { item_index }
            | DefKind::Sum { item_index, .. }
            | DefKind::Record { item_index }
            | DefKind::Const { item_index }
            | DefKind::Trait { item_index }
            | DefKind::Method { item_index, .. }
            | DefKind::ExternFn { item_index, .. } => item_index,
            DefKind::AssocType { .. } => continue,
        };
        if item >= std_item {
            first = i;
            break;
        }
    }
    first as u32
}

/// The forms of [`KINDS`] and [`LATER_KINDS`] that `line` of a dump
/// prints: its kind, `(Kind`; each field outside quotes, `key=value`, but a
/// number's, as `package=<n>`, or a `bool`'s, after the kind, as `(Kind
/// key=value`; and an import the loader reported, `(Import Reported`.
/// Fields whose values are numbers, as `item=3`, come back too, as
/// `item=<n>`, and those whose values are ids as `trait=<id>`.
pub fn forms_on(line: &str) -> Vec<String> {
    let mut found = Vec::new();
    let node = line.trim_start();
    let Some(rest) = node.strip_prefix('(') else {
        return found;
    };
    let kind: String = rest
        .chars()
        .take_while(|c| *c != ' ' && *c != ')')
        .collect();
    found.push(format!("({kind}"));
    // Words, split at the spaces outside quotes: a quote inside a string is
    // written `\x22`, so each `"` opens or closes one.
    let mut quoted = false;
    let mut word = String::new();
    for c in node.chars().chain([' ']) {
        match c {
            '"' => {
                quoted = !quoted;
                word.push(c);
            }
            ' ' if !quoted => {
                let w = word.trim_end_matches(')');
                match w.split_once('=') {
                    Some((key, value @ ("true" | "false"))) => {
                        found.push(format!("({kind} {key}={value}"));
                    }
                    Some((key, value))
                        if !value.is_empty() && value.bytes().all(|b| b.is_ascii_digit()) =>
                    {
                        found.push(format!("{key}=<n>"));
                    }
                    Some((key, value))
                        if value.len() > 1
                            && value.starts_with(['u', 's'])
                            && value[1..].bytes().all(|b| b.is_ascii_digit()) =>
                    {
                        found.push(format!("{key}=<id>"));
                    }
                    Some(_) if !w.contains('"') => found.push(w.to_string()),
                    _ if kind == "Import" && w == "Reported" => {
                        found.push("(Import Reported".to_string());
                    }
                    _ => {}
                }
                word.clear();
            }
            _ => word.push(c),
        }
    }
    found
}

/// Whether `form`, from [`forms_on`], is one of [`KINDS`] or
/// [`LATER_KINDS`], or a field whose value is a number or an id, which no
/// list holds but `package=<n>`: the differential checks that every form a
/// dump prints is listed, so a form added to the dump is added to a list.
pub fn listed(form: &str) -> bool {
    form.ends_with("=<n>")
        || form.ends_with("=<id>")
        || KINDS.contains(&form)
        || LATER_KINDS.contains(&form)
}

/// A package key as the dump prints it.
fn package(key: Option<u32>) -> String {
    match key {
        None => "None".to_string(),
        Some(k) => k.to_string(),
    }
}

/// The line kind of a definition, one per [`DefKind`] variant.
pub fn def_kind(kind: &DefKind) -> &'static str {
    match kind {
        DefKind::Fn { .. } => "Def.Fn",
        DefKind::Sum { .. } => "Def.Sum",
        DefKind::Record { .. } => "Def.Record",
        DefKind::Const { .. } => "Def.Const",
        DefKind::Trait { .. } => "Def.Trait",
        DefKind::Method { .. } => "Def.Method",
        DefKind::ExternFn { .. } => "Def.ExternFn",
        DefKind::AssocType { .. } => "Def.AssocType",
    }
}

/// The line kind of a value's resolution, one per [`Res`] variant.
pub fn res_kind(res: &Res) -> &'static str {
    match res {
        Res::Def(_) => "Value.Def",
        Res::Variant(..) => "Value.Variant",
        Res::Builtin(_) => "Value.Builtin",
    }
}

/// The line kind of a diagnostic, one per [`Severity`] variant. A
/// diagnostic of severity `Note` is `NoteDiagnostic`, apart from a
/// diagnostic's own note, `Note`.
pub fn severity_kind(severity: Severity) -> &'static str {
    match severity {
        Severity::Error => "Error",
        Severity::Warning => "Warning",
        Severity::Note => "NoteDiagnostic",
        Severity::Help => "Help",
    }
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

    /// ` mode=test` under test mode, and nothing under `check`'s, so that
    /// 5.2a's dumps are as they were.
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

    /// ` ` and a place: the span's file's label quoted, `:`, the span. A
    /// file with no label is `?`.
    fn place(&mut self, span: Span) {
        self.out.push(' ');
        match self.labels.iter().find(|(f, _)| *f == span.file) {
            Some((_, label)) => quote(&mut self.out, label),
            None => self.out.push('?'),
        }
        let _ = write!(self.out, ":{}..{}", span.start, span.end);
    }

    /// A definition's id: `u<n>` for the program's own, `s<n>` for std's.
    fn id(&self, id: DefId) -> String {
        if id.0 >= self.std_def {
            format!("s{}", id.0 - self.std_def)
        } else {
            format!("u{}", id.0)
        }
    }

    // --- The parts ---

    /// std's part: its modules, definitions, namespaces, exports, items
    /// and `missing`, each numbered from std's first.
    fn std_part(
        &mut self,
        target: CompileTarget,
        test: bool,
        defs: &Definitions,
        std_module: usize,
        std_def: u32,
        std_item: usize,
    ) {
        self.open(0, "Std");
        self.field("target", target.name());
        self.mode(test);
        let modules = defs.modules.len() - std_module;
        self.field("modules", &modules.to_string());
        self.field("defs", &(defs.defs.len() as u32 - std_def).to_string());
        self.field("items", &(defs.item_module.len() - std_item).to_string());
        for (index, m) in defs.modules[std_module..].iter().enumerate() {
            self.open(1, "Module");
            let _ = write!(self.out, " {index}");
            self.string(&m.name);
            self.field("package", &package(defs.packages[std_module + index]));
            for (name, t) in &defs.importable[std_module + index] {
                self.open(2, "Importable");
                self.string(name);
                let _ = write!(self.out, " {}", t.0 as usize - std_module);
                self.close();
            }
            self.close();
        }
        self.defs(defs, std_def..defs.defs.len() as u32, std_item);
        self.scopes(defs, std_module..defs.modules.len(), std_module);
        self.exports(defs, std_module..defs.modules.len(), std_module);
        self.items(defs, std_item..defs.item_module.len(), std_item, std_module);
        let mut missing: Vec<(&String, &&str)> = defs.missing.iter().collect();
        missing.sort();
        for (name, module) in missing {
            self.open(1, "Missing");
            self.string(name);
            self.string(module);
            self.close();
        }
        self.close();
    }

    /// The definitions `range`, their items numbered from `first_item`.
    fn defs(&mut self, defs: &Definitions, range: std::ops::Range<u32>, first_item: usize) {
        for i in range {
            let def = &defs.defs[i as usize];
            self.open(1, def_kind(&def.kind));
            let id = self.id(DefId(i));
            let _ = write!(self.out, " {id}");
            self.string(&def.name);
            self.place(def.span);
            let item = |index: usize| (index - first_item).to_string();
            match &def.kind {
                DefKind::Fn { item_index }
                | DefKind::Record { item_index }
                | DefKind::Const { item_index }
                | DefKind::Trait { item_index } => self.field("item", &item(*item_index)),
                DefKind::Sum {
                    item_index,
                    variants,
                } => {
                    self.field("item", &item(*item_index));
                    for v in variants {
                        self.open(2, "Variant");
                        self.string(&v.name);
                        self.place(v.span);
                        self.field("arity", &v.arity.to_string());
                        self.close();
                    }
                }
                DefKind::Method {
                    item_index,
                    method_index,
                    owner,
                } => {
                    self.field("item", &item(*item_index));
                    self.field("method", &method_index.to_string());
                    let owner = match owner {
                        MethodOwner::Impl => "Impl",
                        MethodOwner::TraitDefault => "TraitDefault",
                    };
                    self.field("owner", owner);
                }
                DefKind::ExternFn {
                    item_index,
                    fn_index,
                } => {
                    self.field("item", &item(*item_index));
                    self.field("fn", &fn_index.to_string());
                }
                DefKind::AssocType { trait_def } => {
                    let t = self.id(*trait_def);
                    self.field("trait", &t);
                }
            }
            self.close();
        }
    }

    /// Each module of `range`'s three namespaces, sorted by name, the
    /// modules numbered from `first`.
    fn scopes(&mut self, defs: &Definitions, range: std::ops::Range<usize>, first: usize) {
        for mid in range {
            let scope = &defs.modules[mid];
            self.open(1, "Scope");
            let _ = write!(self.out, " {}", mid - first);
            let mut values: Vec<(&String, &Res)> = scope.values.iter().collect();
            values.sort_by(|a, b| a.0.cmp(b.0));
            for (name, res) in values {
                self.value(name, *res);
            }
            let mut types: Vec<(&String, &DefId)> = scope.types.iter().collect();
            types.sort_by(|a, b| a.0.cmp(b.0));
            for (name, id) in types {
                self.binding("Type", name, *id);
            }
            let mut traits: Vec<(&String, &DefId)> = scope.traits.iter().collect();
            traits.sort_by(|a, b| a.0.cmp(b.0));
            for (name, id) in traits {
                self.binding("Trait", name, *id);
            }
            self.close();
        }
    }

    /// Each module of `range`'s exports, in the order they were exported:
    /// values, then types, then traits.
    fn exports(&mut self, defs: &Definitions, range: std::ops::Range<usize>, first: usize) {
        for mid in range {
            let e = &defs.exports[mid];
            self.open(1, "Exports");
            let _ = write!(self.out, " {}", mid - first);
            for (name, res) in e.values_in_order() {
                self.value(&name, res);
            }
            for (name, id) in e.types_in_order() {
                self.binding("Type", &name, id);
            }
            for (name, id) in e.traits_in_order() {
                self.binding("Trait", &name, id);
            }
            self.close();
        }
    }

    /// A value's line: its name and what it resolves to.
    fn value(&mut self, name: &str, res: Res) {
        self.open(2, res_kind(&res));
        self.string(name);
        match res {
            Res::Def(id) => {
                let id = self.id(id);
                let _ = write!(self.out, " {id}");
            }
            Res::Variant(sum, index) => {
                let sum = self.id(sum);
                let _ = write!(self.out, " {sum} {index}");
            }
            Res::Builtin(b) => {
                let _ = write!(self.out, " {b:?}");
            }
        }
        self.close();
    }

    /// A type's or a trait's line: its name and its definition.
    fn binding(&mut self, kind: &str, name: &str, id: DefId) {
        self.open(2, kind);
        self.string(name);
        let id = self.id(id);
        let _ = write!(self.out, " {id}");
        self.close();
    }

    /// The merged items `range` as runs of one module, `(Items start..end
    /// module)`, the items numbered from `first_item` and the modules from
    /// `first_module`.
    fn items(
        &mut self,
        defs: &Definitions,
        range: std::ops::Range<usize>,
        first_item: usize,
        first_module: usize,
    ) {
        let mut start = range.start;
        while start < range.end {
            let module = defs.item_module[start];
            let mut end = start + 1;
            while end < range.end && defs.item_module[end] == module {
                end += 1;
            }
            self.open(1, "Items");
            let _ = write!(
                self.out,
                " {}..{} {}",
                start - first_item,
                end - first_item,
                module as usize - first_module
            );
            self.close();
            start = end;
        }
    }

    /// `(Diagnostics`, each diagnostic in order, `)`: its severity's kind,
    /// code and message, then its labels in order, notes and fixes.
    fn diagnostics(&mut self, diagnostics: &[Diagnostic]) {
        self.open(0, "Diagnostics");
        for diag in diagnostics {
            self.open(1, severity_kind(diag.severity));
            self.string(&diag.code);
            self.string(&diag.message);
            for label in &diag.labels {
                self.open(
                    2,
                    if label.primary {
                        "Primary"
                    } else {
                        "Secondary"
                    },
                );
                self.place(label.span);
                self.string(&label.message);
                self.close();
            }
            for note in &diag.notes {
                self.open(2, "Note");
                self.string(note);
                self.close();
            }
            for fix in &diag.fixes {
                self.open(2, "Fix");
                self.string(&fix.title);
                for edit in &fix.edits {
                    self.open(3, "Edit");
                    self.place(edit.span);
                    self.string(&edit.text);
                    self.close();
                }
                self.close();
            }
            self.close();
        }
        self.close();
        self.out.push('\n');
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{name_imports, resolve_program_for, STD_MODULES};
    use nova_diagnostics::FileDb;

    /// A program whose two modules print every form of [`KINDS`]: the
    /// loader's imports and what each module could import, every kind of
    /// definition, every kind of binding, exports, items and the four
    /// kinds of split, and diagnostics with secondary labels, notes and a
    /// fix.
    const MAIN: &str = "import lib\nimport lib::{secret}\nimport nosuch\nimport dom\n\
        import lib as l\nimport a::b\n\
        type Name = String\nrecord Int { x: Int }\n\
        @tset\nfn main() {\n    g\n    (1)\n    g\n    [1]\n    g\n    -1\n}\n\
        @test\nrecord R {}\n@bogus\npub const K: Int = 1\n\
        pub type Shade = | Red | Ok(Int)\n\
        trait Show {\n    type Item\n    fn twice(self) {}\n}\n\
        impl Show for R {\n    fn twice(self) {}\n}\n\
        extern \"C\" {\n    fn abs(x: Int) -> Int\n}\n\
        fn println() {}\nfn dup() {}\nfn dup() {}\nfn alpha() -> Int { 0 }\n\
        fn back() -> Int {\n    return\n    1\n}\n";
    const LIB: &str = "import lib\npub fn alpha() -> Int { 1 }\nfn secret() -> Int { 2 }\n\
        pub record P {}\npub trait T {}\n";

    /// Parse `src` as `file`.
    fn parse(src: &str, file: FileId) -> nova_ast::File {
        let (tokens, _) = nova_lexer::lex(src, file);
        let parsed = nova_parser::parse_recovering(&tokens, file);
        assert!(parsed.errors.is_empty(), "{:?}", parsed.errors);
        parsed.file
    }

    /// The std modules registered in `db` as the driver registers them.
    fn std_files(db: &mut FileDb) -> (Vec<FileId>, Vec<(FileId, String)>) {
        let mut ids = Vec::new();
        let mut labels = Vec::new();
        for &(name, src) in &STD_MODULES {
            let label = format!("<std/{}>", name.strip_prefix("$std.").unwrap_or(name));
            let id = db.add(label.clone(), src);
            ids.push(id);
            labels.push((id, label));
        }
        (ids, labels)
    }

    /// The program part of [`MAIN`] and [`LIB`], and std's part of the same
    /// resolution.
    fn the_program() -> (String, String, Vec<(FileId, String)>, Vec<FileId>) {
        let mut db = FileDb::new();
        let main_id = db.add("main.nova", MAIN);
        let lib_id = db.add("lib.nova", LIB);
        let (std_ids, mut labels) = std_files(&mut db);
        let main = parse(MAIN, main_id);
        let lib = parse(LIB, lib_id);
        let mut sources = [
            ModuleSource::new("main", &main),
            ModuleSource::new("lib", &lib),
        ];
        name_imports(&mut sources);
        sources[0].text = Some(MAIN);
        sources[1].text = Some(LIB);
        let r = resolve_program_for(CompileTarget::Native, &sources, &std_ids, None);
        let std_labels = labels.clone();
        labels.push((main_id, "main".to_string()));
        labels.push((lib_id, "lib".to_string()));
        let program = dump_program(
            CompileTarget::Native,
            false,
            &sources,
            &labels,
            Some(&r),
            &r.diagnostics,
        );
        let std = std_part_of(CompileTarget::Native, false, &labels, &r);
        (program, std, std_labels, std_ids)
    }

    /// The expected program part, `dump_test_program.txt`, as the tests
    /// read it: no `.gitattributes` rule covers it, so a Windows checkout
    /// with git's `core.autocrlf` gives it `\r\n` line endings, which are
    /// read as `\n`.
    const WANT_PROGRAM: &str = include_str!("dump_test_program.txt");

    /// Assert that `program`'s part, all but its modules' bindings of std's
    /// names and of the builtins, which std's part and the builtin table
    /// pin, is `want`, with each `\r\n` in `want` read as `\n`.
    fn assert_own_part(program: &str, want: &str) {
        let own: Vec<&str> = program
            .lines()
            .filter(|l| {
                let binding = ["    (Value.", "    (Type ", "    (Trait "]
                    .iter()
                    .any(|p| l.starts_with(p));
                !(binding && (l.contains("(Value.Builtin") || l.contains(" s")))
            })
            .collect();
        assert_eq!(own.join("\n") + "\n", want.replace("\r\n", "\n"));
    }

    /// The program's part, all but its modules' bindings of std's names and
    /// of the builtins.
    #[test]
    fn a_programs_part_prints_its_modules_definitions_items_and_diagnostics() {
        let (program, _, _, _) = the_program();
        assert_own_part(&program, WANT_PROGRAM);
    }

    /// The same, with the expected part as a Windows checkout has it.
    #[test]
    fn the_expected_program_part_may_have_windows_line_endings() {
        let (program, _, _, _) = the_program();
        let crlf = WANT_PROGRAM.replace("\r\n", "\n").replace('\n', "\r\n");
        assert!(crlf.contains("\r\n"));
        assert_own_part(&program, &crlf);
    }

    /// Every form of [`KINDS`] is in the program's part, its std's part, or
    /// the part of a program that stopped before the resolver, whose import
    /// the loader reported; and every form they print is listed.
    #[test]
    fn the_program_prints_every_form() {
        let (program, std, _, _) = the_program();
        let mut db = FileDb::new();
        let id = db.add("main.nova", "import ui\n");
        let tree = parse("import ui\n", id);
        let mut module = ModuleSource::new("main", &tree);
        module
            .imports
            .insert("ui".to_string(), ImportTarget::Reported);
        let labels = [(id, "main".to_string())];
        let stopped = dump_program(CompileTarget::Native, false, &[module], &labels, None, &[]);
        assert_eq!(
            stopped,
            "(Program target=native resolved=false\n  (Module 0 \"main\" package=None\n    \
             (Import \"ui\" Reported)))\n(Diagnostics)\n"
        );
        let later = the_5_2b_forms();
        let mut found = std::collections::BTreeSet::new();
        for line in program
            .lines()
            .chain(std.lines())
            .chain(stopped.lines())
            .chain(later.lines())
        {
            found.extend(forms_on(line));
        }
        let missing: Vec<&&str> = KINDS.iter().filter(|k| !found.contains(**k)).collect();
        assert!(missing.is_empty(), "{missing:?}");
        let unlisted: Vec<&String> = found.iter().filter(|f| !listed(f)).collect();
        assert!(unlisted.is_empty(), "{unlisted:?}");
    }

    /// What only 5.2b's programs print (ADR 0054): a package's module keys,
    /// in a test-mode program whose module keeps two tests, then std's part
    /// for the WASM target, with its `missing` names.
    fn the_5_2b_forms() -> String {
        let src = "@test\nfn plain() {}\n@test(should_panic)\nfn panics() {}\n";
        let mut db = FileDb::new();
        let main_id = db.add("main.nova", src);
        let (mut std_ids, mut labels) = std_files(&mut db);
        let (name, text) = crate::STD_TEST_MODULE;
        let test_id = db.add("<std/test>", text);
        labels.push((test_id, "<std/test>".to_string()));
        let main = parse(src, main_id);
        let mut module = ModuleSource::new("main", &main);
        module.package = Some(0);
        let r = resolve_program_for(
            CompileTarget::Native,
            std::slice::from_ref(&module),
            &std_ids,
            Some(((name, text), test_id)),
        );
        labels.push((main_id, "main".to_string()));
        let mut out = dump_program(
            CompileTarget::Native,
            true,
            &[module],
            &labels,
            Some(&r),
            &r.diagnostics,
        );
        assert!(
            out.starts_with(
                "(Program target=native mode=test resolved=true\n  (Module 0 \"main\" package=0)"
            ),
            "{out}"
        );
        std_ids.clear();
        let mut db = FileDb::new();
        let mut labels = Vec::new();
        for &(name, src) in &crate::WASM_STD_MODULES {
            let label = format!("<std/{}>", name.strip_prefix("$std.").unwrap_or(name));
            let id = db.add(label.clone(), src);
            std_ids.push(id);
            labels.push((id, label));
        }
        let wasm = resolve_program_for(CompileTarget::Wasm, &[], &std_ids, None);
        out.push_str(&dump_std(CompileTarget::Wasm, false, &labels, &wasm));
        out
    }

    /// std's part of a program's resolution is std's part of a program of
    /// no module of its own: the user's modules change no std scope, and
    /// the ids, modules and items are std-relative.
    #[test]
    fn std_part_is_the_same_in_every_program() {
        let (_, std, std_labels, std_ids) = the_program();
        let alone = resolve_program_for(CompileTarget::Native, &[], &std_ids, None);
        assert_eq!(
            std,
            std_part_of(CompileTarget::Native, false, &std_labels, &alone)
        );
        let full = dump_std(CompileTarget::Native, false, &std_labels, &alone);
        assert_eq!(full, std.clone() + "(Diagnostics)\n");
        assert!(
            std.starts_with("(Std target=native modules=21 "),
            "{}",
            &std[..80]
        );
        assert!(!std.contains(" u"), "std's part names a user definition");
    }

    /// Under test mode too, std's part of a program's resolution is std's
    /// part of a program of no module of its own: `std/test` joins std,
    /// after its other modules, and its names are globbed into theirs, but
    /// a user module's are not.
    #[test]
    fn std_part_is_the_same_in_every_test_mode_program() {
        let src = "@test\nfn plain() {}\n";
        let mut db = FileDb::new();
        let main_id = db.add("main.nova", src);
        let (std_ids, mut labels) = std_files(&mut db);
        let (name, text) = crate::STD_TEST_MODULE;
        let test_id = db.add("<std/test>", text);
        labels.push((test_id, "<std/test>".to_string()));
        let extra = Some(((name, text), test_id));
        let main = parse(src, main_id);
        let module = ModuleSource::new("main", &main);
        let r = resolve_program_for(
            CompileTarget::Native,
            std::slice::from_ref(&module),
            &std_ids,
            extra,
        );
        let alone = resolve_program_for(CompileTarget::Native, &[], &std_ids, extra);
        let std = std_part_of(CompileTarget::Native, true, &labels, &r);
        assert_eq!(
            std,
            std_part_of(CompileTarget::Native, true, &labels, &alone)
        );
        let full = dump_std(CompileTarget::Native, true, &labels, &alone);
        assert_eq!(full, std.clone() + "(Diagnostics)\n");
        assert!(
            std.starts_with("(Std target=native mode=test modules=22 "),
            "{}",
            &std[..80]
        );
        assert!(std.contains("\n  (Module 21 \"$std.test\" package=None)"));
        assert!(!std.contains(" u"), "std's part names a user definition");
    }

    /// [`resolve_program_for`] is [`crate::parse_std`] then
    /// [`crate::resolve_program_with_std`], which the resolution dumps call
    /// with std parsed once for every root (ADR 0053 §9): one `ParsedStd`
    /// resolves two programs in turn, each with diagnostics, to the same
    /// merged file, definitions, diagnostics, tests and import occurrences
    /// as `resolve_program_for` gives each.
    #[test]
    fn parsing_std_once_resolves_as_resolve_program_for_does() {
        let mut db = FileDb::new();
        let main_id = db.add("main.nova", MAIN);
        let lib_id = db.add("lib.nova", LIB);
        let (std_ids, _) = std_files(&mut db);
        let (main, lib) = (parse(MAIN, main_id), parse(LIB, lib_id));
        let mut two = [
            ModuleSource::new("main", &main),
            ModuleSource::new("lib", &lib),
        ];
        name_imports(&mut two);
        let mut one = [ModuleSource::new("lib", &lib)];
        name_imports(&mut one);
        let std = crate::parse_std(CompileTarget::Native, &std_ids, None);
        for program in [&two[..], &one[..]] {
            let a = resolve_program_for(CompileTarget::Native, program, &std_ids, None);
            let b = crate::resolve_program_with_std(CompileTarget::Native, program, &std);
            assert!(!a.diagnostics.is_empty());
            assert_eq!(
                nova_ast::dump::dump_file(&a.file),
                nova_ast::dump::dump_file(&b.file)
            );
            for (x, y) in [
                (
                    format!("{:?}", a.definitions),
                    format!("{:?}", b.definitions),
                ),
                (
                    format!("{:?}", a.diagnostics),
                    format!("{:?}", b.diagnostics),
                ),
                (format!("{:?}", a.tests), format!("{:?}", b.tests)),
                (format!("{:?}", a.imports), format!("{:?}", b.imports)),
            ] {
                assert_eq!(x, y);
            }
        }
    }

    /// Each kind of definition, of value and of diagnostic has its form in
    /// [`KINDS`] or [`LATER_KINDS`].
    #[test]
    fn every_kind_of_definition_value_and_diagnostic_is_listed() {
        let listed = |k: &str| {
            let form = format!("({k}");
            KINDS.contains(&form.as_str()) || LATER_KINDS.contains(&form.as_str())
        };
        let item_index = 0;
        let kinds = [
            DefKind::Fn { item_index },
            DefKind::Sum {
                item_index,
                variants: Vec::new(),
            },
            DefKind::Record { item_index },
            DefKind::Const { item_index },
            DefKind::Trait { item_index },
            DefKind::Method {
                item_index,
                method_index: 0,
                owner: MethodOwner::Impl,
            },
            DefKind::ExternFn {
                item_index,
                fn_index: 0,
            },
            DefKind::AssocType {
                trait_def: DefId(0),
            },
        ];
        for kind in &kinds {
            assert!(listed(def_kind(kind)), "{kind:?}");
        }
        for res in [
            Res::Def(DefId(0)),
            Res::Variant(DefId(0), 0),
            Res::Builtin(crate::Builtin::Println),
        ] {
            assert!(listed(res_kind(&res)), "{res:?}");
        }
        for severity in [
            Severity::Error,
            Severity::Warning,
            Severity::Note,
            Severity::Help,
        ] {
            assert!(listed(severity_kind(severity)), "{severity:?}");
        }
    }

    /// std resolved once ([`crate::SharedStd`], 5.2b, ADR 0054 §9) is
    /// shared only when std alone gives no diagnostic and holds no `@test`
    /// function, either of which the shared path would drop: with a std
    /// whose `core` defines a name twice (E0002), or whose `ui` marks a
    /// function `@test`, each program is resolved in full, and its dump,
    /// under `check` and under test mode, is the full path's, std's E0002
    /// or std's test kept.
    #[test]
    fn std_with_a_diagnostic_or_a_test_is_resolved_for_each_program() {
        let alterations = [
            (
                "$std.core",
                "fn mix64(x: Int) -> Int {",
                "fn escape_common(x: Int) -> Int {",
                "(Error \"E0002\" \"duplicate definition of `escape_common`\"",
            ),
            (
                "$std.ui",
                "\nfn flush() {",
                "\n@test\nfn flush() {",
                " \"flush\" should_panic=false)",
            ),
        ];
        for (module, old, new, mark) in alterations {
            let mut db = FileDb::new();
            let main_id = db.add("main.nova", MAIN);
            let lib_id = db.add("lib.nova", LIB);
            let mut labels = vec![(main_id, "main".to_string()), (lib_id, "lib".to_string())];
            let mut texts = Vec::new();
            for &(name, src) in &STD_MODULES {
                let text = if name == module {
                    assert_eq!(src.matches(old).count(), 1, "{module}: {old}");
                    src.replacen(old, new, 1)
                } else {
                    src.to_string()
                };
                let label = format!("<std/{}>", name.strip_prefix("$std.").unwrap_or(name));
                let id = db.add(label.clone(), text.clone());
                labels.push((id, label));
                texts.push((name, text, id));
            }
            let mut diagnostics = Vec::new();
            let modules = texts
                .iter()
                .map(|(name, text, id)| {
                    let (file, diags) = crate::std_module(text, *id);
                    diagnostics.extend(diags);
                    (name.to_string(), file)
                })
                .collect();
            let parsed = crate::ParsedStd {
                modules,
                diagnostics,
            };
            let shared = crate::SharedStd::new(CompileTarget::Native, parsed);
            assert!(!shared.shareable, "{module}");
            let main = parse(MAIN, main_id);
            let lib = parse(LIB, lib_id);
            let mut sources = [
                ModuleSource::new("main", &main),
                ModuleSource::new("lib", &lib),
            ];
            name_imports(&mut sources);
            sources[0].text = Some(MAIN);
            sources[1].text = Some(LIB);
            let full =
                crate::resolve_program_with_std(CompileTarget::Native, &sources, shared.parsed());
            let once = crate::resolve_program_with_shared_std(&sources, &shared);
            for test in [false, true] {
                let dump = |r: &crate::ProgramResolution| {
                    dump_program(
                        CompileTarget::Native,
                        test,
                        &sources,
                        &labels,
                        Some(r),
                        &r.diagnostics,
                    )
                };
                let (want, got) = (dump(&full), dump(&once));
                assert!(want.contains(mark), "{module}: {want}");
                assert_eq!(got, want, "{module}, test mode {test}");
            }
        }
    }
}
