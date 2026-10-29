//! Maranget's usefulness algorithm for `match` exhaustiveness and reachability.
//!
//! Implements the decision procedure from Luc Maranget, *"Warnings for pattern
//! matching"* (JFP 2007). A candidate row `q` is **useful** with respect to a
//! pattern matrix `P` when some value matches `q` but no row of `P`. From that
//! one predicate:
//!
//! - a `match` is **exhaustive** iff a single wildcard row is *not* useful
//!   against the arm matrix (nothing is left uncovered);
//! - arm *i* is **reachable** iff its pattern row is useful against the matrix
//!   of the earlier arms.
//!
//! The search reconstructs **witness** patterns for the values a non-exhaustive
//! match fails to cover (e.g. `Some(_)`, `false`).
//!
//! The algorithm is written for arbitrarily nested patterns, and since 5.0b
//! (spec `docs/superpowers/specs/2026-10-26-phase-5-0-the-language-and-std-a-compiler-needs-design.md`
//! §4.4) the checker feeds it nested ones: variants and tuples to any depth,
//! or-patterns, which it expands into their alternatives, and `Int`, `Char`
//! and `String` literals and ranges.
//!
//! **Ranges are not split.** A literal and a range are each a constructor of
//! their own, equal only to the same literal or the same range, of a type
//! with too many values to list. So a `match` on them needs a `_` or a name to
//! be exhaustive, and a value an earlier range covers is still useful to a
//! later arm unless the same range or literal came first. Treating
//! overlapping constructors as disjoint only ever drops rows from a
//! specialization, which makes a row *more* useful: an arm is never reported
//! unreachable when it is not, and a match is never accepted when it is not
//! exhaustive. What it gives up is reporting an arm that earlier ranges
//! cover, and accepting a match whose ranges cover a whole type without a
//! `_` (5.0 spec §4.4, §12 item 14).

use nova_hir::{SumType, Ty};
use nova_resolver::DefId;

/// A pattern constructor: the "shape" a value can take that a pattern tests.
#[derive(Clone, PartialEq, Debug)]
pub enum Ctor {
    /// The `variant`-th variant of the sum type `DefId`.
    Variant(DefId, u32),
    Bool(bool),
    Int(i64),
    Str(String),
    Char(char),
    /// An `Int` range, or a `Char` range by code point, both ends included.
    Range(i64, i64),
    /// The one constructor of a tuple type of this arity.
    Tuple(u32),
}

/// A pattern in the checker's normalized form: either a wildcard (which also
/// stands in for an irrefutable binding) or a constructor applied to
/// sub-patterns.
#[derive(Clone, Debug)]
pub enum Pat {
    Wild,
    Ctor(Ctor, Vec<Pat>),
    /// An or-pattern: it matches what any alternative matches.
    Or(Vec<Pat>),
}

/// Context for the algorithm: the module's sum-type declarations, used to
/// enumerate a type's constructor signature and their arities.
pub struct MatchCx<'a> {
    sums: &'a [SumType],
    /// Whether a witness names a value a column's literals and ranges miss,
    /// as E0022 shows it, rather than `_`, as E0020 does.
    name_values: bool,
}

/// One row of witness patterns (aligned with the current columns).
type Witness = Vec<Pat>;

impl<'a> MatchCx<'a> {
    pub fn new(sums: &'a [SumType]) -> Self {
        Self {
            sums,
            name_values: false,
        }
    }

    /// This context, its witnesses naming a value of an `Int`, `Char` or
    /// `String` column that the column's literals and ranges miss.
    pub fn naming_values(self) -> Self {
        Self {
            name_values: true,
            ..self
        }
    }

