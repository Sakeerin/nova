//! Nova's diagnostics as LSP's (spec
//! `docs/superpowers/specs/2026-10-08-phase-3-2-lsp-core-design.md` §6.3).

use std::collections::HashMap;
use std::path::Path;

use lsp_types as lsp;
use nova_diagnostics::{Diagnostic, FileId, Label, LineIndex, Severity, Span};
use nova_driver::Analysis;
use nova_pm::PackageId;

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

/// The LSP diagnostics `analysis` has for `file` (spec §6.3, and 3.3a §6).
///
/// Each diagnostic is shown in one place:
/// - the file of its primary label, or of its first label, among the root
///   package's own modules;
/// - else, given the project's `manifest`, that `nova.toml` for a label in
///   it;
/// - else, for a label in a dependency's file or manifest, the root's
///   manifest entry through which the dependency is reached, its message
///   naming where it really is, such as "(in geom: ../geom/src/lib.nova:3:5)";
/// - else `entry`'s first line, naming the place it has: the 3.2
///   fallback, for E0601 and for labels in std.
///
/// Its other labels in the own modules become related information, under
/// the URIs `uri_of` gives their paths.
pub fn diagnostics_for(
    analysis: &Analysis,
    file: FileId,
    entry: FileId,
    manifest: Option<FileId>,
    uri_of: &dyn Fn(&Path) -> String,
) -> Vec<lsp::Diagnostic> {
    // A dependency's modules are its own project's (spec 3.3a §6).
    let own: Vec<FileId> = analysis
        .modules
        .iter()
        .zip(&analysis.module_packages)
        .filter(|(_, package)| matches!(package, None | Some(PackageId(0))))
        .map(|((file, _), _)| *file)
        .collect();
    let mut indexes: HashMap<FileId, LineIndex> = HashMap::new();
    let mut out = Vec::new();
    for d in &analysis.diagnostics {
        let shown = shown(analysis, d, entry, manifest, &own);
        if shown.file != file {
            continue;
        }
        let at = span_range(analysis, &mut indexes, shown.at);
        let mut related = Vec::new();
        for other in &d.labels {
            if shown.label.is_some_and(|l| std::ptr::eq(l, other))
                || !own.contains(&other.span.file)
            {
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
            message: message(d, shown.suffix.as_deref()),
            related_information: (!related.is_empty()).then_some(related),
            ..Default::default()
        });
    }
    out
}

/// Where one diagnostic is shown.
struct Shown<'d> {
    file: FileId,
    at: Span,
    /// The label it is shown at, when that is one of its own.
    label: Option<&'d Label>,
    /// Where it really is, when it is shown somewhere else.
    suffix: Option<String>,
}

fn shown<'d>(
    analysis: &Analysis,
    d: &'d Diagnostic,
    entry: FileId,
    manifest: Option<FileId>,
    own: &[FileId],
) -> Shown<'d> {
    let label = d
        .labels
        .iter()
        .find(|l| l.primary && own.contains(&l.span.file))
        .or_else(|| d.labels.iter().find(|l| own.contains(&l.span.file)));
    if let Some(l) = label {
        return Shown {
            file: l.span.file,
            at: l.span,
            label: Some(l),
            suffix: None,
        };
    }
    let first = d.labels.iter().find(|l| l.primary).or(d.labels.first());
    if let (Some(manifest), Some(l)) = (manifest, first) {
        if l.span.file == manifest {
            return Shown {
                file: manifest,
                at: l.span,
                label: Some(l),
                suffix: None,
            };
        }
        let graph = analysis.graph.as_ref();
        let reached = package_of_file(analysis, l.span.file)
            .filter(|package| *package != PackageId(0))
            .and_then(|package| Some((package, graph?.reached_through(package)?)));
        if let (Some(graph), Some((package, at))) = (graph, reached) {
            let name = &graph.package(package).name;
            return Shown {
                file: manifest,
                at,
                label: None,
                suffix: Some(format!("(in {name}: {})", place(analysis, l.span))),
            };
        }
    }
    Shown {
        file: entry,
        at: Span::point(0, entry),
        label: None,
        suffix: d
            .labels
            .first()
            .map(|l| format!("(at {})", place(analysis, l.span))),
    }
}

/// The package `file` belongs to: a module's, or a manifest's.
fn package_of_file(analysis: &Analysis, file: FileId) -> Option<PackageId> {
    if let Some(i) = analysis.modules.iter().position(|(f, _)| *f == file) {
        return analysis.module_packages[i];
    }
    let graph = analysis.graph.as_ref()?;
    graph
        .packages
        .iter()
        .position(|p| p.manifest_file == file)
        .map(|i| PackageId(i as u32))
}

/// `name:line:column` of where `span` starts.
fn place(analysis: &Analysis, span: Span) -> String {
    let name = analysis.db.get_name(span.file).unwrap_or("?");
    match analysis.db.location(span.file, span.start) {
        Some((line, column)) => format!("{name}:{line}:{column}"),
        None => name.to_string(),
    }
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

/// The message, its notes one per line, and where it really is when it is
/// shown somewhere else, such as `(at <std/core>:12:5)`.
fn message(d: &Diagnostic, suffix: Option<&str>) -> String {
    let mut m = d.message.clone();
    for note in &d.notes {
        m.push('\n');
        m.push_str(note);
    }
    if let Some(suffix) = suffix {
        m.push('\n');
        m.push_str(suffix);
    }
    m
}
