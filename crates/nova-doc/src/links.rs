//! The link check (spec §11).

use std::collections::{HashMap, HashSet};
use std::path::Path;

/// A link that leads nowhere: the page it is on, from the site's root,
/// and the link as written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrokenLink {
    /// The page, from the site's root, with `/`.
    pub page: String,
    /// The link, as its attribute holds it, unescaped.
    pub link: String,
}

/// Every link in the site at `dir` that is not absolute and leads to no
/// file of the site, or to no `id` on its page (spec §11). Pages in order
/// of their paths, and each page's links in order.
pub fn check_links(dir: &Path) -> Vec<BrokenLink> {
    let mut pages = Vec::new();
    walk(dir, "", &mut pages);
    pages.sort();
    let texts: HashMap<&str, String> = pages
        .iter()
        .map(|p| {
            (
                p.as_str(),
                std::fs::read_to_string(dir.join(p)).unwrap_or_default(),
            )
        })
        .collect();
    let ids: HashMap<&str, HashSet<String>> = texts
        .iter()
        .map(|(p, t)| (*p, attributes(t, "id").into_iter().collect()))
        .collect();
    let mut broken = Vec::new();
    for page in &pages {
        let text = &texts[page.as_str()];
        for link in attributes(text, "href")
            .into_iter()
            .chain(attributes(text, "src"))
        {
            if !resolves(dir, page, &link, &ids) {
                broken.push(BrokenLink {
                    page: page.clone(),
                    link,
                });
            }
        }
    }
    broken
}

/// Each `.html` file under `dir`, by its path from the site's root.
fn walk(dir: &Path, prefix: &str, out: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let rel = if prefix.is_empty() {
            name.clone()
        } else {
            format!("{prefix}/{name}")
        };
        let path = entry.path();
        if path.is_dir() {
            walk(&path, &rel, out);
        } else if name.ends_with(".html") {
            out.push(rel);
        }
    }
}

/// The values of every `name="…"` attribute in `html`'s tags, unescaped.
/// Only a tag, from a `<` to the next `>`, is read: a page's text can show
/// `href="…"`, but never a raw `<`, which is escaped.
fn attributes(html: &str, name: &str) -> Vec<String> {
    let pattern = format!(" {name}=\"");
    let mut out = Vec::new();
    let mut rest = html;
    while let Some(open) = rest.find('<') {
        let tag = &rest[open..];
        let close = tag.find('>').unwrap_or(tag.len());
        let mut inside = &tag[..close];
        while let Some(at) = inside.find(&pattern) {
            let value = &inside[at + pattern.len()..];
            let end = value.find('"').unwrap_or(value.len());
            out.push(unescape(&value[..end]));
            inside = &value[end..];
        }
        rest = &tag[close..];
    }
    out
}

/// Whether `link`, on `page`, leads to a file of the site, and to an `id`
/// on it when it names one.
fn resolves(dir: &Path, page: &str, link: &str, ids: &HashMap<&str, HashSet<String>>) -> bool {
    if link.starts_with("//") || has_scheme(link) {
        return true;
    }
    let (path, fragment) = link.split_once('#').unwrap_or((link, ""));
    let path = path.split('?').next().unwrap_or("");
    let target = if path.is_empty() {
        page.to_string()
    } else {
        match join(page, &decode(path)) {
            Some(t) => t,
            None => return false,
        }
    };
    match ids.get(target.as_str()) {
        Some(on_page) => fragment.is_empty() || on_page.contains(&decode(fragment)),
        None => fragment.is_empty() && dir.join(&target).is_file(),
    }
}

/// `link`, relative to `page`'s directory, as a path from the site's root;
/// `None` when it leaves the site or starts at a root.
fn join(page: &str, link: &str) -> Option<String> {
    if link.starts_with('/') {
        return None;
    }
    let mut parts: Vec<&str> = page.split('/').collect();
    parts.pop();
    for part in link.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop()?;
            }
            p => parts.push(p),
        }
    }
    Some(parts.join("/"))
}

