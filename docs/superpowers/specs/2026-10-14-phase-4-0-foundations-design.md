# Phase 4.0, "Foundations and the first WASM program": design

- **Date:** 2026-10-14.
- **Status:** approved by the user on 2026-10-14.
- **The user's decisions, 2026-10-14:**
  - the WASM runtime takes route (a), from the spike (`docs/phase-4-plan.md`
    §3, decision 1);
  - the runtime is written in Nova, with memory builtins only std can call;
  - 4.0 covers every MIR statement, without a collector.
- **Branch:** `claude/loving-archimedes-frtf5q`.
- **Inputs:**
  - `docs/phase-4-plan.md` §3 (decisions 1, 2, 5, 8, 12 and 13) and §4's
    4.0 entry;
  - `nova-spec/14-CODEGEN.md` §2.3, §3, §4 and §8;
  - `nova-spec/30-FRONTEND.md` §7 and §10.

## 1. What 4.0 delivers

After 4.0, `nova build --target wasm` turns a program into a page and a
module that run in a browser or under Node. The program may use all of the
language and six std modules. Programs that need a collector, async or an
OS do not build yet.

1. **ADR 0038** records Phase 4's scope (the plan's decision 13). **ADR
   0039** records the WASM backend and its runtime (§10).
2. **A target.**
   - `nova build --target wasm`, with `nova bundle` as its alias, and
     `nova check --target wasm` (§3).
   - The WASM target compiles its own set of std modules: six of today's,
     and a new `std/rt` (§3.2).
3. **`nova-codegen-wasm`** lowers every MIR statement to one WASM module
   with `wasm-encoder` (§4). It joins the crates published at the next
   release (§8).
4. **`std/rt`**, the runtime written in Nova (§5). It holds the allocator,
   strings, bytes and checks, on nine memory builtins. In 4.0 the allocator
   only grows; the collector is 4.1's.
5. **`app.js`**, the shim. It loads the module in a browser or in Node, and
   provides what JS does best (§6): printing, `Float` text both ways, case
   mapping, and the hash seed.
6. **`dist/`** holds `index.html`, `app.js` and `app.wasm` (§7).
7. **Tests** run the WASM builds under Node, and CI runs them on all three
   operating systems (§9).

**The gate** (the plan's §4, as the user changed it on 2026-10-14):
- `examples/01-hello-world`, built with `--target wasm`, prints under Node
  what its native build prints, on all three operating systems.
- So does each `tests/runtime/` program §9.2 lists. The five listed
  panicking programs fail with the native build's message.
- Hello world's `app.wasm` and `app.js` together are under 30 KB gzipped,
  checked by a test.

## 2. The starting point (read on 2026-10-14)

- **The crates.**
  - `crates/nova-codegen-wasm` and `crates/nova-bundler` hold a one-line
    `src/lib.rs` each, and `publish = false`.
  - `nova build` takes `-o` and `--release` only
    (`crates/nova-cli/src/cmd/run.rs`).
  - Neither name is taken on crates.io: both returned 404 from the index on
    2026-10-14.
- **MIR** (`crates/nova-mir/src/lib.rs`).
  - Values are `I64`, `F64`, `I8`, `Ptr` or `Unit`. Every heap value is a
    `Ptr`.
  - Heap objects are 8-byte slots: a sum is `{tag, fields…}`, a record
    `{fields…}`, an array `{len, elems…}`, and a closure `{code, env}`.
  - A string is a pointer to a 16-byte header, `{len, ptr}`. The native
    backends make a literal's header by calling `nova_rt_str_new` each time
    the literal is evaluated.
  - The backends call `nova_rt_alloc` for every `Make*` statement.
  - Generics are monomorphized from `main` (`mono.rs`), so MIR holds only
    the functions `main` reaches.
- **Int arithmetic wraps.** `9223372036854775807 + 1` prints
  `-9223372036854775808` (checked with `nova` 0.3.2).
- **Integer division by zero traps.** Natively it exits with signal 4,
  status 132, and prints nothing. `panic` prints `nova: panic: <message>`
  and aborts, status 134.
- **`Float` `%`** is refused by both native backends: "float remainder (%)
  is not supported yet".
- **std** (`std/lib.rs`, `nova_resolver::STD_MODULES`). 16 modules are
  compiled into every program and glob-imported into every module. Their
  intrinsics are the resolver's `Builtin`s, and `Builtin::STD_ONLY` (81 of
  them) are visible only inside std.
