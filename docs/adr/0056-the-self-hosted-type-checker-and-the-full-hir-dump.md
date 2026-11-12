# ADR 0056 — The self-hosted type checker, and the full HIR dump

## Status

Proposed (2026-11-12), written as 5.3b's prototype is built (spec
`docs/superpowers/specs/2026-11-06-phase-5-3-the-type-checker-and-hir-design.md`
§4). Part 1 (S0's side) wrote §1 to §7: S0's order of bodies, the full
dump's function forms with one example of every form, the closure-id
rule, std's bodies shared with the fallback and the bug it works around,
the body cases, the harness's switch, and part 1's figures. Parts 2 to 5
(N's side, the comparison, the fuzzer's body mutations, the depth cases,
CI and the figures) finish it.

## Decision

### 1. S0 checks the program's bodies, then std's, each in `DefId` order

`check_with` (pass 6, `Checker::check_bodies`) checks the program's
functions, methods and constants, then std's, each in `DefId` order:
functions, then methods by their sorted ids, then constants. Before, the
methods came in `FxHashMap` order (`method_locs.keys()`), which put
eight methods' diagnostics in the order 7 4 1 8 5 2 6 3. The constant
cycles (E0081) and the newline splits (E0103) follow, then the closures
and wrappers in the order they were lifted, so the program's take the ids
from `defs.len()` on and std's follow them.

**What a user sees** (the CHANGELOG's Changed): diagnostics inside
methods in declaration order, and closures' and wrappers' names
(`closure$N`, `fnval$N`) renumbered, in symbols only. `nova check` over
every base-tracked `.nova` file prints what the base prints but for 11
checks, each the same diagnostics reordered (§7). A test pins it
(`bodies_are_checked_the_programs_first_each_in_def_id_order`).

**Why the program's first** (spec §4.1): std's closures then follow all
of the program's, so skipping std's bodies (§4) changes no id a program
prints.

### 2. The full dump

`nova check --dump-hir` prints 5.3a's part (ADR 0055 §2) and then every
function, after the externs and before the diagnostics; the diagnostics
are then every diagnostic the type checker gave, the bodies' too. std's
part (`--std`) adds ` functions=<n>` to its first line. The hidden
`nova check --dump-hir-items`, by the same parser and refusals, prints
5.3a's part alone, which the differential compares until N checks bodies
(§6).

**Order:** a part's own functions in `hir::Module`'s order (§1):
functions, methods and constants by `DefId`, then the closures and
wrappers as they were lifted. A function is std's part's when its id is
std's, or, for a closure or wrapper, when it was lifted from std's
bodies (§3).

**The grammar** (`I(d)` is `d` levels of indentation; `span` is
`start..end` in the function's file, or a whole place, label and span,
should a node be in another file, which none is today):

```
entries    = { record } { sum } { trait } { impl } { sig } { extern } { function }
function   = I(1) "(Function " id " " string " generics=" uint " takes_env=" bool
             " captures=" uint " params=" uint " async=" bool " " place
             bounds(2) I(2) "(Ret " ty ")" { local } expr(2) ")"
local      = I(2) "(LocalDecl l" uint " " string " mut=" bool " " span " " ty ")"
expr(d)    = I(d) "(" kind { " " value } [ " " flag "=" bool ] " " span " " ty
             [ I(d+1) "(SelfType " ty ")" ] [ I(d+1) "(TypeArgs" { " " ty } ")" ]
             { expr(d+1) | arm(d+1) } ")"
arm(d)     = I(d) "(Arm guard=" bool " " span pattern(d+1) [ expr(d+1) ] expr(d+1) ")"
pattern(d) = I(d) "(PatWildcard)" | I(d) "(PatBind l" uint ")" | I(d) "(PatInt " int ")"
           | I(d) "(PatBool " bool ")" | I(d) "(PatStr " string ")" | I(d) "(PatChar " uint ")"
           | I(d) "(PatRange " int " " int ")"
           | I(d) "(PatVariant " id " " uint { pattern(d+1) } ")"
           | I(d) "(PatTuple" { pattern(d+1) } ")" | I(d) "(PatOr" { pattern(d+1) } ")"
```

| Kind | Values | Flag | Children |
|---|---|---|---|
| `IntLit` | the value in decimal | | |
| `FloatLit` | the 16 lowercase hex digits of `to_bits` (5.1's) | | |
| `BoolLit` | `true` or `false` | | |
| `StrLit` | the string, quoted as 5.1 quotes it | | |
| `CharLit` | the code point in decimal (5.1's) | | |
| `Unit`, `Break`, `Continue` | | | |
| `Local` | `l<n>` | | |
| `MakeClosure` | the function's id | | `TypeArgs`; the captures |
| `Call` | `Def <id>`, `Builtin <name>` (the Rust variant's name, as the resolution dump and N's `BUILTIN_VARIANTS` name it) or `Local l<n>` | | `TypeArgs`; the arguments |
| `MakeVariant` | the sum's id, the variant's index | | the arguments |
| `MakeRecord` | the record's id | | the fields, in declared order |
| `MakeTuple`, `MakeArray`, `StrConcat` | | | the elements |
| `FieldGet` | the index | | the target |
| `FieldSet` | the index | | the target, the value |
| `ArrayRepeat` | | | the value, the length |
| `Index` | | | the target, the index |
| `IndexSet` | | | the target, the index, the value |
| `ArrayLen`, `ToStr`, `Await` | | | the operand |
| `TraitCall` | the trait's id, the method's index | `receiver` | `SelfType`; `TypeArgs`; the receiver, when there is one; the arguments |
| `Binary` | `Add` … `Shr` (`hir::BinOp`'s names) | | the two operands |
| `LogicalAnd`, `LogicalOr` | | | the two operands |
| `Unary` | `Neg`, `Not` or `BitNot` | | the operand |
| `Let`, `Assign` | `l<n>` | | the value |
| `Block` | | `tail` | the statements, then the tail when there is one |
| `If` | | `else` | the condition, the branch, then the other when there is one |
| `While` | | | the condition, the body |
| `Match` | | | the scrutinee, then each arm |
| `Return` | | `value` | the value, when there is one |

A pattern carries no span or type (`hir::Pattern` has none), so it prints
one line per pattern, its sub-patterns a level deeper; a range prints as
stored, both ends included (`1..3` is `1 2`). `TypeArgs` prints only when
there are any. Locals print in `hir::Function::locals`' order, `l<n>`
their index; a closure's are its own, captures first. A flag is a
`key=bool` field, so the harness's form check sees both of its values.

`nova_typeck::dump::BODY_KINDS` lists every form a function prints; the
harness checks that the full dumps of std, the corpus and the cases print
each of them and nothing unlisted; 5.3a's `KINDS` stay the item part's.

**One example of every form**, S0's `nova check --dump-hir forms.nova`
of the source after it:

```
;; forms.nova:
;;   record P { x: Int }
;;   type T = | A(Int, Bool) | B
;;   fn f<U: Eq>(u: U, mut p: P, xs: [Int]) -> Int {
;;       p.x = xs[0] + -1
;;       let g = |y: Int| y
;;       let mut i = 0
;;       while i < 2 && !false || true { i = i + 1; if i > 5 { break } else { continue } }
;;       let mut a = [0; 2]
;;       a[1] = a.len()
;;       let t = (1, 'c')
;;       if u.eq(u) { return t.0 }
;;       match A(g(2), true) {
;;           A(1..3 | 7, b) if b => 0
;;           A(n, _) => n
;;           B => P { x: [5][0] }.x
;;       }
;;   }
;;   fn s(o: (String, Char, Bool)) -> String {
;;       match o {
;;           ("a", 'b', true) => "${1.5}"
;;           _ => "${Int::default()}"
;;       }
;;   }
;;   async fn h() -> Int { 1 }
;;   async fn k() { let v = h().await; println("v=${v}"); let u = (); return }
;;   fn w() { let z = 1 }
(Hir target=native checked=true
  (StdStart module=1 def=7 item=7)
  (Record u0 "P" generics=0
    (Field "x" Int))
  (Sum u1 "T" generics=0
    (Variant "A" Int Bool)
    (Variant "B"))
  (Sig u2 generics=1
    (Bound 0 s19)
    (Param T0)
    (Param u0)
    (Param [Int])
    (Ret Int))
  (Sig u3 generics=0
    (Param (String, Char, Bool))
    (Ret String))
  (Sig u4 generics=0
    (Ret Future<Int>))
  (Sig u5 generics=0
    (Ret Future<()>))
  (Sig u6 generics=0
    (Ret ()))
  (Function u2 "f" generics=1 takes_env=false captures=0 params=3 async=false "forms":51..52
    (Bound 0 s19)
    (Ret Int)
    (LocalDecl l0 "u" mut=false 60..61 T0)
    (LocalDecl l1 "p" mut=true 70..71 u0)
    (LocalDecl l2 "xs" mut=false 76..78 [Int])
    (LocalDecl l3 "y" mut=false 130..131 Int)
    (LocalDecl l4 "g" mut=false 125..126 fn(Int) -> Int)
    (LocalDecl l5 "i" mut=true 152..153 Int)
    (LocalDecl l6 "a" mut=true 256..257 [Int])
    (LocalDecl l7 "t" mut=false 294..295 (Int, Char))
    (LocalDecl l8 "b" mut=false 383..384 Bool)
    (LocalDecl l9 "n" mut=false 406..407 Int)
    (LocalDecl l10 "__f_x" mut=false 434..435 Int)
    (Block tail=true 94..455 Int
      (FieldSet 0 100..116 ()
        (Local l1 100..101 u0)
        (Binary Add 106..116 Int
          (Index 106..111 Int
            (Local l2 106..108 [Int])
            (IntLit 0 109..110 Int))
          (Unary Neg 114..116 Int
            (IntLit 1 115..116 Int))))
      (Let l4 121..139 ()
        (MakeClosure u7 129..139 fn(Int) -> Int
          (TypeArgs T0)))
      (Let l5 144..157 ()
        (IntLit 0 156..157 Int))
      (While 162..243 ()
        (LogicalOr 168..191 Bool
          (LogicalAnd 168..183 Bool
            (Binary Lt 168..173 Bool
              (Local l5 168..169 Int)
              (IntLit 2 172..173 Int))
            (Unary Not 177..183 Bool
              (BoolLit false 178..183 Bool)))
          (BoolLit true 187..191 Bool))
        (Block tail=true 192..243 !
          (Assign l5 194..203 ()
            (Binary Add 198..203 Int
              (Local l5 198..199 Int)
              (IntLit 1 202..203 Int)))
          (If else=true 205..241 !
            (Binary Gt 208..213 Bool
              (Local l5 208..209 Int)
              (IntLit 5 212..213 Int))
            (Block tail=true 214..223 !
              (Break 216..221 !))
            (Block tail=true 229..241 !
              (Continue 231..239 !)))))
      (Let l6 248..266 ()
        (ArrayRepeat 260..266 [Int]
          (IntLit 0 261..262 Int)
          (IntLit 2 264..265 Int)))
      (IndexSet 271..285 ()
        (Local l6 271..272 [Int])
        (IntLit 1 273..274 Int)
        (ArrayLen 278..285 Int
          (Local l6 278..279 [Int])))
      (Let l7 290..306 ()
        (MakeTuple 298..306 (Int, Char)
          (IntLit 1 299..300 Int)
          (CharLit 99 302..305 Char)))
      (If else=false 311..336 ()
        (TraitCall s19 0 receiver=true 314..321 Bool
          (SelfType T0)
          (Local l0 314..315 T0)
          (Local l0 319..320 T0))
        (Block tail=true 322..336 !
          (Return value=true 324..334 !
            (FieldGet 0 331..334 Int
              (Local l7 331..332 (Int, Char))))))
      (Match 341..453 Int
        (MakeVariant u1 0 347..360 u1
          (Call Local l4 349..353 Int
            (IntLit 2 351..352 Int))
          (BoolLit true 355..359 Bool))
        (Arm guard=true 371..385
          (PatVariant u1 0
            (PatOr
              (PatRange 1 2)
              (PatInt 7))
            (PatBind l8))
          (Local l8 389..390 Bool)
          (IntLit 0 394..395 Int))
        (Arm guard=false 404..411
          (PatVariant u1 0
            (PatBind l9)
            (PatWildcard))
          (Local l9 415..416 Int))
        (Arm guard=false 425..426
          (PatVariant u1 1)
          (FieldGet 0 430..447 Int
            (Block tail=true 430..445 u0
              (Let l10 430..445 ()
                (Index 437..443 Int
                  (MakeArray 437..440 [Int]
                    (IntLit 5 438..439 Int))
                  (IntLit 0 441..442 Int)))
              (MakeRecord u0 430..445 u0
                (Local l10 430..445 Int))))))))
  (Function u3 "s" generics=0 takes_env=false captures=0 params=1 async=false "forms":459..460
    (Ret String)
    (LocalDecl l0 "o" mut=false 461..462 (String, Char, Bool))
    (Block tail=true 496..589 String
      (Match 502..587 String
        (Local l0 508..509 (String, Char, Bool))
        (Arm guard=false 520..536
          (PatTuple
            (PatStr "a")
            (PatChar 98)
            (PatBool true))
          (StrConcat 540..548 String
            (ToStr 543..546 String
              (FloatLit 3ff8000000000000 543..546 Float))))
        (Arm guard=false 557..558
          (PatWildcard)
          (StrConcat 562..581 String
            (ToStr 565..579 String
              (TraitCall s23 0 receiver=false 565..579 Int
                (SelfType Int))))))))
  (Function u4 "h" generics=0 takes_env=false captures=0 params=0 async=true "forms":599..600
    (Ret Future<Int>)
    (Block tail=true 610..615 Int
      (IntLit 1 612..613 Int)))
  (Function u5 "k" generics=0 takes_env=false captures=0 params=0 async=true "forms":625..626
    (Ret Future<()>)
    (LocalDecl l0 "v" mut=false 635..636 Int)
    (LocalDecl l1 "u" mut=false 673..674 ())
    (Block tail=true 629..689 !
      (Let l0 631..649 ()
        (Await 639..648 Int
          (Call Def u4 639..642 Future<Int>)))
      (Call Builtin Println 650..667 ()
        (StrConcat 658..666 String
          (StrLit "v=" 658..666 String)
          (ToStr 663..664 String
            (Local l0 663..664 Int))))
      (Let l1 669..680 ()
        (Unit 677..679 ()))
      (Return value=false 681..687 !)))
  (Function u6 "w" generics=0 takes_env=false captures=0 params=0 async=false "forms":693..694
    (Ret ())
    (LocalDecl l0 "z" mut=false 703..704 Int)
    (Block tail=false 697..710 ()
      (Let l0 699..708 ()
        (IntLit 1 707..708 Int))))
  (Function u7 "closure$u7" generics=1 takes_env=true captures=0 params=1 async=false "forms":129..139
    (Bound 0 s19)
    (Ret Int)
    (LocalDecl l0 "y" mut=false 130..131 Int)
    (Local l0 138..139 Int)))
(Diagnostics)
```

The closure `closure$u7` is `u7`: the program has seven definitions
(`StdStart … def=7`), and its first closure prints as though it came
right after them (§3), its `DefId` being past every definition, std's
included. std's part prints its own closures as `s<n>` past std's
definitions: natively `"fnval$s713"` is its first.

**Sizes** (S0, 2026-11-12): std's part natively 24,066 lines
(1,035,157 bytes), 681 functions; for the WASM target 24,848 (1,048,056),
708; under test mode 24,137 (1,037,959), 684; 5.3a's item part was 3,107
lines natively. The corpus, natively: the 251 loose programs 78,316 lines
(3.30 MB), 44 package files 93,189 (4.00 MB), N's 30 package roots, each
of which holds all of N, 690,269 (31.7 MB); the 39 WASM programs 9,825
(0.42 MB); the 10 files with tests, under test mode, 164,793 (7.48 MB).

### 3. Closure ids

A closure or wrapper takes a `DefId` past every definition
(`next_closure_def`, from `defs.len()`), std's included, so 5.3a's id rule
printed a program's first closure as `s<n>`, and std's closures, numbered
after the program's, moved with the program's closure count (the
research's headline 4). S0's ids stay as they are (§1); the dump maps
them:

- the program's closures and wrappers are those lifted from the
  program's bodies, `defs.len()` up to the first of std's (`Checked`
  records where std's begin); the `k`-th prints `u<D+k>`, `D` the
  program's definitions (`StdStart`'s `def`), as though they came right
  after them;
- std's, the rest, the `k`-th prints `s<S+k>`, `S` std's definitions, as
  though they came right after std's;
- a lifted function's name holds its printed id: `closure$u7`,
  `fnval$s713`.

So std's part is the same text in every program of a mode (the driver's
`std_part_of_a_programs_check_is_std_part`, now with closures on both
sides), and a program's closures print the same whatever std's size
(`shared_bodies_check_what_the_full_check_does` checks a program against
std with one more module). N maps its own ids the same way.

**If wrong:** a closure lifted from a program's body while std's are
checked, or the other way, would print in the wrong part. No check lifts
across bodies: a closure or wrapper is lifted while its enclosing body is
checked and finalized with it (`pending_closures`). The equivalence test
(§4) and the std-part test would show it. The rule costs nothing at run
time: two numbers in `Checked`.

### 4. std's bodies, shared, and the fallback

std's bodies are checked once per process and mode: `SharedItems::new`
checks std alone in full, items and bodies, and shares nothing should
either give a diagnostic. A program's shared check
(`nova_typeck::check_program_shared`) reads std's tables (ADR 0055 §4),
checks the program's bodies alone, and stops at the program's segment:
std's bodies are std's part, which `--std` prints. Its closures take the
ids the full check gives them (§1), and its diagnostics are the full
check's, since std's bodies give none.

**The bug, and the fallback** (the user's ruling of 2026-11-12): method
lookup scans every impl, with no filter by the traits in scope
(`resolve_method_on`, `find_inherent_method`), so a program can change
how std's bodies check. The research's probes: `trait Mine { fn
flush(self) -> Int }` with `impl Mine for Stdout` gives E0015 inside
std's `ui/ssr` (`stdout().flush()`); an inherent `impl Stdout { fn flush
… }` gives nothing, and std's call then binds to the program's method;
an inherent `Map::new` gives E0074 and 20 E0015s in std's bodies. **This
is a bug in S0, recorded here and not fixed in 5.3b:** std's bodies
should not see a program's impls. S0's behaviour is unchanged. So the
shared path takes the full check
(`nova_typeck::implements_on_a_type_it_does_not_own`) when a program has
an impl, inherent or a trait's, whose self type is not one of its own
records or sums: a primitive, an array, or one of std's types; and, to
err on the safe side, when the item passes did not collect every impl
the program wrote (an impl on a tuple, a function type or a projection,
which E0010 and E0900 refuse). The full check prints std's bodies'
diagnostics among the program's, so the dump equals the full check's
byte for byte.

**Who takes it:** natively 51 roots: the corpus's six std files checked
as programs (`std/{bytes,collections,core,fmt,json,strings}/lib.nova`),
five programs
(`tests/runtime/{generic_impls,hidden_trait_methods,method_generics,trait_assoc_fn,where_clauses}.nova`)
and 40 cases (the research's twelve probes, now
`cases/check/std_bodies_*`, which must take it, and 5.3a's and 5.2's
impl cases); for the WASM target one case, under test mode one. A program
that takes it costs a second resolution and a full check.

**The guard:** the driver's
`std_checked_once_dumps_what_std_checked_for_each_program_dumps` compares
the shared and unshared dumps, the full dumps and 5.3a's item dumps, over
every corpus root and case of each mode, in order and reversed, checks
that every `std_bodies_*` case takes the full check, and lists those that
do.

### 5. The body cases

The type checker's 426 body-level tests (the 550 that lex a source, less
5.3a's 112 and the language server's 12 probe tests), with 724 sources,
become cases by 5.3a's rule (ADR 0055 §5): the research's instrumentation
of the test helpers (`r53b_rec`, a script, not committed) writes each
source a test lexes under its name; a test's key is `check__<test>` for
`check.rs`'s tests and `<module>__<test>` for `lang_tests`,
`pattern_tests`, `std_tests` and `tuple_tests`; a case is `<key>.nova`,
or `<key>_<k>.nova` for a test that lexed more than one source, `k` the
source's place; the two-module test is a directory, `main.nova` and
`lex.nova`; a source holding `@test` goes under test mode. A source whose
text is a loose case's already (`check*` and `resolve*` of its mode) or
an earlier source's is dropped: 77, which leaves 36 tests with no case of
their own. That gives 641 files and one directory under `cases/check/`
and four files under `cases/check-test/`; with 5.3a's 164, the twelve
probes of §4 and one hand case for a fix of every kind of name
(`body_fixes_for_every_need`), `cases/check/` holds 819 programs. Every
one of the 38 codes occurs among them (the sixteen body codes that had
no case before, E0011 … E0103, all through the extracted sources), and
every note, secondary label and fix form occurs in the cases, the corpus
or std.

**Rust-only** (49): the 12 probe tests; `infer.rs`'s 13 and
`usefulness.rs`'s 14; the builtin tables' 6
(`builtin_signatures_are_what_the_std_call_sites_use`,
`reserved_type_names_is_exactly_the_seven_expected_names`,
`shutdown_requested_is_std_only_and_returns_bool`,
`the_file_builtins_are_std_only_and_return_status_words`,
`the_process_builtins_are_std_only_with_their_signatures`,
`the_stream_builtins_are_std_only_and_return_status_words`);
`std_tests::an_arrays_head_is_array_whatever_its_element`,
`tuple_tests::a_tuple_displays_as_written`; and `items.rs`'s 2. 5.3b adds
two more: the order test (§1) and `items.rs`'s shared bodies (§4).

### 6. The harness, S0's side

`selfhost.rs` gains `N_BODIES`, false in part 1. Until N checks bodies,
every comparison with N takes S0's `check --dump-hir-items` (`hir_s0`),
so 5.3a's checks hold as they did, and the full dump's sets run through
S0 alone, through `run`: std's three parts, the corpus natively, for the
WASM target and under test mode, and the cases with both line endings;
each dump well formed (`assert_well_formed`: every line at its depth,
the parentheses balanced), every form of `KINDS` and `BODY_KINDS` and
each of the 38 codes present, std's parts free of a program's id. Later
parts set `N_BODIES`, and every set is then compared byte for byte.

### 7. Part 1's figures

- **`nova check` over every base-tracked `.nova` file:** 1,263 checks (`checkcmp.sh`,
  the base `e5c555d0`): 11 differ, each the same diagnostics in another
  order, methods' now in declaration order:
  `selfhost/src/{typeck,typeck_tests,typeck_traits}.nova` checked alone
  and `std/{bytes,core,dom,http,json,net,strings,time}/lib.nova` checked
  as programs. `nova parse --check` and `--ast` over its 1,157 files: none
  differs. Every `--help` is the base's (22 of 22), and every usage error
  (342 runs under six `argv[0]`s, 10 in a terminal).
- **S0's time** (debug, best of five): std's full part 102 ms natively,
  109 ms for the WASM target, 102 ms under test mode, against 63, 67 and
  62 ms for the item part. The full dump against the item dump: the loose
  corpus 2.66 s (1.36), the package files 0.65 s (0.49), N's roots 3.63 s
  (1.49), the WASM programs 0.30 s (0.26), the files with tests 0.97 s
  (0.45), `cases/check/` 6.79 s (3.95).
- **`selfhost.rs` locally**, three runs: before part 1, 36.30, 37.19 and
  39.89 s (52 tests); after, 47.85, 48.36 and 47.49 s (55 tests), the full dump's
  sets on S0 alone and the 659 new cases among them.
