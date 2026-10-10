//! Organize imports in the server: each import's group, and what of it is
//! unused (spec
//! `docs/superpowers/specs/2026-10-10-phase-3-4b-fixes-and-colour-design.md`
//! §6.2, §6.4; plan decision 11).

use std::collections::HashMap;
use std::path::Path;

use lsp_types as lsp;
use nova_diagnostics::{FileId, LineIndex, Severity, Span};
use nova_driver::Analysis;
use nova_fmt::{Group, ImportView, TextEdit, Verdict};
use nova_resolver::{Definitions, Index, ModuleId, Occurrence, Target};

use crate::analysis::Answer;
use crate::convert;
use crate::navigate::Locator;

/// The edits organizing `answer`'s file makes, or `None` when organizing
/// changes nothing.
pub fn edits(answer: &Answer) -> Option<Vec<TextEdit>> {
    let a = &answer.analysis;
    let text = a.db.get_source(answer.file)?;
    let removal = may_remove(a, answer.file);
    nova_fmt::organize(text, &|view| verdict(a, answer.file, view, removal))
}

/// The organize imports action for `answer`'s file (spec §5, §6).
// `WorkspaceEdit::changes` is a `HashMap` keyed by `lsp::Uri`, whose
// parsed form caches into a `Cell`; the key is never changed here.
#[allow(clippy::mutable_key_type)]
pub fn action(answer: &Answer, uri_of: &dyn Fn(&Path) -> String) -> Option<lsp::CodeAction> {
    let edits = edits(answer)?;
    let a = &answer.analysis;
    let text = a.db.get_source(answer.file)?;
    let uri = Locator::new(a, uri_of).uri(answer.file)?;
    let lines = LineIndex::new(text);
    let edits: Vec<lsp::TextEdit> = edits
        .into_iter()
        .map(|e| lsp::TextEdit {
            range: convert::range(&lines, e.start, e.end),
            new_text: e.text,
        })
        .collect();
    Some(lsp::CodeAction {
        title: "Organize imports".to_string(),
        kind: Some(lsp::CodeActionKind::SOURCE_ORGANIZE_IMPORTS),
        edit: Some(lsp::WorkspaceEdit {
            changes: Some(HashMap::from([(uri, edits)])),
            ..Default::default()
        }),
        ..Default::default()
    })
}

/// Whether unused imports may go (spec §6.4): no error in `file`, and no
/// lex or parse error in any file, since `keep_going` filters out the
/// E0001s a dropped item causes.
fn may_remove(a: &Analysis, file: FileId) -> bool {
    !a.diagnostics.iter().any(|d| {
        matches!(d.code.as_str(), "L0001" | "P0001")
            || (d.severity == Severity::Error && d.labels.iter().any(|l| l.span.file == file))
    })
}

/// One import's group and what of it is unused (spec §6.2, §6.4).
fn verdict(a: &Analysis, file: FileId, view: &ImportView, removal: bool) -> Verdict {
    let first = Span::new(
        view.path_start,
        view.path_start + view.first.len() as u32,
        file,
    );
    let module = a.index.as_ref().and_then(|index| module_at(index, first));
    let group = match module {
        Some(m) if !beside(a, file, m) => Group::Dependency,
        _ => Group::Module,
    };
    let kept = Verdict {
        group,
        unused_glob: false,
        unused_names: Vec::new(),
    };
    let (Some(m), Some(index), Some(defs), true) =
        (module, a.index.as_ref(), a.definitions.as_ref(), removal)
    else {
        return kept;
    };
    if view.glob {
        let used = index.occurrences.iter().any(|o| {
            o.span.file == file
                && !inside_an_import(o, view.imports)
                && declared_in(a, defs, &o.target, m)
        });
        Verdict {
            unused_glob: !used,
            ..kept
        }
    } else {
        let unused_names = view
            .names
            .iter()
            .filter(|(name, start)| {
                let at = Span::new(*start, *start + name.len() as u32, file);
                !list_name_used(index, at, view.imports)
            })
            .map(|(name, _)| name.to_string())
            .collect();
        Verdict {
            unused_names,
            ..kept
        }
    }
}

/// The module the import whose first segment is at `first` names.
fn module_at(index: &Index, first: Span) -> Option<ModuleId> {
    index.occurrences.iter().find_map(|o| match o.target {
        Target::Module(m) if o.span == first => Some(m),
        _ => None,
    })
}

/// Whether module `m`'s file is in `file`'s directory (plan decision 11).
fn beside(a: &Analysis, file: FileId, m: ModuleId) -> bool {
    let dir = |f: FileId| {
        a.modules
            .iter()
            .find(|(id, _)| *id == f)
            .and_then(|(_, p)| p.parent().map(Path::to_path_buf))
    };
    let Some((target, _)) = a.modules.get(m.0 as usize) else {
        return false;
    };
    dir(*target).is_some() && dir(*target) == dir(file)
}

/// Whether `o` lies inside one of the file's imports, `imports` (plan
/// decision 20): an import's own occurrences are never uses.
fn inside_an_import(o: &Occurrence, imports: &[(u32, u32)]) -> bool {
    imports
        .iter()
        .any(|&(start, end)| o.span.start >= start && o.span.end <= end)
}

/// Whether the list name at `at` is used (spec §6.4): an occurrence in its
/// file, outside every import, targets what it binds. A call of a trait's
/// method uses the trait, and a variant its sum type.
fn list_name_used(index: &Index, at: Span, imports: &[(u32, u32)]) -> bool {
    let bound: Vec<Target> = index
        .occurrences
        .iter()
        .filter(|o| o.span == at)
        .map(|o| o.target)
        .collect();
    index
        .occurrences
        .iter()
        .filter(|o| o.span.file == at.file && !inside_an_import(o, imports))
        .any(|o| {
            bound.iter().any(|b| match (*b, o.target) {
                (b, t) if b == t => true,
                (Target::Def(t), Target::TraitMethod(u, _)) => t == u,
                (Target::Def(s), Target::Variant(u, _)) => s == u,
                _ => false,
            })
        })
}

