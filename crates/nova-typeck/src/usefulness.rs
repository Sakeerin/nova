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
//! The algorithm is written for arbitrarily nested patterns; Nova's Phase 1
//! pattern language only ever feeds it shallow rows (variant payloads are
//! irrefutable binds/wildcards), but the recursion, specialization and default
//! matrices are the general ones, so the checker stays correct as the pattern
//! language grows.

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
}

/// A pattern in the checker's normalized form: either a wildcard (which also
/// stands in for an irrefutable binding) or a constructor applied to
/// sub-patterns.
#[derive(Clone, Debug)]
pub enum Pat {
    Wild,
    Ctor(Ctor, Vec<Pat>),
}

/// Context for the algorithm: the module's sum-type declarations, used to
/// enumerate a type's constructor signature and their arities.
pub struct MatchCx<'a> {
    sums: &'a [SumType],
}

/// One row of witness patterns (aligned with the current columns).
type Witness = Vec<Pat>;

impl<'a> MatchCx<'a> {
    pub fn new(sums: &'a [SumType]) -> Self {
        Self { sums }
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

        match &q[0] {
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
            Ctor::Bool(_) | Ctor::Int(_) | Ctor::Str(_) => Vec::new(),
        }
    }

    fn arity(&self, c: &Ctor) -> usize {
        match c {
            Ctor::Variant(sum_id, vi) => self
                .variant(*sum_id, *vi)
                .map(|v| v.fields.len())
                .unwrap_or(0),
            Ctor::Bool(_) | Ctor::Int(_) | Ctor::Str(_) => 0,
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
}
