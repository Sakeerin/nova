//! 5.0b's patterns in the checker (spec
//! `docs/superpowers/specs/2026-10-26-phase-5-0-the-language-and-std-a-compiler-needs-design.md`
//! §4.3 to §4.5): tuple, or- and range patterns, the binding rules of an
//! or-pattern (E0100), empty ranges (E0101), irrefutable destructuring in
//! `let` and `for` (E0022, decided by usefulness), and the unreachable
//! alternatives of an or-pattern (E0021).

use std::ops::Range;

use nova_ast as ast;
use nova_diagnostics::{suggest, Diagnostic, Span, Spanned};
use nova_hir as hir;
use nova_hir::{LocalId, Ty};
use nova_resolver::{DefKind, Res, ScopeEntry, Target};

use super::fixes::Binding;
use super::{to_useful_pat, Checker, FnCtx};
use crate::usefulness;

/// The checker's state while it checks one pattern.
#[derive(Default)]
pub(super) struct PatState {
    /// The or-patterns being checked, innermost last.
    frames: Vec<OrFrame>,
    /// Every binding the pattern makes is mutable: set for `let mut P`,
    /// since `mut` does not parse inside a pattern.
    pub(super) mutable: bool,
    /// The pattern is a destructuring `let`'s, whose bindings `let mut P`
    /// can make mutable (E0060's note says so).
    pub(super) in_let: bool,
}

/// One or-pattern being checked.
#[derive(Default)]
struct OrFrame {
    /// The first alternative's bindings, once it is checked: each later
    /// alternative binds these same locals.
    first: Option<Vec<PatBind>>,
    /// The bindings of the alternative being checked, nested or-patterns'
    /// included.
    current: Vec<PatBind>,
}

#[derive(Clone)]
struct PatBind {
    name: String,
    local: LocalId,
    ty: Ty,
    span: Span,
}

/// How a `for` loop binds each element: a plain local, as before 5.0b, or
/// an irrefutable pattern.
pub(super) enum LoopBind {
    Local(LocalId),
    Pattern(hir::Pattern),
}

impl LoopBind {
    /// The pattern that binds an element, for a `match` arm.
    pub(super) fn into_pattern(self) -> hir::Pattern {
        match self {
            LoopBind::Local(l) => hir::Pattern::Bind(l),
            LoopBind::Pattern(p) => p,
        }
    }

    /// The statement that binds `init`: a `let` for a plain local, and for
    /// a pattern a one-arm `match`, whose arm binds the pattern's locals for
    /// the rest of the block (MIR's locals are the function's).
    pub(super) fn bind(self, init: hir::Expr, span: Span) -> hir::Expr {
        match self {
            LoopBind::Local(local) => hir::Expr {
                kind: hir::ExprKind::Let {
                    local,
                    init: Box::new(init),
                },
                ty: Ty::Unit,
                span,
            },
            LoopBind::Pattern(pattern) => let_pattern(pattern, init, span),
        }
    }
}

/// `let P = init` for a destructuring `P`: a `match` with one arm, whose body
/// is `()`.
pub(super) fn let_pattern(pattern: hir::Pattern, init: hir::Expr, span: Span) -> hir::Expr {
    hir::Expr {
        kind: hir::ExprKind::Match {
            scrutinee: Box::new(init),
            arms: vec![hir::Arm {
                pattern,
                guard: None,
                body: hir::Expr {
                    kind: hir::ExprKind::Unit,
                    ty: Ty::Unit,
                    span,
                },
                span,
            }],
        },
        ty: Ty::Unit,
        span,
    }
}

