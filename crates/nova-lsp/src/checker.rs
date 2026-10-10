//! The checker thread: analyses run here, off the protocol's thread (spec
//! `docs/superpowers/specs/2026-10-08-phase-3-2-lsp-core-design.md` §6.2,
//! §6.6).
//!
//! It runs one job at a time. Jobs queued for the same project collapse to
//! the latest. A finished job is published only if no newer job for its
//! project was submitted while it ran, so a stale result is never shown.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{mpsc, Arc, Mutex};

use nova_diagnostics::FileId;
use nova_driver::{analyze_program, Analysis, Options, Program, Roots, Sources};
use nova_pm::PackageId;

use crate::convert;
use crate::uri;
use crate::workspace::{declares_main, Document, Overlay, PathKey, ProjectKey};

/// One project's check.
pub struct Job {
    pub project: ProjectKey,
    /// Rises with every job the server submits.
    pub generation: u64,
    pub overlay: Overlay,
    /// The project's open documents.
    pub open: Vec<Document>,
    /// No document of the project is open: clear what it published.
    pub clear: bool,
}

/// A `textDocument/publishDiagnostics` to send.
pub struct Publish {
    pub uri: String,
    pub version: Option<i32>,
    pub diagnostics: Vec<lsp_types::Diagnostic>,
}

/// The handle the protocol's thread keeps.
pub struct Checker {
    jobs: mpsc::Sender<Job>,
    /// The newest generation submitted for each project.
    newest: Arc<Mutex<HashMap<ProjectKey, u64>>>,
    /// Each project's package directories, from its last published check
    /// (spec 3.3a §6, decision 10).
    dirs: Arc<Mutex<HashMap<ProjectKey, Vec<PathKey>>>>,
}

impl Checker {
    /// Start the thread. It hands each result to `publish`.
    pub fn spawn(publish: impl Fn(Publish) + Send + 'static) -> Checker {
        let (jobs, queue) = mpsc::channel::<Job>();
        let newest: Arc<Mutex<HashMap<ProjectKey, u64>>> = Arc::default();
        let dirs: Arc<Mutex<HashMap<ProjectKey, Vec<PathKey>>>> = Arc::default();
        let seen = Arc::clone(&newest);
        let written = Arc::clone(&dirs);
        std::thread::Builder::new()
            .name("nova-lsp-checker".to_string())
            .spawn(move || serve(&queue, &seen, &written, &publish))
            .expect("start the checker thread");
        Checker { jobs, newest, dirs }
    }

    pub fn submit(&self, job: Job) {
        self.newest
            .lock()
            .expect("the newest map")
            .insert(job.project.clone(), job.generation);
        let _ = self.jobs.send(job);
    }

    /// Whether a change to `path` can change `project`'s diagnostics (spec
    /// 3.3a §6): it is under one of the project's package directories.
    /// Before a project's first check is published, and for a loose file,
    /// that is [`ProjectKey::holds`].
    pub fn reaches(&self, project: &ProjectKey, path: &Path) -> bool {
        let dirs = self.dirs.lock().expect("the dirs map");
        match dirs.get(project) {
            Some(dirs) if !dirs.is_empty() => {
                let key = PathKey::of(path);
                dirs.iter().any(|dir| key.is_under(dir))
            }
            _ => project.holds(path),
        }
    }
}

fn serve(
    queue: &mpsc::Receiver<Job>,
    newest: &Mutex<HashMap<ProjectKey, u64>>,
    dirs: &Mutex<HashMap<ProjectKey, Vec<PathKey>>>,
    publish: &dyn Fn(Publish),
) {
    // The URIs each project last published for, to clear the ones it no
    // longer reaches.
    let mut published: HashMap<ProjectKey, HashSet<String>> = HashMap::new();
    while let Ok(first) = queue.recv() {
        let mut latest: Vec<Job> = vec![first];
        while let Ok(job) = queue.try_recv() {
            match latest.iter_mut().find(|j| j.project == job.project) {
                Some(slot) => *slot = job,
                None => latest.push(job),
            }
        }
        for job in latest {
            let results = if job.clear {
                Some((Vec::new(), Vec::new()))
            } else {
                check(&job)
            };
            // A panic keeps the last good diagnostics (spec §6.7).
            let Some((results, reached)) = results else {
                continue;
            };
            let current = newest
                .lock()
                .expect("the newest map")
                .get(&job.project)
                .copied();
            if current != Some(job.generation) {
                continue;
            }
            let before = published.remove(&job.project).unwrap_or_default();
            let now: HashSet<String> = results.iter().map(|p| p.uri.clone()).collect();
            // Clear what the project no longer publishes for before sending
            // what it does. A file's URI changes spelling when it is opened or
            // closed (the server's `C:` against VS Code's `c%3A` on Windows),
            // and a client that maps both to one file must end with the new
            // diagnostics, not with the old spelling's clear.
            for uri in before.difference(&now) {
                publish(Publish {
                    uri: uri.clone(),
                    version: None,
                    diagnostics: Vec::new(),
                });
            }
            for p in results {
                publish(p);
            }
            let mut known = dirs.lock().expect("the dirs map");
            if job.clear {
                known.remove(&job.project);
            } else {
                published.insert(job.project.clone(), now);
                known.insert(job.project.clone(), reached);
            }
        }
    }
}

/// Why an analysis gave nothing.
enum Failed {
    /// The entry could not be read.
    Unreadable,
    /// The front end panicked.
    Panicked,
}

