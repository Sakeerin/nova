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
