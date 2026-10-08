//! The checker thread: analyses run here, off the protocol's thread (spec
//! `docs/superpowers/specs/2026-10-08-phase-3-2-lsp-core-design.md` §6.6).

use std::path::PathBuf;
use std::sync::mpsc;

use nova_driver::{analyze, Options, Sources};

use crate::convert;
use crate::workspace::{declares_main, Overlay};

/// One analysis to run.
pub struct Job {
    /// The file to check, as its own entry.
    pub path: PathBuf,
    /// Where its diagnostics go.
    pub uri: String,
    pub version: Option<i32>,
    pub overlay: Overlay,
    /// Only clear the file's diagnostics: it was closed.
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
}

impl Checker {
    /// Start the thread. It runs jobs in order and hands each result to
    /// `publish`.
    pub fn spawn(publish: impl Fn(Publish) + Send + 'static) -> Checker {
        let (jobs, queue) = mpsc::channel::<Job>();
        std::thread::Builder::new()
            .name("nova-lsp-checker".to_string())
            .spawn(move || {
                for job in queue {
                    if let Some(p) = run(&job) {
                        publish(p);
                    }
                }
            })
            .expect("start the checker thread");
        Checker { jobs }
    }

    pub fn submit(&self, job: Job) {
        let _ = self.jobs.send(job);
    }
}

fn run(job: &Job) -> Option<Publish> {
    if job.clear {
        return Some(Publish {
            uri: job.uri.clone(),
            version: None,
            diagnostics: Vec::new(),
        });
    }
    let module_only = !job
        .overlay
        .read(&job.path)
        .map(|t| declares_main(&t))
        .unwrap_or(true);
    let options = Options {
        keep_going: true,
        tests: true,
        module_only,
        probe: None,
    };
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        analyze(&job.path, &job.overlay, &options)
    }));
    let analysis = match outcome {
        Ok(Ok(a)) => a,
        Ok(Err(e)) => {
            tracing::warn!("nova lsp: cannot read {}: {e}", job.path.display());
            return None;
        }
        Err(_) => {
            tracing::warn!("nova lsp: the front end panicked on {}", job.path.display());
            return None;
        }
    };
    let entry = analysis.modules.first()?.0;
    Some(Publish {
        uri: job.uri.clone(),
        version: job.version,
        diagnostics: convert::diagnostics_for(&analysis, entry, entry),
    })
}
