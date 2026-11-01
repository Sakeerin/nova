//! The AST dump: one file's tree as text, one node per line, which the
//! self-hosted parser (`selfhost/`) prints too, so that the two parsers
//! can be compared byte for byte (spec
//! `docs/superpowers/specs/2026-10-30-phase-5-1-the-lexer-the-parser-and-the-ast-design.md`
//! §4.3; ADR 0052 writes the grammar out, with an example of every kind).
//! It carries no stability promise: it changes whenever the tree does, on
//! both sides in one pull request. `nova_parser::dump_ast` adds the
//! errors after it, and `nova parse --dump-ast` a `== FILE` header before.
//!
//! - A node is a line: its indentation, two spaces per depth; `(` and its
//!   kind; then, each after one space, its span `start..end` if the tree
//!   holds one; its positional values; its fields `key=value`. Its
//!   children follow, one per line, one level deeper, and its `)` closes
//!   it at the end of its last line, after its last child's.
//! - The kinds: a struct's name, and an enum's value `Enum.Variant`, both
//!   as Rust names them, but for three the spec names: a `///` line is
//!   `Doc`, an `@attribute` `Attr` and a newline split `Split`.
//! - A `Spanned` node prints its span. An enum value that holds a struct,
//!   such as `Item::Function(Function)`, is a node whose one child is the
//!   struct's node, which has no span of its own.
//! - A struct's or a variant's named fields print in their order: the
//!   scalars on its line, `key=value` with the Rust field's name; the
//!   nodes as children. A tuple variant's scalars print on its line as
//!   positional values, without a key; so do a `Doc`'s text, an `Attr`'s
//!   name and arguments, and a `Path` node's segments.
//! - An absent `Option` prints nothing, and so does an empty list. The
//!   one `Option` whose absence a list's emptiness would hide, an
//!   element's children in a view, also prints `children=Some` or
//!   `children=None`.
//! - Values: a `bool` as `true` or `false`; a field-less enum (`Visibility`,
//!   `BinOp`, `UnOp`, `AssignOp`, `SplitKind`) as its variant's name; a
//!   string in double quotes, with each character below 0x20, 0x7F, `"` and
//!   `\` written `\xHH` (two lowercase hex digits) and every other as it
//!   is, as the token dump writes them; a name (`Spanned<String>`) as its
//!   quoted string, `@` and its span; a path as its segments' names joined
//!   by `::`; a literal as `Int:` and the value in decimal, `Float:` and
//!   the 16 lowercase hex digits of `f64::to_bits`, `Str:` and the quoted
//!   string, `Char:` and the code point in decimal, or `Bool:` and `true` or
//!   `false`.
//! - The tree is one `(File` node: its items, then its newline splits,
//!   `(Split start..end kind=Kind)`. The text ends with a line break.

use std::fmt::Write;

use nova_diagnostics::{Span, Spanned};

use crate::expr::{Expr, FieldInit, Literal, MatchArm, StringPart, ViewArg, ViewNode};
use crate::item::{
    AssocTypeBinding, Attribute, ConstDecl, ExternBlock, ExternItem, Function, FunctionSig,
    ImplBlock, Import, ImportKind, Module, Param, Record, RecordField, TraitDecl, TraitItem,
    TypeDecl, TypeDef, Variant, WhereBound,
};
use crate::pattern::{FieldPat, Pattern};
use crate::ty::{Type, TypeParam};
use crate::{Block, File, Item, Path, Stmt};

/// Every kind and value the dump can print, by which the differential
/// checks that the corpus and the cases hold each of them
/// (`crates/nova-cli/tests/selfhost.rs`): a node's kind as `(Kind`, a
/// field-less enum's value as `key=Value`, a literal's as `Int:`, and each
/// `bool` field's two values and `children=`'s two, after their node's
/// kind, as `(Kind key=value`. Every struct, enum variant and `bool` field
/// of the tree is here: `every_declared_form_is_listed` reads them from
/// this crate's sources and fails until a new one is listed.
pub const KINDS: &[&str] = &[
    // The 24 structs (`Attribute` is `Attr`, `NewlineSplit` is `Split`),
    // and the `///` lines.
    "(File",
    "(Split",
    "(Block",
    "(Path",
    "(TypeParam",
    "(FieldPat",
    "(MatchArm",
    "(FieldInit",
    "(Attr",
    "(Param",
    "(WhereBound",
    "(Function",
    "(Record",
    "(RecordField",
    "(TypeDecl",
    "(Variant",
    "(TraitDecl",
    "(FunctionSig",
    "(ImplBlock",
    "(AssocTypeBinding",
    "(ConstDecl",
    "(Import",
    "(Module",
    "(ExternBlock",
    "(Doc",
    // `Item`
    "(Item.Function",
    "(Item.Record",
    "(Item.Type",
    "(Item.Trait",
    "(Item.Impl",
    "(Item.Const",
    "(Item.Import",
    "(Item.Module",
    "(Item.Extern",
    // `Stmt`
    "(Stmt.Let",
    "(Stmt.Expr",
    "(Stmt.Item",
    // `Expr`
    "(Expr.Lit",
    "(Expr.Path",
    "(Expr.Tuple",
    "(Expr.Array",
    "(Expr.ArrayRepeat",
    "(Expr.Block",
    "(Expr.If",
    "(Expr.Match",
    "(Expr.While",
    "(Expr.For",
    "(Expr.Range",
    "(Expr.Return",
    "(Expr.Break",
    "(Expr.Continue",
    "(Expr.Closure",
    "(Expr.Record",
    "(Expr.Binary",
    "(Expr.Unary",
    "(Expr.Call",
    "(Expr.Index",
    "(Expr.Field",
    "(Expr.Try",
    "(Expr.Await",
    "(Expr.Cast",
    "(Expr.Assign",
    "(Expr.StringInterp",
    "(Expr.View",
    // `ViewNode`, `ViewArg`, `StringPart`
    "(ViewNode.Element",
    "(ViewNode.Expr",
    "(ViewArg.Named",
    "(ViewArg.Positional",
    "(StringPart.Lit",
    "(StringPart.Expr",
    // `Pattern`
    "(Pattern.Wildcard",
    "(Pattern.Lit",
    "(Pattern.Ident",
    "(Pattern.Binding",
    "(Pattern.TupleStruct",
    "(Pattern.Record",
    "(Pattern.Tuple",
    "(Pattern.Array",
    "(Pattern.Or",
    "(Pattern.Range",
    "(Pattern.Path",
    // `Type`
    "(Type.Path",
    "(Type.Ref",
    "(Type.Ptr",
    "(Type.Array",
    "(Type.Tuple",
    "(Type.Fn",
    "(Type.Optional",
    "(Type.Infer",
    // `TypeDef`, `TraitItem`, `ImportKind`, `ExternItem`
    "(TypeDef.Alias",
    "(TypeDef.Sum",
    "(TraitItem.Required",
    "(TraitItem.Provided",
    "(TraitItem.AssocType",
    "(ImportKind.Simple",
    "(ImportKind.Alias",
    "(ImportKind.List",
    "(ExternItem.Fn",
    // `Literal`
    "Int:",
    "Float:",
    "Str:",
    "Char:",
    "Bool:",
    // `Visibility`
    "vis=Pub",
    "vis=Private",
    // `SplitKind`
    "kind=Call",
    "kind=Index",
    "kind=Minus",
    "kind=Value",
    // `BinOp`
    "op=Or",
    "op=And",
    "op=Eq",
    "op=Ne",
    "op=Lt",
    "op=Le",
    "op=Gt",
    "op=Ge",
    "op=BitOr",
    "op=BitXor",
    "op=BitAnd",
    "op=Shl",
    "op=Shr",
    "op=Add",
    "op=Sub",
    "op=Mul",
    "op=Div",
    "op=Rem",
    // `UnOp`
    "op=Neg",
    "op=Not",
    "op=BitNot",
    "op=Ref",
    "op=RefMut",
    "op=Deref",
    // `AssignOp`
    "op=Assign",
    "op=AddAssign",
    "op=SubAssign",
    "op=MulAssign",
    "op=DivAssign",
    "op=RemAssign",
    "op=BitOrAssign",
    "op=BitAndAssign",
    "op=BitXorAssign",
    "op=ShlAssign",
    "op=ShrAssign",
    // The `bool` fields, and an element's `children`, whose absence the
    // dump prints (`Pattern.Ident`'s `is_mut` is never `true`: no parser
    // builds a `mut` binding inside a pattern).
    "(Stmt.Let is_mut=true",
    "(Stmt.Let is_mut=false",
    "(Param is_mut=true",
    "(Param is_mut=false",
    "(Function is_async=true",
    "(Function is_async=false",
    "(Function is_component=true",
    "(Function is_component=false",
    "(FunctionSig is_async=true",
    "(FunctionSig is_async=false",
    "(Expr.Range inclusive=true",
    "(Expr.Range inclusive=false",
    "(Pattern.Ident is_mut=false",
    "(Pattern.Record rest=true",
    "(Pattern.Record rest=false",
    "(Pattern.Range inclusive=true",
    "(Pattern.Range inclusive=false",
    "(Type.Ref is_mut=true",
    "(Type.Ref is_mut=false",
    "(Type.Ptr is_mut=true",
    "(Type.Ptr is_mut=false",
    "(ViewNode.Element children=Some",
    "(ViewNode.Element children=None",
];

