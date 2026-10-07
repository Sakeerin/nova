//! Wadler's pretty printer (spec
//! `docs/superpowers/specs/2026-10-07-phase-3-1-formatter-design.md` §5.2).
//!
//! A [`Doc::Group`] prints flat, with every [`Doc::Line`] a space and every
//! [`Doc::SoftLine`] nothing, when it fits in the rest of the line, and
//! broken, each of them a line break, when it does not. A group holding a
//! [`Doc::HardLine`] or [`Doc::BreakParent`] is always broken.

/// Spaces per indentation level (spec §6).
pub(crate) const INDENT: usize = 4;

/// A document.
#[derive(Clone, Debug)]
pub(crate) enum Doc {
    Nil,
    /// Text with no line break.
    Text(String),
    /// Text that may hold line breaks, printed exactly as given: a multi-line
    /// string literal or block comment. Only its first line counts when
    /// fitting, and its inner lines are never re-indented.
    Verbatim(String),
    /// A space when its group is flat, a line break when broken.
    Line,
    /// Nothing when its group is flat, a line break when broken.
    SoftLine,
    /// Always a line break. It breaks every group around it.
    HardLine,
    /// One more level of indentation after each line break inside.
    Nest(Box<Doc>),
    /// Flat if it fits, otherwise broken. The flag says it must break;
    /// [`render`] works it out before printing.
    Group(Box<Doc>, bool),
    /// The first document when the enclosing group is broken, the second
    /// when it is flat.
    IfBreak(Box<Doc>, Box<Doc>),
    /// Text printed just before the next line break, so that a trailing line
    /// comment goes after the `,` printed after it.
    LineSuffix(String),
    /// Breaks every group around it.
    BreakParent,
    Concat(Vec<Doc>),
}

pub(crate) fn text(s: impl Into<String>) -> Doc {
    Doc::Text(s.into())
}

pub(crate) fn concat(parts: Vec<Doc>) -> Doc {
    Doc::Concat(parts)
}

pub(crate) fn group(doc: Doc) -> Doc {
    Doc::Group(Box::new(doc), false)
}

pub(crate) fn nest(doc: Doc) -> Doc {
    Doc::Nest(Box::new(doc))
}

pub(crate) fn if_break(broken: Doc, flat: Doc) -> Doc {
    Doc::IfBreak(Box::new(broken), Box::new(flat))
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Flat,
    Break,
}

/// Render `doc` with lines `width` columns wide, counted in Unicode scalar
/// values. No line ends in whitespace the renderer added.
pub(crate) fn render(mut doc: Doc, width: usize) -> String {
    propagate(&mut doc);
    let mut out = Out::default();
    let mut stack: Vec<(usize, Mode, &Doc)> = vec![(0, Mode::Break, &doc)];
    while let Some((indent, mode, d)) = stack.pop() {
        match d {
            Doc::Nil | Doc::BreakParent => {}
            Doc::Text(s) | Doc::Verbatim(s) => out.write(s),
            Doc::Concat(parts) => stack.extend(parts.iter().rev().map(|p| (indent, mode, p))),
            Doc::Nest(inner) => stack.push((indent + INDENT, mode, inner)),
            Doc::Group(inner, broken) => {
                let left = width as isize - out.col as isize;
                let flat = mode == Mode::Flat || (!*broken && fits(left, indent, inner, &stack));
                stack.push((indent, if flat { Mode::Flat } else { Mode::Break }, inner));
            }
            Doc::IfBreak(broken, flat) => {
                stack.push((
                    indent,
                    mode,
                    if mode == Mode::Break { broken } else { flat },
                ));
            }
            Doc::LineSuffix(s) => out.suffix.push(s.clone()),
            Doc::Line if mode == Mode::Flat => out.write(" "),
            Doc::SoftLine if mode == Mode::Flat => {}
            Doc::Line | Doc::SoftLine | Doc::HardLine => out.newline(indent),
        }
    }
    out.finish()
}

/// Whether `first`, flat, and then the rest of its line fit in `left`
/// columns. The rest is `rest`, read from its top, in the modes it was
/// pushed with, up to the first line break.
fn fits(mut left: isize, indent: usize, first: &Doc, rest: &[(usize, Mode, &Doc)]) -> bool {
    let mut work: Vec<(usize, Mode, &Doc)> = vec![(indent, Mode::Flat, first)];
    let mut rest_at = rest.len();
    loop {
        if left < 0 {
            return false;
        }
        let (indent, mode, d) = match work.pop() {
            Some(item) => item,
            None if rest_at == 0 => return true,
            None => {
                rest_at -= 1;
                rest[rest_at]
            }
        };
        match d {
            Doc::Nil | Doc::BreakParent | Doc::LineSuffix(_) => {}
            Doc::Text(s) => left -= s.chars().count() as isize,
            Doc::Verbatim(s) => match s.find('\n') {
                Some(i) => return left >= s[..i].chars().count() as isize,
                None => left -= s.chars().count() as isize,
            },
            Doc::Concat(parts) => work.extend(parts.iter().rev().map(|p| (indent, mode, p))),
            Doc::Nest(inner) => work.push((indent + INDENT, mode, inner)),
            Doc::Group(inner, broken) => {
                work.push((indent, if *broken { Mode::Break } else { mode }, inner));
            }
            Doc::IfBreak(broken, flat) => {
                work.push((
                    indent,
                    mode,
                    if mode == Mode::Break { broken } else { flat },
                ));
            }
            Doc::Line if mode == Mode::Flat => left -= 1,
            Doc::SoftLine if mode == Mode::Flat => {}
            Doc::Line | Doc::SoftLine | Doc::HardLine => return true,
        }
    }
}

