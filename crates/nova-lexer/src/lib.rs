//! Lexer for the Nova programming language.
//!
//! Converts Nova source text into a `Vec<Spanned<Token>>`. Uses `logos` for
//! the core token recognition and a hand-written state machine wrapper for
//! string interpolation (`"Hello, ${name}!"`).
//!
//! # Usage
//!
//! ```
//! use nova_diagnostics::{FileDb, FileId};
//! use nova_lexer::lex;
//!
//! let mut db = FileDb::new();
//! let file = db.add("<test>", r#"fn main() { println("hi") }"#);
//! let (tokens, errors) = lex(r#"fn main() { println("hi") }"#, file);
//! assert!(errors.is_empty());
//! ```

mod error;
mod token;

pub use error::LexError;
pub use token::Token;

use nova_diagnostics::{FileId, Span, Spanned};

/// Lex `source` and return all tokens and any errors encountered.
///
/// Errors are non-fatal: the lexer continues after a bad character and collects
/// all errors in the returned `Vec`.
pub fn lex(source: &str, file: FileId) -> (Vec<Spanned<Token>>, Vec<LexError>) {
    let mut lexer = Lexer::new(source, file);
    lexer.tokenize()
}

/// Stateful lexer that wraps the `logos`-generated token recognizer and adds
/// string-interpolation tracking.
pub struct Lexer<'src> {
    source: &'src str,
    file: FileId,
    // Current byte position in source.
    pos: usize,
    // Stack depth of `${...}` nesting inside strings.
    interp_depth: usize,
    // Whether we are currently inside a string literal (between delimiters).
    in_string: bool,
}

impl<'src> Lexer<'src> {
    pub fn new(source: &'src str, file: FileId) -> Self {
        Self {
            source,
            file,
            pos: 0,
            interp_depth: 0,
            in_string: false,
        }
    }

    pub fn tokenize(&mut self) -> (Vec<Spanned<Token>>, Vec<LexError>) {
        let mut tokens = Vec::new();
        let mut errors = Vec::new();

        while self.pos < self.source.len() {
            match self.next_token() {
                Ok(Some(tok)) => tokens.push(tok),
                Ok(None) => {}
                Err(e) => errors.push(e),
            }
        }

        tokens.push(Spanned::new(
            Token::Eof,
            Span::point(self.source.len() as u32, self.file),
        ));

        (tokens, errors)
    }

    fn span(&self, start: usize, end: usize) -> Span {
        Span::new(start as u32, end as u32, self.file)
    }

    fn next_token(&mut self) -> Result<Option<Spanned<Token>>, LexError> {
        // Inside a string (including resuming after `${expr}`) whitespace is
        // significant — dispatch to string-content lexing before any skipping.
        if self.in_string {
            if self.pos >= self.source.len() {
                return Ok(None);
            }
            return self.lex_string_content();
        }

        // Skip whitespace
        while self.pos < self.source.len() {
            let ch = self.source.as_bytes()[self.pos];
            if ch == b' ' || ch == b'\t' || ch == b'\n' || ch == b'\r' {
                self.pos += 1;
            } else {
                break;
            }
        }

        if self.pos >= self.source.len() {
            return Ok(None);
        }

        // String literal handling
        if self.source.as_bytes()[self.pos] == b'"' {
            return self.lex_string_start();
        }

        // Check for raw strings
        if self.pos + 1 < self.source.len()
            && self.source.as_bytes()[self.pos] == b'r'
            && (self.source.as_bytes()[self.pos + 1] == b'"'
                || self.source.as_bytes()[self.pos + 1] == b'#')
        {
            return self.lex_raw_string();
        }

        // Use the logos-generated lexer for everything else
        self.lex_logos()
    }

    fn lex_string_start(&mut self) -> Result<Option<Spanned<Token>>, LexError> {
        let start = self.pos;
        self.pos += 1; // skip opening `"`
        self.in_string = true;
        // Emit StrStart, then lex content on next call
        Ok(Some(Spanned::new(
            Token::StrStart,
            self.span(start, self.pos),
        )))
    }

