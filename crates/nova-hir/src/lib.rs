//! High-level IR for the Nova compiler.
//!
//! HIR is the *desugared, typed* representation produced by `nova-typeck`
//! from the AST and consumed by `nova-mir`:
//!
//! - every expression carries a [`Ty`] and a `Span`
//! - string interpolation is desugared to [`ExprKind::StrConcat`] over
//!   [`ExprKind::ToStr`] conversions
//! - compound assignment is desugared to a plain assign of a binary op
//! - names are resolved: locals are [`LocalId`], callees are [`Callee`]
//!
//! The [`Ty`] type also serves as the type representation used *during*
//! inference (it can contain [`Ty::Var`] inference variables); a fully
//! checked module contains no `Var` types.

use nova_diagnostics::Span;
use nova_resolver::{Builtin, DefId};

/// A type. This is the shared type representation across typeck, HIR and
/// monomorphization (spec `12-TYPESYSTEM.md` §3, trimmed to the Phase 1
/// feature set).
#[derive(Debug, Clone, PartialEq)]
pub enum Ty {
    /// 64-bit signed integer (the default integer type).
    Int,
    /// 64-bit float (the default float type).
    Float,
    Bool,
    Char,
    String,
    /// The unit type `()`.
    Unit,
    /// A function type `fn(A, B) -> R`.
    Fn {
        params: Vec<Ty>,
        ret: Box<Ty>,
    },
    /// A user-defined sum type with generic arguments.
    Sum {
        def_id: DefId,
        args: Vec<Ty>,
    },
    /// A user-defined record (struct) type with generic arguments.
    Record {
        def_id: DefId,
        args: Vec<Ty>,
    },
    /// A heap array `[T]` with elements of a single type.
    Array(Box<Ty>),
    /// A generic type parameter of the enclosing item (`T`).
    Param(u32),
    /// A projection onto a trait's associated type: `<on>::Name`, where
    /// `assoc` is the associated type's own `DefId` (see
    /// `DefKind::AssocType`).
    ///
    /// This is the one `Ty` variant that is **not** a type by itself — it is a
    /// *request* for one, answerable only with the impl table. It is therefore
    /// normalized away at three seams (typeck's `normalize`,
    /// `check_impl_conformance`, and `mono` after `subst`), and the unifier
    /// never has to decide a projection against a concrete type.
    ///
    /// `on` is `Param(k)` inside a generic body, a concrete type at an
    /// ordinary use site, and — provably — never an unsolved `Var`:
    /// `check_method_call` rejects an uninferred receiver with `E0011` before
    /// any return type is computed, and a user-written `I::Item` names a
    /// generic parameter. See the design doc §4.2.
    Assoc {
        on: Box<Ty>,
        assoc: DefId,
    },
    /// An unsolved inference variable (only during type checking).
    Var(u32),
    /// The never type `!` (return, panic); coerces to anything.
    Never,
    /// Placeholder after a type error, suppresses cascading errors.
    Error,
}

impl Ty {
    /// Substitute `Param(i)` with `args[i]`. Used for generic instantiation
    /// and monomorphization.
    pub fn subst(&self, args: &[Ty]) -> Ty {
        match self {
            Ty::Param(i) => args.get(*i as usize).cloned().unwrap_or(Ty::Error),
            Ty::Fn { params, ret } => Ty::Fn {
                params: params.iter().map(|p| p.subst(args)).collect(),
                ret: Box::new(ret.subst(args)),
            },
            Ty::Sum { def_id, args: a } => Ty::Sum {
                def_id: *def_id,
                args: a.iter().map(|t| t.subst(args)).collect(),
            },
            Ty::Record { def_id, args: a } => Ty::Record {
                def_id: *def_id,
                args: a.iter().map(|t| t.subst(args)).collect(),
            },
            Ty::Array(elem) => Ty::Array(Box::new(elem.subst(args))),
            Ty::Assoc { on, assoc } => Ty::Assoc {
                on: Box::new(on.subst(args)),
                assoc: *assoc,
            },
            other => other.clone(),
        }
    }

    /// Whether this type (transitively) mentions the generic parameter `idx`.
    pub fn mentions_param(&self, idx: u32) -> bool {
        match self {
            Ty::Param(k) => *k == idx,
            Ty::Fn { params, ret } => {
                params.iter().any(|p| p.mentions_param(idx)) || ret.mentions_param(idx)
            }
            Ty::Sum { args, .. } | Ty::Record { args, .. } => {
                args.iter().any(|a| a.mentions_param(idx))
            }
            Ty::Array(elem) => elem.mentions_param(idx),
            Ty::Assoc { on, .. } => on.mentions_param(idx),
            _ => false,
        }
    }

