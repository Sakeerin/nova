# ADR 0056 — The self-hosted type checker, and the full HIR dump

## Status

Proposed (2026-11-12), written as 5.3b's prototype is built (spec
`docs/superpowers/specs/2026-11-06-phase-5-3-the-type-checker-and-hir-design.md`
§4). Part 1 (S0's side) wrote §1 to §7: S0's order of bodies, the full
dump's function forms with one example of every form, the closure-id
rule, std's bodies shared with the fallback and the bug it works around,
the body cases, the harness's switch, and part 1's figures. Parts 2 to
4 (N's frame and expressions, members, control flow, patterns and std's
bodies) wrote §8. Part 5 (views, the harness in its final form, the body
fuzzer, the depth cases and the figures) wrote §9 to §16 and finished §4,
§6 and §8. CI's figures are §14's placeholder, which the measurement run
fills.

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
the shared and unshared full dumps over every corpus root and case of
each mode, in order and reversed, checks that every `std_bodies_*` case
takes the full check, and lists those that do (at part 5: 76 natively,
one for the WASM target, one under test mode). Parts 1 to 4 compared
5.3a's item dumps too; part 5 dropped them with the item-only dump (§10):
the full dump holds the item tables, and on a root that takes the full
check the shared item tables are built and set aside, printed nowhere.
The unshared pass, which keeps nothing from one root to the next, runs
in three parts on threads of its own beside the shared path's forward
and backward passes (95 s locally in one thread, 47 s so).

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

### 6. The harness, S0's side (part 1)

`selfhost.rs` gained `N_BODIES`, false in part 1. Until N checks bodies,
every comparison with N takes S0's `check --dump-hir-items` (`hir_s0`),
so 5.3a's checks hold as they did, and the full dump's sets run through
S0 alone, through `run`: std's three parts, the corpus natively, for the
WASM target and under test mode, and the cases with both line endings;
each dump well formed (`assert_well_formed`: every line at its depth,
the parentheses balanced), every form of `KINDS` and `BODY_KINDS` and
each of the 38 codes present, std's parts free of a program's id. Part 5
removed `N_BODIES` and compares every set byte for byte (§10).

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

### 8. N's bodies, part by part (parts 2 to 5)

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

**Until N checked every body** (`N_BODIES` false, parts 2 to 4; this
paragraph is their record), `selfhost hir` checks
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

Part 5 adds views: `typeck_view.nova` (S0's `check/view.rs`: `ui`,
`component_ret`, which moves there from `typeck_ty.nova`, `check_view`
with E0094, `view_node`, `view_text`, `element` with E0095,
`element_builder` with E0096 and the type checker's E0001, `view_attr`
and `view_handler` with E0097, `reader`, `ui_variant`, and the free
`text_child`, `is_str_lit`, `sum`, `str_lit` and `array`). S0's
`note_use` calls are the language server's index (decision 8), not
ported. `view_handler`'s two guards keep S0's order, each unification
kept when a later one fails. Of `fixes.rs`, nothing was left after part
4: every function has its N counterpart (`visible_locals` is `FnCtx`'s,
`offers` is `offers_value` for a value, a type's and a trait's being
offered by namespace), which the cases' fixes and the body fuzzer
compare. Part 5 then removed the scaffolding of the paragraph above: the
stubs, `Checker::unported`, `UNPORTED`, exit 3, `selfhost hir --items`,
S0's hidden `check --dump-hir-items` with the driver's item-only dumps,
`n_bodies.txt` and its test. No hidden form remains but `check
--dump-hir` (S0) and `selfhost hir` (N); `--dump-hir-items` is refused
as the base refuses any unknown argument, byte for byte (§10).

**Part 5's figures** (2026-11-12, at `dd765c16`): N refuses nothing. Of
1,217 roots (826 cases of the three modes, 391 corpus roots: 251 loose
programs, 88 package files, 39 WASM programs, 13 files with tests) all
agree, and std's three full parts; none differs. With CRLF line endings
the 814 loose cases of `cases/check/` agree, and every input of the
type (120), name (162), mutation (750), soup (6,000), depth (190) and
edge (93) fuzzers, the body fuzzer's 120 (§11), the 45 depth cases (§12)
and 26 hand cases, one per aimed code of the body fuzzer's appended
functions. N's files: `infer.nova` 152 lines, `typeck_body.nova` 1,065,
`typeck_expr.nova` 665, `typeck_call.nova` 797, `typeck_method.nova`
857, `typeck_place.nova` 403, `typeck_closure.nova` 208,
`typeck_pattern.nova` 1,275, `usefulness.nova` 532, `typeck_for.nova`
438, `typeck_view.nova` 377, and the tests `typeck_body_tests.nova` 291,
`typeck_member_tests.nova` 195, `typeck_pattern_tests.nova` 394;
`fixes.nova` 758, `hir.nova` 765, `hir_dump.nova` 797, `typeck.nova`
1,134, `typeck_ty.nova` 486; N is 25,531 lines. N's `nova test`: 93
tests, all passing on both runtimes.

### 9. Inference variables in S0's order

`fresh` stands in N where S0 calls it, in the same order: 19 sites in
N (`typeck_call.nova` 5, `typeck_method.nova` 5, `typeck_expr.nova` 3,
`typeck_pattern.nova` 3, `typeck_closure.nova` 2, `typeck_place.nova`
1) against S0's 19 (`check.rs` 18, `patterns.rs` 1), each in the
function it mirrors, at the place S0's stands, so a `?N` in a message
and every type the dump prints number alike. N's own tests pin a
program's numbering (`typeck_body_tests.nova`), and every input above
compares them. **If wrong:** a call moved renumbers
every later variable of that function; the dump, which prints a type on
every node, shows it at the first node it reaches.

### 10. The differential, in its final form

`selfhost.rs`'s HIR tests, each through `run` and `settled_stdout`, S0's
`check --dump-hir` against N's `selfhost hir --std DIR`, byte for byte,
exit codes included, every compared dump well formed
(`assert_well_formed`, now by bytes):

- `the_hir_dumps_agree_on_std_the_corpus_and_the_cases`: std's three
  full parts (each with its functions, no program's id), the corpus
  natively (loose programs and package files), for the WASM target and
  under test mode, and the cases of each mode; each root's part is there
  and most reach the type checker; every form printed is listed, every
  form of `KINDS` and `BODY_KINDS` occurs and none of `LATER_KINDS`; the
  item passes' E0092 for the WASM target; each of the 38 codes occurs.
- `the_hir_dumps_agree_on_the_cases_with_windows_line_endings`: the
  cases of each mode as a Windows checkout has them; each part there,
  the codes those of the cases as they are.
- `the_hir_dump_has_a_header_per_root_and_std_its_own_part`,
  `the_hir_dump_flag_is_hidden_and_refused_as_the_resolution_dumps`
  (`--dump-hir-items` among the unknown arguments now),
  `every_run_of_a_hir_dump_settles_e0004s_paths_on_windows`,
  `the_well_formedness_check_refuses_what_it_should`,
  `n_checks_std_for_each_program_when_std_gives_an_item_diagnostic`,
  `both_sides_refuse_no_file` and
  `both_sides_take_test_in_every_position_alike`: 5.3a's, on the full
  dump.
- `n_checks_128_levels_of_every_kind_on_a_1_mb_main_thread` (§12), 5.3a's
  item test generalised; `the_type_fuzzers_item_inputs_agree_and_hold_each_aimed_code`
  (ADR 0055 §6, on the full dump); `the_body_fuzzers_inputs_agree_and_hold_each_aimed_code`
  (§11).

**Merged, each removing a run without removing a check:** 5.3a's
`the_hir_dumps_agree_on_std_the_corpus_and_the_cases` and part 1's
`the_full_hir_dumps_agree_on_std_the_corpus_and_the_cases` ran the same
inputs, one comparing the item part with N, one the full dump on S0
alone: one test now, with both sets of assertions (5.3a's floors, the
checked share, `LATER_KINDS` absent, the item E0092; part 1's std
functions, `BODY_KINDS`, the 38 codes). The two CRLF tests likewise, the
temporary directory `crlf-full-hir` gone. The header test's std-part
loop, a weaker copy of the corpus test's std checks, went. The type and
body fuzzers' bases are read from the loose corpus's compared dump
(`loose_hir`, once a run) rather than from an S0 run of their own; the
cases of each mode are dumped once a run (`hir_cases`) for the corpus and
CRLF tests; the forms are read on the thread that made each dump, each
distinct line once. The 38-code assertion holds on the cases and the
corpus alone, which is stronger than on the differential's inputs with
the fuzzers' (spec §4.6); the fuzzers' counts are reported beside it.

**Each check this part added, and what it guards** (for the plan's
mutants): the views' comparison (any line of `typeck_view.nova`, through
the cases' E0094 to E0097 and the corpus's components); the merged
corpus test's `BODY_KINDS` and 38 codes (a form or a code no input
reaches); `--dump-hir-items` refused as the base refuses it (the removed
form left half in place); the depth test's body kinds and its check that
only `items` and `optionals` give a diagnostic (a generator gone wrong,
or N's stack); the body fuzzer's checked-only, per-seed count, body-E0001
and floors assertions (a mutation that stops before the type checker, a
seed that writes fewer inputs, an item E0001 counted as a body's, an
aimed code no longer reached); the driver test's three unshared parts
(the same comparisons as before, by parts).

### 11. The body fuzzer

`fuzz::body_mutate` (spec §4.6) mutates the type fuzzer's bases (loose
corpus programs that import nothing and check clean) one to
three times inside their functions' and methods' bodies, by the tokens
there: an integer, a string or a `true` made a value of another type; an
argument added to a call, or its last one dropped; a field or a method
renamed (`fz_<name>`); a one-line `match` arm deleted; an integer given
`?` or `.await` (outside an `async fn`); a `let mut` made `let`; a
`let`'s annotation removed; `break` put first in a body; or one of
`fuzz::BODY_ITEMS` appended, 26 functions each aimed at a code (the
type checker's E0001 for a value and for a function, E0010, E0011,
E0013, E0014, E0015, E0016, E0020, E0021, E0022, E0060, E0080, E0081,
E0086, E0087, E0098, E0099, E0100, E0104), each name numbered from the
input's own. A mutation that would not parse is skipped; none renames
what an item declares, so every input reaches the type checker, which
the test asserts. **Seeds 501, 502 and 503**, 40 inputs each, in one
process a side. Counts over the fixed seeds: E0001 13 (all the body's,
by their messages), E0010 28, E0011 25, E0013 7, E0014 31, E0015 4,
E0016 33, E0020 25, E0021 4, E0022 4, E0060 12, E0080 15, E0081 14,
E0086 11, E0087 3, E0098 11, E0099 9, E0100 2, E0103 4, E0104 7: every
floor of two (E0001, E0010, E0011, E0013, E0014, E0015, E0016, E0020,
E0021, E0060, E0080, E0098, E0099) holds. The type fuzzer, now on the
full dump: E0001 32, E0010 28, E0011 13, E0012 53, E0013 5, E0015 10,
E0070 21, E0071 18, E0072 15, E0073 14, E0074 18, E0076 5, E0077 20,
E0078 21, E0105 9, E0403 23, E0900 48. Among the cases and the corpus
every one of the 38 codes occurs (E0098 and E0087 once each, E0086 and
E0094 twice; E0001 905 times).

