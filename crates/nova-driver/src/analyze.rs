//! The front end for the language server (spec
//! `docs/superpowers/specs/2026-10-08-phase-3-2-lsp-core-design.md` §3).
//!
//! `analyze` runs what `check_file` runs, with three differences:
//! - its sources come through [`Sources`], so an editor's unsaved buffers
//!   are read;
//! - with `keep_going`, every stage runs whatever the earlier ones found;
//! - its diagnostics are returned rather than printed.
//!
//! The CLI's entry points share [`load_program`] with it, through
//! [`DiskSources`], and keep their own staged behaviour.

use std::collections::{HashSet, VecDeque};
use std::path::{Path, PathBuf};

use nova_diagnostics::{Diagnostic, FileDb, FileId, Severity};
use nova_resolver::{Definitions, ModuleSource};
use nova_typeck::{CheckOptions, ProbePoint, ProbeResult};

/// Where `analyze` reads a module's text.
pub trait Sources {
    /// The text of `path`: an editor's buffer if one is open, else the file.
    fn read(&self, path: &Path) -> std::io::Result<String>;

    /// Whether `a` and `b` name the same file. The default compares them as
    /// written; the language server compares normalised paths.
    fn same_file(&self, a: &Path, b: &Path) -> bool {
        a == b
    }
}

/// Reads every module from disk: the CLI's sources.
pub struct DiskSources;

impl Sources for DiskSources {
    fn read(&self, path: &Path) -> std::io::Result<String> {
        std::fs::read_to_string(path)
    }
}

/// What `analyze` does beyond checking.
#[derive(Debug, Clone, Default)]
pub struct Options {
    /// Run every stage whatever the earlier ones found (spec §3.2).
    pub keep_going: bool,
    /// Keep `@test` functions and put `std/test` in scope, as `nova test`
    /// does (spec §13, decision 14).
    pub tests: bool,
    /// Check the entry as a module, not a program: no MIR lowering (spec
    /// §3.3).
    pub module_only: bool,
    /// Record what is at this place (spec §4.1).
    pub probe: Option<Probe>,
}

/// A place in one of the program's files.
#[derive(Debug, Clone)]
pub struct Probe {
    pub path: PathBuf,
    pub offset: u32,
}

/// What `analyze` found.
pub struct Analysis {
    /// Every file read, std included.
    pub db: FileDb,
    pub diagnostics: Vec<Diagnostic>,
    /// The program's own modules, by the paths they were read from, in load
    /// order: `modules[i]` is `ModuleId(i)`.
    pub modules: Vec<(FileId, PathBuf)>,
    pub definitions: Option<Definitions>,
    /// The typed module, partial when errors were found.
    pub module: Option<nova_hir::Module>,
    pub probe: ProbeResult,
}

/// One module [`load_program`] read.
pub(crate) struct Loaded {
    pub name: String,
    pub path: PathBuf,
    pub file: FileId,
    pub ast: nova_ast::File,
}

/// Read, lex and parse `entry` and every module it transitively imports,
/// each `<name>.nova` beside the entry. A module that cannot be read is
/// skipped: the resolver reports its `import`.
///
/// Returns:
/// - the modules, in load order;
/// - their lex and parse diagnostics, in the order the CLI has always
///   printed them;
/// - the names of the items the parser dropped.
///
/// `Err` only when the entry itself cannot be read.
pub(crate) fn load_program(
    entry: &Path,
    sources: &dyn Sources,
    db: &mut FileDb,
) -> std::io::Result<(Vec<Loaded>, Vec<Diagnostic>, Vec<String>)> {
    let dir = entry.parent().map(Path::to_path_buf).unwrap_or_default();
    let mut out: Vec<Loaded> = Vec::new();
    let mut diagnostics = Vec::new();
    let mut dropped = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    let mut queue: VecDeque<(String, PathBuf)> = VecDeque::new();
    queue.push_back((
        crate::FrontendContext::module_name(entry),
        entry.to_path_buf(),
    ));

    while let Some((name, path)) = queue.pop_front() {
        if !seen.insert(name.clone()) {
            continue;
        }
        let is_entry = out.is_empty() && seen.len() == 1;
        let source = match sources.read(&path) {
            Ok(s) => s,
            Err(e) if is_entry => return Err(e),
            // A missing imported module: skip; the resolver flags the import.
            Err(_) => continue,
        };
        let file = db.add(path.display().to_string(), source.as_str());

        let (tokens, lex_errors) = nova_lexer::lex(&source, file);
        diagnostics.extend(lex_errors.iter().map(|e| {
            Diagnostic::error("L0001", e.to_string()).with_primary_label(e.span(), "here")
        }));
        let parsed = nova_parser::parse_recovering(&tokens, file);
        diagnostics.extend(parsed.errors.iter().map(|e| {
            Diagnostic::error("P0001", e.to_string()).with_primary_label(e.span(), "here")
        }));
        dropped.extend(parsed.dropped.into_iter().map(|n| n.value));

        // Queue imported modules, resolved beside the entry. Only
        // single-segment imports name a module file; the resolver rejects
        // the others.
        for item in &parsed.file.items {
            if let nova_ast::Item::Import(imp) = &item.value {
                if let [seg] = imp.path.value.segments.as_slice() {
                    let mod_name = seg.value.clone();
                    if !seen.contains(&mod_name) {
                        let mod_path = dir.join(format!("{mod_name}.nova"));
                        queue.push_back((mod_name, mod_path));
                    }
                }
            }
        }
        out.push(Loaded {
            name,
            path,
            file,
            ast: parsed.file,
        });
    }
    Ok((out, diagnostics, dropped))
}

