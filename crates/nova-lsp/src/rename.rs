//! Prepare rename and rename (spec
//! `docs/superpowers/specs/2026-10-10-phase-3-4a-navigation-design.md`
//! §5.4, §5.5).

use std::collections::HashMap;
use std::path::Path;

use lsp_types as lsp;
use nova_diagnostics::{FileId, LineIndex, Span};
use nova_driver::Analysis;
use nova_lexer::Token;
use nova_pm::PackageId;
use nova_resolver::{Occurrence, Role, Target};

use crate::analysis::Answer;
use crate::workspace::{Overlay, PathKey};
use crate::{convert, navigate, std_cache};

/// Why a rename is refused: a `RequestFailed` error's message.
#[derive(Debug)]
pub struct Refused(pub String);

/// A name that may be renamed: where it is, how it is spelt, and every
/// target the rename covers.
struct Found<'a> {
    at: &'a Occurrence,
    old: String,
    family: Vec<Target>,
}

/// Prepare rename at byte `offset` of `path` (spec §5.4): the name's range
/// and spelling, `None` for no name, or a refusal.
pub fn prepare(
    answer: &Answer,
    path: &Path,
    offset: u32,
) -> Result<Option<lsp::PrepareRenameResponse>, Refused> {
    let Some(found) = renameable(answer, path, offset)? else {
        return Ok(None);
    };
    let Some(source) = answer.analysis.db.get_source(answer.file) else {
        return Ok(None);
    };
    let range = convert::range(
        &LineIndex::new(source),
        found.at.span.start,
        found.at.span.end,
    );
    Ok(Some(lsp::PrepareRenameResponse::RangeWithPlaceholder {
        range,
        placeholder: found.old,
    }))
}

fn renameable<'a>(
    answer: &'a Answer,
    path: &Path,
    offset: u32,
) -> Result<Option<Found<'a>>, Refused> {
    let a = &answer.analysis;
    let (Some(index), Some(defs)) = (a.index.as_ref(), a.definitions.as_ref()) else {
        return Ok(None);
    };
    let Some(at) = index.at(defs, answer.file, offset) else {
        return Ok(None);
    };
    let old =
        a.db.get_source(at.span.file)
            .and_then(|s| s.get(at.span.start as usize..at.span.end as usize))
            .unwrap_or("")
            .to_string();
    let refuse = |why: String| Err(Refused(why));
    if nova_lexer::KEYWORDS.contains(&old.as_str()) {
        return refuse(format!("`{old}` is a keyword and cannot be renamed"));
    }
    if std_cache::in_std_cache(path) || in_registry(path) {
        return refuse(format!("`{old}` is in nova's cache and cannot be renamed"));
    }
    match at.target {
        Target::Builtin(_) | Target::BuiltinMethod(_) | Target::Primitive(_) => {
            return refuse(format!("`{old}` is built in and cannot be renamed"));
        }
        Target::Module(_) => {
            return refuse(
                "a module or package is renamed by renaming its file or its `nova.toml`"
                    .to_string(),
            );
        }
        _ => {}
    }
    let family = index.family(&at.target);
    for target in &family {
        let Some(decl) = index.declaration(target) else {
            return refuse(format!("`{old}` has no declaration to rename"));
        };
        match owner(a, decl.span.file) {
            Owner::Own => {}
            Owner::Std => {
                return refuse(format!("`{old}` is declared in std and cannot be renamed"))
            }
            Owner::Dependency(name) => {
                return refuse(format!(
                    "`{old}` is declared in the dependency `{name}` and cannot be renamed"
                ));
            }
            Owner::Elsewhere => {
                return refuse(format!(
                    "`{old}` is not declared in this project and cannot be renamed"
                ));
            }
        }
    }
    Ok(Some(Found { at, old, family }))
}

fn in_registry(path: &Path) -> bool {
    nova_pm::registry_dir().is_some_and(|r| PathKey::of(path).is_under(&PathKey::of(&r)))
}

enum Owner {
    Own,
    Std,
    Dependency(String),
    Elsewhere,
}

/// Whose a declaration's file is (plan decision 15).
fn owner(a: &Analysis, file: FileId) -> Owner {
    if a.db.get_name(file).is_some_and(|n| n.starts_with("<std/")) {
        return Owner::Std;
    }
    let Some(i) = a.modules.iter().position(|(f, _)| *f == file) else {
        return Owner::Elsewhere;
    };
    match a.module_packages.get(i).copied().flatten() {
        None | Some(PackageId(0)) => Owner::Own,
        Some(pid) => Owner::Dependency(
            a.graph
                .as_ref()
                .map(|g| g.package(pid).name.clone())
                .unwrap_or_default(),
        ),
    }
}

