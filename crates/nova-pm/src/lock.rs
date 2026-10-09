//! `nova.lock` (spec
//! `docs/superpowers/specs/2026-10-09-phase-3-3b-index-and-publishing-design.md`
//! §4.6): the registry packages a project builds with, and the index they
//! came from.

use std::ops::Range;

use nova_diagnostics::{Diagnostic, FileId, Span};
use semver::Version;
use toml_edit::{ImDocument, Item};

/// The lockfile's name, beside `nova.toml`.
pub const LOCKFILE: &str = "nova.lock";

/// A project's `nova.lock`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lock {
    /// The canonical form of the index its packages came from.
    pub index: String,
    /// Sorted by name.
    pub packages: Vec<LockedPackage>,
}

/// One registry package of a [`Lock`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LockedPackage {
    pub name: String,
    pub version: Version,
    /// The SHA-256 of its tarball, 64 lower-case hex digits.
    pub checksum: String,
    /// The names of its dependencies, sorted.
    pub dependencies: Vec<String>,
}

impl Lock {
    pub fn find(&self, name: &str) -> Option<&LockedPackage> {
        self.packages.iter().find(|p| p.name == name)
    }

    /// The lock as nova writes it, with `\n` line endings.
    pub fn to_text(&self) -> String {
        let mut out = String::from("# Written by nova. Commit it for a program.\nversion = 1\n");
        out.push_str(&format!("index = {}\n", quote(&self.index)));
        for package in &self.packages {
            out.push_str("\n[[package]]\n");
            out.push_str(&format!("name = {}\n", quote(&package.name)));
            out.push_str(&format!(
                "version = {}\n",
                quote(&package.version.to_string())
            ));
            out.push_str(&format!("checksum = {}\n", quote(&package.checksum)));
            let names: Vec<String> = package.dependencies.iter().map(|n| quote(n)).collect();
            out.push_str(&format!("dependencies = [{}]\n", names.join(", ")));
        }
        out
    }
}

/// `text` as a TOML basic string.
fn quote(text: &str) -> String {
    let mut out = String::from("\"");
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if c.is_control() => out.push_str(&format!("\\u{:04X}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Parse `text`, the contents of a `nova.lock` that a `FileDb` holds as
/// `file`. Anything a lock written by nova would not hold is M0016.
pub fn parse_lock(text: &str, file: FileId) -> Result<Lock, Diagnostic> {
    let bad = |message: &str, at: Option<Range<usize>>| {
        let at = at.unwrap_or(0..0);
        Diagnostic::error("M0016", format!("nova.lock cannot be read: {message}"))
            .with_primary_label(Span::new(at.start as u32, at.end as u32, file), "here")
            .with_note("delete nova.lock, or run `nova update`")
    };
    let document = ImDocument::parse(text).map_err(|e| bad(e.message().trim(), e.span()))?;
    let table = document.as_table();
    if table.get("version").and_then(Item::as_integer) != Some(1) {
        return Err(bad(
            "its `version` is not 1",
            table.get("version").and_then(Item::span),
        ));
    }
    let index = table
        .get("index")
        .and_then(Item::as_str)
        .ok_or_else(|| bad("it has no `index`", None))?
        .to_string();
    let mut packages = Vec::new();
    if let Some(item) = table.get("package") {
        let tables = item
            .as_array_of_tables()
            .ok_or_else(|| bad("`package` is not a list of tables", item.span()))?;
        for package in tables.iter() {
            let field = |key: &str| package.get(key).and_then(Item::as_str);
            let (Some(name), Some(version), Some(checksum)) =
                (field("name"), field("version"), field("checksum"))
            else {
                return Err(bad(
                    "a package lacks its name, version or checksum",
                    package.span(),
                ));
            };
            let version = Version::parse(version)
                .map_err(|e| bad(&format!("version `{version}`: {e}"), package.span()))?;
            let dependencies = match package.get("dependencies") {
                None => Vec::new(),
                Some(item) => item
                    .as_array()
                    .and_then(|array| {
                        array
                            .iter()
                            .map(|v| v.as_str().map(str::to_string))
                            .collect::<Option<Vec<_>>>()
                    })
                    .ok_or_else(|| bad("`dependencies` is not a list of names", item.span()))?,
            };
            packages.push(LockedPackage {
                name: name.to_string(),
                version,
                checksum: checksum.to_string(),
                dependencies,
            });
        }
    }
    packages.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(Lock { index, packages })
}
