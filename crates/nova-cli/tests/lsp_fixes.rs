//! `nova lsp`'s code actions and semantic tokens, driven over stdio (spec
//! `docs/superpowers/specs/2026-10-10-phase-3-4b-fixes-and-colour-design.md`
//! §5-§7, §9.6, §10).

mod lsp_client;

use lsp_client::{file_uri, lock_and_cache, project, registry_app, same_uri, Client};
use serde_json::{json, Value};

fn open(client: &mut Client, uri: &str, text: &str) {
    client.notify(
        "textDocument/didOpen",
        json!({ "textDocument": { "uri": uri, "languageId": "nova", "version": 1, "text": text } }),
    );
}

/// The LSP position of byte `byte` of `text`.
fn position(text: &str, byte: usize) -> Value {
    let line = text[..byte].matches('\n').count();
    let line_start = text[..byte].rfind('\n').map_or(0, |i| i + 1);
    json!({ "line": line, "character": text[line_start..byte].encode_utf16().count() })
}

/// The position of the start of the `n`th `marker` (from 0) in `text`.
fn at(text: &str, marker: &str, n: usize) -> Value {
    let start = text
        .match_indices(marker)
        .nth(n)
        .unwrap_or_else(|| panic!("no `{marker}` number {n}"))
        .0;
    position(text, start)
}

/// The code actions for `start..end`, with `only` when given.
fn actions_in(
    client: &mut Client,
    uri: &str,
    start: Value,
    end: Value,
    only: Option<&[&str]>,
) -> Vec<Value> {
    let mut context = json!({ "diagnostics": [] });
    if let Some(only) = only {
        context["only"] = json!(only);
    }
    let response = client.request(
        "textDocument/codeAction",
        json!({ "textDocument": { "uri": uri }, "range": { "start": start, "end": end }, "context": context }),
    );
    response["result"]
        .as_array()
        .unwrap_or_else(|| panic!("{response}"))
        .clone()
}

/// The code actions at the empty range `at`, as VS Code sends a cursor.
fn actions(client: &mut Client, uri: &str, at: Value, only: Option<&[&str]>) -> Vec<Value> {
    actions_in(client, uri, at.clone(), at, only)
}

/// The action titled `title`.
#[track_caller]
fn titled<'a>(found: &'a [Value], title: &str) -> &'a Value {
    found
        .iter()
        .find(|a| a["title"] == title)
        .unwrap_or_else(|| panic!("no {title:?} in {found:?}"))
}

/// The edits an action makes in the document `uri`.
#[track_caller]
fn edits_in(action: &Value, uri: &str) -> Vec<Value> {
    let changes = action["edit"]["changes"]
        .as_object()
        .unwrap_or_else(|| panic!("{action}"));
    changes
        .iter()
        .find(|(u, _)| same_uri(u, uri))
        .map(|(_, e)| e.as_array().unwrap().clone())
        .unwrap_or_else(|| panic!("no edit for {uri} in {action}"))
}

// === Task 10: quick fixes (spec §5) ===

const MUTABLE: &str = "fn main() {\n    let x = 1\n    x = 2\n    println(\"${x}\")\n}\n";

#[test]
fn the_server_offers_quick_fixes_and_organize_imports() {
    let dir = project("capabilities", &[("main.nova", "fn main() {}\n")]);
    let client = Client::start(&dir, false);
    assert_eq!(
        client.initialized["capabilities"]["codeActionProvider"]["codeActionKinds"],
        json!(["quickfix", "source.organizeImports"]),
        "{}",
        client.initialized
    );
}

#[test]
fn make_mutable_at_a_cursor() {
    let dir = project("make-mutable", &[("main.nova", MUTABLE)]);
    let uri = file_uri(&dir.join("src").join("main.nova"));
    let mut client = Client::start(&dir, false);
    open(&mut client, &uri, MUTABLE);
    let found = actions(&mut client, &uri, at(MUTABLE, "x = 2", 0), None);
    let action = titled(&found, "Make `x` mutable");
    assert_eq!(action["kind"], "quickfix");
    assert_eq!(action["isPreferred"], true);
    assert_eq!(action["diagnostics"][0]["code"], "E0060");
    let point = json!({ "line": 1, "character": 8 });
    assert_eq!(
        edits_in(action, &uri),
        [json!({ "range": { "start": point, "end": point }, "newText": "mut " })]
    );
    // A cursor touching the label's end counts too (spec §5).
    let end = position(MUTABLE, MUTABLE.find("x = 2").unwrap() + "x = 2".len());
    titled(&actions(&mut client, &uri, end, None), "Make `x` mutable");
}

