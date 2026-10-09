//! `nova lsp`, driven over stdio (spec
//! `docs/superpowers/specs/2026-10-08-phase-3-2-lsp-core-design.md` §9.3).

mod lsp_client;

use lsp_client::{file_uri, fresh_dir, same_uri, Client};
use serde_json::{json, Value};

/// Thai and an emoji before the planted error, so a byte column and a
/// UTF-16 column differ (gate item 1).
const PLANTED_LINE: &str = "    let s = \"ก😀\"; let x: Int = \"s\"";

fn planted() -> String {
    format!("fn main() {{\n{PLANTED_LINE}\n}}\n")
}

/// The UTF-16 range of the planted `"s"`, on line 1.
fn planted_range() -> Value {
    let at = PLANTED_LINE.rfind("\"s\"").unwrap();
    let start = PLANTED_LINE[..at].encode_utf16().count();
    json!({
        "start": { "line": 1, "character": start },
        "end": { "line": 1, "character": start + 3 },
    })
}

fn open(client: &mut Client, uri: &str, text: &str) {
    client.notify(
        "textDocument/didOpen",
        json!({ "textDocument": { "uri": uri, "languageId": "nova", "version": 1, "text": text } }),
    );
}

fn codes(params: &Value) -> Vec<String> {
    params["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| d["code"].as_str().unwrap_or("").to_string())
        .collect()
}

fn nonempty(params: &Value) -> bool {
    !params["diagnostics"].as_array().unwrap().is_empty()
}

#[test]
fn a_planted_error_gets_its_diagnostic_with_an_exact_utf16_range() {
    let dir = fresh_dir("planted");
    let file = dir.join("main.nova");
    std::fs::write(&file, planted()).unwrap();
    let uri = file_uri(&file);
    let mut client = Client::start(&dir, false);
    open(&mut client, &uri, &planted());
    let params = client.diagnostics(&uri, nonempty);
    assert_eq!(codes(&params), ["E0010"]);
    assert_eq!(params["diagnostics"][0]["range"], planted_range());
    assert_eq!(params["version"], 1);
    // An open document's diagnostics come back under the client's own URI.
    assert_eq!(params["uri"], uri);
    assert_eq!(client.shutdown_and_exit().code(), Some(0));
}

#[test]
fn a_crlf_document_gets_exact_ranges() {
    let dir = fresh_dir("crlf");
    let file = dir.join("main.nova");
    let text = planted().replace('\n', "\r\n");
    std::fs::write(&file, &text).unwrap();
    let uri = file_uri(&file);
    let mut client = Client::start(&dir, false);
    open(&mut client, &uri, &text);
    let params = client.diagnostics(&uri, nonempty);
    assert_eq!(params["diagnostics"][0]["range"], planted_range());
}

#[test]
fn an_untitled_buffer_gets_diagnostics() {
    let dir = fresh_dir("untitled");
    let mut client = Client::start(&dir, false);
    open(&mut client, "untitled:Untitled-1", &planted());
    let params = client.diagnostics("untitled:Untitled-1", nonempty);
    assert_eq!(codes(&params), ["E0010"]);
}

#[test]
fn shutdown_then_exit_is_code_0_and_exit_alone_is_code_1() {
    let dir = fresh_dir("exit");
    assert_eq!(
        Client::start(&dir, false).shutdown_and_exit().code(),
        Some(0)
    );
    assert_eq!(
        Client::start(&dir, false).exit_without_shutdown().code(),
        Some(1)
    );
}

#[test]
fn an_unknown_request_gets_method_not_found() {
    let dir = fresh_dir("unknown");
    let mut client = Client::start(&dir, false);
    let response = client.request(
        "textDocument/hover",
        json!({
            "textDocument": { "uri": file_uri(&dir.join("x.nova")) },
            "position": { "line": 0, "character": 0 },
        }),
    );
    assert_eq!(response["error"]["code"], -32601);
}

// === Task 11: projects, ownership, watched files, stale results ===

const MANIFEST: &str = "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2026\"\n";

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

fn change(client: &mut Client, uri: &str, version: i32, text: &str) {
    client.notify(
        "textDocument/didChange",
        json!({
            "textDocument": { "uri": uri, "version": version },
            "contentChanges": [{ "text": text }],
        }),
    );
}

const MAIN_IMPORTS_GEOMETRY: &str = "import geometry\nfn main() {}\n";
const GEOMETRY_BROKEN: &str = "pub fn area() -> Int { \"s\" }\n";
const GEOMETRY_FIXED: &str = "pub fn area() -> Int { 1 }\n";

#[test]
fn a_module_fixed_on_disk_clears_its_diagnostic() {
    let dir = project(
        "watched",
        &[
            ("main.nova", MAIN_IMPORTS_GEOMETRY),
            ("geometry.nova", GEOMETRY_BROKEN),
        ],
    );
    let main = dir.join("src").join("main.nova");
    let geometry = dir.join("src").join("geometry.nova");
    let mut client = Client::start(&dir, true);
    open(&mut client, &file_uri(&main), MAIN_IMPORTS_GEOMETRY);
    // The project's analysis owns geometry.nova, which is not open.
    let before = client.diagnostics(&file_uri(&geometry), nonempty);
    assert_eq!(codes(&before), ["E0010"]);
    // The server registered its watcher.
    client.wait_for(|m| m["method"] == "client/registerCapability");
    std::fs::write(&geometry, GEOMETRY_FIXED).unwrap();
    client.notify(
        "workspace/didChangeWatchedFiles",
        json!({ "changes": [{ "uri": file_uri(&geometry), "type": 2 }] }),
    );
    client.diagnostics(&file_uri(&geometry), |p| !nonempty(p));
}

#[test]
fn a_burst_of_edits_ends_with_the_last_edits_diagnostics() {
    let dir = fresh_dir("burst");
    let file = dir.join("main.nova");
    std::fs::write(&file, "fn main() {}\n").unwrap();
    let uri = file_uri(&file);
    let mut client = Client::start(&dir, false);
    open(&mut client, &uri, "fn main() {}\n");
    client.diagnostics(&uri, |p| p["version"] == 1);
    // Odd versions hold the planted error; the last, 20, does not.
    for version in 2..=20 {
        let text = if version % 2 == 1 {
            planted()
        } else {
            "fn main() {}\n".to_string()
        };
        change(&mut client, &uri, version, &text);
    }
    // Version 2's check starts at once and is stale before it ends, and
    // versions 3 to 19 collapse unchecked. So the next publish is version
    // 20's: a stale one would come first.
    let next = client.diagnostics(&uri, |_| true);
    assert_eq!(next["version"], 20, "{next}");
    assert!(!nonempty(&next), "{next}");
}

#[test]
fn closing_an_unsaved_buffer_rechecks_from_disk() {
    let dir = project(
        "close",
        &[
            ("main.nova", MAIN_IMPORTS_GEOMETRY),
            ("geometry.nova", GEOMETRY_BROKEN),
        ],
    );
    let main = dir.join("src").join("main.nova");
    let geometry = dir.join("src").join("geometry.nova");
    let geometry_uri = file_uri(&geometry);
    let mut client = Client::start(&dir, false);
    open(&mut client, &file_uri(&main), MAIN_IMPORTS_GEOMETRY);
    // An unsaved buffer that fixes what the disk still has wrong.
    open(&mut client, &geometry_uri, GEOMETRY_FIXED);
    client.diagnostics(&geometry_uri, |p| p["version"] == 1 && !nonempty(p));
    client.clear_unread();
    client.notify(
        "textDocument/didClose",
        json!({ "textDocument": { "uri": geometry_uri } }),
    );
    // main.nova is still open, so the project is re-checked, reading
    // geometry.nova from disk. What the editor ends with is the last
    // publish for geometry.nova under any spelling of its URI.
    let sentinel = sentinel(&mut client, "close");
    let after = client
        .last_diagnostics_before(&geometry_uri, &sentinel)
        .expect("a publish for geometry.nova");
    assert_eq!(codes(&after), ["E0010"], "{after}");
}

#[test]
fn closing_the_last_file_clears_the_project() {
    let dir = project(
        "last",
        &[
            ("main.nova", MAIN_IMPORTS_GEOMETRY),
            ("geometry.nova", GEOMETRY_BROKEN),
        ],
    );
    let main_uri = file_uri(&dir.join("src").join("main.nova"));
    let geometry_uri = file_uri(&dir.join("src").join("geometry.nova"));
    let mut client = Client::start(&dir, false);
    open(&mut client, &main_uri, MAIN_IMPORTS_GEOMETRY);
    client.diagnostics(&geometry_uri, nonempty);
    client.clear_unread();
    client.notify(
        "textDocument/didClose",
        json!({ "textDocument": { "uri": main_uri } }),
    );
    client.diagnostics(&geometry_uri, |p| !nonempty(p));
}

#[test]
fn an_unreached_project_file_is_checked_as_a_module() {
    // A guard more than a new behaviour: Task 10 already checked a file
    // with no `main` as a module. Here the file is in a project whose entry
    // does not import it, which is the ownership rule's case (spec §6.2).
    let dir = project(
        "unreached",
        &[
            ("main.nova", "fn main() {}\n"),
            ("extra.nova", "fn helper() -> Int { 1 }\n"),
        ],
    );
    let extra = dir.join("src").join("extra.nova");
    let mut client = Client::start(&dir, false);
    open(
        &mut client,
        &file_uri(&dir.join("src").join("main.nova")),
        "fn main() {}\n",
    );
    open(&mut client, &file_uri(&extra), "fn helper() -> Int { 1 }\n");
    let params = client.diagnostics(&file_uri(&extra), |p| p["version"] == 1);
    assert!(!nonempty(&params), "no E0601 for a module: {params}");
}

#[test]
fn a_secondary_label_becomes_related_information() {
    let dir = fresh_dir("related");
    let file = dir.join("main.nova");
    let text = "fn a() {}\nfn a() {}\nfn main() {}\n";
    std::fs::write(&file, text).unwrap();
    let uri = file_uri(&file);
    let mut client = Client::start(&dir, false);
    open(&mut client, &uri, text);
    let params = client.diagnostics(&uri, nonempty);
    let d = &params["diagnostics"][0];
    assert_eq!(d["code"], "E0002", "{params}");
    let related = &d["relatedInformation"][0];
    assert_eq!(related["message"], "first defined here");
    assert_eq!(
        related["location"]["range"]["start"],
        json!({ "line": 0, "character": 3 })
    );
}

#[test]
fn a_diagnostic_with_no_place_goes_on_the_entrys_first_line() {
    // MIR's E0601 has no label (spec §6.3's fallback). A project's entry is a
    // program, so a `src/main.nova` with no `main` gets it.
    let dir = project("no-main", &[("main.nova", "fn helper() {}\n")]);
    let uri = file_uri(&dir.join("src").join("main.nova"));
    let mut client = Client::start(&dir, false);
    open(&mut client, &uri, "fn helper() {}\n");
    let params = client.diagnostics(&uri, nonempty);
    assert_eq!(codes(&params), ["E0601"], "{params}");
    assert_eq!(
        params["diagnostics"][0]["range"],
        json!({ "start": { "line": 0, "character": 0 }, "end": { "line": 0, "character": 0 } })
    );
}

// === Task 12: completion ===

/// The labels of a completion response.
fn labels(response: &Value) -> Vec<String> {
    response["result"]
        .as_array()
        .unwrap_or_else(|| panic!("no items: {response}"))
        .iter()
        .map(|i| i["label"].as_str().unwrap().to_string())
        .collect()
}

/// The LSP position just after the first `marker` in `text`.
fn after(text: &str, marker: &str) -> Value {
    let at = text.find(marker).unwrap() + marker.len();
    let line = text[..at].matches('\n').count();
    let line_start = text[..at].rfind('\n').map_or(0, |i| i + 1);
    json!({ "line": line, "character": text[line_start..at].encode_utf16().count() })
}

fn complete(client: &mut Client, uri: &str, position: Value) -> Value {
    client.request(
        "textDocument/completion",
        json!({ "textDocument": { "uri": uri }, "position": position }),
    )
}

/// Start a server on a loose file holding `text`, opened.
fn loose(name: &str, text: &str) -> (Client, String) {
    let dir = fresh_dir(name);
    let file = dir.join("main.nova");
    std::fs::write(&file, text).unwrap();
    let uri = file_uri(&file);
    let mut client = Client::start(&dir, false);
    open(&mut client, &uri, text);
    (client, uri)
}

const BROKEN_WITH_DOTS: &str = "record Point { x: Int, y: Int }\n\
fn broken( {\n}\n\
fn main() {\n    let p = Point { x: 1, y: 2 }\n    let v: Vec<Int> = Vec::new()\n    p.\n    let n = 1\n    v.\n}\n";

#[test]
fn completion_works_in_a_file_with_a_syntax_error() {
    let (mut client, uri) = loose("complete", BROKEN_WITH_DOTS);
    let diagnostics = client.diagnostics(&uri, nonempty);
    assert!(
        codes(&diagnostics).contains(&"P0001".to_string()),
        "{diagnostics}"
    );
    // A record field, after `p.` (gate item 3)...
    let fields = complete(&mut client, &uri, after(BROKEN_WITH_DOTS, "    p."));
    let names = labels(&fields);
    assert!(
        names.contains(&"x".to_string()) && names.contains(&"y".to_string()),
        "{names:?}"
    );
    let x = fields["result"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["label"] == "x")
        .unwrap();
    assert_eq!(x["detail"], "Int");
    // ...and a std method, after `v.`, with its declaration as its detail.
    let methods = complete(&mut client, &uri, after(BROKEN_WITH_DOTS, "    v."));
    let names = labels(&methods);
    assert!(names.contains(&"push".to_string()), "{names:?}");
    assert!(
        !names.contains(&"data".to_string()),
        "std's private field: {names:?}"
    );
    let push = methods["result"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["label"] == "push")
        .unwrap();
    assert_eq!(push["detail"], "fn push(mut self, x: T)");
}

#[test]
fn completion_after_a_dot_followed_by_a_name_on_the_next_line() {
    let text = "fn main() {\n    let s = \"a\"\n    s.\n    println(\"x\")\n}\n";
    let (mut client, uri) = loose("next-line", text);
    let names = labels(&complete(&mut client, &uri, after(text, "    s.")));
    assert!(names.contains(&"len".to_string()), "{names:?}");
}

#[test]
fn completion_offers_locals_names_types_and_keywords() {
    let text = "fn helper() {}\nfn main() {\n    let count = 1\n    \n}\n";
    let (mut client, uri) = loose("names", text);
    let names = labels(&complete(
        &mut client,
        &uri,
        after(text, "let count = 1\n    "),
    ));
    for want in [
        "count", "helper", "main", "println", "Vec", "Int", "let", "match",
    ] {
        assert!(
            names.contains(&want.to_string()),
            "{want} missing from {names:?}"
        );
    }
}

#[test]
fn no_completion_inside_a_string_or_a_comment() {
    let text = "fn main() {\n    let s = \"in a string\" // in a comment\n}\n";
    let (mut client, uri) = loose("literal", text);
    assert!(labels(&complete(&mut client, &uri, after(text, "in a"))).is_empty());
    assert!(labels(&complete(&mut client, &uri, after(text, "// in"))).is_empty());
}

#[test]
fn completion_in_a_file_main_does_not_import_yet() {
    // Review Focus 3: a new module, before `main.nova` imports it.
    let extra = "fn helper() {\n    let s = \"a\"\n    s.\n}\n";
    let dir = project(
        "not-yet",
        &[("main.nova", "fn main() {}\n"), ("extra.nova", extra)],
    );
    let uri = file_uri(&dir.join("src").join("extra.nova"));
    let mut client = Client::start(&dir, false);
    open(&mut client, &uri, extra);
    let names = labels(&complete(&mut client, &uri, after(extra, "    s.")));
    assert!(names.contains(&"len".to_string()), "{names:?}");
}

// === Task 13: formatting ===

/// Format the open document `uri`, and return the response.
fn format(client: &mut Client, uri: &str) -> Value {
    client.request(
        "textDocument/formatting",
        json!({ "textDocument": { "uri": uri }, "options": { "tabSize": 4, "insertSpaces": true } }),
    )
}

/// A loose file holding `text`, opened, in a directory whose `.editorconfig`
/// says `root = true`, so none above it can change line endings.
fn formattable(name: &str, text: &str) -> (Client, String) {
    let dir = fresh_dir(name);
    std::fs::write(dir.join(".editorconfig"), "root = true\n").unwrap();
    let file = dir.join("main.nova");
    std::fs::write(&file, text).unwrap();
    let uri = file_uri(&file);
    let mut client = Client::start(&dir, false);
    open(&mut client, &uri, text);
    (client, uri)
}

#[test]
fn the_document_is_formatted() {
    let text = "fn main() {\nprintln(\"hi\")\n}\n";
    let (mut client, uri) = formattable("format", text);
    let response = format(&mut client, &uri);
    let edits = response["result"]
        .as_array()
        .unwrap_or_else(|| panic!("{response}"));
    assert_eq!(edits.len(), 1, "{response}");
    assert_eq!(edits[0]["newText"], "fn main() { println(\"hi\") }\n");
    assert_eq!(
        edits[0]["range"],
        json!({ "start": { "line": 0, "character": 0 }, "end": { "line": 3, "character": 0 } })
    );
}

#[test]
fn a_formatted_or_broken_buffer_gets_no_edit() {
    let (mut client, uri) = formattable("format-none", "fn main() { println(\"hi\") }\n");
    assert_eq!(format(&mut client, &uri)["result"], json!([]));
    let (mut client, uri) = formattable("format-broken", "fn main( {\n");
    assert_eq!(format(&mut client, &uri)["result"], json!([]));
}

// === Task 14: the latency budget (spec §6.8, §9.5) ===

use std::time::Instant;

/// The largest example, which the budget is measured on.
fn json_api() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("examples")
        .join("05-json-api")
        .join("src")
        .join("main.nova")
}

