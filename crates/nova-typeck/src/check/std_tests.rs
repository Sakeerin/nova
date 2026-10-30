//! The checker's tests of 5.0c's std (spec
//! `docs/superpowers/specs/2026-10-26-phase-5-0-the-language-and-std-a-compiler-needs-design.md`
//! §5):
//! - `TyHead::Array`, `[T]`'s impls and `Vec`'s helpers, and the E0010,
//!   E0014, E0060 and E0074 an impl on arrays meets;
//! - E0105, a program's trait method one of std's inherent methods hides;
//! - the std-only builtins kept from user code, and the public form a
//!   program that names one is pointed to;
//! - the new items' types;
//! - `OrderedMap`'s types;
//! - `std/process`'s types, and its builtins kept from user code.

use super::*;
use nova_diagnostics::{FileId, Severity};
use nova_lexer::lex;
use nova_parser::parse;
use nova_resolver::resolve;

/// `src` checked, the resolver's diagnostics first.
fn checked(src: &str) -> CheckResult {
    let file_id = FileId::DUMMY;
    let (tokens, lex_errors) = lex(src, file_id);
    assert!(lex_errors.is_empty(), "lex errors: {lex_errors:?}");
    let (ast, parse_errors) = parse(&tokens, file_id);
    assert!(parse_errors.is_empty(), "parse errors: {parse_errors:?}");
    let resolved = resolve(&ast.expect("an AST"));
    let mut result = check(&resolved.file, &resolved.definitions);
    let mut all = resolved.diagnostics;
    all.append(&mut result.diagnostics);
    result.diagnostics = all;
    result
}

fn errors(src: &str) -> Vec<Diagnostic> {
    checked(src)
        .diagnostics
        .into_iter()
        .filter(|d| d.severity == Severity::Error)
        .collect()
}

fn clean(src: &str) {
    let all = checked(src).diagnostics;
    assert!(all.is_empty(), "{all:#?}");
}

/// The type of the local `name` in `main`.
fn local_ty(r: &CheckResult, name: &str) -> Ty {
    r.module
        .functions
        .iter()
        .find(|f| f.name == "main")
        .expect("a `main`")
        .locals
        .iter()
        .find(|l| l.name == name)
        .unwrap_or_else(|| panic!("a local `{name}`"))
        .ty
        .clone()
}

#[test]
fn an_arrays_head_is_array_whatever_its_element() {
    assert_eq!(Ty::Array(Box::new(Ty::Int)).head(), Some(TyHead::Array));
    assert_eq!(
        Ty::Array(Box::new(Ty::Array(Box::new(Ty::String)))).head(),
        Some(TyHead::Array)
    );
}

#[test]
fn std_array_methods_type_check_through_the_arrays_head() {
    let r = checked(
        "fn main() {\n    let a = [1, 2, 3]\n    let s = a.slice(0, 2)\n    \
         let it = a.iter()\n    let n = a.len()\n    let w = [\"x\"].slice(0, 1)\n}\n",
    );
    assert!(r.diagnostics.is_empty(), "{:#?}", r.diagnostics);
    assert_eq!(local_ty(&r, "s"), Ty::Array(Box::new(Ty::Int)));
    assert_eq!(local_ty(&r, "w"), Ty::Array(Box::new(Ty::String)));
    assert_eq!(local_ty(&r, "n"), Ty::Int);
    assert!(
        matches!(&local_ty(&r, "it"), Ty::Record { args, .. } if args.as_slice() == [Ty::Int]),
        "{:?}",
        local_ty(&r, "it")
    );
}

#[test]
fn an_array_iterators_adapters_infer_their_item() {
    clean(
        "fn main() {\n    let v = [1, 2, 3].iter().map(|x| x * 2).collect()\n    \
         println(\"${v.at(0)}\")\n}\n",
    );
}

/// A program may write an inherent impl on arrays, as on `String` (spec
/// §5.5), generic or on one element type; the concrete one applies only to
/// its element type.
#[test]
fn a_program_may_write_an_impl_on_arrays() {
    clean(
        "impl<T> [T] {\n    fn second(self) -> T { self[1] }\n}\n\
         impl [Int] {\n    fn total(self) -> Int {\n        let mut t = 0\n        \
         for x in self { t += x }\n        t\n    }\n}\n\
         fn main() {\n    println(\"${[\"a\", \"b\"].second()} ${[1, 2].total()}\")\n}\n",
    );
    let e = errors(
        "impl [Int] {\n    fn total(self) -> Int { 0 }\n}\n\
         fn main() {\n    println(\"${[\"a\"].total()}\")\n}\n",
    );
    assert_eq!(e.len(), 1, "{e:#?}");
    assert_eq!(e[0].code, "E0014");
    assert_eq!(e[0].message, "no method `total` on array type `[String]`");
}

