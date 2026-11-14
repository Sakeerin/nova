# ADR 0057 — The self-hosted MIR, and the MIR dump

## Status

Draft (2026-11-14). Written as 5.4's prototype is built, on the local
branch `proto/5-4`
(`docs/superpowers/specs/2026-11-14-phase-5-4-mir-design.md`). Part 1
(S0's side) wrote §1 to §9:
- §1, the hidden flag and its before form;
- §2, the dump's grammar, with one example of every form, and the
  decisions it took;
- §3, the hook before the async transform;
- §4, `MirDumps`;
- §5, std's checked bodies kept and spliced into each program, the
  fallback and the guard;
- §6, why 5.4 is native only;
- §7, the cases, those left out, and the codes with no case;
- §8, the harness's switch;
- §9, part 1's figures.

Later parts write N's port, its idioms, the differential in its final
form, the MIR fuzzer, the depth cases and N's stack, the time and CI's
figures, the rule that a change to MIR changes both compilers, and the
Consequences.

## Decision

### 1. The hidden flag, `nova check --dump-mir [--before-async] FILE…`

ADR 0053's mechanism (spec §3.1, the user's decision 8): the form is
taken only when `--dump-mir` is the first argument after `check`.
`DumpCmd::from_args` gains a third arm, and `DumpCmd` becomes an enum
of the two kinds it parses: `Front`, the resolution and HIR dumps with
`DumpResolutionCmd`'s arguments, and `Mir(DumpMirCmd)`.

`DumpMirCmd` is a parser of its own. It takes `--before-async`, once,
and files, with `--` before a file that starts with `-`. Anything else
is one clap error under the usage `nova check --dump-mir [OPTIONS]
[FILES]...`, exit 2. That includes `--std`, `--target`, `--test`,
`--help`, `-h`, the flag again and `--before-async` twice.

`--dump-mir=…` is not the form, so the base's refusal stands, and
`--dump-mir=before-async` with it. With no file it prints `error:
--dump-mir needs a FILE` on stderr, nothing on stdout, and exits 2.

Each file prints `== FILE` (the path as given) and its part. A root it
cannot read prints `IOERR` and the error's kind, the rest are dumped,
and the exit code is 2.

- **No `--std`, `--target` or `--test`.** MIR is monomorphized per
  program from `main` and every symbol carries an absolute `DefId`, so it
  has no std part. WASM's and test mode's MIR are not compared (§6).
- **`--before-async` an option, not a second leading flag.** The
  mechanism finds one flag per dump. A second entry point, as 5.3b's
  transitional `--dump-hir-items` was, would give one permanent dump two
  hidden entries.
- **Checked by:**
  - `the_mir_dump_flag_is_hidden_and_refused_as_the_spec_says`
    (`selfhost.rs`): `check --help` names neither `--dump-mir` nor
    `--before-async`; every base shape around the flag gives the base's
    error, byte for byte; each refusal of the form's own parser is one
    error under its usage line; `--` takes a root that starts with `-`.
  - `usagecmp.sh` (§9): 474 base shapes under six `argv[0]`s, and the
    form's refusals.

### 2. The dump

`nova_mir::dump` (new; the printer lives in its stage's crate, as each
earlier dump does). `dump_program(labels, reached)` gives a root's part.
`Reached` says how far the root got: `NotRun`, `NotChecked`, or
`Lowered { result, async_lowered }`.

**The grammar.** `I(d)` is `d` levels of two-space indentation. A list
is in parentheses, its items one space apart, `()` when empty.

```
part      = mir NL diagnostics
mir       = "(Mir runs=false)"
          | "(Mir runs=true checked=false)"
          | "(Mir runs=true checked=true lowered=false)"
          | "(Mir runs=true checked=true lowered=true async_lowered=" bool
            " functions=" uint " externs=" uint { extern } { function } ")"
extern    = I(1) "(Extern " string " abi=" ("C" | "Js") " (" { class } ") ret=" class ")"
function  = I(1) "(Function " string " params=" uint " takes_env=" bool
            " captures=" uint " ret=" class " async=" bool { temp } { block } ")"
temp      = I(2) "(Temp t" uint " " class ")"
block     = I(2) "(Block bb" uint { stmt } term ")"
stmt      = I(3) "(" Kind { " " value } ")"
term      = I(3) ( "(Goto " bb ")" | "(Branch " t " " bb " " bb ")"
                 | "(Switch " t " (" { int ":" bb } ") " bb ")"
                 | "(Return " (t | "_") ")" | "(Trap)" )
class     = "I64" | "F64" | "I8" | "Ptr" | "Unit"
diagnostics = 5.2's "(Diagnostics" … ")", MIR's alone
```