#[test]
fn make_public_edits_the_sibling_modules_uri() {
    let main = "import lib::{hidden}\n\nfn main() {\n    println(\"${hidden()}\")\n}\n";
    let dir = project(
        "make-public",
        &[
            ("main.nova", main),
            ("lib.nova", "fn hidden() -> Int {\n    1\n}\n"),
        ],
    );
    let uri = file_uri(&dir.join("src").join("main.nova"));
    let lib = file_uri(&dir.join("src").join("lib.nova"));
    let mut client = Client::start(&dir, false);
    open(&mut client, &uri, main);
    let found = actions(&mut client, &uri, at(main, "hidden}", 0), None);
    let action = titled(&found, "Make `hidden` public in `lib`");
    let point = json!({ "line": 0, "character": 0 });
    assert_eq!(
        edits_in(action, &lib),
        [json!({ "range": { "start": point, "end": point }, "newText": "pub " })]
    );
}

#[test]
fn only_quickfix_or_source_filters_the_actions() {
    let dir = project("only", &[("main.nova", MUTABLE)]);
    let uri = file_uri(&dir.join("src").join("main.nova"));
    let mut client = Client::start(&dir, false);
    open(&mut client, &uri, MUTABLE);
    let fixes = actions(
        &mut client,
        &uri,
        at(MUTABLE, "x = 2", 0),
        Some(&["quickfix"]),
    );
    titled(&fixes, "Make `x` mutable");
    let source = actions(
        &mut client,
        &uri,
        at(MUTABLE, "x = 2", 0),
        Some(&["source"]),
    );
    assert!(source.iter().all(|a| a["kind"] != "quickfix"), "{source:?}");
}

#[test]
fn no_actions_inside_a_downloaded_package() {
    let (app, home) = registry_app("downloaded", "0.1");
    let lib = "pub fn area() -> Int {\n    let x = 1\n    x = 2\n    x\n}\n";
    let geom = lock_and_cache(&app, &home, lib);
    let uri = file_uri(&geom.join("src").join("lib.nova"));
    let mut client = Client::start_with_env(&app, false, &[("NOVA_HOME", &home)]);
    open(&mut client, &uri, lib);
    assert_eq!(
        actions(&mut client, &uri, at(lib, "x = 2", 0), None),
        Vec::<Value>::new()
    );
}

#[test]
fn a_quick_fix_edits_an_unsaved_buffer_at_its_own_offsets() {
    // Review Focus 1: the buffer holds two lines the disk does not.
    let dir = project("unsaved", &[("main.nova", MUTABLE)]);
    let uri = file_uri(&dir.join("src").join("main.nova"));
    let buffer = format!("// one\n// two\n{MUTABLE}");
    let mut client = Client::start(&dir, false);
    open(&mut client, &uri, &buffer);
    let found = actions(&mut client, &uri, at(&buffer, "x = 2", 0), None);
    let action = titled(&found, "Make `x` mutable");
    assert_eq!(
        edits_in(action, &uri)[0]["range"]["start"],
        json!({ "line": 3, "character": 8 })
    );
}

#[test]
fn a_quick_fixs_range_counts_utf16_after_thai_and_an_emoji() {
    // Review Focus 2: `x` follows three Thai characters and an emoji, one
    // UTF-16 unit each and two.
    let text =
        "fn main() {\n    let s = \"ไทย😀\"; let x = 1\n    x = 2\n    println(\"${s}${x}\")\n}\n";
    let dir = project("utf16-fix", &[("main.nova", text)]);
    let uri = file_uri(&dir.join("src").join("main.nova"));
    let mut client = Client::start(&dir, false);
    open(&mut client, &uri, text);
    let found = actions(&mut client, &uri, at(text, "x = 2", 0), None);
    let action = titled(&found, "Make `x` mutable");
    assert_eq!(
        edits_in(action, &uri)[0]["range"]["start"],
        json!({ "line": 1, "character": 25 })
    );
}

#[test]
fn two_errors_with_one_fix_offer_it_once() {
    // Review Focus 3.
    let text = "fn main() {\n    let x = 1\n    x = 2\n    x = 3\n    println(\"${x}\")\n}\n";
    let dir = project("one-fix-once", &[("main.nova", text)]);
    let uri = file_uri(&dir.join("src").join("main.nova"));
    let mut client = Client::start(&dir, false);
    open(&mut client, &uri, text);
    let found = actions_in(
        &mut client,
        &uri,
        at(text, "x = 2", 0),
        at(text, "    println", 0),
        None,
    );
    let mutable: Vec<&Value> = found
        .iter()
        .filter(|a| a["title"] == "Make `x` mutable")
        .collect();
    assert_eq!(mutable.len(), 1, "{found:?}");
    assert_eq!(
        mutable[0]["diagnostics"].as_array().unwrap().len(),
        2,
        "{found:?}"
    );
}

