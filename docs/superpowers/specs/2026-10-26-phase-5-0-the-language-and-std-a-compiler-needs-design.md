# Phase 5.0, "The language and std a compiler needs": design

- **Date:** 2026-10-26.
- **Status:** a draft for approval.
- **The user's decisions, 2026-10-26:**
  1. **Three pull requests, built in order: 5.0a, 5.0b, 5.0c.**
     - 5.0a, the small language and CI: ADR 0048, the golden suite
       through the LLVM backend in CI, `?`, `+=` on fields and elements,
       the newline rule, and `String + String`;
     - 5.0b, tuples and patterns;
     - 5.0c, std.
  2. **`?` converts nothing; `map_err` does.** It works on `Result` and
     on `Option`, and the error type of the value and of the function
     must be the same. It is desugared in the type checker; no backend
     changes.
  3. **Tuples have no impls, and reach every backend, WASM included.**
     They are anonymous records. With no `Eq` and no `Hash`, a tuple is
     not a `Map` key.
  4. **Patterns: nesting, guards, or-patterns, and char and range
     patterns.** This goes past the phase plan's list, on the user's
     word. There is no `is` and no `if let`.
  5. **The newline rule is a rule of the parser.** In a block, a line
     that starts with `(`, `[` or `-` begins a new statement; a bare
     `return` or `break` ends at the line break; `.` and every other
     operator continue the line before. No file in the repository
     changes.
  6. **`std/process`: plain functions `run` and `status`, with `env` and
     `current_exe`.** They block, and they are native only: on WASM,
     `std/process` is E0092.
  7. **`clang`: `ubuntu-24.04` and Ubuntu's `clang-18`, as a
     `cargo test`.** The runner is pinned, the `clang` is Ubuntu's
     18.1.3, its version is asserted, and the test is required under
     `NOVA_REQUIRE_CLANG=1`.
  8. **The four small items, all chosen:**
     - a string builder, a std record over `Vec<String>`, and
       `String + String`;
     - `OrderedMap`, a new type; `Map` does not change;
     - `Float.to_bits`, with a WASM shim;
     - a warning when a variant of the program's own shadows one of
       std's.
- **Not asked; the recommendation taken, which the user can overturn:**
  `match` is lowered to MIR by test chains, which keep today's flat
  switch, not by a Maranget decision tree (§4.6; §12 item 17).
- **Branch:** `claude/loving-archimedes-frtf5q-50`, at `1a1bbf9` (the
  approved phase plan), for this spec. Each part is built on a branch of
  its own from `main`, after the part before it merges; its plan names
  it.
