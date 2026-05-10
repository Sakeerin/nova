# 10 — Lexer Specification

> Crate: `nova-lexer`
> Phase: 0
> Depends on: `nova-diagnostics`

---

## 1. Goals

- Convert `&str` source to `Vec<Spanned<Token>>` or iterator
- Preserve span info (byte offsets) for every token
- Recover from errors (don't stop at first bad char)
- UTF-8 aware
- Fast: > 50 MB/s on hot paths (use `logos` if performance matters; `chumsky` lex if simpler)

**Decision:** Use `logos` for the lexer. Chumsky for parser only. `logos` is ~2-3x faster and has explicit token regex patterns.

---

## 2. Token Set

Implement these as `enum Token` with `#[derive(Logos)]`:

### 2.1 Keywords
```
let mut const fn return if else while for in break continue
match type record trait impl import module pub
async await async_fn extern unsafe true false
as is where with self Self
```

### 2.2 Punctuation
```
( ) [ ] { }
, ; : :: -> => . .. ..= @ ?
= == != < <= > >= && || !
+ - * / % += -= *= /= %=
& | ^ ~ << >> &= |= ^= <<= >>=
| (pipe in patterns)
```

### 2.3 Literals
```
INT_LIT       — 0, 42, 1_000_000, 0xff, 0b1010, 0o755
FLOAT_LIT     — 1.5, 1e10, 1.5e-3, 1_000.5
STRING_LIT    — "hello", "with \n escapes", "with ${interp}"
CHAR_LIT      — 'a', '\n', '\u{1F600}'
RAW_STRING    — r"raw\nstring", r#"with "quotes""#
```

### 2.4 Identifiers
```
IDENT         — [a-zA-Z_][a-zA-Z0-9_]*
DOC_COMMENT   — /// rest of line, captured as token (passed to parser)
```

Comments (`//`, `/* */`) are skipped, NOT emitted.

---

## 3. String Interpolation Handling

`"Hello, ${name}!"` is tricky. Two approaches:

**Approach A (chosen):** Lexer emits compound tokens:
```
STR_START("Hello, ")
INTERP_OPEN
IDENT(name)
INTERP_CLOSE
STR_PART("!")
STR_END
```

The parser then assembles these into a `StringLiteral { parts: Vec<StringPart> }` AST node.

This requires the lexer to track a stack:
- When inside `"..."`, scan until `${` or `"`
- On `${`, push state, switch to expression-lexing mode
- On matching `}`, pop state, resume string-lexing

Implement via `logos` callbacks + manual state machine wrapper in `Lexer::next_token()`.

---

## 4. Span Type

```rust
// in nova-diagnostics
pub struct Span {
    pub start: u32,  // byte offset
    pub end: u32,
    pub file: FileId,
}

pub struct Spanned<T> {
    pub value: T,
    pub span: Span,
}
```

Use `u32` not `usize` to keep tokens compact.

---

## 5. Public API

```rust
pub struct Lexer<'src> { /* ... */ }

impl<'src> Lexer<'src> {
    pub fn new(source: &'src str, file: FileId) -> Self;
    pub fn tokenize(&mut self) -> (Vec<Spanned<Token>>, Vec<LexError>);
}

pub fn lex(source: &str, file: FileId) -> (Vec<Spanned<Token>>, Vec<LexError>);
```

Errors are non-fatal — collect and continue.

---

## 6. Error Cases

Each becomes a `LexError` with span:
- `UnterminatedString`
- `UnterminatedBlockComment`
- `InvalidEscapeSequence(char)`
- `InvalidUnicodeEscape`
- `InvalidNumberLiteral`
- `UnexpectedCharacter(char)`

---

## 7. Tests Required

In `crates/nova-lexer/tests/`:

1. **Snapshot tests** (`insta`) for token streams of:
   - All keywords
   - Each operator
   - Each literal kind (including edge cases: `0`, `0.0`, `0xff`, `1e10`, `1_000`)
   - String with all escape sequences
   - Interpolated string nested 2 levels: `"a${b("c${d}")}e"`
   - Raw strings with various `#` counts

2. **Property tests** (`proptest`):
   - Lex never panics on arbitrary UTF-8 input
   - Roundtrip: `tokens.iter().map(|t| t.text()).collect::<String>() == original` (when no whitespace involved)

3. **Fuzz target** in `fuzz/fuzz_targets/lex.rs`: feed random bytes, must not panic.

4. **Snapshot of error messages** for each error case.

---

## 8. Performance Target

- Lex `1MB` of representative Nova source in `< 30ms` on M2-class hardware
- Add a Criterion benchmark in `crates/nova-lexer/benches/lex.rs`

---

## 9. Reference Implementation Sketch

```rust
use logos::Logos;

#[derive(Logos, Debug, Clone, PartialEq)]
#[logos(skip r"[ \t\n\r]+")]
#[logos(skip r"//[^\n]*")]
pub enum Token {
    // Keywords
    #[token("let")] Let,
    #[token("mut")] Mut,
    #[token("fn")] Fn,
    // ... etc

    // Punctuation
    #[token("(")] LParen,
    #[token(")")] RParen,
    #[token("->")] Arrow,
    #[token("=>")] FatArrow,
    // ... etc

    // Literals — callbacks to parse value
    #[regex(r"[0-9][0-9_]*", parse_int)]
    Int(i64),

    #[regex(r"[a-zA-Z_][a-zA-Z0-9_]*", |lex| lex.slice().to_owned())]
    Ident(String),

    // String handled separately (state machine wrapper)
    StringStart,
    StringPart(String),
    StringEnd,
    InterpOpen,
    InterpClose,

    Error,
}

fn parse_int(lex: &mut logos::Lexer<Token>) -> Option<i64> {
    let s = lex.slice().replace('_', "");
    s.parse().ok()
}
```

Then wrap with `Lexer` struct that handles the string state machine and produces `Spanned<Token>`.
