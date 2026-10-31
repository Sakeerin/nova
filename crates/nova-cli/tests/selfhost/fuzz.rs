//! The differential fuzzer's inputs (spec
//! `docs/superpowers/specs/2026-10-30-phase-5-1-the-lexer-the-parser-and-the-ast-design.md`
//! §3.5, ADR 0051): a splitmix64 PRNG, so that no crate is needed and a
//! seed names the same soup on every system, and the generators. A seed's
//! mutations are the same on every system too for the same corpus files,
//! which `selfhost.rs` picks from their sorted list and reads with each
//! `\r\n` as `\n`; an untracked local `.nova` file joins the corpus, and
//! changes them.
//!
//! 1. [`soup`]: lexical soup, as the 5.0 spike's `gen.py` made it, with
//!    more atoms and an unterminated tail now and then, weighted so that
//!    all seven lex errors occur among the fixed seeds' inputs.
//! 2. [`mutate`]: a corpus file, lexed by S0's lexer into its tokens'
//!    spans, then edited a few times: a whole token deleted, duplicated,
//!    swapped with another or inserted elsewhere, or one ASCII byte
//!    replaced. Most of the result stays well formed, so a lexer, and in
//!    5.1b a parser, gets far into it.
//! 3. Deep nesting is 5.1b's (spec §3.5 item 3, §4.4): a `deep(rng, kind,
//!    depth) -> String` here, beside these two, whose inputs `selfhost.rs`
//!    hands to `agree_on_inputs` as it does theirs.
//!
//! Beside them, [`edges`]: fixed inputs that no file under `cases/` can
//! hold as it is, such as a lone `\r` or a NUL.

use nova_diagnostics::FileId;

/// splitmix64 (Steele, Lea and Flood, 2014): one `u64` of state, and an
/// output that passes BigCrush, which is plenty for choosing atoms.
pub struct SplitMix64(u64);

impl SplitMix64 {
    pub fn new(seed: u64) -> Self {
        Self(seed)
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// A number in `0..n`; `n` must not be 0.
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }

    /// `true` once in `n` times, on average.
    pub fn one_in(&mut self, n: usize) -> bool {
        self.below(n) == 0
    }

    pub fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[self.below(items.len())]
    }
}

/// A backslash, so that no escape spelled with one is written in this file
/// as a Rust escape by mistake.
const BS: &str = "\\";

/// The soup's atoms: `gen.py`'s, then the ones 5.1a adds.
fn atoms() -> Vec<String> {
    let mut atoms: Vec<String> = "\"${}'r# \n\t/*.eExXbBoO_u+-=<>&|!^%~@?:;,()[]0123456789aZ"
        .chars()
        .map(String::from)
        .collect();
    atoms.push(BS.to_string());
    let words: &[&str] = &[
        // gen.py's
        "é",
        "€",
        "😀",
        "\r",
        "\0",
        "\x7f",
        "`",
        "0x",
        "0b",
        "0o",
        "1e",
        "1.5",
        "1..2",
        "0x7FFFFFFFFFFFFFFF",
        "9223372036854775807",
        "9223372036854775808",
        "0xFFFFFFFFFFFFFFFFF",
        "///",
        "////",
        "/*",
        "*/",
        "//",
        "r#\"",
        "\"#",
        "r\"",
        "'é'",
        "let",
        "fn",
        "Self",
        "self",
        "match",
        "${",
        "\"a\"",
        "1_000",
        "1e+5",
        "1.5e-3",
        "00.5",
        "007",
        "1_.5",
        "1._5",
        "0.1e",
        "..=",
        "<<=",
        ">>=",
        "\"${f(R { v: 1 })}\"",
        "{",
        "}",
        // 5.1a's: line endings, numbers, tuple indexes, raw strings, holes
        "\r\n",
        "\r\n",
        "0_81.5",
        "t.1.2",
        ".0.1",
        "x.007.5",
        "t.1.99999999999999999999",
        "1e400",
        "1_0.5",
        ".5",
        "1.0e10",
        "0b2",
        "0o9",
        "0x_1",
        "99999999999999999999",
        "0b11111111111111111111111111111111111111111111111111111111111111111",
        "0o7777777777777777777777",
        "r##\"",
        "\"##",
        "r#x",
        "r#",
        "r##",
        "\"${\"${",
        "\"${x}\"",
        "'''",
        "'a",
        "async",
        "await",
        "record",
        "where",
        "with",
        "is",
        "/// doc ",
        "/* a\nb */",
        // A Windows checkout's line endings inside every kind of token.
        "\"a\r\nb\rc\"",
        "r#\"a\r\nb\r\"#",
        "/// doc\r",
        "/// doc \r\r\n",
        "// c\r",
        "/* \r\n */",
        "'\r'",
    ];
    atoms.extend(words.iter().map(|s| s.to_string()));
    // Escapes, built from `BS`.
    for tail in [
        "u{",
        "u{41}",
        "u{D800}",
        "u{110000}",
        "u{+41}",
        "u{}",
        "u{10FFFF}",
        "u{0000000041}",
        "u{-1}",
        "u{fffffffff}",
        "u",
        "0",
        "\"",
        "é",
        "q",
        "n",
        "$",
        "\n",
    ] {
        atoms.push(format!("{BS}{tail}"));
    }
    atoms.push(BS.repeat(2));
    for tail in ["n", "x", "q", "'", "u{41}", "\n"] {
        atoms.push(format!("'{BS}{tail}'"));
    }
    // Characters that are whitespace to Unicode but not to Nova: no-break
    // space, ideographic space, next line, line separator, and a byte order
    // mark.
    for code in [0xA0, 0x3000, 0x85, 0x2028, 0xFEFF] {
        let c = char::from_u32(code).expect("a character");
        atoms.push(c.to_string());
        atoms.push(format!("///x{c}"));
    }
    atoms
}

