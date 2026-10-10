//! The fixes the front end attaches to its diagnostics (spec
//! `docs/superpowers/specs/2026-10-10-phase-3-4b-fixes-and-colour-design.md`
//! §4, §9.1), and the tables they are made from (§4.6). One test per case
//! of §9.1 (plan decision 17).

use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};

use nova_diagnostics::{Diagnostic, Fix};
use nova_driver::{analyze, analyze_program, Analysis, Options, Program, Roots, Sources};
use nova_resolver::ModuleId;

/// Buffers by path, then the disk: what the language server's overlay does.
#[derive(Clone)]
struct Buffers(Vec<(PathBuf, String)>);

impl Sources for Buffers {
    fn read(&self, path: &Path) -> io::Result<String> {
        match self.0.iter().find(|(p, _)| p == path) {
            Some((_, text)) => Ok(text.clone()),
            None => std::fs::read_to_string(path),
        }
    }
}

/// The language server's options: every stage, `@test` bodies, no MIR.
fn options() -> Options {
    Options {
        keep_going: true,
        tests: true,
        module_only: true,
        ..Options::default()
    }
}

const MAIN: &str = "mem/main.nova";

/// `files`, each `(path, text)`, as buffers.
fn buffers(files: &[(&str, &str)]) -> Buffers {
    Buffers(
        files
            .iter()
            .map(|(p, t)| (PathBuf::from(p), t.to_string()))
            .collect(),
    )
}

/// The loose program whose entry is `mem/main.nova`, analysed over `b`.
fn loose(b: &Buffers) -> Analysis {
    analyze(Path::new(MAIN), b, &options()).expect("the entry is readable")
}

/// A fresh directory with a fixed name, so each run replaces the last.
fn fresh(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("nova-driver-fixes-{name}"));
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

/// A project named `demo` in a fresh directory, holding `files`.
fn project(name: &str, files: &[(&str, &str)]) -> PathBuf {
    let dir = fresh(name);
    write(&dir, &[("nova.toml", manifest("demo", "").as_str())]);
    write(&dir, files);
    dir
}

/// `dir/app`, holding `app`, which depends by path on `dir/geom`, whose
/// `src/lib.nova` is `geom_lib`. Returns `dir/app`.
fn app_and_geom(name: &str, app: &[(&str, &str)], geom_lib: &str) -> PathBuf {
    let dir = fresh(name);
    write(
        &dir.join("geom"),
        &[
            ("nova.toml", manifest("geom", "").as_str()),
            ("src/lib.nova", geom_lib),
        ],
    );
    let deps = "\n[dependencies]\ngeom = { path = \"../geom\" }\n";
    let app_dir = dir.join("app");
    write(&app_dir, &[("nova.toml", manifest("app", deps).as_str())]);
    write(&app_dir, app);
    app_dir
}

/// `dir`'s whole project, its program, library and tests, over `b`.
fn whole(dir: &Path, b: &Buffers) -> Analysis {
    analyze_program(Program::for_package(dir, Roots::Test), b, &options())
        .expect("the project reads")
}

fn messages(a: &Analysis) -> String {
    a.diagnostics
        .iter()
        .map(|d| {
            let fixes: Vec<&str> = d.fixes.iter().map(|f| f.title.as_str()).collect();
            format!("{} {} {:?} {fixes:?}\n", d.code, d.message, d.notes)
        })
        .collect()
}

/// The first diagnostic with `code` whose message holds `part`.
#[track_caller]
fn diagnostic<'a>(a: &'a Analysis, code: &str, part: &str) -> &'a Diagnostic {
    a.diagnostics
        .iter()
        .find(|d| d.code == code && d.message.contains(part))
        .unwrap_or_else(|| panic!("no {code} holding {part:?}:\n{}", messages(a)))
}

/// `d`'s fix titled `title`.
#[track_caller]
fn fix<'a>(d: &'a Diagnostic, title: &str) -> &'a Fix {
    d.fixes
        .iter()
        .find(|f| f.title == title)
        .unwrap_or_else(|| panic!("no fix {title:?}: {:?}", d.fixes))
}

