//! The package graph (specs
//! `docs/superpowers/specs/2026-10-08-phase-3-3a-local-packages-design.md`
//! §3 and
//! `docs/superpowers/specs/2026-10-09-phase-3-3b-index-and-publishing-design.md`
//! §5.4): the root package, and every package its dependencies reach,
//! found by path or through `nova.lock` and the registry directory. The
//! graph never touches the network.

use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};

use nova_diagnostics::{Diagnostic, FileDb, FileId, Severity, Span};
use semver::VersionReq;

use crate::lock::{parse_lock, Lock, LOCKFILE};
use crate::manifest::{Dependency, Manifest};
use crate::resolve::Requirement;
use crate::{import_name, index_dir_name, real_path, registry_dir, MANIFEST};

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
    /// Whether it was found in the registry directory (spec 3.3b §5.4).
    /// Its warnings are not shown.
    pub registry: bool,
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

/// Where the graph finds registry packages (spec 3.3b §5.4).
#[derive(Debug, Clone)]
pub struct Offline {
    /// `$NOVA_HOME/registry`. `None` when `$NOVA_HOME` cannot be found.
    pub registry: Option<PathBuf>,
    /// The lock to use instead of the root's `nova.lock`: `nova add`
    /// checks the lock it would write this way.
    pub lock: Option<Lock>,
    /// Whether the root's `[dev-dependencies]` are read. Publishing's
    /// verification reads none (spec 3.3b §6.6).
    pub dev: bool,
}

impl Offline {
    /// The registry directory from this process's environment, the root's
    /// own `nova.lock`, and its dev-dependencies.
    pub fn from_env() -> Offline {
        Offline {
            registry: registry_dir(),
            lock: None,
            dev: true,
        }
    }
}

/// Read the package at `root`, the directory holding its `nova.toml` (empty
/// for the current directory), and every package its dependencies reach.
/// Every manifest goes into `db`, so each diagnostic's label renders.
///
/// The graph is partial on error: what resolved is kept, with the
/// diagnostics. It is `None` only when the root's own manifest cannot be
/// read or parsed.
pub fn graph(root: &Path, db: &mut FileDb) -> (Option<Graph>, Vec<Diagnostic>) {
    graph_with(root, None, &Offline::from_env(), db)
}

/// [`graph`], with the root's `nova.toml` taken from `text` when it is
/// given: `nova add` checks a new entry this way before writing it.
pub fn graph_from(
    root: &Path,
    text: Option<&str>,
    db: &mut FileDb,
) -> (Option<Graph>, Vec<Diagnostic>) {
    graph_with(root, text, &Offline::from_env(), db)
}

/// [`graph_from`], with registry packages found as `offline` says.
pub fn graph_with(
    root: &Path,
    text: Option<&str>,
    offline: &Offline,
    db: &mut FileDb,
) -> (Option<Graph>, Vec<Diagnostic>) {
    let mut builder = Builder::new(root, db, offline, Mode::Graph);
    let Some(package) = builder.read(root, text, false) else {
        return (None, builder.diagnostics);
    };
    builder.check_root(&package);
    builder.packages.push(package);
    builder.walk();
    let Builder {
        diagnostics,
        packages,
        ..
    } = builder;
    (Some(Graph { packages }), diagnostics)
}

/// Every version entry that the root and its path packages declare, for
/// the resolver (spec 3.3b §4.1): who made each one, and the root's entry
/// through which it is reached. The root's dev-dependencies count when
/// `dev` is set. The root's `nova.toml` is `text` when it is given.
///
/// Registry packages are not read: their requirements come from the index.
/// The diagnostics are the manifests' and the path graph's, without M0013,
/// since `nova add` syncs a package before it has a source file.
pub fn requirements(
    root: &Path,
    text: Option<&str>,
    dev: bool,
    db: &mut FileDb,
) -> (Vec<Requirement>, Vec<Diagnostic>) {
    let offline = Offline {
        registry: None,
        lock: None,
        dev,
    };
    let mut builder = Builder::new(root, db, &offline, Mode::Collect);
    let Some(package) = builder.read(root, text, false) else {
        return (Vec::new(), builder.diagnostics);
    };
    builder.check_root(&package);
    builder.packages.push(package);
    builder.walk();
    let graph = Graph {
        packages: std::mem::take(&mut builder.packages),
    };
    let found = builder
        .requirements
        .into_iter()
        .map(|(mut requirement, from)| {
            if let Some(span) = graph.reached_through(from) {
                requirement.span = span;
            }
            requirement
        })
        .collect();
    (found, builder.diagnostics)
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

/// What the builder does with a version entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    /// Find its package through the lock and the registry directory.
    Graph,
    /// Record it as a [`Requirement`].
    Collect,
}

