//! `nova.toml`: what 3.0 reads, and the diagnostics that point into it
//! (spec `docs/superpowers/specs/2026-10-07-phase-3-0-foundations-design.md`
//! §5).

use std::ops::Range;
use std::path::PathBuf;

use nova_diagnostics::{Diagnostic, FileId, Severity, Span};
use toml_edit::{ImDocument, Item, Table, TableLike};

use crate::name::check_name;

/// The only edition this nova knows.
pub const EDITION: &str = "2026";

const PACKAGE_KEYS: [&str; 9] = [
    "name",
    "version",
    "edition",
    "description",
    "license",
    "repository",
    "authors",
    "keywords",
    "categories",
];

/// A parsed `nova.toml`.
#[derive(Debug, Clone, PartialEq)]
pub struct Manifest {
    pub package: Package,
    pub dependencies: Vec<Dependency>,
    pub dev_dependencies: Vec<Dependency>,
}

/// `[package]`.
#[derive(Debug, Clone, PartialEq)]
pub struct Package {
    pub name: String,
    pub version: semver::Version,
    /// Always `"2026"`, the only edition.
    pub edition: String,
    pub description: Option<String>,
    pub license: Option<String>,
    pub repository: Option<String>,
    pub authors: Vec<String>,
    pub keywords: Vec<String>,
    pub categories: Vec<String>,
    /// Where `[package]` is, for a diagnostic about the package as a
    /// whole (M0013, spec 3.3a §3.5).
    pub span: Span,
}

/// One entry of `[dependencies]` or `[dev-dependencies]`: a version
/// requirement or a path, never both (spec §5.1).
#[derive(Debug, Clone, PartialEq)]
pub struct Dependency {
    pub name: String,
    pub version: Option<semver::VersionReq>,
    pub path: Option<PathBuf>,
    /// Where its `path` value is, when it has one: M0007 and M0009 point
    /// there (spec 3.3a §3.5).
    pub path_span: Option<Span>,
    /// The entry's key, where a diagnostic about the entry points.
    pub span: Span,
}

/// Parse `source`, the text of a `nova.toml` that a `FileDb` holds as
/// `file`. The parser keeps positions (toml_edit's `ImDocument`), so every
/// diagnostic carries one. The manifest is returned when no diagnostic is
/// an error; warnings (M0006) leave it in place.
pub fn parse(source: &str, file: FileId) -> (Option<Manifest>, Vec<Diagnostic>) {
    let document = match ImDocument::parse(source) {
        Ok(document) => document,
        Err(error) => {
            let diagnostic = Diagnostic::error("M0001", "nova.toml is not valid TOML")
                .with_primary_label(span(file, error.span()), error.message().trim().to_string());
            return (None, vec![diagnostic]);
        }
    };
    let mut checker = Checker {
        file,
        diagnostics: Vec::new(),
    };
    let manifest = checker.manifest(document.as_table());
    let failed = checker
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity == Severity::Error);
    (manifest.filter(|_| !failed), checker.diagnostics)
}

fn span(file: FileId, range: Option<Range<usize>>) -> Span {
    let range = range.unwrap_or(0..0);
    Span::new(range.start as u32, range.end as u32, file)
}

struct Checker {
    file: FileId,
    diagnostics: Vec<Diagnostic>,
}

impl Checker {
    fn error(&mut self, code: &str, message: String, at: Option<Range<usize>>, label: &str) {
        let diagnostic =
            Diagnostic::error(code, message).with_primary_label(span(self.file, at), label);
        self.diagnostics.push(diagnostic);
    }

