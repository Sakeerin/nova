//! Pipeline tests: source → lex → parse → resolve → typeck → MIR.

use nova_diagnostics::FileId;
use nova_lexer::lex;
use nova_mir::lower_module;
use nova_parser::parse;
use nova_resolver::resolve;
use nova_typeck::check;

fn mir_for(src: &str) -> nova_mir::Module {
    let file_id = FileId::DUMMY;
    let (tokens, lex_errors) = lex(src, file_id);
    assert!(lex_errors.is_empty(), "lex errors: {lex_errors:?}");
    let (ast, parse_errors) = parse(&tokens, file_id);
    assert!(parse_errors.is_empty(), "parse errors: {parse_errors:?}");
    let ast = ast.expect("no AST");
    let resolved = resolve(&ast);
    assert!(
        resolved.diagnostics.is_empty(),
        "resolve: {:?}",
        resolved.diagnostics
    );
    let checked = check(&resolved.file, &resolved.definitions);
    assert!(
        checked.diagnostics.is_empty(),
        "typeck: {:?}",
        checked.diagnostics
    );
    lower_module(&checked.module).expect("MIR lowering failed")
}

fn function_names(mir: &nova_mir::Module) -> Vec<&str> {
    mir.functions.iter().map(|f| f.name.as_str()).collect()
}

#[test]
fn hello_world_lowers() {
    let mir = mir_for("fn main() { println(\"hi\") }");
    assert_eq!(function_names(&mir), vec!["main"]);
}

#[test]
fn fibonacci_lowers() {
    let mir = mir_for(
        "fn fib(n: Int) -> Int {\n\
             if n <= 1 { n } else { fib(n - 1) + fib(n - 2) }\n\
         }\n\
         fn main() { println(\"${fib(10)}\") }",
    );
    let names = function_names(&mir);
    assert!(names.contains(&"main"));
    assert!(
        names.iter().any(|n| n.starts_with("fib.")),
        "names: {names:?}"
    );
}

#[test]
fn generics_monomorphize_per_instance() {
    let mir = mir_for(
        "fn identity<T>(x: T) -> T { x }\n\
         fn main() {\n\
             let n = identity(1)\n\
             let s = identity(\"s\")\n\
             println(\"${n}${s}\")\n\
         }",
    );
    let names = function_names(&mir);
    assert!(
        names
            .iter()
            .any(|n| n.starts_with("identity.") && n.ends_with("$i")),
        "names: {names:?}"
    );
    assert!(
        names
            .iter()
            .any(|n| n.starts_with("identity.") && n.ends_with("$s")),
        "names: {names:?}"
    );
    // Two instances + main.
    assert_eq!(mir.functions.len(), 3);
}

#[test]
fn match_on_enum_lowers_to_switch() {
    let mir = mir_for(
        "type Shape = | Circle(Int) | Rect(Int, Int) | Empty\n\
         fn area(s: Shape) -> Int {\n\
             match s { Circle(r) => 3 * r * r, Rect(w, h) => w * h, Empty => 0, }\n\
         }\n\
         fn main() { println(\"${area(Circle(10))}\") }",
    );
    let area = mir
        .functions
        .iter()
        .find(|f| f.name.starts_with("area."))
        .expect("area lowered");
    let has_switch = area
        .blocks
        .iter()
        .any(|b| matches!(b.term, nova_mir::Terminator::Switch { .. }));
    assert!(has_switch, "match should lower to a Switch terminator");
}

#[test]
fn records_lower_to_make_and_field() {
    let mir = mir_for(
        "record Point { x: Int, y: Int }\n\
         fn main() {\n\
             let p = Point { x: 3, y: 4 }\n\
             println(\"${p.x}\")\n\
         }",
    );
    let main = mir
        .functions
        .iter()
        .find(|f| f.name == "main")
        .expect("main");
    let has_make = main.blocks.iter().any(|b| {
        b.stmts
            .iter()
            .any(|s| matches!(s, nova_mir::Stmt::MakeRecord { .. }))
    });
    let has_field = main.blocks.iter().any(|b| {
        b.stmts
            .iter()
            .any(|s| matches!(s, nova_mir::Stmt::RecordField { .. }))
    });
    assert!(has_make, "record literal should lower to MakeRecord");
    assert!(has_field, "field access should lower to RecordField");
}

