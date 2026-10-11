//! Docs as Markdown (spec §9.1, §9.3; plan decision 12).

use nova_diagnostics::Spanned;
use pulldown_cmark::{html, CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};

use crate::escape;
use crate::highlight::highlight;

/// Docs, rendered: their HTML, and their first paragraph as plain text,
/// which lists and search results show (spec §9.1).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Docs {
    pub html: String,
    pub summary: String,
}

/// Doc lines as the AST keeps them, each the text after its `///`, as one
/// Markdown text: each line loses one leading space.
pub(crate) fn join(lines: &[Spanned<String>]) -> String {
    lines
        .iter()
        .map(|l| l.value.strip_prefix(' ').unwrap_or(&l.value))
        .collect::<Vec<_>>()
        .join("\n")
}

/// `text` rendered (spec §9.1): raw HTML as text, a link or an image to
/// a script as its text alone, headings moved down `level` levels, and
/// Nova code blocks coloured (§9.3).
pub(crate) fn render(text: &str, level: u8) -> Docs {
    if text.trim().is_empty() {
        return Docs::default();
    }
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    let mut events: Vec<Event> = Vec::new();
    // A code block being read: whether it is Nova, and its text so far.
    let mut code: Option<(bool, String)> = None;
    // For each link or image open, whether it is kept (decision 10).
    let mut kept: Vec<bool> = Vec::new();
    for event in Parser::new_ext(text, options) {
        if code.is_some() {
            match event {
                Event::Text(t) => {
                    if let Some((_, buf)) = code.as_mut() {
                        buf.push_str(&t);
                    }
                }
                Event::End(TagEnd::CodeBlock) => {
                    if let Some((nova, buf)) = code.take() {
                        let body = if nova { highlight(&buf) } else { escape(&buf) };
                        events.push(Event::Html(
                            format!("<pre class=\"code\"><code>{body}</code></pre>\n").into(),
                        ));
                    }
                }
                _ => {}
            }
            continue;
        }
        match event {
            Event::Start(Tag::CodeBlock(kind)) => {
                let nova = match kind {
                    CodeBlockKind::Indented => true,
                    CodeBlockKind::Fenced(info) => {
                        let lang = info
                            .split(|c: char| c == ',' || c.is_whitespace())
                            .next()
                            .unwrap_or("");
                        lang.is_empty() || lang == "nova"
                    }
                };
                code = Some((nova, String::new()));
            }
            Event::Html(h) | Event::InlineHtml(h) => events.push(Event::Text(h)),
            // A link or an image to a script is dropped, and its text
            // stays as plain text.
            Event::Start(Tag::Link { ref dest_url, .. } | Tag::Image { ref dest_url, .. }) => {
                let keep = linkable(dest_url);
                kept.push(keep);
                if keep {
                    events.push(event);
                }
            }
            Event::End(TagEnd::Link | TagEnd::Image) => {
                if kept.pop().unwrap_or(true) {
                    events.push(event);
                }
            }
            Event::Start(Tag::Heading {
                level: l,
                id,
                classes,
                attrs,
            }) => events.push(Event::Start(Tag::Heading {
                level: shift(l, level),
                id,
                classes,
                attrs,
            })),
            Event::End(TagEnd::Heading(l)) => {
                events.push(Event::End(TagEnd::Heading(shift(l, level))))
            }
            other => events.push(other),
        }
    }
    let summary = summary(&events);
    let mut html = String::new();
    html::push_html(&mut html, events.into_iter());
    Docs { html, summary }
}

/// Whether `dest` may be a link's or an image's URL: a relative URL, a
/// `#fragment`, or one whose scheme is `http`, `https` or `mailto`. Any
/// other scheme, as `javascript:`, could put a script into the site
/// (spec §9.1, decision 10).
fn linkable(dest: &str) -> bool {
    let dest = dest.trim_start_matches(|c: char| c.is_whitespace() || c.is_ascii_control());
    let Some(end) = dest.find([':', '/', '?', '#']) else {
        return true;
    };
    let scheme = &dest[..end];
    let mut chars = scheme.chars();
    let is_scheme = dest[end..].starts_with(':')
        && chars.next().is_some_and(|c| c.is_ascii_alphabetic())
        && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'));
    !is_scheme
        || ["http", "https", "mailto"]
            .iter()
            .any(|s| scheme.eq_ignore_ascii_case(s))
}

/// `level` moved down `by` levels, stopping at `<h6>` (plan decision 12).
fn shift(level: HeadingLevel, by: u8) -> HeadingLevel {
    let n = by
        + match level {
            HeadingLevel::H1 => 1,
            HeadingLevel::H2 => 2,
            HeadingLevel::H3 => 3,
            HeadingLevel::H4 => 4,
            HeadingLevel::H5 => 5,
            HeadingLevel::H6 => 6,
        };
    match n {
        1 => HeadingLevel::H1,
        2 => HeadingLevel::H2,
        3 => HeadingLevel::H3,
        4 => HeadingLevel::H4,
        5 => HeadingLevel::H5,
        _ => HeadingLevel::H6,
    }
}

