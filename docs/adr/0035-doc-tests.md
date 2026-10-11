# ADR 0035 — Doc tests

## Status

Accepted, 2026-10-11 (Phase 3.5b, branch `claude/loving-archimedes-frtf5q`;
spec `docs/superpowers/specs/2026-10-11-phase-3-5b-doc-tests-design.md`).

## Context

`nova-spec/40-TOOLING.md` §5 asks that code examples in docs be testable
with `nova test --doc`, and the phase plan's decision 9 left it to 3.5's
spec. 3.5a (ADR 0034) built the site, which renders code blocks but runs
none, and kept code blocks out of std's docs until they could be tested.

On 2026-10-11 the user decided that plain `nova test` runs doc tests too
and `nova test --doc` runs them alone, as with `cargo test`; that std gets
an example per module and a few more; that hidden lines are the only
code-block option; and that `nova init --lib` keeps refusing a directory
with a `nova.toml`, saying how to add a library.

Verifying std's examples found an older bug: a `nova test` program that
held a closure, or a function passed as a value, did not build.

## Decision

1. **Which blocks run.** The Nova code blocks in the docs a library's
   site shows: its exports, their members and documented methods, and its
   modules' `//!` docs. A program's blocks and a dependency's are shown,
   not run. std's run only under `nova test --doc --std`.
   - An info string is split at commas and whitespace, and empty words
     are dropped. A fenced block with no word or with `nova` first, and an
     indented block, run. `ignore` first, or after `nova`, is shown as Nova
     and not run. Any other first word is shown as written.
   - Any other word after `nova` is unknown: `nova test` warns, naming the
     word and the block's line, and runs the block as a plain test.
     `should_panic`, `no_run` and `compile_fail` are unknown words.
2. **Hidden lines.** In a Nova block, a line whose first non-blank
   characters are `# `, or a lone `#`, runs less that mark, and is shown
   neither on the site nor in hover. A `text` block shows it as written.
   `nova-doc`'s `hidden` is the one rule, and hover shares it through
   `hide_lines`.
3. **A block becomes a test module.**
   - Its leading `import` lines, with the blank and comment lines among
     them, go to the module's top.
   - A package's block gets `import <package>` first, unless it imports
     the package itself. std's need no import.
   - A block that declares `fn main` stays as written, and the test's
     body is `let _ = main()`, or `let _ = block_on(main())` when it is
     `async`, so what `main` returns is dropped, as a program drops it.
     Any other block is the body of the test function, so its last line
     is no value.
   - The test is `@test fn nova_doctest`. In a program of doc test
     modules only that function is collected, so a `@test` the block
     declares does not run.
4. **Names are the site's paths** and the block's first line, its
   opening fence or an indented block's first line, in a file named
   relative to the package's root: `shapes (src/lib.nova:8)`,
   `std::time (std/time/lib.nova:17)`. An impl's own docs take the path
   it is listed under. The filter matches the name.
5. **A binary of their own.** The doc tests compile into one test binary,
   after the unit tests' and apart from it, and each runs in its own
   process. `nova test` prints them as a second section, `running N doc
   tests`. Both sections run when the unit tests compile, and the closing
   error counts each section's failures. After the unit section, the doc
   build shows only the diagnostics in doc tests' code, so no warning
   shows twice; an error always shows.
6. **Diagnostics point at the doc comment:** the source file, the line
   holding the code, and its column there, a hidden line's past its `# `.
   Code a doc test adds points at the block's first line. std's name the
   module's file, `std/<module>/lib.nova`, as the doc test's name does.
7. **The driver.** `RootKind::Doc` (`Program::doc_tests`) is a module of
   the root package that imports the package and its dependencies, as a
   `tests/` module does, but no file beside it. `build_test_program_with`
   reads modules through any `Sources` and passes each diagnostic through
   a `MapDiagnostic`, which may move its labels, or hide a warning but
   never an error. Doc tests' modules are served from memory: only the
   test binary is written.
8. **The closure fix.** The test `main`'s `DefId` is past every function
   the type checker lifts, not only past resolution's definitions.
9. **`nova-lsp` depends on `nova-doc`,** for the hidden-line rule: one
   `Cargo.lock` edge between workspace crates, and no new package.

## Alternatives

- **The doc tests in the unit tests' binary.** One build fewer, but a
  broken example would stop the unit tests from running.
- **Generated files.** A doc test's module written to disk, with its
  diagnostics shown there, would point the reader at a file they never
  wrote, and leave files behind.
- **A block's items inside the test function.** Nova has no nested items
  (E0900), so a block that defines one writes `fn main`.
- **rustdoc's other options,** `should_panic`, `no_run`, `compile_fail`
  and comparing output. The user chose hidden lines alone.
- **The hidden-line rule copied into `nova-lsp`.** No new edge, but two
  rules that could drift apart.

## Consequences

- The second build costs time in a library with examples. A debug `nova
  test` in a copy of `shapes`, which has no unit test, took 0.46 s with
  the library analysed for doc tests and none found, and 0.89 s with its
  two examples (medians of seven warm runs). The prototype measured
  0.40 s before 3.5b. A package with no library is not analysed for them.
- std has 32 doc tests: an example in each of its 17 modules' `//!` docs,
  and 15 on items whose use the docs alone leave unclear, such as
  `Map::get`, `block_on`, `timeout` and `Mutex::lock`. They touch no
  network but `127.0.0.1`, and write only under `temp_dir()`.
  `std_docs.rs` asks for an example in every module, and `doc_test.rs`
  for at least 32 passing.
- The examples moved std's lines. Each commit that moved them re-pointed
  the live citations of those lines, in `nova-spec/20-STDLIB.md`, std's
  own comments, tests' comments and `docs/benchmarks/README.md`. Dated
  records keep the lines of their time.
- `examples/05-geometry`'s library has two: `shapes (src/lib.nova:8)` and
  `shapes::units::Length::plus (src/units.nova:17)`. CI's install job
  runs them with the installed `nova` (`.github/scripts/doc-gate.sh`).
- An example in a library submodule's docs can name only what
  `import <package>` brings in, the library's `lib.nova` exports, since
  module-qualified paths and nested import paths are E0900 today. So
  `Length::plus`'s example in `shapes` reaches a `Length` through
  `rectangle(...).perimeter()`.
- A plain body's trailing value is E0010 at the example's own line, and
  it names the generated function: "`nova_doctest` should return `()` but
  its body has type `Int`".
- An example may call `assert_eq`, which exists only in a test binary.
  rustdoc's examples read the same way, and std/test's module docs say
  where the assertions exist.
- 3.5b leaves out `should_panic`, `no_run`, `compile_fail` and comparing
  an example's output; doc tests of a program's items and of
  dependencies; `nova check` and the language server checking examples as
  you type; an example per public std function; `[Name]` links and a
  page per item; and PR #109's deferred minors.

## References

- `docs/superpowers/specs/2026-10-11-phase-3-5b-doc-tests-design.md`
- `docs/superpowers/plans/2026-10-11-phase-3-5b-doc-tests.md`
- ADR 0030 (package modules), ADR 0034 (the documentation generator)
- `docs/phase-3-plan.md` §3 decision 9, and §4's 3.5 entry
