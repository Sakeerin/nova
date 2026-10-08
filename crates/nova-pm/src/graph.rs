//! The package graph (spec
//! `docs/superpowers/specs/2026-10-08-phase-3-3a-local-packages-design.md`
//! §3): the root package, and every package its path dependencies reach.

use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};

use nova_diagnostics::{Diagnostic, FileDb, FileId, Span};

use crate::manifest::{Dependency, Manifest};
use crate::{import_name, real_path, MANIFEST};

/// A package's index in its [`Graph`]. The root is `PackageId(0)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PackageId(pub u32);

/// A dependency that resolved.
#[derive(Debug, Clone)]
pub struct Edge {
    /// The name the dependent imports it by (spec §3.4).
    pub import_name: String,
    pub package: PackageId,
    /// The entry in the declaring manifest.
    pub span: Span,
}

/// One package of a [`Graph`].
#[derive(Debug, Clone)]
pub struct GraphPackage {
    pub name: String,
    pub import_name: String,
    /// Where its files are read: the root as the caller spelled it, and a
    /// dependency joined onto its declarer's `dir`, such as `../geom`.
    pub dir: PathBuf,
    /// [`real_path`] of `dir`, which identifies the package (spec §3.3).
    pub canonical: PathBuf,
    pub manifest: Manifest,
    /// Its `nova.toml`, in the caller's `FileDb`.
    pub manifest_file: FileId,
    pub has_lib: bool,
    pub has_main: bool,
    pub dependencies: Vec<Edge>,
    /// Read for the root package only (spec §3.3).
    pub dev_dependencies: Vec<Edge>,
}

/// The root package, and every package its dependencies reach.
#[derive(Debug, Clone)]
pub struct Graph {
    /// The root first.
    pub packages: Vec<GraphPackage>,
}

impl Graph {
    pub fn root(&self) -> &GraphPackage {
        &self.packages[0]
    }

    pub fn package(&self, id: PackageId) -> &GraphPackage {
        &self.packages[id.0 as usize]
    }

    /// Every package's canonical directory, the root's first.
    pub fn dirs(&self) -> Vec<PathBuf> {
        self.packages.iter().map(|p| p.canonical.clone()).collect()
    }

    /// The root's own entry through which `id` is first reached, breadth
    /// first, dependencies before dev-dependencies. `None` for the root.
    /// The language server shows a dependency's problems there (spec §6).
    pub fn reached_through(&self, id: PackageId) -> Option<Span> {
        let root = self.root();
        let mut via: HashMap<PackageId, Span> = HashMap::new();
        let mut queue = VecDeque::new();
        for edge in root.dependencies.iter().chain(&root.dev_dependencies) {
            if edge.package != PackageId(0) && !via.contains_key(&edge.package) {
                via.insert(edge.package, edge.span);
                queue.push_back(edge.package);
            }
        }
        while let Some(package) = queue.pop_front() {
            let span = via[&package];
            for edge in &self.package(package).dependencies {
                if edge.package != PackageId(0) && !via.contains_key(&edge.package) {
                    via.insert(edge.package, span);
                    queue.push_back(edge.package);
                }
            }
        }
        via.get(&id).copied()
    }
}

/// Read the package at `root`, the directory holding its `nova.toml` (empty
/// for the current directory), and every package its path dependencies
/// reach. Every manifest goes into `db`, so each diagnostic's label renders.
///
/// The graph is partial on error: what resolved is kept, with the
/// diagnostics. It is `None` only when the root's own manifest cannot be
/// read or parsed.
pub fn graph(root: &Path, db: &mut FileDb) -> (Option<Graph>, Vec<Diagnostic>) {
    graph_from(root, None, db)
}

/// [`graph`], with the root's `nova.toml` taken from `text` when it is
/// given: `nova add` checks a new entry this way before writing it.
pub fn graph_from(
    root: &Path,
    text: Option<&str>,
    db: &mut FileDb,
) -> (Option<Graph>, Vec<Diagnostic>) {
    let mut builder = Builder {
        db,
        diagnostics: Vec::new(),
        packages: Vec::new(),
    };
    let Some(package) = builder.read(root, text) else {
        return (None, builder.diagnostics);
    };
    if !package.has_lib && !package.has_main {
        builder.error(
            "M0013",
            format!(
                "package `{}` has neither src/lib.nova nor src/main.nova",
                package.name
            ),
            package.manifest.package.span,
            "a package needs a library, a program, or both",
        );
    }
    if package.has_lib && is_keyword(&package.import_name) {
        builder.error(
            "M0012",
            format!(
                "library `{}` would be imported as `{}`, which is a keyword",
                package.name, package.import_name
            ),
            package.manifest.package.span,
            "choose another name",
        );
    }
    builder.packages.push(package);
    let mut queue = VecDeque::from([PackageId(0)]);
    while let Some(id) = queue.pop_front() {
        builder.expand(id, &mut queue);
    }
    builder.cycles();
    let Builder {
        diagnostics,
        packages,
        ..
    } = builder;
    (Some(Graph { packages }), diagnostics)
}

