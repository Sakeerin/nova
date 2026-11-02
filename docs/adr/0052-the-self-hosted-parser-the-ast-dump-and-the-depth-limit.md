# ADR 0052 — The self-hosted parser, the AST dump and the depth limit

## Status

Accepted (2026-10-31). Branch `claude/loving-archimedes-frtf5q-51b`
(`docs/superpowers/specs/2026-10-30-phase-5-1-the-lexer-the-parser-and-the-ast-design.md`
§4). §§1–5 are S0's side of 5.1b: the AST dump and `nova parse
--dump-ast`, the depth limit and P0002, S0's pipeline thread, the error
cases and the deep generator. §§6–10 are N's: its AST and parser, each
Rust idiom's Nova translation, the comparison of the two dumps, N's own
stack at the limit, the speed figures, and the rule that the grammar
changes in both compilers at once. The limit of 128 levels, the code
P0002, the 64 MB thread and the error corpus are the spec's
recommendations, which the user did not overturn; the dump's indented
S-expression with spans is the user's decision 3 of 2026-10-30; counting
each postfix operator after a chain's first toward the same 128 levels,
which the spec's list of levels does not name, is the user's choice of
2026-10-31 (their ruling 8), which covers each `?` of an optional type
`T?` after the first by the same principle (§2). Six departures from the
spec are the plan's, each marked **(differs)** where it is described: a
split's and an attribute's form in the dump (§1), the dump split between
two crates (§1), a block that is not a level (§2), P0002 dropping the
item in progress (§2), N's own tests in `src/` (§6), and S0's release
speed measured as a library (§9). The rest is what the spec left to the
plan.

## Context

5.1b ports `nova-parser` and `nova-ast` to Nova, and compares N's tree
and errors with S0's through a dump both print (spec §4.3). S0's only
rendering of a tree was `nova parse --ast`, Rust's pretty `Debug`: 51.8 MB
for all of std, with floats and strings in Rust's own syntax. S0's parser
had no depth guard: on its 8 MB main thread, under `nova check`, a debug
build passed 230 nested parentheses and aborted at 240, passed 190 nested
`if`s and aborted at 200, passed a chain of 750 operators and aborted at
800, and passed a chain of 500 method calls and aborted at 600; Windows
gives a main thread 1 MB. ADR 0051 found that `logos`
recurses once per digit, so a 4,000-character decimal number overflowed
S0 at 1 MB. The parser's errors were all P0001, attached by each caller.

## Decision

### 1. The AST dump

`nova_ast::dump::dump_file(&File) -> String` writes the tree;
`nova_parser::dump_ast(source) -> String` lexes and parses `source` and
adds its errors; the hidden `nova parse --dump-ast FILE…` prints, for
each file, `== FILE` (the path as given) and its dump, or `IOERR` and the
error's kind for a file it cannot read, then exits 2 after the rest, as
`--dump-tokens` does (ADR 0051). The tree lives in `nova-ast`, where the
nodes are; the errors need the parser's and the lexer's types, so the
whole dump is `nova-parser`'s **(differs from the spec**, whose §7 puts
one `dump` module in `nova-ast`). The flag refuses `--ast`, `--check` and
`--dump-tokens`, and `--help` is unchanged.

**The grammar.** Every line ends with `\n`, the last included.

```
dump      = tree errors
tree      = "(File" { "\n" node(1) } ")" "\n"
node(d)   = indent(d) "(" kind [ " " span ] { " " value } { " " field }
            { "\n" node(d + 1) } ")"
indent(d) = d times "  "
span      = uint ".." uint                       -- byte offsets
field     = key "=" value                        -- key: the Rust field's name
value     = bool | word | string | name | path | literal
bool      = "true" | "false"
word      = a field-less enum's variant: Visibility (Pub, Private),
            BinOp, UnOp, AssignOp, SplitKind (Call, Index, Minus, Value);
            or Some, None (an element's children)
string    = '"' { char } '"'                     -- a char below 0x20, 0x7F,
                                                 -- '"' and '\' as "\x" hh
name      = string "@" span                      -- a Spanned<String>
path      = name { "::" name }
literal   = "Int:" int | "Float:" 16 hex digits of f64::to_bits
          | "Str:" string | "Char:" code point in decimal | "Bool:" bool
errors    = { "ERR " uint " " uint " " message "\n" }
            { "PARSE " uint " " uint " " message "\n" { "NOTE " message "\n" } }
message   = the Display text, each char below 0x20 and 0x7F as "\x" hh
```

`hh` is two lowercase hex digits; `uint` and `int` are decimal. A node's
`)` comes right after its last child's, so a line ends with as many `)` as
nodes it closes. Its values are positional (a tuple variant's scalars, a
`Doc`'s text, an `Attr`'s name and arguments, a `Path` node's segments);
its fields are its named scalars, in the struct's order, then its children
in the struct's order. An absent `Option` and an empty list print nothing.
A `Spanned` node prints its span; a node the tree holds without one (every
struct but `Block` and `Path` under a `Spanned`, an `Item` under
`Stmt.Item`, a `Block` under `Expr.Block`) prints none. Lex errors come
first, all of them, as the callers render them before the parse errors.

**Every kind.** A struct's kind is its name, and an enum value's
`Enum.Variant`, both as Rust names them, but for three the spec named:
`Doc` (a `///` line), `Attr` (`Attribute`) and `Split` (`NewlineSplit`).
Two of them print otherwise than the spec's sketch (§4.3) **(differs
from the spec)**: a split's kind is the field `kind=Call`, not a bare
`Call`, the uniform `key=value` form of every field-less enum; and an
attribute's name and arguments are names with their spans,
`"test"@1..5 "should_panic"@6..18`, not a bare name and quoted strings,
since each is a `Spanned<String>` and the dump gives every name its span.
An enum value that holds a struct is a node whose one child is the
struct's node. `nova_ast::dump::KINDS` lists the 167 forms (24 structs,
`Doc`, the 18 enums' 119 variants as node kinds, `key=Word` values and
literal prefixes, and each of the 11 `bool` fields' two values and
`children=`'s two, after their node's kind, as `(Type.Ptr is_mut=false`,
but `Pattern.Ident`'s `is_mut=true`, which no parser builds). A test reads
the `struct` and `enum` declarations of `nova-ast`'s sources and
checks that `KINDS` is exactly what they imply, so a new variant or
`bool` field fails until it is listed; another checks that each form
appears in S0's dump, and in N's, of some corpus file or case.

