# Phase 4.3, "`std/ui`: reactivity and element builders", Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** a program builds its page from signals and builder calls, and the
page keeps itself up to date. `std/ui` has signals, effects, memos, owners
that dispose of what they made, `batch` and `root`; `set` runs the effects
that read a signal before it returns, each once, after the memos it reads
are current. Views are data that `std/ui/html`'s builders make: `mount`
builds one into the page on WASM, and `render_to_string` writes one as
escaped HTML on either target. A program writes `import ui` and `import
ui::html`, std's first nested import path. The runtime keeps `std/ui`'s
state for the thread, or the page. The gate: `tests/dom/ui_counter.nova`
counts three clicks under the fake DOM in `cargo test` on all three
systems and in Chromium in CI, and `tests/runtime/ui_*.nova`, `30-FRONTEND.md`
§13's unit tests, pass natively and on WASM.

**Architecture:**
- **`nova-resolver`:** three std-only builtins, `ui_state_present`,
  `ui_state_load<T>` and `ui_state_store<T>`; `$std.ui`, `$std.ui.html`
  and `$std.ui.backend` in both targets' std, the backend with a source per
  target; `IMPORTED_STD` gains `ui` and `ui::html`, and a nested path
  resolves there before E0900; `STD_INTERNAL` becomes a list, with the
  backend.
- **`nova-typeck`, `nova-mir`:** the builtins' signatures, generic in the
  caller's first type parameter as `task_output` is, and their `RtFunc`s:
  `nova-runtime` functions natively, `std/rt` functions on WASM.
- **`nova-runtime`:** `ui.rs`, a `thread_local` cell whose record is a
  collector root from its first store.
- **`std/rt`:** state word 104, its three functions, and its mark in
  `gc_collect`.
- **`std/ui`:** new: reactivity, `View` and `Attr`, `show`, `for_each`,
  `render_to_string` and `mount`. **`std/ui/html`:** new: the builders and
  attribute helpers. **`std/ui/backend/{native,wasm}.nova`:** new: the
  page through `std/dom` on WASM, and a stub natively whose every function
  panics with `mount`'s message.
- **`nova-std`, `nova-doc`:** the four files embedded; `std/ui` and
  `std/ui/html` documented, the second as `std/ui/html/`, and the backend
  not.
- **Tests:** `tests/runtime/ui_*`, eight programs; `tests/dom/ui_*`, five
  programs with their harnesses; the `browser` job clicks the builder
  counter too.
- **Records:** ADR 0042, and dated notes.

**Tech Stack:**
- Rust: MSRV 1.78, edition 2021.
- **No new crate; `Cargo.lock` does not change** (spec §10).
- JS for the tests' harnesses: ES modules, run by Node 22, with 4.2's fake
  DOM unchanged.
- Playwright 1.56.1, installed by the CI job only.
- Tests: unit tests in `nova-resolver`, `nova-typeck`, `nova-runtime`,
  `nova-std` and `nova-doc`; `crates/nova-mir/tests/lower_tests.rs`;
  `crates/nova-cli/tests/run_tests.rs`, `wasm_tests.rs` and
  `doc_test.rs`; `crates/nova-driver/tests/index_complete.rs`; programs in
  `tests/runtime/` and `tests/dom/`.

**Spec:** `docs/superpowers/specs/2026-10-17-phase-4-3-std-ui-reactivity-and-views-design.md`,
approved by the user on 2026-10-17.
- Read it before Task 1: it is the authority this plan argues from.
- Its §14 lists the decisions made while writing it, and its §2 the code as
  it stood. This plan's decisions say where the code departs from it.

**Checked on 2026-10-17, before execution:**
- **A prototype built 4.3 whole** on a local branch from `87cb0a8`, in
  this tree, with every test below; a script then split it into one
  commit per task, from its final tree, and each task's tests were split
  from its code. The patches below are those commits.
- **A replay applied them** to `87cb0a8` on a scratch branch, task by task,
  as the steps say (the plan's own commit, Task 0's base, adds only this
  file):
  - every task's tree was the prototype's exactly;
  - each test-first step's failure was seen by applying that task's test
    patch before its code patch, and each step's counts are that run's;
  - after each of Tasks 1 to 4, `cargo fmt`, clippy with `-D warnings` and
    `nova fmt` were clean;
  - `Cargo.lock` did not change.
- **The gate passed in the replay:** `tests/dom/ui_counter.nova` under the
  fake DOM, as is and under `NOVA_GC_STRESS`, and in Chromium through
  Playwright 1.56.1 with `tests/browser/counter.mjs`; the other four DOM
  programs; and `tests/runtime/ui_*` natively and on WASM, as is, and those
  that do not panic under each target's stress.
- **On the last commit, in the replay:**
  - the full suite gave 2286 passed, 1 failed (the IPv6 test) and 10
    ignored, against the baseline's 2258, 1 and 10;
  - clippy with `-D warnings` and both formatters were clean;
  - the builder counter was 16,458 and 4,439 bytes by `gzip -9`
    (`app.wasm` and `app.js`), where 4.2's counter is 10,134 and
    4,143.
- **All 38 mutants** in Task 7 failed their named tests, with
  the results given, run on the replay's last commit.
- **A fact-check read the first draft** against the spec, the prototype
  and the replay's output, with probes of its own. It found three faults
  in the prototype's `std/ui`, each fixed here with a test that fails
  without the fix:
  - Disposing a root could run one of its effects again: a cleanup that
    set a signal the effect still read queued it mid-dispose, and the
    re-run's child effect was never disposed. `clean` now drops the
    owner's subscriptions first, `dispose` marks it before cleaning it,
    and `root`'s dispose function is a batch (decision 23; `ui_cleanup`).
  - `mount` checked no property's name, so `Property("innerHTML", …)`
    put raw HTML in the page, against spec §7.2. Both renderers now check
    every name through `check_element`, which refuses `innerHTML` and
    `outerHTML` (decision 18; `ui_mount_html`,
    `ui_checks_every_name_and_never_parses_html`).
  - The `on_mount` of a render that a `set` during the build replaced
    still ran, for nodes no longer in the page. Each region render has
    its own owner, and a callback whose owner is disposed is dropped
    (decision 24; `ui_mount`).
  - It also found untested code, each now tested: the `disposed` mark,
    attribute names and `mount`'s tag check, a handler's batch and
    untracked read, an effect's first run as a batch, observer order, a
    mount target's existing children, `on_mount`'s order, and the old
    rows' handles (decision 26). All but the last three have mutants
    (21 to 31). And it found figures and records here that were wrong,
    now corrected: the leak's count of calls, the failure texts of Tasks
    2 and 3, `ui_collect`'s description, ADR 0042's escaping and collector
    claims, and the departures it did not list (decisions 8 and 25).
- **A second check read the fixes,** with probes in Chromium and under
  the fake DOM, and found three more faults, fixed here the same way:
  - `srcdoc`, as a property or an attribute, still put HTML in the page
    through an iframe, on both renderers. It is refused with the other
    two, in any case (decision 18; mutants 36 and 37).
  - A waiting `on_mount` ran with whatever owner was current, none after
    a `mount`, so what it made outlived its row. It runs with its own
    owner (decision 24; `ui_mount`, mutant 35).
  - A memo's run was not a batch when a read outside any computed it,
    and a cleanup that read the memo during its own run ran it again.
    The read is a batch, and the memo is marked clean before its last run
    is disposed (decision 23; `ui_memo`, mutants 32 to 34; and mutant 38
    for subscriptions dropped first).
  - It also corrected decision 10's order, mutant 3's output, decision
    23's citation and the wording of what `ui_mount_html` shows.
- **The replay above, and the mutants, were run after both checks' fixes.**
- **Faults found while prototyping, fixed here:**
  - `root(f: fn(fn()))`, as the spec wrote it, left a program no way to
    keep the dispose function: a closure captures by value, so the
    assignment `ui_cleanup` first made changed the closure's copy, and the
    root was never disposed. `root` returns `f`'s result (decision 1).
  - A region's render made its views before it counted as a build, so an
    `on_mount` in a `show` branch that appeared after the mount ran before
    its nodes were in the page. The views are made inside the build, and
    `ui_show_for_each`'s `on_mount` now looks for its own node (mutant 17).
  - In `std/ui/backend/wasm.nova`, a closure that began a line after an
    expression, `| | listener.remove()`, parsed as `|` continuing that
    expression; the closure is bound to a name first.
  - `nova-doc` named the nested module `ui.html`, with the page
    `std/ui.html/index.html`, and documented the backend; and `nova test
    --doc --std` named its doc test's file `std/ui.html/lib.nova`
    (decision 5).
  - Natively a panic aborts `nova run` by a signal, so `run_tests.rs`'s
    panic tests assert a failure, not status 134, as 4.2's do.

## Global Constraints

- **The workspace's minimum Rust stays 1.78,** edition 2021. CI's MSRV job
  runs `cargo check --locked --workspace` with `RUSTFLAGS=-D warnings`. No
  `Option::is_none_or` (1.82), no `std::path::absolute` (1.79).
- **`Cargo.lock` does not change.** Any change to it stops the task: ask
  the user. So does anything that needs a new crate.
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
- **Nothing in `cargo test` or `nova` installs a package.** Playwright is
  the `browser` job's alone; Task 5's local run uses the copy already on
  this machine, and is skipped where there is none.
- **`nova-cli`'s tests need two files beside `target/debug/nova`:**
  `libnova_runtime.a` and the `nova-bench-http` binary. A build of the
  workspace writes both, and `cargo test -p nova-cli` does not: `nova
  build` and `nova test` link the first, so a test of them runs the
  runtime the last workspace build left there. Task 0's build puts both
  there; Task 1 changes `nova-runtime`, so its Step 4 builds the workspace
  again before any `-p nova-cli` run. After a `cargo clean`, do the
  same.
- **std is embedded at build time.** After a task changes a file under
  `std/`, a run of `target/debug/nova` by hand needs `cargo build --locked
  -p nova-cli` first; `cargo test` rebuilds it itself.
- **Work in one tree.** Two worktrees of this repository sharing one
  `target/` share artifact names, so cargo can take one tree's build as
  fresh for the other (4.0's ledger). Never build in another worktree.
- **Nothing is published, and no tag is pushed.** Merge by rebase, on the
  user's word only. Never enable auto-merge.
- **This machine is Linux.** Windows and macOS results come from CI, which
  runs all three systems, Node included.

## Conventions

- **Scratch directory:** `$P` is the session's scratch directory, outside
  the repository. Set it in every shell before a command that names `$P`.
- **The ledger:** `$P/ledger-43x.md`. Every ruling made while executing goes
  there, with its cost if wrong; so do each task's figures.
- **Commits.** Each task's message is written to `$P/msg-43-<task>.txt`:
  the subject the task names, a blank line, and the session's attribution
  lines. Then `git add -A`; `git status --short` lists only the task's
  files; and `git commit -F "$P/msg-43-<task>.txt"`.
- **The baseline's full output** is `$P/base-43.txt` (Task 0).
- **Runs.** Cargo runs with `CARGO_INCREMENTAL=0`, and
  - `W` stands for `NOVA_REQUIRE_NODE=1 cargo test -p nova-cli --test
    wasm_tests -- ui every_runtime each_left_out`;
  - `R` for `cargo test -p nova-cli --test run_tests ui_`;
  - `L` for `cargo test -p nova-mir --test lower_tests`.
- **The patches.** Each task's code is in patch blocks below, each marked
  `<!-- patch NAME -->` and fenced with four backticks. They are the
  prototype's commits, so apply them; do not retype them. From the
  repository's root:

  ```sh
  PLAN=docs/superpowers/plans/2026-10-17-phase-4-3-std-ui-reactivity-and-views.md   # this file, committed
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
  patch; the prototype wrote each from its program's own run, natively for
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

1. **The page's graph freed by the collector** while it is still in use,
   so the next click reads freed memory: `std/ui`'s state is a root, word
   104 on WASM and a `thread_local` root natively →
   `ui_collect` on WASM under `NOVA_GC_STRESS`, the five `ui_*` programs
   that do not panic under native stress (Task 2), the DOM programs under
   stress (Task 4); the state builtins reach `std/rt` on WASM (Task 1);
   mutants 3 to 5.
2. **A computation that runs twice for one `set`, or with a stale input:**
   dirty and check marks, memos brought up to date before a "check"
   effect runs, an effect's first run as a batch, and effects run in the
   order they subscribed → `ui_signal`'s diamond, branch, first run and
   order, `ui_memo`'s counts (Task 2), mutants 6 to 9, 27 and 28.
3. **A cleanup that never runs, or runs in the wrong order,** or a
   computation that runs while it is cleaned or after it is disposed, even
   when a cleanup sets or reads what it read → `ui_cleanup` and `ui_memo`
   (Task 2), and `ui_show_for_each`'s hidden branch (Task 4), mutants 10,
   11, 20, 21, 30, 32 to 34 and 38.
4. **A loop that hangs a page:** an effect that sets what it reads →
   `ui_loop`'s settle limit (Task 2), mutant 12.
5. **Markup a value breaks out of, or raw HTML put in the page:** every
   text and attribute value escaped by `render_to_string`, every tag,
   attribute and property name checked by both renderers, and `innerHTML`,
   `outerHTML` and `srcdoc` refused in any case → `ui_render`,
   `ui_render_bad_name` and `ui_checks_every_name_and_never_parses_html`
   (Task 3), `ui_mount_html` (Task 4), mutants 13, 14, 22 to 24, 36 and
   37.
6. **A region that leaves old nodes behind, or keeps old listeners:** the
   nodes between its markers removed, a disposed render's listeners
   removed by its cleanups → `ui_show_for_each`, whose calls in use fall
   back as rows are rebuilt and whose live handles stay level when the same
   rows are set again (Task 4), mutants 15 and 16.
7. **`on_mount` before the nodes are in the page, for nodes a rebuild
   removed, or making what outlives its row** → `ui_mount`, and
   `ui_show_for_each`'s `on_mount`, which looks for its own node (Task 4),
   and `ui_render`, where it never runs (Task 3); mutants 17, 19, 29, 31
   and 35.
8. **A handler that subscribes the effect that made its event, or runs its
   `set`s one flush each** → `ui_attrs` (Task 4), mutants 25 and 26.
9. **A program's own nested import path accepted, or a std name it can't
   reach:** `ui::html` resolves, `a::b` is still E0900 and `ui::nope`
   E0001; the backend and the state builtins stay std's → the resolver's
   tests and `ui_imports_check_on_both_targets` (Tasks 2 to 4), mutants 1,
   2 and 18.

## Decisions

Made while prototyping, each against the spec. Where one departs from the
spec, it says so and why; ADR 0042 records the departures from
`30-FRONTEND.md`.

1. **`root` returns what its function returns:** `root<T>(f: fn(fn()) ->
   T) -> T`, where the spec wrote `root(f: fn(fn()))`. A closure captures
   by value, so assigning the dispose function to a variable outside `f`
   changes `f`'s copy only, and a program could never call it; returning it
   is how `ui_cleanup` does. SolidJS's `createRoot` does the same.
2. **`Attr` has a fifth variant, `DynProperty(String, fn() -> String)`,**
   for `dyn_value`, which the spec lists but whose `Attr` had no reactive
   property. `render_to_string` writes it, as it writes `Property`, as the
   attribute of the same name.
3. **The backend's functions differ from the spec's list:**
   - `backend_next_sibling` and `backend_is_same` replace `backend_parent`.
     A region removes the nodes between its two markers by walking from
     the first to the second, so a region inside another is removed with
     it, whatever it has rebuilt since; the parent a region builds into is
     the one it was built in, kept by its effect.
   - `backend_listen` returns the function that removes the listener, and
     `build_attr` adds it to the owner's cleanups, so a disposed render
     releases its listeners' closures from `std/rt`'s table (spec §8 says
     they are removed first; it did not say how).
   - The native `Node` holds one unused field, `unused: Int`.
4. **`nova-driver` does not change** (spec §3 and §10 said it would). Its
   loader already skips a nested path, and the resolver's fallback to
   `IMPORTED_STD` finds `ui::html` there. `std_source` stays one lookup by
   name: its one reader, a std doc test's diagnostic, runs natively, and
   the database already holds each target's own text under one name.
5. **`nova-doc` changes, which spec §10 did not list:** a nested std
   module's page is `std/ui/html/index.html`, named `ui/html`, and its
   path is `std::ui::html`; a std doc test names `std/ui/html/lib.nova`;
   and a module in `STD_INTERNAL` has no page, so the backend is not
   documented. `STD_INTERNAL` becomes `pub` for it.
6. **A memo computes once when it is made,** since a `Signal<T>` must hold
   a value and `T` has no default; after that it is lazy, as spec §5.2
   item 4 says.
7. **A new effect's first run is inside a `batch`,** so the signals it sets
   run their effects once it returns, in the same flush, and not inside
   it (spec §5.2 item 5): `ui_signal`'s `p + q` prints twice, not three
   times (mutant 27).
8. **The settle limit counts the queue's entries that run** (those not
   clean when reached): a flush runs 100,000 of them, and the next one
   panics, `std/ui: effects did not settle`. Spec §5.2 item 6's "runs
   effects 100,000 times" is read as the most a flush may run.
9. **A computation is marked clean before its body runs,** so an effect
   that sets a signal it reads queues itself again, and `ui_loop` reaches
   the settle limit instead of ending with a stale value.
10. **A signal's observers keep their order** when one is dropped (an
    order-keeping removal, not a swap), so effects run in the order they
    subscribed; one that runs again subscribes again, after the others.
    `ui_signal`'s first, second and third run in that order after the one
    between them stops reading (mutant 28).
11. **`ui_render_bad_name` is its own program,** where the spec put the
    bad name's panic in `ui_render`, so `ui_render` is a golden program
    that ends normally.
12. **`ui_collect` is added:** an `async fn main` whose sleeps end turns,
    so on WASM under `NOVA_GC_STRESS` the collector runs while the state
    is live; it is the only `tests/runtime` program that collects on WASM.
    Without word 104's mark it traps (mutant 4).
13. **`mount` panicking natively is an inline test** in `run_tests.rs`,
    not a `tests/runtime` program: every program there runs on WASM under
    Node, which has no page, or is left out for a WASM error, and this one
    has neither.
14. **std's modules are listed after the target's others,** `$std.ui`,
    `$std.ui.html` and `$std.ui.backend`: after `$std.process` natively,
    and after `$std.dom` and before `$std.rt` on WASM. None globs into a
    user's module, so the order decides only their indices.
15. **Each segment of a std module's nested path is an occurrence**, `ui`
    of `std/ui` and `html` of `std/ui/html`, so `index_complete` finds every
    name of a program that writes `import ui::html::{…}` indexed.
16. **A nested path whose first segment is a std module's import name,**
    `ui` or `dom`, that names nothing is E0001, "cannot find module
    `ui::nope`"; any other nested path is still E0900.
17. **A handler gets the target's `value` property,** `""` for a button.
    It runs untracked and in a `batch` (spec §5.2 item 8): `ui_attrs`'s
    button that sets two signals prints their sum once, and a handler an
    effect's `click()` runs subscribes nothing to that effect (mutants 25
    and 26).
18. **Names, and no raw HTML:**
    - Both renderers call one function, `check_element`, before they
      write an element: it checks the tag, each attribute's name and each
      property's name, and refuses the three names that parse their value
      as HTML, `innerHTML`, `outerHTML` and `srcdoc` (an iframe's whole
      document), as attributes or properties and in any case, as HTML
      reads an attribute's name (spec §7.2, "no way to insert raw HTML";
      `docs/phase-4-plan.md` decision 11). Its message is `std/ui:
      "innerHTML" parses HTML, which std/ui does not insert`. `ui_mount_html`
      and `ui_checks_every_name_and_never_parses_html` check it (mutants 22
      to 24, 36 and 37).
    - An event's kind is not checked: it is never written as markup, and
      `addEventListener` takes any string.
    - What is code is the program's to place. An attribute's value is
      written as given, escaped by `render_to_string` and set as a value by
      `mount`: what an `onclick` attribute or a `javascript:` URL does is
      the program's, as in a page written by hand, and so is the text of a
      `script` element the program makes, which `mount` builds as a text
      node and the browser runs. None of these is HTML put into the page;
      ADR 0042 says so.
19. **`mount` appends after the target's children,** and keeps its root in
    `UiState` for the page's life: `ui_mount`'s target holds a paragraph
    first.
20. **`show` has no type parameter,** where `30-FRONTEND.md` §4 wrote
    `show<T>`.
21. **`ui_state_store` replaces a stored record's root** natively, though
    `std/ui` stores once: the function keeps the registry balanced either
    way.
22. **One script clicks both counters in CI,** `tests/browser/counter.mjs`,
    since `ui_counter` has 4.2's ids.
23. **Running again or being disposed never runs what it cleans** (spec
    §5.2 item 2):
    - `clean` drops the owner's subscriptions first, then disposes of its
      children, then runs its cleanups, so a cleanup that sets what its own
      effect read does not run it again (`ui_cleanup`, mutant 38).
      SolidJS's `cleanNode` drops sources first too.
    - `run_owner` marks a computation clean before it cleans it, and
      `dispose` marks an owner disposed before it cleans it, so a cleanup
      that reads a memo being run or disposed gets its last value and runs
      nothing (`ui_memo`, mutants 32 and 34). The mark is also what keeps
      a child that its parent's run disposed from running, when the same
      `set` queued both (`ui_cleanup`, mutant 21).
    - `root`'s dispose function, an effect's run and a read that computes
      a memo are each a `batch`, so what a cleanup sets runs once the whole
      clean is done, and never a computation that is being disposed
      (`ui_signal`, `ui_cleanup` and `ui_memo`, mutants 27, 30 and 33).
    - Each list is emptied before it is walked, so a cleanup can neither
      add to it nor take from it.
24. **Each region render has its own owner,** a child of the region's
    effect, made fresh on each run, as spec §5.2 item 2 lists. `f` is read
    with that owner current and the effect as the listener, and its views
    are built in it. An `on_mount` callback that waits records its owner;
    `run_mounted` skips one whose owner is disposed, so a render that a
    `set` during the build replaced runs none of its callbacks, for nodes
    that are gone; and it runs the others with their owner current, so
    what a callback makes is disposed with its render (spec §5.2 item 7;
    `ui_mount`, mutants 29, 31 and 35).
25. **`Signal<T>`'s fields are reachable,** where spec §5.1 says "private
    fields": Nova has no field privacy (`20-STDLIB.md`, 2026-08-14). Writing
    `count.val` changes the value without telling anyone, as changing a
    record read from a signal does; `std/ui`'s module doc says to use `set`
    or `update`. ADR 0042 lists it with the departures.
26. **The old rows' handles are checked by a rebuild of the same rows**
    (spec §9.3): `ui_show_for_each`'s `#again` sets the rows to themselves,
    and under stress the collection after it releases handles and leaves
    as many live as the one before. Counting released handles alone would
    not do: every collection releases some. No mutant in Task 7 leaks the
    old rows' handles without leaking their listeners too, which the calls'
    assertion before it catches; this one states spec §9.3's claim, for a
    leak of nodes alone.

---

### Task 0: The baseline

The branch is at this plan's commit, whose parent is `87cb0a8`, the
approved spec. The plan's commit adds this file alone, so the patches
apply to it as to `87cb0a8`.

- [ ] **Step 1: The tree.**
  - `git status --short` prints nothing, on
    `claude/loving-archimedes-frtf5q-43`.
  - `git diff --stat 87cb0a8 HEAD` lists only
    `docs/superpowers/plans/2026-10-17-phase-4-3-std-ui-reactivity-and-views.md`.
  - `BASE=$(git rev-parse --short HEAD)`: the plan's commit, recorded in
    the ledger. Task 7's review reads from it.
  - `df -h /` shows at least 8G free; if not, remove scratch directories
    of earlier sub-phases first.
- [ ] **Step 2: Node.** `node --version` prints `v22.` and more.
- [ ] **Step 3: The suite.**

  ```sh
  cargo build --locked --workspace
  CARGO_INCREMENTAL=0 NOVA_REQUIRE_NODE=1 cargo test --locked --workspace --no-fail-fast > "$P/base-43.txt" 2>&1
  grep '^test result' "$P/base-43.txt" | awk '{p += $4; f += $6; i += $8} END {print p, f, i}'
  grep ' FAILED$' "$P/base-43.txt"
  ```

  Expected: `2258 1 10`, and one failure, `test
  net::tests::an_ipv6_connect_parks_on_its_first_poll_and_then_establishes
  ... FAILED`. It needs IPv6, which this machine lacks, as in every
  earlier phase.
- [ ] **Step 4: The ledger.** Start `$P/ledger-43x.md`, headed with the
  base commit and the baseline's figures.

### Task 1: `std/ui`'s state, kept for the thread or the page

Spec §4, §8; decision 21.

**Files:**
- Create: `crates/nova-runtime/src/ui.rs`
- Modify:
  - `crates/nova-resolver/src/lib.rs`
  - `crates/nova-typeck/src/check.rs`
  - `crates/nova-mir/src/lib.rs`, `crates/nova-mir/src/lower.rs`
  - `crates/nova-runtime/src/lib.rs`
  - `std/rt/lib.nova`
- Test: `crates/nova-mir/tests/lower_tests.rs`

- [ ] **Step 1: The test.** `patch_block t1-tests`:
  `the_ui_state_builtins_lower_to_the_runtime_or_std_rt`. A stand-in std
  module, passed as `extra_std` as `nova test` passes `std/test`, stores
  and loads a record through generic wrappers, as `std/ui` will. Natively
  each builtin is one `CallRuntime` of its `RtFunc`; on WASM none is, and
  each wrapper calls `std/rt`'s `rt_ui_state_present`, `rt_ui_state_load`
  or `rt_ui_state_store`.
- [ ] **Step 2: Run, and see it fail.** `L`. Expected: the test does not
  build: ``error[E0432]: unresolved imports `nova_mir::RtFunc::UiStateLoad`,
  `nova_mir::RtFunc::UiStatePresent`, `nova_mir::RtFunc::UiStateStore` ``.
- [ ] **Step 3: The code.** `patch_block t1-code`:
  - **`nova-resolver`:** `Builtin::UiStatePresent`, `UiStateLoad` and
    `UiStateStore`, named `ui_state_present`, `ui_state_load` and
    `ui_state_store`, in `STD_ONLY`, which holds 84.
  - **`nova-typeck`:** their signatures, `() -> Bool`, `() -> T` and `(T)`,
    `T` the caller's first type parameter as for `task_output`; the
    builtin table's test names each one's caller.
  - **`nova-mir`:** `RtFunc::UiStatePresent`, `UiStateLoad` and
    `UiStateStore`: symbols `nova_rt_ui_state_*`, signatures `() -> i8`,
    `() -> ptr` and `(ptr) -> unit`, and on WASM `std/rt`'s
    `rt_ui_state_*`; each builtin lowers to its runtime call.
  - **`nova-runtime`:** `ui.rs`: a `thread_local` cell; a store roots the
    new record and releases the root of the one it replaces. Its symbols
    join the JIT's table. Test: `a_stored_state_reads_back_and_is_rooted`.
  - **`std/rt`:** `UI_STATE`, state word 104, its three functions, and
    `gc_collect`'s mark of the record; the state words' comment names it.
- [ ] **Step 4: Run.**
  - `cargo build --locked --workspace`, first: `libnova_runtime.a` must
    hold the three new functions before any later `nova build` or `nova
    test` links it (the global constraints).
  - `L`. Expected: `60 passed`.
  - `CARGO_INCREMENTAL=0 cargo test -p nova-runtime --lib ui`. Expected:
    `8 passed`.
  - `CARGO_INCREMENTAL=0 cargo test -p nova-typeck --lib builtin`.
    Expected: `13 passed`.
  - `CARGO_INCREMENTAL=0 cargo test -p nova-resolver --lib`. Expected:
    `49 passed`.
- [ ] **Step 5: The checks,** as the conventions list them.
- [ ] **Step 6: Commit** `runtime: std/ui's state, kept for the thread or the page`.

**Patch `t1-tests`:**

<!-- patch t1-tests -->

````diff
diff --git a/crates/nova-mir/tests/lower_tests.rs b/crates/nova-mir/tests/lower_tests.rs
index e62379c..907c8a7 100644
--- a/crates/nova-mir/tests/lower_tests.rs
+++ b/crates/nova-mir/tests/lower_tests.rs
@@ -2082,3 +2082,93 @@ fn a_std_extern_js_block_is_wasms_alone() {
         native.diagnostics
     );
 }
+
+/// `std/ui`'s state builtins (spec
+/// `docs/superpowers/specs/2026-10-17-phase-4-3-std-ui-reactivity-and-views-design.md`
+/// §4): natively each is a call of the runtime; on WASM each is a call of
+/// `std/rt`'s function for it, and none is left to the shim. The std module
+/// is a stand-in passed as `extra_std`, as `nova test` passes `std/test`; its
+/// generic wrappers give the builtins their type, as `std/ui`'s do.
+#[test]
+fn the_ui_state_builtins_lower_to_the_runtime_or_std_rt() {
+    use nova_resolver::{resolve_program_for, CompileTarget, Def, DefKind, ModuleSource};
+
+    let stub = "record LowerTestsState { n: Int }\n\
+                fn lower_tests_load<T>() -> T { ui_state_load() }\n\
+                fn lower_tests_store<T>(s: T) { ui_state_store(s) }\n\
+                pub fn lower_tests_state() -> Int {\n\
+                    if !ui_state_present() { lower_tests_store(LowerTestsState { n: 7 }) }\n\
+                    let s: LowerTestsState = lower_tests_load()\n\
+                    s.n\n\
+                }\n";
+    let lower = |target: CompileTarget| {
+        let (tokens, _) = lex(
+            "fn main() { println(\"${lower_tests_state()}\") }",
+            FileId::DUMMY,
+        );
+        let (ast, _) = parse(&tokens, FileId::DUMMY);
+        let ast = ast.expect("no AST");
+        let sources = [ModuleSource::new("main", &ast)];
+        let std_files: Vec<FileId> = target.std_modules().iter().map(|_| FileId::DUMMY).collect();
+        let resolved = resolve_program_for(
+            target,
+            &sources,
+            &std_files,
+            Some((("$test.ui_stub", stub), FileId::DUMMY)),
+        );
+        assert!(
+            resolved.diagnostics.is_empty(),
+            "{:?}",
+            resolved.diagnostics
+        );
+        let checked = check(&resolved.file, &resolved.definitions);
+        assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
+        let defs = &resolved.definitions;
+        let mut impls = nova_mir::RuntimeImpls::default();
+        for f in &checked.module.functions {
+            if let Some(Def {
+                kind: DefKind::Fn { item_index },
+                ..
+            }) = defs.defs().get(f.def_id.0 as usize)
+            {
+                if defs.module_name(defs.module_of(*item_index)) == "$std.rt" {
+                    impls.functions.insert(f.name.clone(), f.def_id);
+                }
+            }
+        }
+        let runtime = (target == CompileTarget::Wasm).then_some(&impls);
+        nova_mir::lower_module_with(&checked.module, &nova_mir::Options { runtime })
+            .expect("lowers")
+    };
+    use nova_mir::RtFunc::{UiStateLoad, UiStatePresent, UiStateStore};
+    let native = lower(CompileTarget::Native);
+    assert_eq!(
+        count_rt(&stmts_of(&native, "lower_tests_state"), UiStatePresent),
+        1
+    );
+    assert_eq!(
+        count_rt(&stmts_of(&native, "lower_tests_load"), UiStateLoad),
+        1
+    );
+    assert_eq!(
+        count_rt(&stmts_of(&native, "lower_tests_store"), UiStateStore),
+        1
+    );
+    let wasm = lower(CompileTarget::Wasm);
+    for (f, rt) in [
+        ("lower_tests_state", "rt_ui_state_present."),
+        ("lower_tests_load", "rt_ui_state_load."),
+        ("lower_tests_store", "rt_ui_state_store."),
+    ] {
+        let stmts = stmts_of(&wasm, f);
+        for want in [UiStatePresent, UiStateLoad, UiStateStore] {
+            assert_eq!(count_rt(&stmts, want), 0, "{f}");
+        }
+        assert!(
+            stmts.iter().any(
+                |s| matches!(s, nova_mir::Stmt::Call { callee, .. } if callee.starts_with(rt))
+            ),
+            "{f} calls {rt}"
+        );
+    }
+}
````

