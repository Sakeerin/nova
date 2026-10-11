//! What doc tests need from the driver (spec
//! `docs/superpowers/specs/2026-10-11-phase-3-5b-doc-tests-design.md`
//! §11.2, §12.2): a test binary built from sources that are not on disk,
//! and diagnostics mapped before they are rendered.
//!
//! The tests that link need the runtime library beside the test executable,
//! which `cargo build --workspace` puts there, as for `nova-cli`'s
//! `run_tests`.

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::process::Command;

use nova_diagnostics::{render, Diagnostic, FileDb, Span};
use nova_driver::{build_test_program_with, Program, Sources};

/// Files served from memory, then from the disk.
struct Memory(Vec<(PathBuf, String)>);

impl Sources for Memory {
    fn read(&self, path: &Path) -> std::io::Result<String> {
        match self.0.iter().find(|(p, _)| p == path) {
            Some((_, text)) => Ok(text.clone()),
            None => std::fs::read_to_string(path),
        }
    }
}

/// A directory with a fixed name under the system's temporary directory,
/// removed and not made again: the files in it are served from memory.
fn nowhere(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("nova-driver-doc-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

/// The binary's output when `NOVA_TEST_INDEX` is `index`, or its inventory.
fn run(exe: &Path, index: Option<&str>) -> std::process::Output {
    let mut command = Command::new(exe);
    match index {
        Some(index) => command.env("NOVA_TEST_INDEX", index),
        None => command.env_remove("NOVA_TEST_INDEX"),
    };
    command.output().expect("the test binary runs")
}

#[test]
fn a_test_binary_builds_from_sources_not_on_disk() {
    let dir = nowhere("memory");
    let main = dir.join("main.nova");
    let memory = Memory(vec![
        (
            main.clone(),
            "import helper\n\n@test\nfn adds() {\n    assert_eq(add(1, 2), 3)\n}\n\n\
             @test\nfn fails() {\n    assert_eq(add(1, 2), 4)\n}\n"
                .to_string(),
        ),
        (
            dir.join("helper.nova"),
            "pub fn add(a: Int, b: Int) -> Int {\n    a + b\n}\n".to_string(),
        ),
    ]);
    let (exe, tests) =
        build_test_program_with(Program::loose(&main), &memory, None).expect("it builds");
    let names: Vec<&str> = tests.iter().map(|t| t.name.as_str()).collect();
    assert_eq!(names, ["adds", "fails"]);
    assert!(!dir.exists(), "nothing is written beside the sources");
    assert_eq!(
        String::from_utf8_lossy(&run(&exe, None).stdout),
        "2\nadds\nfails\n"
    );
    assert!(run(&exe, Some("0")).status.success());
    let failed = run(&exe, Some("1"));
    assert!(!failed.status.success());
    assert!(String::from_utf8_lossy(&failed.stderr).contains("nova: panic:"));
}

#[test]
fn a_mapped_diagnostic_renders_at_its_source_line() {
    // Spec 3.5b §7: the map moves a label from the module the build reads
    // to the doc comment that holds its code, a file it adds to the
    // database.
    let dir = nowhere("map");
    let main = dir.join("main.nova");
    let code = "@test\nfn uses_nope() {\n    assert_eq(nope(2), 4)\n}\n";
    let doc_name = "docs/lib.nova";
    let doc = "/// Twice.\n///\n/// ```\n/// assert_eq(nope(2), 4)\n/// ```\n\
               pub fn twice(x: Int) -> Int {\n    x * 2\n}\n";
    let memory = Memory(vec![(main.clone(), code.to_string())]);
    let virtual_name = main.display().to_string();
    let shift = doc.find("nope(2").unwrap() as u32 - code.find("nope(2").unwrap() as u32;
    let rendered = RefCell::new(String::new());
    let map = |db: &mut FileDb, d: &mut Diagnostic| {
        for label in &mut d.labels {
            if db.get_name(label.span.file) == Some(virtual_name.as_str()) {
                let file = db.id_of(doc_name).unwrap_or_else(|| db.add(doc_name, doc));
                label.span = Span::new(label.span.start + shift, label.span.end + shift, file);
            }
        }
        rendered
            .borrow_mut()
            .push_str(&render::render_to_string(db, std::slice::from_ref(d)));
        true
    };
    let error = build_test_program_with(Program::loose(&main), &memory, Some(&map))
        .expect_err("`nope` is not defined");
    assert_eq!(
        error.to_string(),
        "could not compile due to 1 previous error"
    );
    let rendered = rendered.into_inner();
    assert!(rendered.contains("E0001"), "{rendered}");
    assert!(rendered.contains("docs/lib.nova:4:15"), "{rendered}");
    assert!(
        rendered.contains("4 │ /// assert_eq(nope(2), 4)"),
        "{rendered}"
    );
}

#[test]
fn a_map_sees_every_diagnostic_and_cannot_hide_an_error() {
    let dir = nowhere("hide");
    let main = dir.join("main.nova");
    let memory = Memory(vec![(
        main.clone(),
        "@test\nfn t() {\n    let x = 1\n    match x {\n        _ => {}\n        1 => {}\n    }\n    \
         let y: Int = \"s\"\n}\n"
            .to_string(),
    )]);
    let seen = RefCell::new(Vec::new());
    let map = |_: &mut FileDb, d: &mut Diagnostic| {
        seen.borrow_mut().push(d.code.clone());
        false
    };
    let error = build_test_program_with(Program::loose(&main), &memory, Some(&map))
        .expect_err("a String is not an Int");
    // The map said `false` to both, and the error was still counted.
    assert_eq!(
        error.to_string(),
        "could not compile due to 1 previous error"
    );
    assert_eq!(seen.into_inner(), ["E0021", "E0010"]);
}
