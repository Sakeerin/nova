//! Placing an edit by the lines of a source text (spec
//! `docs/superpowers/specs/2026-10-10-phase-3-4b-fixes-and-colour-design.md`
//! §4.7). Offsets are bytes. The rules step over ASCII only, so every
//! offset they return is on a character boundary.

/// The text's own line ending: `\r\n` when it holds one, else `\n`.
pub fn ending(text: &str) -> &'static str {
    if text.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    }
}

/// Where the line holding byte `at` starts.
pub fn line_start(text: &str, at: usize) -> usize {
    text[..at].rfind('\n').map_or(0, |i| i + 1)
}

/// Where the line after the one holding byte `at` starts, or the text's
/// end.
pub fn next_line_start(text: &str, at: usize) -> usize {
    text[at..].find('\n').map_or(text.len(), |i| at + i + 1)
}

/// Whether `line` holds only spaces, tabs and its line ending.
pub fn is_blank(line: &str) -> bool {
    line.chars().all(|c| matches!(c, ' ' | '\t' | '\r' | '\n'))
}

/// Whether `line`'s first characters after spaces and tabs are `//`.
pub fn is_comment_line(line: &str) -> bool {
    line.trim_start_matches([' ', '\t']).starts_with("//")
}

/// The edit adding `line` on a line of its own after the line holding byte
/// `at`: where it goes, and what it inserts, in the text's own ending.
pub fn line_after(text: &str, at: usize, line: &str) -> (usize, String) {
    let nl = ending(text);
    // Past spaces and whole block comments, so a block comment that starts
    // on this line and goes on stays whole (spec 3.5a §3, follow-up 9).
    let bytes = text.as_bytes();
    let mut i = at;
    loop {
        while i < bytes.len() && matches!(bytes[i], b' ' | b'\t') {
            i += 1;
        }
        if !text[i..].starts_with("/*") {
            break;
        }
        i = text[i + 2..]
            .find("*/")
            .map_or(text.len(), |n| i + 2 + n + 2);
    }
    let next = next_line_start(text, i);
    if next == text.len() && !text.ends_with('\n') {
        (next, format!("{nl}{line}"))
    } else {
        (next, format!("{line}{nl}"))
    }
}

/// The edit adding `line` and a blank line before the item starting at
/// byte `item_start`, above the comment lines directly over it. A comment
/// a blank line separates from the item stays above (spec §4.7).
pub fn line_before_item(text: &str, item_start: usize, line: &str) -> (usize, String) {
    let nl = ending(text);
    let mut at = line_start(text, item_start);
    // The comments directly above, each with no blank line after it (spec
    // 3.5a §3, follow-up 9). One that ends on the line `at` starts, beside
    // a comment already taken or the item, comes with that line.
    for &(start, end) in comments_before(text, at).iter().rev() {
        if end > at {
            at = at.min(line_start(text, start));
            continue;
        }
        if text[end..at].matches('\n').count() > 1 {
            break;
        }
        at = line_start(text, start);
    }
    (at, format!("{line}{nl}{nl}"))
}

/// The comments before byte `end` of `text`, which holds only comments
/// and blank space there, by a scan from its start (plan decision 5).
/// Block comments do not nest.
fn comments_before(text: &str, end: usize) -> Vec<(usize, usize)> {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < end {
        match bytes[i] {
            b' ' | b'\t' | b'\r' | b'\n' => i += 1,
            b'/' if bytes.get(i + 1) == Some(&b'/') => {
                let e = text[i..].find('\n').map_or(text.len(), |n| i + n);
                out.push((i, e));
                i = e;
            }
            b'/' if bytes.get(i + 1) == Some(&b'*') => {
                let e = text[i + 2..]
                    .find("*/")
                    .map_or(text.len(), |n| i + 2 + n + 2);
                out.push((i, e));
                i = e;
            }
            _ => break,
        }
    }
    out
}

