//! Hover (spec
//! `docs/superpowers/specs/2026-10-10-phase-3-4a-navigation-design.md`
//! §5.1). Declarations and docs are read from the source text (plan
//! decision 6).

use lsp_types as lsp;
use nova_diagnostics::LineIndex;
use nova_driver::Analysis;
use nova_resolver::{DefKind, Target};

use crate::analysis::Answer;
use crate::convert;

/// The hover at byte `offset` of the request's file.
pub fn hover(answer: &Answer, offset: u32) -> Option<lsp::Hover> {
    let a = &answer.analysis;
    let o = a
        .index
        .as_ref()?
        .at(a.definitions.as_ref()?, answer.file, offset)?;
    let mut value = format!("```nova\n{}\n```", declaration(a, &o.target)?);
    let docs = docs_of(a, &o.target);
    if !docs.is_empty() {
        value.push_str("\n\n");
        value.push_str(&docs);
    }
    let source = a.db.get_source(answer.file)?;
    let range = convert::range(&LineIndex::new(source), o.span.start, o.span.end);
    Some(lsp::Hover {
        contents: lsp::HoverContents::Markup(lsp::MarkupContent {
            kind: lsp::MarkupKind::Markdown,
            value,
        }),
        range: Some(range),
    })
}

/// The one line hover shows for `target` (spec §5.1's table).
pub fn declaration(a: &Analysis, target: &Target) -> Option<String> {
    let index = a.index.as_ref()?;
    let defs = a.definitions.as_ref()?;
    match target {
        Target::Builtin(b) => return Some(nova_typeck::builtin_text(*b, defs)),
        Target::BuiltinMethod(name) => return Some(format!("fn {name}(self) -> Int")),
        Target::Primitive(name) => return Some(format!("type {name}")),
        Target::Module(m) => return Some(module_line(a, m.0 as usize)),
        _ => {}
    }
    let decl = index.declaration(target)?.span;
    let text = a.db.get_source(decl.file)?;
    let at = decl.start as usize;
    let name = text.get(at..decl.end as usize)?;
    Some(match target {
        Target::Local(span) => local_line(text, at, name, index.types.get(span)),
        Target::TypeParam(_) => collapse(&text[at..end_of(text, at, &[',', '>'], true)]),
        Target::Field(..) => collapse(&text[at..end_of(text, at, &[',', '}', '\n'], true)]),
        Target::Variant(..) => collapse(&text[at..end_of(text, at, &['|', '\n'], false)]),
        Target::TraitMethod(..) => item_line(text, at, &['{', ';', '}', '\n']),
        Target::Def(id) => match &defs.def(*id).kind {
            DefKind::Fn { .. } | DefKind::Method { .. } => item_line(text, at, &['{', ';']),
            DefKind::ExternFn { .. }
            | DefKind::AssocType { .. }
            | DefKind::Record { .. }
            | DefKind::Trait { .. } => item_line(text, at, &['{', ';', '}', '\n']),
            DefKind::Sum { .. } | DefKind::Const { .. } => {
                item_line(text, at, &['=', '{', ';', '}', '\n'])
            }
        },
        _ => name.to_string(),
    })
}

/// A declaration from its item's first token to the first of `stops`
/// outside brackets, on one line. The item starts after the last `{`, `;`
/// or `}` on the name's line, so a method written on its trait's or
/// impl's line is shown alone. `pub`, `async` and a `where` clause are
/// kept.
fn item_line(text: &str, at: usize, stops: &[char]) -> String {
    let line_start = text[..at].rfind('\n').map_or(0, |i| i + 1);
    let item_start = text[line_start..at]
        .rfind(|c: char| matches!(c, '{' | ';' | '}'))
        .map_or(line_start, |i| line_start + i + 1);
    let lead = text[item_start..at].len() - text[item_start..at].trim_start().len();
    let start = item_start + lead;
    collapse(&text[start..end_of(text, start, stops, false)])
}

/// The first of `stops` at bracket depth 0 from `from`, or the text's end.
/// With `angles`, `<` and `>` count as brackets too.
fn end_of(text: &str, from: usize, stops: &[char], angles: bool) -> usize {
    let mut depth = 0i32;
    for (i, c) in text[from..].char_indices() {
        if depth <= 0 && stops.contains(&c) {
            return from + i;
        }
        match c {
            '(' | '[' => depth += 1,
            ')' | ']' => depth -= 1,
            '<' if angles => depth += 1,
            '>' if angles => depth -= 1,
            _ => {}
        }
    }
    text.len()
}

