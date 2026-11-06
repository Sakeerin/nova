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
//! 4. [`name_mutate`] (5.2a, spec
//!    `docs/superpowers/specs/2026-11-02-phase-5-2-the-loader-and-the-resolver-design.md`
//!    §3.5): a loose corpus program that parses, mutated at the level of
//!    names, beside four sibling modules ([`NAME_SIBLINGS`]), so that the
//!    resolver's codes occur among the fixed seeds' inputs. 5.2b (spec
//!    §4.5, ADR 0054) adds two siblings, each `name_mutate` then one edit
//!    of its own: [`wasm_name_mutate`], for the WASM target's programs, and
//!    [`package_name_mutate`], for a module of the package
//!    [`PACKAGE_FILES`] lays out, with its sibling modules, a dependency,
//!    a dev-dependency and a module named as a dependency.
//!
//! 5. [`type_mutate`] (5.3a, spec
//!    `docs/superpowers/specs/2026-11-06-phase-5-3-the-type-checker-and-hir-design.md`
//!    §3.6): a corpus program that checks at the item level, mutated at
//!    the level of items, so that the item passes' codes occur among the
//!    fixed seeds' inputs: a parameter's, a return's or a field's type, a
//!    duplicated generic, a trait impl's method dropped, added or retyped,
//!    an impl duplicated, and items appended that give each aimed code.
//!    5.3b adds the body mutations.
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
/// binary chains and prefix operators, with `while`, `for`, postfix chains,
/// optional types `T?` and items in blocks; and all of them mixed.
pub const DEEP_KINDS: [&str; 16] = [
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
    "optionals",
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
/// Up to the limit, each is well typed but `items` and `optionals`, whose
/// nested functions and `T?` are E0900; `rng` picks the wrappers of
/// `mixed`.
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
        // `d + 1` `?`s: each after the first is a level, as a postfix
        // chain's operators are.
        "optionals" => format!("fn f(x: Int{}) {{}}\n\nfn main() {{}}\n", "?".repeat(d + 1)),
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

// --- 5.2a: the name-level fuzzer (spec
// `docs/superpowers/specs/2026-11-02-phase-5-2-the-loader-and-the-resolver-design.md`
// §3.5; ADR 0053) ---

/// The modules every name-mutated input sits beside, `(stem, source)`, so
/// that an import of a sibling finds one: `shapes` exports one item of
/// each kind and keeps one private, and `helpers` exports a `main`, which
/// clashes with a program's own when imported whole, and a variant that
/// hides std's `Some` (E0102). `vault` keeps one private item of each kind
/// that a fix can make public, and `hidden` twice, the second with its
/// keyword on a line of its own, so no fix is offered for it; `keeper`
/// imports `vault` whole and declares a record named as `vault`'s private
/// function `Kept`, so the fix that would make `Kept` public is dropped.
pub const NAME_SIBLINGS: [(&str, &str); 4] = [
    (
        "shapes",
        "pub fn area() -> Int { 1 }\nfn secret() -> Int { 2 }\npub record Box { w: Int }\n\
         pub type Tint = | Light | Dark\npub trait Shade {}\npub const LIMIT: Int = 3\n",
    ),
    (
        "helpers",
        "pub fn main() {}\npub fn helper() -> Int { 0 }\npub type Signal = | Some(Int) | Quiet\n",
    ),
    (
        "vault",
        "fn hidden() -> Int { 1 }\nconst\n    hidden: Int = 2\nconst SEALED: Int = 3\n\
         record Cell { n: Int }\ntype Hue = | Pale\ntrait Veiled {}\nfn Kept() -> Int { 4 }\n",
    ),
    ("keeper", "import vault\nrecord Kept { n: Int }\n"),
];

/// A name a renamed definition can take, to collide with a builtin.
const BUILTIN_NAMES: [&str; 5] = ["println", "print", "eprint", "eprintln", "panic"];

/// Names std brings into every module: types, variants, traits and
/// functions, which a program's own definition shadows, or, for a
/// variant, hides with E0102.
const STD_NAMES: [&str; 11] = [
    "Option",
    "Vec",
    "Map",
    "Some",
    "None",
    "Ok",
    "Err",
    "Display",
    "Iterator",
    "read_to_string",
    "sleep",
];

/// The built-in type names (`nova_resolver::RESERVED_TYPE_NAMES`): a
/// record or a sum under one is E0089.
const RESERVED_NAMES: [&str; 7] = ["Int", "Float", "Bool", "Char", "String", "Future", "Bytes"];

/// Names only std's modules see (`Builtin::STD_ONLY`), which a program may
/// define freely.
const STD_ONLY_NAMES: [&str; 5] = [
    "str_cmp",
    "str_hash",
    "char_to_int",
    "fs_read",
    "task_spawn",
];

/// The imports a mutation adds at the top of a file: a module that is not
/// there, a sibling whole or by a list (a private name, a missing one),
/// std's imported modules (`dom` is not native's, E0092), a nested path
/// under one and elsewhere, an alias (E0900), `{self}`, the input's own
/// stem, a self-import, and `vault`'s private items, with `keeper`, which
/// a fix for `Kept` would bind twice.
const NAME_IMPORTS: [&str; 18] = [
    "import nosuch",
    "import shapes",
    "import shapes::{secret}",
    "import shapes::{area, nothing}",
    "import shapes::{Box, Shade, LIMIT, Tint, Light}",
    "import helpers",
    "import helpers::{main}",
    "import dom",
    "import ui",
    "import ui::nope",
    "import ui::html::{div}",
    "import a::b",
    "import shapes as s",
    "import {self}",
    "import helpers::{Signal, Some}",
    "import vault::{hidden}",
    "import vault::{SEALED, Cell, Hue, Veiled}",
    "import vault::{Kept}\nimport keeper",
];

/// The attributes a mutation puts on an item: unknown ones (E0082, with a
/// fix on a function when the name is close to `test`), `@test` (E0083 on
/// anything but a function; a function with one is stripped under
/// `check`), and malformed `@test`s.
const NAME_ATTRIBUTES: [&str; 6] = [
    "@tset",
    "@inline",
    "@test",
    "@test(bogus)",
    "@test(should_panic, nope)",
    "@deprecated(soon)",
];

/// The items a mutation adds at the end of a file: a type alias (E0900), a
/// sum whose variants hide std's (E0102), a trait twice (E0002), an
/// `extern` block, a function under a std-only name, a record under a
/// built-in type's name (E0089), a second `main`, a `@test` record and
/// impl block (E0083), a trait with a misspelt one (E0082), and impls for
/// types that name their methods `anon`.
const NAME_ITEMS: [&str; 11] = [
    "type Alias = Int\n",
    "type Mine = | Some(Int) | Nope | Err\n",
    "trait Twice {}\ntrait Twice {}\n",
    "extern \"C\" {\n    fn abs(x: Int) -> Int\n}\n",
    "fn str_cmp(a: Int) -> Int { a }\n",
    "record Bytes { n: Int }\n",
    "pub fn main() {}\n",
    "@test\nrecord Probe { n: Int }\n",
    "@tset\ntrait Probing {}\n",
    "record Held { n: Int }\n@test\nimpl Held {}\n",
    "trait Marked {}\nimpl Marked for () { fn mark(self) {} }\nimpl Marked for fn(Int) -> Int { fn mark(self) {} }\n",
];

/// The package every [`package_name_mutate`] input is a module of, `(path,
/// text)` under the fuzzer's directory: `app`, whose `src/` and `tests/`
/// hold [`NAME_SIBLINGS`] and the inputs, and `src/` a module named
/// `clash`, as a dependency is (E0004), and a `main.nova`, which the
/// package graph requires; `app` has no `src/lib.nova`, so a `tests/`
/// module's `import app` is E0001 with a note. Its dependencies:
/// `dep`, whose library imports its own `inner` and keeps a private
/// function and a `@test` (stripped under test mode, being a
/// dependency's); `clash`; and the dev-dependency `devdep`, which a `src/`
/// module may not import (E0001 with a note).
pub const PACKAGE_FILES: [(&str, &str); 10] = [
    (
        "app/nova.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2026\"\n\n\
         [dependencies]\ndep = { path = \"../dep\" }\nclash = { path = \"../clash\" }\n\n\
         [dev-dependencies]\ndevdep = { path = \"../devdep\" }\n",
    ),
    ("app/src/main.nova", "fn main() {}\n"),
    ("app/src/clash.nova", "pub fn twin() -> Int { 0 }\n"),
    (
        "dep/nova.toml",
        "[package]\nname = \"dep\"\nversion = \"0.1.0\"\nedition = \"2026\"\n",
    ),
    (
        "dep/src/lib.nova",
        "import inner\npub fn lift() -> Int { ground() }\npub record Gauge { n: Int }\n\
         fn sunk() -> Int { 0 }\n@test\nfn a_dependencys_test() {}\n",
    ),
    ("dep/src/inner.nova", "pub fn ground() -> Int { 0 }\n"),
    (
        "clash/nova.toml",
        "[package]\nname = \"clash\"\nversion = \"0.1.0\"\nedition = \"2026\"\n",
    ),
    ("clash/src/lib.nova", "pub fn twin() -> Int { 1 }\n"),
    (
        "devdep/nova.toml",
        "[package]\nname = \"devdep\"\nversion = \"0.1.0\"\nedition = \"2026\"\n",
    ),
    ("devdep/src/lib.nova", "pub fn expect() -> Int { 2 }\n"),
];

/// The forms [`PACKAGE_FILES`]' manifests are written in, `app`'s and
/// `dep`'s, input `i` of a seed in a package whose manifests are form `i %
/// 4` (5.2b, ADR 0054 §5): each a form N's manifest reader takes and
/// nova-pm reads with no M-code, so that each of the reader's forms meets
/// the loader, E0004's line and span in a manifest included. Form 0 is
/// [`PACKAGE_FILES`]' own. Form 1 has comments, a header with spaces, a
/// name in `'...'`, a pre-release, more keys, the dependency tables in
/// another order and `[package]`'s last value before a comment; form 2 the
/// dependency tables as inline tables before the header, the keys in
/// another order and the largest version; form 3 a byte-order mark and
/// `\r\n`. `dep`'s forms add dev-dependencies, which nova-pm reads and,
/// in a dependency's manifest, never resolves: a version entry, and a
/// path to no package.
pub fn package_manifests() -> [(String, String); 4] {
    let app = PACKAGE_FILES[0].1.to_string();
    let dep = PACKAGE_FILES[3].1.to_string();
    let bom = char::from_u32(0xFEFF)
        .expect("the byte-order mark")
        .to_string();
    [
        (app.clone(), dep.clone()),
        (
            "# The fuzzer's package, written another way.\n\n[ package ]\nname = 'app'   # its name\n\
             version = \"1.2.3-alpha.1\"\ndescription = \"d\"\nauthors = [\"a\", 'b',]\n\
             edition = '2026' # the last value\n\n[dev-dependencies]\ndevdep = {path='../devdep'}\n\n\
             [dependencies]\nclash = { path = \"../clash\" }   # a module's name too\n\
             dep = { path = \"../dep\" }\n"
                .to_string(),
            format!("{dep}\n[dev-dependencies]\njson = \"^1.2\"\n"),
        ),
        (
            "dependencies = { dep = { path = \"../dep\" }, clash = { path = \"../clash\" } }\n\
             dev-dependencies = { devdep = { path = \"../devdep\" } }\n\n[package]\nedition = \"2026\"\n\
             version = \"18446744073709551615.0.0\"\nname = \"app\"\nlicense = \"MIT\"\n"
                .to_string(),
            dep.replace("0.1.0", "0.0.18446744073709551615")
                + "\n[dev-dependencies]\nlocal = { path = \"../nowhere\" }\nreg = { version = \"=1\" }\n",
        ),
        (
            format!("{bom}{}", app.replace('\n', "\r\n")),
            dep.replace('\n', "\r\n"),
        ),
    ]
}

/// What [`wasm_name_mutate`] adds to a WASM program, one of them, after
/// [`name_mutate`]'s mutations: at the top (`true`) an import of `ui::ssr`,
/// which only the native target has (E0092), of `ui::router`, or of
/// `dom`, which only WASM has; or at the end an `extern "C"` or `extern
/// "JS"` block (E0090).
const WASM_ADDITIONS: [(bool, &str); 5] = [
    (true, "import ui::ssr"),
    (true, "import ui::router"),
    (true, "import dom"),
    (false, "extern \"C\" {\n    fn abs(x: Int) -> Int\n}\n"),
    (false, "extern \"JS\" {\n    fn now() -> Float\n}\n"),
];

/// What [`package_name_mutate`] adds to a module of [`PACKAGE_FILES`]'
/// `app`, one of them, at the top, after [`name_mutate`]'s mutations: an
/// import of the dependency whole, by a list and by a private name, of
/// the dev-dependency, of `clash`, a module and a dependency at once
/// (E0004), and of the package itself, which a `tests/` module sees by
/// its name but which has no library.
const PACKAGE_ADDITIONS: [(bool, &str); 6] = [
    (true, "import dep"),
    (true, "import dep::{lift, Gauge}"),
    (true, "import dep::{sunk}"),
    (true, "import devdep"),
    (true, "import clash"),
    (true, "import app"),
];

/// `text` with one of `additions` that the rng picks, at the top or at the
/// end, when the result still parses.
fn add_one(rng: &mut SplitMix64, text: String, additions: &[(bool, &str)]) -> String {
    let (top, addition) = *rng.pick(additions);
    let next = if top {
        format!("{addition}\n{text}")
    } else {
        format!("{}\n{addition}", text.trim_end_matches('\n'))
    };
    if named_items(&next).is_some() {
        next
    } else {
        text
    }
}

/// One top-level item, as the name fuzzer edits it: its span, and its
/// name's span, its name and whether it is `pub` when it has one, with
/// whether it introduces a type name (a record or a sum) and whether it is
/// a function.
struct NamedItem {
    span: (usize, usize),
    name: Option<(usize, usize, String)>,
    is_pub: bool,
    is_type: bool,
    is_fn: bool,
}

/// `source`'s items, if it lexes and parses without an error.
fn named_items(source: &str) -> Option<Vec<NamedItem>> {
    use nova_ast::item::Visibility;
    use nova_ast::Item;
    let (tokens, lex_errors) = nova_lexer::lex(source, FileId::DUMMY);
    if !lex_errors.is_empty() {
        return None;
    }
    let parsed = nova_parser::parse_recovering(&tokens, FileId::DUMMY);
    if !parsed.errors.is_empty() {
        return None;
    }
    let items = parsed
        .file
        .items
        .iter()
        .map(|item| {
            let (name, vis, is_type) = match &item.value {
                Item::Function(f) => (Some(&f.name), Some(f.vis), false),
                Item::Record(r) => (Some(&r.name), Some(r.vis), true),
                Item::Type(t) => (Some(&t.name), Some(t.vis), true),
                Item::Trait(t) => (Some(&t.name), Some(t.vis), false),
                Item::Const(c) => (Some(&c.name), Some(c.vis), false),
                _ => (None, None, false),
            };
            NamedItem {
                span: (item.span.start as usize, item.span.end as usize),
                name: name.map(|n| (n.span.start as usize, n.span.end as usize, n.value.clone())),
                is_pub: vis == Some(Visibility::Pub),
                is_type,
                is_fn: matches!(item.value, Item::Function(_)),
            }
        })
        .collect();
    Some(items)
}

/// Where the line holding byte `at` starts.
fn line_start(source: &str, at: usize) -> usize {
    source[..at].rfind('\n').map_or(0, |i| i + 1)
}

/// `source`, a program that parses, mutated one to three times at the
/// level of names: a definition renamed to collide with another, with a
/// builtin, with a std name, a built-in type's or a std-only builtin's; an
/// item duplicated or deleted; `pub` toggled; an import, an attribute or
/// an item from the tables above added. `stem` is the input's own file
/// stem, which a self-import names. A mutation that would leave the
/// program unparsable is skipped, so every input reaches the resolver.
pub fn name_mutate(rng: &mut SplitMix64, source: &str, stem: &str) -> String {
    let mut text = source.to_string();
    for _ in 0..1 + rng.below(3) {
        let Some(items) = named_items(&text) else {
            break;
        };
        let named: Vec<&NamedItem> = items.iter().filter(|i| i.name.is_some()).collect();
        let edited = match rng.below(28) {
            // Renames: 0-2 to another item's name, 3-4 a builtin's, 5-6 a
            // std name, 7-8 a built-in type's (on a type), 9 a std-only
            // builtin's.
            k @ 0..=9 if !named.is_empty() => {
                let types: Vec<&NamedItem> = named.iter().copied().filter(|i| i.is_type).collect();
                let item = if matches!(k, 7 | 8) && !types.is_empty() {
                    *rng.pick(&types)
                } else {
                    *rng.pick(&named)
                };
                let (start, end, _) = item.name.clone().expect("a named item");
                let to = match k {
                    0..=2 => rng.pick(&named).name.clone().expect("a named item").2,
                    3 | 4 => rng.pick(&BUILTIN_NAMES).to_string(),
                    5 | 6 => rng.pick(&STD_NAMES).to_string(),
                    7 | 8 => rng.pick(&RESERVED_NAMES).to_string(),
                    _ => rng.pick(&STD_ONLY_NAMES).to_string(),
                };
                Some(format!("{}{to}{}", &text[..start], &text[end..]))
            }
            // An item duplicated at the end of the file, or deleted.
            10..=12 if !items.is_empty() => {
                let item = rng.pick(&items);
                let copy = &text[item.span.0..item.span.1];
                Some(format!("{}\n{copy}\n", text.trim_end_matches('\n')))
            }
            13 | 14 if !items.is_empty() => {
                let item = rng.pick(&items);
                Some(format!("{}{}", &text[..item.span.0], &text[item.span.1..]))
            }
            // `pub` toggled.
            15 | 16 if !named.is_empty() => {
                let item = rng.pick(&named);
                let (start, _, _) = item.name.clone().expect("a named item");
                let keyword = nova_diagnostics::lines::keyword_start(&text, start);
                match (item.is_pub, keyword) {
                    (false, Some(at)) => Some(format!("{}pub {}", &text[..at], &text[at..])),
                    (true, Some(at)) => text[..at]
                        .rfind("pub ")
                        .filter(|&p| p >= item.span.0)
                        .map(|p| format!("{}{}", &text[..p], &text[p + 4..])),
                    _ => None,
                }
            }
            // An import at the top.
            17..=21 => {
                let import = rng.pick(&NAME_IMPORTS).replace("{self}", stem);
                Some(format!("{import}\n{text}"))
            }
            // An attribute on an item's line: a `@test` on an item that is
            // not a function half the time, since one on a function is
            // stripped.
            22..=24 if !named.is_empty() => {
                let attr = rng.pick(&NAME_ATTRIBUTES);
                let others: Vec<&NamedItem> = named.iter().copied().filter(|i| !i.is_fn).collect();
                let item = if attr.starts_with("@test") && !others.is_empty() && rng.one_in(2) {
                    *rng.pick(&others)
                } else {
                    *rng.pick(&named)
                };
                let (start, _, _) = item.name.clone().expect("a named item");
                let at = line_start(&text, start);
                Some(format!("{}{attr}\n{}", &text[..at], &text[at..]))
            }
            // An item at the end.
            _ => {
                let item = rng.pick(&NAME_ITEMS);
                Some(format!("{}\n{item}", text.trim_end_matches('\n')))
            }
        };
        if let Some(next) = edited {
            if named_items(&next).is_some() {
                text = next;
            }
        }
    }
    text
}

/// [`name_mutate`] for a WASM program (5.2b), then one of
/// [`WASM_ADDITIONS`].
pub fn wasm_name_mutate(rng: &mut SplitMix64, source: &str, stem: &str) -> String {
    let text = name_mutate(rng, source, stem);
    add_one(rng, text, &WASM_ADDITIONS)
}

/// [`name_mutate`] for a module of [`PACKAGE_FILES`]' `app` (5.2b), then
/// one of [`PACKAGE_ADDITIONS`]: whether the module goes in `tests/` (one
/// in three) rather than `src/`, and its text.
pub fn package_name_mutate(rng: &mut SplitMix64, source: &str, stem: &str) -> (bool, String) {
    let tests = rng.one_in(3);
    let text = name_mutate(rng, source, stem);
    (tests, add_one(rng, text, &PACKAGE_ADDITIONS))
}

/// Whether `source` lexes and parses without an error: what the name
/// fuzzer starts from.
pub fn parses(source: &str) -> bool {
    named_items(source).is_some()
}

/// A type that the item passes refuse, for [`type_mutate`]'s retyping:
/// E0001 (a type nowhere, `_`), E0012 (arities) and E0900 (the types not
/// supported yet).
const TYPE_REPLACEMENTS: [&str; 10] = [
    "FzNowhere",
    "Option<Int, Int>",
    "Option",
    "Int<Bool>",
    "&Int",
    "Int?",
    "_",
    "[FzMissing]",
    "core::Int",
    "Future<Int, Bool>",
];

/// Items [`type_mutate`] appends, `{n}` replaced by a number of the
/// input's own so that no name repeats, each aimed at an item code: the
/// code it gives, then the text; `{chain}` is [`binding_chain`]'s.
pub const TYPE_ITEMS: [(&str, &str); 24] = [
    ("E0001", "fn fz_missing{n}(x: FzNowhere{n}) {}"),
    ("E0001", "impl FzNoTrait{n} for Int {}"),
    ("E0001", "fn fz_bound{n}<T: FzNoBound{n}>(x: T) {}"),
    ("E0012", "fn fz_arity{n}(x: Option) -> Result<Int> { panic(\"\") }"),
    ("E0015", "trait FzOne{n} {\n    type Out\n}\ntrait FzTwo{n} {\n    type Out\n}\nfn fz_amb{n}<T: FzOne{n} + FzTwo{n}>(x: T::Out) {}"),
    ("E0070", "trait FzAssoc{n} {\n    type Item\n    fn get(self) -> Int\n}\nrecord FzR{n} {}\nimpl FzAssoc{n} for FzR{n} {}"),
    ("E0071", "record FzIn{n} {}\nimpl FzIn{n} {\n    type Item = Int\n}"),
    ("E0071", "trait FzSmall{n} {}\nrecord FzS{n} {}\nimpl FzSmall{n} for FzS{n} {\n    fn extra(self) {}\n}"),
    ("E0072", "trait FzSup{n} {}\ntrait FzSub{n}: FzSup{n} {}\nrecord FzT{n} {}\nimpl FzSub{n} for FzT{n} {}"),
    ("E0072", "trait FzRet{n} {\n    fn f(self) -> Int\n}\nrecord FzU{n} {}\nimpl FzRet{n} for FzU{n} {\n    fn f(self) -> Bool { true }\n}"),
    ("E0073", "record FzV{n} {}\nimpl<T> FzV{n} {}"),
    ("E0074", "record FzW{n} {}\nimpl FzW{n} {\n    fn m(self) {}\n}\nimpl FzW{n} {\n    fn m(self) {}\n}"),
    ("E0074", "record FzX{n} {}\nimpl Eq for FzX{n} {\n    fn eq(self, o: FzX{n}) -> Bool { true }\n}\nimpl Eq for FzX{n} {\n    fn eq(self, o: FzX{n}) -> Bool { false }\n}"),
    ("E0076", "fn fz_self{n}<Self>() {}"),
    ("E0077", "trait FzCyc{n} {\n    type A\n    type B\n}\nrecord FzY{n} {}\nimpl FzCyc{n} for FzY{n} {\n    type A = Self::B\n    type B = Self::A\n}"),
    ("E0078", "{chain}"),
    ("E0403", "record FzDup{n}<T, T> { t: T }"),
    ("E0403", "trait FzBind{n} {\n    type Item\n}\nimpl FzBind{n} for Bool {\n    type Item = Int\n    type Item = Int\n}"),
    ("E0105", "trait FzLast{n} {\n    fn last(self) -> Int\n}\nimpl<T> FzLast{n} for Vec<T> {\n    fn last(self) -> Int { 0 }\n}"),
    ("E0010", "impl (Int, Bool) {}"),
    ("E0010", "trait FzArr{n} {}\nimpl FzArr{n} for [Int] {}"),
    ("E0900", "extern \"C\" {\n    fn fz_ffi{n}(x: String) -> [Int]\n}"),
    ("E0900", "extern \"Rust\" {\n    fn fz_abi{n}()\n}"),
    ("E0900", "extern \"C\" {\n    fn nova_fz{n}()\n}"),
];

/// What [`type_mutate`] edits in place: the spans of the program's types
/// (parameters', returns', fields'), of its generic lists' first
/// parameters, of its trait impls' methods and its impls.
struct TypeSites {
    types: Vec<(usize, usize)>,
    generics: Vec<(usize, usize)>,
    /// Each trait impl's methods: (start of its line, end of its body,
    /// end of its return type or of its parameter list's last type).
    methods: Vec<(usize, usize, Option<(usize, usize)>)>,
    /// Each trait impl's body's last byte, its `}`.
    trait_impl_ends: Vec<usize>,
    impls: Vec<(usize, usize)>,
}

fn type_sites(source: &str) -> Option<TypeSites> {
    use nova_ast::Item;
    let (tokens, lex_errors) = nova_lexer::lex(source, FileId::DUMMY);
    if !lex_errors.is_empty() {
        return None;
    }
    let parsed = nova_parser::parse_recovering(&tokens, FileId::DUMMY);
    if !parsed.errors.is_empty() {
        return None;
    }
    let span = |s: nova_diagnostics::Span| (s.start as usize, s.end as usize);
    let mut sites = TypeSites {
        types: Vec::new(),
        generics: Vec::new(),
        methods: Vec::new(),
        trait_impl_ends: Vec::new(),
        impls: Vec::new(),
    };
    let function = |f: &nova_ast::item::Function, sites: &mut TypeSites| {
        sites.types.extend(
            f.params
                .iter()
                .filter(|p| p.name.value != "self")
                .map(|p| span(p.ty.span)),
        );
        sites.types.extend(f.return_ty.iter().map(|t| span(t.span)));
        sites
            .generics
            .extend(f.generics.first().map(|g| span(g.name.span)));
    };
    for item in &parsed.file.items {
        match &item.value {
            Item::Function(f) => function(f, &mut sites),
            Item::Record(r) => {
                sites.types.extend(r.fields.iter().map(|f| span(f.ty.span)));
                sites
                    .generics
                    .extend(r.generics.first().map(|g| span(g.name.span)));
            }
            Item::Impl(b) => {
                sites.impls.push(span(item.span));
                sites
                    .generics
                    .extend(b.generics.first().map(|g| span(g.name.span)));
                for f in &b.functions {
                    function(f, &mut sites);
                    if b.trait_.is_some() {
                        let start = line_start(source, f.name.span.start as usize);
                        let typed = f.return_ty.as_ref().map(|t| span(t.span));
                        sites.methods.push((start, f.body.span.end as usize, typed));
                    }
                }
                if b.trait_.is_some() {
                    sites.trait_impl_ends.push(item.span.end as usize - 1);
                }
            }
            _ => {}
        }
    }
    Some(sites)
}

/// `source`, a program that checks at the item level, mutated one to
/// three times at the level of items (module docs, 5.): a type replaced
/// by one of [`TYPE_REPLACEMENTS`], a generic list's first parameter
/// duplicated, a trait impl's method dropped, added or given another
/// return type, an impl duplicated, or one of [`TYPE_ITEMS`] appended,
/// each numbered from `n`. A mutation that would not parse is skipped;
/// every name an appended item declares is new, so every input reaches
/// the type checker, which the harness checks.
pub fn type_mutate(rng: &mut SplitMix64, source: &str, n: usize) -> String {
    let mut text = source.to_string();
    for step in 0..1 + rng.below(3) {
        let Some(sites) = type_sites(&text) else {
            break;
        };
        let splice = |(start, end): (usize, usize), with: &str| {
            format!("{}{with}{}", &text[..start], &text[end..])
        };
        let edited = match rng.below(20) {
            0..=3 if !sites.types.is_empty() => Some(splice(
                *rng.pick(&sites.types),
                rng.pick(&TYPE_REPLACEMENTS),
            )),
            4 if !sites.generics.is_empty() => {
                let (start, end) = *rng.pick(&sites.generics);
                let name = text[start..end].to_string();
                Some(splice((start, end), &format!("{name}, {name}")))
            }
            5 if !sites.methods.is_empty() => {
                let (start, end, _) = *rng.pick(&sites.methods);
                Some(splice((start, end), ""))
            }
            6 if !sites.trait_impl_ends.is_empty() => {
                let at = *rng.pick(&sites.trait_impl_ends);
                Some(splice(
                    (at, at),
                    &format!("    fn fz_added{n}(self) {{}}\n"),
                ))
            }
            7 if sites.methods.iter().any(|m| m.2.is_some()) => {
                let typed: Vec<(usize, usize)> = sites.methods.iter().filter_map(|m| m.2).collect();
                Some(splice(*rng.pick(&typed), "(Int, Bool)"))
            }
            8 if !sites.impls.is_empty() => {
                let (start, end) = *rng.pick(&sites.impls);
                let copy = text[start..end].to_string();
                Some(format!("{}\n{copy}\n", text.trim_end_matches('\n')))
            }
            _ => {
                let (_, item) = rng.pick(&TYPE_ITEMS);
                let item = item
                    .replace("{chain}", &binding_chain())
                    .replace("{n}", &format!("{n}x{step}"));
                Some(format!("{}\n{item}\n", text.trim_end_matches('\n')))
            }
        };
        if let Some(next) = edited {
            if type_sites(&next).is_some() {
                text = next;
            }
        }
    }
    text
}

/// An impl whose associated types double at each of 17 steps, so that the
/// method signature's check meets more type nodes than normalizing allows
/// (E0078), as the type checker's own test of the limit builds one.
fn binding_chain() -> String {
    let mut text = String::from("record FzPair{n}<A, B> { a: A, b: B }\ntrait FzChain{n} {\n");
    for i in 0..17 {
        text.push_str(&format!("    type A{i}\n"));
    }
    text.push_str(
        "    fn put(self, y: Self::A0) -> Int\n}\nrecord FzW{n} {}\nimpl FzChain{n} for FzW{n} {\n",
    );
    for i in 0..16 {
        let next = i + 1;
        text.push_str(&format!(
            "    type A{i} = FzPair{{n}}<Self::A{next}, Self::A{next}>\n"
        ));
    }
    text.push_str("    type A16 = Int\n    fn put(self, y: Self::A0) -> Int { 0 }\n}");
    text
}
