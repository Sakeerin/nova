//! `nova lsp`'s navigation, driven over stdio (spec
//! `docs/superpowers/specs/2026-10-10-phase-3-4a-navigation-design.md`
//! §8.4, §9).

mod lsp_client;

#[allow(unused_imports)]
use lsp_client::{file_uri, fresh_dir, same_uri, Client};
use serde_json::{json, Value};

const MANIFEST: &str = "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2026\"\n";

fn open(client: &mut Client, uri: &str, text: &str) {
    client.notify(
        "textDocument/didOpen",
        json!({ "textDocument": { "uri": uri, "languageId": "nova", "version": 1, "text": text } }),
    );
}

/// A project: `nova.toml`, and `src/` holding `files`.
fn project(name: &str, files: &[(&str, &str)]) -> std::path::PathBuf {
    let dir = fresh_dir(name);
    std::fs::write(dir.join("nova.toml"), MANIFEST).unwrap();
    std::fs::create_dir_all(dir.join("src")).unwrap();
    for (file, text) in files {
        std::fs::write(dir.join("src").join(file), text).unwrap();
    }
    dir
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

/// The range of `word` at the start of the `n`th `marker` in `text`.
fn range(text: &str, marker: &str, n: usize, word: &str) -> Value {
    let start = text.match_indices(marker).nth(n).unwrap().0;
    json!({ "start": position(text, start), "end": position(text, start + word.len()) })
}

fn hover(client: &mut Client, uri: &str, position: Value) -> Value {
    client.request(
        "textDocument/hover",
        json!({ "textDocument": { "uri": uri }, "position": position }),
    )
}

/// A response's result, after checking it is not an error. A missing
/// `result` reads as `null`, so a null check alone would pass on an error.
#[track_caller]
fn ok(response: &Value) -> Value {
    assert!(response["error"].is_null(), "an error: {response}");
    response["result"].clone()
}

/// A hover response's Markdown.
#[track_caller]
fn markdown(response: &Value) -> String {
    response["result"]["contents"]["value"]
        .as_str()
        .unwrap_or_else(|| panic!("no hover: {response}"))
        .to_string()
}

/// A hover's code block: its Markdown's first line inside the fence.
#[track_caller]
fn code(response: &Value) -> String {
    let md = markdown(response);
    md.strip_prefix("```nova\n")
        .and_then(|rest| rest.split("\n```").next())
        .unwrap_or_else(|| panic!("no code block: {md}"))
        .to_string()
}

// === Task 7: hover ===

const AREA: &str = "/// The area of a square.\n///\n/// Sides are whole numbers.\n\
pub fn area(side: Int) -> Int { side * side }\n\n\
fn main() {\n    let total = area(3)\n    let mut n = 0\n    n = total\n}\n";

#[test]
fn hover_shows_a_functions_signature_and_its_docs() {
    let dir = project("hover-docs", &[("main.nova", AREA)]);
    let uri = file_uri(&dir.join("src").join("main.nova"));
    let mut client = Client::start(&dir, false);
    open(&mut client, &uri, AREA);
    let response = hover(&mut client, &uri, at(AREA, "area(3)", 0));
    assert_eq!(
        markdown(&response),
        "```nova\npub fn area(side: Int) -> Int\n```\n\nThe area of a square.\n\nSides are whole numbers."
    );
    assert_eq!(
        response["result"]["range"],
        range(AREA, "area(3)", 0, "area")
    );
}

#[test]
fn hover_shows_a_locals_inferred_type() {
    let dir = project("hover-local", &[("main.nova", AREA)]);
    let uri = file_uri(&dir.join("src").join("main.nova"));
    let mut client = Client::start(&dir, false);
    open(&mut client, &uri, AREA);
    assert_eq!(
        code(&hover(&mut client, &uri, at(AREA, "total", 0))),
        "let total: Int"
    );
    assert_eq!(
        code(&hover(&mut client, &uri, at(AREA, "n = total", 0))),
        "let mut n: Int"
    );
}

const KINDS: &str = "/// A point.\nrecord Point { x: Int }\n\
type Shape = | Round(Int) | Flat\n\
trait Show {\n    type Out\n    fn show(self) -> String\n}\n\
impl Show for Point {\n    type Out = Int\n    fn show(self) -> String { \"p\" }\n}\n\
pub const LIMIT: Int = 3\n\
fn keep<T: Show>(t: T) -> T {\n    let kept = t\n    kept\n}\n\
fn main() {\n    let p = Point { x: 1 }\n    let s = p.show()\n    let r = Round(LIMIT)\n    let xs = [1]\n    let n = xs.len()\n    print(\"x\")\n    let mut v = Vec::new()\n    v.push(1)\n}\n";

#[test]
fn hover_on_each_kind_of_name() {
    let dir = project("hover-kinds", &[("main.nova", KINDS)]);
    let uri = file_uri(&dir.join("src").join("main.nova"));
    let mut client = Client::start(&dir, false);
    open(&mut client, &uri, KINDS);
    let cases: &[(&str, usize, &str)] = &[
        ("Point { x: 1", 0, "record Point"),
        ("x: 1", 0, "x: Int"),
        ("Shape", 0, "type Shape"),
        ("Round(LIMIT)", 0, "Round(Int)"),
        ("LIMIT)", 0, "pub const LIMIT: Int"),
        ("show()", 0, "fn show(self) -> String"),
        ("Show>", 0, "trait Show"),
        ("Out = Int", 0, "type Out"),
        ("T) -> T", 0, "T: Show"),
        ("kept = t", 0, "let kept: T"),
        ("len()", 0, "fn len(self) -> Int"),
        ("print(", 0, "fn print(String) -> ()"),
        ("Int }", 0, "type Int"),
    ];
    for (marker, n, expected) in cases {
        let response = hover(&mut client, &uri, at(KINDS, marker, *n));
        assert_eq!(code(&response), *expected, "at `{marker}`: {response}");
    }
    // A record's docs.
    let point = markdown(&hover(&mut client, &uri, at(KINDS, "Point { x: 1", 0)));
    assert!(point.ends_with("\n\nA point."), "{point}");
    // A std method: its signature as std writes it.
    let push = code(&hover(&mut client, &uri, at(KINDS, "push(1)", 0)));
    assert!(push.contains("fn push("), "{push}");
}

#[test]
fn hover_on_a_module_and_on_a_package() {
    let dir = project(
        "hover-module",
        &[
            ("main.nova", "import geometry\nfn main() {}\n"),
            ("geometry.nova", "pub fn area() -> Int { 1 }\n"),
        ],
    );
    let uri = file_uri(&dir.join("src").join("main.nova"));
    let mut client = Client::start(&dir, false);
    open(&mut client, &uri, "import geometry\nfn main() {}\n");
    let response = hover(&mut client, &uri, at("import geometry", "geometry", 0));
    assert_eq!(code(&response), "module geometry");

    let (app, _geom) = app_and_library("hover-package", "pub fn area() -> Int { 1 }\n");
    let main = file_uri(&app.join("src").join("main.nova"));
    let mut client = Client::start(&app, false);
    open(&mut client, &main, APP_MAIN);
    let response = hover(&mut client, &main, at(APP_MAIN, "geom", 0));
    assert_eq!(code(&response), "package geom 0.1.0");
}

#[test]
fn hover_works_in_a_file_with_a_syntax_error() {
    let text = "fn broken( {\n}\nfn main() {\n    let total = 1\n    let b = total\n}\n";
    let dir = project("hover-broken", &[("main.nova", text)]);
    let uri = file_uri(&dir.join("src").join("main.nova"));
    let mut client = Client::start(&dir, false);
    open(&mut client, &uri, text);
    assert_eq!(
        code(&hover(&mut client, &uri, at(text, "total", 1))),
        "let total: Int"
    );
}

#[test]
fn hover_on_no_name_or_an_unopened_document_is_null() {
    let dir = project("hover-null", &[("main.nova", AREA)]);
    let uri = file_uri(&dir.join("src").join("main.nova"));
    let mut client = Client::start(&dir, false);
    // Not opened yet.
    assert_eq!(
        ok(&hover(&mut client, &uri, at(AREA, "area(3)", 0))),
        Value::Null
    );
    open(&mut client, &uri, AREA);
    // A blank line.
    assert_eq!(
        ok(&hover(
            &mut client,
            &uri,
            json!({ "line": 4, "character": 0 })
        )),
        Value::Null
    );
}

// === Phase 3.3a's app and library, as in lsp.rs ===

const APP_MAIN: &str = "import geom\n\nfn main() {\n    let a: Int = area()\n}\n";

/// `app`, which depends on `geom` by path, in one fresh directory. `geom`'s
/// `src/lib.nova` is `lib`.
fn app_and_library(name: &str, lib: &str) -> (std::path::PathBuf, std::path::PathBuf) {
    let dir = fresh_dir(name);
    let geom = dir.join("geom");
    std::fs::create_dir_all(geom.join("src")).unwrap();
    std::fs::write(geom.join("nova.toml"), MANIFEST.replace("demo", "geom")).unwrap();
    std::fs::write(geom.join("src").join("lib.nova"), lib).unwrap();
    let app = dir.join("app");
    std::fs::create_dir_all(app.join("src")).unwrap();
    let manifest = format!(
        "{}\n[dependencies]\ngeom = {{ path = \"../geom\" }}\n",
        MANIFEST.replace("demo", "app")
    );
    std::fs::write(app.join("nova.toml"), manifest).unwrap();
    std::fs::write(app.join("src").join("main.nova"), APP_MAIN).unwrap();
    (app, geom)
}
