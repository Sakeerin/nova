//! Abstract Syntax Tree node definitions for the Nova language.
//!
//! All nodes carry `Spanned<T>` wrappers so the parser, type checker, and
//! diagnostics always have precise source locations.
//!
//! Major node categories live in sub-modules re-exported here:
//! - `item` — top-level items (functions, records, traits, …)
//! - `expr` — expressions
//! - `pattern` — match patterns
//! - `ty` — type expressions

pub mod expr;
pub mod item;
pub mod pattern;
pub mod ty;

pub use expr::{BinOp, Expr, FieldInit, Literal, MatchArm, StringPart, UnOp};
pub use item::{
    AssignOp, AssocTypeBinding, ConstDecl, ExternBlock, Function, ImplBlock, Import,
    Module as ModuleDecl, Param, Record, RecordField, TraitDecl, TraitItem, TypeDecl, Visibility,
    WhereBound,
};
pub use pattern::Pattern;
pub use ty::{Type, TypeParam};

use nova_diagnostics::Spanned;

/// The top-level compilation unit — one `.nova` source file.
#[derive(Debug, Clone)]
pub struct File {
    pub items: Vec<Spanned<Item>>,
}

/// Every kind of top-level item that can appear in a Nova file.
#[derive(Debug, Clone)]
pub enum Item {
    Function(Function),
    Record(Record),
    Type(TypeDecl),
    Trait(TraitDecl),
    Impl(ImplBlock),
    Const(ConstDecl),
    Import(Import),
    Module(ModuleDecl),
    Extern(ExternBlock),
}

/// A statement — may appear inside a block.
#[derive(Debug, Clone)]
pub enum Stmt {
    Let {
        is_mut: bool,
        pattern: Spanned<Pattern>,
        ty: Option<Spanned<Type>>,
        init: Option<Spanned<Expr>>,
    },
    Expr(Spanned<Expr>),
    Item(Box<Item>),
}

/// A `{ ... }` block — a sequence of statements with an optional trailing expression.
#[derive(Debug, Clone)]
pub struct Block {
    pub stmts: Vec<Spanned<Stmt>>,
    /// The final expression whose value is the block's value, if present.
    pub trailing: Option<Box<Spanned<Expr>>>,
}

/// A `path::to::item` reference.
#[derive(Debug, Clone)]
pub struct Path {
    pub segments: Vec<Spanned<String>>,
}

impl Path {
    pub fn single(name: Spanned<String>) -> Self {
        Self {
            segments: vec![name],
        }
    }
}
