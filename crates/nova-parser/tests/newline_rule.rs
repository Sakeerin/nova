//! The newline rule (spec
//! `docs/superpowers/specs/2026-10-26-phase-5-0-the-language-and-std-a-compiler-needs-design.md`
//! §3.5): in statement context, a line break ends the expression before a
//! `(`, a `[` or a binary `-`; a `return` or `break` at the end of a line
//! takes no value. Each test renders a function body's statements in a
//! compact form that leaves the spans out.

use nova_ast::expr::{BinOp, Expr, Literal, StringPart, UnOp};
use nova_ast::pattern::Pattern;
use nova_ast::{Block, Item, SplitKind, Stmt};
use nova_diagnostics::FileDb;
use nova_lexer::lex;
use nova_parser::parse;

/// The statements and tail of `f`'s body, in `src`, one string each; a
/// tail is marked `tail:`. Panics on a lex or parse error.
fn body(src: &str) -> Vec<String> {
    let mut db = FileDb::new();
    let id = db.add("newline.nova", src);
    let (tokens, lex_errors) = lex(src, id);
    assert!(lex_errors.is_empty(), "{lex_errors:?}");
    let (file, errors) = parse(&tokens, id);
    assert!(errors.is_empty(), "{src}\n{errors:?}");
    let file = file.expect("a file");
    let f = file
        .items
        .iter()
        .find_map(|i| match &i.value {
            Item::Function(f) => Some(f),
            _ => None,
        })
        .expect("a function");
    block(&f.body.value)
}