    /// Whether this type (transitively) contains any generic `Param`.
    pub fn has_params(&self) -> bool {
        match self {
            Ty::Param(_) => true,
            Ty::Fn { params, ret } => params.iter().any(Ty::has_params) || ret.has_params(),
            Ty::Sum { args, .. } | Ty::Record { args, .. } => args.iter().any(Ty::has_params),
            Ty::Array(elem) => elem.has_params(),
            Ty::Assoc { on, .. } => on.has_params(),
            _ => false,
        }
    }

    /// Whether this type (transitively) contains any inference variable.
    pub fn has_vars(&self) -> bool {
        match self {
            Ty::Var(_) => true,
            Ty::Fn { params, ret } => params.iter().any(Ty::has_vars) || ret.has_vars(),
            Ty::Sum { args, .. } | Ty::Record { args, .. } => args.iter().any(Ty::has_vars),
            Ty::Array(elem) => elem.has_vars(),
            Ty::Assoc { on, .. } => on.has_vars(),
            _ => false,
        }
    }

    /// The nominal head of this type, if it has one — used to key impl
    /// lookups. Generic arguments do not participate (Phase 1 has no
    /// overlapping impls per head).
    pub fn head(&self) -> Option<TyHead> {
        match self {
            Ty::Int => Some(TyHead::Int),
            Ty::Float => Some(TyHead::Float),
            Ty::Bool => Some(TyHead::Bool),
            Ty::Char => Some(TyHead::Char),
            Ty::String => Some(TyHead::String),
            Ty::Sum { def_id, .. } => Some(TyHead::Sum(*def_id)),
            Ty::Record { def_id, .. } => Some(TyHead::Record(*def_id)),
            _ => None,
        }
    }

    /// One-directional match: treat `self` as a pattern that may contain
    /// `Param(k)` and match it against the concrete `ground` type, recording
    /// each `Param(k)` → ground binding into `out` (grown as needed). A
    /// parameter seen twice must bind to the same type. Returns `false` on a
    /// structural mismatch. Used to recover a generic impl's type arguments
    /// from a concrete receiver type (e.g. `Box<T>` vs `Box<Int>` → `T=Int`).
    pub fn match_pattern(&self, ground: &Ty, out: &mut Vec<Option<Ty>>) -> bool {
        match (self, ground) {
            (Ty::Param(k), g) => {
                let k = *k as usize;
                if out.len() <= k {
                    out.resize(k + 1, None);
                }
                match &out[k] {
                    Some(existing) => existing == g,
                    None => {
                        out[k] = Some(g.clone());
                        true
                    }
                }
            }
            (
                Ty::Record {
                    def_id: d1,
                    args: a1,
                },
                Ty::Record {
                    def_id: d2,
                    args: a2,
                },
            )
            | (
                Ty::Sum {
                    def_id: d1,
                    args: a1,
                },
                Ty::Sum {
                    def_id: d2,
                    args: a2,
                },
            ) => {
                d1 == d2
                    && a1.len() == a2.len()
                    && a1.iter().zip(a2).all(|(p, g)| p.match_pattern(g, out))
            }
            (Ty::Array(e1), Ty::Array(e2)) => e1.match_pattern(e2, out),
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
                    && p1.iter().zip(p2).all(|(a, b)| a.match_pattern(b, out))
                    && r1.match_pattern(r2, out)
            }
            (Ty::Assoc { on: p, assoc: pa }, Ty::Assoc { on: g, assoc: ga }) => {
                pa == ga && p.match_pattern(g, out)
            }
            // Primitives, `Unit`, etc.: match iff identical.
            (a, b) => a == b,
        }
    }
}

/// The nominal head of a type, used as the key for impl lookups
/// (spec `12-TYPESYSTEM.md` §5.2, `ImplTable` indexed by trait + type head).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TyHead {
    Int,
    Float,
    Bool,
    Char,
    String,
    Sum(DefId),
    Record(DefId),
}

/// A local variable slot within a function (parameters come first).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LocalId(pub u32);

/// Metadata for one local variable.
#[derive(Debug, Clone)]
pub struct Local {
    pub name: String,
    pub ty: Ty,
    pub is_mut: bool,
    pub span: Span,
}

/// An `extern` function declaration: a C-ABI import with no Nova body. Only its
/// signature and (unmangled) C symbol name are needed to declare and call it.
#[derive(Debug, Clone)]
pub struct ExternFn {
    pub def_id: DefId,
    /// The raw C symbol to import and call (never mangled).
    pub symbol: String,
    pub params: Vec<Ty>,
    pub ret: Ty,
    /// Declaration site, for conflicting-signature diagnostics.
    pub span: Span,
}