/// Mark every group that holds a hard break as broken, and say whether
/// `doc` holds one.
fn propagate(doc: &mut Doc) -> bool {
    match doc {
        Doc::HardLine | Doc::BreakParent => true,
        Doc::Nest(inner) => propagate(inner),
        Doc::Group(inner, broken) => {
            let hard = propagate(inner);
            *broken |= hard;
            hard
        }
        Doc::IfBreak(broken, flat) => {
            let a = propagate(broken);
            let b = propagate(flat);
            a || b
        }
        Doc::Concat(parts) => {
            let mut hard = false;
            for p in parts {
                hard |= propagate(p);
            }
            hard
        }
        _ => false,
    }
}

/// The text being written.
#[derive(Default)]
struct Out {
    text: String,
    /// The column the next character lands in, pending indentation included.
    col: usize,
    /// Indentation still to be written before the next text on this line.
    pending: Option<usize>,
    /// Line suffixes waiting for the next line break.
    suffix: Vec<String>,
}

impl Out {
    fn write(&mut self, s: &str) {
        if s.is_empty() {
            return;
        }
        if let Some(n) = self.pending.take() {
            self.text.extend(std::iter::repeat(' ').take(n));
        }
        self.text.push_str(s);
        match s.rfind('\n') {
            Some(i) => self.col = s[i + 1..].chars().count(),
            None => self.col += s.chars().count(),
        }
    }

    fn newline(&mut self, indent: usize) {
        for s in std::mem::take(&mut self.suffix) {
            self.write(&s);
        }
        while self.text.ends_with(' ') {
            self.text.pop();
        }
        self.text.push('\n');
        self.col = indent;
        self.pending = Some(indent);
    }

    fn finish(mut self) -> String {
        for s in std::mem::take(&mut self.suffix) {
            self.write(&s);
        }
        self.text
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_group_that_fits_is_flat() {
        let doc = group(concat(vec![text("a"), Doc::Line, text("b")]));
        assert_eq!(render(doc, 3), "a b");
    }

    #[test]
    fn a_group_that_does_not_fit_breaks() {
        let doc = group(concat(vec![text("a"), Doc::Line, text("b")]));
        assert_eq!(render(doc, 2), "a\nb");
    }

    #[test]
    fn nest_indents_after_each_break() {
        let doc = group(concat(vec![
            text("f("),
            nest(concat(vec![Doc::SoftLine, text("x")])),
            Doc::SoftLine,
            text(")"),
        ]));
        assert_eq!(render(doc.clone(), 4), "f(x)");
        assert_eq!(render(doc, 3), "f(\n    x\n)");
    }

    #[test]
    fn if_break_follows_its_group() {
        let doc = group(concat(vec![
            text("[a"),
            nest(concat(vec![
                Doc::SoftLine,
                text("b"),
                if_break(text(","), Doc::Nil),
            ])),
            Doc::SoftLine,
            text("]"),
        ]));
        assert_eq!(render(doc.clone(), 4), "[ab]");
        assert_eq!(render(doc, 3), "[a\n    b,\n]");
    }

    #[test]
    fn a_hard_line_breaks_every_group_around_it() {
        let doc = group(concat(vec![
            text("a"),
            Doc::Line,
            group(concat(vec![text("b"), Doc::HardLine, text("c")])),
        ]));
        assert_eq!(render(doc, 100), "a\nb\nc");
    }

    #[test]
    fn a_line_suffix_waits_for_the_next_line_break() {
        let doc = concat(vec![
            text("a"),
            Doc::LineSuffix(" // c".into()),
            text(","),
            Doc::HardLine,
            text("b"),
        ]);
        assert_eq!(render(doc, 100), "a, // c\nb");
    }

    #[test]
    fn a_line_suffix_left_at_the_end_is_printed() {
        let doc = concat(vec![text("a"), Doc::LineSuffix(" // end".into())]);
        assert_eq!(render(doc, 100), "a // end");
    }

    #[test]
    fn no_line_ends_in_added_whitespace_and_blank_lines_stay_empty() {
        let doc = nest(concat(vec![
            text("a "),
            Doc::HardLine,
            Doc::HardLine,
            text("b"),
        ]));
        assert_eq!(render(doc, 100), "a\n\n    b");
    }

    #[test]
    fn verbatim_text_counts_only_its_first_line_and_keeps_its_own() {
        let doc = group(concat(vec![
            text("f("),
            nest(concat(vec![
                Doc::SoftLine,
                Doc::Verbatim("\"a\n  b\"".into()),
            ])),
            Doc::SoftLine,
            text(")"),
        ]));
        assert_eq!(render(doc, 5), "f(\"a\n  b\")");
    }

    #[test]
    fn columns_count_characters_not_bytes() {
        // Ten Thai letters are 30 bytes but 10 columns.
        let thai = "ก".repeat(10);
        let doc = group(concat(vec![text(thai.clone()), Doc::Line, text("x")]));
        assert_eq!(render(doc.clone(), 12), format!("{thai} x"));
        assert_eq!(render(doc, 11), format!("{thai}\nx"));
    }
}
