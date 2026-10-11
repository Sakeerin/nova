# Phase 3.5b, "Doc tests", Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** a code block in a library's docs is a test. `nova test` runs the
library's doc tests after its unit and integration tests, `nova test
--doc` runs them alone, and `nova test --doc --std` runs std's. A compile
error in an example points at the doc comment's own line. Hidden `# `
lines run but are not shown. std gets one tested example per module and
15 more; the example library `shapes` gets two, run in CI with the
installed `nova`. A pre-existing bug that stopped any test program holding
a closure from building is fixed first.

**Architecture:**
- **`nova-driver`:** the test `main`'s `DefId` clears every function's
  (the closure fix); `build_test_program_with(program, sources, map)`
  reads modules through any `Sources` and passes each diagnostic through a
  `MapDiagnostic`; `RootKind::Doc` roots import as a `tests/` module does,
  with no siblings, and a program with them collects only their
  `nova_doctest`; `Program::doc_tests`.
- **`nova-doc`:** `markdown.rs` gains the code-block rule (`Block`,
  `classify`, `hidden`), `code_blocks` over `pulldown-cmark`'s offsets,
  `hide_lines`, and `DocText`, docs that keep their place in the source;
  `doctest.rs` turns the blocks the site shows into test modules
  (`DocTest`), with a line map back to the source.
- **`nova-lsp`:** hover leaves out hidden lines through `nova-doc`.
- **`nova-cli`:** `nova test` gains its doc section, `--doc` and `--std`;
  `nova init --lib`'s message in a package.
- **std:** an example in every module's `//!` docs, and 15 on single
  items.
- **`examples/05-geometry/shapes`:** two doc tests; `doc-gate.sh` runs them.

**Tech Stack:**
- Rust: MSRV 1.78, edition 2021.
- No new crate. `Cargo.lock` changes by one line, in Task 4: `nova-lsp`
  gains `nova-doc` in its dependency list, approved by the user with the
  spec on 2026-10-11 (spec decision 13).
- Tests: unit tests in `nova-doc`, `nova-driver` and `nova-lsp`;
  `crates/nova-driver/tests/doc_tests.rs`; `nova` driven through
  `assert_cmd` in `crates/nova-cli/tests/doc_test.rs`, `run_tests.rs`,
  `project.rs` and `lsp_navigation.rs`.

**Spec:** `docs/superpowers/specs/2026-10-11-phase-3-5b-doc-tests-design.md`,
approved by the user on 2026-10-11 and amended while planning (its header
says how).
- Read it before Task 1: it is the authority this plan argues from.
- Its §19 lists the decisions, and its §2 the code as it stood.

**Checked on 2026-10-11, before execution:**
- **A prototype** built every task in a scratch worktree on a detached
  `HEAD` at `9d953d5`, test-first, and the code blocks below were copied
  from its commits by script. For each task, a second script applied the
  task's blocks to the files before it and got the task's commit back
  exactly. The std examples of Tasks 11 to 14 were then dry-run as written:
  all 32 pass as doc tests, and the plan's commands, expected messages and
  layout were corrected where they differed.
- **On the prototype's final state,** the full suite gave 1989 passed, 1
  failed (the environmental IPv6 test) and 10 ignored, against the
  baseline's 1942, 1 and 10. Clippy with `-D warnings`, the 1.78 check and
  both formatters were clean. The spec's mutants 1 to 8 each failed their
  named tests, and each std task's failing Step 1 is mutant 9 in effect.
- **Two faults were found and are fixed here:** Task 2's tests compared
  stderr exactly while CI sets `RUST_BACKTRACE` (they now turn it off), and
  Task 8 is written against Task 6's `DOC_TEST_FUNCTION` from the start.
- **Measured** (debug build, the median of seven warm runs of plain `nova
  test` in a copy of `shapes`, which has no unit test): 0.40 s before this
  plan, 0.48 s with a library analysed for doc tests and none found, 0.88 s
  with Task 15's two examples. `nova test --doc --std` runs std's 32 in
  about 1.2 s.
- FACTCHECK-PLACEHOLDER

## Global Constraints

- **The workspace's minimum Rust stays 1.78,** edition 2021. CI's MSRV job
  runs `cargo check --locked --workspace` with `RUSTFLAGS=-D warnings`.
  No `Option::is_none_or` (1.82), no `std::path::absolute` (1.79).
  `cargo +1.78 check -p nova-cli --all-targets` cannot run here: cargo
  1.78 cannot parse `assert_cmd` 2.2.1's manifest (3.5a's Task 14 ruling).
- **`Cargo.lock` changes once, in Task 4,** by exactly the one line above.
  Any other change to it stops the task: ask the user.
- **No `unwrap()` outside `#[cfg(test)]` and test files:** `expect("why")`
  or `?` (agent.md §4).
- **Tests never touch the internet.** Every CLI test sets `NOVA_HOME` to a
  fresh directory. Every test directory has a fixed name under the
  system's temporary directory, removed and made again at the test's
  start. std's examples use only `127.0.0.1`, and write only under
  `temp_dir()`, in a directory named for their module.
- **Doc tests write no file.** Their modules are served from memory (spec
  §11.2); only the test binary is written, where `nova test` writes its
  binaries.
- **The driver's linking tests** find the runtime library beside the test
  executable, as `run_tests` does. CI's `cargo build --locked --workspace`
  puts it there before `cargo test`; run `cargo build -p nova-cli` first
  on a clean target.
- **Nothing is published, and no tag is pushed.** Merge by rebase, on the
  user's word only. Never enable auto-merge.
- **This machine is Linux.** Windows and macOS results come from CI, which
  runs all three systems.

## Conventions

As the 3.5a plan's: each task is test-first (the failing test, its run and
the expected failure, the code, its run and the expected pass), copies the
code blocks below by line range rather than retyping them, runs `cargo
fmt --all`, clippy with `-D warnings` and the 1.78 check on what it
touched, and ends with one commit whose message ends with the session's
attribution lines. A test written after its code on purpose is marked
*guard* in its comment. A difference from an `Expected:` is fixed in the
smallest way and recorded in the ledger as a ruling, with its cost if
wrong.

## Review Focus

The inputs most likely to bite a user, most likely first. Each has its
test in the task that owns the code:

1. **A compile error in an example,** which must point at the `///` line
   and column, not a generated file → the CLI's compile-error test
   (Task 9), and mutant 6.
2. **A program with a closure,** which `nova test` could not build before
   Task 1 → Task 1's regression test.
3. **An example that imports the package itself,** which must not get a
   second import (E0002) → Task 8's module tests, and mutant 3.
4. **A library whose own tests must not run twice,** once per binary →
   Task 6's collection test, and mutant 7.
5. **A hidden line,** which runs but is shown neither on the site nor in
   hover → Tasks 3 and 4, and mutants 1 and 2.
6. **A library on Windows with an empty root,** where the library's file
   name must agree between `nova-doc`'s analysis and the doc test program
   (decision 7) → the CLI's compile-error test, run from the package
   directory on all three systems in CI.

## Decisions: where this plan settles what the spec leaves open

1. The test `main`'s `DefId` is one past the largest function `DefId` after type checking, and at least `defs().len()`, computed in the driver: the smallest fix, and `nova-typeck`'s `CheckResult` keeps its shape.
2. Task 1's regression test is in `nova-cli`'s `run_tests.rs`: it needs a linked binary and `nova test`'s report.
3. The driver's linking tests stay in `crates/nova-driver/tests/doc_tests.rs`: they find the runtime library beside the test executable as `run_tests` does through `nova` (`target/debug/`), which CI's `cargo build --locked --workspace` puts there before `cargo test`. On a clean target, a bare `cargo test -p nova-driver` fails the three that link, as `run_tests` would.
4. A package's doc test program lists `src/lib.nova` as a root **after** the doc roots, not first: the loader then names the library `under(root, "src/lib.nova")`, as `nova doc`'s analysis (`Roots::Check`) names it, where an import's spelling differs on Windows under an empty root; and the first doc root stays the entry, so the binary is `doc0`, fingerprinted apart from a library-only package's unit test binary, which a library entry would have overwritten.
5. `Program::doc_tests(Some(..), vec![])` has no root, so a build of it fails "nothing to compile" rather than build the library's own tests under the doc binary's name.
6. A doc module counts as a test file, so its test is named `<file stem>::nova_doctest`; in a program with a doc root only doc modules' `@test fn nova_doctest` are kept (spec §5.2, amended), even if `Src` roots are present.
7. Std's doc tests (`Program::doc_tests(None, ..)`) import nothing: every `import` is E0001, and std's names come from the resolver's prelude, `std/test`'s under `nova test`.
8. `MapDiagnostic` takes `&mut FileDb`, so a map can add a file (std's `std/time/lib.nova`, absent while load diagnostics render), and returns `bool`: `false` hides a warning, never an error.
9. That a hidden warning is neither shown nor counted is tested inside the driver, on `FrontendContext`'s counts: a rendered diagnostic goes to the process's stderr, which an integration test cannot read.
10. `code_blocks`, `CodeBlock` and `CodeLine` are public and re-exported, as are `classify`, `hidden`, `hide_lines` and `Block`. Reason: std's completeness check (§9.2) can then count the blocks in module docs by the parser alone (3.5a plan decision 17), and their fields are not dead code before Task 8 reads them.
11. A lone `#` runs as its indentation, which is an empty line in effect. Reason: §4.3 says "as an empty line", indentation compiles as nothing, and `hidden` stays one rule: where the mark starts, and how long it is.
12. An `ignore`d block warns of nothing, an unknown word included. Reason: §4.2's warning is for a block run "as a plain test", and `ignore` stops the block running.
13. In `nova,nova`, the second `nova` is an unknown word. Reason: §4.2 read literally: any word after the first that is not `ignore` is unknown.
14. A doc test's name uses the package's manifest name, as the site's paths do (`my-shapes::circle`), and its added import uses the import name (`import my_shapes`). Reason: §5.3 names a test by the site's path, and an import must spell the import name.
15. An impl's own docs are named by the path the impl is listed under. That is `shapes::Rect` with its type, `{module qual}::{type}` among a module's other impls, and `std::Int` on a primitive's page. Reason: amended §5.3, and the search index's paths.
16. A block in a trait impl method's docs is neither shown nor tested, and nothing reports it. Reason: 3.5a decision 28 leaves trait impl methods out of the site, and §4.1 tests what the site shows.
17. The line in a doc test's name is the opening fence's, or an indented block's first line. Reason: this is §5.3's "the block's first line in the source", and §7 maps added lines there too, so a name and a diagnostic in added code agree.
18. An empty Nova block is a doc test whose body is empty, and it passes. Reason: §4.2 exempts no block for its content.
19. Doc tests come in the site's order. The root module comes first, then the other modules by name. In each module: its `//!` docs, then its items in source order, each followed by its members and its impls, then the module's other impls. std's primitives' impls come last. Reason: the order is deterministic, and it is the order a reader meets them.
20. `DocTest.file` is the `FileDb` name nova-doc's own analysis gave the file: `Path::display` of the path the loader read, or `<std/x>` for std. The doc test program must therefore be built from the same `root` spelling, with `src/lib.nova` as one of its roots: Task 6's `Program::doc_tests(Some(root), paths)` does both. Reason: only a root's path is spelled by `under(root, "src/lib.nova")`. A library reached through an import is `module_dir(..).join("lib.nova")`, whose separators differ on Windows when `root` is empty. Task 8's Interfaces give the whole contract.
21. `package_doc_tests` runs its own front-end analysis: index on, no MIR, no std analysis. Reason: it reads exactly what the site reads (§11.1). It costs one analysis per `nova test` in a library, which the CLI part measures.
22. Task 7 marks `DocText::source` `#[allow(dead_code)]`, with a comment, and Task 8 removes those two lines. Reason: until doc tests read it, the library has no caller of it. Task 7's new fields are not flagged, on stable or on 1.78.
23. `library_package` runs no `clash` check. Reason: doc tests write no pages, and `nova doc` still runs the check.
24. `document_std` builds std's package through `std_package()`. Reason: one place builds it.
25. The doc section's blank line and `running N doc tests` print after its build succeeds, so a compile error in an example leaves stdout ending in the unit section's result line. Reason: §6.3, "a compile error in either binary stops the run there".
26. A filter that selects only doc tests still prints the unit section, as `running 0 tests` and its result line; one that selects only unit tests prints `running 0 doc tests` and builds nothing. Reason: both sections run (§6.3), and the driver is never given no doc test (Task 6's contract).
27. The doc binary holds every doc test, even under a filter. Reason: as the unit binary does; the first module names it (`doc0`), so a filter never builds it under another name.
28. A filter that matches nothing names what was available: `(1 test and 3 doc tests available)`, `(3 doc tests available)` under `--doc`, and today's `(1 test available)` when there is no doc section. Reason: §6.1, "as today"; the unit-only message is unchanged.
29. The closing error counts each section that ran, joined by `; `: `0 tests failed, 0 trapped; 1 doc test failed, 0 trapped`, or `1 doc test failed, 0 trapped` under `--doc`; with no doc section it is today's. Reason: §6.3, "the closing error counts each section's failures".
30. An unknown word's warning is `warning: src/lib.nova:11: unknown word `should_panic` in a code block's info string; the block runs as a plain test`, one per block (`unknown words `a`, `b``), printed for the doc tests a run selects, before the build. Reason: §4.2 names the word and the line; `nova doc`'s broken-link warning puts the place first.
31. Plain `nova test` looks for doc tests only in a project whose root has `src/lib.nova`, after the unit build; `--doc` checks for `src/lib.nova` before syncing. Reason: §6.1's two messages, and no sync for a command that is refused.
32. A package's doc test modules are `<root>/<doc tests>/doc<N>.nova` and std's `<std doc tests>/doc<N>.nova`, relative to the current directory. Reason: `<` and `>` cannot be in a Windows file name and are unlikely on Unix; the stem `doc0`, which names the binary, is portable; the driver fingerprints a path no file holds from the current directory.
33. After the unit section, the doc build hides every warning with no label in a doc test's module, the manifest's included, since the doc build reads the graph again; errors always show (the driver forces it). Under `--doc` or `--std` nothing is hidden. Reason: §6.3 as amended.
34. When `package_doc_tests` fails (the library has errors, which only `--doc` reaches, since otherwise the unit build fails first), its diagnostics render and the error is the driver's wording, "could not compile due to N previous errors". Reason: one wording for a failed compile.
35. Std's module text comes from a new `nova_driver::std_source(name)`, Task 10. Reason: a std doc test's lex, parse and import errors render before the build adds `<std/x>` to the database, and nova-cli cannot read `nova_resolver::STD_MODULES` without a new `Cargo.lock` edge.
36. The CLI test of `--doc --std` asks only that every std doc test passes, whatever their number. Reason: std has none until Tasks 11 to 14 add them, which then need no edit here; std's gate test asks for one per module.
37. The CLI tests set `RUST_BACKTRACE=0` and `NO_COLOR=1`. Reason: CI sets `RUST_BACKTRACE=1` for every job, and anyhow then adds a backtrace to `Error: ...`, which an exact stderr comparison would see.
38. The library's `//!` example follows the module docs' two paragraphs, introduced by a sentence, and measures `circle(1.0)` and `rectangle(2.0, 3.0)` alone and together: `shapes[1].area()` is `6.0` and `total_area(shapes)` is `PI + 6.0`. Reason: spec §10, "measuring a circle and a rectangle"; it uses what `src/main.nova` uses.
39. The floats compare exactly with `assert_eq`. Reason: `total_area` adds `0.0 + PI * 1.0 * 1.0`, which is `PI`, then `6.0`, which is `PI + 6.0` bit for bit; checked by the passing test.
40. The item worth an example is `Length::plus`, whose example gets a `Length` from `rectangle(1.0, 1.0).perimeter()`, 4, and lays it end to end with itself: `.value` is `8.0`. Reason: a dependent, and so a doc test, cannot name `Length` (ADR 0030), but can use one it is given; its name, `shapes::units::Length::plus (src/units.nova:17)`, is the amended §5.3's method path.
41. `Length::plus`'s one-line summary stays its first paragraph, and the example follows a sentence of its own. Reason: the site's summary of an item's docs is their first paragraph, which reads as before.
42. `doc-gate.sh` runs `nova test --doc` in the copy's `shapes`, after its `nova doc` checks, and requires exit 0 and the last line `test result: ok. 2 passed; 0 failed; 0 trapped; 2 total`, printing what nova printed when either fails. Reason: spec §10 names `--doc`; the exact line pins the count, so a doc test lost by a later change fails CI.
43. CI's step keeps its `run:` line and gains "and runs its doc tests" in its name, with the 3.5b gate cited above it. Reason: the call is unchanged, and the name says what the step checks.
44. The gate test runs plain `nova test` in a fresh copy of `shapes`, and compares all of stdout: the empty unit section, then both doc tests by name. Reason: §12.4, and §10's "`nova test` inside `examples/05-geometry/shapes` runs them".

## Contracts between tasks

What the driver (Tasks 5 and 6) and the command line (Tasks 9 and 10) give the tasks after them.

- `nova_driver::build_test_program_with(program: Program, sources: &dyn
  Sources, map: Option<&MapDiagnostic>) -> Result<(PathBuf, Vec<TestFn>)>`
  and `pub type MapDiagnostic<'a> = dyn Fn(&mut FileDb, &mut Diagnostic) ->
  bool + 'a;`.
  - `map` sees every diagnostic of the build, after a registry package's
    warnings are dropped, and before it is counted and rendered. It may move
    labels into other files: `db.id_of(name).unwrap_or_else(|| db.add(name,
    text))`. Keep `start <= end <= text.len()` on character boundaries, and
    clear `fixes` on a moved diagnostic (their edits name the virtual
    file).
  - A virtual module's `FileDb` name is `path.display().to_string()`.
  - std's modules enter the database as `<std/x>` only after loading:
    lex, parse and import diagnostics render before them.
- `nova_driver::Program::doc_tests(package: Option<&Path>, paths:
  Vec<PathBuf>) -> Program`, `RootKind::Doc`, and `pub const
  DOC_TEST_FUNCTION: &str = "nova_doctest"`.
  - Each path must be unique: it is the module's identity.
  - Module `i` is `paths[i]`; the library, then its imports, follow.
  - The tests are `<stem>::nova_doctest` in `paths` order, one per doc
    module that declares it, so `tests[i]` is doc test `i` when every
    module declares one.
  - The first path names the binary.
  - The library's `FileDb` name is `under(root, "src/lib.nova").display()`,
    the name `nova doc`'s analysis records for the same `root` spelling.
  - Never build an empty `paths`: print `running 0 doc tests` without the
    driver (spec §6.1).
  - `nova-doc`'s `doctest::FUNCTION` duplicates `DOC_TEST_FUNCTION`; it can
    be `pub use nova_driver::DOC_TEST_FUNCTION as FUNCTION;`.
- `nova test [filter]`: the unit section as today, then, in a project with
  `src/lib.nova` and at least one doc test, a blank line and the doc
  section. `nova test --doc [filter]` and `nova test --doc --std [filter]`
  print the doc section alone. Each doc test's line is `test <DocTest.name>
  ... ok`.
- `pub fn nova_driver::std_source(name: &str) -> Option<&'static str>`:
  the text of the std module the database names `name` (`<std/time>`,
  `<std/test>`), or `None`.
- std's examples (Tasks 11 to 14) are checked with `nova test --doc --std`;
  a failing one prints its name, `std::time (std/time/lib.nova:14)`, and a
  compile error in one points at its `//!` line.

## File map

| File | Task | Responsibility |
|---|---|---|
| `crates/nova-driver/src/lib.rs`, `crates/nova-cli/tests/run_tests.rs` | 1 | The test `main`'s `DefId` |
| `crates/nova-cli/src/cmd/new.rs`, `crates/nova-cli/tests/project.rs` | 2 | `nova init --lib`'s message |
| `crates/nova-doc/src/{markdown,lib}.rs` | 3 | The code-block rule, hidden lines, `code_blocks`, `hide_lines` |
| `crates/nova-lsp/{Cargo.toml,src/hover.rs}`, `Cargo.lock`, `crates/nova-cli/tests/lsp_navigation.rs` | 4 | Hover hides hidden lines |
| `crates/nova-driver/src/lib.rs`, `crates/nova-driver/tests/doc_tests.rs` | 5 | Modules from `Sources`; mapped diagnostics |
| `crates/nova-driver/src/{program,lib}.rs`, `crates/nova-driver/tests/doc_tests.rs` | 6 | Doc test roots; collection |
| `crates/nova-doc/src/{markdown,signature,model}.rs` | 7 | Docs keep their source |
| `crates/nova-doc/src/{doctest,model,lib}.rs` | 8 | Doc tests as test modules |
| `crates/nova-cli/src/cmd/test.rs`, `crates/nova-cli/tests/doc_test.rs` | 9, 10 | `nova test`'s doc section, `--doc`, `--std` |
| `std/*/lib.nova`, `crates/nova-doc/tests/std_docs.rs` | 11-14 | std's examples; the completeness condition |
| `examples/05-geometry/{shapes,README.md}`, `.github/scripts/doc-gate.sh`, `crates/nova-cli/tests/doc_test.rs` | 15 | The example's doc tests; CI |
| `docs/adr/0035-doc-tests.md`, records | 16 | The records |

---

### Task 0: Baseline

Recorded on 2026-10-11 at `40db350`, the branch's base, in the ledger:
`cargo test --workspace --locked --no-fail-fast` gave 1942 passed, 1
failed, 10 ignored; the one failure is
`nova-runtime net::tests::an_ipv6_connect_parks_on_its_first_poll_and_then_establishes`,
which needs IPv6 `::1` and is environmental here. Nothing to run; every
later full run compares against this.

---

### Task 1: A test program may hold a function value

A closure, or a named function passed as a value, in a program `nova test`
builds fails with "internal codegen error (this is a compiler bug) ...
defining C main wrapper ... mismatched argument count". The type checker
numbers the functions it lifts for them from `defs().len()` on
(`next_closure_def`, `crates/nova-typeck/src/check.rs`), and the
synthesized test `main` took `DefId(defs().len())` too, so the first
function value and the test `main` shared a `DefId`. `nova run` and `nova
build` are unaffected. std's examples hit it.

**Files:**
- Modify: `crates/nova-driver/src/lib.rs` (`FrontendContext::check` and its
  doc comment)
- Test: `crates/nova-cli/tests/run_tests.rs`

**Interfaces:** none change.

- [ ] **Step 1: Write the failing test**

In `crates/nova-cli/tests/run_tests.rs`, replace the end of the test `an_unfiltered_run_with_no_tests_still_succeeds`:

````rust
    let dir = write_test_project("nova-test-cli-no-tests-at-all", "fn main() { }\n");
    let assert = nova().current_dir(&dir).arg("test").assert().success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout).replace("\r\n", "\n");
    assert_eq!(
        stdout,
        "running 0 tests\n\
         \n\
         test result: ok. 0 passed; 0 failed; 0 trapped; 0 total\n"
    );
}
````

with:

````rust
    let dir = write_test_project("nova-test-cli-no-tests-at-all", "fn main() { }\n");
    let assert = nova().current_dir(&dir).arg("test").assert().success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout).replace("\r\n", "\n");
    assert_eq!(
        stdout,
        "running 0 tests\n\
         \n\
         test result: ok. 0 passed; 0 failed; 0 trapped; 0 total\n"
    );
}

/// A function value in a test program: a closure, and a named function
/// passed as a value. The type checker numbers the functions it lifts for
/// them past every resolved definition, where the synthesized `main` took
/// its `DefId` too, so the first of them collided with it and codegen
/// failed ("mismatched argument count").
#[test]
fn a_test_program_may_hold_function_values() {
    let dir = write_test_project(
        "nova-test-cli-function-values",
        "fn inc(x: Int) -> Int { x + 1 }\n\
         fn apply(f: fn(Int) -> Int, x: Int) -> Int { f(x) }\n\
         @test\nfn a_closure() {\n    let f = |x: Int| x + 1\n    assert_eq(f(1), 2)\n}\n\
         @test\nfn a_named_function_as_a_value() {\n    assert_eq(apply(inc, 1), 2)\n    \
         assert_eq(Some(1).map(inc).unwrap_or(0), 2)\n}\n",
    );
    let assert = nova().current_dir(&dir).arg("test").assert().success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout).replace("\r\n", "\n");
    assert_eq!(
        stdout,
        "running 2 tests\n\
         test a_closure ... ok\n\
         test a_named_function_as_a_value ... ok\n\
         \n\
         test result: ok. 2 passed; 0 failed; 0 trapped; 2 total\n"
    );
}
````

- [ ] **Step 2: Run it to see it fail**

Run: `cd /home/user/nova && cargo test -p nova-cli --test run_tests a_test_program_may_hold 2>&1 | grep -E "^test |mismatched argument count|test result" | head -5`
Expected: FAIL, codegen rejecting the test `main` (the count of filtered
tests is 307 at `9d953d5`, with this test):

````text
test a_test_program_may_hold_function_values ... FAILED
    3: - inst0 (v2 = call fn0()): mismatched argument count for `v2 = call fn0()`: got 0, expected 2
; error: inst0 (v2 = call fn0()): mismatched argument count for `v2 = call fn0()`: got 0, expected 2
    3: - inst0 (v2 = call fn0()): mismatched argument count for `v2 = call fn0()`: got 0, expected 2
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 307 filtered out; finished in …
````

- [ ] **Step 3: The fix**

In `crates/nova-driver/src/lib.rs`, in `check`'s doc comment, its first paragraph, replace:

````rust
    /// a `DefId` guaranteed unused by anything resolution allocated — or
    /// `None` if any stage reported errors.
````

with:

````rust
    /// a `DefId` that no definition and no function of the module holds — or
    /// `None` if any stage reported errors.
````

Then, in the same doc comment, its last paragraph, replace:

````rust
    /// The fresh `DefId` is `resolved.definitions.defs().len()`: `Definitions`
    /// documents `defs` as one global counter shared by every definition kind
    /// (functions, sums, records, traits, methods, externs, associated
    /// types), allocated by `push_def` in strict `Vec::len()` order — so every
    /// id resolution ever handed out is `< that length`, and this value
    /// cannot collide with any of them. `build_test_binary` uses it to name a
    /// synthesized `main` that cannot alias an id one of the `@test`
    /// functions it calls by `DefId` already owns.
````

with:

````rust
    /// The fresh `DefId` is past both kinds of id the module holds.
    /// `Definitions` documents `defs` as one global counter shared by every
    /// definition kind (functions, sums, records, traits, methods, externs,
    /// associated types), allocated by `push_def` in strict `Vec::len()`
    /// order, so every id resolution handed out is `< defs().len()`. The type
    /// checker then numbers the functions it lifts for closures and for named
    /// functions used as values from `defs().len()` on, so the fresh id is
    /// also past every function's. `build_test_binary` uses it to name a
    /// synthesized `main` that cannot alias an id a `@test` function or a
    /// lifted closure already owns.
````

Then, in `check`'s body, after resolution, replace:

````rust
        let fresh_def_id = DefId(resolved.definitions.defs().len() as u32);
        let mut tests = resolved.tests;
````

with:

````rust
        let mut tests = resolved.tests;
````

Then, at the end of `check`, replace:

````rust
        keep_the_entry_main(&mut module, &resolved.definitions);
        Ok(Some((module, tests, fresh_def_id)))
````

with:

````rust
        keep_the_entry_main(&mut module, &resolved.definitions);
        let fresh_def_id = DefId(
            module
                .functions
                .iter()
                .map(|f| f.def_id.0 + 1)
                .fold(resolved.definitions.defs().len() as u32, u32::max),
        );
        Ok(Some((module, tests, fresh_def_id)))
````

- [ ] **Step 4: Run the tests**

Run: `cd /home/user/nova && cargo test -p nova-cli --test run_tests 2>&1 | grep -E "^test result|FAILED"`
Expected: `test result: ok. 307 passed; 0 failed; 1 ignored` (the ignored
one is `extern_ffi_run`, on Linux).

Run: `cd /home/user/nova && cargo test -p nova-driver 2>&1 | grep -E "^test result|FAILED"`
Expected: every line `ok`: 33, 12, 1, 33, 24, 1, 14 and 5 passed, and 0 for
the doc-tests.

- [ ] **Step 5: Commit**

Run `cargo fmt --all` and `cargo clippy -p nova-driver -p nova-cli --all-targets --locked -- -D warnings` (expected: `Finished`).
Message: `fix: a test program may hold a closure`, body: nova test's
synthesized main took the first DefId past resolution, which the type
checker also gives the first closure or function value it lifts, so codegen
failed; it now takes an id past every function's too.

---

### Task 2: `nova init --lib`'s message (spec §3)

**Files:**
- Modify: `crates/nova-cli/src/cmd/new.rs` (`init`)
- Test: `crates/nova-cli/tests/project.rs`

**Interfaces:** none change. `InitCmd`'s `--lib` is its `lib: bool`.

- [ ] **Step 1: Write the failing tests**

In `crates/nova-cli/tests/project.rs`, replace the end of the test `init_lib_beside_a_program_makes_a_package_with_both`:

````rust
    let tested = nova().current_dir(&dir).arg("test").assert().success();
    assert!(
        stdout(&tested).contains("2 passed; 0 failed"),
        "{}",
        stdout(&tested)
    );
}
````

with:

````rust
    let tested = nova().current_dir(&dir).arg("test").assert().success();
    assert!(
        stdout(&tested).contains("2 passed; 0 failed"),
        "{}",
        stdout(&tested)
    );
}

#[test]
fn init_lib_in_a_package_says_how_to_add_its_library() {
    // Spec 3.5b §3: `nova init` still refuses a directory with a
    // nova.toml, and with `--lib` in a package with no library it says how
    // to add one.
    let dir = fresh_dir("init-lib-package").join("app");
    write_project(&dir, "app", HELLO);
    // CI sets RUST_BACKTRACE, which would add a backtrace to the error.
    let out = nova()
        .current_dir(&dir)
        .env("RUST_BACKTRACE", "0")
        .args(["init", "--lib"])
        .assert()
        .failure();
    assert_eq!(
        stderr(&out),
        "Error: this directory already has a nova.toml; to add a library to its package, \
         write src/lib.nova\n"
    );
    assert_eq!(read(dir.join("nova.toml")), manifest("app"));
    assert!(!dir.join("src").join("lib.nova").exists());
    assert!(!dir.join("tests").exists());
}

#[test]
fn init_keeps_its_plain_refusal_unless_a_library_is_missing() {
    // Spec 3.5b §3: otherwise the message is unchanged.
    let plain = "Error: this directory already has a nova.toml\n";
    let dir = fresh_dir("init-lib-has-lib").join("lib");
    write_project(&dir, "lib", HELLO);
    std::fs::write(dir.join("src").join("lib.nova"), TEMPLATE_LIB).unwrap();
    // CI sets RUST_BACKTRACE, which would add a backtrace to the error.
    let out = nova()
        .current_dir(&dir)
        .env("RUST_BACKTRACE", "0")
        .args(["init", "--lib"])
        .assert()
        .failure();
    assert_eq!(stderr(&out), plain);
    let dir = fresh_dir("init-no-lib-flag").join("app");
    write_project(&dir, "app", HELLO);
    let out = nova()
        .current_dir(&dir)
        .env("RUST_BACKTRACE", "0")
        .arg("init")
        .assert()
        .failure();
    assert_eq!(stderr(&out), plain);
}
````

- [ ] **Step 2: Run them to see one fail**

Run: `cd /home/user/nova && cargo test -p nova-cli --test project init_ 2>&1 | grep -E "^test |left|right|test result"`
Expected: FAIL:

````text
test init_lib_in_a_package_says_how_to_add_its_library ... FAILED
  left: "Error: this directory already has a nova.toml\n"
 right: "Error: this directory already has a nova.toml; to add a library to its package, write src/lib.nova\n"
test result: FAILED. 6 passed; 1 failed; 0 ignored; 0 measured; 17 filtered out
````

`init_keeps_its_plain_refusal_unless_a_library_is_missing` passes already:
it guards the unchanged message.

- [ ] **Step 3: The message**

In `crates/nova-cli/src/cmd/new.rs`, replace:

````rust
    if cwd.join(nova_pm::MANIFEST).exists() {
        bail!("this directory already has a nova.toml");
    }
````

with:

````rust
    if cwd.join(nova_pm::MANIFEST).exists() {
        // Spec 3.5b §3: still refused, but `--lib` in a package with no
        // library says how to add one.
        if cmd.lib && !cwd.join("src").join("lib.nova").exists() {
            bail!(
                "this directory already has a nova.toml; to add a library to its package, \
                 write src/lib.nova"
            );
        }
        bail!("this directory already has a nova.toml");
    }
````

- [ ] **Step 4: Run the tests**

Run: `cd /home/user/nova && cargo test -p nova-cli --test project 2>&1 | grep -E "^test result|FAILED"`
Expected: `test result: ok. 24 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out`.

- [ ] **Step 5: Commit**

Run `cargo fmt --all` and `cargo clippy -p nova-cli --all-targets --locked -- -D warnings` (expected: `Finished`).
Message: `feat: nova init --lib says how to add a library to a package`,
body: in a directory with a nova.toml and no src/lib.nova, --lib's refusal
now says to write src/lib.nova (spec 3.5b §3); the plain refusal is
unchanged.

