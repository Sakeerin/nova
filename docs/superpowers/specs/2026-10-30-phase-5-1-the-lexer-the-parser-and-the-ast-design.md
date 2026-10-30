# Phase 5.1, "The lexer, the parser and the AST": design

- **Date:** 2026-10-30.
- **Status:** approved by the user (2026-10-30).
- **The user's decisions, 2026-10-30:**
  1. **Two pull requests, built in order: 5.1a, then 5.1b.**
     - 5.1a: the package `selfhost/`, the lexer, a token dump on both
       sides, the fuzzer and the differential harness;
     - 5.1b: the AST, the parser, an AST dump on both sides, the
       comparison of parse errors, the depth limit, and the speed
       measurement.
  2. **A depth limit in both parsers.** S0 and N count how deeply the
       source nests. Past a fixed limit, each reports a new error code at
       that point, in place of a stack overflow. It is a small language
       change, recorded in an ADR and the CHANGELOG.
  3. **The AST dump is an indented S-expression with spans.** One node
     per line, indented by its depth, with a byte span on every node:
     ```
     (Fn 0..42 name=add pub=false
       (Param 7..13 x (TyPath 10..13 Int))
       (Block 24..42 …))
     ```
  4. **Parse errors are compared completely.** Every error, in order,
     with its span, its message and its notes. The newline rule's
     splits, from which E0103 is computed, are compared too.
  5. **The fuzzer is Rust, in `cargo test`, with fixed seeds.** It uses
     its own PRNG and no new crate. It writes random soup, and corpus
     files mutated token by token so that the parser gets far into them.
     CI runs the fixed seeds on every system; longer runs are local,
     through an environment variable.
  6. **Speed is measured and recorded, with no threshold.** Lexing and
     parsing all of std is timed against S0 with an optimized runtime,
     with peak memory, and recorded in an ADR. If N is more than 10×
     slower, the user is asked before 5.2 starts (phase plan decision 8).
  7. **The differential runs in `cargo test` on all three systems.** N is
     built by `nova build` (Cranelift, no `clang`), on Linux, macOS and
     Windows.
- **Not asked; the recommendation taken, which the user can overturn:**
  - the limit is 128 levels, and the code is P0002 (§4.4);
  - S0 runs its pipeline on a thread with a 64 MB stack, so the limit,
    not the platform's main-thread stack, is what a deep input meets
    (§4.4);
  - floats are dumped by their bits, not by a decimal rendering (§3.3);
  - the parser's tests' inline sources become the error corpus (§4.6).
- **Branch:** `claude/loving-archimedes-frtf5q-51`, at `25be9f3c` (`main`
  after 5.0c), for this spec. Each part is built on a branch of its own
  from `main` after the part before it merges; its plan names it.
