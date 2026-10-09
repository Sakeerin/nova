//! Publishing to an index on GitHub (spec §7, §8): the index's files
//! through the Contents API, and each version's tarball as a release
//! asset. Every request here carries the token, and goes only to
//! api.github.com and uploads.github.com, or to a loopback
//! `NOVA_GITHUB_API` in tests. None follows a redirect.

use base64::Engine;

use crate::http::{shown, Http};
use crate::line::Line;
use crate::location::index_path;
use crate::pack::MAX_TARBALL;
use crate::publish::check_new;
use crate::read::Reader;

const API: &str = "https://api.github.com";
const UPLOADS: &str = "https://uploads.github.com";

/// The GitHub repository behind an index, and a token for it.
pub struct GitHub {
    http: Http,
    token: String,
    api: String,
    uploads: String,
    /// `owner/name`, from the index's config.json `api`.
    pub repo: String,
}

/// What a file write did.
#[derive(Debug, PartialEq, Eq)]
pub enum Written {
    Done,
    /// The file changed since it was read.
    Conflict,
}

/// A release: its id, and its assets as `(id, name)`.
#[derive(Debug)]
pub struct Release {
    pub id: u64,
    pub assets: Vec<(u64, String)>,
}

type Answer = (u16, Vec<u8>);

impl GitHub {
    /// The API for `repo`, with `token`. `api`, for tests, replaces both API
    /// hosts, and may only be `http://127.0.0.1:<port>` or
    /// `http://[::1]:<port>` (spec §8).
    pub fn new(repo: &str, token: &str, api: Option<&str>) -> Result<GitHub, String> {
        let valid = |part: &str| {
            !part.is_empty()
                && part
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || "-_.".contains(c))
        };
        if !repo
            .split_once('/')
            .is_some_and(|(owner, name)| valid(owner) && valid(name))
        {
            return Err(format!(
                "config.json's `api` must be `owner/repository`, not `{repo}`"
            ));
        }
        let (api, uploads) = match api.filter(|a| !a.is_empty()) {
            None => (API.to_string(), UPLOADS.to_string()),
            Some(url) => {
                loopback_only(url)?;
                let base = url.trim_end_matches('/').to_string();
                (base.clone(), base)
            }
        };
        Ok(GitHub {
            http: Http::new(),
            token: token.to_string(),
            api,
            uploads,
            repo: repo.to_string(),
        })
    }

    /// [`GitHub::new`], with `NOVA_GITHUB_API` from the environment.
    pub fn from_env(repo: &str, token: &str) -> Result<GitHub, String> {
        let api = std::env::var("NOVA_GITHUB_API").ok();
        GitHub::new(repo, token, api.as_deref())
    }

    fn url(&self, rest: &str) -> String {
        format!("{}/repos/{}{rest}", self.api, self.repo)
    }

    fn authorized<B>(&self, request: ureq::RequestBuilder<B>) -> ureq::RequestBuilder<B> {
        request
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
    }

    fn get(&self, url: &str) -> Result<Answer, String> {
        answer(self.authorized(self.http.agent().get(url)).call(), url)
    }

    fn delete(&self, url: &str) -> Result<Answer, String> {
        answer(self.authorized(self.http.agent().delete(url)).call(), url)
    }

    fn post(&self, url: &str, content_type: &str, body: &[u8]) -> Result<Answer, String> {
        let request = self
            .authorized(self.http.agent().post(url))
            .header("Content-Type", content_type);
        answer(request.send(body), url)
    }

    fn put(&self, url: &str, body: &[u8]) -> Result<Answer, String> {
        let request = self
            .authorized(self.http.agent().put(url))
            .header("Content-Type", "application/json");
        answer(request.send(body), url)
    }

    /// Whether the token can push to the repository (spec §6.7).
    pub fn can_push(&self) -> Result<bool, String> {
        let (status, body) = self.get(&self.url(""))?;
        match status {
            200 => Ok(json(&body)?["permissions"]["push"].as_bool() == Some(true)),
            401 => Err("GitHub refused the token".to_string()),
            404 => Err(format!(
                "the repository {} does not exist, or the token cannot see it",
                self.repo
            )),
            _ => Err(unexpected(status, &self.repo)),
        }
    }

    /// The file at `path` on the repository's default branch, and its blob
    /// sha; `None` when there is none.
    pub fn read_file(&self, path: &str) -> Result<Option<(String, String)>, String> {
        let (status, body) = self.get(&self.url(&format!("/contents/{}", encode_path(path))))?;
        match status {
            200 => {
                let value = json(&body)?;
                let content: String = value["content"]
                    .as_str()
                    .unwrap_or("")
                    .chars()
                    .filter(|c| !c.is_whitespace())
                    .collect();
                let bytes = base64::engine::general_purpose::STANDARD
                    .decode(content)
                    .map_err(|e| format!("GitHub sent {path} in bad base64: {e}"))?;
                let text = String::from_utf8(bytes)
                    .map_err(|_| format!("{path} in {} is not UTF-8", self.repo))?;
                let sha = value["sha"].as_str().unwrap_or("").to_string();
                Ok(Some((text, sha)))
            }
            404 => Ok(None),
            401 => Err("GitHub refused the token".to_string()),
            _ => Err(unexpected(status, &self.repo)),
        }
    }

    /// Write `text` at `path`, replacing the blob `sha` read with it, or
    /// creating the file when `sha` is `None`.
    pub fn write_file(
        &self,
        path: &str,
        text: &str,
        sha: Option<&str>,
        message: &str,
    ) -> Result<Written, String> {
        let mut body = serde_json::json!({
            "message": message,
            "content": base64::engine::general_purpose::STANDARD.encode(text),
        });
        if let Some(sha) = sha {
            body["sha"] = serde_json::Value::from(sha);
        }
        let url = self.url(&format!("/contents/{}", encode_path(path)));
        let (status, _) = self.put(&url, body.to_string().as_bytes())?;
        match status {
            200 | 201 => Ok(Written::Done),
            409 => Ok(Written::Conflict),
            401 => Err("GitHub refused the token".to_string()),
            _ => Err(unexpected(status, &self.repo)),
        }
    }

    /// The release tagged `tag`, made when there is none.
    pub fn release(&self, tag: &str) -> Result<Release, String> {
        let (status, body) = self.get(&self.url(&format!("/releases/tags/{}", encode(tag))))?;
        match status {
            200 => return parse_release(&body),
            404 => {}
            401 => return Err("GitHub refused the token".to_string()),
            _ => return Err(unexpected(status, &self.repo)),
        }
        let request = serde_json::json!({"tag_name": tag, "name": tag});
        let (status, body) = self.post(
            &self.url("/releases"),
            "application/json",
            request.to_string().as_bytes(),
        )?;
        match status {
            201 => parse_release(&body),
            _ => Err(unexpected(status, &self.repo)),
        }
    }

    pub fn delete_asset(&self, id: u64) -> Result<(), String> {
        let (status, _) = self.delete(&self.url(&format!("/releases/assets/{id}")))?;
        match status {
            204 | 404 => Ok(()),
            _ => Err(unexpected(status, &self.repo)),
        }
    }

    pub fn upload_asset(&self, release: u64, name: &str, bytes: &[u8]) -> Result<(), String> {
        let url = format!(
            "{}/repos/{}/releases/{release}/assets?name={}",
            self.uploads,
            self.repo,
            encode(name)
        );
        let (status, _) = self.post(&url, "application/gzip", bytes)?;
        match status {
            201 => Ok(()),
            _ => Err(unexpected(status, &self.repo)),
        }
    }
}

