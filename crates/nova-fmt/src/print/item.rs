//! Items (spec
//! `docs/superpowers/specs/2026-10-07-phase-3-1-formatter-design.md` §6,
//! "Items").

use nova_ast::item::{
    AssocTypeBinding, Attribute, ConstDecl, ExternBlock, ExternItem, Function, FunctionSig,
    ImplBlock, Import, ImportKind, Param, Record, RecordField, TraitDecl, TraitItem, TypeDecl,
    TypeDef, Variant, Visibility, WhereBound,
};
use nova_ast::ty::{Type, TypeParam};
use nova_ast::Item;
use nova_diagnostics::{Span, Spanned};
use nova_lexer::Token;

use super::{Blank, List, Printer, Vertical};
use crate::doc::{concat, group, if_break, nest, text, Doc};

/// The tokens a function's keyword may start with.
const FN_KEYWORDS: [Token; 3] = [Token::Fn, Token::Async, Token::Pub];

/// `pub ` or nothing.
fn vis(v: Visibility) -> &'static str {
    if v == Visibility::Pub {
        "pub "
    } else {
        ""
    }
}

/// A member of an `impl`, `trait` or `extern` body.
#[derive(Clone, Copy)]
enum Member<'a> {
    Fn(&'a Function),
    Const(&'a ConstDecl),
    Binding(&'a AssocTypeBinding),
    Trait(&'a TraitItem),
    Extern(&'a ExternItem),
}

impl<'s, 't> Printer<'s, 't> {
    /// An item (spec §6). `top` marks a top-level item: only a top-level
    /// import's `{…}` list is sorted.
    pub(super) fn item(&mut self, item: &Item, span: Span, top: bool) -> Doc {
        match item {
            Item::Function(f) => self.function(f),
            Item::Record(r) => self.record(r, span),
            Item::Type(t) => self.type_decl(t),
            Item::Trait(t) => self.trait_decl(t, span),
            Item::Impl(i) => self.impl_block(i, span),
            Item::Const(c) => self.const_decl(c),
            Item::Import(i) => self.import(i, top),
            Item::Module(m) => {
                let keyword = self.src.start_with(m.path.span.start, &[Token::Module]);
                let head = self.item_head(&m.docs, &m.attrs, keyword);
                concat(vec![head, text("module "), self.path(&m.path.value)])
            }
            Item::Extern(e) => self.extern_block(e, span),
        }
    }

    /// Docs, then attributes one per line (spec §6, "Order"), then the
    /// comments between them and the keyword at `keyword`, which move after
    /// them (the 3.1 plan's decision 9).
    fn item_head(&mut self, docs: &[Spanned<String>], attrs: &[Attribute], keyword: u32) -> Doc {
        concat(vec![
            self.docs(docs),
            self.attrs(attrs),
            self.leading(keyword),
        ])
    }

    /// Attributes, one per line.
    fn attrs(&self, attrs: &[Attribute]) -> Doc {
        let mut parts = Vec::new();
        for a in attrs {
            let args = if a.args.is_empty() {
                String::new()
            } else {
                let names: Vec<&str> = a.args.iter().map(|x| x.value.as_str()).collect();
                format!("({})", names.join(", "))
            };
            parts.push(text(format!("@{}{args}", a.name.value)));
            parts.push(Doc::HardLine);
        }
        concat(parts)
    }

    /// A function. Its body shares a group with its signature, so a
    /// one-statement body stays on one line only when the whole function
    /// fits (spec §6, "Blocks"; the 3.1 plan's decision 6).
    fn function(&mut self, f: &Function) -> Doc {
        let keyword = self.src.start_with(f.name.span.start, &FN_KEYWORDS);
        let head = self.item_head(&f.docs, &f.attrs, keyword);
        let words = format!(
            "{}{}fn ",
            vis(f.vis),
            if f.is_async { "async " } else { "" }
        );
        let sig = self.signature(
            &f.name,
            &f.generics,
            &f.params,
            f.return_ty.as_ref(),
            &f.where_clause,
            true,
        );
        let gap = if f.where_clause.is_empty() {
            text(" ")
        } else {
            Doc::Nil
        };
        let body = self.block_parts(&f.body.value, f.body.span);
        concat(vec![head, group(concat(vec![text(words), sig, gap, body]))])
    }

    /// `name<generics>(params) -> ret where …`. The parameters break first,
    /// then the `where` clause (spec §6, "Signatures"). With `body`, a broken
    /// clause ends with a trailing comma and a line break before the `{`.
    fn signature(
        &mut self,
        name: &Spanned<String>,
        generics: &[TypeParam],
        params: &[Param],
        ret: Option<&Spanned<Type>>,
        where_clause: &[WhereBound],
        body: bool,
    ) -> Doc {
        let mut parts = vec![self.name(name), self.generics(generics)];
        let close = self
            .src
            .parens_from(name.span.end)
            .map_or(name.span.end, |(_, close)| close);
        parts.push(self.comma_list(
            List::PARENS,
            params,
            |p, x| p.param_extent(x),
            |p, x| p.param(x),
            close,
        ));
        if let Some(r) = ret {
            parts.push(text(" -> "));
            parts.push(self.ty(r));
        }
        if !where_clause.is_empty() {
            parts.push(self.where_clause(where_clause, body));
        }
        group(concat(parts))
    }

    /// `<T: A + B, U>`, or nothing.
    fn generics(&mut self, generics: &[TypeParam]) -> Doc {
        let Some(last) = generics.last() else {
            return Doc::Nil;
        };
        let last_end = self.type_param_extent(last).1;
        let close = self.src.find_from(last_end, &Token::Gt).unwrap_or(last_end);
        group(self.comma_list(
            List::ANGLES,
            generics,
            |p, g| p.type_param_extent(g),
            |p, g| p.type_param(g),
            close,
        ))
    }

    fn type_param(&mut self, g: &TypeParam) -> Doc {
        if g.bounds.is_empty() {
            self.name(&g.name)
        } else {
            concat(vec![self.name(&g.name), text(": "), self.bounds(&g.bounds)])
        }
    }

    /// A bound's span is its first token's (`grammar.rs:482-484`), so a
    /// list of bounds ends where its last path's last segment ends.
    fn type_param_extent(&self, g: &TypeParam) -> (u32, u32) {
        let end = g
            .bounds
            .last()
            .and_then(|b| b.value.segments.last())
            .map_or(g.name.span.end, |s| s.span.end);
        (g.name.span.start, end)
    }

    /// A parameter. `self`, and a closure parameter written without a type,
    /// print bare.
    pub(super) fn param(&mut self, p: &Param) -> Doc {
        let mut parts = vec![text(if p.is_mut { "mut " } else { "" }), self.name(&p.name)];
        if !bare(p) {
            parts.push(text(": "));
            parts.push(self.ty(&p.ty));
        }
        concat(parts)
    }

    fn param_extent(&self, p: &Param) -> (u32, u32) {
        let start = if p.is_mut {
            self.src.start_with(p.name.span.start, &[Token::Mut])
        } else {
            p.name.span.start
        };
        let end = if bare(p) {
            p.name.span.end
        } else {
            self.ty_end(&p.ty)
        };
        (start, end)
    }

    /// ` where T: A, U: B`, in a group of its own (spec §6, "Signatures").
    fn where_clause(&mut self, bounds: &[WhereBound], body: bool) -> Doc {
        let mut inner = Vec::new();
        for (i, b) in bounds.iter().enumerate() {
            let (start, end) = self.where_bound_extent(b);
            inner.push(Doc::Line);
            inner.push(self.leading(start));
            inner.push(self.ty(&b.ty));
            inner.push(text(": "));
            inner.push(self.bounds(&b.bounds));
            inner.push(self.trailing(end));
            inner.push(if i + 1 < bounds.len() {
                text(",")
            } else if body {
                // The parser takes a trailing comma only before the body's
                // `{` (the 3.1 plan's decision 4).
                if_break(text(","), Doc::Nil)
            } else {
                Doc::Nil
            });
            if let Some(at) = self.src.token_end(end, &Token::Comma) {
                inner.push(self.trailing(at));
            }
        }
        let mut parts = vec![Doc::Line, text("where"), nest(concat(inner))];
        if body {
            parts.push(Doc::Line);
        }
        group(concat(parts))
    }

    fn where_bound_extent(&self, b: &WhereBound) -> (u32, u32) {
        let end = b
            .bounds
            .last()
            .and_then(|p| p.value.segments.last())
            .map_or(b.ty.span.end, |s| s.span.end);
        (b.ty.span.start, end)
    }

    fn record(&mut self, r: &Record, span: Span) -> Doc {
        let keyword = self
            .src
            .start_with(r.name.span.start, &[Token::Record, Token::Pub]);
        let head = concat(vec![
            self.item_head(&r.docs, &r.attrs, keyword),
            text(format!("{}record ", vis(r.vis))),
            self.name(&r.name),
            self.generics(&r.generics),
            text(" "),
        ]);
        let fields = self.comma_list(
            List::FIELDS,
            &r.fields,
            |p, f| p.field_extent(f),
            |p, f| p.record_field(f),
            span.end - 1,
        );
        concat(vec![head, group(fields)])
    }

    fn record_field(&mut self, f: &RecordField) -> Doc {
        let start = self.src.start_with(f.name.span.start, &[Token::Pub]);
        concat(vec![
            self.item_head(&f.docs, &[], start),
            text(vis(f.vis)),
            self.name(&f.name),
            text(": "),
            self.ty(&f.ty),
        ])
    }

    fn field_extent(&self, f: &RecordField) -> (u32, u32) {
        let start = match f.docs.first() {
            Some(d) => d.span.start,
            None => self.src.start_with(f.name.span.start, &[Token::Pub]),
        };
        (start, self.ty_end(&f.ty))
    }

    fn type_decl(&mut self, t: &TypeDecl) -> Doc {
        let keyword = self
            .src
            .start_with(t.name.span.start, &[Token::Type, Token::Pub]);
        let head = concat(vec![
            self.item_head(&t.docs, &t.attrs, keyword),
            text(format!("{}type ", vis(t.vis))),
            self.name(&t.name),
            self.generics(&t.generics),
            text(" ="),
        ]);
        let variants = match &t.def {
            TypeDef::Alias(ty) => return concat(vec![head, text(" "), self.ty(ty)]),
            TypeDef::Sum(variants) => variants,
        };
        let mut inner = Vec::new();
        let mut prev_end: Option<u32> = None;
        for v in variants {
            let (start, end) = self.variant_extent(v);
            // The author's blank lines between variants are kept, and one
            // breaks the list (spec §5.3).
            if prev_end.is_some_and(|p| self.src.blank_line_in(p, start)) {
                inner.push(Doc::HardLine);
            }
            inner.push(Doc::Line);
            inner.push(self.leading(start));
            let pipe = self.src.start_with(v.name.span.start, &[Token::Pipe]);
            inner.push(self.item_head(&v.docs, &[], pipe));
            inner.push(text("| "));
            inner.push(self.name(&v.name));
            if !v.fields.is_empty() {
                inner.push(group(self.comma_list(
                    List::PARENS,
                    &v.fields,
                    |p, x| (x.span.start, p.ty_end(x)),
                    |p, x| p.ty(x),
                    end - 1,
                )));
            }
            inner.push(self.trailing(end));
            prev_end = Some(end);
        }
        concat(vec![head, group(nest(concat(inner)))])
    }

    /// A variant runs from its docs or its `|` to its name, or to the `)`
    /// after its fields.
    fn variant_extent(&self, v: &Variant) -> (u32, u32) {
        let start = match v.docs.first() {
            Some(d) => d.span.start,
            None => self.src.start_with(v.name.span.start, &[Token::Pipe]),
        };
        let end = match v.fields.last() {
            None => v.name.span.end,
            Some(last) => {
                let after = self.ty_end(last);
                self.src
                    .find_from(after, &Token::RParen)
                    .map_or(after, |at| at + 1)
            }
        };
        (start, end)
    }

    fn trait_decl(&mut self, t: &TraitDecl, span: Span) -> Doc {
        let keyword = self
            .src
            .start_with(t.name.span.start, &[Token::Trait, Token::Pub]);
        let mut head = vec![
            self.item_head(&t.docs, &t.attrs, keyword),
            text(format!("{}trait ", vis(t.vis))),
            self.name(&t.name),
            self.generics(&t.generics),
        ];
        if !t.supertraits.is_empty() {
            head.push(text(": "));
            head.push(self.bounds(&t.supertraits));
        }
        head.push(if t.where_clause.is_empty() {
            text(" ")
        } else {
            self.where_clause(&t.where_clause, true)
        });
        let members: Vec<Member> = t.items.iter().map(Member::Trait).collect();
        let body = self.members(&members, span);
        concat(vec![concat(head), body])
    }

    fn impl_block(&mut self, i: &ImplBlock, span: Span) -> Doc {
        let keyword = self
            .src
            .find_from(span.start, &Token::Impl)
            .unwrap_or(span.start);
        let mut head = vec![
            self.item_head(&i.docs, &i.attrs, keyword),
            text("impl"),
            self.generics(&i.generics),
            text(" "),
        ];
        if let Some(tr) = &i.trait_ {
            // As written: the parser keeps the trait's path but drops its
            // type arguments (`grammar.rs:2511-2515`), so `impl Into<String>
            // for X` printed from the AST would lose `<String>`, and the
            // self-check could not notice.
            head.push(self.leading(tr.span.start));
            head.push(self.verbatim(tr.span));
            head.push(text(" for "));
        }
        head.push(self.ty(&i.ty));
        head.push(if i.where_clause.is_empty() {
            text(" ")
        } else {
            self.where_clause(&i.where_clause, true)
        });
        let mut members: Vec<Member> = i
            .functions
            .iter()
            .map(Member::Fn)
            .chain(i.consts.iter().map(Member::Const))
            .chain(i.assoc_types.iter().map(Member::Binding))
            .collect();
        // The AST keeps the three kinds in separate lists: print them in
        // source order (spec §5.3).
        members.sort_by_key(|m| self.member_extent(*m).0);
        let body = self.members(&members, span);
        concat(vec![concat(head), body])
    }

    fn const_decl(&mut self, c: &ConstDecl) -> Doc {
        let keyword = self
            .src
            .start_with(c.name.span.start, &[Token::Const, Token::Pub]);
        concat(vec![
            self.item_head(&c.docs, &c.attrs, keyword),
            text(format!("{}const ", vis(c.vis))),
            self.name(&c.name),
            text(": "),
            self.ty(&c.ty),
            text(" = "),
            self.expr(&c.value),
        ])
    }

    /// An import. With `sort`, a `{…}` list is sorted by name (spec §6).
    fn import(&mut self, i: &Import, sort: bool) -> Doc {
        let keyword = self.src.start_with(i.path.span.start, &[Token::Import]);
        let mut parts = vec![
            self.item_head(&i.docs, &i.attrs, keyword),
            text("import "),
            self.path(&i.path.value),
        ];
        match &i.kind {
            ImportKind::Simple => {}
            ImportKind::Alias(a) => {
                parts.push(text(" as "));
                parts.push(self.name(a));
            }
            ImportKind::List(names) => {
                let after = names
                    .iter()
                    .map(|n| n.span.end)
                    .max()
                    .unwrap_or(i.path.span.end);
                let close = self.src.find_from(after, &Token::RBrace).unwrap_or(after);
                parts.push(text("::"));
                // Each name takes its comments in source order and keeps
                // them when the list is sorted, as an import does in a
                // sorted run (spec §6).
                let entries =
                    self.list_entries(names, |_, n| (n.span.start, n.span.end), |p, n| p.name(n));
                let mut keyed: Vec<_> = names.iter().map(|n| &n.value).zip(entries).collect();
                if sort {
                    keyed.sort_by_key(|k| k.0);
                }
                let entries = keyed.into_iter().map(|(_, e)| e).collect();
                parts.push(group(self.assemble_list(List::IMPORTS, entries, close)));
            }
        }
        concat(parts)
    }

    fn extern_block(&mut self, e: &ExternBlock, span: Span) -> Doc {
        let keyword = self
            .src
            .find_from(span.start, &Token::Extern)
            .unwrap_or(span.start);
        let head = self.item_head(&e.docs, &e.attrs, keyword);
        let abi = e
            .abi
            .as_ref()
            .map_or(String::new(), |a| format!("\"{a}\" "));
        let members: Vec<Member> = e.items.iter().map(Member::Extern).collect();
        concat(vec![
            head,
            text(format!("extern {abi}")),
            self.members(&members, span),
        ])
    }

    /// A `{ … }` body of members, one per line, with the author's blank
    /// lines (spec §6): `{}` when it holds nothing.
    fn members(&mut self, members: &[Member], span: Span) -> Doc {
        let open = self
            .src
            .find_from(span.start, &Token::LBrace)
            .unwrap_or(span.start);
        let lead = self.leading(open);
        let close = span.end - 1;
        if members.is_empty() && !self.has_comment_before(close) {
            return concat(vec![lead, text("{}")]);
        }
        let after_open = self.trailing(open + 1);
        let mut v = Vertical::new(open + 1, Blank::Keep);
        for m in members {
            let (start, end) = self.member_extent(*m);
            let end = self.src.token_end(end, &Token::Semicolon).unwrap_or(end);
            self.vertical_item(&mut v, start, end, None, |p| p.member(*m));
        }
        let inner = self.vertical_finish(v, close);
        concat(vec![
            lead,
            text("{"),
            after_open,
            nest(concat(vec![Doc::HardLine, inner])),
            Doc::HardLine,
            text("}"),
        ])
    }

    fn member(&mut self, m: Member) -> Doc {
        match m {
            Member::Fn(f) | Member::Trait(TraitItem::Provided(f)) => self.function(f),
            Member::Const(c) => self.const_decl(c),
            Member::Binding(b) => {
                let keyword = self.src.start_with(b.name.span.start, &[Token::Type]);
                concat(vec![
                    self.item_head(&b.docs, &[], keyword),
                    text("type "),
                    self.name(&b.name),
                    text(" = "),
                    self.ty(&b.ty),
                ])
            }
            Member::Trait(TraitItem::Required(sig)) | Member::Extern(ExternItem::Fn(sig)) => {
                self.function_sig(sig)
            }
            Member::Trait(TraitItem::AssocType { docs, name, bounds }) => {
                let keyword = self.src.start_with(name.span.start, &[Token::Type]);
                let mut parts = vec![
                    self.item_head(docs, &[], keyword),
                    text("type "),
                    self.name(name),
                ];
                if !bounds.is_empty() {
                    parts.push(text(": "));
                    parts.push(self.bounds(bounds));
                }
                concat(parts)
            }
        }
    }

    /// A signature with no body: a required trait method or an extern
    /// function.
    fn function_sig(&mut self, s: &FunctionSig) -> Doc {
        let keyword = self.src.start_with(s.name.span.start, &FN_KEYWORDS);
        let words = if s.is_async { "async fn " } else { "fn " };
        concat(vec![
            self.item_head(&s.docs, &[], keyword),
            text(words),
            self.signature(
                &s.name,
                &s.generics,
                &s.params,
                s.return_ty.as_ref(),
                &s.where_clause,
                false,
            ),
        ])
    }

    /// Where a member starts, from its docs or its keywords, and ends.
    fn member_extent(&self, m: Member) -> (u32, u32) {
        match m {
            Member::Fn(f) | Member::Trait(TraitItem::Provided(f)) => (
                self.member_start(&f.docs, f.name.span.start, &FN_KEYWORDS),
                f.body.span.end,
            ),
            Member::Const(c) => (
                self.member_start(&c.docs, c.name.span.start, &[Token::Const, Token::Pub]),
                c.value.span.end,
            ),
            Member::Binding(b) => (
                self.member_start(&b.docs, b.name.span.start, &[Token::Type]),
                self.ty_end(&b.ty),
            ),
            Member::Trait(TraitItem::Required(s)) | Member::Extern(ExternItem::Fn(s)) => (
                self.member_start(&s.docs, s.name.span.start, &FN_KEYWORDS),
                self.sig_end(s),
            ),
            Member::Trait(TraitItem::AssocType { docs, name, bounds }) => {
                let end = bounds
                    .last()
                    .and_then(|b| b.value.segments.last())
                    .map_or(name.span.end, |s| s.span.end);
                (
                    self.member_start(docs, name.span.start, &[Token::Type]),
                    end,
                )
            }
        }
    }

    fn member_start(&self, docs: &[Spanned<String>], name: u32, keywords: &[Token]) -> u32 {
        match docs.first() {
            Some(d) => d.span.start,
            None => self.src.start_with(name, keywords),
        }
    }

    /// Where a bodyless signature ends: its last bound, its return type, or
    /// its `)`.
    fn sig_end(&self, s: &FunctionSig) -> u32 {
        if let Some(last) = s.where_clause.last() {
            return self.where_bound_extent(last).1;
        }
        if let Some(r) = &s.return_ty {
            return self.ty_end(r);
        }
        self.src
            .parens_from(s.name.span.end)
            .map_or(s.name.span.end, |(_, close)| close + 1)
    }
}

/// Whether `p` was written without a type: `self`, or a closure parameter.
/// The parser gives it an inferred type spanning its name
/// (`grammar.rs:441-447`, `:2031-2034`).
fn bare(p: &Param) -> bool {
    matches!(p.ty.value, Type::Infer) && p.ty.span == p.name.span
}