/// A program's own `iter` on arrays takes the name std's has, which two
/// overlapping inherent impls may not both define.
#[test]
fn a_programs_array_method_that_std_has_is_e0074() {
    let e = errors("impl<T> [T] {\n    fn iter(self) -> Int { 0 }\n}\nfn main() {}\n");
    assert!(
        e.iter().any(|d| d.code == "E0074"
            && d.message == "method `iter` is defined by multiple overlapping inherent impls"),
        "{e:#?}"
    );
}

/// A program's own `len` on arrays takes the name of arrays' built-in
/// `len`, which a call takes first, so its impl is E0074, as `impl String`'s
/// or `impl<T> Vec<T>`'s `len` is, labelled at the program's impl, its
/// primary label, and at std's `impl<T> [T]` (5.0 spec §5.5; the review of
/// 5.0c, which found `[1, 2, 3].len()` printing 3 past such an impl, with
/// no diagnostic). With arguments or without, `len` is the builtin's name.
#[test]
fn a_programs_len_on_arrays_is_e0074() {
    for (src, ty) in [
        (
            "impl [Int] {\n    fn len(self) -> Int { 99 }\n}\n\
             fn main() {\n    println(\"${[1, 2, 3].len()}\")\n}\n",
            "[Int]",
        ),
        (
            "impl<T> [T] {\n    fn len(self, n: Int) -> Int { n }\n}\nfn main() {}\n",
            "[T]",
        ),
    ] {
        let e = errors(src);
        let d = e
            .iter()
            .find(|d| d.code == "E0074")
            .unwrap_or_else(|| panic!("an E0074 for {ty}: {e:#?}"));
        assert_eq!(
            d.message,
            "method `len` is defined by multiple overlapping inherent impls"
        );
        let at = src.find(ty).expect("the impl's type") as u32;
        assert!(
            d.labels.iter().any(|l| l.primary
                && l.message == "here"
                && l.span.start == at
                && l.span.end == at + ty.len() as u32),
            "{d:#?}"
        );
        assert!(
            d.labels.iter().any(|l| !l.primary
                && l.message == "`len` is also defined by std, for every array"),
            "{d:#?}"
        );
    }
}

/// A method no impl has is still E0014, and its fix offers `len` with the
/// impls' methods.
#[test]
fn an_unknown_array_method_is_e0014_with_its_methods_offered() {
    let e = errors("fn main() {\n    let n = [1, 2].sclice(0, 1)\n}\n");
    assert_eq!(e.len(), 1, "{e:#?}");
    assert_eq!(e[0].code, "E0014");
    assert_eq!(e[0].message, "no method `sclice` on array type `[Int]`");
    assert_eq!(
        e[0].fixes
            .iter()
            .map(|f| f.title.as_str())
            .collect::<Vec<_>>(),
        ["change `sclice` to `slice`"]
    );
    let e = errors("fn main() {\n    let n = [1, 2].lne()\n}\n");
    assert_eq!(
        e[0].fixes
            .iter()
            .map(|f| f.title.as_str())
            .collect::<Vec<_>>(),
        ["change `lne` to `len`"]
    );
}

/// The std-only builtins std/core and std/strings gain are std's own: a
/// program that names one finds nothing (spec §5.2 to §5.4).
#[test]
fn the_new_builtins_are_not_visible_to_a_program() {
    for (call, name) in [
        ("int_to_char(65)", "int_to_char"),
        ("str_as_bytes(\"a\")", "str_as_bytes"),
        ("str_is_float(\"1\")", "str_is_float"),
        ("float_to_bits(1.0)", "float_to_bits"),
    ] {
        let e = errors(&format!("fn main() {{\n    let x = {call}\n}}\n"));
        assert!(
            e.iter()
                .any(|d| d.code == "E0001" && d.message.contains(&format!("`{name}`"))),
            "{name}: {e:#?}"
        );
    }
}