/// A fully checked module: type layouts plus typed functions.
#[derive(Debug, Default)]
pub struct Module {
    pub sums: Vec<SumType>,
    pub records: Vec<RecordType>,
    pub traits: Vec<TraitDef>,
    pub impls: Vec<ImplInfo>,
    pub functions: Vec<Function>,
    /// `extern` (FFI) function declarations reachable in the program.
    pub externs: Vec<ExternFn>,
}

impl Module {
    /// Find the extern function declared under `def_id`, if any.
    pub fn extern_fn(&self, def_id: DefId) -> Option<&ExternFn> {
        self.externs.iter().find(|e| e.def_id == def_id)
    }

    /// Find the sum type declared under `def_id`, if any.
    pub fn sum(&self, def_id: DefId) -> Option<&SumType> {
        self.sums.iter().find(|s| s.def_id == def_id)
    }

    /// Find the record type declared under `def_id`, if any.
    pub fn record(&self, def_id: DefId) -> Option<&RecordType> {
        self.records.iter().find(|r| r.def_id == def_id)
    }

    /// Find the trait declared under `def_id`, if any.
    pub fn trait_def(&self, def_id: DefId) -> Option<&TraitDef> {
        self.traits.iter().find(|t| t.def_id == def_id)
    }

    /// Find the function declared under `def_id`, if any.
    pub fn function(&self, def_id: DefId) -> Option<&Function> {
        self.functions.iter().find(|f| f.def_id == def_id)
    }

    /// Resolve trait method `method` (index into the trait's methods) for a
    /// concrete self type to the compiled function it should dispatch to,
    /// paired with the type arguments that function must be instantiated with:
    ///
    /// - if the matching impl provides the method, dispatch to it, instantiated
    ///   with the impl's own type arguments recovered from `self_ty`
    ///   (`impl<T> Show for Box<T>` called on `Box<Int>` → `[Int]`);
    /// - otherwise dispatch to the trait's default body, which is generic over
    ///   `Self` → `[self_ty]`.
    ///
    /// Returns `None` if no impl of the trait fits `self_ty`.
    pub fn resolve_method_full(
        &self,
        trait_id: DefId,
        method: u32,
        self_ty: &Ty,
    ) -> Option<(DefId, Vec<Ty>)> {
        let tr = self.trait_def(trait_id)?;
        let method_name = &tr.methods.get(method as usize)?.name;
        let head = self_ty.head()?;
        // Among all impls of this trait for the head, select the one whose
        // self-type pattern actually fits — not merely the first sharing the
        // head. Coherence (no overlapping impls) guarantees at most one fits.
        let (imp, impl_args) = self
            .impls
            .iter()
            .filter(|i| i.trait_id == Some(trait_id) && i.self_head == head)
            .find_map(|i| i.match_args(self_ty).map(|args| (i, args)))?;
        if let Some((_, def)) = imp.methods.iter().find(|(n, _)| n == method_name) {
            Some((*def, impl_args))
        } else {
            let def = tr.methods[method as usize].default_def?;
            Some((def, vec![self_ty.clone()]))
        }
    }
}

/// Whether two impl self-type patterns share a common ground instance — i.e.
/// the impls overlap and one concrete type could match both. `a` uses generic
/// parameters `0..a_generics`; `b`'s parameters are an independent namespace
/// `0..b_generics`. Exact: first-order unification with an occurs check, so
/// genuinely disjoint patterns (`Pair<Int, Bool>` vs `Pair<Int, String>`) are
/// never reported as overlapping, while a generic and a specific pattern
/// (`Box<T>` vs `Box<Int>`) are.
pub fn self_types_overlap(a: &Ty, a_generics: u32, b: &Ty, b_generics: u32) -> bool {
    // A projection's value is unknown until normalized, so assume it could
    // coincide with anything unless both sides are projections that provably
    // differ. A false E0074 is a loud error; a missed overlap is a silent
    // miscompile, so err toward overlap.
    match (a, b) {
        (Ty::Assoc { on: a1, assoc: x }, Ty::Assoc { on: b1, assoc: y }) => {
            x != y || self_types_overlap(a1, a_generics, b1, b_generics)
        }
        (Ty::Assoc { .. }, _) | (_, Ty::Assoc { .. }) => true,
        _ => {
            // Shift `b`'s parameters into a disjoint range so the two namespaces
            // do not collide in the shared substitution.
            let b = shift_params(b, a_generics);
            let mut subst: Vec<Option<Ty>> = vec![None; (a_generics + b_generics) as usize];
            unify_patterns(a, &b, &mut subst)
        }
    }
}