/// The dump of `file`'s tree (module docs).
pub fn dump_file(file: &File) -> String {
    let mut d = Dump::default();
    d.open(0, "File", None);
    for item in &file.items {
        d.item(1, Some(item.span), &item.value);
    }
    for split in &file.newline_splits {
        d.open(1, "Split", Some(split.at));
        d.field("kind", format_args!("{:?}", split.kind));
        d.close();
    }
    d.close();
    d.out.push('\n');
    d.out
}

/// `text` in double quotes, with each character below 0x20, 0x7F, `"` and
/// `\` as `\xHH` and every other as it is: the token dump's rule
/// (`nova_lexer::dump_tokens`).
pub fn quote(out: &mut String, text: &str) {
    out.push('"');
    for c in text.chars() {
        if (c as u32) < 0x20 || c == '\x7f' || c == '"' || c == '\\' {
            let _ = write!(out, "\\x{:02x}", c as u32);
        } else {
            out.push(c);
        }
    }
    out.push('"');
}

/// An error's message or note, which is not quoted: each character below
/// 0x20 and 0x7F as `\xHH`, so that it stays on its line, and every other,
/// `"` and `\` included, as it is (the token dump's `ERR` rule).
pub fn escape_message(out: &mut String, text: &str) {
    for c in text.chars() {
        if (c as u32) < 0x20 || c == '\x7f' {
            let _ = write!(out, "\\x{:02x}", c as u32);
        } else {
            out.push(c);
        }
    }
}

#[derive(Default)]
struct Dump {
    out: String,
}

impl Dump {
    // --- Lines and values ---

    /// A node's line, up to its kind and span.
    fn open(&mut self, depth: usize, kind: &str, span: Option<Span>) {
        if !self.out.is_empty() {
            self.out.push('\n');
        }
        for _ in 0..depth {
            self.out.push_str("  ");
        }
        self.out.push('(');
        self.out.push_str(kind);
        if let Some(span) = span {
            let _ = write!(self.out, " {}..{}", span.start, span.end);
        }
    }

    fn close(&mut self) {
        self.out.push(')');
    }

    /// ` key=value`.
    fn field(&mut self, key: &str, value: std::fmt::Arguments) {
        let _ = write!(self.out, " {key}={value}");
    }

    fn bool_field(&mut self, key: &str, value: bool) {
        self.field(key, format_args!("{value}"));
    }

    /// ` key=` and the name.
    fn name_field(&mut self, key: &str, name: &Spanned<String>) {
        let _ = write!(self.out, " {key}=");
        self.name(name);
    }

    /// ` key=` and the path.
    fn path_field(&mut self, key: &str, path: &Path) {
        let _ = write!(self.out, " {key}=");
        self.path(path);
    }

    /// A name: quoted, `@`, its span.
    fn name(&mut self, name: &Spanned<String>) {
        quote(&mut self.out, &name.value);
        let _ = write!(self.out, "@{}..{}", name.span.start, name.span.end);
    }

    /// A path: its segments' names, joined by `::`.
    fn path(&mut self, path: &Path) {
        for (i, segment) in path.segments.iter().enumerate() {
            if i > 0 {
                self.out.push_str("::");
            }
            self.name(segment);
        }
    }

    /// ` ` and a positional string.
    fn text(&mut self, text: &str) {
        self.out.push(' ');
        quote(&mut self.out, text);
    }

    /// ` ` and a literal.
    fn literal(&mut self, lit: &Literal) {
        match lit {
            Literal::Int(n) => {
                let _ = write!(self.out, " Int:{n}");
            }
            Literal::Float(f) => {
                let _ = write!(self.out, " Float:{:016x}", f.to_bits());
            }
            Literal::Str(s) => {
                self.out.push_str(" Str:");
                quote(&mut self.out, s);
            }
            Literal::Char(c) => {
                let _ = write!(self.out, " Char:{}", *c as u32);
            }
            Literal::Bool(b) => {
                let _ = write!(self.out, " Bool:{b}");
            }
        }
    }

