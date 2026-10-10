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

// === Task 5: import a name, and did you mean (spec §4.2, §4.3; §9.1
// cases 11-23, 25 and 26) ===

const GEOMETRY: &str = "pub record Point { x: Int, y: Int }\n\npub fn origin() -> Point {\n    Point { x: 0, y: 0 }\n}\n\npub fn manhattan(p: Point) -> Int {\n    p.x + p.y\n}\n\npub trait Shape {\n    fn area(self) -> Int\n}\n\npub fn one() -> Int {\n    1\n}\n";

const KINDS: &str = "pub type Shape =\n  | Circle(Int)\n  | Empty\n\npub record P { v: Int }\n\nimpl P {\n    fn new() -> P {\n        P { v: 7 }\n    }\n}\n\npub fn make() -> Shape {\n    Empty\n}\n\npub fn one() -> Int {\n    1\n}\n";

/// In the loose program `files`, the diagnostic `code` holding `part`
/// offers `title`, which leaves `expect` in the file ending `file`;
/// applied, the diagnostic goes and nothing comes.
#[track_caller]
fn offers(files: &[(&str, &str)], code: &str, part: &str, title: &str, file: &str, expect: &str) {
    let b = buffers(files);
    let a = loose(&b);
    let d = diagnostic(&a, code, part);
    let edited = apply(&a, fix(d, title), &b);
    let after = text(&edited, file);
    assert!(after.contains(expect), "{after}");
    assert_fixes(&a, &loose(&edited), code, &[]);
}

/// The diagnostic `code` holding `part` offers no import.
#[track_caller]
fn no_import(files: &[(&str, &str)], code: &str, part: &str) {
    let a = loose(&buffers(files));
    let d = diagnostic(&a, code, part);
    assert!(
        d.fixes.iter().all(|f| !f.title.starts_with("import")),
        "{:?}",
        d.fixes
    );
}

#[test]
fn case_11_import_extends_an_existing_list() {
    offers(
        &[
            (
                MAIN,
                "import geometry::{origin}\n\nfn main() {\n    let d = manhattan(origin())\n    println(\"${d}\")\n}\n",
            ),
            ("mem/geometry.nova", GEOMETRY),
        ],
        "E0001",
        "cannot find function `manhattan`",
        "import `manhattan` from `geometry`",
        "main.nova",
        "import geometry::{origin, manhattan}\n",
    );
}

#[test]
fn case_12_import_adds_a_line_after_the_last_import() {
    for nl in ["\n", "\r\n"] {
        let main =
            "import shapes // the shapes\n\nfn main() {\n    println(\"${twice()} ${one()}\")\n}\n"
                .replace('\n', nl);
        let shapes =
            "import geometry\n\npub fn twice() -> Int {\n    one() * 2\n}\n".replace('\n', nl);
        offers(
            &[
                (MAIN, main.as_str()),
                ("mem/shapes.nova", shapes.as_str()),
                ("mem/geometry.nova", GEOMETRY),
            ],
            "E0001",
            "cannot find function `one`",
            "import `one` from `geometry`",
            "main.nova",
            &format!("import shapes // the shapes{nl}import geometry::{{one}}{nl}{nl}fn main()"),
        );
    }
}

#[test]
fn case_13_import_goes_above_the_first_item_below_a_header() {
    for (k, nl) in ["\n", "\r\n"].into_iter().enumerate() {
        let main = "// A header.\n\n// What main does.\nfn main() {\n    println(\"${twice()} ${one()}\")\n}\n"
            .replace('\n', nl);
        let lib = "import geometry\nimport shapes\n\npub fn name() -> String {\n    \"demo\"\n}\n"
            .replace('\n', nl);
        let geometry = GEOMETRY.replace('\n', nl);
        let shapes = "pub fn twice() -> Int {\n    2\n}\n".replace('\n', nl);
        let dir = project(
            &format!("above-the-first-item-{k}"),
            &[
                ("src/main.nova", main.as_str()),
                ("src/lib.nova", lib.as_str()),
                ("src/geometry.nova", geometry.as_str()),
                ("src/shapes.nova", shapes.as_str()),
            ],
        );
        let none = Buffers(Vec::new());
        let a = whole(&dir, &none);
        let d = diagnostic(&a, "E0001", "cannot find function `one`");
        let edited = apply(&a, fix(d, "import `one` from `geometry`"), &none);
        let after = text(&edited, "main.nova");
        let want = format!(
            "// A header.{nl}{nl}import geometry::{{one}}{nl}{nl}// What main does.{nl}fn main()"
        );
        assert!(after.starts_with(&want), "{after}");
        assert_fixes(&a, &whole(&dir, &edited), "E0001", &[]);
    }
}

