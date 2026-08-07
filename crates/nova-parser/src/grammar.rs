//! The actual grammar rules, implemented as a hand-written recursive descent
//! parser with chumsky-style error recovery stubs.
//!
//! Operator precedence for expressions is handled by explicit layering
//! (assignment → or → and → … → postfix), matching the spec table exactly.

use nova_ast::{
    expr::{AssignOp, BinOp, Expr, FieldInit, Literal, MatchArm, StringPart, UnOp},
    item::{
        AssocTypeBinding, Attribute, ConstDecl, ExternBlock, ExternItem, Function, FunctionSig,
        ImplBlock, Import, ImportKind, Module, Param, Record, RecordField, TraitDecl, TraitItem,
        TypeDecl, TypeDef, Variant, Visibility, WhereBound,
    },
    pattern::{FieldPat, Pattern},
    ty::{Type, TypeParam},
    Block, File, Item, Path, Stmt,
};
use nova_diagnostics::{FileId, Span, Spanned};
use nova_lexer::Token;

use crate::ParseError;

// ---------------------------------------------------------------------------
// Parser state
// ---------------------------------------------------------------------------

struct Parser<'a> {
    tokens: &'a [Spanned<Token>],
    pos: usize,
    file: FileId,
    errors: Vec<ParseError>,
    /// When `true`, `Ident {` in expression position is NOT parsed as a record
    /// literal. Set by scrutinee positions (if/while/for/match conditions) to
    /// avoid ambiguity with the following `{ block }`.
    no_struct_literal: bool,
    /// Count of closing `>` "borrowed" from a `>>` token while closing nested
    /// generic argument lists (`Option<Option<Int>>`). The lexer glues `>>`
    /// into one token, so closing an inner list splits it: one `>` closes the
    /// inner list now, the remainder is recorded here for the enclosing list.
    pending_gt: usize,
}

impl<'a> Parser<'a> {
    fn new(tokens: &'a [Spanned<Token>], file: FileId) -> Self {
        Self {
            tokens,
            pos: 0,
            file,
            errors: Vec::new(),
            no_struct_literal: false,
            pending_gt: 0,
        }
    }

    /// Run `f` with `no_struct_literal = true`, then restore the previous value.
    fn in_no_struct_ctx<R>(&mut self, f: impl FnOnce(&mut Self) -> R) -> R {
        let prev = self.no_struct_literal;
        self.no_struct_literal = true;
        let result = f(self);
        self.no_struct_literal = prev;
        result
    }

    /// Run `f` with `no_struct_literal = false`, then restore the previous
    /// value. Used for a `${…}` interpolation hole: the ambiguity that motivates
    /// the flag cannot arise there, because the hole's own `}` is a distinct
    /// `InterpClose` token and no `{ block }` can follow it.
    fn in_struct_ok_ctx<R>(&mut self, f: impl FnOnce(&mut Self) -> R) -> R {
        let prev = self.no_struct_literal;
        self.no_struct_literal = false;
        let result = f(self);
        self.no_struct_literal = prev;
        result
    }

    // --- Token access ---

    fn peek(&self) -> &Token {
        self.tokens
            .get(self.pos)
            .map(|s| &s.value)
            .unwrap_or(&Token::Eof)
    }

    fn peek_span(&self) -> Span {
        self.tokens
            .get(self.pos)
            .map(|s| s.span)
            .unwrap_or_else(|| Span::point(0, self.file))
    }

    fn advance(&mut self) -> Spanned<Token> {
        let tok = self
            .tokens
            .get(self.pos)
            .cloned()
            .unwrap_or_else(|| Spanned::new(Token::Eof, Span::point(0, self.file)));
        if self.pos < self.tokens.len() {
            self.pos += 1;
        }
        tok
    }

    fn check(&self, tok: &Token) -> bool {
        std::mem::discriminant(self.peek()) == std::mem::discriminant(tok)
    }

    fn eat(&mut self, tok: &Token) -> Option<Spanned<Token>> {
        if std::mem::discriminant(self.peek()) == std::mem::discriminant(tok) {
            Some(self.advance())
        } else {
            None
        }
    }

    fn expect(&mut self, tok: &Token, ctx: &str) -> Option<Spanned<Token>> {
        if std::mem::discriminant(self.peek()) == std::mem::discriminant(tok) {
            Some(self.advance())
        } else {
            let span = self.peek_span();
            self.errors.push(ParseError::Expected {
                expected: format!("{} (in {})", tok.description(), ctx),
                found: self.peek().description().to_owned(),
                span,
            });
            None
        }
    }

    fn is_at_end(&self) -> bool {
        matches!(self.peek(), Token::Eof)
    }

    // --- Synchronisation (error recovery) ---

    /// Skip tokens until we find something that looks like the start of an item
    /// or a closing delimiter. Used at item level for error recovery.
    fn sync_to_item_boundary(&mut self) {
        while !self.is_at_end() {
            match self.peek() {
                Token::Fn
                | Token::Pub
                | Token::Record
                | Token::Trait
                | Token::Impl
                | Token::Type
                | Token::Const
                | Token::Import
                | Token::Module
                | Token::Extern
                | Token::RBrace => break,
                _ => {
                    self.advance();
                }
            }
        }
    }

