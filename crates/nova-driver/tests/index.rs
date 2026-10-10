//! The index of name occurrences (spec
//! `docs/superpowers/specs/2026-10-10-phase-3-4a-navigation-design.md` §3,
//! §8.1).

use std::io;
use std::path::{Path, PathBuf};

use nova_diagnostics::FileId;
use nova_driver::{analyze, Analysis, Options, Sources};
use nova_resolver::{Index, Occurrence, Role, Target};

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

fn options() -> Options {
    Options {
        keep_going: true,
        tests: true,
        module_only: true,
        index: true,
        ..Options::default()
    }
}

const MAIN: &str = "mem/main.nova";

/// Analyse `files`, each `(path, text)`; the first is the entry.
fn analyse(files: &[(&str, &str)]) -> Analysis {
    let buffers = Buffers(
        files
            .iter()
            .map(|(p, t)| (PathBuf::from(p), t.to_string()))
            .collect(),
    );
    analyze(Path::new(files[0].0), &buffers, &options()).expect("the entry is readable")
}

fn index(a: &Analysis) -> &Index {
    a.index.as_ref().expect("the index was asked for")
}

fn word_starts<'t>(text: &'t str, word: &'t str) -> impl Iterator<Item = usize> + 't {
    let ident = |c: Option<char>| c.is_some_and(|c| c.is_ascii_alphanumeric() || c == '_');
    text.match_indices(word).map(|(i, _)| i).filter(move |&i| {
        !ident(text[..i].chars().next_back()) && !ident(text[i + word.len()..].chars().next())
    })
}

/// Where the `n`th whole word `word` (from 0) is in `path`'s text.
#[track_caller]
fn place(a: &Analysis, path: &str, word: &str, n: usize) -> (FileId, u32) {
    let (file, _) = a
        .modules
        .iter()
        .find(|(_, p)| p == Path::new(path))
        .unwrap_or_else(|| panic!("{path} is not a module"));
    let text = a.db.get_source(*file).unwrap();
    let start = word_starts(text, word)
        .nth(n)
        .unwrap_or_else(|| panic!("no `{word}` number {n} in {path}"));
    (*file, start as u32)
}

/// The occurrence a request at the `n`th `word` finds.
#[track_caller]
fn at<'a>(a: &'a Analysis, path: &str, word: &str, n: usize) -> &'a Occurrence {
    let (file, start) = place(a, path, word, n);
    index(a)
        .at(a.definitions.as_ref().unwrap(), file, start)
        .unwrap_or_else(|| panic!("nothing at `{word}` number {n}"))
}

/// The `n`th `word` is a use, and the `d`th `word` in `decl_path`
/// declares what it uses. Returns the target.
#[track_caller]
fn uses(a: &Analysis, path: &str, word: &str, n: usize, decl_path: &str, d: usize) -> Target {
    let o = at(a, path, word, n);
    assert_eq!(o.role, Role::Use, "`{word}` number {n}: {o:?}");
    let decl = index(a)
        .declaration(&o.target)
        .unwrap_or_else(|| panic!("no declaration of {o:?}"));
    assert_eq!(
        (decl.span.file, decl.span.start),
        place(a, decl_path, word, d),
        "`{word}` number {n} is {o:?}"
    );
    o.target
}

/// The `n`th `word` declares something: what.
#[track_caller]
fn declares(a: &Analysis, path: &str, word: &str, n: usize) -> Target {
    let o = at(a, path, word, n);
    assert_eq!(o.role, Role::Declaration, "`{word}` number {n}: {o:?}");
    o.target
}

/// Nothing is recorded over the `n`th `word`.
#[track_caller]
fn nothing_at(a: &Analysis, path: &str, word: &str, n: usize) {
    let (file, start) = place(a, path, word, n);
    let end = start + word.len() as u32;
    let found: Vec<&Occurrence> = index(a)
        .occurrences
        .iter()
        .filter(|o| o.span.file == file && o.span.start < end && start < o.span.end)
        .collect();
    assert!(found.is_empty(), "`{word}` number {n}: {found:?}");
}