impl<'a> Checker<'a> {
    /// Bind `name` in a pattern. Inside the second or a later alternative of
    /// an or-pattern, the name is the first alternative's local, at the
    /// same type (E0100 otherwise).
    pub(super) fn bind_pattern_name(
        &mut self,
        fcx: &mut FnCtx,
        name: &Spanned<String>,
        ty: &Ty,
        is_mut: bool,
    ) -> hir::Pattern {
        let first = self
            .pats
            .frames
            .iter()
            .rev()
            .find_map(|f| f.first.as_ref())
            .map(|first| first.iter().find(|b| b.name == name.value).cloned());
        let bind = match first {
            Some(Some(b)) => {
                if !fcx.icx.unify(ty, &b.ty) {
                    let (here, there) = (self.show(ty, fcx), self.show(&b.ty, fcx));
                    self.diagnostics.push(
                        Diagnostic::error(
                            "E0100",
                            format!("the alternatives bind `{}` at different types", name.value),
                        )
                        .with_primary_label(
                            name.span,
                            format!(
                                "`{}` is `{here}` here, `{there}` in the first alternative",
                                name.value
                            ),
                        ),
                    );
                }
                self.note_use(name.span, Target::Local(b.span));
                b
            }
            found => {
                if found.is_some() {
                    self.diagnostics.push(
                        Diagnostic::error("E0100", "the alternatives bind different names")
                            .with_primary_label(
                                name.span,
                                format!("`{}` is not bound in the first alternative", name.value),
                            ),
                    );
                }
                // A binding whose name starts with an upper-case letter is
                // most often a variant misspelt or not in scope: as a binding
                // it matches every value (the user's ruling of 2026-10-28).
                if name.value.chars().next().is_some_and(char::is_uppercase) {
                    self.warn_upper_case_binding(fcx, name, ty);
                }
                let is_mut = is_mut || self.pats.mutable;
                let binding = if self.pats.in_let {
                    Binding::LetPattern
                } else {
                    Binding::Pattern
                };
                let local = self.bind_local(
                    fcx,
                    name.value.clone(),
                    ty.clone(),
                    is_mut,
                    name.span,
                    binding,
                );
                PatBind {
                    name: name.value.clone(),
                    local,
                    ty: ty.clone(),
                    span: name.span,
                }
            }
        };
        let local = bind.local;
        for frame in &mut self.pats.frames {
            if !frame.current.iter().any(|b| b.name == bind.name) {
                frame.current.push(bind.clone());
            }
        }
        hir::Pattern::Bind(local)
    }

    /// E0104 at `name`, a pattern's binding that starts with an upper-case
    /// letter (the user's ruling of 2026-10-28): a constant's name gets its
    /// own note, and a name near a variant's "did you mean".
    fn warn_upper_case_binding(&mut self, fcx: &FnCtx, name: &Spanned<String>, ty: &Ty) {
        let is_const = matches!(
            self.defs.resolve_value(self.cur_module, &name.value),
            Some(Res::Def(id)) if matches!(self.defs.def(id).kind, DefKind::Const { .. })
        );
        let note = if is_const {
            format!(
                "a pattern cannot match a constant by name; use a guard: `n if n == {}`",
                name.value
            )
        } else {
            "a name that starts with an upper-case letter reads as a variant; if one was \
             meant, check its spelling or import it; to bind a value, use a lower-case name"
                .to_string()
        };
        let mut d = Diagnostic::warning(
            "E0104",
            format!(
                "`{}` binds a value: no variant of that name is in scope",
                name.value
            ),
        )
        .with_primary_label(name.span, "this binding matches any value")
        .with_note(note);
        // The variants the name could mean: the matched sum's, and those in
        // scope.
        let mut variants: Vec<String> = Vec::new();
        if let Some(sum) = super::fixes::scrutinee_sum(fcx, ty) {
            if let DefKind::Sum { variants: vs, .. } = &self.defs.def(sum).kind {
                variants.extend(vs.iter().map(|v| v.name.clone()));
            }
        }
        for (n, entry) in self.defs.names_in_scope(self.cur_module) {
            if matches!(entry, ScopeEntry::Value(Res::Variant(..))) {
                variants.push(n);
            }
        }
        let near = if is_const {
            None
        } else {
            suggest::closest(&name.value, variants.iter().map(String::as_str))
        };
        if let Some(to) = near {
            d = d.with_note(format!("did you mean `{to}`?"));
        }
        self.diagnostics.push(d);
        if near.is_some() {
            self.push_fix(suggest::change_to(
                name,
                variants.iter().map(String::as_str),
            ));
        }
    }