fn block(b: &Block) -> Vec<String> {
    let mut out: Vec<String> = b
        .stmts
        .iter()
        .map(|s| match &s.value {
            Stmt::Let { pattern, init, .. } => format!(
                "let {} = {}",
                pat(&pattern.value),
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

fn pat(p: &Pattern) -> String {
    match p {
        Pattern::Wildcard => "_".to_string(),
        Pattern::Ident { name, .. } => name.value.clone(),
        Pattern::Tuple(ps) => format!(
            "({})",
            ps.iter()
                .map(|p| pat(&p.value))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Pattern::Lit(Literal::Int(n)) => n.to_string(),
        Pattern::Array(ps) => format!(
            "[{}]",
            ps.iter()
                .map(|p| pat(&p.value))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Pattern::TupleStruct { path, fields } => format!(
            "{}({})",
            path.segments
                .iter()
                .map(|s| s.value.as_str())
                .collect::<Vec<_>>()
                .join("::"),
            fields
                .iter()
                .map(|p| pat(&p.value))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        other => format!("{other:?}"),
    }
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
        Expr::Lit(Literal::Bool(b)) => b.to_string(),
        Expr::Path(p) => p
            .segments
            .iter()
            .map(|s| s.value.as_str())
            .collect::<Vec<_>>()
            .join("::"),
        Expr::Tuple(xs) => format!("({})", list(xs)),
        Expr::Array(xs) => format!("[{}]", list(xs)),
        Expr::Block(b) => format!("{{{}}}", block(b).join("; ")),
        Expr::If { cond, then, else_ } => format!(
            "if {} {{{}}}{}",
            show(&cond.value),
            block(&then.value).join("; "),
            else_
                .as_ref()
                .map_or(String::new(), |e| format!(" else {}", show(&e.value)))
        ),
        Expr::Match { scrutinee, arms } => format!(
            "match {} {{{}}}",
            show(&scrutinee.value),
            arms.iter()
                .map(|a| format!(
                    "{}{} => {}",
                    pat(&a.pattern.value),
                    a.guard
                        .as_ref()
                        .map_or(String::new(), |g| format!(" if {}", show(&g.value))),
                    show(&a.body.value)
                ))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Expr::While { cond, body } => format!(
            "while {} {{{}}}",
            show(&cond.value),
            block(&body.value).join("; ")
        ),
        Expr::Return(v) => match v {
            Some(v) => format!("return {}", show(&v.value)),
            None => "return".to_string(),
        },
        Expr::Break(v) => match v {
            Some(v) => format!("break {}", show(&v.value)),
            None => "break".to_string(),
        },
        Expr::Record { path, fields, .. } => format!(
            "{} {{{}}}",
            path.segments
                .iter()
                .map(|s| s.value.as_str())
                .collect::<Vec<_>>()
                .join("::"),
            fields
                .iter()
                .map(|f| format!(
                    "{}: {}",
                    f.name.value,
                    f.value.as_ref().map_or("-".to_string(), |v| show(&v.value))
                ))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Expr::Binary { op, lhs, rhs } => {
            let o = match op {
                BinOp::Add => "+",
                BinOp::Sub => "-",
                BinOp::Mul => "*",
                BinOp::Or => "||",
                BinOp::Lt => "<",
                _ => "op",
            };
            format!("({} {o} {})", show(&lhs.value), show(&rhs.value))
        }
        Expr::Unary { op, expr } => {
            let o = match op {
                UnOp::Neg => "-",
                UnOp::Not => "!",
                _ => "u",
            };
            format!("{o}{}", show(&expr.value))
        }
        Expr::Call { callee, args } => format!("{}({})", show(&callee.value), list(args)),
        Expr::Index { target, index } => {
            format!("{}[{}]", show(&target.value), show(&index.value))
        }
        Expr::Field { target, field } => format!("{}.{}", show(&target.value), field.value),
        Expr::Closure { body, .. } => format!("|..| {}", show(&body.value)),
        Expr::Assign { lhs, rhs, .. } => format!("{} = {}", show(&lhs.value), show(&rhs.value)),
        Expr::Try(x) => format!("{}?", show(&x.value)),
        Expr::StringInterp(parts) => format!(
            "\"{}\"",
            parts
                .iter()
                .map(|p| match p {
                    StringPart::Lit(s) => s.clone(),
                    StringPart::Expr(e) => format!("${{{}}}", show(&e.value)),
                })
                .collect::<String>()
        ),
        other => format!("{other:?}"),
    }
}

fn f(stmts: &str) -> String {
    format!("fn f() {{\n{stmts}\n}}\n")
}

/// The errors of `f`'s body `stmts`.
fn errors(stmts: &str) -> Vec<nova_parser::ParseError> {
    let src = f(stmts);
    let mut db = FileDb::new();
    let id = db.add("errors.nova", src.as_str());
    let (tokens, _) = lex(&src, id);
    parse(&tokens, id).1
}

// --- `(`, `[` and `-` at a line start, in statement context ---

#[test]
fn a_parenthesis_at_a_line_start_begins_a_statement() {
    assert_eq!(body(&f("g\n(x)")), ["g", "tail:x"]);
    assert_eq!(body(&f("g\n(x, y)")), ["g", "tail:(x, y)"]);
    // On the same line, it is a call, as before.
    assert_eq!(body(&f("g(x)")), ["tail:g(x)"]);
}

#[test]
fn a_bracket_at_a_line_start_begins_a_statement() {
    assert_eq!(body(&f("g\n[1, 2]")), ["g", "tail:[1, 2]"]);
    assert_eq!(body(&f("a[0]")), ["tail:a[0]"]);
}

#[test]
fn a_minus_at_a_line_start_begins_a_statement() {
    assert_eq!(body(&f("let z = x\n-y")), ["let z = x", "tail:-y"]);
    assert_eq!(body(&f("let z = x - y")), ["let z = (x - y)"]);
}

#[test]
fn other_operators_and_dots_continue_the_line_before() {
    assert_eq!(body(&f("let z = x\n+ y")), ["let z = (x + y)"]);
    assert_eq!(body(&f("let z = x\n* y")), ["let z = (x * y)"]);
    assert_eq!(body(&f("let z = x\n|| y")), ["let z = (x || y)"]);
    assert_eq!(
        body(&f("let s = parts\n    .iter()\n    .count()")),
        ["let s = parts.iter().count()"]
    );
    assert_eq!(body(&f("let v = x\n?")), ["let v = x?"]);
}

#[test]
fn a_minus_at_a_line_end_continues() {
    assert_eq!(body(&f("let z = x -\n    y")), ["let z = (x - y)"]);
    assert_eq!(body(&f("let z = x -\n    -y")), ["let z = (x - -y)"]);
}

#[test]
fn the_rule_holds_in_a_let_value_and_a_tail() {
    assert_eq!(body(&f("let a = g\n(h)")), ["let a = g", "tail:h"]);
    assert_eq!(body(&f("g\n-1")), ["g", "tail:-1"]);
}

#[test]
fn the_rule_holds_in_a_match_arm() {
    assert_eq!(
        body(&f("match x {\n    A => g\n    (a) => h\n}")),
        ["tail:match x {A => g, a => h}"]
    );
    assert_eq!(
        body(&f("match x {\n    A => g\n    [a] => h\n}")),
        ["tail:match x {A => g, [a] => h}"]
    );
}

#[test]
fn a_tuple_pattern_arm_after_an_arm_body_begins_an_arm() {
    assert_eq!(
        body(&f("match p {\n    (0, b) => b\n    (a, b) => a\n}")),
        ["tail:match p {(0, b) => b, (a, b) => a}"]
    );
}

// --- where the rule is off ---

#[test]
fn the_rule_is_off_inside_parentheses() {
    assert_eq!(body(&f("let z = (x\n-y)")), ["let z = (x - y)"]);
    assert_eq!(body(&f("let z = (g\n(1))")), ["let z = g(1)"]);
    assert_eq!(body(&f("let t = (g\n(1), 2)")), ["let t = (g(1), 2)"]);
}

#[test]
fn the_rule_is_off_inside_brackets() {
    assert_eq!(
        body(&f("let a = [x\n-y, g\n(1)]")),
        ["let a = [(x - y), g(1)]"]
    );
    assert_eq!(body(&f("let a = b[i\n-1]")), ["let a = b[(i - 1)]"]);
}

#[test]
fn the_rule_is_off_inside_arguments() {
    assert_eq!(body(&f("h(x\n-y, g\n[0])")), ["tail:h((x - y), g[0])"]);
}

#[test]
fn the_rule_is_off_inside_a_record_literal() {
    assert_eq!(
        body(&f("let r = R { a: x\n-y, b: g\n(1) }")),
        ["let r = R {a: (x - y), b: g(1)}"]
    );
}

#[test]
fn the_rule_is_off_in_an_if_head() {
    assert_eq!(
        body(&f("if x\n-y < 0 { g() }")),
        ["tail:if ((x - y) < 0) {tail:g()}"]
    );
    assert_eq!(
        body(&f("while g\n(1) { h() }")),
        ["tail:while g(1) {tail:h()}"]
    );
    assert_eq!(
        body(&f("match g\n(1) { _ => 0 }")),
        ["tail:match g(1) {_ => 0}"]
    );
}

#[test]
fn the_rule_is_off_in_a_guard() {
    assert_eq!(
        body(&f("match x {\n    a if a\n    -1 < 0 => a\n    _ => 0\n}")),
        ["tail:match x {a if ((a - 1) < 0) => a, _ => 0}"]
    );
}

#[test]
fn a_block_inside_parentheses_turns_the_rule_back_on() {
    assert_eq!(
        body(&f("let v = g({\n    h\n    (1)\n})")),
        ["let v = g({h; tail:1})"]
    );
}

#[test]
fn a_match_inside_parentheses_turns_the_rule_back_on_in_its_arms() {
    // An arm's body is statement context wherever the `match` is: a line
    // starting with `(` or `[` after a body begins the next arm.
    assert_eq!(
        body(&f("let v = (match x {\n    A => g\n    (a) => h\n})")),
        ["let v = match x {A => g, a => h}"]
    );
    assert_eq!(
        body(&f("k(match x {\n    A => g\n    [a] => h\n})")),
        ["tail:k(match x {A => g, [a] => h})"]
    );
}

#[test]
fn a_brace_on_a_new_line_keeps_its_meaning() {
    assert_eq!(body(&f("let r = R\n{ a: 1 }")), ["let r = R {a: 1}"]);
}

// --- after the `}` of `if`, `match` and `while` ---

#[test]
fn a_parenthesis_after_an_if_begins_a_statement() {
    assert_eq!(
        body(&f("let w = if c { 1 } else { 2 }\n(a, b)")),
        ["let w = if c {tail:1} else {tail:2}", "tail:(a, b)"]
    );
}

#[test]
fn a_parenthesis_after_a_match_begins_a_statement() {
    assert_eq!(
        body(&f("match x { _ => g }\n(a)")),
        ["match x {_ => g}", "tail:a"]
    );
}

#[test]
fn a_bracket_after_a_while_begins_a_statement() {
    assert_eq!(
        body(&f("while c { g() }\n[1]")),
        ["while c {tail:g()}", "tail:[1]"]
    );
}

// --- `return` and `break` ---

#[test]
fn return_at_a_line_end_takes_no_value() {
    assert_eq!(body(&f("return\ncleanup()")), ["return", "tail:cleanup()"]);
    assert_eq!(body(&f("return x")), ["tail:return x"]);
    assert_eq!(body(&f("return g(\n    1)")), ["tail:return g(1)"]);
}

#[test]
fn break_at_a_line_end_takes_no_value() {
    assert_eq!(
        body(&f("while c {\n    break\n    g()\n}")),
        ["tail:while c {break; tail:g()}"]
    );
    assert_eq!(
        body(&f("while c { break x }")),
        ["tail:while c {tail:break x}"]
    );
}

#[test]
fn return_at_a_line_end_takes_no_value_inside_parentheses_too() {
    // `g(return` then `x)`: the `return` ends at the line break, so `x`
    // cannot follow it in the arguments.
    assert!(!errors("g(return\n    x)").is_empty());
}

#[test]
fn a_bare_return_in_a_match_arm_ends_the_arm() {
    assert_eq!(
        body(&f("match x {\n    A => return\n    B => 1\n}")),
        ["tail:match x {A => return, B => 1}"]
    );
}

// --- comments ---

#[test]
fn a_line_comment_is_a_line_break() {
    assert_eq!(body(&f("g // note\n(x)")), ["g", "tail:x"]);
}

#[test]
fn a_block_comment_that_holds_a_line_break_is_one() {
    assert_eq!(body(&f("g /* a\n b */ (x)")), ["g", "tail:x"]);
    // One that holds none is not.
    assert_eq!(body(&f("g /* a */ (x)")), ["tail:g(x)"]);
}

#[test]
fn a_semicolon_still_ends_a_statement() {
    assert_eq!(body(&f("g; (x)")), ["g", "tail:x"]);
    assert_eq!(body(&f("let z = x; -y")), ["let z = x", "tail:-y"]);
}

#[test]
fn a_view_keeps_its_rule() {
    // In a view's children, a line break is not significant: a `;` ends a
    // child before a `(`, as before the rule.
    let src = "fn f() -> Html { view { g\n(x) } }\n";
    let mut db = FileDb::new();
    let id = db.add("v.nova", src);
    let (tokens, _) = lex(src, id);
    let (file, errors) = parse(&tokens, id);
    assert!(errors.is_empty(), "{errors:?}");
    let file = file.expect("a file");
    let Item::Function(func) = &file.items[0].value else {
        panic!("a function")
    };
    let Some(tail) = &func.body.value.trailing else {
        panic!("a tail")
    };
    let Expr::View(children) = &tail.value else {
        panic!("a view")
    };
    assert_eq!(children.len(), 1);
}

#[test]
fn the_rule_is_off_in_an_interpolation_hole() {
    assert_eq!(
        body(&f("let s = \"${a\n(b)} and ${c\n-d}\"")),
        ["let s = \"${a(b)} and ${(c - d)}\""]
    );
}

#[test]
fn the_rule_is_off_in_a_nested_items_signature_and_value() {
    // A nested `const`'s value continues across a line break, as a
    // top-level one does; its body-less item is not statement context.
    assert_eq!(body(&f("const C: Int = a\n-1\nC")), ["item", "tail:C"]);
}

// --- the splits the parser records for E0103 (the user's ruling of 2026-10-27) ---

/// Each split of `f`'s body `stmts`: the text of the token the line break
/// cut off, and what it would have been.
fn splits(stmts: &str) -> Vec<(String, SplitKind)> {
    let src = f(stmts);
    let mut db = FileDb::new();
    let id = db.add("splits.nova", src.as_str());
    let (tokens, _) = lex(&src, id);
    let (file, errors) = parse(&tokens, id);
    assert!(errors.is_empty(), "{src}\n{errors:?}");
    file.expect("a file")
        .newline_splits
        .iter()
        .map(|s| {
            (
                src[s.at.start as usize..s.at.end as usize].to_string(),
                s.kind,
            )
        })
        .collect()
}

#[test]
fn each_split_the_rule_makes_is_recorded() {
    assert_eq!(splits("g\n(1)"), [("(".to_string(), SplitKind::Call)]);
    assert_eq!(splits("g\n[1]"), [("[".to_string(), SplitKind::Index)]);
    assert_eq!(
        splits("let z = x\n-y"),
        [("-".to_string(), SplitKind::Minus)]
    );
    assert_eq!(
        splits("return a\n- b"),
        [("-".to_string(), SplitKind::Minus)]
    );
    assert_eq!(splits("return\ng()"), [("g".to_string(), SplitKind::Value)]);
    assert_eq!(
        splits("while true {\n    break\n    x\n}"),
        [("x".to_string(), SplitKind::Value)]
    );
}

#[test]
fn a_written_semicolon_records_no_split() {
    assert!(splits("g;\n(1)").is_empty());
    assert!(splits("let z = x;\n-y").is_empty());
    assert!(splits("return;\ng()").is_empty());
}

#[test]
fn what_the_old_parser_could_not_have_continued_records_no_split() {
    // The next arm after an arm's body, and a `let` or an item after a bare
    // `return`, were never part of the statement before.
    assert!(splits("match x {\n    A => g\n    (a) => h\n}").is_empty());
    assert!(splits("match x {\n    A => return\n    B => h\n}").is_empty());
    assert!(splits("return\nlet y = 1").is_empty());
    assert!(splits("return\nfn g() {}").is_empty());
    // A line break where the rule is off is no split either.
    assert!(splits("let z = (x\n-y)").is_empty());
}

#[test]
fn a_split_after_a_bare_return_is_recorded_once() {
    // The line after a bare `return` is no operand of it: no call, index or
    // `-` takes the keyword, so one split, of the value.
    assert_eq!(
        splits("return\n(g)(1)"),
        [("(".to_string(), SplitKind::Value)]
    );
    assert_eq!(splits("return\n-1"), [("-".to_string(), SplitKind::Value)]);
    assert_eq!(body(&f("return\n(g)(1)")), ["return", "tail:g(1)"]);
}

#[test]
fn a_closures_bare_return_before_a_parenthesis_or_bracket_is_one_split() {
    // The closure's body, a bare `return`, records the line's `Value`; the
    // `let` around the closure then declines the `(` or `[` at the same
    // token, which records nothing more.
    assert_eq!(
        splits("let h = || return\n(5)"),
        [("(".to_string(), SplitKind::Value)]
    );
    assert_eq!(
        splits("let h = || return\n[1, 2].len()"),
        [("[".to_string(), SplitKind::Value)]
    );
}

#[test]
fn a_bare_return_ending_an_arm_records_no_split() {
    assert!(splits("match x {\n    0 => return\n    (1) => 2\n    _ => 3\n}").is_empty());
}

#[test]
fn a_value_ending_an_arm_records_no_split_however_often_it_is_met() {
    // A `return`'s value or a closure's body ends at the `(`, and so does
    // the expression that holds it: both meet the split, and the arm drops
    // each.
    for body in ["return a", "break a", "|y| y", "return { a }"] {
        let src = format!("match x {{\n    0 => {body}\n    (1) => 2\n    _ => 3\n}}");
        assert!(splits(&src).is_empty(), "{body}");
    }
}

#[test]
fn a_bare_return_before_a_token_no_expression_starts_with_reads_as_before() {
    // `g(return` then `)`: as before the rule, the `)` is read as the
    // value's start, a parse error, and no split is recorded.
    assert!(!errors("g(return\n)").is_empty());
    assert!(!errors("let v = [return\n]").is_empty());
}

#[test]
fn a_parse_error_at_an_arm_the_rule_began_names_the_rule() {
    let errs = errors("match x {\n    0 => a\n    - 1,\n    _ => 2\n}");
    assert!(!errs.is_empty());
    assert!(
        errs[0]
            .notes()
            .iter()
            .any(|n| n.contains("begins the next match arm")),
        "{errs:?}"
    );
}

#[test]
fn a_parse_error_anywhere_in_an_arms_head_the_rule_began_names_the_rule() {
    // `g` then `(5)`: the parser before the rule read `g(5)` as the body;
    // now `(5)` is the next arm's pattern, and the P0001 lands at `_`.
    let errs = errors("match x {\n    0 => g\n    (5)\n    _ => 1\n}");
    assert_eq!(errs.len(), 1, "{errs:?}");
    assert!(
        errs[0]
            .notes()
            .iter()
            .any(|n| n.contains("begins the next match arm")),
        "{errs:?}"
    );
    // A bare `return` then `0,`: the base read `return 0`; now `0` is the
    // next arm's pattern, and the P0001 lands at `,`, with a note fitted to
    // the keyword.
    let errs = errors("match x {\n    None => return\n    0,\n    Some(y) => y,\n}");
    assert_eq!(errs.len(), 1, "{errs:?}");
    assert!(
        errs[0]
            .notes()
            .iter()
            .any(|n| n.contains("takes no value, so the next line begins the next match arm")),
        "{errs:?}"
    );
}

#[test]
fn a_bare_return_in_a_view_child_ends_the_child() {
    // As in any context, the `return` ends at the line break, the next line
    // is the next child, and the split is recorded: no parse error after it.
    let src = "fn f() -> Html {\n    view {\n        return\n        (1)\n    }\n}\n";
    let mut db = FileDb::new();
    let id = db.add("v.nova", src);
    let (tokens, _) = lex(src, id);
    let (file, errors) = parse(&tokens, id);
    assert!(errors.is_empty(), "{errors:?}");
    let file = file.expect("a file");
    let splits: Vec<(String, SplitKind)> = file
        .newline_splits
        .iter()
        .map(|s| {
            (
                src[s.at.start as usize..s.at.end as usize].to_string(),
                s.kind,
            )
        })
        .collect();
    assert_eq!(splits, [("(".to_string(), SplitKind::Value)]);
    let Item::Function(func) = &file.items[0].value else {
        panic!("a function")
    };
    let Some(tail) = &func.body.value.trailing else {
        panic!("a tail")
    };
    let Expr::View(children) = &tail.value else {
        panic!("a view")
    };
    assert_eq!(children.len(), 2);
}