- **Six modules stand alone.** With `STD_MODULES` cut to core, bytes,
  collections, strings, fmt and json, `nova`'s front end resolved and
  checked a program with no diagnostic. This was checked in a scratch
  worktree on 2026-10-14, with nothing kept.
  - Together they call 26 builtins. Besides `println` and `panic`, those
    are:
    - `char_to_int`, `int_hash_seed`;
    - `str_chars`, `str_cmp` and `str_hash`;
    - `str_from_chars`, `str_index_of`, `str_join` and `str_len_chars`;
    - `str_to_lower` and `str_to_upper`;
    - `float_fixed`, `json_quote` and `str_to_float`;
    - and ten `bytes_*`.
  - log needs io and time, and time needs task, so all three wait for 4.1.
- **The spike** (the plan's decision 1, in scratch space) lowered real MIR
  with `wasm-encoder` 0.243.0. Its module, validated by `wasmparser`
  0.243.0, printed under Node 22 what `nova run` prints.
  - JS formats `Float`s with the same shortest digits as Rust, except in
    exact ties: 261 of 2,005,144 doubles differed.
  - JS's `toFixed` rounds ties up where Rust rounds them to even:
    `0.125` to two places is `0.13` in JS and `0.12` in Rust, and `2.5`
    to none is `3` against `2`.
- **The crates on Rust 1.78.** `wasm-encoder = "=0.243.0"` in
  `nova-codegen-wasm`, and `wasmparser` 0.243.0 as its dev-dependency, were
  tried in a scratch worktree with the repository's `Cargo.lock`.
  - `wasmparser` was taken without default features, with `std`,
    `validate`, `features` and `simd`.
  - `cargo +1.78 check -p nova-codegen-wasm --tests` passed.
  - The lockfile grew by 29 lines: three new packages, `leb128fmt` 0.1.0,
    `wasm-encoder` and `wasmparser`. `wasmparser` reuses the lock's
    `bitflags` 2.13.1, `indexmap` and `semver`.
  - A fresh resolve on cargo 1.78 instead fails, on `indexmap` 2.14.2's
    `edition2024` manifest. The committed lock prevents that, as it does
    for the MSRV leg today.
- **The tools.**
  - **This machine:** Node 22.
  - **GitHub's hosted runners:** Node, on all three systems.
  - **CI's `test` job** sets up Rust only. Its `vscode` job uses
    `actions/setup-node@v4` with Node 22.

## 3. The target

### 3.1 Commands

- **`nova build --target <native|wasm>`.** `native`, today's build, is the
  default.
  - With `wasm` it writes `dist/` (§7).
  - `-o <dir>` names that directory.
  - `--release` with `--target wasm` is an error: "`--release` does not
    apply to `--target wasm`".
- **`nova bundle [FILE] [-o <dir>]`** is `nova build --target wasm`
  (`40-TOOLING.md` §1.1).
- **`nova check --target <native|wasm>`** checks against that target's std
  and rules.
- `run`, `test`, `doc`, `fmt` and the language server stay native.
- `nova_driver` gains a `Target` (`Native`, `Wasm`), which reaches the
  resolver (std, builtins, `extern`), MIR lowering (§5.4) and the backend.

### 3.2 std per target

- **Native** compiles today's 16 modules, unchanged.
- **WASM** compiles seven:
  - core, bytes, collections, strings, fmt and json, from today's;
  - **`$std.rt`**, `std/rt/lib.nova`, new (§5). It is embedded by
    `nova-std` like the others.
- **Each target's list** is a constant beside `STD_MODULES`. The driver
  registers each module's source as `<std/name>`, as today. The language
  server, `nova doc` and `nova test` keep the native list.
- **`$std.rt` is never in the native list.** Its functions are not `pub`,
  so the glob import puts none of them into a program's scope.
- **A name from a native-only module is unresolved on WASM.** For example,
  `read_to_string` is "cannot find function". The note that names the
  target and the module is 4.1's, with 4.1's final set (the plan's
  decision 5).

### 3.3 Rules per target

- **`extern "C"` on WASM is E0090:** "extern functions are not available
  on target wasm".
- **The memory builtins (§5.1)** are `Builtin::STD_ONLY`, and they are
  seeded only when the target is WASM. On native they do not exist.
- **A runtime function WASM lacks is E0091.** That is any `RtFunc` that is
  neither in `std/rt`, nor a host function, nor an intrinsic (§4.6). For
  example, `async fn main` needs `task_block_on`. The error names the
  function and the target: "this program needs `task_block_on`, which
  target wasm does not have". `nova check --target wasm` reports it too,
  because it runs MIR lowering, as `check` does today.

## 4. The backend (`nova-codegen-wasm`)

`compile_module(mir: &nova_mir::Module, opts) -> Result<Output>` returns the
module's bytes and its import list, from which the shim is written (§6).

### 4.1 Values and memory

- **Values map as:**
  - `I64` to `i64`;
  - `F64` to `f64`;
  - `I8` to `i32`, 0 or 1;
  - `Ptr` to `i32`;
  - `Unit` to no value.

  Each non-`Unit` temp is one WASM local.
- **Memory** is one 32-bit memory, 256 pages (16 MB) at first and at most
  16384 (1 GB) (`30-FRONTEND.md` §10). It is exported as `memory`.
- **Its layout:**
  - `0..8` stays unused, so address 0 is never an object;
  - `8..64` holds `std/rt`'s state words (§5.2);
  - from 64, the data segment: string literals (§4.5);
  - from the data's end, rounded up to 8, the heap, which `std/rt` grows.
- **Slots stay 8 bytes,** so MIR's offsets are unchanged.
  - `I64` and `F64` fill a slot.
  - `I8` is stored as one byte at the slot's start, as the Cranelift
    backend does.
  - A `Ptr` is stored zero-extended to 8 bytes and loaded as its low 4
    bytes. A slot read as a whole word, by `std/rt` or by 4.1's collector,
    then holds the address.

### 4.2 Control flow

- **MIR's blocks become structured control flow** by the algorithm of
  Ramsey's "Beyond Relooper" (2022), for reducible graphs: the dominator
  tree, reverse postorder, and `block`/`loop` nesting at merge points and
  loop headers.
- **Every graph MIR builds today is reducible,** because Nova has no
  `goto`.
- **An irreducible graph** falls back to a dispatch loop for that function:
  a `$pc` local, one `block` per MIR block, and a `br_table`. The spike
  used this form; a unit test keeps it working.
- **Terminators:**
  - `Switch` becomes a `br_table` when its arms are dense, and a chain of
    comparisons otherwise;
  - `Trap` becomes `unreachable`;
  - `Return` becomes `return`.

### 4.3 Calls and function values

- **A direct `Call`** is `call`. Arguments of type `Unit` pass nothing,
  and a call whose result goes nowhere drops it.
- **Function values.** Every function whose address MIR takes (by
  `MakeClosure`, or by `async_lower`'s futures) goes into one `funcref`
  table, through one element segment. Its code pointer is its table index.
- **`CallIndirect`** loads the index and the environment, then calls
  `call_indirect` with the type `(env i32, params…) -> ret`.

### 4.4 Heap objects

- `MakeSum`, `MakeRecord`, `MakeArray`, `MakeClosure` and `ArrayAlloc`
  allocate through `std/rt`'s `alloc` (§5.2). They then store at the same
  offsets the native backends use.
- `SumTag`, `SumField`, `RecordField`, `SetField`, `ArrayLen`, `ArrayGet`
  and `ArraySet` load and store at those offsets, with the widths of §4.1.

### 4.5 String literals

- **Each distinct literal** is emitted once into the data segment: its
  16-byte header `{len: i64, ptr: i32, 0: i32}`, then its bytes.
- **`ConstStr` is an `i32.const`** of the header's address. Nothing is
  allocated, where the native backends call `nova_rt_str_new` each time.
  The header lies outside the heap, which 4.1's collector skips.

### 4.6 Runtime calls

Each `RtFunc` MIR names is one of four kinds on WASM:

| Kind | `RtFunc`s | The backend emits |
|---|---|---|
| **Nova** | `Alloc`, `CheckBounds`, `StrConcat`, `StrConcatN`, `IntToStr`, `BoolToStr`, `CharToStr`, `StrEq`, `StrCmp`, `StrHash`, `StrLenChars`, `StrChars`, `StrFromChars`, `StrJoin`, `StrIndexOf`, `JsonQuote`, and the ten `Bytes*` | a `call` to `std/rt`'s function (§5.4) |
| **Host** | `Print`, `Println`, `EPrint`, `EPrintln`, `Panic`, `FloatToStr`, `FloatFixed`, `StrToFloat`, `StrToUpper`, `StrToLower`, `IntHashSeed` | a `call` to an import from `env` (§6) |
| **Intrinsic** | the nine memory builtins (§5.1) | the instructions inline |
| **Unavailable** | every other | E0091 (§3.3) |

`Alloc` is `nova_rt_alloc`, which the native backends also call directly
for `Make*`; on WASM it is `std/rt`'s `alloc`.

### 4.7 Imports and exports

- **Imports:** `env.<name>` for each host function the program uses, and
  nothing else.
- **Exports:**
  - `main`, the program's entry;
  - `memory`;
  - `nova_alloc`, `std/rt`'s allocator, which the shim calls to place the
    strings host functions return.
- **Validation.** Tests validate every module they build with
  `wasmparser`; `nova` itself does not depend on it.

## 5. The runtime (`std/rt`)

### 5.1 Memory builtins

Nine new `Builtin`s, std-only and WASM-only (§3.3). Each is lowered to
instructions in line (§4.6). Addresses and values are `Int`s.

| Builtin | Instructions |
|---|---|
| `mem_load8(addr: Int) -> Int` | `i32.load8_u` |
| `mem_store8(addr: Int, v: Int)` | `i32.store8` |
| `mem_load64(addr: Int) -> Int` | `i64.load` |
| `mem_store64(addr: Int, v: Int)` | `i64.store` |
| `mem_copy(dst: Int, src: Int, n: Int)` | `memory.copy` |
| `mem_fill(dst: Int, byte: Int, n: Int)` | `memory.fill` |
| `mem_pages() -> Int` | `memory.size` |
| `mem_grow(pages: Int) -> Int` | `memory.grow` |
| `heap_base() -> Int` | the heap's first address, a constant (§4.1) |

Each wraps its `Int` address to 32 bits.

### 5.2 Allocation

- **The heap pointer is a state word,** at address 8. `std/rt` has no
  mutable globals, since Nova has none, so its state lives at fixed
  addresses that `const`s name.
- **`alloc(size: Int) -> Int`:**
  1. takes `max(size, 8)`, rounded up to 8;
  2. bumps the heap pointer;
  3. grows memory by whole pages when the end passes `mem_pages()`;
  4. panics with "out of memory" when `mem_grow` returns -1.
- **New memory is zeroed,** and the bump never reuses it. So every object
  starts zeroed, as the native `gc::alloc` guarantees.
- **The pointer starts at `heap_base()`** the first time `alloc` runs, while
  its word is still 0.
- **Nothing is freed in 4.0.** A program's memory grows until it ends.

### 5.3 Its functions

- **Each is a port of `nova-runtime`'s function of the same name,** with
  the same results. For example:
  - `str_concat_n` reads the array `{len, elems…}`;
  - `str_index_of` searches bytes, then counts characters;
  - `str_hash` mixes with the program's seed, as `nova_rt_str_hash` does.
- **A runtime value is an `Int` address.** Each function takes and returns
  `Int`s, where `nova-runtime`'s take pointers.
- **Allocating strings.** `str_alloc(len)` gives a header, with the bytes
  inline at offset 16, as `alloc_str_object` does.
- **Panics.** `check_bounds` panics with `nova-runtime`'s message, "array
  index {i} out of bounds for length {len}", through `panic`.

### 5.4 Routing runtime calls to Nova

- **The table.** `nova_mir` gains `RuntimeImpls`, a table from each Nova
  `RtFunc` (§4.6) to a `DefId`. The driver fills it on WASM by looking up
  `std/rt`'s functions by name.
- **`lower_module_with(module, &Options { runtime: Some(&impls) })`** runs
  mono's usual worklist:
  - `Alloc`'s function is a root beside `main`;
  - after a function is lowered, each `CallRuntime` naming a Nova `RtFunc`
    becomes a `Call` to the function's mangled name, which joins the
    worklist.
- **The result:** the module holds only the runtime functions the program
  reaches, as it holds only the program's own.
- **The ABI at those calls.** MIR passes a `Ptr` where `std/rt`'s function
  takes an `Int`. The backend converts at each call:
  - an argument from `i32` with `i64.extend_i32_u`;
  - a result to `i32` with `i32.wrap_i64`.

  It does so wherever a call's argument temp is `Ptr` and the callee's
  parameter is `I64`, or the reverse for a result. Only these calls can
  differ.
- **`Module` also names the allocator's symbol,** which the backend calls
  for `Make*` (§4.4) and exports as `nova_alloc` (§4.7).
- **Native lowering is unchanged:** it passes no table.

## 6. The shim (`app.js`)

`app.js` is an ES module, generated per program, holding only the host
functions the module imports. It exports `default async function
init(options)`, which loads the module, instantiates it and calls `main`.

- **Loading.**
  - In a browser it fetches `app.wasm` beside itself, with
    `instantiateStreaming`.
  - Under Node it reads the file.
  - Run directly, as `node dist/app.js`, it calls `init()` itself.
- **Output.**
  - Under Node, `print` and `println` write to `process.stdout`, and the
    `eprint` pair to `process.stderr`.
  - In a browser they write whole lines to `console.log` and
    `console.error`, and the part of a line still unprinted when `main`
    returns.
- **Panics.**
  1. `panic(msg)` writes `nova: panic: <msg>` to stderr, then throws.
  2. `init` catches it.
  3. Under Node it sets `process.exitCode = 134`, the native abort's
     status, and returns, so that output already written drains.
- **Traps.**
  - A `WebAssembly.RuntimeError` (a division by zero, `unreachable`) writes
    `nova: trap: <its message>`, with exit status 132, the native trap's.
  - A stack overflow writes `nova: stack overflow`, with status 134.
- **Strings from JS** are UTF-8, placed by `nova_alloc(16 + len)` with the
  header written as §5.3 describes.
- **The host functions, each held to `nova-runtime`'s result (§9.3):**
  - **`float_to_str`** writes Rust's `Display` for `f64`:
    - shortest round-trip digits, and never an exponent;
    - `NaN`, `inf`, `-inf` and `-0`.

    It starts from `String(v)`. In an exact tie between two shortest
    candidates, it picks the one Rust picks. Ties are decided exactly, with
    `BigInt` arithmetic on the double's bits.
  - **`float_fixed(v, places)`** clamps `places` to `0..=17` and rounds the
    exact value half to even, with `BigInt`, as Rust's `{:.N}` does. It
    keeps `-0.0`, `NaN` and `inf`, and values of `1e21` and above.
  - **`str_to_float(s)`** accepts Rust's `f64` grammar exactly:
    - an optional sign;
    - digits with an optional point and exponent, with at least one digit
      before the exponent;
    - `inf`, `infinity` and `nan`, in any case;
    - no spaces, underscores or hex.

    A match converts with `Number`, which rounds correctly, as Rust does.
    Anything else is `0.0`, as `nova_rt_str_to_float`'s `unwrap_or(0.0)`.
  - **`str_to_upper` and `str_to_lower`** use `toUpperCase` and
    `toLowerCase`. Both apply full Unicode mappings, as Rust's `str`
    methods do. The two can differ only where the browser's Unicode version
    differs from Rust's.
  - **`int_hash_seed`** draws 64 random bits from `crypto.getRandomValues`,
    which browsers and Node 22 both provide.

## 7. Output

- **`dist/` holds three files:**
  - `index.html`: a page that loads `app.js` as a module and calls
    `init()`;
  - `app.js` (§6);
  - `app.wasm`.
- **Where it goes.** In a project, `dist/` is at the project's root,
  beside `nova.toml` (`30-FRONTEND.md` §7.1). For a loose file, it is in the
  current directory. `-o <dir>` overrides both.
- **Replacing it.** Each build replaces those three files and leaves
  anything else in the directory alone.
- **`nova build` prints `built <dir>`.**
- **Not yet:** `manifest.json`, `assets/` and a user's own `index.html` are
  4.3's, with `nova-bundler`.

## 8. Crates and the release set

- **`nova-codegen-wasm`:**
  - depends on `wasm-encoder = "=0.243.0"`;
  - takes `wasmparser = { version = "=0.243.0", default-features = false,
    features = ["std", "validate", "features", "simd"] }` as a
    dev-dependency only;
  - adds what §2 measured to `Cargo.lock`: 29 lines, three packages.
- **`nova-driver` depends on `nova-codegen-wasm`,** so it becomes a
  published crate, the 20th:
  - it loses `publish = false`, and gains `readme` and license files, as
    `nova-codegen-llvm` has;
  - it joins `[workspace.dependencies]` with its exact pin;
  - it joins `publish-crates.sh`'s list, after `nova-codegen-llvm`;
  - `package-gate.sh`'s count becomes 20, and so does
    `release_manifests.rs`'s;
  - the runbook names 20 crates.
- **`nova-bundler`** stays unpublished and empty until 4.3.

## 9. Testing

### 9.1 Node

- **The tests that run WASM call `node`.**
- **Without it**, a test prints that it was skipped and passes. Under
  `NOVA_REQUIRE_NODE=1`, it fails instead.
- **CI's `test` job:**
  - sets `NOVA_REQUIRE_NODE=1`;
  - installs Node 22 with `actions/setup-node@v4`, as the `vscode` job
    does, on all three systems.
- **Each test builds into a fixed directory** under the test target's
  temporary directory, as the user's rules ask.

### 9.2 The gate's programs

`crates/nova-cli/tests/wasm_tests.rs` builds each program with
`nova build --target wasm -o <dir>`, runs `node <dir>/app.js`, and compares
its stdout with the program's `.stdout` file.

**`examples/01-hello-world`.**

**From `tests/runtime/`, by behaviour:**

| Group | Programs |
|---|---|
| Control flow and constants | `break_continue`, `constants`, `for_loops` |
| Data | `array_repeat`, `arrays`, `collections`, `field_assign`, `map_keys`, `match_enum`, `records` |
| Functions, generics and traits | `assoc_fn`, `assoc_types`, `closures`, `generic_impls`, `generic_trait_methods`, `generics`, `iterator`, `method_generics`, `supertraits`, `trait_assoc_fn`, `traits`, `where_clauses` |
| std | `prelude_option_result`, `std_core`, `strings` |
| Bytes | `bytes_basics` |
| Formatting | `fmt_float_edge`, `fmt_float_fixed`, `fmt_int_pad`, `fmt_string_pad` |
| JSON | `json_depth_empty`, `json_depth_leaf`, `json_object_forged_map`, `json_parse_numbers`, `json_parse_strings`, `json_parse_values`, `json_render_deep`, `json_render_guard_under`, `json_round_trip`, `json_stringify`, `json_stringify_escapes` |

**Programs that must fail,** each with its native message on stderr and a
failing status, as `run_tests.rs` checks them natively:
- `panic_unwrap` prints `7`, then panics with "called get on None";
- `bytes_from_ints_too_high` and `bytes_from_ints_too_low` panic with
  "nova_rt_bytes_from_ints: element out of range 0..=255", which `std/rt`'s
  port keeps word for word;
- `json_render_guard` and `json_stringify_cycle` panic with "stringify:
  nesting too deep or cyclic value". Rendering keeps its pending work on
  the heap, not the stack (`std/json`), so neither needs a deep stack.

**Left out, each with its reason:**

| Program | Reason |
|---|---|
| `bytes_reserved` | It is a compile error by design |
| `extern_ffi` | It uses `extern "C"`, which is E0090 on WASM |
| `fs_io_types`, `http_*`, `system_time_iso8601`, `nova_test` | They need a module WASM lacks |

A listed program that fails only because it recurses deeper than a WASM
stack holds under Node's default moves to the left-out list (§11). The
ledger and the PR then record its measured depth.

### 9.3 Differential tests

The host functions are held to `nova-runtime`'s results by programs built
both ways, whose outputs must be equal byte for byte:
- **`Float` text:**
  - the spike's tie cases, such as `1658206780088562.25`;
  - quotients `i / 7.0` and products `i * 0.1` for `i` in `0..2000`;
  - powers of ten from `1e-320` to `1e308`, and their neighbours;
  - `0.0`, `-0.0`, `NaN`, `inf` and `-inf`.
- **`float_fixed`:**
  - exact ties at each precision, such as `0.125` to 2, and `2.5` and
    `0.5` to 0;
  - values from `1e20` to `1e22`;
  - every precision from 0 to 17.
- **`str_to_float`:** every case of §6's grammar, valid and not.
- **Case mapping:** ASCII; `ß`, `İ` and final `Σ`; and a line of mixed
  scripts.
- **Bytes and strings:** the `bytes_*` and `str_*` functions on empty,
  ASCII and multi-byte input.

The programs live in `tests/wasm/`. Each is checked natively against its
`.stdout` file, and on WASM against the same file.

### 9.4 Unit tests in `nova-codegen-wasm`

- Every statement and terminator, lowered and validated.
- The structuring of loops, nested loops, `break` and `continue`, and
  dense and sparse `Switch`es.
- The dispatch-loop fallback on a hand-built irreducible graph.
- The ABI conversions at runtime calls (§5.4).

### 9.5 The size check

The gate's test builds `examples/01-hello-world`. It asserts that
`app.wasm` and `app.js`, each compressed with `flate2` at the best level,
add up to under 30 KB. It also prints the figure, which the PR records.
`flate2` is in the lockfile already.

### 9.6 What stays as it is

All of today's tests, the MSRV leg on Rust 1.78, `cargo fmt`, `clippy`,
and `nova fmt --check std examples`, which now includes `std/rt`.

## 10. Records

- **ADR 0038, "Phase 4's scope".** The plan's decision 13, as approved:
  - what Phase 4 leaves out;
  - the departures the user decided on 2026-10-14: live reload, no
    `axum`, no `walrus`, a JSON file for the blog, the collector between
    turns, and component syntax as specified.
- **ADR 0039, "The WASM backend and its runtime".**
  - Route (a), with the spike's measurements.
  - The runtime in Nova, on memory builtins.
  - The host functions, and the exactness each is held to.
  - The value mapping and memory layout.
  - The structuring.
  - Node for the tests.
  - Each is recorded against `13-RUNTIME.md` §8 (which names
    `wasm-bindgen` and `web-sys`) and `14-CODEGEN.md` §8.
- **Dated notes:**
  - `14-CODEGEN.md` §2.3 and §8;
  - `13-RUNTIME.md` §8;
  - `30-FRONTEND.md` §7;
  - `40-TOOLING.md` §1.1;
  - the master spec's dependency list (no `walrus`; `wasm-encoder`
    pinned at 0.243.0; `wasmparser`).
- **The rest:**
  - `ARCHITECTURE.md` and `crates/README.md`, for `nova-codegen-wasm` and
    `std/rt`;
  - `CHANGELOG.md`'s `[Unreleased]`;
  - `docs/releasing.md`'s crate count;
  - `docs/phase-4-plan.md`'s 4.0 entry, amended for the gate the user
    changed.

## 11. Risks

1. **The structuring algorithm is the largest piece of new code.** The
   unit tests cover each shape. The fallback keeps a program working if a
   graph surprises it.
2. **The ABI conversion at runtime calls** is the one place where a
   temp's type and a callee's differ. It is limited to `Ptr` against `I64`,
   and unit-tested in both directions.
3. **Recursion depth.** V8's stack under Node's default, about 1 MB, holds
   fewer frames than a native main thread's (8 MB by default on Linux and
   macOS). Browsers are similar. A program that recurses that deep fails
   with `nova: stack overflow` on WASM; 4.1 records the depth a WASM build
   supports.