fn shift_params(t: &Ty, by: u32) -> Ty {
    match t {
        Ty::Param(k) => Ty::Param(k + by),
        Ty::Fn { params, ret } => Ty::Fn {
            params: params.iter().map(|p| shift_params(p, by)).collect(),
            ret: Box::new(shift_params(ret, by)),
        },
        Ty::Sum { def_id, args } => Ty::Sum {
            def_id: *def_id,
            args: args.iter().map(|a| shift_params(a, by)).collect(),
        },
        Ty::Record { def_id, args } => Ty::Record {
            def_id: *def_id,
            args: args.iter().map(|a| shift_params(a, by)).collect(),
        },
        Ty::Array(elem) => Ty::Array(Box::new(shift_params(elem, by))),
        other => other.clone(),
    }
}

/// Resolve `t` through the substitution one level (following a bound `Param`).
fn walk_param(t: &Ty, subst: &[Option<Ty>]) -> Ty {
    match t {
        Ty::Param(k) => match subst.get(*k as usize).and_then(|o| o.as_ref()) {
            Some(bound) => walk_param(bound, subst),
            None => t.clone(),
        },
        _ => t.clone(),
    }
}

fn occurs(k: u32, t: &Ty, subst: &[Option<Ty>]) -> bool {
    match walk_param(t, subst) {
        Ty::Param(j) => j == k,
        Ty::Fn { params, ret } => {
            params.iter().any(|p| occurs(k, p, subst)) || occurs(k, &ret, subst)
        }
        Ty::Sum { args, .. } | Ty::Record { args, .. } => args.iter().any(|a| occurs(k, a, subst)),
        Ty::Array(elem) => occurs(k, &elem, subst),
        _ => false,
    }
}

fn unify_patterns(a: &Ty, b: &Ty, subst: &mut Vec<Option<Ty>>) -> bool {
    let a = walk_param(a, subst);
    let b = walk_param(b, subst);
    match (&a, &b) {
        (Ty::Param(i), Ty::Param(j)) if i == j => true,
        (Ty::Param(i), _) => {
            if occurs(*i, &b, subst) {
                return false;
            }
            subst[*i as usize] = Some(b.clone());
            true
        }
        (_, Ty::Param(j)) => {
            if occurs(*j, &a, subst) {
                return false;
            }
            subst[*j as usize] = Some(a.clone());
            true
        }
        (
            Ty::Record {
                def_id: d1,
                args: a1,
            },
            Ty::Record {
                def_id: d2,
                args: a2,
            },
        )
        | (
            Ty::Sum {
                def_id: d1,
                args: a1,
            },
            Ty::Sum {
                def_id: d2,
                args: a2,
            },
        ) => {
            d1 == d2
                && a1.len() == a2.len()
                && a1.iter().zip(a2).all(|(x, y)| unify_patterns(x, y, subst))
        }
        (Ty::Array(e1), Ty::Array(e2)) => unify_patterns(e1, e2, subst),
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
                && p1.iter().zip(p2).all(|(x, y)| unify_patterns(x, y, subst))
                && unify_patterns(r1, r2, subst)
        }
        // Primitives, `Unit`, `Error`: overlap iff identical.
        (x, y) => x == y,
    }
}

/// A trait declaration with its method signatures.
#[derive(Debug, Clone)]
pub struct TraitDef {
    pub def_id: DefId,
    pub name: String,
    /// Traits this one requires (`trait Ord: Eq` → `[Eq]`), resolved and
    /// deduplicated. Direct supertraits only — the transitive closure is taken
    /// where it is needed, so a cyclic declaration cannot make this list
    /// infinite.
    ///
    /// The graph formed by these lists **can itself be cyclic**: `trait A: B`
    /// together with `trait B: A` is accepted (`nova-typeck` only ever
    /// requires satisfiability, not acyclicity). A transitive walk over this
    /// field — chasing `supertraits` into each named trait's own
    /// `supertraits`, and so on — must therefore track which traits it has
    /// already visited, or it will loop forever on such a declaration. (The
    /// existing transitive expansion in `nova-typeck` does this by construction:
    /// it only ever appends an id it has not already recorded.)
    pub supertraits: Vec<DefId>,
    pub methods: Vec<TraitMethod>,
    /// Associated types this trait declares, in declaration order, each with
    /// its own `DefId` (`DefKind::AssocType`). An impl must bind every one of
    /// them; `check_impl_conformance` enforces that.
    pub assoc_types: Vec<(String, DefId)>,
}