#[test]
fn field_assignment_lowers_to_set_field() {
    let mir = mir_for(
        "record Point { x: Int, y: Int }\n\
         fn main() {\n\
             let mut p = Point { x: 3, y: 4 }\n\
             p.y = 5\n\
             println(\"${p.y}\")\n\
         }",
    );
    let main = mir
        .functions
        .iter()
        .find(|f| f.name == "main")
        .expect("main");
    let stmts: Vec<&nova_mir::Stmt> = main.blocks.iter().flat_map(|b| b.stmts.iter()).collect();
    let sets: Vec<&nova_mir::Stmt> = stmts
        .iter()
        .copied()
        .filter(|s| matches!(s, nova_mir::Stmt::SetField { .. }))
        .collect();
    assert_eq!(sets.len(), 1, "one field store, got {sets:?}");
    let nova_mir::Stmt::SetField { index, ty, .. } = sets[0] else {
        unreachable!("filtered to SetField")
    };
    // `y` is the second field, so the store offset is 8*1 — the same offset
    // `RecordField` loads it from.
    assert_eq!(*index, 1);
    assert_eq!(*ty, nova_mir::MirTy::I64);
}

#[test]
fn generic_record_monomorphizes_field_types() {
    // A generic record used at two element types compiles without error;
    // the point is that lowering handles Record args after substitution.
    let mir = mir_for(
        "record Box<T> { value: T }\n\
         fn main() {\n\
             let a = Box { value: 1 }\n\
             let b = Box { value: \"hi\" }\n\
             println(\"${a.value} ${b.value}\")\n\
         }",
    );
    assert!(mir.functions.iter().any(|f| f.name == "main"));
}

#[test]
fn unreferenced_functions_are_not_emitted() {
    let mir = mir_for("fn unused() { }\nfn main() { }");
    assert_eq!(function_names(&mir), vec!["main"]);
}

#[test]
fn std_core_types_used_without_methods_emit_no_symbols() {
    // `std/core` is compiled into every program as the implicit prelude (ADR
    // 0004), and it grows every phase (already ~20 methods across
    // Option/Result/Display/Debug/Eq/Ord/Clone/Default), so reachability
    // pruning rooted at `main` (`crates/nova-mir/src/mono.rs`) is the only
    // reason a Nova binary isn't bloated by the whole standard library.
    //
    // This is materially stronger than `hello_world_lowers`: that test's
    // `main` never mentions `Option`/`Result` at all, so it can't tell "no
    // std/core symbol leaked" apart from "typeck/lowering never even touched
    // a std/core generic" — a much easier bar to clear. Here `main` binds a
    // `None`, a `Some`, an `Ok`, and an `Err` (all four Option/Result
    // variants) and pattern-matches one of them, so both typeck and MIR
    // lowering must handle std/core's generic sum types directly.
    // Constructing a variant lowers to `MakeVariant` and matching lowers to
    // a `Switch` — neither is a call — and `main` calls none of
    // `Option`/`Result`'s methods (`is_some`, `is_none`, `map`, `and_then`,
    // `unwrap`, `unwrap_or`, `ok_or`, `is_ok`, `is_err`, `map_err`, ...), so
    // pruning must still emit `main` alone.
    let mir = mir_for(
        "fn main() {\n\
             let none_val: Option<Int> = None\n\
             let some_val = Some(1)\n\
             let ok_val: Result<Int, String> = Ok(2)\n\
             let err_val: Result<Int, String> = Err(\"e\")\n\
             match some_val {\n\
                 Some(_) => println(\"some\"),\n\
                 None => println(\"none\"),\n\
             }\n\
         }",
    );
    let names = function_names(&mir);
    assert_eq!(
        names,
        vec!["main"],
        "constructing std/core types without calling their methods leaked a \
         symbol into the module: {names:?}"
    );
}

/// The `std/collections` half of the same guarantee (Phase 2.2a, Task 9).
/// `std/collections` is compiled into every program alongside `std/core`, and
/// it is now ~20 methods across `Vec`/`Map`/`Set` — plus, through `Map`, the
/// `Hash`/`Eq` impls and `mix64` those pull in from `std/core`. Reachability
/// pruning rooted at `main` (`crates/nova-mir/src/mono.rs`) is the only reason
/// a program that never touches a collection does not pay for all of it.
///
/// The program below is deliberately *not* trivial: it uses a repeat-array
/// literal (`[0; n]`, the feature `Vec` and `Map` are built on), a record with
/// an inherent method, a generic function, and an `Option` it matches on. So
/// typeck and MIR lowering both run over the same machinery the collections
/// use — it is only the collections themselves that are absent.
///
/// The marker list is checked for non-vacuity in the same test: a second
/// program that *does* use `Vec`, `Map` and `Set` must contain every marker.
/// Without that control, a typo'd marker (or a mangling-scheme change) would
/// make the first assertion pass for the wrong reason. Together they are what
/// makes this test fail if pruning broke: were the whole std module emitted,
/// the first program's symbols would look like the second's.
#[test]
fn std_collections_unused_emit_no_symbols() {
    // Every collection symbol is mangled from its impl's nominal head, so
    // these four substrings cover the module: `Vec_T.*`, `Map_K_V.*`,
    // `Set_T.*`, and `Map`'s private `mix64` dependency in std/core.
    const MARKERS: [&str; 4] = ["Vec_T", "Map_K_V", "Set_T", "mix64"];

    let mir = mir_for(
        "record P { v: Int }\n\
         impl P { fn scaled(self, k: Int) -> Int { self.v * k } }\n\
         fn first<T>(xs: [T], fallback: T) -> T {\n\
             if xs.len() == 0 { fallback } else { xs[0] }\n\
         }\n\
         fn main() {\n\
             let n = 3\n\
             let a = [7; n]\n\
             let p = P { v: first(a, 0) + a.len() }\n\
             let o: Option<Int> = Some(p.scaled(2))\n\
             match o { Some(x) => println(\"${x}\"), None => println(\"none\"), }\n\
         }",
    );
    let names = function_names(&mir);
    for marker in MARKERS {
        assert!(
            !names.iter().any(|n| n.contains(marker)),
            "a program that touches no collection emitted a `{marker}` symbol — \
             std/collections is no longer pruned and every Nova binary just \
             grew: {names:?}"
        );
    }

    // Control: the markers are real, so the assertion above is not vacuous.
    let used = mir_for(
        "fn main() {\n\
             let mut v = Vec::new()\n\
             v.push(1)\n\
             let mut m = Map::new()\n\
             let _ = m.insert(1, 2)\n\
             let mut s = Set::new()\n\
             let _ = s.insert(3)\n\
             println(\"${v.len()} ${m.len()} ${s.len()}\")\n\
         }",
    );
    let used_names = function_names(&used);
    for marker in MARKERS {
        assert!(
            used_names.iter().any(|n| n.contains(marker)),
            "marker `{marker}` matches no symbol even when the collections are \
             used, so the pruning assertion above proves nothing: {used_names:?}"
        );
    }
}