fn run(program: Program, overlay: &Overlay, module_only: bool) -> Result<Analysis, Failed> {
    let entry = program
        .roots
        .first()
        .map(|root| root.path.clone())
        .unwrap_or_default();
    let options = Options {
        keep_going: true,
        tests: true,
        module_only,
        probe: None,
        index: false,
    };
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        analyze_program(program, overlay, &options)
    })) {
        Ok(Ok(a)) => Ok(a),
        Ok(Err(e)) => {
            tracing::warn!("nova lsp: cannot read {}: {e}", entry.display());
            Err(Failed::Unreadable)
        }
        Err(_) => {
            tracing::warn!("nova lsp: the front end panicked on {}", entry.display());
            Err(Failed::Panicked)
        }
    }
}

/// The diagnostics `job` publishes (spec §6.2's ownership rule):
/// - a project's analysis, from its program, library and `tests/` (spec
///   3.3a §6), owns its own package's modules;
/// - each open project file it does not reach is checked on its own, as a
///   module;
/// - a loose file is its own entry, and a module unless it declares
///   `fn main`;
/// - a project in nova's cache of downloaded packages is not checked (spec
///   3.3b §5.5).
///
/// The publishes, and the project's package directories; `None` after a
/// panic.
fn check(job: &Job) -> Option<(Vec<Publish>, Vec<PathKey>)> {
    // A downloaded package belongs to no project: nothing is checked or
    // published for its files (spec 3.3b §5.5).
    if in_the_cache(&job.project) {
        return Some((Vec::new(), Vec::new()));
    }
    let mut out = Vec::new();
    let mut dirs: Vec<PathKey> = Vec::new();
    match &job.project {
        ProjectKey::Loose(file) => {
            let module_only = !job
                .overlay
                .read(file)
                .map(|t| declares_main(&t))
                .unwrap_or(true);
            match run(Program::loose(file), &job.overlay, module_only) {
                Ok(a) => publish_own(job, &a, false, &mut out),
                Err(Failed::Panicked) => return None,
                Err(Failed::Unreadable) => {}
            }
        }
        ProjectKey::Root(dir) => {
            dirs.push(PathKey::of(dir));
            let mut reached: Vec<PathKey> = Vec::new();
            // The program, the library and tests/, in test mode; a
            // library alone is checked as a module (spec 3.3a §6).
            match run(Program::for_package(dir, Roots::Test), &job.overlay, false) {
                Ok(a) => {
                    if let Some(graph) = &a.graph {
                        dirs = graph.dirs().iter().map(|d| PathKey::of(d)).collect();
                    }
                    reached = a.modules.iter().map(|(_, p)| PathKey::of(p)).collect();
                    publish_own(job, &a, true, &mut out);
                }
                Err(Failed::Panicked) => return None,
                // Nothing to read: every open file is checked on its own.
                Err(Failed::Unreadable) => {}
            }
            for doc in &job.open {
                if reached.contains(&PathKey::of(&doc.path)) {
                    continue;
                }
                // The project's own analysis publishes the graph's problems
                // under its nova.toml; this file need not repeat them on its
                // first line.
                let mut program = Program::for_file(&doc.path);
                program.diagnostics.clear();
                match run(program, &job.overlay, true) {
                    Ok(a) => publish_own(job, &a, false, &mut out),
                    Err(Failed::Panicked) => return None,
                    Err(Failed::Unreadable) => {}
                }
            }
        }
    }
    Some((out, dirs))
}

/// Whether `project` is inside one of nova's caches: downloaded packages
/// (spec 3.3b §5.5), or std's sources (3.4a §6).
fn in_the_cache(project: &ProjectKey) -> bool {
    let path = match project {
        ProjectKey::Root(dir) => dir.as_path(),
        ProjectKey::Loose(file) => file.as_path(),
    };
    if crate::std_cache::in_std_cache(path) {
        return true;
    }
    let Some(registry) = nova_pm::registry_dir() else {
        return false;
    };
    PathKey::of(path).is_under(&PathKey::of(&registry))
}

/// One publish per file `analysis` owns (spec 3.3a §6): with `all`, each
/// module of the root package and the root's `nova.toml`, else its entry
/// alone. A dependency's files are its own project's to publish.
fn publish_own(job: &Job, analysis: &Analysis, all: bool, out: &mut Vec<Publish>) {
    let manifest = analysis.manifest.filter(|_| all);
    // A package with no module to read has only its manifest.
    let Some(entry) = analysis.modules.first().map(|(file, _)| *file).or(manifest) else {
        return;
    };
    let open = |path: &Path| {
        job.open
            .iter()
            .find(|d| PathKey::of(&d.path) == PathKey::of(path))
    };
    let uri_of = |path: &Path| match open(path) {
        Some(doc) => doc.uri.clone(),
        None => uri::from_path(path),
    };
    let mut owned: Vec<(FileId, PathBuf)> = if all {
        analysis
            .modules
            .iter()
            .zip(&analysis.module_packages)
            .filter(|(_, package)| matches!(package, None | Some(PackageId(0))))
            .map(|(module, _)| module.clone())
            .collect()
    } else {
        analysis.modules.iter().take(1).cloned().collect()
    };
    if let Some(file) = manifest {
        if let Some(name) = analysis.db.get_name(file) {
            owned.push((file, PathBuf::from(name)));
        }
    }
    for (file, path) in &owned {
        out.push(Publish {
            uri: uri_of(path),
            version: open(path).map(|d| d.version),
            diagnostics: convert::diagnostics_for(analysis, *file, entry, manifest, &uri_of),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_diagnostics_check_leaves_the_index_off() {
        // A guard for spec 3.4a §3.6: diagnostics never pay for the index.
        let path = std::env::temp_dir()
            .join("nova-lsp-checker-index")
            .join("main.nova");
        let overlay = Overlay::default().with(&path, "fn main() {}\n".to_string());
        let a = run(Program::loose(&path), &overlay, true)
            .ok()
            .expect("the buffer is analysed");
        assert!(a.index.is_none());
    }
}
