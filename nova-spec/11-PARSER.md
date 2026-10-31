# 11 — Parser & Grammar Specification

> Crate: `nova-parser`, `nova-ast`
> Phase: 0
> Depends on: `nova-lexer`, `nova-diagnostics`

---

## 1. Approach

- **Parser library:** `chumsky` v0.10 (parser combinators with error recovery)
- **Expressions:** Pratt parsing for operator precedence (chumsky's `pratt` combinator)
- **Error recovery:** must produce partial AST + errors; never abort on first error
- **Output:** `Spanned<Expr>`, `Spanned<Stmt>`, etc., with full span coverage

**Amended 2026-10-06 (branch `phase-2-closeout`): the parser does not use
chumsky.** It is hand-written recursive descent
(`crates/nova-parser/src/grammar.rs`). Operator precedence comes from explicit
layering, assignment down to postfix, not from a Pratt combinator. After an
error the parser skips to the next item boundary (an item keyword such as
`fn`, `pub` or `record`, or `}`) or statement boundary (`;` or `}`), the first
two levels §5 lists, and `parse` currently always returns `Some`. So the
first two bullets above, the grammar's "(Pratt)" label, §4's "rare with
chumsky" and §5's `recover_with` describe a design that was not built. §7's
fuzz target does not exist either: there is no `fuzz/` directory.
`docs/adr/0025-phase-2-boundary.md` decides the hand-written parser stays, and
maps fuzzing to Phase 6.

---

## 2. Formal Grammar (EBNF)

```ebnf
(* === TOP LEVEL === *)

file        = { item } ;

item        = function
            | record_decl
            | type_decl
            | trait_decl
            | impl_block
            | const_decl
            | import_decl
            | module_decl
            | extern_block ;

(* === ITEMS === *)

function    = [ "pub" ] [ "async" ] "fn" ident
              [ generics ] "(" [ params ] ")"
              [ "->" type ]
              [ where_clause ]
              block ;

params      = param { "," param } [ "," ] ;
param       = [ "mut" ] ident ":" type ;

record_decl = [ "pub" ] "record" ident [ generics ]
              "{" { record_field } "}" ;
record_field= [ "pub" ] ident ":" type "," ;

type_decl   = [ "pub" ] "type" ident [ generics ] "="
              type_expr ";" ;
type_expr   = type
            | "|" variant { "|" variant } ;  (* sum types *)
variant     = ident [ "(" type { "," type } ")" ] ;

trait_decl  = [ "pub" ] "trait" ident [ generics ]
              [ ":" trait_bound { "+" trait_bound } ]
              "{" { trait_item } "}" ;
trait_item  = function_sig ";" | function ;

impl_block  = "impl" [ generics ] [ trait_path "for" ] type
              [ where_clause ]
              "{" { function | const_decl } "}" ;

const_decl  = [ "pub" ] "const" ident ":" type "=" expr ";" ;

import_decl = "import" path [ "as" ident | "{" import_list "}" ] ";" ;
import_list = ident { "," ident } [ "," ] ;

module_decl = "module" path ";" ;

extern_block= [ attr ] "extern" [ string_lit ]
              "{" { extern_item } "}" ;

(* === TYPES === *)

type        = type_atom { "?" }                 (* optional sugar *)
type_atom   = path [ generic_args ]
            | "&" [ "mut" ] type                (* references *)
            | "*" [ "mut" ] type                (* raw pointers, unsafe only *)
            | "[" type "]"                      (* array/slice *)
            | "(" [ type { "," type } [ "," ] ] ")"  (* tuple, () = unit *)
            | "fn" "(" [ type { "," type } ] ")" [ "->" type ] ;

generics    = "<" type_param { "," type_param } ">" ;
type_param  = ident [ ":" trait_bound { "+" trait_bound } ] ;
generic_args= "<" type { "," type } ">" ;
trait_bound = path [ generic_args ] ;
where_clause= "where" type ":" trait_bound { "+" trait_bound }
              { "," type ":" trait_bound { "+" trait_bound } } ;

(* === STATEMENTS === *)

block       = "{" { stmt } [ expr ] "}" ;
stmt        = let_stmt
            | expr_stmt
            | item ;                            (* nested items allowed *)

let_stmt    = "let" [ "mut" ] pattern [ ":" type ] [ "=" expr ] ";" ;
expr_stmt   = expr ";" ;

(* === PATTERNS === *)

pattern     = "_"                               (* wildcard *)
            | literal
            | ident [ "@" pattern ]             (* binding *)
            | path "(" pattern { "," pattern } ")"  (* enum variant *)
            | path "{" field_pat { "," field_pat } "}"  (* record *)
            | "(" pattern { "," pattern } ")"   (* tuple *)
            | "[" pattern { "," pattern } "]"   (* array *)
            | pattern "|" pattern               (* or-patterns *)
            | range_pattern ;

field_pat   = ident [ ":" pattern ] ;

(* === EXPRESSIONS (Pratt) === *)

expr        = pratt_expr ;

(* Atomic expressions *)
atom        = literal
            | path
            | "(" expr ")"                      (* paren or unit *)
            | "(" expr "," expr { "," expr } ")"  (* tuple *)
            | "[" [ expr { "," expr } ] "]"     (* array literal *)
            | block
            | if_expr
            | match_expr
            | while_expr
            | for_expr
            | "return" [ expr ]
            | "break" [ expr ]
            | "continue"
            | closure
            | record_literal ;

if_expr     = "if" expr block [ "else" ( if_expr | block ) ] ;

match_expr  = "match" expr "{" { match_arm } "}" ;
match_arm   = pattern [ "if" expr ] "=>" expr "," ;

while_expr  = "while" expr block ;
for_expr    = "for" pattern "in" expr block ;

closure     = "|" [ params_loose ] "|" [ "->" type ] expr ;
params_loose= ident [ ":" type ] { "," ident [ ":" type ] } ;

record_literal = path "{" [ field_init { "," field_init } ] "}" ;
field_init  = ident [ ":" expr ]                (* shorthand if missing *)
            | ".." expr ;                       (* spread *)

(* === OPERATORS (precedence low to high) === *)

(* Precedence table:
   1   ||
   2   &&
   3   == != < <= > >=
   4   |
   5   ^
   6   &
   7   << >>
   8   + -
   9   * / %
   10  as is
   11  unary: - ! ~ &mut & *
   12  postfix: () [] . ? .await
*)

(* === MISC === *)

path        = ident { "::" ident } ;
literal     = INT_LIT | FLOAT_LIT | STRING_LIT | CHAR_LIT
            | "true" | "false" ;
attr        = "@" ident [ "(" attr_args ")" ] ;
attr_args   = attr_arg { "," attr_arg } ;
attr_arg    = ident [ "=" literal ] | literal ;
```

**Amended 2026-10-07 (branch `phase-3-1-formatter`):** three changes to the
grammar above (`docs/superpowers/specs/2026-10-07-phase-3-1-formatter-design.md`
§4):
- **Doc comments.** One or more `///` lines may come before an item, at the
  top level or in a block; a trait member; an impl member; a record field; a
  sum-type variant; or a function in an `extern` block. On a top-level item
  they may come before, between or after its attributes. The node gains
  `docs`, one entry per line. Anywhere else a `///` is an error: "a doc
  comment must come right before an item, a field or a variant; use `//`
  for a plain comment".
- **`where`** takes a trailing comma before the `{`, `;` or `}` that ends
  the clause.
- **Pattern spans.** A parenthesised pattern, and the unit pattern `()`, are
  spanned over their parentheses, as parenthesised expressions and types
  already were.

**Amended 2026-10-08 (branch `phase-3-2-lsp-core`):** two recoveries for
the language server (`docs/superpowers/specs/2026-10-08-phase-3-2-lsp-core-design.md`
§3.2, §3.4):
- A `.` followed by something other than a name still reports P0001, and
  now keeps its receiver as a field access with an empty name.
- `parse_recovering` also names the top-level items dropped after their
  names were read.

`parse` is unchanged, and only the server's analysis type-checks the empty
name.

**Amended 2026-10-17 (branch `claude/loving-archimedes-frtf5q-44`):** component syntax
(RFC 0001; `docs/superpowers/specs/2026-10-17-phase-4-4-component-syntax-design.md` §3):

```ebnf
item        += [ "pub" ] "component" ident "(" [ params ] ")" block ;
atom        += "view" "{" { child } "}"
             | "||" [ "->" type ] expr ;
child        = ( element | postfix_expr ) [ ";" ] ;
element      = ident "{" { child } "}"
             | ident "(" [ view_args ] ")" [ "{" { child } "}" ] ;
view_args    = view_arg { "," view_arg } [ "," ] ;
view_arg     = ( ident | keyword ) ":" expr | expr ;
```

- `component` is a keyword where an item starts, a name and `(` follow,
  and `{` (or `->`) follows its parameters;
  the node is a `Function` with `is_component` set and no return type. One
  written with `->` is an error, "a component returns a `View`, which it
  does not write".
- `view` is a keyword where an expression starts and `{` follows, but not
  where a record literal could not start, in an `if`, `while`, `for` or
  `match` head. Inside its braces, record literals are allowed again.
- An element is a name and a block, or a name and arguments holding a
  named one, or both; a name and positional arguments alone is a call.
  Named arguments parse nowhere else.
- A `;` may end a view's child, so the next can begin with `(`, `[` or
  `{`.

**Amended 2026-10-26 (branch `claude/loving-archimedes-frtf5q-50`):** **the
newline rule**
(`docs/superpowers/specs/2026-10-26-phase-5-0-the-language-and-std-a-compiler-needs-design.md`
§3.5; ADR 0043, whose lexer skips line breaks, and ADR 0028 §5, whose `;`
fmt printed around it). The grammar above writes `;` after each statement;
the parser has never needed one, and now implements the master spec's
"newline terminates statement" as follows. The lexer marks each token that
follows a line break, a comment holding one included. **In statement
context, a line break ends the expression before a `(` (a call), a `[` (an
index) or a binary `-`;** the token begins the next statement or match arm.
Every other operator, and `.`, continue the expression, as before.

**Statement context** is the expression of a statement, a block's tail, a
`let`'s value, a `return`'s or `break`'s value, and a match arm's body. It
is off inside `(` and `)`, `[` and `]`, argument lists, record literals'
braces, interpolation holes, a view's children and an element's arguments, a
guard, and the heads of `if`, `while`, `for` and `match`, which end at their
`{`; a block anywhere, and a match's arms, turn it back on. A `{` on a new
line keeps its meaning. **A `return` or `break` at the end of a line takes
no value,** wherever it is.

So `let z = x`, then `-y` on the next line, is two statements;
`if c { 1 } else { 2 }`, then `(a, b)`, is not a call of the `if`. The
parser records each such split, where the parser before the rule would
have continued, and the checker warns at each (E0103), but at the next arm
after an arm's body; a `;` written at the end of the line before says the
break is meant, so there is no split to record. A view's children keep
their own rule: a `;` ends a child before a `(` or `[`.

**Amended 2026-10-27 (branch `claude/loving-archimedes-frtf5q-50b`):** **tuples
and `t.N`**
(`docs/superpowers/specs/2026-10-26-phase-5-0-the-language-and-std-a-compiler-needs-design.md`
§4.1, §4.2). Tuple types, literals and patterns parse as the grammar above
writes them, and as the parser already read them: `()` is unit, `(e)` and
`(T)` are the expression and the type, and a tuple of one is written with a
trailing comma, `(e,)` and `(T,)`. A tuple literal is parenthesised, so the
newline rule is off inside it. **A tuple's element is a postfix `.` and an
index:**

```ebnf
postfix_op  = … | "." INT_LIT ;   (* t.0, plain decimal digits *)
```

The index is plain decimal digits with no leading zero: `t.01`, `t.1_0`,
`t.0x1` and `t.00` are refused (P0001, "a tuple index is plain decimal
digits, with no leading zero, `_` or prefix"), as Rust does, so each index
has one spelling. `t.1e5` is a float there, and P0001 as before. The node is
`Expr::Field` whose field is the index's digits, so `t.0` is
`Field { field: "0" }`. **The lexer splits a float after a `.`:** a float
token whose text is digits, `.`, digits (`1.0`) that follows a `.` becomes an
integer, a `.` and an integer, so `t.1.0` reads as `(t.1).0` and `t.0.1.2`
chains. It splits by the source text, not the value, so `t.1.10` is
`(t.1).10`; after a `..`, or after any other token, `1.0` stays a float, as
does a float with a `_` or an exponent.

**Amended 2026-10-27 (branch `claude/loving-archimedes-frtf5q-50b`):**
**patterns**
(`docs/superpowers/specs/2026-10-26-phase-5-0-the-language-and-std-a-compiler-needs-design.md`
§4.3 to §4.5). The parser reads patterns as before; 5.0b's checker takes
more of them. `range_pattern`, which the grammar above leaves undefined, is
what the parser reads:

```ebnf
range_pattern = INT_LIT ( ".." | "..=" ) INT_LIT   (* 0..10, 0..=9 *)
              | CHAR_LIT "..=" CHAR_LIT ;          (* 'a'..='z' *)
```

An end is a literal, never a path or an expression, and there is no `-` in a
pattern, so a negative end does not parse. An alternative of an or-pattern
is a pattern without a top-level `|`; parentheses group one, `Some(1 | 2)`
needs none, and `(p)` is `p`. A tuple pattern of one is `(p,)`, and `()` is
the unit pattern. `mut` does not parse inside a pattern; `let mut (a, b) =
e` makes each binding of the pattern mutable. A guard, `pattern if expr =>`,
ends at its `=>`, so the newline rule is off in it. **The checker takes**
`_`, names, `Int`, `Bool`, `String` and `Char` literals, variants with field
patterns, tuples, or-patterns at any depth, ranges and guards in `match`;
an irrefutable pattern in `let` and `for`. Record and array patterns,
`x @ p` and `Float` literals stay E0900. `for p in a` runs over an array
`a` too, beside a range and an iterator: the checker desugars it to an index
loop that evaluates `a` once and reads each element when it reaches it, the
index advanced before the body. A match arm that starts with a
tuple pattern, after an arm that ended at a line break, begins a new arm by
the newline rule above.

---

## 3. AST Node Definitions (Rust)

In `crates/nova-ast/src/lib.rs`:

```rust
use nova_diagnostics::{Span, Spanned};

#[derive(Debug, Clone)]
pub struct File {
    pub items: Vec<Spanned<Item>>,
}

#[derive(Debug, Clone)]
pub enum Item {
    Function(Function),
    Record(Record),
    Type(TypeDecl),
    Trait(TraitDecl),
    Impl(ImplBlock),
    Const(ConstDecl),
    Import(Import),
    Module(Module),
    Extern(ExternBlock),
}

#[derive(Debug, Clone)]
pub struct Function {
    pub vis: Visibility,
    pub is_async: bool,
    pub name: Spanned<String>,
    pub generics: Vec<TypeParam>,
    pub params: Vec<Param>,
    pub return_ty: Option<Spanned<Type>>,
    pub where_clause: Vec<WhereBound>,
    pub body: Spanned<Block>,
}

#[derive(Debug, Clone)]
pub struct Param {
    pub is_mut: bool,
    pub name: Spanned<String>,
    pub ty: Spanned<Type>,
}

#[derive(Debug, Clone)]
pub enum Visibility { Pub, Private }

#[derive(Debug, Clone)]
pub enum Type {
    Path { path: Path, args: Vec<Spanned<Type>> },
    Ref { is_mut: bool, inner: Box<Spanned<Type>> },
    Array(Box<Spanned<Type>>),
    Tuple(Vec<Spanned<Type>>),
    Fn { params: Vec<Spanned<Type>>, ret: Box<Spanned<Type>> },
    Optional(Box<Spanned<Type>>),
    Infer,  // _
}

#[derive(Debug, Clone)]
pub enum Stmt {
    Let { is_mut: bool, pattern: Spanned<Pattern>, ty: Option<Spanned<Type>>, init: Option<Spanned<Expr>> },
    Expr(Spanned<Expr>),
    Item(Item),
}

#[derive(Debug, Clone)]
pub struct Block {
    pub stmts: Vec<Spanned<Stmt>>,
    pub trailing: Option<Box<Spanned<Expr>>>,  // expression-as-block-result
}

#[derive(Debug, Clone)]
pub enum Expr {
    Lit(Literal),
    Path(Path),
    Tuple(Vec<Spanned<Expr>>),
    Array(Vec<Spanned<Expr>>),
    Block(Block),
    If { cond: Box<Spanned<Expr>>, then: Box<Spanned<Block>>, else_: Option<Box<Spanned<Expr>>> },
    Match { scrutinee: Box<Spanned<Expr>>, arms: Vec<MatchArm> },
    While { cond: Box<Spanned<Expr>>, body: Box<Spanned<Block>> },
    For { pattern: Spanned<Pattern>, iter: Box<Spanned<Expr>>, body: Box<Spanned<Block>> },
    Return(Option<Box<Spanned<Expr>>>),
    Break(Option<Box<Spanned<Expr>>>),
    Continue,
    Closure { params: Vec<Param>, ret: Option<Spanned<Type>>, body: Box<Spanned<Expr>> },
    Record { path: Path, fields: Vec<FieldInit>, base: Option<Box<Spanned<Expr>>> },
    Binary { op: BinOp, lhs: Box<Spanned<Expr>>, rhs: Box<Spanned<Expr>> },
    Unary { op: UnOp, expr: Box<Spanned<Expr>> },
    Call { callee: Box<Spanned<Expr>>, args: Vec<Spanned<Expr>> },
    Index { target: Box<Spanned<Expr>>, index: Box<Spanned<Expr>> },
    Field { target: Box<Spanned<Expr>>, field: Spanned<String> },
    Try(Box<Spanned<Expr>>),  // expr?
    Await(Box<Spanned<Expr>>),
    Cast { expr: Box<Spanned<Expr>>, ty: Spanned<Type> },
    Assign { op: AssignOp, lhs: Box<Spanned<Expr>>, rhs: Box<Spanned<Expr>> },
    StringInterp(Vec<StringPart>),
}

#[derive(Debug, Clone)]
pub enum StringPart {
    Lit(String),
    Expr(Spanned<Expr>),
}

#[derive(Debug, Clone)]
pub enum Literal {
    Int(i64),
    Float(f64),
    Str(String),
    Char(char),
    Bool(bool),
}

// ... and so on for Pattern, BinOp, UnOp, etc.
```

Place each major variant in its own file (`expr.rs`, `pattern.rs`, `ty.rs`, `item.rs`) re-exported from `lib.rs`.

---

## 4. Public API

```rust
pub fn parse(tokens: &[Spanned<Token>], file: FileId)
    -> (Option<File>, Vec<ParseError>);
```

- Returns `Some(File)` even with errors when recovery succeeds
- Returns `None` only on catastrophic failure (rare with chumsky)

**Amended 2026-10-30 (branch `claude/loving-archimedes-frtf5q-51a`):** **the
token dump**
(`docs/superpowers/specs/2026-10-30-phase-5-1-the-lexer-the-parser-and-the-ast-design.md`
§3.3; ADR 0051, which writes the format out). `nova_lexer::dump_tokens(source)`
renders one source's tokens, lex errors and comments as text, one per line:
`start end Kind[ payload][ NL][ PD]` for each token, `Eof` last, with the
`Token` variant's name, decimal `Int`s, a `Float`'s 16 hex digits of bits, a
`Char`'s code point, and text in double quotes with control bytes, `"` and
`\` written `\xHH`; then `ERR start end message` for each lex error; then
`COMMENT start end Line|Block`. The hidden `nova parse --dump-tokens FILE…`
prints it after a `== FILE` header per file, and the self-hosted lexer's
`selfhost tokens FILE…` prints the same bytes. It carries no stability
promise: it changes with the tokens, in both compilers at once. 5.1b adds an
AST dump beside it.

---

## 5. Error Recovery

Chumsky's `recover_with` strategies — apply at these levels (most permissive first):
- **Item level:** sync to next `fn`, `record`, `pub`, `import`, etc.
- **Statement level:** sync to next `;` or `}`
- **Expression level:** sync to operator, `,`, or `)`

Every error → `ParseError` with span + expected/found info → diagnostics crate renders.

---

## 6. Operator Precedence Table

| Prec | Operators | Associativity |
|---|---|---|
| 1 | `\|\|` | Left |
| 2 | `&&` | Left |
| 3 | `==` `!=` `<` `<=` `>` `>=` | None (chained = error) |
| 4 | `\|` | Left |
| 5 | `^` | Left |
| 6 | `&` | Left |
| 7 | `<<` `>>` | Left |
| 8 | `+` `-` | Left |
| 9 | `*` `/` `%` | Left |
| 10 | `as` `is` | Left |
| 11 (prefix) | `-` `!` `~` `&` `&mut` `*` | — |
| 12 (postfix) | `()` `[]` `.` `?` `.await` | — |
| 13 (assign, right-assoc) | `=` `+=` `-=` `*=` `/=` `%=` `\|=` `&=` `^=` `<<=` `>>=` | Right |

Comparison operators (==, !=, <, etc.) are non-chainable: `a < b < c` is an error with helpful suggestion.

**Amended 2026-10-26 (branch `claude/loving-archimedes-frtf5q-50`):** a
postfix `(` or `[`, and a binary `-`, after a line break in statement
context begin the next statement (§2's newline rule). Compound assignment
works on any place plain assignment accepts, a field (`self.f += x`) and an
element (`a[i] -= 1`) included, as this table always allowed: the receiver
and the index are evaluated once
(`docs/superpowers/specs/2026-10-26-phase-5-0-the-language-and-std-a-compiler-needs-design.md`
§3.4). The postfix `?` works on `Option` and `Result` (`12-TYPESYSTEM.md`
§7).

---

## 7. Tests Required

1. **Snapshot tests** for parsing each item kind, expression kind, statement kind
2. **Recovery tests:** intentionally malformed code, assert AST has expected partial structure + errors
3. **Roundtrip tests:** `parse(format(parse(s)))` produces same AST (after Phase 3 formatter exists)
4. **Property tests:** generate random valid AST → format → reparse → assert equal
5. **Fuzz target:** `fuzz/fuzz_targets/parse.rs` — never panic

---

## 8. Reference Snippets (for testing)

Files in `crates/nova-parser/tests/fixtures/`:

```nova
// fixtures/hello.nova
fn main() {
    println("Hello, World!")
}
```

```nova
// fixtures/generics.nova
fn map<T, U>(list: [T], f: fn(T) -> U) -> [U] {
    let mut out = []
    for item in list {
        out.push(f(item))
    }
    out
}
```

```nova
// fixtures/sum_type.nova
type Result<T, E> =
  | Ok(T)
  | Err(E)

fn divide(a: Int, b: Int) -> Result<Int, String> {
    if b == 0 {
        Err("division by zero")
    } else {
        Ok(a / b)
    }
}
```

```nova
// fixtures/match.nova
fn describe(n: Int) -> String {
    match n {
        0 => "zero",
        1..=9 => "small",
        n if n < 0 => "negative",
        _ => "large",
    }
}
```

```nova
// fixtures/async.nova
async fn fetch_user(id: Int) -> Result<User, HttpError> {
    let res = http.get("/users/${id}").await?
    let user = res.json::<User>()?
    Ok(user)
}
```

```nova
// fixtures/trait_impl.nova
trait Display {
    fn fmt(self) -> String
}

record Point { x: Float, y: Float }

impl Display for Point {
    fn fmt(self) -> String {
        "(${self.x}, ${self.y})"
    }
}
```

These must all parse successfully with zero errors.