#[test]
fn trait_method_dispatches_to_impl() {
    let mir = mir_for(
        "record P { v: Int }\n\
         trait Show { fn name(self) -> String }\n\
         impl Show for P { fn name(self) -> String { \"p\" } }\n\
         fn label<T: Show>(x: T) -> String { x.name() }\n\
         fn main() { println(label(P { v: 1 })) }",
    );
    // The impl method function must be emitted and reachable.
    assert!(
        mir.functions.iter().any(|f| f.name.contains("name")),
        "impl method should be monomorphized: {:?}",
        function_names(&mir)
    );
}

#[test]
fn generic_impl_method_monomorphizes_per_element_type() {
    // `impl<T> Box<T> { fn get(self) -> T }` used at Int and String must
    // produce two distinct monomorphized method instances.
    let mir = mir_for(
        "record Box<T> { value: T }\n\
         impl<T> Box<T> { fn get(self) -> T { self.value } }\n\
         fn main() {\n\
             let a = Box { value: 1 }\n\
             let b = Box { value: \"s\" }\n\
             println(\"${a.get()} ${b.get()}\")\n\
         }",
    );
    let names = function_names(&mir);
    assert!(
        names.iter().filter(|n| n.contains("get")).count() >= 2,
        "expected two `get` instances, got {names:?}"
    );
}

#[test]
fn generic_trait_impl_dispatches_to_instance() {
    // A trait method resolved through a generic impl must reach a
    // monomorphized impl-method instance.
    let mir = mir_for(
        "record Box<T> { value: T }\n\
         trait Tag { fn tag(self) -> String }\n\
         impl<T> Tag for Box<T> { fn tag(self) -> String { \"b\" } }\n\
         fn main() { let b = Box { value: 1 }\n println(b.tag()) }",
    );
    assert!(
        mir.functions.iter().any(|f| f.name.contains("tag")),
        "impl method should be monomorphized: {:?}",
        function_names(&mir)
    );
}

#[test]
fn conditional_generic_impl_unsatisfied_reports_e0013() {
    // `Wrap<Bool>` is not `Show` (Bool is not) even though a generic impl of
    // Show for Wrap<T> exists — monomorphization must reject it.
    let codes = diagnostics_for(
        "trait Show { fn show(self) -> String }\n\
         record Wrap<T> { inner: T }\n\
         impl Show for Int { fn show(self) -> String { \"i\" } }\n\
         impl<T: Show> Show for Wrap<T> { fn show(self) -> String { self.inner.show() } }\n\
         fn present<X: Show>(x: X) -> String { x.show() }\n\
         fn main() { println(present(Wrap { inner: true })) }",
    );
    assert!(codes.contains(&"E0013".to_string()), "codes: {codes:?}");
}

#[test]
fn method_generic_monomorphizes_per_instance() {
    // `Box<T>::map<U>` used at U=Int and U=String produces two distinct
    // monomorphized method instances (the method's own generic, not the impl's).
    let mir = mir_for(
        "record Box<T> { value: T }\n\
         impl<T> Box<T> {\n\
             fn map<U>(self, f: fn(T) -> U) -> Box<U> { Box { value: f(self.value) } }\n\
         }\n\
         fn twice(n: Int) -> Int { n * 2 }\n\
         fn label(n: Int) -> String { \"${n}\" }\n\
         fn main() {\n\
             let a = Box { value: 5 }\n\
             let b = a.map(twice)\n\
             let c = a.map(label)\n\
             println(\"${b.value} ${c.value}\")\n\
         }",
    );
    let names = function_names(&mir);
    assert!(
        names.iter().filter(|n| n.contains("map")).count() >= 2,
        "expected two `map` instances, got {names:?}"
    );
}

