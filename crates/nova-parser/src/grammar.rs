//! The actual grammar rules, implemented as a hand-written recursive descent
//! parser with chumsky-style error recovery stubs.
//!
//! Operator precedence for expressions is handled by explicit layering
//! (assignment → or → and → … → postfix), matching the spec table exactly.

use nova_ast::{
    expr::{
        AssignOp, BinOp, Expr, FieldInit, Literal, MatchArm, StringPart, UnOp, ViewArg, ViewNode,
    },
    item::{
        AssocTypeBinding, Attribute, ConstDecl, ExternBlock, ExternItem, Function, FunctionSig,
        ImplBlock, Import, ImportKind, Module, Param, Record, RecordField, TraitDecl, TraitItem,
        TypeDecl, TypeDef, Variant, Visibility, WhereBound,
    },
    pattern::{FieldPat, Pattern},
    ty::{Type, TypeParam},
    Block, File, Item, NewlineSplit, Path, SplitKind, Stmt,
};
use nova_diagnostics::{FileId, Span, Spanned};
use nova_lexer::{Token, Tokens};

use crate::ParseError;

/// The error for a `///` that documents nothing (spec
/// `docs/superpowers/specs/2026-10-07-phase-3-1-formatter-design.md` §4).
const MISPLACED_DOC: &str =
    "a doc comment must come right before an item, a field or a variant; use `//` for a plain comment";

// ---------------------------------------------------------------------------
// Parser state
// ---------------------------------------------------------------------------

struct Parser<'a> {
    tokens: &'a Tokens,
    pos: usize,
    file: FileId,
    errors: Vec<ParseError>,
    /// When `true`, `Ident {` in expression position is NOT parsed as a record
    /// literal. Set by scrutinee positions (if/while/for/match conditions) to
    /// avoid ambiguity with the following `{ block }`.
    no_struct_literal: bool,
    /// Whether the parser is in statement context, where the newline rule
    /// holds (spec
    /// `docs/superpowers/specs/2026-10-26-phase-5-0-the-language-and-std-a-compiler-needs-design.md`
    /// §3.5): a line break ends the expression before a `(`, a `[` or a
    /// binary `-`. On for a block's statements and tail and a match arm's
    /// body; off inside parentheses, brackets, arguments, record literals,
    /// interpolation holes, views, items, and the heads of `if`, `while`,
    /// `for` and `match`. Saved and restored like `no_struct_literal`.
    stmt_ctx: bool,
    /// Count of closing `>` "borrowed" from a `>>` token while closing nested
    /// generic argument lists (`Option<Option<Int>>`). The lexer glues `>>`
    /// into one token, so closing an inner list splits it: one `>` closes the
    /// inner list now, the remainder is recorded here for the enclosing list.
    pending_gt: usize,
    /// The names of top-level items that failed to parse after their names
    /// were read: `fn f(x: Int {` drops `f` (spec
    /// `docs/superpowers/specs/2026-10-08-phase-3-2-lsp-core-design.md`
    /// §3.2). The language server stops reporting their uses as unresolved.
    dropped: Vec<Spanned<String>>,
    /// Each place the newline rule ended a statement the parser before it
    /// would have continued (spec §3.5), for the checker's E0103.
    splits: Vec<NewlineSplit>,
    /// The position after a `return` or `break` that a line break ended:
    /// no operator, call or index there takes the bare keyword as its
    /// operand. Cleared when the expression that holds it ends.
    halt: Option<usize>,
    /// The split that ended the last match arm's body: a P0001 in the head
    /// of the arm that begins there gets a note naming the rule.
    arm_end_split: Option<NewlineSplit>,
}