    fn lex_string_content(&mut self) -> Result<Option<Spanned<Token>>, LexError> {
        let src = self.source.as_bytes();
        let start = self.pos;

        // Check for end of string
        if src[self.pos] == b'"' {
            self.pos += 1;
            self.in_string = false;
            return Ok(Some(Spanned::new(
                Token::StrEnd,
                self.span(start, self.pos),
            )));
        }

        // Check for interpolation open `${`
        if self.pos + 1 < src.len() && src[self.pos] == b'$' && src[self.pos + 1] == b'{' {
            self.pos += 2;
            self.in_string = false; // switch to expression mode
            self.interp_depth += 1;
            return Ok(Some(Spanned::new(
                Token::InterpOpen,
                self.span(start, self.pos),
            )));
        }

        // Collect a string segment
        let mut buf = String::new();
        while self.pos < src.len() && src[self.pos] != b'"' {
            // Check for interpolation
            if self.pos + 1 < src.len() && src[self.pos] == b'$' && src[self.pos + 1] == b'{' {
                break;
            }
            // Handle escape sequences
            if src[self.pos] == b'\\' {
                self.pos += 1;
                if self.pos >= src.len() {
                    return Err(LexError::UnterminatedString(self.span(start, self.pos)));
                }
                match src[self.pos] {
                    b'n' => {
                        buf.push('\n');
                        self.pos += 1;
                    }
                    b't' => {
                        buf.push('\t');
                        self.pos += 1;
                    }
                    b'r' => {
                        buf.push('\r');
                        self.pos += 1;
                    }
                    b'"' => {
                        buf.push('"');
                        self.pos += 1;
                    }
                    b'\\' => {
                        buf.push('\\');
                        self.pos += 1;
                    }
                    b'0' => {
                        buf.push('\0');
                        self.pos += 1;
                    }
                    b'u' => {
                        self.pos += 1;
                        if self.pos < src.len() && src[self.pos] == b'{' {
                            self.pos += 1;
                            let hex_start = self.pos;
                            while self.pos < src.len() && src[self.pos] != b'}' {
                                self.pos += 1;
                            }
                            let hex = &self.source[hex_start..self.pos];
                            if self.pos < src.len() {
                                self.pos += 1;
                            } // skip `}`
                            match u32::from_str_radix(hex, 16).ok().and_then(char::from_u32) {
                                Some(c) => buf.push(c),
                                None => {
                                    return Err(LexError::InvalidUnicodeEscape(
                                        self.span(start, self.pos),
                                    ))
                                }
                            }
                        } else {
                            return Err(LexError::InvalidEscapeSequence(
                                'u',
                                self.span(start, self.pos),
                            ));
                        }
                    }
                    c => {
                        return Err(LexError::InvalidEscapeSequence(
                            c as char,
                            self.span(start, self.pos),
                        ));
                    }
                }
            } else {
                // Safety: valid UTF-8 since source is &str
                let ch_len = utf8_char_width(src[self.pos]);
                let slice = &self.source[self.pos..self.pos + ch_len];
                buf.push_str(slice);
                self.pos += ch_len;
            }
        }

        if self.pos >= src.len() && !buf.is_empty() {
            return Err(LexError::UnterminatedString(self.span(start, self.pos)));
        }

        let end = self.pos;
        Ok(Some(Spanned::new(
            Token::StrPart(buf),
            self.span(start, end),
        )))
    }

    fn lex_raw_string(&mut self) -> Result<Option<Spanned<Token>>, LexError> {
        let start = self.pos;
        self.pos += 1; // skip `r`
        let mut hash_count = 0usize;
        let src = self.source.as_bytes();

        while self.pos < src.len() && src[self.pos] == b'#' {
            hash_count += 1;
            self.pos += 1;
        }

        if self.pos >= src.len() || src[self.pos] != b'"' {
            return Err(LexError::InvalidNumberLiteral(self.span(start, self.pos)));
        }
        self.pos += 1; // skip `"`

        let mut content = String::new();
        loop {
            if self.pos >= src.len() {
                return Err(LexError::UnterminatedString(self.span(start, self.pos)));
            }
            if src[self.pos] == b'"' {
                // Check for matching `"###...`
                let mut end_hashes = 0;
                let mut look = self.pos + 1;
                while look < src.len() && src[look] == b'#' && end_hashes < hash_count {
                    end_hashes += 1;
                    look += 1;
                }
                if end_hashes == hash_count {
                    self.pos = look;
                    break;
                }
            }
            let ch_len = utf8_char_width(src[self.pos]);
            content.push_str(&self.source[self.pos..self.pos + ch_len]);
            self.pos += ch_len;
        }

        Ok(Some(Spanned::new(
            Token::RawStr(content),
            self.span(start, self.pos),
        )))
    }

