//! Organize imports' block (spec
//! `docs/superpowers/specs/2026-10-10-phase-3-4b-fixes-and-colour-design.md`
//! §6). The caller judges each import: its group, and what of it is
//! unused. The block is printed by the formatter itself (plan decision 10).

use nova_ast::item::ImportKind;
use nova_ast::Item;
use nova_diagnostics::lines;
use nova_lexer::Token;

use crate::source::Source;

/// One import, as the caller sees it to judge it.
pub struct ImportView<'a> {
    /// Its path, as `a::b`.
    pub path: String,
    /// Where the path's first segment starts.
    pub path_start: u32,
    /// The path's first segment.
    pub first: &'a str,
    /// Whether it is a glob, `import m`.
    pub glob: bool,
    /// A `{…}` list's names, each with where it starts.
    pub names: Vec<(&'a str, u32)>,
    /// Where every top-level import of the file starts and ends: an
    /// occurrence inside one is never a use (plan decision 20).
    pub imports: &'a [(u32, u32)],
}

/// Which group an import goes in (spec §6.2): dependencies first.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Group {
    Dependency,
    Module,
}

/// The caller's judgement of one import (spec §6.2, §6.4).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Verdict {
    pub group: Group,
    /// A glob nothing in the file uses.
    pub unused_glob: bool,
    /// The names of a list nothing in the file uses.
    pub unused_names: Vec<String>,
}

/// Replace bytes `start..end` of the text with `text`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextEdit {
    pub start: u32,
    pub end: u32,
    pub text: String,
}

/// One import, gathered for the block.
struct Entry {
    group: Group,
    path: String,
    /// `None` for a glob.
    names: Option<Vec<String>>,
    /// Its comment, doc and attribute lines, in order.
    lead: Vec<String>,
    /// A comment ending its line.
    trail: Option<String>,
}

/// The edit organizing `text`'s imports makes (spec §6): one, from the
/// first import to the last change. `None` when the text does not parse,
/// an import shares a line with other code or is an `import … as`, or
/// nothing would change (plan decision 10).
pub fn organize(text: &str, judge: &dyn Fn(&ImportView) -> Verdict) -> Option<Vec<TextEdit>> {
    let src = Source::parse(text, "<organize>").ok()?;
    let items = &src.file.items;
    let imports: Vec<usize> = (0..items.len())
        .filter(|&i| matches!(items[i].value, Item::Import(_)))
        .collect();
    if imports.is_empty() {
        return None;
    }
    let spans: Vec<(u32, u32)> = imports
        .iter()
        .map(|&i| (items[i].span.start, items[i].span.end))
        .collect();
    // The bytes each import owns: its lead lines through its line's end.
    let mut regions: Vec<(usize, usize)> = Vec::new();
    let mut entries: Vec<Entry> = Vec::new();
    for (k, &i) in imports.iter().enumerate() {
        let Item::Import(imp) = &items[i].value else {
            continue;
        };
        if matches!(imp.kind, ImportKind::Alias(_)) {
            return None;
        }
        let (start, end) = (items[i].span.start as usize, items[i].span.end as usize);
        let first_line = lines::line_start(text, start);
        if !text[first_line..start].trim_matches([' ', '\t']).is_empty() {
            return None;
        }
        let line_end = lines::next_line_start(text, end);
        let rest = text[end..line_end]
            .trim_end_matches(['\r', '\n'])
            .trim_matches([' ', '\t']);
        let trail = match rest {
            "" => None,
            r if r.starts_with("//") => Some(r.to_string()),
            _ => return None,
        };
        // After another import, every line between them is this one's lead
        // (spec §6.5); after anything else, the comments directly above.
        let lead_start = if k > 0 && imports[k - 1] + 1 == i {
            regions[k - 1].1
        } else {
            comments_above(text, first_line)
        };
        let mut lead: Vec<String> = text[lead_start..first_line]
            .lines()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty())
            .collect();
        // Its `///` docs and attributes: the item's text before `import`.
        let keyword = src.find_from(items[i].span.start, &Token::Import)? as usize;
        lead.extend(
            text[start..keyword]
                .lines()
                .map(|l| l.trim().to_string())
                .filter(|l| !l.is_empty()),
        );
        regions.push((lead_start, line_end));
        let segments = &imp.path.value.segments;
        let first = segments.first()?;
        let view = ImportView {
            path: segments
                .iter()
                .map(|s| s.value.as_str())
                .collect::<Vec<_>>()
                .join("::"),
            path_start: first.span.start,
            first: &first.value,
            glob: matches!(imp.kind, ImportKind::Simple),
            names: match &imp.kind {
                ImportKind::List(names) => names
                    .iter()
                    .map(|n| (n.value.as_str(), n.span.start))
                    .collect(),
                _ => Vec::new(),
            },
            imports: &spans,
        };
        let verdict = judge(&view);
        let (names, removed) = match &imp.kind {
            ImportKind::List(listed) => {
                let kept: Vec<String> = listed
                    .iter()
                    .map(|n| n.value.clone())
                    .filter(|n| !verdict.unused_names.contains(n))
                    .collect();
                let removed = !listed.is_empty() && kept.is_empty();
                (Some(kept), removed)
            }
            _ => (None, verdict.unused_glob),
        };
        if !removed {
            entries.push(Entry {
                group: verdict.group,
                path: view.path,
                names,
                lead,
                trail,
            });
        }
    }
    let nl = lines::ending(text);
    let block = render(merge(entries))?.replace('\n', nl);
    let out = assemble(text, &regions, &block, nl);
    if out == text {
        return None;
    }
    // One edit, from the first import to the last byte that changes.
    let start = regions[0].0;
    let room = (text.len() - start).min(out.len() - start);
    let mut same = 0;
    while same < room
        && text.as_bytes()[text.len() - 1 - same] == out.as_bytes()[out.len() - 1 - same]
    {
        same += 1;
    }
    while !text.is_char_boundary(text.len() - same) {
        same -= 1;
    }
    Some(vec![TextEdit {
        start: start as u32,
        end: (text.len() - same) as u32,
        text: out[start..out.len() - same].to_string(),
    }])
}