/// The name of the file `target` is declared in.
// Task 4's tests are its first callers.
#[allow(dead_code)]
#[track_caller]
fn declared_in(a: &Analysis, target: &Target) -> String {
    let decl = index(a)
        .declaration(target)
        .unwrap_or_else(|| panic!("no declaration of {target:?}"));
    a.db.get_name(decl.span.file).unwrap().to_string()
}

/// The type hover shows for the local declared at the `n`th `word`.
#[track_caller]
fn local_type(a: &Analysis, path: &str, word: &str, n: usize) -> Option<String> {
    let Target::Local(span) = declares(a, path, word, n) else {
        panic!("`{word}` number {n} is not a local");
    };
    index(a).types.get(&span).cloned()
}

// === Task 3: declarations, locals and their types ===

#[test]
fn a_locals_use_resolves_to_its_let() {
    let a = analyse(&[(
        MAIN,
        "fn main() {\n    let total = 1\n    let b = total + 2\n}\n",
    )]);
    let t = uses(&a, MAIN, "total", 1, MAIN, 0);
    assert!(matches!(t, Target::Local(_)), "{t:?}");
}

#[test]
fn a_shadowing_let_is_a_new_local() {
    let a = analyse(&[(
        MAIN,
        "fn main() {\n    let x = 1\n    let x = x + 1\n    let y = x\n}\n",
    )]);
    // `x + 1` uses the first `x`; `let y = x` the second.
    uses(&a, MAIN, "x", 2, MAIN, 0);
    uses(&a, MAIN, "x", 3, MAIN, 1);
}

#[test]
fn parameters_self_and_an_assignments_target_are_locals() {
    let text = "record Counter { n: Int }\n\
impl Counter {\n    fn bump(mut self, by: Int) -> Int {\n        let mut step = by\n        step = step + 1\n        self.n + step\n    }\n}\n\
fn main() {}\n";
    let a = analyse(&[(MAIN, text)]);
    uses(&a, MAIN, "by", 1, MAIN, 0);
    uses(&a, MAIN, "step", 1, MAIN, 0);
    uses(&a, MAIN, "step", 2, MAIN, 0);
    uses(&a, MAIN, "step", 3, MAIN, 0);
    uses(&a, MAIN, "self", 1, MAIN, 0);
}

#[test]
fn closure_parameters_and_for_variables_are_locals() {
    let text = "fn main() {\n    let add = |k: Int| k + 1\n    for i in 0..3 {\n        let j = i\n    }\n}\n";
    let a = analyse(&[(MAIN, text)]);
    uses(&a, MAIN, "k", 1, MAIN, 0);
    uses(&a, MAIN, "i", 1, MAIN, 0);
}

#[test]
fn a_match_binding_and_a_variant_payload_are_locals() {
    let text = "fn main() {\n    let o: Option<Int> = Some(1)\n    let v = match o {\n        Some(n) => n,\n        other => 0,\n    }\n}\n";
    let a = analyse(&[(MAIN, text)]);
    uses(&a, MAIN, "n", 1, MAIN, 0);
    assert!(matches!(declares(&a, MAIN, "other", 0), Target::Local(_)));
}