    /// Handle `}` that closes a string interpolation.
    fn maybe_close_interp(&mut self, start: usize) -> Option<Spanned<Token>> {
        if self.interp_depth > 0 {
            self.interp_depth -= 1;
            self.in_string = true;
            Some(Spanned::new(Token::InterpClose, self.span(start, self.pos)))
        } else {
            Some(Spanned::new(Token::RBrace, self.span(start, self.pos)))
        }
    }

    fn lex_logos(&mut self) -> Result<Option<Spanned<Token>>, LexError> {
        use logos::Logos;
        // We run logos on the remaining slice.
        let remaining = &self.source[self.pos..];
        let mut lex = RawToken::lexer(remaining);

        match lex.next() {
            None => {
                self.pos = self.source.len();
                Ok(None)
            }
            Some(Ok(raw)) => {
                // `lex.span()` is relative to `remaining` and excludes any
                // trivia logos skipped (comments, whitespace), so the token
                // may not start at `self.pos`. Account for the skipped
                // prefix, otherwise `pos` drifts into the comment text.
                let tok_span = lex.span();
                let tok_start = self.pos + tok_span.start;
                let tok_end = self.pos + tok_span.end;
                self.pos = tok_end;
                let span = self.span(tok_start, tok_end);

                // Special case: `}` might close an interpolation.
                if raw == RawToken::RBrace {
                    return Ok(self.maybe_close_interp(tok_start));
                }

                let tok = raw_to_token(raw, lex.slice());
                Ok(Some(Spanned::new(tok, span)))
            }
            Some(Err(())) => {
                // Skip any trivia before the offending character so the
                // error span points at the character itself.
                let err_offset = lex.span().start;
                let ch = remaining[err_offset..].chars().next().unwrap_or('?');
                let err_start = self.pos + err_offset;
                self.pos = err_start + ch.len_utf8();
                Err(LexError::UnexpectedCharacter(
                    ch,
                    self.span(err_start, self.pos),
                ))
            }
        }
    }
}

fn utf8_char_width(b: u8) -> usize {
    match b {
        0x00..=0x7F => 1,
        0xC0..=0xDF => 2,
        0xE0..=0xEF => 3,
        0xF0..=0xFF => 4,
        _ => 1,
    }
}

