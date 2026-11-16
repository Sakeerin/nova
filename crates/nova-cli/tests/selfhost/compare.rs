//! The comparison of S0's and N's dumps, and the report of the first place
//! they differ: the file, the line of its dump and of its source (in a MIR
//! dump, which has no spans, its function, block and statement), both
//! sides' lines, and how to reproduce it.

use std::path::Path;

/// A pair of commands whose outputs must agree: S0's arguments to `nova`,
/// and N's to `selfhost`, each followed by the files.
pub struct Dump {
    pub s0: &'static [&'static str],
    pub n: &'static [&'static str],
}

/// 5.1a's token dump.
pub const TOKENS: Dump = Dump {
    s0: &["parse", "--dump-tokens"],
    n: &["tokens"],
};

/// 5.1b's AST dump (ADR 0052).
pub const AST: Dump = Dump {
    s0: &["parse", "--dump-ast"],
    n: &["ast"],
};

/// Where two dumps first differ.
#[derive(Debug, PartialEq, Eq)]
pub struct Mismatch {
    /// The file whose dump differs: its `== FILE` header's path, or `None`
    /// before the first header.
    pub file: Option<String>,
    /// The line within that file's dump, 1 for the line after its header;
    /// 0 for the header itself, where one side's output ended before it.
    pub dump_line: usize,
    /// S0's line there, or `None` past the end of its output. A line that
    /// differs from N's only in lacking the final line break ends
    /// ` (no line break)`.
    pub s0: Option<String>,
    /// N's line there, or `None` past the end of its output, marked as S0's.
    pub n: Option<String>,
    /// In a MIR root's part, where the line is ([`MirWalk::place`]), which
    /// the report gives in place of a source line; `None` in any other
    /// dump, before the first header, and at a header.
    pub place: Option<String>,
}

/// The first line where `s0` and `n` differ, or `None` if they are equal.
/// They are compared as bytes, so output that is not UTF-8 differs where
/// its bytes do; the lines are shown lossily.
pub fn first_difference(s0: &[u8], n: &[u8]) -> Option<Mismatch> {
    if s0 == n {
        return None;
    }
    let mut left = s0.split_inclusive(|&b| b == b'\n');
    let mut right = n.split_inclusive(|&b| b == b'\n');
    let mut file = None;
    let mut dump_line = 0;
    let mut walk = MirWalk::default();
    loop {
        let (a, b) = (left.next(), right.next());
        if a != b {
            // Lines that differ only in a final line break, which one side
            // lacks, would read the same: the one without it says so.
            let unbroken = |s: &[u8], other: Option<&[u8]>| {
                !s.ends_with(b"\n") && other.is_some_and(|o| o.strip_suffix(b"\n") == Some(s))
            };
            let line = |s: Option<&[u8]>, other: Option<&[u8]>| {
                s.map(|s| {
                    if unbroken(s, other) {
                        format!("{} (no line break)", text(s))
                    } else {
                        text(s.strip_suffix(b"\n").unwrap_or(s))
                    }
                })
            };
            // One side ended where the other begins a file: that file is
            // the one the ended side never reached, so it is named.
            let header = match (a, b) {
                (None, Some(h)) | (Some(h), None) => h.strip_prefix(b"== "),
                _ => None,
            };
            return Some(match header {
                Some(path) => Mismatch {
                    file: Some(text(path).trim_end_matches('\n').to_string()),
                    dump_line: 0,
                    s0: line(a, b),
                    n: line(b, a),
                    place: None,
                },
                None => Mismatch {
                    place: match a.or(b) {
                        Some(differs) if file.is_some() => walk.place(dump_line + 1, differs),
                        _ => None,
                    },
                    file,
                    dump_line: dump_line + 1,
                    s0: line(a, b),
                    n: line(b, a),
                },
            });
        }
        let a = a.expect("equal outputs were returned above");
        match a.strip_prefix(b"== ") {
            Some(path) => {
                file = Some(text(path).trim_end_matches('\n').to_string());
                dump_line = 0;
            }
            None => {
                dump_line += 1;
                walk.step(dump_line, a);
            }
        }
    }
}

