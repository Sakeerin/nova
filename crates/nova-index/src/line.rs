//! An index file's lines, and `config.json` (spec §3.1, §3.2).

use nova_pm::Candidate;
use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};

/// The line format this nova reads and writes.
pub const LINE_VERSION: u64 = 1;

/// One version of a package: one line of its index file. The fields are in
/// the order they are written.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Line {
    pub name: String,
    pub vers: String,
    /// Its `[dependencies]`, never its dev-dependencies.
    pub deps: Vec<LineDep>,
    /// The tarball's SHA-256, 64 lower-case hex digits.
    pub cksum: String,
    pub v: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LineDep {
    pub name: String,
    pub req: String,
}

impl Line {
    /// The line as an index file holds it, without its `\n`.
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).expect("a line always serializes")
    }

    /// The resolver's view of this version.
    pub fn candidate(&self) -> Result<Candidate, String> {
        let version =
            Version::parse(&self.vers).map_err(|e| format!("version `{}`: {e}", self.vers))?;
        let deps = self
            .deps
            .iter()
            .map(|dep| {
                VersionReq::parse(&dep.req)
                    .map(|req| (dep.name.clone(), req))
                    .map_err(|e| format!("requirement `{}` on `{}`: {e}", dep.req, dep.name))
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Candidate {
            version,
            deps,
            checksum: self.cksum.clone(),
        })
    }
}

/// The lines of an index file that `file` names. A line of a later format
/// (its `v` is not [`LINE_VERSION`]) is skipped silently, so the format
/// can grow; a line that does not parse is skipped with a note, and never
/// stops a command (spec §3.1).
pub fn parse_lines(text: &str, file: &str) -> (Vec<Line>, Vec<String>) {
    let mut lines = Vec::new();
    let mut notes = Vec::new();
    for (index, raw) in text.lines().enumerate() {
        let raw = raw.trim();
        if raw.is_empty() {
            continue;
        }
        let note = |reason: String| {
            format!(
                "{file}:{}: skipped a line that does not parse: {reason}",
                index + 1
            )
        };
        let value: serde_json::Value = match serde_json::from_str(raw) {
            Ok(value) => value,
            Err(error) => {
                notes.push(note(error.to_string()));
                continue;
            }
        };
        match value.get("v").and_then(serde_json::Value::as_u64) {
            Some(LINE_VERSION) => {}
            Some(_) => continue,
            None => {
                notes.push(note("it has no `v`".to_string()));
                continue;
            }
        }
        let parsed = serde_json::from_value::<Line>(value)
            .map_err(|e| e.to_string())
            .and_then(|line| line.candidate().map(|_| line));
        match parsed {
            Ok(line) => lines.push(line),
            Err(reason) => notes.push(note(reason)),
        }
    }
    (lines, notes)
}

/// `config.json`, at an index's root (spec §3.2).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Config {
    /// Where tarballs are: a template with `{name}` and `{version}`.
    pub dl: String,
    /// The GitHub repository `nova publish` writes to, `owner/name`.
    #[serde(default)]
    pub api: Option<String>,
}

pub fn parse_config(text: &str) -> Result<Config, String> {
    serde_json::from_str(text).map_err(|e| format!("the index's config.json does not parse: {e}"))
}