4. **The host functions' exactness.** The spike found one difference, and
   §2 another. §9.3's tests cover each host function, but case mapping
   rests on the browser's Unicode tables.
5. **No collector.** A program that allocates without end exhausts 1 GB.
   The gate's programs are small, and 4.1 adds the collector.

## 12. Not in 4.0

- The collector, the WASM std's other modules, async, and the full golden
  suite (4.1).
- `std/dom`, handles and the fake DOM (4.2).
- `nova-bundler`, assets, `manifest.json`, a user's `index.html`, and
  `wasm-opt` (4.3).
- The note that names a missing module's target (4.1).
- `nova run --target wasm`, and the language server on WASM's std.

## 13. Decisions made while writing this spec

The user's decisions did not settle these, and each is open to review.

1. `nova check --target wasm` is in 4.0 (§3.1), since it costs only the
   driver's plumbing.
2. `--release` with `--target wasm` is an error, not ignored (§3.1).
3. The codes E0090 and E0091 (§3.3); E0089 is the highest in use today.
4. String literals' headers are static, in the data segment, so a literal
   allocates nothing (§4.5). The native backends allocate one per
   evaluation.
5. Memory starts at 16 MB, as `30-FRONTEND.md` §10 says. A smaller start
   would cost the same, since pages are reserved, not written.
6. A `Ptr` is stored zero-extended in its 8-byte slot (§4.1), so whole-word
   reads see the address.
7. The ABI conversion is done by the backend at each call (§5.4), not by
   new casting builtins, so `std/rt` is written with `Int` addresses only.
8. The exit statuses under Node mirror the native ones: 134 for a panic
   and 132 for a trap (§6).
9. `nova-codegen-wasm` is published from the next release, and
   `nova-bundler` stays unpublished until 4.3 (§8).
10. `wasmparser` is a dev-dependency only (§8): tests validate modules,
    and `nova` does not.
11. `dist/` is at a project's root, as `30-FRONTEND.md` §7.1 shows, not
    under `target/` (§7).
12. Node 22 is installed in CI's `test` job (§9.1), rather than relying on
    each runner's own version.