/// Where a line of a MIR root's part is (spec 5.4 §4.3): MIR has no spans,
/// so the report names the function and the block above the line, and the
/// statement's index there, where other dumps give a source line. The part
/// is walked line by line, as [`first_difference`] reads the lines both
/// sides share, by the dump's indentation (ADR 0053 §1, ADR 0057 §2): at
/// depth 0 the root's `(Mir` line and `(Diagnostics`; at depth 1 a
/// `(Function` line, or an `(Extern` or a diagnostic's `(Error`; at depth 2
/// a function's `(Temp` and `(Block` lines; at depth 3 a block's
/// statements, then its terminator.
#[derive(Clone, Default)]
struct MirWalk {
    /// Whether the part is a MIR root's: its first line starts `(Mir `.
    mir: bool,
    /// The function the last line walked is in, its name as the dump
    /// quotes it; `None` outside any function.
    function: Option<String>,
    /// The block the last line walked is in, `bb<n>`; `None` outside any.
    block: Option<String>,
    /// The block's statement lines walked, its terminator counted as one.
    statements: usize,
}

impl MirWalk {
    /// Walk `line`, line `dump_line` of the root's part.
    fn step(&mut self, dump_line: usize, line: &[u8]) {
        if dump_line == 1 {
            self.mir = line.starts_with(b"(Mir ");
        }
        if !self.mir {
            return;
        }
        let line = text(line);
        let line = line.trim_end_matches('\n');
        let node = line.trim_start_matches(' ');
        match (line.len() - node.len()) / 2 {
            0 => {
                self.function = None;
                self.block = None;
            }
            1 => {
                self.function = node
                    .strip_prefix("(Function ")
                    .and_then(|rest| rest.split(" params=").next())
                    .map(str::to_string);
                self.block = None;
            }
            2 => {
                self.block = node
                    .strip_prefix("(Block ")
                    .map(|b| b.trim_end_matches(')').to_string());
                self.statements = 0;
            }
            _ => self.statements += 1,
        }
    }

    /// The place of `line`, the part's line `dump_line`, which differs,
    /// after the lines walked, which both sides share, so either side's
    /// text gives them. The line itself is S0's, or N's where S0's output
    /// ended; its own kind decides what is named:
    /// - the part's first line, the root's `(Mir` line: that alone;
    /// - a line outside any function (an extern, MIR's diagnostics):
    ///   "outside any function";
    /// - a function's header: that function, by the differing line's name;
    /// - a temp: the function, "at its temps";
    /// - a block's header: the function and that block;
    /// - a statement or a terminator: the function, the block, and its
    ///   index in the block from 0, as `Block::stmts` counts, the
    ///   terminator after the last statement.
    ///
    /// `None` for a part that is not a MIR root's, and at a header: the next
    /// root's `== FILE` line, where one side printed a line more than the
    /// other, is in no root's part.
    fn place(&self, dump_line: usize, line: &[u8]) -> Option<String> {
        if line.starts_with(b"== ") {
            return None;
        }
        if dump_line == 1 {
            return line
                .starts_with(b"(Mir ")
                .then(|| "the root's first line".to_string());
        }
        if !self.mir {
            return None;
        }
        let mut at = self.clone();
        at.step(dump_line, line);
        let depth = line.iter().take_while(|&&b| b == b' ').count() / 2;
        Some(match (&at.function, &at.block, depth) {
            (None, _, _) => "outside any function".to_string(),
            (Some(f), None, 1) => format!("function {f}, at its header"),
            (Some(f), None, _) => format!("function {f}, at its temps"),
            (Some(f), Some(b), 2) => format!("function {f}, block {b}, at its header"),
            (Some(f), Some(b), _) => {
                format!("function {f}, block {b}, statement {}", self.statements)
            }
        })
    }
}