    /// `(p1, ..., pn)`, and `()` against the unit type.
    pub(super) fn tuple_pattern(
        &mut self,
        fcx: &mut FnCtx,
        elems: &[Spanned<ast::Pattern>],
        scrut_ty: &Ty,
        span: Span,
    ) -> hir::Pattern {
        if elems.is_empty() {
            self.unify_pattern_ty(fcx, &Ty::Unit, scrut_ty, span);
            return hir::Pattern::Wildcard;
        }
        let tys = match fcx.icx.apply(scrut_ty) {
            Ty::Tuple(tys) if tys.len() == elems.len() => Some(tys),
            Ty::Tuple(tys) => {
                self.error(
                    "E0010",
                    format!(
                        "this pattern has {} element{} but the matched value is a tuple of {}, `{}`",
                        elems.len(),
                        if elems.len() == 1 { "" } else { "s" },
                        tys.len(),
                        self.show(scrut_ty, fcx),
                    ),
                    span,
                );
                None
            }
            Ty::Error => Some(vec![Ty::Error; elems.len()]),
            _ => {
                let fresh: Vec<Ty> = elems.iter().map(|_| fcx.icx.fresh()).collect();
                let tuple = Ty::Tuple(fresh.clone());
                if fcx.icx.unify(scrut_ty, &tuple) {
                    Some(fresh)
                } else {
                    self.error(
                        "E0010",
                        format!(
                            "a tuple pattern cannot match a value of type `{}`",
                            self.show(scrut_ty, fcx),
                        ),
                        span,
                    );
                    None
                }
            }
        };
        let Some(tys) = tys else {
            // Bind the elements' names anyway, so their uses are not E0001
            // too.
            for e in elems {
                self.check_pattern(fcx, e, &Ty::Error);
            }
            return hir::Pattern::Wildcard;
        };
        hir::Pattern::Tuple(
            elems
                .iter()
                .zip(tys.iter())
                .map(|(e, t)| self.check_pattern(fcx, e, t))
                .collect(),
        )
    }

    /// `p1 | p2 | ...`: each alternative binds the first one's names, at
    /// their types (E0100).
    pub(super) fn or_pattern(
        &mut self,
        fcx: &mut FnCtx,
        alts: &[Spanned<ast::Pattern>],
        scrut_ty: &Ty,
    ) -> hir::Pattern {
        self.pats.frames.push(OrFrame::default());
        let mut out = Vec::new();
        for (i, alt) in alts.iter().enumerate() {
            self.pats.frames.last_mut().expect("pushed").current.clear();
            out.push(self.check_pattern(fcx, alt, scrut_ty));
            let frame = self.pats.frames.last_mut().expect("pushed");
            match &frame.first {
                None if i == 0 => frame.first = Some(frame.current.clone()),
                None => {}
                Some(first) => {
                    let missing: Vec<String> = first
                        .iter()
                        .filter(|b| !frame.current.iter().any(|c| c.name == b.name))
                        .map(|b| b.name.clone())
                        .collect();
                    for name in missing {
                        self.diagnostics.push(
                            Diagnostic::error("E0100", "the alternatives bind different names")
                                .with_primary_label(
                                    alt.span,
                                    format!("`{name}` is missing from this alternative"),
                                )
                                .with_note(format!(
                                    "every alternative of an or-pattern binds the same names; \
                                     the first one binds `{name}`"
                                )),
                        );
                    }
                }
            }
        }
        self.pats.frames.pop();
        hir::Pattern::Or(out)
    }

    /// `lo..hi` and `lo..=hi`, over `Int`, and `lo..=hi` over `Char`, as
    /// the parser reads them. An empty range is E0101.
    pub(super) fn range_pattern(
        &mut self,
        fcx: &mut FnCtx,
        lo: &Spanned<ast::Pattern>,
        hi: &Spanned<ast::Pattern>,
        inclusive: bool,
        scrut_ty: &Ty,
        span: Span,
    ) -> hir::Pattern {
        use ast::{Literal, Pattern as P};
        let (lo_v, hi_v, ty, shown) = match (&lo.value, &hi.value) {
            (P::Lit(Literal::Int(a)), P::Lit(Literal::Int(b))) => {
                (*a, *b, Ty::Int, (a.to_string(), b.to_string()))
            }
            (P::Lit(Literal::Char(a)), P::Lit(Literal::Char(b))) => (
                *a as i64,
                *b as i64,
                Ty::Char,
                (format!("{a:?}"), format!("{b:?}")),
            ),
            _ => {
                self.error(
                    "E0010",
                    "a range pattern's ends must be two `Int` or two `Char` literals",
                    span,
                );
                return hir::Pattern::Wildcard;
            }
        };
        self.unify_pattern_ty(fcx, &ty, scrut_ty, span);
        let empty = if inclusive { lo_v > hi_v } else { lo_v >= hi_v };
        if empty {
            let (a, b) = shown;
            let why = if lo_v == hi_v {
                format!("`{a}..{b}` stops before its start")
            } else {
                format!("`{a}` is above `{b}`")
            };
            self.diagnostics.push(
                Diagnostic::error("E0101", "this range matches nothing")
                    .with_primary_label(span, why),
            );
            // Kept as written, not as `_`, so the arms after it are not
            // reported unreachable as well; the program does not compile.
        }
        hir::Pattern::Range {
            lo: lo_v,
            hi: if inclusive { hi_v } else { hi_v - 1 },
        }
    }

