//! Tuples in the parser (spec
//! `docs/superpowers/specs/2026-10-26-phase-5-0-the-language-and-std-a-compiler-needs-design.md`
//! §4.1, §4.2): tuple types and literals as the parser already read them,
//! and `t.N`, a field whose name is its index. Each test renders a
//! function's body, or its signature's types, in a compact form that leaves
//! the spans out.

use nova_ast::expr::{AssignOp, BinOp, Expr, Literal};
use nova_ast::ty::Type;
use nova_ast::{Block, Item, Stmt};
use nova_diagnostics::FileDb;
use nova_lexer::lex;
use nova_parser::{parse, ParseError};

fn parse_src(src: &str) -> (nova_ast::File, Vec<ParseError>) {
    let mut db = FileDb::new();
    let id = db.add("tuples.nova", src);
    let (tokens, lex_errors) = lex(src, id);
    assert!(lex_errors.is_empty(), "{lex_errors:?}");
    let (file, errors) = parse(&tokens, id);
    (file.expect("a file"), errors)
}

fn function(file: &nova_ast::File) -> &nova_ast::item::Function {
    file.items
        .iter()
        .find_map(|i| match &i.value {
            Item::Function(f) => Some(f),
            _ => None,
        })
        .expect("a function")
}

/// The statements and tail of the body of `fn f() { <body> }`, one string
/// each; a tail is marked `tail:`. Panics on a parse error.
fn body(body: &str) -> Vec<String> {
    let src = format!("fn f() {{\n{body}\n}}\n");
    let (file, errors) = parse_src(&src);
    assert!(errors.is_empty(), "{src}\n{errors:?}");
    block(&function(&file).body.value)
}

/// The parse errors of `fn f() { <body> }`, and its tail.
fn errors_of(body: &str) -> (Vec<String>, Vec<String>) {
    let src = format!("fn f() {{\n{body}\n}}\n");
    let (file, errors) = parse_src(&src);
    (
        errors.iter().map(|e| e.to_string()).collect(),
        block(&function(&file).body.value),
    )
}

/// The parameter types and the return type of `fn f<sig>`.
fn sig(sig: &str) -> Vec<String> {
    let src = format!("fn f{sig} {{}}\n");
    let (file, errors) = parse_src(&src);
    assert!(errors.is_empty(), "{src}\n{errors:?}");
    let f = function(&file);
    let mut out: Vec<String> = f.params.iter().map(|p| ty(&p.ty.value)).collect();
    if let Some(r) = &f.return_ty {
        out.push(format!("-> {}", ty(&r.value)));
    }
    out
}

fn ty(t: &Type) -> String {
    match t {
        Type::Path { path, args } => {
            let name = path
                .segments
                .iter()
                .map(|s| s.value.as_str())
                .collect::<Vec<_>>()
                .join("::");
            if args.is_empty() {
                name
            } else {
                let args: Vec<String> = args.iter().map(|a| ty(&a.value)).collect();
                format!("{name}<{}>", args.join(", "))
            }
        }
        Type::Tuple(items) => {
            let items: Vec<String> = items.iter().map(|i| ty(&i.value)).collect();
            format!("tuple({})", items.join(", "))
        }
        Type::Array(e) => format!("[{}]", ty(&e.value)),
        other => format!("{other:?}"),
    }
}

fn block(b: &Block) -> Vec<String> {
    let mut out: Vec<String> = b
        .stmts
        .iter()
        .map(|s| match &s.value {
            Stmt::Let { init, .. } => format!(
                "let = {}",
                init.as_ref().map_or("-".to_string(), |e| show(&e.value))
            ),
            Stmt::Expr(e) => show(&e.value),
            Stmt::Item(_) => "item".to_string(),
        })
        .collect();
    if let Some(t) = &b.trailing {
        out.push(format!("tail:{}", show(&t.value)));
    }
    out
}

fn list(xs: &[nova_diagnostics::Spanned<Expr>]) -> String {
    xs.iter()
        .map(|x| show(&x.value))
        .collect::<Vec<_>>()
        .join(", ")
}

fn show(e: &Expr) -> String {
    match e {
        Expr::Lit(Literal::Int(n)) => n.to_string(),
        Expr::Lit(Literal::Float(f)) => format!("{f:?}"),
        Expr::Path(p) => p
            .segments
            .iter()
            .map(|s| s.value.as_str())
            .collect::<Vec<_>>()
            .join("::"),
        Expr::Tuple(xs) => format!("tuple({})", list(xs)),
        Expr::Field { target, field } => format!("{}.<{}>", show(&target.value), field.value),
        Expr::Call { callee, args } => format!("{}({})", show(&callee.value), list(args)),
        Expr::Binary { op, lhs, rhs } => {
            let op = match op {
                BinOp::Add => "+",
                BinOp::Sub => "-",
                BinOp::Mul => "*",
                _ => "op",
            };
            format!("[{} {op} {}]", show(&lhs.value), show(&rhs.value))
        }
        Expr::Assign { op, lhs, rhs } => {
            let op = match op {
                AssignOp::Assign => "=",
                AssignOp::AddAssign => "+=",
                _ => "op=",
            };
            format!("{} {op} {}", show(&lhs.value), show(&rhs.value))
        }
        Expr::Await(x) => format!("{}.await", show(&x.value)),
        other => format!("{other:?}"),
    }
}