**A statement** prints its `Stmt` variant's name, then every field in
declaration order:
- a destination as `t<n>`, or `_` when it is absent;
- a temp as `t<n>`, a block as `bb<n>`;
- an integer in decimal;
- a float as the 16 lowercase hex digits of `to_bits` (5.1's form);
- a string or a symbol quoted as 5.1 quotes it;
- a `BinOp`, an `OperandClass`, a `MirTy`, an `RtFunc` or a `MemOp` by
  its variant's name;
- a list in parentheses. A capture's, field's or element's class sits
  beside its temp (`t3:Ptr`); a switch arm's value sits beside its
  block (`0:bb2`).

| Kind | Values |
|---|---|
| `ConstInt`, `ConstFloat`, `ConstBool`, `ConstStr` | dst, the value |
| `ConstUnit` | dst |
| `Copy`, `Not`, `BitNot` | dst, src |
| `Bin` | dst, op, operand class, lhs, rhs |
| `Neg` | dst, operand class, src |
| `Call` | dst or `_`, the callee's symbol, the arguments |
| `CallIndirect` | dst or `_`, the callee's temp, the parameters' classes, the return's class, the arguments |
| `Await` | dst or `_`, the future |
| `CallRuntime` | dst or `_`, the runtime function, the arguments |
| `MakeClosure` | dst, the code's symbol, the captures with their classes |
| `MakeSum` | dst, the tag, the fields with their classes |
| `SumTag` | dst, the sum |
| `SumField` | dst, the sum, the index, the class |
| `MakeRecord`, `MakeArray` | dst, the fields or elements with their classes |
| `RecordField` | dst, the record, the index, the class |
| `SetField` | the record, the index, the value, the class |
| `ArrayAlloc` | dst, the length |
| `ArrayLen` | dst, the array |
| `ArrayGet` | dst, the array, the index, the class |
| `ArraySet` | the array, the index, the value, the class |
| `Mem` | dst or `_`, the operation, the arguments |

**Order** is MIR's own, nothing sorted:
- `Module::functions` as the worklist made them (a LIFO stack seeded with
  `main`), the async `main`'s shim after them, the transform's wrappers
  after all;
- the externs as collected;
- temps and blocks by index.

`runtime_alloc` and `runtime_exports` are WASM's, empty natively, and
not printed.

**Symbols print raw,** with absolute `DefId`s (`block_on.255$i`,
`closure$722.722`). No std part is possible, and none is needed: the
splice (§5) gives std's functions the full check's ids.

**`KINDS`** lists every form the dump prints: each line's kind, and each
`key=value` field whose value is a word. Every `key=bool` field is
listed with both values. **`NOT_NATIVE`** lists the two forms only a
WASM build's MIR holds, `(Mem` and `abi=Js`. The printer takes any
module, so it prints them.

**One example of every form.** S0's `nova check --dump-mir` of
`cases/mir/mir_every_native_form.nova`, shown in part: every line below
is S0's, byte for byte, and `…` stands for lines left out.

```
;; extern "C" { fn labs(x: Int) -> Int }
;; record P { x: Int  y: Float }
;; type S = | A(Int) | B
;; fn pick(s: S) -> Int { match s { A(n) => n  B => 0 } }
;; fn flip(b: Bool) -> Bool { !b }
;; fn half(f: Float) -> Float { -f / 2.0 }
;; async fn one() -> Int { 1 }
;; async fn two() -> Int { one().await + 1 }
;; fn main() {
;;     let mut p = P { x: ~1, y: half(3.0) }
;;     p.x = -p.x
;;     let mut a = [0; 3]
;;     a[1] = a.len() + [5, 6][1]
;;     let k = 2
;;     let f = |n: Int| n * k
;;     let g = pick
;;     println("${f(a[1])} ${g(A(4))} ${flip(true)} ${p.y} ${labs(-3)} ${block_on(two())}")
;; }
== mir_every_native_form.nova
(Mir runs=true checked=true lowered=true async_lowered=true functions=11 externs=1
  (Extern "labs" abi=C (I64) ret=I64)
  (Function "main" params=0 takes_env=false captures=0 ret=Unit async=false
    (Temp t0 I64)
    …
    (Block bb0
      (ConstInt t8 1)
      (BitNot t9 t8)
      (Copy t0 t9)
      (ConstUnit t10)
      (ConstFloat t11 4008000000000000)
      (Call t12 "half.5" (t11))
      …
      (MakeRecord t14 (t0:I64 t1:F64))
      …
      (RecordField t16 t2 0 I64)
      (Neg t17 Int t16)
      (SetField t2 0 t17 I64)
      …
      (Bin t23 Lt Int t20 t22)
      (Branch t23 bb1 bb2))
    (Block bb1
      (ConstStr t24 "array length must not be negative")
      (CallRuntime _ Panic (t24))
      (Trap))
    …
    (Block bb4
      (ArrayAlloc t21 t20)
      (ConstInt t28 0)
      (Goto bb5))
    …
    (Block bb6
      (ArraySet t21 t28 t19 I64)
      …
    (Block bb7
      …
      (ArrayLen t34 t3)
      (ConstInt t35 5)
      (ConstInt t36 6)
      (MakeArray t37 (t35:I64 t36:I64))
      …
      (ArrayGet t40 t37 t38 I64)
      …
      (MakeClosure t46 "closure$722.722" (t4:I64))
      …
      (CallIndirect t53 t6 (I64) I64 (t52))
      …
      (MakeSum t57 0 (t56:I64))
      …
      (ConstBool t61 true)
      (Call t62 "flip.4" (t61))
      …
      (Return _)))
  …
  (Function "flip.4" params=1 takes_env=false captures=0 ret=I8 async=false
    (Temp t0 I8)
    (Temp t1 I8)
    (Block bb0
      (Not t1 t0)
      (Return t1)))
  (Function "fnval$723.723" params=1 takes_env=true captures=0 ret=I64 async=false
    (Temp t0 Ptr)
    (Temp t1 Ptr)
    (Temp t2 I64)
    (Block bb0
      (Call t2 "pick.3" (t1))
      (Return t2)))
  (Function "pick.3" params=1 takes_env=false captures=0 ret=I64 async=false
    (Temp t0 Ptr)
    (Temp t1 I64)
    (Temp t2 I64)
    (Temp t3 I64)
    (Temp t4 I64)
    (Temp t5 I64)
    (Block bb0
      (SumTag t3 t0)
      (Switch t3 (0:bb2 1:bb3) bb4))
    (Block bb1
      (Return t2))
    (Block bb2
      (SumField t4 t0 0 I64)
      (Copy t1 t4)
      (Copy t2 t1)
      (Goto bb1))
    (Block bb3
      (ConstInt t5 0)
      (Copy t2 t5)
      (Goto bb1))
    (Block bb4
      (Trap)))
  (Function "closure$722.722" params=1 takes_env=true captures=1 ret=I64 async=false
    (Temp t0 Ptr)
    (Temp t1 I64)
    (Temp t2 I64)
    (Temp t3 I64)
    (Block bb0
      (RecordField t2 t0 0 I64)
      (Bin t3 Mul Int t1 t2)
      (Return t3)))
  (Function "half.5" params=1 takes_env=false captures=0 ret=F64 async=false
    (Temp t0 F64)
    (Temp t1 F64)
    (Temp t2 F64)
    (Temp t3 F64)
    (Block bb0
      (Neg t1 Float t0)
      (ConstFloat t2 4000000000000000)
      (Bin t3 Div Float t1 t2)
      (Return t3)))
  (Function "two.7" params=0 takes_env=false captures=0 ret=Ptr async=false
    (Temp t0 I64)
    (Temp t1 Ptr)
    (Temp t2 I64)
    (Temp t3 Ptr)
    (Block bb0
      (ConstInt t0 48)
      (CallRuntime t1 Alloc (t0))
      (ConstInt t2 0)
      (SetField t1 0 t2 I64)
      (MakeClosure t3 "two.7$poll" ())
      (SetField t3 1 t1 Ptr)
      (Return t3)))
  …)
(Diagnostics)
```

The before form, `--dump-mir --before-async`, of the same file prints
`async_lowered=false functions=9`. `two.7` and `one.6` appear there as
lowering left them, `async=true` with their awaits, where the after form
has `two.7$poll`, `one.6$poll` and the two wrappers:

```
  (Function "two.7" params=0 takes_env=false captures=0 ret=I64 async=true
    (Temp t0 Ptr)
    (Temp t1 I64)
    (Temp t2 I64)
    (Temp t3 I64)
    (Block bb0
      (Call t0 "one.6" ())
      (Await t1 t0)
      (ConstInt t2 1)
      (Bin t3 Add Int t1 t2)
      (Return t3)))
```

The first line's other forms, and MIR's diagnostics: `d.nova` is the
header test's `fn main() { let x: Int = "s" }`, which stops in the type
checker; the rest are `cases/mir/`'s.

```
== mir_a_package_module_does_not_run/src/shapes.nova
(Mir runs=false)
(Diagnostics)
== d.nova
(Mir runs=true checked=false)
(Diagnostics)
== mir_e0601_no_main.nova
(Mir runs=true checked=true lowered=false)
(Diagnostics
  (Error "E0601" "no `main` function found; `nova run` needs a `fn main()` entry point"))
== mir_e0601_a_generic_main.nova
(Mir runs=true checked=true lowered=false)
(Diagnostics
  (Error "E0601" "`main` cannot be generic or take parameters"
    (Primary "mir_e0601_a_generic_main":3..7 "invalid `main` signature")))
```

E0013's tuple note prints as `(Note "a tuple has no impls, …")`, from
`lower_tests__a_tuple_map_key_is_e0013_with_a_note.nova`.

`(Mem t0 Load8 (t0))` and `(Extern "js_flag" abi=Js () ret=I8)` occur in
no native dump. The dump's unit test,
`the_dump_prints_one_example_of_every_form`, prints them from a module
built by hand. That module holds every statement, terminator, class
and ABI once, the transform's flag both ways, a diagnostic with no place
and one with a place and a note, and the two parts that never lowered.
The test checks that every form of `KINDS` occurs and that nothing
unlisted does.

**Decisions the spec left open** (the main session's ruling of
2026-11-14 keeps each):
- **Every part ends with `(Diagnostics)`,** empty for `runs=false`,
  `checked=false` and `lowered=true`. Every part is then the same two
  nodes, as every earlier dump's part is, and a reader never asks whether
  a missing node means "no diagnostic". MIR's diagnostics stop lowering,
  so only `lowered=false` holds any.
- **`runs=false` is decided before the root is read**, from
  `Program::for_file_with` (`only_if_runs` in `ResolutionDumps::front`).
  A package's module that does not run is never loaded, so it costs
  nothing, and a missing or unreadable one prints `runs=false`, not
  `IOERR`. Reading a root `check` would not lower, only to report it,
  would be work no comparison needs.
- **An extern prints as `(Extern "sym" abi=C (F64) ret=F64)`.** The ABI
  is a word field, as the HIR dump's `abi=C`, so the form check sees each
  ABI. The return is `ret=`, as a function's is, so `ret=<class>` is one
  family of forms for both. The parameters are a list of classes, which a
  `key=value` field could not carry without making each class a form.
- **A switch arm prints as `0:bb2`,** the same shape as a capture's
  `t3:Ptr`: one word per arm, and a list of pairs that never nests
  parentheses.
- **`(Return _)`** for a return of no value, as an absent destination
  is `_`: one spelling of "nothing" in every operand position.
- **`NOT_NATIVE` is `(Mem` and `abi=Js`,** one more than the spec's "but
  `Mem`". A JS extern is declared only by WASM's std, so the native dump
  never prints one either. Listing both in the code keeps the harness's
  exceptions in one place, beside `KINDS`.
- **5.1's `quote` is re-exported from `nova_resolver::dump`.**
  `nova-mir` depends on `nova-resolver` already, but not on `nova-ast`,
  where `quote` lives. Adding that dependency would change `nova-mir`'s
  manifest and `Cargo.lock`; a copy of the function would be a second
  definition of the same escape rule.
- **One temp a line** (the spec's recommendation): a report then names
  the one temp that differs.

### 3. The hook

`nova_mir::Options` gains `stop_before_async: bool`, false by default.
When set, `lower_module_with` returns the module just before
`async_lower::transform`, with the async `main`'s shim already pushed. It
then skips the two `debug_assert!`s on the transform's output. Natively
nothing else follows the transform; WASM's routing and E0091 are the
target's. No user path sets the option. The call sites that build
`Options` by its fields, the driver's `mir_options` and two of
`lower_tests.rs`, now take `..Default::default()`.

### 4. `MirDumps`

`nova_driver::MirDumps::new(before_async)` sits beside `HirDumps` and
shares its front end and its `SharedItems`, natively and under `check`'s
mode. For each root:
1. A root it cannot read is `IOERR`. A root that does not run prints
   `runs=false` before the loader (§2).
2. `check`'s pipeline runs up to the resolver, stopping on an error.
3. The program is checked: shared with std's functions spliced in
   (§5), or in full on the fallback. An error from any stage before MIR
   gives `checked=false`; warnings do not stop it.
4. `keep_the_entry_main` runs, then native lowering, with
   `stop_before_async` for the before form.
5. The part is printed.

Inside the driver, one check gives any of a root's three dumps:
`HirDumps::dumps(path, Wanted)` returns a `RootDumps` (the HIR dump, the
MIR dump after the transform, the MIR dump before it). The hidden HIR
dump asks for the first alone, the MIR dump for one form, and the guard
for all three (§5). `Checked::into_module` builds `check_with`'s module
from the full check.

`MirDumps::unshared(before_async)` checks every program in full, for
the guard. `nova check`, `build`, `run`, `test` and the language server
keep `check_with` and their own paths: `checkcmp.sh` finds no
difference (§9).

### 5. std's bodies, shared and spliced (the user's decision 2)

- **Kept.** `SharedItems::new` already checked std's bodies, for their
  diagnostics; it now keeps their functions (`SharedItems::functions`).
  The order is `hir::Module`'s: std's functions, methods and constants by
  `DefId`, then its closures and wrappers from `S`, std's number of
  definitions, as they were lifted.
- **Spliced.** `nova_typeck::shared_module(checked, shared)`, beside
  `Rebase` in `check/items.rs`. Let `D` be the program's own definitions
  (`Checked::closures - S`) and `P` its closures and wrappers
  (`Checked::std_closures - Checked::closures`). A `Shift` copies each of
  std's functions and moves its ids:
  - every id below `S` moves up by `D`: the function's own `def_id` and
    its bounds, `Callee::Def`, `MakeClosure`'s function, `MakeVariant`'s
    sum, `MakeRecord`'s record, `TraitCall`'s trait, a `PatVariant`'s
    sum, and every `Ty::Sum`, `Ty::Record` and `Ty::Assoc` on nodes,
    locals, `ret_ty`, type arguments and self types;
  - every closure's or wrapper's id, `S` and up, moves up by `D + P`,
    past the program's own;
  - a lifted function named `closure$N` or `fnval$N`, whose name holds
    its id, takes the new id;
  - nothing else changes: locals, variant, field and method indices,
    names, and spans, whose files every program registers std-first.

  No id beyond the spec's list was needed. The walk matches every
  `ExprKind`, `Ty` and `Pattern` variant with no catch-all arm, so a new
  kind will not compile until the splice handles it.
- **Where they go.** std's functions follow the program's functions,
  methods and constants, and std's closures follow the program's, as the
  full check orders `hir::Module::functions`
  (`Checker::check_bodies`). The tables are the shared check's, std's
  parts already rebased (ADR 0055 §4).
- **Equal to the full check, byte for byte.**
  `shared_module_is_the_full_checks_module` (`items.rs`) compares
  `shared_module`'s module with `check_with`'s by their `Debug` text.
  It runs three programs, with no closure, one, and four with a wrapper,
  a method and a constant, against a std of one more module. That module
  holds a closure, a wrapper, a record, a sum, a trait, an impl, a
  generic function and a constant, so `D` and `P` both vary.

  The test passed on its first run. Each of these breakages makes it
  fail: moving closures by `D` alone, keeping the old names, leaving a
  pattern's sum unmoved, or leaving `Assoc`'s id unmoved (std's own
  bodies hold projections).
- **The guard.** The driver's
  `std_checked_once_dumps_what_std_checked_for_each_program_dumps`
  compares, natively, each root's shared and unshared MIR dumps in both
  forms, beside its HIR dump and from the same check. It covers every
  corpus root and every case of `cases/resolve/`, `cases/check/` and
  `cases/mir/`, in order and reversed, so no root is checked twice for
  MIR. 677 roots lower.
- **The fallback holds** (ADR 0056 §4). A program that implements
  anything on a type it does not own takes the full check and is not
  spliced; so does every program when std alone gives a diagnostic or
  holds a test. The guard lists the roots that take it: 85 natively,
  where 5.3b had 78. The new seven are `cases/mir/`'s extracted E0013
  sources that implement a trait on a primitive.

### 6. Native only (differs), and why (the user's decision 4)

5.4 compares native MIR alone.
- **WASM's MIR differs:** its own std, `route_runtime`, its roots and
  exports, `Mem` statements, `Js` externs and E0091. Only `check --target
  wasm` shows it, and N's backend is native (ADR 0048).
- **Test mode's MIR** exists only under `nova test` (a synthesized
  `main`, `std/test`, kept `@test`s, package-wide), which Phase 5 does not
  port.

So the form takes no `--target` or `--test`, and the 35 WASM programs
are not compared. Both stay S0's, recorded here as (differs).

### 7. The cases

`cases/mir/` (spec §7.1 items 3 and 4), 83 roots.

**Extracted:** the sources `nova-mir`'s native tests lower.
- A temporary instrumentation (a script outside the repository, never
  committed) recorded every source passed to `lower_tests.rs`'s
  `mir_for`, `diagnostics_for` and `wasm_mir_without`, and to
  `pattern_lowering.rs`'s `mir_for`, which takes `program()`'s sources.
  It also recorded the inline sources of three tests whose pipelines are
  their own and which a program can reach through the driver:
  `an_async_main_is_wrapped_in_a_shim_that_drives_it`,
  `a_tuple_map_key_is_e0013_with_a_note` and
  `a_tuple_of_one_in_a_bound_message_keeps_its_comma`.
- Every `format!` is expanded, `deep_wrap_program` included. A case is
  `<testfile>__<test>.nova`, or `_<k>` after the test's name when the
  test lowered more than one native source, `k` the source's place among
  them.
- 89 sources were recorded. The 85 native ones keep 74, after
  dropping those whose text is a case of `cases/check/` already (7, each
  of them clean, so it reaches MIR as a HIR case) and repeats within the
  set (4).
- None of the 74 stops before MIR through the driver, the
  `diagnostics_for` sources included. 62 lower clean; 12 give E0013 at
  monomorphization's bound check, two of those with the tuple note.

**Left out:**
- **The 4 WASM sources** (the research counted 7 call sites):
  - `a_runtime_function_wasm_lacks_is_e0091`;
  - `a_wasm_build_calls_std_rt_where_a_native_build_calls_the_runtime`
    (its native `mir_for` source is kept);
  - `a_wasm_build_holds_only_the_runtime_functions_it_reaches`;
  - `a_wasm_build_of_an_async_main_holds_the_event_loop`.
- **5 tests whose programs need a stand-in std module**, or WASM's std,
  which no program can give the driver:
  - `http_parse_request_lowers_to_a_runtime_call`;
  - `crypto_builtins_reach_their_runtime_functions`;
  - `the_ui_state_builtins_lower_to_the_runtime_or_std_rt`;
  - `the_router_state_builtins_lower_to_the_runtime_or_std_rt`;
  - `a_std_extern_js_block_is_wasms_alone`.
- **Rust-only:** the 38 inline tests, on hand-built MIR and HIR.

**By hand:**
- `mir_e0601_no_main` (the E0601 with no place);
- `mir_e0601_a_generic_main` (`fn main<T>()`);
- `mir_e0075_conflicting_externs/` (a program of three modules
  declaring `sqrt` twice);
- `mir_e0078_a_wide_chain` and `mir_e0078_a_deep_chain` (both of
  E0078's limits, as `run_tests.rs`'s
  `mono_reports_which_normalization_limit_a_projection_overflowed`
  builds them, at 16 and 65 links);
- `mir_e0079_an_unbound_projection`;
- `mir_every_native_form` (§2's example: every statement a native build
  prints, every terminator, an extern, every class as a return, both
  values of each flag);
- `mir_a_package_module_does_not_run/` (its `src/main.nova` lowers, its
  `src/shapes.nova` is `runs=false`, so N's `runs` is compared on a
  case).

**The codes.** Among the cases occur:
- E0601 at both sites;
- E0013 at `mono.rs:276` (from the extracted sources);
- E0075;
- E0078 at both limits;
- E0079.

Three sites have no case:

- **E0013 in lowering** (`lower.rs:974`, "no implementation of trait …
  for the receiver type") **is unreachable from source** (the main
  session's ruling, 2026-11-14). The spec's claim that it was reproduced
  is wrong: the research's two E0013 probes both reach `mono.rs:276`.
  N ports it as written, and the gate reads "E0013 at `mono.rs:276`".
  The reasoning:
  - `resolve_method_full` filters the impls as `impl_satisfies` does,
    by trait and head, then by `match_args`, but without the impls'
    bounds. So it finds an impl whenever the bound check passes.
  - Monomorphization checks every bound of an instance before lowering
    it, supertraits, an impl's bounds and a method's own bounds included,
    and does not lower an instance that fails.
  - The type checker resolves a trait call on a concrete receiver
    structurally (E0014 otherwise), requires a supertrait's impl
    (E0072), and refuses bounds on an associated type (E0900).

  The prototype's 13 probes (scratch, `l974*.nova`), with S0's outcomes:
  - a bound on an associated type: E0900 and E0014;
  - a supertrait's default body over a type with no impl of the
    supertrait: E0072;
  - a conditional trait impl called directly: E0013 at `mono.rs:276`;
  - a conditional inherent impl called directly: E0013 at `mono.rs:276`;
  - an impl on `Box<Int>` called on `Box<Bool>`: E0014;
  - the same through an unbounded generic function: E0014;
  - an impl on `Pair<T, T>` through an unbounded generic function:
    E0014;
  - a subtrait's impl over `W<T>` with the supertrait's on `W<Int>`
    alone: E0072;
  - a method on `!`: E0014;
  - `!` compared with `!`: lowers clean, with no trait call;
  - `!` passed through an `Eq`-bounded parameter: the type checker's
    E0013 on `==`;
  - a receiver whose type is still an inference variable, twice: E0014.

  No input of the corpus or the cases prints its message.
- **E0011 in monomorphization** is unreachable, as the research judged:
  `mono` normalizes type arguments before they reach it, and nothing
  gives it a parameter or a variable. Ported as written, no case.
- **E0091** is the WASM target's (§6).

### 8. The harness, S0's side (part 1)

`selfhost.rs` gains `MIR_S0`, `MIR_BEFORE_S0`, `MirForm`, `mir_in` (N's
side, `mir --std DIR [--before-async]`, for later parts) and `N_MIR`,
false in part 1. While it is false, the MIR inputs run through S0 alone,
through `run`, so `settled_stdout` settles them. `settled_stdout` now
takes S0's `check --dump-mir` and N's `mir` as it takes the HIR dump's;
no MIR message names a path, so nothing changes today.

The inputs:
- the native corpus's roots that run: the 251 loose programs and the 10
  packages' `src/main.nova`, N's among them;
- the HIR cases that reach MIR: the 342 roots of `cases/check/` whose
  part in the HIR test's run holds no error;
- `cases/mir/`'s 83 roots;
- the cases again with `\r\n`.

Root lists pass `HIR_ARGS_LIMIT` in parts (`agree_on_mir_in`, through
`agree_in_parts`).

The checks:
- a header and a `(Mir` line per root;
- well formed (`assert_well_formed`);
- every form of `KINDS` but `NOT_NATIVE`'s, and nothing unlisted;
- `(Await` and `(Function async=true` in the before form alone;
- the two forms equal on every root whose after form holds no `$poll`
  function (76 corpus roots and 19 cases hold one);
- each of `MIR_CODES` at its site among the cases;
- with `\r\n`, the same parts and codes.

Tests: `the_mir_dump_flag_is_hidden_and_refused_as_the_spec_says`,
`the_mir_dump_has_a_header_per_root_and_says_how_far_each_got`,
`every_run_of_a_mir_dump_settles_e0004s_paths_on_windows`,
`the_mir_dumps_agree_on_the_corpus_and_the_cases` and
`the_mir_dumps_agree_on_the_cases_with_windows_line_endings`. Later parts
set `N_MIR`, and every set is then compared byte for byte.

### 9. Part 1's figures

Measured locally on 4 cores, 2026-11-14, at `599ac774`, against the
base `e73353cf` (main `9389a84b` plus the spec).

**The dump's size**, S0, every root of the set:

| Set | Roots | After the transform | Before it |
|---|---:|---:|---:|
| loose corpus | 251 | 397,307 lines, 9,709,606 bytes | 349,848 lines, 8,413,584 bytes |
| packages' mains but N's | 9 | 72,932, 1,744,590 | 72,932, 1,744,597 |
| N's `selfhost/src/main.nova` | 1 | 195,598, 4,702,226 | 187,912, 4,481,610 |
| native corpus, all | 261 | 665,837, 16,156,422 | 610,692, 14,639,791 |
| HIR cases that reach MIR | 342 | 18,247, 478,928 | 18,141, 476,629 |
| `cases/mir/` | 83 | 7,067, 186,819 | 6,004, 159,728 |

**S0's time** for each set, best of three, in seconds. The column is
the opt-level and the form: 0 is the plain debug build, 1 is
`CARGO_PROFILE_DEV_OPT_LEVEL=1`.

| Set | 0, after | 0, before | 1, after | 1, before |
|---|---:|---:|---:|---:|
| loose corpus | 4.40 | 4.53 | 1.89 | 1.92 |
| packages' mains but N's | 0.40 | 0.34 | 0.14 | 0.13 |
| N's root | 0.98 | 0.96 | 0.42 | 0.41 |
| HIR cases that reach MIR | 5.42 | 5.63 | 2.40 | 2.28 |
| `cases/mir/` | 1.34 | 1.41 | 0.54 | 0.55 |

**The splice against the full check,** per program that reaches MIR,
from timing hooks in a scratch build. Each figure is the type check
plus the module's build, in milliseconds. The shared rows include the
roots that fall back to the full check.

| Set | 0, shared + splice | 0, full | 1, shared + splice | 1, full |
|---|---:|---:|---:|---:|
| loose corpus (212) | 7.8 (splice 4.2) | 32.0 | 2.9 (splice 2.0) | 10.0 |
| HIR cases (342) | 9.0 (3.8) | 33.0 | 3.2 (1.8) | 10.4 |
| `cases/mir/` (82) | 8.7 (3.8) | 34.0 | 3.5 (2.0) | 11.1 |
| packages' mains (8) | 51.8 | 86.4 | 34.0 | 34.1 |

The splice copies all of std's functions for each program, about 2 ms
at `opt-level = 1`, more than the program's own shared check on most
roots. It is still a third of a full check.

**`selfhost.rs`**, three runs at most:
- as CI's Selfhost job runs it (`NOVA_EMBED_RUNTIME=1
  CARGO_PROFILE_DEV_OPT_LEVEL=1`): 37.62, 37.53 and 38.03 s before (55
  tests); 44.27, 43.75 and 42.66 s after (60 tests);
- plain: 125.87 and 135.20 s before; 147.10 and 149.09 s after.

**The driver's unit tests:** `nova-driver`'s 42 alone took 77.7 s
before and 85.4 s after. The guard alone took 61.8 and 62.1 s with HIR
only (the base's work), 72.3 s with both MIR forms, and 75.2 s once
`cases/mir/` joined it. 5.3b's first CI run took 93.88 s for this binary
on Linux.

**The full suite** (`cargo test --workspace`, Node required): 2,835
passed, 1 failed, 10 ignored, against the base's 2,833, 1 and 10. The
one failure is the IPv6 test, which fails here before and after. The two
new tests are `nova-mir`'s `the_dump_prints_one_example_of_every_form`
and `nova-typeck`'s `shared_module_is_the_full_checks_module`.

**Byte for byte with the base**, by scripts kept outside the repository
(5.3b's, adapted), over every `.nova` file the base tracks:
- `nova check`: 1,943 checks, none differs;
- `nova parse --check` and `--ast`: 3,674 runs over 1,837 files, none
  differs;
- `nova test` and `nova build` in every package: 22 runs, none differs;
- every `--help`: 22 of 22, no hidden flag shown;
- usage errors: 474 runs under six `argv[0]`s and 10 in a terminal, all
  identical; every refusal of the hidden forms (114 of
  `--dump-resolution`'s, 330 of `--dump-hir`'s and `--dump-mir`'s) one
  error, exit 2, nothing on stdout.

`Cargo.lock` is unchanged.