#[test]
fn case_14_import_from_a_dependencys_library() {
    let app = app_and_geom(
        "from-a-dependency",
        &[
            (
                "src/main.nova",
                "import geom\nimport shapes\n\nfn main() {\n    println(\"${area()} ${twice()}\")\n}\n",
            ),
            ("src/shapes.nova", "pub fn twice() -> Int {\n    side() * 2\n}\n"),
        ],
        "pub fn area() -> Int {\n    1\n}\n\npub fn side() -> Int {\n    2\n}\n",
    );
    let none = Buffers(Vec::new());
    let a = whole(&app, &none);
    let d = diagnostic(&a, "E0001", "cannot find function `side`");
    let edited = apply(&a, fix(d, "import `side` from `geom`"), &none);
    let after = text(&edited, "shapes.nova");
    assert!(
        after.starts_with("import geom::{side}\n\npub fn twice()"),
        "{after}"
    );
    assert_fixes(&a, &whole(&app, &edited), "E0001", &[]);
}

#[test]
fn case_15_import_a_type_a_record_and_a_trait() {
    let geometry = ("mem/geometry.nova", GEOMETRY);
    offers(
        &[
            (
                MAIN,
                "import geometry::{origin}\n\nfn show(p: Point) -> Int {\n    p.x\n}\n\nfn main() {\n    println(\"${show(origin())}\")\n}\n",
            ),
            geometry,
        ],
        "E0001",
        "cannot find type `Point`",
        "import `Point` from `geometry`",
        "main.nova",
        "import geometry::{origin, Point}\n",
    );
    offers(
        &[
            (
                MAIN,
                "import geometry::{origin}\n\nfn main() {\n    let p = Point { x: 1, y: 2 }\n    println(\"${p.x}\")\n}\n",
            ),
            geometry,
        ],
        "E0001",
        "cannot find record `Point`",
        "import `Point` from `geometry`",
        "main.nova",
        "import geometry::{origin, Point}\n",
    );
    offers(
        &[
            (
                MAIN,
                "import geometry::{origin}\n\nrecord Sq { s: Int }\n\nimpl Shape for Sq {\n    fn area(self) -> Int {\n        self.s * self.s\n    }\n}\n\nfn main() {}\n",
            ),
            geometry,
        ],
        "E0001",
        "cannot find trait `Shape`",
        "import `Shape` from `geometry`",
        "main.nova",
        "import geometry::{origin, Shape}\n",
    );
}

#[test]
fn case_16_import_an_unknown_qualifier() {
    let kinds = ("mem/kinds.nova", KINDS);
    offers(
        &[
            (
                MAIN,
                "import kinds::{one}\n\nfn main() {\n    let p = P::new()\n    println(\"${p.v} ${one()}\")\n}\n",
            ),
            kinds,
        ],
        "E0900",
        "module-qualified paths",
        "import `P` from `kinds`",
        "main.nova",
        "import kinds::{one, P}\n",
    );
    offers(
        &[
            (
                MAIN,
                "import kinds::{one}\n\nfn main() {\n    let e = Shape::Empty\n}\n",
            ),
            kinds,
        ],
        "E0900",
        "module-qualified paths",
        "import `Shape` from `kinds`",
        "main.nova",
        "import kinds::{one, Shape}\n",
    );
    offers(
        &[
            (
                MAIN,
                "import kinds::{make}\n\nfn main() {\n    match make() {\n        Shape::Circle(r) => println(\"${r}\")\n        Shape::Empty => println(\"empty\")\n    }\n}\n",
            ),
            kinds,
        ],
        "E0001",
        "cannot resolve this pattern",
        "import `Shape` from `kinds`",
        "main.nova",
        "import kinds::{make, Shape}\n",
    );
}

