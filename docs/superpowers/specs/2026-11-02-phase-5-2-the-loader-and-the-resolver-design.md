# Phase 5.2, "The loader and the resolver": design

- **Date:** 2026-11-02.
- **Status:** draft, for the user's approval.
- **The user's decisions, 2026-11-02:**
  1. **5.2 is the loader and the resolver, and HIR moves to 5.3.**
     - HIR is the type checker's output in S0, and paths, locals and types
       are resolved inside the type checker.
     - So 5.2 ports what happens before the type checker: loading a
       program's modules, and the item-level resolver. That covers
       definitions, imports, std's globs, the builtin table, std-only
       names and the resolver's 11 codes.
     - HIR is ported with the type checker in 5.3, where it is built.
     - The phase plan is amended to say so.
  2. **N reads std from disk, through `--std DIR`.**
     - The harness passes the repository's `std/`.
     - In 5.5 the driver finds std next to its own executable or in
       `$NOVA_HOME`, as the language server already does.
  3. **S0 keeps insertion order where an error order depended on a hash
     map.**
     - The E0002s for names a glob import clashes with come out in the
       order their names were declared.
     - The fix keeps a `Vec` beside the map and adds no crate.
     - It is a small change in the order of errors, recorded in the
       CHANGELOG.
  4. **Two pull requests, built in order: 5.2a, then 5.2b.**
     - **5.2a:**
       - the resolution dump on both sides;
       - N's loader and resolver for loose programs (a file and the
         sibling files it imports);
       - std read from disk;
       - the resolver's 11 codes;
       - the error cases and a name-level fuzzer.
     - **5.2b:**
       - packages (`nova.toml`, `src/` and `tests/`, a path dependency);
       - the WASM target and test mode;
       - the live heap's figures, recorded.
  5. **The dump is an S-expression like 5.1's, and std's part is dumped
     once.**
     - std's resolution is the same in every native program, so it is
       dumped and compared once per target.
     - A program's part prints std's definitions by their std-relative
       ids.
  6. **The inputs come from a name-level fuzzer and the Rust tests'
     sources.**
     - The fuzzer mutates programs that already parse, at the level of
       names, so that each of the 11 codes occurs.
     - The inline sources of the resolver's and the type checker's tests
       become fixed cases, as 5.1b did.
  7. **The live heap is measured with outside tools, and recorded in the
     ADR.**
     - There is no runtime change.
     - Peak memory and GC pauses are taken with RSS and gdb, as the
       research did, once N's resolver runs on all of std.
- **Not asked; the recommendation taken, which the user can overturn:**
  - diagnostics are compared completely, as in 5.1 (§3.6);
  - S0's dump is a hidden flag on `nova check` (§3.4).
- **Branch:** `claude/loving-archimedes-frtf5q-52`, at `1fa7cd3f` (`main`
  after 5.1b), for this spec. Each part is built on a branch of its own
  from `main`; its plan names it.
- **Inputs:**
  - `docs/phase-5-plan.md`: §1, §2 items 1, 2, 7 and 8, §3 decisions 4,
    6, 8 and 9, §4's 5.2 entry and its three questions, and risks 1, 2, 4
    and 7;
  - the research for this spec (scratch only, summarised in §2);
  - ADRs 0048 to 0052;
  - the 5.1 spec and the 5.1a and 5.1b plans.

## 1. What 5.2 delivers

N's third and fourth stages come after the lexer and the parser:

1. a **loader** that turns a program's root file into its modules in
   S0's order;
2. a **resolver** that builds what S0's `resolve_program_for` builds:
   - the definitions and their ids and names (method names mangled as S0
     mangles them);
   - each module's three namespaces (values, types and traits), with
     builtins seeded, std's exports globbed in, and std-only names seen
     only inside std;
   - the exports, the tests and the merged item list;
   - every diagnostic in S0's order.

A canonical **resolution dump** on both sides pins all of it, and the 5.1
differential compares it on the corpus, the cases and the fuzzer's
inputs. From 5.2a on, the resolver exists twice. A change to how names
resolve changes both compilers in one pull request (phase plan
decision 6).

## 2. The starting point

Measured on `main` at `1fa7cd3f`.

1. **Sizes,** counted in code lines as 5.1 counted them:
   - `nova-resolver` is 1,921 lines with 53 inline tests (the phase plan
     said 1,953). The builtin table is 385 of them, and the std tables
     about 80.
   - `nova-hir` is 810 lines with 30 tests. Only the type checker builds
     HIR (`check.rs:233-241`).
   - The driver's `program.rs` is 646 lines, about 400 of them the loader
     for loose programs and packages.
