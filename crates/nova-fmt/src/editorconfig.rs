//! The two `.editorconfig` settings `nova fmt` reads (spec
//! `docs/superpowers/specs/2026-10-07-phase-3-1-formatter-design.md` §7.3):
//! `end_of_line` and `insert_final_newline`. Every other key is ignored.

use std::path::{Path, PathBuf};

/// What the `.editorconfig` files say about one file: `None` where nothing
/// sets a value.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct Settings {
    /// `Some(true)` for `crlf`, `Some(false)` for `lf`.
    pub crlf: Option<bool>,
    pub insert_final_newline: Option<bool>,
}

/// The settings for `file`, an absolute path, from the `.editorconfig` in
/// its directory and those above it, up to one that says `root = true`.
/// Nearer files override farther ones, and later sections earlier ones.
pub(crate) fn settings_for(file: &Path) -> Settings {
    let mut found: Vec<(PathBuf, String)> = Vec::new();
    for dir in file.ancestors().skip(1) {
        if let Ok(text) = std::fs::read_to_string(dir.join(".editorconfig")) {
            let root = is_root(&text);
            found.push((dir.to_path_buf(), text));
            if root {
                break;
            }
        }
    }
    let mut settings = Settings::default();
    for (dir, text) in found.iter().rev() {
        if let Ok(rel) = file.strip_prefix(dir) {
            let rel = rel.to_string_lossy().replace('\\', "/");
            apply(&mut settings, text, &rel);
        }
    }
    settings
}

/// Whether `text`'s preamble, before its first section, says `root = true`.
fn is_root(text: &str) -> bool {
    for line in text.lines() {
        if line.trim_start().starts_with('[') {
            return false;
        }
        if let Some((key, value)) = pair(line) {
            if key == "root" {
                return value == "true";
            }
        }
    }
    false
}

/// A `key = value` line, both trimmed and lowercased: keys and values are
/// case-insensitive. `None` for a blank line, a comment or a section.
fn pair(line: &str) -> Option<(String, String)> {
    let line = line.trim();
    if line.is_empty() || line.starts_with('#') || line.starts_with(';') || line.starts_with('[') {
        return None;
    }
    let (key, value) = line.split_once('=')?;
    Some((
        key.trim().to_ascii_lowercase(),
        value.trim().to_ascii_lowercase(),
    ))
}

/// Apply, in order, the sections of `text` that cover `rel`: the file's
/// path from the `.editorconfig`'s directory, with `/` separators.
fn apply(settings: &mut Settings, text: &str, rel: &str) {
    let mut active = false;
    for line in text.lines() {
        let line = line.trim();
        if let Some(glob) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            active = section_matches(glob, rel);
            continue;
        }
        if !active {
            continue;
        }
        let Some((key, value)) = pair(line) else {
            continue;
        };
        match (key.as_str(), value.as_str()) {
            ("end_of_line", "lf") => settings.crlf = Some(false),
            ("end_of_line", "crlf") => settings.crlf = Some(true),
            ("end_of_line", "unset") => settings.crlf = None,
            ("insert_final_newline", "true") => settings.insert_final_newline = Some(true),
            ("insert_final_newline", "false") => settings.insert_final_newline = Some(false),
            ("insert_final_newline", "unset") => settings.insert_final_newline = None,
            _ => {}
        }
    }
}

/// Whether section `glob` covers `rel`. A glob with no `/` matches the
/// file's name at any depth; one with a `/` matches the whole path from the
/// `.editorconfig`'s directory.
fn section_matches(glob: &str, rel: &str) -> bool {
    let (glob, target) = if glob.contains('/') {
        (glob.strip_prefix('/').unwrap_or(glob), rel)
    } else {
        (glob, rel.rsplit('/').next().unwrap_or(rel))
    };
    let target: Vec<char> = target.chars().collect();
    expand(glob)
        .iter()
        .any(|g| matches(&g.chars().collect::<Vec<char>>(), &target))
}

/// `glob` with its `{a,b}` alternatives expanded. A `{1..3}` range is not
/// supported, so a glob holding one expands to nothing and never matches.
fn expand(glob: &str) -> Vec<String> {
    let chars: Vec<char> = glob.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        match chars[i] {
            '\\' => i += 2,
            '{' => {
                let Some((close, commas)) = brace_group(&chars, i) else {
                    // An unclosed `{` is a literal.
                    i += 1;
                    continue;
                };
                if commas.is_empty() {
                    let inner: String = chars[i + 1..close].iter().collect();
                    if inner.contains("..") {
                        return Vec::new();
                    }
                    // `{word}`, with no comma, is literal.
                    i = close + 1;
                    continue;
                }
                let prefix: String = chars[..i].iter().collect();
                let suffix: String = chars[close + 1..].iter().collect();
                let mut out = Vec::new();
                let mut from = i + 1;
                for &to in commas.iter().chain(std::iter::once(&close)) {
                    let alternative: String = chars[from..to].iter().collect();
                    out.extend(expand(&format!("{prefix}{alternative}{suffix}")));
                    from = to + 1;
                }
                return out;
            }
            _ => i += 1,
        }
    }
    vec![glob.to_owned()]
}

/// The `}` that closes the `{` at `open`, and the commas directly inside
/// it.
fn brace_group(chars: &[char], open: usize) -> Option<(usize, Vec<usize>)> {
    let mut depth = 0;
    let mut commas = Vec::new();
    let mut i = open;
    while i < chars.len() {
        match chars[i] {
            '\\' => i += 1,
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some((i, commas));
                }
            }
            ',' if depth == 1 => commas.push(i),
            _ => {}
        }
        i += 1;
    }
    None
}