#[test]
fn method_generic_bound_unsatisfied_reports_e0013() {
    // `tag<U: Show>` called with a `Bool` (no `Show for Bool`) — the method's
    // own generic bound must be checked at monomorphization.
    let codes = diagnostics_for(
        "record Box<T> { value: T }\n\
         trait Show { fn show(self) -> String }\n\
         impl Show for Int { fn show(self) -> String { \"i\" } }\n\
         impl<T> Box<T> { fn tag<U: Show>(self, x: U) -> String { x.show() } }\n\
         fn main() { let a = Box { value: 1 }\n println(a.tag(true)) }",
    );
    assert!(codes.contains(&"E0013".to_string()), "codes: {codes:?}");
}

#[test]
fn where_clause_bound_unsatisfied_reports_e0013() {
    // A `where`-clause bound is enforced at monomorphization exactly like an
    // inline bound: `label<T> where T: Show` called with a `Bool` -> E0013.
    let codes = diagnostics_for(
        "trait Show { fn show(self) -> String }\n\
         impl Show for Int { fn show(self) -> String { \"i\" } }\n\
         fn label<T>(x: T) -> String where T: Show { x.show() }\n\
         fn main() { println(label(true)) }",
    );
    assert!(codes.contains(&"E0013".to_string()), "codes: {codes:?}");
}

#[test]
fn generic_trait_method_monomorphizes_per_instance() {
    // A generic trait method (here a default body) called at U=Int and U=String
    // produces two distinct monomorphized instances.
    let mir = mir_for(
        "trait Mapper { fn raw(self) -> Int\n \
             fn remap<U>(self, f: fn(Int) -> U) -> U { f(self.raw()) } }\n\
         record C { n: Int }\n\
         impl Mapper for C { fn raw(self) -> Int { self.n } }\n\
         fn dbl(n: Int) -> Int { n * 2 }\n\
         fn lbl(n: Int) -> String { \"${n}\" }\n\
         fn main() { let c = C { n: 1 }\n println(\"${c.remap(dbl)} ${c.remap(lbl)}\") }",
    );
    let names = function_names(&mir);
    assert!(
        names.iter().filter(|n| n.contains("remap")).count() >= 2,
        "expected two `remap` instances, got {names:?}"
    );
}

#[test]
fn generic_trait_method_bound_unsatisfied_reports_e0013() {
    // A method-generic bound on a trait method is enforced at monomorphization.
    let codes = diagnostics_for(
        "trait Show { fn show(self) -> String }\n\
         impl Show for Int { fn show(self) -> String { \"i\" } }\n\
         trait Tagger { fn tag<U: Show>(self, x: U) -> String }\n\
         record T { v: Int }\n\
         impl Tagger for T { fn tag<U: Show>(self, x: U) -> String { x.show() } }\n\
         fn main() { let t = T { v: 1 }\n println(t.tag(true)) }",
    );
    assert!(codes.contains(&"E0013".to_string()), "codes: {codes:?}");
}

#[test]
fn repeated_param_trait_impl_mismatch_reports_e0013() {
    // A trait impl on `Pair<T, T>` must not satisfy a bound for
    // `Pair<Int, String>` — structural matching, not just head, gates the
    // monomorphization bound check. (A *direct* call on such a receiver is
    // rejected earlier at typeck as E0014; here the mismatch reaches mono
    // through a generic bound.)
    let codes = diagnostics_for(
        "record Pair<A, B> { first: A, second: B }\n\
         trait Same { fn same(self) -> Int }\n\
         impl<T> Same for Pair<T, T> { fn same(self) -> Int { 1 } }\n\
         fn use_it<X: Same>(x: X) -> Int { x.same() }\n\
         fn main() { let p = Pair { first: 1, second: \"x\" }\n println(\"${use_it(p)}\") }",
    );
    assert!(codes.contains(&"E0013".to_string()), "codes: {codes:?}");
}

/// Build a program that wraps `core` in `depth` layers of `Wrap` and requires
/// the whole thing to be `Display` (only true if `core`'s type is `Display`).
fn deep_wrap_program(core: &str, depth: usize) -> String {
    let mut inner = core.to_string();
    for _ in 0..depth {
        inner = format!("Wrap {{ inner: {inner} }}");
    }
    format!(
        "trait Display {{ fn fmt(self) -> String }}\n\
         record Wrap<T> {{ inner: T }}\n\
         impl Display for Int {{ fn fmt(self) -> String {{ \"i\" }} }}\n\
         impl<T: Display> Display for Wrap<T> {{ fn fmt(self) -> String {{ \"w\" }} }}\n\
         fn describe<T: Display>(x: T) -> String {{ x.fmt() }}\n\
         fn main() {{ let w = {inner}\n println(describe(w)) }}"
    )
}