    /// Witnesses that `q` matches but no row of `matrix` does. An empty result
    /// means `q` is **not useful**. Each row of `matrix`, `q`, and each witness
    /// is aligned with `col_tys`.
    pub fn usefulness(&self, matrix: &[Vec<Pat>], q: &[Pat], col_tys: &[Ty]) -> Vec<Witness> {
        // Base case: no columns left. Useful iff the matrix has no rows (a value
        // reached this far unmatched); the sole witness is the empty row.
        if q.is_empty() {
            return if matrix.is_empty() {
                vec![Vec::new()]
            } else {
                Vec::new()
            };
        }
        // A row whose head is an or-pattern stands for one row per
        // alternative.
        let expanded;
        let matrix = if matrix.iter().any(|r| matches!(r[0], Pat::Or(_))) {
            expanded = expand_or_heads(matrix);
            &expanded[..]
        } else {
            matrix
        };

        match &q[0] {
            // `q` is useful when one of its alternatives is.
            Pat::Or(alts) => {
                let mut out = Vec::new();
                for a in alts {
                    let mut row = vec![a.clone()];
                    row.extend_from_slice(&q[1..]);
                    out.extend(self.usefulness(matrix, &row, col_tys));
                }
                out
            }
            Pat::Ctor(c, args) => {
                let spec = self.specialize(matrix, c);
                let spec_tys = self.specialized_tys(c, col_tys);
                let mut sq = args.clone();
                sq.extend_from_slice(&q[1..]);
                self.usefulness(&spec, &sq, &spec_tys)
                    .into_iter()
                    .map(|w| reassemble(c, w, args.len()))
                    .collect()
            }
            Pat::Wild => {
                let present = column_ctors(matrix);
                let col0 = tuple_ty(&col_tys[0], &present);
                let col_tys = &with_head(col0, col_tys)[..];
                if self.is_complete(&col_tys[0], &present) {
                    // Every constructor of the type is present: `q` is useful
                    // only if it is useful under some specific constructor.
                    let mut out = Vec::new();
                    for c in self.signature(&col_tys[0]) {
                        let arity = self.arity(&c);
                        let spec = self.specialize(matrix, &c);
                        let spec_tys = self.specialized_tys(&c, col_tys);
                        let mut sq = vec![Pat::Wild; arity];
                        sq.extend_from_slice(&q[1..]);
                        out.extend(
                            self.usefulness(&spec, &sq, &spec_tys)
                                .into_iter()
                                .map(|w| reassemble(&c, w, arity)),
                        );
                    }
                    out
                } else {
                    // The column's constructors are incomplete: recurse on the
                    // default matrix and prepend a missing constructor (or `_`).
                    let def = default_matrix(matrix);
                    let tail = self.usefulness(&def, &q[1..], &col_tys[1..]);
                    let heads = self.missing_heads(&col_tys[0], &present);
                    let mut out = Vec::new();
                    for w in &tail {
                        for h in &heads {
                            let mut row = vec![h.clone()];
                            row.extend_from_slice(w);
                            out.push(row);
                        }
                    }
                    // Where E0022 asks, also the values under each present
                    // constructor, so `for (1, true) in p` names `(1, false)`
                    // beside `(0, _)`.
                    if self.name_values {
                        for c in &present {
                            let arity = self.arity(c);
                            let spec = self.specialize(matrix, c);
                            let spec_tys = self.specialized_tys(c, col_tys);
                            let mut sq = vec![Pat::Wild; arity];
                            sq.extend_from_slice(&q[1..]);
                            out.extend(
                                self.usefulness(&spec, &sq, &spec_tys)
                                    .into_iter()
                                    .map(|w| reassemble(c, w, arity)),
                            );
                        }
                    }
                    out
                }
            }
        }
    }

    /// Specialize the matrix by constructor `c`: keep rows whose head matches
    /// `c` (expanding its sub-patterns into new leading columns) or is a
    /// wildcard (contributing `arity(c)` fresh wildcards), dropping the rest.
    fn specialize(&self, matrix: &[Vec<Pat>], c: &Ctor) -> Vec<Vec<Pat>> {
        let arity = self.arity(c);
        let mut out = Vec::new();
        for row in matrix {
            match &row[0] {
                Pat::Ctor(rc, rargs) if rc == c => {
                    let mut nr = rargs.clone();
                    nr.extend_from_slice(&row[1..]);
                    out.push(nr);
                }
                Pat::Ctor(_, _) => {}
                Pat::Wild => {
                    let mut nr = vec![Pat::Wild; arity];
                    nr.extend_from_slice(&row[1..]);
                    out.push(nr);
                }
                Pat::Or(alts) => {
                    let rows: Vec<Vec<Pat>> = alts
                        .iter()
                        .map(|a| {
                            let mut r = vec![a.clone()];
                            r.extend_from_slice(&row[1..]);
                            r
                        })
                        .collect();
                    out.extend(self.specialize(&rows, c));
                }
            }
        }
        out
    }