/// Whether `target` is declared in module `m`'s file: an item, a variant
/// or field of its types, or a method of its traits.
fn declared_in(a: &Analysis, defs: &Definitions, target: &Target, m: ModuleId) -> bool {
    let Some((file, _)) = a.modules.get(m.0 as usize) else {
        return false;
    };
    let id = match *target {
        Target::Def(id)
        | Target::Variant(id, _)
        | Target::Field(id, _)
        | Target::TraitMethod(id, _) => id,
        _ => return false,
    };
    defs.def(id).span.file == *file
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace::Overlay;

    /// `files`' first, analysed as the server would and organized; `None`
    /// when organizing changes nothing.
    fn organized(name: &str, files: &[(&str, &str)]) -> Option<String> {
        let dir = std::env::temp_dir().join(format!("nova-lsp-organize-{name}"));
        let mut overlay = Overlay::default();
        for (file, text) in files {
            overlay = overlay.with(&dir.join(file), text.to_string());
        }
        let options = crate::analysis::options(None, true);
        let answer = crate::analysis::answering(&dir.join(files[0].0), &overlay, &options)
            .expect("an answer");
        let mut text = answer
            .analysis
            .db
            .get_source(answer.file)
            .unwrap()
            .to_string();
        for e in edits(&answer)?.iter().rev() {
            text.replace_range(e.start as usize..e.end as usize, &e.text);
        }
        Some(text)
    }

    const GEOMETRY: &str =
        "pub fn origin() -> Int {\n    1\n}\n\npub fn manhattan() -> Int {\n    2\n}\n";

    #[test]
    fn a_list_name_unused_goes_and_a_used_one_stays() {
        assert_eq!(
            organized(
                "list",
                &[
                    ("main.nova", "import geometry::{manhattan, origin}\n\nfn main() {\n    let o = origin()\n}\n"),
                    ("geometry.nova", GEOMETRY),
                ]
            )
            .as_deref(),
            Some("import geometry::{origin}\n\nfn main() {\n    let o = origin()\n}\n")
        );
    }

    #[test]
    fn a_glob_unused_goes_and_a_used_one_stays() {
        assert_eq!(
            organized(
                "glob",
                &[
                    (
                        "main.nova",
                        "import geometry\nimport shout\n\nfn main() {\n    let o = origin()\n}\n"
                    ),
                    ("geometry.nova", GEOMETRY),
                    ("shout.nova", "pub fn shout() -> String {\n    \"!\"\n}\n"),
                ]
            )
            .as_deref(),
            Some("import geometry\n\nfn main() {\n    let o = origin()\n}\n")
        );
    }

    #[test]
    fn a_trait_used_only_through_its_methods_stays() {
        let loud = "pub trait Loud {\n    fn shout(self) -> String\n}\n\npub record T { s: String }\n\nimpl Loud for T {\n    fn shout(self) -> String {\n        self.s\n    }\n}\n\npub fn make() -> T {\n    T { s: \"hi\" }\n}\n";
        assert_eq!(
            organized(
                "trait",
                &[
                    (
                        "main.nova",
                        "import loud::{Loud, make}\n\nfn main() {\n    let s = make().shout()\n}\n"
                    ),
                    ("loud.nova", loud),
                ]
            ),
            None
        );
    }

    #[test]
    fn a_sum_type_used_only_through_its_variant_stays() {
        assert_eq!(
            organized(
                "variant",
                &[
                    (
                        "main.nova",
                        "import kinds::{Empty, Shape}\n\nfn main() {\n    let e = Empty\n}\n"
                    ),
                    (
                        "kinds.nova",
                        "pub type Shape =\n  | Circle(Int)\n  | Empty\n"
                    ),
                ]
            ),
            None
        );
    }

    #[test]
    fn names_of_one_import_are_not_uses_of_each_other() {
        // Plan decision 20: `Empty`'s own occurrence, in the import, is a
        // variant of `Shape`, and must not keep it.
        assert_eq!(
            organized(
                "one-import",
                &[
                    (
                        "main.nova",
                        "import kinds::{Empty, Shape}\n\nfn main() {}\n"
                    ),
                    (
                        "kinds.nova",
                        "pub type Shape =\n  | Circle(Int)\n  | Empty\n"
                    ),
                ]
            )
            .as_deref(),
            Some("fn main() {}\n")
        );
    }

    #[test]
    fn an_error_in_the_file_keeps_unused_imports() {
        assert_eq!(
            organized(
                "error",
                &[
                    ("main.nova", "import geometry::{manhattan, origin}\n\nfn main() {\n    let o = origin()\n    let n: Int = \"no\"\n}\n"),
                    ("geometry.nova", GEOMETRY),
                ]
            ),
            None
        );
    }

    #[test]
    fn a_parse_error_elsewhere_keeps_unused_imports() {
        // Spec §6.4: a dropped item's E0001s are filtered out, so the parse
        // error must block removal itself.
        let broken = "pub fn origin() -> Int {\n    1\n}\n\npub fn manhattan( -> Int {\n    2\n}\n";
        assert_eq!(
            organized(
                "parse-error",
                &[
                    ("main.nova", "import geometry::{manhattan, origin}\n\nfn main() {\n    let o = origin()\n    let m = manhattan()\n}\n"),
                    ("geometry.nova", broken),
                ]
            ),
            None
        );
    }
}
