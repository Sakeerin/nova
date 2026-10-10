//! `nova lsp`'s navigation, driven over stdio (spec
//! `docs/superpowers/specs/2026-10-10-phase-3-4a-navigation-design.md`
//! §8.4, §9).

mod lsp_client;

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

// === Task 8: std's cache, and go to definition ===

fn definition(client: &mut Client, uri: &str, position: Value) -> Value {
    client.request(
        "textDocument/definition",
        json!({ "textDocument": { "uri": uri }, "position": position }),
    )
}

/// A location's path, from its `file:` URI.
fn path_of(location: &Value) -> std::path::PathBuf {
    let uri = location["uri"]
        .as_str()
        .unwrap_or_else(|| panic!("no location: {location}"));
    let decoded = lsp_client::decode(uri.strip_prefix("file://").unwrap());
    let trimmed = if decoded.as_bytes().get(2) == Some(&b':') {
        &decoded[1..]
    } else {
        &decoded[..]
    };
    std::path::PathBuf::from(trimmed)
}

/// The text a location's range covers, read from its file.
fn text_at(location: &Value) -> String {
    let text = std::fs::read_to_string(path_of(location)).unwrap();
    let range = &location["range"];
    let offset = |p: &Value| {
        let line = p["line"].as_u64().unwrap() as usize;
        let character = p["character"].as_u64().unwrap() as usize;
        let start = text
            .split_inclusive('\n')
            .take(line)
            .map(str::len)
            .sum::<usize>();
        let rest = &text[start..];
        let mut units = 0;
        for (i, c) in rest.char_indices() {
            if units >= character {
                return start + i;
            }
            units += c.len_utf16();
        }
        text.len()
    };
    text[offset(&range["start"])..offset(&range["end"])].to_string()
}

const MAIN_IMPORTS_GEOMETRY: &str =
    "import geometry\n\nfn main() {\n    let total = area()\n    let again = total\n    print(\"x\")\n}\n";
const GEOMETRY: &str = "/// One.\npub fn area() -> Int { 1 }\n";

#[test]
fn definition_in_the_same_file_and_into_another_module() {
    let dir = project(
        "definition-modules",
        &[
            ("main.nova", MAIN_IMPORTS_GEOMETRY),
            ("geometry.nova", GEOMETRY),
        ],
    );
    let main = dir.join("src").join("main.nova");
    let uri = file_uri(&main);
    let mut client = Client::start(&dir, false);
    open(&mut client, &uri, MAIN_IMPORTS_GEOMETRY);
    let local =
        definition(&mut client, &uri, at(MAIN_IMPORTS_GEOMETRY, "total", 1))["result"].clone();
    assert!(same_uri(local["uri"].as_str().unwrap(), &uri), "{local}");
    assert_eq!(
        local["range"],
        range(MAIN_IMPORTS_GEOMETRY, "total", 0, "total")
    );
    let area =
        definition(&mut client, &uri, at(MAIN_IMPORTS_GEOMETRY, "area()", 0))["result"].clone();
    assert!(
        same_uri(
            area["uri"].as_str().unwrap(),
            &file_uri(&dir.join("src").join("geometry.nova"))
        ),
        "{area}"
    );
    assert_eq!(text_at(&area), "area");
    // `import geometry` goes to its file's start.
    let module =
        definition(&mut client, &uri, at(MAIN_IMPORTS_GEOMETRY, "geometry", 0))["result"].clone();
    assert!(path_of(&module).ends_with("geometry.nova"), "{module}");
    assert_eq!(
        module["range"]["start"],
        json!({ "line": 0, "character": 0 })
    );
    // A builtin has no definition.
    assert_eq!(
        ok(&definition(
            &mut client,
            &uri,
            at(MAIN_IMPORTS_GEOMETRY, "print", 0)
        )),
        Value::Null
    );
}

#[test]
fn definition_of_a_trait_dispatched_call_is_the_traits_declaration() {
    // Spec decision 6, and the test spec §8.5's second mutant breaks.
    let text = "trait Show {\n    fn show(self) -> String\n}\nrecord A { n: Int }\n\
impl Show for A {\n    fn show(self) -> String { \"a\" }\n}\n\
fn main() {\n    let a = A { n: 1 }\n    let s = a.show()\n}\n";
    let dir = project("definition-trait", &[("main.nova", text)]);
    let uri = file_uri(&dir.join("src").join("main.nova"));
    let mut client = Client::start(&dir, false);
    open(&mut client, &uri, text);
    let found = ok(&definition(&mut client, &uri, at(text, "show()", 0)));
    // The trait's `fn show`, not the impl's.
    assert_eq!(found["range"], range(text, "show", 0, "show"));
}

