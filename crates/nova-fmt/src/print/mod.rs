//! The AST, walked into a document (spec
//! `docs/superpowers/specs/2026-10-07-phase-3-1-formatter-design.md` §5.3,
//! §5.4 and §6).
//!
//! The printer reaches every node in source order, so a comment taken when
//! the printer reaches it lands in order. The comment hooks are at the end
//! of this file.

mod expr;
mod item;
mod pattern;
mod ty;

use nova_ast::{Item, Path};
use nova_diagnostics::{Span, Spanned};
use nova_lexer::{Comment, CommentKind, Token};

use crate::comments::Comments;
use crate::doc::{concat, if_break, nest, text, Doc};
use crate::source::Source;

/// Tokens that would continue the statement or match arm before them, since
/// the parser ignores line breaks (spec §5.6).
const CONTINUES: [Token; 7] = [
    Token::LParen,
    Token::LBracket,
    Token::LBrace,
    Token::Minus,
    Token::Star,
    Token::Amp,
    Token::Pipe,
];

/// Print `src`'s AST in the canonical layout (spec §6): `\n` line endings
/// and exactly one final newline, or nothing for an input with nothing in it.
pub(crate) fn print(src: &Source) -> String {
    let mut printer = Printer {
        src,
        comments: Comments::new(&src.comments),
    };
    let doc = printer.file();
    let out = crate::doc::render(doc, crate::WIDTH);
    let body = out.trim_end();
    if body.is_empty() {
        String::new()
    } else {
        format!("{body}\n")
    }
}

/// Walks one input's AST into a document.
pub(crate) struct Printer<'s, 't> {
    src: &'s Source<'t>,
    comments: Comments<'s>,
}

/// How a vertical list spaces its elements (spec §5.3).
#[derive(Clone, Copy, PartialEq, Eq)]
enum Blank {
    /// The author's blank lines, at most one.
    Keep,
    /// Exactly one: between top-level items.
    One,
}

/// Whether a comma list's last element takes a comma (spec §6).
#[derive(Clone, Copy, PartialEq, Eq)]
enum LastComma {
    /// Only when the list is broken.
    Broken,
    /// Always: a one-element tuple, `(a,)`.
    Always,
    /// Never: after a `..` rest or a `..base`.
    Never,
}

/// One element of a comma list, with the comments around it
/// (`Printer::list_entries`).
struct ListEntry {
    start: u32,
    /// Where the element ends in the source: after its comma, if it has one.
    end: u32,
    lead: Doc,
    body: Doc,
    /// The comments that end its line, before its comma.
    trail: Doc,
    /// The comments that end its line, after its comma.
    after_comma: Doc,
}

/// How a comma list is printed (spec §6, "Comma lists").
#[derive(Clone, Copy)]
struct List {
    open: &'static str,
    close: &'static str,
    /// A space inside the delimiters when flat: `{ a, b }`, not `(a, b)`.
    spaced: bool,
    last: LastComma,
    /// Whether the author's blank lines between elements are kept, as
    /// between a record's fields (spec §5.3). One breaks the list.
    blanks: bool,
}

impl List {
    const PARENS: List = List {
        open: "(",
        close: ")",
        spaced: false,
        last: LastComma::Broken,
        blanks: false,
    };
    const BRACKETS: List = List {
        open: "[",
        close: "]",
        spaced: false,
        last: LastComma::Broken,
        blanks: false,
    };
    const ANGLES: List = List {
        open: "<",
        close: ">",
        spaced: false,
        last: LastComma::Broken,
        blanks: false,
    };
    const BRACES: List = List {
        open: "{",
        close: "}",
        spaced: true,
        last: LastComma::Broken,
        blanks: false,
    };
    /// A record's fields.
    const FIELDS: List = List {
        open: "{",
        close: "}",
        spaced: true,
        last: LastComma::Broken,
        blanks: true,
    };
    /// An import's `{a, b}`, which the corpus writes without spaces.
    const IMPORTS: List = List {
        open: "{",
        close: "}",
        spaced: false,
        last: LastComma::Broken,
        blanks: false,
    };

    fn last(self, last: LastComma) -> List {
        List { last, ..self }
    }
}

/// A vertical list being built, one element per line (spec §5.3).
struct Vertical {
    parts: Vec<Doc>,
    /// Where the last element, or the opening delimiter, ends.
    prev_end: u32,
    first: bool,
    blank: Blank,
}

