//! Code actions: each fix as a quick fix, and organize imports (spec
//! `docs/superpowers/specs/2026-10-10-phase-3-4b-fixes-and-colour-design.md`
//! §5).

use std::collections::HashMap;
use std::path::Path;

use lsp_types as lsp;
use nova_diagnostics::{Diagnostic, Fix};

use crate::analysis::Answer;
use crate::navigate::Locator;
use crate::rename::{in_registry, owner, Owner};
use crate::{convert, std_cache};

/// Whether an action of `kind` is wanted, given the request's `only`: its
/// kind is an entry, or starts with an entry and a `.` (spec §5).
fn wanted(kind: &lsp::CodeActionKind, only: Option<&[lsp::CodeActionKind]>) -> bool {
    only.map_or(true, |only| {
        only.iter().any(|o| {
            kind.as_str() == o.as_str() || kind.as_str().starts_with(&format!("{}.", o.as_str()))
        })
    })
}

/// The actions for bytes `start..end` of `path`'s document (spec §5).
/// None inside std's cache or a downloaded package, as rename refuses
/// there.
pub fn code_actions(
    answer: &Answer,
    path: &Path,
    start: u32,
    end: u32,
    only: Option<&[lsp::CodeActionKind]>,
    uri_of: &dyn Fn(&Path) -> String,
) -> Vec<lsp::CodeActionOrCommand> {
    if std_cache::in_std_cache(path) || in_registry(path) {
        return Vec::new();
    }
    let mut out = Vec::new();
    if wanted(&lsp::CodeActionKind::QUICKFIX, only) {
        out.extend(quick_fixes(answer, start, end, uri_of));
    }
    out
}

/// Each fix of each diagnostic whose primary label overlaps
/// `start..end`, once, in the diagnostics' order (spec §5). A fix with an
/// edit outside the owning project is dropped.
fn quick_fixes(
    answer: &Answer,
    start: u32,
    end: u32,
    uri_of: &dyn Fn(&Path) -> String,
) -> Vec<lsp::CodeActionOrCommand> {
    let a = &answer.analysis;
    let mut found: Vec<(u32, &Diagnostic, &Fix)> = Vec::new();
    for d in &a.diagnostics {
        let Some(label) = d.labels.iter().find(|l| l.primary) else {
            continue;
        };
        if label.span.file != answer.file || label.span.start > end || start > label.span.end {
            continue;
        }
        for fix in &d.fixes {
            if fix
                .edits
                .iter()
                .all(|e| matches!(owner(a, e.span.file), Owner::Own))
            {
                found.push((label.span.start, d, fix));
            }
        }
    }
    found.sort_by_key(|(at, _, _)| *at);
    let locator = Locator::new(a, uri_of);
    let mut actions: Vec<(&Fix, lsp::CodeAction)> = Vec::new();
    for (_, d, fix) in found {
        let diagnostic = lsp_diagnostic(&locator, d);
        if let Some((_, action)) = actions.iter_mut().find(|(f, _)| **f == *fix) {
            action
                .diagnostics
                .get_or_insert_with(Vec::new)
                .extend(diagnostic);
            continue;
        }
        let Some(edit) = workspace_edit(&locator, fix) else {
            continue;
        };
        actions.push((
            fix,
            lsp::CodeAction {
                title: capitalised(&fix.title),
                kind: Some(lsp::CodeActionKind::QUICKFIX),
                diagnostics: Some(diagnostic.into_iter().collect()),
                edit: Some(edit),
                is_preferred: Some(d.fixes.len() == 1),
                ..Default::default()
            },
        ));
    }
    actions
        .into_iter()
        .map(|(_, action)| lsp::CodeActionOrCommand::CodeAction(action))
        .collect()
}

/// `d` as the server publishes it, at its primary label, without related
/// information (plan decision 14).
fn lsp_diagnostic(locator: &Locator, d: &Diagnostic) -> Option<lsp::Diagnostic> {
    let label = d.labels.iter().find(|l| l.primary)?;
    Some(lsp::Diagnostic {
        range: locator.range(label.span)?,
        severity: Some(convert::severity(d.severity)),
        code: Some(lsp::NumberOrString::String(d.code.clone())),
        source: Some("nova".to_string()),
        message: convert::message(d, None),
        ..Default::default()
    })
}

/// `fix`'s edits as LSP's, each file under the URI the locator gives it;
/// `None` when one cannot be placed.
// `WorkspaceEdit::changes` is a `HashMap` keyed by `lsp::Uri`, whose
// parsed form caches into a `Cell`; the key is never changed here.
#[allow(clippy::mutable_key_type)]
fn workspace_edit(locator: &Locator, fix: &Fix) -> Option<lsp::WorkspaceEdit> {
    let mut changes: HashMap<lsp::Uri, Vec<lsp::TextEdit>> = HashMap::new();
    for e in &fix.edits {
        let uri = locator.uri(e.span.file)?;
        let range = locator.range(e.span)?;
        changes.entry(uri).or_default().push(lsp::TextEdit {
            range,
            new_text: e.text.clone(),
        });
    }
    Some(lsp::WorkspaceEdit {
        changes: Some(changes),
        ..Default::default()
    })
}

/// A fix's title as the editor shows it (spec decision 16).
fn capitalised(title: &str) -> String {
    let mut chars = title.chars();
    match chars.next() {
        Some(c) => c.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}
