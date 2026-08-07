//! The unification engine: inference variables, occurs check, substitution.

use nova_hir::Ty;

/// Inference context: a table of type variables and their solutions.
///
/// Uses plain substitution (Robinson unification) — variable count per
/// function body is small at this stage, so no union-find is needed yet.
#[derive(Debug, Default)]
pub struct InferCtx {
    vars: Vec<Option<Ty>>,
}

impl InferCtx {
    /// Allocate a fresh, unsolved inference variable.
    pub fn fresh(&mut self) -> Ty {
        let id = self.vars.len() as u32;
        self.vars.push(None);
        Ty::Var(id)
    }

    /// Follow variable bindings one level at a time until reaching a
    /// non-variable type or an unsolved variable.
    pub fn shallow_resolve(&self, ty: &Ty) -> Ty {
        let mut ty = ty.clone();
        while let Ty::Var(v) = ty {
            match &self.vars[v as usize] {
                Some(t) => ty = t.clone(),
                None => return Ty::Var(v),
            }
        }
        ty
    }

    /// Deeply apply the current substitution to a type.
    pub fn apply(&self, ty: &Ty) -> Ty {
        match self.shallow_resolve(ty) {
            Ty::Fn { params, ret } => Ty::Fn {
                params: params.iter().map(|p| self.apply(p)).collect(),
                ret: Box::new(self.apply(&ret)),
            },
            Ty::Sum { def_id, args } => Ty::Sum {
                def_id,
                args: args.iter().map(|a| self.apply(a)).collect(),
            },
            Ty::Record { def_id, args } => Ty::Record {
                def_id,
                args: args.iter().map(|a| self.apply(a)).collect(),
            },
            Ty::Array(elem) => Ty::Array(Box::new(self.apply(&elem))),
            Ty::Future(out) => Ty::Future(Box::new(self.apply(&out))),
            Ty::Assoc { on, assoc } => Ty::Assoc {
                on: Box::new(self.apply(&on)),
                assoc,
            },
            other => other,
        }
    }

    /// Occurs check: does variable `v` appear in `ty` (after resolution)?
    fn occurs(&self, v: u32, ty: &Ty) -> bool {
        match self.shallow_resolve(ty) {
            Ty::Var(w) => v == w,
            Ty::Fn { params, ret } => {
                params.iter().any(|p| self.occurs(v, p)) || self.occurs(v, &ret)
            }
            Ty::Sum { args, .. } | Ty::Record { args, .. } => {
                args.iter().any(|a| self.occurs(v, a))
            }
            Ty::Array(elem) => self.occurs(v, &elem),
            Ty::Future(out) => self.occurs(v, &out),
            Ty::Assoc { on, .. } => self.occurs(v, &on),
            _ => false,
        }
    }

