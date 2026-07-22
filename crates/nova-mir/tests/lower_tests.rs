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
    let checked = check(&ast, &resolved.definitions);
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
    assert!(names.contains(&"fib"));
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
    assert!(names.contains(&"identity$i"), "names: {names:?}");
    assert!(names.contains(&"identity$s"), "names: {names:?}");
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
        .find(|f| f.name == "area")
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
fn repeated_param_trait_impl_mismatch_reports_e0013() {
    // A trait impl on `Pair<T, T>` must not dispatch for `Pair<Int, String>` —
    // structural matching, not just head, gates selection.
    let codes = diagnostics_for(
        "record Pair<A, B> { first: A, second: B }\n\
         trait Same { fn same(self) -> Int }\n\
         impl<T> Same for Pair<T, T> { fn same(self) -> Int { 1 } }\n\
         fn main() { let p = Pair { first: 1, second: \"x\" }\n println(\"${p.same()}\") }",
    );
    assert!(codes.contains(&"E0013".to_string()), "codes: {codes:?}");
}

fn diagnostics_for(src: &str) -> Vec<String> {
    let file_id = FileId::DUMMY;
    let (tokens, _) = lex(src, file_id);
    let (ast, _) = parse(&tokens, file_id);
    let ast = ast.expect("no AST");
    let resolved = resolve(&ast);
    let checked = check(&ast, &resolved.definitions);
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
    assert!(names.contains(&"double"), "names: {names:?}");
    assert!(names.contains(&"apply_twice$i"), "names: {names:?}");
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
        .find(|f| f.name == "apply_twice$i")
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