#[test]
fn definition_into_a_path_dependency_is_at_its_real_path() {
    let (app, geom) = app_and_library("definition-dependency", GEOMETRY);
    let main = file_uri(&app.join("src").join("main.nova"));
    let mut client = Client::start(&app, false);
    open(&mut client, &main, APP_MAIN);
    let found = definition(&mut client, &main, at(APP_MAIN, "area()", 0))["result"].clone();
    // Not `app/../geom/...`.
    assert!(
        same_uri(
            found["uri"].as_str().unwrap(),
            &file_uri(&geom.join("src").join("lib.nova"))
        ),
        "{found}"
    );
    assert_eq!(text_at(&found), "area");
}

/// A fresh `NOVA_HOME`.
fn home(name: &str) -> std::path::PathBuf {
    fresh_dir(&format!("{name}-home"))
}

const USES_STD: &str = "fn main() {\n    let mut v = Vec::new()\n    v.push(1)\n}\n";

#[test]
fn definition_into_std_opens_its_cache() {
    let home = home("definition-std");
    let dir = project("definition-std", &[("main.nova", USES_STD)]);
    let uri = file_uri(&dir.join("src").join("main.nova"));
    let mut client = Client::start_with_env(&dir, false, &[("NOVA_HOME", &home)]);
    open(&mut client, &uri, USES_STD);
    let found = definition(&mut client, &uri, at(USES_STD, "push", 0))["result"].clone();
    let file = path_of(&found);
    assert!(file.starts_with(home.join("std")), "{}", file.display());
    assert_eq!(file.file_name().unwrap(), "collections.nova");
    assert_eq!(text_at(&found), "push");
    assert!(std::fs::metadata(&file).unwrap().permissions().readonly());
    // Every module is there.
    let dir_of = file.parent().unwrap();
    for name in ["core", "json", "test"] {
        assert!(dir_of.join(format!("{name}.nova")).is_file(), "{name}");
    }
}

#[test]
fn std_cache_is_reused_and_a_damaged_file_is_replaced() {
    let home = home("std-cache-reuse");
    let dir = project("std-cache-reuse", &[("main.nova", USES_STD)]);
    let uri = file_uri(&dir.join("src").join("main.nova"));
    let mut client = Client::start_with_env(&dir, false, &[("NOVA_HOME", &home)]);
    open(&mut client, &uri, USES_STD);
    let file = path_of(&definition(&mut client, &uri, at(USES_STD, "push", 0))["result"]);
    let written = std::fs::metadata(&file).unwrap().modified().unwrap();
    let text = std::fs::read_to_string(&file).unwrap();
    definition(&mut client, &uri, at(USES_STD, "push", 0));
    assert_eq!(
        std::fs::metadata(&file).unwrap().modified().unwrap(),
        written
    );
    // Damage it: the next request replaces it.
    let mut perms = std::fs::metadata(&file).unwrap().permissions();
    #[allow(clippy::permissions_set_readonly_false)]
    perms.set_readonly(false);
    std::fs::set_permissions(&file, perms).unwrap();
    std::fs::write(&file, "damaged").unwrap();
    definition(&mut client, &uri, at(USES_STD, "push", 0));
    assert_eq!(std::fs::read_to_string(&file).unwrap(), text);
}

#[test]
fn two_servers_write_std_cache_at_once() {
    let home = home("std-cache-two");
    let dir = project("std-cache-two", &[("main.nova", USES_STD)]);
    let uri = file_uri(&dir.join("src").join("main.nova"));
    let mut a = Client::start_with_env(&dir, false, &[("NOVA_HOME", &home)]);
    let mut b = Client::start_with_env(&dir, false, &[("NOVA_HOME", &home)]);
    open(&mut a, &uri, USES_STD);
    open(&mut b, &uri, USES_STD);
    let (ra, rb) = std::thread::scope(|s| {
        let ta = s.spawn(|| definition(&mut a, &uri, at(USES_STD, "push", 0)));
        let tb = s.spawn(|| definition(&mut b, &uri, at(USES_STD, "push", 0)));
        (ta.join().unwrap(), tb.join().unwrap())
    });
    for found in [&ra["result"], &rb["result"]] {
        assert_eq!(text_at(found), "push", "{found}");
    }
}