    /// Column types after specializing on `c`: `c`'s argument types replace the
    /// first column, the rest are unchanged.
    fn specialized_tys(&self, c: &Ctor, col_tys: &[Ty]) -> Vec<Ty> {
        let mut tys = self.arg_tys(c, &col_tys[0]);
        tys.extend_from_slice(&col_tys[1..]);
        tys
    }

    /// The types of a constructor's fields, with the column type's generic
    /// arguments substituted in.
    fn arg_tys(&self, c: &Ctor, ty0: &Ty) -> Vec<Ty> {
        match c {
            Ctor::Variant(sum_id, vi) => {
                let args = match ty0 {
                    Ty::Sum { args, .. } => args.clone(),
                    _ => Vec::new(),
                };
                self.variant(*sum_id, *vi)
                    .map(|v| v.fields.iter().map(|f| f.subst(&args)).collect())
                    .unwrap_or_default()
            }
            Ctor::Tuple(n) => match ty0 {
                Ty::Tuple(elems) if elems.len() == *n as usize => elems.clone(),
                _ => vec![Ty::Error; *n as usize],
            },
            Ctor::Bool(_) | Ctor::Int(_) | Ctor::Str(_) | Ctor::Char(_) | Ctor::Range(..) => {
                Vec::new()
            }
        }
    }

    fn arity(&self, c: &Ctor) -> usize {
        match c {
            Ctor::Variant(sum_id, vi) => self
                .variant(*sum_id, *vi)
                .map(|v| v.fields.len())
                .unwrap_or(0),
            Ctor::Tuple(n) => *n as usize,
            Ctor::Bool(_) | Ctor::Int(_) | Ctor::Str(_) | Ctor::Char(_) | Ctor::Range(..) => 0,
        }
    }

    /// The complete constructor signature of a type, or empty for a type with
    /// no finite, enumerable signature (integers, strings, opaque types).
    fn signature(&self, ty: &Ty) -> Vec<Ctor> {
        match ty {
            Ty::Bool => vec![Ctor::Bool(true), Ctor::Bool(false)],
            Ty::Sum { def_id, .. } => self
                .sums
                .iter()
                .find(|s| s.def_id == *def_id)
                .map(|s| {
                    (0..s.variants.len() as u32)
                        .map(|i| Ctor::Variant(*def_id, i))
                        .collect()
                })
                .unwrap_or_default(),
            Ty::Tuple(elems) => vec![Ctor::Tuple(elems.len() as u32)],
            _ => Vec::new(),
        }
    }

    /// Whether `present` covers every constructor of `ty`. Types with no finite
    /// signature (integers, strings, …) are never complete via listed values.
    fn is_complete(&self, ty: &Ty, present: &[Ctor]) -> bool {
        let sig = self.signature(ty);
        !sig.is_empty() && sig.iter().all(|c| present.contains(c))
    }

    /// Missing constructors of `ty` as witness heads (each applied to
    /// wildcards). For a type with no enumerable signature, a single `_`.
    fn missing_heads(&self, ty: &Ty, present: &[Ctor]) -> Vec<Pat> {
        let sig = self.signature(ty);
        if sig.is_empty() {
            // A type with too many values to list: where asked, name one the
            // column's literals and ranges miss, so the witness says which
            // value is not matched, `(_, 0)` rather than `(_, _)`.
            let named = missing_value(ty, present).filter(|_| self.name_values);
            return vec![named.map_or(Pat::Wild, |c| Pat::Ctor(c, Vec::new()))];
        }
        if present.is_empty() {
            // No row names a constructor here: `_` says it all, where
            // listing each would repeat the rest of the witness per value.
            return vec![Pat::Wild];
        }
        let missing: Vec<Pat> = sig
            .iter()
            .filter(|c| !present.contains(c))
            .map(|c| Pat::Ctor(c.clone(), vec![Pat::Wild; self.arity(c)]))
            .collect();
        if missing.is_empty() {
            vec![Pat::Wild]
        } else {
            missing
        }
    }