/// `option(t, inner)`: whether `t` is an `Option` of `inner`.
fn option(t: &Ty, inner: Ty) -> bool {
    matches!(t, Ty::Sum { args, .. } if args.as_slice() == [inner])
}

/// `Vec`'s helpers' types (spec §5.5).
#[test]
fn vecs_helpers_have_the_specs_types() {
    let r = checked(
        "fn main() {\n    let mut v: Vec<Int> = Vec::from_array([1])\n    \
         let va = v.to_array()\n    let at = v.at(0)\n    let last = v.last()\n}\n",
    );
    assert!(r.diagnostics.is_empty(), "{:#?}", r.diagnostics);
    assert_eq!(local_ty(&r, "va"), Ty::Array(Box::new(Ty::Int)));
    assert_eq!(local_ty(&r, "at"), Ty::Int);
    assert_eq!(local_ty(&r, "last"), Ty::Int);
}

/// The builder's types, and a `String`'s length in bytes (spec §5.1,
/// §5.2).
#[test]
fn the_builder_has_the_specs_types() {
    let r = checked(
        "fn main() {\n    let bl = \"a\".byte_len()\n    \
         let mut sb = StringBuilder::new()\n    sb.push(\"x\")\n    sb.push_char('y')\n    \
         let sl = sb.byte_len()\n    let se = sb.is_empty()\n    let st = sb.to_string()\n    \
         sb.clear()\n}\n",
    );
    assert!(r.diagnostics.is_empty(), "{:#?}", r.diagnostics);
    for name in ["bl", "sl"] {
        assert_eq!(local_ty(&r, name), Ty::Int, "{name}");
    }
    assert_eq!(local_ty(&r, "se"), Ty::Bool);
    assert_eq!(local_ty(&r, "st"), Ty::String);
}

/// The conversions' and byte access's types (spec §5.2).
#[test]
fn the_conversions_and_byte_access_have_the_specs_types() {
    let r = checked(
        "fn main() {\n    let c = char_from_int(65)\n    let i = 'a'.to_int()\n    \
         let s = string_from_chars(['a'])\n    let ba = \"a\".byte_at(0)\n    \
         let bs = \"ab\".byte_slice(0, 1)\n}\n",
    );
    assert!(r.diagnostics.is_empty(), "{:#?}", r.diagnostics);
    assert!(option(&local_ty(&r, "c"), Ty::Char));
    for name in ["i", "ba"] {
        assert_eq!(local_ty(&r, name), Ty::Int, "{name}");
    }
    for name in ["s", "bs"] {
        assert_eq!(local_ty(&r, name), Ty::String, "{name}");
    }
}

/// `Float.to_bits`'s type (spec §5.4).
#[test]
fn to_bits_has_the_specs_type() {
    let r = checked("fn main() {\n    let fb = (1.0).to_bits()\n}\n");
    assert!(r.diagnostics.is_empty(), "{:#?}", r.diagnostics);
    assert_eq!(local_ty(&r, "fb"), Ty::Int);
}

/// The parsers' types (spec §5.3).
#[test]
fn the_parsers_have_the_specs_types() {
    let r = checked(
        "fn main() {\n    let pi = \"1\".parse_int()\n    \
         let pr = \"1\".parse_int_radix(16)\n    let pf = \"1\".parse_float()\n}\n",
    );
    assert!(r.diagnostics.is_empty(), "{:#?}", r.diagnostics);
    assert!(option(&local_ty(&r, "pi"), Ty::Int));
    assert!(option(&local_ty(&r, "pr"), Ty::Int));
    assert!(option(&local_ty(&r, "pf"), Ty::Float));
}

/// `push` and `clear` take `mut self`, as `Vec`'s do.
#[test]
fn a_builder_mutates_only_through_a_mutable_binding() {
    let e = errors("fn main() {\n    let b = StringBuilder::new()\n    b.push(\"x\")\n}\n");
    assert_eq!(e.len(), 1, "{e:#?}");
    assert_eq!(e[0].code, "E0060");
}

/// The six process builtins are std's own (spec §5.7): a program that
/// names one finds nothing.
#[test]
fn the_process_builtins_are_not_visible_to_a_program() {
    for (call, name) in [
        ("process_run(\"p\", [], true)", "process_run"),
        ("process_take_status()", "process_take_status"),
        ("process_take_stdout()", "process_take_stdout"),
        ("process_take_stderr()", "process_take_stderr"),
        ("process_env(\"HOME\")", "process_env"),
        ("process_current_exe()", "process_current_exe"),
    ] {
        let e = errors(&format!("fn main() {{\n    let x = {call}\n}}\n"));
        assert!(
            e.iter()
                .any(|d| d.code == "E0001" && d.message.contains(&format!("`{name}`"))),
            "{name}: {e:#?}"
        );
    }
}