/// `bytes` as text, each run that is not UTF-8 as a replacement character.
fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// The 1-based source line of the offset a dump line starts with (a token's
/// start, or an error's or comment's; in an AST dump, a node's span's start
/// or a parse error's), if `file` can be read from `dir`.
fn source_line(dir: &Path, file: &str, line: &str) -> Option<usize> {
    let mut fields = line.trim_start().split(' ');
    let first = fields.next()?;
    let offset: usize = match first {
        "ERR" | "COMMENT" | "PARSE" => fields.next()?.parse().ok()?,
        node if node.starts_with('(') => fields.next()?.split_once("..")?.0.parse().ok()?,
        _ => first.parse().ok()?,
    };
    let source = std::fs::read(dir.join(file)).ok()?;
    let before = source.get(..offset.min(source.len()))?;
    Some(1 + before.iter().filter(|&&b| b == b'\n').count())
}

/// The report of `m`, a difference in `dump` on files read from `dir`, with
/// the commands that reproduce it.
pub fn report(m: &Mismatch, dump: &Dump, dir: &Path, nova: &Path, selfhost: &Path) -> String {
    let file = m.file.as_deref().unwrap_or("(before the first file)");
    let shown = |s: &Option<String>| match s {
        Some(s) => s.clone(),
        None => "(nothing: the output ended)".to_string(),
    };
    let at = if m.dump_line == 0 {
        "at its header: one side's output ended before it".to_string()
    } else {
        // A MIR dump's line has no span: its place stands for the source
        // line.
        let at_source = match &m.place {
            Some(place) => format!(", {place}"),
            None => {
                m.s0.as_deref()
                    .or(m.n.as_deref())
                    .and_then(|line| source_line(dir, file, line))
                    .map(|l| format!(", source line {l}"))
                    .unwrap_or_default()
            }
        };
        format!("at line {} of its dump{at_source}", m.dump_line)
    };
    let (s0_args, n_args) = (dump.s0.join(" "), dump.n.join(" "));
    let s0_label = format!("S0 (nova {s0_args}):");
    let n_label = format!("N  (selfhost {n_args}):");
    let width = s0_label.len().max(n_label.len());
    format!(
        "the dumps differ in {file}, {at}\n\
         \x20 {s0_label:<width$} {s0}\n\
         \x20 {n_label:<width$} {n}\n\
         reproduce, in {dir}:\n\
         \x20 {nova} {s0_args} {file} > s0.txt\n\
         \x20 {selfhost} {n_args} {file} > n.txt\n\
         \x20 diff s0.txt n.txt\n\
         and add the file to crates/nova-cli/tests/selfhost/cases/ once it is fixed",
        s0 = shown(&m.s0),
        n = shown(&m.n),
        dir = dir.display(),
        nova = nova.display(),
        selfhost = selfhost.display(),
    )
}

/// A resolution dump as both sides are compared on `windows` (ADR 0054
/// §7, and decision 4 of its §8): each `\` of E0004's and E0093's messages, the
/// two that print a path, read as `/`. S0 writes such a path as
/// `Path::join` does, with `\` on Windows, which the dump escapes as
/// `\x5c`, where N writes `/`. Every other line, and every line elsewhere,
/// is kept as it is.
pub fn settle_paths(dump: &[u8], windows: bool) -> Vec<u8> {
    if !windows {
        return dump.to_vec();
    }
    let mut out = Vec::with_capacity(dump.len());
    for line in dump.split_inclusive(|&b| b == b'\n') {
        let path_message =
            line.starts_with(b"  (Error \"E0004\" ") || line.starts_with(b"  (Error \"E0093\" ");
        if !path_message {
            out.extend_from_slice(line);
            continue;
        }
        let mut rest = line;
        while !rest.is_empty() {
            if rest.starts_with(b"\\x5c") {
                out.push(b'/');
                rest = &rest[4..];
            } else {
                out.push(rest[0]);
                rest = &rest[1..];
            }
        }
    }
    out
}