/// One method of a trait. In `params`/`ret`, `Ty::Param(0)` denotes `Self` and
/// `Ty::Param(1..=generics)` the method's own generic parameters (a generic
/// method like `fn map<U>(self, …)`). `params` never itself contains a
/// `self` entry: when the method declares a `self` receiver it is implicit
/// rather than stored here, but a trait method need not declare one at all
/// (e.g. `fn zero() -> Int`), in which case `params` is just its declared
/// parameters.
#[derive(Debug, Clone)]
pub struct TraitMethod {
    pub name: String,
    pub params: Vec<Ty>,
    pub ret: Ty,
    /// Whether the method declares a `self` receiver. A method without one is
    /// an associated function, called as `Type::name(…)` with no receiver —
    /// never as `value.name(…)`. `params` looks the same either way (`self` is
    /// never stored in it), so this flag is the only record of the difference,
    /// and dispatch that ignores it lowers a receiver argument into a callee
    /// that has no slot for it.
    pub has_self: bool,
    /// Number of the method's own generic parameters (not counting `Self`).
    pub generics: u32,
    /// Trait bounds per method generic parameter (indexed `0..generics`).
    pub bounds: Vec<Vec<DefId>>,
    /// The compiled default-body function, if the trait provides a default.
    pub default_def: Option<DefId>,
}

/// One `impl` block: an inherent impl (`trait_id: None`) or a trait impl.
#[derive(Debug, Clone)]
pub struct ImplInfo {
    pub trait_id: Option<DefId>,
    /// The nominal head of the self type — the impl-table lookup key.
    pub self_head: TyHead,
    /// The impl's self type, with `Param(k)` standing for the impl's k-th
    /// generic parameter (e.g. `Box<Param(0)>` for `impl<T> … for Box<T>`).
    /// A non-generic impl's self type contains no `Param`.
    pub self_ty: Ty,
    /// Number of generic parameters the impl introduces.
    pub generics: u32,
    /// Trait bounds on each impl generic parameter (`impl<T: Bound>`), indexed
    /// by parameter position — empty for an unconstrained or non-generic impl.
    pub bounds: Vec<Vec<DefId>>,
    /// `(method name, compiled method function DefId)` for methods this
    /// impl defines directly.
    pub methods: Vec<(String, DefId)>,
    /// Associated types this impl binds, keyed by the associated type's own
    /// `DefId` (`DefKind::AssocType`, the same id `TraitDef::assoc_types`
    /// carries). The bound type may contain the impl's `Param(k)`, so
    /// normalization must substitute the impl's arguments before using it.
    ///
    /// Empty for an inherent impl: only a trait declares associated types, so
    /// only a trait impl can bind one.
    pub assoc_bindings: Vec<(DefId, Ty)>,
}

impl ImplInfo {
    /// Recover this impl's type arguments (in `Param` order) from a concrete
    /// self type. Returns `None` when the self type does not actually match the
    /// impl's self-type pattern — sharing a head is not enough, so an impl with
    /// a repeated (`Pair<T, T>`) or partially-concrete (`Pair<Int, T>`) self
    /// type only applies to receivers that genuinely fit it. A parameter the
    /// pattern does not mention resolves to `Ty::Error` (an unconstrained,
    /// unused impl generic, which never reaches code).
    pub fn match_args(&self, concrete: &Ty) -> Option<Vec<Ty>> {
        let mut out: Vec<Option<Ty>> = Vec::new();
        if !self.self_ty.match_pattern(concrete, &mut out) {
            return None;
        }
        Some(
            (0..self.generics as usize)
                .map(|i| out.get(i).cloned().flatten().unwrap_or(Ty::Error))
                .collect(),
        )
    }
}

/// A record (struct) declaration with typed fields.
#[derive(Debug, Clone)]
pub struct RecordType {
    pub def_id: DefId,
    pub name: String,
    /// Number of generic parameters.
    pub generics: u32,
    pub fields: Vec<RecordField>,
}

/// One field of a record; the type may reference `Ty::Param`.
#[derive(Debug, Clone)]
pub struct RecordField {
    pub name: String,
    pub ty: Ty,
}

/// A sum type declaration with typed variant payloads.
#[derive(Debug, Clone)]
pub struct SumType {
    pub def_id: DefId,
    pub name: String,
    /// Number of generic parameters.
    pub generics: u32,
    pub variants: Vec<Variant>,
}

/// One variant of a sum type; field types may reference `Ty::Param`.
#[derive(Debug, Clone)]
pub struct Variant {
    pub name: String,
    pub fields: Vec<Ty>,
}