/// Whether `link` starts with a URL scheme, such as `https:` or `mailto:`.
fn has_scheme(link: &str) -> bool {
    let Some(colon) = link.find(':') else {
        return false;
    };
    let scheme = &link[..colon];
    scheme.starts_with(|c: char| c.is_ascii_alphabetic())
        && scheme
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'))
}

/// `text` with its `%XX` escapes decoded.
fn decode(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let hex = bytes
            .get(i + 1..i + 3)
            .and_then(|h| std::str::from_utf8(h).ok())
            .and_then(|h| u8::from_str_radix(h, 16).ok());
        match (bytes[i], hex) {
            (b'%', Some(b)) => {
                out.push(b);
                i += 3;
            }
            (b, _) => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// An attribute's value with its entities decoded.
fn unescape(text: &str) -> String {
    text.replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// A fresh site of `files` in a directory with a fixed name.
    fn site(name: &str, files: &[(&str, &str)]) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("nova-doc-links-{name}"));
        let _ = std::fs::remove_dir_all(&dir);
        for (path, text) in files {
            let path = dir.join(path);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, text).unwrap();
        }
        dir
    }

    fn broken(page: &str, link: &str) -> BrokenLink {
        BrokenLink {
            page: page.to_string(),
            link: link.to_string(),
        }
    }

    #[test]
    fn a_missing_file_and_a_missing_fragment_are_broken() {
        let dir = site(
            "broken",
            &[
                (
                    "index.html",
                    "<a href=\"a/index.html\">a</a> <a href=\"a/index.html#here\">h</a> \
                     <a href=\"a/index.html#gone\">g</a> <a href=\"nope.html\">n</a> \
                     <script src=\"x.js\"></script>",
                ),
                (
                    "a/index.html",
                    "<h3 id=\"here\">Here</h3> <a href=\"#here\">s</a> <a href=\"#missing\">m</a> \
                     <a href=\"../index.html\">u</a> <a href=\"../b/index.html#there\">b</a> \
                     <a href=\"../../out.html\">o</a>",
                ),
                ("b/index.html", "<p id=\"there\">There</p>"),
                ("x.js", ""),
            ],
        );
        assert_eq!(
            check_links(&dir),
            vec![
                broken("a/index.html", "#missing"),
                broken("a/index.html", "../../out.html"),
                broken("index.html", "a/index.html#gone"),
                broken("index.html", "nope.html"),
            ]
        );
    }

    #[test]
    fn absolute_urls_are_not_checked() {
        let dir = site(
            "absolute",
            &[(
                "index.html",
                "<a href=\"https://example.com/x\">x</a> <a href=\"mailto:a@b.c\">m</a> \
                 <a href=\"//cdn.example/x.js\">c</a>",
            )],
        );
        assert_eq!(check_links(&dir), Vec::new());
    }

    #[test]
    fn a_link_written_in_text_is_not_checked() {
        // pulldown-cmark does not escape `"` in text or inline code, so a
        // doc can show `href="…"` outside any tag.
        let dir = site(
            "text-link",
            &[(
                "index.html",
                "<p>Write <code>&lt;a href=\"gone.html\"&gt;</code> or src=\"gone.js\".</p>",
            )],
        );
        assert_eq!(check_links(&dir), Vec::new());
    }

    #[test]
    fn an_id_written_in_text_is_no_target() {
        let dir = site(
            "text-id",
            &[(
                "index.html",
                "<p>Not a target: id=\"fn.fake\".</p> <a href=\"#fn.fake\">f</a>",
            )],
        );
        assert_eq!(check_links(&dir), vec![broken("index.html", "#fn.fake")]);
    }

    #[test]
    fn an_escaped_or_encoded_link_is_read_as_its_target() {
        let dir = site(
            "encoded",
            &[
                ("index.html", "<a href=\"a%20b.html#x&amp;y\">e</a>"),
                ("a b.html", "<p id=\"x&amp;y\">p</p>"),
            ],
        );
        assert_eq!(check_links(&dir), Vec::new());
    }
}