#[test]
fn deeply_nested_unsatisfiable_bound_is_rejected() {
    // Regression: a depth cap in the bound check once accepted this past ~17
    // levels. `Bool` is never `Display`, so it must be E0013 at any depth.
    let codes = diagnostics_for(&deep_wrap_program("true", 20));
    assert!(codes.contains(&"E0013".to_string()), "codes: {codes:?}");
}

#[test]
fn deeply_nested_satisfiable_bound_is_accepted() {
    // The mirror: an `Int` core is `Display`, so a deep `Wrap` nest must still
    // compile (the fix must not turn the cap into a false rejection).
    let codes = diagnostics_for(&deep_wrap_program("0", 20));
    assert!(codes.is_empty(), "unexpected diagnostics: {codes:?}");
}

#[test]
fn non_overlapping_concrete_impls_lower_to_distinct_functions() {
    // Two concrete impls of one trait for the same head, both called: each must
    // become its own monomorphized function. A prior bug named both
    // `Pair.Foo.foo` (head only), so they collided and one call miscompiled to
    // the other's body.
    let mir = mir_for(
        "record Pair<A, B> { first: A, second: B }\n\
         trait Foo { fn foo(self) -> String }\n\
         impl Foo for Pair<Int, Bool> { fn foo(self) -> String { \"b\" } }\n\
         impl Foo for Pair<Int, Int> { fn foo(self) -> String { \"i\" } }\n\
         fn main() {\n\
             let pb = Pair { first: 1, second: true }\n\
             let pii = Pair { first: 1, second: 2 }\n\
             println(pb.foo())\n\
             println(pii.foo())\n\
         }",
    );
    let foo_fns = mir
        .functions
        .iter()
        .filter(|f| f.name.contains("foo"))
        .count();
    assert_eq!(
        foo_fns,
        2,
        "both concrete impl methods must be emitted as distinct functions: {:?}",
        function_names(&mir)
    );
}

#[test]
fn trait_associated_function_through_bound_lowers_without_a_receiver() {
    // `make<T: Zero>() -> T` at `T = Int` must monomorphize to the `Int` impl's
    // `zero`, and — the point of the test — the lowered call must pass *no*
    // receiver argument. `lower_trait_call` unconditionally prepended the
    // receiver temp, which for a receiver-less callee is the exact shape that
    // makes Cranelift reject the module ("mismatched argument count: got 1,
    // expected 0"), so asserting only that the program lowers is not enough.
    let mir = mir_for(
        "trait Zero { fn zero() -> Self }\n\
         impl Zero for Int { fn zero() -> Int { 0 } }\n\
         fn make<T: Zero>() -> T { T::zero() }\n\
         fn main() { let n: Int = make()\n println(\"${n}\") }",
    );
    let names = function_names(&mir);
    assert!(
        names.iter().any(|n| n.contains("zero")),
        "the Int impl's `zero` must be monomorphized: {names:?}"
    );
    // Every call to a `zero` instance, wherever it was lowered, passes no args.
    let mut zero_calls = 0;
    for f in &mir.functions {
        for b in &f.blocks {
            for s in &b.stmts {
                if let nova_mir::Stmt::Call { callee, args, .. } = s {
                    if callee.contains("zero") {
                        zero_calls += 1;
                        assert!(
                            args.is_empty(),
                            "`{callee}` is receiver-less but was called with \
                             {} argument(s) from `{}`",
                            args.len(),
                            f.name
                        );
                    }
                }
            }
        }
    }
    assert_eq!(zero_calls, 1, "expected exactly one call to `zero`");
}

fn diagnostics_for(src: &str) -> Vec<String> {
    let file_id = FileId::DUMMY;
    let (tokens, _) = lex(src, file_id);
    let (ast, _) = parse(&tokens, file_id);
    let ast = ast.expect("no AST");
    let resolved = resolve(&ast);
    let checked = check(&resolved.file, &resolved.definitions);
    match lower_module(&checked.module) {
        Ok(_) => Vec::new(),
        Err(diags) => diags.into_iter().map(|d| d.code).collect(),
    }
}

#[test]
fn unsatisfied_trait_bound_reports_e0013() {
    // `label` requires `T: Show`, but `Q` has no `Show` impl.
    let codes = diagnostics_for(
        "record Q { v: Int }\n\
         trait Show { fn name(self) -> String }\n\
         fn label<T: Show>(x: T) -> String { \"x\" }\n\
         fn main() { println(label(Q { v: 1 })) }",
    );
    assert!(codes.contains(&"E0013".to_string()), "codes: {codes:?}");
}