/// Time `n` edits, each sent when no check is running, and `n` completions,
/// on `05-json-api`. Returns each list in milliseconds, sorted.
fn measure(n: usize) -> (Vec<u128>, Vec<u128>) {
    let path = json_api();
    let text = std::fs::read_to_string(&path).unwrap();
    let uri = file_uri(&path);
    let mut client = Client::start(path.parent().unwrap(), false);
    open(&mut client, &uri, &text);
    client.diagnostics(&uri, |p| p["version"] == 1);
    let mut edits = Vec::new();
    for k in 0..n {
        let version = 2 + k as i32;
        let edited = format!("{text}// edit {k}\n");
        let started = Instant::now();
        change(&mut client, &uri, version, &edited);
        client.diagnostics(&uri, |p| p["version"] == version);
        edits.push(started.elapsed().as_millis());
    }
    // `self.users.` holds a `Map`: completion after it lists std's members.
    let position = after(&text, "        self.users.");
    let mut completions = Vec::new();
    for _ in 0..n {
        let started = Instant::now();
        let response = complete(&mut client, &uri, position.clone());
        completions.push(started.elapsed().as_millis());
        assert!(
            labels(&response).contains(&"insert".to_string()),
            "{response}"
        );
    }
    edits.sort_unstable();
    completions.sort_unstable();
    (edits, completions)
}