/// Endings that leave something open at the end of the file: what
/// `UnterminatedInterpolation`, `UnterminatedBlockComment`,
/// `UnterminatedString` and `InvalidNumberLiteral` need, and random soup
/// rarely makes.
const TAILS: [&str; 12] = [
    "\"${",
    "\"${ x",
    "\"a${\"${",
    "\"${ { }",
    "/* never closed",
    "\"open",
    "r#\"open",
    "r#",
    "'",
    "\"esc\\",
    "// a line comment ending in a carriage return\r",
    "/// a doc comment ending in a carriage return\r",
];

/// One input of lexical soup: up to 200 atoms, and once in five an
/// unterminated tail.
pub fn soup(rng: &mut SplitMix64, atoms: &[String]) -> String {
    let mut out = String::new();
    for _ in 0..rng.below(201) {
        out.push_str(rng.pick(atoms));
    }
    if rng.one_in(5) {
        out.push_str(rng.pick(&TAILS));
    }
    out
}

/// `count` inputs of soup from `seed`.
pub fn soups(seed: u64, count: usize) -> Vec<String> {
    let atoms = atoms();
    let mut rng = SplitMix64::new(seed);
    (0..count).map(|_| soup(&mut rng, &atoms)).collect()
}

/// Bytes a byte flip writes: ones that open, close or change a token.
const FLIPS: &[u8] = b"\"'{}$\\/*\nr#0123456789._eEx+-=<>()[];,: \t\r\0";

/// Tokens an insertion can write when the file has none of its own.
const SPARE: [&str; 8] = ["(", "}", "\"", "${", "let", "0x", "1.5", "/*"];

/// `source`, edited by token and by byte (module docs).
pub fn mutate(rng: &mut SplitMix64, source: &str) -> String {
    let (tokens, _) = nova_lexer::lex(source, FileId::DUMMY);
    // Each token, but `Eof`, with the text before it; then the text after
    // the last one.
    let mut pieces: Vec<(String, String)> = Vec::new();
    let mut prev = 0usize;
    for t in tokens.iter() {
        if t.value == nova_lexer::Token::Eof {
            break;
        }
        let (start, end) = (t.span.start as usize, t.span.end as usize);
        let start = start.max(prev);
        let end = end.max(start);
        pieces.push((
            source[prev..start].to_string(),
            source[start..end].to_string(),
        ));
        prev = end;
    }
    let tail = source[prev..].to_string();

    for _ in 0..1 + rng.below(8) {
        let n = pieces.len();
        match rng.below(5) {
            0 if n > 0 => {
                // Delete a token, keeping the space before it.
                let i = rng.below(n);
                pieces[i].1.clear();
            }
            1 if n > 0 => {
                let i = rng.below(n);
                let copy = (" ".to_string(), pieces[i].1.clone());
                pieces.insert(i + 1, copy);
            }
            2 if n > 1 => {
                // Swap two tokens' texts; a token swapped with itself stays.
                let (i, j) = (rng.below(n), rng.below(n));
                let text = pieces[i].1.clone();
                pieces[i].1 = std::mem::replace(&mut pieces[j].1, text);
            }
            3 => {
                let text = if n > 0 {
                    pieces[rng.below(n)].1.clone()
                } else {
                    rng.pick(&SPARE).to_string()
                };
                let gap = if rng.one_in(2) { "" } else { " " };
                pieces.insert(rng.below(n + 1), (gap.to_string(), text));
            }
            _ => {}
        }
    }

    let mut out = String::with_capacity(source.len() + 64);
    for (gap, text) in &pieces {
        out.push_str(gap);
        out.push_str(text);
    }
    out.push_str(&tail);

    // A few single-byte replacements, of ASCII bytes only, so the text stays
    // UTF-8.
    let mut bytes = out.into_bytes();
    if !bytes.is_empty() {
        for _ in 0..rng.below(4) {
            let at = rng.below(bytes.len());
            if bytes[at].is_ascii() {
                bytes[at] = *rng.pick(FLIPS);
            }
        }
    }
    String::from_utf8(bytes).expect("only ASCII bytes were replaced")
}

