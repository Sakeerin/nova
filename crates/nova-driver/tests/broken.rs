//! Broken programs never panic or hang (spec
//! `docs/superpowers/specs/2026-10-08-phase-3-2-lsp-core-design.md` §9.2).
//!
//! Each example, cut at 16 evenly spaced character boundaries, and each
//! `tests/runtime/*.nova`, cut at 2, is analysed with `keep_going`, as the
//! language server analyses a file being typed. Each analysis, with the
//! index on, must return within 10 s without panicking, and every
//! occurrence it records must lie inside its file. Every fix it makes must
//! apply, and its first three fixes, applied one at a time, must analyse
//! again without panicking (spec 3.4b §9.2; plan decision 12).

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

/// Whether every occurrence the index recorded lies inside its file's text
/// (spec 3.4a §8.3).
fn spans_inside(a: &nova_driver::Analysis) -> bool {
    a.index.as_ref().map_or(true, |index| {
        index.occurrences.iter().all(|o| {
            a.db.get_source(o.span.file)
                .is_some_and(|s| o.span.start <= o.span.end && o.span.end as usize <= s.len())
        })
    })
}

/// A cut program with one fix applied: the fixed files' texts, then the
/// cut, then the disk.
struct Fixed<'c> {
    cut: &'c Cut,
    files: Vec<(PathBuf, String)>,
}

impl Sources for Fixed<'_> {
    fn read(&self, path: &Path) -> std::io::Result<String> {
        match self.files.iter().find(|(p, _)| p == path) {
            Some((_, text)) => Ok(text.clone()),
            None => self.cut.read(path),
        }
    }
}

/// Spec 3.4b §9.2: every fix's edits lie inside their file, on character
/// boundaries, and do not overlap; and the first three, each applied
/// alone, analyse again without panicking. How many fixes it checked.
fn fixes_hold(
    a: &nova_driver::Analysis,
    path: &Path,
    cut: &Cut,
    options: &Options,
) -> Result<usize, String> {
    let fixes: Vec<&nova_diagnostics::Fix> = a.diagnostics.iter().flat_map(|d| &d.fixes).collect();
    for (k, fix) in fixes.iter().enumerate() {
        let Some(edited) = fix.apply(&a.db) else {
            return Err(format!("a fix does not apply: {fix:?}"));
        };
        if k >= 3 {
            continue;
        }
        let files = edited
            .into_iter()
            .map(|(file, text)| (PathBuf::from(a.db.get_name(file).unwrap_or("")), text))
            .collect();
        let fixed = Fixed { cut, files };
        let again = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            analyze(path, &fixed, options)
        }));
        if again.is_err() {
            return Err(format!("panicked after the fix {:?}", fix.title));
        }
    }
    Ok(fixes.len())
}

/// Spec 3.4b §9.2: a cut drops only its last item, whole, so the programs
/// above carry no fix. This one plants each fix that stays in its file, in
/// an item of its own, and is swept whole and cut (plan Task 7's ruling).
const FIX_BEARING: &str = "record P { width: Int }\n\ntype Shape =\n  | Empty\n  | Circle(Int)\n\n@tset\nfn helper() {}\n\nfn bump() -> Int {\n    let x = 0\n    x = 1\n    x\n}\n\nfn total() -> Int {\n    let count = 1\n    cuont + 1\n}\n\nfn width(p: P) -> Int {\n    p.widht\n}\n\nfn pick(s: Shape) -> Int {\n    match s {\n        _ => 0\n        Shape::Empty => 1\n    }\n}\n\nfn main() {\n    println(\"${bump()} ${total()}\")\n}\n";

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
    // The planted fixes, whole and at 16 cuts (spec 3.4b §9.2).
    let planted = root.join("tests/broken-fixes.nova");
    jobs.push((planted.clone(), FIX_BEARING.to_string()));
    for cut in cuts(FIX_BEARING, 16) {
        jobs.push((planted.clone(), cut));
    }
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
                index: true,
                ..Options::default()
            };
            let sources = Cut {
                path: path.clone(),
                text: text.clone(),
            };
            let started = Instant::now();
            let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                analyze(&path, &sources, &options)
            }));
            let took = started.elapsed();
            let (spans_ok, fixes) = match &outcome {
                Ok(Ok(a)) => (spans_inside(a), fixes_hold(a, &path, &sources, &options)),
                _ => (true, Ok(0)),
            };
            current.lock().unwrap()[t] = None;
            if done
                .send((path, text.len(), outcome.is_ok(), spans_ok, fixes, took))
                .is_err()
            {
                break;
            }
        });
    }
    drop(done);

    let mut failures: Vec<String> = Vec::new();
    let mut checked = 0usize;
    for _ in 0..total {
        match results.recv_timeout(Duration::from_secs(30)) {
            Ok((path, len, ok, spans_ok, fixes, took)) => {
                if !ok {
                    failures.push(format!("panicked: {} cut at byte {len}", path.display()));
                } else if !spans_ok {
                    failures.push(format!(
                        "an occurrence outside its file: {} cut at byte {len}",
                        path.display()
                    ));
                } else if let Err(why) = &fixes {
                    failures.push(format!("{why}: {} cut at byte {len}", path.display()));
                } else if took > Duration::from_secs(10) {
                    failures.push(format!(
                        "took {took:?}: {} cut at byte {len}",
                        path.display()
                    ));
                }
                checked += fixes.unwrap_or(0);
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
    eprintln!("{checked} fixes checked");
    assert!(
        failures.is_empty(),
        "{} of {total}:\n{}",
        failures.len(),
        failures.join("\n")
    );
    // The sweep is not vacuous: the cut programs carry fixes. Checked
    // after the failures, which name a fix that does not apply.
    assert!(checked > 0, "no cut program had a fix");
}
