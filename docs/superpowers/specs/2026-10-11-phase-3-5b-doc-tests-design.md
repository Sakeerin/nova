# Phase 3.5b, "Doc tests": design

> Status: **draft for review** (2026-10-11). The second half of Phase 3.5,
> "`nova doc`" (`docs/phase-3-plan.md` §4, the 3.5 entry and §3 decision
> 9; `nova-spec/40-TOOLING.md` §5 and §7). Phase 3.5a is merged: PR #109,
> ADR 0034. On 2026-10-11 the user made four decisions (§19, decisions 1
> to 4):
> - plain `nova test` runs doc tests too, as `cargo test` does, and
>   `nova test --doc` runs them alone;
> - std gets one tested example per module, plus examples on the few
>   functions whose use is not obvious;
> - of the code-block options, only hidden lines (`# `);
> - `nova init --lib` keeps refusing a directory that has a `nova.toml`,
>   and its message says how to add a library.
>
> Branch `claude/loving-archimedes-frtf5q`.

A code block in a library's docs is a test. `nova test` compiles each one
as a test of the package, runs it in its own process, and reports it by
the item it documents and the line it starts on:

```text
running 2 doc tests
test shapes::circle (src/lib.nova:41) ... ok
test shapes::units (src/units.nova:3) ... ok

test result: ok. 2 passed; 0 failed; 0 trapped; 2 total
```

A compile error in one points at the doc comment's own line. std's
modules each get an example that runs in CI, and so does the example
library `shapes`.

## 1. What 3.5b delivers

- **Doc tests** (§4 to §7): the Nova code blocks in the docs that a
  library's site shows, each compiled and run as a test.
- **`nova test`** runs them after the unit and integration tests;
  **`nova test --doc`** runs them alone, and **`nova test --doc --std`**
  runs std's (§6).
- **Diagnostics point at the docs** (§7).
- **Hidden lines:** a `# ` line runs but is not shown, on the site or in
  hover (§5.3, §8).
- **std's examples:** one per module, in its `//!` docs, and a few more
  (§9).
- **`examples/05-geometry`'s library** gets examples, and CI runs them
  with the installed `nova` (§10).
- **The follow-up:** `nova init --lib`'s message in a directory that has
  a `nova.toml` (§3).

## 2. The starting point (read on 2026-10-11)

### `nova test`

- `crates/nova-cli/src/cmd/test.rs` builds one test binary
  (`nova_driver::build_test_program`) and runs each `@test` function in
  its own process, chosen by `NOVA_TEST_INDEX`.
- A process ends one of three ways: passed, panicked (stderr holds
  `nova: panic:`), or trapped. `@test(should_panic)` inverts only the
  panicked case.
- Its one argument is a filter, a substring of a test's name. A filter
  that matches nothing is an error; an unfiltered run with no tests is
  not.
- It prints `running N tests`, a line per test, a blank line, and
  `test result: ok. P passed; F failed; T trapped; N total` (or
  `FAILED`).
- Under `Roots::Test` it loads `src/main.nova`, `src/lib.nova` and every
  top-level `tests/*.nova`, and collects the root package's tests. A
  dependency's tests are never run (40-TOOLING §7's 3.3a note).
- `std/test`'s `assert`, `assert_eq` and `assert_ne` exist only in a test
  binary.

### Imports and std

- A test module of the root package sees the root package under its own
  import name (3.3a spec §4.3). `import shapes` brings the library's
  exports into scope by their names, as `examples/05-geometry/src/main.nova`
  uses `Shape` and `circle`.
- Every module sees std's names without an import.
- An item inside a function body is E0900, "items nested inside function
  bodies are not supported yet" (probed).
- `#` is not a Nova character: a line starting with it is L0001,
  "unexpected character `#`" (probed). So `# ` can mark a hidden line
  without clashing with code.
- `block_on(work())` runs an `async fn` from a test body, and an
  `async fn main` runs under `block_on` (both probed).

### Docs

- 3.5a's site (`crates/nova-doc`) documents a library's exports and the
  `//!` docs of its modules (3.5a spec §5). It renders Markdown with
  `pulldown-cmark`, and treats as Nova a fenced block with no language or
  `nova`, and an indented block (3.5a spec §9.3), splitting the info
  string at commas and whitespace and reading its first word.