| Node | Span | On its line, in order | Children, in order |
|---|---|---|---|
| `File` | – | – | `Item.*` (spanned), `Split` |
| `Split` | `at` | `kind=` | – |
| `Doc` | yes | text | – |
| `Attr` | the `@` | name, each argument (names) | – |
| `Item.Function` … `Item.Extern` (9) | under `File` | – | its struct |
| `Function` | – | `vis= is_async= is_component= name=` | `Doc`, `Attr`, `TypeParam`, `Param`, return type, `WhereBound`, `Block` |
| `FunctionSig` | – | `is_async= name=` | `Doc`, `TypeParam`, `Param`, return type, `WhereBound` |
| `Param` | – | `is_mut= name=` | type |
| `TypeParam` | – | `name=` | `Path` (bounds) |
| `WhereBound` | – | – | type, `Path` (bounds) |
| `Record` | – | `vis= name=` | `Doc`, `Attr`, `TypeParam`, `RecordField` |
| `RecordField` | – | `vis= name=` | `Doc`, type |
| `TypeDecl` | – | `vis= name=` | `Doc`, `Attr`, `TypeParam`, `TypeDef.*` |
| `TypeDef.Alias` | – | – | type |
| `TypeDef.Sum` | – | – | `Variant` |
| `Variant` | – | `name=` | `Doc`, types |
| `TraitDecl` | – | `vis= name=` | `Doc`, `Attr`, `TypeParam`, `Path` (supertraits), `WhereBound`, `TraitItem.*` |
| `TraitItem.Required` / `.Provided` | – | – | `FunctionSig` / `Function` |
| `TraitItem.AssocType` | – | `name=` | `Doc`, `Path` (bounds) |
| `ImplBlock` | – | – | `Doc`, `Attr`, `TypeParam`, `Path` (the trait), type, `WhereBound`, `Function`, `ConstDecl`, `AssocTypeBinding` |
| `AssocTypeBinding` | – | `name=` | `Doc`, type |
| `ConstDecl` | – | `vis= name=` | `Doc`, `Attr`, type, expression |
| `Import` | – | – | `Doc`, `Attr`, `Path`, `ImportKind.*` |
| `ImportKind.Simple` / `.Alias` / `.List` | – | – / name / names | – |
| `Module` | – | – | `Doc`, `Attr`, `Path` |
| `ExternBlock` | – | `abi=` (a string, if any) | `Doc`, `Attr`, `ExternItem.Fn` |
| `ExternItem.Fn` | – | – | `FunctionSig` |
| `Path` | yes | the path | – |
| `Block` | under a `Spanned` | – | `Stmt.*`, the trailing expression |
| `Stmt.Let` | yes | `is_mut=` | pattern, type, initializer |
| `Stmt.Expr` / `Stmt.Item` | yes | – | expression / `Item.*` (no span) |
| `Expr.Lit` | yes | literal | – |
| `Expr.Path` | yes | the path | – |
| `Expr.Tuple`, `Expr.Array` | yes | – | expressions |
| `Expr.ArrayRepeat` | yes | – | `init`, `len` |
| `Expr.Block` | yes | – | `Block` (no span) |
| `Expr.If` | yes | – | condition, `Block`, else |
| `Expr.Match` | yes | – | scrutinee, `MatchArm` |
| `MatchArm` | – | – | pattern, guard, body |
| `Expr.While` | yes | – | condition, `Block` |
| `Expr.For` | yes | – | pattern, iterable, `Block` |
| `Expr.Range` | yes | `inclusive=` | `lo`, `hi` |
| `Expr.Return`, `Expr.Break` | yes | – | the value |
| `Expr.Continue` | yes | – | – |
| `Expr.Closure` | yes | – | `Param`, return type, body |
| `Expr.Record` | yes | `path=` | `FieldInit`, the base |
| `FieldInit` | – | `name=` | value |
| `Expr.Binary` | yes | `op=` (BinOp) | `lhs`, `rhs` |
| `Expr.Unary` | yes | `op=` (UnOp) | operand |
| `Expr.Call` | yes | – | callee, arguments |
| `Expr.Index` | yes | – | target, index |
| `Expr.Field` | yes | `field=` | target |
| `Expr.Try`, `Expr.Await` | yes | – | operand |
| `Expr.Cast` | yes | – | operand, type |
| `Expr.Assign` | yes | `op=` (AssignOp) | `lhs`, `rhs` |
| `Expr.StringInterp` | yes | – | `StringPart.*` |
| `StringPart.Lit` / `.Expr` | – | text / – | – / expression |
| `Expr.View` | yes | – | `ViewNode.*` |
| `ViewNode.Element` | yes | `name= children=Some\|None` | `ViewArg.*`, `ViewNode.*` |
| `ViewNode.Expr` | yes | – | expression |
| `ViewArg.Named` / `.Positional` | – | `name=` / – | expression |
| `Pattern.Wildcard` | yes | – | – |
| `Pattern.Lit` | yes | literal | – |
| `Pattern.Ident` | yes | `is_mut= name=` | – |
| `Pattern.Binding` | yes | `name=` | pattern |
| `Pattern.TupleStruct` | yes | `path=` | patterns |
| `Pattern.Record` | yes | `path= rest=` | `FieldPat` |
| `FieldPat` | – | `name=` | pattern |
| `Pattern.Tuple`, `.Array`, `.Or` | yes | – | patterns |
| `Pattern.Range` | yes | `inclusive=` | `lo`, `hi` |
| `Pattern.Path` | yes | the path | – |
| `Type.Path` | yes | `path=` | arguments |
| `Type.Ref`, `Type.Ptr` | yes | `is_mut=` | type |
| `Type.Array`, `Type.Optional` | yes | – | type |
| `Type.Tuple` | yes | – | types |
| `Type.Fn` | yes | – | parameters, return type |
| `Type.Infer` | yes | – | – |

`ViewNode.Element`'s `children=` is the one field the spec's rules did not
give: its children are an `Option` of a list, and `None` and an empty list
would print alike.

**One example of every kind**, each S0's dump of the source after it,
reduced to the node's own subtree, the kinds after the first appearing
inside it:

```
;; File: fn main() {}\n
(File
  (Item.Function 0..12
    (Function vis=Private is_async=false is_component=false name="main"@3..7
      (Block 10..12))))

;; Split: fn f() {\n    g\n    (x)\n}\n
(Split 19..20 kind=Call)

;; Doc: /// Adds.\nfn f() {}\n
(Doc 0..9 " Adds.")

;; Attr: @test(should_panic)\nfn t() {}\n
(Attr 0..1 "test"@1..5 "should_panic"@6..18)

;; Item.Function, Function, Block: pub async fn f() {}\n
(Item.Function 0..19
  (Function vis=Pub is_async=true is_component=false name="f"@13..14
    (Block 17..19)))

;; Param: fn f(mut x: Int, self) {}\n
(Param is_mut=true name="x"@9..10
  (Type.Path 12..15 path="Int"@12..15))

;; TypeParam, Path: fn f<T: std::fmt::Display>() {}\n
(TypeParam name="T"@5..6
  (Path 8..11 "std"@8..11::"fmt"@13..16::"Display"@18..25))

;; WhereBound: fn f<T>(x: T) where T: Show + Eq {}\n
(WhereBound
  (Type.Path 20..21 path="T"@20..21)
  (Path 23..27 "Show"@23..27)
  (Path 30..32 "Eq"@30..32))

;; Item.Record, Record, RecordField: pub record P {\n    pub x: Int,\n}\n
(Item.Record 0..32
  (Record vis=Pub name="P"@11..12
    (RecordField vis=Pub name="x"@23..24
      (Type.Path 26..29 path="Int"@26..29))))

;; Item.Type, TypeDecl, TypeDef.Sum, Variant: type C = | Red | Rgb(Int, Int)\n
(Item.Type 0..30
  (TypeDecl vis=Private name="C"@5..6
    (TypeDef.Sum
      (Variant name="Red"@11..14)
      (Variant name="Rgb"@17..20
        (Type.Path 21..24 path="Int"@21..24)
        (Type.Path 26..29 path="Int"@26..29)))))

;; TypeDef.Alias: type Name = String\n
(TypeDef.Alias
  (Type.Path 12..18 path="String"@12..18))

;; Item.Trait, TraitDecl: trait Show: Base {}\n
(Item.Trait 0..19
  (TraitDecl vis=Private name="Show"@6..10
    (Path 12..16 "Base"@12..16)))

;; TraitItem.Required, FunctionSig: trait Show {\n    fn show(self) -> String\n}\n
(TraitItem.Required
  (FunctionSig is_async=false name="show"@20..24
    (Param is_mut=false name="self"@25..29
      (Type.Infer 25..29))
    (Type.Path 34..40 path="String"@34..40)))

;; TraitItem.Provided: trait Show {\n    fn twice(self) {}\n}\n
(TraitItem.Provided
  (Function vis=Private is_async=false is_component=false name="twice"@20..25
    (Param is_mut=false name="self"@26..30
      (Type.Infer 26..30))
    (Block 32..34)))

;; TraitItem.AssocType: trait It {\n    type Item: Show\n}\n
(TraitItem.AssocType name="Item"@20..24
  (Path 26..30 "Show"@26..30))

;; Item.Impl, ImplBlock: impl Show for P {}\n
(Item.Impl 0..18
  (ImplBlock
    (Path 5..9 "Show"@5..9)
    (Type.Path 14..15 path="P"@14..15)))

;; AssocTypeBinding: impl It for P {\n    type Item = Int\n}\n
(AssocTypeBinding name="Item"@25..29
  (Type.Path 32..35 path="Int"@32..35))

;; Item.Const, ConstDecl: const K: Int = 1\n
(Item.Const 0..16
  (ConstDecl vis=Private name="K"@6..7
    (Type.Path 9..12 path="Int"@9..12)
    (Expr.Lit 15..16 Int:1)))

;; Item.Import, Import, ImportKind.Simple: import json\n
(Item.Import 0..11
  (Import
    (Path 7..11 "json"@7..11)
    (ImportKind.Simple)))

;; ImportKind.Alias: import json as j\n
(ImportKind.Alias "j"@15..16)

;; ImportKind.List: import a::{b, c}\n
(ImportKind.List "b"@11..12 "c"@14..15)

;; Item.Module, Module: module util\n
(Item.Module 0..11
  (Module
    (Path 7..11 "util"@7..11)))

;; Item.Extern, ExternBlock, ExternItem.Fn: extern "C" {\n    fn abs(x: Int) -> Int\n}\n
(Item.Extern 0..40
  (ExternBlock abi="C"
    (ExternItem.Fn
      (FunctionSig is_async=false name="abs"@20..23
        (Param is_mut=false name="x"@24..25
          (Type.Path 27..30 path="Int"@27..30))
        (Type.Path 35..38 path="Int"@35..38)))))

;; Stmt.Let: fn f() {\n    let mut x: Int = 1\n}\n
(Stmt.Let 13..31 is_mut=true
  (Pattern.Ident 21..22 is_mut=false name="x"@21..22)
  (Type.Path 24..27 path="Int"@24..27)
  (Expr.Lit 30..31 Int:1))

;; Stmt.Expr: fn f() {\n    g();\n    1\n}\n
(Stmt.Expr 13..17
  (Expr.Call 13..16
    (Expr.Path 13..14 "g"@13..14)))

;; Stmt.Item: fn f() {\n    fn g() {}\n}\n
(Stmt.Item 13..22
  (Item.Function
    (Function vis=Private is_async=false is_component=false name="g"@16..17
      (Block 20..22))))

;; Expr.Lit: fn f() {\n    1\n}\n
(Expr.Lit 13..14 Int:1)

;; Expr.Path: fn f() {\n    a::b\n}\n
(Expr.Path 13..17 "a"@13..14::"b"@16..17)

;; Expr.Tuple: fn f() {\n    (true, r"a\n", 2.5)\n}\n
(Expr.Tuple 13..32
  (Expr.Lit 14..18 Bool:true)
  (Expr.Lit 20..26 Str:"a\x5cn")
  (Expr.Lit 28..31 Float:4004000000000000))

;; Expr.Array: fn f() {\n    ['a', 'b']\n}\n
(Expr.Array 13..23
  (Expr.Lit 14..17 Char:97)
  (Expr.Lit 19..22 Char:98))

;; Expr.ArrayRepeat: fn f() {\n    [0; 3]\n}\n
(Expr.ArrayRepeat 13..19
  (Expr.Lit 14..15 Int:0)
  (Expr.Lit 17..18 Int:3))

;; Expr.Block: fn f() {\n    { 1 }\n}\n
(Expr.Block 13..18
  (Block
    (Expr.Lit 15..16 Int:1)))

;; Expr.If: fn f() {\n    if c { 1 } else { 2 }\n}\n
(Expr.If 13..34
  (Expr.Path 16..17 "c"@16..17)
  (Block 18..23
    (Expr.Lit 20..21 Int:1))
  (Expr.Block 29..34
    (Block
      (Expr.Lit 31..32 Int:2))))

;; Expr.Match, MatchArm: fn f() {\n    match x { 0 if y => 1, _ => 2 }\n}\n
(Expr.Match 13..44
  (Expr.Path 19..20 "x"@19..20)
  (MatchArm
    (Pattern.Lit 23..24 Int:0)
    (Expr.Path 28..29 "y"@28..29)
    (Expr.Lit 33..34 Int:1))
  (MatchArm
    (Pattern.Wildcard 36..37)
    (Expr.Lit 41..42 Int:2)))

;; Expr.While: fn f() {\n    while c { g() }\n}\n
(Expr.While 13..28
  (Expr.Path 19..20 "c"@19..20)
  (Block 21..28
    (Expr.Call 23..26
      (Expr.Path 23..24 "g"@23..24))))

;; Expr.For: fn f() {\n    for i in xs { g(i) }\n}\n
(Expr.For 13..33
  (Pattern.Ident 17..18 is_mut=false name="i"@17..18)
  (Expr.Path 22..24 "xs"@22..24)
  (Block 25..33
    (Expr.Call 27..31
      (Expr.Path 27..28 "g"@27..28)
      (Expr.Path 29..30 "i"@29..30))))

;; Expr.Range: fn f() {\n    0..=9\n}\n
(Expr.Range 13..18 inclusive=true
  (Expr.Lit 13..14 Int:0)
  (Expr.Lit 17..18 Int:9))

;; Expr.Return: fn f() {\n    return 1\n}\n
(Expr.Return 13..21
  (Expr.Lit 20..21 Int:1))

;; Expr.Break: fn f() {\n    break\n}\n
(Expr.Break 13..18)

;; Expr.Continue: fn f() {\n    continue\n}\n
(Expr.Continue 13..21)

;; Expr.Closure: fn f() {\n    |x: Int| -> Int x\n}\n
(Expr.Closure 13..30
  (Param is_mut=false name="x"@14..15
    (Type.Path 17..20 path="Int"@17..20))
  (Type.Path 25..28 path="Int"@25..28)
  (Expr.Path 29..30 "x"@29..30))

;; Expr.Record, FieldInit: fn f() {\n    P { x, y: 1, ..p }\n}\n
(Expr.Record 13..31 path="P"@13..14
  (FieldInit name="x"@17..18)
  (FieldInit name="y"@20..21
    (Expr.Lit 23..24 Int:1))
  (Expr.Path 28..29 "p"@28..29))

;; Expr.Binary: fn f() {\n    a + b\n}\n
(Expr.Binary 13..18 op=Add
  (Expr.Path 13..14 "a"@13..14)
  (Expr.Path 17..18 "b"@17..18))

;; Expr.Unary: fn f() {\n    -x\n}\n
(Expr.Unary 13..15 op=Neg
  (Expr.Path 14..15 "x"@14..15))

;; Expr.Call: fn f() {\n    g(1, x)\n}\n
(Expr.Call 13..20
  (Expr.Path 13..14 "g"@13..14)
  (Expr.Lit 15..16 Int:1)
  (Expr.Path 18..19 "x"@18..19))

;; Expr.Index: fn f() {\n    a[i]\n}\n
(Expr.Index 13..17
  (Expr.Path 13..14 "a"@13..14)
  (Expr.Path 15..16 "i"@15..16))

;; Expr.Field: fn f() {\n    p.x\n}\n
(Expr.Field 13..16 field="x"@15..16
  (Expr.Path 13..14 "p"@13..14))

;; Expr.Try: fn f() {\n    g()?\n}\n
(Expr.Try 13..17
  (Expr.Call 13..16
    (Expr.Path 13..14 "g"@13..14)))

;; Expr.Await: fn f() {\n    g().await\n}\n
(Expr.Await 13..22
  (Expr.Call 13..16
    (Expr.Path 13..14 "g"@13..14)))

;; Expr.Cast: fn f() {\n    n as Float\n}\n
(Expr.Cast 13..23
  (Expr.Path 13..14 "n"@13..14)
  (Type.Path 18..23 path="Float"@18..23))

;; Expr.Assign: fn f() {\n    x += 1\n}\n
(Expr.Assign 13..19 op=AddAssign
  (Expr.Path 13..14 "x"@13..14)
  (Expr.Lit 18..19 Int:1))

;; Expr.StringInterp, StringPart.Lit, StringPart.Expr: fn f() {\n    "a${b}"\n}\n
(Expr.StringInterp 13..20
  (StringPart.Lit "a")
  (StringPart.Expr
    (Expr.Path 17..18 "b"@17..18)))

;; Expr.View, ViewNode.Element, ViewArg.Positional, ViewArg.Named, ViewNode.Expr: fn f() {\n    view { button(on, label: "x") { text("hi") } }\n}\n
(Expr.View 13..59
  (ViewNode.Element 20..57 name="button"@20..26 children=Some
    (ViewArg.Positional
      (Expr.Path 27..29 "on"@27..29))
    (ViewArg.Named name="label"@31..36
      (Expr.StringInterp 38..41
        (StringPart.Lit "x")))
    (ViewNode.Expr 45..55
      (Expr.Call 45..55
        (Expr.Path 45..49 "text"@45..49)
        (Expr.StringInterp 50..54
          (StringPart.Lit "hi"))))))

;; Pattern.Wildcard: fn f() {\n    let _ = 1\n}\n
(Pattern.Wildcard 17..18)

;; Pattern.Lit: fn f() {\n    match c { 'a' => 1 }\n}\n
(Pattern.Lit 23..26 Char:97)

;; Pattern.Ident: fn f() {\n    let x = 1\n}\n
(Pattern.Ident 17..18 is_mut=false name="x"@17..18)

;; Pattern.Binding: fn f() {\n    match n { d @ 1..=9 => d }\n}\n
(Pattern.Binding 23..32 name="d"@23..24
  (Pattern.Range 27..32 inclusive=true
    (Pattern.Lit 27..28 Int:1)
    (Pattern.Lit 31..32 Int:9)))

;; Pattern.TupleStruct: fn f() {\n    let Some(x) = o\n}\n
(Pattern.TupleStruct 17..24 path="Some"@17..21
  (Pattern.Ident 22..23 is_mut=false name="x"@22..23))

;; Pattern.Record, FieldPat: fn f() {\n    let P { x, y: 0, .. } = p\n}\n
(Pattern.Record 17..34 path="P"@17..18 rest=true
  (FieldPat name="x"@21..22)
  (FieldPat name="y"@24..25
    (Pattern.Lit 27..28 Int:0)))

;; Pattern.Tuple: fn f() {\n    let (a, b) = t\n}\n
(Pattern.Tuple 17..23
  (Pattern.Ident 18..19 is_mut=false name="a"@18..19)
  (Pattern.Ident 21..22 is_mut=false name="b"@21..22))

;; Pattern.Array: fn f() {\n    let [a, b] = xs\n}\n
(Pattern.Array 17..23
  (Pattern.Ident 18..19 is_mut=false name="a"@18..19)
  (Pattern.Ident 21..22 is_mut=false name="b"@21..22))

;; Pattern.Or: fn f() {\n    match c { A | B => 1 }\n}\n
(Pattern.Or 23..28
  (Pattern.Ident 23..24 is_mut=false name="A"@23..24)
  (Pattern.Ident 27..28 is_mut=false name="B"@27..28))

;; Pattern.Range: fn f() {\n    match c { 'a'..='z' => 1 }\n}\n
(Pattern.Range 23..32 inclusive=true
  (Pattern.Lit 23..26 Char:97)
  (Pattern.Lit 29..32 Char:122))

;; Pattern.Path: fn f() {\n    match c { C::Red => 1 }\n}\n
(Pattern.Path 23..29 "C"@23..24::"Red"@26..29)

;; Type.Path: fn f(x: Vec<Int>) {}\n
(Type.Path 8..16 path="Vec"@8..11
  (Type.Path 12..15 path="Int"@12..15))

;; Type.Ref: fn f(x: &mut Int) {}\n
(Type.Ref 8..16 is_mut=true
  (Type.Path 13..16 path="Int"@13..16))

;; Type.Ptr: fn f(x: *Int) {}\n
(Type.Ptr 8..12 is_mut=false
  (Type.Path 9..12 path="Int"@9..12))

;; Type.Array: fn f(x: [Int]) {}\n
(Type.Array 8..13
  (Type.Path 9..12 path="Int"@9..12))

;; Type.Tuple: fn f(x: (Int, Bool)) {}\n
(Type.Tuple 8..19
  (Type.Path 9..12 path="Int"@9..12)
  (Type.Path 14..18 path="Bool"@14..18))

;; Type.Fn: fn f(x: fn(Int) -> Bool) {}\n
(Type.Fn 8..23
  (Type.Path 11..14 path="Int"@11..14)
  (Type.Path 19..23 path="Bool"@19..23))

;; Type.Optional: fn f(x: Int?) {}\n
(Type.Optional 8..12
  (Type.Path 8..11 path="Int"@8..11))

;; Type.Infer: fn f() {\n    |x| x\n}\n
(Type.Infer 14..15)

```

