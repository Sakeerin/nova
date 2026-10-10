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

// === Task 4: types, bounds, projections, impl headers ===

#[test]
fn a_type_annotation_uses_its_type_and_arguments() {
    let text = "record Point { x: Int }\n\
fn origin(p: Point) -> Option<Point> { None }\n\
fn main() {\n    let q: Point = Point { x: 1 }\n}\n";
    let a = analyse(&[(MAIN, text)]);
    uses(&a, MAIN, "Point", 1, MAIN, 0); // a parameter's type
    uses(&a, MAIN, "Point", 2, MAIN, 0); // a generic argument
    uses(&a, MAIN, "Point", 3, MAIN, 0); // a let's annotation
    assert_eq!(at(&a, MAIN, "Int", 0).target, Target::Primitive("Int"));
    let option = at(&a, MAIN, "Option", 0).target;
    assert!(declared_in(&a, &option).starts_with("<std/"), "{option:?}");
}

#[test]
fn a_type_parameters_uses_resolve_to_its_declaration() {
    let text = "record Pair<A> { a: A }\n\
impl<K> Pair<K> {\n    fn get<M>(self, m: M) -> K { self.a }\n}\n\
fn wrap<T>(t: T) -> Pair<T> {\n    let p: Pair<T> = Pair { a: t }\n    p\n}\n\
fn main() {}\n";
    let a = analyse(&[(MAIN, text)]);
    uses(&a, MAIN, "A", 1, MAIN, 0);
    uses(&a, MAIN, "K", 1, MAIN, 0); // the impl's self type
    uses(&a, MAIN, "K", 2, MAIN, 0); // a method's return type: the impl's
    uses(&a, MAIN, "M", 1, MAIN, 0);
    uses(&a, MAIN, "T", 1, MAIN, 0);
    uses(&a, MAIN, "T", 2, MAIN, 0);
    uses(&a, MAIN, "T", 3, MAIN, 0); // inside the body
}

#[test]
fn a_projection_uses_its_parameter_and_associated_type() {
    let text = "trait Source {\n    type Item\n    fn take(self) -> Self::Item\n}\n\
record Box { v: Int }\n\
impl Source for Box {\n    type Item = Int\n    fn take(self) -> Self::Item { self.v }\n}\n\
fn pull<S: Source>(s: S) -> S::Item { s.take() }\n\
fn main() {}\n";
    let a = analyse(&[(MAIN, text)]);
    let item = declares(&a, MAIN, "Item", 0);
    assert!(matches!(item, Target::Def(_)), "{item:?}");
    assert_eq!(at(&a, MAIN, "Item", 1).target, item); // the trait's Self::Item
    assert_eq!(at(&a, MAIN, "Item", 2).target, item); // the impl's binding
    assert_eq!(at(&a, MAIN, "Item", 3).target, item); // the impl's Self::Item
    assert_eq!(at(&a, MAIN, "Item", 4).target, item); // S::Item
    uses(&a, MAIN, "S", 2, MAIN, 0); // the projection's base
                                     // `Self` is not recorded (spec decision 20).
    nothing_at(&a, MAIN, "Self", 0);
    nothing_at(&a, MAIN, "Self", 1);
}

#[test]
fn bounds_where_clauses_supertraits_and_impl_headers_use_their_traits() {
    let text = "trait Named { fn name(self) -> String }\n\
trait Loud: Named { fn shout(self) -> String }\n\
record Dog { n: Int }\n\
impl Named for Dog { fn name(self) -> String { \"d\" } }\n\
fn greet<T: Named>(t: T) -> String { t.name() }\n\
fn call<U>(u: U) -> String where U: Named { u.name() }\n\
fn main() {}\n";
    let a = analyse(&[(MAIN, text)]);
    uses(&a, MAIN, "Named", 1, MAIN, 0); // a supertrait
    uses(&a, MAIN, "Named", 2, MAIN, 0); // an impl header
    uses(&a, MAIN, "Named", 3, MAIN, 0); // an inline bound
    uses(&a, MAIN, "Named", 4, MAIN, 0); // a where clause
    uses(&a, MAIN, "Dog", 1, MAIN, 0); // an impl's self type
    uses(&a, MAIN, "U", 2, MAIN, 0); // a where clause's parameter
}

// === Task 5: values, calls, records, fields, methods, patterns, families ===

