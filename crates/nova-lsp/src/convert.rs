//! Nova's diagnostics as LSP's (spec
//! `docs/superpowers/specs/2026-10-08-phase-3-2-lsp-core-design.md` §6.3).

use std::collections::HashMap;
use std::path::Path;

use lsp_types as lsp;
use nova_diagnostics::{Diagnostic, FileId, LineIndex, Severity, Span};
use nova_driver::Analysis;

/// LSP's range for bytes `start..end` of the indexed text.
pub fn range(index: &LineIndex, start: u32, end: u32) -> lsp::Range {
    let (sl, sc) = index.position(start);
    let (el, ec) = index.position(end);
    lsp::Range {
        start: lsp::Position {
            line: sl,
            character: sc,
        },
        end: lsp::Position {
            line: el,
            character: ec,
        },
    }
}

/// The LSP diagnostics `analysis` has for `file`.
///
/// A diagnostic goes to the file of its primary label, or of its first label
/// in one of the program's own files. One with no label in the program's
/// files goes to `entry`'s first line, naming the place it has: the spec
/// §6.3 fallback, for E0601 and for labels in std. Its other labels in the
/// program's files become related information, under the URIs `uri_of`
/// gives their paths.
pub fn diagnostics_for(
    analysis: &Analysis,
    file: FileId,
    entry: FileId,
    uri_of: &dyn Fn(&Path) -> String,
) -> Vec<lsp::Diagnostic> {
    let own: Vec<FileId> = analysis.modules.iter().map(|(f, _)| *f).collect();
    let mut indexes: HashMap<FileId, LineIndex> = HashMap::new();
    let mut out = Vec::new();
    for d in &analysis.diagnostics {
        let label = d
            .labels
            .iter()
            .find(|l| l.primary && own.contains(&l.span.file))
            .or_else(|| d.labels.iter().find(|l| own.contains(&l.span.file)));
        let place = label.map_or(entry, |l| l.span.file);
        if place != file {
            continue;
        }
        let at = match label {
            Some(l) => span_range(analysis, &mut indexes, l.span),
            None => span_range(analysis, &mut indexes, Span::point(0, entry)),
        };
        let mut related = Vec::new();
        for other in &d.labels {
            if label.is_some_and(|l| std::ptr::eq(l, other)) || !own.contains(&other.span.file) {
                continue;
            }
            let Some((_, path)) = analysis.modules.iter().find(|(f, _)| *f == other.span.file)
            else {
                continue;
            };
            let Some(uri) = crate::uri::parse(&uri_of(path)) else {
                continue;
            };
            related.push(lsp::DiagnosticRelatedInformation {
                location: lsp::Location {
                    uri,
                    range: span_range(analysis, &mut indexes, other.span),
                },
                message: other.message.clone(),
            });
        }
        out.push(lsp::Diagnostic {
            range: at,
            severity: Some(severity(d.severity)),
            code: Some(lsp::NumberOrString::String(d.code.clone())),
            source: Some("nova".to_string()),
            message: message(analysis, d, label.is_none()),
            related_information: (!related.is_empty()).then_some(related),
            ..Default::default()
        });
    }
    out
}

/// The LSP range of `span`, indexing its file's text once.
fn span_range(
    analysis: &Analysis,
    indexes: &mut HashMap<FileId, LineIndex>,
    span: Span,
) -> lsp::Range {
    let index = indexes
        .entry(span.file)
        .or_insert_with(|| LineIndex::new(analysis.db.get_source(span.file).unwrap_or("")));
    range(index, span.start, span.end)
}

fn severity(s: Severity) -> lsp::DiagnosticSeverity {
    match s {
        Severity::Error => lsp::DiagnosticSeverity::ERROR,
        Severity::Warning => lsp::DiagnosticSeverity::WARNING,
        Severity::Note => lsp::DiagnosticSeverity::INFORMATION,
        Severity::Help => lsp::DiagnosticSeverity::HINT,
    }
}

/// The message, its notes one per line, and for a diagnostic placed by the
/// fallback, where it really points, such as `<std/core>:12:5`.
fn message(analysis: &Analysis, d: &Diagnostic, fallback: bool) -> String {
    let mut m = d.message.clone();
    for note in &d.notes {
        m.push('\n');
        m.push_str(note);
    }
    if fallback {
        if let Some(l) = d.labels.first() {
            let name = analysis.db.get_name(l.span.file);
            let at = analysis.db.location(l.span.file, l.span.start);
            if let (Some(name), Some((line, column))) = (name, at) {
                m.push_str(&format!("\n(at {name}:{line}:{column})"));
            }
        }
    }
    m
}