### 2. The depth limit, P0002

**A level.** `Parser::deeper` raises one counter, `depth`, and every level
is entered through it. It counts:

- each expression (`parse_expr`), type (`parse_type`) and pattern
  (`parse_pattern`): so every operand inside parentheses, a block's
  statements, a call's arguments, an index, an array, a record literal's
  fields, an interpolation hole, a closure's body, a `return`'s value, the
  heads of `if`, `while`, `for` and `match`, a match arm's guard and body;
  each type argument; each sub-pattern;
- the operand of a prefix operator, `&` and `&mut` included;
- each `else if`;
- each operator of a binary chain, assignment, `..` and `as` included,
  for the rest of its chain: the right operand of a chain's `k`th
  operator is `k` levels deeper than the chain's first operand, and so is
  the type after its `k`th `as` (`Parser::parse_type_here` reads a type at
  the cursor's level; `parse_type` is `nested(parse_type_here)`);
- each postfix operator of a chain after its first (a call, an index, a
  field, `?`, `.await`), for the rest of the chain, a call directly after
  any `.` postfix counting with it as one operator (a method call
  `.name(…)`, and as well `.await(…)` and a tuple field's `.0(…)`, which
  both parsers count alike): the `k`th operator of a chain is
  `k - 1` levels deeper than its receiver, and a call's or an index's
  expressions one deeper again, as any expression is. The spec does not
  list postfix chains: they are counted by the user's choice of
  2026-10-31 (ruling 8), asked because, uncounted, a chain of about
  5,000 method calls overflowed `nova check` even on the 64 MB thread
  (§3); see below;
- each `?` of an optional type `T?` after the first, for the rest of the
  type: `T` followed by `k` `?`s is `k - 1` levels deeper than `T`. It is
  a chain as a postfix chain is, which the first build missed: uncounted,
  each `?` wrapped one more `Type::Optional`, and 40,000 of them after a
  parameter's type overflowed `nova fmt --check` on S0's 64 MB thread,
  while N crashed on a 1 MB stack at 14,000 where S0 printed the tree
  (5.1b's review). The user's ruling 8 covers it by the same principle:
  after a type's name, 129 `?`s parse, 128 levels, and 130 are P0002, at
  the token after the 130th;
- an item inside a block;
- the children of a `view` and of each element.

The file's outermost expression, type, pattern or nested item is level 0,
so `n` nested constructs of any one kind are `n` levels: 128 nested
parentheses, blocks, calls, `if`s (an `if` and its `else if`s count
alike), `match`es, prefix operators, binary operators, casts, types,
patterns or items parse, and 129 are P0002, as the spec's gate (§4.8)
reads. `depth` counts every entry, the outermost's included, so it is one
more than the level, and `deeper` refuses once it passes `MAX_DEPTH + 1`.
(The first build of this counted the outermost as level 1, so only 127
parentheses fitted, and the type after `as` was a level of its own beyond
its operator's.) A block is no level of its own: its statements'
expressions are **(differs from the spec**, which lists a block: counting
a block and its statements' expressions both would make `{ x }` two
levels and 64 nested blocks the limit). Each entry but the chains' is
`Parser::nested(f)`, which saves the depth, calls `deeper`, runs `f` and
restores the depth however `f` ends. `parse_postfix_from` saves the depth,
runs the chain (`parse_postfix_chain`, which calls `deeper` after taking
each operator but the first and a method call's `(`) and restores it
however the chain ends, so a view child, which reads a postfix expression
outside `parse_expr`, gets it back too. `parse_type_here` saves the depth,
reads the type and its `?`s (`parse_optional_chain`, which calls `deeper`
after each `?` but the first) and restores it, so a type's siblings, and
the type after the next `as`, do not add up. Each binary function
(`parse_assign`, `parse_range`, `parse_or`, …, `parse_mul`, `parse_cast`)
saves the depth on entry, calls `deeper` after each operator it takes, and
restores the depth when its chain ends; a failure returns `None` straight
up to `parse_expr`, which restores it. So, as a statement, `((x))` is 2
levels deep, `{ { x } }` 2, `if c { if d { x } }` 2 (an `if`'s condition
and its block's tail are one deeper than the `if`), `if a { x } else if b
{ y }` 2, `1 + 2 + x` 2, `!!x` 2, `0 as Int as T` 2, `a.b.c` 1,
`a.f().g()` 1, `a.f(x)` 0 with `x` at 1, and `fn g() { fn h() { x } }` 2;
as a parameter's type, `Vec<Vec<Int>>` is 2 and `Int???` 2. So after a name 129 field
accesses in a row parse, and 129 method calls without arguments; a call's
arguments are a level deeper than the call, so 128 calls with arguments
parse and the 129th's are P0002. `Router::new()` is itself the chain's
first operator, so a router built as one chain, one `.get("/", h)` a
route, parses 127 routes and is P0002 at the 128th's arguments (measured
on both parsers). Past the limit, P0002 is at the token after the
operator that crossed it, as after a binary operator.

**Past the limit.** `nova_parser::MAX_DEPTH` is 128. The first time the
depth passes it, `deeper` records the number of errors so far and the span
of the token at the cursor, and returns `false`; the caller returns
`None`. From then on `peek` reads the end of the file, as it does after a
bare `return`, so every function unwinds as at the end of the file. When
the file ends, the errors pushed since are dropped and
`ParseError::TooDeep { span }` is pushed: "this is nested too deeply (more
than 128 levels)". The items finished before are kept, the one being
parsed is dropped, however far it got, no split is recorded after it, and
P0002 is the file's last error **(differs from the spec**, which says the
tree holds what came before: one clean error, and both parsers stop at
the same point without carrying a half-built tree). `ParseError::code()` gives "P0002" for it, "P0001" for
every other, and every caller (`nova parse`, the driver's loader, the
resolver's std loader, `nova fmt`) attaches it; the language server's
organize-imports treats P0002 as a syntax error too. The dump's `PARSE`
line carries no code: P0002's message names it.

**An abandoned attempt.** The parser tries the tokens after a type's `<`
as generic arguments and rolls back if they are not: after a cast, `x as
Int < (a)` is then a comparison, and a type elsewhere ends before the
`<`, which is then an error. The attempt can be deeper than the reading
that follows: in `x as Int < ((a))` the attempt reads `((a))` two levels
below the start of the line (the `as`, then the type argument), and the
comparison reads it one below (the `<`), so 127 parentheses took the
attempt past the limit and S0 reported P0002 where
the base parsed the line (a fault the second fact-check found, in both
parsers). The rollback (`Parser::roll_back`) now restores the position,
the pending `>`s and the P0002 state as they were, drops the errors the
attempt pushed after it crossed (which it read at the end of the file;
those before stay, as the base kept them), and keeps the crossing waiting
in `rolled_back`, with the token where it was made. If an error comes
before the parser, reading the tokens the other way, reaches that token,
the crossing stands, as if never rolled back: a parameter's type
`Vec<Vec<…>>` past the limit, whose lists roll back one by one and leave
`Vec` before a `<` that the parameter list refuses, is one P0002 at its
deepest token, as before. Once the token is read, the crossing goes: the
comparison above parses at 127 parentheses and is P0002 at its own
deepest token at 128, and after two casts the attempt is two levels
deeper, so 126 parse. Where the base's own attempt reported an error past
the limit, as with `x as Int < (((1)))` at 127 parentheses (no type is
`1`), that error goes with the crossing; where the other reading fails
before the token, as `x as Int < (*mut (((a))))` does at `mut`, which no
expression has, P0002 stands where the base reported P0001. The trait
body's lookahead, the other rollback, re-reads its tokens only after a
`{`, which a crossing hides (`peek` reads the end of the file), and at the
same depth; the `component` and `view` lookaheads read tokens, not
nesting. `depth.rs` and two cases hold each shape.

**Measured.** Over the repository's tracked `.nova` files, N's new
sources and the cases (588 files), but for the three cases written to sit
at or past the limit (`chains_and_casts_at_the_limit.nova` and the two of
the rollback; the review's `optional_types_at_and_past_the_limit.nova`
came later, and no other file holds a type with two `?`s, so counting
them changes no figure here), the deepest is 25 levels
(`examples/07-fullstack-blog/src/server.nova`, whose router is one chain
of 21 calls, `Server::new()` and 20 method calls, 41 postfix operators,
the repository's longest); the next are 18 (`selfhost/src/lexer.nova`)
and 14 (`tests/dom/ui_show_for_each.nova`, `tests/dom/timers.nova`), and
N's parser nests at most 13 (`parse_pattern.nova`). Each is counted by
building N with `MAX_DEPTH` lowered until the file meets P0002. The
longest binary chain is 9 operators and the longest `else if` chain 14,
both in `lexer.nova`. No file comes near 128.

**Probed at the limit,** each by S0 and N, the dumps compared: 128 and
129 levels of parentheses, blocks, `if`s, `else if`s, match arms,
closures' bodies, interpolation holes (nested, and holding parentheses),
calls, indexes, arrays, tuples, prefix operators, binary chains,
assignments, ranges, casts, a cast's generic type, generic arguments in a
parameter's and a `let`'s type, tuple, function and optional types,
tuple-struct, tuple, record and or-patterns, items in blocks, `while`,
`for`, `return`, record literals, views, and chains of fields, indexes,
`?`, `.await`, tuple fields, calls and method calls, and a type's `?`s
(since the review): every one parses at
128 and is one P0002, the file's last line, at 129; `nova check` runs
every stage on each at 128 without overflowing, and `nova fmt` on each at
129 reports the one P0002 and leaves the file as it is. Uncounted, a
chain of about 5,000 method calls overflowed `nova check`, `run` and
`build` on the 64 MB thread (4,000 passed), and the base binary on its
8 MB main thread passed 500 and aborted at 600; counted, a chain of
100,000 is P0002.
Wide inputs need no count: 20,000 `let`s, statements, array elements,
interpolations, match arms, items or tuple elements pass `nova check`.

### 3. S0's pipeline on a 64 MB thread

`nova`'s `main` parses its arguments, sets up logging and the embedded
runtime, then runs the command on a thread with
`nova_driver::PIPELINE_STACK`, 64 MB, named `main`:

- the command's result returns to the main thread, which returns it as
  before (an error prints `Error: …` and exits 1);
- a panic prints on that thread, `thread 'main' … panicked at`, as on the
  main thread before, and resumes on the main thread with
  `resume_unwind`, which unwinds out of `main` without printing again:
  exit code 101;
- `std::process::exit` (`fmt`, `lsp`, `parse --dump-*`) ends the process
  from either thread, flushing standard output as before; a stack
  overflow aborts it from either, with its message and exit code 134;
- stdin and stdout are the process's; no command installs a signal
  handler, so Ctrl-C ends `nova` as before. Probed against the base
  binary: every command's exit code and both streams on success and
  failure; `nova run` of a program that reads stdin, exits with a code,
  panics, or recurses past its stack (the same depth overflows both,
  between 520,000 and 550,000 frames on 8 MB); `nova test` with a passing,
  failing, panicking, exiting and overflowing test; `nova dev`, `nova run`
  and `nova lsp` under SIGINT and SIGTERM: all as before;
- **`nova run` compiles on that thread and runs the program on the main
  thread.** The thread hands back the JIT's `CompiledProgram`, wrapped in
  `cmd::run::Compiled` with an `unsafe impl Send`. Rust cannot prove it
  `Send` because the JIT module holds raw pointers; it is sound because
  the module owns the code and data those pointers name (mapped memory,
  not thread-bound), the entry point is a pointer into that memory, the
  thread that compiled it has ended, so it is used by one thread at a
  time, and the runtime's per-thread state (the heap, the executor, the
  file and socket tables) is made on first use by the thread that runs
  the program: compiling calls nothing of the runtime but its symbol
  table, and `set_args`, which the main thread calls, is process-wide. So
  a program has the same stack under `nova run` as before, and as when
  built: the platform's main thread's. Running it on the 64 MB thread
  would have let a deep recursion pass under `nova run` and crash when
  built; `nova_run_runs_the_program_on_the_main_thread` holds the main
  thread at 8 MB and checks that a recursion 2,000,000 deep overflows it
  where 100,000 runs.

The language server's checker thread, and `nova dev`'s rebuild thread,
get the same stack; its protocol thread is the command's.

The thread bounds what the parser's recursion needs, and the later
stages' recursion over the tree, since the depth limit bounds both: every
chain that makes the tree deeper is counted toward it (binary, postfix,
and a type's `?`s, §2). It does not bound every overflow: `logos`'s per-digit
recursion (ADR 0051) is not a level, so a decimal literal of 210,000
digits or more aborts debug S0's 64 MB thread (measured with `nova parse
--dump-tokens` and `--dump-ast`: 210,000, 250,000 and 300,000 digits abort
in about 50 ms, in the first scan of the literal, one frame a digit; a
release build's smaller frames take more). Shorter literals do not
overflow, but are slow: the literal fails as a number, and each digit is
then scanned again with the digits after it, every scan shallower than
the first, so the time is quadratic (20,000 digits take 7.6 s, 40,000
take 42 s); 207,500 digits had not aborted after 60 s. No generator
makes such a number. `nova test` runs built test binaries, whose stack
is the platform's, as before.

### 4. The error cases

The spec's ~160 inline sources became 262 files in
`crates/nova-cli/tests/selfhost/cases/`. They were not copied by hand: the
five test binaries (`lexer_tests`, `keywords`, `parser_tests`,
`newline_rule`, `tuples`) ran with `lex` and `lex_with_comments`
temporarily recording each source and the name of the test thread that
lexed it, so a source a helper builds (`fn f() {\n…\n}`) is recorded as it
is parsed. Then: the property tests' random inputs were skipped; the test
that lexes every `.nova` file of the tree was dropped; a source equal to a
tracked file (the parser's six fixtures, already corpus files) was
dropped; duplicates were kept once, under the first test in sorted order;
the 32 keywords became one file, a line each. A file is named `<test
file>__<test>[_<n>].nova`. The six sources holding a `\r` or another
control character, which a checked-out text file cannot keep, joined the
differential's edge inputs (`fuzz::edges`), as did three decimal numbers
of 4,000 characters (ADR 0051's known bug). One more case,
`every_other_node.nova`, holds the twenty kinds the base's corpus does not
(twelve that no test's source holds either, and a module, an item in a
block, an array pattern, a type alias and the splits before `[`, `-` and a
value) and the three values no corpus file holds (a record pattern
without `..`, a pointer type without `mut`, a trait's `async`
signature), so that the corpus and that file alone name every form. A last,
`chains_and_casts_at_the_limit.nova`, holds a chain of each binary
precedence and a postfix chain, each followed by an operator of the next
lower one whose operand is 100 parentheses deep, a view whose first
child is a postfix chain and whose second's argument is 100 deep, and 128
casts: a parser that does not lower the depth after a chain, or counts a
cast's type one level deeper, reports P0002 there (the mutants found that
no input did). Another, `optional_types_at_and_past_the_limit.nova` (from
5.1b's review), holds a field's type with 129 `?`s, two casts whose types
hold 100 `?`s each, and a parameter's type with 130: a parser that does
not count a type's `?`s, or does not lower the depth when the type ends,
differs from the other there. Both lexers agree on all of them.

### 5. The deep generator

`fuzz::deep(rng, kind, depth)` writes a program whose deepest level is
`depth`, nested as one of 15 kinds (parentheses, blocks, `if`, `else if`
chains, `match`, `while`, `for`, calls, prefix operators, binary chains,
chains of method calls, types, optional types `T?`, patterns, items in
blocks) or mixed, eight
wrappers of one level each in a random order. Up to the limit each is well
typed (but nested items and `T?`, which the checker refuses as E0900), so `nova
check` runs every stage on it. `fuzz::deeps(seed, mixed)` gives each kind
at 126 to 130 levels and `mixed` inputs at depths in that range: seeds 201
and 202, 20 mixed each, 190 inputs, through the token and AST
differentials (§7). S0's `parse --check` gives no error up to 128 and
exactly one P0002 past it on all of seed 201's, and `check` the same at
128 and 129, but for the nested items' and `T?`'s E0900; neither overflows.

### 6. N's AST and parser

`selfhost/src/` gains nine modules:

| Module | What it ports | Code lines | Rust's |
|---|---|---:|---:|
| `ast.nova` | `nova-ast`'s `lib`, `expr`, `item`, `pattern`, `ty` | 245 | 477 |
| `parser.nova` | `grammar.rs`'s state, token access, recovery, docs, `component` lookahead, paths and helpers; `error.rs`; `lib.rs` | 450 | |
| `parse_item.nova` | the file, attributes, items, functions, records … `extern` | 685 | |
| `parse_type.nova` | types, generic arguments, `>>`, `T?` | 171 | |
| `parse_stmt.nova` | blocks and statements | 78 | |
| `parse_expr.nova` | the precedence ladder, postfix, atoms, `if`/`match`, closures, record literals | 762 | |
| `parse_view.nova` | `view { … }` | 134 | |
| `parse_pattern.nova` | patterns | 259 | |
| the parser, together | `grammar.rs`, `error.rs`, `lib.rs` | 2,539 | 2,983 |
| `ast_dump.nova` | `nova-ast`'s `dump.rs` and `nova-parser`'s | 813 | 985 |
| `parser_tests.nova` | N's own tests (six) | 145 | – |

Code lines are counted as 5.1a counted them: no blank line, no line that
begins with `//` (so no doc comment), and nothing of a Rust file from its
first `#[cfg(test)]` on. The Rust parser was 2,836 at the spec's base and
is 2,983 after P0002, this ADR's changes and the review's (`grammar.rs`
2,909, `error.rs` 53, `lib.rs` 21), the AST 476 and 477; the Rust dump's
985 are `nova-ast/src/dump.rs`'s 958, `KINDS` included, which N has no
use for, and `nova-parser/src/dump.rs`'s 27. `token.nova` grows
from 196 code lines to 303 (`Token.kind`, `Span::point`, `Span.merge`,
`Spanned::new`), and `lexer.nova` from 793 to 827 (`KEYWORDS`).

The modules follow `grammar.rs`'s areas, as the spec planned; Nova's flat
modules allowed it as it is, since an `impl Parser` may sit in any module
that imports `parser`, and the modules may import one another in a cycle.
Each Rust function has a Nova function of the same name and shape, in the
same order, with its comments; the one function added is `value_at` (the
token at an index, or `Eof` past the end, where the Rust reads
`self.tokens.get(i)` and every caller asks for a token other than
`Eof`). Every message, context string and note is the Rust's word for
word; so are the generic-arguments rollback that keeps its errors and
holds back a crossing (§2), `>>`
splitting through `pending_gt`, the statement recovery that ignores line
breaks, the trailing expression with or without its `;`, the
`component` and `view` lookahead, the trait body's lookahead that does
truncate its errors, the newline rule's splits, the halt after a bare
`return` or `break`, the note on the arm after a split, and the depth
counter, entry for entry. Not ported: the names of dropped items, which
only the language server reads (spec §4.2).

**The tree.** A struct is a record with the same fields in the same order,
an enum a sum with the same variants in the same order, a variant with
named fields holding them positionally (a comment names them). Spans are
`token.nova`'s `Span`, two `Int`s with no `FileId`. A sum is recursive
without a box, so `Box<Spanned<Expr>>` is `Spanned<Expr>` and
`Option<Box<…>>` is `Option<…>`. Two choices the spec did not make:

- **A variant is named `EnumVariant`** (`Expr::If` is `ExprIf`,
  `Pattern::Lit` is `PatternLit`, `BinOp::Add` is `BinOpAdd`). A variant's
  name is a value of its module, one module may hold only one of a name
  (E0002), and a module may not import two (E0002 again); the AST's Rust
  names repeat across its enums (`Lit`, `Path`, `Tuple`, `Array`, `Range`,
  `Ref`) and with the tokens' (`If`, `Match`, `Fn`, `Type`, `Pub`, `Eq`,
  `Int`), which every parser module imports. The dump's `Enum.Variant` is
  the name with a dot.
- **`Spanned` is generic**, `Spanned<T>`, as nova-diagnostics' is: 5.1a's
  `Spanned` held a `Token` alone, and the tree needs it for nodes and
  names (`Spanned<Expr>`, `Spanned<String>`); the lexer's 17 uses became
  `Spanned<Token>`.

**Each Rust idiom's translation**, one way everywhere:

| Rust | Nova |
|---|---|
| `std::mem::discriminant(a) == std::mem::discriminant(b)` in `check`, `eat`, `expect`, `next_after_docs_is` | `a.kind() == b.kind()`: `Token.kind()` numbers the 92 variants in declaration order; every kind the parser asks for has no payload |
| `match self.peek().clone()`, `if let Token::X(v) = …` | a `match` on the token `peek` returns: a token is a reference, so nothing is copied |
| `self.in_stmt_ctx(on, \|p\| …)`, `in_no_struct_ctx`, `in_head_ctx`, `in_struct_ok_ctx` (21 uses) | the flag saved by `enter_stmt_ctx(on)` / `enter_no_struct_ctx(on)`, which set it and return the old value, the call, then the flag restored; a `?` comes after the restore |
| `self.nested(\|p\| …)` | the depth saved, `deeper()`, the call, the depth restored; a closure with more than one call in it is a function `…_here` (`parse_type_here`, `parse_pattern_here`, `parse_view_children_here`), and S0's are the same functions, so both read alike |
| `static HALT: Token = Token::Eof` | the field `eof`, an `Eof` |
| `&'a Tokens` | the field `tokens` |
| `halt: Option<usize>` | `halt: Int`, -1 for none |
| `self.tokens.get(self.pos.saturating_sub(1)).map(\|s\| s.span).unwrap_or(start)` (36 copies) | `self.prev_span_or(start)`, the Rust's own helper |
| `matches!(t, A \| B)` | a `match` with an or-pattern |
| `errors.truncate(n)`; `errors.remove(i)` then `insert(i, …)` | `pop` to `n`; `set(i, …)` |
| `splits.sort_by_key(\|s\| s.at.start)` | a stable insertion sort (`sorted_by_start`) |
| `let mut f = …` bound by a pattern | `let mut f = f`: a pattern's bindings are immutable |
| `nova_lexer::KEYWORDS`, `trim_matches('`')` | `lexer.nova`'s `KEYWORDS`, `trim_backticks` |

`peek`, `peek_span`, `advance` and `prev_span_or` index the tokens after a
bounds check rather than through `Vec.get`, whose `Some` a Nova program
allocates. Three functions are named apart from the Rust's because their
names are Nova keywords: the dump's `record`, `import` and `module` are
`record_decl`, `import_decl` and `module_decl`, and a local `record` is
`rec`.

**The command line.** `selfhost ast FILE…` prints, for each file, `== FILE`
and its AST dump, `IOERR` and the kind for a file it cannot read and exit
code 2 after the rest, and refuses no file with exit code 2, as `selfhost
tokens` does (ADR 0051). `ast_dump.nova` writes the grammar of §1 byte for
byte, through `dump.nova`'s `push_escaped` and `hex16`.

**N's own tests.** Six `@test`s in `parser_tests.nova`, in `src/` as
5.1a's are **(differs from the spec**, which puts them in
`selfhost/tests/`, where a module cannot reach the parser; ADR 0051): the generic-arguments rollback and `>>`, statement
recovery, the trailing expression, `component` and `view`, a bare
`return` and the arm note, and 128 against 129 parentheses. Each expected
dump is S0's. `nova test` in `selfhost/` runs 17.

**Differences found while building.** None: on its first build N's dump
equalled S0's on every tracked file and case, and no input of the fixed
seeds or of the longer searches below has shown a difference. So no case
was added.

### 7. The comparison

`selfhost.rs`'s `agree` runs every batch through `nova parse --dump-ast`
and `selfhost ast` after the token dumps, so the AST dumps, errors and
notes included, are compared byte for byte on: the corpus (every tracked
`.nova` file outside `selfhost/`, then N's sources) and its Windows
checkout; the 279 cases (5.1a's 12, the 262 recorded in §4,
`every_other_node.nova`, `chains_and_casts_at_the_limit.nova`,
`optional_types_at_and_past_the_limit.nova` and the two of the rollback,
§2); the
edge inputs; the soup of seed 1 (2,000 inputs; seeds 2 and 3 go through
the token dump alone); the 750 mutated corpus files of seeds 101–103; and
the 190 deep inputs of seeds 201 and 202. The every-kind test reads N's
dumps of the corpus and the cases as well as S0's, and each holds all 167
forms of `KINDS`. A mismatch's report names the source line of an AST line
too (a node's start, or a `PARSE` line's). A command with no file is
refused by both sides, with nothing printed and exit code 2, for either
dump. The corpus test counts one tree per file in S0's output, the soup's
and the mutations' find trees in it, and the deep inputs' find P0002, so a
pass that skips the AST dump fails.

**Time.** On a 4-core Linux machine, debug build, `selfhost.rs` ran in
about 10 s with the token dump alone; with the AST dump, the soup's pass
cut to one seed of three, it takes 21.5–23.4 s (about 50 s of CPU), three
runs, with the postfix kind, the hand case and the AST pass's assertions
that step 0 added; all the soup through both would add about 1 s of CPU.
In 5.1a CI took 1.4 to 2.1 times as long on macOS as here, so about 35 to
50 s there, under the minute past which the spec asks the user. N's AST
dump is the cost: with the debug runtime it parses the corpus in about 3 s
where S0's debug build takes 0.4 s. The soup is made for the lexer and is
mostly parse errors, so its AST pass was the one cut; the mutations, which
reach far into the parser, all go through.

**Longer searches**, locally: `NOVA_SELFHOST_FUZZ=7:6000` and
`11:20000`, 26,000 inputs of soup and 26,000 mutated files through both
dumps, showed no difference.

### 8. N's own stack at the limit

A Nova program runs on its platform's main thread (spec §4.4), so N, built
by `nova build` (Cranelift), was run on the deep generator's 128-level
input of every kind (and 126, 127, 129, 130) with `ulimit -s 8192` and
`ulimit -s 1024`, which stands in for Windows' main thread:

| Kind | 8 MB | 1 MB | Least stack that passed at 128 (of 512, 256, 128, 64, 32 KB) |
|---|---|---|---|
| parentheses, blocks, `if`, `match`, `while`, `for`, calls | passes | passes | 256 KB |
| items in blocks | passes | passes | 128 KB |
| `else if`, prefix operators, binary chains, method calls, types, optional types, patterns | passes | passes | 64 KB |

So N has at least four times the stack it needs at 128 levels on 1 MB,
measured on Linux, and past the limit it stops at 129 like S0. Windows
x64 frames may be 1.5 to 2 times larger (the callee's shadow space, saved
`xmm` registers), which would leave twice the stack needed; CI's Windows
run of the test below, on that system's real 1 MB main thread, is the
check. The limit stays 128 and the
link is unchanged. `n_parses_128_levels_of_every_kind_on_a_1_mb_main_thread`
runs every kind at 128 and 129 through N on 8 MB and 1 MB on Linux and
macOS, and on Windows' own 1 MB main thread, and checks that N exits 0,
with no P0002 at 128 and one at 129.

### 9. Speed

Measured locally on 4 cores (spec §4.7), best of three, peak memory by
`wait4`'s `ru_maxrss` (`/usr/bin/time` is not installed here). N is
`selfhost ast`, built by `nova build` with `NOVA_RUNTIME_LIB` pointing at
a release `libnova_runtime.a` built in an isolated target directory, and
with the debug runtime. A release `nova` would have taken the disk below
the 6.3 GB this work keeps free, so S0 was measured two ways **(differs
from the spec**, which times S0 built in release mode): its debug
`nova parse` (the `nova` binary), and its release lexer, parser and dump
(`nova_parser::dump_ast`, or `lex` and `parse` alone for `--check`)
built into a temporary example of `nova-parser` that reads the files and
prints the same dump on a 64 MB thread, not kept. Inputs: all of `std/`
(25 files, 697 KB) concatenated, the 25 files given to one process, and
the 25 files one process each.

| | std concatenated | | 25 files, one process | | 25 processes | |
|---|---:|---:|---:|---:|---:|---:|
| N, release runtime | 0.244 s | 46 MB | 0.149 s | 12 MB | 0.253 s | 11 MB |
| N, debug runtime | 1.21 s | 47 MB | 0.89 s | 13 MB | 1.09 s | 12 MB |
| S0 release, dump | 0.040 s | 18 MB | 0.027 s | 9 MB | 0.109 s | 9 MB |
| S0 release, lex and parse | 0.030 s | 14 MB | 0.022 s | 9 MB | 0.101 s | 9 MB |
| S0 debug, `parse --dump-ast` | 0.117 s | 29 MB | 0.108 s | 16 MB | 0.261 s | 16 MB |
| S0 debug, `parse --check` | 0.081 s | 26 MB | – | – | 0.227 s | 16 MB |

**The factor, N over S0, lexing, parsing and dumping, with the release
runtime: 6.1× release S0 on std concatenated, 5.5× on the files in one
process** (2.1× and 1.4× S0's debug build); with the debug runtime, 30×
and 33× release S0. That is under the 10× past which the user is asked
(user decision 6). N's peak memory is 2.6× release S0's on the one large
file. N with the debug runtime is what CI runs.

### 10. The grammar changes in both compilers

From 5.1b on the grammar exists twice, as the tokens have since 5.1a
(ADR 0051). A change to the parser or the tree, in what it accepts, what
it builds, what it reports or where, changes `nova-parser`/`nova-ast` and
`selfhost/` in one pull request, the dump on both sides with them; the
differential fails the change that touches one alone, and an input that
once showed a difference becomes a case under `cases/`.

## Consequences

- A program nested past 128 levels no longer compiles: P0002, where
  debug S0 aborted from 240 nested parentheses, 200 nested `if`s, 800
  operators or 600 method calls in a chain. Nothing in the repository but
  the cases written to test the limit is past 25.
- P0002 drops the whole item in progress, however much of it parsed, and
  everything after it in the file: one error for a file nested too deeply,
  and nothing else from it, the items before it kept.
- A `nova` panic and its exit code are as they were; a test runs one
  (`println!` into `/dev/full`, on Linux).
- Each command costs one thread and 64 MB of address space, committed
  only as used.
- `nova run`'s program and the compiler are on different threads, which
  needs one `unsafe impl Send`. If the JIT ever keeps thread-bound state,
  that impl is the place to look.
- A postfix chain counts too, by the user's choice of 2026-10-31, which
  the spec does not list (§2): each postfix operator after a chain's
  first is a level, a call directly after any `.` postfix one with it (a
  method call, `.await(…)`, `.0(…)`), and a call's arguments are a level
  deeper than the call; so is each `?` of a type `T?` after the first, by
  the same principle, 130 of them P0002. After a name, a chain of 130 field accesses, or
  of 130 method calls without arguments, is P0002, and of 129 with
  arguments. A router built as one chain from `Router::new()`, one
  `.get("/", h)` a route, parses 127 routes and is P0002 at the 128th;
  the longest chain in the repository is the router's in
  `examples/07-fullstack-blog/src/server.nova`, 21 calls.
- The error cases are 262 small files, regenerated by the recording rather
  than kept in step with the tests by hand; a new test's source joins
  them only when the recording is run again.
- Every grammar change is made twice (§10), and checked on every system by
  a harness that now takes about twice the CPU it took in 5.1a.
- The AST's variants carry their enum's name in N and not in S0, so a
  reviewer reads `ExprIf` against `Expr::If`.
- N is about 6× slower than release S0 with a release runtime and 30×
  with the debug one; the debug runtime is what the differential uses, so
  its time grows with the inputs N parses.

## References

- `docs/superpowers/specs/2026-10-30-phase-5-1-the-lexer-the-parser-and-the-ast-design.md`
  §§1, 2, 4–9.
- ADR 0051 (the token dump, the harness, and S0's known stack bug).
- `nova-spec/11-PARSER.md` §§4 and 5, and `12-TYPESYSTEM.md` §8, their
  notes of 2026-10-31.
- `crates/nova-ast/src/dump.rs`; `crates/nova-parser/src/{grammar,dump,error,lib}.rs`
  and `tests/depth.rs`; `crates/nova-cli/src/{main.rs,cmd/parse.rs,cmd/run.rs}`;
  `crates/nova-cli/tests/selfhost.rs` and `tests/selfhost/`.
- `selfhost/src/{ast,parser,parse_item,parse_type,parse_stmt,parse_expr,parse_view,parse_pattern,ast_dump,parser_tests}.nova`.