/// `OrderedMap`'s types (spec §5.6).
#[test]
fn ordered_map_has_the_specs_types() {
    let r = checked(
        "fn main() {\n    let mut m: OrderedMap<String, Int> = OrderedMap::new()\n    \
         let ins = m.insert(\"a\", 1)\n    let g = m.get(\"a\")\n    \
         let rm = m.remove(\"a\")\n    let ck = m.contains_key(\"a\")\n    \
         let n = m.len()\n    let ie = m.is_empty()\n    let ks = m.keys()\n    \
         let vs = m.values()\n    let es = m.entries()\n    let mut it = m.iter()\n    \
         let nx = it.next()\n}\n",
    );
    assert!(r.diagnostics.is_empty(), "{:#?}", r.diagnostics);
    for name in ["ins", "g", "rm"] {
        assert!(option(&local_ty(&r, name), Ty::Int), "{name}");
    }
    let pair = Ty::Tuple(vec![Ty::String, Ty::Int]);
    assert!(option(&local_ty(&r, "nx"), pair.clone()));
    assert_eq!(local_ty(&r, "n"), Ty::Int);
    for name in ["ck", "ie"] {
        assert_eq!(local_ty(&r, name), Ty::Bool, "{name}");
    }
    assert_eq!(local_ty(&r, "ks"), Ty::Array(Box::new(Ty::String)));
    assert_eq!(local_ty(&r, "vs"), Ty::Array(Box::new(Ty::Int)));
    assert_eq!(local_ty(&r, "es"), Ty::Array(Box::new(pair)));
}

/// `std/process`'s types (spec §5.7).
#[test]
fn std_process_has_the_specs_types() {
    let r = checked(
        "fn main() {\n    let o = run(\"p\", [\"a\"]).unwrap()\n    let os = o.status\n    \
         let oo = o.stdout\n    let oe = o.stderr\n    \
         let st = status(\"p\", []).unwrap()\n    let ev = env(\"HOME\")\n    \
         let ce = current_exe().unwrap()\n}\n",
    );
    assert!(r.diagnostics.is_empty(), "{:#?}", r.diagnostics);
    assert!(option(&local_ty(&r, "ev"), Ty::String));
    for name in ["os", "st"] {
        assert_eq!(local_ty(&r, name), Ty::Int, "{name}");
    }
    assert_eq!(local_ty(&r, "oo"), Ty::Bytes);
    assert_eq!(local_ty(&r, "oe"), Ty::Bytes);
    assert_eq!(local_ty(&r, "ce"), Ty::String);
}

/// `insert` and `remove` take `mut self`, as `Map`'s do.
#[test]
fn an_ordered_map_mutates_only_through_a_mutable_binding() {
    let e = errors(
        "fn main() {\n    let m: OrderedMap<Int, Int> = OrderedMap::new()\n    \
         let _ = m.insert(1, 2)\n}\n",
    );
    assert_eq!(e.len(), 1, "{e:#?}");
    assert_eq!(e[0].code, "E0060");
}

/// `run`, `status` and `env` are in every module's scope, and a program's
/// own definition of one shadows it, silently, as one of `args` does (spec
/// §5.7, §10 risk 7).
#[test]
fn a_programs_own_run_status_and_env_shadow_stds() {
    clean(
        "fn run(n: Int) -> Int { n }\nfn status() -> String { \"ok\" }\n\
         fn main() {\n    let env = 3\n    let a = run(env)\n    let b = status()\n    \
         println(\"${a} ${b}\")\n}\n",
    );
}

/// `run` takes its arguments as an array of `String`s, not one string a
/// shell would split (spec §5.7).
#[test]
fn run_takes_an_array_of_strings() {
    let e = errors("fn main() {\n    let _ = run(\"ls\", \"-l\")\n}\n");
    assert!(e.iter().any(|d| d.code == "E0010"), "{e:#?}");
}

