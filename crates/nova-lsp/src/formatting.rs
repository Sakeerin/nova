//! Document formatting (spec
//! `docs/superpowers/specs/2026-10-08-phase-3-2-lsp-core-design.md` §6.5).

use std::path::Path;

use lsp_types as lsp;
use nova_diagnostics::LineIndex;

/// The edits that format `text`, the buffer for `path`:
/// - one replacing the whole document;
/// - none if the buffer is already formatted, has a syntax error, or the
///   self-check refuses the output.
///
/// A refusal goes to the log, so format-on-save never raises an error.
pub fn format_document(path: &Path, text: &str) -> Vec<lsp::TextEdit> {
    match nova_fmt::format_buffer(path, text) {
        Ok(formatted) if formatted == text => Vec::new(),
        Ok(formatted) => {
            let index = LineIndex::new(text);
            vec![lsp::TextEdit {
                range: crate::convert::range(&index, 0, index.len()),
                new_text: formatted,
            }]
        }
        Err(e) => {
            tracing::warn!("nova lsp: not formatting {}: {e}", path.display());
            Vec::new()
        }
    }
}
