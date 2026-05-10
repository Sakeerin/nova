//! Pattern AST nodes (used in `let`, `match` arms, `for` loops).

use nova_diagnostics::Spanned;

use crate::{expr::Literal, Path};

/// A pattern.
#[derive(Debug, Clone)]
pub enum Pattern {
    /// `_` — wildcard, matches anything without binding.
    Wildcard,

    /// A literal value pattern: `0`, `true`, `'a'`, `"str"`.
    Lit(Literal),

    /// A bare identifier binding: `x`, `mut y`.
    Ident { is_mut: bool, name: Spanned<String> },

    /// A binding with a sub-pattern: `x @ 0..=9`.
    Binding {
        name: Spanned<String>,
        inner: Box<Spanned<Pattern>>,
    },

    /// A tuple-struct / enum variant pattern: `Some(x)`, `Ok(v)`.
    TupleStruct {
        path: Path,
        fields: Vec<Spanned<Pattern>>,
    },

    /// A record / struct pattern: `Point { x, y }`.
    Record {
        path: Path,
        fields: Vec<FieldPat>,
        rest: bool,
    },

    /// A tuple pattern: `(a, b, c)`.
    Tuple(Vec<Spanned<Pattern>>),

    /// An array pattern: `[a, b, c]`.
    Array(Vec<Spanned<Pattern>>),

    /// An or-pattern: `None | Some(0)`.
    Or(Vec<Spanned<Pattern>>),

    /// A range pattern: `0..=9`, `'a'..='z'`.
    Range {
        lo: Box<Spanned<Pattern>>,
        hi: Box<Spanned<Pattern>>,
        inclusive: bool,
    },

    /// A bare path that could be an enum variant without payload: `None`, `MyEnum::Variant`.
    Path(Path),
}

/// A single field in a record pattern.
#[derive(Debug, Clone)]
pub struct FieldPat {
    pub name: Spanned<String>,
    /// `None` means shorthand `{ x }` which binds to a variable named `x`.
    pub pattern: Option<Spanned<Pattern>>,
}