/// `base` with `fix` applied: each file it edits, read from a buffer of
/// its new text.
#[track_caller]
fn apply(a: &Analysis, fix: &Fix, base: &Buffers) -> Buffers {
    let edited = fix
        .apply(&a.db)
        .unwrap_or_else(|| panic!("the fix does not apply: {fix:?}"));
    let mut files = base.0.clone();
    for (file, text) in edited {
        let path = PathBuf::from(a.db.get_name(file).unwrap());
        files.retain(|(p, _)| *p != path);
        files.push((path, text));
    }
    Buffers(files)
}

/// The text of `b`'s buffer for the file whose path ends with `end`.
#[track_caller]
fn text<'b>(b: &'b Buffers, end: &str) -> &'b str {
    b.0.iter()
        .find(|(p, _)| p.ends_with(end))
        .map(|(_, t)| t.as_str())
        .unwrap_or_else(|| panic!("no buffer for {end}"))
}

/// How many diagnostics of each code `a` has.
fn counts(a: &Analysis) -> HashMap<String, usize> {
    let mut out = HashMap::new();
    for d in &a.diagnostics {
        *out.entry(d.code.clone()).or_insert(0) += 1;
    }
    out
}

/// Spec §3.2's promise: `code` is less common after the fix, and no code
/// but those in `exposes` is more common.
#[track_caller]
fn assert_fixes(before: &Analysis, after: &Analysis, code: &str, exposes: &[&str]) {
    let (b, a) = (counts(before), counts(after));
    assert!(
        a.get(code).copied().unwrap_or(0) < b.get(code).copied().unwrap_or(0),
        "{code} is not less common:\n{}",
        messages(after)
    );
    for (c, n) in &a {
        if !exposes.contains(&c.as_str()) {
            assert!(
                *n <= b.get(c).copied().unwrap_or(0),
                "{c} became more common:\n{}",
                messages(after)
            );
        }
    }
}

// === Task 3: the tables a fix is made from (spec §4.6) ===

#[test]
fn every_loaded_module_knows_what_it_could_import() {
    let app = app_and_geom(
        "importable",
        &[
            (
                "src/main.nova",
                "import geom\nimport shapes\n\nfn main() {}\n",
            ),
            ("src/shapes.nova", "pub fn side() -> Int {\n    2\n}\n"),
            (
                "src/lib.nova",
                "pub fn name() -> String {\n    \"app\"\n}\n",
            ),
            ("tests/check.nova", "import app\n\n@test\nfn t() {}\n"),
        ],
        "pub fn area() -> Int {\n    1\n}\n",
    );
    let a = whole(&app, &Buffers(Vec::new()));
    assert!(a.diagnostics.is_empty(), "{}", messages(&a));
    let d = a.definitions.as_ref().unwrap();
    let module = |end: &str| {
        let i = a
            .modules
            .iter()
            .position(|(_, p)| p.ends_with(end))
            .unwrap_or_else(|| panic!("no module {end}"));
        ModuleId(i as u32)
    };
    let main = module("app/src/main.nova");
    let shapes = module("app/src/shapes.nova");
    let lib = module("app/src/lib.nova");
    let check = module("app/tests/check.nova");
    let geom = module("geom/src/lib.nova");
    assert_eq!(
        d.importable(main).to_vec(),
        [
            ("geom".to_string(), geom),
            ("lib".to_string(), lib),
            ("shapes".to_string(), shapes),
        ]
    );
    assert_eq!(
        d.importable(check).to_vec(),
        [("app".to_string(), lib), ("geom".to_string(), geom)]
    );
    assert_eq!(
        d.importable(geom).to_vec(),
        Vec::<(String, ModuleId)>::new()
    );
    assert!(d.same_package(main, check) && !d.same_package(main, geom));
}

// === Task 4: make it mutable (spec §4.1; §9.1 cases 1-10) ===

/// The E0060 holding `part`, in the loose program `src`, has one fix,
/// "make `name` mutable", which leaves `decl` in the text; applied, the
/// error goes and nothing comes. The fix replaces the note (spec §3.3).
#[track_caller]
fn makes_mutable(src: &str, part: &str, name: &str, decl: &str) {
    let b = buffers(&[(MAIN, src)]);
    let a = loose(&b);
    let d = diagnostic(&a, "E0060", part);
    assert!(
        d.notes.is_empty(),
        "the fix replaces the note: {:?}",
        d.notes
    );
    assert_eq!(d.fixes.len(), 1, "{:?}", d.fixes);
    let edited = apply(&a, fix(d, &format!("make `{name}` mutable")), &b);
    let after = text(&edited, "main.nova");
    assert!(after.contains(decl), "{after}");
    assert_fixes(&a, &loose(&edited), "E0060", &[]);
}