    // --- Shared children ---

    fn docs(&mut self, depth: usize, docs: &[Spanned<String>]) {
        for doc in docs {
            self.open(depth, "Doc", Some(doc.span));
            self.text(&doc.value);
            self.close();
        }
    }

    fn attrs(&mut self, depth: usize, attrs: &[Attribute]) {
        for attr in attrs {
            self.open(depth, "Attr", Some(attr.span));
            self.out.push(' ');
            self.name(&attr.name);
            for arg in &attr.args {
                self.out.push(' ');
                self.name(arg);
            }
            self.close();
        }
    }

    fn path_node(&mut self, depth: usize, path: &Spanned<Path>) {
        self.open(depth, "Path", Some(path.span));
        self.out.push(' ');
        self.path(&path.value);
        self.close();
    }

    fn paths(&mut self, depth: usize, paths: &[Spanned<Path>]) {
        for path in paths {
            self.path_node(depth, path);
        }
    }

    fn generics(&mut self, depth: usize, generics: &[TypeParam]) {
        for g in generics {
            self.open(depth, "TypeParam", None);
            self.name_field("name", &g.name);
            self.paths(depth + 1, &g.bounds);
            self.close();
        }
    }

    fn params(&mut self, depth: usize, params: &[Param]) {
        for p in params {
            self.open(depth, "Param", None);
            self.bool_field("is_mut", p.is_mut);
            self.name_field("name", &p.name);
            self.ty(depth + 1, &p.ty);
            self.close();
        }
    }

    fn where_clause(&mut self, depth: usize, bounds: &[WhereBound]) {
        for b in bounds {
            self.open(depth, "WhereBound", None);
            self.ty(depth + 1, &b.ty);
            self.paths(depth + 1, &b.bounds);
            self.close();
        }
    }

    // --- Items ---

    fn item(&mut self, depth: usize, span: Option<Span>, item: &Item) {
        let kind = match item {
            Item::Function(_) => "Item.Function",
            Item::Record(_) => "Item.Record",
            Item::Type(_) => "Item.Type",
            Item::Trait(_) => "Item.Trait",
            Item::Impl(_) => "Item.Impl",
            Item::Const(_) => "Item.Const",
            Item::Import(_) => "Item.Import",
            Item::Module(_) => "Item.Module",
            Item::Extern(_) => "Item.Extern",
        };
        self.open(depth, kind, span);
        let depth = depth + 1;
        match item {
            Item::Function(f) => self.function(depth, f),
            Item::Record(r) => self.record(depth, r),
            Item::Type(t) => self.type_decl(depth, t),
            Item::Trait(t) => self.trait_decl(depth, t),
            Item::Impl(i) => self.impl_block(depth, i),
            Item::Const(c) => self.const_decl(depth, c),
            Item::Import(i) => self.import(depth, i),
            Item::Module(m) => self.module(depth, m),
            Item::Extern(e) => self.extern_block(depth, e),
        }
        self.close();
    }

    fn function(&mut self, depth: usize, f: &Function) {
        self.open(depth, "Function", None);
        self.field("vis", format_args!("{:?}", f.vis));
        self.bool_field("is_async", f.is_async);
        self.bool_field("is_component", f.is_component);
        self.name_field("name", &f.name);
        let depth = depth + 1;
        self.docs(depth, &f.docs);
        self.attrs(depth, &f.attrs);
        self.generics(depth, &f.generics);
        self.params(depth, &f.params);
        if let Some(t) = &f.return_ty {
            self.ty(depth, t);
        }
        self.where_clause(depth, &f.where_clause);
        self.block(depth, Some(f.body.span), &f.body.value);
        self.close();
    }

    fn function_sig(&mut self, depth: usize, f: &FunctionSig) {
        self.open(depth, "FunctionSig", None);
        self.bool_field("is_async", f.is_async);
        self.name_field("name", &f.name);
        let depth = depth + 1;
        self.docs(depth, &f.docs);
        self.generics(depth, &f.generics);
        self.params(depth, &f.params);
        if let Some(t) = &f.return_ty {
            self.ty(depth, t);
        }
        self.where_clause(depth, &f.where_clause);
        self.close();
    }

    fn record(&mut self, depth: usize, r: &Record) {
        self.open(depth, "Record", None);
        self.field("vis", format_args!("{:?}", r.vis));
        self.name_field("name", &r.name);
        let depth = depth + 1;
        self.docs(depth, &r.docs);
        self.attrs(depth, &r.attrs);
        self.generics(depth, &r.generics);
        for f in &r.fields {
            self.record_field(depth, f);
        }
        self.close();
    }

    fn record_field(&mut self, depth: usize, f: &RecordField) {
        self.open(depth, "RecordField", None);
        self.field("vis", format_args!("{:?}", f.vis));
        self.name_field("name", &f.name);
        self.docs(depth + 1, &f.docs);
        self.ty(depth + 1, &f.ty);
        self.close();
    }

    fn type_decl(&mut self, depth: usize, t: &TypeDecl) {
        self.open(depth, "TypeDecl", None);
        self.field("vis", format_args!("{:?}", t.vis));
        self.name_field("name", &t.name);
        let depth = depth + 1;
        self.docs(depth, &t.docs);
        self.attrs(depth, &t.attrs);
        self.generics(depth, &t.generics);
        match &t.def {
            TypeDef::Alias(ty) => {
                self.open(depth, "TypeDef.Alias", None);
                self.ty(depth + 1, ty);
                self.close();
            }
            TypeDef::Sum(variants) => {
                self.open(depth, "TypeDef.Sum", None);
                for v in variants {
                    self.variant(depth + 1, v);
                }
                self.close();
            }
        }
        self.close();
    }

    fn variant(&mut self, depth: usize, v: &Variant) {
        self.open(depth, "Variant", None);
        self.name_field("name", &v.name);
        self.docs(depth + 1, &v.docs);
        for t in &v.fields {
            self.ty(depth + 1, t);
        }
        self.close();
    }

