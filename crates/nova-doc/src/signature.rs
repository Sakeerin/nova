//! Signatures as written (spec §8; plan decisions 10 and 11).

use nova_ast::item::WhereBound;
use nova_ast::Type;
use nova_diagnostics::{lines, FileId, Span, Spanned};
use nova_lexer::{Comment, Token};

use crate::highlight::class;
use crate::markdown::DocText;
use crate::{escape, span};

/// One file's text, with its tokens and its comments, lexed once.
pub(crate) struct FileText<'a> {
    pub text: &'a str,
    pub tokens: Vec<Spanned<Token>>,
    pub comments: Vec<Comment>,
}

impl<'a> FileText<'a> {
    pub fn new(text: &'a str, file: FileId) -> FileText<'a> {
        let (tokens, comments, _) = nova_lexer::lex_with_comments(text, file);
        FileText {
            text,
            tokens,
            comments,
        }
    }

    /// Where the first token `want` accepts starts, at or after byte
    /// `from`.
    pub fn next(&self, from: usize, want: fn(&Token) -> bool) -> Option<usize> {
        self.tokens
            .iter()
            .find(|t| t.span.start as usize >= from && want(&t.value))
            .map(|t| t.span.start as usize)
    }

    /// Where the `(…)` that opens at the first `(` at or after byte `from`
    /// ends.
    pub fn group_end(&self, from: usize) -> Option<usize> {
        let mut depth = 0usize;
        for t in self.tokens.iter().filter(|t| t.span.start as usize >= from) {
            match t.value {
                Token::LParen => depth += 1,
                Token::RParen if depth == 1 => return Some(t.span.end as usize),
                Token::RParen if depth > 1 => depth -= 1,
                _ => {}
            }
        }
        None
    }

    /// The module's docs (spec §9.2): its `//!` lines before its first
    /// token, each without `//!` and one space, and where each starts in
    /// the file (3.5b spec §7). A comment's span ends before its line
    /// break, and before a CRLF's `\r`.
    pub fn module_docs(&self) -> DocText {
        let first = self
            .tokens
            .iter()
            .find(|t| t.value != Token::Eof)
            .map_or(self.text.len(), |t| t.span.start as usize);
        let mut d = DocText::default();
        for c in self
            .comments
            .iter()
            .filter(|c| c.span.end as usize <= first)
        {
            let Some(line) = self.text.get(c.span.start as usize..c.span.end as usize) else {
                continue;
            };
            let Some(rest) = line.strip_prefix("//!") else {
                continue;
            };
            let body = rest.strip_prefix(' ').unwrap_or(rest);
            d.push(
                c.span.file,
                c.span.start + (line.len() - body.len()) as u32,
                body,
            );
        }
        d
    }
}

/// Where a declaration's text starts (spec §8): its keyword, with an
/// `async` and a `pub` before it; or, with no keyword, as for a field or a
/// variant, its name, with a `pub` before it.
pub(crate) fn decl_start(text: &str, name_start: usize) -> usize {
    let at = lines::keyword_start(text, name_start).unwrap_or(name_start);
    let before = text[..at].trim_end_matches([' ', '\t']);
    match before.strip_suffix("pub") {
        Some(rest) if !rest.ends_with(|c: char| c.is_ascii_alphanumeric() || c == '_') => {
            rest.len()
        }
        _ => at,
    }
}

/// Where a function's signature ends (plan decision 10): the `)` closing
/// its parameters, its return type, or its last `where` bound, whichever
/// ends last.
pub(crate) fn fn_end(
    ft: &FileText,
    name: Span,
    ret: Option<&Spanned<Type>>,
    wheres: &[WhereBound],
) -> usize {
    let mut end = ft.group_end(name.end as usize).unwrap_or(name.end as usize);
    if let Some(r) = ret {
        end = end.max(r.span.end as usize);
    }
    for w in wheres {
        end = end.max(w.ty.span.end as usize);
        for b in &w.bounds {
            end = end.max(b.span.end as usize);
        }
    }
    end
}

/// The HTML of the declaration `ft.text[start..end]` (spec §8): comments
/// dropped, its lines less the indentation of the line it starts on,
/// keywords and literals coloured, `own`, its name, marked, and each other
/// name `link` gives a URL to made a link.
pub(crate) fn render(
    ft: &FileText,
    start: usize,
    end: usize,
    own: Option<Span>,
    link: &dyn Fn(Span) -> Option<String>,
) -> String {
    let text = ft.text;
    let line = lines::line_start(text, start);
    let base = text[line..].len() - text[line..].trim_start_matches([' ', '\t']).len();
    let mut out = String::new();
    let mut last: Option<usize> = None;
    for t in &ft.tokens {
        let (s, e) = (t.span.start as usize, t.span.end as usize);
        if s < start || e > end || matches!(t.value, Token::Eof | Token::DocComment(_)) {
            continue;
        }
        if let Some(prev) = last {
            out.push_str(&gap(&text[prev..s], base));
        }
        let word = &text[s..e];
        match &t.value {
            Token::Ident(_) if own == Some(t.span) => span(&mut out, "name", word),
            Token::Ident(_) => match link(t.span) {
                Some(url) => {
                    out.push_str("<a href=\"");
                    out.push_str(&escape(&url));
                    out.push_str("\">");
                    out.push_str(&escape(word));
                    out.push_str("</a>");
                }
                None => out.push_str(&escape(word)),
            },
            other => match class(other) {
                Some(c) => span(&mut out, c, word),
                None => out.push_str(&escape(word)),
            },
        }
        last = Some(e);
    }
    out
}

/// What lies between two tokens, which is only spaces and comments, as it
/// shows (plan decision 11): nothing, one space, or a line break and the
/// next line's indentation less `base` characters. A block comment counts
/// as one space, except one that begins a line, which goes with the spaces
/// after it, so the line keeps its own indentation.
pub(crate) fn gap(between: &str, base: usize) -> String {
    let mut plain = String::new();
    let mut rest = between;
    while let Some(c) = rest.chars().next() {
        if let Some(after) = rest.strip_prefix("//") {
            rest = &after[after.find('\n').unwrap_or(after.len())..];
        } else if let Some(after) = rest.strip_prefix("/*") {
            rest = &after[after.find("*/").map_or(after.len(), |i| i + 2)..];
            let line = plain.rfind('\n').map(|i| &plain[i + 1..]);
            if line.is_some_and(|l| l.chars().all(|c| matches!(c, ' ' | '\t'))) {
                rest = rest.trim_start_matches([' ', '\t']);
            } else {
                plain.push(' ');
            }
        } else {
            plain.push(c);
            rest = &rest[c.len_utf8()..];
        }
    }
    match plain.rfind('\n') {
        None if plain.is_empty() => String::new(),
        None => " ".to_string(),
        Some(i) => {
            let indent = &plain[i + 1..];
            let drop = indent
                .chars()
                .take(base)
                .take_while(|c| matches!(c, ' ' | '\t'))
                .count();
            format!("\n{}", &indent[drop..])
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nova_ast::item::TypeDef;
    use nova_ast::Item;

    fn parse(text: &str) -> nova_ast::File {
        let (tokens, errors) = nova_lexer::lex(text, FileId::DUMMY);
        assert!(errors.is_empty(), "{errors:?}");
        let (file, errors) = nova_parser::parse(&tokens, FileId::DUMMY);
        assert!(errors.is_empty(), "{errors:?}");
        file.expect("the file parses")
    }

    #[test]
    fn the_gap_between_two_tokens() {
        assert_eq!(gap("", 0), "");
        assert_eq!(gap("   ", 0), " ");
        assert_eq!(gap(" /* c */ ", 0), " ");
        assert_eq!(gap("/*c*/", 0), " ");
        assert_eq!(gap(" // why\n        ", 4), "\n    ");
        assert_eq!(gap("\r\n\r\n    ", 4), "\n");
        // A block comment that begins a line takes the spaces after it.
        assert_eq!(gap("\n        /* the left */ ", 4), "\n    ");
        assert_eq!(gap("\n  /* a */\t/* b */ ", 0), "\n  ");
        assert_eq!(gap(" // why\n  /* c */\n    ", 2), "\n  ");
    }

    #[test]
    fn a_function_stops_before_its_body_and_keeps_pub_async_and_where() {
        let text = "/// Doc.\npub async fn go<T>(x: T) -> T where T: Show { x }\n";
        let file = parse(text);
        let Item::Function(f) = &file.items[0].value else {
            panic!("not a function")
        };
        let ft = FileText::new(text, FileId::DUMMY);
        let start = decl_start(text, f.name.span.start as usize);
        let end = fn_end(&ft, f.name.span, f.return_ty.as_ref(), &f.where_clause);
        assert_eq!(
            render(&ft, start, end, Some(f.name.span), &|_| None),
            "<span class=\"kw\">pub</span> <span class=\"kw\">async</span> \
             <span class=\"kw\">fn</span> <span class=\"name\">go</span>&lt;T&gt;(x: T) -&gt; T \
             <span class=\"kw\">where</span> T: Show"
        );
    }

    #[test]
    fn a_broken_signature_keeps_its_lines_less_their_indentation() {
        // Review Focus 1.
        let text = "impl P {\n    pub fn far(\n        self,\n        // the distance\n        to: Int,\n    ) -> Int {\n        to\n    }\n}\n";
        let file = parse(text);
        let Item::Impl(imp) = &file.items[0].value else {
            panic!("not an impl")
        };
        let f = &imp.functions[0];
        let ft = FileText::new(text, FileId::DUMMY);
        let start = decl_start(text, f.name.span.start as usize);
        let end = fn_end(&ft, f.name.span, f.return_ty.as_ref(), &f.where_clause);
        assert_eq!(
            render(&ft, start, end, Some(f.name.span), &|_| None),
            "<span class=\"kw\">pub</span> <span class=\"kw\">fn</span> \
             <span class=\"name\">far</span>(\n    <span class=\"kw\">self</span>,\n    \
             to: Int,\n) -&gt; Int"
        );
    }

    #[test]
    fn a_block_comment_that_begins_a_line_keeps_its_indentation() {
        let text = "impl P {\n    pub fn far(\n        self,\n        /* the left */ to: Int,\n    ) -> Int {\n        to\n    }\n}\n";
        let file = parse(text);
        let Item::Impl(imp) = &file.items[0].value else {
            panic!("not an impl")
        };
        let f = &imp.functions[0];
        let ft = FileText::new(text, FileId::DUMMY);
        let start = decl_start(text, f.name.span.start as usize);
        let end = fn_end(&ft, f.name.span, f.return_ty.as_ref(), &f.where_clause);
        assert_eq!(
            render(&ft, start, end, Some(f.name.span), &|_| None),
            "<span class=\"kw\">pub</span> <span class=\"kw\">fn</span> \
             <span class=\"name\">far</span>(\n    <span class=\"kw\">self</span>,\n    \
             to: Int,\n) -&gt; Int"
        );
    }

    #[test]
    fn names_link_and_the_items_own_name_does_not() {
        let text = "pub fn area(r: Rect, n: Int) -> Float { 0.0 }\n";
        let ft = FileText::new(text, FileId::DUMMY);
        let at = text.find("Rect").unwrap() as u32;
        let rect = Span::new(at, at + 4, FileId::DUMMY);
        let own = Span::new(7, 11, FileId::DUMMY);
        let end = text.find(" {").unwrap();
        let link = |s: Span| (s == rect).then(|| "../g/index.html#record.Rect".to_string());
        assert_eq!(
            render(&ft, 0, end, Some(own), &link),
            "<span class=\"kw\">pub</span> <span class=\"kw\">fn</span> \
             <span class=\"name\">area</span>(r: <a href=\"../g/index.html#record.Rect\">Rect</a>, \
             n: Int) -&gt; Float"
        );
    }

    #[test]
    fn a_record_a_sum_type_a_variant_a_trait_and_a_constant_end_where_they_should() {
        let text = "pub record P<T> { x: T }\npub type S = | A | B(Int)\npub trait Show: Eq { fn show(self) -> String }\npub const N: Int = 1 + 2\n";
        let file = parse(text);
        let ft = FileText::new(text, FileId::DUMMY);
        let plain = |start: usize, end: usize| render(&ft, start, end, None, &|_| None);
        let brace = |from: u32| ft.next(from as usize, |t| *t == Token::LBrace).unwrap();
        let Item::Record(r) = &file.items[0].value else {
            panic!()
        };
        assert_eq!(
            plain(
                decl_start(text, r.name.span.start as usize),
                brace(r.name.span.end)
            ),
            "<span class=\"kw\">pub</span> <span class=\"kw\">record</span> P&lt;T&gt;"
        );
        let Item::Type(t) = &file.items[1].value else {
            panic!()
        };
        let eq = ft
            .next(t.name.span.end as usize, |t| *t == Token::Eq)
            .unwrap();
        assert_eq!(
            plain(decl_start(text, t.name.span.start as usize), eq),
            "<span class=\"kw\">pub</span> <span class=\"kw\">type</span> S"
        );
        let TypeDef::Sum(variants) = &t.def else {
            panic!()
        };
        let b = &variants[1];
        let end = ft.group_end(b.name.span.end as usize).unwrap();
        assert_eq!(plain(b.name.span.start as usize, end), "B(Int)");
        let Item::Trait(tr) = &file.items[2].value else {
            panic!()
        };
        assert_eq!(
            plain(
                decl_start(text, tr.name.span.start as usize),
                brace(tr.name.span.end)
            ),
            "<span class=\"kw\">pub</span> <span class=\"kw\">trait</span> Show: Eq"
        );
        let Item::Const(c) = &file.items[3].value else {
            panic!()
        };
        assert_eq!(
            plain(
                decl_start(text, c.name.span.start as usize),
                c.value.span.end as usize
            ),
            "<span class=\"kw\">pub</span> <span class=\"kw\">const</span> N: Int = \
             <span class=\"lit\">1</span> + <span class=\"lit\">2</span>"
        );
    }

    #[test]
    fn a_field_starts_at_its_pub() {
        let text = "record P {\n    pub x: Int,\n    y: Int,\n}\n";
        assert_eq!(
            decl_start(text, text.find("x:").unwrap()),
            text.find("pub x").unwrap()
        );
        assert_eq!(
            decl_start(text, text.find("y:").unwrap()),
            text.find("y:").unwrap()
        );
    }

    #[test]
    fn module_docs_are_the_bang_lines_before_the_first_token() {
        let ft = FileText::new(
            "// A note.\n//! The core.\n//!\n//! More.\nfn f() {}\n//! Not docs.\n",
            FileId::DUMMY,
        );
        assert_eq!(ft.module_docs().text, "The core.\n\nMore.");
        assert_eq!(
            FileText::new("fn f() {}\n", FileId::DUMMY)
                .module_docs()
                .text,
            ""
        );
        assert_eq!(
            FileText::new("//! Only docs.\n", FileId::DUMMY)
                .module_docs()
                .text,
            "Only docs."
        );
    }

    #[test]
    fn module_docs_map_back_to_their_source() {
        // 3.5b spec §7: `//!` lines need not be adjacent, so each keeps its
        // own place; and the same with CRLF.
        for src in [
            "//! One.\n// A note.\n//!Two.\n/* block */\n//!   three\n//!\nfn f() {}\n",
            "//! One.\r\n// A note.\r\n//!Two.\r\n/* block */\r\n//!   three\r\n//!\r\nfn f() {}\r\n",
        ] {
            let d = FileText::new(src, FileId::DUMMY).module_docs();
            assert_eq!(d.text, "One.\nTwo.\n  three\n", "{src:?}");
            assert_eq!(d.file, Some(FileId::DUMMY));
            for word in ["One.", "Two.", "three", "hree"] {
                assert_eq!(
                    d.source(d.text.find(word).unwrap()),
                    Some(src.find(word).unwrap() as u32),
                    "{word:?} in {src:?}"
                );
            }
            // The bare `//!` is the end of its `//!`.
            let bare = src.rfind("//!").unwrap() as u32;
            assert_eq!(d.source(d.text.len()), Some(bare + 3));
        }
        assert_eq!(
            FileText::new("fn f() {}\n", FileId::DUMMY)
                .module_docs()
                .file,
            None
        );
    }
}