/// Whether `name` is one of Nova's keywords, which no import name may be.
fn is_keyword(name: &str) -> bool {
    nova_lexer::KEYWORDS.contains(&name)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Visit {
    New,
    OnPath,
    Done,
}

struct Builder<'a> {
    db: &'a mut FileDb,
    diagnostics: Vec<Diagnostic>,
    packages: Vec<GraphPackage>,
}

impl Builder<'_> {
    fn error(&mut self, code: &str, message: String, at: Span, label: &str) {
        self.diagnostics
            .push(Diagnostic::error(code, message).with_primary_label(at, label));
    }

    /// The package whose manifest is `dir/nova.toml`, or `text` when it is
    /// given, with no edges yet. `None` when the manifest cannot be read or
    /// has errors.
    fn read(&mut self, dir: &Path, text: Option<&str>) -> Option<GraphPackage> {
        let path = dir.join(MANIFEST);
        let source = match text {
            Some(text) => text.to_string(),
            None => match std::fs::read_to_string(&path) {
                Ok(source) => source,
                Err(error) => {
                    self.diagnostics.push(Diagnostic::error(
                        "M0001",
                        format!("cannot read {}: {error}", path.display()),
                    ));
                    return None;
                }
            },
        };
        let file = self.db.add(path.display().to_string(), source.as_str());
        let (manifest, diagnostics) = crate::manifest::parse(&source, file);
        self.diagnostics.extend(diagnostics);
        let manifest = manifest?;
        let canonical = real_path(if dir.as_os_str().is_empty() {
            Path::new(".")
        } else {
            dir
        });
        let src = dir.join("src");
        Some(GraphPackage {
            name: manifest.package.name.clone(),
            import_name: import_name(&manifest.package.name),
            dir: dir.to_path_buf(),
            canonical,
            has_lib: src.join("lib.nova").is_file(),
            has_main: src.join("main.nova").is_file(),
            manifest,
            manifest_file: file,
            dependencies: Vec::new(),
            dev_dependencies: Vec::new(),
        })
    }

    /// Resolve package `id`'s entries, and for the root its
    /// dev-dependencies, queueing each package read for the first time.
    fn expand(&mut self, id: PackageId, queue: &mut VecDeque<PackageId>) {
        let package = &self.packages[id.0 as usize];
        let mut entries: Vec<(Dependency, bool)> = package
            .manifest
            .dependencies
            .iter()
            .cloned()
            .map(|d| (d, false))
            .collect();
        if id == PackageId(0) {
            entries.extend(
                package
                    .manifest
                    .dev_dependencies
                    .iter()
                    .cloned()
                    .map(|d| (d, true)),
            );
        }
        let usable = self.names_ok(id, &entries);
        for ((dependency, dev), ok) in entries.iter().zip(usable) {
            if !ok {
                continue;
            }
            if let Some(edge) = self.edge(id, dependency, queue) {
                let package = &mut self.packages[id.0 as usize];
                if *dev {
                    package.dev_dependencies.push(edge);
                } else {
                    package.dependencies.push(edge);
                }
            }
        }
    }

    /// M0012 (spec §3.4): which of package `id`'s entries may be resolved.
    /// `[dependencies]` and `[dev-dependencies]` count together, since a
    /// test module sees both.
    fn names_ok(&mut self, id: PackageId, entries: &[(Dependency, bool)]) -> Vec<bool> {
        let own = self.packages[id.0 as usize].import_name.clone();
        let mut seen: Vec<(String, String)> = Vec::new();
        let mut ok = Vec::new();
        for (dependency, _) in entries {
            let name = import_name(&dependency.name);
            let problem = if is_keyword(&name) {
                Some(format!(
                    "dependency `{}` would be imported as `{name}`, which is a keyword",
                    dependency.name
                ))
            } else if let Some((_, key)) = seen.iter().find(|(n, _)| *n == name) {
                Some(if *key == dependency.name {
                    format!(
                        "`{}` is in both [dependencies] and [dev-dependencies]",
                        dependency.name
                    )
                } else {
                    format!(
                        "`{key}` and `{}` are both imported as `{name}`",
                        dependency.name
                    )
                })
            } else if id == PackageId(0) && name == own {
                Some(format!(
                    "dependency `{}` would be imported as `{name}`, this package's own name",
                    dependency.name
                ))
            } else {
                None
            };
            match problem {
                Some(message) => {
                    self.error("M0012", message, dependency.span, "choose another name");
                    ok.push(false);
                }
                None => {
                    seen.push((name, dependency.name.clone()));
                    ok.push(true);
                }
            }
        }
        ok
    }

    /// Resolve one entry of package `from` (spec §3.2, §3.3), reading the
    /// package it names if this is the first time.
    fn edge(
        &mut self,
        from: PackageId,
        dependency: &Dependency,
        queue: &mut VecDeque<PackageId>,
    ) -> Option<Edge> {
        let Some(path) = &dependency.path else {
            self.diagnostics.push(
                Diagnostic::error(
                    "M0005",
                    format!(
                        "dependency `{}` is a registry dependency; registry dependencies \
                         arrive with the package index",
                        dependency.name
                    ),
                )
                .with_primary_label(dependency.span, "declared here")
                .with_note("use `path = \"...\"` for a local package"),
            );
            return None;
        };
        let at = dependency.path_span.unwrap_or(dependency.span);
        let declarer = &self.packages[from.0 as usize];
        let dir = declarer.dir.join(path);
        let declarer_name = declarer.name.clone();
        let declarer_canonical = declarer.canonical.clone();
        if !dir.join(MANIFEST).is_file() {
            self.error(
                "M0007",
                format!(
                    "dependency `{}`: {} is not a directory holding a nova.toml",
                    dependency.name,
                    dir.display()
                ),
                at,
                "no nova.toml here",
            );
            return None;
        }
        let canonical = real_path(&dir);
        if canonical == declarer_canonical {
            self.error(
                "M0010",
                format!("dependency cycle: `{declarer_name}` depends on itself"),
                at,
                "this is the package's own directory",
            );
            return None;
        }
        let package = match self.packages.iter().position(|p| p.canonical == canonical) {
            Some(index) => {
                let found = &self.packages[index].name;
                if *found != dependency.name {
                    let message =
                        format!("dependency `{}` is the package `{found}`", dependency.name);
                    self.error(
                        "M0008",
                        message,
                        dependency.span,
                        "an entry's key must be the package's name",
                    );
                    return None;
                }
                // Only the root can be reached again without its library
                // checked: a dev-dependency that depends back on it.
                if !self.packages[index].has_lib {
                    self.error(
                        "M0009",
                        format!(
                            "dependency `{}` is not a library: {} has no src/lib.nova",
                            dependency.name,
                            dir.display()
                        ),
                        at,
                        "a dependency needs src/lib.nova",
                    );
                    return None;
                }
                PackageId(index as u32)
            }
            None => {
                let new = self.read(&dir, None)?;
                if new.name != dependency.name {
                    let message = format!(
                        "dependency `{}` is the package `{}`",
                        dependency.name, new.name
                    );
                    self.error(
                        "M0008",
                        message,
                        dependency.span,
                        "an entry's key must be the package's name",
                    );
                    return None;
                }
                if !new.has_lib {
                    self.error(
                        "M0009",
                        format!(
                            "dependency `{}` is not a library: {} has no src/lib.nova",
                            dependency.name,
                            dir.display()
                        ),
                        at,
                        "a dependency needs src/lib.nova",
                    );
                    return None;
                }
                if let Some(other) = self.packages.iter().find(|p| p.name == new.name) {
                    let message = format!(
                        "two packages named `{}` in one build: {} and {}",
                        new.name,
                        other.dir.display(),
                        dir.display()
                    );
                    self.error(
                        "M0011",
                        message,
                        dependency.span,
                        "the second package of that name",
                    );
                    return None;
                }
                self.packages.push(new);
                let id = PackageId(self.packages.len() as u32 - 1);
                queue.push_back(id);
                id
            }
        };
        Some(Edge {
            import_name: import_name(&dependency.name),
            package,
            span: dependency.span,
        })
    }

    /// M0010 for each cycle of `[dependencies]` edges (spec §3.3), found
    /// depth first from every package.
    fn cycles(&mut self) {
        let count = self.packages.len();
        let mut state = vec![Visit::New; count];
        let mut path = Vec::new();
        for start in 0..count {
            if state[start] == Visit::New {
                self.visit(PackageId(start as u32), &mut state, &mut path);
            }
        }
    }

    fn visit(&mut self, id: PackageId, state: &mut [Visit], path: &mut Vec<PackageId>) {
        state[id.0 as usize] = Visit::OnPath;
        path.push(id);
        for edge in self.packages[id.0 as usize].dependencies.clone() {
            match state[edge.package.0 as usize] {
                Visit::OnPath => {
                    let start = path.iter().position(|p| *p == edge.package).unwrap_or(0);
                    let names: Vec<&str> = path[start..]
                        .iter()
                        .chain([&edge.package])
                        .map(|p| self.packages[p.0 as usize].name.as_str())
                        .collect();
                    let message = format!("dependency cycle: {}", names.join(" -> "));
                    self.error("M0010", message, edge.span, "this entry closes the cycle");
                }
                Visit::New => self.visit(edge.package, state, path),
                Visit::Done => {}
            }
        }
        path.pop();
        state[id.0 as usize] = Visit::Done;
    }
}