/// The E0060 holding `part` has no fix, and keeps a note holding `note`.
#[track_caller]
fn no_mutable_fix(src: &str, part: &str, note: &str) {
    let a = loose(&buffers(&[(MAIN, src)]));
    let d = diagnostic(&a, "E0060", part);
    assert!(d.fixes.is_empty(), "{:?}", d.fixes);
    assert!(d.notes.iter().any(|n| n.contains(note)), "{:?}", d.notes);
}

#[test]
fn case_01_make_mutable_for_a_let() {
    makes_mutable(
        "fn main() {\n    let x = 0\n    x = 1\n    println(\"${x}\")\n}\n",
        "cannot assign to immutable variable `x`",
        "x",
        "let mut x = 0",
    );
}

#[test]
fn case_02_make_mutable_for_a_compound_assignment() {
    makes_mutable(
        "fn main() {\n    let x = 0\n    x += 1\n    println(\"${x}\")\n}\n",
        "cannot assign to immutable variable `x`",
        "x",
        "let mut x = 0",
    );
}

#[test]
fn case_03_make_mutable_for_an_element() {
    makes_mutable(
        "fn main() {\n    let a = [1, 2]\n    a[0] = 3\n    println(\"${a[0]}\")\n}\n",
        "an element of immutable `a`",
        "a",
        "let mut a = [1, 2]",
    );
}

#[test]
fn case_04_make_mutable_for_a_field() {
    makes_mutable(
        "record P { v: Int }\n\nfn main() {\n    let p = P { v: 1 }\n    p.v = 2\n    println(\"${p.v}\")\n}\n",
        "a field of immutable `p`",
        "p",
        "let mut p = P { v: 1 }",
    );
}

#[test]
fn case_05_make_mutable_for_a_mut_self_call() {
    makes_mutable(
        "record P { v: Int }\n\nimpl P {\n    fn bump(mut self) {\n        self.v = self.v + 1\n    }\n}\n\nfn main() {\n    let p = P { v: 1 }\n    p.bump()\n}\n",
        "mutates its receiver, but `p` is immutable",
        "p",
        "let mut p = P { v: 1 }",
    );
}

#[test]
fn case_06_make_mutable_for_a_parameter() {
    makes_mutable(
        "fn twice(n: Int) -> Int {\n    n = n * 2\n    n\n}\n\nfn main() {\n    println(\"${twice(2)}\")\n}\n",
        "cannot assign to immutable variable `n`",
        "n",
        "fn twice(mut n: Int)",
    );
}

#[test]
fn case_07_make_mutable_for_a_closure_parameter() {
    makes_mutable(
        "fn main() {\n    let f = |n: Int| {\n        n = n + 1\n        n\n    }\n    println(\"${f(1)}\")\n}\n",
        "cannot assign to immutable variable `n`",
        "n",
        "|mut n: Int|",
    );
}

#[test]
fn case_08_no_make_mutable_for_a_match_binding() {
    no_mutable_fix(
        "fn main() {\n    let o = Some(1)\n    match o {\n        Some(v) => {\n            v = 2\n        }\n        None => {}\n    }\n}\n",
        "cannot assign to immutable variable `v`",
        "let mut v",
    );
}

#[test]
fn case_09_no_make_mutable_for_a_for_variable() {
    no_mutable_fix(
        "fn main() {\n    for i in 0..3 {\n        i = i + 1\n    }\n}\n",
        "cannot assign to immutable variable `i`",
        "let mut i",
    );
}

#[test]
fn case_10_no_make_mutable_for_self() {
    no_mutable_fix(
        "record C { n: Int }\n\nimpl C {\n    fn reset(self) {\n        self.n = 0\n    }\n}\n\nfn main() {}\n",
        "a field of immutable `self`",
        "mut self",
    );
}