/// The first paragraph's text, without its markup.
fn summary(events: &[Event]) -> String {
    let mut out = String::new();
    let mut inside = false;
    for e in events {
        match e {
            Event::Start(Tag::Paragraph) => inside = true,
            Event::End(TagEnd::Paragraph) if inside => break,
            Event::Text(t) | Event::Code(t) if inside => out.push_str(t),
            Event::SoftBreak | Event::HardBreak if inside => out.push(' '),
            _ => {}
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use nova_diagnostics::{FileId, Span};

    #[test]
    fn a_paragraph_emphasis_a_list_and_a_table() {
        let d = render(
            "Some *text*.\n\n- one\n- two\n\n| a | b |\n|---|---|\n| 1 | 2 |",
            3,
        );
        assert_eq!(
            d.html,
            "<p>Some <em>text</em>.</p>\n<ul>\n<li>one</li>\n<li>two</li>\n</ul>\n\
             <table><thead><tr><th>a</th><th>b</th></tr></thead><tbody>\n\
             <tr><td>1</td><td>2</td></tr>\n</tbody></table>\n"
        );
        assert_eq!(d.summary, "Some text.");
    }

    #[test]
    fn raw_html_shows_as_text() {
        // Review Focus 2.
        let d = render("Click <b>here</b>.\n\n<script>alert(1)</script>\n", 3);
        assert!(!d.html.contains("<script>"), "{}", d.html);
        assert!(!d.html.contains("<b>"), "{}", d.html);
        assert!(
            d.html.contains("&lt;script&gt;alert(1)&lt;/script&gt;"),
            "{}",
            d.html
        );
        assert!(
            d.html.contains("Click &lt;b&gt;here&lt;/b&gt;."),
            "{}",
            d.html
        );
    }

    #[test]
    fn a_link_to_a_script_is_its_text_alone() {
        // Decision 10: a dependency's docs cannot put a script into the site.
        for (doc, text) in [
            ("[one](javascript:alert(1))", "one"),
            ("[two](JavaScript:alert(2))", "two"),
            ("<javascript:alert(3)>", "javascript:alert(3)"),
            ("[four](< \tjavascript:alert(4)>)", "four"),
            ("[five](vbscript:msgbox(5))", "five"),
            ("[six](data:text/html,six)", "six"),
            ("[seven][s]\n\n[s]: javascript:alert(7)", "seven"),
        ] {
            let d = render(doc, 3);
            assert!(!d.html.contains("<a"), "{doc}: {}", d.html);
            assert!(d.html.contains(text), "{doc}: {}", d.html);
        }
        let d = render("![picture](javascript:alert(8))", 3);
        assert!(!d.html.contains("<img"), "{}", d.html);
        assert_eq!(d.html, "<p>picture</p>\n");
        let d = render("[![badge](javascript:alert(9))](ok.html)", 3);
        assert_eq!(d.html, "<p><a href=\"ok.html\">badge</a></p>\n");
    }

    #[test]
    fn web_mail_relative_and_fragment_links_stay_links() {
        let d = render(
            "[a](https://example.com/) [b](HTTP://example.com/) \
             [c](mailto:me@example.com) [d](../geom/index.html#fn.area) \
             [e](#fn.main) [f](dir/a:b.html) ![g](pic.png)",
            3,
        );
        for href in [
            "<a href=\"https://example.com/\">a</a>",
            "<a href=\"HTTP://example.com/\">b</a>",
            "<a href=\"mailto:me@example.com\">c</a>",
            "<a href=\"../geom/index.html#fn.area\">d</a>",
            "<a href=\"#fn.main\">e</a>",
            "<a href=\"dir/a:b.html\">f</a>",
            "<img src=\"pic.png\" alt=\"g\" />",
        ] {
            assert!(d.html.contains(href), "{href}: {}", d.html);
        }
    }

    #[test]
    fn headings_move_down_below_their_items() {
        let d = render("# Errors\n\n###### Deep", 3);
        assert_eq!(d.html, "<h4>Errors</h4>\n<h6>Deep</h6>\n");
    }

    #[test]
    fn nova_code_is_highlighted_and_other_code_is_not() {
        let d = render(
            "```\nfn f() {}\n```\n\n```text\nfn f() {}\n```\n\n    let x = 1\n",
            3,
        );
        assert_eq!(
            d.html,
            "<pre class=\"code\"><code><span class=\"kw\">fn</span> f() {}\n</code></pre>\n\
             <pre class=\"code\"><code>fn f() {}\n</code></pre>\n\
             <pre class=\"code\"><code><span class=\"kw\">let</span> x = \
             <span class=\"lit\">1</span>\n</code></pre>\n"
        );
    }

    #[test]
    fn the_summary_is_the_first_paragraph_as_plain_text() {
        let d = render("Reads `n` bytes,\nor fewer.\n\nThen more.", 3);
        assert_eq!(d.summary, "Reads n bytes, or fewer.");
    }

    #[test]
    fn empty_docs_render_nothing() {
        assert_eq!(render("", 3), Docs::default());
        assert_eq!(render("  \n", 3), Docs::default());
    }

    #[test]
    fn lines_join_without_one_leading_space() {
        let line = |s: &str| Spanned::new(s.to_string(), Span::point(0, FileId::DUMMY));
        assert_eq!(
            join(&[line(" One."), line(""), line("  Two.")]),
            "One.\n\n Two."
        );
    }
}