    fn trait_decl(&mut self, depth: usize, t: &TraitDecl) {
        self.open(depth, "TraitDecl", None);
        self.field("vis", format_args!("{:?}", t.vis));
        self.name_field("name", &t.name);
        let depth = depth + 1;
        self.docs(depth, &t.docs);
        self.attrs(depth, &t.attrs);
        self.generics(depth, &t.generics);
        self.paths(depth, &t.supertraits);
        self.where_clause(depth, &t.where_clause);
        for item in &t.items {
            match item {
                TraitItem::Required(sig) => {
                    self.open(depth, "TraitItem.Required", None);
                    self.function_sig(depth + 1, sig);
                }
                TraitItem::Provided(f) => {
                    self.open(depth, "TraitItem.Provided", None);
                    self.function(depth + 1, f);
                }
                TraitItem::AssocType { docs, name, bounds } => {
                    self.open(depth, "TraitItem.AssocType", None);
                    self.name_field("name", name);
                    self.docs(depth + 1, docs);
                    self.paths(depth + 1, bounds);
                }
            }
            self.close();
        }
        self.close();
    }

    fn impl_block(&mut self, depth: usize, i: &ImplBlock) {
        self.open(depth, "ImplBlock", None);
        let depth = depth + 1;
        self.docs(depth, &i.docs);
        self.attrs(depth, &i.attrs);
        self.generics(depth, &i.generics);
        if let Some(t) = &i.trait_ {
            self.path_node(depth, t);
        }
        self.ty(depth, &i.ty);
        self.where_clause(depth, &i.where_clause);
        for f in &i.functions {
            self.function(depth, f);
        }
        for c in &i.consts {
            self.const_decl(depth, c);
        }
        for a in &i.assoc_types {
            self.assoc_type_binding(depth, a);
        }
        self.close();
    }

    fn assoc_type_binding(&mut self, depth: usize, a: &AssocTypeBinding) {
        self.open(depth, "AssocTypeBinding", None);
        self.name_field("name", &a.name);
        self.docs(depth + 1, &a.docs);
        self.ty(depth + 1, &a.ty);
        self.close();
    }

    fn const_decl(&mut self, depth: usize, c: &ConstDecl) {
        self.open(depth, "ConstDecl", None);
        self.field("vis", format_args!("{:?}", c.vis));
        self.name_field("name", &c.name);
        let depth = depth + 1;
        self.docs(depth, &c.docs);
        self.attrs(depth, &c.attrs);
        self.ty(depth, &c.ty);
        self.expr(depth, &c.value);
        self.close();
    }

    fn import(&mut self, depth: usize, i: &Import) {
        self.open(depth, "Import", None);
        let depth = depth + 1;
        self.docs(depth, &i.docs);
        self.attrs(depth, &i.attrs);
        self.path_node(depth, &i.path);
        match &i.kind {
            ImportKind::Simple => self.open(depth, "ImportKind.Simple", None),
            ImportKind::Alias(name) => {
                self.open(depth, "ImportKind.Alias", None);
                self.out.push(' ');
                self.name(name);
            }
            ImportKind::List(names) => {
                self.open(depth, "ImportKind.List", None);
                for name in names {
                    self.out.push(' ');
                    self.name(name);
                }
            }
        }
        self.close();
        self.close();
    }

    fn module(&mut self, depth: usize, m: &Module) {
        self.open(depth, "Module", None);
        self.docs(depth + 1, &m.docs);
        self.attrs(depth + 1, &m.attrs);
        self.path_node(depth + 1, &m.path);
        self.close();
    }

    fn extern_block(&mut self, depth: usize, e: &ExternBlock) {
        self.open(depth, "ExternBlock", None);
        if let Some(abi) = &e.abi {
            self.out.push_str(" abi=");
            quote(&mut self.out, abi);
        }
        let depth = depth + 1;
        self.docs(depth, &e.docs);
        self.attrs(depth, &e.attrs);
        for item in &e.items {
            match item {
                ExternItem::Fn(sig) => {
                    self.open(depth, "ExternItem.Fn", None);
                    self.function_sig(depth + 1, sig);
                    self.close();
                }
            }
        }
        self.close();
    }

    // --- Blocks and statements ---

    fn block(&mut self, depth: usize, span: Option<Span>, b: &Block) {
        self.open(depth, "Block", span);
        for s in &b.stmts {
            self.stmt(depth + 1, s);
        }
        if let Some(t) = &b.trailing {
            self.expr(depth + 1, t);
        }
        self.close();
    }

    fn stmt(&mut self, depth: usize, s: &Spanned<Stmt>) {
        match &s.value {
            Stmt::Let {
                is_mut,
                pattern,
                ty,
                init,
            } => {
                self.open(depth, "Stmt.Let", Some(s.span));
                self.bool_field("is_mut", *is_mut);
                self.pattern(depth + 1, pattern);
                if let Some(t) = ty {
                    self.ty(depth + 1, t);
                }
                if let Some(e) = init {
                    self.expr(depth + 1, e);
                }
            }
            Stmt::Expr(e) => {
                self.open(depth, "Stmt.Expr", Some(s.span));
                self.expr(depth + 1, e);
            }
            Stmt::Item(item) => {
                self.open(depth, "Stmt.Item", Some(s.span));
                self.item(depth + 1, None, item);
            }
        }
        self.close();
    }

    // --- Expressions ---

    fn exprs(&mut self, depth: usize, exprs: &[Spanned<Expr>]) {
        for e in exprs {
            self.expr(depth, e);
        }
    }