/// `Map`'s key contract is `K: Hash + Eq`, and a record implementing neither
/// cannot be a key. The bound lives on the *inherent* `impl<K: Hash + Eq, V>
/// Map<K, V>` in `std/collections`, so this also pins that an impl-level bound
/// on an inherent impl is discharged at all.
///
/// This is the pipeline stage that enforces it: `nova-typeck`'s `check` reports
/// nothing for this program, because every trait bound in Nova is verified
/// during monomorphization (`12-TYPESYSTEM.md` §5.4). `diagnostics_for` runs
/// `check` + `lower_module`, which is exactly the pair `nova check` runs
/// (`nova_driver::check_file`), so the code asserted here is the code a user
/// actually sees.
#[test]
fn map_key_without_hash_reports_e0013() {
    let codes = diagnostics_for(
        "record Unhashable { v: Int }\n\
         fn main() {\n\
             let mut m = Map::new()\n\
             let k = Unhashable { v: 1 }\n\
             let p = m.insert(k, 1)\n\
             println(\"${m.len()}\")\n\
         }",
    );
    assert!(codes.contains(&"E0013".to_string()), "codes: {codes:?}");
}

#[test]
fn fn_as_value_lowers_to_closure_and_indirect_call() {
    let mir = mir_for(
        "fn double(n: Int) -> Int { n * 2 }\n\
         fn apply_twice<T>(f: fn(T) -> T, x: T) -> T { f(f(x)) }\n\
         fn main() { println(\"${apply_twice(double, 5)}\") }",
    );
    let names = function_names(&mir);
    assert!(
        names.iter().any(|n| n.starts_with("double.")),
        "names: {names:?}"
    );
    assert!(
        names
            .iter()
            .any(|n| n.starts_with("apply_twice.") && n.ends_with("$i")),
        "names: {names:?}"
    );
    // A bare fn used as a value becomes a fat-pointer wrapper (MakeClosure).
    let main = mir.functions.iter().find(|f| f.name == "main").unwrap();
    let has_make = main.blocks.iter().any(|b| {
        b.stmts
            .iter()
            .any(|s| matches!(s, nova_mir::Stmt::MakeClosure { .. }))
    });
    assert!(has_make, "bare-fn value should lower to MakeClosure");
    let apply = mir
        .functions
        .iter()
        .find(|f| f.name.starts_with("apply_twice.") && f.name.ends_with("$i"))
        .expect("instance exists");
    let has_indirect = apply.blocks.iter().any(|b| {
        b.stmts
            .iter()
            .any(|s| matches!(s, nova_mir::Stmt::CallIndirect { .. }))
    });
    assert!(has_indirect, "call through fn param should be indirect");
}

#[test]
fn arrays_lower_with_bounds_check() {
    let mir = mir_for(
        "fn main() {\n\
             let mut xs = [1, 2, 3]\n\
             xs[0] = xs[1]\n\
             println(\"${xs.len()} ${xs[2]}\")\n\
         }",
    );
    use nova_mir::{RtFunc, Stmt};
    let main = mir.functions.iter().find(|f| f.name == "main").unwrap();
    let stmts: Vec<&Stmt> = main.blocks.iter().flat_map(|b| b.stmts.iter()).collect();
    assert!(
        stmts.iter().any(|s| matches!(s, Stmt::MakeArray { .. })),
        "MakeArray"
    );
    assert!(
        stmts.iter().any(|s| matches!(s, Stmt::ArrayGet { .. })),
        "ArrayGet"
    );
    assert!(
        stmts.iter().any(|s| matches!(s, Stmt::ArraySet { .. })),
        "ArraySet"
    );
    assert!(
        stmts.iter().any(|s| matches!(s, Stmt::ArrayLen { .. })),
        "ArrayLen"
    );
    // Every index access is preceded by a bounds-check runtime call.
    assert!(
        stmts.iter().any(|s| matches!(
            s,
            Stmt::CallRuntime {
                func: RtFunc::CheckBounds,
                ..
            }
        )),
        "bounds check"
    );
}

#[test]
fn repeat_array_lowers_to_alloc_plus_fill_loop() {
    use nova_mir::{RtFunc, Stmt};
    let mir = mir_for("fn main() { let n = 3\n let a = [7; n]\n println(\"${a[0]}\") }");
    let main = mir
        .functions
        .iter()
        .find(|f| f.name == "main")
        .expect("main exists");
    let stmts: Vec<&Stmt> = main.blocks.iter().flat_map(|b| b.stmts.iter()).collect();
    assert!(
        stmts.iter().any(|s| matches!(s, Stmt::ArrayAlloc { .. })),
        "expected an ArrayAlloc"
    );
    assert!(
        stmts.iter().any(|s| matches!(s, Stmt::ArraySet { .. })),
        "expected the fill loop's ArraySet"
    );
    // The fill loop needs more than one block, unlike a static array literal.
    assert!(
        main.blocks.len() > 1,
        "expected a loop, got {} block(s)",
        main.blocks.len()
    );
    // A negative length is guarded, not clamped: the lowering emits a
    // comparison and a `panic` call it can branch to, so `[x; -1]` aborts at
    // the allocation rather than silently producing an empty array.
    assert!(
        stmts.iter().any(|s| matches!(
            s,
            Stmt::CallRuntime {
                func: RtFunc::Panic,
                ..
            }
        )),
        "expected the negative-length guard's panic call"
    );
}

