//! Blocks, statements and expressions (spec
//! `docs/superpowers/specs/2026-10-07-phase-3-1-formatter-design.md` §5.6,
//! §6).

use nova_ast::expr::{AssignOp, BinOp, Expr, FieldInit, MatchArm, UnOp};
use nova_ast::item::Param;
use nova_ast::ty::Type;
use nova_ast::{Block, Path, Stmt};
use nova_diagnostics::{Span, Spanned};
use nova_lexer::Token;

use super::{Blank, LastComma, List, Printer, Vertical, CONTINUES};
use crate::doc::{concat, group, nest, text, Doc};

/// A statement in a block, or the block's trailing expression.
#[derive(Clone, Copy)]
enum Elem<'a> {
    Stmt(&'a Spanned<Stmt>),
    Tail(&'a Spanned<Expr>),
}

impl Elem<'_> {
    fn span(&self) -> Span {
        match self {
            Elem::Stmt(s) => s.span,
            Elem::Tail(e) => e.span,
        }
    }
}

/// One step of a method chain (spec §6, "Method chains").
enum Seg<'a> {
    /// `.name(args)`, with the call's span, which ends at its `)`.
    Call(&'a Spanned<String>, &'a [Spanned<Expr>], Span),
    Field(&'a Spanned<String>),
    Try,
    Await,
    Index(&'a Spanned<Expr>),
}

impl<'s, 't> Printer<'s, 't> {
    /// An expression, after any comments before it, inside one pair of the
    /// author's parentheses if it had any (spec §5.3). A tuple's own
    /// parentheses are its syntax, so it loses any extra pair.
    pub(super) fn expr(&mut self, e: &Spanned<Expr>) -> Doc {
        let lead = self.leading(e.span.start);
        if matches!(e.value, Expr::Tuple(_)) {
            let own = self.src.unparen(e.span, true);
            return concat(vec![lead, self.expr_kind(e, own)]);
        }
        let inner = self.src.unparen(e.span, false);
        if inner == e.span {
            return concat(vec![lead, self.expr_kind(e, inner)]);
        }
        let inner_lead = self.leading(inner.start);
        let body = self.expr_kind(e, inner);
        concat(vec![lead, text("("), inner_lead, body, text(")")])
    }

    /// `e` without its parentheses. `span` is `e`'s span inside them.
    fn expr_kind(&mut self, e: &Spanned<Expr>, span: Span) -> Doc {
        match &e.value {
            Expr::Lit(_) | Expr::StringInterp(_) => self.verbatim(span),
            Expr::Path(path) => self.path(path),
            Expr::Tuple(items) => {
                let last = if items.len() == 1 {
                    LastComma::Always
                } else {
                    LastComma::Broken
                };
                group(self.comma_list(
                    List::PARENS.last(last),
                    items,
                    |_, x| (x.span.start, x.span.end),
                    |p, x| p.expr(x),
                    span.end - 1,
                ))
            }
            Expr::Array(items) => group(self.comma_list(
                List::BRACKETS,
                items,
                |_, x| (x.span.start, x.span.end),
                |p, x| p.expr(x),
                span.end - 1,
            )),
            Expr::ArrayRepeat { init, len } => concat(vec![
                text("["),
                self.expr(init),
                text("; "),
                self.expr(len),
                text("]"),
            ]),
            Expr::Block(b) => group(self.block_parts(b, span)),
            Expr::If { .. } => self.if_chain(e),
            Expr::Match { scrutinee, arms } => self.match_expr(scrutinee, arms, span),
            Expr::While { cond, body } => group(concat(vec![
                text("while "),
                self.expr(cond),
                text(" "),
                self.block_parts(&body.value, body.span),
            ])),
            Expr::For {
                pattern,
                iter,
                body,
            } => group(concat(vec![
                text("for "),
                self.pattern(pattern),
                text(" in "),
                self.expr(iter),
                text(" "),
                self.block_parts(&body.value, body.span),
            ])),
            Expr::Range { lo, hi, inclusive } => concat(vec![
                self.expr(lo),
                text(if *inclusive { "..=" } else { ".." }),
                self.expr(hi),
            ]),
            Expr::Return(value) => self.keyword_value("return", value.as_deref()),
            Expr::Break(value) => self.keyword_value("break", value.as_deref()),
            Expr::Continue => text("continue"),
            Expr::Closure { params, ret, body } => self.closure(params, ret.as_ref(), body),
            Expr::Record { path, fields, base } => {
                self.record_literal(path, fields, base.as_deref(), span)
            }
            Expr::Binary { .. } => self.binary(e),
            // `&` takes a postfix expression (`grammar.rs:1556-1560`), so a
            // reference to a reference is written `&(&x)`, and its
            // parentheses are kept: no `&&` can form here.
            Expr::Unary { op, expr } => concat(vec![text(un_op(*op)), self.expr(expr)]),
            Expr::Call { .. }
            | Expr::Field { .. }
            | Expr::Try(_)
            | Expr::Await(_)
            | Expr::Index { .. } => match self.method_chain(e, span) {
                Some(chain) => chain,
                None => self.postfix(e, span),
            },
            Expr::Cast { expr, ty } => concat(vec![self.expr(expr), text(" as "), self.ty(ty)]),
            Expr::Assign { op, lhs, rhs } => concat(vec![
                self.expr(lhs),
                text(format!(" {} ", assign_op(*op))),
                self.expr(rhs),
            ]),
        }
    }

    /// A postfix expression that is not a method chain. `span` is `e`'s, in
    /// its parentheses.
    fn postfix(&mut self, e: &Spanned<Expr>, span: Span) -> Doc {
        match &e.value {
            Expr::Call { callee, args } => concat(vec![self.expr(callee), self.args(args, span)]),
            Expr::Field { target, field } => {
                let base = self.expr(target);
                if !self.has_comment_before(field.span.start) {
                    return concat(vec![base, self.dot(field)]);
                }
                // A comment before the `.` puts the access on a line of its
                // own, indented once, after the comment (spec §5.4).
                let trail = self.trailing(target.span.end);
                concat(vec![
                    base,
                    trail,
                    nest(concat(vec![Doc::HardLine, self.dot(field)])),
                ])
            }
            Expr::Try(x) => concat(vec![self.expr(x), text("?")]),
            Expr::Await(x) => concat(vec![self.expr(x), text(".await")]),
            Expr::Index { target, index } => concat(vec![
                self.expr(target),
                text("["),
                self.expr(index),
                text("]"),
            ]),
            _ => unreachable!("postfix is only called on a postfix expression"),
        }
    }

    /// A call's `(args)`. `call` is the call's span, which ends at its `)`.
    fn args(&mut self, args: &[Spanned<Expr>], call: Span) -> Doc {
        group(self.comma_list(
            List::PARENS,
            args,
            |_, x| (x.span.start, x.span.end),
            |p, x| p.expr(x),
            call.end - 1,
        ))
    }

    /// A chain of two or more method calls: flat when it fits, otherwise
    /// each `.name(…)` on its own line, indented once (spec §6, "Method
    /// chains"; the 3.1 plan's decision 7). `None` for anything else.
    fn method_chain(&mut self, e: &Spanned<Expr>, span: Span) -> Option<Doc> {
        let mut segs = Vec::new();
        let mut cur = e;
        loop {
            let top = std::ptr::eq(cur, e);
            if !top && self.src.is_parenthesized(cur.span) {
                break;
            }
            match &cur.value {
                Expr::Call { callee, args } => match &callee.value {
                    Expr::Field { target, field } if !self.src.is_parenthesized(callee.span) => {
                        segs.push(Seg::Call(field, args, if top { span } else { cur.span }));
                        cur = target.as_ref();
                    }
                    _ => break,
                },
                Expr::Field { target, field } => {
                    segs.push(Seg::Field(field));
                    cur = target.as_ref();
                }
                Expr::Try(x) => {
                    segs.push(Seg::Try);
                    cur = x.as_ref();
                }
                Expr::Await(x) => {
                    segs.push(Seg::Await);
                    cur = x.as_ref();
                }
                Expr::Index { target, index } => {
                    segs.push(Seg::Index(index));
                    cur = target.as_ref();
                }
                _ => break,
            }
        }
        if segs.iter().filter(|s| matches!(s, Seg::Call(..))).count() < 2 {
            return None;
        }
        segs.reverse();
        let mut head = vec![self.expr(cur)];
        let mut tail = Vec::new();
        let mut called = false;
        // Where the text printed so far ends in the source, when known.
        let mut end = Some(cur.span.end);
        for s in &segs {
            if let Seg::Call(name, ..) | Seg::Field(name) = s {
                // A comment that ends the line before a step stays on that
                // line. One on a line of its own leads the step, before its
                // `.` (spec §5.4).
                let commented = self.has_comment_before(name.span.start);
                if let Some(at) = end {
                    let t = self.trailing(at);
                    if called {
                        tail.push(t);
                    } else {
                        head.push(t);
                    }
                }
                if matches!(s, Seg::Call(..)) {
                    tail.push(Doc::SoftLine);
                    called = true;
                } else if commented {
                    if called {
                        tail.push(Doc::HardLine);
                    } else {
                        head.push(nest(Doc::HardLine));
                    }
                }
            }
            let d = self.seg(s);
            end = match s {
                Seg::Call(_, _, span) => Some(span.end),
                Seg::Field(name) => Some(name.span.end),
                Seg::Try | Seg::Await | Seg::Index(_) => None,
            };
            if called {
                tail.push(d);
            } else {
                head.push(d);
            }
        }
        Some(group(concat(vec![concat(head), nest(concat(tail))])))
    }

    fn seg(&mut self, s: &Seg) -> Doc {
        match s {
            Seg::Call(name, args, span) => concat(vec![self.dot(name), self.args(args, *span)]),
            Seg::Field(name) => self.dot(name),
            Seg::Try => text("?"),
            Seg::Await => text(".await"),
            Seg::Index(i) => concat(vec![text("["), self.expr(i), text("]")]),
        }
    }

    /// `.name`, after any comments before it. They lead the step, so they go
    /// before its `.` (spec §5.4).
    fn dot(&mut self, name: &Spanned<String>) -> Doc {
        concat(vec![
            self.leading(name.span.start),
            text("."),
            text(name.value.clone()),
        ])
    }

    /// A block's `{ … }`, after any comments before its `{`, without a group
    /// of its own, so that the construct holding it can share one (spec §6,
    /// "Blocks"). One statement and no comment may stay on one line;
    /// anything else is one statement per line.
    pub(super) fn block_parts(&mut self, b: &Block, span: Span) -> Doc {
        let lead = self.leading(span.start);
        let close = span.end - 1;
        let mut elems: Vec<Elem> = b.stmts.iter().map(Elem::Stmt).collect();
        if let Some(t) = &b.trailing {
            elems.push(Elem::Tail(t));
        }
        let commented = self.has_comment_before(close);
        if elems.is_empty() && !commented {
            return concat(vec![lead, text("{}")]);
        }
        if elems.len() == 1 && !commented {
            let d = self.elem(elems[0]);
            return concat(vec![
                lead,
                text("{"),
                nest(concat(vec![Doc::Line, d])),
                Doc::Line,
                text("}"),
            ]);
        }
        let after_open = self.trailing(span.start + 1);
        let mut v = Vertical::new(span.start + 1, Blank::Keep);
        for (i, el) in elems.iter().enumerate() {
            let sep = elems.get(i + 1).and_then(|next| self.separator(*el, *next));
            let s = el.span();
            // A trailing expression's span stops before a `;` after it.
            let end = self
                .src
                .token_end(s.end, &Token::Semicolon)
                .unwrap_or(s.end);
            self.vertical_item(&mut v, s.start, end, sep, |p| p.elem(*el));
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

    /// `;` between two statements where the second would otherwise continue
    /// the first (spec §5.6). A `;` after an item would not parse, and no
    /// item can be continued.
    fn separator(&self, cur: Elem, next: Elem) -> Option<&'static str> {
        if let Elem::Stmt(s) = cur {
            if matches!(s.value, Stmt::Item(_)) {
                return None;
            }
        }
        // A `return` or `break` with no value would take the next statement
        // as its value.
        if matches!(
            self.src.last_code_token(cur.span().end),
            Some(Token::Return | Token::Break)
        ) {
            return Some(";");
        }
        CONTINUES
            .contains(self.src.token_from(next.span().start))
            .then_some(";")
    }

    fn elem(&mut self, el: Elem) -> Doc {
        match el {
            Elem::Tail(e) => self.expr(e),
            Elem::Stmt(s) => match &s.value {
                Stmt::Expr(e) => self.expr(e),
                Stmt::Item(item) => self.item(item, s.span, false),
                Stmt::Let {
                    is_mut,
                    pattern,
                    ty,
                    init,
                } => {
                    let mut parts = vec![
                        text(if *is_mut { "let mut " } else { "let " }),
                        self.pattern(pattern),
                    ];
                    if let Some(t) = ty {
                        parts.push(text(": "));
                        parts.push(self.ty(t));
                    }
                    if let Some(x) = init {
                        parts.push(text(" = "));
                        parts.push(self.expr(x));
                    }
                    concat(parts)
                }
            },
        }
    }

    /// `if … { … } else if … { … } else { … }`, one group for the whole
    /// chain, so its blocks are all on one line or all broken (spec §6,
    /// "Blocks").
    fn if_chain(&mut self, e: &Spanned<Expr>) -> Doc {
        let mut parts = Vec::new();
        let mut cur = e;
        loop {
            let Expr::If { cond, then, else_ } = &cur.value else {
                unreachable!("if_chain is only called on an if")
            };
            parts.push(text("if "));
            parts.push(self.expr(cond));
            parts.push(text(" "));
            parts.push(self.block_parts(&then.value, then.span));
            let Some(next) = else_.as_deref() else {
                break;
            };
            parts.push(self.trailing(then.span.end));
            parts.push(text(" else "));
            match &next.value {
                Expr::If { .. } => {
                    parts.push(self.leading(next.span.start));
                    cur = next;
                }
                Expr::Block(b) => {
                    parts.push(self.block_parts(b, next.span));
                    break;
                }
                _ => {
                    parts.push(self.expr(next));
                    break;
                }
            }
        }
        group(concat(parts))
    }

    /// `match x { … }`: one arm per line, without commas but for §5.6's.
    fn match_expr(&mut self, scrutinee: &Spanned<Expr>, arms: &[MatchArm], span: Span) -> Doc {
        let head = concat(vec![text("match "), self.expr(scrutinee), text(" ")]);
        let open = self
            .src
            .find_from(scrutinee.span.end, &Token::LBrace)
            .unwrap_or(scrutinee.span.end);
        let lead = self.leading(open);
        let close = span.end - 1;
        if arms.is_empty() && !self.has_comment_before(close) {
            return concat(vec![head, lead, text("{}")]);
        }
        let after_open = self.trailing(open + 1);
        let mut v = Vertical::new(open + 1, Blank::Keep);
        for (i, arm) in arms.iter().enumerate() {
            let start = arm.pattern.span.start;
            let end = self
                .src
                .token_end(arm.body.span.end, &Token::Comma)
                .unwrap_or(arm.body.span.end);
            let sep = arms
                .get(i + 1)
                .filter(|n| CONTINUES.contains(self.src.token_from(n.pattern.span.start)))
                .map(|_| ",");
            self.vertical_item(&mut v, start, end, sep, |p| p.arm(arm));
        }
        let inner = self.vertical_finish(v, close);
        concat(vec![
            head,
            lead,
            text("{"),
            after_open,
            nest(concat(vec![Doc::HardLine, inner])),
            Doc::HardLine,
            text("}"),
        ])
    }

    /// `pattern if guard => body`, in a group that a block body shares.
    fn arm(&mut self, arm: &MatchArm) -> Doc {
        let mut parts = vec![self.pattern(&arm.pattern)];
        if let Some(g) = &arm.guard {
            parts.push(text(" if "));
            parts.push(self.expr(g));
        }
        parts.push(text(" => "));
        parts.push(self.body(&arm.body));
        group(concat(parts))
    }

    /// A match arm's or a closure's body. A block body shares the group of
    /// the construct holding it (spec §6, "Blocks").
    fn body(&mut self, body: &Spanned<Expr>) -> Doc {
        match &body.value {
            Expr::Block(b) if !self.src.is_parenthesized(body.span) => {
                self.block_parts(b, body.span)
            }
            _ => self.expr(body),
        }
    }

    /// `|a, b| body`. Its parameters never break (the 3.1 plan's decision 8).
    fn closure(
        &mut self,
        params: &[Param],
        ret: Option<&Spanned<Type>>,
        body: &Spanned<Expr>,
    ) -> Doc {
        let mut parts = Vec::new();
        if params.is_empty() {
            // `||` is one token, which no closure starts with (spec §5.3).
            parts.push(text("| |"));
        } else {
            parts.push(text("|"));
            for (i, p) in params.iter().enumerate() {
                if i > 0 {
                    parts.push(text(", "));
                }
                parts.push(self.param(p));
            }
            parts.push(text("|"));
        }
        if let Some(r) = ret {
            parts.push(text(" -> "));
            parts.push(self.ty(r));
        }
        parts.push(text(" "));
        parts.push(self.body(body));
        group(concat(parts))
    }

    /// `Path { field: value, short, ..base }`: no comma after `..base`
    /// (spec §6).
    fn record_literal(
        &mut self,
        path: &Path,
        fields: &[FieldInit],
        base: Option<&Spanned<Expr>>,
        span: Span,
    ) -> Doc {
        enum Part<'a> {
            Field(&'a FieldInit),
            Base(&'a Spanned<Expr>),
        }
        let head = concat(vec![self.path(path), text(" ")]);
        let mut parts: Vec<Part> = fields.iter().map(Part::Field).collect();
        if let Some(b) = base {
            parts.push(Part::Base(b));
        }
        let last = if base.is_some() {
            LastComma::Never
        } else {
            LastComma::Broken
        };
        let body = self.comma_list(
            List::BRACES.last(last),
            &parts,
            |p, part| match part {
                Part::Field(f) => (
                    f.name.span.start,
                    f.value.as_ref().map_or(f.name.span.end, |v| v.span.end),
                ),
                Part::Base(b) => (p.src.start_with(b.span.start, &[Token::DotDot]), b.span.end),
            },
            |p, part| match part {
                Part::Field(f) => match &f.value {
                    None => p.name(&f.name),
                    Some(v) => concat(vec![p.name(&f.name), text(": "), p.expr(v)]),
                },
                Part::Base(b) => concat(vec![text(".."), p.expr(b)]),
            },
            span.end - 1,
        );
        concat(vec![head, group(body)])
    }

    /// A chain of one precedence level: flat when it fits, otherwise broken
    /// before each operator, indented once (spec §6, "Binary expressions").
    fn binary(&mut self, e: &Spanned<Expr>) -> Doc {
        let Expr::Binary { op, .. } = &e.value else {
            unreachable!("binary is only called on a binary expression")
        };
        let mut operands = Vec::new();
        self.flatten(e, precedence(*op), true, &mut operands);
        let first = self.expr(operands[0].1);
        let mut rest = Vec::new();
        for &(op, x) in &operands[1..] {
            let op = op.expect("every operand after the first has an operator");
            rest.push(Doc::Line);
            rest.push(text(format!("{} ", bin_op(op))));
            rest.push(self.expr(x));
        }
        group(concat(vec![first, nest(concat(rest))]))
    }

    /// `e`'s operands at precedence `level`, left to right, each with the
    /// operator before it. A parenthesised operand is one operand.
    fn flatten<'a>(
        &self,
        e: &'a Spanned<Expr>,
        level: u8,
        top: bool,
        out: &mut Vec<(Option<BinOp>, &'a Spanned<Expr>)>,
    ) {
        match &e.value {
            Expr::Binary { op, lhs, rhs }
                if precedence(*op) == level && (top || !self.src.is_parenthesized(e.span)) =>
            {
                self.flatten(lhs, level, false, out);
                out.push((Some(*op), rhs.as_ref()));
            }
            _ => out.push((None, e)),
        }
    }

    fn keyword_value(&mut self, keyword: &str, value: Option<&Spanned<Expr>>) -> Doc {
        match value {
            None => text(keyword),
            Some(v) => concat(vec![text(format!("{keyword} ")), self.expr(v)]),
        }
    }
}

/// Binding strength, weakest first, as the parser's layers have it
/// (`grammar.rs:1324-1527`).
fn precedence(op: BinOp) -> u8 {
    match op {
        BinOp::Or => 1,
        BinOp::And => 2,
        BinOp::Eq | BinOp::Ne | BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => 3,
        BinOp::BitOr => 4,
        BinOp::BitXor => 5,
        BinOp::BitAnd => 6,
        BinOp::Shl | BinOp::Shr => 7,
        BinOp::Add | BinOp::Sub => 8,
        BinOp::Mul | BinOp::Div | BinOp::Rem => 9,
    }
}

fn bin_op(op: BinOp) -> &'static str {
    match op {
        BinOp::Or => "||",
        BinOp::And => "&&",
        BinOp::Eq => "==",
        BinOp::Ne => "!=",
        BinOp::Lt => "<",
        BinOp::Le => "<=",
        BinOp::Gt => ">",
        BinOp::Ge => ">=",
        BinOp::BitOr => "|",
        BinOp::BitXor => "^",
        BinOp::BitAnd => "&",
        BinOp::Shl => "<<",
        BinOp::Shr => ">>",
        BinOp::Add => "+",
        BinOp::Sub => "-",
        BinOp::Mul => "*",
        BinOp::Div => "/",
        BinOp::Rem => "%",
    }
}

fn un_op(op: UnOp) -> &'static str {
    match op {
        UnOp::Neg => "-",
        UnOp::Not => "!",
        UnOp::BitNot => "~",
        UnOp::Ref => "&",
        UnOp::RefMut => "&mut ",
        UnOp::Deref => "*",
    }
}

fn assign_op(op: AssignOp) -> &'static str {
    match op {
        AssignOp::Assign => "=",
        AssignOp::AddAssign => "+=",
        AssignOp::SubAssign => "-=",
        AssignOp::MulAssign => "*=",
        AssignOp::DivAssign => "/=",
        AssignOp::RemAssign => "%=",
        AssignOp::BitOrAssign => "|=",
        AssignOp::BitAndAssign => "&=",
        AssignOp::BitXorAssign => "^=",
        AssignOp::ShlAssign => "<<=",
        AssignOp::ShrAssign => ">>=",
    }
}
