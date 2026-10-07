//! Types (spec
//! `docs/superpowers/specs/2026-10-07-phase-3-1-formatter-design.md` §6).

use nova_ast::ty::Type;
use nova_diagnostics::{Span, Spanned};
use nova_lexer::Token;

use super::{LastComma, List, Printer};
use crate::doc::{concat, group, text, Doc};

impl<'s, 't> Printer<'s, 't> {
    /// A type, after any comments before it, inside one pair of the author's
    /// parentheses if it had any (spec §5.3).
    pub(super) fn ty(&mut self, t: &Spanned<Type>) -> Doc {
        let lead = self.leading(t.span.start);
        if matches!(t.value, Type::Tuple(_)) {
            let own = self.src.unparen(t.span, true);
            return concat(vec![lead, self.ty_kind(t, own)]);
        }
        let inner = self.src.unparen(t.span, false);
        if inner == t.span {
            return concat(vec![lead, self.ty_kind(t, inner)]);
        }
        let inner_lead = self.leading(inner.start);
        let body = self.ty_kind(t, inner);
        concat(vec![lead, text("("), inner_lead, body, text(")")])
    }

    /// `t` without its parentheses. `span` is `t`'s span inside them.
    fn ty_kind(&mut self, t: &Spanned<Type>, span: Span) -> Doc {
        match &t.value {
            Type::Path { path, args } => {
                let head = self.path(path);
                let Some(last) = args.last() else {
                    return head;
                };
                // The `>` may be half of a `>>` token (`grammar.rs:1058-1078`),
                // so the list closes where its last argument ends.
                let close = self.ty_end(last);
                concat(vec![
                    head,
                    group(self.comma_list(
                        List::ANGLES,
                        args,
                        |p, x| (x.span.start, p.ty_end(x)),
                        |p, x| p.ty(x),
                        close,
                    )),
                ])
            }
            Type::Ref { is_mut, inner } => {
                // `&&` is one token, so a reference to a reference keeps its
                // space: `& &T` (spec §5.3).
                let glued = !*is_mut
                    && !self.src.is_parenthesized(inner.span)
                    && matches!(inner.value, Type::Ref { .. });
                let prefix = if *is_mut {
                    "&mut "
                } else if glued {
                    "& "
                } else {
                    "&"
                };
                concat(vec![text(prefix), self.ty(inner)])
            }
            Type::Ptr { is_mut, inner } => concat(vec![
                text(if *is_mut { "*mut " } else { "*" }),
                self.ty(inner),
            ]),
            Type::Array(inner) => concat(vec![text("["), self.ty(inner), text("]")]),
            Type::Tuple(items) => {
                let last = if items.len() == 1 {
                    LastComma::Always
                } else {
                    LastComma::Broken
                };
                group(self.comma_list(
                    List::PARENS.last(last),
                    items,
                    |p, x| (x.span.start, p.ty_end(x)),
                    |p, x| p.ty(x),
                    span.end - 1,
                ))
            }
            Type::Fn { params, ret } => {
                let close = self
                    .src
                    .parens_from(span.start)
                    .map_or(span.end, |(_, close)| close);
                let mut parts = vec![
                    text("fn"),
                    group(self.comma_list(
                        List::PARENS,
                        params,
                        |p, x| (x.span.start, p.ty_end(x)),
                        |p, x| p.ty(x),
                        close,
                    )),
                ];
                // `fn(T)` and `fn(T) -> ()` are the same type, so no unit
                // return is printed (spec §6).
                if !matches!(&ret.value, Type::Tuple(v) if v.is_empty()) {
                    parts.push(text(" -> "));
                    parts.push(self.ty(ret));
                }
                concat(parts)
            }
            Type::Optional(inner) => concat(vec![self.ty(inner), text("?")]),
            Type::Infer => text("_"),
        }
    }

    /// Where `t` ends. The parser gives a `fn(…)` type written without `->`
    /// a unit return spanning the token after it, and spans the type over
    /// that token too (spec §2), so such a type ends at its `)`. So does a
    /// reference or a pointer to one.
    pub(super) fn ty_end(&self, t: &Spanned<Type>) -> u32 {
        if self.src.is_parenthesized(t.span) {
            return t.span.end;
        }
        match &t.value {
            Type::Fn { ret, .. }
                if self.src.last_code_token(ret.span.start) != Some(&Token::Arrow) =>
            {
                self.src
                    .parens_from(t.span.start)
                    .map_or(t.span.end, |(_, close)| close + 1)
            }
            Type::Ref { inner, .. } | Type::Ptr { inner, .. } => self.ty_end(inner),
            _ => t.span.end,
        }
    }
}