fn collapse(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// `let x: T`, `let mut x: T`, or `let x` when the type is unknown. A
/// parameter no body binds shows its annotation as written.
fn local_line(text: &str, at: usize, name: &str, ty: Option<&String>) -> String {
    let before = text[..at].trim_end();
    let mutable = before
        .strip_suffix("mut")
        .is_some_and(|rest| !rest.ends_with(|c: char| c.is_ascii_alphanumeric() || c == '_'));
    let head = if mutable {
        format!("let mut {name}")
    } else {
        format!("let {name}")
    };
    if let Some(ty) = ty {
        return format!("{head}: {ty}");
    }
    let after_name = at + name.len();
    let rest = text[after_name..].trim_start();
    if let Some(annotation) = rest.strip_prefix(':') {
        let from = text.len() - annotation.len();
        let end = end_of(text, from, &[',', ')', '=', '{', ';', '\n'], true);
        return format!("{head}: {}", collapse(&text[from..end]));
    }
    head
}

/// `module m`, or `package <name> <version>` for a package's library.
fn module_line(a: &Analysis, m: usize) -> String {
    let Some((_, path)) = a.modules.get(m) else {
        return "module".to_string();
    };
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("?");
    if stem == "lib" {
        if let (Some(Some(pid)), Some(graph)) = (a.module_packages.get(m), a.graph.as_ref()) {
            let package = &graph.package(*pid).manifest.package;
            return format!("package {} {}", package.name, package.version);
        }
    }
    format!("module {stem}")
}

/// The `///` docs of `target`'s declaration (spec §5.1).
fn docs_of(a: &Analysis, target: &Target) -> String {
    if !matches!(
        target,
        Target::Def(_) | Target::Field(..) | Target::Variant(..) | Target::TraitMethod(..)
    ) {
        return String::new();
    }
    let Some(decl) = a.index.as_ref().and_then(|i| i.declaration(target)) else {
        return String::new();
    };
    let Some(text) = a.db.get_source(decl.span.file) else {
        return String::new();
    };
    let at = decl.span.start as usize;
    // A field or variant has docs only when it starts its line; one on its
    // record's or sum's own line would take the item's.
    if matches!(target, Target::Field(..) | Target::Variant(..)) {
        let line_start = text[..at].rfind('\n').map_or(0, |i| i + 1);
        if !text[line_start..at]
            .chars()
            .all(|c| c.is_whitespace() || c == '|')
        {
            return String::new();
        }
    }
    docs_above(text, at)
}

/// The `///` lines directly above the line holding byte `at`, attributes
/// (`@…`) skipped, each without `///` and one following space.
fn docs_above(text: &str, at: usize) -> String {
    let mut lines: Vec<&str> = Vec::new();
    let mut end = text[..at].rfind('\n').map_or(0, |i| i + 1);
    while end > 0 {
        let start = text[..end - 1].rfind('\n').map_or(0, |i| i + 1);
        let line = text[start..end - 1].trim_end_matches('\r').trim();
        match line.strip_prefix("///") {
            // `////` is an ordinary comment.
            Some(doc) if !doc.starts_with('/') => lines.push(doc.strip_prefix(' ').unwrap_or(doc)),
            Some(_) => break,
            None if line.starts_with('@') => {}
            None => break,
        }
        end = start;
    }
    lines.reverse();
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn docs_above_skip_attributes_and_stop_at_code() {
        let text = "fn a() {}\n/// One.\n///\n/// Two.\n@test\nfn b() {}\n";
        let at = text.find("fn b").unwrap();
        assert_eq!(docs_above(text, at), "One.\n\nTwo.");
        let at = text.find("fn a").unwrap();
        assert_eq!(docs_above(text, at), "");
        let crlf = "//// not a doc\r\n/// Doc.\r\nfn c() {}\r\n";
        assert_eq!(docs_above(crlf, crlf.find("fn c").unwrap()), "Doc.");
    }

    #[test]
    fn an_item_line_keeps_pub_async_and_where_and_stops_at_its_body() {
        let text = "    pub async fn f<T>(x: T) -> T\n    where T: Show\n    { x }\n";
        let at = text.find("f<").unwrap();
        assert_eq!(
            item_line(text, at, &['{', ';']),
            "pub async fn f<T>(x: T) -> T where T: Show"
        );
    }

    #[test]
    fn an_item_on_its_containers_line_starts_at_the_item() {
        let text =
            "trait Show { fn show(self) -> String }\nimpl Show for A { fn show(self) -> String { \"a\" } }\n";
        let at = text.find("show(").unwrap();
        assert_eq!(
            item_line(text, at, &['{', ';', '}', '\n']),
            "fn show(self) -> String"
        );
        let at = text.rfind("show(").unwrap();
        assert_eq!(item_line(text, at, &['{', ';']), "fn show(self) -> String");
    }

    #[test]
    fn a_fields_type_keeps_its_generic_arguments() {
        let text = "record S { m: Map<String, Int>, n: Int }";
        let at = text.find("m:").unwrap();
        assert_eq!(
            collapse(&text[at..end_of(text, at, &[',', '}', '\n'], true)]),
            "m: Map<String, Int>"
        );
    }

    #[test]
    fn a_type_parameters_bounds_stop_at_its_comma_or_angle() {
        let text = "fn f<T: Into<Vec<Int>>, U>() {}";
        let at = text.find("T:").unwrap();
        assert_eq!(
            collapse(&text[at..end_of(text, at, &[',', '>'], true)]),
            "T: Into<Vec<Int>>"
        );
        let at = text.find("U>").unwrap();
        assert_eq!(
            collapse(&text[at..end_of(text, at, &[',', '>'], true)]),
            "U"
        );
    }

    #[test]
    fn a_local_without_a_type_shows_its_annotation() {
        let text = "fn scaled(self, factor: Int) -> Int";
        let at = text.find("factor").unwrap();
        assert_eq!(local_line(text, at, "factor", None), "let factor: Int");
        let text = "fn f(mut x: [Int])";
        let at = text.find("x:").unwrap();
        assert_eq!(local_line(text, at, "x", None), "let mut x: [Int]");
        assert_eq!(local_line("let y = 1", 4, "y", None), "let y");
    }
}
