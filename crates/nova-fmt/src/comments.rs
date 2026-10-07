//! The comment cursor (spec
//! `docs/superpowers/specs/2026-10-07-phase-3-1-formatter-design.md` §5.4).
//!
//! The printer reaches the AST's nodes in source order and takes each
//! comment as it reaches it, so no comment can come out of order (the 3.1
//! plan's decision 5).

use nova_lexer::{Comment, CommentKind};

/// One input's comments, and how many the printer has taken.
pub(crate) struct Comments<'c> {
    all: &'c [Comment],
    next: usize,
}

impl<'c> Comments<'c> {
    pub fn new(all: &'c [Comment]) -> Comments<'c> {
        Comments { all, next: 0 }
    }

    /// Take every comment not yet taken that starts before `offset`.
    pub fn take_before(&mut self, offset: u32) -> &'c [Comment] {
        let start = self.next;
        while self.next_starts_before(offset) {
            self.next += 1;
        }
        &self.all[start..self.next]
    }

    /// Whether a comment not yet taken starts before `offset`.
    pub fn next_starts_before(&self, offset: u32) -> bool {
        self.all
            .get(self.next)
            .is_some_and(|c| c.span.start < offset)
    }

    /// Take the comments that end `offset`'s line in `text`: those after
    /// `offset` with only spaces, tabs and each other before them, up to a
    /// line comment, or a block comment the end of the line follows. A block
    /// comment with code after it on its line is left for that code (spec
    /// §5.4, "remaining").
    pub fn take_trailing(&mut self, text: &str, offset: u32) -> &'c [Comment] {
        let start = self.next;
        let mut end = start;
        let mut at = offset as usize;
        let mut k = start;
        while let Some(c) = self.all.get(k) {
            at += spaces(&text[at..]);
            if c.span.start as usize != at {
                break;
            }
            at = c.span.end as usize;
            k += 1;
            if c.kind == CommentKind::Line {
                end = k;
                break;
            }
            let rest = &text[at..];
            let after = &rest[spaces(rest)..];
            if after.is_empty() || after.starts_with('\n') {
                end = k;
            }
        }
        self.next = end;
        &self.all[start..end]
    }
}

/// How many spaces and tabs `s` starts with.
fn spaces(s: &str) -> usize {
    s.len() - s.trim_start_matches(|c| c == ' ' || c == '\t').len()
}

#[cfg(test)]
mod tests {
    use super::*;
    use nova_diagnostics::FileDb;

    fn comments(text: &str) -> Vec<Comment> {
        let mut db = FileDb::new();
        let id = db.add("t.nova", text);
        nova_lexer::lex_with_comments(text, id).1
    }

    fn texts<'a>(text: &'a str, taken: &[Comment]) -> Vec<&'a str> {
        taken
            .iter()
            .map(|c| &text[c.span.start as usize..c.span.end as usize])
            .collect()
    }

    #[test]
    fn take_before_takes_each_comment_once_in_order() {
        let text = "a // one\nb /* two */ c\n// three\n";
        let all = comments(text);
        let mut cs = Comments::new(&all);
        let end = text.len() as u32;
        assert_eq!(texts(text, cs.take_before(9)), vec!["// one"]);
        assert!(cs.next_starts_before(end));
        assert_eq!(
            texts(text, cs.take_before(end)),
            vec!["/* two */", "// three"]
        );
        assert!(cs.take_before(end).is_empty());
        assert!(!cs.next_starts_before(end));
    }

    #[test]
    fn take_trailing_takes_only_what_ends_the_line() {
        let text = "a /* x */ // y\nb /* z */ c\nd\n// own\n";
        let all = comments(text);
        let mut cs = Comments::new(&all);
        let after = |ch: char| text.find(ch).unwrap() as u32 + 1;
        // After `a`, a block comment and a line comment end the line.
        assert_eq!(
            texts(text, cs.take_trailing(text, after('a'))),
            vec!["/* x */", "// y"]
        );
        // After `b`, code follows the block comment: it belongs to `c`.
        assert!(cs.take_trailing(text, after('b')).is_empty());
        assert_eq!(texts(text, cs.take_before(after('c') - 1)), vec!["/* z */"]);
        // After `d`, the next comment is on a line of its own.
        assert!(cs.take_trailing(text, after('d')).is_empty());
        assert!(cs.next_starts_before(text.len() as u32));
    }
}
