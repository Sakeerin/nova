//! Expression AST nodes.

use nova_diagnostics::Spanned;

use crate::{item::Param, pattern::Pattern, ty::Type, Block, Path};

/// An expression.
#[derive(Debug, Clone)]
pub enum Expr {
    Lit(Literal),
    Path(Path),
    Tuple(Vec<Spanned<Expr>>),
    Array(Vec<Spanned<Expr>>),
    Block(Block),

    If {
        cond: Box<Spanned<Expr>>,
        then: Box<Spanned<Block>>,
        else_: Option<Box<Spanned<Expr>>>,
    },

    Match {
        scrutinee: Box<Spanned<Expr>>,
        arms: Vec<MatchArm>,
    },

    While {
        cond: Box<Spanned<Expr>>,
        body: Box<Spanned<Block>>,
    },

    For {
        pattern: Spanned<Pattern>,
        iter: Box<Spanned<Expr>>,
        body: Box<Spanned<Block>>,
    },

    Return(Option<Box<Spanned<Expr>>>),
    Break(Option<Box<Spanned<Expr>>>),
    Continue,

    Closure {
        params: Vec<Param>,
        ret: Option<Spanned<Type>>,
        body: Box<Spanned<Expr>>,
    },

    Record {
        path: Path,
        fields: Vec<FieldInit>,
        /// `..base` spread
        base: Option<Box<Spanned<Expr>>>,
    },

    Binary {
        op: BinOp,
        lhs: Box<Spanned<Expr>>,
        rhs: Box<Spanned<Expr>>,
    },

    Unary {
        op: UnOp,
        expr: Box<Spanned<Expr>>,
    },

    Call {
        callee: Box<Spanned<Expr>>,
        args: Vec<Spanned<Expr>>,
    },

    Index {
        target: Box<Spanned<Expr>>,
        index: Box<Spanned<Expr>>,
    },

    Field {
        target: Box<Spanned<Expr>>,
        field: Spanned<String>,
    },

    /// `expr?` — the try / error-propagation operator.
    Try(Box<Spanned<Expr>>),

    /// `expr.await`.
    Await(Box<Spanned<Expr>>),

    /// `expr as Type`.
    Cast {
        expr: Box<Spanned<Expr>>,
        ty: Spanned<Type>,
    },

    /// `lhs = rhs`, `lhs += rhs`, etc.
    Assign {
        op: AssignOp,
        lhs: Box<Spanned<Expr>>,
        rhs: Box<Spanned<Expr>>,
    },

    /// A string with interpolated expressions: `"Hello, ${name}!"`.
    StringInterp(Vec<StringPart>),
}

/// A segment of a string-interpolation expression.
#[derive(Debug, Clone)]
pub enum StringPart {
    Lit(String),
    Expr(Spanned<Expr>),
}

/// A scalar literal value.
#[derive(Debug, Clone)]
pub enum Literal {
    Int(i64),
    Float(f64),
    Str(String),
    Char(char),
    Bool(bool),
}

/// A single arm in a `match` expression.
#[derive(Debug, Clone)]
pub struct MatchArm {
    pub pattern: Spanned<Pattern>,
    pub guard: Option<Spanned<Expr>>,
    pub body: Spanned<Expr>,
}

/// Field initialiser in a record literal: `point { x: 1.0, y: 2.0 }`.
#[derive(Debug, Clone)]
pub struct FieldInit {
    pub name: Spanned<String>,
    /// `None` for shorthand `{ x }` (equivalent to `{ x: x }`).
    pub value: Option<Spanned<Expr>>,
}

/// Binary operators in order of precedence (low → high, mirroring the spec table).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Or,     // ||
    And,    // &&
    Eq,     // ==
    Ne,     // !=
    Lt,     // <
    Le,     // <=
    Gt,     // >
    Ge,     // >=
    BitOr,  // |
    BitXor, // ^
    BitAnd, // &
    Shl,    // <<
    Shr,    // >>
    Add,    // +
    Sub,    // -
    Mul,    // *
    Div,    // /
    Rem,    // %
}

/// Unary (prefix) operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnOp {
    Neg,    // -
    Not,    // !
    BitNot, // ~
    Ref,    // &
    RefMut, // &mut
    Deref,  // *
}

/// Assignment operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssignOp {
    Assign,
    AddAssign,
    SubAssign,
    MulAssign,
    DivAssign,
    RemAssign,
    BitOrAssign,
    BitAndAssign,
    BitXorAssign,
    ShlAssign,
    ShrAssign,
}
