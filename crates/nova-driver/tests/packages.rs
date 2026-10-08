//! The loader on packages (spec
//! `docs/superpowers/specs/2026-10-08-phase-3-3a-local-packages-design.md`
//! §4, §7.2).

use std::path::{Path, PathBuf};

use nova_driver::{analyze, analyze_program, Analysis, DiskSources, Options, Program, Roots};
use nova_pm::PackageId;

/// A fresh, empty directory with a fixed name, so each run replaces the last.
fn fresh(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("nova-driver-packages-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Write each `(path, text)` under `dir`.
fn write(dir: &Path, files: &[(&str, &str)]) {
    for (path, text) in files {
        let path = dir.join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }
}

/// A `nova.toml` for `name`, with `extra` appended.
fn manifest(name: &str, extra: &str) -> String {
    format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2026\"\n{extra}")
}

const GEOM_DEPENDENCY: &str = "\n[dependencies]\ngeom = { path = \"../geom\" }\n";

/// `dir/geom`: a library whose `lib.nova` uses its own `utils.nova`.
fn geom(dir: &Path) {
    write(
        &dir.join("geom"),
        &[
            ("nova.toml", manifest("geom", "").as_str()),
            (
                "src/lib.nova",
                "import utils\n\npub fn area() -> Int {\n    width() * width()\n}\n",
            ),
            ("src/utils.nova", "pub fn width() -> Int {\n    3\n}\n"),
        ],
    );
}

fn options(tests: bool) -> Options {
    Options {
        tests,
        ..Options::default()
    }
}

fn check(program: Program) -> Analysis {
    analyze_program(program, &DiskSources, &options(false)).expect("the entry reads")
}

fn codes(a: &Analysis) -> Vec<&str> {
    a.diagnostics.iter().map(|d| d.code.as_str()).collect()
}

fn messages(a: &Analysis) -> String {
    a.diagnostics
        .iter()
        .map(|d| format!("{} {} {:?}\n", d.code, d.message, d.notes))
        .collect()
}

fn paths(a: &Analysis) -> Vec<&Path> {
    a.modules.iter().map(|(_, path)| path.as_path()).collect()
}

#[test]
fn two_packages_utils_are_two_modules() {
    let dir = fresh("two-utils");
    geom(&dir);
    let app = dir.join("app");
    write(
        &app,
        &[
            ("nova.toml", manifest("app", GEOM_DEPENDENCY).as_str()),
            (
                "src/main.nova",
                "import geom\nimport utils\n\nfn main() {\n    println(\"area ${area()}\")\n    println(label())\n}\n",
            ),
            ("src/utils.nova", "pub fn label() -> String {\n    \"app utils\"\n}\n"),
        ],
    );
    let a = check(Program::for_package(&app, Roots::Program));
    assert!(a.diagnostics.is_empty(), "{}", messages(&a));
    let paths = paths(&a);
    assert_eq!(paths.len(), 4, "{paths:?}");
    assert!(paths[1].ends_with("geom/src/lib.nova"), "{paths:?}");
    assert!(paths[2].ends_with("app/src/utils.nova"), "{paths:?}");
    assert!(paths[3].ends_with("geom/src/utils.nova"), "{paths:?}");
    let ids = [PackageId(0), PackageId(1), PackageId(0), PackageId(1)];
    assert_eq!(a.module_packages, ids.map(Some));
}

#[test]
fn src_utils_and_tests_utils_are_two_modules() {
    let dir = fresh("src-and-tests");
    write(
        &dir,
        &[
            ("nova.toml", manifest("app", "").as_str()),
            ("src/main.nova", "import utils\n\nfn main() {\n    println(label())\n}\n"),
            ("src/utils.nova", "pub fn label() -> String {\n    \"src\"\n}\n"),
            (
                "tests/api.nova",
                "import utils\n\n@test\nfn the_tests_utils_is_seen() {\n    assert_eq(helper(), \"tests\")\n}\n",
            ),
            ("tests/utils.nova", "pub fn helper() -> String {\n    \"tests\"\n}\n"),
        ],
    );
    let program = Program::for_package(&dir, Roots::Test);
    let a = analyze_program(program, &DiskSources, &options(true)).unwrap();
    assert!(a.diagnostics.is_empty(), "{}", messages(&a));
    // The roots, then what `main` imports.
    let paths = paths(&a);
    assert_eq!(paths.len(), 4, "{paths:?}");
    assert!(paths[1].ends_with("tests/api.nova"), "{paths:?}");
    assert!(paths[2].ends_with("tests/utils.nova"), "{paths:?}");
    assert!(paths[3].ends_with("src/utils.nova"), "{paths:?}");
}

#[test]
fn a_file_and_a_dependency_with_one_name_is_e0004() {
    let dir = fresh("clash");
    geom(&dir);
    let app = dir.join("app");
    write(
        &app,
        &[
            ("nova.toml", manifest("app", GEOM_DEPENDENCY).as_str()),
            ("src/main.nova", "import geom\n\nfn main() {}\n"),
            ("src/geom.nova", "pub fn area() -> Int {\n    1\n}\n"),
        ],
    );
    let a = check(Program::for_package(&app, Roots::Program));
    assert_eq!(codes(&a), ["E0004"], "{}", messages(&a));
    let message = &a.diagnostics[0].message;
    assert!(
        message.contains("`geom` is both a module of this package and a dependency"),
        "{message}"
    );
    assert!(
        message.contains("geom.nova") && message.contains("nova.toml:7"),
        "{message}"
    );
}

#[test]
fn a_dev_dependency_imported_from_src_is_e0001_with_a_note() {
    let dir = fresh("dev-from-src");
    geom(&dir);
    let app = dir.join("app");
    let extra = "\n[dev-dependencies]\ngeom = { path = \"../geom\" }\n";
    write(
        &app,
        &[
            ("nova.toml", manifest("app", extra).as_str()),
            ("src/main.nova", "import geom\n\nfn main() {}\n"),
        ],
    );
    let a = check(Program::for_package(&app, Roots::Program));
    assert_eq!(codes(&a), ["E0001"], "{}", messages(&a));
    assert!(a.diagnostics[0]
        .message
        .contains("cannot find module `geom`"));
    assert_eq!(
        a.diagnostics[0].notes,
        ["`geom` is a dev-dependency, which only `tests/` files can import"]
    );
}

#[test]
fn an_import_matches_a_files_case_exactly() {
    let dir = fresh("case");
    write(
        &dir,
        &[
            ("main.nova", "import Utils\n\nfn main() {}\n"),
            ("utils.nova", "pub fn f() -> Int {\n    1\n}\n"),
        ],
    );
    let a = check(Program::loose(&dir.join("main.nova")));
    assert_eq!(codes(&a), ["E0001"], "{}", messages(&a));
    assert!(a.diagnostics[0]
        .message
        .contains("cannot find module `Utils`"));
}

#[test]
fn tests_files_import_the_package_a_dev_dependency_and_each_other() {
    let dir = fresh("tests-imports");
    geom(&dir);
    write(
        &dir.join("helper"),
        &[
            ("nova.toml", manifest("helper", "").as_str()),
            (
                "src/lib.nova",
                "pub fn twice(n: Int) -> Int {\n    n * 2\n}\n",
            ),
        ],
    );
    let app = dir.join("app");
    let extra = "\n[dependencies]\ngeom = { path = \"../geom\" }\n\n\
                 [dev-dependencies]\nhelper = { path = \"../helper\" }\n";
    write(
        &app,
        &[
            ("nova.toml", manifest("app", extra).as_str()),
            (
                "src/lib.nova",
                "pub fn name() -> String {\n    \"app\"\n}\n",
            ),
            (
                "tests/api.nova",
                "import app\nimport geom\nimport helper\nimport util\n\n@test\n\
                 fn sees_all_four() {\n    assert_eq(name(), \"app\")\n    \
                 assert_eq(area(), 9)\n    assert_eq(twice(2), 4)\n    assert_eq(one(), 1)\n}\n",
            ),
            ("tests/util.nova", "pub fn one() -> Int {\n    1\n}\n"),
        ],
    );
    let program = Program::for_package(&app, Roots::Test);
    let a = analyze_program(program, &DiskSources, &options(true)).unwrap();
    assert!(a.diagnostics.is_empty(), "{}", messages(&a));
}

#[test]
fn a_file_argument_in_src_sees_its_packages_dependencies() {
    let dir = fresh("file-argument");
    geom(&dir);
    let app = dir.join("app");
    write(
        &app,
        &[
            ("nova.toml", manifest("app", GEOM_DEPENDENCY).as_str()),
            (
                "src/main.nova",
                "import geom\n\nfn main() {\n    println(\"${area()}\")\n}\n",
            ),
        ],
    );
    let entry = app.join("src").join("main.nova");
    let a = analyze(&entry, &DiskSources, &options(false)).unwrap();
    assert!(a.diagnostics.is_empty(), "{}", messages(&a));
    assert_eq!(a.modules.len(), 3, "{:?}", paths(&a));
}

#[test]
fn a_loose_file_beside_a_broken_manifest_reads_no_manifest() {
    let dir = fresh("loose");
    write(
        &dir,
        &[
            ("nova.toml", "this is not [ valid toml"),
            (
                "hello.nova",
                "import helper\n\nfn main() {\n    println(greeting())\n}\n",
            ),
            (
                "helper.nova",
                "pub fn greeting() -> String {\n    \"hi\"\n}\n",
            ),
        ],
    );
    let program = Program::for_file(&dir.join("hello.nova"));
    assert!(program.graph.is_none() && program.diagnostics.is_empty());
    let a = check(program);
    assert!(a.diagnostics.is_empty(), "{}", messages(&a));
}

#[test]
fn an_error_in_a_dependency_is_reported_in_its_own_file() {
    // Review Focus 2: the CLI renders this analysis's diagnostics the same
    // way, through the same `FileDb`.
    let dir = fresh("dependency-error");
    write(
        &dir.join("geom"),
        &[
            ("nova.toml", manifest("geom", "").as_str()),
            ("src/lib.nova", "pub fn area() -> Int {\n    \"nine\"\n}\n"),
        ],
    );
    let app = dir.join("app");
    write(
        &app,
        &[
            ("nova.toml", manifest("app", GEOM_DEPENDENCY).as_str()),
            (
                "src/main.nova",
                "import geom\n\nfn main() {\n    println(\"${area()}\")\n}\n",
            ),
        ],
    );
    let a = check(Program::for_package(&app, Roots::Program));
    assert_eq!(a.diagnostics.len(), 1, "{}", messages(&a));
    let rendered = nova_diagnostics::render::render_to_string(&a.db, &a.diagnostics);
    let place = Path::new("geom").join("src").join("lib.nova");
    assert!(
        rendered.contains(&format!("{}:", place.display())),
        "{rendered}"
    );
}

#[test]
fn a_library_without_a_program_is_checked_as_a_module() {
    let dir = fresh("library-only");
    geom(&dir);
    let program = Program::for_package(&dir.join("geom"), Roots::Check);
    assert!(!program.runs);
    // MIR would find no `main`: E0601.
    let a = check(program);
    assert!(a.diagnostics.is_empty(), "{}", messages(&a));
}

#[test]
fn a_graph_error_stops_the_analysis_before_loading() {
    let dir = fresh("graph-error");
    let extra = "\n[dependencies]\ngeom = { path = \"../nowhere\" }\n";
    write(
        &dir,
        &[
            ("nova.toml", manifest("app", extra).as_str()),
            ("src/main.nova", "fn main() {}\n"),
        ],
    );
    let a = check(Program::for_package(&dir, Roots::Program));
    assert_eq!(codes(&a), ["M0007"], "{}", messages(&a));
    assert!(a.modules.is_empty());
}

#[test]
fn an_entry_without_main_is_e0601_even_when_a_dependency_has_one() {
    let dir = fresh("no-main");
    write(
        &dir.join("geom"),
        &[
            ("nova.toml", manifest("geom", "").as_str()),
            (
                "src/lib.nova",
                "pub fn area() -> Int {\n    9\n}\n\nfn main() {\n    println(\"geom's demo\")\n}\n",
            ),
        ],
    );
    let app = dir.join("app");
    write(
        &app,
        &[
            ("nova.toml", manifest("app", GEOM_DEPENDENCY).as_str()),
            (
                "src/main.nova",
                "import geom\n\nfn run() {\n    println(\"${area()}\")\n}\n",
            ),
        ],
    );
    let a = check(Program::for_package(&app, Roots::Program));
    assert_eq!(codes(&a), ["E0601"], "{}", messages(&a));
}

#[test]
fn a_dependencys_tests_are_stripped_and_the_roots_kept() {
    let dir = fresh("strip");
    write(
        &dir.join("geom"),
        &[
            ("nova.toml", manifest("geom", "").as_str()),
            (
                "src/lib.nova",
                "pub fn area() -> Int {\n    9\n}\n\n@test\nfn geom_checks_its_area() {\n    assert_eq(area(), 9)\n}\n",
            ),
        ],
    );
    let app = dir.join("app");
    write(
        &app,
        &[
            ("nova.toml", manifest("app", GEOM_DEPENDENCY).as_str()),
            (
                "src/main.nova",
                "import geom\n\nfn main() {\n    println(\"${area()}\")\n}\n\n\
                 @test\nfn app_checks_the_area() {\n    assert_eq(area(), 9)\n}\n",
            ),
        ],
    );
    let program = Program::for_package(&app, Roots::Test);
    let a = analyze_program(program, &DiskSources, &options(true)).unwrap();
    assert!(a.diagnostics.is_empty(), "{}", messages(&a));
    let defs = a.definitions.as_ref().unwrap();
    let defined = |name: &str| defs.defs().iter().any(|d| d.name == name);
    assert!(defined("app_checks_the_area"));
    assert!(!defined("geom_checks_its_area"));
}
