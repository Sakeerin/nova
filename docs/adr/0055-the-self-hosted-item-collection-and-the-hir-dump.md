# ADR 0055 — The self-hosted item collection, and the HIR dump

## Status

Accepted (2026-11-11). Branch `claude/loving-archimedes-frtf5q-53a`
(`docs/superpowers/specs/2026-11-06-phase-5-3-the-type-checker-and-hir-design.md`
§3), written in three parts: part A (S0's side) wrote §1 to §7, part B
(N's side, the comparison, the figures) §8 to §14. §1 is S0's item stage
as a function; §2 the HIR dump's grammar, with one example of every
form; §3 the hidden flag; §4 std's item tables shared, its fallback and
its guard; §5 the cases; §6 the type fuzzer's item mutations; §7 the
decisions part A took; §8 N's code and idioms; §9 the differential; §10
the decisions part B took; §11 the time, and CI's projection; §12 N's
stack; §13 N's memory; §14 the rule that a change to item collection
changes both compilers. Part C wrote §15, CI's figures and the user's
choice: `selfhost.rs` in a job of its own on each system, N on a release
runtime; and §12's release-runtime figures. The 5.3a plan's step 0,
before the plan split the work into tasks, wrote §9's count of 164
cases, the "Measured again" tables of §12 and §13, §12's account of the
harness's 1 MB tests, §15's figures of CI run 42774093193, and the
Consequences. 5.3b (the bodies, the full dump) writes ADR 0056.

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
;;       fn scale<U: Ord>(mut self, by: U) -> Bool { true }\n}\n
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
import fix (both edits) and did-you-mean, did-you-mean a primitive or a
type parameter, an import fix held back by a clash, none when two
modules export the name, no fix in a dependency, std's supertraits in
a program's bounds (the last five,
before the split, for mutants that passed); every item E0092 site and a
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
8. **A record prints no bounds:** a record's bounds are a resolution
   scope for its fields' projections, not a constraint, and `RecordType`
   keeps none; its fields' types show what they resolved to.
9. **`abi=Js` occurs only in std's WASM part:** a program's `extern "js"`
   is E0900, so no program part prints it.
10. **E0099's note is a body diagnostic,** 5.3b's: test mode changes no
    item diagnostic, and `cases/check-test/` holds tests beside items.

### 8. N's code

| File | Lines | Code | What it ports |
|---|---:|---:|---|
| `selfhost/src/hir.nova` | 585 | 455 | `nova-hir`'s `Ty` (18 variants), `TyHead`, the tables, `subst`, the queries, `match_pattern`, `match_args`, `normalize_ty` with its two limits, `self_types_overlap`; `ty_eq`, `head_eq` |
| `selfhost/src/typeck.nova` | 1,071 | 860 | the state (`Checker`), `check_items`, `share_items`, `check_items_shared` and the splices, pass 2, arities, supertraits, records, sums, `display_ty`, `display_ty_named`, the message helpers |
| `selfhost/src/typeck_traits.nova` | 1,094 | 1,032 | `collect_traits`, `collect_impls` and its six checks |
| `selfhost/src/typeck_ty.nova` | 511 | 455 | `convert_ty`, projections, `find_assoc_type`, `normalize`, bounds, `where`, signatures, `component_ret`, externs and FFI |
| `selfhost/src/fixes.nova` | 346 | 275 | `push_fix`, `name_fixes` for a type and a trait, the import fix and its edit, did-you-mean; lines.rs's placing |
| `selfhost/src/hir_dump.nova` | 354 | 299 | `dump.rs` |
| `selfhost/src/typeck_tests.nova` | 359 | 309 | N's 12 tests of the item passes' edges |

Beside them `main.nova` gains `selfhost hir` (the resolution dump's
command, generalised: `dump_command`, `dump_hir`, `labels_and_texts`),
`resolver.nova` keeps std's resolution whole in `SharedStd` (`alone`) and
gains `resolve_program_shared_for_check`, and `resolution_dump.nova`
shares `std_start_of` and `diagnostics_text`. Code lines as 5.1 and 5.2
counted them, no blank and no comment line: 3,376 without the tests,
about the 2,900 the spec projected plus the dump and the command.

