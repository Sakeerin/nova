//! Docs as Markdown (spec §9.1, §9.3; plan decision 12), and the one rule
//! for code blocks and hidden lines that the site, doc tests and hover
//! share (3.5b spec §4.2, §4.3).

use nova_diagnostics::Spanned;
use pulldown_cmark::{html, CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};

use crate::escape;
use crate::highlight::highlight;

/// The options the site renders with, and doc tests and hover read with,
/// so all three find the same code blocks: a table can end a block.
fn options() -> Options {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options
}

/// What a code block is (3.5b spec §4.2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Block {
    /// Nova, highlighted and run as a doc test. `unknown` holds the words
    /// after `nova` that mean nothing, which `nova test` warns about.
    Test {
        /// The unknown words, in order.
        unknown: Vec<String>,
    },
    /// Nova, highlighted, not run: its first word is `ignore`, or a word
    /// after `nova` is.
    Ignored,
    /// Another language, shown as written.
    Other,
}

impl Block {
    /// Whether it is Nova: highlighted, its hidden lines left out.
    pub fn is_nova(&self) -> bool {
        !matches!(self, Block::Other)
    }
}

/// The block whose info string is `info`, or an indented block for `None`
/// (3.5b spec §4.2). The info string is split at commas and whitespace
/// into words, and empty words are dropped.
pub fn classify(info: Option<&str>) -> Block {
    let Some(info) = info else {
        return Block::Test {
            unknown: Vec::new(),
        };
    };
    let mut words = info
        .split(|c: char| c == ',' || c.is_whitespace())
        .filter(|w| !w.is_empty());
    match words.next() {
        None => Block::Test {
            unknown: Vec::new(),
        },
        Some("ignore") => Block::Ignored,
        Some("nova") => {
            let rest: Vec<&str> = words.collect();
            if rest.contains(&"ignore") {
                Block::Ignored
            } else {
                Block::Test {
                    unknown: rest.iter().map(|w| w.to_string()).collect(),
                }
            }
        }
        Some(_) => Block::Other,
    }
}

/// For a hidden line of a Nova block (3.5b spec §4.3), where its `# `, or
/// its lone `#`, starts and how many bytes it is; `None` for a line that
/// shows. It runs less that mark.
pub fn hidden(line: &str) -> Option<(usize, usize)> {
    let rest = line.trim_start_matches([' ', '\t']);
    let at = line.len() - rest.len();
    if rest.starts_with("# ") {
        Some((at, 2))
    } else if rest == "#" {
        Some((at, 1))
    } else {
        None
    }
}

/// A line of a code block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeLine {
    /// The line as the block holds it, without its line break.
    pub text: String,
    /// How many bytes at its start the parser made up: the rest of a tab
    /// that a container or the block's indentation took part of.
    pub pad: usize,
    /// Where `text[pad..]` starts in the Markdown.
    pub at: usize,
}

/// A code block, and where its lines are in the Markdown.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeBlock {
    /// What it is.
    pub block: Block,
    /// Where it starts in the Markdown: its opening fence, or an indented
    /// block's first line.
    pub start: usize,
    /// Its lines, in order.
    pub lines: Vec<CodeLine>,
}

/// Every code block of the Markdown `text`, as the site finds them.
pub fn code_blocks(text: &str) -> Vec<CodeBlock> {
    let mut out: Vec<CodeBlock> = Vec::new();
    let mut open: Option<CodeBlock> = None;
    let mut line: Option<CodeLine> = None;
    for (event, range) in Parser::new_ext(text, options()).into_offset_iter() {
        match event {
            Event::Start(Tag::CodeBlock(kind)) => {
                let block = match &kind {
                    CodeBlockKind::Indented => classify(None),
                    CodeBlockKind::Fenced(info) => classify(Some(info)),
                };
                open = Some(CodeBlock {
                    block,
                    start: range.start,
                    lines: Vec::new(),
                });
            }
            Event::Text(t) => {
                let Some(b) = open.as_mut() else {
                    continue;
                };
                if range.is_empty() {
                    // Spaces the parser made up for a tab's remainder.
                    let l = line.get_or_insert_with(|| CodeLine {
                        text: String::new(),
                        pad: 0,
                        at: range.start,
                    });
                    l.text.push_str(&t);
                    l.pad += t.len();
                    l.at = range.start;
                    continue;
                }
                // `t` is `text[range]`: perhaps several lines, perhaps
                // starting with the line break of a CRLF line.
                let mut at = range.start;
                for piece in t.split_inclusive('\n') {
                    let (body, ends) = match piece.strip_suffix('\n') {
                        Some(body) => (body, true),
                        None => (piece, false),
                    };
                    let l = line.get_or_insert_with(|| CodeLine {
                        text: String::new(),
                        pad: 0,
                        at,
                    });
                    if l.text.len() == l.pad {
                        l.at = at;
                    }
                    l.text.push_str(body);
                    if ends {
                        b.lines.extend(line.take());
                    }
                    at += piece.len();
                }
            }
            Event::End(TagEnd::CodeBlock) => {
                if let Some(mut b) = open.take() {
                    // An unclosed fence's last line has no line break.
                    b.lines.extend(line.take());
                    out.push(b);
                }
            }
            _ => {}
        }
    }
    out
}