    /// Skip to next `;` or `}` for statement recovery.
    fn sync_to_stmt_boundary(&mut self) {
        while !self.is_at_end() {
            match self.peek() {
                Token::Semicolon => {
                    self.advance();
                    break;
                }
                Token::RBrace => break,
                _ => {
                    self.advance();
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

pub(crate) fn parse_file(
    tokens: &[Spanned<Token>],
    file: FileId,
) -> (Option<File>, Vec<ParseError>) {
    let mut p = Parser::new(tokens, file);
    let ast_file = p.parse_file();
    (Some(ast_file), p.errors)
}

// ---------------------------------------------------------------------------
// File
// ---------------------------------------------------------------------------

impl<'a> Parser<'a> {
    fn parse_file(&mut self) -> File {
        let mut items = Vec::new();
        while !self.is_at_end() {
            match self.try_parse_item() {
                Some(item) => items.push(item),
                None => {
                    // Progress, guaranteed here rather than assumed from the
                    // two halves. `try_parse_item`'s fallthrough arm reports and
                    // returns `None` **without consuming** the offending token,
                    // and `sync_to_item_boundary` stops *at* its boundary tokens
                    // without consuming one either — so whenever the token that
                    // failed is itself a stop, neither advances and this loop
                    // re-peeks it forever.
                    //
                    // That combination was unreachable while every stop was an
                    // item-start keyword, because `try_parse_item` has an arm for
                    // each of those. Adding `RBrace` as a stop (so impl-body
                    // recovery cannot escape the impl) made it reachable from a
                    // two-line file: `}` followed by `fn main() { }` hung
                    // `nova check` with no output — measured, killed at 15 s.
                    //
                    // Checked-progress rather than an unconditional `advance()`
                    // before the sync, which is what the impl-body `_` arm does.
                    // There the arm has already peeked the offending token, so it
                    // knows exactly what it is discarding; here `try_parse_item`
                    // may have consumed an arbitrary prefix before failing, and
                    // an unconditional advance would then eat the *next* item's
                    // first token — turning one bad item into two. This form also
                    // keeps termination true for any stop token added later,
                    // which is the property that was quietly false above.
                    let before = self.pos;
                    self.sync_to_item_boundary();
                    if self.pos == before {
                        self.advance();
                    }
                }
            }
        }
        File { items }
    }

    /// Parse zero or more leading `@name` / `@name(a, b)` attributes.
    ///
    /// Placement is not checked here — an attribute on a record parses and is
    /// kept; whether it belongs there is the resolver's decision, not the
    /// parser's. Whole-branch review: the resolver reports `E0083` (and
    /// every other attribute diagnostic) against `attr.name.span`, never
    /// `attr.span` — the field this function builds, which is only the
    /// leading `@` — so `E0083`'s span does not come from here. The parser's
    /// job is syntax; the known-attribute set and its placement rules are
    /// Task 2's (`nova-resolver`).
    fn parse_attributes(&mut self) -> Vec<Attribute> {
        let mut attrs = Vec::new();
        while self.peek() == &Token::At {
            let start = self.peek_span();
            self.advance();
            let Some(name) = self.parse_ident("in attribute name") else {
                break;
            };
            let mut args = Vec::new();
            if self.peek() == &Token::LParen {
                self.advance();
                while self.peek() != &Token::RParen && !self.is_at_end() {
                    let Some(arg) = self.parse_ident("in attribute arguments") else {
                        break;
                    };
                    args.push(arg);
                    if self.peek() == &Token::Comma {
                        self.advance();
                    } else {
                        break;
                    }
                }
                self.expect(&Token::RParen, "in attribute arguments");
            }
            let span = start; // widen only if the file has a span-join helper;
                              // there is no `prev_span` in grammar.rs.
            attrs.push(Attribute { name, args, span });
        }
        attrs
    }

    fn try_parse_item(&mut self) -> Option<Spanned<Item>> {
        let start = self.peek_span();
        let attrs = self.parse_attributes();
        let vis = self.parse_visibility();

        let item = match self.peek() {
            Token::Fn | Token::Async => {
                let mut func = self.parse_function(vis)?;
                func.attrs = attrs;
                Item::Function(func)
            }
            Token::Record => {
                self.advance();
                let mut record = self.parse_record(vis)?;
                record.attrs = attrs;
                Item::Record(record)
            }
            Token::Type => {
                self.advance();
                let mut td = self.parse_type_decl(vis)?;
                td.attrs = attrs;
                Item::Type(td)
            }
            Token::Trait => {
                self.advance();
                let mut tr = self.parse_trait_decl(vis)?;
                tr.attrs = attrs;
                Item::Trait(tr)
            }
            Token::Impl => {
                self.advance();
                let mut impl_ = self.parse_impl_block()?;
                impl_.attrs = attrs;
                Item::Impl(impl_)
            }
            Token::Const => {
                self.advance();
                let mut c = self.parse_const_decl(vis)?;
                c.attrs = attrs;
                Item::Const(c)
            }
            Token::Import => {
                self.advance();
                let mut imp = self.parse_import()?;
                imp.attrs = attrs;
                Item::Import(imp)
            }
            Token::Module => {
                self.advance();
                let mut m = self.parse_module()?;
                m.attrs = attrs;
                Item::Module(m)
            }
            Token::Extern => {
                self.advance();
                let mut e = self.parse_extern_block()?;
                e.attrs = attrs;
                Item::Extern(e)
            }
            _ => {
                let span = self.peek_span();
                self.errors.push(ParseError::Expected {
                    expected: "item (fn, record, type, trait, impl, const, import, module, extern)"
                        .into(),
                    found: self.peek().description().to_owned(),
                    span,
                });
                return None;
            }
        };

        let end = self
            .tokens
            .get(self.pos.saturating_sub(1))
            .map(|s| s.span)
            .unwrap_or(start);
        let span = start.merge(end);
        Some(Spanned::new(item, span))
    }

    fn parse_visibility(&mut self) -> Visibility {
        if self.eat(&Token::Pub).is_some() {
            Visibility::Pub
        } else {
            Visibility::Private
        }
    }
}

// ---------------------------------------------------------------------------
// Functions
// ---------------------------------------------------------------------------

impl<'a> Parser<'a> {
    fn parse_function(&mut self, vis: Visibility) -> Option<Function> {
        let is_async = self.eat(&Token::Async).is_some();
        self.expect(&Token::Fn, "function declaration")?;
        let name = self.parse_ident("function name")?;
        let generics = self.parse_generics_opt();
        self.expect(&Token::LParen, "function parameters")?;
        let params = self.parse_params();
        self.expect(&Token::RParen, "function parameters")?;
        let return_ty = if self.eat(&Token::Arrow).is_some() {
            Some(self.parse_type("return type")?)
        } else {
            None
        };
        let where_clause = self.parse_where_clause_opt();
        let body = self.parse_block("function body")?;

        Some(Function {
            attrs: Vec::new(),
            vis,
            is_async,
            name,
            generics,
            params,
            return_ty,
            where_clause,
            body,
        })
    }

    fn parse_function_sig(&mut self) -> Option<FunctionSig> {
        let is_async = self.eat(&Token::Async).is_some();
        self.expect(&Token::Fn, "function signature")?;
        let name = self.parse_ident("function name")?;
        let generics = self.parse_generics_opt();
        self.expect(&Token::LParen, "function parameters")?;
        let params = self.parse_params();
        self.expect(&Token::RParen, "function parameters")?;
        let return_ty = if self.eat(&Token::Arrow).is_some() {
            Some(self.parse_type("return type")?)
        } else {
            None
        };
        let where_clause = self.parse_where_clause_opt();
        Some(FunctionSig {
            is_async,
            name,
            generics,
            params,
            return_ty,
            where_clause,
        })
    }

    fn parse_params(&mut self) -> Vec<Param> {
        let mut params = Vec::new();
        while !self.check(&Token::RParen) && !self.is_at_end() {
            if let Some(p) = self.parse_param() {
                params.push(p);
            }
            if self.eat(&Token::Comma).is_none() {
                break;
            }
        }
        params
    }

    fn parse_param(&mut self) -> Option<Param> {
        let is_mut = self.eat(&Token::Mut).is_some();
        let name = self.parse_ident("parameter name")?;

        // `self` may appear without a type annotation in method signatures.
        if name.value == "self" && !self.check(&Token::Colon) {
            let span = name.span;
            return Some(Param {
                is_mut,
                name,
                ty: Spanned::new(Type::Infer, span),
            });
        }

        self.expect(&Token::Colon, "parameter type")?;
        let ty = self.parse_type("parameter type")?;
        Some(Param { is_mut, name, ty })
    }

    fn parse_generics_opt(&mut self) -> Vec<TypeParam> {
        if !self.check(&Token::Lt) {
            return Vec::new();
        }
        self.advance(); // <
        let mut params = Vec::new();
        while !self.check(&Token::Gt) && !self.is_at_end() {
            let name = match self.parse_ident("type parameter") {
                Some(n) => n,
                None => break,
            };
            let mut bounds = Vec::new();
            if self.eat(&Token::Colon).is_some() {
                bounds = self.parse_trait_bounds();
            }
            params.push(TypeParam { name, bounds });
            if self.eat(&Token::Comma).is_none() {
                break;
            }
        }
        self.eat(&Token::Gt);
        params
    }

    fn parse_trait_bounds(&mut self) -> Vec<Spanned<Path>> {
        let mut bounds = Vec::new();
        loop {
            let span = self.peek_span();
            if let Some(path) = self.try_parse_path() {
                bounds.push(Spanned::new(path, span));
            } else {
                break;
            }
            if self.eat(&Token::Plus).is_none() {
                break;
            }
        }
        bounds
    }

    fn parse_where_clause_opt(&mut self) -> Vec<WhereBound> {
        if !self.check(&Token::Where) {
            return Vec::new();
        }
        self.advance(); // where
        let mut bounds = Vec::new();
        while let Some(ty) = self.parse_type("where bound type") {
            if self.expect(&Token::Colon, "where bound").is_none() {
                break;
            }
            let trait_bounds = self.parse_trait_bounds();
            bounds.push(WhereBound {
                ty,
                bounds: trait_bounds,
            });
            if self.eat(&Token::Comma).is_none() {
                break;
            }
        }
        bounds
    }
}

// ---------------------------------------------------------------------------
// Records, types, traits, impls, const, import, module, extern
// ---------------------------------------------------------------------------

impl<'a> Parser<'a> {
    fn parse_record(&mut self, vis: Visibility) -> Option<Record> {
        let name = self.parse_ident("record name")?;
        let generics = self.parse_generics_opt();
        self.expect(&Token::LBrace, "record body")?;
        let mut fields = Vec::new();
        while !self.check(&Token::RBrace) && !self.is_at_end() {
            let fvis = self.parse_visibility();
            let fname = match self.parse_ident("field name") {
                Some(n) => n,
                None => {
                    self.sync_to_stmt_boundary();
                    continue;
                }
            };
            if self.expect(&Token::Colon, "field type").is_none() {
                self.sync_to_stmt_boundary();
                continue;
            }
            let fty = match self.parse_type("field type") {
                Some(t) => t,
                None => {
                    self.sync_to_stmt_boundary();
                    continue;
                }
            };
            self.eat(&Token::Comma);
            fields.push(RecordField {
                vis: fvis,
                name: fname,
                ty: fty,
            });
        }
        self.expect(&Token::RBrace, "record body")?;
        Some(Record {
            attrs: Vec::new(),
            vis,
            name,
            generics,
            fields,
        })
    }

    fn parse_type_decl(&mut self, vis: Visibility) -> Option<TypeDecl> {
        let name = self.parse_ident("type name")?;
        let generics = self.parse_generics_opt();
        self.expect(&Token::Eq, "type definition")?;

        // Sum type: starts with `|`
        let def = if self.check(&Token::Pipe) {
            let mut variants = Vec::new();
            while self.eat(&Token::Pipe).is_some() {
                let vname = match self.parse_ident("variant name") {
                    Some(n) => n,
                    None => break,
                };
                let mut fields = Vec::new();
                if self.eat(&Token::LParen).is_some() {
                    while !self.check(&Token::RParen) && !self.is_at_end() {
                        if let Some(t) = self.parse_type("variant field type") {
                            fields.push(t);
                        }
                        if self.eat(&Token::Comma).is_none() {
                            break;
                        }
                    }
                    self.eat(&Token::RParen);
                }
                variants.push(Variant {
                    name: vname,
                    fields,
                });
            }
            TypeDef::Sum(variants)
        } else {
            let ty = self.parse_type("type alias")?;
            TypeDef::Alias(ty)
        };

        self.eat(&Token::Semicolon);
        Some(TypeDecl {
            attrs: Vec::new(),
            vis,
            name,
            generics,
            def,
        })
    }

    fn parse_trait_decl(&mut self, vis: Visibility) -> Option<TraitDecl> {
        let name = self.parse_ident("trait name")?;
        let generics = self.parse_generics_opt();
        let mut supertraits = Vec::new();
        if self.eat(&Token::Colon).is_some() {
            supertraits = self.parse_trait_bounds();
        }
        let where_clause = self.parse_where_clause_opt();
        self.expect(&Token::LBrace, "trait body")?;
        let mut items = Vec::new();
        while !self.check(&Token::RBrace) && !self.is_at_end() {
            // An associated type declaration (`type Item` or `type Item:
            // Display`) must be handled before the speculative
            // `parse_function_sig()` below: that call expects `fn` as its
            // very first token, so reaching it with `type` ahead fails and
            // falls into `sync_to_stmt_boundary()`, producing a spurious
            // parse error even though this arm exists.
            if self.eat(&Token::Type).is_some() {
                if let Some(name) = self.parse_ident("associated type name") {
                    let bounds = if self.eat(&Token::Colon).is_some() {
                        self.parse_trait_bounds()
                    } else {
                        Vec::new()
                    };
                    self.eat(&Token::Semicolon);
                    items.push(TraitItem::AssocType { name, bounds });
                } else {
                    self.sync_to_stmt_boundary();
                }
                continue;
            }
            // Look ahead: if there's a body `{`, it's a provided method
            let saved_pos = self.pos;
            let saved_errors_len = self.errors.len();
            let is_async = self.check(&Token::Async);
            if let Some(sig) = self.parse_function_sig() {
                if self.check(&Token::LBrace) {
                    // Provided method — we need to re-parse as Function.
                    // Roll back and parse as full function.
                    self.pos = saved_pos;
                    self.errors.truncate(saved_errors_len);
                    let func_vis = Visibility::Private;
                    if let Some(func) = self.parse_function(func_vis) {
                        items.push(TraitItem::Provided(func));
                    }
                } else {
                    // Required method — expect semicolon.
                    self.eat(&Token::Semicolon);
                    items.push(TraitItem::Required(sig));
                }
            } else {
                self.sync_to_stmt_boundary();
            }
            let _ = is_async;
        }
        self.expect(&Token::RBrace, "trait body")?;
        Some(TraitDecl {
            attrs: Vec::new(),
            vis,
            name,
            generics,
            supertraits,
            where_clause,
            items,
        })
    }

    fn parse_impl_block(&mut self) -> Option<ImplBlock> {
        let generics = self.parse_generics_opt();

        // Could be `impl Type` or `impl Trait for Type`
        // We need lookahead to distinguish. Try to parse a type; if followed
        // by `for`, treat it as the trait path.
        let first_ty = self.parse_type("impl target")?;
        let (trait_, ty) = if self.eat(&Token::For).is_some() {
            let for_ty = self.parse_type("impl target type")?;
            // Convert first_ty (Type) to a path for the trait
            let trait_path = type_to_path(&first_ty)?;
            let trait_span = first_ty.span;
            (Some(Spanned::new(trait_path, trait_span)), for_ty)
        } else {
            (None, first_ty)
        };

        let where_clause = self.parse_where_clause_opt();
        self.expect(&Token::LBrace, "impl block")?;
        let mut functions = Vec::new();
        let mut consts = Vec::new();
        let mut assoc_types = Vec::new();
        // Loop invariant: every arm below advances `self.pos` by at least one
        // token, so this terminates. `parse_function` opens with
        // `expect(&Token::Fn, …)`, which consumes on the match this arm has
        // already peeked; the `const` arm advances before parsing; and the
        // fallthrough arm advances explicitly (see its own comment). Anything
        // added here must keep that invariant, because neither
        // `sync_to_item_boundary` nor `sync_to_stmt_boundary` provides it —
        // both stop *at* their boundary token without consuming it.
        while !self.check(&Token::RBrace) && !self.is_at_end() {
            // Captured before `parse_visibility` consumes it, so the `pub`
            // rejection in the `type` arm can point at the `pub` itself.
            let vis_span = self.peek_span();
            let vis = self.parse_visibility();
            match self.peek() {
                Token::Fn | Token::Async => {
                    if let Some(f) = self.parse_function(vis) {
                        functions.push(f);
                    } else {
                        self.sync_to_item_boundary();
                    }
                }
                // An associated-type binding: `type Item = Int`. Unlike the
                // trait body's own `type` handling, there is no speculative
                // `parse_function_sig()` to sequence around here — this match
                // dispatches on a single peeked token, so arm order is
                // irrelevant.
                Token::Type => {
                    self.advance();
                    if vis != Visibility::Private {
                        // `ast::AssocTypeBinding` has no `vis` field on
                        // purpose (see its doc comment), so a `pub` here would
                        // otherwise be parsed and then dropped without a word.
                        self.errors.push(ParseError::Custom {
                            message: "an associated type binding in an impl cannot be `pub`".into(),
                            span: vis_span,
                        });
                    }
                    match self.parse_ident("associated type name") {
                        Some(name) => {
                            if self.expect(&Token::Eq, "associated type binding").is_some() {
                                if let Some(ty) = self.parse_type("associated type binding") {
                                    self.eat(&Token::Semicolon);
                                    assoc_types.push(AssocTypeBinding { name, ty });
                                } else {
                                    self.sync_to_stmt_boundary();
                                }
                            } else {
                                self.sync_to_stmt_boundary();
                            }
                        }
                        None => self.sync_to_stmt_boundary(),
                    }
                }
                Token::Const => {
                    self.advance();
                    if let Some(c) = self.parse_const_decl(vis) {
                        consts.push(c);
                    } else {
                        self.sync_to_stmt_boundary();
                    }
                }
                _ => {
                    let span = self.peek_span();
                    self.errors.push(ParseError::Expected {
                        expected: "fn or const inside impl".into(),
                        found: self.peek().description().to_owned(),
                        span,
                    });
                    // Consume the offending token before syncing. This is the
                    // arm's half of the loop invariant above, and it is not
                    // optional: `sync_to_item_boundary` breaks at any of ten
                    // item-start tokens (`fn`, `pub`, `record`, `trait`,
                    // `impl`, `type`, `const`, `import`, `module`, `extern`)
                    // *without* consuming one, so a token that is both
                    // unexpected here and an item start — every one of
                    // `record`, `trait`, `impl`, `type`, `import`, `module`,
                    // `extern` inside an impl body — was re-peeked forever.
                    // Syncing alone recovers only from tokens that are not
                    // item starts, which is why `impl W { 42 }` always worked
                    // and `impl W { record R { } }` never terminated.
                    self.advance();
                    self.sync_to_item_boundary();
                }
            }
        }
        self.expect(&Token::RBrace, "impl block")?;
        Some(ImplBlock {
            attrs: Vec::new(),
            generics,
            trait_,
            ty,
            where_clause,
            functions,
            consts,
            assoc_types,
        })
    }

    fn parse_const_decl(&mut self, vis: Visibility) -> Option<ConstDecl> {
        let name = self.parse_ident("const name")?;
        self.expect(&Token::Colon, "const type")?;
        let ty = self.parse_type("const type")?;
        self.expect(&Token::Eq, "const value")?;
        let value = self.parse_expr("const value")?;
        self.eat(&Token::Semicolon);
        Some(ConstDecl {
            attrs: Vec::new(),
            vis,
            name,
            ty,
            value,
        })
    }

    fn parse_import(&mut self) -> Option<Import> {
        let start = self.peek_span();
        let path = self.parse_path("import path")?;
        let path_span = start.merge(
            self.tokens
                .get(self.pos.saturating_sub(1))
                .map(|s| s.span)
                .unwrap_or(start),
        );

        let kind = if self.eat(&Token::As).is_some() {
            let alias = self.parse_ident("import alias")?;
            ImportKind::Alias(alias)
        } else if self.eat(&Token::LBrace).is_some() {
            let mut names = Vec::new();
            while !self.check(&Token::RBrace) && !self.is_at_end() {
                if let Some(n) = self.parse_ident("import name") {
                    names.push(n);
                }
                if self.eat(&Token::Comma).is_none() {
                    break;
                }
            }
            self.expect(&Token::RBrace, "import list")?;
            ImportKind::List(names)
        } else {
            ImportKind::Simple
        };

        self.eat(&Token::Semicolon);
        Some(Import {
            attrs: Vec::new(),
            path: Spanned::new(path, path_span),
            kind,
        })
    }

    fn parse_module(&mut self) -> Option<Module> {
        let start = self.peek_span();
        let path = self.parse_path("module path")?;
        let span = start.merge(
            self.tokens
                .get(self.pos.saturating_sub(1))
                .map(|s| s.span)
                .unwrap_or(start),
        );
        self.eat(&Token::Semicolon);
        Some(Module {
            attrs: Vec::new(),
            path: Spanned::new(path, span),
        })
    }

    fn parse_extern_block(&mut self) -> Option<ExternBlock> {
        let abi = if let Token::StrStart = self.peek() {
            // consume StrStart, StrPart (the ABI string), StrEnd
            self.advance();
            let abi = if let Token::StrPart(s) = &self.peek().clone() {
                let s = s.clone();
                self.advance();
                Some(s)
            } else {
                None
            };
            self.eat(&Token::StrEnd);
            abi
        } else {
            None
        };

        self.expect(&Token::LBrace, "extern block")?;
        let mut items = Vec::new();
        while !self.check(&Token::RBrace) && !self.is_at_end() {
            if let Some(sig) = self.parse_function_sig() {
                self.eat(&Token::Semicolon);
                items.push(ExternItem::Fn(sig));
            } else {
                self.sync_to_stmt_boundary();
            }
        }
        self.expect(&Token::RBrace, "extern block")?;
        Some(ExternBlock {
            attrs: Vec::new(),
            abi,
            items,
        })
    }
}

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

impl<'a> Parser<'a> {
    fn parse_type(&mut self, ctx: &str) -> Option<Spanned<Type>> {
        let start = self.peek_span();
        let mut ty = self.parse_type_atom(ctx)?;

        // Optional sugar: `T?`
        while self.eat(&Token::Question).is_some() {
            let end = self
                .tokens
                .get(self.pos.saturating_sub(1))
                .map(|s| s.span)
                .unwrap_or(start);
            ty = Spanned::new(Type::Optional(Box::new(ty)), start.merge(end));
        }

        Some(ty)
    }

    fn parse_type_atom(&mut self, ctx: &str) -> Option<Spanned<Type>> {
        let start = self.peek_span();
        match self.peek().clone() {
            Token::Amp => {
                self.advance();
                let is_mut = self.eat(&Token::Mut).is_some();
                let inner = self.parse_type(ctx)?;
                let span = start.merge(inner.span);
                Some(Spanned::new(
                    Type::Ref {
                        is_mut,
                        inner: Box::new(inner),
                    },
                    span,
                ))
            }
            Token::Star => {
                self.advance();
                let is_mut = self.eat(&Token::Mut).is_some();
                let inner = self.parse_type(ctx)?;
                let span = start.merge(inner.span);
                Some(Spanned::new(
                    Type::Ptr {
                        is_mut,
                        inner: Box::new(inner),
                    },
                    span,
                ))
            }
            Token::LBracket => {
                self.advance();
                let inner = self.parse_type(ctx)?;
                self.expect(&Token::RBracket, "array type")?;
                let end = self
                    .tokens
                    .get(self.pos.saturating_sub(1))
                    .map(|s| s.span)
                    .unwrap_or(start);
                Some(Spanned::new(Type::Array(Box::new(inner)), start.merge(end)))
            }
            Token::LParen => {
                self.advance();
                if self.eat(&Token::RParen).is_some() {
                    // unit type `()`
                    let end = self
                        .tokens
                        .get(self.pos.saturating_sub(1))
                        .map(|s| s.span)
                        .unwrap_or(start);
                    return Some(Spanned::new(Type::Tuple(vec![]), start.merge(end)));
                }
                let first = self.parse_type(ctx)?;
                if self.eat(&Token::RParen).is_some() {
                    // Single-element paren — not a tuple, just the type
                    let end = self
                        .tokens
                        .get(self.pos.saturating_sub(1))
                        .map(|s| s.span)
                        .unwrap_or(start);
                    return Some(Spanned::new(first.value, start.merge(end)));
                }
                // Tuple
                let mut elems = vec![first];
                while self.eat(&Token::Comma).is_some() && !self.check(&Token::RParen) {
                    if let Some(t) = self.parse_type(ctx) {
                        elems.push(t);
                    }
                }
                self.expect(&Token::RParen, "tuple type")?;
                let end = self
                    .tokens
                    .get(self.pos.saturating_sub(1))
                    .map(|s| s.span)
                    .unwrap_or(start);
                Some(Spanned::new(Type::Tuple(elems), start.merge(end)))
            }
            Token::Fn => {
                self.advance();
                self.expect(&Token::LParen, "fn type params")?;
                let mut params = Vec::new();
                while !self.check(&Token::RParen) && !self.is_at_end() {
                    if let Some(t) = self.parse_type(ctx) {
                        params.push(t);
                    }
                    if self.eat(&Token::Comma).is_none() {
                        break;
                    }
                }
                self.expect(&Token::RParen, "fn type params")?;
                let ret = if self.eat(&Token::Arrow).is_some() {
                    self.parse_type(ctx)?
                } else {
                    let unit_span = self.peek_span();
                    Spanned::new(Type::Tuple(vec![]), unit_span)
                };
                let end = ret.span;
                Some(Spanned::new(
                    Type::Fn {
                        params,
                        ret: Box::new(ret),
                    },
                    start.merge(end),
                ))
            }
            Token::Ident(_) | Token::SelfUpper => {
                let path = self.parse_path(ctx)?;
                // Generic args `<T, U>`
                let args = self.parse_generic_args_opt(ctx);
                let end = self
                    .tokens
                    .get(self.pos.saturating_sub(1))
                    .map(|s| s.span)
                    .unwrap_or(start);
                Some(Spanned::new(Type::Path { path, args }, start.merge(end)))
            }
            _ => {
                let span = self.peek_span();
                self.errors.push(ParseError::Expected {
                    expected: format!("type (in {})", ctx),
                    found: self.peek().description().to_owned(),
                    span,
                });
                None
            }
        }
    }

    /// Whether the cursor is at a closing `>` for a generic argument list: a
    /// `>` pending from an earlier split `>>`, or a `>` / `>>` token.
    fn at_generic_close(&self) -> bool {
        self.pending_gt > 0 || matches!(self.peek(), Token::Gt | Token::GtGt)
    }

    /// Consume one closing `>`. A glued `>>` token yields one `>` now and
    /// records the other in `pending_gt` for the enclosing list, so
    /// `Option<Option<Int>>` closes correctly. Returns false if not at a `>`.
    fn eat_generic_close(&mut self) -> bool {
        if self.pending_gt > 0 {
            self.pending_gt -= 1;
            return true;
        }
        match self.peek() {
            Token::Gt => {
                self.advance();
                true
            }
            Token::GtGt => {
                self.advance();
                self.pending_gt += 1;
                true
            }
            _ => false,
        }
    }

    fn parse_generic_args_opt(&mut self, ctx: &str) -> Vec<Spanned<Type>> {
        if !self.check(&Token::Lt) {
            return Vec::new();
        }
        // Peek ahead: if next after `<` looks like a type, treat as generic args.
        // Otherwise this `<` might be a comparison operator.
        let saved = self.pos;
        let saved_pending = self.pending_gt;
        self.advance(); // consume `<`
        let mut args = Vec::new();
        while !self.at_generic_close() && !self.is_at_end() {
            if let Some(t) = self.parse_type(ctx) {
                args.push(t);
            } else {
                // Wasn't a type — roll back
                self.pos = saved;
                self.pending_gt = saved_pending;
                return Vec::new();
            }
            // A `>` split from a `>>` now closes this list; a following comma (if
            // any) belongs to an enclosing list, so stop here.
            if self.pending_gt > 0 {
                break;
            }
            if self.eat(&Token::Comma).is_none() {
                break;
            }
        }
        if !self.eat_generic_close() {
            // Didn't close — roll back
            self.pos = saved;
            self.pending_gt = saved_pending;
            return Vec::new();
        }
        args
    }
}

// ---------------------------------------------------------------------------
// Blocks and statements
// ---------------------------------------------------------------------------

impl<'a> Parser<'a> {
    fn parse_block(&mut self, ctx: &str) -> Option<Spanned<Block>> {
        let start = self.peek_span();
        self.expect(&Token::LBrace, ctx)?;
        let mut stmts = Vec::new();
        let mut trailing: Option<Box<Spanned<Expr>>> = None;

        while !self.check(&Token::RBrace) && !self.is_at_end() {
            // Check if this could be the trailing expression.
            // Parse a statement; if it's an expression without semicolon at the
            // block-final position, treat as trailing.
            if let Some(stmt) = self.try_parse_stmt() {
                match stmt.value {
                    Stmt::Expr(ref e) if !self.check(&Token::RBrace) => {
                        stmts.push(stmt);
                    }
                    Stmt::Expr(e) => {
                        // Could be trailing — but we already consumed it.
                        // Check if there's a semicolon; if not, it's trailing.
                        trailing = Some(Box::new(e));
                    }
                    _ => {
                        stmts.push(stmt);
                    }
                }
            } else {
                self.sync_to_stmt_boundary();
            }
        }

        self.expect(&Token::RBrace, ctx)?;
        let end = self
            .tokens
            .get(self.pos.saturating_sub(1))
            .map(|s| s.span)
            .unwrap_or(start);
        Some(Spanned::new(Block { stmts, trailing }, start.merge(end)))
    }

    fn try_parse_stmt(&mut self) -> Option<Spanned<Stmt>> {
        let start = self.peek_span();

        // `let` statement
        if self.check(&Token::Let) {
            return self.parse_let_stmt();
        }

        // Nested items
        if matches!(
            self.peek(),
            Token::Fn
                | Token::Async
                | Token::Record
                | Token::Trait
                | Token::Impl
                | Token::Type
                | Token::Const
                | Token::Import
                | Token::Module
                | Token::Extern
        ) || (self.check(&Token::Pub) && {
            // check next token
            matches!(
                self.tokens.get(self.pos + 1).map(|s| &s.value),
                Some(Token::Fn)
                    | Some(Token::Record)
                    | Some(Token::Trait)
                    | Some(Token::Type)
                    | Some(Token::Const)
                    | Some(Token::Impl)
            )
        }) {
            let item = self.try_parse_item()?;
            let span = item.span;
            return Some(Spanned::new(Stmt::Item(Box::new(item.value)), span));
        }

        // Expression statement
        let expr = self.parse_expr("statement")?;
        let has_semi = self.eat(&Token::Semicolon).is_some();
        let end = self
            .tokens
            .get(self.pos.saturating_sub(1))
            .map(|s| s.span)
            .unwrap_or(start);
        let span = start.merge(end);

        let _ = has_semi;
        Some(Spanned::new(Stmt::Expr(expr), span))
    }

    fn parse_let_stmt(&mut self) -> Option<Spanned<Stmt>> {
        let start = self.peek_span();
        self.expect(&Token::Let, "let statement")?;
        let is_mut = self.eat(&Token::Mut).is_some();
        let pattern = self.parse_pattern("let pattern")?;
        let ty = if self.eat(&Token::Colon).is_some() {
            Some(self.parse_type("let type")?)
        } else {
            None
        };
        let init = if self.eat(&Token::Eq).is_some() {
            Some(self.parse_expr("let initializer")?)
        } else {
            None
        };
        self.eat(&Token::Semicolon);
        let end = self
            .tokens
            .get(self.pos.saturating_sub(1))
            .map(|s| s.span)
            .unwrap_or(start);
        Some(Spanned::new(
            Stmt::Let {
                is_mut,
                pattern,
                ty,
                init,
            },
            start.merge(end),
        ))
    }
}

// ---------------------------------------------------------------------------
// Expressions (precedence climbing)
// ---------------------------------------------------------------------------

impl<'a> Parser<'a> {
    fn parse_expr(&mut self, ctx: &str) -> Option<Spanned<Expr>> {
        self.parse_assign(ctx)
    }

    fn parse_assign(&mut self, ctx: &str) -> Option<Spanned<Expr>> {
        let start = self.peek_span();
        let lhs = self.parse_range(ctx)?;

        let op = match self.peek() {
            Token::Eq => Some(AssignOp::Assign),
            Token::PlusEq => Some(AssignOp::AddAssign),
            Token::MinusEq => Some(AssignOp::SubAssign),
            Token::StarEq => Some(AssignOp::MulAssign),
            Token::SlashEq => Some(AssignOp::DivAssign),
            Token::PercentEq => Some(AssignOp::RemAssign),
            Token::PipeEq => Some(AssignOp::BitOrAssign),
            Token::AmpEq => Some(AssignOp::BitAndAssign),
            Token::CaretEq => Some(AssignOp::BitXorAssign),
            Token::LtLtEq => Some(AssignOp::ShlAssign),
            Token::GtGtEq => Some(AssignOp::ShrAssign),
            _ => None,
        };

        if let Some(op) = op {
            self.advance();
            let rhs = self.parse_assign(ctx)?; // right-associative
            let span = start.merge(rhs.span);
            Some(Spanned::new(
                Expr::Assign {
                    op,
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                },
                span,
            ))
        } else {
            Some(lhs)
        }
    }

    /// Range expressions `lo..hi` / `lo..=hi`, just above assignment in
    /// precedence and non-chainable.
    fn parse_range(&mut self, ctx: &str) -> Option<Spanned<Expr>> {
        let start = self.peek_span();
        let lo = self.parse_or(ctx)?;
        let inclusive = if self.eat(&Token::DotDotEq).is_some() {
            true
        } else if self.eat(&Token::DotDot).is_some() {
            false
        } else {
            return Some(lo);
        };
        let hi = self.parse_or(ctx)?;
        let span = start.merge(hi.span);
        Some(Spanned::new(
            Expr::Range {
                lo: Box::new(lo),
                hi: Box::new(hi),
                inclusive,
            },
            span,
        ))
    }

    fn parse_or(&mut self, ctx: &str) -> Option<Spanned<Expr>> {
        let start = self.peek_span();
        let mut lhs = self.parse_and(ctx)?;
        while self.eat(&Token::PipePipe).is_some() {
            let rhs = self.parse_and(ctx)?;
            let span = start.merge(rhs.span);
            lhs = Spanned::new(
                Expr::Binary {
                    op: BinOp::Or,
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                },
                span,
            );
        }
        Some(lhs)
    }

    fn parse_and(&mut self, ctx: &str) -> Option<Spanned<Expr>> {
        let start = self.peek_span();
        let mut lhs = self.parse_compare(ctx)?;
        while self.eat(&Token::AmpAmp).is_some() {
            let rhs = self.parse_compare(ctx)?;
            let span = start.merge(rhs.span);
            lhs = Spanned::new(
                Expr::Binary {
                    op: BinOp::And,
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                },
                span,
            );
        }
        Some(lhs)
    }

    fn parse_compare(&mut self, ctx: &str) -> Option<Spanned<Expr>> {
        let start = self.peek_span();
        let lhs = self.parse_bitor(ctx)?;

        let op = match self.peek() {
            Token::EqEq => Some(BinOp::Eq),
            Token::BangEq => Some(BinOp::Ne),
            Token::Lt => Some(BinOp::Lt),
            Token::LtEq => Some(BinOp::Le),
            Token::Gt => Some(BinOp::Gt),
            Token::GtEq => Some(BinOp::Ge),
            _ => None,
        };

        if let Some(op) = op {
            self.advance();
            let rhs = self.parse_bitor(ctx)?;
            let span = start.merge(rhs.span);
            let result = Spanned::new(
                Expr::Binary {
                    op,
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                },
                span,
            );
            // Non-chainable: `a < b < c` is an error
            if matches!(
                self.peek(),
                Token::EqEq | Token::BangEq | Token::Lt | Token::LtEq | Token::Gt | Token::GtEq
            ) {
                let chain_span = self.peek_span();
                self.errors
                    .push(ParseError::ChainedComparison { span: chain_span });
            }
            Some(result)
        } else {
            Some(lhs)
        }
    }

    fn parse_bitor(&mut self, ctx: &str) -> Option<Spanned<Expr>> {
        let start = self.peek_span();
        let mut lhs = self.parse_bitxor(ctx)?;
        while self.eat(&Token::Pipe).is_some() {
            let rhs = self.parse_bitxor(ctx)?;
            let span = start.merge(rhs.span);
            lhs = Spanned::new(
                Expr::Binary {
                    op: BinOp::BitOr,
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                },
                span,
            );
        }
        Some(lhs)
    }

    fn parse_bitxor(&mut self, ctx: &str) -> Option<Spanned<Expr>> {
        let start = self.peek_span();
        let mut lhs = self.parse_bitand(ctx)?;
        while self.eat(&Token::Caret).is_some() {
            let rhs = self.parse_bitand(ctx)?;
            let span = start.merge(rhs.span);
            lhs = Spanned::new(
                Expr::Binary {
                    op: BinOp::BitXor,
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                },
                span,
            );
        }
        Some(lhs)
    }

    fn parse_bitand(&mut self, ctx: &str) -> Option<Spanned<Expr>> {
        let start = self.peek_span();
        let mut lhs = self.parse_shift(ctx)?;
        while self.eat(&Token::Amp).is_some() {
            let rhs = self.parse_shift(ctx)?;
            let span = start.merge(rhs.span);
            lhs = Spanned::new(
                Expr::Binary {
                    op: BinOp::BitAnd,
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                },
                span,
            );
        }
        Some(lhs)
    }

    fn parse_shift(&mut self, ctx: &str) -> Option<Spanned<Expr>> {
        let start = self.peek_span();
        let mut lhs = self.parse_add(ctx)?;
        loop {
            let op = match self.peek() {
                Token::LtLt => BinOp::Shl,
                Token::GtGt => BinOp::Shr,
                _ => break,
            };
            self.advance();
            let rhs = self.parse_add(ctx)?;
            let span = start.merge(rhs.span);
            lhs = Spanned::new(
                Expr::Binary {
                    op,
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                },
                span,
            );
        }
        Some(lhs)
    }

    fn parse_add(&mut self, ctx: &str) -> Option<Spanned<Expr>> {
        let start = self.peek_span();
        let mut lhs = self.parse_mul(ctx)?;
        loop {
            let op = match self.peek() {
                Token::Plus => BinOp::Add,
                Token::Minus => BinOp::Sub,
                _ => break,
            };
            self.advance();
            let rhs = self.parse_mul(ctx)?;
            let span = start.merge(rhs.span);
            lhs = Spanned::new(
                Expr::Binary {
                    op,
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                },
                span,
            );
        }
        Some(lhs)
    }

    fn parse_mul(&mut self, ctx: &str) -> Option<Spanned<Expr>> {
        let start = self.peek_span();
        let mut lhs = self.parse_cast(ctx)?;
        loop {
            let op = match self.peek() {
                Token::Star => BinOp::Mul,
                Token::Slash => BinOp::Div,
                Token::Percent => BinOp::Rem,
                _ => break,
            };
            self.advance();
            let rhs = self.parse_cast(ctx)?;
            let span = start.merge(rhs.span);
            lhs = Spanned::new(
                Expr::Binary {
                    op,
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                },
                span,
            );
        }
        Some(lhs)
    }

    fn parse_cast(&mut self, ctx: &str) -> Option<Spanned<Expr>> {
        let start = self.peek_span();
        let mut expr = self.parse_unary(ctx)?;
        loop {
            if self.eat(&Token::As).is_some() {
                let ty = self.parse_type(ctx)?;
                let span = start.merge(ty.span);
                expr = Spanned::new(
                    Expr::Cast {
                        expr: Box::new(expr),
                        ty,
                    },
                    span,
                );
            } else {
                break;
            }
        }
        Some(expr)
    }

    fn parse_unary(&mut self, ctx: &str) -> Option<Spanned<Expr>> {
        let start = self.peek_span();
        let op = match self.peek() {
            Token::Minus => Some(UnOp::Neg),
            Token::Bang => Some(UnOp::Not),
            Token::Tilde => Some(UnOp::BitNot),
            Token::Star => Some(UnOp::Deref),
            Token::Amp => {
                // Could be `&` or `&mut`
                self.advance();
                let is_mut = self.eat(&Token::Mut).is_some();
                let expr = self.parse_postfix(ctx)?;
                let span = start.merge(expr.span);
                let op = if is_mut { UnOp::RefMut } else { UnOp::Ref };
                return Some(Spanned::new(
                    Expr::Unary {
                        op,
                        expr: Box::new(expr),
                    },
                    span,
                ));
            }
            _ => None,
        };

        if let Some(op) = op {
            self.advance();
            let expr = self.parse_unary(ctx)?;
            let span = start.merge(expr.span);
            Some(Spanned::new(
                Expr::Unary {
                    op,
                    expr: Box::new(expr),
                },
                span,
            ))
        } else {
            self.parse_postfix(ctx)
        }
    }

    fn parse_postfix(&mut self, ctx: &str) -> Option<Spanned<Expr>> {
        let start = self.peek_span();
        let mut expr = self.parse_atom(ctx)?;

        loop {
            let cur_span = expr.span;
            match self.peek().clone() {
                Token::LParen => {
                    // Function call
                    self.advance();
                    let mut args = Vec::new();
                    while !self.check(&Token::RParen) && !self.is_at_end() {
                        if let Some(a) = self.parse_expr(ctx) {
                            args.push(a);
                        }
                        if self.eat(&Token::Comma).is_none() {
                            break;
                        }
                    }
                    self.expect(&Token::RParen, "call arguments")?;
                    let end = self
                        .tokens
                        .get(self.pos.saturating_sub(1))
                        .map(|s| s.span)
                        .unwrap_or(cur_span);
                    expr = Spanned::new(
                        Expr::Call {
                            callee: Box::new(expr),
                            args,
                        },
                        start.merge(end),
                    );
                }
                Token::LBracket => {
                    // Index
                    self.advance();
                    let index = self.parse_expr(ctx)?;
                    self.expect(&Token::RBracket, "index expression")?;
                    let end = self
                        .tokens
                        .get(self.pos.saturating_sub(1))
                        .map(|s| s.span)
                        .unwrap_or(cur_span);
                    expr = Spanned::new(
                        Expr::Index {
                            target: Box::new(expr),
                            index: Box::new(index),
                        },
                        start.merge(end),
                    );
                }
                Token::Dot => {
                    self.advance();
                    // `.await` or `.field` or `.method()`
                    if self.eat(&Token::Await).is_some() {
                        let end = self
                            .tokens
                            .get(self.pos.saturating_sub(1))
                            .map(|s| s.span)
                            .unwrap_or(cur_span);
                        expr = Spanned::new(Expr::Await(Box::new(expr)), start.merge(end));
                    } else {
                        let field = self.parse_ident("field access")?;
                        let end = field.span;
                        expr = Spanned::new(
                            Expr::Field {
                                target: Box::new(expr),
                                field,
                            },
                            start.merge(end),
                        );
                    }
                }
                Token::Question => {
                    self.advance();
                    let end = self
                        .tokens
                        .get(self.pos.saturating_sub(1))
                        .map(|s| s.span)
                        .unwrap_or(cur_span);
                    expr = Spanned::new(Expr::Try(Box::new(expr)), start.merge(end));
                }
                _ => break,
            }
        }

        Some(expr)
    }

    fn parse_atom(&mut self, ctx: &str) -> Option<Spanned<Expr>> {
        let start = self.peek_span();
        match self.peek().clone() {
            // Literals
            Token::Int(n) => {
                self.advance();
                Some(Spanned::new(Expr::Lit(Literal::Int(n)), start))
            }
            Token::Float(f) => {
                self.advance();
                Some(Spanned::new(Expr::Lit(Literal::Float(f)), start))
            }
            Token::Char(c) => {
                self.advance();
                Some(Spanned::new(Expr::Lit(Literal::Char(c)), start))
            }
            Token::True => {
                self.advance();
                Some(Spanned::new(Expr::Lit(Literal::Bool(true)), start))
            }
            Token::False => {
                self.advance();
                Some(Spanned::new(Expr::Lit(Literal::Bool(false)), start))
            }

            // String interpolation
            Token::StrStart => self.parse_string_expr(),
            Token::RawStr(s) => {
                let s = s.clone();
                self.advance();
                Some(Spanned::new(Expr::Lit(Literal::Str(s)), start))
            }

            // Parenthesised / tuple
            Token::LParen => {
                self.advance();
                if self.eat(&Token::RParen).is_some() {
                    // Unit `()`
                    let end = self
                        .tokens
                        .get(self.pos.saturating_sub(1))
                        .map(|s| s.span)
                        .unwrap_or(start);
                    return Some(Spanned::new(Expr::Tuple(vec![]), start.merge(end)));
                }
                let first = self.parse_expr(ctx)?;
                if self.eat(&Token::RParen).is_some() {
                    // Just a parenthesised expression
                    let end = self
                        .tokens
                        .get(self.pos.saturating_sub(1))
                        .map(|s| s.span)
                        .unwrap_or(start);
                    return Some(Spanned::new(first.value, start.merge(end)));
                }
                // Tuple
                let mut elems = vec![first];
                while self.eat(&Token::Comma).is_some() && !self.check(&Token::RParen) {
                    if let Some(e) = self.parse_expr(ctx) {
                        elems.push(e);
                    }
                }
                self.expect(&Token::RParen, "tuple")?;
                let end = self
                    .tokens
                    .get(self.pos.saturating_sub(1))
                    .map(|s| s.span)
                    .unwrap_or(start);
                Some(Spanned::new(Expr::Tuple(elems), start.merge(end)))
            }

            // Array literal — either `[a, b, c]` or the repeat form `[init; n]`
            Token::LBracket => {
                self.advance();
                let mut elems = Vec::new();
                let mut first = true;
                while !self.check(&Token::RBracket) && !self.is_at_end() {
                    if let Some(e) = self.parse_expr(ctx) {
                        // `[init; n]`: a `;` after the *first* element switches
                        // to the repeat form, whose second operand is a length
                        // rather than another element.
                        if first && self.eat(&Token::Semicolon).is_some() {
                            let len = self.parse_expr(ctx)?;
                            self.expect(&Token::RBracket, "repeat array literal")?;
                            let end = self
                                .tokens
                                .get(self.pos.saturating_sub(1))
                                .map(|s| s.span)
                                .unwrap_or(start);
                            return Some(Spanned::new(
                                Expr::ArrayRepeat {
                                    init: Box::new(e),
                                    len: Box::new(len),
                                },
                                start.merge(end),
                            ));
                        }
                        elems.push(e);
                    }
                    first = false;
                    if self.eat(&Token::Comma).is_none() {
                        break;
                    }
                }
                self.expect(&Token::RBracket, "array literal")?;
                let end = self
                    .tokens
                    .get(self.pos.saturating_sub(1))
                    .map(|s| s.span)
                    .unwrap_or(start);
                Some(Spanned::new(Expr::Array(elems), start.merge(end)))
            }

            // Block expression
            Token::LBrace => {
                let block = self.parse_block(ctx)?;
                let span = block.span;
                Some(Spanned::new(Expr::Block(block.value), span))
            }

            // If
            Token::If => self.parse_if_expr(ctx),

            // Match
            Token::Match => self.parse_match_expr(ctx),

            // While
            Token::While => {
                self.advance();
                let cond = self.in_no_struct_ctx(|p| p.parse_expr(ctx))?;
                let body = self.parse_block(ctx)?;
                let span = start.merge(body.span);
                Some(Spanned::new(
                    Expr::While {
                        cond: Box::new(cond),
                        body: Box::new(body),
                    },
                    span,
                ))
            }

            // For
            Token::For => {
                self.advance();
                let pattern = self.parse_pattern(ctx)?;
                self.expect(&Token::In, "for loop")?;
                let iter = self.in_no_struct_ctx(|p| p.parse_expr(ctx))?;
                let body = self.parse_block(ctx)?;
                let span = start.merge(body.span);
                Some(Spanned::new(
                    Expr::For {
                        pattern,
                        iter: Box::new(iter),
                        body: Box::new(body),
                    },
                    span,
                ))
            }

            // Return
            Token::Return => {
                self.advance();
                // Optional expression
                let val = if !matches!(self.peek(), Token::Semicolon | Token::RBrace | Token::Eof) {
                    Some(Box::new(self.parse_expr(ctx)?))
                } else {
                    None
                };
                let end = val.as_ref().map(|e| e.span).unwrap_or(start);
                Some(Spanned::new(Expr::Return(val), start.merge(end)))
            }

            // Break
            Token::Break => {
                self.advance();
                let val = if !matches!(self.peek(), Token::Semicolon | Token::RBrace | Token::Eof) {
                    Some(Box::new(self.parse_expr(ctx)?))
                } else {
                    None
                };
                let end = val.as_ref().map(|e| e.span).unwrap_or(start);
                Some(Spanned::new(Expr::Break(val), start.merge(end)))
            }

            Token::Continue => {
                self.advance();
                Some(Spanned::new(Expr::Continue, start))
            }

            // Closure: |params| body
            Token::Pipe => self.parse_closure(ctx),

            // Path or record literal
            Token::Ident(_) | Token::SelfLower | Token::SelfUpper => {
                let path = self.parse_path(ctx)?;
                let path_end = self
                    .tokens
                    .get(self.pos.saturating_sub(1))
                    .map(|s| s.span)
                    .unwrap_or(start);

                // Record literal: `Type { field: val, ... }` — only when not in a
                // scrutinee context (if/while/for/match conditions), where `{` is the block.
                if self.check(&Token::LBrace) && !self.no_struct_literal {
                    return self.parse_record_literal(path, start);
                }

                Some(Spanned::new(Expr::Path(path), start.merge(path_end)))
            }

            _ => {
                let span = self.peek_span();
                self.errors.push(ParseError::Expected {
                    expected: format!("expression (in {})", ctx),
                    found: self.peek().description().to_owned(),
                    span,
                });
                None
            }
        }
    }

    fn parse_string_expr(&mut self) -> Option<Spanned<Expr>> {
        let start = self.peek_span();
        self.expect(&Token::StrStart, "string literal")?;
        let mut parts = Vec::new();

        loop {
            match self.peek().clone() {
                Token::StrEnd => {
                    self.advance();
                    break;
                }
                Token::StrPart(s) => {
                    let s = s.clone();
                    self.advance();
                    parts.push(StringPart::Lit(s));
                }
                Token::InterpOpen => {
                    self.advance();
                    // A hole is delimited, so a record literal inside it is
                    // unambiguous even when the string itself sits in a
                    // no-struct-literal position (`if "${R { v: 1 }}" == s {`).
                    if let Some(expr) =
                        self.in_struct_ok_ctx(|p| p.parse_expr("string interpolation"))
                    {
                        parts.push(StringPart::Expr(expr));
                    }
                    self.expect(&Token::InterpClose, "string interpolation")?;
                }
                Token::Eof => {
                    let span = self.peek_span();
                    self.errors.push(ParseError::UnexpectedEof { span });
                    break;
                }
                _ => {
                    self.advance();
                }
            }
        }

        let end = self
            .tokens
            .get(self.pos.saturating_sub(1))
            .map(|s| s.span)
            .unwrap_or(start);
        Some(Spanned::new(Expr::StringInterp(parts), start.merge(end)))
    }

    fn parse_if_expr(&mut self, ctx: &str) -> Option<Spanned<Expr>> {
        let start = self.peek_span();
        self.expect(&Token::If, "if expression")?;
        let cond = self.in_no_struct_ctx(|p| p.parse_expr(ctx))?;
        let then = self.parse_block(ctx)?;
        let else_ = if self.eat(&Token::Else).is_some() {
            if self.check(&Token::If) {
                let e = self.parse_if_expr(ctx)?;
                Some(Box::new(e))
            } else {
                let b = self.parse_block(ctx)?;
                let span = b.span;
                Some(Box::new(Spanned::new(Expr::Block(b.value), span)))
            }
        } else {
            None
        };
        let end = else_.as_ref().map(|e| e.span).unwrap_or(then.span);
        Some(Spanned::new(
            Expr::If {
                cond: Box::new(cond),
                then: Box::new(then),
                else_,
            },
            start.merge(end),
        ))
    }

    fn parse_match_expr(&mut self, ctx: &str) -> Option<Spanned<Expr>> {
        let start = self.peek_span();
        self.expect(&Token::Match, "match expression")?;
        let scrutinee = self.in_no_struct_ctx(|p| p.parse_expr(ctx))?;
        self.expect(&Token::LBrace, "match body")?;
        let mut arms = Vec::new();
        while !self.check(&Token::RBrace) && !self.is_at_end() {
            if let Some(arm) = self.parse_match_arm(ctx) {
                arms.push(arm);
            } else {
                self.sync_to_stmt_boundary();
            }
        }
        self.expect(&Token::RBrace, "match body")?;
        let end = self
            .tokens
            .get(self.pos.saturating_sub(1))
            .map(|s| s.span)
            .unwrap_or(start);
        Some(Spanned::new(
            Expr::Match {
                scrutinee: Box::new(scrutinee),
                arms,
            },
            start.merge(end),
        ))
    }

    fn parse_match_arm(&mut self, ctx: &str) -> Option<MatchArm> {
        let pattern = self.parse_pattern(ctx)?;
        let guard = if self.eat(&Token::If).is_some() {
            Some(self.parse_expr(ctx)?)
        } else {
            None
        };
        self.expect(&Token::FatArrow, "match arm")?;
        let body = self.parse_expr(ctx)?;
        self.eat(&Token::Comma);
        Some(MatchArm {
            pattern,
            guard,
            body,
        })
    }

    fn parse_closure(&mut self, ctx: &str) -> Option<Spanned<Expr>> {
        let start = self.peek_span();
        self.expect(&Token::Pipe, "closure")?;
        let mut params = Vec::new();
        while !self.check(&Token::Pipe) && !self.is_at_end() {
            let is_mut = self.eat(&Token::Mut).is_some();
            let name = self.parse_ident("closure parameter")?;
            let ty = if self.eat(&Token::Colon).is_some() {
                self.parse_type(ctx)?
            } else {
                // Inferred type
                let span = name.span;
                Spanned::new(nova_ast::ty::Type::Infer, span)
            };
            params.push(Param { is_mut, name, ty });
            if self.eat(&Token::Comma).is_none() {
                break;
            }
        }
        self.expect(&Token::Pipe, "closure parameters end")?;
        let ret = if self.eat(&Token::Arrow).is_some() {
            Some(self.parse_type(ctx)?)
        } else {
            None
        };
        let body = self.parse_expr(ctx)?;
        let span = start.merge(body.span);
        Some(Spanned::new(
            Expr::Closure {
                params,
                ret,
                body: Box::new(body),
            },
            span,
        ))
    }

    fn parse_record_literal(&mut self, path: Path, start: Span) -> Option<Spanned<Expr>> {
        self.expect(&Token::LBrace, "record literal")?;
        let mut fields = Vec::new();
        let mut base = None;

        while !self.check(&Token::RBrace) && !self.is_at_end() {
            if self.eat(&Token::DotDot).is_some() {
                base = Some(Box::new(self.parse_expr("record spread")?));
                self.eat(&Token::Comma);
                break;
            }
            let name = match self.parse_ident("field name") {
                Some(n) => n,
                None => break,
            };
            let value = if self.eat(&Token::Colon).is_some() {
                Some(self.parse_expr("field value")?)
            } else {
                None
            };
            fields.push(FieldInit { name, value });
            if self.eat(&Token::Comma).is_none() {
                break;
            }
        }

        self.expect(&Token::RBrace, "record literal")?;
        let end = self
            .tokens
            .get(self.pos.saturating_sub(1))
            .map(|s| s.span)
            .unwrap_or(start);
        Some(Spanned::new(
            Expr::Record { path, fields, base },
            start.merge(end),
        ))
    }
}

// ---------------------------------------------------------------------------
// Patterns
// ---------------------------------------------------------------------------

impl<'a> Parser<'a> {
    fn parse_pattern(&mut self, ctx: &str) -> Option<Spanned<Pattern>> {
        let start = self.peek_span();
        let pat = self.parse_pattern_atom(ctx)?;

        // Or patterns
        if self.check(&Token::Pipe) {
            let mut alternatives = vec![pat];
            while self.eat(&Token::Pipe).is_some() {
                if let Some(p) = self.parse_pattern_atom(ctx) {
                    alternatives.push(p);
                } else {
                    break;
                }
            }
            let end = alternatives.last().map(|p| p.span).unwrap_or(start);
            return Some(Spanned::new(Pattern::Or(alternatives), start.merge(end)));
        }

        Some(pat)
    }

    fn parse_pattern_atom(&mut self, ctx: &str) -> Option<Spanned<Pattern>> {
        let start = self.peek_span();
        match self.peek().clone() {
            Token::Ident(ref s) if s == "_" => {
                self.advance();
                Some(Spanned::new(Pattern::Wildcard, start))
            }
            Token::Int(n) => {
                self.advance();
                // Range pattern?
                if self.eat(&Token::DotDot).is_some() {
                    let inclusive = self.eat(&Token::Eq).is_some();
                    let hi_start = self.peek_span();
                    if let Token::Int(hi) = self.peek().clone() {
                        self.advance();
                        let end = self
                            .tokens
                            .get(self.pos.saturating_sub(1))
                            .map(|s| s.span)
                            .unwrap_or(hi_start);
                        return Some(Spanned::new(
                            Pattern::Range {
                                lo: Box::new(Spanned::new(Pattern::Lit(Literal::Int(n)), start)),
                                hi: Box::new(Spanned::new(Pattern::Lit(Literal::Int(hi)), end)),
                                inclusive,
                            },
                            start.merge(end),
                        ));
                    }
                } else if self.eat(&Token::DotDotEq).is_some() {
                    let hi_start = self.peek_span();
                    if let Token::Int(hi) = self.peek().clone() {
                        self.advance();
                        let end = self
                            .tokens
                            .get(self.pos.saturating_sub(1))
                            .map(|s| s.span)
                            .unwrap_or(hi_start);
                        return Some(Spanned::new(
                            Pattern::Range {
                                lo: Box::new(Spanned::new(Pattern::Lit(Literal::Int(n)), start)),
                                hi: Box::new(Spanned::new(Pattern::Lit(Literal::Int(hi)), end)),
                                inclusive: true,
                            },
                            start.merge(end),
                        ));
                    }
                }
                Some(Spanned::new(Pattern::Lit(Literal::Int(n)), start))
            }
            Token::Float(f) => {
                self.advance();
                Some(Spanned::new(Pattern::Lit(Literal::Float(f)), start))
            }
            Token::Char(c) => {
                self.advance();
                // Range: 'a'..='z'
                if self.eat(&Token::DotDotEq).is_some() {
                    let hi_start = self.peek_span();
                    if let Token::Char(hi) = self.peek().clone() {
                        self.advance();
                        let end = self
                            .tokens
                            .get(self.pos.saturating_sub(1))
                            .map(|s| s.span)
                            .unwrap_or(hi_start);
                        return Some(Spanned::new(
                            Pattern::Range {
                                lo: Box::new(Spanned::new(Pattern::Lit(Literal::Char(c)), start)),
                                hi: Box::new(Spanned::new(Pattern::Lit(Literal::Char(hi)), end)),
                                inclusive: true,
                            },
                            start.merge(end),
                        ));
                    }
                }
                Some(Spanned::new(Pattern::Lit(Literal::Char(c)), start))
            }
            Token::True => {
                self.advance();
                Some(Spanned::new(Pattern::Lit(Literal::Bool(true)), start))
            }
            Token::False => {
                self.advance();
                Some(Spanned::new(Pattern::Lit(Literal::Bool(false)), start))
            }
            Token::StrStart => {
                // A string literal in a pattern
                self.advance();
                let mut s = String::new();
                loop {
                    match self.peek().clone() {
                        Token::StrPart(p) => {
                            s.push_str(&p);
                            self.advance();
                        }
                        Token::StrEnd => {
                            self.advance();
                            break;
                        }
                        _ => break,
                    }
                }
                let end = self
                    .tokens
                    .get(self.pos.saturating_sub(1))
                    .map(|sp| sp.span)
                    .unwrap_or(start);
                Some(Spanned::new(
                    Pattern::Lit(Literal::Str(s)),
                    start.merge(end),
                ))
            }
            Token::LParen => {
                self.advance();
                if self.eat(&Token::RParen).is_some() {
                    return Some(Spanned::new(Pattern::Tuple(vec![]), start));
                }
                let first = self.parse_pattern(ctx)?;
                if self.eat(&Token::RParen).is_some() {
                    return Some(Spanned::new(first.value, start));
                }
                let mut elems = vec![first];
                while self.eat(&Token::Comma).is_some() && !self.check(&Token::RParen) {
                    if let Some(p) = self.parse_pattern(ctx) {
                        elems.push(p);
                    }
                }
                self.expect(&Token::RParen, "tuple pattern")?;
                let end = self
                    .tokens
                    .get(self.pos.saturating_sub(1))
                    .map(|s| s.span)
                    .unwrap_or(start);
                Some(Spanned::new(Pattern::Tuple(elems), start.merge(end)))
            }
            Token::LBracket => {
                self.advance();
                let mut elems = Vec::new();
                while !self.check(&Token::RBracket) && !self.is_at_end() {
                    if let Some(p) = self.parse_pattern(ctx) {
                        elems.push(p);
                    }
                    if self.eat(&Token::Comma).is_none() {
                        break;
                    }
                }
                self.expect(&Token::RBracket, "array pattern")?;
                let end = self
                    .tokens
                    .get(self.pos.saturating_sub(1))
                    .map(|s| s.span)
                    .unwrap_or(start);
                Some(Spanned::new(Pattern::Array(elems), start.merge(end)))
            }
            Token::Ident(_) | Token::SelfLower => {
                let name = self.parse_ident(ctx)?;

                // Path with `::` — could be an enum variant
                if self.check(&Token::ColonColon) {
                    let mut path = Path::single(name);
                    while self.eat(&Token::ColonColon).is_some() {
                        if let Some(seg) = self.parse_ident(ctx) {
                            path.segments.push(seg);
                        } else {
                            break;
                        }
                    }
                    // Tuple variant: `Variant(pat, ...)`
                    if self.check(&Token::LParen) {
                        self.advance();
                        let mut fields = Vec::new();
                        while !self.check(&Token::RParen) && !self.is_at_end() {
                            if let Some(p) = self.parse_pattern(ctx) {
                                fields.push(p);
                            }
                            if self.eat(&Token::Comma).is_none() {
                                break;
                            }
                        }
                        self.expect(&Token::RParen, "variant pattern")?;
                        let end = self
                            .tokens
                            .get(self.pos.saturating_sub(1))
                            .map(|s| s.span)
                            .unwrap_or(start);
                        return Some(Spanned::new(
                            Pattern::TupleStruct { path, fields },
                            start.merge(end),
                        ));
                    }
                    let end = self
                        .tokens
                        .get(self.pos.saturating_sub(1))
                        .map(|s| s.span)
                        .unwrap_or(start);
                    return Some(Spanned::new(Pattern::Path(path), start.merge(end)));
                }

                // Binding with `@`
                if self.eat(&Token::At).is_some() {
                    let inner = self.parse_pattern(ctx)?;
                    let span = start.merge(inner.span);
                    return Some(Spanned::new(
                        Pattern::Binding {
                            name,
                            inner: Box::new(inner),
                        },
                        span,
                    ));
                }

                // Tuple struct: `Some(pat)`
                if self.check(&Token::LParen) {
                    let path = Path::single(name.clone());
                    self.advance();
                    let mut fields = Vec::new();
                    while !self.check(&Token::RParen) && !self.is_at_end() {
                        if let Some(p) = self.parse_pattern(ctx) {
                            fields.push(p);
                        }
                        if self.eat(&Token::Comma).is_none() {
                            break;
                        }
                    }
                    self.expect(&Token::RParen, "tuple struct pattern")?;
                    let end = self
                        .tokens
                        .get(self.pos.saturating_sub(1))
                        .map(|s| s.span)
                        .unwrap_or(start);
                    return Some(Spanned::new(
                        Pattern::TupleStruct { path, fields },
                        start.merge(end),
                    ));
                }

                // Record pattern: `Point { x, y }`
                if self.check(&Token::LBrace) {
                    let path = Path::single(name.clone());
                    self.advance();
                    let mut field_pats = Vec::new();
                    let mut rest = false;
                    while !self.check(&Token::RBrace) && !self.is_at_end() {
                        if self.eat(&Token::DotDot).is_some() {
                            rest = true;
                            break;
                        }
                        let fname = match self.parse_ident(ctx) {
                            Some(n) => n,
                            None => break,
                        };
                        let pattern = if self.eat(&Token::Colon).is_some() {
                            Some(self.parse_pattern(ctx)?)
                        } else {
                            None
                        };
                        field_pats.push(FieldPat {
                            name: fname,
                            pattern,
                        });
                        if self.eat(&Token::Comma).is_none() {
                            break;
                        }
                    }
                    self.expect(&Token::RBrace, "record pattern")?;
                    let end = self
                        .tokens
                        .get(self.pos.saturating_sub(1))
                        .map(|s| s.span)
                        .unwrap_or(start);
                    return Some(Spanned::new(
                        Pattern::Record {
                            path,
                            fields: field_pats,
                            rest,
                        },
                        start.merge(end),
                    ));
                }

                Some(Spanned::new(
                    Pattern::Ident {
                        is_mut: false,
                        name,
                    },
                    start,
                ))
            }
            _ => {
                let span = self.peek_span();
                self.errors.push(ParseError::Expected {
                    expected: format!("pattern (in {})", ctx),
                    found: self.peek().description().to_owned(),
                    span,
                });
                None
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Paths and identifiers
// ---------------------------------------------------------------------------

impl<'a> Parser<'a> {
    fn parse_ident(&mut self, ctx: &str) -> Option<Spanned<String>> {
        let span = self.peek_span();
        match self.peek().clone() {
            Token::Ident(s) => {
                let s = s.clone();
                self.advance();
                Some(Spanned::new(s, span))
            }
            Token::SelfLower => {
                self.advance();
                Some(Spanned::new("self".into(), span))
            }
            Token::SelfUpper => {
                self.advance();
                Some(Spanned::new("Self".into(), span))
            }
            _ => {
                self.errors.push(ParseError::Expected {
                    expected: format!("identifier (in {})", ctx),
                    found: self.peek().description().to_owned(),
                    span,
                });
                None
            }
        }
    }

    fn try_parse_path(&mut self) -> Option<Path> {
        let span = self.peek_span();
        let first = match self.peek().clone() {
            Token::Ident(s) => {
                let s = s.clone();
                self.advance();
                Spanned::new(s, span)
            }
            Token::SelfLower => {
                self.advance();
                Spanned::new("self".into(), span)
            }
            Token::SelfUpper => {
                self.advance();
                Spanned::new("Self".into(), span)
            }
            _ => return None,
        };
        let mut path = Path::single(first);
        while self.eat(&Token::ColonColon).is_some() {
            let seg_span = self.peek_span();
            match self.peek().clone() {
                Token::Ident(s) => {
                    let s = s.clone();
                    self.advance();
                    path.segments.push(Spanned::new(s, seg_span));
                }
                _ => break,
            }
        }
        Some(path)
    }

    fn parse_path(&mut self, ctx: &str) -> Option<Path> {
        match self.try_parse_path() {
            Some(p) => Some(p),
            None => {
                let span = self.peek_span();
                self.errors.push(ParseError::Expected {
                    expected: format!("path (in {})", ctx),
                    found: self.peek().description().to_owned(),
                    span,
                });
                None
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Try to convert a `Type::Path` to a `Path` (for trait-impl disambiguation).
fn type_to_path(ty: &Spanned<Type>) -> Option<Path> {
    match &ty.value {
        Type::Path { path, args: _ } => Some(path.clone()),
        _ => None,
    }
}
