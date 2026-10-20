# Phase 4.6, "SSR, hydration and SSG", Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** a page is rendered on a server and comes alive in the browser, and a
site is rendered ahead of time. A project has two entries that share their
components through a module such as `src/app.nova`: `src/main.nova`, the
client, built for WASM, and `src/server.nova`, the server, built natively.
`std/ui`'s `render_hydratable` writes a view as HTML with comment markers
around each `show` and `for_each` and between two texts side by side;
`hydrate("#app", App)` walks the view against those nodes in order and adopts
them, starting their effects and handlers, and at the first difference reports
it with `console.error` and builds the view again; `hydrate_with` first reads
the server's data, JSON in the `<script type="application/json"
id="nova-data">` just after the root, with `FromJson`, or takes the fallback
the program gives when the page holds none, as under `nova dev`. The
native-only `ui::ssr` renders the page at a location (`page`, `page_with`,
`document`) and serves a built `dist/` (`serve`): the files its manifest
lists, and every other `GET` as a page inside `index.html`'s `<div id="app">`,
emptied first, with `404` where only the catch-all route serves the path.
`std/ui/router` gains `with_location`, `found` and `static_paths`, and
`std/http` `Server::fallback` and `listen_on`. `nova build --ssr` builds the
client into `dist/` and the server into `target/<profile>/<name>-server`;
`--ssg` then runs the server with `--ssg <dist>`, which writes
`dist/<path>/index.html` for each static route, and lists those pages in
`manifest.json`. The gate, in CI's `browser` job through Playwright:
`tests/browser/ssr-app`, built with `--ssg`, hydrates in Chromium without
making its nodes again, a page its server renders and a page SSG wrote alike,
and a click updates the server's nodes.

**Architecture:**
- **`std/ui/lib.nova`:** `render_hydratable` beside `render_to_string`, both
  through one writer that takes `marks`; `hydrate`, `hydrate_with` with its
  fallback, `root_of` and `hydrate_into`; the walk: `Walk`, `Cursor`, `Found`,
  `blank`, `advance`, `is_marker`, `clipped`, `found`, `missed`, `left_over`,
  `walk_views`, `walk_view`, `adopt_attr`, `adopt_text` and `adopt_region`.
- **`std/ui/backend/{wasm,native}.nova`:** `backend_first_child`,
  `backend_kind`, `backend_text_of`, `backend_report` and
  `backend_data(root)`, which reads the root's next element, on WASM;
  natively each panics with `mount`'s message.
- **`std/dom`, the shim:** `console_error`, and the shim's
  `dom_console_error`.
- **`std/ui/router/lib.nova`:** `with_location`, `found` and
  `static_paths`; `Routing.location`, `starting`, `first_match` and
  `catch_all`.
- **`std/http/lib.nova`:** `Server.unmatched`, `fallback` and `listen_on`;
  `listen` binds, then calls `listen_on`.
- **`std/ui/ssr/lib.nova`:** the module, whose `serve` drops a manifest's
  path that could leave `dist` (`inside`, `plain_path`, `plain_piece`) and
  whose `document` empties the root it fills, registered in `std/lib.rs`, the
  resolver's `STD_MODULES` and `IMPORTED_STD` (not `WASM_STD_MODULES`),
  whose `missing_names` now leaves an imported module's names out, and
  `nova-doc`'s list of modules.
- **`nova-driver`:** `Roots::Server`, a project's program rooted at
  `src/server.nova`.
- **`nova-cli`:** `cmd/run.rs`'s `--ssr` and `--ssg`, `target:
  Option<TargetArg>`, `build_client`, `build_ssr` and `generate`, with
  `page_in`, which lists a page only by plain names inside the client's
  directory, both made absolute from the server's working directory, and
  `ended`, a failing server's end in the same words on
  every system;
  `bundle.rs`'s `Listing`, which `finish` now returns, `add_pages` and
  `write_manifest`.
- **Tests:** `tests/runtime/ui_hydratable*`, `ui_router_server*`,
  `http_listen_on*` and `http_server_dispatch*`; the projects
  `tests/dom/hydrate/` and `tests/dom/hydrate_mismatch/` with their
  harnesses, and `fake_dom.mjs`; the project `tests/ssr/site/`;
  `crates/nova-cli/tests/ssr_tests.rs`; and `tests/browser/ssr.mjs` and
  `ssr-app/` in the `browser` job.
- **Records:** ADR 0045, and dated notes.

**Tech Stack:**
- Rust: MSRV 1.78, edition 2021.
- **No new crate; `Cargo.lock` does not change** (spec §10). The server is
  `std/http`'s `Server` on `std/net`, in Nova; the CLI's run of the server
  is `std::process`.
- JS: ES modules run by Node 22 for the fake-DOM harnesses, with 4.2's
  fake DOM given comments, an `innerHTML` parser and a console that keeps
  its errors.
- Playwright 1.56.1, installed by the CI job only.
- Tests: unit tests in `nova-resolver` and `nova-cli`'s `bundle.rs` and
  `cmd/run.rs`;
  `crates/nova-cli/tests/run_tests.rs`, `wasm_tests.rs`, `doc_test.rs` and
  `ssr_tests.rs`; `crates/nova-driver/tests/index_complete.rs` and
  `packages.rs`; programs in `tests/runtime/`, projects in `tests/dom/` and
  `tests/ssr/`; doc tests in `std/ui`, `std/ui/router`, `std/ui/ssr` and
  `std/http`.

**Spec:** `docs/superpowers/specs/2026-10-19-phase-4-6-ssr-hydration-and-ssg-design.md`,
approved by the user on 2026-10-19, with its §6.2 escapes restored the same
day (`a5c0335`).
- Read it before Task 1: it is the authority this plan argues from.
- Its header holds the user's twelve decisions, its §14 the decisions made
  while writing it, and its §2 the code as it stood at `2b7a6ed`. This
  plan's decisions say where the code departs from the spec, or fills in
  what it left open.
- The user's thirteenth decision came on 2026-10-20, after the fact-check
  below: `hydrate_with` takes a fallback, `hydrate_with("#app", Data {
  names: [] }, App)`, which it uses when the page holds no data from the
  server, so `nova dev` shows the page. It adds a parameter to the spec's
  API; decision 29 builds it, and ADR 0045 records it.

**Checked on 2026-10-19 and 2026-10-20, before execution:**
- **A prototype built 4.6 whole** on a local branch from `f8c7d0f`, the
  approved spec, in this tree, in three parts, then its records:
  - part A, hydration: `render_hydratable`, `hydrate` and `hydrate_with`,
    the backends, `console_error`, the fake DOM's parser, and the
    hydration tests; 20 breakages of its code, each made on purpose, failed
    its tests;
  - part B, the server: the router's three functions, `ui::ssr`, `std/http`'s
    two methods, the resolver, the driver and the CLI, and their tests; 21
    breakages failed its tests;
  - part C, the gate: `tests/browser/ssr-app` and `ssr.mjs`, which failed
    when the client mounted instead of hydrating (a timeout) and when the
    server's data differed from the client's (`34 nodes became 30`); then
    the records. Part C read every departure of A and B against the code.

  The parts' departures from the spec, and what they found, are this
  plan's decisions 13 to 31; the fact-check's fixes are decisions 29 and
  32 to 39.
- **A fact-check** read this plan's first draft against the spec, the code
  and the replay, without changing anything. It confirmed hydration in
  Chromium, on many views a browser's parser rebuilds, the walk, the
  mismatch and the collector, the escaping, the router's functions,
  `std/http`, the driver, the CLI's refusals, SSG's paths on Unix, MSRV,
  every "Expected:", the figures, and the patches. It found nine things,
  which the prototype's `3f1f035 4.6 prototype: the fact-check's fixes`
  and `aebb875 4.6 prototype: CI builds the runtime library, and the
  records` answer:
  1. **`nova dev` and `hydrate_with`:** the first draft's decision 29 let
     `hydrate_with` panic under `nova dev`, overriding the user's decision
     10. Asked, the user made decision 13: a fallback (decision 29).
  2. **The data's element:** `backend_data` read the page's first
     `#nova-data`, which a view's own element with that id, inside the
     root, could be. Now it reads the root's next element alone, a
     `script` with that id and `type` (decision 32).
  3. **A page outside `dist/`:** `--ssg` took a server's `ssg: wrote
     dist/../nova.toml` for the page `../nova.toml`, which `serve` then
     served at `/..%2fnova.toml`. Now `serve` drops such a manifest path,
     and `--ssg` lists a page only by plain names inside the directory,
     failing the build on any other line (decisions 33 and 39).
  4. **`document` and the records:** `document` panicked on a template
     whose root held whitespace or a placeholder, which `serve` emptied and
     served. Now `document` empties it too, and panics only with no `<div
     id="app">` (decision 34), and a root never closed is no error
     (decision 38). The records said a failing server ends `nova` with its
     status, not the 1 after a signal; ADR 0045 cited a wrong decision for
     the hydrate panic and left out the line feed a `pre` or `textarea`
     drops: fixed.
  5. **A whitespace root:** its whitespace stayed, before the view built
     after it. Now it is emptied first (decision 35).
  6. **`found` and a fragment:** `GET /about#x` rendered About and
     answered `404`. Now `found` drops the fragment (decision 36).
  7. **Test gaps:** `serve`'s decoding and query, a listed file not on
     disk, a `<divx`, the `a comment` reading and a form feed in an empty
     root had no test. Now `/app%2ejs`, `/app.js?v=1` and `/gone.js`,
     `document`'s `divx` line, `hydrate_mismatch`'s `comment` root and
     `hydrate`'s `feed` root test them.
  8. **Windows:** four checks in `ssr_tests.rs` would fail there: the
     expected server's path written with `/`, and `ExitStatus`'s words,
     `exit code: 3` there and `exit status: 3` here. Now the paths are
     joined by components, and `nova` says `with exit code <code>` or
     `stopped by signal <number>` on every system (decision 37).
  9. **CI's `browser` job** built `-p nova-cli` alone, so a debug `nova`
     found no `libnova_runtime.a` beside it, and `ssr.mjs`'s `nova build
     --ssg` could not link the server; the replay had passed on Task 0's
     workspace build. Now the job builds `-p nova-cli -p nova-runtime`,
     and Task 5 runs with that build's library alone.

  Two of its findings stand, recorded in ADR 0045's consequences: SSG does
  not refuse a name Windows reserves or alters (`CON`, a piece ending in a
  dot or holding `:`), though such a page is still written inside its
  directory; and a comment of the template that holds `</div>`, inside the
  root, ends the root early when it is emptied, which a view cannot write.
  A nit stands too: the spec's risk 5 says the CLI's tests build once,
  where `ssr_tests.rs` builds the project in each of its tests; the spec's
  words, which no code changes.
- **A second check** read the fixed prototype and this plan again, and
  found no serious fault: the nine fixes and decision 13 work, in Chromium
  as under the fake DOM. Its findings, each fixed in the prototype's
  `23a6e93 4.6 prototype: the second check's fixes`, a code fix with a
  test that fails without it:
  1. **The data's scan was untested:** that it stops at the root's first
     following element, and steps over texts and comments. `hydrate`'s
     `data` root now has whitespace and a comment before its `script`, and
     a `behind` root is followed by a `p`, then the data's `script`, and
     falls back (decision 32; mutants 90 and 91).
  2. **`page_in` compared spellings:** a server's absolute `ssg: wrote`
     path inside `dist/` was refused from the project's root and taken
     from `src/`, and the records over-claimed ("no `.`", "read as
     written"). Now a relative path is read from the server's working
     directory and `dist` made absolute the same way, and the records say
     what Rust's path components do (decision 33;
     `ssg_takes_a_page_named_by_an_absolute_path`).
  3. **`document`'s and `serve`'s docs** said a placeholder is emptied
     first, but a root no `</div>` closes keeps what follows it: they say
     so now (decision 38).
  4. **ADR 0045's `nova dev` consequence** assumed an empty root: a
     project page whose root holds a placeholder logs a mismatch on each
     load, and builds; it says so now.
  5. **A stale data `script`** past whitespace after the root was kept,
     and the client, which steps over whitespace, read it; one with no
     `</script>` was untested. Now the first goes, the whitespace kept, and
     the second stays, recorded, each tested in
     `ssr_page_page_with_and_document` (decisions 19 and 38).
  6. **The first check's nit** on the spec's risk 5 was neither fixed nor
     listed as standing: it is listed above.
  7. **`40-TOOLING.md`** had an 88-column line, and left out "once the
     server has ended well": both fixed.
- **A replay applied it** to `a5c0335` on a scratch branch, task by task,
  as the steps say, each task's tests before its code; each test-first
  step's failure is that run's.
  - A script split the prototype's final tree into one commit per task,
    each task's tests split from its code. Files that change in more than
    one task carry their intermediate versions: `index_complete.rs` counts
    154 programs and 179 files after Task 1, 156 and 181 after Task 2, and
    182 files after Task 3; `run_tests.rs` and `wasm_tests.rs` take Task 1's
    hunks, the fact-check's `hydrate_with` fallback among them, then Task
    2's, then the rest in Task 3; the resolver's test is in Task 3's tests,
    its code in Task 3's code; `bundle.rs`'s and `cmd/run.rs`'s unit tests
    are in Task 4's tests; `tests/ssr/site` is in Task 3's tests, but for
    `src/stray.nova`, which only Task 4's `ssr_tests.rs` runs.
  - Every tests step fails cleanly before its code, with no stub: Nova
    compile errors in the child `nova` in Tasks 1 to 3, `clap` refusing
    `--ssr` and `--ssg` in Task 4, and Rust compile errors there, in
    `packages.rs` and in `nova-cli`'s unit tests.
  - The patches below are those commits. Each applies with `git apply
    --whitespace=error-all` in order on `a5c0335`, with no warning, and the
    last tree is the prototype's head, `23a6e93`. No added line has
    trailing whitespace or a raw CR; the only non-ASCII characters added
    are `§`, `…` and `—`.
  - After the fact-check's fixes, the records commit (reading the records
    against the code found the phase plan's 4.6 amendment silent on
    `hydrate_with`'s fallback, which the prototype's `9a6ae03 4.6
    prototype: records` adds), the mutants' two more gaps (below) and the
    second check's fixes, the split was made again, and all ten steps
    replayed on it from a fresh `NOVA_HOME`; every "Expected:" below is
    that last run's.
- **The mutants.** A first run of 77, on the first split, caught 75. Two
  passed their tests, and the prototype's `5fdaf52 4.6 prototype: close two
  test gaps` closed both, in six test files and no code:
  - a `Property` left unset on an adopted element passed, since the fake
    DOM read a textarea's `value` from its attribute, which the server
    writes and a browser ignores: the fake DOM now reads it as a browser
    does, and `hydrate`'s counter holds a textarea with a fixed
    `Property("value", …)`, whose value `hydrate.mjs` prints before and
    after hydration (mutant 18);
  - SSG's check for `\`, NUL, `?` and `#` in a piece passed, since the test
    refused only `/a/../b`: `refuse.nova` now takes its bad path from its
    first argument, and the test tries all six, each into a fresh
    directory (mutant 59).

  The fact-check's fixes then moved or changed the code ten of them edit
  (27, 29 to 32, 41, 43, 58, 59 and 75): each is rewritten to break the
  same behaviour; 30's panic is gone, so 30 puts it back in place of the
  fallback. Twelve more, 78 to 89, break the fixes. A first run of those
  89 caught 87; two passed, since no test page
  put, just after a root, an element with the id `nova-data` that is not a
  `script` (80, the data's element taken whatever its tag), or a
  `nova-data` `script` with no `type="application/json"` (81, taken
  without its `type`). The prototype's `ee6718e 4.6 prototype: close two
  more test gaps` closed both, in the hydrate project, its harness and its
  golden: two more roots, `decoy`, which the server fills, followed by a
  `div` with the data's id and type, and `untyped`, empty, followed by a
  `script` with the data's id and no type, each holding other guests as
  JSON, hydrate with the fallback, with no report. The second check then
  found the data's scan untested; its fix's tests come with two more
  mutants, 90 and 91, and its restructured emptying moved the code that 44
  and 45 edit, which are rewritten to break the same behaviour. On the
  final tree all 91 failed their named tests, each through a wrong output,
  a panic or a wrong exit code, and none through a build that broke.
- **The gate passed in the replay,** locally: every script of the `browser`
  job through Playwright 1.56.1 in Chromium, `ssr.mjs` among them, on the
  final split's Task 5 tree, with the runtime library beside `nova` from
  `cargo build --locked -p nova-cli -p nova-runtime` alone, as CI builds
  it, and no `NOVA_RUNTIME_LIB`.
- **The final checks ran on the final split's last commit:**
  - `cargo fmt`, clippy with `-D warnings` and `nova fmt` were clean after
    every task;
  - the full suite gave 2390 passed, 1 failed (the IPv6 test) and 10
    ignored, against the base's `2364 1 10`: 26 new tests;
  - `06-counter-spa`'s `app.wasm` and `app.js` are 21,070 bytes by `flate2`
    at level 9, as at the base, against 51,200: 06 does not call
    `hydrate`;
  - `ssr-app`'s client is 41,260 bytes gzipped, as `ssr.mjs` prints it,
    not limited (spec §12, risk 4);
  - Rust 1.78's check was clean, and `Cargo.lock` did not change.
- **What the checks left standing,** each recorded or handed on:
  - SSG does not refuse a name Windows reserves, and a template comment
    holding `</div>` fools the root's emptying (the fact-check, above);
  - an object's keys in the data come out in its `Map`'s order, so two
    `--ssg` builds are not byte for byte the same (decision 30);
  - a static route written with a trailing `/` fails `--ssg` (decision
    31);
  - Rust 1.78 checks the libraries and binaries, not the tests:
    `assert_cmd` 2.2.1's manifest does not parse on 1.78. Task 7's check is
    4.5's exactly;
  - `tests/runtime/http_server_dispatch.nova` is not in `nova fmt`'s layout
    at the base, for three lines it already had; `tests/runtime` is in no
    check's paths, so the plan checks the three new programs by name.

## Global Constraints

- **The workspace's minimum Rust stays 1.78,** edition 2021. CI's MSRV job
  runs `cargo check --locked --workspace` with `RUSTFLAGS=-D warnings`. No
  `std::sync::LazyLock` or `LazyCell` (1.80), no `Option::is_none_or`
  (1.82), no `std::path::absolute` (1.79).
- **`Cargo.lock` does not change.** Any change to it stops the task: ask
  the user. So does anything that needs a new crate (spec §10). The server
  is Nova's own `std/http` on `std/net`, and `nova build --ssg` runs it
  with `std::process`.
- **No `unwrap()` outside `#[cfg(test)]` and test files:** `expect("why")`,
  `unwrap_or…` or `?`.
- **Tests never touch the internet.** Every server a test starts binds
  `127.0.0.1:0`: `tests/ssr/site`'s server and `ssr-app`'s print the port
  they bound, which the test reads, and `http_listen_on.nova` reads its own
  listener's port and connects to it; `elsewhere.nova` asks for
  `127.0.0.1:99999`, which no system binds. **No test uses port 3000,** the spec's example address and `nova
  dev`'s default. A test that needs `NOVA_HOME` sets it to a fresh
  directory. Every test directory has a fixed name under the system's
  temporary directory, removed and made again at the test's start. Node
  runs only what a test built.
- **A test that needs Node** passes without running when `node` is not on
  `PATH`, and fails instead under `NOVA_REQUIRE_NODE=1`, which CI sets.
  Run the WASM tests here with `NOVA_REQUIRE_NODE=1`: Node 22 is installed.
- **Nothing in `cargo test` or `nova` installs a package,** and nothing
  installs `wasm-opt`. Playwright is the `browser` job's alone; Task 5's
  local run uses the copy already on this machine, and is skipped where
  there is none.
- **`nova-cli`'s tests need two files beside `target/debug/nova`:**
  `libnova_runtime.a` and the `nova-bench-http` binary. A build of the
  workspace writes both; Task 0's build puts them there. 4.6 changes no
  runtime crate, so neither needs building again. After a `cargo clean`,
  build the workspace again.
- **std is embedded at build time.** After a task changes a file under
  `std/`, a run of `target/debug/nova` by hand needs `cargo build --locked
  -p nova-cli` first (Task 5, Step 2); `cargo test` rebuilds it itself.
- **Never clean `/tmp/nova-*` while tests run:** `nova test` keeps built
  binaries in `/tmp/nova-test-bin`, and a test whose binary goes fails.
- **Work in one tree.** Two worktrees of this repository sharing one
  `target/` share artifact names, so cargo can take one tree's build as
  fresh for the other (4.0's ledger). Never build in another worktree.
- **Trailing spaces are expected output.** `tests/dom/router_history.stdout`
  and `tests/runtime/ui_router.stdout` end lines with a space, as 4.5 left
  them. 4.6's patches add no line with trailing whitespace, and apply with
  no warning; still, never apply a patch with `--whitespace=fix`, and never
  let an editor trim this plan or a `.stdout` file.
- **Never use the Write tool for text that holds a `\u` escape followed by
  four hex digits:** it decodes the escape into the character. The patches
  hold such escapes (`std/ui/ssr/lib.nova`'s `script_safe` and its doc,
  `run_tests.rs`'s goldens, the ADR), and so does this plan. Apply the
  patches with `patch_block`, from the committed plan; write anything else
  that holds one with a script, and check it with `grep -c`.
- **Nothing writes into the repository but the patches.** The hydration
  tests run `nova run src/server.nova` in `tests/dom/<name>/`, which
  writes nothing there, and write the server's output, the client's build
  and the harness into the temporary directory; the `ssr_*` tests and
  `ssr_tests.rs` work in copies of `tests/ssr/site`; `ssr.mjs` copies
  `ssr-app` into the directory it is given, outside the tree. Task 5
  checks it.
- **Nothing is published, and no tag is pushed.** Merge by rebase, on the
  user's word only. Never enable auto-merge.
- **This machine is Linux.** Windows and macOS results come from CI, which
  runs all three systems, Node included: SSG's paths and the CLI's
  handling of the server's `ssg: wrote` lines on Windows, where `\` is a
  separator too, `ssr_tests.rs`'s paths and its failing server's words
  there, and the termination request `ssr_serve_answers_files_and_pages`
  sends there, are CI's to run. `release_applies_to_the_client_and_the_server`
  passes without running where no `clang` is on `PATH`.

## Conventions

- **Scratch directory:** `$P` is the session's scratch directory, outside
  the repository. Set it in every shell before a command that names `$P`.
- **The ledger:** `$P/ledger-46x.md`. Every ruling made while executing goes
  there, with its cost if wrong; so do each task's figures.
- **Commits.** Each task's message is written to `$P/msg-46-<task>.txt`:
  the subject the task names, a blank line, and the session's attribution
  lines. Then `git add -A`; `git status --short` lists only the task's
  files; and `git commit -F "$P/msg-46-<task>.txt"`.
- **The baseline's full output** is `$P/base-46.txt` (Task 0).
- **Runs.** The replay ran every command with these variables set. Each
  shell sets them too, with three shorthands, and runs every command from
  the repository's root:

  ```sh
  export CARGO_INCREMENTAL=0 NO_COLOR=1 RUST_BACKTRACE=0 NOVA_HOME="$P/home-46"
  mkdir -p "$NOVA_HOME"
  W() { NOVA_REQUIRE_NODE=1 cargo test -p nova-cli --test wasm_tests -- "$@"; }
  R() { cargo test -p nova-cli --test run_tests -- "$@"; }
  X() { cargo test -p nova-driver --test index_complete "$@"; }
  ```

  So `W hydrate_` runs `wasm_tests`'s tests whose names hold `hydrate_`,
  `R ssr_` `run_tests`'s, and `X` the corpus test. `NOVA_HOME` keeps a run
  by hand off `~/.nova`; every test that needs one sets its own.
- **The patches.** Each task's code is in patch blocks below, each marked
  `<!-- patch NAME -->` and fenced with four backticks. They are the
  replay's commits, so apply them; do not retype them. From the
  repository's root:

  ```sh
  PLAN=docs/superpowers/plans/2026-10-20-phase-4-6-ssr-hydration-and-ssg.md   # this file, committed
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

  `patch_block t1-tests` applies the block named `t1-tests`, and prints
  nothing. A block that does not apply means the tree differs from the
  plan's: stop, and record why in the ledger.
- **Expected outputs are written once, from a run, and read.** Each
  `.stdout` under `tests/runtime/` and `tests/dom/` is in Task 1's or Task
  2's test patch: the replay wrote `ui_hydratable.stdout`,
  `ui_router_server.stdout`, `http_listen_on.stdout` and
  `http_server_dispatch.stdout`'s new lines from the program's native run,
  and `hydrate.stdout` and `hydrate_mismatch.stdout` from the harness's run
  under the fake DOM, and each was read against the spec.
- **Each task** is test first: its tests' patch, their run and the expected
  failure, then its code's patch, its run and the expected pass. Unit tests in
  the files a task changes ride in its code's patch, but for the resolver's,
  `bundle.rs`'s and `cmd/run.rs`'s, which split from their code; Task 7's
  mutants show they bite. Task 5's patch is the browser test and CI alone, and
  Task 6's the records. Each task then runs its checks:
  - `cargo fmt --all -- --check`, which prints nothing;
  - the workspace's clippy, `cargo clippy --locked --all-targets
    --all-features -- -D warnings`, as CI runs it, which ends with
    `Finished` and no warning;
  - `cargo run --locked -q -p nova-cli -- fmt --check std examples
    tests/wasm tests/dom tests/browser/router-app`, with `tests/ssr` after
    it from Task 3 on, and `tests/browser/ssr-app` before `tests/ssr` from
    Task 5 on, which prints nothing.

  It ends with one commit. A difference from an `Expected:` is fixed in
  the smallest way and recorded in the ledger as a ruling, with its cost if
  wrong.


## Review Focus

The inputs most likely to bite a user, most likely first. Each has its test
in the task that owns the code:

1. **The walk's cursor and its edge cases:** an element's tag compared
   lower-cased; the cursor stepped past each node adopted, and into an
   element's first child; a text compared with the view's, a `DynText`'s as
   its first run reads it; an empty text, which the server could not write,
   given a new node before a marker, an element or the end; the separator
   stepped over before a text that follows a text, and only then; a
   region's first run walked from the node after its `<!--[-->` and ended
   at its `<!--]-->`, its later runs built before that end; a node left in
   an element or in the root; a `Property` set, a `Dynamic`'s effect,
   handlers added; `on_mount` after the walk, not during it; an empty
   root's whitespace, form feed included, removed before the view is built
   → `hydrate_adopts_the_servers_nodes` and
   `hydrate_reports_a_mismatch_and_builds_again` (Task 1), as they are and
   under stress; mutants 7 to 22 and 82.
2. **A mismatch's cleanup:** the first difference alone, each side in the
   spec's words or decision 14's, clipped to 40 characters; the walk's
   owner disposed, so its cleanups run, its `on_mount` callbacks never do,
   and its effects and handlers stop (the server's button, adopted, then
   does nothing), and dropped from the roots; the root emptied; the view
   built again by calling `f` again → `hydrate_reports_a_mismatch_and_builds_again`
   (Task 1); mutants 23 to 28.
3. **Markers that could be forged:** a program's text holding `<!--|-->`
   or `<!--[-->`, escaped as `&lt;!--`; `render_to_string`, which writes
   none; the data's `<!--`, written `\u003c!--`; and `emptied`'s count of
   `<div` and `</div>`, which holds only because text and attribute values
   are escaped → `ui_hydratable.nova` (Task 1),
   `ssr_page_page_with_and_document` (Task 3); mutants 1 to 6.
4. **Escaping in data, text and attributes, and the data read back:** `&`,
   `<` and `>` in text, `"` too in an attribute, a CR as `&#13;`; the
   data's `<`, `>`, `&`, U+2028 and U+2029 as escapes that parse back to the
   same value; the data's element, its type and id, just after the root;
   `hydrate_with` reading the root's next element alone, past texts and
   comments and never past the first element, and only a `script` with
   that id and that type, never a view's own `nova-data`; with no data, its
   fallback, built into an empty root and walked in a filled one; and its
   panic for data that does not parse → `ui_hydratable.nova`, `hydrate`'s
   `data`, `filled`, `fallback`, `decoy`, `untyped` and `behind` roots and
   `hydrate_panics_without_its_root_or_on_data_that_does_not_parse` (Task
   1), `ssr_page_page_with_and_document` (Task 3), and `ssr.mjs`'s title,
   which holds `</script><!-- &` (Task 5); mutants 4, 29 to 32, 39 to 43,
   78 to 81, 90 and 91.
5. **`serve`'s list of files and its status codes:** no file read that the
   manifest does not list, by any spelling of `..`; a manifest's own path
   with a piece that is empty, `.` or `..`, or that holds `\` or NUL,
   dropped as the manifest is read, since a manifest written by hand can
   list `../secret`; the path before `?`, decoded, less one `/`, as
   `/app%2ejs` and `/app.js?v=1` ask; a listed file not on disk; the `.`
   rule; `200` or `404` by `found`, the query kept; each type and
   `nosniff`; `GET` alone; the errors that stop it starting; the URL line
   with the port bound → `ssr_serve_answers_files_and_pages` and
   `ssr_serve_reports_what_stops_it_starting` (Task 3); mutants 36, 46 to
   54 and 85.
6. **Emptying a filled root:** the `</div>` that balances the root, a
   `<divx` not counted, the data's `script` after it dropped, just after
   or past whitespace, and one with no `</script>` left as it is, an empty
   root left as it is; `document` emptying it too, whitespace, a placeholder or
   SSG's page, and panicking only when there is no `<div id="app">`; a root
   no `</div>` closes, kept after an empty root; SSG run over its own
   output; a server started after `--ssg`. A comment of the template that
   holds `</div>` ends the root early, and stands (ADR 0045): a view cannot
   write one → `ssr_page_page_with_and_document` and
   `ssr_ssg_writes_the_static_pages_and_refuses_a_bad_path`, which runs SSG
   twice (Task 3), and `ssr.mjs`, whose server starts after `--ssg` (Task
   5); mutants 41, 43 to 45 and 84.
7. **SSG's paths and refusals:** the static routes, none with a `:name`
   piece or the catch-all, each once, in order, as written; each piece of a
   path checked for empty, `.`, `..`, `\`, NUL, `?` and `#`; a refusal after
   the pages before it, and nothing written after; `/` over the template it
   read; the directories made; `--ssg` with no directory; one `ssg: wrote`
   line per page → `ui_router_server.nova` (Task 2),
   `ssr_ssg_writes_the_static_pages_and_refuses_a_bad_path` (Task 3),
   `ssg_writes_a_page_per_static_route_and_lists_them` (Task 4); mutants
   37, 38 and 55 to 60.
8. **The router at another location:** `with_location`'s start, its
   restore and its nesting, a `#fragment` dropped, by `found` too; a router
   made outside it at the page's location → `ui_router_server.nova`,
   natively and on WASM, as it is and under stress (Task 2); mutants 33 to
   35 and 83.
9. **`std/http` and the import:** the fallback for a `GET` alone, kept by
   `get`, replaced by a second `fallback`, never set on the server it was
   called on; `listen_on`; `listen` as `bind` and `listen_on`; `import
   ui::ssr` natively, E0092 on WASM, and E0001 for a bare name on both →
   `http_server_dispatch_run`, `http_listen_on_run` and
   `http_server_example_serves_and_exits_cleanly` (Task 2),
   `ssr_is_imported_by_name_and_only_natively` in the resolver and in
   `wasm_tests` (Task 3); mutants 54 and 61 to 65.
10. **The driver's root:** `Roots::Server` rooted at `src/server.nova` alone,
    with the package's modules and dependencies, without `src/main.nova` or
    `src/lib.nova`; no root where there is no server →
    `a_server_is_rooted_at_src_server_nova` (Task 4); mutants 66 and 67.
11. **The CLI's refusals and builds:** each refusal before anything is
    built, no `dist/` and no `target/` after it, in its order and words;
    the server's name; `-o` for the client alone; the server run from the
    project's root with the client's directory made absolute; its output
    passed through; the pages listed, each a file inside the client's
    directory by plain names alone, both made absolute from the server's
    working directory, so that an absolute path names a page wherever
    `nova` runs, and every file measured again; a line
    naming any other file failing the build once the server has ended
    well; a failing server's exit code, and its end told in the same words
    on every system; `--release` for both → `ssr_tests.rs`,
    `ssg_pages_join_the_manifest_with_their_sizes` and `cmd/run.rs`'s two
    unit tests (Task 4); mutants 68 to 77 and 86 to 89.
12. **A test that passes without its code:** each mutant names one that
    would. The gaps the prototype closed show the kind: a fake DOM that read
    a textarea's value from its attribute, a refusal test with one bad
    path, no page whose root is followed by an element that looks like the
    data and is not, and none whose data came past whitespace or after
    another element (the second check). The
    hydration harnesses print how many of the server's nodes are kept, in
    order, and which are new; their stress runs count the calls left in
    `std/rt`'s table; `ssr.mjs` checks every node under `#app` is `===` the
    one it held before `app.wasm` loaded, with no node more or less, and
    that the console has no error.
13. **Windows paths in SSG and in the CLI tests:** SSG joins a page's
    pieces with `/` and refuses `\` on every system, but not a name Windows
    reserves or alters, `CON`, a piece that ends in a dot or holds `:`,
    which stands (ADR 0045): such a page is still written inside its
    directory; `page_in` strips the client's directory from each `ssg:
    wrote` line with `Path::strip_prefix`, by components, which on Windows
    takes `/` and `\` alike, and keeps the page only when each name below
    it is a plain one; the server's `.exe`; `ssr_tests.rs`'s `listed` and
    `built_server`, and `run_tests.rs`'s `under`, join components;
    `ended`'s words, the same on every system; `SsrServer` starts `nova` in
    a process group of its own there, and the test's termination request
    is a Ctrl-Break. This machine cannot run them: CI's Windows leg does.

## Decisions

Made while writing the spec (its §14), prototyping, replaying and
fact-checking, each as built, and each read against the code. Where one
departs from the spec or fills in what it left open, it says so and why, and
is marked **(differs)**; ADR 0045 records them, and the departures from
`30-FRONTEND.md` §7.4 and the master spec.

**The spec's decisions (its §14), as built**

1. **`render_to_string` does not change; `render_hydratable` shares its
   writer** (spec §14.1, §3). `render(f, marks)` runs both: one batch, a
   root owner disposed once the HTML is written. `write_view`,
   `write_views`, `write_attr` and `escape` take `marks`, and
   `render_to_string(f)` is `render(f, false)`, so its output, its tests
   and its doc comment are 4.5's (mutants 5 and 6).
2. **The markers** (spec §14.2, §3). With `marks`, a region is written
   between `<!--[-->` and `<!--]-->`; `<!--|-->` goes between two text views
   side by side, each a `Text` or a `DynText`, siblings among an element's
   children or a region's views; an empty text writes nothing but keeps its
   separators; and a CR, in text or in an attribute's value, is `&#13;`. A
   program's `<!--` arrives escaped, as `&lt;!--`, so no marker can be
   forged: `ui_hydratable.nova`'s last line writes the text `<!--|-->`
   (mutants 1 to 4).
3. **The walk checks tags and texts, not attributes, and sets properties**
   (spec §14.3, §4.2). An element's tag is compared lower-cased.
   `adopt_attr` skips a `Static` attribute and gives any other `build_attr`,
   as `mount` does: a `Property` is set, `Dynamic` and `DynProperty` get
   their effects, whose first run writes the value again, and `On` is
   listened for. `hydrate`'s counter holds a textarea with a fixed
   `Property("value", …)`, which the server writes as an attribute that a
   browser ignores on a textarea, so its value shows the property set
   (mutants 18, 19 and 21).
4. **An empty root mounts with no report** (spec §14.4, §4.1).
   **(differs)** Its whitespace is HTML's, space, tab, LF, CR and FF, not
   `String::trim`'s, which takes U+00A0 and three other spaces besides and
   leaves a form feed: HTML's parser and CSS take those five as
   whitespace, and a root holding `&nbsp;` holds text the page shows. Only
   text nodes count, so a comment makes a root not empty. `hydrate.mjs`'s
   `blank` root holds a space, an LF and a tab, and its `feed` root a form
   feed and an LF; the whitespace goes before the view is built (decision
   35) (mutants 7, 8 and 82).
5. **The data's element is `<script type="application/json" id="nova-data">`,
   just after the root, and the root is `<div id="app">`** (spec §14.5, §6.2),
   which `document` empties before it fills it (decision 34). `page_with`
   writes the data as `stringify(data.to_json())`, with `<`, `>` and `&` as
   `\u003c`, `\u003e` and `\u0026`, and U+2028 and U+2029 as `\u2028` and
   `\u2029`, as spec §6.2 says, so no `</script>` or `<!--` in the data can
   end or change its element. `hydrate_with` finds its root first, then reads
   the data from the element just after it (decision 32), parses it and makes
   its `T` (mutants 29, 31, 39 to 43 and 79 to 81).
6. **The router's three functions** (spec §14.6, §5). **(differs)** in what
   §5 left open:
   - `with_location` sets a new field, `Routing.location`, runs `f`, and
     puts back the one before, so calls nest. A router made inside it
     starts there, through `starting`, less any `#fragment`, since a
     router's location never holds one (4.5's decision 5);
   - `found` takes any pattern whose pieces are `["*"]` as the catch-all,
     as matching does: `*` and `/*`, as the spec says, and `//*` too; and
     it drops a `#fragment`, as `with_location` does (decision 36);
   - `static_paths` keeps each path once, comparing them as written, so
     `/about` and `/about/` are two.

   Mutants 33 to 38 and 83.
7. **`ui::ssr`'s names** (spec §14.7, §6.1): `Page { body, data }`,
   `page`, `page_with`, `document`, `Site { address, dist, routes, render
   }` and `serve`, the records' fields public. `page(location, f)` is
   `with_location(location, || render_hydratable(f))`. The module imports
   `ui` and `ui::router`; `std/http`, `std/net`, `std/fs`, `std/json` and
   `std/process` are in a native module's prelude.
8. **`serve`'s rules** (spec §14.8, §6.3). It reads the template, then the
   manifest, then binds. A request's path is its target before any `?`,
   percent-decoded by the router's rule (a `%` not followed by two hex
   digits, or bytes that are not UTF-8, leave the target as written), less
   one leading `/`. A path the manifest lists is that file, with a
   `Content-Type` by its extension in any case, and `nosniff`; any other
   whose last piece holds a `.` is `Response::not_found()`, `404` and the
   text `not found`; any other is the page, `200` when `found(routes,
   target)`, the query kept, and otherwise `404`. `/..%2fsecret` decodes
   to `../secret`, which no manifest lists and whose last piece holds no
   `.`: it is a page, `404` since only the catch-all serves it, and no
   file is read. Any method but `GET` gets `not_found` from `Server` before
   the fallback runs (mutants 46 to 54).
9. **`std/http`'s two additions** (spec §14.9, §8.3). `Server` gains
   `unmatched: Option<fn(Request) -> Response>`. `fallback(handler)`
   returns a copy that holds it, as `get` returns a copy, and `get` keeps
   it; `dispatch` calls it for a `GET` no route matches, and only for a
   `GET`. `listen_on(listener)` is `listen`'s loop, and installs the signal
   handler too, for a program that bound without `listen`; `listen` is
   `bind`, then `listen_on`. Spec §9.2's "unit tests" of these are
   `std/http`'s own kind: `http_server_dispatch.nova`'s fallback lines,
   `http_listen_on.nova` over TCP on port 0, and `fallback`'s doc test
   (mutants 54 and 61 to 63).
10. **SSG is `serve` run with `--ssg <dir>`** (spec §14.10, §6.4).
    **(differs)** in what §6.4 and §7 left open:
    - `serve` looks for `--ssg` among the program's arguments after its
      name, and takes the next as the directory, or `""` when none follows;
    - it reads `<dir>/index.html`, then writes each of `static_paths`, `/`
      over `<dir>/index.html`, printing `ssg: wrote <file>` for each, such
      as `ssg: wrote dist/about/index.html`;
    - **the CLI learns the pages from those lines,** on the server's
      stdout, which it passes through: each file under the directory it
      gave the server becomes a path from it, with `/` between names, and
      joins the manifest, and every listed file is measured again, since
      the root's page replaced `index.html`.

    Mutants 55 to 60, 75 and 77.
11. **The server's executable is `target/<profile>/<name>-server`** (spec
    §14.11, §7), with the system's executable suffix. `-o` names the
    client's directory alone (mutants 72 and 73).
12. **`std/dom::console_error`** (spec §14.12, §8.1), with the shim's
    `dom_console_error`, which calls `console.error`; `backend_report`
    calls it.

**Hydration (the prototype's part A)**

13. **`hydrate`'s panic for a selector that matches nothing names the
    function called:** `std/ui: hydrate: no element matches "…"`, or
    `std/ui: hydrate_with: …`. **(differs)** Spec §4.1 has `mount`'s panic,
    `std/ui: mount: no element matches "…"`; a page that hydrates is told
    which call failed
    (`hydrate_panics_without_its_root_or_on_data_that_does_not_parse`).
14. **The found side of a mismatch.** **(differs)** Spec §4.3 reads each
    side as `<tag>`, `text "…"`, `a region`, `the end of a region` or
    `nothing`. Besides:
    - a separator found is reported as the text after it, which the
      server wrote there, or as `text ""` when no text follows it; a
      `<!--|-->` means nothing to the program's author;
    - a comment that is not a marker is reported as `a comment`, a reading
      §4.3 does not list: `hydrate_mismatch`'s `comment` root holds `<!--
      a note -->`, as a server written by hand can;
    - a missing separator reads `expected text "<next>", found …`, the text
      the view has next.

    Mutants 23 to 25.
15. **The `<error>` of the data's panic is `JsonError.msg`,**
    whether `parse` or `T::from_json` refused the text: `std/ui:
    hydrate_with: the page's data does not parse: expected a value`, or
    `expected a string`. **(differs)** Spec §4.1 left `<error>` open
    (mutant 31).
16. **The disposed walk owner is removed from the mounted roots,** `std/ui`'s
    `roots`, so nothing holds it once the view is built again.
    **(differs)** Spec §4.3 says only that it is disposed. No mutant drops
    the removal alone; mutant 26 drops the disposal.
17. **The fake DOM's parser turns CR, and CR LF, into LF,** as a browser's
    does, so a raw CR would arrive as LF and `&#13;` is tested. It throws a
    `SyntaxError` on an entity it does not know and on HTML it cannot
    parse, such as an end tag it does not expect; an element's
    `textContent` skips comments, as a browser's does; and a textarea's
    `value` is the one set, or else its text, not its `value` attribute.
    **(differs)** Spec §9.3 listed what the parser reads, not these.
18. **The hydration tests are projects under `tests/dom/`:** `hydrate/` and
    `hydrate_mismatch/`, each `nova.toml`, `src/view.nova`,
    `src/server.nova` and `src/main.nova`, beside `<name>.mjs` and
    `<name>.stdout`. `hydration_program` runs the server natively from the
    project, which prints one line per root, its id and its HTML, and a
    line `nova-data` and the JSON; builds the client for WASM into the
    test's temporary directory; and runs the harness there, which sets each
    root's `innerHTML`. So a root's text cannot hold an LF. **(differs)**
    Spec §9.3 said each test takes a native render's HTML, and left the
    means open.

**The server (the prototype's part B)**

19. **`serve` and SSG empty a filled root, and drop the data's `script`
    after it, before using a page as the template.** **(differs)** After
    `--ssg`, `dist/index.html` is the rendered `/`, which holds no `<div
    id="app"></div>`: without this, `serve` would refuse it, and a `--ssg`
    build could not be served. `emptied` ends the root at the `</div>` that
    balances its `<div id="app">`, counting each `<div` followed by a space,
    `>`, `/`, a tab, an LF or a CR, in either case; that holds since a
    view's text and attribute values are escaped. The data's `script` goes
    when it follows the root's `</div>` alone or past whitespace, as the
    client's `hydrate_with` would find it there, the whitespace kept (the
    second check: one past a line feed stayed, and the client read it). SSG
    run again over its own output writes the same pages. `document` empties
    a filled root too (decision 34) (mutants 44, 45 and 84).
20. **A fifth refusal, `--ssr` or `--ssg` outside a project, and the
    refusals' words.** **(differs)** Spec §7 gave four, and the words of
    the last. In the order they are checked, each before anything is built:
    - a file argument: ``` `--ssr` builds the project's src/main.nova and
      src/server.nova; it takes no file ```;
    - `--target`: ``` `--ssr` builds the client for wasm and the server
      natively; it takes no `--target` ```, which is why `--target` lost
      its default: native is still the default when it is left out;
    - no project: ``` `--ssr` builds a project, and there is no nova.toml
      here or above ```;
    - a library: `nova build`'s own, ``` `<name>` is a library: it has no
      src/main.nova; `nova check` and `nova test` work on it ```;
    - no server: ``` `--ssr` needs src/server.nova, the server's entry ```.

    Each says `--ssg` for `--ssg`, and for `--ssr --ssg` (mutants 68 to
    71).
21. **`serve`'s own errors,** each an `IoError` whose message `serve`'s
    caller prints. **(differs)** Spec §6.3 named only an unreadable file
    and a template with no root:
    - `ssr: cannot read <file>: …`, for the template or the manifest, and
      `ssr: <file> has no <div id="app">`, for the template (decision 34);
    - a manifest with no `files` object: `ssr: <dist>/manifest.json has no
      "files" object`;
    - an address that cannot be bound: `ssr: cannot bind <address>: …`;
    - `--ssg` with no directory: `ssg: --ssg needs a directory`;
    - SSG's own: `ssg: cannot read <file>: …` and `ssg: <file> has no <div
      id="app">` for its template, `ssg: cannot make <dir>: …`, `ssg:
      cannot write <file>: …`, and its refusal, `ssg: the route path
      "<path>" names no file: a piece of it is empty, "." or "..", or holds
      "\", NUL, "?" or "#"`.

    Mutants 46, 47 and 60.
22. **A listed file that cannot be read when it is asked for gets `404`.**
    **(differs)** Spec §6.3 did not say. `ssr_serve_answers_files_and_pages`
    asks for `gone.js`, which the manifest lists and `dist/` lacks.
23. **The `nosniff` header is written `x-content-type-options`,** in lower
    case, as `std/http` writes each header name. **(differs)** Spec §6.3
    wrote `X-Content-Type-Options`; HTTP's header names ignore case (mutant
    51).
24. **On WASM, a name of `ui::ssr` without its import is E0001, as
    natively.** **(differs)** Spec §6.1 made the import E0092 and said no
    more. 4.2's `missing_names` makes a name of a std module that the
    target lacks E0092; it now leaves an imported module's names out, since
    no module sees them without the import, which is E0092 itself (mutants
    64 and 65).
25. **Under `--ssg`, a failing server ends `nova` with the server's exit
    code.** **(differs)** Spec §7 said "a failure fails the build, with its
    status". `nova` prints `Error: the server failed to generate the
    pages, <how it ended>: <server>`, in decision 37's words, then exits
    with the server's code, or 1 when it has none. The manifest is not
    written again, so pages on disk are not listed (mutants 74, 88 and
    89).
26. **A listed file is read when it is asked for, through `fs_read` and
    `fs_take_bytes`,** the std-only builtins `std/fs` is written on, since
    a handler is a plain function and cannot await. **(differs)** Spec
    §6.3 did not say.
27. **`tests/ssr/site` belongs to Task 3's tests.** `run_tests.rs`'s
    `ssr_serve_*` and `ssr_ssg_*` copy it and run its programs with `nova
    run src/<entry>.nova`, which needs no driver root; Task 4's
    `ssr_tests.rs` builds copies of it. Its `src/stray.nova`, which only
    `ssr_tests.rs` runs, comes in Task 4's tests.

**The browser test and the records (the prototype's part C)**

28. **`tests/browser/ssr-app` waits for hydration through `on_mount`.** Its
    root's `data-state` reads `server` in the HTML and `hydrated` once the
    client's `on_mount` sets it, which runs after the walk. `ssr.mjs` holds
    `app.wasm` back, keeps a handle to every node under `#app`, lets it
    load, waits for `hydrated`, and checks each handle is `===` the node at
    its place, with no node more or less.
29. **`hydrate_with` takes a fallback** (the user's decision 13, of
    2026-10-20): `hydrate_with(selector, fallback, f)`, such as
    `hydrate_with("#app", Data { title: "", names: [] }, App)`. When the
    page holds no data from the server, as under `nova dev`, it hydrates
    `f(fallback)` as `hydrate` would: an empty root gets the view built
    (the user's decision 10), and a root the server filled is walked with
    it, and mismatches where the server rendered other data. Data that is
    there but does not parse still panics (decision 15). **(differs)** Spec
    §4.1's `hydrate_with(selector, f)` took no fallback, and panicked with
    `std/ui: hydrate_with: the page has no data from the server` on a page
    with none; that panic is gone. This replaces the prototype's first
    answer, that `hydrate_with` panicked under `nova dev`, which the
    fact-check found overrode the user's decision 10. `hydrate`'s `filled`
    root, which the server filled, and its `fallback` root, empty, have no
    data after them; ADR 0045 records the decision (mutants 30 and 78).
30. **SSG's JSON key order varies between runs:** an object's keys come out
    in its `Map`'s order, which each process seeds anew (the user's
    decision 4), so two
    `--ssg` builds, or two servers, can write one value with its keys in
    another order. The client reads the same value. The tests accept either
    order of the fixture's two keys (`ssr_pages`).
31. **A static route ending in `/` is refused by SSG:** `/about/` has an
    empty piece, so `nova build --ssg` fails on it. ADR 0045 records it.

**The fact-check's fixes (2026-10-20)**

A read-only check of the prototype against the spec, the code and this
plan found nine things (Checked on). The prototype's `3f1f035 4.6
prototype: the fact-check's fixes` and `aebb875 4.6 prototype: CI builds
the runtime library, and the records` answer them; besides decision 29:

32. **The data is the root's next element: a `script` with its id and its
    type.** On WASM, `backend_data(root)` reads the first sibling after the
    root that is an element, past texts and comments, and returns its text
    only when its tag is `script`, its `id` `nova-data` and its `type`
    `application/json`, as `document` writes it; any other element there,
    or none, is no data. `hydrate_with` finds its root first, then reads
    the data; natively it panics with `mount`'s message as it looks for
    the root, as before. **(differs)** Spec §8.2's `backend_data()` read
    `#nova-data`, the first element with that id in the page, so a view's
    own element with that id, inside the root and so before the data,
    stood for the data. `hydrate`'s `data` root holds a `<p
    id="nova-data">`, and its data comes past whitespace and a comment; its
    `decoy` root is followed by a `div` with the data's id and type, its
    `untyped` root by a `script` with its id and no type, and its `behind`
    root by a `p` and then the data's `script`, each holding other guests,
    and each hydrates with the fallback (mutants 29, 79 to 81, 90 and 91).
33. **The manifest's paths are checked on both sides.** **(differs)** Spec
    §6.3 served what the manifest lists, and §7 had the CLI list the pages
    the server wrote, with no check of either:
    - `serve` drops, as it reads the manifest, each path with a piece that
      is empty, `.` or `..`, or that holds `\` or NUL (`inside`,
      `plain_path`, and `plain_piece`, which SSG's `file_pieces` shares),
      so a manifest written by hand that lists `../secret` serves nothing
      outside `dist`; `ssr_site`'s manifest lists it;
    - `nova build --ssg` reads a `ssg: wrote <file>` line's file, if
      relative, from the server's working directory, the project's root,
      and makes the directory it gave the server absolute the same way, so
      the answer does not hang on where `nova` runs (the second check: an
      absolute path inside `dist/` was refused from the project's root and
      taken from `src/`). It lists the file only when it lies in that
      directory and each name below it is a plain one by Rust's path
      components, a `Component::Normal`, with no `..`, root or prefix,
      while a `.` piece and a trailing `/` are normalised away; nothing is
      asked of the file system (`page_in`). `Path::strip_prefix` on the
      line as written was lexical: `dist/../nova.toml` became the page
      `../nova.toml`, which `serve` then served. Such a line fails the build
      (decision 39). `ssr_tests.rs`'s
      `ssg_takes_a_page_named_by_an_absolute_path` runs a server that names
      its page by an absolute path, from the project's root and from
      `src/`.

    Mutants 85 and 86.
34. **`document` empties a filled root, and panics only when there is no
    root.** It fills the first `<div id="app">` through `emptied`, as
    `serve` and SSG do (decision 19): whitespace, a placeholder, or a page
    and its data as SSG wrote them give way to the page. It panics only
    when the template has no `<div id="app">`, with `ssr: the page has no
    <div id="app">`, and `serve`'s and SSG's errors say `has no <div
    id="app">` too (decision 21). **(differs)** Spec §6.2's `document`
    replaced a `<div id="app"></div>` written just so, and panicked with
    `ssr: the page has no <div id="app"></div>` on any other template,
    while `serve` emptied and served the same one (mutants 41, 43 and 84).
35. **A root that holds only whitespace is emptied before the view is
    built.** **(differs)** Spec §4.1 builds into such a root as into an
    empty one, with no report, and said no more: its whitespace stayed,
    and the view went after it (mutant 82).
36. **`found` drops a `#fragment`,** as `with_location` does, so the page
    `GET /about#x` renders and its status agree: About, and `200`.
    **(differs)** Spec §5 did not say; the page was About, with `404`.
    `ui_router_server.nova` asks `found` for `/q#top` (mutant 83).
37. **A failing server's end is told in `nova`'s own words:** `with exit
    code <code>`, or, on Unix, `stopped by signal <number>` when a signal
    stopped it and so it has no code (`ended`). **(differs)** Spec §7 said
    "with its status"; `ExitStatus`'s own words differ between systems,
    `exit status: 3` on Linux and `exit code: 3` on Windows. `ssr_tests.rs`
    checks the code, 3, and the whole line, and `cmd/run.rs`'s unit test
    both words (mutants 88 and 89).
38. **A root that no `</div>` closes is no longer an error.** `emptied`
    keeps what follows the `<div id="app">` after an empty root, as
    `document`'s and `serve`'s docs say, and leaves a data `script` with no
    `</script>` in place: nothing tells where its text ends, and such a
    template is broken HTML, whose rest a browser reads as that script's
    text, so `document` puts the page's own data before it and changes
    nothing else. **(differs)** The spec did not say, and the prototype
    took such a template for one with no root: `serve` refused it, and
    `document` panicked. `ssr_page_page_with_and_document`'s `unclosed` and
    `unclosed data` lines.
39. **A `ssg: wrote` line that names no file inside the output directory
    fails the build, after the server exits.** `nova` keeps the first such
    line, lists none of the server's pages, and, once the server has ended
    well, fails with ``the server's line `<line>` names no file inside
    <dist>``; a server that fails is reported first, as decision 25 says.
    **(differs)** Spec §7 did not say; the prototype skipped such a line.
    `ssr_tests.rs`'s `stray.nova` says it wrote `dist/index.html` and
    `dist/../nova.toml` (mutant 87).

---

### Task 0: The baseline

The branch is at this plan's commit, whose parent is `a5c0335`, the
approved spec with its §6.2 escapes restored. The plan's commit adds this
file alone, so the patches apply to it as to `a5c0335`.

- [ ] **Step 1: The tree.**
  - `git status --short` prints nothing, on
    `claude/loving-archimedes-frtf5q-46`.
  - `git diff --stat a5c0335 HEAD` lists only
    `docs/superpowers/plans/2026-10-20-phase-4-6-ssr-hydration-and-ssg.md`.
  - `BASE=$(git rev-parse --short HEAD)`: the plan's commit, recorded in
    the ledger. Task 7's review reads from it.
  - `df -h /` shows at least 8G free; if not, remove scratch directories
    of earlier sub-phases first.
- [ ] **Step 2: Node, the browser and `clang`.** `node --version` prints
  `v22.` and more; `ls -d /opt/node-tools/node_modules/playwright
  /opt/pw-browsers` lists both, for Task 5's local run (skip that run where
  either is missing, and say so in the ledger); `command -v clang` prints a
  path, so Task 4's release test runs (where it prints nothing, that test
  passes without running: say so in the ledger).
- [ ] **Step 3: The suite.** In a shell set up as the conventions say:

  ```sh
  cargo build --locked --workspace
  NOVA_REQUIRE_NODE=1 cargo test --locked --workspace --no-fail-fast > "$P/base-46.txt" 2>&1
  grep '^test result' "$P/base-46.txt" | awk '{p += $4; f += $6; i += $8} END {print p, f, i}'
  grep ' FAILED$' "$P/base-46.txt"
  ```

  Expected: `2364 1 10`, and one failure, `test
  net::tests::an_ipv6_connect_parks_on_its_first_poll_and_then_establishes
  ... FAILED`. It needs IPv6, which this machine lacks, as in every
  earlier phase. (`a5c0335`'s tree is `2b7a6ed`'s and the spec; 4.5's last
  run there, after its review's fixes, gave these figures.)
- [ ] **Step 4: The ledger.** Start `$P/ledger-46x.md`, headed with the
  base commit and the baseline's figures.

### Task 1: Hydratable HTML, and hydration

Spec §3, §4, §8.1, §8.2; decisions 1 to 5, 12 to 18, 29, 32 and 35.

**Files:**
- Modify:
  - `std/ui/lib.nova` (`render_hydratable`, `hydrate`, `hydrate_with`, the
    walk)
  - `std/ui/backend/wasm.nova`, `native.nova` (five functions each)
  - `std/dom/lib.nova` (`console_error`), `crates/nova-codegen-wasm/src/shim.rs`
    (`dom_console_error`)
- Test:
  - `tests/runtime/ui_hydratable.nova`, `ui_hydratable.stdout`
  - `tests/dom/hydrate/nova.toml`, `src/main.nova`, `src/server.nova`,
    `src/view.nova`; `tests/dom/hydrate.mjs`, `hydrate.stdout`
  - `tests/dom/hydrate_mismatch/nova.toml`, `src/main.nova`,
    `src/server.nova`, `src/view.nova`; `tests/dom/hydrate_mismatch.mjs`,
    `hydrate_mismatch.stdout`
  - `tests/dom/fake_dom.mjs`
  - `crates/nova-cli/tests/run_tests.rs`, `wasm_tests.rs`
  - `crates/nova-driver/tests/index_complete.rs`

- [ ] **Step 1: The tests.** `patch_block t1-tests`. The patch holds:
  - `tests/runtime/ui_hydratable.nova`: each view written by
    `render_hydratable`, then by `render_to_string`, which is as it was, its
    CRs shown as `\r`: nested regions, a `show` holding a `for_each` and a
    `for_each` whose items hold a `show`; a hidden `show` and a `for_each`
    of nothing, beside text; empty texts between texts and alone; texts
    side by side, fixed and dynamic, in an element and in a region, where a
    marker parts a text from the region's own; a CR alone and before an LF,
    in text and in an attribute, beside `<`, `"` and `&`; and the text
    `<!--|-->`, escaped. It joins `ui_programs_run_and_survive_gc_stress`,
    natively, and the gate as `gate::ui_hydratable` and
    `stress::ui_hydratable`, on WASM under the fake DOM, with one `.stdout`.
  - `run_tests.rs`'s `ui_hydrate_panics_natively`: natively
    `render_hydratable` renders, then `hydrate`, and `hydrate_with` with a
    fallback in a second program, panic with `mount`'s message, `std/ui:
    mount needs a build for WASM; render_to_string renders on any target`,
    which the native backend gives as they look for their root.
  - `tests/dom/fake_dom.mjs`: comment nodes, and `createComment`; an
    `innerHTML` setter whose parser reads what `render_hydratable` writes; a
    textarea's `value`; and `console.error`, kept in `win.errors` and printed
    at once as `console.error: …` (decision 17).
  - `wasm_tests.rs`'s `hydration_program(name, env)`, which runs a project
    of `tests/dom/` as decision 18 says, and three tests:
    - `hydrate_adopts_the_servers_nodes`, on `tests/dom/hydrate/`: twelve
      roots, `counter` (a class and an input's value that follow a count, a
      dynamic text after a text, a button, the textarea of decision 3),
      `lists` (a `show`, and a `for_each` whose rows hold a `show`, between
      fixed rows, and a `for_each` of nothing), `texts` (texts side by side
      with empty ones among them, a region of texts, a CR), `data`
      (`hydrate_with`'s guests, whose title holds `</script> & <b>`, read from
      the `script` after the root, past whitespace and a comment, though the
      view holds a `<p id="nova-data">`), `filled` (a root the server filled,
      with no data after it: walked with the fallback, decision 29), `decoy`
      (filled too, and followed by a `div` with the data's id and type that
      holds other guests: no data, so walked with the fallback, decision 32),
      and `fallback`, `untyped` (followed by a `script` with the data's id and
      no type that holds other guests: no data), `behind` (followed by a `p`,
      then the data's `script` holding other guests, which the client never
      reaches), `empty`, `blank` and `feed`, which the server leaves empty,
      the last two holding whitespace, a form feed in `feed`'s. Each root
      keeps every node the server wrote, as the same objects and in order, and
      gains only the empty texts the server could not write; `on_mount` runs
      after the walk, so the click it makes counts; clicks update the adopted
      nodes; regions build again between the server's markers; and the six
      empty roots get the view built, `fallback`'s, `untyped`'s and `behind`'s
      from the fallback, with their whitespace removed and no report. As it
      is, then under stress, where the collections keep `7, 7, 7, 7, 7, 8, 8,
      8, 8` calls.
    - `hydrate_reports_a_mismatch_and_builds_again`, on
      `tests/dom/hydrate_mismatch/`: twelve roots, each hydrated with a view
      a little other than the server's, one holding a comment that is no
      marker, as a server written by hand can write, each reporting its
      first difference in its exact words, then holding the client's view and
      none of the server's nodes; the walk's cleanup runs and its
      `on_mount` never does; the server's button, adopted by the walk, does
      nothing once clicked. Under stress every collection keeps one call.
    - `hydrate_panics_without_its_root_or_on_data_that_does_not_parse`:
      status 134 and each panic, `std/ui: hydrate: no element matches
      "#nope"`, the same for `hydrate_with`, and the data's two `does not
      parse` panics, `expected a value` and `expected a string`, the data in
      a `script` with its type and id just after the root. A page with no
      data is no panic (decision 29).
  - `index_complete.rs`: 154 programs and 179 files.
- [ ] **Step 2: Run, and see them fail.**
  - `R ui_programs_run_and_survive_gc_stress` (about 15 s). Expected: `0
    passed; 1 failed`: "Unexpected failure." of `nova run
    tests/runtime/ui_hydratable.nova`: ``error[E0001]: cannot find
    function `render_hydratable` in this scope`` at `ui_hydratable.nova:24:13`,
    `could not compile due to 1 previous error`.
  - `R ui_hydrate_panics_natively`. Expected: `0 passed; 1 failed`:
    ``assertion `left == right` failed: plain.nova``, `left: ""`, `right:
    "fine\n"`: the program does not compile.
  - `W ui_hydratable`. Expected: `0 passed; 2 failed`:
    `gate::ui_hydratable` and `stress::ui_hydratable`, each "Unexpected
    failure." of its `nova build --target wasm`: the same E0001, 1 error.
  - `W hydrate_`. Expected: `0 passed; 3 failed`:
    `hydrate_adopts_the_servers_nodes` and
    `hydrate_reports_a_mismatch_and_builds_again`, each "Unexpected
    failure." of the server's `nova run src/server.nova`: the same E0001,
    `could not compile due to 6 previous errors` and `due to 11 previous
    errors`; and `hydrate_panics_without_its_root_or_on_data_that_does_not_parse`
    at its first `nova build --target wasm main.nova`: ``error[E0001]:
    cannot find function `hydrate` in this scope``, 1 error.
  - `W every_runtime_program_is_in_the_gate_or_left_out`. Expected: `1
    passed`: the program and the gate's lists arrive together.
  - `X` (about 25 s). Expected: `0 passed; 1 failed`:
    `every_name_in_the_corpora_is_indexed` (`index_complete.rs:257`):
    "…/tests/runtime/ui_hydratable.nova does not check cleanly: [\"E0001
    cannot find function `render_hydratable` in this scope\"]".
- [ ] **Step 3: The code.** `patch_block t1-code`:
  - **`std/ui/lib.nova`:** `render(f, marks)`, which `render_to_string` and
    `render_hydratable` call, and `write_view`, `write_views`, `write_attr`
    and `escape` with `marks` (decisions 1 and 2); `is_text`; `hydrate`,
    `hydrate_with`, which takes the fallback it uses when the page holds no
    data (decision 29), `root_of`, which finds the root first, and
    `hydrate_into`: the walk under a root owner kept as `mount` keeps one,
    `on_mount` after it, and on a mismatch the report, the owner disposed and
    dropped from the roots, the root emptied, and the view built as `mount`
    builds it, as it is into a blank root, emptied first (decisions 13, 15, 16
    and 35); the walk itself, `Walk`, `Cursor`, `Found`, `blank` (decision 4),
    `advance`, `is_marker`, `clipped`, `found` (decision 14), `missed`,
    `left_over`, `walk_views`, `walk_view`, `adopt_attr` (decision 3),
    `adopt_text` and `adopt_region`. Its doc tests, which render natively:
    `render_hydratable`'s, `hydrate`'s and `hydrate_with`'s.
  - **The backends:** on WASM `backend_first_child`, `backend_kind`,
    `backend_text_of`, `backend_report` and `backend_data(root)`, which
    reads the root's next element when it is the data's `script` (decision
    32); natively each panics with `mount`'s message.
  - **`std/dom`:** `console_error`, an `extern "js"` function's wrapper,
    and the shim's `dom_console_error` (decision 12); `Element`'s and
    `tag`'s docs name comments.
- [ ] **Step 4: Run.**
  - The same six commands. Expected: `R ui_programs_run_and_survive_gc_stress`
    `1 passed`; `R ui_hydrate_panics_natively` `1 passed`; `W
    ui_hydratable` `2 passed`; `W hydrate_` `3 passed`; `W
    every_runtime_program_is_in_the_gate_or_left_out` `1 passed`; `X` `1
    passed`.
  - The whole binaries:
    - `cargo test -p nova-std`. Expected: `2 passed`.
    - `cargo test -p nova-codegen-wasm --lib`. Expected: `34 passed`.
    - `cargo test -p nova-doc --lib`. Expected: `61 passed`.
    - `cargo test -p nova-cli --test doc_test`. Expected: `17 passed`:
      `std/ui`'s three new examples run inside
      `doc_std_runs_stds_doc_tests_inside_a_project_or_not`.
    - `R`. Expected: `312 passed; 0 failed; 1 ignored` (311 before), in
      about a minute.
    - `W`. Expected: `227 passed` (222 before).
- [ ] **Step 5: The checks,** as the conventions list them, and `cargo run
  --locked -q -p nova-cli -- fmt --check tests/runtime/ui_hydratable.nova`,
  which prints nothing.
- [ ] **Step 6: Commit** `std/ui: render_hydratable, hydrate and hydrate_with; std/dom: console_error`.

**Patch `t1-tests`:**

<!-- patch t1-tests -->

````diff
diff --git a/crates/nova-cli/tests/run_tests.rs b/crates/nova-cli/tests/run_tests.rs
index 980d43e23060252fe3aa6fd4447371cef1a408c3..bc0dfbd36ac61e5283b387a3d1b26b1fd2dd1057 100644
--- a/crates/nova-cli/tests/run_tests.rs
+++ b/crates/nova-cli/tests/run_tests.rs
@@ -11307,11 +11307,11 @@ fn example_05_geometry_runs() {
 
 /// `std/ui`'s programs (spec
 /// `docs/superpowers/specs/2026-10-17-phase-4-3-std-ui-reactivity-and-views-design.md`
-/// §9.2), and `std/ui/router`'s paths and queries (spec 4.5 §3.2 to §3.5):
-/// each prints what `30-FRONTEND.md` §13's unit tests check, and runs
-/// the same with `NOVA_GC_STRESS=1` (collect on every allocation), so a part
-/// of the graph only the state's root keeps, freed, shows as wrong output or
-/// a crash.
+/// §9.2), `std/ui/router`'s paths and queries (spec 4.5 §3.2 to §3.5), and
+/// `render_hydratable`'s markers (spec 4.6 §9.2): each prints what
+/// `30-FRONTEND.md` §13's unit tests check, and runs the same with
+/// `NOVA_GC_STRESS=1` (collect on every allocation), so a part of the graph
+/// only the state's root keeps, freed, shows as wrong output or a crash.
 #[test]
 fn ui_programs_run_and_survive_gc_stress() {
     for name in [
@@ -11322,6 +11322,7 @@ fn ui_programs_run_and_survive_gc_stress() {
         "ui_collect",
         "ui_view",
         "ui_router",
+        "ui_hydratable",
     ] {
         let source = repo_root().join(format!("tests/runtime/{name}.nova"));
         let expected = std::fs::read_to_string(source.with_extension("stdout"))
@@ -11449,6 +11450,50 @@ fn ui_checks_every_name_and_never_parses_html() {
     }
 }
 
+/// Natively there is no page: `hydrate` and `hydrate_with` panic with
+/// `mount`'s message, before they look for a root or for data, and
+/// `render_hydratable` still renders (spec 4.6 §4.1).
+#[test]
+fn ui_hydrate_panics_natively() {
+    let dir = std::env::temp_dir().join("nova-ui-hydrate-native");
+    let _ = std::fs::remove_dir_all(&dir);
+    std::fs::create_dir_all(&dir).expect("temp dir");
+    for (file, call) in [
+        ("plain.nova", "hydrate(\"#app\", | | text(\"no page\"))"),
+        (
+            "data.nova",
+            "hydrate_with(\"#app\", \"none\", |s: String| text(s))",
+        ),
+    ] {
+        std::fs::write(
+            dir.join(file),
+            format!(
+                "import ui\nimport ui::html::{{text}}\n\nfn main() {{\n    println(render_hydratable(| | text(\"fine\")))\n    {call}\n    println(\"not reached\")\n}}\n"
+            ),
+        )
+        .expect("write the program");
+        let a = nova()
+            .current_dir(&dir)
+            .arg("run")
+            .arg(file)
+            .assert()
+            .failure();
+        let o = a.get_output();
+        assert_eq!(
+            String::from_utf8_lossy(&o.stdout).replace("\r\n", "\n"),
+            "fine\n",
+            "{file}"
+        );
+        let err = String::from_utf8_lossy(&o.stderr).replace("\r\n", "\n");
+        assert!(
+            err.ends_with(
+                "nova: panic: std/ui: mount needs a build for WASM; render_to_string renders on any target\n"
+            ),
+            "{file}: {err}"
+        );
+    }
+}
+
 /// Natively there is no page: `mount` panics, saying how to render, and
 /// `render_to_string` still renders (spec 4.3 §7.1).
 #[test]
diff --git a/crates/nova-cli/tests/wasm_tests.rs b/crates/nova-cli/tests/wasm_tests.rs
index 5fc01f46e7e6a4a705a4ba0e6c81edb03d7a11e6..f7974504826165f582f8030580b971d7589905b2 100644
--- a/crates/nova-cli/tests/wasm_tests.rs
+++ b/crates/nova-cli/tests/wasm_tests.rs
@@ -1902,6 +1902,8 @@ mod gate {
         ui_signal,
         // `component` and `view { … }` (spec 4.4 §9.2).
         ui_view,
+        // `render_hydratable` (spec 4.6 §9.2).
+        ui_hydratable,
     );
 
     /// The programs a test of their own in this module checks, by its name.
@@ -2224,6 +2226,7 @@ mod stress {
         ui_memo,
         ui_render,
         ui_signal,
+        ui_hydratable,
     );
 
     #[test]
@@ -2395,6 +2398,197 @@ fn ui_mount_never_inserts_html() {
     }
 }
 
+/// A hydration test of `tests/dom/` (spec
+/// `docs/superpowers/specs/2026-10-19-phase-4-6-ssr-hydration-and-ssg-design.md`
+/// §9.3), the pair end to end: the project `tests/dom/<name>/` has a server,
+/// `src/server.nova`, which, run natively from the project, prints a line
+/// per root of the page, its id and the HTML `render_hydratable` writes for
+/// it; and a client, `src/main.nova`, which, built for WASM, hydrates those
+/// roots. The harness, `<name>.mjs`, reads the server's output from
+/// `server.txt` beside it, puts each root's HTML into the page through the
+/// fake DOM's parser, runs the client with `env` set, and drives and prints
+/// the page. Its stdout is `<name>.stdout`. Returns its stderr.
+fn hydration_program(name: &str, env: &[(&str, &str)]) -> String {
+    let Some(mut node) = node() else {
+        return String::new();
+    };
+    let tests = repo_root().join("tests/dom");
+    let project = tests.join(name);
+    let expected = std::fs::read_to_string(tests.join(format!("{name}.stdout")))
+        .expect("the expected output")
+        .replace("\r\n", "\n");
+    let kind = if env.is_empty() {
+        "hydrate"
+    } else {
+        "hydrate-stress"
+    };
+    let dir = fresh(&format!("{kind}-{name}"));
+    let home = dir.join("nova-home");
+    let server = nova_at(&project, &home)
+        .args(["run", "src/server.nova"])
+        .assert()
+        .success();
+    std::fs::write(dir.join("server.txt"), stdout(&server)).expect("write the server's output");
+    let dist = dir.join("dist");
+    nova_at(&project, &home)
+        .args(["build", "--target", "wasm", "-o", &dist.to_string_lossy()])
+        .assert()
+        .success();
+    for file in ["fake_dom.mjs".to_string(), format!("{name}.mjs")] {
+        std::fs::copy(tests.join(&file), dir.join(&file)).expect("copy the harness");
+    }
+    let a = node
+        .arg(dir.join(format!("{name}.mjs")))
+        .envs(env.iter().copied())
+        .timeout(std::time::Duration::from_secs(60))
+        .assert()
+        .success();
+    assert_eq!(stdout(&a), expected, "{}", stderr(&a));
+    stderr(&a)
+}
+
+/// Hydration in place (spec 4.6 §4.1, §4.2): each root keeps the server's
+/// nodes, as the same objects and in order, and gains only the empty texts
+/// the server could not write. `on_mount` runs once the walk has added the
+/// handlers, so the click it makes counts. Clicks update the adopted nodes,
+/// a dynamic attribute and a property; an adopted row's handler runs; `show`
+/// and `for_each` build again between the server's markers, nested regions
+/// too; texts side by side, empty ones and a carriage return are adopted,
+/// and the dynamic one writes to its own node; `hydrate_with` reads the
+/// server's data, escaped for a `script` element, from the `script` after
+/// its root, past whitespace and a comment, though the view holds an
+/// element whose id is `nova-data` too; with no data after its root,
+/// `hydrate_with` walks a root the server filled with its fallback, and
+/// builds its fallback's view into an empty one (the user's decision 13,
+/// ADR 0045), as it does when the first element after the root holds other
+/// guests as JSON but is a `div` with the data's id and type, or a `script`
+/// with its id and no type, or a `p`, the data's `script` coming only after
+/// it; and a root with nothing, or only whitespace, a form feed among it,
+/// gets the view built, the whitespace removed first, with no report. Under
+/// stress the collector keeps the adopted nodes' seven handlers, and the
+/// rebuilt rows' three listeners replace the adopted rows' two: eight calls.
+#[test]
+fn hydrate_adopts_the_servers_nodes() {
+    hydration_program("hydrate", &[]);
+    let err = hydration_program("hydrate", &STRESS);
+    if node().is_none() {
+        return;
+    }
+    assert_eq!(calls_after_each(&err), [7, 7, 7, 7, 7, 8, 8, 8, 8], "{err}");
+}
+
+/// A mismatch (spec 4.6 §4.3): for each root, the first difference, in the
+/// spec's words, reported with `console.error`: another tag, another text
+/// (each side clipped to 40 characters), a missing region, its end, a
+/// missing node, a node left over in an element and in the root, a missing
+/// separator, or one where the client has no text, and a comment that is no
+/// marker, which a server written by hand can write. Then the root
+/// holds the client's view, built afresh, and none of the server's nodes.
+/// The walk's cleanups run; its `on_mount` callbacks never do; and its
+/// effects and handlers stop: a click on the server's button, which the walk
+/// had adopted, does nothing, and the count's effect runs once for a click
+/// on the client's. Under stress, the client's button's call is the only
+/// one left in std/rt's table.
+#[test]
+fn hydrate_reports_a_mismatch_and_builds_again() {
+    hydration_program("hydrate_mismatch", &[]);
+    let err = hydration_program("hydrate_mismatch", &STRESS);
+    if node().is_none() {
+        return;
+    }
+    let calls = calls_after_each(&err);
+    assert!(!calls.is_empty(), "{err}");
+    assert!(calls.iter().all(|&c| c == 1), "{err}");
+}
+
+/// `hydrate` and `hydrate_with` panic (spec 4.6 §4.1): when no element
+/// matches, as `mount` does, naming the function; and when the data's JSON,
+/// in the `nova-data` script after the root, does not parse, or its
+/// `from_json` refuses it. A page with no data is no panic: `hydrate_with`
+/// uses its fallback (the user's decision 13, ADR 0045), as
+/// `hydrate_adopts_the_servers_nodes` checks.
+#[test]
+fn hydrate_panics_without_its_root_or_on_data_that_does_not_parse() {
+    let Some(_) = node() else { return };
+    let dir = fresh("hydrate-panics");
+    std::fs::copy(
+        repo_root().join("tests/dom/fake_dom.mjs"),
+        dir.join("fake_dom.mjs"),
+    )
+    .expect("copy the fake DOM");
+    for (name, call, data, message) in [
+        (
+            "root",
+            "hydrate(\"#nope\", | | text(\"x\"))",
+            None,
+            "std/ui: hydrate: no element matches \"#nope\"",
+        ),
+        (
+            "root_with",
+            "hydrate_with(\"#nope\", \"none\", |s: String| text(s))",
+            Some("\"x\""),
+            "std/ui: hydrate_with: no element matches \"#nope\"",
+        ),
+        (
+            "bad_json",
+            "hydrate_with(\"#app\", \"none\", |s: String| text(s))",
+            Some("[\"x\","),
+            "std/ui: hydrate_with: the page's data does not parse: expected a value",
+        ),
+        (
+            "bad_value",
+            "hydrate_with(\"#app\", \"none\", |s: String| text(s))",
+            Some("7"),
+            "std/ui: hydrate_with: the page's data does not parse: expected a string",
+        ),
+    ] {
+        let data = match data {
+            Some(json) => format!(
+                "const data = document.createElement(\"script\");\n\
+                 data.setAttribute(\"type\", \"application/json\");\n\
+                 data.setAttribute(\"id\", \"nova-data\");\n\
+                 data.textContent = {json:?};\n\
+                 document.body.appendChild(data);\n"
+            ),
+            None => String::new(),
+        };
+        write(
+            &dir,
+            &[
+                (
+                    &format!("{name}/main.nova"),
+                    &format!(
+                        "import ui\nimport ui::html::{{text}}\n\nfn main() {{\n    {call}\n    println(\"not reached\")\n}}\n"
+                    ),
+                ),
+                (
+                    &format!("{name}/run.mjs"),
+                    &format!(
+                        "import {{ install }} from \"../fake_dom.mjs\";\ninstall();\n\
+                         const app = document.createElement(\"div\");\n\
+                         app.setAttribute(\"id\", \"app\");\n\
+                         app.innerHTML = \"x\";\n\
+                         document.body.appendChild(app);\n{data}\
+                         const {{ default: init }} = await import(\"./dist/app.js\");\nawait init();\n"
+                    ),
+                ),
+            ],
+        );
+        build_wasm(&dir.join(name), Path::new("main.nova"));
+        let a = Command::new("node")
+            .arg(dir.join(name).join("run.mjs"))
+            .timeout(std::time::Duration::from_secs(60))
+            .assert()
+            .code(134);
+        assert_eq!(stdout(&a), "", "{name}");
+        let err = stderr(&a);
+        assert!(
+            err.contains(&format!("nova: panic: {message}\n")),
+            "{name}: {err}"
+        );
+    }
+}
+
 /// The router by the hash (spec
 /// `docs/superpowers/specs/2026-10-18-phase-4-5-router-and-nova-dev-design.md`
 /// §3, §5.2): the root route at `/`; a link's click, its default prevented,
diff --git a/crates/nova-driver/tests/index_complete.rs b/crates/nova-driver/tests/index_complete.rs
index 793a227dea4d015c1d23536648ef0a8fa792eb0d..2de061f68ac2a9e797d50a91658e2d3e70ed5502 100644
--- a/crates/nova-driver/tests/index_complete.rs
+++ b/crates/nova-driver/tests/index_complete.rs
@@ -241,9 +241,9 @@ fn every_name_is_covered(a: &Analysis, file: FileId, failures: &mut Vec<String>)
 #[test]
 fn every_name_in_the_corpora_is_indexed() {
     let programs = programs();
-    // 8 examples, 144 top-level runtime programs and the modules program,
-    // on 2026-10-18, after 4.5 added `ui_router` and `ui_router_no_to`.
-    assert_eq!(programs.len(), 153, "{programs:#?}");
+    // 8 examples, 145 top-level runtime programs and the modules program,
+    // on 2026-10-19, after 4.6 added `ui_hydratable`.
+    assert_eq!(programs.len(), 154, "{programs:#?}");
     let mut files = 0;
     let mut failures: Vec<String> = Vec::new();
     for (k, path) in programs.iter().enumerate() {
@@ -270,9 +270,9 @@ fn every_name_in_the_corpora_is_indexed() {
             every_name_is_covered(&a, file, &mut failures);
         }
     }
-    // 8 + 144 + 3 files of user code, 05-geometry's library's 2, and
+    // 8 + 145 + 3 files of user code, 05-geometry's library's 2, and
     // native std's 21.
-    assert_eq!(files, 178);
+    assert_eq!(files, 179);
     failures.sort();
     failures.dedup();
     assert!(
diff --git a/tests/dom/fake_dom.mjs b/tests/dom/fake_dom.mjs
index e469918abca212b5d879130b37f9cb51e2f61b68..fef55b550f36f0356db8ffbf8e8a2bc5f5a4cbe9 100644
--- a/tests/dom/fake_dom.mjs
+++ b/tests/dom/fake_dom.mjs
@@ -8,6 +8,14 @@
 // collection under stress would free the closure behind it. And as a
 // browser does, it keeps an exception a listener throws, in `uncaught`,
 // and goes on: the dispatch that called the listener returns normally.
+//
+// For hydration (spec
+// docs/superpowers/specs/2026-10-19-phase-4-6-ssr-hydration-and-ssg-design.md
+// §9.3), it has comments, an `innerHTML` setter that parses the HTML
+// `render_hydratable` writes, a textarea's value that, as a browser's, is
+// not its `value` attribute, and a console whose errors it keeps, in
+// `errors`, and prints at once, so that a harness's output shows them in
+// order.
 
 // What listeners threw, as a browser reports them.
 const uncaught = [];
@@ -140,8 +148,12 @@ class FakeNode extends Target {
     siblings.splice(siblings.indexOf(this), 1);
     this.parentNode = null;
   }
+  // The text of every text node inside, as a browser's: comments' is not.
   get textContent() {
-    return this.childNodes.map((c) => c.textContent).join("");
+    return this.childNodes
+      .filter((c) => !(c instanceof FakeComment))
+      .map((c) => c.textContent)
+      .join("");
   }
   set textContent(value) {
     for (const c of [...this.childNodes]) c.remove();
@@ -169,8 +181,101 @@ class FakeText extends FakeNode {
   }
 }
 
+class FakeComment extends FakeNode {
+  constructor(data, doc) {
+    super("#comment", doc);
+    this.data = data;
+  }
+  get textContent() {
+    return this.data;
+  }
+  set textContent(value) {
+    this.data = String(value);
+  }
+  get outerHTML() {
+    return `<!--${this.data}-->`;
+  }
+}
+
 const VOID = new Set(["br", "hr", "img", "input", "link", "meta"]);
 
+// The void elements std/ui writes, which the parser closes as it opens them.
+const PARSED_VOID = new Set(["br", "hr", "img", "input"]);
+
+// The entities `render_hydratable` writes.
+const ENTITIES = new Map([
+  ["&amp;", "&"],
+  ["&lt;", "<"],
+  ["&gt;", ">"],
+  ["&quot;", '"'],
+  ["&#13;", "\r"],
+]);
+
+// `s` with its entities decoded; any other `&` is a SyntaxError, since
+// std/ui escapes every one.
+function decode(s) {
+  return s.replace(/&[^&;]*;?/g, (m) => {
+    if (!ENTITIES.has(m)) throw domError("SyntaxError", `the fake DOM does not parse "${m}"`);
+    return ENTITIES.get(m);
+  });
+}
+
+// Parse `html` into `parent`'s children, as a browser parses what
+// `render_hydratable` writes: elements, their double-quoted attributes, the
+// void elements std/ui knows, comments, and text with five entities. As a
+// browser's parser does, it first turns each CR LF, and each CR left, into
+// LF. Anything else, an unclosed or a stray tag included, is a SyntaxError.
+function parseInto(parent, html) {
+  const s = String(html).replace(/\r\n?/g, "\n");
+  const doc = parent.ownerDocument;
+  const open = [parent];
+  const top = () => open[open.length - 1];
+  const start = /<([a-zA-Z][a-zA-Z0-9-]*)/y;
+  const attr = /\s+([^\s"'>\/=]+)="([^"]*)"/y;
+  const end = /<\/([a-zA-Z][a-zA-Z0-9-]*)>/y;
+  let at = 0;
+  while (at < s.length) {
+    if (s.startsWith("<!--", at)) {
+      const close = s.indexOf("-->", at + 4);
+      if (close < 0) throw domError("SyntaxError", "an unclosed comment");
+      top().appendChild(new FakeComment(s.slice(at + 4, close), doc));
+      at = close + 3;
+    } else if (s.startsWith("</", at)) {
+      end.lastIndex = at;
+      const m = end.exec(s);
+      const tag = m === null ? null : m[1].toLowerCase();
+      if (tag === null || open.length === 1 || top().localName !== tag) {
+        throw domError("SyntaxError", `an unexpected end tag at ${at}: ${s.slice(at, at + 20)}`);
+      }
+      open.pop();
+      at = end.lastIndex;
+    } else if (s[at] === "<") {
+      start.lastIndex = at;
+      const m = start.exec(s);
+      if (m === null) throw domError("SyntaxError", `a "<" that opens no tag at ${at}`);
+      const el = doc.createElement(m[1]);
+      at = start.lastIndex;
+      for (;;) {
+        attr.lastIndex = at;
+        const a = attr.exec(s);
+        if (a === null) break;
+        el.setAttribute(a[1], decode(a[2]));
+        at = attr.lastIndex;
+      }
+      if (s[at] !== ">") throw domError("SyntaxError", `a tag not closed by ">" at ${at}`);
+      at += 1;
+      top().appendChild(el);
+      if (!PARSED_VOID.has(el.localName)) open.push(el);
+    } else {
+      const next = s.indexOf("<", at);
+      const stop = next < 0 ? s.length : next;
+      top().appendChild(new FakeText(decode(s.slice(at, stop)), doc));
+      at = stop;
+    }
+  }
+  if (open.length !== 1) throw domError("SyntaxError", `an unclosed <${top().localName}>`);
+}
+
 function escapeText(s) {
   return s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
 }
@@ -220,7 +325,11 @@ class FakeElement extends FakeNode {
   set className(value) {
     this.setAttribute("class", value);
   }
+  // A textarea's value is its text until it is set, as in a browser, which
+  // ignores a `value` attribute on one; another element's falls back to the
+  // attribute.
   get value() {
+    if (this.localName === "textarea") return this.ownValue ?? this.textContent;
     return this.ownValue ?? this.getAttribute("value") ?? "";
   }
   set value(v) {
@@ -258,6 +367,11 @@ class FakeElement extends FakeNode {
   get innerHTML() {
     return this.childNodes.map((c) => c.outerHTML).join("");
   }
+  // Replaces the children with what `html` parses to (`parseInto`).
+  set innerHTML(html) {
+    for (const c of [...this.childNodes]) c.remove();
+    parseInto(this, html);
+  }
   get outerHTML() {
     let attrs = "";
     for (const [name, value] of this.attrs) attrs += ` ${name}="${escapeAttribute(value)}"`;
@@ -326,6 +440,9 @@ class FakeDocument extends FakeNode {
   createTextNode(data) {
     return new FakeText(String(data), this);
   }
+  createComment(data) {
+    return new FakeComment(String(data), this);
+  }
   getElementById(id) {
     return this.documentElement.querySelectorAll(`#${id}`)[0] ?? null;
   }
@@ -425,6 +542,8 @@ class FakeWindow extends Target {
     this.routes = {};
     this.requests = [];
     this.uncaught = uncaught;
+    // What `console.error` was given.
+    this.errors = [];
   }
   // A reply after a timer, never from the network: a route's
   // `{ status, body }`, or a `TypeError` for `{ error }` or no route.
@@ -450,7 +569,9 @@ class FakeWindow extends Target {
 }
 
 // Install the page as `window`, `document`, `location`, `history` and
-// `fetch`, at `options.url` (`http://localhost/` by default).
+// `fetch`, at `options.url` (`http://localhost/` by default), and its
+// console's `error`, which keeps each message in `errors` and prints it to
+// stdout at once, after `console.error: `.
 export function install(options = {}) {
   const win = new FakeWindow(options.url ?? "http://localhost/");
   globalThis.window = win;
@@ -458,6 +579,11 @@ export function install(options = {}) {
   globalThis.location = win.location;
   globalThis.history = win.history;
   globalThis.fetch = (url, init) => win.fetch(url, init);
+  console.error = (...args) => {
+    const message = args.map(String).join(" ");
+    win.errors.push(message);
+    process.stdout.write(`console.error: ${message}\n`);
+  };
   return win;
 }
 
diff --git a/tests/dom/hydrate.mjs b/tests/dom/hydrate.mjs
new file mode 100644
index 0000000000000000000000000000000000000000..2bd967cf043d37603404cc3d5df97ccace80f559
--- /dev/null
+++ b/tests/dom/hydrate.mjs
@@ -0,0 +1,149 @@
+// Hydrate what the server rendered (spec
+// docs/superpowers/specs/2026-10-19-phase-4-6-ssr-hydration-and-ssg-design.md
+// §9.3). `server.txt`, beside this file, is what `hydrate/src/server.nova`
+// printed natively: each root's id and HTML, and the guests' JSON. The page
+// gets each root with that HTML, the data's `script` element after the
+// guests' root, past whitespace and a comment, which the client steps over,
+// and six roots the server left empty, one of them holding whitespace and
+// one a form feed. The guests' view holds an element whose id is
+// `nova-data` too. Three roots have, after them, an element that is not
+// the data though it holds other guests as JSON: after `decoy`, which the
+// server filled, a `div` with the data's id and type; after `untyped`,
+// empty, a `script` with the data's id and no type; and after `behind`,
+// empty, a plain `p` and then a `script` with the data's id and type, which
+// the client never reaches, since the first element after a root is the
+// data or there is none. Each hydrates with its fallback. Once the client has
+// hydrated them, this prints each root, says which of the server's nodes
+// are still there and which nodes are new, and reads the counter's
+// textarea, whose fixed value the server wrote as an attribute; then it
+// clicks, a turn apart, printing the page after each.
+import { readFileSync } from "node:fs";
+import { install } from "./fake_dom.mjs";
+const win = install();
+const server = new Map();
+for (const line of readFileSync(new URL("./server.txt", import.meta.url), "utf8").split("\n")) {
+  const at = line.indexOf(" ");
+  if (at > 0) server.set(line.slice(0, at), line.slice(at + 1));
+}
+// Guests that are not the data, which no root may show.
+const other = JSON.stringify({ title: "Not the data", names: ["Eve"] });
+const roots = {};
+for (const name of ["counter", "lists", "texts", "data", "filled", "fallback", "decoy", "untyped", "behind", "empty", "blank", "feed"]) {
+  const root = document.createElement("div");
+  root.setAttribute("id", name);
+  if (server.has(name)) root.innerHTML = server.get(name);
+  document.body.appendChild(root);
+  roots[name] = root;
+  if (name === "data") {
+    document.body.appendChild(document.createTextNode("\n  "));
+    document.body.appendChild(document.createComment(" the guests "));
+    document.body.appendChild(document.createTextNode("\n  "));
+    const data = document.createElement("script");
+    data.setAttribute("type", "application/json");
+    data.setAttribute("id", "nova-data");
+    data.textContent = server.get("nova-data");
+    document.body.appendChild(data);
+  }
+  if (name === "decoy" || name === "untyped") {
+    const next = document.createElement(name === "decoy" ? "div" : "script");
+    if (name === "decoy") next.setAttribute("type", "application/json");
+    next.setAttribute("id", "nova-data");
+    next.textContent = other;
+    document.body.appendChild(next);
+  }
+  if (name === "behind") {
+    const plain = document.createElement("p");
+    plain.textContent = "not the data";
+    document.body.appendChild(plain);
+    const late = document.createElement("script");
+    late.setAttribute("type", "application/json");
+    late.setAttribute("id", "nova-data");
+    late.textContent = other;
+    document.body.appendChild(late);
+  }
+}
+roots.blank.appendChild(document.createTextNode(" \n\t "));
+roots.feed.appendChild(document.createTextNode("\f\n"));
+
+// Each node under `root`, before its children.
+function nodes(root) {
+  const out = [];
+  const visit = (n) => {
+    for (const c of n.childNodes) {
+      out.push(c);
+      visit(c);
+    }
+  };
+  visit(root);
+  return out;
+}
+const before = {};
+for (const name in roots) before[name] = nodes(roots[name]);
+// The counter's textarea: the server wrote its value as an attribute,
+// which a textarea ignores, so until the client sets the property its value
+// is its text, "".
+const note = roots.counter.querySelector("#note");
+console.log(`counter: before hydration, the textarea's value is "${note.value}"`);
+
+// A string to print on one line.
+const line = (s) =>
+  s.replace(/\n/g, "\\n").replace(/\r/g, "\\r").replace(/\t/g, "\\t").replace(/\f/g, "\\f");
+const describe = (n) =>
+  n.nodeName === "#text" ? `text "${line(n.data)}"` : n.nodeName === "#comment" ? `<!--${n.data}-->` : `<${n.localName}>`;
+const show = (name) => console.log(`${name}: ${line(roots[name].innerHTML)}`);
+// How many of the server's nodes under the root are there still, whether in
+// their order, and the nodes the client made.
+function kept(name) {
+  const after = nodes(roots[name]);
+  const stayed = after.filter((n) => before[name].includes(n));
+  const made = after.filter((n) => !before[name].includes(n));
+  const order = stayed.every((n, i) => n === before[name][i]) ? "in order" : "out of order";
+  const news = made.length === 0 ? "none" : made.map(describe).join(", ");
+  console.log(`${name}: ${stayed.length} of ${before[name].length} server nodes kept, ${order}; new: ${news}`);
+}
+const turn = () => new Promise((r) => setTimeout(r, 0));
+async function click(selector) {
+  document.querySelector(selector).click();
+  await turn();
+}
+
+const { default: init } = await import("./dist/app.js");
+await init();
+for (const name in roots) {
+  show(name);
+  kept(name);
+}
+console.log(`counter: the input's value is ${roots.counter.querySelector("input").value}`);
+const same = roots.counter.querySelector("#note") === note;
+console.log(`counter: the textarea is the server's: ${same}; its value is "${note.value}"`);
+
+// The counter's adopted nodes follow its signal.
+await click("#inc");
+show("counter");
+kept("counter");
+console.log(`counter: the input's value is ${roots.counter.querySelector("input").value}`);
+
+// An adopted row's handler; then the regions build again between the
+// server's markers, which stay, but for the rows' own, which go with them.
+const markers = before.lists.filter((n) => n.nodeName === "#comment" && n.parentNode.localName !== "li");
+const rows = [...roots.lists.querySelectorAll("#rows li")].slice(1, 3);
+rows[1].click();
+await turn();
+for (const step of ["#toggle", "#toggle", "#more"]) {
+  await click(step);
+  show("lists");
+}
+console.log(`lists: the server's ${markers.length} outer markers are in the page: ${markers.every((m) => roots.lists.contains(m))}`);
+console.log(`lists: a row the server wrote is in the page: ${rows.some((r) => roots.lists.contains(r))}`);
+roots.lists.querySelectorAll("#rows li")[3].click();
+await turn();
+
+// The dynamic text writes to its own node, between the separators.
+await click("#word");
+show("texts");
+kept("texts");
+
+// The guests' list builds again with one more.
+await click("#guest");
+show("data");
+console.log(`console errors: ${win.errors.length}`);
diff --git a/tests/dom/hydrate.stdout b/tests/dom/hydrate.stdout
new file mode 100644
index 0000000000000000000000000000000000000000..e2417c24fac872b144d571c8a13b0e21ba8e53d6
--- /dev/null
+++ b/tests/dom/hydrate.stdout
@@ -0,0 +1,42 @@
+counter: before hydration, the textarea's value is ""
+hydrated
+counter: <div class="odd"><p>Count: <!--|-->1</p><button id="inc">+</button><input type="text" value="0"><textarea id="note" value="a fixed note"></textarea></div>
+counter: 9 of 9 server nodes kept, in order; new: none
+lists: <div><button id="toggle">toggle</button><button id="more">more</button><!--[--><p id="panel">panel</p><!--]--><ul id="rows"><li>first</li><!--[--><li>row 1<!--[--><span>!</span><!--]--></li><li>row 2<!--[--><span>!</span><!--]--></li><!--]--><li>last</li></ul><ul id="none"><!--[--><!--]--></ul></div>
+lists: 31 of 31 server nodes kept, in order; new: none
+texts: <div><p id="greeting">Hello, <!--|-->world<!--|--><!--|-->!</p><p id="alone"></p><p id="around"><span>s</span></p><p id="listed">[<!--[-->x<!--|-->y<!--]-->]</p><p id="cr">a\rb</p><button id="word">word</button></div>
+texts: 24 of 24 server nodes kept, in order; new: text "", text "", text "", text ""
+data: <div><h2>Guests &lt;/script&gt; &amp; &lt;b&gt;friends&lt;/b&gt;</h2><p id="nova-data">not the data</p><ul><!--[--><li>Ada</li><li>Grace</li><!--]--></ul><button id="guest">guest</button></div>
+data: 14 of 14 server nodes kept, in order; new: none
+filled: <div><h2>No data</h2><ul><!--[--><li>Hedy</li><!--]--></ul></div>
+filled: 8 of 8 server nodes kept, in order; new: none
+fallback: <div><h2>No data</h2><ul><li>Hedy</li></ul></div>
+fallback: 0 of 0 server nodes kept, in order; new: <div>, <h2>, text "No data", <ul>, text "", <li>, text "Hedy", text ""
+decoy: <div><h2>No data</h2><ul><!--[--><li>Hedy</li><!--]--></ul></div>
+decoy: 8 of 8 server nodes kept, in order; new: none
+untyped: <div><h2>No data</h2><ul><li>Hedy</li></ul></div>
+untyped: 0 of 0 server nodes kept, in order; new: <div>, <h2>, text "No data", <ul>, text "", <li>, text "Hedy", text ""
+behind: <div><h2>No data</h2><ul><li>Hedy</li></ul></div>
+behind: 0 of 0 server nodes kept, in order; new: <div>, <h2>, text "No data", <ul>, text "", <li>, text "Hedy", text ""
+empty: <p>builthere</p>
+empty: 0 of 0 server nodes kept, in order; new: <p>, text "built", text "", text "here"
+blank: <p>builthere</p>
+blank: 0 of 1 server nodes kept, in order; new: <p>, text "built", text "", text "here"
+feed: <p>builthere</p>
+feed: 0 of 1 server nodes kept, in order; new: <p>, text "built", text "", text "here"
+counter: the input's value is 1
+counter: the textarea is the server's: true; its value is "a fixed note"
+counter: <div class="even"><p>Count: <!--|-->2</p><button id="inc">+</button><input type="text" value="0"><textarea id="note" value="a fixed note"></textarea></div>
+counter: 9 of 9 server nodes kept, in order; new: none
+counter: the input's value is 2
+clicked row 2
+lists: <div><button id="toggle">toggle</button><button id="more">more</button><!--[--><!--]--><ul id="rows"><li>first</li><!--[--><li>row 1<!--[--><!--]--></li><li>row 2<!--[--><!--]--></li><!--]--><li>last</li></ul><ul id="none"><!--[--><!--]--></ul></div>
+lists: <div><button id="toggle">toggle</button><button id="more">more</button><!--[--><p id="panel">panel</p><!--]--><ul id="rows"><li>first</li><!--[--><li>row 1<!--[--><span>!</span><!--]--></li><li>row 2<!--[--><span>!</span><!--]--></li><!--]--><li>last</li></ul><ul id="none"><!--[--><!--]--></ul></div>
+lists: <div><button id="toggle">toggle</button><button id="more">more</button><!--[--><p id="panel">panel</p><!--]--><ul id="rows"><li>first</li><!--[--><li>row 1<span>!</span></li><li>row 2<span>!</span></li><li>row 3<span>!</span></li><!--]--><li>last</li></ul><ul id="none"><!--[--><!--]--></ul></div>
+lists: the server's 6 outer markers are in the page: true
+lists: a row the server wrote is in the page: false
+clicked row 3
+texts: <div><p id="greeting">Hello, <!--|-->there<!--|--><!--|-->!</p><p id="alone"></p><p id="around"><span>s</span></p><p id="listed">[<!--[-->x<!--|-->y<!--]-->]</p><p id="cr">a\rb</p><button id="word">word</button></div>
+texts: 24 of 24 server nodes kept, in order; new: text "", text "", text "", text ""
+data: <div><h2>Guests &lt;/script&gt; &amp; &lt;b&gt;friends&lt;/b&gt;</h2><p id="nova-data">not the data</p><ul><!--[--><li>Ada</li><li>Grace</li><li>Hedy</li><!--]--></ul><button id="guest">guest</button></div>
+console errors: 0
diff --git a/tests/dom/hydrate/nova.toml b/tests/dom/hydrate/nova.toml
new file mode 100644
index 0000000000000000000000000000000000000000..aa3c06a9642d175132e70139619ea258d35c4c0e
--- /dev/null
+++ b/tests/dom/hydrate/nova.toml
@@ -0,0 +1,5 @@
+[package]
+name = "hydrate"
+version = "0.1.0"
+edition = "2026"
+description = "Views a native server renders and a WASM client hydrates, which wasm_tests.rs runs under the fake DOM"
diff --git a/tests/dom/hydrate/src/main.nova b/tests/dom/hydrate/src/main.nova
new file mode 100644
index 0000000000000000000000000000000000000000..94307adfcba9a017dc115f4835e9ffd53a70d2d4
--- /dev/null
+++ b/tests/dom/hydrate/src/main.nova
@@ -0,0 +1,44 @@
+//! The client's side of `tests/dom/hydrate.mjs` (spec
+//! docs/superpowers/specs/2026-10-19-phase-4-6-ssr-hydration-and-ssg-design.md
+//! §4, §9.3): it hydrates each root the server rendered, the guests with
+//! the server's data, and two with no data after it with the guests it falls
+//! back on, one of them followed by a `div` with the data's id and type; and
+//! the roots the server left empty: three with those guests, one of them
+//! followed by a `script` with the data's id and no type, one by a `p` and
+//! then the data's `script`, and three, one blank and one holding a form
+//! feed, with a fresh view.
+
+import dom
+import ui
+import view::{counter, fallback, fresh, guests, listed, lists, texts}
+
+fn main() {
+    // `on_mount` runs once the walk has given the button its handler, so its
+    // click counts.
+    hydrate(
+        "#counter",
+        || {
+            on_mount(
+                || {
+                    match query_selector("#inc") {
+                        Some(b) => b.click()
+                        None => println("no button")
+                    }
+                },
+            )
+            counter()
+        },
+    )
+    hydrate("#lists", lists)
+    hydrate("#texts", texts)
+    hydrate_with("#data", fallback(), guests)
+    hydrate_with("#filled", fallback(), listed)
+    hydrate_with("#fallback", fallback(), listed)
+    hydrate_with("#decoy", fallback(), listed)
+    hydrate_with("#untyped", fallback(), listed)
+    hydrate_with("#behind", fallback(), listed)
+    hydrate("#empty", fresh)
+    hydrate("#blank", fresh)
+    hydrate("#feed", fresh)
+    println("hydrated")
+}
diff --git a/tests/dom/hydrate/src/server.nova b/tests/dom/hydrate/src/server.nova
new file mode 100644
index 0000000000000000000000000000000000000000..496e102c1580fa5f821f54c79896570dd9bb01f1
--- /dev/null
+++ b/tests/dom/hydrate/src/server.nova
@@ -0,0 +1,43 @@
+//! The server's side of `tests/dom/hydrate.mjs`: for each root of the page,
+//! a line of its id and the HTML `render_hydratable` writes for its view,
+//! two of them rendered with the guests the client falls back on, and a
+//! line `nova-data` and the guests as JSON, made safe for a `script`
+//! element as the spec writes it (spec
+//! docs/superpowers/specs/2026-10-19-phase-4-6-ssr-hydration-and-ssg-design.md
+//! §6.2).
+
+import ui
+import view::{Guests, counter, fallback, guests, listed, lists, texts}
+
+// `json` with `<`, `>` and `&` written as escapes, which only JSON strings
+// hold, so that a `script` element holds it whole.
+fn safe(json: String) -> String {
+    let cs = json.chars()
+    let mut parts = [""; cs.len()]
+    let mut i = 0
+    while i < cs.len() {
+        let c = cs[i]
+        parts[i] = if c == '<' {
+            "\\u003c"
+        } else if c == '>' {
+            "\\u003e"
+        } else if c == '&' {
+            "\\u0026"
+        } else {
+            "${c}"
+        }
+        i = i + 1
+    }
+    "".join(parts)
+}
+
+fn main() {
+    let g = Guests { title: "Guests </script> & <b>friends</b>", names: ["Ada", "Grace"] }
+    println("counter ${render_hydratable(counter)}")
+    println("lists ${render_hydratable(lists)}")
+    println("texts ${render_hydratable(texts)}")
+    println("data ${render_hydratable(|| guests(g))}")
+    println("filled ${render_hydratable(|| listed(fallback()))}")
+    println("decoy ${render_hydratable(|| listed(fallback()))}")
+    println("nova-data ${safe(stringify(g.to_json()))}")
+}
diff --git a/tests/dom/hydrate/src/view.nova b/tests/dom/hydrate/src/view.nova
new file mode 100644
index 0000000000000000000000000000000000000000..62212c45506242d7725cc3b61b3592903a8abf8f
--- /dev/null
+++ b/tests/dom/hydrate/src/view.nova
@@ -0,0 +1,203 @@
+//! The views `server.nova` renders natively and `main.nova` hydrates under
+//! the fake DOM (spec
+//! docs/superpowers/specs/2026-10-19-phase-4-6-ssr-hydration-and-ssg-design.md
+//! §9.3), one for each root of the page.
+
+import ui
+import ui::html::{
+    attr,
+    button,
+    div,
+    dyn_attr,
+    dyn_text,
+    h2,
+    id,
+    input,
+    li,
+    on_click,
+    p,
+    span,
+    text,
+    textarea,
+    ul,
+}
+
+/// A count, its label and its number side by side, a button that adds one,
+/// a class and an input's value that follow the count, and a textarea whose
+/// value is a fixed property: the server can only write it as an attribute,
+/// which a textarea ignores, so hydration must set it.
+pub fn counter() -> View {
+    let count = signal(0)
+    div(
+        [dyn_attr("class", || if count.value() % 2 == 0 { "even" } else { "odd" })],
+        [
+            p([], [text("Count: "), dyn_text(|| "${count.value()}")]),
+            button([id("inc"), on_click(|| count.update(|n: Int| n + 1))], [text("+")]),
+            input([attr("type", "text"), DynProperty("value", || "${count.value()}")]),
+            textarea([id("note"), Property("value", "a fixed note")], []),
+        ],
+    )
+}
+
+// `xs`, and `x` after them.
+fn plus(xs: [String], x: String) -> [String] {
+    let mut out = [x; xs.len() + 1]
+    let mut i = 0
+    while i < xs.len() {
+        out[i] = xs[i]
+        i = i + 1
+    }
+    out
+}
+
+/// A `show` and a `for_each` whose rows hold a `show` each, between fixed
+/// rows, and a `for_each` of nothing, with buttons that change them.
+pub fn lists() -> View {
+    let open = signal(true)
+    let rows = signal(["row 1", "row 2"])
+    let none: [String] = []
+    div(
+        [],
+        [
+            button([id("toggle"), on_click(|| open.update(|o: Bool| !o))], [text("toggle")]),
+            button(
+                [
+                    id("more"),
+                    on_click(|| rows.update(|xs: [String]| plus(xs, "row ${xs.len() + 1}"))),
+                ],
+                [text("more")],
+            ),
+            show(open, || p([id("panel")], [text("panel")])),
+            ul(
+                [id("rows")],
+                [
+                    li([], [text("first")]),
+                    for_each(
+                        rows,
+                        |r: String| li(
+                            [on_click(|| println("clicked ${r}"))],
+                            [text(r), show(open, || span([], [text("!")]))],
+                        ),
+                    ),
+                    li([], [text("last")]),
+                ],
+            ),
+            ul([id("none")], [for_each(signal(none), |r: String| li([], [text(r)]))]),
+        ],
+    )
+}
+
+/// Texts side by side, fixed and dynamic, with empty ones among them and
+/// alone; texts in a region; a carriage return; and a button that changes
+/// the dynamic text.
+pub fn texts() -> View {
+    let word = signal("world")
+    div(
+        [],
+        [
+            p([id("greeting")], [text("Hello, "), dyn_text(|| word.value()), text(""), text("!")]),
+            p([id("alone")], [text("")]),
+            p([id("around")], [dyn_text(|| ""), span([], [text("s")]), text("")]),
+            p(
+                [id("listed")],
+                [text("["), for_each(signal(["x", "y"]), |s: String| text(s)), text("]")],
+            ),
+            p([id("cr")], [text("a\rb")]),
+            button([id("word"), on_click(|| word.set("there"))], [text("word")]),
+        ],
+    )
+}
+
+/// The data the server hands the client: a title, which holds what JSON in
+/// a page must escape, and names.
+pub record Guests { title: String, names: [String] }
+
+impl ToJson for Guests {
+    fn to_json(self) -> JsonValue {
+        let mut names = [Null; self.names.len()]
+        let mut i = 0
+        while i < self.names.len() {
+            names[i] = self.names[i].to_json()
+            i = i + 1
+        }
+        let mut fields: Map<String, JsonValue> = Map::new()
+        fields.insert("title", self.title.to_json())
+        fields.insert("names", Array(names))
+        Object(fields)
+    }
+}
+
+// The string `v` holds, or `""` when it holds none.
+fn string_of(v: Option<JsonValue>) -> String {
+    match v {
+        Some(x) => {
+            match String::from_json(x) {
+                Ok(s) => s
+                Err(e) => ""
+            }
+        }
+        None => ""
+    }
+}
+
+impl FromJson for Guests {
+    fn from_json(v: JsonValue) -> Result<Guests, JsonError> {
+        let fields = match v {
+            Object(m) => m
+            _ => return Err(JsonError { msg: "expected an object", at: 0 })
+        }
+        let list = match fields.get("names") {
+            Some(x) => {
+                match x {
+                    Array(xs) => xs
+                    _ => return Err(JsonError { msg: "expected names", at: 0 })
+                }
+            }
+            None => return Err(JsonError { msg: "expected names", at: 0 })
+        }
+        let mut names = [""; list.len()]
+        let mut i = 0
+        while i < list.len() {
+            names[i] = string_of(Some(list[i]))
+            i = i + 1
+        }
+        Ok(Guests { title: string_of(fields.get("title")), names: names })
+    }
+}
+
+/// The server's data, the title and a list of names, with a button that adds
+/// one. Its paragraph's id is `nova-data`, as the data's own `script` is,
+/// and `hydrate_with` reads the data from the `script` all the same.
+pub fn guests(g: Guests) -> View {
+    let names = signal(g.names)
+    div(
+        [],
+        [
+            h2([], [text(g.title)]),
+            p([id("nova-data")], [text("not the data")]),
+            ul([], [for_each(names, |n: String| li([], [text(n)]))]),
+            button(
+                [id("guest"), on_click(|| names.update(|xs: [String]| plus(xs, "Hedy")))],
+                [text("guest")],
+            ),
+        ],
+    )
+}
+
+/// The guests `hydrate_with` falls back on where the page holds no data from
+/// the server.
+pub fn fallback() -> Guests { Guests { title: "No data", names: ["Hedy"] } }
+
+/// The guests' title and names, with no handler.
+pub fn listed(g: Guests) -> View {
+    div(
+        [],
+        [
+            h2([], [text(g.title)]),
+            ul([], [for_each(signal(g.names), |n: String| li([], [text(n)]))]),
+        ],
+    )
+}
+
+/// What the client builds where the server wrote nothing.
+pub fn fresh() -> View { p([], [text("built"), text(""), dyn_text(|| "here")]) }
diff --git a/tests/dom/hydrate_mismatch.mjs b/tests/dom/hydrate_mismatch.mjs
new file mode 100644
index 0000000000000000000000000000000000000000..be6cafc6bfb40a5c8d3f9146a0d152fbf503b14c
--- /dev/null
+++ b/tests/dom/hydrate_mismatch.mjs
@@ -0,0 +1,52 @@
+// Hydrate other views than the server rendered (spec
+// docs/superpowers/specs/2026-10-19-phase-4-6-ssr-hydration-and-ssg-design.md
+// §4.3, §9.3). `server.txt`, beside this file, is what
+// `hydrate_mismatch/src/server.nova` printed natively: each root's id and
+// HTML. The client reports each root's first difference with
+// `console.error`, which the fake DOM prints in order, and builds the
+// client's view there instead. This prints each root then, and says whether
+// any of the server's nodes is left; then it clicks the server's button,
+// whose handler the rebuild removed, and the client's, a turn apart.
+import { readFileSync } from "node:fs";
+import { install } from "./fake_dom.mjs";
+const win = install();
+const roots = {};
+for (const line of readFileSync(new URL("./server.txt", import.meta.url), "utf8").split("\n")) {
+  const at = line.indexOf(" ");
+  if (at <= 0) continue;
+  const root = document.createElement("div");
+  root.setAttribute("id", line.slice(0, at));
+  root.innerHTML = line.slice(at + 1);
+  document.body.appendChild(root);
+  roots[line.slice(0, at)] = root;
+}
+// Each node under `root`.
+function nodes(root) {
+  const out = [];
+  const visit = (n) => {
+    for (const c of n.childNodes) {
+      out.push(c);
+      visit(c);
+    }
+  };
+  visit(root);
+  return out;
+}
+const before = {};
+for (const name in roots) before[name] = nodes(roots[name]);
+const serverButton = roots.stops.querySelector("#go");
+const turn = () => new Promise((r) => setTimeout(r, 0));
+
+const { default: init } = await import("./dist/app.js");
+await init();
+for (const name in roots) {
+  const left = before[name].filter((n) => roots[name].contains(n)).length;
+  console.log(`${name}: ${roots[name].innerHTML} (server nodes left: ${left})`);
+}
+serverButton.click();
+await turn();
+console.log("clicked the server's button");
+roots.stops.querySelector("#go").click();
+await turn();
+console.log(`stops: ${roots.stops.innerHTML}`);
+console.log(`console errors: ${win.errors.length}`);
diff --git a/tests/dom/hydrate_mismatch.stdout b/tests/dom/hydrate_mismatch.stdout
new file mode 100644
index 0000000000000000000000000000000000000000..01b24457fb1d4dc4759e9ab664c949e945b31488
--- /dev/null
+++ b/tests/dom/hydrate_mismatch.stdout
@@ -0,0 +1,33 @@
+console.error: nova: hydration mismatch: expected <p>, found <div>; building the view again
+tag: a cleanup ran
+tag: on_mount ran
+console.error: nova: hydration mismatch: expected text "The client reads this text, which is lon", found text "The server wrote this text, which is lon"; building the view again
+console.error: nova: hydration mismatch: expected a region, found <p>; building the view again
+console.error: nova: hydration mismatch: expected the end of a region, found <li>; building the view again
+console.error: nova: hydration mismatch: expected <li>, found the end of a region; building the view again
+console.error: nova: hydration mismatch: expected <em>, found nothing; building the view again
+console.error: nova: hydration mismatch: expected nothing, found <em>; building the view again
+console.error: nova: hydration mismatch: expected text "b", found nothing; building the view again
+console.error: nova: hydration mismatch: expected <span>, found text "b"; building the view again
+console.error: nova: hydration mismatch: expected nothing, found <p>; building the view again
+console.error: nova: hydration mismatch: expected <p>, found a comment; building the view again
+stops: the count reads 0
+console.error: nova: hydration mismatch: expected <em>, found <span>; building the view again
+stops: the count reads 0
+hydrated
+tag: <p>a</p> (server nodes left: 0)
+text: <p>The client reads this text, which is longer than forty characters</p> (server nodes left: 0)
+region: <div><p>x</p></div> (server nodes left: 0)
+end: <ul><li>a</li></ul> (server nodes left: 0)
+missing: <ul><li>a</li><li>b</li></ul> (server nodes left: 0)
+nothing: <div><span></span><em></em></div> (server nodes left: 0)
+over: <div><span></span></div> (server nodes left: 0)
+separator: <p>ab</p> (server nodes left: 0)
+joined: <p>a<span></span></p> (server nodes left: 0)
+root:  (server nodes left: 0)
+comment: <div><p>a</p></div> (server nodes left: 0)
+stops: <div><button id="go">go</button>0<em>client</em></div> (server nodes left: 0)
+clicked the server's button
+stops: the count reads 1
+stops: <div><button id="go">go</button>1<em>client</em></div>
+console errors: 12
diff --git a/tests/dom/hydrate_mismatch/nova.toml b/tests/dom/hydrate_mismatch/nova.toml
new file mode 100644
index 0000000000000000000000000000000000000000..076bebdab4edf679bdf8a632cf7e0a3b0728d700
--- /dev/null
+++ b/tests/dom/hydrate_mismatch/nova.toml
@@ -0,0 +1,5 @@
+[package]
+name = "hydrate-mismatch"
+version = "0.1.0"
+edition = "2026"
+description = "Views a native server renders and a WASM client hydrates as other views, which wasm_tests.rs runs under the fake DOM"
diff --git a/tests/dom/hydrate_mismatch/src/main.nova b/tests/dom/hydrate_mismatch/src/main.nova
new file mode 100644
index 0000000000000000000000000000000000000000..ce5b3e0cf6bb17d1e8186435449f0f238dc134dd
--- /dev/null
+++ b/tests/dom/hydrate_mismatch/src/main.nova
@@ -0,0 +1,46 @@
+//! The client's side of `tests/dom/hydrate_mismatch.mjs` (spec
+//! docs/superpowers/specs/2026-10-19-phase-4-6-ssr-hydration-and-ssg-design.md
+//! §4.3): it hydrates each root with a view other than the server's, so each
+//! reports its first difference and builds the view again.
+
+import ui
+import view::{
+    comment_client,
+    end_client,
+    joined_client,
+    missing_client,
+    nothing_client,
+    over_client,
+    region_client,
+    root_client,
+    separator_client,
+    stops_client,
+    tag_client,
+    text_client,
+}
+
+fn main() {
+    // The walk's cleanups run when it is disposed, and its `on_mount`
+    // callbacks never; the build's run once it is done.
+    hydrate(
+        "#tag",
+        || {
+            on_cleanup(|| println("tag: a cleanup ran"))
+            on_mount(|| println("tag: on_mount ran"))
+            tag_client()
+        },
+    )
+    hydrate("#text", text_client)
+    hydrate("#region", region_client)
+    hydrate("#end", end_client)
+    hydrate("#missing", missing_client)
+    hydrate("#nothing", nothing_client)
+    hydrate("#over", over_client)
+    hydrate("#separator", separator_client)
+    hydrate("#joined", joined_client)
+    hydrate("#root", root_client)
+    hydrate("#comment", comment_client)
+    let n = signal(0)
+    hydrate("#stops", || stops_client(n))
+    println("hydrated")
+}
diff --git a/tests/dom/hydrate_mismatch/src/server.nova b/tests/dom/hydrate_mismatch/src/server.nova
new file mode 100644
index 0000000000000000000000000000000000000000..b98a7bbe4716ab4355adefed0caf64f66067c6e8
--- /dev/null
+++ b/tests/dom/hydrate_mismatch/src/server.nova
@@ -0,0 +1,34 @@
+//! The server's side of `tests/dom/hydrate_mismatch.mjs`: for each root of
+//! the page, a line of its id and the HTML `render_hydratable` writes for
+//! the server's view of it; and one root's HTML written by hand, with a
+//! comment that is no marker where the client's view has an element.
+
+import ui
+import view::{
+    end_server,
+    joined_server,
+    missing_server,
+    nothing_server,
+    over_server,
+    region_server,
+    root_server,
+    separator_server,
+    stops_server,
+    tag_server,
+    text_server,
+}
+
+fn main() {
+    println("tag ${render_hydratable(tag_server)}")
+    println("text ${render_hydratable(text_server)}")
+    println("region ${render_hydratable(region_server)}")
+    println("end ${render_hydratable(end_server)}")
+    println("missing ${render_hydratable(missing_server)}")
+    println("nothing ${render_hydratable(nothing_server)}")
+    println("over ${render_hydratable(over_server)}")
+    println("separator ${render_hydratable(separator_server)}")
+    println("joined ${render_hydratable(joined_server)}")
+    println("root ${render_hydratable(root_server)}")
+    println("comment <div><!-- a note --><p>a</p></div>")
+    println("stops ${render_hydratable(stops_server)}")
+}
diff --git a/tests/dom/hydrate_mismatch/src/view.nova b/tests/dom/hydrate_mismatch/src/view.nova
new file mode 100644
index 0000000000000000000000000000000000000000..fda385aea0754de9d3807263c9b931b65ee68b15
--- /dev/null
+++ b/tests/dom/hydrate_mismatch/src/view.nova
@@ -0,0 +1,88 @@
+//! The views of `tests/dom/hydrate_mismatch.mjs` (spec
+//! docs/superpowers/specs/2026-10-19-phase-4-6-ssr-hydration-and-ssg-design.md
+//! §4.3, §9.3): for each root, the view `server.nova` renders, and another,
+//! a little different, which `main.nova` hydrates there.
+
+import ui
+import ui::html::{button, div, dyn_text, em, id, li, on_click, p, span, text, ul}
+
+fn items(xs: [String]) -> View { ul([], [for_each(signal(xs), |x: String| li([], [text(x)]))]) }
+
+/// Another tag.
+pub fn tag_server() -> View { div([], [text("a")]) }
+
+pub fn tag_client() -> View { p([], [text("a")]) }
+
+/// Another text, longer than a mismatch quotes.
+pub fn text_server() -> View {
+    p([], [text("The server wrote this text, which is longer than forty characters")])
+}
+
+pub fn text_client() -> View {
+    p([], [dyn_text(|| "The client reads this text, which is longer than forty characters")])
+}
+
+/// A region where the server had none.
+pub fn region_server() -> View { div([], [p([], [text("x")])]) }
+
+pub fn region_client() -> View { div([], [show(signal(true), || p([], [text("x")]))]) }
+
+/// A region with fewer views, and one with more.
+pub fn end_server() -> View { items(["a", "b"]) }
+
+pub fn end_client() -> View { items(["a"]) }
+
+pub fn missing_server() -> View { items(["a"]) }
+
+pub fn missing_client() -> View { items(["a", "b"]) }
+
+/// An element with a child more, and one with a child less.
+pub fn nothing_server() -> View { div([], [span([], [])]) }
+
+pub fn nothing_client() -> View { div([], [span([], []), em([], [])]) }
+
+pub fn over_server() -> View { div([], [span([], []), em([], [])]) }
+
+pub fn over_client() -> View { div([], [span([], [])]) }
+
+/// A text more, after another, and an element after a text where the server
+/// had a second text.
+pub fn separator_server() -> View { p([], [text("a")]) }
+
+pub fn separator_client() -> View { p([], [text("a"), dyn_text(|| "b")]) }
+
+pub fn joined_server() -> View { p([], [text("a"), text("b")]) }
+
+pub fn joined_client() -> View { p([], [text("a"), span([], [])]) }
+
+/// A root with more than the client's view: a node left after it.
+pub fn root_server() -> View { p([], [text("x")]) }
+
+pub fn root_client() -> View { text("") }
+
+/// An element where the server's HTML holds a comment that is no marker,
+/// which a server written by hand can write, as `server.nova` does.
+pub fn comment_client() -> View { div([], [p([], [text("a")])]) }
+
+/// A difference after the walk has adopted a button, given it a handler,
+/// and begun a dynamic text's effect, which the rebuild stops.
+pub fn stops_server() -> View {
+    div([], [button([id("go")], [text("go")]), dyn_text(|| "0"), span([], [text("server")])])
+}
+
+pub fn stops_client(n: Signal<Int>) -> View {
+    div(
+        [],
+        [
+            button([id("go"), on_click(|| n.update(|k: Int| k + 1))], [text("go")]),
+            dyn_text(
+                || {
+                    let k = n.value()
+                    println("stops: the count reads ${k}")
+                    "${k}"
+                },
+            ),
+            em([], [text("client")]),
+        ],
+    )
+}
diff --git a/tests/runtime/ui_hydratable.nova b/tests/runtime/ui_hydratable.nova
new file mode 100644
index 0000000000000000000000000000000000000000..409830373b9497ded795ac32f1869a56eb69c670
--- /dev/null
+++ b/tests/runtime/ui_hydratable.nova
@@ -0,0 +1,69 @@
+// `render_hydratable` (spec
+// docs/superpowers/specs/2026-10-19-phase-4-6-ssr-hydration-and-ssg-design.md
+// §3, §9.2): nested regions, empty regions, empty texts between others,
+// texts side by side in an element and in a region, and a carriage return in
+// text and in an attribute; each line then `render_to_string` of the same
+// view, which is as it was, with its carriage returns shown as `\r`.
+import ui
+import ui::html::{attr, div, dyn_text, li, p, span, text, ul}
+
+// `s` with each carriage return written `\r`, to print it.
+fn shown(s: String) -> String {
+    let cs = s.chars()
+    let mut parts = [""; cs.len()]
+    let mut i = 0
+    while i < cs.len() {
+        parts[i] = if cs[i] == '\r' { "\\r" } else { "${cs[i]}" }
+        i = i + 1
+    }
+    "".join(parts)
+}
+
+// Both renders of `f`.
+fn both(f: fn() -> View) {
+    println(render_hydratable(f))
+    println(shown(render_to_string(f)))
+}
+
+fn main() {
+    // Nested regions: a `show` holding a `for_each`, and a `for_each` whose
+    // items hold a `show`.
+    let open = signal(true)
+    let items = signal(["a", "b"])
+    both(|| div([], [show(open, || ul([], [for_each(items, |s: String| li([], [text(s)]))]))]))
+    both(
+        || ul(
+            [],
+            [for_each(items, |s: String| li([], [text(s), show(open, || span([], [text("!")]))]))],
+        ),
+    )
+
+    // Empty regions: a hidden `show`, and a `for_each` over nothing, beside
+    // text.
+    open.set(false)
+    let none: [String] = []
+    items.set(none)
+    both(|| div([], [show(open, || p([], [text("hidden")])), text("x")]))
+    both(|| ul([], [text("before"), for_each(items, |s: String| li([], [text(s)])), text("after")]))
+
+    // Empty texts between other texts, and alone.
+    both(|| p([], [text("a"), text(""), text("b")]))
+    both(|| p([], [text(""), dyn_text(|| ""), text("")]))
+    both(|| p([], [text("")]))
+    both(|| p([], [dyn_text(|| ""), span([], [text("s")]), text("")]))
+
+    // Texts side by side in an element, fixed and dynamic, and in a region,
+    // where the marker parts a text from the region's own.
+    let n = signal(2)
+    both(|| div([], [text("count: "), dyn_text(|| "${n.value()}"), text(" times")]))
+    both(|| div([], [text("x"), span([], [text("y")]), text("z")]))
+    items.set(["one", "two", "three"])
+    both(|| p([], [text("["), for_each(items, |s: String| text(s)), text("]")]))
+    open.set(true)
+    both(|| p([], [show(open, || text("a")), show(open, || text("b"))]))
+
+    // A carriage return, alone and before a line feed, in text and in an
+    // attribute; escaping as before.
+    both(|| p([attr("title", "one\rtwo\r\n<\"&\">")], [text("a\rb\r\nc & <d>")]))
+    both(|| p([], [dyn_text(|| "\r"), text("<!--|-->")]))
+}
diff --git a/tests/runtime/ui_hydratable.stdout b/tests/runtime/ui_hydratable.stdout
new file mode 100644
index 0000000000000000000000000000000000000000..9e517ff0e83fc3314799660af2151a87da20a5a9
--- /dev/null
+++ b/tests/runtime/ui_hydratable.stdout
@@ -0,0 +1,32 @@
+<div><!--[--><ul><!--[--><li>a</li><li>b</li><!--]--></ul><!--]--></div>
+<div><ul><li>a</li><li>b</li></ul></div>
+<ul><!--[--><li>a<!--[--><span>!</span><!--]--></li><li>b<!--[--><span>!</span><!--]--></li><!--]--></ul>
+<ul><li>a<span>!</span></li><li>b<span>!</span></li></ul>
+<div><!--[--><!--]-->x</div>
+<div>x</div>
+<ul>before<!--[--><!--]-->after</ul>
+<ul>beforeafter</ul>
+<p>a<!--|--><!--|-->b</p>
+<p>ab</p>
+<p><!--|--><!--|--></p>
+<p></p>
+<p></p>
+<p></p>
+<p><span>s</span></p>
+<p><span>s</span></p>
+<div>count: <!--|-->2<!--|--> times</div>
+<div>count: 2 times</div>
+<div>x<span>y</span>z</div>
+<div>x<span>y</span>z</div>
+<p>[<!--[-->one<!--|-->two<!--|-->three<!--]-->]</p>
+<p>[onetwothree]</p>
+<p><!--[-->a<!--]--><!--[-->b<!--]--></p>
+<p>ab</p>
+<p title="one&#13;two&#13;
+&lt;&quot;&amp;&quot;&gt;">a&#13;b&#13;
+c &amp; &lt;d&gt;</p>
+<p title="one\rtwo\r
+&lt;&quot;&amp;&quot;&gt;">a\rb\r
+c &amp; &lt;d&gt;</p>
+<p>&#13;<!--|-->&lt;!--|--&gt;</p>
+<p>\r&lt;!--|--&gt;</p>
````

**Patch `t1-code`:**

<!-- patch t1-code -->

````diff
diff --git a/crates/nova-codegen-wasm/src/shim.rs b/crates/nova-codegen-wasm/src/shim.rs
index 241594dbdb851e6d510ed5980e3e4dbc85af8356..34fdc40f66ae00ab4febec6ba2031c9313dbba0e 100644
--- a/crates/nova-codegen-wasm/src/shim.rs
+++ b/crates/nova-codegen-wasm/src/shim.rs
@@ -166,6 +166,7 @@ fn js_function(name: &str) -> Option<&'static str> {
         "dom_fetch_status" => "(t) => BigInt(object(t).status)",
         "dom_fetch_ok" => "(t) => (object(t).ok ? 1 : 0)",
         "dom_fetch_body" => "(t) => string(object(t).body)",
+        "dom_console_error" => "(s) => { console.error(text(s)); }",
         _ => return None,
     })
 }
diff --git a/std/dom/lib.nova b/std/dom/lib.nova
index 93b9c52fcd558e564cddb4c0550e14e3ceabaa4e..2575b0a08d77b24dca5f49c7cee78305e4ec652b 100644
--- a/std/dom/lib.nova
+++ b/std/dom/lib.nova
@@ -1,6 +1,6 @@
 //! The browser's page, from a program built for WASM: its elements and
 //! text, their attributes and properties, events, the location and history,
-//! timers, and `fetch`.
+//! timers, `fetch`, and the console.
 //!
 //! It is not in the prelude: a module that uses it imports it, with
 //! `import dom` for every name, or `import dom::{Element, create_element}`
@@ -73,9 +73,10 @@ extern "js" {
     fn dom_fetch_status(token: Int) -> Int
     fn dom_fetch_ok(token: Int) -> Bool
     fn dom_fetch_body(token: Int) -> String
+    fn dom_console_error(message: String)
 }
 
-/// A node of the page: an element, or a text node.
+/// A node of the page: an element, a text node, or a comment.
 ///
 /// Each value holds a handle to the JS object, which a collection releases
 /// once no value holds it. A value built by hand panics when used, unless its
@@ -259,6 +260,10 @@ pub fn history_replace(url: String) { dom_history_replace(url) }
 /// Goes back one history entry: a `popstate` event on the window follows.
 pub fn history_back() { dom_history_back() }
 
+/// Writes `message` to the browser's console as an error, as
+/// `console.error` does.
+pub fn console_error(message: String) { dom_console_error(message) }
+
 /// The page's `<body>`.
 pub fn body() -> Element { node(dom_body()) }
 
@@ -281,7 +286,7 @@ pub fn element_by_id(id: String) -> Option<Element> { maybe_node(dom_element_by_
 
 impl Element {
     /// The node's name in lower case: `"div"` for a `<div>`, `"#text"` for
-    /// a text node.
+    /// a text node, `"#comment"` for a comment.
     pub fn tag(self) -> String { dom_tag(handle_of(self)) }
 
     /// Sets the attribute `name` to `value`.
diff --git a/std/ui/backend/native.nova b/std/ui/backend/native.nova
index afe5f098c6a8841d8453c89450c00ccb128e544b..9a27450146bcc5161dd985d3314ae51e01599be7 100644
--- a/std/ui/backend/native.nova
+++ b/std/ui/backend/native.nova
@@ -43,6 +43,21 @@ pub fn backend_next_sibling(node: Node) -> Option<Node> { no_page() }
 /// Whether two nodes are one.
 pub fn backend_is_same(a: Node, b: Node) -> Bool { no_page() }
 
+/// The node's first child.
+pub fn backend_first_child(node: Node) -> Option<Node> { no_page() }
+
+/// The node's kind: its name in lower case.
+pub fn backend_kind(node: Node) -> String { no_page() }
+
+/// The node's text.
+pub fn backend_text_of(node: Node) -> String { no_page() }
+
+/// Writes `message` to the console as an error.
+pub fn backend_report(message: String) { no_page() }
+
+/// The server's data: the text of the `nova-data` script after `root`.
+pub fn backend_data(root: Node) -> Option<String> { no_page() }
+
 /// Calls `f` with the target's `value` on each event of kind `kind`, until
 /// the function returned is called.
 pub fn backend_listen(node: Node, kind: String, f: fn(String)) -> fn() { no_page() }
diff --git a/std/ui/backend/wasm.nova b/std/ui/backend/wasm.nova
index 081b2214e49174bb7dccef65f2fbf88d3dc33f70..bf94f62468708b8179cbb0f05ee8e0c8439d8985 100644
--- a/std/ui/backend/wasm.nova
+++ b/std/ui/backend/wasm.nova
@@ -53,6 +53,44 @@ pub fn backend_next_sibling(node: Node) -> Option<Node> { wrap(node.el.next_sibl
 /// Whether two nodes are one.
 pub fn backend_is_same(a: Node, b: Node) -> Bool { a.el.is_same(b.el) }
 
+/// The node's first child (spec
+/// docs/superpowers/specs/2026-10-19-phase-4-6-ssr-hydration-and-ssg-design.md
+/// §8.2).
+pub fn backend_first_child(node: Node) -> Option<Node> { wrap(node.el.first_child()) }
+
+/// The node's kind: its name in lower case, as `"div"`, `"#text"` or
+/// `"#comment"`.
+pub fn backend_kind(node: Node) -> String { node.el.tag() }
+
+/// The node's text: a text node's or a comment's own, or the text of
+/// everything in an element.
+pub fn backend_text_of(node: Node) -> String { node.el.text() }
+
+/// Writes `message` to the console as an error.
+pub fn backend_report(message: String) { console_error(message) }
+
+/// The server's data for the page whose root is `root`: the text of the
+/// root's next element, when it is a `<script type="application/json"
+/// id="nova-data">`, as `ui::ssr` writes it just after the root; or `None`
+/// (spec docs/superpowers/specs/2026-10-19-phase-4-6-ssr-hydration-and-ssg-design.md
+/// §8.2). An element with that id elsewhere, such as in the root's view, is
+/// not the data.
+pub fn backend_data(root: Node) -> Option<String> {
+    let mut at = root.el.next_sibling()
+    while at.is_some() {
+        let e = at.unwrap()
+        let kind = e.tag()
+        if !kind.starts_with("#") {
+            let named = e.attribute("id").unwrap_or("") == "nova-data"
+            let json = e.attribute("type").unwrap_or("") == "application/json"
+            if kind == "script" && named && json { return Some(e.text()) }
+            return None
+        }
+        at = e.next_sibling()
+    }
+    None
+}
+
 /// Calls `f` with the target's `value` on each event of kind `kind`, until
 /// the function returned is called.
 pub fn backend_listen(node: Node, kind: String, f: fn(String)) -> fn() {
diff --git a/std/ui/lib.nova b/std/ui/lib.nova
index 0b246b1342580947c1edbff64f2fc1785f7f694b..c2d7b5fb69a8f48d5cf38a7c6053d6abf66b6daf 100644
--- a/std/ui/lib.nova
+++ b/std/ui/lib.nova
@@ -5,7 +5,9 @@
 //! `import ui` for every name, or `import ui::{signal, effect}` for some.
 //! `import ui::html` brings the element builders, `div`, `button`, `text` and
 //! the rest, which make the `View`s that `mount` puts into the page and
-//! `render_to_string` writes as HTML.
+//! `render_to_string` writes as HTML. A server writes a view with
+//! `render_hydratable`, and its client, a WASM build of the same view,
+//! adopts that HTML with `hydrate`.
 //!
 //! `set` runs the effects that read a signal before it returns, each once,
 //! and only after every memo it reads is current. A memo computes when it is
@@ -473,8 +475,9 @@ pub fn on_cleanup(f: fn()) {
     }
 }
 
-/// Runs `f` once its nodes are in the page: after the `mount`, or the
-/// `show` or `for_each` render, that is building, or at once when none is.
+/// Runs `f` once its nodes are in the page: after the `mount` or the
+/// `hydrate`, or the `show` or `for_each` render, that is building, or at
+/// once when none is.
 /// It runs untracked, with the owner it was added in, which owns what it
 /// makes. It never runs under `render_to_string`, nor when its render is
 /// disposed, or the effect that added it runs again, before its nodes are in
@@ -647,8 +650,10 @@ fn check_element(tag: String, attrs: [Attr]) {
 // The elements written with no closing tag and no children.
 fn is_void(tag: String) -> Bool { tag == "input" || tag == "img" || tag == "br" || tag == "hr" }
 
-// `text` with `&`, `<` and `>` escaped, and `"` too in an attribute.
-fn escape(text: String, attribute: Bool) -> String {
+// `text` with `&`, `<` and `>` escaped, and `"` too in an attribute; with
+// `marks`, a carriage return too, which the browser's parser would turn
+// into a line feed.
+fn escape(text: String, attribute: Bool, marks: Bool) -> String {
     let cs = text.chars()
     let mut parts = Vec::new()
     let mut i = 0
@@ -662,6 +667,8 @@ fn escape(text: String, attribute: Bool) -> String {
             parts.push("&gt;")
         } else if attribute && c == '"' {
             parts.push("&quot;")
+        } else if marks && c == '\r' {
+            parts.push("&#13;")
         } else {
             parts.push("${c}")
         }
@@ -680,8 +687,20 @@ fn joined(parts: Vec<String>) -> String {
     "".join(all)
 }
 
-// Write `v` as HTML into `out`.
-fn write_view(v: View, out: Vec<String>) {
+// Whether `v` is text, fixed or dynamic.
+fn is_text(v: View) -> Bool {
+    match v {
+        Text(t) => true
+        DynText(f) => true
+        _ => false
+    }
+}
+
+// Write `v` as HTML into `out`. With `marks`, as `render_hydratable` writes
+// it (spec docs/superpowers/specs/2026-10-19-phase-4-6-ssr-hydration-and-ssg-design.md
+// §3): a region between `<!--[-->` and `<!--]-->`, and a carriage return as
+// `&#13;`.
+fn write_view(v: View, out: Vec<String>, marks: Bool) {
     let mut o = out
     match v {
         Element(tag, attrs, children) => {
@@ -689,46 +708,54 @@ fn write_view(v: View, out: Vec<String>) {
             o.push("<${tag}")
             let mut i = 0
             while i < attrs.len() {
-                write_attr(attrs[i], out)
+                write_attr(attrs[i], out, marks)
                 i = i + 1
             }
             o.push(">")
             if !is_void(tag) {
-                write_views(children, out)
+                write_views(children, out, marks)
                 o.push("</${tag}>")
             }
         }
-        Text(t) => o.push(escape(t, false))
-        DynText(f) => o.push(escape(f(), false))
-        Region(f) => write_views(f(), out)
+        Text(t) => o.push(escape(t, false, marks))
+        DynText(f) => o.push(escape(f(), false, marks))
+        Region(f) => {
+            if marks { o.push("<!--[-->") }
+            write_views(f(), out, marks)
+            if marks { o.push("<!--]-->") }
+        }
     }
 }
 
-fn write_views(views: [View], out: Vec<String>) {
+// Write `views`, siblings, into `out`; with `marks`, `<!--|-->` between two
+// texts side by side, so that the browser's parse keeps them apart.
+fn write_views(views: [View], out: Vec<String>, marks: Bool) {
+    let mut o = out
+    let mut after_text = false
     let mut i = 0
     while i < views.len() {
-        write_view(views[i], out)
+        let text = is_text(views[i])
+        if marks && after_text && text { o.push("<!--|-->") }
+        write_view(views[i], out, marks)
+        after_text = text
         i = i + 1
     }
 }
 
-fn write_attr(a: Attr, out: Vec<String>) {
+fn write_attr(a: Attr, out: Vec<String>, marks: Bool) {
     let mut o = out
     match a {
-        Static(name, value) => o.push(" ${name}=\"${escape(value, true)}\"")
-        Dynamic(name, f) => o.push(" ${name}=\"${escape(f(), true)}\"")
-        Property(name, value) => o.push(" ${name}=\"${escape(value, true)}\"")
-        DynProperty(name, f) => o.push(" ${name}=\"${escape(f(), true)}\"")
+        Static(name, value) => o.push(" ${name}=\"${escape(value, true, marks)}\"")
+        Dynamic(name, f) => o.push(" ${name}=\"${escape(f(), true, marks)}\"")
+        Property(name, value) => o.push(" ${name}=\"${escape(value, true, marks)}\"")
+        DynProperty(name, f) => o.push(" ${name}=\"${escape(f(), true, marks)}\"")
         On(kind, f) => {}
     }
 }
 
-/// `f`'s view as HTML. `f` runs in a new owner, which is disposed once the
-/// HTML is written; text and attributes are read once, handlers are left
-/// out, and `on_mount` never runs. Text and attribute values are escaped,
-/// the text of a `script` or `style` element too. It is a batch: what its
-/// `set`s run, such as a region `mount` built, runs once it is done.
-pub fn render_to_string(f: fn() -> View) -> String {
+// `f`'s view as HTML, written with `marks` or without, as the two renderers
+// write it.
+fn render(f: fn() -> View, marks: Bool) -> String {
     let out = Vec::new()
     batch(
         || {
@@ -736,7 +763,7 @@ pub fn render_to_string(f: fn() -> View) -> String {
             s.rendering = s.rendering + 1
             root(
                 |dispose: fn()| {
-                    write_view(f(), out)
+                    write_view(f(), out, marks)
                     dispose()
                 },
             )
@@ -746,6 +773,38 @@ pub fn render_to_string(f: fn() -> View) -> String {
     joined(out)
 }
 
+/// `f`'s view as HTML. `f` runs in a new owner, which is disposed once the
+/// HTML is written; text and attributes are read once, handlers are left
+/// out, and `on_mount` never runs. Text and attribute values are escaped,
+/// the text of a `script` or `style` element too. It is a batch: what its
+/// `set`s run, such as a region `mount` built, runs once it is done.
+pub fn render_to_string(f: fn() -> View) -> String { render(f, false) }
+
+/// `f`'s view as HTML for `hydrate` to adopt in the page: what
+/// `render_to_string` writes, with three additions, so that the browser's
+/// parse of it has each node `mount` would build. A `show` or a `for_each`
+/// is written between the comments `<!--[-->` and `<!--]-->`. Two texts side
+/// by side, among an element's children or a region's views, are separated
+/// by `<!--|-->`; an empty text still writes nothing, but keeps its
+/// separators. And a carriage return, in text or in an attribute's value, is
+/// written `&#13;`, since the browser's parser turns a raw one into a line
+/// feed. No marker can come from a program's text, which is escaped.
+///
+/// A server writes it into the page, and the client hydrates the same view:
+///
+/// ```
+/// import ui
+/// import ui::html::{div, dyn_text, li, text, ul}
+///
+/// let items = signal(["a", "b"])
+/// let list = || ul([], [for_each(items, |s: String| li([], [text(s)]))])
+/// let html = render_hydratable(|| div([], [text("count: "), dyn_text(|| "2"), list()]))
+/// assert_eq(html, "<div>count: <!--|-->2<ul><!--[--><li>a</li><li>b</li><!--]--></ul></div>")
+/// // A carriage return, which `render_to_string` writes as it is.
+/// assert_eq(render_hydratable(|| text("a\rb")), "a&#13;b")
+/// ```
+pub fn render_hydratable(f: fn() -> View) -> String { render(f, true) }
+
 /// Builds `f`'s view into the first element of the page `selector` matches,
 /// after its children, and keeps it up to date. It panics when no element
 /// matches, and natively, where there is no page.
@@ -765,6 +824,158 @@ pub fn mount(selector: String, f: fn() -> View) {
     run_mounted(start)
 }
 
+/// Adopts the HTML `render_hydratable` wrote for the same view into the
+/// first element of the page `selector` matches, and keeps it up to date:
+/// the server's nodes stay in the page, and the effects and handlers `mount`
+/// would give its own nodes start on them. `on_mount` callbacks run once
+/// that is done. The server writes `render_hydratable(App)` into the page's
+/// root, and the client, a WASM build of the same `App`, calls
+/// `hydrate("#app", App)`.
+///
+/// It walks `f()`'s view against the root's nodes in order: an element must
+/// have the view's tag, and a text its text; attributes are not compared. At
+/// the first difference, it writes `nova: hydration mismatch: expected …,
+/// found …; building the view again` with `console.error`, disposes of what
+/// it began, removes the root's children, and builds the view there as
+/// `mount` does, calling `f` again. A root with no child, or only
+/// whitespace, as a page served without the server's HTML has, gets the view
+/// built the same way, with no report: its whitespace is removed first. It
+/// panics when no element matches, and natively, where there is no page.
+///
+/// What a server writes for a view, which `hydrate("#app", app)` then adopts
+/// in the page, the button's handler added and the count's text followed:
+///
+/// ```
+/// import ui
+/// import ui::html::{button, div, dyn_text, on_click, text}
+///
+/// let count = signal(2)
+/// let add = on_click(|| count.update(|n: Int| n + 1))
+/// let app = || div([], [text("count: "), dyn_text(|| "${count.value()}"), button([add], [text("+")])])
+/// assert_eq(render_hydratable(app), "<div>count: <!--|-->2<button>+</button></div>")
+/// ```
+pub fn hydrate(selector: String, f: fn() -> View) { hydrate_into(root_of("hydrate", selector), f) }
+
+/// `hydrate` for a view of the server's data. The data is the JSON text of
+/// the `<script type="application/json" id="nova-data">` the server wrote
+/// just after the root: the root's next element, when it is a `script` with
+/// that `id` and that `type`. An element of the view with the id `nova-data`
+/// is never the data. `hydrate_with` makes a `T` of that text with
+/// `T::from_json`, and hydrates `f` of it, the view the server rendered with
+/// `render_hydratable`. When the page holds no data from the server, as
+/// under `nova dev`, it uses `fallback` instead, as `hydrate` would: an
+/// empty root gets `f(fallback)`'s view built, and a root the server filled
+/// is walked with it. It panics when the data's text does not parse, or
+/// `from_json` refuses it, and as `hydrate` does.
+///
+/// The server renders the view of a value, and writes the value beside it:
+///
+/// ```
+/// import ui
+/// import ui::html::{li, text, ul}
+///
+/// fn names(xs: [String]) -> View { ul([], [for_each(signal(xs), |s: String| li([], [text(s)]))]) }
+///
+/// fn main() {
+///     let data = ["Ada", "Grace"]
+///     let html = render_hydratable(|| names(data))
+///     assert_eq(html, "<ul><!--[--><li>Ada</li><li>Grace</li><!--]--></ul>")
+///     // The page holds the HTML, and after its root the data as JSON,
+///     // escaped, in its `nova-data`. The client calls
+///     // `hydrate_with("#app", none, names)`, where `none` is an empty
+///     // `[String]`: a page with no data, as `nova dev` serves, shows an
+///     // empty list.
+///     let none: [String] = []
+///     assert_eq(render_hydratable(|| names(none)), "<ul><!--[--><!--]--></ul>")
+/// }
+/// ```
+pub fn hydrate_with<T: FromJson>(selector: String, fallback: T, f: fn(T) -> View) {
+    let parent = root_of("hydrate_with", selector)
+    let data = match backend_data(parent) {
+        Some(text) => {
+            match parse(text) {
+                Ok(v) => {
+                    match T::from_json(v) {
+                        Ok(d) => d
+                        Err(e) => {
+                            panic("std/ui: hydrate_with: the page's data does not parse: ${e.msg}")
+                        }
+                    }
+                }
+                Err(e) => panic("std/ui: hydrate_with: the page's data does not parse: ${e.msg}")
+            }
+        }
+        None => fallback
+    }
+    hydrate_into(parent, || f(data))
+}
+
+// The first element `selector` matches, the root of `name`'s call, which
+// panics, naming the function, when none does.
+fn root_of(name: String, selector: String) -> Node {
+    match backend_query(selector) {
+        Some(p) => p
+        None => panic("std/ui: ${name}: no element matches \"${selector}\"")
+    }
+}
+
+// `hydrate` into `parent` (spec
+// docs/superpowers/specs/2026-10-19-phase-4-6-ssr-hydration-and-ssg-design.md
+// §4). The walk has a root owner, kept as `mount` keeps one; on a mismatch
+// it is disposed and dropped, the root emptied, and the view built afresh
+// under a new one. A blank root is emptied too, so the view is not built
+// after its whitespace. `on_mount` callbacks wait for the walk, or the
+// build, and those the disposed owner queued never run.
+fn hydrate_into(parent: Node, f: fn() -> View) {
+    let mut s = ui()
+    let start = s.mounted.len()
+    if !blank(parent) {
+        let o = new_owner(ROOT, || {})
+        let mut kept = s.roots
+        kept.push(o)
+        let w = Walk { failed: false, message: "" }
+        s.building = s.building + 1
+        with_context(
+            Some(o),
+            None,
+            || {
+                let c = Cursor { parent: parent, at: backend_first_child(parent) }
+                walk_view(f(), false, w, c)
+                left_over(w, c)
+            },
+        )
+        s.building = s.building - 1
+        if !w.failed {
+            run_mounted(start)
+            return
+        }
+        backend_report("nova: hydration mismatch: ${w.message}; building the view again")
+        batch(|| dispose(o))
+        let mut live = Vec::new()
+        let mut i = 0
+        while i < s.roots.len() {
+            let r = at(s.roots, i)
+            if r.id != o.id { live.push(r) }
+            i = i + 1
+        }
+        s.roots = live
+    }
+    let mut emptied = false
+    while !emptied {
+        match backend_first_child(parent) {
+            Some(n) => backend_remove(n)
+            None => emptied = true
+        }
+    }
+    let o = new_owner(ROOT, || {})
+    let mut kept = s.roots
+    kept.push(o)
+    s.building = s.building + 1
+    with_context(Some(o), None, || build(f(), parent, None))
+    s.building = s.building - 1
+    run_mounted(start)
+}
+
 // Put `node` into `parent`, before `before` or last.
 fn put(parent: Node, node: Node, before: Option<Node>) {
     match before {
@@ -859,3 +1070,253 @@ fn build_attr(a: Attr, el: Node) {
         }
     }
 }
+
+// Hydration's walk (spec
+// docs/superpowers/specs/2026-10-19-phase-4-6-ssr-hydration-and-ssg-design.md
+// §4.2, §4.3): a cursor steps through the server's nodes in the order
+// `build` would make them, and each view adopts the nodes it would have
+// built, with the effects and handlers `build` would give them. The first
+// difference is kept in the walk, which stops there.
+
+// A walk, and its first difference.
+record Walk { failed: Bool, message: String }
+
+// Where a walk is among `parent`'s children: at a node, or past the last.
+record Cursor { parent: Node, at: Option<Node> }
+
+// What the walk found for an effect: a dynamic text's node, and the text
+// its first run read; or a region's end marker.
+record Found { node: Option<Node>, text: String }
+
+// Whether `node` holds nothing to adopt: no child, or only text that is
+// all whitespace, as HTML reads it.
+fn blank(node: Node) -> Bool {
+    let mut child = backend_first_child(node)
+    let mut done = false
+    while !done {
+        match child {
+            Some(n) => {
+                if backend_kind(n) != "#text" { return false }
+                let cs = backend_text_of(n).chars()
+                let mut i = 0
+                while i < cs.len() {
+                    let c = cs[i]
+                    let space = c == ' ' || c == '\t' || c == '\n' || c == '\r'
+                    if !space && "${c}" != "\u{c}" { return false }
+                    i = i + 1
+                }
+                child = backend_next_sibling(n)
+            }
+            None => done = true
+        }
+    }
+    true
+}
+
+// Step `c` to the next node.
+fn advance(c: Cursor) {
+    let mut m = c
+    m.at = match c.at {
+        Some(n) => backend_next_sibling(n)
+        None => None
+    }
+}
+
+// Whether `node` is the comment `<!--${data}-->`.
+fn is_marker(node: Node, data: String) -> Bool {
+    backend_kind(node) == "#comment" && backend_text_of(node) == data
+}
+
+// At most the first 40 characters of `t`.
+fn clipped(t: String) -> String { if t.len() > 40 { t.slice(0, 40) } else { t } }
+
+// What the walk found at `here`, as a mismatch names it. A separator stands
+// for the text after it, which the server wrote there.
+fn found(here: Option<Node>) -> String {
+    match here {
+        Some(n) => {
+            let kind = backend_kind(n)
+            if kind == "#text" { return "text \"${clipped(backend_text_of(n))}\"" }
+            if kind != "#comment" { return "<${kind}>" }
+            let data = backend_text_of(n)
+            if data == "[" { return "a region" }
+            if data == "]" { return "the end of a region" }
+            if data != "|" { return "a comment" }
+            match backend_next_sibling(n) {
+                Some(next) => {
+                    if backend_kind(next) == "#text" {
+                        return "text \"${clipped(backend_text_of(next))}\""
+                    }
+                }
+                None => {}
+            }
+            "text \"\""
+        }
+        None => "nothing"
+    }
+}
+
+// Stop the walk at `c`, which is not `expected`.
+fn missed(w: Walk, expected: String, c: Cursor) {
+    let mut m = w
+    m.failed = true
+    m.message = "expected ${expected}, found ${found(c.at)}"
+}
+
+// After the views of an element or of the root, no node may be left.
+fn left_over(w: Walk, c: Cursor) { if !w.failed && c.at.is_some() { missed(w, "nothing", c) } }
+
+fn walk_views(views: [View], w: Walk, c: Cursor) {
+    let mut after_text = false
+    let mut i = 0
+    while i < views.len() && !w.failed {
+        let text = is_text(views[i])
+        walk_view(views[i], after_text && text, w, c)
+        after_text = text
+        i = i + 1
+    }
+}
+
+// Walk `v` at `c`, adopting its nodes. `joined` says a text came just before
+// it and it is text too, so the separator `render_hydratable` wrote between
+// them comes first.
+fn walk_view(v: View, joined: Bool, w: Walk, c: Cursor) {
+    match v {
+        Element(tag, attrs, children) => {
+            check_element(tag, attrs)
+            let name = tag.to_lower()
+            match c.at {
+                Some(el) => {
+                    if backend_kind(el) == name {
+                        advance(c)
+                        let mut i = 0
+                        while i < attrs.len() {
+                            adopt_attr(attrs[i], el)
+                            i = i + 1
+                        }
+                        if !is_void(tag) {
+                            let inner = Cursor { parent: el, at: backend_first_child(el) }
+                            walk_views(children, w, inner)
+                            left_over(w, inner)
+                        }
+                        return
+                    }
+                }
+                None => {}
+            }
+            missed(w, "<${name}>", c)
+        }
+        Text(t) => { let _ = adopt_text(t, joined, w, c) }
+        DynText(f) => {
+            let got = Found { node: None, text: "" }
+            effect(
+                || {
+                    let t = f()
+                    let mut m = got
+                    match got.node {
+                        Some(n) => backend_set_text(n, t)
+                        None => m.text = t
+                    }
+                },
+            )
+            let mut m = got
+            m.node = adopt_text(got.text, joined, w, c)
+        }
+        Region(f) => {
+            match c.at {
+                Some(start) => {
+                    if is_marker(start, "[") {
+                        advance(c)
+                        let end = Found { node: None, text: "" }
+                        effect(
+                            || {
+                                match end.node {
+                                    Some(e) => build_region(f, c.parent, start, e)
+                                    None => adopt_region(f, w, c, end)
+                                }
+                            },
+                        )
+                        return
+                    }
+                }
+                None => {}
+            }
+            missed(w, "a region", c)
+        }
+    }
+}
+
+// An attribute of an adopted element: the HTML carried a fixed one, and a
+// property as an attribute, so that is set; dynamic ones get their effects,
+// whose first run writes them again, and handlers are added, as `build`
+// does.
+fn adopt_attr(a: Attr, el: Node) {
+    match a {
+        Static(name, value) => {}
+        _ => build_attr(a, el)
+    }
+}
+
+// The node of a text view whose text is `t`: past the separator `joined`
+// asks for, the text node at `c`, which must hold `t`; or, where there is
+// none and `t` is empty, a new empty one put there, since an empty text
+// writes nothing. `None` once the walk stops.
+fn adopt_text(t: String, joined: Bool, w: Walk, c: Cursor) -> Option<Node> {
+    let expected = "text \"${clipped(t)}\""
+    if joined {
+        let separated = match c.at {
+            Some(n) => is_marker(n, "|")
+            None => false
+        }
+        if !separated {
+            missed(w, expected, c)
+            return None
+        }
+        advance(c)
+    }
+    match c.at {
+        Some(n) => {
+            if backend_kind(n) == "#text" {
+                if backend_text_of(n) != t {
+                    missed(w, expected, c)
+                    return None
+                }
+                advance(c)
+                return Some(n)
+            }
+        }
+        None => {}
+    }
+    if t != "" {
+        missed(w, expected, c)
+        return None
+    }
+    let n = backend_text("")
+    put(c.parent, n, c.at)
+    Some(n)
+}
+
+// A region's first run under hydration, as `build_region`'s with a walk for
+// a build: its views, read as the effect running this, walked under a
+// render of their own from the node after its `<!--[-->`; then its
+// `<!--]-->`, which `end` keeps for the later runs, which build before it as
+// under `mount`.
+fn adopt_region(f: fn() -> [View], w: Walk, c: Cursor, end: Found) {
+    let render = new_owner(ROOT, || {})
+    adopt(render)
+    let views = with_context(Some(render), ui().listener, f)
+    with_context(Some(render), None, || walk_views(views, w, c))
+    if w.failed { return }
+    match c.at {
+        Some(n) => {
+            if is_marker(n, "]") {
+                advance(c)
+                let mut m = end
+                m.node = Some(n)
+                return
+            }
+        }
+        None => {}
+    }
+    missed(w, "the end of a region", c)
+}
````


### Task 2: The router at a location, and `std/http`'s `fallback` and `listen_on`

Spec §5, §8.3; decisions 6, 9 and 36.

**Files:**
- Modify: `std/ui/router/lib.nova`, `std/http/lib.nova`
- Test:
  - `tests/runtime/ui_router_server.nova`, `ui_router_server.stdout`
  - `tests/runtime/http_listen_on.nova`, `http_listen_on.stdout`
  - `tests/runtime/http_server_dispatch.nova`, `http_server_dispatch.stdout`
  - `crates/nova-cli/tests/run_tests.rs`, `wasm_tests.rs`
  - `crates/nova-driver/tests/index_complete.rs`

- [ ] **Step 1: The tests.** `patch_block t2-tests`. The patch holds:
  - `tests/runtime/ui_router_server.nova`: `with_location` rendering a
    `HistoryRouter` at `/posts/1`, at `/q?a=1`, at a path only the catch-all
    serves, and at `/posts/3#top`, whose fragment it drops, and a hash
    `Router` at `/posts/2`; two calls nested, then the location restored, so a
    router made after them renders the home route; a value returned through
    it; `found` for static routes, a route with a parameter, percent-encoded
    or with a query, `/q#top` and `/q?a=1#top`, whose fragment it drops, paths
    of the wrong length, the catch-all, no routes, no match, `/*`, and a route
    after a catch-all; and `static_paths` with repeats, a parameter, the
    catch-all and no routes. It joins `ui_programs_run_and_survive_gc_stress`,
    natively, and the gate as `gate::ui_router_server`, on WASM under the fake
    DOM at `http://localhost/`, each as it is and under stress, with one
    `.stdout`. `gate::ui_router` and `gate::ui_router_server` share
    `router_program(name)`.
  - `tests/runtime/http_listen_on.nova`: the program binds `127.0.0.1:0`,
    reads the port, and serves that listener with `listen_on`; a client
    then gets, over TCP on one connection, a route's answer, the fallback's
    for a `GET` no route matches, its query kept, and `not_found` for a
    `POST`, and the program ends with `exit`. `run_tests.rs`'s
    `http_listen_on_run` runs it as it is and under stress;
    `wasm_tests.rs`'s `LEFT_OUT` names it, `std/http, std/net, std/process`.
  - `tests/runtime/http_server_dispatch.nova`: `fallback` answers a `GET`
    no route matches, query included, and nothing else; a route still
    answers its own path, added before or after it; a second `fallback`
    replaces the first; and the server it was called on keeps none.
  - `index_complete.rs`: 156 programs and 181 files.
- [ ] **Step 2: Run, and see them fail.**
  - `R ui_programs_run_and_survive_gc_stress`. Expected: `0 passed; 1
    failed`: "Unexpected failure." of `nova run
    tests/runtime/ui_router_server.nova`: ``error[E0001]: `found` is not a
    public item of module `ui::router` ``, then the same for
    `static_paths` and `with_location`: `could not compile due to 3
    previous errors`.
  - `R http_listen_on_run`. Expected: `0 passed; 1 failed`: "Unexpected
    failure.": ``error[E0014]: no method `fallback` on type `Server` ``,
    then three `error[E0011]: cannot infer the type here; add a type
    annotation`: `could not compile due to 4 previous errors`.
  - `R http_server_dispatch_run`. Expected: `0 passed; 1 failed`: the same
    E0014, then four E0011: `could not compile due to 5 previous errors`.
  - `W gate::ui_router_server`. Expected: `0 passed; 1 failed`:
    "Unexpected failure." of `nova build --target wasm`: the three E0001 of
    the first command.
  - `W every_runtime_program_is_in_the_gate_or_left_out`. Expected: `1
    passed`.
  - `X`. Expected: `0 passed; 1 failed`:
    `every_name_in_the_corpora_is_indexed` (`index_complete.rs:258`):
    "…/tests/runtime/http_listen_on.nova does not check cleanly: [\"E0014
    no method `fallback` on type `Server`\", \"E0011 cannot infer the type
    here; add a type annotation\", …]".
- [ ] **Step 3: The code.** `patch_block t2-code`:
  - **`std/ui/router/lib.nova`:** `Routing.location`; `routed` starts a
    router at `starting(history)`, the location `with_location` gives, or
    else the page's, either less any `#fragment`; `with_location`, `found`,
    which drops a `#fragment` too (decision 36), and `static_paths`, each
    with its doc test; `first_match`, which
    `matching` now calls, and `catch_all`, which `params_of` now calls
    (decision 6). The module's doc names the three.
  - **`std/http/lib.nova`:** `Server.unmatched`; `fallback`, with its doc
    test; `get` keeps the fallback; `dispatch` calls it for a `GET` no route
    matches; `listen_on`, and `listen` as `bind`, then `listen_on`
    (decision 9).
- [ ] **Step 4: Run.**
  - The same six commands. Expected: `R ui_programs_run_and_survive_gc_stress`
    `1 passed`; `R http_listen_on_run` `1 passed`; `R
    http_server_dispatch_run` `1 passed`; `W gate::ui_router_server` `1
    passed`; `W every_runtime_program_is_in_the_gate_or_left_out` `1
    passed`; `X` `1 passed`.
  - The whole binaries:
    - `cargo test -p nova-std`. Expected: `2 passed`.
    - `cargo test -p nova-doc --lib`. Expected: `61 passed`.
    - `cargo test -p nova-cli --test doc_test`. Expected: `17 passed`.
    - `R`. Expected: `313 passed; 0 failed; 1 ignored`.
    - `W`. Expected: `228 passed`.
- [ ] **Step 5: The checks,** as the conventions list them, and `cargo run
  --locked -q -p nova-cli -- fmt --check tests/runtime/ui_router_server.nova
  tests/runtime/http_listen_on.nova`, which prints nothing. Not
  `http_server_dispatch.nova`: `nova fmt` would reformat three lines the
  base already had, and the lines this task adds are in its layout.
- [ ] **Step 6: Commit** `std/ui/router: with_location, found and static_paths; std/http: Server::fallback and listen_on`.

**Patch `t2-tests`:**

<!-- patch t2-tests -->

````diff
diff --git a/crates/nova-cli/tests/run_tests.rs b/crates/nova-cli/tests/run_tests.rs
index bc0dfbd36ac61e5283b387a3d1b26b1fd2dd1057..fd5ea8f0e47e3321cf62181c19327ff5ed7bf26b 100644
--- a/crates/nova-cli/tests/run_tests.rs
+++ b/crates/nova-cli/tests/run_tests.rs
@@ -9540,6 +9540,9 @@ fn http_keepalive_run() {
 /// GET routes, a handler that reads its request, 404 for an unknown path and
 /// for a non-GET method, and the JSON response's headers and bytes. Design:
 /// docs/superpowers/specs/2026-10-04-examples-03-http-server-design.md §7.2.
+/// Then `Server::fallback` (spec
+/// `docs/superpowers/specs/2026-10-19-phase-4-6-ssr-hydration-and-ssg-design.md`
+/// §8.3): a GET no route matches, and only that.
 #[test]
 fn http_server_dispatch_run() {
     let expected =
@@ -9554,6 +9557,26 @@ fn http_server_dispatch_run() {
         .stdout(expected);
 }
 
+/// `std/http`'s `Server::listen_on` (spec 4.6 §8.3): a listener the program
+/// bound on port 0 itself, served over TCP, with a route, the fallback and a
+/// `POST` answered on one connection, as it is and under GC stress.
+#[test]
+fn http_listen_on_run() {
+    let source = repo_root().join("tests/runtime/http_listen_on.nova");
+    let expected = std::fs::read_to_string(source.with_extension("stdout"))
+        .expect("expected-output fixture exists")
+        .replace("\r\n", "\n");
+    for stress in [false, true] {
+        let mut cmd = nova();
+        if stress {
+            cmd.env("NOVA_GC_STRESS", "1");
+        }
+        let a = cmd.arg("run").arg(&source).assert().success();
+        let out = String::from_utf8_lossy(&a.get_output().stdout).replace("\r\n", "\n");
+        assert_eq!(out, expected, "stress {stress}");
+    }
+}
+
 /// `std/process`'s `args()` under `nova run FILE -- ARGS`: index 0 is FILE as
 /// written, and an argument with spaces and an empty one each arrive whole.
 /// Run from the repository root so FILE is a fixed relative path. Design:
@@ -11308,7 +11331,8 @@ fn example_05_geometry_runs() {
 /// `std/ui`'s programs (spec
 /// `docs/superpowers/specs/2026-10-17-phase-4-3-std-ui-reactivity-and-views-design.md`
 /// §9.2), `std/ui/router`'s paths and queries (spec 4.5 §3.2 to §3.5), and
-/// `render_hydratable`'s markers (spec 4.6 §9.2): each prints what
+/// `render_hydratable`'s markers and the router's functions for a server
+/// (spec 4.6 §9.2): each prints what
 /// `30-FRONTEND.md` §13's unit tests check, and runs the same with
 /// `NOVA_GC_STRESS=1` (collect on every allocation), so a part of the graph
 /// only the state's root keeps, freed, shows as wrong output or a crash.
@@ -11323,6 +11347,7 @@ fn ui_programs_run_and_survive_gc_stress() {
         "ui_view",
         "ui_router",
         "ui_hydratable",
+        "ui_router_server",
     ] {
         let source = repo_root().join(format!("tests/runtime/{name}.nova"));
         let expected = std::fs::read_to_string(source.with_extension("stdout"))
diff --git a/crates/nova-cli/tests/wasm_tests.rs b/crates/nova-cli/tests/wasm_tests.rs
index f7974504826165f582f8030580b971d7589905b2..78644941e60a55a77d10fce17d3b686d8f0b0b45 100644
--- a/crates/nova-cli/tests/wasm_tests.rs
+++ b/crates/nova-cli/tests/wasm_tests.rs
@@ -1928,6 +1928,7 @@ mod gate {
         "ui_render_bad_name",
         "ui_router",
         "ui_router_no_to",
+        "ui_router_server",
     ];
 
     /// `eprint` and `eprintln` write to stderr, as `run_tests.rs` checks
@@ -2096,13 +2097,26 @@ mod gate {
     /// stress.
     #[test]
     fn ui_router() {
+        router_program("ui_router");
+    }
+
+    /// The router's functions for a server (spec 4.6 §5, §9.2): on WASM too,
+    /// under the fake DOM, as `ui_router` runs.
+    #[test]
+    fn ui_router_server() {
+        router_program("ui_router_server");
+    }
+
+    /// `tests/runtime/<name>.nova` under the fake DOM, as it is and under
+    /// stress, printing what it prints natively.
+    fn router_program(name: &str) {
         let Some(_) = super::node() else { return };
         let tests = super::repo_root().join("tests");
-        let source = tests.join("runtime/ui_router.nova");
+        let source = tests.join(format!("runtime/{name}.nova"));
         let expected = std::fs::read_to_string(source.with_extension("stdout"))
             .expect("the expected output")
             .replace("\r\n", "\n");
-        let dir = super::fresh("gate-ui_router");
+        let dir = super::fresh(&format!("gate-{name}"));
         std::fs::copy(tests.join("dom/fake_dom.mjs"), dir.join("fake_dom.mjs"))
             .expect("copy the fake DOM");
         std::fs::write(
@@ -2746,6 +2760,7 @@ const LEFT_OUT: &[(&str, &str)] = &[
     ("http_offsets", "std/http"),
     ("http_partial", "std/http"),
     ("http_serialise", "std/http"),
+    ("http_listen_on", "std/http, std/net, std/process"),
     ("http_server_dispatch", "std/http"),
     ("io_broken_stderr", "std/io"),
     ("io_broken_stdout", "std/io"),
diff --git a/crates/nova-driver/tests/index_complete.rs b/crates/nova-driver/tests/index_complete.rs
index 2de061f68ac2a9e797d50a91658e2d3e70ed5502..99a59595ad33f97e1fa0b1226f5f27fa00b6a7ee 100644
--- a/crates/nova-driver/tests/index_complete.rs
+++ b/crates/nova-driver/tests/index_complete.rs
@@ -241,9 +241,10 @@ fn every_name_is_covered(a: &Analysis, file: FileId, failures: &mut Vec<String>)
 #[test]
 fn every_name_in_the_corpora_is_indexed() {
     let programs = programs();
-    // 8 examples, 145 top-level runtime programs and the modules program,
-    // on 2026-10-19, after 4.6 added `ui_hydratable`.
-    assert_eq!(programs.len(), 154, "{programs:#?}");
+    // 8 examples, 147 top-level runtime programs and the modules program,
+    // on 2026-10-19, after 4.6 added `ui_hydratable`, `ui_router_server` and
+    // `http_listen_on`.
+    assert_eq!(programs.len(), 156, "{programs:#?}");
     let mut files = 0;
     let mut failures: Vec<String> = Vec::new();
     for (k, path) in programs.iter().enumerate() {
@@ -270,9 +271,9 @@ fn every_name_in_the_corpora_is_indexed() {
             every_name_is_covered(&a, file, &mut failures);
         }
     }
-    // 8 + 145 + 3 files of user code, 05-geometry's library's 2, and
+    // 8 + 147 + 3 files of user code, 05-geometry's library's 2, and
     // native std's 21.
-    assert_eq!(files, 179);
+    assert_eq!(files, 181);
     failures.sort();
     failures.dedup();
     assert!(
diff --git a/tests/runtime/http_listen_on.nova b/tests/runtime/http_listen_on.nova
new file mode 100644
index 0000000000000000000000000000000000000000..cb3e4ba86bfdd488d32792f656c369daf0fc56a1
--- /dev/null
+++ b/tests/runtime/http_listen_on.nova
@@ -0,0 +1,85 @@
+// `std/http`'s `Server::listen_on` (spec
+// docs/superpowers/specs/2026-10-19-phase-4-6-ssr-hydration-and-ssg-design.md
+// §8.3): the program binds port 0 itself, reads the port the system chose,
+// and serves that listener; a client then gets, over TCP and on one
+// connection, a route's answer, the fallback's for a GET no route matches,
+// and `not_found` for a POST. The status line and the body are printed, not
+// the headers, whose order a `Map` decides. `listen_on` serves until a
+// signal, so the program ends with `exit`.
+
+// One request on `s`, and its answer's status line and body.
+async fn ask(s: TcpStream, request: String) -> String {
+    let _ = match s.write(bytes_from_string(request)).await {
+        Ok(n) => n
+        Err(e) => panic("client write: ${e.message}")
+    }
+    let mut text = ""
+    let mut done = false
+    while !done {
+        let chunk = match s.read(4096).await {
+            Ok(b) => b
+            Err(e) => panic("client read: ${e.message}")
+        }
+        if chunk.len() == 0 { panic("the server closed the connection") }
+        text = "${text}${chunk.to_string().unwrap_or("<not utf-8>")}"
+        done = framed(text)
+    }
+    let status = text.slice(0, text.index_of("\r\n").unwrap_or(0))
+    let body = text.slice(text.index_of("\r\n\r\n").unwrap_or(0) + 4, text.len())
+    "${status} | ${body}"
+}
+
+// Whether `text` holds a whole response: its head, and as many bytes of body
+// as its `content-length` says.
+fn framed(text: String) -> Bool {
+    let end = match text.index_of("\r\n\r\n") {
+        Some(at) => at
+        None => return false
+    }
+    let lines = text.slice(0, end).split("\r\n")
+    let mut want = 0
+    let mut i = 0
+    while i < lines.len() {
+        let line = lines[i].to_lower()
+        if line.starts_with("content-length: ") { want = number(line.slice(16, line.len())) }
+        i = i + 1
+    }
+    text.len() - end - 4 >= want
+}
+
+// The decimal number `digits` writes.
+fn number(digits: String) -> Int {
+    let cs = digits.chars()
+    let mut n = 0
+    let mut i = 0
+    while i < cs.len() {
+        n = n * 10 + "0123456789".index_of("${cs[i]}").unwrap_or(0)
+        i = i + 1
+    }
+    n
+}
+
+async fn main() {
+    let l = match bind("127.0.0.1:0") {
+        Ok(l) => l
+        Err(e) => panic("bind: ${e.message}")
+    }
+    let port = match l.local_port() {
+        Ok(p) => p
+        Err(e) => panic("local_port: ${e.message}")
+    }
+    let app = Server::new()
+        .get("/", |_| Response::text(200, "home"))
+        .fallback(|r: Request| Response::text(404, "no page at ${r.path}"))
+    let _ = spawn(app.listen_on(l))
+
+    let s = match connect("127.0.0.1:${port}").await {
+        Ok(s) => s
+        Err(e) => panic("client connect: ${e.message}")
+    }
+    println(ask(s, "GET / HTTP/1.1\r\nHost: h\r\n\r\n").await)
+    println(ask(s, "GET /nope?x=1 HTTP/1.1\r\nHost: h\r\n\r\n").await)
+    println(ask(s, "POST /nope HTTP/1.1\r\nHost: h\r\nContent-Length: 2\r\n\r\nhi").await)
+    let _ = s.close().await
+    exit(0)
+}
diff --git a/tests/runtime/http_listen_on.stdout b/tests/runtime/http_listen_on.stdout
new file mode 100644
index 0000000000000000000000000000000000000000..b472126e00bbed0476ff992cd67a68a0caf23911
--- /dev/null
+++ b/tests/runtime/http_listen_on.stdout
@@ -0,0 +1,3 @@
+HTTP/1.1 200 OK | home
+HTTP/1.1 404 Not Found | no page at /nope?x=1
+HTTP/1.1 404 Not Found | not found
diff --git a/tests/runtime/http_server_dispatch.nova b/tests/runtime/http_server_dispatch.nova
index 571728f7603e2d6cf62706a73285b7e7f4f43dd6..0d2d9d4bbd6821116daf972cc6c256d6092c61a6 100644
--- a/tests/runtime/http_server_dispatch.nova
+++ b/tests/runtime/http_server_dispatch.nova
@@ -1,6 +1,10 @@
 // `std/http`'s `Server::dispatch` and `Response::json`, with no socket.
 // `examples/03-http-server` drives the same routes over TCP; this pins the
-// routing and the JSON response's bytes on their own.
+// routing and the JSON response's bytes on their own. Then `fallback` (spec
+// docs/superpowers/specs/2026-10-19-phase-4-6-ssr-hydration-and-ssg-design.md
+// §8.3): it answers a GET no route matches, its query included, and nothing
+// else; a route still answers its own path, before or after it; a second one
+// replaces the first; and the server it is called on keeps none.
 
 fn status_ok() -> JsonValue {
     let mut m: Map<String, JsonValue> = Map::new()
@@ -55,4 +59,27 @@ fn main() {
     println("s1: /a ${status_of(s1, "/a")} /b ${status_of(s1, "/b")} /c ${status_of(s1, "/c")}")
     println("s2: /a ${status_of(s2, "/a")} /b ${status_of(s2, "/b")} /c ${status_of(s2, "/c")}")
     println("base: /a ${status_of(base, "/a")} /b ${status_of(base, "/b")} /c ${status_of(base, "/c")}")
+
+    let fb = app.fallback(|r: Request| Response::text(404, "fallback for ${r.path}"))
+    let f1 = fb.dispatch(request(Get, "/nope?x=1"))
+    println("fallback: GET /nope?x=1 -> ${f1.status} ${body_text(f1)}")
+    let f2 = fb.dispatch(request(Get, "/"))
+    println("fallback: GET / -> ${f2.status} ${body_text(f2)}")
+    let f3 = fb.dispatch(request(Post, "/nope"))
+    println("fallback: POST /nope -> ${f3.status} ${body_text(f3)}")
+    let f4 = fb.dispatch(request(Head, "/nope"))
+    println("fallback: HEAD /nope -> ${f4.status} ${body_text(f4)}")
+    let later = fb.get("/later", |_| Response::text(200, "later"))
+    let l1 = later.dispatch(request(Get, "/later"))
+    let l2 = later.dispatch(request(Get, "/other"))
+    println(
+        "route after fallback: /later -> ${l1.status} ${body_text(l1)}, /other -> ${body_text(l2)}",
+    )
+    let twice = fb.fallback(|_| Response::text(410, "gone"))
+    let t1 = twice.dispatch(request(Get, "/nope"))
+    println("second fallback: GET /nope -> ${t1.status} ${body_text(t1)}")
+    let a1 = app.dispatch(request(Get, "/nope"))
+    println("app after fallback: GET /nope -> ${a1.status} ${body_text(a1)}")
+    let f5 = fb.dispatch(request(Get, "/nope"))
+    println("fallback after another: GET /nope -> ${body_text(f5)}")
 }
diff --git a/tests/runtime/http_server_dispatch.stdout b/tests/runtime/http_server_dispatch.stdout
index 2d673efa3b58263453277050de46612c24ee5e76..0678fbbc667ba30b2affb80dee90506a1dd61892 100644
--- a/tests/runtime/http_server_dispatch.stdout
+++ b/tests/runtime/http_server_dispatch.stdout
@@ -6,3 +6,11 @@ POST / -> 404
 s1: /a 200 /b 200 /c 404
 s2: /a 200 /b 404 /c 200
 base: /a 200 /b 404 /c 404
+fallback: GET /nope?x=1 -> 404 fallback for /nope?x=1
+fallback: GET / -> 200 Hello from Nova!
+fallback: POST /nope -> 404 not found
+fallback: HEAD /nope -> 404 not found
+route after fallback: /later -> 200 later, /other -> fallback for /other
+second fallback: GET /nope -> 410 gone
+app after fallback: GET /nope -> 404 not found
+fallback after another: GET /nope -> fallback for /nope
diff --git a/tests/runtime/ui_router_server.nova b/tests/runtime/ui_router_server.nova
new file mode 100644
index 0000000000000000000000000000000000000000..f8addca346b6982f256c0d4eb64df549285a6e2c
--- /dev/null
+++ b/tests/runtime/ui_router_server.nova
@@ -0,0 +1,126 @@
+// `std/ui/router`'s functions for a server (spec
+// docs/superpowers/specs/2026-10-19-phase-4-6-ssr-hydration-and-ssg-design.md
+// §5). `with_location` renders a router at `/posts/1` and at `/q?a=1`,
+// under either router, with a link's `href` its router's; it drops a
+// `#fragment`, nests, returns its function's value, and restores the
+// location after, which is then `/` natively and under the fake DOM. Then
+// `found` for static routes, a route with a parameter, the catch-all as `*`
+// and as `/*`, and no match, a `#fragment` left out as `with_location`
+// leaves it; and `static_paths` with repeats, parameters and the catch-all.
+import ui
+import ui::html::{div, p, text}
+import ui::router::{
+    HistoryRouter,
+    Route,
+    Router,
+    found,
+    link,
+    static_paths,
+    use_params,
+    use_query,
+    with_location,
+}
+
+component Home() {
+    view {
+        p { text("home") }
+    }
+}
+
+component Post() {
+    let id = use_params().get("id").unwrap_or("?")
+    view {
+        div {
+            p { text("post ${id}") }
+            link(to: "/posts/${id}/edit") { text("edit") }
+        }
+    }
+}
+
+component Query() {
+    let a = use_query().get("a").unwrap_or("?")
+    view {
+        p { text("query a=${a}") }
+    }
+}
+
+component Missing() {
+    view {
+        p { text("missing") }
+    }
+}
+
+fn routes() -> [Route] {
+    [
+        Route { path: "/", component: Home },
+        Route { path: "/posts/:id", component: Post },
+        Route { path: "/q", component: Query },
+        Route { path: "*", component: Missing },
+    ]
+}
+
+// The history router's page at `location`.
+fn at(location: String) -> String {
+    with_location(location, || render_to_string(|| HistoryRouter(routes())))
+}
+
+// The history router's page where nothing sets the location.
+fn here() -> String { render_to_string(|| HistoryRouter(routes())) }
+
+fn main() {
+    println("/posts/1: ${at("/posts/1")}")
+    println("/q?a=1: ${at("/q?a=1")}")
+    println("/nowhere: ${at("/nowhere")}")
+    println("/posts/3#top: ${at("/posts/3#top")}")
+    let hashed = with_location("/posts/2", || render_to_string(|| Router(routes())))
+    println("hash router at /posts/2: ${hashed}")
+    let nested = with_location(
+        "/posts/1",
+        || {
+            let inner = at("/q?a=2")
+            "${inner}, then ${here()}"
+        },
+    )
+    println("nested: ${nested}")
+    println("restored: ${here()}")
+    println("a value: ${with_location("/x", || 6 * 7)}")
+
+    let rs = routes()
+    let locations = [
+        "/",
+        "/posts/1",
+        "/posts/a%20b?x=2",
+        "/q?a=1",
+        "/q#top",
+        "/q?a=1#top",
+        "/posts",
+        "/posts/1/2",
+        "/nowhere",
+    ]
+    let mut i = 0
+    while i < locations.len() {
+        println("found ${locations[i]}: ${found(rs, locations[i])}")
+        i = i + 1
+    }
+    let none: [Route] = []
+    println("found with no routes: ${found(none, "/")}")
+    println("found with no match: ${found([Route { path: "/a", component: Home }], "/b")}")
+    let rooted_star = [Route { path: "/*", component: Missing }]
+    println("found under /*: ${found(rooted_star, "/a")}")
+    let after = [Route { path: "*", component: Missing }, Route { path: "/a", component: Home }]
+    println("found after a catch-all: ${found(after, "/a")}")
+
+    let many = [
+        Route { path: "/", component: Home },
+        Route { path: "/about", component: Home },
+        Route { path: "/posts/:id", component: Post },
+        Route { path: "/about", component: Missing },
+        Route { path: "/docs/intro", component: Home },
+        Route { path: "/:lang/docs", component: Home },
+        Route { path: "/*", component: Missing },
+        Route { path: "/", component: Missing },
+        Route { path: "*", component: Missing },
+    ]
+    println("static paths: ${", ".join(static_paths(many))}")
+    println("static paths of none: [${", ".join(static_paths(none))}]")
+}
diff --git a/tests/runtime/ui_router_server.stdout b/tests/runtime/ui_router_server.stdout
new file mode 100644
index 0000000000000000000000000000000000000000..62747ad921d12a73e4a6862803f7825cddf90471
--- /dev/null
+++ b/tests/runtime/ui_router_server.stdout
@@ -0,0 +1,23 @@
+/posts/1: <div><p>post 1</p><a href="/posts/1/edit" data-nova-link="">edit</a></div>
+/q?a=1: <p>query a=1</p>
+/nowhere: <p>missing</p>
+/posts/3#top: <div><p>post 3</p><a href="/posts/3/edit" data-nova-link="">edit</a></div>
+hash router at /posts/2: <div><p>post 2</p><a href="#/posts/2/edit" data-nova-link="">edit</a></div>
+nested: <p>query a=2</p>, then <div><p>post 1</p><a href="/posts/1/edit" data-nova-link="">edit</a></div>
+restored: <p>home</p>
+a value: 42
+found /: true
+found /posts/1: true
+found /posts/a%20b?x=2: true
+found /q?a=1: true
+found /q#top: true
+found /q?a=1#top: true
+found /posts: false
+found /posts/1/2: false
+found /nowhere: false
+found with no routes: false
+found with no match: false
+found under /*: false
+found after a catch-all: false
+static paths: /, /about, /docs/intro
+static paths of none: []
````

**Patch `t2-code`:**

<!-- patch t2-code -->

````diff
diff --git a/std/http/lib.nova b/std/http/lib.nova
index 0ce5870aef237d82472f198cefc9260a98fbc799..a0df53c90816f863e361a68d809c56eed6f99e3e 100644
--- a/std/http/lib.nova
+++ b/std/http/lib.nova
@@ -2,10 +2,11 @@
 //! and a router that serves GET routes.
 //!
 //! `Server` is the simple way in: give it a handler for each path with `get`,
-//! then `listen` on an address. A handler is a plain function from a
-//! `Request` to a `Response`, so it cannot await anything. `read_request`,
-//! `write_response` and `parse_request_head` are the parts underneath, for
-//! serving a `std/net` `TcpStream` yourself. There is no HTTP client. Every
+//! and one for every other `GET` with `fallback`, then `listen` on an
+//! address, or `listen_on` a listener already bound. A handler is a plain
+//! function from a `Request` to a `Response`, so it cannot await anything.
+//! `read_request`, `write_response` and `parse_request_head` are the parts
+//! underneath, for serving a `std/net` `TcpStream` yourself. There is no HTTP client. Every
 //! module sees these names without an import, and a module's own definition
 //! of one of them shadows the one here.
 //!
@@ -1000,15 +1001,17 @@ const SERVER_TICK_MS: Int = 100
 // dropped at this deadline, so a shutdown always finishes.
 const SERVER_REQUEST_MS: Int = 10000
 
-// A routing table and nothing else. Build one with `Server::new()` and `get`,
-// then `listen`.
+// A routing table, and the handler of a `GET` no route matches. Build one
+// with `Server::new()`, `get` and `fallback`, then `listen` or `listen_on`.
 /// A router that serves GET routes over HTTP/1.1.
 ///
 /// Build one with `Server::new()` and a call to `get` for each route, then
-/// serve it with `listen`. A request goes to the handler of the route whose
-/// path equals the request's target exactly, query string included; any
-/// other target, and any method but `GET`, gets `Response::not_found()`.
-pub record Server { routes: Vec<Route> }
+/// serve it with `listen`, or `listen_on` a listener already bound. A request
+/// goes to the handler of the route whose path equals the request's target
+/// exactly, query string included; any other `GET` goes to the `fallback`
+/// handler, when there is one; and any other request, any method but `GET`
+/// included, gets `Response::not_found()`.
+pub record Server { routes: Vec<Route>, unmatched: Option<fn(Request) -> Response> }
 
 // One GET route: an exact path, and the handler that answers it.
 record Route { path: String, handler: fn(Request) -> Response }
@@ -1018,8 +1021,8 @@ record Route { path: String, handler: fn(Request) -> Response }
 record InFlight { req: Request, rest: Bytes }
 
 impl Server {
-    /// Returns a server with no routes.
-    pub fn new() -> Server { Server { routes: Vec::new() } }
+    /// Returns a server with no routes and no fallback.
+    pub fn new() -> Server { Server { routes: Vec::new(), unmatched: None } }
 
     // Adds a GET route. Takes `self` rather than `mut self`, and returns
     // `Server` rather than section 6's `Self`: a `mut self` call on the
@@ -1048,15 +1051,46 @@ impl Server {
             i = i + 1
         }
         rs.push(Route { path: path, handler: handler })
-        Server { routes: rs }
+        Server { routes: rs, unmatched: self.unmatched }
+    }
+
+    // Sets the fallback. Like `get`, it takes `self` and returns a new
+    // server, so `self` keeps its own; the route table is shared between the
+    // two, which is safe since `get` copies it before it adds a route.
+    /// Returns a copy of this server whose `handler` answers each `GET`
+    /// request no route matches, in place of `Response::not_found()`.
+    ///
+    /// `self` is left unchanged, and calls chain with `get`'s in any order:
+    /// a route still answers its own path. A second `fallback` replaces the
+    /// first. Any method but `GET` still gets `not_found`.
+    ///
+    /// ```
+    /// let app = Server::new()
+    ///     .get("/", |_| Response::text(200, "home"))
+    ///     .fallback(|r: Request| Response::text(404, "no page at ${r.path}"))
+    /// let get = |path: String| Request {
+    ///     method: Get,
+    ///     path: path,
+    ///     headers: Map::new(),
+    ///     body: bytes_from_string(""),
+    /// }
+    /// assert_eq(app.dispatch(get("/")).body.to_string().unwrap(), "home")
+    /// let missing = app.dispatch(get("/nope?x=1"))
+    /// assert_eq(missing.status, 404)
+    /// assert_eq(missing.body.to_string().unwrap(), "no page at /nope?x=1")
+    /// ```
+    pub fn fallback(self, handler: fn(Request) -> Response) -> Server {
+        Server { routes: self.routes, unmatched: Some(handler) }
     }
 
     // The response to `req`: the handler of the GET route whose path equals
-    // `req.path` exactly, or `Response::not_found()` for any other path or
-    // method. Socket-free, so a test can call it directly.
+    // `req.path` exactly, else the fallback's for a GET, or
+    // `Response::not_found()` for any other path or method. Socket-free, so a
+    // test can call it directly.
     /// Returns the response to `req`: what the handler of the route whose
-    /// path equals `req.path` returns, or `Response::not_found()` if there is
-    /// none.
+    /// path equals `req.path` returns, or if there is none what the
+    /// `fallback` handler returns, or `Response::not_found()` if there is no
+    /// fallback either.
     ///
     /// Only a `GET` request reaches a handler; any other method, `HEAD`
     /// included, gets `not_found`. It touches no connection, so a test can
@@ -1079,7 +1113,10 @@ impl Server {
                     }
                     i = i + 1
                 }
-                Response::not_found()
+                match self.unmatched {
+                    Some(h) => h(req)
+                    None => Response::not_found()
+                }
             }
             _ => Response::not_found()
         }
@@ -1144,6 +1181,28 @@ impl Server {
             Ok(l) => l
             Err(e) => return Err(e)
         }
+        self.listen_on(l).await
+    }
+
+    // `listen`'s loop, on a listener the caller bound: so a program can bind
+    // port 0 and print the port it got, then serve it (spec
+    // docs/superpowers/specs/2026-10-19-phase-4-6-ssr-hydration-and-ssg-design.md
+    // §8.3). `listen` binds, then calls it. The signal handler is installed
+    // here too, for a caller that did not install it before its own `bind`;
+    // a second install is a no-op.
+    /// Serves HTTP on `listener`, already bound with `std/net`'s `bind`, until
+    /// the process is asked to stop, as `listen` does on the address it binds.
+    ///
+    /// A program binds first when it must know the address before serving,
+    /// such as the port the system chose for port 0, which `local_port`
+    /// reads. Everything else is as `listen` says: the connections, the
+    /// limits, the stop on SIGTERM or SIGINT, after which the listener is
+    /// closed and it returns `Ok(())`, and an `IoError` if accepting a
+    /// connection fails. A signal that arrives between the `bind` and this
+    /// call ends the program, unless something has already called `listen`
+    /// or `listen_on`.
+    pub async fn listen_on(self, l: TcpListener) -> Result<(), IoError> {
+        let _ = shutdown_requested()
         // The flag is checked on every turn, not only when an accept times
         // out: under steady traffic the timeout arm may never run. The accept
         // is polled under `timeout` because a bare one never returns without a
diff --git a/std/ui/router/lib.nova b/std/ui/router/lib.nova
index a70eee0b38598bc056106820a283eaa4a8afca8c..1523c1523fc781f5e5564d5d47c0b3f41d29d8a0 100644
--- a/std/ui/router/lib.nova
+++ b/std/ui/router/lib.nova
@@ -18,8 +18,11 @@
 //! matches. There is one router per page: the last one made is the one
 //! `navigate`, `use_params` and `use_query` speak to.
 //!
-//! Natively, where there is no address, the location is `/`, so
-//! `render_to_string` writes the root route:
+//! A server renders the page at another location with `with_location`,
+//! answers `404` where `found` says only the catch-all serves the location,
+//! and pre-renders the `static_paths`. Natively, where there is no address,
+//! the location is otherwise `/`, so `render_to_string` writes the root
+//! route:
 //!
 //! ```
 //! import ui
@@ -80,14 +83,18 @@ record Active { id: Int, history: Bool, location: Signal<String> }
 
 // The router's state for the thread or the page (spec §3.5): the router
 // mounted last, the parameters of the route being built while it is, the
-// next router's number, and a signal set each time the router mounted last
-// changes, which every link's `href` reads. A signal has no owner, so the
-// state can make it whenever it is first asked for.
+// next router's number, a signal set each time the router mounted last
+// changes, which every link's `href` reads, and the location a router made
+// inside `with_location` starts at (spec
+// docs/superpowers/specs/2026-10-19-phase-4-6-ssr-hydration-and-ssg-design.md
+// §5). A signal has no owner, so the state can make it whenever it is first
+// asked for.
 record Routing {
     active: Option<Active>,
     params: Option<Map<String, String>>,
     next_id: Int,
     changes: Signal<Int>,
+    location: Option<String>,
 }
 
 // A route that matched: its component and the parameters its pattern took.
@@ -100,7 +107,9 @@ fn store_routing<T>(state: T) { router_state_store(state) }
 // The state, made the first time it is asked for.
 fn routing() -> Routing {
     if !router_state_present() {
-        store_routing(Routing { active: None, params: None, next_id: 1, changes: signal(0) })
+        store_routing(
+            Routing { active: None, params: None, next_id: 1, changes: signal(0), location: None },
+        )
     }
     load_routing()
 }
@@ -177,7 +186,7 @@ fn routed(history: Bool, routes: [Route]) -> View {
     let mut s = routing()
     let id = s.next_id
     s.next_id = id + 1
-    let location = signal(located(history))
+    let location = signal(starting(history))
     s.active = Some(Active { id: id, history: history, location: location })
     changed(s)
     // `navigate` sets the location itself, to what the page reports, before
@@ -303,6 +312,15 @@ pub fn navigate(to: String) {
 // The page's location, less any `#fragment`.
 fn located(history: Bool) -> String { unfragmented(backend_location(history)) }
 
+// The location a router made now starts at: the one `with_location` gives,
+// or else the page's, either less any `#fragment`.
+fn starting(history: Bool) -> String {
+    match routing().location {
+        Some(at) => unfragmented(at)
+        None => located(history)
+    }
+}
+
 // `location` before its first `#`: a router's location never holds a
 // fragment. One reaches it in hash mode as a second `#`, through
 // `navigate`, a `link` or a typed hash, and natively through `navigate`;
@@ -450,6 +468,127 @@ pub fn use_query() -> Map<String, String> {
     }
 }
 
+/// Runs `f` so that a router made inside it starts at `location`, a path
+/// and its query as a request's target writes them, such as `/posts/1?tab=2`,
+/// rather than at the page's address. The previous location is restored once
+/// `f` returns, so calls nest. It is how a server renders the page at a
+/// path, natively, where the page's location is always `/`; on WASM it
+/// applies too. A router made inside it reads the page's address again each
+/// time that changes, as any router does.
+///
+/// ```
+/// import ui
+/// import ui::html::{p, text}
+/// import ui::router::{HistoryRouter, Route, use_params, with_location}
+///
+/// component Post() {
+///     let id = use_params().get("id").unwrap_or("?")
+///     view { p { text("Post ${id}") } }
+/// }
+///
+/// fn main() {
+///     let routes = [Route { path: "/posts/:id", component: Post }]
+///     let html = with_location("/posts/1", || render_to_string(|| HistoryRouter(routes)))
+///     assert_eq(html, "<p>Post 1</p>")
+///     // Outside it, a router starts at the page's location: natively `/`,
+///     // which no route matches.
+///     assert_eq(render_to_string(|| HistoryRouter(routes)), "")
+/// }
+/// ```
+pub fn with_location<T>(location: String, f: fn() -> T) -> T {
+    let mut s = routing()
+    let was = s.location
+    s.location = Some(location)
+    let out = f()
+    s.location = was
+    out
+}
+
+/// Whether a route other than the catch-all serves `location`, a path and
+/// its query: true when the first route that matches it, as a router picks
+/// it, is not `"*"` (nor `"/*"`), even when its pattern has parameters, and
+/// false when only the catch-all matches, or no route does. A `#fragment` is
+/// left out, as `with_location` leaves it out. A server answers `404` with
+/// the page when it is false.
+///
+/// ```
+/// import ui
+/// import ui::html::{p, text}
+/// import ui::router::{Route, found}
+///
+/// component Page() { view { p { text("A page") } } }
+///
+/// fn main() {
+///     let routes = [
+///         Route { path: "/", component: Page },
+///         Route { path: "/posts/:id", component: Page },
+///         Route { path: "*", component: Page },
+///     ]
+///     assert(found(routes, "/"), "a static route")
+///     assert(found(routes, "/posts/1?tab=2"), "a route with a parameter")
+///     assert(found(routes, "/#top"), "a fragment")
+///     assert(!found(routes, "/nowhere"), "only the catch-all")
+///     assert(!found([Route { path: "/", component: Page }], "/about"), "no route")
+/// }
+/// ```
+pub fn found(routes: [Route], location: String) -> Bool {
+    match first_match(routes, unfragmented(location)) {
+        Some(hit) => !catch_all(pieces_of(hit.route.path))
+        None => false
+    }
+}
+
+/// The paths of the static routes, which a site can render ahead of time:
+/// those with no `:name` piece that are not the catch-all, in order, each as
+/// written, and each once. `nova build --ssg` writes a page for each.
+///
+/// ```
+/// import ui
+/// import ui::html::{p, text}
+/// import ui::router::{Route, static_paths}
+///
+/// component Page() { view { p { text("A page") } } }
+///
+/// fn main() {
+///     let routes = [
+///         Route { path: "/", component: Page },
+///         Route { path: "/about", component: Page },
+///         Route { path: "/posts/:id", component: Page },
+///         Route { path: "/about", component: Page },
+///         Route { path: "*", component: Page },
+///     ]
+///     assert_eq(", ".join(static_paths(routes)), "/, /about")
+/// }
+/// ```
+pub fn static_paths(routes: [Route]) -> [String] {
+    let mut kept = Vec::new()
+    let mut i = 0
+    while i < routes.len() {
+        let path = routes[i].path
+        let pattern = pieces_of(path)
+        let mut fixed = !catch_all(pattern)
+        let mut k = 0
+        while k < pattern.len() {
+            if pattern[k].starts_with(":") { fixed = false }
+            k = k + 1
+        }
+        let mut j = 0
+        while j < kept.len() {
+            if kept.get(j).unwrap_or("") == path { fixed = false }
+            j = j + 1
+        }
+        if fixed { kept.push(path) }
+        i = i + 1
+    }
+    let mut out = [""; kept.len()]
+    let mut n = 0
+    while n < kept.len() {
+        out[n] = kept.get(n).unwrap_or("")
+        n = n + 1
+    }
+    out
+}
+
 /// An `<a>` that goes to a route without loading the page. Its `to`
 /// attribute, fixed or reactive, is the path, and becomes its `href`:
 /// `#/posts/1` under `Router`, and `/posts/1` under `HistoryRouter` and with
@@ -540,6 +679,20 @@ fn rooted(to: String) -> String {
 
 // The routes that match `location`: the first that does, or none.
 fn matching(routes: [Route], location: String) -> [Matched] {
+    match first_match(routes, location) {
+        Some(hit) => [Matched { component: hit.route.component, params: hit.params }]
+        None => {
+            let none: [Matched] = []
+            none
+        }
+    }
+}
+
+// A route that matched a location, and the parameters its pattern took.
+record Hit { route: Route, params: Map<String, String> }
+
+// The first route that matches `location`, a path and its query, or `None`.
+fn first_match(routes: [Route], location: String) -> Option<Hit> {
     let path = match location.index_of("?") {
         Some(at) => location.slice(0, at)
         None => location
@@ -549,20 +702,22 @@ fn matching(routes: [Route], location: String) -> [Matched] {
     while i < routes.len() {
         let r = routes[i]
         match params_of(pieces_of(r.path), pieces) {
-            Some(params) => return [Matched { component: r.component, params: params }]
+            Some(params) => return Some(Hit { route: r, params: params })
             None => {}
         }
         i = i + 1
     }
-    let none: [Matched] = []
-    none
+    None
 }
 
+// Whether `pattern`, a route's pieces, is the catch-all's: `"*"`, or `"/*"`.
+fn catch_all(pattern: [String]) -> Bool { pattern.len() == 1 && pattern[0] == "*" }
+
 // The parameters `pattern` takes from `pieces`, or `None` when it does not
 // match them.
 fn params_of(pattern: [String], pieces: [String]) -> Option<Map<String, String>> {
     let mut params: Map<String, String> = Map::new()
-    if pattern.len() == 1 && pattern[0] == "*" { return Some(params) }
+    if catch_all(pattern) { return Some(params) }
     if pattern.len() != pieces.len() { return None }
     let mut i = 0
     while i < pattern.len() {
````


### Task 3: `ui::ssr`

Spec §6, §8.4; decisions 5, 7, 8, 10, 19, 21 to 24, 26, 27, 33, 34 and
38.

**Files:**
- Create: `std/ui/ssr/lib.nova`
- Modify:
  - `std/lib.rs` (`UI_SSR`, `ALL`)
  - `crates/nova-resolver/src/lib.rs` (`STD_MODULES`, `IMPORTED_STD`,
    `missing_names`)
  - `crates/nova-doc/src/model.rs` (its test's list of modules)
- Test:
  - `tests/ssr/site/nova.toml`, `src/app.nova`, `src/main.nova`,
    `src/server.nova`, `src/refuse.nova`, `src/elsewhere.nova`
  - `crates/nova-cli/tests/run_tests.rs`, `wasm_tests.rs`, `doc_test.rs`
  - `crates/nova-resolver/src/lib.rs` (its test alone)
  - `crates/nova-driver/tests/index_complete.rs`

- [ ] **Step 1: The tests.** `patch_block t3-tests`. The patch holds:
  - **`tests/ssr/site`,** a project with a client and a server (decision
    27): `src/app.nova` holds the data, a record with `ToJson` and
    `FromJson` whose title holds `</script>`, `&` and U+2028, the routes
    `/`, `/about`, `/docs/intro`, `/posts/:id` and `*`, and the app;
    `src/main.nova` hydrates it, with no title and no names as its
    fallback; `src/server.nova` serves `dist/` on
    `127.0.0.1:0`, prints an error and exits with 3 when `serve` fails, and
    prints `stopped` when a signal stops it; `src/elsewhere.nova` asks for
    `127.0.0.1:99999`; and `src/refuse.nova` puts a static route whose
    path names no file third, after `/` and `/about`: `/a/../b`, or, when
    its first argument is `empty`, `backslash`, `nul`, `query` or `hash`,
    `/a//b`, `/a\b`, `/a` NUL `b`, `/a?b` or `/a#b`.
  - **`run_tests.rs`,** each test in a fresh copy of the project with
    `dist/` written by hand: a template, a script, a WASM module and an
    image the manifest lists, and a file it does not; a file outside
    `dist/` too:
    - `ssr_page_page_with_and_document`: `page` at `/posts/1?tab=2`, query
      and all, with no data; `page_with` of a string holding `</script>`,
      `&`, U+2028, U+2029 and `<!--`, its data `"a \u003c/script\u003e
      \u0026 b \u2028\u2029 \u003c!--"`, which parses back to the same
      string; `document` filling the first `<div id="app">` alone, with the
      data's `script` after it when there is data, a root that holds
      whitespace, a placeholder with a `div` of its own, a `<divx>`, or a
      page and its data as SSG writes them emptied first, and one never
      closed keeping what follows it (decisions 34 and 38); a stale data
      `script` past a line feed after the root dropped, the line feed kept,
      though the page has no data, and one with no `</script>` left as it
      is (decisions 19 and 38); then the panic `ssr: the page has no <div
      id="app">`.
    - `ssr_serve_answers_files_and_pages`, on the port the server prints:
      `/` and `/posts/7?tab=1` are pages with their data; the four listed
      files come with their types and `nosniff`, `app.js` asked for as
      `/app%2ejs` and `/app.js?v=1` too; `/missing.png`, `/secret.txt`,
      `/manifest.json`, `/gone.js`, which the manifest lists and `dist/`
      lacks (decision 22), `/..%2fsecret` and `/%2e%2e/secret`, though the
      manifest lists `../secret` (decision 33), are `404`, and none shows
      what it should not; `/nowhere` is `404`
      and the rendered "Nothing here"; a `POST` is `404`; a termination
      request stops it, `serve` returns `Ok`, and the program prints
      `stopped` and exits with 0. A page's data may come with its two keys
      in either order (decision 30).
    - `ssr_serve_reports_what_stops_it_starting`: exit 3, and an address
      that cannot be bound, a manifest with no `files`, no manifest, a
      template with no root and no template, each named (decision 21).
    - `ssr_ssg_writes_the_static_pages_and_refuses_a_bad_path`: `--ssg
      dist` writes `/`, `/about` and `/docs/intro`, each line printed,
      none for `/posts/:id` or `*`, and run again over its own output
      writes the same pages (decision 19); `refuse.nova`'s six paths, each
      into a fresh `fresh/`: exit 3, the two pages before it, the refusal
      naming the path, and nothing more on disk; and `--ssg` with no
      directory.
  - **The import:** the resolver's `ssr_is_imported_by_name_and_only_natively`,
    and `wasm_tests.rs`'s of the same name: natively `import ui::ssr::{…}`
    binds what it names; on WASM the import is E0092, ``error[E0092]:
    `ui::ssr` is a std module, which target wasm does not have``; and a bare
    `page` is ``error[E0001]: cannot find `page` in this scope`` on both
    targets (decision 24).
  - **`doc_test.rs`:** `std::ui::ssr`'s examples must run, as
    `std::ui::html`'s and `std::ui::router`'s do.
  - **`index_complete.rs`:** 182 files, native std's 22.
- [ ] **Step 2: Run, and see them fail.**
  - `R ssr_` (about 15 s). Expected: `0 passed; 4 failed`, each program's
    build ``error[E0001]: cannot find module `ui::ssr` ``, `could not
    compile due to 1 previous error`:
    - `ssr_page_page_with_and_document`: ``assertion `left == right`
      failed``, `left: ""`;
    - `ssr_serve_answers_files_and_pages`: `no serving line, but ""`, with
      the server's stderr;
    - `ssr_serve_reports_what_stops_it_starting`: `Unexpected return code,
      failed var == 3`, `var: 1`;
    - `ssr_ssg_writes_the_static_pages_and_refuses_a_bad_path`:
      "Unexpected failure.".
  - `cargo test -p nova-resolver --lib ssr_is_imported_by_name_and_only_natively`.
    Expected: `0 passed; 1 failed`, at `lib.rs:3659`: the native
    diagnostics are not empty, `[Diagnostic { severity: Error, code:
    "E0001", message: "cannot find module `ui::ssr`", … }]`.
  - `W ssr_is_imported_by_name_and_only_natively`. Expected: `0 passed; 1
    failed`: "Unexpected failure." of the native `nova check main.nova`:
    the same E0001.
  - `cargo test -p nova-cli --test doc_test doc_std_runs_stds_doc_tests`.
    Expected: `0 passed; 1 failed`:
    `doc_std_runs_stds_doc_tests_inside_a_project_or_not`
    (`doc_test.rs:413`), "no example of std::ui::ssr ran:", after std's own
    run, `48 passed; 0 failed; 0 trapped; 48 total`.
  - `X`. Expected: `0 passed; 1 failed`, at `index_complete.rs:276`:
    ``assertion `left == right` failed``, `left: 181`, `right: 182`.
  - `cargo test -p nova-doc --lib`. Expected: `61 passed`: its list of
    modules changes with the code, which it follows.
- [ ] **Step 3: The code.** `patch_block t3-code`:
  - **`std/ui/ssr/lib.nova`:** `Page`, `page`, `page_with` and its
    `script_safe`, and `document`, which empties the root it fills
    (decisions 5, 7 and 34); `Site` and `serve`: `ssg_dir`, the template
    read and `emptied`, with `html_space` (decisions 19 and 38), the manifest read and
    `listed`, which drops a path that could leave `dist` through `inside`,
    `plain_path` and `plain_piece` (decision 33), the bind, the URL line and its flush, then
    `Server::new().fallback(…).listen_on(l)` (decisions 8 and 21); `Served`,
    `answer`, `decoded` and `hex_digit`, `file` (decisions 22, 23 and 26)
    and `content_type`; `generate` and `file_pieces`, which checks each
    piece with `plain_piece` too (decision 10);
    `app_root`, `matches_at`, `ends_name`, `unreadable`, `rootless` and
    `host_of`. Its doc tests render natively: the module's, `page`'s,
    `page_with`'s and `document`'s; `serve`'s is marked `ignore`, since it
    serves until a signal.
  - **Registration:** `std/lib.rs`'s `UI_SSR`, `ALL` (21) and its test's
    25 files; the resolver's `STD_MODULES` (21) and `IMPORTED_STD` (5), and
    `missing_names`, which leaves an imported module's names out (decision
    24); `nova-doc`'s list of modules gains `ui/ssr`.
- [ ] **Step 4: Run.**
  - The same commands. Expected: `R ssr_` `4 passed` (about 15 s); the
    resolver's test `1 passed`; `W ssr_is_imported_by_name_and_only_natively`
    `1 passed`; `doc_test doc_std_runs_stds_doc_tests` `1 passed`; `X` `1
    passed`.
  - The whole binaries:
    - `cargo test -p nova-resolver`. Expected: `54 passed` (53 before).
    - `cargo test -p nova-std`. Expected: `2 passed`.
    - `cargo test -p nova-doc --lib`. Expected: `61 passed`.
    - `cargo test -p nova-cli --test doc_test`. Expected: `17 passed`.
    - `R`. Expected: `317 passed; 0 failed; 1 ignored`.
    - `W`. Expected: `229 passed`.
    - `cargo test -p nova-driver`. Expected: each binary passes: the
      library `37`, `analyze` `12`, `broken` `1`, `doc_tests` `8`, `fixes`
      `33`, `index` `24`, `index_complete` `1`, `packages` `14`, `registry`
      `5`.
- [ ] **Step 5: The checks,** as the conventions list them, `nova fmt`'s
  now with `tests/ssr`.
- [ ] **Step 6: Commit** `std/ui/ssr: page, page_with, document and serve, which a native program imports`.

**Patch `t3-tests`:**

<!-- patch t3-tests -->

````diff
diff --git a/crates/nova-cli/tests/doc_test.rs b/crates/nova-cli/tests/doc_test.rs
index b94fbc33ef17bebc0c6db55b154b473d5c24f295..8af44e1216e78c6fba1b67a722b8c492c7624797 100644
--- a/crates/nova-cli/tests/doc_test.rs
+++ b/crates/nova-cli/tests/doc_test.rs
@@ -409,7 +409,7 @@ fn doc_std_runs_stds_doc_tests_inside_a_project_or_not() {
                 "no example of std::{m} ran:\n{out}"
             );
         }
-        for m in ["html", "router"] {
+        for m in ["html", "router", "ssr"] {
             assert!(
                 out.contains(&format!("\ntest std::ui::{m} (std/ui/{m}/lib.nova:")),
                 "no example of std::ui::{m} ran:\n{out}"
diff --git a/crates/nova-cli/tests/run_tests.rs b/crates/nova-cli/tests/run_tests.rs
index fd5ea8f0e47e3321cf62181c19327ff5ed7bf26b..2d7c0ff22db0359a985e7ad94d5618aca7561be7 100644
--- a/crates/nova-cli/tests/run_tests.rs
+++ b/crates/nova-cli/tests/run_tests.rs
@@ -11550,3 +11550,646 @@ fn ui_mount_panics_natively() {
         "{err}"
     );
 }
+
+/// `ui::ssr`'s low level (spec
+/// `docs/superpowers/specs/2026-10-19-phase-4-6-ssr-hydration-and-ssg-design.md`
+/// §6.2): `page` renders at its location, query and all, with no data;
+/// `page_with` renders its data's view, and writes the data as JSON with
+/// `<`, `>`, `&`, U+2028 and U+2029 as escapes, which parse back to the same
+/// value; and `document` fills the first `<div id="app">` alone, with the
+/// data's `script` after it when there is data. A root that holds
+/// whitespace, a placeholder with a `div` of its own, a `<divx>`, which is
+/// no `div`, or a page and its data, as SSG writes them, is emptied first,
+/// and one never closed keeps what follows it. A stale data `script` past
+/// whitespace after the root goes too, the whitespace kept, though the page
+/// has no data; one with no `</script>` stays as it is. Then `document`
+/// panics on a template with no root, after what the program printed
+/// first.
+#[test]
+fn ssr_page_page_with_and_document() {
+    let dir = std::env::temp_dir().join("nova-ssr-page");
+    let _ = std::fs::remove_dir_all(&dir);
+    std::fs::create_dir_all(&dir).expect("temp dir");
+    let program = "import ui\n\
+        import ui::html::{p, text}\n\
+        import ui::router::{HistoryRouter, Route, use_params, use_query}\n\
+        import ui::ssr::{Page, document, page, page_with}\n\
+        \n\
+        component Post() {\n\
+        \x20   let id = use_params().get(\"id\").unwrap_or(\"?\")\n\
+        \x20   let tab = use_query().get(\"tab\").unwrap_or(\"?\")\n\
+        \x20   view { p { text(\"post ${id}, tab ${tab}\") } }\n\
+        }\n\
+        \n\
+        fn shown(s: String) -> View { p([], [text(s)]) }\n\
+        \n\
+        fn main() {\n\
+        \x20   let routes = [Route { path: \"/posts/:id\", component: Post }]\n\
+        \x20   let a = page(\"/posts/1?tab=2\", | | HistoryRouter(routes))\n\
+        \x20   println(\"page: ${a.body} ${a.data.is_none()}\")\n\
+        \x20   let words = \"a </script> & b \\u{2028}\\u{2029} <!--\"\n\
+        \x20   let b = page_with(\"/\", words, shown)\n\
+        \x20   println(\"page_with: ${b.body}\")\n\
+        \x20   let data = b.data.unwrap_or(\"none\")\n\
+        \x20   println(\"data: ${data}\")\n\
+        \x20   let back = match parse(data) {\n\
+        \x20       Ok(v) => String::from_json(v).unwrap_or(\"not a string\")\n\
+        \x20       Err(e) => e.msg\n\
+        \x20   }\n\
+        \x20   println(\"parses back: ${back == words}\")\n\
+        \x20   let t = \"<body><div id=\\\"app\\\"></div><div id=\\\"app\\\"></div></body>\"\n\
+        \x20   println(\"document: ${document(t, a)}\")\n\
+        \x20   let filled = Page { body: \"x\", data: Some(\"[1]\") }\n\
+        \x20   println(\"with data: ${document(t, filled)}\")\n\
+        \x20   let blank = \"<div id=\\\"app\\\"> \\n\\t</div><p>after</p>\"\n\
+        \x20   println(\"blank: ${document(blank, filled)}\")\n\
+        \x20   let waiting = \"<main><div id=\\\"app\\\"><div class=\\\"spinner\\\"><p>Loading</p></div></div><script src=\\\"/app.js\\\"></script></main>\"\n\
+        \x20   println(\"placeholder: ${document(waiting, filled)}\")\n\
+        \x20   let divx = \"<div id=\\\"app\\\"><divx>old</divx></div><p>after</p>\"\n\
+        \x20   println(\"divx: ${document(divx, filled)}\")\n\
+        \x20   let built = \"<div id=\\\"app\\\"><p>old</p></div><script type=\\\"application/json\\\" id=\\\"nova-data\\\">[0]</script><p>after</p>\"\n\
+        \x20   println(\"rendered: ${document(built, filled)}\")\n\
+        \x20   let open = \"<div id=\\\"app\\\"><p>open\"\n\
+        \x20   println(\"unclosed: ${document(open, filled)}\")\n\
+        \x20   let bare = Page { body: \"x\", data: None }\n\
+        \x20   let spaced = \"<div id=\\\"app\\\"><p>old</p></div>\\n<script type=\\\"application/json\\\" id=\\\"nova-data\\\">[0]</script><p>after</p>\"\n\
+        \x20   println(\"spaced: ${document(spaced, bare)}\")\n\
+        \x20   let cut = \"<div id=\\\"app\\\"><p>old</p></div><script type=\\\"application/json\\\" id=\\\"nova-data\\\">[0]<p>after</p>\"\n\
+        \x20   println(\"unclosed data: ${document(cut, filled)}\")\n\
+        \x20   println(document(\"<div id=\\\"root\\\"></div>\", a))\n\
+        \x20   println(\"not reached\")\n\
+        }\n";
+    std::fs::write(dir.join("main.nova"), program).expect("write the program");
+    let a = nova()
+        .current_dir(&dir)
+        .env("RUST_BACKTRACE", "0")
+        .arg("run")
+        .arg("main.nova")
+        .assert()
+        .failure();
+    let o = a.get_output();
+    assert_eq!(
+        String::from_utf8_lossy(&o.stdout).replace("\r\n", "\n"),
+        "page: <!--[--><p>post 1, tab 2</p><!--]--> true\n\
+         page_with: <p>a &lt;/script&gt; &amp; b \u{2028}\u{2029} &lt;!--</p>\n\
+         data: \"a \\u003c/script\\u003e \\u0026 b \\u2028\\u2029 \\u003c!--\"\n\
+         parses back: true\n\
+         document: <body><div id=\"app\"><!--[--><p>post 1, tab 2</p><!--]--></div><div id=\"app\"></div></body>\n\
+         with data: <body><div id=\"app\">x</div><script type=\"application/json\" id=\"nova-data\">[1]</script><div id=\"app\"></div></body>\n\
+         blank: <div id=\"app\">x</div><script type=\"application/json\" id=\"nova-data\">[1]</script><p>after</p>\n\
+         placeholder: <main><div id=\"app\">x</div><script type=\"application/json\" id=\"nova-data\">[1]</script><script src=\"/app.js\"></script></main>\n\
+         divx: <div id=\"app\">x</div><script type=\"application/json\" id=\"nova-data\">[1]</script><p>after</p>\n\
+         rendered: <div id=\"app\">x</div><script type=\"application/json\" id=\"nova-data\">[1]</script><p>after</p>\n\
+         unclosed: <div id=\"app\">x</div><script type=\"application/json\" id=\"nova-data\">[1]</script><p>open\n\
+         spaced: <div id=\"app\">x</div>\n<p>after</p>\n\
+         unclosed data: <div id=\"app\">x</div><script type=\"application/json\" id=\"nova-data\">[1]</script><script type=\"application/json\" id=\"nova-data\">[0]<p>after</p>\n"
+    );
+    let err = String::from_utf8_lossy(&o.stderr).replace("\r\n", "\n");
+    assert!(
+        err.ends_with("nova: panic: ssr: the page has no <div id=\"app\">\n"),
+        "{err}"
+    );
+}
+
+/// A copy of `tests/ssr/site` in a fresh directory named for `name`, with a
+/// fresh `NOVA_HOME` beside it, and `dist/` written by hand: a template, a
+/// script, a WASM module and an image the manifest lists, and a file it does
+/// not. A file sits outside `dist/` too, `secret`, which the manifest lists
+/// as `../secret`; and the manifest lists `gone.js`, which is not there.
+fn ssr_site(name: &str) -> (PathBuf, PathBuf) {
+    let base = std::env::temp_dir().join(format!("nova-ssr-{name}"));
+    let _ = std::fs::remove_dir_all(&base);
+    let site = base.join("site");
+    std::fs::create_dir_all(site.join("src")).expect("temp dir");
+    let fixture = repo_root().join("tests/ssr/site");
+    std::fs::copy(fixture.join("nova.toml"), site.join("nova.toml")).expect("copy nova.toml");
+    for entry in std::fs::read_dir(fixture.join("src")).expect("the fixture's src") {
+        let path = entry.expect("an entry").path();
+        std::fs::copy(
+            &path,
+            site.join("src").join(path.file_name().expect("a name")),
+        )
+        .expect("copy a source");
+    }
+    let dist = site.join("dist");
+    std::fs::create_dir_all(dist.join("assets")).expect("dist");
+    for (path, text) in [
+        ("index.html", SSR_TEMPLATE),
+        ("app.js", "export default function init() {}\n"),
+        ("assets/logo.svg", "<svg></svg>"),
+        ("secret.txt", "not listed"),
+        (
+            "manifest.json",
+            "{\"files\": {\"index.html\": 1, \"app.js\": 1, \"app.wasm\": 8, \"assets/logo.svg\": 1, \
+             \"../secret\": 12, \"gone.js\": 1}}\n",
+        ),
+    ] {
+        std::fs::write(dist.join(path), text).expect("write dist");
+    }
+    std::fs::write(dist.join("app.wasm"), b"\0asm\x01\0\0\0").expect("write app.wasm");
+    std::fs::write(site.join("secret"), "outside dist").expect("write a secret");
+    (site, base.join("home"))
+}
+
+/// The template `ssr_site` writes.
+const SSR_TEMPLATE: &str = "<!doctype html>\n<title>Site</title>\n<body>\n<div id=\"app\"></div>\n<script type=\"module\" src=\"/app.js\"></script>\n</body>\n";
+
+/// What `tests/ssr/site`'s app renders around `route`, the route's own HTML.
+fn ssr_view(route: &str) -> String {
+    format!(
+        "<div><h1>Names &lt;/script&gt; &amp; \u{2028} more</h1><ul><!--[--><li>Ada</li><li>Grace</li><!--]--></ul><!--[-->{route}<!--]--></div>"
+    )
+}
+
+/// The page `document` makes of `template` for `view` with the fixture's
+/// data: one for each order of the data's two keys, which a `Map` decides.
+fn ssr_pages(template: &str, view: &str) -> [String; 2] {
+    let title = "\"title\":\"Names \\u003c/script\\u003e \\u0026 \\u2028 more\"";
+    let names = "\"names\":[\"Ada\",\"Grace\"]";
+    [
+        format!("{{{title},{names}}}"),
+        format!("{{{names},{title}}}"),
+    ]
+    .map(|json| {
+        template.replacen(
+            "<div id=\"app\"></div>",
+            &format!(
+                "<div id=\"app\">{view}</div><script type=\"application/json\" id=\"nova-data\">{json}</script>"
+            ),
+            1,
+        )
+    })
+}
+
+/// A running `nova run src/<entry>.nova` in a copy of `tests/ssr/site`,
+/// with the port its first line names. Its streams are drained on threads,
+/// and dropping it kills a server still running.
+struct SsrServer {
+    child: std::process::Child,
+    port: u16,
+    stdout: Option<std::thread::JoinHandle<String>>,
+    stderr: Option<std::thread::JoinHandle<String>>,
+}
+
+impl SsrServer {
+    fn spawn(site: &std::path::Path, home: &std::path::Path) -> Self {
+        use std::io::{BufRead, BufReader, Read};
+        use std::process::Stdio;
+        let mut cmd = std::process::Command::new(assert_cmd::cargo::cargo_bin("nova"));
+        cmd.current_dir(site)
+            .env("NOVA_HOME", home)
+            .env("NO_COLOR", "1")
+            .env("RUST_BACKTRACE", "0")
+            .args(["run", "src/server.nova"])
+            .stdin(Stdio::null())
+            .stdout(Stdio::piped())
+            .stderr(Stdio::piped());
+        #[cfg(windows)]
+        {
+            use std::os::windows::process::CommandExt;
+            cmd.creation_flags(0x0000_0200);
+        }
+        let mut child = cmd.spawn().expect("spawn the server");
+        let mut out = BufReader::new(child.stdout.take().expect("stdout was piped"));
+        let mut err = child.stderr.take().expect("stderr was piped");
+        let stderr = std::thread::spawn(move || {
+            let mut s = String::new();
+            let _ = err.read_to_string(&mut s);
+            s
+        });
+        // The first line comes once the program has compiled and bound; if
+        // it fails first, the stream closes and the line is empty.
+        let mut first = String::new();
+        let _ = out.read_line(&mut first);
+        let stdout = std::thread::spawn(move || {
+            let mut s = String::new();
+            let _ = out.read_to_string(&mut s);
+            s
+        });
+        let mut server = SsrServer {
+            child,
+            port: 0,
+            stdout: Some(stdout),
+            stderr: Some(stderr),
+        };
+        match first
+            .trim_end()
+            .strip_prefix("ssr: serving http://127.0.0.1:")
+            .and_then(|p| p.strip_suffix('/'))
+            .and_then(|p| p.parse::<u16>().ok())
+        {
+            Some(port) => server.port = port,
+            None => server.kill_and_panic(&format!("no serving line, but {first:?}")),
+        }
+        server
+    }
+
+    /// One request on a connection of its own: the status, the headers by
+    /// lower-cased name, and the body, framed by `content-length`.
+    fn ask(&mut self, request: &str) -> (u16, std::collections::HashMap<String, String>, Vec<u8>) {
+        match ssr_exchange(self.port, request) {
+            Ok(r) => (r.status, r.headers, r.body),
+            Err(e) => self.kill_and_panic(&format!("{request:?}: {e}")),
+        }
+    }
+
+    fn wait_for_exit(&mut self, limit: std::time::Duration) -> Option<std::process::ExitStatus> {
+        let deadline = std::time::Instant::now() + limit;
+        loop {
+            match self.child.try_wait().expect("try_wait") {
+                Some(s) => return Some(s),
+                None if std::time::Instant::now() >= deadline => return None,
+                None => std::thread::sleep(std::time::Duration::from_millis(50)),
+            }
+        }
+    }
+
+    /// Both streams, the first line left out. Call once the process has
+    /// exited.
+    fn streams(&mut self) -> (String, String) {
+        let out = self
+            .stdout
+            .take()
+            .map(|h| h.join().unwrap_or_default())
+            .unwrap_or_default();
+        let err = self
+            .stderr
+            .take()
+            .map(|h| h.join().unwrap_or_default())
+            .unwrap_or_default();
+        (out, err)
+    }
+
+    fn kill_and_panic(&mut self, why: &str) -> ! {
+        let _ = self.child.kill();
+        let status = self.child.wait();
+        let (out, err) = self.streams();
+        panic!("{why}; killed: {status:?}; stdout={out:?} stderr={err:?}");
+    }
+}
+
+impl Drop for SsrServer {
+    fn drop(&mut self) {
+        if matches!(self.child.try_wait(), Ok(None)) {
+            let _ = self.child.kill();
+            let _ = self.child.wait();
+        }
+    }
+}
+
+/// A response `serve` sent: its status, its headers by lower-cased name, and
+/// its body.
+struct SsrResponse {
+    status: u16,
+    headers: std::collections::HashMap<String, String>,
+    body: Vec<u8>,
+}
+
+/// `request` sent to `127.0.0.1:port` on a new connection, and the response
+/// read whole.
+fn ssr_exchange(port: u16, request: &str) -> Result<SsrResponse, String> {
+    use std::io::{Read, Write};
+    let mut sock =
+        std::net::TcpStream::connect(("127.0.0.1", port)).map_err(|e| format!("connect: {e}"))?;
+    let limit = Some(std::time::Duration::from_secs(10));
+    sock.set_read_timeout(limit).map_err(|e| e.to_string())?;
+    sock.set_write_timeout(limit).map_err(|e| e.to_string())?;
+    sock.write_all(request.as_bytes())
+        .map_err(|e| format!("write: {e}"))?;
+    let mut buf = Vec::new();
+    let mut chunk = [0u8; 4096];
+    loop {
+        if let Some(end) = buf.windows(4).position(|w| w == b"\r\n\r\n").map(|i| i + 4) {
+            let head = String::from_utf8_lossy(&buf[..end]).to_string();
+            let mut lines = head.split("\r\n");
+            let status = lines
+                .next()
+                .and_then(|l| l.split_whitespace().nth(1))
+                .and_then(|s| s.parse::<u16>().ok())
+                .ok_or_else(|| format!("no status line in {head:?}"))?;
+            let mut headers = std::collections::HashMap::new();
+            for line in lines {
+                if let Some((name, value)) = line.split_once(':') {
+                    headers.insert(name.trim().to_ascii_lowercase(), value.trim().to_string());
+                }
+            }
+            let len: usize = headers
+                .get("content-length")
+                .and_then(|v| v.parse().ok())
+                .ok_or_else(|| format!("no content-length in {head:?}"))?;
+            if buf.len() >= end + len {
+                return Ok(SsrResponse {
+                    status,
+                    headers,
+                    body: buf[end..end + len].to_vec(),
+                });
+            }
+        }
+        match sock.read(&mut chunk) {
+            Ok(0) => return Err(format!("closed early: {:?}", String::from_utf8_lossy(&buf))),
+            Ok(n) => buf.extend_from_slice(&chunk[..n]),
+            Err(e) => return Err(format!("read: {e}")),
+        }
+    }
+}
+
+/// `ui::ssr::serve` end to end, on port 0 (spec 4.6 §6.3, §9.2): it prints
+/// the port it bound; `/` is the template filled with the page and its data;
+/// a page keeps its query and takes its route's parameter; the files the
+/// manifest lists come with their content types and `nosniff`, asked for
+/// with an escape or a query too; a name with a `.` it does not list, a file
+/// in `dist/` it does not list, a listed file not on disk, and a path that
+/// would leave `dist/`, which the manifest lists, all get `404`, and nothing
+/// outside `dist/` is read; a path only the catch-all serves gets `404` and
+/// the rendered page; a `POST` gets `404`; and a signal stops it, and
+/// `serve` returns `Ok`.
+#[test]
+fn ssr_serve_answers_files_and_pages() {
+    use std::time::Duration;
+    let (site, home) = ssr_site("serve");
+    let mut server = SsrServer::spawn(&site, &home);
+    let get = |path: &str| format!("GET {path} HTTP/1.1\r\nhost: x\r\n\r\n");
+    let text = |body: &[u8]| String::from_utf8_lossy(body).to_string();
+
+    let (status, headers, body) = server.ask(&get("/"));
+    assert_eq!(status, 200);
+    assert_eq!(headers["content-type"], "text/html; charset=utf-8");
+    let home_view = ssr_view("<p>Home <a href=\"/about\" data-nova-link=\"\">About</a></p>");
+    assert!(
+        ssr_pages(SSR_TEMPLATE, &home_view).contains(&text(&body)),
+        "{}",
+        text(&body)
+    );
+
+    let (status, _, body) = server.ask(&get("/posts/7?tab=1"));
+    assert_eq!(status, 200);
+    assert!(
+        ssr_pages(SSR_TEMPLATE, &ssr_view("<p>Post 7</p>")).contains(&text(&body)),
+        "{}",
+        text(&body)
+    );
+
+    for (path, kind, bytes) in [
+        (
+            "/app.js",
+            "text/javascript; charset=utf-8",
+            &b"export default function init() {}\n"[..],
+        ),
+        (
+            "/app%2ejs",
+            "text/javascript; charset=utf-8",
+            &b"export default function init() {}\n"[..],
+        ),
+        (
+            "/app.js?v=1",
+            "text/javascript; charset=utf-8",
+            &b"export default function init() {}\n"[..],
+        ),
+        ("/app.wasm", "application/wasm", &b"\0asm\x01\0\0\0"[..]),
+        ("/assets/logo.svg", "image/svg+xml", &b"<svg></svg>"[..]),
+        (
+            "/index.html",
+            "text/html; charset=utf-8",
+            SSR_TEMPLATE.as_bytes(),
+        ),
+    ] {
+        let (status, headers, body) = server.ask(&get(path));
+        assert_eq!(status, 200, "{path}");
+        assert_eq!(headers["content-type"], kind, "{path}");
+        assert_eq!(headers["x-content-type-options"], "nosniff", "{path}");
+        assert_eq!(body, bytes, "{path}");
+    }
+
+    for path in [
+        "/missing.png",
+        "/secret.txt",
+        "/manifest.json",
+        "/gone.js",
+        "/..%2fsecret",
+        "/%2e%2e/secret",
+    ] {
+        let (status, _, body) = server.ask(&get(path));
+        assert_eq!(status, 404, "{path}");
+        assert!(!text(&body).contains("outside dist"), "{path}");
+        assert!(!text(&body).contains("not listed"), "{path}");
+    }
+    for path in ["/missing.png", "/gone.js"] {
+        let (_, headers, body) = server.ask(&get(path));
+        assert_eq!(
+            headers["content-type"], "text/plain; charset=utf-8",
+            "{path}"
+        );
+        assert_eq!(text(&body), "not found", "{path}");
+    }
+
+    let (status, headers, body) = server.ask(&get("/nowhere"));
+    assert_eq!(status, 404);
+    assert_eq!(headers["content-type"], "text/html; charset=utf-8");
+    assert!(
+        ssr_pages(SSR_TEMPLATE, &ssr_view("<p>Nothing here</p>")).contains(&text(&body)),
+        "{}",
+        text(&body)
+    );
+
+    let (status, _, body) = server.ask("POST / HTTP/1.1\r\nhost: x\r\ncontent-length: 2\r\n\r\nhi");
+    assert_eq!(status, 404);
+    assert_eq!(text(&body), "not found");
+
+    if let Err(e) = send_termination_request(server.child.id()) {
+        server.kill_and_panic(&format!("could not deliver the signal: {e}"));
+    }
+    let status = match server.wait_for_exit(Duration::from_secs(10)) {
+        Some(s) => s,
+        None => server.kill_and_panic("the server did not stop within 10 s of the signal"),
+    };
+    let (out, err) = server.streams();
+    assert_eq!(
+        status.code(),
+        Some(0),
+        "{}; stdout={out:?} stderr={err:?}",
+        describe_status(&status)
+    );
+    assert_eq!(out.replace("\r\n", "\n"), "stopped\n", "stderr={err:?}");
+}
+
+/// What stops `serve` starting (spec 4.6 §6.3), each an `Err` naming its
+/// file or address, which the fixture's server prints before it exits with
+/// 3: no template, a template with no root, no manifest, a manifest with no
+/// `files`, and an address that cannot be bound.
+#[test]
+fn ssr_serve_reports_what_stops_it_starting() {
+    let (site, home) = ssr_site("start");
+    let dist = site.join("dist");
+    let run = |entry: &str| {
+        let a = nova()
+            .current_dir(&site)
+            .env("NOVA_HOME", &home)
+            .env("NO_COLOR", "1")
+            .env("RUST_BACKTRACE", "0")
+            .args(["run", entry])
+            .assert()
+            .code(3);
+        let o = a.get_output();
+        assert_eq!(String::from_utf8_lossy(&o.stdout), "", "{entry}");
+        String::from_utf8_lossy(&o.stderr).replace("\r\n", "\n")
+    };
+    let err = run("src/elsewhere.nova");
+    assert!(
+        err.starts_with("ssr: cannot bind 127.0.0.1:99999: "),
+        "{err}"
+    );
+
+    std::fs::write(dist.join("manifest.json"), "{\"name\": \"site\"}\n").expect("a manifest");
+    assert_eq!(
+        run("src/server.nova"),
+        "ssr: dist/manifest.json has no \"files\" object\n"
+    );
+    std::fs::remove_file(dist.join("manifest.json")).expect("remove the manifest");
+    let err = run("src/server.nova");
+    assert!(
+        err.starts_with("ssr: cannot read dist/manifest.json: "),
+        "{err}"
+    );
+
+    std::fs::write(dist.join("index.html"), "<div id=\"root\"></div>\n").expect("a template");
+    assert_eq!(
+        run("src/server.nova"),
+        "ssr: dist/index.html has no <div id=\"app\">\n"
+    );
+    std::fs::remove_file(dist.join("index.html")).expect("remove the template");
+    let err = run("src/server.nova");
+    assert!(
+        err.starts_with("ssr: cannot read dist/index.html: "),
+        "{err}"
+    );
+}
+
+/// SSG (spec 4.6 §6.4): `serve` given `--ssg <dir>` binds nothing, and
+/// writes a page for each static route, `/` over the template it read, the
+/// others in directories it makes, skipping the route with a parameter and
+/// the catch-all, and prints a line for each. Run again over its own output,
+/// it writes the same pages. A route path that names no file, for a piece
+/// of it that is `..` or empty, or holds `\`, NUL, `?` or `#`, is refused,
+/// with an `Err` naming it, after the pages before it, and nothing more is
+/// written; and `--ssg` with no directory is refused.
+#[test]
+fn ssr_ssg_writes_the_static_pages_and_refuses_a_bad_path() {
+    let (site, home) = ssr_site("ssg");
+    let dist = site.join("dist");
+    let run = |args: &[&str]| {
+        nova()
+            .current_dir(&site)
+            .env("NOVA_HOME", &home)
+            .env("NO_COLOR", "1")
+            .env("RUST_BACKTRACE", "0")
+            .arg("run")
+            .args(args)
+            .assert()
+    };
+    let read = |path: &str| std::fs::read_to_string(dist.join(path)).expect("a page");
+    let pages = [
+        (
+            "index.html",
+            ssr_view("<p>Home <a href=\"/about\" data-nova-link=\"\">About</a></p>"),
+        ),
+        ("about/index.html", ssr_view("<p>About</p>")),
+        ("docs/intro/index.html", ssr_view("<p>Intro</p>")),
+    ];
+    for round in 0..2 {
+        let a = run(&["src/server.nova", "--", "--ssg", "dist"]).success();
+        assert_eq!(
+            String::from_utf8_lossy(&a.get_output().stdout).replace("\r\n", "\n"),
+            "ssg: wrote dist/index.html\n\
+             ssg: wrote dist/about/index.html\n\
+             ssg: wrote dist/docs/intro/index.html\n\
+             stopped\n",
+            "round {round}"
+        );
+        for (path, view) in &pages {
+            assert!(
+                ssr_pages(SSR_TEMPLATE, view).contains(&read(path)),
+                "round {round}, {path}: {}",
+                read(path)
+            );
+        }
+    }
+    assert!(!dist.join("posts").exists());
+    assert_eq!(read("app.js"), "export default function init() {}\n");
+
+    // `refuse.nova`'s third route, `/a/../b` or the path its first argument
+    // names, each into a fresh directory, of which the two pages before it
+    // are then all there is.
+    let fresh = site.join("fresh");
+    let under = |dir: &std::path::Path| {
+        let mut out = Vec::new();
+        let mut dirs = vec![dir.to_path_buf()];
+        while let Some(d) = dirs.pop() {
+            for entry in std::fs::read_dir(&d).expect("a directory") {
+                let path = entry.expect("an entry").path();
+                let rel = path.strip_prefix(dir).expect("under the directory");
+                let names: Vec<String> = rel
+                    .components()
+                    .map(|c| c.as_os_str().to_string_lossy().into_owned())
+                    .collect();
+                out.push(names.join("/"));
+                if path.is_dir() {
+                    dirs.push(path);
+                }
+            }
+        }
+        out.sort();
+        out
+    };
+    for (name, path) in [
+        (None, "/a/../b"),
+        (Some("empty"), "/a//b"),
+        (Some("backslash"), "/a\\b"),
+        (Some("nul"), "/a\0b"),
+        (Some("query"), "/a?b"),
+        (Some("hash"), "/a#b"),
+    ] {
+        let _ = std::fs::remove_dir_all(&fresh);
+        std::fs::create_dir_all(&fresh).expect("a directory");
+        std::fs::write(fresh.join("index.html"), SSR_TEMPLATE).expect("a template");
+        let mut args = vec!["src/refuse.nova", "--"];
+        args.extend(name);
+        args.extend(["--ssg", "fresh"]);
+        let a = run(&args).code(3);
+        let o = a.get_output();
+        assert_eq!(
+            String::from_utf8_lossy(&o.stdout).replace("\r\n", "\n"),
+            "ssg: wrote fresh/index.html\nssg: wrote fresh/about/index.html\n",
+            "{path:?}"
+        );
+        assert_eq!(
+            String::from_utf8_lossy(&o.stderr).replace("\r\n", "\n"),
+            format!(
+                "ssg: the route path \"{path}\" names no file: a piece of it is empty, \".\" or \
+                 \"..\", or holds \"\\\", NUL, \"?\" or \"#\"\n"
+            ),
+            "{path:?}"
+        );
+        assert_eq!(
+            under(&fresh),
+            ["about", "about/index.html", "index.html"],
+            "{path:?}"
+        );
+        assert!(!site.join("b").exists(), "{path:?}");
+        let about = std::fs::read_to_string(fresh.join("about/index.html")).expect("a page");
+        assert_eq!(
+            about,
+            SSR_TEMPLATE.replacen(
+                "<div id=\"app\"></div>",
+                &format!("<div id=\"app\">{}</div>", ssr_view("<p>About</p>")),
+                1
+            ),
+            "{path:?}"
+        );
+    }
+
+    let a = run(&["src/server.nova", "--", "--ssg"]).code(3);
+    assert_eq!(
+        String::from_utf8_lossy(&a.get_output().stderr).replace("\r\n", "\n"),
+        "ssg: --ssg needs a directory\n"
+    );
+}
diff --git a/crates/nova-cli/tests/wasm_tests.rs b/crates/nova-cli/tests/wasm_tests.rs
index 78644941e60a55a77d10fce17d3b686d8f0b0b45..dce4fc1340e82fa66435f5b1355ac428a94fb9ae 100644
--- a/crates/nova-cli/tests/wasm_tests.rs
+++ b/crates/nova-cli/tests/wasm_tests.rs
@@ -229,6 +229,43 @@ fn dom_is_imported_by_name_and_only_on_wasm() {
     );
 }
 
+/// A native program imports `std/ui/ssr` by name; on WASM the import is
+/// E0092, and its names, never in any prelude, are unknown there without it,
+/// as natively (spec 4.6 §6.1, §8.4).
+#[test]
+fn ssr_is_imported_by_name_and_only_natively() {
+    let dir = fresh("import-ssr");
+    write(
+        &dir,
+        &[
+            (
+                "main.nova",
+                "import ui\nimport ui::ssr::{Page, page}\nfn f(p: Page) -> Int { 0 }\nfn main() { println(\"x\") }\n",
+            ),
+            (
+                "bare.nova",
+                "fn main() {\n    let p = page\n    println(\"x\")\n}\n",
+            ),
+        ],
+    );
+    nova_in(&dir, &["check", "main.nova"]).success();
+    let wasm = nova_in(&dir, &["check", "--target", "wasm", "main.nova"]).failure();
+    assert!(
+        stderr(&wasm)
+            .contains("error[E0092]: `ui::ssr` is a std module, which target wasm does not have"),
+        "{}",
+        stderr(&wasm)
+    );
+    for target in ["native", "wasm"] {
+        let bare = nova_in(&dir, &["check", "--target", target, "bare.nova"]).failure();
+        assert!(
+            stderr(&bare).contains("error[E0001]: cannot find `page` in this scope"),
+            "{target}: {}",
+            stderr(&bare)
+        );
+    }
+}
+
 /// A module of the program under a std module's name is E0093, a file
 /// beside a loose program's or a package's (spec 4.2 §3.2).
 #[test]
diff --git a/crates/nova-driver/tests/index_complete.rs b/crates/nova-driver/tests/index_complete.rs
index 99a59595ad33f97e1fa0b1226f5f27fa00b6a7ee..683ac6c0bfa7cdf218790391bae3755dd2d57677 100644
--- a/crates/nova-driver/tests/index_complete.rs
+++ b/crates/nova-driver/tests/index_complete.rs
@@ -272,8 +272,8 @@ fn every_name_in_the_corpora_is_indexed() {
         }
     }
     // 8 + 147 + 3 files of user code, 05-geometry's library's 2, and
-    // native std's 21.
-    assert_eq!(files, 181);
+    // native std's 22, `std/ui/ssr` among them.
+    assert_eq!(files, 182);
     failures.sort();
     failures.dedup();
     assert!(
diff --git a/crates/nova-resolver/src/lib.rs b/crates/nova-resolver/src/lib.rs
index 3eb7e1d17e0269ea55edfd4ca83e4e5ba0ba4204..3a84953ebe0050925377dc91e9636eae8711fce5 100644
--- a/crates/nova-resolver/src/lib.rs
+++ b/crates/nova-resolver/src/lib.rs
@@ -3646,6 +3646,42 @@ mod tests {
         );
     }
 
+    /// `std/ui/ssr` is native only (spec 4.6 §8.4): natively a program sees
+    /// its names through `import ui::ssr` alone, and on WASM, which does not
+    /// compile it, that import is E0092, while its names, which no module
+    /// sees without it, are not E0092 by themselves there.
+    #[test]
+    fn ssr_is_imported_by_name_and_only_natively() {
+        let native = resolve_for(
+            CompileTarget::Native,
+            "import ui::ssr::{page}\nfn main() { }\n",
+        );
+        assert!(native.diagnostics.is_empty(), "{:?}", native.diagnostics);
+        assert!(native
+            .definitions
+            .resolve_value(ModuleId(0), "page")
+            .is_some());
+        assert!(native
+            .definitions
+            .resolve_value(ModuleId(0), "serve")
+            .is_none());
+        let bare = resolve_for(CompileTarget::Native, "fn main() { }\n");
+        assert!(bare
+            .definitions
+            .resolve_value(ModuleId(0), "page")
+            .is_none());
+        let wasm = resolve_for(CompileTarget::Wasm, "import ui::ssr\nfn main() { }\n");
+        let codes: Vec<&str> = wasm.diagnostics.iter().map(|d| d.code.as_str()).collect();
+        assert_eq!(codes, ["E0092"], "{:?}", wasm.diagnostics);
+        assert_eq!(
+            wasm.diagnostics[0].message,
+            "`ui::ssr` is a std module, which target wasm does not have"
+        );
+        for name in ["page", "serve", "Site", "document"] {
+            assert_eq!(wasm.definitions.missing_module(name), None, "{name}");
+        }
+    }
+
     /// `std/ui` globs into no module: a program sees its names only through
     /// `import ui`, on either target (spec 4.3 §3).
     #[test]
diff --git a/tests/ssr/site/nova.toml b/tests/ssr/site/nova.toml
new file mode 100644
index 0000000000000000000000000000000000000000..5c5408b19bf8692e7fb0df5026eaeb447d01ab62
--- /dev/null
+++ b/tests/ssr/site/nova.toml
@@ -0,0 +1,5 @@
+[package]
+name = "site"
+version = "0.1.0"
+edition = "2026"
+description = "A project with a client and a server, which run_tests.rs serves and ssr_tests.rs builds with --ssr and --ssg"
diff --git a/tests/ssr/site/src/app.nova b/tests/ssr/site/src/app.nova
new file mode 100644
index 0000000000000000000000000000000000000000..c238793b9d573bc8047e9b69c9b03730ad20ca77
--- /dev/null
+++ b/tests/ssr/site/src/app.nova
@@ -0,0 +1,130 @@
+//! What the client, `main.nova`, and the server, `server.nova`, share (spec
+//! docs/superpowers/specs/2026-10-19-phase-4-6-ssr-hydration-and-ssg-design.md
+//! §1): the server's data, the routes, and the app, which shows the data's
+//! title and names above the route the location matches.
+
+import ui
+import ui::html::{div, h1, li, p, text, ul}
+import ui::router::{HistoryRouter, Route, link, use_params}
+
+/// The server's data.
+pub record Data { title: String, names: [String] }
+
+impl ToJson for Data {
+    fn to_json(self) -> JsonValue {
+        let mut names = [Null; self.names.len()]
+        let mut i = 0
+        while i < self.names.len() {
+            names[i] = self.names[i].to_json()
+            i = i + 1
+        }
+        let mut fields: Map<String, JsonValue> = Map::new()
+        fields.insert("title", self.title.to_json())
+        fields.insert("names", Array(names))
+        Object(fields)
+    }
+}
+
+// The string `v` holds, or `""` when it holds none.
+fn string_of(v: Option<JsonValue>) -> String {
+    match v {
+        Some(x) => {
+            match String::from_json(x) {
+                Ok(s) => s
+                Err(e) => ""
+            }
+        }
+        None => ""
+    }
+}
+
+impl FromJson for Data {
+    fn from_json(v: JsonValue) -> Result<Data, JsonError> {
+        let fields = match v {
+            Object(m) => m
+            _ => return Err(JsonError { msg: "expected an object", at: 0 })
+        }
+        let list = match fields.get("names") {
+            Some(x) => {
+                match x {
+                    Array(xs) => xs
+                    _ => return Err(JsonError { msg: "expected names", at: 0 })
+                }
+            }
+            None => return Err(JsonError { msg: "expected names", at: 0 })
+        }
+        let mut names = [""; list.len()]
+        let mut i = 0
+        while i < list.len() {
+            names[i] = string_of(Some(list[i]))
+            i = i + 1
+        }
+        Ok(Data { title: string_of(fields.get("title")), names: names })
+    }
+}
+
+/// The data the server renders with.
+pub fn data() -> Data { Data { title: "Names </script> & \u{2028} more", names: ["Ada", "Grace"] } }
+
+component Home() {
+    view {
+        p {
+            text("Home ")
+            link(to: "/about") { text("About") }
+        }
+    }
+}
+
+component About() {
+    view {
+        p { text("About") }
+    }
+}
+
+component Intro() {
+    view {
+        p { text("Intro") }
+    }
+}
+
+component Post() {
+    let id = use_params().get("id").unwrap_or("?")
+    view {
+        p { text("Post ${id}") }
+    }
+}
+
+component NotFound() {
+    view {
+        p { text("Nothing here") }
+    }
+}
+
+/// The routes: three static ones, one with a parameter, and the catch-all.
+pub fn routes() -> [Route] {
+    [
+        Route { path: "/", component: Home },
+        Route { path: "/about", component: About },
+        Route { path: "/docs/intro", component: Intro },
+        Route { path: "/posts/:id", component: Post },
+        Route { path: "*", component: NotFound },
+    ]
+}
+
+/// The app: the data's title and names, then the route.
+pub component App(data: Data) {
+    view {
+        div {
+            h1 { text(data.title) }
+            ul {
+                for_each(
+                    signal(data.names),
+                    |name: String| view {
+                        li { text(name) }
+                    },
+                )
+            }
+            HistoryRouter(routes())
+        }
+    }
+}
diff --git a/tests/ssr/site/src/elsewhere.nova b/tests/ssr/site/src/elsewhere.nova
new file mode 100644
index 0000000000000000000000000000000000000000..01261470f4c537f4f06098aea6c8e96ffb0c62b6
--- /dev/null
+++ b/tests/ssr/site/src/elsewhere.nova
@@ -0,0 +1,18 @@
+//! A server whose address cannot be bound, which `serve` reports.
+
+import app::{App, data, routes}
+import ui
+import ui::ssr::{Page, Site, page, serve}
+
+fn render(location: String) -> Page { page(location, || App(data())) }
+
+async fn main() {
+    let site = Site { address: "127.0.0.1:99999", dist: "dist", routes: routes(), render: render }
+    match serve(site).await {
+        Ok(_) => println("stopped")
+        Err(e) => {
+            eprintln(e.message)
+            exit(3)
+        }
+    }
+}
diff --git a/tests/ssr/site/src/main.nova b/tests/ssr/site/src/main.nova
new file mode 100644
index 0000000000000000000000000000000000000000..0152fd1207b107bf6fb3a198068484e7f2a96452
--- /dev/null
+++ b/tests/ssr/site/src/main.nova
@@ -0,0 +1,7 @@
+//! The client: it hydrates the page the server rendered, with the server's
+//! data, or with no title and no names on a page with none.
+
+import app::{App, Data}
+import ui
+
+fn main() { hydrate_with("#app", Data { title: "", names: [] }, App) }
diff --git a/tests/ssr/site/src/refuse.nova b/tests/ssr/site/src/refuse.nova
new file mode 100644
index 0000000000000000000000000000000000000000..bbe5bb56594ffe84774894518e40b7af54477fac
--- /dev/null
+++ b/tests/ssr/site/src/refuse.nova
@@ -0,0 +1,38 @@
+//! A server whose third static route names no file, which `--ssg` refuses
+//! after it has written the first two, writing nothing more. The route's
+//! path is `/a/../b`, or, when the first argument names one, another such
+//! path: `empty`, `/a//b`; `backslash`, `/a\b`; `nul`, `/a`, NUL and `b`;
+//! `query`, `/a?b`; `hash`, `/a#b`. An argument cannot hold NUL, so each is
+//! named.
+
+import app::{App, data, routes}
+import ui
+import ui::router::{Route}
+import ui::ssr::{Page, Site, page, serve}
+
+fn render(location: String) -> Page { page(location, || App(data())) }
+
+// The path `name` names, or `/a/../b` when it names none, as when `nova
+// build --ssg` runs this with `--ssg` first.
+fn bad_path(name: String) -> String {
+    if name == "empty" { return "/a//b" }
+    if name == "backslash" { return "/a\\b" }
+    if name == "nul" { return "/a\0b" }
+    if name == "query" { return "/a?b" }
+    if name == "hash" { return "/a#b" }
+    "/a/../b"
+}
+
+async fn main() {
+    let all = routes()
+    let path = bad_path(args().get(1).unwrap_or(""))
+    let bad = [all[0], all[1], Route { path: path, component: all[2].component }, all[2]]
+    let site = Site { address: "127.0.0.1:0", dist: "dist", routes: bad, render: render }
+    match serve(site).await {
+        Ok(_) => println("stopped")
+        Err(e) => {
+            eprintln(e.message)
+            exit(3)
+        }
+    }
+}
diff --git a/tests/ssr/site/src/server.nova b/tests/ssr/site/src/server.nova
new file mode 100644
index 0000000000000000000000000000000000000000..8a9eaca29afbc0cd627be8c190fcbad82db686ed
--- /dev/null
+++ b/tests/ssr/site/src/server.nova
@@ -0,0 +1,21 @@
+//! The server: it serves `dist/` on a port the system chooses, rendering
+//! each page with the data; given `--ssg <dir>`, it writes the static pages
+//! into `<dir>` instead. It prints an error and exits with 3 when `serve`
+//! fails, and prints `stopped` when a signal stops it.
+
+import app::{App, data, routes}
+import ui
+import ui::ssr::{Page, Site, page_with, serve}
+
+fn render(location: String) -> Page { page_with(location, data(), App) }
+
+async fn main() {
+    let site = Site { address: "127.0.0.1:0", dist: "dist", routes: routes(), render: render }
+    match serve(site).await {
+        Ok(_) => println("stopped")
+        Err(e) => {
+            eprintln(e.message)
+            exit(3)
+        }
+    }
+}
````

**Patch `t3-code`:**

<!-- patch t3-code -->

````diff
diff --git a/crates/nova-doc/src/model.rs b/crates/nova-doc/src/model.rs
index 154b08c6e195e69b6668102759e4008b281971c8..4ffdc4826a498540fc3179dd4abfa9088d573053 100644
--- a/crates/nova-doc/src/model.rs
+++ b/crates/nova-doc/src/model.rs
@@ -1405,7 +1405,8 @@ mod tests {
                 "time",
                 "ui",
                 "ui/html",
-                "ui/router"
+                "ui/router",
+                "ui/ssr"
             ]
         );
         let core = module(&d.site, "std/core/index.html");
diff --git a/crates/nova-resolver/src/lib.rs b/crates/nova-resolver/src/lib.rs
index 3a84953ebe0050925377dc91e9636eae8711fce5..2482334084add76e3ba95a1b40977125eadeaea6 100644
--- a/crates/nova-resolver/src/lib.rs
+++ b/crates/nova-resolver/src/lib.rs
@@ -2066,12 +2066,15 @@ pub fn resolve_program_for(
 /// Each public name of each std module `target` does not compile, mapped to
 /// that module's name; the first module to export a name keeps it
 /// (spec 4.1 §3.2). The modules are parsed and their items collected as
-/// for a build, into scopes that are then dropped.
+/// for a build, into scopes that are then dropped. An imported module's
+/// names are left out, as `std/ui/ssr`'s: no module sees them without its
+/// import, which is E0092 itself on such a target (spec 4.6 §8.4).
 fn missing_names(target: CompileTarget) -> FxHashMap<String, &'static str> {
     let compiled = target.std_modules();
     let mut missing: FxHashMap<String, &'static str> = FxHashMap::default();
     for &(name, src) in &STD_MODULES {
-        if compiled.iter().any(|&(n, _)| n == name) {
+        if compiled.iter().any(|&(n, _)| n == name) || IMPORTED_STD.iter().any(|&(_, m)| m == name)
+        {
             continue;
         }
         let (Some(file), _) = std_module(src, FileId::DUMMY) else {
@@ -2104,9 +2107,9 @@ fn missing_names(target: CompileTarget) -> FxHashMap<String, &'static str> {
 }
 
 /// The embedded standard-library modules, compiled as implicit modules and
-/// glob-imported into every user module, but for `std/ui`, `std/ui/html`
-/// and `std/ui/router`, which a program imports, and `std/ui`'s backend,
-/// which only std sees (spec 4.3 §3, §7.3; 4.5 §3). Order is significant
+/// glob-imported into every user module, but for `std/ui`, `std/ui/html`,
+/// `std/ui/router` and `std/ui/ssr`, which a program imports, and `std/ui`'s
+/// backend, which only std sees (spec 4.3 §3, §7.3; 4.5 §3; 4.6 §8.4). Order is significant
 /// only in that it fixes module indices; user modules always come first,
 /// then these in the order listed here — `std/core` stays first so its
 /// module index is unchanged from when it was the only embedded module.
@@ -2116,7 +2119,7 @@ fn missing_names(target: CompileTarget) -> FxHashMap<String, &'static str> {
 /// self-contained executable and `cargo package` keeps the files. Each name
 /// is `$std.*`, not a valid identifier, so it can never collide with a user
 /// module name or be named in an `import`.
-pub const STD_MODULES: [(&str, &str); 20] = [
+pub const STD_MODULES: [(&str, &str); 21] = [
     ("$std.core", nova_std::CORE),
     ("$std.bytes", nova_std::BYTES),
     ("$std.io", nova_std::IO),
@@ -2137,6 +2140,7 @@ pub const STD_MODULES: [(&str, &str); 20] = [
     ("$std.ui.html", nova_std::UI_HTML),
     ("$std.ui.backend", nova_std::UI_BACKEND_NATIVE),
     ("$std.ui.router", nova_std::UI_ROUTER),
+    ("$std.ui.ssr", nova_std::UI_SSR),
 ];
 
 /// What a program is compiled for (spec
@@ -2201,12 +2205,14 @@ pub const WASM_STD_MODULES: [(&str, &str); 16] = [
 /// The std modules a program imports by name instead of finding in its
 /// prelude, by the name an `import` writes, and the module's own name
 /// (spec `docs/superpowers/specs/2026-10-16-phase-4-2-std-dom-and-the-shim-design.md`
-/// §3.2; 4.5 §3).
-pub const IMPORTED_STD: [(&str, &str); 4] = [
+/// §3.2; 4.5 §3). `dom` is WASM's alone and `ui::ssr` native's alone (4.6
+/// §8.4): on the other target the import is E0092.
+pub const IMPORTED_STD: [(&str, &str); 5] = [
     ("dom", "$std.dom"),
     ("ui", "$std.ui"),
     ("ui::html", "$std.ui.html"),
     ("ui::router", "$std.ui.router"),
+    ("ui::ssr", "$std.ui.ssr"),
 ];
 
 /// The names a project's module or a dependency may not take: those of
diff --git a/std/lib.rs b/std/lib.rs
index 0e0913ffbf25cc66599f649bd70662b520b5e91e..c8f00ba96dc12e835c685ea2f9813f3f0afe0a62 100644
--- a/std/lib.rs
+++ b/std/lib.rs
@@ -2,7 +2,7 @@
 //! build time so the compiler stays one self-contained executable.
 //!
 //! Every `include_str!` path stays inside this package, so `cargo package`
-//! carries all 24 files, and its verification step builds the packaged copy
+//! carries all 25 files, and its verification step builds the packaged copy
 //! on its own (spec
 //! `docs/superpowers/specs/2026-10-07-phase-3-0-foundations-design.md` §3).
 //! `nova-resolver` gives each module its `$std.*` name and its place.
@@ -53,6 +53,11 @@ pub const UI: &str = include_str!("ui/lib.nova");
 pub const UI_HTML: &str = include_str!("ui/html/lib.nova");
 /// `std/ui/router/lib.nova`: routing by the URL, which a program imports.
 pub const UI_ROUTER: &str = include_str!("ui/router/lib.nova");
+/// `std/ui/ssr/lib.nova`: server-side rendering, which a native program
+/// imports (spec
+/// `docs/superpowers/specs/2026-10-19-phase-4-6-ssr-hydration-and-ssg-design.md`
+/// §6).
+pub const UI_SSR: &str = include_str!("ui/ssr/lib.nova");
 /// `std/ui/backend/native.nova`, `std/ui`'s backend natively. Not in
 /// [`ALL`]: only std's modules see it (spec
 /// `docs/superpowers/specs/2026-10-17-phase-4-3-std-ui-reactivity-and-views-design.md`
@@ -70,7 +75,7 @@ pub const RT: &str = include_str!("rt/lib.nova");
 
 /// Every documented module, by its directory under `std/`: all but
 /// [`RT`], [`DOM`] and `std/ui`'s backend.
-pub const ALL: [(&str, &str); 20] = [
+pub const ALL: [(&str, &str); 21] = [
     ("bytes", BYTES),
     ("collections", COLLECTIONS),
     ("core", CORE),
@@ -91,6 +96,7 @@ pub const ALL: [(&str, &str); 20] = [
     ("ui", UI),
     ("ui/html", UI_HTML),
     ("ui/router", UI_ROUTER),
+    ("ui/ssr", UI_SSR),
 ];
 
 #[cfg(test)]
@@ -140,7 +146,7 @@ mod tests {
             )
             .collect();
         assert_eq!(embedded, on_disk);
-        assert_eq!(on_disk.len(), 24);
+        assert_eq!(on_disk.len(), 25);
     }
 
     /// Each entry carries its own file, not a neighbour's.
diff --git a/std/ui/ssr/lib.nova b/std/ui/ssr/lib.nova
new file mode 100644
index 0000000000000000000000000000000000000000..2156d6ea8fce14547a9831fc613859a8bfc5c239
--- /dev/null
+++ b/std/ui/ssr/lib.nova
@@ -0,0 +1,700 @@
+//! Server-side rendering: a page whose view is already written as HTML,
+//! which the client's `hydrate` then adopts, and a server that answers with
+//! such pages; or, given `--ssg <dir>`, writes them ahead of time.
+//!
+//! It is native only: a program imports it with `import ui::ssr`, or
+//! `import ui::ssr::{…}`, and on WASM that import is E0092, as `import dom`
+//! is natively. A project with a server has two entries that share their
+//! components through a module such as `src/app.nova`: `src/main.nova`, the
+//! client, which calls `hydrate("#app", App)` or `hydrate_with`, and
+//! `src/server.nova`, which calls `serve`. `nova build --ssr` builds both:
+//! the client into `dist/`, the server into `target/debug/<name>-server`.
+//! `nova build --ssg` then runs the server once with `--ssg dist`, which
+//! writes a page for each static route.
+//!
+//! `page` renders the view at a location, `page_with` with the server's data
+//! beside it, and `document` puts a page into the built `index.html`, in its
+//! `<div id="app">`:
+//!
+//! ```
+//! import ui
+//! import ui::html::{p, text}
+//! import ui::router::{HistoryRouter, Route}
+//! import ui::ssr::{document, page}
+//!
+//! component Home() { view { p { text("Home") } } }
+//!
+//! component About() { view { p { text("About") } } }
+//!
+//! fn main() {
+//!     let routes = [
+//!         Route { path: "/", component: Home },
+//!         Route { path: "/about", component: About },
+//!     ]
+//!     let template = "<body><div id=\"app\"></div><script src=\"/app.js\"></script></body>"
+//!     let html = document(template, page("/about", || HistoryRouter(routes)))
+//!     assert_eq(
+//!         html,
+//!         "<body><div id=\"app\"><!--[--><p>About</p><!--]--></div><script src=\"/app.js\"></script></body>",
+//!     )
+//! }
+//! ```
+
+// The design (spec
+// docs/superpowers/specs/2026-10-19-phase-4-6-ssr-hydration-and-ssg-design.md
+// §6). The low level is three functions of strings: `page` and `page_with`
+// render a view at a location through `std/ui/router`'s `with_location`,
+// and `document` fills the template. `serve` is `std/http`'s `Server` with
+// one fallback that answers every `GET`: a file the build's manifest lists,
+// a 404 for any other name with a `.`, or a page. A file is read when it is
+// asked for, through the builtins `std/fs` is written on, since a handler
+// cannot await; no path outside the manifest is ever read, so there is no
+// traversal to refuse; and a manifest's path that would leave `dist`, with
+// a piece that is empty, `.` or `..`, is never served either. Given
+// `--ssg <dir>`, `serve` writes the static routes' pages instead of
+// binding. Both read the template from an `index.html` that SSG may already
+// have filled with the root's page, and a project's own page may hold a
+// placeholder there, so `document`, which both fill pages with, empties the
+// root first (`emptied`).
+
+import ui
+import ui::router::{Route, found, static_paths, with_location}
+
+/// A page a server renders: the view's HTML, and the server's data for
+/// `hydrate_with`, if any. `document` puts it into the page's template.
+pub record Page {
+    /// The view, as `render_hydratable` writes it.
+    pub body: String,
+    /// The data as JSON, made safe for a `script` element, or `None`.
+    pub data: Option<String>,
+}
+
+/// The page at `location`, a path and its query such as `/posts/1?tab=2`:
+/// `f`'s view rendered by `render_hydratable` with a router made inside it
+/// starting at `location`, as `with_location` makes it, and no data. The
+/// client hydrates it with `hydrate("#app", f)`.
+///
+/// ```
+/// import ui
+/// import ui::html::{p, text}
+/// import ui::router::{HistoryRouter, Route, use_params}
+/// import ui::ssr::{page}
+///
+/// component Post() {
+///     let id = use_params().get("id").unwrap_or("?")
+///     view { p { text("Post ${id}") } }
+/// }
+///
+/// fn main() {
+///     let p = page("/posts/7", || HistoryRouter([Route { path: "/posts/:id", component: Post }]))
+///     assert_eq(p.body, "<!--[--><p>Post 7</p><!--]-->")
+///     assert(p.data.is_none(), "a page has no data")
+/// }
+/// ```
+pub fn page(location: String, f: fn() -> View) -> Page {
+    Page { body: with_location(location, || render_hydratable(f)), data: None }
+}
+
+/// The page at `location` for `data`: `f(data)`'s view rendered as `page`
+/// renders a view, and `data` written as JSON, as `stringify` writes
+/// `data.to_json()`, then made safe for a `script` element: `<`, `>` and `&`
+/// are written as the escapes `\u003c`, `\u003e` and `\u0026`, and U+2028
+/// and U+2029 as `\u2028` and `\u2029`. Those characters occur only inside
+/// JSON's strings, so the text parses back to the same value, and no
+/// `</script>` in the data can end its element. The client hydrates it with
+/// `hydrate_with("#app", fallback, f)`, which reads the data back with
+/// `T::from_json`.
+///
+/// ```
+/// import ui
+/// import ui::html::{p, text}
+/// import ui::ssr::{page_with}
+///
+/// fn greeting(name: String) -> View { p([], [text("Hello, ${name}")]) }
+///
+/// fn main() {
+///     let p = page_with("/", "Ada & <Grace>", greeting)
+///     assert_eq(p.body, "<p>Hello, Ada &amp; &lt;Grace&gt;</p>")
+///     assert_eq(p.data.unwrap(), "\"Ada \\u0026 \\u003cGrace\\u003e\"")
+/// }
+/// ```
+pub fn page_with<T: ToJson>(location: String, data: T, f: fn(T) -> View) -> Page {
+    let body = with_location(location, || render_hydratable(|| f(data)))
+    Page { body: body, data: Some(script_safe(stringify(data.to_json()))) }
+}
+
+/// `template`, a page's HTML such as the built `dist/index.html`, with its
+/// first `<div id="app">` holding `page`'s body, and, when the page has
+/// data, followed by `<script type="application/json" id="nova-data">`
+/// holding the data, just after the root, where `hydrate_with` reads it. A
+/// root that already holds something, whitespace, a placeholder, or a page
+/// SSG wrote, is emptied first, and a data element after it, alone or past
+/// whitespace, dropped, as `serve` and SSG empty it. A root that no
+/// `</div>` closes keeps what follows it: an empty root goes before it, and
+/// the rest stays; so does a data element with no `</script>`. Nothing else
+/// of the template changes: its head, title and scripts serve the server's
+/// pages and the client alike.
+///
+/// It panics with `ssr: the page has no <div id="app">` when the template
+/// has none, written just so.
+///
+/// ```
+/// import ui
+/// import ui::ssr::{Page, document}
+///
+/// let template = "<title>Names</title><div id=\"app\"></div>"
+/// let html = document(template, Page { body: "<p>Ada</p>", data: Some("[\"Ada\"]") })
+/// assert_eq(
+///     html,
+///     "<title>Names</title><div id=\"app\"><p>Ada</p></div><script type=\"application/json\" id=\"nova-data\">[\"Ada\"]</script>",
+/// )
+/// assert_eq(document("<div id=\"app\"></div>", Page { body: "x", data: None }), "<div id=\"app\">x</div>")
+/// // A placeholder in the root gives way to the page.
+/// let waiting = "<div id=\"app\">\n  <div class=\"spinner\"></div>\n</div><script src=\"/app.js\"></script>"
+/// assert_eq(
+///     document(waiting, Page { body: "x", data: None }),
+///     "<div id=\"app\">x</div><script src=\"/app.js\"></script>",
+/// )
+/// ```
+pub fn document(template: String, page: Page) -> String {
+    let empty = match emptied(template) {
+        Some(t) => t
+        None => panic("ssr: the page has no <div id=\"app\">")
+    }
+    let root = app_root()
+    let at = empty.index_of(root).unwrap_or(0)
+    let data = match page.data {
+        Some(d) => "<script type=\"application/json\" id=\"nova-data\">${d}</script>"
+        None => ""
+    }
+    let before = empty.slice(0, at)
+    let after = empty.slice(at + root.len(), empty.len())
+    "${before}<div id=\"app\">${page.body}</div>${data}${after}"
+}
+
+/// What `serve` serves: an address, the built site, the app's routes, and
+/// the page at each location.
+pub record Site {
+    /// The address to listen on, `"host:port"`, such as `"127.0.0.1:3000"`;
+    /// port 0 lets the system choose a free port.
+    pub address: String,
+    /// The directory `nova build --ssr` wrote, such as `"dist"`: its
+    /// `index.html` is every page's template, and its `manifest.json` lists
+    /// the files served.
+    pub dist: String,
+    /// The app's routes: a page's status is `200` when `found` says a route
+    /// serves its location, and SSG writes their `static_paths`.
+    pub routes: [Route],
+    /// The page at a location, a path and its query, such as one of `page`
+    /// or `page_with`.
+    pub render: fn(String) -> Page,
+}
+
+/// Serves `site` over HTTP until the process is asked to stop, then returns
+/// `Ok(())`; or, when the program's arguments hold `--ssg` and a directory,
+/// writes the site's static pages into that directory, and returns.
+///
+/// **Serving.** It first reads `<dist>/index.html`, the template, and
+/// `<dist>/manifest.json`, whose `files` are what it may serve, and returns
+/// an `IoError` naming the file when either cannot be read, when the
+/// manifest has no `files`, or when the template has no `<div id="app">`.
+/// An `index.html` whose root holds a page, as the one SSG writes for `/`
+/// does, or a placeholder, is a template too: its root is emptied, and the
+/// data after it dropped, as `document` empties it; a root that no `</div>`
+/// closes keeps what follows it, after an empty root. A manifest's path with
+/// a piece that is empty, `.` or `..`, or that holds `\` or NUL, which
+/// could name a file outside `dist`, is never served. It binds `address`,
+/// returning an `IoError` naming it when that fails, then prints
+/// `ssr: serving http://<host>:<port>/`, the port the one bound, and flushes
+/// it. Each `GET` is answered by its path: the target before any `?`,
+/// percent-decoded, less its leading `/`.
+///
+/// - A path that the manifest lists gets `200` and the file, read then, with
+///   the `Content-Type` its extension gives (`html`, `js` and `mjs`, `wasm`,
+///   `css`, `json`, `svg`, `png`, `jpg`, `gif`, `webp`, `ico`, `txt` and
+///   `woff2`; any other is `application/octet-stream`) and
+///   `X-Content-Type-Options: nosniff`. One that cannot be read gets `404`.
+/// - Any other path whose last piece holds a `.` gets `404 not found`.
+/// - Any other path is a page: `document(template, render(target))`, the
+///   target with its query, as `text/html; charset=utf-8`, with `200` when
+///   `found(routes, target)`, and `404` otherwise.
+///
+/// No file the manifest does not list is ever read, so a file added after
+/// the build is served only once the server starts again. Any method but
+/// `GET` gets `404`. It is built on `std/http`'s `Server`, so connections,
+/// limits and the stop on SIGTERM or SIGINT are as `Server::listen` says.
+///
+/// **SSG.** With `--ssg <dir>` in the program's arguments it binds nothing.
+/// It reads `<dir>/index.html` as the template, as serving does, so SSG can
+/// run again over its own output, then for each of `static_paths(routes)`
+/// writes `document(template, render(path))`: `/` to `<dir>/index.html`,
+/// which it overwrites, and any other path to
+/// `<dir>/<its pieces>/index.html`, making the directories. It prints
+/// `ssg: wrote <file>` for each. A path with a piece that is empty, `.` or
+/// `..`, or that holds `\`, NUL, `?` or `#`, is refused: it returns an
+/// `IoError` naming the path, and writes nothing more. `nova build --ssg`
+/// runs the server this way.
+///
+/// A server's `main`, for a project whose `src/app.nova` holds `App`,
+/// `Data` and `routes`:
+///
+/// ```ignore
+/// import ui
+/// import ui::ssr::{Page, Site, page_with, serve}
+/// import app::{App, Data, routes}
+///
+/// fn render(location: String) -> Page {
+///     page_with(location, Data { names: ["Ada", "Grace"] }, App)
+/// }
+///
+/// async fn main() {
+///     let site = Site { address: "127.0.0.1:3000", dist: "dist", routes: routes(), render: render }
+///     match serve(site).await {
+///         Ok(_) => {}
+///         Err(e) => {
+///             eprintln(e.message)
+///             exit(1)
+///         }
+///     }
+/// }
+/// ```
+pub async fn serve(site: Site) -> Result<(), IoError> {
+    match ssg_dir(args()) {
+        Some(dir) => return generate(site, dir).await
+        None => {}
+    }
+    let index = "${site.dist}/index.html"
+    let template = match read_to_string(index).await {
+        Ok(t) => {
+            match emptied(t) {
+                Some(t) => t
+                None => return Err(rootless("ssr", index))
+            }
+        }
+        Err(e) => return Err(unreadable("ssr", index, e))
+    }
+    let listing = "${site.dist}/manifest.json"
+    let text = match read_to_string(listing).await {
+        Ok(t) => t
+        Err(e) => return Err(unreadable("ssr", listing, e))
+    }
+    let files = match listed(text) {
+        Some(files) => files
+        None => {
+            return Err(
+                IoError { kind: InvalidData, message: "ssr: ${listing} has no \"files\" object" },
+            )
+        }
+    }
+    // As `Server::listen` does before its `bind`: from here a signal stops
+    // the server gracefully, even before `listen_on` runs.
+    let _ = shutdown_requested()
+    let l = match bind(site.address) {
+        Ok(l) => l
+        Err(e) => {
+            return Err(
+                IoError { kind: e.kind, message: "ssr: cannot bind ${site.address}: ${e.message}" },
+            )
+        }
+    }
+    let port = match l.local_port() {
+        Ok(p) => p
+        Err(e) => {
+            let _ = l.close().await
+            return Err(e)
+        }
+    }
+    println("ssr: serving http://${host_of(site.address)}:${port}/")
+    let _ = stdout().flush().await
+    let served = Served {
+        dist: site.dist,
+        files: files,
+        template: template,
+        routes: site.routes,
+        render: site.render,
+    }
+    Server::new().fallback(|r: Request| answer(served, r)).listen_on(l).await
+}
+
+// What each request is answered from: the site, and what `serve` read when
+// it started.
+record Served {
+    dist: String,
+    files: Map<String, JsonValue>,
+    template: String,
+    routes: [Route],
+    render: fn(String) -> Page,
+}
+
+// The response to a `GET` of `req.path` (spec §6.3). `Server` sends any
+// other method `not_found` before it asks.
+fn answer(s: Served, req: Request) -> Response {
+    let target = req.path
+    let raw = match target.index_of("?") {
+        Some(at) => target.slice(0, at)
+        None => target
+    }
+    let decoded_path = decoded(raw)
+    let path = if decoded_path.starts_with("/") {
+        decoded_path.slice(1, decoded_path.len())
+    } else {
+        decoded_path
+    }
+    if s.files.get(path).is_some() { return file(s.dist, path) }
+    let pieces = path.split("/")
+    if pieces[pieces.len() - 1].contains(".") { return Response::not_found() }
+    let render = s.render
+    let body = bytes_from_string(document(s.template, render(target)))
+    let mut h: Map<String, String> = Map::new()
+    h.insert("content-length", "${body.len()}")
+    h.insert("content-type", "text/html; charset=utf-8")
+    let status = if found(s.routes, target) { 200 } else { 404 }
+    Response { status: status, headers: h, body: body }
+}
+
+// `200` and the file at `path` under `dist`, read now, or `404` when it
+// cannot be read. The read is `std/fs`'s `read` without its `async`, which
+// a handler cannot await: the builtin, then its slot, with nothing between.
+fn file(dist: String, path: String) -> Response {
+    if fs_read("${dist}/${path}") != 0 { return Response::not_found() }
+    let body = fs_take_bytes()
+    let mut h: Map<String, String> = Map::new()
+    h.insert("content-length", "${body.len()}")
+    h.insert("content-type", content_type(path))
+    h.insert("x-content-type-options", "nosniff")
+    Response { status: 200, headers: h, body: body }
+}
+
+// The `Content-Type` of a file named `path`, by its extension in any case,
+// as `nova dev` gives it, with `webp` too.
+fn content_type(path: String) -> String {
+    let pieces = path.split("/")
+    let name = pieces[pieces.len() - 1]
+    let parts = name.split(".")
+    let ext = if parts.len() < 2 { "" } else { parts[parts.len() - 1].to_lower() }
+    if ext == "html" { return "text/html; charset=utf-8" }
+    if ext == "js" || ext == "mjs" { return "text/javascript; charset=utf-8" }
+    if ext == "wasm" { return "application/wasm" }
+    if ext == "css" { return "text/css; charset=utf-8" }
+    if ext == "json" { return "application/json" }
+    if ext == "svg" { return "image/svg+xml" }
+    if ext == "png" { return "image/png" }
+    if ext == "jpg" { return "image/jpeg" }
+    if ext == "gif" { return "image/gif" }
+    if ext == "webp" { return "image/webp" }
+    if ext == "ico" { return "image/x-icon" }
+    if ext == "txt" { return "text/plain; charset=utf-8" }
+    if ext == "woff2" { return "font/woff2" }
+    "application/octet-stream"
+}
+
+// SSG (spec §6.4): the static routes' pages written into `dir`, each
+// rendered into `dir/index.html` as it was before the first is written.
+async fn generate(site: Site, dir: String) -> Result<(), IoError> {
+    if dir == "" {
+        return Err(IoError { kind: InvalidData, message: "ssg: --ssg needs a directory" })
+    }
+    let index = "${dir}/index.html"
+    let template = match read_to_string(index).await {
+        Ok(t) => {
+            match emptied(t) {
+                Some(t) => t
+                None => return Err(rootless("ssg", index))
+            }
+        }
+        Err(e) => return Err(unreadable("ssg", index, e))
+    }
+    let render = site.render
+    let paths = static_paths(site.routes)
+    let mut i = 0
+    while i < paths.len() {
+        let path = paths[i]
+        let pieces = match file_pieces(path) {
+            Some(pieces) => pieces
+            None => {
+                return Err(
+                    IoError {
+                        kind: InvalidData,
+                        message: "ssg: the route path \"${path}\" names no file: a piece of it is empty, \".\" or \"..\", or holds \"\\\", NUL, \"?\" or \"#\"",
+                    },
+                )
+            }
+        }
+        let folder = if pieces.len() == 0 { dir } else { "${dir}/${"/".join(pieces)}" }
+        match create_dir_all(folder).await {
+            Ok(_) => {}
+            Err(e) => {
+                return Err(
+                    IoError { kind: e.kind, message: "ssg: cannot make ${folder}: ${e.message}" },
+                )
+            }
+        }
+        let file = "${folder}/index.html"
+        match write_string(file, document(template, render(path))).await {
+            Ok(_) => {}
+            Err(e) => {
+                return Err(
+                    IoError { kind: e.kind, message: "ssg: cannot write ${file}: ${e.message}" },
+                )
+            }
+        }
+        println("ssg: wrote ${file}")
+        i = i + 1
+    }
+    Ok(())
+}
+
+// The pieces of the directory a static route's page goes into, below SSG's
+// directory: `path` less one leading `/`, split at `/`, so none for `/`; or
+// `None` when a piece is empty, `.` or `..`, or holds `\`, NUL, `?` or `#`.
+fn file_pieces(path: String) -> Option<[String]> {
+    let rest = if path.starts_with("/") { path.slice(1, path.len()) } else { path }
+    if rest == "" {
+        let none: [String] = []
+        return Some(none)
+    }
+    let pieces = rest.split("/")
+    let mut i = 0
+    while i < pieces.len() {
+        let p = pieces[i]
+        if !plain_piece(p) || p.contains("?") || p.contains("#") { return None }
+        i = i + 1
+    }
+    Some(pieces)
+}
+
+// The directory after `--ssg` in the program's arguments, after its name:
+// `None` when there is no `--ssg`, and `""` when nothing follows it.
+fn ssg_dir(argv: Vec<String>) -> Option<String> {
+    let mut i = 1
+    while i < argv.len() {
+        if argv.get(i).unwrap_or("") == "--ssg" { return Some(argv.get(i + 1).unwrap_or("")) }
+        i = i + 1
+    }
+    None
+}
+
+// The manifest's `files`, by path, from its JSON text, less those `inside`
+// leaves out; or `None` when it has no `files` object.
+fn listed(text: String) -> Option<Map<String, JsonValue>> {
+    match parse(text) {
+        Ok(v) => {
+            match v {
+                Object(m) => {
+                    match m.get("files") {
+                        Some(f) => {
+                            match f {
+                                Object(files) => Some(inside(files))
+                                _ => None
+                            }
+                        }
+                        None => None
+                    }
+                }
+                _ => None
+            }
+        }
+        Err(_) => None
+    }
+}
+
+// `files` less each path that could name a file outside the directory it is
+// read from: one with a piece that is empty, `.` or `..`, or that holds `\`
+// or NUL, as `file_pieces` refuses a route's path. A hand-written manifest
+// can list one; it is never served.
+fn inside(files: Map<String, JsonValue>) -> Map<String, JsonValue> {
+    let mut kept = files
+    let names = files.keys()
+    let mut i = 0
+    while i < names.len() {
+        if !plain_path(names[i]) { let _ = kept.remove(names[i]) }
+        i = i + 1
+    }
+    kept
+}
+
+// Whether each piece of `path`, between its `/`s, is plain (`plain_piece`).
+fn plain_path(path: String) -> Bool {
+    let pieces = path.split("/")
+    let mut i = 0
+    while i < pieces.len() {
+        if !plain_piece(pieces[i]) { return false }
+        i = i + 1
+    }
+    true
+}
+
+// Whether `piece` names a file or a directory below the one it is in: it is
+// not empty, `.` or `..`, and holds no `\` or NUL.
+fn plain_piece(piece: String) -> Bool {
+    let special = piece == "" || piece == "." || piece == ".."
+    !special && !piece.contains("\\") && !piece.contains("\0")
+}
+
+// The root a page's view goes into, written just so.
+fn app_root() -> String { "<div id=\"app\"></div>" }
+
+// The template in `html`, a page's HTML: `html` with its first
+// `<div id="app">` emptied, written `<div id="app"></div>`, and the data's
+// `script` just after it removed. The built `index.html`'s root is empty
+// already; a project's own page may hold whitespace or a placeholder there;
+// and the `index.html` SSG wrote for `/` holds a view and its data, so that
+// a server started after `nova build --ssg`, or SSG run again, finds the
+// page it was built with. A view's HTML holds no `<div` or `</div>` but its
+// own elements', since text and attribute values are escaped, so the root
+// ends at the `</div>` that balances it; a comment of the template holding
+// one could end it early. The data's `script` goes when it follows the
+// root, alone or past whitespace, as the client's `hydrate_with` would read
+// it there, and has its `</script>`; the whitespace stays. A data `script`
+// with no `</script>` stays as it is, since nothing tells where its text
+// ends, and a root no `</div>` balances keeps what follows it, after an
+// empty root. `None` when there is no `<div id="app">`.
+fn emptied(html: String) -> Option<String> {
+    let open = "<div id=\"app\">"
+    let at = match html.index_of(open) {
+        Some(at) => at
+        None => return None
+    }
+    let cs = html.chars()
+    let mut depth = 1
+    let mut i = at + open.len()
+    while depth > 0 && i < cs.len() {
+        if matches_at(cs, i, "</div>") {
+            depth = depth - 1
+            if depth > 0 { i = i + 6 }
+        } else {
+            if matches_at(cs, i, "<div") && i + 4 < cs.len() && ends_name(cs[i + 4]) {
+                depth = depth + 1
+            }
+            i = i + 1
+        }
+    }
+    let start = at + open.len()
+    if depth > 0 {
+        return Some("${html.slice(0, at)}${app_root()}${html.slice(start, html.len())}")
+    }
+    let end = i + 6
+    let mut rest = html.slice(end, html.len())
+    let mut k = end
+    while k < cs.len() && html_space(cs[k]) { k = k + 1 }
+    if matches_at(cs, k, "<script type=\"application/json\" id=\"nova-data\">") {
+        match html.slice(k, html.len()).index_of("</script>") {
+            Some(close) => rest = "${html.slice(end, k)}${html.slice(k + close + 9, html.len())}"
+            None => {}
+        }
+    }
+    Some("${html.slice(0, at)}${app_root()}${rest}")
+}
+
+// Whether `word`, in lower case, is written in `cs` at `at`, ASCII letters
+// in either case.
+fn matches_at(cs: [Char], at: Int, word: String) -> Bool {
+    let ws = word.chars()
+    if at + ws.len() > cs.len() { return false }
+    let mut k = 0
+    while k < ws.len() {
+        let c = cs[at + k]
+        let same = c == ws[k] || (c >= 'A' && c <= 'Z' && "${c}".to_lower() == "${ws[k]}")
+        if !same { return false }
+        k = k + 1
+    }
+    true
+}
+
+// Whether `c` is whitespace as HTML reads it: a space, a tab, an LF, a CR
+// or a form feed.
+fn html_space(c: Char) -> Bool {
+    c == ' ' || c == '\t' || c == '\n' || c == '\r' || "${c}" == "\u{c}"
+}
+
+// Whether `c` ends a tag's name, as a space, `>` or `/` does.
+fn ends_name(c: Char) -> Bool {
+    c == ' ' || c == '>' || c == '/' || c == '\t' || c == '\n' || c == '\r'
+}
+
+// The error for a file that cannot be read, naming it.
+fn unreadable(by: String, path: String, e: IoError) -> IoError {
+    IoError { kind: e.kind, message: "${by}: cannot read ${path}: ${e.message}" }
+}
+
+// The error for a template with no root, naming it.
+fn rootless(by: String, path: String) -> IoError {
+    IoError { kind: InvalidData, message: "${by}: ${path} has no <div id=\"app\">" }
+}
+
+// The host of `address`, `"host:port"`: all before its last `:`.
+fn host_of(address: String) -> String {
+    let cs = address.chars()
+    let mut at = cs.len()
+    let mut i = 0
+    while i < cs.len() {
+        if cs[i] == ':' { at = i }
+        i = i + 1
+    }
+    address.slice(0, at)
+}
+
+// `json` made safe for a `script` element (spec §6.2): `<`, `>` and `&`, and
+// U+2028 and U+2029, as JSON's `\u` escapes, which only its strings can hold.
+fn script_safe(json: String) -> String {
+    let cs = json.chars()
+    let mut parts = [""; cs.len()]
+    let mut i = 0
+    while i < cs.len() {
+        let c = cs[i]
+        let code = char_to_int(c)
+        parts[i] = if c == '<' {
+            "\\u003c"
+        } else if c == '>' {
+            "\\u003e"
+        } else if c == '&' {
+            "\\u0026"
+        } else if code == 8232 {
+            "\\u2028"
+        } else if code == 8233 {
+            "\\u2029"
+        } else {
+            "${c}"
+        }
+        i = i + 1
+    }
+    "".join(parts)
+}
+
+// `text` percent-decoded as UTF-8, or as written when a `%` is not followed
+// by two hex digits or the bytes are not UTF-8: `std/ui/router`'s rule.
+fn decoded(text: String) -> String {
+    if !text.contains("%") { return text }
+    let bs = bytes_from_string(text).to_ints()
+    let mut out = [0; bs.len()]
+    let mut n = 0
+    let mut i = 0
+    while i < bs.len() {
+        if bs[i] == 37 {
+            if i + 2 >= bs.len() { return text }
+            let hi = hex_digit(bs[i + 1])
+            let lo = hex_digit(bs[i + 2])
+            if hi < 0 || lo < 0 { return text }
+            out[n] = hi * 16 + lo
+            i = i + 3
+        } else {
+            out[n] = bs[i]
+            i = i + 1
+        }
+        n = n + 1
+    }
+    bytes_from_ints(out).slice(0, n).to_string().unwrap_or(text)
+}
+
+// The value of the hex digit whose ASCII code is `b`, or -1.
+fn hex_digit(b: Int) -> Int {
+    if b >= 48 && b <= 57 {
+        b - 48
+    } else if b >= 65 && b <= 70 {
+        b - 55
+    } else if b >= 97 && b <= 102 {
+        b - 87
+    } else {
+        -1
+    }
+}
````


### Task 4: `nova build --ssr` and `--ssg`, and the driver's server

Spec §7, §8.4; decisions 10, 11, 20, 25, 33, 37 and 39.

**Files:**
- Create: `crates/nova-cli/tests/ssr_tests.rs`
- Modify:
  - `crates/nova-driver/src/program.rs` (`Roots::Server`)
  - `crates/nova-cli/src/cmd/run.rs` (`--ssr`, `--ssg`, `build_client`,
    `build_ssr`, `generate`, `page_in`, `ended`)
  - `crates/nova-cli/src/bundle.rs` (`Listing`, `finish`, `add_pages`,
    `write_manifest`)
- Test:
  - `crates/nova-cli/tests/ssr_tests.rs`
  - `tests/ssr/site/src/stray.nova`
  - `crates/nova-driver/tests/packages.rs`
  - `crates/nova-cli/src/bundle.rs` (its test alone)
  - `crates/nova-cli/src/cmd/run.rs` (its two tests alone)

- [ ] **Step 1: The tests.** `patch_block t4-tests`. The patch holds:
  - **`ssr_tests.rs`,** each test on a fresh copy of `tests/ssr/site` in a
    directory with a fixed name, with a fresh `NOVA_HOME` beside it; the
    server's path it expects, `built_server`, is joined by components, as
    `nova` joins it, so it prints alike on every system:
    - `ssr_builds_the_client_and_the_server`: `--ssr` writes the client
      into `dist/`, as `--target wasm` does, its page's root still empty and
      no route's page there, and the server into `target/debug/site-server`,
      which then runs: given `--ssg`, it writes the pages. With `-o`, the
      client goes to the directory named, and the server where it went
      before.
    - `ssg_writes_a_page_per_static_route_and_lists_them`: `--ssg` builds
      as `--ssr` does, then runs the server from the project's root with
      `--ssg dist`, its output passing through: a page for each static
      route, `/` over `dist/index.html`, none for the route with a
      parameter or the catch-all; the manifest lists each page with its
      size, and `index.html` with its new one; the server stays.
    - `ssr_and_ssg_together_is_ssg_from_anywhere_in_the_project`: `--ssr
      --ssg -o ../out`, run from `src/`: the server still runs from the
      project's root and is given the client's directory, so the pages
      land beside the client's files, and the manifest lists them.
    - `a_failing_server_fails_ssg_with_its_status`: with `refuse.nova` as
      the server, exit 3, after its output: its two pages, its refusal, and
      the line `Error: the server failed to generate the pages, with exit
      code 3: target/debug/site-server` (decision 37); the pages are on
      disk, and the manifest does not list them.
    - `ssg_refuses_a_page_outside_the_output_directory`: with
      `src/stray.nova` as the server, which says it wrote
      `dist/index.html` and `dist/../nova.toml` and writes neither,
      `--ssg` fails once it has ended well: stdout ends with those two
      lines, stderr holds ``Error: the server's line `ssg: wrote
      dist/../nova.toml` names no file inside dist``, and the manifest
      lists the client's files alone (decisions 33 and 39).
    - `ssg_takes_a_page_named_by_an_absolute_path`: a server written by the
      test, which writes `<dir>/abs/index.html` and names it by an absolute
      path, `<dir>` read from the project's root if relative, has the page
      listed with its size whether `nova build --ssg` runs in the
      project's root, which gives it `--ssg dist`, or in `src/` (decision
      33).
    - `ssr_and_ssg_refuse_before_building`: each refusal of decision 20, in
      its words, with no `dist/` and no `target/` after it.
    - `release_applies_to_the_client_and_the_server`: `--ssr --release`:
      the manifest says `"release": true`, and the server is
      `target/release/site-server`, with none under `target/debug/`. It
      passes without running where no `clang` is on `PATH`.
  - **`packages.rs`'s `a_server_is_rooted_at_src_server_nova`:**
    `Roots::Server` roots a package's program at `src/server.nova` alone,
    which imports the package's own module and its dependency, and leaves
    `src/main.nova` and `src/lib.nova` out; a package with no server has no
    root, and does not run.
  - **`bundle.rs`'s `ssg_pages_join_the_manifest_with_their_sizes`:**
    `add_pages` lists `about/index.html` with its size, and measures the
    files already listed again: `index.html` is 15 bytes once the root's
    page has replaced it.
  - **`cmd/run.rs`'s two tests,** a `tests` module at the end of the
    base's file: `a_page_lies_in_dist_with_plain_names`, where `page_in`
    keeps `out/dist/index.html` and `out/dist/about/index.html` and refuses
    `out/dist/../nova.toml`, `out/dist/a/../../nova.toml`, a file elsewhere,
    an absolute path and `out/dist` itself; and, on Unix,
    `a_failed_server_is_told_by_its_code_or_its_signal`, where `ended`
    reads `with exit code 3` and `stopped by signal 9`.
- [ ] **Step 2: Run, and see them fail.**
  - `cargo test -p nova-cli --test ssr_tests`. Expected: it builds; `0
    passed; 8 failed`:
    - `ssr_and_ssg_refuse_before_building`, at `ssr_tests.rs:346`:
      `["build", "--ssr", "src/main.nova"]: error: unexpected argument
      '--ssr' found`;
    - `a_failing_server_fails_ssg_with_its_status`: `Unexpected return
      code, failed var == 3`, `var: 2`, `clap`'s status;
    - `ssg_refuses_a_page_outside_the_output_directory`, at
      `ssr_tests.rs:268`: its check that stdout ends with the server's two
      lines, which prints stdout, empty here;
    - `ssr_builds_the_client_and_the_server`,
      `ssg_writes_a_page_per_static_route_and_lists_them`,
      `ssr_and_ssg_together_is_ssg_from_anywhere_in_the_project`,
      `ssg_takes_a_page_named_by_an_absolute_path` and
      `release_applies_to_the_client_and_the_server`, each "Unexpected
      failure." with `error: unexpected argument '--ssr' found`, or
      `'--ssg'`.

    Where no `clang` is on `PATH` (Task 0), the release test passes
    without running: `1 passed; 7 failed`.
  - `cargo test -p nova-driver --test packages`. Expected: it does not
    build: ``error[E0599]: no variant, associated function, or constant
    named `Server` found for enum `Roots` in the current scope``, at
    `packages.rs:475` and `496`: ``could not compile `nova-driver` (test
    "packages") due to 2 previous errors``.
  - `cargo test -p nova-cli --bin nova bundle::`, then `cargo test -p
    nova-cli --bin nova cmd::run::`. Expected: neither builds, each with
    the same six errors: ``error[E0433]: cannot find type `Path` in this
    scope``, twice, at `run.rs:241` and `242` (the base's `run.rs` imports
    `PathBuf` alone; the code adds `Path`); ``error[E0599]: no method named
    `add_pages` found for struct `Bundle` in the current scope``
    (`bundle.rs:371`); ``error[E0425]: cannot find function `page_in` in
    this scope`` (`run.rs:242`); and ``error[E0425]: cannot find function
    `ended` in this scope``, at `run.rs:264` and `266`; with the warning
    ``unused import: `super::*` `` (`run.rs:235`): ``could not compile
    `nova-cli` (bin "nova" test) due to 6 previous errors; 1 warning
    emitted``.
- [ ] **Step 3: The code.** `patch_block t4-code`:
  - **`nova-driver`:** `Roots::Server`: `Program::for_package` roots the
    program at `src/server.nova` instead of `src/main.nova`, and never adds
    `src/lib.nova` to it, as for `Roots::Program`.
  - **`cmd/run.rs`:** `BuildCmd`'s `--ssr` and `--ssg`, and `target`, now
    an `Option`, native when left out (decision 20); `build_wasm` calls
    `build_client`, which builds the page and returns its directory, its
    bundle and its listing; `build_ssr`: the refusals, in order, then
    `project::sync`, the client, and the server at
    `target/<profile>/<name>-server` (decision 11); and `generate`, which
    runs the server from the project's root with `--ssg <dist>`, the
    directory made absolute unless that root is the current directory,
    passes its stdout through, reads each `ssg: wrote` line's file, if
    relative, from the project's root, as it makes that directory absolute,
    keeps the pages under it through `page_in`, which takes a file only
    when each name below the directory is a plain one, and on the server's
    failure prints how it ended in `ended`'s words and exits with its code,
    or, once it has ended well, fails on the first line whose file
    `page_in` refused (decisions 10, 25, 33, 37 and 39);
    `could_not_compile`, shared.
  - **`bundle.rs`:** `Listing`, which `finish` now returns; `add_pages`,
    which lists the pages and measures every listed file again; and
    `write_manifest`, which both call.
- [ ] **Step 4: Run.**
  - The same four. Expected: `ssr_tests` `8 passed` (about 10 s);
    `packages` `15 passed` (14 before); `--bin nova bundle::` `3 passed; 0
    failed; 0 ignored; 0 measured; 37 filtered out`; `--bin nova
    cmd::run::` `2 passed; 0 failed; 0 ignored; 0 measured; 38 filtered
    out`.
  - The whole binaries:
    - `cargo test -p nova-cli --bin nova`. Expected: `40 passed` (37
      before).
    - `cargo test -p nova-driver`. Expected: as in Task 3, with `packages`
      `15`.
    - `NOVA_REQUIRE_NODE=1 cargo test -p nova-cli --no-fail-fast`, in about
      two minutes. Expected: every binary passes: the unit tests `40`,
      `build_support` `12`, `deps` `7`, `dev_tests` `6`, `doc` `13`,
      `doc_test` `17`, `fmt` `16`, `install_scripts` `9`, `lsp` `36` (1
      ignored), `lsp_fixes` `16`, `lsp_navigation` `38`, `packages` `10`,
      `project` `25`, `publish_script` `13`, `registry` `22`,
      `release_manifests` `7`, `run_tests` `317` (1 ignored), `ssr_tests`
      `8`, `vscode_extension` `3`, `wasm_tests` `229`.
- [ ] **Step 5: The checks,** as the conventions list them.
- [ ] **Step 6: Commit** `cli: nova build --ssr and --ssg; the driver roots a server at src/server.nova`.

**Patch `t4-tests`:**

<!-- patch t4-tests -->

````diff
diff --git a/crates/nova-cli/src/bundle.rs b/crates/nova-cli/src/bundle.rs
index 4d155078546738cc31d0e0282b63d0bf0bfaa4c2..32d8cf36131e5e3b49d5d5223d8bbb7e520f6ff5 100644
--- a/crates/nova-cli/src/bundle.rs
+++ b/crates/nova-cli/src/bundle.rs
@@ -342,6 +342,43 @@ mod tests {
         assert_eq!(page, "mine");
     }
 
+    /// `add_pages` lists the pages `nova build --ssg`'s server wrote, with
+    /// their sizes, and measures the files already listed again, since the
+    /// root's page has replaced `index.html` (the 4.6 spec §7).
+    #[test]
+    fn ssg_pages_join_the_manifest_with_their_sizes() {
+        let dir = std::env::temp_dir().join("nova-cli-bundle-ssg-pages");
+        let _ = std::fs::remove_dir_all(&dir);
+        std::fs::create_dir_all(dir.join("about")).expect("a directory");
+        for (name, text) in [
+            ("index.html", "page"),
+            ("app.js", "js"),
+            ("app.wasm", "wasm"),
+            ("package.json", "{}"),
+        ] {
+            std::fs::write(dir.join(name), text).expect("a file");
+        }
+        let bundle = Bundle {
+            root: None,
+            name: "site".to_string(),
+            version: None,
+            release: false,
+        };
+        let listing = bundle.finish(&dir).expect("the bundle finishes");
+        std::fs::write(dir.join("index.html"), "the root's page").expect("a page");
+        std::fs::write(dir.join("about").join("index.html"), "about").expect("a page");
+        bundle
+            .add_pages(&dir, listing, &["about/index.html".to_string()])
+            .expect("the pages are listed");
+        let manifest = std::fs::read_to_string(dir.join("manifest.json")).expect("the manifest");
+        assert!(
+            manifest.contains(
+                "    \"about/index.html\": 5,\n    \"app.js\": 2,\n    \"app.wasm\": 4,\n    \"index.html\": 15,\n"
+            ),
+            "{manifest}"
+        );
+    }
+
     #[test]
     fn quote_escapes_what_json_needs() {
         assert_eq!(quote("plain"), "\"plain\"");
diff --git a/crates/nova-cli/src/cmd/run.rs b/crates/nova-cli/src/cmd/run.rs
index 336e2e0075a1b12f3faed63abc88e0040fed1955..7b2cff51eccfc327a0deb3b9b1aff1bd7f12da4e 100644
--- a/crates/nova-cli/src/cmd/run.rs
+++ b/crates/nova-cli/src/cmd/run.rs
@@ -229,3 +229,40 @@ pub fn check(cmd: CheckCmd) -> Result<()> {
         }
     }
 }
+
+#[cfg(test)]
+mod tests {
+    use super::*;
+
+    /// A page `--ssg` lists lies in the output directory, each name below it
+    /// a plain one (ADR 0045).
+    #[test]
+    fn a_page_lies_in_dist_with_plain_names() {
+        let dist = Path::new("out/dist");
+        let page = |file: &str| page_in(dist, Path::new(file));
+        assert_eq!(page("out/dist/index.html").as_deref(), Some("index.html"));
+        assert_eq!(
+            page("out/dist/about/index.html").as_deref(),
+            Some("about/index.html")
+        );
+        assert_eq!(page("out/dist/../nova.toml"), None);
+        assert_eq!(page("out/dist/a/../../nova.toml"), None);
+        assert_eq!(page("out/elsewhere/index.html"), None);
+        assert_eq!(page("/out/dist/index.html"), None);
+        assert_eq!(page("out/dist"), None);
+        assert_eq!(page("out/dist/"), None);
+    }
+
+    /// A failed server's end is told by its code, or on Unix by the signal
+    /// that stopped it, and never by `ExitStatus`'s own words, which differ
+    /// between systems.
+    #[cfg(unix)]
+    #[test]
+    fn a_failed_server_is_told_by_its_code_or_its_signal() {
+        use std::os::unix::process::ExitStatusExt;
+        let code = std::process::ExitStatus::from_raw(3 << 8);
+        assert_eq!(ended(code), "with exit code 3");
+        let killed = std::process::ExitStatus::from_raw(9);
+        assert_eq!(ended(killed), "stopped by signal 9");
+    }
+}
diff --git a/crates/nova-cli/tests/ssr_tests.rs b/crates/nova-cli/tests/ssr_tests.rs
new file mode 100644
index 0000000000000000000000000000000000000000..42ea672a6482ea0faa821046a64ba5076192858d
--- /dev/null
+++ b/crates/nova-cli/tests/ssr_tests.rs
@@ -0,0 +1,419 @@
+//! `nova build --ssr` and `--ssg` (spec
+//! `docs/superpowers/specs/2026-10-19-phase-4-6-ssr-hydration-and-ssg-design.md`
+//! §7, §9.4), on copies of `tests/ssr/site`: a project whose client,
+//! `src/main.nova`, and server, `src/server.nova`, share `src/app.nova`,
+//! with three static routes, one with a parameter, and the catch-all.
+
+use std::collections::BTreeMap;
+use std::path::{Path, PathBuf};
+
+use assert_cmd::assert::Assert;
+use assert_cmd::Command;
+
+fn repo_root() -> PathBuf {
+    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
+}
+
+/// A copy of `tests/ssr/site` in a fresh directory with a fixed name, so each
+/// run replaces the last, and the fresh `NOVA_HOME` beside it.
+fn site(name: &str) -> (PathBuf, PathBuf) {
+    let base = std::env::temp_dir().join(format!("nova-ssr-build-{name}"));
+    let _ = std::fs::remove_dir_all(&base);
+    let dir = base.join("site");
+    std::fs::create_dir_all(dir.join("src")).expect("create the project");
+    let fixture = repo_root().join("tests/ssr/site");
+    std::fs::copy(fixture.join("nova.toml"), dir.join("nova.toml")).expect("copy nova.toml");
+    for entry in std::fs::read_dir(fixture.join("src")).expect("the fixture's sources") {
+        let path = entry.expect("an entry").path();
+        let to = dir.join("src").join(path.file_name().expect("a name"));
+        std::fs::copy(&path, to).expect("copy a source");
+    }
+    (dir, base.join("home"))
+}
+
+/// `nova` in `dir`, with `NOVA_HOME` at `home`, no colour and no backtrace.
+fn nova_at(dir: &Path, home: &Path) -> Command {
+    let mut cmd = Command::cargo_bin("nova").expect("nova binary builds");
+    cmd.current_dir(dir)
+        .env("NOVA_HOME", home)
+        .env("NO_COLOR", "1")
+        .env("RUST_BACKTRACE", "0");
+    cmd
+}
+
+fn stdout(a: &Assert) -> String {
+    String::from_utf8_lossy(&a.get_output().stdout).replace("\r\n", "\n")
+}
+
+fn stderr(a: &Assert) -> String {
+    String::from_utf8_lossy(&a.get_output().stderr).replace("\r\n", "\n")
+}
+
+/// The server's executable under `target/<profile>/`.
+fn server(dir: &Path, profile: &str) -> PathBuf {
+    dir.join(built_server(profile))
+}
+
+/// The server's executable from the project's root, `target/<profile>/`
+/// and its name, joined as `nova` joins them, so it prints as `nova` does
+/// on every system.
+fn built_server(profile: &str) -> PathBuf {
+    Path::new("target")
+        .join(profile)
+        .join(format!("site-server{}", std::env::consts::EXE_SUFFIX))
+}
+
+/// The files `manifest.json` in `dist` lists, with their sizes, as
+/// `bundle.rs` writes it: one `"path": size` line each.
+fn listed(dist: &Path) -> BTreeMap<String, u64> {
+    let text = std::fs::read_to_string(dist.join("manifest.json")).expect("manifest.json");
+    let files = text.split("\"files\": {").nth(1).expect("a files object");
+    files
+        .lines()
+        .filter_map(|line| {
+            let (path, size) = line.trim().trim_end_matches(',').split_once("\": ")?;
+            Some((path.trim_start_matches('"').to_string(), size.parse().ok()?))
+        })
+        .collect()
+}
+
+/// Each file the manifest lists has the size it gives.
+fn sizes_match(dist: &Path) {
+    for (path, size) in listed(dist) {
+        let real = std::fs::metadata(dist.join(&path))
+            .unwrap_or_else(|e| panic!("{path}: {e}"))
+            .len();
+        assert_eq!(real, size, "{path}");
+    }
+}
+
+/// `--ssr` (spec §7): the client into `dist/`, as `--target wasm` builds it,
+/// the page's root still empty and no page of a route written, and the
+/// server into `target/debug/site-server`, which then runs: given `--ssg`,
+/// it writes the pages. With `-o`, the client goes to the directory named,
+/// and the server to the same place as before.
+#[test]
+fn ssr_builds_the_client_and_the_server() {
+    let (dir, home) = site("ssr");
+    let a = nova_at(&dir, &home)
+        .args(["build", "--ssr"])
+        .assert()
+        .success();
+    let exe = built_server("debug");
+    assert_eq!(stdout(&a), format!("built dist\nbuilt {}\n", exe.display()));
+    let dist = dir.join("dist");
+    let names: Vec<String> = listed(&dist).into_keys().collect();
+    assert_eq!(names, ["app.js", "app.wasm", "index.html", "package.json"]);
+    sizes_match(&dist);
+    let page = std::fs::read_to_string(dist.join("index.html")).expect("the page");
+    assert!(page.contains("<div id=\"app\"></div>"), "{page}");
+    assert!(!dist.join("about").exists());
+    assert!(server(&dir, "debug").is_file());
+
+    let generated = std::process::Command::new(server(&dir, "debug"))
+        .current_dir(&dir)
+        .args(["--ssg", "dist"])
+        .output()
+        .expect("run the server");
+    assert!(generated.status.success(), "{generated:?}");
+    assert!(dist.join("about/index.html").is_file());
+
+    let _ = std::fs::remove_dir_all(&dist);
+    std::fs::remove_file(server(&dir, "debug")).expect("remove the server");
+    let a = nova_at(&dir, &home)
+        .args(["build", "--ssr", "-o", "public_html"])
+        .assert()
+        .success();
+    assert_eq!(
+        stdout(&a),
+        format!("built public_html\nbuilt {}\n", exe.display())
+    );
+    assert!(dir.join("public_html/app.wasm").is_file());
+    assert!(!dist.exists());
+    assert!(server(&dir, "debug").is_file());
+}
+
+/// `--ssg` (spec §7, §9.4): it builds as `--ssr` does, then runs the server
+/// from the project's root with `--ssg dist`, whose output passes through.
+/// That writes one page per static route, `/` over `dist/index.html`, and
+/// none for the route with a parameter or the catch-all; and the manifest
+/// lists each page with its size, and `index.html` with its new one. The
+/// server's executable stays.
+#[test]
+fn ssg_writes_a_page_per_static_route_and_lists_them() {
+    let (dir, home) = site("ssg");
+    let a = nova_at(&dir, &home)
+        .args(["build", "--ssg"])
+        .assert()
+        .success();
+    assert_eq!(
+        stdout(&a),
+        format!(
+            "built dist\nbuilt {}\n\
+             ssg: wrote dist/index.html\n\
+             ssg: wrote dist/about/index.html\n\
+             ssg: wrote dist/docs/intro/index.html\n\
+             stopped\n",
+            built_server("debug").display()
+        )
+    );
+    let dist = dir.join("dist");
+    let names: Vec<String> = listed(&dist).into_keys().collect();
+    assert_eq!(
+        names,
+        [
+            "about/index.html",
+            "app.js",
+            "app.wasm",
+            "docs/intro/index.html",
+            "index.html",
+            "package.json"
+        ]
+    );
+    sizes_match(&dist);
+    for (path, view) in [
+        (
+            "index.html",
+            "<p>Home <a href=\"/about\" data-nova-link=\"\">About</a></p>",
+        ),
+        ("about/index.html", "<p>About</p>"),
+        ("docs/intro/index.html", "<p>Intro</p>"),
+    ] {
+        let page = std::fs::read_to_string(dist.join(path)).expect("a page");
+        assert!(
+            page.contains(&format!("<!--[-->{view}<!--]-->")),
+            "{path}: {page}"
+        );
+        assert!(page.contains("id=\"nova-data\""), "{path}: {page}");
+        assert!(
+            page.contains("import init from \"/app.js\""),
+            "{path}: {page}"
+        );
+    }
+    assert!(!dist.join("posts").exists());
+    assert!(server(&dir, "debug").is_file());
+}
+
+/// `--ssr --ssg` is `--ssg`, from a subdirectory too: the server still runs
+/// from the project's root, and is given the client's directory, here the
+/// one `-o` names from the subdirectory, so the pages land beside the
+/// client's files and the manifest lists them.
+#[test]
+fn ssr_and_ssg_together_is_ssg_from_anywhere_in_the_project() {
+    let (dir, home) = site("both");
+    let src = dir.join("src");
+    let a = nova_at(&src, &home)
+        .args(["build", "--ssr", "--ssg", "-o", "../out"])
+        .assert()
+        .success();
+    let out = stdout(&a);
+    assert!(out.starts_with("built ../out\nbuilt "), "{out}");
+    assert_eq!(out.matches("ssg: wrote ").count(), 3, "{out}");
+    assert!(out.ends_with("/docs/intro/index.html\nstopped\n"), "{out}");
+    let out_dir = dir.join("out");
+    assert!(out_dir.join("about/index.html").is_file());
+    assert!(out_dir.join("docs/intro/index.html").is_file());
+    assert!(!dir.join("dist").exists() && !src.join("about").exists());
+    assert!(listed(&out_dir).contains_key("docs/intro/index.html"));
+    sizes_match(&out_dir);
+}
+
+/// A server that fails under `--ssg` fails the build with its exit code,
+/// after its output, which `nova` names in the same words on every system:
+/// the pages it wrote are on disk, but the manifest does not list them.
+#[test]
+fn a_failing_server_fails_ssg_with_its_status() {
+    let (dir, home) = site("fails");
+    std::fs::copy(dir.join("src/refuse.nova"), dir.join("src/server.nova"))
+        .expect("the refusing server");
+    let a = nova_at(&dir, &home)
+        .args(["build", "--ssg"])
+        .assert()
+        .code(3);
+    let out = stdout(&a);
+    assert!(
+        out.ends_with("ssg: wrote dist/index.html\nssg: wrote dist/about/index.html\n"),
+        "{out}"
+    );
+    let err = stderr(&a);
+    assert!(
+        err.contains("ssg: the route path \"/a/../b\" names no file"),
+        "{err}"
+    );
+    assert!(
+        err.contains(&format!(
+            "Error: the server failed to generate the pages, with exit code 3: {}\n",
+            built_server("debug").display()
+        )),
+        "{err}"
+    );
+    assert!(dir.join("dist/about/index.html").is_file());
+    assert!(!listed(&dir.join("dist")).contains_key("about/index.html"));
+}
+
+/// A server whose `ssg: wrote` line names a file outside the output
+/// directory, `dist/../nova.toml`, fails `--ssg` once it has ended well,
+/// with an error naming the line, and the manifest lists none of its pages,
+/// not even the one inside `dist/` (ADR 0045).
+#[test]
+fn ssg_refuses_a_page_outside_the_output_directory() {
+    let (dir, home) = site("stray");
+    std::fs::copy(dir.join("src/stray.nova"), dir.join("src/server.nova"))
+        .expect("the stray server");
+    let a = nova_at(&dir, &home)
+        .args(["build", "--ssg"])
+        .assert()
+        .failure();
+    let out = stdout(&a);
+    assert!(
+        out.ends_with("ssg: wrote dist/index.html\nssg: wrote dist/../nova.toml\n"),
+        "{out}"
+    );
+    let err = stderr(&a);
+    assert!(
+        err.contains(
+            "Error: the server's line `ssg: wrote dist/../nova.toml` names no file inside dist\n"
+        ),
+        "{err}"
+    );
+    let names: Vec<String> = listed(&dir.join("dist")).into_keys().collect();
+    assert_eq!(names, ["app.js", "app.wasm", "index.html", "package.json"]);
+}
+
+/// A server whose `ssg: wrote` line names its page by an absolute path in
+/// the output directory has it listed, whether `nova` runs in the project's
+/// root, which gives the server `--ssg dist`, or in `src/`, which gives it
+/// that directory made absolute: a relative path is read from the server's
+/// working directory, the project's root, and the output directory is made
+/// absolute the same way, so the pages do not hang on where `nova` runs
+/// (ADR 0045).
+#[test]
+fn ssg_takes_a_page_named_by_an_absolute_path() {
+    let (dir, home) = site("absolute");
+    // The project's root as `nova`'s working directory reads it there: with
+    // its links resolved, as the system gives it, on Unix.
+    let root = if cfg!(windows) {
+        dir.clone()
+    } else {
+        dir.canonicalize().expect("the project's root")
+    };
+    let quoted = root
+        .display()
+        .to_string()
+        .replace('\\', "\\\\")
+        .replace('"', "\\\"");
+    std::fs::write(
+        dir.join("src/server.nova"),
+        format!(
+            "//! Given `--ssg <dir>`, writes `<dir>/abs/index.html` and names it by\n\
+             //! an absolute path, `<dir>` read from the project's root if relative.\n\
+             \n\
+             async fn main() {{\n\
+             \x20   let dir = args().get(2).unwrap_or(\"dist\")\n\
+             \x20   let absolute = dir.starts_with(\"/\") || dir.slice(1, 2) == \":\"\n\
+             \x20   let at = if absolute {{ dir }} else {{ \"{quoted}/${{dir}}\" }}\n\
+             \x20   let _ = create_dir_all(\"${{at}}/abs\").await\n\
+             \x20   let _ = write_string(\"${{at}}/abs/index.html\", \"absolute\").await\n\
+             \x20   println(\"ssg: wrote ${{at}}/abs/index.html\")\n\
+             }}\n"
+        ),
+    )
+    .expect("the absolute server");
+    for at in [dir.clone(), dir.join("src")] {
+        let a = nova_at(&at, &home)
+            .args(["build", "--ssg"])
+            .assert()
+            .success();
+        let out = stdout(&a);
+        assert!(
+            out.contains("\nssg: wrote ") && out.ends_with("/abs/index.html\n"),
+            "{out}"
+        );
+        let files = listed(&dir.join("dist"));
+        assert_eq!(files.get("abs/index.html"), Some(&8), "{files:?}");
+        sizes_match(&dir.join("dist"));
+    }
+}
+
+/// The refusals (spec §7), each before anything is built: a file argument;
+/// `--target`; a library; no `src/server.nova`, for each flag; and, outside
+/// any project, no `nova.toml`.
+#[test]
+fn ssr_and_ssg_refuse_before_building() {
+    let (dir, home) = site("refusals");
+    let refused = |dir: &Path, args: &[&str], says: &str| {
+        let a = nova_at(dir, &home).args(args).assert().failure();
+        assert!(stderr(&a).contains(says), "{args:?}: {}", stderr(&a));
+        assert!(!dir.join("dist").exists(), "{args:?}");
+        assert!(!dir.join("target").exists(), "{args:?}");
+    };
+    refused(
+        &dir,
+        &["build", "--ssr", "src/main.nova"],
+        "`--ssr` builds the project's src/main.nova and src/server.nova; it takes no file",
+    );
+    refused(
+        &dir,
+        &["build", "--ssg", "--target", "wasm"],
+        "`--ssg` builds the client for wasm and the server natively; it takes no `--target`",
+    );
+    refused(
+        &dir,
+        &["build", "--ssr", "--target", "native"],
+        "`--ssr` builds the client for wasm and the server natively; it takes no `--target`",
+    );
+    std::fs::remove_file(dir.join("src/server.nova")).expect("remove the server");
+    refused(
+        &dir,
+        &["build", "--ssr"],
+        "`--ssr` needs src/server.nova, the server's entry",
+    );
+    refused(
+        &dir,
+        &["build", "--ssr", "--ssg"],
+        "`--ssg` needs src/server.nova, the server's entry",
+    );
+    std::fs::copy(
+        repo_root().join("tests/ssr/site/src/server.nova"),
+        dir.join("src/server.nova"),
+    )
+    .expect("put the server back");
+    std::fs::rename(dir.join("src/main.nova"), dir.join("src/lib.nova")).expect("a library");
+    refused(
+        &dir,
+        &["build", "--ssg"],
+        "`site` is a library: it has no src/main.nova",
+    );
+    let loose = dir.with_file_name("loose");
+    std::fs::create_dir_all(loose.join("src")).expect("a directory");
+    std::fs::write(loose.join("src/server.nova"), "fn main() {}\n").expect("a server");
+    refused(
+        &loose,
+        &["build", "--ssr"],
+        "`--ssr` builds a project, and there is no nova.toml here or above",
+    );
+}
+
+/// `--release` applies to both builds: the manifest says so, and the server
+/// is `target/release/site-server`. Skipped where no `clang` is on PATH, as
+/// every release build's test is.
+#[test]
+fn release_applies_to_the_client_and_the_server() {
+    let clang = std::process::Command::new("clang")
+        .arg("--version")
+        .output()
+        .is_ok();
+    if !clang {
+        eprintln!("skipping: no clang on PATH");
+        return;
+    }
+    let (dir, home) = site("release");
+    nova_at(&dir, &home)
+        .args(["build", "--ssr", "--release"])
+        .assert()
+        .success();
+    let manifest = std::fs::read_to_string(dir.join("dist/manifest.json")).expect("manifest");
+    assert!(manifest.contains("\"release\": true"), "{manifest}");
+    assert!(server(&dir, "release").is_file());
+    assert!(!server(&dir, "debug").exists());
+}
diff --git a/crates/nova-driver/tests/packages.rs b/crates/nova-driver/tests/packages.rs
index 7fe8bfbcbd60e2d84107f512c767ff891314fc83..4e0e98002585a0589462ab34b6adb55a4eb3c024 100644
--- a/crates/nova-driver/tests/packages.rs
+++ b/crates/nova-driver/tests/packages.rs
@@ -444,3 +444,55 @@ fn a_missing_sibling_is_probed_once_per_directory() {
         .count();
     assert_eq!(probes, 1);
 }
+
+/// A package's server (spec
+/// `docs/superpowers/specs/2026-10-19-phase-4-6-ssr-hydration-and-ssg-design.md`
+/// §8.4): `Roots::Server` roots the program at `src/server.nova` alone,
+/// which imports the package's own modules and its dependencies, as the
+/// client does, and leaves `src/main.nova` and `src/lib.nova` out. A package
+/// with no server has no root.
+#[test]
+fn a_server_is_rooted_at_src_server_nova() {
+    let dir = fresh("server");
+    geom(&dir);
+    let app = dir.join("app");
+    write(
+        &app,
+        &[
+            ("nova.toml", manifest("app", GEOM_DEPENDENCY).as_str()),
+            (
+                "src/main.nova",
+                "import shared\n\nfn main() {\n    println(\"client ${label()}\")\n}\n",
+            ),
+            (
+                "src/server.nova",
+                "import geom\nimport shared\n\nfn main() {\n    println(\"server ${label()} ${area()}\")\n}\n",
+            ),
+            ("src/shared.nova", "pub fn label() -> String {\n    \"shared\"\n}\n"),
+            ("src/lib.nova", "pub fn unused() -> Int {\n    1\n}\n"),
+        ],
+    );
+    let program = Program::for_package(&app, Roots::Server);
+    assert!(program.runs);
+    assert_eq!(program.roots.len(), 1);
+    let a = check(program);
+    assert!(a.diagnostics.is_empty(), "{}", messages(&a));
+    let paths = paths(&a);
+    assert!(paths[0].ends_with("app/src/server.nova"), "{paths:?}");
+    assert!(
+        paths.iter().any(|p| p.ends_with("geom/src/lib.nova")),
+        "{paths:?}"
+    );
+    assert!(
+        paths.iter().any(|p| p.ends_with("app/src/shared.nova")),
+        "{paths:?}"
+    );
+    assert!(
+        !paths
+            .iter()
+            .any(|p| p.ends_with("app/src/main.nova") || p.ends_with("app/src/lib.nova")),
+        "{paths:?}"
+    );
+    let none = Program::for_package(&dir.join("geom"), Roots::Server);
+    assert!(none.roots.is_empty() && !none.runs);
+}
diff --git a/tests/ssr/site/src/stray.nova b/tests/ssr/site/src/stray.nova
new file mode 100644
index 0000000000000000000000000000000000000000..0f5e3d744f882bc6bf3aa9de50314f40f3b22102
--- /dev/null
+++ b/tests/ssr/site/src/stray.nova
@@ -0,0 +1,9 @@
+//! A server that, given `--ssg <dir>`, says it wrote two pages, one in
+//! `<dir>` and one outside it, `<dir>/../nova.toml`, and writes neither.
+//! `nova build --ssg` lists neither, and fails, naming the second line.
+
+fn main() {
+    let dir = args().get(2).unwrap_or("dist")
+    println("ssg: wrote ${dir}/index.html")
+    println("ssg: wrote ${dir}/../nova.toml")
+}
````

**Patch `t4-code`:**

<!-- patch t4-code -->

````diff
diff --git a/crates/nova-cli/src/bundle.rs b/crates/nova-cli/src/bundle.rs
index 32d8cf36131e5e3b49d5d5223d8bbb7e520f6ff5..ba29246ffb0dcd028dd66d554e12d64cbe0627d3 100644
--- a/crates/nova-cli/src/bundle.rs
+++ b/crates/nova-cli/src/bundle.rs
@@ -15,6 +15,17 @@ use crate::project::{self, Mode};
 /// The files the compiler writes into `dist/` (spec 4.0 §7).
 const BUILT: [&str; 4] = ["index.html", "app.js", "app.wasm", "package.json"];
 
+/// What [`Bundle::finish`] listed in `manifest.json`, to which `nova build
+/// --ssg` adds the pages its server wrote (the 4.6 spec
+/// `docs/superpowers/specs/2026-10-19-phase-4-6-ssr-hydration-and-ssg-design.md`
+/// §7).
+pub struct Listing {
+    wasm_opt: bool,
+    /// Each file by its path from `dist/`, with `/` between names, and its
+    /// size in bytes.
+    files: BTreeMap<String, u64>,
+}
+
 /// What a build's bundle needs to know of its program.
 pub struct Bundle {
     /// The project's root, beside its `nova.toml`; `None` for a program
@@ -79,8 +90,8 @@ impl Bundle {
     /// `wasm-opt` and `manifest.json`, in that order, so the manifest lists
     /// what is there at the end. `assets/` is the build's only when it
     /// copies a `public/` into it: otherwise it is left as it is, and not
-    /// listed.
-    pub fn finish(&self, out: &Path) -> Result<()> {
+    /// listed. Returns what the manifest lists.
+    pub fn finish(&self, out: &Path) -> Result<Listing> {
         let mut copied = false;
         if let Some(root) = &self.root {
             let page = root.join("index.html");
@@ -113,8 +124,28 @@ impl Bundle {
         if copied {
             list(&out.join("assets"), "assets", &mut files)?;
         }
+        let listing = Listing { wasm_opt, files };
+        self.write_manifest(out, &listing)?;
+        Ok(listing)
+    }
+
+    /// After `nova build --ssg`'s server has written its pages into `out`:
+    /// `manifest.json` again, with `pages`, each a path from `out` such as
+    /// `about/index.html`, added to `listing`, and every file measured
+    /// again, since the root's page replaced `index.html`.
+    pub fn add_pages(&self, out: &Path, mut listing: Listing, pages: &[String]) -> Result<()> {
+        for page in pages {
+            listing.files.insert(page.clone(), 0);
+        }
+        for (name, bytes) in listing.files.iter_mut() {
+            *bytes = size(&out.join(name))?;
+        }
+        self.write_manifest(out, &listing)
+    }
+
+    fn write_manifest(&self, out: &Path, listing: &Listing) -> Result<()> {
         let path = out.join("manifest.json");
-        std::fs::write(&path, self.manifest(wasm_opt, &files))
+        std::fs::write(&path, self.manifest(listing.wasm_opt, &listing.files))
             .with_context(|| format!("writing {}", path.display()))
     }
 
diff --git a/crates/nova-cli/src/cmd/run.rs b/crates/nova-cli/src/cmd/run.rs
index 7b2cff51eccfc327a0deb3b9b1aff1bd7f12da4e..d7e0d2f0ce9453e1c8408ad5eb176efe7949f6f3 100644
--- a/crates/nova-cli/src/cmd/run.rs
+++ b/crates/nova-cli/src/cmd/run.rs
@@ -1,17 +1,21 @@
 //! `nova run [FILE]` — compile and execute a Nova program,
-//! `nova build [FILE]` — compile to a standalone executable, and
+//! `nova build [FILE]` — compile to a standalone executable, a page with
+//! `--target wasm`, or with `--ssr` and `--ssg` a project's client and
+//! server, and
 //! `nova check [FILE]` — type-check without running.
 //!
 //! With no FILE, each works on the project around the current directory, or
 //! on `src/main.nova` outside any project (`crate::project`).
 
-use std::path::PathBuf;
+use std::io::{BufRead, BufReader, Write};
+use std::path::{Component, Path, PathBuf};
+use std::process::{Command, Stdio};
 
-use anyhow::{Context, Result};
+use anyhow::{bail, Context, Result};
 use clap::Args;
 use nova_driver::{Outcome, Program, Roots};
 
-use crate::bundle::Bundle;
+use crate::bundle::{Bundle, Listing};
 use crate::project::{self, Mode};
 
 #[derive(Args)]
@@ -64,21 +68,37 @@ pub struct BuildCmd {
     /// Output executable path (default: target/debug/<name> in a project,
     /// or target/release/<name> with --release; otherwise `<file stem>` in
     /// the current directory; each with the platform executable suffix).
+    /// With `--target wasm`, `--ssr` or `--ssg`, the page's directory; the
+    /// server of `--ssr` is always target/<profile>/<name>-server.
     #[arg(short, long)]
     output: Option<PathBuf>,
 
     /// Optimizing build via the LLVM backend (emits LLVM IR and compiles it
     /// with a discovered `clang`/`llc`); the default is the fast Cranelift
     /// backend. With `--target wasm`, runs `wasm-opt -Oz` on app.wasm when
-    /// `wasm-opt` is on PATH.
+    /// `wasm-opt` is on PATH. With `--ssr` or `--ssg`, both builds.
     #[arg(long)]
     release: bool,
 
-    /// What to build for: `native`, an executable; or `wasm`, a page in a
-    /// directory (default: dist/ at the project's root, or in the current
-    /// directory), which `-o` names.
-    #[arg(long, value_enum, default_value_t = TargetArg::Native)]
-    target: TargetArg,
+    /// What to build for: `native`, an executable (the default); or
+    /// `wasm`, a page in a directory (default: dist/ at the project's root,
+    /// or in the current directory), which `-o` names.
+    #[arg(long, value_enum)]
+    target: Option<TargetArg>,
+
+    /// Build a project for server-side rendering: the client from
+    /// src/main.nova for WASM, as `--target wasm` builds it, into dist/ or
+    /// the directory `-o` names; and the server from src/server.nova,
+    /// natively, into target/debug/<name>-server, or
+    /// target/release/<name>-server with --release.
+    #[arg(long)]
+    ssr: bool,
+
+    /// Build as `--ssr` does, then generate the static routes' pages: run
+    /// the server from the project's root with `--ssg <dist>`, which writes
+    /// dist/<path>/index.html for each, and list them in dist/manifest.json.
+    #[arg(long)]
+    ssg: bool,
 }
 
 #[derive(Args)]
@@ -126,7 +146,9 @@ pub fn bundle(cmd: BundleCmd) -> Result<()> {
         file: cmd.file,
         output: cmd.output,
         release: false,
-        target: TargetArg::Wasm,
+        target: Some(TargetArg::Wasm),
+        ssr: false,
+        ssg: false,
     })
 }
 
@@ -138,22 +160,216 @@ fn build_wasm(cmd: BuildCmd) -> Result<()> {
     let mode = project::mode(cmd.file)?;
     project::refuse_library(&mode)?;
     project::sync(&mode)?;
-    let program = project::program_to_run(&mode)?;
-    let output = wasm_output(cmd.output, &mode);
-    let bundle = Bundle::of(&mode, &program, cmd.release);
+    build_client(&mode, cmd.output, cmd.release).map(|_| ())
+}
+
+/// A page built for WASM from `mode`'s program, as `nova build --target
+/// wasm` builds it, and `built <dir>` printed. Returns its directory, its
+/// bundle, and what its manifest lists.
+fn build_client(
+    mode: &Mode,
+    output: Option<PathBuf>,
+    release: bool,
+) -> Result<(PathBuf, Bundle, Listing)> {
+    let program = project::program_to_run(mode)?;
+    let output = wasm_output(output, mode);
+    let bundle = Bundle::of(mode, &program, release);
     bundle.check_output(&output)?;
     bundle.prepare(&output)?;
     match nova_driver::build_program_wasm(program, &output)? {
         Outcome::Ok(dir) => {
-            bundle.finish(&dir)?;
+            let listing = bundle.finish(&dir)?;
             println!("built {}", dir.display());
-            Ok(())
+            Ok((dir, bundle, listing))
+        }
+        Outcome::Failed { errors } => bail!(could_not_compile(errors)),
+    }
+}
+
+fn could_not_compile(errors: usize) -> String {
+    format!(
+        "could not compile due to {errors} previous error{}",
+        if errors == 1 { "" } else { "s" }
+    )
+}
+
+/// `nova build --ssr` and `--ssg` (the 4.6 spec
+/// `docs/superpowers/specs/2026-10-19-phase-4-6-ssr-hydration-and-ssg-design.md`
+/// §7): the project's client, from `src/main.nova`, into `dist/` or the
+/// directory `-o` names, then its server, from `src/server.nova`, into
+/// `target/<profile>/<name>-server`. `--ssg` then runs the server with
+/// `--ssg <dist>` and lists the pages it wrote in the manifest. What it
+/// refuses, it refuses before anything is built.
+fn build_ssr(cmd: BuildCmd) -> Result<()> {
+    let flag = if cmd.ssg { "--ssg" } else { "--ssr" };
+    if cmd.file.is_some() {
+        bail!("`{flag}` builds the project's src/main.nova and src/server.nova; it takes no file");
+    }
+    if cmd.target.is_some() {
+        bail!(
+            "`{flag}` builds the client for wasm and the server natively; it takes no `--target`"
+        );
+    }
+    let mode = project::mode(None)?;
+    let Mode::Project { root, target_dir } = &mode else {
+        bail!("`{flag}` builds a project, and there is no nova.toml here or above");
+    };
+    project::refuse_library(&mode)?;
+    if !root.join("src").join("server.nova").is_file() {
+        bail!("`{flag}` needs src/server.nova, the server's entry");
+    }
+    project::sync(&mode)?;
+    let (dist, bundle, listing) = build_client(&mode, cmd.output, cmd.release)?;
+
+    let program = mode.program(Roots::Server);
+    let dir = target_dir.join(if cmd.release { "release" } else { "debug" });
+    if !program.has_errors() {
+        std::fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;
+    }
+    let server = dir.join(format!(
+        "{}-server{}",
+        project::package_name(&program),
+        std::env::consts::EXE_SUFFIX
+    ));
+    let built = if cmd.release {
+        nova_driver::build_program_release(program, &server)?
+    } else {
+        nova_driver::build_program(program, &server)?
+    };
+    match built {
+        Outcome::Ok(path) => println!("built {}", path.display()),
+        Outcome::Failed { errors } => bail!(could_not_compile(errors)),
+    }
+    if !cmd.ssg {
+        return Ok(());
+    }
+    let pages = generate(root, &server, &dist)?;
+    bundle.add_pages(&dist, listing, &pages)
+}
+
+/// `--ssg`'s run of the server: from the project's root, with
+/// `--ssg <dist>`, its output passed through. Returns the pages it says it
+/// wrote, each a path from `dist`, from its `ssg: wrote <file>` lines.
+///
+/// A server that fails ends `nova` once its output has passed through:
+/// `nova` prints `Error: the server failed to generate the pages, with exit
+/// code <code>: <server>` and exits with that code; or, when a signal stops
+/// the server, which so has no exit code, it prints `…, stopped by signal
+/// <number>: …` and exits with 1. A `ssg: wrote` line's file, if relative,
+/// is read from the server's working directory, the project's root, and
+/// `dist` is made absolute the same way, so the answer does not hang on
+/// where `nova` runs; a line whose file does not then lie in `dist`, each
+/// name below it a plain one by Rust's path components (no `..`, root or
+/// prefix; a `.` piece and a trailing `/` are normalised away), fails the
+/// build, with an error naming the line, once the server has ended well;
+/// such a file is never listed.
+fn generate(root: &Path, server: &Path, dist: &Path) -> Result<Vec<String>> {
+    let cwd = std::env::current_dir().context("reading the current directory")?;
+    // The server runs in the project's root, so a relative path to it, or
+    // to `dist`, is made absolute unless that root is the current directory.
+    let from_root = |path: &Path| {
+        if path.is_absolute() || root.as_os_str().is_empty() {
+            path.to_path_buf()
+        } else {
+            cwd.join(path)
+        }
+    };
+    let dist = from_root(dist);
+    // The server's working directory, absolute, from which a relative path
+    // in its lines is read, and `dist` as the server reads it.
+    let base = cwd.join(root);
+    let dist_from_base = base.join(&dist);
+    let mut command = Command::new(cwd.join(server));
+    command
+        .arg("--ssg")
+        .arg(&dist)
+        .stdout(Stdio::piped())
+        .stderr(Stdio::inherit());
+    if !root.as_os_str().is_empty() {
+        command.current_dir(root);
+    }
+    let mut child = command
+        .spawn()
+        .with_context(|| format!("running {}", server.display()))?;
+    let stdout = child.stdout.take().context("the server's output")?;
+    let mut pages = Vec::new();
+    let mut stray: Option<String> = None;
+    let mut out = std::io::stdout();
+    for line in BufReader::new(stdout).split(b'\n') {
+        let line = line.context("reading the server's output")?;
+        out.write_all(&line)
+            .and_then(|()| out.write_all(b"\n"))
+            .and_then(|()| out.flush())
+            .context("writing the server's output")?;
+        let text = String::from_utf8_lossy(&line);
+        let text = text.trim_end_matches('\r');
+        let Some(file) = text.strip_prefix("ssg: wrote ") else {
+            continue;
+        };
+        match page_in(&dist_from_base, &base.join(file)) {
+            Some(page) => pages.push(page),
+            None => {
+                stray.get_or_insert_with(|| text.to_string());
+            }
+        }
+    }
+    let status = child
+        .wait()
+        .with_context(|| format!("running {}", server.display()))?;
+    if !status.success() {
+        eprintln!(
+            "Error: the server failed to generate the pages, {}: {}",
+            ended(status),
+            server.display()
+        );
+        std::process::exit(status.code().filter(|&c| c != 0).unwrap_or(1));
+    }
+    if let Some(line) = stray {
+        bail!(
+            "the server's line `{line}` names no file inside {}",
+            dist.display()
+        );
+    }
+    Ok(pages)
+}
+
+/// The page a `ssg: wrote <file>` line names: `file`'s path from `dist`,
+/// with `/` between names, when it lies in `dist` and each name below it is
+/// a plain one by Rust's path components, with no `..`, root or prefix (a
+/// `.` piece and a trailing `/` are normalised away, as `components` does);
+/// `None` otherwise, `dist` itself included. Both paths are compared as
+/// given, and nothing is asked of the file system: `generate` makes both
+/// absolute from the server's working directory first.
+fn page_in(dist: &Path, file: &Path) -> Option<String> {
+    let page = file.strip_prefix(dist).ok()?;
+    let mut names = Vec::new();
+    for c in page.components() {
+        match c {
+            Component::Normal(name) => names.push(name.to_string_lossy().into_owned()),
+            _ => return None,
+        }
+    }
+    if names.is_empty() {
+        return None;
+    }
+    Some(names.join("/"))
+}
+
+/// How a server that failed ended, in the same words on every system:
+/// `with exit code <code>`, or, on Unix, `stopped by signal <number>` when
+/// a signal stopped it and it has no code.
+fn ended(status: std::process::ExitStatus) -> String {
+    if let Some(code) = status.code() {
+        return format!("with exit code {code}");
+    }
+    #[cfg(unix)]
+    {
+        use std::os::unix::process::ExitStatusExt;
+        if let Some(signal) = status.signal() {
+            return format!("stopped by signal {signal}");
         }
-        Outcome::Failed { errors } => anyhow::bail!(
-            "could not compile due to {errors} previous error{}",
-            if errors == 1 { "" } else { "s" }
-        ),
     }
+    "with no exit code".to_string()
 }
 
 /// The directory `nova build --target wasm` writes, which `nova dev` builds
@@ -168,7 +384,10 @@ pub fn wasm_output(output: Option<PathBuf>, mode: &Mode) -> PathBuf {
 }
 
 pub fn build(cmd: BuildCmd) -> Result<()> {
-    if cmd.target == TargetArg::Wasm {
+    if cmd.ssr || cmd.ssg {
+        return build_ssr(cmd);
+    }
+    if cmd.target == Some(TargetArg::Wasm) {
         return build_wasm(cmd);
     }
     let mode = project::mode(cmd.file)?;
@@ -207,10 +426,7 @@ pub fn build(cmd: BuildCmd) -> Result<()> {
             println!("built {}", path.display());
             Ok(())
         }
-        Outcome::Failed { errors } => anyhow::bail!(
-            "could not compile due to {errors} previous error{}",
-            if errors == 1 { "" } else { "s" }
-        ),
+        Outcome::Failed { errors } => bail!(could_not_compile(errors)),
     }
 }
 
diff --git a/crates/nova-driver/src/program.rs b/crates/nova-driver/src/program.rs
index 74dbe8d3d085608bf082f58a15d06abb27715134..470137ce0f88d5e5f66002626ea54ac5a7f72fe2 100644
--- a/crates/nova-driver/src/program.rs
+++ b/crates/nova-driver/src/program.rs
@@ -22,6 +22,11 @@ pub enum Roots {
     /// Those, then `tests/*.nova` sorted by name: `nova test` and the
     /// language server.
     Test,
+    /// `src/server.nova` alone: the server `nova build --ssr` builds
+    /// natively, with the package's modules and dependencies (spec
+    /// `docs/superpowers/specs/2026-10-19-phase-4-6-ssr-hydration-and-ssg-design.md`
+    /// §8.4).
+    Server,
 }
 
 /// Where a root is.
@@ -58,7 +63,8 @@ pub struct Program {
     pub graph: Option<Graph>,
     /// The entry first.
     pub roots: Vec<Root>,
-    /// Whether MIR runs: the entry is `src/main.nova`, or a loose file.
+    /// Whether MIR runs: the entry is `src/main.nova`, a loose file, or a
+    /// server's `src/server.nova`.
     pub runs: bool,
     /// The graph's diagnostics.
     pub diagnostics: Vec<Diagnostic>,
@@ -114,7 +120,12 @@ impl Program {
         let mut db = FileDb::new();
         let (graph, diagnostics) = nova_pm::graph_with(root, None, offline, &mut db);
         let mut list = Vec::new();
-        let main = under(root, "src/main.nova");
+        let entry = if roots == Roots::Server {
+            "src/server.nova"
+        } else {
+            "src/main.nova"
+        };
+        let main = under(root, entry);
         if main.is_file() {
             list.push(Root {
                 path: main,
@@ -123,7 +134,7 @@ impl Program {
         }
         let runs = !list.is_empty();
         let lib = under(root, "src/lib.nova");
-        if roots != Roots::Program && lib.is_file() {
+        if !matches!(roots, Roots::Program | Roots::Server) && lib.is_file() {
             list.push(Root {
                 path: lib,
                 kind: RootKind::Src,
````


### Task 5: The browser test and CI

Spec §1 (the gate), §9.5, and the user's decision 11; decision 28.

**Files:**
- Create:
  - `tests/browser/ssr.mjs`
  - `tests/browser/ssr-app/nova.toml`, `src/app.nova`, `src/main.nova`,
    `src/server.nova`
- Modify: `.github/workflows/ci.yml` (the `browser` job)

- [ ] **Step 1: The test and CI.** `patch_block t5-code`:
  - **`tests/browser/ssr-app`,** a project with a client and a server:
    `src/app.nova` holds the data, a title holding `</script><!-- &` and
    three names, with `ToJson` and `FromJson`; a `for_each` over the names;
    links; and a `HistoryRouter` over `/`, a counter and a `show`,
    `/about`, `/posts/:id` and `*`; its root's `data-state` reads `server`
    in the HTML and `hydrated` once the client's `on_mount` has run
    (decision 28). `src/main.nova` calls `hydrate_with("#app", Data {
    title: "", names: [] }, App)`, with no title and no names for a page
    with no data, as `nova dev` serves (decision 29);
    `src/server.nova` serves on its first argument, or on `127.0.0.1:0`,
    and prints `serve`'s `Err` and exits with 1. It has no `index.html` of
    its own: the generated page, titled `Nova`, is the template.
  - **`tests/browser/ssr.mjs`,** run as `node tests/browser/ssr.mjs <nova>
    <work dir> <dir holding node_modules/playwright>`: it copies
    `ssr-app` into the work directory, emptied first, builds it there with
    `nova build --ssg`, with `NOVA_HOME` beside it, checks that the manifest
    lists `about/index.html` and that SSG wrote nothing for `/posts/:id`,
    starts `target/debug/ssr-app-server`, and reads the port from its first
    line.
    Through Playwright: `/` answers `200` and hydrates in place, and a
    click on `+` updates the count's own text node, kept from the server,
    and the `show` hides and shows again; a link goes to `/about` without
    a reload; `/posts/2` answers `200` and hydrates in place; `/nowhere`
    answers `404` with the "not found" page, which hydrates in place; and a
    static Node server serves `dist/`, whose `/about/` arrives already
    rendered, with its data, and hydrates in place. On every page the
    title reads as written, the console has no error but the one Chromium
    logs for `/nowhere`'s own `404`, and the page throws none. It prints
    the client's size, gzipped, and kills the server at the end, pass or
    fail.
  - **CI's `browser` job:** its build becomes `cargo build --locked -p
    nova-cli -p nova-runtime`, since cargo leaves `libnova_runtime.a` in
    `target/debug/`, beside `nova`, only when the build names the runtime,
    and a debug `nova` carries none: without it, `ssr.mjs`'s `nova build
    --ssg` cannot link the server (the fact-check's first finding); one
    more step, `SSR and SSG in Chromium`, runs `ssr.mjs` with
    `target/debug/nova`, a work directory in `$RUNNER_TEMP`, and the job's
    Playwright; the job's comment names 4.6's gate.
- [ ] **Step 2: Run the `browser` job, in Chromium,** where Task 0 found
  Playwright: its builds and every script, as CI runs them. As in CI, the
  runtime library beside `nova` must come from the job's own build, and no
  `NOVA_RUNTIME_LIB` may name one: so the library Task 0's workspace build
  left is removed first, and the build puts it back.

  ```sh
  unset NOVA_RUNTIME_LIB
  rm -f target/debug/libnova_runtime.a
  cargo build --locked -p nova-cli -p nova-runtime
  ls target/debug/libnova_runtime.a
  git status --short --ignored --untracked-files=all examples tests > "$P/status-46.txt"
  B="$P/browser-46"
  export PLAYWRIGHT_BROWSERS_PATH=/opt/pw-browsers
  target/debug/nova build --target wasm tests/dom/counter.nova -o "$B/counter"
  target/debug/nova build --target wasm tests/dom/ui_counter.nova -o "$B/ui_counter"
  (cd examples/06-counter-spa && ../../target/debug/nova build --target wasm -o "$B/counter-spa")
  node tests/browser/counter.mjs "$B/counter" /opt/node-tools
  node tests/browser/counter.mjs "$B/ui_counter" /opt/node-tools
  node tests/browser/spa.mjs "$B/counter-spa" /opt/node-tools
  node tests/browser/dev.mjs target/debug/nova "$B/dev" /opt/node-tools
  node tests/browser/router.mjs target/debug/nova "$B/router" /opt/node-tools
  node tests/browser/ssr.mjs target/debug/nova "$B/ssr" /opt/node-tools
  git status --short --ignored --untracked-files=all examples tests | diff "$P/status-46.txt" -
  ```

  Expected:
  - the build ends with `Finished`, `ls` names the library it put back,
    and each of the three builds prints `built <its directory>`;
  - `counter.mjs`, twice: `counter: three clicks, and the count reads 3`;
  - `spa.mjs`: `counter-spa: +, +, +, -, and the count reads 2`;
  - `dev.mjs`, in about 6 seconds: `dev: counts at build <start>.1,
    reloads into .2 on an edit, into .3 after a missed one, shows the
    overlay for the type error of .4 and keeps it over a reload, and counts
    again at .5`, where `<start>` is the server's start in milliseconds,
    such as the replay's `1792475991723`;
  - `router.mjs`, in about 2 seconds: `router: a link to /posts/2 without a
    reload, back home, a reload at /posts/2 that stays at build <start>.1,
    /nowhere not found, and a query read`;
  - `ssr.mjs`, in about 4 seconds once its build is done: `ssr: / hydrates
    in place (34 nodes kept) and counts, a link to /about without a reload,
    /posts/2 in place (24), /nowhere 404 in place (26), and SSG's /about/
    arrives rendered and hydrates in place (26); the client is 41260 bytes
    gzipped`. Record the client's size in the ledger: it is printed, not
    limited (spec §12, risk 4);
  - `diff` prints nothing: the builds write outside the tree, and the
    scripts work in copies. In the replay the status itself printed
    nothing.
- [ ] **Step 3: The checks,** as the conventions list them, `nova fmt`'s
  now with `tests/browser/ssr-app` before `tests/ssr`.
- [ ] **Step 4: Commit** `ci: SSR and SSG in Chromium`.

**Patch `t5-code`:**

<!-- patch t5-code -->

````diff
diff --git a/.github/workflows/ci.yml b/.github/workflows/ci.yml
index 376a9730b5f233169b8274b9f92f2084ff2a6e7c..296a9854aa4c0f64d8382261674563e4a5ec6eb4 100644
--- a/.github/workflows/ci.yml
+++ b/.github/workflows/ci.yml
@@ -102,9 +102,15 @@ jobs:
   # §5.4): `dev.mjs` runs `06-counter-spa` under `nova dev`, edits it, breaks
   # it and fixes it; `router.mjs` drives `tests/browser/router-app`, in
   # history mode, under `nova dev`. Each copies its project into
-  # `$RUNNER_TEMP` and kills `nova dev` at the end. Playwright, pinned, is
-  # installed here only, into a scratch directory; nothing in `cargo test` or
-  # `nova` installs a package. Ubuntu only: one browser job suffices.
+  # `$RUNNER_TEMP` and kills `nova dev` at the end. And 4.6's gate (spec
+  # docs/superpowers/specs/2026-10-19-phase-4-6-ssr-hydration-and-ssg-design.md
+  # §9.5): `ssr.mjs` builds `tests/browser/ssr-app` with `nova build --ssg`
+  # in `$RUNNER_TEMP`, starts its server on port 0, and checks that a
+  # server-rendered page, and a page SSG wrote, served by a static server,
+  # hydrate in place: the server's nodes stay, and a click updates them. It
+  # kills the server at the end. Playwright, pinned, is installed here only,
+  # into a scratch directory; nothing in `cargo test` or `nova` installs a
+  # package. Ubuntu only: one browser job suffices.
   browser:
     name: Browser (Chromium)
     runs-on: ubuntu-latest
@@ -115,8 +121,13 @@ jobs:
       - uses: actions/setup-node@v4
         with:
           node-version: 22
-      - name: cargo build -p nova-cli
-        run: cargo build --locked -p nova-cli
+      # `nova-runtime` too: cargo leaves `libnova_runtime.a` in
+      # `target/debug/`, beside `nova`, only when the runtime is a package
+      # the build names; as `nova-cli`'s dependency it stays in `deps/`. A
+      # debug `nova` carries no runtime, and finds it there, to link the
+      # native server `ssr.mjs` builds with `nova build --ssg`.
+      - name: cargo build -p nova-cli -p nova-runtime
+        run: cargo build --locked -p nova-cli -p nova-runtime
       - name: Build the counters for WASM
         run: |
           target/debug/nova build --target wasm tests/dom/counter.nova -o "$RUNNER_TEMP/counter"
@@ -135,6 +146,8 @@ jobs:
         run: |
           node tests/browser/dev.mjs target/debug/nova "$RUNNER_TEMP/dev" "$RUNNER_TEMP/pw"
           node tests/browser/router.mjs target/debug/nova "$RUNNER_TEMP/router" "$RUNNER_TEMP/pw"
+      - name: SSR and SSG in Chromium
+        run: node tests/browser/ssr.mjs target/debug/nova "$RUNNER_TEMP/ssr" "$RUNNER_TEMP/pw"
 
   fmt:
     name: Rustfmt
diff --git a/tests/browser/ssr-app/nova.toml b/tests/browser/ssr-app/nova.toml
new file mode 100644
index 0000000000000000000000000000000000000000..ad552c66edae9d07d05054f692a91f04281c073b
--- /dev/null
+++ b/tests/browser/ssr-app/nova.toml
@@ -0,0 +1,5 @@
+[package]
+name = "ssr-app"
+version = "0.1.0"
+edition = "2026"
+description = "A project with a client and a server, which tests/browser/ssr.mjs builds with --ssg, serves and hydrates in Chromium"
diff --git a/tests/browser/ssr-app/src/app.nova b/tests/browser/ssr-app/src/app.nova
new file mode 100644
index 0000000000000000000000000000000000000000..686f2ef1372cff1f4e05407791ed2327fe961a7f
--- /dev/null
+++ b/tests/browser/ssr-app/src/app.nova
@@ -0,0 +1,163 @@
+//! What the client, `main.nova`, and the server, `server.nova`, share (spec
+//! docs/superpowers/specs/2026-10-19-phase-4-6-ssr-hydration-and-ssg-design.md
+//! §1, §9.5): the server's data, the routes, and the app. The app shows the
+//! data's title and names, a `for_each` over them, then links, then the
+//! route the location matches: a home page with a counter and a `show`, an
+//! about page, a post by its id, and a page for any other path.
+//!
+//! `tests/browser/ssr.mjs` builds it with `nova build --ssg`, serves it with
+//! its server, and serves the statically generated pages with a server of
+//! its own; in each case the client hydrates the HTML in place. The root's
+//! `data-state` reads `server` in the HTML and `hydrated` once the client's
+//! `on_mount` has run, which happens after hydration's walk: the script
+//! waits for that.
+
+import ui
+import ui::html::{button, div, h1, h2, li, nav, p, ul}
+import ui::router::{HistoryRouter, Route, link, use_params}
+
+/// The server's data, which reaches the client as JSON. The title holds
+/// what a `script` element's text must not, `</script>`, `<!--` and `&`,
+/// so the page's escaping is tested in the browser.
+pub record Data { title: String, names: [String] }
+
+impl ToJson for Data {
+    fn to_json(self) -> JsonValue {
+        let mut names = [Null; self.names.len()]
+        let mut i = 0
+        while i < self.names.len() {
+            names[i] = self.names[i].to_json()
+            i = i + 1
+        }
+        let mut fields: Map<String, JsonValue> = Map::new()
+        fields.insert("title", self.title.to_json())
+        fields.insert("names", Array(names))
+        Object(fields)
+    }
+}
+
+// The string `v` holds, or an error naming `what`.
+fn string_in(v: Option<JsonValue>, what: String) -> Result<String, JsonError> {
+    match v {
+        Some(x) => String::from_json(x)
+        None => Err(JsonError { msg: "expected ${what}", at: 0 })
+    }
+}
+
+impl FromJson for Data {
+    fn from_json(v: JsonValue) -> Result<Data, JsonError> {
+        let fields = match v {
+            Object(m) => m
+            _ => return Err(JsonError { msg: "expected an object", at: 0 })
+        }
+        let title = match string_in(fields.get("title"), "a title") {
+            Ok(t) => t
+            Err(e) => return Err(e)
+        }
+        let list = match fields.get("names") {
+            Some(x) => {
+                match x {
+                    Array(xs) => xs
+                    _ => return Err(JsonError { msg: "expected names", at: 0 })
+                }
+            }
+            None => return Err(JsonError { msg: "expected names", at: 0 })
+        }
+        let mut names = [""; list.len()]
+        let mut i = 0
+        while i < list.len() {
+            names[i] = match string_in(Some(list[i]), "a name") {
+                Ok(name) => name
+                Err(e) => return Err(e)
+            }
+            i = i + 1
+        }
+        Ok(Data { title: title, names: names })
+    }
+}
+
+/// The data the server renders every page with.
+pub fn data() -> Data {
+    Data { title: "Guests </script><!-- & friends", names: ["Ada", "Grace", "Katherine"] }
+}
+
+/// The home page: a counter, and a `show` that a button hides and shows.
+component Home() {
+    let count = signal(0)
+    let open = signal(true)
+    view {
+        div {
+            h2(id: "page") { text("Home") }
+            p(id: "count") { text("Clicked ${count.value()} times") }
+            button(id: "add", on_click: || count.update(|n| n + 1)) { text("+") }
+            button(id: "toggle", on_click: || open.update(|o| !o)) { text("Toggle") }
+            show(
+                open,
+                || view {
+                    p(id: "details") { text("Details") }
+                },
+            )
+        }
+    }
+}
+
+component About() {
+    view {
+        div {
+            h2(id: "page") { text("About") }
+            p { text("A page rendered ahead of time.") }
+        }
+    }
+}
+
+/// A post by its id, a route with a parameter, which SSG skips.
+component Post() {
+    let id = use_params().get("id").unwrap_or("?")
+    view {
+        div { h2(id: "page") { text("Post ${id}") } }
+    }
+}
+
+component NotFound() {
+    view {
+        div {
+            h2(id: "page") { text("Not found") }
+            p { text("No route matches this path.") }
+        }
+    }
+}
+
+/// The routes: two static ones, one with a parameter, and the catch-all.
+pub fn routes() -> [Route] {
+    [
+        Route { path: "/", component: Home },
+        Route { path: "/about", component: About },
+        Route { path: "/posts/:id", component: Post },
+        Route { path: "*", component: NotFound },
+    ]
+}
+
+/// The app: the data's title and names, the links, then the route.
+pub component App(data: Data) {
+    let state = signal("server")
+    on_mount(|| state.set("hydrated"))
+    view {
+        div(id: "root", data_state: state.value()) {
+            h1(id: "title") { text(data.title) }
+            ul(id: "names") {
+                for_each(
+                    signal(data.names),
+                    |name: String| view {
+                        li { text(name) }
+                    },
+                )
+            }
+            nav {
+                link(id: "home", to: "/") { text("Home") }
+                link(id: "about", to: "/about") { text("About") }
+                link(id: "post", to: "/posts/2") { text("Post 2") }
+            }
+            HistoryRouter(routes())
+        }
+    }
+}
diff --git a/tests/browser/ssr-app/src/main.nova b/tests/browser/ssr-app/src/main.nova
new file mode 100644
index 0000000000000000000000000000000000000000..088741cea6ef9966cce3b1265bc15c0e3aa87755
--- /dev/null
+++ b/tests/browser/ssr-app/src/main.nova
@@ -0,0 +1,8 @@
+//! The client: it hydrates the page the server rendered, with the server's
+//! data, which it reads from the `nova-data` after the page's root; or, on a
+//! page with no data, as `nova dev` serves, with no title and no names.
+
+import app::{App, Data}
+import ui
+
+fn main() { hydrate_with("#app", Data { title: "", names: [] }, App) }
diff --git a/tests/browser/ssr-app/src/server.nova b/tests/browser/ssr-app/src/server.nova
new file mode 100644
index 0000000000000000000000000000000000000000..4cf839792dba34f8877d92a516d37a627f3d7cf5
--- /dev/null
+++ b/tests/browser/ssr-app/src/server.nova
@@ -0,0 +1,29 @@
+//! The server: it serves `dist/` on the address its first argument names,
+//! or on `127.0.0.1:0`, a port the system chooses, rendering each page with
+//! the data; given `--ssg <dir>`, as `nova build --ssg` runs it, it writes
+//! the static pages into `<dir>` instead. When `serve` fails, it prints the
+//! error and exits with status 1, so a refused page fails the build.
+
+import app::{App, data, routes}
+import ui
+import ui::ssr::{Page, Site, page_with, serve}
+
+fn render(location: String) -> Page { page_with(location, data(), App) }
+
+// The address to bind: the first argument, unless it is a flag such as
+// `--ssg`, or else `127.0.0.1:0`.
+fn address() -> String {
+    let given = args().get(1).unwrap_or("--")
+    if given.starts_with("--") { "127.0.0.1:0" } else { given }
+}
+
+async fn main() {
+    let site = Site { address: address(), dist: "dist", routes: routes(), render: render }
+    match serve(site).await {
+        Ok(_) => {}
+        Err(e) => {
+            eprintln(e.message)
+            exit(1)
+        }
+    }
+}
diff --git a/tests/browser/ssr.mjs b/tests/browser/ssr.mjs
new file mode 100644
index 0000000000000000000000000000000000000000..7506c370505b1f844982dc567db286fc6ee08edd
--- /dev/null
+++ b/tests/browser/ssr.mjs
@@ -0,0 +1,313 @@
+// The gate of 4.6 in Chromium (spec
+// docs/superpowers/specs/2026-10-19-phase-4-6-ssr-hydration-and-ssg-design.md
+// §9.5, the user's decision 11), run by CI's `browser` job:
+//
+//     node tests/browser/ssr.mjs <nova> <work dir> <dir holding node_modules/playwright>
+//
+// It copies `tests/browser/ssr-app`, a project with a client and a server,
+// into the work directory, emptied first, and builds it there with `nova
+// build --ssg`, with `NOVA_HOME` beside it: `dist/` holds the client, the
+// generated page titled `Nova`, and the pages SSG wrote for `/` and
+// `/about`, which `manifest.json` lists, and none for `/posts/:id`. It starts
+// the server, `target/debug/ssr-app-server`, which binds `127.0.0.1:0`, and
+// reads the port from the line `ssr: serving http://127.0.0.1:<port>/`.
+//
+// A page "hydrates in place" when, opened with `app.wasm` held back
+// (`page.route`), its `#root` reads `data-state="server"`, and every node
+// under `#app`, elements, texts and comments, is kept as a handle; then,
+// once `app.wasm` loads and the client's `on_mount` sets `data-state` to
+// `hydrated`, which it does after hydration's walk, each handle is `===` the
+// node at its place in the page, with no node more or less. Through
+// Playwright:
+// 1. `/` answers `200` and hydrates in place; a click on `+` updates the
+//    count's own text node, kept from the server; the `show` hides and
+//    shows again;
+// 2. a link goes to `/about`, and a marker on `window` that a reload would
+//    lose is still there;
+// 3. `/posts/2` answers `200` and hydrates in place, showing the post;
+// 4. `/nowhere` answers `404` with the "not found" page, which hydrates in
+//    place too;
+// 5. a static Node server, as `counter.mjs`'s, serves `dist/`; it answers
+//    `/about/` with `dist/about/index.html`, which already holds the
+//    rendered page and its data, and that page hydrates in place.
+// On every page the title the server's data carries, which holds
+// `</script>`, `<!--` and `&`, reads as written; the console has no error
+// but the one Chromium logs for `/nowhere`'s own `404`, and the page throws
+// none. The summary line ends with the client's size, `app.wasm` and
+// `app.js` gzipped, printed and not limited (spec §12). The server is
+// killed at the end, pass or fail. Every wait polls, for 30 seconds at
+// most. Playwright is the job's own install, as for `counter.mjs`.
+
+import { spawn } from "node:child_process";
+import { createServer } from "node:http";
+import { cp, mkdir, readFile, rm, stat } from "node:fs/promises";
+import { createRequire } from "node:module";
+import { dirname, extname, join, resolve, sep } from "node:path";
+import { createInterface } from "node:readline";
+import { fileURLToPath } from "node:url";
+import { gzipSync } from "node:zlib";
+
+const [nova, work, modules] = process.argv.slice(2).map((p) => resolve(p));
+const { chromium } = createRequire(join(modules, "noop.js"))("playwright");
+const repo = resolve(dirname(fileURLToPath(import.meta.url)), "..", "..");
+const LIMIT = 30000;
+const TITLE = "Guests </script><!-- & friends";
+
+// Runs `command` in `dir` to its end: its status and its output.
+function run(command, args, dir, env) {
+  return new Promise((done, failed) => {
+    const child = spawn(command, args, { cwd: dir, env, stdio: ["ignore", "pipe", "pipe"] });
+    let output = "";
+    child.stdout.on("data", (chunk) => {
+      output += chunk;
+    });
+    child.stderr.on("data", (chunk) => {
+      output += chunk;
+    });
+    child.on("error", failed);
+    child.on("exit", (code) => done({ code, output }));
+  });
+}
+
+// The server in `dir`: its URL, its stderr so far, and a kill.
+async function startServer(dir, env) {
+  const exe = process.platform === "win32" ? ".exe" : "";
+  const child = spawn(join(dir, "target", "debug", `ssr-app-server${exe}`), [], {
+    cwd: dir,
+    env,
+    stdio: ["ignore", "pipe", "pipe"],
+  });
+  let stderr = "";
+  child.stderr.on("data", (chunk) => {
+    stderr += chunk;
+  });
+  const server = { url: null, stderr: () => stderr, kill: () => child.kill() };
+  try {
+    server.url = await new Promise((found, failed) => {
+      const timer = setTimeout(() => failed(new Error("no URL within 30 s")), LIMIT);
+      createInterface({ input: child.stdout }).on("line", (line) => {
+        const url = /^ssr: serving (http:\/\/127\.0\.0\.1:\d+\/)$/.exec(line)?.[1];
+        if (url) {
+          clearTimeout(timer);
+          found(url);
+        }
+      });
+      child.on("exit", (code) => failed(new Error(`the server exited with ${code}`)));
+    });
+  } catch (e) {
+    child.kill();
+    throw new Error(`${e.message}; the server's stderr:\n${stderr}`);
+  }
+  return server;
+}
+
+// A static server of `dist/`'s files, and nothing outside it, as
+// `counter.mjs`'s: a path ending in `/` names its directory's `index.html`.
+const types = {
+  ".html": "text/html",
+  ".js": "text/javascript",
+  ".json": "application/json",
+  ".wasm": "application/wasm",
+};
+async function startStatic(dist) {
+  const server = createServer(async (req, res) => {
+    const path = decodeURIComponent(new URL(req.url, "http://127.0.0.1").pathname);
+    const file = resolve(join(dist, path.endsWith("/") ? `${path}index.html` : path));
+    if (file !== dist && !file.startsWith(dist + sep)) {
+      res.writeHead(403).end();
+      return;
+    }
+    try {
+      const body = await readFile(file);
+      res.writeHead(200, { "content-type": types[extname(file)] ?? "application/octet-stream" });
+      res.end(body);
+    } catch {
+      res.writeHead(404).end();
+    }
+  });
+  await new Promise((ready) => server.listen(0, "127.0.0.1", ready));
+  return { url: `http://127.0.0.1:${server.address().port}/`, close: () => server.close() };
+}
+
+// Polls `test(arg)` in the page until it holds.
+async function until(page, test, arg, what) {
+  const deadline = Date.now() + LIMIT;
+  for (;;) {
+    try {
+      if (await page.evaluate(test, arg)) return;
+    } catch {
+      // The page is loading.
+    }
+    if (Date.now() > deadline) {
+      const now = await page.evaluate(() => document.getElementById("page")?.textContent).catch(() => "?");
+      throw new Error(
+        `timed out waiting for ${what}; the page's heading reads ${JSON.stringify(now)} at ${page.url()}`,
+      );
+    }
+    await new Promise((later) => setTimeout(later, 100));
+  }
+}
+
+const heading = (page) => page.evaluate(() => document.getElementById("page")?.textContent ?? null);
+const state = (page) => page.evaluate(() => document.getElementById("root")?.dataset.state ?? null);
+
+// Opens `url` with `app.wasm` held back, checks the server's HTML is there,
+// keeps a handle to each node under `#app`, lets `app.wasm` load, waits for
+// hydration, and checks every handle is still the page's node, in place.
+// Returns the response's status, the handles, and how many there are.
+async function hydratesInPlace(page, url, expected) {
+  let release;
+  const held = new Promise((go) => {
+    release = go;
+  });
+  await page.route("**/app.wasm", async (route) => {
+    await held;
+    await route.continue();
+  });
+  try {
+    const response = await page.goto(url, { waitUntil: "domcontentloaded" });
+    if ((await state(page)) !== "server") {
+      throw new Error(`${url}: before app.wasm loads, #root's data-state is ${await state(page)}, not "server"`);
+    }
+    if ((await heading(page)) !== expected) {
+      throw new Error(`${url}: the server's HTML shows ${JSON.stringify(await heading(page))}, not ${expected}`);
+    }
+    const kept = await page.evaluateHandle(() => window.novaNodes());
+    const count = await kept.evaluate((nodes) => nodes.length);
+    release();
+    await until(page, () => document.getElementById("root")?.dataset.state === "hydrated", null, `${url} to hydrate`);
+    const moved = await page.evaluate((nodes) => {
+      const now = window.novaNodes();
+      if (now.length !== nodes.length) return `${nodes.length} nodes became ${now.length}`;
+      const at = nodes.findIndex((node, i) => node !== now[i]);
+      return at < 0 ? null : `node ${at}, ${nodes[at].nodeName}, is not the server's`;
+    }, kept);
+    if (moved) throw new Error(`${url} did not hydrate in place: ${moved}`);
+    if ((await heading(page)) !== expected) {
+      throw new Error(`${url}: the hydrated page shows ${JSON.stringify(await heading(page))}, not ${expected}`);
+    }
+    const title = await page.evaluate(() => document.getElementById("title")?.textContent);
+    if (title !== TITLE) throw new Error(`${url}: the data's title reads ${JSON.stringify(title)}`);
+    if ((await page.title()) !== "Nova") {
+      throw new Error(
+        `${url}: the page is titled ${JSON.stringify(await page.title())}, not the generated page's "Nova"`,
+      );
+    }
+    return { status: response.status(), kept, count };
+  } finally {
+    release();
+    await page.unroute("**/app.wasm");
+  }
+}
+
+const dir = join(work, "ssr-app");
+const dist = join(dir, "dist");
+const env = { ...process.env, NOVA_HOME: join(work, "nova-home"), NO_COLOR: "1" };
+await rm(work, { recursive: true, force: true });
+await mkdir(work, { recursive: true });
+await cp(join(repo, "tests", "browser", "ssr-app"), dir, { recursive: true });
+
+const build = await run(nova, ["build", "--ssg"], dir, env);
+if (build.code !== 0) throw new Error(`nova build --ssg exited with ${build.code}:\n${build.output}`);
+const listed = Object.keys(JSON.parse(await readFile(join(dist, "manifest.json"), "utf8")).files);
+if (!listed.includes("about/index.html")) throw new Error(`the manifest lists ${listed.join(", ")}`);
+if (await stat(join(dist, "posts")).catch(() => null)) throw new Error("SSG wrote a page for /posts/:id");
+
+const server = await startServer(dir, env);
+let browser = null;
+let files = null;
+try {
+  browser = await chromium.launch();
+  const page = await browser.newPage();
+  const errors = [];
+  // The one error a `404` page logs, the browser's own for its document.
+  let missing = null;
+  page.on("console", (m) => {
+    if (m.type() !== "error") return;
+    if (m.location().url === missing && /status of 404/.test(m.text())) return;
+    errors.push(m.text());
+  });
+  page.on("pageerror", (e) => errors.push(String(e)));
+  const clean = (what) => {
+    if (errors.length > 0) throw new Error(`${what}: the console has errors: ${errors.join("; ")}`);
+  };
+  await page.addInitScript(() => {
+    window.novaNodes = () => {
+      const walker = document.createTreeWalker(document.getElementById("app"), NodeFilter.SHOW_ALL);
+      const nodes = [];
+      while (walker.nextNode()) nodes.push(walker.currentNode);
+      return nodes;
+    };
+  });
+  const path = () => new URL(page.url()).pathname;
+
+  // 1. `/`: in place, then a click counts in the server's own text node, and
+  // the `show` builds again between the server's markers.
+  const home = await hydratesInPlace(page, server.url, "Home");
+  if (home.status !== 200) throw new Error(`/ answered ${home.status}`);
+  await page.click("#add");
+  await until(page, () => document.getElementById("count")?.textContent === "Clicked 1 times", null, "the count");
+  const counted = await page.evaluate((nodes) => {
+    const p = document.getElementById("count");
+    return nodes.includes(p) && nodes.includes(p.firstChild) && p.firstChild.data === "Clicked 1 times";
+  }, home.kept);
+  if (!counted) throw new Error("the click did not update the server's nodes");
+  await page.click("#toggle");
+  await until(page, () => document.getElementById("details") === null, null, "the show to hide");
+  await page.click("#toggle");
+  await until(page, () => document.getElementById("details")?.textContent === "Details", null, "the show to show");
+  clean("/");
+
+  // 2. A link's click goes to `/about` without a reload.
+  await page.evaluate(() => {
+    window.novaSsrMarker = true;
+  });
+  await page.click("#about");
+  await until(page, () => document.getElementById("page")?.textContent === "About", null, "the about page");
+  if (path() !== "/about") throw new Error(`the URL is ${page.url()}, not /about`);
+  const marked = await page.evaluate(() => window.novaSsrMarker === true);
+  if (!marked) throw new Error("the link's click reloaded the page");
+  clean("the link to /about");
+
+  // 3. A route with a parameter, rendered by the server.
+  const post = await hydratesInPlace(page, new URL("/posts/2", server.url).href, "Post 2");
+  if (post.status !== 200) throw new Error(`/posts/2 answered ${post.status}`);
+  clean("/posts/2");
+
+  // 4. A path only the catch-all matches.
+  missing = new URL("/nowhere", server.url).href;
+  const nowhere = await hydratesInPlace(page, missing, "Not found");
+  if (nowhere.status !== 404) throw new Error(`/nowhere answered ${nowhere.status}, not 404`);
+  clean("/nowhere");
+
+  // 5. SSG's page for `/about`, from a static server.
+  files = await startStatic(dist);
+  const about = new URL("/about/", files.url).href;
+  const fetched = await (await fetch(about)).text();
+  if (fetched !== (await readFile(join(dist, "about", "index.html"), "utf8"))) {
+    throw new Error("the static server's /about/ is not dist/about/index.html");
+  }
+  if (!fetched.includes('<h2 id="page">About</h2>') || !fetched.includes('id="nova-data"')) {
+    throw new Error(`dist/about/index.html was not rendered:\n${fetched}`);
+  }
+  const generated = await hydratesInPlace(page, about, "About");
+  clean("SSG's /about/");
+
+  // The client's size, printed and not limited (spec §12, risk 4), measured
+  // as the size test measures 06's: each file gzipped at the best level.
+  let gzipped = 0;
+  for (const name of ["app.wasm", "app.js"]) {
+    gzipped += gzipSync(await readFile(join(dist, name)), { level: 9 }).length;
+  }
+
+  console.log(
+    `ssr: / hydrates in place (${home.count} nodes kept) and counts, a link to /about without a reload, /posts/2 in place (${post.count}), /nowhere 404 in place (${nowhere.count}), and SSG's /about/ arrives rendered and hydrates in place (${generated.count}); the client is ${gzipped} bytes gzipped`,
+  );
+} catch (e) {
+  console.error(`the server's stderr:\n${server.stderr()}`);
+  throw e;
+} finally {
+  await browser?.close();
+  files?.close();
+  server.kill();
+}
````


### Task 6: The records

Spec §11; decisions 29 to 39, which ADR 0045 records, with the user's
decision 13.

**Files:**
- Create: `docs/adr/0045-ssr-hydration-and-ssg.md`
- Modify:
  - `nova-spec/00-MASTER-SPEC.md` (Phase 4, items 8 and 9),
    `20-STDLIB.md` (§1, §6), `30-FRONTEND.md` (§4, §7.4), `40-TOOLING.md`
    (§1.1)
  - `docs/phase-4-plan.md` (4.6's entry), `CHANGELOG.md`, `ARCHITECTURE.md`

- [ ] **Step 1: The records.** `patch_block t6-code`.
- [ ] **Step 2: Read them against the code.** Each claim a record makes is
  one this plan's code makes true, and each departure ADR 0045 names is one
  of this plan's decisions:
  - the user's decisions and the spec's §14, against decisions 1 to 12;
  - the markers, the walk, the mismatch's words, the empty root and the
    cleanup, against `std/ui/lib.nova`'s `render`, `write_views`,
    `escape`, `root_of`, `hydrate_into`, `blank`, `found`, `adopt_attr`,
    `adopt_text` and `adopt_region`;
  - the data, its element, its escaping and the fallback, against
    `page_with`, `script_safe`, `document`, `hydrate_with` and
    `backend_data(root)`;
  - `serve`'s start, its errors, its answers and their headers, against
    `serve`, `answer`, `file` and `content_type`; the manifest's paths,
    against `listed`, `inside`, `plain_path` and `plain_piece`; the filled
    root, against `emptied` and `document`;
  - SSG's paths, its refusals and its lines, against `generate`,
    `file_pieces` and `ssg_dir`;
  - the router's functions, against `with_location`, `starting`, `found`,
    `catch_all` and `static_paths`; `std/http`'s, against `fallback`,
    `dispatch` and `listen_on`;
  - the CLI's refusals, their order and words, the server's path,
    `--ssg`'s run, the pages and the exit, against `build_ssr`, `generate`,
    `page_in`, `ended` and `Bundle::add_pages`; the import, against
    `IMPORTED_STD` and `missing_names`;
  - the consequences: `hydrate_with`'s fallback under `nova dev`, and the
    mismatch a placeholder root logs there on each load, the data's key
    order, a route ending in `/`, a name Windows reserves, a
    template comment that holds `</div>`, a server that must report
    `serve`'s `Err`; `40-TOOLING.md`'s failing server, `with exit code
    <code>` or `stopped by signal <number>`, and 1 for the second
    (decisions 25 and 37); the phase plan's 4.6 amendment, whose
    `hydrate_with` uses a fallback when the page holds no data. The replay
    found that amendment silent on the fallback, and the prototype's
    `9a6ae03 4.6 prototype: records` added it; the second check found the
    records over-claiming what `page_in` checks ("no `.`", "read as
    written"), and `23a6e93` rewords them: Rust's path components, a `.`
    piece and a trailing `/` normalised away. The patch holds both;
  - the figures, against the runs: 21,070 bytes for `06-counter-spa`,
    which Task 7 measures by `flate2` at level 9, as the test does;
    41,260 bytes for `ssr-app`'s client, which Task 5's line printed.

  A record that says what the code does not is fixed here, and the ledger
  says so.
- [ ] **Step 3: Run.** `cargo test -p nova-std`. Expected: `2 passed`. Then
  `cargo build --locked --workspace`. Expected: it ends with `Finished`.
- [ ] **Step 4: The checks,** as the conventions list them.
- [ ] **Step 5: Commit** `docs: ADR 0045, and the notes for 4.6`.

**Patch `t6-code`:**

<!-- patch t6-code -->

````diff
diff --git a/ARCHITECTURE.md b/ARCHITECTURE.md
index cecd2b2f772df6efed2ab63bb93ff3896e2515cb..cff7b021808611e4868a9e8b5e04b8e495479293 100644
--- a/ARCHITECTURE.md
+++ b/ARCHITECTURE.md
@@ -33,7 +33,7 @@ nova-mir            → Mid-level IR
 
 | Crate | Purpose |
 |---|---|
-| `nova-cli` | Binary entry point, CLI argument parsing; a WASM build's bundle: the page, `assets/`, `manifest.json` and `wasm-opt` (ADR 0043); `nova dev`: its builds in a child process, its watcher, and its server on `std::net` with the event stream that reloads a page or shows its overlay (ADR 0044) |
+| `nova-cli` | Binary entry point, CLI argument parsing; a WASM build's bundle: the page, `assets/`, `manifest.json` and `wasm-opt` (ADR 0043); `nova dev`: its builds in a child process, its watcher, and its server on `std::net` with the event stream that reloads a page or shows its overlay (ADR 0044); `nova build --ssr` and `--ssg`: a project's client and its server, the server's run that writes the static pages, and those pages in the manifest (ADR 0045) |
 | `nova-driver` | Orchestrates the compilation pipeline |
 | `nova-diagnostics` | Shared error reporting infrastructure, and the fixes a diagnostic suggests |
 | `nova-lexer` | Source → tokens (uses `logos`) |
@@ -47,7 +47,7 @@ nova-mir            → Mid-level IR
 | `nova-codegen-llvm` | Optimized release backend |
 | `nova-codegen-wasm` | WebAssembly backend: MIR to one module with `wasm-encoder`, structured control flow, and the JS shim (ADR 0039) |
 | `nova-runtime` | GC, async runtime, panic handling |
-| `nova-std` (in `std/`) | The standard library's `.nova` sources, embedded for the compiler; `std/rt` is a WASM build's runtime, in Nova: its allocator, collector and executor (ADRs 0039, 0040); `std/dom`, a WASM build's page, which a program imports (ADR 0041); `std/ui` and `std/ui/html`, reactivity and views, which a program imports, and `std/ui`'s backend, one source per target (ADR 0042); `std/ui/router`, routing by the URL's hash or path, which a program imports (ADR 0044) |
+| `nova-std` (in `std/`) | The standard library's `.nova` sources, embedded for the compiler; `std/rt` is a WASM build's runtime, in Nova: its allocator, collector and executor (ADRs 0039, 0040); `std/dom`, a WASM build's page, which a program imports (ADR 0041); `std/ui` and `std/ui/html`, reactivity and views, which a program imports, and `std/ui`'s backend, one source per target (ADR 0042); `std/ui/router`, routing by the URL's hash or path, which a program imports (ADR 0044); `std/ui/ssr`, server-side rendering and SSG, which a native program imports (ADR 0045) |
 | `nova-fmt` | The formatter: prints the AST in one fixed layout, keeps every comment, and refuses output that would change the program (ADR 0028); organize imports' block |
 | `nova-lsp` | The language server, `nova lsp`: diagnostics, completion, formatting, hover, definition, references, rename, code actions and semantic tokens over `nova_driver::analyze` (ADR 0029, ADR 0032, ADR 0033) |
 | `nova-test` | A stub, `publish = false`; the test runner is `nova-cli`'s `cmd/test.rs` |
diff --git a/CHANGELOG.md b/CHANGELOG.md
index 6127b4999c1ac0cf505023b06c66d1b562ff9d4f..fcdd9f3213b94d36914dc17752b4c1513a907846 100644
--- a/CHANGELOG.md
+++ b/CHANGELOG.md
@@ -87,6 +87,34 @@ Nova uses [Semantic Versioning](https://semver.org/spec/v2.0.0.html).
   whose `Host` is not the server's, is refused (ADR 0044).
 - **`Event.modified()`** in `std/dom`: whether Ctrl, Meta, Shift or Alt was
   down, or a mouse button other than the main one was pressed.
+- **`nova build --ssr` and `--ssg`:** a project with `src/server.nova`
+  beside `src/main.nova` builds its client into `dist/` and its server
+  into `target/debug/<name>-server`; `--ssg` then runs the server once,
+  which writes `dist/<path>/index.html` for each route with no parameter,
+  already rendered, and lists the pages in `manifest.json` (ADR 0045).
+- **Hydration:** `render_hydratable(f)` writes a view as HTML a client can
+  adopt, and `hydrate("#app", App)` adopts it in the page: the server's
+  nodes stay, and their effects and handlers start.
+  `hydrate_with("#app", fallback, App)` reads the server's data, written as
+  JSON into the page just after its root, with `FromJson`, or uses
+  `fallback` when the page holds none, as under `nova dev`. A page whose
+  nodes differ from the view says so with `console.error`, and is built
+  again (ADR 0045).
+- **`std/ui/ssr`,** which a native program imports with `import ui::ssr`:
+  `serve(Site { … })` serves a built `dist/` and renders each other `GET`
+  into its `index.html`, with `404` for a path no route but `*` serves;
+  `page`, `page_with` and `document` render a page to a string, for a
+  server written by hand (ADR 0045).
+- **`with_location`, `found` and `static_paths`** in `std/ui/router`:
+  render a router at a given path, tell whether a route other than `*`
+  serves a path, and list the routes with no parameter.
+- **`Server::fallback` and `Server::listen_on`** in `std/http`: a handler
+  for every `GET` no route matches, and serving a listener the program has
+  already bound, such as one on port 0.
+- **`console_error`** in `std/dom`.
+- **A CI test of SSR and SSG in Chromium,** `tests/browser/ssr.mjs`: a
+  server-rendered page and a statically generated one hydrate without
+  making their nodes again.
 
 ### Changed
 
diff --git a/docs/adr/0045-ssr-hydration-and-ssg.md b/docs/adr/0045-ssr-hydration-and-ssg.md
new file mode 100644
index 0000000000000000000000000000000000000000..498307c275793dd1ae98a6a687ce035de0221f42
--- /dev/null
+++ b/docs/adr/0045-ssr-hydration-and-ssg.md
@@ -0,0 +1,413 @@
+# ADR 0045 — SSR, hydration and SSG
+
+## Status
+
+Accepted, 2026-10-19 (Phase 4.6, branch
+`claude/loving-archimedes-frtf5q-46`; spec
+`docs/superpowers/specs/2026-10-19-phase-4-6-ssr-hydration-and-ssg-design.md`).
+Two entries in one project; a walk in order over comment markers; a
+mismatch reported and the view built again; no hash seed shared; the built
+`dist/index.html` as the page; SSG of the static routes by the serving
+function; two levels of serving in a native-only std module; typed data
+through JSON; the names `hydrate`, `ui::ssr`, `serve` and `page`; `nova
+dev` unchanged; the gate and its tests; and `404` with the rendered page
+are the user's decisions of 2026-10-19 (decisions 1 to 12 below).
+`hydrate_with`'s fallback is the user's decision of 2026-10-20, after the
+prototype's fact-check (decision 13), and adds a parameter to the spec's
+API. The spec settled the rest (its §14).
+
+## Context
+
+After 4.5 a program could route in the page and run under `nova dev`, but
+it rendered only in the browser. `nova-spec/30-FRONTEND.md` §7.4 sketches
+`nova build --ssr`, "emit native binary that renders HTML", and `nova
+build --ssg`, "pre-render at build time", with the same component code on
+a server that renders to a string and a client that "attaches to existing
+DOM, replays effects". `00-MASTER-SPEC.md` §1.8 puts "SSR/SSG: built-in
+flags on `nova build`" among the points not up for debate. ADR 0042 said a
+new renderer "walks the same `View`", and ADR 0044 left rendering a given
+path natively, and serving a rendered program, to 4.6. The repository 4.6
+starts from (spec §2, read at `2b7a6ed`) has:
+- `render_to_string`, which writes no marker for a `show` or a `for_each`
+  and runs two texts side by side together, so a browser's parse of its
+  HTML has fewer nodes than `mount` builds;
+- a WASM backend that cannot read a node's first child, kind or text, and
+  a `std/dom` with no way to write to the console;
+- a router that reads its location from the backend when it is made,
+  natively always `/`;
+- a `Map` that iterates in table order, seeded at random in each process,
+  natively and on WASM alike;
+- `std/http`'s `Server`, which routes a `GET` by its exact target, answers
+  anything else `not_found()`, and binds inside `listen`, printing nothing;
+- a project whose program is `src/main.nova` alone, and a `nova build`
+  that takes `--target`, `--release` and `-o`;
+- a WASM build that keeps only what `main` reaches.
+
+## Decision
+
+### The user's decisions
+
+1. **Two entries, one project.** `src/main.nova` is the client, built for
+   WASM; `src/server.nova` is the server, built natively; what both run
+   lives in a module of the project, such as `src/app.nova`, which each
+   imports with `import app::{…}`. `nova build --ssr` builds both.
+2. **Hydration walks the nodes in order.** The server writes comment
+   markers around each `show` and `for_each` region, and between two texts
+   side by side; elements get no attribute for it.
+3. **A mismatch** is reported with `console.error`, naming the first
+   difference; the server's nodes under the root are dropped, and the view
+   is built afresh. The tests assert that their pages hydrate with no
+   mismatch.
+4. **No seed is shared** between server and client, since sending the
+   server's hash seed to every visitor would defeat its HashDoS
+   protection. A view must not show a `Map`'s order: it sorts first. The
+   mismatch check catches a view that forgets.
+5. **The page is the built `dist/index.html`,** generated or the
+   project's own. The server and SSG put the rendered HTML inside its
+   `<div id="app"></div>`; the client calls `hydrate("#app", App)`. The
+   head, the title and the scripts stay in that one file for both.
+6. **SSG renders the static routes,** those with no `:name` piece that are
+   not the catch-all. `nova build --ssg` builds the client and the server,
+   then runs the server once, and the std function that serves writes
+   `dist/<path>/index.html` for each static route instead. Routes with
+   parameters are skipped.
+7. **std serves at two levels,** in a std module only a native program can
+   import: one function, `serve`, that serves `dist/`'s files and renders
+   every other `GET` as a page; and functions that render the page at one
+   path to a string, for a server written by hand, as 4.7's blog will be.
+   `POST`, cookies and path parameters stay 4.7's.
+8. **The server's data reaches the client typed, through JSON:** a
+   `ToJson` value, escaped, in a `<script type="application/json">`, which
+   the client reads back with `FromJson` and hydrates with.
+9. **The names:** `hydrate` in `std/ui` beside `mount`, with
+   `hydrate_with` for the server's data; `import ui::ssr`, native only, as
+   `dom` is WASM only; `ssr::serve` the high level and `ssr::page` the low.
+10. **`nova dev` does not change;** it stays client only. In an SSR
+    project its page has an empty root, where `hydrate` builds the view.
+    4.7, or the backlog, decides on a development mode for SSR.
+11. **The gate and the tests:** `tests/browser/ssr-app`, with a router, a
+    `show`, a `for_each`, a counter and the server's data; in CI,
+    Playwright opens a server-rendered page and a statically generated
+    one, clicks, and checks that the server's nodes were not made again;
+    in `cargo test`, the fake DOM parses the HTML `std/ui` writes, and
+    hydration and its mismatches are tested. No example changes; 07 is
+    4.7's. `tests/browser/ssr.mjs` builds the app with `--ssg`, opens `/`,
+    `/posts/2` and `/nowhere` from its server and SSG's `/about/` from a
+    static server, each with `app.wasm` held back, keeps a handle to every
+    node under `#app`, and checks that each is `===` the page's node once
+    the app's `on_mount`, which runs after the walk, has set its root's
+    `data-state` from `server` to `hydrated`.
+12. **A path no route serves gets `404`** with the rendered page, when it
+    falls to `*` or matches no route; the router says which route a path
+    matches.
+13. **`hydrate_with` takes a fallback** (2026-10-20):
+    `hydrate_with("#app", Data { names: [] }, App)`. When the page holds no
+    data from the server, it builds or hydrates with the fallback the
+    program gives, so `nova dev` shows the page (decision 10). The spec's
+    `hydrate_with(selector, f)` becomes `hydrate_with(selector, fallback,
+    f)`.
+
+### Decided in the spec (§14)
+
+1. **`render_to_string` does not change.** Hydratable HTML has its own
+   renderer, `render_hydratable(f)` in `std/ui`, beside the walk that
+   reads it. It writes what `render_to_string` writes, with three
+   additions.
+2. **The markers** are `<!--[-->` and `<!--]-->` around a region's views,
+   and `<!--|-->` between two text views side by side, siblings among an
+   element's children or a region's views; an empty text writes nothing,
+   but keeps its separators. A carriage return, in text or in an
+   attribute's value, is written `&#13;`, since the browser's parser turns
+   a raw one into a line feed. No marker can come from a program's text,
+   which is escaped.
+3. **The walk checks tags and texts, not attributes.** `hydrate(selector,
+   f)` walks `f()`'s view against the root's child nodes under a root
+   owner it keeps, as `mount` keeps one. An element must have the view's
+   tag, compared in lower case, and is adopted: `Static` attributes are
+   neither written nor checked, `Property` is set, `Dynamic` and
+   `DynProperty` get their effects, whose first run writes the value
+   again, and `On` is listened for; then its children are walked, and no
+   node may be left after them. A text node must hold the view's text, a
+   `DynText`'s as its effect's first run reads it; where the view's text
+   is empty and no text node is there, an empty one is inserted. A region
+   must start at `<!--[-->`; its first run adopts its views' nodes, and
+   its later runs build between the two markers as under `mount`, which
+   would have made the same two nodes; it must end at `<!--]-->`.
+   `on_mount` callbacks run after the walk. At the first difference (a
+   wrong tag, a text that differs, a missing marker or node, a node left
+   over), the walk stops, `nova: hydration mismatch: expected <p>, found
+   <div>; building the view again` goes to `console.error`, each side
+   read as `<tag>`, `text "…"` (its first 40 characters), `a region`, `the
+   end of a region` or `nothing`, the walk's owner is disposed, the root's
+   children are removed, and the view is built there as `mount` builds
+   it, calling `f` again; `on_mount` callbacks queued under the disposed
+   owner never run.
+4. **An empty root mounts with no report:** no child, or only whitespace
+   text, as `nova dev`'s page and a page served without SSR have. Its
+   whitespace is removed first, so the view is not built after it.
+5. **The data's element** is `<script type="application/json"
+   id="nova-data">`, just after the root, and the root's id is `app`.
+   `hydrate_with(selector, fallback, f)` reads the data from the root's
+   next element sibling alone, past any text or comment between them, when
+   it is a `script` whose `id` is `nova-data` and whose `type` is
+   `application/json`, as `document` writes it: an element of the view with
+   the id `nova-data`, inside the root, is never the data, nor is any
+   element after the first. It parses that element's text, makes a `T` with
+   `T::from_json`, and hydrates `|| f(data)`. With no such element, it
+   hydrates `|| f(fallback)` (decision 13): an empty root gets the view
+   built, and a filled one is walked with it. Data that is there but does
+   not parse still panics, `std/ui: hydrate_with: the page's data does not
+   parse: <error>`. `page_with` writes
+   `stringify(data.to_json())` with `<`, `>` and `&` as `\u003c`, `\u003e`
+   and `\u0026`, and U+2028 and U+2029 as `\u2028` and `\u2029`: those
+   characters occur only inside JSON's strings, so the text parses back to
+   the same value, and no `</script>` or `<!--` in the data can end or
+   change its element.
+6. **The router's server functions** are `with_location(location, f)`,
+   under which a router made starts at `location` and which nests and
+   restores; `found(routes, location)`, true when the first route that
+   matches is not the catch-all; and `static_paths(routes)`, the static
+   routes' paths in order, each once, as written.
+7. **`ui::ssr`'s names** are `Page { body, data }`, `page(location, f)`,
+   `page_with(location, data, f)`, `document(template, page)`, `Site {
+   address, dist, routes, render }` and `serve(site)`. `Site.render` maps a
+   location to a `Page`, so each page can choose its own data. `document`
+   empties the template's first `<div id="app">`, as `serve` and SSG empty
+   it, puts the body into it and the data's element just after it, and
+   panics with `ssr: the page has no <div id="app">` when there is none.
+8. **`serve` serves only the files `manifest.json` lists,** answers only
+   `GET`, and applies the `.` rule `nova dev` uses. It reads the template
+   and the manifest when it starts, returning an `Err` naming a file it
+   cannot read; binds `site.address`; and prints `ssr: serving
+   http://<host>:<port>/` with the port it bound, so a test binds port 0. A
+   request's path, before any `?`, percent-decoded, less its leading `/`,
+   is a listed file (`200`, a `Content-Type` by its extension and
+   `X-Content-Type-Options: nosniff`), or else a `404` when its last piece
+   holds a `.`, or else a page: `document(template, render(target))`, the
+   target with its query, `200` when `found` says a route serves it and
+   `404` otherwise. No file outside the manifest is ever read; and a
+   manifest's path with a piece that is empty, `.` or `..`, or that holds
+   `\` or NUL, as SSG refuses a route's path, is dropped when the manifest
+   is read, so a hand-written manifest listing `../secret` serves nothing
+   outside `dist`. SIGINT or SIGTERM stops it, and it returns `Ok(())`.
+9. **`std/http` gains `Server::fallback(handler)`,** which answers a `GET`
+   no route matches, and `Server::listen_on(listener)`, which serves a
+   listener the program bound; `listen` is `bind`, then `listen_on`, as
+   before.
+10. **SSG is `serve` run with `--ssg <dir>`:** it binds nothing, reads
+    `<dir>/index.html` as the template, writes `document(template,
+    render(path))` for each of `static_paths`, `/` over `<dir>/index.html`
+    and any other path to `<dir>/<its pieces>/index.html`, printing `ssg:
+    wrote <file>` for each, and refuses a path with a piece that is empty,
+    `.` or `..`, or holds `\`, NUL, `?` or `#`, returning an `Err` naming it
+    and writing nothing more. The CLI adds the pages to the manifest. A
+    server written by hand is pre-rendered too if its `main` calls `serve`
+    when given `--ssg`.
+11. **The server's executable** is `target/<profile>/<name>-server`. `-o`
+    names the client's directory only. `nova build --ssr` refuses a file
+    argument, `--target`, a library, and a project with no
+    `src/server.nova`, before anything is built; `--ssr --ssg` is `--ssg`;
+    `--release` applies to both builds. `nova build --ssg` runs the server
+    from the project's root with `--ssg <dist>`, its output passing
+    through, and a failure fails the build.
+12. **Mismatches are reported through `std/dom::console_error`,** new,
+    with its shim function. The WASM backend gains `backend_first_child`,
+    `backend_kind`, `backend_text_of`, `backend_report` and
+    `backend_data(root)`, which reads the root's next element sibling;
+    natively each panics with `mount`'s message, and so do
+    `hydrate` and `hydrate_with`. The resolver registers `ui::ssr` as an
+    imported std module natively and refuses its import on WASM with
+    E0092; `nova-doc` lists it; the driver builds a project's program
+    rooted at `src/server.nova`, with its modules and dependencies.
+
+## What departs from the spec, or fills it in
+
+- **`hydrate`'s panic for a selector that matches nothing names the
+  function,** `std/ui: hydrate: no element matches "…"`, or
+  `hydrate_with:`, where spec §4.1 has `mount`'s words.
+- **`hydrate_with` panics only for data that does not parse.** Spec §4.1's
+  panic for a page with no data, `std/ui: hydrate_with: the page has no
+  data from the server`, is gone: such a page uses the fallback (decision
+  13). The spec's `backend_data()`, "the text of `#nova-data`", the first
+  element with that id in the page, is `backend_data(root)`, the root's
+  next element sibling, so that a view's own element with that id cannot
+  stand for the data.
+- **The mismatch's words:** a `<!--|-->` found is read as the text after
+  it, which the server wrote there; a comment that is no marker is `a
+  comment`, a reading spec §4.3 does not list; and a missing separator
+  reads `expected text "<the next text>", found …`.
+- **The whitespace of an empty root** is HTML's: space, tab, LF, CR and
+  FF, and it is removed before the view is built. The `<error>` of
+  `hydrate_with`'s panic for data that does not parse is the `JsonError`'s
+  message, whether `parse` or `from_json` refused the text.
+- **The disposed walk's owner** is removed from the roots `std/ui` keeps,
+  so nothing holds it after the view is built again.
+- **The fake DOM's parser** turns CR and CRLF into LF, as a browser does,
+  so `&#13;` is tested under it; it throws on an entity it does not know,
+  and on HTML it cannot parse, such as an end tag it does not expect.
+- **A filled root is a template too.** After `--ssg`, `dist/index.html`
+  is the rendered `/`, and a project's own page may hold whitespace or a
+  placeholder in its root. So `document`, which `serve` and SSG fill each
+  page with, empties the root, and drops a data element after it, just
+  after or past whitespace, where the client would read it, before filling
+  it, where spec §6.2's `document` replaced the first
+  `<div id="app"></div>`, and panicked on a template with none, one whose
+  root holds anything included. `serve` and SSG
+  empty the template's root once when they read it, and refuse a template
+  with no `<div id="app">`, `ssr: <dist>/index.html has no <div
+  id="app">`, or `ssg: …`. The root's end is the `</div>` that balances
+  it; a root no `</div>` balances keeps what follows it, after an empty
+  root. A data element with no `</script>` stays where it is, since
+  nothing tells where its text ends: such a template is broken HTML, whose
+  rest a browser reads as that script's text. So SSG can run again over
+  its own output, and a server started after `--ssg` renders each page
+  from the page it was built with.
+- **`serve`'s own errors:** an address it cannot bind, `ssr: cannot bind
+  <address>: …`; a manifest with no `files` object; and `--ssg` with no
+  directory after it, `ssg: --ssg needs a directory`. A listed file that
+  cannot be read when asked for gets `404`. It reads a file through the
+  builtins `std/fs` is written on, since a handler cannot await.
+- **The `nosniff` header is written `x-content-type-options`,** in lower
+  case, as `std/http` writes every header name.
+- **A fifth refusal:** `--ssr` or `--ssg` outside a project, "builds a
+  project, and there is no nova.toml here or above". The spec gave only
+  the text for a missing `src/server.nova`; the CLI words the others
+  itself: "builds the project's src/main.nova and src/server.nova; it
+  takes no file", "builds the client for wasm and the server natively; it
+  takes no `--target`", and the library refusal `nova build` already
+  makes.
+- **The pages `--ssg` lists** are the files the server's `ssg: wrote
+  <file>` lines name, which the CLI reads from the server's stdout as it
+  passes it through; then every file the manifest lists is measured again,
+  since the root's page replaced `index.html`. A line's file, read from
+  the server's working directory, the project's root, if relative, must
+  lie in the output directory, made absolute the same way, so the answer
+  does not hang on where `nova` runs; and each name below it must be a
+  plain one by Rust's path components, with no `..`, root or prefix, while
+  a `.` piece and a trailing `/` are normalised away, and nothing is asked
+  of the file system. Otherwise, once the server has ended well, the build
+  fails with ``the server's line `…` names no file inside
+  <dist>``, and the manifest lists none of its pages. When the server
+  fails, `nova` prints `Error: the server failed to generate the pages,
+  with exit code <code>: <server>` and exits with that code; when a signal
+  stops it, which leaves it no code, `…, stopped by signal <number>: …`,
+  and exits with 1. These words are the same on every system, where
+  `ExitStatus`'s own are not.
+- **On WASM, a name of `ui::ssr` with no import** is E0001, as natively:
+  the resolver leaves an imported module's names out of the list of names
+  WASM lacks, since no module sees them without the import, which is
+  E0092 itself.
+- **`with_location` and `found` drop a `#fragment`,** as a router's
+  location never holds one, so the page a location renders and its status
+  agree; `found` takes any pattern whose only piece is `*` for the
+  catch-all, `/*` included.
+
+## What departs from `30-FRONTEND.md` §7.4 and the master spec
+
+- **`--ssr` does not emit a renderer:** it builds the project's own
+  server, `src/server.nova`, which calls `ui::ssr::serve` or renders with
+  `page` in handlers of its own, beside the client.
+- **`--ssg` runs that server** once after the build, and pre-renders only
+  the routes with no parameter; the master spec's Phase 4 item 9,
+  "compile-time route enumeration", becomes `static_paths` over the
+  program's own routes when the built server runs, and no list of paths
+  can be given.
+- **The server renders with `render_hydratable`,** not `render_to_string`
+  (master spec item 8), so that the browser's parse has each node `mount`
+  would make.
+- **Hydration adopts the server's nodes and starts their effects;** it
+  does not replay them over the page. A `DynText`'s first run is compared
+  with the text it adopts, a dynamic attribute's first run writes it
+  again, and a mismatch builds the view anew.
+- **The server's data travels in the page,** a step §7.4 does not name.
+- **SSR and SSG stay flags on `nova build`,** as §1.8 has them, and `nova
+  dev` does not change.
+
+## Consequences
+
+- **The known causes of a mismatch,** each of which builds the page again
+  and says so in the console: HTML the browser's parser rebuilds, such as
+  a `div` inside a `p` or a `tr` without a `tbody`; a `script` or `style`
+  element's text, which the parser does not decode; a NUL in text, which
+  the parser drops; a line feed that begins a `pre`'s or a `textarea`'s
+  text, which the parser drops too; and a value that differs between the
+  two renders, such as the time, a random number, or a `Map`'s order
+  (decision 4).
+- **The risks:**
+  - the browser's parser can build another tree from the HTML; the page
+    still works, since the mismatch builds it again, but the server's work
+    is wasted;
+  - a render that is not deterministic mismatches every time: programs
+    pass such values as data, and sort;
+  - `render` is synchronous, so a page cannot await its data; 4.6's comes
+    from memory, and loading per request is 4.7's question;
+  - hydration adds code to every client that calls `hydrate`;
+    `06-counter-spa` does not, and its size test reads 21,070 bytes
+    gzipped, the 21 KB 4.4 recorded. `ssr-app`'s client, built without
+    `--release` and so without `wasm-opt`, measured 41,260 bytes gzipped
+    when `ssr.mjs` printed it; its size is printed, not limited;
+  - a server is a second program to build natively, slow under
+    `--release`.
+- **Under `nova dev`, `hydrate_with` builds the page from its fallback**
+  (decision 13): the page holds no `nova-data`, its root is empty, and the
+  client builds `f(fallback)` there, as `hydrate` builds its view, so an
+  SSR project with data still runs under `nova dev`, without the server's
+  data, until 4.7, or the backlog, gives `nova dev` a mode for SSR. A page
+  a server filled with no data after the root is walked with the fallback,
+  and mismatches where the server rendered other data. A project's own
+  page whose root holds a placeholder is not empty either: under `nova
+  dev` the client walks the placeholder, reports a mismatch with
+  `console.error` on every load, and builds the view, which works.
+- **A hash `Router` is not for SSR:** a server never sees the hash, so it
+  renders `/`, and a page opened at `#/posts/1` mismatches and is built
+  again. `HistoryRouter` is the router for SSR.
+- **The data is public:** it is written into the page for every visitor.
+- **An object's keys in the data** come out in its `Map`'s order, which
+  each process seeds anew, so two `--ssg` builds, or two servers, can
+  write the same value with its keys in another order. The client reads
+  the same value; the pages are not byte for byte the same.
+- **After `--ssg`, `/index.html` and `/about/index.html` are files** the
+  manifest lists, served as SSG wrote them, while `/` and `/about` are
+  rendered on each request. A file added to `dist/` after the build is not
+  served until the server starts again.
+- **A static route written with an empty piece,** such as `/about/`, is
+  refused by SSG, and fails `nova build --ssg`.
+- **SSG does not refuse a name Windows reserves or alters,** such as
+  `CON`, a piece that ends in a dot, or one that holds `:`: on Windows such
+  a route's page may fail to be written, or land under another name. It
+  is still written inside `<dir>`, since a piece that is empty, `.` or
+  `..`, or holds `\`, is refused.
+- **A comment in the template can fool the root's emptying,** which counts
+  `<div` and `</div>` and reads no comment: one inside the root holding
+  `</div>` ends the root there, and the rest of the root stays in the
+  page. A view cannot, since its text and attribute values are escaped,
+  and `render_hydratable` writes no comment but its markers.
+- **A server must report `serve`'s `Err`,** print it and exit with a
+  failure, or an SSG refusal passes silently; `tests/browser/ssr-app`'s
+  server does.
+- **No crate is added,** and `Cargo.lock` does not change.
+
+## What is left out
+
+- A development mode for SSR in `nova dev` (decision 10).
+- Partial hydration, or islands: 4.7 decides whether the blog's comments
+  need more than whole-page hydration.
+- Streaming HTML, and asynchronous data in `render`.
+- `POST`, cookies and path parameters in `std/http`, which are 4.7's.
+- Pre-rendering routes with parameters, and a list of paths given by the
+  program.
+- Checking attributes during hydration.
+- A keyed `for_each`.
+- A `nova serve` command.
+- `nova check` of `src/server.nova` as a second root.
+
+## References
+
+- `docs/phase-4-plan.md` §3, decisions 10 and 11, §4's 4.6 entry, risk 5,
+  and §5's security rules.
+- The spec named above.
+- ADR 0042, `std/ui`; ADR 0043, the bundle; ADR 0044, the router and
+  `nova dev`; ADR 0022, the shutdown signals `serve` inherits.
+- `nova-spec/30-FRONTEND.md` §4 and §7.4; `nova-spec/40-TOOLING.md` §1.1;
+  `nova-spec/20-STDLIB.md` §1 and §6; `nova-spec/00-MASTER-SPEC.md` §1.8
+  and Phase 4.
diff --git a/docs/phase-4-plan.md b/docs/phase-4-plan.md
index 187c10b919043407789c0ca440361ee95440a41d..1821c7cd3a5c667e02d18fac8a8cdf200a9ecda6 100644
--- a/docs/phase-4-plan.md
+++ b/docs/phase-4-plan.md
@@ -568,6 +568,27 @@ ADR 0044).
     without re-creating its nodes.
   - `--ssg` writes one file per static route.
 
+**Amended 2026-10-19 (branch `claude/loving-archimedes-frtf5q-46`):** by the
+user's decisions of 2026-10-19, a project has two entries, `src/main.nova`
+for the client and `src/server.nova` for the server, sharing a module;
+`nova build --ssr` builds both, and `--ssg` then runs the server once,
+which writes a page for each static route. The server writes
+`render_hydratable`'s HTML, with comment markers around regions and between
+adjacent texts, into the built `dist/index.html`, through the native-only
+`ui::ssr`: `serve` at the high level, `page` at the low. The client's
+`hydrate` walks the nodes in order, and on a mismatch reports it and builds
+the view again; data passes as JSON, read back by `hydrate_with`, which
+uses a fallback the program gives when the page holds none, as under `nova
+dev` (the user's decision 13, of 2026-10-20); no hash seed is shared, so a
+view sorts a `Map`'s keys first; a path no route but `*` serves gets `404`
+with its page; and `nova dev` does not change. `std/http` gains
+`Server::fallback` and `listen_on`. The gate is `tests/browser/ssr-app` in
+Chromium (`tests/browser/ssr.mjs`): a server-rendered page and a page SSG
+wrote each hydrate in place, the server's nodes kept and updated by a
+click
+(`docs/superpowers/specs/2026-10-19-phase-4-6-ssr-hydration-and-ssg-design.md`;
+ADR 0045).
+
 ### 4.7 — `examples/07-fullstack-blog`
 - **The blog as `60-EXAMPLES.md` §7 describes it,** with a JSON file
   (decision 11):
diff --git a/nova-spec/00-MASTER-SPEC.md b/nova-spec/00-MASTER-SPEC.md
index 1377fa24f001f288d6e73ffb6886800cca63dd89..638c3d753e9ccf4424ce3d7586ff9043fa0ca96d 100644
--- a/nova-spec/00-MASTER-SPEC.md
+++ b/nova-spec/00-MASTER-SPEC.md
@@ -682,7 +682,22 @@ Implement std modules in order (each module is a doc in [20-STDLIB.md]):
    whose state starts again, by the user's decision of 2026-10-14 (ADRs
    0038 and 0044).
 8. SSR: render to string in native runtime, hydrate in WASM
+   **Amended 2026-10-19 (branch `claude/loving-archimedes-frtf5q-46`):**
+   `nova build --ssr` builds a project's client for WASM and its server,
+   `src/server.nova`, natively. The server renders with
+   `render_hydratable`, which marks regions and adjacent texts with
+   comments, and serves through `ui::ssr::serve`, or through handlers of
+   its own that call `ui::ssr::page`; the client calls
+   `hydrate`, which adopts the server's nodes in order, or builds the view
+   again after reporting a mismatch, by the user's decisions of 2026-10-19
+   (ADR 0045).
 9. SSG: compile-time route enumeration
+   **Amended 2026-10-19 (branch `claude/loving-archimedes-frtf5q-46`):**
+   the routes are enumerated when the built server runs, not by the
+   compiler: `nova build --ssg` runs the server once with `--ssg <dist>`,
+   and `serve` writes a page for each route with no `:name` piece that is
+   not `*`; routes with parameters are skipped, by the user's decision 6
+   of 2026-10-19 (ADR 0045).
 
 **Gate:** `examples/06-counter-spa` and `examples/07-fullstack-blog` work in browser, pass Lighthouse 95+.
 
diff --git a/nova-spec/20-STDLIB.md b/nova-spec/20-STDLIB.md
index c5470b62a9f0b07ad3e166de596eb07e14fd7e9e..b0bab0a270bc4f41e195a3d3b3a39b508b9dccd9 100644
--- a/nova-spec/20-STDLIB.md
+++ b/nova-spec/20-STDLIB.md
@@ -65,6 +65,18 @@ only nested import path. Their functions are listed in
 `docs/superpowers/specs/2026-10-17-phase-4-3-std-ui-reactivity-and-views-design.md`
 §5 to §7 (ADR 0042).
 
+**Amended 2026-10-19 (branch `claude/loving-archimedes-frtf5q-46`):
+`std/ui/ssr`**, a native build's only, is imported with `import ui::ssr`,
+and on WASM that import is E0092, as `import dom` is natively. It holds
+`Page`, `page`, `page_with` and `document`, which render the page at a
+location to a string, and `Site` and `serve`, which serve a built `dist/`
+and render each other `GET` as a page, or, given `--ssg <dir>`, write the
+static routes' pages. `std/ui` gains `render_hydratable`, `hydrate` and
+`hydrate_with`; `std/ui/router` gains `with_location`, `found` and
+`static_paths`; and `std/dom` gains `console_error`. They are listed in
+`docs/superpowers/specs/2026-10-19-phase-4-6-ssr-hydration-and-ssg-design.md`
+§3 to §6 and §8 (ADR 0045).
+
 ---
 
 ## 2. `std/core` — Foundational Types
@@ -524,6 +536,19 @@ narrower than the code block below.** `std/http` now has `Server` with
 
 See `docs/superpowers/specs/2026-10-04-examples-03-http-server-design.md`.
 
+**Amended 2026-10-19 (branch `claude/loving-archimedes-frtf5q-46`):**
+`Server` gains two methods. `fallback(handler)` returns a copy of the
+server whose `handler` answers each `GET` no route matches, in place of
+`Response::not_found()`; any other method still gets `not_found`, and a
+second `fallback` replaces the first. `listen_on(listener)` serves a
+`TcpListener` the program has already bound, such as one on port 0 whose
+`local_port` it prints first, with `listen`'s connections, limits and stop
+on SIGTERM or SIGINT; `listen(addr)` is now `bind`, then `listen_on`, and
+behaves as before. `ui::ssr`'s `serve` is built on both. `post`, cookies
+and path parameters are still not shipped
+(`docs/superpowers/specs/2026-10-19-phase-4-6-ssr-hydration-and-ssg-design.md`
+§8.3; ADR 0045).
+
 ```nova
 module std.http
 
diff --git a/nova-spec/30-FRONTEND.md b/nova-spec/30-FRONTEND.md
index 046041c5a1b35a085fd184ba9f429d03bd72efa7..6537e2bb45ed0a31c9ee4cf745cff3be6a1e875a 100644
--- a/nova-spec/30-FRONTEND.md
+++ b/nova-spec/30-FRONTEND.md
@@ -177,6 +177,18 @@ signals change. A value that is not a `String`, or a handler of another
 type, is E0097. A positional argument is an `Attr`, so 4.3's helpers still
 work. Named arguments exist only there (`docs/superpowers/specs/2026-10-17-phase-4-4-component-syntax-design.md` §4; ADR 0043).
 
+**Amended 2026-10-19 (branch `claude/loving-archimedes-frtf5q-46`):** a
+view must not show a `Map`'s order. Each process seeds its `Map`s at
+random, natively and on WASM, and no seed passes from a server to its
+client, since that would hand every visitor the server's HashDoS seed; so
+a server's render and the client's hydration can iterate one `Map` in
+two orders. A view sorts a `Map`'s `keys()` before it shows them, and
+takes a value that differs between two renders, such as the time or a
+random number, as the server's data; hydration reports a view that forgets
+as a mismatch, and builds it again
+(`docs/superpowers/specs/2026-10-19-phase-4-6-ssr-hydration-and-ssg-design.md`
+decision 4; ADR 0045).
+
 ---
 
 ## 5. DOM Bindings (`std/dom`)
@@ -366,6 +378,37 @@ Same component code runs on:
 
 Hydration: client downloads WASM, attaches to existing DOM, replays effects.
 
+**Amended 2026-10-19 (branch `claude/loving-archimedes-frtf5q-46`):** 4.6
+builds both flags for a project with two entries: `src/main.nova`, the
+client, and `src/server.nova`, the server, which share components through
+a module such as `src/app.nova`. `nova build --ssr` builds the client into
+`dist/` as `--target wasm` does, and the server natively into
+`target/debug/<name>-server`; `--ssg` then runs the server once with
+`--ssg <dist>`, which writes `dist/<path>/index.html` for each route with no
+`:name` piece that is not `*`, and lists those pages in `manifest.json`.
+The server is the project's own program: it imports `ui::ssr`, native
+only, and calls `serve(Site { address, dist, routes, render })`, which
+serves `dist/`'s listed files and renders any other `GET` into the built
+`index.html`'s `<div id="app"></div>`, with `404` where no route but `*`
+serves the path;
+`page`, `page_with` and `document` render a page to a string for a server
+written by hand. The server writes `render_hydratable`'s HTML, which marks
+each `show` and `for_each` with `<!--[-->` and `<!--]-->` and two texts side
+by side with `<!--|-->`, and the client calls `hydrate("#app", App)`, or
+`hydrate_with("#app", fallback, App)` for data the server wrote as JSON
+in a `<script type="application/json" id="nova-data">` just after the root,
+or the `fallback` the program gives when the page holds none. Hydration
+walks the view against the page's nodes in order and adopts them, checking
+tags and texts but not attributes, and starts their effects and handlers
+rather than replaying them; at the first difference it reports `nova:
+hydration mismatch: …` with `console.error`, empties the root and builds
+the view afresh. An empty root is built into with no report, as under
+`nova dev`, which does not change; there `hydrate_with` builds the page
+from its fallback (the user's decision 13, of 2026-10-20). A hash `Router`
+is not for SSR; `HistoryRouter` is
+(`docs/superpowers/specs/2026-10-19-phase-4-6-ssr-hydration-and-ssg-design.md`
+§3 to §7; ADR 0045).
+
 ---
 
 ## 8. Dev Server (`nova dev`)
diff --git a/nova-spec/40-TOOLING.md b/nova-spec/40-TOOLING.md
index a53472d1398ed2894e0a2c7c1d030b3df919feed..ed905959bedd0c2060c4ba2ca08fe87f7a0f6169 100644
--- a/nova-spec/40-TOOLING.md
+++ b/nova-spec/40-TOOLING.md
@@ -93,6 +93,32 @@ choose another with --port` and status 1
 (`docs/superpowers/specs/2026-10-18-phase-4-5-router-and-nova-dev-design.md`
 §4; ADR 0044).
 
+**Amended 2026-10-19 (branch `claude/loving-archimedes-frtf5q-46`):** `nova
+build --ssr` and `nova build --ssg` exist, for a project with
+`src/server.nova` beside `src/main.nova`. `--ssr` builds the client from
+`src/main.nova` as `--target wasm` does, into `dist/` or the directory `-o`
+names, and the server from `src/server.nova` natively, into
+`target/debug/<name>-server`, or `target/release/<name>-server` with
+`--release`, which applies to both; it prints `built <dist>` and `built
+<server>`. `--ssg` builds both, then runs the server from the project's
+root with `--ssg <dist>`, its output passing through, and adds the pages it
+wrote, its `ssg: wrote <file>` lines, to `manifest.json`. A line's file,
+read from the project's root if relative, must lie inside `<dist>`, made
+absolute the same way, with each name below it a plain one by Rust's path
+components: no `..`, root or prefix, while a `.` piece and a trailing `/`
+are normalised away. Any other line fails the build, once the server has
+ended well. A server that fails ends `nova` with its exit code, after
+`Error: the server failed to generate the pages, with exit code <code>:
+<server>`; one a signal stops, which has no exit code, ends it with status
+1, after `…, stopped by signal <number>: <server>`. `--ssr --ssg` is
+`--ssg`. Each refuses, before building anything, a file argument,
+`--target`, a directory in no project, a library, and a project with no
+`src/server.nova`: "`--ssr` needs src/server.nova, the server's entry".
+`nova dev`, `nova run`, `nova check` and `nova bundle` do not change; `nova
+run src/server.nova` runs the server
+(`docs/superpowers/specs/2026-10-19-phase-4-6-ssr-hydration-and-ssg-design.md`
+§7; ADR 0045).
+
 ### 1.2 Project Template (`nova new`)
 
 ```
````


### Task 7: Verification, the mutants, the review, the PR

- [ ] **Step 1: The full checks.** First, `df -h /` must show at least 8G
  free; if not, remove scratch directories of earlier sub-phases.

  ```sh
  cargo fmt --all -- --check
  cargo clippy --locked --all-targets --all-features -- -D warnings
  cargo run --locked -q -p nova-cli -- fmt --check std examples tests/wasm tests/dom tests/browser/router-app tests/browser/ssr-app tests/ssr
  cargo run --locked -q -p nova-cli -- fmt --check tests/runtime/ui_hydratable.nova tests/runtime/ui_router_server.nova tests/runtime/http_listen_on.nova
  cargo build --locked --workspace
  NOVA_REQUIRE_NODE=1 cargo test --locked --workspace --no-fail-fast > "$P/full-46x.txt" 2>&1
  grep '^test result' "$P/full-46x.txt" | awk '{p += $4; f += $6; i += $8} END {print p, f, i}'
  grep ' FAILED$' "$P/full-46x.txt"
  W the_counter_spa_is_under_50_kb_gzipped --nocapture
  ```

  Expected:
  - both formatters print nothing, and clippy ends with `Finished`, with
    no warning or error;
  - the suite, in about 4 minutes: `2390 1 10`, against the baseline's
    `2364 1 10`: 26 new tests:
    - 7 in `wasm_tests`: `gate::ui_hydratable`, `stress::ui_hydratable`,
      `gate::ui_router_server`, `hydrate_adopts_the_servers_nodes`,
      `hydrate_reports_a_mismatch_and_builds_again`,
      `hydrate_panics_without_its_root_or_on_data_that_does_not_parse` and
      `ssr_is_imported_by_name_and_only_natively`;
    - 6 in `run_tests`: `ui_hydrate_panics_natively`, `http_listen_on_run`,
      `ssr_page_page_with_and_document`, `ssr_serve_answers_files_and_pages`,
      `ssr_serve_reports_what_stops_it_starting` and
      `ssr_ssg_writes_the_static_pages_and_refuses_a_bad_path`, while
      `ui_hydratable` and `ui_router_server` join
      `ui_programs_run_and_survive_gc_stress`;
    - 8 in `ssr_tests`, new;
    - 3 in `nova-cli`'s binary:
      `bundle::tests::ssg_pages_join_the_manifest_with_their_sizes`,
      `cmd::run::tests::a_page_lies_in_dist_with_plain_names` and
      `cmd::run::tests::a_failed_server_is_told_by_its_code_or_its_signal`;
    - one each in `nova-resolver`
      (`tests::ssr_is_imported_by_name_and_only_natively`) and
      `nova-driver`'s `packages` (`a_server_is_rooted_at_src_server_nova`).

    `doc_test`, `index_complete`, `nova-doc`'s library and `nova-std` keep
    their counts, and run more: `std/ui`'s, `std/ui/router`'s,
    `std/http`'s and `std/ui/ssr`'s new examples run inside
    `doc_std_runs_stds_doc_tests_inside_a_project_or_not`;
  - the one failure is Task 0's IPv6 test;
  - the size: `1 passed`, and it prints `06-counter-spa: 21070 bytes
    gzipped`, by `flate2` at level 9, against 51,200, as at the base: 06
    does not call `hydrate`, so the WASM build keeps none of it. Record it
    in the ledger.
- [ ] **Step 2: The lockfile.** `git diff --numstat a5c0335 -- Cargo.lock`.
  Expected: nothing.
- [ ] **Step 3: Rust 1.78.**

  ```sh
  RUSTFLAGS="-D warnings" CARGO_TARGET_DIR="$P/msrv-46" cargo +1.78 check --locked --workspace
  ```

  Expected: clean, in about a minute (the replay's check took 33 seconds). It
  checks the libraries and binaries, not the tests, as 4.5's did:
  `assert_cmd` 2.2.1's manifest does not parse on 1.78. Then `rm -rf
  "$P/msrv-46"`.
- [ ] **Step 4: The mutants.** One at a time, each edit is made, its tests
  run, and the file restored with `git checkout -- <file>`.
  - Each edit replaces one exact text, which occurs once in its file; "the
    line goes" removes the whole line holding it.
  - Each must fail the tests named. A mutant that passes them is a gap:
    record it in the ledger, and write the missing test before the
    review.
  - At the end, `git status --short` prints nothing.

  Each command runs one test binary with one filter or none: cargo's
  filters are joined by "or", so a second name would widen the run, not
  narrow it. Each names the tests that fail in its run; the rest of that
  run passes. A mutant of `std/` needs nothing more: `cargo test` rebuilds
  the embedded std. Mutant 39's texts hold a `\u` escape: make that
  edit with a script (Global Constraints). The replay ran all 91 in about
  14 minutes, each failing through a wrong output, a panic or a wrong exit
  code, never through a build that broke (88 and 89 print Rust's `unused
  variable` warning, and build): a run that prints a compile error, Rust's
  or Nova's, differs from the replay, so check the edit.

  1. **A region opens with another comment.** `std/ui/lib.nova`:
     `if marks { o.push("<!--[-->") }` becomes `if marks { o.push("<!--(-->") }`.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests ui_programs_run_and_survive_gc_stress`.
     - Expected: fails: `ui_programs_run_and_survive_gc_stress`.
  2. **A region closes with another comment.** `std/ui/lib.nova`:
     `if marks { o.push("<!--]-->") }` becomes `if marks { o.push("<!--)-->") }`.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests ui_programs_run_and_survive_gc_stress`.
     - Expected: fails: `ui_programs_run_and_survive_gc_stress`.
  3. **Two texts side by side get no separator.** `std/ui/lib.nova`:
     `if marks && after_text && text { o.push("<!--|-->") }`: the line goes.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests ui_programs_run_and_survive_gc_stress`.
     - Expected: fails: `ui_programs_run_and_survive_gc_stress`.
  4. **A carriage return is written raw.** `std/ui/lib.nova`:
     `} else if marks && c == '\r' {` becomes `} else if false && c == '\r' {`.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests ui_programs_run_and_survive_gc_stress`.
     - Expected: fails: `ui_programs_run_and_survive_gc_stress`.
  5. **`render_to_string` escapes a carriage return too.** `std/ui/lib.nova`:
     `} else if marks && c == '\r' {` becomes `} else if c == '\r' {`.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests ui_programs_run_and_survive_gc_stress`.
     - Expected: fails: `ui_programs_run_and_survive_gc_stress`.
  6. **`render_to_string` writes the markers.** `std/ui/lib.nova`:
     replace

     ```
     pub fn render_to_string(f: fn() -> View) -> String { render(f, false) }
     ```

     with

     ```
     pub fn render_to_string(f: fn() -> View) -> String { render(f, true) }
     ```

     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests ui_programs_run_and_survive_gc_stress`.
     - Expected: fails: `ui_programs_run_and_survive_gc_stress`.
  7. **An empty root is walked, not built.** `std/ui/lib.nova`:
     `if !blank(parent) {` becomes `if true {`.
     - Run `CARGO_INCREMENTAL=0 NOVA_REQUIRE_NODE=1 cargo test -p nova-cli --test wasm_tests hydrate_adopts_the_servers_nodes`.
     - Expected: fails: `hydrate_adopts_the_servers_nodes`.
  8. **Only a space is whitespace in an empty root.** `std/ui/lib.nova`:
     `let space = c == ' ' || c == '\t' || c == '\n' || c == '\r'` becomes `let space = c == ' '`.
     - Run `CARGO_INCREMENTAL=0 NOVA_REQUIRE_NODE=1 cargo test -p nova-cli --test wasm_tests hydrate_adopts_the_servers_nodes`.
     - Expected: fails: `hydrate_adopts_the_servers_nodes`.
  9. **Any element is adopted, whatever its tag.** `std/ui/lib.nova`:
     `if backend_kind(el) == name {` becomes `if true {`.
     - Run `CARGO_INCREMENTAL=0 NOVA_REQUIRE_NODE=1 cargo test -p nova-cli --test wasm_tests hydrate_reports_a_mismatch_and_builds_again`.
     - Expected: fails: `hydrate_reports_a_mismatch_and_builds_again`.
  10. **A text is adopted, whatever it holds.** `std/ui/lib.nova`:
     `if backend_text_of(n) != t {` becomes `if false {`.
     - Run `CARGO_INCREMENTAL=0 NOVA_REQUIRE_NODE=1 cargo test -p nova-cli --test wasm_tests hydrate_reports_a_mismatch_and_builds_again`.
     - Expected: fails: `hydrate_reports_a_mismatch_and_builds_again`.
  11. **A dynamic text keeps no node it adopted.** `std/ui/lib.nova`:
     `m.node = adopt_text(got.text, joined, w, c)` becomes `let _ = adopt_text(got.text, joined, w, c)`.
     - Run `CARGO_INCREMENTAL=0 NOVA_REQUIRE_NODE=1 cargo test -p nova-cli --test wasm_tests hydrate_adopts_the_servers_nodes`.
     - Expected: fails: `hydrate_adopts_the_servers_nodes`.
  12. **An empty text's new node is not put in the page.** `std/ui/lib.nova`:
     `put(c.parent, n, c.at)`: the line goes.
     - Run `CARGO_INCREMENTAL=0 NOVA_REQUIRE_NODE=1 cargo test -p nova-cli --test wasm_tests hydrate_adopts_the_servers_nodes`.
     - Expected: fails: `hydrate_adopts_the_servers_nodes`.
  13. **The separator before a text is not stepped over.** `std/ui/lib.nova`:
     `if joined {` becomes `if false {`.
     - Run `CARGO_INCREMENTAL=0 NOVA_REQUIRE_NODE=1 cargo test -p nova-cli --test wasm_tests hydrate_adopts_the_servers_nodes`.
     - Expected: fails: `hydrate_adopts_the_servers_nodes`.
  14. **A region's first run does not walk its views.** `std/ui/lib.nova`:
     `with_context(Some(render), None, || walk_views(views, w, c))`: the line goes.
     - Run `CARGO_INCREMENTAL=0 NOVA_REQUIRE_NODE=1 cargo test -p nova-cli --test wasm_tests hydrate_adopts_the_servers_nodes`.
     - Expected: fails: `hydrate_adopts_the_servers_nodes`.
  15. **A region's later runs build nothing.** `std/ui/lib.nova`:
     `Some(e) => build_region(f, c.parent, start, e)` becomes `Some(e) => {}`.
     - Run `CARGO_INCREMENTAL=0 NOVA_REQUIRE_NODE=1 cargo test -p nova-cli --test wasm_tests hydrate_adopts_the_servers_nodes`.
     - Expected: fails: `hydrate_adopts_the_servers_nodes`.
  16. **A node left in an element is no difference.** `std/ui/lib.nova`:
     `left_over(w, inner)`: the line goes.
     - Run `CARGO_INCREMENTAL=0 NOVA_REQUIRE_NODE=1 cargo test -p nova-cli --test wasm_tests hydrate_reports_a_mismatch_and_builds_again`.
     - Expected: fails: `hydrate_reports_a_mismatch_and_builds_again`.
  17. **A node left in the root is no difference.** `std/ui/lib.nova`:
     `left_over(w, c)`: the line goes.
     - Run `CARGO_INCREMENTAL=0 NOVA_REQUIRE_NODE=1 cargo test -p nova-cli --test wasm_tests hydrate_reports_a_mismatch_and_builds_again`.
     - Expected: fails: `hydrate_reports_a_mismatch_and_builds_again`.
  18. **A `Property` is not set on an adopted element.** `std/ui/lib.nova`:
     replace

     ```
             Static(name, value) => {}
             _ => build_attr(a, el)
     ```

     with

     ```
             Static(name, value) => {}
             Property(name, value) => {}
             _ => build_attr(a, el)
     ```

     - Run `CARGO_INCREMENTAL=0 NOVA_REQUIRE_NODE=1 cargo test -p nova-cli --test wasm_tests hydrate_adopts_the_servers_nodes`.
     - Expected: fails: `hydrate_adopts_the_servers_nodes`.
  19. **A dynamic attribute gets no effect.** `std/ui/lib.nova`:
     replace

     ```
             Static(name, value) => {}
             _ => build_attr(a, el)
     ```

     with

     ```
             Static(name, value) => {}
             Dynamic(name, f) => {}
             _ => build_attr(a, el)
     ```

     - Run `CARGO_INCREMENTAL=0 NOVA_REQUIRE_NODE=1 cargo test -p nova-cli --test wasm_tests hydrate_adopts_the_servers_nodes`.
     - Expected: fails: `hydrate_adopts_the_servers_nodes`.
  20. **A dynamic text's later runs write nothing.** `std/ui/lib.nova`:
     `Some(n) => backend_set_text(n, t)` becomes `Some(n) => {}`.
     - Run `CARGO_INCREMENTAL=0 NOVA_REQUIRE_NODE=1 cargo test -p nova-cli --test wasm_tests hydrate_adopts_the_servers_nodes`.
     - Expected: fails: `hydrate_adopts_the_servers_nodes`.
  21. **An adopted element's handlers are not added.** `std/ui/lib.nova`:
     replace

     ```
             Static(name, value) => {}
             _ => build_attr(a, el)
     ```

     with

     ```
             Static(name, value) => {}
             On(kind, f) => {}
             _ => build_attr(a, el)
     ```

     - Run `CARGO_INCREMENTAL=0 NOVA_REQUIRE_NODE=1 cargo test -p nova-cli --test wasm_tests hydrate_adopts_the_servers_nodes`.
     - Expected: fails: `hydrate_adopts_the_servers_nodes`.
  22. **`on_mount` runs during the walk.** `std/ui/lib.nova`:
     replace

     ```
             let w = Walk { failed: false, message: "" }
             s.building = s.building + 1
     ```

     with

     ```
             let w = Walk { failed: false, message: "" }
     ```

     - Run `CARGO_INCREMENTAL=0 NOVA_REQUIRE_NODE=1 cargo test -p nova-cli --test wasm_tests hydrate_adopts_the_servers_nodes`.
     - Expected: fails: `hydrate_adopts_the_servers_nodes`.
  23. **A text found is reported without its quotes.** `std/ui/lib.nova`:
     replace

     ```
                 if kind == "#text" { return "text \"${clipped(backend_text_of(n))}\"" }
     ```

     with

     ```
                 if kind == "#text" { return "text ${clipped(backend_text_of(n))}" }
     ```

     - Run `CARGO_INCREMENTAL=0 NOVA_REQUIRE_NODE=1 cargo test -p nova-cli --test wasm_tests hydrate_reports_a_mismatch_and_builds_again`.
     - Expected: fails: `hydrate_reports_a_mismatch_and_builds_again`.
  24. **A separator found is reported as a comment.** `std/ui/lib.nova`:
     `if data != "|" { return "a comment" }` becomes `if true { return "a comment" }`.
     - Run `CARGO_INCREMENTAL=0 NOVA_REQUIRE_NODE=1 cargo test -p nova-cli --test wasm_tests hydrate_reports_a_mismatch_and_builds_again`.
     - Expected: fails: `hydrate_reports_a_mismatch_and_builds_again`.
  25. **A text is clipped to 41 characters.** `std/ui/lib.nova`:
     replace

     ```
     fn clipped(t: String) -> String { if t.len() > 40 { t.slice(0, 40) } else { t } }
     ```

     with

     ```
     fn clipped(t: String) -> String { if t.len() > 41 { t.slice(0, 41) } else { t } }
     ```

     - Run `CARGO_INCREMENTAL=0 NOVA_REQUIRE_NODE=1 cargo test -p nova-cli --test wasm_tests hydrate_reports_a_mismatch_and_builds_again`.
     - Expected: fails: `hydrate_reports_a_mismatch_and_builds_again`.
  26. **The walk's owner is not disposed.** `std/ui/lib.nova`:
     replace

     ```
             batch(|| dispose(o))
             let mut live
     ```

     with

     ```
             let mut live
     ```

     - Run `CARGO_INCREMENTAL=0 NOVA_REQUIRE_NODE=1 cargo test -p nova-cli --test wasm_tests hydrate_reports_a_mismatch_and_builds_again`.
     - Expected: fails: `hydrate_reports_a_mismatch_and_builds_again`.
  27. **The root keeps the server's nodes.** `std/ui/lib.nova`:
     replace

     ```
         let mut emptied = false
         while !emptied {
     ```

     with

     ```
         let mut emptied = true
         while !emptied {
     ```

     - Run `CARGO_INCREMENTAL=0 NOVA_REQUIRE_NODE=1 cargo test -p nova-cli --test wasm_tests hydrate_reports_a_mismatch_and_builds_again`.
     - Expected: fails: `hydrate_reports_a_mismatch_and_builds_again`.
  28. **The view is not built again.** `std/ui/lib.nova`:
     replace

     ```
             backend_report("nova: hydration mismatch: ${w.message}; building the view again")
     ```

     with

     ```
             backend_report("nova: hydration mismatch: ${w.message}; building the view again")
             return
     ```

     - Run `CARGO_INCREMENTAL=0 NOVA_REQUIRE_NODE=1 cargo test -p nova-cli --test wasm_tests hydrate_reports_a_mismatch_and_builds_again`.
     - Expected: fails: `hydrate_reports_a_mismatch_and_builds_again`.
  29. **The data is read from a `script` whose id is `nova_data`.** `std/ui/backend/wasm.nova`:
     `let named = e.attribute("id").unwrap_or("") == "nova-data"` becomes `let named = e.attribute("id").unwrap_or("") == "nova_data"`.
     - Run `CARGO_INCREMENTAL=0 NOVA_REQUIRE_NODE=1 cargo test -p nova-cli --test wasm_tests hydrate_`.
     - Expected: fails: `hydrate_adopts_the_servers_nodes`, `hydrate_panics_without_its_root_or_on_data_that_does_not_parse`.
  30. **A page with no data panics, and the fallback is not used.** `std/ui/lib.nova`:
     replace

     ```
             None => fallback
     ```

     with

     ```
             None => panic("std/ui: hydrate_with: the page has no data from the server")
     ```

     - Run `CARGO_INCREMENTAL=0 NOVA_REQUIRE_NODE=1 cargo test -p nova-cli --test wasm_tests hydrate_adopts_the_servers_nodes`.
     - Expected: fails: `hydrate_adopts_the_servers_nodes`.
  31. **The parse panic says something else.** `std/ui/lib.nova`:
     replace

     ```
                     Err(e) => panic("std/ui: hydrate_with: the page's data does not parse: ${e.msg}")
     ```

     with

     ```
                     Err(e) => panic("std/ui: hydrate_with: bad JSON: ${e.msg}")
     ```

     - Run `CARGO_INCREMENTAL=0 NOVA_REQUIRE_NODE=1 cargo test -p nova-cli --test wasm_tests hydrate_panics_without_its_root_or_on_data_that_does_not_parse`.
     - Expected: fails: `hydrate_panics_without_its_root_or_on_data_that_does_not_parse`.
  32. **Natively no element matches, not mount's panic.** `std/ui/backend/native.nova`:
     `pub fn backend_query(selector: String) -> Option<Node> { no_page() }` becomes `pub fn backend_query(selector: String) -> Option<Node> { None }`.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests ui_hydrate_panics_natively`.
     - Expected: fails: `ui_hydrate_panics_natively`.
  33. **`with_location` does not restore the location.** `std/ui/router/lib.nova`:
     `s.location = was`: the line goes.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests ui_programs_run_and_survive_gc_stress`.
     - Expected: fails: `ui_programs_run_and_survive_gc_stress`.
  34. **`with_location` restores no location, not the one before.** `std/ui/router/lib.nova`:
     `s.location = was` becomes `s.location = None`.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests ui_programs_run_and_survive_gc_stress`.
     - Expected: fails: `ui_programs_run_and_survive_gc_stress`.
  35. **A router starts at the page's location inside `with_location`.** `std/ui/router/lib.nova`:
     `let location = signal(starting(history))` becomes `let location = signal(located(history))`.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests ui_programs_run_and_survive_gc_stress`.
     - Expected: fails: `ui_programs_run_and_survive_gc_stress`.
  36. **`found` is true for the catch-all.** `std/ui/router/lib.nova`:
     `Some(hit) => !catch_all(pieces_of(hit.route.path))` becomes `Some(hit) => true`.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests ui_programs_run_and_survive_gc_stress`.
     - Expected: fails: `ui_programs_run_and_survive_gc_stress`.
  37. **`static_paths` repeats a path.** `std/ui/router/lib.nova`:
     `if kept.get(j).unwrap_or("") == path { fixed = false }`: the line goes.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests ui_programs_run_and_survive_gc_stress`.
     - Expected: fails: `ui_programs_run_and_survive_gc_stress`.
  38. **`static_paths` keeps a route with a parameter.** `std/ui/router/lib.nova`:
     `if pattern[k].starts_with(":") { fixed = false }`: the line goes.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests ui_programs_run_and_survive_gc_stress`.
     - Expected: fails: `ui_programs_run_and_survive_gc_stress`.
  39. **`<` in the data is written as it is.** `std/ui/ssr/lib.nova`:
     replace

     ```
             parts[i] = if c == '<' {
                 "\\u003c"
     ```

     with

     ```
             parts[i] = if c == '<' {
                 "<"
     ```

     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests ssr_page_page_with_and_document`.
     - Expected: fails: `ssr_page_page_with_and_document`.
  40. **U+2028 in the data is written as it is.** `std/ui/ssr/lib.nova`:
     `} else if code == 8232 {` becomes `} else if false {`.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests ssr_page_page_with_and_document`.
     - Expected: fails: `ssr_page_page_with_and_document`.
  41. **`document` drops the template after the root.** `std/ui/ssr/lib.nova`:
     `let after = empty.slice(at + root.len(), empty.len())` becomes `let after = ""`.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests ssr_page_page_with_and_document`.
     - Expected: fails: `ssr_page_page_with_and_document`.
  42. **The data's `script` has no type.** `std/ui/ssr/lib.nova`:
     replace

     ```
             Some(d) => "<script type=\"application/json\" id=\"nova-data\">${d}</script>"
     ```

     with

     ```
             Some(d) => "<script id=\"nova-data\">${d}</script>"
     ```

     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests ssr_page_page_with_and_document`.
     - Expected: fails: `ssr_page_page_with_and_document`.
  43. **`document`'s panic says something else.** `std/ui/ssr/lib.nova`:
     `None => panic("ssr: the page has no <div id=\"app\">")` becomes `None => panic("ssr: no root")`.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests ssr_page_page_with_and_document`.
     - Expected: fails: `ssr_page_page_with_and_document`.
  44. **A filled root is not emptied.** `std/ui/ssr/lib.nova`:
     `Some("${html.slice(0, at)}${app_root()}${rest}")` becomes `Some(html)`.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests ssr_ssg_writes_the_static_pages_and_refuses_a_bad_path`.
     - Expected: fails: `ssr_ssg_writes_the_static_pages_and_refuses_a_bad_path`.
  45. **The data after a filled root stays.** `std/ui/ssr/lib.nova`:
     replace

     ```
         if matches_at(cs, k, "<script type=\"application/json\" id=\"nova-data\">") {
     ```

     with

     ```
         if false {
     ```

     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests ssr_ssg_writes_the_static_pages_and_refuses_a_bad_path`.
     - Expected: fails: `ssr_ssg_writes_the_static_pages_and_refuses_a_bad_path`.
  46. **An unreadable template's error does not name it.** `std/ui/ssr/lib.nova`:
     `Err(e) => return Err(unreadable("ssr", index, e))` becomes `Err(e) => return Err(unreadable("ssr", "the template", e))`.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests ssr_serve_reports_what_stops_it_starting`.
     - Expected: fails: `ssr_serve_reports_what_stops_it_starting`.
  47. **A manifest with no files is reported otherwise.** `std/ui/ssr/lib.nova`:
     replace

     ```
     IoError { kind: InvalidData, message: "ssr: ${listing} has no \"files\" object" },
     ```

     with

     ```
     IoError { kind: InvalidData, message: "ssr: ${listing} is not a manifest" },
     ```

     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests ssr_serve_reports_what_stops_it_starting`.
     - Expected: fails: `ssr_serve_reports_what_stops_it_starting`.
  48. **The URL printed is the address, not the port bound.** `std/ui/ssr/lib.nova`:
     `println("ssr: serving http://${host_of(site.address)}:${port}/")` becomes `println("ssr: serving http://${site.address}/")`.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests ssr_serve_answers_files_and_pages`.
     - Expected: fails: `ssr_serve_answers_files_and_pages`.
  49. **Any file in dist/ is served.** `std/ui/ssr/lib.nova`:
     `if s.files.get(path).is_some() { return file(s.dist, path) }` becomes `if true { return file(s.dist, path) }`.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests ssr_serve_answers_files_and_pages`.
     - Expected: fails: `ssr_serve_answers_files_and_pages`.
  50. **`.wasm` is `application/octet-stream`.** `std/ui/ssr/lib.nova`:
     `if ext == "wasm" { return "application/wasm" }`: the line goes.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests ssr_serve_answers_files_and_pages`.
     - Expected: fails: `ssr_serve_answers_files_and_pages`.
  51. **A file has no `nosniff`.** `std/ui/ssr/lib.nova`:
     `h.insert("x-content-type-options", "nosniff")`: the line goes.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests ssr_serve_answers_files_and_pages`.
     - Expected: fails: `ssr_serve_answers_files_and_pages`.
  52. **A name with a `.` that is no file is a page.** `std/ui/ssr/lib.nova`:
     the line holding

     ```
         if pieces[pieces.len() - 1].contains(".") { return Response::not_found() }
     ```

     goes.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests ssr_serve_answers_files_and_pages`.
     - Expected: fails: `ssr_serve_answers_files_and_pages`.
  53. **A page is `200` whatever `found` says.** `std/ui/ssr/lib.nova`:
     `let status = if found(s.routes, target) { 200 } else { 404 }` becomes `let status = 200`.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests ssr_serve_answers_files_and_pages`.
     - Expected: fails: `ssr_serve_answers_files_and_pages`.
  54. **The fallback answers every method.** `std/http/lib.nova`:
     replace

     ```
                     }
                 }
                 _ => Response::not_found()
     ```

     with

     ```
                     }
                 }
                 _ => {
                     match self.unmatched {
                         Some(h) => h(req)
                         None => Response::not_found()
                     }
                 }
     ```

     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests ssr_serve_answers_files_and_pages`.
     - Expected: fails: `ssr_serve_answers_files_and_pages`.
  55. **A page's directories are joined with `-`.** `std/ui/ssr/lib.nova`:
     replace

     ```
             let folder = if pieces.len() == 0 { dir } else { "${dir}/${"/".join(pieces)}" }
     ```

     with

     ```
             let folder = if pieces.len() == 0 { dir } else { "${dir}/${"-".join(pieces)}" }
     ```

     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests ssr_ssg_writes_the_static_pages_and_refuses_a_bad_path`.
     - Expected: fails: `ssr_ssg_writes_the_static_pages_and_refuses_a_bad_path`.
  56. **`/` is refused as a path with an empty piece.** `std/ui/ssr/lib.nova`:
     `if rest == "" {` becomes `if false {`.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests ssr_ssg_writes_the_static_pages_and_refuses_a_bad_path`.
     - Expected: fails: `ssr_ssg_writes_the_static_pages_and_refuses_a_bad_path`.
  57. **The page's directories are not made.** `std/ui/ssr/lib.nova`:
     `match create_dir_all(folder).await {` becomes `match create_dir_all(dir).await {`.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests ssr_ssg_writes_the_static_pages_and_refuses_a_bad_path`.
     - Expected: fails: `ssr_ssg_writes_the_static_pages_and_refuses_a_bad_path`.
  58. **A `..` piece is not refused.** `std/ui/ssr/lib.nova`:
     `let special = piece == "" || piece == "." || piece == ".."` becomes `let special = piece == "" || piece == "."`.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests ssr_ssg_writes_the_static_pages_and_refuses_a_bad_path`.
     - Expected: fails: `ssr_ssg_writes_the_static_pages_and_refuses_a_bad_path`.
  59. **A piece holding `\`, NUL, `?` or `#` is not refused.** `std/ui/ssr/lib.nova`:
     replace

     ```
             if !plain_piece(p) || p.contains("?") || p.contains("#") { return None }
     ```

     with

     ```
             if p == "" || p == "." || p == ".." { return None }
     ```

     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests ssr_ssg_writes_the_static_pages_and_refuses_a_bad_path`.
     - Expected: fails: `ssr_ssg_writes_the_static_pages_and_refuses_a_bad_path`.
  60. **`--ssg` with no directory is not refused.** `std/ui/ssr/lib.nova`:
     `if dir == "" {` becomes `if false {`.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests ssr_ssg_writes_the_static_pages_and_refuses_a_bad_path`.
     - Expected: fails: `ssr_ssg_writes_the_static_pages_and_refuses_a_bad_path`.
  61. **The fallback is never called.** `std/http/lib.nova`:
     replace

     ```
                         Some(h) => h(req)
                         None => Response::not_found()
     ```

     with

     ```
                         Some(h) => Response::not_found()
                         None => Response::not_found()
     ```

     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests http_server_dispatch_run`.
     - Expected: fails: `http_server_dispatch_run`.
  62. **`get` drops the fallback.** `std/http/lib.nova`:
     `Server { routes: rs, unmatched: self.unmatched }` becomes `Server { routes: rs, unmatched: None }`.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests http_server_dispatch_run`.
     - Expected: fails: `http_server_dispatch_run`.
  63. **`listen` binds and does not serve.** `std/http/lib.nova`:
     replace

     ```
                 Err(e) => return Err(e)
             }
             self.listen_on(l).await
     ```

     with

     ```
                 Err(e) => return Err(e)
             }
             let _ = l.close().await
             Ok(())
     ```

     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests http_server_example_serves_and_exits_cleanly`.
     - Expected: fails: `http_server_example_serves_and_exits_cleanly`.
  64. **`import ui::ssr` names `std/ui/html`.** `crates/nova-resolver/src/lib.rs`:
     `("ui::ssr", "$std.ui.ssr"),` becomes `("ui::ssr", "$std.ui.html"),`.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-resolver --lib ssr_is_imported_by_name_and_only_natively`.
     - Expected: fails: `tests::ssr_is_imported_by_name_and_only_natively`.
  65. **On WASM, `ui::ssr`'s names are E0092 without the import.** `crates/nova-resolver/src/lib.rs`:
     replace

     ```
             if compiled.iter().any(|&(n, _)| n == name) || IMPORTED_STD.iter().any(|&(_, m)| m == name)
             {
     ```

     with

     ```
             if compiled.iter().any(|&(n, _)| n == name) {
     ```

     - Run `CARGO_INCREMENTAL=0 NOVA_REQUIRE_NODE=1 cargo test -p nova-cli --test wasm_tests ssr_is_imported_by_name_and_only_natively`.
     - Expected: fails: `ssr_is_imported_by_name_and_only_natively`.
  66. **The server's root is src/main.nova.** `crates/nova-driver/src/program.rs`:
     `"src/server.nova"` becomes `"src/main.nova"`.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-driver --test packages a_server_is_rooted_at_src_server_nova`.
     - Expected: fails: `a_server_is_rooted_at_src_server_nova`.
  67. **The server's program takes src/lib.nova too.** `crates/nova-driver/src/program.rs`:
     `if !matches!(roots, Roots::Program | Roots::Server) && lib.is_file() {` becomes `if roots != Roots::Program && lib.is_file() {`.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-driver --test packages a_server_is_rooted_at_src_server_nova`.
     - Expected: fails: `a_server_is_rooted_at_src_server_nova`.
  68. **`--ssr` takes a file.** `crates/nova-cli/src/cmd/run.rs`:
     `if cmd.file.is_some() {` becomes `if false {`.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test ssr_tests ssr_and_ssg_refuse_before_building`.
     - Expected: fails: `ssr_and_ssg_refuse_before_building`.
  69. **`--ssr` takes `--target`.** `crates/nova-cli/src/cmd/run.rs`:
     `if cmd.target.is_some() {` becomes `if false {`.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test ssr_tests ssr_and_ssg_refuse_before_building`.
     - Expected: fails: `ssr_and_ssg_refuse_before_building`.
  70. **Outside a project, the refusal says otherwise.** `crates/nova-cli/src/cmd/run.rs`:
     replace

     ```
             bail!("`{flag}` builds a project, and there is no nova.toml here or above");
     ```

     with

     ```
             bail!("`{flag}` needs a project");
     ```

     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test ssr_tests ssr_and_ssg_refuse_before_building`.
     - Expected: fails: `ssr_and_ssg_refuse_before_building`.
  71. **A missing src/server.nova is found only after the client is built.** `crates/nova-cli/src/cmd/run.rs`:
     `if !root.join("src").join("server.nova").is_file() {` becomes `if false {`.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test ssr_tests ssr_and_ssg_refuse_before_building`.
     - Expected: fails: `ssr_and_ssg_refuse_before_building`.
  72. **The server is target/debug/<name>.** `crates/nova-cli/src/cmd/run.rs`:
     `"{}-server{}",` becomes `"{}{}",`.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test ssr_tests ssr_builds_the_client_and_the_server`.
     - Expected: fails: `ssr_builds_the_client_and_the_server`.
  73. **`-o` is ignored.** `crates/nova-cli/src/cmd/run.rs`:
     replace

     ```
         let (dist, bundle, listing) = build_client(&mode, cmd.output, cmd.release)?;
     ```

     with

     ```
         let (dist, bundle, listing) = build_client(&mode, None, cmd.release)?;
     ```

     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test ssr_tests ssr_builds_the_client_and_the_server`.
     - Expected: fails: `ssr_builds_the_client_and_the_server`.
  74. **A failing server ends nova with 1, not its status.** `crates/nova-cli/src/cmd/run.rs`:
     `std::process::exit(status.code().filter(|&c| c != 0).unwrap_or(1));` becomes `std::process::exit(1);`.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test ssr_tests a_failing_server_fails_ssg_with_its_status`.
     - Expected: fails: `a_failing_server_fails_ssg_with_its_status`.
  75. **The pages the server wrote are not listed.** `crates/nova-cli/src/cmd/run.rs`:
     `Some(page) => pages.push(page),` becomes `Some(_) => {}`.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test ssr_tests ssg_writes_a_page_per_static_route_and_lists_them`.
     - Expected: fails: `ssg_writes_a_page_per_static_route_and_lists_them`.
  76. **The server is given `-o`'s path as written, from the project's root.** `crates/nova-cli/src/cmd/run.rs`:
     `let dist = from_root(dist);` becomes `let dist = dist.to_path_buf();`.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test ssr_tests ssr_and_ssg_together_is_ssg_from_anywhere_in_the_project`.
     - Expected: fails: `ssr_and_ssg_together_is_ssg_from_anywhere_in_the_project`.
  77. **`add_pages` measures nothing again.** `crates/nova-cli/src/bundle.rs`:
     `*bytes = size(&out.join(name))?;`: the line goes.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --bin nova bundle::`.
     - Expected: fails: `bundle::tests::ssg_pages_join_the_manifest_with_their_sizes`.
  78. **With no data, a filled root is built afresh, not walked with the fallback.** `std/ui/lib.nova`:
     replace

     ```
             None => fallback
     ```

     with

     ```
             None => {
                 let mut gone = false
                 while !gone {
                     match backend_first_child(parent) {
                         Some(n) => backend_remove(n)
                         None => gone = true
                     }
                 }
                 fallback
             }
     ```

     - Run `CARGO_INCREMENTAL=0 NOVA_REQUIRE_NODE=1 cargo test -p nova-cli --test wasm_tests hydrate_adopts_the_servers_nodes`.
     - Expected: fails: `hydrate_adopts_the_servers_nodes`.
  79. **The data is the page's first element whose id is `nova-data`, not the root's next.** `std/ui/backend/wasm.nova`:
     replace

     ```
         let mut at = root.el.next_sibling()
     ```

     with

     ```
         match element_by_id("nova-data") {
             Some(e) => return Some(e.text())
             None => {}
         }
         let mut at = root.el.next_sibling()
     ```

     - Run `CARGO_INCREMENTAL=0 NOVA_REQUIRE_NODE=1 cargo test -p nova-cli --test wasm_tests hydrate_adopts_the_servers_nodes`.
     - Expected: fails: `hydrate_adopts_the_servers_nodes`.
  80. **The data's element may be any element, not only a `script`.** `std/ui/backend/wasm.nova`:
     `if kind == "script" && named && json { return Some(e.text()) }` becomes `if named && json { return Some(e.text()) }`.
     - Run `CARGO_INCREMENTAL=0 NOVA_REQUIRE_NODE=1 cargo test -p nova-cli --test wasm_tests hydrate_`.
     - Expected: fails: `hydrate_adopts_the_servers_nodes`.
  81. **The data's element needs no `type`.** `std/ui/backend/wasm.nova`:
     `if kind == "script" && named && json { return Some(e.text()) }` becomes `if kind == "script" && named { return Some(e.text()) }`.
     - Run `CARGO_INCREMENTAL=0 NOVA_REQUIRE_NODE=1 cargo test -p nova-cli --test wasm_tests hydrate_`.
     - Expected: fails: `hydrate_adopts_the_servers_nodes`.
  82. **A blank root keeps its whitespace, and the view is built after it.** `std/ui/lib.nova`:
     replace

     ```
         let mut emptied = false
         while !emptied {
     ```

     with

     ```
         let mut emptied = blank(parent)
         while !emptied {
     ```

     - Run `CARGO_INCREMENTAL=0 NOVA_REQUIRE_NODE=1 cargo test -p nova-cli --test wasm_tests hydrate_adopts_the_servers_nodes`.
     - Expected: fails: `hydrate_adopts_the_servers_nodes`.
  83. **`found` reads a `#fragment` as part of the path.** `std/ui/router/lib.nova`:
     `match first_match(routes, unfragmented(location)) {` becomes `match first_match(routes, location) {`.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests ui_programs_run_and_survive_gc_stress`.
     - Expected: fails: `ui_programs_run_and_survive_gc_stress`.
  84. **`document` fills a root that holds something without emptying it.** `std/ui/ssr/lib.nova`:
     replace

     ```
         let empty = match emptied(template) {
             Some(t) => t
     ```

     with

     ```
         let empty = match emptied(template) {
             Some(t) => template
     ```

     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests ssr_page_page_with_and_document`.
     - Expected: fails: `ssr_page_page_with_and_document`.
  85. **`serve` keeps a manifest's path that leaves `dist`.** `std/ui/ssr/lib.nova`:
     `Object(files) => Some(inside(files))` becomes `Object(files) => Some(files)`.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests ssr_serve_answers_files_and_pages`.
     - Expected: fails: `ssr_serve_answers_files_and_pages`.
  86. **A `ssg: wrote` file with `..` below the output directory is a page.** `crates/nova-cli/src/cmd/run.rs`:
     `_ => return None,` becomes `_ => {}`.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test ssr_tests ssg_refuses_a_page_outside_the_output_directory`.
     - Expected: fails: `ssg_refuses_a_page_outside_the_output_directory`.
  87. **A `ssg: wrote` line naming no file inside the output directory is ignored.** `crates/nova-cli/src/cmd/run.rs`:
     `stray.get_or_insert_with(|| text.to_string());`: the line goes.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test ssr_tests ssg_refuses_a_page_outside_the_output_directory`.
     - Expected: fails: `ssg_refuses_a_page_outside_the_output_directory`.
  88. **A failed server's exit code is told in `ExitStatus`'s words.** `crates/nova-cli/src/cmd/run.rs`:
     `return format!("with exit code {code}");` becomes `return format!("with {status}");`.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test ssr_tests a_failing_server_fails_ssg_with_its_status`.
     - Expected: fails: `a_failing_server_fails_ssg_with_its_status`.
  89. **A signal that stopped the server is told in `ExitStatus`'s words.** `crates/nova-cli/src/cmd/run.rs`:
     `return format!("stopped by signal {signal}");` becomes `return format!("with {status}");`.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --bin nova cmd::run::`.
     - Expected: fails: `cmd::run::tests::a_failed_server_is_told_by_its_code_or_its_signal`.
  90. **The data's scan goes on past the first element after the root.** `std/ui/backend/wasm.nova`:
     replace

     ```
                 return None
             }
             at = e.next_sibling()
     ```

     with

     ```
             }
             at = e.next_sibling()
     ```

     - Run `CARGO_INCREMENTAL=0 NOVA_REQUIRE_NODE=1 cargo test -p nova-cli --test wasm_tests hydrate_adopts_the_servers_nodes`.
     - Expected: fails: `hydrate_adopts_the_servers_nodes`.
  91. **The data's scan stops at a text or a comment after the root.** `std/ui/backend/wasm.nova`:
     replace

     ```
             at = e.next_sibling()
         }
         None
     }
     ```

     with

     ```
             at = None
         }
         None
     }
     ```

     - Run `CARGO_INCREMENTAL=0 NOVA_REQUIRE_NODE=1 cargo test -p nova-cli --test wasm_tests hydrate_adopts_the_servers_nodes`.
     - Expected: fails: `hydrate_adopts_the_servers_nodes`.
- [ ] **Step 5: The review's inputs.** `06-counter-spa`'s size and the full
  run's figures, as Step 1 measured them, `ssr-app`'s client size, as Task
  5's line printed it, and the mutants' outcome go in the ledger.
- [ ] **Step 6: One fresh read-only review, then one fix pass** (the
  user's workflow).
  - The reviewer reads `git diff $BASE..HEAD` (Task 0) against the spec,
    ADR 0045 and this plan, and is told to hunt for:
    - a walk that adopts the wrong node or none: the cursor past each node
      adopted and into each element, a tag compared without its case, a
      text compared with the wrong text, an empty text's new node put in
      the wrong place, a separator stepped over when it should not be or
      not when it should, a region's end, a node left over, a `Property`
      left unset, a handler not added, `on_mount` run during the walk; a
      blank root's whitespace left before the view (decision 35);
    - data read from the wrong element: any but the root's next element,
      one that is not a `script`, or lacks the id or the type, or a view's
      own `nova-data`; the fallback not used when there is no data, or a
      filled root built afresh instead of walked with it (decisions 29 and
      32);
    - a mismatch whose cleanup leaves something behind: the walk's
      effects, handlers or `on_mount` callbacks still live, its owner kept
      in the roots, a server's node left in the root, or `f`'s view not
      built again (decision 16);
    - a marker a program could forge, through text, an attribute, the data
      or the template, and `emptied`'s count of `<div` and `</div>`, which
      holds only while every text and attribute value is escaped
      (decision 19);
    - escaping that lets the data end its `script`, or change the page:
      `</script>`, `<!--`, `&`, U+2028 and U+2029, and a CR in text or an
      attribute;
    - a file served that the manifest does not list, by any spelling or
      decoding of its path, on any system, or that it lists by a path that
      leaves `dist` (decision 33); a status that is not the spec's, a
      `Content-Type` wrong, `nosniff` missing; a method other than `GET`
      reaching the fallback;
    - a filled root emptied wrongly, by `serve`, SSG or `document`: its
      end, the data after it, an `index.html` written by hand whose `app`
      root holds whitespace or a placeholder, a root never closed (decisions
      34 and 38), and a template comment holding `</div>`, which stands;
    - SSG writing outside its directory, or past a refused path; a path
      `static_paths` should not give, or a repeat; `/` and a route ending
      in `/` (decision 31);
    - the driver's root taking `src/main.nova` or `src/lib.nova` into the
      server's program, or missing a dependency;
    - a refusal that comes after something is built, or in other words
      than decision 20's; the server's path or name on Windows; the pages
      `generate` keeps from the server's lines, a page outside the
      client's directory or named with `..` or `.`, and the build such a
      line fails (decisions 33 and 39); a failing server's exit code and
      its words, on Unix and on Windows (decisions 25 and 37);
    - a test that could pass without the code it names, as the four the
      prototype's two gap fixes closed did (Checked on);
    - Windows paths in SSG and in the CLI tests: `\` and `/` in a page's
      path, `Path::strip_prefix` and the plain names on the server's
      lines, the `.exe`, the termination request, and a name Windows
      reserves, which SSG does not refuse and which stands (ADR 0045);
    - a record that says what the code does not, MSRV 1.78, and tests that
      touch the internet, use port 3000, or use temp directories without
      fixed names.
  - Each finding is fixed in one commit, `4.6: the review's fixes`, or
    answered in the ledger with the reason it stands.
  - After the fixes, Step 1 runs again.
- [ ] **Step 7: The PR.**
  - Push: `git push -u origin claude/loving-archimedes-frtf5q-46`.
  - Open the PR to `main` with the repository's template headings, if it
    has one. The body names no local path and no person.
  - It holds:
    - what 4.6 adds;
    - the gate's evidence: `tests/browser/ssr-app` in Chromium, a page its
      server renders and a page SSG wrote each hydrating in place, every
      node kept and a click updating them, and `/nowhere`'s `404`; the
      hydration tests under the fake DOM, as they are and under stress;
      the `ssr_*` tests and `ssr_tests.rs` on all three systems; the sizes,
      `06-counter-spa`'s unchanged and `ssr-app`'s client's printed;
    - the user's thirteen decisions, the thirteenth made after the
      fact-check, and this plan's decisions, numbered as here, with 4, 6,
      10, 13 to 26, 29 and 32 to 39 marked as differing from the spec;
    - that `Cargo.lock` is unchanged;
    - the fact-check's nine findings, what fixed each, and the two that
      stand, with the nit on the spec's risk 5; the second check's seven
      findings and their fixes (Checked on);
    - the mutants, the two gaps the first run found, the two the
      fact-check's fixes left (80 and 81), the tests that closed them, and
      the review's findings and what came of each.
  - Read CI once it ends. Expected: green on all three systems, and the
    `browser` job green; the test job runs the WASM tests under Node 22
    with `NOVA_REQUIRE_NODE=1`.
- [ ] **Step 8: The merge,** only on the user's word.
  - By rebase, pinned to the PR's head commit. Never auto-merge.
  - Then `git fetch origin main` and `git diff <head> origin/main
    --stat`, which prints nothing: the merged tree is the PR's.
