//! The comparison of S0's and N's dumps, and the report of the first place
//! they differ: the file, the line of its dump and of its source, both
//! sides' lines, and how to reproduce it.

use std::path::Path;

/// A pair of commands whose outputs must agree: S0's arguments to `nova`,
/// and N's to `selfhost`, each followed by the files.
pub struct Dump {
    pub s0: &'static [&'static str],
    pub n: &'static [&'static str],
}

/// 5.1a's token dump. 5.1b's AST dump is a second `Dump` beside it.
pub const TOKENS: Dump = Dump {
    s0: &["parse", "--dump-tokens"],
    n: &["tokens"],
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
                },
                None => Mismatch {
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
            None => dump_line += 1,
        }
    }
}

/// `bytes` as text, each run that is not UTF-8 as a replacement character.
fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// The 1-based source line of the offset a dump line starts with (a token's
/// start, or an error's or comment's), if `file` can be read from `dir`.
fn source_line(dir: &Path, file: &str, line: &str) -> Option<usize> {
    let mut fields = line.split(' ');
    let first = fields.next()?;
    let offset: usize = match first {
        "ERR" | "COMMENT" => fields.next()?.parse().ok()?,
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
        let at_source =
            m.s0.as_deref()
                .or(m.n.as_deref())
                .and_then(|line| source_line(dir, file, line))
                .map(|l| format!(", source line {l}"))
                .unwrap_or_default();
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
