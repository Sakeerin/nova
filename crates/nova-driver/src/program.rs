//! What the driver loads (spec
//! `docs/superpowers/specs/2026-10-08-phase-3-3a-local-packages-design.md`
//! §4): a [`Program`] names its roots and carries the package graph, and
//! [`load_program`] decides what each `import` names.

use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};

use nova_diagnostics::{Diagnostic, FileDb, FileId, Severity, Span};
use nova_pm::{Edge, Graph, GraphPackage, Offline, PackageId};
use nova_resolver::ImportTarget;

use crate::analyze::Sources;

/// The roots a command takes (spec §4.4's table), each only if it exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Roots {
    /// `src/main.nova`: `nova run` and `nova build`.
    Program,
    /// `src/main.nova`, then `src/lib.nova`: `nova check`.
    Check,
    /// Those, then `tests/*.nova` sorted by name: `nova test` and the
    /// language server.
    Test,
}

/// Where a root is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RootKind {
    /// Directly in the root package's `src/`.
    Src,
    /// Directly in the root package's `tests/`.
    Tests,
    /// In no package (spec §4.1).
    Loose,
}

/// A file the loader starts from.
#[derive(Debug, Clone)]
pub struct Root {
    pub path: PathBuf,
    pub kind: RootKind,
}

/// What to load (spec §4.4).
pub struct Program {
    /// Every manifest the graph read. The loader adds every source file.
    pub db: FileDb,
    /// `None` for a loose program, or when the root's manifest cannot be
    /// read or parsed.
    pub graph: Option<Graph>,
    /// The entry first.
    pub roots: Vec<Root>,
    /// Whether MIR runs: the entry is `src/main.nova`, or a loose file.
    pub runs: bool,
    /// The graph's diagnostics.
    pub diagnostics: Vec<Diagnostic>,
}

impl Program {
    /// A loose program: `path`, and the files beside it that it imports.
    pub fn loose(path: &Path) -> Program {
        Program {
            db: FileDb::new(),
            graph: None,
            roots: vec![Root {
                path: path.to_path_buf(),
                kind: RootKind::Loose,
            }],
            runs: true,
            diagnostics: Vec::new(),
        }
    }

    /// The program whose entry is `path` (spec §4.1). It is a module of its
    /// package when it is directly in a package's `src/` or `tests/`, and
    /// otherwise loose, reading no manifest.
    pub fn for_file(path: &Path) -> Program {
        let Some((root, kind)) = package_of(path) else {
            return Program::loose(path);
        };
        let mut db = FileDb::new();
        let (graph, diagnostics) = nova_pm::graph(&root, &mut db);
        let runs = kind == RootKind::Src && path.file_stem().is_some_and(|stem| stem == "main");
        Program {
            db,
            graph,
            roots: vec![Root {
                path: path.to_path_buf(),
                kind,
            }],
            runs,
            diagnostics,
        }
    }

    /// The package whose `nova.toml` is in `root` (empty for the current
    /// directory), with the roots `roots` names that exist.
    pub fn for_package(root: &Path, roots: Roots) -> Program {
        Program::for_package_in(root, roots, &Offline::from_env())
    }

    /// [`Program::for_package`], with registry packages found as `offline`
    /// says (spec 3.3b §5.4): tests pass the registry directory, and
    /// publishing's verification reads no dev-dependencies.
    pub fn for_package_in(root: &Path, roots: Roots, offline: &Offline) -> Program {
        let mut db = FileDb::new();
        let (graph, diagnostics) = nova_pm::graph_with(root, None, offline, &mut db);
        let mut list = Vec::new();
        let main = under(root, "src/main.nova");
        if main.is_file() {
            list.push(Root {
                path: main,
                kind: RootKind::Src,
            });
        }
        let runs = !list.is_empty();
        let lib = under(root, "src/lib.nova");
        if roots != Roots::Program && lib.is_file() {
            list.push(Root {
                path: lib,
                kind: RootKind::Src,
            });
        }
        if roots == Roots::Test {
            list.extend(
                test_files(&under(root, "tests"))
                    .into_iter()
                    .map(|path| Root {
                        path,
                        kind: RootKind::Tests,
                    }),
            );
        }
        Program {
            db,
            graph,
            roots: list,
            runs,
            diagnostics,
        }
    }

    /// Whether the graph found an error.
    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|d| d.severity == Severity::Error)
    }

    /// The root package's `nova.toml` in `db`, once read. The language
    /// server publishes the graph's problems under it (spec §6).
    pub fn manifest(&self) -> Option<FileId> {
        match &self.graph {
            Some(graph) => Some(graph.root().manifest_file),
            // Without a graph only the root's manifest was read, and each of
            // its diagnostics is labelled in it.
            None => self
                .diagnostics
                .iter()
                .flat_map(|d| &d.labels)
                .map(|label| label.span.file)
                .next(),
        }
    }
}

