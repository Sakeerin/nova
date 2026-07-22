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
}