#[test]
fn case_17_no_import_when_two_modules_export_the_name() {
    no_import(
        &[
            (
                MAIN,
                "import a::{x}\nimport b::{y}\n\nfn main() {\n    let s = x() + y() + g()\n}\n",
            ),
            (
                "mem/a.nova",
                "pub fn x() -> Int {\n    1\n}\n\npub fn g() -> Int {\n    2\n}\n",
            ),
            (
                "mem/b.nova",
                "pub fn y() -> Int {\n    1\n}\n\npub fn g() -> Int {\n    3\n}\n",
            ),
        ],
        "E0001",
        "cannot find function `g`",
    );
}

#[test]
fn case_18_no_import_that_binds_a_name_the_module_has_but_std_does_not_block() {
    let geo = (
        "mem/geo.nova",
        "pub record Area { v: Int }\n\npub fn Area() -> Int {\n    2\n}\n\npub record Map { v: Int }\n\npub fn Map() -> Int {\n    3\n}\n\npub fn one() -> Int {\n    1\n}\n",
    );
    no_import(
        &[
            (
                MAIN,
                "import geo::{one}\n\nrecord Area { w: Int }\n\nfn main() {\n    let n = Area() + one()\n}\n",
            ),
            geo,
        ],
        "E0001",
        "cannot find function `Area`",
    );
    // `Map` is also std's type, which an import wins over (spec §4.2).
    offers(
        &[
            (
                MAIN,
                "import geo::{one}\n\nfn main() {\n    let n = Map() + one()\n}\n",
            ),
            geo,
        ],
        "E0001",
        "cannot find function `Map`",
        "import `Map` from `geo`",
        "main.nova",
        "import geo::{one, Map}\n",
    );
}

#[test]
fn case_19_no_import_of_a_sum_type_for_a_record_literal() {
    no_import(
        &[
            (
                MAIN,
                "import geo::{one}\n\nfn main() {\n    let p = Point { x: 1 }\n}\n",
            ),
            (
                "mem/geo.nova",
                "pub type Point =\n  | Origin\n\npub fn one() -> Int {\n    1\n}\n",
            ),
        ],
        "E0001",
        "cannot find record `Point`",
    );
}

#[test]
fn case_20_did_you_mean_a_local() {
    offers(
        &[(
            MAIN,
            "fn main() {\n    let count = 1\n    let total = cuont + 1\n    println(\"${total}\")\n}\n",
        )],
        "E0001",
        "cannot find `cuont`",
        "change `cuont` to `count`",
        "main.nova",
        "let total = count + 1",
    );
}

#[test]
fn case_21_did_you_mean_a_type_and_a_qualifier() {
    offers(
        &[(
            MAIN,
            "record Point { x: Int }\n\nfn show(p: Piont) -> Int {\n    p.x\n}\n\nfn main() {}\n",
        )],
        "E0001",
        "cannot find type `Piont`",
        "change `Piont` to `Point`",
        "main.nova",
        "fn show(p: Point)",
    );
    offers(
        &[(
            MAIN,
            "type Shape =\n  | Empty\n\nfn main() {\n    let s = Shpae::Empty\n}\n",
        )],
        "E0900",
        "module-qualified paths",
        "change `Shpae` to `Shape`",
        "main.nova",
        "let s = Shape::Empty",
    );
}

#[test]
fn case_22_did_you_mean_a_field_in_a_read_and_a_literal() {
    offers(
        &[(
            MAIN,
            "record P { width: Int }\n\nfn main() {\n    let p = P { widht: 1 }\n}\n",
        )],
        "E0014",
        "has no field `widht`",
        "change `widht` to `width`",
        "main.nova",
        "P { width: 1 }",
    );
    offers(
        &[(
            MAIN,
            "record P { width: Int }\n\nfn main() {\n    let p = P { width: 1 }\n    let w = p.widht + 1\n}\n",
        )],
        "E0014",
        "no field `widht` on record `P`",
        "change `widht` to `width`",
        "main.nova",
        "let w = p.width + 1",
    );
}

