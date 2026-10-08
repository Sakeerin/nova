//! The Nova language server, `nova lsp` (spec
//! `docs/superpowers/specs/2026-10-08-phase-3-2-lsp-core-design.md` §6;
//! ADR 0029).
//!
//! The protocol runs on the calling thread through `lsp-server`, and
//! analyses run on the checker thread. Stdout carries only protocol
//! messages; logs go through `tracing`, which `nova lsp` sends to stderr.

mod checker;
mod completion;
mod convert;
mod formatting;
mod uri;
mod workspace;

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use anyhow::Result;
use lsp_server::{Connection, ErrorCode, Message, Notification, Request, Response};
use lsp_types as lsp;
use lsp_types::notification::{
    DidChangeTextDocument, DidChangeWatchedFiles, DidCloseTextDocument, DidOpenTextDocument,
    DidSaveTextDocument, Exit, Notification as _, PublishDiagnostics,
};
use lsp_types::request::{Completion, Formatting, RegisterCapability, Request as _};
use nova_diagnostics::LineIndex;

use checker::{Checker, Job, Publish};
use workspace::{ProjectKey, Workspace};

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
    let params: lsp::InitializeParams = serde_json::from_value(params)?;
    let can_watch = params
        .capabilities
        .workspace
        .as_ref()
        .and_then(|w| w.did_change_watched_files.as_ref())
        .and_then(|d| d.dynamic_registration)
        .unwrap_or(false);
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
        generation: 0,
        active: HashSet::new(),
    };
    if can_watch {
        server.register_watcher();
    }
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
        completion_provider: Some(lsp::CompletionOptions {
            trigger_characters: Some(vec![".".to_string()]),
            ..Default::default()
        }),
        document_formatting_provider: Some(lsp::OneOf::Left(true)),
        ..Default::default()
    }
}

struct Server<'c> {
    connection: &'c Connection,
    workspace: Workspace,
    checker: Checker,
    /// Rises with every job submitted.
    generation: u64,
    /// The projects of the open documents, as of the last refresh.
    active: HashSet<ProjectKey>,
}

