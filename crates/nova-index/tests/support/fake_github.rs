//! A stand-in for GitHub (plan Task 12): the Contents API for one
//! repository's files, its releases and their assets, the files raw as
//! raw.githubusercontent.com serves them (under `/raw/`), and the assets
//! as release downloads (under `/dl/`). Every request is recorded by the
//! `Server` underneath.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use base64::Engine;

use super::{Request, Response, Server};

/// A release asset: its id, its name and its bytes.
pub type Asset = (u64, String, Vec<u8>);

/// What the stand-in holds.
#[derive(Default)]
pub struct State {
    /// Each file by path: its text and its blob sha.
    pub files: HashMap<String, (String, String)>,
    /// Each release by tag: its id and its assets.
    pub releases: HashMap<String, (u64, Vec<Asset>)>,
    /// The last id handed out.
    pub next_id: u64,
    /// Whether the token may push.
    pub push: bool,
    /// How many writes to refuse with 409, each after appending
    /// `interloper` to the file, as another publisher would.
    pub conflicts: usize,
    pub interloper: Option<String>,
}

impl State {
    fn id(&mut self) -> u64 {
        self.next_id += 1;
        self.next_id
    }

    fn put(&mut self, path: &str, text: String) {
        let sha = format!("sha-{}", self.id());
        self.files.insert(path.to_string(), (text, sha));
    }
}

pub struct FakeGitHub {
    pub server: Server,
    pub state: Arc<Mutex<State>>,
}

impl FakeGitHub {
    pub const TOKEN: &'static str = "ghp_test";
    pub const REPO: &'static str = "owner/index";

    /// The stand-in, its repository holding `config` as config.json, with
    /// push access.
    pub fn start(config: &str) -> FakeGitHub {
        let mut state = State {
            push: true,
            ..State::default()
        };
        state.put("config.json", config.to_string());
        let state = Arc::new(Mutex::new(state));
        let shared = Arc::clone(&state);
        let server = Server::start(move |request| handle(&shared, request));
        FakeGitHub { server, state }
    }

    /// What `NOVA_GITHUB_API` is set to.
    pub fn api(&self) -> String {
        self.server.url.clone()
    }

    /// The text of a file in the repository.
    pub fn file(&self, path: &str) -> Option<String> {
        self.state
            .lock()
            .unwrap()
            .files
            .get(path)
            .map(|(text, _)| text.clone())
    }
}

fn json(status: u16, value: serde_json::Value) -> Response {
    Response::with_status(status, value.to_string())
}

fn release_json(id: u64, assets: &[Asset]) -> serde_json::Value {
    let assets: Vec<serde_json::Value> = assets
        .iter()
        .map(|(id, name, _)| serde_json::json!({"id": id, "name": name}))
        .collect();
    serde_json::json!({"id": id, "assets": assets})
}

fn handle(state: &Mutex<State>, request: &Request) -> Response {
    let mut state = state.lock().unwrap();
    let (path, query) = request
        .path
        .split_once('?')
        .unwrap_or((request.path.as_str(), ""));
    if let Some(file) = path.strip_prefix("/raw/") {
        return match state.files.get(file) {
            Some((text, _)) => Response::ok(text.clone()),
            None => Response::status(404),
        };
    }
    if let Some(name) = path.strip_prefix("/dl/") {
        let found = state
            .releases
            .values()
            .flat_map(|(_, assets)| assets)
            .find(|(_, asset, _)| asset == name)
            .map(|(_, _, bytes)| bytes.clone());
        return found.map_or(Response::status(404), Response::ok);
    }
    let bearer = format!("Bearer {}", FakeGitHub::TOKEN);
    if request.header("authorization") != Some(bearer.as_str()) {
        return json(401, serde_json::json!({"message": "Bad credentials"}));
    }
    let Some(rest) = path.strip_prefix(&format!("/repos/{}", FakeGitHub::REPO)) else {
        return Response::status(404);
    };
    let base64 = base64::engine::general_purpose::STANDARD;
    match request.method.as_str() {
        "GET" if rest.is_empty() => json(
            200,
            serde_json::json!({"permissions": {"push": state.push}}),
        ),
        "GET" if rest.starts_with("/contents/") => {
            match state.files.get(&rest["/contents/".len()..]) {
                Some((text, sha)) => {
                    // GitHub breaks the base64 into lines.
                    let encoded = base64.encode(text);
                    let lines: Vec<&str> = encoded
                        .as_bytes()
                        .chunks(60)
                        .map(|chunk| std::str::from_utf8(chunk).unwrap())
                        .collect();
                    json(
                        200,
                        serde_json::json!({"content": lines.join("\n"), "sha": sha, "encoding": "base64"}),
                    )
                }
                None => json(404, serde_json::json!({"message": "Not Found"})),
            }
        }
        "PUT" if rest.starts_with("/contents/") => {
            let file = rest["/contents/".len()..].to_string();
            let body: serde_json::Value = serde_json::from_slice(&request.body).unwrap();
            if state.conflicts > 0 {
                state.conflicts -= 1;
                let mut text = state
                    .files
                    .get(&file)
                    .map(|(text, _)| text.clone())
                    .unwrap_or_default();
                text.push_str(state.interloper.as_deref().unwrap_or(""));
                state.put(&file, text);
                return json(409, serde_json::json!({"message": "is at a different sha"}));
            }
            let current = state.files.get(&file).map(|(_, sha)| sha.clone());
            if body["sha"].as_str().map(str::to_string) != current {
                return json(409, serde_json::json!({"message": "sha does not match"}));
            }
            let content = base64.decode(body["content"].as_str().unwrap()).unwrap();
            state.put(&file, String::from_utf8(content).unwrap());
            json(201, serde_json::json!({"content": {"path": file}}))
        }
        "GET" if rest.starts_with("/releases/tags/") => {
            match state.releases.get(&rest["/releases/tags/".len()..]) {
                Some((id, assets)) => json(200, release_json(*id, assets)),
                None => json(404, serde_json::json!({"message": "Not Found"})),
            }
        }
        "POST" if rest == "/releases" => {
            let body: serde_json::Value = serde_json::from_slice(&request.body).unwrap();
            let tag = body["tag_name"].as_str().unwrap().to_string();
            let id = state.id();
            state.releases.insert(tag, (id, Vec::new()));
            json(201, release_json(id, &[]))
        }
        "DELETE" if rest.starts_with("/releases/assets/") => {
            let id: u64 = rest["/releases/assets/".len()..].parse().unwrap();
            for (_, assets) in state.releases.values_mut() {
                assets.retain(|(asset, _, _)| *asset != id);
            }
            Response::status(204)
        }
        "POST" if rest.starts_with("/releases/") && rest.ends_with("/assets") => {
            let release: u64 = rest["/releases/".len()..rest.len() - "/assets".len()]
                .parse()
                .unwrap();
            let name = query.strip_prefix("name=").unwrap_or("").to_string();
            let id = state.id();
            let body = request.body.clone();
            match state
                .releases
                .values_mut()
                .find(|(release_id, _)| *release_id == release)
            {
                Some((_, assets)) => {
                    assets.push((id, name.clone(), body));
                    json(201, serde_json::json!({"id": id, "name": name}))
                }
                None => Response::status(404),
            }
        }
        _ => Response::status(404),
    }
}