#[test]
fn calls_and_values_use_their_definitions() {
    let text = "const LIMIT: Int = 3\n\
fn double(n: Int) -> Int { n * 2 }\n\
fn main() {\n    let a = double(LIMIT)\n    let f = double\n    println(\"x\")\n}\n";
    let a = analyse(&[(MAIN, text)]);
    uses(&a, MAIN, "double", 1, MAIN, 0); // a direct call
    uses(&a, MAIN, "double", 2, MAIN, 0); // a function as a value
    uses(&a, MAIN, "LIMIT", 1, MAIN, 0);
    assert!(matches!(
        at(&a, MAIN, "println", 0).target,
        Target::Builtin(_)
    ));
}

#[test]
fn variants_use_their_sum_and_variant() {
    let text = "type Shape = | Round(Int) | Flat\n\
fn main() {\n    let a = Round(1)\n    let b = Shape::Round(2)\n    let c = Flat\n    let d = Shape::Flat\n    let e: Option<Int> = None\n}\n";
    let a = analyse(&[(MAIN, text)]);
    uses(&a, MAIN, "Round", 1, MAIN, 0);
    uses(&a, MAIN, "Round", 2, MAIN, 0);
    uses(&a, MAIN, "Shape", 1, MAIN, 0);
    uses(&a, MAIN, "Flat", 1, MAIN, 0);
    uses(&a, MAIN, "Flat", 2, MAIN, 0);
    uses(&a, MAIN, "Shape", 2, MAIN, 0);
    let none = at(&a, MAIN, "None", 0).target;
    assert!(declared_in(&a, &none).starts_with("<std/"), "{none:?}");
}

#[test]
fn associated_functions_use_their_qualifier_and_function() {
    let text = "record Point { x: Int }\n\
impl Point {\n    fn origin() -> Point { Point { x: 0 } }\n}\n\
trait Zero { fn zero() -> Self }\n\
impl Zero for Int { fn zero() -> Int { 0 } }\n\
fn pick<Z: Zero>() -> Z { Z::zero() }\n\
fn main() {\n    let p = Point::origin()\n    let n = Int::zero()\n}\n";
    let a = analyse(&[(MAIN, text)]);
    uses(&a, MAIN, "origin", 1, MAIN, 0);
    uses(&a, MAIN, "Point", 4, MAIN, 0); // the qualifier of Point::origin
    let Target::Def(zero) = declares(&a, MAIN, "Zero", 0) else {
        panic!("a trait is a Def");
    };
    // Through a bound and through an impl: the trait's method.
    assert_eq!(at(&a, MAIN, "zero", 2).target, Target::TraitMethod(zero, 0));
    assert_eq!(at(&a, MAIN, "zero", 3).target, Target::TraitMethod(zero, 0));
    uses(&a, MAIN, "Z", 2, MAIN, 0);
    // `Int` number 3 is the qualifier of `Int::zero()`.
    assert_eq!(at(&a, MAIN, "Int", 3).target, Target::Primitive("Int"));
}

#[test]
fn record_literals_and_field_accesses_use_their_fields() {
    let text = "record Point { x: Int, y: Int }\n\
fn main() {\n    let x = 1\n    let mut p = Point { x, y: 2 }\n    p.y = p.x + p.y\n}\n";
    let a = analyse(&[(MAIN, text)]);
    let Target::Def(point) = declares(&a, MAIN, "Point", 0) else {
        panic!("a record is a Def");
    };
    uses(&a, MAIN, "Point", 1, MAIN, 0);
    // `{ x }`: a field use and a value use, both marked; `at` takes the value.
    let (file, start) = place(&a, MAIN, "x", 2);
    let here: Vec<&Occurrence> = index(&a)
        .occurrences
        .iter()
        .filter(|o| o.span.file == file && o.span.start == start)
        .collect();
    assert_eq!(here.len(), 2, "{here:?}");
    assert!(here.iter().all(|o| o.shorthand), "{here:?}");
    assert!(
        here.iter().any(|o| o.target == Target::Field(point, 0)),
        "{here:?}"
    );
    uses(&a, MAIN, "x", 2, MAIN, 1);
    assert_eq!(at(&a, MAIN, "y", 1).target, Target::Field(point, 1)); // `y: 2`
    assert_eq!(at(&a, MAIN, "y", 2).target, Target::Field(point, 1)); // the write
    assert_eq!(at(&a, MAIN, "x", 3).target, Target::Field(point, 0)); // `p.x`
    assert_eq!(at(&a, MAIN, "y", 3).target, Target::Field(point, 1)); // `p.y`
}

