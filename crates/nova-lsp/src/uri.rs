//! `file:` URIs and paths (spec
//! `docs/superpowers/specs/2026-10-08-phase-3-2-lsp-core-design.md` §6.9).

use std::path::{Path, PathBuf};

/// The path a `file:` URI names, or `None` for any other URI. Percent
/// encoding is decoded, so `file:///d%3A/x/y.nova` is `d:\x\y.nova` on
/// Windows. A URI with an authority (`file://server/share`) is not a local
/// file.
pub fn to_path(uri: &str) -> Option<PathBuf> {
    let rest = uri.strip_prefix("file://")?;
    if !rest.starts_with('/') {
        return None;
    }
    let decoded = percent_decode(rest);
    if cfg!(windows) {
        let b = decoded.as_bytes();
        if b.len() >= 3 && b[1].is_ascii_alphabetic() && b[2] == b':' {
            return Some(PathBuf::from(decoded[1..].replace('/', "\\")));
        }
        return None;
    }
    Some(PathBuf::from(decoded))
}

/// A `file:` URI for the absolute `path`, with every byte outside
/// `A-Za-z0-9-._~/:` percent-encoded.
pub fn from_path(path: &Path) -> String {
    let mut s = path.to_string_lossy().replace('\\', "/");
    if !s.starts_with('/') {
        s.insert(0, '/');
    }
    let mut out = String::from("file://");
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b"-._~/:".contains(&b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// The path an untitled buffer is checked under. It exists only in the
/// overlay (decision 7).
pub fn untitled_path(uri: &str) -> Option<PathBuf> {
    let name = uri.strip_prefix("untitled:")?;
    let safe: String = name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();
    Some(
        std::env::temp_dir()
            .join("nova-untitled")
            .join(format!("{safe}.nova")),
    )
}

/// The path a document is checked under: its file's, or an untitled
/// buffer's.
pub fn document_path(uri: &str) -> Option<PathBuf> {
    to_path(uri).or_else(|| untitled_path(uri))
}

/// An `lsp_types::Uri` as text. This and [`parse`] are the only two places
/// that touch `lsp_types::Uri`'s own API.
pub fn text(uri: &lsp_types::Uri) -> String {
    uri.as_str().to_owned()
}

/// Text as an `lsp_types::Uri`, or `None` if it does not parse.
pub fn parse(s: &str) -> Option<lsp_types::Uri> {
    s.parse().ok()
}

fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let hex = |c: u8| (c as char).to_digit(16).map(|d| d as u8);
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            if let (Some(h), Some(l)) = (hex(b[i + 1]), hex(b[i + 2])) {
                out.push(h * 16 + l);
                i += 3;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(windows)]
    #[test]
    fn a_windows_uri_with_an_encoded_drive_letter_is_its_path() {
        assert_eq!(
            to_path("file:///d%3A/x/y.nova"),
            Some(PathBuf::from(r"d:\x\y.nova"))
        );
        assert_eq!(
            to_path("file:///D:/x/y.nova"),
            Some(PathBuf::from(r"D:\x\y.nova"))
        );
    }

    #[test]
    fn uris_with_spaces_and_thai_round_trip() {
        let path = if cfg!(windows) {
            PathBuf::from(r"C:\Users\a b\โปรเจกต์\main.nova")
        } else {
            PathBuf::from("/home/a b/โปรเจกต์/main.nova")
        };
        let uri = from_path(&path);
        assert!(uri.starts_with("file:///"), "{uri}");
        assert!(!uri.contains(' '), "{uri}");
        assert_eq!(to_path(&uri), Some(path));
    }

    #[test]
    fn other_uris_are_not_files() {
        assert_eq!(to_path("untitled:Untitled-1"), None);
        assert_eq!(to_path("file://server/share/x.nova"), None);
        let untitled = untitled_path("untitled:Untitled-1").unwrap();
        assert!(untitled.ends_with("Untitled-1.nova"), "{untitled:?}");
        assert_eq!(document_path("untitled:Untitled-1"), Some(untitled));
    }
}