fn summary(name: &str, ms: &[u128]) -> String {
    format!(
        "{name}: median {} ms, max {} ms, of {}",
        ms[ms.len() / 2],
        ms[ms.len() - 1],
        ms.len()
    )
}

#[test]
fn edits_and_completions_stay_within_the_ci_bound() {
    // Gate item 8: 2 s each, with the debug binary.
    let (edits, completions) = measure(3);
    let bound = 2000;
    assert!(
        edits.iter().chain(&completions).all(|&ms| ms <= bound),
        "{}; {}; the CI bound is {bound} ms",
        summary("edits", &edits),
        summary("completions", &completions)
    );
}

#[test]
#[ignore = "the development host's figure (spec §9.5): cargo test --release -p nova-cli --test lsp -- --ignored --nocapture latency"]
fn latency_on_05_json_api() {
    let (edits, completions) = measure(20);
    // In a release build, the 200 ms budget. CI's advisory `--ignored` step
    // runs this in a debug build, where only the 2 s CI bound applies.
    let bound = if cfg!(debug_assertions) { 2000 } else { 200 };
    let binary = assert_cmd::cargo::cargo_bin("nova");
    let meta = std::fs::metadata(&binary).unwrap();
    println!("{}", summary("edit to diagnostics", &edits));
    println!("{}", summary("completion", &completions));
    println!(
        "binary {} ({} bytes, modified {:?})",
        binary.display(),
        meta.len(),
        meta.modified().ok()
    );
    assert!(
        edits.iter().chain(&completions).all(|&ms| ms <= bound),
        "the budget is {bound} ms for the median and the maximum"
    );
}