#[test]
fn static_array_literal_still_lowers_without_a_loop() {
    // The repeat form's fill loop must not leak into the static literal path:
    // `[1, 2, 3]` stays a single `MakeArray` in one block.
    use nova_mir::Stmt;
    let mir = mir_for("fn main() { let a = [1, 2, 3]\n println(\"${a[0]}\") }");
    let main = mir
        .functions
        .iter()
        .find(|f| f.name == "main")
        .expect("main exists");
    let stmts: Vec<&Stmt> = main.blocks.iter().flat_map(|b| b.stmts.iter()).collect();
    assert!(
        stmts.iter().any(|s| matches!(s, Stmt::MakeArray { .. })),
        "expected a MakeArray"
    );
    assert!(
        !stmts.iter().any(|s| matches!(s, Stmt::ArrayAlloc { .. })),
        "a static literal should not use the runtime-length ArrayAlloc"
    );
    assert_eq!(main.blocks.len(), 1, "a static literal needs no branching");
}

#[test]
fn break_and_continue_lower_to_gotos() {
    // A while loop with break and continue lowers without panicking and the
    // body contains the extra control flow.
    let mir = mir_for(
        "fn main() {\n\
             let mut i = 0\n\
             while i < 10 {\n\
                 i = i + 1\n\
                 if i == 3 { continue }\n\
                 if i == 7 { break }\n\
             }\n\
             println(\"${i}\")\n\
         }",
    );
    let main = mir.functions.iter().find(|f| f.name == "main").unwrap();
    // A loop produces multiple Goto/Branch terminators; just assert the
    // function has several blocks (header, body, branches, exit, dead).
    assert!(
        main.blocks.len() >= 6,
        "expected several blocks for break/continue, got {}",
        main.blocks.len()
    );
}

#[test]
fn closure_lowers_to_env_taking_function() {
    let mir = mir_for(
        "fn main() {\n\
             let base = 10\n\
             let f = |n| n + base\n\
             println(\"${f(5)}\")\n\
         }",
    );
    // The lifted closure function takes an env and captures one value.
    let closure = mir
        .functions
        .iter()
        .find(|f| f.takes_env && f.capture_count == 1)
        .expect("a closure with one capture was lifted");
    // Its entry loads the captured value from the environment.
    let loads_capture = closure.blocks.iter().any(|b| {
        b.stmts
            .iter()
            .any(|s| matches!(s, nova_mir::Stmt::RecordField { .. }))
    });
    assert!(loads_capture, "closure should load its capture from env");
}

// === Supertraits (`trait B: A`) at monomorphization ===

#[test]
fn supertrait_derived_bound_is_discharged() {
    // A bound `T: B` expands to `[B, A]` in typeck, so monomorphizing `sum` at
    // `T = R` makes `impl_satisfies` check `A` as well. Typeck's E0072 guarantees
    // the `impl A for R` that discharges it, and both dispatched methods must
    // reach monomorphized instances.
    let mir = mir_for(
        "trait A { fn a(self) -> Int }\n\
         trait B: A { fn b(self) -> Int }\n\
         record R { v: Int }\n\
         impl A for R { fn a(self) -> Int { 1 } }\n\
         impl B for R { fn b(self) -> Int { 2 } }\n\
         fn sum<T: B>(x: T) -> Int { x.a() + x.b() }\n\
         fn main() { let r = R { v: 0 }\n println(\"${sum(r)}\") }",
    );
    let names = function_names(&mir);
    assert!(
        names.iter().any(|n| n.contains("A.a")),
        "the supertrait method must be monomorphized: {names:?}"
    );
    assert!(
        names.iter().any(|n| n.contains("B.b")),
        "the subtrait method must be monomorphized: {names:?}"
    );
}

