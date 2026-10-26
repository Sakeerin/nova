# Phase 5 Plan — Self-hosting

> Status: **a draft for approval** (2026-10-26). Derived from
> `nova-spec/00-MASTER-SPEC.md` §3 (Phase 5), §1 ("Self-hosting target")
> and §7, `nova-spec/14-CODEGEN.md` §2.2 and its note of 2026-10-06,
> `nova-spec/13-RUNTIME.md` §6, and `docs/adr/0001-native-aot.md`. The
> user's decisions of 2026-10-26 are in §1. Each sub-phase below gets its
> own spec, plan and review before it is built.

## 1. Goal (from the spec)

Phase 5 = "Self-hosting". The master spec's goal is "Compiler written in
Nova", and its gate reads: "CI builds stage2 == stage1 byte-for-byte (or
close — may differ in non-deterministic codegen, document tolerances)."

The master spec lists six items for Phase 5:

1. port the lexer to Nova;
2. port the parser to Nova;
3. port AST, HIR and MIR to Nova;
4. port the type checker to Nova;
5. backend bindings: "bind LLVM via `unsafe` extern from Nova";
6. bootstrap: stage0 (Rust) → stage1 (Nova compiled by stage0) → stage2
   (Nova compiled by stage1), and "assert stage1 == stage2".

No other spec file plans self-hosting: `nova-spec/README.md` gives it 24
weeks, and ADR 0001 says only that it "requires bootstrapping". The master
spec's §7 closes the phase with the tag `v0.5.0`.

This plan names the stages as follows:
- **S0** is the Rust `nova`;
- **N** is the Nova compiler's source, in `selfhost/`;
- **S1 = S0(N)**, a native binary whose machine code came from the Rust
  compiler's emitter;
- **S2 = S1(N)**, whose machine code came from the Nova emitter;
- **S3 = S2(N)**.

**The user's decisions, 2026-10-26:**
- **Scope: the pipeline, and `build` and `check`.** The lexer, parser,
  resolver, HIR, type checker, MIR, the textual LLVM IR emitter, the
  driver, and the commands `build` and `check`. The language server,
  `nova fmt`, `nova doc`, the package manager and the WASM backend stay
  Rust (decision 1).
- **The backend: textual `.ll` and `clang`,** as the Rust `--release`
  already does. The port is about 633 lines, and std gains a way to start
  a process. An ADR records the departure from "bind LLVM via `unsafe`
  extern" (decision 2).
- **The gate: S1 == S2, and an IR comparison, on Linux.**
  - S0 and S1 emit identical `.ll` on a fixed corpus.
  - S1 == S2 byte for byte, and S2 == S3.
  - The same pinned `clang` builds every stage.
  - The tolerances are recorded in an ADR (decision 3).
- **Where it lives, and what ships: `selfhost/` in this repository, and
  `v0.5.0` ships the Rust `nova`.** N is a Nova package. There is no new
  Rust crate, and `clang` is needed in CI only (decision 4).
- **5.0, the minimum:** `?`, a string builder, `Char`, `Int` and `String`
  conversions, array helpers, an insertion-ordered map, process spawn and
  environment variables, and runs of the golden suite through the LLVM
  backend in CI (decision 5).
- **Two compilers: compared in CI.** Each sub-phase compares tokens, the
  AST, MIR or the IR with the Rust compiler's on the corpus. The language
  may still grow, and a change is made in both compilers (decision 6).
- **CI: the bootstrap on every pull request.** If it is too slow, the user
  is asked before it moves to a nightly job (decision 7).
- **A spike first:** the lexer, ported to today's Nova in scratch space,
  with each workaround recorded and its speed measured against S0, before
  this plan (§2 item 9).
- **5.0's additions, after the spike: all of them.** Tuples, nested
  patterns, `+=` on fields, `parse_int` and `parse_float`, and a fix or a
  lint for the silent newline continuation (a line that starts with `-`
  or `(`). 5.0 may split into 5.0a and 5.0b (§4).
- **Speed: measured, and fixed when needed.** S1 and S2 link an optimized
  runtime. The parser's speed is measured in 5.1. Interned string
  literals and unboxed `Option` and `Result` are built only if the
  bootstrap is too slow, and only on the user's word (decision 8).