/// `NOVA_GITHUB_API` may only be a loopback address with a port (spec §8).
fn loopback_only(url: &str) -> Result<(), String> {
    let lower = url.to_ascii_lowercase();
    let port = lower
        .strip_prefix("http://127.0.0.1:")
        .or_else(|| lower.strip_prefix("http://[::1]:"))
        .map(|rest| rest.trim_end_matches('/'));
    match port {
        Some(port) if port.parse::<u16>().is_ok() => Ok(()),
        _ => Err(format!(
            "NOVA_GITHUB_API may only be http://127.0.0.1:<port> or http://[::1]:<port>, not \
             {url}"
        )),
    }
}

/// A response's status and body. An error names the URL, never a header.
fn answer(
    result: Result<ureq::http::Response<ureq::Body>, ureq::Error>,
    url: &str,
) -> Result<Answer, String> {
    let mut response = result.map_err(|e| format!("cannot reach {}: {e}", shown(url)))?;
    let status = response.status().as_u16();
    let body = response
        .body_mut()
        .with_config()
        .limit(MAX_TARBALL)
        .read_to_vec()
        .map_err(|e| format!("cannot read {}: {e}", shown(url)))?;
    Ok((status, body))
}

fn json(body: &[u8]) -> Result<serde_json::Value, String> {
    serde_json::from_slice(body).map_err(|e| format!("GitHub's answer does not parse: {e}"))
}

