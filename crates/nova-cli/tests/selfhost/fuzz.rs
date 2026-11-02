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
//! 3. [`deep`]: a program nested in one way, or in several mixed, to a
//!    given depth around the parser's limit of 128 levels (spec §3.5 item
//!    3, §4.4; ADR 0052): well typed up to the limit, so that `nova check`
//!    goes through every stage on it, and P0002 past it.
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
    // The lexer's tests' inline sources that a file under `cases/` cannot
    // hold, for a `\r` or another control character (ADR 0052).
    out.extend(
        [
            // lexer_tests.rs: `a_line_comments_span_stops_before_a_crlf`
            "// a\r\nfn main() {}\r\n",
            // lexer_tests.rs: `a_lone_carriage_return_is_a_line_break`
            "b\r(d)",
            // lexer_tests.rs: `a_token_after_a_line_break_is_marked`
            "a\n(b)\r\n  - c",
            // lexer_tests.rs: `doc_text_keeps_its_indentation_and_drops_trailing_whitespace`
            "///   indented  \r\nfn f() {}",
            // lexer_tests.rs: `the_token_dump_writes_every_field`
            "let s = \"a\x01\x7f\\\"\\\\é\" r#\"q\\\"# // c\n/* b */ 07 1_0 0x1 12 2.5 'é' t.1.2 \x02 \"\\q\"\n/// doc \nfn 0.1",
            // lexer_tests.rs: `the_token_dump_of_nothing_is_eof`
            "\r\n1",
        ]
        .map(String::from),
    );
    // Decimal numbers of 4,000 characters, plain, with `_` and before
    // `.5`: `logos` recurses once per character on them, which overflowed
    // S0's 1 MB main thread on Windows until its pipeline moved to a
    // thread of its own (ADR 0051's known bug; ADR 0052).
    out.push(format!("let a = {}\n", "1".repeat(4000)));
    out.push(format!("let a = {}1\n", "1_".repeat(2000)));
    out.push(format!("let a = {}.5\n", "1".repeat(4000)));
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

/// The kinds of nesting [`deep`] writes: the spec's parentheses, blocks,
/// `if`, `match`, calls, types and patterns, and its `else if` chains,
/// binary chains and prefix operators, with `while`, `for`, postfix chains
/// and items in blocks; and all of them mixed.
pub const DEEP_KINDS: [&str; 15] = [
    "parentheses",
    "blocks",
    "ifs",
    "else-ifs",
    "matches",
    "whiles",
    "fors",
    "calls",
    "prefix",
    "binary",
    "postfix",
    "types",
    "patterns",
    "items",
    "mixed",
];

/// The depths [`deeps`] writes each kind at: around the limit, 128.
pub const DEEP_DEPTHS: std::ops::RangeInclusive<usize> = 126..=130;

/// A program whose deepest level, as the parser counts them, is `depth`
/// (at least 2), nested as `kind` names ([`DEEP_KINDS`]). The outermost
/// expression, type or pattern is level 0, and each wrapper one more, so
/// `depth` is how many wrappers there are: 128 parentheses parse, 129 are
/// P0002.
/// Up to the limit, each is well typed but `items`, whose nested functions
/// are E0900; `rng` picks the wrappers of `mixed`.
pub fn deep(rng: &mut SplitMix64, kind: &str, depth: usize) -> String {
    let d = depth;
    let wrap = |open: &str, inner: &str, close: &str, n: usize| {
        format!("{}{inner}{}", open.repeat(n), close.repeat(n))
    };
    let main = |body: String| format!("fn main() {{\n    {body}\n}}\n");
    let value = |x: String| main(format!("let x = {x}\n    println(\"${{x}}\")"));
    match kind {
        "parentheses" => value(wrap("(", "1", ")", d)),
        "blocks" => value(wrap("{ ", "1", " }", d)),
        "ifs" => value(wrap("if true { ", "1", " } else { 0 }", d)),
        "else-ifs" => value(format!(
            "if false {{ 0 }}{} else {{ 1 }}",
            " else if false { 0 }".repeat(d - 1)
        )),
        "matches" => value(wrap("match 0 { _ => ", "1", " }", d)),
        "whiles" => main(wrap("while false { ", "", " }", d).replace("{  }", "{}")),
        "fors" => main(format!(
            "let xs = [1]\n    {}",
            wrap("for x in xs { ", "", " }", d).replace("{  }", "{}")
        )),
        "calls" => format!(
            "fn f(x: Int) -> Int {{ x }}\n\n{}",
            value(wrap("f(", "1", ")", d))
        ),
        "prefix" => value(format!("{}true", "!".repeat(d))),
        "binary" => value(format!("{}1", "1 + ".repeat(d))),
        // `d` method calls, then a field: the chain's first operator is
        // not a level, each after it one more, a method call one operator.
        "postfix" => format!(
            "record R {{ x: Int }}\n\nimpl R {{\n    fn f(self) -> R {{ self }}\n}}\n\n{}",
            value(format!("R {{ x: 1 }}{}.x", ".f()".repeat(d)))
        ),
        "types" => format!(
            "fn f(x: {}) {{}}\n\nfn main() {{}}\n",
            wrap("Option<", "Int", ">", d)
        ),
        "patterns" => main(format!(
            "let {} = 1\n    println(\"${{x}}\")",
            wrap("(", "x", ")", d)
        )),
        "items" => main(wrap("fn g() { ", "()", " }", d)),
        "mixed" => {
            // Each wrapper one level, every one an `Int`. A prefix `-`
            // binds tighter than `+`, so `1 + ` never comes straight
            // inside one.
            const WRAPPERS: [(&str, &str); 8] = [
                ("(", ")"),
                ("{ ", " }"),
                ("if true { ", " } else { 0 }"),
                ("match 0 { _ => ", " }"),
                ("f(", ")"),
                ("1 + ", ""),
                ("[", "][0]"),
                ("- ", ""),
            ];
            let (mut open, mut close) = (String::new(), String::new());
            let mut last = 0;
            for _ in 0..d {
                let mut w = rng.below(WRAPPERS.len());
                if last == 7 && w == 5 {
                    w = 0;
                }
                last = w;
                open.push_str(WRAPPERS[w].0);
                close.insert_str(0, WRAPPERS[w].1);
            }
            format!(
                "fn f(x: Int) -> Int {{ x }}\n\n{}",
                value(format!("{open}1{close}"))
            )
        }
        other => panic!("no kind of nesting {other}"),
    }
}

/// Every kind but `mixed` at each of [`DEEP_DEPTHS`], then `mixed`
/// `count` times, at depths `seed` picks among them.
pub fn deeps(seed: u64, mixed: usize) -> Vec<(String, usize, String)> {
    let mut rng = SplitMix64::new(seed);
    let mut out = Vec::new();
    for kind in DEEP_KINDS.iter().filter(|k| **k != "mixed") {
        for depth in DEEP_DEPTHS {
            out.push((kind.to_string(), depth, deep(&mut rng, kind, depth)));
        }
    }
    let depths: Vec<usize> = DEEP_DEPTHS.collect();
    for _ in 0..mixed {
        let depth = *rng.pick(&depths);
        out.push(("mixed".to_string(), depth, deep(&mut rng, "mixed", depth)));
    }
    out
}