/// Where the keyword before the item name at byte `name_start` starts:
/// `fn`, `record`, `type`, `trait` or `const`, with an `async` before
/// `fn` (spec §4.7).
pub fn keyword_start(text: &str, name_start: usize) -> Option<usize> {
    let bytes = text.as_bytes();
    let back_over_space = |mut i: usize| {
        while i > 0 && matches!(bytes[i - 1], b' ' | b'\t') {
            i -= 1;
        }
        i
    };
    let back_over_word = |mut i: usize| {
        while i > 0 && bytes[i - 1].is_ascii_alphabetic() {
            i -= 1;
        }
        i
    };
    let end = back_over_space(name_start);
    let start = back_over_word(end);
    let keyword = &text[start..end];
    if !matches!(
        keyword,
        "fn" | "record" | "type" | "trait" | "const" | "component"
    ) {
        return None;
    }
    if keyword == "fn" {
        let end = back_over_space(start);
        let word = back_over_word(end);
        if &text[word..end] == "async" {
            return Some(word);
        }
    }
    Some(start)
}

/// What removing a match arm, bytes `start..end`, takes (spec §4.7): a
/// comma after it and the spaces after that, and when nothing else shares
/// its lines but a `//` comment ending them, those whole lines. `None`
/// when anything but a comma, a `//` comment, a line ending, the text's end
/// or `}` follows the arm (plan decision 13).
pub fn arm_removal(text: &str, start: usize, end: usize) -> Option<(usize, usize)> {
    let bytes = text.as_bytes();
    let skip = |mut i: usize| {
        while i < bytes.len() && matches!(bytes[i], b' ' | b'\t') {
            i += 1;
        }
        i
    };
    let mut after = skip(end);
    match bytes.get(after) {
        Some(b',') => after = skip(after + 1),
        None | Some(b'\r' | b'\n' | b'}') => {}
        Some(b'/') if bytes.get(after + 1) == Some(&b'/') => {}
        Some(_) => return None,
    }
    let first = line_start(text, start);
    let alone_before = text[first..start]
        .bytes()
        .all(|b| matches!(b, b' ' | b'\t'));
    let alone_after =
        text[after..].starts_with("//") || matches!(bytes.get(after), None | Some(b'\r' | b'\n'));
    if alone_before && alone_after {
        Some((first, next_line_start(text, after)))
    } else {
        Some((start, after))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_line_after_keeps_a_trailing_comment_on_its_line() {
        let text = "import a // why\nfn main() {}\n";
        assert_eq!(
            line_after(text, "import a".len(), "import b::{x}"),
            (16, "import b::{x}\n".to_string())
        );
        let crlf = "import a\r\nfn f() {}\r\n";
        assert_eq!(
            line_after(crlf, 8, "import b"),
            (10, "import b\r\n".to_string())
        );
    }

    #[test]
    fn a_line_after_the_last_line_without_an_ending_gets_one() {
        assert_eq!(
            line_after("import a", 8, "import b"),
            (8, "\nimport b".to_string())
        );
    }

    #[test]
    fn a_line_before_an_item_goes_above_its_comments_and_below_a_header() {
        let text = "// header\n\n// about f\n/// Doc.\nfn f() {}\n";
        let item = text.find("/// Doc.").unwrap();
        assert_eq!(
            line_before_item(text, item, "import m::{x}"),
            (11, "import m::{x}\n\n".to_string())
        );
    }

    #[test]
    fn the_keyword_before_a_name_takes_async_with_it() {
        let text = "async fn go() {}\nrecord P {}\ntype S =\n  | A\nfn f() {}\nlet x = 1\n";
        assert_eq!(keyword_start(text, text.find("go").unwrap()), Some(0));
        assert_eq!(
            keyword_start(text, text.find("P {").unwrap()),
            text.find("record")
        );
        assert_eq!(
            keyword_start(text, text.find("S =").unwrap()),
            text.find("type")
        );
        assert_eq!(
            keyword_start(text, text.find("f()").unwrap()),
            text.find("fn f")
        );
        assert_eq!(keyword_start(text, text.find("x =").unwrap()), None);
        // A component's keyword, which only an item's place makes one (the
        // 4.4 spec §3.1).
        let text = "component Card() {}\n";
        assert_eq!(keyword_start(text, text.find("Card").unwrap()), Some(0));
    }

    #[test]
    fn an_arm_alone_on_its_lines_goes_with_them() {
        let text = "match x {\n    1 => a,\n    _ => {\n        b\n    }\n}\n";
        let start = text.find("1 =>").unwrap();
        assert_eq!(
            arm_removal(text, start, start + "1 => a".len()),
            Some((10, 22))
        );
        let start = text.find("_ =>").unwrap();
        let end = text.rfind("    }").unwrap() + "    }".len();
        assert_eq!(
            arm_removal(text, start, end).map(|(s, e)| &text[s..e]),
            Some("    _ => {\n        b\n    }\n")
        );
        // A comment ending the arm's line goes with it.
        let text = "match x {\n    1 => a, // one\n    _ => b\n}\n";
        let start = text.find("1 =>").unwrap();
        assert_eq!(
            arm_removal(text, start, start + "1 => a".len()),
            Some((10, 29))
        );
    }

    #[test]
    fn an_arm_sharing_its_line_goes_alone_with_its_comma() {
        let text = "match x { 1 => a, 2 => b }";
        let start = text.find("1 =>").unwrap();
        assert_eq!(
            arm_removal(text, start, start + 6).map(|(s, e)| &text[s..e]),
            Some("1 => a, ")
        );
        let start = text.find("2 =>").unwrap();
        assert_eq!(
            arm_removal(text, start, start + 6).map(|(s, e)| &text[s..e]),
            Some("2 => b ")
        );
        // A comment between the body and its comma: no fix (plan decision 13).
        assert_eq!(arm_removal("1 => a /* c */,", 0, 6), None);
    }

    #[test]
    fn the_line_ending_is_the_texts_own() {
        assert_eq!(ending("a\r\nb\r\n"), "\r\n");
        assert_eq!(ending("a\nb"), "\n");
        assert!(is_blank("  \t\r\n") && !is_blank(" x\n"));
        assert!(is_comment_line("    // c\n") && !is_comment_line("x // c\n"));
    }

    #[test]
    fn a_line_after_an_import_goes_past_a_block_comment_that_goes_on() {
        // Follow-up 9 (spec 3.5a §3; plan decision 5).
        let text = "import a /* one\n   two */\nfn main() {}\n";
        let after = text.find("two */\n").unwrap() + "two */\n".len();
        assert_eq!(
            line_after(text, "import a".len(), "import b::{x}"),
            (after, "import b::{x}\n".to_string())
        );
        let crlf = text.replace('\n', "\r\n");
        let after = crlf.find("two */\r\n").unwrap() + "two */\r\n".len();
        assert_eq!(
            line_after(&crlf, "import a".len(), "import b::{x}"),
            (after, "import b::{x}\r\n".to_string())
        );
    }

    #[test]
    fn a_line_before_an_item_goes_above_a_block_comment_over_it() {
        // Follow-up 9: the comment's blank line is inside it, and the
        // header a blank line separates stays above.
        let text = "// header\n\n/* about f\n\n   more */\nfn f() {}\n";
        let item = text.find("fn f").unwrap();
        assert_eq!(
            line_before_item(text, item, "import m::{x}"),
            (
                text.find("/* about").unwrap(),
                "import m::{x}\n\n".to_string()
            )
        );
        let crlf = text.replace('\n', "\r\n");
        let item = crlf.find("fn f").unwrap();
        assert_eq!(
            line_before_item(&crlf, item, "import m::{x}"),
            (
                crlf.find("/* about").unwrap(),
                "import m::{x}\r\n\r\n".to_string()
            )
        );
    }

    #[test]
    fn comments_sharing_a_line_above_an_item_go_with_it() {
        // Follow-up 9: two comments on one line, and a block comment ending
        // on the item's own line, are walked over whole, not sliced.
        let two = "/* a */ // b\nfn f() {}\n";
        assert_eq!(
            line_before_item(two, two.find("fn f").unwrap(), "import m::{x}"),
            (0, "import m::{x}\n\n".to_string())
        );
        let shared = "/* a\n */ fn f() {}\n";
        assert_eq!(
            line_before_item(shared, shared.find("fn f").unwrap(), "import m::{x}"),
            (0, "import m::{x}\n\n".to_string())
        );
    }
}
