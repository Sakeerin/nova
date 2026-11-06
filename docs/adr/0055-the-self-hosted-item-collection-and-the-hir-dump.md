# ADR 0055 — The self-hosted item collection, and the HIR dump

## Status

Proposed (2026-11-06), being written: the 5.3a prototype's part A (S0's
side) wrote §1 to §6; part B (N's side, the comparison, the figures)
finishes it. Branch `proto/5-3a-whole`
(`docs/superpowers/specs/2026-11-06-phase-5-3-the-type-checker-and-hir-design.md`
§3). §1 is S0's item stage as a function; §2 the HIR dump's grammar, with
one example of every form; §3 the hidden flag; §4 std's item tables
shared, its fallback and its guard; §5 the cases; §6 the type fuzzer's
item mutations; §7 the decisions part A took. Part B adds N's code and
idioms, the differential on both sides, CI's figures, N's stack and
memory, and the rule that a change to item collection changes both
compilers. 5.3b (the bodies, the full dump) writes ADR 0056.

## Decision

### 1. `check_items`: S0's item stage, as a function

`nova_typeck::check_items(file, defs, options)` runs passes 1 to 5 of
`check_with` and returns `ItemTables`: the records, sums, traits, impls,
signatures and externs, the tables the bodies read (receivers, method
places, impl selves, arities, supertraits, the impls' spans and owners),
and the item diagnostics in the order the passes emitted them.
`check_with` calls it first and restores its checker from the tables: a
pure refactor, `nova check` over every base-tracked `.nova` file printing
what the base prints (1,078 checks, `checkcmp.sh`), and a unit test that
`check_items`' diagnostics are the prefix of `check_with`'s and its
tables the module's.

**Passes 1 to 5, and 2 to 5.** The spec says both (§3.1). They are one
boundary seen from two sides: pass 1 is the language server's index
(`note_item_declarations`), which records nothing unless asked and gives
no diagnostic. S0's `check_items` keeps it, so that `check_with` calls one
function and its order is as it was; N ports passes 2 to 5 (decision 8),
and the dump, which asks for no index, shows passes 2 to 5 alone.

### 2. The HIR dump

`nova_typeck::dump::dump_program` writes a program's part,
`nova_typeck::dump::dump_std` std's part; `nova_driver::HirDumps` runs the
resolution dump's pipeline for each root (ADR 0053 §1, ADR 0054 §1), then,
when no error stopped it, the item passes with `check`'s options (the
sources, no probe, no index), and dumps their output as returned, before
the driver's `keep_the_entry_main`. `nova_driver::dump_std_hir` checks a
program of no module of its own and dumps std's part.

**Conventions:** the resolution dump's (ADR 0053 §1): two spaces a depth,
`(Kind`, values, fields `key=value`, children a level deeper, `)` at the
end of the last line; names, symbols and messages quoted as the AST dump
quotes strings; a place is a file's label quoted, `:` and a span.

**Ids:** the resolution dump's: `u<n>` for a program's own definition,
`s<n>` for std's, its `DefId` less the first std definition's. An impl has
no `DefId`; it is numbered from 0 within its part, in the table's order.

**Types,** by structure, with ids, not names (spec §3.3):

```
ty = "Int" | "Float" | "Bool" | "Char" | "String" | "Bytes" | "()"
   | "fn(" [ty {", " ty}] ") -> " ty
   | "Future<" ty ">" | "[" ty "]"
   | "(" ty "," ")" | "(" ty ", " ty {", " ty} ")"     -- a tuple
   | id ["<" ty {", " ty} ">"]                           -- a record or a sum
   | "T" uint                                            -- a type parameter
   | ty "::" id                                          -- a projection
   | "?" uint | "!" | "{error}"                          -- a variable, never, error
```

A record and a sum print alike, as their id: which one it is, the
resolution dump's `(Def.Record` or `(Def.Sum` says, and `convert_ty`
decides it from the definition's kind alone.

**The grammar** (5.3a's part; `I(d)` is `d` levels of indentation):

```
part      = "IOERR " kind "\n" | program diagnostics
program   = "(Hir target=" target [" mode=test"] " checked=" bool
            [ "\n" I(1) "(StdStart module=" uint " def=" uint " item=" uint ")"
              entries ] ")\n"
std       = "(HirStd target=" target [" mode=test"] " records=" uint " sums=" uint
            " traits=" uint " impls=" uint " sigs=" uint " externs=" uint
            entries ")\n" diagnostics
entries   = { record } { sum } { trait } { impl } { sig } { extern }
            -- 5.3b adds every function here, after the externs
record    = I(1) "(Record " id " " string " generics=" uint { I(2) "(Field " string " " ty ")" } ")"
sum       = I(1) "(Sum " id " " string " generics=" uint
            { I(2) "(Variant " string { " " ty } ")" } ")"
trait     = I(1) "(Trait " id " " string { I(2) "(Supertrait " id ")" }
            { I(2) "(AssocType " string " " id ")" } { method } ")"
method    = I(2) "(Method " string " has_self=" bool " mut_self=" bool " generics=" uint
            " default=" (id | "None") bounds(3) signature(3) ")"
impl      = I(1) "(Impl " uint " trait=" (id | "None") " head=" head " generics=" uint " " place
            I(2) "(SelfType " ty ")" bounds(2)
            { I(2) "(Method " string " " id ")" } { I(2) "(AssocBinding " id " " ty ")" } ")"
head      = "Int" | "Float" | "Bool" | "Char" | "String" | "Bytes" | "Array" | id
sig       = I(1) "(Sig " id " generics=" uint [" selfless=" bool " mut_self=" bool]
            bounds(2) signature(2) ")"     -- the two flags on an impl's method only
extern    = I(1) "(Extern " id " " string " abi=" ("C" | "Js") " " place signature(2) ")"
bounds(d) = { I(d) "(Bound " uint { " " id } ")" }    -- every generic parameter's, empty or not
signature(d) = { I(d) "(Param " ty ")" } I(d) "(Ret " ty ")"
diagnostics = -- the resolution dump's (ADR 0053 §1), every diagnostic so far
```

Tables print in the order the type checker collected them: records, then
sums, traits and impls in `DefId` or item order, each with the program's
before std's; signatures, a map, sorted by `DefId`: functions, methods
(an impl's and a trait's default), constants (no parameters, the declared
type as `Ret`) and externs. A method's signature holds its receiver's type
as its first parameter when it has one (`selfless=false`), as `FnSig`
does; a trait's method, which has no `FnSig`, holds no receiver, as
`TraitMethod` does, and its bounds exclude `Self`'s. A program that stops
before the type checker prints `checked=false` and the diagnostics so far;
its part repeats only `(StdStart …)` and those of the resolution dump.

**One example of every form**, S0's dump of the source after it:

```
;; nova check --dump-hir shapes.nova
;; shapes.nova: record Pair<T> { a: T, b: Int }\ntype Shape = | Dot | Line(Int, Float)\n
;;   trait Area: Eq {\n    type Unit\n    fn area(self) -> Self::Unit\n
;;   fn scale<U: Ord>(mut self, by: U) -> Bool { true }\n}\n
;;   impl<T: Eq> Area for Pair<T> {\n    type Unit = [T]\n    fn area(self) -> [T] { [] }\n}\n
;;   impl Eq for Pair<Int> {\n    fn eq(self, o: Pair<Int>) -> Bool { true }\n}\n
;;   impl Shape {\n    fn origin() -> Shape { Dot }\n}\n
;;   extern "C" {\n    fn abs(x: Int) -> Int\n}\nconst LIMIT: Int = 3\n
;;   async fn fetch(t: (Int, String)) -> Option<Int> { None }\n
(Hir target=native checked=true
  (StdStart module=1 def=11 item=9)
  (Record u0 "Pair" generics=1
    (Field "a" T0)
    (Field "b" Int))
  (Sum u1 "Shape" generics=0
    (Variant "Dot")
    (Variant "Line" Int Float))
  (Trait u2 "Area"
    (Supertrait s19)
    (AssocType "Unit" u3)
    (Method "area" has_self=true mut_self=false generics=0 default=None
      (Ret T0::u3))
    (Method "scale" has_self=true mut_self=true generics=1 default=u4
      (Bound 0 s21 s19)
      (Param T1)
      (Ret Bool)))
  (Impl 0 trait=u2 head=u0 generics=1 "shapes":211..218
    (SelfType u0<T0>)
    (Bound 0 s19)
    (Method "area" u5)
    (AssocBinding u3 [T0]))
  (Impl 1 trait=s19 head=u0 generics=0 "shapes":287..296
    (SelfType u0<Int>)
    (Method "eq" u6))
  (Impl 2 trait=None head=u1 generics=0 "shapes":353..358
    (SelfType u1)
    (Method "origin" u7))
  (Sig u4 generics=2
    (Bound 0 u2 s19)
    (Bound 1 s21 s19)
    (Param T0)
    (Param T1)
    (Ret Bool))
  (Sig u5 generics=1 selfless=false mut_self=false
    (Bound 0 s19)
    (Param u0<T0>)
    (Ret [T0]))
  (Sig u6 generics=0 selfless=false mut_self=false
    (Param u0<Int>)
    (Param u0<Int>)
    (Ret Bool))
  (Sig u7 generics=0 selfless=true mut_self=false
    (Ret u1))
  (Sig u8 generics=0
    (Param Int)
    (Ret Int))
  (Sig u9 generics=0
    (Ret Int))
  (Sig u10 generics=0
    (Param (Int, String))
    (Ret Future<s0<Int>>))
  (Extern u8 "abs" abi=C "shapes":416..419
    (Param Int)
    (Ret Int)))
(Diagnostics
  (Error "E0072" "the trait `Area` requires `Eq`, which `Pair<T0>` does not implement"
    (Primary "shapes":211..218 "here")))

;; A note and a fix, after the program's part:
;; nova check --dump-hir errs.nova
;; errs.nova: impl Eq for [Int] {}\nfn f(x: Strin) {}\n
(Diagnostics
  (Error "E0010" "a trait cannot be implemented for an array"
    (Primary "errs":12..17 "an array type")
    (Note "an array takes inherent impls only, `impl<T> [T] { … }` or `impl [Int] { … }`; wrap the array in a record to implement a trait for it"))
  (Error "E0001" "cannot find type `Strin`"
    (Primary "errs":29..34 "here")
    (Fix "change `Strin` to `Stdin`"
      (Edit "errs":29..34 "Stdin"))))

;; checked=false: the resolver's E0002 stops the program:
;; c.nova: fn f(x: Nope) {}\nfn f() {}\n
(Hir target=native checked=false)
(Diagnostics
  (Error "E0002" "duplicate definition of `f`"
    (Primary "c":20..21 "redefined here")
    (Secondary "c":3..4 "first defined here")))

;; std's part, and abi=Js, from nova check --dump-hir --std --target wasm:
(HirStd target=wasm records=49 sums=10 traits=10 impls=82 sigs=711 externs=61
  (Extern s307 "dom_body" abi=Js "<std/dom>":922..930
    (Ret Int)))
(Diagnostics)
```

`nova_typeck::dump::KINDS` lists every form, `LATER_KINDS` the two
severities no stage gives; the harness checks that std, the corpus and
the cases print every listed form and nothing unlisted. **Sizes:** std's
native part is 3,107 lines (79,476 bytes), its WASM part 3,436 (83,762),
its test-mode part 3,121 (79,737): 63 records, 14 sums, 12 traits, 95
impls and 623 signatures natively.

### 3. The hidden flag

Taken only when `--dump-hir` is the first argument after `check`, by the
resolution dump's own parser (ADR 0053 §1), generalised to either flag
(`DumpCmd`): the same arguments (`--std`, `--target native|wasm`,
`--test`, files, each option once), the same refusals in clap's words
under the usage `nova check --dump-hir`, and the same three errors of its
own, naming `--dump-hir`. Every other command line, `--help` and argv[0]
included, is the base's, byte for byte (`helpcmp.sh`, `usagecmp.sh` with
`--dump-hir`'s shapes, `argv0.py`). Both dumps' outputs are settled in the
harness's `run`, so E0004's and E0093's Windows paths compare alike
(ADR 0054 §7).

### 4. std's item tables, shared

`nova_typeck::SharedItems` holds std's `ItemTables`, built once per
process and mode from the check of a program of no module of its own
(`SharedStd::alone`, which 5.2b's shared resolution now keeps whole).
Each program is resolved by `resolve_program_with_shared_std_for_check`:
5.2b's shared path, then std's definitions, module scopes and items'
modules appended, rebased past the program's (`Def::rebased`,
`ModuleScope::rebased`), so the type checker reads std's definitions as
the full resolution numbers them; std's items are not in the merged file.
`check_items_shared` runs each pass over the program's own definitions
(`Checker::own_defs`) and splices std's part of each table, rebased,
where the full check would have collected std's items: at the end of each
pass's loop, after the program's (`Splice`), so tables keep the full
check's order and no pass sees a std entry the full check had not yet
collected. The impl checks (signatures, coherence, hidden methods,
supertrait impls) then run over the program's impls and std's together,
as the full check runs them. A program never writes the shared state.

**The fallback:** should std's items give a diagnostic, or 5.2b's shared
resolution not apply (std's diagnostic or `@test`), every program is
resolved and checked in full. A unit test builds std with one more module
and checks that the shared path gives a program what the full check gives
it (E0074 and E0105 against std's impls among it) and that a std whose
items give a diagnostic is not shareable.

**The guard:** the driver's test
`std_checked_once_dumps_what_std_checked_for_each_program_dumps` compares
the shared and unshared HIR dumps byte for byte over every corpus root and
every case (`resolve*` and `check*`) of each mode, in order and reversed;
another checks that std's entries of a two-module program's check print as
std's part. Only the dump shares: `nova check`, `build`, `run`, `test` and
the language server keep `check_with` on the full resolution.

### 5. The cases

`cases/check/` (under `check`), `cases/check-wasm/` (the WASM target) and
`cases/check-test/` (test mode), laid out and named as the resolver's
(ADR 0054 §2): the directory names the mode. The item-level tests'
sources come from a temporary instrumentation of the type checker's test
helpers (each source a test lexes, written under the test's name; a
script, not committed). A test is item-level when one of its sources
gives an item diagnostic (the HIR dump's diagnostics beyond the
resolution dump's) or its body reads the item tables: 114 of the 550
tests that lex a source, whose 158 sources give 154 cases (one dropped as
a 5.2 case's text, three as repeats), the one two-module test a directory
rooted at `main.nova`. The other 436 tests are body-level and wait for
5.3b, which re-runs the same instrumentation and rule. Hand cases (names
without `__`): every item E0001 site, every E0012 form, the unsupported
types and an extern's `where`, the array `len` and inherent overlaps, the
import fix (both edits) and did-you-mean; every item E0092 site and a
component on WASM; tests beside items and an item error under test mode.
No case needs `fn main`. The harness runs them again with `\r\n`.

### 6. The type fuzzer's item mutations

`fuzz::type_mutate`: a loose corpus program that imports nothing and whose
HIR dump reaches the type checker with no diagnostic (S0 picks them, the
same on every system), mutated one to three times: a parameter's,
return's or field's type replaced (E0001, E0012, E0900), a generic list's
first parameter duplicated (E0403), a trait impl's method dropped (E0070),
added (E0071) or retyped (E0072), an impl duplicated (E0074), or one of
`fuzz::TYPE_ITEMS` appended, each aimed at a code: E0001, E0010, E0012,
E0015, E0070 to E0074, E0076 to E0078, E0105, E0403 and E0900. Seeds 401
to 403, 40 inputs each. Every input reaches the type checker, and each
aimed code occurs at least twice, which the harness asserts; on S0's dump
(part A): E0001 30, E0010 16, E0012 50, E0015 11, E0070 10, E0071 14,
E0072 21, E0073 18, E0074 23, E0076 5, E0077 20, E0078 19, E0105 4, E0403
17, E0900 46. E0092 is the WASM target's and occurs in the cases.

### 7. The decisions part A took

1. **`check_items` covers passes 1 to 5** (§1); N ports 2 to 5.
2. **The dump lives in `nova-typeck`** (`dump.rs`), since it reads the
   checker's private tables; it reuses the resolution dump's diagnostics
   printer, places and `StdStart` (`write_diagnostics`, `write_place`,
   `std_start`), made public for it.
3. **A record and a sum print as their id** (§2), as the spec writes them.
4. **An impl's place** is its self type's span, the one its diagnostics
   name; an impl's method's receiver flags are its `Sig`'s.
5. **Every generic parameter's bounds print, empty or not**, so a slot's
   count is in the dump.
6. **The sharing splices per pass** (§4) rather than seeding std's tables
   first, which would change the tables' order and what a pass sees.
7. **Item-level tests** are chosen by S0's dumps, a rule 5.3b can re-run.