- **Inputs:**
  - `docs/phase-5-plan.md`: §1 (the user's decisions), §2 items 3 to 8,
    §3 decisions 1, 2, 4, 9 and 10, §4's 5.0 entry and its questions,
    §5, and risks 8 and 9;
  - the research for this spec: the per-item findings with their files
    and lines, the phase's research, and the lexer spike's report
    (scratch only, summarised in §2);
  - `nova-spec/00-MASTER-SPEC.md` §1 (line 55, `?`; line 66,
    semicolons) and Phase 5; `11-PARSER.md` §2 and §6;
    `12-TYPESYSTEM.md` §3 and §8; `13-RUNTIME.md` §2.2; `14-CODEGEN.md`
    §4 and §6.2; `20-STDLIB.md` §17 and its module index;
  - ADR 0017 (`channel` without tuples), ADR 0028 §5 (fmt's `;`), ADR
    0043 (a lexer that skips line breaks), ADR 0047 (Phase 4's
    boundary and backlog).

## 1. What 5.0 delivers

After 5.0, the Rust `nova` (S0) compiles the language the Nova compiler
(N) will be written in, and std has what N's driver needs. Nothing of N
is written yet; that starts in 5.1.

- **5.0a, the small language and CI** (§3):
  - ADR 0048, Phase 5's scope and backend;
  - every golden program of `tests/runtime/` built with `--release`,
    through `clang`, and run, in a `cargo test` that CI's Ubuntu leg
    requires, on a pinned `clang` 18.1.3;
  - `?` on `Result` and `Option`;
  - `+=` and its kin on a field (`self.f += x`) and an element
    (`a[i] -= 1`);
  - the newline rule;
  - `String + String`;
  - two warnings: a variant that shadows one of std's, and a statement
    whose value is unused.
- **5.0b, tuples and patterns** (§4):
  - tuple types, literals and fields (`t.0`), on all four backends;
  - patterns that nest, with guards, or-patterns, `Char` literals and
    ranges, in `match`; tuple patterns, in `match`, `let` and `for`;
  - exhaustiveness and unreachable arms over all of them;
  - MIR lowering by test chains;
  - `for x in arr` over an array.
- **5.0c, std** (§5):
  - `StringBuilder`, over `Vec<String>`;
  - `Char` and `Int`, `[Char]` and `String`, and byte access to a
    `String` in constant time;
  - `parse_int`, `parse_int_radix` and `parse_float`;
  - `Float.to_bits`;
  - `iter` and `slice` on arrays, and `Vec` to and from `[T]`;
  - `OrderedMap`;
  - `std/process`: `run`, `status`, `env` and `current_exe`.
- **The gate** (the phase plan's, by part):
  - small programs use each new feature under `run`, `build` and
    `--release`, and, where the target has it, on WASM;
  - from 5.0a on, the golden suite passes under `--release` on Ubuntu,
    so 5.0b's and 5.0c's programs join it as they land;
  - the WASM suite still passes, and `nova fmt --check` passes on every
    `.nova` file of `std/`, `examples/` and `tests/`.

## 2. The starting point

Read on 2026-10-26, at `1a1bbf9`, with `nova` 0.4.0.

- **Each missing feature is one E0900.** They all come from
  `Checker::unsupported` (`crates/nova-typeck/src/check.rs:7577-7592`):
  - `?`: the parser builds `Expr::Try` (`grammar.rs:1971-1979`) and fmt
    prints it; the checker refuses it (`check.rs:3541`);
  - `r.f op= e` (`check.rs:6812`) and `a[i] op= e` (`check.rs:6749`);
    a local's `x op= e` already desugars to `x = x op e`
    (`check.rs:6612-6633`);
  - tuple types (`check.rs:2675`) and literals (`check.rs:3533`); tuple
    patterns fall into "this pattern form" (`check.rs:7266-7275`), with
    or-patterns, ranges, records, arrays and `@`;
  - nested patterns (`check.rs:7347`), `Char` and `Float` literal
    patterns (`check.rs:7091`), and guards (`check.rs:6890`);
  - `for x in arr` (`check.rs:4712`);
  - `let (a, b) = p` is E0022 (`check.rs:3321-3333`).
- **The parser has most of it already.** Tuple types, literals and
  patterns, or-patterns, ranges (`Int` with `..` and `..=`, `Char` with
  `..=`), guards and `for` patterns all parse, and fmt prints each.
- **`t.0` does not parse.** It is P0001, "expected identifier (in field
  access), found integer literal"; `t.1.0` lexes `1.0` as one float
  (`crates/nova-lexer/src/lib.rs:784`). `Ty` and `TyHead` have no tuple
  (`crates/nova-hir/src/lib.rs:23-90, 234-246`).
- **HIR patterns are flat,** `Variant { binders }`, and MIR lowers one
  level: a `Switch` on the tag or the value, or a chain of string
  compares (`crates/nova-mir/src/lower.rs:1259-1430`). Usefulness is
  Maranget's general algorithm already (`usefulness.rs:1-20`); only its
  conversion from HIR is shallow (`check.rs:8194-8205`), and it already
  skips guarded arms (`check.rs:6990`).
- **The lexer drops line breaks** (ADR 0043), and the parser ends a
  statement only where the expression cannot go on
  (`grammar.rs:1472-1499`). `let z = x`, then `-y` on the next line,
  silently means `x - y`. fmt works around it: it prints `;` before a
  statement that starts with one of `CONTINUES` (`(` `[` `{` `-` `*`
  `&` `|`, `crates/nova-fmt/src/print/mod.rs:24`), and after a bare
  `return` or `break` (`print/expr.rs:349-366`). **fmt breaks a long
  binary chain before each operator** (`print/expr.rs:674-691`).
- **Across all 275 `.nova` files,** in statement context: no line
  starts with `(`, `[`, `-`, `*` or `&`, and no `return` or `break`
  takes its value from the next line. 58 lines start with `.` and 7
  with `||`. So the newline rule changes the meaning of no file.
- **`String + String` is E0013** (`check.rs:6443-6450`). Interpolation
  lowers to one `str_concat_n`, so appending in a loop is quadratic:
  20,000 appends took 95 ms; 200,000 parts in a `Vec<String>` and one
  `join` took 77 ms with an optimized runtime.
- **Conversions are std-only:** `char_to_int` (a register move),
  `str_from_chars` and `str_to_float` (which returns `0.0` on bad
  input). There is no `Int` to `Char` at all, and no `as`. String
  indexing decodes the whole string on each call.
- **`[T]` has only `len()`** (`check.rs:5916-5938`), and no impl can
  name it. `Vec<T>` is `{ len, data: [T] }`, with no way to a `[T]`.
- **`Map` hashes with a per-process random seed,** so its `keys()`
  order changes between runs (`std/collections/lib.nova:191-227,
  500-505`).
- **`std/process` is `args()` and `exit()`,** 74 lines. There is no
  environment and no way to start a process. `link.rs` reads
  `NOVA_CLANG`, `NOVA_LLC`, `CC` and `NOVA_RUNTIME_LIB`, runs tools with
  captured output, and finds `libnova_runtime.a` beside the executable
  through `current_exe()`.
- **The LLVM `.ll` emitter writes each float as the hex of its bits**
  (`crates/nova-codegen-llvm/src/lib.rs:171-172`), so N needs
  `Float.to_bits` to emit S0's text in 5.5.
- **`--release` through `clang`:** of the 136 programs in
  `tests/runtime/` with a `.stdout`, 126 pass when built with
  `--release` (Ubuntu clang 18.1.3, debug runtime), 40 of them async.
  The other 10 need what their tests set up (standard input, arguments,
  a port, a directory) or are `@test` files, and give the same output as
  a Cranelift build. With 4 jobs, the run takes 48 s. Today only
  `release_builds_and_runs_when_clang_available` (`run_tests.rs:
  1748-1780`) and the browser job's blog build use `clang`, and nothing
  installs or pins it.
- **Diagnostic codes in use:** E0001 to E0004, E0010 to E0016, E0020 to
  E0022, E0060, E0070 to E0097, and E0900 and E0902. **E0098 onward is
  free.** A warning is a `Diagnostic::warning` with an E-code, as E0021
  is.
- **The spike** ported the lexer to today's Nova and hit each gap this
  spec closes: 23 `+=` rewrites, an inner `match` per nested pattern, a
  mutated position instead of a returned tuple, 45 lines of string and
  array workarounds, a float parser routed through `std/json` (where
  its one fuzz bug came from), and a `Number` variant that silently hid
  `JsonValue::Number`.

## 3. 5.0a, the small language and CI

### 3.1 ADR 0048, Phase 5's scope and backend

- **Scope:** N covers the pipeline from source to `.ll` (lexer, parser,
  resolver, HIR, type checker, MIR, the textual LLVM IR emitter), the
  driver, and the commands `build` and `check`. Diagnostics are compared
  by code and span, not by rendered text.
- **What Phase 5 leaves out,** each to the backlog unless a sub-phase's
  spec pulls it back with a ruling that names ADR 0048:
  - pointer FFI, `unsafe` blocks, linking C libraries, and a binding of
    LLVM's C API;
  - porting `run`, `test`, fmt, the language server, doc, the package
    manager, and the Cranelift and WASM backends;
  - S1 as the shipped `nova`; `v0.5.0` ships the Rust `nova`;
  - the byte gate on macOS and Windows;
  - N built for WASM, which Phase 6's playground needs;
  - trait objects, mutable globals and nested module directories.
- **The backend:** N writes textual `.ll` and runs `clang` and `cc`, as
  S0's `--release` does, through 5.0c's `std/process`. This departs
  from the master spec's "bind LLVM via `unsafe` extern", as
  `14-CODEGEN.md`'s note of 2026-10-06 departed from `inkwell`. Emitting
  S0's own text is what makes the gate testable.
- **The `clang` CI pins:** Ubuntu's `clang-18`, 18.1.3, on the
  `ubuntu-24.04` runner, its version asserted. When the runner image is
  retired, the pin moves by a new ADR.
- **5.0's three parts,** and how they amend the phase plan's 5.0a and
  5.0b.

### 3.2 The golden suite through LLVM

- **A new `#[test]`, `release_golden_suite`, in
  `crates/nova-cli/tests/run_tests.rs`:**
  - it walks every `tests/runtime/*.nova` that has a `.stdout`, less an
    exclusion table that names each file left out and why;
  - it builds each with `nova build --release` into a fixed temporary
    directory, `nova-release-golden/`, on a few threads at once
    (`std::thread::scope`, at most four);
  - it runs each binary with standard input closed and a time limit,
    and compares its standard output, with `\r` removed, with the
    `.stdout` file, and its exit status with the one the table expects
    (0 unless named);
  - it reports every failure at the end, not only the first, and prints
    one line: `release golden: <n> passed, <m> excluded, <clang>`.
- **Its gate:** without `NOVA_REQUIRE_CLANG=1`, the test prints that it
  did not run and passes, so a local `cargo test` and the macOS and
  Windows legs skip it. With `NOVA_REQUIRE_CLANG=1`, it runs, and fails
  when the `clang` that `nova` would use (`NOVA_CLANG`, or `clang`)
  cannot run, as `NOVA_REQUIRE_NODE` does for WASM
  (`wasm_tests.rs:496-510`).
- **The exclusion table, today:** `io_streams`, `process_args`,
  `file_open_dir`, `fs_permission_denied`, five `net_*` programs, and
  `nova_test`. A program whose setup is simple (standard input, an
  argument) may get it in the walk instead; the plan decides.
- **`release_builds_and_runs_when_clang_available`** stays as it is.
- **CI:** the `test` job's Linux leg runs on `ubuntu-24.04` in place of
  `ubuntu-latest`. On Linux only, a step installs `clang-18` with
  `apt-get` and fails unless `clang-18 --version` says `18.1.3`, and the
  `cargo test` step sets `NOVA_REQUIRE_CLANG=1` and
  `NOVA_CLANG=clang-18`. The other jobs keep their runners.
- **Not in the walk:** `NOVA_GC_STRESS`, since two programs time out
  under it on both backends (§10 item 6).
- **WASM, fmt, LSP, doc:** nothing.
- **Records:** ADR 0048 (the pin); `50-TESTING.md` and `14-CODEGEN.md`
  get dated notes; CHANGELOG's Changed (CI) entry.

### 3.3 `?`

```nova
fn first_word(s: String) -> Option<String> {
    let i = s.index_of(" ")?
    Some(s.slice(0, i))
}

async fn load(path: String) -> Result<Config, IoError> {
    let text = read_to_string(path).await?
    parse_config(text).map_err(|e: ParseError| io_error(e))
}
```

- **Semantics.** `e?` evaluates `e` once.
  - On `Option<T>`: `Some(v)` gives `v`; `None` returns `None` from the
    enclosing function, whose return type must be an `Option<U>`.
  - On `Result<T, E>`: `Ok(v)` gives `v`; `Err(x)` returns `Err(x)`,
    re-wrapped since the two `Result`s differ in `T`. The function's
    return type must be a `Result<U, E>` with the same `E`: nothing is
    converted, and `map_err` converts (decision 2).
  - The enclosing function is the nearest `fn` or closure. In an
    `async fn` its return type is the declared one, not the future; in
    a closure it is the closure's return type, which `?` constrains
    when it is still being inferred.
  - It is a postfix operator at precedence 12, as the parser has it:
    `a()?.b()?`, `x.await?` and `xs.get(0)?.len()` work.
- **`Option` and `Result` are std/core's,** found by their `DefId`s,
  not by the names in scope; `?` on a program's own sum named `Option`
  is E0098.
- **Errors:**
  - **E0098**, "`?` needs an `Option` or a `Result`": the operand is any
    other type. A note names the type.
  - **E0099**, "`?` cannot return from this function": the enclosing
    function's return type is neither, or is the other of the two. On
    an `Option` in a function that returns a `Result`, a note suggests
    `.ok_or(err)?`; in `main` or a `@test` function, a note says they
    return `()`.
  - **E0010** when the error types differ, with a note pointing at
    `map_err`.
- **Layers:** the type checker only. `e?` becomes the HIR the `for`
  desugar already builds (`check.rs:4790-4830`): a `Match` with a
  variant arm that binds the payload and an arm that returns. The
  `match` is flat, so MIR lowers it as today.
- **WASM:** nothing to build; one WASM program tests it.
- **fmt, LSP, doc:** nothing; fmt already prints `?`, and hover shows
  the type of `e?` through the HIR.
- **Tests:**
  - checker: each code and note; `?` in a closure whose return type is
    inferred; a program's own `Option`;
  - runtime programs: both sums; chains; inside `async fn` with
    `.await?`; in a closure; in a loop; in a `match` scrutinee; under
    `NOVA_GC_STRESS=1`; on WASM.
- **Records:** dated notes where the spec defers `?`
  (`00-MASTER-SPEC.md:358`, `20-STDLIB.md:1304`, `60-EXAMPLES.md:235,
  337, 359`) and in `12-TYPESYSTEM.md` (no conversion), whose §8
  catalog gains E0098 and E0099; CHANGELOG's Added.

### 3.4 `+=` and its kin on fields and elements

```nova
self.pos += 1
self.counts[k] -= 1
grid[i][j] *= 2
self.v.data[i] |= mask
```

- **Semantics.** `p op= e`, for any place `p` that plain assignment
  accepts and each operator `+= -= *= /= %= &= |= ^= <<= >>=`, means
  `p = p op e`, except that the place's receiver, and its index, are
  evaluated once. The order is: the receiver, the index, the old value,
  `e`, then the store, left to right, as a local's desugar does.
- **Errors:** those of plain assignment to the place (E0060 for an
  immutable root), and those of `op` on the two types (E0013).
- **Layers:** the type checker only. The receiver and the index go into
  unscoped locals (`fcx.new_local_unscoped`), then `check_field_set` or
  `check_index_set` stores `t.f op e` or `a[k] op e`, reusing
  `binary_result_ty`. Records and arrays are references, so the
  temporary aliases the place. MIR, backends and WASM: nothing.
- **fmt, LSP, doc:** nothing.
- **Tests:** every operator on a field and an element; nested places;
  a receiver with a side effect, run once (a call that counts); E0060
  on an immutable root; `+=` on a `String` field once §3.6 lands.
- **Records:** CHANGELOG's Added. `11-PARSER.md` §6 already allows it.

### 3.5 The newline rule

```nova
let z = x
-y            // a new statement, as `-y;`; E0103
let w = if c { 1 } else { 2 }
(a, b)        // a new statement, not a call of the `if`
return
cleanup()     // never the value of the `return`
let s = parts
    .iter()   // `.` continues, as today
```

- **The rule (decision 5).** In statement context, a line break ends
  the expression before a `(` (a call), a `[` (an index) or a binary
  `-`; the token begins the next statement or match arm. Every other
  operator, and `.`, continues the expression, as today.
- **Statement context** is the expression of a statement, a block's
  tail, a `let`'s value, a `return`'s or `break`'s value, and a match
  arm's body. It is off inside `(` and `)`, `[` and `]`, argument
  lists, record literals' braces, and the heads of `if`, `while`, `for`
  and `match`, which end at their `{`; a block anywhere turns it back
  on. A `{` on a new line keeps today's meaning.
- **`return` and `break` at the end of a line** take no value, wherever
  they are.
- **It implements the master spec,** "newline terminates statement"
  (`00-MASTER-SPEC.md:66`), which today's parser does not; its grammar
  in `11-PARSER.md` §2 still writes `;` after each statement.
- **It matters for 5.0b:** an arm that starts with a tuple pattern,
  `(a, b) => …`, on the line after an arm's body, would otherwise call
  that body.
- **Errors:** none new in the parser. **E0103**, a warning, "this value
  is not used", flags a statement, not a block's tail, that is a unary
  or binary operator other than an assignment, `&&` or `||`, a literal,
  a name, or a parenthesised, tuple or array expression. So a `-y` or a
  `(w)` left by the rule is reported, not silently discarded.
- **Layers:**
  - the parser needs to know where line breaks are: the lexer marks
    each token that follows one (a block comment that holds a line break
    counts), or the parser reads the source; the plan settles which
    (§13). A flag saved and restored like `in_no_struct_ctx`
    (`grammar.rs:69-89`) carries statement context;
  - the postfix loop does not take `(` or `[` after a line break in
    statement context, nor the binary loop `-`; `return` and `break`
    look for a line break before their value;
  - the type checker emits E0103.
- **fmt:**
  - `CONTINUES` loses `(`, `[` and `-`, and fmt no longer prints `;`
    after a bare `return` or `break`; no file holds either `;` today;
  - **a broken binary chain never starts a line with `-` in statement
    context,** since that line would now begin a statement. The plan
    chooses between the `-` ending its line and parentheses around the
    chain (§13), and fmt's tests check that every file of the corpus
    reformats to the same tree.
- **LSP, doc, doc tests:** they parse with `nova-parser`, so they follow.
- **WASM:** front end only.
- **The 5.1 lexer port** copies the line-break mark.
- **Tests:**
  - parser: each of `(`, `[` and `-` at a line start, in a block, a
    match arm, a `let`, inside parentheses, brackets, arguments and a
    record literal, in an `if` head, and after the `}` of `if`, `match`
    and `while`; `return` and `break` at a line end, with and without a
    value on the same line; a block comment holding a line break;
  - fmt: round trips; a long chain with `-` reformats to text that
    parses to the same tree; `nova fmt --check std examples tests`
    unchanged;
  - checker: E0103 on each form, and not on calls, assignments,
    `if`, `match` or a tail;
  - the spike's corpus: the same ASTs before and after.
- **Records:** dated notes in `11-PARSER.md` §2 and §6 and
  `40-TOOLING.md` (fmt), pointing at ADR 0028 §5 and ADR 0043;
  `12-TYPESYSTEM.md` §8 gains E0103; CHANGELOG's Changed, with the one
  way a program can change meaning (a line that starts with `(`, `[` or
  `-`).

### 3.6 `String + String`

- **Semantics.** `a + b`, both `String`, is a new `String` holding `a`
  then `b`. `s += t` follows, on a local and, with §3.4, on a field or
  an element. `String + Char` or `+ Int` stays E0013, as today, with a
  note suggesting interpolation.
- **Cost.** Each `+` copies both. Appending in a loop is quadratic, as
  interpolation is; the docs point at `StringBuilder` (§5.1).
- **Layers:** `binary_result_ty` accepts `String` for `Add`; MIR lowers
  it to the existing `StrConcat`. WASM has `rt_str_concat`
  (`std/rt/lib.nova:284`); the native backends call the runtime's.
- **fmt, LSP, doc:** nothing.
- **Tests:** runtime programs: empty strings, non-ASCII, `+=` on a
  local, in a loop, under GC stress, on WASM; an IR test that it lowers
  to one call.
- **Records:** a dated note in `12-TYPESYSTEM.md` (operators) and
  `20-STDLIB.md`'s strings; CHANGELOG's Added.

### 3.7 The variant-shadowing warning

- **E0102**, a warning, at a variant's definition: "variant `Number`
  hides std's `JsonValue::Number`". A note says that in this module the
  bare name now means the program's own variant, and that std's is
  `JsonValue::Number`.
- **When:** a sum type of the package being compiled, or of a path
  dependency, has a variant whose name is also a variant that std brings
  into every module (`Some`, `None`, `Ok`, `Err`, `IoErrorKind`'s, and
  the rest). std's own modules are never warned about. Shadowing by a
  function, a record or a type is not warned about, as `args` today.
- **Layers:** the resolver, which already tells a std name from a
  program's (`bound_outside_std`, `crates/nova-resolver/src/lib.rs:
  1628`).
- **WASM, fmt, doc:** nothing; the LSP shows it as it shows E0021.
- **Tests:** resolver tests for a hit, a qualified use that is fine, a
  std-only name, and no warning inside std; a CLI test that the warning
  prints and the program still builds. The plan counts the files of
  the repository where it fires.
- **Records:** `12-TYPESYSTEM.md` §8 gains E0102; CHANGELOG's Added.

## 4. 5.0b, tuples and patterns

### 4.1 Tuple types and literals

```nova
fn div_mod(a: Int, b: Int) -> (Int, Int) { (a / b, a % b) }

let p: (String, Int) = ("x", 1)
let one = (5,)                 // a tuple of one
let v: Vec<(Int, Token)> = Vec::new()
```

- **A tuple type** is `(T1, …, Tn)` for n of one or more; `()` stays
  the unit type, and `(T)` is `T`. A tuple of one is written `(T,)`, as
  the parser already reads it.
- **Tuples are anonymous records** (decision 3): two tuple types are the
  same when their element types are. They work as type arguments, in
  generic functions (`fn swap<A, B>(p: (A, B)) -> (B, A)`), in closures,
  in `Option` and `Vec`, and across `.await`.
- **They have no impls,** and no impl can name one: there is no
  `TyHead` for a tuple. So `==`, interpolation, `assert_eq` and `Map`
  keys need an `Eq`, `Display` or `Hash` a tuple lacks, and are E0013.
  `extern "C"` refuses them, as any type but `Int`, `Float` and `Bool`.
- **Layout:** a tuple lowers as a record does, `MakeRecord`, a heap
  object with 8-byte slots. This departs from `13-RUNTIME.md` §2.2 (no
  header when on the stack) and `14-CODEGEN.md` §4 (flattened when
  small), as records already do; ADR 0049 records it.
- **Errors:** E0010 for a tuple of the wrong arity or element type;
  E0011 when an element's type cannot be inferred.
- **Layers:**
  - HIR: `Ty::Tuple(Vec<Ty>)`, with each walk over `Ty` (substitution,
    application, occurs, unification, display, `match_pattern`);
    `Ty::Array` has 17 such arms in `nova-hir` and 34 in `nova-typeck`;
  - the checker: tuple types, literals and inference;
  - MIR: `mir_ty` (a pointer), `MakeRecord`, and mono's `type_name` and
    `mangle_ty`, which must give distinct tuple types distinct names
    (`crates/nova-mir/src/mono.rs:594-620`; a shared mangling has
    shipped as a miscompile twice);
  - the backends: nothing, since none sees a HIR type.
- **WASM:** through MIR records; tested in 5.0b.
- **fmt, doc:** they print tuples already. **LSP:** hover shows
  `(Int, String)` through `display_ty`.

### 4.2 Tuple fields: `t.0`

- **`t.N`** reads element `N`, counted from 0, of a tuple `t`, and is
  represented as `Expr::Field { field: "N" }`, so the AST, fmt and the
  language server's field handling keep working.
- **An index is plain decimal digits.** Whether `t.01`, `t.1_0` and
  `t.0x1` are refused (P0001), as Rust does, is the plan's to settle
  (§13).
- **The lexer:** after a `.` token, a float token whose text is digits,
  `.`, digits (`1.0`) is split into an integer, a `.` and an integer, so
  `t.1.0` reads as `(t.1).0` and `t.0.1.2` chains. A `..` before it, or
  any other token, leaves `1.0` a float. It splits by the source text,
  not the value, so `t.1.10` is `(t.1).10`.
- **The parser** accepts an integer after `.` (`grammar.rs:1930-1972`).
- **Assignment:** `t.0 = x`, and `t.0 += x`, follow the rule for a
  record field's.
- **Errors:** E0014, "no field `2` on `(Int, Int)`", for an index past
  the end or `.N` on a type that is not a tuple; E0011 on a value whose
  type is not yet known.
- **LSP:** no completion of indexes in 5.0.
- **The 5.1 lexer port** copies the split.

### 4.3 Patterns

```nova
match tok {
    Some(Token::Ident(name)) if is_keyword(name) => keyword(name)
    Some(Token::Int(0) | Token::Int(1)) => small()
    Some(Token::Char('a'..='z')) => lower()
    Some(Token::Int(n)) if n < 0 => negative(n)
    Some(_) => other()
    None => end()
}

let (q, r) = div_mod(a, b)
for (name, ty) in fields.entries() { … }
```

- **Nesting:** a variant's fields, and a tuple's elements, may be any
  pattern, to any depth.
- **The forms a pattern may take in 5.0b:**
  - `_`, a name (`x`, `mut x`);
  - literals: `Int`, `Bool`, `String`, and `Char` (new);
  - a variant with or without fields, by its bare or qualified name;
  - a tuple, `(p1, …, pn)`;
  - an or-pattern, `p1 | p2`, at any depth;
  - a range: `Int` with `lo..hi` or `lo..=hi`, `Char` with `lo..=hi`,
    as the parser reads them.
- **Still E0900:** record patterns, array patterns, `x @ p`, and `Float`
  literals.
- **A guard,** `pattern if cond =>`, in a `match` arm: `cond` is a
  `Bool` (E0010 otherwise), sees the arm's bindings, and runs after
  they bind; when it is false, matching goes on with the next arm.
- **Or-patterns** bind the same names, with the same types, in every
  alternative.
- **`let` and `for`** take an irrefutable pattern: names, `_`, tuples of
  irrefutable patterns, and a variant of a sum that has only one. A
  refutable one stays E0022, now decided by usefulness. There is no
  `is`, `if let` or `while let` (decision 4).
- **Errors:**
  - **E0100**, "the alternatives bind different names", or the same
    name with different types; the label names the missing binding;
  - **E0101**, "this range matches nothing", for `lo > hi`, or
    `lo >= hi` with `..`;
  - E0010 for a pattern of the wrong type or arity, or a range over a
    type that is not `Int` or `Char`;
  - E0020, E0021 and E0022 as today, over the new forms.
- **Layers:**
  - HIR: a recursive `Pattern` (`Variant { fields: Vec<Pattern> }`,
    `Tuple`, `Bind`, `Lit`, `Range`, `Or`), and `Match` arms with
    guards;
  - the checker: `check_pattern` recurses; guards; `let` and `for`
    destructuring;
  - usefulness: §4.4; MIR: §4.6;
  - the parser and fmt: nothing.
- **WASM:** MIR's existing statements only, so nothing.

### 4.4 Exhaustiveness and unreachable arms

- The conversion from HIR to usefulness's patterns recurses, and gains
  `Ctor::Tuple(arity)`, a type with one constructor.
- **Or-patterns** expand into their alternatives, as Maranget's
  algorithm does.
- **Guarded arms** count toward neither exhaustiveness nor another arm's
  reachability, as today.
- **`Int`, `Char` and `String` literals and ranges** are each their own
  constructor, of a type with too many values to list. So a `match` on
  them needs a `_` or a name to be exhaustive, and an arm is reported
  unreachable when an earlier arm covers it without a range; a range
  that earlier ranges cover is not reported. Whether usefulness splits
  ranges, as Rust's does, is the plan's to settle (§13).
- **Witnesses** nest: E0020 names `Some(Err(_))`, not `Some(_)`.
- **Tests:** usefulness unit tests with nested witnesses, unreachable
  nested arms, first match wins, or-patterns, guards, literals and
  ranges inside variants, tuples of sums, and generic sums.

### 4.5 `for x in arr`

- **`for p in a`,** where `a` is a `[T]`, runs the body once per
  element, in order, with `p` (§4.3) bound to it. `a` is evaluated once;
  its elements are read as the loop reaches them, so a write to a later
  element is seen. `break` and `continue` work.
- **Layers:** the checker desugars it to an index loop beside the
  range desugar (`check.rs:4478-4530`), the index advanced before the
  body so `continue` is safe. MIR and backends: nothing.
- **Errors:** as today for a value that is neither an array, a range
  nor an iterator.
- **Tests:** empty arrays; `break` and `continue`; a tuple pattern; a
  write inside the loop; WASM.

### 4.6 MIR: lowering by test chains

- **A `match` whose arms are all flat,** as today's checker accepts
  them, lowers exactly as today: one `Switch`, or a chain of string
  compares. Its MIR is byte for byte the same, so no golden or IR test
  moves.
- **Any other `match` lowers wholly by chains.** The scrutinee goes
  into a local once. Each arm, in order, is a chain of tests, left to
  right and depth first: a tag (`SumTag`), an `Int`, `Char` or `Bool`
  value, a string (`StrEq`), or a range (two compares), each on a value
  projected with `SumField` or `RecordField` into a temporary. A failed
  test jumps to the next arm's chain. After the tests, the arm binds
  its names, runs its guard if it has one, and enters its body; a false
  guard jumps to the next arm. After the last arm comes `unreachable`,
  which exhaustiveness guarantees.
- **An or-pattern** is one chain per alternative, each of which binds
  the same locals and jumps to one block that runs the guard and the
  body.
- **`let` and `for` destructuring** lower to projections, since their
  patterns are irrefutable.
- **No new MIR statement,** so the Cranelift, LLVM and WASM backends,
  and `async_lower`, need nothing.
- **Why chains (the recommendation, §12 item 17):** they are the
  simplest to write, to review and to port in 5.4, and they keep flat
  matches' MIR. A Maranget decision tree, as `14-CODEGEN.md` §6.2
  says, tests fewer times at run time, but duplicates or shares arm
  bodies, and is harder to write and to port. ADR 0049 records the
  departure.
- **Tests:** MIR tests of a nested match, a guard, an or-pattern and a
  range; IR tests that flat matches still emit `switch`; runtime
  programs (§7.3) under all four backends and GC stress.

### 4.7 5.0b's records

- **ADR 0049, "Tuples and patterns":** tuples as anonymous records, with
  no impls; their layout's departure; the pattern forms in and out;
  lowering by chains rather than a decision tree.
- **Dated notes:** `11-PARSER.md` §2 (patterns, defining
  `range_pattern` as the parser reads it; `t.N`); `12-TYPESYSTEM.md` §3
  (`Ty::Tuple`) and §8 (E0100, E0101); `13-RUNTIME.md` §2.2 and
  `14-CODEGEN.md` §4 and §6.2; `20-STDLIB.md:2142, 2340-2342` (tuples
  exist; `channel` and `Map` stay as ADR 0017 left them).
- **CHANGELOG's Added.**

## 5. 5.0c, std

Each item is Nova in std, on the targets its module has, unless it says
otherwise. Every new public item has a doc comment with an example, run
as a doc test.

### 5.1 `StringBuilder` (std/strings)

```nova
pub record StringBuilder { … }    // parts: Vec<String>, bytes: Int

impl StringBuilder {
    pub fn new() -> StringBuilder
    pub fn push(mut self, s: String)
    pub fn push_char(mut self, c: Char)
    pub fn byte_len(self) -> Int
    pub fn is_empty(self) -> Bool
    pub fn clear(mut self)
    pub fn to_string(self) -> String
}

impl Display for StringBuilder { … }
```

- **`push` is constant time,** amortised; `to_string` joins the parts
  in one copy, through `str_join`, so building a string of n bytes from
  any number of pushes is linear. `to_string` leaves the builder as it
  was.
- **`byte_len`** is the length in bytes, kept as parts are pushed;
  there is no `len`, since `String.len()` counts characters and that
  would cost a walk.
- **WASM:** `rt_str_join` exists (`std/rt/lib.nova:560`). Nothing else.
- **Tests:** empty, one part, non-ASCII, `push_char`, `clear`, `to_string`
  twice, interpolation through `Display`; a program that pushes a
  million one-byte parts, which a quadratic builder could not finish in
  the harness's time limit, with no timing assertion; GC stress on a
  smaller count; WASM.

### 5.2 `Char`, `Int` and `String` (std/core, std/strings)

```nova
impl Char { pub fn to_int(self) -> Int }
pub fn char_from_int(n: Int) -> Option<Char>
pub fn string_from_chars(cs: [Char]) -> String

impl String {
    pub fn byte_len(self) -> Int
    pub fn byte_at(self, i: Int) -> Int
    pub fn byte_slice(self, start: Int, end: Int) -> String
}
```

- **`to_int`** wraps `char_to_int`. **`char_from_int`** is `None` below
  0, above `0x10FFFF`, and for the surrogates `0xD800` to `0xDFFF`. It
  checks the range in Nova, then a new std-only builtin, `int_to_char`,
  makes the `Char`: a register move, as `char_to_int` is, with no
  runtime symbol on any target.
- **`string_from_chars`** wraps `str_from_chars`. The names follow
  `bytes_from_string` and `bytes_from_ints`.
- **Byte access in constant time:** `byte_len` is the length in bytes;
  `byte_at(i)` is the byte at `i`, from 0 to 255; `byte_slice(a, b)` is
  a copy of bytes `a` to `b`. `byte_at` panics past the end, and
  `byte_slice` when a bound is out of range, `a > b`, or a bound falls
  inside a character, as `a[i]` panics: neither allocates an `Option`,
  which the spike showed is what a lexer's time goes to. `String` and
  `Bytes` share a layout (`crates/nova-hir/src/lib.rs:31-43`).
- **WASM:** the register move needs nothing; byte access uses the
  `Bytes` builtins std/rt already has, or new ones in std/rt (§13).
- **Tests:** every edge of the scalar values both ways; byte access on
  one-, two-, three- and four-byte characters; each panic; WASM.

### 5.3 `parse_int`, `parse_int_radix` and `parse_float` (std/strings)

```nova
impl String {
    pub fn parse_int(self) -> Option<Int>
    pub fn parse_int_radix(self, radix: Int) -> Option<Int>
    pub fn parse_float(self) -> Option<Float>
}
```

- **`parse_int`:** an optional `+` or `-`, then one or more ASCII
  digits, and nothing else: no space, no `_`. `None` when the text is
  anything else or the value leaves `Int`'s range; `-9223372036854775808`
  parses.
- **`parse_int_radix`:** the same in radix 2 to 36, digits and letters
  of either case, with no prefix; it panics on any other radix. N needs
  it for `0x`, `0o` and `0b` literals.
- **`parse_float`:** the text Rust's `f64::from_str` accepts, and the
  value it gives, on both targets: an optional sign, digits with an
  optional `.` and exponent, or `inf`, `infinity` and `nan` in any case;
  `None` otherwise. So N parses a float literal by the algorithm S0's
  lexer uses.
- **Layers:** `parse_int` and `parse_int_radix` are Nova. `parse_float`
  is a new std-only validity builtin and `str_to_float`, natively from
  Rust's parser.
- **WASM:** `str_to_float` is a shim entry already; the validity check
  is one more, which follows Rust's grammar, and maps `inf` and
  `infinity` to JavaScript's `Infinity`.
- **Tests:** one table of cases, run natively and on WASM: signs, both
  ends of `Int`, overflow, `_` and spaces refused, each radix's digits,
  exponents, `inf` and `nan`, and the empty string.

### 5.4 `Float.to_bits` (std/core)

- **`impl Float { pub fn to_bits(self) -> Int }`:** the IEEE 754 bits of
  the value, as Rust's `f64::to_bits`, read as a signed `Int`. There is
  no `from_bits`; N needs only this way.
- **Layers:** a new std-only builtin, `float_to_bits`. Natively, a
  runtime function or a MIR bit cast (§13).
- **WASM:** a shim entry (decision 8), through a `DataView`, returning
  the `BigInt` the shim's `Int` results use.
- **Tests:** `0.0`, `-0.0`, `1.0`, the smallest subnormal, the largest
  finite, both infinities, a NaN; the same on WASM; the hex the `.ll`
  emitter writes for each.

### 5.5 Arrays and `Vec` (std/collections)

```nova
impl<T> [T] {
    pub fn iter(self) -> ArrayIter<T>
    pub fn slice(self, start: Int, end: Int) -> [T]
}
pub record ArrayIter<T> { … }     // impl<T> Iterator for ArrayIter<T>

impl<T> Vec<T> {
    pub fn to_array(self) -> [T]
    pub fn from_array(a: [T]) -> Vec<T>
}
```

- **`iter`** yields each element in order. **`slice(a, b)`** is a new
  array of the elements `a` to `b`, and panics when a bound is out of
  range or `a > b`.
- **`to_array`** copies the `Vec`'s elements into a new array of its
  length; **`from_array`** copies an array into a new `Vec`. Neither
  shares storage, since arrays are references.
- **The checker:** `Ty::head` gains `TyHead::Array`, so std can write
  `impl<T> [T]`, and an array's methods are looked up in it after
  `len`. Whether a program may write an inherent impl on `[T]` follows
  the rule for `impl String`.
- **WASM:** Nova over existing MIR, so nothing.
- **Tests:** empty arrays, slices at both ends, each panic, iterator
  adapters over `iter()`, round trips; WASM.

### 5.6 `OrderedMap` (std/collections)

```nova
pub record OrderedMap<K, V> { … }

impl<K: Hash + Eq, V> OrderedMap<K, V> {
    pub fn new() -> OrderedMap<K, V>
    pub fn len(self) -> Int
    pub fn is_empty(self) -> Bool
    pub fn insert(mut self, k: K, v: V) -> Option<V>
    pub fn get(self, k: K) -> Option<V>
    pub fn contains_key(self, k: K) -> Bool
    pub fn remove(mut self, k: K) -> Option<V>
    pub fn keys(self) -> [K]
    pub fn values(self) -> [V]
    pub fn entries(self) -> [(K, V)]
    pub fn iter(self) -> OrderedMapIter<K, V>   // yields (K, V)
}
```

- **Order:** the order keys were first inserted. Updating a key keeps
  its place; removing it and inserting it again puts it last.
- **Inside:** keys and values in insertion order, with a `Map<K, Int>`
  from key to position; `remove` leaves a gap, and gaps are compacted as
  the map grows. The `Map` is used for lookups only, so the order never
  depends on its seed.
- **`Map` does not change** (decision 8); its docs point at
  `OrderedMap` for a stable order.
- **WASM:** Nova, so std/collections has it on both targets.
- **Tests:** order after updates, removals, reinsertion and growth;
  `entries` and `iter`; a program run twice that prints the same order
  each time; WASM.

### 5.7 `std/process`: `run`, `status`, `env` and `current_exe`

```nova
pub record ProcessOutput {
    pub status: Int,
    pub stdout: Bytes,
    pub stderr: Bytes,
}

pub fn run(program: String, args: [String]) -> Result<ProcessOutput, IoError>
pub fn status(program: String, args: [String]) -> Result<Int, IoError>
pub fn env(name: String) -> Option<String>
pub fn current_exe() -> Result<String, IoError>
```

- **`run`** starts `program` with `args`, its standard input empty, and
  waits for it, capturing its standard output and error. **`status`**
  does the same with the three streams inherited. Both are plain `fn`s
  that block the thread, executor included (decision 6); their docs
  say so.
- **`program`** is a path, or a name searched for on `PATH`, as Rust's
  `Command` does, `.exe` included on Windows. The arguments are passed
  as they are, with no shell. The child inherits the environment and
  the working directory.
- **The status** is the exit code; a death by signal on Unix is
  `128 + signal`, as shells report it.
- **Errors** follow `std/fs`'s mapping of Rust's error kinds: a missing
  program is `NotFound`, one that cannot be run `PermissionDenied`.
- **`env(name)`** is `None` when the variable is unset, its value is not
  UTF-8, or `name` is empty or holds `=` or NUL.
- **`current_exe()`** is the running executable's path. Under
  `nova run`, that is `nova` itself, which its docs say.
- **Layers:**
  - the runtime: builtins over `std::process::Command` and `std::env`,
    on `std/fs`'s wire contract, a status code and `take` builtins
    (`std/fs/lib.nova:97-101`, `std/io/lib.nova:153`); the child's
    output is held until taken;
  - each builtin is in the resolver's enum, names and `STD_ONLY` list,
    the checker's signatures, `RtFunc` and the runtime's `symbols()`;
    the backends need nothing per builtin;
  - `std/process/lib.nova` and its module doc.
- **The names are glob-imported into every module,** as `args` and
  `exit` are; a program's own `run`, `status` or `env` shadows them.
  The record is `ProcessOutput`, not `Output`, which trait code uses.
- **WASM:** `std/process` is not among WASM's modules, so a WASM program
  that names `run` gets E0092 (decision 6). Nothing to build; one test.
- **Tests,** on all three systems, with no network:
  - a program built with `nova build` that runs itself through
    `current_exe()` with an argument, as a child that prints to both
    streams and exits with a chosen status; captured output, the status
    through `status`, arguments with spaces and non-ASCII;
  - a missing program is `NotFound`;
  - `env` for a variable the test sets on the program, and for one
    unset;
  - E0092 on WASM.

### 5.8 5.0c's records

- **ADR 0050, "The std a compiler needs":** the builder over
  `Vec<String>` rather than `14-CODEGEN.md` §6.1's
  `String::with_capacity`; `OrderedMap` beside `Map`; `std/process`'s
  `run` and `status` in place of `20-STDLIB.md`'s `spawn`, blocking
  while `std/fs` is async, and native only; panicking byte access.
- **Dated notes:** `20-STDLIB.md`'s module index and §17 (process),
  strings, collections and core; `14-CODEGEN.md` §6.1.
- **CHANGELOG's Added;** the documentation site follows from std's doc
  comments.

## 6. New diagnostic codes

| Code | Kind | Title | Part |
|---|---|---|---|
| E0098 | error | `?` needs an `Option` or a `Result` | 5.0a |
| E0099 | error | `?` cannot return from this function | 5.0a |
| E0100 | error | the alternatives bind different names | 5.0b |
| E0101 | error | this range matches nothing | 5.0b |
| E0102 | warning | a variant hides one of std's | 5.0a |
| E0103 | warning | this value is not used | 5.0a |

Codes are taken in this order whatever order the parts' tasks run in.
Every other error in 5.0 reuses a code: E0010, E0011, E0013, E0014,
E0020 to E0022, E0060 and E0092.

## 7. Testing

### 7.1 Unchanged rules

- Tests never touch the internet. `clang` is installed by CI's job, never
  by a test or by `nova`.
- Temporary directories have fixed names; a test that needs `$NOVA_HOME`
  sets it to a fresh one.
- MSRV is Rust 1.78. No new crate, and `Cargo.lock` does not change.

### 7.2 Each layer

- **Lexer** (`nova-lexer`'s tests): the line-break mark; `t.0`, `t.1.0`,
  `t.0.1.2`, `t.1.10`; `1.0` and `0..1` elsewhere unchanged.
- **Parser** (`nova-parser/tests`): the newline rule's cases (§3.5);
  `.N`; patterns and guards as they parse today.
- **Checker** (inline in `check.rs` and `check/`): each new code; `?`'s
  inference; `+=`'s places; tuples through generics; patterns, guards,
  or-patterns, ranges, `let` and `for` destructuring; `[T]`'s impls.
- **Usefulness:** §4.4's cases.
- **MIR** (`nova-mir/tests`): chains, guards, or-patterns; flat matches
  unchanged; tuple mangling distinct.
- **IR** (`nova-codegen-llvm/tests/ir_tests.rs`): `switch` for flat
  matches; `String + String`.
- **fmt:** the newline rule's layout, and every file of the corpus
  reformatting to the same tree.

### 7.3 Runtime programs

- Each feature has programs with goldens under `tests/runtime/`, run by
  `run_tests.rs` under `run` and `build`, and by
  `release_golden_suite` under `--release` from 5.0a on, so each of
  5.0b's and 5.0c's programs runs through the LLVM backend too.
- A program per part runs under `NOVA_GC_STRESS=1`: `?` across
  `.await`, tuples held across `.await`, nested matches, the builder.
- **WASM** (`wasm_tests.rs`, under `NOVA_REQUIRE_NODE=1` in CI): `?`,
  `String + String`, tuples, patterns, `for x in arr`, the builder,
  conversions, `parse_float`'s table, `to_bits`, arrays, `OrderedMap`,
  and E0092 for `std/process`.
- **Doc tests** for each new public std item.

### 7.4 Checks

`cargo fmt`, `cargo clippy -D warnings`, `nova fmt --check` over `std`,
`examples` and `tests`, std's doc tests, the MSRV job, the WASM suite,
and `git diff` on `Cargo.lock` empty, in each part.

## 8. Crates and files

- **5.0a:** `nova-lexer` (the line-break mark), `nova-parser`,
  `nova-resolver` (E0102), `nova-typeck` (`?`, `+=`, `String + String`,
  E0103), `nova-mir` (`StrConcat` for `+`), `nova-fmt`;
  `crates/nova-cli/tests/run_tests.rs`; `.github/workflows/ci.yml`;
  `docs/adr/0048-*.md`.
- **5.0b:** `nova-lexer` (the split), `nova-parser` (`.N`), `nova-hir`,
  `nova-typeck` (with `usefulness.rs`), `nova-mir` (`lower.rs`,
  `mono.rs`); `docs/adr/0049-*.md`.
- **5.0c:** `std/strings`, `std/core`, `std/collections`, `std/process`;
  `nova-resolver` and `nova-typeck` (builtins, `TyHead::Array`);
  `nova-mir` (the register move); `nova-runtime` (`process.rs`, the
  float builtins); `nova-codegen-wasm`'s shim and std/rt;
  `docs/adr/0050-*.md`.

## 9. Records

- **ADRs:** 0048 (5.0a), 0049 (5.0b), 0050 (5.0c).
- **The phase plan's 5.0 amendment,** dated, in 5.0a: three parts in
  place of 5.0a and 5.0b, and what moved where.
- **Dated notes** in the spec files each section names, and the master
  spec's Phase 5 when 5.0c lands.
- **CHANGELOG's Unreleased,** in each part: Added (each feature),
  Changed (the newline rule; CI's runner and `clang`).
- **`12-TYPESYSTEM.md` §8's catalog** gains each new code with its part.

## 10. Risks

1. **The newline rule changes a program's meaning silently** where a
   line starts with `(`, `[` or `-` and meant to continue. None in the
   repository does; E0103 reports the statements it leaves, and
   CHANGELOG says so. fmt must change in the same pull request, or its
   own broken chains would be misread.
2. **Lowering by chains can miscompile** a nested match. Flat matches
   keep their MIR; every pattern program runs on four backends, under
   GC stress, and under `--release`.
3. **Tuple mangling:** two tuple types sharing a name has the shape of
   two earlier miscompiles; mono's tests check distinct names.
4. **The lexer's split** could change other tokens. Its tests, and the
   spike's token dump over the corpus, check that only `.N` changes.
5. **The `clang` pin:** `ubuntu-24.04` will be retired, and `apt` could
   ship a different 18.1.3 build; the version step fails loudly, and a
   new ADR moves the pin.
6. **The golden walk's time,** about a minute on four cores, and slow
   programs under a time limit. It runs on one leg, without GC stress,
   in parallel.
7. **Global names:** `run`, `status`, `env` and `current_exe` join every
   module's scope. A program's own shadows them silently, as `args` is
   shadowed today.
8. **E0102 may fire in the repository's own programs** (a variant named
   `NotFound` or `Get`); the plan counts them and renames or accepts.
9. **`current_exe()` under `nova run`** is `nova`, so the process tests
   build their program first; on Windows a running executable is
   locked, so each test removes its build only after its child ends.
10. **Two compilers:** 5.0 changes only S0. 5.1 ports the lexer with the
    line-break mark and the split, and its differential checks them.

## 11. Not in 5.0

- `is`, `if let`, `while let`; record, array, `@` and `Float` patterns.
- Impls on tuples (`Eq`, `Hash`, `Clone`, `Debug`, `Display`), tuple
  `Map` keys, and structural `==`.
- A conversion for `?`, a `From` trait, and generic traits.
- `Map::entries` or `iter` with tuples, and `channel` returning a tuple
  (ADR 0017); `Map` is unchanged.
- A decision tree for `match`.
- A builder backed by a runtime byte buffer; `OrderedSet`.
- `as` casts, turbofish (`.parse::<Int>()`), `Float.from_bits`.
- A `Command` builder with a working directory, an environment or
  standard input; asynchronous processes; processes on WASM.
- `loop`, `while true` known to diverge, `[String; 32]`, `mut` fields,
  trait objects and mutable globals.
- Interned string literals, unboxed `Option` and `Result`, and inline
  bounds checks (the phase plan's decision 8).

## 12. Decisions made while writing this spec

1. **Three ADRs, one a part.** 0048 is the scope and the backend. 5.0a's
   language items implement the master spec, so they take dated notes;
   0049 and 0050 record 5.0b's and 5.0c's departures.
2. **The golden walk runs only under `NOVA_REQUIRE_CLANG=1`,** so a
   local `cargo test` and the macOS and Windows legs skip it; the
   hello-world `--release` test stays.
3. **CI's step asserts `clang`'s version,** not the test, so a local run
   with another `clang` still works.
4. **Only the `test` job's Linux leg moves to `ubuntu-24.04`.**
5. **`?` finds `Option` and `Result` by std/core's `DefId`s;** a
   program's own `Option` is not one.
6. **Two codes for `?`,** one for the operand and one for the function.
7. **E0103, a warning for an unused value,** catches what the newline
   rule leaves behind; it is not on calls, assignments, `&&`, `||`,
   `if`, `match` or a tail.
8. **fmt drops `(`, `[` and `-` from `CONTINUES`, and the `;` after a
   bare `return` or `break`,** and never starts a line with `-` in a
   broken chain in statement context.
9. **The newline rule is off in parentheses, brackets, arguments, record
   literals and the heads of `if`, `while`, `for` and `match`;** a `{`
   on a new line keeps today's meaning.
10. **E0102 covers variants only,** those std brings into every module,
    in the package and its path dependencies, never within std.
11. **A tuple of one, `(T,)`, is allowed,** since the parser and fmt
    already have it; `()` stays unit.
12. **The lexer splits `1.0` after a `.` by its source text,** not the
    parser by its value.
13. **`for` takes an irrefutable pattern,** by `let`'s rule.
14. **Literal and range patterns are each their own constructor** for
    usefulness, so no range is reported unreachable by other ranges.
15. **Range forms stay as the parser reads them:** `Int` `..` and `..=`,
    `Char` `..=`.
16. **Record, array, `@` and `Float` patterns stay E0900.**
17. **Lowering by chains** (the recommendation taken without asking):
    flat matches keep their switch, and a match with any other arm
    lowers wholly by chains.
18. **`StringBuilder`** is in std/strings, with `byte_len` and no `len`;
    `to_string` joins and leaves the builder as it was; it has
    `Display`.
19. **`byte_at` and `byte_slice` panic** rather than return an `Option`,
    so a lexer allocates nothing per byte.
20. **`parse_int_radix` joins `parse_int`,** for N's prefixed literals;
    `parse_float` follows Rust's grammar on both targets.
21. **`char_from_int` and `string_from_chars` are free functions,** named
    after `bytes_from_string`.
22. **`ProcessOutput`, not `Output`;** `run`'s standard input is empty;
    a signal is `128 + signal`.
23. **`Float.to_bits` alone,** with no `from_bits`.
24. **`Vec::to_array` and `from_array` copy.**

## 13. What each part's plan settles

- **5.0a:**
  - how the parser learns where line breaks are: a mark on each token,
    a table beside the tokens, or the source (14 call sites of `parse`
    in 12 files);
  - fmt's layout for a broken chain at `-`: the operator ends its line,
    or parentheses go around the chain;
  - the walk's exclusion table against setups for the simple programs,
    its time limit, its exit-status table and its thread count;
  - the CI steps' YAML;
  - E0103's exact list of forms, and E0102's list of std's variants and
    where the resolver computes it;
  - the count of files where E0102 or E0103 fires, and what is done
    about each.
- **5.0b:**
  - each site that walks `Ty`, and the mangled form of a tuple type;
  - the HIR pattern's shape and the projection temporaries of a chain;
  - whether `t.01`, `t.1_0` and `t.0x1` are P0001;
  - whether usefulness splits ranges, and whether an unreachable
    alternative inside an or-pattern is reported;
  - whether a flat outer `match` switches before inner chains, if MIR's
    size calls for it.
- **5.0c:**
  - byte access through new builtins or a no-copy view as `Bytes`;
  - a `str_join` that takes a count, to spare the builder a copy;
  - `float_to_bits` natively as a runtime call or a MIR bit cast;
  - the process builtins' names and wire contract, and how a test sets
    an environment variable on a built program;
  - whether `Vec` gains `at(i) -> T` and `last()`, which allocate no
    `Option`;
  - `OrderedMap`'s compaction threshold, and which file each item goes
    in.
