//! HTTP, for reading an index and downloading tarballs (spec §3.3, §5.2,
//! §8). Redirects are followed here rather than by ureq, so each target is
//! checked, and no request made here carries a token.

use std::time::Duration;

/// Redirects followed before giving up.
const MAX_REDIRECTS: usize = 5;

/// The largest index file read: 10 MiB.
pub const MAX_INDEX_FILE: u64 = 10 * 1024 * 1024;

/// An HTTP client for reads that need no credentials.
pub struct Http {
    agent: ureq::Agent,
}

impl Default for Http {
    fn default() -> Self {
        Http::new()
    }
}

impl Http {
    /// A status is a value, not an error; ureq follows no redirect; a
    /// request takes at most 60 seconds; the User-Agent names nova and its
    /// version only. ureq's default configuration reads the proxy from
    /// `HTTPS_PROXY`, `HTTP_PROXY` and `NO_PROXY` (spec §9).
    pub fn new() -> Http {
        let config = ureq::Agent::config_builder()
            .http_status_as_error(false)
            .max_redirects(0)
            .max_redirects_will_error(false)
            .timeout_global(Some(Duration::from_secs(60)))
            .user_agent(format!("nova/{}", env!("CARGO_PKG_VERSION")))
            .build();
        Http {
            agent: ureq::Agent::new_with_config(config),
        }
    }

    /// GET `url`, reading at most `limit` bytes of its body. `Ok(None)` for
    /// a 404. Up to five redirects are followed, each checked by
    /// [`check_url`].
    pub fn get(&self, url: &str, limit: u64) -> Result<Option<Vec<u8>>, String> {
        let mut url = url.to_string();
        for _ in 0..=MAX_REDIRECTS {
            check_url(&url)?;
            let mut response = self
                .agent
                .get(&url)
                .call()
                .map_err(|e| format!("cannot reach {}: {e}", shown(&url)))?;
            let status = response.status().as_u16();
            if matches!(status, 301 | 302 | 303 | 307 | 308) {
                let location = response
                    .headers()
                    .get("location")
                    .and_then(|value| value.to_str().ok())
                    .ok_or_else(|| format!("{} redirected nowhere", shown(&url)))?
                    .to_string();
                url = resolve_location(&url, &location);
                continue;
            }
            return match status {
                200 => response
                    .body_mut()
                    .with_config()
                    .limit(limit)
                    .read_to_vec()
                    .map(Some)
                    .map_err(|e| format!("cannot read {}: {e}", shown(&url))),
                404 => Ok(None),
                _ => Err(format!("{} answered HTTP {status}", shown(&url))),
            };
        }
        Err(format!("too many redirects, the last to {}", shown(&url)))
    }
}

/// Whether nova may fetch `url` (spec §3.3): `https://`, or `http://` on
/// 127.0.0.1 or [::1] only.
pub fn check_url(url: &str) -> Result<(), String> {
    let lower = url.to_ascii_lowercase();
    if lower.starts_with("https://") {
        return Ok(());
    }
    if let Some(rest) = lower.strip_prefix("http://") {
        let authority = rest.split(['/', '?', '#']).next().unwrap_or("");
        let host = match authority.rfind(']') {
            Some(end) => &authority[..=end],
            None => authority.split(':').next().unwrap_or(""),
        };
        if host == "127.0.0.1" || host == "[::1]" {
            return Ok(());
        }
    }
    Err(format!(
        "refused {}: only https://, or http:// on 127.0.0.1 or [::1], is allowed",
        shown(url)
    ))
}

/// `url` without its query, for messages: a signed download URL's query
/// is long, and says nothing a reader needs.
pub(crate) fn shown(url: &str) -> &str {
    url.split('?').next().unwrap_or(url)
}

/// A redirect's `Location`, against the URL that gave it.
fn resolve_location(base: &str, location: &str) -> String {
    let lower = location.to_ascii_lowercase();
    if lower.starts_with("http://") || lower.starts_with("https://") {
        return location.to_string();
    }
    let (scheme, rest) = base.split_once("://").unwrap_or(("https", base));
    if let Some(authority_and_path) = location.strip_prefix("//") {
        return format!("{scheme}://{authority_and_path}");
    }
    let authority = rest.split('/').next().unwrap_or("");
    if location.starts_with('/') {
        return format!("{scheme}://{authority}{location}");
    }
    let path = &rest[authority.len()..];
    let dir = &path[..path.rfind('/').map_or(0, |i| i + 1)];
    let dir = if dir.is_empty() { "/" } else { dir };
    format!("{scheme}://{authority}{dir}{location}")
}
