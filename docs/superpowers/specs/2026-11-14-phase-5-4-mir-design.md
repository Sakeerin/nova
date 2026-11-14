# Phase 5.4, "MIR: lowering, monomorphization and `async_lower`": design

- **Date:** 2026-11-14.
- **Status:** approved by the user (2026-11-14).
- **The user's decisions, 2026-11-14:**
  1. **One pull request, split into tasks, test first,** as 5.3b; the
     dump shows MIR after the async transform, with a before-transform
     form to locate a divergence.
  2. **std's bodies shared and spliced in:** both sides keep std's
     checked functions in the shared state and splice them into each
     program with their ids shifted; a test checks that the shared result
     equals the full check's; CI is measured from the prototype before
     the plan.
  3. **The inputs:** the corpus, the HIR cases that reach MIR, about 85
     sources from `lower_tests.rs` and `pattern_lowering.rs` as `.nova`
     cases, and hand cases for every MIR code reachable from source;
     about 38 internal tests stay Rust-only.
  4. **Native only:** WASM and test-mode MIR stay S0's, recorded as
     (differs) in the ADR; the 35 WASM programs are not compared.
  5. **Both forms compared:** the main dump after the async transform
     (what the emitter reads), a second before it, so a difference is
     placed in lowering or in `async_lower`, and the early tasks compare
     before `async_lower` is ported.
  6. **A new type-preserving fuzzer** aimed at MIR (new generic
     instances, awaits, match shapes, closures), with floors so each main
     feature occurs, within CI's budget.
  7. **`RtFunc::symbol` and `signature` come with the emitter in 5.5;**
     5.4 ports only the variant names the dump shows.
  8. **A hidden `nova check --dump-mir`** (ADR 0053's mechanism); N's
     side `selfhost mir`; every `--help` unchanged.
- **Not asked; the recommendation taken, which the user can overturn:**
  the before form is an option, `--dump-mir --before-async`, and the form
  takes no `--std`, `--target` or `--test` (§3.1); the printer lives in
  `nova-mir`, the hook in `nova_mir::Options` (§3.2); a root that does not
  run prints `(Mir runs=false)`, one that stops before MIR `checked=false`
  and no diagnostic (§4.1); one line per temp (§4.4); once N ports
  `async_lower`, the before form is compared only where it can differ
  (§7.2); 5.3b's stack rule holds (§7.5); no frozen list (§9).
- **Branch:** `claude/loving-archimedes-frtf5q-54`, from `main` at
  `9389a84b` (5.3b's merge), for this spec; the plan and the work follow
  on it. Nothing is pushed without the user's word.
- **Inputs:** `docs/phase-5-plan.md` (§4's 5.4 and 5.5 entries, §6 risks
  3 to 5); the research for this spec (scratch only, summarised in §2);
  ADRs 0048, 0053, 0055 and 0056; the 5.3 spec.

## 1. What 5.4 delivers

N's sixth stage is **MIR**, what S0's `nova_mir::lower_module_with`
builds natively from the HIR 5.3 builds: monomorphization from `main`;
every reached function lowered to temps, blocks, statements and
terminators, `match` compiled to switches and chains; the async
transform (each `async fn` split into its `$poll` function and a
wrapper, an async `main`'s shim); every MIR diagnostic, in S0's order. A
canonical **MIR dump** on both sides pins it, after the transform and
before it. From 5.4 MIR exists twice; a change to either changes both
compilers in one pull request (phase plan decision 6).

**The phase plan is amended: 5.4 is native only** (decision 4). WASM MIR
differs (its std, `route_runtime`, roots and exports, 9,049 `Mem`
statements over the 35 programs, `Js` externs, E0091), only `check
--target wasm` shows it, and N's backend is native (ADR 0048). Test-mode
MIR exists only under `nova test` (a synthesized `main`, `std/test`,
kept `@test`s, package-wide), which Phase 5 does not port; the HIR
dump's per-file `--test` has no MIR counterpart. **The amended gate:**
native MIR dumps, after and before `async_lower`, identical to S0's on
the native corpus roots that run, the HIR cases that reach MIR, the MIR
cases, the depth cases and the MIR fuzzer's programs, which all
type-check; MIR's diagnostics included. The size is about 3,400 code
lines with 38 inline and 84 integration tests, not 2,835 (§2 item 1).

## 2. The starting point

Measured on `main` at `9389a84b`, 2026-11-14, with a probe S0 (timing
hooks and a throwaway MIR printer, never committed); line numbers
re-checked for this spec.

1. **Sizes,** in code lines, test modules skipped by brace matching
   (5.3's rule undercounts `lib.rs`, whose test module sits mid-file,
   `lib.rs:917-946`): `nova-mir` 3,411 (`lib.rs` 772, `lower.rs` 1,361,
   `mono.rs` 688, `async_lower.rs` 590), 38 inline tests;
   `tests/lower_tests.rs` 74 tests, `pattern_lowering.rs` 10. Natively,
   without `on_wasm` (117), `WasmRuntime` (8), `RtFunc::symbol` (116)
   and `signature` (127): `lib.rs` about 400, `mono.rs` about 530. Also
   the nova-hir helpers N lacks (about 65: `has_params`, `Module`'s
   lookups, `resolve_method_full`, `Pattern::is_flat`), and
   `keep_the_entry_main` (17, `nova-driver/src/lib.rs:761`) and
   `mir_options` (3, `:753`).
2. **MIR is deterministic; S0 needs no change for order.** `nova-mir`
   iterates no hash map (`done` insert and test only, `mono.rs:113`).
   The worklist is a LIFO stack seeded with `main` (`:114`, popped at
   `:172`); temps and blocks are numbered as created
   (`lower.rs:185-196`); HIR is read by `DefId` and first match only.
   Measured: 210 of 210 `.ll` files identical across two processes, 30
   of 30 from another directory, paths and `-o`.
3. **No MIR printer exists,** only `#[derive(Debug)]` (`lib.rs`: `MirTy`,
   5 classes, `:43`; `Stmt`, 26 variants, `:1093`; `MemOp`, WASM only,
   `:1284`; `Terminator`, 5, `:1317`; `RtFunc`, 111, `:159`). No spans.
4. **No std part of MIR is possible:** MIR is monomorphized per program
   from `main`, every symbol carries an absolute `DefId` (`mangle`,
   `lib.rs:1381`), and std's ids follow the program's, so one std
   instance is `Vec_T.push.147`, `.149`, `.151` or `.152` by program.
5. **The shared path throws std's checked bodies away:**
   `SharedItems::new` checks them for their diagnostics alone
   (`nova-typeck/src/check/items.rs:74-82`), `check_program_shared`
   returns the program's functions alone (`:187-201`). N's `SharedItems`
   keeps its whole checker, std's functions with it
   (`selfhost/src/typeck.nova:125`, `:144-153`).
6. **Lowering is cheap; type checking dominates:** over the 597 inputs
   that reach MIR (220 native roots, 35 WASM, 342 cases), S0 at
   `opt-level = 1` lowers in 0.235 s and type-checks in full in 8.24 s.
7. **MIR lowers only what `main` reaches:** 342 of the 832 HIR cases
   reach MIR natively (341 clean, 1 E0601); of the 341, 216 lower only
   `main`, 155 a trivial one. Corpus and cases hold no MIR code but E0601.
8. **Diagnostics:** 9 sites, 7 codes: E0601 (`mono.rs:96`, no span;
   `:104`), E0075 (`:205`), E0011 (`:248`), E0013 (`:276`,
   `lower.rs:974`), E0078 (`:336`), E0079 (`:352`), E0091 (`:482`, WASM
   only). Both E0601s, E0075, both E0013s and E0079 were reproduced from
   source; a runtime test reaches E0078 (`run_tests.rs:3843`); nothing
   reaches E0011. `check` lowers only a program that runs, a loose file
   or a package's `src/main.nova` (`nova-driver/src/lib.rs:102-116`,
   `program.rs:107`), and only when the type checker gave no error, after
   `keep_the_entry_main` (`lib.rs:1364`).
9. **The transform is a pass of its own** (`mono.rs:395`), after the shim
   is pushed (`:388-391`). It touches only `is_async` functions, each
   rewritten as `<name>$poll` with a wrapper appended after all functions
   (`async_lower.rs:203-215`); with none, it changes nothing. The native
   corpus has 9,353 functions and 569 awaits before it, 9,771 functions
   (418 `$poll`) after; 76 of its 210 programs and 2 cases hold an async
   function.
10. **N:** 44 files, 25,771 lines; its type checker, HIR and dump are
    0.94 of S0's size; no language blocker, idioms only (§6.2). S0's
    pipeline runs on 64 MB (`PIPELINE_STACK`, `lib.rs:549`), N on the
    main thread, 1 MB on Windows; N's stack in MIR is not measured.

## 3. S0's changes

Each the smallest that works; none visible to a user.

### 3.1 The hidden flag (decision 8)

`nova check --dump-mir [--before-async] FILE…` prints, for each file,
`== FILE` (the path as given) and its part, or `IOERR` and the error's
kind for a root it cannot read, after which the rest are dumped and the
exit code is 2; with no file, one line on stderr and exit 2.

- **ADR 0053's mechanism:** taken only when `--dump-mir` is the first
  argument after `check`. `DumpCmd::from_args`
  (`nova-cli/src/cmd/run.rs:556`) gains a third arm with a parser of its
  own, `DumpMirCmd`: `--before-async` and files, each option once, `--`
  before a file that starts with `-`. Anything else (`--std`,
  `--target`, `--test`, `--help`, `-h`, `--dump-mir` again) is one clap
  error under the usage `nova check --dump-mir …`, exit 2;
  `--dump-mir=…` is not the form, so the base's refusal stands.
  `CheckCmd`, `--help`, clap's tips and every other command line stay the
  base's, byte for byte.
- **No `--std`, `--target` or `--test`:** MIR has no std part (§2 item
  4); WASM and test-mode MIR are not compared (decision 4).
- **`--before-async` an option** (not asked): the mechanism finds one
  flag per dump; `--dump-mir=before-async` is the base's refusal by ADR
  0053's rule; a second leading flag, like 5.3b's transitional
  `--dump-hir-items`, would give one permanent dump two hidden entries.

### 3.2 The printer and the hook

- **`nova_mir::dump`** (new), as each earlier dump lives in its stage's
  crate: a root's part from a `Module` or MIR's diagnostics, and `KINDS`,
  every form it prints. Diagnostics go through
  `nova_resolver::dump::write_diagnostics`, 5.2's form, places under
  their files' labels (`nova-mir` depends on `nova-resolver` already).
- **The hook:** `nova_mir::Options` (`mono.rs:24`) gains
  `stop_before_async: bool`, false by default. Set, `lower_module_with`
  returns the module before `async_lower::transform` (`mono.rs:395`),
  the shim pushed, and skips the two `debug_assert!`s on the transform's
  output (`:407-421`). Natively nothing else follows the transform (the
  WASM routing, `:396-406`, is the target's). No user path sets it.

### 3.3 `MirDumps`, in the driver

`nova_driver::MirDumps`, beside `HirDumps`, sharing its front end and
`SharedItems`. For each root: (1) one it cannot read is `IOERR`, as in
the HIR dump; one that does not run prints `(Mir runs=false)`, decided
as `check` decides it (`Program::for_file_with`), before the front end;
(2) `check`'s pipeline to the resolver, stopping on an error; (3) the
shared check with std's functions spliced in (§5), or
the full check for a program that implements anything on a type it does
not own or when std is not shareable; (4) `keep_the_entry_main`, then
`lower_module_with` natively, with `stop_before_async` for the before
form; (5) the dump. **A user sees nothing:** `nova check`, `build`,
`run`, `test` and the language server keep `check_with` and their paths.

**Checked by:** the hidden-flag test, extended to `--dump-mir`; the
dump's unit test, one example of every form; `nova check` over every
tracked `.nova` file printing what the base's prints, byte for byte (as
5.3b's `checkcmp.sh`); the golden runtime tests unchanged.

## 4. The dump

### 4.1 One root's part

```
== tests/runtime/closures.nova
(Mir runs=true checked=true lowered=true async_lowered=true functions=12 externs=0
  (Function "main" params=0 takes_env=false captures=0 ret=Unit async=false
    (Temp t0 I64)
    …
      (Call t39 "apply.0" (t4 t38))
      (CallRuntime t40 IntToStr (t39))
      …
      (CallRuntime _ Println (t41))
  …)
(Diagnostics)
```

The probe's sample in the proposed form; ADR 0057 writes the grammar, one
example per form. The first line says how far the root got: `(Mir
runs=false)`, a package module `check` does not lower (a line, not
nothing, so every root shows and N's `runs` is checked, at no cost);
`(Mir runs=true checked=false)`, an error before MIR, with no diagnostic,
since the HIR dump compares the earlier stages' on the same inputs and
each grammar keeps one owner ADR; `(Mir runs=true checked=true
lowered=false)`, MIR's diagnostics and no module; `lowered=true` with
`async_lowered=true` (the main form) or `false` (`--before-async`), the
counts, the externs and every function. `(Diagnostics)` holds MIR's
alone, in 5.2's form; the first E0601 has no label, so prints no place.

### 4.2 Order and contents

- **Order:** MIR's own, nothing sorted: `Module::functions` as the
  worklist made them, the shim after the loop, the wrappers after all;
  externs as collected; temps and blocks by index.
- **An extern:** symbol, ABI, parameter and return classes. Natively
  `runtime_alloc` and `runtime_exports` are empty and not printed.
- **A function:** name, `params`, `takes_env`, `captures`
  (`capture_count`), `ret`, `is_async`; each temp `t<n>` with its class;
  each block `bb<n>`, its statements, then its terminator (`Goto`,
  `Branch`, `Switch` with each arm's value and block then the default,
  `Return` with its temp or none, `Trap`).
- **A statement:** one line, its `Stmt` variant's name, then every field
  in declaration order: `_` for an absent destination; temps `t<n>`;
  integers in decimal; floats as the 16 hex digits of `to_bits` (5.1's);
  strings and symbols quoted as 5.1 quotes them; `BinOp`,
  `OperandClass`, `MirTy`, `RtFunc` and `MemOp` by variant name; lists in
  parentheses, a capture's, field's or element's class beside its temp
  (`t3:Ptr`). `Mem` never occurs natively (§6.1).
- **Symbols** print raw, with absolute `DefId`s (`Vec_T.push.147`,
  `closure$724.724`). No std part is possible (§2 item 4), and none is
  needed: both sides number with S0's absolute ids (N maps them only in
  its HIR dump, `selfhost/src/hir_dump.nova:182-192`), and the splice
  gives std's functions the full check's ids (§5).

### 4.3 Locating a divergence

MIR has no spans, so for a MIR dump `compare.rs`'s report names the
root, the function and block above the first line that differs, and the
statement's index there, where other dumps give a source line. **The
before form adds** each `async fn` as lowering left it, `async=true`
with its `Await` statements, before the split into `$poll` and wrapper:
a difference there is in lowering or monomorphization; one in the after
form alone is in `async_lower`.

### 4.4 Size

Measured with the probe's printer, one line per temp: the native corpus's
210 programs (9,771 functions, 246,741 temps, 70,146 blocks, 268,237
statements) about 675k lines and 14.9 MB, half the HIR program parts';
N's root 197k lines, 4.4 MB; the cases 18.6k. With each function's temps
on one line, about 428k lines. **One line per temp** (not asked): the
report then shows the one temp that differs, not a line of thousands of
classes; S0 prints 15 MB in about 0.1 s. **Checked by:** the dump's unit
test, the header test and the form check (§7.3).

## 5. std's bodies, shared (decision 2)

- **Kept:** S0's `SharedItems::new` keeps the functions of std's check,
  where it drops them today: std's ids from 0, its closures from `S`,
  the number of std's definitions. N's holds them already (§2 item 5).
- **Spliced, on both sides** (new: `nova_typeck::shared_module`, beside
  `Rebase` in `check/items.rs`). With `D` the program's own definitions
  (`StdStart`'s `def`) and `P` its closures and wrappers (`Checked`'s
  `std_closures - closures`), std's functions are copied with:
  - every id below `S` moved up by `D`: the function's `def_id` and
    `bounds`; `Callee::Def`, `MakeClosure`'s function, `MakeVariant`'s
    sum, `MakeRecord`'s record, `TraitCall`'s trait, `PatVariant`'s sum;
    every type (`Ty::Sum`, `Ty::Record`, `Ty::Assoc`) on nodes, locals,
    `ret_ty`, `TypeArgs` and `SelfType`;
  - every closure or wrapper id, `S` and up, moved up by `D + P`, past
    the program's closures, wherever it appears;
  - the names `closure$N` and `fnval$N` (`check.rs:5720`, `:5796`)
    rewritten with the new id, since they reach MIR's symbols;
  - nothing else: locals, variant, field and method indices, and spans,
    whose files the dump registers std-first, the same in every program.
- **Where they go:** after the program's functions, methods and
  constants, std's closures after the program's, as the full check orders
  `hir::Module::functions` (`check.rs:3245-3281`). MIR does not depend
  on that order (§2 item 2), but it makes the module the full check's, so
  the two compare whole. The tables are the shared path's, std's parts
  rebased already (ADR 0055 §4). Wrappers are made afresh, with no cache
  (`make_fn_wrapper`), so std's segment does not depend on the program.
- **Equal to the full check, byte for byte:** a unit test beside
  `shared_bodies_check_what_the_full_check_does` (`check/items.rs:890`)
  compares `shared_module`'s `hir::Module` with `check_with`'s by their
  `Debug` text, over programs with no closure, one and several, against
  a std holding a closure (that test's `$std.zz`), so `P` varies.
- **The guard, both orders:** the driver's
  `std_checked_once_dumps_what_std_checked_for_each_program_dumps`
  (`nova-driver/src/lib.rs:1808`) compares, in its native set and its
  existing passes, so no second full check runs, the shared MIR dumps
  (both forms) with the unshared ones over every native root and case,
  in order and reversed.
- **5.3b's fallback holds:** a program that implements anything on a type
  it does not own (ADR 0056 §4; 78 roots natively) takes the full check
  and is not spliced; so does every program when std alone gives a
  diagnostic or holds a test. The guard lists them.

**Checked by:** the unit test and the guard; N's splice by the
differential, N's MIR dump being built on it.

## 6. What N ports

### 6.1 The code

`lower_module_with`'s native path, function by function, with S0's
names: MIR's types, `mangle` and `mangle_ty`; `RT_FUNC_VARIANTS`, the 111
names in `RtFunc::ALL`'s order, as `BUILTIN_VARIANTS` holds `Builtin`'s
(no `symbol` or `signature`, decision 7); `lower.rs` whole
(`lower_function`, `lower_expr`, `lower_call` with its 118-arm builtin
table, `lower_trait_call`, operators, `if`, array repeat, `match`);
`mono.rs` natively (the worklist, externs and E0075, names, bounds and
E0013, `Specializer` with E0078 and E0079, E0601, E0011 ported as
written though nothing reaches it, `type_name`, the shim);
`async_lower.rs`'s `transform`; the nova-hir helpers,
`keep_the_entry_main` and `mir_options`. About 2,970 Rust lines, half of
5.3b's 6,400. Not ported: the WASM code (about 290 lines) and the `Mem`
builtins' lowering, which no native program reaches: `mem_load8` in a
native program is E0001, before MIR (measured for this spec).

### 6.2 Idioms

- **`Int` codes or helpers where Rust uses `==`:** `MirTy`,
  `OperandClass`, and `RtFunc` as an index into `RT_FUNC_VARIANTS`, at
  the 14 sites (as `async_lower.rs:225`, or `mono.rs:203`'s
  `prev.params != params` on two `Vec<MirTy>`).
- **New nodes, never rewritten HIR:** records alias, so the specializer
  builds each instance's nodes afresh and never writes into the generic
  function's HIR, which every instance reads; the splice copies std's
  shared functions the same way.
- **The worklist** a `Vec` used as a stack; `done` a `Map<String, Bool>`
  read by lookup only (phase plan decision 9); `Module::function` an
  index map from `DefId` to position, where S0 scans
  (`nova-hir/src/lib.rs:434-436`), the same output since `DefId`s are
  unique in `functions`.
- **Asserts:** `async_lower`'s four `expect`s (`:517`, `:690`, `:782`,
  `:807`) and S0's `debug_assert!`s become `panic`s with S0's words,
  never reached on a valid input; S0's debug build keeps its asserts, at
  `opt-level = 1` too (ADR 0056 §14). Floats go by `Float.to_bits`.

### 6.3 Files and command

- **Where:** `selfhost/src/mir.nova` (types, names, `mangle`),
  `lower.nova` (through `lower_expr`), `lower_call.nova` (calls to array
  repeat), `lower_match.nova`, `mono.nova` (with `Specializer` and the
  driver's pieces), `async_lower.nova`, `mir_dump.nova`, `mir_tests.nova`;
  the helpers in `hir.nova`. The plan may cut them otherwise, none past
  about 1,500 lines, as the type checker's (largest 1,418).
- **`selfhost mir --std DIR [--before-async] FILE…`:** std read from DIR
  and checked once, every root splicing its functions as S0's does. It
  refuses S0's form's shapes (`--std-part`, `--target`, `--test`, an
  option twice, no file) with exit 2 and nothing on stdout; an
  unreadable root is `IOERR`, exit 2 after the rest; a manifest N's
  reader refuses stops it with exit 1, as `selfhost hir`.

**Checked by:** the differential (§7); N's `@test`s: `mangle_ty`'s codes,
a generic instantiated twice and a splice done twice with their sources
unchanged, `async_lower`'s state slots.

## 7. The harness

### 7.1 Inputs

`selfhost.rs` gains the MIR dump (S0's `check --dump-mir`, N's `mir
--std DIR`), compared byte for byte with exit codes through
`compare_batch`:

1. **The native corpus roots that run:** the 251 loose roots and the
   packages' `src/main.nova` files; 220 reach MIR (210 clean, 10 E0601).
   N's `selfhost/src/main.nova`, the heaviest, is the only one of the
   HIR test's 44 package roots of N that runs.
2. **The HIR cases that reach MIR:** those of `cases/check/` whose part
   in the HIR test's run holds no error (342 of 825 today), so no case
   runs its front end for one line.
3. **`cases/mir/`, extracted** by a temporary instrumentation of
   `mir_for`, `diagnostics_for` and `program()` (a script, not
   committed): every source a native test lowers, as
   `<testfile>__<test>[_n].nova`, each `format!` expanded (15 use
   `deep_wrap_program`); about 85. The 7 WASM sources are left out, and
   so is a `diagnostics_for` source whose ignored type error stops it
   before MIR through the driver, each listed in the ADR.
4. **`cases/mir/`, by hand:** a case per MIR site reachable natively:
   E0601 with no `main` and with a generic one, E0075, E0013 at both
   sites, E0078, E0079. **E0011** is judged unreachable and **E0091** is
   WASM only; the ADR says both.
5. **The MIR fuzzer's** 120 inputs (§7.4) and **the depth cases** (§7.5).

### 7.2 Both forms, line endings, Windows

- **Both forms (decision 5):** the after form on every input; the before
  form on every input until N ports `async_lower`, then on the inputs
  whose after form holds a `$poll` function (76 corpus programs, 2
  cases, the fuzzer's async additions). Elsewhere the forms are equal by
  construction (§2 item 9), and N's after form pins its lowering there.
- **`\r\n`:** `cases/mir/` and the HIR cases that reach MIR again with
  `\r\n` (`copy_crlf`): E0601's and E0013's places are byte offsets.
- **Windows:** root lists past `HIR_ARGS_LIMIT` (16,000 bytes) go in
  parts side by side (`agree_in_parts`, ADR 0056 §14). `run`'s
  `settled_stdout` settles the MIR dump as the HIR dump; no MIR message
  names a path (checked for this spec), so nothing changes today, but one
  rule covers every dump that runs the loader.

### 7.3 Refusals, headers, forms, codes

Refusals on both sides, §3.1's and §6.3's shapes. One header per root:
an unreadable root, a package module, a program that stops in the type
checker, an E0601, a clean program. Every form of `KINDS` but `Mem`
among the native dumps, nothing unlisted, each `key=bool` field with both
values; `Await` and `async=true` in the before form only. Every code:
E0601 and E0013 at both sites, E0075, E0078, E0079, among the cases.

### 7.4 The MIR fuzzer (decision 6)

`fuzz::mir_mutate`, seeds of its own (601 to 603), 40 inputs each.

- **Bases:** loose corpus programs that import no module and lower
  clean, read from the corpus's MIR dump as the type fuzzer reads its
  from the HIR dump; `\r\n` read as `\n`, so a seed gives the same inputs
  on every system.
- **Closed additions:** new items under fresh names (`__m<n>…`) and a
  statement at the start of `main`'s body that reaches them:
  - **generic instances:** a generic function, record or sum with a
    method, at types the seed draws (primitives, tuples, arrays,
    `Option`, `Result` and `Vec` of them, its own types), some through a
    bound and an impl on its own type;
  - **awaits:** an `async fn` awaiting another, values live across each
    await, driven from a plain `main` by std's `block_on` (which checks
    and lowers so, measured for this spec);
  - **match shapes** over its own sum: nested variant and tuple patterns,
    guards, or-patterns, ranges, string and char literals, a wildcard;
  - **closures:** captures of each class, a closure passed to a generic
    function and called through a function-typed parameter, a bare
    function used as a value.
- **Every input type-checks and lowers clean.** An addition names only
  its own items, std's and fresh names, and its call is a statement of
  type `()`, so it type-checks by construction; the test fails on any
  input but `lowered=true` with no diagnostic, never skipping it.
- **Floors:** each kind in at least 10 of the inputs, by its mark in
  S0's dump (an instance's symbol with type arguments, a `$poll`, a
  `Switch`, a `MakeClosure` and a `CallIndirect`, in an addition's
  function). Its time is measured in the prototype (§8).

### 7.5 The depth cases

5.3b's 19 body kinds at their depths (128, or 64 and 127 where the parser
stops sooner, ADR 0056 §12), made to lower: each nested body in a
function `main` calls, once plain and once in a generic function `main`
instantiates, so `lower_expr` and `Specializer` both walk it; and types
nested 128 levels in an instance's type argument (`mangle_ty`,
substitution). N lowers them on a 1 MB main thread, on CI's Windows leg
and locally under a 1 MB limit. If N overflows, 5.3b's answer 2 holds
(not asked): the depth it survives is recorded as (differs), 5.5's
driver runs N on a large-stack thread, and the runtime does not change.

**Rust-only:** the 38 inline tests (`async_lower`'s 30 on hand-built
MIR, `mono`'s 7 on hand-built HIR and `mangle_ty`, `lib`'s 1). The 84
integration tests stay too, their sources cases.

## 8. Measured in the prototype, before the plan

- **CI's time (decision 2):** with the user's word before the push, the
  prototype goes to `claude/loving-archimedes-frtf5q-54-ci` with a draft
  pull request, closed unmerged, as 5.3a's and 5.3b's did. Its
  `selfhost.rs` figures on all three systems come before the plan; past
  the lines (60 s Linux and macOS, 70 s Windows, for the Selfhost job:
  N on the release runtime, S0 at `opt-level = 1`, ADRs 0055 §15 and
  0056 §14) the user is asked again with them. Coverage is never cut
  without asking.
- **The projection,** std spliced: +3 to 12 s of CPU, +1 to 4 s of local
  wall, about +2 to 11 s on CI at the research's CI-to-local factors (1.4
  to 2.9), for one form; the before form on the async inputs adds a
  share, not projected. A full check per program, which decision 2 set
  aside, projects 54 to 68 s on Linux and macOS, 63 to 126 s on Windows.
- **Past projections missed CI both ways:** 5.2b's undershot by about
  half; 5.3b's macOS was projected at 60 s and took 86.45 s in its first
  run; 5.3a's overshot. **Today's CI spreads wider than the increment:**
  at 5.3b's head (run 43559046222, two attempts, no change between them)
  and its merge (run 43570327222), `selfhost.rs` took 39.22 to 42.28 s
  on Linux, 21.98 to 49.25 s on macOS and 41.62 to 90.74 s on Windows.
- **Also:** the Test job (S0 lowers the 597 inputs in 0.52 s at
  `opt-level = 0`, but `nova-driver`'s unit tests, the guard most of
  them, took 93.88 s on Linux in 5.3b's first CI run); N's stack
  (§7.5); N's peak RSS and GC pauses on its root's MIR (about 72k temps,
  77k statements, over 46.9 MB of live HIR), no threshold; the canonical
  dumps' sizes; the fuzzer's time.

## 9. The cut, for the plan (decision 1)

Suggested, each task test first: (1) **S0's dump:** `nova_mir::dump`,
`stop_before_async`, the flag and its parser, `MirDumps` with the full
check, S0's own tests; (2) **std spliced:** `SharedItems` keeps std's
functions, `shared_module`, its unit test, the guard in both orders, the
fallback; (3) **the cases,** extracted and by hand, checked on S0's side;
(4) **N's lowering and monomorphization** (§6.1 but `async_lower`) and
`selfhost mir --before-async`, the before form compared on every input,
N's plain `selfhost mir` refusing with exit 3 until (5); (5) **N's
`async_lower`,** the after form on every input, the before form as §7.2,
the refusal removed; (6) **the fuzzer, the depth cases and `\r\n`,** N's
stack measured; (7) **the records** (§14).

**No frozen list** (not asked): the before form is the transitional
mechanism, and each of N's tasks ends with every input compared in the
form it reaches. Task 4 is about 2,400 of N's lines, a little above each
of 5.3b's three N tasks; if the plan cuts it at `match` (340 lines), its
first half uses 5.3b's frozen list.

## 10. Gate

- The native MIR dumps of the corpus roots that run, the HIR cases that
  reach MIR, `cases/mir/` (both line endings), the fixed seeds' fuzzer
  inputs and the depth cases are identical on all three systems: after
  the transform on every input, before it where §7.2 compares it.
- Every fuzzer input lowers clean and the floors hold; E0601 and E0013
  occur at both sites, E0075, E0078 and E0079 at theirs; every form of
  `KINDS` but `Mem` occurs.
- The spliced module equals the full check's; the shared MIR dumps equal
  the unshared ones over every native root and case in both orders; the
  fallback's roots are listed.
- `nova check` over every tracked file prints what the base's prints,
  byte for byte; `--dump-mir` is in no help or usage line.
- N lowers the depth cases on a 1 MB stack, or the depth it survives is
  recorded; CI is measured and the user has answered; N's memory is
  recorded.
- The full suite, fmt, clippy, the MSRV job, `nova fmt --check` over
  `selfhost/`, and `git diff` on `Cargo.lock` are clean.

## 11. New diagnostic codes

None. S0's MIR and its diagnostics are unchanged.

## 12. Testing

Unchanged rules: tests never touch the internet; fixed temporary names;
`$NOVA_HOME` fresh where needed; MSRV 1.78, no new crate, `Cargo.lock`
unchanged; no `clang`. Then §3's and §5's S0 tests, §7's differential
beside the 5.1 to 5.3 dumps, and N's `@test`s (§6.3).

## 13. Crates and files

`nova-mir` (`src/dump.rs`, `Options`); `nova-typeck` (`SharedItems`,
`shared_module`); `nova-driver` (`MirDumps`, the guard); `nova-cli`
(`DumpMirCmd`; `tests/selfhost.rs`, `fuzz.rs`, `compare.rs`,
`cases/mir/`); `selfhost/` (§6.3); `docs/adr/0057-*.md`; the records'
files (§14). No new crate.

## 14. Records

- **ADR 0057:** the dump's grammar with an example of every form; the
  flag and its before form; the splice, what shifts, the fallback and the
  guard; native only, (differs), and why; N's idioms; the cases, those
  left out, E0011 and E0091; the fuzzer; the depth cases and N's stack;
  the figures; the rule that a change to MIR changes both compilers.
- **`docs/phase-5-plan.md`,** dated: 5.4 native only, the amended gate,
  the corrected size (§1). **Dated notes** in
  `nova-spec/00-MASTER-SPEC.md`'s Phase 5 and `nova-spec/14-CODEGEN.md`
  (MIR, §6.3 generics). **`CHANGELOG.md`:** Added (N's MIR, the hidden
  dump); nothing Changed.

## 15. Risks

Ranked.

1. **The splice's id shifting.** A missed id (a local's type, a
   pattern's sum, a closure's name) changes symbols only where a program
   reaches that std function, perhaps in none of today's. The unit test
   compares whole modules, the guard every input both ways. If the splice
   cannot be made equal, the full check per program is the way back, past
   the lines (§8), and the user is asked.
2. **CI time on Windows:** a few seconds projected, but 41.62 to 90.74 s
   on one commit, and projections have missed both ways. Real figures
   come first; the user decides.
3. **N's stack** in `lower_expr` and `Specializer`, larger frames on 1 MB
   against S0's 64 MB: measured; 5.3b's rule applies.
4. **Aliasing:** an instance writing into the generic function's nodes
   changes every later instance, only where there are two; the splice
   carries the same risk across programs. N's `@test`s, the fuzzer and
   the guard's reversed pass show it.
5. **The fuzzer's programs fail to type-check:** closed additions hold by
   construction, and the test fails rather than skips.
6. **The dump's size and time:** about 675k lines natively per form;
   N's printing speed is not measured.
7. **The two compilers drift** if a change to MIR lands in Rust only; the
   differential in every pull request enforces the rule.

## 16. Not in 5.4

- WASM MIR: `on_wasm`, `WasmRuntime`, `route_runtime`, roots and
  exports, `runtime_impls`, the `Mem` builtins, E0091 (decision 4).
- Test-mode MIR: `synthesize_test_main` and its helpers, the
  rename-and-push, `name_tests`, `fresh_def_id` (about 165 driver lines).
- `RtFunc::symbol` and `signature` (5.5, decision 7).
- The `.ll` emitter, `--emit-ll`, the driver's command line, `build`,
  `run`, and diagnostics rendered as text in N (5.5).
- The bug by which std's bodies see a program's impls (ADR 0056 §4),
  worked around by the fallback, not fixed.
- A runtime change for N's stack or speed, unless the user rules.
