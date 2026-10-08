//! Completion (spec
//! `docs/superpowers/specs/2026-10-08-phase-3-2-lsp-core-design.md` §6.4).

use std::collections::HashSet;
use std::path::Path;

use lsp_types as lsp;
use lsp_types::CompletionItemKind as Kind;
use nova_diagnostics::FileId;
use nova_driver::{analyze, analyze_program, Analysis, Options, Probe, Program, Roots};
use nova_lexer::Token;
use nova_resolver::{DefKind, ModuleId, Res, ScopeEntry};
use nova_typeck::{Member, MemberKind};

use crate::workspace::{Overlay, PathKey, ProjectKey};

/// The completion items at byte `offset` of `text`, the buffer for `path`.
pub fn complete(
    path: &Path,
    text: &str,
    offset: u32,
    overlay: &Overlay,
) -> Vec<lsp::CompletionItem> {
    if in_literal_or_comment(text, offset) {
        return Vec::new();
    }
    let Some(analysis) = analysis_at(path, offset, overlay) else {
        return Vec::new();
    };
    if analysis.probe.receiver.is_some() {
        members(&analysis)
    } else {
        names(&analysis, path)
    }
}

/// The analysis that owns `path`, with the probe at `offset` (decision 9):
/// its project's analysis, if that reaches it, or else its own, which
/// `Program::for_file` finds the package of (spec 3.3a §6). Completion needs
/// no MIR, so `module_only` is always on.
fn analysis_at(path: &Path, offset: u32, overlay: &Overlay) -> Option<Analysis> {
    let options = Options {
        keep_going: true,
        tests: true,
        module_only: true,
        probe: Some(Probe {
            path: path.to_path_buf(),
            offset,
        }),
    };
    let project = ProjectKey::of(path);
    if let ProjectKey::Root(dir) = &project {
        let program = Program::for_package(dir, Roots::Test);
        if let Some(a) = guarded(|| analyze_program(program, overlay, &options).ok()) {
            if a.modules
                .iter()
                .any(|(_, p)| PathKey::of(p) == PathKey::of(path))
            {
                return Some(a);
            }
        }
    }
    // A loose file, or a project file its entry does not reach: either way
    // it is analysed as its own entry.
    guarded(|| analyze(path, overlay, &options).ok())
}

fn guarded<T>(f: impl FnOnce() -> Option<T>) -> Option<T> {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)) {
        Ok(v) => v,
        Err(_) => {
            tracing::warn!("nova lsp: the front end panicked during completion");
            None
        }
    }
}

/// Whether `offset` is inside a string, a character literal or a comment
/// (decision 14). Interpolations, `${…}`, are code.
fn in_literal_or_comment(text: &str, offset: u32) -> bool {
    let (tokens, comments, _) = nova_lexer::lex_with_comments(text, FileId::DUMMY);
    if comments
        .iter()
        .any(|c| c.span.start < offset && offset <= c.span.end)
    {
        return true;
    }
    tokens.iter().any(|t| {
        let (start, end) = (t.span.start, t.span.end);
        match t.value {
            Token::StrPart(_) | Token::RawStr(_) | Token::Char(_) => start < offset && offset < end,
            // Just inside the opening or closing quote.
            Token::StrStart => end == offset,
            Token::StrEnd => start == offset,
            _ => false,
        }
    })
}

fn item(label: &str, kind: Kind, detail: Option<String>) -> lsp::CompletionItem {
    lsp::CompletionItem {
        label: label.to_string(),
        kind: Some(kind),
        detail,
        ..Default::default()
    }
}

/// After `.`: the receiver's members (spec §4.2).
fn members(analysis: &Analysis) -> Vec<lsp::CompletionItem> {
    analysis
        .probe
        .members
        .iter()
        .map(|m| {
            let kind = match m.kind {
                MemberKind::Field => Kind::FIELD,
                MemberKind::Method => Kind::METHOD,
            };
            item(&m.name, kind, Some(member_detail(analysis, m)))
        })
        .collect()
}

