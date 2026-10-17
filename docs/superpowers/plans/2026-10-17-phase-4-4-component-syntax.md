# Phase 4.4, "Component syntax", Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** a program writes its views as `30-FRONTEND.md` §3 does.
`component Name(params) { … }` is a function that returns `std/ui`'s
`View`; `view { … }` is a tree of elements, text and views, whose elements
take their attributes by name and their children in a block; and `||` is a
closure with no parameters everywhere. The parser keeps the sugar as its
own nodes, the type checker desugars it into 4.3's builders and variants,
and `nova fmt` prints it in one layout, under its three checks; the
repository's `| |` become `||`. The bundler gets its last pieces: a
project's own `index.html`, `assets/` from `public/`, `manifest.json`, and
`wasm-opt` under `--release`. The gate: `examples/06-counter-spa` counts
its clicks under the fake DOM in `cargo test` on all three systems and in
Chromium in CI, and its `app.wasm` and `app.js` are under 50 KB gzipped.

**Architecture:**
- **`nova-ast`:** `Expr::View(Vec<Spanned<ViewNode>>)`, `ViewNode`
  (an element or an expression) and `ViewArg` (named or positional);
  `Function::is_component`.
- **`nova-parser`:** `component` items and `view` atoms, both contextual;
  elements, named arguments and trailing blocks inside a view; an optional
  `;` between children; `||` as a closure where an expression starts.
- **`nova-fmt`:** `||`; `component`; a view's and an element's layout;
  `CONTINUES` gains `||`, and a view's child gets `;` as a statement does.
- **`nova-typeck`:** `check/view.rs`: a component's return type, and a
  view's desugaring into calls of its builders and `std/ui`'s variants;
  E0094 to E0097.
- **`nova-lsp`, `tools/vscode-nova`:** a `keyword` semantic token for the
  two words where they are keywords; two grammar rules.
- **`nova-diagnostics`, `nova-doc`:** `keyword_start` knows `component`,
  so a doc page shows a component as written.
- **`nova-cli`:** `bundle.rs`: the page, `assets/`, `manifest.json` and
  `wasm-opt`, after the compiler has written `dist/`. **`nova-codegen-wasm`:**
  the generated page gains `<div id="app"></div>`.
- **`examples/06-counter-spa`**, its fake-DOM harness, and
  `tests/browser/spa.mjs` in the `browser` job.
- **Records:** RFC 0001, ADR 0043, and dated notes.

**Tech Stack:**
- Rust: MSRV 1.78, edition 2021.
- **No new crate; `Cargo.lock` does not change** (spec §9.1).
- JS for the tests' harnesses: ES modules, run by Node 22, with 4.2's fake
  DOM unchanged.
- Playwright 1.56.1, installed by the CI job only.
- Tests: unit tests in `nova-typeck`, `nova-lsp`, `nova-diagnostics`,
  `nova-doc` and `nova-cli`'s `bundle.rs`; `crates/nova-parser/tests/parser_tests.rs`;
  `crates/nova-fmt/tests/layout.rs` and `source_rules.rs`;
  `crates/nova-cli/tests/run_tests.rs`, `wasm_tests.rs` and
  `vscode_extension.rs`; `crates/nova-driver/tests/index_complete.rs`;
  programs in `tests/runtime/` and `tests/dom/`.

**Spec:** `docs/superpowers/specs/2026-10-17-phase-4-4-component-syntax-design.md`,
approved by the user on 2026-10-17.
- Read it before Task 1: it is the authority this plan argues from.
- Its §14 lists the decisions made while writing it, and its §2 the code as
  it stood. This plan's decisions say where the code departs from it.

**Checked on 2026-10-17, before execution:**
- **A prototype built 4.4 whole** on a local branch from `3a5b1ad`, in
  this tree, with every test below.
- **A replay applied it** to `3a5b1ad` on a scratch branch, task by task,
  as the steps say, each task's tests before its code; each test-first
  step's failure is that run's. The replay found one fault in the
  prototype, one test that could harm the machine, and tests too weak to
  catch what their mutants broke, each fixed here:
  - **Every string literal was reactive.** `is_str_lit` matched
    `Expr::Lit(Literal::Str)`, which only a raw string `r"…"` is: every
    other string is a `StringInterp`. So `class: "a"` became `Dynamic` and `text("+")`
    `DynText`, an effect each, with the same output; no test that renders
    could see it. A literal is now a `StringInterp` of literal parts, and
    `a_views_parts_are_fixed_or_read_again_by_literalness` reads the HIR
    (decision 7).
  - **A refused output directory's test wrote into the system's
    temporary directory** when a mutant broke the guard it tests: `-o ..`
    from a project in `$TMPDIR/nova-wasm-test-bundle-refused` is
    `$TMPDIR` itself, where the build wrote its files and replaced
    `assets/`. The project now sits one level down, so `..` is the test's
    own directory (Task 4).
  - Too weak, each now tested: E0096 for a builder of two other parameters, or the right ones and
    another return type; the E0010 and E0097 messages; a `fn() -> Int`
    handler; `component` as a name before a call, as line breaks are
    skipped; a keyword token beside a variable's at `if view`; and the
    count before the clicks in both 06 harnesses, so a fixed text cannot
    pass (decision 19). The quoter of `manifest.json` has a unit test.
  - A script then split the replay's final tree into one commit per task,
    each task's tests split from its code. The patches below are those
    commits; each task's tree is the replay's.
- **The gate passed in the replay:** `06-counter-spa` under the fake DOM,
  as is and under `NOVA_GC_STRESS`, and in Chromium through Playwright
  1.56.1 with `tests/browser/spa.mjs`; 21,019 bytes of `app.wasm` and
  `app.js` by `flate2` at level 9, as the test measures, against 51,200,
  without `wasm-opt`.
- **On the last commit, in the replay:**
  - the full suite gave 2316 passed, 1 failed (the IPv6 test) and 10
    ignored, against the base's `2286 1 10` (its run's one other failure,
    `an_app_runs_builds_and_tests_with_a_published_library`, came from a
    cleanup of `/tmp` during the run, which removed `nova test`'s cache of
    binaries; it passes alone at the base);
  - clippy with `-D warnings`, both formatters and Rust 1.78's check
    were clean, and `Cargo.lock` did not change;
  - after each task, `cargo fmt`, clippy and `nova fmt` were clean.
- **A fact-check read the first draft** against the spec, the replay's
  trees and logs, with probes of its own, and found two faults in the
  bundle, one in the parser and errors in this plan, each fixed here:
  - `-o nope/..` got past the guard on the output directory: the
    generated page was written over the project's `index.html`, which the
    copy then emptied, and the project's own `assets/` was deleted.
    `resolved` now takes `..` away, `finish` never copies the page onto
    itself, and the test tries `nope/..` and `src/..` (decision 15;
    mutants 47 and 51).
  - A link in `public/` to a file outside the project was deployed. A
    link is copied only when it leads to a file inside `public/`, and the
    test has one of each on Unix (decision 16; mutant 56).
  - A statement `component`, a variable, followed by a call on the next
    line, read as a component item. `component_at` wants the body's `{`
    after the parameters (decision 1; mutant 14).
  - Untested: a raw string as a literal, and `on_` alone, now E0097
    (decisions 6 and 7; mutants 21 and 25).
  - In this plan: the figure for the last commit was taken before the
    final change, and is now measured on it; Task 5's check that a build
    writes nothing into the example now lists untracked files one by
    one; the size is labelled as `flate2`'s; decision 19 and Task 2's gate
    commands are corrected; and the records' wording on keywords and the
    output directory is made exact.
- **A second check read those fixes,** with probes of every spelling and
  link it could think of, and found two more ways out of the output
  directory and a parser case left, each fixed or named here:
  - A link already in `dist/` at a name the build writes, such as
    `dist/index.html -> ../index.html`, was written through: the
    generated page replaced the project's own. `Bundle::prepare` removes a
    link at each such name before the compiler writes (decision 15;
    mutant 49), and the test puts three there on Unix.
  - A `public/` that is itself a link was followed outside the project.
    It is not copied, and the build says so (decision 16; mutant 54).
  - A statement that is a variable named `component`, followed by a call
    and then a bare block, each on its own line, still reads as a
    component: the parser has no line breaks to tell. Decision 1 and
    ADR 0043 name it.
  - The page guard's unit test spelled the output directory as the root
    is spelled, so a guard that compared paths as written passed it; it
    spells it `sub/..` now (mutant 52). `-o` through a link to the
    parent is tested on Unix. On Windows `resolved` takes `..` away by
    the text first, as Win32 does; this machine cannot test that
    (decision 15).
- **The runs above, Tasks 4 to 6's walk, the full suite and the mutants,
  were run after both checks' fixes;** Tasks 1 to 3's commits did not
  change.
- **All 67 mutants** in Task 7 failed their named tests, run on
  the replay's last commit.

## Global Constraints

- **The workspace's minimum Rust stays 1.78,** edition 2021. CI's MSRV job
  runs `cargo check --locked --workspace` with `RUSTFLAGS=-D warnings`. No
  `Option::is_none_or` (1.82), no `std::path::absolute` (1.79).
- **`Cargo.lock` does not change.** Any change to it stops the task: ask
  the user. So does anything that needs a new crate. `serde_json` is a
  dev-dependency of `nova-cli` only, so `bundle.rs` quotes its JSON itself.
- **No `unwrap()` outside `#[cfg(test)]` and test files:** `expect("why")`
  or `?`.
- **Tests never touch the internet.** A test that needs `NOVA_HOME` sets it
  to a fresh directory. Every test directory has a fixed name under the
  system's temporary directory, removed and made again at the test's
  start. Node runs only what a test built.
- **A test that needs Node** passes without running when `node` is not on
  `PATH`, and fails instead under `NOVA_REQUIRE_NODE=1`, which CI sets.
  Run the WASM tests here with `NOVA_REQUIRE_NODE=1`: Node 22 is
  installed.
- **Nothing in `cargo test` or `nova` installs a package,** and nothing
  installs `wasm-opt`. Playwright is the `browser` job's alone; Task 5's
  local run uses the copy already on this machine, and is skipped where
  there is none.
- **`nova-cli`'s tests need two files beside `target/debug/nova`:**
  `libnova_runtime.a` and the `nova-bench-http` binary. A build of the
  workspace writes both; Task 0's build puts them there, and no task
  changes `nova-runtime`. After a `cargo clean`, build the workspace again.
- **std is embedded at build time.** After a task changes a file under
  `std/`, a run of `target/debug/nova` by hand needs `cargo build --locked
  -p nova-cli` first; `cargo test` rebuilds it itself.
- **Never clean `/tmp/nova-*` while tests run:** `nova test` keeps built
  binaries in `/tmp/nova-test-bin`, and a test whose binary goes fails.