/// Whether `glob`, its braces expanded, matches all of `path`: `*` within
/// a path segment, `**` across them, `?` one character, `[…]` and `[!…]` a
/// class, and `\` an escape.
fn matches(glob: &[char], path: &[char]) -> bool {
    match glob {
        [] => path.is_empty(),
        ['*', '*', rest @ ..] => (0..=path.len()).any(|k| matches(rest, &path[k..])),
        ['*', rest @ ..] => (0..=path.len())
            .take_while(|&k| k == 0 || path[k - 1] != '/')
            .any(|k| matches(rest, &path[k..])),
        ['?', rest @ ..] => path.first().is_some_and(|&c| c != '/') && matches(rest, &path[1..]),
        ['[', ..] => class(glob, path),
        ['\\', c, rest @ ..] | [c, rest @ ..] => {
            path.first() == Some(c) && matches(rest, &path[1..])
        }
    }
}

/// A `[…]` or `[!…]` class at the start of `glob`, with `a-z` ranges, then
/// the rest of `glob`. A `[` with no `]` is a literal.
fn class(glob: &[char], path: &[char]) -> bool {
    let Some(end) = glob.iter().skip(1).position(|&c| c == ']').map(|p| p + 1) else {
        return path.first() == Some(&'[') && matches(&glob[1..], &path[1..]);
    };
    let (negated, set) = match &glob[1..end] {
        ['!', set @ ..] => (true, set),
        set => (false, set),
    };
    let Some(&c) = path.first() else {
        return false;
    };
    let mut hit = false;
    let mut k = 0;
    while k < set.len() {
        if k + 2 < set.len() && set[k + 1] == '-' {
            hit |= set[k] <= c && c <= set[k + 2];
            k += 3;
        } else {
            hit |= set[k] == c;
            k += 1;
        }
    }
    c != '/' && hit != negated && matches(&glob[end + 1..], &path[1..])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn braces_expand_and_numeric_ranges_never_match() {
        assert_eq!(expand("*.{nova,toml}"), vec!["*.nova", "*.toml"]);
        assert_eq!(expand("{a,{b,c}}"), vec!["a", "b", "c"]);
        assert_eq!(expand("{word}.nova"), vec!["{word}.nova"]);
        assert!(expand("{1..3}.nova").is_empty());
    }

    #[test]
    fn globs_match_as_editorconfig_says() {
        for (glob, path, want) in [
            ("*", "src/main.nova", true),
            ("*.nova", "main.nova", true),
            ("*.nova", "src/deep/main.nova", true),
            ("*.toml", "main.nova", false),
            ("src/*.nova", "src/main.nova", true),
            ("src/*.nova", "src/deep/main.nova", false),
            ("src/**.nova", "src/deep/main.nova", true),
            ("/main.nova", "main.nova", true),
            ("main.nov?", "main.nova", true),
            ("[lm]ain.nova", "main.nova", true),
            ("[!m]ain.nova", "main.nova", false),
            ("[a-k]ain.nova", "main.nova", false),
            ("{lib,main}.nova", "main.nova", true),
            ("{1..3}.nova", "1.nova", false),
        ] {
            assert_eq!(section_matches(glob, path), want, "[{glob}] on {path}");
        }
    }

    /// A fresh directory for one test, under the system temp directory. Its
    /// name is fixed, so each run replaces the last run's.
    fn fresh_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("nova-fmt-editorconfig-{name}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create the test directory");
        dir
    }

    #[test]
    fn nearer_files_and_later_sections_win() {
        let dir = fresh_dir("nearer");
        let sub = dir.join("sub");
        std::fs::create_dir_all(&sub).unwrap();
        std::fs::write(
            dir.join(".editorconfig"),
            "root = true\n[*]\nend_of_line = crlf\ninsert_final_newline = false\n",
        )
        .unwrap();
        std::fs::write(
            sub.join(".editorconfig"),
            "[*.nova]\nend_of_line = lf\n\n# a comment\n[x.nova]\nEND_OF_LINE = CRLF\n",
        )
        .unwrap();
        let both = |crlf| Settings {
            crlf: Some(crlf),
            insert_final_newline: Some(false),
        };
        assert_eq!(settings_for(&sub.join("a.nova")), both(false));
        assert_eq!(settings_for(&sub.join("x.nova")), both(true));
        assert_eq!(settings_for(&dir.join("b.nova")), both(true));
    }

    #[test]
    fn the_search_stops_at_root() {
        let dir = fresh_dir("root");
        let inner = dir.join("inner");
        std::fs::create_dir_all(&inner).unwrap();
        std::fs::write(
            dir.join(".editorconfig"),
            "root = true\n[*]\nend_of_line = crlf\n",
        )
        .unwrap();
        std::fs::write(inner.join(".editorconfig"), "root = true\n").unwrap();
        assert_eq!(settings_for(&inner.join("a.nova")), Settings::default());
    }

    #[test]
    fn other_keys_and_other_values_are_ignored() {
        let dir = fresh_dir("ignored");
        std::fs::write(
            dir.join(".editorconfig"),
            "root = true\n[*]\nindent_style = tab\nindent_size = 2\nend_of_line = cr\n\
             insert_final_newline = maybe\n",
        )
        .unwrap();
        assert_eq!(settings_for(&dir.join("a.nova")), Settings::default());
    }
}