    /// Unify two types. Returns `false` on mismatch (the caller reports the
    /// diagnostic — it has the spans and the name table).
    ///
    /// `Never` unifies with anything (it is the bottom type), and `Error`
    /// unifies with anything to suppress cascading diagnostics.
    pub fn unify(&mut self, a: &Ty, b: &Ty) -> bool {
        let a = self.shallow_resolve(a);
        let b = self.shallow_resolve(b);
        match (&a, &b) {
            (Ty::Var(v), _) => {
                if let Ty::Var(w) = b {
                    if *v == w {
                        return true;
                    }
                }
                if self.occurs(*v, &b) {
                    return false;
                }
                self.vars[*v as usize] = Some(b);
                true
            }
            (_, Ty::Var(w)) => {
                if self.occurs(*w, &a) {
                    return false;
                }
                self.vars[*w as usize] = Some(a);
                true
            }
            (Ty::Error, _) | (_, Ty::Error) => true,
            (Ty::Never, _) | (_, Ty::Never) => true,
            (Ty::Int, Ty::Int)
            | (Ty::Float, Ty::Float)
            | (Ty::Bool, Ty::Bool)
            | (Ty::Char, Ty::Char)
            | (Ty::String, Ty::String)
            | (Ty::Unit, Ty::Unit) => true,
            (Ty::Param(i), Ty::Param(j)) => i == j,
            (
                Ty::Fn {
                    params: p1,
                    ret: r1,
                },
                Ty::Fn {
                    params: p2,
                    ret: r2,
                },
            ) => {
                p1.len() == p2.len()
                    && p1
                        .clone()
                        .iter()
                        .zip(p2.clone().iter())
                        .all(|(x, y)| self.unify(x, y))
                    && self.unify(&r1.clone(), &r2.clone())
            }
            (
                Ty::Sum {
                    def_id: d1,
                    args: a1,
                },
                Ty::Sum {
                    def_id: d2,
                    args: a2,
                },
            )
            | (
                Ty::Record {
                    def_id: d1,
                    args: a1,
                },
                Ty::Record {
                    def_id: d2,
                    args: a2,
                },
            ) => {
                d1 == d2
                    && a1.len() == a2.len()
                    && a1
                        .clone()
                        .iter()
                        .zip(a2.clone().iter())
                        .all(|(x, y)| self.unify(x, y))
            }
            (Ty::Array(e1), Ty::Array(e2)) => self.unify(&e1.clone(), &e2.clone()),
            (Ty::Future(o1), Ty::Future(o2)) => self.unify(&o1.clone(), &o2.clone()),
            (Ty::Assoc { on: o1, assoc: a1 }, Ty::Assoc { on: o2, assoc: a2 }) => {
                a1 == a2 && self.unify(&o1.clone(), &o2.clone())
            }
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unify_var_binds() {
        let mut cx = InferCtx::default();
        let v = cx.fresh();
        assert!(cx.unify(&v, &Ty::Int));
        assert_eq!(cx.apply(&v), Ty::Int);
    }

    #[test]
    fn unify_mismatch_fails() {
        let mut cx = InferCtx::default();
        assert!(!cx.unify(&Ty::Int, &Ty::String));
    }

    #[test]
    fn occurs_check_rejects_infinite_type() {
        let mut cx = InferCtx::default();
        let v = cx.fresh();
        let f = Ty::Fn {
            params: vec![v.clone()],
            ret: Box::new(Ty::Unit),
        };
        assert!(!cx.unify(&v, &f));
    }

    #[test]
    fn never_unifies_with_anything() {
        let mut cx = InferCtx::default();
        assert!(cx.unify(&Ty::Never, &Ty::Int));
        assert!(cx.unify(&Ty::String, &Ty::Never));
    }

    #[test]
    fn fn_types_unify_structurally() {
        let mut cx = InferCtx::default();
        let v = cx.fresh();
        let a = Ty::Fn {
            params: vec![Ty::Int],
            ret: Box::new(v.clone()),
        };
        let b = Ty::Fn {
            params: vec![Ty::Int],
            ret: Box::new(Ty::Bool),
        };
        assert!(cx.unify(&a, &b));
        assert_eq!(cx.apply(&v), Ty::Bool);
    }

    // A projection is opaque to the unifier: two projections match only when
    // they name the same associated type on unifiable Self types. Anything
    // else must already have been normalized away by the caller, which is why
    // there is no Assoc-vs-concrete arm.
    #[test]
    fn assoc_unifies_only_with_the_same_projection() {
        use nova_resolver::DefId;
        let item = DefId(7);
        let other = DefId(8);
        let mut icx = InferCtx::default();
        let a = Ty::Assoc {
            on: Box::new(Ty::Int),
            assoc: item,
        };
        let b = Ty::Assoc {
            on: Box::new(Ty::Int),
            assoc: item,
        };
        assert!(icx.unify(&a, &b), "same projection on same Self");

        let c = Ty::Assoc {
            on: Box::new(Ty::Bool),
            assoc: item,
        };
        assert!(!icx.unify(&a, &c), "same name, different Self");

        let d = Ty::Assoc {
            on: Box::new(Ty::Int),
            assoc: other,
        };
        assert!(!icx.unify(&a, &d), "different associated type");

        assert!(!icx.unify(&a, &Ty::Int), "a projection is not Int");
    }

    // The Self type is an ordinary type position, so a variable inside it must
    // solve through unification like any other.
    #[test]
    fn assoc_unification_solves_a_var_inside_the_self_type() {
        use nova_resolver::DefId;
        let item = DefId(7);
        let mut icx = InferCtx::default();
        let v = icx.fresh();
        let a = Ty::Assoc {
            on: Box::new(v.clone()),
            assoc: item,
        };
        let b = Ty::Assoc {
            on: Box::new(Ty::Int),
            assoc: item,
        };
        assert!(icx.unify(&a, &b));
        assert_eq!(icx.apply(&v), Ty::Int);
    }

    // occurs must see through a projection, or `?0 == ?0::Item` would bind a
    // variable to a type containing itself and `apply` would not terminate.
    #[test]
    fn occurs_looks_inside_a_projection() {
        use nova_resolver::DefId;
        let mut icx = InferCtx::default();
        let v = icx.fresh();
        let proj = Ty::Assoc {
            on: Box::new(v.clone()),
            assoc: DefId(7),
        };
        assert!(
            !icx.unify(&v, &proj),
            "occurs check must reject ?0 = ?0::Item"
        );
    }

    // occurs must not flag a projection whose Self type has nothing to do
    // with the variable — only recursing into `on` and actually finding the
    // variable there should reject the unification. (A mutant that ignores
    // `on` and always reports "found" would make this spuriously fail and
    // reject an otherwise-valid unification.)
    #[test]
    fn occurs_does_not_reject_a_var_unrelated_to_the_projections_self_type() {
        use nova_resolver::DefId;
        let mut icx = InferCtx::default();
        let v = icx.fresh();
        let proj = Ty::Assoc {
            on: Box::new(Ty::Int),
            assoc: DefId(7),
        };
        assert!(
            icx.unify(&v, &proj),
            "v does not occur in `proj`'s Self type, so unification must succeed"
        );
        assert_eq!(icx.apply(&v), proj);
    }

    // `apply` backs `display_ty` (via `show`), so a broken Assoc arm would
    // render a stale unresolved variable inside every diagnostic that
    // mentions a projection. Exercise `apply` on a value that is itself
    // `Ty::Assoc` at the top level, not just on the bare `Var` that resolves
    // to one (as `assoc_unification_solves_a_var_inside_the_self_type` does).
    #[test]
    fn apply_resolves_a_solved_var_inside_a_top_level_projection() {
        use nova_resolver::DefId;
        let mut icx = InferCtx::default();
        let v = icx.fresh();
        assert!(icx.unify(&v, &Ty::Int));
        let proj = Ty::Assoc {
            on: Box::new(v.clone()),
            assoc: DefId(7),
        };
        let resolved = icx.apply(&proj);
        assert_eq!(
            resolved,
            Ty::Assoc {
                on: Box::new(Ty::Int),
                assoc: DefId(7),
            },
            "apply must resolve the solved var inside `on`, not just pass the projection through"
        );
    }
}