/// `relative`, a `/`-separated path, under `root`. Under an empty root it
/// is `relative` as written, so `nova run` at a project's root still names
/// `src/main.nova` on every system, as it always has.
fn under(root: &Path, relative: &str) -> PathBuf {
    if root.as_os_str().is_empty() {
        PathBuf::from(relative)
    } else {
        relative
            .split('/')
            .fold(root.to_path_buf(), |path, part| path.join(part))
    }
}

/// The package `path` is a module of, and where in it, when `path` is
/// directly in the `src/` or `tests/` of a directory holding a `nova.toml`
/// (spec §4.1). The language server finds its projects the same way.
pub fn package_of(path: &Path) -> Option<(PathBuf, RootKind)> {
    let parent = path.parent()?;
    let kind = match parent.file_name()?.to_str()? {
        "src" => RootKind::Src,
        "tests" => RootKind::Tests,
        _ => return None,
    };
    let root = parent.parent()?;
    root.join(nova_pm::MANIFEST)
        .is_file()
        .then(|| (root.to_path_buf(), kind))
}

/// The `*.nova` files directly in `dir`, sorted by name (spec §4.4). A
/// missing directory has none.
fn test_files(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut files: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "nova") && path.is_file())
        .collect();
    files.sort();
    files
}

/// The files of `modules` that belong to a registry package (spec 3.3b
/// §5.4). A registry manifest's warnings never leave the graph.
pub(crate) fn registry_files(graph: Option<&Graph>, modules: &[Loaded]) -> HashSet<FileId> {
    let Some(graph) = graph else {
        return HashSet::new();
    };
    modules
        .iter()
        .filter(|m| m.package.is_some_and(|p| graph.package(p).registry))
        .map(|m| m.file)
        .collect()
}

/// Whether `d` is a warning about a registry package alone, which is not
/// shown, as Cargo caps a dependency's lints: it has a label, and every
/// label is in `registry`.
pub(crate) fn a_dependencys_warning(d: &Diagnostic, registry: &HashSet<FileId>) -> bool {
    d.severity == Severity::Warning
        && !d.labels.is_empty()
        && d.labels
            .iter()
            .all(|label| registry.contains(&label.span.file))
}

/// One module [`load_program`] read.
pub(crate) struct Loaded {
    /// A label for messages (spec §4.5): `utils`, `tests/utils`, `geom` or
    /// `geom/utils`.
    pub name: String,
    pub path: PathBuf,
    pub file: FileId,
    pub ast: nova_ast::File,
    /// Its package. `None` for a loose module.
    pub package: Option<PackageId>,
    /// Whether it is directly in its package's `tests/`.
    pub test_file: bool,
    /// What each of its imports names, by first segment (spec §4.3).
    pub imports: HashMap<String, ImportTarget>,
}

/// What [`load_program`] found.
#[derive(Default)]
pub(crate) struct Load {
    /// In load order: `modules[i]` is `ModuleId(i)`.
    pub modules: Vec<Loaded>,
    /// Lex, parse and import diagnostics, in load order.
    pub diagnostics: Vec<Diagnostic>,
    /// The names of the items the parser dropped.
    pub dropped: Vec<String>,
}

/// A module's identity (spec §4.2).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum Place {
    /// A file directly in a package's `src/`, or `tests/` when `tests`.
    Package {
        package: PackageId,
        tests: bool,
        stem: String,
    },
    /// A loose file.
    Loose { dir: PathBuf, stem: String },
}

impl Place {
    fn package(&self) -> Option<PackageId> {
        match self {
            Place::Package { package, .. } => Some(*package),
            Place::Loose { .. } => None,
        }
    }

    fn is_test(&self) -> bool {
        matches!(self, Place::Package { tests: true, .. })
    }
}

/// What an import names, before every module is loaded.
enum Pending {
    Place(Place),
    Reported,
}

/// What [`Loader::find`] found for one import.
enum Found {
    /// A module: its identity and its file.
    Module(Place, PathBuf),
    /// An error, reported here (spec §4.3, rules 3–5).
    Reported(Diagnostic),
    /// Nothing: the resolver reports "cannot find module".
    Nothing,
}