**The idioms,** beyond ADR 0053's (an id an `Int`, -1 for none; a record
a reference):

- **Equality:** `ty_eq`, `tys_eq`, `head_eq` and `res_eq` where Rust
  compares with the derived `==`, which a user sum lacks (E0013).
- **Maps by `DefId`** (`sigs`, `method_locs`, `selfless`, `mut_self`,
  `supertraits`, `type_arity`) are lists as long as the definitions,
  indexed by the id: looked up as the maps are, and read in id order
  where the dump sorts a map's keys, so no map's order reaches output.
  `impl_selves` stays a `Map` (looked up only); each trait's associated
  types are listed once (`assoc_of`), which `find_assoc_type` reads in
  place of a scan of every definition.
- **Composite keys:** `(item, method)` is `item * 2^20 + method`.
- **Files:** a span carries no file in N, so the checker keeps the
  current module's label (`cur_file`) beside `cur_module`, a span kept in
  a table is a `FileSpan`, and a check that reads a table's spans (the
  signature, coherence, hidden-method and supertrait checks, `normalize`
  from them, pass 2) names the file of each diagnostic (`error_in`).
- **Copies:** a record is a reference, so where Rust clones a scope or a
  bound list before adding to it (`collect_impls`' methods, a trait
  method's scope) N copies it (`copy_scope`, `copy_bounds`), and a
  binding poisoned to `{error}` is set in the impl's own list.
- **Not ported:** the language server's index (`note_*`,
  `enter_type_params`, `type_param_target`) and probe; the fixes for
  needs other than a type and a trait (5.3b's).
- **std shared:** `share_items` checks std alone once per process and
  mode; each program is resolved by `resolve_program_shared_for_check`
  (std's definitions rebased and appended, `rebased_def`; std's module
  scopes appended as they are, read with std's offset, `scope_offset`,
  §10.1) and checked by `check_items_shared`, whose splices add std's
  part of each table where S0's do. When std's resolution or items give a
  diagnostic, every program is resolved and checked in full, as S0 does.

### 9. The differential

`N_HIR` is set: every HIR set of `crates/nova-cli/tests/selfhost.rs` is
compared byte for byte, exit codes included, through `compare_batch`,
and every comparison goes through `run`:

- std's part natively, for the WASM target and under test mode;
- the corpus: 251 loose programs and 74 package files natively, 39 WASM
  programs for the WASM target, 10 files with tests under test mode, held
  to 5.2b's floors (240, 60, 35, 8);
- the cases: 164 under `check` (154 extracted, 10 by hand), 2 for the
  WASM target, 1 under test mode, and all of them again with `\r\n`;
- the type fuzzer's 120 inputs (seeds 401 to 403), part A's floors held
  on N's dumps, which are S0's: E0001 30, E0010 16, E0012 50, E0015 11,
  E0070 10, E0071 14, E0072 21, E0073 18, E0074 23, E0076 5, E0077 20,
  E0078 19, E0105 4, E0403 17, E0900 46;
- the refusals: no root under each mode, test mode for the WASM target
  with a root or std's part, and the 15 argument shapes of
  `both_sides_take_test_in_every_position_alike`, each through both
  dumps; an unreadable root (`IOERR NotFound`, `IOERR InvalidData`);
- the fallback: with a std copy whose `core` names a missing type (an
  item E0001, no resolver diagnostic), N checks std for each program, and
  each checked program's dump is S0's plus std's E0001.

**What the comparison found:** nothing. N's first build gave S0's bytes
on std's three parts, the cases, the corpus and the fuzzer's inputs; the
port's only faults were compile errors (`import` and `module` are
keywords, Nova has no `loop`). Each of the 16 item codes occurs in the
inputs. N's own 44 tests pass (`the_selfhost_tests_pass`), 12 of them the
item passes'.