### 12. The depth cases, and N's stack

`n_checks_128_levels_of_every_kind_on_a_1_mb_main_thread` runs N's
`selfhost hir` on 45 programs under `ulimit -s 1024` (Linux, macOS; on
Windows the main thread as it is), its dump S0's: 5.3a's ten written
nestings of a type in all twelve item positions; every kind of
`fuzz::DEEP_KINDS` at 128 levels (parentheses, blocks, `if`, `else if`,
`match`, `while`, `for`, calls, prefix and binary operators, postfix
chains, types, optionals, patterns, nested items, and a mixed one); and
`fuzz::BODY_DEEP_KINDS` at the deepest level the parser takes: closures
in closures, called at once, capturing and with parameters (64), arrays,
indexing, tuples, interpolation, method chains, `?`, a `let`'s
annotation, or- and `let` patterns, `match` in an arm and as a scrutinee
(128), tuple and variant patterns in a `match` arm and `for` over a
range, an array and with a tuple pattern (127). **Why not 128:** a
closure called at once, `(|| e)()`, is two of the parser's levels, its
parentheses and the closure, so 64 closures are 128 levels; a `match`
arm's pattern starts a level in, the arm's own, so 127 patterns nest
there; 127 `for` loops nest around the statement in the innermost, a
level of its own. Every program reaches the type checker and none but
`items` and `optionals` (E0900) gives a diagnostic. **N passes every
one on 1 MB on both runtimes**; nothing overflows, so answer 2's
departure is not needed here (ADR 0055 §12's normalized type stays the
one recorded).

