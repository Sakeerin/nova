# ADR 0056 — The self-hosted type checker, and the full HIR dump

## Status

Proposed (2026-11-12), written as 5.3b's prototype is built (spec
`docs/superpowers/specs/2026-11-06-phase-5-3-the-type-checker-and-hir-design.md`
§4). Part 1 (S0's side) wrote §1 to §7: S0's order of bodies, the full
dump's function forms with one example of every form, the closure-id
rule, std's bodies shared with the fallback and the bug it works around,
the body cases, the harness's switch, and part 1's figures. Part 2 (N's
frame and expressions) wrote §8. Parts 3 to 5 (the rest of N's side, the
comparison, the fuzzer's body mutations, the depth cases, CI and the
figures) finish it.

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

**Who takes it:** natively 76 roots: the corpus's six std files checked
as programs (`std/{bytes,collections,core,fmt,json,strings}/lib.nova`),
five programs
(`tests/runtime/{generic_impls,hidden_trait_methods,method_generics,trait_assoc_fn,where_clauses}.nova`),
62 of `cases/check/` (the research's twelve probes, now
`cases/check/std_bodies_*`, which must take it, and 5.3a's, 5.2's and the
body tests' impl cases) and three of `cases/resolve/`; for the WASM target
one case, under test mode one (the driver's guard lists them; part 2
corrected the count, which was taken before every case was in). A program
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

### 8. N's bodies, part by part (parts 2 to 4)

N checks the program's bodies as S0 does, function by function:
`infer.nova` (S0's `infer.rs`: plain substitution, the occurs check, `!`
and the error type unifying with anything, a failed unification keeping
the bindings it made), `typeck_body.nova` (the order of §1, a function's,
a method's and a constant's body, blocks, finalizing with E0011, E0081,
E0103 and the helpers `child_exprs`, `collect_captures`, `remap_locals`,
`split_operand`), `typeck_expr.nova` (`check_expr`, `.await` with E0086
and E0087, interpolation, paths, operators, arrays, indexing) and
`typeck_call.nova` (calls, `builtin_signature` by the variant's name,
variants); `fixes.nova` gains the `Need`s of a value, a callee and a
qualifier. N's idioms, beyond ADR 0055's:

- HIR's body types are in `hir.nova`; a `LocalId` is an `Int`. An `Expr`
  is a record of kind, type and span, so finalizing and remapping rewrite
  it in place (spec §4.2); a kind is a sum, whose payload is replaced, not
  rewritten. Names `ast.nova` holds too carry `Hir` (`HirExpr`,
  `HirFunction`, `HirPattern`); a kind's variant is `Ek` and its Rust name
  (`EkCall`), a pattern's `Pat` and the dump's name.
- `fresh` stands where S0 calls it, in the same order, so `?N` numbers
  alike; N's own tests pin a program's numbering.
- A `Binding` is kept by its declaration's file label and span, and a
  quiet newline split by its label, start and end, as S0 keeps them by
  `Span`.
- N checks the program's bodies, then std's, as S0's `check_bodies`
  does (§1); the shared path stops after the program's (§4). Until part
  4, N checked the program's alone and refused the rest.

Part 3 adds the members: `typeck_method.nova` (record literals, fields
and tuple elements, method lookup with `MethodRes`, method calls,
`emit_inherent_call`, `emit_assoc_call`, `emit_trait_call` with
`TraitCallSelf`, `qualifier_self_ty` and its `fresh` calls, interpolation
through `Display` by `try_display`, `shown_method`), `typeck_place.nova`
(assignment, `place_root`, `require_mutable_place` with `MutTarget` and
`PlaceRoot`, element and field sets and their compound forms),
`typeck_closure.nova` (closures lifted and functions as values wrapped,
each taking the next id past every definition in the order met, named
`closure$<id>` and `fnval$<id>`, which the dump prints under §3's rule)
and, in `typeck_call.nova`, `T::f(…)` through a bound, `Type::f(…)`,
`core_sum` and `?`. `fixes.nova` gains a record literal's `Need`,
`mutable_fix`, `immutable_note`, `field_fix` and `method_fix`. Beyond
part 2's idioms:

- `check_call`'s two longest arms (`T::f`, `Type::f`) and `check_try`'s
  E0099 are functions of their own in N, called where S0's code stands.
- A receiver's or a field initializer's AST travels beside its checked
  form, in `TraitCallSelf` as in S0, so `place_root` walks the place as
  written.
- A lifted closure's locals are copies of the enclosing function's, as
  S0's `clone`, since finalizing rewrites a local in place.

**Until N checks every body** (`N_BODIES` false), `selfhost hir` checks
the program's bodies and prints the full dump, and `selfhost hir
--items` prints 5.3a's item part, which every 5.3a comparison takes (the
harness's `n_hir`, beside S0's `--dump-hir-items`). A construct a later
part ports is one stub in N, which records what it is: N then prints
`UNPORTED` and why after the root's header and exits 3, so a half-checked
body is never compared as though it were complete. The stubs, after
part 4: views (part 5). Parts 2 and 3 also stubbed `match`, `for`,
destructuring `let` and std's bodies (part 4). `crates/nova-cli/tests/selfhost/n_bodies.txt`
lists the inputs whose full program part N gives, and
`n_gives_the_full_dump_of_the_listed_inputs_and_refuses_the_rest` holds it
both ways over every input of the full dump's sets, std's three parts
listed as `<mode> std` since part 4: each listed input's part is S0's,
byte for byte, and every other input is one N refuses. Since part 4 the
test runs its batches on every core, N's own package roots three at a
time, as one set's thread was its long pole. Each part adds the inputs it then gives; the last sets
`N_BODIES`, and removes the list, the test, `UNPORTED` and both item-only
forms.

**Part 2's figures** (2026-11-12): of 1,204 inputs (826 cases of the
three modes, 378 corpus roots of the three modes), N gives 384 (352
cases, 32 corpus roots; 364 reach the type checker, 362 with functions,
582 functions) and refuses 820; none differs from S0's. With CRLF line
endings, the 814 loose cases of `cases/check/` give the same. Of 5.3a's
167 item cases, 112 agree and 55 are refused (27 take the full check).

**Part 3's figures** (2026-11-12): of 1,211 inputs (826 cases of the
three modes, 385 corpus roots of the three modes), N gives 770 (622
cases, 148 corpus roots) and refuses 441 (`match` 243, the fallback's
roots 75, `for` 65, views 37, destructuring `let` 21); none differs from
S0's. With CRLF line endings, the 814 loose cases of `cases/check/`
give 616 and refuse 198, none differing. 1,524 mutants of the listed
cases and corpus roots, aimed at the members, and every input of the
type, name, mutation, soup, depth and edge fuzzers: none differs. Of
5.3a's 167 item cases, 138 agree and 29 are refused.

Part 4 adds control flow and patterns: `usefulness.nova` (S0's
`usefulness.rs`: `Ctor`, the pattern as `UPat`, `MatchCx` with
`naming_values`, `missing_value`), `typeck_pattern.nova` (`check_match`,
the reachability and exhaustiveness glue with E0020, E0021 and its
"remove the unreachable arm" fix, `render_witness`, `check_pattern` with
its variant, tuple, or- and range patterns, and `patterns.rs`'s
`PatState`, E0100, E0101, E0022, E0104 with its "did you mean" fix, the
or-alternatives' E0021 and `restrict_or`), `typeck_for.nova` (`for` over
a range, an array and an `Iterator`-shaped `next`, `LoopBind`,
`let_pattern`) and the destructuring `let` in `typeck_body.nova`;
`fixes.nova` gains a pattern's `Need`, `scrutinee_sum`, `remove_arm_fix`
and lines.rs's `arm_removal`. std's bodies: `share_items` checks them
once per process and mode, on the checker its tables come from, whose
tables a body never changes, and shares nothing should they give a
diagnostic (S0's `SharedItems::new`, which collects std's items a second
time to check them); `selfhost hir --std-part` prints std's full part;
a program the shared path cannot check (§4) is resolved again in full
and checked with std's bodies, as S0's driver does. Beyond the earlier
parts' idioms:

- `UPat`'s variants carry `Up`, as `hir.nova`'s `HirPattern` holds
  `PatOr` and `PatTuple`; an or-pattern's tree node is `(parent node,
  parent alternative, alternatives)`, the parent -1 at the top.
- A matrix's rows and a witness are built new wherever S0 clones a
  slice, since a `Vec` is shared when passed.
- A witness's `String` and `Char` print as Rust's `{:?}` does
  (`debug_str`, `debug_char`): `\0`, `\t`, `\r`, `\n`, `\\` and the
  quote escaped, a control character, a format character, a separator, a
  private use and the commonest grapheme extenders (combining marks,
  variation selectors, the zero-width non-joiner) as the code point's
  escape. Rust's full `is_printable` and `Grapheme_Extend` tables are
  not ported (a departure; no input holds another such character).
- A `for` loop's hidden locals are made where S0 makes them, a range's
  `__run` after its body, since their ids reach the dump.

**Part 4's figures** (2026-11-12): std's full part is S0's, byte for
byte, in the three modes, at the first run. Of 1,216 roots and std's
three parts (826 cases of the three modes, 390 corpus roots of the three
modes), N gives 1,159 (806 cases, 350 corpus roots, std's three parts),
every fallback root among them, and refuses 60, all for views (20
cases, 40 corpus roots); none differs from S0's. With CRLF line endings,
the 814 loose cases of `cases/check/` give 795 and refuse 19 (views).
2,059 mutants of the listed cases and corpus roots, aimed at the members
and at `match`, patterns, `for` and destructuring `let`, the research's
16 probes of §4, 13 hand cases of the three modes and a two-module
case, and every input of the type, name, mutation, soup, depth and edge
fuzzers: none differs.

| N's run | Wall, debug runtime | Wall, release runtime | Peak RSS | GCs | Max live | S0's wall |
|---|---:|---:|---:|---:|---:|---:|
| `hir --std-part` | 0.70 s | 0.17 s | 32.8 MB | 14 | 14.2 MB | 0.11 s |
| `hir --target wasm --std-part` | 0.77 s | 0.20 s | 38.1 MB | 14 | 11.1 MB | 0.11 s |
| `hir --test --std-part` | 0.71 s | 0.17 s | 33.1 MB | 14 | 14.2 MB | 0.10 s |
| `hir` N's root | 4.15 s | 1.08 s | 145.2 MB | 21 | 54.6 MB | 0.66 s |

Against ADR 0055 §13's item stage (std's part 0.29 s, 15.0 MB, 3.9 MB
live; N's root 1.07 s, 40.1 MB, 13.6 MB live on the debug runtime), the
bodies multiply std's part's time by 2.4 and its live heap by 3.6, and
N's root's time by 3.9 and its live heap by 4. Each `selfhost hir` run
checks std's bodies once: 0.17 s more a run on the debug runtime. N's
stack, under `ulimit -s` on Linux, its dump compared with S0's: 127
nested `match`es (in an arm and as a scrutinee), `for` loops (over a
range, an array and a tuple pattern), variant and tuple patterns
(`let`) and or-patterns, and 63 closures in closures, all on 1 MB on
both runtimes, the least stack 200 to 216 KB for the expressions and
48 to 64 KB for the patterns. The 795 listed loose cases of
`cases/check/` take S0 6.5 s, N 21.8 s on the debug runtime and 5.6 s
on the release one. `selfhost.rs` locally: 115 to 117 s (56 tests),
against part 3's 74 to 82 s.