    /// A destructuring pattern in a `let` or a `for` must match every value
    /// of `ty`: E0022, with the values it misses, when usefulness finds one.
    pub(super) fn require_irrefutable(
        &mut self,
        fcx: &mut FnCtx,
        pat: &hir::Pattern,
        ty: &Ty,
        span: Span,
        what: &str,
    ) {
        let ty = fcx.icx.apply(ty);
        if matches!(ty, Ty::Var(_) | Ty::Param(_) | Ty::Error | Ty::Never) {
            return;
        }
        let sums = self.sums.clone();
        let cx = usefulness::MatchCx::new(&sums).naming_values();
        let row = vec![to_useful_pat(pat)];
        let witnesses = cx.usefulness(&[row], &[usefulness::Pat::Wild], &[ty]);
        if witnesses.is_empty() {
            return;
        }
        let mut rendered: Vec<String> = witnesses
            .iter()
            .filter_map(|w| w.first())
            .map(|p| format!("`{}`", self.render_witness(p)))
            .collect();
        rendered.dedup();
        self.diagnostics.push(
            Diagnostic::error("E0022", format!("refutable pattern in {what}"))
                .with_primary_label(
                    span,
                    format!(
                        "{} {} not matched",
                        rendered.join(", "),
                        if rendered.len() == 1 { "is" } else { "are" }
                    ),
                )
                .with_note(
                    "`let` and `for` take a pattern every value matches: names, `_`, tuples \
                     of these, and a variant of a sum that has only one; use `match` for the rest"
                        .to_string(),
                ),
        );
    }

    /// How a `for` loop binds each element of type `ty`: a name or `_` as
    /// before, or an irrefutable pattern (5.0 spec §4.3, §12 item 13).
    ///
    /// A `for` loop variable is immutable, so assigning it is `E0060`. `mut`
    /// does not parse in this position (`for mut i in …` is P0001), so the
    /// `Ident` arm's `is_mut` is always `false`.
    pub(super) fn for_loop_binding(
        &mut self,
        fcx: &mut FnCtx,
        pattern: &Spanned<ast::Pattern>,
        ty: &Ty,
    ) -> LoopBind {
        match &pattern.value {
            ast::Pattern::Ident { name, .. } => LoopBind::Local(self.bind_local(
                fcx,
                name.value.clone(),
                ty.clone(),
                false,
                name.span,
                Binding::Other,
            )),
            ast::Pattern::Wildcard => LoopBind::Local(self.bind_local(
                fcx,
                "_".to_string(),
                ty.clone(),
                false,
                pattern.span,
                Binding::Other,
            )),
            _ => {
                let errors = self.error_count();
                let pat = self.check_pattern(fcx, pattern, ty);
                if self.error_count() == errors {
                    self.require_irrefutable(fcx, &pat, ty, pattern.span, "a `for` loop");
                }
                LoopBind::Pattern(pat)
            }
        }
    }

    /// Report each alternative of an or-pattern in a reachable, unguarded
    /// arm that no value can reach: every value it matches is matched by an
    /// earlier arm, or by an earlier alternative of the same or-pattern. An
    /// alternative inside one already reported is not reported again.
    pub(super) fn check_or_alternatives(
        &mut self,
        cx: &usefulness::MatchCx,
        prior: &[Vec<usefulness::Pat>],
        pat: &usefulness::Pat,
        spans: &[Vec<Span>],
        col: &[Ty],
    ) {
        let tree = or_tree(pat);
        let mut dead: Vec<(usize, usize)> = Vec::new();
        for (node, (parent, alts)) in tree.iter().enumerate() {
            if ancestors(&tree, *parent).any(|a| dead.contains(&a)) {
                continue;
            }
            for k in 0..*alts {
                let q = restrict_or(pat, node, k..k + 1);
                let mut matrix = prior.to_vec();
                if k > 0 {
                    matrix.push(vec![restrict_or(pat, node, 0..k)]);
                }
                if !cx
                    .usefulness(&matrix, std::slice::from_ref(&q), col)
                    .is_empty()
                {
                    continue;
                }
                dead.push((node, k));
                let Some(span) = spans.get(node).and_then(|s| s.get(k)) else {
                    continue;
                };
                self.diagnostics.push(
                    Diagnostic::warning("E0021", "unreachable alternative")
                        .with_primary_label(*span, "this alternative is never reached")
                        .with_note(
                            "an earlier arm, or an earlier alternative of this or-pattern, \
                             already matches every value this one would"
                                .to_string(),
                        ),
                );
            }
        }
    }
}