impl<'a> Parser<'a> {
    fn new(tokens: &'a Tokens, file: FileId) -> Self {
        Self {
            tokens,
            pos: 0,
            file,
            errors: Vec::new(),
            no_struct_literal: false,
            stmt_ctx: false,
            pending_gt: 0,
            dropped: Vec::new(),
            splits: Vec::new(),
            halt: None,
            arm_end_split: None,
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
    /// `InterpClose` token and no `{ block }` can follow it. The hole is
    /// delimited, so it is not statement context either.
    fn in_struct_ok_ctx<R>(&mut self, f: impl FnOnce(&mut Self) -> R) -> R {
        let prev = self.no_struct_literal;
        self.no_struct_literal = false;
        let result = self.in_stmt_ctx(false, f);
        self.no_struct_literal = prev;
        result
    }

    /// Run `f` for the head of an `if`, `while`, `for` or `match`, which
    /// ends at its `{`: no record literal, and not statement context.
    fn in_head_ctx<R>(&mut self, f: impl FnOnce(&mut Self) -> R) -> R {
        self.in_no_struct_ctx(|p| p.in_stmt_ctx(false, f))
    }

    /// Run `f` with `stmt_ctx = on`, then restore the previous value.
    fn in_stmt_ctx<R>(&mut self, on: bool, f: impl FnOnce(&mut Self) -> R) -> R {
        let prev = self.stmt_ctx;
        self.stmt_ctx = on;
        let result = f(self);
        self.stmt_ctx = prev;
        result
    }

    /// Whether a line break comes before the token at the cursor.
    fn line_break_here(&self) -> bool {
        self.tokens.line_break_before(self.pos)
    }

    /// Whether the newline rule ends the expression before the token at the
    /// cursor: a line break before it, in statement context. The callers
    /// ask it of a `(`, a `[` and a binary `-`.
    fn newline_ends_expr(&self) -> bool {
        self.stmt_ctx && self.line_break_here()
    }

    /// Whether a `return` or `break` just read takes no value: the next
    /// token ends a statement or a block, or a line break comes before it,
    /// in any context.
    fn bare_keyword_ends(&mut self) -> bool {
        if matches!(self.peek(), Token::Semicolon | Token::RBrace | Token::Eof) {
            return true;
        }
        if !self.line_break_here() {
            return false;
        }
        // A `let` or an item after the line break is a statement of its own.
        if matches!(self.peek(), Token::Let) || self.is_item_start_at(self.pos) {
            self.halt = Some(self.pos);
            return true;
        }
        // A token that cannot begin an expression, such as `)`, `,` or
        // `=>`, is read as before the rule: as the value's start, an error.
        if !can_start_expr(self.peek()) {
            return false;
        }
        // The line after would have been the value: the rule split it off.
        self.record_split(SplitKind::Value);
        self.halt = Some(self.pos);
        true
    }

    /// Whether the parser stands where a line break ended a bare `return` or
    /// `break`: the token there belongs to whatever follows the keyword's
    /// expression, not to the expression.
    fn halted(&self) -> bool {
        self.halt == Some(self.pos)
    }

    /// A P0001 in the head of an arm that the newline rule began, by ending
    /// the arm before at a line break: the first error the arm gave, from
    /// `first` on, wherever it lands, gets a note naming the rule, since the
    /// parser before the rule read that line into the body. An error in
    /// the body of an arm whose head parsed is not the rule's doing: the
    /// base would have met that `=>` after an expression, an error too.
    fn note_arm_split(&mut self, begun: Option<NewlineSplit>, first: usize) {
        let Some(split) = begun else {
            return;
        };
        if first >= self.errors.len() {
            return;
        }
        let note = match split.kind {
            SplitKind::Value => {
                "a `return` or `break` at the end of a line takes no value, so the \
                 next line begins the next match arm (the newline rule); to return \
                 that line's value, put it on the same line as the `return` or `break`"
            }
            _ => {
                "a line that starts with `(`, `[` or `-` begins the next match arm \
                 (the newline rule); to continue the arm's body, end the line before \
                 with the operator"
            }
        };
        let error = self.errors.remove(first);
        self.errors.insert(
            first,
            ParseError::Noted {
                error: Box::new(error),
                note: note.to_string(),
            },
        );
    }

    /// Record that the newline rule ends a statement before the token at
    /// the cursor, which the parser before the rule would have continued.
    /// One split a token: the first recorded there stands, so a closure's
    /// bare `return` (a `Value`) and the call or index the expression
    /// around it then declines (`Call` or `Index`) give one warning.
    fn record_split(&mut self, kind: SplitKind) {
        let at = self.peek_span();
        if self.splits.iter().any(|s| s.at == at) {
            return;
        }
        self.splits.push(NewlineSplit { at, kind });
    }

    // --- Token access ---

    fn peek(&self) -> &Token {
        if self.halted() {
            return &HALT;
        }
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
            if self.component_at(self.pos) {
                break;
            }
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

    // --- Doc comments (spec §4) ---

    /// Consume consecutive `///` lines and return them, in order.
    fn take_docs(&mut self) -> Vec<Spanned<String>> {
        let mut docs = Vec::new();
        while let Token::DocComment(text) = self.peek().clone() {
            let span = self.peek_span();
            self.advance();
            docs.push(Spanned::new(text, span));
        }
        docs
    }

    /// Report a `///` that documents nothing.
    fn misplaced_doc(&mut self, span: Span) {
        self.errors.push(ParseError::Custom {
            message: MISPLACED_DOC.into(),
            span,
        });
    }

    /// Report and skip the `///` lines here, which document nothing. Returns
    /// whether there were any.
    fn reject_docs(&mut self) -> bool {
        let docs = self.take_docs();
        for d in &docs {
            self.misplaced_doc(d.span);
        }
        !docs.is_empty()
    }

    /// The index of the first token at or after the cursor that is not a
    /// `///` line.
    fn docs_end(&self) -> usize {
        let mut i = self.pos;
        while matches!(
            self.tokens.get(i).map(|t| &t.value),
            Some(Token::DocComment(_))
        ) {
            i += 1;
        }
        i
    }

    /// Whether the token after any `///` lines is `tok`.
    fn next_after_docs_is(&self, tok: &Token) -> bool {
        self.tokens
            .get(self.docs_end())
            .is_some_and(|t| std::mem::discriminant(&t.value) == std::mem::discriminant(tok))
    }

    /// Whether the tokens at `i` are `component Name (`, where `component` is
    /// a keyword (RFC 0001); anywhere else it is an identifier.
    fn component_at(&self, i: usize) -> bool {
        let at = |k: usize| self.tokens.get(k).map(|t| &t.value);
        if !(matches!(at(i), Some(Token::Ident(w)) if w == "component")
            && matches!(at(i + 1), Some(Token::Ident(_)))
            && matches!(at(i + 2), Some(Token::LParen)))
        {
            return false;
        }
        // Its parameters' `)`, then its body's `{`, or a `->` that
        // `parse_component` refuses. Line breaks are skipped, so without
        // this a statement `component`, a variable, followed by a call
        // `f(x)` on the next line would read as an item.
        let mut depth = 0usize;
        let mut k = i + 2;
        loop {
            match at(k) {
                Some(Token::LParen) => depth += 1,
                Some(Token::RParen) => {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                None | Some(Token::Eof) => return false,
                _ => {}
            }
            k += 1;
        }
        matches!(at(k + 1), Some(Token::LBrace | Token::Arrow))
    }

    /// Whether the token at `i` starts an item that may appear inside a block.
    fn is_item_start_at(&self, i: usize) -> bool {
        let at = |k: usize| self.tokens.get(k).map(|t| &t.value);
        if self.component_at(i) || (matches!(at(i), Some(Token::Pub)) && self.component_at(i + 1)) {
            return true;
        }
        matches!(
            at(i),
            Some(
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
            )
        ) || (matches!(at(i), Some(Token::Pub))
            && matches!(
                at(i + 1),
                Some(
                    Token::Fn
                        | Token::Record
                        | Token::Trait
                        | Token::Type
                        | Token::Const
                        | Token::Impl
                )
            ))
    }

    /// A top-level item's docs and `@attributes`, which may be interleaved.
    fn parse_item_prefix(&mut self) -> (Vec<Spanned<String>>, Vec<Attribute>) {
        let mut docs = Vec::new();
        let mut attrs = Vec::new();
        loop {
            docs.extend(self.take_docs());
            if self.peek() != &Token::At {
                break;
            }
            attrs.extend(self.parse_attributes());
        }
        (docs, attrs)
    }
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

pub(crate) fn parse_file(tokens: &Tokens, file: FileId) -> (Option<File>, Vec<ParseError>) {
    let parsed = parse_recovering(tokens, file);
    (Some(parsed.file), parsed.errors)
}

pub(crate) fn parse_recovering(tokens: &Tokens, file: FileId) -> crate::Parsed {
    let mut p = Parser::new(tokens, file);
    let ast_file = p.parse_file();
    crate::Parsed {
        file: ast_file,
        errors: p.errors,
        dropped: p.dropped,
    }
}

// ---------------------------------------------------------------------------
// File
// ---------------------------------------------------------------------------

impl<'a> Parser<'a> {
    fn parse_file(&mut self) -> File {
        let mut items = Vec::new();
        while !self.is_at_end() {
            let start = self.pos;
            match self.try_parse_item() {
                Some(item) => items.push(item),
                None => {
                    self.note_dropped_item(start);
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
        let mut newline_splits = std::mem::take(&mut self.splits);
        newline_splits.sort_by_key(|s| s.at.start);
        File {
            items,
            newline_splits,
        }
    }

    /// The item that started at token `start` failed. If it got as far as
    /// its name, remember the name (spec
    /// `docs/superpowers/specs/2026-10-08-phase-3-2-lsp-core-design.md`
    /// §3.2). Only the tokens the failed attempt consumed are read.
    fn note_dropped_item(&mut self, start: usize) {
        let consumed = &self.tokens[start..self.pos.min(self.tokens.len())];
        let keyword = consumed.iter().enumerate().position(|(k, t)| {
            matches!(
                t.value,
                Token::Fn | Token::Record | Token::Type | Token::Trait | Token::Const
            ) || (matches!(&t.value, Token::Ident(w) if w == "component")
                && matches!(consumed.get(k + 1).map(|n| &n.value), Some(Token::Ident(_)))
                && matches!(consumed.get(k + 2).map(|n| &n.value), Some(Token::LParen)))
        });
        if let Some(k) = keyword {
            if let Some(Token::Ident(name)) = consumed.get(k + 1).map(|t| &t.value) {
                let span = consumed[k + 1].span;
                self.dropped.push(Spanned::new(name.clone(), span));
            }
        }
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
        let (docs, attrs) = self.parse_item_prefix();
        let vis = self.parse_visibility();

        let item = match self.peek() {
            Token::Fn | Token::Async => {
                let mut func = self.parse_function(vis)?;
                func.attrs = attrs;
                func.docs = docs;
                Item::Function(func)
            }
            Token::Record => {
                self.advance();
                let mut record = self.parse_record(vis)?;
                record.attrs = attrs;
                record.docs = docs;
                Item::Record(record)
            }
            Token::Type => {
                self.advance();
                let mut td = self.parse_type_decl(vis)?;
                td.attrs = attrs;
                td.docs = docs;
                Item::Type(td)
            }
            Token::Trait => {
                self.advance();
                let mut tr = self.parse_trait_decl(vis)?;
                tr.attrs = attrs;
                tr.docs = docs;
                Item::Trait(tr)
            }
            Token::Impl => {
                self.advance();
                let mut impl_ = self.parse_impl_block()?;
                impl_.attrs = attrs;
                impl_.docs = docs;
                Item::Impl(impl_)
            }
            Token::Const => {
                self.advance();
                let mut c = self.parse_const_decl(vis)?;
                c.attrs = attrs;
                c.docs = docs;
                Item::Const(c)
            }
            Token::Import => {
                self.advance();
                let mut imp = self.parse_import()?;
                imp.attrs = attrs;
                imp.docs = docs;
                Item::Import(imp)
            }
            Token::Module => {
                self.advance();
                let mut m = self.parse_module()?;
                m.attrs = attrs;
                m.docs = docs;
                Item::Module(m)
            }
            Token::Extern => {
                self.advance();
                let mut e = self.parse_extern_block()?;
                e.attrs = attrs;
                e.docs = docs;
                Item::Extern(e)
            }
            Token::Ident(_) if self.component_at(self.pos) => {
                let mut func = self.parse_component(vis)?;
                func.attrs = attrs;
                func.docs = docs;
                Item::Function(func)
            }
            _ => {
                let span = self.peek_span();
                if let Some(first) = docs.first() {
                    // Docs with no item after them (spec §4).
                    self.misplaced_doc(first.span);
                    return None;
                }
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
            docs: Vec::new(),
            attrs: Vec::new(),
            vis,
            is_async,
            is_component: false,
            name,
            generics,
            params,
            return_ty,
            where_clause,
            body,
        })
    }

    /// `component Name(params) { body }` (RFC 0001): a function returning
    /// `std/ui`'s `View`, with no type parameters and no written return type.
    fn parse_component(&mut self, vis: Visibility) -> Option<Function> {
        self.advance(); // `component`
        let name = self.parse_ident("component name")?;
        self.expect(&Token::LParen, "component parameters")?;
        let params = self.parse_params();
        self.expect(&Token::RParen, "component parameters")?;
        if let Some(arrow) = self.eat(&Token::Arrow) {
            self.errors.push(ParseError::Custom {
                message: "a component returns a `View`, which it does not write".into(),
                span: arrow.span,
            });
            // The type is read and dropped, so that the body still parses.
            self.parse_type("return type")?;
        }
        let body = self.parse_block("component body")?;
        Some(Function {
            docs: Vec::new(),
            attrs: Vec::new(),
            vis,
            is_async: false,
            is_component: true,
            name,
            generics: Vec::new(),
            params,
            return_ty: None,
            where_clause: Vec::new(),
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
            docs: Vec::new(),
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
            if self.reject_docs() {
                continue;
            }
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
            // A trailing comma (spec §4): the clause ends at the body or the
            // end of a signature.
            if matches!(
                self.peek(),
                Token::LBrace | Token::Semicolon | Token::RBrace
            ) {
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
            let docs = self.take_docs();
            if self.check(&Token::RBrace) || self.is_at_end() {
                if let Some(first) = docs.first() {
                    self.misplaced_doc(first.span);
                }
                continue;
            }
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
                docs,
                vis: fvis,
                name: fname,
                ty: fty,
            });
        }
        self.expect(&Token::RBrace, "record body")?;
        Some(Record {
            docs: Vec::new(),
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

        // Sum type: starts with `|`, after the first variant's docs if any.
        // A variant's docs come before its `|`; docs with no `|` after them
        // belong to whatever follows the type, so they are left for it.
        let def = if self.next_after_docs_is(&Token::Pipe) {
            let mut variants = Vec::new();
            while self.next_after_docs_is(&Token::Pipe) {
                let docs = self.take_docs();
                self.advance(); // the `|`
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
                    docs,
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
            docs: Vec::new(),
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
            let docs = self.take_docs();
            if self.check(&Token::RBrace) || self.is_at_end() {
                if let Some(first) = docs.first() {
                    self.misplaced_doc(first.span);
                }
                continue;
            }
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
                    items.push(TraitItem::AssocType { docs, name, bounds });
                } else {
                    self.sync_to_stmt_boundary();
                }
                continue;
            }
            // Look ahead: if there's a body `{`, it's a provided method
            let saved_pos = self.pos;
            let saved_errors_len = self.errors.len();
            let is_async = self.check(&Token::Async);
            if let Some(mut sig) = self.parse_function_sig() {
                if self.check(&Token::LBrace) {
                    // Provided method — we need to re-parse as Function.
                    // Roll back and parse as full function.
                    self.pos = saved_pos;
                    self.errors.truncate(saved_errors_len);
                    let func_vis = Visibility::Private;
                    if let Some(mut func) = self.parse_function(func_vis) {
                        func.docs = docs;
                        items.push(TraitItem::Provided(func));
                    }
                } else {
                    // Required method — expect semicolon.
                    self.eat(&Token::Semicolon);
                    sig.docs = docs;
                    items.push(TraitItem::Required(sig));
                }
            } else {
                self.sync_to_stmt_boundary();
            }
            let _ = is_async;
        }
        self.expect(&Token::RBrace, "trait body")?;
        Some(TraitDecl {
            docs: Vec::new(),
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
            let docs = self.take_docs();
            if self.check(&Token::RBrace) || self.is_at_end() {
                if let Some(first) = docs.first() {
                    self.misplaced_doc(first.span);
                }
                continue;
            }
            // Captured before `parse_visibility` consumes it, so the `pub`
            // rejection in the `type` arm can point at the `pub` itself.
            let vis_span = self.peek_span();
            let vis = self.parse_visibility();
            match self.peek() {
                Token::Fn | Token::Async => {
                    if let Some(mut f) = self.parse_function(vis) {
                        f.docs = docs;
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
                                    assoc_types.push(AssocTypeBinding { docs, name, ty });
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
                    if let Some(mut c) = self.parse_const_decl(vis) {
                        c.docs = docs;
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
            docs: Vec::new(),
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
            docs: Vec::new(),
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
            docs: Vec::new(),
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
            docs: Vec::new(),
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
            let docs = self.take_docs();
            if self.check(&Token::RBrace) || self.is_at_end() {
                if let Some(first) = docs.first() {
                    self.misplaced_doc(first.span);
                }
                continue;
            }
            if let Some(mut sig) = self.parse_function_sig() {
                sig.docs = docs;
                self.eat(&Token::Semicolon);
                items.push(ExternItem::Fn(sig));
            } else {
                self.sync_to_stmt_boundary();
            }
        }
        self.expect(&Token::RBrace, "extern block")?;
        Some(ExternBlock {
            docs: Vec::new(),
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
        // A block anywhere turns statement context on (spec §3.5).
        self.in_stmt_ctx(true, |p| p.parse_block_inner(ctx))
    }

    fn parse_block_inner(&mut self, ctx: &str) -> Option<Spanned<Block>> {
        let start = self.peek_span();
        self.expect(&Token::LBrace, ctx)?;
        let mut stmts = Vec::new();
        let mut trailing: Option<Box<Spanned<Expr>>> = None;

        while !self.check(&Token::RBrace) && !self.is_at_end() {
            // A `///` documents a nested item. Anywhere else in a block it is
            // misplaced (spec §4).
            if matches!(self.peek(), Token::DocComment(_))
                && !self.is_item_start_at(self.docs_end())
            {
                self.reject_docs();
                continue;
            }
            // Check if this could be the trailing expression.
            // Parse a statement; if it's an expression without semicolon at the
            // block-final position, treat as trailing.
            if let Some(stmt) = self.try_parse_stmt() {
                match stmt.value {
                    // `_`, not `ref e`: this arm pushes `stmt` whole and never
                    // reads the expression, and the guard does not either. A
                    // by-value binding would move out of `stmt.value` and
                    // break the `push`, which is what the old `ref` was
                    // guarding against -- but `_` binds nothing at all, so it
                    // is safe here and does not warn. Only rustc 1.78 reported
                    // the unused binding; stable does not, which is why this
                    // survived until the MSRV job began running its own
                    // compiler.
                    Stmt::Expr(_) if !self.check(&Token::RBrace) => {
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

        // Nested items, documented or not (spec §4)
        if self.is_item_start_at(self.docs_end()) {
            let item = self.in_stmt_ctx(false, |p| p.try_parse_item())?;
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
        let expr = self.parse_assign(ctx);
        self.end_halt();
        expr
    }

    /// The expression that holds a bare `return` or `break` ends here: what
    /// follows reads the token again.
    fn end_halt(&mut self) {
        if self.halted() {
            self.halt = None;
        }
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
                // A `-` after a line break in statement context begins the
                // next statement (spec §3.5).
                Token::Minus if self.newline_ends_expr() => {
                    self.record_split(SplitKind::Minus);
                    break;
                }
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
        let expr = self.parse_atom(ctx)?;
        self.parse_postfix_from(start, expr, ctx)
    }

    /// The calls, indexes, fields, `.await`s and `?`s after `expr`, which
    /// began at `start`.
    fn parse_postfix_from(
        &mut self,
        start: Span,
        mut expr: Spanned<Expr>,
        ctx: &str,
    ) -> Option<Spanned<Expr>> {
        loop {
            let cur_span = expr.span;
            match self.peek().clone() {
                // A `(` or `[` after a line break in statement context
                // begins the next statement (spec §3.5).
                Token::LParen | Token::LBracket if self.newline_ends_expr() => {
                    let kind = if self.check(&Token::LParen) {
                        SplitKind::Call
                    } else {
                        SplitKind::Index
                    };
                    self.record_split(kind);
                    break;
                }
                Token::LParen => {
                    // Function call
                    self.advance();
                    let args = self.in_stmt_ctx(false, |p| {
                        let mut args = Vec::new();
                        while !p.check(&Token::RParen) && !p.is_at_end() {
                            if let Some(a) = p.parse_expr(ctx) {
                                args.push(a);
                            }
                            if p.eat(&Token::Comma).is_none() {
                                break;
                            }
                        }
                        args
                    });
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
                    let index = self.in_stmt_ctx(false, |p| p.parse_expr(ctx))?;
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
                    } else if let Token::Int(n) = self.peek().clone() {
                        // `t.N`, a tuple's element (spec
                        // docs/superpowers/specs/2026-10-26-phase-5-0-the-language-and-std-a-compiler-needs-design.md
                        // §4.2): plain decimal digits only, so `t.01`,
                        // `t.1_0` and `t.0x1` are refused, as Rust does.
                        let span = self.peek_span();
                        if !self.tokens.plain_decimal(self.pos) {
                            self.errors.push(ParseError::Custom {
                                message: format!(
                                    "a tuple index is plain decimal digits, with no leading \
                                     zero, `_` or prefix: write `.{n}`"
                                ),
                                span,
                            });
                        }
                        self.advance();
                        expr = Spanned::new(
                            Expr::Field {
                                target: Box::new(expr),
                                field: Spanned::new(n.to_string(), span),
                            },
                            start.merge(span),
                        );
                    } else if !matches!(
                        self.peek(),
                        Token::Ident(_) | Token::SelfLower | Token::SelfUpper
                    ) {
                        // An unfinished `foo.` keeps its receiver, so an
                        // editor can complete after the `.` (spec
                        // docs/superpowers/specs/2026-10-08-phase-3-2-lsp-core-design.md
                        // §3.4). The error is the one `parse_ident` reports.
                        self.errors.push(ParseError::Expected {
                            expected: "identifier (in field access)".to_owned(),
                            found: self.peek().description().to_owned(),
                            span: self.peek_span(),
                        });
                        let dot_end = self.tokens[self.pos - 1].span.end;
                        let gap = Span::new(dot_end, dot_end, self.file);
                        expr = Spanned::new(
                            Expr::Field {
                                target: Box::new(expr),
                                field: Spanned::new(String::new(), gap),
                            },
                            start.merge(gap),
                        );
                        break;
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
            Token::LParen => self.in_stmt_ctx(false, |p| p.parse_paren(start, ctx)),

            // Array literal — either `[a, b, c]` or the repeat form `[init; n]`
            Token::LBracket => self.in_stmt_ctx(false, |p| p.parse_array_literal(start, ctx)),

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
                let cond = self.in_head_ctx(|p| p.parse_expr(ctx))?;
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
                let iter = self.in_head_ctx(|p| p.parse_expr(ctx))?;
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
                // Optional expression; none at the end of a line (spec §3.5).
                let val = if !self.bare_keyword_ends() {
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
                let val = if !self.bare_keyword_ends() {
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

            // Closure: |params| body, or `||` with none (RFC 0001).
            Token::Pipe | Token::PipePipe => self.parse_closure(ctx),

            // `view { … }`, where `view` is a keyword (RFC 0001).
            Token::Ident(w)
                if w == "view"
                    && !self.no_struct_literal
                    && matches!(
                        self.tokens.get(self.pos + 1).map(|t| &t.value),
                        Some(Token::LBrace)
                    ) =>
            {
                self.parse_view()
            }

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

    /// `()`, `(e)` or a tuple, at `(`, which began at `start`.
    fn parse_paren(&mut self, start: Span, ctx: &str) -> Option<Spanned<Expr>> {
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

    /// An array literal, `[a, b, c]` or `[init; n]`, at `[`, which began at
    /// `start`.
    fn parse_array_literal(&mut self, start: Span, ctx: &str) -> Option<Spanned<Expr>> {
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
        let cond = self.in_head_ctx(|p| p.parse_expr(ctx))?;
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
        let scrutinee = self.in_head_ctx(|p| p.parse_expr(ctx))?;
        self.expect(&Token::LBrace, "match body")?;
        let mut arms = Vec::new();
        while !self.check(&Token::RBrace) && !self.is_at_end() {
            if self.reject_docs() {
                continue;
            }
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
        // The split that ended the arm before, if this arm begins at it.
        let start = self.peek_span().start;
        let begun = self.arm_end_split.take().filter(|s| s.at.start == start);
        let errors = self.errors.len();
        let Some((pattern, guard)) = self.parse_match_arm_head(ctx) else {
            self.note_arm_split(begun, errors);
            return None;
        };
        // An arm's body is statement context, so a line that starts with
        // `(` begins the next arm (spec §3.5).
        let splits = self.splits.len();
        let body = self.in_stmt_ctx(true, |p| p.parse_expr(ctx))?;
        // A split at the body's end begins the next arm: no warning for it,
        // however many times it was met (a `return`'s value and the
        // `return` around it each meet it; `record_split` keeps one). Where
        // the parser before the rule read that line into the body, the next
        // arm cannot parse (it needs a pattern and `=>`, and no expression
        // is followed by `=>`), and its P0001 gets a note naming the rule.
        let end = self.peek_span().start;
        self.arm_end_split = None;
        while self.splits.len() > splits && self.splits.last().map(|s| s.at.start) == Some(end) {
            self.arm_end_split = self.splits.pop();
        }
        self.eat(&Token::Comma);
        Some(MatchArm {
            pattern,
            guard,
            body,
        })
    }

    /// An arm's pattern, its guard if any, and its `=>`.
    fn parse_match_arm_head(
        &mut self,
        ctx: &str,
    ) -> Option<(Spanned<Pattern>, Option<Spanned<Expr>>)> {
        let pattern = self.parse_pattern(ctx)?;
        // A guard ends at its `=>`, as a head ends at its `{`.
        let guard = if self.eat(&Token::If).is_some() {
            Some(self.in_stmt_ctx(false, |p| p.parse_expr(ctx))?)
        } else {
            None
        };
        self.expect(&Token::FatArrow, "match arm")?;
        Some((pattern, guard))
    }

    fn parse_closure(&mut self, ctx: &str) -> Option<Spanned<Expr>> {
        let start = self.peek_span();
        let mut params = Vec::new();
        if self.eat(&Token::PipePipe).is_none() {
            self.expect(&Token::Pipe, "closure")?;
            self.parse_closure_params(&mut params, ctx)?;
        }
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

    /// A closure's parameters, after its `|`, up to and with its closing `|`.
    fn parse_closure_params(&mut self, params: &mut Vec<Param>, ctx: &str) -> Option<()> {
        while !self.check(&Token::Pipe) && !self.is_at_end() {
            if self.reject_docs() {
                continue;
            }
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
        Some(())
    }

    /// The span of the token just consumed, or `fallback` at the start.
    fn prev_span_or(&self, fallback: Span) -> Span {
        self.tokens
            .get(self.pos.saturating_sub(1))
            .map(|s| s.span)
            .unwrap_or(fallback)
    }

    /// `view { child* }` (RFC 0001), at `view`.
    fn parse_view(&mut self) -> Option<Spanned<Expr>> {
        let start = self.peek_span();
        // A view's children keep their `;` rule: the newline rule is off in
        // them, as in an element's arguments.
        self.advance();
        let children = self.in_stmt_ctx(false, |p| p.parse_view_children())?;
        let span = start.merge(self.prev_span_or(start));
        Some(Spanned::new(Expr::View(children), span))
    }

    /// `{ child* }`, the children of a `view` or an element. A `{` inside is
    /// an element's or a block's again, whatever context the view is in.
    fn parse_view_children(&mut self) -> Option<Vec<Spanned<ViewNode>>> {
        self.expect(&Token::LBrace, "view children")?;
        let saved = std::mem::replace(&mut self.no_struct_literal, false);
        let mut children = Vec::new();
        while !self.check(&Token::RBrace) && !self.is_at_end() {
            // A `;` may end a child, as one ends a statement, so that the
            // next can start with `(`, `[` or `{`.
            if self.reject_docs() || self.eat(&Token::Semicolon).is_some() {
                continue;
            }
            match self.parse_view_child() {
                Some(child) => children.push(child),
                None => {
                    self.no_struct_literal = saved;
                    return None;
                }
            }
        }
        self.no_struct_literal = saved;
        self.expect(&Token::RBrace, "view children")?;
        Some(children)
    }

    /// One child of a view: an element, or a postfix expression.
    fn parse_view_child(&mut self) -> Option<Spanned<ViewNode>> {
        let start = self.peek_span();
        if let Token::Ident(name) = self.peek().clone() {
            let next = self.tokens.get(self.pos + 1).map(|t| t.value.clone());
            if name != "view" && matches!(next, Some(Token::LBrace)) {
                let name = Spanned::new(name, start);
                self.advance();
                let children = self.parse_view_children()?;
                let span = start.merge(self.prev_span_or(start));
                return Some(Spanned::new(
                    ViewNode::Element {
                        name,
                        args: Vec::new(),
                        children: Some(children),
                    },
                    span,
                ));
            }
            if matches!(next, Some(Token::LParen)) {
                let name = Spanned::new(name, start);
                self.advance(); // the name
                self.advance(); // `(`
                let args = self.parse_view_args()?;
                let named = args.iter().any(|a| matches!(a, ViewArg::Named { .. }));
                if self.check(&Token::LBrace) {
                    let children = self.parse_view_children()?;
                    let span = start.merge(self.prev_span_or(start));
                    return Some(Spanned::new(
                        ViewNode::Element {
                            name,
                            args,
                            children: Some(children),
                        },
                        span,
                    ));
                }
                let span = start.merge(self.prev_span_or(start));
                if named {
                    return Some(Spanned::new(
                        ViewNode::Element {
                            name,
                            args,
                            children: None,
                        },
                        span,
                    ));
                }
                // Positional arguments alone and no block: an ordinary call.
                let args = args
                    .into_iter()
                    .map(|a| match a {
                        ViewArg::Positional(e) => e,
                        ViewArg::Named { value, .. } => value,
                    })
                    .collect();
                let callee = Spanned::new(Expr::Path(Path::single(name.clone())), name.span);
                let call = Spanned::new(
                    Expr::Call {
                        callee: Box::new(callee),
                        args,
                    },
                    span,
                );
                let expr = self.parse_postfix_from(start, call, "view child")?;
                self.end_halt();
                let span = expr.span;
                return Some(Spanned::new(ViewNode::Expr(expr), span));
            }
        }
        // A child is an expression of its own, as `parse_expr` reads one: a
        // bare `return` or `break` in it ends with it.
        let expr = self.parse_postfix("view child");
        self.end_halt();
        let expr = expr?;
        let span = expr.span;
        Some(Spanned::new(ViewNode::Expr(expr), span))
    }

    /// An element's arguments, after its `(`, up to and with its `)`:
    /// `name: value` or an expression, separated by commas.
    fn parse_view_args(&mut self) -> Option<Vec<ViewArg>> {
        let mut args = Vec::new();
        while !self.check(&Token::RParen) && !self.is_at_end() {
            let colon_next = matches!(
                self.tokens.get(self.pos + 1).map(|t| &t.value),
                Some(Token::Colon)
            );
            if let (Some(word), true) = (self.arg_name(), colon_next) {
                let name = Spanned::new(word, self.peek_span());
                self.advance(); // the name
                self.advance(); // `:`
                let value = self.parse_expr("element argument")?;
                args.push(ViewArg::Named { name, value });
            } else {
                args.push(ViewArg::Positional(self.parse_expr("element argument")?));
            }
            if self.eat(&Token::Comma).is_none() {
                break;
            }
        }
        self.expect(&Token::RParen, "element arguments")?;
        Some(args)
    }

    /// The word at the cursor, if it can name an element's argument: an
    /// identifier, or a keyword such as `type` or `for`.
    fn arg_name(&self) -> Option<String> {
        match self.peek() {
            Token::Ident(w) => Some(w.clone()),
            other => {
                let word = other.description().trim_matches('`');
                nova_lexer::KEYWORDS
                    .contains(&word)
                    .then(|| word.to_string())
            }
        }
    }

    fn parse_record_literal(&mut self, path: Path, start: Span) -> Option<Spanned<Expr>> {
        self.in_stmt_ctx(false, |p| p.parse_record_literal_inner(path, start))
    }

    fn parse_record_literal_inner(&mut self, path: Path, start: Span) -> Option<Spanned<Expr>> {
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
                // Spans over the parentheses, as for expressions and types,
                // so the formatter can keep them (spec §4).
                if let Some(close) = self.eat(&Token::RParen) {
                    return Some(Spanned::new(
                        Pattern::Tuple(vec![]),
                        start.merge(close.span),
                    ));
                }
                let first = self.parse_pattern(ctx)?;
                if let Some(close) = self.eat(&Token::RParen) {
                    return Some(Spanned::new(first.value, start.merge(close.span)));
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

/// What [`Parser::peek`] reads where a line break ended a bare `return` or
/// `break` (`Parser::halted`): nothing, so no operator, call or index takes
/// the keyword as its operand.
static HALT: Token = Token::Eof;

/// Whether `tok` can begin an expression: a literal, a name, a bracket, a
/// prefix operator, a closure, or a keyword that begins one.
fn can_start_expr(tok: &Token) -> bool {
    matches!(
        tok,
        Token::Int(_)
            | Token::Float(_)
            | Token::Char(_)
            | Token::True
            | Token::False
            | Token::StrStart
            | Token::RawStr(_)
            | Token::LParen
            | Token::LBracket
            | Token::LBrace
            | Token::If
            | Token::Match
            | Token::While
            | Token::For
            | Token::Return
            | Token::Break
            | Token::Continue
            | Token::Pipe
            | Token::PipePipe
            | Token::Ident(_)
            | Token::SelfLower
            | Token::SelfUpper
            | Token::Minus
            | Token::Bang
            | Token::Tilde
            | Token::Star
            | Token::Amp
    )
}
