# Phase 5.3, "The type checker and HIR": design

- **Date:** 2026-11-06.
- **Status:** approved by the user (2026-11-06).
- **The user's decisions, 2026-11-06:**
  1. **Two pull requests, split by stage: 5.3a, then 5.3b.**
     - **5.3a, the items:** signatures, records, sums, traits, impls,
       externs and the item-level errors (about 2,900 lines), with a dump
       of the tables over std and the whole corpus.
     - **5.3b, the bodies:** inference, expressions, patterns,
       exhaustiveness and closures (about 6,400 lines), with the full HIR.
  2. **std is shared, and CI is measured before the plan.** Both sides
     type-check std once per process and mode, as 5.2b's option B
     resolves it, from the start. Real CI figures are taken from the
     prototype before the plan is written, and the user is asked again
     with them.
  3. **The dump is the full HIR:** the tables, the signatures, and every
     function with its locals and its expression tree, with types and
     spans.
  4. **S0 checks methods in `DefId` order,** and std's closures take ids
     in a segment of their own, so std's part of the dump prints once.
     The CHANGELOG records it.
  5. **Diagnostics are compared completely, fixes included:** code,
     severity, span, message, notes, secondary labels and fixes.
     `fixes.rs` is ported in 5.3b.
  6. **The inline tests become cases:** about 527 tests (637 sources),
     `format!` and loops expanded, become `.nova` files both compilers
     check, as 5.2 did for the resolver. About 46 internal tests stay
     Rust-only.
  7. **A new type-level fuzzer** mutates programs that check (literal
     types, argument counts, field names, deleted match arms, `?` on a
     value that is not an `Option`, …), with floors so that every main
     code occurs, within the CI time budget.
  8. **The language server's probe and index are not ported;** they stay
     S0's.
- **Not asked; the recommendation taken, which the user can overturn:**
  - the HIR dump is separate from the resolution dump, and S0's flag is
    `nova check --dump-hir`, N's command `selfhost hir` (§3.3);
  - S0 checks the program's bodies before std's, which gives decision 4's
    segment with no new id rule; the change lands in 5.3b (§4.1);
  - the part of `fixes.rs` the item passes call is ported in 5.3a, so
    that 5.3a's diagnostics are complete as decision 5 asks (§3.2);
  - floats print as 5.1's dump prints them, the 16 hex digits of
    `to_bits` (§4.3);
  - the dump shows the type checker's output before the driver's
    `keep_the_entry_main` (§3.3);
  - the fuzzer's item mutations come in 5.3a, its body mutations in 5.3b.
