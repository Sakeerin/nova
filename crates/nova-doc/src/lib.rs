//! `nova doc`: the documentation site (spec
//! `docs/superpowers/specs/2026-10-11-phase-3-5a-the-documentation-site-design.md`;
//! ADR 0034).

// Until Task 13 writes a site, most of the crate is reached only by its
// tests. Task 13 removes this line.
#![allow(dead_code)]

mod highlight;
mod markdown;

/// `text` with `&`, `<`, `>`, `"` and `'` written as HTML entities.
pub(crate) fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            c => out.push(c),
        }
    }
    out
}

/// `text`, escaped, in a `<span>` of `class`.
pub(crate) fn span(out: &mut String, class: &str, text: &str) {
    out.push_str("<span class=\"");
    out.push_str(class);
    out.push_str("\">");
    out.push_str(&escape(text));
    out.push_str("</span>");
}