- **Inputs:**
  - `docs/phase-5-plan.md`: §1 (the user's decisions), §2 items 3, 8 and
    9, §3 decisions 4, 6, 8, 9 and 10, §4's 5.1 entry and its three
    questions, §5, and risks 3, 4 and 5;
  - the research for this spec (scratch only, summarised in §2), and the
    lexer spike's report;
  - `nova-spec/11-PARSER.md` §§1, 4, 5 and 7; `00-MASTER-SPEC.md`'s
    Phase 5;
  - ADR 0048 (Phase 5's scope and backend), ADR 0049 (tuples and
    patterns), ADR 0050 (the std a compiler needs).

## 1. What 5.1 delivers

N's first modules, in `selfhost/`, and the harness every later sub-phase
reuses:

- **5.1a:** a Nova package that lexes a file exactly as `nova-lexer` does,
  and prints the same token dump as S0's new hidden dump, on every file of
  the corpus and on the fuzzer's inputs.
- **5.1b:** a parser and an AST in Nova that build the same tree as
  `nova-parser` and report the same errors, in order, with the same
  spans, messages and notes. The trees are compared through a canonical
  dump that S0 gains too. Both parsers refuse input nested past 128
  levels with P0002.

From 5.1a on, the lexer exists twice. Any change to the language's
tokens changes both compilers in one pull request, and the differential
enforces it (phase plan decision 6). From 5.1b on, the same holds for
the grammar.

## 2. The starting point

Measured on `main` at `25be9f3c`.

1. **Size.** In code lines (no blanks, comments or tests): the lexer is
   1,002, the parser 2,836 and the AST 476. That is 4,314 in all, 327
   more than the phase plan's figures. None of the three crates has an
   inline test. Their integration tests (`lexer_tests.rs`, `keywords.rs`,
   `parser_tests.rs`, `newline_rule.rs`, `tuples.rs`) hold 160 tests.
2. **The lexer.**
   - `Token` has 92 variants. Each is `Spanned`, with byte offsets
     (`u32`) and a `FileId`.
   - `lex` returns `Tokens`, which mark whether a line break comes before
     each token (comments included) and whether an `Int` is plain decimal
     digits.
   - A `Float` straight after a `.` is split into `Int`, `.`, `Int`
     (`t.1.0`).
   - `//` and `/* */` are returned only by `lex_with_comments`; `///` is a
     `DocComment` token.
   - Its 7 errors are all reported as L0001.
   - The scanner is logos 0.14, longest match. Two quirks follow from it,
     and N keeps both:
     - an integer that overflows is `UnexpectedCharacter` on its first
       digit, after which lexing moves on one character;
     - `InvalidNumberLiteral` comes only from `r#` with no `"` after it.
3. **The parser.**
   - It is recursive descent: 102 functions, with a 13-level precedence
     ladder rather than a Pratt table. One level of parentheses costs
     about 17 Rust frames.
   - Every error is P0001. The callers attach the code, never the
     parser. `ParseError` has 5 variants, and its messages use 77 context
     strings such as "(in let pattern)".
   - There is no cap on errors: 5,000 bytes of random soup gave 237.
   - It recovers to the next item keyword or `}`, or, inside a block, to
     the next `;` or `}`. Line breaks are not recovery points.
   - Its state beyond the tokens:
     - the struct-literal restriction in heads;
     - the newline rule's statement context, and its recorded splits;
     - the split of `>>` when closing generic lists;
     - the halt after a bare `return`;
     - the note on the arm after a split;
     - the names of dropped items, which only the language server uses.
   - One rollback, of generic arguments, restores the position but keeps
     the errors pushed meanwhile. N reproduces it.
4. **The AST.**
   - 24 structs, and 18 enums with 119 variants (`Expr` has 27).
   - Every node and name is `Spanned`. There are no IDs, no arena and no
     interning. Doc comments and `@attributes` are fields.
   - Parentheses are not a node: the inner node's span widens to cover
     them.
   - `nova parse --ast` prints Rust's pretty `Debug`: 27,030 lines for
     `std/core/lib.nova`, and 51.8 MB for all of std.
5. **Depth.**
   - S0 runs on the OS main thread, which has 8 MB on Linux and no guard.
     Its debug build aborts (exit 134) at about 230 nested parentheses,
     212 blocks, 198 `if`s, 216 `match`es or 264 calls.
   - `nova check` overflows sooner on long operator chains: about 800
     `+`s and about 1,056 `else if`s, because the resolver and the type
     checker recurse once per node.
   - The corpus nests at most about 14 deep.
   - A Nova program runs on the main thread too. A probe frame passes at
     about 130,000 frames, and a deeper one dies with a bare SIGSEGV (exit
     139) when built.
6. **The corpus.**
   - `std/`, `tests/` and `examples/` hold 281 `.nova` files (33,607
     lines, 1.36 MB). All of them lex and parse cleanly.
   - Of the 295 tracked `.nova` files, 2 fail on purpose, each with
     P0001: a parser fixture and an editor-completion fixture.
   - So error cases come only from the fuzzer, and from the Rust tests'
     inline sources.
7. **Speed.**
   - S0's debug build parses all of std, concatenated, in 0.11–0.14 s,
     with 24 MB peak memory.
   - There is no release `nova` here, and the spike's release runtime
     predates 5.0c.
   - The spike's lexer was about 5× slower than a release Rust lexer with
     an optimized runtime, and about 30× slower with the debug runtime.
8. **The spike against 5.0.**
   - `?`, tuples, nested patterns, `+=` on fields, byte access, the
     builder, `parse_int`, `parse_float`, the conversions and
     `OrderedMap` remove every workaround the spike recorded.
   - These remain:
     - a string literal allocates each time it is evaluated, so keyword
       dispatch goes by the first byte and the length;
     - `Some`, `Ok`, records and tuples are boxed;
     - there is no `loop`, and `while true` is not known to diverge;
     - there are no `[T; N]` types, negative literal patterns or `mut`
       record fields.
   - Reading a file is async only, so N's `main` is an `async fn`.

## 3. 5.1a, the lexer and the harness

### 3.1 The package

- `selfhost/` is a Nova package: `nova.toml` (name `selfhost`, edition
  2026, no dependencies), `src/main.nova`, flat modules and `tests/`.
- 5.1a's modules:
  - `token.nova`: the token kinds, their payloads, and each kind's
    `description()`, word for word as the Rust;
  - `lexer.nova`: the scanner, the line-break and plain-decimal marks,
    the tuple-index split, and comments;
  - `dump.nova`: the token dump (§3.3);
  - `main.nova`: the command line (§3.2).
- `nova fmt --check` covers `selfhost/`, in CI's step with std and
  examples.
- N's own `@test`s live in `selfhost/tests/`. A `cargo test` runs
  `nova test` there.

### 3.2 The command line

- `selfhost tokens FILE…` prints the token dump of each file, in order.
  Each file starts with a header line `== FILE` (the path as given).
- It reads each file with `std/fs` (so `main` is `async fn`), and exits
  0 even when a file has lex errors. The errors are part of the dump.
- An unreadable file prints `== FILE`, then `IOERR` and the error kind,
  then exits 2 after the remaining files.

### 3.3 The token dump, on both sides

S0 gains a hidden flag, `nova parse --dump-tokens FILE…`, with output
identical to N's. It is hidden from `--help` and carries no stability
promise. Per file:

- one line per token: `start end Kind [payload] [NL] [PD]`;
  - `NL` marks a line break before the token, and `PD` an `Int` that is
    plain decimal;
  - `Int` prints in decimal, and `Float` prints as its 16 hex digits of
    bits (`Float.to_bits`), so no decimal rendering must agree;
  - `Char` prints as its code point in decimal;
  - strings (`StrPart`, `RawStr`, `Ident` and `DocComment` text) print in
    double quotes. Bytes below 0x20, 0x7F, `"` and `\` print as `\xHH`;
    everything else prints as is;
  - `Eof` is the last token line;
- then one line per lex error, in the order they were found:
  `ERR start end message`, where the message is the error's `Display`
  text;
- then one line per comment: `COMMENT start end Line|Block`.

The format extends the spike's, adding the two marks, bits for floats,
and the header. The flag accepts several files, so the harness spawns
one S0 process per batch rather than per file.

### 3.4 The differential harness

`crates/nova-cli/tests/selfhost.rs`, a `cargo test` on all three
systems:

- **Build N once per test binary** with `nova build` in `selfhost/`. The
  output goes into a fixed temporary directory, and the build is shared
  through a `OnceLock`.
- **The corpus:**
  - every tracked `.nova` file outside `selfhost/`, listed from the
    source tree, not from `git`, with the two invalid fixtures among
    them;
  - N's own sources.

  Each batch goes through N and through S0 once. The two outputs must be
  identical byte for byte. On a mismatch, the test prints the first
  differing file and line, both sides' lines, and how to reproduce it.
- **The fuzzer:** §3.5's inputs, written to a fixed temporary directory
  and compared the same way.
- **The fixed seeds and counts** are chosen so the whole file runs in
  under a minute on CI's slowest system. The plan measures it.
- `NOVA_SELFHOST_FUZZ=SEED:COUNT` runs a longer local search. It never
  runs in CI.

### 3.5 The fuzzer

A module under `crates/nova-cli/tests/selfhost/`. It uses a splitmix64
PRNG written there, and no new crate. It has three generators:

1. **Lexical soup,** as the spike's `gen.py` made it:
   - quotes, `${`, `}`, `\u{…}`, `r#"`, comments, numbers that overflow,
     non-ASCII text, NUL, and `\r\n`;
   - weighted so that all 7 lex errors occur in every CI run, which the
     harness asserts.
2. **Mutated corpus files.** A corpus file is lexed by S0 into its
   tokens' spans. A random number of edits then delete, duplicate, swap
   or insert whole tokens, or flip single bytes. Most of each result stays
   well formed, so the lexer, and in 5.1b the parser, get far into it.
3. **Deep nesting** (5.1b): parentheses, blocks, `if`, `match`, calls,
   types and patterns, at depths around the limit (§4.4).

Every input that once found a difference is added as a fixed case under
`crates/nova-cli/tests/selfhost/cases/`.

### 3.6 Gate for 5.1a

- The token dumps are identical on the whole corpus, on all three
  systems.
- They are identical on the fixed seeds' inputs, every lex error kind
  among them.
- `nova test` passes in `selfhost/`, and `nova fmt --check` covers it.
- The full suite, `cargo fmt`, clippy with `-D warnings`, the MSRV job,
  and `git diff` on `Cargo.lock` are clean.

## 4. 5.1b, the parser and the AST

### 4.1 The AST in Nova

- `ast.nova` mirrors `nova-ast`: one Nova record per struct and one sum
  per enum, with the same fields, in the same order.
- Spans are a record of two `Int`s; the `FileId` is not kept, since N
  parses one file at a time until 5.2.
- Recursive nodes need no `Box`: Nova's sums are recursive. `Option<Box<…>>`
  becomes `Option<…>`.
- Names are `String`s. There is no interning (phase plan decision 8).

### 4.2 The parser in Nova

- The modules follow `grammar.rs`'s areas: `parser.nova` (state,
  tokens, recovery and errors), `parse_item.nova`, `parse_type.nova`,
  `parse_stmt.nova`, `parse_expr.nova`, `parse_pattern.nova` and
  `parse_view.nova`.
