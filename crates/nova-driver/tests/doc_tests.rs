//! What doc tests need from the driver (spec
//! `docs/superpowers/specs/2026-10-11-phase-3-5b-doc-tests-design.md`
//! §11.2, §12.2): a test binary built from sources that are not on disk,
//! diagnostics mapped before they are rendered, and doc test modules.
//!
//! The tests that link need the runtime library beside the test executable,
//! which `cargo build --workspace` puts there, as for `nova-cli`'s
//! `run_tests`.

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::process::Command;

use nova_diagnostics::{render, Diagnostic, FileDb, Span};
use nova_driver::{
    analyze_program, build_test_program_with, Options, Program, Sources, DOC_TEST_FUNCTION,
};
use nova_resolver::ModuleId;

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

/// A fresh `shapes` package in a directory with a fixed name: a library
/// with its own test, a second module, and two `tests/` files, one named
/// for the package.
fn shapes(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("nova-driver-doc-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    let files = [
        (
            "nova.toml",
            "[package]\nname = \"shapes\"\nversion = \"0.1.0\"\nedition = \"2026\"\n",
        ),
        (
            "src/lib.nova",
            "import units\n\n/// Twice.\npub fn twice(x: Int) -> Int {\n    x * 2 + zero()\n}\n\n\
             @test\nfn own_test() {\n    assert_eq(twice(1), 2)\n}\n",
        ),
        ("src/units.nova", "pub fn zero() -> Int {\n    0\n}\n"),
        ("tests/helpers.nova", "pub fn h() -> Int {\n    1\n}\n"),
        ("tests/shapes.nova", "pub fn s() -> Int {\n    1\n}\n"),
    ];
    for (path, text) in files {
        let path = dir.join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }
    dir
}

/// A doc test whose body is `body`, as a plain block becomes one.
fn plain(body: &str) -> String {
    format!("import shapes\n\n@test\nfn {DOC_TEST_FUNCTION}() {{\n    {body}\n}}\n")
}

/// A doc test that declares `fn main` with `body`, and its own `@test fn
/// extra`.
fn with_main(body: &str) -> String {
    format!(
        "import shapes\n\nfn main() {{\n    {body}\n}}\n\n@test\nfn extra() {{\n    main()\n}}\n\n\
         @test\nfn {DOC_TEST_FUNCTION}() {{\n    main()\n}}\n"
    )
}

fn options() -> Options {
    Options {
        tests: true,
        ..Options::default()
    }
}

fn messages(diagnostics: &[Diagnostic]) -> Vec<String> {
    diagnostics
        .iter()
        .map(|d| format!("{} {}", d.code, d.message))
        .collect()
}

#[test]
fn doc_tests_import_their_package_and_only_their_own_test_is_collected() {
    let dir = shapes("imports");
    let doc0 = dir.join("<doc tests>").join("doc0.nova");
    let doc1 = dir.join("<doc tests>").join("doc1.nova");
    let memory = Memory(vec![
        (doc0.clone(), plain("assert_eq(twice(2), 4)")),
        (doc1.clone(), with_main("assert_eq(twice(3), 6)")),
    ]);
    let program = Program::doc_tests(Some(&dir), vec![doc0.clone(), doc1.clone()]);
    let a = analyze_program(program, &memory, &options()).expect("the entry reads");
    assert_eq!(messages(&a.diagnostics), Vec::<String>::new());
    // The doc tests first, then the library, read as its root names it.
    let paths: Vec<&Path> = a.modules.iter().map(|(_, p)| p.as_path()).collect();
    assert_eq!(
        paths[..4],
        [
            doc0.as_path(),
            doc1.as_path(),
            &dir.join("src").join("lib.nova"),
            &dir.join("src").join("units.nova"),
        ]
    );
    let defs = a.definitions.as_ref().expect("resolved");
    let labels: Vec<&str> = (0..4).map(|i| defs.module_name(ModuleId(i))).collect();
    assert_eq!(labels, ["doc/doc0", "doc/doc1", "lib", "units"]);
    let names: Vec<&str> = defs.defs().iter().map(|d| d.name.as_str()).collect();
    assert!(
        !names.contains(&"own_test"),
        "the library's test is collected"
    );
    assert!(!names.contains(&"extra"), "a block's own test is collected");
    let doc_tests = names.iter().filter(|n| **n == DOC_TEST_FUNCTION).count();
    assert_eq!(doc_tests, 2);
}

