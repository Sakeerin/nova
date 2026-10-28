//! [`Tokens`]: the lexer's output, each token marked with whether a line
//! break comes before it (spec
//! `docs/superpowers/specs/2026-10-26-phase-5-0-the-language-and-std-a-compiler-needs-design.md`
//! §3.5).

use std::ops::Deref;

use nova_diagnostics::Spanned;

use crate::Token;

/// The tokens of one source, in order, ending with [`Token::Eof`], and for
/// each one whether a line break comes before it.
///
/// The parser reads the marks for the newline rule: in statement context, a
/// line break ends an expression before a `(`, a `[` or a binary `-`, and a
/// `return` or `break` at the end of a line takes no value. A line break
/// inside a comment between two tokens counts, so a `//` comment, or a `/*
/// */` comment that holds one, marks the token after it. A line break is a
/// `\n`, or a lone `\r`, an old Mac line ending.
///
/// It reads as a slice of the tokens, so code that only walks the tokens
/// needs no change; [`lex`](crate::lex) is the one place that builds it, so
/// no caller of the parser can leave the marks out.
#[derive(Debug, Clone, Default)]
pub struct Tokens {
    tokens: Vec<Spanned<Token>>,
    line_break_before: Vec<bool>,
    /// For each token, whether it is an integer spelled in plain decimal
    /// digits with no leading zero: what a tuple index `t.N` must be.
    plain_decimal: Vec<bool>,
}

impl Tokens {
    /// `tokens`, lexed from `source`, each marked by the source text between
    /// it and the token before it (or the start of the source).
    pub fn marked(tokens: Vec<Spanned<Token>>, source: &str) -> Self {
        let bytes = source.as_bytes();
        let mut prev_end = 0usize;
        let line_break_before = tokens
            .iter()
            .map(|t| {
                let start = (t.span.start as usize).min(bytes.len());
                let gap = bytes.get(prev_end.min(start)..start).unwrap_or(&[]);
                prev_end = prev_end.max(t.span.end as usize);
                gap.iter().any(|&b| b == b'\n' || b == b'\r')
            })
            .collect();
        let plain_decimal = tokens
            .iter()
            .map(|t| {
                let text = bytes
                    .get(t.span.start as usize..t.span.end as usize)
                    .unwrap_or(&[]);
                matches!(t.value, Token::Int(_))
                    && !text.is_empty()
                    && text.iter().all(u8::is_ascii_digit)
                    && (text.len() == 1 || text[0] != b'0')
            })
            .collect();
        Self {
            tokens,
            line_break_before,
            plain_decimal,
        }
    }

    /// Whether the token at `index` is an integer written in plain decimal
    /// digits with no leading zero (`0`, `7`, `12`; not `01`, `1_0` or
    /// `0x1`): the only spelling a tuple index `t.N` takes (spec
    /// `docs/superpowers/specs/2026-10-26-phase-5-0-the-language-and-std-a-compiler-needs-design.md`
    /// §4.2). `false` past the end.
    pub fn plain_decimal(&self, index: usize) -> bool {
        self.plain_decimal.get(index).copied().unwrap_or(false)
    }

    /// Whether a line break comes before the token at `index`. `false` past
    /// the end.
    pub fn line_break_before(&self, index: usize) -> bool {
        self.line_break_before.get(index).copied().unwrap_or(false)
    }

    /// The tokens alone.
    pub fn into_vec(self) -> Vec<Spanned<Token>> {
        self.tokens
    }
}

impl Deref for Tokens {
    type Target = [Spanned<Token>];

    fn deref(&self) -> &[Spanned<Token>] {
        &self.tokens
    }
}

impl<'a> IntoIterator for &'a Tokens {
    type Item = &'a Spanned<Token>;
    type IntoIter = std::slice::Iter<'a, Spanned<Token>>;

    fn into_iter(self) -> Self::IntoIter {
        self.tokens.iter()
    }
}

impl IntoIterator for Tokens {
    type Item = Spanned<Token>;
    type IntoIter = std::vec::IntoIter<Spanned<Token>>;

    fn into_iter(self) -> Self::IntoIter {
        self.tokens.into_iter()
    }
}