#[test]
fn conditional_impl_discharges_derived_supertrait_bound() {
    // The case that decides whether `impl_satisfies` needs to know about
    // supertraits: every impl here is *conditional* (`impl<T: B> … for W<T>`), so
    // discharging `W<R>: A` recurses into the impl's own bounds — which are
    // themselves supertrait-expanded to `[B, A]`. If either the outer or the
    // recursive step could not discharge a supertrait-derived bound, this would
    // fail with E0013 instead of lowering.
    let mir = mir_for(
        "trait A { fn a(self) -> Int }\n\
         trait B: A { fn b(self) -> Int }\n\
         record R { v: Int }\n\
         record W<T> { inner: T }\n\
         impl A for R { fn a(self) -> Int { 1 } }\n\
         impl B for R { fn b(self) -> Int { 2 } }\n\
         impl<T: B> A for W<T> { fn a(self) -> Int { self.inner.a() } }\n\
         impl<T: B> B for W<T> { fn b(self) -> Int { self.inner.b() } }\n\
         fn sum<U: B>(x: U) -> Int { x.a() + x.b() }\n\
         fn main() {\n\
             let w = W { inner: R { v: 0 } }\n\
             println(\"${sum(w)}\")\n\
         }",
    );
    let names = function_names(&mir);
    assert!(
        names.iter().filter(|n| n.contains("A.a")).count() >= 2,
        "`a` should be monomorphized for both `W<R>` and `R`: {names:?}"
    );
}

#[test]
fn supertrait_impl_with_a_narrower_bound_reports_e0013() {
    // Typeck's supertrait check matches self types structurally, so
    // `impl<T: Show> A for W<T>` counts as covering `impl<T> B for W<T>` even
    // though it is conditional on a bound the subtrait impl does not require.
    // The gap is closed at monomorphization rather than left to miscompile:
    // `W<Bool>` satisfies `B` but not `A`, which is E0013 (a diagnostic, not an
    // ICE and not silent acceptance).
    let codes = diagnostics_for(
        "trait Show { fn show(self) -> String }\n\
         trait A { fn a(self) -> Int }\n\
         trait B: A { fn b(self) -> Int }\n\
         record W<T> { inner: T }\n\
         impl Show for Int { fn show(self) -> String { \"i\" } }\n\
         impl<T: Show> A for W<T> { fn a(self) -> Int { 1 } }\n\
         impl<T> B for W<T> { fn b(self) -> Int { 2 } }\n\
         fn sum<U: B>(x: U) -> Int { x.b() }\n\
         fn main() {\n\
             let w = W { inner: true }\n\
             println(\"${sum(w)}\")\n\
         }",
    );
    assert!(codes.contains(&"E0013".to_string()), "codes: {codes:?}");
}

#[test]
fn supertrait_default_body_dispatches_at_monomorphization() {
    // `B`'s default body calls `self.a()` through the supertrait-expanded `Self`
    // bound; monomorphizing it at `Self = R` must resolve that call to `impl A
    // for R`'s method rather than fail to find an impl.
    let mir = mir_for(
        "trait A { fn a(self) -> Int }\n\
         trait B: A { fn b(self) -> Int { self.a() + 1 } }\n\
         record R { v: Int }\n\
         impl A for R { fn a(self) -> Int { 1 } }\n\
         impl B for R { }\n\
         fn main() { let r = R { v: 0 }\n println(\"${r.b()}\") }",
    );
    let names = function_names(&mir);
    assert!(
        names.iter().any(|n| n.contains("A.a")),
        "the supertrait method called from the default body must be \
         monomorphized: {names:?}"
    );
}

/// The two std-only builtins behind `Hash` lower differently on purpose:
/// `str_hash` becomes a runtime call, `char_to_int` becomes no call at all —
/// `Char` and `Int` are both `MirTy::I64`, so the conversion is a register
/// move (`lower_call`'s `None` arm). Pinned here because nothing else
/// distinguishes "lowered to a move" from "lowered to a call that the linker
/// happens to resolve": the end-to-end fixture would pass either way, and the
/// difference is a permanent runtime ABI symbol.
#[test]
fn hash_builtins_lower_to_a_runtime_call_and_a_move() {
    use nova_mir::{RtFunc, Stmt};
    let mir = mir_for("fn main() { println(\"${('a').hash()} ${(\"s\").hash()}\") }");
    let find = |prefix: &str| {
        mir.functions
            .iter()
            .find(|f| f.name.starts_with(prefix))
            .unwrap_or_else(|| panic!("no `{prefix}*`: {:?}", function_names(&mir)))
    };
    let rt_calls = |f: &nova_mir::Function| -> Vec<RtFunc> {
        f.blocks
            .iter()
            .flat_map(|b| &b.stmts)
            .filter_map(|s| match s {
                Stmt::CallRuntime { func, .. } => Some(*func),
                _ => None,
            })
            .collect()
    };
    assert_eq!(
        rt_calls(find("String.Hash.hash")),
        vec![RtFunc::StrHash],
        "`impl Hash for String` reaches the runtime exactly once"
    );
    let char_hash = find("Char.Hash.hash");
    assert!(
        rt_calls(char_hash).is_empty(),
        "`impl Hash for Char` must reach no runtime function: {:?}",
        rt_calls(char_hash)
    );
    assert!(
        char_hash
            .blocks
            .iter()
            .flat_map(|b| &b.stmts)
            .any(|s| matches!(s, Stmt::Copy { .. })),
        "`char_to_int` lowers to a `Copy`"
    );
}
