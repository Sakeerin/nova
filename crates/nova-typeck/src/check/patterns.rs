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
    /// can make mutable (E0060's note says so): its head as E0060's note
    /// quotes it, `(…)` or `W(…)`.
    pub(super) let_head: Option<String>,
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

/// A destructuring `let`'s pattern as E0060's note quotes it after
/// `let mut `: its head with the rest elided, `(…)` for a tuple, `W(…)` or
/// `Wrap::W(…)` for a variant.
pub(super) fn let_head(pattern: &ast::Pattern) -> String {
    match pattern {
        ast::Pattern::TupleStruct { path, .. } => {
            let path: Vec<&str> = path.segments.iter().map(|s| s.value.as_str()).collect();
            format!("{}(…)", path.join("::"))
        }
        ast::Pattern::Tuple(_) => "(…)".to_string(),
        _ => "…".to_string(),
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
                let binding = match &self.pats.let_head {
                    Some(head) => Binding::LetPattern(head.clone()),
                    None => Binding::Pattern,
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
        // The variants the name could mean, of a sum the matched value's
        // type unifies with: the matched sum's, and those in scope; any sum's
        // while that type is not yet known.
        let col = fcx.icx.apply(ty);
        let fits = |sum: nova_resolver::DefId| match &col {
            Ty::Sum { def_id, .. } => *def_id == sum,
            Ty::Var(_) | Ty::Error => true,
            _ => false,
        };
        let matched = super::fixes::scrutinee_sum(fcx, ty);
        let mut variants: Vec<String> = Vec::new();
        let mut unimported = None;
        if let Some(sum) = matched {
            if let DefKind::Sum { variants: vs, .. } = &self.defs.def(sum).kind {
                variants.extend(vs.iter().map(|v| v.name.clone()));
                if vs.iter().any(|v| v.name == name.value) {
                    unimported = Some(self.defs.def(sum).name.clone());
                }
            }
        }
        for (n, entry) in self.defs.names_in_scope(self.cur_module) {
            if let ScopeEntry::Value(Res::Variant(sum, _)) = entry {
                if fits(sum) {
                    variants.push(n);
                }
            }
        }
        let near = if is_const || unimported.is_some() {
            None
        } else {
            suggest::closest(&name.value, variants.iter().map(String::as_str))
        };
        if let Some(sum) = &unimported {
            // The matched sum has a variant of this very name, which is not
            // in scope here.
            d = d.with_note(format!(
                "`{0}` is a variant of `{sum}` but is not in scope here: write \
                 `{sum}::{0}`, or import it",
                name.value
            ));
        }
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
        let witnesses = cx.usefulness(&[row], &[usefulness::Pat::Wild], std::slice::from_ref(&ty));
        if witnesses.is_empty() {
            return;
        }
        let mut rendered: Vec<String> = witnesses
            .iter()
            .filter_map(|w| w.first())
            .map(|p| format!("`{}`", self.render_witness(p, &ty)))
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

    /// Report each alternative of an or-pattern in a reachable arm that no
    /// value can reach, in the order the test chain tries them (MIR's
    /// `lower_pattern_test`): every value that matches the arm with this
    /// alternative is matched by an earlier arm, or takes an earlier
    /// alternative of this or-pattern or of one it sits in. An alternative
    /// inside one already reported is not reported again.
    ///
    /// A guarded arm's alternatives are checked too: its guard runs once,
    /// with the bindings of the first alternative that matches, and a false
    /// guard goes on to the next arm (spec §4.6), so an alternative that
    /// only a false guard could lead to, as in Rust, is never reached; its
    /// E0021 says so.
    pub(super) fn check_or_alternatives(
        &mut self,
        cx: &usefulness::MatchCx,
        prior: &[Vec<usefulness::Pat>],
        pat: &usefulness::Pat,
        spans: &[Vec<Span>],
        col: &[Ty],
        guarded: bool,
    ) {
        let tree = or_tree(pat);
        let mut dead: Vec<(usize, usize)> = Vec::new();
        for (node, (parent, alts)) in tree.iter().enumerate() {
            if ancestors(&tree, *parent).any(|a| dead.contains(&a)) {
                continue;
            }
            // The alternative each enclosing or-pattern takes on the way
            // here, outermost first.
            let mut path: Vec<(usize, usize)> = ancestors(&tree, *parent).collect();
            path.reverse();
            // A value reaches an enclosing or-pattern's alternative `j` only
            // if it matches none of the alternatives before `j`, as the
            // chain tries them in turn.
            let mut earlier: Vec<Vec<usefulness::Pat>> = Vec::new();
            for (i, &(a, j)) in path.iter().enumerate() {
                if j > 0 {
                    let mut keep = only(&path[..i]);
                    keep.push((a, 0..j));
                    earlier.push(vec![restrict_or(pat, &keep)]);
                }
            }
            for k in 0..*alts {
                let mut keep = only(&path);
                keep.push((node, k..k + 1));
                let q = restrict_or(pat, &keep);
                let mut matrix = prior.to_vec();
                matrix.extend(earlier.iter().cloned());
                if k > 0 {
                    let mut keep = only(&path);
                    keep.push((node, 0..k));
                    matrix.push(vec![restrict_or(pat, &keep)]);
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
                let mut d = Diagnostic::warning("E0021", "unreachable alternative")
                    .with_primary_label(*span, "this alternative is never reached")
                    .with_note(
                        "an earlier arm, or an earlier alternative of this or-pattern, \
                         already matches every value this one would"
                            .to_string(),
                    );
                if guarded
                    && !cx
                        .usefulness(prior, std::slice::from_ref(&q), col)
                        .is_empty()
                {
                    d = d.with_note(
                        "the arm's guard runs once, with the bindings of the first \
                         alternative that matches; when it is false, matching goes on with \
                         the next arm, not with this alternative"
                            .to_string(),
                    );
                }
                self.diagnostics.push(d);
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

/// Each `(node, k)` of `path` as a restriction to that one alternative.
fn only(path: &[(usize, usize)]) -> Vec<(usize, Range<usize>)> {
    path.iter().map(|&(n, k)| (n, k..k + 1)).collect()
}

/// `pat` with each or-pattern `keep` names, counted as [`or_tree`] counts,
/// cut to the alternatives in its range.
fn restrict_or(pat: &usefulness::Pat, keep: &[(usize, Range<usize>)]) -> usefulness::Pat {
    fn go(p: &usefulness::Pat, keep: &[(usize, Range<usize>)], n: &mut usize) -> usefulness::Pat {
        match p {
            usefulness::Pat::Wild => usefulness::Pat::Wild,
            usefulness::Pat::Ctor(c, args) => {
                usefulness::Pat::Ctor(c.clone(), args.iter().map(|a| go(a, keep, n)).collect())
            }
            usefulness::Pat::Or(alts) => {
                let me = *n;
                *n += 1;
                let range = keep.iter().find(|(t, _)| *t == me).map(|(_, r)| r);
                let mut kept = Vec::new();
                for (k, a) in alts.iter().enumerate() {
                    let a = go(a, keep, n);
                    if range.map_or(true, |r| r.contains(&k)) {
                        kept.push(a);
                    }
                }
                usefulness::Pat::Or(kept)
            }
        }
    }
    go(pat, keep, &mut 0)
}

impl<'a> Checker<'a> {
    /// `for p in a`, `a` an array (5.0 spec §4.5), desugars to
    /// `{ let __arr = a  let __n = __arr.len()  let mut __i = 0
    ///    while __i < __n { <bind p to __arr[__i]>  __i = __i + 1  body } }`.
    ///
    /// `a` is evaluated once, and each element is read when the loop reaches
    /// it, so a write to a later element is seen. The index is advanced
    /// before the body, so `continue` is safe. The hidden locals are
    /// unscoped, so no source name reaches or shadows them.
    pub(super) fn check_for_array(
        &mut self,
        fcx: &mut FnCtx,
        pattern: &Spanned<ast::Pattern>,
        iterable: hir::Expr,
        elem: Ty,
        body: &Spanned<ast::Block>,
        span: Span,
    ) -> hir::Expr {
        let arr_ty = iterable.ty.clone();
        let arr = fcx.new_local_unscoped("__arr".to_string(), arr_ty.clone(), false, span);
        let len = fcx.new_local_unscoped("__n".to_string(), Ty::Int, false, span);
        let idx = fcx.new_local_unscoped("__i".to_string(), Ty::Int, true, span);

        fcx.scopes.push(Default::default());
        let bind = self.for_loop_binding(fcx, pattern, &elem);
        fcx.loop_depth += 1;
        let body_hir = self.check_block(fcx, &body.value, body.span);
        fcx.loop_depth -= 1;
        fcx.scopes.pop();

        let expr = |kind, ty| hir::Expr { kind, ty, span };
        let read = |local, ty| expr(hir::ExprKind::Local(local), ty);
        let let_stmt = |local, init| {
            expr(
                hir::ExprKind::Let {
                    local,
                    init: Box::new(init),
                },
                Ty::Unit,
            )
        };
        let element = expr(
            hir::ExprKind::Index {
                target: Box::new(read(arr, arr_ty.clone())),
                index: Box::new(read(idx, Ty::Int)),
            },
            elem,
        );
        let advance = expr(
            hir::ExprKind::Assign {
                local: idx,
                value: Box::new(expr(
                    hir::ExprKind::Binary {
                        op: hir::BinOp::Add,
                        lhs: Box::new(read(idx, Ty::Int)),
                        rhs: Box::new(expr(hir::ExprKind::IntLit(1), Ty::Int)),
                    },
                    Ty::Int,
                )),
            },
            Ty::Unit,
        );
        let cond = expr(
            hir::ExprKind::Binary {
                op: hir::BinOp::Lt,
                lhs: Box::new(read(idx, Ty::Int)),
                rhs: Box::new(read(len, Ty::Int)),
            },
            Ty::Bool,
        );
        let while_body = expr(
            hir::ExprKind::Block {
                stmts: vec![bind.bind(element, span), advance, body_hir],
                trailing: None,
            },
            Ty::Unit,
        );
        let len_init = expr(
            hir::ExprKind::ArrayLen {
                target: Box::new(read(arr, arr_ty)),
            },
            Ty::Int,
        );
        expr(
            hir::ExprKind::Block {
                stmts: vec![
                    let_stmt(arr, iterable),
                    let_stmt(len, len_init),
                    let_stmt(idx, expr(hir::ExprKind::IntLit(0), Ty::Int)),
                    expr(
                        hir::ExprKind::While {
                            cond: Box::new(cond),
                            body: Box::new(while_body),
                        },
                        Ty::Unit,
                    ),
                ],
                trailing: None,
            },
            Ty::Unit,
        )
    }
}
