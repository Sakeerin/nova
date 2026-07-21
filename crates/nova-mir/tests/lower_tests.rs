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
fn fn_as_value_lowers_to_fnaddr_and_indirect_call() {
    let mir = mir_for(
        "fn double(n: Int) -> Int { n * 2 }\n\
         fn apply_twice<T>(f: fn(T) -> T, x: T) -> T { f(f(x)) }\n\
         fn main() { println(\"${apply_twice(double, 5)}\") }",
    );
    let names = function_names(&mir);
    assert!(names.contains(&"double"), "names: {names:?}");
    assert!(names.contains(&"apply_twice$i"), "names: {names:?}");
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