    fn expr(&mut self, depth: usize, e: &Spanned<Expr>) {
        let span = Some(e.span);
        let inner = depth + 1;
        match &e.value {
            Expr::Lit(lit) => {
                self.open(depth, "Expr.Lit", span);
                self.literal(lit);
            }
            Expr::Path(path) => {
                self.open(depth, "Expr.Path", span);
                self.out.push(' ');
                self.path(path);
            }
            Expr::Tuple(items) => {
                self.open(depth, "Expr.Tuple", span);
                self.exprs(inner, items);
            }
            Expr::Array(items) => {
                self.open(depth, "Expr.Array", span);
                self.exprs(inner, items);
            }
            Expr::ArrayRepeat { init, len } => {
                self.open(depth, "Expr.ArrayRepeat", span);
                self.expr(inner, init);
                self.expr(inner, len);
            }
            Expr::Block(b) => {
                self.open(depth, "Expr.Block", span);
                self.block(inner, None, b);
            }
            Expr::If { cond, then, else_ } => {
                self.open(depth, "Expr.If", span);
                self.expr(inner, cond);
                self.block(inner, Some(then.span), &then.value);
                if let Some(e) = else_ {
                    self.expr(inner, e);
                }
            }
            Expr::Match { scrutinee, arms } => {
                self.open(depth, "Expr.Match", span);
                self.expr(inner, scrutinee);
                for arm in arms {
                    self.match_arm(inner, arm);
                }
            }
            Expr::While { cond, body } => {
                self.open(depth, "Expr.While", span);
                self.expr(inner, cond);
                self.block(inner, Some(body.span), &body.value);
            }
            Expr::For {
                pattern,
                iter,
                body,
            } => {
                self.open(depth, "Expr.For", span);
                self.pattern(inner, pattern);
                self.expr(inner, iter);
                self.block(inner, Some(body.span), &body.value);
            }
            Expr::Range { lo, hi, inclusive } => {
                self.open(depth, "Expr.Range", span);
                self.bool_field("inclusive", *inclusive);
                self.expr(inner, lo);
                self.expr(inner, hi);
            }
            Expr::Return(value) => {
                self.open(depth, "Expr.Return", span);
                if let Some(v) = value {
                    self.expr(inner, v);
                }
            }
            Expr::Break(value) => {
                self.open(depth, "Expr.Break", span);
                if let Some(v) = value {
                    self.expr(inner, v);
                }
            }
            Expr::Continue => self.open(depth, "Expr.Continue", span),
            Expr::Closure { params, ret, body } => {
                self.open(depth, "Expr.Closure", span);
                self.params(inner, params);
                if let Some(t) = ret {
                    self.ty(inner, t);
                }
                self.expr(inner, body);
            }
            Expr::Record { path, fields, base } => {
                self.open(depth, "Expr.Record", span);
                self.path_field("path", path);
                for f in fields {
                    self.field_init(inner, f);
                }
                if let Some(b) = base {
                    self.expr(inner, b);
                }
            }
            Expr::Binary { op, lhs, rhs } => {
                self.open(depth, "Expr.Binary", span);
                self.field("op", format_args!("{op:?}"));
                self.expr(inner, lhs);
                self.expr(inner, rhs);
            }
            Expr::Unary { op, expr } => {
                self.open(depth, "Expr.Unary", span);
                self.field("op", format_args!("{op:?}"));
                self.expr(inner, expr);
            }
            Expr::Call { callee, args } => {
                self.open(depth, "Expr.Call", span);
                self.expr(inner, callee);
                self.exprs(inner, args);
            }
            Expr::Index { target, index } => {
                self.open(depth, "Expr.Index", span);
                self.expr(inner, target);
                self.expr(inner, index);
            }
            Expr::Field { target, field } => {
                self.open(depth, "Expr.Field", span);
                self.name_field("field", field);
                self.expr(inner, target);
            }
            Expr::Try(e) => {
                self.open(depth, "Expr.Try", span);
                self.expr(inner, e);
            }
            Expr::Await(e) => {
                self.open(depth, "Expr.Await", span);
                self.expr(inner, e);
            }
            Expr::Cast { expr, ty } => {
                self.open(depth, "Expr.Cast", span);
                self.expr(inner, expr);
                self.ty(inner, ty);
            }
            Expr::Assign { op, lhs, rhs } => {
                self.open(depth, "Expr.Assign", span);
                self.field("op", format_args!("{op:?}"));
                self.expr(inner, lhs);
                self.expr(inner, rhs);
            }
            Expr::StringInterp(parts) => {
                self.open(depth, "Expr.StringInterp", span);
                for part in parts {
                    match part {
                        StringPart::Lit(text) => {
                            self.open(inner, "StringPart.Lit", None);
                            self.text(text);
                        }
                        StringPart::Expr(e) => {
                            self.open(inner, "StringPart.Expr", None);
                            self.expr(inner + 1, e);
                        }
                    }
                    self.close();
                }
            }
            Expr::View(nodes) => {
                self.open(depth, "Expr.View", span);
                self.view_nodes(inner, nodes);
            }
        }
        self.close();
    }

    fn match_arm(&mut self, depth: usize, arm: &MatchArm) {
        self.open(depth, "MatchArm", None);
        self.pattern(depth + 1, &arm.pattern);
        if let Some(g) = &arm.guard {
            self.expr(depth + 1, g);
        }
        self.expr(depth + 1, &arm.body);
        self.close();
    }

    fn field_init(&mut self, depth: usize, f: &FieldInit) {
        self.open(depth, "FieldInit", None);
        self.name_field("name", &f.name);
        if let Some(v) = &f.value {
            self.expr(depth + 1, v);
        }
        self.close();
    }

    fn view_nodes(&mut self, depth: usize, nodes: &[Spanned<ViewNode>]) {
        for node in nodes {
            match &node.value {
                ViewNode::Element {
                    name,
                    args,
                    children,
                } => {
                    self.open(depth, "ViewNode.Element", Some(node.span));
                    self.name_field("name", name);
                    let some = if children.is_some() { "Some" } else { "None" };
                    self.field("children", format_args!("{some}"));
                    for arg in args {
                        match arg {
                            ViewArg::Named { name, value } => {
                                self.open(depth + 1, "ViewArg.Named", None);
                                self.name_field("name", name);
                                self.expr(depth + 2, value);
                            }
                            ViewArg::Positional(e) => {
                                self.open(depth + 1, "ViewArg.Positional", None);
                                self.expr(depth + 2, e);
                            }
                        }
                        self.close();
                    }
                    if let Some(children) = children {
                        self.view_nodes(depth + 1, children);
                    }
                }
                ViewNode::Expr(e) => {
                    self.open(depth, "ViewNode.Expr", Some(node.span));
                    self.expr(depth + 1, e);
                }
            }
            self.close();
        }
    }

    // --- Patterns ---

    fn patterns(&mut self, depth: usize, patterns: &[Spanned<Pattern>]) {
        for p in patterns {
            self.pattern(depth, p);
        }
    }

    fn pattern(&mut self, depth: usize, p: &Spanned<Pattern>) {
        let span = Some(p.span);
        let inner = depth + 1;
        match &p.value {
            Pattern::Wildcard => self.open(depth, "Pattern.Wildcard", span),
            Pattern::Lit(lit) => {
                self.open(depth, "Pattern.Lit", span);
                self.literal(lit);
            }
            Pattern::Ident { is_mut, name } => {
                self.open(depth, "Pattern.Ident", span);
                self.bool_field("is_mut", *is_mut);
                self.name_field("name", name);
            }
            Pattern::Binding { name, inner: pat } => {
                self.open(depth, "Pattern.Binding", span);
                self.name_field("name", name);
                self.pattern(inner, pat);
            }
            Pattern::TupleStruct { path, fields } => {
                self.open(depth, "Pattern.TupleStruct", span);
                self.path_field("path", path);
                self.patterns(inner, fields);
            }
            Pattern::Record { path, fields, rest } => {
                self.open(depth, "Pattern.Record", span);
                self.path_field("path", path);
                self.bool_field("rest", *rest);
                for f in fields {
                    self.field_pat(inner, f);
                }
            }
            Pattern::Tuple(items) => {
                self.open(depth, "Pattern.Tuple", span);
                self.patterns(inner, items);
            }
            Pattern::Array(items) => {
                self.open(depth, "Pattern.Array", span);
                self.patterns(inner, items);
            }
            Pattern::Or(items) => {
                self.open(depth, "Pattern.Or", span);
                self.patterns(inner, items);
            }
            Pattern::Range { lo, hi, inclusive } => {
                self.open(depth, "Pattern.Range", span);
                self.bool_field("inclusive", *inclusive);
                self.pattern(inner, lo);
                self.pattern(inner, hi);
            }
            Pattern::Path(path) => {
                self.open(depth, "Pattern.Path", span);
                self.out.push(' ');
                self.path(path);
            }
        }
        self.close();
    }