/// Convert a `RawToken` (logos output) into a `Token`.
fn raw_to_token(raw: RawToken, slice: &str) -> Token {
    match raw {
        RawToken::Let => Token::Let,
        RawToken::Mut => Token::Mut,
        RawToken::Const => Token::Const,
        RawToken::Fn => Token::Fn,
        RawToken::Return => Token::Return,
        RawToken::If => Token::If,
        RawToken::Else => Token::Else,
        RawToken::While => Token::While,
        RawToken::For => Token::For,
        RawToken::In => Token::In,
        RawToken::Break => Token::Break,
        RawToken::Continue => Token::Continue,
        RawToken::Match => Token::Match,
        RawToken::Type => Token::Type,
        RawToken::Record => Token::Record,
        RawToken::Trait => Token::Trait,
        RawToken::Impl => Token::Impl,
        RawToken::Import => Token::Import,
        RawToken::Module => Token::Module,
        RawToken::Pub => Token::Pub,
        RawToken::Async => Token::Async,
        RawToken::Await => Token::Await,
        RawToken::Extern => Token::Extern,
        RawToken::Unsafe => Token::Unsafe,
        RawToken::True => Token::True,
        RawToken::False => Token::False,
        RawToken::As => Token::As,
        RawToken::Is => Token::Is,
        RawToken::Where => Token::Where,
        RawToken::With => Token::With,
        RawToken::SelfLower => Token::SelfLower,
        RawToken::SelfUpper => Token::SelfUpper,
        RawToken::LParen => Token::LParen,
        RawToken::RParen => Token::RParen,
        RawToken::LBracket => Token::LBracket,
        RawToken::RBracket => Token::RBracket,
        RawToken::LBrace => Token::LBrace,
        RawToken::RBrace => Token::RBrace, // handled before this function for interp
        RawToken::Comma => Token::Comma,
        RawToken::Semicolon => Token::Semicolon,
        RawToken::Colon => Token::Colon,
        RawToken::ColonColon => Token::ColonColon,
        RawToken::Arrow => Token::Arrow,
        RawToken::FatArrow => Token::FatArrow,
        RawToken::Dot => Token::Dot,
        RawToken::DotDot => Token::DotDot,
        RawToken::DotDotEq => Token::DotDotEq,
        RawToken::At => Token::At,
        RawToken::Question => Token::Question,
        RawToken::Eq => Token::Eq,
        RawToken::EqEq => Token::EqEq,
        RawToken::BangEq => Token::BangEq,
        RawToken::Lt => Token::Lt,
        RawToken::LtEq => Token::LtEq,
        RawToken::Gt => Token::Gt,
        RawToken::GtEq => Token::GtEq,
        RawToken::AmpAmp => Token::AmpAmp,
        RawToken::PipePipe => Token::PipePipe,
        RawToken::Bang => Token::Bang,
        RawToken::Plus => Token::Plus,
        RawToken::Minus => Token::Minus,
        RawToken::Star => Token::Star,
        RawToken::Slash => Token::Slash,
        RawToken::Percent => Token::Percent,
        RawToken::PlusEq => Token::PlusEq,
        RawToken::MinusEq => Token::MinusEq,
        RawToken::StarEq => Token::StarEq,
        RawToken::SlashEq => Token::SlashEq,
        RawToken::PercentEq => Token::PercentEq,
        RawToken::Amp => Token::Amp,
        RawToken::Pipe => Token::Pipe,
        RawToken::Caret => Token::Caret,
        RawToken::Tilde => Token::Tilde,
        RawToken::LtLt => Token::LtLt,
        RawToken::GtGt => Token::GtGt,
        RawToken::AmpEq => Token::AmpEq,
        RawToken::PipeEq => Token::PipeEq,
        RawToken::CaretEq => Token::CaretEq,
        RawToken::LtLtEq => Token::LtLtEq,
        RawToken::GtGtEq => Token::GtGtEq,
        RawToken::Float(f) => Token::Float(f),
        RawToken::Int(n) => Token::Int(n),
        RawToken::Char(c) => Token::Char(c),
        RawToken::DocComment => Token::DocComment(slice[3..].trim().to_owned()),
        RawToken::Ident => Token::Ident(slice.to_owned()),
    }
}

// ---------------------------------------------------------------------------
// logos token enum (internal)
// ---------------------------------------------------------------------------

#[derive(logos::Logos, Debug, Clone, PartialEq)]
#[logos(skip r"[ \t\n\r]+")]
#[logos(skip r"//[^\n]*")] // line comment
#[logos(skip r"/\*([^*]|\*[^/])*\*/")] // block comment (non-recursive)
enum RawToken {
    // Keywords (must come before Ident)
    #[token("let")]
    Let,
    #[token("mut")]
    Mut,
    #[token("const")]
    Const,
    #[token("fn")]
    Fn,
    #[token("return")]
    Return,
    #[token("if")]
    If,
    #[token("else")]
    Else,
    #[token("while")]
    While,
    #[token("for")]
    For,
    #[token("in")]
    In,
    #[token("break")]
    Break,
    #[token("continue")]
    Continue,
    #[token("match")]
    Match,
    #[token("type")]
    Type,
    #[token("record")]
    Record,
    #[token("trait")]
    Trait,
    #[token("impl")]
    Impl,
    #[token("import")]
    Import,
    #[token("module")]
    Module,
    #[token("pub")]
    Pub,
    #[token("async")]
    Async,
    #[token("await")]
    Await,
    #[token("extern")]
    Extern,
    #[token("unsafe")]
    Unsafe,
    #[token("true")]
    True,
    #[token("false")]
    False,
    #[token("as")]
    As,
    #[token("is")]
    Is,
    #[token("where")]
    Where,
    #[token("with")]
    With,
    #[token("self")]
    SelfLower,
    #[token("Self")]
    SelfUpper,

