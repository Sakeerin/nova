# ADR 0058 — The self-hosted `.ll` emitter, and the `.ll` dump

## Status

Accepted (2026-11-18). Branch `claude/loving-archimedes-frtf5q-55`
(`docs/superpowers/specs/2026-11-18-phase-5-5a-the-ll-emitter-design.md`),
written as 5.5a's prototype was built. CI's figures (§13) are from the
prototype's run on all three systems, each under its line (spec §9).

Part 1 (S0's side) built what §1 to §3, §6 and §8 record:
- §1, the hidden `--ll` and its refusals;
- §2, the `.ll` in the dump, with one example of every line kind;
- §3, what `compile_ir` is called with, and the equality with `build
  --release`;
- §6, the inputs and the 17 new cases;
- §8, the coverage test and `LL_UNREACHABLE`.

Part 2 (N's side) built what §4, §5, §7, §9 and §10 record:
- §4, N's port and its idioms;
- §5, the emitter's errors;
- §7, the harness, which compares the `.ll` on every input;
- §9, the `.ll` mismatch report and its test;
- §10, the mutants.

Part 3 measured N's least stack on the depth cases with `--ll` (§12)
and wrote the records: §11 to §16 and the Consequences, with the
figures of parts 1 and 2 (§11).

## Decision

### 1. The option, `nova check --dump-mir --ll FILE…`

ADR 0053's mechanism, as ADR 0057 §1 (spec §3.1, the user's decision
2). `DumpMirCmd` (`nova-cli/src/cmd/run.rs`) gains `--ll`, taken once,
anywhere after `--dump-mir`, before or after the files, and before
`--`. Each root's after part is printed as in 5.4, followed, for a root
that lowered, by its `.ll` (§2).

The refusals, as built (each nothing on stdout, exit 2):
- **`--ll` twice** is clap's error under the form's usage line:
  `error: the argument '--ll' cannot be used multiple times`, then
  `Usage: nova check --dump-mir [OPTIONS] [FILES]...`.
- **`--ll=x`** is clap's error under a usage line that names the flag
  used, as the base's `--before-async=1` already does: `error:
  unexpected value 'x' for '--ll' found; no more were expected`, then
  `Usage: nova check --dump-mir --ll [FILES]...`. The spec wrote
  `[OPTIONS] [FILES]...` for every shape (§15).
- **`--ll` with `--before-async`,** in either order, with or without a
  file: one line, `error: --dump-mir --ll takes no --before-async`,
  checked before the missing file's `error: --dump-mir needs a FILE`.
  The emitter reads the after form alone; the before form holds an
  `Await` in every async program (§5).
- **`nova check --ll`, `nova check --dump-ll`,** `--ll` before
  `--dump-mir`, after a plain `check`'s file, or inside `--dump-mir=…`:
  not the form, so the base's refusal stands, byte for byte (for `nova
  check --ll FILE`: `error: unexpected argument '--ll' found`, clap's
  tip, `Usage: nova check [OPTIONS] [FILE]`).

After `--`, `--ll` is a root's name (`-- --ll` prints `== --ll` and
`IOERR NotFound`). Without `--ll` the dump is 5.4's, byte for byte
(§11's `mircmp`).
`--ll` is in no `--help` and no usage line of the base's shapes.

N's side is `selfhost mir --std DIR [--before-async | --ll] FILE…`:
`--ll` once; twice, as `--ll=…`, or with `--before-async`, N prints its
usage line, `usage: selfhost mir --std DIR [--before-async | --ll]
FILE...`, on stderr, nothing on stdout, and exits 2.

**Checked by:**
- `the_mir_dump_flag_is_hidden_and_refused_as_the_spec_says`
  (`selfhost.rs`), extended: `check --help` does not name `--ll`; the
  five base shapes above give the base's error; eleven `--ll` shapes of
  the form's own parser are one error each, exit 2, nothing on stdout:
  `--ll` twice, twice around a file, `--ll=x`, `--ll=`, with `--std`,
  with `--help`, with `--before-async` in three orders and with it but
  no file (these four the line above), and alone with no file; `--ll`
  before `--` takes a root that starts with `-`.
- `both_sides_refuse_the_mir_dumps_shapes_alike` gains `--ll` twice,
  `--ll=x` and `--ll` with `--before-async` in both orders: exit 2 and
  nothing on stdout on both sides, and N's exact usage line.
- `usagecmp.sh` (§11): 558 base shapes under six `argv[0]`s, 10 in a
  terminal, and the hidden forms' refusals, the `--ll` shapes added.

### 2. The `.ll` in the dump

The form (spec §4). A root that lowered (`lowered=true`) is followed,
after its `(Diagnostics)`, by one of:
- **`(Ll ok)`, then `nova_codegen_llvm::compile_ir`'s text, byte for
  byte,** ending in its own line break. There is no closing line: the
  `.ll` runs to the next `== ` header or the end. No line of the `.ll`
  starts with `(` or `== `, so 5.4's readers stop where they stopped.
- **One line `(Ll error "…")`:** the emitter's own words, quoted as the
  dumps quote a string (`nova_resolver::dump::quote`, 5.1's), without
  `build --release`'s context line. The dump's exit code stays 0, as for
  a part with MIR's diagnostics.

A root that did not lower (`runs=false`, `checked=false`,
`lowered=false`, `IOERR`) prints no `(Ll` line, and `compile_ir` is
never called on it: `(Ll` occurs exactly when `lowered=true`. The
before form takes no `.ll`.

**One example of every line kind,** S0's `nova check --dump-mir --ll`
at `8cb1b42b`, shown in part; every line below is S0's, byte for byte,
and `…` stands for lines left out. The root is
`cases/mir/ir_tests__recursion_and_branch_lower.nova`, `ir_tests.rs`'s
`fn fib(n: Int) -> Int { if n <= 1 { n } else { fib(n - 1) + fib(n -
2) } }` and `fn main() { println("${fib(5)}") }`:

```
== crates/nova-cli/tests/selfhost/cases/mir/ir_tests__recursion_and_branch_lower.nova
(Mir runs=true checked=true lowered=true async_lowered=true functions=2 externs=0
…
(Diagnostics)
(Ll ok)
; LLVM IR emitted by the Nova compiler (release backend).
; Requires LLVM >= 15 (opaque pointers).

declare void @llvm.trap()
declare void @nova_rt_println(ptr)
…
declare void @nova_rt_router_state_store(ptr)
declare ptr @nova_rt_str_new(ptr, i64)

define void @"nova_main"() {
entry:
  %t0.slot = alloca i64
…
define i64 @"fib.0"(i64 %p0) {
entry:
  %t0.slot = alloca i64
…
  store i64 %p0, ptr %t0.slot
  br label %bb0
bb0:
  store i64 1, ptr %t2.slot
  %r0 = load i64, ptr %t0.slot
  %r1 = load i64, ptr %t2.slot
  %r2 = icmp sle i64 %r0, %r1
  %r3 = zext i1 %r2 to i8
  store i8 %r3, ptr %t3.slot
  %r4 = load i8, ptr %t3.slot
  %r5 = icmp ne i8 %r4, 0
  br i1 %r5, label %bb1, label %bb3
bb1:
…
bb2:
  %r7 = load i64, ptr %t1.slot
  ret i64 %r7
…
  br label %bb2
}


define i32 @main(i32 %argc, ptr %argv) {
entry:
  call void @nova_main()
  ret i32 0
}
```

In order: the two comment lines and a blank line; `llvm.trap`'s
declaration; the 111 runtime functions' declarations in `RtFunc::ALL`'s
order; `nova_rt_str_new`'s, which `ConstStr` calls and no `RtFunc`
names; each extern's; a blank line; each function, `main` emitted as
`nova_main`, each followed by a blank line, its temps' `alloca`s and its
parameters' stores under `entry:`, then its MIR blocks as `bbN:`, every
instruction indented two spaces, `%rN` restarting at 0 in each function;
the string globals, then a blank line (two blank lines in a row where
there is no string, as here); the C entry wrapper. The kinds this root
does not show, from other inputs at the same commit:
- an extern's declaration, `tests/runtime/extern_ffi.nova`:
  `declare i64 @"llabs"(i64)`, `declare double @"pow"(double, double)`;
- string globals, `cases/mir/ll_a_string_of_every_escape_class.nova`,
  every escape class: `@str0 = private unnamed_addr constant [8 x i8]
  c"say \22hi\22", align 1`, `c"a\5Cb"`, `c"tab\09here\00nul\0D\0A"`,
  `c"caf\C3\A9 \E2\94\80"`, the empty string `[1 x i8] c"\00"` and a
  raw line break in a literal, `c"first\0Asecond"`;
- a float, `cases/mir/ll_a_float_le_is_an_ordered_compare.nova`: `store
  double 0x3FF8000000000000, ptr %t2.slot`, `%r4 = fcmp ole double %r2,
  %r3`;
- a switch on an `I8` and the trap,
  `cases/mir/ir_tests__bool_match_switches_on_i8_not_i64.nova`: `switch
  i8 %r0, label %bb4 [ i8 1, label %bb2 i8 0, label %bb3 ]`, then `call
  void @llvm.trap()` and `unreachable`;
- the error line, `cases/mir/ll_a_float_remainder_errs.nova`: `(Ll
  error "float remainder (%) is not supported by the LLVM backend")`.

**Why this form** (not asked; the spec's recommendation): the part
still opens with `(Mir` and 5.4's grammar still ends at
`(Diagnostics)`; the harness cuts each `(Ll` part off before 5.4's
checks (§7) and the report's walk enters the `.ll` at `(Ll ok)` (§9).
Quoting each line into 5.4's grammar would change the bytes, so the
`.ll` would no longer be the intermediate `build --release` writes, and
would cost N quoting about 62 MB.

### 3. What `compile_ir` is called with, and `build --release`

`MirDumps::new(before_async, ll)` and `MirDumps::unshared(before_async,
ll)` (`nova-driver/src/lib.rs`) take whether to print the `.ll`;
`Wanted` gains `ll`, asked for only with the after form. In
`HirDumps::dumps`, when the after form lowered (`Some(Ok(module))`), the
driver appends `ll_part(module)`: `(Ll ok)` and `compile_ir(module)`'s
text, or `(Ll error ` and the error's `to_string()` quoted. The printing
lives in the driver: `nova-mir` cannot depend on the emitter, and the
emitter's crate does not depend on `nova-resolver`, so another place
would change a manifest and `Cargo.lock`. S0's emitter and MIR do not
change.

**The module is the one `build --release` emits** (spec §2 item 4): the
after form's, std's shared functions spliced in, `keep_the_entry_main`
and `mir_options(None)` natively; `build_program_release` takes
`lower_to_mir`'s, from the full check. 5.4's unit test and guard hold
the two modules equal (ADR 0057 §5), and `compile_ir` is a function of
the module alone. **The driver's guard is not extended** (not asked):
`Wanted::ALL` keeps `ll: false`, so the Test job's slowest binary (ADR
0057 §19) does no more work.

**Checked by:**
- `the_ll_dump_is_what_build_release_writes` (`selfhost.rs`, S0 alone,
  temporary directory `nova-selfhost-ll-build-release`): one `check
  --dump-mir --ll` of `tests/runtime/generics.nova`,
  `examples/05-geometry/src/main.nova` (a package with a path
  dependency, given as FILE), `tests/runtime/async_tasks.nova`,
  `tests/runtime/extern_ffi.nova`, `cases/mir/mir_every_native_form.nova`
  and the float `%` case; then, for each, `nova build --release FILE -o
  outN` with `NOVA_CLANG` and `NOVA_LLC` a file that is not there and a
  fresh `$NOVA_HOME`. Each exits 1. The five that emit leave
  `outN.nova.ll`, equal byte for byte to the root's `.ll` in the dump,
  and say `release build failed; generated LLVM IR left at `; the float
  `%` case leaves none, and its stderr ends with the words its `(Ll
  error` line quotes. `examples/01-hello-world`, whose `src/main.nova`
  spec §4's example shows, has no `nova.toml`, so it is not a package
  (§15).
- Two driver unit tests, `mod ll_dumps` (temporary directory
  `nova-driver-ll-dump`):
  - `the_ll_part_is_the_emitters_text_or_its_words_quoted`: `(Ll ok)`
    and the text for a module of one empty `main`; and `(Ll error "…")`
    with each error's words for modules built by hand: an `Await`, a
    float `Rem`, a float `BitAnd`, and a module whose one function is
    not `main`.
  - `the_ll_dump_is_the_module_build_release_emits`: on six programs (a
    hello with an escaped quote, an async program, an extern block with
    a closure, a float `%`, no `main`, a type error), the dump with
    `--ll` is the dump without it, then `(Ll ok)` and `compile_ir` of
    `lower_to_mir`'s module, the function `build_program_release`
    calls; the float `%` gives the error both ways; the two that do not
    lower get no `.ll`; the before form is the same with `ll` or without.
- The header test: the after form with `--ll`, its `(Ll` parts cut off,
  equals plain `--dump-mir` on its roots.

### 4. N's port

N's emitter is a port of `nova-codegen-llvm/src/lib.rs` (761 lines,
633 of code) and of `RtFunc::symbol` and `signature`
(`nova-mir/src/lib.rs`), function by function and with S0's names (spec
§5.1):

| N | lines (code) | ports |
|---|---:|---|
| `emit_ll.nova` (new) | 690 (562) | `compile_ir`, `rt_func_decl`, `llty`, `escape_bytes`, `float_hex`, `FnEmit` and its helpers, `stmt`, `arg_list`, `emit_call`, `binop`, `terminator` |
| `mir.nova` | +154, −3 | `RT_FUNC_SIGNATURES` (111 entries), `rt_func_symbol`, `rt_func_signature`, `read_signature` |
| `main.nova` | +63, −26 | `selfhost mir --ll`, `ll_part` beside `mir_of` |
| `emit_ll_tests.nova` (new) | 200 (163) | N's six `@test`s |

Code lines exclude blank lines and lines that start with `//` after
their indentation. N's `selfhost/src` is then 55 files and 31,399
lines, and `nova test` passes 108 (102 before).

**Idioms,** beyond ADR 0057 §10's:
- **Text:** one `StringBuilder`, parts pushed and joined once (S0 builds
  the functions' text, then the header). The header reads nothing the
  functions make, so N writes it first into the same builder; the bytes
  are the same. A root's `.ll` is built whole, as S0 builds it (§5.2 of
  the spec, not asked), and printed with its part.
- **`escape_bytes` onto the builder:** printable ASCII but `"` and `\`
  pushed in runs, every other byte `\XX` with two uppercase hex digits
  and no `x`, by bytes (`byte_at`), never by characters. dump.nova's
  `push_escaped`, lowercase `\xHH` that keeps bytes past ASCII, is not
  reused.
- **The empty string's `\00`** is a case at the global, `[1 x i8]
  c"\00"`; every other length is the byte length.
- **`float_hex`:** `0x` and the 16 uppercase hex digits of
  `Float.to_bits`, by an uppercase helper (`hex_upper`), S0's
  `0x{:016X}`.
- **`RtTable`, read once:** each runtime function's declaration,
  symbol and return class, read from `RT_FUNC_SIGNATURES` once per
  module, since each read of a Nova `const` builds the array anew.
- **The signature table's strings:** each entry is the symbol and a
  string `"(Ptr, I64) -> Ptr"`, the classes by their variants' names,
  read by `read_signature`; generated from S0's `.ll` header, and every
  `.ll` compared checks it (§7).
- **`ret_of`'s order:** a `Map` read by lookup alone, the functions
  inserted, then the externs, so an extern's class replaces a
  function's of the same symbol, as S0's `collect` over the two keeps
  the last; a callee it lacks is `Unit`. `string_index` is a `Map` read
  by lookup alone, the strings in a `Vec` by first use. No map's order
  reaches the text.
- **Codes:** a class is its `MirTy` code, `llty` a table of them; a
  runtime function its index. Hard-coded as in S0: 8-byte slots, `i64`
  sizes, the heap layouts of sums, records, arrays and closures, the
  env-first indirect call, `main` as `nova_main` and the C wrapper.

**N's `@test`s** (`emit_ll_tests.nova`):
`escape_bytes_writes_each_byte_class_as_s0_does`;
`float_hex_is_the_bits_in_16_uppercase_digits` (0, −0.0, 1.5, a NaN from
`"NaN".parse_float()`, 1e300, 2.5e-308, −inf);
`a_symbol_both_a_function_and_an_extern_returns_the_externs_class` (and
a missing callee's `Unit`);
`every_runtime_function_has_a_distinct_symbol_and_its_signatures_declaration`
(S0's `rt_func_tests`, and six declarations exactly);
`a_small_module_is_the_header_its_function_and_the_c_main`;
`each_error_of_the_emitter_is_in_s0s_words` (§5). They also run on the
debug runtime in the Selfhost job's last step; locally they pass on
it.

**The command:** `selfhost mir` takes `--ll` (§1); `mir_of` appends
`ll_part` to a `lowered=true` part, from the module it lowered: `(Ll
ok)` and the text, or `(Ll error "…")` quoted with `push_escaped`, as
S0's `quote` quotes it.

### 5. The emitter's errors (the user's decision 4)

N's `compile_ir` returns `Result<String, String>`, each `bail!` ported
with S0's words:
- **float `%`** (`Bin` `Rem` on `Float`): reachable, since it
  type-checks. `build --release` prints "Error: internal LLVM IR
  generation error (this is a compiler bug)", "Caused by:" and the
  words, exit 1; the dump prints `(Ll error "float remainder (%) is not
  supported by the LLVM backend")` on both sides, exit 0. Its case is
  `cases/mir/ll_a_float_remainder_errs.nova`, the only input that gives
  an error (§8). S0's bug is not fixed (§14).
- **float bitwise** ("bitwise operators are not defined for Float"),
  **no `main`** ("no `main` function in MIR module") and **`Await`**
  ("an `.await` marker reached codegen: `nova-mir`'s async
  state-machine transform must split every `async fn` body at its await
  points before a module is emitted"): ported as written, **unreachable
  natively**, as ADR 0057 §7 records E0011. `&`, `|`, `^`, `<<` and `&=`
  on `Float`, and `a & b` in a generic function, are E0013 before MIR,
  and `~` on a `Float` is E0010; lowering makes no other float `Bin`. No
  `main` is E0601 first. An `Await` is in the before form alone, which
  takes no `.ll`. Each is pinned by S0's unit test (§3) and by N's
  `each_error_of_the_emitter_is_in_s0s_words`, on modules built by hand,
  with the same words (float `&` and `<<` for the bitwise one).
- **`Mem`** ("a WASM memory access (…"): **not ported.** N's MIR has no
  `Mem` statement (5.4 left WASM's MIR out, ADR 0057 §6), so the arm has
  nothing to match.

### 6. The inputs and the cases

`MirForm::After` is S0's `check --dump-mir --ll` (`MIR_LL_S0`) and N's
`mir --std DIR --ll`, so the `.ll` is compared wherever 5.4 compares the
after form, through `compare_batch`, byte for byte with exit codes, per
host, nothing normalized (spec §6.1, the user's decision 3):
1. the native corpus's roots that run: 251 loose and 10 packages'
   `src/main.nova`; 210 lower and print a `.ll`, N's
   `selfhost/src/main.nova` among them;
2. the HIR cases that reach MIR (342; 341 print a `.ll`) and
   `cases/mir/` (103 roots; 82 print an `(Ll` part, 81 a `.ll` and the
   float `%` case its error);
3. the MIR fuzzer's 120 inputs and the 25 depth cases (ADR 0057 §16,
   §17), unchanged; N runs the depth cases with `--ll` on a 1 MB main
   thread (§12);
4. the cases again with `\r\n`, as 5.4, the `.ll` included.

The before form stays 5.4's, without `--ll`. Root lists pass
`HIR_ARGS_LIMIT` in parts, as 5.4.

**The 17 new cases,** in `cases/mir/` (not asked: `cases/mir/` is
already an after-form input with both line endings, so no new list or
test; their MIR is compared too):
- **14 `ir_tests__<test>[_n].nova`,** the sources
  `nova-codegen-llvm/tests/ir_tests.rs` feeds through `ir_for`,
  extracted as 5.4 extracted `lower_tests.rs`'s: a temporary
  instrumentation, a script outside the repository and never committed,
  recorded every source; every `format!` is expanded; `_n` numbers a
  test's sources when it has more than one. 20 sources were recorded;
  **6 were dropped as duplicates:** both hello worlds (each the text of
  `cases/check/check__hello_world_checks.nova`), `float_bits`
  (`tests/runtime/float_bits.nova`, already corpus), and the await-free
  async function, the field assignment and the repeat array (each the
  text of a `lower_tests__` case). The spec expected about 18 (§15).
- **3 by hand:**
  - `ll_a_float_remainder_errs` (§5);
  - `ll_a_float_le_is_an_ordered_compare`, `<=` on two `Float`s both
    ways: `Le` on `Float` was the one (operator, class) pair no input
    reached (spec §2 item 7);
  - `ll_a_string_of_every_escape_class`: `"`, `\`, a tab, a NUL, CR LF,
    UTF-8 of two and three bytes, an empty string and a raw line break
    in a literal (§2's globals). The `\r\n` test then compares
    `c"first\0D\0Asecond"` against the checkout's `c"first\0Asecond"`.

`Switch` on an `I8` comes from
`ir_tests__bool_match_switches_on_i8_not_i64`, so no hand case was
needed for it. All 17 lower; only the float `%` case's `.ll` is an
error.

### 7. The harness

Part 1 ran S0's after form with `--ll` while N's did not: a switch,
`N_LL = false`, had the comparison read S0's output without its `(Ll`
parts. Part 2 removed the switch: `mir_in(MirForm::After)` always
passes `--ll`, and `compare_batch` compares raw bytes and exit codes.

- **`split_ll`** cuts each root's `(Ll` part off S0's output after the
  comparison, so every 5.4 check reads 5.4's text: well formed, a header
  and a `(Mir` line per root, `KINDS`, the two forms equal where no
  `$poll` occurs, `MIR_CODES`.
- **`check_ll`,** in every after-form run: `(Ll ok)` or `(Ll error`
  exactly when `lowered=true`; `(Ll error` on
  `FLOAT_REMAINDER_CASE` alone, with its words; each `.ll` starts with
  the header's first comment and ends in a line break.
- `agree_on_mir_forms` returns the after form, the before form and the
  `.ll` parts; the corpus test and the coverage test share one run of
  the package mains (`package_mir`, a `OnceLock`).
- **The depth test,** `n_lowers_the_depth_cases_on_a_1_mb_main_thread`,
  runs N with `--ll` under `ulimit -s 1024` (on Windows, the main thread
  as it is, 1 MB), and its check that `bodies-for-ranges-127.nova` fails
  on 96 KB also passes `--ll`.
- `settled_stdout` touches no `.ll` line: none starts `  (Error`.

### 8. The coverage test (the user's decision 6)

`every_form_the_emitter_handles_occurs_in_the_compared_ll`, on the
native corpus's roots that run and the cases, sharing their run with
`the_mir_dumps_agree_on_the_corpus_and_the_cases`, reads S0's output of
each root that printed an `(Ll` part: its MIR after form, which
determines its `.ll`, and the `.ll` itself. **190 forms:**
- 25 statements (`LL_STATEMENTS`: every native statement but `Await`)
  and 5 terminators, by the MIR dump's kinds;
- the 28 (operator, class) pairs the type checker admits in a `Bin`
  (`LL_BINS`): 16 on `Int`, 10 on `Float`, `Eq` and `Ne` on `Bool`;
- 16 others: `(ConstStr "")`, `Neg` on `Int` and on `Float`, `Switch`
  on `I8` and on `I64` by the discriminant's class, `Return` with a
  value and without, a `Unit` temp, a function that takes an env, a
  `Call` to an extern; in the `.ll`, a string global with `\22`, with
  `\5C`, with a control byte and with a byte past ASCII, `c"\00"`, and
  an extern's `declare`;
- 111 `call @SYMBOL`, one per runtime function, read from each `.ll`'s
  header, which the test asserts the same in every `.ll`
  (`nova_rt_str_new` is `ConstStr`'s, which no `RtFunc` names, and is
  not among them);
- the 5 errors (`LL_ERRORS`), each `(Ll error NAME)`; an error of no
  list fails the test.

**Result** (2026-11-18): 633 roots, 1 of them an error; **183 of 190
forms found**, 111 runtime functions declared. Seen in fewer than three
roots: `Eq` and `Le` on `Float` (2 each), `BitNot` (1), the float `%`
error (1), `Switch` on `I8` (1), and 13 runtime functions' calls (1 or
2 each): four of crypto, four of files, `fs_create_dir`, `fs_rename`,
stderr's write and flush, and stdout's flush.

**`LL_UNREACHABLE`,** the 7 forms that may be absent, each with its
reason; the test fails if one occurs, if it names no form, or if any
form found is in no list:
- `call @nova_rt_test_selector`: only test mode's synthesized `main`
  calls it, and test mode's code generation is not compared;
- `call @nova_rt_task_is_done`: no native std function calls it, and a
  program's call is E0001;
- `call @nova_rt_task_take_output`: no MIR statement emits it;
- `(Ll error no main)`: lowering gives E0601 first;
- `(Ll error Await)`: an `Await` is in the before form alone, which
  takes no `.ll`;
- `(Ll error Mem)`: a memory access is in a WASM build's MIR alone;
- `(Ll error float bitwise)`: E0013 and E0010 before MIR, and lowering
  makes no other float `Bin` (§5).

All 111 declarations are in every `.ll`'s header, so every symbol and
signature is compared on every input whatever the calls: a slip in the
signature of `TaskTakeOutput`, which nothing calls, failed every one of
the 777 `.ll`s (§10).

### 9. The `.ll` mismatch report (the user's decision 5)

`compare.rs`'s `MirWalk`, which names a MIR dump's function, block and
statement (ADR 0057 §11), enters a root's `.ll` at a depth-0 line `(Ll
ok)` and counts its lines from 1: a `define` line enters a function,
named by its symbol as quoted (`"g.1"`), or `main` for the C wrapper; a
label (`entry:`, `bbN:`) inside a function enters a block; `}` leaves
the function. After the root and the line of its dump, the report names
`line 27 of its .ll, define "g.1", block bb1`, a `define` line's own
function with no block, `line 7 of its .ll, outside any define` for the
header, a declaration, a string global or a blank line between them, or
`at its (Ll line` at `(Ll ok)` or `(Ll error`. Both lines and the
reproduce commands (`nova check --dump-mir --ll FILE`, `selfhost mir
--std DIR --ll FILE`) follow, as for every dump. The walk starts afresh
at each root; every other report is as it was.

`the_ll_mismatch_report_names_the_define_and_block` pins it on dumps
written by hand: two roots, the first with two functions (`main` and
`g.1`), their `.ll` with a string global and the C wrapper, the second
an error. One full report (line 27 of the `.ll`, `define "g.1", block
bb1`) is compared whole, then 17 places by the first line they name: the
header, a declaration and the blank line after them; the first `define`
line, `entry:`, a slot, a line of `bb0` and the line before `}`; the
second `define`, a line of its `bb0` and its label `bb1`; a string
global; the C wrapper (`define main, block entry`); the `(Ll` line and
the MIR line before it; the second root's `(Ll` line and its MIR; and,
apart, N leaving the `.ll` out, reported at S0's `(Ll` line.

### 10. The mutants

Each was applied to `selfhost/src` in the working tree, N built from a
copy, and run through N's `@test`s, a script comparing every input of
§6 but `\r\n` (851 roots, 777 of them with a `.ll`) and `SH`'s
`the_mir_dumps_agree_on_the_corpus_and_the_cases`; then restored, never
committed. **All 12 were killed:**

| Mutant | Killed by | Roots that differ | The report's first place |
|---|---|---:|---|
| `float_hex` lowercase | differential, `float_hex_is_the_bits_in_16_uppercase_digits`, `each_error_of_the_emitter_is_in_s0s_words` | 124 | `05-geometry`, `define "PI.3", block bb0` |
| an escape's digits lowercase | differential, `escape_bytes_writes_each_byte_class_as_s0_does` | 79 | `07-fullstack-blog`, line 36949 of its `.ll`, outside any define (`@str43`, `c"\0c"`) |
| an `x` in each escape | differential, the same `@test` | 96 | `06-counter-spa`, outside any define (`\x22`) |
| `ret_of` with the function winning | **the `@test` alone** (`a_symbol_both_a_function_and_an_extern_returns_the_externs_class`) | 0 | none |
| `%rN` not restarted per function | differential | 534 | `05-geometry`, `define "total_area.13", block bb0` |
| strings numbered by last use | differential | 238 | `06-counter-spa`, `define "build.536", block bb5` (the `getelementptr`) |
| `TaskTakeOutput`'s return a `Ptr` | differential | 777 | `05-geometry`, line 37 of its `.ll`, outside any define |
| a global's length in characters | differential | 31 | N's root, line 389957 of its `.ll`, outside any define |
| the empty string without its NUL | differential | 145 | `06-counter-spa`, outside any define (`[0 x i8] c""`) |
| a record's size without `max(1)` | differential | 27 | `03-http-server`, `define "timeout.311$S3LiR93E$poll", block bb8` |
| a switch always on `i64` | differential, one case | 1 | `ir_tests__bool_match_switches_on_i8_not_i64`, `define "classify.0", block bb0` |
| float `Le` unordered (`fcmp ule`) | differential, one case | 1 | `ll_a_float_le_is_an_ordered_compare`, `define "nova_main", block bb0` |

**`ret_of` with the function winning cannot be killed by any input,
natively:** a function's symbol and an extern's coincide only for
`main`, which E0900 reserves against an extern of that name, or E0002
(a duplicate definition) in the same module as `fn main`, and every
other function's symbol holds a `.` (`fib.0`), which no extern's can.
N's `@test` pins it, as spec §14 risk 5 expected; spec §6.4's statement
that every mutant fails the differential does not hold for it (§15). The
`I8` switch and the ordered float compare are each held by one case,
the case that §6 added for them.

### 11. The figures of parts 1 and 2

Measured locally on 4 cores, 2026-11-18: part 1's in its session, on
its tree (S0, the harness and the cases those of its head `f3b32068`,
whose last commit changed a driver unit test alone), part 2's at
`8cb1b42b`; the `.ll`'s size by set and N on the debug runtime again
on the tree this ADR is merged with (the merged tree), whose code is
that of `36f74a9a` (it differs from it only in this ADR and two doc
comments moved in `crates/nova-driver/src/lib.rs`), and whose
`selfhost/` is as at `8cb1b42b`. Scripts and logs are outside the
repository, not committed.

**Agreement,** S0's `check --dump-mir --ll` against N's `mir --ll`,
byte for byte with exit codes, per root (part 2):

| Set | Roots | Print an `(Ll` part | Agree | Differ |
|---|---:|---:|---:|---:|
| native corpus's roots that run (N's own included) | 261 | 210 | 261 | 0 |
| HIR cases that reach MIR | 342 | 341 | 342 | 0 |
| `cases/mir/` | 103 | 82 | 103 | 0 |
| the MIR fuzzer's inputs | 120 | 120 | 120 | 0 |
| the depth cases | 25 | 25 | 25 | 0 |

The cases again with `\r\n` agree too (`SH`'s test). No frozen list.
On a 1 MB main thread (release N, `ulimit -s 1024`), all 851 agree, N's
root included.

**The `.ll`'s size,** S0, per set, on the merged tree, from the
`--nocapture` lines of the coverage test, the fuzzer's test and the
depth test. The
corpus's and the cases' rows are the coverage test's count, each root's
`(Ll ok)` line with its `.ll`; the fuzzer's and the depth cases' are
their tests' count, each root's `== FILE` header too:

| Set | Roots | Lines | Bytes |
|---|---:|---:|---:|
| loose corpus | 202 | 748,170 | 23,225,549 |
| packages' mains but N's | 7 | 128,620 | 3,787,437 |
| N's `selfhost/src/main.nova` | 1 | 390,750 | 12,252,987 |
| HIR cases | 341 | 71,478 | 2,469,445 |
| `cases/mir/` | 82 | 65,547 | 2,056,287 |
| the MIR fuzzer | 120 | 462,319 | 14,555,389 |
| the depth cases | 25 | 120,531 | 3,617,081 |
| all | 778 | 1,987,415 | 61,964,175 |

Part 1 measured the same rows at `f3b32068`, before N held its own
emitter: N's root then gave 379,439 lines and 11,842,977 bytes, and all
1,976,104 and 61,554,165; every other row was as above. The spec
projected about 62 MB (the fuzzer's about 14, the depth cases' about
4.5).

**S0's after form without and with `--ll`,** one process per set,
median wall time in seconds (part 1; the sets as the script took them:
`cases/mir/`'s 98 files, its package cases left out; 9 package mains):

| Set | opt-level 0 | opt-level 1 |
|---|---|---|
| loose corpus (251) | 4.64 → 4.92 | 1.77 → 1.88 |
| packages' mains but N's (9) | 0.31 → 0.35 | 0.14 → 0.17 |
| N's root | 1.13 → 1.25 | 0.53 → 0.59 |
| HIR cases (341) | 5.55 → 5.71 | 2.41 → 2.54 |
| `cases/mir/` (98) | 1.64 → 1.70 | 0.75 → 0.80 |
| the fuzzer (120) | 2.10 → 2.30 | 0.90 → 1.02 |
| the depth cases (25) | 2.30 → 2.35 | 0.97 → 0.96 |
| all | 17.67 → 18.60 (+0.92) | 7.47 → 7.95 (+0.48) |

**N on the release runtime, without and with `--ll`** (part 2), one
process per set: median wall of three, peak RSS the largest `VmHWM` of
the three, polled; collections and the largest live heap from one
`NOVA_GC_DEBUG` run. The loose and package sets are split by file name
here (245 and 15), not as the harness splits them (251 and 10):

| Set | Wall | Peak RSS | Collections | Largest live heap | stdout |
|---|---|---|---|---|---|
| N's root | 1.93 → 2.35 s | **211 → 303 MB** | 23 → 24 | 101 → 102 MB | 5.49 → 17.74 MB |
| loose corpus (245) | 3.98 → 4.60 s | 41 → 39 MB | 102 → 120 | 11 → 12 MB | 8.74 → 29.77 MB |
| package mains (15) | 0.52 → 0.70 s | 44 → 48 MB | 21 → 25 | 13 → 15 MB | 2.71 → 8.69 MB |
| HIR cases (341) | 4.89 → 4.99 s | 39 → 38 MB | 115 → 120 | 11 → 11 MB | 0.48 → 2.94 MB |
| `cases/mir/` (103) | 1.45 → 1.50 s | 48 → 49 MB | 44 → 45 | 14 → 15 MB | 0.76 → 2.82 MB |
| the fuzzer (120) | 2.10 → 2.56 s | 43 → 41 MB | 60 → 76 | 13 → 12 MB | 5.86 → 20.41 MB |
| the depth cases (25) | 1.23 → 1.34 s | 40 → 44 MB | 42 → 45 | 12 → 13 MB | 1.89 → 5.50 MB |

**N's root rises by 92 MB of RSS** (211 to 303 MB), against the spec's
projected +40 to 60 MB on about 207 MB: the 12.25 MB `.ll` of N's root
with its own emitter (12,252,987 bytes; stdout 5.49 to 17.74 MB) is
built whole, as parts and then one joined string, and printed beside
both modules. No threshold applies; if CI's memory asks for it,
printing the `.ll` as it is made (spec §5.2), the same bytes, is the
fallback.

**N on the debug runtime, without and with `--ll`,** on the merged
tree, by the same method (the loose corpus as the harness lists it, 251 files):

| Set | Wall | Peak RSS | Collections | Largest live heap | stdout |
|---|---|---|---|---|---|
| N's root | 5.82 → 7.66 s | **212 → 320 MB** | 23 → 24 | 102 → 148 MB | 5.49 → 17.74 MB |
| loose corpus (251) | 13.66 → 16.45 s | 42 → 44 MB | 103 → 122 | 12 → 14 MB | 9.71 → 32.94 MB |
| the fuzzer (120) | 7.20 → 8.82 s | 44 → 40 MB | 60 → 73 | 12 → 12 MB | 5.86 → 20.41 MB |
| the depth cases (25) | 4.41 → 4.80 s | 41 → 48 MB | 42 → 45 | 12 → 15 MB | 1.89 → 5.50 MB |

On the debug runtime N's root rises by 108 MB of RSS, and its largest
live heap by 46 MB; the release runtime does not show that jump in the
live heap (101 to 102 MB, above). The same run on the release runtime on
the merged tree gave N's root's row as above: 211 → 303 MB, 23 → 24
collections, 101 → 102 MB.

**`selfhost.rs`,** each comparison interleaved in one session (sessions
on this machine differ by up to a factor of two, so only figures of one
session compare):

| Step | Style | Before | After |
|---|---|---|---|
| part 1 (S0 prints the `.ll`, N not yet), against the base's harness with the new cases set aside | plain | 88.05 s | 91.95, 92.69 s |
| part 1 | as CI runs it | 54.04, 54.23, 55.22 s | 53.75, 53.26, 53.84 s |
| part 2 (N prints and the `.ll` is compared), against part 1's head `f3b32068` | plain | 215.82, 212.35, 216.69 s | 212.14, 210.80, 202.34 s |
| part 2 | as CI runs it | 50.74, 51.19, 53.50 s | 52.43, 52.57, 54.39 s |
| part 2, the eight MIR tests alone | as CI runs it | 19.36, 18.46 s | 20.16, 19.49 s |

"As CI runs it" is `NOVA_EMBED_RUNTIME=1
CARGO_PROFILE_DEV_OPT_LEVEL=1`. Part 1 adds nothing measurable as CI
runs it, and about 4 s plain against the one interleaved run before;
part 2 adds about +1.4 s to the median as CI runs it, about +0.9 s of it
in the MIR tests, and nothing measurable plain, where the HIR dump's
package parts are the longest processes. The tests: 64 at the base, 66
after part 1 (the coverage test and the `build --release` test), 67
after part 2 (the report's test). The spec projected +1 to 2 s of local
wall.

**Verification:**
- the full suite (`cargo test --workspace`, Node required): 2,837
  passed, 1 failed (the IPv6 test, as on the base), 10 ignored, against
  the base's 2,835, 1 and 10; the two new tests are the driver's;
- `SH` passes 67; N's `nova test` 108 passed, and N's tests on the debug
  runtime pass;
- `cargo fmt --all -- --check`, both clippy commands as CI runs them,
  `nova fmt --check std examples selfhost` and Rust 1.78's `cargo check
  --locked --workspace` with `-D warnings` are clean; `Cargo.lock` and
  the manifests are unchanged;
- **byte for byte with the base's `nova`** (`db640e97`), by part 1's
  scripts after part 2: `nova check` 2,045 checks; `nova parse --check`
  and `--ast` 3,876 runs over 1,938 files; `nova test` and `nova build`
  in each of the 11 packages outside `crates/` (whose packages are test
  fixtures, not compared), 22 runs; every `--help`, 22 of 22, no hidden
  flag shown; usage errors 558 runs under six `argv[0]`s and 10 in a
  terminal; `--dump-ast` as `--dump-tokens` 25 of 25; every refusal of
  the hidden forms (114 of `--dump-resolution`'s, 522 of `--dump-hir`'s
  and `--dump-mir`'s) one error, exit 2, nothing on stdout; and
  `--dump-mir` without `--ll`, in both forms, on 1,292 roots in 26
  batches (part 1's script, run after part 2). On the merged tree, by
  the plan's script, `--dump-mir` without `--ll` in both forms on 1,955
  roots (the 1,938 `.nova` files the base tracks and the 17 new cases)
  in 40 batches: 0 differ. None differs.

### 12. N's least stack on the depth cases, with `--ll` (part 3)

The least main-thread stack on which N's `selfhost mir --std DIR [--ll]
FILE` exits 0 and prints S0's part (`check --dump-mir [--ll] FILE`), on
each of the 25 depth cases (ADR 0057 §17), by ADR 0057 §17's method:
Linux, `ulimit -s`, 4 KB bisection, each run's output compared with
S0's, 2026-11-18, at `8cb1b42b`. N was built from that tree by S0's
`nova build` twice: against the debug runtime beside S0, and against
the release runtime CI's S0 embeds (`NOVA_RUNTIME_LIB` set to its
extracted copy); both binaries are byte for byte part 2's. Two rounds,
each form of each runtime in turn; a cell gives the two rounds, in KB.
The last column is ADR 0057 §17's, without `--ll`, 2026-11-14:

| Input | Levels | release, without | release, `--ll` | debug, without | debug, `--ll` | ADR 0057 §17, release / debug |
|---|---:|---:|---:|---:|---:|---:|
| body `annotations` | 128 | 64, 60 | 56, 64 | 64, 60 | 60, 60 | 56 / 60 |
| body `arrays` | 128 | 200, 196 | 200, 200 | 200, 200 | 204, 204 | 196 / 196 |
| body `closure-captures` | 64 | 200, 192 | 196, 196 | 196, 204 | 200, 204 | 196 / 196 |
| body `closure-params` | 64 | 200, 196 | 200, 196 | 208, 200 | 208, 204 | 200 / 204 |
| body `closures` | 64 | 196, 200 | 196, 200 | 200, 200 | 200, 200 | 200 / 200 |
| body `for-arrays` | 127 | 208, 208 | 208, 208 | 208, 208 | 212, 216 | 212 / 212 |
| body `for-patterns` | 127 | 212, 212 | 212, 212 | 212, 212 | 216, 212 | 204 / 216 |
| body `for-ranges` | 127 | 208, 208 | 208, 212 | 212, 216 | 212, 208 | 212 / 212 |
| body `indexes` | 128 | 200, 196 | 196, 200 | 204, 204 | 204, 204 | 200 / 200 |
| body `interpolations` | 128 | 200, 196 | 196, 200 | 200, 196 | 196, 200 | 200 / 196 |
| body `let-patterns` | 128 | 192, 196 | 192, 200 | 196, 200 | 196, 200 | 196 / 200 |
| body `match-arms` | 128 | 208, 208 | 204, 208 | 216, 208 | 208, 212 | 204 / 208 |
| body `match-scrutinees` | 128 | 196, 200 | 196, 200 | 200, 204 | 200, 200 | 200 / 204 |
| body `method-chains` | 128 | 48, 52 | 48, 48 | 56, 56 | 56, 56 | 48 / 56 |
| body `or-patterns` | 128 | 52, 52 | 56, 56 | 56, 56 | 60, 56 | 56 / 56 |
| body `questions` | 128 | 196, 200 | 200, 196 | 200, 200 | 200, 200 | 200 / 200 |
| body `tuple-patterns` | 127 | 196, 200 | 200, 192 | 200, 200 | 204, 200 | 196 / 200 |
| body `tuples` | 128 | 196, 196 | 200, 196 | 200, 200 | 200, 200 | 200 / 196 |
| body `variant-patterns` | 127 | 196, 200 | 196, 196 | 200, 196 | 200, 196 | 196 / 196 |
| type argument, `arrays` | 128 | 44, 52 | 52, 48 | 52, 48 | 52, 56 | 48 / 52 |
| type argument, `fn-parameters` | 128 | 52, 52 | 52, 48 | 56, 56 | 56, 56 | 52 / 52 |
| type argument, `fn-returns` | 128 | 52, 52 | 52, 52 | 48, 48 | 56, 48 | 48 / 56 |
| type argument, `generics` | 128 | 60, 60 | 64, 56 | 64, 60 | 64, 60 | 64 / 60 |
| type argument, `options` | 128 | 56, 60 | 60, 60 | 64, 60 | 56, 60 | 60 / 64 |
| type argument, `tuples` | 128 | 48, 52 | 52, 52 | 52, 52 | 52, 56 | 52 / 48 |

**The emitter adds nothing measurable to N's stack.** With `--ll` the
least stack is 48 to 212 KB on the release runtime and 48 to 216 KB on
the debug one; without it, 44 to 212 KB and 48 to 216 KB. On every
input, with `--ll` and without differ by at most 8 KB (two bisection
steps), as two rounds of one form do, and today's figures without
`--ll` are within 8 KB of ADR 0057 §17's. The emitter does not recurse
on nesting, and the deep types' rows, whose symbols are 262 to 520
characters, need no more with it. **No case needs more than the 1 MB
the harness gives:** the most is 216 KB, so N has about 4.7 times the
room it needs, and 5.3b's rule (record the depth N survives as
(differs)) is not needed. The test's 96 KB check (§7) still fails
`bodies-for-ranges-127.nova`, so the stack is the one set.

### 13. CI's figures

The prototype at `8cb1b42b` went to
`claude/loving-archimedes-frtf5q-55-ci` with the user's word, as a
draft pull request (#144), measurement only, closed unmerged, as #142.
Its run, 44259975902 (the emitter and harness alone), failed the three
Selfhost jobs (`ubuntu-24.04`, `macos-latest`, `windows-latest`) at
their `cargo test --test selfhost` step, every other job green, on one
test: `the_ll_dump_is_what_build_release_writes` read the float `%`
case's stderr, which under CI's `RUST_BACKTRACE=1` ends in `anyhow`'s
backtrace, not in the error's words. The test now runs `nova build
--release` with `RUST_BACKTRACE=0` and `RUST_LIB_BACKTRACE=0`;
reproduced and fixed locally with `RUST_BACKTRACE=1` set. That fix,
`36f74a9a`, with the records' commit `3c6f1a07` before it, was then
pushed to the same branch, under the same draft pull request; its run,
44260509656, passed all 19 jobs.

`selfhost.rs`'s main step, `finished in`, against the lines (ADR 0057
§19) and two runs of `main` at 5.4's merge:

| System | Lines | 5.5a (run 44260509656) | `main`, run 43853141585 | 5.4's pull request, run 43850851663 |
|---|---:|---:|---:|---:|
| Linux | 80 s | 51.39 s (67 tests) | 77.03 s (64) | 71.98 s |
| macOS | 100 s | 29.94 s (65) | 31.72 s (62) | 89.34 s |
| Windows | 110 s | 93.86 s (61) | 79.94 s (58) | 96.77 s |

All three are under their lines. The runners' spread between runs of
the same tree is larger than 5.5a's cost: `main`'s own tree measured
71.98 and 77.03 s on Linux and 31.72 and 89.34 s on macOS, so one run
cannot show 5.5a's increment on CI. Locally, as CI runs it, the
increment is about +1.4 s (section 11). The Test job's main step summed
2838 passed, 0 failed, 10 ignored on Linux, 2838/0/9 on macOS and
2815/0/9 on Windows, two more than 5.4's (the driver's new unit tests).
The advisory `--ignored` step failed on Linux (`extern_ffi_run`) and on
macOS (three GC scan tests), as that step is allowed to (it runs with
`continue-on-error`); no `.ll` test is among them. N's memory on its
root on CI's runners is not reported by the job.

### 14. 5.5b's questions, deferred

5.5 splits (the user's decision 1): 5.5b is the driver, `clang` and
`cc`, finding the runtime, N's `build` and `check`, and the golden
suite built by S1. Its questions are asked when 5.5b starts, not
decided here (spec §1, research §11 items 3 to 8,
`docs/phase-5-plan.md` §5.5's "Questions for its spec"):
1. where the S1 golden suite runs (the Selfhost job's Ubuntu leg with
   `clang-18`, a job of its own, or the Test job), which runtime its
   programs link, and whether S1 is built by Cranelift or `--release`;
2. the intermediate file's name, the tolerance: one `-o` basename for
   every stage, a constant name, or a `source_filename` line in both
   emitters;
3. N's command line and exit codes, how N finds std and the runtime,
   project mode without `current_dir`, and whether stderr is compared;
4. `check`'s output: codespan's layout ported, or codes and spans in
   the dumps' form (ADR 0048 decision 1);
5. N's `build` without `--release` (N has no Cranelift): `-O2` or
   `-O0`;
6. platforms: Windows `link.exe` discovery is not portable; N's `build`
   on Linux and macOS only, or an OS query in std, in both compilers.

Also left out of 5.5a (spec §15): WASM's and test mode's code
generation, and Cranelift; optimizing the `.ll` or changing its text
(no `source_filename`, triple or datalayout); fixing float `%` in S0
(the user's decision 4); a runtime change for N's speed, memory or
stack, unless the user rules.

### 15. Departures from the spec, and decisions it does not settle

**From the spec,** each recorded where it falls:
- **`ir_tests.rs` gives 14 cases, not about 18** (§6): 20 sources, of
  which both hellos, `float_bits` and three `lower_tests__` duplicates
  were dropped.
- **clap's usage line for `--ll=x` names the flags used**,
  `Usage: nova check --dump-mir --ll [FILES]...`, as the base's
  `--before-async=1` does; spec §3.1 gives `[OPTIONS] [FILES]...`,
  which `--ll` twice prints (§1).
- **§3.3's package is `examples/05-geometry`**, a package with a path
  dependency: `examples/01-hello-world` has no `nova.toml` (§3).
- **N's root rises by 92 MB of RSS** (211 to 303 MB), not the
  projected +40 to 60 MB (§11). Measured, recorded; the fallback stays
  unbuilt (spec §5.2).
- **The `ret_of` mutant is killed by N's `@test` alone** (§10): spec
  §6.4 expects each mutant to fail the differential, which no native
  input can for this one; spec §14 risk 5 foresaw it.
- **The spec's 407 cases that lower** counted `cases/mir/` by the
  research's list (66 of 92 roots). By the harness's lists, the base's
  86 roots of `cases/mir/` give 65, so 406 with the HIR cases' 341; with
  the 17 new, 423 (§6).

**Decisions the spec does not settle,** each with its cost if wrong
(all cheap):
- Part 1's switch turned on when S0's arguments held `--ll` and N's did
  not, rather than reading `N_LL`; part 2 removed both.
- `check_ll` runs whatever the switch said, so S0's `.ll` was checked
  in part 1 already.
- 5.4's refusal shapes keep plain `MIR_S0` on S0's side.
- `nova_rt_str_new` is not among the coverage's calls (§8).
- A root whose `.ll` is an error adds only its error's form to the
  coverage; each error is named through `LL_ERRORS`, an unknown one
  failing the test.
- The coverage test is a `#[test]` of its own, sharing its run with the
  corpus test through `OnceLock`s.
- N writes the header first into the one builder (§4); `RtTable` is
  read once per module; `escape_bytes` writes onto the builder and the
  empty string's `\00` is a case at the global; the signature table
  holds strings; `ll_part` lives in `main.nova` beside `mir_of`.
- The `.ll` is built whole on both sides; nothing yet asks for printing
  it as it is made.

### 16. The rule

From 5.5 the `.ll` emitter exists twice, in `nova-codegen-llvm` and in
`selfhost/`. A change to either changes both in one pull request: the
emitter's text (the header, the declarations, the instructions, the
names, the escapes, the float spelling, the globals, the C `main`), its
errors' words, `RtFunc`'s symbols, signatures or order in `RtFunc::ALL`,
and the runtime's ABI as the emitter spells it (symbols, classes, slot
sizes, the layouts of sums, records, arrays and closures, the env-first
indirect call). The `.ll` differential (§6, §8, §12) holds them equal
(phase plan decision 6; ADR 0057 §21 extended, which already names a
runtime function's name or place in `RtFunc::ALL`). The coverage
test's lists (`LL_STATEMENTS`, `LL_BINS`, `LL_ERRORS`,
`LL_UNREACHABLE`) change with any new statement, operator or error, and
with a runtime function no input calls. WASM's and test mode's code
generation stay S0's alone.

## Consequences

- N's front end now reaches textual LLVM IR, the same bytes as S0's on
  every input the differential holds, with no `clang`, on Linux, macOS
  and Windows (§13). 5.5b links it.
- The MIR dump's after form prints about 62 MB more over every input;
  locally, as CI runs it, `selfhost.rs` grew about 1.4 s (§11); on CI
  it stayed under each line, where one run cannot show the increment
  within the runners' spread (§13).
- N's root peaks at about 303 MB of RSS with its `.ll`, 92 MB more than
  without (§11).
- N lowers and emits the depth cases on 1 MB with about 4.7 times the
  room it needs (§12). ADR 0055 §12's normalized type is still the one
  input recorded as (differs) for N's stack.
- Float `%` still type-checks and fails in `build --release`; its case
  shows both sides agree on the error (§5).