// === The final review's fixes ===

/// Open a loose file in a directory of its own, and return its URI. Its
/// publish comes after every publish of a job submitted before it.
fn sentinel(client: &mut Client, name: &str) -> String {
    let dir = fresh_dir(&format!("{name}-sentinel"));
    let file = dir.join("main.nova");
    std::fs::write(&file, "fn main() {}\n").unwrap();
    let uri = file_uri(&file);
    open(client, &uri, "fn main() {}\n");
    uri
}

#[test]
fn opening_a_reached_module_keeps_its_diagnostics() {
    // geometry.nova is published under the server's spelling of its URI
    // until it is opened, then under the client's. On Windows the two differ
    // (`C:` and `c%3A`) while naming one file, so the old one's clear must
    // not come after the new diagnostics.
    let dir = project(
        "open-reached",
        &[
            ("main.nova", MAIN_IMPORTS_GEOMETRY),
            ("geometry.nova", GEOMETRY_BROKEN),
        ],
    );
    let geometry_uri = file_uri(&dir.join("src").join("geometry.nova"));
    let mut client = Client::start(&dir, false);
    open(
        &mut client,
        &file_uri(&dir.join("src").join("main.nova")),
        MAIN_IMPORTS_GEOMETRY,
    );
    client.diagnostics(&geometry_uri, nonempty);
    client.clear_unread();
    open(&mut client, &geometry_uri, GEOMETRY_BROKEN);
    let sentinel = sentinel(&mut client, "open-reached");
    let last = client
        .last_diagnostics_before(&geometry_uri, &sentinel)
        .expect("a publish for geometry.nova");
    assert_eq!(codes(&last), ["E0010"], "{last}");
}

