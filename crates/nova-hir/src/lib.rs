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
    Fn { params: Vec<Ty>, ret: Box<Ty> },
    /// A user-defined sum type with generic arguments.
    Sum { def_id: DefId, args: Vec<Ty> },
    /// A generic type parameter of the enclosing item (`T`).
    Param(u32),
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
            Ty::Param(i) => args
                .get(*i as usize)
                .cloned()
                .unwrap_or(Ty::Error),
            Ty::Fn { params, ret } => Ty::Fn {
                params: params.iter().map(|p| p.subst(args)).collect(),
                ret: Box::new(ret.subst(args)),
            },
            Ty::Sum { def_id, args: a } => Ty::Sum {
                def_id: *def_id,
                args: a.iter().map(|t| t.subst(args)).collect(),
            },
            other => other.clone(),
        }
    }

    /// Whether this type (transitively) contains any generic `Param`.
    pub fn has_params(&self) -> bool {
        match self {
            Ty::Param(_) => true,
            Ty::Fn { params, ret } => params.iter().any(Ty::has_params) || ret.has_params(),
            Ty::Sum { args, .. } => args.iter().any(Ty::has_params),
            _ => false,
        }
    }

    /// Whether this type (transitively) contains any inference variable.
    pub fn has_vars(&self) -> bool {
        match self {
            Ty::Var(_) => true,
            Ty::Fn { params, ret } => params.iter().any(Ty::has_vars) || ret.has_vars(),
            Ty::Sum { args, .. } => args.iter().any(Ty::has_vars),
            _ => false,
        }
    }
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

/// A fully checked module: sum type layouts plus typed functions.
#[derive(Debug, Default)]
pub struct Module {
    pub sums: Vec<SumType>,
    pub functions: Vec<Function>,
}

impl Module {
    /// Find the sum type declared under `def_id`, if any.
    pub fn sum(&self, def_id: DefId) -> Option<&SumType> {
        self.sums.iter().find(|s| s.def_id == def_id)
    }

    /// Find the function declared under `def_id`, if any.
    pub fn function(&self, def_id: DefId) -> Option<&Function> {
        self.functions.iter().find(|f| f.def_id == def_id)
    }
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
    /// The first `params` entries of `locals` are the parameters.
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
    /// Reference a top-level function as a value (e.g. passed as callback).
    /// `type_args` instantiate the callee's generics.
    FnRef { def: DefId, type_args: Vec<Ty> },
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
    /// A non-short-circuit binary operation.
    Binary {
        op: BinOp,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
    },
    /// Short-circuit `&&`.
    LogicalAnd { lhs: Box<Expr>, rhs: Box<Expr> },
    /// Short-circuit `||`.
    LogicalOr { lhs: Box<Expr>, rhs: Box<Expr> },
    Unary {
        op: UnOp,
        expr: Box<Expr>,
    },
    /// `let` binding as a unit-typed statement expression.
    Let { local: LocalId, init: Box<Expr> },
    /// Assignment to a mutable local; unit-typed.
    Assign { local: LocalId, value: Box<Expr> },
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
    While { cond: Box<Expr>, body: Box<Expr> },
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
