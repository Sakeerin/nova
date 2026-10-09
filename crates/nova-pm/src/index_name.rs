//! An index's canonical form, and its cache directory's name (spec
//! `docs/superpowers/specs/2026-10-09-phase-3-3b-index-and-publishing-design.md`
//! §3.3 and §5.1).

use std::path::{Path, PathBuf};

use crate::real_path;

/// The canonical form of an index's location: how indexes are compared,
/// and what `nova.lock` and the cache are keyed by.
///
/// - `https://…`, or `http://` on `127.0.0.1` or `[::1]` only: the scheme
///   and host in lower case, no default port, one `/` at the end.
/// - A `file:` URL or an absolute path: the directory's real path, as
///   `file:///` and the path with `/` separators and an upper-case drive
///   letter, with one `/` at the end.
pub fn canonical_index(location: &str) -> Result<String, String> {
    let location = location.trim();
    let lower = location.to_ascii_lowercase();
    if lower.starts_with("https://") || lower.starts_with("http://") {
        return canonical_http(location);
    }
    if lower.starts_with("file:") {
        let path = file_url_path(&location[5..])?;
        return Ok(canonical_local(&path));
    }
    let path = Path::new(location);
    if path.is_absolute() {
        return Ok(canonical_local(path));
    }
    Err(format!(
        "`{location}` is not an index: give an https:// URL, a file: URL or an absolute path"
    ))
}

/// The directory a canonical `file:` index names; `None` for an HTTP one.
pub fn local_index_path(canonical: &str) -> Option<PathBuf> {
    let rest = canonical.strip_prefix("file://")?.trim_end_matches('/');
    Some(PathBuf::from(strip_drive_slash(rest)))
}

/// The cache directory's name for a canonical index (spec §5.1): its host
/// without a port, keeping only `a-z`, `0-9`, `.` and `-` (`local` for a
/// local index), then `-` and the CRC-32 of the canonical form.
pub fn index_dir_name(canonical: &str) -> String {
    let host = match canonical.split_once("://") {
        Some((scheme, rest)) if scheme != "file" => {
            let authority = rest.split('/').next().unwrap_or("");
            split_port(authority)
                .0
                .chars()
                .filter(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == '.' || *c == '-')
                .collect()
        }
        _ => "local".to_string(),
    };
    format!("{host}-{:08x}", crc32fast::hash(canonical.as_bytes()))
}

fn canonical_http(url: &str) -> Result<String, String> {
    let (scheme, rest) = url
        .split_once("://")
        .expect("the caller checked the scheme");
    let scheme = scheme.to_ascii_lowercase();
    if rest.contains(['?', '#']) {
        return Err(format!("`{url}`: an index URL has no query or fragment"));
    }
    let (authority, path) = match rest.find('/') {
        Some(i) => (&rest[..i], &rest[i..]),
        None => (rest, "/"),
    };
    if authority.is_empty() || authority.contains('@') {
        return Err(format!(
            "`{url}`: an index URL needs a host, and no user name"
        ));
    }
    let authority = authority.to_ascii_lowercase();
    let (host, port) = split_port(&authority);
    if scheme == "http" && host != "127.0.0.1" && host != "[::1]" {
        return Err(format!(
            "`{url}`: plain http:// is allowed only for 127.0.0.1 and [::1]; use https://"
        ));
    }
    let default = if scheme == "https" { "443" } else { "80" };
    let authority = match port {
        Some(port) if port != default => format!("{host}:{port}"),
        _ => host.to_string(),
    };
    Ok(format!(
        "{scheme}://{authority}{}/",
        path.trim_end_matches('/')
    ))
}

/// An authority's host and port. An IPv6 host keeps its brackets.
fn split_port(authority: &str) -> (&str, Option<&str>) {
    if let Some(end) = authority.rfind(']') {
        return match authority[end + 1..].strip_prefix(':') {
            Some(port) => (&authority[..=end], Some(port)),
            None => (authority, None),
        };
    }
    match authority.rsplit_once(':') {
        Some((host, port)) => (host, Some(port)),
        None => (authority, None),
    }
}

/// The local path a `file:` URL's remainder names (RFC 8089): `//` with an
/// empty or `localhost` host dropped, percent-escapes decoded, and on
/// Windows the `/` before a drive letter dropped.
fn file_url_path(rest: &str) -> Result<PathBuf, String> {
    let path = match rest.strip_prefix("//") {
        Some(after) => {
            let (host, path) = match after.find('/') {
                Some(i) => (&after[..i], &after[i..]),
                None => (after, ""),
            };
            if !host.is_empty() && !host.eq_ignore_ascii_case("localhost") {
                return Err(format!(
                    "file://{after}: a file: index must be a directory on this machine"
                ));
            }
            path
        }
        None => rest,
    };
    let decoded = percent_decode(path)?;
    let path = strip_drive_slash(&decoded).to_string();
    if !Path::new(&path).is_absolute() {
        return Err(format!("file:{rest} does not name an absolute path"));
    }
    Ok(PathBuf::from(path))
}

/// `/C:/x` as `C:/x`; anything else unchanged.
fn strip_drive_slash(path: &str) -> &str {
    let bytes = path.as_bytes();
    if bytes.len() >= 3 && bytes[0] == b'/' && bytes[1].is_ascii_alphabetic() && bytes[2] == b':' {
        &path[1..]
    } else {
        path
    }
}

fn percent_decode(text: &str) -> Result<String, String> {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let byte = text
                .get(i + 1..i + 3)
                .filter(|hex| hex.bytes().all(|b| b.is_ascii_hexdigit()))
                .and_then(|hex| u8::from_str_radix(hex, 16).ok());
            let Some(byte) = byte else {
                return Err(format!("`{text}` has a bad percent-escape"));
            };
            out.push(byte);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).map_err(|_| format!("`{text}` is not UTF-8 once decoded"))
}

/// The real path of `path`'s longest existing ancestor, with the rest
/// appended. A local index that does not exist, or no longer does, keeps
/// the canonical form it has when it does: Windows' 8.3 names and macOS's
/// `/var` for `/private/var` are resolved either way.
fn real_prefix(path: &Path) -> PathBuf {
    let mut existing = path;
    let mut rest = Vec::new();
    while !existing.exists() {
        match (existing.parent(), existing.file_name()) {
            (Some(parent), Some(name)) => {
                rest.push(name);
                existing = parent;
            }
            _ => break,
        }
    }
    let mut real = real_path(existing);
    for name in rest.iter().rev() {
        real.push(name);
    }
    real
}

/// A local directory's canonical form.
fn canonical_local(path: &Path) -> String {
    let mut text = real_prefix(path).to_string_lossy().replace('\\', "/");
    // An upper-case drive letter, so `c:` and `C:` are one index.
    if text.as_bytes().get(1) == Some(&b':') {
        if let Some(drive) = text.get_mut(..1) {
            drive.make_ascii_uppercase();
        }
    }
    let text = text.trim_end_matches('/');
    if text.starts_with('/') {
        format!("file://{text}/")
    } else {
        format!("file:///{text}/")
    }
}