**Patch `t1-code`:**

<!-- patch t1-code -->

````diff
diff --git a/crates/nova-mir/src/lib.rs b/crates/nova-mir/src/lib.rs
index 6d6ff9e..1d0c2c8 100644
--- a/crates/nova-mir/src/lib.rs
+++ b/crates/nova-mir/src/lib.rs
@@ -530,6 +530,13 @@ rt_funcs! {
     ProcessArg,
     /// `(i64 code) -> unit` — `std::process::exit`; never returns.
     ProcessExit,
+    /// `() -> i8` — whether `std/ui` has stored its state for this thread
+    /// (`crates/nova-runtime/src/ui.rs`; spec 4.3 §4).
+    UiStatePresent,
+    /// `() -> ptr` — the state stored, a record.
+    UiStateLoad,
+    /// `(ptr state) -> unit` — keep `state`, a collector root from then on.
+    UiStateStore,
 }
 
 impl RtFunc {
@@ -630,6 +637,9 @@ impl RtFunc {
             RtFunc::ProcessArgCount => "nova_rt_process_arg_count",
             RtFunc::ProcessArg => "nova_rt_process_arg",
             RtFunc::ProcessExit => "nova_rt_process_exit",
+            RtFunc::UiStatePresent => "nova_rt_ui_state_present",
+            RtFunc::UiStateLoad => "nova_rt_ui_state_load",
+            RtFunc::UiStateStore => "nova_rt_ui_state_store",
         }
     }
 
@@ -820,6 +830,9 @@ impl RtFunc {
             RtFunc::ProcessArgCount => (vec![], MirTy::I64),
             RtFunc::ProcessArg => (vec![MirTy::I64], MirTy::Ptr),
             RtFunc::ProcessExit => (vec![MirTy::I64], MirTy::Unit),
+            RtFunc::UiStatePresent => (vec![], MirTy::I8),
+            RtFunc::UiStateLoad => (vec![], MirTy::Ptr),
+            RtFunc::UiStateStore => (vec![MirTy::Ptr], MirTy::Unit),
         }
     }
 }