#[test]
fn a_nested_project_owns_its_own_files() {
    // `outer/nova.toml` and `outer/sub/nova.toml`: a file of `sub` belongs
    // to `sub` alone, so `outer`'s analysis never publishes for it.
    let outer = project("nested", &[("main.nova", "fn main() {}\n")]);
    let sub = outer.join("sub");
    std::fs::create_dir_all(sub.join("src")).unwrap();
    std::fs::write(sub.join("nova.toml"), MANIFEST).unwrap();
    let sub_main = sub.join("src").join("main.nova");
    // No `fn main`: as its project's entry, it gets MIR's E0601.
    std::fs::write(&sub_main, "fn helper() {}\n").unwrap();
    let sub_uri = file_uri(&sub_main);
    let mut client = Client::start(&outer, false);
    open(&mut client, &sub_uri, "fn helper() {}\n");
    client.diagnostics(&sub_uri, nonempty);
    client.clear_unread();
    open(
        &mut client,
        &file_uri(&outer.join("src").join("main.nova")),
        "fn main() {}\n",
    );
    let sentinel = sentinel(&mut client, "nested");
    // Opening the outer project's file need not publish for sub's at all;
    // whatever is published for it must be sub's own E0601.
    if let Some(last) = client.last_diagnostics_before(&sub_uri, &sentinel) {
        assert_eq!(codes(&last), ["E0601"], "outer overwrote sub: {last}");
    }
}