---

### Task 3: The code-block rule, hidden lines, and the site

**Files:**
- Modify: `crates/nova-doc/src/markdown.rs`
- Modify: `crates/nova-doc/src/lib.rs` (one `pub use` line)

**Interfaces:**
- Produces (public, re-exported from `nova_doc`):
  - `Block { Test { unknown: Vec<String> }, Ignored, Other }`
    (`Debug, Clone, PartialEq, Eq`), and `Block::is_nova(&self) -> bool`;
  - `classify(info: Option<&str>) -> Block`, where `None` is an indented
    block;
  - `hidden(line: &str) -> Option<(usize, usize)>`: where a hidden line's
    `# `, or its lone `#`, starts, and how many bytes it is;
  - `CodeLine { text: String, pad: usize, at: usize }` and
    `CodeBlock { block: Block, start: usize, lines: Vec<CodeLine> }`
    (`Debug, Clone, PartialEq, Eq`);
  - `code_blocks(text: &str) -> Vec<CodeBlock>`, where `at` and `start`
    are offsets into `text`;
  - `hide_lines(markdown: &str) -> String`.
- Produces (crate-private): `markdown::options() -> Options`, shared by
  `render`, `code_blocks` and `hide_lines`; and `markdown::shown(code:
  &str) -> String`.
- Changes: `render` highlights `ignore` blocks as Nova, and leaves hidden
  lines out of Nova blocks. An info string's empty words are dropped, so
  `,text` is a `text` block.

- [ ] **Step 1: Write the failing tests**

In `crates/nova-doc/src/markdown.rs`'s `mod tests`, insert before
`    #[test]` / `    fn lines_join_without_one_leading_space() {`, with a
blank line after it:

````rust
    #[test]
    fn blocks_are_classified_by_their_first_word() {
        // 3.5b spec §4.2's table, and an unknown word.
        let test = |unknown: &[&str]| Block::Test {
            unknown: unknown.iter().map(|w| w.to_string()).collect(),
        };
        for (info, block) in [
            (None, test(&[])),
            (Some(""), test(&[])),
            (Some("nova"), test(&[])),
            (Some("nova,ignore"), Block::Ignored),
            (Some("nova ignore"), Block::Ignored),
            (Some("nova, ignore"), Block::Ignored),
            (Some("ignore"), Block::Ignored),
            (Some("text"), Block::Other),
            (Some("sh"), Block::Other),
            (Some(",text"), Block::Other),
            (Some("nova,should_panic"), test(&["should_panic"])),
            (
                Some("nova,,no_run  compile_fail"),
                test(&["no_run", "compile_fail"]),
            ),
        ] {
            assert_eq!(classify(info), block, "{info:?}");
        }
        assert!(test(&[]).is_nova());
        assert!(Block::Ignored.is_nova());
        assert!(!Block::Other.is_nova());
    }

    #[test]
    fn a_hidden_line_starts_with_a_hash_and_a_space_or_is_a_lone_hash() {
        assert_eq!(hidden("# let a = 1"), Some((0, 2)));
        assert_eq!(hidden("    # x"), Some((4, 2)));
        assert_eq!(hidden("\t# x"), Some((1, 2)));
        assert_eq!(hidden("#"), Some((0, 1)));
        assert_eq!(hidden("  #"), Some((2, 1)));
        assert_eq!(hidden("#x"), None);
        assert_eq!(hidden("// # x"), None);
        assert_eq!(hidden("let a = 1 # x"), None);
        assert_eq!(hidden(""), None);
    }

    #[test]
    fn hidden_lines_are_left_out_of_nova_code_on_the_page() {
        // Mutant 1: a `# ` line of a Nova block is not shown; one of a
        // `text` block is.
        let d = render(
            "```\n# let a = 1\n#\nlet b = a\n```\n\n```text\n# shown\n```\n\n    \
             # let c = 2\n    let d = c\n",
            3,
        );
        assert_eq!(
            d.html,
            "<pre class=\"code\"><code><span class=\"kw\">let</span> b = a\n</code></pre>\n\
             <pre class=\"code\"><code># shown\n</code></pre>\n\
             <pre class=\"code\"><code><span class=\"kw\">let</span> d = c\n</code></pre>\n"
        );
    }

    #[test]
    fn an_ignored_block_is_highlighted_as_nova() {
        let d = render("```ignore\n# let a = 1\nfn f() {}\n```", 3);
        assert_eq!(
            d.html,
            "<pre class=\"code\"><code><span class=\"kw\">fn</span> f() {}\n</code></pre>\n"
        );
    }

    #[test]
    fn code_blocks_give_each_line_and_where_it_starts() {
        let text = "Use:\n\n```nova,ignore\nlet a = 1\n# let b = a\n```\n\n- In a list:\n\n      \
                    let c = 3\n\n> ```\n> let d = 4\n> ```\n\n```text\nunclosed\nlast";
        let at = |s: &str| text.find(s).unwrap();
        let line = |s: &str| CodeLine {
            text: s.to_string(),
            pad: 0,
            at: at(s),
        };
        let test = Block::Test {
            unknown: Vec::new(),
        };
        assert_eq!(
            code_blocks(text),
            [
                CodeBlock {
                    block: Block::Ignored,
                    start: at("```nova"),
                    lines: vec![line("let a = 1"), line("# let b = a")],
                },
                CodeBlock {
                    block: test.clone(),
                    start: at("let c"),
                    lines: vec![line("let c = 3")],
                },
                CodeBlock {
                    block: test.clone(),
                    start: at("```\n> let d"),
                    lines: vec![line("let d = 4")],
                },
                CodeBlock {
                    block: Block::Other,
                    start: at("```text"),
                    lines: vec![line("unclosed"), line("last")],
                },
            ]
        );
        // A tab a list item took part of: two spaces made up before `code`.
        assert_eq!(
            code_blocks("- a\n\n \t\tcode"),
            [CodeBlock {
                block: test,
                start: 8,
                lines: vec![CodeLine {
                    text: "  code".to_string(),
                    pad: 2,
                    at: 8,
                }],
            }]
        );
    }

    #[test]
    fn hide_lines_cuts_only_the_hidden_lines_of_nova_blocks() {
        let md = "Use it:\n\n```\n# let x = 1\nlet y = x\n```\n\n```text\n# shown\n```\n\n\
                  > ```nova\n> # q\n> let z = 1\n> ```\n\n# Heading\n\n    # let w = 0\n    \
                  let v = w\n\n```ignore\n#\nlet u = 1\n```\n";
        assert_eq!(
            hide_lines(md),
            "Use it:\n\n```\nlet y = x\n```\n\n```text\n# shown\n```\n\n\
             > ```nova\n> let z = 1\n> ```\n\n# Heading\n\n    let v = w\n\n\
             ```ignore\nlet u = 1\n```\n"
        );
        assert_eq!(hide_lines("No code # here."), "No code # here.");
        assert_eq!(hide_lines("# Title\n\nText."), "# Title\n\nText.");
    }
````

Run: `cd /home/user/nova && cargo test -p nova-doc --lib --locked 2>&1 | grep -E "^error" | sort | uniq -c`
Expected:

```text
      1 error: could not compile `nova-doc` (lib test) due to 35 previous errors
      5 error[E0422]: cannot find struct, variant or union type `CodeBlock` in this scope
      2 error[E0422]: cannot find struct, variant or union type `CodeLine` in this scope
      1 error[E0425]: cannot find function `classify` in this scope
      2 error[E0425]: cannot find function `code_blocks` in this scope
      9 error[E0425]: cannot find function `hidden` in this scope
      3 error[E0425]: cannot find function `hide_lines` in this scope
     13 error[E0433]: cannot find type `Block` in this scope
```

- [ ] **Step 2: The shared rule**

In `crates/nova-doc/src/markdown.rs`, replace the module doc comment's
one line:

```rust
//! Docs as Markdown (spec §9.1, §9.3; plan decision 12).
```

with:

```rust
//! Docs as Markdown (spec §9.1, §9.3; plan decision 12), and the one rule
//! for code blocks and hidden lines that the site, doc tests and hover
//! share (3.5b spec §4.2, §4.3).
```

Then insert after `use crate::highlight::highlight;` and the blank line
after it, before `/// Docs, rendered: their HTML, and their first
paragraph as plain text,`, with a blank line after it:

```rust
/// The options the site renders with, and doc tests and hover read with,
/// so all three find the same code blocks: a table can end a block.
fn options() -> Options {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options
}

/// What a code block is (3.5b spec §4.2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Block {
    /// Nova, highlighted and run as a doc test. `unknown` holds the words
    /// after `nova` that mean nothing, which `nova test` warns about.
    Test {
        /// The unknown words, in order.
        unknown: Vec<String>,
    },
    /// Nova, highlighted, not run: its first word is `ignore`, or a word
    /// after `nova` is.
    Ignored,
    /// Another language, shown as written.
    Other,
}

impl Block {
    /// Whether it is Nova: highlighted, its hidden lines left out.
    pub fn is_nova(&self) -> bool {
        !matches!(self, Block::Other)
    }
}

/// The block whose info string is `info`, or an indented block for `None`
/// (3.5b spec §4.2). The info string is split at commas and whitespace
/// into words, and empty words are dropped.
pub fn classify(info: Option<&str>) -> Block {
    let Some(info) = info else {
        return Block::Test {
            unknown: Vec::new(),
        };
    };
    let mut words = info
        .split(|c: char| c == ',' || c.is_whitespace())
        .filter(|w| !w.is_empty());
    match words.next() {
        None => Block::Test {
            unknown: Vec::new(),
        },
        Some("ignore") => Block::Ignored,
        Some("nova") => {
            let rest: Vec<&str> = words.collect();
            if rest.contains(&"ignore") {
                Block::Ignored
            } else {
                Block::Test {
                    unknown: rest.iter().map(|w| w.to_string()).collect(),
                }
            }
        }
        Some(_) => Block::Other,
    }
}

/// For a hidden line of a Nova block (3.5b spec §4.3), where its `# `, or
/// its lone `#`, starts and how many bytes it is; `None` for a line that
/// shows. It runs less that mark.
pub fn hidden(line: &str) -> Option<(usize, usize)> {
    let rest = line.trim_start_matches([' ', '\t']);
    let at = line.len() - rest.len();
    if rest.starts_with("# ") {
        Some((at, 2))
    } else if rest == "#" {
        Some((at, 1))
    } else {
        None
    }
}

/// A line of a code block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeLine {
    /// The line as the block holds it, without its line break.
    pub text: String,
    /// How many bytes at its start the parser made up: the rest of a tab
    /// that a container or the block's indentation took part of.
    pub pad: usize,
    /// Where `text[pad..]` starts in the Markdown.
    pub at: usize,
}

/// A code block, and where its lines are in the Markdown.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeBlock {
    /// What it is.
    pub block: Block,
    /// Where it starts in the Markdown: its opening fence, or an indented
    /// block's first line.
    pub start: usize,
    /// Its lines, in order.
    pub lines: Vec<CodeLine>,
}

/// Every code block of the Markdown `text`, as the site finds them.
pub fn code_blocks(text: &str) -> Vec<CodeBlock> {
    let mut out: Vec<CodeBlock> = Vec::new();
    let mut open: Option<CodeBlock> = None;
    let mut line: Option<CodeLine> = None;
    for (event, range) in Parser::new_ext(text, options()).into_offset_iter() {
        match event {
            Event::Start(Tag::CodeBlock(kind)) => {
                let block = match &kind {
                    CodeBlockKind::Indented => classify(None),
                    CodeBlockKind::Fenced(info) => classify(Some(info)),
                };
                open = Some(CodeBlock {
                    block,
                    start: range.start,
                    lines: Vec::new(),
                });
            }
            Event::Text(t) => {
                let Some(b) = open.as_mut() else {
                    continue;
                };
                if range.is_empty() {
                    // Spaces the parser made up for a tab's remainder.
                    let l = line.get_or_insert_with(|| CodeLine {
                        text: String::new(),
                        pad: 0,
                        at: range.start,
                    });
                    l.text.push_str(&t);
                    l.pad += t.len();
                    l.at = range.start;
                    continue;
                }
                // `t` is `text[range]`: perhaps several lines, perhaps
                // starting with the line break of a CRLF line.
                let mut at = range.start;
                for piece in t.split_inclusive('\n') {
                    let (body, ends) = match piece.strip_suffix('\n') {
                        Some(body) => (body, true),
                        None => (piece, false),
                    };
                    let l = line.get_or_insert_with(|| CodeLine {
                        text: String::new(),
                        pad: 0,
                        at,
                    });
                    if l.text.len() == l.pad {
                        l.at = at;
                    }
                    l.text.push_str(body);
                    if ends {
                        b.lines.extend(line.take());
                    }
                    at += piece.len();
                }
            }
            Event::End(TagEnd::CodeBlock) => {
                if let Some(mut b) = open.take() {
                    // An unclosed fence's last line has no line break.
                    b.lines.extend(line.take());
                    out.push(b);
                }
            }
            _ => {}
        }
    }
    out
}

/// `markdown` with the hidden lines of its Nova blocks left out (3.5b spec
/// §8.2), every other byte as it was. Hover shows docs through it.
pub fn hide_lines(markdown: &str) -> String {
    if !markdown.contains('#') {
        return markdown.to_string();
    }
    let mut out = String::with_capacity(markdown.len());
    let mut from = 0;
    for b in code_blocks(markdown).iter().filter(|b| b.block.is_nova()) {
        for l in b.lines.iter().filter(|l| hidden(&l.text).is_some()) {
            // The whole Markdown line, with its `> ` or list indentation
            // and its line break.
            let start = markdown[..l.at].rfind('\n').map_or(0, |i| i + 1);
            let end = markdown[l.at..]
                .find('\n')
                .map_or(markdown.len(), |i| l.at + i + 1);
            out.push_str(&markdown[from..start]);
            from = end;
        }
    }
    out.push_str(&markdown[from..]);
    out
}

