//! Type expression AST nodes.

use nova_diagnostics::Spanned;

use crate::Path;

/// A type expression.
#[derive(Debug, Clone)]
pub enum Type {
    /// A named type, possibly with generic arguments: `Vec<Int>`, `String`.
    Path {
        path: Path,
        args: Vec<Spanned<Type>>,
    },
    /// A shared reference `&T` or mutable reference `&mut T`.
    Ref {
        is_mut: bool,
        inner: Box<Spanned<Type>>,
    },
    /// A raw pointer `*T` or `*mut T` (unsafe only).
    Ptr {
        is_mut: bool,
        inner: Box<Spanned<Type>>,
    },
    /// An array / slice type `[T]`.
    Array(Box<Spanned<Type>>),
    /// A tuple type `(A, B, C)`. Empty tuple `()` is the unit type.
    Tuple(Vec<Spanned<Type>>),
    /// A function type `fn(A, B) -> C`.
    Fn {
        params: Vec<Spanned<Type>>,
        ret: Box<Spanned<Type>>,
    },
    /// The optional sugar `T?` (equivalent to `Option<T>`).
    Optional(Box<Spanned<Type>>),
    /// A placeholder `_` that tells the compiler to infer the type.
    Infer,
}

/// A generic type parameter, e.g. `T: Display + Clone`.
#[derive(Debug, Clone)]
pub struct TypeParam {
    pub name: Spanned<String>,
    pub bounds: Vec<Spanned<Path>>,
}
