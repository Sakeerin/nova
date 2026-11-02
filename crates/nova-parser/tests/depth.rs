//! The depth limit (spec
//! `docs/superpowers/specs/2026-10-30-phase-5-1-the-lexer-the-parser-and-the-ast-design.md`
//! §4.4, ADR 0052): each kind of nesting parses at 128 levels, and at 129
//! gives one P0002 at the token where the limit was crossed, after which
//! the parser stops: no further error, and the items before it kept.
//!
//! Each input puts the token where its deepest level begins at the word
//! `leaf` (`Leaf` for a type); the file's outermost expression, type or
//! pattern is level 0 (`Parser::deeper`), so an input `d` levels deep is
//! `d` nested parentheses, blocks, calls or other constructs of its kind. The parses run on a thread with
//! a 64 MB stack, as S0's commands do, since a debug parser 128 levels
//! deep needs more than a test thread's 2 MB.

use nova_diagnostics::FileId;
use nova_lexer::lex;
use nova_parser::{parse, ParseError, MAX_DEPTH};

/// `f` on a thread with a 64 MB stack.
fn big_stack<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> T {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(f)
        .expect("start a thread")
        .join()
        .expect("the thread ends")
}

/// `src`'s top-level item names and parse errors.
fn parse_src(src: String) -> (Vec<String>, Vec<ParseError>) {
    big_stack(move || {
        let (tokens, lex_errors) = lex(&src, FileId::DUMMY);
        assert!(lex_errors.is_empty(), "{lex_errors:?}");
        let (file, errors) = parse(&tokens, FileId::DUMMY);
        let names = file
            .expect("a file")
            .items
            .iter()
            .map(|i| match &i.value {
                nova_ast::Item::Function(f) => f.name.value.clone(),
                other => format!("{other:?}"),
            })
            .collect();
        (names, errors)
    })
}

/// `n` copies of `s`.
fn rep(s: &str, n: usize) -> String {
    s.repeat(n)
}

/// `fn f() { body }`.
fn in_body(body: String) -> String {
    format!("fn f() {{\n    {body}\n}}\n")
}