/// Read, lex and parse every module the roots reach, breadth first from the
/// roots in order (spec §4.4), deciding what each import names (§4.3).
///
/// A module that cannot be read is skipped, and an import of it is then
/// "cannot find module". `Err` only when the entry itself cannot be read.
/// Without a graph, every root is loose.
pub(crate) fn load_program(
    graph: Option<&Graph>,
    roots: &[Root],
    sources: &dyn Sources,
    db: &mut FileDb,
) -> std::io::Result<Load> {
    let mut loader = Loader {
        graph,
        sources,
        listings: HashMap::new(),
    };
    let mut load = Load::default();
    let mut pending: Vec<Vec<(String, Pending)>> = Vec::new();
    let mut places: HashMap<Place, usize> = HashMap::new();
    let mut tried: HashSet<Place> = HashSet::new();
    let mut queue: VecDeque<(Place, PathBuf)> = roots
        .iter()
        .map(|root| (loader.root_place(root), root.path.clone()))
        .collect();
    let mut is_entry = true;

    while let Some((place, path)) = queue.pop_front() {
        let entry = std::mem::replace(&mut is_entry, false);
        if !tried.insert(place.clone()) {
            continue;
        }
        let source = match sources.read(&path) {
            Ok(source) => source,
            Err(e) if entry => return Err(e),
            Err(_) => continue,
        };
        let file = db.add(path.display().to_string(), source.as_str());

        let (tokens, lex_errors) = nova_lexer::lex(&source, file);
        load.diagnostics.extend(lex_errors.iter().map(|e| {
            Diagnostic::error("L0001", e.to_string()).with_primary_label(e.span(), "here")
        }));
        let parsed = nova_parser::parse_recovering(&tokens, file);
        load.diagnostics.extend(parsed.errors.iter().map(|e| {
            Diagnostic::error("P0001", e.to_string()).with_primary_label(e.span(), "here")
        }));
        load.dropped
            .extend(parsed.dropped.into_iter().map(|n| n.value));

        // Only a single-segment import names a module; the resolver rejects
        // the others.
        let mut imports = Vec::new();
        for item in &parsed.file.items {
            let nova_ast::Item::Import(import) = &item.value else {
                continue;
            };
            let [segment] = import.path.value.segments.as_slice() else {
                continue;
            };
            let name = segment.value.clone();
            match loader.find(&place, &name, import.path.span, db) {
                Found::Module(target, target_path) => {
                    queue.push_back((target.clone(), target_path));
                    imports.push((name, Pending::Place(target)));
                }
                Found::Reported(diagnostic) => {
                    load.diagnostics.push(diagnostic);
                    imports.push((name, Pending::Reported));
                }
                Found::Nothing => {}
            }
        }
        places.insert(place.clone(), load.modules.len());
        load.modules.push(Loaded {
            name: loader.label(&place),
            path,
            file,
            ast: parsed.file,
            package: place.package(),
            test_file: place.is_test(),
            imports: HashMap::new(),
        });
        pending.push(imports);
    }

    for (module, imports) in load.modules.iter_mut().zip(pending) {
        for (name, target) in imports {
            let target = match target {
                Pending::Place(place) => match places.get(&place) {
                    Some(&index) => ImportTarget::Module(index),
                    // It could not be read.
                    None => continue,
                },
                Pending::Reported => ImportTarget::Reported,
            };
            module.imports.insert(name, target);
        }
    }
    Ok(load)
}

struct Loader<'a> {
    graph: Option<&'a Graph>,
    sources: &'a dyn Sources,
    /// Each directory's file names, read once (spec §4.3, decision 5).
    listings: HashMap<PathBuf, Vec<String>>,
}

impl<'a> Loader<'a> {
    fn root_place(&self, root: &Root) -> Place {
        let stem = crate::FrontendContext::module_name(&root.path);
        match (root.kind, self.graph) {
            (RootKind::Src, Some(_)) => Place::Package {
                package: PackageId(0),
                tests: false,
                stem,
            },
            (RootKind::Tests, Some(_)) => Place::Package {
                package: PackageId(0),
                tests: true,
                stem,
            },
            _ => Place::Loose {
                dir: root
                    .path
                    .parent()
                    .map(Path::to_path_buf)
                    .unwrap_or_default(),
                stem,
            },
        }
    }

    /// The module's label (spec §4.5).
    fn label(&self, place: &Place) -> String {
        match place {
            Place::Loose { stem, .. } => stem.clone(),
            Place::Package {
                package: PackageId(0),
                tests,
                stem,
            } => {
                if *tests {
                    format!("tests/{stem}")
                } else {
                    stem.clone()
                }
            }
            Place::Package { package, stem, .. } => {
                let import = &self.graph().package(*package).import_name;
                if stem == "lib" {
                    import.clone()
                } else {
                    format!("{import}/{stem}")
                }
            }
        }
    }

