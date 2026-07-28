//! Top-level item AST nodes.

use nova_diagnostics::Spanned;

use crate::{expr::Expr, ty::Type, Block, Path};

/// Re-export from expr for convenience (AssignOp needed by callers of item).
pub use crate::expr::AssignOp;

/// Visibility of a declaration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Visibility {
    Pub,
    Private,
}

/// A function parameter.
#[derive(Debug, Clone)]
pub struct Param {
    pub is_mut: bool,
    pub name: Spanned<String>,
    pub ty: Spanned<Type>,
}

/// A `where` clause bound, e.g. `T: Display + Clone`.
#[derive(Debug, Clone)]
pub struct WhereBound {
    pub ty: Spanned<Type>,
    pub bounds: Vec<Spanned<Path>>,
}

/// A function or method declaration.
#[derive(Debug, Clone)]
pub struct Function {
    pub vis: Visibility,
    pub is_async: bool,
    pub name: Spanned<String>,
    pub generics: Vec<crate::ty::TypeParam>,
    pub params: Vec<Param>,
    pub return_ty: Option<Spanned<Type>>,
    pub where_clause: Vec<WhereBound>,
    pub body: Spanned<Block>,
}

/// A `record` (struct) declaration.
#[derive(Debug, Clone)]
pub struct Record {
    pub vis: Visibility,
    pub name: Spanned<String>,
    pub generics: Vec<crate::ty::TypeParam>,
    pub fields: Vec<RecordField>,
}

/// A single field inside a `record { ... }` body.
#[derive(Debug, Clone)]
pub struct RecordField {
    pub vis: Visibility,
    pub name: Spanned<String>,
    pub ty: Spanned<Type>,
}

/// A `type` alias or sum type declaration.
///
/// ```nova
/// type Color = | Red | Green | Blue
/// type Alias<T> = Vec<T>
/// ```
#[derive(Debug, Clone)]
pub struct TypeDecl {
    pub vis: Visibility,
    pub name: Spanned<String>,
    pub generics: Vec<crate::ty::TypeParam>,
    pub def: TypeDef,
}

/// The right-hand side of a `type` declaration.
#[derive(Debug, Clone)]
pub enum TypeDef {
    /// A plain type alias.
    Alias(Spanned<Type>),
    /// A sum type (algebraic data type / enum).
    Sum(Vec<Variant>),
}

/// A variant of a sum type.
#[derive(Debug, Clone)]
pub struct Variant {
    pub name: Spanned<String>,
    /// Fields of a tuple-style variant, e.g. `Ok(T)` has one field.
    pub fields: Vec<Spanned<Type>>,
}

/// A `trait` declaration.
#[derive(Debug, Clone)]
pub struct TraitDecl {
    pub vis: Visibility,
    pub name: Spanned<String>,
    pub generics: Vec<crate::ty::TypeParam>,
    pub supertraits: Vec<Spanned<Path>>,
    /// A `where` clause on the trait itself (e.g. `trait B where Self: A`), as
    /// opposed to the `trait B: A` shorthand (stored in `supertraits`). Parsed
    /// but not otherwise supported yet — `nova-typeck` reports `E0900` for a
    /// non-empty clause here rather than acting on it.
    pub where_clause: Vec<WhereBound>,
    pub items: Vec<TraitItem>,
}

/// An item inside a trait body.
#[derive(Debug, Clone)]
pub enum TraitItem {
    /// A required method (signature only).
    Required(FunctionSig),
    /// A provided method (with default body).
    Provided(Function),
    /// An associated type declaration: `type Item`.
    ///
    /// `bounds` is parsed (`type Item: Display`) but **not** supported —
    /// `nova-typeck` reports `E0900` for a non-empty list, the same way a
    /// `where` clause on a trait is parsed here and rejected there. Parsing it
    /// gives a precise span and a real diagnostic instead of a syntax error.
    AssocType {
        name: Spanned<String>,
        bounds: Vec<Spanned<Path>>,
    },
}

/// A function signature without a body.
#[derive(Debug, Clone)]
pub struct FunctionSig {
    pub is_async: bool,
    pub name: Spanned<String>,
    pub generics: Vec<crate::ty::TypeParam>,
    pub params: Vec<Param>,
    pub return_ty: Option<Spanned<Type>>,
    pub where_clause: Vec<WhereBound>,
}

/// An `impl` block.
#[derive(Debug, Clone)]
pub struct ImplBlock {
    pub generics: Vec<crate::ty::TypeParam>,
    /// If `Some`, this is a trait impl: `impl Trait for Type`.
    pub trait_: Option<Spanned<Path>>,
    pub ty: Spanned<Type>,
    pub where_clause: Vec<WhereBound>,
    pub functions: Vec<Function>,
    pub consts: Vec<ConstDecl>,
}

/// A `const` declaration.
#[derive(Debug, Clone)]
pub struct ConstDecl {
    pub vis: Visibility,
    pub name: Spanned<String>,
    pub ty: Spanned<Type>,
    pub value: Spanned<Expr>,
}

/// An `import` declaration.
#[derive(Debug, Clone)]
pub struct Import {
    pub path: Spanned<Path>,
    pub kind: ImportKind,
}

/// How an import is bound into the current namespace.
#[derive(Debug, Clone)]
pub enum ImportKind {
    /// `import foo` — binds the last segment name.
    Simple,
    /// `import foo as bar` — binds under an alias.
    Alias(Spanned<String>),
    /// `import foo::{a, b, c}` — destructures selected names.
    List(Vec<Spanned<String>>),
}

/// A `module` declaration (refers to another file).
#[derive(Debug, Clone)]
pub struct Module {
    pub path: Spanned<Path>,
}

/// An `extern` block for FFI declarations.
#[derive(Debug, Clone)]
pub struct ExternBlock {
    pub abi: Option<String>,
    pub items: Vec<ExternItem>,
}

/// An item inside an `extern` block.
#[derive(Debug, Clone)]
pub enum ExternItem {
    Fn(FunctionSig),
}