#[test]
fn method_calls_use_their_method() {
    let text = "record Counter { n: Int }\n\
impl Counter {\n    fn get(self) -> Int { self.n }\n}\n\
trait Show { fn show(self) -> String }\n\
impl Show for Counter { fn show(self) -> String { \"c\" } }\n\
fn main() {\n    let c = Counter { n: 1 }\n    let a = c.get()\n    let b = c.show()\n    let xs = [1, 2]\n    let n = xs.len()\n}\n";
    let a = analyse(&[(MAIN, text)]);
    uses(&a, MAIN, "get", 1, MAIN, 0);
    let Target::Def(show) = declares(&a, MAIN, "Show", 0) else {
        panic!("a trait is a Def");
    };
    assert_eq!(at(&a, MAIN, "show", 2).target, Target::TraitMethod(show, 0));
    assert_eq!(at(&a, MAIN, "len", 0).target, Target::BuiltinMethod("len"));
}

#[test]
fn patterns_use_their_variants() {
    let text = "type Shape = | Round(Int) | Flat\n\
fn size(s: Shape) -> Int {\n    match s {\n        Round(r) => r,\n        Shape::Flat => 0,\n    }\n}\n\
fn other(s: Shape) -> Int {\n    match s {\n        Shape::Round(r) => r,\n        Flat => 0,\n    }\n}\n\
fn main() {}\n";
    let a = analyse(&[(MAIN, text)]);
    uses(&a, MAIN, "Round", 1, MAIN, 0);
    uses(&a, MAIN, "Flat", 1, MAIN, 0);
    uses(&a, MAIN, "Round", 2, MAIN, 0);
    uses(&a, MAIN, "Flat", 2, MAIN, 0);
    uses(&a, MAIN, "Shape", 2, MAIN, 0);
    uses(&a, MAIN, "Shape", 4, MAIN, 0);
    uses(&a, MAIN, "r", 1, MAIN, 0);
}

#[test]
fn an_impl_method_is_in_its_trait_methods_family() {
    let text = "trait Show { fn show(self) -> String }\n\
record A { n: Int }\nrecord B { n: Int }\n\
impl Show for A { fn show(self) -> String { \"a\" } }\n\
impl Show for B { fn show(self) -> String { \"b\" } }\n\
fn main() {}\n";
    let a = analyse(&[(MAIN, text)]);
    let trait_method = declares(&a, MAIN, "show", 0);
    let a_show = declares(&a, MAIN, "show", 1);
    let b_show = declares(&a, MAIN, "show", 2);
    assert!(
        matches!(trait_method, Target::TraitMethod(..)),
        "{trait_method:?}"
    );
    assert_eq!(
        index(&a).family(&a_show),
        vec![trait_method, a_show, b_show]
    );
}

#[test]
fn the_checkers_own_names_are_not_recorded() {
    // Spec §3.5: a `for` loop's `next` and interpolation's `fmt` are calls
    // the checker makes, not names the user wrote. Every occurrence in the
    // file covers an identifier the source holds, and none is a trait
    // method, since the source calls none.
    let text = "record P { n: Int }\n\
impl Display for P {\n    fn fmt(self) -> String { \"p\" }\n}\n\
fn main() {\n    let p = P { n: 1 }\n    let s = \"${p}\"\n    let mut v = Vec::new()\n    v.push(1)\n    for x in v.iter() {\n        let y = x\n    }\n}\n";
    let a = analyse(&[(MAIN, text)]);
    let (file, _) = &a.modules[0];
    let source = a.db.get_source(*file).unwrap();
    for o in index(&a)
        .occurrences
        .iter()
        .filter(|o| o.span.file == *file)
    {
        assert!(!matches!(o.target, Target::TraitMethod(..)), "{o:?}");
        let word = &source[o.span.start as usize..o.span.end as usize];
        assert!(
            !word.is_empty() && word.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'),
            "{o:?} covers `{word}`"
        );
    }
}

#[test]
fn the_index_records_how_each_local_was_declared() {
    // Spec 3.4b §4.6, §7.2: parameters and `mut`, for semantic tokens.
    let src = "fn f(mut a: Int, b: Int) -> Int {\n    let mut c = a\n    let d = b\n    c = d\n    c\n}\n\nfn main() {}\n";
    let a = analyse(&[(MAIN, src)]);
    let flags = |word: &str| {
        let (file, start) = place(&a, MAIN, word, 0);
        let span = nova_diagnostics::Span::new(start, start + word.len() as u32, file);
        index(&a).locals.get(&span).copied()
    };
    let declared = |parameter, mutable| Some(nova_resolver::LocalFlags { parameter, mutable });
    assert_eq!(flags("a"), declared(true, true));
    assert_eq!(flags("b"), declared(true, false));
    assert_eq!(flags("c"), declared(false, true));
    assert_eq!(flags("d"), declared(false, false));
}