/// The spans of each or-pattern's alternatives in `hir`, in the pre-order
/// [`or_tree`] numbers them in. The checked pattern follows the source's
/// shape wherever it has an or-pattern inside it; where it does not (a
/// pattern replaced by `_` after an error), it has none to number.
pub(super) fn or_alternative_spans(
    ast: &Spanned<ast::Pattern>,
    hir: &hir::Pattern,
    out: &mut Vec<Vec<Span>>,
) {
    match (&ast.value, hir) {
        (ast::Pattern::Or(a), hir::Pattern::Or(h)) if a.len() == h.len() => {
            out.push(a.iter().map(|x| x.span).collect());
            for (x, y) in a.iter().zip(h) {
                or_alternative_spans(x, y, out);
            }
        }
        (ast::Pattern::TupleStruct { fields: a, .. }, hir::Pattern::Variant { fields: h, .. })
        | (ast::Pattern::Tuple(a), hir::Pattern::Tuple(h))
            if a.len() == h.len() =>
        {
            for (x, y) in a.iter().zip(h) {
                or_alternative_spans(x, y, out);
            }
        }
        _ => {
            // Keep the numbering: one entry, with no spans, per or-pattern.
            let count = or_tree(&to_useful_pat(hir)).len();
            out.extend(std::iter::repeat(Vec::new()).take(count));
        }
    }
}

/// Each or-pattern of `pat`, in pre-order: the or-pattern and alternative it
/// sits in, if any, and its number of alternatives.
fn or_tree(pat: &usefulness::Pat) -> Vec<(Option<(usize, usize)>, usize)> {
    fn walk(
        p: &usefulness::Pat,
        parent: Option<(usize, usize)>,
        out: &mut Vec<(Option<(usize, usize)>, usize)>,
    ) {
        match p {
            usefulness::Pat::Wild => {}
            usefulness::Pat::Ctor(_, args) => {
                for a in args {
                    walk(a, parent, out);
                }
            }
            usefulness::Pat::Or(alts) => {
                let me = out.len();
                out.push((parent, alts.len()));
                for (k, a) in alts.iter().enumerate() {
                    walk(a, Some((me, k)), out);
                }
            }
        }
    }
    let mut out = Vec::new();
    walk(pat, None, &mut out);
    out
}

/// The or-pattern and alternative `start` names, and those it sits in.
fn ancestors(
    tree: &[(Option<(usize, usize)>, usize)],
    start: Option<(usize, usize)>,
) -> impl Iterator<Item = (usize, usize)> + '_ {
    std::iter::successors(start, move |(node, _)| tree[*node].0)
}

/// `pat` with its `target`-th or-pattern, counted as [`or_tree`] counts,
/// cut to the alternatives in `keep`.
fn restrict_or(pat: &usefulness::Pat, target: usize, keep: Range<usize>) -> usefulness::Pat {
    fn go(
        p: &usefulness::Pat,
        target: usize,
        keep: &Range<usize>,
        n: &mut usize,
    ) -> usefulness::Pat {
        match p {
            usefulness::Pat::Wild => usefulness::Pat::Wild,
            usefulness::Pat::Ctor(c, args) => usefulness::Pat::Ctor(
                c.clone(),
                args.iter().map(|a| go(a, target, keep, n)).collect(),
            ),
            usefulness::Pat::Or(alts) => {
                let me = *n;
                *n += 1;
                let mut kept = Vec::new();
                for (k, a) in alts.iter().enumerate() {
                    let a = go(a, target, keep, n);
                    if me != target || keep.contains(&k) {
                        kept.push(a);
                    }
                }
                usefulness::Pat::Or(kept)
            }
        }
    }
    go(pat, target, &keep, &mut 0)
}