/// Where the comment lines directly above the line starting at `line`
/// start, with no blank line between (spec §6.5).
fn comments_above(text: &str, line: usize) -> usize {
    let mut at = line;
    while at > 0 {
        let begin = lines::line_start(text, at - 1);
        if lines::is_comment_line(&text[begin..at]) {
            at = begin;
        } else {
            break;
        }
    }
    at
}

/// Imports of one path become one (spec §6.3): a glob absorbs a list
/// beside it, and lists merge. Sorted by group, then path.
fn merge(entries: Vec<Entry>) -> Vec<Entry> {
    let mut out: Vec<Entry> = Vec::new();
    for e in entries {
        match out.iter_mut().find(|m| m.path == e.path) {
            Some(m) => {
                m.lead.extend(e.lead);
                m.lead.extend(e.trail);
                let both_lists = m.names.is_some() && e.names.is_some();
                if both_lists {
                    if let (Some(a), Some(b)) = (m.names.as_mut(), e.names) {
                        a.extend(b);
                    }
                } else {
                    m.names = None;
                }
            }
            None => out.push(e),
        }
    }
    for e in &mut out {
        if let Some(names) = &mut e.names {
            names.sort();
            names.dedup();
        }
    }
    out.sort_by(|a, b| (a.group, &a.path).cmp(&(b.group, &b.path)));
    out
}

/// The block: each group's imports, a blank line between groups, printed
/// by the formatter (plan decision 10). Empty when no import is left.
fn render(entries: Vec<Entry>) -> Option<String> {
    if entries.is_empty() {
        return Some(String::new());
    }
    let mut block = String::new();
    for (k, e) in entries.iter().enumerate() {
        if k > 0 && entries[k - 1].group != e.group {
            block.push('\n');
        }
        for l in &e.lead {
            block.push_str(l);
            block.push('\n');
        }
        block.push_str("import ");
        block.push_str(&e.path);
        if let Some(names) = &e.names {
            block.push_str("::{");
            block.push_str(&names.join(", "));
            block.push('}');
        }
        if let Some(t) = &e.trail {
            block.push(' ');
            block.push_str(t);
        }
        block.push('\n');
    }
    crate::format_named(&block, "<organize>").ok()
}

/// `text` with the block at the first import's place and every other
/// import's region gone. A removed import's blank line goes: the one after
/// it, or, at the text's end, the one before it (spec §6.2).
fn assemble(text: &str, regions: &[(usize, usize)], block: &str, nl: &str) -> String {
    let blank = format!("{nl}{nl}");
    let mut out = String::from(&text[..regions[0].0]);
    for (k, &(_, end)) in regions.iter().enumerate() {
        let piece = if k == 0 { block } else { "" };
        out.push_str(piece);
        let gap_end = regions.get(k + 1).map_or(text.len(), |r| r.0);
        let mut gap = &text[end..gap_end];
        if piece.is_empty() && (out.is_empty() || out.ends_with(&blank)) {
            if let Some(rest) = gap.strip_prefix(nl) {
                gap = rest;
            } else if gap.is_empty() && gap_end == text.len() && !out.is_empty() {
                out.truncate(out.len() - nl.len());
            }
        }
        out.push_str(gap);
    }
    out
}