2. **The pipeline:**
   - `check_program` makes the package graph (`Program::for_file`).
   - `load_program` loads breadth first from the roots: it lexes and
     parses each module and finds each single-segment import (a sibling
     file, a dependency, or E0004, E0093 or E0001). The order of modules
     is the order of that search.
   - Any error so far stops the pipeline. `@test` functions are then
     stripped under `run`, `build` and `check`.
   - `resolve_program_for` lexes and parses the 21 native std modules
     again and appends them after the user's modules.
   - It seeds every scope with the 5 global builtins, and std's scopes
     also with the 100 std-only names (and the 13 WASM-only ones).
   - Pass 1 collects items into global `DefId`s and binds names. Pass 2
     resolves imports. Then std's exports are globbed in at the lowest
     priority, and E0102 is checked.
   - The result is `ProgramResolution`: one merged `File`, plus
     `Definitions`, diagnostics and tests.
   - The type checker resolves paths, locals and types, and builds HIR.
3. **The resolver's 11 codes,** in about 17 message forms:
   - E0001 (4 forms) and E0002 (6);
   - E0082 to E0085;
   - E0089, E0090 and E0092;
   - E0102, a warning;
   - E0900 (3 forms).

   Most messages are plain format strings with at most one secondary
   label and one note; a few carry a fix. The loader adds E0001, E0004 and
   E0093. The common E0001, "cannot find `x` in this scope", is the type
   checker's (5.3).
4. **Hash order:**
   - A glob import that clashes with local names gives its E0002s in
     `FxHash` order. That order is stable, but N cannot reproduce it.
   - No other resolver output depends on hash order. The namespaces are
     looked up by name, never iterated into output.
   - The type checker's method order (`method_locs.keys()`) is 5.3's to
     fix.
5. **The corpus:**
   - `nova check` passes on 236 of the 281 files.
   - The others are std's 25 libraries, 16 WASM-only files, three files
     with no `main`, and one file with a resolver error on purpose
     (`bytes_reserved.nova`, E0089).
   - So the errors must come from cases and the fuzzer.
   - Nine directories are packages; only `05-geometry` has a dependency.
   - `tests/runtime/modules/` is a loose program that imports sibling
     files.
6. **N today:**
   - `selfhost ast` lexes, parses and dumps all of std in 1.2 s with
     48 MB peak memory (debug runtime).
   - All 21 std ASTs held alive take about 4 MB of live heap.
   - GC pauses are 30–53 ms with the debug runtime and 13–17 ms with a
     release one. They were measured with gdb, since the runtime has no
     timer.
   - Maps keyed by strings, sets and `OrderedMap` all work, and a record
     mutated through a shared reference works.
   - One new idiom is needed: a `mut self` call on a temporary is E0060,
     so it becomes `let mut t = v.at(i); t.bind(…)`.
7. **Speed:**
   - S0 debug spends 0.065 s lexing and parsing std, and about 0.015 s in
     the resolver.
   - N with the debug runtime spends about 0.4 s on std alone. So N
     parses std once per batch, never once per program.

## 3. 5.2a, loose programs

### 3.1 S0's order of errors (a small change)

- `Exports` (or the glob step) keeps the insertion order of names in a
  `Vec` beside its map.
- The E0002s for names a glob import clashes with then come out in the
  order those names were declared.
- No other output changes. A test pins the new order. The CHANGELOG's
  Changed records it, and the ADR records why.

### 3.2 N's loader for loose programs

- `loader.nova` ports `load_program`'s loose path:
  - breadth first from the root;
  - each module lexed and parsed with N's parser;
  - each single-segment import found as a sibling file of the root's
    directory, or refused with E0093 for a std module's name;
  - the labels and the module order exactly as S0's.
- It stops as S0 stops, on any lex, parse or load error, and its
  diagnostics are part of the dump.
- `@test` functions are stripped, as S0's driver strips them under
  `check`.

### 3.3 N's resolver

- `resolver.nova` ports `nova-resolver` function by function, with the
  same names and structure, so a reviewer can read the two side by side.
  It includes:
  - the builtin table and the std tables, word for word;
  - `collect_item`, `resolve_import`, `import_std_module` and E0102;
  - the method-name mangling, since these names reach symbols in 5.5.