/// A kind of nesting: its name, and a function from the depth `d` (at
/// least 2) to a file whose deepest level is `d`: `d` constructs of the
/// kind, each inside the one before.
type Kind = (&'static str, fn(usize) -> String);

/// The kinds of nesting.
fn kinds() -> Vec<Kind> {
    vec![
        ("parentheses", |d| {
            in_body(format!("{}leaf{}", rep("(", d), rep(")", d)))
        }),
        ("blocks", |d| {
            in_body(format!("{}leaf{}", rep("{ ", d), rep(" }", d)))
        }),
        ("calls", |d| {
            in_body(format!("{}leaf{}", rep("g(", d), rep(")", d)))
        }),
        ("indexes", |d| {
            in_body(format!("{}leaf{}", rep("a[", d), rep("]", d)))
        }),
        ("arrays", |d| {
            in_body(format!("{}leaf{}", rep("[", d), rep("]", d)))
        }),
        ("record literals", |d| {
            in_body(format!("{}leaf{}", rep("R { f: ", d), rep(" }", d)))
        }),
        ("interpolation holes", |d| {
            in_body(format!("{}leaf{}", rep("\"${", d), rep("}\"", d)))
        }),
        ("closures", |d| in_body(format!("{}leaf", rep("|| ", d)))),
        ("returns", |d| in_body(format!("{}leaf", rep("return ", d)))),
        ("prefix operators", |d| {
            // `&` takes a postfix expression, not another prefix one, so
            // it comes last.
            let ops = ["! ", "- ", "~ ", "* "];
            let prefix: String = (0..d - 1).map(|i| ops[i % ops.len()]).collect();
            in_body(format!("{prefix}&mut leaf"))
        }),
        ("a binary chain", |d| {
            in_body(format!("{}leaf", rep("1 + ", d)))
        }),
        ("a postfix chain", |d| {
            // The chain's first operator is not a level, and each after it
            // one more: the `k`th field is `k - 1` levels deep.
            in_body(format!("a{}.leaf", rep(".x", d)))
        }),
        ("method calls", |d| {
            // A method call's `(…)` counts with its `.name`.
            in_body(format!("a{}.leaf()", rep(".f()", d)))
        }),
        ("calls after `.await` and tuple indexes", |d| {
            // A call directly after any `.` postfix counts with it, as a
            // method call's `(…)` with its `.name`.
            let ops = [".await()", ".0()"];
            let chain: String = (0..d).map(|i| ops[i % ops.len()]).collect();
            in_body(format!("a{chain}.leaf()"))
        }),
        ("assignments", |d| {
            in_body(format!("{}leaf", rep("a = ", d)))
        }),
        ("casts", |d| {
            // The `k`th `as`, and its type, are level `k`.
            in_body(format!("0{} as Leaf", rep(" as Int", d - 1)))
        }),
        ("ifs", |d| {
            in_body(format!(
                "{}if leaf {{ 0 }} else {{ 0 }}{}",
                rep("if c { ", d - 1),
                rep(" } else { 0 }", d - 1)
            ))
        }),
        ("an else-if chain", |d| {
            in_body(format!(
                "if c {{ 0 }}{} else if leaf {{ 0 }} else {{ 0 }}",
                rep(" else if c { 0 }", d - 2)
            ))
        }),
        ("matches", |d| {
            in_body(format!(
                "{}match leaf {{ _ => 0 }}{}",
                rep("match c { _ => ", d - 1),
                rep(" }", d - 1)
            ))
        }),
        ("whiles", |d| {
            in_body(format!(
                "{}while leaf {{ 0 }}{}",
                rep("while c { ", d - 1),
                rep(" }", d - 1)
            ))
        }),
        ("fors", |d| {
            in_body(format!(
                "{}for leaf in c {{ 0 }}{}",
                rep("for x in c { ", d - 1),
                rep(" }", d - 1)
            ))
        }),
        ("items in blocks", |d| {
            in_body(format!("{}leaf{}", rep("fn g() { ", d), rep(" }", d)))
        }),
        ("types", |d| {
            // Each wrapper one level: generic arguments (with `>>` glued),
            // an array, a tuple, a reference, a pointer, a function type.
            let wrappers: [(&str, &str); 6] = [
                ("Vec<", ">"),
                ("[", "]"),
                ("(", ", Int)"),
                ("&", ""),
                ("*mut ", ""),
                ("fn(", ") -> Int"),
            ];
            let (mut open, mut close) = (String::new(), String::new());
            for i in 0..d {
                let (o, c) = wrappers[i % wrappers.len()];
                open.push_str(o);
                close.insert_str(0, c);
            }
            format!("fn f(x: {open}Leaf{close}) {{}}\n")
        }),
        ("optional types", |d| {
            // `T?`: each `?` after the first is a level, as a postfix
            // chain's operators are, so `d + 1` of them are `d` levels; the
            // line break ends the `let`, and `leaf` is the next statement.
            in_body(format!("let x: Int{}\n    leaf", rep("?", d + 1)))
        }),
        ("patterns", |d| {
            let wrappers: [(&str, &str); 6] = [
                ("S(", ")"),
                ("(", ", 0)"),
                ("[", "]"),
                ("R { f: ", " }"),
                ("m::S(", ")"),
                ("a @ ", ""),
            ];
            let (mut open, mut close) = (String::new(), String::new());
            for i in 0..d {
                let (o, c) = wrappers[i % wrappers.len()];
                open.push_str(o);
                close.insert_str(0, c);
            }
            in_body(format!("let {open}leaf{close} = 0"))
        }),
        ("mixed expressions", |d| {
            // Every wrapper is one level. A prefix operator binds tighter
            // than `+`, so `1 + ` never comes straight inside one.
            let wrappers: [(&str, &str); 11] = [
                ("(", ")"),
                ("{ ", " }"),
                ("g(", ")"),
                ("a[", "]"),
                ("[", "]"),
                ("!", ""),
                ("1 + ", ""),
                ("|| ", ""),
                ("if c { ", " } else { 0 }"),
                ("match c { _ => ", " }"),
                ("R { f: ", " }"),
            ];
            let (mut open, mut close) = (String::new(), String::new());
            let mut state = 7usize;
            let mut last = 0;
            for _ in 0..d - 1 {
                state = state
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1442695040888963407);
                let mut w = (state >> 33) % wrappers.len();
                if last == 5 && w == 6 {
                    w = 0;
                }
                last = w;
                let (o, c) = wrappers[w];
                open.push_str(o);
                close.insert_str(0, c);
            }
            in_body(format!("{open}(leaf){close}"))
        }),
    ]
}

#[test]
fn every_kind_of_nesting_parses_at_the_limit() {
    for (kind, src) in kinds() {
        let src = src(MAX_DEPTH);
        let (names, errors) = parse_src(src.clone());
        assert!(
            errors.is_empty(),
            "{kind} at {MAX_DEPTH}: {errors:?}\n{src}"
        );
        assert_eq!(names, ["f"], "{kind}");
    }
}

