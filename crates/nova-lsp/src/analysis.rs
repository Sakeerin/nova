//! Which analysis answers a request (spec
//! `docs/superpowers/specs/2026-10-10-phase-3-4a-navigation-design.md` §4).
//! Completion and navigation share it.

use std::path::{Path, PathBuf};

use nova_diagnostics::FileId;
use nova_driver::{analyze_program, Analysis, Options, Probe, Program, Roots};

use crate::workspace::{Overlay, PathKey, ProjectKey};

/// The program an answer comes from, so that rename can analyse it again.
#[derive(Debug, Clone)]
pub enum Scope {
    /// A project's program, library and `tests/`.
    Project(PathBuf),
    /// A file on its own: `Program::for_file` finds its package.
    File(PathBuf),
    /// A file in std's cache (spec §4, rule 2), answered from an in-memory
    /// program that includes std. Rename never re-analyses one (it refuses
    /// inside the caches), so the module is not kept.
    Std,
}

/// An analysis, the request's file in it, and where it came from.
pub struct Answer {
    pub analysis: Analysis,
    pub file: FileId,
    // Task 11's rename check is its first reader.
    #[allow(dead_code)]
    pub scope: Scope,
}

/// A request's options: every stage runs, `@test` bodies are checked, no
/// MIR, and the probe or the index as asked.
pub fn options(probe: Option<Probe>, index: bool) -> Options {
    Options {
        keep_going: true,
        tests: true,
        module_only: true,
        probe,
        index,
    }
}

/// Analyse `scope` over `overlay`, guarded against a panic.
pub fn analyse(scope: &Scope, overlay: &Overlay, options: &Options) -> Option<Analysis> {
    let (program, overlay) = match scope {
        Scope::Project(dir) => (Program::for_package(dir, Roots::Test), overlay.clone()),
        Scope::File(path) => (Program::for_file(path), overlay.clone()),
        Scope::Std => {
            // Plan decision 14: `fn main() {}`, which exists only in the
            // overlay.
            let entry = std::env::temp_dir().join("nova-lsp-std").join("main.nova");
            let overlay = overlay.with(&entry, "fn main() {}\n".to_string());
            (Program::loose(&entry), overlay)
        }
    };
    guarded(|| analyze_program(program, &overlay, options).ok())
}

/// The analysis that answers a request in `path` (spec §4, rule 1): its
/// project's, if that reaches the file, or else the file's own.
pub fn answering(path: &Path, overlay: &Overlay, options: &Options) -> Option<Answer> {
    if let Some(module) = crate::std_cache::module_of(path) {
        let scope = Scope::Std;
        let analysis = analyse(&scope, overlay, options)?;
        let file = analysis.db.id_of(&format!("<std/{module}>"))?;
        return Some(Answer {
            analysis,
            file,
            scope,
        });
    }
    if let ProjectKey::Root(dir) = ProjectKey::of(path) {
        let scope = Scope::Project(dir);
        if let Some(analysis) = analyse(&scope, overlay, options) {
            if let Some(file) = file_of(&analysis, path) {
                return Some(Answer {
                    analysis,
                    file,
                    scope,
                });
            }
        }
    }
    let scope = Scope::File(path.to_path_buf());
    let analysis = analyse(&scope, overlay, options)?;
    let file = file_of(&analysis, path)?;
    Some(Answer {
        analysis,
        file,
        scope,
    })
}

/// `path`'s file among `a`'s modules.
fn file_of(a: &Analysis, path: &Path) -> Option<FileId> {
    a.modules
        .iter()
        .find(|(_, p)| PathKey::of(p) == PathKey::of(path))
        .map(|(file, _)| *file)
}

/// `f`, with a panic in the front end logged and turned into `None`.
pub fn guarded<T>(f: impl FnOnce() -> Option<T>) -> Option<T> {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)) {
        Ok(v) => v,
        Err(_) => {
            tracing::warn!("nova lsp: the front end panicked answering a request");
            None
        }
    }
}
