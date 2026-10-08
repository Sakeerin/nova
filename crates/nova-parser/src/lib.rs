//! Parser for the Nova programming language.
//!
//! Converts a `Vec<Spanned<Token>>` (from `nova-lexer`) into an `ast::File`.
//! A hand-written recursive-descent parser: after an error it skips to the next
//! item or statement boundary and keeps going.
//!
//! # Usage
//!
//! ```
//! use nova_diagnostics::FileDb;
//! use nova_lexer::lex;
//! use nova_parser::parse;
//!
//! let mut db = FileDb::new();
//! let file_id = db.add("main.nova", "fn main() {}");
//! let source = db.get_source(file_id).unwrap();
//! let (tokens, _lex_errs) = lex(source, file_id);
//! let (ast, _parse_errs) = parse(&tokens, file_id);
//! ```

mod error;
mod grammar;

pub use error::ParseError;

use nova_ast::File;
use nova_diagnostics::{FileId, Spanned};
use nova_lexer::Token;

/// Parse a token stream into a `File` AST.
///
/// Returns `Some(File)` even when errors are present: the parser skips to the
/// next item or statement boundary after an error and keeps going. It
/// currently never returns `None`.
pub fn parse(tokens: &[Spanned<Token>], file: FileId) -> (Option<File>, Vec<ParseError>) {
    grammar::parse_file(tokens, file)
}

/// What [`parse_recovering`] returns.
#[derive(Debug)]
pub struct Parsed {
    /// The file, with every item that parsed.
    pub file: File,
    pub errors: Vec<ParseError>,
    /// The names of the top-level items dropped after their names were
    /// read: `fn f(x: Int {` drops `f`.
    pub dropped: Vec<Spanned<String>>,
}

/// [`parse`], also naming the items it dropped (spec
/// `docs/superpowers/specs/2026-10-08-phase-3-2-lsp-core-design.md` §3.2).
/// The language server's analysis calls this; every other caller uses
/// [`parse`], whose result is the same file and errors.
pub fn parse_recovering(tokens: &[Spanned<Token>], file: FileId) -> Parsed {
    grammar::parse_recovering(tokens, file)
}