/// Run the front end on `entry` for the language server (spec §3).
pub fn analyze(
    entry: &Path,
    sources: &dyn Sources,
    options: &Options,
) -> std::io::Result<Analysis> {
    let mut db = FileDb::new();
    let (loaded, mut diagnostics, dropped) = load_program(entry, sources, &mut db)?;
    let mut analysis = Analysis {
        db,
        diagnostics: Vec::new(),
        modules: loaded.iter().map(|m| (m.file, m.path.clone())).collect(),
        definitions: None,
        module: None,
        probe: ProbeResult::default(),
    };
    let stop = |diags: &[Diagnostic]| !options.keep_going && has_error(diags);
    if stop(&diagnostics) {
        analysis.diagnostics = diagnostics;
        return Ok(analysis);
    }

    let mut files: Vec<(String, nova_ast::File)> =
        loaded.into_iter().map(|m| (m.name, m.ast)).collect();
    if !options.tests {
        crate::strip_test_functions(&mut files);
    }
    let std_files: Vec<FileId> = nova_resolver::STD_MODULES
        .iter()
        .map(|&(name, src)| {
            let short = name.strip_prefix("$std.").unwrap_or(name);
            analysis.db.add(format!("<std/{short}>"), src)
        })
        .collect();
    let extra_std = options.tests.then(|| {
        let (name, src) = nova_resolver::STD_TEST_MODULE;
        let short = name.strip_prefix("$std.").unwrap_or(name);
        let file = analysis.db.add(format!("<std/{short}>"), src);
        (nova_resolver::STD_TEST_MODULE, file)
    });
    let module_sources: Vec<ModuleSource> = files
        .iter()
        .map(|(name, file)| ModuleSource {
            name: name.clone(),
            file,
        })
        .collect();
    let resolved = nova_resolver::resolve_program(&module_sources, &std_files, extra_std);
    diagnostics.extend(resolved.diagnostics);
    if stop(&diagnostics) {
        analysis.diagnostics = diagnostics;
        analysis.definitions = Some(resolved.definitions);
        return Ok(analysis);
    }

    let probe = options.probe.as_ref().and_then(|p| {
        analysis
            .modules
            .iter()
            .find(|(_, path)| sources.same_file(path, &p.path))
            .map(|(file, _)| ProbePoint {
                file: *file,
                offset: p.offset,
            })
    });
    let checked = nova_typeck::check_with(
        &resolved.file,
        &resolved.definitions,
        &CheckOptions { probe },
    );
    diagnostics.extend(checked.diagnostics);
    // MIR lowering assumes a well-formed program (spec §3.3).
    if !options.module_only && !has_error(&diagnostics) {
        if let Err(mir) = nova_mir::lower_module(&checked.module) {
            diagnostics.extend(mir);
        }
    }
    if options.keep_going {
        diagnostics.retain(|d| !about_a_dropped_name(d, &dropped));
    }
    analysis.diagnostics = diagnostics;
    analysis.definitions = Some(resolved.definitions);
    analysis.module = Some(checked.module);
    analysis.probe = checked.probe;
    Ok(analysis)
}

fn has_error(diagnostics: &[Diagnostic]) -> bool {
    diagnostics.iter().any(|d| d.severity == Severity::Error)
}

/// Whether `d` is an E0001 about an item the parser dropped mid-edit (spec
/// §3.2): its message's first backticked name is that item's.
///
/// Every E0001 message names what it cannot find first, in backticks:
/// - "cannot find `x` in this scope";
/// - "cannot find function `f` in this scope";
/// - "cannot find type `T`", "cannot find record `R`", "cannot find trait
///   `T`";
/// - "`x` is not a public item of module `m`".
///
/// `tests/analyze.rs` pins each of them.
fn about_a_dropped_name(d: &Diagnostic, dropped: &[String]) -> bool {
    if d.code != "E0001" {
        return false;
    }
    let mut parts = d.message.split('`');
    parts.next();
    parts
        .next()
        .is_some_and(|name| dropped.iter().any(|n| n == name))
}