@@ -934,6 +947,9 @@ impl RtFunc {
             RtFunc::TaskSleepFutureNanos => WasmRuntime::Nova("rt_task_sleep_future_nanos"),
             RtFunc::TaskJoinFuture => WasmRuntime::Nova("rt_task_join_future"),
             RtFunc::TaskTimeoutFuture => WasmRuntime::Nova("rt_task_timeout_future"),
+            RtFunc::UiStatePresent => WasmRuntime::Nova("rt_ui_state_present"),
+            RtFunc::UiStateLoad => WasmRuntime::Nova("rt_ui_state_load"),
+            RtFunc::UiStateStore => WasmRuntime::Nova("rt_ui_state_store"),
             RtFunc::TestSelector
             | RtFunc::FsReadToString
             | RtFunc::FsWriteString
diff --git a/crates/nova-mir/src/lower.rs b/crates/nova-mir/src/lower.rs
index 4fd4339..c30bbf8 100644
--- a/crates/nova-mir/src/lower.rs
+++ b/crates/nova-mir/src/lower.rs
@@ -787,6 +787,9 @@ impl<'a> Lowerer<'a> {
                     Builtin::ProcessArgCount => Lowering::Runtime(RtFunc::ProcessArgCount),
                     Builtin::ProcessArg => Lowering::Runtime(RtFunc::ProcessArg),
                     Builtin::ProcessExit => Lowering::Runtime(RtFunc::ProcessExit),
+                    Builtin::UiStatePresent => Lowering::Runtime(RtFunc::UiStatePresent),
+                    Builtin::UiStateLoad => Lowering::Runtime(RtFunc::UiStateLoad),
+                    Builtin::UiStateStore => Lowering::Runtime(RtFunc::UiStateStore),
                     Builtin::MemLoad8 => Lowering::Mem(MemOp::Load8),
                     Builtin::MemStore8 => Lowering::Mem(MemOp::Store8),
                     Builtin::MemLoad64 => Lowering::Mem(MemOp::Load64),
diff --git a/crates/nova-resolver/src/lib.rs b/crates/nova-resolver/src/lib.rs
index 398507e..d60bc40 100644
--- a/crates/nova-resolver/src/lib.rs
+++ b/crates/nova-resolver/src/lib.rs
@@ -899,6 +899,22 @@ builtins! {
     /// after flushing stdout, and never returns. Runtime symbol
     /// `nova_rt_process_exit`. Std-only.
     ProcessExit,
+    /// `ui_state_present() -> Bool` — whether `std/ui` has stored its state
+    /// for this thread, or this page (spec
+    /// `docs/superpowers/specs/2026-10-17-phase-4-3-std-ui-reactivity-and-views-design.md`
+    /// §4). Runtime symbol `nova_rt_ui_state_present`; on WASM, `std/rt`'s
+    /// `rt_ui_state_present`. Std-only.
+    UiStatePresent,
+    /// `ui_state_load<T>() -> T` — the state [`Builtin::UiStateStore`]
+    /// stored. `T` is the caller's first type parameter, as for
+    /// [`Builtin::TaskOutput`]; `std/ui` calls it with its own record.
+    /// Runtime symbol `nova_rt_ui_state_load`. Std-only.
+    UiStateLoad,
+    /// `ui_state_store<T>(state: T)` — keep `state`, a record, for the
+    /// thread or the page: natively a collector root from then on, on WASM
+    /// in `std/rt`'s state word 104, which the collector marks. Runtime
+    /// symbol `nova_rt_ui_state_store`. Std-only.
+    UiStateStore,
     /// `mem_load8(addr: Int) -> Int` — the byte at `addr`, zero-extended.
     /// `std/rt`, the runtime of a WASM build, reads memory through this and
     /// the eight builtins after it (spec
@@ -1046,6 +1062,9 @@ impl Builtin {
             Builtin::ProcessArgCount => "process_arg_count",
             Builtin::ProcessArg => "process_arg",
             Builtin::ProcessExit => "process_exit",
+            Builtin::UiStatePresent => "ui_state_present",
+            Builtin::UiStateLoad => "ui_state_load",
+            Builtin::UiStateStore => "ui_state_store",
             Builtin::MemLoad8 => "mem_load8",
             Builtin::MemStore8 => "mem_store8",
             Builtin::MemLoad64 => "mem_load64",
@@ -1086,7 +1105,7 @@ impl Builtin {
     /// consecutive review rounds (see the Phase 2.2b whole-branch review),
     /// because the roster is duplicated information that only this array
     /// needs to stay exact.
-    pub const STD_ONLY: [Builtin; 81] = [
+    pub const STD_ONLY: [Builtin; 84] = [
         Builtin::StrCmp,
         Builtin::StrHash,
         Builtin::CharToInt,
@@ -1168,6 +1187,9 @@ impl Builtin {
         Builtin::ProcessArgCount,
         Builtin::ProcessArg,
         Builtin::ProcessExit,
+        Builtin::UiStatePresent,
+        Builtin::UiStateLoad,
+        Builtin::UiStateStore,
     ];
 
     /// Builtins seeded into an std module's scope only when the target is
diff --git a/crates/nova-runtime/src/lib.rs b/crates/nova-runtime/src/lib.rs
index 234cf58..1f4cfb5 100644
--- a/crates/nova-runtime/src/lib.rs
+++ b/crates/nova-runtime/src/lib.rs
@@ -99,6 +99,8 @@ pub mod signal;
 pub mod task;
 /// The monotonic clock behind `std/time`.
 mod time;
+/// The state of `std/ui`, kept for the thread.
+mod ui;
 
 /// A Nova string value: immutable UTF-8, `{ len, ptr }`.
 ///
@@ -1241,6 +1243,18 @@ pub fn symbols() -> Vec<(&'static str, *const u8)> {
             "nova_rt_process_exit",
             process::nova_rt_process_exit as *const u8,
         ),
+        (
+            "nova_rt_ui_state_present",
+            ui::nova_rt_ui_state_present as *const u8,
+        ),
+        (
+            "nova_rt_ui_state_load",
+            ui::nova_rt_ui_state_load as *const u8,
+        ),
+        (
+            "nova_rt_ui_state_store",
+            ui::nova_rt_ui_state_store as *const u8,
+        ),
     ]
 }
 
diff --git a/crates/nova-runtime/src/ui.rs b/crates/nova-runtime/src/ui.rs
new file mode 100644
index 0000000..813e69f
--- /dev/null
+++ b/crates/nova-runtime/src/ui.rs
@@ -0,0 +1,61 @@
+//! The state of `std/ui`: one record per thread, which `std/ui` makes the
+//! first time it needs it (spec
+//! `docs/superpowers/specs/2026-10-17-phase-4-3-std-ui-reactivity-and-views-design.md`
+//! §4). The current computation and owner, the effects waiting to run, and
+//! the roots a program keeps all hang from it, so it is a collector root
+//! from its first store for the rest of the thread's life.
+
+use std::cell::Cell;
+
+use crate::gc;
+
+thread_local! {
+    /// The record's address, or 0 before the first store.
+    static STATE: Cell<usize> = const { Cell::new(0) };
+}
+
+/// `ui_state_present() -> Bool`: whether a state is stored, as `1` or `0`.
+#[no_mangle]
+pub extern "C-unwind" fn nova_rt_ui_state_present() -> i8 {
+    i8::from(STATE.with(Cell::get) != 0)
+}
+
+/// `ui_state_load<T>() -> T`: the record stored, or null before the first
+/// store, which `std/ui` never asks for.
+#[no_mangle]
+pub extern "C-unwind" fn nova_rt_ui_state_load() -> *mut u8 {
+    STATE.with(Cell::get) as *mut u8
+}
+
+/// `ui_state_store<T>(state: T)`: keep `state`, rooting it and releasing the
+/// root of the record it replaces.
+#[no_mangle]
+pub extern "C-unwind" fn nova_rt_ui_state_store(state: *mut u8) {
+    gc::add_root(state);
+    let old = STATE.with(|s| s.replace(state as usize));
+    if old != 0 {
+        gc::remove_root(old as *mut u8);
+    }
+}
+
+#[cfg(test)]
+mod tests {
+    use super::*;
+
+    /// A stored record is a root until another replaces it, and the state
+    /// reads back what was stored.
+    #[test]
+    fn a_stored_state_reads_back_and_is_rooted() {
+        assert_eq!(nova_rt_ui_state_present(), 0);
+        let a = crate::nova_rt_alloc(16);
+        nova_rt_ui_state_store(a);
+        assert_eq!(nova_rt_ui_state_present(), 1);
+        assert_eq!(nova_rt_ui_state_load(), a);
+        assert_eq!(gc::root_count(a as usize), 1);
+        let b = crate::nova_rt_alloc(16);
+        nova_rt_ui_state_store(b);
+        assert_eq!(nova_rt_ui_state_load(), b);
+        assert_eq!(gc::root_count(b as usize), 1);
+        assert_eq!(gc::root_count(a as usize), 0);
+    }
+}
diff --git a/crates/nova-typeck/src/check.rs b/crates/nova-typeck/src/check.rs
index 54d355f..fddb160 100644
--- a/crates/nova-typeck/src/check.rs
+++ b/crates/nova-typeck/src/check.rs
@@ -4267,6 +4267,9 @@ impl<'a> Checker<'a> {
             | Builtin::ProcessArgCount
             | Builtin::ProcessArg
             | Builtin::ProcessExit
+            | Builtin::UiStatePresent
+            | Builtin::UiStateLoad
+            | Builtin::UiStateStore
             | Builtin::MemLoad8
             | Builtin::MemStore8
             | Builtin::MemLoad64
@@ -8142,6 +8145,9 @@ pub(crate) fn builtin_signature(builtin: Builtin) -> (Vec<Ty>, Ty) {
         Builtin::ProcessArgCount => (vec![], Ty::Int),
         Builtin::ProcessArg => (vec![Ty::Int], Ty::String),
         Builtin::ProcessExit => (vec![Ty::Int], Ty::Unit),
+        Builtin::UiStatePresent => (vec![], Ty::Bool),
+        Builtin::UiStateLoad => (vec![], Ty::Param(0)),
+        Builtin::UiStateStore => (vec![Ty::Param(0)], Ty::Unit),
         Builtin::MemLoad8 | Builtin::MemLoad64 => (vec![Ty::Int], Ty::Int),
         Builtin::MemStore8 | Builtin::MemStore64 => (vec![Ty::Int, Ty::Int], Ty::Unit),
         Builtin::MemCopy | Builtin::MemFill => (vec![Ty::Int, Ty::Int, Ty::Int], Ty::Unit),
@@ -16370,6 +16376,18 @@ mod tests {
                     (vec![Ty::Int], Ty::Unit),
                     "`process_exit(code)` in `std/process`'s `exit`",
                 ),
+                Builtin::UiStatePresent => (
+                    (vec![], Ty::Bool),
+                    "`ui_state_present()` in `std/ui`'s `ui`",
+                ),
+                Builtin::UiStateLoad => (
+                    (vec![], Ty::Param(0)),
+                    "`ui_state_load()` in `std/ui`'s `load_state`",
+                ),
+                Builtin::UiStateStore => (
+                    (vec![Ty::Param(0)], Ty::Unit),
+                    "`ui_state_store(state)` in `std/ui`'s `store_state`",
+                ),
                 Builtin::MemLoad8 | Builtin::MemLoad64 => (
                     (vec![Ty::Int], Ty::Int),
                     "`mem_load8(addr)` / `mem_load64(addr)` in `std/rt`",
diff --git a/std/rt/lib.nova b/std/rt/lib.nova
index 170d487..4fb9fae 100644
--- a/std/rt/lib.nova
+++ b/std/rt/lib.nova
@@ -18,10 +18,11 @@
 // target's std sees.
 
 // The runtime's state words, at fixed addresses below the data segment
-// (spec 4.0 §4.1, 4.1 §6.1, 4.2 §4.2, §5.1): Nova has no mutable globals.
-// Each is named where it is used: `HEAP_META`, `STR_SEED`,
+// (spec 4.0 §4.1, 4.1 §6.1, 4.2 §4.2, §5.1, 4.3 §4): Nova has no mutable
+// globals. Each is named where it is used: `HEAP_META`, `STR_SEED`,
 // `STR_SEED_DRAWN`, `LOG_SET`, `LOG_LEVEL`, `EXEC`, `GC_SINCE`, `GC_NEXT`,
-// `GC_COUNT`, `GC_SWITCHES`, `HANDLES` and `CALLS`, at 8 to 96.
+// `GC_COUNT`, `GC_SWITCHES`, `HANDLES`, `CALLS` and `UI_STATE`, at 8 to
+// 104.
 
 // The allocator (spec 4.1 §6.2), ported from `nova-runtime`'s
 // `gc/pages.rs`. Memory from the first 64 KiB boundary at or above
@@ -1514,6 +1515,21 @@ fn rt_call(id: Int, arg: Int, outer: Int) -> Int {
     turn_end(e)
 }
 
+// The state of `std/ui` (spec
+// docs/superpowers/specs/2026-10-17-phase-4-3-std-ui-reactivity-and-views-design.md
+// §4): at state word 104, its record's address, or 0 before the first store.
+// The collector marks the record, and so everything a mounted page keeps.
+const UI_STATE: Int = 104
+
+// `ui_state_present() -> Bool`.
+fn rt_ui_state_present() -> Bool { mem_load64(UI_STATE) != 0 }
+
+// `ui_state_load<T>() -> T`: the record.
+fn rt_ui_state_load() -> Int { mem_load64(UI_STATE) }
+
+// `ui_state_store<T>(state: T)`.
+fn rt_ui_state_store(state: Int) { mem_store64(UI_STATE, state) }
+
 // JS's objects, as `std/dom` holds them (spec
 // docs/superpowers/specs/2026-10-16-phase-4-2-std-dom-and-the-shim-design.md
 // §4): a box is a 16-byte leaf holding a handle of the shim's table, then 0.
@@ -1621,6 +1637,8 @@ fn gc_collect() {
         gc_mark(m, r)
         gc_mark(m, mem_load64(r))
     }
+    let u = mem_load64(UI_STATE)
+    if u != 0 { gc_mark(m, u) }
     let c = mem_load64(CALLS)
     if c != 0 {
         gc_mark(m, c)
````

### Task 2: Reactivity, imported with `import ui`

Spec §3, §4, §5, §9.2; decisions 1, 6 to 10, 12, 14, 23 and 25.

**Files:**
- Create: `std/ui/lib.nova`
- Modify:
  - `crates/nova-resolver/src/lib.rs`
  - `std/lib.rs`
  - `crates/nova-doc/src/model.rs`
- Test:
  - `tests/runtime/ui_signal`, `ui_cleanup`, `ui_memo` and `ui_collect`,
    each `.nova` and `.stdout`; `ui_memo_set.nova` and `ui_loop.nova`
  - `crates/nova-cli/tests/run_tests.rs`, `wasm_tests.rs`, `doc_test.rs`
  - `crates/nova-driver/tests/index_complete.rs`

- [ ] **Step 1: The tests.** `patch_block t2-tests`:
  - **The programs,** each printing what it checks:
    - `ui_signal`: an effect runs again on `set` and on `update`, every
      time; once a branch is not taken, its signal no longer runs the
      effect; `batch` runs it once, after it returns; a diamond's effect
      runs once for each `set` of its source; what an effect's first run
      sets runs once, after it; and effects run in the order they
      subscribed, though one between them stops reading.
    - `ui_cleanup`: `on_cleanup` runs before an effect runs again, its
      children's first, then its own, the last added first; `root`'s
      dispose function disposes of everything, the root's own cleanup
      last, and a disposed effect never runs again. A cleanup that sets
      what its own effect read does not run it again; one that sets the
      signal its effect read runs nothing of a disposed root again, and
      what it sets elsewhere runs once the dispose is done; and a child
      that its parent's run disposed does not run, though the same `set`
      queued both.
    - `ui_memo`: a memo computes when it is made; with two readers it
      computes once per change; one nothing reads computes when it is
      read, not when its input changes; the read that computes it is a
      batch, so a child its run disposes does not run; a cleanup of its
      last run that reads it gets the last value and computes nothing; and
      a disposed memo never computes, though its cleanup reads it after
      its input changed.
    - `ui_collect`: an `async fn main` whose sleeps end turns, a memo,
      an effect in a root, and junk allocated between, so collections run
      while the state is live.
    - `ui_memo_set` and `ui_loop`: `set` on a memo, and an effect that
      sets the signal it reads, each panic.
  - **`run_tests.rs`:** `ui_programs_run_and_survive_gc_stress` runs the
    four natively, as is and under `NOVA_GC_STRESS`;
    `ui_programs_panic_with_their_message` the two panics.
  - **`wasm_tests.rs`:** the four join the gate's programs and its stress
    module, and the two panics `ALSO`, with their tests.
  - **`doc_test.rs`:** `ui` is among the modules whose example runs.
  - **`index_complete.rs`:** 147 programs and 169 files, with the six
    programs and `std/ui`.
- [ ] **Step 2: Run, and see them fail.**
  - `R`. Expected: `0 passed; 2 failed`, as no program builds:
    `ui_programs_run_and_survive_gc_stress` with ``error[E0001]: cannot
    find module `ui` ``, and `ui_programs_panic_with_their_message` at its
    stdout, `left: ""` and `right: "doubled 2\n"`.
  - `W`. Expected: `7 passed; 10 failed`: the gate's six new programs and the
    stress module's four, each at ``error[E0001]: cannot find module `ui` ``.
  - `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test doc_test doc_std`.
    Expected: `0 passed; 1 failed`, `no example of std::ui ran`.
  - `CARGO_INCREMENTAL=0 cargo test -p nova-driver --test index_complete`.
    Expected: `0 passed; 1 failed`: `tests/runtime/ui_cleanup.nova does not
    check cleanly`, with ``E0001 cannot find module `ui` ``.
- [ ] **Step 3: The code.** `patch_block t2-code`:
  - **`std/ui/lib.nova`:** `Owner`, `Source`, `Mounted` and `UiState`;
    `ui()`, which makes the state the first time, through the state
    builtins; `track`, `mark`, `flush`, `refresh`; `run_owner`, which
    marks the computation clean before it cleans it; `drain`; `clean`,
    which drops subscriptions before it runs cleanups; and `dispose`,
    which marks the owner first;
    `Signal<T>` with `value`, whose read of a stale memo is a batch,
    `set` and `update`; `signal`, `effect`,
    `memo`, `on_cleanup`, `on_mount`, `root` and `batch`. Its module
    documentation's example runs as a doc test.
  - **`nova-resolver`:** `$std.ui` in both targets' std, after
    `$std.process` natively and after `$std.dom` on WASM; `IMPORTED_STD`
    gains `("ui", "$std.ui")`. Tests: `ui_is_seen_only_through_an_import`,
    and `a_wasm_build_compiles_wasms_std_modules` lists 13.
  - **`std/lib.rs`:** `UI`, in `ALL`; `every_std_module_is_embedded` walks
    `std/` for every `.nova` file and counts 20, and
    `each_entry_is_its_own_file` checks `std/dom` too.
  - **`nova-doc`:** the std package's modules include `ui`.
- [ ] **Step 4: Run.**
  - `R`. Expected: `2 passed`.
  - `W`. Expected: `17 passed`.
  - The doc test: `1 passed`; the index: `1 passed`.
  - `CARGO_INCREMENTAL=0 cargo test -p nova-resolver -p nova-std -p nova-doc --lib`.
    Expected: `112 passed` in all.
- [ ] **Step 5: The checks,** as the conventions list them, the new
  programs' `nova fmt` included.
- [ ] **Step 6: Commit** `std/ui: reactivity, imported with import ui`.

**Patch `t2-tests`:**

<!-- patch t2-tests -->

````diff
diff --git a/crates/nova-cli/tests/doc_test.rs b/crates/nova-cli/tests/doc_test.rs
index 818c75e..fbeae1d 100644
--- a/crates/nova-cli/tests/doc_test.rs
+++ b/crates/nova-cli/tests/doc_test.rs
@@ -368,7 +368,7 @@ fn a_warning_is_shown_once() {
 fn doc_std_runs_stds_doc_tests_inside_a_project_or_not() {
     // Spec §6.1, §9.2, §13: std's examples run, at least one per module,
     // inside a project or not, and every one passes.
-    const MODULES: [&str; 17] = [
+    const MODULES: [&str; 18] = [
         "bytes",
         "collections",
         "core",
@@ -386,6 +386,7 @@ fn doc_std_runs_stds_doc_tests_inside_a_project_or_not() {
         "task",
         "test",
         "time",
+        "ui",
     ];
     for dir in [fresh("std-outside"), calc("std-inside", LIB)] {
         let a = nova_test(&dir, &["--doc", "--std"]).success();
diff --git a/crates/nova-cli/tests/run_tests.rs b/crates/nova-cli/tests/run_tests.rs
index c0a725a..e532def 100644
--- a/crates/nova-cli/tests/run_tests.rs
+++ b/crates/nova-cli/tests/run_tests.rs
@@ -11304,3 +11304,60 @@ fn example_05_geometry_runs() {
              total area 9.141592653589793\n",
         );
 }
+
+/// `std/ui`'s programs (spec
+/// `docs/superpowers/specs/2026-10-17-phase-4-3-std-ui-reactivity-and-views-design.md`
+/// §9.2): each prints what `30-FRONTEND.md` §13's unit tests check, and runs
+/// the same with `NOVA_GC_STRESS=1` (collect on every allocation), so a part
+/// of the graph only the state's root keeps, freed, shows as wrong output or
+/// a crash.
+#[test]
+fn ui_programs_run_and_survive_gc_stress() {
+    for name in ["ui_signal", "ui_cleanup", "ui_memo", "ui_collect"] {
+        let source = repo_root().join(format!("tests/runtime/{name}.nova"));
+        let expected = std::fs::read_to_string(source.with_extension("stdout"))
+            .expect("expected-output fixture exists")
+            .replace("\r\n", "\n");
+        for stress in [false, true] {
+            let mut cmd = nova();
+            if stress {
+                cmd.env("NOVA_GC_STRESS", "1");
+            }
+            let a = cmd.arg("run").arg(&source).assert().success();
+            let out = String::from_utf8_lossy(&a.get_output().stdout).replace("\r\n", "\n");
+            assert_eq!(out, expected, "{name}, stress {stress}");
+        }
+    }
+}
+
+/// `std/ui`'s panics (spec 4.3 §5): `set` on a memo, and an effect that
+/// never settles, each with what the program printed first.
+#[test]
+fn ui_programs_panic_with_their_message() {
+    for (name, out, message) in [
+        (
+            "ui_memo_set",
+            "doubled 2\n",
+            "nova: panic: std/ui: set() on a memo\n",
+        ),
+        (
+            "ui_loop",
+            "",
+            "nova: panic: std/ui: effects did not settle\n",
+        ),
+    ] {
+        let a = nova()
+            .arg("run")
+            .arg(repo_root().join(format!("tests/runtime/{name}.nova")))
+            .assert()
+            .failure();
+        let o = a.get_output();
+        assert_eq!(
+            String::from_utf8_lossy(&o.stdout).replace("\r\n", "\n"),
+            out,
+            "{name}"
+        );
+        let err = String::from_utf8_lossy(&o.stderr).replace("\r\n", "\n");
+        assert!(err.ends_with(message), "{name}: {err}");
+    }
+}
diff --git a/crates/nova-cli/tests/wasm_tests.rs b/crates/nova-cli/tests/wasm_tests.rs
index 215866d..5fa620a 100644
--- a/crates/nova-cli/tests/wasm_tests.rs
+++ b/crates/nova-cli/tests/wasm_tests.rs
@@ -1536,6 +1536,11 @@ mod gate {
         hash,
         hash_diffusion,
         json_traits,
+        // `std/ui`: reactivity, and its state across turns (spec 4.3 §9.2).
+        ui_cleanup,
+        ui_collect,
+        ui_memo,
+        ui_signal,
     );
 
     /// The programs a test of their own in this module checks, by its name.
@@ -1555,6 +1560,8 @@ mod gate {
         "recycled_task_state",
         "task_deadlock",
         "eprint_family",
+        "ui_memo_set",
+        "ui_loop",
     ];
 
     /// `eprint` and `eprintln` write to stderr, as `run_tests.rs` checks
@@ -1679,6 +1686,24 @@ mod gate {
         );
     }
 
+    #[test]
+    fn ui_memo_set() {
+        super::gate_panics(
+            "ui_memo_set",
+            "nova: panic: std/ui: set() on a memo",
+            |out| out == "doubled 2\n",
+        );
+    }
+
+    #[test]
+    fn ui_loop() {
+        super::gate_panics(
+            "ui_loop",
+            "nova: panic: std/ui: effects did not settle",
+            str::is_empty,
+        );
+    }
+
     #[test]
     fn panic_unwrap() {
         super::gate_panics("panic_unwrap", "nova: panic: called get on None", |out| {
@@ -1777,6 +1802,10 @@ mod stress {
         json_stringify_escapes,
         json_parse_strings,
         bytes_api,
+        ui_cleanup,
+        ui_collect,
+        ui_memo,
+        ui_signal,
     );
 
     #[test]
diff --git a/crates/nova-driver/tests/index_complete.rs b/crates/nova-driver/tests/index_complete.rs
index dbdca73..e2ee26a 100644
--- a/crates/nova-driver/tests/index_complete.rs
+++ b/crates/nova-driver/tests/index_complete.rs
@@ -186,9 +186,9 @@ fn every_name_is_covered(a: &Analysis, file: FileId, failures: &mut Vec<String>)
 #[test]
 fn every_name_in_the_corpora_is_indexed() {
     let programs = programs();
-    // 7 examples, 133 top-level runtime programs and the modules program,
-    // on 2026-10-11.
-    assert_eq!(programs.len(), 141, "{programs:#?}");
+    // 7 examples, 139 top-level runtime programs and the modules program,
+    // on 2026-10-17.
+    assert_eq!(programs.len(), 147, "{programs:#?}");
     let mut files = 0;
     let mut failures: Vec<String> = Vec::new();
     for (k, path) in programs.iter().enumerate() {
@@ -215,9 +215,9 @@ fn every_name_in_the_corpora_is_indexed() {
             every_name_is_covered(&a, file, &mut failures);
         }
     }
-    // 7 + 133 + 3 files of user code, 05-geometry's library's 2, and
-    // std's 17.
-    assert_eq!(files, 162);
+    // 7 + 139 + 3 files of user code, 05-geometry's library's 2, and
+    // native std's 18.
+    assert_eq!(files, 169);
     failures.sort();
     failures.dedup();
     assert!(
diff --git a/tests/runtime/ui_cleanup.nova b/tests/runtime/ui_cleanup.nova
new file mode 100644
index 0000000..b1a35e4
--- /dev/null
+++ b/tests/runtime/ui_cleanup.nova
@@ -0,0 +1,87 @@
+// Effect cleanup (spec 4.3 §9.2): `on_cleanup` runs before an effect runs
+// again and when its owner is disposed, the last added first; a cleanup that
+// sets what its own effect read does not run the effect again; a disposed
+// effect never runs again, though a cleanup sets what it read or the `set`
+// that disposed it queued it too; and `root`'s dispose function, which is a
+// batch.
+import ui
+
+fn main() {
+    let n = signal(1)
+    let stop = root(
+        |dispose: fn()| {
+            effect(
+                | | {
+                    let v = n.value()
+                    println("run ${v}")
+                    on_cleanup(| | println("first cleanup of ${v}"))
+                    on_cleanup(| | println("second cleanup of ${v}"))
+                    effect(
+                        | | {
+                            println("inner of ${v}")
+                            on_cleanup(| | println("inner cleanup of ${v}"))
+                        },
+                    )
+                },
+            )
+            on_cleanup(| | println("root cleanup"))
+            dispose
+        },
+    )
+    n.set(2)
+    println("disposing")
+    stop()
+    n.set(3)
+    println("after: no run")
+
+    // An effect drops what it read before its cleanups run, so one that sets
+    // what the effect read does not run it again.
+    let count = signal(0)
+    effect(
+        | | {
+            println("count is ${count.value()}")
+            on_cleanup(| | count.update(|c: Int| c + 1))
+        },
+    )
+    count.set(10)
+
+    // A cleanup that sets what its effect read runs nothing of the root
+    // again; what it sets elsewhere runs once the dispose is done.
+    let s = signal(0)
+    let elsewhere = signal(0)
+    effect(| | println("elsewhere sees ${elsewhere.value()}"))
+    let stop_again = root(
+        |dispose: fn()| {
+            effect(
+                | | {
+                    let v = s.value()
+                    println("outer of ${v}")
+                    on_cleanup(| | println("last cleanup of ${v}"))
+                    on_cleanup(
+                        | | {
+                            s.set(-1)
+                            elsewhere.set(1)
+                        },
+                    )
+                    effect(| | println("inner of ${v} sees ${s.value()}"))
+                },
+            )
+            dispose
+        },
+    )
+    stop_again()
+    s.set(5)
+    println("after the second dispose: no run")
+
+    // A child its parent's run disposes does not run, though the same `set`
+    // queued both.
+    let a = signal(1)
+    effect(
+        | | {
+            let v = a.value()
+            println("parent of ${v}")
+            effect(| | println("child of ${v} sees ${a.value()}"))
+        },
+    )
+    a.set(2)
+}
diff --git a/tests/runtime/ui_cleanup.stdout b/tests/runtime/ui_cleanup.stdout
new file mode 100644
index 0000000..f438a27
--- /dev/null
+++ b/tests/runtime/ui_cleanup.stdout
@@ -0,0 +1,25 @@
+run 1
+inner of 1
+inner cleanup of 1
+second cleanup of 1
+first cleanup of 1
+run 2
+inner of 2
+disposing
+inner cleanup of 2
+second cleanup of 2
+first cleanup of 2
+root cleanup
+after: no run
+count is 0
+count is 11
+elsewhere sees 0
+outer of 0
+inner of 0 sees 0
+last cleanup of 0
+elsewhere sees 1
+after the second dispose: no run
+parent of 1
+child of 1 sees 1
+parent of 2
+child of 2 sees 2
diff --git a/tests/runtime/ui_collect.nova b/tests/runtime/ui_collect.nova
new file mode 100644
index 0000000..09844cf
--- /dev/null
+++ b/tests/runtime/ui_collect.nova
@@ -0,0 +1,27 @@
+// The state `std/ui` keeps lives across turns, where collections run: a
+// collection between them frees nothing an effect, a memo or a signal still
+// uses (spec 4.3 §8).
+import ui
+
+async fn main() {
+    let log = signal("")
+    let n = signal(1)
+    let doubled = memo(| | n.value() * 2)
+    let stop = root(
+        |dispose: fn()| {
+            effect(| | log.set("n=${n.value()} doubled=${doubled.value()}"))
+            dispose
+        },
+    )
+    let mut i = 0
+    while i < 3 {
+        sleep(Duration::from_millis(1)).await
+        let junk = [0; 1000]
+        n.update(|v: Int| v + 1)
+        println(log.value())
+        i = i + 1
+    }
+    stop()
+    n.set(100)
+    println("after dispose: ${log.value()}")
+}
diff --git a/tests/runtime/ui_collect.stdout b/tests/runtime/ui_collect.stdout
new file mode 100644
index 0000000..1a90418
--- /dev/null
+++ b/tests/runtime/ui_collect.stdout
@@ -0,0 +1,4 @@
+n=2 doubled=4
+n=3 doubled=6
+n=4 doubled=8
+after dispose: n=4 doubled=8
diff --git a/tests/runtime/ui_loop.nova b/tests/runtime/ui_loop.nova
new file mode 100644
index 0000000..fa58ddb
--- /dev/null
+++ b/tests/runtime/ui_loop.nova
@@ -0,0 +1,9 @@
+// An effect that sets the signal it reads never settles, and panics
+// (spec 4.3 §5.2).
+import ui
+
+fn main() {
+    let n = signal(0)
+    effect(| | n.set(n.value() + 1))
+    println("not reached")
+}
diff --git a/tests/runtime/ui_memo.nova b/tests/runtime/ui_memo.nova
new file mode 100644
index 0000000..09a6d58
--- /dev/null
+++ b/tests/runtime/ui_memo.nova
@@ -0,0 +1,106 @@
+// Memo memoization (spec 4.3 §9.2): a memo computes once however many read
+// it, and only when it is read after something it read changed. A read that
+// computes it is a batch; a cleanup of its last run that reads it gets the
+// last value and computes nothing; and a disposed memo never computes.
+import ui
+
+fn main() {
+    let base = signal(2)
+    let computed = signal(0)
+    let squared = memo(
+        | | {
+            computed.update(|n: Int| n + 1)
+            base.value() * base.value()
+        },
+    )
+    println("made: computed ${computed.value()}")
+    effect(| | println("reader one: ${squared.value()}"))
+    effect(| | println("reader two: ${squared.value()}"))
+    println("two readers: computed ${computed.value()}")
+    base.set(3)
+    println("after set: computed ${computed.value()}")
+
+    // Nothing reads `lazy`, so a change does not compute it.
+    let lazy = memo(
+        | | {
+            println("lazy computes")
+            base.value() + 100
+        },
+    )
+    base.set(4)
+    base.set(5)
+    println("read: ${lazy.value()}")
+    println("again: ${lazy.value()}")
+
+    // The read that computes a memo is a batch, so a child the run disposes
+    // does not run, though a sibling's cleanup sets what that child read.
+    let x = signal(0)
+    let input = signal(1)
+    let parent = memo(
+        | | {
+            let v = input.value()
+            effect(| | println("the memo's child of ${v} sees x=${x.value()}"))
+            effect(| | on_cleanup(| | x.update(|n: Int| n + 1)))
+            v
+        },
+    )
+    input.set(2)
+    println("read: ${parent.value()}")
+
+    // A memo is clean before its last run is disposed, so a cleanup of that
+    // run that reads it gets the last value and computes nothing.
+    let none: Option<Signal<Int>> = None
+    let itself = signal(none)
+    let n = signal(1)
+    let tens = memo(
+        | | {
+            let v = n.value()
+            println("tens computes from ${v}")
+            on_cleanup(
+                | | {
+                    match itself.value() {
+                        Some(t) => println("the cleanup of ${v} reads ${t.value()}")
+                        None => {}
+                    }
+                },
+            )
+            v * 10
+        },
+    )
+    itself.set(Some(tens))
+    n.set(2)
+    println("tens: ${tens.value()}")
+
+    // Disposing marks a memo first, so a cleanup that reads it after its
+    // input changed gets the last value, and the memo never computes again.
+    let held = signal(none)
+    let m_in = signal(1)
+    let stop = root(
+        |dispose: fn()| {
+            let hundreds = memo(
+                | | {
+                    let v = m_in.value()
+                    println("hundreds computes from ${v}")
+                    on_cleanup(
+                        | | {
+                            match held.value() {
+                                Some(h) => println("the disposed memo reads ${h.value()}")
+                                None => {}
+                            }
+                        },
+                    )
+                    v * 100
+                },
+            )
+            held.set(Some(hundreds))
+            dispose
+        },
+    )
+    batch(
+        | | {
+            m_in.set(2)
+            stop()
+        },
+    )
+    println("disposed")
+}
diff --git a/tests/runtime/ui_memo.stdout b/tests/runtime/ui_memo.stdout
new file mode 100644
index 0000000..f1e4bfc
--- /dev/null
+++ b/tests/runtime/ui_memo.stdout
@@ -0,0 +1,25 @@
+made: computed 1
+reader one: 4
+reader two: 4
+two readers: computed 1
+reader one: 9
+reader two: 9
+after set: computed 2
+lazy computes
+reader one: 16
+reader two: 16
+reader one: 25
+reader two: 25
+lazy computes
+read: 105
+again: 105
+the memo's child of 1 sees x=0
+the memo's child of 2 sees x=1
+read: 2
+tens computes from 1
+the cleanup of 1 reads 10
+tens computes from 2
+tens: 20
+hundreds computes from 1
+the disposed memo reads 100
+disposed
diff --git a/tests/runtime/ui_memo_set.nova b/tests/runtime/ui_memo_set.nova
new file mode 100644
index 0000000..ff268dd
--- /dev/null
+++ b/tests/runtime/ui_memo_set.nova
@@ -0,0 +1,10 @@
+// `set` on a memo panics (spec 4.3 §5.1).
+import ui
+
+fn main() {
+    let a = signal(1)
+    let doubled = memo(| | a.value() * 2)
+    println("doubled ${doubled.value()}")
+    doubled.set(10)
+    println("not reached")
+}
diff --git a/tests/runtime/ui_signal.nova b/tests/runtime/ui_signal.nova
new file mode 100644
index 0000000..d3f51d2
--- /dev/null
+++ b/tests/runtime/ui_signal.nova
@@ -0,0 +1,82 @@
+// Signal reactivity (spec 4.3 §9.2): an effect runs again on `set` and
+// `update`, drops what a branch it no longer takes read, runs once for a
+// `batch` and for an effect's first run, and once for each `set` of a
+// diamond's source; effects run in the order they subscribed.
+import ui
+
+fn main() {
+    // `set` and `update`.
+    let count = signal(1)
+    let runs = signal(0)
+    effect(
+        | | {
+            println("count is ${count.value()}")
+            runs.update(|n: Int| n + 1)
+        },
+    )
+    count.set(2)
+    count.update(|n: Int| n * 10)
+    count.set(20)
+    println("runs: ${runs.value()}")
+
+    // A branch: once `flag` is false, `a` is no longer read.
+    let flag = signal(true)
+    let a = signal("a1")
+    let b = signal("b1")
+    effect(
+        | | {
+            if flag.value() {
+                println("branch a: ${a.value()}")
+            } else {
+                println("branch b: ${b.value()}")
+            }
+        },
+    )
+    a.set("a2")
+    flag.set(false)
+    a.set("a3")
+    b.set("b2")
+
+    // A batch runs the effect once, after it returns.
+    let x = signal(0)
+    let y = signal(0)
+    effect(| | println("sum: ${x.value() + y.value()}"))
+    batch(
+        | | {
+            x.set(1)
+            y.set(2)
+            println("inside the batch")
+        },
+    )
+
+    // A diamond: `d` reads `b` and `c`, which both read `top`.
+    let top = signal(1)
+    let left = memo(| | top.value() + 1)
+    let right = memo(| | top.value() * 2)
+    effect(| | println("diamond: ${left.value()} ${right.value()}"))
+    top.set(5)
+    top.set(6)
+
+    // An effect's first run is a batch: what it sets runs once, after it.
+    let p = signal(0)
+    let q = signal(0)
+    effect(| | println("p + q: ${p.value() + q.value()}"))
+    effect(
+        | | {
+            p.set(1)
+            q.set(2)
+            println("the first run is done")
+        },
+    )
+
+    // Effects run in the order they subscribed, which one that stops
+    // reading the signal leaves as it was.
+    let o = signal(0)
+    let reading = signal(true)
+    effect(| | println("first sees ${o.value()}"))
+    effect(| | { if reading.value() { println("the dropped one sees ${o.value()}") } })
+    effect(| | println("second sees ${o.value()}"))
+    effect(| | println("third sees ${o.value()}"))
+    reading.set(false)
+    o.set(1)
+}
diff --git a/tests/runtime/ui_signal.stdout b/tests/runtime/ui_signal.stdout
new file mode 100644
index 0000000..e347e24
--- /dev/null
+++ b/tests/runtime/ui_signal.stdout
@@ -0,0 +1,25 @@
+count is 1
+count is 2
+count is 20
+count is 20
+runs: 4
+branch a: a1
+branch a: a2
+branch b: b1
+branch b: b2
+sum: 0
+inside the batch
+sum: 3
+diamond: 2 2
+diamond: 6 10
+diamond: 7 12
+p + q: 0
+the first run is done
+p + q: 3
+first sees 0
+the dropped one sees 0
+second sees 0
+third sees 0
+first sees 1
+second sees 1
+third sees 1
````

**Patch `t2-code`:**

<!-- patch t2-code -->

````diff
diff --git a/crates/nova-doc/src/model.rs b/crates/nova-doc/src/model.rs
index f0d8f19..ad9c45f 100644
--- a/crates/nova-doc/src/model.rs
+++ b/crates/nova-doc/src/model.rs
@@ -1392,7 +1392,8 @@ mod tests {
                 "sync",
                 "task",
                 "test",
-                "time"
+                "time",
+                "ui"
             ]
         );
         let core = module(&d.site, "std/core/index.html");
diff --git a/crates/nova-resolver/src/lib.rs b/crates/nova-resolver/src/lib.rs
index d60bc40..cbf422e 100644
--- a/crates/nova-resolver/src/lib.rs
+++ b/crates/nova-resolver/src/lib.rs
@@ -2076,7 +2076,8 @@ fn missing_names(target: CompileTarget) -> FxHashMap<String, &'static str> {
 }
 
 /// The embedded standard-library modules, compiled as implicit modules and
-/// glob-imported into every user module. Order is significant only in that it
+/// glob-imported into every user module, but for `std/ui`, which a program
+/// imports (spec 4.3 §3). Order is significant only in that it
 /// fixes module indices; user modules always come first, then these in the
 /// order listed here — `std/core` stays first so its module index is
 /// unchanged from when it was the only embedded module. The sources come
@@ -2085,7 +2086,7 @@ fn missing_names(target: CompileTarget) -> FxHashMap<String, &'static str> {
 /// self-contained executable and `cargo package` keeps the files. Each name
 /// is `$std.*`, not a valid identifier, so it can never collide with a user
 /// module name or be named in an `import`.
-pub const STD_MODULES: [(&str, &str); 16] = [
+pub const STD_MODULES: [(&str, &str); 17] = [
     ("$std.core", nova_std::CORE),
     ("$std.bytes", nova_std::BYTES),
     ("$std.io", nova_std::IO),
@@ -2102,6 +2103,7 @@ pub const STD_MODULES: [(&str, &str); 16] = [
     ("$std.http", nova_std::HTTP),
     ("$std.crypto", nova_std::CRYPTO),
     ("$std.process", nova_std::PROCESS),
+    ("$std.ui", nova_std::UI),
 ];
 
 /// What a program is compiled for (spec
@@ -2138,12 +2140,12 @@ impl CompileTarget {
 }
 
 /// The std modules a WASM build compiles (spec 4.0 §3.2, 4.1 §3.1, 4.2
-/// §3.2): the ten of [`STD_MODULES`] that need no operating system, in the
-/// same order, then `std/dom`, which only an `import` reaches, then
-/// `std/rt`, the runtime written in Nova. Their names are those of
+/// §3.2, 4.3 §3): the ten of [`STD_MODULES`] that need no operating
+/// system, in the same order, then `std/dom`, which only an `import`
+/// reaches, then `std/ui`, then `std/rt`, the runtime written in Nova. Their names are those of
 /// [`STD_MODULES`], so `std/core` keeps its place first, and the same order
 /// gives a name the same winner in std's glob imports.
-pub const WASM_STD_MODULES: [(&str, &str); 12] = [
+pub const WASM_STD_MODULES: [(&str, &str); 13] = [
     ("$std.core", nova_std::CORE),
     ("$std.bytes", nova_std::BYTES),
     ("$std.collections", nova_std::COLLECTIONS),
@@ -2155,6 +2157,7 @@ pub const WASM_STD_MODULES: [(&str, &str); 12] = [
     ("$std.json", nova_std::JSON),
     ("$std.log", nova_std::LOG),
     ("$std.dom", nova_std::DOM),
+    ("$std.ui", nova_std::UI),
     ("$std.rt", nova_std::RT),
 ];
 
@@ -2162,7 +2165,7 @@ pub const WASM_STD_MODULES: [(&str, &str); 12] = [
 /// prelude, by the name an `import` writes, and the module's own name
 /// (spec `docs/superpowers/specs/2026-10-16-phase-4-2-std-dom-and-the-shim-design.md`
 /// §3.2).
-pub const IMPORTED_STD: [(&str, &str); 1] = [("dom", "$std.dom")];
+pub const IMPORTED_STD: [(&str, &str); 2] = [("dom", "$std.dom"), ("ui", "$std.ui")];
 
 /// The names a project's module or a dependency may not take: those of
 /// std's imported modules, with 4.3's `ui` (spec 4.2 §3.2).
@@ -3504,8 +3507,9 @@ mod tests {
     }
 
     /// A WASM build compiles its own std (spec 4.0 §3.2, 4.1 §3.1, 4.2
-    /// §3.2): ten of the native modules, in native's order, then `std/dom`
-    /// and `std/rt`, after the user's. A native build has neither.
+    /// §3.2, 4.3 §3): ten of the native modules, in native's order, then
+    /// `std/dom`, `std/ui` and `std/rt`, after the user's. A native build
+    /// has neither `std/dom` nor `std/rt`.
     #[test]
     fn a_wasm_build_compiles_wasms_std_modules() {
         let wasm = resolve_for(CompileTarget::Wasm, "fn main() { }\n");
@@ -3527,6 +3531,7 @@ mod tests {
                 "$std.json",
                 "$std.log",
                 "$std.dom",
+                "$std.ui",
                 "$std.rt"
             ]
         );
@@ -3569,6 +3574,37 @@ mod tests {
         );
     }
 
+    /// `std/ui` globs into no module: a program sees its names only through
+    /// `import ui`, on either target (spec 4.3 §3).
+    #[test]
+    fn ui_is_seen_only_through_an_import() {
+        for target in [CompileTarget::Native, CompileTarget::Wasm] {
+            let bare = resolve_for(target, "fn main() { }\n");
+            assert!(bare.diagnostics.is_empty(), "{:?}", bare.diagnostics);
+            assert!(bare
+                .definitions
+                .resolve_value(ModuleId(0), "signal")
+                .is_none());
+            let listed = resolve_for(target, "import ui::{signal}\nfn main() { }\n");
+            assert!(listed.diagnostics.is_empty(), "{:?}", listed.diagnostics);
+            assert!(listed
+                .definitions
+                .resolve_value(ModuleId(0), "signal")
+                .is_some());
+            assert!(listed
+                .definitions
+                .resolve_value(ModuleId(0), "effect")
+                .is_none());
+            let all = resolve_for(target, "import ui\nfn main() { }\n");
+            assert!(all.diagnostics.is_empty(), "{:?}", all.diagnostics);
+            assert!(all.definitions.resolve_value(ModuleId(0), "memo").is_some());
+            assert!(all
+                .definitions
+                .resolve_type(ModuleId(0), "Signal")
+                .is_some());
+        }
+    }
+
     /// A WASM build knows the module of each public name of the std
     /// modules it does not compile, in every namespace; a native build
     /// compiles them all, and knows none (spec 4.1 §3.2).
diff --git a/std/lib.rs b/std/lib.rs
index e50f682..fc53412 100644
--- a/std/lib.rs
+++ b/std/lib.rs
@@ -2,7 +2,7 @@
 //! build time so the compiler stays one self-contained executable.
 //!
 //! Every `include_str!` path stays inside this package, so `cargo package`
-//! carries all 19 files, and its verification step builds the packaged copy
+//! carries all 20 files, and its verification step builds the packaged copy
 //! on its own (spec
 //! `docs/superpowers/specs/2026-10-07-phase-3-0-foundations-design.md` §3).
 //! `nova-resolver` gives each module its `$std.*` name and its place.
@@ -47,6 +47,8 @@ pub const TASK: &str = include_str!("task/lib.nova");
 pub const TEST: &str = include_str!("test/lib.nova");
 /// `std/time/lib.nova`.
 pub const TIME: &str = include_str!("time/lib.nova");
+/// `std/ui/lib.nova`: reactivity and views, which a program imports.
+pub const UI: &str = include_str!("ui/lib.nova");
 /// `std/rt/lib.nova`, the runtime of a WASM build, written in Nova. Not in
 /// [`ALL`]: only a WASM build compiles it, and its few public items are for
 /// std's other modules alone, so it is not documented (spec
@@ -55,8 +57,8 @@ pub const TIME: &str = include_str!("time/lib.nova");
 pub const RT: &str = include_str!("rt/lib.nova");
 
 /// Every documented module, by its directory under `std/`: all but
-/// [`RT`].
-pub const ALL: [(&str, &str); 17] = [
+/// [`RT`] and [`DOM`].
+pub const ALL: [(&str, &str); 18] = [
     ("bytes", BYTES),
     ("collections", COLLECTIONS),
     ("core", CORE),
@@ -74,6 +76,7 @@ pub const ALL: [(&str, &str); 17] = [
     ("task", TASK),
     ("test", TEST),
     ("time", TIME),
+    ("ui", UI),
 ];
 
 #[cfg(test)]
@@ -81,30 +84,41 @@ mod tests {
     use std::collections::BTreeSet;
     use std::path::Path;
 
-    /// Every `std/*/lib.nova` on disk is embedded, and nothing else is:
-    /// [`super::ALL`]'s, `std/rt`'s as [`super::RT`], and `std/dom`'s as
+    /// Every `.nova` file under `std/` is embedded, and nothing else:
+    /// [`super::ALL`]'s, `std/rt`'s as [`super::RT`], `std/dom`'s as
     /// [`super::DOM`].
     #[test]
     fn every_std_module_is_embedded() {
-        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
-        let on_disk: BTreeSet<String> = std::fs::read_dir(root)
-            .expect("read std/")
-            .map(|entry| entry.expect("read an entry of std/").path())
-            .filter(|path| path.join("lib.nova").is_file())
-            .map(|path| {
-                path.file_name()
-                    .expect("a directory name")
+        fn walk(dir: &Path, under: &str, out: &mut BTreeSet<String>) {
+            for entry in std::fs::read_dir(dir).expect("read a directory of std/") {
+                let path = entry.expect("read an entry of std/").path();
+                let name = path
+                    .file_name()
+                    .expect("a file name")
                     .to_string_lossy()
-                    .into_owned()
-            })
-            .collect();
+                    .into_owned();
+                let rel = if under.is_empty() {
+                    name
+                } else {
+                    format!("{under}/{name}")
+                };
+                if path.is_dir() {
+                    walk(&path, &rel, out);
+                } else if rel.ends_with(".nova") {
+                    out.insert(rel);
+                }
+            }
+        }
+        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
+        let mut on_disk = BTreeSet::new();
+        walk(root, "", &mut on_disk);
         let embedded: BTreeSet<String> = super::ALL
             .iter()
-            .map(|(name, _)| name.to_string())
-            .chain(["rt".to_string(), "dom".to_string()])
+            .map(|(name, _)| format!("{name}/lib.nova"))
+            .chain(["rt/lib.nova", "dom/lib.nova"].map(String::from))
             .collect();
         assert_eq!(embedded, on_disk);
-        assert_eq!(on_disk.len(), 19);
+        assert_eq!(on_disk.len(), 20);
     }
 
     /// Each entry carries its own file, not a neighbour's.
@@ -116,8 +130,9 @@ mod tests {
                 .expect("read a std module");
             assert_eq!(source, on_disk, "std/{name}/lib.nova");
         }
-        let rt = std::fs::read_to_string(root.join("rt").join("lib.nova"))
-            .expect("read std/rt/lib.nova");
-        assert_eq!(super::RT, rt, "std/rt/lib.nova");
+        for (path, source) in [("rt/lib.nova", super::RT), ("dom/lib.nova", super::DOM)] {
+            let on_disk = std::fs::read_to_string(root.join(path)).expect("read a std module");
+            assert_eq!(source, on_disk, "std/{path}");
+        }
     }
 }
diff --git a/std/ui/lib.nova b/std/ui/lib.nova
new file mode 100644
index 0000000..c41bb39
--- /dev/null
+++ b/std/ui/lib.nova
@@ -0,0 +1,457 @@
+//! Reactivity: a signal holds a value, and the effects and memos that read
+//! it run again when it changes.
+//!
+//! It is not in the prelude: a module that uses it imports it, with
+//! `import ui` for every name, or `import ui::{signal, effect}` for some.
+//!
+//! `set` runs the effects that read a signal before it returns, each once,
+//! and only after every memo it reads is current. A memo computes when it is
+//! made, then again when it is read after something it read changed. An
+//! effect, a memo and a `root` own the effects and memos made while they
+//! run: running again or being disposed disposes those first,
+//! then runs the cleanups `on_cleanup` added, the last added first.
+//!
+//! The state behind all of this is the thread's, or the page's on WASM, and
+//! lives as long as it does. A value read from a signal is the signal's own:
+//! changing a record read from one changes the signal without telling
+//! anyone, and `set` or `update` is how to tell.
+//!
+//! A signal, and an effect that reads it:
+//!
+//! ```
+//! import ui
+//!
+//! let count = signal(1)
+//! let seen = signal(0)
+//! effect(| | seen.set(count.value() * 10))
+//! count.set(2)
+//! assert_eq(seen.value(), 20)
+//! ```
+
+// The design (spec
+// docs/superpowers/specs/2026-10-17-phase-4-3-std-ui-reactivity-and-views-design.md
+// §4 to §7). Every computation, root and render is an `Owner`, every signal
+// has a `Source`, and both are records, so each refers to the other: a
+// source to the computations that read it, a computation to the sources it
+// read on its last run. `set` marks what reads the signal dirty and what is
+// downstream of that "check"; a queue of effects then runs, each first
+// bringing the memos it read up to date, and running only if one of them
+// ran or it is dirty itself.
+
+// An owner's kinds.
+const ROOT: Int = 0
+
+const EFFECT: Int = 1
+
+const MEMO: Int = 2
+
+// A computation's states: current, maybe stale, and stale.
+const CLEAN: Int = 0
+
+const CHECK: Int = 1
+
+const DIRTY: Int = 2
+
+// The effect runs one flush may make before it is taken for a loop.
+const SETTLE: Int = 100000
+
+// An effect, a memo, a root, or a region's render.
+record Owner {
+    id: Int,
+    kind: Int,
+    state: Int,
+    disposed: Bool,
+    // What a computation runs: for a memo, it also stores the value.
+    run: fn(),
+    // What it owns: what it made on its last run, and its cleanups.
+    children: Vec<Owner>,
+    cleanups: Vec<fn()>,
+    // What it read on its last run.
+    sources: Vec<Source>,
+    // A memo's own source, which its readers subscribe to.
+    out: Option<Source>,
+}
+
+// What a signal's readers subscribe to.
+record Source {
+    id: Int,
+    observers: Vec<Owner>,
+    // The memo whose value this is.
+    memo: Option<Owner>,
+}
+
+// An `on_mount` callback waiting for its nodes, and the owner it was added
+// in, which is disposed when its nodes are gone before they were in the page.
+record Mounted { owner: Option<Owner>, f: fn() }
+
+// The state of the thread's or the page's reactivity (spec §4): what is
+// running and who owns what it makes, the effects waiting to run, and the
+// roots `mount` keeps.
+record UiState {
+    next_id: Int,
+    listener: Option<Owner>,
+    owner: Option<Owner>,
+    queue: Vec<Owner>,
+    batch: Int,
+    flushing: Bool,
+    roots: Vec<Owner>,
+    // How many builds of nodes are in progress, and how many string
+    // renders: `on_mount` waits for the one and is dropped by the other.
+    building: Int,
+    rendering: Int,
+    mounted: Vec<Mounted>,
+}
+
+fn load_state<T>() -> T { ui_state_load() }
+
+fn store_state<T>(state: T) { ui_state_store(state) }
+
+// The state, made the first time it is asked for.
+fn ui() -> UiState {
+    if !ui_state_present() {
+        store_state(
+            UiState {
+                next_id: 1,
+                listener: None,
+                owner: None,
+                queue: Vec::new(),
+                batch: 0,
+                flushing: false,
+                roots: Vec::new(),
+                building: 0,
+                rendering: 0,
+                mounted: Vec::new(),
+            },
+        )
+    }
+    load_state()
+}
+
+fn new_id() -> Int {
+    let mut s = ui()
+    let id = s.next_id
+    s.next_id = id + 1
+    id
+}
+
+fn new_owner(kind: Int, run: fn()) -> Owner {
+    Owner {
+        id: new_id(),
+        kind: kind,
+        state: CLEAN,
+        disposed: false,
+        run: run,
+        children: Vec::new(),
+        cleanups: Vec::new(),
+        sources: Vec::new(),
+        out: None,
+    }
+}
+
+// Element `i` of `v`, which is in range.
+fn at<T>(v: Vec<T>, i: Int) -> T {
+    match v.get(i) {
+        Some(x) => x
+        None => panic("std/ui: an index out of range")
+    }
+}
+
+// Make `o` a child of the current owner, if there is one.
+fn adopt(o: Owner) {
+    match ui().owner {
+        Some(p) => {
+            let mut kids = p.children
+            kids.push(o)
+        }
+        None => {}
+    }
+}
+
+// Run `f` with `owner` and `listener` current, then put back what was.
+fn with_context<T>(owner: Option<Owner>, listener: Option<Owner>, f: fn() -> T) -> T {
+    let mut s = ui()
+    let was_owner = s.owner
+    let was_listener = s.listener
+    s.owner = owner
+    s.listener = listener
+    let out = f()
+    s.owner = was_owner
+    s.listener = was_listener
+    out
+}
+
+// Run `f` with no listener: what it reads subscribes nothing.
+fn untracked<T>(f: fn() -> T) -> T { with_context(ui().owner, None, f) }
+
+// Subscribe the current listener to `src`, once.
+fn track(src: Source) {
+    match ui().listener {
+        Some(l) => {
+            let mut i = 0
+            while i < l.sources.len() {
+                if at(l.sources, i).id == src.id { return }
+                i = i + 1
+            }
+            let mut read = l.sources
+            read.push(src)
+            let mut obs = src.observers
+            obs.push(l)
+        }
+        None => {}
+    }
+}
+
+// Mark `o` at least `state`; the first mark queues an effect, and marks a
+// memo's readers "check".
+fn mark(o: Owner, state: Int) {
+    if o.disposed || o.state >= state { return }
+    let was = o.state
+    let mut m = o
+    m.state = state
+    if was != CLEAN { return }
+    if o.kind == EFFECT {
+        let mut q = ui().queue
+        q.push(o)
+    } else if o.kind == MEMO {
+        match o.out {
+            Some(src) => mark_all(src, CHECK)
+            None => {}
+        }
+    }
+}
+
+fn mark_all(src: Source, state: Int) {
+    let mut i = 0
+    while i < src.observers.len() {
+        mark(at(src.observers, i), state)
+        i = i + 1
+    }
+}
+
+// Run the queue, unless a batch or another flush will.
+fn flush() {
+    let mut s = ui()
+    if s.flushing || s.batch > 0 { return }
+    s.flushing = true
+    let mut runs = 0
+    let mut i = 0
+    while i < s.queue.len() {
+        let o = at(s.queue, i)
+        i = i + 1
+        if !o.disposed && o.state != CLEAN {
+            runs = runs + 1
+            if runs > SETTLE { panic("std/ui: effects did not settle") }
+            refresh(o)
+        }
+    }
+    let mut q = s.queue
+    q.clear()
+    s.flushing = false
+}
+
+// Bring `o` up to date: a "check" first brings the memos it read up to
+// date, and it runs only if one of them ran or it is dirty.
+fn refresh(o: Owner) {
+    if o.disposed { return }
+    if o.state == CHECK {
+        let mut i = 0
+        while i < o.sources.len() && o.state == CHECK {
+            match at(o.sources, i).memo {
+                Some(m) => refresh(m)
+                None => {}
+            }
+            i = i + 1
+        }
+    }
+    if o.state == DIRTY {
+        run_owner(o)
+    } else {
+        let mut m = o
+        m.state = CLEAN
+    }
+}
+
+// Run computation `o`: mark it clean, so that nothing its last run's
+// cleanups read runs it again, dispose of what that run made, then run it as
+// the listener and the owner. A memo that ran makes its readers dirty, since
+// every value counts as a change.
+fn run_owner(o: Owner) {
+    let mut m = o
+    m.state = CLEAN
+    clean(o)
+    let f = o.run
+    with_context(Some(o), Some(o), f)
+    if o.kind == MEMO {
+        match o.out {
+            Some(src) => mark_all(src, DIRTY)
+            None => {}
+        }
+    }
+}
+
+// The elements of `v`, which is emptied, so that what runs while they are
+// used can neither add to them nor take them away.
+fn drain<T>(v: Vec<T>) -> Vec<T> {
+    let mut out = Vec::new()
+    let mut i = 0
+    while i < v.len() {
+        out.push(at(v, i))
+        i = i + 1
+    }
+    let mut w = v
+    w.clear()
+    out
+}
+
+// Drop `o`'s subscriptions, so that nothing its cleanups set runs it again;
+// then dispose of what it owns, last made first, and run its cleanups, last
+// added first.
+fn clean(o: Owner) {
+    let read = drain(o.sources)
+    let mut k = 0
+    while k < read.len() {
+        drop_observer(at(read, k), o.id)
+        k = k + 1
+    }
+    let kids = drain(o.children)
+    let mut i = kids.len() - 1
+    while i >= 0 {
+        dispose(at(kids, i))
+        i = i - 1
+    }
+    let ups = drain(o.cleanups)
+    let mut j = ups.len() - 1
+    while j >= 0 {
+        let f = at(ups, j)
+        with_context(None, None, f)
+        j = j - 1
+    }
+}
+
+// Mark `o` disposed, so that it never runs again, then clean it.
+fn dispose(o: Owner) {
+    let mut m = o
+    m.disposed = true
+    clean(o)
+}
+
+// Take the computation `id` out of `src`'s observers, keeping their order.
+fn drop_observer(src: Source, id: Int) {
+    let mut obs = src.observers
+    let n = obs.len()
+    let mut i = 0
+    while i < n && at(obs, i).id != id { i = i + 1 }
+    if i == n { return }
+    while i + 1 < n {
+        obs.set(i, at(obs, i + 1))
+        i = i + 1
+    }
+    let last = obs.pop()
+}
+
+/// A value that effects and memos read, and run again when it changes.
+pub record Signal<T> { src: Source, val: T }
+
+impl<T> Signal<T> {
+    /// The value. Read in an effect or a memo, it subscribes that
+    /// computation, which runs again when the value changes.
+    pub fn value(self) -> T {
+        match self.src.memo {
+            Some(m) => { if m.state != CLEAN { batch(| | refresh(m)) } }
+            None => {}
+        }
+        track(self.src)
+        self.val
+    }
+
+    /// Stores `new_value` and runs what reads the signal: before returning,
+    /// or inside `batch` or an effect, once that returns. It does so every
+    /// time, even for an equal value. Calling it on a memo's signal panics.
+    pub fn set(self, new_value: T) {
+        match self.src.memo {
+            Some(m) => panic("std/ui: set() on a memo")
+            None => {}
+        }
+        let mut me = self
+        me.val = new_value
+        mark_all(self.src, DIRTY)
+        flush()
+    }
+
+    /// Sets the value to `f` of the current one, which it reads without
+    /// subscribing.
+    pub fn update(self, f: fn(T) -> T) { self.set(f(self.val)) }
+}
+
+/// A new signal holding `initial`.
+pub fn signal<T>(initial: T) -> Signal<T> {
+    Signal { src: Source { id: new_id(), observers: Vec::new(), memo: None }, val: initial }
+}
+
+/// Runs `f` now, and again each time a signal it read on its last run
+/// changes. The current owner owns it.
+pub fn effect(f: fn()) {
+    let o = new_owner(EFFECT, f)
+    adopt(o)
+    batch(| | run_owner(o))
+}
+
+/// A signal whose value is `f`'s: `f` runs now, then again when it is read
+/// after a signal it read changed. Its `set` panics.
+pub fn memo<T>(f: fn() -> T) -> Signal<T> {
+    let mut o = new_owner(MEMO, | | {})
+    adopt(o)
+    let first = with_context(Some(o), Some(o), f)
+    let src = Source { id: new_id(), observers: Vec::new(), memo: Some(o) }
+    let s = Signal { src: src, val: first }
+    o.run = | | {
+        let mut t = s
+        t.val = f()
+    }
+    o.out = Some(src)
+    s
+}
+
+/// Adds `f` to the current owner's cleanups: it runs when the owner runs
+/// again or is disposed. Outside an owner it never runs.
+pub fn on_cleanup(f: fn()) {
+    match ui().owner {
+        Some(o) => {
+            let mut ups = o.cleanups
+            ups.push(f)
+        }
+        None => {}
+    }
+}
+
+/// Runs `f` once its nodes are in the page: after the `mount`, or the
+/// `show` or `for_each` render, that is building, or at once when none is.
+/// It runs untracked, with the owner it was added in, which owns what it
+/// makes. It never runs under `render_to_string`, nor when its render is
+/// disposed before its nodes are in the page.
+pub fn on_mount(f: fn()) {
+    let s = ui()
+    if s.rendering > 0 { return }
+    if s.building > 0 {
+        let mut waiting = s.mounted
+        waiting.push(Mounted { owner: s.owner, f: f })
+    } else {
+        untracked(f)
+    }
+}
+
+/// Runs `f` with a new owner, which no other owns, and gives `f` the
+/// function that disposes of it, in a `batch`; returns what `f` returns.
+pub fn root<T>(f: fn(fn()) -> T) -> T {
+    let o = new_owner(ROOT, | | {})
+    with_context(Some(o), None, | | f(| | batch(| | dispose(o))))
+}
+
+/// Runs `f`, and the effects its `set`s queue once it returns, each once.
+pub fn batch(f: fn()) {
+    let mut s = ui()
+    s.batch = s.batch + 1
+    f()
+    s.batch = s.batch - 1
+    flush()
+}
````

### Task 3: Views, `std/ui/html` and `render_to_string`

Spec §3, §6, §7.2, §9.2, §9.5; decisions 2, 5, 11, 15, 16, 18 and 20.

**Files:**
- Create: `std/ui/html/lib.nova`
- Modify:
  - `std/ui/lib.nova`
  - `crates/nova-resolver/src/lib.rs`
  - `std/lib.rs`
  - `crates/nova-doc/src/model.rs`, `crates/nova-doc/src/doctest.rs`
- Test:
  - `tests/runtime/ui_render.nova` and `.stdout`,
    `tests/runtime/ui_render_bad_name.nova`
  - `crates/nova-cli/tests/run_tests.rs`, `wasm_tests.rs`, `doc_test.rs`
  - `crates/nova-driver/tests/index_complete.rs`

- [ ] **Step 1: The tests.** `patch_block t3-tests`:
  - **`ui_render`:** nested elements and a void element; escaping, of
    text and of attribute values; dynamic text and attributes read once,
    a handler left out, a property written as an attribute; `show` both
    ways; `for_each` with items and with none; `element("main", …)`; and a
    cleanup that runs when the render is disposed, where `on_mount` never
    does.
  - **`ui_render_bad_name`:** a tag with a hyphen renders; one with a space
    panics, ``std/ui: "p onclick=x" is not a tag name``.
  - **`run_tests.rs`, `wasm_tests.rs`:** the two join Task 2's lists, as
    a golden program and a panic. `ui_checks_every_name_and_never_parses_html`,
    in `run_tests.rs`, renders seven programs, each a fine paragraph and
    then one with a bad name: an attribute's, a dynamic attribute's, a
    property's, `innerHTML`, `outerHTML`, the property `srcdoc` and the
    attribute `SrcDoc`, each panicking with its message. And `ui_imports_check_on_both_targets`, in `wasm_tests.rs`:
    `import ui::html::{div, text}` checks on both targets, `import a::b`
    is E0900, `import ui::nope` E0001, "cannot find module `ui::nope`",
    and `signal` without an import E0001.
  - **`doc_test.rs`:** `std::ui::html`'s example runs, named
    `std/ui/html/lib.nova`.
  - **`index_complete.rs`:** 149 programs and 172 files.
- [ ] **Step 2: Run, and see them fail.**
  - `R`. Expected: `0 passed; 3 failed`, as no program that imports
    `ui::html` builds: `ui_programs_run_and_survive_gc_stress` with
    ``error[E0900]: qualified or nested import paths (`a::b`) are not
    supported yet``; `ui_programs_panic_with_their_message` at its stdout,
    `left: ""` and `right: "<my-tag>fine</my-tag>\n"`; and
    `ui_checks_every_name_and_never_parses_html` at its stdout, `left: ""`
    and `right: "<p class=\"ok\">fine</p>\n"`.
  - `W`. Expected: `17 passed; 4 failed`: `gate::ui_render`,
    `gate::ui_render_bad_name`, `stress::ui_render` and
    `ui_imports_check_on_both_targets`, at ``error[E0900]: qualified or nested
    import paths (`a::b`) are not supported yet``.
  - The doc test: `0 passed; 1 failed`, `no example of std::ui::html ran`; the
    index: `0 passed; 1 failed`: `tests/runtime/ui_render.nova does not check
    cleanly`, at E0900.
- [ ] **Step 3: The code.** `patch_block t3-code`:
  - **`std/ui/lib.nova`:** `View` and `Attr`; `show` and `for_each`, each
    a `Region` whose views are built untracked; `check_name`;
    `check_attr`, which refuses `innerHTML`, `outerHTML` and `srcdoc` in
    any case; and `check_element`, which checks an element's tag and every
    name in its attributes before any is written; `escape` and `render_to_string`,
    which renders in a `root` it then disposes. Its example renders a view
    too.
  - **`std/ui/html/lib.nova`:** `element`, the 35 elements and 4 void
    ones, `text` and `dyn_text`, `attr`, `dyn_attr`, the 8 named
    attributes, `value`, `dyn_value`, `on`, `on_click` and `on_input`.
  - **`nova-resolver`:** `$std.ui.html` in both targets' std;
    `IMPORTED_STD` gains `("ui::html", "$std.ui.html")`; `resolve_import`
    looks a nested path up there before its E0900, and reports one under a
    std import name that names nothing as E0001; each segment of a std
    nested path is an occurrence. Tests:
    `a_nested_import_path_names_only_stds_modules`, and the WASM list of
    14.
  - **`std/lib.rs`:** `UI_HTML`, in `ALL` as `ui/html`; 21 files.
  - **`nova-doc`:** `std_path`: a nested module's page is
    `std/ui/html/index.html`, its name `ui/html` and its path
    `std::ui::html`; `std_place` names its doc tests' file
    `std/ui/html/lib.nova`.
- [ ] **Step 4: Run.**
  - `R`. Expected: `3 passed`.
  - `W`. Expected: `21 passed`.
  - The doc test: `1 passed`; the index: `1 passed`.
  - `CARGO_INCREMENTAL=0 cargo test -p nova-resolver -p nova-std -p nova-doc --lib`.
    Expected: `113 passed` in all.
  - `CARGO_INCREMENTAL=0 cargo test -p nova-doc`. Expected: `64 passed`:
    std's site has a page for `std/ui/html`, and its docs are complete.
- [ ] **Step 5: The checks,** as the conventions list them.
- [ ] **Step 6: Commit** `std/ui: views, std/ui/html, and render_to_string`.

**Patch `t3-tests`:**

<!-- patch t3-tests -->

````diff
diff --git a/crates/nova-cli/tests/doc_test.rs b/crates/nova-cli/tests/doc_test.rs
index fbeae1d..03b3b36 100644
--- a/crates/nova-cli/tests/doc_test.rs
+++ b/crates/nova-cli/tests/doc_test.rs
@@ -409,6 +409,10 @@ fn doc_std_runs_stds_doc_tests_inside_a_project_or_not() {
                 "no example of std::{m} ran:\n{out}"
             );
         }
+        assert!(
+            out.contains("\ntest std::ui::html (std/ui/html/lib.nova:"),
+            "no example of std::ui::html ran:\n{out}"
+        );
         assert!(!out.contains("calc"), "{out}");
         assert_eq!(stderr(&a), "");
     }
diff --git a/crates/nova-cli/tests/run_tests.rs b/crates/nova-cli/tests/run_tests.rs
index e532def..2281c7b 100644
--- a/crates/nova-cli/tests/run_tests.rs
+++ b/crates/nova-cli/tests/run_tests.rs
@@ -11313,7 +11313,13 @@ fn example_05_geometry_runs() {
 /// a crash.
 #[test]
 fn ui_programs_run_and_survive_gc_stress() {
-    for name in ["ui_signal", "ui_cleanup", "ui_memo", "ui_collect"] {
+    for name in [
+        "ui_signal",
+        "ui_cleanup",
+        "ui_memo",
+        "ui_render",
+        "ui_collect",
+    ] {
         let source = repo_root().join(format!("tests/runtime/{name}.nova"));
         let expected = std::fs::read_to_string(source.with_extension("stdout"))
             .expect("expected-output fixture exists")
@@ -11330,8 +11336,9 @@ fn ui_programs_run_and_survive_gc_stress() {
     }
 }
 
-/// `std/ui`'s panics (spec 4.3 §5): `set` on a memo, and an effect that
-/// never settles, each with what the program printed first.
+/// `std/ui`'s panics (spec 4.3 §5, §6.1): `set` on a memo, an effect that
+/// never settles, and a name that is not a tag's, each with what the
+/// program printed first.
 #[test]
 fn ui_programs_panic_with_their_message() {
     for (name, out, message) in [
@@ -11345,6 +11352,11 @@ fn ui_programs_panic_with_their_message() {
             "",
             "nova: panic: std/ui: effects did not settle\n",
         ),
+        (
+            "ui_render_bad_name",
+            "<my-tag>fine</my-tag>\n",
+            "nova: panic: std/ui: \"p onclick=x\" is not a tag name\n",
+        ),
     ] {
         let a = nova()
             .arg("run")
@@ -11361,3 +11373,70 @@ fn ui_programs_panic_with_their_message() {
         assert!(err.ends_with(message), "{name}: {err}");
     }
 }
+
+/// Every name `std/ui` writes is checked: an attribute's, a dynamic one's
+/// and a property's; and a name that parses HTML is refused, as a property
+/// or an attribute, in any case (spec 4.3 §6.1, §7.2). Each program prints a
+/// fine render, then panics at the bad one.
+#[test]
+fn ui_checks_every_name_and_never_parses_html() {
+    let dir = std::env::temp_dir().join("nova-ui-names");
+    let _ = std::fs::remove_dir_all(&dir);
+    std::fs::create_dir_all(&dir).expect("temp dir");
+    for (k, (attrs, message)) in [
+        (
+            "attr(\"on click\", \"x\")",
+            "\"on click\" is not an attribute name",
+        ),
+        (
+            "dyn_attr(\"a=b\", | | \"x\")",
+            "\"a=b\" is not an attribute name",
+        ),
+        ("Property(\"1st\", \"x\")", "\"1st\" is not a property name"),
+        (
+            "Property(\"innerHTML\", \"<b>x</b>\")",
+            "\"innerHTML\" parses HTML, which std/ui does not insert",
+        ),
+        (
+            "DynProperty(\"outerHTML\", | | \"<b>x</b>\")",
+            "\"outerHTML\" parses HTML, which std/ui does not insert",
+        ),
+        (
+            "Property(\"srcdoc\", \"<b>x</b>\")",
+            "\"srcdoc\" parses HTML, which std/ui does not insert",
+        ),
+        (
+            "attr(\"SrcDoc\", \"<b>x</b>\")",
+            "\"SrcDoc\" parses HTML, which std/ui does not insert",
+        ),
+    ]
+    .into_iter()
+    .enumerate()
+    {
+        let file = format!("bad{k}.nova");
+        std::fs::write(
+            dir.join(&file),
+            format!(
+                "import ui\nimport ui::html::{{attr, dyn_attr, p, text}}\n\nfn main() {{\n    println(render_to_string(| | p([attr(\"class\", \"ok\")], [text(\"fine\")])))\n    println(render_to_string(| | p([attr(\"id\", \"x\"), {attrs}], [])))\n    println(\"not reached\")\n}}\n"
+            ),
+        )
+        .expect("write the program");
+        let a = nova()
+            .current_dir(&dir)
+            .arg("run")
+            .arg(&file)
+            .assert()
+            .failure();
+        let o = a.get_output();
+        assert_eq!(
+            String::from_utf8_lossy(&o.stdout).replace("\r\n", "\n"),
+            "<p class=\"ok\">fine</p>\n",
+            "{attrs}"
+        );
+        let err = String::from_utf8_lossy(&o.stderr).replace("\r\n", "\n");
+        assert!(
+            err.ends_with(&format!("nova: panic: std/ui: {message}\n")),
+            "{attrs}: {err}"
+        );
+    }
+}
diff --git a/crates/nova-cli/tests/wasm_tests.rs b/crates/nova-cli/tests/wasm_tests.rs
index 5fa620a..c4bfb18 100644
--- a/crates/nova-cli/tests/wasm_tests.rs
+++ b/crates/nova-cli/tests/wasm_tests.rs
@@ -1536,10 +1536,12 @@ mod gate {
         hash,
         hash_diffusion,
         json_traits,
-        // `std/ui`: reactivity, and its state across turns (spec 4.3 §9.2).
+        // `std/ui`: reactivity, rendering to a string, and its state across
+        // turns (spec 4.3 §9.2).
         ui_cleanup,
         ui_collect,
         ui_memo,
+        ui_render,
         ui_signal,
     );
 
@@ -1562,6 +1564,7 @@ mod gate {
         "eprint_family",
         "ui_memo_set",
         "ui_loop",
+        "ui_render_bad_name",
     ];
 
     /// `eprint` and `eprintln` write to stderr, as `run_tests.rs` checks
@@ -1704,6 +1707,15 @@ mod gate {
         );
     }
 
+    #[test]
+    fn ui_render_bad_name() {
+        super::gate_panics(
+            "ui_render_bad_name",
+            "nova: panic: std/ui: \"p onclick=x\" is not a tag name",
+            |out| out == "<my-tag>fine</my-tag>\n",
+        );
+    }
+
     #[test]
     fn panic_unwrap() {
         super::gate_panics("panic_unwrap", "nova: panic: called get on None", |out| {
@@ -1805,6 +1817,7 @@ mod stress {
         ui_cleanup,
         ui_collect,
         ui_memo,
+        ui_render,
         ui_signal,
     );
 
@@ -1820,6 +1833,41 @@ mod stress {
     }
 }
 
+/// `import ui::html::{div}` checks on both targets; a program's own nested
+/// path is still E0900, `ui::nope` is E0001, and `signal` without an import
+/// is E0001 (spec 4.3 §9.5).
+#[test]
+fn ui_imports_check_on_both_targets() {
+    let dir = fresh("ui-imports");
+    write(
+        &dir,
+        &[
+            (
+                "ok.nova",
+                "import ui\nimport ui::html::{div, text}\n\nfn main() {\n    println(render_to_string(| | div([], [text(\"x\")])))\n}\n",
+            ),
+            ("nested.nova", "import a::b\n\nfn main() {}\n"),
+            ("nope.nova", "import ui::nope\n\nfn main() {}\n"),
+            ("bare.nova", "fn main() {\n    let s = signal(0)\n}\n"),
+        ],
+    );
+    for target in ["native", "wasm"] {
+        nova_in(&dir, &["check", "--target", target, "ok.nova"]).success();
+        for (file, code, says) in [
+            ("nested.nova", "E0900", "qualified or nested import paths"),
+            ("nope.nova", "E0001", "cannot find module `ui::nope`"),
+            ("bare.nova", "E0001", "cannot find function `signal`"),
+        ] {
+            let a = nova_in(&dir, &["check", "--target", target, file]).failure();
+            let err = stderr(&a);
+            assert!(
+                err.contains(&format!("error[{code}]")) && err.contains(says),
+                "{target} {file}: {err}"
+            );
+        }
+    }
+}
+
 /// The `tests/runtime/` programs WASM does not run, each with its reason:
 /// the native-only std modules it names, each E0092 on WASM, or the error
 /// that is why it never can (spec 4.1 §9.2).
diff --git a/crates/nova-driver/tests/index_complete.rs b/crates/nova-driver/tests/index_complete.rs
index e2ee26a..63e1612 100644
--- a/crates/nova-driver/tests/index_complete.rs
+++ b/crates/nova-driver/tests/index_complete.rs
@@ -186,9 +186,9 @@ fn every_name_is_covered(a: &Analysis, file: FileId, failures: &mut Vec<String>)
 #[test]
 fn every_name_in_the_corpora_is_indexed() {
     let programs = programs();
-    // 7 examples, 139 top-level runtime programs and the modules program,
+    // 7 examples, 141 top-level runtime programs and the modules program,
     // on 2026-10-17.
-    assert_eq!(programs.len(), 147, "{programs:#?}");
+    assert_eq!(programs.len(), 149, "{programs:#?}");
     let mut files = 0;
     let mut failures: Vec<String> = Vec::new();
     for (k, path) in programs.iter().enumerate() {
@@ -215,9 +215,9 @@ fn every_name_in_the_corpora_is_indexed() {
             every_name_is_covered(&a, file, &mut failures);
         }
     }
-    // 7 + 139 + 3 files of user code, 05-geometry's library's 2, and
-    // native std's 18.
-    assert_eq!(files, 169);
+    // 7 + 141 + 3 files of user code, 05-geometry's library's 2, and
+    // native std's 19.
+    assert_eq!(files, 172);
     failures.sort();
     failures.dedup();
     assert!(
diff --git a/tests/runtime/ui_render.nova b/tests/runtime/ui_render.nova
new file mode 100644
index 0000000..a119a47
--- /dev/null
+++ b/tests/runtime/ui_render.nova
@@ -0,0 +1,103 @@
+// `render_to_string` (spec 4.3 §7.2): nested elements, static and dynamic
+// attributes and properties, `show` and `for_each` both ways, escaping,
+// void elements, and handlers left out.
+import ui
+import ui::html::{
+    attr,
+    button,
+    class,
+    div,
+    dyn_attr,
+    dyn_text,
+    element,
+    h1,
+    img,
+    input,
+    li,
+    on_click,
+    p,
+    span,
+    src,
+    text,
+    ul,
+    value,
+}
+
+record Item { name: String, done: Bool }
+
+fn main() {
+    // Nested elements, attributes, and a void element.
+    println(
+        render_to_string(
+            | | div(
+                [class("card"), attr("data-id", "7")],
+                [
+                    h1([], [text("Title")]),
+                    img([src("a.png")]),
+                    p([], [span([], [text("one")]), text(" two")]),
+                ],
+            ),
+        ),
+    )
+
+    // Escaping: text escapes `&`, `<` and `>`; attributes `"` too.
+    println(
+        render_to_string(
+            | | p([attr("title", "say \"hi\" & <go>")], [text("1 < 2 & 3 > 2 \"q\"")]),
+        ),
+    )
+
+    // Dynamic text and attributes are read once; handlers are left out;
+    // a property is written as the attribute.
+    let n = signal(3)
+    let html = render_to_string(
+        | | div(
+            [dyn_attr("class", | | "n${n.value()}")],
+            [
+                dyn_text(| | "n is ${n.value()}"),
+                button([on_click(| | n.set(0))], [text("reset")]),
+                input([value("x\"y"), attr("type", "text")]),
+            ],
+        ),
+    )
+    println(html)
+
+    // `show`, both ways.
+    let open = signal(true)
+    let panel = | | div([], [show(open, | | p([], [text("open")])), text("|")])
+    println(render_to_string(panel))
+    open.set(false)
+    println(render_to_string(panel))
+
+    // `for_each`, with items and with none.
+    let items = signal(
+        [Item { name: "milk", done: true }, Item { name: "eggs & ham", done: false }],
+    )
+    let list = | | ul(
+        [],
+        [
+            for_each(
+                items,
+                |it: Item| li([class(if it.done { "done" } else { "todo" })], [text(it.name)]),
+            ),
+        ],
+    )
+    println(render_to_string(list))
+    let none: [Item] = []
+    items.set(none)
+    println(render_to_string(list))
+
+    // `element` writes a tag with no builder of its own.
+    println(render_to_string(| | element("main", [], [text("body")])))
+
+    // A cleanup in the render runs when it is disposed; `on_mount` never.
+    println(
+        render_to_string(
+            | | {
+                on_cleanup(| | println("cleanup ran"))
+                on_mount(| | println("on_mount ran"))
+                text("done")
+            },
+        ),
+    )
+}
diff --git a/tests/runtime/ui_render.stdout b/tests/runtime/ui_render.stdout
new file mode 100644
index 0000000..0ffe2b3
--- /dev/null
+++ b/tests/runtime/ui_render.stdout
@@ -0,0 +1,10 @@
+<div class="card" data-id="7"><h1>Title</h1><img src="a.png"><p><span>one</span> two</p></div>
+<p title="say &quot;hi&quot; &amp; &lt;go&gt;">1 &lt; 2 &amp; 3 &gt; 2 "q"</p>
+<div class="n3">n is 3<button>reset</button><input value="x&quot;y" type="text"></div>
+<div><p>open</p>|</div>
+<div>|</div>
+<ul><li class="done">milk</li><li class="todo">eggs &amp; ham</li></ul>
+<ul></ul>
+<main>body</main>
+cleanup ran
+done
diff --git a/tests/runtime/ui_render_bad_name.nova b/tests/runtime/ui_render_bad_name.nova
new file mode 100644
index 0000000..b1628d4
--- /dev/null
+++ b/tests/runtime/ui_render_bad_name.nova
@@ -0,0 +1,10 @@
+// A tag name that is not ASCII letters, digits and `-`, starting with a
+// letter, panics (spec 4.3 §6.1).
+import ui
+import ui::html::{element, text}
+
+fn main() {
+    println(render_to_string(| | element("my-tag", [], [text("fine")])))
+    println(render_to_string(| | element("p onclick=x", [], [])))
+    println("not reached")
+}
````

**Patch `t3-code`:**

<!-- patch t3-code -->

````diff
diff --git a/crates/nova-doc/src/doctest.rs b/crates/nova-doc/src/doctest.rs
index 811b37b..945580c 100644
--- a/crates/nova-doc/src/doctest.rs
+++ b/crates/nova-doc/src/doctest.rs
@@ -104,10 +104,11 @@ pub fn std_doc_tests() -> Result<Vec<DocTest>, Failure> {
 }
 
 /// A std module's file as a doc test's name shows it: `<std/time>` is
-/// `std/time/lib.nova` (spec §5.3, §7).
+/// `std/time/lib.nova`, and `<std/ui.html>` `std/ui/html/lib.nova` (spec
+/// §5.3, §7; spec 4.3 §3).
 fn std_place(name: &str) -> String {
     match name.strip_prefix("<std/").and_then(|n| n.strip_suffix('>')) {
-        Some(m) => format!("std/{m}/lib.nova"),
+        Some(m) => format!("std/{}/lib.nova", m.replace('.', "/")),
         None => name.to_string(),
     }
 }
diff --git a/crates/nova-doc/src/model.rs b/crates/nova-doc/src/model.rs
index ad9c45f..df6c355 100644
--- a/crates/nova-doc/src/model.rs
+++ b/crates/nova-doc/src/model.rs
@@ -499,6 +499,12 @@ impl<'a> Ctx<'a> {
         self.defs.module_name(m).strip_prefix("$std.")
     }
 
+    /// A std module's path under `std/`, as `ui/html` for `$std.ui.html`
+    /// (spec 4.3 §3).
+    fn std_path(&self, m: ModuleId) -> Option<String> {
+        Some(self.std_name(m)?.replace('.', "/"))
+    }
+
     fn module_file(&self, m: ModuleId) -> Option<FileId> {
         let i = m.0 as usize;
         if i < self.users {
@@ -530,7 +536,7 @@ impl<'a> Ctx<'a> {
     fn page(&self, m: ModuleId) -> Option<String> {
         let i = m.0 as usize;
         if i >= self.users {
-            return Some(format!("std/{}/index.html", self.std_name(m)?));
+            return Some(format!("std/{}/index.html", self.std_path(m)?));
         }
         if self.std {
             return None;
@@ -693,8 +699,8 @@ impl<'a> Ctx<'a> {
         let page = self.page(m)?;
         let ft = self.texts.get(&self.module_file(m)?)?;
         let (name, qual, root) = if m.0 as usize >= self.users {
-            let n = self.std_name(m)?.to_string();
-            let qual = format!("std::{n}");
+            let n = self.std_path(m)?;
+            let qual = format!("std::{}", n.replace('/', "::"));
             (n, qual, false)
         } else {
             let stem = self
@@ -1393,7 +1399,8 @@ mod tests {
                 "task",
                 "test",
                 "time",
-                "ui"
+                "ui",
+                "ui/html"
             ]
         );
         let core = module(&d.site, "std/core/index.html");
diff --git a/crates/nova-resolver/src/lib.rs b/crates/nova-resolver/src/lib.rs
index cbf422e..e9e0685 100644
--- a/crates/nova-resolver/src/lib.rs
+++ b/crates/nova-resolver/src/lib.rs
@@ -2076,8 +2076,8 @@ fn missing_names(target: CompileTarget) -> FxHashMap<String, &'static str> {
 }
 
 /// The embedded standard-library modules, compiled as implicit modules and
-/// glob-imported into every user module, but for `std/ui`, which a program
-/// imports (spec 4.3 §3). Order is significant only in that it
+/// glob-imported into every user module, but for `std/ui` and `std/ui/html`,
+/// which a program imports (spec 4.3 §3). Order is significant only in that it
 /// fixes module indices; user modules always come first, then these in the
 /// order listed here — `std/core` stays first so its module index is
 /// unchanged from when it was the only embedded module. The sources come
@@ -2086,7 +2086,7 @@ fn missing_names(target: CompileTarget) -> FxHashMap<String, &'static str> {
 /// self-contained executable and `cargo package` keeps the files. Each name
 /// is `$std.*`, not a valid identifier, so it can never collide with a user
 /// module name or be named in an `import`.
-pub const STD_MODULES: [(&str, &str); 17] = [
+pub const STD_MODULES: [(&str, &str); 18] = [
     ("$std.core", nova_std::CORE),
     ("$std.bytes", nova_std::BYTES),
     ("$std.io", nova_std::IO),
@@ -2104,6 +2104,7 @@ pub const STD_MODULES: [(&str, &str); 17] = [
     ("$std.crypto", nova_std::CRYPTO),
     ("$std.process", nova_std::PROCESS),
     ("$std.ui", nova_std::UI),
+    ("$std.ui.html", nova_std::UI_HTML),
 ];
 
 /// What a program is compiled for (spec
@@ -2142,10 +2143,11 @@ impl CompileTarget {
 /// The std modules a WASM build compiles (spec 4.0 §3.2, 4.1 §3.1, 4.2
 /// §3.2, 4.3 §3): the ten of [`STD_MODULES`] that need no operating
 /// system, in the same order, then `std/dom`, which only an `import`
-/// reaches, then `std/ui`, then `std/rt`, the runtime written in Nova. Their names are those of
+/// reaches, then `std/ui` and `std/ui/html`, then `std/rt`, the runtime
+/// written in Nova. Their names are those of
 /// [`STD_MODULES`], so `std/core` keeps its place first, and the same order
 /// gives a name the same winner in std's glob imports.
-pub const WASM_STD_MODULES: [(&str, &str); 13] = [
+pub const WASM_STD_MODULES: [(&str, &str); 14] = [
     ("$std.core", nova_std::CORE),
     ("$std.bytes", nova_std::BYTES),
     ("$std.collections", nova_std::COLLECTIONS),
@@ -2158,6 +2160,7 @@ pub const WASM_STD_MODULES: [(&str, &str); 13] = [
     ("$std.log", nova_std::LOG),
     ("$std.dom", nova_std::DOM),
     ("$std.ui", nova_std::UI),
+    ("$std.ui.html", nova_std::UI_HTML),
     ("$std.rt", nova_std::RT),
 ];
 
@@ -2165,7 +2168,11 @@ pub const WASM_STD_MODULES: [(&str, &str); 13] = [
 /// prelude, by the name an `import` writes, and the module's own name
 /// (spec `docs/superpowers/specs/2026-10-16-phase-4-2-std-dom-and-the-shim-design.md`
 /// §3.2).
-pub const IMPORTED_STD: [(&str, &str); 2] = [("dom", "$std.dom"), ("ui", "$std.ui")];
+pub const IMPORTED_STD: [(&str, &str); 3] = [
+    ("dom", "$std.dom"),
+    ("ui", "$std.ui"),
+    ("ui::html", "$std.ui.html"),
+];
 
 /// The names a project's module or a dependency may not take: those of
 /// std's imported modules, with 4.3's `ui` (spec 4.2 §3.2).
@@ -2791,24 +2798,33 @@ fn resolve_import(
     public_fixes: &mut Vec<(usize, usize, String)>,
 ) {
     let span = imp.path.span;
-    // Only single-segment module imports are supported. A qualified or nested
-    // path (`a::b`) would otherwise silently drop its leading segments and bind
+    let segments = &imp.path.value.segments;
+    let joined = segments
+        .iter()
+        .map(|s| s.value.as_str())
+        .collect::<Vec<_>>()
+        .join("::");
+    // Only single-segment module imports are supported, and std's nested
+    // ones, as `ui::html` (spec 4.3 §3). Any other qualified or nested path
+    // (`a::b`) would otherwise silently drop its leading segments and bind
     // an unrelated module; reject it explicitly like `import ... as`.
-    if imp.path.value.segments.len() > 1 {
-        diagnostics.push(unsupported(
-            span,
-            "qualified or nested import paths (`a::b`) are not supported yet; \
-             import a top-level module by its name",
-        ));
+    if segments.len() > 1 && !IMPORTED_STD.iter().any(|&(n, _)| n == joined) {
+        let first = segments[0].value.as_str();
+        if IMPORTED_STD.iter().any(|&(n, _)| n == first) {
+            diagnostics.push(
+                Diagnostic::error("E0001", format!("cannot find module `{joined}`"))
+                    .with_primary_label(span, "no such module"),
+            );
+        } else {
+            diagnostics.push(unsupported(
+                span,
+                "qualified or nested import paths (`a::b`) are not supported yet; \
+                 import a top-level module by its name",
+            ));
+        }
         return;
     }
-    let target_name = imp
-        .path
-        .value
-        .segments
-        .last()
-        .map(|s| s.value.as_str())
-        .unwrap_or("");
+    let target_name = joined.as_str();
     let target = match modules[mid].imports.get(target_name) {
         Some(ImportTarget::Module(target)) => *target,
         Some(ImportTarget::Reported) => return,
@@ -2848,15 +2864,32 @@ fn resolve_import(
         );
         return;
     }
-    // Spec 3.4a §3.3: the module, at its own segment. Only one-segment
-    // paths reach here, and `path.span` would also cover a list's `::`.
-    if let Some(first) = imp.path.value.segments.first() {
-        occurrences.push(Occurrence {
-            span: first.span,
-            role: Role::Use,
-            target: Target::Module(ModuleId(target as u32)),
-            shorthand: false,
-        });
+    // Spec 3.4a §3.3: the module, at its own segment; `path.span` would
+    // also cover a list's `::`. Each segment of a std module's nested path
+    // names the module its path so far names, as `ui` in `ui::html` (spec
+    // 4.3 §3).
+    for (k, segment) in segments.iter().enumerate() {
+        let named = if k + 1 == segments.len() {
+            Some(target)
+        } else {
+            let prefix = segments[..=k]
+                .iter()
+                .map(|s| s.value.as_str())
+                .collect::<Vec<_>>()
+                .join("::");
+            IMPORTED_STD
+                .iter()
+                .find(|&&(n, _)| n == prefix)
+                .and_then(|&(_, std_name)| modules.iter().position(|m| m.name == std_name))
+        };
+        if let Some(t) = named {
+            occurrences.push(Occurrence {
+                span: segment.span,
+                role: Role::Use,
+                target: Target::Module(ModuleId(t as u32)),
+                shorthand: false,
+            });
+        }
     }
     match &imp.kind {
         ImportKind::Simple => {
@@ -3508,8 +3541,8 @@ mod tests {
 
     /// A WASM build compiles its own std (spec 4.0 §3.2, 4.1 §3.1, 4.2
     /// §3.2, 4.3 §3): ten of the native modules, in native's order, then
-    /// `std/dom`, `std/ui` and `std/rt`, after the user's. A native build
-    /// has neither `std/dom` nor `std/rt`.
+    /// `std/dom`, `std/ui`, `std/ui/html` and `std/rt`, after the user's. A
+    /// native build has neither `std/dom` nor `std/rt`.
     #[test]
     fn a_wasm_build_compiles_wasms_std_modules() {
         let wasm = resolve_for(CompileTarget::Wasm, "fn main() { }\n");
@@ -3532,6 +3565,7 @@ mod tests {
                 "$std.log",
                 "$std.dom",
                 "$std.ui",
+                "$std.ui.html",
                 "$std.rt"
             ]
         );
@@ -3605,6 +3639,35 @@ mod tests {
         }
     }
 
+    /// A nested import path names a std module, as `ui::html` does; any
+    /// other is E0900 still, and one under std's name that names nothing is
+    /// E0001 (spec 4.3 §3).
+    #[test]
+    fn a_nested_import_path_names_only_stds_modules() {
+        for target in [CompileTarget::Native, CompileTarget::Wasm] {
+            let listed = resolve_for(target, "import ui::html::{div}\nfn main() { }\n");
+            assert!(listed.diagnostics.is_empty(), "{:?}", listed.diagnostics);
+            assert!(listed
+                .definitions
+                .resolve_value(ModuleId(0), "div")
+                .is_some());
+            assert!(listed
+                .definitions
+                .resolve_value(ModuleId(0), "span")
+                .is_none());
+            let all = resolve_for(target, "import ui::html\nfn main() { }\n");
+            assert!(all.diagnostics.is_empty(), "{:?}", all.diagnostics);
+            assert!(all.definitions.resolve_value(ModuleId(0), "span").is_some());
+            let other = resolve_for(target, "import a::b\nfn main() { }\n");
+            let codes: Vec<&str> = other.diagnostics.iter().map(|d| d.code.as_str()).collect();
+            assert_eq!(codes, ["E0900"], "{:?}", other.diagnostics);
+            let nope = resolve_for(target, "import ui::nope\nfn main() { }\n");
+            let codes: Vec<&str> = nope.diagnostics.iter().map(|d| d.code.as_str()).collect();
+            assert_eq!(codes, ["E0001"], "{:?}", nope.diagnostics);
+            assert_eq!(nope.diagnostics[0].message, "cannot find module `ui::nope`");
+        }
+    }
+
     /// A WASM build knows the module of each public name of the std
     /// modules it does not compile, in every namespace; a native build
     /// compiles them all, and knows none (spec 4.1 §3.2).
diff --git a/std/lib.rs b/std/lib.rs
index fc53412..cb811da 100644
--- a/std/lib.rs
+++ b/std/lib.rs
@@ -2,7 +2,7 @@
 //! build time so the compiler stays one self-contained executable.
 //!
 //! Every `include_str!` path stays inside this package, so `cargo package`
-//! carries all 20 files, and its verification step builds the packaged copy
+//! carries all 21 files, and its verification step builds the packaged copy
 //! on its own (spec
 //! `docs/superpowers/specs/2026-10-07-phase-3-0-foundations-design.md` §3).
 //! `nova-resolver` gives each module its `$std.*` name and its place.
@@ -49,6 +49,8 @@ pub const TEST: &str = include_str!("test/lib.nova");
 pub const TIME: &str = include_str!("time/lib.nova");
 /// `std/ui/lib.nova`: reactivity and views, which a program imports.
 pub const UI: &str = include_str!("ui/lib.nova");
+/// `std/ui/html/lib.nova`: the element builders, which a program imports.
+pub const UI_HTML: &str = include_str!("ui/html/lib.nova");
 /// `std/rt/lib.nova`, the runtime of a WASM build, written in Nova. Not in
 /// [`ALL`]: only a WASM build compiles it, and its few public items are for
 /// std's other modules alone, so it is not documented (spec
@@ -58,7 +60,7 @@ pub const RT: &str = include_str!("rt/lib.nova");
 
 /// Every documented module, by its directory under `std/`: all but
 /// [`RT`] and [`DOM`].
-pub const ALL: [(&str, &str); 18] = [
+pub const ALL: [(&str, &str); 19] = [
     ("bytes", BYTES),
     ("collections", COLLECTIONS),
     ("core", CORE),
@@ -77,6 +79,7 @@ pub const ALL: [(&str, &str); 18] = [
     ("test", TEST),
     ("time", TIME),
     ("ui", UI),
+    ("ui/html", UI_HTML),
 ];
 
 #[cfg(test)]
@@ -118,7 +121,7 @@ mod tests {
             .chain(["rt/lib.nova", "dom/lib.nova"].map(String::from))
             .collect();
         assert_eq!(embedded, on_disk);
-        assert_eq!(on_disk.len(), 20);
+        assert_eq!(on_disk.len(), 21);
     }
 
     /// Each entry carries its own file, not a neighbour's.
diff --git a/std/ui/html/lib.nova b/std/ui/html/lib.nova
new file mode 100644
index 0000000..ce431c2
--- /dev/null
+++ b/std/ui/html/lib.nova
@@ -0,0 +1,217 @@
+//! The element builders: functions that make `std/ui`'s `View`s and
+//! `Attr`s, for `mount` to put into the page or `render_to_string` to write
+//! as HTML.
+//!
+//! It is not in the prelude: `import ui::html` binds every builder, about 60
+//! names, and `import ui::html::{div, text}` binds some. A module that defines
+//! one of the same names, such as `text`, `input` or `label`, gets E0002 from
+//! the first, and should import a list instead.
+//!
+//! An element takes its attributes and its children as two arrays; a void
+//! element, such as `input`, takes no children. `text` is fixed text and
+//! `dyn_text` text read again when the signals it reads change. `type` and
+//! `for` are keywords, so `attr("type", "checkbox")` writes those, and
+//! `element("main", [], [])` writes a `<main>`, which has no builder of its
+//! own: it would clash with a program's `fn main`.
+//!
+//! A list with a class, and the HTML it renders to:
+//!
+//! ```
+//! import ui
+//! import ui::html::{class, li, text, ul}
+//!
+//! let html = render_to_string(| | ul([class("menu")], [li([], [text("a & b")])]))
+//! assert_eq(html, "<ul class=\"menu\"><li>a &amp; b</li></ul>")
+//! ```
+
+import ui
+
+/// The element `tag`, which must be ASCII letters, digits and `-`, starting
+/// with a letter, with `attrs` and `children`.
+pub fn element(tag: String, attrs: [Attr], children: [View]) -> View {
+    Element(tag, attrs, children)
+}
+
+/// A `<div>` element.
+pub fn div(attrs: [Attr], children: [View]) -> View { Element("div", attrs, children) }
+
+/// A `<span>` element.
+pub fn span(attrs: [Attr], children: [View]) -> View { Element("span", attrs, children) }
+
+/// A `<p>` element.
+pub fn p(attrs: [Attr], children: [View]) -> View { Element("p", attrs, children) }
+
+/// A `<a>` element.
+pub fn a(attrs: [Attr], children: [View]) -> View { Element("a", attrs, children) }
+
+/// A `<button>` element.
+pub fn button(attrs: [Attr], children: [View]) -> View { Element("button", attrs, children) }
+
+/// A `<label>` element.
+pub fn label(attrs: [Attr], children: [View]) -> View { Element("label", attrs, children) }
+
+/// A `<form>` element.
+pub fn form(attrs: [Attr], children: [View]) -> View { Element("form", attrs, children) }
+
+/// A `<ul>` element.
+pub fn ul(attrs: [Attr], children: [View]) -> View { Element("ul", attrs, children) }
+
+/// A `<ol>` element.
+pub fn ol(attrs: [Attr], children: [View]) -> View { Element("ol", attrs, children) }
+
+/// A `<li>` element.
+pub fn li(attrs: [Attr], children: [View]) -> View { Element("li", attrs, children) }
+
+/// A `<h1>` element.
+pub fn h1(attrs: [Attr], children: [View]) -> View { Element("h1", attrs, children) }
+
+/// A `<h2>` element.
+pub fn h2(attrs: [Attr], children: [View]) -> View { Element("h2", attrs, children) }
+
+/// A `<h3>` element.
+pub fn h3(attrs: [Attr], children: [View]) -> View { Element("h3", attrs, children) }
+
+/// A `<h4>` element.
+pub fn h4(attrs: [Attr], children: [View]) -> View { Element("h4", attrs, children) }
+
+/// A `<h5>` element.
+pub fn h5(attrs: [Attr], children: [View]) -> View { Element("h5", attrs, children) }
+
+/// A `<h6>` element.
+pub fn h6(attrs: [Attr], children: [View]) -> View { Element("h6", attrs, children) }
+
+/// A `<section>` element.
+pub fn section(attrs: [Attr], children: [View]) -> View { Element("section", attrs, children) }
+
+/// A `<header>` element.
+pub fn header(attrs: [Attr], children: [View]) -> View { Element("header", attrs, children) }
+
+/// A `<footer>` element.
+pub fn footer(attrs: [Attr], children: [View]) -> View { Element("footer", attrs, children) }
+
+/// A `<nav>` element.
+pub fn nav(attrs: [Attr], children: [View]) -> View { Element("nav", attrs, children) }
+
+/// A `<article>` element.
+pub fn article(attrs: [Attr], children: [View]) -> View { Element("article", attrs, children) }
+
+/// A `<strong>` element.
+pub fn strong(attrs: [Attr], children: [View]) -> View { Element("strong", attrs, children) }
+
+/// A `<em>` element.
+pub fn em(attrs: [Attr], children: [View]) -> View { Element("em", attrs, children) }
+
+/// A `<code>` element.
+pub fn code(attrs: [Attr], children: [View]) -> View { Element("code", attrs, children) }
+
+/// A `<pre>` element.
+pub fn pre(attrs: [Attr], children: [View]) -> View { Element("pre", attrs, children) }
+
+/// A `<small>` element.
+pub fn small(attrs: [Attr], children: [View]) -> View { Element("small", attrs, children) }
+
+/// A `<table>` element.
+pub fn table(attrs: [Attr], children: [View]) -> View { Element("table", attrs, children) }
+
+/// A `<thead>` element.
+pub fn thead(attrs: [Attr], children: [View]) -> View { Element("thead", attrs, children) }
+
+/// A `<tbody>` element.
+pub fn tbody(attrs: [Attr], children: [View]) -> View { Element("tbody", attrs, children) }
+
+/// A `<tr>` element.
+pub fn tr(attrs: [Attr], children: [View]) -> View { Element("tr", attrs, children) }
+
+/// A `<th>` element.
+pub fn th(attrs: [Attr], children: [View]) -> View { Element("th", attrs, children) }
+
+/// A `<td>` element.
+pub fn td(attrs: [Attr], children: [View]) -> View { Element("td", attrs, children) }
+
+/// A `<select>` element.
+pub fn select(attrs: [Attr], children: [View]) -> View { Element("select", attrs, children) }
+
+/// A `<option>` element.
+pub fn option(attrs: [Attr], children: [View]) -> View { Element("option", attrs, children) }
+
+/// A `<textarea>` element.
+pub fn textarea(attrs: [Attr], children: [View]) -> View { Element("textarea", attrs, children) }
+
+/// An `<input>` element, which has no children.
+pub fn input(attrs: [Attr]) -> View {
+    let none: [View] = []
+    Element("input", attrs, none)
+}
+
+/// An `<img>` element, which has no children.
+pub fn img(attrs: [Attr]) -> View {
+    let none: [View] = []
+    Element("img", attrs, none)
+}
+
+/// An `<br>` element, which has no children.
+pub fn br(attrs: [Attr]) -> View {
+    let none: [View] = []
+    Element("br", attrs, none)
+}
+
+/// An `<hr>` element, which has no children.
+pub fn hr(attrs: [Attr]) -> View {
+    let none: [View] = []
+    Element("hr", attrs, none)
+}
+
+/// The text `content`, escaped when it is written.
+pub fn text(content: String) -> View { Text(content) }
+
+/// Text that `content` gives, read again each time a signal it read
+/// changes.
+pub fn dyn_text(content: fn() -> String) -> View { DynText(content) }
+
+/// The attribute `name` set to `value`.
+pub fn attr(name: String, value: String) -> Attr { Static(name, value) }
+
+/// The attribute `name`, set to what `value` gives and again each time a
+/// signal it read changes.
+pub fn dyn_attr(name: String, value: fn() -> String) -> Attr { Dynamic(name, value) }
+
+/// The `class` attribute.
+pub fn class(value: String) -> Attr { Static("class", value) }
+
+/// The `id` attribute.
+pub fn id(value: String) -> Attr { Static("id", value) }
+
+/// The `style` attribute.
+pub fn style(value: String) -> Attr { Static("style", value) }
+
+/// The `href` attribute.
+pub fn href(value: String) -> Attr { Static("href", value) }
+
+/// The `src` attribute.
+pub fn src(value: String) -> Attr { Static("src", value) }
+
+/// The `alt` attribute.
+pub fn alt(value: String) -> Attr { Static("alt", value) }
+
+/// The `name` attribute.
+pub fn name(value: String) -> Attr { Static("name", value) }
+
+/// The `placeholder` attribute.
+pub fn placeholder(value: String) -> Attr { Static("placeholder", value) }
+
+/// The `value` property, as an input's text.
+pub fn value(v: String) -> Attr { Property("value", v) }
+
+/// The `value` property, set to what `v` gives and again each time a signal
+/// it read changes.
+pub fn dyn_value(v: fn() -> String) -> Attr { DynProperty("value", v) }
+
+/// Calls `f` on each event of kind `kind`, such as `"click"`.
+pub fn on(kind: String, f: fn()) -> Attr { On(kind, |unused: String| f()) }
+
+/// Calls `f` on each click.
+pub fn on_click(f: fn()) -> Attr { On("click", |unused: String| f()) }
+
+/// Calls `f` with the element's `value` on each `input` event, as a text
+/// field's each keystroke.
+pub fn on_input(f: fn(String)) -> Attr { On("input", f) }
diff --git a/std/ui/lib.nova b/std/ui/lib.nova
index c41bb39..05a0300 100644
--- a/std/ui/lib.nova
+++ b/std/ui/lib.nova
@@ -1,8 +1,10 @@
-//! Reactivity: a signal holds a value, and the effects and memos that read
-//! it run again when it changes.
+//! Reactivity, and views that update themselves: a signal holds a value,
+//! and the effects and memos that read it run again when it changes.
 //!
 //! It is not in the prelude: a module that uses it imports it, with
 //! `import ui` for every name, or `import ui::{signal, effect}` for some.
+//! `import ui::html` brings the element builders, `div`, `button`, `text` and
+//! the rest, which make the `View`s that `render_to_string` writes as HTML.
 //!
 //! `set` runs the effects that read a signal before it returns, each once,
 //! and only after every memo it reads is current. A memo computes when it is
@@ -16,16 +18,20 @@
 //! changing a record read from one changes the signal without telling
 //! anyone, and `set` or `update` is how to tell.
 //!
-//! A signal, and an effect that reads it:
+//! A signal, an effect that reads it, and a view rendered to HTML:
 //!
 //! ```
 //! import ui
+//! import ui::html::{div, dyn_text, text}
 //!
 //! let count = signal(1)
 //! let seen = signal(0)
 //! effect(| | seen.set(count.value() * 10))
 //! count.set(2)
 //! assert_eq(seen.value(), 20)
+//!
+//! let html = render_to_string(| | div([], [text("count: "), dyn_text(| | "${count.value()}")]))
+//! assert_eq(html, "<div>count: 2</div>")
 //! ```
 
 // The design (spec
@@ -455,3 +461,206 @@ pub fn batch(f: fn()) {
     s.batch = s.batch - 1
     flush()
 }
+
+/// What a page shows: an element, text, or a part that changes. The
+/// builders of `std/ui/html` make them.
+pub type View =
+    /// An element: its tag, its attributes, and its children.
+    | Element(String, [Attr], [View])
+    /// Text.
+    | Text(String)
+    /// Text read again when the signals it read change.
+    | DynText(fn() -> String)
+    /// Views that change as a whole: `show`'s and `for_each`'s.
+    | Region(fn() -> [View])
+
+/// An attribute of an element, a property, or an event's handler.
+pub type Attr =
+    /// An attribute's name and value.
+    | Static(String, String)
+    /// An attribute whose value is read again when its signals change.
+    | Dynamic(String, fn() -> String)
+    /// A property, such as an input's `value`.
+    | Property(String, String)
+    /// A property whose value is read again when its signals change.
+    | DynProperty(String, fn() -> String)
+    /// A handler for events of a kind, given the target's `value`.
+    | On(String, fn(String))
+
+// `f` with no listener, for the views `show` and `for_each` build: what
+// those read is theirs, not the region's.
+fn view_of(f: fn() -> View) -> View { untracked(f) }
+
+/// `then()`'s view while `when` holds, and nothing otherwise. It is built
+/// again each time `when` is set to true, even when it already was.
+pub fn show(when: Signal<Bool>, then: fn() -> View) -> View {
+    Region(
+        | | {
+            if when.value() {
+                [view_of(then)]
+            } else {
+                let none: [View] = []
+                none
+            }
+        },
+    )
+}
+
+/// `render` of each item, in order. The whole list is built again each
+/// time `items` changes.
+pub fn for_each<T>(items: Signal<[T]>, render: fn(T) -> View) -> View {
+    Region(
+        | | {
+            let xs = items.value()
+            let mut out = [Text(""); xs.len()]
+            let mut i = 0
+            while i < xs.len() {
+                let x = xs[i]
+                out[i] = view_of(| | render(x))
+                i = i + 1
+            }
+            out
+        },
+    )
+}
+
+// A name `std/ui` writes into markup, checked: ASCII letters, digits and
+// `-`, starting with a letter.
+fn check_name(name: String, what: String) {
+    let cs = name.chars()
+    let mut ok = cs.len() > 0
+    let mut i = 0
+    while ok && i < cs.len() {
+        let c = cs[i]
+        let letter = (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z')
+        let digit = c >= '0' && c <= '9'
+        ok = letter || (i > 0 && (digit || c == '-'))
+        i = i + 1
+    }
+    if !ok { panic("std/ui: \"${name}\" is not ${what}") }
+}
+
+// An attribute's or a property's name, checked as `what`. Three parse their
+// value as HTML, which `std/ui` never inserts: `innerHTML` and `outerHTML`,
+// and `srcdoc`, an iframe's whole document. They are refused as either, and
+// without regard to case, as HTML reads an attribute's name.
+fn check_attr(name: String, what: String) {
+    check_name(name, what)
+    let n = name.to_lower()
+    if n == "innerhtml" || n == "outerhtml" || n == "srcdoc" {
+        panic("std/ui: \"${name}\" parses HTML, which std/ui does not insert")
+    }
+}
+
+// The names of element `tag` and of its attributes and properties, which
+// both renderers check before they write any of them.
+fn check_element(tag: String, attrs: [Attr]) {
+    check_name(tag, "a tag name")
+    let mut i = 0
+    while i < attrs.len() {
+        match attrs[i] {
+            Static(name, value) => check_attr(name, "an attribute name")
+            Dynamic(name, f) => check_attr(name, "an attribute name")
+            Property(name, value) => check_attr(name, "a property name")
+            DynProperty(name, f) => check_attr(name, "a property name")
+            On(kind, f) => {}
+        }
+        i = i + 1
+    }
+}
+
+// The elements written with no closing tag and no children.
+fn is_void(tag: String) -> Bool { tag == "input" || tag == "img" || tag == "br" || tag == "hr" }
+
+// `text` with `&`, `<` and `>` escaped, and `"` too in an attribute.
+fn escape(text: String, attribute: Bool) -> String {
+    let cs = text.chars()
+    let mut parts = Vec::new()
+    let mut i = 0
+    while i < cs.len() {
+        let c = cs[i]
+        if c == '&' {
+            parts.push("&amp;")
+        } else if c == '<' {
+            parts.push("&lt;")
+        } else if c == '>' {
+            parts.push("&gt;")
+        } else if attribute && c == '"' {
+            parts.push("&quot;")
+        } else {
+            parts.push("${c}")
+        }
+        i = i + 1
+    }
+    joined(parts)
+}
+
+fn joined(parts: Vec<String>) -> String {
+    let mut all = [""; parts.len()]
+    let mut i = 0
+    while i < parts.len() {
+        all[i] = at(parts, i)
+        i = i + 1
+    }
+    "".join(all)
+}
+
+// Write `v` as HTML into `out`.
+fn write_view(v: View, out: Vec<String>) {
+    let mut o = out
+    match v {
+        Element(tag, attrs, children) => {
+            check_element(tag, attrs)
+            o.push("<${tag}")
+            let mut i = 0
+            while i < attrs.len() {
+                write_attr(attrs[i], out)
+                i = i + 1
+            }
+            o.push(">")
+            if !is_void(tag) {
+                write_views(children, out)
+                o.push("</${tag}>")
+            }
+        }
+        Text(t) => o.push(escape(t, false))
+        DynText(f) => o.push(escape(f(), false))
+        Region(f) => write_views(f(), out)
+    }
+}
+
+fn write_views(views: [View], out: Vec<String>) {
+    let mut i = 0
+    while i < views.len() {
+        write_view(views[i], out)
+        i = i + 1
+    }
+}
+
+fn write_attr(a: Attr, out: Vec<String>) {
+    let mut o = out
+    match a {
+        Static(name, value) => o.push(" ${name}=\"${escape(value, true)}\"")
+        Dynamic(name, f) => o.push(" ${name}=\"${escape(f(), true)}\"")
+        Property(name, value) => o.push(" ${name}=\"${escape(value, true)}\"")
+        DynProperty(name, f) => o.push(" ${name}=\"${escape(f(), true)}\"")
+        On(kind, f) => {}
+    }
+}
+
+/// `f`'s view as HTML. `f` runs in a new owner, which is disposed once the
+/// HTML is written; text and attributes are read once, handlers are left
+/// out, and `on_mount` never runs. Text and attribute values are escaped.
+pub fn render_to_string(f: fn() -> View) -> String {
+    let mut s = ui()
+    s.rendering = s.rendering + 1
+    let out = Vec::new()
+    root(
+        |dispose: fn()| {
+            write_view(f(), out)
+            dispose()
+        },
+    )
+    s.rendering = s.rendering - 1
+    joined(out)
+}
````

### Task 4: `mount`, and a backend per target

Spec §5.2 items 7 and 8, §7.1, §7.2, §7.3, §8, §9.3; decisions 3, 4, 5,
13, 17 to 19, 24 and 26.

**Files:**
- Create: `std/ui/backend/native.nova`, `std/ui/backend/wasm.nova`
- Modify:
  - `std/ui/lib.nova`
  - `crates/nova-resolver/src/lib.rs`
  - `std/lib.rs`
  - `crates/nova-doc/src/model.rs`
- Test:
  - `tests/dom/ui_counter`, `ui_show_for_each`, `ui_attrs`, `ui_mount`
    and `ui_mount_html`, each `.nova`, `.mjs` and `.stdout`
  - `crates/nova-cli/tests/run_tests.rs`, `wasm_tests.rs`
  - `crates/nova-driver/tests/index_complete.rs`

- [ ] **Step 1: The tests.** `patch_block t4-tests`:
  - **`ui_counter`,** the gate: a `p#count` with `dyn_text` and a
    `button#add` whose `on_click` adds one, mounted into `body`; its
    harness is 4.2's `counter.mjs`, three clicks a turn apart.
  - **`ui_show_for_each`:** a toggle that shows a panel, whose `on_mount`
    finds `#panel` in the page and whose cleanup runs when it hides; and
    a list whose rows, between two fixed items, are rebuilt as rows are
    added, and once more when `#again` sets the same rows, a row's handler
    still answering.
  - **`ui_attrs`:** an `input` with a fixed `value` and `on_input`, a
    paragraph with a dynamic class and text, and an `input` with
    `dyn_value`; the harness types twice. A button sets two signals that
    an effect sums, which prints once; and an effect clicks `#probe`,
    whose handler reads a signal that `main` then sets, without running
    that effect again.
  - **`ui_mount`:** the mount goes after the paragraph `#app` already
    holds; its two `on_mount`s find its node in the page, in the order
    they were added; a `for_each` whose rows a `dyn_text` sets during the
    build runs only the new row's `on_mount`, whose cleanup runs when
    `main` sets the rows again; one outside any mount runs at once; and
    mounting into `#nope` panics.
  - **`ui_mount_html`:** a mount, then one whose paragraph has the
    property `innerHTML`, which panics before any of its nodes is in the
    page.
  - **`wasm_tests.rs`:** a test for each, as is and under stress:
    - the counter keeps one call in use and collects four times;
    - the rows' calls in use go 5, 5, 6, 7, 7, 7, 7, where twelve after the
      second "more" would mean the old rows' listeners leaked;
    - and after `#again` as many handles are live as before it.
  - **`run_tests.rs`:** `ui_mount_panics_natively`, an inline program:
    `render_to_string` renders, then `mount` panics with its message.
  - **`index_complete.rs`:** 173 files, with the backend's native source.
- [ ] **Step 2: Run, and see them fail.**
  - `R`. Expected: `3 passed; 1 failed`: `ui_mount_panics_natively`, with
    `left: ""` and `right: "fine\n"`: the program does not build, since
    `mount` does not exist yet (`W`'s output shows the error).
  - `W`. Expected: `21 passed; 5 failed`: the five new tests, at
    ``error[E0001]: cannot find function `mount` in this scope``.
  - The index: `0 passed; 1 failed`, `left: 172`, `right: 173`.
- [ ] **Step 3: The code.** `patch_block t4-code`:
  - **`std/ui/backend/wasm.nova`:** `Node` wraps a `dom::Element`, and
    each `backend_*` function calls `std/dom`; `backend_listen` returns the
    function that removes its listener.
  - **`std/ui/backend/native.nova`:** the same names, each panicking with
    ``std/ui: mount needs a build for WASM; render_to_string renders on
    any target``.
  - **`std/ui/lib.nova`:** `mount`, `build`, `build_region` and
    `build_attr`: an effect for each part that changes; `build` checks an
    element with `check_element` before it makes the node; a region
    between two empty text nodes, whose render counts as a build before
    its views are made, so their `on_mount` waits for their nodes, and has
    its own owner, which the region's effect disposes when it runs again;
    `run_mounted`, which skips a callback whose owner is disposed and runs
    the others with their owner current; and a handler runs untracked, in
    a `batch`.
  - **`nova-resolver`:** `$std.ui.backend` in both targets' std, each with
    its own source; `STD_INTERNAL` is `pub` and a list, `std/rt` and the
    backend. Tests: `uis_backend_is_seen_by_std_alone`, and the WASM list
    of 15.
  - **`std/lib.rs`:** `UI_BACKEND_NATIVE` and `UI_BACKEND_WASM`, outside
    `ALL`; 23 files.
  - **`nova-doc`:** a module in `STD_INTERNAL` has no page.
- [ ] **Step 4: Run.**
  - `R`. Expected: `4 passed`.
  - `W`. Expected: `26 passed`.
  - The index: `1 passed`.
  - `CARGO_INCREMENTAL=0 cargo test -p nova-resolver -p nova-std -p nova-doc --lib`.
    Expected: `114 passed` in all.
- [ ] **Step 5: The checks,** as the conventions list them.
- [ ] **Step 6: Commit** `std/ui: mount, and a backend per target`.

**Patch `t4-tests`:**

<!-- patch t4-tests -->

````diff
diff --git a/crates/nova-cli/tests/run_tests.rs b/crates/nova-cli/tests/run_tests.rs
index 2281c7b..4bcb25a 100644
--- a/crates/nova-cli/tests/run_tests.rs
+++ b/crates/nova-cli/tests/run_tests.rs
@@ -11440,3 +11440,35 @@ fn ui_checks_every_name_and_never_parses_html() {
         );
     }
 }
+
+/// Natively there is no page: `mount` panics, saying how to render, and
+/// `render_to_string` still renders (spec 4.3 §7.1).
+#[test]
+fn ui_mount_panics_natively() {
+    let dir = std::env::temp_dir().join("nova-ui-mount-native");
+    let _ = std::fs::remove_dir_all(&dir);
+    std::fs::create_dir_all(&dir).expect("temp dir");
+    std::fs::write(
+        dir.join("main.nova"),
+        "import ui\nimport ui::html::{text}\n\nfn main() {\n    println(render_to_string(| | text(\"fine\")))\n    mount(\"#app\", | | text(\"no page\"))\n    println(\"not reached\")\n}\n",
+    )
+    .expect("write main.nova");
+    let a = nova()
+        .current_dir(&dir)
+        .arg("run")
+        .arg("main.nova")
+        .assert()
+        .failure();
+    let o = a.get_output();
+    assert_eq!(
+        String::from_utf8_lossy(&o.stdout).replace("\r\n", "\n"),
+        "fine\n"
+    );
+    let err = String::from_utf8_lossy(&o.stderr);
+    assert!(
+        err.contains(
+            "nova: panic: std/ui: mount needs a build for WASM; render_to_string renders on any target"
+        ),
+        "{err}"
+    );
+}
diff --git a/crates/nova-cli/tests/wasm_tests.rs b/crates/nova-cli/tests/wasm_tests.rs
index c4bfb18..5dc3c28 100644
--- a/crates/nova-cli/tests/wasm_tests.rs
+++ b/crates/nova-cli/tests/wasm_tests.rs
@@ -1833,6 +1833,88 @@ mod stress {
     }
 }
 
+/// The gate of 4.3 (spec
+/// `docs/superpowers/specs/2026-10-17-phase-4-3-std-ui-reactivity-and-views-design.md`
+/// §1): the counter written with a signal and builder calls, which three
+/// clicks make read 3. Under stress the collector runs after `main` and
+/// after each click, and keeps what the page uses through `std/ui`'s state:
+/// the button's listener, one call, and its handles.
+#[test]
+fn ui_the_builder_counter_counts_clicks() {
+    dom_program("ui_counter", &[]);
+    let err = dom_program("ui_counter", &STRESS);
+    if node().is_none() {
+        return;
+    }
+    assert_eq!(live_after_each(&err).len(), 4, "{err}");
+    assert_eq!(calls_after_each(&err), [1, 1, 1, 1], "{err}");
+}
+
+/// `show` and `for_each` rebuild their part of the page between their
+/// markers, and a hidden branch's cleanups run (spec 4.3 §6.3). Under
+/// stress, a rebuilt row's listener leaves std/rt's table: after two more
+/// rows, the page holds seven calls, not twelve; and the old rows' handles
+/// are released (§9.3): setting the same rows again leaves as many live.
+#[test]
+fn ui_show_and_for_each_rebuild_their_part() {
+    dom_program("ui_show_for_each", &[]);
+    let err = dom_program("ui_show_for_each", &STRESS);
+    if node().is_none() {
+        return;
+    }
+    assert_eq!(calls_after_each(&err), [5, 5, 6, 7, 7, 7, 7], "{err}");
+    let handles = handles_released(&err);
+    assert_eq!(handles.len(), 7, "{err}");
+    let (_, before) = handles[3];
+    let (released, after) = handles[4];
+    assert!(released > 0 && after == before, "{err}");
+}
+
+/// A dynamic class, a `value` property, fixed and dynamic, and `on_input`
+/// given what was typed; a handler runs in a batch and reads without
+/// subscribing, even when an effect clicked (spec 4.3 §5.2, §6.2).
+#[test]
+fn ui_attributes_follow_their_signals() {
+    dom_program("ui_attrs", &[]);
+    dom_program("ui_attrs", &STRESS);
+}
+
+/// A mount goes after what its element holds; `on_mount` runs once the
+/// mount's nodes are in the page, in order, but not for a render disposed
+/// first, and at once outside a mount; a selector nothing matches panics
+/// (spec 4.3 §5.2, §7.1).
+#[test]
+fn ui_mount_runs_on_mount_and_panics_without_a_match() {
+    for env in [&[][..], &STRESS[..]] {
+        let Some(a) = dom_run("ui_mount", env) else {
+            return;
+        };
+        let err = stderr(&a.code(134));
+        assert!(
+            err.contains("nova: panic: std/ui: mount: no element matches \"#nope\"\n"),
+            "{err}"
+        );
+    }
+}
+
+/// `mount` checks names as `render_to_string` does, and refuses
+/// `innerHTML` before any node of the view is in the page (spec 4.3 §7.2).
+#[test]
+fn ui_mount_never_inserts_html() {
+    for env in [&[][..], &STRESS[..]] {
+        let Some(a) = dom_run("ui_mount_html", env) else {
+            return;
+        };
+        let err = stderr(&a.code(134));
+        assert!(
+            err.contains(
+                "nova: panic: std/ui: \"innerHTML\" parses HTML, which std/ui does not insert\n"
+            ),
+            "{err}"
+        );
+    }
+}
+
 /// `import ui::html::{div}` checks on both targets; a program's own nested
 /// path is still E0900, `ui::nope` is E0001, and `signal` without an import
 /// is E0001 (spec 4.3 §9.5).
diff --git a/crates/nova-driver/tests/index_complete.rs b/crates/nova-driver/tests/index_complete.rs
index 63e1612..edac8c1 100644
--- a/crates/nova-driver/tests/index_complete.rs
+++ b/crates/nova-driver/tests/index_complete.rs
@@ -216,8 +216,8 @@ fn every_name_in_the_corpora_is_indexed() {
         }
     }
     // 7 + 141 + 3 files of user code, 05-geometry's library's 2, and
-    // native std's 19.
-    assert_eq!(files, 172);
+    // native std's 20.
+    assert_eq!(files, 173);
     failures.sort();
     failures.dedup();
     assert!(
diff --git a/tests/dom/ui_attrs.mjs b/tests/dom/ui_attrs.mjs
new file mode 100644
index 0000000..70a335b
--- /dev/null
+++ b/tests/dom/ui_attrs.mjs
@@ -0,0 +1,23 @@
+// Type into the field twice, a turn apart, printing the page and the
+// fields' values after each; then click the button that sets two signals.
+import { FakeEvent, install } from "./fake_dom.mjs";
+install();
+const app = document.createElement("div");
+app.setAttribute("id", "app");
+document.body.appendChild(app);
+const { default: init } = await import("./dist/app.js");
+await init();
+const field = document.getElementById("name");
+const show = () => {
+  console.log(app.innerHTML);
+  console.log(`name=${field.value} echo=${document.getElementById("echo").value}`);
+};
+show();
+for (const typed of ["nova", ""]) {
+  field.value = typed;
+  field.dispatchEvent(new FakeEvent("input", { bubbles: true }));
+  await new Promise((r) => setTimeout(r, 0));
+  show();
+}
+document.getElementById("both").click();
+await new Promise((r) => setTimeout(r, 0));
diff --git a/tests/dom/ui_attrs.nova b/tests/dom/ui_attrs.nova
new file mode 100644
index 0000000..cdc1a94
--- /dev/null
+++ b/tests/dom/ui_attrs.nova
@@ -0,0 +1,81 @@
+// Attributes, properties and input in the page (spec 4.3 §5.2, §6.2,
+// §7.1): a dynamic class, a `value` property, fixed and dynamic, and
+// `on_input` given what was typed; a handler runs in a batch, and reads
+// without subscribing, even when an effect is what made its event.
+
+import dom
+import ui
+import ui::html::{
+    attr,
+    button,
+    div,
+    dyn_attr,
+    dyn_text,
+    dyn_value,
+    id,
+    input,
+    on_click,
+    on_input,
+    p,
+    text,
+    value,
+}
+
+fn main() {
+    let name = signal("")
+    let left = signal(0)
+    let right = signal(0)
+    let seen = signal(0)
+    mount(
+        "#app",
+        | | div(
+            [],
+            [
+                input(
+                    [
+                        id("name"),
+                        attr("type", "text"),
+                        value("start"),
+                        on_input(|v: String| name.set(v)),
+                    ],
+                ),
+                p(
+                    [
+                        id("greeting"),
+                        dyn_attr("class", | | if name.value() == "" { "empty" } else { "full" }),
+                    ],
+                    [dyn_text(| | "hello, ${name.value()}")],
+                ),
+                input([id("echo"), dyn_value(| | name.value())]),
+                button(
+                    [
+                        id("both"),
+                        on_click(
+                            | | {
+                                left.set(1)
+                                right.set(2)
+                            },
+                        ),
+                    ],
+                    [text("both")],
+                ),
+                button(
+                    [id("probe"), on_click(| | println("the probe sees ${seen.value()}"))],
+                    [text("probe")],
+                ),
+            ],
+        ),
+    )
+    effect(| | println("the sum is ${left.value() + right.value()}"))
+    effect(
+        | | {
+            println("an effect clicks the probe")
+            match query_selector("#probe") {
+                Some(e) => e.click()
+                None => println("no probe")
+            }
+        },
+    )
+    seen.set(1)
+    println("seen is set")
+}
diff --git a/tests/dom/ui_attrs.stdout b/tests/dom/ui_attrs.stdout
new file mode 100644
index 0000000..0a39beb
--- /dev/null
+++ b/tests/dom/ui_attrs.stdout
@@ -0,0 +1,11 @@
+the sum is 0
+an effect clicks the probe
+the probe sees 0
+seen is set
+<div><input id="name" type="text"><p id="greeting" class="empty">hello, </p><input id="echo"><button id="both">both</button><button id="probe">probe</button></div>
+name=start echo=
+<div><input id="name" type="text"><p id="greeting" class="full">hello, nova</p><input id="echo"><button id="both">both</button><button id="probe">probe</button></div>
+name=nova echo=nova
+<div><input id="name" type="text"><p id="greeting" class="empty">hello, </p><input id="echo"><button id="both">both</button><button id="probe">probe</button></div>
+name= echo=
+the sum is 3
diff --git a/tests/dom/ui_counter.mjs b/tests/dom/ui_counter.mjs
new file mode 100644
index 0000000..e27f0f6
--- /dev/null
+++ b/tests/dom/ui_counter.mjs
@@ -0,0 +1,12 @@
+// Click the button three times, a turn apart, then print the page.
+import { install } from "./fake_dom.mjs";
+install();
+const { default: init } = await import("./dist/app.js");
+await init();
+const button = document.getElementById("add");
+for (let i = 0; i < 3; i++) {
+  button.click();
+  await new Promise((r) => setTimeout(r, 0));
+}
+console.log(document.getElementById("count").textContent);
+console.log(document.body.outerHTML);
diff --git a/tests/dom/ui_counter.nova b/tests/dom/ui_counter.nova
new file mode 100644
index 0000000..624d721
--- /dev/null
+++ b/tests/dom/ui_counter.nova
@@ -0,0 +1,21 @@
+// The gate of 4.3 (spec
+// docs/superpowers/specs/2026-10-17-phase-4-3-std-ui-reactivity-and-views-design.md
+// §1): 4.2's counter, written with a signal and builder calls. It has 4.2's
+// ids, `#add` and `#count`, so `tests/browser/counter.mjs` clicks either.
+
+import ui
+import ui::html::{button, div, dyn_text, id, on_click, p, text}
+
+fn main() {
+    let count = signal(0)
+    mount(
+        "body",
+        | | div(
+            [],
+            [
+                p([id("count")], [dyn_text(| | "${count.value()}")]),
+                button([id("add"), on_click(| | count.update(|n: Int| n + 1))], [text("add one")]),
+            ],
+        ),
+    )
+}
diff --git a/tests/dom/ui_counter.stdout b/tests/dom/ui_counter.stdout
new file mode 100644
index 0000000..c88d722
--- /dev/null
+++ b/tests/dom/ui_counter.stdout
@@ -0,0 +1,2 @@
+3
+<body><div><p id="count">3</p><button id="add">add one</button></div></body>
diff --git a/tests/dom/ui_mount.mjs b/tests/dom/ui_mount.mjs
new file mode 100644
index 0000000..18902e7
--- /dev/null
+++ b/tests/dom/ui_mount.mjs
@@ -0,0 +1,17 @@
+// Mount into an element that holds a paragraph already, then print the
+// page; `main` panics at its second mount.
+import { install } from "./fake_dom.mjs";
+install();
+const app = document.createElement("div");
+app.setAttribute("id", "app");
+const old = document.createElement("p");
+old.setAttribute("id", "old");
+old.textContent = "here first";
+app.appendChild(old);
+document.body.appendChild(app);
+const { default: init } = await import("./dist/app.js");
+try {
+  await init();
+} finally {
+  console.log(document.body.outerHTML);
+}
diff --git a/tests/dom/ui_mount.nova b/tests/dom/ui_mount.nova
new file mode 100644
index 0000000..7d22170
--- /dev/null
+++ b/tests/dom/ui_mount.nova
@@ -0,0 +1,61 @@
+// `mount` and `on_mount` (spec 4.3 §5.2, §7.1): a mount puts its nodes after
+// what the element holds; the callbacks a mount registers run once its nodes
+// are in the page, in the order they were added, but not one whose render a
+// `set` during the build disposed; each runs with the owner it was added in,
+// so its cleanup runs when its render goes; one outside any mount runs at
+// once; and mounting into a selector nothing matches panics.
+
+import dom
+import ui
+import ui::html::{div, dyn_text, id, li, p, text, ul}
+
+// A row whose `on_mount` says whether the row's node is in the page, and
+// adds a cleanup that says when the row is gone.
+fn row(r: Int) -> View {
+    on_mount(
+        | | {
+            match query_selector("#row${r}") {
+                Some(e) => println("on_mount of row ${r}: in the page")
+                None => println("on_mount of row ${r}: not in the page")
+            }
+            on_cleanup(| | println("row ${r} is gone"))
+        },
+    )
+    li([id("row${r}")], [text("row ${r}")])
+}
+
+fn main() {
+    let rows = signal([1])
+    mount(
+        "#app",
+        | | {
+            on_mount(
+                | | {
+                    match query_selector("#hello") {
+                        Some(e) => println("on_mount: #hello is in the page, reading ${e.text()}")
+                        None => println("on_mount: #hello is not in the page")
+                    }
+                },
+            )
+            on_mount(| | println("the second on_mount"))
+            println("building")
+            div(
+                [],
+                [
+                    p([id("hello")], [text("hi")]),
+                    ul([], [for_each(rows, row)]),
+                    dyn_text(
+                        | | {
+                            rows.set([2])
+                            ""
+                        },
+                    ),
+                ],
+            )
+        },
+    )
+    rows.set([3])
+    on_mount(| | println("on_mount outside a mount runs at once"))
+    mount("#nope", | | text("never"))
+    println("not reached")
+}
diff --git a/tests/dom/ui_mount.stdout b/tests/dom/ui_mount.stdout
new file mode 100644
index 0000000..cd65de4
--- /dev/null
+++ b/tests/dom/ui_mount.stdout
@@ -0,0 +1,8 @@
+building
+on_mount: #hello is in the page, reading hi
+the second on_mount
+on_mount of row 2: in the page
+row 2 is gone
+on_mount of row 3: in the page
+on_mount outside a mount runs at once
+<body><div id="app"><p id="old">here first</p><div><p id="hello">hi</p><ul><li id="row3">row 3</li></ul></div></div></body>
diff --git a/tests/dom/ui_mount_html.mjs b/tests/dom/ui_mount_html.mjs
new file mode 100644
index 0000000..1c4742e
--- /dev/null
+++ b/tests/dom/ui_mount_html.mjs
@@ -0,0 +1,13 @@
+// Mount, then print the page; `main` panics at its second mount, before any
+// of its nodes is in the page.
+import { install } from "./fake_dom.mjs";
+install();
+const app = document.createElement("div");
+app.setAttribute("id", "app");
+document.body.appendChild(app);
+const { default: init } = await import("./dist/app.js");
+try {
+  await init();
+} finally {
+  console.log(document.body.outerHTML);
+}
diff --git a/tests/dom/ui_mount_html.nova b/tests/dom/ui_mount_html.nova
new file mode 100644
index 0000000..56e14eb
--- /dev/null
+++ b/tests/dom/ui_mount_html.nova
@@ -0,0 +1,12 @@
+// `mount` checks the names it writes as `render_to_string` does, and refuses
+// a property that parses HTML before any node of its view is in the page
+// (spec 4.3 §7.2).
+
+import ui
+import ui::html::{div, p, text}
+
+fn main() {
+    mount("#app", | | p([], [text("fine")]))
+    mount("#app", | | div([], [p([Property("innerHTML", "<img src=x onerror=alert(1)>")], [])]))
+    println("not reached")
+}
diff --git a/tests/dom/ui_mount_html.stdout b/tests/dom/ui_mount_html.stdout
new file mode 100644
index 0000000..b1e85e0
--- /dev/null
+++ b/tests/dom/ui_mount_html.stdout
@@ -0,0 +1 @@
+<body><div id="app"><p>fine</p></div></body>
diff --git a/tests/dom/ui_show_for_each.mjs b/tests/dom/ui_show_for_each.mjs
new file mode 100644
index 0000000..644b0dc
--- /dev/null
+++ b/tests/dom/ui_show_for_each.mjs
@@ -0,0 +1,19 @@
+// Toggle the panel on and off, add rows, set the same rows again, and click
+// a row, a turn apart, printing the page after each.
+import { install } from "./fake_dom.mjs";
+install();
+const app = document.createElement("div");
+app.setAttribute("id", "app");
+document.body.appendChild(app);
+const { default: init } = await import("./dist/app.js");
+await init();
+const turn = () => new Promise((r) => setTimeout(r, 0));
+const show = (what) => console.log(`${what}: ${app.innerHTML}`);
+show("mounted");
+for (const step of ["toggle", "more", "more", "again", "toggle"]) {
+  document.getElementById(step).click();
+  await turn();
+  show(step);
+}
+document.querySelectorAll("#list li")[2].click();
+await turn();
diff --git a/tests/dom/ui_show_for_each.nova b/tests/dom/ui_show_for_each.nova
new file mode 100644
index 0000000..b947ef6
--- /dev/null
+++ b/tests/dom/ui_show_for_each.nova
@@ -0,0 +1,65 @@
+// `show` and `for_each` in the page (spec 4.3 §6.3, §7.1): `show` builds its
+// branch when its signal becomes true and disposes of it, running its
+// cleanups, when it becomes false; `for_each` builds its rows again each
+// time its list is set, the same list too, between its markers, leaving the
+// nodes around it as they are.
+
+import dom
+import ui
+import ui::html::{button, div, id, li, on_click, p, text, ul}
+
+fn more(xs: [String]) -> [String] {
+    let mut out = [""; xs.len() + 1]
+    let mut i = 0
+    while i < xs.len() {
+        out[i] = xs[i]
+        i = i + 1
+    }
+    out[xs.len()] = "row ${xs.len() + 1}"
+    out
+}
+
+fn main() {
+    let open = signal(false)
+    let rows = signal(["row 1", "row 2"])
+    mount(
+        "#app",
+        | | div(
+            [],
+            [
+                button([id("toggle"), on_click(| | open.update(|o: Bool| !o))], [text("toggle")]),
+                show(
+                    open,
+                    | | {
+                        on_mount(
+                            | | {
+                                match query_selector("#panel") {
+                                    Some(e) => println("on_mount: the panel is in the page")
+                                    None => println("on_mount: the panel is not in the page")
+                                }
+                            },
+                        )
+                        on_cleanup(| | println("the panel is gone"))
+                        p([id("panel")], [text("panel")])
+                    },
+                ),
+                ul(
+                    [id("list")],
+                    [
+                        li([], [text("first")]),
+                        for_each(
+                            rows,
+                            |r: String| li([on_click(| | println("clicked ${r}"))], [text(r)]),
+                        ),
+                        li([], [text("last")]),
+                    ],
+                ),
+                button([id("more"), on_click(| | rows.update(more))], [text("more")]),
+                button(
+                    [id("again"), on_click(| | rows.update(|xs: [String]| xs))],
+                    [text("again")],
+                ),
+            ],
+        ),
+    )
+}
diff --git a/tests/dom/ui_show_for_each.stdout b/tests/dom/ui_show_for_each.stdout
new file mode 100644
index 0000000..a0c8449
--- /dev/null
+++ b/tests/dom/ui_show_for_each.stdout
@@ -0,0 +1,9 @@
+mounted: <div><button id="toggle">toggle</button><ul id="list"><li>first</li><li>row 1</li><li>row 2</li><li>last</li></ul><button id="more">more</button><button id="again">again</button></div>
+on_mount: the panel is in the page
+toggle: <div><button id="toggle">toggle</button><p id="panel">panel</p><ul id="list"><li>first</li><li>row 1</li><li>row 2</li><li>last</li></ul><button id="more">more</button><button id="again">again</button></div>
+more: <div><button id="toggle">toggle</button><p id="panel">panel</p><ul id="list"><li>first</li><li>row 1</li><li>row 2</li><li>row 3</li><li>last</li></ul><button id="more">more</button><button id="again">again</button></div>
+more: <div><button id="toggle">toggle</button><p id="panel">panel</p><ul id="list"><li>first</li><li>row 1</li><li>row 2</li><li>row 3</li><li>row 4</li><li>last</li></ul><button id="more">more</button><button id="again">again</button></div>
+again: <div><button id="toggle">toggle</button><p id="panel">panel</p><ul id="list"><li>first</li><li>row 1</li><li>row 2</li><li>row 3</li><li>row 4</li><li>last</li></ul><button id="more">more</button><button id="again">again</button></div>
+the panel is gone
+toggle: <div><button id="toggle">toggle</button><ul id="list"><li>first</li><li>row 1</li><li>row 2</li><li>row 3</li><li>row 4</li><li>last</li></ul><button id="more">more</button><button id="again">again</button></div>
+clicked row 2
````

**Patch `t4-code`:**

<!-- patch t4-code -->

````diff
diff --git a/crates/nova-doc/src/model.rs b/crates/nova-doc/src/model.rs
index df6c355..ee55904 100644
--- a/crates/nova-doc/src/model.rs
+++ b/crates/nova-doc/src/model.rs
@@ -500,8 +500,12 @@ impl<'a> Ctx<'a> {
     }
 
     /// A std module's path under `std/`, as `ui/html` for `$std.ui.html`
-    /// (spec 4.3 §3).
+    /// (spec 4.3 §3), or `None` for one only std sees, as `std/ui`'s
+    /// backend (§7.3).
     fn std_path(&self, m: ModuleId) -> Option<String> {
+        if nova_resolver::STD_INTERNAL.contains(&self.defs.module_name(m)) {
+            return None;
+        }
         Some(self.std_name(m)?.replace('.', "/"))
     }
 
diff --git a/crates/nova-resolver/src/lib.rs b/crates/nova-resolver/src/lib.rs
index e9e0685..94adcde 100644
--- a/crates/nova-resolver/src/lib.rs
+++ b/crates/nova-resolver/src/lib.rs
@@ -2006,7 +2006,11 @@ pub fn resolve_program_for(
         if IMPORTED_STD.iter().any(|&(_, m)| m == name) {
             continue;
         }
-        let from = if name == STD_INTERNAL { std_start } else { 0 };
+        let from = if STD_INTERNAL.contains(&name) {
+            std_start
+        } else {
+            0
+        };
         import_std_module(&mut definitions, std_exports, mid, from);
     }
 
@@ -2077,7 +2081,8 @@ fn missing_names(target: CompileTarget) -> FxHashMap<String, &'static str> {
 
 /// The embedded standard-library modules, compiled as implicit modules and
 /// glob-imported into every user module, but for `std/ui` and `std/ui/html`,
-/// which a program imports (spec 4.3 §3). Order is significant only in that it
+/// which a program imports, and `std/ui`'s backend, which only std sees
+/// (spec 4.3 §3, §7.3). Order is significant only in that it
 /// fixes module indices; user modules always come first, then these in the
 /// order listed here — `std/core` stays first so its module index is
 /// unchanged from when it was the only embedded module. The sources come
@@ -2086,7 +2091,7 @@ fn missing_names(target: CompileTarget) -> FxHashMap<String, &'static str> {
 /// self-contained executable and `cargo package` keeps the files. Each name
 /// is `$std.*`, not a valid identifier, so it can never collide with a user
 /// module name or be named in an `import`.
-pub const STD_MODULES: [(&str, &str); 18] = [
+pub const STD_MODULES: [(&str, &str); 19] = [
     ("$std.core", nova_std::CORE),
     ("$std.bytes", nova_std::BYTES),
     ("$std.io", nova_std::IO),
@@ -2105,6 +2110,7 @@ pub const STD_MODULES: [(&str, &str); 18] = [
     ("$std.process", nova_std::PROCESS),
     ("$std.ui", nova_std::UI),
     ("$std.ui.html", nova_std::UI_HTML),
+    ("$std.ui.backend", nova_std::UI_BACKEND_NATIVE),
 ];
 
 /// What a program is compiled for (spec
@@ -2143,11 +2149,11 @@ impl CompileTarget {
 /// The std modules a WASM build compiles (spec 4.0 §3.2, 4.1 §3.1, 4.2
 /// §3.2, 4.3 §3): the ten of [`STD_MODULES`] that need no operating
 /// system, in the same order, then `std/dom`, which only an `import`
-/// reaches, then `std/ui` and `std/ui/html`, then `std/rt`, the runtime
-/// written in Nova. Their names are those of
+/// reaches, then `std/ui`'s three, with the backend's WASM source, then
+/// `std/rt`, the runtime written in Nova. Their names are those of
 /// [`STD_MODULES`], so `std/core` keeps its place first, and the same order
 /// gives a name the same winner in std's glob imports.
-pub const WASM_STD_MODULES: [(&str, &str); 14] = [
+pub const WASM_STD_MODULES: [(&str, &str); 15] = [
     ("$std.core", nova_std::CORE),
     ("$std.bytes", nova_std::BYTES),
     ("$std.collections", nova_std::COLLECTIONS),
@@ -2161,6 +2167,7 @@ pub const WASM_STD_MODULES: [(&str, &str); 14] = [
     ("$std.dom", nova_std::DOM),
     ("$std.ui", nova_std::UI),
     ("$std.ui.html", nova_std::UI_HTML),
+    ("$std.ui.backend", nova_std::UI_BACKEND_WASM),
     ("$std.rt", nova_std::RT),
 ];
 
@@ -2178,9 +2185,10 @@ pub const IMPORTED_STD: [(&str, &str); 3] = [
 /// std's imported modules, with 4.3's `ui` (spec 4.2 §3.2).
 pub const RESERVED_MODULE_NAMES: [&str; 2] = ["dom", "ui"];
 
-/// The std module whose public names only std's other modules see:
-/// `std/rt`, the runtime, whose functions `std/dom` calls (spec 4.2 §5).
-const STD_INTERNAL: &str = "$std.rt";
+/// The std modules whose public names only std's other modules see:
+/// `std/rt`, the runtime, whose functions `std/dom` calls (spec 4.2 §5),
+/// and `std/ui`'s backend, which has one source per target (spec 4.3 §7.3).
+pub const STD_INTERNAL: [&str; 2] = ["$std.rt", "$std.ui.backend"];
 
 /// `std/test`, seeded only under `nova test`. Kept out of [`STD_MODULES`] so
 /// that array's length does not depend on compilation mode: every consumer
@@ -3541,8 +3549,8 @@ mod tests {
 
     /// A WASM build compiles its own std (spec 4.0 §3.2, 4.1 §3.1, 4.2
     /// §3.2, 4.3 §3): ten of the native modules, in native's order, then
-    /// `std/dom`, `std/ui`, `std/ui/html` and `std/rt`, after the user's. A
-    /// native build has neither `std/dom` nor `std/rt`.
+    /// `std/dom`, `std/ui`'s three and `std/rt`, after the user's. A native
+    /// build has neither `std/dom` nor `std/rt`.
     #[test]
     fn a_wasm_build_compiles_wasms_std_modules() {
         let wasm = resolve_for(CompileTarget::Wasm, "fn main() { }\n");
@@ -3566,6 +3574,7 @@ mod tests {
                 "$std.dom",
                 "$std.ui",
                 "$std.ui.html",
+                "$std.ui.backend",
                 "$std.rt"
             ]
         );
@@ -3668,6 +3677,27 @@ mod tests {
         }
     }
 
+    /// `std/ui`'s backend is internal: `std/ui` sees its names, and a
+    /// program does not, even one that imports `ui` (spec 4.3 §7.3).
+    #[test]
+    fn uis_backend_is_seen_by_std_alone() {
+        for target in [CompileTarget::Native, CompileTarget::Wasm] {
+            let r = resolve_for(target, "import ui\nfn main() { }\n");
+            assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
+            assert!(r
+                .definitions
+                .resolve_value(ModuleId(0), "backend_query")
+                .is_none());
+            let ui = (1..=target.std_modules().len())
+                .find(|&m| r.definitions.module_name(ModuleId(m as u32)) == "$std.ui")
+                .expect("std/ui is compiled");
+            assert!(r
+                .definitions
+                .resolve_value(ModuleId(ui as u32), "backend_query")
+                .is_some());
+        }
+    }
+
     /// A WASM build knows the module of each public name of the std
     /// modules it does not compile, in every namespace; a native build
     /// compiles them all, and knows none (spec 4.1 §3.2).
diff --git a/std/lib.rs b/std/lib.rs
index cb811da..76527ef 100644
--- a/std/lib.rs
+++ b/std/lib.rs
@@ -2,7 +2,7 @@
 //! build time so the compiler stays one self-contained executable.
 //!
 //! Every `include_str!` path stays inside this package, so `cargo package`
-//! carries all 21 files, and its verification step builds the packaged copy
+//! carries all 23 files, and its verification step builds the packaged copy
 //! on its own (spec
 //! `docs/superpowers/specs/2026-10-07-phase-3-0-foundations-design.md` §3).
 //! `nova-resolver` gives each module its `$std.*` name and its place.
@@ -51,6 +51,14 @@ pub const TIME: &str = include_str!("time/lib.nova");
 pub const UI: &str = include_str!("ui/lib.nova");
 /// `std/ui/html/lib.nova`: the element builders, which a program imports.
 pub const UI_HTML: &str = include_str!("ui/html/lib.nova");
+/// `std/ui/backend/native.nova`, `std/ui`'s backend natively. Not in
+/// [`ALL`]: only std's modules see it (spec
+/// `docs/superpowers/specs/2026-10-17-phase-4-3-std-ui-reactivity-and-views-design.md`
+/// §7.3).
+pub const UI_BACKEND_NATIVE: &str = include_str!("ui/backend/native.nova");
+/// `std/ui/backend/wasm.nova`, `std/ui`'s backend on WASM, through
+/// [`DOM`]. Not in [`ALL`].
+pub const UI_BACKEND_WASM: &str = include_str!("ui/backend/wasm.nova");
 /// `std/rt/lib.nova`, the runtime of a WASM build, written in Nova. Not in
 /// [`ALL`]: only a WASM build compiles it, and its few public items are for
 /// std's other modules alone, so it is not documented (spec
@@ -59,7 +67,7 @@ pub const UI_HTML: &str = include_str!("ui/html/lib.nova");
 pub const RT: &str = include_str!("rt/lib.nova");
 
 /// Every documented module, by its directory under `std/`: all but
-/// [`RT`] and [`DOM`].
+/// [`RT`], [`DOM`] and `std/ui`'s backend.
 pub const ALL: [(&str, &str); 19] = [
     ("bytes", BYTES),
     ("collections", COLLECTIONS),
@@ -89,7 +97,7 @@ mod tests {
 
     /// Every `.nova` file under `std/` is embedded, and nothing else:
     /// [`super::ALL`]'s, `std/rt`'s as [`super::RT`], `std/dom`'s as
-    /// [`super::DOM`].
+    /// [`super::DOM`], and `std/ui`'s backend's two sources.
     #[test]
     fn every_std_module_is_embedded() {
         fn walk(dir: &Path, under: &str, out: &mut BTreeSet<String>) {
@@ -118,10 +126,18 @@ mod tests {
         let embedded: BTreeSet<String> = super::ALL
             .iter()
             .map(|(name, _)| format!("{name}/lib.nova"))
-            .chain(["rt/lib.nova", "dom/lib.nova"].map(String::from))
+            .chain(
+                [
+                    "rt/lib.nova",
+                    "dom/lib.nova",
+                    "ui/backend/native.nova",
+                    "ui/backend/wasm.nova",
+                ]
+                .map(String::from),
+            )
             .collect();
         assert_eq!(embedded, on_disk);
-        assert_eq!(on_disk.len(), 21);
+        assert_eq!(on_disk.len(), 23);
     }
 
     /// Each entry carries its own file, not a neighbour's.
@@ -133,7 +149,12 @@ mod tests {
                 .expect("read a std module");
             assert_eq!(source, on_disk, "std/{name}/lib.nova");
         }
-        for (path, source) in [("rt/lib.nova", super::RT), ("dom/lib.nova", super::DOM)] {
+        for (path, source) in [
+            ("rt/lib.nova", super::RT),
+            ("dom/lib.nova", super::DOM),
+            ("ui/backend/native.nova", super::UI_BACKEND_NATIVE),
+            ("ui/backend/wasm.nova", super::UI_BACKEND_WASM),
+        ] {
             let on_disk = std::fs::read_to_string(root.join(path)).expect("read a std module");
             assert_eq!(source, on_disk, "std/{path}");
         }
diff --git a/std/ui/backend/native.nova b/std/ui/backend/native.nova
new file mode 100644
index 0000000..47c93a2
--- /dev/null
+++ b/std/ui/backend/native.nova
@@ -0,0 +1,48 @@
+// `std/ui`'s backend natively, where there is no page (spec
+// docs/superpowers/specs/2026-10-17-phase-4-3-std-ui-reactivity-and-views-design.md
+// §7.3): each function panics with `mount`'s message, and only `mount`
+// reaches one. `std/ui/backend/wasm.nova` has the same names on WASM.
+
+/// A node of the page, which natively there is none of.
+pub record Node { unused: Int }
+
+fn no_page<T>() -> T {
+    panic("std/ui: mount needs a build for WASM; render_to_string renders on any target")
+}
+
+/// The first element `selector` matches.
+pub fn backend_query(selector: String) -> Option<Node> { no_page() }
+
+/// A new element.
+pub fn backend_element(tag: String) -> Node { no_page() }
+
+/// A new text node.
+pub fn backend_text(content: String) -> Node { no_page() }
+
+/// Replaces a text node's text.
+pub fn backend_set_text(node: Node, content: String) { no_page() }
+
+/// Sets an attribute.
+pub fn backend_set_attr(node: Node, name: String, value: String) { no_page() }
+
+/// Sets a property.
+pub fn backend_set_prop(node: Node, name: String, value: String) { no_page() }
+
+/// Adds `child` as `parent`'s last child.
+pub fn backend_append(parent: Node, child: Node) { no_page() }
+
+/// Adds `child` to `parent` before `before`.
+pub fn backend_insert_before(parent: Node, child: Node, before: Node) { no_page() }
+
+/// Removes a node from its parent.
+pub fn backend_remove(node: Node) { no_page() }
+
+/// The node after `node` in its parent.
+pub fn backend_next_sibling(node: Node) -> Option<Node> { no_page() }
+
+/// Whether two nodes are one.
+pub fn backend_is_same(a: Node, b: Node) -> Bool { no_page() }
+
+/// Calls `f` with the target's `value` on each event of kind `kind`, until
+/// the function returned is called.
+pub fn backend_listen(node: Node, kind: String, f: fn(String)) -> fn() { no_page() }
diff --git a/std/ui/backend/wasm.nova b/std/ui/backend/wasm.nova
new file mode 100644
index 0000000..b1cb2f3
--- /dev/null
+++ b/std/ui/backend/wasm.nova
@@ -0,0 +1,70 @@
+// `std/ui`'s backend on WASM: the page, through `std/dom` (spec
+// docs/superpowers/specs/2026-10-17-phase-4-3-std-ui-reactivity-and-views-design.md
+// §7.3). `std/ui/backend/native.nova` has the same names natively.
+
+import dom
+
+/// A node of the page.
+pub record Node { el: Element }
+
+fn wrap(e: Option<Element>) -> Option<Node> {
+    match e {
+        Some(x) => Some(Node { el: x })
+        None => None
+    }
+}
+
+/// The first element `selector` matches.
+pub fn backend_query(selector: String) -> Option<Node> { wrap(query_selector(selector)) }
+
+/// A new element.
+pub fn backend_element(tag: String) -> Node { Node { el: create_element(tag) } }
+
+/// A new text node.
+pub fn backend_text(content: String) -> Node { Node { el: create_text(content) } }
+
+/// Replaces a text node's text.
+pub fn backend_set_text(node: Node, content: String) { node.el.set_text(content) }
+
+/// Sets an attribute.
+pub fn backend_set_attr(node: Node, name: String, value: String) {
+    node.el.set_attribute(name, value)
+}
+
+/// Sets a property.
+pub fn backend_set_prop(node: Node, name: String, value: String) {
+    node.el.set_property(name, value)
+}
+
+/// Adds `child` as `parent`'s last child.
+pub fn backend_append(parent: Node, child: Node) { parent.el.append_child(child.el) }
+
+/// Adds `child` to `parent` before `before`.
+pub fn backend_insert_before(parent: Node, child: Node, before: Node) {
+    parent.el.insert_before(child.el, Some(before.el))
+}
+
+/// Removes a node from its parent.
+pub fn backend_remove(node: Node) { node.el.remove() }
+
+/// The node after `node` in its parent.
+pub fn backend_next_sibling(node: Node) -> Option<Node> { wrap(node.el.next_sibling()) }
+
+/// Whether two nodes are one.
+pub fn backend_is_same(a: Node, b: Node) -> Bool { a.el.is_same(b.el) }
+
+/// Calls `f` with the target's `value` on each event of kind `kind`, until
+/// the function returned is called.
+pub fn backend_listen(node: Node, kind: String, f: fn(String)) -> fn() {
+    let mut listener = node.el.add_event_listener(
+        kind,
+        |e: Event| {
+            match e.target() {
+                Some(t) => f(t.property("value"))
+                None => f("")
+            }
+        },
+    )
+    let stop = | | listener.remove()
+    stop
+}
diff --git a/std/ui/lib.nova b/std/ui/lib.nova
index 05a0300..e9dd417 100644
--- a/std/ui/lib.nova
+++ b/std/ui/lib.nova
@@ -4,13 +4,14 @@
 //! It is not in the prelude: a module that uses it imports it, with
 //! `import ui` for every name, or `import ui::{signal, effect}` for some.
 //! `import ui::html` brings the element builders, `div`, `button`, `text` and
-//! the rest, which make the `View`s that `render_to_string` writes as HTML.
+//! the rest, which make the `View`s that `mount` puts into the page and
+//! `render_to_string` writes as HTML.
 //!
 //! `set` runs the effects that read a signal before it returns, each once,
 //! and only after every memo it reads is current. A memo computes when it is
 //! made, then again when it is read after something it read changed. An
-//! effect, a memo and a `root` own the effects and memos made while they
-//! run: running again or being disposed disposes those first,
+//! effect, a memo, a `root`, and each `mount` own the effects and memos made
+//! while they run: running again or being disposed disposes those first,
 //! then runs the cleanups `on_cleanup` added, the last added first.
 //!
 //! The state behind all of this is the thread's, or the page's on WASM, and
@@ -446,6 +447,31 @@ pub fn on_mount(f: fn()) {
     }
 }
 
+// Run the `on_mount` callbacks from `start` on, in order, each with the owner
+// it was added in, so that what it makes is disposed with its render; skip
+// those whose owner is disposed; and drop them.
+fn run_mounted(start: Int) {
+    let waiting = ui().mounted
+    let mut later = Vec::new()
+    let mut i = start
+    while i < waiting.len() {
+        later.push(at(waiting, i))
+        i = i + 1
+    }
+    let mut w = waiting
+    while w.len() > start { let last = w.pop() }
+    let mut j = 0
+    while j < later.len() {
+        let m = at(later, j)
+        let live = match m.owner {
+            Some(o) => !o.disposed
+            None => true
+        }
+        if live { with_context(m.owner, None, m.f) }
+        j = j + 1
+    }
+}
+
 /// Runs `f` with a new owner, which no other owns, and gives `f` the
 /// function that disposes of it, in a `batch`; returns what `f` returns.
 pub fn root<T>(f: fn(fn()) -> T) -> T {
@@ -664,3 +690,117 @@ pub fn render_to_string(f: fn() -> View) -> String {
     s.rendering = s.rendering - 1
     joined(out)
 }
+
+/// Builds `f`'s view into the first element of the page `selector` matches,
+/// after its children, and keeps it up to date. It panics when no element
+/// matches, and natively, where there is no page.
+pub fn mount(selector: String, f: fn() -> View) {
+    let parent = match backend_query(selector) {
+        Some(p) => p
+        None => panic("std/ui: mount: no element matches \"${selector}\"")
+    }
+    let o = new_owner(ROOT, | | {})
+    let mut s = ui()
+    let mut kept = s.roots
+    kept.push(o)
+    let start = s.mounted.len()
+    s.building = s.building + 1
+    with_context(Some(o), None, | | build(f(), parent, None))
+    s.building = s.building - 1
+    run_mounted(start)
+}
+
+// Put `node` into `parent`, before `before` or last.
+fn put(parent: Node, node: Node, before: Option<Node>) {
+    match before {
+        Some(b) => backend_insert_before(parent, node, b)
+        None => backend_append(parent, node)
+    }
+}
+
+// Build `v`'s nodes into `parent`, before `before` or last, with an effect
+// for each part that changes.
+fn build(v: View, parent: Node, before: Option<Node>) {
+    match v {
+        Element(tag, attrs, children) => {
+            check_element(tag, attrs)
+            let el = backend_element(tag)
+            let mut i = 0
+            while i < attrs.len() {
+                build_attr(attrs[i], el)
+                i = i + 1
+            }
+            if !is_void(tag) {
+                let mut j = 0
+                while j < children.len() {
+                    build(children[j], el, None)
+                    j = j + 1
+                }
+            }
+            put(parent, el, before)
+        }
+        Text(t) => put(parent, backend_text(t), before)
+        DynText(f) => {
+            let n = backend_text("")
+            effect(| | backend_set_text(n, f()))
+            put(parent, n, before)
+        }
+        Region(f) => {
+            let start = backend_text("")
+            let end = backend_text("")
+            put(parent, start, before)
+            put(parent, end, before)
+            effect(| | build_region(f, parent, start, end))
+        }
+    }
+}
+
+// A region's render: what `f` gives, read as the effect running this, so
+// that its signals are the region's, built before `end` once the nodes
+// between the markers are removed. Each render has its own owner, which the
+// effect owns, so running again disposes of the last render first: its
+// effects, its handlers, and its `on_mount` callbacks not yet run. An
+// `on_mount` in the render waits for its nodes, and for the outer build's
+// when there is one.
+fn build_region(f: fn() -> [View], parent: Node, start: Node, end: Node) {
+    let mut s = ui()
+    let outer = s.building > 0
+    let begin = s.mounted.len()
+    s.building = s.building + 1
+    let render = new_owner(ROOT, | | {})
+    adopt(render)
+    let views = with_context(Some(render), s.listener, f)
+    let mut done = false
+    while !done {
+        match backend_next_sibling(start) {
+            Some(n) => { if backend_is_same(n, end) { done = true } else { backend_remove(n) } }
+            None => done = true
+        }
+    }
+    with_context(
+        Some(render),
+        None,
+        | | {
+            let mut i = 0
+            while i < views.len() {
+                build(views[i], parent, Some(end))
+                i = i + 1
+            }
+        },
+    )
+    s.building = s.building - 1
+    if !outer { run_mounted(begin) }
+}
+
+fn build_attr(a: Attr, el: Node) {
+    match a {
+        Static(name, value) => backend_set_attr(el, name, value)
+        Dynamic(name, f) => effect(| | backend_set_attr(el, name, f()))
+        Property(name, value) => backend_set_prop(el, name, value)
+        DynProperty(name, f) => effect(| | backend_set_prop(el, name, f()))
+        On(kind, f) => {
+            let stop = backend_listen(el, kind, |v: String| untracked(| | batch(| | f(v))))
+            on_cleanup(stop)
+        }
+    }
+}
````

### Task 5: The browser job clicks the builder counter too

Spec §9.4; decision 22.

**Files:**
- Modify: `.github/workflows/ci.yml`

- [ ] **Step 1: The job.** `patch_block t5-code`: the `browser` job builds
  `tests/dom/ui_counter.nova` beside `counter.nova`, and runs
  `tests/browser/counter.mjs` on each; its comment says why one script
  serves both.
- [ ] **Step 2: Click it here, where Playwright is installed.**

  ```sh
  rm -rf "$P/uic"
  cargo build --locked -p nova-cli
  target/debug/nova build --target wasm tests/dom/ui_counter.nova -o "$P/uic"
  PLAYWRIGHT_BROWSERS_PATH=/opt/pw-browsers node tests/browser/counter.mjs "$P/uic" /opt/node-tools
  ```

  Expected: `counter: three clicks, and the count reads 3`. Where
  `/opt/node-tools/node_modules/playwright` or `/opt/pw-browsers` does not
  exist, skip the step and say so in the ledger: CI's job is the check.
- [ ] **Step 3: Commit** `ci: the browser job clicks the builder counter too`.

**Patch `t5-code`:**

<!-- patch t5-code -->

````diff
diff --git a/.github/workflows/ci.yml b/.github/workflows/ci.yml
index 6dad933..968c20f 100644
--- a/.github/workflows/ci.yml
+++ b/.github/workflows/ci.yml
@@ -90,12 +90,14 @@ jobs:
         continue-on-error: true
         run: cargo test --locked --workspace --all-features --no-fail-fast -- --ignored
 
-  # 4.2's gate in a real browser (spec
+  # 4.2's and 4.3's gates in a real browser (spec
   # docs/superpowers/specs/2026-10-16-phase-4-2-std-dom-and-the-shim-design.md
-  # §10.3): the counter `cargo test` drives under the fake DOM, built for
-  # WASM and clicked in Chromium through Playwright. Playwright, pinned, is
-  # installed here only, into a scratch directory; nothing in `cargo test` or
-  # `nova` installs a package. Ubuntu only: one browser job suffices.
+  # §10.3, and 4.3's §9.4): the counters `cargo test` drives under the fake
+  # DOM, one written with std/dom and one with std/ui, built for WASM and
+  # clicked in Chromium through Playwright. Both have the ids `#add` and
+  # `#count`, so one script clicks either. Playwright, pinned, is installed
+  # here only, into a scratch directory; nothing in `cargo test` or `nova`
+  # installs a package. Ubuntu only: one browser job suffices.
   browser:
     name: Browser (Chromium)
     runs-on: ubuntu-latest
@@ -108,14 +110,18 @@ jobs:
           node-version: 22
       - name: cargo build -p nova-cli
         run: cargo build --locked -p nova-cli
-      - name: Build the counter for WASM
-        run: target/debug/nova build --target wasm tests/dom/counter.nova -o "$RUNNER_TEMP/counter"
+      - name: Build the counters for WASM
+        run: |
+          target/debug/nova build --target wasm tests/dom/counter.nova -o "$RUNNER_TEMP/counter"
+          target/debug/nova build --target wasm tests/dom/ui_counter.nova -o "$RUNNER_TEMP/ui_counter"
       - name: Install Playwright and Chromium
         run: |
           npm install --no-save --prefix "$RUNNER_TEMP/pw" playwright@1.56.1
           "$RUNNER_TEMP/pw/node_modules/.bin/playwright" install --with-deps chromium
-      - name: Click the counter in Chromium
-        run: node tests/browser/counter.mjs "$RUNNER_TEMP/counter" "$RUNNER_TEMP/pw"
+      - name: Click the counters in Chromium
+        run: |
+          node tests/browser/counter.mjs "$RUNNER_TEMP/counter" "$RUNNER_TEMP/pw"
+          node tests/browser/counter.mjs "$RUNNER_TEMP/ui_counter" "$RUNNER_TEMP/pw"
 
   fmt:
     name: Rustfmt
````

### Task 6: The records

Spec §11; every decision.

**Files:**
- Create: `docs/adr/0042-std-ui-reactivity-and-views.md`
- Modify: `nova-spec/30-FRONTEND.md`, `nova-spec/13-RUNTIME.md`,
  `nova-spec/20-STDLIB.md`, `docs/phase-4-plan.md`, `CHANGELOG.md`,
  `ARCHITECTURE.md`

- [ ] **Step 1: The records.** `patch_block t6-code`:
  - **ADR 0042:** the runtime's state, synchronous propagation with lazy
    memos, views as data and two renderers, the backend with two sources,
    nested imports for std, and the departures from `30-FRONTEND.md`,
    `root`'s result, `DynProperty`, `show`'s missing type parameter and
    `Signal<T>`'s reachable fields among them. It says that both renderers
    check every name and refuse `innerHTML`, `outerHTML` and `srcdoc`,
    that `render_to_string` escapes and `mount` sets values, never markup,
    that code in a value is the program's to place, and that `gc_collect`
    marks word 104.
  - **Dated notes,** each naming the spec and the ADR: `30-FRONTEND.md`
    §2, §4, §7 and §13; `13-RUNTIME.md` §8; `20-STDLIB.md`'s index; the
    phase plan's 4.3 entry, and its 4.4 entry, which gains the bundler's
    pieces.
  - **`CHANGELOG.md`:** `std/ui`, under `[Unreleased]`'s Added.
  - **`ARCHITECTURE.md`:** the `nova-std` row.
- [ ] **Step 2: The sizes ADR 0042 gives.**

  ```sh
  rm -rf "$P/sz" && mkdir -p "$P/sz"
  for n in counter ui_counter; do
    target/debug/nova build --target wasm tests/dom/$n.nova -o "$P/sz/$n" > /dev/null
    for f in app.wasm app.js; do echo "$n $f $(gzip -9 -c "$P/sz/$n/$f" | wc -c)"; done
  done
  ```

  Expected: `counter app.wasm 10134`, `counter app.js 4143`,
  `ui_counter app.wasm 16458` and `ui_counter app.js 4439`: the
  ADR's 10.1 and 4.1 KB, and 16.5 and 4.4 KB. A difference of more than
  0.1 KB is fixed in the ADR and recorded in the ledger.
- [ ] **Step 3: Commit** `docs: ADR 0042, and the notes for 4.3`.

**Patch `t6-code`:**

<!-- patch t6-code -->

````diff
diff --git a/ARCHITECTURE.md b/ARCHITECTURE.md
index 96d50e9..a5a53da 100644
--- a/ARCHITECTURE.md
+++ b/ARCHITECTURE.md
@@ -47,7 +47,7 @@ nova-mir            → Mid-level IR
 | `nova-codegen-llvm` | Optimized release backend |
 | `nova-codegen-wasm` | WebAssembly backend: MIR to one module with `wasm-encoder`, structured control flow, and the JS shim (ADR 0039) |
 | `nova-runtime` | GC, async runtime, panic handling |
-| `nova-std` (in `std/`) | The standard library's `.nova` sources, embedded for the compiler; `std/rt` is a WASM build's runtime, in Nova: its allocator, collector and executor (ADRs 0039, 0040); `std/dom`, a WASM build's page, which a program imports (ADR 0041) |
+| `nova-std` (in `std/`) | The standard library's `.nova` sources, embedded for the compiler; `std/rt` is a WASM build's runtime, in Nova: its allocator, collector and executor (ADRs 0039, 0040); `std/dom`, a WASM build's page, which a program imports (ADR 0041); `std/ui` and `std/ui/html`, reactivity and views, which a program imports, and `std/ui`'s backend, one source per target (ADR 0042) |
 | `nova-fmt` | The formatter: prints the AST in one fixed layout, keeps every comment, and refuses output that would change the program (ADR 0028); organize imports' block |
 | `nova-lsp` | The language server, `nova lsp`: diagnostics, completion, formatting, hover, definition, references, rename, code actions and semantic tokens over `nova_driver::analyze` (ADR 0029, ADR 0032, ADR 0033) |
 | `nova-test` | A stub, `publish = false`; the test runner is `nova-cli`'s `cmd/test.rs` |
diff --git a/CHANGELOG.md b/CHANGELOG.md
index 6ee1f49..32a2332 100644
--- a/CHANGELOG.md
+++ b/CHANGELOG.md
@@ -41,6 +41,14 @@ Nova uses [Semantic Versioning](https://semver.org/spec/v2.0.0.html).
   a listener, a timer or a `fetch` (ADR 0041).
 - **A CI job drives a WASM program in Chromium** through Playwright, which
   only that job installs.
+- **`std/ui`:** signals, effects and memos, with owners that dispose of
+  what they made and run its cleanups, and `batch` and `root`. `set` runs
+  the effects that read a signal before it returns, each once. Views are
+  values the builders of `std/ui/html` make, `div([class("x")],
+  [text("hi")])`, with `show` and `for_each` for parts that change;
+  `mount` keeps one up to date in the page on WASM, and `render_to_string`
+  writes one as escaped HTML on either target. A program imports them with
+  `import ui` and `import ui::html` (ADR 0042).
 
 ### Changed
 
diff --git a/docs/adr/0042-std-ui-reactivity-and-views.md b/docs/adr/0042-std-ui-reactivity-and-views.md
new file mode 100644
index 0000000..dafd9b0
--- /dev/null
+++ b/docs/adr/0042-std-ui-reactivity-and-views.md
@@ -0,0 +1,125 @@
+# ADR 0042 — `std/ui`: reactivity and views
+
+## Status
+
+Accepted, 2026-10-17 (Phase 4.3, branch
+`claude/loving-archimedes-frtf5q-43`; spec
+`docs/superpowers/specs/2026-10-17-phase-4-3-std-ui-reactivity-and-views-design.md`).
+Builders taking two arrays, effects run inside `set`, the runtime's
+implicit context, `import ui` and `import ui::html`, an unbounded
+`Signal<T>` whose `set` always notifies, `for_each` rebuilding its whole
+list, the bundler's pieces moving to 4.4, and `mount(selector, f)` with
+`render_to_string` are the user's decisions of 2026-10-16.
+
+## Context
+
+After 4.2 a WASM build could build a page and answer clicks through
+`std/dom`, one call at a time. `nova-spec/30-FRONTEND.md` §2 to §4 wants
+signals, effects and memos, and builder calls that keep a page up to date,
+but the language 4.3 starts from has:
+- no trait objects, so a signal's readers must be one record type;
+- no mutable globals, so the current computation must live somewhere the
+  runtime keeps;
+- no way to compare closures, so a reader is found again by an id;
+- one std source per module, the same on every target;
+- and `import` paths of one segment.
+
+## Decision
+
+1. **The runtime keeps `std/ui`'s state.** Three std-only builtins,
+   `ui_state_present`, `ui_state_load<T>` and `ui_state_store<T>`, keep one
+   record per thread, or per page on WASM:
+   - natively in a `thread_local` of `nova-runtime`, a collector root from
+     its first store;
+   - on WASM in `std/rt`'s state word 104, which `gc_collect` marks.
+   The current listener and owner, the queue of effects, and the roots
+   `mount` keeps all hang from it, so neither the program nor `std/ui`'s
+   API passes a context around.
+2. **Propagation is synchronous and glitch-free, and memos are lazy.**
+   - `set` marks what reads the signal dirty and what lies downstream of a
+     memo "check", then runs the queue of effects before it returns, unless
+     inside `batch` or another flush.
+   - An effect that is only "check" first brings the memos it read up to
+     date, and runs only if one of them ran, so a diamond runs once.
+   - A memo computes when it is made, as a signal needs a value, then again
+     only when something reads it after an input changed.
+   - `set` notifies every time, since most types have no `==`; a flush that
+     would run effects more than 100,000 times panics.
+   - Every computation, root and render is an `Owner`. Before it runs again,
+     or when it is disposed, it drops what it read, so that nothing its
+     cleanups set runs it again; then disposes of what it made, and runs its
+     cleanups, each in reverse order. A computation is marked clean before
+     that, and a disposed one is marked disposed, so a cleanup that reads it
+     gets its last value; a disposed owner never runs again. `root`'s
+     dispose function, an effect's run and a read that computes a memo are
+     each a batch.
+3. **Views are data.** `View` and `Attr` are sum types the builders make.
+   `render_to_string` walks a view into escaped HTML on any target; `mount`
+   walks it into DOM nodes on WASM, with an effect for each part that
+   changes. `show` and `for_each` are one primitive, a `Region` between two
+   empty text nodes, rebuilt whole each time its signal is set; each render
+   has its own owner, so a rebuild disposes of the last one's effects,
+   listeners and `on_mount` callbacks not yet run; a callback runs with
+   that owner, which owns what it makes.
+   - Both renderers check every tag, attribute and property name before
+     they write any, and refuse `innerHTML`, `outerHTML` and `srcdoc`,
+     which parse their value as HTML, as attributes or properties and in
+     any case: `std/ui` inserts no raw HTML.
+   - `render_to_string` escapes every text and attribute value; `mount`
+     hands values to the DOM as text and attribute values, never as markup.
+   - What is code is the program's to place: an attribute's value is
+     written as given, so what an `onclick` or a `javascript:` URL does is
+     the program's, as is the text of a `script` element it makes.
+4. **`std/ui`'s backend has two sources.** `$std.ui.backend` is
+   `std/ui/backend/wasm.nova` on WASM, calling `std/dom`, and
+   `std/ui/backend/native.nova` natively, where each function panics with
+   `mount`'s message. Like `std/rt`, it is internal: only std's modules see
+   its names. It is the first std module whose text differs by target. A
+   region finds its nodes by walking from its first marker to its second,
+   so a region inside another is removed with it; a listener's removal is a
+   cleanup of the owner that built it.
+5. **std's modules may have nested import paths.** `import ui::html` and
+   `import ui::html::{div}` name `$std.ui.html` through `IMPORTED_STD`; a
+   user's nested path is still E0900, and `import ui::nope` is E0001.
+6. **The departures from `30-FRONTEND.md`:**
+   - `Signal<T>` has no `Clone` bound: `value()` returns the signal's own
+     value;
+   - `memo` returns a `Signal<T>`, whose `set` panics;
+   - `root`, `batch` and `Signal::update` are added, and `root` returns
+     what its function returns, which is how a program keeps the dispose
+     function, since a closure captures by value;
+   - `mount(selector, f)`, not `mount(App)`;
+   - builders take positional arrays until 4.4's `view { … }`;
+   - reactive text is its own builder, `dyn_text`, as there is no
+     overloading, and a reactive property has its own variant,
+     `DynProperty`;
+   - handlers get the target's `value` as a `String`, not an event;
+   - `show` takes a `Signal<Bool>` and has no type parameter;
+   - `Signal<T>`'s fields are reachable, as Nova has no private fields;
+     writing one changes the value without telling anyone.
+
+## Consequences
+
+- **A page's whole graph is reachable from one root**, word 104 on WASM,
+  which `gc_collect` marks; 4.2's closure table needs nothing new: a
+  disposed owner is dropped by its parent, its listeners removed by its
+  cleanups, and the collector frees the rest and releases their handles.
+- **A new renderer, such as 4.6's hydration, walks the same `View`** and
+  needs a backend function only where it touches nodes.
+- **`std/ui` lands in every page that uses it.** The builder counter is
+  16.5 KB of WASM and 4.4 KB of `app.js` by `gzip -9`, where 4.2's counter
+  is 10.1 KB and 4.1 KB; 4.4's gate measures `06-counter-spa` against
+  50 KB.
+- **A keyed `for_each`, `untrack`, context, resources and error
+  boundaries** are not in 4.3; each can be added as new functions without
+  changing these.
+
+## References
+
+- `docs/phase-4-plan.md` §3, decisions 3, 5, 6, 7, 10 and 11, and §4's 4.3
+  entry.
+- The spec named above.
+- ADR 0038, the recorded scope; ADR 0040, the collector and the executor;
+  ADR 0041, `std/dom` and the boundary with JS.
+- `nova-spec/30-FRONTEND.md` §2 to §4 and §13; `nova-spec/13-RUNTIME.md`
+  §8; `nova-spec/20-STDLIB.md`.
diff --git a/docs/phase-4-plan.md b/docs/phase-4-plan.md
index f60e3ce..05bb13e 100644
--- a/docs/phase-4-plan.md
+++ b/docs/phase-4-plan.md
@@ -494,12 +494,27 @@ ADR 0041).
   - §13's unit tests pass on both targets: signal reactivity, effect
     cleanup, and memo memoization.
 
+**Amended 2026-10-17 (branch `claude/loving-archimedes-frtf5q-43`):** by the
+user's decisions of 2026-10-16, builders take two arrays; `set` runs its
+effects before it returns; the runtime keeps the current effect and owner;
+a program writes `import ui` and `import ui::html`; `Signal<T>` has no
+bound and `set` always notifies; `for_each` rebuilds its whole list;
+`mount(selector, f)` and `render_to_string(f)` are the renderers, the
+string one on both targets; and the bundler's pieces move to 4.4. `std/ui`
+reaches the page through a backend with one source per target. The gate
+is `tests/dom/ui_counter.nova` under the fake DOM and in Chromium, and
+`tests/runtime/ui_*.nova` on both targets
+(`docs/superpowers/specs/2026-10-17-phase-4-3-std-ui-reactivity-and-views-design.md`; ADR 0042).
+
 ### 4.4 — Component syntax
 - **RFC 0001 and its ADR** (decision 7).
 - **The language:** the lexer, parser, desugaring, type checker, formatter
   and the server's semantic tokens, and the extension's grammar.
 - **`examples/06-counter-spa`** as `60-EXAMPLES.md` §6 writes it, with
   decision 6's import line.
+- **The bundler's pieces 4.3 left** (amended 2026-10-17, branch
+  `claude/loving-archimedes-frtf5q-43`): a user's own `index.html`,
+  `assets/` from `public/`, `manifest.json`, and `wasm-opt`.
 - **Gate:**
   - `06-counter-spa` builds, and its counter responds to clicks in
     Chromium.
diff --git a/nova-spec/13-RUNTIME.md b/nova-spec/13-RUNTIME.md
index 47817f0..750bfd6 100644
--- a/nova-spec/13-RUNTIME.md
+++ b/nova-spec/13-RUNTIME.md
@@ -642,6 +642,15 @@ from JS, `nova_call` or `nova_wake`, is a turn
 (`docs/superpowers/specs/2026-10-16-phase-4-2-std-dom-and-the-shim-design.md`
 §4, §5, §7; ADR 0041).
 
+**Amended 2026-10-17 (branch `claude/loving-archimedes-frtf5q-43`):** 4.3
+keeps `std/ui`'s state, one record, for the thread or the page: natively in
+a `thread_local` of `nova-runtime`, a collector root from its first store;
+on WASM at `std/rt`'s state word 104, which `gc_collect` marks. Three
+std-only builtins, `ui_state_present`, `ui_state_load` and
+`ui_state_store`, reach it
+(`docs/superpowers/specs/2026-10-17-phase-4-3-std-ui-reactivity-and-views-design.md`
+§4, §8; ADR 0042).
+
 ---
 
 ## 9. Build Config Impact
diff --git a/nova-spec/20-STDLIB.md b/nova-spec/20-STDLIB.md
index 025ef56..c5470b6 100644
--- a/nova-spec/20-STDLIB.md
+++ b/nova-spec/20-STDLIB.md
@@ -58,6 +58,13 @@ Its functions are listed in
 `docs/superpowers/specs/2026-10-16-phase-4-2-std-dom-and-the-shim-design.md`
 §6 (ADR 0041).
 
+**AMENDED 2026-10-17 (branch `claude/loving-archimedes-frtf5q-43`): `std/ui`
+and `std/ui/html`**, on both targets, are imported too: `import ui` for
+reactivity and views, `import ui::html` for the element builders, std's
+only nested import path. Their functions are listed in
+`docs/superpowers/specs/2026-10-17-phase-4-3-std-ui-reactivity-and-views-design.md`
+§5 to §7 (ADR 0042).
+
 ---
 
 ## 2. `std/core` — Foundational Types
diff --git a/nova-spec/30-FRONTEND.md b/nova-spec/30-FRONTEND.md
index f371a74..4af08f8 100644
--- a/nova-spec/30-FRONTEND.md
+++ b/nova-spec/30-FRONTEND.md
@@ -45,6 +45,17 @@ pub fn on_cleanup(f: fn())
 
 Implementation: each signal owns a list of subscriber effects. `value()` registers current effect (tracked via thread-local "current effect" pointer). `set()` runs all subscribers.
 
+**Amended 2026-10-17 (branch `claude/loving-archimedes-frtf5q-43`):** 4.3
+builds `std/ui`, which a program imports with `import ui`, on both targets.
+`Signal<T>` has no `Clone` bound, and `set` notifies every time; `memo`
+returns a `Signal<T>` whose `set` panics; and `root`, `batch` and `update`
+are added, `root` returning what its function returns. `set` runs the
+effects that read the signal before it returns, each once and after the
+memos it reads are current; memos are lazy. The current effect and owner
+are in a record the runtime keeps for the thread or the page
+(`docs/superpowers/specs/2026-10-17-phase-4-3-std-ui-reactivity-and-views-design.md`
+§4, §5; ADR 0042).
+
 ---
 
 ## 3. Component Model
@@ -123,6 +134,18 @@ pub fn for_each<T>(items: Signal<[T]>, render: fn(T) -> View) -> View
 
 The `view { div { ... } }` macro-like syntax is parser-supported sugar that maps to these builders.
 
+**Amended 2026-10-17 (branch `claude/loving-archimedes-frtf5q-43`):** 4.3
+builds `std/ui/html`, imported with `import ui::html` or a list from it.
+An element takes its attributes and its children as two arrays, `[Attr]`
+and `[View]`, until 4.4's `view { … }`; `dyn_text` is reactive text, and
+`dyn_attr` and `dyn_value` reactive attributes and properties. `View` is
+data, which `mount(selector, f)` builds into the page on WASM and
+`render_to_string(f)` writes as escaped HTML on either target; neither
+inserts raw HTML, and both refuse `innerHTML`, `outerHTML` and `srcdoc`. `show` and `for_each` are in `std/ui`, and each rebuilds its
+whole part each time its signal is set; `show` has no type parameter
+(`docs/superpowers/specs/2026-10-17-phase-4-3-std-ui-reactivity-and-views-design.md`
+§6, §7; ADR 0042).
+
 ---
 
 ## 5. DOM Bindings (`std/dom`)
@@ -219,6 +242,13 @@ naming the module
 (`docs/superpowers/specs/2026-10-15-phase-4-1-the-language-on-wasm-design.md`
 §3, §5, §7; ADR 0040).
 
+**Amended 2026-10-17 (branch `claude/loving-archimedes-frtf5q-43`):** by the
+user's decision of 2026-10-16, a user's own `index.html`, `assets/` from
+`public/`, `manifest.json` and `wasm-opt` move from 4.3 to 4.4, with
+`examples/06-counter-spa`
+(`docs/superpowers/specs/2026-10-17-phase-4-3-std-ui-reactivity-and-views-design.md`
+decision 7).
+
 ### 7.1 Output Layout
 ```
 dist/
@@ -359,3 +389,10 @@ examples/
 - Integration: render component, simulate events (use `wasm-bindgen-test`)
 - E2E: Playwright tests against `nova dev` server
 - Visual regression: snapshot DOM state
+
+**Amended 2026-10-17 (branch `claude/loving-archimedes-frtf5q-43`):** the
+unit tests are `tests/runtime/ui_*.nova`, which run natively and on WASM,
+those that do not panic under each target's collector stress too;
+`tests/dom/ui_*.nova` render and click under the fake DOM, and CI's
+`browser` job clicks the builder counter in Chromium
+(`docs/superpowers/specs/2026-10-17-phase-4-3-std-ui-reactivity-and-views-design.md` §9).
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
  CARGO_INCREMENTAL=0 NOVA_REQUIRE_NODE=1 cargo test --locked --workspace --no-fail-fast > "$P/full-43x.txt" 2>&1
  grep '^test result' "$P/full-43x.txt" | awk '{p += $4; f += $6; i += $8} END {print p, f, i}'
  grep ' FAILED$' "$P/full-43x.txt"
  ```

  Expected:
  - both formatters print nothing, and clippy ends with `Finished`, with
    no warning or error;
  - `2286 1 10`, against the baseline's `2258 1 10`: 28 new tests, 19 in
    `wasm_tests`, four in `run_tests`, three in `nova-resolver`, and one
    each in `lower_tests` and `nova-runtime`;
  - the one failure is Task 0's IPv6 test.
- [ ] **Step 2: The lockfile.** `git diff --numstat 87cb0a8 -- Cargo.lock`.
  Expected: nothing.
- [ ] **Step 3: Rust 1.78.**

  ```sh
  RUSTFLAGS="-D warnings" CARGO_TARGET_DIR="$P/msrv-43" cargo +1.78 check --locked --workspace
  ```

  Expected: clean, in about a minute. Then `rm -rf "$P/msrv-43"`.
- [ ] **Step 4: The mutants.** One at a time, each edit is made, its tests
  run, and the file restored with `git checkout -- <file>`.
  - Each edit replaces one exact text, which occurs once in its file; "the
    line goes" removes the whole line holding it.
  - Each must fail the tests named. A mutant that passes them is a gap:
    record it in the ledger, and write the missing test before the
    review.
  - At the end, `git status --short` prints nothing.

  Each command runs one test filter: cargo's filters are joined by "or",
  so a second name after `W`'s own would widen the run, not narrow it.
  `N` stands for `CARGO_INCREMENTAL=0 NOVA_REQUIRE_NODE=1 cargo test -p
  nova-cli --test wasm_tests`, and `C` for `CARGO_INCREMENTAL=0 cargo test
  -p nova-cli --test run_tests`.

  1. **`ui::nope` is E0900.** `crates/nova-resolver/src/lib.rs`, in
     `resolve_import`:
     `if IMPORTED_STD.iter().any(|&(n, _)| n == first) {` becomes
     `if false {`.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-resolver --lib a_nested_import_path_names_only_stds_modules`,
       then `N ui_imports_check_on_both_targets`.
     - Expected: both fail; the first with `left: ["E0900"]`, `right:
       ["E0001"]`, and the second at `nope.nova`.
  2. **The backend seen by every module.** `crates/nova-resolver/src/lib.rs`:
     `pub const STD_INTERNAL: [&str; 2] = ["$std.rt", "$std.ui.backend"];`
     becomes `pub const STD_INTERNAL: [&str; 2] = ["$std.rt", "$std.rt"];`.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-resolver --lib uis_backend_is_seen_by_std_alone`.
     - Expected: fails at `r.definitions.resolve_value(ModuleId(0),
       "backend_query").is_none()`.
  3. **Natively, the state is no root.** `crates/nova-runtime/src/ui.rs`:
     the line `    gc::add_root(state);` goes.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-runtime --lib ui::`,
       then `C ui_programs_run_and_survive_gc_stress`.
     - Expected: both fail; the first with `left: 0`, `right: 1`, and the
       second because `ui_signal` under `NOVA_GC_STRESS` is killed by a
       signal partway through.
  4. **On WASM, word 104 is not marked.** `std/rt/lib.nova`, in
     `gc_collect`: `    if u != 0 { gc_mark(m, u) }` becomes
     `    if u == -1 { gc_mark(m, u) }`.
     - Run `N stress::ui_collect`.
     - Expected: fails; `node` reports `nova: trap: unreachable`.
  5. **On WASM, a state builtin is left to the shim.**
     `crates/nova-mir/src/lib.rs`:
     `            RtFunc::UiStateLoad => WasmRuntime::Nova("rt_ui_state_load"),`
     becomes
     `            RtFunc::UiStateLoad => WasmRuntime::Host("ui_state_load"),`.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-mir --test lower_tests the_ui_state_builtins_lower_to_the_runtime_or_std_rt`.
     - Expected: fails at `lower_tests_load`, `left: 1`, `right: 0`.
  6. **A "check" effect never brings its memos up to date.**
     `std/ui/lib.nova`, in `refresh`:
     `        while i < o.sources.len() && o.state == CHECK {` becomes
     `        while false {`.
     - Run `C ui_programs_run_and_survive_gc_stress`.
     - Expected: fails at `ui_signal`: after `diamond: 2 2` the diamond
       prints nothing more.
  7. **A memo's readers never hear that its inputs changed.**
     `std/ui/lib.nova`, in `mark`:
     `            Some(src) => mark_all(src, CHECK)` becomes
     `            Some(src) => {}`.
     - Run `C ui_programs_run_and_survive_gc_stress`.
     - Expected: fails at `ui_signal`, as mutant 6.
  8. **A memo that ran leaves its readers "check".** `std/ui/lib.nova`, in
     `run_owner`: `            Some(src) => mark_all(src, DIRTY)` becomes
     `            Some(src) => {}`.
     - Run `C ui_programs_run_and_survive_gc_stress`.
     - Expected: fails at `ui_signal`, as mutant 6.
  9. **A computation keeps what it read on its last run.**
     `std/ui/lib.nova`, in `clean`:
     `        drop_observer(at(read, k), o.id)` becomes
     `        let unused = at(read, k)`.
     - Run `C ui_programs_run_and_survive_gc_stress`.
     - Expected: fails at `ui_signal`: `branch b: b1` twice, as `a.set("a3")`
       still runs the effect.
  10. **Cleanups run in the order they were added.** `std/ui/lib.nova`, in
      `clean`: `        let f = at(ups, j)` becomes
      `        let f = at(ups, ups.len() - 1 - j)`.
      - Run `C ui_programs_run_and_survive_gc_stress`.
      - Expected: fails at `ui_cleanup`: `first cleanup of 1` before
        `second cleanup of 1`.
  11. **A disposed owner's children are left alone.** `std/ui/lib.nova`, in
      `clean`: `        dispose(at(kids, i))` becomes
      `        let unused = at(kids, i)`.
      - Run `C ui_programs_run_and_survive_gc_stress`.
      - Expected: fails at `ui_cleanup`: no `inner cleanup of` line.
  12. **No settle limit: a loop just stops.** `std/ui/lib.nova`, in `flush`:
      `            if runs > SETTLE { panic("std/ui: effects did not settle") }`
      becomes `            if runs > SETTLE { s.queue.clear() }`.
      - Run `C ui_programs_panic_with_their_message`.
      - Expected: fails with `Unexpected success` for `ui_loop`.
  13. **An attribute value's quote is not escaped.** `std/ui/lib.nova`, in
      `escape`: `        } else if attribute && c == '"' {` becomes
      `        } else if false {`.
      - Run `C ui_programs_run_and_survive_gc_stress`.
      - Expected: fails at `ui_render`: `title="say "hi" …`.
  14. **Any tag name is accepted.** `std/ui/lib.nova`, in `check_name`:
      `    if !ok { panic("std/ui: \"${name}\" is not ${what}") }` becomes
      `    if false { panic("std/ui: \"${name}\" is not ${what}") }`.
      - Run `C ui_programs_panic_with_their_message`.
      - Expected: fails with `Unexpected success` for `ui_render_bad_name`.
  15. **A region keeps its old nodes.** `std/ui/lib.nova`, in
      `build_region`:
      `            Some(n) => { if backend_is_same(n, end) { done = true } else { backend_remove(n) } }`
      becomes `            Some(n) => done = true`.
      - Run `N ui_show_and_for_each_rebuild_their_part`.
      - Expected: fails on the page's text: the old rows stay beside the
        new.
  16. **A rebuilt row's listener is never removed.** `std/ui/lib.nova`, in
      `build_attr`: the line `            on_cleanup(stop)` goes.
      - Run `N ui_show_and_for_each_rebuild_their_part`.
      - Expected: fails with the calls in use `[5, 5, 8, 12, 16, 16, 16]`,
        where `[5, 5, 6, 7, 7, 7, 7]` is right.
  17. **A region's views are made before its build begins.**
      `std/ui/lib.nova`, in `build_region`: the line
      `    s.building = s.building + 1` moves from before
      `    let render = new_owner(ROOT, | | {})` to after
      `    let views = with_context(Some(render), s.listener, f)`; the four
      lines, from the first to the last, occur once.
      - Run `N ui_show_and_for_each_rebuild_their_part`.
      - Expected: fails: `on_mount: the panel is not in the page`.
  18. **A nested path's first segment is no occurrence.**
      `crates/nova-resolver/src/lib.rs`, in `resolve_import`:
      `                .and_then(|&(_, std_name)| modules.iter().position(|m| m.name == std_name))`
      becomes `                .and_then(|_| None::<usize>)`.
      - Run `CARGO_INCREMENTAL=0 cargo test -p nova-driver --test index_complete`.
      - Expected: fails: ``tests/runtime/ui_render.nova:5:8 `ui`: no
        occurrence``, and the same in `ui_render_bad_name.nova`.
  19. **`render_to_string` runs `on_mount`.** `std/ui/lib.nova`, in
      `on_mount`: the line `    if s.rendering > 0 { return }` goes.
      - Run `C ui_programs_run_and_survive_gc_stress`.
      - Expected: fails at `ui_render`: `on_mount ran` is printed.
  20. **`root`'s dispose function does nothing.** `std/ui/lib.nova`, in
      `root`:
      `    with_context(Some(o), None, | | f(| | batch(| | dispose(o))))`
      becomes `    with_context(Some(o), None, | | f(| | {}))`.
      - Run `C ui_programs_run_and_survive_gc_stress`.
      - Expected: fails at `ui_cleanup`: `run 3` after the dispose, and no
        `root cleanup`.
  21. **A disposed owner is not marked so.** `std/ui/lib.nova`, in
      `dispose`: the line `    m.disposed = true` goes.
      - Run `C ui_programs_run_and_survive_gc_stress`.
      - Expected: fails at `ui_cleanup`: `child of 1 sees 2` after `child
        of 2 sees 2`, the disposed child run by the flush that queued it.
  22. **`mount` writes names unchecked.** `std/ui/lib.nova`, in `build`:
      the two lines `            check_element(tag, attrs)` and
      `            let el = backend_element(tag)` become the second alone.
      - Run `N ui_mount_never_inserts_html`.
      - Expected: fails: the panic is `std/dom: dom_set_property:
        TypeError: Cannot set property innerHTML of #<FakeElement> which
        has only a getter`, the fake DOM refusing what a browser would
        parse.
  23. **No name is refused for parsing HTML.** `std/ui/lib.nova`, in
      `check_attr`:
      `    if n == "innerhtml" || n == "outerhtml" || n == "srcdoc" {` becomes
      `    if false {`.
      - Run `C ui_checks_every_name_and_never_parses_html`.
      - Expected: fails with `Unexpected success`, at `bad3.nova`.
  24. **An attribute's name is not checked.** `std/ui/lib.nova`, in
      `check_element`:
      `            Static(name, value) => check_attr(name, "an attribute name")`
      becomes `            Static(name, value) => {}`.
      - Run `C ui_checks_every_name_and_never_parses_html`.
      - Expected: fails with `Unexpected success`, at `bad0.nova`.
  25. **A handler runs outside a batch.** `std/ui/lib.nova`, in
      `build_attr`: `|v: String| untracked(| | batch(| | f(v)))` becomes
      `|v: String| untracked(| | f(v))`.
      - Run `N ui_attributes_follow_their_signals`.
      - Expected: fails on the page's output: `the sum is 1` before `the
        sum is 3`.
  26. **A handler subscribes the computation that made its event.**
      `std/ui/lib.nova`, in `build_attr`:
      `|v: String| untracked(| | batch(| | f(v)))` becomes
      `|v: String| batch(| | f(v))`.
      - Run `N ui_attributes_follow_their_signals`.
      - Expected: fails: `an effect clicks the probe` and `the probe sees
        1` again before `seen is set`.
  27. **An effect's first run is no batch.** `std/ui/lib.nova`, in
      `effect`: `    batch(| | run_owner(o))` becomes `    run_owner(o)`.
      - Run `C ui_programs_run_and_survive_gc_stress`.
      - Expected: fails at `ui_signal`: `p + q: 1` between `p + q: 0` and
        `p + q: 3`, all before `the first run is done`.
  28. **Dropping an observer moves the last into its place.**
      `std/ui/lib.nova`, in `drop_observer`: the four lines from
      `    while i + 1 < n {` to its closing `    }` become
      `    obs.set(i, at(obs, n - 1))`.
      - Run `C ui_programs_run_and_survive_gc_stress`.
      - Expected: fails at `ui_signal`: `third sees 1` before `second sees
        1`.
  29. **A disposed render's `on_mount` still runs.** `std/ui/lib.nova`, in
      `run_mounted`: `        if live { with_context(m.owner, None, m.f) }`
      becomes `        with_context(m.owner, None, m.f)`.
      - Run `N ui_mount_runs_on_mount_and_panics_without_a_match`.
      - Expected: fails: `on_mount of row 1: not in the page`.
  30. **`root`'s dispose function is no batch.** `std/ui/lib.nova`, in
      `root`:
      `    with_context(Some(o), None, | | f(| | batch(| | dispose(o))))`
      becomes `    with_context(Some(o), None, | | f(| | dispose(o)))`.
      - Run `C ui_programs_run_and_survive_gc_stress`.
      - Expected: fails at `ui_cleanup`: `elsewhere sees 1` before `last
        cleanup of 0`.
  31. **A region's render has no owner of its own.** `std/ui/lib.nova`, in
      `build_region`:
      `    let views = with_context(Some(render), s.listener, f)` becomes
      `    let views = f()`, so the views' `on_mount` callbacks belong to the
      region's effect, which is never disposed.
      - Run `N ui_mount_runs_on_mount_and_panics_without_a_match`.
      - Expected: fails as mutant 29.
  32. **A memo is marked clean only after its last run is disposed.**
      `std/ui/lib.nova`, in `run_owner`: the three lines
      `    let mut m = o`, `    m.state = CLEAN` and `    clean(o)` become
      `    clean(o)`, `    let mut m = o` and `    m.state = CLEAN`.
      - Run `C ui_programs_run_and_survive_gc_stress`.
      - Expected: fails at `ui_memo`: `tens computes from 2`, then `the
        cleanup of 1 reads 20`: the cleanup's read ran the memo inside its
        own run.
  33. **A read that computes a memo is no batch.** `std/ui/lib.nova`, in
      `Signal::value`:
      `            Some(m) => { if m.state != CLEAN { batch(| | refresh(m)) } }`
      becomes `            Some(m) => refresh(m)`.
      - Run `C ui_programs_run_and_survive_gc_stress`.
      - Expected: fails at `ui_memo`: `the memo's child of 1 sees x=1`,
        the child its run was disposing.
  34. **`dispose` marks the owner only after it cleans it.**
      `std/ui/lib.nova`, in `dispose`: the three lines `    let mut m = o`,
      `    m.disposed = true` and `    clean(o)` become `    clean(o)`,
      `    let mut m = o` and `    m.disposed = true`.
      - Run `C ui_programs_run_and_survive_gc_stress`.
      - Expected: fails at `ui_memo`: `hundreds computes from 2`, then
        `the disposed memo reads 200`.
  35. **A waiting `on_mount` runs with whatever owner is current.**
      `std/ui/lib.nova`, in `run_mounted`:
      `        if live { with_context(m.owner, None, m.f) }` becomes
      `        if live { untracked(m.f) }`.
      - Run `N ui_mount_runs_on_mount_and_panics_without_a_match`.
      - Expected: fails: no `row 2 is gone`, as the cleanup that row 2's
        `on_mount` added belongs to no owner.
  36. **`srcdoc` is allowed.** `std/ui/lib.nova`, in `check_attr`:
      `    if n == "innerhtml" || n == "outerhtml" || n == "srcdoc" {`
      becomes `    if n == "innerhtml" || n == "outerhtml" {`.
      - Run `C ui_checks_every_name_and_never_parses_html`.
      - Expected: fails with `Unexpected success`, at `bad5.nova`.
  37. **A refused name is compared with its case.** `std/ui/lib.nova`, in
      `check_attr`: `    let n = name.to_lower()` becomes
      `    let n = name`.
      - Run `C ui_checks_every_name_and_never_parses_html`.
      - Expected: fails with `Unexpected success`, at `bad3.nova`, whose
        `innerHTML` no longer equals `innerhtml`.
  38. **`clean` drops what the owner read only after its cleanups run.**
      `std/ui/lib.nova`, in `clean`: the six lines from
      `    let read = drain(o.sources)` to the first loop's closing `    }`
      move to the end of the function, after the cleanups' loop.
      - Run `C ui_programs_run_and_survive_gc_stress`.
      - Expected: fails at `ui_cleanup`, which panics with `std/ui:
        effects did not settle`: each run's cleanup sets `count`, which
        the effect still reads, and queues it again.

  A first draft of mutant 6 made a "check" effect run without its check
  (`if o.state != CLEAN` in place of `if o.state == DIRTY`), and it passed:
  every value a memo computes counts as a change, so a "check" effect
  always finds a memo that ran, and runs anyway. The check is what orders
  the memos first, which today's mutant 6 removes.

  `ui_cleanup`'s case of a disposed root whose cleanup sets what its
  effect read guards against decision 23's three guards undone together,
  as in the first draft; mutants 30, 34 and 38 each undo one, and each
  fails on a case of its own.
- [ ] **Step 5: The review's inputs.** The builder counter's sizes, as
  Task 6 measured them, and the full run's figures go in the ledger.
- [ ] **Step 6: One fresh read-only review, then one fix pass** (the
  user's workflow).
  - The reviewer reads `git diff $BASE..HEAD` (Task 0) against the spec and
    this plan, and is told to hunt for:
    - a part of the graph only the state's root keeps, which the collector
      can free, natively or on WASM;
    - a computation that runs twice for one `set`, runs with a stale
      input, or runs after its owner was disposed; a cleanup that never
      runs, or runs out of order;
    - a node a region leaves behind, a listener a disposed render keeps,
      and an `on_mount` that runs before its nodes are in the page, or
      after they are gone;
    - a value that breaks out of its markup, a name that is not checked,
      and any way to put raw HTML in the page;
    - a handler that subscribes what is running, or is not in a batch;
    - a nested import path a program can write beyond std's, and a std
      name a program can reach;
    - a test that could pass without the code it names;
    - a record that says what the code does not.
  - Each finding is fixed in one commit, `std/ui: the review's fixes`, or
    answered in the ledger with the reason it stands.
  - After the fixes, Step 1 runs again.
- [ ] **Step 7: The PR.**
  - Push: `git push -u origin claude/loving-archimedes-frtf5q-43`.
  - Open the PR to `main` with the repository's template headings, if it
    has one. The body names no local path and no person.
  - It holds:
    - what 4.3 adds;
    - the gate's evidence: the builder counter under the fake DOM, as is
      and under stress, and in Chromium in the `browser` job; the
      `tests/runtime/ui_*` programs on both targets; the sizes;
    - the decisions, numbered as here, with 1 to 5, 11 to 13 and 25 marked
      as differing from the spec;
    - that `Cargo.lock` is unchanged;
    - the mutants, and the review's findings and what came of each.
  - Read CI once it ends. Expected: green on all three systems, and the
    `browser` job green; the test job runs the WASM tests under Node 22
    with `NOVA_REQUIRE_NODE=1`.
- [ ] **Step 8: The merge,** only on the user's word.
  - By rebase, pinned to the PR's head commit. Never auto-merge.
  - Then `git fetch origin main` and `git diff <head> origin/main
    --stat`, which prints nothing: the merged tree is the PR's.