### 10. The decisions part B took

The spec does not settle these; each with its cost if wrong.

1. **std's module scopes are not rebased per program.** S0 appends std's
   scopes rebased (`ModuleScope::rebased`); N appends them as they are and
   adds std's offset where it reads one (`resolve_type_in`,
   `resolve_trait_in`, `resolve_value_in`). Rebasing would copy std's 25
   scopes of about a thousand names each for every program. If wrong: a
   read of a std scope by another path would miss the offset; the
   differential shows it at once (`component_ret` is the only reader of a
   std scope today).
2. **Lists by `DefId` in place of maps** (§8). If wrong: memory of a few
   lists as long as the definitions (about 1,500 entries) per program.
3. **`selfhost hir` shares `selfhost resolve`'s parser and refusals**, as
   S0's two hidden flags share theirs, so the shapes compare alike.
4. **No unshared mode in N's command line:** N's sharing was measured
   (§11) from a temporary build that never shares, not from a flag.
5. **The files** follow spec §3.5 (`typeck`, `typeck_traits`,
   `typeck_ty`, `fixes`, `hir_dump`), none past 1,100 lines.
6. **N's tests resolve their sources without std** (`check_sources`), so
   their expected values are S0's for the same source wherever std adds
   nothing; the import fix's test makes every module importable, which a
   loose program's loader does not.

### 11. The time, and CI's projection

Measured locally on 2026-11-06, debug S0 and N with the debug runtime,
one process a side over the loose corpus (251 roots), three runs:

| Run | Wall | User |
|---|---:|---:|
| S0 `check --dump-hir` | 1.17-1.25 s | 1.14-1.18 s |
| N `hir` (std shared) | 3.12-3.29 s | 3.06-3.14 s |
| N `hir`, std resolved and checked per program | 7.47-7.72 s | 7.39-7.65 s |
| S0 `check --dump-resolution` | 0.35-0.39 s | 0.32-0.34 s |
| N `resolve` | 1.78-1.94 s | 1.72-1.83 s |

N's HIR dump runs **2.6×** as long as S0's (the resolver's ratio is 5.3×
here, as the research measured): the item stage itself costs N about
1.3 s over the corpus (5 ms a root) and S0 about 0.85 s (3.4 ms a root).
On the 74 package files N takes 9.8 s against S0's 1.6 s, as its
resolution dump does (9.7 s against 1.4 s): 30 of them are N's own
modules, each loading all of N, and parsing dominates.

**Sharing, both sides:** N 3.1-3.3 s shared against 7.5-7.7 s unshared
(2.4×); S0 1.25 s against 3.03 s (part A, in process, before the splice
fix that brought shared to 1.21 s). Unshared, N spends about 17 ms more
a root, resolving std and checking its items again; checking std's items
once costs N about 0.02 s a process (one root's HIR dump 0.305 s against
its resolution dump's 0.289 s). On both sides most of the saving is the
resolution's (5.2b's): S0 measured item-level sharing alone, std's
resolution shared either way, as roughly break-even (part A).