impl Vertical {
    fn new(open: u32, blank: Blank) -> Vertical {
        Vertical {
            parts: Vec::new(),
            prev_end: open,
            first: true,
            blank,
        }
    }
}

impl<'s, 't> Printer<'s, 't> {
    /// The file: top-level items exactly one blank line apart, and each run
    /// of imports sorted, one per line (spec §6).
    fn file(&mut self) -> Doc {
        let src = self.src;
        let items = &src.file.items;
        let mut v = Vertical::new(0, Blank::One);
        let mut i = 0;
        while i < items.len() {
            if matches!(items[i].value, Item::Import(_)) {
                let start = i;
                while i < items.len() && matches!(items[i].value, Item::Import(_)) {
                    i += 1;
                }
                self.import_run(&mut v, &items[start..i]);
            } else {
                let item = &items[i];
                self.vertical_item(&mut v, item.span.start, item.span.end, None, |p| {
                    p.item(&item.value, item.span, true)
                });
                i += 1;
            }
        }
        self.vertical_finish(v, src.text.len() as u32)
    }

    /// A run of imports, sorted by path and printed one per line, with no
    /// blank line inside the run. Each import takes its comments with it,
    /// except comments that a blank line separates from the run's first
    /// import: those belong to the file, and stay above the run (spec §6;
    /// the 3.1 plan's decision 3).
    fn import_run(&mut self, v: &mut Vertical, run: &[Spanned<Item>]) {
        struct Entry {
            key: String,
            lead: Vec<Comment>,
            doc: Doc,
            trail: Doc,
        }
        let mut header = Vec::new();
        let mut entries = Vec::new();
        for (n, item) in run.iter().enumerate() {
            let mut lead = self.take_comments_before(item.span.start);
            if n == 0 {
                let split = (0..lead.len())
                    .rev()
                    .find(|&k| {
                        let next = lead.get(k + 1).map_or(item.span.start, |c| c.span.start);
                        self.src.blank_line_in(lead[k].span.end, next)
                    })
                    .map_or(0, |k| k + 1);
                header = lead.drain(..split).collect();
            }
            let doc = self.item(&item.value, item.span, true);
            let trail = self.trailing(item.span.end);
            entries.push(Entry {
                key: crate::check::import_key(&item.value),
                lead,
                doc,
                trail,
            });
        }
        let first_pos = header
            .first()
            .or(entries[0].lead.first())
            .map_or(run[0].span.start, |c| c.span.start);
        if !v.first {
            v.parts.push(Doc::HardLine);
            if v.blank == Blank::One || self.src.blank_line_in(v.prev_end, first_pos) {
                v.parts.push(Doc::HardLine);
            }
        }
        for (k, c) in header.iter().enumerate() {
            v.parts.push(self.comment_doc(c));
            v.parts.push(Doc::HardLine);
            let next = header
                .get(k + 1)
                .map_or(run[0].span.start, |n| n.span.start);
            if self.src.blank_line_in(c.span.end, next) {
                v.parts.push(Doc::HardLine);
            }
        }
        entries.sort_by(|a, b| a.key.cmp(&b.key));
        for (n, entry) in entries.into_iter().enumerate() {
            if n > 0 {
                v.parts.push(Doc::HardLine);
            }
            for c in &entry.lead {
                v.parts.push(self.comment_doc(c));
                v.parts.push(Doc::HardLine);
            }
            v.parts.push(entry.doc);
            v.parts.push(entry.trail);
        }
        v.prev_end = run[run.len() - 1].span.end;
        v.first = false;
    }

    /// Add one element to `v` (spec §5.3, §5.4): the blank line before it,
    /// its leading comments, its document, `sep` where a separator is needed
    /// (§5.6), and its trailing comments.
    fn vertical_item(
        &mut self,
        v: &mut Vertical,
        start: u32,
        end: u32,
        sep: Option<&'static str>,
        print: impl FnOnce(&mut Self) -> Doc,
    ) {
        let comments = self.take_comments_before(start);
        let first_pos = comments.first().map_or(start, |c| c.span.start);
        if !v.first {
            v.parts.push(Doc::HardLine);
            if v.blank == Blank::One || self.src.blank_line_in(v.prev_end, first_pos) {
                v.parts.push(Doc::HardLine);
            }
        }
        for (k, c) in comments.iter().enumerate() {
            let next = comments.get(k + 1).map_or(start, |n| n.span.start);
            v.parts.push(self.comment_doc(c));
            if self.breaks_after(c, next) {
                v.parts.push(Doc::HardLine);
                if self.src.blank_line_in(c.span.end, next) {
                    v.parts.push(Doc::HardLine);
                }
            } else {
                v.parts.push(text(" "));
            }
        }
        v.parts.push(print(self));
        if let Some(s) = sep {
            v.parts.push(text(s));
        }
        v.parts.push(self.trailing(end));
        v.prev_end = end;
        v.first = false;
    }