#[test]
fn every_kind_of_nesting_past_the_limit_is_one_p0002_at_its_deepest_token() {
    for (kind, src) in kinds() {
        let deep = src(MAX_DEPTH + 1);
        let leaf = deep
            .find("leaf")
            .or_else(|| deep.find("Leaf"))
            .expect("a leaf") as u32;
        // The items before it are kept; nothing after it is read, an error
        // included.
        let src = format!("fn before() {{}}\n{deep}fn after( {{\n) ]\n");
        let leaf = leaf + "fn before() {}\n".len() as u32;
        let (names, errors) = parse_src(src.clone());
        assert_eq!(errors.len(), 1, "{kind}: {errors:?}");
        let error = &errors[0];
        assert!(
            matches!(error, ParseError::TooDeep { .. }),
            "{kind}: {error:?}"
        );
        assert_eq!(error.code(), "P0002");
        assert_eq!(
            error.to_string(),
            "this is nested too deeply (more than 128 levels)"
        );
        assert_eq!(
            (error.span().start, error.span().end),
            (leaf, leaf + 4),
            "{kind}: {}",
            &src[error.span().start as usize..]
        );
        assert_eq!(names, ["before"], "{kind}");
    }
}

/// An element's or a view's children are a level, and its child
/// expressions are not: past the limit, the `{` that opens the children.
#[test]
fn view_children_are_a_level() {
    let view = |d: usize| {
        // The view's children are level 1, each element's one deeper.
        in_body(format!(
            "view {{ {}x{} }}",
            rep("div { ", d - 1),
            rep(" }", d - 1)
        ))
    };
    let (_, errors) = parse_src(view(MAX_DEPTH));
    assert!(errors.is_empty(), "{errors:?}");
    let src = view(MAX_DEPTH + 1);
    let last = src.rfind("div {").expect("a div") + "div ".len();
    let (_, errors) = parse_src(src);
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert_eq!(errors[0].code(), "P0002");
    assert_eq!(errors[0].span().start as usize, last);
}

/// An error before the limit is crossed stays, before the P0002; a file
/// nested past it twice has one P0002, at the first.
#[test]
fn errors_before_the_limit_stay_and_there_is_one_p0002() {
    let deep = format!("{}1{}", rep("(", MAX_DEPTH + 1), rep(")", MAX_DEPTH + 1));
    let src = format!("fn a() {{ let = 1; }}\nfn b() {{ {deep} }}\nfn c() {{ {deep} }}\n");
    let (names, errors) = parse_src(src);
    let codes: Vec<&str> = errors.iter().map(|e| e.code()).collect();
    assert_eq!(codes, ["P0001", "P0002"], "{errors:?}");
    assert_eq!(names, ["a"]);
}

/// The depth goes back down after each operand, operator chain, postfix
/// chain, optional type and sub-pattern: many siblings, each nested deep,
/// are fine, and so is a chain of operators in each of many parentheses,
/// an operand 100 deep after a chain of 99 operators of a higher
/// precedence, and a second cast after one whose type holds 100 `?`s.
#[test]
fn the_depth_goes_back_down_after_each_sibling() {
    let nest = format!("{}1{}", rep("(", 100), rep(")", 100));
    let args = vec![nest; 300].join(", ");
    let chain = format!("({});", rep("1 + ", 100) + "1");
    let chains = vec![chain; 300].join(" ");
    let calls = vec![format!("a{}", rep(".f(x)[0]?", 40)); 300].join("\n    ");
    // A chain of each binary precedence, and a postfix chain, then an
    // operator of the next lower one whose operand is 100 deep; and a view
    // whose first child is a postfix chain, its second's argument 100 deep.
    let deep = format!("{}1{}", rep("(", 100), rep(")", 100));
    let lowers = [
        (" as Int", " * "),
        (" * 1", " + "),
        (" + 1", " << "),
        (" << 1", " & "),
        (" & 1", " ^ "),
        (" ^ 1", " | "),
        (" | 1", " < "),
        (" && x", " || "),
        (" || x", " .. "),
        (".x", " * "),
    ]
    .map(|(op, lower)| format!("x{}{lower}{deep}", rep(op, 99)))
    .join("\n    ")
        + &format!("\n    view {{ a{} g({deep}) }}", rep(".x", 99))
        + &format!("\n    x as Int{q} as Int{q} * {deep}", q = rep("?", 100));
    let types = vec![format!("{}Int{}", rep("Vec<", 100), rep(">", 100)); 50].join(", ");
    let pats = vec![format!("{}x{}", rep("S(", 100), rep(")", 100)); 50].join(" | ");
    let src = format!(
        "fn f(t: ({types})) {{\n    g({args})\n    {chains}\n    {calls}\n    {lowers}\n    match t {{ {pats} => 0 }}\n    \
         if a {{ 1 }} else if b {{ 2 }} else {{ 3 }}\n    if a {{ 1 }} else if b {{ 2 }} else {{ 3 }}\n}}\n"
    );
    let (names, errors) = parse_src(src);
    assert!(errors.is_empty(), "{errors:?}");
    assert_eq!(names, ["f"]);
}