**`selfhost.rs`, three runs each:** before part B (`819b9224`, 50 tests)
22.87, 22.48, 23.17 s, user 72.8-73.5 s; after (51 tests) 33.77, 33.98,
33.82 s, user 109.2-109.9 s: **+11 s wall (+48%), +36 s CPU**. Run alone,
the HIR corpus test went from 4.8 s CPU to 27.9 s, the CRLF cases from
0.8 to 6.6 s, the type fuzzer from 1.9 to 7.4 s, the header test from
0.2 to 4.9 s (N's build in it); the rest is unchanged.

**CI, projected (not measured):** today's CI-to-local factors, from run
42171538582 (52.6 / 53.3 / 61.6 s against 20.5 s), are 2.57 on Linux,
2.60 on macOS and 3.00 on Windows, which put `selfhost.rs` after 5.3a at
about **87 s on Linux, 88 s on macOS and 102 s on Windows**, Windows past
5.2b's 70 s line. 5.2b's lesson holds: its local projection undershot CI
(Linux 24 to 25 s projected, 37.90 s measured), so these are a floor.
The real figures came from the measurement branch on the user's word,
and the user was asked with them (spec §3.7): §15.

### 12. N's stack

N runs on its main thread, 1 MB on Windows. Part B measured it on Linux
(2026-11-06) under `ulimit -s` with inputs built for it (a script, not
committed, lost with its container), each compared with S0's dump:

- **Every written nesting at 128 levels** (the parser's limit): arrays,
  a generic record's arguments, `fn` types, tuples, an E0072 between two
  128-level types and an E0073 naming a 127-level self type: identical to
  S0's on 1 MB, and still on 256 KB.
- **A type built by normalization,** deeper than any written one: 64
  chained associated types, each binding `k` arrays around the next
  (64 × `k` levels, within the 10,000 steps), which E0072 then shows. On
  1 MB part B's N survived `k` up to 126 (8,064 levels) and overflowed
  (SIGSEGV, exit 139) at `k` = 127 (8,128 levels, the deepest the limits
  admit) when its output went to a file or a pipe, as the harness's
  does; to `/dev/null` it survived, a margin of a few frames. On 512 KB
  it failed from 4,096 levels, on 256 KB from 2,048. S0 checks all of
  them on its 64 MB thread. These figures are part B's, kept as its
  record: the construct rebuilt since overflows a link earlier on both
  runtimes (the tables below), and the latest table is the depth this
  ADR records.
- **A 2,000-trait supertrait chain** (`with_supertraits` is a loop): on
  256 KB.

So N overflows where S0 does not, but only on that construct: as last
measured (the second table below), N survives 64 linked associated
types each nesting 124 arrays (7,936 levels) on both runtimes; on the
debug runtime it overflows at 125 (8,000 levels), and on the release
runtime at 125 or at 126 (8,064 levels), 125 being borderline there:
N is within a few hundred bytes of the 1 MB limit; nothing
in the corpus, the cases or the fuzzer comes near. The user ruled
(2026-11-06): a departure from S0, recorded here with the depth N
survives; 5.5's driver runs N on a large-stack thread, and nothing in
the language or the runtime changes now.

**On the release runtime** (part C, 2026-11-10), since the Selfhost job
links N against one (§15): measured on Linux, 1 MB, output to a file,
with the construct rebuilt (a script, not committed; part B's was lost
with its container) on a new container, so both runtimes were measured
again side by side:

| N linked against | Survives | Overflows | Least stack, `k` = 64 (4,096 levels) | Least stack, `k` = 124 (7,936 levels) |
|---|---:|---:|---:|---:|
| the debug runtime | `k` = 124 (7,936 levels) | `k` = 125 (8,000) | 540 KB | 1,020 KB |
| the release runtime | `k` = 124 (7,936 levels) | `k` = 125 (8,000) | 540 KB | 1,024 KB |

The least stack is found to 4 KB, each run's dump compared with S0's.
The depth is the same: the frames are N's own, Cranelift's code for
`normalize_ty` and `display_ty`, at about 128 bytes a level; the
runtime's are leaves above them, the release runtime's at most 4 KB
more on this input. The rebuilt construct overflows two arrays a link
earlier than part B's (124 against 126 on the debug runtime), a
difference of input or container, not of runtime.

**The written nestings in the harness,** on 1 MB on every system (under
`ulimit -s 1024` on Linux and macOS, on Windows's main thread as it
is), on both runtimes: `n_parses_128_levels_of_every_kind_on_a_1_mb_main_thread`
runs N's parser (`selfhost ast`) on every nesting the deep generator
writes, at 128 levels and at 129; `n_checks_items_holding_128_levels_of_every_kind_on_a_1_mb_main_thread`
runs N's item passes (`selfhost hir`) on every written nesting of a type
(arrays, a generic record's arguments, tuples, `fn` types' parameters
and returns, parentheses, optionals, references, mutable references,
pointers) at 128 levels in each item position (a parameter, a return,
a field, a variant's payload, a constant, a trait's method, an impl's
self type, an impl's method alike and unlike its trait's, an associated
type's binding, a `where` bound, an extern), its dump compared with
S0's. Neither comes near the limit: N's dumps of the item test's
inputs are S0's on 64 KB, on either runtime.

**Measured again** (2026-11-10 at `4d0eca73` and 2026-11-11 at
`5bda68ce`, earlier splits of this branch for its plan; `nova` and N
are this branch's, byte for byte, at both; on a slower container with
stable Rust 1.97), the same way (a script, not committed; its source is
in the 5.3a plan, Task 5, Step 7), three runs on each day, and a
third measurement on 2026-11-11 by the plan's second fact-check, at a
later split whose `nova` and N are this branch's too:

| N linked against | Survives | Overflows | Least stack, `k` = 64 | Least stack, `k` = 124 |
|---|---:|---:|---:|---:|
| the debug runtime | `k` = 124 (7,936 levels) | `k` = 125 (8,000) | 544 KB | 1,016 KB, then 1,020 KB |
| the release runtime | `k` = 124; 125 on the first day and in the third measurement | `k` = 125 on the second day; 126 | 540 KB | 1,020 KB, then 1,016 KB |

Within a link and 4 KB of the figures above, and of each other: at
`k` = 124 and 125 N is within a few hundred bytes of the 1 MB limit,
where a run's environment, which shares the main thread's stack, moves
the outcome (the release runtime took `k` = 125 three times on the
first day, failed it three times on the second, and took it again in
the third measurement). This table is the latest measurement: N
survives `k` = 124 on both runtimes; the debug runtime overflows at
125, the release runtime at 125 or 126, borderline at 125.
### 13. N's memory

ADR 0054 §10's method (part B, 2026-11-06; a script, not committed,
after 5.2b's): peak RSS and wall best of three, GC pauses from
`nova_gc_collect_roots` to the end of the sweep under gdb, and
`NOVA_GC_DEBUG`'s largest live heap:

| Runtime | Input | Wall | Peak RSS | Pauses (median / p95 / max, total) | Max live |
|---|---|---:|---:|---|---:|
| debug | `resolve` N's root | 1.08 s | 45.6 MB | 12.9 / 69.3 / 83.1 ms, 376 ms | 13.3 MB |
| debug | `hir` N's root | 1.07 s | 40.1 MB | 13.5 / 69.8 / 92.0 ms, 392 ms | 13.6 MB |
| debug | `hir --std-part` | 0.29 s | 15.0 MB | 5.9 / 24.3 / 24.3 ms, 102 ms | 3.9 MB |
| debug | `hir --target wasm --std-part` | 0.37 s | 18.0 MB | 7.9 / 24.1 / 32.3 ms, 130 ms | 5.6 MB |
| debug | `hir` the loose corpus | 3.12 s | 26.2 MB | 28.3 / 32.9 / 41.8 ms, 1,140 ms | 6.8 MB |
| release | `resolve` N's root | 0.25 s | 44.6 MB | 4.0 / 21.1 / 25.0 ms, 113 ms | 13.3 MB |
| release | `hir` N's root | 0.26 s | 39.2 MB | 3.7 / 20.1 / 23.8 ms, 107 ms | 13.6 MB |
| release | `hir --std-part` | 0.07 s | 14.1 MB | 1.7 / 6.8 / 6.8 ms, 27 ms | 3.9 MB |
| release | `hir --target wasm --std-part` | 0.09 s | 17.1 MB | 2.0 / 6.5 / 9.1 ms, 35 ms | 5.6 MB |
| release | `hir` the loose corpus | 0.76 s | 25.2 MB | 8.2 / 9.1 / 11.7 ms, 325 ms | 6.8 MB |

std's item tables add little to the live heap (N's root: 13.6 MB against
the resolution's 13.3 MB); over the corpus, std's shared state alive
throughout, the live heap stays at 6.8 MB, but the debug runtime spends
over a third of the run (1.14 of 3.12 s) in 46 collections. Recorded, no
threshold (spec §3.7).

**Measured again** (2026-11-10, for the plan, at `4d0eca73`, an
earlier split of this branch whose `nova` and N are this branch's, byte
for byte, on a slower container, the same way: a script, not committed,
its source in the 5.3a plan, Task 5, Step 7): peak RSS within 0.2 MB of
the table's, the same collections and live heaps, and the times longer
by about half on this container:

| Runtime | Input | Wall | Peak RSS | Pauses (median / p95 / max, total) | Max live |
|---|---|---:|---:|---|---:|
| debug | `resolve` N's root | 1.64 s | 45.6 MB | 20.6 / 115.5 / 124.8 ms, 601 ms | 13.3 MB |
| debug | `hir` N's root | 1.54 s | 40.1 MB | 19.2 / 106.9 / 125.9 ms, 582 ms | 13.6 MB |
| debug | `hir --std-part` | 0.43 s | 14.9 MB | 10.0 / 33.2 / 33.2 ms, 148 ms | 3.9 MB |
| debug | `hir --target wasm --std-part` | 0.58 s | 18.0 MB | 12.9 / 34.5 / 48.4 ms, 211 ms | 5.6 MB |
| debug | `hir` the loose corpus | 4.85 s | 26.2 MB | 41.5 / 54.0 / 61.2 ms, 1,709 ms | 6.8 MB |
| release | `resolve` N's root | 0.38 s | 44.6 MB | 4.7 / 29.1 / 38.5 ms, 159 ms | 13.3 MB |
| release | `hir` N's root | 0.37 s | 39.1 MB | 6.1 / 34.1 / 40.8 ms, 175 ms | 13.6 MB |
| release | `hir --std-part` | 0.11 s | 14.2 MB | 2.4 / 8.6 / 8.6 ms, 37 ms | 3.9 MB |
| release | `hir --target wasm --std-part` | 0.15 s | 17.1 MB | 3.2 / 8.1 / 12.6 ms, 50 ms | 5.6 MB |
| release | `hir` the loose corpus | 1.08 s | 25.0 MB | 9.5 / 14.4 / 15.5 ms, 405 ms | 6.8 MB |

### 14. The rule

From 5.3a item collection exists twice, in `nova-typeck` (`check_items`,
passes 2 to 5) and in `selfhost/` (§8): a change to either (a pass, a
table, a diagnostic's words, order or fixes, `convert_ty`, normalization)
changes both in one pull request, and the HIR dump's differential (§9)
holds them equal (phase plan decision 6). The dump's grammar (§2) changes
with them, on both sides.

### 15. CI: `selfhost.rs` in a job of its own, on a release runtime

**Measured:** CI run 42201484370 (draft PR #136, head `d8016169`,
closed unmerged; every check green) put `selfhost.rs` at **70.80 s on
Linux (51 tests), 86.85 s on macOS (49) and 92.54 s on Windows (45)**,
against PR #135's 52.60, 53.25 and 61.58 s and §11's projection of 87,
88 and 102 s: Windows past the 70 s line 5.2b set. The Test jobs took
525, 485 and 751 s (82, 73 and 167 s of them `cargo build`), the
Browser job 752 s.

**The options measured** (2026-11-06): (a) fewer corpus roots, those
that cover the rest; (b) fewer fuzzer inputs; (c) `selfhost.rs` in a CI
job of its own on each system, out of the Test job; (d) N linked against
a release runtime. For (d), `NOVA_EMBED_RUNTIME=1` makes nova-cli's
build script build the runtime with `--release` in a nested cargo and
embed it in the debug `nova`, which links every program it builds
against it (`nova build --release` is LLVM through `clang`, with the
same runtime, so it would not do): locally `selfhost.rs` went from
33.7 s to 17.3 s (108 s to 47 s of CPU), for about 18 s more of a cold
build (8 s the nested build, 8 s gzip of the 28.8 MB library, 2 s
relinking `nova`), with nothing CI lacks and no change to `Cargo.lock`.
(c) and (d) together were projected at about **36, 45 and 48 s** for
the file in its own job, and a Test job shorter by 70 to 93 s. The
figures of (a) and (b) were in the measurement's scratch files, lost
with the container; neither was chosen.

**The user's choice** (2026-11-06): (c) and (d), and no coverage cut:
every root, every fuzzer input and every test of the file still run, on
each system.

**The job** (`.github/workflows/ci.yml`, `selfhost`): the Test job's
three systems, toolchain and cache; `NOVA_EMBED_RUNTIME=1` set for the
job alone, since in the Test job it would move `run_tests` and every
other test that links a program onto the release runtime; `cargo build
--locked --workspace`, then `cargo test --locked --workspace
--all-features --no-fail-fast --test selfhost`, then N's own tests on
the debug runtime (below). Swatinem/rust-cache keys its cache by the
job's id, so this job's `target/`, which holds the embedded runtime,
never reaches the Test job's.

**Out of the Test job:** the target says `test = false`
(`crates/nova-cli/Cargo.toml`), so `cargo test --workspace` skips it and
`cargo test --workspace --test selfhost` runs it, with `Cargo.lock`
unchanged. The other ways were worse: cargo has no way to exclude one
test target from `cargo test --workspace` (`--exclude` drops a whole
package, all of nova-cli's tests); libtest's `--skip` filters names
across every binary, still builds and starts this one, and would skip
a namesake elsewhere; and a variable that returns early would make the
file pass without running. The cost is local: `cargo test --workspace`
no longer runs the file, so `CONTRIBUTING.md`, `agent.md` and the pull
request template add `cargo test --workspace --test selfhost`, and
`cargo clippy --all-features --test selfhost -- -D warnings` beside
their clippy command. A
`test = false` target is also skipped by `--all-targets`, so the clippy
job lints it by name in a second command (one command with both flags
does not: `--all-targets` wins). The ignored-tests step loses nothing:
the file has no `#[ignore]`d test.

**Totals:** `cargo test --locked --workspace --all-features -- --list`
on Linux lists **2,841 tests in 111 binaries** at this branch's last
commit, where `selfhost.rs` among them would make 2,893 in 112 (part C
counted 2,840 and 2,891, before a resolver test and the harness's
128-level item test, §12, were added); the binary gone is nova-cli's
`tests/selfhost.rs`, with its 52 tests (50 on macOS, 46 on Windows). A
plan that counts the suite's totals (`S`) counts them without it from
here, and counts `selfhost.rs`'s (`SH`, `cargo test --locked
--workspace --test selfhost`) apart; a list compared binary by binary,
as 5.2b's `names.py` compares them, shows the binary removed with its
tests and nothing else.

**N on the debug runtime:** the Selfhost job runs N on the release
runtime only, and every other test that links a program runs on the
debug runtime in the Test job; what the move alone would lose is N, a
large program, under the debug runtime's assertions (5 `debug_assert`s,
the collector's pages and the executor's among them) and overflow
checks. One cheap check keeps it: the job's last step runs
`the_selfhost_tests_pass` (`nova test` builds N with its 44 tests and
runs them) with `NOVA_RUNTIME_LIB` naming the debug runtime that
`cargo build --workspace` leaves beside `nova` (`libnova_runtime.a`,
`nova_runtime.lib` on Windows), a variable that outranks the embedded
runtime (a path that does not exist fails the test, which shows it is
read). Measured locally (2026-11-10, debug runtime):

| Check | Where | Cost |
|---|---|---:|
| `the_selfhost_tests_pass` | the Test job: `selfhost.rs` built there again (2.3 s) and the test | 8.2 s |
| `the_corpus_dumps_agree` | the Test job, the same | 13.7 s |
| both | the Test job, the same | 13.9 s |
| `the_selfhost_tests_pass` | the Selfhost job, after its test step: nothing to build | 5.8 s |

In the Test job each would put back part of what the choice took out of
it (about 20 to 40 s on CI at §11's factors), so it runs in the Selfhost
job, which builds nothing more for it. Not chosen: a corpus comparison
on the debug runtime, which would build N a second time and double the
step. If wrong: a fault only N's corpus-sized heaps reach under the
debug runtime's checks goes unseen in CI; the release runtime runs
those heaps every time.

**The 1 MB stack tests** (`n_parses_128_levels_of_every_kind_on_a_1_mb_main_thread`,
N's parser, and `n_checks_items_holding_128_levels_of_every_kind_on_a_1_mb_main_thread`,
N's item passes) now run on the release runtime's frames; N's depth
there is the debug runtime's (§12).

**Locally** (2026-11-10, a new container, slower than 11-06's): `SH`
before the change 57.25 and 62.54 s (182.4 and 202.0 s of CPU); the
job's command after it, without `NOVA_EMBED_RUNTIME`, 63.00 and 60.49 s
(198.2 and 195.7 s), and with it 31.36, 29.04 and 28.49 s (86.5, 80.6
and 78.5 s), **0.48 to 0.50×**, as on 11-06 (0.51×). A cold build of the
job's test, a fresh target directory, the nested build included: 73.3 s
(212 s of CPU).

**Measured on CI** (2026-11-10, run 42774093193, draft PR #137, head
`cc38ac37`, part C, closed unmerged; every check green): the Selfhost
job ran `selfhost.rs` in **37.10 s on Linux (51 tests), 27.53 s on macOS
(49) and 55.38 s on Windows (45)**, against the Test job's 70.80, 86.85
and 92.54 s in run 42201484370 and (c) and (d)'s projection of 36, 45
and 48 s; the job took 2m48s, 2m13s and 4m57s, its build 1m20s,
59s and 2m19s and the test target's build 27, 21 and 54 s, and its last
step, `the_selfhost_tests_pass` on the debug runtime, 5.49, 3.89 and
7.35 s. The Test job took 9m38s, 7m00s and 9m02s (Windows had taken
12m31s), and its sums were `2830 passed, 10 ignored` on Linux and `2807
passed, 9 ignored` on Windows. Rust's cache said "No cache found" on
that first run. Every figure is under its line (70 s on Windows, 60 s
on Linux and macOS), so the user was not asked again. The release
workflow ran too, as a dry run ("Publish the release" skipped). This
branch adds two tests to these: a resolver test to the Test job's
suite, and the 128-level item test (§12) to `selfhost.rs`.

## Consequences

- S0's type checker has an item stage of its own, `check_items`, which
  `check_with` calls first (§1); `nova check`, `build`, `run`, `test` and
  the language server give what they gave. The hidden `nova check
  --dump-hir` prints the item stage's tables, signatures and
  diagnostics (§2, §3), and is in no help or usage line.
- N collects a program's items as S0 does, passes 2 to 5 function for
  function, and prints the same dump, byte for byte, on std's three
  parts, every corpus root in the three modes, the cases with both line
  endings and the type fuzzer's inputs (§8, §9). 5.3b ports the bodies
  on top of it and extends the dump to the full HIR (ADR 0056).
- Both sides collect std's items once per process and mode, and check
  std for each program when std alone gives an item diagnostic (§4,
  §8).
- Item collection exists twice from here: a change to it changes both
  compilers in one pull request (§14).
- `selfhost.rs` runs in a CI job of its own on each system, N linked
  against a release runtime, and no longer in `cargo test --workspace`
  (`test = false`): contributors run `cargo test --workspace --test
  selfhost` and a second clippy command (§15). N's own tests still run
  once on the debug runtime there.
- N overflows its 1 MB main thread on a normalized type about 8,000
  levels deep (8,000 on the debug runtime, 8,000 or 8,064 on the
  release one), where S0 does not; 5.5's driver runs N on a large-stack thread
  (§12). N's heap is recorded, with no threshold (§13).