/// A program's impl that defines a method std's impl on the same type has,
/// `iter` on arrays, is E0074 labelled at the program's impl, its primary
/// label, and at std's, which the program cannot change (5.0 spec §5.5;
/// 5.0c's second fact-check).
#[test]
fn e0074_against_stds_array_impl_labels_the_programs_impl() {
    let src = "impl<T> [T] {\n    fn iter(self) -> Int { 0 }\n}\nfn main() {}\n";
    let e = errors(src);
    let d = e.iter().find(|d| d.code == "E0074").expect("an E0074");
    let at = src.find("[T]").expect("the impl's type") as u32;
    assert!(
        d.labels
            .iter()
            .any(|l| l.primary && l.span.start == at && l.span.end == at + 3),
        "{d:#?}"
    );
    assert!(
        d.labels
            .iter()
            .any(|l| !l.primary && l.message == "`iter` is also defined by std's impl"),
        "{d:#?}"
    );
}

/// A std-only builtin that std's public surface wraps, named by a program,
/// is E0001 with a note naming that surface, and no "did you mean": its
/// nearest name can be another operation, `char_from_int` for
/// `char_to_int` (5.0 spec §5.2 to §5.4).
#[test]
fn a_std_only_builtin_named_by_a_program_gets_its_public_form() {
    for (call, form) in [
        ("char_to_int('a')", "`c.to_int()`"),
        ("int_to_char(65)", "`char_from_int(n)`"),
        ("str_from_chars(['a'])", "`string_from_chars(cs)`"),
        ("str_as_bytes(\"a\")", "`byte_at(i)`"),
        ("str_is_float(\"1\")", "`s.parse_float()`"),
        ("str_to_float(\"1\")", "`s.parse_float()`"),
        ("float_to_bits(1.0)", "`x.to_bits()`"),
    ] {
        let e = errors(&format!("fn main() {{\n    let x = {call}\n}}\n"));
        let d = e.iter().find(|d| d.code == "E0001").expect("an E0001");
        assert!(d.fixes.is_empty(), "{call}: {:#?}", d.fixes);
        assert!(
            d.notes
                .iter()
                .any(|n| n.contains("is internal to std; use") && n.contains(form)),
            "{call}: {:#?}",
            d.notes
        );
    }
}

/// The note is for a value or a function wanted: the same name where a type
/// is wanted is E0001 "cannot find type", with no note, since no builtin is
/// a type (the review of 5.0c).
#[test]
fn a_std_only_builtins_name_as_a_type_gets_no_public_form() {
    let e = errors("fn f(x: str_as_bytes) {}\nfn main() {}\n");
    let d = e.iter().find(|d| d.code == "E0001").expect("an E0001");
    assert!(d.message.starts_with("cannot find type"), "{d:#?}");
    assert!(
        !d.notes.iter().any(|n| n.contains("is internal to std")),
        "{:#?}",
        d.notes
    );
}

fn warnings(src: &str) -> Vec<Diagnostic> {
    checked(src)
        .diagnostics
        .into_iter()
        .filter(|d| d.severity == Severity::Warning)
        .collect()
}

/// The messages of `src`'s E0105s.
fn e0105s(src: &str) -> Vec<String> {
    warnings(src)
        .into_iter()
        .filter(|d| d.code == "E0105")
        .map(|d| d.message)
        .collect()
}

/// A program's trait method that one of std's inherent methods hides, as
/// `Vec`'s `last` and `at` do since 5.0c, is E0105, a warning at the trait
/// impl's method: a call takes the inherent method (the user's ruling on
/// 5.0c's first fact-check). The type is shown with the impl's own
/// parameter names, a method that takes arguments as `x.at(…)`, and the
/// note's generic function's parameter by a name the impl does not use.
#[test]
fn a_programs_trait_method_hidden_by_a_std_method_is_e0105() {
    let src = "trait Top {\n    fn last(self) -> Int\n    fn at(self, i: Int) -> Int\n}\n\
               impl<T> Top for Vec<T> {\n    fn last(self) -> Int { 0 - 1 }\n    \
               fn at(self, i: Int) -> Int { i }\n}\n\
               fn main() {}\n";
    let w = warnings(src);
    let messages: Vec<&str> = w
        .iter()
        .filter(|d| d.code == "E0105")
        .map(|d| d.message.as_str())
        .collect();
    assert_eq!(
        messages,
        [
            "std's inherent method `last` on `Vec<T>` hides `Top::last`",
            "std's inherent method `at` on `Vec<T>` hides `Top::at`",
        ],
        "{w:#?}"
    );
    let first = w.iter().find(|d| d.code == "E0105").expect("an E0105");
    let at = src.find("last(self) -> Int {").expect("the impl's method") as u32;
    assert!(
        first.labels.iter().any(|l| l.primary && l.span.start == at),
        "{first:#?}"
    );
    assert!(
        first
            .notes
            .iter()
            .any(|n| n.contains("generic over `U: Top`, whose `x.last()` is the trait's")),
        "{first:#?}"
    );
    let second = w
        .iter()
        .rfind(|d| d.code == "E0105")
        .expect("a second E0105");
    assert!(
        second
            .notes
            .iter()
            .any(|n| n.contains("whose `x.at(…)` is the trait's")),
        "{second:#?}"
    );
}