// === Task 11: organize imports (spec §6) ===

/// `text` with the LSP `edits` applied.
fn applied(text: &str, edits: &[Value]) -> String {
    let offset = |p: &Value| {
        let line = p["line"].as_u64().unwrap() as usize;
        let character = p["character"].as_u64().unwrap() as usize;
        let start: usize = text.split_inclusive('\n').take(line).map(str::len).sum();
        let (mut units, mut at) = (0, start);
        for c in text[start..].chars() {
            if units >= character {
                break;
            }
            units += c.len_utf16();
            at += c.len_utf8();
        }
        at
    };
    let mut spans: Vec<(usize, usize, String)> = edits
        .iter()
        .map(|e| {
            (
                offset(&e["range"]["start"]),
                offset(&e["range"]["end"]),
                e["newText"].as_str().unwrap().to_string(),
            )
        })
        .collect();
    spans.sort_by_key(|s| s.0);
    let mut out = text.to_string();
    for (start, end, new) in spans.iter().rev() {
        out.replace_range(*start..*end, new);
    }
    out
}

const SHAPES: &str = "pub fn twice() -> Int {\n    2\n}\n";
const EXTRA: &str = "pub fn unused() -> Int {\n    0\n}\n";

#[test]
fn organize_imports_groups_and_drops_an_unused_import() {
    let main = "import shapes\nimport geom\nimport extra\n\nfn main() {\n    println(\"${area()} ${twice()}\")\n}\n";
    let (app, _geom) =
        lsp_client::app_with_main("organize", "pub fn area() -> Int {\n    1\n}\n", main);
    std::fs::write(app.join("src").join("shapes.nova"), SHAPES).unwrap();
    std::fs::write(app.join("src").join("extra.nova"), EXTRA).unwrap();
    let uri = file_uri(&app.join("src").join("main.nova"));
    let mut client = Client::start(&app, false);
    open(&mut client, &uri, main);
    let found = actions(
        &mut client,
        &uri,
        at(main, "fn main", 0),
        Some(&["source.organizeImports"]),
    );
    assert_eq!(found.len(), 1, "{found:?}");
    let action = titled(&found, "Organize imports");
    assert_eq!(action["kind"], "source.organizeImports");
    assert_eq!(
        applied(main, &edits_in(action, &uri)),
        "import geom\n\nimport shapes\n\nfn main() {\n    println(\"${area()} ${twice()}\")\n}\n"
    );
}

#[test]
fn organize_imports_in_a_file_with_an_error_keeps_unused_imports() {
    let main = "import shapes\nimport extra\n\nfn main() {\n    let n: Int = \"no\"\n    println(\"${twice()}\")\n}\n";
    let dir = project(
        "organize-error",
        &[
            ("main.nova", main),
            ("shapes.nova", SHAPES),
            ("extra.nova", EXTRA),
        ],
    );
    let uri = file_uri(&dir.join("src").join("main.nova"));
    let mut client = Client::start(&dir, false);
    open(&mut client, &uri, main);
    let found = actions(
        &mut client,
        &uri,
        at(main, "fn main", 0),
        Some(&["source.organizeImports"]),
    );
    let action = titled(&found, "Organize imports");
    assert_eq!(
        applied(main, &edits_in(action, &uri)),
        "import extra\nimport shapes\n\nfn main() {\n    let n: Int = \"no\"\n    println(\"${twice()}\")\n}\n"
    );
}

#[test]
fn only_source_returns_organize_imports_alone() {
    let main = "import shapes\nimport extra\n\nfn main() {\n    let x = 1\n    x = 2\n    println(\"${x} ${twice()} ${unused()}\")\n}\n";
    let dir = project(
        "organize-only",
        &[
            ("main.nova", main),
            ("shapes.nova", SHAPES),
            ("extra.nova", EXTRA),
        ],
    );
    let uri = file_uri(&dir.join("src").join("main.nova"));
    let mut client = Client::start(&dir, false);
    open(&mut client, &uri, main);
    let source = actions(&mut client, &uri, at(main, "x = 2", 0), Some(&["source"]));
    let kinds: Vec<&str> = source.iter().map(|a| a["kind"].as_str().unwrap()).collect();
    assert_eq!(kinds, ["source.organizeImports"]);
    let fixes = actions(&mut client, &uri, at(main, "x = 2", 0), Some(&["quickfix"]));
    assert!(
        !fixes.is_empty() && fixes.iter().all(|a| a["kind"] == "quickfix"),
        "{fixes:?}"
    );
}

