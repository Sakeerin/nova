//! The layout rules (spec
//! `docs/superpowers/specs/2026-10-07-phase-3-1-formatter-design.md` §6).

mod common;
use common::{assert_formats, assert_stable};

#[test]
fn a_function_that_fits_stays_on_one_line() {
    assert_formats(
        "fn add(a: Int, b: Int) -> Int {\n    a + b\n}\n",
        "fn add(a: Int, b: Int) -> Int { a + b }\n",
    );
}

#[test]
fn a_block_with_two_statements_breaks() {
    assert_formats(
        "fn main() { let x = 1\n println(\"${x}\") }\n",
        "fn main() {\n    let x = 1\n    println(\"${x}\")\n}\n",
    );
}

#[test]
fn exactly_one_blank_line_between_top_level_items() {
    assert_formats(
        "fn a() {}\nfn b() {}\n\n\n\nfn c() {}\n",
        "fn a() {}\n\nfn b() {}\n\nfn c() {}\n",
    );
}

#[test]
fn blank_lines_in_a_block_are_kept_once_and_dropped_at_its_edges() {
    assert_formats(
        "fn main() {\n\n    let a = 1\n\n\n    let b = 2\n    println(\"${a}${b}\")\n\n}\n",
        "fn main() {\n    let a = 1\n\n    let b = 2\n    println(\"${a}${b}\")\n}\n",
    );
}

#[test]
fn an_if_else_that_fits_stays_on_one_line() {
    assert_formats(
        "fn f(b: Bool) -> String {\n    if b {\n        \"true\"\n    } else {\n        \"false\"\n    }\n}\n",
        "fn f(b: Bool) -> String { if b { \"true\" } else { \"false\" } }\n",
    );
}

#[test]
fn an_if_else_breaks_all_its_blocks_together() {
    assert_formats(
        "fn f(b: Bool) {\n    if b { println(\"a\") } else { println(\"b\")\n println(\"c\") }\n}\n",
        "fn f(b: Bool) {\n    if b {\n        println(\"a\")\n    } else {\n        println(\"b\")\n        println(\"c\")\n    }\n}\n",
    );
}

#[test]
fn else_if_stays_on_the_closing_braces_line() {
    assert_formats(
        "fn f(n: Int) -> Int {\n    if n < 0 { 0 }\n    else if n == 0 { 1 }\n    else { 2 }\n}\n",
        "fn f(n: Int) -> Int { if n < 0 { 0 } else if n == 0 { 1 } else { 2 } }\n",
    );
}

#[test]
fn a_long_signature_breaks_its_parameters_and_its_body() {
    assert_formats(
        "pub fn connect(host: String, port: Int, timeout: Duration, retries: Int, verbose: Bool) -> Result<Connection, NetError> {\n    open(host)\n}\n",
        "pub fn connect(\n    host: String,\n    port: Int,\n    timeout: Duration,\n    retries: Int,\n    verbose: Bool,\n) -> Result<Connection, NetError> {\n    open(host)\n}\n",
    );
}

#[test]
fn a_body_too_long_for_the_line_breaks_but_its_signature_stays() {
    // From std/collections: the function does not fit on one line, but its
    // signature does, and so does its body's one expression.
    assert_stable(
        "pub fn get(self, i: Int) -> Option<T> {\n    if i < 0 { None } else { if i >= self.len { None } else { Some(self.data[i]) } }\n}\n",
    );
}

#[test]
fn a_record_that_fits_goes_on_one_line() {
    assert_formats(
        "pub record CryptoError {\n    pub kind: CryptoErrorKind\n    pub message: String\n}\n",
        "pub record CryptoError { pub kind: CryptoErrorKind, pub message: String }\n",
    );
}

#[test]
fn a_record_that_does_not_fit_gets_one_field_per_line_with_commas() {
    assert_formats(
        "record Config { name: String, version: String, description: String, license: String, repository: String }\n",
        "record Config {\n    name: String,\n    version: String,\n    description: String,\n    license: String,\n    repository: String,\n}\n",
    );
}

#[test]
fn blank_lines_between_fields_and_between_variants_are_kept() {
    // One blank line survives however many there were, and it breaks the
    // list (spec §5.3).
    assert_formats(
        "record P {\n    x: Int,\n\n\n    y: Int,\n}\n\ntype T =\n    | A\n\n    | B\n",
        "record P {\n    x: Int,\n\n    y: Int,\n}\n\ntype T =\n    | A\n\n    | B\n",
    );
}

#[test]
fn a_sum_type_that_fits_goes_on_one_line() {
    assert_formats(
        "pub type Method =\n    | Get\n    | Post\n",
        "pub type Method = | Get | Post\n",
    );
    assert_stable("pub type Option<T> = | Some(T) | None\n");
}

#[test]
fn a_sum_type_that_does_not_fit_gets_one_variant_per_line() {
    assert_formats(
        "pub type CryptoErrorKind = | EntropyUnavailable | InvalidLength | RequestTooLarge | InvalidKey | VerificationFailed\n",
        "pub type CryptoErrorKind =\n    | EntropyUnavailable\n    | InvalidLength\n    | RequestTooLarge\n    | InvalidKey\n    | VerificationFailed\n",
    );
}

#[test]
fn a_method_chain_breaks_before_each_call() {
    assert_formats(
        "fn main() {\n    let app = Router::new().get(\"/\", |_| Response::text(200, \"Hello from Nova!\")).get(\"/health\", |_| Response::json(status_ok()))\n    serve(app)\n}\n",
        "fn main() {\n    let app = Router::new()\n        .get(\"/\", |_| Response::text(200, \"Hello from Nova!\"))\n        .get(\"/health\", |_| Response::json(status_ok()))\n    serve(app)\n}\n",
    );
}