fn unexpected(status: u16, repo: &str) -> String {
    format!("GitHub answered HTTP {status} for {repo}")
}

fn parse_release(body: &[u8]) -> Result<Release, String> {
    let value = json(body)?;
    let id = value["id"].as_u64().ok_or("GitHub's release has no id")?;
    let assets = value["assets"]
        .as_array()
        .map(|assets| {
            assets
                .iter()
                .filter_map(|a| Some((a["id"].as_u64()?, a["name"].as_str()?.to_string())))
                .collect()
        })
        .unwrap_or_default();
    Ok(Release { id, assets })
}

/// `text` percent-encoded for one path segment or query value.
fn encode(text: &str) -> String {
    text.bytes()
        .map(|b| {
            if b.is_ascii_alphanumeric() || b"-._~".contains(&b) {
                (b as char).to_string()
            } else {
                format!("%{b:02X}")
            }
        })
        .collect()
}

fn encode_path(path: &str) -> String {
    path.split('/').map(encode).collect::<Vec<_>>().join("/")
}

/// `text`, an index file, with `line` appended.
fn appended(text: &str, line: &Line) -> String {
    let mut text = text.to_string();
    if !text.is_empty() && !text.ends_with('\n') {
        text.push('\n');
    }
    text.push_str(&line.to_json());
    text.push('\n');
    text
}

/// Publish `line` and `tarball` to `github` (spec §7):
/// 1. read the package's index file, and refuse a version already there
///    or a name in another case;
/// 2. find or make the release `<name>-<version>`;
/// 3. upload the tarball as its asset, deleting an earlier attempt's;
/// 4. append the line with the file's `sha`, the commit point; if the file
///    changed in between, read it again, check again, and try once more.
pub fn publish_github(github: &GitHub, line: &Line, tarball: &[u8]) -> Result<(), String> {
    let file = index_path(&line.name);
    let tag = format!("{}-{}", line.name, line.vers);
    let asset = format!("{tag}.nova-pkg");
    let (text, sha) = github
        .read_file(&file)?
        .map_or((String::new(), None), |(t, s)| (t, Some(s)));
    check_new(&text, &file, line)?;
    let release = github.release(&tag)?;
    for (id, name) in &release.assets {
        if *name == asset {
            github.delete_asset(*id)?;
        }
    }
    github.upload_asset(release.id, &asset, tarball)?;
    let message = format!("Publish {} {}", line.name, line.vers);
    if github.write_file(&file, &appended(&text, line), sha.as_deref(), &message)? == Written::Done
    {
        return Ok(());
    }
    let (text, sha) = github
        .read_file(&file)?
        .map_or((String::new(), None), |(t, s)| (t, Some(s)));
    check_new(&text, &file, line)?;
    match github.write_file(&file, &appended(&text, line), sha.as_deref(), &message)? {
        Written::Done => Ok(()),
        Written::Conflict => Err(format!(
            "{file} changed twice while {} {} was being published; run `nova publish` again",
            line.name, line.vers
        )),
    }
}

/// An index's files read through the GitHub API, which is never stale, as
/// publishing's verification needs (spec §3.3, §6.6).
pub struct ApiReader<'a> {
    pub github: &'a GitHub,
}

impl Reader for ApiReader<'_> {
    fn file(&mut self, path: &str) -> Result<Option<String>, String> {
        Ok(self.github.read_file(path)?.map(|(text, _)| text))
    }
}