    /// The graph, borrowed for `'a` rather than from `self`, so `has` can
    /// run while it is held.
    fn graph(&self) -> &'a Graph {
        self.graph.expect("a package's module has a graph")
    }

    /// What `import name`, at `at` in the module at `from`, names (spec
    /// §4.3).
    fn find(&mut self, from: &Place, name: &str, at: Span, db: &FileDb) -> Found {
        let (package, tests) = match from {
            Place::Loose { dir, .. } => {
                if !self.has(dir, name) {
                    return Found::Nothing;
                }
                let place = Place::Loose {
                    dir: dir.clone(),
                    stem: name.to_string(),
                };
                return Found::Module(place, dir.join(format!("{name}.nova")));
            }
            Place::Package { package, tests, .. } => (*package, *tests),
        };
        let graph = self.graph();
        let dir = module_dir(graph.package(package), tests);
        let edge = visible_edges(graph, package, tests)
            .into_iter()
            .find(|edge| edge.import_name == name);
        let file = self.has(&dir, name).then(|| {
            (
                Place::Package {
                    package,
                    tests,
                    stem: name.to_string(),
                },
                dir.join(format!("{name}.nova")),
            )
        });
        match (file, edge) {
            (Some((_, path)), Some(edge)) => Found::Reported(clash(name, &path, &edge, at, db)),
            (Some((place, path)), None) => Found::Module(place, path),
            (None, Some(edge)) => {
                let target = graph.package(edge.package);
                if !target.has_lib {
                    // The package itself, seen from its own tests (rule 5).
                    return Found::Reported(
                        Diagnostic::error("E0001", format!("cannot find module `{name}`"))
                            .with_primary_label(at, "no such module")
                            .with_note(format!("`{}` has no src/lib.nova", target.name)),
                    );
                }
                Found::Module(
                    Place::Package {
                        package: edge.package,
                        tests: false,
                        stem: "lib".to_string(),
                    },
                    module_dir(target, false).join("lib.nova"),
                )
            }
            (None, None) => {
                let dev = !tests
                    && package == PackageId(0)
                    && graph
                        .root()
                        .dev_dependencies
                        .iter()
                        .any(|edge| edge.import_name == name);
                if dev {
                    Found::Reported(
                        Diagnostic::error("E0001", format!("cannot find module `{name}`"))
                            .with_primary_label(at, "no such module")
                            .with_note(format!(
                                "`{name}` is a dev-dependency, which only `tests/` files can import"
                            )),
                    )
                } else {
                    Found::Nothing
                }
            }
        }
    }

    /// Whether `dir` holds `<stem>.nova`, its name matching exactly (spec
    /// §4.3). A name the disk does not hold in any case may be an editor's
    /// unsaved buffer, read through `Sources`.
    ///
    /// An empty `dir`, a loose entry named without one (`nova run
    /// main.nova`), is the current directory: `read_dir("")` fails on every
    /// system, which would leave the case-insensitive read to decide.
    fn has(&mut self, dir: &Path, stem: &str) -> bool {
        let name = format!("{stem}.nova");
        let listed = if dir.as_os_str().is_empty() {
            Path::new(".")
        } else {
            dir
        };
        let listing = self.listings.entry(dir.to_path_buf()).or_insert_with(|| {
            std::fs::read_dir(listed)
                .map(|entries| {
                    entries
                        .filter_map(Result::ok)
                        .filter_map(|entry| entry.file_name().into_string().ok())
                        .collect()
                })
                .unwrap_or_default()
        });
        if listing.contains(&name) {
            return true;
        }
        if listing.iter().any(|n| n.eq_ignore_ascii_case(&name)) {
            return false;
        }
        self.sources.read(&dir.join(&name)).is_ok()
    }
}

/// A package's `src/`, or `tests/` when `tests`.
fn module_dir(package: &GraphPackage, tests: bool) -> PathBuf {
    package.dir.join(if tests { "tests" } else { "src" })
}

/// The dependencies a module of `package` sees (spec §4.3). A test module
/// also sees the root's dev-dependencies, and the root itself by its import
/// name. Only the root's test modules are ever loaded.
fn visible_edges(graph: &Graph, package: PackageId, tests: bool) -> Vec<Edge> {
    let p = graph.package(package);
    let mut edges = p.dependencies.clone();
    if tests {
        edges.extend(p.dev_dependencies.iter().cloned());
        edges.push(Edge {
            import_name: p.import_name.clone(),
            package,
            span: p.manifest.package.span,
        });
    }
    edges
}

/// E0004 (spec §4.3, rule 3): `name` is both `file` and the dependency
/// `edge`. The message names both, so it reads in full where a related
/// label cannot show.
fn clash(name: &str, file: &Path, edge: &Edge, at: Span, db: &FileDb) -> Diagnostic {
    let manifest = db.get_name(edge.span.file).unwrap_or("nova.toml");
    let line = db
        .location(edge.span.file, edge.span.start)
        .map_or(0, |(line, _)| line);
    Diagnostic::error(
        "E0004",
        format!(
            "`{name}` is both a module of this package and a dependency: {}, and the entry \
             at {manifest}:{line}",
            file.display()
        ),
    )
    .with_primary_label(at, "this import is ambiguous")
    .with_secondary_label(edge.span, "the dependency")
}