#[test]
fn a_tuple_literal_has_two_or_more_elements_or_a_trailing_comma() {
    assert_eq!(body("(1, 2)"), ["tail:tuple(1, 2)"]);
    assert_eq!(body("(1, 2,)"), ["tail:tuple(1, 2)"]);
    assert_eq!(body("(5,)"), ["tail:tuple(5)"]);
    assert_eq!(
        body("((1, 2), (3,))"),
        ["tail:tuple(tuple(1, 2), tuple(3))"]
    );
}

#[test]
fn unit_and_a_parenthesised_expression_stay_what_they_are() {
    assert_eq!(body("()"), ["tail:tuple()"]);
    assert_eq!(body("(5)"), ["tail:5"]);
    assert_eq!(body("((5))"), ["tail:5"]);
}

#[test]
fn tuple_types_parse_with_unit_and_a_parenthesised_type_unchanged() {
    assert_eq!(
        sig("(a: (Int, String), b: (Int,), c: (Int), d: ()) -> (Bool, (Int, Int))"),
        [
            "tuple(Int, String)",
            "tuple(Int)",
            "Int",
            "tuple()",
            "-> tuple(Bool, tuple(Int, Int))"
        ]
    );
    assert_eq!(
        sig("(v: Vec<(Int, String)>, o: Option<(Int,)>, a: [(Int, Int)])"),
        [
            "Vec<tuple(Int, String)>",
            "Option<tuple(Int)>",
            "[tuple(Int, Int)]"
        ]
    );
}

#[test]
fn a_tuple_field_is_named_by_its_index() {
    assert_eq!(body("t.0"), ["tail:t.<0>"]);
    assert_eq!(body("t.12"), ["tail:t.<12>"]);
    assert_eq!(body("f(x).1"), ["tail:f(x).<1>"]);
}

#[test]
fn nested_tuple_fields_read_left_to_right() {
    // The lexer splits `1.0`, `0.1` and `1.10` after a `.`.
    assert_eq!(body("t.1.0"), ["tail:t.<1>.<0>"]);
    assert_eq!(body("t.0.1.2"), ["tail:t.<0>.<1>.<2>"]);
    assert_eq!(body("t.1.10"), ["tail:t.<1>.<10>"]);
    assert_eq!(body("t.0.1.2.3.4"), ["tail:t.<0>.<1>.<2>.<3>.<4>"]);
}

#[test]
fn a_tuple_field_takes_the_postfix_forms_after_it() {
    assert_eq!(body("t.0.len()"), ["tail:t.<0>.<len>()"]);
    assert_eq!(body("t.0.await"), ["tail:t.<0>.await"]);
    assert_eq!(body("t.0.x.1"), ["tail:t.<0>.<x>.<1>"]);
    assert_eq!(body("t.0 + t.1 * 2"), ["tail:[t.<0> + [t.<1> * 2]]"]);
}

#[test]
fn a_tuple_field_is_a_place() {
    assert_eq!(body("t.0 = 1"), ["tail:t.<0> = 1"]);
    assert_eq!(body("t.1.0 += 2"), ["tail:t.<1>.<0> += 2"]);
}

#[test]
fn a_float_elsewhere_stays_a_float() {
    assert_eq!(body("t.0 + 1.5"), ["tail:[t.<0> + 1.5]"]);
}

#[test]
fn an_index_that_is_not_plain_decimal_digits_is_refused() {
    for (src, written) in [
        ("t.01", "1"),
        ("t.1_0", "10"),
        ("t.0x1", "1"),
        ("t.00", "0"),
        ("t.0b1", "1"),
    ] {
        let (errors, tail) = errors_of(src);
        assert_eq!(
            errors,
            [format!(
                "a tuple index is plain decimal digits, with no leading zero, `_` or prefix: \
                 write `.{written}`"
            )],
            "{src}"
        );
        // The access is kept, so the checker and the editor still see it.
        assert_eq!(tail, [format!("tail:t.<{written}>")], "{src}");
    }
}

#[test]
fn an_index_with_an_exponent_is_a_float_and_refused() {
    let (errors, _) = errors_of("t.1e5");
    assert_eq!(
        errors,
        ["expected identifier (in field access), found float literal"]
    );
}

#[test]
fn the_newline_rule_is_off_inside_a_tuple_literal() {
    // A tuple literal is parenthesised, so a `(` or `-` at a line start
    // inside it continues the line before (5.0 spec §3.5).
    assert_eq!(
        body("let t = (\n    f\n    (1),\n    a\n    - b,\n)\nt"),
        ["let = tuple(f(1), [a - b])", "tail:t"]
    );
}

#[test]
fn a_tuple_on_the_line_after_a_statement_begins_a_new_one() {
    assert_eq!(
        body("let x = g()\n(x, x)"),
        ["let = g()", "tail:tuple(x, x)"]
    );
    assert_eq!(body("g();\n(1, 2)"), ["g()", "tail:tuple(1, 2)"]);
}