/// E0105's note names the trait's method with the trait's own arguments,
/// `Top::at(x, …)` for one that takes more than `self`, as the call Nova
/// does not have, and the generic function's `x.at(…)` by the trait's
/// arguments too, where the label's `x.at(…)` is std's; its type parameter
/// is `T` unless the impl uses that name, then `U` (the review of 5.0c).
#[test]
fn e0105s_note_names_the_traits_arguments_and_a_fresh_parameter() {
    let note = |src: &str| -> (String, String) {
        let w = warnings(src);
        let d = w.iter().find(|d| d.code == "E0105").expect("an E0105");
        let label = d
            .labels
            .iter()
            .find(|l| l.primary)
            .expect("a primary label")
            .message
            .clone();
        (label, d.notes.join("\n"))
    };
    let (label, n) = note(
        "trait Top {\n    fn at(self, i: Int, j: Int) -> Int\n}\n\
         impl<T> Top for Vec<T> {\n    fn at(self, i: Int, j: Int) -> Int { i }\n}\n\
         fn main() {}\n",
    );
    assert_eq!(
        label,
        "`x.at(…)` on a `Vec<T>` runs std's method, not this one"
    );
    assert!(n.contains("Nova has no `Top::at(x, …)` call"), "{n}");
    assert!(
        n.contains("generic over `U: Top`, whose `x.at(…)` is the trait's"),
        "{n}"
    );
    let (label, n) = note(
        "trait Top {\n    fn at(self) -> Int\n}\n\
         impl Top for Vec<Int> {\n    fn at(self) -> Int { 0 }\n}\n\
         fn main() {}\n",
    );
    assert_eq!(
        label,
        "`x.at(…)` on a `Vec<Int>` runs std's method, not this one"
    );
    assert!(n.contains("Nova has no `Top::at(x)` call"), "{n}");
    assert!(
        n.contains("generic over `T: Top`, whose `x.at()` is the trait's"),
        "{n}"
    );
    let (_, n) = note(
        "trait Mk {\n    fn from_array(a: [Int]) -> Self\n}\n\
         impl<T> Mk for Vec<T> {\n    fn from_array(a: [Int]) -> Vec<T> { Vec::new() }\n}\n\
         fn main() {}\n",
    );
    assert!(
        n.contains("generic over `U: Mk`, whose `U::from_array(…)` is the trait's"),
        "{n}"
    );
}

/// No E0105 for a trait method std has no inherent method of, for one
/// named as std's but on a type std's is not on, for one the program's own
/// inherent method hides, or where only one of the two takes `self`: a
/// method call never picks a selfless inherent function, and a path reaches
/// a static trait function (5.0c's second fact-check).
#[test]
fn a_trait_method_std_does_not_have_is_not_e0105() {
    clean(
        "trait Size {\n    fn width(self) -> Int\n}\n\
         impl Size for String {\n    fn width(self) -> Int { self.len() }\n}\n\
         trait Bytes {\n    fn byte_len(self) -> Int\n}\n\
         impl Bytes for Int {\n    fn byte_len(self) -> Int { 8 }\n}\n\
         trait Halve {\n    fn half(self) -> Int\n}\n\
         impl Halve for Int {\n    fn half(self) -> Int { self / 2 }\n}\n\
         impl Int {\n    fn half(self) -> Int { self / 2 }\n}\n\
         trait Tr {\n    fn from_array(self) -> Int\n}\n\
         impl Tr for Vec<Int> {\n    fn from_array(self) -> Int { 42 }\n}\n\
         trait Fresh {\n    fn new(self) -> Int\n}\n\
         impl Fresh for Vec<Int> {\n    fn new(self) -> Int { 7 }\n}\n\
         fn main() {\n    let n = 3\n    let v: Vec<Int> = Vec::new()\n    \
         println(\"${\"ab\".width()} ${n.byte_len()} ${n.half()} ${v.from_array()} ${v.new()}\")\n}\n",
    );
}

