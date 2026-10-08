//! The front end for the language server (spec
//! `docs/superpowers/specs/2026-10-08-phase-3-2-lsp-core-design.md` §3).
//!
//! `analyze` runs what `check_file` runs, with three differences:
//! - its sources come through [`Sources`], so an editor's unsaved buffers
//!   are read;
//! - with `keep_going`, every stage runs whatever the earlier ones found;
//! - its diagnostics are returned rather than printed.
//!
//! The CLI's entry points share [`crate::program::load_program`] with it,
//! through [`DiskSources`], and keep their own staged behaviour.

use std::path::{Path, PathBuf};

use nova_diagnostics::{Diagnostic, FileDb, FileId, Severity};
use nova_pm::{Graph, PackageId};
use nova_resolver::{Definitions, ModuleSource};
use nova_typeck::{CheckOptions, ProbePoint, ProbeResult};

use crate::program::{load_program, Load, Program};

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
    /// Each module's package, parallel to `modules`. `None` for a loose
    /// module (spec 3.3a §6).
    pub module_packages: Vec<Option<PackageId>>,
    /// The package graph, as far as it resolved.
    pub graph: Option<Graph>,
    pub definitions: Option<Definitions>,
    /// The typed module, partial when errors were found.
    pub module: Option<nova_hir::Module>,
    pub probe: ProbeResult,
}

/// Run the front end on `entry` for the language server (spec §3): the
/// program [`Program::for_file`] finds for it.
pub fn analyze(
    entry: &Path,
    sources: &dyn Sources,
    options: &Options,
) -> std::io::Result<Analysis> {
    analyze_program(Program::for_file(entry), sources, options)
}

/// Run the front end on `program` (spec 3.3a §4.4, §6). MIR runs only
/// when the program runs and `options.module_only` is off.
pub fn analyze_program(
    program: Program,
    sources: &dyn Sources,
    options: &Options,
) -> std::io::Result<Analysis> {
    let Program {
        mut db,
        graph,
        roots,
        runs,
        mut diagnostics,
    } = program;
    let stop = |diags: &[Diagnostic]| !options.keep_going && has_error(diags);
    let load = if stop(&diagnostics) {
        Load::default()
    } else {
        load_program(graph.as_ref(), &roots, sources, &mut db)?
    };
    diagnostics.extend(load.diagnostics);
    let dropped = load.dropped;
    let mut modules = load.modules;
    let mut analysis = Analysis {
        db,
        diagnostics: Vec::new(),
        modules: modules.iter().map(|m| (m.file, m.path.clone())).collect(),
        module_packages: modules.iter().map(|m| m.package).collect(),
        graph,
        definitions: None,
        module: None,
        probe: ProbeResult::default(),
    };
    if stop(&diagnostics) {
        analysis.diagnostics = diagnostics;
        return Ok(analysis);
    }

    if !options.tests {
        crate::strip_test_functions(&mut modules);
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
    let module_sources: Vec<ModuleSource> = modules
        .iter()
        .map(|m| ModuleSource {
            name: m.name.clone(),
            file: &m.ast,
            imports: m.imports.clone(),
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
    // MIR lowering assumes a well-formed program (spec §3.3), and a
    // program without an entry `main` is checked as a module (3.3a §4.4).
    let module_only = options.module_only || !runs;
    if !module_only && !has_error(&diagnostics) {
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