#[test]
fn a_doc_test_sees_no_file_beside_it() {
    // `tests/helpers.nova` is not its sibling, and `tests/shapes.nova` does
    // not clash with the package (E0004).
    let dir = shapes("siblings");
    let doc0 = dir.join("<doc tests>").join("doc0.nova");
    let memory = Memory(vec![(
        doc0.clone(),
        format!("import helpers\n{}", plain("assert_eq(twice(1), 2)")),
    )]);
    let a = analyze_program(
        Program::doc_tests(Some(&dir), vec![doc0]),
        &memory,
        &options(),
    )
    .expect("the entry reads");
    assert_eq!(
        messages(&a.diagnostics),
        ["E0001 cannot find module `helpers`"]
    );
}

#[test]
fn a_binary_of_doc_tests_runs_each_alone() {
    let dir = shapes("binary");
    let doc0 = dir.join("<doc tests>").join("doc0.nova");
    let doc1 = dir.join("<doc tests>").join("doc1.nova");
    let memory = Memory(vec![
        (doc0.clone(), plain("assert_eq(twice(2), 4)")),
        (doc1.clone(), with_main("assert_eq(twice(3), 7)")),
    ]);
    let program = Program::doc_tests(Some(&dir), vec![doc0, doc1]);
    let (exe, tests) = build_test_program_with(program, &memory, None).expect("it builds");
    // Named for the first doc test, apart from the unit tests' binary.
    assert_eq!(exe.file_stem().and_then(|s| s.to_str()), Some("doc0"));
    let names: Vec<&str> = tests.iter().map(|t| t.name.as_str()).collect();
    assert_eq!(names, ["doc0::nova_doctest", "doc1::nova_doctest"]);
    assert_eq!(
        String::from_utf8_lossy(&run(&exe, None).stdout),
        "2\ndoc0::nova_doctest\ndoc1::nova_doctest\n"
    );
    assert!(run(&exe, Some("0")).status.success());
    let failed = run(&exe, Some("1"));
    assert!(!failed.status.success());
    assert!(String::from_utf8_lossy(&failed.stderr).contains("nova: panic:"));
}

#[test]
fn std_doc_tests_import_nothing() {
    let dir = Path::new("<nova driver std doc tests>");
    let doc0 = dir.join("doc0.nova");
    let doc1 = dir.join("doc1.nova");
    let test = |body: &str| format!("@test\nfn {DOC_TEST_FUNCTION}() {{\n    {body}\n}}\n");
    let memory = Memory(vec![
        (doc0.clone(), test("assert_eq(1 + 1, 2)")),
        (
            doc1.clone(),
            format!("import doc0\n\n{}", test("assert(true, \"t\")")),
        ),
    ]);
    let program = Program::doc_tests(None, vec![doc0.clone(), doc1.clone()]);
    let a = analyze_program(program, &memory, &options()).expect("the entry reads");
    assert_eq!(
        messages(&a.diagnostics),
        ["E0001 cannot find module `doc0`"]
    );
    let memory = Memory(vec![
        (doc0.clone(), test("assert_eq(1 + 1, 2)")),
        (doc1.clone(), test("assert(true, \"t\")")),
    ]);
    let program = Program::doc_tests(None, vec![doc0, doc1]);
    let (exe, tests) = build_test_program_with(program, &memory, None).expect("it builds");
    let names: Vec<&str> = tests.iter().map(|t| t.name.as_str()).collect();
    assert_eq!(names, ["doc0::nova_doctest", "doc1::nova_doctest"]);
    assert!(run(&exe, Some("0")).status.success());
    assert!(run(&exe, Some("1")).status.success());
}

#[test]
fn no_doc_tests_build_nothing() {
    // Not the library's own tests, under the doc tests' name.
    let dir = shapes("none");
    let program = Program::doc_tests(Some(&dir), Vec::new());
    let error = build_test_program_with(program, &Memory(Vec::new()), None)
        .expect_err("there is nothing to build");
    assert_eq!(
        error.to_string(),
        "nothing to compile: the package has no source file for this"
    );
}
