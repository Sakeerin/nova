# Phase 5.5a, "the `.ll` emitter, ported to Nova, its output compared byte for byte": design

- **Date:** 2026-11-18.
- **Status:** approved by the user (2026-11-18).
- **The user's decisions, 2026-11-17:**
  1. **5.5 splits into 5.5a and 5.5b.** 5.5a: the emitter, and its `.ll`
     compared byte for byte on all three systems, with no `clang`. 5.5b:
     the driver, linking, N's `build` and `check`, and the golden suite
     with S1; its questions are asked when 5.5b starts.
  2. **The `.ll` printed beside the MIR:** a hidden option of `nova check
     --dump-mir` prints each root's `.ll` after its MIR after form, in the
     same process; N's `selfhost mir` the same; compared in the existing
     MIR tests; about +2 to 4 s of CI; every `--help` unchanged.
  3. **Every input that lowers:** the corpus's 210 roots that lower clean
     (N's own among them), the 407 cases that lower clean,
     `ir_tests.rs`'s sources as about 18 cases, the MIR fuzzer's 120
     inputs, the 25 depth cases and a new float `%` case; all three
     systems; no `clang`.
  4. **The emitter's errors ported as written, with a case:** N ports
     every `bail!` as written and prints its error in the dump as S0
     does; a case for float `%`; the others recorded unreachable natively,
     as 5.4's E0011; S0's bug (float `%` type-checks, then the emitter
     refuses it) not fixed.
  5. **The `.ll` mismatch report names the `define` and the block,** as
     5.4's MIR report: the root, the function and the `bbN` above the
     first differing line, both lines, the reproduce commands; with a test
     and mutants.
  6. **No new fuzzer:** 5.4's inputs, and a test that every statement,
     terminator and constant form the emitter knows, and each of the 111
     `RtFunc`s, occurs in the compared `.ll`; hand cases fill the gaps.
  7. **CI measured from the prototype first:** pushed to
     `claude/loving-archimedes-frtf5q-55-ci` with a draft pull request,
     measurement only, closed unmerged, as #142; the push itself is asked
     again before it happens; past a line, the user is asked.
  8. **The rule:** from 5.5, a change to the emitter, to `RtFunc`'s
     symbols or signatures, or to the runtime's ABI changes S0 and N in one
     pull request; recorded in the ADR and the CHANGELOG.
- **Not asked; the recommendation taken, which the user can overturn:**
  the option is `--ll`, refused with `--before-async` (§3.1); the `.ll`
  follows the root's `(Diagnostics)` under a line `(Ll ok)`, an error is
  one line `(Ll error "…")`, and a root that does not lower prints none
  (§4); the new cases go in `cases/mir/` (§6.1); the coverage test reads
  the MIR after form of the roots whose `.ll` was compared, and the `.ll`
  for what only it shows (§6.3); the driver's guard is not extended
  (§3.3); N builds a root's `.ll` whole, as S0 does (§5.2); the `Mem`
  error has nothing to port (§5.3); one pull request, split into tasks,
  test first, as 5.4 (§10).
- **Branch:** `claude/loving-archimedes-frtf5q-55`, from `main` at
  `1570ddbd` (5.4's merge), for this spec; the plan and the work follow
  on it. Nothing is pushed without the user's word.
- **Inputs:** `docs/phase-5-plan.md` (§3, §4's 5.5 and 5.6); ADRs 0053 and
  0057; the 5.4 spec; the research (summarised in §2).

## 1. What 5.5a delivers

N's seventh stage is **the `.ll` emitter**, what S0's
`nova_codegen_llvm::compile_ir` prints from native MIR after the async
transform. The hidden `--ll` of `nova check --dump-mir` prints it per
root after the root's MIR, N's `selfhost mir --ll` the same, and the MIR
tests compare both byte for byte on every input, per host. From 5.5a the
emitter exists twice (§11).

**The phase plan is amended (decision 1).** 5.5 splits:
- **5.5a (this spec):** `RtFunc::symbol`/`signature` and the emitter
  ported, the `.ll` dump, the harness. N is still built by S0's `nova
  build` (Cranelift); nothing needs `clang`. The plan's public
  `--emit-ll` becomes the hidden `--ll` of the MIR dump (decision 2).
  The gate is §16.
- **5.5b:** the driver, `clang` and `cc`, finding the runtime, N's `build`
  and `check`, the golden suite built by S1. Its open questions are
  deferred, not decided here (research §11, items 3 to 8):
  1. where the S1 golden suite runs (the Selfhost job's Ubuntu leg with
     clang-18, a job of its own, the Test job), which runtime it links,
     and whether S1 is built by Cranelift or `--release`;
  2. the intermediate file's name (the tolerance): one `-o` basename for
     every stage, a constant name, or a `source_filename` line;
  3. N's command line and exit codes, how N finds std and the runtime,
     project mode without `current_dir`, whether stderr is compared;
  4. `check`'s output: codespan's layout ported, or codes and spans in the
     dumps' form (ADR 0048 decision 1);
  5. N's `build` without `--release` (N has no Cranelift): `-O2` or `-O0`;
  6. platforms: Windows `link.exe` discovery is not portable; N's `build`
     on Linux and macOS only, or an OS query in std, in both compilers.

## 2. The starting point

Measured on `1570ddbd`, 2026-11-16 and 17 (the research), and on
2026-11-18 for this spec with the debug S0 left in `target/` by 5.4's
last build (not rebuilt; the base's last commit changed only tests and
docs). Line numbers re-checked for this spec.

1. **S0 has no `--emit-ll`.** The `.ll` exists only as `build
   --release`'s intermediate `<out>.nova.ll` (`intermediate`,
   `nova-driver/src/lib.rs:860`), written after `compile_ir` (`:888`),
   removed after a successful link. With `NOVA_CLANG=/nonexistent
   NOVA_LLC=/nonexistent` it is kept: exit 1, "release build failed;
   generated LLVM IR left at …".
2. **The emitter is small and reads only MIR:**
   `nova-codegen-llvm/src/lib.rs`, 761 lines, 633 of code, no inline
   test; `compile_ir(&Module)` (`:65`) takes nothing else. With
   `RtFunc::symbol` and `signature` (`nova-mir/src/lib.rs:589-706`,
   `:708-915`, 241 code lines) about 874 Rust lines to port, about 900 to
   1,000 Nova lines at 5.4's ratio. `tests/ir_tests.rs`: 749 lines, 19
   tests, 20 sources through `ir_for` (`:12`), one a second hello and one
   `tests/runtime/float_bits.nova`, already corpus: about 18 new sources.
3. **The `.ll` is deterministic and names no host.** Two unordered maps,
   lookups only: `ret_of` (`:71`), `string_index` (`:79`); strings
   numbered by first use; `%rN` restarts per function (`:89`). The 210
   corpus roots and 407 cases gave the same bytes twice, the 210 also from
   another directory and `-o` name. No triple, datalayout, metadata or
   path is in the text. `\r\n` reaches it only through a raw line break
   in a string literal (`c"a\0D\0Ab"`); 175 of 175 `tests/runtime`
   programs gave the same `.ll` from `\r\n` copies.
4. **What `build --release` emits** (`build_program_release`,
   `:883-918`): `lower_to_mir` (`:1306-1325`), that is the full check
   (`check_with`), `keep_the_entry_main` (`:1574`; the runtime impls only
   for WASM, `:1575-1577`), and `lower_module_with` with
   `mir_options(None)` natively (`:960`), `stop_before_async` false. The
   MIR dump's after form lowers the shared check's module with std's
   functions spliced in, or the full check's, then `keep_the_entry_main`
   and the same options (`:504-514`). 5.4's unit test and guard hold the
   two modules equal (ADR 0057 §5).
5. **The emitter's errors** (`bail!`): no `main` (`:67`), `Await`
   (`:389`), `Mem` (`:399`), float `%` (`:695`), float bitwise (`:697`).
   Float `%` alone is reachable: it type-checks, and `build --release`
   prints "Error: internal LLVM IR generation error (this is a compiler
   bug)", "Caused by:" and the bail's words, exit 1; `%=` the same.
   Float bitwise is not: `&`, `|`, `^`, `<<`, `&=` and `a & b` in a
   generic function are E0013 before MIR, `~` on a `Float` E0010; lowering
   makes no other float `Bin` (both measured for this spec). No `main` is
   E0601 first; `Await` is in the before form alone; `Mem` is WASM's.
6. **Sizes of the `.ll`:** hello 137 lines, 5,262 bytes; corpus median
   56,935 bytes; N's root 379,438 lines, 11,842,969 bytes, 1,970
   `define`s; the 210 corpus roots 38.85 MB; the 407 cases 4.42 MB.
   Projected: the fuzzer's about 14 MB, the depth cases' about 4.5 MB,
   about 62 MB in all.
7. **Coverage today** (measured for this spec, S0's after form over the
   210 corpus roots and 407 cases): every native statement and
   terminator; 27 of the 28 (operator, class) pairs the type checker
   admits, all but `Le` on `Float`; `Switch` on `I64` discriminants only,
   never `I8`, which a `Bool` match makes (`ir_tests.rs`'s
   `bool_match_switches_on_i8_not_i64` is one); 108 of the 111 `RtFunc`s
   called. Never called: `TestSelector` (test mode's synthesized `main`
   alone), `TaskIsDone` (no native std function calls it; a user's call
   is E0001), `TaskTakeOutput` (no MIR statement emits it,
   `nova-mir/src/lib.rs:253-258`). Every `.ll` declares all 111.
8. **Time.** S0's emit: 15% of `build --release`'s instructions on N's
   root, +0.7 to 1.0 s of CPU over the corpus. N prints at 400 to 550
   instructions a byte, its collector about 36%. CI today (#143, run
   43850851663): `selfhost.rs` took 71.98 s on Linux, 89.34 s on macOS and
   96.77 s on Windows, against lines of 80, 100 and 110 s (ADR 0057 §19).
9. **N:** 53 files, 30,321 lines; `StringBuilder`, byte access,
   `Float.to_bits`; `hex16` (`selfhost/src/dump.nova:92`) prints
   lowercase, `push_escaped` (`:73`) writes lowercase `\xHH` and keeps
   bytes past ASCII. N's MIR has no `Mem` statement
   (`selfhost/src/mir.nova:260`); an `RtFunc` is an index into
   `RT_FUNC_VARIANTS` (`:74`). No language blocker.

## 3. S0's change

The smallest that works; none visible to a user.

### 3.1 The option (decision 2)

`nova check --dump-mir --ll FILE…`: each root's after part, then, for a
root that lowered, its `.ll` (§4).

- **ADR 0053's mechanism, as ADR 0057 §1:** `DumpMirCmd`
  (`nova-cli/src/cmd/run.rs:562`) gains `--ll`, once, anywhere after
  `--dump-mir`. A second `--ll`, `--ll=…` and every other shape stay one
  clap error under the usage `nova check --dump-mir [OPTIONS]
  [FILES]...`, exit 2. `nova check --ll` and `--dump-ll` are not the form:
  the base's refusal stands, byte for byte. `CheckCmd`, `--help`, clap's
  tips and every other command line stay the base's.
- **Refused with `--before-async`:** one line on stderr, `error:
  --dump-mir --ll takes no --before-async`, nothing on stdout, exit 2, as
  the resolution dump refuses `--test` with `--target wasm` (`dump_code`).
  The emitter reads the after form alone; the before form holds an
  `Await` in every async program, which would make each one's `.ll` an
  error.
- **Without `--ll`** the output is 5.4's, byte for byte.

### 3.2 Where it is printed

`MirDumps` (`nova-driver/src/lib.rs:592`) takes whether to print the
`.ll`. In `dumps_with`, when the after form lowered (`after`, `:514`, is
`Ok(module)`), the driver calls `nova_codegen_llvm::compile_ir(&module)`
and appends §4's part to the after part, an error's words through
`nova_resolver::dump`'s `quote` (5.1's). The printing lives in the
driver: `nova-mir` cannot depend on the emitter, and the emitter's crate
does not depend on `nova-resolver`, so another place would change a
manifest and `Cargo.lock`. S0's emitter does not change.

### 3.3 The `.ll` is the intermediate `build --release` writes

The dump emits the after form's module, the one `build --release` emits
(§2 item 4). **Checked by** `the_ll_dump_is_what_build_release_writes`
(`selfhost.rs`, S0 alone): on a few programs (a loose program, a
package's `src/main.nova`, an async program, one with an extern,
`mir_every_native_form.nova`), `nova build --release FILE -o OUT` with
`NOVA_CLANG=/nonexistent NOVA_LLC=/nonexistent` and a fresh `$NOVA_HOME`
exits 1 and leaves `OUT.nova.ll`, equal byte for byte to the root's
`.ll` in the dump; on the float `%` case `build` leaves no `.ll`, and its
stderr ends with the words the dump's `(Ll error` line quotes.

**The driver's guard is not extended** (not asked): `compile_ir` is a
function of the module alone, so equal MIR gives equal `.ll`, and the
guard is the Test job's slowest binary (ADR 0057 §19).

**Also checked by:** the MIR flag's test, extended (`--ll` in no help or
usage line, the base's errors around it, the form's refusals); `--dump-mir
--ll` without its `(Ll` parts equals `--dump-mir` on the header test's
roots; a driver unit test: `(Ll ok)` and the text for a small module,
and `(Ll error …)` with each error's words for modules built by hand (an
`Await`, a float `Rem`, a float `BitAnd`, no `main`).

## 4. The `.ll` in the dump

```
== examples/01-hello-world/src/main.nova
(Mir runs=true checked=true lowered=true async_lowered=true functions=1 externs=0
  …)
(Diagnostics)
(Ll ok)
; LLVM IR emitted by the Nova compiler (release backend).
; Requires LLVM >= 15 (opaque pointers).

declare void @llvm.trap()
…
define void @"nova_main"() {
entry:
  …
}

@str0 = private unnamed_addr constant [13 x i8] c"Hello, World!", align 1
…
}
== the next root
```

- **`(Ll ok)`, then `compile_ir`'s text, byte for byte,** ending in its
  own line break. No closing line: the `.ll` runs to the next `== `
  header or the end. No `.ll` line starts with `(` or `== `: each is a
  comment, a `declare`, a `define`, a label, an instruction indented two
  spaces, `}`, a string global or blank, and a literal's line breaks are
  escaped.
- **An emitter error is one line,** `(Ll error "float remainder (%) is
  not supported by the LLVM backend")`: the bail's own words, without the
  driver's context line; exit 0, as a part with MIR's diagnostics.
- **A root that did not lower prints no `(Ll`:** `runs=false`,
  `checked=false`, `lowered=false` and `IOERR` show how far it got
  already, and `compile_ir` is never called on it. `(Ll` occurs exactly
  when `lowered=true`.
- **The smallest form that keeps 5.4's readers:** the part still opens
  with `(Mir` and 5.4's grammar still ends at `(Diagnostics)`; the
  harness cuts each `(Ll` part off before 5.4's checks (§6.2), and the
  report's walk enters the `.ll` at `(Ll ok)` (§6.4). Quoting each line
  into 5.4's grammar would change the bytes, so the `.ll` would no longer
  be the intermediate, and would cost N quoting 62 MB.
- **Size:** about 62 MB more over every input, N's root 11.84 MB (§2).

## 5. What N ports

### 5.1 The code and files

- **`RtFunc::symbol` and `signature`** (5.4's decision 7) in
  `selfhost/src/mir.nova`, beside `RT_FUNC_VARIANTS`: the 111 symbols and
  signatures in `RtFunc::ALL`'s order, indexed as the variants are.
- **The emitter,** `selfhost/src/emit_ll.nova` (new, about 900 to 1,000
  lines, under 5.4's 1,500 a file), with S0's names: `compile_ir`,
  `rt_func_decl`, `llty`, `escape_bytes`, `float_hex`, `FnEmit` and its
  helpers, `stmt`, `arg_list`, `emit_call`, `binop`, `terminator`.
- **`selfhost mir --std DIR [--before-async | --ll] FILE…`**
  (`selfhost/src/main.nova:285`): `--ll` once; with `--before-async`,
  twice or as `--ll=…`, N prints its usage line on stderr, nothing on
  stdout, exit 2. `mir_of` (`:427`) appends §4's part to a
  `lowered=true` part, from the module it lowered.
- **`selfhost/src/emit_ll_tests.nova`:** `@test`s for the escapes of each
  byte class, `float_hex` of `-0.0`, a NaN and an exponent, `ret_of`'s
  duplicate key and missing callee, the symbols distinct (`rt_func_tests`
  ported), and the errors' words.

### 5.2 Idioms

- **Text:** `StringBuilder`, parts and one join, as S0 builds `body` then
  `out`. A root's `.ll` is built whole (not asked) and printed with its
  part; if N's memory on its root asks for it (§9), the header, functions
  and globals are printed as made, the same bytes.
- **Escapes and floats:** a new `escape_ll`, `\XX` with two uppercase
  digits and no `x`, for every byte outside 0x20 to 0x7e and for `"` and
  `\`, by bytes (`byte_at`), never by characters; `push_escaped` is not
  reused. A float is `0x` and the 16 uppercase hex digits of
  `Float.to_bits` (S0's `0x{:016X}`), by an uppercase helper beside
  `hex16`. An empty string is `[1 x i8] c"\00"`; lengths are byte
  lengths. Integers in decimal by interpolation.
- **`ret_of` and `string_index`** as `Map` lookups only (phase plan
  decision 9), the strings in a `Vec` by first use. **`ret_of`'s
  duplicate key:** S0 collects the functions, then the externs, into one
  `HashMap`, so an extern's class replaces a function's of the same
  symbol; N inserts in that order and overwrites. A callee missing from
  it is `Unit` (`:378`).
- **Codes, not `==` on variants,** as 5.4: `MirTy` to its LLVM spelling by
  a table, `RtFunc` by index. Hard-coded as in S0: 8-byte slots, `i64`
  sizes, the heap layouts, the env-first indirect call, `main` emitted as
  `nova_main` and the C wrapper.

### 5.3 The errors (decision 4)

N's `compile_ir` returns `Result<String, String>`, each error in S0's
words:
- **float `%`:** reachable; its case (§6.1);
- **float bitwise, no `main`, `Await`:** ported as written, unreachable
  natively (§2 item 5), each pinned by an N `@test` on a module built by
  hand and by S0's unit test with the same words (§3.3); the ADR records
  them, as ADR 0057 §7 records E0011;
- **`Mem`:** not ported: N's MIR has no `Mem` statement (5.4 left WASM's
  out), so the arm has nothing to match; recorded in the ADR.

## 6. The harness

### 6.1 Inputs (decision 3)

`MirForm::After` (`selfhost.rs`) becomes S0's `check --dump-mir --ll` and
N's `mir --std DIR --ll`, so the `.ll` is compared wherever 5.4 compares
the after form, through `compare_batch`, byte for byte with exit codes,
per host, nothing normalized:
1. **the native corpus's roots that run** (251 loose, 10 packages'
   `src/main.nova`): 210 lower clean and print a `.ll`, N's
   `selfhost/src/main.nova` among them;
2. **the HIR cases that reach MIR and `cases/mir/`:** 407 lower clean;
3. **`ir_tests.rs`'s sources,** about 18, extracted as 5.4 extracted
   `lower_tests.rs`'s, as `cases/mir/ir_tests__<test>[_n].nova` (not
   asked: `cases/mir/` is already an after-form input with both line
   endings, so no new list or test; their MIR is compared too);
4. **the float `%` case,** `cases/mir/ll_a_float_remainder_errs.nova`,
   and the hand cases §6.3 asks for, `cases/mir/ll_*.nova`;
5. **the MIR fuzzer's 120 inputs and the 25 depth cases** (ADR 0057 §16,
   §17), unchanged; N runs the depth cases with `--ll` on a 1 MB main
   thread. The emitter does not recurse on nesting, but symbols reach 520
   characters: N's least stack is measured in the prototype; if N
   overflows, 5.3b's answer 2 holds, as in 5.4.

`\r\n`: the cases again with `\r\n`, as 5.4, the `.ll` included; a hand
case holds a string literal with a raw line break, so `\0D\0A` is
compared. Windows: root lists past `HIR_ARGS_LIMIT` in parts, as 5.4;
`settled_stdout` touches no `.ll` line (none starts `  (Error`). The
before form stays 5.4's, without `--ll`.

### 6.2 5.4's checks, and refusals

One helper, `split_ll`, cuts each root's `(Ll` part off S0's output after
the comparison, so every 5.4 check reads 5.4's text: well formed, a
header and a `(Mir` line per root, `KINDS`, the two forms equal where no
`$poll` occurs, `MIR_CODES`. **New:** `(Ll ok)` or `(Ll error` exactly
when `lowered=true`; `(Ll error` on the float `%` case alone, with its
words; each `.ll` ends in a line break.
`both_sides_refuse_the_mir_dumps_shapes_alike` gains `--ll` twice,
`--ll=x`, and `--ll` with `--before-async` in both orders: exit 2 and
nothing on stdout on both sides.

### 6.3 The coverage test (decision 6)

`every_form_the_emitter_handles_occurs_in_the_compared_ll`, on the corpus
and the cases (one run, as 5.4's `KINDS` check), reads S0's output of the
roots whose `.ll` was compared:
- **their MIR after form,** which determines their `.ll`: each of the 25
  native statement kinds and 5 terminators; each constant form
  (`ConstInt`, `ConstFloat`, `ConstBool`, `ConstStr` and an empty one,
  `ConstUnit`); the 28 (operator, class) pairs the type checker admits
  (16 on `Int`, 10 on `Float`, `Eq` and `Ne` on `Bool`); `Neg` on `Int`
  and `Float`; `Switch` on `I8` and `I64`; `Return` with and without a
  value; a `Unit` temp; a function taking an env; a `Call` to an extern;
- **the `.ll` itself:** each `RtFunc`'s symbol in a `call`; string
  globals with each escape (`\22`, `\5C`, a control byte, a byte past
  ASCII); `c"\00"`; an extern's `declare`.

`LL_UNREACHABLE` lists what may be absent, each with its reason:
`TestSelector`, `TaskIsDone`, `TaskTakeOutput` (§2 item 7) and the
errors but float `%` (§5.3). Missing today: `Le` on `Float` and `Switch`
on `I8` (§2 item 7); `ir_tests.rs`'s sources or a hand case fill them,
the plan confirms which. All 111 declarations are in every `.ll`'s
header, so every symbol and signature is compared on every input,
whatever the calls.

### 6.4 The `.ll` mismatch report (decision 5)

`compare.rs`'s `MirWalk` gains the `.ll`: at a depth-0 line `(Ll ok)` it
enters the root's `.ll`, counting its lines; a `define` line names the
function (the symbol in `@"…"`, or `main` for the C wrapper); a label
(`entry:`, `bbN:`) names the block; `}` leaves the function. After the
root and the line of its dump, the report gives `line 702 of its .ll,
define "g.1", block bb1`; outside a function (a declaration, a string
global) `line 7 of its .ll, outside any define`; at the `(Ll` line, `at
its (Ll line`. Both lines and the reproduce commands (`nova check
--dump-mir --ll FILE`, `selfhost mir --std DIR --ll FILE`) follow, as for
every dump; every other report is as it was.

**Checked by** `the_ll_mismatch_report_names_the_define_and_block`, on
hand-written dumps with two `define`s in one root and a difference in
the header, a declaration, `entry:`, a block and a string global; and by
**mutants** run once in the prototype, each failing the differential
with the right `define` and block: lowercase `float_hex`, a lowercase or
`x` escape, `ret_of` with the function winning, `%rN` not restarted,
strings numbered by last use.

## 7. Errors and diagnostics

No new diagnostic code; S0's emitter, MIR and diagnostics do not change.
The emitter's error stays an internal error in `build --release` (float
`%`, S0's bug, not fixed); in the dump it is data, `(Ll error "…")`,
exit 0. The dump's other exit codes are 5.4's: 2 for an unreadable root,
after the rest, and for a refused shape.

## 8. Testing

Unchanged rules: tests never touch the internet; fixed temporary names;
`$NOVA_HOME` fresh where needed; MSRV 1.78 (no `is_none_or`, `LazyLock`,
`std::path::absolute`); no new crate, `Cargo.lock` unchanged; no `clang`
(§3.3's test points `NOVA_CLANG` and `NOVA_LLC` at a missing file). Then
§3's S0 tests, §6's differential beside 5.4's, and N's `@test`s, which
also run on the debug runtime in the Selfhost job's last step.

## 9. CI time (decision 7)

- **The projection** (research §9, not measured): the `.ll` printed in
  the after form's processes, about +1 to 2 s of local wall and +1.5 to
  3.5 s on CI; N's emit +3.2 to 4.6 s of CPU on the release runtime, S0's
  +0.15 to 0.3 s at `opt-level = 1`. The margins under the lines (§2 item
  8): 8.0 s on Linux, 10.7 s on macOS, 13.2 s on Windows.
- **Projections have undershot:** 5.4 projected +1 to 4 s of local wall
  and measured +15 to 18 s (ADR 0057 §19, §20), about 4 to 5 times. A
  miss that size here would put Linux past 80 s.
- **The prototype on CI before the plan:** with the user's word for that
  push, asked again at that step, it goes to
  `claude/loving-archimedes-frtf5q-55-ci` with a draft pull request,
  measurement only, closed unmerged, as #142. Its `selfhost.rs` figures
  on all three systems come before the plan; past a line (80 s Linux, 100
  s macOS, 110 s Windows) the user is asked with them. Coverage is never
  cut without asking.
- **Also measured in the prototype:** N's peak RSS on its root (about 207
  MB today, projected +40 to 60 MB) and collections (`NOVA_GC_DEBUG`); N's
  least stack on the depth cases with `--ll`; the `.ll`'s size per set.

## 10. The cut, for the plan

One pull request, split into tasks, test first, as 5.4 (not asked):
(1) **S0's `--ll`:** the option and its refusal, `MirDumps`, the unit
test, §3.3's tests, `split_ll`, S0 printing the `.ll` in the after form
with N's side not yet asked for it; (2) **the cases:** `ir_tests.rs`'s
sources extracted, the float `%` and hand cases, the coverage test on
S0's side; (3) **N's emitter:** `RtFunc`'s symbols and signatures,
`emit_ll.nova`, `selfhost mir --ll`, the `.ll` compared on every input,
the report and its test; (4) **the measurements:** the depth cases'
stack, N's memory, the mutants, the prototype on CI (§9); (5) **the
records** (§13).

## 11. The rule (decision 8)

From 5.5 the `.ll` emitter exists twice, in `nova-codegen-llvm` and in
`selfhost/`. A change to either changes both in one pull request: the
emitter's text (header, declarations, instructions, names, escapes,
float spelling, globals, the C `main`), its errors' words, `RtFunc`'s
symbols, signatures or order in `RtFunc::ALL`, and the runtime's ABI as
the emitter spells it (symbols, classes, slot sizes, the layouts of sums,
records, arrays and closures, the env-first indirect call). The `.ll`
differential holds them equal (phase plan decision 6; ADR 0057 §21
extended). Recorded in ADR 0058's "The rule" and in the CHANGELOG. WASM's
and test mode's code generation stay S0's alone.

## 12. Crates and files

`nova-cli` (`src/cmd/run.rs`, `DumpMirCmd`'s `--ll`; `tests/selfhost.rs`;
`tests/selfhost/compare.rs`; `tests/selfhost/cases/mir/`); `nova-driver`
(`MirDumps`, the unit test); `selfhost/` (§5.1); `docs/adr/0058-*.md`;
§13's files. `nova-codegen-llvm` and `nova-mir` unchanged. No new crate,
no manifest change.

## 13. Records

- **ADR 0058,** the self-hosted `.ll` emitter and the `.ll` dump: the
  option and its refusals; the form (§4), an example of each line kind;
  what `compile_ir` is called with, and the equality with `build
  --release`; N's idioms; the errors, the float `%` case, the
  unreachable ones and `Mem`; the inputs and cases; the coverage test and
  `LL_UNREACHABLE`; the report; the figures (CI, N's memory and stack);
  5.5b's deferred questions; the rule (§11).
- **Dated notes:** `docs/phase-5-plan.md` §5.5 (the split, `--ll` for
  `--emit-ll`, 5.5a's gate, 5.5b's questions deferred);
  `nova-spec/14-CODEGEN.md` §2.2 (the emitter twice, the hidden dump);
  `nova-spec/00-MASTER-SPEC.md`'s Phase 5.
- **`CHANGELOG.md`:** Added (N's `.ll` emitter, the hidden `--ll`, the
  rule); nothing Changed.

## 14. Risks

Ranked.

1. **CI time:** +1.5 to 3.5 s projected against margins of 8.0, 10.7 and
   13.2 s, but 5.4's projection undershot about 4 to 5 times, and one
   commit's spread on Windows has been wider than an increment (ADR 0057
   §19). CI's figures come first; past a line, the user decides.
2. **N's memory on its own root:** an 11.84 MB `.ll` built in one process
   beside both modules, its printing bound by the collector. Measured; if
   too much, N prints the `.ll` as it makes it (§5.2).
3. **N's stack in the depth cases** with the emitter, on 1 MB: measured;
   5.3b's rule applies.
4. **Hex case and escaping:** N's helpers are lowercase `\xHH` and keep
   bytes past ASCII (§2 item 9); the `@test`s, the escape case and the
   mutants show a slip.
5. **`ret_of`'s duplicate key:** perhaps in no input, so the differential
   may never show it; N's `@test` pins it, and a missing callee's `Unit`.
6. **A `.ll` equal on both sides but not what `build --release` writes**
   (the spliced module against the full check's): 5.4's guard and §3.3's
   test.

## 15. Not in 5.5a

- All of 5.5b (§1), Windows linking among it.
- WASM and test-mode code generation (5.4 was native only); Cranelift.
- Optimizing the `.ll` or changing its text: no `source_filename`, triple
  or datalayout.
- Fixing float `%` in S0 (decision 4); a runtime change for N's speed,
  memory or stack, unless the user rules.

## 16. Gate

- On Linux, macOS and Windows, per host, S1's `.ll` (N's `selfhost mir
  --ll`) equals S0's (`nova check --dump-mir --ll`) byte for byte, with
  exit codes, on the 210 corpus roots that lower clean (N's own among
  them), the 407 cases, `ir_tests.rs`'s sources, the float `%` and hand
  cases, `cases/mir/` again with `\r\n`, the MIR fuzzer's 120 inputs and
  the 25 depth cases.
- The emitter's error agrees on the float `%` case, `(Ll error` with S0's
  words on both sides; the unreachable errors' words are pinned by S0's
  unit test and N's `@test`s.
- The coverage test passes: every form of §6.3 occurs in the compared
  `.ll`, or is in `LL_UNREACHABLE` with its reason.
- The dump's `.ll` equals `build --release`'s intermediate on §3.3's
  programs; `--dump-mir` without `--ll` prints 5.4's output; `--ll` is in
  no help or usage line; both sides refuse the same shapes.
- The `.ll` report names the `define` and the block; its test and the
  mutants pass.
- N runs the depth cases with `--ll` on a 1 MB stack, or the depth it
  survives is recorded; N's memory is recorded.
- CI is measured from the prototype, and `selfhost.rs` is within 80, 100
  and 110 s, or the user has ruled.
- The full suite, fmt, clippy, the MSRV job, `nova fmt --check` over
  `selfhost/`, and `git diff` on `Cargo.lock` are clean.