    fn variant(&self, sum_id: DefId, vi: u32) -> Option<&nova_hir::Variant> {
        self.sums
            .iter()
            .find(|s| s.def_id == sum_id)
            .and_then(|s| s.variants.get(vi as usize))
    }
}

/// A value of `ty`, an `Int`, a `Char` or a `String`, that no literal or
/// range of `present` matches: the first from `0`, `'a'` or `""` on. `None`
/// when `present` holds none of the type's constructors (`_` says it all)
/// or a short search finds no such value.
fn missing_value(ty: &Ty, present: &[Ctor]) -> Option<Ctor> {
    let covers = |v: i64| {
        present.iter().any(|c| match c {
            Ctor::Int(n) => *n == v,
            Ctor::Char(ch) => *ch as i64 == v,
            Ctor::Range(lo, hi) => *lo <= v && v <= *hi,
            _ => false,
        })
    };
    let first_free = |start: i64| {
        let mut v = start;
        for _ in 0..=present.len() {
            if !covers(v) {
                return Some(v);
            }
            let past = present.iter().find_map(|c| match c {
                Ctor::Range(lo, hi) if *lo <= v && v <= *hi => Some(*hi),
                _ => None,
            });
            v = past.unwrap_or(v).checked_add(1)?;
        }
        None
    };
    match ty {
        Ty::Int
            if present
                .iter()
                .any(|c| matches!(c, Ctor::Int(_) | Ctor::Range(..))) =>
        {
            first_free(0).map(Ctor::Int)
        }
        Ty::Char
            if present
                .iter()
                .any(|c| matches!(c, Ctor::Char(_) | Ctor::Range(..))) =>
        {
            first_free('a' as i64)
                .and_then(|v| u32::try_from(v).ok())
                .and_then(char::from_u32)
                .map(Ctor::Char)
        }
        Ty::String if present.iter().any(|c| matches!(c, Ctor::Str(_))) => {
            std::iter::once(String::new())
                .chain(('a'..='z').map(String::from))
                .find(|s| !present.contains(&Ctor::Str(s.clone())))
                .map(Ctor::Str)
        }
        _ => None,
    }
}

/// `matrix` with each row whose head is an or-pattern replaced by one row
/// per alternative, until no head is one.
fn expand_or_heads(matrix: &[Vec<Pat>]) -> Vec<Vec<Pat>> {
    let mut out = Vec::new();
    for row in matrix {
        match &row[0] {
            Pat::Or(alts) => {
                let rows: Vec<Vec<Pat>> = alts
                    .iter()
                    .map(|a| {
                        let mut r = vec![a.clone()];
                        r.extend_from_slice(&row[1..]);
                        r
                    })
                    .collect();
                out.extend(expand_or_heads(&rows));
            }
            _ => out.push(row.clone()),
        }
    }
    out
}

/// The first column's type, as a tuple of the arity the column's rows use
/// when inference has not yet made it one: a tuple has one constructor
/// whatever its elements' types.
fn tuple_ty(ty: &Ty, present: &[Ctor]) -> Option<Ty> {
    if matches!(ty, Ty::Tuple(_)) {
        return None;
    }
    present.iter().find_map(|c| match c {
        Ctor::Tuple(n) => Some(Ty::Tuple(vec![Ty::Error; *n as usize])),
        _ => None,
    })
}

/// `col_tys` with its first type replaced by `head`, if there is one.
fn with_head(head: Option<Ty>, col_tys: &[Ty]) -> Vec<Ty> {
    let mut tys = col_tys.to_vec();
    if let Some(h) = head {
        tys[0] = h;
    }
    tys
}