- `--std DIR`: N reads the 21 native std modules from `DIR`, in
  `STD_MODULES`' order, and parses them once per process.
- A `Diagnostic` record (`diagnostic.nova`) holds severity, code, file,
  span, message, secondary labels, notes and fixes, as S0's does.

### 3.4 The resolution dump, on both sides

- **S0:**
  - `nova check --dump-resolution FILE…` dumps each program's part.
  - `nova check --dump-resolution --std` dumps std's part for the native
    target.
  - Both are hidden flags: `--help` stays unchanged.
- **N:** `selfhost resolve --std DIR FILE…` and `selfhost resolve --std
  DIR --std-part`.
- **The format** is an indented S-expression like 5.1's, written down in
  full in the ADR, with one example per form:
  - the modules in order: label, std or not, and package;
  - every definition: id, name, kind, span, item index, owner, and
    variants with arity;
  - each module's three namespaces, sorted by name: name and what it
    resolves to (a definition, a variant or a builtin);
  - exports in insertion order, tests in order, the merged newline
    splits;
  - then every diagnostic: severity, code, file, span, message, secondary
    labels, notes and fixes.
- **Ids:**
  - std's definitions come after the user's in S0's numbering.
  - A program's part prints user ids as `u<n>` and std's as `s<n>`
    (std-relative).
  - std's part prints `s<n>` throughout, so it is the same for every
    native program.

### 3.5 The harness

- `selfhost.rs` gains a resolution pass: S0's and N's dumps are compared
  byte for byte, with exit codes.
- **Std's part** is compared once.
- **Every program in the corpus,** a file run alone or a loose root with
  its sibling imports, is compared. Packages are 5.2b's: the harness
  skips files under a `nova.toml` and says so.
- **The cases:** the inline sources of `nova-resolver`'s tests and of the
  type checker's tests that reach the resolver, extracted as 5.1b
  extracted the parser's. Files that need more than one module become
  small directories.
- **A name-level fuzzer** (`fuzz::names`) mutates corpus programs that
  parse. It:
  - renames a definition to collide with another, or with a builtin or a
    std name;
  - duplicates or deletes an item, and toggles `pub`;
  - adds imports of missing, private, std and sibling names;
  - adds unknown attributes and malformed `@test`s;
  - uses std-only names.

  It is weighted so that every one of the 11 codes occurs in CI's fixed
  seeds, which the harness asserts.
- CI time stays under a minute per system. N's resolver takes many files
  per process and parses std once.

### 3.6 Diagnostics compared completely

- Each diagnostic's code, severity, span, message, secondary labels,
  notes and fixes (title and edit) are compared, in order, as 5.1 did.
- The resolver's 11 codes, the loader's three, and the lexer's and
  parser's errors before them are all included.

### 3.7 Gate for 5.2a

- std's part is identical.
- Every loose corpus program's dump is identical on all three systems.
- The cases and the fixed seeds' fuzzer inputs give identical dumps,
  with all 11 codes occurring.
- S0's new error order is pinned by a test.
- The full suite, fmt, clippy, the MSRV job, `nova fmt --check` over
  `selfhost/`, and `git diff` on `Cargo.lock` are clean.

## 4. 5.2b, packages, WASM and tests

### 4.1 Packages

- N's loader gains packages:
  - a minimal `nova.toml` reader: the package's name and its path
    dependencies, the only fields the loader uses;
  - `src/` and `tests/` modules;
  - a path dependency's `src/lib.nova`;
  - E0004 (an import that is both a sibling and a dependency);
  - E0001's two notes (a dependency without `src/lib.nova`, a
    dev-dependency used from `src/`).
- The labels (`tests/stem`, `import_name/stem`) and module order match
  S0's.
- The corpus's nine packages, `05-geometry`'s dependency among them,
  join the differential.

### 4.2 The WASM target

- `--target wasm` on both sides adds:
  - the 13 WASM-only builtins in std's scopes;
  - `IMPORTED_STD`'s `ui` modules;
  - E0090 for user `extern` blocks;
  - E0092 for native-only std modules;
  - the `missing` names.
- std's WASM part is dumped once, beside the native one.
- `tests/dom` and `tests/wasm` join under `--target wasm`.

### 4.3 Test mode