Least stack (Linux, `ulimit -s`, 4 KB bisection, each run's dump
compared with S0's, 2026-11-12):

| Input | Levels | Debug runtime | Release runtime |
|---|---:|---:|---:|
| body `annotations` | 128 | 68 KB | 64 KB |
| body `arrays` | 128 | 204 KB | 200 KB |
| body `binary` | 128 | 44 KB | 44 KB |
| body `blocks` | 128 | 212 KB | 212 KB |
| body `calls` | 128 | 200 KB | 200 KB |
| body `closure-captures` | 64 | 200 KB | 200 KB |
| body `closure-params` | 64 | 200 KB | 196 KB |
| body `closures` | 64 | 200 KB | 196 KB |
| body `else-ifs` | 128 | 56 KB | 48 KB |
| body `for-arrays` | 127 | 216 KB | 208 KB |
| body `for-patterns` | 127 | 212 KB | 212 KB |
| body `for-ranges` | 127 | 212 KB | 208 KB |
| body `fors` | 128 | 216 KB | 212 KB |
| body `ifs` | 128 | 220 KB | 224 KB |
| body `indexes` | 128 | 204 KB | 196 KB |
| body `interpolations` | 128 | 196 KB | 196 KB |
| body `items` | 128 | 68 KB | 68 KB |
| body `let-patterns` | 128 | 56 KB | 52 KB |
| body `match-arms` | 128 | 208 KB | 204 KB |
| body `match-scrutinees` | 128 | 200 KB | 192 KB |
| body `matches` | 128 | 208 KB | 204 KB |
| body `method-chains` | 128 | 52 KB | 44 KB |
| body `mixed` | 128 | 160 KB | 160 KB |
| body `optionals` | 128 | 44 KB | 40 KB |
| body `or-patterns` | 128 | 52 KB | 56 KB |
| body `parentheses` | 128 | 200 KB | 192 KB |
| body `patterns` | 128 | 48 KB | 48 KB |
| body `postfix` | 128 | 48 KB | 48 KB |
| body `prefix` | 128 | 44 KB | 40 KB |
| body `questions` | 128 | 200 KB | 200 KB |
| body `tuple-patterns` | 127 | 200 KB | 196 KB |
| body `tuples` | 128 | 200 KB | 196 KB |
| body `types` | 128 | 60 KB | 60 KB |
| body `variant-patterns` | 127 | 196 KB | 196 KB |
| body `whiles` | 128 | 212 KB | 212 KB |
| item type, `arrays` | 128 | 60 KB | 48 KB |
| item type, `fn-parameters` | 128 | 52 KB | 52 KB |
| item type, `fn-returns` | 128 | 52 KB | 52 KB |
| item type, `generics` | 128 | 68 KB | 60 KB |
| item type, `mutable-references` | 128 | 56 KB | 48 KB |
| item type, `optionals` | 128 | 44 KB | 40 KB |
| item type, `parentheses` | 128 | 52 KB | 52 KB |
| item type, `pointers` | 128 | 56 KB | 48 KB |
| item type, `references` | 128 | 56 KB | 52 KB |
| item type, `tuples` | 128 | 48 KB | 52 KB |

The least stacks are the parser's: up to 224 KB for nested expressions
and blocks (200 to 216 KB in part 4), 40 to 68 KB for patterns, chains,
types and items; the release runtime within 8 KB of the debug one.

### 13. N's memory and GC pauses

ADR 0055 §13's method: wall best of three and peak RSS (`wait4`), GC
pauses from `nova_gc_collect_roots` to the return of its caller, after
the sweep, under gdb, and `NOVA_GC_DEBUG`'s collections and largest live
heap (2026-11-12, a script, not committed: `heap.sh`, with 5.3a's
`meas.py` and `gcpause.py`):

| Runtime | Input | Wall | Peak RSS | Pauses (median / p95 / max, total) | Collections, max live |
|---|---|---:|---:|---|---:|
| debug | `hir --std-part` | 0.84 s | 33.4 MB | 9.4 / 59.5 / 91.4 ms, 326 ms | 14, 13.5 MB |
| debug | `hir --target wasm --std-part` | 1.02 s | 35.3 MB | 13.8 / 83.6 / 126.7 ms, 451 ms | 15, 17.8 MB |
| debug | `hir --test --std-part` | 0.82 s | 33.4 MB | 10.4 / 63.5 / 85.3 ms, 312 ms | 14, 13.6 MB |
| debug | `hir` N's root | 5.16 s | 152.0 MB | 45.0 / 332.2 / 459.6 ms, 2,048 ms | 21, 46.7 MB |
| debug | `hir` the loose corpus | 9.46 s | 43.9 MB | 64.0 / 90.6 / 109.1 ms, 3,649 ms | 60, 12.1 MB |
| release | `hir --std-part` | 0.17 s | 32.6 MB | 2.2 / 13.1 / 19.7 ms, 74 ms | 14, 13.5 MB |
| release | `hir --target wasm --std-part` | 0.21 s | 34.5 MB | 3.9 / 20.0 / 27.9 ms, 106 ms | 15, 17.8 MB |
| release | `hir --test --std-part` | 0.17 s | 32.6 MB | 2.3 / 13.5 / 17.9 ms, 71 ms | 14, 13.6 MB |
| release | `hir` N's root | 1.11 s | 151.1 MB | 7.0 / 78.4 / 98.7 ms, 468 ms | 21, 46.7 MB |
| release | `hir` the loose corpus | 2.01 s | 42.8 MB | 14.1 / 22.2 / 29.7 ms, 816 ms | 60, 12.1 MB |

Against ADR 0055 §13 (5.3a's item stage): std's part's live heap goes
from 3.9 to 13.5 MB (3.5×), N's root's from 13.6 to 46.7 MB (3.4×). ADR
0054 §10 projected pauses of 0.5 to 1 s on the debug runtime at five to
ten times 5.2's live heap; the longest measured is 0.46 s, on N's root,
and 0.13 s on std's parts; the release runtime's longest is 0.10 s. The
debug runtime spends 39% of the loose corpus's run (3.6 of 9.5 s) in 60
collections. Recorded, no threshold (spec §4.6).

### 14. The time, and CI's projection

Local, debug S0, 4 cores (2026-11-12):

- **`selfhost.rs`, three runs:** on the debug runtime 98.66, 94.16 and
  95.71 s (54 tests; part 4: 115 to 117 s with 56); on the release
  runtime CI uses (`NOVA_EMBED_RUNTIME=1`) 43.07, 47.41 and 45.10 s
  (user 142 to 157 s). The base, `e5c555d0` (5.3a), on the release
  runtime the same day: 19.38, 20.83 and 21.84 s (52 tests).
- **The slowest tests,** each run alone (N's build, about 5.5 s, in
  each): on the release runtime the corpus test 24.8 s (70 s of CPU),
  the CRLF test 18.8 s, the resolution dump's corpus test 11.4 s, the
  type fuzzer 10.1 s, the depth test 9.5 s (the body fuzzer 8.6 s); on
  the debug runtime the corpus test 55.1 s, the CRLF test 50.3 s, the
  resolution dump's 21.0 s, the type fuzzer 18.6 s, the mutated corpus
  15.0 s. Of the corpus test's process time, N's 44 package roots, each a
  dump of most of N, take 26 s (S0 10.3 s, N 15.6 s on the release
  runtime), the files with tests 5.7 s, `cases/check/` 10.7 s.
- **The Test job:** `cargo test --workspace` gives 2,832 passed, one
  failure (the container's IPv6 test, as at the base) and 10 ignored, as
  before part 5; its slowest is `nova-driver`'s unit tests, 61.6 s, the
  equivalence test (§4) 47 s of it (95 s before its unshared pass went
  to threads of its own), then `nova-typeck`'s 589 tests in 23.4 s.

**CI, projected (not measured):** 5.3a's CI run 42774093193 (its
release-runtime job, at `cc38ac37`, 5.3a's tree) gave 37.10, 27.53 and
55.38 s, against the base's 20.7 s here: factors 1.79 (Linux), 1.33
(macOS) and 2.68 (Windows), which put 5.3b's `selfhost.rs` at about **81
s on Linux, 60 s on macOS and 121 s on Windows**, past the lines (60 s,
60 s, 70 s). Per answer 3 the user decides, with CI's figures.

**Options measured, none applied** (each `selfhost.rs` on the release
runtime, two runs):

| Option | Coverage | Local | Projected Linux / macOS / Windows |
|---|---|---:|---:|
| none (this branch) | all | 43.1-47.4 s | 81 / 60 / 121 s |
| (a) of N's own package roots and files with tests, `selfhost/src/main.nova` alone in the HIR test | a cut: 43 of N's 44 package roots and 8 of its 13 files with tests leave the HIR dump's corpus (the resolution and AST dumps keep them) | 32.3, 35.5 s | 61 / 45 / 91 s |
| (b) S0 built at `opt-level = 1` in the Selfhost job (`CARGO_PROFILE_DEV_OPT_LEVEL=1`) | none: the same source, debug assertions and overflow checks kept | 30.4, 28.3 s; the workspace's build 1 min 46 s and the test's 61 s locally | 53 / 39 / 79 s |
| (a) and (b) | as (a) | 23.9, 22.6 s | 42 / 31 / 62 s |

Not measured: N and S0 parsing a package's modules once per process for
all of its roots (no output changes; about a quarter of a package root's
time on each side).

**CI's figures:** _(placeholder: the main session fills this from the
measurement run on `claude/loving-archimedes-frtf5q-53b-ci`: `selfhost.rs`
per system, the Test job's totals, and the user's answer.)_

### 15. Departures from the spec, and decisions it does not settle

- **(differs) The closure-id rule** (§3) is the dump's: a program's
  closures print `u<D+k>` and std's `s<S+k>`, and the names hold the
  printed id (`closure$u7`), where spec §4.1 says "`u<n>`, names as they
  are". S0's own ids and symbols are §1's.
- **(differs) The depth cases** reach 128 written levels but for
  closures (64), patterns in a `match` arm and `for` loops (127), by the
  parser's count (§12).
- **(differs) The 38-code assertion** holds on the cases and the corpus
  alone, not on the differential's inputs with the fuzzers' (§10):
  stronger.
- **(differs) Witnesses' strings and characters** approximate Rust's
  `{:?}` (§8, part 4).
- **(differs) 5.3a's item-only dump** (`check --dump-hir-items`) existed
  only while 5.3b was built and is gone; the full dump holds the item
  part, and the driver's equivalence test compares the full dumps alone
  (§4).
- **CI past the lines** (§14): the user's, per answer 3; nothing cut.
- **N checks std's bodies on the checker its tables come from**, where
  S0 collects std's items a second time (part 4); the fallback re-resolves
  a program in full on both sides (§4).
- **The body fuzzer's bases** are the type fuzzer's, now read from the
  full dump, so a base checks clean in its bodies too; the type fuzzer's
  inputs changed with it (its floors hold).

### 16. The rule

From 5.3b the whole type checker exists twice, in `nova-typeck` and in
`selfhost/`: a change to either (a pass, the order of bodies, a
diagnostic's words, order, notes, labels or fixes, a `fresh` call, a HIR
node, std's sharing or its fallback) changes both in one pull request,
and the full dump's differential (§10) holds them equal (phase plan
decision 6, ADR 0055 §14 extended). The dump's grammar (§2) changes with
them, on both sides.

## Consequences

- 5.4 starts from the HIR the full dump shows, the same on both sides
  for every input the differential holds.
- `selfhost.rs` doubles locally on the release runtime (21 to 45 s) and
  projects past CI's lines; the user decides among the measured options
  (§14), with CI's figures.
- The bug of §4 (std's bodies see a program's impls) stays in S0, worked
  around by the fallback, to be fixed later in both compilers at once.
- N on a 1 MB stack meets the depth cases with room (at most 224 KB),
  and the debug runtime's pauses stay under half a second.