/// The edge inputs: hand-made, each a place where the two lexers could
/// part, made here rather than kept under `cases/` because a file there
/// could not hold it as it is: a checkout on Windows turns `\n` into
/// `\r\n`, and the repository's text holds no line ending in `\r`, no NUL
/// and no backslash-u escape.
pub fn edges() -> Vec<String> {
    let c = |code: u32| char::from_u32(code).expect("a character").to_string();
    let mut out: Vec<String> = [
        // No text, a byte order mark alone and before code, no final line
        // break.
        "",
        "let a = 1",
        // A lone `\r` and `\r\n` between tokens, in strings, raw strings,
        // holes, chars, block comments, and ending doc and line comments,
        // the last at the end of the file.
        "let a = 1\rlet b = 2\r",
        "let a = 1\r\nlet b = 2\r\n",
        "\r",
        "\r\n\r\n",
        "let s = \"a\r\nb\rc\"\r\n",
        "let s = r#\"a\r\nb\r\"#\r\n",
        "\"${\r\nx\r}\"",
        "'\r' '\n'",
        "/* a\r\nb */ x\r",
        "/// doc\r\nfn f() {}\r\n/// tail \r\r\n///\r\n",
        "/// doc\r",
        "let a = 1 // end\r",
        "// x\r\r",
        "x //\r",
        // NUL, outside and inside every kind of token.
        "let a\0 = \"x\0y\" // c\0\n/* \0 */ 'a\0 '\0' r\"\0\" /// \0\n",
        // Endings that leave something open at the last byte.
        "0x",
        "0b",
        "0o",
        "0x1",
        "1e",
        "1e+",
        "1.",
        "1.5e",
        "'",
        "'a",
        "'\\",
        "r",
        "r#",
        "r##",
        "\"",
        "\"$",
        "\"${",
        "\"${ {",
        "/*",
        "/",
        "//",
        "///",
        "\"esc\\",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    out.push(c(0xFEFF));
    out.push(format!("{}let a = 1\n", c(0xFEFF)));
    // Every escape, then each kind of Unicode escape, built from `BS`: the
    // largest, past it, the surrogates, a leading `+` and `-`, empty, past
    // `u32`, seventeen digits whose value wraps an `Int` into a character,
    // and a `"` inside the braces.
    out.push(format!(
        "\"{BS}n{BS}t{BS}r{BS}\"{BS}{BS}{BS}0{BS}q{BS}$\" '{BS}n' '{BS}t' '{BS}r' '{BS}{BS}' '{BS}'' '{BS}0' '{BS}\"'"
    ));
    for hex in [
        "10FFFF",
        "110000",
        "D800",
        "DFFF",
        "0",
        "+10",
        "-1",
        "",
        "ffffffff",
        "100000000",
        "10000000000000041",
        "e9\"x",
        "41",
    ] {
        out.push(format!("\"{BS}u{{{hex}}}\" '{BS}u{{{hex}}}'"));
    }
    // A doc comment ending in each character Rust's `trim_end` drops,
    // Unicode's White_Space, but `\n`, which ends the comment, and `\r`,
    // above; then in five it keeps: U+180E, U+200B, U+FEFF, DEL and U+0001.
    let white_space = [0x09, 0x0B, 0x0C, 0x20, 0x85, 0xA0, 0x1680]
        .into_iter()
        .chain(0x2000..=0x200A)
        .chain([0x2028, 0x2029, 0x202F, 0x205F, 0x3000]);
    for code in white_space.chain([0x180E, 0x200B, 0xFEFF, 0x7F, 0x01]) {
        out.push(format!("/// d{}\n///{}", c(code), c(code)));
    }
    out
}