#[test]
fn a_request_inside_std_cache_is_answered_and_nothing_is_published() {
    let home = home("std-cache-inside");
    let dir = project("std-cache-inside", &[("main.nova", USES_STD)]);
    let uri = file_uri(&dir.join("src").join("main.nova"));
    let mut client = Client::start_with_env(&dir, false, &[("NOVA_HOME", &home)]);
    open(&mut client, &uri, USES_STD);
    let file = path_of(&definition(&mut client, &uri, at(USES_STD, "push", 0))["result"]);
    let text = std::fs::read_to_string(&file).unwrap();
    let std_uri = file_uri(&file);
    client.clear_unread();
    open(&mut client, &std_uri, &text);
    // Hover on `Vec`'s name, in its declaration inside std's own file.
    let start = text.find("record Vec").unwrap() + "record ".len();
    let response = hover(&mut client, &std_uri, position(&text, start));
    assert!(code(&response).contains("record Vec"), "{response}");
    // Nothing is published for it.
    let sentinel_dir = fresh_dir("std-cache-inside-sentinel");
    let sentinel = file_uri(&sentinel_dir.join("main.nova"));
    std::fs::write(sentinel_dir.join("main.nova"), "fn main() {}\n").unwrap();
    open(&mut client, &sentinel, "fn main() {}\n");
    assert_eq!(client.last_diagnostics_before(&std_uri, &sentinel), None);
}

// Review Focus 2.
const WIDE: &str =
    "fn main() {\n    let s = \"ก😀\"; let total = 1\n    let b = \"ก😀\"; let c = total\n}\n";

#[test]
fn ranges_count_utf16_after_thai_and_an_emoji() {
    let dir = project("definition-utf16", &[("main.nova", WIDE)]);
    let uri = file_uri(&dir.join("src").join("main.nova"));
    let mut client = Client::start(&dir, false);
    open(&mut client, &uri, WIDE);
    let found = definition(&mut client, &uri, at(WIDE, "total", 1))["result"].clone();
    assert_eq!(found["range"], range(WIDE, "total", 0, "total"));
    let response = hover(&mut client, &uri, at(WIDE, "total", 1));
    assert_eq!(
        response["result"]["range"],
        range(WIDE, "total", 1, "total")
    );
}

// === Phase 3.3b's registry helpers, as in lsp.rs ===

const INDEX: &str = "https://example.test/index/";

/// `dir/app`, which depends on `geom = "<req>"` and runs `APP_MAIN`, and
/// `dir/home`, the server's `NOVA_HOME`.
fn registry_app(name: &str, req: &str) -> (std::path::PathBuf, std::path::PathBuf) {
    let dir = fresh_dir(name);
    let app = dir.join("app");
    std::fs::create_dir_all(app.join("src")).unwrap();
    std::fs::write(
        app.join("nova.toml"),
        format!(
            "{}\n[dependencies]\ngeom = \"{req}\"\n",
            MANIFEST.replace("demo", "app")
        ),
    )
    .unwrap();
    std::fs::write(app.join("src").join("main.nova"), APP_MAIN).unwrap();
    (app, dir.join("home"))
}

/// `geom` 0.1.0, locked in `app`'s nova.lock and unpacked under `home`,
/// with `lib` as its lib.nova. Its directory.
fn lock_and_cache(app: &std::path::Path, home: &std::path::Path, lib: &str) -> std::path::PathBuf {
    let geom = home
        .join("registry")
        .join("src")
        .join(nova_pm::index_dir_name(INDEX))
        .join("geom-0.1.0");
    std::fs::create_dir_all(geom.join("src")).unwrap();
    std::fs::write(geom.join("nova.toml"), MANIFEST.replace("demo", "geom")).unwrap();
    std::fs::write(geom.join("src").join("lib.nova"), lib).unwrap();
    std::fs::write(
        app.join("nova.lock"),
        format!(
            "version = 1\nindex = \"{INDEX}\"\n\n[[package]]\nname = \"geom\"\n\
             version = \"0.1.0\"\nchecksum = \"00\"\ndependencies = []\n"
        ),
    )
    .unwrap();
    geom
}

#[test]
fn definition_into_a_downloaded_package() {
    let (app, home) = registry_app("definition-registry", "0.1");
    let geom = lock_and_cache(&app, &home, GEOMETRY);
    let main = file_uri(&app.join("src").join("main.nova"));
    let mut client = Client::start_with_env(&app, false, &[("NOVA_HOME", &home)]);
    open(&mut client, &main, APP_MAIN);
    let found = definition(&mut client, &main, at(APP_MAIN, "area()", 0))["result"].clone();
    assert!(
        same_uri(
            found["uri"].as_str().unwrap(),
            &file_uri(&geom.join("src").join("lib.nova"))
        ),
        "{found}"
    );
    assert_eq!(text_at(&found), "area");
}