    /// Close `v` before `close`: the comments left there, one per line
    /// (spec §5.4).
    fn vertical_finish(&mut self, mut v: Vertical, close: u32) -> Doc {
        for c in self.take_comments_before(close) {
            if !v.first {
                v.parts.push(Doc::HardLine);
                if self.src.blank_line_in(v.prev_end, c.span.start) {
                    v.parts.push(Doc::HardLine);
                }
            }
            v.parts.push(self.comment_doc(&c));
            v.prev_end = c.span.end;
            v.first = false;
        }
        concat(v.parts)
    }

    /// A comma list (spec §6, "Comma lists"): flat as `(a, b)`, or as
    /// `{ a, b }` when `list.spaced`; broken with one element per line, each
    /// followed by a comma, as `list.last` says for the last. `extent` gives
    /// each element's start and end in the source; `close_at` is where the
    /// closing delimiter starts. Not grouped: callers group it, or share its
    /// line breaks with more of their construct.
    fn comma_list<T>(
        &mut self,
        list: List,
        items: &[T],
        extent: impl Fn(&Self, &T) -> (u32, u32),
        print: impl FnMut(&mut Self, &T) -> Doc,
        close_at: u32,
    ) -> Doc {
        let entries = self.list_entries(items, extent, print);
        self.assemble_list(list, entries, close_at)
    }

    /// Each element of a comma list with the comments around it, taken in
    /// source order. A caller that reorders the elements reorders these, so
    /// every comment stays with its element.
    fn list_entries<T>(
        &mut self,
        items: &[T],
        extent: impl Fn(&Self, &T) -> (u32, u32),
        mut print: impl FnMut(&mut Self, &T) -> Doc,
    ) -> Vec<ListEntry> {
        let mut entries = Vec::new();
        for item in items {
            let (start, end) = extent(&*self, item);
            let lead = self.leading(start);
            let body = print(self, item);
            let trail = self.trailing(end);
            let comma_end = self.src.token_end(end, &Token::Comma);
            let after_comma = match comma_end {
                Some(at) => self.trailing(at),
                None => Doc::Nil,
            };
            entries.push(ListEntry {
                start,
                end: comma_end.unwrap_or(end),
                lead,
                body,
                trail,
                after_comma,
            });
        }
        entries
    }

    /// The comma list `comma_list` prints, from its elements in the order
    /// they are to be printed.
    fn assemble_list(&mut self, list: List, entries: Vec<ListEntry>, close_at: u32) -> Doc {
        if entries.is_empty() {
            return match self.dangling(close_at) {
                None => text(format!("{}{}", list.open, list.close)),
                Some(d) => concat(vec![
                    text(list.open),
                    nest(concat(vec![Doc::HardLine, d])),
                    Doc::HardLine,
                    text(list.close),
                ]),
            };
        }
        let edge = if list.spaced {
            Doc::Line
        } else {
            Doc::SoftLine
        };
        let mut inner = vec![edge.clone()];
        let mut prev_end = 0;
        let count = entries.len();
        for (i, e) in entries.into_iter().enumerate() {
            if i > 0 {
                if list.blanks && self.src.blank_line_in(prev_end, e.start) {
                    inner.push(Doc::HardLine);
                }
                inner.push(Doc::Line);
            }
            inner.push(e.lead);
            inner.push(e.body);
            inner.push(e.trail);
            inner.push(if i + 1 < count {
                text(",")
            } else {
                match list.last {
                    LastComma::Broken => if_break(text(","), Doc::Nil),
                    LastComma::Always => text(","),
                    LastComma::Never => Doc::Nil,
                }
            });
            inner.push(e.after_comma);
            prev_end = e.end;
        }
        if let Some(d) = self.dangling(close_at) {
            inner.push(Doc::HardLine);
            inner.push(d);
        }
        concat(vec![
            text(list.open),
            nest(concat(inner)),
            edge,
            text(list.close),
        ])
    }

    /// A name, after any comments before it.
    fn name(&mut self, n: &Spanned<String>) -> Doc {
        concat(vec![self.leading(n.span.start), text(n.value.clone())])
    }