#[test]
fn case_23_did_you_mean_a_method_one_from_a_trait_not_imported() {
    offers(
        &[(
            MAIN,
            "record P { v: Int }\n\nimpl P {\n    fn double(self) -> Int {\n        self.v * 2\n    }\n}\n\nfn main() {\n    let p = P { v: 1 }\n    let d = p.doubel()\n}\n",
        )],
        "E0014",
        "no method `doubel`",
        "change `doubel` to `double`",
        "main.nova",
        "let d = p.double()",
    );
    // Method calls resolve through every impl, imported or not (spec §4.3).
    offers(
        &[
            (MAIN, "import loud::{make}\n\nfn main() {\n    let s = make().shuot()\n}\n"),
            (
                "mem/loud.nova",
                "pub trait Loud {\n    fn shout(self) -> String\n}\n\npub record T { s: String }\n\nimpl Loud for T {\n    fn shout(self) -> String {\n        self.s\n    }\n}\n\npub fn make() -> T {\n    T { s: \"hi\" }\n}\n",
            ),
        ],
        "E0014",
        "no method `shuot`",
        "change `shuot` to `shout`",
        "main.nova",
        "let s = make().shout()",
    );
}

#[test]
fn case_25_did_you_mean_a_name_equal_ignoring_case_beyond_the_distance() {
    offers(
        &[(
            MAIN,
            "fn main() {\n    let httpclient = 1\n    let x = HTTPCLIENT + 1\n}\n",
        )],
        "E0001",
        "cannot find `HTTPCLIENT`",
        "change `HTTPCLIENT` to `httpclient`",
        "main.nova",
        "let x = httpclient + 1",
    );
}

#[test]
fn case_26_no_did_you_mean_beyond_the_distance_and_its_edge() {
    let a = loose(&buffers(&[(
        MAIN,
        "fn main() {\n    let abcde = 1\n    let x = abxde + axxde\n}\n",
    )]));
    let near = diagnostic(&a, "E0001", "cannot find `abxde`");
    fix(near, "change `abxde` to `abcde`");
    let far = diagnostic(&a, "E0001", "cannot find `axxde`");
    assert!(far.fixes.is_empty(), "{:?}", far.fixes);
}

// === Task 6: make public, the attributes, an unreachable arm (spec §4.3,
// §4.4, §4.5; §9.1 cases 24 and 27-30) ===

#[test]
fn case_24_did_you_mean_an_attribute_and_a_test_argument() {
    offers(
        &[(MAIN, "@tset\nfn t() {}\n\nfn main() {}\n")],
        "E0082",
        "unknown attribute `@tset`",
        "change `tset` to `test`",
        "main.nova",
        "@test\nfn t()",
    );
    offers(
        &[(
            MAIN,
            "@test(should_panik)\nfn t() {\n    panic(\"x\")\n}\n\nfn main() {}\n",
        )],
        "E0085",
        "unknown `@test` argument `should_panik`",
        "change `should_panik` to `should_panic`",
        "main.nova",
        "@test(should_panic)",
    );
}

#[test]
fn case_27_make_public_edits_the_sibling_module() {
    for nl in ["\n", "\r\n"] {
        let main = "import lib::{hidden}\n\nfn main() {\n    println(\"${hidden()}\")\n}\n"
            .replace('\n', nl);
        let lib = "/// The secret.\nfn hidden() -> Int {\n    1\n}\n".replace('\n', nl);
        offers(
            &[(MAIN, main.as_str()), ("mem/lib.nova", lib.as_str())],
            "E0001",
            "`hidden` is not a public item of module `lib`",
            "make `hidden` public in `lib`",
            "lib.nova",
            &format!("/// The secret.{nl}pub fn hidden()"),
        );
    }
}

#[test]
fn case_28_make_public_from_tests_edits_the_packages_library() {
    for (k, nl) in ["\n", "\r\n"].into_iter().enumerate() {
        let lib = "fn name() -> String {\n    \"app\"\n}\n".replace('\n', nl);
        let test = "import demo::{name}\n\n@test\nfn t() {\n    assert_eq(name(), \"app\")\n}\n"
            .replace('\n', nl);
        let dir = project(
            &format!("public-from-tests-{k}"),
            &[
                ("src/lib.nova", lib.as_str()),
                ("tests/t.nova", test.as_str()),
            ],
        );
        let none = Buffers(Vec::new());
        let a = whole(&dir, &none);
        let d = diagnostic(&a, "E0001", "`name` is not a public item of module `demo`");
        let edited = apply(&a, fix(d, "make `name` public in `demo`"), &none);
        let after = text(&edited, "lib.nova");
        assert!(after.starts_with("pub fn name()"), "{after}");
        assert_fixes(&a, &whole(&dir, &edited), "E0001", &[]);
    }
}