#[test]
fn a_loose_file_is_rechecked_when_a_sibling_it_imports_changes() {
    // No `nova.toml`, as in examples/: main.nova imports geometry.nova from
    // beside it, and each is its own loose project.
    let dir = fresh_dir("loose-siblings");
    let main = dir.join("main.nova");
    let geometry = dir.join("geometry.nova");
    let main_text = "import geometry::{area}\nfn main() {\n    let x: Int = area()\n}\n";
    std::fs::write(&main, main_text).unwrap();
    std::fs::write(&geometry, GEOMETRY_FIXED).unwrap();
    let main_uri = file_uri(&main);
    let geometry_uri = file_uri(&geometry);
    let mut client = Client::start(&dir, false);
    open(&mut client, &main_uri, main_text);
    client.diagnostics(&main_uri, |p| p["version"] == 1 && !nonempty(p));
    open(&mut client, &geometry_uri, GEOMETRY_FIXED);
    // `area` now returns a String, which main.nova's `let x: Int` rejects.
    change(
        &mut client,
        &geometry_uri,
        2,
        "pub fn area() -> String { \"s\" }\n",
    );
    let after = client.diagnostics(&main_uri, nonempty);
    assert_eq!(codes(&after), ["E0010"], "{after}");
}

// === Phase 3.3a: packages (spec
// docs/superpowers/specs/2026-10-08-phase-3-3a-local-packages-design.md §6, §7.4) ===

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

#[test]
fn the_apps_analysis_publishes_nothing_for_its_dependency() {
    let (app, _geom) = app_and_library("dependency-owner", GEOMETRY_BROKEN);
    let main_uri = file_uri(&app.join("src").join("main.nova"));
    let mut client = Client::start(&app, false);
    open(&mut client, &main_uri, APP_MAIN);
    client.diagnostics(&main_uri, |_| true);
    // The first publish for one of geom's files, under any spelling (the
    // app reads them as `app/../geom/...`), or for the sentinel, whose
    // publish comes after everything the app's check sent.
    let sentinel = sentinel(&mut client, "dependency-owner");
    let first = client.wait_for(|m| {
        m["method"] == "textDocument/publishDiagnostics"
            && m["params"]["uri"].as_str().is_some_and(|u| {
                same_uri(u, &sentinel)
                    || u.ends_with("/geom/src/lib.nova")
                    || u.ends_with("/geom/nova.toml")
            })
    });
    let uri = first["params"]["uri"].as_str().unwrap();
    assert!(
        same_uri(uri, &sentinel),
        "the app published for its dependency: {first}"
    );
}

#[test]
fn a_tests_file_gets_its_diagnostics() {
    // A guard: the server already checked an unreached file on its own, and
    // the driver finds a tests/ file's package (Task 4).
    let dir = project("tests-file", &[("lib.nova", GEOMETRY_FIXED)]);
    let text = "import demo\n\n@test\nfn area_is_text() {\n    let s: String = area()\n}\n";
    std::fs::create_dir_all(dir.join("tests")).unwrap();
    let file = dir.join("tests").join("api.nova");
    std::fs::write(&file, text).unwrap();
    let uri = file_uri(&file);
    let mut client = Client::start(&dir, false);
    open(&mut client, &uri, text);
    let params = client.diagnostics(&uri, nonempty);
    assert_eq!(codes(&params), ["E0010"], "{params}");
}

#[test]
fn a_library_without_a_program_is_checked_as_a_module() {
    // A guard: a library gets its own errors, and no E0601.
    let dir = project("library-only", &[("lib.nova", GEOMETRY_BROKEN)]);
    let uri = file_uri(&dir.join("src").join("lib.nova"));
    let mut client = Client::start(&dir, false);
    open(&mut client, &uri, GEOMETRY_BROKEN);
    let params = client.diagnostics(&uri, nonempty);
    assert_eq!(codes(&params), ["E0010"], "{params}");
}