- std has docs for every public item and no code block, held back until
  3.5b tests them (3.5a decision 24).
- Hover (`crates/nova-lsp/src/hover.rs`) shows the `///` lines above a
  declaration as Markdown, as written.

### The driver

- `FrontendContext` reads every module from disk (`DiskSources`) and
  checks that the entry file opens. `program::load_program` already takes
  any `Sources`, as the language server's and the sweep's do.
- A root's kind, `Src`, `Tests` or `Loose`, decides where its module sits
  and what it may import (`Loader::root_place`).
- `build_test_program` renames every `main` and adds a dispatching one,
  so a test binary may hold modules that each define `main`.

## 3. The follow-up: `nova init --lib`

- `nova init` still refuses a directory that has a `nova.toml` (3.3a
  decision 14, settled by the user on 2026-10-11).
- With `--lib`, when the package has no `src/lib.nova`, the message
  becomes: "this directory already has a nova.toml; to add a library to
  its package, write src/lib.nova".
- Otherwise the message is unchanged.

## 4. Which code blocks are tests

### 4.1 Which docs

- **A library's.** The doc tests of a package are the code blocks in the
  docs that its library's site shows (3.5a spec §5): its exports' `///`
  docs, their members' and documented methods', and the `//!` docs of the
  library's modules.
- **Not a program's.** A package with no `src/lib.nova` has no doc tests,
  as `rustdoc` tests a library's docs only. Its code blocks are shown, not
  run.
- **Not a dependency's,** as a dependency's tests are never run.
- **std's** run only under `nova test --doc --std` (§6.1).

### 4.2 Which blocks

A block's info string is split at commas and whitespace into words.

| Block | Shown as | Tested |
|---|---|---|
| fenced, no words | Nova | yes |
| fenced, first word `nova` | Nova | yes, unless a word is `ignore` |
| fenced, first word `ignore` | Nova | no |
| fenced, any other first word (`text`, `sh`, …) | written | no |
| indented | Nova | yes |

- A word after `nova` other than `ignore` is unknown. `nova test` warns,
  naming the word and the block's line, and runs the block as a plain
  test. `should_panic`, `no_run` and `compile_fail` are unknown words in
  3.5b (decision 3).
- `ignore` blocks are highlighted as Nova on the site, which is the one
  change to 3.5a's rule.

### 4.3 Hidden lines

- In a Nova block, a line whose first non-blank characters are `# `, or
  that is `#` alone after its indentation, is **hidden**.
- A hidden line runs, less its `# ` (or as an empty line).
- It is not shown on the site (§8.1) or in hover (§8.2).
- Only Nova blocks hide lines. A `# ` line in a `text` or `sh` block is
  shown as written.

## 5. From a block to a test

Each doc test is a module of its own, compiled into one test binary with
the others (§6.2).

### 5.1 Its imports

- **Leading imports are hoisted.** The `import` lines at the top of the
  block (blank and comment lines between them allowed) go to the module's
  top.