- **The sub-phases: the lexer joins the parser in 5.1.** 5.0 language and
  std; 5.1 lexer, parser and AST; 5.2 resolver and HIR; 5.3 the type
  checker, which may split; 5.4 MIR; 5.5 the `.ll` emitter and the driver;
  5.6 the bootstrap gate, the boundary ADR and `v0.5.0`. Each has a
  differential check and a fuzz check against the Rust compiler.

## 2. Reality check — what Phase 5 needs that the repo lacks

Measured on `main` at `cc829f5` (`v0.4.0`'s records), 2026-10-26, with
`nova` 0.4.0.

1. **What would be ported is about 20,000 lines of code.**
   - Rust code lines, without tests or comments:
     - lexer 904, parser 2,630, AST 453, HIR 732;
     - resolver 1,953, with the builtin table and `STD_MODULES`;
     - type checker 7,404;
     - MIR 2,835 (`async_lower.rs`, `mono.rs`, `lower.rs`);
     - driver 1,794 (`program.rs`, `link.rs`, `runtime_cache.rs`);
     - the LLVM emitter 633, and diagnostics 631.
   - That is about 28,000 lines with comments. **The type checker is 37%
     of it**; `check.rs` alone is 17,694 lines, with 394 tests inline.
   - Out of scope (decision 1): LSP 3,888, fmt 4,059, doc 4,817, pm 2,045,
     index 1,746, the CLI, and the Cranelift and WASM backends.
2. **Every program type-checks all of std.**
   - std is 12,978 lines of Nova in 21 modules (`STD_MODULES`), each
     glob-imported into every program, so hello world checks async fns,
     traits with associated types, `std/http` and `std/ui`.
   - So N needs the whole language at once, and so does S1 to build N.
     A subset compiler is not possible unless std loads on demand.
   - std's sources are embedded in S0 through the `nova-std` crate. N
     cannot embed files, so it reads them from disk or from a generated
     module (5.2's question).
3. **The language lacks what a compiler is written with.** Each of these
   fails today, in a probe or in the spike:
   - `?` parses, then fails with E0900, "not supported yet". The parser
     alone would use it in about 164 places.
   - Tuples fail with E0900: about 130 sites in the type checker, 59 in
     the resolver, 49 in MIR.
   - Nested patterns (`Ok(Some(t))`) fail with E0900: up to 115 sites in
     the parser and 30 in the type checker.
   - `self.f += x` fails with E0900; the spike rewrote 23.
   - `String + String` fails with E0013, and there is no string builder.
     Appending by interpolation is quadratic: 20,000 appends took 493 ms.
   - `char_to_int` and `str_from_chars` are std-only, and there are no
     `as` casts. User code goes through UTF-8 bytes.
   - String indexing decodes the whole string on each call, so a lexer
     works on `Bytes.to_ints()` and `Bytes.slice`.
   - `[T]` has no `.iter()` or `.slice()`, `for x in arr` fails with
     E0900, and a `Vec` cannot become a `[T]`.
   - Parsing a float or an `Int` from a string is std-only.
   - **A line that starts with `-` or `(` continues the line before it.**
     `let z = x`, then `-y` on the next line, silently means `x - y`.
     Ported code will hit this.
   - Smaller gaps: `while x is P` and `[String; 32]` fail with P0001;
     `while true` is not known to diverge; a local variant name silently
     shadows a std one; there are no `mut` record fields, trait objects
     or mutable globals; closures capture by value; file I/O is async
     only, so N's `main` is an `async fn`.
   - What works: payload sums, recursive sums without boxing, `match` with
     exhaustiveness, traits, generics, closures, `mut self` methods over
     `Vec` fields, string `match`, flat modules, `for x in vec.iter()`,
     bit operations, and recursion 100,000 deep.
4. **A Nova program cannot start a process.** `std/process` is `args` and
   `exit` only (74 lines), and there are no environment variables. So N
   cannot run `clang` or the linker.
5. **There is no pointer FFI.**
   - `extern "C"` takes only `Int`, `Float` and `Bool`, with no generics
     or async.
   - `unsafe` is a keyword with no parser rule, and `13-RUNTIME.md` §6 is
     "NOT BUILT".
   - There is no way to link a C library, and no `llvm-c` headers on this
     machine. Binding LLVM's C API would need about 150 functions, and
     `libLLVM` on all three systems (decision 2).
6. **The LLVM backend is textual and barely run.**
   - `build --release` writes `<out>.nova.ll` and runs `clang -O2 -c`, or
     `llc`, then links with `libnova_runtime.a` through `cc`. It needs
     LLVM 15 or newer.
   - CI installs no LLVM. The golden suite runs through the JIT and
     Cranelift objects only, and no test runs async code the LLVM backend
     emitted.
7. **Today's codegen is deterministic; std's `Map` is not.**
   - Cranelift objects are byte-identical across runs, output names and
     source paths. The `.ll` of `05-json-api` (24,241 lines) is identical
     across three runs. WASM builds are identical.
   - A release binary is identical only for the same output file name:
     `intermediate()` embeds `<out>.nova.ll` in the object, so `-o r1`
     and `-o r2` differ in one byte. The same name in two directories
     gives identical bytes.
   - No timestamps reach any output. Rust `HashMap`s are used only for
     lookups, never iterated into output.
   - **std's `Map` hashes its keys with a per-process random seed,** so
     its `keys()` order changes between runs, and nothing overrides the
     seed. A Nova compiler that iterated a `Map` into its output would be
     nondeterministic, so std needs an **insertion-ordered map**.
   - GC addresses must never reach output either.
8. **Nova code is slower, mostly through allocation.**
   - The spike's lexer is **about 5× slower than Rust with an optimized
     runtime, and about 30× with the debug one** a checkout links. LLVM
     beats Cranelift by only about 10%.
   - Allocation is 60–70% of its instructions and the GC about 15%:
     every `Ok`, `Some` and record is boxed, 4–5 allocations a token.
     Bounds checks are out-of-line calls, 9%.
   - String literals allocate on every evaluation: a 32-arm keyword
     `match` cost 48% of the time, until dispatching on the first byte
     and the length halved the total.
   - Peak memory is 4× Rust's (104 MB against 27 MB on 4.6 MB of input).
   - For scale, S0 checks hello world, with all of std, in 0.13 s as a
     debug build.
9. **The spike: the lexer, in today's Nova (scratch only).**
   - 892 code lines against Rust's 904; `logos` became a 250-line scanner.
   - Token dumps were **byte-identical on 264 of 264 files**: all 261
     under `std/`, `examples/` and `tests/`, and the port's own 3 (162,121
     lines).
   - A fuzzer wrote 23,000 files over 5 seeds, with about 200,000 lex
     errors of all 7 kinds. All matched after one fix, to a bug in the
     port's float workaround, which the corpus could not have shown.
   - Untested: recursion depth (the lexer is iterative) and GC pauses on
     a large live heap, such as the AST of all of std.
10. **The tools on this machine:** `clang` 18 and `llc`, and `libLLVM`
    17, 18 and 20 without headers. GitHub's `ubuntu-latest` has `clang`.

## 3. Key decisions (recommended now, confirmed before building)

Each is confirmed in its sub-phase's spec, and recorded in an ADR where it
departs from the master spec.

1. **Scope (5.0, by the user's decision).**
   - N covers the pipeline from source to `.ll`, the driver, and the
     commands `build` and `check`.
   - Diagnostics are compared by code and span, not by rendered text,
     unless a sub-phase's spec ports the renderer.
   - ADR 0048 records the scope and what Phase 5 leaves out:
     - pointer FFI, `unsafe` blocks, linking C libraries, and an LLVM C
       API binding;
     - porting `run`, `test`, fmt, the language server, doc, the package
       manager, and the Cranelift and WASM backends;
     - S1 as the shipped `nova`;
     - the byte gate on macOS and Windows;
     - N built for WASM, which the Phase 6 playground needs;
     - trait objects, mutable globals, and nested module directories.
   - A sub-phase's spec may pull one back in, with a ruling that names
     ADR 0048.
2. **The backend: textual `.ll` through `clang` (by the user's
   decision).**
   - N ports `nova-codegen-llvm`'s emitter: formatting strings only.
   - N runs `clang` on its `.ll` and `cc` on the object and
     `libnova_runtime.a`, as `link.rs` does, through 5.0's process spawn.
   - The spawn is a runtime builtin written in Rust, not user FFI.
   - ADR 0048 records the departure from "bind LLVM via `unsafe` extern",
     as `14-CODEGEN.md`'s note of 2026-10-06 recorded "no `inkwell`".
     Pointer FFI and `unsafe` go to the backlog.
   - Emitting the same text as S0 is what makes the gate testable
     (decision 3).
3. **The gate (5.6, by the user's decision).** On Linux, in CI, with one
   pinned `clang`:
   1. **The IR differential:** S0 and S1 emit identical `.ll` for each
      file of a fixed corpus: `tests/runtime/`, `examples/` and N.
   2. **S1 == S2 byte for byte.** It follows from 1 when S0 builds S1
      with `--release` (not Cranelift) and both stages go through the
      same `clang`, flags and link.
   3. **S2 == S3**, the classic fixpoint, which needs no agreement with
      Rust.

   The tolerances, recorded in the boundary ADR:
   - the intermediate file name the object embeds: a constant name, or
     the same output name for every stage;
   - the `clang` version, pinned;
   - no build IDs (`-Wl,--build-id=none`, or a comparison after `strip`);
   - Linux only. macOS and Windows keep their behaviour tests.
4. **Where N lives (by the user's decision).**
   - `selfhost/` is a Nova package with flat modules, about 40 files.
   - There is no new Rust crate. The Rust side gains only what N needs
     from std and the runtime (5.0), and S0's dumps (decision 6).
   - Any new crate, or any change to `Cargo.lock`, is asked of the user
     first.
   - `clang` is installed in CI only. `cargo test` never needs it, and a
     test that needs it fails in CI when it is missing.
5. **What ships (by the user's decision).** `v0.5.0` ships the Rust
   `nova`, released by `docs/releasing.md`. N is in the repository but
   not published, and S1 is no user's tool.
6. **The differential and the fuzz harness, in every sub-phase (by the
   user's decision).**
   - S0 gains hidden dumps that print each stage's output in one
     canonical text: tokens, the AST, resolution or HIR, types and
     diagnostics, MIR, and `.ll`. N prints the same.
   - Each sub-phase's gate compares N's dump with S0's on the corpus, and
     on random inputs from a fuzzer, as the spike did.
   - Until 5.5, S1 can be built by `nova build` (Cranelift), so these
     checks need no `clang` and run in `cargo test` on all three
     systems.
   - CI runs the fuzzer on fixed seeds, so its runs repeat. Longer runs
     are local.
   - **Two compilers in step:** once a part is ported, a change to the
     language changes both, in one pull request, and the differential
     enforces it. A part not yet ported changes in Rust only, and is
     ported from the Rust of its day.
7. **The bootstrap on every pull request (by the user's decision).** A CI
   job on `ubuntu-latest` builds S1, S2 and S3 and runs the gate. If it
   is too slow, the user is asked before it moves to a nightly job.
8. **Speed (by the user's decision).**
   - S1 and S2 link a release build of `libnova_runtime.a`
     (`NOVA_RUNTIME_LIB`).
   - 5.1 measures the parser on all of std, and 5.2 the peak memory and
     the GC's pauses with std's whole AST alive.
   - N is written to allocate little: it dispatches on bytes rather than
     on string literals, and keeps order in `Vec`s.
   - Work in the runtime or the compiler for speed (interned string
     literals, unboxed `Option` and `Result`, inline bounds checks) is
     done only if the bootstrap is too slow, and only on the user's
     word.
9. **Determinism, by construction.**
   - N iterates only `Vec`s and 5.0's insertion-ordered map into its
     output. A seeded `Map` is for lookups only, which 5.6's spec checks
     by a search of N.
   - No address, time or environment value reaches the output.
10. **Rust, and tests.**
    - Rust changes build on 1.78, the workspace's minimum, which CI's
      MSRV job checks.
    - Tests never touch the internet.

## 4. Sub-phases (each gated, reviewed, and merged on the user's word)

Each sub-phase builds on the one before it. Each runs the established
loop: a spec, a plan, the implementation, one fresh review and one fix
pass, a PR, and a merge on the user's word.

From 5.1 on, each gate has two parts: N's dump equals S0's on the corpus
(`std/`, `tests/`, `examples/` and N), and on the fuzzer's inputs.

### 5.0 — The language and std that a compiler needs
May split into 5.0a and 5.0b, as its spec decides.

- **ADR 0048**, the scope and the backend (decisions 1 and 2).
- **5.0a, the language,** in S0's lexer, parser, type checker, MIR and both
  native backends, with `nova fmt` and the language server following:
  - `?` on `Result` and `Option`;
  - tuples: types, literals, patterns and fields;
  - nested patterns;
  - `+=` and its kin on fields (`self.f += x`);
  - the newline continuation: a fix, or a lint that flags a line
    starting with `-` or `(` that continues the one before.
- **5.0b, std and CI:**
  - a string builder, with linear appends;
  - `Char` ↔ `Int` and `[Char]` → `String` for user code;
  - `parse_int` and `parse_float`;
  - array helpers: `iter`, `slice`, and `Vec` to `[T]`;
  - an insertion-ordered map;
  - `std/process`: spawn a process and wait for its exit code, and read
    environment variables;
  - the golden suite under `--release` in CI, with a pinned `clang`
    installed by the job.
- **Needs first:** nothing; it starts the phase.
- **Gate:**
  - Small programs use each new feature, under `run`, `build` and
    `--release`.
  - The golden suite passes under `--release` on Ubuntu.
  - The WASM suite still passes, and `nova fmt`'s checks pass on every
    `.nova` file.
- **Questions for its spec:**
  - a fix or a lint for the continuation, and what it does to existing
    code;
  - the builder's API, and whether `String + String` joins it;
  - the map's name, and whether `Map` keeps its seed;
  - spawn's API (`run(cmd, args)`, with or without captured output);
  - whether tuples reach the WASM backend in 5.0;
  - the `clang` version CI pins;
  - whether the shadowing of std's variant names gets a warning.

### 5.1 — The lexer, the parser and the AST
- **N's first modules:** the tokens, a hand-written scanner, the AST,
  recursive descent with error recovery, and the P-codes.
- **The dumps:** a token dump on both sides (the spike's format), and a
  canonical AST dump: S0's `nova parse --ast` prints Rust's `Debug`, so
  S0 gains a hidden dump that N matches.
- **The speed:** the parser measured on all of std, against S0, with an
  optimized runtime (decision 8).
- **Needs first:** 5.0a's `?`, tuples and nested patterns; 5.0b's
  strings and bytes.
- **Gate:**
  - token and AST dumps identical to S0's on the corpus;
  - parse errors identical by code and span;
  - the fuzzer's random sources give the same tokens, errors and trees.
- **Questions for its spec:** the AST dump's format; how the fuzzer makes
  inputs a parser gets far into (mutated corpus files, as well as random
  soup); how deep the recursion goes before N's stack runs out.

### 5.2 — The resolver and HIR
- **Modules:** loading a package's flat directory, std's 21 modules, the
  imports, scopes, and the builtin table.
- **HIR,** and a canonical dump of resolution and HIR.
- **The live heap:** peak memory and GC pauses with std's whole AST
  alive (decision 8).
- **Needs first:** 5.1, and 5.0b's insertion-ordered map.
- **Gate:**
  - HIR dumps identical to S0's on the corpus;
  - E0001-class diagnostics identical by code and span;
  - the fuzzer's mutated programs give the same dumps and diagnostics.
- **Questions for its spec:** how N finds std's sources (from disk, a
  path, or a generated module); the dump's format; how builtins and
  std-only names are tabled.

### 5.3 — The type checker
The largest part, 7,404 code lines; its spec may split it, for example:
- **5.3a:** inference, expressions, patterns and exhaustiveness;
- **5.3b:** traits and associated types, generics and bounds, and async
  signatures.

- **Needs first:** 5.2; tuples and nested patterns throughout.
- **Gate:**
  - N type-checks all of std and the corpus;
  - the inferred types (a dump) and the diagnostics, by code and span,
    are identical to S0's;
  - `check` agrees with S0's on the fuzzer's mutated programs.
- **Questions for its spec:** the split; what the type dump shows; how
  the 394 inline tests' cases reach N, for example as `.nova` files both
  compilers check.

### 5.4 — MIR
- **Lowering, monomorphization and `async_lower`,** with a canonical MIR
  dump.
- **Needs first:** 5.3.
- **Gate:** MIR dumps identical to S0's on the corpus, and on the
  fuzzer's programs that type-check.
- **Questions for its spec:** the MIR dump's format, and whether S0's
  numbering of locals and blocks is deterministic enough to compare.

### 5.5 — The `.ll` emitter, the driver and linking
- **The emitter,** a port of `nova-codegen-llvm`, and `--emit-ll`.
- **The driver:** the module loader, `clang` and `cc` through 5.0b's
  spawn, finding `libnova_runtime.a` as `link.rs` does, and the commands
  `build` and `check`.
- **Needs first:** 5.4, and 5.0b's spawn, environment variables and
  `--release` in CI.
- **Gate:**
  - `S1 --emit-ll` equals S0's `.ll` on the corpus, including N itself;
  - programs S1 builds pass the golden suite on Ubuntu.
- **Questions for its spec:** the intermediate file's name (the
  tolerance); N's command line and exit codes; whether `check` renders
  diagnostics or prints codes and spans.

### 5.6 — The bootstrap gate and `v0.5.0`
- **The CI job** that builds S1, S2 and S3 on every pull request, and
  runs decision 3's checks; its time measured (decision 7).
- **The records:**
  - documentation for `selfhost/` and the new std surface;
  - `CHANGELOG.md`;
  - dated notes in the specs;
  - **the boundary ADR**, as ADR 0047 closed Phase 4, with the gate's
    tolerances, the inventory and the backlog.
- **The release `v0.5.0`,** of the Rust `nova`, by the runbook
  (`docs/releasing.md`). Every publish waits for the user's word.
- **Gate:** S0 and S1 emit identical `.ll` on the corpus; S1 == S2 and
  S2 == S3, byte for byte; in CI on `ubuntu-latest`, with reproducible
  commands.
- **Questions for its spec:** the job's time and whether it stays per
  pull request; how N's determinism is checked (decision 9); what the
  release archive says about `selfhost/`.

## 5. Cross-cutting

- **Testing.**
  - Every differential runs in `cargo test` through the `nova` binary,
    with S1 built by Cranelift, until 5.5 needs `clang` (decision 6).
  - The fuzzer runs on fixed seeds in CI.
  - N's own unit tests are `nova test` files under `selfhost/`.
  - Tests never touch the internet.
- **Dependencies.**
  - No new Rust crate. Any crate, or any change to `Cargo.lock`, is asked
    of the user first, after its spec shows the change.
  - Rust changes build on 1.78, which CI's MSRV job checks.
  - `clang` is installed by CI jobs only, pinned, and never by a test or
    by `nova`.
- **Records.**
  - The master spec's Phase 5 and `14-CODEGEN.md` get dated notes as each
    sub-phase lands.
  - Each departure from them gets an ADR.
- **Language changes during the phase** land in both compilers once the
  part is ported (decision 6), and get `nova fmt` and the language
  server's support as before.
- **Publishing.** Every publish and every tag waits for the user's word.

## 6. Top risks

1. **The type checker is 37% of the port,** and std forces the whole
   language on N at once (§2 items 1 and 2). 5.3 may split, and its
   differential shows each divergence by span.
2. **The GC with a large live heap.** The spike never held more than one
   file's tokens; the AST and types of all of std are larger. 5.2
   measures pauses and peak memory before the type checker is built on
   them.
3. **Recursion depth.** Recursive descent and the type checker recurse on
   nesting. 100,000 frames ran in a probe, but with small frames; 5.1
   measures deep inputs.
4. **Two compilers drift apart.** Each port follows a moving Rust
   compiler. The differential in every pull request (decision 6) is the
   guard; a language change costs twice once its part is ported.
5. **Speed, and CI's time.** A compiler 5–10× slower than Rust, and 4×
   larger in memory, builds N twice for the gate. It is measured in 5.1
   and 5.6; a move to nightly, or work on speed, waits for the user.
6. **`clang` drifts.** A different `clang` can emit different objects, so
   the byte gate pins one version and the boundary ADR records it.
7. **std's `Map` order.** A seeded `Map` iterated into the output breaks
   S1 == S2 only sometimes. The insertion-ordered map (5.0b) and a search
   of N (decision 9) guard it.
8. **The release backend is barely tested today** (§2 item 6). Its golden
   runs start in 5.0b, before anything depends on them.
9. **The silent newline continuation** can mis-port code with no error;
   5.0a fixes it or flags it before the port begins.

## 7. Suggested first step

5.0's spec: the language additions and the std surface of 5.0a and 5.0b,
ADR 0048's scope and backend, and the `clang` that CI pins. The spike for
the lexer is done (§2 item 9), so nothing else comes first.