#[test]
fn a_dependency_error_shows_on_the_apps_manifest_entry() {
    let (app, _geom) = app_and_library("dependency-error", GEOMETRY_BROKEN);
    let mut client = Client::start(&app, false);
    open(
        &mut client,
        &file_uri(&app.join("src").join("main.nova")),
        APP_MAIN,
    );
    let params = client.diagnostics(&file_uri(&app.join("nova.toml")), nonempty);
    assert_eq!(codes(&params), ["E0010"], "{params}");
    let d = &params["diagnostics"][0];
    assert_eq!(d["range"]["start"]["line"], 6, "the `geom` entry: {params}");
    let message = d["message"].as_str().unwrap();
    assert!(
        message.contains("(in geom: ") && message.contains("lib.nova:1:"),
        "{message}"
    );
}

#[test]
fn editing_a_dependency_rechecks_its_dependent() {
    let (app, geom) = app_and_library("dependency-edit", GEOMETRY_FIXED);
    let toml = file_uri(&app.join("nova.toml"));
    let lib = file_uri(&geom.join("src").join("lib.nova"));
    let mut client = Client::start(&app, false);
    open(
        &mut client,
        &file_uri(&app.join("src").join("main.nova")),
        APP_MAIN,
    );
    client.diagnostics(&toml, |p| !nonempty(p));
    open(&mut client, &lib, GEOMETRY_FIXED);
    client.diagnostics(&lib, |p| !nonempty(p));
    // geom's buffer, never saved: the app reads it through the overlay.
    change(&mut client, &lib, 2, GEOMETRY_BROKEN);
    let params = client.diagnostics(&toml, nonempty);
    assert_eq!(codes(&params), ["E0010"], "{params}");
}

#[test]
fn a_manifest_error_is_published_under_its_nova_toml() {
    let dir = project("manifest-error", &[("main.nova", "fn main() {}\n")]);
    std::fs::write(dir.join("nova.toml"), MANIFEST.replace("2026", "2021")).unwrap();
    let mut client = Client::start(&dir, false);
    open(
        &mut client,
        &file_uri(&dir.join("src").join("main.nova")),
        "fn main() {}\n",
    );
    let params = client.diagnostics(&file_uri(&dir.join("nova.toml")), nonempty);
    assert_eq!(codes(&params), ["M0003"], "{params}");
    assert_eq!(
        params["diagnostics"][0]["range"]["start"]["line"], 3,
        "{params}"
    );
}

#[test]
fn a_dependencys_manifest_is_published_by_its_own_project_only() {
    let (app, geom) = app_and_library("dependency-manifest", GEOMETRY_FIXED);
    let warned = format!(
        "{}\n[features]\ndefault = []\n",
        MANIFEST.replace("demo", "geom")
    );
    std::fs::write(geom.join("nova.toml"), warned).unwrap();
    let geom_toml = file_uri(&geom.join("nova.toml"));
    let mut client = Client::start(&app, false);
    open(
        &mut client,
        &file_uri(&app.join("src").join("main.nova")),
        APP_MAIN,
    );
    // geom's manifest under any spelling (Task 10's ruling), or the
    // sentinel's publish, which comes after everything the app's check sent.
    let sentinel = sentinel(&mut client, "dependency-manifest");
    let first = client.wait_for(|m| {
        m["method"] == "textDocument/publishDiagnostics"
            && m["params"]["uri"]
                .as_str()
                .is_some_and(|u| same_uri(u, &sentinel) || u.ends_with("/geom/nova.toml"))
    });
    let uri = first["params"]["uri"].as_str().unwrap();
    assert!(
        same_uri(uri, &sentinel),
        "the app published geom's nova.toml: {first}"
    );
    client.clear_unread();
    open(
        &mut client,
        &file_uri(&geom.join("src").join("lib.nova")),
        GEOMETRY_FIXED,
    );
    let params = client.diagnostics(&geom_toml, nonempty);
    assert_eq!(codes(&params), ["M0006"], "{params}");
}