/// `markdown` with the hidden lines of its Nova blocks left out (3.5b spec
/// §8.2), every other byte as it was. Hover shows docs through it.
pub fn hide_lines(markdown: &str) -> String {
    if !markdown.contains('#') {
        return markdown.to_string();
    }
    let mut out = String::with_capacity(markdown.len());
    let mut from = 0;
    for b in code_blocks(markdown).iter().filter(|b| b.block.is_nova()) {
        for l in b.lines.iter().filter(|l| hidden(&l.text).is_some()) {
            // The whole Markdown line, with its `> ` or list indentation
            // and its line break.
            let start = markdown[..l.at].rfind('\n').map_or(0, |i| i + 1);
            let end = markdown[l.at..]
                .find('\n')
                .map_or(markdown.len(), |i| l.at + i + 1);
            out.push_str(&markdown[from..start]);
            from = end;
        }
    }
    out.push_str(&markdown[from..]);
    out
}

/// A Nova block's code as the page shows it, its hidden lines left out
/// (3.5b spec §8.1).
fn shown(code: &str) -> String {
    code.split_inclusive('\n')
        .filter(|l| hidden(l.strip_suffix('\n').unwrap_or(l)).is_none())
        .collect()
}

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
/// Nova code blocks coloured (§9.3), `ignore` blocks among them, without
/// their hidden lines (3.5b spec §8.1).
pub(crate) fn render(text: &str, level: u8) -> Docs {
    if text.trim().is_empty() {
        return Docs::default();
    }
    let mut events: Vec<Event> = Vec::new();
    // A code block being read: whether it is Nova, and its text so far.
    let mut code: Option<(bool, String)> = None;
    // For each link or image open, whether it is kept (decision 10).
    let mut kept: Vec<bool> = Vec::new();
    for event in Parser::new_ext(text, options()) {
        if code.is_some() {
            match event {
                Event::Text(t) => {
                    if let Some((_, buf)) = code.as_mut() {
                        buf.push_str(&t);
                    }
                }
                Event::End(TagEnd::CodeBlock) => {
                    if let Some((nova, buf)) = code.take() {
                        let body = if nova {
                            highlight(&shown(&buf))
                        } else {
                            escape(&buf)
                        };
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
                let block = match &kind {
                    CodeBlockKind::Indented => classify(None),
                    CodeBlockKind::Fenced(info) => classify(Some(info)),
                };
                code = Some((block.is_nova(), String::new()));
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
    fn blocks_are_classified_by_their_first_word() {
        // 3.5b spec §4.2's table, and an unknown word.
        let test = |unknown: &[&str]| Block::Test {
            unknown: unknown.iter().map(|w| w.to_string()).collect(),
        };
        for (info, block) in [
            (None, test(&[])),
            (Some(""), test(&[])),
            (Some("nova"), test(&[])),
            (Some("nova,ignore"), Block::Ignored),
            (Some("nova ignore"), Block::Ignored),
            (Some("nova, ignore"), Block::Ignored),
            (Some("ignore"), Block::Ignored),
            (Some("text"), Block::Other),
            (Some("sh"), Block::Other),
            (Some(",text"), Block::Other),
            (Some("nova,should_panic"), test(&["should_panic"])),
            (
                Some("nova,,no_run  compile_fail"),
                test(&["no_run", "compile_fail"]),
            ),
        ] {
            assert_eq!(classify(info), block, "{info:?}");
        }
        assert!(test(&[]).is_nova());
        assert!(Block::Ignored.is_nova());
        assert!(!Block::Other.is_nova());
    }

    #[test]
    fn a_hidden_line_starts_with_a_hash_and_a_space_or_is_a_lone_hash() {
        assert_eq!(hidden("# let a = 1"), Some((0, 2)));
        assert_eq!(hidden("    # x"), Some((4, 2)));
        assert_eq!(hidden("\t# x"), Some((1, 2)));
        assert_eq!(hidden("#"), Some((0, 1)));
        assert_eq!(hidden("  #"), Some((2, 1)));
        assert_eq!(hidden("#x"), None);
        assert_eq!(hidden("// # x"), None);
        assert_eq!(hidden("let a = 1 # x"), None);
        assert_eq!(hidden(""), None);
    }

    #[test]
    fn hidden_lines_are_left_out_of_nova_code_on_the_page() {
        // Mutant 1: a `# ` line of a Nova block is not shown; one of a
        // `text` block is.
        let d = render(
            "```\n# let a = 1\n#\nlet b = a\n```\n\n```text\n# shown\n```\n\n    \
             # let c = 2\n    let d = c\n",
            3,
        );
        assert_eq!(
            d.html,
            "<pre class=\"code\"><code><span class=\"kw\">let</span> b = a\n</code></pre>\n\
             <pre class=\"code\"><code># shown\n</code></pre>\n\
             <pre class=\"code\"><code><span class=\"kw\">let</span> d = c\n</code></pre>\n"
        );
    }

    #[test]
    fn an_ignored_block_is_highlighted_as_nova() {
        let d = render("```ignore\n# let a = 1\nfn f() {}\n```", 3);
        assert_eq!(
            d.html,
            "<pre class=\"code\"><code><span class=\"kw\">fn</span> f() {}\n</code></pre>\n"
        );
    }

    #[test]
    fn code_blocks_give_each_line_and_where_it_starts() {
        let text = "Use:\n\n```nova,ignore\nlet a = 1\n# let b = a\n```\n\n- In a list:\n\n      \
                    let c = 3\n\n> ```\n> let d = 4\n> ```\n\n```text\nunclosed\nlast";
        let at = |s: &str| text.find(s).unwrap();
        let line = |s: &str| CodeLine {
            text: s.to_string(),
            pad: 0,
            at: at(s),
        };
        let test = Block::Test {
            unknown: Vec::new(),
        };
        assert_eq!(
            code_blocks(text),
            [
                CodeBlock {
                    block: Block::Ignored,
                    start: at("```nova"),
                    lines: vec![line("let a = 1"), line("# let b = a")],
                },
                CodeBlock {
                    block: test.clone(),
                    start: at("let c"),
                    lines: vec![line("let c = 3")],
                },
                CodeBlock {
                    block: test.clone(),
                    start: at("```\n> let d"),
                    lines: vec![line("let d = 4")],
                },
                CodeBlock {
                    block: Block::Other,
                    start: at("```text"),
                    lines: vec![line("unclosed"), line("last")],
                },
            ]
        );
        // A tab a list item took part of: two spaces made up before `code`.
        assert_eq!(
            code_blocks("- a\n\n \t\tcode"),
            [CodeBlock {
                block: test,
                start: 8,
                lines: vec![CodeLine {
                    text: "  code".to_string(),
                    pad: 2,
                    at: 8,
                }],
            }]
        );
    }

    #[test]
    fn hide_lines_cuts_only_the_hidden_lines_of_nova_blocks() {
        let md = "Use it:\n\n```\n# let x = 1\nlet y = x\n```\n\n```text\n# shown\n```\n\n\
                  > ```nova\n> # q\n> let z = 1\n> ```\n\n# Heading\n\n    # let w = 0\n    \
                  let v = w\n\n```ignore\n#\nlet u = 1\n```\n";
        assert_eq!(
            hide_lines(md),
            "Use it:\n\n```\nlet y = x\n```\n\n```text\n# shown\n```\n\n\
             > ```nova\n> let z = 1\n> ```\n\n# Heading\n\n    let v = w\n\n\
             ```ignore\nlet u = 1\n```\n"
        );
        assert_eq!(hide_lines("No code # here."), "No code # here.");
        assert_eq!(hide_lines("# Title\n\nText."), "# Title\n\nText.");
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