/// A Nova block's code as the page shows it, its hidden lines left out
/// (3.5b spec §8.1).
fn shown(code: &str) -> String {
    code.split_inclusive('\n')
        .filter(|l| hidden(l.strip_suffix('\n').unwrap_or(l)).is_none())
        .collect()
}
```

- [ ] **Step 3: The site's rendering**

In `render`, replace its doc comment and its first lines, through the
`let body` line:

```rust
/// `text` rendered (spec §9.1): raw HTML as text, a link or an image to
/// a script as its text alone, headings moved down `level` levels, and
/// Nova code blocks coloured (§9.3).
pub(crate) fn render(text: &str, level: u8) -> Docs {
    if text.trim().is_empty() {
        return Docs::default();
    }
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    let mut events: Vec<Event> = Vec::new();
    // A code block being read: whether it is Nova, and its text so far.
    let mut code: Option<(bool, String)> = None;
    // For each link or image open, whether it is kept (decision 10).
    let mut kept: Vec<bool> = Vec::new();
    for event in Parser::new_ext(text, options) {
        if code.is_some() {
            match event {
                Event::Text(t) => {
                    if let Some((_, buf)) = code.as_mut() {
                        buf.push_str(&t);
                    }
                }
                Event::End(TagEnd::CodeBlock) => {
                    if let Some((nova, buf)) = code.take() {
                        let body = if nova { highlight(&buf) } else { escape(&buf) };
```

with:

```rust
/// `text` rendered (spec §9.1): raw HTML as text, a link or an image to
/// a script as its text alone, headings moved down `level` levels, and
/// Nova code blocks coloured (§9.3), `ignore` blocks among them, without
/// their hidden lines (3.5b spec §8.1).
pub(crate) fn render(text: &str, level: u8) -> Docs {
    if text.trim().is_empty() {
        return Docs::default();
    }
    let mut events: Vec<Event> = Vec::new();
    // A code block being read: whether it is Nova, and its text so far.
    let mut code: Option<(bool, String)> = None;
    // For each link or image open, whether it is kept (decision 10).
    let mut kept: Vec<bool> = Vec::new();
    for event in Parser::new_ext(text, options()) {
        if code.is_some() {
            match event {
                Event::Text(t) => {
                    if let Some((_, buf)) = code.as_mut() {
                        buf.push_str(&t);
                    }
                }
                Event::End(TagEnd::CodeBlock) => {
                    if let Some((nova, buf)) = code.take() {
                        let body = if nova {
                            highlight(&shown(&buf))
                        } else {
                            escape(&buf)
                        };
```

Then, further down in `render`, replace the code block's arm:

```rust
            Event::Start(Tag::CodeBlock(kind)) => {
                let nova = match kind {
                    CodeBlockKind::Indented => true,
                    CodeBlockKind::Fenced(info) => {
                        let lang = info
                            .split(|c: char| c == ',' || c.is_whitespace())
                            .next()
                            .unwrap_or("");
                        lang.is_empty() || lang == "nova"
                    }
                };
                code = Some((nova, String::new()));
            }
```

with:

```rust
            Event::Start(Tag::CodeBlock(kind)) => {
                let block = match &kind {
                    CodeBlockKind::Indented => classify(None),
                    CodeBlockKind::Fenced(info) => classify(Some(info)),
                };
                code = Some((block.is_nova(), String::new()));
            }
```

- [ ] **Step 4: Re-export the rule**

In `crates/nova-doc/src/lib.rs`, after
`pub use links::{check_links, BrokenLink};`, add:

```rust
pub use markdown::{classify, code_blocks, hidden, hide_lines, Block, CodeBlock, CodeLine};
```

- [ ] **Step 5: Run the tests**

Run: `cd /home/user/nova && cargo test -p nova-doc --locked 2>&1 | grep -E "^test result|FAILED|panicked"`
Expected: four lines. The unit tests' line reads `test result: ok. 51
passed`, then `gate.rs` has 2 passed, `std_docs.rs` 1 passed, and the
doc-tests 0 passed.

- [ ] **Step 6: Clippy, format, and commit**

Run: `cd /home/user/nova && cargo fmt --all -- --check && cargo clippy -p nova-doc --all-targets --locked -- -D warnings 2>&1 | tail -1`
Expected: no diff, then `Finished`.

Stage `crates/nova-doc/src/markdown.rs` and `crates/nova-doc/src/lib.rs`.
Message: `feat(nova-doc): the code-block rule, hidden lines, and ignore
blocks as Nova`, body: One rule for which blocks are Nova and which lines
are hidden, shared by the site, doc tests and hover (3.5b spec §4.2,
§4.3, §8.1).

---

### Task 4: Hover hides hidden lines

**Files:**
- Modify: `crates/nova-lsp/Cargo.toml`, and `Cargo.lock` (its one
  approved change: a single line)
- Modify: `crates/nova-lsp/src/hover.rs`
- Modify (test): `crates/nova-cli/tests/lsp_navigation.rs` (one test,
  appended)

**Interfaces:**
- Consumes: `nova_doc::hide_lines` (Task 3).
- Produces: hover's docs without their examples' hidden lines (spec
  §8.2). `nova-lsp` depends on `nova-doc`: one new edge between workspace
  crates, and no new package (spec decision 13).
- Unchanged: `docs_above`, and its test. The declaration's fence that
  `hover()` builds does not pass through `hide_lines`. Text with no `#`
  comes back byte for byte, so every existing hover test holds.

- [ ] **Step 1: Write the failing tests**

In `crates/nova-lsp/src/hover.rs`'s `mod tests`, insert after
`docs_above_skip_attributes_and_stop_at_code`'s closing `    }`, with a
blank line before it:

````rust
    #[test]
    fn hover_leaves_out_an_examples_hidden_lines() {
        // 3.5b spec §8.2: the example as the site shows it.
        let text = "/// Doubles `n`.\n///\n/// ```\n/// # let n = 2\n/// let d = twice(n)\n\
                    /// ```\nfn twice(n: Int) -> Int { n * 2 }\n\nfn main() {\n    \
                    let d = twice(1)\n}\n";
        let path = std::env::temp_dir()
            .join("nova-lsp-hover-hidden")
            .join("main.nova");
        let overlay = crate::workspace::Overlay::default().with(&path, text.to_string());
        let options = crate::analysis::options(None, true);
        let answer = crate::analysis::answering(&path, &overlay, &options).expect("an answer");
        let at = text.find("twice(1)").unwrap() as u32;
        let lsp::HoverContents::Markup(found) = hover(&answer, at).expect("a hover").contents
        else {
            panic!("not Markdown");
        };
        assert_eq!(
            found.value,
            "```nova\nfn twice(n: Int) -> Int\n```\n\nDoubles `n`.\n\n```\nlet d = twice(n)\n```"
        );
    }
````

Append to `crates/nova-cli/tests/lsp_navigation.rs`, after a blank line:

````rust
// === 3.5b: hover hides an example's hidden lines (spec §8.2, §12.4) ===

const EXAMPLE: &str = "/// Doubles `n`.\n///\n/// ```\n/// # let hidden = 1\n\
/// let shown = twice(1)\n/// ```\npub fn twice(n: Int) -> Int { n * 2 }\n\n\
fn main() {\n    let d = twice(2)\n}\n";

#[test]
fn hover_leaves_out_an_examples_hidden_lines() {
    let dir = project("hover-hidden-lines", &[("main.nova", EXAMPLE)]);
    let uri = file_uri(&dir.join("src").join("main.nova"));
    let mut client = Client::start(&dir, false);
    open(&mut client, &uri, EXAMPLE);
    let md = markdown(&hover(&mut client, &uri, at(EXAMPLE, "twice(2)", 0)));
    assert!(!md.contains("hidden"), "{md}");
    assert_eq!(
        md,
        "```nova\npub fn twice(n: Int) -> Int\n```\n\nDoubles `n`.\n\n```\nlet shown = twice(1)\n```"
    );
}
````

Run: `cd /home/user/nova && cargo test -p nova-lsp --lib --locked hover 2>&1 | grep -E "^test result|panicked|left|right"`
Expected:

````text
thread 'hover::tests::hover_leaves_out_an_examples_hidden_lines' (…) panicked at crates/nova-lsp/src/hover.rs:233:9:
assertion `left == right` failed
  left: "```nova\nfn twice(n: Int) -> Int\n```\n\nDoubles `n`.\n\n```\n# let n = 2\nlet d = twice(n)\n```"
 right: "```nova\nfn twice(n: Int) -> Int\n```\n\nDoubles `n`.\n\n```\nlet d = twice(n)\n```"
test result: FAILED. 6 passed; 1 failed; 0 ignored; 0 measured; 17 filtered out; …
````

Run: `cd /home/user/nova && cargo test -p nova-cli --locked --test lsp_navigation hover_leaves 2>&1 | grep -E "^test result|panicked"`
Expected: a panic at the `assert!(!md.contains("hidden"), "{md}")` line,
then `test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 37
filtered out; …`.

- [ ] **Step 2: The dependency, and its one line in `Cargo.lock`**

In `crates/nova-lsp/Cargo.toml`, after `nova-fmt = { path =
"../nova-fmt" }`, add:

```toml
# Hover hides an example's hidden lines by the site's rule (spec
# docs/superpowers/specs/2026-10-11-phase-3-5b-doc-tests-design.md §11.4).
nova-doc = { path = "../nova-doc" }
```

In `Cargo.lock`, in `[[package]]` `name = "nova-lsp"`'s `dependencies`,
add this line between ` "nova-diagnostics",` and ` "nova-driver",`:

```text
 "nova-doc",
```

Run: `cd /home/user/nova && cargo metadata --locked --format-version 1 > /dev/null && git diff --stat Cargo.lock && git diff Cargo.lock | grep -E '^[-+] '`
Expected: the command succeeds, with `Cargo.lock | 1 +` and the single
line `+ "nova-doc",`. Cargo may download other platforms' crates into its
cache, but the lock does not change. Any other change to `Cargo.lock`
stops the task: ask the user.

- [ ] **Step 3: Hover through the rule**

In `crates/nova-lsp/src/hover.rs`, replace `docs_of`'s doc comment:

```rust
/// The `///` docs of `target`'s declaration (spec §5.1).
```

with:

```rust
/// The `///` docs of `target`'s declaration (spec §5.1), without their
/// examples' hidden lines.
```

and its last line and closing brace:

```rust
    docs_above(text, at)
}
```

with:

```rust
    // An example's hidden lines are left out, as on the site (3.5b spec
    // §8.2).
    nova_doc::hide_lines(&docs_above(text, at))
}
```

- [ ] **Step 4: Run the tests**

Run: `cd /home/user/nova && cargo test -p nova-lsp --locked 2>&1 | grep -E "^test result|FAILED|panicked" && cargo test -p nova-cli --locked --test lsp_navigation 2>&1 | grep -E "^test result|FAILED|panicked"`
Expected: `test result: ok. 24 passed` for `nova-lsp`'s unit tests, its
doc-tests at 0, then `test result: ok. 38 passed` for `lsp_navigation`.

- [ ] **Step 5: Clippy, the MSRV, format, and commit**

Run: `cd /home/user/nova && cargo fmt --all -- --check && cargo clippy -p nova-lsp --all-targets --locked -- -D warnings 2>&1 | tail -1 && RUSTFLAGS="-D warnings" cargo +1.78 check --locked -p nova-lsp --all-targets 2>&1 | tail -1`
Expected: no diff, then `Finished` twice.

Stage `Cargo.lock`, `crates/nova-lsp/Cargo.toml`,
`crates/nova-lsp/src/hover.rs` and
`crates/nova-cli/tests/lsp_navigation.rs`. Message: `feat(nova-lsp):
hover leaves out an example's hidden lines`, body: Hover shows docs
through nova-doc's rule, so an editor shows the example a reader sees on
the site (3.5b spec §8.2, §11.4); nova-lsp gains its one new dependency
edge, on nova-doc.

---

### Task 5: Modules from `Sources`, and mapped diagnostics (spec §7, §11.2)

**Files:**
- Modify: `crates/nova-driver/src/lib.rs` (`build_test_program`, the new
  `MapDiagnostic` and `build_test_program_with`, `path_fingerprint`,
  `FrontendContext` and its `new`, `with`, `render` and `load_modules`, and a
  new `#[cfg(test)] mod doc_test_builds`)
- Create: `crates/nova-driver/tests/doc_tests.rs`

**Interfaces:**
- Produces: `pub type MapDiagnostic<'a> = dyn Fn(&mut FileDb, &mut
  Diagnostic) -> bool + 'a;` and `pub fn build_test_program_with(program:
  Program, sources: &dyn Sources, map: Option<&MapDiagnostic>) ->
  Result<(PathBuf, Vec<TestFn>)>`, both at the crate root; the contract is
  in the Contracts section.
- Consumes: `Sources` and `DiskSources`, public since Phase 3.2.
- `build_test_program(program)` is `build_test_program_with(program,
  &DiskSources, None)`, so every existing caller is unchanged. The entry is
  now checked by reading it through `sources`, where it was opened from
  disk.
- The alias must carry `+ 'a`: without it, `dyn Fn` defaults to
  `'static`, and a caller's closure that borrows a local is refused. The
  type written inline trips clippy's `type_complexity`.

- [ ] **Step 1: Write the failing tests**

Create `crates/nova-driver/tests/doc_tests.rs`:

````rust
//! What doc tests need from the driver (spec
//! `docs/superpowers/specs/2026-10-11-phase-3-5b-doc-tests-design.md`
//! §11.2, §12.2): a test binary built from sources that are not on disk,
//! and diagnostics mapped before they are rendered.
//!
//! The tests that link need the runtime library beside the test executable,
//! which `cargo build --workspace` puts there, as for `nova-cli`'s
//! `run_tests`.

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::process::Command;

use nova_diagnostics::{render, Diagnostic, FileDb, Span};
use nova_driver::{build_test_program_with, Program, Sources};

/// Files served from memory, then from the disk.
struct Memory(Vec<(PathBuf, String)>);

impl Sources for Memory {
    fn read(&self, path: &Path) -> std::io::Result<String> {
        match self.0.iter().find(|(p, _)| p == path) {
            Some((_, text)) => Ok(text.clone()),
            None => std::fs::read_to_string(path),
        }
    }
}

/// A directory with a fixed name under the system's temporary directory,
/// removed and not made again: the files in it are served from memory.
fn nowhere(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("nova-driver-doc-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

/// The binary's output when `NOVA_TEST_INDEX` is `index`, or its inventory.
fn run(exe: &Path, index: Option<&str>) -> std::process::Output {
    let mut command = Command::new(exe);
    match index {
        Some(index) => command.env("NOVA_TEST_INDEX", index),
        None => command.env_remove("NOVA_TEST_INDEX"),
    };
    command.output().expect("the test binary runs")
}

#[test]
fn a_test_binary_builds_from_sources_not_on_disk() {
    let dir = nowhere("memory");
    let main = dir.join("main.nova");
    let memory = Memory(vec![
        (
            main.clone(),
            "import helper\n\n@test\nfn adds() {\n    assert_eq(add(1, 2), 3)\n}\n\n\
             @test\nfn fails() {\n    assert_eq(add(1, 2), 4)\n}\n"
                .to_string(),
        ),
        (
            dir.join("helper.nova"),
            "pub fn add(a: Int, b: Int) -> Int {\n    a + b\n}\n".to_string(),
        ),
    ]);
    let (exe, tests) =
        build_test_program_with(Program::loose(&main), &memory, None).expect("it builds");
    let names: Vec<&str> = tests.iter().map(|t| t.name.as_str()).collect();
    assert_eq!(names, ["adds", "fails"]);
    assert!(!dir.exists(), "nothing is written beside the sources");
    assert_eq!(
        String::from_utf8_lossy(&run(&exe, None).stdout),
        "2\nadds\nfails\n"
    );
    assert!(run(&exe, Some("0")).status.success());
    let failed = run(&exe, Some("1"));
    assert!(!failed.status.success());
    assert!(String::from_utf8_lossy(&failed.stderr).contains("nova: panic:"));
}

#[test]
fn a_mapped_diagnostic_renders_at_its_source_line() {
    // Spec 3.5b §7: the map moves a label from the module the build reads
    // to the doc comment that holds its code, a file it adds to the
    // database.
    let dir = nowhere("map");
    let main = dir.join("main.nova");
    let code = "@test\nfn uses_nope() {\n    assert_eq(nope(2), 4)\n}\n";
    let doc_name = "docs/lib.nova";
    let doc = "/// Twice.\n///\n/// ```\n/// assert_eq(nope(2), 4)\n/// ```\n\
               pub fn twice(x: Int) -> Int {\n    x * 2\n}\n";
    let memory = Memory(vec![(main.clone(), code.to_string())]);
    let virtual_name = main.display().to_string();
    let shift = doc.find("nope(2").unwrap() as u32 - code.find("nope(2").unwrap() as u32;
    let rendered = RefCell::new(String::new());
    let map = |db: &mut FileDb, d: &mut Diagnostic| {
        for label in &mut d.labels {
            if db.get_name(label.span.file) == Some(virtual_name.as_str()) {
                let file = db.id_of(doc_name).unwrap_or_else(|| db.add(doc_name, doc));
                label.span = Span::new(label.span.start + shift, label.span.end + shift, file);
            }
        }
        rendered
            .borrow_mut()
            .push_str(&render::render_to_string(db, std::slice::from_ref(d)));
        true
    };
    let error = build_test_program_with(Program::loose(&main), &memory, Some(&map))
        .expect_err("`nope` is not defined");
    assert_eq!(
        error.to_string(),
        "could not compile due to 1 previous error"
    );
    let rendered = rendered.into_inner();
    assert!(rendered.contains("E0001"), "{rendered}");
    assert!(rendered.contains("docs/lib.nova:4:15"), "{rendered}");
    assert!(
        rendered.contains("4 │ /// assert_eq(nope(2), 4)"),
        "{rendered}"
    );
}

#[test]
fn a_map_sees_every_diagnostic_and_cannot_hide_an_error() {
    let dir = nowhere("hide");
    let main = dir.join("main.nova");
    let memory = Memory(vec![(
        main.clone(),
        "@test\nfn t() {\n    let x = 1\n    match x {\n        _ => {}\n        1 => {}\n    }\n    \
         let y: Int = \"s\"\n}\n"
            .to_string(),
    )]);
    let seen = RefCell::new(Vec::new());
    let map = |_: &mut FileDb, d: &mut Diagnostic| {
        seen.borrow_mut().push(d.code.clone());
        false
    };
    let error = build_test_program_with(Program::loose(&main), &memory, Some(&map))
        .expect_err("a String is not an Int");
    // The map said `false` to both, and the error was still counted.
    assert_eq!(
        error.to_string(),
        "could not compile due to 1 previous error"
    );
    assert_eq!(seen.into_inner(), ["E0021", "E0010"]);
}
````

In `crates/nova-driver/src/lib.rs`, add the unit tests before the doc comment of `mod async_end_to_end`, whose first line is unique: replace

````rust
/// End-to-end proof that a generated `async fn` actually *runs*: the future
````

with:

````rust
/// What doc tests need of a build (spec
/// `docs/superpowers/specs/2026-10-11-phase-3-5b-doc-tests-design.md`
/// §7, §11.2), where only this crate can see it.
#[cfg(test)]
mod doc_test_builds {
    use super::*;

    /// Every path holds an empty program.
    struct Memory;

    impl Sources for Memory {
        fn read(&self, _: &Path) -> std::io::Result<String> {
            Ok("fn main() {}\n".to_string())
        }
    }

    #[test]
    fn a_map_hides_the_warnings_it_rejects_and_never_an_error() {
        let map = |_: &mut FileDb, _: &mut Diagnostic| false;
        let program = Program::loose(Path::new("<nova driver: memory>/main.nova"));
        let mut ctx = FrontendContext::with(program, &Memory, Some(&map)).expect("the entry reads");
        ctx.render(&[
            Diagnostic::warning("W0000", "a warning"),
            Diagnostic::error("E0000", "an error"),
        ]);
        assert_eq!((ctx.errors, ctx.warnings), (1, 0));
    }

    #[test]
    fn a_path_no_file_holds_is_fingerprinted_from_the_current_directory() {
        // Two packages' relative `<doc tests>/doc0.nova` must not share a
        // test binary.
        let relative = Path::new("<nova driver: no such dir>").join("doc0.nova");
        let absolute = std::env::current_dir()
            .expect("a current directory")
            .join(&relative);
        assert_eq!(path_fingerprint(&relative), path_fingerprint(&absolute));
    }
}

/// End-to-end proof that a generated `async fn` actually *runs*: the future
````

- [ ] **Step 2: Run them to see them fail**

Run: `cd /home/user/nova && cargo test -p nova-driver --test doc_tests 2>&1 | grep -E "^error"`
Expected: FAIL to compile: ``error[E0432]: unresolved import `nova_driver::build_test_program_with` ``.

Run: `cd /home/user/nova && cargo test -p nova-driver --lib doc_test_builds 2>&1 | grep -E "^error"`
Expected: FAIL to compile: ``error[E0599]: no associated function or constant named `with` found for struct `FrontendContext` in the current scope``.

- [ ] **Step 3: `Sources`, the map, and the fingerprint**

In `crates/nova-driver/src/lib.rs`, in `build_test_program`, replace:

````rust
/// [`build_test_binary`] on `program`: the test binary of its roots.
pub fn build_test_program(program: Program) -> Result<(PathBuf, Vec<TestFn>)> {
    let entry = program
        .roots
        .first()
        .map(|root| root.path.clone())
        .unwrap_or_default();
    let mut ctx = FrontendContext::new(program)?;
````

with:

````rust
/// Rewrites a diagnostic before it is counted and rendered, with the
/// program's files at hand (spec
/// `docs/superpowers/specs/2026-10-11-phase-3-5b-doc-tests-design.md` §7):
/// it may move the diagnostic's labels into other files of the database,
/// adding them. `false` hides a warning; an error is always shown.
pub type MapDiagnostic<'a> = dyn Fn(&mut FileDb, &mut Diagnostic) -> bool + 'a;

/// [`build_test_binary`] on `program`: the test binary of its roots.
pub fn build_test_program(program: Program) -> Result<(PathBuf, Vec<TestFn>)> {
    build_test_program_with(program, &DiskSources, None)
}

/// [`build_test_program`], reading every module through `sources`, and
/// passing each diagnostic through `map` before it is counted and rendered
/// (spec 3.5b §7, §11.2). Doc tests are modules no file holds.
pub fn build_test_program_with(
    program: Program,
    sources: &dyn Sources,
    map: Option<&MapDiagnostic>,
) -> Result<(PathBuf, Vec<TestFn>)> {
    let entry = program
        .roots
        .first()
        .map(|root| root.path.clone())
        .unwrap_or_default();
    let mut ctx = FrontendContext::with(program, sources, map)?;
````

Then, in `path_fingerprint`, replace:

````rust
/// A short, deterministic fingerprint of `path`'s canonicalized form, for
/// `build_test_binary`'s output directory. Canonicalizing first (falling
/// back to the path as given if that fails — it cannot fail here in
/// practice, since `build_test_binary` has already opened and read this
/// file by the time this runs) means two different relative spellings of
/// the same file collide on purpose, while two files that merely share a
/// name in different directories do not.
fn path_fingerprint(path: &Path) -> String {
    let canon = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
````

with:

````rust
/// A short, deterministic fingerprint of `path`'s canonicalized form, for
/// `build_test_binary`'s output directory. Canonicalizing first means two
/// different relative spellings of the same file collide on purpose, while
/// two files that merely share a name in different directories do not. A
/// path no file holds, such as a doc test's (spec 3.5b §11.2), is joined
/// onto the current directory instead, so two packages' relative
/// `<doc tests>/doc0.nova` do not share a binary.
fn path_fingerprint(path: &Path) -> String {
    let canon = path.canonicalize().unwrap_or_else(|_| {
        std::env::current_dir()
            .map(|dir| dir.join(path))
            .unwrap_or_else(|_| path.to_path_buf())
    });
````

Then, in `FrontendContext` and its `new`, replace:

````rust
/// Shared front-end state: what to load, the file database, and error
/// accounting.
struct FrontendContext {
    db: FileDb,
    graph: Option<nova_pm::Graph>,
    roots: Vec<Root>,
    errors: usize,
    warnings: usize,
    /// The registry packages' modules, once loaded: a warning only about
    /// them is not shown.
    registry_files: HashSet<FileId>,
}

impl FrontendContext {
    /// Take `program`, rendering its graph's diagnostics. When they hold
    /// no error, the entry must exist: imported modules are read lazily.
    fn new(program: Program) -> Result<Self> {
        let Program {
````

with:

````rust
/// Shared front-end state: what to load, the file database, and error
/// accounting.
struct FrontendContext<'a> {
    db: FileDb,
    graph: Option<nova_pm::Graph>,
    roots: Vec<Root>,
    errors: usize,
    warnings: usize,
    /// The registry packages' modules, once loaded: a warning only about
    /// them is not shown.
    registry_files: HashSet<FileId>,
    /// Where every module is read (spec 3.5b §11.2).
    sources: &'a dyn Sources,
    /// Called on each diagnostic before it is counted and rendered (spec
    /// 3.5b §7).
    map: Option<&'a MapDiagnostic<'a>>,
}

impl<'a> FrontendContext<'a> {
    /// Take `program`, rendering its graph's diagnostics. When they hold
    /// no error, the entry must exist: imported modules are read lazily.
    fn new(program: Program) -> Result<Self> {
        FrontendContext::with(program, &DiskSources, None)
    }

    /// [`FrontendContext::new`], reading every module through `sources`
    /// and passing each diagnostic through `map`.
    fn with(
        program: Program,
        sources: &'a dyn Sources,
        map: Option<&'a MapDiagnostic<'a>>,
    ) -> Result<Self> {
        let Program {
````

Then, at the end of what is now `FrontendContext::with`, replace:

````rust
            errors: 0,
            warnings: 0,
            registry_files: HashSet::new(),
        };
        ctx.render(&diagnostics);
        if ctx.errors == 0 {
            let entry = ctx
                .entry()
                .context("nothing to compile: the package has no source file for this")?;
            std::fs::File::open(entry)
                .with_context(|| format!("failed to open {}", entry.display()))?;
        }
        Ok(ctx)
    }
````

with:

````rust
            errors: 0,
            warnings: 0,
            registry_files: HashSet::new(),
            sources,
            map,
        };
        ctx.render(&diagnostics);
        if ctx.errors == 0 {
            let entry = ctx
                .entry()
                .context("nothing to compile: the package has no source file for this")?;
            ctx.sources
                .read(entry)
                .with_context(|| format!("failed to open {}", entry.display()))?;
        }
        Ok(ctx)
    }
````

Then, at the start of `FrontendContext::render`, replace:

````rust
    fn render(&mut self, diagnostics: &[Diagnostic]) {
        let shown: Vec<Diagnostic> = diagnostics
            .iter()
            .filter(|d| !program::a_dependencys_warning(d, &self.registry_files))
            .cloned()
            .collect();
````

with:

````rust
    fn render(&mut self, diagnostics: &[Diagnostic]) {
        let mut shown: Vec<Diagnostic> = diagnostics
            .iter()
            .filter(|d| !program::a_dependencys_warning(d, &self.registry_files))
            .cloned()
            .collect();
        if let Some(map) = self.map {
            let db = &mut self.db;
            shown.retain_mut(|d| map(db, d) || d.severity == Severity::Error);
        }
````

Then, in `FrontendContext::load_modules`, replace:

````rust
    /// Load, lex and parse every module the roots reach, from disk,
    /// rendering the lex, parse and import diagnostics. The loading itself
    /// is [`program::load_program`], which the language server shares.
    fn load_modules(&mut self) -> Result<Vec<Loaded>> {
        let entry = self.entry().map(Path::to_path_buf).unwrap_or_default();
        let load =
            program::load_program(self.graph.as_ref(), &self.roots, &DiskSources, &mut self.db)
                .with_context(|| format!("failed to read {}", entry.display()))?;
````

with:

````rust
    /// Load, lex and parse every module the roots reach, through
    /// `sources`, rendering the lex, parse and import diagnostics. The
    /// loading itself is [`program::load_program`], which the language
    /// server shares.
    fn load_modules(&mut self) -> Result<Vec<Loaded>> {
        let entry = self.entry().map(Path::to_path_buf).unwrap_or_default();
        let load =
            program::load_program(self.graph.as_ref(), &self.roots, self.sources, &mut self.db)
                .with_context(|| format!("failed to read {}", entry.display()))?;
````

- [ ] **Step 4: Run the tests**

The tests that link need `target/debug/libnova_runtime.a`; if it is
missing, run `cd /home/user/nova && cargo build --locked --workspace` first.

Run: `cd /home/user/nova && cargo test -p nova-driver 2>&1 | grep -E "^test result|FAILED"`
Expected: every line `ok`: 35 (the unit tests, two of them new), 12, 1,
3 (`doc_tests`), 33, 24, 1, 14 and 5 passed, and 0 for the doc-tests.

Run: `cd /home/user/nova && cargo test -p nova-cli --test run_tests --test packages --test project 2>&1 | grep -E "^test result|FAILED"`
Expected: `test result: ok. 10 passed`, `test result: ok. 24 passed`, and
`test result: ok. 307 passed; 0 failed; 1 ignored`.

- [ ] **Step 5: Commit**

Run `cargo fmt --all` and `cargo clippy -p nova-driver --all-targets --locked -- -D warnings` (expected: `Finished`).
Message: `feat(nova-driver): a test binary from any Sources, with mapped
diagnostics`, body: build_test_program_with reads every module through a
Sources and passes each diagnostic through a map before it is counted and
rendered (spec 3.5b §7, §11.2); a path no file holds is fingerprinted from
the current directory.

---

### Task 6: Doc test roots, and collection (spec §5, §6.2)

**Files:**
- Modify: `crates/nova-driver/src/program.rs` (`RootKind`,
  `Program::doc_tests`, `Loaded`, `Place`, `load_program`, `Loader`'s
  `root_place`, `label` and `find`)
- Modify: `crates/nova-driver/src/lib.rs` (`DOC_TEST_FUNCTION`,
  `strip_test_functions` and its doc comment, the comment above its call in
  `check`)
- Test: `crates/nova-driver/tests/doc_tests.rs`

**Interfaces:**
- Produces: `RootKind::Doc`; `pub fn Program::doc_tests(package:
  Option<&Path>, paths: Vec<PathBuf>) -> Program`; `pub const
  DOC_TEST_FUNCTION: &str = "nova_doctest"`. The contract is in the
  Contracts section.
- `analyze` and the language server are unaffected: `analyze_program`
  calls `strip_test_functions` with no doc root, whose behaviour is
  unchanged, and the language server never makes a `Doc` root.
- `RootKind` gains a variant; nothing outside `program.rs` matches on it.

- [ ] **Step 1: Write the failing tests**

In `crates/nova-driver/tests/doc_tests.rs`, replace the module doc's last two lines:

````rust
//! §11.2, §12.2): a test binary built from sources that are not on disk,
//! and diagnostics mapped before they are rendered.
````

with:

````rust
//! §11.2, §12.2): a test binary built from sources that are not on disk,
//! diagnostics mapped before they are rendered, and doc test modules.
````

Then replace the `nova_driver` import:

````rust
use nova_driver::{build_test_program_with, Program, Sources};
````

with:

````rust
use nova_driver::{
    analyze_program, build_test_program_with, Options, Program, Sources, DOC_TEST_FUNCTION,
};
use nova_resolver::ModuleId;
````

Then append, at the end of the file:

````rust

/// A fresh `shapes` package in a directory with a fixed name: a library
/// with its own test, a second module, and two `tests/` files, one named
/// for the package.
fn shapes(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("nova-driver-doc-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    let files = [
        (
            "nova.toml",
            "[package]\nname = \"shapes\"\nversion = \"0.1.0\"\nedition = \"2026\"\n",
        ),
        (
            "src/lib.nova",
            "import units\n\n/// Twice.\npub fn twice(x: Int) -> Int {\n    x * 2 + zero()\n}\n\n\
             @test\nfn own_test() {\n    assert_eq(twice(1), 2)\n}\n",
        ),
        ("src/units.nova", "pub fn zero() -> Int {\n    0\n}\n"),
        ("tests/helpers.nova", "pub fn h() -> Int {\n    1\n}\n"),
        ("tests/shapes.nova", "pub fn s() -> Int {\n    1\n}\n"),
    ];
    for (path, text) in files {
        let path = dir.join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }
    dir
}

/// A doc test whose body is `body`, as a plain block becomes one.
fn plain(body: &str) -> String {
    format!("import shapes\n\n@test\nfn {DOC_TEST_FUNCTION}() {{\n    {body}\n}}\n")
}

/// A doc test that declares `fn main` with `body`, and its own `@test fn
/// extra`.
fn with_main(body: &str) -> String {
    format!(
        "import shapes\n\nfn main() {{\n    {body}\n}}\n\n@test\nfn extra() {{\n    main()\n}}\n\n\
         @test\nfn {DOC_TEST_FUNCTION}() {{\n    main()\n}}\n"
    )
}

fn options() -> Options {
    Options {
        tests: true,
        ..Options::default()
    }
}

fn messages(diagnostics: &[Diagnostic]) -> Vec<String> {
    diagnostics
        .iter()
        .map(|d| format!("{} {}", d.code, d.message))
        .collect()
}

#[test]
fn doc_tests_import_their_package_and_only_their_own_test_is_collected() {
    let dir = shapes("imports");
    let doc0 = dir.join("<doc tests>").join("doc0.nova");
    let doc1 = dir.join("<doc tests>").join("doc1.nova");
    let memory = Memory(vec![
        (doc0.clone(), plain("assert_eq(twice(2), 4)")),
        (doc1.clone(), with_main("assert_eq(twice(3), 6)")),
    ]);
    let program = Program::doc_tests(Some(&dir), vec![doc0.clone(), doc1.clone()]);
    let a = analyze_program(program, &memory, &options()).expect("the entry reads");
    assert_eq!(messages(&a.diagnostics), Vec::<String>::new());
    // The doc tests first, then the library, read as its root names it.
    let paths: Vec<&Path> = a.modules.iter().map(|(_, p)| p.as_path()).collect();
    assert_eq!(
        paths[..4],
        [
            doc0.as_path(),
            doc1.as_path(),
            &dir.join("src").join("lib.nova"),
            &dir.join("src").join("units.nova"),
        ]
    );
    let defs = a.definitions.as_ref().expect("resolved");
    let labels: Vec<&str> = (0..4).map(|i| defs.module_name(ModuleId(i))).collect();
    assert_eq!(labels, ["doc/doc0", "doc/doc1", "lib", "units"]);
    let names: Vec<&str> = defs.defs().iter().map(|d| d.name.as_str()).collect();
    assert!(
        !names.contains(&"own_test"),
        "the library's test is collected"
    );
    assert!(!names.contains(&"extra"), "a block's own test is collected");
    let doc_tests = names.iter().filter(|n| **n == DOC_TEST_FUNCTION).count();
    assert_eq!(doc_tests, 2);
}

#[test]
fn a_doc_test_sees_no_file_beside_it() {
    // `tests/helpers.nova` is not its sibling, and `tests/shapes.nova` does
    // not clash with the package (E0004).
    let dir = shapes("siblings");
    let doc0 = dir.join("<doc tests>").join("doc0.nova");
    let memory = Memory(vec![(
        doc0.clone(),
        format!("import helpers\n{}", plain("assert_eq(twice(1), 2)")),
    )]);
    let a = analyze_program(
        Program::doc_tests(Some(&dir), vec![doc0]),
        &memory,
        &options(),
    )
    .expect("the entry reads");
    assert_eq!(
        messages(&a.diagnostics),
        ["E0001 cannot find module `helpers`"]
    );
}

#[test]
fn a_binary_of_doc_tests_runs_each_alone() {
    let dir = shapes("binary");
    let doc0 = dir.join("<doc tests>").join("doc0.nova");
    let doc1 = dir.join("<doc tests>").join("doc1.nova");
    let memory = Memory(vec![
        (doc0.clone(), plain("assert_eq(twice(2), 4)")),
        (doc1.clone(), with_main("assert_eq(twice(3), 7)")),
    ]);
    let program = Program::doc_tests(Some(&dir), vec![doc0, doc1]);
    let (exe, tests) = build_test_program_with(program, &memory, None).expect("it builds");
    // Named for the first doc test, apart from the unit tests' binary.
    assert_eq!(exe.file_stem().and_then(|s| s.to_str()), Some("doc0"));
    let names: Vec<&str> = tests.iter().map(|t| t.name.as_str()).collect();
    assert_eq!(names, ["doc0::nova_doctest", "doc1::nova_doctest"]);
    assert_eq!(
        String::from_utf8_lossy(&run(&exe, None).stdout),
        "2\ndoc0::nova_doctest\ndoc1::nova_doctest\n"
    );
    assert!(run(&exe, Some("0")).status.success());
    let failed = run(&exe, Some("1"));
    assert!(!failed.status.success());
    assert!(String::from_utf8_lossy(&failed.stderr).contains("nova: panic:"));
}

#[test]
fn std_doc_tests_import_nothing() {
    let dir = Path::new("<nova driver std doc tests>");
    let doc0 = dir.join("doc0.nova");
    let doc1 = dir.join("doc1.nova");
    let test = |body: &str| format!("@test\nfn {DOC_TEST_FUNCTION}() {{\n    {body}\n}}\n");
    let memory = Memory(vec![
        (doc0.clone(), test("assert_eq(1 + 1, 2)")),
        (
            doc1.clone(),
            format!("import doc0\n\n{}", test("assert(true, \"t\")")),
        ),
    ]);
    let program = Program::doc_tests(None, vec![doc0.clone(), doc1.clone()]);
    let a = analyze_program(program, &memory, &options()).expect("the entry reads");
    assert_eq!(
        messages(&a.diagnostics),
        ["E0001 cannot find module `doc0`"]
    );
    let memory = Memory(vec![
        (doc0.clone(), test("assert_eq(1 + 1, 2)")),
        (doc1.clone(), test("assert(true, \"t\")")),
    ]);
    let program = Program::doc_tests(None, vec![doc0, doc1]);
    let (exe, tests) = build_test_program_with(program, &memory, None).expect("it builds");
    let names: Vec<&str> = tests.iter().map(|t| t.name.as_str()).collect();
    assert_eq!(names, ["doc0::nova_doctest", "doc1::nova_doctest"]);
    assert!(run(&exe, Some("0")).status.success());
    assert!(run(&exe, Some("1")).status.success());
}

#[test]
fn no_doc_tests_build_nothing() {
    // Not the library's own tests, under the doc tests' name.
    let dir = shapes("none");
    let program = Program::doc_tests(Some(&dir), Vec::new());
    let error = build_test_program_with(program, &Memory(Vec::new()), None)
        .expect_err("there is nothing to build");
    assert_eq!(
        error.to_string(),
        "nothing to compile: the package has no source file for this"
    );
}
````

- [ ] **Step 2: Run them to see them fail**

Run: `cd /home/user/nova && cargo test -p nova-driver --test doc_tests 2>&1 | grep -E "^error" | sort | uniq -c`
Expected: FAIL to compile, `due to 7 previous errors`:
``error[E0432]: unresolved import `nova_driver::DOC_TEST_FUNCTION` `` once,
and ``error[E0599]: no associated function or constant named `doc_tests`
found for struct `nova_driver::Program` in the current scope`` six times.

- [ ] **Step 3: The doc test root**

In `crates/nova-driver/src/program.rs`, in `RootKind`, replace:

````rust
    /// In no package (spec §4.1).
    Loose,
}
````

with:

````rust
    /// In no package (spec §4.1).
    Loose,
    /// A doc test module (spec
    /// `docs/superpowers/specs/2026-10-11-phase-3-5b-doc-tests-design.md`
    /// §5), at a path of its own that need not be on disk. In a package's
    /// program it imports what a `tests/` module imports, and no file
    /// beside it; without a graph (std's doc tests) it imports nothing. A
    /// program with one keeps only the test each doc test module adds.
    Doc,
}
````

Then, in `impl Program`, before `has_errors`, replace:

````rust
    /// Whether the graph found an error.
    pub fn has_errors(&self) -> bool {
````

with:

````rust
    /// A program of doc tests (spec 3.5b §5, §6.2), each of `paths` a
    /// [`RootKind::Doc`] root, read through the `Sources` given to
    /// `build_test_program_with`: of the package whose `nova.toml` is in
    /// `package`, or of no package for std's (`None`).
    ///
    /// A package's library, `src/lib.nova`, is a root after them. The
    /// loader then reads it under the path the root names, as `nova doc`'s
    /// analysis does, rather than the one an import names, which differs on
    /// Windows under an empty root; a diagnostic's map finds the library in
    /// the database by that name (spec 3.5b §7). The first doc test stays
    /// the entry, which names the binary apart from the unit tests'.
    ///
    /// With no `paths` there is no root, and a build fails with "nothing
    /// to compile" rather than build the library's own tests.
    pub fn doc_tests(package: Option<&Path>, paths: Vec<PathBuf>) -> Program {
        let mut roots: Vec<Root> = paths
            .into_iter()
            .map(|path| Root {
                path,
                kind: RootKind::Doc,
            })
            .collect();
        match package {
            Some(root) => {
                let lib = under(root, "src/lib.nova");
                if !roots.is_empty() && lib.is_file() {
                    roots.push(Root {
                        path: lib,
                        kind: RootKind::Src,
                    });
                }
                Program {
                    roots,
                    runs: false,
                    ..Program::for_package(root, Roots::Program)
                }
            }
            None => Program {
                db: FileDb::new(),
                graph: None,
                roots,
                runs: false,
                diagnostics: Vec::new(),
            },
        }
    }

    /// Whether the graph found an error.
    pub fn has_errors(&self) -> bool {
````

Then, in `Loaded`'s fields, replace:

````rust
    /// Whether it is directly in its package's `tests/`.
    pub test_file: bool,
````

with:

````rust
    /// Whether it is directly in its package's `tests/`, or a doc test
    /// module: its tests are named `<file stem>::<function>`.
    pub test_file: bool,
    /// Whether it is a doc test module (spec 3.5b §6.2).
    pub doc: bool,
````

Then, in `Place` and its methods, replace:

````rust
    /// A loose file.
    Loose { dir: PathBuf, stem: String },
}

impl Place {
    fn package(&self) -> Option<PackageId> {
        match self {
            Place::Package { package, .. } => Some(*package),
            Place::Loose { .. } => None,
        }
    }

    fn is_test(&self) -> bool {
        matches!(self, Place::Package { tests: true, .. })
    }
}
````

with:

````rust
    /// A loose file.
    Loose { dir: PathBuf, stem: String },
    /// A doc test module, by its path (spec 3.5b §5): of the root package,
    /// or of none for std's. Nothing imports one.
    Doc {
        package: Option<PackageId>,
        path: PathBuf,
    },
}

impl Place {
    fn package(&self) -> Option<PackageId> {
        match self {
            Place::Package { package, .. } => Some(*package),
            Place::Loose { .. } => None,
            Place::Doc { package, .. } => *package,
        }
    }

    fn is_test(&self) -> bool {
        matches!(self, Place::Package { tests: true, .. } | Place::Doc { .. })
    }

    fn is_doc(&self) -> bool {
        matches!(self, Place::Doc { .. })
    }
}
````

Then, in the `Loaded` literal of `load_program`, replace:

````rust
            test_file: place.is_test(),
            imports: HashMap::new(),
````

with:

````rust
            test_file: place.is_test(),
            doc: place.is_doc(),
            imports: HashMap::new(),
````

Then, in the importables of `load_program`, replace:

````rust
            if let (Place::Package { package, tests, .. }, Some(graph)) = (from, graph) {
                names.extend(
                    visible_edges(graph, *package, *tests)
                        .into_iter()
                        .map(|e| e.import_name),
                );
            }
````

with:

````rust
            let edges = match (from, graph) {
                (Place::Package { package, tests, .. }, Some(graph)) => {
                    visible_edges(graph, *package, *tests)
                }
                (
                    Place::Doc {
                        package: Some(package),
                        ..
                    },
                    Some(graph),
                ) => visible_edges(graph, *package, true),
                _ => Vec::new(),
            };
            names.extend(edges.into_iter().map(|e| e.import_name));
````

Then, in `Loader::root_place`, replace:

````rust
            (RootKind::Tests, Some(_)) => Place::Package {
                package: PackageId(0),
                tests: true,
                stem,
            },
            _ => Place::Loose {
````

with:

````rust
            (RootKind::Tests, Some(_)) => Place::Package {
                package: PackageId(0),
                tests: true,
                stem,
            },
            (RootKind::Doc, graph) => Place::Doc {
                package: graph.map(|_| PackageId(0)),
                path: root.path.clone(),
            },
            _ => Place::Loose {
````

Then, in `Loader::label`, replace:

````rust
    fn label(&self, place: &Place) -> String {
        match place {
            Place::Loose { stem, .. } => stem.clone(),
````

with:

````rust
    fn label(&self, place: &Place) -> String {
        match place {
            Place::Loose { stem, .. } => stem.clone(),
            Place::Doc { path, .. } => format!("doc/{}", crate::FrontendContext::module_name(path)),
````

Then, at the head of `Loader::find`, replace:

````rust
    fn find(&mut self, from: &Place, name: &str, at: Span, db: &FileDb) -> Found {
        let (package, tests) = match from {
````

with:

````rust
    fn find(&mut self, from: &Place, name: &str, at: Span, db: &FileDb) -> Found {
        let (package, tests, siblings) = match from {
````

Then, in `Loader::find`'s match on `from`, replace:

````rust
            Place::Package { package, tests, .. } => (*package, *tests),
        };
````

with:

````rust
            Place::Package { package, tests, .. } => (*package, *tests, true),
            // What a `tests/` module sees, less the files beside it (spec
            // 3.5b §5.1).
            Place::Doc {
                package: Some(package),
                ..
            } => (*package, true, false),
            // std's doc tests import nothing.
            Place::Doc { package: None, .. } => return Found::Nothing,
        };
````

Then, in `Loader::find`'s sibling lookup, replace:

````rust
        let file = self.has(&dir, name).then(|| {
````

with:

````rust
        let file = (siblings && self.has(&dir, name)).then(|| {
````

- [ ] **Step 4: Collect only the test each doc test adds**

In `crates/nova-driver/src/lib.rs`, at `NOT_THE_ENTRY_MAIN`, replace:

````rust
const NOT_THE_ENTRY_MAIN: &str = "main.not_the_entry";
````

with:

````rust
const NOT_THE_ENTRY_MAIN: &str = "main.not_the_entry";

/// The test function each doc test module adds (spec
/// `docs/superpowers/specs/2026-10-11-phase-3-5b-doc-tests-design.md`
/// §5.2): in a program of doc tests, the only `@test` function collected.
/// Each module is its own namespace, so each can use this name.
pub const DOC_TEST_FUNCTION: &str = "nova_doctest";
````

Then, in the comment above `strip_test_functions`' call in `check`, replace:

````rust
        // Phase 3.3a: under `nova test` the root package's tests are
        // kept and a dependency's stripped (spec
        // docs/superpowers/specs/2026-10-08-phase-3-3a-local-packages-design.md
        // §4.6).
        strip_test_functions(&mut modules, with_test_module);
````

with:

````rust
        // Phase 3.3a: under `nova test` the root package's tests are
        // kept and a dependency's stripped (spec
        // docs/superpowers/specs/2026-10-08-phase-3-3a-local-packages-design.md
        // §4.6). Phase 3.5b: a program of doc tests keeps only the test
        // each doc test adds (spec 3.5b §5.2).
        strip_test_functions(&mut modules, with_test_module);
````

Then, at the head of `strip_test_functions`' doc comment, replace:

````rust
/// Remove top-level `@test` functions from `modules`, in place: from every
/// module, or with `keep_root_tests` only from a dependency's (spec 3.3a
/// §4.6). `nova test` and the language server keep the root package's
/// tests, and a loose program's; a dependency's tests never run in its
/// dependent's binary.
///
/// What follows was written when this ran only for `nova run`, `build`
/// and `check`, and every module was the program's own.
///
/// Only ever called with `with_test_module = false` (`nova run`/`build`/
/// `check`; never `build_test_binary`'s `nova test` path). A `@test`
/// function is ordinary Nova source with no special compilation unit of its
/// own — before this fix,
````

with:

````rust
/// Remove top-level `@test` functions from `modules`, in place, before
/// resolution:
/// - without `keep_root_tests` (`nova run`, `build` and `check`), every one;
/// - with it (`nova test`, and the language server's `tests` option), a
///   dependency's, whose tests never run in its dependent's binary (spec
///   3.3a §4.6), keeping the root package's and a loose program's;
/// - with it, in a program of doc tests (spec
///   `docs/superpowers/specs/2026-10-11-phase-3-5b-doc-tests-design.md`
///   §5.2, §6.2), every one but the [`DOC_TEST_FUNCTION`] each doc test
///   module adds: the library's tests run in the unit tests' binary, and a
///   `@test` function a block declares is not collected.
///
/// What follows was written when this ran only for `nova run`, `build`
/// and `check`, and every module was the program's own; why a test is
/// removed before resolution still holds for every one it removes.
///
/// A `@test` function is ordinary Nova source with no special compilation
/// unit of its own — before this fix,
````

Then, in the tail of that doc comment, and the function, replace:

````rust
/// collection behavior — and every test that exercises it directly —
/// completely unchanged; only the four real compilation entry points that
/// pass `with_test_module = false` are affected, and `nova test` itself
/// never calls this at all.
///
/// As a consequence, `nova_resolver::validate_test_function`'s attribute-
/// shape diagnostics (`E0082`-`E0085`) do not fire for a stripped function
/// under `nova build`/`check`/`run` — matching `#[cfg(test)]`, where an
/// invalid test signature is likewise uncaught outside `cargo test`. `nova
/// test` validates every `@test` function's shape regardless, since this
/// function is never called on that path.
fn strip_test_functions(modules: &mut [Loaded], keep_root_tests: bool) {
    for module in modules.iter_mut() {
        let root = matches!(module.package, None | Some(PackageId(0)));
        if keep_root_tests && root {
            continue;
        }
        module.ast.items.retain(|item| {
            !matches!(
                &item.value,
                nova_ast::Item::Function(f) if f.attrs.iter().any(|a| a.name.value == "test")
            )
        });
    }
}
````

with:

````rust
/// collection behavior — and every test that exercises it directly —
/// completely unchanged.
///
/// As a consequence, `nova_resolver::validate_test_function`'s attribute-
/// shape diagnostics (`E0082`-`E0085`) do not fire for a stripped function
/// — matching `#[cfg(test)]`, where an invalid test signature is likewise
/// uncaught outside `cargo test`. `nova test` validates the shape of every
/// `@test` function it keeps.
fn strip_test_functions(modules: &mut [Loaded], keep_root_tests: bool) {
    let doc_tests = modules.iter().any(|m| m.doc);
    for module in modules.iter_mut() {
        let root = matches!(module.package, None | Some(PackageId(0)));
        let doc = module.doc;
        module.ast.items.retain(|item| match &item.value {
            nova_ast::Item::Function(f) if f.attrs.iter().any(|a| a.name.value == "test") => {
                keep_root_tests
                    && if doc_tests {
                        doc && f.name.value == DOC_TEST_FUNCTION
                    } else {
                        root
                    }
            }
            _ => true,
        });
    }
}
````

- [ ] **Step 5: Run the tests**

Run: `cd /home/user/nova && cargo test -p nova-driver --test doc_tests 2>&1 | grep -E "^test result|FAILED"`
Expected: `test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out`.

Run: `cd /home/user/nova && cargo test -p nova-driver 2>&1 | grep -E "^test result|FAILED"`
Expected: every line `ok`: 35, 12, 1, 8, 33, 24, 1, 14 and 5 passed, and 0
for the doc-tests.

Run: `cd /home/user/nova && cargo test -p nova-cli --test run_tests --test packages --test project --test lsp --test lsp_navigation --test lsp_fixes 2>&1 | grep -E "^test result|FAILED"`
Expected: no FAILED line. Measured on the prototype: `lsp` 36 passed and 1
ignored, `lsp_fixes` 16, `lsp_navigation` 38 (with the hover task's new
test), `packages` 10, `project` 24, `run_tests` 307 passed and 1 ignored. And `cd /home/user/nova && cargo test -p nova-lsp -p nova-doc 2>&1 | grep -E "FAILED|panicked"` prints nothing.

- [ ] **Step 6: The checks, and commit**

Run: `cd /home/user/nova && cargo fmt --all && cargo clippy -p nova-driver -p nova-cli --all-targets --locked -- -D warnings 2>&1 | tail -1 && RUSTFLAGS="-D warnings" cargo +1.78 check --locked -p nova-driver -p nova-cli 2>&1 | tail -1`
Expected: `Finished` twice.

Message: `feat(nova-driver): doc test modules, and only their own test is
collected`, body: RootKind::Doc and Program::doc_tests: a doc test module
imports what a tests/ module does, less its siblings, or nothing for std's;
a program of them keeps only each one's nova_doctest (spec 3.5b §5, §6.2).

- [ ] **Step 7: Mutants (spec §12.5's mutant 7, and two more)**

Each, on the commit of Step 6 and undone with `git checkout --
crates/nova-driver/src`, must fail the tests named:
1. In `strip_test_functions`, `doc && f.name.value == DOC_TEST_FUNCTION`
   becomes `root` (the library's tests collected) →
   `doc_tests_import_their_package_and_only_their_own_test_is_collected`
   and `a_binary_of_doc_tests_runs_each_alone`.
2. It becomes `doc` (a block's own `@test` collected) → the same two.
3. In `Loader::find`, `} => (*package, true, false),` becomes
   `} => (*package, true, true),` (siblings seen) →
   `a_doc_test_sees_no_file_beside_it`, and the two above.

Run each with `cd /home/user/nova && cargo test -p nova-driver --test doc_tests 2>&1 | grep -E "^test .*FAILED|test result"`.
Measured on the prototype: exactly those tests fail.

---

### Task 7: Docs keep their source

**Files:**
- Modify: `crates/nova-doc/src/markdown.rs`,
  `crates/nova-doc/src/signature.rs` and `crates/nova-doc/src/model.rs`

**Interfaces:**
- Produces (crate-private):
  - `markdown::DocText { text: String, file: Option<FileId>, lines:
    Vec<(usize, u32)> }` (`Debug, Clone, Default, PartialEq, Eq`). Each
    entry of `lines` is where a line starts in `text`, and where that
    byte is in `file`.
  - `DocText::push(&mut self, file: FileId, at: u32, body: &str)` and
    `DocText::source(&self, offset: usize) -> Option<u32>`.
  - `markdown::join(lines: &[Spanned<String>]) -> DocText`, which
    returned a `String`.
  - `Docs.source: DocText`, empty for docs no file holds (a primitive's,
    a builtin's, a trait's listing of an impl).
  - `markdown::render_source(source: DocText, level: u8) -> Docs`.
  - `FileText::module_docs(&self) -> DocText`, which returned a `String`.
- Consumes: nothing new. `render.rs` and `search.rs` read only `html` and
  `summary`.
- **The arithmetic** (spec §7):
  - A `///` line's text starts at `span.start + 3`, plus one when `join`
    drops its leading space.
  - A `//!` line's starts at `span.start + (line.len() - body.len())`.
  - Each joined line is an unbroken run of its file's bytes, so byte `k`
    of a line is at its start plus `k`.
  - Lines map one by one, since `//!` lines need not be adjacent: a `//`
    comment or a block comment may sit between them.
  - CRLF needs nothing. The lexer keeps the `\r` out of a `///` line's
    text and out of a comment's span.
  - A bare `///` maps to the end of its `///`, where no code is.

- [ ] **Step 1: Write the failing tests**

In `crates/nova-doc/src/markdown.rs`'s `mod tests`, replace:

```rust
    #[test]
    fn lines_join_without_one_leading_space() {
        let line = |s: &str| Spanned::new(s.to_string(), Span::point(0, FileId::DUMMY));
        assert_eq!(
            join(&[line(" One."), line(""), line("  Two.")]),
            "One.\n\n Two."
        );
    }
```

with:

```rust
    #[test]
    fn lines_join_without_one_leading_space() {
        let line = |s: &str| Spanned::new(s.to_string(), Span::point(0, FileId::DUMMY));
        assert_eq!(
            join(&[line(" One."), line(""), line("  Two.")]).text,
            "One.\n\n Two."
        );
    }

    /// The `///` lines of `src`, as the parser keeps them.
    fn doc_lines(src: &str) -> Vec<Spanned<String>> {
        nova_lexer::lex(src, FileId::DUMMY)
            .0
            .into_iter()
            .filter_map(|t| match t.value {
                nova_lexer::Token::DocComment(s) => Some(Spanned::new(s, t.span)),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn joined_lines_map_back_to_their_source() {
        // 3.5b spec §7: a line with and without its leading space, a bare
        // `///`, an indented one, and the same with CRLF.
        for src in [
            "/// One.\n///\n///Two.\n    ///   three\nfn f() {}\n",
            "/// One.\r\n///\r\n///Two.\r\n    ///   three\r\nfn f() {}\r\n",
        ] {
            let d = join(&doc_lines(src));
            assert_eq!(d.text, "One.\n\nTwo.\n  three", "{src:?}");
            assert_eq!(d.file, Some(FileId::DUMMY));
            assert_eq!(d.lines.len(), 4);
            for word in ["One.", "Two.", "three", "hree"] {
                assert_eq!(
                    d.source(d.text.find(word).unwrap()),
                    Some(src.find(word).unwrap() as u32),
                    "{word:?} in {src:?}"
                );
            }
            // A bare `///` is the end of its `///`.
            let bare = src.find("\n///").unwrap() as u32 + 1;
            assert_eq!(d.source(d.text.find("\n\n").unwrap() + 1), Some(bare + 3));
        }
        assert_eq!(join(&[]).source(0), None);
        assert_eq!(join(&[]).file, None);
    }

    #[test]
    fn rendered_docs_keep_their_source() {
        let d = render_source(join(&doc_lines("/// One.\nfn f() {}\n")), 3);
        assert_eq!(d.html, "<p>One.</p>\n");
        assert_eq!(d.source.text, "One.");
        assert_eq!(d.source.source(0), Some(4));
        assert_eq!(render("One.", 3).source, DocText::default());
    }
```

In `crates/nova-doc/src/signature.rs`'s `mod tests`, replace:

```rust
    #[test]
    fn module_docs_are_the_bang_lines_before_the_first_token() {
        let ft = FileText::new(
            "// A note.\n//! The core.\n//!\n//! More.\nfn f() {}\n//! Not docs.\n",
            FileId::DUMMY,
        );
        assert_eq!(ft.module_docs(), "The core.\n\nMore.");
        assert_eq!(
            FileText::new("fn f() {}\n", FileId::DUMMY).module_docs(),
            ""
        );
        assert_eq!(
            FileText::new("//! Only docs.\n", FileId::DUMMY).module_docs(),
            "Only docs."
        );
    }
```

with:

```rust
    #[test]
    fn module_docs_are_the_bang_lines_before_the_first_token() {
        let ft = FileText::new(
            "// A note.\n//! The core.\n//!\n//! More.\nfn f() {}\n//! Not docs.\n",
            FileId::DUMMY,
        );
        assert_eq!(ft.module_docs().text, "The core.\n\nMore.");
        assert_eq!(
            FileText::new("fn f() {}\n", FileId::DUMMY)
                .module_docs()
                .text,
            ""
        );
        assert_eq!(
            FileText::new("//! Only docs.\n", FileId::DUMMY)
                .module_docs()
                .text,
            "Only docs."
        );
    }

    #[test]
    fn module_docs_map_back_to_their_source() {
        // 3.5b spec §7: `//!` lines need not be adjacent, so each keeps its
        // own place; and the same with CRLF.
        for src in [
            "//! One.\n// A note.\n//!Two.\n/* block */\n//!   three\n//!\nfn f() {}\n",
            "//! One.\r\n// A note.\r\n//!Two.\r\n/* block */\r\n//!   three\r\n//!\r\nfn f() {}\r\n",
        ] {
            let d = FileText::new(src, FileId::DUMMY).module_docs();
            assert_eq!(d.text, "One.\nTwo.\n  three\n", "{src:?}");
            assert_eq!(d.file, Some(FileId::DUMMY));
            for word in ["One.", "Two.", "three", "hree"] {
                assert_eq!(
                    d.source(d.text.find(word).unwrap()),
                    Some(src.find(word).unwrap() as u32),
                    "{word:?} in {src:?}"
                );
            }
            // The bare `//!` is the end of its `//!`.
            let bare = src.rfind("//!").unwrap() as u32;
            assert_eq!(d.source(d.text.len()), Some(bare + 3));
        }
        assert_eq!(
            FileText::new("fn f() {}\n", FileId::DUMMY)
                .module_docs()
                .file,
            None
        );
    }
```

Run: `cd /home/user/nova && cargo test -p nova-doc --lib --locked 2>&1 | grep -E "^error" | sort | uniq -c`
Expected:

```text
      1 error: could not compile `nova-doc` (lib test) due to 23 previous errors
      1 error[E0425]: cannot find function `render_source` in this scope
      1 error[E0433]: cannot find type `DocText` in this scope
      5 error[E0599]: no method named `source` found for struct `std::string::String` in the current scope
      4 error[E0609]: no field `file` on type `std::string::String`
      1 error[E0609]: no field `source` on type `markdown::Docs`
     10 error[E0609]: no field `text` on type `std::string::String`
      1 error[E0615]: attempted to take value of method `lines` on type `std::string::String`
```

- [ ] **Step 2: `DocText`, `join` and `render_source`**

In `crates/nova-doc/src/markdown.rs`, replace
`use nova_diagnostics::Spanned;` with:

```rust
use nova_diagnostics::{FileId, Spanned};
```

Replace `Docs` and `join`:

```rust
/// Docs, rendered: their HTML, and their first paragraph as plain text,
/// which lists and search results show (spec §9.1).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Docs {
    pub html: String,
    pub summary: String,
}

/// Doc lines as the AST keeps them, each the text after its `///`, as one
/// Markdown text: each line loses one leading space.
pub(crate) fn join(lines: &[Spanned<String>]) -> String {
    lines
        .iter()
        .map(|l| l.value.strip_prefix(' ').unwrap_or(&l.value))
        .collect::<Vec<_>>()
        .join("\n")
}
```

with:

```rust
/// Docs, rendered: their HTML, and their first paragraph as plain text,
/// which lists and search results show (spec §9.1).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Docs {
    pub html: String,
    pub summary: String,
    /// The Markdown they were rendered from, and where its lines are in
    /// their file; empty for docs no file holds (3.5b spec §11.1).
    pub source: DocText,
}

/// Docs as one Markdown text, and where each of its lines starts in its
/// file (3.5b spec §7).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct DocText {
    pub text: String,
    /// The file its lines are in; `None` for docs no file holds, as a
    /// primitive's or a builtin's.
    pub file: Option<FileId>,
    /// For each line of `text`: where it starts in `text`, and where that
    /// byte is in `file`.
    pub lines: Vec<(usize, u32)>,
}

impl DocText {
    /// Adds a line, `body`, whose first byte is byte `at` of `file`.
    pub(crate) fn push(&mut self, file: FileId, at: u32, body: &str) {
        if !self.lines.is_empty() {
            self.text.push('\n');
        }
        self.lines.push((self.text.len(), at));
        self.text.push_str(body);
        self.file = Some(file);
    }

    /// Where byte `offset` of `text` is in `file`: each line is a run of
    /// its file's bytes.
    // Read by doc tests from Task 8, which removes this line.
    #[allow(dead_code)]
    pub(crate) fn source(&self, offset: usize) -> Option<u32> {
        let i = self
            .lines
            .partition_point(|&(start, _)| start <= offset)
            .checked_sub(1)?;
        let (start, at) = self.lines[i];
        Some(at + (offset - start) as u32)
    }
}

/// Doc lines as the AST keeps them, each the text after its `///`, as one
/// Markdown text: each line loses one leading space. A line's text starts
/// three bytes into its span, after `///`; the lexer drops only trailing
/// whitespace, a `\r` included (3.5b spec §7).
pub(crate) fn join(lines: &[Spanned<String>]) -> DocText {
    let mut d = DocText::default();
    for l in lines {
        let body = l.value.strip_prefix(' ').unwrap_or(&l.value);
        let skipped = (l.value.len() - body.len()) as u32;
        d.push(l.span.file, l.span.start + 3 + skipped, body);
    }
    d
}

/// Docs rendered as [`render`] renders them, keeping their Markdown and
/// its place in the source, which doc tests read (3.5b spec §11.1).
pub(crate) fn render_source(source: DocText, level: u8) -> Docs {
    let mut d = render(&source.text, level);
    d.source = source;
    d
}
```

At the end of `render`, replace:

```rust
    Docs { html, summary }
```

with:

```rust
    Docs {
        html,
        summary,
        source: DocText::default(),
    }
```

- [ ] **Step 3: Module docs**

In `crates/nova-doc/src/signature.rs`, after
`use crate::highlight::class;`, add:

```rust
use crate::markdown::DocText;
```

Replace `module_docs`:

```rust
    /// The module's docs (spec §9.2): its `//!` lines before its first
    /// token, each without `//!` and one space.
    pub fn module_docs(&self) -> String {
        let first = self
            .tokens
            .iter()
            .find(|t| t.value != Token::Eof)
            .map_or(self.text.len(), |t| t.span.start as usize);
        self.comments
            .iter()
            .filter(|c| c.span.end as usize <= first)
            .filter_map(|c| self.text.get(c.span.start as usize..c.span.end as usize))
            .filter_map(|c| c.strip_prefix("//!"))
            .map(|l| l.strip_prefix(' ').unwrap_or(l))
            .collect::<Vec<_>>()
            .join("\n")
    }
```

with:

```rust
    /// The module's docs (spec §9.2): its `//!` lines before its first
    /// token, each without `//!` and one space, and where each starts in
    /// the file (3.5b spec §7). A comment's span ends before its line
    /// break, and before a CRLF's `\r`.
    pub fn module_docs(&self) -> DocText {
        let first = self
            .tokens
            .iter()
            .find(|t| t.value != Token::Eof)
            .map_or(self.text.len(), |t| t.span.start as usize);
        let mut d = DocText::default();
        for c in self
            .comments
            .iter()
            .filter(|c| c.span.end as usize <= first)
        {
            let Some(line) = self.text.get(c.span.start as usize..c.span.end as usize) else {
                continue;
            };
            let Some(rest) = line.strip_prefix("//!") else {
                continue;
            };
            let body = rest.strip_prefix(' ').unwrap_or(rest);
            d.push(
                c.span.file,
                c.span.start + (line.len() - body.len()) as u32,
                body,
            );
        }
        d
    }
```

- [ ] **Step 4: The model keeps the source**

In `crates/nova-doc/src/model.rs`, replace:

```rust
/// Doc lines rendered, their headings moved down `level` levels.
fn docs_of(lines: &[Spanned<String>], level: u8) -> Docs {
    markdown::render(&markdown::join(lines), level)
}
```

with:

```rust
/// Doc lines rendered, their headings moved down `level` levels, keeping
/// their source (3.5b spec §11.1).
fn docs_of(lines: &[Spanned<String>], level: u8) -> Docs {
    markdown::render_source(markdown::join(lines), level)
}
```

and, in `Ctx::module`, replace the line
`            docs: markdown::render(&ft.module_docs(), 1),` with:

```rust
            docs: markdown::render_source(ft.module_docs(), 1),
```

- [ ] **Step 5: Run the tests**

Run: `cd /home/user/nova && cargo test -p nova-doc --locked 2>&1 | grep -E "^test result|FAILED|panicked"`
Expected: `test result: ok. 54 passed` for the unit tests, then 2, 1 and
0, as in Task 3.

- [ ] **Step 6: Clippy, the MSRV, format, and commit**

Run: `cd /home/user/nova && cargo fmt --all -- --check && cargo clippy -p nova-doc --all-targets --locked -- -D warnings 2>&1 | tail -1 && RUSTFLAGS="-D warnings" cargo +1.78 check --locked -p nova-doc --all-targets 2>&1 | tail -1`
Expected: no diff, then `Finished` twice. Without Task 7's
`#[allow(dead_code)]` on `DocText::source`, clippy reports "method
`source` is never used". Task 8 removes the allow.

Stage the three files. Message: `feat(nova-doc): docs keep their
source`, body: Joined `///` lines and `//!` module docs keep where each
line starts in its file, so a doc test can point a diagnostic at its doc
line (3.5b spec §7, §11.1).

---

### Task 8: `doctest.rs`

**Files:**
- Create: `crates/nova-doc/src/doctest.rs`
- Modify: `crates/nova-doc/src/lib.rs` (`mod doctest;`, and a `pub use`)
- Modify: `crates/nova-doc/src/model.rs` (`document_package` split in
  three)
- Modify: `crates/nova-doc/src/markdown.rs` (Task 7's allow goes)

**Interfaces:**
- Consumes: `markdown::{code_blocks, hidden, Block, CodeBlock, DocText,
  Docs}` (Task 3, Task 7), and the model's `Package`, `Module`, `Item`, `Member`,
  `Impl` and `Primitive`.
- Produces (public, re-exported from `nova_doc`):
  - `DocTest { name: String, place: String, text: String, file: String,
    start: u32, lines: Vec<Origin>, unknown: Vec<String> }` (`Debug,
    Clone, PartialEq, Eq`);
  - `Origin { Added, Code { at: u32, cut: Option<(u32, u32)> } }`
    (`Debug, Clone, Copy, PartialEq, Eq`);
  - `DocTest::map(&self, offset: u32) -> u32`;
  - `package_doc_tests(root: &Path) -> Result<Option<Vec<DocTest>>,
    Failure>`, which is `Ok(None)` for a package with no
    `src/lib.nova`;
  - `std_doc_tests() -> Result<Vec<DocTest>, Failure>`.
- Produces (crate-private):
  - `doctest::FUNCTION`, a `use` of `nova_driver::DOC_TEST_FUNCTION`;
  - `model::library_package(root: &Path) -> Result<Option<(Package,
    Analysis)>, Failure>`;
  - `model::std_package() -> Result<(Package, Analysis), Failure>`;
  - `model::package_analysis(root: &Path) -> Result<(Analysis, bool),
    Failure>`, private to `model.rs`.
- **Names** (amended §5.3): `{module qual}::{item}` and
  `{path}::{member}`; a method by the site's path
  (`shapes::units::Length::plus`); an impl's own docs by the path it is
  listed under; module docs by the module's qual (the package's name for
  the root module); std's as `std::time (std/time/lib.nova:3)`. The place
  is the path relative to the package's root, joined with `/`, and the
  line of the opening fence (or of an indented block's first line).
- **The module a block becomes** (§5). Body lines are never indented, so
  columns are kept:

  ```text
  import <package>                   Added; unless a line imports the package; none for std
  <leading import lines>             Code; through the last leading import, with blank and comment lines
  without a top-level `fn main`:
  @test fn nova_doctest() {          Added
  <every other line, as written>     Code
  }                                  Added
  with one:
  <every other line, as written>     Code
  @test fn nova_doctest() { main() } Added; `{ block_on(main()) }` for `async fn main`
  ```

  The text ends with a line break.
- **The contract with Task 6:**
  - `DocTest.file` is the name nova-doc's analysis gave the block's file
    in its `FileDb`. User modules are named `path.display()` of the path
    the loader read: the library's root module `under(root,
    "src/lib.nova")`, its other modules `root/src/<stem>.nova`
    (`module_dir(..).join(..)`), and std's modules `<std/time>` and so on.
  - The doc test program names the files alike when its graph comes
    from the same `root`, as the user spelled it (`""` for the current
    directory), and `src/lib.nova` is one of its roots. The loader queues
    every root before any module an import reaches, so the library is
    read by its root path even when it is listed after the doc test
    modules, as Task 6's `Program::doc_tests(Some(root), paths)`
    lists it. Its other modules are reached from it by import, as in
    nova-doc's analysis. So the CLI passes the same `root` to
    `package_doc_tests` and to `Program::doc_tests`, and its map finds the
    file by `db.id_of(&test.file)`. Task 8's test
    `doc_tests_are_named_by_the_sites_paths` pins both names.
  - `start`, and every `Origin::Code`'s `at`, are byte offsets into that
    file. `map` takes a byte offset into `text` and gives one into `file`.
    A span is mapped by its start and its end apart, with the end raised
    to the start if it falls before it. An offset on an added line, or
    past the text, maps to `start`.
  - Each doc test module declares exactly one `FUNCTION`. The driver
    collects only that function from a doc test module, so a block's own
    `@test` functions never run (§5.2), and test `i` is doc test `i`.

- [ ] **Step 1: Write the failing tests**

Create `crates/nova-doc/src/doctest.rs` with its doc comment and its
tests:

````rust
//! Doc tests: the Nova code blocks in the docs a library's site shows,
//! each as a test module (spec
//! `docs/superpowers/specs/2026-10-11-phase-3-5b-doc-tests-design.md` §4
//! to §7).

#[cfg(test)]
mod tests {
    use super::*;
    use crate::signature::FileText;
    use std::path::PathBuf;

    /// A fresh, empty directory with a fixed name, so each run replaces the
    /// last.
    fn fresh(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("nova-doc-doctest-{name}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write(dir: &Path, files: &[(&str, &str)]) {
        for (path, text) in files {
            let path = dir.join(path);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, text).unwrap();
        }
    }

    fn manifest(name: &str) -> String {
        format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2026\"\n")
    }

    /// [`package_doc_tests`], its diagnostics rendered when it fails.
    fn doc_tests(root: &Path) -> Option<Vec<DocTest>> {
        match package_doc_tests(root) {
            Ok(tests) => tests,
            Err(Failure::Diagnostics { db, diagnostics }) => panic!(
                "{}",
                nova_diagnostics::render::render_to_string(&db, &diagnostics)
            ),
            Err(Failure::Other(why)) => panic!("{why}"),
        }
    }

    /// Lines that run, line `i` from source offset `100 * i`.
    fn lines(texts: &[&str]) -> Vec<RunLine> {
        texts
            .iter()
            .enumerate()
            .map(|(i, t)| RunLine {
                text: t.to_string(),
                origin: Origin::Code {
                    at: 100 * i as u32,
                    cut: None,
                },
            })
            .collect()
    }

    /// The `///` lines of `src`, read as `file`.
    fn doc_lines(src: &str, file: FileId) -> Vec<Spanned<String>> {
        nova_lexer::lex(src, file)
            .0
            .into_iter()
            .filter_map(|t| match t.value {
                Token::DocComment(s) => Some(Spanned::new(s, t.span)),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn a_block_becomes_a_module() {
        // Spec §5.1 and §5.2. Mutant 3: the package import is not doubled.
        let code = |at: u32| Origin::Code { at, cut: None };
        let (text, origins) = module(
            Some("shapes"),
            lines(&[
                "// Measure.",
                "import shapes::{circle}",
                "",
                "let c = circle(1.0)",
            ]),
        );
        assert_eq!(
            text,
            "// Measure.\nimport shapes::{circle}\n@test fn nova_doctest() {\n\n\
             let c = circle(1.0)\n}\n"
        );
        assert_eq!(
            origins,
            [
                code(0),
                code(100),
                Origin::Added,
                code(200),
                code(300),
                Origin::Added
            ]
        );
        for (import, block, expected) in [
            (
                Some("shapes"),
                &["let c = circle(1.0)"][..],
                "import shapes\n@test fn nova_doctest() {\nlet c = circle(1.0)\n}\n",
            ),
            (
                Some("shapes"),
                &["import shapes", "let c = circle(1.0)"][..],
                "import shapes\n@test fn nova_doctest() {\nlet c = circle(1.0)\n}\n",
            ),
            (
                Some("shapes"),
                &["let s = \"import shapes\" // import shapes"][..],
                "import shapes\n@test fn nova_doctest() {\n\
                 let s = \"import shapes\" // import shapes\n}\n",
            ),
            (
                Some("shapes"),
                &["fn main() {", "    let a = 1", "}"][..],
                "import shapes\nfn main() {\n    let a = 1\n}\n\
                 @test fn nova_doctest() { main() }\n",
            ),
            (
                Some("shapes"),
                &["async fn main() {", "}"][..],
                "import shapes\nasync fn main() {\n}\n\
                 @test fn nova_doctest() { block_on(main()) }\n",
            ),
            (
                Some("shapes"),
                &["fn helper() { fn main() {} }", "helper()"][..],
                "import shapes\n@test fn nova_doctest() {\nfn helper() { fn main() {} }\n\
                 helper()\n}\n",
            ),
            (
                None,
                &["import a::{", "  b,", "}", "f()"][..],
                "import a::{\n  b,\n}\n@test fn nova_doctest() {\nf()\n}\n",
            ),
            (
                None,
                &["import x; let a = 1", "let b = 2"][..],
                "@test fn nova_doctest() {\nimport x; let a = 1\nlet b = 2\n}\n",
            ),
        ] {
            assert_eq!(module(import, lines(block)).0, expected, "{block:?}");
        }
        let (_, origins) = module(Some("shapes"), lines(&["fn main() {", "}"]));
        assert_eq!(origins, [Origin::Added, code(0), code(100), Origin::Added]);
    }

    #[test]
    fn the_line_map_points_into_the_doc_comment() {
        // Spec §7: a body line; a hidden line, less its `# `; and an added
        // line, at the opening fence or an indented block's first line.
        // Plain, CRLF, in a list, in a quote, and indented by a tab.
        for (src, line) in [
            (
                "/// A circle.\n///\n/// ```\n/// # let r = 1.0\n/// let c = circle(r)\n/// ```\n\
                 pub fn circle(r: Float) -> Float { r }\n",
                3,
            ),
            (
                "/// A circle.\r\n///\r\n/// ```\r\n/// # let r = 1.0\r\n/// let c = circle(r)\r\n\
                 /// ```\r\npub fn circle(r: Float) -> Float { r }\r\n",
                3,
            ),
            (
                "    /// - In a list:\n    ///\n    ///   ```\n    ///   # let r = 1.0\n    \
                 ///   let c = circle(r)\n    ///   ```\n    pub fn circle(r: Float) -> Float { r }\n",
                3,
            ),
            (
                "///> ```\n///> # let r = 1.0\n///> let c = circle(r)\n///> ```\nfn f() {}\n",
                1,
            ),
            ("///\tlet r = 1.0\n///\tlet c = circle(r)\nfn f() {}\n", 1),
        ] {
            let mut db = FileDb::new();
            let file = db.add("src/lib.nova", src);
            let doc = markdown::join(&doc_lines(src, file));
            let tests = doc_tests_of("shapes::circle", &doc, &db, Some("shapes"), &|_| {
                "src/lib.nova".to_string()
            });
            assert_eq!(tests.len(), 1, "{src:?}");
            let t = &tests[0];
            assert_eq!(t.name, format!("shapes::circle (src/lib.nova:{line})"));
            assert_eq!(t.place, format!("src/lib.nova:{line}"));
            assert_eq!(t.file, "src/lib.nova");
            // Mutant 2: a hidden line runs less its `# `.
            assert_eq!(
                t.text,
                "import shapes\n@test fn nova_doctest() {\nlet r = 1.0\nlet c = circle(r)\n}\n",
                "{src:?}"
            );
            let at = |s: &str| src.find(s).unwrap() as u32;
            let made = |s: &str| t.text.find(s).unwrap() as u32;
            assert_eq!(t.map(made("c = circle")), at("c = circle"), "{src:?}");
            assert_eq!(t.map(made("let r")), at("let r"), "{src:?}");
            assert_eq!(t.map(made("1.0")), at("1.0"), "{src:?}");
            let first = src.find("```").or_else(|| src.find("let r")).unwrap() as u32;
            assert_eq!(t.start, first, "{src:?}");
            // The added import, and the added closing brace.
            assert_eq!(t.map(0), first);
            assert_eq!(t.map(t.text.len() as u32 - 2), first);
        }
    }

    #[test]
    fn a_std_doc_test_imports_nothing_and_names_its_modules_file() {
        let src = "//! Time.\n//!\n//! ```\n//! let d = 1\n//! ```\npub fn now() -> Int { 1 }\n";
        let mut db = FileDb::new();
        let file = db.add("<std/time>", src);
        let doc = FileText::new(src, file).module_docs();
        let tests = doc_tests_of("std::time", &doc, &db, None, &|f| {
            std_place(db.get_name(f).unwrap_or("?"))
        });
        assert_eq!(tests.len(), 1);
        let t = &tests[0];
        assert_eq!(t.name, "std::time (std/time/lib.nova:3)");
        assert_eq!(t.file, "<std/time>");
        assert_eq!(t.text, "@test fn nova_doctest() {\nlet d = 1\n}\n");
        assert_eq!(
            t.map(t.text.find("d = 1").unwrap() as u32),
            src.find("d = 1").unwrap() as u32
        );
        let Ok(tests) = std_doc_tests() else {
            panic!("std's doc tests are not read")
        };
        for t in &tests {
            assert!(t.name.starts_with("std::"), "{}", t.name);
            assert!(t.place.starts_with("std/"), "{}", t.place);
            assert!(t.file.starts_with("<std/"), "{}", t.file);
        }
    }

    const SHAPES_LIB: &str = "//! Shapes.\n//!\n//! ```\n//! let a = circle(1.0)\n//! ```\n\n\
        import units\n\n\
        /// A circle's area.\n///\n/// ```nova,should_panic\n/// let a = circle(2.0)\n/// ```\n\
        ///\n/// ```text\n/// not a test\n/// ```\n\
        pub fn circle(r: Float) -> Float { 3.0 * r * r }\n\n\
        /// A rectangle.\npub record Rect {\n    /// Its width.\n    ///\n    /// ```\n    \
        /// let r = Rect { width: 1.0 }\n    /// ```\n    pub width: Float,\n}\n\n\
        /// Rect's own methods.\n///\n///     let r = Rect { width: 2.0 }\n\
        impl Rect {\n    /// Its width, twice.\n    ///\n    /// ```ignore\n    \
        /// let w = Rect { width: 1.0 }.twice()\n    /// ```\n    \
        pub fn twice(self) -> Float { self.width * 2.0 }\n}\n\n\
        /// Not exported.\n///\n/// ```\n/// let s = secret()\n/// ```\n\
        fn secret() -> Int { 1 }\n";

    const SHAPES_UNITS: &str = "//! Lengths.\n//!\n//! ```\n//! let l = Length { metres: 1.0 }\n\
        //! ```\n\n/// A length.\npub record Length {\n    pub metres: Float,\n}\n\n\
        impl Length {\n    /// The sum of two lengths.\n    ///\n    /// ```\n    \
        /// let l = Length { metres: 1.0 }.plus(Length { metres: 2.0 })\n    /// ```\n    \
        pub fn plus(self, other: Length) -> Length {\n        \
        Length { metres: self.metres + other.metres }\n    }\n}\n";

    #[test]
    fn doc_tests_are_named_by_the_sites_paths() {
        // Spec §5.3: the package, an item, a member, an impl's own docs, a
        // module, and a method by the site's path.
        let dir = fresh("names");
        write(
            &dir,
            &[
                ("nova.toml", &manifest("shapes")),
                ("src/lib.nova", SHAPES_LIB),
                ("src/units.nova", SHAPES_UNITS),
            ],
        );
        let tests = doc_tests(&dir).expect("a library");
        let names: Vec<&str> = tests.iter().map(|t| t.name.as_str()).collect();
        assert_eq!(
            names,
            [
                "shapes (src/lib.nova:3)",
                "shapes::circle (src/lib.nova:11)",
                "shapes::Rect::width (src/lib.nova:24)",
                "shapes::Rect (src/lib.nova:32)",
                "shapes::units (src/units.nova:3)",
                "shapes::units::Length::plus (src/units.nova:15)",
            ]
        );
        assert_eq!(tests[1].unknown, ["should_panic"]);
        assert!(tests[0].unknown.is_empty());
        assert_eq!(
            tests[1].text,
            "import shapes\n@test fn nova_doctest() {\nlet a = circle(2.0)\n}\n"
        );
        // The names the loader gives the files, which the doc test
        // program's database shares.
        let lib = dir.join("src").join("lib.nova").display().to_string();
        let units = dir.join("src").join("units.nova").display().to_string();
        assert_eq!(tests[0].file, lib);
        assert_eq!(tests[5].file, units);
        assert_eq!(tests[0].start, SHAPES_LIB.find("```").unwrap() as u32);
    }

    #[test]
    fn only_a_librarys_documented_items_give_doc_tests() {
        // Spec §4.1. Mutant 5: a program's blocks are not tests.
        let dir = fresh("program");
        write(
            &dir,
            &[
                ("nova.toml", &manifest("app")),
                (
                    "src/main.nova",
                    "/// Runs.\n///\n/// ```\n/// main()\n/// ```\nfn main() {}\n",
                ),
            ],
        );
        assert_eq!(doc_tests(&dir), None);
        // A private item's block, a `text` block and an `ignore`d one.
        let dir = fresh("quiet");
        write(
            &dir,
            &[
                ("nova.toml", &manifest("quiet")),
                (
                    "src/lib.nova",
                    "/// ```\n/// let a = secret()\n/// ```\nfn secret() -> Int { 1 }\n\n\
                     /// ```text\n/// shown()\n/// ```\npub fn shown() -> Int { secret() }\n\n\
                     /// ```ignore\n/// skipped()\n/// ```\npub fn skipped() -> Int { 2 }\n",
                ),
            ],
        );
        assert_eq!(doc_tests(&dir), Some(Vec::new()));
    }
}
````

In `crates/nova-doc/src/lib.rs`, add `mod doctest;` before
`mod highlight;`.

Run: `cd /home/user/nova && cargo test -p nova-doc --lib --locked 2>&1 | grep -E "^error" | sort | uniq -c`
Expected:

```text
      1 error: could not compile `nova-doc` (lib test) due to 28 previous errors; 1 warning emitted
      1 error[E0422]: cannot find struct, variant or union type `RunLine` in this scope
      2 error[E0425]: cannot find function `doc_tests_of` in this scope
      3 error[E0425]: cannot find function `module` in this scope
      1 error[E0425]: cannot find function `package_doc_tests` in this scope
      1 error[E0425]: cannot find function `std_doc_tests` in this scope
      1 error[E0425]: cannot find function `std_place` in this scope
      1 error[E0425]: cannot find type `DocTest` in this scope
      1 error[E0425]: cannot find type `FileId` in this scope
      2 error[E0425]: cannot find type `Path` in this scope
      1 error[E0425]: cannot find type `RunLine` in this scope
      1 error[E0425]: cannot find type `Spanned` in this scope
      1 error[E0433]: cannot find module or crate `markdown` in this scope
      2 error[E0433]: cannot find type `Failure` in this scope
      2 error[E0433]: cannot find type `FileDb` in this scope
      6 error[E0433]: cannot find type `Origin` in this scope
      1 error[E0433]: cannot find type `Spanned` in this scope
      1 error[E0433]: cannot find type `Token` in this scope
```

- [ ] **Step 2: The model, split for doc tests**

In `crates/nova-doc/src/model.rs`, replace `document_package` and
`document_std`:

```rust
/// Document the package whose `nova.toml` is in `root` (empty for the
/// current directory), its dependencies and std (spec §5.1).
pub fn document_package(root: &Path) -> Result<Documented, Failure> {
    let mut program = Program::for_package(root, Roots::Check);
    let stem = |p: &Path| p.file_stem().and_then(|s| s.to_str()).map(str::to_string);
    let library = program
        .roots
        .iter()
        .any(|r| stem(&r.path).as_deref() == Some("lib"));
    let keep = if library { "lib" } else { "main" };
    program
        .roots
        .retain(|r| stem(&r.path).as_deref() == Some(keep));
    program.runs = false;
    let options = Options {
        index: true,
        module_only: true,
        ..Options::default()
    };
    let analysis = analyze_program(program, &DiskSources, &options)
        .map_err(|e| Failure::Other(format!("cannot read the program: {e}")))?;
    if analysis
        .diagnostics
        .iter()
        .any(|d| d.severity == Severity::Error)
    {
        return Err(Failure::Diagnostics {
            db: analysis.db,
            diagnostics: analysis.diagnostics,
        });
    }
    let std = std_analysis()?;
    let mut packages = Ctx::new(&analysis, false, !library)?.packages();
    packages.push(Ctx::new(&std, true, false)?.std_package());
    clash(&packages)?;
    let start = format!("{}/index.html", packages[0].name);
    let warnings = analysis.diagnostics.clone();
    Ok(Documented {
        site: Site { packages, start },
        db: analysis.db,
        warnings,
    })
}

/// Document std alone (spec §4.1, `--std`).
pub fn document_std() -> Result<Documented, Failure> {
    let std = std_analysis()?;
    let package = Ctx::new(&std, true, false)?.std_package();
    Ok(Documented {
        site: Site {
            packages: vec![package],
            start: "std/index.html".to_string(),
        },
        db: FileDb::new(),
        warnings: Vec::new(),
    })
}
```

with:

```rust
/// Document the package whose `nova.toml` is in `root` (empty for the
/// current directory), its dependencies and std (spec §5.1).
pub fn document_package(root: &Path) -> Result<Documented, Failure> {
    let (analysis, library) = package_analysis(root)?;
    let std = std_analysis()?;
    let mut packages = Ctx::new(&analysis, false, !library)?.packages();
    packages.push(Ctx::new(&std, true, false)?.std_package());
    clash(&packages)?;
    let start = format!("{}/index.html", packages[0].name);
    let warnings = analysis.diagnostics.clone();
    Ok(Documented {
        site: Site { packages, start },
        db: analysis.db,
        warnings,
    })
}

/// A library's own package, as its site documents it, and the analysis it
/// came from; `None` for a package with no library (3.5b spec §4.1).
pub(crate) fn library_package(root: &Path) -> Result<Option<(Package, Analysis)>, Failure> {
    let (analysis, library) = package_analysis(root)?;
    if !library {
        return Ok(None);
    }
    let packages = Ctx::new(&analysis, false, false)?.packages();
    let name = analysis.graph.as_ref().map(|g| g.root().name.clone());
    let package = packages
        .into_iter()
        .find(|p| Some(&p.name) == name.as_ref());
    Ok(package.map(|p| (p, analysis)))
}

/// std's package, as its site documents it, and std's analysis.
pub(crate) fn std_package() -> Result<(Package, Analysis), Failure> {
    let std = std_analysis()?;
    let package = Ctx::new(&std, true, false)?.std_package();
    Ok((package, std))
}

/// The analysis a package's site reads, of its library when it has one,
/// else of its program (spec §5.1), and whether it is a library. Errors
/// are a failure.
fn package_analysis(root: &Path) -> Result<(Analysis, bool), Failure> {
    let mut program = Program::for_package(root, Roots::Check);
    let stem = |p: &Path| p.file_stem().and_then(|s| s.to_str()).map(str::to_string);
    let library = program
        .roots
        .iter()
        .any(|r| stem(&r.path).as_deref() == Some("lib"));
    let keep = if library { "lib" } else { "main" };
    program
        .roots
        .retain(|r| stem(&r.path).as_deref() == Some(keep));
    program.runs = false;
    let options = Options {
        index: true,
        module_only: true,
        ..Options::default()
    };
    let analysis = analyze_program(program, &DiskSources, &options)
        .map_err(|e| Failure::Other(format!("cannot read the program: {e}")))?;
    if analysis
        .diagnostics
        .iter()
        .any(|d| d.severity == Severity::Error)
    {
        return Err(Failure::Diagnostics {
            db: analysis.db,
            diagnostics: analysis.diagnostics,
        });
    }
    Ok((analysis, library))
}

/// Document std alone (spec §4.1, `--std`).
pub fn document_std() -> Result<Documented, Failure> {
    let (package, _) = std_package()?;
    Ok(Documented {
        site: Site {
            packages: vec![package],
            start: "std/index.html".to_string(),
        },
        db: FileDb::new(),
        warnings: Vec::new(),
    })
}
```

- [ ] **Step 3: Doc tests**

In `crates/nova-doc/src/doctest.rs`, insert between the module doc
comment's blank line and `#[cfg(test)]`, with a blank line after it:

```rust
use std::path::{Component, Path};

use nova_diagnostics::{FileDb, FileId, Spanned};
use nova_lexer::Token;

use crate::markdown::{self, Block, CodeBlock, DocText, Docs};
use crate::model::{self, Failure, Impl, Package};

// The test function each doc test module declares (spec §5.2), which
// the driver collects.
use nova_driver::DOC_TEST_FUNCTION as FUNCTION;

/// Where a line of a doc test's text comes from (spec §7).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Origin {
    /// A line the doc test adds: the package's import, or the test
    /// function's.
    Added,
    /// A line of the block.
    Code {
        /// The offset in the file that its column 0 maps to.
        at: u32,
        /// For a hidden line, the column its `# ` was cut at, and how many
        /// bytes were cut.
        cut: Option<(u32, u32)>,
    },
}

/// A doc test: one Nova code block of the docs a library's site shows, as
/// a test module (spec §5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocTest {
    /// `shapes::circle (src/lib.nova:41)` (spec §5.3).
    pub name: String,
    /// `src/lib.nova:41`: where the block starts, which an unknown word's
    /// warning names (spec §4.2).
    pub place: String,
    /// The module's text (spec §5), ending in a line break.
    pub text: String,
    /// The block's file, by its name in the program's `FileDb`: the path
    /// the loader read it from, as `Path::display` writes it, or
    /// `<std/time>` for std's.
    pub file: String,
    /// Where the block starts in `file`: its opening fence, or an indented
    /// block's first line, which added lines map to (spec §7).
    pub start: u32,
    /// Where each line of `text` comes from.
    pub lines: Vec<Origin>,
    /// The words after `nova` that mean nothing (spec §4.2).
    pub unknown: Vec<String>,
}

impl DocTest {
    /// Where byte `offset` of `text` is in `file` (spec §7). A hidden
    /// line's column after its cut moves past its `# `, and an added line
    /// is at `start`.
    pub fn map(&self, offset: u32) -> u32 {
        let bytes = self.text.as_bytes();
        let offset = (offset as usize).min(bytes.len());
        let line = bytes[..offset].iter().filter(|&&b| b == b'\n').count();
        let line_start = bytes[..offset]
            .iter()
            .rposition(|&b| b == b'\n')
            .map_or(0, |i| i + 1);
        let col = (offset - line_start) as u32;
        match self.lines.get(line) {
            Some(Origin::Code {
                at,
                cut: Some((c, len)),
            }) if col >= *c => at + col + len,
            Some(Origin::Code { at, .. }) => at + col,
            _ => self.start,
        }
    }
}

/// The doc tests of the package whose `nova.toml` is in `root` (spec
/// §4.1), in the site's order; `None` when it has no library.
pub fn package_doc_tests(root: &Path) -> Result<Option<Vec<DocTest>>, Failure> {
    let Some((package, a)) = model::library_package(root)? else {
        return Ok(None);
    };
    let Some(graph) = a.graph.as_ref() else {
        return Ok(Some(Vec::new()));
    };
    let (dir, import) = (&graph.root().dir, graph.root().import_name.as_str());
    let shown = |file: FileId| match a.modules.iter().find(|(f, _)| *f == file) {
        Some((_, path)) => slashed(path.strip_prefix(dir).unwrap_or(path)),
        None => a.db.get_name(file).unwrap_or("?").to_string(),
    };
    Ok(Some(collect(&package, &a.db, Some(import), &shown)))
}

/// std's doc tests (spec §6.1, `--std`), in the site's order.
pub fn std_doc_tests() -> Result<Vec<DocTest>, Failure> {
    let (package, a) = model::std_package()?;
    let shown = |file: FileId| std_place(a.db.get_name(file).unwrap_or("?"));
    Ok(collect(&package, &a.db, None, &shown))
}

/// A std module's file as a doc test's name shows it: `<std/time>` is
/// `std/time/lib.nova` (spec §5.3, §7).
fn std_place(name: &str) -> String {
    match name.strip_prefix("<std/").and_then(|n| n.strip_suffix('>')) {
        Some(m) => format!("std/{m}/lib.nova"),
        None => name.to_string(),
    }
}

/// `path`'s parts, joined with `/` (spec §5.3).
fn slashed(path: &Path) -> String {
    path.components()
        .filter_map(|c| match c {
            Component::Normal(s) => s.to_str(),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/")
}

/// The doc tests of `package`'s docs. `import` is the package's import
/// name, `None` for std; `shown` names a file as a doc test's name shows
/// it.
fn collect(
    package: &Package,
    db: &FileDb,
    import: Option<&str>,
    shown: &dyn Fn(FileId) -> String,
) -> Vec<DocTest> {
    let mut tests = Vec::new();
    for (path, docs) in documented(package) {
        tests.extend(doc_tests_of(&path, &docs.source, db, import, shown));
    }
    tests
}

/// Every documented thing's docs, by the path a doc test of it is named
/// by (spec §5.3): the search index's paths, in the site's order. Docs no
/// file holds are among them, and give no doc test.
fn documented(package: &Package) -> Vec<(String, &Docs)> {
    fn impl_docs<'p>(out: &mut Vec<(String, &'p Docs)>, path: &str, i: &'p Impl) {
        out.push((path.to_string(), &i.docs));
        for m in &i.methods {
            out.push((format!("{path}::{}", m.name), &m.docs));
        }
    }
    let mut out = Vec::new();
    for m in &package.modules {
        out.push((m.qual.clone(), &m.docs));
        for it in &m.items {
            let path = format!("{}::{}", m.qual, it.name);
            out.push((path.clone(), &it.docs));
            for member in &it.members {
                out.push((format!("{path}::{}", member.name), &member.docs));
            }
            for i in &it.impls {
                impl_docs(&mut out, &path, i);
            }
        }
        for i in &m.other_impls {
            impl_docs(&mut out, &format!("{}::{}", m.qual, i.on), i);
        }
    }
    for pr in &package.primitives {
        for i in &pr.impls {
            impl_docs(&mut out, &format!("std::{}", pr.name), i);
        }
    }
    out
}

/// The doc tests of one doc text, `path`'s: its blocks that run (spec
/// §4.2).
fn doc_tests_of(
    path: &str,
    doc: &DocText,
    db: &FileDb,
    import: Option<&str>,
    shown: &dyn Fn(FileId) -> String,
) -> Vec<DocTest> {
    let Some(file) = doc.file else {
        return Vec::new();
    };
    let Some(file_name) = db.get_name(file) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for b in markdown::code_blocks(&doc.text) {
        let Block::Test { unknown } = &b.block else {
            continue;
        };
        let Some(start) = doc.source(b.start) else {
            continue;
        };
        let line = db.location(file, start).map_or(1, |(l, _)| l);
        let place = format!("{}:{line}", shown(file));
        let (text, lines) = module(import, run_lines(&b, doc));
        out.push(DocTest {
            name: format!("{path} ({place})"),
            place,
            text,
            file: file_name.to_string(),
            start,
            lines,
            unknown: unknown.clone(),
        });
    }
    out
}

/// A block's line as it runs (spec §4.3), and where it comes from.
struct RunLine {
    text: String,
    origin: Origin,
}

/// `b`'s lines as they run, each mapped through `doc` into its file: a
/// hidden line less its `# `.
fn run_lines(b: &CodeBlock, doc: &DocText) -> Vec<RunLine> {
    b.lines
        .iter()
        .map(|l| {
            // `text[pad..]` starts at `doc.source(l.at)`; a made-up column
            // maps into the tab before it.
            let at = doc.source(l.at).unwrap_or(0).saturating_sub(l.pad as u32);
            match markdown::hidden(&l.text) {
                Some((cut, len)) => RunLine {
                    text: format!("{}{}", &l.text[..cut], &l.text[cut + len..]),
                    origin: Origin::Code {
                        at,
                        cut: Some((cut as u32, len as u32)),
                    },
                },
                None => RunLine {
                    text: l.text.clone(),
                    origin: Origin::Code { at, cut: None },
                },
            }
        })
        .collect()
}

/// A block's test module (spec §5), and where each of its lines comes
/// from. `import` is the package's import name, `None` for std's. Lines
/// keep their columns: none is indented.
fn module(import: Option<&str>, lines: Vec<RunLine>) -> (String, Vec<Origin>) {
    let code = lines
        .iter()
        .map(|l| l.text.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    // Comments and strings are not tokens, so `import` in either is not
    // an import.
    let (tokens, _) = nova_lexer::lex(&code, FileId::DUMMY);
    let hoisted = leading_imports(&code, &tokens);
    let main = declares_main(&tokens);
    let mut out: Vec<(String, Origin)> = Vec::new();
    if let Some(p) = import {
        if !imports(&tokens, p) {
            out.push((format!("import {p}"), Origin::Added));
        }
    }
    let mut rest = lines.into_iter();
    out.extend(rest.by_ref().take(hoisted).map(|l| (l.text, l.origin)));
    match main {
        Some(is_async) => {
            out.extend(rest.map(|l| (l.text, l.origin)));
            let call = if is_async {
                "block_on(main())"
            } else {
                "main()"
            };
            out.push((format!("@test fn {FUNCTION}() {{ {call} }}"), Origin::Added));
        }
        None => {
            out.push((format!("@test fn {FUNCTION}() {{"), Origin::Added));
            out.extend(rest.map(|l| (l.text, l.origin)));
            out.push(("}".to_string(), Origin::Added));
        }
    }
    let mut text = out
        .iter()
        .map(|(t, _)| t.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    text.push('\n');
    (text, out.into_iter().map(|(_, o)| o).collect())
}

/// The index of the token after the import at `tokens[i]`: `import`, a
/// path, then `::{…}`, `{…}` or `as name`, and a `;`.
fn import_end(tokens: &[Spanned<Token>], i: usize) -> usize {
    let at = |j: usize| tokens.get(j).map(|t| &t.value);
    let mut j = i + 1;
    if matches!(at(j), Some(Token::Ident(_))) {
        j += 1;
    }
    while at(j) == Some(&Token::ColonColon) && matches!(at(j + 1), Some(Token::Ident(_))) {
        j += 2;
    }
    if at(j) == Some(&Token::ColonColon) {
        j += 1;
    }
    match at(j) {
        Some(Token::LBrace) => {
            while !matches!(at(j), None | Some(Token::RBrace) | Some(Token::Eof)) {
                j += 1;
            }
            if at(j) == Some(&Token::RBrace) {
                j += 1;
            }
        }
        Some(Token::As) => {
            j += 1;
            if matches!(at(j), Some(Token::Ident(_))) {
                j += 1;
            }
        }
        _ => {}
    }
    if at(j) == Some(&Token::Semicolon) {
        j += 1;
    }
    j
}

/// How many lines at the top of `code` are its leading imports, with the
/// blank and comment lines among them (spec §5.1). An import whose last
/// line goes on with other code stops them.
fn leading_imports(code: &str, tokens: &[Spanned<Token>]) -> usize {
    let line = |offset: u32| {
        code.as_bytes()[..(offset as usize).min(code.len())]
            .iter()
            .filter(|&&b| b == b'\n')
            .count()
    };
    let mut lines = 0;
    let mut i = 0;
    while tokens.get(i).is_some_and(|t| t.value == Token::Import) {
        let end = import_end(tokens, i);
        let last = line(tokens[end - 1].span.end.saturating_sub(1));
        let next = &tokens[end.min(tokens.len() - 1)];
        if next.value != Token::Eof && line(next.span.start) <= last {
            break;
        }
        lines = last + 1;
        i = end;
    }
    lines
}

/// Whether `tokens` declare `fn main` at the top level, and whether it is
/// `async` (spec §5.2).
fn declares_main(tokens: &[Spanned<Token>]) -> Option<bool> {
    let mut depth = 0usize;
    for (i, t) in tokens.iter().enumerate() {
        match &t.value {
            Token::LBrace => depth += 1,
            Token::RBrace => depth = depth.saturating_sub(1),
            Token::Fn if depth == 0 => {
                let named_main = matches!(
                    tokens.get(i + 1).map(|t| &t.value),
                    Some(Token::Ident(n)) if n == "main"
                );
                if named_main {
                    return Some(i > 0 && tokens[i - 1].value == Token::Async);
                }
            }
            _ => {}
        }
    }
    None
}

/// Whether `tokens` import the package `name` itself (spec §5.1):
/// `import name`, `import name::{…}` or `import name as …`.
fn imports(tokens: &[Spanned<Token>], name: &str) -> bool {
    tokens
        .windows(2)
        .any(|w| w[0].value == Token::Import && matches!(&w[1].value, Token::Ident(n) if n == name))
}
```

- [ ] **Step 4: Re-export, and drop Task 7's allow**

In `crates/nova-doc/src/lib.rs`, before
`pub use links::{check_links, BrokenLink};`, add:

```rust
pub use doctest::{package_doc_tests, std_doc_tests, DocTest, Origin};
```

In `crates/nova-doc/src/markdown.rs`, delete the two lines above
`    pub(crate) fn source(&self, offset: usize) -> Option<u32> {`:

```rust
    // Read by doc tests from Task 8, which removes this line.
    #[allow(dead_code)]
```

- [ ] **Step 5: Run the tests**

Run: `cd /home/user/nova && cargo test -p nova-doc --locked 2>&1 | grep -E "^test result|FAILED|panicked"`
Expected: `test result: ok. 59 passed` for the unit tests, then 2, 1 and
0, as in Task 3.

- [ ] **Step 6: Clippy, the MSRV, format, and commit**

Run: `cd /home/user/nova && cargo fmt --all -- --check && cargo clippy -p nova-doc -p nova-lsp --all-targets --locked -- -D warnings 2>&1 | tail -1 && RUSTFLAGS="-D warnings" cargo +1.78 check --locked -p nova-doc --all-targets 2>&1 | tail -1 && git diff 9d953d5 -- Cargo.lock | grep -E '^[-+] '`
Expected: no diff, `Finished` twice, and only Task 4's line,
`+ "nova-doc",`.

Stage `crates/nova-doc/src/doctest.rs`, `crates/nova-doc/src/lib.rs`,
`crates/nova-doc/src/markdown.rs` and `crates/nova-doc/src/model.rs`.
Message: `feat(nova-doc): a library's doc tests, as test modules`, body:
Each Nova block in the docs a library's site shows becomes a test module
named by its item and line, with a map from each of its lines to the doc
comment (3.5b spec §4.1, §5, §7, §11.1).

---

### Task 9: `nova test --doc`, and the doc section of `nova test` (spec §6, §7)

**Files:**
- Modify: `crates/nova-cli/src/cmd/test.rs` (the module doc, the imports,
  `TestCmd`, `run`, and the new `run_doc_tests`, `has_library`,
  `library_doc_tests`, `failure`, `DOC_TESTS`, `run_doc_section`,
  `build_doc_tests`, `Overlay`, `to_the_docs`, `on_a_char`,
  `warn_unknown_words`, `inventory`, `select`, `ensure_matched`, `Tally`,
  `run_tests` and `finish`, most of the last six carved from the old `run`)
- Create: `crates/nova-cli/tests/doc_test.rs`

**Interfaces:**
- Consumes: `nova_doc::{package_doc_tests, DocTest, Failure}` (Task 8);
  `nova_driver::{build_test_program_with, Program::doc_tests, Sources,
  DiskSources}` and the `MapDiagnostic` contract (Task 5, Task 6);
  `crate::project::{mode, sync, Mode}`.
- `TestFn` is not nameable in nova-cli (it is `nova_resolver`'s), so the
  run loop takes each binary's names and `should_panic` flags as vectors.
- The unit section's output and messages are byte for byte today's: the
  guard test `a_library_with_no_doc_tests_prints_what_it_printed_before`
  passes before and after, and `run_tests`' 307 tests pass unchanged.

- [ ] **Step 1: Write the failing tests**

Create `crates/nova-cli/tests/doc_test.rs`:

````rust
//! `nova test`'s doc tests, end to end (spec
//! `docs/superpowers/specs/2026-10-11-phase-3-5b-doc-tests-design.md`
//! §6, §7, §12.3).

use std::path::{Path, PathBuf};

use assert_cmd::assert::Assert;
use assert_cmd::Command;

fn nova() -> Command {
    Command::cargo_bin("nova").expect("nova binary builds")
}

/// A fresh, empty directory with a fixed name, so each run replaces the
/// last.
fn fresh(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("nova-doc-test-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create the test directory");
    dir
}

fn write(dir: &Path, files: &[(&str, &str)]) {
    for (path, text) in files {
        let path = dir.join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }
}

/// `nova test ARGS` in `dir`, with a fresh `NOVA_HOME` beside it. Colour
/// is off, as `nova` colours its diagnostics even into a pipe, and so is
/// anyhow's backtrace, which CI's `RUST_BACKTRACE=1` adds to the closing
/// error.
fn nova_test(dir: &Path, args: &[&str]) -> Assert {
    let home = dir.with_file_name(format!(
        "{}-home",
        dir.file_name().unwrap().to_string_lossy()
    ));
    let _ = std::fs::remove_dir_all(&home);
    nova()
        .current_dir(dir)
        .env("NOVA_HOME", &home)
        .env("NO_COLOR", "1")
        .env("RUST_BACKTRACE", "0")
        .arg("test")
        .args(args)
        .assert()
}

fn stdout(a: &Assert) -> String {
    String::from_utf8_lossy(&a.get_output().stdout).replace("\r\n", "\n")
}

fn stderr(a: &Assert) -> String {
    String::from_utf8_lossy(&a.get_output().stderr).replace("\r\n", "\n")
}

const MANIFEST: &str = "[package]\nname = \"calc\"\nversion = \"0.1.0\"\nedition = \"2026\"\n";

/// The library's root module: an example in its `//!` docs, one on
/// `twice`, and a test of its own.
const LIB: &str = "//! Arithmetic, checked.\n//!\n//! ```\n//! assert_eq(twice(2) + 1, 5)\n//! ```\n\n\
                   import units\n\n/// Twice `x`.\n///\n/// ```\n/// assert_eq(twice(3), 6)\n/// ```\n\
                   pub fn twice(x: Int) -> Int { x * 2 + zero() }\n\n\
                   @test\nfn twice_doubles() { assert_eq(twice(1), 2) }\n";

/// Its second module, with an example in its `//!` docs.
const UNITS: &str = "//! Units of nothing.\n//!\n//! ```\n//! assert_eq(twice(0), 0)\n//! ```\n\n\
                     /// Zero.\npub fn zero() -> Int { 0 }\n";

/// A library whose docs hold no block that runs: a `text` block, and an
/// `ignore`d one that would fail.
const NO_DOC_TESTS: &str = "/// Twice `x`:\n///\n/// ```text\n/// twice(3) is 6\n/// ```\n///\n\
                            /// ```ignore\n/// assert_eq(twice(3), 7)\n/// ```\n\
                            pub fn twice(x: Int) -> Int { x * 2 }\n\n\
                            @test\nfn twice_doubles() { assert_eq(twice(1), 2) }\n";

/// The unit section of a run of `calc`, or of `NO_DOC_TESTS`.
const UNIT_SECTION: &str = "running 1 test\n\
                            test twice_doubles ... ok\n\
                            \n\
                            test result: ok. 1 passed; 0 failed; 0 trapped; 1 total\n";

/// The doc section of a run of `calc` (spec §5.3, §6.3).
const DOC_SECTION: &str = "running 3 doc tests\n\
                           test calc (src/lib.nova:3) ... ok\n\
                           test calc::twice (src/lib.nova:11) ... ok\n\
                           test calc::units (src/units.nova:3) ... ok\n\
                           \n\
                           test result: ok. 3 passed; 0 failed; 0 trapped; 3 total\n";

/// A package `calc` in a fresh directory named for `name`, with `files`.
fn package(name: &str, files: &[(&str, &str)]) -> PathBuf {
    let dir = fresh(name).join("calc");
    write(&dir, &[("nova.toml", MANIFEST)]);
    write(&dir, files);
    dir
}

/// The library `calc`, its root module `lib`.
fn calc(name: &str, lib: &str) -> PathBuf {
    package(name, &[("src/lib.nova", lib), ("src/units.nova", UNITS)])
}

#[test]
fn a_library_runs_its_unit_tests_then_its_doc_tests() {
    let dir = calc("both", LIB);
    let a = nova_test(&dir, &[]).success();
    assert_eq!(stdout(&a), format!("{UNIT_SECTION}\n{DOC_SECTION}"));
    assert_eq!(stderr(&a), "");
}

#[test]
fn a_failing_example_fails_the_run_by_its_name() {
    // Mutant 8 (spec §12.5): a failing doc test reported as passing.
    let dir = calc("failing", &LIB.replace("twice(3), 6", "twice(3), 7"));
    let a = nova_test(&dir, &[]).failure();
    assert_eq!(
        stdout(&a),
        format!(
            "{UNIT_SECTION}\n\
             running 3 doc tests\n\
             test calc (src/lib.nova:3) ... ok\n\
             test calc::twice (src/lib.nova:11) ... FAILED\n\
             \x20   nova: panic: assertion failed: 6 != 7\n\
             test calc::units (src/units.nova:3) ... ok\n\
             \n\
             test result: FAILED. 2 passed; 1 failed; 0 trapped; 3 total\n"
        )
    );
    assert_eq!(
        stderr(&a),
        "Error: 0 tests failed, 0 trapped; 1 doc test failed, 0 trapped\n"
    );
}

#[test]
fn a_compile_error_in_an_example_points_at_its_doc_line() {
    // Spec §7, and mutant 6 (§12.5): the line map off by one line. The
    // unit section runs; the doc build's error stops the run there.
    let dir = calc(
        "compile-error",
        &LIB.replace("twice(3), 6", "twice(3), nope"),
    );
    let a = nova_test(&dir, &[]).failure();
    assert_eq!(stdout(&a), UNIT_SECTION);
    assert_eq!(
        stderr(&a),
        "error[E0001]: cannot find `nope` in this scope\n   \
         ┌─ src/lib.nova:12:25\n   \
         │\n\
         12 │ /// assert_eq(twice(3), nope)\n   \
         │                         ^^^^ here\n\
         \n\
         Error: could not compile due to 1 previous error\n"
    );
}

#[test]
fn an_unknown_word_warns_and_its_block_runs() {
    let lib = LIB.replace(
        "/// ```\n/// assert_eq(twice(3), 6)",
        "/// ```nova,should_panic\n/// assert_eq(twice(3), 6)",
    );
    let dir = calc("unknown-word", &lib);
    let a = nova_test(&dir, &[]).success();
    assert_eq!(
        stderr(&a),
        "warning: src/lib.nova:11: unknown word `should_panic` in a code block's info \
         string; the block runs as a plain test\n"
    );
    assert_eq!(stdout(&a), format!("{UNIT_SECTION}\n{DOC_SECTION}"));
}

#[test]
fn doc_alone_runs_only_the_doc_tests() {
    let dir = calc("doc-alone", LIB);
    let a = nova_test(&dir, &["--doc"]).success();
    assert_eq!(stdout(&a), DOC_SECTION);
    assert_eq!(stderr(&a), "");
}

#[test]
fn doc_outside_a_project_names_std() {
    let dir = fresh("outside");
    let a = nova_test(&dir, &["--doc"]).failure();
    assert_eq!(
        stderr(&a),
        "Error: nova test --doc tests a package's library; run it inside one, or use \
         `nova test --doc --std`\n"
    );
}

#[test]
fn a_programs_blocks_are_not_doc_tests() {
    // Spec §4.1: a package with no library has no doc tests.
    let dir = package(
        "program",
        &[(
            "src/main.nova",
            "/// Starts.\n///\n/// ```\n/// assert(false, \"never run\")\n/// ```\nfn main() {}\n",
        )],
    );
    let a = nova_test(&dir, &["--doc"]).failure();
    assert_eq!(
        stderr(&a),
        "Error: nova test --doc tests a library's docs, and this package has no \
         src/lib.nova\n"
    );
    let a = nova_test(&dir, &[]).success();
    assert_eq!(
        stdout(&a),
        "running 0 tests\n\ntest result: ok. 0 passed; 0 failed; 0 trapped; 0 total\n"
    );
}

#[test]
fn doc_with_no_doc_tests_runs_none_and_passes() {
    let dir = package("doc-none", &[("src/lib.nova", NO_DOC_TESTS)]);
    let a = nova_test(&dir, &["--doc"]).success();
    assert_eq!(
        stdout(&a),
        "running 0 doc tests\n\ntest result: ok. 0 passed; 0 failed; 0 trapped; 0 total\n"
    );
    assert_eq!(stderr(&a), "");
}

#[test]
fn a_filter_can_match_one_doc_test() {
    // Spec §6.1: a filter matches across both kinds.
    let dir = calc("filter-one", LIB);
    let a = nova_test(&dir, &["units"]).success();
    assert_eq!(
        stdout(&a),
        "running 0 tests\n\
         \n\
         test result: ok. 0 passed; 0 failed; 0 trapped; 0 total\n\
         \n\
         running 1 doc test\n\
         test calc::units (src/units.nova:3) ... ok\n\
         \n\
         test result: ok. 1 passed; 0 failed; 0 trapped; 1 total\n"
    );
}

#[test]
fn a_filter_matching_nothing_is_an_error() {
    let dir = calc("filter-none", LIB);
    let a = nova_test(&dir, &["nosuch"]).failure();
    assert_eq!(stdout(&a), "");
    assert_eq!(
        stderr(&a),
        "Error: no test name contains the filter `nosuch` (1 test and 3 doc tests available)\n"
    );
    // Under `--doc`, a unit test's name matches nothing.
    let a = nova_test(&dir, &["--doc", "twice_doubles"]).failure();
    assert_eq!(stdout(&a), "");
    assert_eq!(
        stderr(&a),
        "Error: no test name contains the filter `twice_doubles` (3 doc tests available)\n"
    );
}

#[test]
fn a_library_with_no_doc_tests_prints_what_it_printed_before() {
    // Spec §6.1: no doc section, and today's output and messages.
    let dir = package("no-doc-tests", &[("src/lib.nova", NO_DOC_TESTS)]);
    let a = nova_test(&dir, &[]).success();
    assert_eq!(stdout(&a), UNIT_SECTION);
    assert_eq!(stderr(&a), "");
    let a = nova_test(&dir, &["nosuch"]).failure();
    assert_eq!(
        stderr(&a),
        "Error: no test name contains the filter `nosuch` (1 test available)\n"
    );
    let failing = NO_DOC_TESTS.replace("twice(1), 2", "twice(1), 3");
    write(&dir, &[("src/lib.nova", &failing)]);
    let a = nova_test(&dir, &[]).failure();
    assert_eq!(
        stdout(&a),
        "running 1 test\n\
         test twice_doubles ... FAILED\n\
         \x20   nova: panic: assertion failed: 2 != 3\n\
         \n\
         test result: FAILED. 0 passed; 1 failed; 0 trapped; 1 total\n"
    );
    assert_eq!(stderr(&a), "Error: 1 test failed, 0 trapped\n");
}

#[test]
fn a_warning_is_shown_once() {
    // Spec §6.3: after the unit section, the doc build shows only what is
    // in doc tests' code; under `--doc` alone it shows the rest too.
    let lib = format!(
        "{LIB}\npub fn sign(x: Int) -> Int {{\n    match x {{\n        _ => 1\n        0 => 0\n    }}\n}}\n"
    );
    let dir = calc("warning-once", &lib);
    write(
        &dir,
        &[("nova.toml", &format!("{MANIFEST}colour = \"blue\"\n"))],
    );
    for args in [&[][..], &["--doc"][..]] {
        let a = nova_test(&dir, args).success();
        let err = stderr(&a);
        assert_eq!(err.matches("warning[E0021]").count(), 1, "{args:?}: {err}");
        assert_eq!(err.matches("warning[M0006]").count(), 1, "{args:?}: {err}");
        assert!(stdout(&a).ends_with(DOC_SECTION), "{}", stdout(&a));
    }
}
````

- [ ] **Step 2: Run them to see them fail**

Run: `cd /home/user/nova && cargo test -p nova-cli --test doc_test 2>&1 | grep -E "^    [a-z_]+$|^test result"`
Expected: FAIL, every test but the guard (`--doc` is not an argument
yet, and plain `nova test` prints no doc section):

````text
    a_compile_error_in_an_example_points_at_its_doc_line
    a_failing_example_fails_the_run_by_its_name
    a_filter_can_match_one_doc_test
    a_filter_matching_nothing_is_an_error
    a_library_runs_its_unit_tests_then_its_doc_tests
    a_programs_blocks_are_not_doc_tests
    a_warning_is_shown_once
    an_unknown_word_warns_and_its_block_runs
    doc_alone_runs_only_the_doc_tests
    doc_outside_a_project_names_std
    doc_with_no_doc_tests_runs_none_and_passes
test result: FAILED. 1 passed; 11 failed; 0 ignored; 0 measured; 0 filtered out; finished in …
````

- [ ] **Step 3: The doc section**

In `crates/nova-cli/src/cmd/test.rs`, replace the module doc's last line
and the imports:

````rust
//! both possible (design doc `2026-08-05-nova-test-design.md` §5).

use std::process::{Command, Output};

use anyhow::{Context, Result};
use clap::Args;
````

with:

````rust
//! both possible (design doc `2026-08-05-nova-test-design.md` §5).
//!
//! Phase 3.5b adds the doc tests, the code blocks of a library's docs (spec
//! `docs/superpowers/specs/2026-10-11-phase-3-5b-doc-tests-design.md` §6):
//! each is a module of a second binary, built after the unit tests' from
//! memory, whose tests run the same way and report by the doc tests' names.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use anyhow::{anyhow, bail, Context, Result};
use clap::Args;
use nova_diagnostics::{render, Diagnostic, FileDb, Severity, Span};
use nova_doc::{DocTest, Failure};
use nova_driver::{DiskSources, Program, Sources};

use crate::project::{self, Mode};
````

Then, at the end of `TestCmd`, replace:

````rust
    filter: Option<String>,
}
````

with:

````rust
    filter: Option<String>,

    /// Run only the doc tests: the code examples in the library's docs.
    #[arg(long)]
    doc: bool,
}
````

Then replace the start of `run`, through the first line of its inventory
comment, with the new `run` and the doc section's functions, and the head
of `inventory`, which keeps that comment and the code after it:

````rust
    // around the current directory, or on `src/main.nova` outside any.
    let mode = crate::project::mode(None)?;
    crate::project::sync(&mode)?;
    let program = mode.program(nova_driver::Roots::Test);
    let (exe, tests) = nova_driver::build_test_program(program)?;

    // Enumerate by asking the compiled binary itself — exactly what a plain,
````

with:

````rust
    // around the current directory, or on `src/main.nova` outside any.
    let mode = project::mode(None)?;
    let filter = cmd.filter.as_deref();
    if cmd.doc {
        return run_doc_tests(&mode, filter);
    }
    project::sync(&mode)?;
    let program = mode.program(nova_driver::Roots::Test);
    let (exe, tests) = nova_driver::build_test_program(program)?;
    let compiled: Vec<String> = tests.iter().map(|t| t.name.clone()).collect();
    let names = inventory(&exe, &compiled)?;
    let should_panic: Vec<bool> = tests.iter().map(|t| t.should_panic).collect();

    // Spec 3.5b §6.1: the doc tests follow when the root package has a
    // library whose docs hold at least one. Otherwise nothing changes.
    let library = match &mode {
        Mode::Project { root, .. } if has_library(root) => Some(root.as_path()),
        _ => None,
    };
    let docs = match library {
        Some(root) => library_doc_tests(root)?,
        None => Vec::new(),
    };
    let doc_names: Vec<String> = docs.iter().map(|d| d.name.clone()).collect();

    // A filter matches across both kinds (spec 3.5b §6.1).
    let selected = select(&names, filter);
    let doc_selected = select(&doc_names, filter);
    let mut available = vec![(names.len(), "test")];
    if !docs.is_empty() {
        available.push((docs.len(), "doc test"));
    }
    ensure_matched(filter, selected.len() + doc_selected.len(), &available)?;

    let tally = run_tests(&exe, "test", &names, &should_panic, &selected)?;
    let mut sections = vec![("test", tally)];
    // Both sections run when the unit tests compile, even if some failed
    // (spec 3.5b §6.3), so one run reports everything.
    if let (Some(root), false) = (library, docs.is_empty()) {
        let tally = run_doc_section(root, &docs, &doc_selected, true)?;
        sections.push(("doc test", tally));
    }
    finish(&sections)
}

/// `nova test --doc [filter]` (spec 3.5b §6.1): the doc tests alone, of
/// the library of the package around the current directory.
fn run_doc_tests(mode: &Mode, filter: Option<&str>) -> Result<()> {
    let Mode::Project { root, .. } = mode else {
        bail!(
            "nova test --doc tests a package's library; run it inside one, or use \
             `nova test --doc --std`"
        );
    };
    if !has_library(root) {
        bail!("nova test --doc tests a library's docs, and this package has no src/lib.nova");
    }
    project::sync(mode)?;
    let docs = library_doc_tests(root)?;
    let names: Vec<String> = docs.iter().map(|d| d.name.clone()).collect();
    let selected = select(&names, filter);
    ensure_matched(filter, selected.len(), &[(docs.len(), "doc test")])?;
    let tally = run_doc_section(root, &docs, &selected, false)?;
    finish(&[("doc test", tally)])
}

/// Whether the package at `root` has a library (spec 3.5b §4.1).
fn has_library(root: &Path) -> bool {
    root.join("src").join("lib.nova").is_file()
}

/// The doc tests of the library of the package at `root`, in the site's
/// order. `root` is spelled as the doc tests' program spells it, so a
/// doc test's file is named alike in both (`nova_doc::DocTest::file`).
fn library_doc_tests(root: &Path) -> Result<Vec<DocTest>> {
    Ok(nova_doc::package_doc_tests(root)
        .map_err(failure)?
        .unwrap_or_default())
}

/// A failure to read the doc tests, as the command's error, its
/// diagnostics printed first.
fn failure(f: Failure) -> anyhow::Error {
    match f {
        Failure::Diagnostics { db, diagnostics } => {
            render::emit_all(&db, &diagnostics);
            let errors = diagnostics
                .iter()
                .filter(|d| d.severity == Severity::Error)
                .count();
            anyhow!(
                "could not compile due to {errors} previous error{}",
                if errors == 1 { "" } else { "s" }
            )
        }
        Failure::Other(why) => anyhow!(why),
    }
}

/// The directory, under a package's root, that its doc tests' modules are
/// named in: no file is there (spec 3.5b §11.2), and `<` and `>` keep one
/// from being made on Windows. Each module is `doc<N>.nova`, and the
/// first names the binary.
const DOC_TESTS: &str = "<doc tests>";

/// The doc section (spec 3.5b §6.2, §6.3): the `selected` of `docs`, the
/// doc tests of the package at `root`, compiled into a binary of their own
/// and each run in its own process. `after_units` says the unit section
/// ran: a blank line comes first, and the build shows only what is in doc
/// tests' code, since the unit build showed the rest.
fn run_doc_section(
    root: &Path,
    docs: &[DocTest],
    selected: &[usize],
    after_units: bool,
) -> Result<Tally> {
    for &i in selected {
        warn_unknown_words(&docs[i]);
    }
    // With none selected there is nothing to build: the driver is never
    // given no doc test (spec 3.5b §6.1).
    let (exe, should_panic) = if selected.is_empty() {
        (PathBuf::new(), Vec::new())
    } else {
        build_doc_tests(root, docs, after_units)?
    };
    if after_units {
        println!();
    }
    let names: Vec<String> = docs.iter().map(|d| d.name.clone()).collect();
    run_tests(&exe, "doc test", &names, &should_panic, selected)
}

/// The binary of `docs`, the doc tests of the package at `root`, and
/// whether each of its tests should panic. Its modules are served from
/// memory, and its diagnostics point at the docs (spec 3.5b §7).
fn build_doc_tests(
    root: &Path,
    docs: &[DocTest],
    after_units: bool,
) -> Result<(PathBuf, Vec<bool>)> {
    let dir = root.join(DOC_TESTS);
    let modules: Vec<(PathBuf, String)> = docs
        .iter()
        .enumerate()
        .map(|(i, d)| (dir.join(format!("doc{i}.nova")), d.text.clone()))
        .collect();
    let program = Program::doc_tests(Some(root), modules.iter().map(|(p, _)| p.clone()).collect());
    // A module's name in the program's `FileDb`, and its doc test.
    let by_name: HashMap<String, usize> = modules
        .iter()
        .enumerate()
        .map(|(i, (path, _))| (path.display().to_string(), i))
        .collect();
    // No warning twice (spec 3.5b §6.3): after the unit section, only what
    // is in doc tests' code.
    let map =
        |db: &mut FileDb, d: &mut Diagnostic| to_the_docs(db, d, &by_name, docs) || !after_units;
    let (exe, tests) =
        nova_driver::build_test_program_with(program, &Overlay(&modules), Some(&map))?;
    // Each doc test module declares one test, and the driver collects only
    // that one, in the modules' order: test `i` is doc test `i`.
    anyhow::ensure!(
        tests.len() == docs.len(),
        "the doc tests' binary holds {} tests for {} doc tests; this is a compiler bug",
        tests.len(),
        docs.len()
    );
    let compiled: Vec<String> = tests.iter().map(|t| t.name.clone()).collect();
    inventory(&exe, &compiled)?;
    Ok((exe, tests.iter().map(|t| t.should_panic).collect()))
}

/// The doc tests' modules from memory, and every other file from the disk.
struct Overlay<'a>(&'a [(PathBuf, String)]);

impl Sources for Overlay<'_> {
    fn read(&self, path: &Path) -> std::io::Result<String> {
        match self.0.iter().find(|(p, _)| p == path) {
            Some((_, text)) => Ok(text.clone()),
            None => DiskSources.read(path),
        }
    }
}

/// Moves each label of `d` that is in a doc test's module, which `by_name`
/// finds by the module's name, to the doc comment its code comes from
/// (spec 3.5b §7), and says whether there was one. A moved diagnostic's
/// fixes are dropped: their edits name the module.
fn to_the_docs(
    db: &mut FileDb,
    d: &mut Diagnostic,
    by_name: &HashMap<String, usize>,
    docs: &[DocTest],
) -> bool {
    let mut in_docs = false;
    for label in &mut d.labels {
        let Some(i) = db
            .get_name(label.span.file)
            .and_then(|name| by_name.get(name))
            .copied()
        else {
            continue;
        };
        in_docs = true;
        let test = &docs[i];
        let Some(file) = db.id_of(&test.file) else {
            continue;
        };
        let text = db.get_source(file).unwrap_or_default();
        let start = on_a_char(text, test.map(label.span.start));
        let end = on_a_char(text, test.map(label.span.end)).max(start);
        label.span = Span::new(start, end, file);
    }
    if in_docs {
        d.fixes.clear();
    }
    in_docs
}

/// `offset`, at most `text`'s length, and moved back to a character
/// boundary.
fn on_a_char(text: &str, offset: u32) -> u32 {
    let mut at = (offset as usize).min(text.len());
    while !text.is_char_boundary(at) {
        at -= 1;
    }
    at as u32
}

/// Spec 3.5b §4.2: an unknown word after `nova` warns, naming the block's
/// line, and the block runs as a plain test.
fn warn_unknown_words(test: &DocTest) {
    if test.unknown.is_empty() {
        return;
    }
    let words: Vec<String> = test.unknown.iter().map(|w| format!("`{w}`")).collect();
    eprintln!(
        "warning: {}: unknown word{} {} in a code block's info string; the block runs as a \
         plain test",
        test.place,
        if words.len() == 1 { "" } else { "s" },
        words.join(", ")
    );
}

/// The test names `exe` prints when `NOVA_TEST_INDEX` is unset, checked
/// against `compiled`, the compiler's own list, by count and by position.
fn inventory(exe: &Path, compiled: &[String]) -> Result<Vec<String>> {
    // Enumerate by asking the compiled binary itself — exactly what a plain,
````

Then, in what is now `inventory`, replace:

````rust
    let inventory = Command::new(&exe)
````

with:

````rust
    let inventory = Command::new(exe)
````

Then replace:

````rust
        names.len() == tests.len(),
        "the compiled binary's inventory ({} tests) disagrees with the compiler's own count \
         ({}); this is a compiler bug (inventory process {status})",
        names.len(),
        tests.len()
````

with:

````rust
        names.len() == compiled.len(),
        "the compiled binary's inventory ({} tests) disagrees with the compiler's own count \
         ({}); this is a compiler bug (inventory process {status})",
        names.len(),
        compiled.len()
````

Then replace the end of `inventory`'s checks, the selection and the
filter's check, which become `select` and `ensure_matched`, and the head
of the run loop, which becomes `run_tests`:

````rust
    // *position by position*. The `should_panic`-by-index comment just below
    // used to claim this was already covered ("see the `ensure!` above,
    // which would have already failed"), which was not true: two
    // same-length lists in different orders would satisfy both length checks
    // and then silently misassociate `should_panic` with the wrong test.
    // This is the check that makes that comment's claim actually hold.
    anyhow::ensure!(
        (0..count).all(|i| names[i] == tests[i].name),
        "the compiled binary's inventory names disagree with the compiler's own test names at \
         the same positions; this is a compiler bug (inventory process {status})"
    );

    // should_panic, by position: `names` (read from the binary, above) and
    // `tests` (returned by `build_test_binary`) are both derived from the
    // identical source-ordered list at compile time, so index `i` names the
    // same test in both — see the `ensure!`s above, which would have already
    // failed if that invariant did not hold.
    let selected: Vec<usize> = (0..count)
        .filter(|&i| match &cmd.filter {
            Some(f) => names[i].contains(f.as_str()),
            None => true,
        })
        .collect();

    // Whole-branch review, finding 4: an *explicit* filter that matches
    // nothing is a failure, not a quiet no-op. Before this, `nova test
    // <typo>` printed `running 0 tests` / `test result: ok` and exited 0 —
    // indistinguishable from an unfiltered run of a project with no tests at
    // all, so a typo'd CI filter reported green having run nothing (ADR 0008
    // §2's own residual-gaps list named this). An *unfiltered* run of a file
    // with no tests is deliberately left alone: `count == 0` with
    // `cmd.filter` absent still exits 0, since nothing was mistyped there.
    if let Some(f) = &cmd.filter {
        anyhow::ensure!(
            !selected.is_empty(),
            "no test name contains the filter `{f}` ({count} test{} available)",
            if count == 1 { "" } else { "s" }
        );
    }

    println!(
        "running {} test{}",
````

with:

````rust
    // *position by position*. The `should_panic`-by-index comment in
    // [`run_tests`] used to claim this was already covered ("see the
    // `ensure!` above, which would have already failed"), which was not
    // true: two same-length lists in different orders would satisfy both
    // length checks and then silently misassociate `should_panic` with the
    // wrong test. This is the check that makes that comment's claim
    // actually hold.
    anyhow::ensure!(
        (0..count).all(|i| names[i] == compiled[i]),
        "the compiled binary's inventory names disagree with the compiler's own test names at \
         the same positions; this is a compiler bug (inventory process {status})"
    );
    Ok(names)
}

/// The indexes of `names` that contain `filter`: all of them without one.
fn select(names: &[String], filter: Option<&str>) -> Vec<usize> {
    (0..names.len())
        .filter(|&i| match filter {
            Some(f) => names[i].contains(f),
            None => true,
        })
        .collect()
}

/// Whole-branch review, finding 4: an *explicit* filter that matches
/// nothing is a failure, not a quiet no-op. Before this, `nova test
/// <typo>` printed `running 0 tests` / `test result: ok` and exited 0 —
/// indistinguishable from an unfiltered run of a project with no tests at
/// all, so a typo'd CI filter reported green having run nothing (ADR 0008
/// §2's own residual-gaps list named this). An *unfiltered* run of a file
/// with no tests is deliberately left alone: `count == 0` with no filter
/// still exits 0, since nothing was mistyped there. `matched` counts what
/// the filter selected across every section, and `available` how many
/// tests of each kind there are (spec 3.5b §6.1).
fn ensure_matched(filter: Option<&str>, matched: usize, available: &[(usize, &str)]) -> Result<()> {
    let Some(f) = filter else {
        return Ok(());
    };
    if matched > 0 {
        return Ok(());
    }
    let available: Vec<String> = available
        .iter()
        .map(|&(n, kind)| format!("{n} {kind}{}", if n == 1 { "" } else { "s" }))
        .collect();
    bail!(
        "no test name contains the filter `{f}` ({} available)",
        available.join(" and ")
    )
}

/// A section's failures and traps, which the closing error counts.
struct Tally {
    failed: usize,
    trapped: usize,
}

/// Run the `selected` tests of `exe`, each in its own process, and print
/// their section: `running N {kind}s`, a line per test by its entry in
/// `names`, a blank line, and the result line (spec 3.5b §6.3).
fn run_tests(
    exe: &Path,
    kind: &str,
    names: &[String],
    should_panic: &[bool],
    selected: &[usize],
) -> Result<Tally> {
    println!(
        "running {} {kind}{}",
````

Then, in `run_tests`' loop, replace:

````rust
    for &i in &selected {
        let name = &names[i];
        let result = Command::new(&exe)
            .env("NOVA_TEST_INDEX", i.to_string())
            .output()
            .with_context(|| format!("failed to run {} for test `{name}`", exe.display()))?;
        let should_panic = tests[i].should_panic;
        match (classify(&result), should_panic) {
````

with:

````rust
    for &i in selected {
        let name = &names[i];
        let result = Command::new(exe)
            .env("NOVA_TEST_INDEX", i.to_string())
            .output()
            .with_context(|| format!("failed to run {} for test `{name}`", exe.display()))?;
        // should_panic, by position: the binary's inventory and the
        // compiler's list are both derived from the identical
        // source-ordered list at compile time, so index `i` names the same
        // test in both — see [`inventory`]'s `ensure!`s, which would have
        // already failed if that invariant did not hold.
        match (classify(&result), should_panic[i]) {
````

Then replace:

````rust
    // Deliberately the length of `selected`, not `count` or `tests.len()`:
````

with:

````rust
    // Deliberately the length of `selected`, not the inventory's count:
````

Then, at the end of what was `run`, replace:

````rust
        if all_ok { "ok" } else { "FAILED" }
    );

    if all_ok {
        Ok(())
    } else {
        anyhow::bail!(
            "{failed} test{} failed, {trapped} trapped",
            if failed == 1 { "" } else { "s" }
        );
    }
}
````

with:

````rust
        if all_ok { "ok" } else { "FAILED" }
    );
    Ok(Tally { failed, trapped })
}

/// The run's end (spec 3.5b §6.3): it fails when a test of any section
/// failed or trapped, and the error counts each section's.
fn finish(sections: &[(&str, Tally)]) -> Result<()> {
    if sections
        .iter()
        .all(|(_, t)| t.failed == 0 && t.trapped == 0)
    {
        return Ok(());
    }
    let counts: Vec<String> = sections
        .iter()
        .map(|(kind, t)| {
            format!(
                "{} {kind}{} failed, {} trapped",
                t.failed,
                if t.failed == 1 { "" } else { "s" },
                t.trapped
            )
        })
        .collect();
    bail!("{}", counts.join("; "))
}
````

- [ ] **Step 4: Run the tests**

Run: `cd /home/user/nova && cargo test -p nova-cli --test doc_test 2>&1 | grep -E "^test result|FAILED"`
Expected: `test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out`.

Run: `cd /home/user/nova && env -u RUST_BACKTRACE cargo test -p nova-cli --test run_tests --test doc --test project 2>&1 | grep -E "^test result|FAILED"`
Expected: `test result: ok. 13 passed` (doc), `test result: ok. 24 passed`
(project) and `test result: ok. 307 passed; 0 failed; 1 ignored` (run_tests).
`env -u` is for Task 2's tests only (see Found while prototyping above).

The spec's mutants 6 and 8 (§12.5) run in the final task, on committed
work. On the prototype each failed exactly its test, and nothing else in
`doc_test`:
- 6, the line map off by one line: in `crates/nova-doc/src/doctest.rs`,
  `DocTest::map`'s `match self.lines.get(line) {` made `match
  self.lines.get(line + 1) {` fails
  `a_compile_error_in_an_example_points_at_its_doc_line` (the error points
  at `src/lib.nova:11:5`, the fence, in place of `12:25`).
- 8, a failing doc test reported as passing: in `run_tests`, an arm
  `(Outcome::Panicked { .. }, false) if kind == "doc test" => { passed +=
  1; println!("test {name} ... ok"); }` before the failing one fails
  `a_failing_example_fails_the_run_by_its_name`.

- [ ] **Step 5: Commit**

Run `cargo fmt --all` and `cargo clippy -p nova-cli --all-targets --locked -- -D warnings` (expected: `Finished`).
Stage `crates/nova-cli/src/cmd/test.rs` and `crates/nova-cli/tests/doc_test.rs`.
Message: `feat(nova-cli): nova test runs a library's doc tests, and --doc
alone`, body: After the unit section, a library's doc tests run as a
second section from a binary of their own, built from memory, with their
diagnostics pointing at the doc comments; --doc runs them alone (spec
3.5b §6, §7).

---

### Task 10: `nova test --doc --std` (spec §6.1)

**Files:**
- Modify: `crates/nova-driver/src/lib.rs` (the new `std_source`, and a
  test in `mod doc_test_builds`)
- Modify: `crates/nova-cli/src/cmd/test.rs` (`TestCmd`'s `--std`, `run`,
  the new `run_std_doc_tests`, `doc_tests_alone`, `STD_DOC_TESTS` and
  `doc_file`; `run_doc_section` and `build_doc_tests` take
  `Option<&Path>`; a unit test)
- Test: `crates/nova-cli/tests/doc_test.rs`

**Interfaces:**
- Produces: `pub fn nova_driver::std_source(name: &str) -> Option<&'static
  str>`, at the crate root.
- Consumes: `nova_doc::std_doc_tests` (Task 8) and
  `Program::doc_tests(None, paths)` (Task 6), whose modules import nothing.
- A std doc test's `DocTest.file` is `<std/time>`; its diagnostics move to
  the file named by its `place` less the line, `std/time/lib.nova`, added
  to the database with `std_source`'s text when missing.

- [ ] **Step 1: Write the failing tests**

In `crates/nova-driver/src/lib.rs`, at the end of `mod doc_test_builds`,
replace:

````rust
        assert_eq!(path_fingerprint(&relative), path_fingerprint(&absolute));
    }
}
````

with:

````rust
        assert_eq!(path_fingerprint(&relative), path_fingerprint(&absolute));
    }

    #[test]
    fn std_source_gives_a_std_modules_text_by_its_name_in_the_database() {
        assert_eq!(
            std_source("<std/core>"),
            Some(nova_resolver::STD_MODULES[0].1)
        );
        assert_eq!(
            std_source("<std/time>"),
            Some(nova_resolver::STD_MODULES[10].1)
        );
        assert_eq!(
            std_source("<std/test>"),
            Some(nova_resolver::STD_TEST_MODULE.1)
        );
        assert_eq!(std_source("<std/nope>"), None);
        assert_eq!(std_source("std/time/lib.nova"), None);
        assert_eq!(std_source("src/lib.nova"), None);
    }
}
````

In `crates/nova-cli/src/cmd/test.rs`, at the top of `mod tests`, replace:

````rust
    use super::*;
````

with:

````rust
    use super::*;
    use nova_doc::Origin;

    #[test]
    fn a_std_doc_tests_diagnostic_points_at_its_modules_file() {
        // Spec 3.5b §7: std's diagnostics name `std/<module>/lib.nova`, as
        // the doc test's name does, before the build adds `<std/time>`.
        let time = nova_driver::std_source("<std/time>").expect("std/time");
        let third_line = time.match_indices('\n').nth(1).expect("three lines").0 + 1;
        let test = DocTest {
            name: "std::time (std/time/lib.nova:2)".to_string(),
            place: "std/time/lib.nova:2".to_string(),
            text: "@test fn nova_doctest() {\nlet d = nope\n}\n".to_string(),
            file: "<std/time>".to_string(),
            start: 0,
            lines: vec![
                Origin::Added,
                Origin::Code {
                    at: third_line as u32,
                    cut: None,
                },
                Origin::Added,
            ],
            unknown: Vec::new(),
        };
        let docs = [test];
        let mut db = FileDb::new();
        let module = db.add("<std doc tests>/doc0.nova", docs[0].text.clone());
        let by_name = HashMap::from([("<std doc tests>/doc0.nova".to_string(), 0)]);
        let nope = docs[0].text.find("nope").unwrap() as u32;
        let mut files = Vec::new();
        for _ in 0..2 {
            let mut d = Diagnostic::error("E0001", "cannot find `nope` in this scope")
                .with_primary_label(Span::new(nope, nope + 4, module), "here")
                .with_fix(nova_diagnostics::Fix::new("a fix", Vec::new()));
            assert!(to_the_docs(&mut db, &mut d, &by_name, &docs));
            let span = d.labels[0].span;
            assert_eq!(db.get_name(span.file), Some("std/time/lib.nova"));
            assert_eq!(db.get_source(span.file), Some(time));
            assert_eq!(db.location(span.file, span.start), Some((3, 9)));
            assert_eq!(span.end - span.start, 4);
            assert!(d.fixes.is_empty());
            files.push(span.file);
        }
        assert_eq!(
            files[0], files[1],
            "the second found the file the first added"
        );
        // A label in no doc test's module stays, and says so.
        let lib = db.add("src/lib.nova", "pub fn f() {}\n");
        let mut d = Diagnostic::warning("W0000", "a warning")
            .with_primary_label(Span::new(0, 3, lib), "here");
        assert!(!to_the_docs(&mut db, &mut d, &by_name, &docs));
        assert_eq!(d.labels[0].span, Span::new(0, 3, lib));
    }
````

In `crates/nova-cli/tests/doc_test.rs`, append, at the end of the file:

````rust

#[test]
fn doc_std_runs_stds_doc_tests_inside_a_project_or_not() {
    // Spec §6.1. std has no example until plan Tasks 11 to 14 add them, so
    // this asks only that every one passes; std's gate test asks for one
    // per module.
    for dir in [fresh("std-outside"), calc("std-inside", LIB)] {
        let a = nova_test(&dir, &["--doc", "--std"]).success();
        let out = stdout(&a);
        let count: usize = out
            .strip_prefix("running ")
            .and_then(|rest| rest.split(' ').next())
            .and_then(|n| n.parse().ok())
            .unwrap_or_else(|| panic!("{out}"));
        assert!(
            out.ends_with(&format!(
                "\ntest result: ok. {count} passed; 0 failed; 0 trapped; {count} total\n"
            )),
            "{out}"
        );
        assert!(!out.contains("calc"), "{out}");
        assert_eq!(stderr(&a), "");
    }
}

#[test]
fn std_needs_doc() {
    let dir = fresh("std-without-doc");
    let a = nova_test(&dir, &["--std"]).failure();
    assert_eq!(a.get_output().status.code(), Some(2), "a usage error");
    assert!(
        stderr(&a)
            .starts_with("error: the following required arguments were not provided:\n  --doc\n"),
        "{}",
        stderr(&a)
    );
}
````

- [ ] **Step 2: Run them to see them fail**

Run: `cd /home/user/nova && cargo test -p nova-driver --lib doc_test_builds 2>&1 | grep -E "^error" | sort | uniq -c`
Expected: FAIL to compile:

````text
      1 error: could not compile `nova-driver` (lib test) due to 6 previous errors
      6 error[E0425]: cannot find function `std_source` in this scope
````

Run: `cd /home/user/nova && cargo test -p nova-cli --bin nova 2>&1 | grep -E "^error"`
Expected: FAIL to compile: ``error[E0425]: cannot find function `std_source` in crate `nova_driver` ``.

Run: `cd /home/user/nova && cargo test -p nova-cli --test doc_test 2>&1 | grep -E "^    [a-z_]+$|^test result"`
Expected: FAIL, as `nova` says ``error: unexpected argument '--std' found``:

````text
    doc_std_runs_stds_doc_tests_inside_a_project_or_not
    std_needs_doc
test result: FAILED. 12 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out; finished in …
````

- [ ] **Step 3: std's doc tests**

In `crates/nova-driver/src/lib.rs`, after `MapDiagnostic`, replace:

````rust
pub type MapDiagnostic<'a> = dyn Fn(&mut FileDb, &mut Diagnostic) -> bool + 'a;
````

with:

````rust
pub type MapDiagnostic<'a> = dyn Fn(&mut FileDb, &mut Diagnostic) -> bool + 'a;

/// The text of the std module a program's `FileDb` names `name`, such as
/// `<std/time>`, or `None` for any other name. The build adds std's modules
/// to the database only after loading, so a [`MapDiagnostic`] that shows a
/// diagnostic in one, as a std doc test's does (spec 3.5b §7), reads its
/// text here.
pub fn std_source(name: &str) -> Option<&'static str> {
    let short = name.strip_prefix("<std/")?.strip_suffix('>')?;
    nova_resolver::STD_MODULES
        .iter()
        .chain(std::iter::once(&nova_resolver::STD_TEST_MODULE))
        .find(|(module, _)| module.strip_prefix("$std.") == Some(short))
        .map(|&(_, text)| text)
}
````

In `crates/nova-cli/src/cmd/test.rs`, replace the `nova_diagnostics`
import:

````rust
use nova_diagnostics::{render, Diagnostic, FileDb, Severity, Span};
````

with:

````rust
use nova_diagnostics::{render, Diagnostic, FileDb, FileId, Severity, Span};
````

Then, at the end of `TestCmd`, replace:

````rust
    doc: bool,
}
````

with:

````rust
    doc: bool,

    /// With --doc: run std's doc tests instead, inside a project or not.
    #[arg(long, requires = "doc")]
    std: bool,
}
````

Then, in `run`, replace:

````rust
    let mode = project::mode(None)?;
    let filter = cmd.filter.as_deref();
````

with:

````rust
    let filter = cmd.filter.as_deref();
    if cmd.std {
        return run_std_doc_tests(filter);
    }
    let mode = project::mode(None)?;
````

Then replace:

````rust
        let tally = run_doc_section(root, &docs, &doc_selected, true)?;
````

with:

````rust
        let tally = run_doc_section(Some(root), &docs, &doc_selected, true)?;
````

Then, at the end of `run_doc_tests`, replace:

````rust
    let docs = library_doc_tests(root)?;
    let names: Vec<String> = docs.iter().map(|d| d.name.clone()).collect();
    let selected = select(&names, filter);
    ensure_matched(filter, selected.len(), &[(docs.len(), "doc test")])?;
    let tally = run_doc_section(root, &docs, &selected, false)?;
````

with:

````rust
    doc_tests_alone(Some(root), &library_doc_tests(root)?, filter)
}

/// `nova test --doc --std [filter]` (spec 3.5b §6.1): std's doc tests,
/// inside a project or not.
fn run_std_doc_tests(filter: Option<&str>) -> Result<()> {
    let docs = nova_doc::std_doc_tests().map_err(failure)?;
    doc_tests_alone(None, &docs, filter)
}

/// The doc section alone, of `docs`: the doc tests of the package at
/// `package`, or std's.
fn doc_tests_alone(package: Option<&Path>, docs: &[DocTest], filter: Option<&str>) -> Result<()> {
    let names: Vec<String> = docs.iter().map(|d| d.name.clone()).collect();
    let selected = select(&names, filter);
    ensure_matched(filter, selected.len(), &[(docs.len(), "doc test")])?;
    let tally = run_doc_section(package, docs, &selected, false)?;
````

Then, before `run_doc_section`, replace its doc comment and first
parameter:

````rust
/// The doc section (spec 3.5b §6.2, §6.3): the `selected` of `docs`, the
/// doc tests of the package at `root`, compiled into a binary of their own
/// and each run in its own process. `after_units` says the unit section
/// ran: a blank line comes first, and the build shows only what is in doc
/// tests' code, since the unit build showed the rest.
fn run_doc_section(
    root: &Path,
````

with:

````rust
/// The directory, under the current directory, that std's doc tests'
/// modules are named in, as [`DOC_TESTS`] is for a package's.
const STD_DOC_TESTS: &str = "<std doc tests>";

/// The doc section (spec 3.5b §6.2, §6.3): the `selected` of `docs`, the
/// doc tests of the package at `package`, or std's, compiled into a binary
/// of their own and each run in its own process. `after_units` says the
/// unit section ran: a blank line comes first, and the build shows only
/// what is in doc tests' code, since the unit build showed the rest.
fn run_doc_section(
    package: Option<&Path>,
````

Then, in `run_doc_section`, replace:

````rust
        build_doc_tests(root, docs, after_units)?
````

with:

````rust
        build_doc_tests(package, docs, after_units)?
````

Then replace `build_doc_tests`' doc comment and head:

````rust
/// The binary of `docs`, the doc tests of the package at `root`, and
/// whether each of its tests should panic. Its modules are served from
/// memory, and its diagnostics point at the docs (spec 3.5b §7).
fn build_doc_tests(
    root: &Path,
    docs: &[DocTest],
    after_units: bool,
) -> Result<(PathBuf, Vec<bool>)> {
    let dir = root.join(DOC_TESTS);
````

with:

````rust
/// The binary of `docs`, the doc tests of the package at `package`, or
/// std's, and whether each of its tests should panic. Its modules are
/// served from memory, and its diagnostics point at the docs (spec 3.5b
/// §7).
fn build_doc_tests(
    package: Option<&Path>,
    docs: &[DocTest],
    after_units: bool,
) -> Result<(PathBuf, Vec<bool>)> {
    let dir = match package {
        Some(root) => root.join(DOC_TESTS),
        None => PathBuf::from(STD_DOC_TESTS),
    };
````

Then, in `build_doc_tests`, replace:

````rust
    let program = Program::doc_tests(Some(root), modules.iter().map(|(p, _)| p.clone()).collect());
````

with:

````rust
    let program = Program::doc_tests(package, modules.iter().map(|(p, _)| p.clone()).collect());
````

Then, in `to_the_docs`, replace:

````rust
        let Some(file) = db.id_of(&test.file) else {
````

with:

````rust
        let Some(file) = doc_file(db, test) else {
````

Then, at the end of `to_the_docs`, replace:

````rust
    in_docs
}
````

with:

````rust
    in_docs
}

/// The file a doc test's block is in, in `db`. A package's is there, by
/// the name its analysis gave it, which the doc tests' program reads it
/// under. A std module's is named `std/<module>/lib.nova`, as the doc
/// test's name shows it (spec 3.5b §7), and is added from std's sources
/// when missing: the build adds std's modules, as `<std/time>` and so on,
/// only after loading.
fn doc_file(db: &mut FileDb, test: &DocTest) -> Option<FileId> {
    let Some(text) = nova_driver::std_source(&test.file) else {
        return db.id_of(&test.file);
    };
    let (name, _) = test.place.rsplit_once(':')?;
    Some(db.id_of(name).unwrap_or_else(|| db.add(name, text)))
}
````

- [ ] **Step 4: Run the tests**

Run: `cd /home/user/nova && cargo test -p nova-driver --lib 2>&1 | grep -E "^test result|FAILED"`
Expected: `test result: ok. 36 passed; 0 failed`.

Run: `cd /home/user/nova && cargo test -p nova-cli --bin nova 2>&1 | grep -E "^test result|FAILED"`
Expected: `test result: ok. 2 passed; 0 failed`.

Run: `cd /home/user/nova && cargo test -p nova-cli --test doc_test 2>&1 | grep -E "^test result|FAILED"`
Expected: `test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out`.

Run: `cd /home/user/nova && cargo build -q -p nova-cli && rm -rf $P/home-c2 && NOVA_HOME=$P/home-c2 target/debug/nova test --doc --std`
Expected, until Task 11 adds std's first examples:

````text
running 0 doc tests

test result: ok. 0 passed; 0 failed; 0 trapped; 0 total
````

- [ ] **Step 5: Commit**

Run `cargo fmt --all` and `cargo clippy -p nova-cli -p nova-driver --all-targets --locked -- -D warnings` (expected: `Finished`).
Stage `crates/nova-cli/src/cmd/test.rs`, `crates/nova-cli/tests/doc_test.rs` and `crates/nova-driver/src/lib.rs`.
Message: `feat(nova-cli): nova test --doc --std runs std's doc tests`,
body: --std, which needs --doc, runs std's doc tests inside a project or
not, and their diagnostics name std/<module>/lib.nova, read through the
driver's new std_source (spec 3.5b §6.1, §7).

---

### Task 11: The completeness condition, and std's examples: core, collections, strings, bytes, fmt

**Files:**
- Modify: `std/core/lib.nova`, `std/collections/lib.nova`, `std/strings/lib.nova`, `std/bytes/lib.nova`, `std/fmt/lib.nova`
- Modify: `crates/nova-doc/tests/std_docs.rs` (the new test)
- Modify: any file whose comment cites a line these files move (Step 4)

- [ ] **Step 1: The completeness condition (spec §9.2)**

Append to `crates/nova-doc/tests/std_docs.rs`:

```rust
/// The modules whose examples are written (plan Tasks 11 to 13); Task 14
/// deletes this, and the test then checks every module.
const EXAMPLED: &[&str] = &["core", "collections", "strings", "bytes", "fmt"];

/// Each std module's `//!` docs hold an example that runs (spec 3.5b §9.2).
#[test]
fn every_std_module_has_an_example() {
    let mut missing = Vec::new();
    for (name, text) in nova_std::ALL {
        if !EXAMPLED.contains(&name) {
            continue;
        }
        let tested = nova_doc::code_blocks(&common::module_docs(text))
            .iter()
            .any(|b| matches!(b.block, nova_doc::Block::Test { .. }));
        if !tested {
            missing.push(format!("std/{name}"));
        }
    }
    assert!(
        missing.is_empty(),
        "{} module(s) without an example in their `//!` docs: {}",
        missing.len(),
        missing.join(", ")
    );
}
```

Run: `cd /home/user/nova && cargo test -p nova-doc --test std_docs 2>&1 | grep -E "module\(s\)|test result"`
Expected: FAIL, the modules in `nova_std::ALL`'s order, which is alphabetical (`std_is_documented` passes):

```text
5 module(s) without an example in their `//!` docs: std/bytes, std/collections, std/core, std/fmt, std/strings
test result: FAILED. 1 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in …
```

- [ ] **Step 2: The module examples**

For each module, append the lines below after the last `//!` line of the file's module docs (the block of `//!` lines at its top), keeping every line as written. Each block was run as a doc test runs it on 2026-10-11, three times, with no failure, and its code is laid out as `nova fmt` lays it out (a block that is not a `fn main` module checked as the body of one).

`std/core/lib.nova`:

```nova
//!
//! An absent value, a failure, and an iterator's lazy `filter` and `map`,
//! run by `collect`:
//!
//! ```
//! // `Option` holds a value that may be absent.
//! let port: Option<Int> = Some(8080)
//! assert_eq(port.map(|p| p + 1).unwrap_or(80), 8081)
//! let missing: Option<Int> = None
//! assert_eq(missing.unwrap_or(80), 80)
//!
//! // `Result` is the outcome of something that can fail.
//! let checked: Result<Int, String> = missing.ok_or("no port given")
//! assert(checked.is_err(), "None became an Err")
//!
//! // An iterator's `filter` and `map` are lazy; `collect` runs them.
//! let mut nums: Vec<Int> = Vec::new()
//! for n in 1..6 { nums.push(n) }
//! let tens = nums.iter().filter(|n| n % 2 == 0).map(|n| n * 10).collect()
//! assert_eq(tens.len(), 2)
//! assert_eq(tens.get(1).unwrap(), 40)
//! ```
```

`std/collections/lib.nova`:

```nova
//!
//! A `Vec`, a `Map` and a `Set` in use:
//!
//! ```
//! let mut scores: Vec<Int> = Vec::new()
//! scores.push(90)
//! scores.push(75)
//! assert_eq(scores.len(), 2)
//! assert(scores.get(5).is_none(), "an index past the end gives None")
//!
//! let mut ages: Map<String, Int> = Map::new()
//! ages.insert("ada", 36)
//! assert_eq(ages.get("ada").unwrap(), 36)
//! assert(!ages.contains_key("alan"), "alan was never inserted")
//!
//! let mut seen: Set<String> = Set::new()
//! assert(seen.insert("nova"), "the first insert adds the value")
//! assert(!seen.insert("nova"), "a repeat is not added again")
//! assert_eq(seen.len(), 1)
//! ```
```

`std/strings/lib.nova`:

```nova
//!
//! Trimming, splitting, joining and searching strings, where every index
//! and length counts characters:
//!
//! ```
//! let line = "  name = Nova  "
//! let parts = line.trim().split(" = ")
//! assert_eq(parts.len(), 2)
//! assert_eq(parts[0], "name")
//! assert_eq(parts[1].to_upper(), "NOVA")
//! assert_eq(", ".join(["a", "b", "c"]), "a, b, c")
//!
//! // Indexes and lengths count characters, not bytes.
//! let word = "héllo"
//! assert_eq(word.len(), 5)
//! assert_eq(bytes_from_string(word).len(), 6)
//! assert_eq(word.slice(1, 3), "él")
//! assert_eq(word.index_of("llo").unwrap(), 2)
//! assert(word.starts_with("hé"), "starts_with compares characters")
//! ```
```

`std/bytes/lib.nova`:

```nova
//!
//! Making `Bytes` from a string and from integers, measuring, slicing,
//! joining and searching them, and turning them back into a `String`:
//!
//! ```
//! let greeting = bytes_from_string("hi!")
//! assert_eq(greeting.len(), 3)
//! assert_eq(greeting.byte_at(0).unwrap(), 104) // 'h'
//! assert_eq(greeting.slice(1, 99).to_string().unwrap(), "i!") // bounds are clamped
//!
//! let line = greeting.concat(bytes_from_ints([10])) // a line feed
//! assert(line.eq(bytes_from_string("hi!\n")), "equal byte for byte")
//! assert_eq(line.index_of(bytes_from_string("!")).unwrap(), 2)
//!
//! // Bytes carry no encoding: these two are not valid UTF-8.
//! let raw = bytes_from_ints([0xff, 0xfe])
//! assert(raw.to_string().is_none(), "not UTF-8, so no String")
//! ```
```

`std/fmt/lib.nova`:

```nova
//!
//! Padding integers and strings to a width, and writing a float to a fixed
//! number of places:
//!
//! ```
//! assert_eq(7.pad(3), "007")
//! assert_eq((-5).pad(3), "-05")
//! assert_eq(1234.pad(2), "1234") // never truncated
//! assert_eq("ab".pad_left(4), "  ab")
//! assert_eq("ab".pad_right(4), "ab  ")
//! assert_eq((100.0 / 3.0).fixed(2), "33.33")
//! assert_eq((2.5).fixed(0), "2") // a tie goes to the even digit
//!
//! // A row of a table, in columns of fixed width.
//! let row = "${"apples".pad_right(8)}|${3.pad(4)}|${(0.5).fixed(2).pad_left(6)}"
//! assert_eq(row, "apples  |0003|  0.50")
//! ```
```

- [ ] **Step 3: The examples on single items (spec §9.1)**

For each, append the lines after the last `///` line of the named item's docs, at that item's indentation (four spaces more inside an `impl` or a `trait`). A line written `# …` is hidden on the site and runs less its `# ` (spec §4.3).

`Iterator`, above `pub trait Iterator` in `std/core/lib.nova`:

```nova
///
/// The adapters are lazy; a consumer runs them:
///
/// ```
/// let mut words: Vec<String> = Vec::new()
/// words.push("map")
/// words.push("filter")
/// words.push("fold")
///
/// // `filter` and `map` do nothing until a consumer such as `fold` runs them.
/// // A closure that calls a method on its parameter names the parameter's type.
/// let lengths = words.iter().filter(|w: String| w.starts_with("f")).map(|w: String| w.len())
/// assert_eq(lengths.fold(0, |acc, n| acc + n), 10)
/// assert(words.iter().any(|w| w == "map"), "`any` stops at the first match")
/// ```
```

`Hash`, above `pub trait Hash` in `std/core/lib.nova`:

```nova
///
/// A record of your own as a `Map` key, with `Eq` and `Hash` that
/// agree:
///
/// ```
/// record Point { x: Int, y: Int }
///
/// impl Eq for Point {
///     fn eq(self, other: Point) -> Bool { self.x == other.x && self.y == other.y }
/// }
///
/// // Points equal by `eq` must hash alike, so combine the fields' hashes.
/// impl Hash for Point {
///     fn hash(self) -> Int { self.x.hash() * 31 + self.y.hash() }
/// }
///
/// fn main() {
///     let mut names: Map<Point, String> = Map::new()
///     names.insert(Point { x: 0, y: 0 }, "origin")
///     assert_eq(names.get(Point { x: 0, y: 0 }).unwrap(), "origin")
///     assert(!names.contains_key(Point { x: 1, y: 0 }), "a different point")
/// }
/// ```
```

`Map::get`, above `Map`'s `pub fn get` in `std/collections/lib.nova`:

```nova
///
/// Telling a missing key from a present one:
///
/// ```
/// let mut stock: Map<String, Int> = Map::new()
/// stock.insert("apples", 3)
///
/// // `get` returns an `Option`: `None` for a key that is not in the map.
/// let pears = match stock.get("pears") {
///     Some(n) => n
///     None => 0
/// }
/// assert_eq(pears, 0)
/// assert_eq(stock.get("apples").unwrap_or(0), 3)
///
/// // `insert` returns the value the key had before, if any.
/// assert_eq(stock.insert("apples", 5).unwrap(), 3)
/// assert(stock.insert("pears", 1).is_none(), "pears is new")
/// assert_eq(stock.len(), 2)
/// ```
```

`Vec`, above `pub record Vec` in `std/collections/lib.nova`:

```nova
///
/// A `Vec` is shared, not copied, when it is assigned:
///
/// ```
/// let mut a: Vec<Int> = Vec::new()
/// a.push(1)
///
/// // Assigning a Vec shares it: `b` is a second name for the same vector.
/// let mut b = a
/// b.push(2)
/// assert_eq(a.len(), 2)
///
/// // For an independent copy, build a new Vec from the values.
/// let mut copy: Vec<Int> = Vec::new()
/// for x in a.iter() { copy.push(x) }
/// copy.push(3)
/// assert_eq(a.len(), 2)
/// assert_eq(copy.len(), 3)
/// ```
```

`String::split`, above `pub fn split` in `std/strings/lib.nova`:

```nova
///
/// Nothing is collapsed or trimmed:
///
/// ```
/// // Nothing is collapsed or trimmed: adjacent separators give an empty piece.
/// let pieces = "a,,b,".split(",")
/// assert_eq(pieces.len(), 4)
/// assert_eq(pieces[1], "")
/// assert_eq(pieces[3], "")
/// assert_eq(",".join(pieces), "a,,b,") // `join` undoes `split`
///
/// assert_eq("".split(",").len(), 1) // one piece: the empty string
/// assert_eq("abc".split("").len(), 3) // an empty separator gives the characters
/// assert_eq("".split("").len(), 0)
/// ```
```

- [ ] **Step 4: Line numbers cited in comments**

The examples move these files' lines. As 3.5a's Tasks 15 to 19 did, re-point every live citation of a line in the files this task changes, in this commit: `git grep -n -E "(core|collections|strings|bytes|fmt)/lib\.nova:[0-9]|lib\.nova:[0-9]+|line [0-9]+" -- crates docs nova-spec std CHANGELOG.md`, then read each cited spot to tell which file a bare `lib.nova:N` or "line N" means, find the code it describes, re-point it, print the new line to check it, and ledger it. Dated records (ADRs, older specs and plans, released CHANGELOG entries) keep theirs.

Expected (measured on 2026-10-11 with this task's lines inserted; each old line was checked to be what its comment describes, and each new one printed), these, in three files that this task's examples do not move:

- `nova-spec/20-STDLIB.md:200`: `std/core/lib.nova:198` to `:220`, `pub trait Display`.
- `nova-spec/20-STDLIB.md:1798`: `std/strings/lib.nova:289-300` to `:324-335`, `join`'s comment through its `pub fn join`.
- `nova-spec/20-STDLIB.md:2158`: `std/collections/lib.nova:78` to `:117`, `Vec`'s `pub fn get`; `:86` to `:125`, its `pub fn set`.
- `nova-spec/20-STDLIB.md:2382`: `std/fmt/lib.nova:45` to `:62`, `if len >= width { return s }`; `:42` to `:59`, `pad`'s signature.
- `std/sync/lib.nova:123-124`: `std/core/lib.nova:67` to `:89`, `Option`'s `pub fn unwrap`; and on line 124, which the grep does not match, the five lines that call it, "430, 460, 478, 541 and 577" to "490, 520, 538, 601 and 637" (each holds `n.unwrap()`, the last twice; the `.unwrap()` calls in the new examples are doc lines, not std/core's code, so the counts stand).
- `std/sync/lib.nova:193`: `std/collections/lib.nova:78` to `:117` and `:86` to `:125`, as above.
- `std/sync/lib.nova:237-238`: `std/core/lib.nova:263` to `:285`, the "`hash % cap` can be *negative*" bullet; `std/collections/lib.nova:164` to `:203`, the line beginning "`hash % cap`. A hash may be negative".
- `crates/nova-cli/tests/run_tests.rs:2475` (std/strings, cited bare): `lib.nova:265` to `:300`, pass 1's `i = i + s.len()`; `lib.nova:279` to `:314`, pass 2's `j = j + s.len()`.
- `crates/nova-cli/tests/run_tests.rs:2605-2606` (std/strings, cited bare): `lib.nova:110` to `:130`, `trim_start_index`'s `cs.len()`; `lib.nova:118` to `:138`, `trim_end_index`'s `floor`.
- `crates/nova-cli/tests/run_tests.rs:2889`: `std/collections/lib.nova:88` to `:127`, `Vec::set`'s `i >= self.len` guard; "line 87's" to "line 126's", its `i < 0` guard.

Left alone: `CHANGELOG.md:1714` (`std/collections/lib.nova:49`, `:53`), under the released `[0.2.0-alpha.2]` entry, and the dated plans, specs and ADRs. The commit body names the three files whose citations were re-pointed.

- [ ] **Step 5: Run**

- `cd /home/user/nova && cargo test -p nova-doc --test std_docs 2>&1 | grep "test result"`. Expected: `ok. 2 passed`.
- `cargo run -q -p nova-cli -- fmt --check std`. Expected: no output.
- `cargo build -q -p nova-cli && NOVA_HOME=$P/home-std target/debug/nova test --doc --std 2>&1 | tail -4`, with `$P/home-std` a fresh directory. Expected: `test result: ok. 10 passed; 0 failed; 0 trapped; 10 total` (5 module examples and 5 on single items so far). Without `tail`, the run lists each doc test by name, which shows each example landed on its item; this task's, among the earlier tasks' (the modules alphabetically, `String::split` last):

  ```text
  test std::bytes (std/bytes/lib.nova:14) ... ok
  test std::collections (std/collections/lib.nova:17) ... ok
  test std::collections::Vec (std/collections/lib.nova:54) ... ok
  test std::collections::Map::get (std/collections/lib.nova:406) ... ok
  test std::core (std/core/lib.nova:14) ... ok
  test std::core::Hash (std/core/lib.nova:305) ... ok
  test std::core::Iterator (std/core/lib.nova:388) ... ok
  test std::fmt (std/fmt/lib.nova:12) ... ok
  test std::strings (std/strings/lib.nova:14) ... ok
  test std::String::split (std/strings/lib.nova:273) ... ok
  ```

- [ ] **Step 6: Commit:** `docs: std's examples: core, collections, strings, bytes, fmt`, body: spec 3.5b §9.

---

### Task 12: std's examples: io, fs, net, process

**Files:**
- Modify: `std/io/lib.nova`, `std/fs/lib.nova`, `std/net/lib.nova`, `std/process/lib.nova`
- Modify: `crates/nova-doc/tests/std_docs.rs` (`EXAMPLED`)
- Modify: any file whose comment cites a line these files move (Step 4)

- [ ] **Step 1:** `EXAMPLED` gains `"io"`, `"fs"`, `"net"`, `"process"`. That takes its line past rustfmt's width, so it is written one name to a line, as `cargo fmt` lays it out:

```rust
const EXAMPLED: &[&str] = &[
    "core",
    "collections",
    "strings",
    "bytes",
    "fmt",
    "io",
    "fs",
    "net",
    "process",
];
```

Run Task 11 Step 1's command. Expected: FAIL, the modules in `nova_std::ALL`'s order:

```text
4 module(s) without an example in their `//!` docs: std/fs, std/io, std/net, std/process
test result: FAILED. 1 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in …
```

- [ ] **Step 2: The module examples**

For each module, append the lines below after the last `//!` line of the file's module docs (the block of `//!` lines at its top), keeping every line as written. Each block was run as a doc test runs it on 2026-10-11, three times, with no failure, and its code is laid out as `nova fmt` lays it out (a block that is not a `fn main` module checked as the body of one).

`std/io/lib.nova`:

```nova
//!
//! Writing to standard output through `Write`, and telling an `IoError` by
//! its kind:
//!
//! ```
//! async fn main() {
//!     // `write` returns how many bytes it took, which can be fewer than given.
//!     let out = stdout()
//!     let n = out.write(bytes_from_string("hello\n")).await.unwrap()
//!     assert(n >= 1 && n <= 6, "some of the six bytes were written")
//!     out.flush().await.unwrap()
//!
//!     // A failure is an `IoError`: match on its `kind`, not its `message`.
//!     let err = match read_to_string("${temp_dir()}/nova-std-io/absent.txt").await {
//!         Ok(_) => panic("nothing should be there")
//!         Err(e) => e
//!     }
//!     let not_found = match err.kind {
//!         NotFound => true
//!         _ => false
//!     }
//!     assert(not_found, "expected NotFound, got: ${err.message}")
//! }
//! ```
```

`std/fs/lib.nova`:

```nova
//!
//! Writing a file in a directory of its own under `temp_dir()`, reading it
//! back, listing the directory, and removing both:
//!
//! ```
//! async fn main() {
//!     let dir = "${temp_dir()}/nova-std-fs"
//!     create_dir_all(dir).await.unwrap()
//!     let path = "${dir}/notes.txt"
//!     write_string(path, "first line\n").await.unwrap()
//!     assert_eq(read_to_string(path).await.unwrap(), "first line\n")
//!
//!     let entries = read_dir(dir).await.unwrap()
//!     assert_eq(entries.len(), 1)
//!     assert_eq(entries[0].name, "notes.txt")
//!     assert(entries[0].is_file, "notes.txt is a file")
//!
//!     remove_dir_all(dir).await.unwrap()
//!     assert(!exists(dir).await, "the directory and its file are gone")
//! }
//! ```
```

`std/net/lib.nova`:

```nova
//!
//! A connection over the loopback address: a listener on a port the system
//! picks, a client that connects to it, and a message read to its end:
//!
//! ```
//! async fn main() {
//!     let listener = bind("127.0.0.1:0").unwrap() // port 0: the system picks one
//!     let addr = "127.0.0.1:${listener.local_port().unwrap()}"
//!     let client = connect(addr).await.unwrap()
//!     let server = listener.accept().await.unwrap()
//!
//!     client.write(bytes_from_string("ping")).await.unwrap()
//!     client.close().await.unwrap() // the server's reads now reach the end
//!     let got = server.read_to_end().await.unwrap()
//!     assert_eq(got.to_string().unwrap(), "ping")
//!
//!     server.close().await.unwrap()
//!     listener.close().await.unwrap()
//! }
//! ```
```

`std/process/lib.nova`:

```nova
//!
//! Reading the program's arguments. `exit` ends the program at once, so the
//! example does not call it:
//!
//! ```
//! let argv = args()
//! // Index 0 names the program; the arguments it was given start at index 1.
//! assert(argv.len() >= 1, "index 0 is always there")
//! let name = argv.get(1).unwrap_or("world") // `get` gives None past the end
//! println("hello, ${name}")
//!
//! // Each call returns a new Vec, so changing one changes no other.
//! let mut mine = args()
//! mine.push("--verbose")
//! assert_eq(args().len(), argv.len())
//! ```
```

- [ ] **Step 3: The examples on single items (spec §9.1)**

For each, append the lines after the last `///` line of the named item's docs, at that item's indentation (four spaces more inside an `impl` or a `trait`). A line written `# …` is hidden on the site and runs less its `# ` (spec §4.3).

`Write::write`, above `Write`'s `fn write` (the trait member, not an impl's) in `std/io/lib.nova`:

```nova
///
/// Writing all of some bytes, a partial write at a time:
///
/// ```
/// // `write` can take fewer bytes than it is given, so writing all of them
/// // means calling it again with the rest.
/// async fn write_all(f: File, data: Bytes) {
///     let mut rest = data
///     while rest.len() > 0 {
///         let n = f.write(rest).await.unwrap()
///         rest = rest.slice(n, rest.len())
///     }
/// }
///
/// async fn main() {
///     let dir = "${temp_dir()}/nova-std-io"
///     create_dir_all(dir).await.unwrap()
///     let f = open("${dir}/out.txt", OpenOptions::writing()).await.unwrap()
///     write_all(f, bytes_from_string("line\n".repeat(1000))).await
///     f.close().await.unwrap()
///     assert_eq(read("${dir}/out.txt").await.unwrap().len(), 5000)
///     # remove_dir_all(dir).await.unwrap()
/// }
/// ```
```

`open`, above `pub async fn open` in `std/fs/lib.nova`:

```nova
///
/// Appending to a file, then reading it back:
///
/// ```
/// async fn main() {
///     let dir = "${temp_dir()}/nova-std-fs-open"
///     create_dir_all(dir).await.unwrap()
///     let path = "${dir}/log.txt"
///     write_string(path, "one\n").await.unwrap()
///
///     // `appending()` keeps what the file holds and adds each write at its end.
///     let log = open(path, OpenOptions::appending()).await.unwrap()
///     log.write(bytes_from_string("two\n")).await.unwrap()
///     log.close().await.unwrap() // nothing closes a File for you
///
///     let reader = open(path, OpenOptions::reading()).await.unwrap()
///     let text = reader.read_to_end().await.unwrap().to_string().unwrap()
///     reader.close().await.unwrap()
///     assert_eq(text, "one\ntwo\n")
///     # remove_dir_all(dir).await.unwrap()
/// }
/// ```
```

`TcpListener::accept`, above `TcpListener`'s `pub async fn accept` in `std/net/lib.nova`:

```nova
///
/// Accepting in a task of its own, so the client can connect from the
/// first:
///
/// ```
/// async fn serve_one(listener: TcpListener) {
///     let conn = listener.accept().await.unwrap()
///     let asked = conn.read(64).await.unwrap()
///     assert_eq(asked.to_string().unwrap(), "ping")
///     conn.write(bytes_from_string("pong")).await.unwrap()
///     conn.close().await.unwrap()
/// }
///
/// async fn main() {
///     let listener = bind("127.0.0.1:0").unwrap()
///     let port = listener.local_port().unwrap()
///     let server = spawn(serve_one(listener)) // accept in a task of its own
///     let client = connect("127.0.0.1:${port}").await.unwrap()
///     client.write(bytes_from_string("ping")).await.unwrap()
///     let reply = client.read_to_end().await.unwrap() // until the server closes
///     assert_eq(reply.to_string().unwrap(), "pong")
///     server.join().await
///     listener.close().await.unwrap()
/// }
/// ```
```

- [ ] **Step 4: Line numbers cited in comments**

The examples move these files' lines. As 3.5a's Tasks 15 to 19 did, re-point every live citation of a line in the files this task changes, in this commit: `git grep -n -E "(io|fs|net|process)/lib\.nova:[0-9]|lib\.nova:[0-9]+|line [0-9]+" -- crates docs nova-spec std CHANGELOG.md`, then read each cited spot to tell which file a bare `lib.nova:N` or "line N" means, find the code it describes, re-point it, print the new line to check it, and ledger it. Dated records (ADRs, older specs and plans, released CHANGELOG entries) keep theirs.

Expected: none to re-point. The grep's hits into std/io, std/fs and std/net are dated records (`docs/adr/0019-offset-table-intrinsic-boundary.md:516`, and the specs of 2026-08-19, 2026-08-23 and 2026-10-04), and none cites std/process; a wider `git grep -E "std/(io|fs|net|process)/lib\.nova"` outside `docs/adr` and `docs/superpowers` names the files only, never a line. The commit body says no citation needed re-pointing.

- [ ] **Step 5: Run**

- `cd /home/user/nova && cargo test -p nova-doc --test std_docs 2>&1 | grep "test result"`. Expected: `ok. 2 passed`.
- `cargo run -q -p nova-cli -- fmt --check std`. Expected: no output.
- `cargo build -q -p nova-cli && NOVA_HOME=$P/home-std target/debug/nova test --doc --std 2>&1 | tail -4`, with `$P/home-std` a fresh directory. Expected: `test result: ok. 17 passed; 0 failed; 0 trapped; 17 total` (9 module examples and 8 on single items so far). Without `tail`, the run lists each doc test by name, which shows each example landed on its item; this task's, among the earlier tasks' (the modules alphabetically, `String::split` last):

  ```text
  test std::fs (std/fs/lib.nova:18) ... ok
  test std::fs::open (std/fs/lib.nova:451) ... ok
  test std::io (std/io/lib.nova:18) ... ok
  test std::io::Write::write (std/io/lib.nova:225) ... ok
  test std::net (std/net/lib.nova:19) ... ok
  test std::net::TcpListener::accept (std/net/lib.nova:384) ... ok
  test std::process (std/process/lib.nova:12) ... ok
  ```

- [ ] **Step 6: Commit:** `docs: std's examples: io, fs, net, process`, body: spec 3.5b §9.

---

### Task 13: std's examples: task, sync, time, log

**Files:**
- Modify: `std/task/lib.nova`, `std/sync/lib.nova`, `std/time/lib.nova`, `std/log/lib.nova`
- Modify: `crates/nova-doc/tests/std_docs.rs` (`EXAMPLED`)
- Modify: any file whose comment cites a line these files move (Step 4)

- [ ] **Step 1:** `EXAMPLED` gains `"task"`, `"sync"`, `"time"`, `"log"`, one to a line after `"process"`. Run Task 11 Step 1's command. Expected: FAIL, the modules in `nova_std::ALL`'s order:

```text
4 module(s) without an example in their `//!` docs: std/log, std/sync, std/task, std/time
test result: FAILED. 1 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in …
```

- [ ] **Step 2: The module examples**

For each module, append the lines below after the last `//!` line of the file's module docs (the block of `//!` lines at its top), keeping every line as written. Each block was run as a doc test runs it on 2026-10-11, three times, with no failure, and its code is laid out as `nova fmt` lays it out (a block that is not a `fn main` module checked as the body of one).

`std/task/lib.nova`:

```nova
//!
//! Two tasks started with `spawn` and joined, run from code that is not
//! `async` by `block_on`:
//!
//! ```
//! async fn square(n: Int) -> Int {
//!     yield_now().await // let the other tasks take a turn
//!     n * n
//! }
//!
//! async fn sum_of_squares() -> Int {
//!     let a = spawn(square(3))
//!     let b = spawn(square(4))
//!     a.join().await + b.join().await
//! }
//!
//! fn main() {
//!     // `block_on` runs async work from code that is not async.
//!     assert_eq(block_on(sum_of_squares()), 25)
//! }
//! ```
```

`std/sync/lib.nova`:

```nova
//!
//! A producer task sends values through a channel and closes it; the
//! receiver takes them until `recv` gives `None`:
//!
//! ```
//! async fn produce(mut tx: Sender<Int>) {
//!     for i in 1..4 { tx.send(i * 10).await }
//!     tx.close() // once the channel is empty, `recv` gives `None`
//! }
//!
//! async fn main() {
//!     let ch: Channel<Int> = channel(2)
//!     spawn(produce(ch.sender()))
//!     let mut rx = ch.receiver()
//!     let mut total = 0
//!     while true {
//!         match rx.recv().await {
//!             Some(v) => total = total + v
//!             None => break
//!         }
//!     }
//!     assert_eq(total, 60)
//! }
//! ```
```

`std/time/lib.nova`:

```nova
//!
//! A `Duration` built and read back, a `sleep` timed with `Instant`, and a
//! wall-clock timestamp:
//!
//! ```
//! let d = Duration::from_millis(1500)
//! assert_eq(d.as_secs(), 1) // rounded toward zero
//! assert_eq(d.as_millis(), 1500)
//!
//! // Instant measures how long something takes.
//! let start = Instant::now()
//! block_on(sleep(Duration::from_millis(10)))
//! assert(start.elapsed().as_millis() >= 10, "sleep waits at least as long as asked")
//!
//! // SystemTime reads the wall clock, for timestamps.
//! let stamp = SystemTime::now().to_iso8601()
//! assert_eq(stamp.len(), 24)
//! assert(stamp.ends_with("Z"), "always in UTC")
//! ```
```

`std/log/lib.nova`:

```nova
//!
//! Logging at the default level, then raising the level so `info` lines are
//! dropped:
//!
//! ```
//! // Writes a line such as `2026-08-19T02:40:13.123Z INFO cache warmed` to
//! // standard error.
//! Log::info("cache warmed")
//!
//! // From here on, only `Warn` and `Error` are written.
//! Log::init_with(LogConfig { level: Warn, format: Human, output: Stderr })
//! Log::info("dropped: below the configured level")
//! Log::warn("disk almost full")
//!
//! // Levels compare by rank.
//! assert(Warn.to_int() > Info.to_int(), "Warn is more severe than Info")
//! assert_eq(Warn.label(), "WARN")
//! ```
```

- [ ] **Step 3: The examples on single items (spec §9.1)**

For each, append the lines after the last `///` line of the named item's docs, at that item's indentation (four spaces more inside an `impl` or a `trait`). A line written `# …` is hidden on the site and runs less its `# ` (spec §4.3).

`block_on`, above `pub fn block_on` in `std/task/lib.nova`:

```nova
///
/// `block_on` runs every task, not only the one it is given:
///
/// ```
/// async fn visit(mut log: Vec<String>, name: String) { log.push(name) }
///
/// fn main() {
///     let mut log: Vec<String> = Vec::new()
///     spawn(visit(log, "task"))
///     log.push("main") // the task has not run: no `block_on` is running yet
///
///     // `block_on` runs every task, not only the one it is given.
///     block_on(yield_now())
///     assert_eq(log.len(), 2)
///     assert_eq(log.get(0).unwrap(), "main")
///     assert_eq(log.get(1).unwrap(), "task")
/// }
/// ```
```

`Mutex::lock`, above `Mutex`'s `pub async fn lock` in `std/sync/lib.nova`:

```nova
///
/// Two tasks adding to one total, each holding the lock across a
/// suspension:
///
/// ```
/// async fn add(mut total: Mutex<Int>, n: Int) {
///     let mut guard = total.lock().await
///     let before = guard.get()
///     yield_now().await // other tasks run here, but none can lock `total`
///     guard.set(before + n)
///     guard.release() // nothing releases a guard for you
/// }
///
/// async fn main() {
///     let mut total = Mutex::new(0)
///     let a = spawn(add(total, 1))
///     let b = spawn(add(total, 2))
///     a.join().await
///     b.join().await
///     assert_eq(total.try_lock().unwrap().get(), 3)
/// }
/// ```
```

`timeout`, above `pub async fn timeout` in `std/time/lib.nova`:

```nova
///
/// One future in time, one too slow:
///
/// ```
/// async fn answer_after(ms: Int) -> Int {
///     sleep(Duration::from_millis(ms)).await
///     42
/// }
///
/// async fn main() {
///     // In time: `Ok` with the future's value.
///     let fast = timeout(Duration::from_secs(5), answer_after(1)).await
///     assert_eq(fast.unwrap_or(0), 42)
///
///     // Too slow: `Err(TimeoutError {})`. The future is abandoned, not
///     // cancelled, and nothing waits for it to finish.
///     let late = timeout(Duration::from_millis(10), answer_after(5000)).await
///     assert(late.is_err(), "10 ms passed before the answer came")
/// }
/// ```
```

- [ ] **Step 4: Line numbers cited in comments**

The examples move these files' lines. As 3.5a's Tasks 15 to 19 did, re-point every live citation of a line in the files this task changes, in this commit: `git grep -n -E "(task|sync|time|log)/lib\.nova:[0-9]|lib\.nova:[0-9]+|line [0-9]+" -- crates docs nova-spec std CHANGELOG.md`, then read each cited spot to tell which file a bare `lib.nova:N` or "line N" means, find the code it describes, re-point it, print the new line to check it, and ledger it. Dated records (ADRs, older specs and plans, released CHANGELOG entries) keep theirs.

Expected: none to re-point. The hits naming these files are dated plans and specs, and three that are not citations: `crates/nova-doc/src/doctest.rs:603` names a doc test of a source its test writes itself; `crates/nova-cli/src/cmd/test.rs:713-714` labels a doc test whose code it places on std/time's third line, which this task does not move (std/time's example goes after its line 12); and the 3.5b spec's `std/time/lib.nova:3` (§5.3) and `:12:5` (§7) illustrate a name's form. No live comment cites a line of std/sync, whose lines this task moves, and std/sync's own citations of std/core and std/collections (Task 11's) still hold. The commit body says no citation needed re-pointing.

- [ ] **Step 5: Run**

- `cd /home/user/nova && cargo test -p nova-doc --test std_docs 2>&1 | grep "test result"`. Expected: `ok. 2 passed`.
- `cargo run -q -p nova-cli -- fmt --check std`. Expected: no output.
- `cargo build -q -p nova-cli && NOVA_HOME=$P/home-std target/debug/nova test --doc --std 2>&1 | tail -4`, with `$P/home-std` a fresh directory. Expected: `test result: ok. 24 passed; 0 failed; 0 trapped; 24 total` (13 module examples and 11 on single items so far). Without `tail`, the run lists each doc test by name, which shows each example landed on its item; this task's, among the earlier tasks' (the modules alphabetically, `String::split` last):

  ```text
  test std::log (std/log/lib.nova:20) ... ok
  test std::sync (std/sync/lib.nova:28) ... ok
  test std::sync::Mutex::lock (std/sync/lib.nova:184) ... ok
  test std::task (std/task/lib.nova:19) ... ok
  test std::task::block_on (std/task/lib.nova:220) ... ok
  test std::time (std/time/lib.nova:17) ... ok
  test std::time::timeout (std/time/lib.nova:278) ... ok
  ```

- [ ] **Step 6: Commit:** `docs: std's examples: task, sync, time, log`, body: spec 3.5b §9.

---

### Task 14: std's examples: json, http, crypto, test; every module checked

**Files:**
- Modify: `std/json/lib.nova`, `std/http/lib.nova`, `std/crypto/lib.nova`, `std/test/lib.nova`
- Modify: `crates/nova-doc/tests/std_docs.rs` (`EXAMPLED` deleted)
- Modify: any file whose comment cites a line these files move (Step 4)

- [ ] **Step 1:** `EXAMPLED` gains `"json"`, `"http"`, `"crypto"`, `"test"`, one to a line after `"log"`. Run Task 11 Step 1's command. Expected: FAIL, the modules in `nova_std::ALL`'s order:

```text
4 module(s) without an example in their `//!` docs: std/crypto, std/http, std/json, std/test
test result: FAILED. 1 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in …
```

- [ ] **Step 2: The module examples**

For each module, append the lines below after the last `//!` line of the file's module docs (the block of `//!` lines at its top), keeping every line as written. Each block was run as a doc test runs it on 2026-10-11, three times, with no failure, and its code is laid out as `nova fmt` lays it out (a block that is not a `fn main` module checked as the body of one).

`std/json/lib.nova`:

```nova
//!
//! Parsing JSON text, reading values out of an object, and writing a value
//! back as compact text:
//!
//! ```
//! let doc = parse("{\"name\": \"nova\", \"stars\": 42}").unwrap()
//! let fields = match doc {
//!     Object(m) => m
//!     _ => panic("expected an object")
//! }
//! assert_eq(String::from_json(fields.get("name").unwrap()).unwrap(), "nova")
//! assert_eq(Int::from_json(fields.get("stars").unwrap()).unwrap(), 42)
//!
//! // stringify writes compact text; every number is a Float, and 1.0 is `1`.
//! let list = Array([Number(1.0), Bool(true), Null, "hi".to_json()])
//! assert_eq(stringify(list), "[1,true,null,\"hi\"]")
//!
//! // parse is strict: no trailing commas, no comments, no single quotes.
//! assert(parse("[1, 2,]").is_err(), "a trailing comma is an error")
//! ```
```

`std/http/lib.nova`:

```nova
//!
//! `Server::listen` serves until the process is stopped, so this example
//! has the server `dispatch` a request parsed from bytes, with no socket:
//!
//! ```
//! // `listen` serves until the process is stopped, so this example asks the
//! // server to `dispatch` a request it parsed from bytes, with no socket.
//! let app = Server::new().get("/hello", |_| Response::text(200, "hi"))
//!
//! let raw = bytes_from_string("GET /hello HTTP/1.1\r\nHost: example\r\nX-Trace: abc\r\n\r\n")
//! let req = parse_request_head(raw, Limits::default()).unwrap().unwrap()
//! assert_eq(req.headers.get("x-trace").unwrap(), "abc") // names are lower-cased
//!
//! let resp = app.dispatch(req)
//! assert_eq(resp.status, 200)
//! assert_eq(resp.headers.get("content-length").unwrap(), "2")
//! let wire = resp.to_bytes().to_string().unwrap()
//! assert(wire.starts_with("HTTP/1.1 200 OK\r\n"), "the status line comes first")
//! assert(wire.ends_with("\r\n\r\nhi"), "then a blank line and the body")
//! ```
```

`std/crypto/lib.nova`:

```nova
//!
//! Hashing bytes, tagging a message with HMAC and checking the tag, and
//! drawing random values:
//!
//! ```
//! let digest = sha256(bytes_from_string("abc"))
//! assert_eq(digest.len(), 32) // raw bytes, not hex
//! assert_eq(digest.byte_at(0).unwrap(), 0xba) // SHA-256("abc") is ba7816bf…
//!
//! // An HMAC tag proves a message came from someone holding the key.
//! let key = bytes_from_string("secret key")
//! let msg = bytes_from_string("amount=10")
//! let tag = hmac_sha256(key, msg)
//! assert(hmac_sha256_verify(key, msg, tag), "the tag matches its message")
//! assert(
//!     !hmac_sha256_verify(key, bytes_from_string("amount=99"), tag),
//!     "a changed message does not",
//! )
//!
//! let roll = random_int(1, 6).unwrap()
//! assert(roll >= 1 && roll <= 6, "both ends are included")
//! assert_eq(random_bytes(16).unwrap().len(), 16)
//! ```
```

`std/test/lib.nova`:

```nova
//!
//! The three assertions, each passing here:
//!
//! ```
//! let total = 2 + 3
//! assert(total > 4, "total is more than 4")
//! assert_eq(total, 5)
//! assert_ne("left", "right")
//!
//! // A failing check panics with both values in their Debug form:
//! // `assert_eq("a", "b")` panics with `assertion failed: "a" != "b"`.
//! assert_eq("a".dbg(), "\"a\"")
//! ```
```

- [ ] **Step 3: The examples on single items (spec §9.1)**

For each, append the lines after the last `///` line of the named item's docs, at that item's indentation (four spaces more inside an `impl` or a `trait`). A line written `# …` is hidden on the site and runs less its `# ` (spec §4.3).

`parse`, above `pub fn parse` in `std/json/lib.nova`:

```nova
///
/// Walking a parsed value, and where an error stopped:
///
/// ```
/// // Walk a parsed value by matching on its variants.
/// let doc = parse("{\"tags\": [\"fast\", \"small\"]}").unwrap()
/// let tags = match doc {
///     Object(fields) => fields.get("tags").unwrap()
///     _ => panic("expected an object")
/// }
/// let count = match tags {
///     Array(items) => items.len()
///     _ => 0
/// }
/// assert_eq(count, 2)
///
/// // An error says what is wrong, and where parse stopped, in characters.
/// match parse("[1, 2,]") {
///     Ok(_) => panic("a trailing comma is not JSON")
///     Err(e) => assert_eq(e.at, 6) // the `]` where a value should be
/// }
/// ```
```

`ToJson`, above `pub trait ToJson` in `std/json/lib.nova`:

```nova
///
/// A record of your own written as JSON:
///
/// ```
/// record Point { x: Int, y: Int }
///
/// impl ToJson for Point {
///     fn to_json(self) -> JsonValue {
///         let mut fields: Map<String, JsonValue> = Map::new()
///         fields.insert("x", self.x.to_json())
///         fields.insert("y", self.y.to_json())
///         Object(fields)
///     }
/// }
///
/// fn main() {
///     let text = stringify(Point { x: 1, y: 2 }.to_json())
///     // An object's members come out in an order that can change between runs.
///     assert(text == "{\"x\":1,\"y\":2}" || text == "{\"y\":2,\"x\":1}", text)
/// }
/// ```
```

`read_request`, above `pub async fn read_request` in `std/http/lib.nova`:

```nova
///
/// One request read from a connection and answered, over the loopback
/// address:
///
/// ```
/// async fn serve_one(listener: TcpListener) {
///     let conn = listener.accept().await.unwrap()
///     let req = read_request(conn, Limits::default()).await.unwrap()
///     let reply = Response::text(200, "got ${req.body.len()} bytes")
///     write_response(conn, reply).await.unwrap()
///     conn.close().await.unwrap()
/// }
///
/// async fn main() {
///     let listener = bind("127.0.0.1:0").unwrap()
///     let server = spawn(serve_one(listener))
///     let client = connect("127.0.0.1:${listener.local_port().unwrap()}").await.unwrap()
///     let ask = "POST /upload HTTP/1.1\r\nContent-Length: 5\r\n\r\nhello"
///     client.write(bytes_from_string(ask)).await.unwrap()
///     let reply = client.read_to_end().await.unwrap().to_string().unwrap()
///     assert(reply.starts_with("HTTP/1.1 200 OK\r\n"), reply)
///     assert(reply.ends_with("\r\n\r\ngot 5 bytes"), reply)
///     server.join().await
///     listener.close().await.unwrap()
/// }
/// ```
```

`parse_request_head`, above `pub fn parse_request_head` in `std/http/lib.nova`:

```nova
///
/// Part of a head, then all of it:
///
/// ```
/// let limits = Limits::default()
///
/// // Part of a head is `Ok(None)`: read more, then try again with it all.
/// let start = bytes_from_string("GET /items HTTP/1.1\r\nHost: shop")
/// assert(parse_request_head(start, limits).unwrap().is_none(), "not a whole head yet")
/// let whole = start.concat(bytes_from_string("\r\n\r\n"))
/// let req = parse_request_head(whole, limits).unwrap().unwrap()
/// assert_eq(req.path, "/items")
/// assert_eq(req.headers.get("host").unwrap(), "shop")
///
/// // A malformed head is an `Err`.
/// assert(
///     parse_request_head(bytes_from_string("HELLO\r\n\r\n"), limits).is_err(),
///     "no target or version",
/// )
/// ```
```

- [ ] **Step 4: Line numbers cited in comments**

The examples move these files' lines. As 3.5a's Tasks 15 to 19 did, re-point every live citation of a line in the files this task changes, in this commit: `git grep -n -E "(json|http|crypto|test)/lib\.nova:[0-9]|lib\.nova:[0-9]+|line [0-9]+" -- crates docs nova-spec std CHANGELOG.md`, then read each cited spot to tell which file a bare `lib.nova:N` or "line N" means, find the code it describes, re-point it, print the new line to check it, and ledger it. Dated records (ADRs, older specs and plans, released CHANGELOG entries) keep theirs.

Expected: one, `docs/benchmarks/README.md:458`: `std/http/lib.nova:618` to `:658`, the line reading "denial-of-service, not a hygiene gap.** Both park with no deadline" in `read_request`'s maintainer comment (it moves by the 40 lines of examples above it, std/http's and `parse_request_head`'s). The README is the living procedure, not a record (3.5a's Task 19 re-pointed the same citation). The other hits into these files are dated records: `docs/superpowers/plans/2026-09-29-request-cost-differential.md:705,714,715`, `docs/superpowers/specs/2026-10-04-examples-03-04-inventory.md:364,393`, `docs/superpowers/specs/2026-10-05-gc-unix-stack-bounds-design.md:404,532` and `docs/superpowers/specs/2026-10-10-phase-3-4a-navigation-design.md:111`. The commit body names the re-pointed README citation.

- [ ] **Step 5: Every module checked.** Delete `EXAMPLED`, its doc comment, and the `if !EXAMPLED.contains(&name) { continue; }` lines. In `crates/nova-cli/tests/doc_test.rs`, the comment saying std has no example until Tasks 11 to 14 add them now says that std's examples run here.

- [ ] **Step 6: Run**

- `cd /home/user/nova && cargo test -p nova-doc --test std_docs 2>&1 | grep "test result"`. Expected: `ok. 2 passed`.
- `cargo run -q -p nova-cli -- fmt --check std`. Expected: no output.
- `cargo build -q -p nova-cli && NOVA_HOME=$P/home-std target/debug/nova test --doc --std 2>&1 | tail -4`, with `$P/home-std` a fresh directory. Expected: `test result: ok. 32 passed; 0 failed; 0 trapped; 32 total` (17 module examples and 15 on single items in all). Without `tail`, the run lists each doc test by name, which shows each example landed on its item; this task's, among the earlier tasks' (the modules alphabetically, `String::split` last):

  ```text
  test std::crypto (std/crypto/lib.nova:17) ... ok
  test std::http (std/http/lib.nova:21) ... ok
  test std::http::parse_request_head (std/http/lib.nova:322) ... ok
  test std::http::read_request (std/http/lib.nova:711) ... ok
  test std::json (std/json/lib.nova:18) ... ok
  test std::json::parse (std/json/lib.nova:623) ... ok
  test std::json::ToJson (std/json/lib.nova:1365) ... ok
  test std::test (std/test/lib.nova:17) ... ok
  ```
- The full suite, `cargo test --workspace --locked --no-fail-fast 2>&1 | tee $P/full-14.txt | grep -E "FAILED|panicked" | head`. Expected: only the environmental IPv6 test, nova-runtime's `net::tests::an_ipv6_connect_parks_on_its_first_poll_and_then_establishes`; the dry run of 2026-10-11 had 1989 passed, 1 failed and 10 ignored over 98 test binaries, `every_std_module_has_an_example` and `doc_std_runs_stds_doc_tests_inside_a_project_or_not` among the passed. No hover or completion test compares the text of an item that gains an example; if one fails, check that the example is the only difference, update its expected text, and ledger it.

- [ ] **Step 7: Commit:** `docs: std's examples: json, http, crypto, test; every module checked`, body: spec 3.5b §9.

---

### Task 15: The example's doc tests, and CI (spec §10, §12.4)

**Files:**
- Modify: `examples/05-geometry/shapes/src/lib.nova` (its `//!` docs),
  `examples/05-geometry/shapes/src/units.nova` (`Length::plus`'s docs),
  `examples/05-geometry/README.md`
- Modify: `.github/scripts/doc-gate.sh`, `.github/workflows/ci.yml` (the
  `install` job's doc-gate step: its comment and name)
- Test: `crates/nova-cli/tests/doc_test.rs`

**Interfaces:** none change. `doc-gate.sh NOVA WORKDIR` is called as
before.

- [ ] **Step 1: Write the failing test**

In `crates/nova-cli/tests/doc_test.rs`, append, at the end of the file:

````rust

/// `from` copied to `to`, without any `target/`.
fn copy(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let name = entry.file_name();
        if name == "target" {
            continue;
        }
        let path = entry.path();
        if path.is_dir() {
            copy(&path, &to.join(&name));
        } else {
            std::fs::copy(&path, to.join(&name)).unwrap();
        }
    }
}

#[test]
fn the_example_library_runs_its_doc_tests() {
    // Spec §10, §12.4: `nova test` inside a copy of the example's library.
    let shapes = fresh("example").join("shapes");
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    copy(&repo.join("examples/05-geometry/shapes"), &shapes);
    let a = nova_test(&shapes, &[]).success();
    assert_eq!(
        stdout(&a),
        "running 0 tests\n\
         \n\
         test result: ok. 0 passed; 0 failed; 0 trapped; 0 total\n\
         \n\
         running 2 doc tests\n\
         test shapes (src/lib.nova:8) ... ok\n\
         test shapes::units::Length::plus (src/units.nova:17) ... ok\n\
         \n\
         test result: ok. 2 passed; 0 failed; 0 trapped; 2 total\n"
    );
    assert_eq!(stderr(&a), "");
}
````

Run: `cd /home/user/nova && cargo test -p nova-cli --test doc_test the_example 2>&1 | grep -E "^    [a-z_]+$|^test result|left:"`
Expected: FAIL, as the library has no example yet:

````text
  left: "running 0 tests\n\ntest result: ok. 0 passed; 0 failed; 0 trapped; 0 total\n"
    the_example_library_runs_its_doc_tests
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 14 filtered out; finished in …
````

- [ ] **Step 2: The examples**

In `examples/05-geometry/shapes/src/lib.nova`, at the end of the module
docs, replace:

````nova
//! radius cannot be mixed up with a bare number.
````

with:

````nova
//! radius cannot be mixed up with a bare number.
//!
//! A circle and a rectangle, measured alone and together:
//!
//! ```
//! let shapes = [circle(1.0), rectangle(2.0, 3.0)]
//! assert_eq(shapes[1].area(), 6.0)
//! assert_eq(total_area(shapes), PI + 6.0)
//! ```
````

In `examples/05-geometry/shapes/src/units.nova`, in `Length::plus`'s
docs, replace:

````nova
    /// This length and `other` laid end to end.
````

with:

````nova
    /// This length and `other` laid end to end.
    ///
    /// A unit square's perimeter, 4, laid end to end with itself:
    ///
    /// ```
    /// let outline = rectangle(1.0, 1.0).perimeter()
    /// assert_eq(outline.plus(outline).value, 8.0)
    /// ```
````

In `examples/05-geometry/README.md`, at the end of "What this
demonstrates", replace:

````markdown
  which `nova doc` renders.
````

with:

````markdown
  which `nova doc` renders.
- **Doc tests:** the code examples in the library's docs are tests, and
  `nova test` inside `shapes` runs them.
````

- [ ] **Step 3: CI runs them**

In `.github/scripts/doc-gate.sh`, in the header comment, replace:

````bash
# with a path library, with no warning, and writes the pages the gate needs.
````

with:

````bash
# with a path library, with no warning, and writes the pages the gate needs.
# And Phase 3.5b's (spec
# docs/superpowers/specs/2026-10-11-phase-3-5b-doc-tests-design.md §10,
# §13): it runs the library's two doc tests, and they pass.
````

Then, after the loop over the pages, replace:

````bash
done
````

with:

````bash
done

cd shapes
if ! "$nova" test --doc > test.out 2> test.err; then
  cat test.out test.err >&2
  echo "doc-gate: nova test --doc failed in shapes" >&2
  exit 1
fi
result=$(tail -n 1 test.out | tr -d '\r')
if [ "$result" != "test result: ok. 2 passed; 0 failed; 0 trapped; 2 total" ]; then
  cat test.out test.err >&2
  echo "doc-gate: nova test --doc printed '$result'" >&2
  exit 1
fi
````

In `.github/workflows/ci.yml`, in the `install` job, replace:

````yaml
      - name: The installed nova documents an example that uses a dependency
````

with:

````yaml
      # And the 3.5b gate's (spec
      # docs/superpowers/specs/2026-10-11-phase-3-5b-doc-tests-design.md
      # §13): the example library's doc tests pass.
      - name: The installed nova documents an example that uses a dependency, and runs its doc tests
````

- [ ] **Step 4: Run**

Run: `cd /home/user/nova && cargo test -p nova-cli --test doc_test 2>&1 | grep -E "^test result|FAILED"`
Expected: `test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out`.

Run: `cd /home/user/nova && cargo run -q -p nova-cli -- fmt --check examples`
Expected: no output.

Run: `cd /home/user/nova && cargo test -p nova-doc --test gate 2>&1 | grep -E "^test result|FAILED" && cargo test -p nova-driver --test index_complete 2>&1 | grep -E "^test result|FAILED"`
Expected: `test result: ok. 2 passed` and `test result: ok. 1 passed`.

Run: `cd /home/user/nova && cargo build -q -p nova-cli && rm -rf $P/gate-35b $P/home-gate-35b && NOVA_HOME=$P/home-gate-35b bash .github/scripts/doc-gate.sh target/debug/nova $P/gate-35b && cat $P/gate-35b/geometry/shapes/test.out`
Expected:

````text
doc-gate: passed
running 2 doc tests
test shapes (src/lib.nova:8) ... ok
test shapes::units::Length::plus (src/units.nova:17) ... ok

test result: ok. 2 passed; 0 failed; 0 trapped; 2 total
````

- [ ] **Step 5: Commit**

Run `cargo fmt --all` (expected: no change).
Stage `.github/scripts/doc-gate.sh`, `.github/workflows/ci.yml`,
`crates/nova-cli/tests/doc_test.rs`, `examples/05-geometry/README.md`,
`examples/05-geometry/shapes/src/lib.nova` and
`examples/05-geometry/shapes/src/units.nova`.
Message: `feat: the geometry example's library has doc tests, and CI runs
them`, body: shapes gets an example in its module docs and one on
Length::plus, which nova test inside shapes runs; doc-gate.sh runs them
with the installed nova (spec 3.5b §10, §12.4).

---

### Task 16: The records

**Files:**
- Create: `docs/adr/0035-doc-tests.md`
- Modify: `nova-spec/40-TOOLING.md` (§5, §7), `docs/phase-3-plan.md` (the
  3.5 entry), `agent.md` (§5), the 3.3a spec (decision 14),
  `CHANGELOG.md`, `README.md`, `ARCHITECTURE.md`, and
  `nova-spec/00-MASTER-SPEC.md` (§6) only if a crate's role changed

- [ ] **Step 1: ADR 0035, "Doc tests"**

Write it in the form of `docs/adr/0034-the-documentation-generator.md`:
Status (Accepted, 2026-10-11, Phase 3.5b, this branch, the spec's path),
Context, Decision, Consequences. Its Decision records, each in a line or
two, from the spec and from the ledger's rulings, never from this plan's
expectations alone:
- which blocks run (a library's documented items; §4.2's table; empty
  words dropped; unknown words warn; `ignore`);
- hidden lines, on the site, in hover, and as they run;
- how a block becomes a test (hoisted imports, the package import unless
  the block imports it, `fn main` and `async fn main`, a plain body that
  ends in no value, only the added test collected);
- names, by the site's paths and the block's first line;
- the separate binary, both sections, and no warning twice;
- diagnostics mapped to the doc comment, and std's named
  `std/<module>/lib.nova`;
- the driver's `RootKind::Doc`, `build_test_program_with` and
  `MapDiagnostic`, and that no file is written;
- the closure fix (Task 1), as found while verifying std's examples;
- `nova-lsp`'s dependency on `nova-doc`.

Its Consequences record, with real figures from the ledger: the cost of
the second build, std's 17 module examples and 15 more, and what 3.5b
leaves out (spec §17).

- [ ] **Step 2: Dated notes**

Each begins `**Amended 2026-10-11 (branch `claude/loving-archimedes-frtf5q`):**`
or, where a running list is worded "Recorded", that word, as the file's
earlier notes do (3.5a's Task 23 ruling):
- `nova-spec/40-TOOLING.md` §5: `nova test --doc` is delivered (the spec's
  path and ADR 0035); §7: `nova test` also runs a library's doc tests,
  `--doc` alone, `--doc --std` std's.
- `docs/phase-3-plan.md`'s 3.5 entry: 3.5b is built; the gate's tests and
  where they ran, stated as 3.5a's note was (pass on Linux here; the
  pull request's CI runs all three systems); 3.5 is complete.
- `agent.md` §5: the rows for `nova-doc` (doc tests) and `nova-lsp` (its
  new dependency), as its earlier notes cite their ADRs.
- The 3.3a spec's decision 14: settled by the user on 2026-10-11, keep
  refusing, with the message of plan Task 2.
- `nova-spec/00-MASTER-SPEC.md` §6, only if a crate's listed role is no
  longer true.

- [ ] **Step 3: CHANGELOG, README, ARCHITECTURE**

- `CHANGELOG.md` `[Unreleased]`, after 3.5a's entries, as 3.5a's follow
  3.4b's:
  - Added: doc tests (`nova test` runs a library's code examples;
    `--doc`, `--doc --std`; hidden lines; diagnostics point at the doc
    comment), std's examples.
  - Fixed: a test program may hold a closure or a function passed as a
    value (Task 1).
  - Changed: `nova init --lib`'s message in a package; `ignore` blocks are
    highlighted as Nova; hover leaves out hidden lines.
- `README.md`'s Documentation section: one or two sentences on doc tests.
- `ARCHITECTURE.md`'s `nova-doc` row: doc tests.

- [ ] **Step 4: The sweep**

Run `git grep -n -i -E "test --doc|doc test|doctest|3\.5b" -- docs nova-spec agent.md README.md ARCHITECTURE.md CHANGELOG.md`.
Every hit must agree with what is built, be a dated record of its own
time, or be covered by Step 2's notes. Ledger the count before and after,
and anything outside these files noticed and not changed.

- [ ] **Step 5: Check and commit**

- `cargo run -q -p nova-cli -- fmt --check std examples`: no output.
- Every path the records cite exists (`ls` each).
- The diff holds no `/home/`, `/tmp/`, `scratchpad` or personal name.
- Commit: `docs: records for Phase 3.5b: ADR 0035, notes, CHANGELOG`, with
  a body in the style of 3.5a's records commit.

---

### Task 17: Final verification, mutants, and the PR body

**Files:** none in the repository; `$P/pr-body-35b.md`.

- [ ] **Step 1: Everything, on Linux**

Each to a file in `$P`, reading the tails:
- `cargo fmt --all -- --check`
- `cargo clippy --locked --all-targets --all-features -- -D warnings`
- `RUSTFLAGS="-D warnings" cargo +1.78 check --locked --workspace`
- `cargo +1.78 check --locked -p nova-doc --all-targets`
- `cargo run -q -p nova-cli -- fmt --check std examples`
- `cargo test --workspace --locked --all-features --no-fail-fast 2>&1 | tee $P/full-final-35b.txt`, then the counting one-liner:
  `grep -E '^test result:' $P/full-final-35b.txt | awk '{p+=$4; f+=$6; i+=$8} END {print p" passed, "f" failed, "i" ignored"}'`

Expected: all clean, and only the environmental IPv6 test failing. Ledger
the counts beside Task 0's (1942 passed, 1 failed, 10 ignored), and
compare test by test with `$P/base-35b.txt`: no test missing or changed
outcome.

- [ ] **Step 2: The spec's mutants (§12.5)**

Each on committed work, undone with `git checkout -- <file>` and a clean
`git status --short` before the next, each failing its named test.
Mutants 1 to 5 and 7 ran in the tasks that own them (Tasks 3, 6, 8); run
6, 8 and 9 here, and any whose earlier run the ledger does not record:
6. The line map off by one line (`DocTest::map` returns the next line's
   offset) → the CLI's compile-error test (Task 9).
8. A failing doc test reported as passing (the doc section's
   `Outcome::Panicked` arm counts a pass) → the CLI's failing-example test
   (Task 9).
9. One std module's example deleted → `every_std_module_has_an_example`.

Ledger each with the test that failed.

- [ ] **Step 3: The PR body**

Write `$P/pr-body-35b.md`, with no local path and no personal name, under
the repository's PR template headings (`## Summary`, `## Changes`,
`## Testing`, `## Phase Gate Criteria`), as PR #109 was, then
`## Final review` and `## Decisions to review` and `## Deferred minors`:
1. Summary: what a user gets; the spec, the plan and ADR 0035; 3.5 is
   complete.
2. Changes: the closure fix first (a bug any user with a closure in a
   tested program hit); what changed by crate; changes in behaviour;
   std's examples; dependencies (no new package; the one `Cargo.lock`
   edge the user approved).
3. Testing: the template's checklist, then Linux (local) against `main`
   at the branch base, Windows and macOS "from this PR's CI"; new tests
   by file; each of the spec's nine mutants failing its named test.
4. Phase Gate Criteria: spec §13, each with its test.
5. Final review: "To be filled in after the review."
6. Decisions to review: the spec's decisions, this plan's decisions, and
   every ledger `Ruling:` line with its cost if wrong, in collapsed
   lists.
7. Deferred minors: from the final review, and PR #109's that 3.5b left
   (spec §17).
8. The session's attribution line for pull requests.

Run: `grep -n -i -E "/home/|/tmp/|claude-0|sakeerin|scratchpad" $P/pr-body-35b.md`
Expected: no output.

There is no commit in this task. The branch is pushed and the PR opened
only after the final review and its fix pass, as the workflow says.

