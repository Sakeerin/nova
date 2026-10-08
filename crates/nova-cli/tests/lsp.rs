//! `nova lsp`, driven over stdio (spec
//! `docs/superpowers/specs/2026-10-08-phase-3-2-lsp-core-design.md` §9.3).

mod lsp_client;

use lsp_client::{file_uri, fresh_dir, Client};
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
    // geometry.nova from disk.
    let after = client.diagnostics(&geometry_uri, nonempty);
    assert_eq!(codes(&after), ["E0010"]);
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