/// A typed function.
#[derive(Debug)]
pub struct Function {
    pub def_id: DefId,
    pub name: String,
    /// Number of generic parameters; generic functions are monomorphized
    /// before codegen.
    pub generics: u32,
    /// Trait bounds per generic parameter (`bounds[i]` are the trait
    /// `DefId`s the i-th parameter must satisfy). Checked at
    /// monomorphization once the concrete type argument is known.
    pub bounds: Vec<Vec<DefId>>,
    /// Whether this function is the code of a function value (a closure or
    /// a bare-fn wrapper). Such functions take a leading environment
    /// pointer in their ABI: `(env_ptr, params...)`. Normal, directly
    /// called functions do not.
    pub takes_env: bool,
    /// For a closure, the number of leading `locals` that are captured
    /// variables loaded from the environment at entry (0 for a bare-fn
    /// wrapper). The captured locals precede the real parameters.
    pub capture_count: u32,
    /// The first `params` entries of `locals` are the real parameters
    /// (after any captures). For a closure the captured locals occupy
    /// indices `0..capture_count` and the parameters `capture_count..`.
    pub params: u32,
    pub locals: Vec<Local>,
    pub ret_ty: Ty,
    pub body: Expr,
    pub span: Span,
}

/// A typed expression.
#[derive(Debug, Clone)]
pub struct Expr {
    pub kind: ExprKind,
    pub ty: Ty,
    pub span: Span,
}

/// What a call dispatches to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Callee {
    /// A top-level function.
    Def(DefId),
    /// A compiler builtin.
    Builtin(Builtin),
    /// An indirect call through a fn-typed local.
    Local(LocalId),
}

/// Binary operators after type checking (operand types are known).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
}

/// Unary operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnOp {
    /// Numeric negation.
    Neg,
    /// Boolean not.
    Not,
    /// Bitwise not (Int).
    BitNot,
}

