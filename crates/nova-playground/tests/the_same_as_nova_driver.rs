//! The playground's copied front end makes what `nova build --target wasm`
//! makes (spec
//! `docs/superpowers/specs/2026-11-28-phase-6-1-the-playground-design.md`
//! §5, test 2): each program of `tools/playground/examples.txt`, and a few
//! written here, is compiled by [`nova_playground::compile`] and by
//! `nova_driver::build_program_wasm`, from `main.nova` in a fresh
//! directory. A success gives the same `app.wasm` and `app.js`, byte for
//! byte; a failure the same error count, and the text `render_to_string`
//! makes of `nova_driver::analyze`'s diagnostics, the file named
//! `main.nova`.

use std::path::{Path, PathBuf};

use nova_playground::{compile, Outcome, FILE_NAME};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
}

/// A fresh, empty directory with a fixed name, so each run replaces the
/// last.
fn fresh(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("nova-playground-test-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create the test directory");
    dir
}

/// A file of the repository, with Windows' line ends made `\n`, as a
/// checkout may give them.
fn read(path: &str) -> String {
    std::fs::read_to_string(repo_root().join(path))
        .unwrap_or_else(|e| panic!("{path}: {e}"))
        .replace("\r\n", "\n")
}

/// Reads each path from the case's directory, so `analyze`'s entry is named
/// `main.nova`, as the playground names it.
struct In(PathBuf);

impl nova_driver::Sources for In {
    fn read(&self, path: &Path) -> std::io::Result<String> {
        std::fs::read_to_string(self.0.join(path))
    }
}

/// `case`, on a thread with the pipeline's stack, as `nova` runs it.
fn on_the_pipelines_stack<T: Send + 'static>(case: impl FnOnce() -> T + Send + 'static) -> T {
    std::thread::Builder::new()
        .stack_size(nova_driver::PIPELINE_STACK)
        .spawn(case)
        .expect("start the case's thread")
        .join()
        .unwrap_or_else(|panic| std::panic::resume_unwind(panic))
}

/// Compile `source` both ways, in `nova-playground-test-<name>`, hold the
/// playground to the driver, and return the playground's outcome.
fn the_same(name: &str, source: &str) -> Outcome {
    let name = name.to_string();
    let source = source.to_string();
    on_the_pipelines_stack(move || {
        let dir = fresh(&name);
        let entry = dir.join(FILE_NAME);
        std::fs::write(&entry, &source).expect("write main.nova");
        let built = nova_driver::build_program_wasm(
            nova_driver::Program::for_file(&entry),
            &dir.join("dist"),
        )
        .unwrap_or_else(|e| panic!("{name}: the driver's own error: {e:#}"));
        let outcome = compile(&source);
        match (&built, &outcome) {
            (nova_driver::Outcome::Ok(dist), Outcome::Compiled { wasm, js, .. }) => {
                let app_wasm = std::fs::read(dist.join("app.wasm")).expect("app.wasm");
                let app_js = std::fs::read_to_string(dist.join("app.js")).expect("app.js");
                assert!(app_wasm == *wasm, "{name}: app.wasm differs");
                assert!(app_js == *js, "{name}: app.js differs");
            }
            (
                nova_driver::Outcome::Failed { errors },
                Outcome::Failed {
                    text,
                    errors: count,
                },
            ) => {
                assert_eq!(count, errors, "{name}: the error count\n{text}");
                let analysis = nova_driver::analyze(
                    Path::new(FILE_NAME),
                    &In(dir.clone()),
                    &nova_driver::Options::default(),
                )
                .unwrap_or_else(|e| panic!("{name}: analyze: {e}"));
                let expected =
                    nova_diagnostics::render::render_to_string(&analysis.db, &analysis.diagnostics);
                assert_eq!(*text, expected, "{name}: the text");
            }
            (nova_driver::Outcome::Ok(_), other) => {
                panic!("{name}: the driver built it, the playground gave {other:?}")
            }
            (nova_driver::Outcome::Failed { errors }, other) => panic!(
                "{name}: the driver failed with {errors} error(s), the playground gave {other:?}"
            ),
        }
        outcome
    })
}

/// One line of `examples.txt`.
struct Example {
    name: String,
    source: String,
    expected: String,
}

fn examples() -> Vec<Example> {
    read("tools/playground/examples.txt")
        .lines()
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(|line| {
            let fields: Vec<&str> = line.split('|').map(str::trim).collect();
            let [name, label, source, expected] = fields.as_slice() else {
                panic!("not `name | label | source | expected`: {line}");
            };
            assert!(!label.is_empty(), "{name}: no label");
            Example {
                name: name.to_string(),
                source: source.to_string(),
                expected: expected.to_string(),
            }
        })
        .collect()
}

#[test]
fn the_fixed_list_holds_eight_examples_each_path_there() {
    let examples = examples();
    let names: Vec<&str> = examples.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(
        names,
        [
            "hello",
            "producer-consumer",
            "traits",
            "json_round_trip",
            "collections",
            "iterator",
            "counter",
            "type_error",
        ]
    );
    for example in &examples {
        assert!(
            repo_root().join(&example.source).is_file(),
            "{}: no {}",
            example.name,
            example.source
        );
        // Only the counter, which draws, has no text to compare.
        if example.name == "counter" {
            assert_eq!(example.expected, "-");
        } else {
            assert!(
                repo_root().join(&example.expected).is_file(),
                "{}: no {}",
                example.name,
                example.expected
            );
        }
    }
}

/// Each example compiles as `nova build --target wasm` compiles it, and
/// only the counter calls `std/dom`.
#[test]
fn every_example_is_compiled_as_nova_compiles_it() {
    for example in examples() {
        let outcome = the_same(&example.name, &read(&example.source));
        match (example.name.as_str(), outcome) {
            ("type_error", Outcome::Failed { text, errors }) => {
                assert_eq!(errors, 1);
                assert_eq!(text, read(&example.expected), "type_error.txt");
            }
            (name, Outcome::Compiled { page, .. }) => {
                assert_eq!(page, name == "counter", "{name}: page");
            }
            (name, other) => panic!("{name}: {other:?}"),
        }
    }
}

/// `nova build` drops a test function before resolving, and so does the
/// playground: kept, its `assert` would not resolve.
#[test]
fn a_test_function_is_dropped_as_nova_drops_it() {
    let outcome = the_same(
        "test-function",
        "fn main() { println(\"hi\") }\n\n@test\nfn t() { assert(1 == 1) }\n",
    );
    assert!(matches!(outcome, Outcome::Compiled { .. }), "{outcome:?}");
}

/// An import of a module that is not there: the playground has no file
/// but `main.nova`.
#[test]
fn a_missing_module_fails_as_nova_fails() {
    let outcome = the_same(
        "missing-module",
        "import nowhere\n\nfn main() {\n    println(\"hi\")\n}\n",
    );
    assert!(
        matches!(outcome, Outcome::Failed { errors: 1, .. }),
        "{outcome:?}"
    );
}

#[test]
fn a_lex_error_fails_as_nova_fails() {
    let outcome = the_same("lex-error", "fn main() {\n    let x = 1 $ 2\n}\n");
    assert!(
        matches!(outcome, Outcome::Failed { errors: 1, .. }),
        "{outcome:?}"
    );
}

#[test]
fn a_parse_error_fails_as_nova_fails() {
    let outcome = the_same("parse-error", "fn main( {\n}\n");
    assert!(
        matches!(outcome, Outcome::Failed { errors: 3, .. }),
        "{outcome:?}"
    );
}