#[test]
fn an_unreached_file_does_not_repeat_the_manifests_diagnostics() {
    // Final review, Important 2: the project's own analysis publishes the
    // graph's problems under nova.toml; a file nothing imports yet must not
    // get them again on its first line.
    let dir = project(
        "unreached-manifest",
        &[
            ("main.nova", "fn main() {}\n"),
            ("shapes.nova", "pub fn area() -> Int { 1 }\n"),
        ],
    );
    std::fs::write(
        dir.join("nova.toml"),
        format!("{MANIFEST}\n[features]\ndefault = []\n"),
    )
    .unwrap();
    let mut client = Client::start(&dir, false);
    open(
        &mut client,
        &file_uri(&dir.join("src").join("main.nova")),
        "fn main() {}\n",
    );
    let toml = client.diagnostics(&file_uri(&dir.join("nova.toml")), nonempty);
    assert_eq!(codes(&toml), ["M0006"], "{toml}");
    let shapes = file_uri(&dir.join("src").join("shapes.nova"));
    open(&mut client, &shapes, "pub fn area() -> Int { 1 }\n");
    let params = client.diagnostics(&shapes, |p| p["version"] == 1);
    assert!(
        !nonempty(&params),
        "the manifest's warning repeated on an unreached file: {params}"
    );
}
// === Phase 3.3b: registry packages (spec
// docs/superpowers/specs/2026-10-09-phase-3-3b-index-and-publishing-design.md
// §5.5, §10.6) ===

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
fn an_entry_not_downloaded_is_m0005_on_its_manifest_entry() {
    // A guard: Task 4's graph, through the server.
    let (app, home) = registry_app("registry-unfetched", "0.1");
    let mut client = Client::start_with_env(&app, false, &[("NOVA_HOME", &home)]);
    open(
        &mut client,
        &file_uri(&app.join("src").join("main.nova")),
        APP_MAIN,
    );
    let params = client.diagnostics(&file_uri(&app.join("nova.toml")), nonempty);
    assert_eq!(codes(&params), ["M0005"], "{params}");
    let message = params["diagnostics"][0]["message"].as_str().unwrap();
    assert!(message.contains("run `nova fetch`"), "{message}");
}

#[test]
fn a_locked_version_that_no_longer_fits_is_m0005() {
    // A guard, as above.
    let (app, home) = registry_app("registry-no-fit", "0.2");
    lock_and_cache(&app, &home, GEOMETRY_FIXED);
    let mut client = Client::start_with_env(&app, false, &[("NOVA_HOME", &home)]);
    open(
        &mut client,
        &file_uri(&app.join("src").join("main.nova")),
        APP_MAIN,
    );
    let params = client.diagnostics(&file_uri(&app.join("nova.toml")), nonempty);
    assert_eq!(codes(&params), ["M0005"], "{params}");
    let message = params["diagnostics"][0]["message"].as_str().unwrap();
    assert!(
        message.contains("is locked at 0.1.0, which does not meet `^0.2`"),
        "{message}"
    );
}

#[test]
fn a_changed_lock_rechecks_the_project() {
    let (app, home) = registry_app("registry-relock", "0.1");
    let toml = file_uri(&app.join("nova.toml"));
    let mut client = Client::start_with_env(&app, true, &[("NOVA_HOME", &home)]);
    open(
        &mut client,
        &file_uri(&app.join("src").join("main.nova")),
        APP_MAIN,
    );
    client.diagnostics(&toml, nonempty);
    let registration = client.wait_for(|m| m["method"] == "client/registerCapability");
    let globs: Vec<String> = registration["params"]["registrations"][0]["registerOptions"]
        ["watchers"]
        .as_array()
        .unwrap()
        .iter()
        .map(|w| w["globPattern"].as_str().unwrap().to_string())
        .collect();
    assert!(globs.iter().any(|g| g == "**/nova.lock"), "{globs:?}");
    // What `nova fetch` leaves: the package unpacked, then the lock.
    lock_and_cache(&app, &home, GEOMETRY_FIXED);
    client.notify(
        "workspace/didChangeWatchedFiles",
        json!({ "changes": [{ "uri": file_uri(&app.join("nova.lock")), "type": 1 }] }),
    );
    client.diagnostics(&toml, |p| !nonempty(p));
}

#[test]
fn a_file_in_the_cache_gets_nothing_published() {
    let (app, home) = registry_app("registry-cached-file", "0.1");
    let geom = lock_and_cache(&app, &home, GEOMETRY_BROKEN);
    let mut client = Client::start_with_env(&app, false, &[("NOVA_HOME", &home)]);
    client.clear_unread();
    let lib = file_uri(&geom.join("src").join("lib.nova"));
    open(&mut client, &lib, GEOMETRY_BROKEN);
    let sentinel = sentinel(&mut client, "registry-cached-file");
    assert_eq!(client.last_diagnostics_before(&lib, &sentinel), None);
}

#[test]
fn a_dependencys_warning_is_not_shown() {
    // A guard: Task 5's dropping, through the server.
    let (app, home) = registry_app("registry-warning", "0.1");
    lock_and_cache(
        &app,
        &home,
        "pub fn area() -> Int {\n    match 1 { _ => 1, 0 => 2 }\n}\n",
    );
    let mut client = Client::start_with_env(&app, false, &[("NOVA_HOME", &home)]);
    let main = file_uri(&app.join("src").join("main.nova"));
    open(&mut client, &main, APP_MAIN);
    let params = client.diagnostics(&main, |_| true);
    assert!(!nonempty(&params), "{params}");
    let params = client.diagnostics(&file_uri(&app.join("nova.toml")), |_| true);
    assert!(!nonempty(&params), "{params}");
}