- Each Rust function has a Nova function of the same name, with the same
  structure, so a reviewer can read the two side by side.
- Each Rust idiom has a fixed translation, recorded in the ADR:
  - `std::mem::discriminant` comparisons become a token-kind number;
  - `peek().clone()` becomes a match on the borrowed token;
  - the 21 save-and-restore closures (`in_*_ctx`) become explicit saves
    and restores around the call;
  - `static HALT` becomes a stored `Eof` token;
  - the `&Tokens` lifetime becomes a field.
- **Behaviour ported exactly:**
  - every message, context string and note, word for word;
  - the generic-arguments rollback that keeps its errors;
  - `>>` splitting;
  - the statement recovery that ignores line breaks;
  - trailing-expression detection;
  - the lookahead for `component` and `view`;
  - the newline rule's splits and the halt after a bare `return`.
- **Not ported:** the names of dropped items. Only the language server
  uses them, and it stays Rust (ADR 0048).

### 4.3 The AST dump, on both sides

S0 gains a hidden flag, `nova parse --dump-ast FILE…`, with output
identical to N's `selfhost ast FILE…`. Per file:

- the header `== FILE`;
- the tree:
  - one node per line, indented two spaces per depth;
  - `(Kind start..end field=value … ` then the children, each on its own
    line, with the closing parentheses at the end of the last child's
    line;
  - scalar fields (names, operators, flags, literals) are written
    `key=value` on the node's line, in the struct's field order;
  - a node's children appear in field order;
  - an absent optional child prints nothing, and an empty list prints
    nothing;
  - literals and strings follow §3.3's rules: decimal `Int`s, floats as
    bits, and escaped strings;
  - doc comments and attributes are children: `(Doc start..end "text")`
    and `(Attr start..end name "arg"…)`;
  - the file's newline splits close the tree, one `(Split start..end
    Kind)` each, in order;
- then the errors, in the order the parser pushed them:
  - `ERR start end message`, with every lex error first, as S0's callers
    render them;
  - `PARSE start end message`;
  - one `NOTE text` line per note, after its error.

The plan writes the exact grammar of the dump, with one example per node
kind. S0's printer is written by walking `nova-ast` as `nova-fmt`'s
printer does. Both sides get a test that every variant of every enum
appears in the dump of some corpus or case file.

### 4.4 The depth limit (a language change)

- **Both parsers count nesting.** A level is one entry into:
  - an expression's operand: parenthesised, in a block, in a call's
    arguments, in an `if`, `match`, `for` or `while`, or behind a prefix
    operator;
  - each further operator of a binary chain, and each further `else if`;
  - a type's arguments;
  - a pattern's sub-patterns;
  - a block.
- **Past 128 levels,** each reports **P0002**, "this is nested too deeply
  (more than 128 levels)", at the token where the limit was crossed. It
  then stops parsing the file: the tree holds what came before, and no
  further error is reported. One clear error replaces a cascade, and
  both parsers stop at the same token.
- **Why 128:**
  - the corpus nests at most about 14;
  - S0's debug build overflows from about 198 levels (`if`) on 8 MB;
  - its later stages from about 800 operators;
  - the limit is far from both.
- **Why a 64 MB stack for S0:** Windows' main thread has 1 MB, and S0's
  debug build there may overflow before 128. The plan probes it.
  - S0's commands that run the pipeline (`check`, `build`, `run`, `test`,
    `parse`, `doc`, `fmt`, and the language server's analysis) run it on
    one spawned thread with a 64 MB stack, as the WASM backend already
    does for its own thread.
  - So the limit, not the platform, decides what a deep input meets.
- **N's own stack:** a Nova program runs on its main thread too.
  - The plan measures N at depth 128 for each kind of nesting, on all
    three systems, through the fuzzer's third generator.
  - If N overflows on any system, the plan stops and asks the user:
    either the limit is lowered, or `nova build` links a larger main
    stack.
- **Records:** P0002 in `11-PARSER.md` §5 and the diagnostics catalog,
  the CHANGELOG's Changed ("deeply nested source is now P0002, not a
  crash"), and the ADR.

### 4.5 Errors compared completely

The dump carries every lex error and every parse error, in order, with
its span and message, and every note. The newline splits are in the
tree. So N matches S0 on what a user would see from `nova parse`. It
also matches on what E0103 is computed from, though E0103 itself is
rendered by the checker (5.3).

### 4.6 The error corpus

- The inline sources of `nova-lexer`'s and `nova-parser`'s integration
  tests, with the parser's six fixtures, are copied into
  `crates/nova-cli/tests/selfhost/cases/` as `.nova` files: about 160
  inputs, each made by hand to hit one path.
- The plan extracts them with a script, and records which test each one
  came from.
- The harness runs them with the corpus.

### 4.7 Speed

- The plan measures, locally and with an optimized runtime
  (`NOVA_RUNTIME_LIB` pointing at a release `libnova_runtime.a`):
  - N lexing and parsing all of std, concatenated, and file by file;
  - the same with S0 built in release mode;
  - peak memory for each.
- The figures, and the factor between N and S0, are recorded in 5.1b's
  ADR.
- If N is more than 10× slower, the user is asked before 5.2 begins
  (user decision 6). CI checks no time.

### 4.8 Gate for 5.1b

- The AST dumps, errors and notes included, are identical on the whole
  corpus and on the error cases, on all three systems.
- They are identical on the fixed seeds' mutated and deep inputs.
- P0002 is reported by both parsers on every kind of nesting at 129
  levels, and never at 128.
- No S0 command overflows its stack on the fuzzer's deep inputs.
- Speed is measured and recorded.
- The 5.1a checks hold.

## 5. New diagnostic code

| Code | Kind | Title | Part |
|---|---|---|---|
| P0002 | error | this is nested too deeply | 5.1b |

5.1a adds no code.

## 6. Testing

### 6.1 Unchanged rules

- Tests never touch the internet.
- Temporary directories have fixed names. A test that needs `$NOVA_HOME`
  sets it to a fresh one.
- MSRV is Rust 1.78. There is no new crate, and `Cargo.lock` does not
  change.
- The differential needs no `clang`.

### 6.2 Each part

- **5.1a:**
  - `selfhost.rs`: the corpus, the soup, the mutations, and the fixed
    cases;
  - a test that the hidden dump is absent from `nova parse --help`;
  - N's own `@test`s for the scanner's edges (escapes, raw strings,
    interpolation depth, the overflow quirk, the float split);
  - a test that the harness's mismatch report names the file and line,
    run against a deliberately wrong dump.
- **5.1b:**
  - the corpus, the cases, the mutations and the deep inputs;
  - P0002 in `nova-parser`'s tests: each kind of nesting at 128 and at
    129, and that parsing stops after it;
  - a test that S0's pipeline runs on the big-stack thread, as a probe
    at depth 128 on all three systems;
  - a test that every AST variant appears in some dump;
  - the language server and fmt on a P0002 file (one diagnostic, no
    crash).

### 6.3 Checks

In each part: `cargo fmt`, `cargo clippy -D warnings`, `nova fmt
--check` over `std`, `examples`, `tests` and `selfhost`, the MSRV job,
the WASM suite, and `git diff` on `Cargo.lock` empty.

## 7. Crates and files

- **5.1a:**
  - `selfhost/` (new);
  - `nova-cli` (the hidden `--dump-tokens`, and
    `tests/selfhost.rs` with `tests/selfhost/`);
  - `nova-lexer` (a `dump_tokens` function next to the lexer, so S0's
    dump lives with the code it describes);
  - `.github/workflows/ci.yml` (`nova fmt --check` gains `selfhost`);
  - `docs/adr/0051-*.md`.
- **5.1b:**
  - `selfhost/` (the AST, the parser, `selfhost ast`);
  - `nova-parser` (the depth count and P0002);
  - `nova-ast` (a `dump` module);
  - `nova-cli` (`--dump-ast`, and the big-stack thread for the
    pipeline);
  - `nova-lsp` (its analysis on the same thread size);
  - `nova-cli/tests/selfhost/cases/`;
  - `docs/adr/0052-*.md`.

## 8. Records

- **ADR 0051** (5.1a):
  - `selfhost/`'s layout and command line;
  - the token dump's format;
  - the harness and the fuzzer;
  - the rule that a change to tokens changes both compilers.
- **ADR 0052** (5.1b):
  - the AST dump's format;
  - each Rust idiom's Nova translation;
  - the depth limit and the big stack, with their measurements;
  - the speed figures;
  - the grammar rule that both compilers change together.
- **Dated notes** in `11-PARSER.md` (§4 the dumps; §5 P0002) and in the
  master spec's Phase 5, as each part lands.
- **The phase plan,** dated in 5.1a: 5.1 is two parts.
- **CHANGELOG's Unreleased,** in each part:
  - Added: the self-hosted lexer, then the parser; the hidden dumps;
  - Changed (5.1b): P0002, and the pipeline's thread.

## 9. Risks

1. **The two lexers or parsers disagree on input no generator makes.**
   The fuzzer's three generators, the hand-made cases, and the
   requirement that every lex-error kind and every AST variant occur all
   narrow it. Every difference ever found becomes a fixed case.
2. **The depth limit rejects real code.** The corpus nests at most about
   14, against a limit of 128. CHANGELOG and P0002's message say what
   changed.
3. **The big-stack thread changes behaviour.** For example, a panic's
   message or exit code, or a thread-local the language server keeps.
   The suite runs every command through it, and the plan checks panics'
   exit codes.
4. **N overflows before 128 on Windows.** The plan measures it, and asks
   the user if so (§4.4).
5. **CI time.** Building N and running the corpus and fuzzer on three
   systems adds time, bounded to under a minute per system by the
   chosen counts. If more is needed, the user is asked.
6. **A slow N.** Recorded, and asked about past 10× (§4.7).
7. **The lexer's quirks** (the overflow rule, logos's one-character skip)
   are easy to get wrong. The fuzzer found one such bug in the spike that
   the corpus could not show, and its soup targets them.

## 10. Not in 5.1

- The resolver, HIR, and std's loading (5.2).
- Rendering diagnostics through `codespan`-style output in N: N prints
  dumps only, until the driver (5.5).
- Interned strings, unboxed `Option`, or other runtime work for speed
  (phase plan decision 8, only on the user's word).
- `loop`, `[T; N]` types, negative literal patterns and `mut` record
  fields: N is written without them.
- Porting `nova-fmt`, the language server or `nova doc` (ADR 0048).
