//! The Nova language server, `nova lsp` (spec
//! `docs/superpowers/specs/2026-10-08-phase-3-2-lsp-core-design.md` §6;
//! ADR 0029).
//!
//! The protocol runs on the calling thread through `lsp-server`, and
//! analyses run on the checker thread. Stdout carries only protocol
//! messages; logs go through `tracing`, which `nova lsp` sends to stderr.

mod checker;
mod convert;
mod uri;
mod workspace;

use anyhow::Result;
use lsp_server::{Connection, ErrorCode, Message, Notification, Request, Response};
use lsp_types as lsp;
use lsp_types::notification::{
    DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument, DidSaveTextDocument, Exit,
    Notification as _, PublishDiagnostics,
};

use checker::{Checker, Job, Publish};
use workspace::Workspace;

/// Serve over stdin and stdout until the client says `exit`. Returns the
/// exit code: 0 after `shutdown` then `exit`, 1 for an `exit` without
/// `shutdown` or a closed stream.
pub fn run() -> Result<i32> {
    let (connection, io_threads) = Connection::stdio();
    let code = serve(&connection)?;
    drop(connection);
    io_threads.join()?;
    Ok(code)
}

fn serve(connection: &Connection) -> Result<i32> {
    let (id, params) = connection.initialize_start()?;
    let _params: lsp::InitializeParams = serde_json::from_value(params)?;
    let result = serde_json::json!({
        "capabilities": capabilities(),
        "serverInfo": { "name": "nova lsp", "version": env!("CARGO_PKG_VERSION") },
    });
    connection.initialize_finish(id, result)?;

    let sender = connection.sender.clone();
    let checker = Checker::spawn(move |p: Publish| {
        let Some(uri) = uri::parse(&p.uri) else {
            tracing::warn!("nova lsp: cannot publish for {}", p.uri);
            return;
        };
        let params = lsp::PublishDiagnosticsParams {
            uri,
            diagnostics: p.diagnostics,
            version: p.version,
        };
        let _ = sender.send(Message::Notification(Notification::new(
            PublishDiagnostics::METHOD.to_string(),
            params,
        )));
    });
    let mut server = Server {
        connection,
        workspace: Workspace::default(),
        checker,
    };
    for message in &connection.receiver {
        match message {
            Message::Request(request) => {
                if connection.handle_shutdown(&request)? {
                    return Ok(0);
                }
                server.request(request);
            }
            Message::Notification(n) if n.method == Exit::METHOD => return Ok(1),
            Message::Notification(n) => server.notification(n),
            Message::Response(_) => {}
        }
    }
    Ok(1)
}

fn capabilities() -> lsp::ServerCapabilities {
    lsp::ServerCapabilities {
        position_encoding: Some(lsp::PositionEncodingKind::UTF16),
        text_document_sync: Some(lsp::TextDocumentSyncCapability::Options(
            lsp::TextDocumentSyncOptions {
                open_close: Some(true),
                change: Some(lsp::TextDocumentSyncKind::FULL),
                save: Some(lsp::TextDocumentSyncSaveOptions::Supported(true)),
                ..Default::default()
            },
        )),
        ..Default::default()
    }
}

struct Server<'c> {
    connection: &'c Connection,
    workspace: Workspace,
    checker: Checker,
}

impl Server<'_> {
    fn notification(&mut self, n: Notification) {
        match n.method.as_str() {
            DidOpenTextDocument::METHOD => {
                if let Ok(p) = serde_json::from_value::<lsp::DidOpenTextDocumentParams>(n.params) {
                    let uri = uri::text(&p.text_document.uri);
                    self.workspace
                        .open(&uri, p.text_document.version, p.text_document.text);
                    self.check(&uri);
                }
            }
            DidChangeTextDocument::METHOD => {
                if let Ok(p) = serde_json::from_value::<lsp::DidChangeTextDocumentParams>(n.params)
                {
                    let uri = uri::text(&p.text_document.uri);
                    // Full sync: the last change holds the whole text.
                    if let Some(change) = p.content_changes.into_iter().last() {
                        self.workspace
                            .change(&uri, p.text_document.version, change.text);
                        self.check(&uri);
                    }
                }
            }
            DidSaveTextDocument::METHOD => {
                if let Ok(p) = serde_json::from_value::<lsp::DidSaveTextDocumentParams>(n.params) {
                    self.check(&uri::text(&p.text_document.uri));
                }
            }
            DidCloseTextDocument::METHOD => {
                if let Ok(p) = serde_json::from_value::<lsp::DidCloseTextDocumentParams>(n.params) {
                    if let Some(doc) = self.workspace.close(&uri::text(&p.text_document.uri)) {
                        self.checker.submit(Job {
                            path: doc.path,
                            uri: doc.uri,
                            version: None,
                            overlay: self.workspace.overlay(),
                            clear: true,
                        });
                    }
                }
            }
            _ => {}
        }
    }

    fn request(&mut self, request: Request) {
        let response = Response::new_err(
            request.id,
            ErrorCode::MethodNotFound as i32,
            format!("nova lsp does not handle {}", request.method),
        );
        self.respond(response);
    }

    fn respond(&self, response: Response) {
        let _ = self.connection.sender.send(Message::Response(response));
    }

    /// Queue a check of an open document.
    fn check(&self, uri: &str) {
        let Some(doc) = self.workspace.get(uri) else {
            return;
        };
        self.checker.submit(Job {
            path: doc.path.clone(),
            uri: doc.uri.clone(),
            version: Some(doc.version),
            overlay: self.workspace.overlay(),
            clear: false,
        });
    }
}