    // Multi-char punctuation (longest match first)
    #[token("::")]
    ColonColon,
    #[token("->")]
    Arrow,
    #[token("=>")]
    FatArrow,
    #[token("..")]
    DotDot,
    #[token("..=")]
    DotDotEq,
    #[token("==")]
    EqEq,
    #[token("!=")]
    BangEq,
    #[token("<=")]
    LtEq,
    #[token(">=")]
    GtEq,
    #[token("&&")]
    AmpAmp,
    #[token("||")]
    PipePipe,
    #[token("+=")]
    PlusEq,
    #[token("-=")]
    MinusEq,
    #[token("*=")]
    StarEq,
    #[token("/=")]
    SlashEq,
    #[token("%=")]
    PercentEq,
    #[token("&=")]
    AmpEq,
    #[token("|=")]
    PipeEq,
    #[token("^=")]
    CaretEq,
    #[token("<<=")]
    LtLtEq,
    #[token(">>=")]
    GtGtEq,
    #[token("<<")]
    LtLt,
    #[token(">>")]
    GtGt,

    // Single-char punctuation
    #[token("(")]
    LParen,
    #[token(")")]
    RParen,
    #[token("[")]
    LBracket,
    #[token("]")]
    RBracket,
    #[token("{")]
    LBrace,
    #[token("}")]
    RBrace,
    #[token(",")]
    Comma,
    #[token(";")]
    Semicolon,
    #[token(":")]
    Colon,
    #[token(".")]
    Dot,
    #[token("@")]
    At,
    #[token("?")]
    Question,
    #[token("=")]
    Eq,
    #[token("<")]
    Lt,
    #[token(">")]
    Gt,
    #[token("!")]
    Bang,
    #[token("+")]
    Plus,
    #[token("-")]
    Minus,
    #[token("*")]
    Star,
    #[token("/")]
    Slash,
    #[token("%")]
    Percent,
    #[token("&")]
    Amp,
    #[token("|")]
    Pipe,
    #[token("^")]
    Caret,
    #[token("~")]
    Tilde,

    // Float literals — must be before Int to get longest match priority
    #[regex(r"[0-9][0-9_]*\.[0-9][0-9_]*([eE][+-]?[0-9]+)?", lex_float)]
    #[regex(r"[0-9][0-9_]*[eE][+-]?[0-9]+", lex_float)]
    Float(f64),

    // Integer literals
    #[regex(r"0x[0-9a-fA-F][0-9a-fA-F_]*", lex_hex_int)]
    #[regex(r"0b[01][01_]*", lex_bin_int)]
    #[regex(r"0o[0-7][0-7_]*", lex_oct_int)]
    #[regex(r"[0-9][0-9_]*", lex_dec_int)]
    Int(i64),

    // Char literals
    #[regex(r"'([^'\\]|\\.)'", lex_char)]
    Char(char),

    // Doc comments (/// ...)
    #[regex(r"///[^\n]*")]
    DocComment,

    // Identifiers (after keywords)
    #[regex(r"[a-zA-Z_][a-zA-Z0-9_]*")]
    Ident,
}

fn lex_hex_int(lex: &mut logos::Lexer<RawToken>) -> Option<i64> {
    let s = lex.slice()[2..].replace('_', "");
    i64::from_str_radix(&s, 16).ok()
}

fn lex_bin_int(lex: &mut logos::Lexer<RawToken>) -> Option<i64> {
    let s = lex.slice()[2..].replace('_', "");
    i64::from_str_radix(&s, 2).ok()
}

fn lex_oct_int(lex: &mut logos::Lexer<RawToken>) -> Option<i64> {
    let s = lex.slice()[2..].replace('_', "");
    i64::from_str_radix(&s, 8).ok()
}

fn lex_dec_int(lex: &mut logos::Lexer<RawToken>) -> Option<i64> {
    lex.slice().replace('_', "").parse().ok()
}

fn lex_float(lex: &mut logos::Lexer<RawToken>) -> Option<f64> {
    lex.slice().replace('_', "").parse().ok()
}

fn lex_char(lex: &mut logos::Lexer<RawToken>) -> Option<char> {
    let s = lex.slice();
    // strip surrounding `'`
    let inner = &s[1..s.len() - 1];
    if inner.starts_with('\\') {
        match inner.as_bytes().get(1)? {
            b'n' => Some('\n'),
            b't' => Some('\t'),
            b'r' => Some('\r'),
            b'\\' => Some('\\'),
            b'\'' => Some('\''),
            b'0' => Some('\0'),
            _ => None,
        }
    } else {
        inner.chars().next()
    }
}
