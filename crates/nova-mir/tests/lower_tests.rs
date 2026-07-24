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