#[test]
fn a_long_condition_breaks_before_each_operator() {
    assert_formats(
        "fn f() -> Bool {\n    first_condition_is_true(a) && second_condition_is_true(b) && third_condition_is_true(c) && fourth(d)\n}\n",
        "fn f() -> Bool {\n    first_condition_is_true(a)\n        && second_condition_is_true(b)\n        && third_condition_is_true(c)\n        && fourth(d)\n}\n",
    );
}

#[test]
fn match_arms_go_one_per_line_without_commas() {
    assert_formats(
        "fn f(x: Int) -> String {\n    match x { 0 => \"zero\", 1 => \"one\", _ => \"many\" }\n}\n",
        "fn f(x: Int) -> String {\n    match x {\n        0 => \"zero\"\n        1 => \"one\"\n        _ => \"many\"\n    }\n}\n",
    );
}

#[test]
fn imports_are_sorted_within_a_run() {
    assert_formats(
        "import zeta\nimport alpha::{c, a, b}\n\nfn main() {}\n",
        "import alpha::{a, b, c}\nimport zeta\n\nfn main() {}\n",
    );
    // Blank lines between imports do not end their run, and do not survive
    // inside it (the 3.1 plan's decision 3).
    assert_formats("import b\n\n\nimport a\n", "import a\nimport b\n");
}

#[test]
fn imports_are_sorted_only_within_their_run() {
    assert_formats(
        "import b\nfn f() {}\nimport a\n",
        "import b\n\nfn f() {}\n\nimport a\n",
    );
}

#[test]
fn a_call_exactly_100_columns_wide_stays_on_one_line() {
    assert_stable(
        "fn main() {\n    report(first_argument_value, second_argument_value, third_argument_value, fourth_argument_value)\n    done()\n}\n",
    );
}

#[test]
fn a_call_101_columns_wide_puts_one_argument_per_line() {
    assert_formats(
        "fn main() {\n    reports(first_argument_value, second_argument_value, third_argument_value, fourth_argument_value)\n    done()\n}\n",
        "fn main() {\n    reports(\n        first_argument_value,\n        second_argument_value,\n        third_argument_value,\n        fourth_argument_value,\n    )\n    done()\n}\n",
    );
}

#[test]
fn small_forms_keep_their_usual_spacing() {
    assert_stable("fn main() { let t = (1,) }\n");
    assert_stable("fn f(p: P) -> P { P { x: 1, ..p } }\n");
    assert_stable("fn main() { for i in 0..=3 { println(\"${i}\") } }\n");
    assert_stable("fn f(x: Int) -> Int { -x + (x as Int) * 2 }\n");
    assert_stable("fn main() { let xs = [0; 4] }\n");
}

#[test]
fn closures_print_their_parameters_bare_or_typed() {
    assert_formats(
        "fn main() { let f = |a: Int, b| a + b\n let g = | | 1\n println(\"${f(1, 2)}${g()}\") }\n",
        "fn main() {\n    let f = |a: Int, b| a + b\n    let g = | | 1\n    println(\"${f(1, 2)}${g()}\")\n}\n",
    );
}

#[test]
fn trait_impl_and_extern_bodies_list_one_member_per_line() {
    assert_formats(
        "trait Shape { fn area(self) -> Float\n fn name(self) -> String { \"shape\" } }\nimpl Shape for Square { fn area(self) -> Float { self.side * self.side } }\nextern \"C\" { fn abs(x: Int) -> Int }\n",
        "trait Shape {\n    fn area(self) -> Float\n    fn name(self) -> String { \"shape\" }\n}\n\nimpl Shape for Square {\n    fn area(self) -> Float { self.side * self.side }\n}\n\nextern \"C\" {\n    fn abs(x: Int) -> Int\n}\n",
    );
}

#[test]
fn consts_and_type_aliases() {
    assert_formats(
        "pub const MAX: Int = 10\ntype Id = Int\n",
        "pub const MAX: Int = 10\n\ntype Id = Int\n",
    );
}

#[test]
fn a_where_clause_that_fits_stays_on_the_signature() {
    assert_stable("fn show<T: Display + Clone>(x: T) -> String where T: Debug { x.fmt() }\n");
}

#[test]
fn a_long_where_clause_takes_lines_of_its_own() {
    assert_formats(
        "fn f<T, U>(x: T, y: U) -> T where T: FirstVeryLongTraitName + SecondVeryLongTraitName, U: ThirdVeryLongTraitName + Fourth { x }\n",
        "fn f<T, U>(\n    x: T,\n    y: U,\n) -> T\nwhere\n    T: FirstVeryLongTraitName + SecondVeryLongTraitName,\n    U: ThirdVeryLongTraitName + Fourth,\n{\n    x\n}\n",
    );
}

#[test]
fn an_impl_keeps_its_traits_type_arguments() {
    assert_formats(
        "impl Into<String> for Name { fn into(self) -> String { self.value } }\n",
        "impl Into<String> for Name {\n    fn into(self) -> String { self.value }\n}\n",
    );
}

#[test]
fn doc_comments_print_before_attributes_and_on_fields_and_variants() {
    assert_formats(
        "@test\n/// Adds one.\n///   Indented.\nfn inc(x: Int) -> Int { x + 1 }\n",
        "/// Adds one.\n///   Indented.\n@test\nfn inc(x: Int) -> Int { x + 1 }\n",
    );
    assert_stable(
        "record P {\n    /// The x.\n    x: Int,\n}\n\ntype T =\n    /// First.\n    | A\n    | B\n",
    );
}
