//! The corpus gate (spec
//! `docs/superpowers/specs/2026-10-07-phase-3-1-formatter-design.md` §9.4).
//!
//! Every `.nova` file in the repository that parses must format to the same
//! program and the same comments, and formatting the output again must
//! change nothing. These checks are written here, apart from the library's
//! own self-check (§5.5), so that a broken self-check cannot pass them.

use std::path::{Path, PathBuf};

use nova_ast::item::ImportKind;
use nova_ast::Item;
use nova_diagnostics::FileDb;
use nova_lexer::CommentKind;

/// The files expected not to parse:
/// - the parser's `async` fixture, which uses a `::<User>` turbofish;
/// - the VS Code smoke test's `complete.nova`, which stops after `p.` so
///   that completion is tested mid-edit (spec
///   `docs/superpowers/specs/2026-10-08-phase-3-2-lsp-core-design.md` §7.6).
const EXPECTED_SKIP: [&str; 2] = [
    "crates/nova-parser/tests/fixtures/async.nova",
    "tools/vscode-nova/test/fixture/src/complete.nova",
];

/// The gate's floor, set in 3.1: the 168 tracked files then, less the one
/// that did not parse.
const AT_LEAST: usize = 167;

/// The repository, two levels above this crate.
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

/// Every `.nova` file under `dir`, skipping `target/`, directories whose
/// names begin with `.`, and symbolic links.
fn nova_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let mut entries: Vec<_> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap())
        .collect();
    entries.sort_by_key(|e| e.file_name());
    for e in entries {
        let name = e.file_name().to_string_lossy().into_owned();
        let kind = e.file_type().unwrap();
        if kind.is_dir() {
            if name != "target" && !name.starts_with('.') {
                nova_files(&e.path(), out);
            }
        } else if kind.is_file() && name.ends_with(".nova") {
            out.push(e.path());
        }
    }
}

/// `debug`, a `{:?}` rendering, without its `, span: Span { … }` fields.
fn without_spans(debug: &str) -> String {
    const FIELD: &str = ", span: Span { ";
    let mut out = String::new();
    let mut rest = debug;
    while let Some(at) = rest.find(FIELD) {
        out.push_str(&rest[..at]);
        let after = &rest[at + FIELD.len()..];
        rest = after.find(" }").map_or("", |end| &after[end + 2..]);
    }
    out.push_str(rest);
    out
}

/// What must survive formatting, or `None` if `text` does not lex and
/// parse: the AST without spans, with each run of top-level imports sorted
/// as the formatter sorts it (spec §6), and the comments, each as its kind
/// and its text without trailing whitespace on any line. The comments
/// inside an import run are compared as a set, since sorting the run moves
/// them (§5.5).
fn summary(text: &str) -> Option<(String, Vec<String>)> {
    let mut db = FileDb::new();
    let id = db.add("corpus.nova", text);
    let (tokens, comments, lex_errors) = nova_lexer::lex_with_comments(text, id);
    let (file, parse_errors) = nova_parser::parse(&tokens, id);
    let mut file = file?;
    if !lex_errors.is_empty() || !parse_errors.is_empty() {
        return None;
    }
    let mut regions = Vec::new();
    let mut i = 0;
    while i < file.items.len() {
        if !matches!(file.items[i].value, Item::Import(_)) {
            i += 1;
            continue;
        }
        let start = i;
        while i < file.items.len() && matches!(file.items[i].value, Item::Import(_)) {
            i += 1;
        }
        let from = if start == 0 {
            0
        } else {
            file.items[start - 1].span.end
        };
        let to = file
            .items
            .get(i)
            .map_or(text.len() as u32, |next| next.span.start);
        regions.push((from, to));
        let run = &mut file.items[start..i];
        run.sort_by_key(|item| match &item.value {
            Item::Import(import) => {
                let names: Vec<&str> = import
                    .path
                    .value
                    .segments
                    .iter()
                    .map(|s| s.value.as_str())
                    .collect();
                names.join("::")
            }
            _ => String::new(),
        });
        for item in run {
            if let Item::Import(import) = &mut item.value {
                if let ImportKind::List(names) = &mut import.kind {
                    names.sort_by(|a, b| a.value.cmp(&b.value));
                }
            }
        }
    }
    let mut ordered = Vec::new();
    let mut in_runs = vec![Vec::new(); regions.len()];
    for c in &comments {
        let kind = match c.kind {
            CommentKind::Line => "line",
            CommentKind::Block => "block",
        };
        let body: Vec<&str> = text[c.span.start as usize..c.span.end as usize]
            .lines()
            .map(str::trim_end)
            .collect();
        let entry = format!("{kind}: {}", body.join("\n"));
        match regions
            .iter()
            .position(|&(s, e)| c.span.start >= s && c.span.start < e)
        {
            Some(k) => in_runs[k].push(entry),
            None => ordered.push(entry),
        }
    }
    for mut run in in_runs {
        run.sort();
        ordered.push(format!("import run: {run:?}"));
    }
    Some((without_spans(&format!("{file:?}")), ordered))
}

#[test]
fn every_corpus_file_formats_to_the_same_program_and_comments() {
    let root = root();
    let mut files = Vec::new();
    nova_files(&root, &mut files);
    let mut checked = 0;
    let mut skipped = Vec::new();
    let mut failures = Vec::new();
    for path in &files {
        let rel = path
            .strip_prefix(&root)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        let text = std::fs::read_to_string(path).unwrap().replace("\r\n", "\n");
        let Some(before) = summary(&text) else {
            skipped.push(rel);
            continue;
        };
        checked += 1;
        let out = match nova_fmt::format_named(&text, &rel) {
            Ok(out) => out,
            Err(e) => {
                failures.push(format!("{rel}: {e:?}"));
                continue;
            }
        };
        match summary(&out) {
            None => failures.push(format!("{rel}: the output does not parse")),
            Some(after) if after.0 != before.0 => failures.push(format!("{rel}: the AST changed")),
            Some(after) if after.1 != before.1 => {
                failures.push(format!("{rel}: the comments changed"))
            }
            Some(_) => {}
        }
        match nova_fmt::format_named(&out, &rel) {
            Ok(again) if again == out => {}
            Ok(_) => failures.push(format!("{rel}: formatting it again changes it")),
            Err(e) => failures.push(format!("{rel}: formatting it again fails: {e:?}")),
        }
    }
    eprintln!("checked {checked} files; skipped {skipped:?}");
    assert!(
        failures.is_empty(),
        "{} failures:\n{}",
        failures.len(),
        failures.join("\n")
    );
    assert_eq!(
        skipped,
        EXPECTED_SKIP.map(str::to_owned).to_vec(),
        "the files that do not parse"
    );
    assert!(checked >= AT_LEAST, "only {checked} files were checked");
}