- Under test mode (`nova test`'s resolution), `@test` functions are kept
  and listed as `TestFn`s, `std/test` joins, and E0083 to E0085 apply.
- The dump lists the tests, and the harness compares test mode on the
  corpus's files that hold tests.

### 4.4 The live heap (phase plan decision 8, risk 2)

- With N's resolver holding all of std's ASTs and definitions, these are
  measured locally:
  - peak memory (RSS);
  - GC pauses, with gdb breakpoints from `nova_gc_collect_roots` to the
    end of the sweep, as the research measured them;
  - with the debug runtime and with a release runtime.
- The figures are recorded in the ADR, with no threshold.
- If a pause or the peak memory looks likely to block 5.3, the user is
  asked.

### 4.5 Gate for 5.2b

- Every corpus program's dump is identical, natively and under
  `--target wasm`, packages included.
- Test mode is identical on every file with tests.
- The fuzzer also mutates packages (imports between sibling modules, a
  dependency) and WASM programs.
- The live heap is measured and recorded.
- 5.2a's checks hold.

## 5. New diagnostic codes

None. 5.2 changes the order of one class of E0002, and nothing else.

## 6. Testing

- **Unchanged rules:**
  - tests never touch the internet;
  - fixed temporary names;
  - `$NOVA_HOME` fresh where needed;
  - MSRV 1.78, no new crate, `Cargo.lock` unchanged;
  - no `clang` needed.
- **5.2a:**
  - the resolution differential (std's part, loose programs, cases,
    names fuzzer);
  - a test of S0's new E0002 order;
  - tests that the hidden flags are hidden;
  - N's own `@test`s for the loader's and the resolver's edges;
  - a check that every code occurs.
- **5.2b:**
  - packages, WASM and test mode in the differential;
  - the fuzzer's package and WASM mutations.

## 7. Crates and files

- **5.2a:**
  - `nova-resolver`: the insertion order, and a `dump` module or a
    `dump_resolution` beside `resolve_program_for`;
  - `nova-cli`: the hidden flags, and `tests/selfhost.rs` with
    `tests/selfhost/`;
  - `selfhost/`: `loader.nova`, `resolver.nova`, `builtins.nova`,
    `diagnostic.nova`, `resolution_dump.nova` and `selfhost resolve`;
  - `docs/adr/0053-*.md`.
- **5.2b:**
  - `selfhost/`: packages, the manifest reader, WASM and test mode;
  - `nova-cli` tests;
  - `docs/adr/0054-*.md`.

## 8. Records

- **ADR 0053** (5.2a):
  - the dump's format and the ids;
  - the loader's and the resolver's idiom translations;
  - std on disk;
  - S0's insertion order;
  - the fuzzer;
  - the rule that a change to resolution changes both compilers.
- **ADR 0054** (5.2b): packages, WASM, test mode, and the live heap's
  figures.
- **The phase plan,** dated in 5.2a:
  - 5.2 is the loader and the resolver, in two parts;
  - HIR moves to 5.3;
  - 5.5's loader item narrows to what 5.2 leaves.
- **Dated notes** in the master spec's Phase 5, and in the spec files
  that describe resolution (`11-PARSER.md` §4's dump list, and
  `12-TYPESYSTEM.md` if it describes the resolver's phases).
- **The CHANGELOG** in each part: Added (N's loader and resolver, the
  hidden dumps); Changed (the E0002 order).

## 9. Risks

1. **Definition ids and mangled names must be exact.** They reach
   symbols in 5.5. The dump prints them, so any difference shows.
2. **The builtin table is long:** 385 lines, ported by hand. Std's part
   of the dump lists every builtin in every scope, so a slip shows.
3. **Errors are rare in the corpus.** The cases, the name-level fuzzer,
   and the assertion that all 11 codes occur cover them.
4. **CI time:** about 240 programs are compared. N parses std once per
   process and the harness batches the programs. If CI exceeds a minute
   on a system, the user is asked.
5. **The type checker's own E0001s** ("cannot find `x`") are not in 5.2.
   They come with 5.3, and the records say so.
6. **The two compilers drift** if a change to the resolver lands in Rust
   only. The differential in every pull request enforces the rule.

## 10. Not in 5.2

- HIR, the type checker's resolution of paths, locals and types, and
  E0001 "cannot find in scope" (5.3).
- The type checker's method order from a hash map (5.3).
- Diagnostics rendered as `codespan` text in N (5.5).
- A runtime GC timer (decision 7).
- The language server's `keep_going` resolution and its index (they stay
  Rust).