/// Distinct constructors appearing in the matrix's first column.
fn column_ctors(matrix: &[Vec<Pat>]) -> Vec<Ctor> {
    let mut out: Vec<Ctor> = Vec::new();
    for row in matrix {
        if let Pat::Ctor(c, _) = &row[0] {
            if !out.contains(c) {
                out.push(c.clone());
            }
        }
    }
    out
}

/// The default matrix: rows whose head is a wildcard, with that column dropped.
fn default_matrix(matrix: &[Vec<Pat>]) -> Vec<Vec<Pat>> {
    matrix
        .iter()
        .filter(|r| matches!(r[0], Pat::Wild))
        .map(|r| r[1..].to_vec())
        .collect()
}

/// Rebuild one witness row after a specialization by `c`: the first `arity`
/// columns are `c`'s arguments, the rest follow unchanged.
fn reassemble(c: &Ctor, mut w: Witness, arity: usize) -> Witness {
    let rest = w.split_off(arity.min(w.len()));
    let mut row = vec![Pat::Ctor(c.clone(), w)];
    row.extend(rest);
    row
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cx() -> MatchCx<'static> {
        // Bool/Int cases need no sum-type table.
        MatchCx::new(&[])
    }

    fn ctor(c: Ctor) -> Pat {
        Pat::Ctor(c, Vec::new())
    }

    #[test]
    fn bool_true_and_false_is_exhaustive() {
        let matrix = vec![vec![ctor(Ctor::Bool(true))], vec![ctor(Ctor::Bool(false))]];
        let w = cx().usefulness(&matrix, &[Pat::Wild], &[Ty::Bool]);
        assert!(w.is_empty(), "expected exhaustive, got witnesses {w:?}");
    }

    #[test]
    fn bool_missing_false_yields_false_witness() {
        let matrix = vec![vec![ctor(Ctor::Bool(true))]];
        let w = cx().usefulness(&matrix, &[Pat::Wild], &[Ty::Bool]);
        assert_eq!(w.len(), 1);
        assert!(matches!(w[0][0], Pat::Ctor(Ctor::Bool(false), _)));
    }

    #[test]
    fn int_literals_need_a_wildcard() {
        let matrix = vec![vec![ctor(Ctor::Int(0))], vec![ctor(Ctor::Int(1))]];
        let w = cx().usefulness(&matrix, &[Pat::Wild], &[Ty::Int]);
        assert_eq!(w.len(), 1);
        assert!(matches!(w[0][0], Pat::Wild));
    }

    #[test]
    fn int_with_wildcard_is_exhaustive() {
        let matrix = vec![vec![ctor(Ctor::Int(0))], vec![Pat::Wild]];
        let w = cx().usefulness(&matrix, &[Pat::Wild], &[Ty::Int]);
        assert!(w.is_empty());
    }

    #[test]
    fn arm_covered_by_earlier_wildcard_is_not_useful() {
        // Reachability: `1` is useless once a wildcard row precedes it.
        let prior = vec![vec![Pat::Wild]];
        let w = cx().usefulness(&prior, &[ctor(Ctor::Int(1))], &[Ty::Int]);
        assert!(w.is_empty(), "arm should be unreachable");
    }

    #[test]
    fn distinct_literal_is_useful() {
        let prior = vec![vec![ctor(Ctor::Int(0))]];
        let w = cx().usefulness(&prior, &[ctor(Ctor::Int(1))], &[Ty::Int]);
        assert!(!w.is_empty(), "a fresh literal should be reachable");
    }

    // --- 5.0b: nested, tuple, or- and range patterns (5.0 spec §4.4) ---

    const OPT: DefId = DefId(1);
    const RES: DefId = DefId(2);

    /// `Opt<T> = Some(T) | None` and `Res<T, E> = Ok(T) | Err(E)`.
    fn sums() -> Vec<SumType> {
        let variant = |name: &str, fields: Vec<Ty>| nova_hir::Variant {
            name: name.to_string(),
            fields,
        };
        vec![
            SumType {
                def_id: OPT,
                name: "Opt".to_string(),
                generics: 1,
                variants: vec![variant("Some", vec![Ty::Param(0)]), variant("None", vec![])],
            },
            SumType {
                def_id: RES,
                name: "Res".to_string(),
                generics: 2,
                variants: vec![
                    variant("Ok", vec![Ty::Param(0)]),
                    variant("Err", vec![Ty::Param(1)]),
                ],
            },
        ]
    }

    fn opt(t: Ty) -> Ty {
        Ty::Sum {
            def_id: OPT,
            args: vec![t],
        }
    }

    fn res(t: Ty, e: Ty) -> Ty {
        Ty::Sum {
            def_id: RES,
            args: vec![t, e],
        }
    }

    fn some(p: Pat) -> Pat {
        Pat::Ctor(Ctor::Variant(OPT, 0), vec![p])
    }

    fn none() -> Pat {
        Pat::Ctor(Ctor::Variant(OPT, 1), vec![])
    }

    fn ok(p: Pat) -> Pat {
        Pat::Ctor(Ctor::Variant(RES, 0), vec![p])
    }

    fn err(p: Pat) -> Pat {
        Pat::Ctor(Ctor::Variant(RES, 1), vec![p])
    }

    fn tuple(ps: Vec<Pat>) -> Pat {
        Pat::Ctor(Ctor::Tuple(ps.len() as u32), ps)
    }

    /// The single witness of `rows` against `ty`, rendered loosely.
    fn witnesses(cx: &MatchCx, rows: Vec<Pat>, ty: Ty) -> Vec<Pat> {
        let matrix: Vec<Vec<Pat>> = rows.into_iter().map(|p| vec![p]).collect();
        cx.usefulness(&matrix, &[Pat::Wild], &[ty])
            .into_iter()
            .map(|mut w| w.remove(0))
            .collect()
    }

    #[test]
    fn a_nested_witness_names_the_inner_variant() {
        let sums = sums();
        let cx = MatchCx::new(&sums);
        let ty = opt(res(Ty::Int, Ty::String));
        let w = witnesses(&cx, vec![some(ok(Pat::Wild)), none()], ty);
        assert_eq!(w.len(), 1, "{w:?}");
        match &w[0] {
            Pat::Ctor(Ctor::Variant(OPT, 0), args) => {
                assert!(
                    matches!(&args[0], Pat::Ctor(Ctor::Variant(RES, 1), _)),
                    "{w:?}"
                )
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn a_nested_arm_an_earlier_one_covers_is_not_useful() {
        let sums = sums();
        let cx = MatchCx::new(&sums);
        let ty = opt(opt(Ty::Int));
        let prior = vec![vec![some(Pat::Wild)]];
        assert!(cx
            .usefulness(
                &prior,
                &[some(some(ctor(Ctor::Int(1))))],
                std::slice::from_ref(&ty)
            )
            .is_empty());
        // First match wins: the more specific arm first leaves the general
        // one useful.
        let prior = vec![vec![some(some(ctor(Ctor::Int(1))))]];
        assert!(!cx.usefulness(&prior, &[some(Pat::Wild)], &[ty]).is_empty());
    }

    #[test]
    fn an_or_pattern_row_covers_every_alternative() {
        let sums = sums();
        let cx = MatchCx::new(&sums);
        let ty = opt(res(Ty::Int, Ty::Int));
        let rows = vec![some(Pat::Or(vec![ok(Pat::Wild), err(Pat::Wild)])), none()];
        assert!(witnesses(&cx, rows, ty.clone()).is_empty());
        // An or-pattern as the candidate row is useful when one
        // alternative is.
        let prior = vec![vec![some(ok(Pat::Wild))]];
        let q = Pat::Or(vec![some(ok(Pat::Wild)), none()]);
        assert!(!cx
            .usefulness(&prior, &[q], std::slice::from_ref(&ty))
            .is_empty());
        let q = Pat::Or(vec![some(ok(ctor(Ctor::Int(3))))]);
        assert!(cx.usefulness(&prior, &[q], &[ty]).is_empty());
    }

    #[test]
    fn a_tuple_of_sums_needs_every_combination() {
        let sums = sums();
        let cx = MatchCx::new(&sums);
        let ty = Ty::Tuple(vec![opt(Ty::Int), Ty::Bool]);
        let rows = vec![
            tuple(vec![some(Pat::Wild), Pat::Wild]),
            tuple(vec![none(), ctor(Ctor::Bool(true))]),
        ];
        let w = witnesses(&cx, rows.clone(), ty.clone());
        assert_eq!(w.len(), 1, "{w:?}");
        match &w[0] {
            Pat::Ctor(Ctor::Tuple(2), args) => {
                assert!(matches!(args[0], Pat::Ctor(Ctor::Variant(OPT, 1), _)));
                assert!(matches!(args[1], Pat::Ctor(Ctor::Bool(false), _)));
            }
            other => panic!("{other:?}"),
        }
        let mut all = rows;
        all.push(tuple(vec![Pat::Wild, ctor(Ctor::Bool(false))]));
        assert!(witnesses(&cx, all, ty).is_empty());
    }

    #[test]
    fn a_tuple_is_one_constructor_even_when_its_type_is_unknown() {
        let sums = sums();
        let cx = MatchCx::new(&sums);
        let rows = vec![tuple(vec![Pat::Wild, Pat::Wild])];
        assert!(witnesses(&cx, rows, Ty::Error).is_empty());
    }

    #[test]
    fn literals_and_ranges_inside_a_variant_need_a_catch_all() {
        let sums = sums();
        let cx = MatchCx::new(&sums);
        let ty = opt(Ty::Char);
        let rows = vec![
            some(ctor(Ctor::Range('a' as i64, 'z' as i64))),
            some(ctor(Ctor::Char('_'))),
            none(),
        ];
        let w = witnesses(&cx, rows.clone(), ty.clone());
        assert_eq!(w.len(), 1, "{w:?}");
        assert!(matches!(&w[0], Pat::Ctor(Ctor::Variant(OPT, 0), a) if matches!(a[0], Pat::Wild)));
        let mut all = rows;
        all.push(some(Pat::Wild));
        assert!(witnesses(&cx, all, ty).is_empty());
    }

    #[test]
    fn ranges_are_not_split() {
        // The same range again is not useful; one inside it still is.
        let prior = vec![vec![ctor(Ctor::Range(1, 10))]];
        assert!(cx()
            .usefulness(&prior, &[ctor(Ctor::Range(1, 10))], &[Ty::Int])
            .is_empty());
        assert!(!cx()
            .usefulness(&prior, &[ctor(Ctor::Range(2, 3))], &[Ty::Int])
            .is_empty());
        assert!(!cx()
            .usefulness(&prior, &[ctor(Ctor::Int(5))], &[Ty::Int])
            .is_empty());
        // A range covering every `Int` still needs a catch-all.
        let all = vec![vec![ctor(Ctor::Range(i64::MIN, i64::MAX))]];
        assert!(!cx().usefulness(&all, &[Pat::Wild], &[Ty::Int]).is_empty());
    }

    #[test]
    fn a_generic_sum_at_depth_takes_its_arguments() {
        let sums = sums();
        let cx = MatchCx::new(&sums);
        // `Opt<Opt<Bool>>`: `Some(Some(true))`, `Some(Some(false))`, `None`
        // leave `Some(None)`.
        let ty = opt(opt(Ty::Bool));
        let rows = vec![
            some(some(ctor(Ctor::Bool(true)))),
            some(some(ctor(Ctor::Bool(false)))),
            none(),
        ];
        let w = witnesses(&cx, rows, ty);
        assert_eq!(w.len(), 1, "{w:?}");
        assert!(matches!(&w[0], Pat::Ctor(Ctor::Variant(OPT, 0), a)
            if matches!(a[0], Pat::Ctor(Ctor::Variant(OPT, 1), _))));
    }
}
