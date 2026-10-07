//! The Nova formatter (spec
//! `docs/superpowers/specs/2026-10-07-phase-3-1-formatter-design.md`).

mod check;
mod comments;
mod doc;
mod editorconfig;
mod file;
mod print;
mod source;

pub use file::{format_file, format_text, FileError, Formatted, LineEnding};

use nova_diagnostics::Diagnostic;

/// The width lines are fitted in, in Unicode scalar values (spec §6).
pub const WIDTH: usize = 100;

/// Why formatting produced nothing.
#[derive(Debug, thiserror::Error)]
pub enum FormatError {
    /// The input does not lex or parse. `rendered` holds the diagnostics as
    /// `nova check` prints them, without colour.
    #[error("the input does not parse:\n{rendered}")]
    Syntax {
        /// The lexer's and the parser's diagnostics, as `nova check` makes
        /// them.
        diagnostics: Vec<Diagnostic>,
        /// Those diagnostics, rendered to text.
        rendered: String,
    },
    /// The self-check failed (spec §5.5): the output would have changed the
    /// program or lost a comment. `first_difference` says where.
    #[error("formatting would change the program or its comments: {first_difference}")]
    Internal {
        /// Where the output first differs from the input.
        first_difference: String,
    },
}

/// Format `source`, naming it `<stdin>` in any diagnostics (spec §5.1).
pub fn format(source: &str) -> Result<String, FormatError> {
    format_named(source, "<stdin>")
}

/// Format `source`, naming it `name` in any diagnostics (spec §5.1): the
/// canonical layout, with `\n` line endings and one final newline, checked
/// against the input before it is returned (§5.5).
pub fn format_named(source: &str, name: &str) -> Result<String, FormatError> {
    format_with(source, name, print::print)
}

/// Format `source` with `print`, then check the output (spec §5.1, §5.5).
/// The public functions pass the real printer; the tests pass broken ones,
/// to watch the check refuse their output.
fn format_with(
    source: &str,
    name: &str,
    print: fn(&source::Source) -> String,
) -> Result<String, FormatError> {
    let text = source.replace("\r\n", "\n");
    let input = source::Source::parse(&text, name)?;
    let output = print(&input);
    check::check(&input, &output)?;
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Prints the input unchanged: always passes the check.
    fn unchanged(src: &source::Source) -> String {
        src.text.to_owned()
    }

    /// Drops every line comment.
    fn without_comments(src: &source::Source) -> String {
        src.text
            .lines()
            .filter(|l| !l.trim_start().starts_with("//"))
            .map(|l| format!("{l}\n"))
            .collect()
    }

    /// Renames `main`: a different program.
    fn renamed(src: &source::Source) -> String {
        src.text.replace("main", "mane")
    }

    /// Prints something that does not parse.
    fn broken(_: &source::Source) -> String {
        "fn (".to_owned()
    }

    /// Prints imports sorted, as the printer does (spec §6).
    fn sorted(_: &source::Source) -> String {
        "import alpha::{a, b}\nimport zeta\n".to_owned()
    }

    /// Sorts two commented imports, moving each comment with its import.
    fn moved(_: &source::Source) -> String {
        "// a\nimport alpha\n// z\nimport zeta\n".to_owned()
    }

    /// Swaps two comments that are not in an import run.
    fn swapped(_: &source::Source) -> String {
        "// b\n// a\nfn main() {}\n".to_owned()
    }

    fn internal(result: Result<String, FormatError>) -> bool {
        matches!(result, Err(FormatError::Internal { .. }))
    }

    #[test]
    fn output_that_keeps_the_program_and_its_comments_passes() {
        let out = format_with("// c\nfn main() {}\n", "<t>", unchanged).unwrap();
        assert_eq!(out, "// c\nfn main() {}\n");
    }

    #[test]
    fn crlf_input_reaches_the_printer_as_lf() {
        let out = format_with("fn main() {}\r\n", "<t>", unchanged).unwrap();
        assert_eq!(out, "fn main() {}\n");
    }

    #[test]
    fn the_self_check_refuses_output_that_lost_a_comment() {
        assert!(internal(format_with(
            "// keep me\nfn main() {}\n",
            "<t>",
            without_comments
        )));
    }

    #[test]
    fn the_self_check_refuses_output_that_changed_the_program() {
        assert!(internal(format_with("fn main() {}\n", "<t>", renamed)));
    }

    #[test]
    fn the_self_check_refuses_output_that_does_not_parse() {
        assert!(internal(format_with("fn main() {}\n", "<t>", broken)));
    }

    #[test]
    fn sorting_imports_is_not_a_change() {
        assert!(format_with("import zeta\nimport alpha::{b, a}\n", "<t>", sorted).is_ok());
    }

    #[test]
    fn comments_in_an_import_run_may_move_with_their_imports() {
        assert!(format_with("// z\nimport zeta\n// a\nimport alpha\n", "<t>", moved).is_ok());
    }

    #[test]
    fn a_comment_moved_out_of_order_elsewhere_is_refused() {
        assert!(internal(format_with(
            "// a\n// b\nfn main() {}\n",
            "<t>",
            swapped
        )));
    }

    #[test]
    fn a_syntax_error_is_reported_as_nova_check_would() {
        match format_with("fn main( {\n", "bad.nova", unchanged) {
            Err(FormatError::Syntax {
                diagnostics,
                rendered,
            }) => {
                assert!(!diagnostics.is_empty());
                assert!(rendered.contains("P0001"), "{rendered}");
                assert!(rendered.contains("bad.nova"), "{rendered}");
            }
            other => panic!("{other:?}"),
        }
    }
}