/// A static trait function that std's inherent function of the same name
/// hides at a path, `Vec::from_array(a)`, is E0105 too, its note naming
/// the path and the generic function's `T::from_array(…)`.
#[test]
fn a_static_trait_function_hidden_by_a_std_function_is_e0105() {
    let src = "trait Mk {\n    fn from_array(a: [Int]) -> Self\n}\n\
               impl Mk for Vec<Int> {\n    fn from_array(a: [Int]) -> Vec<Int> { Vec::new() }\n}\n\
               fn main() {}\n";
    assert_eq!(
        e0105s(src),
        ["std's inherent function `from_array` on `Vec<Int>` hides `Mk::from_array`"]
    );
    let w = warnings(src);
    let d = w.iter().find(|d| d.code == "E0105").expect("an E0105");
    assert!(
        d.labels
            .iter()
            .any(|l| l.primary
                && l.message == "`Vec::from_array(…)` runs std's function, not this one"),
        "{d:#?}"
    );
    assert!(
        d.notes
            .iter()
            .any(|n| n.contains("whose `T::from_array(…)` is the trait's")),
        "{d:#?}"
    );
}

/// An array takes inherent impls alone: a trait impl on one stays E0010, as
/// before 5.0c, while `impl [Int] { … }` checks (the user's ruling on 5.0c's
/// first fact-check).
#[test]
fn a_trait_impl_on_an_array_is_e0010() {
    for src in [
        "impl Display for [Int] {\n    fn fmt(self) -> String { \"a\" }\n}\nfn main() {}\n",
        "impl<T> Hash for [T] {\n    fn hash(self) -> Int { 0 }\n}\nfn main() {}\n",
    ] {
        let e = errors(src);
        assert!(
            e.iter()
                .any(|d| d.code == "E0010"
                    && d.message == "a trait cannot be implemented for an array"),
            "{e:#?}"
        );
    }
    clean("impl [Int] {\n    fn total(self) -> Int { 0 }\n}\nfn main() {}\n");
}

/// An array's method is named as the source writes the type, `[T]::slice`,
/// not the resolver's `Arr_T.slice`, in an arity error and in E0060; a
/// generic named type's as `Vec::at`, not `Vec_T.at` (5.0c's fact-checks).
#[test]
fn an_arity_error_on_an_array_method_names_it_as_written() {
    let e = errors("fn main() {\n    let x = [1, 2].slice(1)\n}\n");
    assert!(
        e.iter().any(|d| d.code == "E0016"
            && d.message == "`[T]::slice` takes 2 argument(s) but 1 were supplied"),
        "{e:#?}"
    );
    let e = errors(
        "impl [Int] {\n    fn bump(mut self) { self[0] = 1 }\n}\n\
         fn main() {\n    let a = [1]\n    a.bump()\n}\n",
    );
    assert!(
        e.iter().any(|d| d.code == "E0060"
            && d.message == "`[Int]::bump` mutates its receiver, but `a` is immutable"),
        "{e:#?}"
    );
    let e = errors("fn main() {\n    let v: Vec<Int> = Vec::new()\n    let x = v.at()\n}\n");
    assert!(
        e.iter().any(|d| d.code == "E0016"
            && d.message == "`Vec::at` takes 1 argument(s) but 0 were supplied"),
        "{e:#?}"
    );
}