    fn manifest(&mut self, top: &Table) -> Option<Manifest> {
        self.unknown_keys(top, "", &["package", "dependencies", "dev-dependencies"]);
        let package = match top.get("package") {
            None => {
                self.error(
                    "M0002",
                    "nova.toml has no `[package]` table".to_string(),
                    None,
                    "expected `[package]` with `name`, `version` and `edition`",
                );
                None
            }
            Some(item) => match item.as_table_like() {
                Some(table) => self.package(table, item.span()),
                None => {
                    self.error(
                        "M0003",
                        format!("`package` must be a table, not {}", item.type_name()),
                        item.span(),
                        "expected a table",
                    );
                    None
                }
            },
        };
        let dependencies = self.dependencies(top, "dependencies");
        let dev_dependencies = self.dependencies(top, "dev-dependencies");
        Some(Manifest {
            package: package?,
            dependencies,
            dev_dependencies,
        })
    }

    fn package(&mut self, table: &dyn TableLike, at: Option<Range<usize>>) -> Option<Package> {
        self.unknown_keys(table, "package.", &PACKAGE_KEYS);
        let name =
            self.required(table, "name", &at)
                .and_then(|(name, at)| match check_name(&name) {
                    Ok(()) => Some(name),
                    Err(message) => {
                        self.error("M0003", message, at, "not a valid package name");
                        None
                    }
                });
        let version = self
            .required(table, "version", &at)
            .and_then(|(text, at)| self.version(&text, at));
        let edition = self
            .required(table, "edition", &at)
            .and_then(|(edition, at)| {
                if edition == EDITION {
                    Some(edition)
                } else {
                    self.error(
                        "M0003",
                        format!("unknown edition `{edition}`"),
                        at,
                        "this nova supports edition \"2026\"",
                    );
                    None
                }
            });
        let description = self.optional_string(table, "description");
        let license = self.optional_string(table, "license");
        let repository = self.optional_string(table, "repository");
        let authors = self.string_array(table, "authors");
        let keywords = self.string_array(table, "keywords");
        let categories = self.string_array(table, "categories");
        Some(Package {
            name: name?,
            version: version?,
            edition: edition?,
            description,
            license,
            repository,
            authors,
            keywords,
            categories,
            span: span(self.file, at),
        })
    }

    /// A required string key of `[package]`, with its value's position.
    fn required(
        &mut self,
        table: &dyn TableLike,
        key: &str,
        table_at: &Option<Range<usize>>,
    ) -> Option<(String, Option<Range<usize>>)> {
        let Some(item) = table.get(key) else {
            self.error(
                "M0002",
                format!("`[package]` has no `{key}`"),
                table_at.clone(),
                &format!("add `{key} = \"...\"` to this table"),
            );
            return None;
        };
        let value = self.string(item, &format!("package.{key}"))?;
        Some((value, item.span()))
    }

    fn string(&mut self, item: &Item, what: &str) -> Option<String> {
        if let Some(value) = item.as_str() {
            return Some(value.to_string());
        }
        self.error(
            "M0003",
            format!("`{what}` must be a string, not {}", item.type_name()),
            item.span(),
            "expected a string",
        );
        None
    }

    fn optional_string(&mut self, table: &dyn TableLike, key: &str) -> Option<String> {
        let item = table.get(key)?;
        self.string(item, &format!("package.{key}"))
    }

    fn string_array(&mut self, table: &dyn TableLike, key: &str) -> Vec<String> {
        let Some(item) = table.get(key) else {
            return Vec::new();
        };
        let strings = item.as_array().and_then(|array| {
            array
                .iter()
                .map(|value| value.as_str().map(str::to_string))
                .collect::<Option<Vec<_>>>()
        });
        strings.unwrap_or_else(|| {
            self.error(
                "M0003",
                format!("`package.{key}` must be an array of strings"),
                item.span(),
                "expected an array of strings",
            );
            Vec::new()
        })
    }

    fn version(&mut self, text: &str, at: Option<Range<usize>>) -> Option<semver::Version> {
        match semver::Version::parse(text) {
            Ok(version) if version.build.is_empty() => Some(version),
            Ok(_) => {
                self.error(
                    "M0003",
                    format!("package version `{text}` carries build metadata"),
                    at,
                    "a package version takes no `+...` part",
                );
                None
            }
            Err(error) => {
                self.error(
                    "M0003",
                    format!("`{text}` is not a version: {error}"),
                    at,
                    "a version is MAJOR.MINOR.PATCH, such as `0.1.0`",
                );
                None
            }
        }
    }

