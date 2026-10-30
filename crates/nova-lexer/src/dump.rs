//! The token dump: one source's tokens, lex errors and comments as text, one
//! item per line, which the self-hosted lexer (`selfhost/`) prints too, so
//! the two lexers can be compared byte for byte (spec
//! `docs/superpowers/specs/2026-10-30-phase-5-1-the-lexer-the-parser-and-the-ast-design.md`
//! §3.3, ADR 0051). It carries no stability promise: it changes whenever
//! the tokens do, on both sides in one pull request.
//!
//! Per source, in this order:
//!
//! - one line per token, `Eof` last: `start end Kind[ payload][ NL][ PD]`.
//!   - `start` and `end` are the span's byte offsets, in decimal.
//!   - `Kind` is the [`Token`] variant's name, as its `Debug` prints it.
//!   - The payload: `Int` in decimal; `Float` as the 16 lowercase hex digits
//!     of `f64::to_bits`; `Char` as its code point in decimal; `StrPart`,
//!     `RawStr`, `Ident` and `DocComment` as their text in double quotes,
//!     with each byte below 0x20, 0x7F, `"` and `\` written `\xHH` (two
//!     lowercase hex digits) and every other byte as it is. No other kind
//!     has one.
//!   - `NL` when [`Tokens::line_break_before`] holds, then `PD` when
//!     [`Tokens::plain_decimal`] does.
//! - one line per lex error, in the order the lexer found them:
//!   `ERR start end message`, the message being the error's `Display`, with
//!   each byte below 0x20 and 0x7F written `\xHH`; `"` and `\` stay as they
//!   are, since the message is not quoted.
//! - one line per comment, in source order: `COMMENT start end Line` or
//!   `COMMENT start end Block`.
//!
//! Every line ends with `\n`, the last one included. The `== FILE` header of
//! `nova parse --dump-tokens` is the command's, not this module's.

use std::fmt::Write;

use nova_diagnostics::FileId;

use crate::{lex_with_comments, CommentKind, Token, Tokens};

/// The token dump of `source` (module docs).
pub fn dump_tokens(source: &str) -> String {
    let (tokens, comments, errors) = lex_with_comments(source, FileId::DUMMY);
    let mut out = String::with_capacity(source.len() * 4);
    dump_token_lines(&mut out, &tokens);
    for e in &errors {
        let span = e.span();
        let _ = write!(out, "ERR {} {} ", span.start, span.end);
        escape_message(&mut out, &e.to_string());
        out.push('\n');
    }
    for c in &comments {
        let kind = match c.kind {
            CommentKind::Line => "Line",
            CommentKind::Block => "Block",
        };
        let _ = writeln!(out, "COMMENT {} {} {kind}", c.span.start, c.span.end);
    }
    out
}

fn dump_token_lines(out: &mut String, tokens: &Tokens) {
    for (i, t) in tokens.iter().enumerate() {
        let _ = write!(out, "{} {} ", t.span.start, t.span.end);
        match &t.value {
            Token::Int(n) => {
                let _ = write!(out, "Int {n}");
            }
            Token::Float(f) => {
                let _ = write!(out, "Float {:016x}", f.to_bits());
            }
            Token::Char(c) => {
                let _ = write!(out, "Char {}", *c as u32);
            }
            Token::StrPart(s) => quoted(out, "StrPart", s),
            Token::RawStr(s) => quoted(out, "RawStr", s),
            Token::Ident(s) => quoted(out, "Ident", s),
            Token::DocComment(s) => quoted(out, "DocComment", s),
            other => {
                let _ = write!(out, "{other:?}");
            }
        }
        if tokens.line_break_before(i) {
            out.push_str(" NL");
        }
        if tokens.plain_decimal(i) {
            out.push_str(" PD");
        }
        out.push('\n');
    }
}

fn quoted(out: &mut String, kind: &str, text: &str) {
    out.push_str(kind);
    out.push_str(" \"");
    escape_text(out, text);
    out.push('"');
}

/// A string payload's text: each byte below 0x20, 0x7F, `"` and `\` as
/// `\xHH`, two lowercase hex digits; every other byte, so every non-ASCII
/// character, as it is.
fn escape_text(out: &mut String, text: &str) {
    for c in text.chars() {
        if (c as u32) < 0x20 || c == '\x7f' || c == '"' || c == '\\' {
            let _ = write!(out, "\\x{:02x}", c as u32);
        } else {
            out.push(c);
        }
    }
}

/// An error's message: each byte below 0x20 and 0x7F as `\xHH`, so the
/// message stays on its line; `"` and `\` as they are, since the message is
/// not quoted.
fn escape_message(out: &mut String, text: &str) {
    for c in text.chars() {
        if (c as u32) < 0x20 || c == '\x7f' {
            let _ = write!(out, "\\x{:02x}", c as u32);
        } else {
            out.push(c);
        }
    }
}