/// An array of a compound element type is named as the source writes it
/// too: `[Vec<Int>]::bump`, not the resolver's `[Vec_Int]::bump`, and so for
/// an `Option`, a tuple and a generic `Map` element, its parameters as the
/// impl names them (the review of 5.0c).
#[test]
fn an_array_of_compound_elements_names_its_method_as_written() {
    let e = errors(
        "impl [Vec<Int>] {\n    fn bump(mut self) { }\n}\n\
         fn main() {\n    let v: Vec<Int> = Vec::new()\n    let a = [v]\n    a.bump()\n}\n",
    );
    assert!(
        e.iter().any(|d| d.code == "E0060"
            && d.message == "`[Vec<Int>]::bump` mutates its receiver, but `a` is immutable"),
        "{e:#?}"
    );
    for (impl_ty, method, value, shown) in [
        (
            "[Option<String>]",
            "two",
            "[Some(\"a\")]",
            "`[Option<String>]::two` takes 1 argument(s) but 0 were supplied",
        ),
        (
            "[(Int, Int)]",
            "four",
            "[(1, 2)]",
            "`[(Int, Int)]::four` takes 1 argument(s) but 0 were supplied",
        ),
    ] {
        let e = errors(&format!(
            "impl {impl_ty} {{\n    fn {method}(self, n: Int) -> Int {{ n }}\n}}\n\
             fn main() {{\n    let x = {value}.{method}()\n}}\n"
        ));
        assert!(
            e.iter().any(|d| d.code == "E0016" && d.message == shown),
            "{impl_ty}: {e:#?}"
        );
    }
    let e = errors(
        "impl<K, V> [Map<K, V>] {\n    fn three(self, n: Int) -> Int { n }\n}\n\
         fn main() {\n    let m: Map<String, Int> = Map::new()\n    let x = [m].three()\n}\n",
    );
    assert!(
        e.iter().any(|d| d.code == "E0016"
            && d.message == "`[Map<K, V>]::three` takes 1 argument(s) but 0 were supplied"),
        "{e:#?}"
    );
}

/// An array in an interpolation is E0013 without the advice to implement
/// `Display`, which E0010 refuses on an array (5.0c's second fact-check).
#[test]
fn an_interpolated_array_is_e0013_without_the_display_advice() {
    let e = errors("fn main() {\n    let a = [1]\n    println(\"${a}\")\n}\n");
    let d = e.iter().find(|d| d.code == "E0013").expect("an E0013");
    assert_eq!(
        d.message,
        "`[Int]` cannot be interpolated into a string; an array cannot implement \
         `Display`; interpolate its elements, or wrap it in a record that does",
        "{d:#?}"
    );
}

/// `String`'s `byte_len` and `Char`'s `to_int`, std's since Task 2, hide a
/// program's trait methods of those names, a trait default included: E0105
/// at the method, or at the impl for a default it takes.
#[test]
fn a_trait_method_hidden_by_strings_and_chars_methods_is_e0105() {
    let src = "trait Size {\n    fn byte_len(self) -> Int\n}\n\
               impl Size for String {\n    fn byte_len(self) -> Int { self.len() }\n}\n\
               trait Codes {\n    fn to_int(self) -> Int { 1000 }\n}\n\
               impl Codes for Char {}\n\
               fn main() {}\n";
    assert_eq!(
        e0105s(src),
        [
            "std's inherent method `byte_len` on `String` hides `Size::byte_len`",
            "std's inherent method `to_int` on `Char` hides `Codes::to_int`",
        ]
    );
}

/// `Float`'s `to_bits`, std's since Task 4, hides a program's trait method
/// of that name: E0105.
#[test]
fn a_trait_method_hidden_by_floats_to_bits_is_e0105() {
    let src = "trait Bits {\n    fn to_bits(self) -> Int\n}\n\
               impl Bits for Float {\n    fn to_bits(self) -> Int { 42 }\n}\n\
               fn main() {}\n";
    assert_eq!(
        e0105s(src),
        ["std's inherent method `to_bits` on `Float` hides `Bits::to_bits`"]
    );
}

/// The parsers, std's since Task 5, hide a program's trait methods of their
/// names: E0105, its call shown with arguments where std's takes them.
#[test]
fn a_trait_method_hidden_by_the_parsers_is_e0105() {
    let src = "trait Parse {\n    fn parse_int_radix(self, r: Int) -> Option<Int>\n}\n\
               impl Parse for String {\n    \
               fn parse_int_radix(self, r: Int) -> Option<Int> { Some(r) }\n}\n\
               fn main() {}\n";
    let w = warnings(src);
    let d = w.iter().find(|d| d.code == "E0105").expect("an E0105");
    assert_eq!(
        d.message,
        "std's inherent method `parse_int_radix` on `String` hides `Parse::parse_int_radix`"
    );
    assert!(
        d.labels.iter().any(|l| l.primary
            && l.message == "`x.parse_int_radix(…)` on a `String` runs std's method, not this one"),
        "{d:#?}"
    );
}
