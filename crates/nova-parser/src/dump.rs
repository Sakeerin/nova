//! The AST dump of a source (spec
//! `docs/superpowers/specs/2026-10-30-phase-5-1-the-lexer-the-parser-and-the-ast-design.md`
//! §4.3, ADR 0052): its tree, as `nova_ast::dump` writes it, then its
//! errors, one per line, in the order they were found:
//!
//! - `ERR start end message` for each lex error, all of them first, as the
//!   callers render them before the parse errors;
//! - `PARSE start end message` for each parse error, then `NOTE note` for
//!   each of its notes. The code is not written: P0002's message names it,
//!   and every other parse error is P0001.
//!
//! A message or a note writes each character below 0x20 and 0x7F as
//! `\xHH`, and every other as it is. The self-hosted parser prints the same
//! bytes (`selfhost ast`), and `nova parse --dump-ast` a `== FILE` header
//! before them.

use std::fmt::Write;

use nova_ast::dump::{dump_file, escape_message};
use nova_diagnostics::FileId;

use crate::parse;

/// The AST dump of `source` (module docs).
pub fn dump_ast(source: &str) -> String {
    let (tokens, lex_errors) = nova_lexer::lex(source, FileId::DUMMY);
    let (file, errors) = parse(&tokens, FileId::DUMMY);
    let mut out = dump_file(&file.unwrap_or_default());
    for e in &lex_errors {
        let span = e.span();
        let _ = write!(out, "ERR {} {} ", span.start, span.end);
        escape_message(&mut out, &e.to_string());
        out.push('\n');
    }
    for e in &errors {
        let span = e.span();
        let _ = write!(out, "PARSE {} {} ", span.start, span.end);
        escape_message(&mut out, &e.to_string());
        out.push('\n');
        for note in e.notes() {
            out.push_str("NOTE ");
            escape_message(&mut out, &note);
            out.push('\n');
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_errors_follow_the_tree() {
        let out = dump_ast("fn f() { \"x }\nfn g( {}\n");
        let errors: Vec<&str> = out
            .lines()
            .filter(|l| !l.starts_with('(') && !l.starts_with(' '))
            .collect();
        assert_eq!(
            errors,
            [
                "ERR 10 23 unterminated string literal",
                "PARSE 23 23 unexpected end of file",
                "PARSE 23 23 expected `}` (in function body), found end of file",
            ],
            "{out}"
        );
        // A note follows its error.
        let out =
            dump_ast("fn f() {\n    match x {\n        A => g\n        (a + 1) => h\n    }\n}\n");
        let lines: Vec<&str> = out.lines().collect();
        let at = lines
            .iter()
            .position(|l| l.starts_with("PARSE "))
            .expect("an error");
        assert!(
            lines[at + 1].starts_with("NOTE a line that starts with `(`"),
            "{out}"
        );
        // The newline rule's splits close the tree.
        let out = dump_ast("fn f() {\n    g\n    (x)\n}\n");
        assert!(out.ends_with("\n  (Split 19..20 kind=Call))\n"), "{out}");
    }

    #[test]
    fn a_file_nested_too_deeply_is_one_p0002() {
        let src = format!("fn f() {{ {}1{} }}\n", "(".repeat(200), ")".repeat(200));
        let out = std::thread::Builder::new()
            .stack_size(64 << 20)
            .spawn(move || dump_ast(&src))
            .expect("a thread")
            .join()
            .expect("the dump");
        assert_eq!(
            out,
            "(File)\nPARSE 138 139 this is nested too deeply (more than 128 levels)\n"
        );
    }
}