- **Branch:** `claude/loving-archimedes-frtf5q-53`, from `main` at
  `2a67c6ce` (5.2b's merge), for this spec. Each part is built on a
  branch of its own from `main`; its plan names it.
- **Inputs:** `docs/phase-5-plan.md` (§3 decisions 4, 6, 8 to 10, §4's 5.3
  and 5.4 entries, risks 1, 2, 3 and 5); the research for this spec
  (scratch only, summarised in §2); ADRs 0053 and 0054 and the 5.2b
  plan's decisions 10, 18 and 24; the 5.2 spec.

## 1. What 5.3 delivers

N's fifth stage, after the resolver, is a **type checker** that builds
what S0's `nova_typeck::check_with` builds:

1. the item tables: records, sums, traits, impls, externs, and every
   function's, method's and constant's signature (5.3a);
2. HIR: every function, method, constant, closure and wrapper, with its
   locals and its typed expression tree (5.3b);
3. every diagnostic in S0's order, fixes included.

A canonical **HIR dump** on both sides pins it, on std, the corpus, the
cases and the fuzzer's inputs. From 5.3a item collection exists twice,
from 5.3b the type checker; a change to either changes both compilers in
one pull request (phase plan decision 6).

**The phase plan is amended (in 5.3a).** It splits 5.3 by feature
(inference, expressions, patterns and exhaustiveness; then traits,
associated types, generics and async). That split cannot check one
corpus program: every program type-checks all of std (§2 item 2), std's
bodies need the trait tables, impl lookup and `Iterator`
(`check.rs:5201`), and even a user body needs std's typed impls (`for`,
method calls, `?` on core's `Option`). Its first half would need a
std-less mode in both compilers and a case set of its own. Item
collection depends on nothing in bodies, so a split by stage checks all
of std and every root at each step. The amendment also corrects the
sizes: 9,246 code lines and 585 tests, not 7,404 and 394.

## 2. The starting point

Measured on `main` at `2a67c6ce` (tree `edb8bead`), 2026-11-06.

1. **Sizes,** in code lines as 5.1 and 5.2 counted them: `nova-typeck`
   9,246 with 585 inline tests (`check.rs` 7,197, `check/patterns.rs` 701,
   `check/fixes.rs` 372, `check/view.rs` 344, `usefulness.rs` 345,
   `infer.rs` 157, `lib.rs` 130); `nova-hir` 810 with 30 tests. About
   10,056 lines to port, about all of N today (9,850 in 23 files).
2. **Every program type-checks all of std.** `check_with`
   (`check.rs:141-246`) runs these passes; pass order is emission order
   (nothing sorts diagnostics, `check.rs:183-201`):
   (1) `note_item_declarations`, the language server's index only;
   (2) `reject_self_type_params`; (3) arities, supertraits, records, sums,
   traits; (4) `collect_impls`, ending with the method signatures,
   coherence, hidden methods and supertrait impls (`check.rs:1549-1556`);
   (5) `collect_signatures` (constants' declared types too,
   `check.rs:2434-2497`), `collect_externs`; (6) the bodies
   (`check.rs:210-227`): `defs.functions()` in `DefId` order,
   `method_locs.keys()`, `const_ids()`; (7) `check_const_cycles` (E0081),
   `warn_newline_splits` (E0103); (8) the lifted closures and wrappers
   appended (`check.rs:231`). std brings 330 functions, 263 methods, 12 traits, about 100 impls and
   47 async functions.
3. **One hash order reaches output:** methods are checked in `FxHashMap`
   order (`check.rs:217`; a probe gave `A.m7, m4, m1, m8, m5, m2, m6,
   m3`, stable over three runs). It decides the order of diagnostics in
   methods, of `hir::Module`'s functions, and closure and wrapper ids and
   names (`closure$N`, `fnval$N`, `check.rs:5682`, `:5758`), which reach
   symbols. No other hash iteration reaches output.
4. **Closure ids** start past every definition, std's included
   (`check.rs:159`, `:3454`). Bodies go user functions, std functions,
   methods of both mixed, constants, so std's closures are numbered among
   the user's and depend on the program.
5. **Inference variables** are numbered per function in the order
   `InferCtx::fresh` is called (19 sites) and reach messages as `?N`
   (`nova-typeck/src/lib.rs:127`: "found `Option<?0>`"). `unify` is not
   transactional: a failed unify keeps its partial bindings.
6. **Nothing prints HIR or types** but `#[derive(Debug)]`. HIR names
   everything by `DefId` (a variant or field by index), never by a
   table's index. The driver renames non-entry `main`s after the type
   checker (`nova-driver/src/lib.rs:562`); MIR finds `main` by name and
   works from a worklist (`nova-mir/src/mono.rs:95`).
7. **Diagnostics:** 154 sites over 38 codes; E0010 33, E0001 20 (one in
   `view.rs`), E0014 13, E0072 8, E0900 8 direct and 33 `unsupported`
   calls, E0092 1 direct and 12 `missing_on_target` calls; 27 notes, 2
   secondary labels (both E0074), 25 fix sites, which need the source
   text (`CheckOptions::sources`, which `check` passes,
   `nova-driver/src/lib.rs:1152`). Mode-only: WASM's E0092 and `"js"`
   ABI E0900, test mode's E0099 note.
8. **The corpus exercises typing, not type errors:** of 318 native roots
   (251 loose, std's 25 files among them, and 67 package files) 265 check
   clean, and all 39 WASM roots; 9 files hold tests. Type errors come only
   from three deliberate fixtures and 21 std files checked as programs.
9. **The tests' sources:** `check_src` (`check.rs:9548`) and its kin
   resolve a Rust string with the 21 native std modules, with no loader
   and no `@test` stripping. Of 585 tests, 446 assert diagnostics only,
   81 diagnostics and HIR, 2 HIR only, 56 feed no source. About 527 (637
   calls) can become cases: 35 use `format!`, 31 loop, 3 need two
   modules, 8 assert fixes.
10. **Time:** S0 spends 0.024 s type-checking std per program (debug), a
    third of a hello `nova check`. N's resolver runs 5.5 to 6.5 times
    slower than S0's. CI run 42171538582 took `selfhost.rs` 2.57 times
    its local time on Linux (52.6 s against 20.5 s), 2.60 on macOS, 3.00
    on Windows.
11. **N's language:** no blocker, three idioms. `==` on a user sum is
    E0013, so N writes `ty_eq`; a tuple cannot be a `Map` key, and
    `Map<Int, _>` works; a record field set through a copy taken from a
    sum's payload is seen through the original, so HIR nodes as records
    can be rewritten in place, as `finalize_expr` does.
12. **Stack:** S0 runs its pipeline on a 64 MB thread
    (`nova-driver/src/lib.rs:350`), N on the main thread, 1 MB on
    Windows. The parser caps nesting at 128 levels (ADR 0052); the type
    checker recurses about as deep, with larger frames. Not measured.

## 3. 5.3a, the items

### 3.1 What 5.3a ports

`check_with`'s passes 2 to 5, function by function, with the same names,
so a reviewer can read the two side by side:

- `reject_self_type_params` and the generic-parameter checks;
- `collect_type_arities`, `collect_supertraits`, `collect_records`,
  `collect_sums`, `collect_traits`;
- `collect_impls` with its checks: conformance, method signatures
  against the trait's, coherence, hidden methods, supertrait impls,
  associated-type cycles;
- `collect_signatures` (an async function's `Future<T>`, constants'
  declared types) and `collect_externs` with its FFI checks;
- bounds and `where` (`resolve_bounds`, `apply_where`, `expand_bounds`);
- all of `convert_ty`, projections, associated types and `normalize`
  (`check.rs:2747-3243`), which items use whole and which needs no
  function context;
- `display_ty` and the message helpers the item passes use.

Not in 5.3a: pass 1 (decision 8), the bodies, and passes 7 and 8, which
read what bodies record. N checks no body in 5.3a; the dump has none.

S0 gains `nova_typeck::check_items`, passes 1 to 5 returning the tables
and the item diagnostics, which `check_with` calls first: a pure
refactor, `check_with`'s output unchanged.

**HIR's types:** `hir.nova` ports `Ty` (18 variants, `Var` included),
`TyHead`, `TraitDef`, `TraitMethod`, `ImplInfo`, `RecordType`,
`RecordField`, `SumType`, `Variant`, `ExternFn`, `ExternAbi`, the
module's tables, and `nova-hir`'s logic (`subst`, `match_pattern`,
`normalize_ty` with its limits, `self_types_overlap`).

### 3.2 Diagnostics in 5.3a

Every diagnostic of the item passes, compared completely, 16 codes:

- **E0001** at 7 sites: "cannot find trait" (`check.rs:753`, `:1276`,
  `:2669`, `:2731`), "cannot find type" (`:2974`), two "no associated
  type" forms (`:2852`, `:3049`). The research counted 6; 7 is the count
  by line range, measured for this spec.
- **E0010** (`:1231`, `:1294`), **E0012** (5, in `convert_ty`),
  **E0015** (`:3057`), **E0070** to **E0074**, **E0076**, **E0077**,
  **E0078**, **E0403** (6), **E0105** (2, a warning);
- **E0092** (5 `missing_on_target` calls) and **E0900** (7 sites and 13
  `unsupported` calls) in the item passes.

The item E0001s carry fixes, so 5.3a ports `push_fix`, `name_fixes` and
what they call (`fixes.rs:152-397`, 207 code lines: the import fix and
its edit, which needs the source text, and "did you mean");
`suggest::closest` is 5.2's. The item diagnostics are the prefix of
`check`'s that passes 2 to 5 emit, after the loader's and the resolver's.

### 3.3 The HIR dump, on both sides

- **S0:** `nova check --dump-hir FILE…` dumps each program's part, `nova
  check --dump-hir --std` std's part; both take `--target` and `--test`,
  as `--dump-resolution` does (ADR 0054).
- **Hidden, by ADR 0053's mechanism:** taken only when `--dump-hir` is
  the first argument after `check`; `main` looks for it before clap, and
  the hidden form's own parser (`DumpResolutionCmd`, generalised to
  either flag) takes the rest. `--help`, clap's tips and every other
  `nova check` command line stay the base's, byte for byte.
- **N:** `selfhost hir --std DIR [--target wasm] [--test] FILE…`, or
  `--std-part` in place of the files.
- **What runs:** `check`'s pipeline: the package graph, the loader, the
  stop on an error, `@test`s stripped (kept under `--test`), the
  resolver, the stop on an error, then the type checker with `check`'s
  options (sources, no probe, no index). The dump shows its output as
  returned, before `keep_the_entry_main` (5.4's).
- **Separate from the resolution dump (not asked):** 5.2's dump already
  compares resolution on these inputs, and repeating it would double the
  output (std's resolution part is 7,728 lines) and its time; each
  grammar keeps one owner ADR; a divergence shows in its own stage. The
  HIR dump repeats only `(StdStart …)` and the earlier stages'
  diagnostics: a program that stops before the type checker prints
  `checked=false` and those.
- **The format:** an indented S-expression as 5.1's and 5.2's, written
  in full in ADR 0055, one example per form. 5.3a's part holds, in
  order: `(StdStart module= def= item=)`; records and sums (generics,
  bounds, field types); traits (supertraits, associated types, each
  method's `has_self`, `mut_self`, generics, bounds, signature and
  default); impls (trait, self head, self type, generics, bounds, each
  method's name and `DefId`, associated-type bindings); every signature
  by `DefId` (generics, bounds, parameter and return types, no `self` or
  `mut self`); externs (symbol, ABI, signature); then every diagnostic,
  in 5.2's form. Tables print in collection order; a map only sorted.
- **Ids and types:**
  - a definition is `u<n>` or `s<n>` (std-relative), as in the
    resolution dump; an impl, which has no `DefId`, is numbered within
    its part;
  - a type prints by structure, with ids, not display names: `Int`,
    `[T]`, `(A, B)`, `fn(A) -> B`, `Future<T>`, `T0`, a record or sum as
    its id and arguments, a projection as its type and the associated
    type's id, `!`, the error type;
  - an inference variable prints as S0 prints it, `?N`, wherever one
    reaches output.
- **Two parts:** std's part, once per target and mode (native, WASM,
  test), holds std's tables and signatures; a program's part only its
  own. A unit test checks that std's part of a two-module program's
  check is std's part of a program of no module.

### 3.4 std shared, at the item level (decision 2)

- **Shared, per process and mode, on both sides:** std parsed (5.2a),
  resolved (5.2b's `SharedStd`) and collected once, from the check of a
  program of no module of its own.
- **Per program:**
  - its modules are resolved by 5.2b's shared path, extended so that the
    type checker can read std's definitions through it;
  - the type checker reads std's tables with std's ids rebased past the
    program's own, as `Exports::rebased` rebases std's exports;
  - each pass runs over the program's items; the checks that pair one
    with std's (conformance to a std trait, coherence and E0074, hidden
    methods and E0105, supertrait impls) pair it with std's rebased
    tables as the full path does. std's items are not collected again.
- **The fallback:** if checking std alone gives a diagnostic (none does
  today), every program is checked in full on both sides, as in 5.2b.
- **The guard:** the driver's test compares S0's shared and full dumps
  byte for byte over every corpus root and case of each mode, in order
  and reversed, as 5.2b's `std_resolved_once_…` test does; the
  differential compares N's shared dumps with S0's.
- **Only the dump shares:** `nova check`, `build`, `run`, `test` and the
  language server keep `check_with` on the full resolution.
- **If the rebasing costs more than it saves,** the prototype measures
  the narrower form (std parsed once, resolved and collected per
  program) beside it, and the user is asked with both figures.

### 3.5 N's code

- **Where:** `selfhost/src/hir.nova`; `typeck.nova` (state,
  `check_items`, arities, supertraits, records, sums);
  `typeck_traits.nova` (traits, impls, their checks); `typeck_ty.nova`
  (`convert_ty`, `normalize`, bounds, signatures, externs); `fixes.nova`;
  `hir_dump.nova`; `selfhost hir` in `main.nova`. The plan may cut them
  otherwise, none past about 1,500 lines (N's largest file today).
- **Idioms:** `ty_eq` and its kin where Rust compares `Ty` with `==`; a
  composite key, `(item, method)` or a `Span`, encoded as one `Int`, and
  a `DefId` as an `Int`; a `Map` for lookups only, a `Vec` or an
  insertion-ordered map for anything iterated into output (phase plan
  decision 9); 5.2's `Diagnostic` record.

### 3.6 The harness

`selfhost.rs` gains a fourth kind of dump, `Dumps::Hir`, compared byte for
byte with exit codes through `compare_batch`:

- **std's parts:** native, WASM and test, side by side, once each.
- **The corpus:** 318 native roots, 39 WASM programs for the WASM target,
  9 files with tests under test mode; one process a side per set and
  mode, as 5.2b.
- **The cases,** extracted by a temporary instrumentation of `check_src`
  and its kin while the type checker's tests run (a script, not
  committed), as 5.2b extracted the resolver's: every source a test
  checks, `<testfile>__<test>[_n]`, so each `format!` or loop gives one
  file per source; a two-module test as a directory rooted at
  `main.nova`; a source holding `@test` in test mode's set, since `check`
  strips what it tests. They go in `cases/check/`, with hand cases for
  the mode-only diagnostics in `cases/check-wasm/` (E0092, the `"js"`
  ABI) and `cases/check-test/` (E0099's note); none needs `fn main`; all
  again with `\r\n`, as 5.2's.
- **The type fuzzer,** `fuzz::type_mutate`, item mutations in 5.3a: a
  parameter's or field's type, a generic's arity, a bound naming a
  missing or wrong trait, an impl's method dropped, added or re-typed, a
  duplicate generic, an associated type unbound or cyclic, overlapping
  impls, an extern with an unsafe type. A mutation that would not parse
  or resolve is skipped, so every input reaches the type checker, which
  the harness asserts. Fixed seeds of its own; a floor of two on each
  code its mutations aim at.
- **Windows paths:** the HIR dump prints E0004's and E0093's messages
  when a program stops in the loader, so `settled_stdout`, in `run`,
  settles S0's `check --dump-hir` and N's `hir` as it settles the
  resolution dump (ADR 0054 §7). No type checker message prints a path
  (checked for this spec).
- **Refusals,** on both sides: the hidden form's (no root, a root and
  `--std`, `--help`, an option twice, `--test --target wasm`), and an
  unreadable root (`IOERR`); a manifest with an M-code is S0's alone, as
  in 5.2b, and no input holds one.
- **Every code occurs:** each of 5.3a's 16 codes, asserted.

### 3.7 Measured in the prototype, before the plan

- **CI's time (decision 2):** with the user's word before the push, the
  prototype goes to a separate measurement branch with a draft pull
  request, closed after, as in 5.2b. `selfhost.rs`'s figures on all three
  systems are recorded before the plan is written, and the user is asked
  again with them, and whenever Windows passes 70 s (5.2b's rule).
  Coverage is never cut without asking.

  The research projected all of 5.3, std shared, at 69 to 90 s on Linux,
  69 to 91 s on macOS and 81 to 106 s on Windows; even with N's type
  checker twice as slow as S0's, Windows passes 70 s. 5.2b's last local
  projection undershot CI by about half (Linux 24 to 25 s projected,
  37.90 s measured), and today's CI-to-local factor (2.6 to 3.0) is not
  5.2b's (about 1.15): the projection is a floor.
- **N's stack:** the item passes on types nested 128 levels on a 1 MB
  stack, on CI's Windows leg and locally under a 1 MB limit. If N
  overflows where S0 does not, the user is asked.
- **N's memory:** peak RSS and GC pauses on std's part, with the debug
  and a release runtime, as ADR 0054 §10; recorded, no threshold.

### 3.8 Gate for 5.3a

- std's three item parts, every corpus root's item dump (natively, for
  the WASM target and under test mode) and the cases (`\n` and `\r\n`)
  are identical on all three systems.
- The fixed seeds' item inputs give identical dumps; each aimed code
  occurs twice or more, and each of the 16 codes somewhere.
- The shared and full dumps are equal, and std's part does not depend on
  the program.
- `nova check` over every tracked file prints what the base's prints,
  byte for byte; the hidden flag is in no help or usage line.
- CI is measured and the user has answered; N's stack and memory are
  recorded.
- The full suite, fmt, clippy, the MSRV job, `nova fmt --check` over
  `selfhost/`, and `git diff` on `Cargo.lock` are clean.

## 4. 5.3b, the bodies

### 4.1 S0's order of bodies (decision 4, a small change)

- `check_with` checks the program's bodies first, then std's, each in
  `DefId` order: functions, methods (their ids sorted, `check.rs:217`),
  constants. No crate is added.
- So closure and wrapper ids come in two segments past every definition,
  the program's then std's, and std's part needs no new id rule: it is
  the check of a program of no module, where std's closures take `s<n>`
  ids right after std's definitions, in every run alike; a program's
  part shows only its own closures, `u<n>`, names as they are.
- **Why the program's first:** std's closures then follow all of the
  program's, so skipping std's bodies (§4.5) changes no id a program
  prints. With std first, every program's closure ids would shift by
  std's count, and std's names would need rewriting in the dump.
- **What a user sees:** diagnostics inside methods in declaration order,
  and closures' and wrappers' names renumbered, in symbols only. std
  gives no diagnostic in any program, so no other order changes.
- A test pins it (diagnostics in eight methods in declaration order;
  std's closures after the program's). The CHANGELOG's Changed records
  it, ADR 0056 why.

### 4.2 What 5.3b ports

The rest of `nova-typeck`, function by function: the bodies
(`check_function`, `check_method`, `check_const`, blocks,
`finalize_function`); `infer.rs` (`fresh`, `shallow_resolve`, `unify`
with its occurs check, wildcards and partial bindings); interpolation,
paths, calls, builtins (`builtin_signature`), variants, arrays and
indexing; `for` and the iterator protocol; `?`; closures, lifting and
wrappers; record literals, fields, method lookup and calls, operators,
assignment and places; `match`, `check/patterns.rs`, `usefulness.rs`,
witnesses and exhaustiveness; `check/view.rs`; async bodies (`.await`,
E0086, E0087); E0081 and E0103; the rest of `fixes.rs` (about 165 code
lines). About 6,400 lines. Not ported: the probe (`check.rs:6050-6239`),
the index (`:8255-8445`), and their 64 `note_*` and 17 probe calls
(decision 8).

**Inference variables in S0's order:** N calls `fresh` where S0 does, in
the same order, since `?N` reaches messages; each of the 19 sites keeps
its place in the code it mirrors.

**HIR's bodies:** `hir.nova` gains `Function`, `Local`, `LocalId`,
`Expr` (36 kinds), `Callee`, `BinOp`, `UnOp`, `Arm` and `Pattern` (10
kinds). An `Expr` is a record (kind, type, span), so finalizing and
remapping locals rewrite it in place (§2 item 11).

### 4.3 The full dump

5.3a's part, plus every function, before the diagnostics:

- **Order:** a part's own functions in `hir::Module`'s order, which §4.1
  fixes: functions, methods and constants by `DefId`, then closures and
  wrappers in the order they were lifted.
- **Each function:** id, name, generics, bounds, `takes_env`,
  `capture_count`, parameter count, return type, `is_async`, place; its
  locals (`l<n>`, name, type, `mut`, span); its body, a tree of each
  node's kind, type and span, with its callee (`Def`, `Builtin` or
  `Local`), type arguments, a trait call's self type, and its patterns.
- **Spans and literals:** a function names its file once by its label
  (5.2's), its nodes print `start..end`; strings are escaped as 5.1's,
  floats are the 16 hex digits of `to_bits`, as 5.1 prints them (Rust's
  `{:?}` and Nova's text differ on `1e21`; bits are exact).
- **Size:** std's AST dump is 28,597 lines; std's HIR part, a type on
  every node, is likely larger (not measured). It prints once per mode.

### 4.4 Diagnostics, all of them

All 38 codes and 154 sites, with the 27 notes, 2 secondary labels and
25 fix sites, compared completely (decision 5). The E0001s include the
type checker's "cannot find `x` in this scope" (`check.rs:4124`,
`:7327`), which 5.2 left to 5.3.

### 4.5 std's bodies, shared

- std's bodies are checked once per process and mode, in std's part; a
  program's check skips them, after its own (§4.1). A program's bodies
  read std's tables (§3.4), never std's bodies, so skipping them changes
  nothing it prints. The fallback and the guard are §3.4's, over the
  full dumps.
- **To find:** an item of a program that changes how a std body checks
  (an impl on a std type that a std body's method call then finds) would
  make std's part depend on the program. The prototype searches for one
  and the guard compares every input; if one exists, the user is asked.

### 4.6 The harness, completed

- std's parts, the corpus and the cases compare the full dump; the cases
  include the 83 tests that assert on HIR and the 8 that assert fixes.
- **Depth cases:** programs at 128 levels of each nesting kind that
  type-checks (blocks, `if`, parentheses, calls, method chains, closures
  in closures, patterns, types).
- **Body mutations:** a literal's type, an argument added or dropped, a
  field renamed, a match arm deleted, `?` on a value that is not an
  `Option` or a `Result`, an assignment to an immutable binding, a
  missing method, `break` outside a loop, `.await` outside an `async fn`,
  an annotation removed. Floors of two on E0010, E0011, E0013, E0014,
  E0015, E0016, E0020, E0021, E0060, E0080, E0098, E0099 and the body
  E0001s; each of the 38 codes occurs in the differential's inputs.
- **Measured, as §3.7:** CI on the measurement branch (the user's word
  before the push, the user asked with the figures); N's stack on the
  depth cases on 1 MB; N's memory and pauses on std's full part (ADR
  0054 §10 projects pauses of 0.5 to 1 s with the debug runtime at a
  live heap five to ten times 5.2's).

### 4.7 Gate for 5.3b

- std's three full parts, every corpus root's full dump (in the three
  modes) and the cases (both line endings) are identical on all three
  systems.
- The fixed seeds' item and body inputs give identical dumps; the floors
  hold, and each of the 38 codes occurs.
- S0's new order is pinned by a test; `nova check` over every tracked
  file prints what the base's prints but for the order of diagnostics in
  methods, each file that differs listed; the golden runtime tests pass
  unchanged.
- N passes the depth cases on a 1 MB stack, or the user has ruled.
- CI is measured and the user has answered; 5.3a's checks hold.

## 5. New diagnostic codes

None. 5.3b reorders the diagnostics inside methods, nothing else.

## 6. Testing

- **Unchanged rules:** tests never touch the internet; fixed temporary
  names; `$NOVA_HOME` fresh where needed; MSRV 1.78, no new crate,
  `Cargo.lock` unchanged; no `clang` needed.
- **5.3a:** §3.6's differential; the shared-against-full test and std's
  part's independence; `check_items` against `check_with`; the hidden
  flag hidden; N's own `@test`s for the item passes' edges.
- **5.3b:** §4.6's additions; S0's new order; N's own `@test`s for
  inference, patterns and usefulness, beside the Rust tests that stay
  (about 46: `infer.rs`, `usefulness.rs`, the builtin tables).
- **CI time:** §3.7. The 5.1 and 5.2 differentials stay as they are.

## 7. Crates and files

- **5.3a:** `nova-typeck` (`check_items`, a `dump` module, std's shared
  state for the dump); `nova-resolver` (the shared path's view of std's
  definitions); `nova-driver` (the HIR dumps, beside `ResolutionDumps`);
  `nova-cli` (the hidden flag, `tests/selfhost.rs`, `tests/selfhost/`
  with `cases/check*` and `fuzz.rs`); `selfhost/` (§3.5's files, their
  tests); `docs/adr/0055-*.md`.
- **5.3b:** `nova-typeck` (the order of bodies, the full dump, std's
  bodies skipped on the shared path); `selfhost/` (`infer.nova`,
  `typeck_expr.nova`, `typeck_call.nova`, `typeck_method.nova`,
  `typeck_place.nova`, `typeck_pattern.nova`, `usefulness.nova`,
  `typeck_view.nova`, the rest of `fixes.nova`, their tests; the plan may
  cut them otherwise); `nova-cli` tests; `docs/adr/0056-*.md`.

## 8. Records

- **ADR 0055** (5.3a): the dump's grammar with an example per form; the
  flag; std shared, its fallback and guard; N's idioms; the cases and the
  item fuzzer; the figures; the rule that a change to item collection
  changes both compilers.
- **ADR 0056** (5.3b): S0's order of bodies and why; the full dump's
  forms; std's bodies shared; inference variables in S0's order; the
  body fuzzer; the figures; the rule extended to the type checker.
- **The phase plan,** dated in 5.3a: 5.3 split by stage, with the
  corrected sizes (§1); 5.4's input is the HIR the dump shows.
- **Dated notes** in the master spec's Phase 5, and in
  `nova-spec/12-TYPESYSTEM.md` where it describes the checker's passes.
- **The CHANGELOG:** 5.3a Added (N's item collection, the hidden dump);
  5.3b Added (N's type checker) and Changed (methods checked in
  declaration order, closure numbers).

## 9. Risks

Ranked.

1. **CI time.** Every projection puts Windows past 70 s, and the last
   local projection undershot CI. Real figures come before each plan,
   and the user decides (§3.7); nothing is cut without asking.
2. **N's stack.** S0 type-checks on 64 MB, N on Windows on 1 MB, with
   larger frames than the parser's. Measured in each prototype; if N
   overflows, the user is asked (a larger thread is a runtime change).
3. **5.3b is large:** about 6,400 lines, twice 5.3a and about twice
   5.1b. The differential shows each divergence by span, and the cases
   are many and small.
4. **Inference-variable order.** A `fresh` call moved or added in N
   renumbers every later `?N` in that function's messages; the cases
   that print `?N` and the fuzzer show it.
5. **std's part's size:** likely larger than its 28,597-line AST dump,
   in three modes; printed and compared once per run.
6. **The order change reaches users** (§4.1): the CHANGELOG says so, and
   the comparison over every tracked file lists each output it changed.
7. **The shared path drifts from the full one.** The equivalence test
   over every input, in both orders, guards it, and the full path takes
   over when std alone gives a diagnostic; §4.5's question is open.
8. **The two compilers drift** if a change to the type checker lands in
   Rust only. The differential in every pull request enforces the rule.

## 10. Not in 5.3

- The language server's probe and index (decision 8), and `analyze`.
- MIR, monomorphization and `async_lower` (5.4), and the driver's work
  after the type checker (`keep_the_entry_main`, `synthesize_test_main`).
- Diagnostics rendered as `codespan` text in N (5.5).
- The package graph's M-codes in N (ADR 0054).
- A runtime change for N's stack or speed, unless the user rules (§9).