/// One replacement: `start..end` of `file` becomes `text`, which holds the
/// new name at `name_at`.
// Task 11's check is the first reader of `name_at` and `role`.
#[allow(dead_code)]
pub struct Edit {
    pub file: FileId,
    pub start: u32,
    pub end: u32,
    pub text: String,
    pub name_at: u32,
    pub role: Role,
}

/// Rename the name at byte `offset` of `path` to `new` (spec §5.4).
/// `Ok(None)` when there is no name there: an empty result, never an
/// error (spec §5).
pub fn rename(
    answer: &Answer,
    path: &Path,
    offset: u32,
    new: &str,
    overlay: &Overlay,
    uri_of: &dyn Fn(&Path) -> String,
) -> Result<Option<lsp::WorkspaceEdit>, Refused> {
    let Some(found) = renameable(answer, path, offset)? else {
        return Ok(None);
    };
    check_new_name(new)?;
    if new == found.old {
        return Ok(Some(lsp::WorkspaceEdit::default()));
    }
    let edits = plan_edits(&answer.analysis, &found, new);
    let _ = overlay; // Task 11's check reads it.
    Ok(Some(workspace_edit(&answer.analysis, &edits, uri_of)))
}

/// Spec §5.4: one identifier, not `_`, and not a built-in type's name.
fn check_new_name(new: &str) -> Result<(), Refused> {
    let (tokens, errors) = nova_lexer::lex(new, FileId::DUMMY);
    let tokens: Vec<&Token> = tokens
        .iter()
        .map(|t| &t.value)
        .filter(|t| !matches!(t, Token::Eof))
        .collect();
    let one_name = errors.is_empty() && matches!(tokens.as_slice(), [Token::Ident(s)] if s == new);
    if !one_name {
        return Err(Refused(format!(
            "`{new}` is not a name: a new name is one identifier, and not a keyword"
        )));
    }
    if new == "_" {
        return Err(Refused("`_` cannot be a name".to_string()));
    }
    if nova_resolver::RESERVED_TYPE_NAMES.contains(&new) {
        return Err(Refused(format!("`{new}` is a built-in type's name")));
    }
    Ok(())
}

/// The edits renaming `found` to `new` makes, in the spec's one order. A
/// shorthand `{ x }` is written out (spec §5.4).
fn plan_edits(a: &Analysis, found: &Found, new: &str) -> Vec<Edit> {
    let Some(index) = a.index.as_ref() else {
        return Vec::new();
    };
    let mut renamed: Vec<&Occurrence> = index
        .occurrences
        .iter()
        .filter(|o| found.family.contains(&o.target))
        .collect();
    navigate::sort_by_place(a, &mut renamed);
    renamed.dedup_by_key(|o| o.span);
    renamed
        .into_iter()
        .map(|o| {
            let (text, name_at) = match (o.shorthand, o.target) {
                (true, Target::Field(..)) => (format!("{new}: {}", found.old), 0),
                (true, _) => (format!("{}: {new}", found.old), found.old.len() as u32 + 2),
                (false, _) => (new.to_string(), 0),
            };
            Edit {
                file: o.span.file,
                start: o.span.start,
                end: o.span.end,
                text,
                name_at,
                role: o.role,
            }
        })
        .collect()
}

/// The edits as LSP's, each file under the URI `uri_of` gives it.
fn workspace_edit(
    a: &Analysis,
    edits: &[Edit],
    uri_of: &dyn Fn(&Path) -> String,
) -> lsp::WorkspaceEdit {
    let locator = navigate::Locator::new(a, uri_of);
    let mut changes: HashMap<lsp::Uri, Vec<lsp::TextEdit>> = HashMap::new();
    for e in edits {
        let span = Span::new(e.start, e.end, e.file);
        let (Some(uri), Some(range)) = (locator.uri(e.file), locator.range(span)) else {
            continue;
        };
        changes.entry(uri).or_default().push(lsp::TextEdit {
            range,
            new_text: e.text.clone(),
        });
    }
    lsp::WorkspaceEdit {
        changes: Some(changes),
        ..Default::default()
    }
}