/// Expression kinds. Desugared relative to the AST: no string interpolation,
/// no compound assignment, no `for` (Phase 1 defers iterators).
#[derive(Debug, Clone)]
pub enum ExprKind {
    IntLit(i64),
    FloatLit(f64),
    BoolLit(bool),
    StrLit(String),
    CharLit(char),
    /// The unit value `()`.
    Unit,
    /// Read a local variable.
    Local(LocalId),
    /// Build a function value (fat pointer `{ code, env }`). `func` is the
    /// code function (a closure body or a bare-fn wrapper), instantiated
    /// with `type_args`; `captures` are the environment's values in order
    /// (empty for a bare-fn wrapper).
    MakeClosure {
        func: DefId,
        type_args: Vec<Ty>,
        captures: Vec<Expr>,
    },
    /// Call a function. `type_args` instantiate the callee's generics when
    /// `func` is `Callee::Def`.
    Call {
        func: Callee,
        type_args: Vec<Ty>,
        args: Vec<Expr>,
    },
    /// Construct a sum type variant. The expression's `ty` carries the
    /// concrete `Sum { args, .. }` instantiation.
    MakeVariant {
        sum: DefId,
        variant: u32,
        args: Vec<Expr>,
    },
    /// Construct a record value. `fields` are in the record's declared
    /// field order; the expression's `ty` is the `Record { args, .. }`
    /// instantiation.
    MakeRecord {
        record: DefId,
        fields: Vec<Expr>,
    },
    /// Read field `index` of a record value.
    FieldGet {
        target: Box<Expr>,
        index: u32,
    },
    /// Store `record.field = value`. `index` is the field's position, so the
    /// store offset is `8 * index` — the same layout `FieldGet` reads.
    /// Unit-typed.
    FieldSet {
        target: Box<Expr>,
        index: u32,
        value: Box<Expr>,
    },
    /// Construct a heap array `{ len, elems... }`; the expression's `ty`
    /// is the `Array(elem)` type.
    MakeArray {
        elems: Vec<Expr>,
    },
    /// `[init; len]` — a heap array of `len` slots, every one holding `init`,
    /// where `len` is a runtime `Int`. Unlike `MakeArray` the element count is
    /// not known statically, so MIR lowering allocates and then fills with a
    /// loop. The expression's `ty` is the `Array(elem)` type.
    ///
    /// `init` is evaluated **once**, and that single value is stored into every
    /// slot — these are not `len` copies. For a heap element type the slots are
    /// therefore one object rather than `len` objects: `[Vec::new(); 2]` is a
    /// single `Vec` in both slots, so a `push` through one is visible through
    /// the other. That follows from Nova's reference semantics (there is no
    /// `Copy` and no clone to insert per slot), and
    /// `tests/runtime/array_repeat.nova` executes the case so a change to
    /// per-slot evaluation cannot happen silently.
    ArrayRepeat {
        init: Box<Expr>,
        len: Box<Expr>,
    },
    /// Read `target[index]` (bounds-checked). `target` is an array.
    Index {
        target: Box<Expr>,
        index: Box<Expr>,
    },
    /// Assign `target[index] = value` (bounds-checked); unit-typed.
    IndexSet {
        target: Box<Expr>,
        index: Box<Expr>,
        value: Box<Expr>,
    },
    /// The length of an array (`arr.len()`).
    ArrayLen {
        target: Box<Expr>,
    },
    /// A trait method call, resolved to a concrete impl during monomorphization
    /// (static dispatch). `self_ty` is the `Self` type — concrete for a call on
    /// a known type, or `Param(k)` inside a generic function (dispatch through
    /// that parameter's bound) until substitution makes it concrete.
    TraitCall {
        trait_id: DefId,
        method: u32,
        self_ty: Ty,
        /// The method's own generic arguments (`Param(1..)`), inferred per call
        /// site; empty for a non-generic trait method.
        type_args: Vec<Ty>,
        /// The receiver of `receiver.method(args)`, or `None` for a trait
        /// associated function called as `Type::method(args)` — one that
        /// declares no `self` (see [`TraitMethod::has_self`]). `None` rather
        /// than a flag beside a placeholder expression so that lowering a
        /// receiver that does not exist is not representable: there is nothing
        /// to lower, so no consumer can accidentally pass one.
        receiver: Option<Box<Expr>>,
        args: Vec<Expr>,
    },
    /// A non-short-circuit binary operation.
    Binary {
        op: BinOp,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
    },
    /// Short-circuit `&&`.
    LogicalAnd {
        lhs: Box<Expr>,
        rhs: Box<Expr>,
    },
    /// Short-circuit `||`.
    LogicalOr {
        lhs: Box<Expr>,
        rhs: Box<Expr>,
    },
    Unary {
        op: UnOp,
        expr: Box<Expr>,
    },
    /// `let` binding as a unit-typed statement expression.
    Let {
        local: LocalId,
        init: Box<Expr>,
    },
    /// Assignment to a mutable local; unit-typed.
    Assign {
        local: LocalId,
        value: Box<Expr>,
    },
    /// `{ stmts; trailing }` — value is `trailing` or unit.
    Block {
        stmts: Vec<Expr>,
        trailing: Option<Box<Expr>>,
    },
    If {
        cond: Box<Expr>,
        then: Box<Expr>,
        else_: Option<Box<Expr>>,
    },
    While {
        cond: Box<Expr>,
        body: Box<Expr>,
    },
    /// Exit the innermost enclosing loop; diverging (`Never`).
    Break,
    /// Skip to the next iteration of the innermost loop; diverging.
    Continue,
    Match {
        scrutinee: Box<Expr>,
        arms: Vec<Arm>,
    },
    Return(Option<Box<Expr>>),
    /// Convert a primitive to `String` (from string interpolation).
    ToStr(Box<Expr>),
    /// Concatenate `String` parts (from string interpolation).
    StrConcat(Vec<Expr>),
}

/// One `match` arm.
#[derive(Debug, Clone)]
pub struct Arm {
    pub pattern: Pattern,
    pub body: Expr,
    pub span: Span,
}

