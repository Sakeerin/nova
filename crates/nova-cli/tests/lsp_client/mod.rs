//! A Language Server Protocol client for the tests. It frames JSON-RPC over
//! a `nova lsp` child's stdin and stdout (spec
//! `docs/superpowers/specs/2026-10-08-phase-3-2-lsp-core-design.md` §9.3).

#![allow(dead_code)]

use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, ExitStatus, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

use serde_json::{json, Value};

/// How long any one wait lasts before the test fails.
pub const WAIT: Duration = Duration::from_secs(30);

pub struct Client {
    child: Child,
    stdin: Option<ChildStdin>,
    messages: Receiver<Value>,
    next_id: i64,
    /// Messages read while waiting for another, oldest first.
    unread: Vec<Value>,
}

impl Client {
    /// Start `nova lsp` and initialize it. With `watch`, the client offers
    /// dynamic registration of watched files, as VS Code does.
    pub fn start(root: &Path, watch: bool) -> Client {
        let mut child = Command::new(assert_cmd::cargo::cargo_bin("nova"))
            .arg("lsp")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .expect("start nova lsp");
        let stdin = child.stdin.take();
        let stdout = child.stdout.take().expect("piped stdout");
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            while let Some(message) = read_message(&mut reader) {
                if tx.send(message).is_err() {
                    break;
                }
            }
        });
        let mut client = Client {
            child,
            stdin,
            messages: rx,
            next_id: 1,
            unread: Vec::new(),
        };
        let capabilities = if watch {
            json!({ "workspace": { "didChangeWatchedFiles": { "dynamicRegistration": true } } })
        } else {
            json!({})
        };
        client.request(
            "initialize",
            json!({ "processId": null, "rootUri": file_uri(root), "capabilities": capabilities }),
        );
        client.notify("initialized", json!({}));
        client
    }

    pub fn notify(&mut self, method: &str, params: Value) {
        self.send(&json!({ "jsonrpc": "2.0", "method": method, "params": params }));
    }

    /// Send a request and return its response.
    pub fn request(&mut self, method: &str, params: Value) -> Value {
        let id = self.next_id;
        self.next_id += 1;
        self.send(&json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }));
        self.wait_for(|m| m.get("method").is_none() && m.get("id") == Some(&json!(id)))
    }

    /// The first message, old or new, that `want` accepts. The others are
    /// kept for later waits. A request from the server, such as
    /// `client/registerCapability`, is answered with `null` as it arrives.
    pub fn wait_for(&mut self, want: impl Fn(&Value) -> bool) -> Value {
        if let Some(i) = self.unread.iter().position(&want) {
            return self.unread.remove(i);
        }
        let deadline = Instant::now() + WAIT;
        loop {
            let left = deadline.saturating_duration_since(Instant::now());
            let message = self.messages.recv_timeout(left).unwrap_or_else(|_| {
                panic!(
                    "no matching message within {WAIT:?}; read but unmatched: {:#?}",
                    self.unread
                )
            });
            if message.get("method").is_some() && message.get("id").is_some() {
                let id = message["id"].clone();
                self.send(&json!({ "jsonrpc": "2.0", "id": id, "result": null }));
            }
            if want(&message) {
                return message;
            }
            self.unread.push(message);
        }
    }

    /// The params of the next `textDocument/publishDiagnostics` for `uri`
    /// that `want` accepts.
    pub fn diagnostics(&mut self, uri: &str, want: impl Fn(&Value) -> bool) -> Value {
        let message = self.wait_for(|m| {
            m["method"] == "textDocument/publishDiagnostics"
                && m["params"]["uri"]
                    .as_str()
                    .is_some_and(|u| same_uri(u, uri))
                && want(&m["params"])
        });
        message["params"].clone()
    }

    /// Forget every message read but not yet matched.
    pub fn clear_unread(&mut self) {
        self.unread.clear();
    }

    /// What an editor shows for `uri` once every publish before the first
    /// one for `sentinel` has arrived: the params of the last of them that
    /// `same_uri` matches, as VS Code keeps one entry per normalised URI.
    /// The server publishes in the order its jobs were submitted, so open
    /// `sentinel` after the action under test. Clear the unread messages
    /// before that action.
    pub fn last_diagnostics_before(&mut self, uri: &str, sentinel: &str) -> Option<Value> {
        let is_publish_for = |m: &Value, u: &str| {
            m["method"] == "textDocument/publishDiagnostics"
                && m["params"]["uri"].as_str().is_some_and(|p| same_uri(p, u))
        };
        self.wait_for(|m| is_publish_for(m, sentinel));
        self.unread
            .iter()
            .filter(|m| is_publish_for(m, uri))
            .last()
            .map(|m| m["params"].clone())
    }

    /// `shutdown`, then `exit`; returns the server's exit status.
    pub fn shutdown_and_exit(mut self) -> ExitStatus {
        self.request("shutdown", Value::Null);
        self.notify("exit", Value::Null);
        self.wait()
    }

    /// `exit` with no `shutdown` first.
    pub fn exit_without_shutdown(mut self) -> ExitStatus {
        self.notify("exit", Value::Null);
        self.wait()
    }

    fn wait(&mut self) -> ExitStatus {
        self.stdin.take();
        let deadline = Instant::now() + WAIT;
        loop {
            if let Some(status) = self.child.try_wait().expect("poll nova lsp") {
                return status;
            }
            assert!(Instant::now() < deadline, "nova lsp did not exit");
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    fn send(&mut self, message: &Value) {
        let body = serde_json::to_vec(message).expect("serialize");
        let stdin = self.stdin.as_mut().expect("stdin is open");
        write!(stdin, "Content-Length: {}\r\n\r\n", body.len()).expect("write header");
        stdin.write_all(&body).expect("write body");
        stdin.flush().expect("flush");
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn read_message(reader: &mut impl BufRead) -> Option<Value> {
    let mut length = None;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).ok()? == 0 {
            return None;
        }
        let line = line.trim_end();
        if line.is_empty() {
            break;
        }
        if let Some(n) = line.strip_prefix("Content-Length: ") {
            length = n.parse().ok();
        }
    }
    let mut body = vec![0; length?];
    reader.read_exact(&mut body).ok()?;
    serde_json::from_slice(&body).ok()
}

/// A `file:` URI for `path`, as VS Code writes one: the drive letter lower
/// case and its `:` as `%3A`, and every byte outside `A-Za-z0-9-._~/`
/// percent-encoded.
pub fn file_uri(path: &Path) -> String {
    let mut s = path.to_string_lossy().replace('\\', "/");
    if s.as_bytes().get(1) == Some(&b':') {
        s = format!("{}{}", s[..1].to_lowercase(), &s[1..]);
    }
    if !s.starts_with('/') {
        s.insert(0, '/');
    }
    let mut out = String::from("file://");
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b"-._~/".contains(&b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// Whether two URIs name the same thing: equal after percent-decoding, and
/// on Windows ignoring case. The server publishes an unopened file under its
/// own spelling of the URI.
pub fn same_uri(a: &str, b: &str) -> bool {
    let norm = |s: &str| {
        let decoded = decode(s);
        if cfg!(windows) {
            decoded.to_lowercase()
        } else {
            decoded
        }
    };
    norm(a) == norm(b)
}

fn decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(b) = u8::from_str_radix(&String::from_utf8_lossy(&bytes[i + 1..i + 3]), 16) {
                out.push(b);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// A fresh directory with a space and Thai in its path, so every URI is
/// percent-encoded (Review Focus 2). Its name is fixed, so each run
/// replaces the last.
///
/// It is spelled as the file system spells it, as an editor opens a file.
/// The temp directory can be named another way: by an 8.3 short name on
/// Windows (`C:\Users\RUNNER~1\...`), or through the `/var` symlink on
/// macOS. The server names a file it publishes for unopened in its real
/// spelling, which a URI built on the other would not match.
pub fn fresh_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir()
        .join("nova lsp tests")
        .join(format!("โปรเจกต์-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create the test directory");
    let real = std::fs::canonicalize(&dir).expect("canonicalize the test directory");
    match real.to_str().and_then(|s| s.strip_prefix(r"\\?\")) {
        Some(rest) => PathBuf::from(rest),
        None => real,
    }
}