/// A member's declaration as written, on one line (decision 3): a field's
/// type, or `fn` and a method's signature.
fn member_detail(analysis: &Analysis, m: &Member) -> String {
    let Some(span) = m.decl else {
        return "fn len(self) -> Int".to_string();
    };
    let Some(source) = analysis.db.get_source(span.file) else {
        return String::new();
    };
    match m.kind {
        MemberKind::Field => collapse(&source[span.start as usize..span.end as usize]),
        MemberKind::Method => format!("fn {}", collapse(signature(&source[span.start as usize..]))),
    }
}

/// From a method's name to its body: up to the first `{` or `;`, or the end
/// of its line, outside parentheses and brackets.
fn signature(from_name: &str) -> &str {
    let mut depth = 0i32;
    for (i, c) in from_name.char_indices() {
        match c {
            '(' | '[' => depth += 1,
            ')' | ']' => depth -= 1,
            '{' | ';' | '\n' if depth <= 0 => return &from_name[..i],
            _ => {}
        }
    }
    from_name
}

fn collapse(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Anywhere else:
/// - the locals at the cursor;
/// - the names the cursor's module sees;
/// - the primitive types;
/// - the keywords.
///
/// A local hides a module name of the same spelling.
fn names(analysis: &Analysis, path: &Path) -> Vec<lsp::CompletionItem> {
    let mut items = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    let defs = analysis.definitions.as_ref();
    for (name, ty) in &analysis.probe.locals {
        if seen.insert(name.clone()) {
            let detail = defs.map(|d| nova_typeck::display_ty(ty, d));
            items.push(item(name, Kind::VARIABLE, detail));
        }
    }
    if let Some(defs) = defs {
        let module = analysis
            .modules
            .iter()
            .position(|(_, p)| PathKey::of(p) == PathKey::of(path))
            .unwrap_or(0);
        for (name, entry) in defs.names_in_scope(ModuleId(module as u32)) {
            if seen.contains(&name) {
                continue;
            }
            let kind = match entry {
                ScopeEntry::Value(Res::Def(id)) => match defs.def(id).kind {
                    DefKind::Const { .. } => Kind::CONSTANT,
                    _ => Kind::FUNCTION,
                },
                ScopeEntry::Value(Res::Variant(..)) => Kind::ENUM_MEMBER,
                ScopeEntry::Value(Res::Builtin(_)) => Kind::FUNCTION,
                ScopeEntry::Type(id) => match defs.def(id).kind {
                    DefKind::Sum { .. } => Kind::ENUM,
                    _ => Kind::STRUCT,
                },
                ScopeEntry::Trait(_) => Kind::INTERFACE,
            };
            items.push(item(&name, kind, None));
        }
    }
    for ty in nova_resolver::RESERVED_TYPE_NAMES {
        if !seen.contains(ty) {
            items.push(item(ty, Kind::STRUCT, None));
        }
    }
    for keyword in nova_lexer::KEYWORDS {
        items.push(item(keyword, Kind::KEYWORD, None));
    }
    items
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_signature_stops_at_its_body_outside_parentheses() {
        assert_eq!(
            signature("push(mut self, x: T) {\n"),
            "push(mut self, x: T) "
        );
        assert_eq!(signature("fmt(self) -> String\n"), "fmt(self) -> String");
        assert_eq!(
            signature("map<U>(self, f: fn(T) -> U) -> Map<U> { x }"),
            "map<U>(self, f: fn(T) -> U) -> Map<U> "
        );
        assert_eq!(
            collapse("push(mut self,\n    x: T) "),
            "push(mut self, x: T)"
        );
    }

    #[test]
    fn strings_chars_and_comments_are_literal_places() {
        let text = "let s = \"ab\" // c\nlet t = 'x'\n";
        let at = |s: &str| text.find(s).unwrap() as u32;
        assert!(in_literal_or_comment(text, at("ab") + 1));
        assert!(in_literal_or_comment(text, at("\"ab") + 1));
        assert!(in_literal_or_comment(text, at("// c") + 3));
        assert!(!in_literal_or_comment(text, at("let s")));
        assert!(!in_literal_or_comment(text, at(" //")));
    }
}
