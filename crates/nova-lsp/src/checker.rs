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
}

impl Checker {
    /// Start the thread. It hands each result to `publish`.
    pub fn spawn(publish: impl Fn(Publish) + Send + 'static) -> Checker {
        let (jobs, queue) = mpsc::channel::<Job>();
        let newest: Arc<Mutex<HashMap<ProjectKey, u64>>> = Arc::default();
        let seen = Arc::clone(&newest);
        std::thread::Builder::new()
            .name("nova-lsp-checker".to_string())
            .spawn(move || serve(&queue, &seen, &publish))
            .expect("start the checker thread");
        Checker { jobs, newest }
    }

    pub fn submit(&self, job: Job) {
        self.newest
            .lock()
            .expect("the newest map")
            .insert(job.project.clone(), job.generation);
        let _ = self.jobs.send(job);
    }
}

fn serve(
    queue: &mpsc::Receiver<Job>,
    newest: &Mutex<HashMap<ProjectKey, u64>>,
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
                Some(Vec::new())
            } else {
                check(&job)
            };
            // A panic keeps the last good diagnostics (spec §6.7).
            let Some(results) = results else {
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
            if !job.clear {
                published.insert(job.project.clone(), now);
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
///   `fn main`.
///
/// `None` after a panic.
fn check(job: &Job) -> Option<Vec<Publish>> {
    let mut out = Vec::new();
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
            let mut reached: Vec<PathKey> = Vec::new();
            // The program, the library and tests/, in test mode; a
            // library alone is checked as a module (spec 3.3a §6).
            match run(Program::for_package(dir, Roots::Test), &job.overlay, false) {
                Ok(a) => {
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
                match run(Program::for_file(&doc.path), &job.overlay, true) {
                    Ok(a) => publish_own(job, &a, false, &mut out),
                    Err(Failed::Panicked) => return None,
                    Err(Failed::Unreadable) => {}
                }
            }
        }
    }
    Some(out)
}

/// One publish per file `analysis` owns (spec 3.3a §6): with `all`, each
/// module of the root package, else its entry alone. A dependency's files
/// are its own project's to publish.
fn publish_own(job: &Job, analysis: &Analysis, all: bool, out: &mut Vec<Publish>) {
    let Some(&(entry, _)) = analysis.modules.first() else {
        return;
    };
    let uri_of = |path: &Path| match job
        .open
        .iter()
        .find(|d| PathKey::of(&d.path) == PathKey::of(path))
    {
        Some(doc) => doc.uri.clone(),
        None => uri::from_path(path),
    };
    let owned: Vec<&(FileId, PathBuf)> = if all {
        analysis
            .modules
            .iter()
            .zip(&analysis.module_packages)
            .filter(|(_, package)| matches!(package, None | Some(PackageId(0))))
            .map(|(module, _)| module)
            .collect()
    } else {
        analysis.modules.iter().take(1).collect()
    };
    for (file, path) in owned {
        let version = job
            .open
            .iter()
            .find(|d| PathKey::of(&d.path) == PathKey::of(path))
            .map(|d| d.version);
        out.push(Publish {
            uri: uri_of(path),
            version,
            diagnostics: convert::diagnostics_for(analysis, *file, entry, &uri_of),
        });
    }
}