    fn dependencies(&mut self, top: &Table, key: &str) -> Vec<Dependency> {
        let Some(item) = top.get(key) else {
            return Vec::new();
        };
        let Some(table) = item.as_table_like() else {
            self.error(
                "M0003",
                format!("`{key}` must be a table, not {}", item.type_name()),
                item.span(),
                "expected a table",
            );
            return Vec::new();
        };
        let mut found = Vec::new();
        for (name, entry) in table.iter() {
            let at = table.get_key_value(name).and_then(|(key, _)| key.span());
            if let Some(dependency) = self.dependency(key, name, entry, at) {
                found.push(dependency);
            }
        }
        found
    }

    fn dependency(
        &mut self,
        table: &str,
        name: &str,
        entry: &Item,
        at: Option<Range<usize>>,
    ) -> Option<Dependency> {
        let position = span(self.file, at.clone());
        if let Some(requirement) = entry.as_str() {
            let version = self.requirement(requirement, entry.span())?;
            return Some(Dependency {
                name: name.to_string(),
                version: Some(version),
                path: None,
                path_span: None,
                span: position,
            });
        }
        let Some(fields) = entry.as_table_like() else {
            self.error(
                "M0003",
                format!(
                    "dependency `{name}` must be a version requirement or a table, not {}",
                    entry.type_name()
                ),
                entry.span(),
                "expected a string or a table",
            );
            return None;
        };
        self.unknown_keys(fields, &format!("{table}.{name}."), &["version", "path"]);
        let path_span = fields.get("path").map(|item| span(self.file, item.span()));
        let version = match fields.get("version") {
            Some(item) => {
                let text = self.string(item, &format!("{table}.{name}.version"))?;
                Some(self.requirement(&text, item.span())?)
            }
            None => None,
        };
        let path = match fields.get("path") {
            Some(item) => Some(PathBuf::from(
                self.string(item, &format!("{table}.{name}.path"))?,
            )),
            None => None,
        };
        match (version, path) {
            (None, None) => {
                self.error(
                    "M0004",
                    format!("dependency `{name}` has neither `version` nor `path`"),
                    at,
                    "add `version = \"...\"` or `path = \"...\"`",
                );
                None
            }
            (Some(_), Some(_)) => {
                self.error(
                    "M0003",
                    format!("dependency `{name}` has both `version` and `path`"),
                    at,
                    "use one of them; what both would mean is not decided yet",
                );
                None
            }
            (version, path) => Some(Dependency {
                name: name.to_string(),
                version,
                path,
                path_span,
                span: position,
            }),
        }
    }

    fn requirement(&mut self, text: &str, at: Option<Range<usize>>) -> Option<semver::VersionReq> {
        match semver::VersionReq::parse(text) {
            Ok(requirement) => Some(requirement),
            Err(error) => {
                self.error(
                    "M0003",
                    format!("`{text}` is not a version requirement: {error}"),
                    at,
                    "a requirement looks like `1.2` or `^0.3`",
                );
                None
            }
        }
    }

    fn unknown_keys(&mut self, table: &dyn TableLike, prefix: &str, known: &[&str]) {
        for (key, item) in table.iter() {
            if known.contains(&key) {
                continue;
            }
            let at = table
                .get_key_value(key)
                .and_then(|(found, _)| found.span())
                .or_else(|| item.span());
            let diagnostic =
                Diagnostic::warning("M0006", format!("unknown key `{prefix}{key}`, ignored"))
                    .with_primary_label(span(self.file, at), "this nova does not read this key");
            self.diagnostics.push(diagnostic);
        }
    }
}
