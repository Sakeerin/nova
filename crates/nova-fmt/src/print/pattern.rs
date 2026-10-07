//! Patterns (spec
//! `docs/superpowers/specs/2026-10-07-phase-3-1-formatter-design.md` §6).

use nova_ast::pattern::{FieldPat, Pattern};
use nova_ast::Path;
use nova_diagnostics::{Span, Spanned};
use nova_lexer::Token;

use super::{LastComma, List, Printer};
use crate::doc::{concat, group, nest, text, Doc};

impl<'s, 't> Printer<'s, 't> {
    /// A pattern, after any comments before it, inside one pair of the
    /// author's parentheses if it had any (spec §5.3).
    pub(super) fn pattern(&mut self, p: &Spanned<Pattern>) -> Doc {
        let lead = self.leading(p.span.start);
        if matches!(p.value, Pattern::Tuple(_)) {
            let own = self.src.unparen(p.span, true);
            return concat(vec![lead, self.pattern_kind(p, own)]);
        }
        let inner = self.src.unparen(p.span, false);
        if inner == p.span {
            return concat(vec![lead, self.pattern_kind(p, inner)]);
        }
        let inner_lead = self.leading(inner.start);
        let body = self.pattern_kind(p, inner);
        concat(vec![lead, text("("), inner_lead, body, text(")")])
    }

    /// `p` without its parentheses. `span` is `p`'s span inside them.
    fn pattern_kind(&mut self, p: &Spanned<Pattern>, span: Span) -> Doc {
        let close = span.end.saturating_sub(1);
        match &p.value {
            Pattern::Wildcard => text("_"),
            Pattern::Lit(_) => self.verbatim(span),
            Pattern::Ident { is_mut, name } => concat(vec![
                text(if *is_mut { "mut " } else { "" }),
                self.name(name),
            ]),
            Pattern::Binding { name, inner } => {
                concat(vec![self.name(name), text(" @ "), self.pattern(inner)])
            }
            Pattern::TupleStruct { path, fields } => concat(vec![
                self.path(path),
                group(self.comma_list(
                    List::PARENS,
                    fields,
                    |_, x| (x.span.start, x.span.end),
                    |pr, x| pr.pattern(x),
                    close,
                )),
            ]),
            Pattern::Record { path, fields, rest } => {
                self.record_pattern(path, fields, *rest, close)
            }
            Pattern::Tuple(items) => {
                let last = if items.len() == 1 {
                    LastComma::Always
                } else {
                    LastComma::Broken
                };
                group(self.comma_list(
                    List::PARENS.last(last),
                    items,
                    |_, x| (x.span.start, x.span.end),
                    |pr, x| pr.pattern(x),
                    close,
                ))
            }
            Pattern::Array(items) => group(self.comma_list(
                List::BRACKETS,
                items,
                |_, x| (x.span.start, x.span.end),
                |pr, x| pr.pattern(x),
                close,
            )),
            Pattern::Or(alternatives) => {
                let first = self.pattern(&alternatives[0]);
                let mut rest = Vec::new();
                for a in &alternatives[1..] {
                    rest.push(Doc::Line);
                    rest.push(text("| "));
                    rest.push(self.pattern(a));
                }
                group(concat(vec![first, nest(concat(rest))]))
            }
            Pattern::Range { lo, hi, inclusive } => concat(vec![
                self.pattern(lo),
                text(if *inclusive { "..=" } else { ".." }),
                self.pattern(hi),
            ]),
            Pattern::Path(path) => self.path(path),
        }
    }

    /// `P { x, y: q, .. }`: no comma after `..`, which the parser rejects
    /// there (spec §2, §6).
    fn record_pattern(&mut self, path: &Path, fields: &[FieldPat], rest: bool, close: u32) -> Doc {
        enum Part<'a> {
            Field(&'a FieldPat),
            /// The `..`, starting here.
            Rest(u32),
        }
        let head = concat(vec![self.path(path), text(" ")]);
        let mut parts: Vec<Part> = fields.iter().map(Part::Field).collect();
        if rest {
            let after = match fields.last() {
                Some(f) => f.pattern.as_ref().map_or(f.name.span.end, |p| p.span.end),
                None => path.segments.last().map_or(0, |s| s.span.end),
            };
            parts.push(Part::Rest(
                self.src.find_from(after, &Token::DotDot).unwrap_or(after),
            ));
        }
        let last = if rest {
            LastComma::Never
        } else {
            LastComma::Broken
        };
        let body = self.comma_list(
            List::BRACES.last(last),
            &parts,
            |_, part| match part {
                Part::Field(f) => (
                    f.name.span.start,
                    f.pattern.as_ref().map_or(f.name.span.end, |p| p.span.end),
                ),
                Part::Rest(at) => (*at, at + 2),
            },
            |pr, part| match part {
                Part::Field(f) => match &f.pattern {
                    None => pr.name(&f.name),
                    Some(p) => concat(vec![pr.name(&f.name), text(": "), pr.pattern(p)]),
                },
                Part::Rest(_) => text(".."),
            },
            close,
        );
        concat(vec![head, group(body)])
    }
}
