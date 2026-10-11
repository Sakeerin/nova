//! `nova doc`: the documentation site (spec
//! `docs/superpowers/specs/2026-10-11-phase-3-5a-the-documentation-site-design.md`;
//! ADR 0034).

mod highlight;
mod links;
mod markdown;
mod model;
mod render;
mod search;
mod signature;
mod write;

pub use links::{check_links, BrokenLink};
pub use markdown::{classify, code_blocks, hidden, hide_lines, Block, CodeBlock, CodeLine};
pub use model::{document_package, document_std, Documented, Failure, Site};
pub use write::{write_site, WriteError, MARKER};

/// The site's stylesheet: its light and dark themes (spec §10.2).
pub(crate) const CSS: &str = include_str!("../assets/nova.css");

/// The site's script: search and the theme toggle (spec §10).
pub(crate) const JS: &str = include_str!("../assets/nova.js");

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
