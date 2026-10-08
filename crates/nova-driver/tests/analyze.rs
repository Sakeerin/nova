//! `analyze` (spec
//! `docs/superpowers/specs/2026-10-08-phase-3-2-lsp-core-design.md` §3,
//! §9.1).

use std::io;
use std::path::{Path, PathBuf};

use nova_driver::{analyze, Analysis, Options, Outcome, Probe, Sources};

/// Buffers by path, then the disk: what the language server's overlay does.
struct Buffers(Vec<(PathBuf, String)>);

impl Sources for Buffers {
    fn read(&self, path: &Path) -> io::Result<String> {
        match self.0.iter().find(|(p, _)| p == path) {
            Some((_, text)) => Ok(text.clone()),
            None => std::fs::read_to_string(path),
        }
    }
}

fn mem(files: &[(&str, &str)]) -> Buffers {
    Buffers(
        files
            .iter()
            .map(|(p, t)| (PathBuf::from(p), t.to_string()))
            .collect(),
    )
}

fn keep_going() -> Options {
    Options {
        keep_going: true,
        tests: true,
        ..Options::default()
    }
}

fn codes(a: &Analysis) -> Vec<&str> {
    a.diagnostics.iter().map(|d| d.code.as_str()).collect()
}

/// A fresh directory with a fixed name, so each run replaces the last.
fn fresh_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("nova-analyze-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

const BOTH: &str = "fn f() {\n    let x = (\n}\nfn g() -> Int { \"s\" }\nfn main() {}\n";

#[test]
fn keep_going_reports_a_syntax_error_and_a_type_error_together() {
    let a = analyze(
        Path::new("mem/main.nova"),
        &mem(&[("mem/main.nova", BOTH)]),
        &keep_going(),
    )
    .unwrap();
    let codes = codes(&a);
    assert!(codes.contains(&"P0001"), "{codes:?}");
    assert!(codes.contains(&"E0010"), "{codes:?}");
}

#[test]
fn check_file_still_stops_at_the_syntax_error() {
    let dir = fresh_dir("staged");
    let file = dir.join("main.nova");
    std::fs::write(&file, BOTH).unwrap();
    match nova_driver::check_file(&file).unwrap() {
        Outcome::Failed { errors } => assert_eq!(errors, 1, "only the syntax error"),
        Outcome::Ok(()) => panic!("check_file accepted a syntax error"),
    }
}

#[test]
fn a_buffer_beats_the_file_on_disk() {
    let dir = fresh_dir("buffer");
    let file = dir.join("main.nova");
    std::fs::write(&file, "fn main() { let x: Int = \"s\" }\n").unwrap();
    let clean = Buffers(vec![(file.clone(), "fn main() {}\n".to_string())]);
    let a = analyze(&file, &clean, &keep_going()).unwrap();
    assert!(a.diagnostics.is_empty(), "{:?}", codes(&a));
}

#[test]
fn a_never_saved_buffer_is_checked() {
    let a = analyze(
        Path::new("nowhere/never/saved.nova"),
        &mem(&[(
            "nowhere/never/saved.nova",
            "fn main() { let x: Int = \"s\" }\n",
        )]),
        &keep_going(),
    )
    .unwrap();
    assert_eq!(codes(&a), ["E0010"]);
}

#[test]
fn an_unreadable_entry_is_an_error() {
    assert!(analyze(Path::new("nowhere/missing.nova"), &mem(&[]), &keep_going()).is_err());
}

#[test]
fn mir_runs_on_a_clean_program_unless_module_only() {
    let src = "fn helper() -> Int { 1 }\n";
    let program = analyze(
        Path::new("m/lib.nova"),
        &mem(&[("m/lib.nova", src)]),
        &keep_going(),
    )
    .unwrap();
    assert_eq!(codes(&program), ["E0601"], "MIR's no-`main` check");
    let module = Options {
        module_only: true,
        ..keep_going()
    };
    let a = analyze(
        Path::new("m/lib.nova"),
        &mem(&[("m/lib.nova", src)]),
        &module,
    )
    .unwrap();
    assert!(a.diagnostics.is_empty(), "{:?}", codes(&a));
}

#[test]
fn with_tests_a_test_body_is_checked() {
    let src = "@test\nfn t() {\n    let x: Int = \"s\"\n}\nfn main() {}\n";
    let a = analyze(
        Path::new("t/main.nova"),
        &mem(&[("t/main.nova", src)]),
        &keep_going(),
    )
    .unwrap();
    assert_eq!(codes(&a), ["E0010"]);
    let no_tests = Options {
        tests: false,
        ..keep_going()
    };
    let a = analyze(
        Path::new("t/main.nova"),
        &mem(&[("t/main.nova", src)]),
        &no_tests,
    )
    .unwrap();
    assert!(a.diagnostics.is_empty(), "{:?}", codes(&a));
}

#[test]
fn a_dropped_name_raises_no_e0001_at_any_site() {
    // Each item breaks right after its name, so item-level recovery resumes
    // at the next item (a `record P {` left open would instead swallow
    // everything up to the next `}`). Each use is of a kind with its own
    // E0001 message (decision 1).
    let main = "import lib::{area}\n\
                fn f(x: Int {\n}\n\
                record P %\n\
                trait T %\n\
                fn main() {\n    f(1)\n    let g = f\n    let p: P = P { x: 1 }\n}\n\
                impl T for Int {}\n";
    let lib = "pub fn area(r: Int {\n}\n";
    let a = analyze(
        Path::new("d/main.nova"),
        &mem(&[("d/main.nova", main), ("d/lib.nova", lib)]),
        &keep_going(),
    )
    .unwrap();
    let codes = codes(&a);
    assert!(codes.contains(&"P0001"), "{codes:?}");
    assert!(!codes.contains(&"E0001"), "{:?}", a.diagnostics);
}

#[test]
fn a_name_never_declared_still_raises_e0001() {
    let src = "fn main() {\n    nothing(1)\n}\n";
    let a = analyze(
        Path::new("u/main.nova"),
        &mem(&[("u/main.nova", src)]),
        &keep_going(),
    )
    .unwrap();
    assert_eq!(codes(&a), ["E0001"]);
}

#[test]
fn modules_are_numbered_in_load_order() {
    let a = analyze(
        Path::new("o/main.nova"),
        &mem(&[
            ("o/main.nova", "import geometry\nfn main() {}\n"),
            ("o/geometry.nova", "pub fn area() -> Int { 1 }\n"),
        ]),
        &keep_going(),
    )
    .unwrap();
    assert!(a.diagnostics.is_empty(), "{:?}", codes(&a));
    assert_eq!(a.modules.len(), 2);
    assert_eq!(a.modules[0].1, PathBuf::from("o/main.nova"));
    assert!(a.modules[1].1.ends_with("geometry.nova"));
    let defs = a.definitions.as_ref().unwrap();
    assert!(defs
        .names_in_scope(nova_resolver::ModuleId(1))
        .iter()
        .any(|(n, _)| n == "area"));
}

#[test]
fn the_probe_reaches_an_imported_module() {
    let geometry = "pub fn area(s: String) -> Int {\n    s.\n}\n";
    let offset = geometry.find("s.").unwrap() as u32 + 2;
    let options = Options {
        probe: Some(Probe {
            path: PathBuf::from("q/geometry.nova"),
            offset,
        }),
        ..keep_going()
    };
    let a = analyze(
        Path::new("q/main.nova"),
        &mem(&[
            ("q/main.nova", "import geometry\nfn main() {}\n"),
            ("q/geometry.nova", geometry),
        ]),
        &options,
    )
    .unwrap();
    assert_eq!(a.probe.receiver, Some(nova_hir::Ty::String));
}