- **The package is imported for it.** A package's doc test gets `import
  <package>` before them, unless one of its lines imports the package
  itself. So an example of `circle` needs no import line, and an example
  that writes `import shapes::{circle}` keeps exactly that.
- **std needs none.** std's names are in every module.

### 5.2 Its body

- **A block that declares `fn main`** is a module as written: its items,
  and its `main`. The test calls `main`, and when `main` is `async`, runs
  it under `block_on`, as a program would.
- **Any other block** is the body of the test function: its lines, after
  the hoisted imports, in order.
- So a block that defines an item writes its own `fn main`, since an item
  inside a function body is E0900. §7's mapping makes that error point at
  the item in the doc comment.

### 5.3 What it is called

- A doc test is named by the item it documents and the block's first
  line in the source: `shapes::circle (src/lib.nova:41)`.
  - A member: `shapes::Rect::width (src/lib.nova:12)`.
  - A method: `shapes::Length::plus (src/units.nova:15)`.
  - A module's `//!` docs: the module's path, `shapes::units
    (src/units.nova:3)`, or the package, `shapes (src/lib.nova:1)`.
  - std's: `std::time (std/time/lib.nova:3)`.
- The path in parentheses is relative to the package's root, with `/`.
- The filter matches against this name.

## 6. Running them

### 6.1 The command

- **`nova test [filter]`** runs the unit and integration tests, as today,
  and then the doc tests, when the root package has a library and its
  docs hold at least one doc test. Otherwise its output is unchanged.
- **`nova test --doc [filter]`** runs the doc tests alone.
  - Outside a project it fails: "nova test --doc tests a package's
    library; run it inside one, or use `nova test --doc --std`".
  - In a package with no library it fails: "nova test --doc tests a
    library's docs, and this package has no src/lib.nova".
  - A library with no doc tests prints `running 0 doc tests` and passes,
    as an unfiltered run with no tests does today.
- **`nova test --doc --std [filter]`** runs std's doc tests, inside a
  project or not. `--std` needs `--doc`.
- A filter that matches nothing, across both kinds, is an error, as
  today.

### 6.2 One binary for the doc tests

- The doc tests are compiled into their own test binary, after the unit
  tests' and apart from it, as `cargo test` builds doc tests apart.
- So a broken example does not stop the unit tests from running, and the
  binary holds only doc tests.
- Each doc test runs in its own process and is classified as today.
- The library's own `@test` functions are not run again there.

### 6.3 Output

- The doc tests print as their own section after the unit tests' result
  line and a blank line: `running N doc tests`, a line per test by its
  name (§5.3), a blank line, and a `test result:` line in today's form.
- An unknown word's warning (§4.2) prints before the section.
- **Both sections run** when the unit and integration tests compile,
  even if some fail, so one run reports everything. A compile error in
  either binary stops the run there.
- **The exit status** fails when any test in either section failed or
  trapped. The closing error counts each section's failures.

## 7. Diagnostics point at the docs

- A diagnostic in a doc test's code points at the doc comment: the source
  file, the doc line holding that code, and its column there.
  - A hidden line's column accounts for its `# `.
  - A diagnostic in code the doc test adds (the package import, the test
    function's wrapper) points at the block's opening fence, or at its
    first line for an indented block.
- So E0001 in an example of `circle` reads `src/lib.nova:44:9`, and the
  snippet under it is the `///` line.
- A warning is mapped the same way.
- A runtime panic prints as today: its message has no position.

## 8. The site and hover

### 8.1 The site

- Hidden lines are left out of a Nova block on the page.
- `ignore` blocks are highlighted as Nova (§4.2).
- Nothing else in 3.5a's rendering changes.

### 8.2 Hover

- Hover leaves hidden lines out of Nova blocks, as the site does, so an
  editor shows the example a reader sees on the page.

## 9. std's examples

### 9.1 What is written

- **One example per module,** in its `//!` docs, introduced by a
  sentence: typical use of the module's main types and functions, checked
  with `assert_eq` or `assert`.
- **A few more,** on functions whose use the docs alone leave unclear,
  such as `timeout`, `block_on` or `Map::get`'s `Option`. The plan names
  each, at most about fifteen.
- Every example:
  - runs in well under a second;
  - touches no network but `127.0.0.1`, and no IPv6;
  - writes files only under `temp_dir()`, in a directory named for its
    module, and removes them;
  - reads no environment variable it did not set, and no input;
  - says what it shows, so a reader learns from it, not only CI.
- Where a module's typical use cannot run safely (`std/http`'s `listen`
  serves until a signal; `std/process`'s `exit` ends the test), the
  example shows what can, such as building a `Response` or reading
  `args()`. A block is `ignore`d only with a sentence saying why, and the
  plan aims for none.
- 3.5a's rules for std's docs still hold (3.5a spec §12.1): a doc says
  what the code does, and never promises what a maintainer comment calls
  unspecified.

### 9.2 Keeping them

- A test runs `nova test --doc --std` and expects every one to pass.
- 3.5a's completeness check gains one condition: each of std's 17
  modules has at least one doc test in its `//!` docs.

## 10. The example: `examples/05-geometry`

- The `shapes` library gets doc tests: one in its `//!` docs, measuring a
  circle and a rectangle, and one on an item whose use is worth showing.
- `nova test` inside `examples/05-geometry/shapes` runs them.
- The README says so.
- CI's `install` job runs `nova test --doc` with the installed `nova` in
  a copy of `shapes` (extending `.github/scripts/doc-gate.sh`).

## 11. Crates and code

No new crate and no new package in `Cargo.lock`.

### 11.1 `nova-doc`

- A new module, `doctest.rs`: a package's doc tests, or std's, as test
  modules.
  - Each has its name (§5.3), its module text (§5), and a map from each
    of its lines to a source line and column (§7).
  - It reads the same documented items as the site, from the same
    analysis, and finds code blocks with `pulldown-cmark`'s offsets.
- `markdown.rs` hides hidden lines and highlights `ignore` blocks.
- A small public function, the one rule for hidden lines and block
  classification, which hover shares.

### 11.2 `nova-driver`

- `build_test_program` can take its modules from a `Sources` other than
  the disk, so doc tests need no files written.
- A root can be a **doc test module** of the root package: it imports as
  a test module does (the package by name, its dependencies and
  dev-dependencies), and only doc test modules' tests are collected from
  a program whose roots are doc tests.
- A diagnostic can be mapped before it is rendered (§7).

### 11.3 `nova-cli`

- `cmd/test.rs` gains `--doc` and `--std`, the second section and its
  output, and shares its run loop between the two sections.
- `cmd/new.rs`'s `init` gains §3's message.

### 11.4 `nova-lsp`

- Hover hides hidden lines through `nova-doc`'s rule (§8.2).
- This adds `nova-doc` to `nova-lsp`'s dependencies: one new edge in
  `Cargo.lock`, between two workspace crates, and no new package
  (decision 13).

## 12. Testing

### 12.1 Unit tests, in `nova-doc`

- Block classification, for every row of §4.2's table and an unknown
  word.
- Hidden lines: hidden on the page, run less their `# `, and kept in a
  `text` block.
- A block becomes a module: hoisted imports; the package import added
  and not doubled; a block with `fn main`; one with `async fn main`; a
  plain body.
- Names, for an item, a member, a method, a module and the package.
- The line map: a body line, a hidden line, and an added line.
- Only a library's documented items give doc tests: a private item's
  block, and a program's, give none.

### 12.2 The driver

- A test binary built from `Sources` that are not on disk.
- A doc test module imports the package by name.
- Only doc test modules' tests are collected.
- A mapped diagnostic renders at its source line.

### 12.3 The command line (`crates/nova-cli/tests/doc_test.rs`)

Each in a fresh fixed directory, with `NOVA_HOME` set to a fresh one:
- `nova test` in a library with doc tests: both sections, and their
  names.
- A failing example: `FAILED`, its name, and a failing exit.
- A compile error in an example: the diagnostic names the doc line.
- An unknown word: the warning, and the block run.
- `--doc` alone; outside a project; in a program-only package; with no
  doc tests.
- A filter matching one doc test, and one matching nothing.
- `nova test` in a library with no code blocks: output exactly as before.
- `nova init --lib` in a package with and without a library (§3).

### 12.4 The gate tests

- `nova test --doc --std` passes, and runs at least one doc test per std
  module.
- `nova test` inside a copy of `examples/05-geometry/shapes` runs its doc
  tests and passes.
- A hover test: an example's hidden line is not in hover's text.

### 12.5 Mutants

Each, run on committed work and undone with `git checkout`, must fail
the test named:
1. Hidden lines shown on the page → the hidden-line unit test.
2. Hidden lines run with their `# ` → the hidden-line unit test and the
   CLI's passing run.
3. The package import always added → the "not doubled" unit test.
4. `ignore` blocks tested → the classification unit test.
5. A program's blocks tested → the "only a library" unit test.
6. The line map off by one line → the CLI's compile-error test.
7. The library's own tests collected in the doc binary → the driver's
   collection test.
8. A failing doc test reported as passing → the CLI's failing-example
   test.
9. One std module's example deleted → the completeness check.

## 13. The gate

The phase plan gives 3.5 one gate, which 3.5a met for the site. 3.5b's
part:

- std's 17 modules each have a doc test, and `nova test --doc --std`
  passes in CI's Test job on Linux, Windows and macOS.
- `shapes`' doc tests pass with the installed `nova` in CI's install job.
- A broken example fails `nova test`, naming its item and line, and a
  compile error in one points at its doc line.
- Each of §12.5's mutants fails its named test.

## 14. Records

- **ADR 0035, "Doc tests":** which blocks run, how a block becomes a
  test, the separate binary, the mapped diagnostics, hidden lines, and
  what 3.5b leaves out.
- Dated notes in:
  - `nova-spec/40-TOOLING.md` §5 and §7;
  - `docs/phase-3-plan.md`'s 3.5 entry: 3.5 is complete;
  - `nova-spec/00-MASTER-SPEC.md` §6, if a crate's role changes;
  - `agent.md` §5;
  - the 3.3a spec's decision 14, settled.
- CHANGELOG `[Unreleased]`, the README's Documentation section, and
  ARCHITECTURE.

## 15. Risks

- **The line map is wrong somewhere,** and an error points at the wrong
  line. Mitigation: unit tests for each kind of line, the compile-error
  CLI test, and mutant 6.
- **std's examples are slow or flaky on one system.** Mitigation: §9.1's
  rules, one process per example, and CI on all three systems before
  merge.
- **An example teaches `assert_eq`,** which exists only in a test binary.
  This is how `rustdoc` examples read too; std/test's module docs say
  where the assertions exist.
- **A second compile per `nova test`** costs time in a library with
  examples. It is skipped where there are none, and its time is measured
  in the plan.
- **Mapping a diagnostic** needs the source file in the program's file
  database. It is there, since a doc test imports the library that holds
  it, and std's modules are added to every program's database (as
  `<std/time>` and so on).

## 16. Crates touched

| Crate | Change |
|---|---|
| `nova-doc` | `doctest.rs`; hidden lines and `ignore` in `markdown.rs`; the shared rule |
| `nova-driver` | modules from `Sources`; doc test roots; collection; mapped diagnostics |
| `nova-cli` | `nova test --doc [--std]` and the second section; `init`'s message |
| `nova-lsp` | hover hides hidden lines; depends on `nova-doc` |
| std | one example per module, and a few more |
| `examples/05-geometry` | `shapes`' examples; README |
| CI | `doc-gate.sh` runs `shapes`' doc tests |

## 17. Not in 3.5b

- `should_panic`, `no_run` and `compile_fail` on code blocks
  (decision 3), and comparing an example's printed output.
- Doc tests of a program's items, and of dependencies.
- `nova check` or the language server checking examples as you type.
- An example per public std function (decision 2 chose one per module).
- `[Name]` links, a page per item, and the rest of 3.5a's §21.
- PR #109's deferred minors: module names that are Windows device names,
  `explorer` with a comma in the path, stale line citations into other
  crates, and the behaviours found while documenting std. Each can be
  pulled in by the user's word.

## 18. Ordering

1. §3's follow-up.
2. The rule for blocks and hidden lines, in `nova-doc`, with the site's
   rendering and hover.
3. The driver's `Sources`, doc test roots, collection and mapping.
4. `doctest.rs`.
5. `nova test --doc` and the second section.
6. `--std`, std's examples, module by module, and the completeness
   condition.
7. `shapes`' examples and CI.
8. Records.

## 19. Decisions

By the user, 2026-10-11:
1. **`nova test` runs doc tests too;** `--doc` runs them alone, as with
   `cargo test`.
2. **std gets one example per module,** plus a few on functions whose use
   is not obvious.
3. **Of the code-block options, only hidden lines.** `should_panic`,
   `no_run` and `compile_fail` are left out.
4. **`nova init --lib` keeps refusing** a directory with a `nova.toml`,
   and says how to add a library (3.3a decision 14).

Made in this spec, for review:

5. **A library's documented items only** are tested, as the site shows
   them and as `rustdoc` tests a library (§4.1).
6. **`ignore` blocks are Nova** on the site (§4.2).
7. **An unknown word warns** and the block runs as a plain test (§4.2).
8. **The package is imported for a doc test** unless it imports it
   itself (§5.1).
9. **A block that defines items writes `fn main`** (§5.2), since Nova has
   no nested items.
10. **Doc tests have their own binary** (§6.2), so a broken example does
    not hide the unit tests' results.
11. **Both sections run** when both compile (§6.3).
12. **Diagnostics are mapped to the doc comment** (§7), not shown in a
    generated file.
13. **`nova-lsp` depends on `nova-doc`** for the hidden-line rule (§11.4):
    one `Cargo.lock` edge between workspace crates, no new package, for
    the user's approval with this spec.
14. **`nova test --doc --std`** runs std's examples (§6.1), so CI and std's
    contributors share one command.
15. **The completeness check asks for an example per std module** (§9.2).
16. **No files are written** for doc tests; their modules come from
    `Sources` (§11.2).