/// An abandoned attempt does not decide the depth alone (ADR 0052). The
/// type after a cast and `<` is first tried as generic arguments, a level
/// deeper than the expression the tokens then are, and after two casts two
/// levels: 127 and 126 parentheses take that attempt past the limit, and
/// its rollback takes the crossing back, with any error pushed after it
/// (the tuple type's missing `)`), so the expression parses, as the parser
/// before the limit read it, and an error later in the file is its own. At
/// 128 the expression itself is too deep: P0002 at its own deepest token.
/// The crossing stands if the tokens, read the other way, fail before its
/// token: a cast's type past the limit, read again as comparisons, and a
/// type `*mut`, which no expression has.
#[test]
fn a_rolled_back_crossing_stands_only_if_the_tokens_fail_the_other_way() {
    let paren = |n: usize| format!("x as Int < {}leaf{}", rep("(", n), rep(")", n));
    let tuple = |n: usize| format!("x as Int < (b, {}leaf{})", rep("(", n - 1), rep(")", n - 1));
    let two = |n: usize| format!("x as A as B < {}leaf{}", rep("(", n), rep(")", n));
    let later = format!(
        "{}\n}}\nfn g() {{ let = 1 }}\nfn h() {{",
        paren(MAX_DEPTH - 1)
    );
    let generic = format!("x as {}Leaf{}", rep("V<", 200), rep(">", 200));
    let ptr = format!("x as Int < (*mut {}leaf{})", rep("(", 126), rep(")", 126));
    for (shape, body, codes) in [
        ("parentheses", paren(MAX_DEPTH - 1), vec![]),
        ("a tuple", tuple(MAX_DEPTH - 1), vec![]),
        ("two casts", two(MAX_DEPTH - 2), vec![]),
        ("an error after", later, vec!["P0001"]),
        ("parentheses", paren(MAX_DEPTH), vec!["P0002"]),
        ("a tuple", tuple(MAX_DEPTH), vec!["P0002"]),
        ("two casts", two(MAX_DEPTH), vec!["P0002"]),
        ("a generic cast type", generic, vec!["P0002"]),
        ("a pointer type", ptr, vec!["P0002"]),
    ] {
        let src = in_body(body);
        let (_, errors) = parse_src(src.clone());
        let found: Vec<&str> = errors.iter().map(|e| e.code()).collect();
        assert_eq!(found, codes, "{shape}: {errors:?}");
        if codes == ["P0002"] {
            let leaf = src.find("leaf").or(src.find("Leaf")).expect("a leaf");
            let at = errors[0].span().start as usize;
            assert!(
                at <= leaf,
                "{shape}: P0002 at {at}, past the leaf at {leaf}"
            );
            if shape == "parentheses" || shape == "a tuple" {
                assert_eq!(at, leaf, "{shape}");
            }
        }
    }
}

/// A call directly after any `.` postfix, `.await` or a tuple index as
/// well as a field, counts with it, as a method call's `(…)` with its
/// `.name`: after a name, 129 `.await()`s or `.0()`s parse, and 130 are
/// one P0002, at the 130th's `await` or `0`.
#[test]
fn a_call_right_after_any_dot_postfix_counts_with_it() {
    for op in [".await()", ".0()"] {
        let (_, errors) = parse_src(in_body(format!("a{}", rep(op, MAX_DEPTH + 1))));
        assert!(errors.is_empty(), "{op} x 129: {errors:?}");
        let src = in_body(format!("a{}", rep(op, MAX_DEPTH + 2)));
        let at = src.rfind(op).expect("an operator") + 1;
        let (_, errors) = parse_src(src);
        let codes: Vec<&str> = errors.iter().map(|e| e.code()).collect();
        assert_eq!(codes, ["P0002"], "{op} x 130");
        assert_eq!(errors[0].span().start as usize, at, "{op} x 130");
    }
}

/// Far past the limit, as a fuzzer could write, the parser stops at 129
/// and never overflows its stack.
#[test]
fn far_past_the_limit_it_stops_without_overflowing() {
    for src in [
        in_body(format!("{}1{}", rep("(", 100_000), rep(")", 100_000))),
        in_body(rep("{ ", 100_000)),
        in_body(rep("!", 100_000) + "x"),
        in_body(rep("1 + ", 100_000) + "1"),
        in_body(format!("a{}", rep(".f()", 100_000))),
        in_body(rep("if c { ", 50_000)),
        format!("fn f(x: {}Int) {{}}\n", rep("Vec<", 100_000)),
        format!("fn f(x: Int{}) {{}}\n", rep("?", 100_000)),
        in_body(format!("let {}x = 0", rep("S(", 100_000))),
    ] {
        let (_, errors) = parse_src(src);
        let codes: Vec<&str> = errors.iter().map(|e| e.code()).collect();
        assert_eq!(codes, ["P0002"]);
    }
}
