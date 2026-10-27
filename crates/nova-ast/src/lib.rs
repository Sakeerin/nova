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

pub use expr::{BinOp, Expr, FieldInit, Literal, MatchArm, StringPart, UnOp, ViewArg, ViewNode};
pub use item::{
    AssignOp, AssocTypeBinding, ConstDecl, ExternBlock, Function, ImplBlock, Import,
    Module as ModuleDecl, Param, Record, RecordField, TraitDecl, TraitItem, TypeDecl, Visibility,
    WhereBound,
};
pub use pattern::Pattern;
pub use ty::{Type, TypeParam};

use nova_diagnostics::{Span, Spanned};

/// The top-level compilation unit — one `.nova` source file.
#[derive(Clone, Default)]
pub struct File {
    pub items: Vec<Spanned<Item>>,
    /// Each place where the newline rule ended a statement that the parser
    /// before it would have continued (5.0 spec
    /// `docs/superpowers/specs/2026-10-26-phase-5-0-the-language-and-std-a-compiler-needs-design.md`
    /// §3.5), in source order: the checker warns at each (E0103).
    pub newline_splits: Vec<NewlineSplit>,
}

/// `items`, and `newline_splits` only when there are any, so a file the
/// rule does not split prints as it did before the rule (`nova parse
/// --ast`).
impl std::fmt::Debug for File {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut d = f.debug_struct("File");
        d.field("items", &self.items);
        if !self.newline_splits.is_empty() {
            d.field("newline_splits", &self.newline_splits);
        }
        d.finish()
    }
}

/// A place where a line break ended a statement in statement context: the
/// token after the break, and what the parser before the rule would have
/// read there.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NewlineSplit {
    /// The first token of the line the rule made a new statement.
    pub at: Span,
    pub kind: SplitKind,
}

/// What a line break ended, by the token that begins the next line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitKind {
    /// A `(`, which would have called the expression before.
    Call,
    /// A `[`, which would have indexed it.
    Index,
    /// A binary `-`, which would have subtracted.
    Minus,
    /// A line after a `return` or `break`, which would have been its value.
    Value,
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