- **Work in one tree.** Two worktrees of this repository sharing one
  `target/` share artifact names, so cargo can take one tree's build as
  fresh for the other (4.0's ledger). Never build in another worktree.
- **A build of `examples/06-counter-spa` writes nothing into the
  repository:** every one names its output with `-o`, outside the tree.
- **Nothing is published, and no tag is pushed.** Merge by rebase, on the
  user's word only. Never enable auto-merge.
- **This machine is Linux.** Windows and macOS results come from CI, which
  runs all three systems, Node included.

## Conventions

- **Scratch directory:** `$P` is the session's scratch directory, outside
  the repository. Set it in every shell before a command that names `$P`.
- **The ledger:** `$P/ledger-44x.md`. Every ruling made while executing goes
  there, with its cost if wrong; so do each task's figures.
- **Commits.** Each task's message is written to `$P/msg-44-<task>.txt`:
  the subject the task names, a blank line, and the session's attribution
  lines. Then `git add -A`; `git status --short` lists only the task's
  files; and `git commit -F "$P/msg-44-<task>.txt"`.
- **The baseline's full output** is `$P/base-44.txt` (Task 0).
- **Runs.** Cargo runs with `CARGO_INCREMENTAL=0`, and
  - `T` stands for `cargo test -p nova-typeck --lib`;
  - `W` for `NOVA_REQUIRE_NODE=1 cargo test -p nova-cli --test wasm_tests --`,
    which a step follows with its filters;
  - `X` for `cargo test -p nova-driver --test index_complete`.
- **The patches.** Each task's code is in patch blocks below, each marked
  `<!-- patch NAME -->` and fenced with four backticks. They are the
  replay's commits, so apply them; do not retype them. From the
  repository's root:

  ```sh
  PLAN=docs/superpowers/plans/2026-10-17-phase-4-4-component-syntax.md   # this file, committed
  block() {
    awk -v name="$1" '
      $0 == "<!-- patch " name " -->" { grab = 1; next }
      grab && /^````diff$/ { inside = 1; next }
      inside && /^````$/ { exit }
      inside { print }
    ' "$PLAN" > "$P/$1.patch"
  }
  patch_block() {
    block "$1" && git apply --check "$P/$1.patch" && git apply "$P/$1.patch"
  }
  ```

  `patch_block t1-tests` applies the block named `t1-tests`. A block that
  does not apply means the tree differs from the plan's: stop, and record
  why in the ledger.
- **Expected outputs are written once, from a run, and read.** Each
  `.stdout` under `tests/runtime/` and `tests/dom/` is in its task's test
  patch; the replay wrote each from its program's own run, natively for
  `tests/runtime/` and under the fake DOM for `tests/dom/`, and each was
  read against the spec.
- **Each task** is test first: its tests' patch, their run and the expected
  failure, then its code's patch, its run and the expected pass. Unit tests
  in the files a task changes ride in its code's patch, and Task 7's
  mutants show they bite. Each task then runs its checks:
  - `cargo fmt --all -- --check`;
  - the workspace's clippy, `cargo clippy --locked --all-targets
    --all-features -- -D warnings`, as CI runs it;
  - `cargo run --locked -q -p nova-cli -- fmt --check std examples
    tests/wasm tests/dom`, when it changes a `.nova` file, and `cargo run
    --locked -q -p nova-cli -- fmt --check tests/runtime/ui_*.nova` when it
    adds one there.

  It ends with one commit. A difference from an `Expected:` is fixed in
  the smallest way and recorded in the ledger as a ruling, with its cost if
  wrong.

## Review Focus

The inputs most likely to bite a user, most likely first. Each has its test
in the task that owns the code:

1. **A view that means other than what it says:** a literal made reactive
   or a value left fixed, an attribute under the wrong name, `value` not
   the property, a handler of the wrong kind, `text` that is a call →
   `a_views_parts_are_fixed_or_read_again_by_literalness` and
   `tests/runtime/ui_view.nova` (Task 2), mutants 16 to 25.
2. **Code that parsed before parsing otherwise:** `component` and `view`
   as names, in an `if` head, before a call; `||` as the logical or →
   `component_and_view_are_names_elsewhere`,
   `view_before_an_if_body_is_a_name` and
   `pipe_pipe_is_a_closure_where_an_expression_starts_and_an_or_elsewhere`
   (Task 1), `component_and_view_are_keywords_only_where_they_are`
   (Task 3), mutants 1, 2, 14 and 40 to 42.
3. **A child that runs into the next,** since line breaks are skipped: the
   parser's `;`, and the formatter's `;` before a child that begins with
   `(`, `[` or `{` → `a_semicolon_ends_a_child_before_a_parenthesis` and
   `view_children_keep_their_separators_and_comments` (Task 1), mutants 4
   and 9.
4. **The formatter changing a program or dropping a comment:** each new
   form under its three checks, the corpus test over every `.nova` file,
   and the reformat done by `nova fmt` alone → `components_and_views_have_one_layout`,
   the corpus test (Task 1), mutants 7 to 13.
5. **An element checked against the wrong function:** a local, a variant,
   a function of other parameters or another return type, all E0096; a
   void element with a block, E0095 → the E0094 to E0097 tests (Task 2),
   mutants 15 and 26 to 34.
6. **A bundle that destroys what is not its own:** `assets/` replaced, or
   the page emptied, in an output directory that holds the project,
   however spelled (`nope/..`, a link), or sits inside `public/`; a write
   through a link already in `dist/`; a link in `public/`, or `public/`
   itself, followed out of the project → `dist_may_not_hold_the_project_or_be_inside_public`,
   `a_projects_own_page_and_public_go_into_dist`,
   `a_public_that_leads_outside_is_not_copied` and
   `the_page_is_never_copied_onto_itself` (Task 4), mutants 46 to 58.
7. **`wasm-opt` run when it should not be, or its failure ignored** →
   `release_runs_wasm_opt_when_it_is_on_path` (Task 4), mutants 62 to 65.
8. **The index:** an element's name hovers as its builder, `text` as
   `std/ui/html`'s, and a named argument is no symbol →
   `an_element_hovers_as_its_builder` (Task 3), `index_complete` (Tasks 2
   and 5), mutants 35 to 39.
9. **The gate's counter not counting, or growing past 50 KB** →
   `the_counter_spa_counts_clicks` and `the_counter_spa_is_under_50_kb_gzipped`
   (Task 5), mutants 66 and 67.

## Decisions

Made while prototyping and replaying, each against the spec. Where one
departs from the spec, it says so and why; ADR 0043 records the syntax's
departures from `30-FRONTEND.md`.

1. **A component is a `Function` with `is_component` set,** not an
   `Item::Component` (spec §10). Every reader of a function, the resolver,
   the index, hover and `nova doc`, reads a component as the function it
   is; only the parser, the formatter and the type checker's return type
   look at the flag. `return_ty` is `None`, and the checker gives it
   `std/ui`'s `View`. `component` is a keyword where an item starts, a name
   and `(` follow it, **and `{` (or a refused `->`) follows its
   parameters**, where spec §3.1 asked only for the name and `(`: line
   breaks are skipped, so a statement that is a variable named
   `component`, followed on the next line by a call `f(x)`, read as an
   item without it (found by the fact-check). One such case is left: a
   variable `component`, then a call, then a bare block, each on its own
   line, reads as a component, since the parser does not see line breaks;
   ADR 0043 names it beside a record named `view`.
2. **A view's children may be separated by `;`,** where spec §3.2 says
   "nothing but space": line breaks are skipped, so a child that begins
   with `(`, `[` or `{` would continue the one before (spec §12). The
   formatter writes `;` before such a child, from the statements' own
   `CONTINUES` list, which gains `||`.
3. **An element's one child stays on its line only when the whole element
   fits** (spec §5), as a one-statement function body does: one group for
   the element. Its arguments are a group of their own, so they stay on
   one line when they fit even where the block breaks. A call whose
   argument is a closure with a view, `show(open, || view { … })`, breaks
   its arguments one per line, as a call with a block-bodied closure
   already does: no last argument is hugged.
4. **An element names a function in scope,** resolved as a call's callee
   is: a local of that name, a variant or any other definition is E0096; a
   name that resolves to nothing is E0001, or E0092 where the target lacks
   its module.
5. **A builder is checked by its signature exactly:** no type parameters,
   parameters `([Attr], [View])` or `([Attr])`, and the return type
   `View`. The call is built in HIR directly, a `Call` of the builder with
   one or two `MakeArray`s, so nothing is checked twice.
6. **Handlers:** `on_kind: h` with `h: fn()` is a call of `std/ui/html`'s
   `on(kind, h)`, and with `h: fn(String)` the variant `On(kind, h)`.
   Either must return `()`: `on_click: || 5` is E0097, and so is `on_`
   alone, which names no event. The handler is
   checked before its use is known, so a closure's unannotated parameter
   is not inferred from the attribute: `|v| v.len()` is E0011, "cannot
   infer the receiver's type", where `|v| println(v)` checks, as `println`
   gives `v` its type; a program writes `|v: String|`.
7. **A literal is a string with no interpolation:** a raw string,
   `r"…"`, which the parser makes a `Literal::Str`, or a `StringInterp`
   whose parts are all literal, as it makes every other string. The
   prototype tested `Literal::Str` alone and so made every other literal
   reactive; the replay's HIR test found it, and tests both kinds.
8. **`text(e)` in a view is noted as a use of `std/ui/html`'s `text`,**
   though it is no call of it (spec §4.4), so hover shows that builder's
   doc and `index_complete` finds the name indexed. It needs no import.
9. **Non-`String` text is E0010,** "text must have type `String`, found
   `T`": spec §4.3's E0097 names attributes, and `text` is a child.
10. **The language server finds the keywords by lexing the file and
    asking the index,** not by walking the tree: `component` before the
    name a function declares and `(`, and `view` before `{` where the index
    has no occurrence. Neither the tree nor the index gains a field: a span
    in the tree would break the formatter's tree check, which strips only
    fields named `span`.
11. **`index_complete` skips** `component` before a name and `(`, `view`
    before `{`, and a name before `:` that starts an argument inside a
    call's `(`, not a `fn`'s or a component's parameters (spec §6).
12. **`nova doc` shows a component as written,** `component Card(title:
    String)`, with `component` coloured as a keyword: `keyword_start` knows
    the word, which also puts the "make public" fix's `pub ` before it. The
    page's heading stays `fn Card`, a function's (spec §10: "documented as
    a function").
13. **The bundle is `nova-cli`'s `bundle.rs`,** where spec §10 listed
    `nova-driver` too: it reads the project's layout, which the CLI's
    `project` module owns. `build_program_wasm` changes not at all; the
    generated page gains `#app` in `nova-codegen-wasm`.
14. **`manifest.json` lists every file the build wrote,** `package.json`
    included, which spec §7's example left out, and `assets/…` for a
    project. It is written by hand, keys in the spec's order, with
    `quote`, a JSON string quoter of its own.
15. **An output directory that holds the project, or lies inside
    `public/`, is refused** before anything is written, however it is
    spelled: `-o .`, `-o ..`, `-o nope/..`, `-o src/..`, `-o public/out`. A
    build replaces `assets/` in its output directory and writes
    `index.html` there, and would otherwise delete a directory that is not
    its own, empty the project's page, or copy `public/` into itself.
    `resolved` makes a path absolute, resolves its links as far as it
    exists, and takes each `..` away after that, as `create_dir_all` finds
    it, or by the text first on Windows, as Win32 does, which this machine
    cannot test; and `finish` never copies the page onto itself. Before
    the compiler writes, `prepare` removes a link at each name the build
    writes (`index.html`, `app.js`, `app.wasm`, `package.json`,
    `manifest.json`), so no write goes through one to a file elsewhere.
    Spec §7 did not say. The first check found the first guard let
    `nope/..` through and emptied the project's `index.html`, and the
    second that a link already in `dist/` was written through.
16. **A link in `public/` is copied, as its file, only when it leads to a
    file inside `public/`;** a link to a directory, to anything outside
    `public/`, or to nothing, is skipped; and a `public/` that is itself a
    link leading outside the project is not copied at all, with the line
    `public/ leads outside the project; it is not copied`. Spec §7 named
    only a link to a directory, but meant that "a copy cannot loop or leave
    the project"; the two checks found a link to a file outside, and a
    `public/` that leads outside, deployed.
17. **`wasm-opt` runs in `dist/`** as `wasm-opt -Oz app.wasm -o app.wasm`,
    found on `PATH` by the OS: "not found" prints spec §7's line; any other
    failure to start it, or a failing status, fails the build. The test's
    stand-in is a native Nova program, `wasm-opt.exe` on Windows, that
    prints its arguments and copies its input to its output; "not found"
    is tested with `PATH` set to an empty directory. `nova bundle` takes no
    `--release`, as before: `nova build --target wasm --release` is the
    way.
18. **The size check uses `flate2` at level 9,** as 4.0's hello-world check
    does, where spec §9.4 said Node's `zlib`: it runs without Node, and
    `flate2` is already a dev-dependency.
19. **The 06 harnesses read the count before the clicks too:** the fake
    DOM's prints it before and after each click, `Count: 0` to `Count: 2`,
    and `spa.mjs` waits for `Count: 0`, then `Count: 2`. So a fixed text
    cannot pass, as a replay mutant showed one could when the final count
    alone was read.
20. **The reformat is `nova fmt`'s alone,** over the 17 files with `| |`
    (128 lines); a line that now fits on one is joined by it. Three doc
    comments and one comment in `tests/dom/timers.nova` are edited by
    hand, since the formatter leaves comments as written. `tests/runtime/`
    is outside CI's `nova fmt --check`, and much of it was never
    formatted: only the files that use `| |` change.

---

### Task 0: The baseline

The branch is at this plan's commit, whose parent is `3a5b1ad`, the
approved spec. The plan's commit adds this file alone, so the patches
apply to it as to `3a5b1ad`.

- [ ] **Step 1: The tree.**
  - `git status --short` prints nothing, on
    `claude/loving-archimedes-frtf5q-44`.
  - `git diff --stat 3a5b1ad HEAD` lists only
    `docs/superpowers/plans/2026-10-17-phase-4-4-component-syntax.md`.
  - `BASE=$(git rev-parse --short HEAD)`: the plan's commit, recorded in
    the ledger. Task 7's review reads from it.
  - `df -h /` shows at least 8G free; if not, remove scratch directories
    of earlier sub-phases first.
- [ ] **Step 2: Node and the browser.** `node --version` prints `v22.` and
  more; `ls /opt/node-tools/node_modules/playwright /opt/pw-browsers`
  lists both, for Task 5's local run (skip that run where either is
  missing, and say so in the ledger).
- [ ] **Step 3: The suite.**

  ```sh
  cargo build --locked --workspace
  CARGO_INCREMENTAL=0 NOVA_REQUIRE_NODE=1 cargo test --locked --workspace --no-fail-fast > "$P/base-44.txt" 2>&1
  grep '^test result' "$P/base-44.txt" | awk '{p += $4; f += $6; i += $8} END {print p, f, i}'
  grep ' FAILED$' "$P/base-44.txt"
  ```

  Expected: `2286 1 10`, and one failure, `test
  net::tests::an_ipv6_connect_parks_on_its_first_poll_and_then_establishes
  ... FAILED`. It needs IPv6, which this machine lacks, as in every
  earlier phase.
- [ ] **Step 4: The ledger.** Start `$P/ledger-44x.md`, headed with the
  base commit and the baseline's figures.

### Task 1: The syntax, `||`, and the reformat

Spec §3, §5; decisions 1 to 3 and 20.

**Files:**
- Modify:
  - `crates/nova-ast/src/expr.rs`, `item.rs`, `lib.rs`
  - `crates/nova-parser/src/grammar.rs`
  - `crates/nova-fmt/src/print/expr.rs`, `item.rs`, `mod.rs`
  - `crates/nova-typeck/src/check.rs` (one line: a view is refused until
    Task 2)
  - the 17 `.nova` files that use `| |` (Step 5)
- Test:
  - `crates/nova-parser/tests/parser_tests.rs`, and its three snapshots,
    which gain `is_component: false`
  - `crates/nova-fmt/tests/layout.rs`, `source_rules.rs`

- [ ] **Step 1: The tests.** `patch_block t1-tests`:
  - the parser: `a_component_is_a_function_that_writes_no_return_type`,
    `component_and_view_are_names_elsewhere`,
    `a_component_with_a_return_type_or_type_parameters_does_not_parse`,
    `pipe_pipe_is_a_closure_where_an_expression_starts_and_an_or_elsewhere`,
    `a_view_holds_elements_and_expressions`,
    `a_semicolon_ends_a_child_before_a_parenthesis`,
    `view_before_an_if_body_is_a_name` and
    `a_named_argument_outside_a_view_is_an_error`;
  - the formatter: `components_and_views_have_one_layout` and
    `view_children_keep_their_separators_and_comments`; `| |` prints `||`
    (in `closures_print_their_parameters_bare_or_typed`, with `|| -> Int
    { 2 }`); a statement that begins with `||` keeps its `;`; and
    `tokens_that_would_merge_keep_a_space` loses its `| |` case.
- [ ] **Step 2: Run, and see them fail.**
  - `CARGO_INCREMENTAL=0 cargo test -p nova-parser --test parser_tests`.
    Expected: it does not build: ``error[E0432]: unresolved imports
    `nova_ast::ViewArg`, `nova_ast::ViewNode` ``, and five more errors.
  - `CARGO_INCREMENTAL=0 cargo test -p nova-fmt --no-fail-fast`.
    Expected: `layout` fails 3 of 33 (`closures_print_their_parameters_bare_or_typed`,
    `components_and_views_have_one_layout`,
    `view_children_keep_their_separators_and_comments`), and
    `source_rules` 1 of 21
    (`each_token_that_could_continue_a_statement_keeps_its_semicolon`).
- [ ] **Step 3: The code.** `patch_block t1-code`:
  - **`nova-ast`:** `Expr::View(Vec<Spanned<ViewNode>>)`; `ViewNode` is
    `Element { name, args: Vec<ViewArg>, children: Option<Vec<…>> }` or
    `Expr(Spanned<Expr>)`; `ViewArg` is `Named { name, value }` or
    `Positional(expr)`; `Function::is_component`.
  - **`nova-parser`:**
    - `component_at(i)`: `component`, a name and `(`, and `{` or `->`
      after the parameters' `)` (decision 1); an item, a nested item and
      the dropped-item recovery all know it. `parse_component`
      reads no type parameters; a `->` is the error "a component returns a
      `View`, which it does not write", and its type is read and dropped
      so the body still parses.
    - `parse_atom`: `Pipe` or `PipePipe` starts a closure; `view` before
      `{`, where a record literal may start, starts a view.
    - `parse_view_children` allows record literals inside its braces
      again, and takes a `;` between children. `parse_view_child`: a name
      and `{` is an element; a name and `(` reads arguments, then a block
      makes an element, a named argument makes one without a block, and
      otherwise the call is an expression, continued by
      `parse_postfix_from`. `arg_name` takes a keyword's text too.
  - **`nova-fmt`:** `Expr::View` and its elements (§5, decision 3); `||`;
    `component Name(params) {`; `CONTINUES` gains `PipePipe`.
  - **`nova-typeck`:** `ast::Expr::View(_)` is refused, "`view { … }`
    blocks are not supported yet" (E0900), until Task 2.
- [ ] **Step 4: Run.**
  - `CARGO_INCREMENTAL=0 cargo test -p nova-parser`. Expected: `56
    passed` in `parser_tests`, and the rest as before.
  - `CARGO_INCREMENTAL=0 cargo test -p nova-fmt --no-fail-fast`. Expected:
    all pass; `layout` `33 passed`, `source_rules` `21 passed`, and the
    corpus test `1 passed`.
  - `T`. Expected: `403 passed`, as before.
- [ ] **Step 5: The reformat.** `patch_block t1-reformat`: the 17 files'
  `| |` become `||`, as `nova fmt` itself wrote them (decision 20),
  with three doc comments in `std/ui` and one comment in
  `tests/dom/timers.nova` edited by hand. Then:
  - `cargo run --locked -q -p nova-cli -- fmt --check std examples
    tests/wasm tests/dom` and `cargo run --locked -q -p nova-cli -- fmt
    --check tests/runtime/ui_*.nova`. Expected: both print nothing: the
    formatter agrees with every changed file.
  - `git grep -c '| |' -- '*.nova'`. Expected: nothing.
  - `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests ui_`.
    Expected: `4 passed`.
  - `W ui_`, then `W timers`. Expected: `19 passed`, then `1 passed`.
  - `X`. Expected: `1 passed`.
- [ ] **Step 6: The checks,** as the conventions list them.
- [ ] **Step 7: Commit** `parser, fmt: component, view and || (RFC 0001), and the repository's | | as ||`.

**Patch `t1-tests`:**

<!-- patch t1-tests -->

````diff
diff --git a/crates/nova-fmt/tests/layout.rs b/crates/nova-fmt/tests/layout.rs
index d2f88d6a71a9019967cab221de3f60200debf69d..22fb55b9b1b581953f1b32a2863ff9ca0fce4c9b 100644
--- a/crates/nova-fmt/tests/layout.rs
+++ b/crates/nova-fmt/tests/layout.rs
@@ -190,8 +190,8 @@ fn small_forms_keep_their_usual_spacing() {
 #[test]
 fn closures_print_their_parameters_bare_or_typed() {
     assert_formats(
-        "fn main() { let f = |a: Int, b| a + b\n let g = | | 1\n println(\"${f(1, 2)}${g()}\") }\n",
-        "fn main() {\n    let f = |a: Int, b| a + b\n    let g = | | 1\n    println(\"${f(1, 2)}${g()}\")\n}\n",
+        "fn main() { let f = |a: Int, b| a + b\n let g = | | 1\n let h = || -> Int { 2 }\n println(\"${f(1, 2)}${g()}${h()}\") }\n",
+        "fn main() {\n    let f = |a: Int, b| a + b\n    let g = || 1\n    let h = || -> Int { 2 }\n    println(\"${f(1, 2)}${g()}${h()}\")\n}\n",
     );
 }
 
@@ -260,3 +260,34 @@ fn a_header_comment_stays_above_the_second_group() {
         "import a\n\n// The project's own.\n\nimport b\nimport c\n",
     );
 }
+
+/// `component` and `view { … }` (the 4.4 spec
+/// `docs/superpowers/specs/2026-10-17-phase-4-4-component-syntax-design.md`
+/// §5): a component is laid out as a `fn`; a view's children are one per
+/// line; an element's one child stays on its line when the whole fits, and
+/// its arguments break as a call's do.
+#[test]
+fn components_and_views_have_one_layout() {
+    assert_formats(
+        "pub component Counter( start: Int ) { let count = signal(start)\n view { div(class: \"counter\") { text(\"Count: ${count.value()}\") button(on_click: || count.update(|n| n + 1)) { text(\"+\") } } } }\n",
+        "pub component Counter(start: Int) {\n    let count = signal(start)\n    view {\n        div(class: \"counter\") {\n            text(\"Count: ${count.value()}\")\n            button(on_click: || count.update(|n| n + 1)) { text(\"+\") }\n        }\n    }\n}\n",
+    );
+    assert_formats(
+        "component A() { view { p() { } } }\n",
+        "component A() {\n    view {\n        p {}\n    }\n}\n",
+    );
+    assert_stable(
+        "component A() {\n    view {\n        button(\n            on_click: || count.update(|n| n - 1),\n            class: \"a very long class name that goes on and on\",\n            id: \"and-an-id\",\n        ) {\n            text(\"-\")\n        }\n    }\n}\n",
+    );
+    assert_stable("component A() {\n    view {\n        input(type: \"text\", for: \"x\", value: v, on_input: |v: String| println(v))\n    }\n}\n");
+}
+
+/// A child that begins with `(`, `[` or `{` gets `;` before it, as a
+/// statement does, and comments keep their places among children.
+#[test]
+fn view_children_keep_their_separators_and_comments() {
+    assert_formats(
+        "component A() { view { div { text(\"a\") ; (b) // after b\n // before c\n c } } }\n",
+        "component A() {\n    view {\n        div {\n            text(\"a\");\n            (b) // after b\n            // before c\n            c\n        }\n    }\n}\n",
+    );
+}
diff --git a/crates/nova-fmt/tests/source_rules.rs b/crates/nova-fmt/tests/source_rules.rs
index 90fe13589bbd1376bae3af8124f3a05f38e13894..a7a3602ed19105c83f1ba116b236e6f0d358ac88 100644
--- a/crates/nova-fmt/tests/source_rules.rs
+++ b/crates/nova-fmt/tests/source_rules.rs
@@ -78,10 +78,7 @@ fn tokens_that_would_merge_keep_a_space() {
     // In an expression `&` takes a postfix operand, so a reference to a
     // reference is written `&(&x)` and keeps its parentheses. A type has no
     // such rule, so `& &T` keeps its space.
-    assert_formats(
-        "fn main() { let f = | | 0\n let r = &(&x) }\n",
-        "fn main() {\n    let f = | | 0\n    let r = &(&x)\n}\n",
-    );
+    assert_stable("fn main() { let r = &(&x) }\n");
     assert_stable("fn f(x: & &Int, y: &mut &Int) {}\n");
 }
 
@@ -143,6 +140,10 @@ fn each_token_that_could_continue_a_statement_keeps_its_semicolon() {
             "fn main() { f(); |x| x }\n",
             "fn main() {\n    f();\n    |x| x\n}\n",
         ),
+        (
+            "fn main() { f(); || x }\n",
+            "fn main() {\n    f();\n    || x\n}\n",
+        ),
     ] {
         assert_formats(input, expected);
     }
diff --git a/crates/nova-parser/tests/parser_tests.rs b/crates/nova-parser/tests/parser_tests.rs
index eadcef649c2c5ddb1f2802609009ee96abbc25e7..61a56493fa217a1e7aadacc67f41fedfaf13340f 100644
--- a/crates/nova-parser/tests/parser_tests.rs
+++ b/crates/nova-parser/tests/parser_tests.rs
@@ -937,3 +937,161 @@ fn parse_recovering_agrees_with_parse_on_a_clean_file() {
         format!("{:?}", file.expect("Some"))
     );
 }
+
+// --- `component`, `view { … }` and `||` (RFC 0001; the 4.4 spec
+// `docs/superpowers/specs/2026-10-17-phase-4-4-component-syntax-design.md`
+// §3, §9.2) ---
+
+/// The functions of a clean parse of `source`.
+fn functions(source: &str) -> Vec<nova_ast::item::Function> {
+    let (file, errs) = parse_str(source);
+    assert!(errs.is_empty(), "{errs:?}");
+    file.items
+        .into_iter()
+        .filter_map(|i| match i.value {
+            Item::Function(f) => Some(f),
+            _ => None,
+        })
+        .collect()
+}
+
+/// The trailing expression of the only function in `source`.
+fn tail(source: &str) -> nova_ast::Expr {
+    let f = functions(source).pop().expect("a function");
+    f.body.value.trailing.expect("a trailing expression").value
+}
+
+/// The children of the view that is `source`'s only function's tail.
+fn view_children(source: &str) -> Vec<Spanned<nova_ast::ViewNode>> {
+    match tail(source) {
+        nova_ast::Expr::View(children) => children,
+        other => panic!("not a view: {other:?}"),
+    }
+}
+
+#[test]
+fn a_component_is_a_function_that_writes_no_return_type() {
+    let fs = functions("component Card(title: String) { 1 }\npub component Page() { 2 }\n");
+    assert!(fs[0].is_component && fs[1].is_component);
+    assert_eq!(fs[0].name.value, "Card");
+    assert_eq!(fs[0].params.len(), 1);
+    assert!(fs[0].return_ty.is_none());
+    assert_eq!(fs[1].vis, Visibility::Pub);
+}
+
+#[test]
+fn component_and_view_are_names_elsewhere() {
+    let fs = functions(
+        "fn component() -> Int { 1 }\n\
+         fn view(x: Int) -> Int { x }\n\
+         fn f() -> Int {\n    let component = component()\n    let view = 2\n    if view > component { view } else { 0 }\n}\n\
+         fn g(view: Bool) -> Int {\n    if view { 1 } else { 2 }\n}\n\
+         fn h() {\n    let mut component = 1\n    component\n    g(true)\n}\n",
+    );
+    assert!(fs.iter().all(|f| !f.is_component));
+}
+
+#[test]
+fn a_component_with_a_return_type_or_type_parameters_does_not_parse() {
+    let (_, errs) = parse_str("component A() -> Int { 1 }\nfn main() { }\n");
+    let messages: Vec<String> = errs.iter().map(|e| e.to_string()).collect();
+    assert_eq!(messages.len(), 1, "{messages:?}");
+    assert!(
+        messages[0].contains("a component returns a `View`, which it does not write"),
+        "{messages:?}"
+    );
+    let (_, errs) = parse_str("component A<T>() { 1 }\nfn main() { }\n");
+    assert!(!errs.is_empty());
+}
+
+#[test]
+fn pipe_pipe_is_a_closure_where_an_expression_starts_and_an_or_elsewhere() {
+    match tail("fn f() { || 1 }\n") {
+        nova_ast::Expr::Closure { params, ret, .. } => {
+            assert!(params.is_empty() && ret.is_none())
+        }
+        other => panic!("not a closure: {other:?}"),
+    }
+    match tail("fn f() { || -> Int { 1 } }\n") {
+        nova_ast::Expr::Closure { params, ret, .. } => {
+            assert!(params.is_empty() && ret.is_some())
+        }
+        other => panic!("not a closure: {other:?}"),
+    }
+    assert!(matches!(
+        tail("fn f(a: Bool, b: Bool) -> Bool { a || b }\n"),
+        nova_ast::Expr::Binary {
+            op: nova_ast::expr::BinOp::Or,
+            ..
+        }
+    ));
+}
+
+#[test]
+fn a_view_holds_elements_and_expressions() {
+    use nova_ast::{ViewArg, ViewNode};
+    let children = view_children(
+        "fn f() {\n    view {\n        div(class: \"a\", for: \"b\", type: \"c\", attr(\"d\", \"e\")) {\n            text(\"x\")\n            input(value: v)\n            span { }\n            Counter()\n            show(open, || view { p { } })\n        }\n    }\n}\n",
+    );
+    assert_eq!(children.len(), 1);
+    let ViewNode::Element {
+        name,
+        args,
+        children: Some(inner),
+    } = &children[0].value
+    else {
+        panic!("not an element with a block: {:?}", children[0]);
+    };
+    assert_eq!(name.value, "div");
+    let names: Vec<&str> = args
+        .iter()
+        .filter_map(|a| match a {
+            ViewArg::Named { name, .. } => Some(name.value.as_str()),
+            ViewArg::Positional(_) => None,
+        })
+        .collect();
+    assert_eq!(names, ["class", "for", "type"]);
+    assert!(matches!(args[3], ViewArg::Positional(_)));
+    assert_eq!(inner.len(), 5);
+    assert!(matches!(inner[0].value, ViewNode::Expr(_)));
+    assert!(matches!(
+        &inner[1].value,
+        ViewNode::Element { children: None, .. }
+    ));
+    assert!(matches!(
+        &inner[2].value,
+        ViewNode::Element { args, children: Some(_), .. } if args.is_empty()
+    ));
+    assert!(matches!(inner[3].value, ViewNode::Expr(_)));
+    assert!(matches!(inner[4].value, ViewNode::Expr(_)));
+}
+
+/// Line breaks are skipped, so a child that begins with `(` continues the
+/// one before, unless a `;` ends it (§12).
+#[test]
+fn a_semicolon_ends_a_child_before_a_parenthesis() {
+    assert_eq!(
+        view_children("fn f() { view { text(\"a\") (text(\"b\")) } }\n").len(),
+        1
+    );
+    assert_eq!(
+        view_children("fn f() { view { text(\"a\"); (text(\"b\")) } }\n").len(),
+        2
+    );
+}
+
+/// A `view` in an `if` head is a name: `{` opens the body.
+#[test]
+fn view_before_an_if_body_is_a_name() {
+    assert!(matches!(
+        tail("fn f(view: Bool) -> Int { if view { 1 } else { 2 } }\n"),
+        nova_ast::Expr::If { .. }
+    ));
+}
+
+/// Named arguments exist in an element's argument list only.
+#[test]
+fn a_named_argument_outside_a_view_is_an_error() {
+    let (_, errs) = parse_str("fn main() { f(a: 1) }\n");
+    assert!(!errs.is_empty());
+}
diff --git a/crates/nova-parser/tests/snapshots/parser_tests__snapshot_function_with_generics.snap b/crates/nova-parser/tests/snapshots/parser_tests__snapshot_function_with_generics.snap
index 4af51a6128739e19aa3a9e046f1ceb6286edbce3..835a43dc08764f35914e34aa7f8422cf6f0283e4 100644
--- a/crates/nova-parser/tests/snapshots/parser_tests__snapshot_function_with_generics.snap
+++ b/crates/nova-parser/tests/snapshots/parser_tests__snapshot_function_with_generics.snap
@@ -12,6 +12,7 @@ Some(
                         attrs: [],
                         vis: Private,
                         is_async: false,
+                        is_component: false,
                         name: Spanned {
                             value: "identity",
                             span: Span {
diff --git a/crates/nova-parser/tests/snapshots/parser_tests__snapshot_hello.snap b/crates/nova-parser/tests/snapshots/parser_tests__snapshot_hello.snap
index fdaea2fcb2165ba5480aa2140d309a77d2e08c1d..bf363108b103011d8f56f1a605a6148bc7218653 100644
--- a/crates/nova-parser/tests/snapshots/parser_tests__snapshot_hello.snap
+++ b/crates/nova-parser/tests/snapshots/parser_tests__snapshot_hello.snap
@@ -12,6 +12,7 @@ Some(
                         attrs: [],
                         vis: Private,
                         is_async: false,
+                        is_component: false,
                         name: Spanned {
                             value: "main",
                             span: Span {
diff --git a/crates/nova-parser/tests/snapshots/parser_tests__snapshot_impl_with_associated_type_binding.snap b/crates/nova-parser/tests/snapshots/parser_tests__snapshot_impl_with_associated_type_binding.snap
index 0d1f9b64191e28740947043a79a08f977b3ed29f..37e1739ebbfc30878cd4dac89fe00f5647c1aa9d 100644
--- a/crates/nova-parser/tests/snapshots/parser_tests__snapshot_impl_with_associated_type_binding.snap
+++ b/crates/nova-parser/tests/snapshots/parser_tests__snapshot_impl_with_associated_type_binding.snap
@@ -69,6 +69,7 @@ Some(
                                 attrs: [],
                                 vis: Private,
                                 is_async: false,
+                                is_component: false,
                                 name: Spanned {
                                     value: "get",
                                     span: Span {
````

**Patch `t1-code`:**

<!-- patch t1-code -->

````diff
diff --git a/crates/nova-ast/src/expr.rs b/crates/nova-ast/src/expr.rs
index d14f21388fa2de2bb49f18b68b8f0a9bf33d668b..ef16695e5b1166f9952d75f00c41d9eefa066e1c 100644
--- a/crates/nova-ast/src/expr.rs
+++ b/crates/nova-ast/src/expr.rs
@@ -120,6 +120,37 @@ pub enum Expr {
 
     /// A string with interpolated expressions: `"Hello, ${name}!"`.
     StringInterp(Vec<StringPart>),
+
+    /// `view { … }`: a tree of elements, text and views, which the type
+    /// checker desugars into `std/ui`'s calls (RFC 0001). It holds the
+    /// children as written; the type checker requires exactly one.
+    View(Vec<Spanned<ViewNode>>),
+}
+
+/// A child of `view { … }` (RFC 0001).
+#[derive(Debug, Clone)]
+pub enum ViewNode {
+    /// `name(args) { children }`: an element, built by the function `name`.
+    /// `children` is `None` when no trailing block follows.
+    Element {
+        name: Spanned<String>,
+        args: Vec<ViewArg>,
+        children: Option<Vec<Spanned<ViewNode>>>,
+    },
+    /// Any other child: `text(e)`, or an expression whose value is a `View`.
+    Expr(Spanned<Expr>),
+}
+
+/// An argument of an element in `view { … }`.
+#[derive(Debug, Clone)]
+pub enum ViewArg {
+    /// `name: value`, an attribute, a property or an event's handler.
+    Named {
+        name: Spanned<String>,
+        value: Spanned<Expr>,
+    },
+    /// An `Attr` expression, used as it is.
+    Positional(Spanned<Expr>),
 }
 
 /// A segment of a string-interpolation expression.
diff --git a/crates/nova-ast/src/item.rs b/crates/nova-ast/src/item.rs
index 35f30f963b24b6ea78b721eb4ec5e5eac38f9769..c1d74db2f707e3b9ea7e3fd52e8c536000d65ce4 100644
--- a/crates/nova-ast/src/item.rs
+++ b/crates/nova-ast/src/item.rs
@@ -51,6 +51,9 @@ pub struct Function {
     pub attrs: Vec<Attribute>,
     pub vis: Visibility,
     pub is_async: bool,
+    /// Written `component Name(params) { … }` (RFC 0001): its return type is
+    /// `std/ui`'s `View`, and `return_ty` is `None`.
+    pub is_component: bool,
     pub name: Spanned<String>,
     pub generics: Vec<crate::ty::TypeParam>,
     pub params: Vec<Param>,
diff --git a/crates/nova-ast/src/lib.rs b/crates/nova-ast/src/lib.rs
index 4fbac59e83e6091e87fb8bcaa1f0b227c964e6d6..a639d13c05676d1baacb6d4086a103f89eaa22d6 100644
--- a/crates/nova-ast/src/lib.rs
+++ b/crates/nova-ast/src/lib.rs
@@ -14,7 +14,7 @@ pub mod item;
 pub mod pattern;
 pub mod ty;
 
-pub use expr::{BinOp, Expr, FieldInit, Literal, MatchArm, StringPart, UnOp};
+pub use expr::{BinOp, Expr, FieldInit, Literal, MatchArm, StringPart, UnOp, ViewArg, ViewNode};
 pub use item::{
     AssignOp, AssocTypeBinding, ConstDecl, ExternBlock, Function, ImplBlock, Import,
     Module as ModuleDecl, Param, Record, RecordField, TraitDecl, TraitItem, TypeDecl, Visibility,
diff --git a/crates/nova-fmt/src/print/expr.rs b/crates/nova-fmt/src/print/expr.rs
index 268c45c67daa51b90fcf795acd6e69158fd0f381..1c0caef37cb95a052d3c4d2d9e51c7a43a88d7eb 100644
--- a/crates/nova-fmt/src/print/expr.rs
+++ b/crates/nova-fmt/src/print/expr.rs
@@ -2,7 +2,7 @@
 //! `docs/superpowers/specs/2026-10-07-phase-3-1-formatter-design.md` §5.6,
 //! §6).
 
-use nova_ast::expr::{AssignOp, BinOp, Expr, FieldInit, MatchArm, UnOp};
+use nova_ast::expr::{AssignOp, BinOp, Expr, FieldInit, MatchArm, UnOp, ViewArg, ViewNode};
 use nova_ast::item::Param;
 use nova_ast::ty::Type;
 use nova_ast::{Block, Path, Stmt};
@@ -142,6 +142,7 @@ impl<'s, 't> Printer<'s, 't> {
                 text(format!(" {} ", assign_op(*op))),
                 self.expr(rhs),
             ]),
+            Expr::View(children) => self.view(children, span),
         }
     }
 
@@ -501,8 +502,7 @@ impl<'s, 't> Printer<'s, 't> {
     ) -> Doc {
         let mut parts = Vec::new();
         if params.is_empty() {
-            // `||` is one token, which no closure starts with (spec §5.3).
-            parts.push(text("| |"));
+            parts.push(text("||"));
         } else {
             parts.push(text("|"));
             for (i, p) in params.iter().enumerate() {
@@ -522,6 +522,110 @@ impl<'s, 't> Printer<'s, 't> {
         group(concat(parts))
     }
 
+    /// `view { … }`: each child on its own line, indented (the 4.4 spec
+    /// §5). `span` is the view's, from `view` to its `}`.
+    fn view(&mut self, children: &[Spanned<ViewNode>], span: Span) -> Doc {
+        let open = self
+            .src
+            .find_from(span.start, &Token::LBrace)
+            .unwrap_or(span.start);
+        let lead = self.leading(open);
+        let close = span.end - 1;
+        if children.is_empty() && !self.has_comment_before(close) {
+            return concat(vec![text("view "), lead, text("{}")]);
+        }
+        let body = self.view_children(children, open, close);
+        concat(vec![text("view "), lead, body])
+    }
+
+    /// The `{ … }` of a view or an element, from its `{` at `open` to its
+    /// `}` at `close`, one child per line. A child that would continue the
+    /// one before it gets `;` before it, as a statement does (§5.6).
+    fn view_children(&mut self, children: &[Spanned<ViewNode>], open: u32, close: u32) -> Doc {
+        let after_open = self.trailing(open + 1);
+        let mut v = Vertical::new(open + 1, Blank::Keep);
+        for (i, child) in children.iter().enumerate() {
+            let sep = children
+                .get(i + 1)
+                .filter(|n| CONTINUES.contains(self.src.token_from(n.span.start)))
+                .map(|_| ";");
+            let end = self
+                .src
+                .token_end(child.span.end, &Token::Semicolon)
+                .unwrap_or(child.span.end);
+            self.vertical_item(&mut v, child.span.start, end, sep, |p| p.view_node(child));
+        }
+        let inner = self.vertical_finish(v, close);
+        concat(vec![
+            text("{"),
+            after_open,
+            nest(concat(vec![Doc::HardLine, inner])),
+            Doc::HardLine,
+            text("}"),
+        ])
+    }
+
+    fn view_node(&mut self, n: &Spanned<ViewNode>) -> Doc {
+        match &n.value {
+            ViewNode::Expr(e) => self.expr(e),
+            ViewNode::Element {
+                name,
+                args,
+                children,
+            } => self.element(name, args, children.as_deref(), n.span),
+        }
+    }
+
+    /// An element, `name(args) { … }`, whose span is `span`. Its one child
+    /// may share its line when no comment is inside; its arguments break as
+    /// a call's do (the 4.4 spec §5).
+    fn element(
+        &mut self,
+        name: &Spanned<String>,
+        args: &[ViewArg],
+        children: Option<&[Spanned<ViewNode>]>,
+        span: Span,
+    ) -> Doc {
+        let mut parts = vec![self.name(name)];
+        let mut after = name.span.end;
+        if let Some(last) = args.last() {
+            let end = view_arg_extent(last).1;
+            let close = self.src.find_from(end, &Token::RParen).unwrap_or(end);
+            parts.push(group(self.comma_list(
+                List::PARENS,
+                args,
+                |_, a| view_arg_extent(a),
+                |p, a| match a {
+                    ViewArg::Named { name, value } => {
+                        concat(vec![p.name(name), text(": "), p.expr(value)])
+                    }
+                    ViewArg::Positional(e) => p.expr(e),
+                },
+                close,
+            )));
+            after = close + 1;
+        }
+        if let Some(children) = children {
+            let open = self.src.find_from(after, &Token::LBrace).unwrap_or(after);
+            let close = span.end - 1;
+            parts.push(text(" "));
+            parts.push(self.leading(open));
+            let commented = self.has_comment_before(close);
+            if children.is_empty() && !commented {
+                parts.push(text("{}"));
+            } else if children.len() == 1 && !commented {
+                let d = self.view_node(&children[0]);
+                parts.push(text("{"));
+                parts.push(nest(concat(vec![Doc::Line, d])));
+                parts.push(Doc::Line);
+                parts.push(text("}"));
+            } else {
+                parts.push(self.view_children(children, open, close));
+            }
+        }
+        group(concat(parts))
+    }
+
     /// `Path { field: value, short, ..base }`: no comma after `..base`
     /// (spec §6).
     fn record_literal(
@@ -614,6 +718,14 @@ impl<'s, 't> Printer<'s, 't> {
     }
 }
 
+/// Where an element's argument starts and ends in the source.
+fn view_arg_extent(a: &ViewArg) -> (u32, u32) {
+    match a {
+        ViewArg::Named { name, value } => (name.span.start, value.span.end),
+        ViewArg::Positional(e) => (e.span.start, e.span.end),
+    }
+}
+
 /// Binding strength, weakest first, as the parser's layers have it
 /// (`grammar.rs:1324-1527`).
 fn precedence(op: BinOp) -> u8 {
diff --git a/crates/nova-fmt/src/print/item.rs b/crates/nova-fmt/src/print/item.rs
index cfde5079321fb280f80372766a8a7e770b479f94..47d3cecd6087ef1c7d36c8f88b5f603834ac6622 100644
--- a/crates/nova-fmt/src/print/item.rs
+++ b/crates/nova-fmt/src/print/item.rs
@@ -89,13 +89,20 @@ impl<'s, 't> Printer<'s, 't> {
     /// one-statement body stays on one line only when the whole function
     /// fits (spec §6, "Blocks"; the 3.1 plan's decision 6).
     fn function(&mut self, f: &Function) -> Doc {
-        let keyword = self.src.start_with(f.name.span.start, &FN_KEYWORDS);
+        let (keyword, words) = if f.is_component {
+            let keywords = [Token::Ident("component".into()), Token::Pub];
+            let keyword = self.src.start_with(f.name.span.start, &keywords);
+            (keyword, format!("{}component ", vis(f.vis)))
+        } else {
+            let keyword = self.src.start_with(f.name.span.start, &FN_KEYWORDS);
+            let words = format!(
+                "{}{}fn ",
+                vis(f.vis),
+                if f.is_async { "async " } else { "" }
+            );
+            (keyword, words)
+        };
         let head = self.item_head(&f.docs, &f.attrs, keyword);
-        let words = format!(
-            "{}{}fn ",
-            vis(f.vis),
-            if f.is_async { "async " } else { "" }
-        );
         let sig = self.signature(
             &f.name,
             &f.generics,
diff --git a/crates/nova-fmt/src/print/mod.rs b/crates/nova-fmt/src/print/mod.rs
index 2253ac670606dde9f92285c234c1b89ac563442a..b3b67de5d12386f05bb571a70ffc46c267421916 100644
--- a/crates/nova-fmt/src/print/mod.rs
+++ b/crates/nova-fmt/src/print/mod.rs
@@ -21,7 +21,7 @@ use crate::source::Source;
 
 /// Tokens that would continue the statement or match arm before them, since
 /// the parser ignores line breaks (spec §5.6).
-const CONTINUES: [Token; 7] = [
+const CONTINUES: [Token; 8] = [
     Token::LParen,
     Token::LBracket,
     Token::LBrace,
@@ -29,6 +29,7 @@ const CONTINUES: [Token; 7] = [
     Token::Star,
     Token::Amp,
     Token::Pipe,
+    Token::PipePipe,
 ];
 
 /// Print `src`'s AST in the canonical layout (spec §6): `\n` line endings
diff --git a/crates/nova-parser/src/grammar.rs b/crates/nova-parser/src/grammar.rs
index 2bb28f9758c0c78899ea31d11b9371746d743734..f594f919a7f8a62e59bf73542831b9099b9e35f7 100644
--- a/crates/nova-parser/src/grammar.rs
+++ b/crates/nova-parser/src/grammar.rs
@@ -5,7 +5,9 @@
 //! (assignment → or → and → … → postfix), matching the spec table exactly.
 
 use nova_ast::{
-    expr::{AssignOp, BinOp, Expr, FieldInit, Literal, MatchArm, StringPart, UnOp},
+    expr::{
+        AssignOp, BinOp, Expr, FieldInit, Literal, MatchArm, StringPart, UnOp, ViewArg, ViewNode,
+    },
     item::{
         AssocTypeBinding, Attribute, ConstDecl, ExternBlock, ExternItem, Function, FunctionSig,
         ImplBlock, Import, ImportKind, Module, Param, Record, RecordField, TraitDecl, TraitItem,
@@ -148,6 +150,9 @@ impl<'a> Parser<'a> {
     /// or a closing delimiter. Used at item level for error recovery.
     fn sync_to_item_boundary(&mut self) {
         while !self.is_at_end() {
+            if self.component_at(self.pos) {
+                break;
+            }
             match self.peek() {
                 Token::Fn
                 | Token::Pub
@@ -234,9 +239,45 @@ impl<'a> Parser<'a> {
             .is_some_and(|t| std::mem::discriminant(&t.value) == std::mem::discriminant(tok))
     }
 
+    /// Whether the tokens at `i` are `component Name (`, where `component` is
+    /// a keyword (RFC 0001); anywhere else it is an identifier.
+    fn component_at(&self, i: usize) -> bool {
+        let at = |k: usize| self.tokens.get(k).map(|t| &t.value);
+        if !(matches!(at(i), Some(Token::Ident(w)) if w == "component")
+            && matches!(at(i + 1), Some(Token::Ident(_)))
+            && matches!(at(i + 2), Some(Token::LParen)))
+        {
+            return false;
+        }
+        // Its parameters' `)`, then its body's `{`, or a `->` that
+        // `parse_component` refuses. Line breaks are skipped, so without
+        // this a statement `component`, a variable, followed by a call
+        // `f(x)` on the next line would read as an item.
+        let mut depth = 0usize;
+        let mut k = i + 2;
+        loop {
+            match at(k) {
+                Some(Token::LParen) => depth += 1,
+                Some(Token::RParen) => {
+                    depth -= 1;
+                    if depth == 0 {
+                        break;
+                    }
+                }
+                None | Some(Token::Eof) => return false,
+                _ => {}
+            }
+            k += 1;
+        }
+        matches!(at(k + 1), Some(Token::LBrace | Token::Arrow))
+    }
+
     /// Whether the token at `i` starts an item that may appear inside a block.
     fn is_item_start_at(&self, i: usize) -> bool {
         let at = |k: usize| self.tokens.get(k).map(|t| &t.value);
+        if self.component_at(i) || (matches!(at(i), Some(Token::Pub)) && self.component_at(i + 1)) {
+            return true;
+        }
         matches!(
             at(i),
             Some(
@@ -356,11 +397,13 @@ impl<'a> Parser<'a> {
     /// §3.2). Only the tokens the failed attempt consumed are read.
     fn note_dropped_item(&mut self, start: usize) {
         let consumed = &self.tokens[start..self.pos.min(self.tokens.len())];
-        let keyword = consumed.iter().position(|t| {
+        let keyword = consumed.iter().enumerate().position(|(k, t)| {
             matches!(
                 t.value,
                 Token::Fn | Token::Record | Token::Type | Token::Trait | Token::Const
-            )
+            ) || (matches!(&t.value, Token::Ident(w) if w == "component")
+                && matches!(consumed.get(k + 1).map(|n| &n.value), Some(Token::Ident(_)))
+                && matches!(consumed.get(k + 2).map(|n| &n.value), Some(Token::LParen)))
         });
         if let Some(k) = keyword {
             if let Some(Token::Ident(name)) = consumed.get(k + 1).map(|t| &t.value) {
@@ -479,6 +522,12 @@ impl<'a> Parser<'a> {
                 e.docs = docs;
                 Item::Extern(e)
             }
+            Token::Ident(_) if self.component_at(self.pos) => {
+                let mut func = self.parse_component(vis)?;
+                func.attrs = attrs;
+                func.docs = docs;
+                Item::Function(func)
+            }
             _ => {
                 let span = self.peek_span();
                 if let Some(first) = docs.first() {
@@ -540,6 +589,7 @@ impl<'a> Parser<'a> {
             attrs: Vec::new(),
             vis,
             is_async,
+            is_component: false,
             name,
             generics,
             params,
@@ -549,6 +599,38 @@ impl<'a> Parser<'a> {
         })
     }
 
+    /// `component Name(params) { body }` (RFC 0001): a function returning
+    /// `std/ui`'s `View`, with no type parameters and no written return type.
+    fn parse_component(&mut self, vis: Visibility) -> Option<Function> {
+        self.advance(); // `component`
+        let name = self.parse_ident("component name")?;
+        self.expect(&Token::LParen, "component parameters")?;
+        let params = self.parse_params();
+        self.expect(&Token::RParen, "component parameters")?;
+        if let Some(arrow) = self.eat(&Token::Arrow) {
+            self.errors.push(ParseError::Custom {
+                message: "a component returns a `View`, which it does not write".into(),
+                span: arrow.span,
+            });
+            // The type is read and dropped, so that the body still parses.
+            self.parse_type("return type")?;
+        }
+        let body = self.parse_block("component body")?;
+        Some(Function {
+            docs: Vec::new(),
+            attrs: Vec::new(),
+            vis,
+            is_async: false,
+            is_component: true,
+            name,
+            generics: Vec::new(),
+            params,
+            return_ty: None,
+            where_clause: Vec::new(),
+            body,
+        })
+    }
+
     fn parse_function_sig(&mut self) -> Option<FunctionSig> {
         let is_async = self.eat(&Token::Async).is_some();
         self.expect(&Token::Fn, "function signature")?;
@@ -1786,8 +1868,18 @@ impl<'a> Parser<'a> {
 
     fn parse_postfix(&mut self, ctx: &str) -> Option<Spanned<Expr>> {
         let start = self.peek_span();
-        let mut expr = self.parse_atom(ctx)?;
-
+        let expr = self.parse_atom(ctx)?;
+        self.parse_postfix_from(start, expr, ctx)
+    }
+
+    /// The calls, indexes, fields, `.await`s and `?`s after `expr`, which
+    /// began at `start`.
+    fn parse_postfix_from(
+        &mut self,
+        start: Span,
+        mut expr: Spanned<Expr>,
+        ctx: &str,
+    ) -> Option<Spanned<Expr>> {
         loop {
             let cur_span = expr.span;
             match self.peek().clone() {
@@ -2085,8 +2177,20 @@ impl<'a> Parser<'a> {
                 Some(Spanned::new(Expr::Continue, start))
             }
 
-            // Closure: |params| body
-            Token::Pipe => self.parse_closure(ctx),
+            // Closure: |params| body, or `||` with none (RFC 0001).
+            Token::Pipe | Token::PipePipe => self.parse_closure(ctx),
+
+            // `view { … }`, where `view` is a keyword (RFC 0001).
+            Token::Ident(w)
+                if w == "view"
+                    && !self.no_struct_literal
+                    && matches!(
+                        self.tokens.get(self.pos + 1).map(|t| &t.value),
+                        Some(Token::LBrace)
+                    ) =>
+            {
+                self.parse_view()
+            }
 
             // Path or record literal
             Token::Ident(_) | Token::SelfLower | Token::SelfUpper => {
@@ -2243,8 +2347,30 @@ impl<'a> Parser<'a> {
 
     fn parse_closure(&mut self, ctx: &str) -> Option<Spanned<Expr>> {
         let start = self.peek_span();
-        self.expect(&Token::Pipe, "closure")?;
         let mut params = Vec::new();
+        if self.eat(&Token::PipePipe).is_none() {
+            self.expect(&Token::Pipe, "closure")?;
+            self.parse_closure_params(&mut params, ctx)?;
+        }
+        let ret = if self.eat(&Token::Arrow).is_some() {
+            Some(self.parse_type(ctx)?)
+        } else {
+            None
+        };
+        let body = self.parse_expr(ctx)?;
+        let span = start.merge(body.span);
+        Some(Spanned::new(
+            Expr::Closure {
+                params,
+                ret,
+                body: Box::new(body),
+            },
+            span,
+        ))
+    }
+
+    /// A closure's parameters, after its `|`, up to and with its closing `|`.
+    fn parse_closure_params(&mut self, params: &mut Vec<Param>, ctx: &str) -> Option<()> {
         while !self.check(&Token::Pipe) && !self.is_at_end() {
             if self.reject_docs() {
                 continue;
@@ -2264,21 +2390,163 @@ impl<'a> Parser<'a> {
             }
         }
         self.expect(&Token::Pipe, "closure parameters end")?;
-        let ret = if self.eat(&Token::Arrow).is_some() {
-            Some(self.parse_type(ctx)?)
-        } else {
-            None
-        };
-        let body = self.parse_expr(ctx)?;
-        let span = start.merge(body.span);
-        Some(Spanned::new(
-            Expr::Closure {
-                params,
-                ret,
-                body: Box::new(body),
-            },
-            span,
-        ))
+        Some(())
+    }
+
+    /// The span of the token just consumed, or `fallback` at the start.
+    fn prev_span_or(&self, fallback: Span) -> Span {
+        self.tokens
+            .get(self.pos.saturating_sub(1))
+            .map(|s| s.span)
+            .unwrap_or(fallback)
+    }
+
+    /// `view { child* }` (RFC 0001), at `view`.
+    fn parse_view(&mut self) -> Option<Spanned<Expr>> {
+        let start = self.peek_span();
+        self.advance(); // `view`
+        let children = self.parse_view_children()?;
+        let span = start.merge(self.prev_span_or(start));
+        Some(Spanned::new(Expr::View(children), span))
+    }
+
+    /// `{ child* }`, the children of a `view` or an element. A `{` inside is
+    /// an element's or a block's again, whatever context the view is in.
+    fn parse_view_children(&mut self) -> Option<Vec<Spanned<ViewNode>>> {
+        self.expect(&Token::LBrace, "view children")?;
+        let saved = std::mem::replace(&mut self.no_struct_literal, false);
+        let mut children = Vec::new();
+        while !self.check(&Token::RBrace) && !self.is_at_end() {
+            // A `;` may end a child, as one ends a statement, so that the
+            // next can start with `(`, `[` or `{`.
+            if self.reject_docs() || self.eat(&Token::Semicolon).is_some() {
+                continue;
+            }
+            match self.parse_view_child() {
+                Some(child) => children.push(child),
+                None => {
+                    self.no_struct_literal = saved;
+                    return None;
+                }
+            }
+        }
+        self.no_struct_literal = saved;
+        self.expect(&Token::RBrace, "view children")?;
+        Some(children)
+    }
+
+    /// One child of a view: an element, or a postfix expression.
+    fn parse_view_child(&mut self) -> Option<Spanned<ViewNode>> {
+        let start = self.peek_span();
+        if let Token::Ident(name) = self.peek().clone() {
+            let next = self.tokens.get(self.pos + 1).map(|t| t.value.clone());
+            if name != "view" && matches!(next, Some(Token::LBrace)) {
+                let name = Spanned::new(name, start);
+                self.advance();
+                let children = self.parse_view_children()?;
+                let span = start.merge(self.prev_span_or(start));
+                return Some(Spanned::new(
+                    ViewNode::Element {
+                        name,
+                        args: Vec::new(),
+                        children: Some(children),
+                    },
+                    span,
+                ));
+            }
+            if matches!(next, Some(Token::LParen)) {
+                let name = Spanned::new(name, start);
+                self.advance(); // the name
+                self.advance(); // `(`
+                let args = self.parse_view_args()?;
+                let named = args.iter().any(|a| matches!(a, ViewArg::Named { .. }));
+                if self.check(&Token::LBrace) {
+                    let children = self.parse_view_children()?;
+                    let span = start.merge(self.prev_span_or(start));
+                    return Some(Spanned::new(
+                        ViewNode::Element {
+                            name,
+                            args,
+                            children: Some(children),
+                        },
+                        span,
+                    ));
+                }
+                let span = start.merge(self.prev_span_or(start));
+                if named {
+                    return Some(Spanned::new(
+                        ViewNode::Element {
+                            name,
+                            args,
+                            children: None,
+                        },
+                        span,
+                    ));
+                }
+                // Positional arguments alone and no block: an ordinary call.
+                let args = args
+                    .into_iter()
+                    .map(|a| match a {
+                        ViewArg::Positional(e) => e,
+                        ViewArg::Named { value, .. } => value,
+                    })
+                    .collect();
+                let callee = Spanned::new(Expr::Path(Path::single(name.clone())), name.span);
+                let call = Spanned::new(
+                    Expr::Call {
+                        callee: Box::new(callee),
+                        args,
+                    },
+                    span,
+                );
+                let expr = self.parse_postfix_from(start, call, "view child")?;
+                let span = expr.span;
+                return Some(Spanned::new(ViewNode::Expr(expr), span));
+            }
+        }
+        let expr = self.parse_postfix("view child")?;
+        let span = expr.span;
+        Some(Spanned::new(ViewNode::Expr(expr), span))
+    }
+
+    /// An element's arguments, after its `(`, up to and with its `)`:
+    /// `name: value` or an expression, separated by commas.
+    fn parse_view_args(&mut self) -> Option<Vec<ViewArg>> {
+        let mut args = Vec::new();
+        while !self.check(&Token::RParen) && !self.is_at_end() {
+            let colon_next = matches!(
+                self.tokens.get(self.pos + 1).map(|t| &t.value),
+                Some(Token::Colon)
+            );
+            if let (Some(word), true) = (self.arg_name(), colon_next) {
+                let name = Spanned::new(word, self.peek_span());
+                self.advance(); // the name
+                self.advance(); // `:`
+                let value = self.parse_expr("element argument")?;
+                args.push(ViewArg::Named { name, value });
+            } else {
+                args.push(ViewArg::Positional(self.parse_expr("element argument")?));
+            }
+            if self.eat(&Token::Comma).is_none() {
+                break;
+            }
+        }
+        self.expect(&Token::RParen, "element arguments")?;
+        Some(args)
+    }
+
+    /// The word at the cursor, if it can name an element's argument: an
+    /// identifier, or a keyword such as `type` or `for`.
+    fn arg_name(&self) -> Option<String> {
+        match self.peek() {
+            Token::Ident(w) => Some(w.clone()),
+            other => {
+                let word = other.description().trim_matches('`');
+                nova_lexer::KEYWORDS
+                    .contains(&word)
+                    .then(|| word.to_string())
+            }
+        }
     }
 
     fn parse_record_literal(&mut self, path: Path, start: Span) -> Option<Spanned<Expr>> {
diff --git a/crates/nova-typeck/src/check.rs b/crates/nova-typeck/src/check.rs
index fddb1608fafd3060a2965db2b712788fae944d83..a14c7af6ced942e989b273fdab36d4cb89bd5be2 100644
--- a/crates/nova-typeck/src/check.rs
+++ b/crates/nova-typeck/src/check.rs
@@ -3576,6 +3576,7 @@ impl<'a> Checker<'a> {
                 }
             }
             ast::Expr::Cast { .. } => self.unsupported_expr(span, "`as` casts"),
+            ast::Expr::View(_) => self.unsupported_expr(span, "`view { … }` blocks"),
         }
     }
 
````

**Patch `t1-reformat`:**

<!-- patch t1-reformat -->

````diff
diff --git a/std/ui/backend/wasm.nova b/std/ui/backend/wasm.nova
index b1cb2f3a3b614f4d0aa05a5b25fc26f511d6d0fe..68aedd123ae0f749147e50d8f5fd8795ca216eb1 100644
--- a/std/ui/backend/wasm.nova
+++ b/std/ui/backend/wasm.nova
@@ -65,6 +65,6 @@ pub fn backend_listen(node: Node, kind: String, f: fn(String)) -> fn() {
             }
         },
     )
-    let stop = | | listener.remove()
+    let stop = || listener.remove()
     stop
 }
diff --git a/std/ui/html/lib.nova b/std/ui/html/lib.nova
index ce431c287ef57354fde41f51423957ba55a6b607..a074c420f46c8242fa98b976be73a8a07d7487ba 100644
--- a/std/ui/html/lib.nova
+++ b/std/ui/html/lib.nova
@@ -20,7 +20,7 @@
 //! import ui
 //! import ui::html::{class, li, text, ul}
 //!
-//! let html = render_to_string(| | ul([class("menu")], [li([], [text("a & b")])]))
+//! let html = render_to_string(|| ul([class("menu")], [li([], [text("a & b")])]))
 //! assert_eq(html, "<ul class=\"menu\"><li>a &amp; b</li></ul>")
 //! ```
 
diff --git a/std/ui/lib.nova b/std/ui/lib.nova
index 27df81049c82db0365347be7d1d239ae93adea75..0b246b1342580947c1edbff64f2fc1785f7f694b 100644
--- a/std/ui/lib.nova
+++ b/std/ui/lib.nova
@@ -30,11 +30,11 @@
 //!
 //! let count = signal(1)
 //! let seen = signal(0)
-//! effect(| | seen.set(count.value() * 10))
+//! effect(|| seen.set(count.value() * 10))
 //! count.set(2)
 //! assert_eq(seen.value(), 20)
 //!
-//! let html = render_to_string(| | div([], [text("count: "), dyn_text(| | "${count.value()}")]))
+//! let html = render_to_string(|| div([], [text("count: "), dyn_text(|| "${count.value()}")]))
 //! assert_eq(html, "<div>count: 2</div>")
 //! ```
 
@@ -400,7 +400,7 @@ impl<T> Signal<T> {
     /// computation, which runs again when the value changes.
     pub fn value(self) -> T {
         match self.src.memo {
-            Some(m) => { if m.state != CLEAN { batch(| | refresh(m)) } }
+            Some(m) => { if m.state != CLEAN { batch(|| refresh(m)) } }
             None => {}
         }
         track(self.src)
@@ -437,18 +437,18 @@ pub fn signal<T>(initial: T) -> Signal<T> {
 pub fn effect(f: fn()) {
     let o = new_owner(EFFECT, f)
     adopt(o)
-    if !o.disposed { batch(| | run_owner(o)) }
+    if !o.disposed { batch(|| run_owner(o)) }
 }
 
 /// A signal whose value is `f`'s: `f` runs now, then again when it is read
 /// after a signal it read changed. Its `set` panics.
 pub fn memo<T>(f: fn() -> T) -> Signal<T> {
-    let mut o = new_owner(MEMO, | | {})
+    let mut o = new_owner(MEMO, || {})
     adopt(o)
     let first = with_context(Some(o), Some(o), f)
     let src = Source { id: new_id(), observers: Vec::new(), memo: Some(o) }
     let s = Signal { src: src, val: first }
-    o.run = | | {
+    o.run = || {
         let mut t = s
         t.val = f()
     }
@@ -524,8 +524,8 @@ fn run_mounted(start: Int) {
 /// Runs `f` with a new owner, which no other owns, and gives `f` the
 /// function that disposes of it, in a `batch`; returns what `f` returns.
 pub fn root<T>(f: fn(fn()) -> T) -> T {
-    let o = new_owner(ROOT, | | {})
-    with_context(Some(o), None, | | f(| | batch(| | dispose(o))))
+    let o = new_owner(ROOT, || {})
+    with_context(Some(o), None, || f(|| batch(|| dispose(o))))
 }
 
 /// Runs `f`, and the effects its `set`s queue once it returns, each once.
@@ -570,7 +570,7 @@ fn view_of(f: fn() -> View) -> View { untracked(f) }
 /// again each time `when` is set to true, even when it already was.
 pub fn show(when: Signal<Bool>, then: fn() -> View) -> View {
     Region(
-        | | {
+        || {
             if when.value() {
                 [view_of(then)]
             } else {
@@ -585,13 +585,13 @@ pub fn show(when: Signal<Bool>, then: fn() -> View) -> View {
 /// time `items` changes.
 pub fn for_each<T>(items: Signal<[T]>, render: fn(T) -> View) -> View {
     Region(
-        | | {
+        || {
             let xs = items.value()
             let mut out = [Text(""); xs.len()]
             let mut i = 0
             while i < xs.len() {
                 let x = xs[i]
-                out[i] = view_of(| | render(x))
+                out[i] = view_of(|| render(x))
                 i = i + 1
             }
             out
@@ -731,7 +731,7 @@ fn write_attr(a: Attr, out: Vec<String>) {
 pub fn render_to_string(f: fn() -> View) -> String {
     let out = Vec::new()
     batch(
-        | | {
+        || {
             let mut s = ui()
             s.rendering = s.rendering + 1
             root(
@@ -754,13 +754,13 @@ pub fn mount(selector: String, f: fn() -> View) {
         Some(p) => p
         None => panic("std/ui: mount: no element matches \"${selector}\"")
     }
-    let o = new_owner(ROOT, | | {})
+    let o = new_owner(ROOT, || {})
     let mut s = ui()
     let mut kept = s.roots
     kept.push(o)
     let start = s.mounted.len()
     s.building = s.building + 1
-    with_context(Some(o), None, | | build(f(), parent, None))
+    with_context(Some(o), None, || build(f(), parent, None))
     s.building = s.building - 1
     run_mounted(start)
 }
@@ -797,7 +797,7 @@ fn build(v: View, parent: Node, before: Option<Node>) {
         Text(t) => put(parent, backend_text(t), before)
         DynText(f) => {
             let n = backend_text("")
-            effect(| | backend_set_text(n, f()))
+            effect(|| backend_set_text(n, f()))
             put(parent, n, before)
         }
         Region(f) => {
@@ -805,7 +805,7 @@ fn build(v: View, parent: Node, before: Option<Node>) {
             let end = backend_text("")
             put(parent, start, before)
             put(parent, end, before)
-            effect(| | build_region(f, parent, start, end))
+            effect(|| build_region(f, parent, start, end))
         }
     }
 }
@@ -822,7 +822,7 @@ fn build_region(f: fn() -> [View], parent: Node, start: Node, end: Node) {
     let outer = s.building > 0
     let begin = s.mounted.len()
     s.building = s.building + 1
-    let render = new_owner(ROOT, | | {})
+    let render = new_owner(ROOT, || {})
     adopt(render)
     let views = with_context(Some(render), s.listener, f)
     let mut done = false
@@ -835,7 +835,7 @@ fn build_region(f: fn() -> [View], parent: Node, start: Node, end: Node) {
     with_context(
         Some(render),
         None,
-        | | {
+        || {
             let mut i = 0
             while i < views.len() {
                 build(views[i], parent, Some(end))
@@ -850,11 +850,11 @@ fn build_region(f: fn() -> [View], parent: Node, start: Node, end: Node) {
 fn build_attr(a: Attr, el: Node) {
     match a {
         Static(name, value) => backend_set_attr(el, name, value)
-        Dynamic(name, f) => effect(| | backend_set_attr(el, name, f()))
+        Dynamic(name, f) => effect(|| backend_set_attr(el, name, f()))
         Property(name, value) => backend_set_prop(el, name, value)
-        DynProperty(name, f) => effect(| | backend_set_prop(el, name, f()))
+        DynProperty(name, f) => effect(|| backend_set_prop(el, name, f()))
         On(kind, f) => {
-            let stop = backend_listen(el, kind, |v: String| untracked(| | batch(| | f(v))))
+            let stop = backend_listen(el, kind, |v: String| untracked(|| batch(|| f(v))))
             on_cleanup(stop)
         }
     }
diff --git a/tests/dom/timers.nova b/tests/dom/timers.nova
index 2d31b25dd7ba1c03e64fa6e075821c5fe781a346..cdf2a691a384724f80b2b032b2e31e6212e690fe 100644
--- a/tests/dom/timers.nova
+++ b/tests/dom/timers.nova
@@ -5,8 +5,7 @@
 // has run, does nothing, though another timer has its entry by then; and a
 // timeout too long for JS, which must not fire at once. Each late line's
 // timer is set by the line before it, and the harness waits until the page
-// says the last has run, so the order holds however slow the machine. (`| |`
-// is a closure with no parameters: `||` comes with 4.4.)
+// says the last has run, so the order holds however slow the machine.
 
 import dom
 
@@ -15,15 +14,15 @@ record Ticks { n: Int, timer: Option<Timer> }
 fn main() {
     // Over 2^31 - 1 ms, which JS would fire at once; the last timeout
     // clears it.
-    let far = set_timeout(3000000000, | | println("far"))
-    let first = set_timeout(1, | | println("timeout 1"))
-    let never = set_timeout(10, | | println("never"))
+    let far = set_timeout(3000000000, || println("far"))
+    let first = set_timeout(1, || println("timeout 1"))
+    let never = set_timeout(10, || println("never"))
     never.clear()
     let mut ticks = Ticks { n: 0, timer: None }
     ticks.timer = Some(
         set_interval(
             40,
-            | | {
+            || {
                 ticks.n = ticks.n + 1
                 println("tick ${ticks.n}")
                 if ticks.n == 3 {
@@ -33,13 +32,13 @@ fn main() {
                     }
                     let _ = set_timeout(
                         20,
-                        | | {
+                        || {
                             // This timeout takes the entry `first` left when
                             // it ran, so clearing `first` now must release
                             // nothing either.
                             let _ = set_timeout(
                                 1,
-                                | | {
+                                || {
                                     println("timeout after the last")
                                     body().set_attribute("data-done", "yes")
                                 },
diff --git a/tests/dom/ui_attrs.nova b/tests/dom/ui_attrs.nova
index cdc1a94933bfa08a793662faaee2d7bd285755d7..e6f898c7e526d6c8732fa029c1aeb3ec940d13df 100644
--- a/tests/dom/ui_attrs.nova
+++ b/tests/dom/ui_attrs.nova
@@ -28,7 +28,7 @@ fn main() {
     let seen = signal(0)
     mount(
         "#app",
-        | | div(
+        || div(
             [],
             [
                 input(
@@ -42,16 +42,16 @@ fn main() {
                 p(
                     [
                         id("greeting"),
-                        dyn_attr("class", | | if name.value() == "" { "empty" } else { "full" }),
+                        dyn_attr("class", || if name.value() == "" { "empty" } else { "full" }),
                     ],
-                    [dyn_text(| | "hello, ${name.value()}")],
+                    [dyn_text(|| "hello, ${name.value()}")],
                 ),
-                input([id("echo"), dyn_value(| | name.value())]),
+                input([id("echo"), dyn_value(|| name.value())]),
                 button(
                     [
                         id("both"),
                         on_click(
-                            | | {
+                            || {
                                 left.set(1)
                                 right.set(2)
                             },
@@ -60,15 +60,15 @@ fn main() {
                     [text("both")],
                 ),
                 button(
-                    [id("probe"), on_click(| | println("the probe sees ${seen.value()}"))],
+                    [id("probe"), on_click(|| println("the probe sees ${seen.value()}"))],
                     [text("probe")],
                 ),
             ],
         ),
     )
-    effect(| | println("the sum is ${left.value() + right.value()}"))
+    effect(|| println("the sum is ${left.value() + right.value()}"))
     effect(
-        | | {
+        || {
             println("an effect clicks the probe")
             match query_selector("#probe") {
                 Some(e) => e.click()
diff --git a/tests/dom/ui_counter.nova b/tests/dom/ui_counter.nova
index 624d721e2ba87919ab61b25b8b6ea3004e9f70b5..381a37fd5748d0e18ca503f78931e1c03f6b3861 100644
--- a/tests/dom/ui_counter.nova
+++ b/tests/dom/ui_counter.nova
@@ -10,11 +10,11 @@ fn main() {
     let count = signal(0)
     mount(
         "body",
-        | | div(
+        || div(
             [],
             [
-                p([id("count")], [dyn_text(| | "${count.value()}")]),
-                button([id("add"), on_click(| | count.update(|n: Int| n + 1))], [text("add one")]),
+                p([id("count")], [dyn_text(|| "${count.value()}")]),
+                button([id("add"), on_click(|| count.update(|n: Int| n + 1))], [text("add one")]),
             ],
         ),
     )
diff --git a/tests/dom/ui_mount.nova b/tests/dom/ui_mount.nova
index 0c22b8012ed7605a0b88cafaffcd24292df72805..f256d099d4b39fa69417a8d88a866bc04b43eaa8 100644
--- a/tests/dom/ui_mount.nova
+++ b/tests/dom/ui_mount.nova
@@ -15,12 +15,12 @@ import ui::html::{div, dyn_text, id, li, p, text, ul}
 // adds a cleanup that says when the row is gone.
 fn row(r: Int) -> View {
     on_mount(
-        | | {
+        || {
             match query_selector("#row${r}") {
                 Some(e) => println("on_mount of row ${r}: in the page")
                 None => println("on_mount of row ${r}: not in the page")
             }
-            on_cleanup(| | println("row ${r} is gone"))
+            on_cleanup(|| println("row ${r} is gone"))
         },
     )
     li([id("row${r}")], [text("row ${r}")])
@@ -31,20 +31,20 @@ fn main() {
     let phase = signal(0)
     mount(
         "#app",
-        | | {
+        || {
             on_mount(
-                | | {
+                || {
                     match query_selector("#hello") {
                         Some(e) => println("on_mount: #hello is in the page, reading ${e.text()}")
                         None => println("on_mount: #hello is not in the page")
                     }
                 },
             )
-            on_mount(| | println("the second on_mount"))
+            on_mount(|| println("the second on_mount"))
             effect(
-                | | {
+                || {
                     let p = phase.value()
-                    on_mount(| | println("on_mount of the effect's run ${p}"))
+                    on_mount(|| println("on_mount of the effect's run ${p}"))
                 },
             )
             println("building")
@@ -54,7 +54,7 @@ fn main() {
                     p([id("hello")], [text("hi")]),
                     ul([], [for_each(rows, row)]),
                     dyn_text(
-                        | | {
+                        || {
                             rows.set([2])
                             phase.set(1)
                             ""
@@ -66,13 +66,13 @@ fn main() {
     )
     rows.set([3])
     let html = render_to_string(
-        | | {
-            on_cleanup(| | rows.set([4]))
+        || {
+            on_cleanup(|| rows.set([4]))
             text("a string")
         },
     )
     println("rendered ${html}")
-    on_mount(| | println("on_mount outside a mount runs at once"))
-    mount("#nope", | | text("never"))
+    on_mount(|| println("on_mount outside a mount runs at once"))
+    mount("#nope", || text("never"))
     println("not reached")
 }
diff --git a/tests/dom/ui_mount_html.nova b/tests/dom/ui_mount_html.nova
index 56e14ebc92eaffed93eea23559fbfdf1e407cedc..470c155ba6c2b502ca7ed4794a2fe915cc97657c 100644
--- a/tests/dom/ui_mount_html.nova
+++ b/tests/dom/ui_mount_html.nova
@@ -6,7 +6,7 @@ import ui
 import ui::html::{div, p, text}
 
 fn main() {
-    mount("#app", | | p([], [text("fine")]))
-    mount("#app", | | div([], [p([Property("innerHTML", "<img src=x onerror=alert(1)>")], [])]))
+    mount("#app", || p([], [text("fine")]))
+    mount("#app", || div([], [p([Property("innerHTML", "<img src=x onerror=alert(1)>")], [])]))
     println("not reached")
 }
diff --git a/tests/dom/ui_show_for_each.nova b/tests/dom/ui_show_for_each.nova
index 14ddc90413f671867f78e4ae74b9666d1478f724..6aa594736f6560c87511f23b7f9628f573ea6eb2 100644
--- a/tests/dom/ui_show_for_each.nova
+++ b/tests/dom/ui_show_for_each.nova
@@ -25,22 +25,22 @@ fn main() {
     let rows = signal(["row 1", "row 2"])
     mount(
         "#app",
-        | | div(
+        || div(
             [],
             [
-                button([id("toggle"), on_click(| | open.update(|o: Bool| !o))], [text("toggle")]),
+                button([id("toggle"), on_click(|| open.update(|o: Bool| !o))], [text("toggle")]),
                 show(
                     open,
-                    | | {
+                    || {
                         on_mount(
-                            | | {
+                            || {
                                 match query_selector("#panel") {
                                     Some(e) => println("on_mount: the panel is in the page")
                                     None => println("on_mount: the panel is not in the page")
                                 }
                             },
                         )
-                        on_cleanup(| | println("the panel is gone"))
+                        on_cleanup(|| println("the panel is gone"))
                         p([id("panel")], [text("panel")])
                     },
                 ),
@@ -51,23 +51,20 @@ fn main() {
                         for_each(
                             rows,
                             |r: String| li(
-                                [on_click(| | println("clicked ${r}"))],
-                                [dyn_text(| | r)],
+                                [on_click(|| println("clicked ${r}"))],
+                                [dyn_text(|| r)],
                             ),
                         ),
                         li([], [text("last")]),
                     ],
                 ),
-                button([id("more"), on_click(| | rows.update(more))], [text("more")]),
-                button(
-                    [id("again"), on_click(| | rows.update(|xs: [String]| xs))],
-                    [text("again")],
-                ),
+                button([id("more"), on_click(|| rows.update(more))], [text("more")]),
+                button([id("again"), on_click(|| rows.update(|xs: [String]| xs))], [text("again")]),
                 button(
                     [
                         id("none"),
                         on_click(
-                            | | {
+                            || {
                                 let empty: [String] = []
                                 rows.set(empty)
                             },
diff --git a/tests/runtime/ui_cleanup.nova b/tests/runtime/ui_cleanup.nova
index a82d02ea339c88d80f8c572655dbbdde0160236d..988fb38b82678cddb8a27bc46d2cfc104f0ff2db 100644
--- a/tests/runtime/ui_cleanup.nova
+++ b/tests/runtime/ui_cleanup.nova
@@ -12,20 +12,20 @@ fn main() {
     let stop = root(
         |dispose: fn()| {
             effect(
-                | | {
+                || {
                     let v = n.value()
                     println("run ${v}")
-                    on_cleanup(| | println("first cleanup of ${v}"))
-                    on_cleanup(| | println("second cleanup of ${v}"))
+                    on_cleanup(|| println("first cleanup of ${v}"))
+                    on_cleanup(|| println("second cleanup of ${v}"))
                     effect(
-                        | | {
+                        || {
                             println("inner of ${v}")
-                            on_cleanup(| | println("inner cleanup of ${v}"))
+                            on_cleanup(|| println("inner cleanup of ${v}"))
                         },
                     )
                 },
             )
-            on_cleanup(| | println("root cleanup"))
+            on_cleanup(|| println("root cleanup"))
             dispose
         },
     )
@@ -39,9 +39,9 @@ fn main() {
     // what the effect read does not run it again.
     let count = signal(0)
     effect(
-        | | {
+        || {
             println("count is ${count.value()}")
-            on_cleanup(| | count.update(|c: Int| c + 1))
+            on_cleanup(|| count.update(|c: Int| c + 1))
         },
     )
     count.set(10)
@@ -50,21 +50,21 @@ fn main() {
     // again; what it sets elsewhere runs once the dispose is done.
     let s = signal(0)
     let elsewhere = signal(0)
-    effect(| | println("elsewhere sees ${elsewhere.value()}"))
+    effect(|| println("elsewhere sees ${elsewhere.value()}"))
     let stop_again = root(
         |dispose: fn()| {
             effect(
-                | | {
+                || {
                     let v = s.value()
                     println("outer of ${v}")
-                    on_cleanup(| | println("last cleanup of ${v}"))
+                    on_cleanup(|| println("last cleanup of ${v}"))
                     on_cleanup(
-                        | | {
+                        || {
                             s.set(-1)
                             elsewhere.set(1)
                         },
                     )
-                    effect(| | println("inner of ${v} sees ${s.value()}"))
+                    effect(|| println("inner of ${v} sees ${s.value()}"))
                 },
             )
             dispose
@@ -78,10 +78,10 @@ fn main() {
     // queued both.
     let a = signal(1)
     effect(
-        | | {
+        || {
             let v = a.value()
             println("parent of ${v}")
-            effect(| | println("child of ${v} sees ${a.value()}"))
+            effect(|| println("child of ${v} sees ${a.value()}"))
         },
     )
     a.set(2)
@@ -91,16 +91,16 @@ fn main() {
     let open = signal(true)
     let index = signal(0)
     effect(
-        | | {
+        || {
             if open.value() {
-                effect(| | println("the child sees index ${index.value()}"))
+                effect(|| println("the child sees index ${index.value()}"))
             } else {
                 println("the parent hides the child")
             }
         },
     )
     batch(
-        | | {
+        || {
             index.set(5)
             open.set(false)
         },
@@ -112,8 +112,8 @@ fn main() {
     root(
         |dispose: fn()| {
             dispose()
-            effect(| | println("made after the dispose, sees t = ${t.value()}"))
-            on_cleanup(| | println("a cleanup added after the dispose runs at once"))
+            effect(|| println("made after the dispose, sees t = ${t.value()}"))
+            on_cleanup(|| println("a cleanup added after the dispose runs at once"))
         },
     )
     t.set(2)
diff --git a/tests/runtime/ui_collect.nova b/tests/runtime/ui_collect.nova
index 09844cfec40e80636e4eae5f76ea47bc50ec1133..73ad7520b4ac232bddc554c70aa6d049ebbc3bd9 100644
--- a/tests/runtime/ui_collect.nova
+++ b/tests/runtime/ui_collect.nova
@@ -6,10 +6,10 @@ import ui
 async fn main() {
     let log = signal("")
     let n = signal(1)
-    let doubled = memo(| | n.value() * 2)
+    let doubled = memo(|| n.value() * 2)
     let stop = root(
         |dispose: fn()| {
-            effect(| | log.set("n=${n.value()} doubled=${doubled.value()}"))
+            effect(|| log.set("n=${n.value()} doubled=${doubled.value()}"))
             dispose
         },
     )
diff --git a/tests/runtime/ui_loop.nova b/tests/runtime/ui_loop.nova
index fa58ddb7db0a61472f122f1388c07df8587b7aa5..bf799469d003e5455dd788198bbf2ceb5be69f6f 100644
--- a/tests/runtime/ui_loop.nova
+++ b/tests/runtime/ui_loop.nova
@@ -4,6 +4,6 @@ import ui
 
 fn main() {
     let n = signal(0)
-    effect(| | n.set(n.value() + 1))
+    effect(|| n.set(n.value() + 1))
     println("not reached")
 }
diff --git a/tests/runtime/ui_memo.nova b/tests/runtime/ui_memo.nova
index 09a6d58e9bd2abb0380810d1684aaf3cd84263c6..96048d4d623f2d1b7bfbf0e60c81f345caf25f70 100644
--- a/tests/runtime/ui_memo.nova
+++ b/tests/runtime/ui_memo.nova
@@ -8,21 +8,21 @@ fn main() {
     let base = signal(2)
     let computed = signal(0)
     let squared = memo(
-        | | {
+        || {
             computed.update(|n: Int| n + 1)
             base.value() * base.value()
         },
     )
     println("made: computed ${computed.value()}")
-    effect(| | println("reader one: ${squared.value()}"))
-    effect(| | println("reader two: ${squared.value()}"))
+    effect(|| println("reader one: ${squared.value()}"))
+    effect(|| println("reader two: ${squared.value()}"))
     println("two readers: computed ${computed.value()}")
     base.set(3)
     println("after set: computed ${computed.value()}")
 
     // Nothing reads `lazy`, so a change does not compute it.
     let lazy = memo(
-        | | {
+        || {
             println("lazy computes")
             base.value() + 100
         },
@@ -37,10 +37,10 @@ fn main() {
     let x = signal(0)
     let input = signal(1)
     let parent = memo(
-        | | {
+        || {
             let v = input.value()
-            effect(| | println("the memo's child of ${v} sees x=${x.value()}"))
-            effect(| | on_cleanup(| | x.update(|n: Int| n + 1)))
+            effect(|| println("the memo's child of ${v} sees x=${x.value()}"))
+            effect(|| on_cleanup(|| x.update(|n: Int| n + 1)))
             v
         },
     )
@@ -53,11 +53,11 @@ fn main() {
     let itself = signal(none)
     let n = signal(1)
     let tens = memo(
-        | | {
+        || {
             let v = n.value()
             println("tens computes from ${v}")
             on_cleanup(
-                | | {
+                || {
                     match itself.value() {
                         Some(t) => println("the cleanup of ${v} reads ${t.value()}")
                         None => {}
@@ -78,11 +78,11 @@ fn main() {
     let stop = root(
         |dispose: fn()| {
             let hundreds = memo(
-                | | {
+                || {
                     let v = m_in.value()
                     println("hundreds computes from ${v}")
                     on_cleanup(
-                        | | {
+                        || {
                             match held.value() {
                                 Some(h) => println("the disposed memo reads ${h.value()}")
                                 None => {}
@@ -97,7 +97,7 @@ fn main() {
         },
     )
     batch(
-        | | {
+        || {
             m_in.set(2)
             stop()
         },
diff --git a/tests/runtime/ui_memo_set.nova b/tests/runtime/ui_memo_set.nova
index ff268dd257106493c315b203dfc79a1576cea61b..94ee98e8b80730f98c3e753a7a5f6e03ea457273 100644
--- a/tests/runtime/ui_memo_set.nova
+++ b/tests/runtime/ui_memo_set.nova
@@ -3,7 +3,7 @@ import ui
 
 fn main() {
     let a = signal(1)
-    let doubled = memo(| | a.value() * 2)
+    let doubled = memo(|| a.value() * 2)
     println("doubled ${doubled.value()}")
     doubled.set(10)
     println("not reached")
diff --git a/tests/runtime/ui_render.nova b/tests/runtime/ui_render.nova
index a119a477f4faeba87cc8ca8c4a594f8c84b355fe..765875a1b7a6f9dc28a838deadd6e8cd57330c06 100644
--- a/tests/runtime/ui_render.nova
+++ b/tests/runtime/ui_render.nova
@@ -29,7 +29,7 @@ fn main() {
     // Nested elements, attributes, and a void element.
     println(
         render_to_string(
-            | | div(
+            || div(
                 [class("card"), attr("data-id", "7")],
                 [
                     h1([], [text("Title")]),
@@ -42,20 +42,18 @@ fn main() {
 
     // Escaping: text escapes `&`, `<` and `>`; attributes `"` too.
     println(
-        render_to_string(
-            | | p([attr("title", "say \"hi\" & <go>")], [text("1 < 2 & 3 > 2 \"q\"")]),
-        ),
+        render_to_string(|| p([attr("title", "say \"hi\" & <go>")], [text("1 < 2 & 3 > 2 \"q\"")])),
     )
 
     // Dynamic text and attributes are read once; handlers are left out;
     // a property is written as the attribute.
     let n = signal(3)
     let html = render_to_string(
-        | | div(
-            [dyn_attr("class", | | "n${n.value()}")],
+        || div(
+            [dyn_attr("class", || "n${n.value()}")],
             [
-                dyn_text(| | "n is ${n.value()}"),
-                button([on_click(| | n.set(0))], [text("reset")]),
+                dyn_text(|| "n is ${n.value()}"),
+                button([on_click(|| n.set(0))], [text("reset")]),
                 input([value("x\"y"), attr("type", "text")]),
             ],
         ),
@@ -64,7 +62,7 @@ fn main() {
 
     // `show`, both ways.
     let open = signal(true)
-    let panel = | | div([], [show(open, | | p([], [text("open")])), text("|")])
+    let panel = || div([], [show(open, || p([], [text("open")])), text("|")])
     println(render_to_string(panel))
     open.set(false)
     println(render_to_string(panel))
@@ -73,7 +71,7 @@ fn main() {
     let items = signal(
         [Item { name: "milk", done: true }, Item { name: "eggs & ham", done: false }],
     )
-    let list = | | ul(
+    let list = || ul(
         [],
         [
             for_each(
@@ -88,14 +86,14 @@ fn main() {
     println(render_to_string(list))
 
     // `element` writes a tag with no builder of its own.
-    println(render_to_string(| | element("main", [], [text("body")])))
+    println(render_to_string(|| element("main", [], [text("body")])))
 
     // A cleanup in the render runs when it is disposed; `on_mount` never.
     println(
         render_to_string(
-            | | {
-                on_cleanup(| | println("cleanup ran"))
-                on_mount(| | println("on_mount ran"))
+            || {
+                on_cleanup(|| println("cleanup ran"))
+                on_mount(|| println("on_mount ran"))
                 text("done")
             },
         ),
diff --git a/tests/runtime/ui_render_bad_name.nova b/tests/runtime/ui_render_bad_name.nova
index b1628d497cc9fd10cbd485ea8ae05f6dc5a05384..93913ee8fdefa85ae2f9a865cc32d295add2760d 100644
--- a/tests/runtime/ui_render_bad_name.nova
+++ b/tests/runtime/ui_render_bad_name.nova
@@ -4,7 +4,7 @@ import ui
 import ui::html::{element, text}
 
 fn main() {
-    println(render_to_string(| | element("my-tag", [], [text("fine")])))
-    println(render_to_string(| | element("p onclick=x", [], [])))
+    println(render_to_string(|| element("my-tag", [], [text("fine")])))
+    println(render_to_string(|| element("p onclick=x", [], [])))
     println("not reached")
 }
diff --git a/tests/runtime/ui_signal.nova b/tests/runtime/ui_signal.nova
index d3f51d22a843ccc1d75a8267d59374cb4566ff00..67789fdb3b9f5f14dabd630a50628a1e4dc293af 100644
--- a/tests/runtime/ui_signal.nova
+++ b/tests/runtime/ui_signal.nova
@@ -9,7 +9,7 @@ fn main() {
     let count = signal(1)
     let runs = signal(0)
     effect(
-        | | {
+        || {
             println("count is ${count.value()}")
             runs.update(|n: Int| n + 1)
         },
@@ -24,7 +24,7 @@ fn main() {
     let a = signal("a1")
     let b = signal("b1")
     effect(
-        | | {
+        || {
             if flag.value() {
                 println("branch a: ${a.value()}")
             } else {
@@ -40,9 +40,9 @@ fn main() {
     // A batch runs the effect once, after it returns.
     let x = signal(0)
     let y = signal(0)
-    effect(| | println("sum: ${x.value() + y.value()}"))
+    effect(|| println("sum: ${x.value() + y.value()}"))
     batch(
-        | | {
+        || {
             x.set(1)
             y.set(2)
             println("inside the batch")
@@ -51,18 +51,18 @@ fn main() {
 
     // A diamond: `d` reads `b` and `c`, which both read `top`.
     let top = signal(1)
-    let left = memo(| | top.value() + 1)
-    let right = memo(| | top.value() * 2)
-    effect(| | println("diamond: ${left.value()} ${right.value()}"))
+    let left = memo(|| top.value() + 1)
+    let right = memo(|| top.value() * 2)
+    effect(|| println("diamond: ${left.value()} ${right.value()}"))
     top.set(5)
     top.set(6)
 
     // An effect's first run is a batch: what it sets runs once, after it.
     let p = signal(0)
     let q = signal(0)
-    effect(| | println("p + q: ${p.value() + q.value()}"))
+    effect(|| println("p + q: ${p.value() + q.value()}"))
     effect(
-        | | {
+        || {
             p.set(1)
             q.set(2)
             println("the first run is done")
@@ -73,10 +73,10 @@ fn main() {
     // reading the signal leaves as it was.
     let o = signal(0)
     let reading = signal(true)
-    effect(| | println("first sees ${o.value()}"))
-    effect(| | { if reading.value() { println("the dropped one sees ${o.value()}") } })
-    effect(| | println("second sees ${o.value()}"))
-    effect(| | println("third sees ${o.value()}"))
+    effect(|| println("first sees ${o.value()}"))
+    effect(|| { if reading.value() { println("the dropped one sees ${o.value()}") } })
+    effect(|| println("second sees ${o.value()}"))
+    effect(|| println("third sees ${o.value()}"))
     reading.set(false)
     o.set(1)
 }
````


### Task 2: The desugaring

Spec §4; decisions 4 to 9 and 11.

**Files:**
- Create: `crates/nova-typeck/src/check/view.rs`
- Modify: `crates/nova-typeck/src/check.rs`
- Test:
  - `tests/runtime/ui_view.nova`, `ui_view.stdout`
  - `crates/nova-cli/tests/run_tests.rs`, `wasm_tests.rs`
  - `crates/nova-driver/tests/index_complete.rs`

- [ ] **Step 1: The tests.** `patch_block t2-tests`:
  - `tests/runtime/ui_view.nova`, on both targets through
    `render_to_string` (spec §9.2): a component with parameters; nested
    elements, a void element, `type:`, `for:` and `aria_label:`; fixed and
    reactive text and attributes, a reactive one reading a signal set
    before the render; `value:`; an `on_click` the render leaves out;
    `show` and `for_each` with `|| view { … }`; and `||` outside a view,
    beside `||` as the logical or. It joins `run_tests.rs`'s `std/ui`
    programs, as is and under stress, and the WASM gate.
  - `index_complete.rs` counts 150 programs and 174 files, and skips
    `component` before a name and `(`, `view` before `{`, and an element's
    named argument (decision 11).
- [ ] **Step 2: Run, and see them fail.**
  - `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests ui_programs_run`.
    Expected: fails at `ui_view`, which does not compile.
    Its output holds "`view { … }` blocks are not supported yet" (E0900).
  - `W gate::ui_view`. Expected: `gate::ui_view` fails.
  - `X`. Expected: `every_name_in_the_corpora_is_indexed` fails.
- [ ] **Step 3: The code.** `patch_block t2-code`:
  - **`check/view.rs`:** `Ui`, `std/ui`'s `View` and `Attr` and
    `std/ui/html`'s `on` and `text`, found by module name, as every
    target compiles them; `component_ret`; `check_view` (one root,
    E0094); `view_node`, `view_text`, `element` (E0095),
    `element_builder` (E0001, E0092, E0096), `view_attr` and
    `view_handler` (E0097); `reader`, `|| value` through `check_closure`;
    `is_str_lit` (decision 7).
  - **`check.rs`:** `mod view`; `Expr::View` through `check_view`; a
    component's return type `component_ret()`. Its tests:
    `a_view_desugars_to_std_uis_calls`,
    `a_views_parts_are_fixed_or_read_again_by_literalness`, which reads
    the HIR (a plain and a raw string fixed, an interpolated one read
    again), and one test for each of E0094 to E0097 (`on_` alone among
    them), and for E0010 with its message.
- [ ] **Step 4: Run.**
  - `T`. Expected: `410 passed`: seven more.
  - `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests ui_programs_run`.
    Expected: `1 passed`.
  - `W gate::`. Expected: `104 passed`.
  - `X`. Expected: `1 passed`.
- [ ] **Step 5: The checks,** as the conventions list them, with `cargo run
  --locked -q -p nova-cli -- fmt --check tests/runtime/ui_*.nova`.
- [ ] **Step 6: Commit** `typeck: a view desugars into std/ui's calls; a component returns View`.

**Patch `t2-tests`:**

<!-- patch t2-tests -->

````diff
diff --git a/crates/nova-cli/tests/run_tests.rs b/crates/nova-cli/tests/run_tests.rs
index 4bcb25abea1f8c30cdaad579ee41f79e849d6097..6dd04301dc7241149b0898fb9a9d330ae6057d10 100644
--- a/crates/nova-cli/tests/run_tests.rs
+++ b/crates/nova-cli/tests/run_tests.rs
@@ -11319,6 +11319,7 @@ fn ui_programs_run_and_survive_gc_stress() {
         "ui_memo",
         "ui_render",
         "ui_collect",
+        "ui_view",
     ] {
         let source = repo_root().join(format!("tests/runtime/{name}.nova"));
         let expected = std::fs::read_to_string(source.with_extension("stdout"))
diff --git a/crates/nova-cli/tests/wasm_tests.rs b/crates/nova-cli/tests/wasm_tests.rs
index e887edfec1e0a4b28c3aa230502e262b18d496c3..d445bcd3feb0245ce54a759b07f1a0aff38723ee 100644
--- a/crates/nova-cli/tests/wasm_tests.rs
+++ b/crates/nova-cli/tests/wasm_tests.rs
@@ -1543,6 +1543,8 @@ mod gate {
         ui_memo,
         ui_render,
         ui_signal,
+        // `component` and `view { … }` (spec 4.4 §9.2).
+        ui_view,
     );
 
     /// The programs a test of their own in this module checks, by its name.
diff --git a/crates/nova-driver/tests/index_complete.rs b/crates/nova-driver/tests/index_complete.rs
index edac8c162386bbfce8b789c825760cd9246c6525..7dec05d85c9a2328e7e569d3808e6343feabb6b2 100644
--- a/crates/nova-driver/tests/index_complete.rs
+++ b/crates/nova-driver/tests/index_complete.rs
@@ -133,10 +133,65 @@ fn skipped(tokens: &[Spanned<Token>], i: usize) -> bool {
     if matches!(prev, Some(Token::Module)) {
         return true;
     }
+    // `component` and `view` where they are keywords (the 4.4 spec §6):
+    // before a name and `(`, and before `{`.
+    let next = |k: usize| tokens.get(i + k).map(|t| &t.value);
+    if name == "component"
+        && matches!(next(1), Some(Token::Ident(_)))
+        && matches!(next(2), Some(Token::LParen))
+    {
+        return true;
+    }
+    if name == "view" && matches!(next(1), Some(Token::LBrace)) {
+        return true;
+    }
+    // A named argument's name in an element, `div(class: "a")`: the
+    // attribute it names is no symbol.
+    if in_element_arguments(tokens, i) {
+        return true;
+    }
     // An attribute's arguments, `@test(should_panic)`.
     in_attribute_arguments(tokens, i)
 }
 
+/// Whether the identifier at `i` names an element's argument: it is
+/// followed by `:`, starts an argument, and the innermost open bracket is
+/// the `(` of a call, not of a `fn` or a `component`'s parameters.
+fn in_element_arguments(tokens: &[Spanned<Token>], i: usize) -> bool {
+    if !matches!(tokens.get(i + 1).map(|t| &t.value), Some(Token::Colon)) {
+        return false;
+    }
+    if !matches!(
+        i.checked_sub(1).map(|j| &tokens[j].value),
+        Some(Token::LParen | Token::Comma)
+    ) {
+        return false;
+    }
+    let mut depth = 0usize;
+    let mut j = i;
+    while j > 0 {
+        j -= 1;
+        match &tokens[j].value {
+            Token::RParen | Token::RBracket | Token::RBrace => depth += 1,
+            Token::LParen if depth == 0 => {
+                let callee = j.checked_sub(1).map(|k| &tokens[k].value);
+                let before = j.checked_sub(2).map(|k| &tokens[k].value);
+                return matches!(callee, Some(Token::Ident(_)))
+                    && !matches!(before, Some(Token::Fn))
+                    && !matches!(before, Some(Token::Ident(w)) if w == "component");
+            }
+            Token::LParen | Token::LBracket | Token::LBrace => {
+                if depth == 0 {
+                    return false;
+                }
+                depth -= 1;
+            }
+            _ => {}
+        }
+    }
+    false
+}
+
 fn in_attribute_arguments(tokens: &[Spanned<Token>], i: usize) -> bool {
     let mut j = i;
     while j > 0 {
@@ -186,9 +241,9 @@ fn every_name_is_covered(a: &Analysis, file: FileId, failures: &mut Vec<String>)
 #[test]
 fn every_name_in_the_corpora_is_indexed() {
     let programs = programs();
-    // 7 examples, 141 top-level runtime programs and the modules program,
-    // on 2026-10-17.
-    assert_eq!(programs.len(), 149, "{programs:#?}");
+    // 7 examples, 142 top-level runtime programs and the modules program,
+    // on 2026-10-17, after 4.4 added `ui_view`.
+    assert_eq!(programs.len(), 150, "{programs:#?}");
     let mut files = 0;
     let mut failures: Vec<String> = Vec::new();
     for (k, path) in programs.iter().enumerate() {
@@ -215,9 +270,9 @@ fn every_name_in_the_corpora_is_indexed() {
             every_name_is_covered(&a, file, &mut failures);
         }
     }
-    // 7 + 141 + 3 files of user code, 05-geometry's library's 2, and
+    // 7 + 142 + 3 files of user code, 05-geometry's library's 2, and
     // native std's 20.
-    assert_eq!(files, 173);
+    assert_eq!(files, 174);
     failures.sort();
     failures.dedup();
     assert!(
diff --git a/tests/runtime/ui_view.nova b/tests/runtime/ui_view.nova
new file mode 100644
index 0000000000000000000000000000000000000000..96159a134d8b6a87742fdcf3c6d941f196c4a4ff
--- /dev/null
+++ b/tests/runtime/ui_view.nova
@@ -0,0 +1,71 @@
+// `component` and `view { … }` (RFC 0001; spec
+// docs/superpowers/specs/2026-10-17-phase-4-4-component-syntax-design.md
+// §9.2), through `render_to_string` on both targets: a component with
+// parameters, nested and void elements, attributes by name, fixed and
+// reactive text and attributes, `value`, a handler the render leaves out,
+// `show` and `for_each` with `|| view { … }`, and `||` outside a view.
+import ui
+import ui::html::{button, class, div, input, label, li, p, span, ul}
+
+record Item { name: String, done: Bool }
+
+/// A card titled `title`, which shows `n`.
+component Card(title: String, n: Signal<Int>) {
+    view {
+        div(class: "card", aria_label: title) {
+            p { text("fixed") }
+            p(class: "n${n.value()}") { text("n is ${n.value()}") }
+            label(for: "box") { text(title) }
+            input(type: "checkbox", id: "box", value: "${n.value()}")
+            button(on_click: || n.set(0), class("reset")) { text("reset") }
+        }
+    }
+}
+
+component Panel(open: Signal<Bool>) {
+    view {
+        div {
+            show(
+                open,
+                || view {
+                    span { text("open") }
+                },
+            )
+            text("|")
+        }
+    }
+}
+
+component List(items: Signal<[Item]>) {
+    view {
+        ul {
+            for_each(
+                items,
+                |it: Item| view {
+                    li(class: if it.done { "done" } else { "todo" }) { text(it.name) }
+                },
+            )
+        }
+    }
+}
+
+fn main() {
+    // A reactive value reads the signal as the render finds it.
+    let n = signal(1)
+    n.set(7)
+    println(render_to_string(|| Card("A & B", n)))
+
+    let open = signal(true)
+    println(render_to_string(|| Panel(open)))
+    open.set(false)
+    println(render_to_string(|| Panel(open)))
+
+    let items = signal([Item { name: "milk", done: true }, Item { name: "eggs", done: false }])
+    println(render_to_string(|| List(items)))
+
+    // `||` is a closure with no parameters outside a view too, and `||`
+    // between two values is still the logical or.
+    let f = || 40 + 2
+    let either = false || true
+    println("${f()} ${either}")
+}
diff --git a/tests/runtime/ui_view.stdout b/tests/runtime/ui_view.stdout
new file mode 100644
index 0000000000000000000000000000000000000000..3fdff94e5c41435af8dadc16978db8b0b8ec7fd3
--- /dev/null
+++ b/tests/runtime/ui_view.stdout
@@ -0,0 +1,5 @@
+<div class="card" aria-label="A &amp; B"><p>fixed</p><p class="n7">n is 7</p><label for="box">A &amp; B</label><input type="checkbox" id="box" value="7"><button class="reset">reset</button></div>
+<div><span>open</span>|</div>
+<div>|</div>
+<ul><li class="done">milk</li><li class="todo">eggs</li></ul>
+42 true
````

**Patch `t2-code`:**

<!-- patch t2-code -->

````diff
diff --git a/crates/nova-typeck/src/check.rs b/crates/nova-typeck/src/check.rs
index a14c7af6ced942e989b273fdab36d4cb89bd5be2..d800dbb82bb55b1bcdb696ded3eee547d55c3e76 100644
--- a/crates/nova-typeck/src/check.rs
+++ b/crates/nova-typeck/src/check.rs
@@ -20,6 +20,7 @@ use crate::{
 };
 
 mod fixes;
+mod view;
 
 use fixes::{Binding, Need};
 
@@ -2131,11 +2132,14 @@ impl<'a> Checker<'a> {
                 .iter()
                 .map(|p| self.convert_ty(&p.ty, &generics, &bounds))
                 .collect();
-            let ret = f
-                .return_ty
-                .as_ref()
-                .map(|t| self.convert_ty(t, &generics, &bounds))
-                .unwrap_or(Ty::Unit);
+            let ret = if f.is_component {
+                self.component_ret()
+            } else {
+                f.return_ty
+                    .as_ref()
+                    .map(|t| self.convert_ty(t, &generics, &bounds))
+                    .unwrap_or(Ty::Unit)
+            };
             // An async fn's declared return type is its future's OUTPUT, per
             // the spec's own signatures (`pub async fn join(self) -> T`), so
             // the signature every caller sees is `Future<ret>`. `check_fn_body`
@@ -3576,7 +3580,7 @@ impl<'a> Checker<'a> {
                 }
             }
             ast::Expr::Cast { .. } => self.unsupported_expr(span, "`as` casts"),
-            ast::Expr::View(_) => self.unsupported_expr(span, "`view { … }` blocks"),
+            ast::Expr::View(children) => self.check_view(fcx, children, span),
         }
     }
 
@@ -17429,4 +17433,223 @@ mod tests {
             vec![("a".to_string(), Ty::String), ("n".to_string(), Ty::Int)]
         );
     }
+
+    /// The codes `src` raises, after `import ui` and two element builders
+    /// (the 4.4 spec §4).
+    fn view_codes(src: &str) -> Vec<String> {
+        let src = format!("import ui\nimport ui::html::{{div, input}}\n{src}\nfn main() {{ }}\n");
+        check_src(&src)
+            .diagnostics
+            .iter()
+            .map(|d| d.code.clone())
+            .collect()
+    }
+
+    /// A component returns `View`, and a view's elements, text, named
+    /// arguments and handlers of both kinds check (§4.1 to §4.4).
+    #[test]
+    fn a_view_desugars_to_std_uis_calls() {
+        let codes = view_codes(
+            "fn h() { }\n\
+             fn typed(v: String) { }\n\
+             component Card(title: String, n: Signal<Int>) {\n\
+             \x20   view {\n\
+             \x20       div(class: \"c\", aria_label: title, on_click: h, on_input: typed) {\n\
+             \x20           text(\"fixed\")\n\
+             \x20           text(\"n ${n.value()}\")\n\
+             \x20           input(type: \"text\", value: title, on_change: |v: String| println(v))\n\
+             \x20           div { }\n\
+             \x20       }\n\
+             \x20   }\n\
+             }\n\
+             fn use_it() { let v: View = Card(\"t\", signal(1)) }",
+        );
+        assert!(codes.is_empty(), "{codes:?}");
+    }
+
+    /// What `component`'s body desugars to, as far as a view goes: each
+    /// call by its function's name and its arguments, each variant by its
+    /// name and its fields, each string literal, and `_` for anything else,
+    /// such as a closure.
+    fn view_parts(src: &str, component: &str) -> String {
+        fn walk(m: &hir::Module, e: &hir::Expr, out: &mut Vec<String>) {
+            match &e.kind {
+                hir::ExprKind::Block {
+                    trailing: Some(t), ..
+                } => walk(m, t, out),
+                hir::ExprKind::Call {
+                    func: hir::Callee::Def(d),
+                    args,
+                    ..
+                } => {
+                    let f = m.functions.iter().find(|f| f.def_id == *d);
+                    out.push(format!("{}(", f.expect("a function").name));
+                    for a in args {
+                        walk(m, a, out);
+                    }
+                    out.push(")".to_string());
+                }
+                hir::ExprKind::MakeArray { elems } | hir::ExprKind::StrConcat(elems) => {
+                    for a in elems {
+                        walk(m, a, out);
+                    }
+                }
+                hir::ExprKind::MakeVariant { sum, variant, args } => {
+                    let s = m.sums.iter().find(|s| s.def_id == *sum).expect("a sum");
+                    out.push(s.variants[*variant as usize].name.clone());
+                    for a in args {
+                        walk(m, a, out);
+                    }
+                }
+                hir::ExprKind::StrLit(s) => out.push(format!("{s:?}")),
+                _ => out.push("_".to_string()),
+            }
+        }
+        let src = format!("import ui\nimport ui::html::{{div, input}}\n{src}\nfn main() {{ }}\n");
+        let r = check_src(&src);
+        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
+        let f = r.module.functions.iter().find(|f| f.name == component);
+        let mut out = Vec::new();
+        walk(&r.module, &f.expect("the component").body, &mut out);
+        out.join(" ")
+    }
+
+    /// The second table of §4.3, and §4.4: a literal is fixed and anything
+    /// else read again; `_` becomes `-`; `value` is the property; `on_kind`
+    /// is `on` for a `fn()` and `On` for a `fn(String)`; `text` is syntax,
+    /// and a one-argument call of another name is a call.
+    #[test]
+    fn a_views_parts_are_fixed_or_read_again_by_literalness() {
+        let parts = view_parts(
+            "fn h() { }\n\
+             fn one(s: String) -> View { div([], []) }\n\
+             component Card(x: String) {\n\
+             \x20   view {\n\
+             \x20       div(class: \"a\", id: r\"r\", aria_label: x, on_click: h, on_input: |v: String| println(v)) {\n\
+             \x20           text(\"t\")\n\
+             \x20           text(x)\n\
+             \x20           input(value: \"v\")\n\
+             \x20           input(value: x, title: \"a ${x}\")\n\
+             \x20           one(\"y\")\n\
+             \x20       }\n\
+             \x20   }\n\
+             }",
+            "Card",
+        );
+        assert_eq!(
+            parts,
+            "div( Static \"class\" \"a\" Static \"id\" \"r\" Dynamic \"aria-label\" _ on( \"click\" _ ) \
+             On \"input\" _ Text \"t\" DynText _ input( Property \"value\" \"v\" ) \
+             input( DynProperty \"value\" _ Dynamic \"title\" _ ) one( \"y\" ) )"
+        );
+    }
+
+    /// `view` holds one root (§4.2).
+    #[test]
+    fn a_view_with_no_root_or_two_is_e0094() {
+        assert_eq!(view_codes("component A() { view { } }"), ["E0094"]);
+        assert_eq!(
+            view_codes("component A() { view { div { } div { } } }"),
+            ["E0094"]
+        );
+    }
+
+    /// A void element takes no block (§4.3).
+    #[test]
+    fn a_void_element_with_children_is_e0095() {
+        assert_eq!(
+            view_codes("component A() { view { input(type: \"x\") { } } }"),
+            ["E0095"]
+        );
+    }
+
+    /// An element names a builder: a function of `([Attr], [View])` or
+    /// `([Attr])` that returns `View`, not another function, a local or a
+    /// variant (§4.3).
+    #[test]
+    fn an_element_that_is_no_builder_is_e0096() {
+        assert_eq!(
+            view_codes("fn f(x: Int) -> View { div([], []) }\ncomponent A() { view { f { } } }"),
+            ["E0096"]
+        );
+        assert_eq!(
+            view_codes("fn A() -> View { let div = 1\n view { div { } } }"),
+            ["E0096"]
+        );
+        assert_eq!(
+            view_codes(
+                "fn two(a: Int, b: Int) -> View { div([], []) }\ncomponent A() { view { two(x: \"a\") { } } }"
+            ),
+            ["E0096"]
+        );
+        assert_eq!(
+            view_codes(
+                "fn b(a: [Attr], c: [View]) -> Int { 1 }\ncomponent A() { view { b(x: \"a\") { } } }"
+            ),
+            ["E0096"]
+        );
+        assert_eq!(
+            view_codes("component A() { view { Some(x: \"a\") } }"),
+            ["E0096"]
+        );
+        assert_eq!(
+            view_codes("component A() { view { nope(x: \"a\") } }"),
+            ["E0001"]
+        );
+    }
+
+    /// An attribute's value is a `String`, and a handler a `fn()` or a
+    /// `fn(String)` (§4.3).
+    #[test]
+    fn an_attribute_or_handler_of_another_type_is_e0097() {
+        assert_eq!(
+            view_codes("component A() { view { div(class: 5) } }"),
+            ["E0097"]
+        );
+        assert_eq!(
+            view_codes("component A() { view { div(value: 1.5) } }"),
+            ["E0097"]
+        );
+        assert_eq!(
+            view_codes("component A() { view { div(on_click: 5) } }"),
+            ["E0097"]
+        );
+        assert_eq!(
+            view_codes("component A() { view { div(on_click: || 5) } }"),
+            ["E0097"]
+        );
+        assert_eq!(
+            view_codes("fn h() { }\ncomponent A() { view { div(on_: h) } }"),
+            ["E0097"]
+        );
+        assert_eq!(
+            view_codes("component A() { view { div(on_click: |n: Int| println(\"x\")) } }"),
+            ["E0097"]
+        );
+    }
+
+    /// A child is a `View`, and a view's text a `String`.
+    #[test]
+    fn a_child_or_text_of_another_type_is_e0010() {
+        assert_eq!(
+            view_codes("component A() { view { div { 5 } } }"),
+            ["E0010"]
+        );
+        let r = check_src(
+            "import ui\nimport ui::html::{div}\n\
+             component A() { view { div(class: 5) { text(5) } } }\nfn main() { }\n",
+        );
+        let found: Vec<(&str, &str)> = r
+            .diagnostics
+            .iter()
+            .map(|d| (d.code.as_str(), d.message.as_str()))
+            .collect();
+        assert_eq!(
+            found,
+            [
+                ("E0097", "`class` takes a `String`, found `Int`"),
+                ("E0010", "text must have type `String`, found `Int`"),
+            ]
+        );
+    }
 }
diff --git a/crates/nova-typeck/src/check/view.rs b/crates/nova-typeck/src/check/view.rs
new file mode 100644
index 0000000000000000000000000000000000000000..54e5b4f8a8826d440eaba7d193ae0f60a5e3a004
--- /dev/null
+++ b/crates/nova-typeck/src/check/view.rs
@@ -0,0 +1,395 @@
+//! `view { … }` and `component` (RFC 0001; the 4.4 spec
+//! `docs/superpowers/specs/2026-10-17-phase-4-4-component-syntax-design.md`
+//! §4). A view's elements, text and attributes become the calls and
+//! variants of `std/ui`'s API here, as string interpolation becomes calls:
+//! no pass rewrites the tree before this one.
+
+use nova_ast as ast;
+use nova_ast::{ViewArg, ViewNode};
+use nova_diagnostics::{Span, Spanned};
+use nova_hir as hir;
+use nova_hir::Ty;
+use nova_resolver::{DefId, DefKind, ModuleId, Res, Target};
+
+use super::{error_expr, Checker, FnCtx};
+
+/// What a view desugars to: `std/ui`'s two sums, and the two builders of
+/// `std/ui/html` it calls or shows.
+struct Ui {
+    view: DefId,
+    attr: DefId,
+    /// `on(kind, f)`, which adapts a `fn()` handler to `On`'s `fn(String)`.
+    on: DefId,
+    /// `text`, which a view's `text(…)` is shown as on hover.
+    text: DefId,
+}
+
+impl<'a> Checker<'a> {
+    /// `std/ui`'s names, which every target compiles (spec 4.3 §7.3), so a
+    /// view needs no import of its own.
+    fn ui(&self) -> Option<Ui> {
+        let module = |name: &str| {
+            (0..self.defs.module_count() as u32)
+                .map(ModuleId)
+                .find(|&m| self.defs.module_name(m) == name)
+        };
+        let ui = module("$std.ui")?;
+        let html = module("$std.ui.html")?;
+        let function = |name: &str| match self.defs.resolve_value(html, name) {
+            Some(Res::Def(id)) => Some(id),
+            _ => None,
+        };
+        Some(Ui {
+            view: self.defs.resolve_type(ui, "View")?,
+            attr: self.defs.resolve_type(ui, "Attr")?,
+            on: function("on")?,
+            text: function("text")?,
+        })
+    }
+
+    /// A component's return type: `std/ui`'s `View` (§4.1).
+    pub(super) fn component_ret(&self) -> Ty {
+        match self.ui() {
+            Some(ui) => sum(ui.view),
+            None => Ty::Error,
+        }
+    }
+
+    /// `view { child }`: the `View` its one child desugars to (§4.2).
+    pub(super) fn check_view(
+        &mut self,
+        fcx: &mut FnCtx,
+        children: &[Spanned<ViewNode>],
+        span: Span,
+    ) -> hir::Expr {
+        let Some(ui) = self.ui() else {
+            return error_expr(span);
+        };
+        if children.len() != 1 {
+            self.error(
+                "E0094",
+                format!("`view` holds one root, found {}", children.len()),
+                span,
+            );
+            for c in children {
+                self.view_node(fcx, &ui, c);
+            }
+            return error_expr(span);
+        }
+        self.view_node(fcx, &ui, &children[0])
+    }
+
+    /// A child: an element, a view's `text(…)`, or an expression of type
+    /// `View`.
+    fn view_node(&mut self, fcx: &mut FnCtx, ui: &Ui, node: &Spanned<ViewNode>) -> hir::Expr {
+        match &node.value {
+            ViewNode::Element {
+                name,
+                args,
+                children,
+            } => self.element(fcx, ui, name, args, children.as_deref(), node.span),
+            ViewNode::Expr(e) => {
+                if let Some((word, content)) = text_child(e) {
+                    return self.view_text(fcx, ui, word, content, e.span);
+                }
+                let x = self.check_expr(fcx, e);
+                self.expect_ty(fcx, &x, &sum(ui.view), "a view's child");
+                x
+            }
+        }
+    }
+
+    /// `text(e)`: `Text(e)` for a string literal, `DynText(|| e)` for
+    /// anything else (§4.4).
+    fn view_text(
+        &mut self,
+        fcx: &mut FnCtx,
+        ui: &Ui,
+        word: &Spanned<String>,
+        content: &Spanned<ast::Expr>,
+        span: Span,
+    ) -> hir::Expr {
+        self.note_use(word.span, Target::Def(ui.text));
+        if is_str_lit(content) {
+            let v = self.check_expr(fcx, content);
+            return self.ui_variant(fcx, ui.view, "Text", vec![v], span);
+        }
+        let (f, ty) = self.reader(fcx, content);
+        if !fcx.icx.unify(&ty, &Ty::String) {
+            self.error(
+                "E0010",
+                format!(
+                    "text must have type `String`, found `{}`",
+                    self.show(&ty, fcx)
+                ),
+                content.span,
+            );
+            return error_expr(span);
+        }
+        self.ui_variant(fcx, ui.view, "DynText", vec![f], span)
+    }
+
+    /// An element: a call of its builder with its attributes and its
+    /// children (§4.3).
+    fn element(
+        &mut self,
+        fcx: &mut FnCtx,
+        ui: &Ui,
+        name: &Spanned<String>,
+        args: &[ViewArg],
+        children: Option<&[Spanned<ViewNode>]>,
+        span: Span,
+    ) -> hir::Expr {
+        let builder = self.element_builder(fcx, ui, name);
+        let attrs: Vec<hir::Expr> = args.iter().map(|a| self.view_attr(fcx, ui, a)).collect();
+        let kids: Vec<hir::Expr> = children
+            .unwrap_or(&[])
+            .iter()
+            .map(|c| self.view_node(fcx, ui, c))
+            .collect();
+        let Some((def_id, takes_children)) = builder else {
+            return error_expr(span);
+        };
+        if !takes_children && children.is_some() {
+            self.error(
+                "E0095",
+                format!("`{}` takes no children", name.value),
+                name.span,
+            );
+            return error_expr(span);
+        }
+        let mut call_args = vec![array(attrs, sum(ui.attr), span)];
+        if takes_children {
+            call_args.push(array(kids, sum(ui.view), span));
+        }
+        hir::Expr {
+            kind: hir::ExprKind::Call {
+                func: hir::Callee::Def(def_id),
+                type_args: Vec::new(),
+                args: call_args,
+            },
+            ty: sum(ui.view),
+            span,
+        }
+    }
+
+    /// The function an element names, and whether it takes children: one
+    /// that takes `([Attr], [View])`, or `([Attr])`, and returns `View`.
+    fn element_builder(
+        &mut self,
+        fcx: &mut FnCtx,
+        ui: &Ui,
+        name: &Spanned<String>,
+    ) -> Option<(DefId, bool)> {
+        let n = name.value.as_str();
+        let not_builder = |this: &mut Self| {
+            this.error(
+                "E0096",
+                format!(
+                    "`{n}` is not an element builder: one takes `([Attr], [View])` or \
+                     `([Attr])` and returns `View`"
+                ),
+                name.span,
+            );
+            None
+        };
+        if fcx.lookup(n).is_some() {
+            return not_builder(self);
+        }
+        let def_id = match self.defs.resolve_value(self.cur_module, n) {
+            Some(Res::Def(id)) if matches!(self.defs.def(id).kind, DefKind::Fn { .. }) => id,
+            None => {
+                if !self.missing_on_target(n, name.span) {
+                    self.error(
+                        "E0001",
+                        format!("cannot find function `{n}` in this scope"),
+                        name.span,
+                    );
+                }
+                return None;
+            }
+            Some(_) => return not_builder(self),
+        };
+        self.note_use(name.span, Target::Def(def_id));
+        let sig = self.sigs.get(&def_id)?.clone();
+        let attrs = Ty::Array(Box::new(sum(ui.attr)));
+        let views = Ty::Array(Box::new(sum(ui.view)));
+        if sig.generics != 0 || sig.ret != sum(ui.view) {
+            return not_builder(self);
+        }
+        if sig.params == [attrs.clone(), views] {
+            Some((def_id, true))
+        } else if sig.params == [attrs] {
+            Some((def_id, false))
+        } else {
+            not_builder(self)
+        }
+    }
+
+    /// An element's argument: an `Attr` as it is, or a named one made
+    /// into one (§4.3's second table).
+    fn view_attr(&mut self, fcx: &mut FnCtx, ui: &Ui, arg: &ViewArg) -> hir::Expr {
+        let (name, value) = match arg {
+            ViewArg::Positional(e) => {
+                let x = self.check_expr(fcx, e);
+                self.expect_ty(fcx, &x, &sum(ui.attr), "an element's attribute");
+                return x;
+            }
+            ViewArg::Named { name, value } => (name, value),
+        };
+        let span = name.span.merge(value.span);
+        if let Some(kind) = name.value.strip_prefix("on_") {
+            if kind.is_empty() {
+                self.error("E0097", "`on_` names no event", name.span);
+                return error_expr(span);
+            }
+            return self.view_handler(fcx, ui, name, kind, value, span);
+        }
+        let key = str_lit(name.value.replace('_', "-"), name.span);
+        let property = name.value == "value";
+        if is_str_lit(value) {
+            let v = self.check_expr(fcx, value);
+            let variant = if property { "Property" } else { "Static" };
+            return self.ui_variant(fcx, ui.attr, variant, vec![key, v], span);
+        }
+        let (f, ty) = self.reader(fcx, value);
+        if !fcx.icx.unify(&ty, &Ty::String) {
+            self.error(
+                "E0097",
+                format!(
+                    "`{}` takes a `String`, found `{}`",
+                    name.value,
+                    self.show(&ty, fcx)
+                ),
+                value.span,
+            );
+            return error_expr(span);
+        }
+        let variant = if property { "DynProperty" } else { "Dynamic" };
+        self.ui_variant(fcx, ui.attr, variant, vec![key, f], span)
+    }
+
+    /// `on_kind: h`: `On("kind", h)` for a `fn(String)`, and `std/ui/html`'s
+    /// `on("kind", h)` for a `fn()`.
+    fn view_handler(
+        &mut self,
+        fcx: &mut FnCtx,
+        ui: &Ui,
+        name: &Spanned<String>,
+        kind: &str,
+        value: &Spanned<ast::Expr>,
+        span: Span,
+    ) -> hir::Expr {
+        let kind = str_lit(kind.to_string(), name.span);
+        let h = self.check_expr(fcx, value);
+        match fcx.icx.apply(&h.ty) {
+            Ty::Fn { params, ret } if params.is_empty() && fcx.icx.unify(&ret, &Ty::Unit) => {
+                hir::Expr {
+                    kind: hir::ExprKind::Call {
+                        func: hir::Callee::Def(ui.on),
+                        type_args: Vec::new(),
+                        args: vec![kind, h],
+                    },
+                    ty: sum(ui.attr),
+                    span,
+                }
+            }
+            Ty::Fn { params, ret }
+                if params.len() == 1
+                    && fcx.icx.unify(&params[0], &Ty::String)
+                    && fcx.icx.unify(&ret, &Ty::Unit) =>
+            {
+                self.ui_variant(fcx, ui.attr, "On", vec![kind, h], span)
+            }
+            Ty::Error => error_expr(span),
+            other => {
+                self.error(
+                    "E0097",
+                    format!(
+                        "`{}` takes a handler, `fn()` or `fn(String)`, found `{}`",
+                        name.value,
+                        self.show(&other, fcx)
+                    ),
+                    value.span,
+                );
+                error_expr(span)
+            }
+        }
+    }
+
+    /// `|| value`, which reads `value` again each time it runs, and the type
+    /// `value` has.
+    fn reader(&mut self, fcx: &mut FnCtx, value: &Spanned<ast::Expr>) -> (hir::Expr, Ty) {
+        let f = self.check_closure(fcx, &[], &None, value, value.span);
+        let ty = match &f.ty {
+            Ty::Fn { ret, .. } => (**ret).clone(),
+            _ => Ty::Error,
+        };
+        (f, ty)
+    }
+
+    /// The variant `name` of `std/ui`'s sum `sum_id`, made of `args`.
+    fn ui_variant(
+        &mut self,
+        fcx: &mut FnCtx,
+        sum_id: DefId,
+        name: &str,
+        args: Vec<hir::Expr>,
+        span: Span,
+    ) -> hir::Expr {
+        match self.variant_index(sum_id, name) {
+            Some(vi) => self.make_variant(fcx, sum_id, vi, args, span),
+            None => error_expr(span),
+        }
+    }
+}
+
+/// A child `text(e)`: the bare name `text` and one argument (§4.4).
+fn text_child(e: &Spanned<ast::Expr>) -> Option<(&Spanned<String>, &Spanned<ast::Expr>)> {
+    let ast::Expr::Call { callee, args } = &e.value else {
+        return None;
+    };
+    let ast::Expr::Path(path) = &callee.value else {
+        return None;
+    };
+    match (path.segments.as_slice(), args.as_slice()) {
+        ([word], [content]) if word.value == "text" => Some((word, content)),
+        _ => None,
+    }
+}
+
+/// A string literal with no interpolation, which a view keeps fixed: a raw
+/// string, `r"…"`, or a `StringInterp` of literal parts alone, as the
+/// parser makes every other string.
+fn is_str_lit(e: &Spanned<ast::Expr>) -> bool {
+    match &e.value {
+        ast::Expr::Lit(ast::Literal::Str(_)) => true,
+        ast::Expr::StringInterp(parts) => {
+            parts.iter().all(|p| matches!(p, ast::StringPart::Lit(_)))
+        }
+        _ => false,
+    }
+}
+
+fn sum(def_id: DefId) -> Ty {
+    Ty::Sum {
+        def_id,
+        args: Vec::new(),
+    }
+}
+
+fn str_lit(s: String, span: Span) -> hir::Expr {
+    hir::Expr {
+        kind: hir::ExprKind::StrLit(s),
+        ty: Ty::String,
+        span,
+    }
+}
+
+fn array(elems: Vec<hir::Expr>, elem: Ty, span: Span) -> hir::Expr {
+    hir::Expr {
+        kind: hir::ExprKind::MakeArray { elems },
+        ty: Ty::Array(Box::new(elem)),
+        span,
+    }
+}
````


### Task 3: Editor support, and components in `nova doc`

Spec §6, §10; decisions 10 and 12.

**Files:**
- Modify:
  - `crates/nova-lsp/src/tokens.rs`
  - `tools/vscode-nova/syntaxes/nova.tmLanguage.json`
  - `crates/nova-diagnostics/src/lines.rs`
  - `crates/nova-doc/src/signature.rs`
- Test:
  - `crates/nova-cli/tests/vscode_extension.rs`
  - `crates/nova-lsp/src/hover.rs` (a test alone)

- [ ] **Step 1: The tests.** `patch_block t3-tests`:
  - `the_grammar_colours_component_and_view_where_they_are_keywords`: the
    grammar's `contextual-keywords` rules, between `function-names` and
    `keywords`, and `KEYWORDS` without either word;
  - `an_element_hovers_as_its_builder`: `div` in a view hovers as
    `std/ui/html`'s `pub fn div(…) -> View`, a component as it is
    written with its doc, and a named argument not at all.
- [ ] **Step 2: Run, and see them fail.**
  - `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test vscode_extension`.
    Expected: `the_grammar_colours_component_and_view_where_they_are_keywords`
    fails, the rest pass.
  - `CARGO_INCREMENTAL=0 cargo test -p nova-lsp --lib hover`. Expected:
    passes already: Task 2's index makes the element a use of its builder,
    and this test pins what hover shows of it.
- [ ] **Step 3: The code.** `patch_block t3-code`:
  - **`nova-lsp`:** the legend's thirteenth type, `keyword`; `keywords`
    finds the two words by lexing and asking the index (decision 10). Its
    test, `component_and_view_are_keywords_only_where_they_are`, counts
    one token at each name a variable holds.
  - **The grammar:** `contextual-keywords`, two rules.
  - **`nova-diagnostics`:** `keyword_start` knows `component`.
  - **`nova-doc`:** `render` colours `component` before the item's name;
    test `a_component_is_shown_as_it_is_written`.
- [ ] **Step 4: Run.**
  - `CARGO_INCREMENTAL=0 cargo test -p nova-lsp --lib`. Expected: `26 passed`.
  - `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test vscode_extension`.
    Expected: `3 passed`.
  - `CARGO_INCREMENTAL=0 cargo test -p nova-diagnostics --lib`, then `-p
    nova-doc --lib`. Expected: `22 passed`, then `61 passed`.
- [ ] **Step 5: The checks,** as the conventions list them.
- [ ] **Step 6: Commit** `lsp, vscode, doc: component and view where they are keywords`.

**Patch `t3-tests`:**

<!-- patch t3-tests -->

````diff
diff --git a/crates/nova-cli/tests/vscode_extension.rs b/crates/nova-cli/tests/vscode_extension.rs
index 41fdc59c1f0c8f3f29f31f3d4291ac431b5cf456..db2f7c9302e406930ebac7ce4a31040b9157625f 100644
--- a/crates/nova-cli/tests/vscode_extension.rs
+++ b/crates/nova-cli/tests/vscode_extension.rs
@@ -37,6 +37,42 @@ fn the_grammar_colours_every_keyword_and_nothing_else() {
     assert_eq!(listed, keywords);
 }
 
+/// `component` and `view` are coloured as keywords only before a name and
+/// `(`, and before `{`: neither is reserved, so `KEYWORDS` lacks both, and
+/// the rule comes before the one for keywords (the 4.4 spec §6).
+#[test]
+fn the_grammar_colours_component_and_view_where_they_are_keywords() {
+    let grammar = json("syntaxes/nova.tmLanguage.json");
+    let includes: Vec<&str> = grammar["patterns"]
+        .as_array()
+        .expect("patterns")
+        .iter()
+        .filter_map(|p| p["include"].as_str())
+        .collect();
+    let at = |name: &str| includes.iter().position(|i| *i == name).unwrap();
+    assert!(at("#function-names") < at("#contextual-keywords"));
+    assert!(at("#contextual-keywords") < at("#keywords"));
+    let rules = grammar["repository"]["contextual-keywords"]["patterns"]
+        .as_array()
+        .expect("contextual keyword rules");
+    assert_eq!(
+        rules[0]["match"],
+        "\\b(component)\\s+([A-Za-z_][A-Za-z0-9_]*)\\s*(?=\\()"
+    );
+    assert_eq!(
+        rules[0]["captures"]["1"]["name"],
+        "keyword.other.component.nova"
+    );
+    assert_eq!(
+        rules[0]["captures"]["2"]["name"],
+        "entity.name.function.nova"
+    );
+    assert_eq!(rules[1]["match"], "\\bview(?=\\s*\\{)");
+    assert_eq!(rules[1]["name"], "keyword.other.view.nova");
+    assert!(!nova_lexer::KEYWORDS.contains(&"component"));
+    assert!(!nova_lexer::KEYWORDS.contains(&"view"));
+}
+
 #[test]
 fn the_extension_has_nova_clis_version_and_its_floor() {
     let package = json("package.json");
diff --git a/crates/nova-lsp/src/hover.rs b/crates/nova-lsp/src/hover.rs
index 5a18ae25d0c8472ab4b791f7aa9dd3c438390643..a99fcbd71251287d38d96c6330da839edf0ddefa 100644
--- a/crates/nova-lsp/src/hover.rs
+++ b/crates/nova-lsp/src/hover.rs
@@ -239,6 +239,37 @@ mod tests {
         );
     }
 
+    /// An element's name hovers as its builder, and a component as it is
+    /// written (the 4.4 spec §6, §9.2).
+    #[test]
+    fn an_element_hovers_as_its_builder() {
+        let text = "import ui\nimport ui::html::{div}\n\n/// A card.\ncomponent Card(title: String) {\n    view {\n        div(class: title) { text(\"hi\") }\n    }\n}\n\nfn main() {\n    println(render_to_string(|| Card(\"t\")))\n}\n";
+        let path = std::env::temp_dir()
+            .join("nova-lsp-hover-element")
+            .join("main.nova");
+        let overlay = crate::workspace::Overlay::default().with(&path, text.to_string());
+        let options = crate::analysis::options(None, true);
+        let answer = crate::analysis::answering(&path, &overlay, &options).expect("an answer");
+        let shown = |at: usize| {
+            let lsp::HoverContents::Markup(found) =
+                hover(&answer, at as u32).expect("a hover").contents
+            else {
+                panic!("not Markdown");
+            };
+            found.value
+        };
+        let div = shown(text.find("div(class").unwrap());
+        assert!(
+            div.starts_with("```nova\npub fn div(attrs: [Attr], children: [View]) -> View\n```"),
+            "{div}"
+        );
+        assert_eq!(
+            shown(text.find("Card(\"t\")").unwrap()),
+            "```nova\ncomponent Card(title: String)\n```\n\nA card."
+        );
+        assert!(hover(&answer, text.find("class:").unwrap() as u32).is_none());
+    }
+
     #[test]
     fn an_item_line_keeps_pub_async_and_where_and_stops_at_its_body() {
         let text = "    pub async fn f<T>(x: T) -> T\n    where T: Show\n    { x }\n";
````

**Patch `t3-code`:**

<!-- patch t3-code -->

````diff
diff --git a/crates/nova-diagnostics/src/lines.rs b/crates/nova-diagnostics/src/lines.rs
index 429dece2e60d98c4602c824266dbc63d5c50dc54..1ab42544e6661b4d35a45a7dddc566067501b8b4 100644
--- a/crates/nova-diagnostics/src/lines.rs
+++ b/crates/nova-diagnostics/src/lines.rs
@@ -130,7 +130,10 @@ pub fn keyword_start(text: &str, name_start: usize) -> Option<usize> {
     let end = back_over_space(name_start);
     let start = back_over_word(end);
     let keyword = &text[start..end];
-    if !matches!(keyword, "fn" | "record" | "type" | "trait" | "const") {
+    if !matches!(
+        keyword,
+        "fn" | "record" | "type" | "trait" | "const" | "component"
+    ) {
         return None;
     }
     if keyword == "fn" {
@@ -229,6 +232,10 @@ mod tests {
             text.find("fn f")
         );
         assert_eq!(keyword_start(text, text.find("x =").unwrap()), None);
+        // A component's keyword, which only an item's place makes one (the
+        // 4.4 spec §3.1).
+        let text = "component Card() {}\n";
+        assert_eq!(keyword_start(text, text.find("Card").unwrap()), Some(0));
     }
 
     #[test]
diff --git a/crates/nova-doc/src/signature.rs b/crates/nova-doc/src/signature.rs
index 7f7ded75f35d8138f6cb425440e969509b13771d..68bc93e9ac8f7be1c661c9c79084fc0e3c5ce34b 100644
--- a/crates/nova-doc/src/signature.rs
+++ b/crates/nova-doc/src/signature.rs
@@ -146,6 +146,11 @@ pub(crate) fn render(
         let word = &text[s..e];
         match &t.value {
             Token::Ident(_) if own == Some(t.span) => span(&mut out, "name", word),
+            // Before the item's own name, a name can only be `component`,
+            // a keyword there (the 4.4 spec §3.1).
+            Token::Ident(w) if w == "component" && own.is_some_and(|o| e <= o.start as usize) => {
+                span(&mut out, "kw", word)
+            }
             Token::Ident(_) => match link(t.span) {
                 Some(url) => {
                     out.push_str("<a href=\"");
@@ -251,6 +256,23 @@ mod tests {
         );
     }
 
+    #[test]
+    fn a_component_is_shown_as_it_is_written() {
+        let text = "/// Doc.\npub component Card(title: String) { view { div { } } }\n";
+        let file = parse(text);
+        let Item::Function(f) = &file.items[0].value else {
+            panic!("not a function")
+        };
+        let ft = FileText::new(text, FileId::DUMMY);
+        let start = decl_start(text, f.name.span.start as usize);
+        let end = fn_end(&ft, f.name.span, f.return_ty.as_ref(), &f.where_clause);
+        assert_eq!(
+            render(&ft, start, end, Some(f.name.span), &|_| None),
+            "<span class=\"kw\">pub</span> <span class=\"kw\">component</span> \
+             <span class=\"name\">Card</span>(title: String)"
+        );
+    }
+
     #[test]
     fn a_broken_signature_keeps_its_lines_less_their_indentation() {
         // Review Focus 1.
diff --git a/crates/nova-lsp/src/tokens.rs b/crates/nova-lsp/src/tokens.rs
index 4c7cab8478865f32efbb502f3eb8a6d65b76938e..f9a07fc6874f4aa47253e663fe3afeb378acc562 100644
--- a/crates/nova-lsp/src/tokens.rs
+++ b/crates/nova-lsp/src/tokens.rs
@@ -1,16 +1,18 @@
 //! Semantic tokens: every name the index records, by what it means (spec
 //! `docs/superpowers/specs/2026-10-10-phase-3-4b-fixes-and-colour-design.md`
-//! §7).
+//! §7), and `component` and `view` where they are keywords (the 4.4 spec
+//! §6).
 
 use lsp_types as lsp;
-use nova_diagnostics::LineIndex;
+use nova_diagnostics::{FileId, LineIndex, Span};
 use nova_driver::Analysis;
+use nova_lexer::Token;
 use nova_resolver::{DefId, DefKind, Definitions, Index, Occurrence, Role, Target};
 
 use crate::analysis::Answer;
 
 /// The token types, in the legend's order (spec §7.1).
-const TYPES: [lsp::SemanticTokenType; 12] = [
+const TYPES: [lsp::SemanticTokenType; 13] = [
     lsp::SemanticTokenType::NAMESPACE,
     lsp::SemanticTokenType::TYPE,
     lsp::SemanticTokenType::STRUCT,
@@ -23,6 +25,7 @@ const TYPES: [lsp::SemanticTokenType; 12] = [
     lsp::SemanticTokenType::ENUM_MEMBER,
     lsp::SemanticTokenType::FUNCTION,
     lsp::SemanticTokenType::METHOD,
+    lsp::SemanticTokenType::KEYWORD,
 ];
 
 const NAMESPACE: u32 = 0;
@@ -37,6 +40,7 @@ const PROPERTY: u32 = 8;
 const ENUM_MEMBER: u32 = 9;
 const FUNCTION: u32 = 10;
 const METHOD: u32 = 11;
+const KEYWORD: u32 = 12;
 
 /// The modifiers' bits, in the legend's order (spec §7.1).
 const DECLARATION: u32 = 1;
@@ -93,9 +97,49 @@ pub fn tokens(answer: &Answer) -> Vec<lsp::SemanticToken> {
         let (ty, mods) = classify(a, defs, index, o);
         spans.push((o.span.start, o.span.end, ty, mods));
     }
+    for (start, end) in keywords(text, answer.file, defs, index) {
+        spans.push((start, end, KEYWORD, 0));
+    }
+    spans.sort_by_key(|s| s.0);
     encode(text, &spans)
 }
 
+/// Where `component` and `view` are keywords in `text` (the 4.4 spec §6),
+/// which the extension's grammar cannot tell: a `component` before the
+/// name a function declares and its `(`, and a `view` before `{` that is
+/// no name the index knows, as it is in an `if` head.
+fn keywords(text: &str, file: FileId, defs: &Definitions, index: &Index) -> Vec<(u32, u32)> {
+    let named = |span: Span| index.occurrences.iter().any(|o| o.span == span);
+    let declares_fn = |span: Span| {
+        index.occurrences.iter().any(|o| {
+            o.span == span
+                && o.role == Role::Declaration
+                && matches!(o.target, Target::Def(id) if matches!(defs.def(id).kind, DefKind::Fn { .. }))
+        })
+    };
+    let (tokens, _) = nova_lexer::lex(text, file);
+    let mut out = Vec::new();
+    for (i, t) in tokens.iter().enumerate() {
+        let Token::Ident(word) = &t.value else {
+            continue;
+        };
+        let next = |k: usize| tokens.get(i + k).map(|n| &n.value);
+        let keyword = match word.as_str() {
+            "component" => {
+                matches!(next(1), Some(Token::Ident(_)))
+                    && matches!(next(2), Some(Token::LParen))
+                    && declares_fn(tokens[i + 1].span)
+            }
+            "view" => matches!(next(1), Some(Token::LBrace)) && !named(t.span),
+            _ => false,
+        };
+        if keyword {
+            out.push((t.span.start, t.span.end));
+        }
+    }
+    out
+}
+
 /// A name's token type and modifiers (spec §7.2).
 fn classify(a: &Analysis, defs: &Definitions, index: &Index, o: &Occurrence) -> (u32, u32) {
     let std = |id: DefId| {
@@ -220,19 +264,24 @@ mod tests {
             .collect()
     }
 
-    /// The token at the `n`th whole `word` of `text`: its type and modifiers.
+    /// Where the `n`th whole `word` of `text` starts.
     #[track_caller]
-    fn token(found: &Decoded, text: &str, word: &str, n: usize) -> (String, Vec<String>) {
+    fn token_offset(text: &str, word: &str, n: usize) -> usize {
         let ident = |c: Option<char>| c.is_some_and(|c| c.is_alphanumeric() || c == '_');
-        let start = text
-            .match_indices(word)
+        text.match_indices(word)
             .map(|(i, _)| i)
             .filter(|&i| {
                 !ident(text[..i].chars().next_back())
                     && !ident(text[i + word.len()..].chars().next())
             })
             .nth(n)
-            .unwrap_or_else(|| panic!("no `{word}` number {n}"));
+            .unwrap_or_else(|| panic!("no `{word}` number {n}"))
+    }
+
+    /// The token at the `n`th whole `word` of `text`: its type and modifiers.
+    #[track_caller]
+    fn token(found: &Decoded, text: &str, word: &str, n: usize) -> (String, Vec<String>) {
+        let start = token_offset(text, word, n);
         found
             .iter()
             .find(|t| t.2 == start)
@@ -308,6 +357,47 @@ mod tests {
         assert!(found.iter().all(|t| t.2 != at_self), "{found:?}");
     }
 
+    /// `component` and `view` are keywords where the parser read them as
+    /// such, and names elsewhere; an element's name is its builder, a
+    /// function of std (the 4.4 spec §6).
+    #[test]
+    fn component_and_view_are_keywords_only_where_they_are() {
+        let text = "import ui\nimport ui::html::{div}\n\ncomponent Card() {\n    view {\n        div(class: \"c\") { text(\"hi\") }\n    }\n}\n\nfn main() {\n    let view = true\n    if view {\n        println(render_to_string(Card))\n    }\n    let component = 1\n    let c = component\n    println(\"${c}\")\n}\n";
+        let a = answer("keywords", &[("main.nova", text)]);
+        let found = decoded(text, &tokens(&a));
+        assert_eq!(token(&found, text, "component", 0), of("keyword", &[]));
+        assert_eq!(token(&found, text, "view", 0), of("keyword", &[]));
+        assert_eq!(
+            token(&found, text, "div", 1),
+            of("function", &["defaultLibrary"])
+        );
+        assert_eq!(
+            token(&found, text, "text", 0),
+            of("function", &["defaultLibrary"])
+        );
+        assert_eq!(
+            token(&found, text, "view", 1),
+            of("variable", &["declaration"])
+        );
+        assert_eq!(token(&found, text, "view", 2), of("variable", &[]));
+        assert_eq!(
+            token(&found, text, "component", 1),
+            of("variable", &["declaration"])
+        );
+        // A name before a call, as line breaks are skipped: `component`
+        // then `println(`, which declares nothing.
+        assert_eq!(token(&found, text, "component", 2), of("variable", &[]));
+        // One token per name: the variable's, and no keyword's beside it.
+        for (word, n) in [("view", 2), ("component", 2)] {
+            let at = token_offset(text, word, n);
+            assert_eq!(found.iter().filter(|t| t.2 == at).count(), 1, "{word}");
+        }
+        assert!(
+            found.iter().all(|t| t.2 != text.find("class").unwrap()),
+            "{found:?}"
+        );
+    }
+
     #[test]
     fn one_token_per_span_the_value_wins() {
         let text = "record P { x: Int }\n\nfn main() {\n    let x = 1\n    let p = P { x }\n}\n";
diff --git a/tools/vscode-nova/syntaxes/nova.tmLanguage.json b/tools/vscode-nova/syntaxes/nova.tmLanguage.json
index 66cdca55f85829f5e13a344ec840cd237fb560e6..52cfd1d931a503fbaaaa7e8bf72f2df6ea39fce2 100644
--- a/tools/vscode-nova/syntaxes/nova.tmLanguage.json
+++ b/tools/vscode-nova/syntaxes/nova.tmLanguage.json
@@ -10,6 +10,7 @@
     { "include": "#numbers" },
     { "include": "#attributes" },
     { "include": "#function-names" },
+    { "include": "#contextual-keywords" },
     { "include": "#keywords" },
     { "include": "#types" }
   ],
@@ -52,6 +53,18 @@
         "2": { "name": "entity.name.function.nova" }
       }
     },
+    "contextual-keywords": {
+      "patterns": [
+        {
+          "match": "\\b(component)\\s+([A-Za-z_][A-Za-z0-9_]*)\\s*(?=\\()",
+          "captures": {
+            "1": { "name": "keyword.other.component.nova" },
+            "2": { "name": "entity.name.function.nova" }
+          }
+        },
+        { "name": "keyword.other.view.nova", "match": "\\bview(?=\\s*\\{)" }
+      ]
+    },
     "keywords": {
       "name": "keyword.other.nova",
       "match": "\\b(let|mut|const|fn|return|if|else|while|for|in|break|continue|match|type|record|trait|impl|import|module|pub|async|await|extern|unsafe|true|false|as|is|where|with|self|Self)\\b"
````


### Task 4: The bundle

Spec §7; decisions 13 to 17.

**Files:**
- Create: `crates/nova-cli/src/bundle.rs`
- Modify:
  - `crates/nova-cli/src/cmd/run.rs`, `crates/nova-cli/src/main.rs`
  - `crates/nova-codegen-wasm/src/shim/index.html`
- Test: `crates/nova-cli/tests/wasm_tests.rs`

- [ ] **Step 1: The tests.** `patch_block t4-tests`:
  - `nova_at(dir, home)`, which `nova_in` now calls, for a run with its
    own `PATH` or a `NOVA_HOME` outside the directory it runs in;
  - `a_single_file_gets_the_generated_page_and_a_manifest`;
    `a_projects_own_page_and_public_go_into_dist`, whose `public/` holds a
    nested file and, on Unix, a link to itself, one to a file inside it
    and one to a file outside it, and whose `dist/` holds, on Unix, links
    at `index.html`, `manifest.json` and `app.js` to the project's page and
    a secret, both unchanged after the build; whose manifest is the same
    after a second build and loses a file removed from `public/`;
    `dist_may_not_hold_the_project_or_be_inside_public`, which tries `.`,
    `..`, `nope/..`, `src/..`, on Unix a link to the parent, and
    `public/out`, keeps the project's page and its own `assets/`, and whose
    project sits one level down so `..` is the test's own directory;
    `a_public_that_leads_outside_is_not_copied`, on Unix;
    `release_runs_wasm_opt_when_it_is_on_path`, with `PATH` an empty
    directory, then a stand-in built from `WASM_OPT` first on it, then
    one that exits 3;
  - `release_does_not_apply_to_wasm` goes: `--release` now builds.
- [ ] **Step 2: Run, and see them fail.** `W manifest`, `W dist`, `W
  release`, `W leads_outside`. Expected: `a_single_file_gets_the_generated_page_and_a_manifest`
  fails; `a_projects_own_page_and_public_go_into_dist` fails, its page the
  generated one, and `dist_may_not_hold_the_project_or_be_inside_public`
  fails, 2 passed; `release_runs_wasm_opt_when_it_is_on_path` fails, 5
  passed; and `a_public_that_leads_outside_is_not_copied` fails.
- [ ] **Step 3: The code.** `patch_block t4-code`:
  - **`bundle.rs`:** `Bundle::of`, `check_output`, `prepare` (links at
    the build's names removed), `finish` (the page, never onto itself,
    `assets/` from a `public/` inside the project, `wasm-opt`,
    `manifest.json`), `manifest`, `quote`, `wasm_opt`, `copy_tree` (links
    inside `public/` alone), `list`, `remove`, `resolved` and `by_text`
    (decisions 15 and 16). Its tests: `quote_escapes_what_json_needs` and
    `the_page_is_never_copied_onto_itself`.
  - **`cmd/run.rs`:** `build_wasm` no longer refuses `--release`; it makes
    the `Bundle`, checks the output directory and prepares it before the
    compiler writes anything, and finishes the bundle after; `--release`'s
    help names `wasm-opt`.
  - **The generated page** gains `<div id="app"></div>`.
- [ ] **Step 4: Run.**
  - `W manifest`, `W dist`, `W release`, `W leads_outside`, `W
    build_for_wasm_writes_dist`, `W bundle_is`. Expected: `1 passed`, `4
    passed`, `6 passed`, `1 passed`, `1 passed` and `1 passed`.
  - `CARGO_INCREMENTAL=0 cargo test -p nova-cli --bin nova bundle`.
    Expected: `2 passed`.
- [ ] **Step 5: The checks,** as the conventions list them.
- [ ] **Step 6: Commit** `cli: the bundle: a project's page, assets/, manifest.json, and wasm-opt under --release`.

**Patch `t4-tests`:**

<!-- patch t4-tests -->

````diff
diff --git a/crates/nova-cli/tests/wasm_tests.rs b/crates/nova-cli/tests/wasm_tests.rs
index d445bcd3feb0245ce54a759b07f1a0aff38723ee..0d44cfa07fba85e6a1b7fe36e37c9e76a6c8ddab 100644
--- a/crates/nova-cli/tests/wasm_tests.rs
+++ b/crates/nova-cli/tests/wasm_tests.rs
@@ -34,14 +34,19 @@ fn nova_in(dir: &Path, args: &[&str]) -> Assert {
         "{}-home",
         dir.file_name().expect("a name").to_string_lossy()
     ));
-    let _ = std::fs::remove_dir_all(&home);
-    nova()
-        .current_dir(dir)
-        .env("NOVA_HOME", &home)
+    nova_at(dir, &home).args(args).assert()
+}
+
+/// `nova` in `dir`, with a fresh `NOVA_HOME` at `home`, no colour and no
+/// backtrace, to be given its arguments.
+fn nova_at(dir: &Path, home: &Path) -> Command {
+    let _ = std::fs::remove_dir_all(home);
+    let mut cmd = nova();
+    cmd.current_dir(dir)
+        .env("NOVA_HOME", home)
         .env("NO_COLOR", "1")
-        .env("RUST_BACKTRACE", "0")
-        .args(args)
-        .assert()
+        .env("RUST_BACKTRACE", "0");
+    cmd
 }
 
 fn stdout(a: &Assert) -> String {
@@ -557,21 +562,339 @@ fn a_project_builds_into_dist_at_its_root() {
     assert!(dir.join("dist").join("app.wasm").is_file());
 }
 
-/// `--release` names the LLVM backend, which WASM does not use.
+/// A program built from a single file gets the generated page, which has
+/// `#app`, no `assets/`, and a manifest named after the file (spec 4.4 §7).
+#[test]
+fn a_single_file_gets_the_generated_page_and_a_manifest() {
+    let dir = fresh("bundle-file");
+    write(&dir, &[("hello.nova", "fn main() { println(\"hi\") }\n")]);
+    nova_in(&dir, &["build", "--target", "wasm", "hello.nova"]).success();
+    let dist = dir.join("dist");
+    let page = std::fs::read_to_string(dist.join("index.html")).expect("index.html");
+    assert!(page.contains("<div id=\"app\"></div>"), "{page}");
+    assert!(!dist.join("assets").exists());
+    let manifest = std::fs::read_to_string(dist.join("manifest.json")).expect("manifest.json");
+    assert!(
+        manifest.starts_with(
+            "{\n  \"name\": \"hello\",\n  \"version\": null,\n  \"release\": false,\n  \"wasm_opt\": false,\n"
+        ),
+        "{manifest}"
+    );
+    assert_eq!(
+        manifest_files(&dist),
+        ["app.js", "app.wasm", "index.html", "package.json"]
+    );
+}
+
+/// The files `dist/manifest.json` lists, each checked against its size.
+fn manifest_files(dist: &Path) -> Vec<String> {
+    let text = std::fs::read_to_string(dist.join("manifest.json")).expect("manifest.json");
+    let manifest: serde_json::Value = serde_json::from_str(&text).expect("JSON");
+    let files = manifest["files"].as_object().expect("files");
+    for (path, size) in files {
+        let on_disk = std::fs::metadata(dist.join(path))
+            .expect("a listed file")
+            .len();
+        assert_eq!(size.as_u64(), Some(on_disk), "{path}");
+    }
+    files.keys().cloned().collect()
+}
+
+/// A project's own `index.html` is copied, and its `public/` becomes
+/// `assets/`, nested files included; on Unix, a link to a file inside
+/// `public/` is copied as the file, and a link to a directory, or to a file
+/// outside `public/`, is skipped. The manifest lists them all, and is the
+/// same after a second build; a file removed from `public/` is gone from
+/// `assets/` after the next build (spec 4.4 §7, §9.3).
 #[test]
-fn release_does_not_apply_to_wasm() {
+fn a_projects_own_page_and_public_go_into_dist() {
+    let dir = fresh("bundle-project");
+    let page = "<!doctype html>\n<title>mine</title>\n<div id=\"app\"></div>\n<script type=\"module\">import init from \"./app.js\"; init();</script>\n";
+    write(
+        &dir,
+        &[
+            (
+                "nova.toml",
+                "[package]\nname = \"site\"\nversion = \"1.2.3\"\nedition = \"2026\"\n",
+            ),
+            ("src/main.nova", "fn main() { println(\"hi\") }\n"),
+            ("index.html", page),
+            ("public/style.css", "body {}\n"),
+            ("public/img/logo.txt", "logo"),
+            ("secret.txt", "not public"),
+        ],
+    );
+    #[cfg(unix)]
+    for (target, link) in [
+        ("public", "public/loop"),
+        ("public/img/logo.txt", "public/logo-link.txt"),
+        ("secret.txt", "public/secret-link.txt"),
+    ] {
+        std::os::unix::fs::symlink(dir.join(target), dir.join(link)).expect("a link");
+    }
+    // Links already at names the build writes: no write goes through them.
+    #[cfg(unix)]
+    for (target, link) in [
+        ("index.html", "dist/index.html"),
+        ("secret.txt", "dist/manifest.json"),
+        ("secret.txt", "dist/app.js"),
+    ] {
+        std::fs::create_dir_all(dir.join("dist")).expect("dist");
+        std::os::unix::fs::symlink(dir.join(target), dir.join(link)).expect("a link");
+    }
+    nova_in(&dir, &["build", "--target", "wasm"]).success();
+    assert_eq!(
+        std::fs::read_to_string(dir.join("index.html")).unwrap(),
+        page
+    );
+    assert_eq!(
+        std::fs::read_to_string(dir.join("secret.txt")).unwrap(),
+        "not public"
+    );
+    let dist = dir.join("dist");
+    let read = |p: &str| std::fs::read_to_string(dist.join(p)).expect(p);
+    assert_eq!(read("index.html"), page);
+    assert_eq!(read("assets/style.css"), "body {}\n");
+    assert_eq!(read("assets/img/logo.txt"), "logo");
+    assert!(!dist.join("assets/loop").exists());
+    assert!(!dist.join("assets/secret-link.txt").exists());
+    let linked: &[&str] = if cfg!(unix) {
+        &["assets/logo-link.txt"]
+    } else {
+        &[]
+    };
+    for name in linked {
+        assert_eq!(read(name), "logo");
+    }
+    let manifest = read("manifest.json");
+    assert!(
+        manifest.starts_with(
+            "{\n  \"name\": \"site\",\n  \"version\": \"1.2.3\",\n  \"release\": false,\n  \"wasm_opt\": false,\n"
+        ),
+        "{manifest}"
+    );
+    let mut files = vec![
+        "app.js",
+        "app.wasm",
+        "assets/img/logo.txt",
+        "assets/style.css",
+        "index.html",
+        "package.json",
+    ];
+    files.extend(linked);
+    files.sort_unstable();
+    assert_eq!(manifest_files(&dist), files);
+    nova_in(&dir, &["build", "--target", "wasm"]).success();
+    assert_eq!(read("manifest.json"), manifest);
+    std::fs::remove_file(dir.join("public/style.css")).expect("remove a file");
+    nova_in(&dir, &["build", "--target", "wasm"]).success();
+    assert!(!dist.join("assets/style.css").exists());
+    assert!(!read("manifest.json").contains("style.css"));
+}
+
+/// An output directory the copies would harm is refused before anything
+/// is written: the project's root, however spelled, one holding it, and one
+/// inside `public/`. The project's page and its own `assets/` stay.
+#[test]
+fn dist_may_not_hold_the_project_or_be_inside_public() {
+    // The project sits one level down, so `..` is this test's own
+    // directory: a build the guard let through writes there, and nowhere
+    // else.
+    let outer = fresh("bundle-refused");
+    let dir = outer.join("site");
+    write(
+        &dir,
+        &[
+            (
+                "nova.toml",
+                "[package]\nname = \"site\"\nversion = \"0.1.0\"\nedition = \"2026\"\n",
+            ),
+            ("src/main.nova", "fn main() { println(\"hi\") }\n"),
+            ("index.html", "mine"),
+            ("public/a.txt", "a"),
+            ("assets/own.txt", "own"),
+        ],
+    );
+    #[cfg(unix)]
+    std::os::unix::fs::symlink("..", dir.join("uplink")).expect("a link");
+    let spellings: &[&str] = if cfg!(unix) {
+        &[".", "..", "nope/..", "src/..", "uplink"]
+    } else {
+        &[".", "..", "nope/..", "src/.."]
+    };
+    for out in spellings {
+        let a = nova_in(&dir, &["build", "--target", "wasm", "-o", out]).failure();
+        assert!(
+            stderr(&a).contains("the output directory holds the project"),
+            "{out}: {}",
+            stderr(&a)
+        );
+    }
+    for built in [
+        outer.join("app.wasm"),
+        outer.join("assets"),
+        dir.join("app.wasm"),
+    ] {
+        assert!(!built.exists(), "{}", built.display());
+    }
+    let a = nova_in(&dir, &["build", "--target", "wasm", "-o", "public/out"]).failure();
+    assert!(stderr(&a).contains("inside `public/`"), "{}", stderr(&a));
+    assert_eq!(
+        std::fs::read_to_string(dir.join("index.html")).unwrap(),
+        "mine"
+    );
+    assert_eq!(
+        std::fs::read_to_string(dir.join("assets/own.txt")).unwrap(),
+        "own"
+    );
+    assert!(!dir.join("public/out").exists());
+}
+
+/// A `public/` that is a link leading outside the project is not copied,
+/// and the build says so (spec 4.4 §7: a copy cannot leave the project).
+#[cfg(unix)]
+#[test]
+fn a_public_that_leads_outside_is_not_copied() {
+    let outer = fresh("bundle-public-outside");
+    let dir = outer.join("site");
+    write(
+        &outer,
+        &[
+            (
+                "site/nova.toml",
+                "[package]\nname = \"site\"\nversion = \"0.1.0\"\nedition = \"2026\"\n",
+            ),
+            ("site/src/main.nova", "fn main() { println(\"hi\") }\n"),
+            ("elsewhere/x.txt", "not the project's"),
+        ],
+    );
+    std::os::unix::fs::symlink(outer.join("elsewhere"), dir.join("public")).expect("a link");
+    let a = nova_in(&dir, &["build", "--target", "wasm"]).success();
+    assert!(
+        stderr(&a).contains("public/ leads outside the project; it is not copied"),
+        "{}",
+        stderr(&a)
+    );
+    assert!(!dir.join("dist/assets").exists());
+    assert!(!std::fs::read_to_string(dir.join("dist/manifest.json"))
+        .expect("manifest.json")
+        .contains("x.txt"));
+}
+
+/// A stand-in for `wasm-opt`: it copies its input, the argument after
+/// `-Oz`, to its output, the one after `-o`, and prints its arguments.
+const WASM_OPT: &str = "fn main() { block_on(go()) }
+
+async fn go() {
+    let argv = args()
+    let mut line = \"wasm-opt:\"
+    let mut i = 1
+    while i < argv.len() {
+        match argv.get(i) {
+            Some(a) => line = \"${line} ${a}\"
+            None => {}
+        }
+        i = i + 1
+    }
+    println(line)
+    match argv.get(2) {
+        Some(input) => match argv.get(4) {
+            Some(output) => match read(input).await {
+                Ok(bytes) => {
+                    let _ = write(output, bytes).await
+                }
+                Err(e) => exit(2)
+            }
+            None => exit(2)
+        }
+        None => exit(2)
+    }
+}
+";
+
+/// `--release` runs `wasm-opt -Oz` when it is on `PATH`, says so in one
+/// line when it is not, and fails when it fails (spec 4.4 §7, §9.3).
+#[test]
+fn release_runs_wasm_opt_when_it_is_on_path() {
     let dir = fresh("release");
-    write(&dir, &[("main.nova", "fn main() { println(\"hi\") }\n")]);
-    let a = nova_in(
+    write(
         &dir,
-        &["build", "--target", "wasm", "--release", "main.nova"],
-    )
-    .failure();
+        &[
+            ("main.nova", "fn main() { println(\"hi\") }\n"),
+            ("standin/ok.nova", WASM_OPT),
+            ("standin/bad.nova", "fn main() { exit(3) }\n"),
+        ],
+    );
+    let exe = format!("wasm-opt{}", std::env::consts::EXE_SUFFIX);
+    for (name, bin) in [("ok", "bin-ok"), ("bad", "bin-bad")] {
+        let out = format!("{bin}/{exe}");
+        std::fs::create_dir_all(dir.join(bin)).expect("a bin directory");
+        nova_in(
+            &dir,
+            &["build", &format!("standin/{name}.nova"), "-o", &out],
+        )
+        .success();
+    }
+    let path_with = |bin: &str| {
+        let mut dirs = vec![dir.join(bin)];
+        dirs.extend(std::env::split_paths(
+            &std::env::var_os("PATH").unwrap_or_default(),
+        ));
+        std::env::join_paths(dirs).expect("a PATH")
+    };
+    let home = dir.with_file_name("nova-wasm-test-release-home");
+    let build = ["build", "--target", "wasm", "--release", "main.nova"];
+
+    // None on PATH: the build says so, and the manifest that none ran.
+    std::fs::create_dir_all(dir.join("bin-none")).expect("an empty directory");
+    let a = nova_at(&dir, &home)
+        .env("PATH", dir.join("bin-none"))
+        .args(build)
+        .assert()
+        .success();
     assert!(
-        stderr(&a).contains("`--release` does not apply to `--target wasm`"),
+        stderr(&a).contains("wasm-opt not found on PATH; app.wasm is not optimized"),
         "{}",
         stderr(&a)
     );
+    let manifest = std::fs::read_to_string(dir.join("dist/manifest.json")).expect("manifest");
+    assert!(
+        manifest.contains("\"release\": true,\n  \"wasm_opt\": false,"),
+        "{manifest}"
+    );
+
+    // The stand-in: run with `-Oz` on `app.wasm`, in `dist/`.
+    let a = nova_at(&dir, &home)
+        .env("PATH", path_with("bin-ok"))
+        .args(build)
+        .assert()
+        .success();
+    assert!(
+        stdout(&a).contains("wasm-opt: -Oz app.wasm -o app.wasm\n"),
+        "{}",
+        stdout(&a)
+    );
+    let manifest = std::fs::read_to_string(dir.join("dist/manifest.json")).expect("manifest");
+    assert!(
+        manifest.contains("\"release\": true,\n  \"wasm_opt\": true,"),
+        "{manifest}"
+    );
+    let wasm = std::fs::read(dir.join("dist/app.wasm")).expect("app.wasm");
+    assert_eq!(&wasm[..4], b"\0asm");
+
+    // One that fails fails the build; without `--release` none runs.
+    let a = nova_at(&dir, &home)
+        .env("PATH", path_with("bin-bad"))
+        .args(build)
+        .assert()
+        .failure();
+    assert!(stderr(&a).contains("wasm-opt failed"), "{}", stderr(&a));
+    let a = nova_at(&dir, &home)
+        .env("PATH", path_with("bin-bad"))
+        .args(["build", "--target", "wasm", "main.nova"])
+        .assert()
+        .success();
+    assert!(!stderr(&a).contains("wasm-opt"), "{}", stderr(&a));
 }
 
 /// The gate: hello world prints under Node what its native build prints
````

**Patch `t4-code`:**

<!-- patch t4-code -->

````diff
diff --git a/crates/nova-cli/src/bundle.rs b/crates/nova-cli/src/bundle.rs
new file mode 100644
index 0000000000000000000000000000000000000000..f4fc035222d988cb29df1cb1ce8992613122f07d
--- /dev/null
+++ b/crates/nova-cli/src/bundle.rs
@@ -0,0 +1,324 @@
+//! What `nova build --target wasm` writes beside `app.js` and `app.wasm`
+//! (the 4.4 spec
+//! `docs/superpowers/specs/2026-10-17-phase-4-4-component-syntax-design.md`
+//! §7): a project's own page, `assets/` from its `public/`, `wasm-opt`
+//! under `--release`, and `manifest.json`.
+
+use std::collections::BTreeMap;
+use std::path::{Component, Path, PathBuf};
+
+use anyhow::{bail, Context, Result};
+use nova_driver::Program;
+
+use crate::project::{self, Mode};
+
+/// The files the compiler writes into `dist/` (spec 4.0 §7).
+const BUILT: [&str; 4] = ["index.html", "app.js", "app.wasm", "package.json"];
+
+/// What a build's bundle needs to know of its program.
+pub struct Bundle {
+    /// The project's root, beside its `nova.toml`; `None` for a program
+    /// built from a single file, which keeps the generated page and has no
+    /// `assets/`.
+    pub root: Option<PathBuf>,
+    pub name: String,
+    pub version: Option<String>,
+    pub release: bool,
+}
+
+impl Bundle {
+    /// `name` and `version` from `nova.toml`, or the file's stem and none.
+    pub fn of(mode: &Mode, program: &Program, release: bool) -> Bundle {
+        match mode {
+            Mode::Project { root, .. } => Bundle {
+                root: Some(root.clone()),
+                name: project::package_name(program),
+                version: program
+                    .graph
+                    .as_ref()
+                    .map(|g| g.root().manifest.package.version.to_string()),
+                release,
+            },
+            Mode::File(file) => Bundle {
+                root: None,
+                name: file
+                    .file_stem()
+                    .map_or_else(|| "out".to_string(), |s| s.to_string_lossy().into_owned()),
+                version: None,
+                release,
+            },
+        }
+    }
+
+    /// Refuse an output directory that the copies would harm: the project's
+    /// root or a directory holding it, whose `index.html` the generated page
+    /// would overwrite and whose `assets/` a build replaces, or one inside
+    /// `public/`, which would copy itself into itself.
+    pub fn check_output(&self, out: &Path) -> Result<()> {
+        let Some(root) = &self.root else {
+            return Ok(());
+        };
+        let out = resolved(out);
+        if resolved(root).starts_with(&out) {
+            bail!("the output directory holds the project; name another with `-o`");
+        }
+        if out.starts_with(resolved(&root.join("public"))) {
+            bail!("the output directory is inside `public/`; name another with `-o`");
+        }
+        Ok(())
+    }
+
+    /// Before the compiler writes `out`: a link at a name the build writes
+    /// is removed, so that no write goes through it to a file elsewhere,
+    /// such as the project's own page.
+    pub fn prepare(&self, out: &Path) -> Result<()> {
+        for name in BUILT.iter().chain(&["manifest.json"]) {
+            let path = out.join(name);
+            let link = std::fs::symlink_metadata(&path).is_ok_and(|m| m.file_type().is_symlink());
+            if link {
+                std::fs::remove_file(&path)
+                    .or_else(|_| std::fs::remove_dir(&path))
+                    .with_context(|| format!("removing {}", path.display()))?;
+            }
+        }
+        Ok(())
+    }
+
+    /// After the compiler has written `out`: the page, `assets/`,
+    /// `wasm-opt` and `manifest.json`, in that order, so the manifest lists
+    /// what is there at the end.
+    pub fn finish(&self, out: &Path) -> Result<()> {
+        if let Some(root) = &self.root {
+            let page = root.join("index.html");
+            let to = out.join("index.html");
+            // Never onto itself: a copy would empty it first.
+            if page.is_file() && resolved(&page) != resolved(&to) {
+                std::fs::copy(&page, &to)
+                    .with_context(|| format!("copying {} to {}", page.display(), to.display()))?;
+            }
+            let assets = out.join("assets");
+            if std::fs::symlink_metadata(&assets).is_ok() {
+                remove(&assets)?;
+            }
+            let public = root.join("public");
+            if public.is_dir() {
+                let base = resolved(&public);
+                if base.starts_with(resolved(root)) {
+                    copy_tree(&public, &assets, &base)?;
+                } else {
+                    eprintln!("public/ leads outside the project; it is not copied");
+                }
+            }
+        }
+        let wasm_opt = self.release && wasm_opt(out)?;
+        let mut files = BTreeMap::new();
+        for name in BUILT {
+            files.insert(name.to_string(), size(&out.join(name))?);
+        }
+        if self.root.is_some() {
+            list(&out.join("assets"), "assets", &mut files)?;
+        }
+        let path = out.join("manifest.json");
+        std::fs::write(&path, self.manifest(wasm_opt, &files))
+            .with_context(|| format!("writing {}", path.display()))
+    }
+
+    /// `manifest.json`: the build's name, its options and its files, sorted,
+    /// with their sizes in bytes, and nothing that changes between two
+    /// builds of the same sources.
+    fn manifest(&self, wasm_opt: bool, files: &BTreeMap<String, u64>) -> String {
+        let version = self.version.as_deref().map_or("null".to_string(), quote);
+        let files: Vec<String> = files
+            .iter()
+            .map(|(path, size)| format!("    {}: {size}", quote(path)))
+            .collect();
+        format!(
+            "{{\n  \"name\": {},\n  \"version\": {version},\n  \"release\": {},\n  \"wasm_opt\": {wasm_opt},\n  \"files\": {{\n{}\n  }}\n}}\n",
+            quote(&self.name),
+            self.release,
+            files.join(",\n"),
+        )
+    }
+}
+
+/// `s` as a JSON string.
+fn quote(s: &str) -> String {
+    let mut out = String::with_capacity(s.len() + 2);
+    out.push('"');
+    for c in s.chars() {
+        match c {
+            '"' => out.push_str("\\\""),
+            '\\' => out.push_str("\\\\"),
+            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
+            c => out.push(c),
+        }
+    }
+    out.push('"');
+    out
+}
+
+/// `wasm-opt -Oz app.wasm -o app.wasm` in `out`, when `wasm-opt` is on
+/// `PATH`: whether it ran. One that fails fails the build.
+fn wasm_opt(out: &Path) -> Result<bool> {
+    let status = std::process::Command::new("wasm-opt")
+        .args(["-Oz", "app.wasm", "-o", "app.wasm"])
+        .current_dir(out)
+        .status();
+    match status {
+        Ok(s) if s.success() => Ok(true),
+        Ok(s) => bail!("wasm-opt failed: {s}"),
+        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
+            eprintln!("wasm-opt not found on PATH; app.wasm is not optimized");
+            Ok(false)
+        }
+        Err(e) => Err(e).context("running wasm-opt"),
+    }
+}
+
+/// `from`'s files into `to`, directories kept. A symbolic link to a file
+/// inside `public` is copied as the file; any other link is skipped, so a
+/// copy can neither loop nor reach outside `public/`.
+fn copy_tree(from: &Path, to: &Path, public: &Path) -> Result<()> {
+    std::fs::create_dir_all(to).with_context(|| format!("creating {}", to.display()))?;
+    let entries = std::fs::read_dir(from).with_context(|| format!("reading {}", from.display()))?;
+    for entry in entries {
+        let entry = entry.with_context(|| format!("reading {}", from.display()))?;
+        let path = entry.path();
+        let dest = to.join(entry.file_name());
+        let kind = entry
+            .file_type()
+            .with_context(|| format!("reading {}", path.display()))?;
+        let inside = || path.is_file() && resolved(&path).starts_with(public);
+        if kind.is_dir() {
+            copy_tree(&path, &dest, public)?;
+        } else if kind.is_file() || (kind.is_symlink() && inside()) {
+            std::fs::copy(&path, &dest)
+                .with_context(|| format!("copying {} to {}", path.display(), dest.display()))?;
+        }
+    }
+    Ok(())
+}
+
+/// Each file under `dir`, by its path from `dist/` with `/` between names.
+fn list(dir: &Path, prefix: &str, files: &mut BTreeMap<String, u64>) -> Result<()> {
+    let Ok(entries) = std::fs::read_dir(dir) else {
+        return Ok(());
+    };
+    for entry in entries {
+        let entry = entry.with_context(|| format!("reading {}", dir.display()))?;
+        let name = format!("{prefix}/{}", entry.file_name().to_string_lossy());
+        let path = entry.path();
+        if path.is_dir() {
+            list(&path, &name, files)?;
+        } else {
+            files.insert(name, size(&path)?);
+        }
+    }
+    Ok(())
+}
+
+fn size(path: &Path) -> Result<u64> {
+    Ok(std::fs::metadata(path)
+        .with_context(|| format!("reading {}", path.display()))?
+        .len())
+}
+
+/// A file, or a directory and all it holds.
+fn remove(path: &Path) -> Result<()> {
+    let dir = std::fs::symlink_metadata(path).is_ok_and(|m| m.is_dir());
+    if dir {
+        std::fs::remove_dir_all(path)
+    } else {
+        std::fs::remove_file(path)
+    }
+    .with_context(|| format!("removing {}", path.display()))
+}
+
+/// `path` absolute, with its links resolved as far as it exists and each
+/// `..` after that taken away, so two spellings of one directory compare
+/// equal before either is created: `nope/..` is the current directory, as
+/// `create_dir_all` and every write through it find. Windows takes each
+/// `..` away by the text first, before it looks at the disk.
+fn resolved(path: &Path) -> PathBuf {
+    let mut out = PathBuf::new();
+    let start = std::env::current_dir().unwrap_or_default();
+    let whole = if path.is_absolute() {
+        path.to_path_buf()
+    } else {
+        start.join(path)
+    };
+    let whole = if cfg!(windows) {
+        by_text(&whole)
+    } else {
+        whole
+    };
+    for part in whole.components() {
+        match part {
+            Component::CurDir => {}
+            Component::ParentDir => {
+                out.pop();
+            }
+            other => out.push(other),
+        }
+        if let Ok(real) = out.canonicalize() {
+            out = real;
+        }
+    }
+    out
+}
+
+/// `path` with each `.` and `..` taken away by the text alone.
+fn by_text(path: &Path) -> PathBuf {
+    let mut out = PathBuf::new();
+    for part in path.components() {
+        match part {
+            Component::CurDir => {}
+            Component::ParentDir => {
+                out.pop();
+            }
+            other => out.push(other),
+        }
+    }
+    out
+}
+
+#[cfg(test)]
+mod tests {
+    use super::{quote, Bundle};
+
+    /// `finish` never copies the project's page onto itself, which would
+    /// empty it, even when `check_output` is not asked first and the output
+    /// directory is spelled another way.
+    #[test]
+    fn the_page_is_never_copied_onto_itself() {
+        let dir = std::env::temp_dir().join("nova-cli-bundle-page-onto-itself");
+        let _ = std::fs::remove_dir_all(&dir);
+        std::fs::create_dir_all(dir.join("sub")).expect("a directory");
+        for (name, text) in [
+            ("index.html", "mine"),
+            ("app.js", "js"),
+            ("app.wasm", "wasm"),
+            ("package.json", "{}"),
+        ] {
+            std::fs::write(dir.join(name), text).expect("a file");
+        }
+        let bundle = Bundle {
+            root: Some(dir.clone()),
+            name: "site".to_string(),
+            version: None,
+            release: false,
+        };
+        bundle
+            .finish(&dir.join("sub").join(".."))
+            .expect("the bundle finishes");
+        let page = std::fs::read_to_string(dir.join("index.html")).expect("the page");
+        assert_eq!(page, "mine");
+    }
+
+    #[test]
+    fn quote_escapes_what_json_needs() {
+        assert_eq!(quote("plain"), "\"plain\"");
+        assert_eq!(quote("a\"b\\c\nd"), "\"a\\\"b\\\\c\\u000ad\"");
+        assert_eq!(quote("ไทย"), "\"ไทย\"");
+    }
+}
diff --git a/crates/nova-cli/src/cmd/run.rs b/crates/nova-cli/src/cmd/run.rs
index 013bfc5ef8a2db95a7e75d3cd413e3a1861d236e..45ec6a2374ed4f01641d0550a3847e1d07903d3f 100644
--- a/crates/nova-cli/src/cmd/run.rs
+++ b/crates/nova-cli/src/cmd/run.rs
@@ -11,6 +11,7 @@ use anyhow::{Context, Result};
 use clap::Args;
 use nova_driver::{Outcome, Program, Roots};
 
+use crate::bundle::Bundle;
 use crate::project::{self, Mode};
 
 #[derive(Args)]
@@ -68,7 +69,8 @@ pub struct BuildCmd {
 
     /// Optimizing build via the LLVM backend (emits LLVM IR and compiles it
     /// with a discovered `clang`/`llc`); the default is the fast Cranelift
-    /// backend.
+    /// backend. With `--target wasm`, runs `wasm-opt -Oz` on app.wasm when
+    /// `wasm-opt` is on PATH.
     #[arg(long)]
     release: bool,
 
@@ -129,11 +131,10 @@ pub fn bundle(cmd: BundleCmd) -> Result<()> {
 }
 
 /// `nova build --target wasm`: `dist/` at the project's root, or in the
-/// current directory, unless `-o` names it (spec 4.0 §7).
+/// current directory, unless `-o` names it (spec 4.0 §7), with what the
+/// 4.4 spec's §7 adds: the project's page, `assets/`, `manifest.json`, and
+/// `wasm-opt` under `--release`.
 fn build_wasm(cmd: BuildCmd) -> Result<()> {
-    if cmd.release {
-        anyhow::bail!("`--release` does not apply to `--target wasm`");
-    }
     let mode = project::mode(cmd.file)?;
     project::refuse_library(&mode)?;
     project::sync(&mode)?;
@@ -143,8 +144,12 @@ fn build_wasm(cmd: BuildCmd) -> Result<()> {
         (None, Mode::Project { root, .. }) => root.join("dist"),
         (None, Mode::File(_)) => PathBuf::from("dist"),
     };
+    let bundle = Bundle::of(&mode, &program, cmd.release);
+    bundle.check_output(&output)?;
+    bundle.prepare(&output)?;
     match nova_driver::build_program_wasm(program, &output)? {
         Outcome::Ok(dir) => {
+            bundle.finish(&dir)?;
             println!("built {}", dir.display());
             Ok(())
         }
diff --git a/crates/nova-cli/src/main.rs b/crates/nova-cli/src/main.rs
index 0ba211f9bf45c6b4480c020219e382dcd6e6b405..270807f5c709fd97846768a67bfb99b3e7777a7a 100644
--- a/crates/nova-cli/src/main.rs
+++ b/crates/nova-cli/src/main.rs
@@ -15,6 +15,7 @@
 //! and Phase 3.5a `nova doc` (spec
 //! `docs/superpowers/specs/2026-10-11-phase-3-5a-the-documentation-site-design.md`).
 
+mod bundle;
 mod cmd;
 mod embedded;
 mod project;
diff --git a/crates/nova-codegen-wasm/src/shim/index.html b/crates/nova-codegen-wasm/src/shim/index.html
index 7aa3b92a9359d482412b87ea19cb9ba46888d39d..788ad9ffe6785f316dca6b47a958125acd0fda2a 100644
--- a/crates/nova-codegen-wasm/src/shim/index.html
+++ b/crates/nova-codegen-wasm/src/shim/index.html
@@ -6,6 +6,7 @@
 <title>Nova</title>
 </head>
 <body>
+<div id="app"></div>
 <script type="module">
 import init from "./app.js";
 init();
````


### Task 5: `06-counter-spa`, the gate

Spec §1, §8, §9.4; decisions 18 and 19.

**Files:**
- Create:
  - `examples/06-counter-spa/nova.toml`, `index.html`, `src/main.nova`,
    `README.md`
  - `tests/browser/spa.mjs`
- Modify: `.github/workflows/ci.yml`
- Test:
  - `tests/dom/counter_spa.mjs`, `counter_spa.stdout`
  - `crates/nova-cli/tests/wasm_tests.rs`
  - `crates/nova-driver/tests/index_complete.rs`

- [ ] **Step 1: The tests.** `patch_block t5-tests`:
  - `build_counter_spa`, which builds the example from its directory into
    a test's own `dist/`, with `NOVA_HOME` beside it;
  - `the_counter_spa_counts_clicks`: the page copied is the example's;
    under the fake DOM, as is and under stress, `counter_spa.mjs` prints
    `Count: 0`, then the count after each of `+`, `+`, `+` and `-`, found
    by their text; and under stress each collection keeps the two
    buttons' calls;
  - `the_counter_spa_is_under_50_kb_gzipped`, by `flate2` at level 9;
  - `index_complete.rs` counts 151 programs and 175 files.
- [ ] **Step 2: Run, and see them fail.** `W counter_spa`, then `X`.
  Expected: both counter tests fail at once, as
  `examples/06-counter-spa` does not exist; `X` fails with `left: 150`,
  `right: 151`.
- [ ] **Step 3: The code.** `patch_block t5-code`:
  - **`examples/06-counter-spa`:** `nova.toml`, `name = "counter-spa"`;
    `index.html`, with `#app` and a module script that imports `init` from
    `./app.js` and calls it; `src/main.nova`, spec §1's program; and a
    README in `60-EXAMPLES.md` §9's template.
  - **`tests/browser/spa.mjs`:** 4.2's `counter.mjs`, clicking by text and
    waiting for `Count: 0`, then `Count: 2`.
  - **CI's `browser` job** builds the example into `$RUNNER_TEMP` and
    clicks it.
- [ ] **Step 4: Run.**
  - `W counter_spa`. Expected: `2 passed`.
  - `X`. Expected: `1 passed`.
  - `cargo run --locked -q -p nova-cli -- fmt --check std examples
    tests/wasm tests/dom`. Expected: nothing.
  - In Chromium, where Task 0 found Playwright:

    ```sh
    (cd examples/06-counter-spa && ../../target/debug/nova build --target wasm -o "$P/spa-dist")
    PLAYWRIGHT_BROWSERS_PATH=/opt/pw-browsers node tests/browser/spa.mjs "$P/spa-dist" /opt/node-tools
    git status --short --ignored --untracked-files=all examples/
    ```

    Expected: `built …`, then `counter-spa: +, +, +, -, and the count
    reads 2`; and `git status` lists the four new files, each `?? 
    examples/06-counter-spa/…`, and nothing else: a build writes nothing
    into the example's directory.
  - The size: `W the_counter_spa_is_under_50_kb_gzipped --nocapture`
    prints `06-counter-spa: 21019 bytes gzipped`, by `flate2` at level
    9, against 51,200. Record it in the ledger.
- [ ] **Step 5: The checks,** as the conventions list them.
- [ ] **Step 6: Commit** `examples: 06-counter-spa, the gate, under the fake DOM and in Chromium`.

**Patch `t5-tests`:**

<!-- patch t5-tests -->

````diff
diff --git a/crates/nova-cli/tests/wasm_tests.rs b/crates/nova-cli/tests/wasm_tests.rs
index 0d44cfa07fba85e6a1b7fe36e37c9e76a6c8ddab..e51a29399edf32c9e49314397c524d18913513a6 100644
--- a/crates/nova-cli/tests/wasm_tests.rs
+++ b/crates/nova-cli/tests/wasm_tests.rs
@@ -2175,6 +2175,82 @@ fn ui_the_builder_counter_counts_clicks() {
     assert_eq!(calls_after_each(&err), [1, 1, 1, 1], "{err}");
 }
 
+/// `examples/06-counter-spa`, built from its project into `dir/dist`, with
+/// `NOVA_HOME` beside `dir`.
+fn build_counter_spa(dir: &Path) -> PathBuf {
+    let dist = dir.join("dist");
+    let home = dir.with_file_name(format!(
+        "{}-home",
+        dir.file_name().expect("a name").to_string_lossy()
+    ));
+    nova_at(&repo_root().join("examples/06-counter-spa"), &home)
+        .args(["build", "--target", "wasm", "-o", &dist.to_string_lossy()])
+        .assert()
+        .success();
+    dist
+}
+
+/// The gate of 4.4 (spec
+/// `docs/superpowers/specs/2026-10-17-phase-4-4-component-syntax-design.md`
+/// §9.4): `06-counter-spa` takes its own page, and under the fake DOM `+`
+/// three times and `-` once leave `Count: 2`, as it is and under stress,
+/// where each collection keeps the same handles and the two buttons'
+/// calls.
+#[test]
+fn the_counter_spa_counts_clicks() {
+    let page = std::fs::read_to_string(repo_root().join("examples/06-counter-spa/index.html"))
+        .expect("the example's page");
+    let dir = fresh("counter-spa");
+    let dist = build_counter_spa(&dir);
+    assert_eq!(
+        std::fs::read_to_string(dist.join("index.html")).expect("index.html"),
+        page
+    );
+    if node().is_none() {
+        return;
+    }
+    let tests = repo_root().join("tests/dom");
+    let expected = std::fs::read_to_string(tests.join("counter_spa.stdout"))
+        .expect("the expected output")
+        .replace("\r\n", "\n");
+    for file in ["fake_dom.mjs", "counter_spa.mjs"] {
+        std::fs::copy(tests.join(file), dir.join(file)).expect("copy the harness");
+    }
+    for env in [&[][..], &STRESS[..]] {
+        let mut node = Command::new("node");
+        let a = node
+            .arg(dir.join("counter_spa.mjs"))
+            .envs(env.iter().copied())
+            .timeout(std::time::Duration::from_secs(60))
+            .assert()
+            .success();
+        assert_eq!(stdout(&a), expected, "{}", stderr(&a));
+        if !env.is_empty() {
+            let err = stderr(&a);
+            let calls = calls_after_each(&err);
+            assert!(!calls.is_empty(), "{err}");
+            assert!(calls.iter().all(|&c| c == 2), "{err}");
+        }
+    }
+}
+
+/// The gate's size: `06-counter-spa`'s module and shim are under 50 KB
+/// gzipped, without `wasm-opt` (spec 4.4 §1, §9.4).
+#[test]
+fn the_counter_spa_is_under_50_kb_gzipped() {
+    use std::io::Write;
+    let dist = build_counter_spa(&fresh("counter-spa-size"));
+    let mut total = 0;
+    for name in ["app.wasm", "app.js"] {
+        let bytes = std::fs::read(dist.join(name)).expect("read a dist file");
+        let mut gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::best());
+        gz.write_all(&bytes).expect("compress");
+        total += gz.finish().expect("compress").len();
+    }
+    println!("06-counter-spa: {total} bytes gzipped");
+    assert!(total < 50 * 1024, "{total} bytes gzipped");
+}
+
 /// `show` and `for_each` rebuild their part of the page between their
 /// markers, and a hidden branch's cleanups run (spec 4.3 §6.3). Under
 /// stress, a rebuilt row's listener leaves std/rt's table: after two more
diff --git a/crates/nova-driver/tests/index_complete.rs b/crates/nova-driver/tests/index_complete.rs
index 7dec05d85c9a2328e7e569d3808e6343feabb6b2..8651c29adfe7f5ba4e355e803ce9698550db1ba3 100644
--- a/crates/nova-driver/tests/index_complete.rs
+++ b/crates/nova-driver/tests/index_complete.rs
@@ -241,9 +241,9 @@ fn every_name_is_covered(a: &Analysis, file: FileId, failures: &mut Vec<String>)
 #[test]
 fn every_name_in_the_corpora_is_indexed() {
     let programs = programs();
-    // 7 examples, 142 top-level runtime programs and the modules program,
-    // on 2026-10-17, after 4.4 added `ui_view`.
-    assert_eq!(programs.len(), 150, "{programs:#?}");
+    // 8 examples, 142 top-level runtime programs and the modules program,
+    // on 2026-10-17, after 4.4 added `06-counter-spa` and `ui_view`.
+    assert_eq!(programs.len(), 151, "{programs:#?}");
     let mut files = 0;
     let mut failures: Vec<String> = Vec::new();
     for (k, path) in programs.iter().enumerate() {
@@ -270,9 +270,9 @@ fn every_name_in_the_corpora_is_indexed() {
             every_name_is_covered(&a, file, &mut failures);
         }
     }
-    // 7 + 142 + 3 files of user code, 05-geometry's library's 2, and
+    // 8 + 142 + 3 files of user code, 05-geometry's library's 2, and
     // native std's 20.
-    assert_eq!(files, 174);
+    assert_eq!(files, 175);
     failures.sort();
     failures.dedup();
     assert!(
diff --git a/tests/dom/counter_spa.mjs b/tests/dom/counter_spa.mjs
new file mode 100644
index 0000000000000000000000000000000000000000..ad2a980719efe215e5c717c84f0b8c869cc02d33
--- /dev/null
+++ b/tests/dom/counter_spa.mjs
@@ -0,0 +1,21 @@
+// The gate of 4.4 (spec
+// docs/superpowers/specs/2026-10-17-phase-4-4-component-syntax-design.md
+// §9.4): `examples/06-counter-spa`, mounted into `#app`. Print the count,
+// then click `+` three times and `-` once, a turn apart, finding the
+// buttons by their text, and print the count after each; then the page.
+import { install } from "./fake_dom.mjs";
+install();
+const app = document.createElement("div");
+app.setAttribute("id", "app");
+document.body.appendChild(app);
+const { default: init } = await import("./dist/app.js");
+await init();
+const button = (label) => document.querySelectorAll("button").find((b) => b.textContent === label);
+const count = () => document.querySelector(".counter").firstChild.textContent;
+console.log(count());
+for (const label of ["+", "+", "+", "-"]) {
+  button(label).click();
+  await new Promise((r) => setTimeout(r, 0));
+  console.log(count());
+}
+console.log(document.body.outerHTML);
diff --git a/tests/dom/counter_spa.stdout b/tests/dom/counter_spa.stdout
new file mode 100644
index 0000000000000000000000000000000000000000..5e0d699ad6f10dc7fa23fced7647c58251f673d4
--- /dev/null
+++ b/tests/dom/counter_spa.stdout
@@ -0,0 +1,6 @@
+Count: 0
+Count: 1
+Count: 2
+Count: 3
+Count: 2
+<body><div id="app"><div class="counter">Count: 2<button>+</button><button>-</button></div></div></body>
````

**Patch `t5-code`:**

<!-- patch t5-code -->

````diff
diff --git a/.github/workflows/ci.yml b/.github/workflows/ci.yml
index 968c20f7877f63bc0c6aacdc80a4af03a9277fd0..271fb70d94da814fe99436d407a3064a3be4f960 100644
--- a/.github/workflows/ci.yml
+++ b/.github/workflows/ci.yml
@@ -95,7 +95,9 @@ jobs:
   # §10.3, and 4.3's §9.4): the counters `cargo test` drives under the fake
   # DOM, one written with std/dom and one with std/ui, built for WASM and
   # clicked in Chromium through Playwright. Both have the ids `#add` and
-  # `#count`, so one script clicks either. Playwright, pinned, is installed
+  # `#count`, so one script clicks either. And 4.4's gate (§9.4):
+  # `examples/06-counter-spa`, built from its project with its own page,
+  # which `spa.mjs` clicks by its buttons' text. Playwright, pinned, is installed
   # here only, into a scratch directory; nothing in `cargo test` or `nova`
   # installs a package. Ubuntu only: one browser job suffices.
   browser:
@@ -114,6 +116,7 @@ jobs:
         run: |
           target/debug/nova build --target wasm tests/dom/counter.nova -o "$RUNNER_TEMP/counter"
           target/debug/nova build --target wasm tests/dom/ui_counter.nova -o "$RUNNER_TEMP/ui_counter"
+          (cd examples/06-counter-spa && ../../target/debug/nova build --target wasm -o "$RUNNER_TEMP/counter-spa")
       - name: Install Playwright and Chromium
         run: |
           npm install --no-save --prefix "$RUNNER_TEMP/pw" playwright@1.56.1
@@ -122,6 +125,7 @@ jobs:
         run: |
           node tests/browser/counter.mjs "$RUNNER_TEMP/counter" "$RUNNER_TEMP/pw"
           node tests/browser/counter.mjs "$RUNNER_TEMP/ui_counter" "$RUNNER_TEMP/pw"
+          node tests/browser/spa.mjs "$RUNNER_TEMP/counter-spa" "$RUNNER_TEMP/pw"
 
   fmt:
     name: Rustfmt
diff --git a/examples/06-counter-spa/README.md b/examples/06-counter-spa/README.md
new file mode 100644
index 0000000000000000000000000000000000000000..70875337c3277b9a3a9c51b6ce6321ea368c0dff
--- /dev/null
+++ b/examples/06-counter-spa/README.md
@@ -0,0 +1,41 @@
+# 06-counter-spa
+
+The smallest WASM frontend: a counter written as a component.
+
+## What this demonstrates
+
+- **`component` and `view { … }`** (RFC 0001): elements with attributes
+  by name and children in a block, and `text(…)`.
+- **Reactivity:** `text("Count: ${count.value()}")` reads a signal, so it
+  is read again, alone, each time a button sets it.
+- **`||`**, a closure with no parameters, for each button's handler.
+- **The bundler:** the project's own `index.html` is copied into `dist/`,
+  beside `app.js`, `app.wasm` and `manifest.json`.
+
+## Run it
+
+```bash
+cd examples/06-counter-spa
+nova build --target wasm
+cd dist && python3 -m http.server 8000
+```
+
+Then open `http://localhost:8000/` in a browser. Any static server works,
+as long as it serves `app.wasm` as `application/wasm`.
+
+## Expected output
+
+The page shows `Count: 0` and two buttons. `+` adds one and `-` takes one
+away: three clicks of `+` and one of `-` leave `Count: 2`.
+
+## Notes
+
+- It differs from `nova-spec/60-EXAMPLES.md` §6 in three places, each the
+  API 4.3 built: `import ui` and `import ui::html::{button, div}`,
+  `count.value()`, and `mount("#app", Counter)`.
+- `nova build --target wasm --release` runs `wasm-opt -Oz` on `app.wasm`
+  when `wasm-opt` is on `PATH`; the gate's size, under 50 KB of `app.wasm`
+  and `app.js` gzipped, is measured without it.
+- `cargo test` clicks it under a fake DOM, and CI's `browser` job in
+  Chromium (`tests/browser/spa.mjs`); the 4.4 spec is
+  `docs/superpowers/specs/2026-10-17-phase-4-4-component-syntax-design.md`.
diff --git a/examples/06-counter-spa/index.html b/examples/06-counter-spa/index.html
new file mode 100644
index 0000000000000000000000000000000000000000..ca7277215bc53b78910075782bf5412e366c1177
--- /dev/null
+++ b/examples/06-counter-spa/index.html
@@ -0,0 +1,15 @@
+<!doctype html>
+<html lang="en">
+<head>
+<meta charset="utf-8">
+<meta name="viewport" content="width=device-width, initial-scale=1">
+<title>Counter</title>
+</head>
+<body>
+<div id="app"></div>
+<script type="module">
+import init from "./app.js";
+init();
+</script>
+</body>
+</html>
diff --git a/examples/06-counter-spa/nova.toml b/examples/06-counter-spa/nova.toml
new file mode 100644
index 0000000000000000000000000000000000000000..cf814fb4bcc202c64be38770ec95fc7a65f427ea
--- /dev/null
+++ b/examples/06-counter-spa/nova.toml
@@ -0,0 +1,5 @@
+[package]
+name = "counter-spa"
+version = "0.1.0"
+edition = "2026"
+description = "The smallest WASM frontend: a counter written as a component"
diff --git a/examples/06-counter-spa/src/main.nova b/examples/06-counter-spa/src/main.nova
new file mode 100644
index 0000000000000000000000000000000000000000..ccbd651faed2429df0d53c18c776525b86975835
--- /dev/null
+++ b/examples/06-counter-spa/src/main.nova
@@ -0,0 +1,20 @@
+//! The smallest WASM frontend: a counter, written as a component with
+//! `view { … }` (RFC 0001), mounted into the page's `#app`.
+
+import ui
+import ui::html::{button, div}
+
+/// A count, and a button to add one and a button to take one away.
+component Counter() {
+    let count = signal(0)
+
+    view {
+        div(class: "counter") {
+            text("Count: ${count.value()}")
+            button(on_click: || count.update(|n| n + 1)) { text("+") }
+            button(on_click: || count.update(|n| n - 1)) { text("-") }
+        }
+    }
+}
+
+fn main() { mount("#app", Counter) }
diff --git a/tests/browser/spa.mjs b/tests/browser/spa.mjs
new file mode 100644
index 0000000000000000000000000000000000000000..7cab785ab338e472820643cb8ac380e2ce404ed0
--- /dev/null
+++ b/tests/browser/spa.mjs
@@ -0,0 +1,71 @@
+// The gate of 4.4 in Chromium (spec
+// docs/superpowers/specs/2026-10-17-phase-4-4-component-syntax-design.md
+// §9.4), run by CI's `browser` job on `examples/06-counter-spa`, built with
+// its own page:
+//
+//     node tests/browser/spa.mjs <dist> <dir holding node_modules/playwright>
+//
+// It serves `dist/` from 127.0.0.1, with the module as `application/wasm`,
+// opens the page, checks the count reads 0, clicks `+` three times and `-`
+// once, finding the buttons by their text, and checks the count reads 2 and
+// the console holds no error. Playwright is the job's own install, as for `counter.mjs`.
+
+import { createServer } from "node:http";
+import { readFile } from "node:fs/promises";
+import { createRequire } from "node:module";
+import { extname, join, resolve, sep } from "node:path";
+
+const [dist, modules] = process.argv.slice(2).map((p) => resolve(p));
+const { chromium } = createRequire(join(modules, "noop.js"))("playwright");
+
+const types = {
+  ".html": "text/html",
+  ".js": "text/javascript",
+  ".json": "application/json",
+  ".wasm": "application/wasm",
+};
+
+// `dist/`'s files, and nothing outside it.
+const server = createServer(async (req, res) => {
+  const path = decodeURIComponent(new URL(req.url, "http://127.0.0.1").pathname);
+  const file = resolve(join(dist, path === "/" ? "index.html" : path));
+  if (file !== dist && !file.startsWith(dist + sep)) {
+    res.writeHead(403).end();
+    return;
+  }
+  try {
+    const body = await readFile(file);
+    res.writeHead(200, { "content-type": types[extname(file)] ?? "application/octet-stream" });
+    res.end(body);
+  } catch {
+    res.writeHead(404).end();
+  }
+});
+await new Promise((ready) => server.listen(0, "127.0.0.1", ready));
+const url = `http://127.0.0.1:${server.address().port}/`;
+
+const browser = await chromium.launch();
+const errors = [];
+try {
+  const page = await browser.newPage();
+  page.on("console", (m) => {
+    if (m.type() === "error") errors.push(m.text());
+  });
+  page.on("pageerror", (e) => errors.push(String(e)));
+  await page.goto(url);
+  const button = (label) => page.locator("#app button", { hasText: label });
+  const reads = (text) =>
+    page.waitForFunction(
+      (t) => document.querySelector("#app .counter")?.firstChild?.textContent === t,
+      text,
+      { timeout: 10000 },
+    );
+  await reads("Count: 0");
+  for (const label of ["+", "+", "+", "-"]) await button(label).click();
+  await reads("Count: 2");
+  if (errors.length > 0) throw new Error(`the console has errors: ${errors.join("; ")}`);
+  console.log("counter-spa: +, +, +, -, and the count reads 2");
+} finally {
+  await browser.close();
+  server.close();
+}
````


### Task 6: The records

Spec §11.

**Files:**
- Create: `docs/rfcs/0001-component-syntax.md`, `docs/adr/0043-component-syntax.md`
- Modify:
  - `nova-spec/10-LEXER.md`, `11-PARSER.md`, `30-FRONTEND.md` (§3, §4, §7,
    §13), `60-EXAMPLES.md` (§6)
  - `docs/phase-4-plan.md` (4.4's entry), `CHANGELOG.md`, `ARCHITECTURE.md`
  - `docs/superpowers/specs/2026-10-07-phase-3-1-formatter-design.md`
    (its sentence that no statement begins with `||`)
  - `std/ui/html/lib.nova` (its module doc names `view`)

- [ ] **Step 1: The records.** `patch_block t6-code`.
- [ ] **Step 2: Read them against the code.** Each claim a record makes is
  one this plan's code makes true: the grammar against `grammar.rs`, the
  desugaring's table against `check/view.rs`, the bundle against
  `bundle.rs`, and the figures (21 KB; 128 lines in 17 files) against
  Task 5's and Task 1's runs. A record that says what the code does not is
  fixed here, and the ledger says so.
- [ ] **Step 3: Run.** `CARGO_INCREMENTAL=0 cargo test -p nova-std`, and
  `cargo run --locked -q -p nova-cli -- fmt --check std`, since a std
  file's doc changed. Expected: both pass.
- [ ] **Step 4: The checks,** as the conventions list them.
- [ ] **Step 5: Commit** `docs: RFC 0001, ADR 0043, and the notes for 4.4`.

**Patch `t6-code`:**

<!-- patch t6-code -->

````diff
diff --git a/ARCHITECTURE.md b/ARCHITECTURE.md
index a5a53daca6fd06b4154421d83b1d7b113a7616a0..52779fbcf4c3ab0b78ccac977452e207db706420 100644
--- a/ARCHITECTURE.md
+++ b/ARCHITECTURE.md
@@ -33,14 +33,14 @@ nova-mir            → Mid-level IR
 
 | Crate | Purpose |
 |---|---|
-| `nova-cli` | Binary entry point, CLI argument parsing |
+| `nova-cli` | Binary entry point, CLI argument parsing; a WASM build's bundle: the page, `assets/`, `manifest.json` and `wasm-opt` (ADR 0043) |
 | `nova-driver` | Orchestrates the compilation pipeline |
 | `nova-diagnostics` | Shared error reporting infrastructure, and the fixes a diagnostic suggests |
 | `nova-lexer` | Source → tokens (uses `logos`) |
 | `nova-ast` | AST node type definitions |
 | `nova-parser` | Tokens → AST (hand-written recursive descent) |
 | `nova-resolver` | Name resolution, module graph; the types of the language server's index of names; the make-public and attribute fixes |
-| `nova-typeck` | Type inference and checking (HM + extensions); records the language server's index of names when asked, and attaches fixes to its errors |
+| `nova-typeck` | Type inference and checking (HM + extensions); desugars string interpolation and `view { … }` (RFC 0001) on the way to HIR; records the language server's index of names when asked, and attaches fixes to its errors |
 | `nova-hir` | High-level IR, desugared AST |
 | `nova-mir` | Mid-level IR, 3-address style |
 | `nova-codegen-cranelift` | Fast debug backend |
@@ -53,7 +53,7 @@ nova-mir            → Mid-level IR
 | `nova-test` | A stub, `publish = false`; the test runner is `nova-cli`'s `cmd/test.rs` |
 | `nova-pm` | Package manager: `nova.toml`, the package graph, `nova.lock` and the resolver; it never touches the network (ADR 0030, ADR 0031) |
 | `nova-index` | The package index: reading, downloading, packing, the sync step and publishing (ADR 0031) |
-| `nova-bundler` | Frontend bundler; a stub, `publish = false`, until Phase 4.3 |
+| `nova-bundler` | A stub, `publish = false`: a WASM build's bundle is `nova-cli`'s (ADR 0043) |
 | `nova-doc` | Documentation generator: `nova doc`'s site, from an analysis of the program and of std, with signatures linked through the checker's index and Markdown docs (ADR 0034); a library's and std's code examples as doc test modules, which `nova test` runs (ADR 0035) |
 | `nova-bench-http` | Keep-alive HTTP load generator for benchmarking `std/http` |
 
diff --git a/CHANGELOG.md b/CHANGELOG.md
index 32a23324be45e59b3e84c56fc13c8c620388f756..b434cf570d7e97a56024c0958b6c7f4613ddc8b1 100644
--- a/CHANGELOG.md
+++ b/CHANGELOG.md
@@ -49,6 +49,21 @@ Nova uses [Semantic Versioning](https://semver.org/spec/v2.0.0.html).
   `mount` keeps one up to date in the page on WASM, and `render_to_string`
   writes one as escaped HTML on either target. A program imports them with
   `import ui` and `import ui::html` (ADR 0042).
+- **Component syntax:** `component Name(params) { … }` is a function that
+  returns a `View`, and `view { … }` writes one as elements, text and
+  views: `div(class: "c") { text("Count: ${count.value()}") }`. An
+  element's named arguments are its attributes, `value` its property and
+  `on_click` its handler; a text or attribute that is not a literal follows
+  the signals it reads. `component` and `view` stay names everywhere else
+  (RFC 0001, ADR 0043).
+- **`||` is a closure with no parameters**, `let f = || 1`, wherever an
+  expression starts.
+- **`nova build --target wasm` finishes the bundle:** a project's own
+  `index.html` is copied into `dist/`, `public/` becomes `assets/`, and
+  `manifest.json` lists what was built; `--release` runs `wasm-opt -Oz`
+  when it is on `PATH`. The generated page has `<div id="app"></div>`.
+- **`examples/06-counter-spa`**, the smallest WASM frontend, 21 KB
+  gzipped.
 
 ### Changed
 
@@ -56,6 +71,9 @@ Nova uses [Semantic Versioning](https://semver.org/spec/v2.0.0.html).
   which an `import` reaches, is E0093, and `nova new`, a manifest and
   `nova publish` refuse either as a package name.
 - **`nova-codegen-wasm` is published** with the next release: 20 crates.
+- **`nova fmt` prints `||`** for a closure with no parameters, where it
+  printed `| |`, and puts `;` before a statement that begins with `||`.
+- **`nova build --target wasm --release` builds** where it was refused.
 
 ## [0.3.2] - 2026-10-14
 
diff --git a/docs/adr/0043-component-syntax.md b/docs/adr/0043-component-syntax.md
new file mode 100644
index 0000000000000000000000000000000000000000..f6859fe725b5782039488e366d631c4f6124cc97
--- /dev/null
+++ b/docs/adr/0043-component-syntax.md
@@ -0,0 +1,109 @@
+# ADR 0043 — Component syntax
+
+## Status
+
+Accepted, 2026-10-17 (Phase 4.4, branch
+`claude/loving-archimedes-frtf5q-44`; spec
+`docs/superpowers/specs/2026-10-17-phase-4-4-component-syntax-design.md`;
+RFC 0001, `docs/rfcs/0001-component-syntax.md`). `count.value()`,
+attributes by name, reactivity by literalness, `||` everywhere with the
+repository reformatted, `wasm-opt` when it is on `PATH`, no `if` or `for`
+in `view`, and a project's `index.html` at its root are the user's
+decisions of 2026-10-17.
+
+## Context
+
+After 4.3 a program built its page with builder calls of two arrays:
+`div([class("counter")], [dyn_text(| | "${count.value()}")])`.
+`nova-spec/30-FRONTEND.md` §3 writes the same page as
+`component Counter() { view { div(class: "counter") { text(…) } } }`. The
+language 4.4 starts from has:
+- a lexer that skips line breaks, so a statement that begins with `(` or
+  `|` continues the one before;
+- 32 reserved words, kept equal to the lexer's tokens and the extension's
+  grammar by tests, and no contextual keyword;
+- `||` as one token, the logical or, so `|| x` did not parse;
+- no named arguments and no trailing blocks;
+- a formatter whose self-check compares trees, so sugar must survive
+  parsing as its own nodes;
+- and no pass that rewrites the tree: the type checker lowers it to HIR,
+  and desugars string interpolation on the way.
+
+## Decision
+
+1. **`component` and `view` are contextual keywords.** `component` is one
+   where an item starts, a name and `(` follow it, and a block (or a `->`,
+   which is refused) follows its parameters; `view` where an expression
+   starts and `{` follows it, outside an `if`, `while`, `for` or `match`
+   head. Elsewhere both are names: no `.nova` file in the repository uses
+   either as one, and `KEYWORDS` keeps its 32 words. Since line breaks are
+   skipped, two programs read otherwise: a record named `view` can no
+   longer be written as a literal, and a statement that is a variable
+   named `component`, followed by a call and then a bare block, reads as
+   a component.
+2. **The sugar is kept in the tree and desugared by the type checker.**
+   - A component is a `Function` with `is_component` set and no written
+     return type; the checker gives it `std/ui`'s `View`. Every other
+     consumer, the resolver, the index and `nova doc` among them, reads it
+     as the function it is.
+   - `Expr::View` holds `ViewNode`s, elements and expressions. The checker
+     turns an element into a call of its builder with two arrays, or one
+     for a void element, after checking the builder takes `([Attr],
+     [View])` or `([Attr])` and returns `View`; so `input` needs no special
+     case, and a program's own builder works.
+   - A view holds one root, since `View` is one node (E0094); a void
+     element takes no block (E0095); an element names a builder (E0096).
+3. **Attributes by name.** Inside an element's argument list, `name: v`
+   is the attribute `name` with each `_` made `-`; `value` is the property;
+   `on_kind` is the handler of the DOM event `kind`, spelled as the DOM
+   spells it. A handler is a `fn(String)`, given to `On` as it is, or a
+   `fn()`, given to `std/ui/html`'s `on`; either returns `()`. Any other
+   type is E0097. Named arguments exist nowhere else.
+4. **Reactivity by literalness.** A string literal with no interpolation
+   is fixed: `Static`, `Property` or `Text`. Any other value is wrapped in
+   a closure: `Dynamic`, `DynProperty` or `DynText`, which `mount` runs in
+   an effect, so the text or attribute alone follows the signals it reads.
+   `text(e)` in a view is this syntax, not a call, and needs no import; it
+   is shown on hover as `std/ui/html`'s `text`.
+5. **`||` is a closure with no parameters wherever an expression starts.**
+   It is the same node as `| |`, which `nova fmt` now prints as `||`; the
+   repository's 128 lines that used `| |` were reformatted by `nova fmt`
+   itself. A statement that begins with `||`, like one that begins with
+   `|`, gets `;` before it, and so does a view child that would continue
+   the one before it; the parser takes an optional `;` between children.
+6. **The bundler's last pieces.** `nova build --target wasm` copies a
+   project's own `index.html` from its root, or writes the generated page,
+   which now has `<div id="app"></div>`; copies `public/` into `assets/`
+   afresh, a link only when it leads to a file inside `public/`, and not
+   at all when `public/` itself leads outside the project; and writes
+   `manifest.json`, which lists every file it wrote with its size and
+   nothing that changes between two builds of the same sources. It
+   refuses an output directory that holds the project, however spelled, or
+   lies inside `public/`, and removes a link at any name it writes before
+   it writes, so no write reaches a file elsewhere. `--release` runs
+   `wasm-opt -Oz` on `app.wasm` when `wasm-opt` is on `PATH`, says so in
+   one line when it is not, and fails when it fails; nothing installs
+   `wasm-opt`.
+
+## Consequences
+
+- **`06-counter-spa` is written as `30-FRONTEND.md` §3 writes it**, but for
+  `import ui`, `value()` and `mount("#app", Counter)`; its module and shim
+  are 21 KB by `gzip -9`, against the gate's 50 KB, without `wasm-opt`.
+- **A value that never changes but is not a literal**, such as `class:
+  name`, costs an effect that runs once; the output is the same.
+- **The language server colours `component` and `view` as keywords** from
+  the parse, by a `keyword` semantic token, since the extension's grammar
+  cannot tell `view {` in an `if` head from a view; its two rules colour
+  the common case before the server answers.
+- **`if` and `for` in a view, fragments, boolean properties such as
+  `checked`, and named arguments outside `view`** are not in 4.4. Each can
+  come later without changing what a view means today.
+
+## References
+
+- `docs/phase-4-plan.md` §3, decisions 6, 7 and 8, and §4's 4.4 entry.
+- The spec and RFC named above.
+- ADR 0028, the formatter; ADR 0042, `std/ui`.
+- `nova-spec/10-LEXER.md`, `11-PARSER.md`, `30-FRONTEND.md` §3, §4 and §7,
+  and `60-EXAMPLES.md` §6.
diff --git a/docs/phase-4-plan.md b/docs/phase-4-plan.md
index 05bb13ed9035cbb6f992fa14d53b91948954be0a..38702d5b49837587bcedf074106e6350a9a3d14b 100644
--- a/docs/phase-4-plan.md
+++ b/docs/phase-4-plan.md
@@ -521,6 +521,18 @@ is `tests/dom/ui_counter.nova` under the fake DOM and in Chromium, and
   - It is under 50 KB gzipped.
   - `nova fmt`'s three checks pass on every `.nova` file.
 
+**Amended 2026-10-17 (branch `claude/loving-archimedes-frtf5q-44`):** by the
+user's decisions of 2026-10-17, a signal is read with `count.value()`; an
+element's named arguments are its attributes by name, `value` its property
+and `on_kind` its handler; a text or attribute that is not a literal is
+reactive by itself; `||` is a closure everywhere, and `nova fmt` reformatted
+the repository's `| |`; `--release` runs `wasm-opt` only when it is on
+`PATH`; a view has no `if` or `for`; and a project's `index.html` is at its
+root. The type checker desugars the syntax, which the tree keeps. The gate
+is `06-counter-spa` under the fake DOM and in Chromium, 21 KB gzipped
+(`docs/superpowers/specs/2026-10-17-phase-4-4-component-syntax-design.md`;
+RFC 0001; ADR 0043).
+
 ### 4.5 — The router and `nova dev`
 - **`std/ui/router`:**
   - `Router` and `Route`;
diff --git a/docs/rfcs/0001-component-syntax.md b/docs/rfcs/0001-component-syntax.md
new file mode 100644
index 0000000000000000000000000000000000000000..e872a398c596e45f2b5ab4b196dae3d89126d946
--- /dev/null
+++ b/docs/rfcs/0001-component-syntax.md
@@ -0,0 +1,122 @@
+# RFC 0001: Component syntax
+
+**Status:** Accepted, 2026-10-17 (Phase 4.4; ADR 0043)
+**Spec:** `docs/superpowers/specs/2026-10-17-phase-4-4-component-syntax-design.md`
+
+## Summary
+
+A component is a function that returns a `View`, written with
+`component`; its view is a tree of elements, text and views, written with
+`view { … }`. Inside a view an element takes its attributes by name and its
+children in a trailing block. Everywhere, `||` is a closure with no
+parameters.
+
+```nova
+import ui
+import ui::html::{button, div}
+
+component Counter() {
+    let count = signal(0)
+
+    view {
+        div(class: "counter") {
+            text("Count: ${count.value()}")
+            button(on_click: || count.update(|n| n + 1)) { text("+") }
+            button(on_click: || count.update(|n| n - 1)) { text("-") }
+        }
+    }
+}
+
+fn main() { mount("#app", Counter) }
+```
+
+## Grammar
+
+```
+item       += [ "pub" ] "component" IDENT "(" params ")" block
+atom       += "view" "{" child* "}"
+            | "||" [ "->" type ] expr
+child       = ( element | postfix_expr ) [ ";" ]
+element     = IDENT "{" child* "}"
+            | IDENT "(" [ view_args ] ")" [ "{" child* "}" ]
+view_args   = view_arg { "," view_arg } [ "," ]
+view_arg    = NAME ":" expr | expr
+NAME        = IDENT | keyword
+```
+
+- `component` is a keyword only where an item starts, a name and `(`
+  follow it, and a block follows its parameters (or a `->`, refused below).
+  A component has no type parameters, no `async` and no written
+  return type: `component A() -> Int` and `component A<T>()` do not parse.
+- `view` is a keyword only where an expression starts and `{` follows it,
+  and not in the head of an `if`, `while`, `for` or `match`, where `{`
+  opens the body.
+- A child is an element or a postfix expression: a name, a call, a method
+  call, a field, a literal, a parenthesised expression, a block, or another
+  view. An expression with an operator is written in parentheses.
+- An element is a name followed by a trailing block, or by an argument
+  list holding at least one named argument, or both. A call with only
+  positional arguments and no block, such as `Counter()` or `show(open,
+  || view { … })`, is an ordinary expression.
+- Line breaks are skipped, so a child that begins with `(`, `[` or `{`
+  would continue the one before; a `;` ends a child, and `nova fmt` writes
+  one there, as it does between statements.
+- `||` where an expression starts is the closure `| |`; elsewhere it is
+  the logical or.
+
+## Meaning
+
+The parser keeps the sugar as its own nodes, and the type checker turns it
+into `std/ui`'s API, as it does string interpolation.
+
+| Written | Means |
+|---|---|
+| `component Name(params) { body }` | `fn Name(params) -> View { body }` |
+| `view { c }` | the `View` `c` means; no child, or several, is E0094 |
+| `name(args) { children }`, `name` taking `([Attr], [View])` | `name([attrs], [children])` |
+| `name(args)`, `name` taking `([Attr])` | `name([attrs])`; a block is E0095 |
+| an element whose `name` is anything else | E0096 |
+| `text("literal")` | `Text("literal")` |
+| `text(e)` | `DynText(\|\| e)` |
+| a positional argument `a` | `a`, an `Attr` |
+| `on_kind: h`, `h: fn()` | `on("kind", h)`, which is `On("kind", \|v: String\| h())` |
+| `on_kind: h`, `h: fn(String)` | `On("kind", h)` |
+| `value: "literal"` | `Property("value", "literal")` |
+| `value: v` | `DynProperty("value", \|\| v)` |
+| `name: "literal"` | `Static("name", "literal")` |
+| `name: v` | `Dynamic("name", \|\| v)` |
+
+- An attribute's name is `name` with each `_` made `-`: `aria_label` is
+  `aria-label`. The event after `on_` keeps the DOM's spelling:
+  `on_keydown` is `keydown`.
+- A literal is a string literal without interpolation; anything else,
+  `"a ${b}"` and `name` included, is read again when a signal it reads
+  changes. A value that is not a `String`, or a handler of another type, is
+  E0097, naming the attribute and the type found.
+- A child that is an expression must be a `View`.
+- `text(e)` needs no import, and the element's builder is the function of
+  that name in scope: `import ui::html::{div}` brings `div`.
+
+## Alternatives the user weighed
+
+- **`count.value` for a signal's value**, as `30-FRONTEND.md` §3 writes
+  it: `Signal` is a record whose value is a field, so a read could not be
+  tracked. `count.value()` it is.
+- **Helper functions for attributes**, `div(class("a"))`, as 4.3's builders
+  take them: they still work as positional arguments, but a name is
+  shorter and needs no import.
+- **Explicit reactivity**, `text(dyn("…"))`: every text that reads a
+  signal would need it, and forgetting it shows a stale page with no
+  error.
+- **`| |` kept**, with `||` only inside views: one spelling everywhere is
+  simpler, and the formatter's tree check proves the reformatting changes
+  nothing.
+- **`if` and `for` inside `view`**: `show` and `for_each` with `|| view {
+  … }` do the same with what `std/ui` already has.
+
+## Not in this RFC
+
+- `if` and `for` in a view; a view with several roots, or fragments.
+- Named arguments and trailing blocks outside `view`.
+- Boolean properties such as `checked`: `Attr` holds strings only.
+- Type parameters on a component.
diff --git a/docs/superpowers/specs/2026-10-07-phase-3-1-formatter-design.md b/docs/superpowers/specs/2026-10-07-phase-3-1-formatter-design.md
index 4d70e1a394594d2ed13182cfd350edac5242b677..c1c4ace281b4c02c178788c235d13e0f8b05dc6e 100644
--- a/docs/superpowers/specs/2026-10-07-phase-3-1-formatter-design.md
+++ b/docs/superpowers/specs/2026-10-07-phase-3-1-formatter-design.md
@@ -839,6 +839,9 @@ review. Items 1, 2 and 4 correct what Sections 1, 2 and 5 said. Items 5 to
 4. §5.6's token set is Section 2's six plus `{`, because a block statement
    after a path would read as a record literal. It does not include `&&`
    or `||`: no statement or arm can begin with either (§2).
+   **Amended 2026-10-17 (Phase 4.4, ADR 0043):** a statement can now begin
+   with `||`, a closure with no parameters, so the list includes it, and
+   an empty closure parameter list prints as `||`.
 5. A `return` or `break` without a value keeps its `;` when a statement
    follows (§5.6). Without it, `return` followed by `x = 1` would re-parse
    as `return x = 1`.
diff --git a/nova-spec/10-LEXER.md b/nova-spec/10-LEXER.md
index d11039fcc48a54ac3f8cb08b5affa758a874fe8b..9bdd0b0c970707db2e25808deb05824921d8ee6e 100644
--- a/nova-spec/10-LEXER.md
+++ b/nova-spec/10-LEXER.md
@@ -71,6 +71,13 @@ plain comments. `lex_with_comments` returns the same tokens, and every plain
 comment, line or block, with its span, for the formatter
 (`docs/superpowers/specs/2026-10-07-phase-3-1-formatter-design.md` §3).
 
+**Amended 2026-10-17 (branch `claude/loving-archimedes-frtf5q-44`):** `component` and
+`view` are contextual keywords: the lexer makes them identifiers, and the
+parser reads them as keywords only in their places (RFC 0001). `KEYWORDS`,
+the 32 reserved words a test keeps equal to the lexer's tokens, does not
+change. `||` stays one token, which the parser takes as a closure with no
+parameters where an expression starts (`docs/superpowers/specs/2026-10-17-phase-4-4-component-syntax-design.md` §2, §3; ADR 0043).
+
 ---
 
 ## 3. String Interpolation Handling
diff --git a/nova-spec/11-PARSER.md b/nova-spec/11-PARSER.md
index c33f5fc237349dccf03778af05616542715da561..1bb95ed76c47fd28712edef2c0de4c7dbe9f063e 100644
--- a/nova-spec/11-PARSER.md
+++ b/nova-spec/11-PARSER.md
@@ -216,6 +216,34 @@ the language server (`docs/superpowers/specs/2026-10-08-phase-3-2-lsp-core-desig
 `parse` is unchanged, and only the server's analysis type-checks the empty
 name.
 
+**Amended 2026-10-17 (branch `claude/loving-archimedes-frtf5q-44`):** component syntax
+(RFC 0001; `docs/superpowers/specs/2026-10-17-phase-4-4-component-syntax-design.md` §3):
+
+```ebnf
+item        += [ "pub" ] "component" ident "(" [ params ] ")" block ;
+atom        += "view" "{" { child } "}"
+             | "||" [ "->" type ] expr ;
+child        = ( element | postfix_expr ) [ ";" ] ;
+element      = ident "{" { child } "}"
+             | ident "(" [ view_args ] ")" [ "{" { child } "}" ] ;
+view_args    = view_arg { "," view_arg } [ "," ] ;
+view_arg     = ( ident | keyword ) ":" expr | expr ;
+```
+
+- `component` is a keyword where an item starts, a name and `(` follow,
+  and `{` (or `->`) follows its parameters;
+  the node is a `Function` with `is_component` set and no return type. One
+  written with `->` is an error, "a component returns a `View`, which it
+  does not write".
+- `view` is a keyword where an expression starts and `{` follows, but not
+  where a record literal could not start, in an `if`, `while`, `for` or
+  `match` head. Inside its braces, record literals are allowed again.
+- An element is a name and a block, or a name and arguments holding a
+  named one, or both; a name and positional arguments alone is a call.
+  Named arguments parse nowhere else.
+- A `;` may end a view's child, so the next can begin with `(`, `[` or
+  `{`.
+
 ---
 
 ## 3. AST Node Definitions (Rust)
diff --git a/nova-spec/30-FRONTEND.md b/nova-spec/30-FRONTEND.md
index 49c1a3cb02ac7a1c56e1b98f6b8da1ecd3622900..41b7675b7d9249a08f920a68f89030859d08ca79 100644
--- a/nova-spec/30-FRONTEND.md
+++ b/nova-spec/30-FRONTEND.md
@@ -105,6 +105,22 @@ fn Counter() -> View {
 
 The `|| ...` closure subscribes to `count` because `count.value` access within an effect tracks dependencies.
 
+**Amended 2026-10-17 (branch `claude/loving-archimedes-frtf5q-44`):** 4.4 builds
+this syntax (RFC 0001). A signal is read with `count.value()`, as 4.3
+built it. `component` and `view` are contextual keywords: `component` where
+an item starts, a name and `(` follow and a block (or a refused `->`)
+follows the parameters, `view` where an expression starts
+and `{` follows, outside an `if`, `while`, `for` or `match` head. A
+component has no type parameters and no written return type. The type
+checker, not a pass before it, desugars a view into 4.3's builders: an
+element becomes a call of the function of its name with `[Attr]` and
+`[View]`, or `[Attr]` alone for a void element; `text("literal")` is
+`Text`, and any other `text(e)` is `DynText(|| e)`, so text that is not a
+literal follows its signals. A view holds one root (E0094); a void element
+takes no block (E0095); an element names a builder (E0096). `if` and `for`
+are not in a view: `show` and `for_each` with `|| view { … }` do that
+(`docs/superpowers/specs/2026-10-17-phase-4-4-component-syntax-design.md` §3, §4; ADR 0043).
+
 ---
 
 ## 4. Element Builder API
@@ -147,6 +163,16 @@ each time its signal is set; `show` has no type parameter
 (`docs/superpowers/specs/2026-10-17-phase-4-3-std-ui-reactivity-and-views-design.md`
 §6, §7; ADR 0042).
 
+**Amended 2026-10-17 (branch `claude/loving-archimedes-frtf5q-44`):** inside a
+view, an element takes attributes by name, `div(class: "a") { … }`, with
+no helper needed: `_` becomes `-`, `value` is the property, and `on_kind` is
+the handler of the DOM event `kind`, a `fn()` or a `fn(String)` given the
+target's value. A string literal with no interpolation is fixed; any other
+value becomes `Dynamic`, `DynProperty` or `DynText`, read again when its
+signals change. A value that is not a `String`, or a handler of another
+type, is E0097. A positional argument is an `Attr`, so 4.3's helpers still
+work. Named arguments exist only there (`docs/superpowers/specs/2026-10-17-phase-4-4-component-syntax-design.md` §4; ADR 0043).
+
 ---
 
 ## 5. DOM Bindings (`std/dom`)
@@ -250,6 +276,20 @@ user's decision of 2026-10-16, a user's own `index.html`, `assets/` from
 (`docs/superpowers/specs/2026-10-17-phase-4-3-std-ui-reactivity-and-views-design.md`
 decision 7).
 
+**Amended 2026-10-17 (branch `claude/loving-archimedes-frtf5q-44`):** 4.4 builds
+them, without `chunks/`. A project's own `index.html`, beside `nova.toml`,
+is copied as it is, and must load `app.js` and call its `init()`;
+otherwise the generated page, which has `<div id="app"></div>`, is written.
+`public/` is copied into `assets/` afresh on each build, a link only when
+it leads to a file inside `public/`, and nothing when `public/` itself
+leads outside the project. `manifest.json` holds the build's name and version, from
+`nova.toml` or the file's stem and `null`, whether it was `--release` and
+whether `wasm-opt` ran, and each file written with its size, sorted, and
+nothing that changes between two builds. `--release` runs `wasm-opt -Oz` on
+`app.wasm` when it is on `PATH`, says so when it is not, and fails when it
+fails. An output directory that holds the project or lies inside `public/`
+is refused, and a link at a name the build writes is removed first (`docs/superpowers/specs/2026-10-17-phase-4-4-component-syntax-design.md` §7; ADR 0043).
+
 ### 7.1 Output Layout
 ```
 dist/
@@ -397,3 +437,10 @@ those that do not panic under each target's collector stress too;
 `tests/dom/ui_*.nova` render and click under the fake DOM, and CI's
 `browser` job clicks the builder counter in Chromium
 (`docs/superpowers/specs/2026-10-17-phase-4-3-std-ui-reactivity-and-views-design.md` §9).
+
+**Amended 2026-10-17 (branch `claude/loving-archimedes-frtf5q-44`):**
+`tests/runtime/ui_view.nova` renders components and views on both targets;
+`examples/06-counter-spa` is clicked under the fake DOM in `cargo test`, as
+it is and under stress, and in Chromium by the `browser` job
+(`tests/browser/spa.mjs`); its `app.wasm` and `app.js` are under 50 KB
+gzipped (`docs/superpowers/specs/2026-10-17-phase-4-4-component-syntax-design.md` §9).
diff --git a/nova-spec/60-EXAMPLES.md b/nova-spec/60-EXAMPLES.md
index 4022222f4849b7add6e2d0ebc6dcea5e9f44a5ab..c304985fb01f79662e2681ce58289549b8e5103a 100644
--- a/nova-spec/60-EXAMPLES.md
+++ b/nova-spec/60-EXAMPLES.md
@@ -648,6 +648,15 @@ fn main() {
 
 **Gate:** `nova dev` opens browser, counter responds to clicks. Bundle size < 50 KB gzipped.
 
+**Amended 2026-10-17 (branch `claude/loving-archimedes-frtf5q-44`):**
+`examples/06-counter-spa` is built in 4.4 and differs from the above in
+three places, each the API 4.3 built: `import ui` and `import
+ui::html::{button, div}`, `count.value()`, and `mount("#app", Counter)`.
+Its `index.html` loads `./app.js` as a module and calls `init()`, and it has
+a `nova.toml` and a README. The gate is `nova build --target wasm` and
+clicks in Chromium, since `nova dev` comes with 4.5; its `app.wasm` and
+`app.js` are 21 KB gzipped (`docs/superpowers/specs/2026-10-17-phase-4-4-component-syntax-design.md` §8; ADR 0043).
+
 ---
 
 ## 7. `07-fullstack-blog` (Phase 4 gate)
diff --git a/std/ui/html/lib.nova b/std/ui/html/lib.nova
index a074c420f46c8242fa98b976be73a8a07d7487ba..3cf9f22dd7185ccfdf6bea1a4da8f4d98de6e304 100644
--- a/std/ui/html/lib.nova
+++ b/std/ui/html/lib.nova
@@ -12,7 +12,9 @@
 //! `dyn_text` text read again when the signals it reads change. `type` and
 //! `for` are keywords, so `attr("type", "checkbox")` writes those, and
 //! `element("main", [], [])` writes a `<main>`, which has no builder of its
-//! own: it would clash with a program's `fn main`.
+//! own: it would clash with a program's `fn main`. Inside `view { … }`
+//! (RFC 0001), an element takes its attributes by name, `type:` and `for:`
+//! included, and its children in a block.
 //!
 //! A list with a class, and the HTML it renders to:
 //!
````


### Task 7: Verification, the mutants, the review, the PR

- [ ] **Step 1: The full checks.** First, `df -h /` must show at least 8G
  free; if not, remove scratch directories of earlier sub-phases.

  ```sh
  cargo fmt --all -- --check
  cargo clippy --locked --all-targets --all-features -- -D warnings
  cargo run --locked -q -p nova-cli -- fmt --check std examples tests/wasm tests/dom
  cargo run --locked -q -p nova-cli -- fmt --check tests/runtime/ui_*.nova
  cargo build --locked --workspace
  CARGO_INCREMENTAL=0 NOVA_REQUIRE_NODE=1 cargo test --locked --workspace --no-fail-fast > "$P/full-44x.txt" 2>&1
  grep '^test result' "$P/full-44x.txt" | awk '{p += $4; f += $6; i += $8} END {print p, f, i}'
  grep ' FAILED$' "$P/full-44x.txt"
  ```

  Expected:
  - both formatters print nothing, and clippy ends with `Finished`, with
    no warning or error;
  - `2316 1 10`, against the baseline's `2286 1 10`: 30 new tests: eight
    in `parser_tests`, seven each in `nova-typeck` and `wasm_tests` (the
    gate's `ui_view` among them, and `a_public_that_leads_outside_is_not_copied`,
    on Unix only; `release_does_not_apply_to_wasm` gone), two each in
    `layout`, `nova-lsp` and `nova-cli`'s `bundle.rs`, and one each in
    `vscode_extension` and `nova-doc`;
  - the one failure is Task 0's IPv6 test.
- [ ] **Step 2: The lockfile.** `git diff --numstat 3a5b1ad -- Cargo.lock`.
  Expected: nothing.
- [ ] **Step 3: Rust 1.78.**

  ```sh
  RUSTFLAGS="-D warnings" CARGO_TARGET_DIR="$P/msrv-44" cargo +1.78 check --locked --workspace
  ```

  Expected: clean, in about a minute. Then `rm -rf "$P/msrv-44"`.
- [ ] **Step 4: The mutants.** One at a time, each edit is made, its tests
  run, and the file restored with `git checkout -- <file>`.
  - Each edit replaces one exact text, which occurs once in its file; "the
    line goes" removes the whole line holding it.
  - Each must fail the tests named. A mutant that passes them is a gap:
    record it in the ledger, and write the missing test before the
    review.
  - At the end, `git status --short` prints nothing.

  Each command runs one test filter: cargo's filters are joined by "or",
  so a second name would widen the run, not narrow it. Each command below
  runs a whole test binary, and names the tests that fail in it.
  1. **`||` is no closure.** `crates/nova-parser/src/grammar.rs`:
     `Token::Pipe | Token::PipePipe => self.parse_closure(ctx),` becomes `Token::Pipe => self.parse_closure(ctx),`.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-parser --test parser_tests`.
     - Expected: fails: `a_view_holds_elements_and_expressions`, `pipe_pipe_is_a_closure_where_an_expression_starts_and_an_or_elsewhere`.
  2. **`view` in an `if` head is a view.** `crates/nova-parser/src/grammar.rs`:
     replace

     ```
                     if w == "view"
                         && !self.no_struct_literal
     ```

     with

     ```
                     if w == "view"
     ```

     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-parser --test parser_tests`.
     - Expected: fails: `component_and_view_are_names_elsewhere`, `view_before_an_if_body_is_a_name`.
  3. **A call of positional arguments is an element.** `crates/nova-parser/src/grammar.rs`:
     `if named {` becomes `if true || named {`.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-parser --test parser_tests`.
     - Expected: fails: `a_semicolon_ends_a_child_before_a_parenthesis`, `a_view_holds_elements_and_expressions`.
  4. **No `;` between children.** `crates/nova-parser/src/grammar.rs`:
     `if self.reject_docs() || self.eat(&Token::Semicolon).is_some() {` becomes `if self.reject_docs() {`.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-parser --test parser_tests`.
     - Expected: fails: `a_semicolon_ends_a_child_before_a_parenthesis`.
  5. **No keyword as an argument's name.** `crates/nova-parser/src/grammar.rs`:
     replace

     ```
                     nova_lexer::KEYWORDS
                         .contains(&word)
     ```

     with

     ```
                     [""]
                         .contains(&word)
     ```

     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-parser --test parser_tests`.
     - Expected: fails: `a_view_holds_elements_and_expressions`.
  6. **A component's `->` drops the item.** `crates/nova-parser/src/grammar.rs`:
     replace

     ```
                 self.parse_type("return type")?;
             }
             let body = self.parse_block("component body")?;
     ```

     with

     ```
                 return None;
             }
             let body = self.parse_block("component body")?;
     ```

     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-parser --test parser_tests`.
     - Expected: fails: `a_component_with_a_return_type_or_type_parameters_does_not_parse`.
  7. **`nova fmt` prints `| |`.** `crates/nova-fmt/src/print/expr.rs`:
     `parts.push(text("||"));` becomes `parts.push(text("| |"));`.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-fmt --no-fail-fast`.
     - Expected: fails: `closures_print_their_parameters_bare_or_typed`, `components_and_views_have_one_layout`, `each_token_that_could_continue_a_statement_keeps_its_semicolon`.
  8. **`||` continues no statement.** `crates/nova-fmt/src/print/mod.rs`:
     replace

     ```
         Token::Pipe,
         Token::PipePipe,
     ];
     ```

     with

     ```
         Token::Pipe,
         Token::Pipe,
     ];
     ```

     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-fmt --no-fail-fast`.
     - Expected: fails: `each_token_that_could_continue_a_statement_keeps_its_semicolon`.
  9. **No `;` before a child that would continue.** `crates/nova-fmt/src/print/expr.rs`:
     replace

     ```
                     .map(|_| ";");
                 let end = self
                     .src
                     .token_end(child.span.end, &Token::Semicolon)
     ```

     with

     ```
                     .map(|_| "");
                 let end = self
                     .src
                     .token_end(child.span.end, &Token::Semicolon)
     ```

     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-fmt --no-fail-fast`.
     - Expected: fails: `view_children_keep_their_separators_and_comments`.
  10. **One child never shares the line.** `crates/nova-fmt/src/print/expr.rs`:
     `} else if children.len() == 1 && !commented {` becomes `} else if false {`.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-fmt --no-fail-fast`.
     - Expected: fails: `components_and_views_have_one_layout`.
  11. **An empty block is not `{}`.** `crates/nova-fmt/src/print/expr.rs`:
     replace

     ```
                 if children.is_empty() && !commented {
                     parts.push(text("{}"));
     ```

     with

     ```
                 if false {
                     parts.push(text("{}"));
     ```

     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-fmt --no-fail-fast`.
     - Expected: fails: `components_and_views_have_one_layout`.
  12. **An element's arguments are dropped.** `crates/nova-fmt/src/print/expr.rs`:
     `if let Some(last) = args.last() {` becomes `if let Some(last) = args.last().filter(|_| false) {`.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-fmt --no-fail-fast`.
     - Expected: fails: `components_and_views_have_one_layout`, `every_corpus_file_formats_to_the_same_program_and_comments`.
  13. **A component prints as `fn`.** `crates/nova-fmt/src/print/item.rs`:
     `(keyword, format!("{}component ", vis(f.vis)))` becomes `(keyword, format!("{}fn ", vis(f.vis)))`.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-fmt --no-fail-fast`.
     - Expected: fails: `components_and_views_have_one_layout`, `every_corpus_file_formats_to_the_same_program_and_comments`, `view_children_keep_their_separators_and_comments`.
  14. **`component` before any call is an item.** `crates/nova-parser/src/grammar.rs`:
     `matches!(at(k + 1), Some(Token::LBrace | Token::Arrow))` becomes `true`.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-parser --test parser_tests`.
     - Expected: fails: `component_and_view_are_names_elsewhere`.
  15. **A component returns `()`.** `crates/nova-typeck/src/check.rs`:
     `self.component_ret()` becomes `Ty::Unit`.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-typeck --lib`.
     - Expected: fails: `check::tests::a_child_or_text_of_another_type_is_e0010`, `check::tests::a_view_desugars_to_std_uis_calls`, `check::tests::a_views_parts_are_fixed_or_read_again_by_literalness`, `check::tests::an_attribute_or_handler_of_another_type_is_e0097`.
  16. **`_` stays `_`.** `crates/nova-typeck/src/check/view.rs`:
     `name.value.replace('_', "-")` becomes `name.value.clone()`.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-typeck --lib`.
     - Expected: fails: `check::tests::a_views_parts_are_fixed_or_read_again_by_literalness`.
  17. **`value` is no property.** `crates/nova-typeck/src/check/view.rs`:
     `let property = name.value == "value";` becomes `let property = false;`.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-typeck --lib`.
     - Expected: fails: `check::tests::a_views_parts_are_fixed_or_read_again_by_literalness`.
  18. **A literal attribute is reactive.** `crates/nova-typeck/src/check/view.rs`:
     replace

     ```
             if is_str_lit(value) {
                 let v = self.check_expr(fcx, value);
     ```

     with

     ```
             if false {
                 let v = self.check_expr(fcx, value);
     ```

     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-typeck --lib`.
     - Expected: fails: `check::tests::a_views_parts_are_fixed_or_read_again_by_literalness`.
  19. **Literal text is reactive.** `crates/nova-typeck/src/check/view.rs`:
     `if is_str_lit(content) {` becomes `if false {`.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-typeck --lib`.
     - Expected: fails: `check::tests::a_views_parts_are_fixed_or_read_again_by_literalness`.
  20. **An interpolated string is a literal.** `crates/nova-typeck/src/check/view.rs`:
     `parts.iter().all(|p| matches!(p, ast::StringPart::Lit(_)))` becomes `parts.iter().any(|p| matches!(p, ast::StringPart::Lit(_)))`.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-typeck --lib`.
     - Expected: fails: `check::tests::a_views_parts_are_fixed_or_read_again_by_literalness`.
  21. **A raw string is reactive.** `crates/nova-typeck/src/check/view.rs`:
     `ast::Expr::Lit(ast::Literal::Str(_)) => true,` becomes `ast::Expr::Lit(ast::Literal::Str(_)) => false,`.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-typeck --lib`.
     - Expected: fails: `check::tests::a_views_parts_are_fixed_or_read_again_by_literalness`.
  22. **A handler may return a value.** `crates/nova-typeck/src/check/view.rs`:
     replace

     ```
                 Ty::Fn { params, ret } if params.is_empty() && fcx.icx.unify(&ret, &Ty::Unit) => {
     ```

     with

     ```
                 Ty::Fn { params, ret } if params.is_empty() => {
     ```

     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-typeck --lib`.
     - Expected: fails: `check::tests::an_attribute_or_handler_of_another_type_is_e0097`.
  23. **Any one-argument call is text.** `crates/nova-typeck/src/check/view.rs`:
     replace

     ```
             ([word], [content]) if word.value == "text" => Some((word, content)),
     ```

     with

     ```
             ([word], [content]) => Some((word, content)),
     ```

     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-typeck --lib`.
     - Expected: fails: `check::tests::a_views_parts_are_fixed_or_read_again_by_literalness`.
  24. **`on_` names no handler.** `crates/nova-typeck/src/check/view.rs`:
     `if let Some(kind) = name.value.strip_prefix("on_") {` becomes `if let Some(kind) = name.value.strip_prefix("on_").filter(|_| false) {`.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-typeck --lib`.
     - Expected: fails: `check::tests::a_view_desugars_to_std_uis_calls`, `check::tests::a_views_parts_are_fixed_or_read_again_by_literalness`.
  25. **`on_` names the event "".** `crates/nova-typeck/src/check/view.rs`:
     `if kind.is_empty() {` becomes `if false {`.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-typeck --lib`.
     - Expected: fails: `check::tests::an_attribute_or_handler_of_another_type_is_e0097`.
  26. **Two roots are one.** `crates/nova-typeck/src/check/view.rs`:
     `if children.len() != 1 {` becomes `if children.is_empty() {`.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-typeck --lib`.
     - Expected: fails: `check::tests::a_view_with_no_root_or_two_is_e0094`.
  27. **A void element takes a block.** `crates/nova-typeck/src/check/view.rs`:
     `if !takes_children && children.is_some() {` becomes `if false {`.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-typeck --lib`.
     - Expected: fails: `check::tests::a_void_element_with_children_is_e0095`.
  28. **A local names a builder.** `crates/nova-typeck/src/check/view.rs`:
     `if fcx.lookup(n).is_some() {` becomes `if false {`.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-typeck --lib`.
     - Expected: fails: `check::tests::an_element_that_is_no_builder_is_e0096`.
  29. **A variant is no error.** `crates/nova-typeck/src/check/view.rs`:
     `Some(_) => return not_builder(self),` becomes `Some(_) => return None,`.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-typeck --lib`.
     - Expected: fails: `check::tests::an_element_that_is_no_builder_is_e0096`.
  30. **Any two parameters make a builder.** `crates/nova-typeck/src/check/view.rs`:
     `if sig.params == [attrs.clone(), views] {` becomes `if sig.params.len() == 2 {`.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-typeck --lib`.
     - Expected: fails: `check::tests::an_element_that_is_no_builder_is_e0096`.
  31. **Any one parameter makes a void builder.** `crates/nova-typeck/src/check/view.rs`:
     `} else if sig.params == [attrs] {` becomes `} else if sig.params.len() == 1 {`.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-typeck --lib`.
     - Expected: fails: `check::tests::an_element_that_is_no_builder_is_e0096`.
  32. **A builder may return anything.** `crates/nova-typeck/src/check/view.rs`:
     `if sig.generics != 0 || sig.ret != sum(ui.view) {` becomes `if sig.generics != 0 {`.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-typeck --lib`.
     - Expected: fails: `check::tests::an_element_that_is_no_builder_is_e0096`.
  33. **An attribute of any type.** `crates/nova-typeck/src/check/view.rs`:
     replace

     ```
             let (f, ty) = self.reader(fcx, value);
             if !fcx.icx.unify(&ty, &Ty::String) {
     ```

     with

     ```
             let (f, ty) = self.reader(fcx, value);
             if false && !fcx.icx.unify(&ty, &Ty::String) {
     ```

     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-typeck --lib`.
     - Expected: fails: `check::tests::a_child_or_text_of_another_type_is_e0010`, `check::tests::an_attribute_or_handler_of_another_type_is_e0097`.
  34. **Text of any type, reported late.** `crates/nova-typeck/src/check/view.rs`:
     replace

     ```
             let (f, ty) = self.reader(fcx, content);
             if !fcx.icx.unify(&ty, &Ty::String) {
     ```

     with

     ```
             let (f, ty) = self.reader(fcx, content);
             if false && !fcx.icx.unify(&ty, &Ty::String) {
     ```

     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-typeck --lib`.
     - Expected: fails: `check::tests::a_child_or_text_of_another_type_is_e0010`.
  35. **`text` is not indexed.** `crates/nova-typeck/src/check/view.rs`:
     `self.note_use(word.span, Target::Def(ui.text));`: the line goes.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-driver --test index_complete`.
     - Expected: fails: `every_name_in_the_corpora_is_indexed`.
  36. **An element's name is not indexed.** `crates/nova-typeck/src/check/view.rs`:
     `self.note_use(name.span, Target::Def(def_id));`: the line goes.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-driver --test index_complete`.
     - Expected: fails: `every_name_in_the_corpora_is_indexed`.
  37. **`index_complete` checks `view`.** `crates/nova-driver/tests/index_complete.rs`:
     `if name == "view" && matches!(next(1), Some(Token::LBrace)) {` becomes `if false {`.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-driver --test index_complete`.
     - Expected: fails: `every_name_in_the_corpora_is_indexed`.
  38. **`index_complete` checks `component`.** `crates/nova-driver/tests/index_complete.rs`:
     `if name == "component"` becomes `if name == "never"`.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-driver --test index_complete`.
     - Expected: fails: `every_name_in_the_corpora_is_indexed`.
  39. **`index_complete` checks named arguments.** `crates/nova-driver/tests/index_complete.rs`:
     `if in_element_arguments(tokens, i) {` becomes `if false {`.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-driver --test index_complete`.
     - Expected: fails: `every_name_in_the_corpora_is_indexed`.
  40. **No keyword tokens.** `crates/nova-lsp/src/tokens.rs`:
     `for (start, end) in keywords(text, answer.file, defs, index) {` becomes `for (start, end) in keywords(text, answer.file, defs, index).into_iter().filter(|_| false) {`.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-lsp --lib`.
     - Expected: fails: `tokens::tests::component_and_view_are_keywords_only_where_they_are`.
  41. **`component` before any call is a keyword.** `crates/nova-lsp/src/tokens.rs`:
     `&& declares_fn(tokens[i + 1].span)`: the line goes.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-lsp --lib`.
     - Expected: fails: `tokens::tests::component_and_view_are_keywords_only_where_they_are`.
  42. **A variable `view` is a keyword too.** `crates/nova-lsp/src/tokens.rs`:
     replace

     ```
                 "view" => matches!(next(1), Some(Token::LBrace)) && !named(t.span),
     ```

     with

     ```
                 "view" => matches!(next(1), Some(Token::LBrace)),
     ```

     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-lsp --lib`.
     - Expected: fails: `tokens::tests::component_and_view_are_keywords_only_where_they_are`.
  43. **`keyword_start` misses `component`.** `crates/nova-diagnostics/src/lines.rs`:
     `"fn" | "record" | "type" | "trait" | "const" | "component"` becomes `"fn" | "record" | "type" | "trait" | "const"`.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-diagnostics --lib`.
     - Expected: fails: `lines::tests::the_keyword_before_a_name_takes_async_with_it`.
  44. **`nova doc` leaves `component` plain.** `crates/nova-doc/src/signature.rs`:
     `Token::Ident(w) if w == "component"` becomes `Token::Ident(w) if w == "never"`.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-doc --lib`.
     - Expected: fails: `signature::tests::a_component_is_shown_as_it_is_written`.
  45. **The grammar's rules are not included.** `tools/vscode-nova/syntaxes/nova.tmLanguage.json`:
     `{ "include": "#contextual-keywords" },`: the line goes.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test vscode_extension`.
     - Expected: fails: `the_grammar_colours_component_and_view_where_they_are_keywords`.
  46. **Only the root itself is refused.** `crates/nova-cli/src/bundle.rs`:
     `if resolved(root).starts_with(&out) {` becomes `if resolved(root) == out {`.
     - Run `CARGO_INCREMENTAL=0 NOVA_REQUIRE_NODE=1 cargo test -p nova-cli --test wasm_tests --no-fail-fast`.
     - Expected: fails: `dist_may_not_hold_the_project_or_be_inside_public`.
  47. **`..` is not taken away.** `crates/nova-cli/src/bundle.rs`:
     replace

     ```
                 Component::ParentDir => {
                     out.pop();
                 }
                 other => out.push(other),
             }
             if let Ok(real)
     ```

     with

     ```
                 Component::ParentDir => {}
                 other => out.push(other),
             }
             if let Ok(real)
     ```

     - Run `CARGO_INCREMENTAL=0 NOVA_REQUIRE_NODE=1 cargo test -p nova-cli --test wasm_tests --no-fail-fast`.
     - Expected: fails: `dist_may_not_hold_the_project_or_be_inside_public`.
  48. **An output directory inside `public/`.** `crates/nova-cli/src/bundle.rs`:
     `if out.starts_with(resolved(&root.join("public"))) {` becomes `if false {`.
     - Run `CARGO_INCREMENTAL=0 NOVA_REQUIRE_NODE=1 cargo test -p nova-cli --test wasm_tests --no-fail-fast`.
     - Expected: fails: `dist_may_not_hold_the_project_or_be_inside_public`.
  49. **Links in `dist/` are written through.** `crates/nova-cli/src/cmd/run.rs`:
     `bundle.prepare(&output)?;`: the line goes.
     - Run `CARGO_INCREMENTAL=0 NOVA_REQUIRE_NODE=1 cargo test -p nova-cli --test wasm_tests --no-fail-fast`.
     - Expected: fails: `a_projects_own_page_and_public_go_into_dist`.
  50. **The project's page is not copied.** `crates/nova-cli/src/bundle.rs`:
     `if page.is_file() && resolved(&page) != resolved(&to) {` becomes `if false {`.
     - Run `CARGO_INCREMENTAL=0 NOVA_REQUIRE_NODE=1 cargo test -p nova-cli --test wasm_tests --no-fail-fast`.
     - Expected: fails: `a_projects_own_page_and_public_go_into_dist`, `the_counter_spa_counts_clicks`.
  51. **The page is copied onto itself.** `crates/nova-cli/src/bundle.rs`:
     `if page.is_file() && resolved(&page) != resolved(&to) {` becomes `if page.is_file() {`.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --bin nova bundle`.
     - Expected: fails: `bundle::tests::the_page_is_never_copied_onto_itself`.
  52. **The page guard compares paths as written.** `crates/nova-cli/src/bundle.rs`:
     `if page.is_file() && resolved(&page) != resolved(&to) {` becomes `if page.is_file() && page != to {`.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --bin nova bundle`.
     - Expected: fails: `bundle::tests::the_page_is_never_copied_onto_itself`.
  53. **`assets/` is not replaced.** `crates/nova-cli/src/bundle.rs`:
     `remove(&assets)?;`: the line goes.
     - Run `CARGO_INCREMENTAL=0 NOVA_REQUIRE_NODE=1 cargo test -p nova-cli --test wasm_tests --no-fail-fast`.
     - Expected: fails: `a_projects_own_page_and_public_go_into_dist`.
  54. **A `public/` outside the project is copied.** `crates/nova-cli/src/bundle.rs`:
     `if base.starts_with(resolved(root)) {` becomes `if true {`.
     - Run `CARGO_INCREMENTAL=0 NOVA_REQUIRE_NODE=1 cargo test -p nova-cli --test wasm_tests --no-fail-fast`.
     - Expected: fails: `a_public_that_leads_outside_is_not_copied`.
  55. **A link to a directory is copied.** `crates/nova-cli/src/bundle.rs`:
     `} else if kind.is_file() || (kind.is_symlink() && inside()) {` becomes `} else if kind.is_file() || kind.is_symlink() {`.
     - Run `CARGO_INCREMENTAL=0 NOVA_REQUIRE_NODE=1 cargo test -p nova-cli --test wasm_tests --no-fail-fast`.
     - Expected: fails: `a_projects_own_page_and_public_go_into_dist`.
  56. **A link to a file outside `public/` is copied.** `crates/nova-cli/src/bundle.rs`:
     replace

     ```
             let inside = || path.is_file() && resolved(&path).starts_with(public);
     ```

     with

     ```
             let inside = || path.is_file();
     ```

     - Run `CARGO_INCREMENTAL=0 NOVA_REQUIRE_NODE=1 cargo test -p nova-cli --test wasm_tests --no-fail-fast`.
     - Expected: fails: `a_projects_own_page_and_public_go_into_dist`.
  57. **`public/`'s directories are not copied.** `crates/nova-cli/src/bundle.rs`:
     replace

     ```
             if kind.is_dir() {
                 copy_tree(&path, &dest, public)?;
     ```

     with

     ```
             if false {
                 copy_tree(&path, &dest, public)?;
     ```

     - Run `CARGO_INCREMENTAL=0 NOVA_REQUIRE_NODE=1 cargo test -p nova-cli --test wasm_tests --no-fail-fast`.
     - Expected: fails: `a_projects_own_page_and_public_go_into_dist`.
  58. **The manifest leaves out `assets/`.** `crates/nova-cli/src/bundle.rs`:
     `list(&out.join("assets"), "assets", &mut files)?;`: the line goes.
     - Run `CARGO_INCREMENTAL=0 NOVA_REQUIRE_NODE=1 cargo test -p nova-cli --test wasm_tests --no-fail-fast`.
     - Expected: fails: `a_projects_own_page_and_public_go_into_dist`.
  59. **The manifest's version is `null`.** `crates/nova-cli/src/bundle.rs`:
     replace

     ```
             let version = self.version.as_deref().map_or("null".to_string(), quote);
     ```

     with

     ```
             let version = "null".to_string();
     ```

     - Run `CARGO_INCREMENTAL=0 NOVA_REQUIRE_NODE=1 cargo test -p nova-cli --test wasm_tests --no-fail-fast`.
     - Expected: fails: `a_projects_own_page_and_public_go_into_dist`.
  60. **The generated page has no `#app`.** `crates/nova-codegen-wasm/src/shim/index.html`:
     `<div id="app"></div>`: the line goes.
     - Run `CARGO_INCREMENTAL=0 NOVA_REQUIRE_NODE=1 cargo test -p nova-cli --test wasm_tests --no-fail-fast`.
     - Expected: fails: `a_single_file_gets_the_generated_page_and_a_manifest`.
  61. **`quote` escapes no `"`.** `crates/nova-cli/src/bundle.rs`:
     `'"' => out.push_str("\\\""),`: the line goes.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --bin nova bundle`.
     - Expected: fails: `bundle::tests::quote_escapes_what_json_needs`.
  62. **`wasm-opt` runs without `--release`.** `crates/nova-cli/src/bundle.rs`:
     `let wasm_opt = self.release && wasm_opt(out)?;` becomes `let wasm_opt = wasm_opt(out)?;`.
     - Run `CARGO_INCREMENTAL=0 NOVA_REQUIRE_NODE=1 cargo test -p nova-cli --test wasm_tests --no-fail-fast`.
     - Expected: fails: `release_runs_wasm_opt_when_it_is_on_path`.
  63. **A failing `wasm-opt` is ignored.** `crates/nova-cli/src/bundle.rs`:
     `Ok(s) => bail!("wasm-opt failed: {s}"),` becomes `Ok(_) => Ok(false),`.
     - Run `CARGO_INCREMENTAL=0 NOVA_REQUIRE_NODE=1 cargo test -p nova-cli --test wasm_tests --no-fail-fast`.
     - Expected: fails: `release_runs_wasm_opt_when_it_is_on_path`.
  64. **No line when `wasm-opt` is missing.** `crates/nova-cli/src/bundle.rs`:
     replace

     ```
                 eprintln!("wasm-opt not found on PATH; app.wasm is not optimized");
     ```

     with nothing.
     - Run `CARGO_INCREMENTAL=0 NOVA_REQUIRE_NODE=1 cargo test -p nova-cli --test wasm_tests --no-fail-fast`.
     - Expected: fails: `release_runs_wasm_opt_when_it_is_on_path`.
  65. **`wasm-opt` runs without `-Oz`.** `crates/nova-cli/src/bundle.rs`:
     `.args(["-Oz", "app.wasm", "-o", "app.wasm"])` becomes `.args(["-O", "app.wasm", "-o", "app.wasm"])`.
     - Run `CARGO_INCREMENTAL=0 NOVA_REQUIRE_NODE=1 cargo test -p nova-cli --test wasm_tests --no-fail-fast`.
     - Expected: fails: `release_runs_wasm_opt_when_it_is_on_path`.
  66. **`-` adds one.** `examples/06-counter-spa/src/main.nova`:
     `count.update(|n| n - 1)` becomes `count.update(|n| n + 1)`.
     - Run `CARGO_INCREMENTAL=0 NOVA_REQUIRE_NODE=1 cargo test -p nova-cli --test wasm_tests --no-fail-fast`.
     - Expected: fails: `the_counter_spa_counts_clicks`.
  67. **The count is fixed text.** `examples/06-counter-spa/src/main.nova`:
     `text("Count: ${count.value()}")` becomes `text("Count: 2")`.
     - Run `CARGO_INCREMENTAL=0 NOVA_REQUIRE_NODE=1 cargo test -p nova-cli --test wasm_tests --no-fail-fast`.
     - Expected: fails: `the_counter_spa_counts_clicks`.

- [ ] **Step 5: The review's inputs.** `06-counter-spa`'s size, as Task 5
  measured it, and the full run's figures go in the ledger.
- [ ] **Step 6: One fresh read-only review, then one fix pass** (the
  user's workflow).
  - The reviewer reads `git diff $BASE..HEAD` (Task 0) against the spec,
    RFC 0001 and this plan, and is told to hunt for:
    - a view that desugars to other than it says: a literal made
      reactive or a value left fixed, a name not turned into its
      attribute, a handler of the wrong kind accepted;
    - a program that parsed before and now parses otherwise, or a
      formatter output that changes the tree or loses a comment;
    - a child that runs into the next, in the parser or the formatter;
    - an element checked against a function that is not a builder, or
      HIR built from arguments whose types were not checked;
    - a bundle that deletes or overwrites what is not its own, follows a
      link out of `public/`, or runs `wasm-opt` when it should not;
    - a keyword token, a hover or an index entry the server gets wrong;
    - a test that could pass without the code it names;
    - a record that says what the code does not, MSRV 1.78, and tests that
      touch the internet or use temp directories without fixed names.
  - Each finding is fixed in one commit, `4.4: the review's fixes`, or
    answered in the ledger with the reason it stands.
  - After the fixes, Step 1 runs again.
- [ ] **Step 7: The PR.**
  - Push: `git push -u origin claude/loving-archimedes-frtf5q-44`.
  - Open the PR to `main` with the repository's template headings, if it
    has one. The body names no local path and no person.
  - It holds:
    - what 4.4 adds;
    - the gate's evidence: `06-counter-spa` under the fake DOM, as is and
      under stress, and in Chromium in the `browser` job; its size;
      `nova fmt`'s checks over every `.nova` file;
    - the decisions, numbered as here, with 1, 2, 13, 14, 15, 16, 18 and
      20 marked as differing from the spec;
    - that `Cargo.lock` is unchanged;
    - the mutants, and the review's findings and what came of each.
  - Read CI once it ends. Expected: green on all three systems, and the
    `browser` job green; the test job runs the WASM tests under Node 22
    with `NOVA_REQUIRE_NODE=1`.
- [ ] **Step 8: The merge,** only on the user's word.
  - By rebase, pinned to the PR's head commit. Never auto-merge.
  - Then `git fetch origin main` and `git diff <head> origin/main
    --stat`, which prints nothing: the merged tree is the PR's.
