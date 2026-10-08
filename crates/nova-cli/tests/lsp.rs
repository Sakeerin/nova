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