    /// `a::b::c`, after any comments before it.
    fn path(&mut self, path: &Path) -> Doc {
        let lead = match path.segments.first() {
            Some(first) => self.leading(first.span.start),
            None => Doc::Nil,
        };
        let joined: Vec<&str> = path.segments.iter().map(|s| s.value.as_str()).collect();
        concat(vec![lead, text(joined.join("::"))])
    }

    /// `A + B`.
    fn bounds(&mut self, bounds: &[Spanned<Path>]) -> Doc {
        let mut parts = Vec::new();
        for (i, b) in bounds.iter().enumerate() {
            if i > 0 {
                parts.push(text(" + "));
            }
            parts.push(self.path(&b.value));
        }
        concat(parts)
    }

    /// Doc comments: each a `///` line, then a line break (spec §4).
    fn docs(&self, docs: &[Spanned<String>]) -> Doc {
        let mut parts = Vec::new();
        for d in docs {
            parts.push(text(format!("///{}", d.value)));
            parts.push(Doc::HardLine);
        }
        concat(parts)
    }

    /// Source text printed exactly as written: literals, strings, an impl's
    /// trait (spec §5.3). Comments inside it belong to it. Callers take the
    /// comments before it first.
    fn verbatim(&mut self, span: Span) -> Doc {
        self.skip_comments_within(span);
        Doc::Verbatim(self.src.slice(span).to_owned())
    }

    /// A comment's text, without trailing whitespace on any of its lines.
    fn comment_doc(&self, c: &Comment) -> Doc {
        let lines: Vec<&str> = self.src.slice(c.span).lines().map(str::trim_end).collect();
        let t = lines.join("\n");
        if t.contains('\n') {
            Doc::Verbatim(t)
        } else {
            Doc::Text(t)
        }
    }

    /// Whether a line break follows comment `c` in the source before `next`.
    fn breaks_after(&self, c: &Comment, next: u32) -> bool {
        let next = next.max(c.span.end);
        c.kind == CommentKind::Line
            || self.src.text[c.span.end as usize..next as usize].contains('\n')
    }

    // --- Comments (spec §5.4; the 3.1 plan's decision 5). ---

    /// Take every comment not yet printed that starts before `offset`.
    fn take_comments_before(&mut self, offset: u32) -> Vec<Comment> {
        self.comments.take_before(offset).to_vec()
    }

    /// Whether a comment not yet printed starts before `offset`.
    fn has_comment_before(&self, offset: u32) -> bool {
        self.comments.next_starts_before(offset)
    }

    /// Drop the comments inside verbatim text, which prints them itself.
    fn skip_comments_within(&mut self, span: Span) {
        self.comments.take_before(span.end);
    }

    /// The comments before `offset`, for a place inside a line: each is
    /// followed by a line break if it is a line comment or one followed it
    /// in the source, and otherwise by a space.
    fn leading(&mut self, offset: u32) -> Doc {
        let taken = self.comments.take_before(offset);
        let mut parts = Vec::new();
        for (k, c) in taken.iter().enumerate() {
            let next = taken.get(k + 1).map_or(offset, |n| n.span.start);
            parts.push(self.comment_doc(c));
            parts.push(if self.breaks_after(c, next) {
                Doc::HardLine
            } else {
                text(" ")
            });
        }
        concat(parts)
    }

    /// The comments that end `offset`'s line. A line comment waits for the
    /// end of the line the printer is on, after any `,` printed there, and
    /// breaks every group around it. A block comment follows after a
    /// space.
    fn trailing(&mut self, offset: u32) -> Doc {
        let taken = self.comments.take_trailing(self.src.text, offset);
        let mut parts = Vec::new();
        for c in taken {
            match c.kind {
                CommentKind::Line => {
                    let line = self.src.slice(c.span).trim_end();
                    parts.push(Doc::LineSuffix(format!(" {line}")));
                    parts.push(Doc::BreakParent);
                }
                CommentKind::Block => {
                    parts.push(text(" "));
                    parts.push(self.comment_doc(c));
                }
            }
        }
        concat(parts)
    }

    /// The comments before `close`, each on its own line, or `None`.
    fn dangling(&mut self, close: u32) -> Option<Doc> {
        let taken = self.comments.take_before(close);
        if taken.is_empty() {
            return None;
        }
        let mut parts = Vec::new();
        for (k, c) in taken.iter().enumerate() {
            if k > 0 {
                parts.push(Doc::HardLine);
            }
            parts.push(self.comment_doc(c));
        }
        Some(concat(parts))
    }
}
