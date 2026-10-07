//! The self-check (spec
//! `docs/superpowers/specs/2026-10-07-phase-3-1-formatter-design.md` §5.5).

use std::ops::Range;

use nova_ast::item::{Import, ImportKind};
use nova_ast::{File, Item};
use nova_lexer::{Comment, CommentKind};

use crate::source::Source;
use crate::FormatError;

/// Check `output` against `input`: it must parse, to the same AST, with the
/// same comments. Any difference is [`FormatError::Internal`], naming the
/// first one.
pub(crate) fn check(input: &Source, output: &str) -> Result<(), FormatError> {
    let out = Source::parse(output, "<formatted>").map_err(|e| match e {
        FormatError::Syntax { rendered, .. } => {
            internal(format!("the output does not parse:\n{rendered}"))
        }
        other => other,
    })?;
    let (before, after) = (fingerprint(&input.file), fingerprint(&out.file));
    if before != after {
        return Err(internal(first_difference("the AST", &before, &after)));
    }
    let (before, after) = (comment_record(input), comment_record(&out));
    if before.ordered != after.ordered {
        let at = before
            .ordered
            .iter()
            .zip(&after.ordered)
            .position(|(a, b)| a != b)
            .unwrap_or(before.ordered.len().min(after.ordered.len()));
        return Err(internal(format!(
            "the comments changed at comment {at}: before {:?}, after {:?}",
            before.ordered.get(at),
            after.ordered.get(at)
        )));
    }
    if before.in_runs != after.in_runs {
        return Err(internal(
            "the comments inside a run of imports changed".to_owned(),
        ));
    }
    Ok(())
}

fn internal(first_difference: String) -> FormatError {
    FormatError::Internal { first_difference }
}

/// The AST without its spans, after the import sorting the printer does
/// (spec §6). Two inputs have the same fingerprint exactly when they are
/// the same program.
pub(crate) fn fingerprint(file: &File) -> String {
    let mut file = file.clone();
    for run in import_runs(&file) {
        let items = &mut file.items[run];
        items.sort_by(|a, b| import_key(&a.value).cmp(&import_key(&b.value)));
        for item in items {
            if let Item::Import(Import {
                kind: ImportKind::List(names),
                ..
            }) = &mut item.value
            {
                names.sort_by(|a, b| a.value.cmp(&b.value));
            }
        }
    }
    strip_spans(&format!("{file:?}"))
}

/// The index ranges of the runs of consecutive top-level imports.
pub(crate) fn import_runs(file: &File) -> Vec<Range<usize>> {
    let mut runs = Vec::new();
    let mut i = 0;
    while i < file.items.len() {
        if matches!(file.items[i].value, Item::Import(_)) {
            let start = i;
            while i < file.items.len() && matches!(file.items[i].value, Item::Import(_)) {
                i += 1;
            }
            runs.push(start..i);
        } else {
            i += 1;
        }
    }
    runs
}

/// What an import sorts by: its path, as `a::b`.
pub(crate) fn import_key(item: &Item) -> String {
    match item {
        Item::Import(import) => import
            .path
            .value
            .segments
            .iter()
            .map(|s| s.value.as_str())
            .collect::<Vec<_>>()
            .join("::"),
        _ => String::new(),
    }
}

/// `debug`, a `{:?}` rendering, without its `, span: Span { … }` fields.
pub(crate) fn strip_spans(debug: &str) -> String {
    const FIELD: &str = ", span: Span { ";
    let mut out = String::with_capacity(debug.len());
    let mut rest = debug;
    while let Some(at) = rest.find(FIELD) {
        out.push_str(&rest[..at]);
        let after = &rest[at + FIELD.len()..];
        rest = match after.find(" }") {
            Some(end) => &after[end + 2..],
            None => "",
        };
    }
    out.push_str(rest);
    out
}

/// A file's comments, each as its kind and its text without trailing
/// whitespace on any line. Those outside import runs are kept in order.
/// Those inside each run are sorted, because sorting the run moves comments
/// with their imports (spec §5.5).
struct CommentRecord {
    ordered: Vec<String>,
    in_runs: Vec<Vec<String>>,
}

fn comment_record(src: &Source) -> CommentRecord {
    let items = &src.file.items;
    // A run's region: from the end of the item before it to the start of
    // the item after it.
    let regions: Vec<(u32, u32)> = import_runs(&src.file)
        .into_iter()
        .map(|run| {
            let start = if run.start == 0 {
                0
            } else {
                items[run.start - 1].span.end
            };
            let end = items
                .get(run.end)
                .map_or(src.text.len() as u32, |next| next.span.start);
            (start, end)
        })
        .collect();
    let mut record = CommentRecord {
        ordered: Vec::new(),
        in_runs: vec![Vec::new(); regions.len()],
    };
    for c in &src.comments {
        let entry = comment_entry(src, c);
        match regions
            .iter()
            .position(|&(s, e)| c.span.start >= s && c.span.start < e)
        {
            Some(k) => record.in_runs[k].push(entry),
            None => record.ordered.push(entry),
        }
    }
    for run in &mut record.in_runs {
        run.sort();
    }
    record
}

fn comment_entry(src: &Source, c: &Comment) -> String {
    let kind = match c.kind {
        CommentKind::Line => "line",
        CommentKind::Block => "block",
    };
    let lines: Vec<&str> = src.slice(c.span).lines().map(str::trim_end).collect();
    format!("{kind}: {}", lines.join("\n"))
}

/// Where `before` and `after` first differ, with some text either side.
fn first_difference(what: &str, before: &str, after: &str) -> String {
    let at = before
        .bytes()
        .zip(after.bytes())
        .position(|(a, b)| a != b)
        .unwrap_or(before.len().min(after.len()));
    format!(
        "{what} changed: before …{}… after …{}…",
        around(before, at),
        around(after, at)
    )
}

/// The text of `s` within 60 bytes of `at`, cut at character boundaries.
fn around(s: &str, at: usize) -> &str {
    let mut lo = at.saturating_sub(60).min(s.len());
    while !s.is_char_boundary(lo) {
        lo -= 1;
    }
    let mut hi = (at + 60).min(s.len());
    while !s.is_char_boundary(hi) {
        hi += 1;
    }
    &s[lo..hi]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_spans_removes_every_span_field() {
        let debug =
            "Spanned { value: Ident(\"x\"), span: Span { start: 0, end: 1, file: FileId(0) } }";
        assert_eq!(strip_spans(debug), "Spanned { value: Ident(\"x\") }");
    }
}