    fn field_pat(&mut self, depth: usize, f: &FieldPat) {
        self.open(depth, "FieldPat", None);
        self.name_field("name", &f.name);
        if let Some(p) = &f.pattern {
            self.pattern(depth + 1, p);
        }
        self.close();
    }

    // --- Types ---

    fn ty(&mut self, depth: usize, t: &Spanned<Type>) {
        let span = Some(t.span);
        let inner = depth + 1;
        match &t.value {
            Type::Path { path, args } => {
                self.open(depth, "Type.Path", span);
                self.path_field("path", path);
                for a in args {
                    self.ty(inner, a);
                }
            }
            Type::Ref { is_mut, inner: ty } => {
                self.open(depth, "Type.Ref", span);
                self.bool_field("is_mut", *is_mut);
                self.ty(inner, ty);
            }
            Type::Ptr { is_mut, inner: ty } => {
                self.open(depth, "Type.Ptr", span);
                self.bool_field("is_mut", *is_mut);
                self.ty(inner, ty);
            }
            Type::Array(ty) => {
                self.open(depth, "Type.Array", span);
                self.ty(inner, ty);
            }
            Type::Tuple(items) => {
                self.open(depth, "Type.Tuple", span);
                for ty in items {
                    self.ty(inner, ty);
                }
            }
            Type::Fn { params, ret } => {
                self.open(depth, "Type.Fn", span);
                for ty in params {
                    self.ty(inner, ty);
                }
                self.ty(inner, ret);
            }
            Type::Optional(ty) => {
                self.open(depth, "Type.Optional", span);
                self.ty(inner, ty);
            }
            Type::Infer => self.open(depth, "Type.Infer", span),
        }
        self.close();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expr::BinOp;
    use crate::item::Visibility;
    use crate::{NewlineSplit, SplitKind};
    use nova_diagnostics::FileId;

    fn sp<T>(value: T, start: u32, end: u32) -> Spanned<T> {
        Spanned::new(value, Span::new(start, end, FileId::DUMMY))
    }

    fn name(s: &str, start: u32) -> Spanned<String> {
        sp(s.to_string(), start, start + s.len() as u32)
    }

    /// A small tree with a field of every kind: a doc, an attribute with
    /// an argument, a field-less enum, a bool, a name, a path, a literal
    /// of each kind with a string to escape, an absent option, an empty
    /// list, a node without a span, and a split.
    #[test]
    fn a_tree_with_every_kind_of_field() {
        let lit = |l: Literal, at: u32| sp(Expr::Lit(l), at, at + 1);
        let body = Block {
            stmts: vec![
                sp(
                    Stmt::Let {
                        is_mut: true,
                        pattern: sp(
                            Pattern::Ident {
                                is_mut: false,
                                name: name("x", 30),
                            },
                            30,
                            31,
                        ),
                        ty: None,
                        init: Some(sp(
                            Expr::Binary {
                                op: BinOp::Add,
                                lhs: Box::new(lit(Literal::Float(0.1), 34)),
                                rhs: Box::new(lit(Literal::Char('é'), 38)),
                            },
                            34,
                            39,
                        )),
                    },
                    22,
                    40,
                ),
                sp(
                    Stmt::Expr(lit(Literal::Str("a\"b\\c\né".into()), 41)),
                    41,
                    42,
                ),
            ],
            trailing: Some(Box::new(sp(
                Expr::Path(Path {
                    segments: vec![name("std", 43), name("x", 48)],
                }),
                43,
                49,
            ))),
        };
        let f = Function {
            docs: vec![sp(" Adds.".to_string(), 0, 9)],
            attrs: vec![Attribute {
                name: name("test", 11),
                args: vec![name("should_panic", 16)],
                span: Span::new(10, 11, FileId::DUMMY),
            }],
            vis: Visibility::Pub,
            is_async: false,
            is_component: false,
            name: name("f", 17),
            generics: Vec::new(),
            params: Vec::new(),
            return_ty: None,
            where_clause: Vec::new(),
            body: sp(body, 20, 50),
        };
        let file = File {
            items: vec![sp(Item::Function(f), 0, 50)],
            newline_splits: vec![NewlineSplit {
                at: Span::new(41, 42, FileId::DUMMY),
                kind: SplitKind::Minus,
            }],
        };
        assert_eq!(
            dump_file(&file),
            "(File\n\
             \x20 (Item.Function 0..50\n\
             \x20   (Function vis=Pub is_async=false is_component=false name=\"f\"@17..18\n\
             \x20     (Doc 0..9 \" Adds.\")\n\
             \x20     (Attr 10..11 \"test\"@11..15 \"should_panic\"@16..28)\n\
             \x20     (Block 20..50\n\
             \x20       (Stmt.Let 22..40 is_mut=true\n\
             \x20         (Pattern.Ident 30..31 is_mut=false name=\"x\"@30..31)\n\
             \x20         (Expr.Binary 34..39 op=Add\n\
             \x20           (Expr.Lit 34..35 Float:3fb999999999999a)\n\
             \x20           (Expr.Lit 38..39 Char:233)))\n\
             \x20       (Stmt.Expr 41..42\n\
             \x20         (Expr.Lit 41..42 Str:\"a\\x22b\\x5cc\\x0aé\"))\n\
             \x20       (Expr.Path 43..49 \"std\"@43..46::\"x\"@48..49))))\n\
             \x20 (Split 41..42 kind=Minus))\n"
        );
        assert_eq!(dump_file(&File::default()), "(File)\n");
    }

    /// Every entry of [`KINDS`] is a kind, a field-less value, a literal's
    /// prefix or a kind and a value, and none is there twice.
    #[test]
    fn the_kinds_are_well_formed_and_distinct() {
        let mut seen = std::collections::HashSet::new();
        for k in KINDS {
            assert!(seen.insert(k), "{k} is listed twice");
            assert!(
                k.starts_with('(') || k.contains('=') || k.ends_with(':'),
                "{k}"
            );
        }
        // 24 structs and `Doc`, the enums' 119 variants, then 11 `bool`
        // fields' values but one and `children`'s two.
        assert_eq!(KINDS.len(), 25 + 119 + 21 + 2);
    }

    /// What the declarations allow but no parser builds (see [`KINDS`]).
    const NEVER_BUILT: &[&str] = &["(Pattern.Ident is_mut=true"];

    /// [`KINDS`] is exactly what this crate's declarations imply, by
    /// [`declared_forms`], with `Doc` added and [`NEVER_BUILT`] taken out:
    /// a new struct, variant or `bool` field fails here until it is listed.
    #[test]
    fn every_declared_form_is_listed() {
        let mut want = declared_forms();
        want.insert("(Doc".to_string());
        for never in NEVER_BUILT {
            assert!(want.remove(*never), "{never} is not declared");
        }
        let have: std::collections::BTreeSet<String> =
            KINDS.iter().map(|k| k.to_string()).collect();
        let missing: Vec<&String> = want.difference(&have).collect();
        let extra: Vec<&String> = have.difference(&want).collect();
        assert!(
            missing.is_empty() && extra.is_empty(),
            "declared but not in KINDS: {missing:?}; in KINDS but not declared: {extra:?}"
        );
    }

    /// One declaration's body: a struct's fields, or an enum's variants,
    /// each with its named fields (none for a unit or tuple variant) and
    /// whether it holds anything.
    enum Decl {
        Struct(Fields),
        Enum(Vec<(String, bool, Fields)>),
    }

    /// Named fields: each one's name and its type's tokens joined.
    type Fields = Vec<(String, String)>;

    /// The forms [`KINDS`] lists, read from the `struct` and `enum`
    /// declarations of this crate's sources but this file by a small
    /// scanner ([`scan`]): a struct's kind (`Attribute` is `Attr`, `NewlineSplit`
    /// `Split`); an enum's variants, `Literal`'s as `Variant:`, a field-less
    /// enum's as `key=Variant` for each field of that type, any other's as
    /// `(Enum.Variant`; and for each `bool` field, and each
    /// `Option<Vec<…>>`, its two values after its node's kind.
    fn declared_forms() -> std::collections::BTreeSet<String> {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/src");
        let mut decls: Vec<(String, Decl)> = Vec::new();
        let mut paths: Vec<_> = std::fs::read_dir(dir)
            .expect("read nova-ast's src")
            .map(|e| e.expect("an entry").path())
            .filter(|p| p.extension().is_some_and(|x| x == "rs"))
            .filter(|p| !p.ends_with("dump.rs"))
            .collect();
        paths.sort();
        assert!(paths.len() >= 5, "{paths:?}");
        for path in paths {
            let text = std::fs::read_to_string(&path).expect("read a source");
            decls.extend(scan(&tokens(&text)));
        }
        let mut forms = std::collections::BTreeSet::new();
        let field_forms = |forms: &mut std::collections::BTreeSet<String>,
                           kind: &str,
                           fields: &[(String, String)]| {
            for (name, ty) in fields {
                let values = match ty.as_str() {
                    "bool" => ["true", "false"],
                    t if t.starts_with("Option<Vec<") => ["Some", "None"],
                    _ => continue,
                };
                for v in values {
                    forms.insert(format!("({kind} {name}={v}"));
                }
            }
        };
        for (name, decl) in &decls {
            match decl {
                Decl::Struct(fields) => {
                    let kind = match name.as_str() {
                        "Attribute" => "Attr",
                        "NewlineSplit" => "Split",
                        n => n,
                    };
                    forms.insert(format!("({kind}"));
                    field_forms(&mut forms, kind, fields);
                }
                Decl::Enum(variants) if name == "Literal" => {
                    for (v, _, _) in variants {
                        forms.insert(format!("{v}:"));
                    }
                }
                Decl::Enum(variants) if variants.iter().all(|(_, holds, _)| !holds) => {
                    // The fields of this type, anywhere, give the key.
                    let keys: Vec<&String> = decls
                        .iter()
                        .flat_map(|(_, d)| match d {
                            Decl::Struct(fields) => fields.iter().collect::<Vec<_>>(),
                            Decl::Enum(vs) => vs.iter().flat_map(|(_, _, f)| f).collect(),
                        })
                        .filter(|(_, ty)| ty == name)
                        .map(|(key, _)| key)
                        .collect();
                    assert!(!keys.is_empty(), "no field has the type {name}");
                    for key in keys {
                        for (v, _, _) in variants {
                            forms.insert(format!("{key}={v}"));
                        }
                    }
                }
                Decl::Enum(variants) => {
                    for (v, _, fields) in variants {
                        let kind = format!("{name}.{v}");
                        forms.insert(format!("({kind}"));
                        field_forms(&mut forms, &kind, fields);
                    }
                }
            }
        }
        forms
    }

    /// `text`'s words and punctuation, line comments left out.
    fn tokens(text: &str) -> Vec<String> {
        let mut out = Vec::new();
        for line in text.lines() {
            let code = line.split("//").next().unwrap_or("");
            let mut word = String::new();
            for c in code.chars() {
                if c.is_alphanumeric() || c == '_' {
                    word.push(c);
                    continue;
                }
                if !word.is_empty() {
                    out.push(std::mem::take(&mut word));
                }
                if !c.is_whitespace() {
                    out.push(c.to_string());
                }
            }
            if !word.is_empty() {
                out.push(word);
            }
        }
        out
    }

    /// The index just past the bracket that closes the one at `at`. The
    /// `>` of a `->` closes nothing.
    fn past_close(t: &[String], at: usize) -> usize {
        let mut depth = 0i32;
        for (i, tok) in t.iter().enumerate().skip(at) {
            match tok.as_str() {
                "(" | "[" | "{" | "<" => depth += 1,
                ">" if i > 0 && t[i - 1] == "-" => {}
                ")" | "]" | "}" | ">" => depth -= 1,
                _ => {}
            }
            if depth == 0 {
                return i + 1;
            }
        }
        panic!("an unclosed {} at token {at}", t[at]);
    }

    /// The named fields between the `{` at `at` and its `}`, `pub` and
    /// attributes left out, each type's tokens joined.
    fn fields(t: &[String], at: usize) -> Fields {
        let end = past_close(t, at) - 1;
        let mut out = Vec::new();
        let mut i = at + 1;
        while i < end {
            match t[i].as_str() {
                "#" => i = past_close(t, i + 1),
                "pub" if t[i + 1] == "(" => i = past_close(t, i + 1),
                "pub" | "," => i += 1,
                _ => {
                    assert_eq!(t[i + 1], ":", "a field at token {i}");
                    let mut j = i + 2;
                    let mut ty = String::new();
                    while j < end && t[j] != "," {
                        let k = if "([{<".contains(t[j].as_str()) {
                            past_close(t, j)
                        } else {
                            j + 1
                        };
                        ty.extend(t[j..k].iter().map(String::as_str));
                        j = k;
                    }
                    out.push((t[i].clone(), ty));
                    i = j;
                }
            }
        }
        out
    }

    /// Each `struct` and `enum` declaration among `t`, whatever its
    /// visibility: its name, any generic parameters and `where` clause,
    /// then its body, `{…}`, or for a struct `;`. Any other `struct` or
    /// `enum`, such as a tuple struct, whose fields have no names to key a
    /// value on, or a variant it cannot read, fails here: no declaration is
    /// passed over unread.
    fn scan(t: &[String]) -> Vec<(String, Decl)> {
        let mut out = Vec::new();
        let mut i = 0;
        while i < t.len() {
            let keyword = t[i].as_str();
            if keyword != "struct" && keyword != "enum" {
                i += 1;
                continue;
            }
            let at = |k: usize| t.get(k).map(String::as_str).unwrap_or("");
            let name = at(i + 1).to_string();
            let named = name.starts_with(|c: char| c.is_alphabetic() || c == '_');
            assert!(named, "`{keyword}` without a name at token {i}");
            let mut open = i + 2;
            if at(open) == "<" {
                open = past_close(t, open);
            }
            if at(open) == "where" {
                while open < t.len() && at(open) != "{" && at(open) != ";" {
                    open += 1;
                }
            }
            match (keyword, at(open)) {
                ("struct", ";") => {
                    out.push((name, Decl::Struct(Vec::new())));
                    i = open + 1;
                }
                ("struct", "{") => {
                    out.push((name, Decl::Struct(fields(t, open))));
                    i = past_close(t, open);
                }
                ("enum", "{") => {
                    out.push((name, Decl::Enum(variants(t, open))));
                    i = past_close(t, open);
                }
                (_, other) => panic!(
                    "`{keyword} {name}` at token {i}: `{other}` where a body was due \
                     (a tuple struct's fields have no names)"
                ),
            }
        }
        out
    }

    /// The variants between the `{` at `at` and its `}`: each one's name,
    /// whether it holds anything, and its named fields.
    fn variants(t: &[String], at: usize) -> Vec<(String, bool, Fields)> {
        let end = past_close(t, at) - 1;
        let mut out = Vec::new();
        let mut j = at + 1;
        while j < end {
            match t[j].as_str() {
                "#" => j = past_close(t, j + 1),
                "," => j += 1,
                v => {
                    let named = v.starts_with(|c: char| c.is_alphabetic() || c == '_');
                    assert!(named, "a variant at token {j}: `{v}`");
                    match t[j + 1].as_str() {
                        "(" => {
                            out.push((v.to_string(), true, Vec::new()));
                            j = past_close(t, j + 1);
                        }
                        "{" => {
                            out.push((v.to_string(), true, fields(t, j + 1)));
                            j = past_close(t, j + 1);
                        }
                        "," | "}" => {
                            out.push((v.to_string(), false, Vec::new()));
                            j += 1;
                        }
                        other => panic!("the variant `{v}` at token {j}: `{other}` after it"),
                    }
                }
            }
        }
        out
    }

    /// The scanner reads what the hand-kept list says of a few known
    /// declarations, so a fault in it cannot hide behind an empty set.
    #[test]
    fn the_scanner_reads_the_declarations() {
        let forms = declared_forms();
        for f in [
            "(File",
            "(Attr",
            "(Split",
            "(Expr.Cast",
            "(ImportKind.Simple",
            "Int:",
            "vis=Private",
            "kind=Value",
            "op=ShrAssign",
            "op=Deref",
            "(Pattern.Record rest=false",
            "(Type.Ptr is_mut=true",
            "(ViewNode.Element children=None",
        ] {
            assert!(forms.contains(f), "{f} not read: {forms:?}");
        }
        assert!(!forms.contains("(Attribute") && !forms.contains("(Literal.Int"));
        // Each declaration's shape is read, or fails: none is passed over.
        let shown = |src: &str| -> String {
            let decls = std::panic::catch_unwind(|| scan(&tokens(src)));
            let Ok(decls) = decls else {
                return "fails".to_string();
            };
            let fields = |f: &Fields| {
                let f: Vec<String> = f.iter().map(|(n, t)| format!("{n}:{t}")).collect();
                f.join(",")
            };
            let decls: Vec<String> = decls
                .iter()
                .map(|(name, decl)| match decl {
                    Decl::Struct(f) => format!("{name}{{{}}}", fields(f)),
                    Decl::Enum(vs) => {
                        let vs: Vec<String> = vs
                            .iter()
                            .map(|(v, holds, f)| match (holds, f.is_empty()) {
                                (false, _) => v.clone(),
                                (true, true) => format!("{v}()"),
                                (true, false) => format!("{v}{{{}}}", fields(f)),
                            })
                            .collect();
                        format!("{name}{{{}}}", vs.join(","))
                    }
                })
                .collect();
            decls.join(" ")
        };
        let mut wrong = Vec::new();
        for (src, want) in [
            ("pub struct G<T> { a: bool }", "G{a:bool}"),
            (
                "pub enum E<T> { A(T), B { x: bool }, C }",
                "E{A(),B{x:bool},C}",
            ),
            ("pub struct W where T: Clone { a: bool }", "W{a:bool}"),
            ("pub(crate) struct C { pub(crate) a: bool }", "C{a:bool}"),
            ("struct S { a: bool } enum D { A, B }", "S{a:bool} D{A,B}"),
            (
                "pub struct F { f: fn(Int) -> Int, b: bool }",
                "F{f:fn(Int)->Int,b:bool}",
            ),
            ("pub struct U;", "U{}"),
            ("pub struct T(pub bool);", "fails"),
            ("pub enum N { A = 1 }", "fails"),
        ] {
            let got = shown(src);
            if got != want {
                wrong.push(format!("`{src}`: read as `{got}`, not `{want}`"));
            }
        }
        assert!(wrong.is_empty(), "{wrong:#?}");
    }
}