/// The root's `nova.lock`, once read.
enum LockState {
    Missing,
    Unreadable,
    Read(Lock),
}

struct Builder<'a> {
    db: &'a mut FileDb,
    offline: &'a Offline,
    mode: Mode,
    /// The root's directory, where its `nova.lock` is.
    root: PathBuf,
    diagnostics: Vec<Diagnostic>,
    packages: Vec<GraphPackage>,
    /// Read at the first registry entry.
    lock: Option<LockState>,
    /// In [`Mode::Collect`]: each version entry, and the package that made
    /// it.
    requirements: Vec<(Requirement, PackageId)>,
}

impl<'a> Builder<'a> {
    fn new(root: &Path, db: &'a mut FileDb, offline: &'a Offline, mode: Mode) -> Builder<'a> {
        Builder {
            db,
            offline,
            mode,
            root: root.to_path_buf(),
            diagnostics: Vec::new(),
            packages: Vec::new(),
            lock: None,
            requirements: Vec::new(),
        }
    }
}

impl Builder<'_> {
    fn error(&mut self, code: &str, message: String, at: Span, label: &str) {
        self.diagnostics
            .push(Diagnostic::error(code, message).with_primary_label(at, label));
    }

    /// M0013 and M0012 on the root package itself. M0013 is the graph's
    /// alone (see [`requirements`]).
    fn check_root(&mut self, package: &GraphPackage) {
        if self.mode == Mode::Graph && !package.has_lib && !package.has_main {
            self.error(
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
            self.error(
                "M0012",
                format!(
                    "library `{}` would be imported as `{}`, which is a keyword",
                    package.name, package.import_name
                ),
                package.manifest.package.span,
                "choose another name",
            );
        }
    }

    /// Expand every package, breadth first from the root, then find the
    /// cycles.
    fn walk(&mut self) {
        let mut queue = VecDeque::from([PackageId(0)]);
        while let Some(id) = queue.pop_front() {
            self.expand(id, &mut queue);
        }
        self.cycles();
    }

    /// The package whose manifest is `dir/nova.toml`, or `text` when it is
    /// given, with no edges yet. `None` when the manifest cannot be read or
    /// has errors. A registry package's warnings are dropped (spec 3.3b
    /// §5.4).
    fn read(&mut self, dir: &Path, text: Option<&str>, registry: bool) -> Option<GraphPackage> {
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
        self.diagnostics.extend(
            diagnostics
                .into_iter()
                .filter(|d| !registry || d.severity == Severity::Error),
        );
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
            registry,
            manifest,
            manifest_file: file,
            dependencies: Vec::new(),
            dev_dependencies: Vec::new(),
        })
    }

    /// Resolve package `id`'s entries, and for the root its
    /// dev-dependencies when they are read, queueing each package read for
    /// the first time.
    fn expand(&mut self, id: PackageId, queue: &mut VecDeque<PackageId>) {
        let package = &self.packages[id.0 as usize];
        let mut entries: Vec<(Dependency, bool)> = package
            .manifest
            .dependencies
            .iter()
            .cloned()
            .map(|d| (d, false))
            .collect();
        if id == PackageId(0) && self.offline.dev {
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
        let path = match (&dependency.path, &dependency.version) {
            (Some(path), _) => path,
            (None, Some(req)) => return self.registry_edge(from, dependency, &req.clone(), queue),
            // The manifest parser gives every entry one or the other.
            (None, None) => return None,
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
                let new = self.read(&dir, None, false)?;
                self.admit(new, dependency, at, queue)?
            }
        };
        Some(Edge {
            import_name: import_name(&dependency.name),
            package,
            span: dependency.span,
        })
    }

    /// A package read for the first time, checked against the entry that
    /// named it (M0008, M0009, M0011), then added to the graph and queued.
    fn admit(
        &mut self,
        new: GraphPackage,
        dependency: &Dependency,
        at: Span,
        queue: &mut VecDeque<PackageId>,
    ) -> Option<PackageId> {
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
                    new.dir.display()
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
                new.dir.display()
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
        Some(id)
    }

    /// Resolve a version entry of package `from` through the lock and the
    /// registry directory (spec 3.3b §5.4), or in [`Mode::Collect`] record
    /// it.
    fn registry_edge(
        &mut self,
        from: PackageId,
        dependency: &Dependency,
        req: &VersionReq,
        queue: &mut VecDeque<PackageId>,
    ) -> Option<Edge> {
        if self.mode == Mode::Collect {
            let by = self.packages[from.0 as usize].name.clone();
            let requirement = Requirement {
                name: dependency.name.clone(),
                req: req.clone(),
                by,
                span: dependency.span,
            };
            self.requirements.push((requirement, from));
            return None;
        }
        let name = &dependency.name;
        let found = self
            .lock()
            .map(|lock| (lock.index.clone(), lock.find(name).cloned()));
        let Some((index, Some(locked))) = found else {
            self.missing(
                dependency,
                format!("dependency `{name}` is not downloaded yet; run `nova fetch`"),
            );
            return None;
        };
        if !req.matches(&locked.version) {
            self.missing(
                dependency,
                format!(
                    "dependency `{name}` is locked at {}, which does not meet `{req}`; run \
                     `nova fetch`",
                    locked.version
                ),
            );
            return None;
        }
        let Some(registry) = self.offline.registry.clone() else {
            self.missing(
                dependency,
                format!(
                    "dependency `{name}` cannot be found: cannot place the package cache: set \
                     NOVA_HOME to a writable directory"
                ),
            );
            return None;
        };
        let dir = registry
            .join("src")
            .join(index_dir_name(&index))
            .join(format!("{name}-{}", locked.version));
        if !dir.join(MANIFEST).is_file() {
            self.missing(
                dependency,
                format!("dependency `{name}` is not downloaded yet; run `nova fetch`"),
            );
            return None;
        }
        let canonical = real_path(&dir);
        let package = match self.packages.iter().position(|p| p.canonical == canonical) {
            Some(index) => PackageId(index as u32),
            None => {
                let new = self.read(&dir, None, true)?;
                // Unpacking checked both (spec §5.2); a copy edited since is
                // refused here.
                if let Some(entry) = new.manifest.dependencies.iter().find(|d| d.path.is_some()) {
                    self.error(
                        "M0017",
                        format!(
                            "downloaded package `{name}` {} has a path dependency `{}`",
                            locked.version, entry.name
                        ),
                        dependency.span,
                        "declared here",
                    );
                    return None;
                }
                let mut names: Vec<String> = new
                    .manifest
                    .dependencies
                    .iter()
                    .map(|d| d.name.clone())
                    .collect();
                names.sort();
                if names != locked.dependencies {
                    self.missing(
                        dependency,
                        format!(
                            "the downloaded copy of `{name}` {} does not match nova.lock; delete \
                             {} and run `nova fetch`",
                            locked.version,
                            dir.display()
                        ),
                    );
                    return None;
                }
                self.admit(new, dependency, dependency.span, queue)?
            }
        };
        Some(Edge {
            import_name: import_name(name),
            package,
            span: dependency.span,
        })
    }

    /// M0005 on `dependency`: a registry entry the cache cannot satisfy.
    fn missing(&mut self, dependency: &Dependency, message: String) {
        self.diagnostics.push(
            Diagnostic::error("M0005", message)
                .with_primary_label(dependency.span, "declared here"),
        );
    }

    /// The lock registry entries are found through: [`Offline::lock`], else
    /// the root's `nova.lock`, read once. M0016 when it cannot be read.
    fn lock(&mut self) -> Option<&Lock> {
        if self.lock.is_none() {
            let state = match &self.offline.lock {
                Some(lock) => LockState::Read(lock.clone()),
                None => {
                    let path = self.root.join(LOCKFILE);
                    match std::fs::read_to_string(&path) {
                        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                            LockState::Missing
                        }
                        Err(error) => {
                            self.diagnostics.push(
                                Diagnostic::error(
                                    "M0016",
                                    format!("nova.lock cannot be read: {error}"),
                                )
                                .with_note("delete nova.lock, or run `nova update`"),
                            );
                            LockState::Unreadable
                        }
                        Ok(text) => {
                            let file = self.db.add(path.display().to_string(), text.as_str());
                            match parse_lock(&text, file) {
                                Ok(lock) => LockState::Read(lock),
                                Err(diagnostic) => {
                                    self.diagnostics.push(diagnostic);
                                    LockState::Unreadable
                                }
                            }
                        }
                    }
                }
            };
            self.lock = Some(state);
        }
        match &self.lock {
            Some(LockState::Read(lock)) => Some(lock),
            _ => None,
        }
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
