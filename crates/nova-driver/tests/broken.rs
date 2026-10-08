//! Broken programs never panic or hang (spec
//! `docs/superpowers/specs/2026-10-08-phase-3-2-lsp-core-design.md` §9.2).
//!
//! Each example, cut at 16 evenly spaced character boundaries, and each
//! `tests/runtime/*.nova`, cut at 2, is analysed with `keep_going`, as the
//! language server analyses a file being typed. Each analysis must return
//! within 10 s without panicking.

use std::path::{Path, PathBuf};
use std::sync::{mpsc, Arc, Mutex};
use std::time::{Duration, Instant};

use nova_driver::{analyze, Options, Sources};

/// One file's text in place of its contents; every other file from disk.
struct Cut {
    path: PathBuf,
    text: String,
}

impl Sources for Cut {
    fn read(&self, path: &Path) -> std::io::Result<String> {
        if path == self.path {
            Ok(self.text.clone())
        } else {
            std::fs::read_to_string(path)
        }
    }
}

/// Per thread, the cut it is analysing, if any: its file, and its length in
/// bytes.
type InFlight = Arc<Mutex<Vec<Option<(PathBuf, usize)>>>>;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// `text` cut at `n` evenly spaced character boundaries, never at its end.
fn cuts(text: &str, n: usize) -> Vec<String> {
    let bounds: Vec<usize> = text.char_indices().map(|(i, _)| i).collect();
    if bounds.is_empty() {
        return Vec::new();
    }
    (1..=n)
        .map(|k| text[..bounds[(bounds.len() * k / (n + 1)).min(bounds.len() - 1)]].to_string())
        .collect()
}

#[test]
fn cut_programs_never_panic_or_hang() {
    let root = root();
    let mut programs: Vec<(PathBuf, usize)> = Vec::new();
    for entry in std::fs::read_dir(root.join("examples")).unwrap() {
        let main = entry.unwrap().path().join("src").join("main.nova");
        if main.is_file() {
            programs.push((main, 16));
        }
    }
    for entry in std::fs::read_dir(root.join("tests").join("runtime")).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_some_and(|e| e == "nova") {
            programs.push((path, 2));
        }
    }
    programs.sort();
    let mut jobs: Vec<(PathBuf, String)> = Vec::new();
    for (path, n) in &programs {
        let text = std::fs::read_to_string(path).unwrap();
        for cut in cuts(&text, *n) {
            jobs.push((path.clone(), cut));
        }
    }
    // 6 examples x 16 + 134 runtime programs x 2 = 364, on 2026-10-08.
    assert!(jobs.len() >= 360, "only {} cut programs", jobs.len());
    let total = jobs.len();

    let threads = std::thread::available_parallelism()
        .map_or(4, |n| n.get())
        .min(8);
    let queue = Arc::new(Mutex::new(jobs));
    // What each thread is analysing, for the report if one hangs.
    let current: InFlight = Arc::new(Mutex::new(vec![None; threads]));
    let (done, results) = mpsc::channel();
    for t in 0..threads {
        let queue = Arc::clone(&queue);
        let current = Arc::clone(&current);
        let done = done.clone();
        std::thread::spawn(move || loop {
            let Some((path, text)) = queue.lock().unwrap().pop() else {
                break;
            };
            current.lock().unwrap()[t] = Some((path.clone(), text.len()));
            let options = Options {
                keep_going: true,
                tests: true,
                ..Options::default()
            };
            let sources = Cut {
                path: path.clone(),
                text: text.clone(),
            };
            let started = Instant::now();
            let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                analyze(&path, &sources, &options).map(|a| a.diagnostics.len())
            }));
            let took = started.elapsed();
            current.lock().unwrap()[t] = None;
            if done
                .send((path, text.len(), outcome.is_ok(), took))
                .is_err()
            {
                break;
            }
        });
    }
    drop(done);

    let mut failures: Vec<String> = Vec::new();
    for _ in 0..total {
        match results.recv_timeout(Duration::from_secs(30)) {
            Ok((path, len, ok, took)) => {
                if !ok {
                    failures.push(format!("panicked: {} cut at byte {len}", path.display()));
                } else if took > Duration::from_secs(10) {
                    failures.push(format!(
                        "took {took:?}: {} cut at byte {len}",
                        path.display()
                    ));
                }
            }
            Err(_) => {
                let stuck: Vec<String> = current
                    .lock()
                    .unwrap()
                    .iter()
                    .flatten()
                    .map(|(p, len)| format!("{} cut at byte {len}", p.display()))
                    .collect();
                panic!("no analysis finished in 30 s; in flight: {stuck:?}");
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {total}:\n{}",
        failures.len(),
        failures.join("\n")
    );
}