#[test]
fn items_fields_variants_and_trait_methods_are_declared() {
    let text = "pub const LIMIT: Int = 3\n\
record Point { x: Int, y: Int }\n\
type Shape = | Round(Int) | Flat\n\
trait Show {\n    fn show(self) -> String\n    fn twice(self) -> String { self.show() }\n}\n\
fn main() {}\n";
    let a = analyse(&[(MAIN, text)]);
    assert!(matches!(declares(&a, MAIN, "LIMIT", 0), Target::Def(_)));
    assert!(matches!(declares(&a, MAIN, "main", 0), Target::Def(_)));
    let Target::Def(point) = declares(&a, MAIN, "Point", 0) else {
        panic!("a record is a Def");
    };
    assert_eq!(declares(&a, MAIN, "y", 0), Target::Field(point, 1));
    let Target::Def(shape) = declares(&a, MAIN, "Shape", 0) else {
        panic!("a sum type is a Def");
    };
    assert_eq!(declares(&a, MAIN, "Flat", 0), Target::Variant(shape, 1));
    let Target::Def(show) = declares(&a, MAIN, "Show", 0) else {
        panic!("a trait is a Def");
    };
    assert_eq!(declares(&a, MAIN, "show", 0), Target::TraitMethod(show, 0));
    // A provided method is declared once, as the trait's method, never as
    // its default body's own `Def`.
    assert_eq!(declares(&a, MAIN, "twice", 0), Target::TraitMethod(show, 1));
    let (file, start) = place(&a, MAIN, "twice", 0);
    let here = index(&a)
        .occurrences
        .iter()
        .filter(|o| o.span.file == file && o.span.start == start)
        .count();
    assert_eq!(here, 1);
}

#[test]
fn parameters_without_a_body_are_declared() {
    let text = "trait Area {\n    fn scaled(self, factor: Int) -> Int\n}\n\
extern \"C\" {\n    fn abs(value: Int) -> Int\n}\n\
fn main() {}\n";
    let a = analyse(&[(MAIN, text)]);
    assert!(matches!(declares(&a, MAIN, "factor", 0), Target::Local(_)));
    assert!(matches!(declares(&a, MAIN, "value", 0), Target::Local(_)));
}

#[test]
fn type_parameters_are_declared() {
    let text = "record Pair<A, B> { a: A, b: B }\nfn swap<T>(t: T) -> T { t }\nfn main() {}\n";
    let a = analyse(&[(MAIN, text)]);
    assert!(matches!(declares(&a, MAIN, "A", 0), Target::TypeParam(_)));
    assert!(matches!(declares(&a, MAIN, "T", 0), Target::TypeParam(_)));
}

#[test]
fn a_locals_type_is_read_after_inference() {
    let text = "fn first<T>(xs: [T]) -> T {\n    let x = xs[0]\n    x\n}\n\
fn main() {\n    let mut v = Vec::new()\n    v.push(1)\n    let f = |k| k + 1\n    let w = Vec::new()\n    let g = |q| q\n}\n";
    let a = analyse(&[(MAIN, text)]);
    // A type parameter by its declared name, not `T0`.
    assert_eq!(local_type(&a, MAIN, "x", 0).as_deref(), Some("T"));
    assert_eq!(local_type(&a, MAIN, "v", 0).as_deref(), Some("Vec<Int>"));
    // A closure's local, from its enclosing function's inference.
    assert_eq!(local_type(&a, MAIN, "k", 0).as_deref(), Some("Int"));
    // An unsolved variable as `_`.
    assert_eq!(local_type(&a, MAIN, "w", 0).as_deref(), Some("Vec<_>"));
    // A wholly unknown type is not stored (spec §3.4).
    assert_eq!(local_type(&a, MAIN, "q", 0), None);
}

#[test]
fn nothing_is_recorded_for_an_unresolved_name_a_wildcard_or_a_placeholder() {
    // `a.` is unfinished: the parser gives its member an empty name.
    let text = "fn main() {\n    let _ = 1\n    let a = missing + 1\n    let b = a.\n}\n";
    let a = analyse(&[(MAIN, text)]);
    nothing_at(&a, MAIN, "missing", 0);
    nothing_at(&a, MAIN, "_", 0);
    let empty: Vec<&Occurrence> = index(&a)
        .occurrences
        .iter()
        .filter(|o| o.span.start >= o.span.end)
        .collect();
    assert!(empty.is_empty(), "{empty:?}");
}

#[test]
fn the_index_is_none_unless_asked_for() {
    let buffers = Buffers(vec![(PathBuf::from(MAIN), "fn main() {}\n".to_string())]);
    let options = Options {
        index: false,
        ..options()
    };
    let a = analyze(Path::new(MAIN), &buffers, &options).unwrap();
    assert!(a.index.is_none());
}