#[test]
fn case_29_no_make_public_for_a_dependencys_item() {
    let app = app_and_geom(
        "public-dependency",
        &[("src/main.nova", "import geom::{secret}\n\nfn main() {}\n")],
        "fn secret() -> Int {\n    1\n}\n\npub fn area() -> Int {\n    1\n}\n",
    );
    let a = whole(&app, &Buffers(Vec::new()));
    let d = diagnostic(
        &a,
        "E0001",
        "`secret` is not a public item of module `geom`",
    );
    assert!(d.fixes.is_empty(), "{:?}", d.fixes);
}

#[test]
fn case_30_remove_an_unreachable_arm() {
    for nl in ["\n", "\r\n"] {
        let after_any = format!("        _ => \"any\"{nl}    }}");
        // On its own line.
        let src = "fn main() {\n    let x = 1\n    let s = match x {\n        _ => \"any\"\n        1 => \"one\"\n    }\n    println(s)\n}\n"
            .replace('\n', nl);
        offers(
            &[(MAIN, src.as_str())],
            "E0021",
            "unreachable match arm",
            "remove the unreachable arm",
            "main.nova",
            &after_any,
        );
        // Over several lines.
        let src = "fn main() {\n    let x = 1\n    let s = match x {\n        _ => \"any\"\n        1 => {\n            \"one\"\n        }\n    }\n    println(s)\n}\n"
            .replace('\n', nl);
        offers(
            &[(MAIN, src.as_str())],
            "E0021",
            "unreachable match arm",
            "remove the unreachable arm",
            "main.nova",
            &after_any,
        );
    }
    // Sharing its line, with a trailing comma.
    offers(
        &[(
            MAIN,
            "fn main() {\n    let x = 1\n    let s = match x { _ => \"any\", 1 => \"one\", }\n    println(s)\n}\n",
        )],
        "E0021",
        "unreachable match arm",
        "remove the unreachable arm",
        "main.nova",
        "match x { _ => \"any\", }",
    );
}

#[test]
fn a_dependencys_errors_offer_no_fix() {
    // Spec §3.1 (plan decision 19): a fix never edits another package, and
    // E0060's note stays where its fix is not offered.
    let app = app_and_geom(
        "dependency-errors",
        &[(
            "src/main.nova",
            "import geom\n\nfn main() {\n    println(\"${area()}\")\n}\n",
        )],
        "@tset\nfn helper() {}\n\npub fn area() -> Int {\n    let x = 1\n    x = 2\n    x\n}\n",
    );
    let a = whole(&app, &Buffers(Vec::new()));
    let mutable = diagnostic(&a, "E0060", "cannot assign to immutable variable `x`");
    assert!(mutable.fixes.is_empty(), "{:?}", mutable.fixes);
    assert!(
        mutable.notes.iter().any(|n| n.contains("let mut x")),
        "{:?}",
        mutable.notes
    );
    let attribute = diagnostic(&a, "E0082", "unknown attribute `@tset`");
    assert!(attribute.fixes.is_empty(), "{:?}", attribute.fixes);
}

// === The final review's fix pass ===

#[test]
fn make_public_is_not_offered_when_a_glob_of_the_module_would_clash() {
    // Spec §3.2: `other` glob-imports `lib` and has its own `helper`, so
    // making `lib`'s `helper` public would bind it twice there (E0002).
    let a = loose(&buffers(&[
        (
            MAIN,
            "import lib::{helper}\nimport other\n\nfn main() {\n    println(\"${helper()} ${twice()}\")\n}\n",
        ),
        ("mem/lib.nova", "fn helper() -> Int {\n    1\n}\n"),
        (
            "mem/other.nova",
            "import lib\n\nfn helper() -> Int {\n    2\n}\n\npub fn twice() -> Int {\n    helper() * 2\n}\n",
        ),
    ]));
    let d = diagnostic(&a, "E0001", "`helper` is not a public item of module `lib`");
    assert!(d.fixes.is_empty(), "{:?}", d.fixes);
}