/// Patterns after checking. Phase 1 supports flat patterns: variant
/// destructuring binds directly to locals (or discards with `None`).
#[derive(Debug, Clone)]
pub enum Pattern {
    /// `_`
    Wildcard,
    /// A catch-all binding: `n => ...`
    Bind(LocalId),
    LitInt(i64),
    LitBool(bool),
    LitStr(String),
    /// `Variant(a, _, c)` — one binder slot per payload field.
    Variant {
        sum: DefId,
        variant: u32,
        binders: Vec<Option<LocalId>>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subst_recurses_into_a_projection_without_normalizing() {
        use nova_resolver::DefId;
        let proj = Ty::Assoc {
            on: Box::new(Ty::Param(0)),
            assoc: DefId(7),
        };
        // subst has no impl table, so it substitutes and stops. Normalizing is
        // the caller's job (typeck's `normalize`, or mono after subst).
        assert_eq!(
            proj.subst(&[Ty::Int]),
            Ty::Assoc {
                on: Box::new(Ty::Int),
                assoc: DefId(7)
            }
        );
    }

    #[test]
    fn a_projection_has_no_head_and_no_param_of_its_own() {
        use nova_resolver::DefId;
        let proj = Ty::Assoc {
            on: Box::new(Ty::Param(2)),
            assoc: DefId(7),
        };
        // No head: impl lookup cannot key on an unnormalized projection.
        assert!(proj.head().is_none());
        // But it does mention the parameter in its Self type, which
        // `E0073`'s unused-impl-parameter check depends on.
        assert!(proj.mentions_param(2));
        assert!(!proj.mentions_param(0));
    }

    #[test]
    fn a_projection_reports_the_params_and_vars_inside_its_self_type() {
        use nova_resolver::DefId;
        let on_param = Ty::Assoc {
            on: Box::new(Ty::Param(0)),
            assoc: DefId(7),
        };
        assert!(on_param.has_params(), "a projection on a Param is generic");
        assert!(!on_param.has_vars());

        let on_var = Ty::Assoc {
            on: Box::new(Ty::Var(3)),
            assoc: DefId(7),
        };
        // Assoc-on-Var is meant to be unreachable (E0011 fires first), but
        // typeck's finalize checks are the net for that argument being wrong,
        // and they are driven by has_vars.
        assert!(
            on_var.has_vars(),
            "a projection on a Var still contains a var"
        );
        assert!(!on_var.has_params());

        let concrete = Ty::Assoc {
            on: Box::new(Ty::Int),
            assoc: DefId(7),
        };
        assert!(!concrete.has_params());
        assert!(!concrete.has_vars());
    }

    // `self_types_overlap`'s two new arms, mutation-tested: the reviewer
    // found that mutating this function to unconditional `false` (the
    // silent-miscompile direction the design doc warns about) left the full
    // workspace suite green before this test existed.
    #[test]
    fn self_types_overlap_treats_a_projection_conservatively() {
        use nova_resolver::DefId;
        let item = DefId(7);
        let other = DefId(8);

        // Same associated type, Self types that provably differ: no overlap.
        let on_int = Ty::Assoc {
            on: Box::new(Ty::Int),
            assoc: item,
        };
        let on_bool = Ty::Assoc {
            on: Box::new(Ty::Bool),
            assoc: item,
        };
        assert!(
            !self_types_overlap(&on_int, 0, &on_bool, 0),
            "same associated type, but Int and Bool can never be the same Self type"
        );

        // Same associated type, Self types that could unify (a generic
        // parameter against a concrete type): overlap.
        let on_param = Ty::Assoc {
            on: Box::new(Ty::Param(0)),
            assoc: item,
        };
        assert!(
            self_types_overlap(&on_param, 1, &on_int, 0),
            "Param(0) can be instantiated to Int, so these can coincide"
        );

        // Different associated types: nothing is known about either, so the
        // conservative answer is still overlap.
        let other_assoc = Ty::Assoc {
            on: Box::new(Ty::Int),
            assoc: other,
        };
        assert!(
            self_types_overlap(&on_int, 0, &other_assoc, 0),
            "different associated types cannot be proven disjoint, so assume overlap"
        );

        // A projection against a non-projection: always assumed to overlap,
        // in both argument orders.
        assert!(self_types_overlap(&on_int, 0, &Ty::Int, 0));
        assert!(self_types_overlap(&Ty::Bool, 0, &on_int, 0));
    }

    // `match_pattern`'s new arm, mutation-tested: the reviewer found that
    // mutating it to unconditional `false` also left the full workspace
    // suite green before this test existed.
    #[test]
    fn match_pattern_treats_assoc_structurally() {
        use nova_resolver::DefId;
        let item = DefId(7);
        let other = DefId(8);

        // Same associated type: the Self-type pattern matches structurally
        // and records its Param binding like any other position.
        let pattern = Ty::Assoc {
            on: Box::new(Ty::Param(0)),
            assoc: item,
        };
        let ground = Ty::Assoc {
            on: Box::new(Ty::Int),
            assoc: item,
        };
        let mut out = Vec::new();
        assert!(pattern.match_pattern(&ground, &mut out));
        assert_eq!(out, vec![Some(Ty::Int)]);

        // Different associated types: never a match, even with identical
        // Self types.
        let different_assoc = Ty::Assoc {
            on: Box::new(Ty::Int),
            assoc: other,
        };
        let same_self = Ty::Assoc {
            on: Box::new(Ty::Int),
            assoc: item,
        };
        assert!(!same_self.match_pattern(&different_assoc, &mut Vec::new()));

        // A projection on only one side is a structural mismatch, not a
        // partial match, in both argument orders.
        assert!(!pattern.match_pattern(&Ty::Int, &mut Vec::new()));
        assert!(!Ty::Int.match_pattern(&ground, &mut Vec::new()));
    }
}