#[test]
fn organize_imports_puts_the_packages_own_library_with_the_dependencies() {
    // Spec §6.2 and spec decision 26: from `tests/`, the package's own library
    // is a dependency, and a sibling test module is the project's own.
    let dir = project(
        "organize-tests",
        &[("lib.nova", "pub fn name() -> String {\n    \"demo\"\n}\n")],
    );
    let tests = dir.join("tests");
    std::fs::create_dir_all(&tests).unwrap();
    std::fs::write(tests.join("util.nova"), "pub fn one() -> Int {\n    1\n}\n").unwrap();
    let test = "import util\nimport demo\n\n@test\nfn t() {\n    assert_eq(name(), \"demo\")\n    assert_eq(one(), 1)\n}\n";
    std::fs::write(tests.join("t.nova"), test).unwrap();
    let uri = file_uri(&tests.join("t.nova"));
    let mut client = Client::start(&dir, false);
    open(&mut client, &uri, test);
    let found = actions(
        &mut client,
        &uri,
        at(test, "@test", 0),
        Some(&["source.organizeImports"]),
    );
    let action = titled(&found, "Organize imports");
    assert_eq!(
        applied(test, &edits_in(action, &uri)),
        "import demo\n\nimport util\n\n@test\nfn t() {\n    assert_eq(name(), \"demo\")\n    assert_eq(one(), 1)\n}\n"
    );
}

// === Task 12: semantic tokens (spec §7) ===

/// The tokens of `uri`, decoded: line, UTF-16 column, length, type index,
/// modifier bits.
fn tokens(client: &mut Client, uri: &str) -> Vec<(u64, u64, u64, u64, u64)> {
    let response = client.request(
        "textDocument/semanticTokens/full",
        json!({ "textDocument": { "uri": uri } }),
    );
    let data: Vec<u64> = response["result"]["data"]
        .as_array()
        .unwrap_or_else(|| panic!("{response}"))
        .iter()
        .map(|v| v.as_u64().unwrap())
        .collect();
    let (mut line, mut col) = (0, 0);
    data.chunks(5)
        .map(|t| {
            line += t[0];
            col = if t[0] == 0 { col + t[1] } else { t[1] };
            (line, col, t[2], t[3], t[4])
        })
        .collect()
}

/// The legend's indices (spec §7.1).
const T_FUNCTION: u64 = 10;
const T_VARIABLE: u64 = 7;
const M_DECLARATION: u64 = 1;
const M_DEFAULT_LIBRARY: u64 = 4;
const M_MUTABLE: u64 = 8;

#[test]
fn the_server_advertises_semantic_tokens() {
    let dir = project("tokens-capability", &[("main.nova", "fn main() {}\n")]);
    let client = Client::start(&dir, false);
    let provider = &client.initialized["capabilities"]["semanticTokensProvider"];
    assert_eq!(provider["full"], true, "{provider}");
    assert_eq!(
        provider["legend"]["tokenTypes"][0], "namespace",
        "{provider}"
    );
    assert_eq!(
        provider["legend"]["tokenModifiers"][3], "mutable",
        "{provider}"
    );
}

#[test]
fn semantic_tokens_name_by_name() {
    let main = "fn main() {\n    let mut x = 1\n    x = x + 1\n    println(\"${x}\")\n}\n";
    let dir = project("tokens", &[("main.nova", main)]);
    let uri = file_uri(&dir.join("src").join("main.nova"));
    let mut client = Client::start(&dir, false);
    open(&mut client, &uri, main);
    let found = tokens(&mut client, &uri);
    assert!(
        found.contains(&(0, 3, 4, T_FUNCTION, M_DECLARATION)),
        "{found:?}"
    );
    assert!(
        found.contains(&(1, 12, 1, T_VARIABLE, M_DECLARATION | M_MUTABLE)),
        "{found:?}"
    );
    assert!(
        found.contains(&(2, 4, 1, T_VARIABLE, M_MUTABLE)),
        "{found:?}"
    );
    assert!(
        found.contains(&(3, 4, 7, T_FUNCTION, M_DEFAULT_LIBRARY)),
        "{found:?}"
    );
}

#[test]
fn semantic_tokens_in_a_file_with_an_error() {
    let main = "fn main() {\n    let x = 1\n    let y = nope + x\n}\n";
    let dir = project("tokens-error", &[("main.nova", main)]);
    let uri = file_uri(&dir.join("src").join("main.nova"));
    let mut client = Client::start(&dir, false);
    open(&mut client, &uri, main);
    let found = tokens(&mut client, &uri);
    assert!(found.contains(&(2, 19, 1, T_VARIABLE, 0)), "{found:?}");
    assert!(
        !found.iter().any(|t| t.0 == 2 && t.1 == 12),
        "an unresolved name: {found:?}"
    );
}