impl Server<'_> {
    fn notification(&mut self, n: Notification) {
        match n.method.as_str() {
            DidOpenTextDocument::METHOD => {
                if let Ok(p) = serde_json::from_value::<lsp::DidOpenTextDocumentParams>(n.params) {
                    let uri = uri::text(&p.text_document.uri);
                    if let Some(path) =
                        self.workspace
                            .open(&uri, p.text_document.version, p.text_document.text)
                    {
                        self.refresh(self.affected(&path));
                    }
                }
            }
            DidChangeTextDocument::METHOD => {
                if let Ok(p) = serde_json::from_value::<lsp::DidChangeTextDocumentParams>(n.params)
                {
                    let uri = uri::text(&p.text_document.uri);
                    // Full sync: the last change holds the whole text.
                    if let Some(change) = p.content_changes.into_iter().last() {
                        if let Some(path) =
                            self.workspace
                                .change(&uri, p.text_document.version, change.text)
                        {
                            self.refresh(self.affected(&path));
                        }
                    }
                }
            }
            DidSaveTextDocument::METHOD => {
                if let Ok(p) = serde_json::from_value::<lsp::DidSaveTextDocumentParams>(n.params) {
                    if let Some(path) = uri::document_path(&uri::text(&p.text_document.uri)) {
                        self.refresh(self.affected(&path));
                    }
                }
            }
            DidCloseTextDocument::METHOD => {
                if let Ok(p) = serde_json::from_value::<lsp::DidCloseTextDocumentParams>(n.params) {
                    if let Some(doc) = self.workspace.close(&uri::text(&p.text_document.uri)) {
                        // Re-checked from disk if the project is still
                        // open; cleared if not.
                        self.refresh(self.affected(&doc.path));
                    }
                }
            }
            DidChangeWatchedFiles::METHOD => {
                if let Ok(p) = serde_json::from_value::<lsp::DidChangeWatchedFilesParams>(n.params)
                {
                    let paths: Vec<PathBuf> = p
                        .changes
                        .iter()
                        .filter_map(|c| uri::to_path(&uri::text(&c.uri)))
                        .collect();
                    let manifest = paths.iter().any(|p| p.ends_with("nova.toml"));
                    let touched: Vec<ProjectKey> = self
                        .workspace
                        .projects()
                        .into_iter()
                        .filter(|project| manifest || paths.iter().any(|p| project.holds(p)))
                        .collect();
                    self.refresh(touched);
                }
            }
            _ => {}
        }
    }

    fn request(&mut self, request: Request) {
        let response = match request.method.as_str() {
            Completion::METHOD => {
                match serde_json::from_value::<lsp::CompletionParams>(request.params) {
                    Ok(p) => {
                        let at = p.text_document_position;
                        let uri = uri::text(&at.text_document.uri);
                        let items = match self.workspace.get(&uri) {
                            Some(doc) => {
                                let offset = LineIndex::new(&doc.text)
                                    .offset(at.position.line, at.position.character);
                                completion::complete(
                                    &doc.path,
                                    &doc.text,
                                    offset,
                                    &self.workspace.overlay(),
                                )
                            }
                            None => Vec::new(),
                        };
                        Response::new_ok(request.id, lsp::CompletionResponse::Array(items))
                    }
                    Err(e) => Response::new_err(
                        request.id,
                        ErrorCode::InvalidParams as i32,
                        e.to_string(),
                    ),
                }
            }
            Formatting::METHOD => {
                match serde_json::from_value::<lsp::DocumentFormattingParams>(request.params) {
                    Ok(p) => {
                        let uri = uri::text(&p.text_document.uri);
                        let edits = match self.workspace.get(&uri) {
                            Some(doc) => formatting::format_document(&doc.path, &doc.text),
                            None => Vec::new(),
                        };
                        Response::new_ok(request.id, edits)
                    }
                    Err(e) => Response::new_err(
                        request.id,
                        ErrorCode::InvalidParams as i32,
                        e.to_string(),
                    ),
                }
            }
            _ => Response::new_err(
                request.id,
                ErrorCode::MethodNotFound as i32,
                format!("nova lsp does not handle {}", request.method),
            ),
        };
        self.respond(response);
    }

    fn respond(&self, response: Response) {
        let _ = self.connection.sender.send(Message::Response(response));
    }

    /// The projects a change to `path` can affect: its own, and every open
    /// project that holds it. A loose file's analysis reads the modules
    /// beside it, so editing one re-checks the loose files that may import
    /// it.
    fn affected(&self, path: &Path) -> Vec<ProjectKey> {
        let mut out = vec![ProjectKey::of(path)];
        for project in self.workspace.projects() {
            if project.holds(path) && !out.contains(&project) {
                out.push(project);
            }
        }
        out
    }

    /// Re-check `touched`, and clear each project that no open document
    /// belongs to any more (spec §6.2).
    fn refresh(&mut self, touched: Vec<ProjectKey>) {
        let now: HashSet<ProjectKey> = self.workspace.projects().into_iter().collect();
        let gone: Vec<ProjectKey> = self.active.difference(&now).cloned().collect();
        for project in gone {
            self.submit(project, true);
        }
        for project in touched {
            if now.contains(&project) {
                self.submit(project, false);
            }
        }
        self.active = now;
    }

    fn submit(&mut self, project: ProjectKey, clear: bool) {
        self.generation += 1;
        let open = self.workspace.open_in(&project);
        self.checker.submit(Job {
            project,
            generation: self.generation,
            overlay: self.workspace.overlay(),
            open,
            clear,
        });
    }

    /// Ask the client to watch `.nova` files and `nova.toml` (spec §6.1).
    fn register_watcher(&self) {
        let watchers = ["**/*.nova", "**/nova.toml"]
            .iter()
            .map(|glob| lsp::FileSystemWatcher {
                glob_pattern: lsp::GlobPattern::String(glob.to_string()),
                kind: None,
            })
            .collect();
        let options = lsp::DidChangeWatchedFilesRegistrationOptions { watchers };
        let params = lsp::RegistrationParams {
            registrations: vec![lsp::Registration {
                id: "nova-watch".to_string(),
                method: DidChangeWatchedFiles::METHOD.to_string(),
                register_options: serde_json::to_value(options).ok(),
            }],
        };
        let request = Request::new(
            lsp_server::RequestId::from("nova-watch".to_string()),
            RegisterCapability::METHOD.to_string(),
            params,
        );
        let _ = self.connection.sender.send(Message::Request(request));
    }
}
