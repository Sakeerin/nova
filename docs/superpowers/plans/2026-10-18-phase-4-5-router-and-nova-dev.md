# Phase 4.5, "The router and `nova dev`", Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** a page routes, and `nova dev` serves it. `std/ui/router`, which a
program imports with `import ui::router`, gives `30-FRONTEND.md` §6's
names: `Router(routes)` routes by the URL's hash and `HistoryRouter(routes)`
by its path, through the history API; a `Route { path, component }` holds
`:name` parameters, which `use_params()` reads, and the route `*` matches
every location; `use_query()` reads the query, `navigate(to)` goes to a
path, and `link(to: …) { … }` is an `<a>` whose plain click goes to its
route without loading the page. It compiles on both targets, and natively
the location is `/`. The generated page loads `/app.js` by its absolute
path, so a page in history mode can be reloaded at any path. `nova dev`
builds a program into `dist/` in a child `nova build --target wasm`, serves
it on `127.0.0.1` from a server on `std::net`, polls its sources, and on a
change builds again and reloads the page, or shows the build's errors over
it. The gate, in CI's `browser` job through Playwright: `06-counter-spa`
under `nova dev` counts its clicks, an edit reloads it with the change, a
type error shows the overlay and its fix takes the overlay away; and a
`HistoryRouter` app under `nova dev` follows a link, the back button and a
reload.

**Architecture:**
- **`nova-resolver`, `nova-typeck`, `nova-mir`, `nova-runtime`, `std/rt`:**
  `router_state_present`, `_load` and `_store`, std-only builtins made as
  `ui_state_*` are: natively a second thread root in `nova-runtime`'s
  `ui.rs`, on WASM `std/rt`'s state word 112, which `gc_collect` marks.
- **`std/dom`, the shim, the fake DOM:** `Event.modified()`; the fake DOM's
  events gain modifier keys and a button, and `click(modifiers)`.
- **`std/ui/backend/{wasm,native}.nova`:** `backend_location`;
  `backend_go` and `backend_replace`, which return the location the page
  reports afterwards; `backend_on_location`; and `backend_on_link_click`,
  whose function says whether it took the click.
- **`std/ui/router/lib.nova`:** the module, registered in `std/lib.rs`, the
  resolver's `STD_MODULES`, `WASM_STD_MODULES` and `IMPORTED_STD`, and
  `nova-doc`'s list of modules.
- **`nova-codegen-wasm`:** the generated page loads `/app.js` by its
  absolute path (the user's decision 9).
- **`nova-cli`:** `cmd/dev.rs`, the command, its builds and its watcher;
  `dev/server.rs`, the HTTP server and the event stream; `dev/reload.js`,
  the reload script and the overlay, embedded with `include_str!`.
  `bundle.rs`'s `check_output_for` and `resolved`, and `cmd/run.rs`'s
  `wasm_output`, are shared with `nova build`.
- **Tests:** `tests/runtime/ui_router*`,
  `tests/dom/router_{hash,history,two}.*`,
  `crates/nova-cli/tests/dev_tests.rs`, and `tests/browser/dev.mjs`,
  `router.mjs` and `router-app/` in the `browser` job.
- **Records:** ADR 0044, and dated notes.

**Tech Stack:**
- Rust: MSRV 1.78, edition 2021.
- **No new crate; `Cargo.lock` does not change** (spec §6). The server is
  `std::net` with a thread per connection; the watcher polls with
  `std::fs`.
- JS: ES modules run by Node 22 for the fake-DOM harnesses, with 4.2's fake
  DOM given modifier keys; `reload.js` runs in the browser.
- Playwright 1.56.1, installed by the CI job only.
- Tests: unit tests in `nova-resolver`, `nova-runtime`, and `nova-cli`'s
  `cmd/dev.rs` and `dev/server.rs`, and one changed in `nova-typeck`;
  `crates/nova-mir/tests/lower_tests.rs`; `crates/nova-cli/tests/run_tests.rs`,
  `wasm_tests.rs`, `doc_test.rs` and `dev_tests.rs`;
  `crates/nova-driver/tests/index_complete.rs`; programs in `tests/runtime/`
  and `tests/dom/`; doc tests in `std/ui/router`.

**Spec:** `docs/superpowers/specs/2026-10-18-phase-4-5-router-and-nova-dev-design.md`,
approved by the user on 2026-10-18.
- Read it before Task 1: it is the authority this plan argues from.
- Its header holds the user's eight decisions, its §10 the decisions made
  while writing it, and its §2 the code as it stood. The user's decision 9,
  of the same day, came after the fact-check below: the generated page
  loads `/app.js` by its absolute path. This plan's decisions say where the
  code departs from the spec.

**Checked on 2026-10-18, before execution:**
- **A prototype built 4.5 whole** on a local branch from `3bcd1c6`, in
  this tree, with every test below. Its first event stream was spec
  §4.5's, which the prototype found wanting and replaced before the split:
  - the script reloaded on a `hello` whose number differed from the first
    one it heard. A page served at build 1 whose stream first connects
    after build 2 hears `hello 2` first, so it never reloads and keeps the
    old build; and a restarted server counts from 1 again, so a page that
    heard `hello 1` before the restart hears it again after;
  - each build now has an id, `<start>.<number>`: the server's start in
    milliseconds and the build's number. The page's script tag carries the
    id current when the page is served, `hello` and `reload` carry the
    current one, and the script reloads on a `hello` that is not its tag's
    (decision 16).
- **A fact-check read the first draft** against the spec, the code and the
  replay, with probes of its own in Chromium, and found faults in the
  router and the server, a decision that narrowed the user's, and gaps in
  the tests, each fixed here with its test:
  - in hash mode a route the browser re-encodes, such as `/posts/café`,
    was built twice: `navigate` set the location as written, and the
    `hashchange` a turn later found it encoded. `backend_go` now returns
    the location the page reports, which `navigate` sets (decision 5);
  - history mode kept a `#fragment` in the location: `/posts/6#sec` showed
    post `6#sec`. The location now never holds one (decision 5);
  - `rooted` kept a leading `//`, so `link(to: "//posts/8")` was a URL of
    another host, whose plain click threw in `pushState`. A leading run of
    `/` and `\` is now one `/` (decision 7);
  - the plan's decision that the generated page keeps `./app.js` narrowed
    the user's decision 3, since such a page cannot be reloaded at
    `/posts/2`. The user's decision 9 settles it: the generated page loads
    `/app.js`, and `tests/browser/router-app` has no page of its own
    (decision 22);
  - a router not mounted last still prevented a marked link's default and
    navigated nowhere. `backend_on_link_click`'s function now says whether
    it took the click, and `tests/dom/router_two` tests it (decision 9);
  - a second click on the same link pushed a second entry. A `to` that is
    the location already now replaces it (decision 6);
  - a single file's watch read every directory below the file's, every
    200 ms; it now reads the file's own directory (decision 20);
  - `.mjs` was `application/octet-stream`, `HEAD /__nova/events` said
    `keep-alive` and then closed, and a head whose lines end in a bare LF
    waited out its timeout. Each is fixed, and the version must be
    `HTTP/1.` and one digit (decision 18);
  - untested: the `C:` refusal, now a `#[cfg(windows)]` loop that CI's
    Windows leg runs; `HEAD /__nova/events`; and the watcher's link to
    nothing;
  - in the records: notes in `00-MASTER-SPEC.md`, and ADR 0044's and the
    notes' wording, `60-EXAMPLES.md`'s size by `flate2` among them.
- **A second check read those fixes** and found two more faults in the
  router, each fixed here with its test, and wording to correct:
  - a tab, LF or CR in a `to`'s leading run, as in `/\t/evil.example/x`,
    still named another host, since a URL's parser deletes them before it
    reads the URL. `rooted` now drops every tab, LF and CR first
    (decision 7; mutant 92);
  - the click fix called the router's function before it prevented the
    default, so a panic on the way handed a taken click back to the
    browser. `backend_on_link_click` now takes `take` and `go`, and
    prevents the default before `go` runs (decision 9; mutant 47);
  - a fragment reaches the location in hash mode and natively, not in
    history mode, and `router_two`'s page stays as it was only in the fake
    DOM, which follows no link: the comments, ADR 0044 and `30-FRONTEND.md`
    §13 now say so.
- **What the checks left standing,** each recorded or handed on:
  - killing `nova dev` other than by Ctrl-C leaves a running build to
    finish on its own (ADR 0044's consequences; decision 19);
  - the event stream's ping loop is untested; `Events::ping` is;
  - `HTTP/1.9` is accepted on purpose, as RFC 9110 §2.5 has a higher minor
    version processed as the highest implemented, and a `Host` ending in a
    bare CR is trimmed (decision 18);
  - a history `navigate` that changes only the fragment replaces the
    entry, since the router ignores fragments (decision 6);
  - `/__nova/reload.js/` and `/__nova//events` are the server's own, and
    on a file system that ignores case `/__NOVA/…` is read from disk: left
    to the review (Task 6, Step 6).
- **A replay applied it** to `3bcd1c6` on a scratch branch, task by task,
  as the steps say, each task's tests before its code; each test-first
  step's failure is that run's.
  - A script split the prototype's final tree into one commit per task,
    each task's tests split from its code. The resolver's test of the state
    builtins looks in `$std.ui` until Task 2 points it at `$std.ui.router`.
  - The patches below are those commits. Each applies with `git apply` in
    order on `3bcd1c6`, and the last tree is the prototype's.
- **The mutants.** A run of 92 after the first check's fixes found four
  that passed their tests, each closed or named here:
  - no test reached, in hash mode, an address whose hash holds a second
    `#`: `router_hash` now goes to `/posts/6#sec` (mutant 27);
  - no test went, in history mode, to the current location written
    another way: `router_history`'s `Café again` goes to `/posts/café`
    while at `/posts/caf%C3%A9`, and adds no entry (mutant 29);
  - `router_history` clicked `Params` after a route without parameters, so
    a handler that saw the last render's parameters passed: it now clicks
    it after `/posts/caf%C3%A9` (mutant 49);
  - the server's plain-name check is Windows's alone once `..` is refused,
    so no mutant of it alone can fail here; it is dropped, the
    `#[cfg(windows)]` loop exercises the check on CI's Windows leg, and
    one mutant removes both checks (mutant 61).
- **The gate passed in the replay,** locally: every script of the
  `browser` job through Playwright 1.56.1 in Chromium, `dev.mjs` and
  `router.mjs` among them, on the tree before the second check's fixes.
- **Every final check ran on the final tree,** the split's last commit,
  whose tree is the prototype's:
  - Task 2's runs before and after its code: `W router` `0 passed; 5
    failed`, then `5 passed`; `R ui_programs` `0 passed; 2 failed`, then
    `2 passed`;
  - `cargo fmt`, clippy with `-D warnings` and both `nova fmt` checks were
    clean;
  - the full suite gave 2357 passed, 1 failed (the IPv6 test) and 10
    ignored, against the base's `2317 1 10`;
  - `06-counter-spa`'s `app.wasm` and `app.js` are 21,070 bytes by
    `flate2` at level 9, as the test measures, against 51,200; 4.4's
    figure was 21,019, and `std/rt`'s three functions and its mark add the
    51;
  - Rust 1.78's check was clean, and `Cargo.lock` did not change;
  - all 92 mutants in Task 6 failed their named tests.

  The other steps' results are the replay's, on trees that differ from the
  final one only in the router's code and tests and in the records; after
  each task there, `cargo fmt`, clippy and `nova fmt` were clean.

## Global Constraints

- **The workspace's minimum Rust stays 1.78,** edition 2021. CI's MSRV job
  runs `cargo check --locked --workspace` with `RUSTFLAGS=-D warnings`. No
  `std::sync::LazyLock` or `LazyCell` (1.80), no `Option::is_none_or`
  (1.82), no `std::path::absolute` (1.79).
- **`Cargo.lock` does not change.** Any change to it stops the task: ask
  the user. So does anything that needs a new crate. The server is
  `std::net`, the watcher `std::fs`, and the event stream is written by
  hand (spec §6, ADR 0038).
- **No `unwrap()` outside `#[cfg(test)]` and test files:** `expect("why")`,
  `unwrap_or…` or `?`.
- **Tests never touch the internet.** Every server a test starts binds
  `127.0.0.1:0`, and `nova dev` runs with `--port 0`; a test reads the port
  from the line it prints. **No test uses port 3000,** `nova dev`'s default
  and `examples/03`'s port. A test that needs `NOVA_HOME` sets it to a fresh
  directory. Every test directory has a fixed name under the system's
  temporary directory, removed and made again at the test's start. Node
  runs only what a test built.
- **A test that needs Node** passes without running when `node` is not on
  `PATH`, and fails instead under `NOVA_REQUIRE_NODE=1`, which CI sets.
  Run the WASM tests here with `NOVA_REQUIRE_NODE=1`: Node 22 is installed.
- **Nothing in `cargo test` or `nova` installs a package,** and nothing
  installs `wasm-opt`. Playwright is the `browser` job's alone; Task 4's
  local run uses the copy already on this machine, and is skipped where
  there is none.
- **`nova-cli`'s tests need two files beside `target/debug/nova`:**
  `libnova_runtime.a` and the `nova-bench-http` binary. A build of the
  workspace writes both; Task 0's build puts them there. **Task 1 changes
  `nova-runtime`,** and a debug `nova` links `target/debug/libnova_runtime.a`
  into every native binary it builds, `nova test --doc`'s included: after
  Task 1's code, `cargo build --locked -p nova-runtime` (Task 1, Step 5),
  before any doc test runs. After a `cargo clean`, build the workspace
  again.
- **std is embedded at build time.** After a task changes a file under
  `std/`, a run of `target/debug/nova` by hand needs `cargo build --locked
  -p nova-cli` first; `cargo test` rebuilds it itself.
- **Never clean `/tmp/nova-*` while tests run:** `nova test` keeps built
  binaries in `/tmp/nova-test-bin`, and a test whose binary goes fails.
- **Work in one tree.** Two worktrees of this repository sharing one
  `target/` share artifact names, so cargo can take one tree's build as
  fresh for the other (4.0's ledger). Never build in another worktree.
- **Trailing spaces are expected output.** `tests/dom/router_history.stdout`
  (`unmounted: /posts/8 `, `after unmount: /posts/8 `) and
  `tests/runtime/ui_router.stdout` (`/posts/7/8: `, `/exact: `) end those
  lines with a space: a router that shows nothing renders nothing after it.
  Never apply a patch with `--whitespace=fix`, never let an editor trim
  this plan or those files, and expect `git apply`'s warning for Task 2's
  tests.
- **Nothing writes into the repository but the patches.** `dev_tests.rs`
  writes its projects in the temporary directory, and both browser scripts
  copy theirs into the directory they are given, outside the tree.
- **Nothing is published, and no tag is pushed.** Merge by rebase, on the
  user's word only. Never enable auto-merge.
- **This machine is Linux.** Windows and macOS results come from CI, which
  runs all three systems, Node included. `a_link_out_of_dist_is_refused`,
  the server's link test and the watcher's link to nothing are Unix's
  alone. A Windows piece such as `C:` is refused by code this machine
  cannot reach: a `#[cfg(windows)]` loop in the server's refusal test runs
  it on CI's Windows leg, and no mutant of it alone is run here (decision
  18).

## Conventions

- **Scratch directory:** `$P` is the session's scratch directory, outside
  the repository. Set it in every shell before a command that names `$P`.
- **The ledger:** `$P/ledger-45x.md`. Every ruling made while executing goes
  there, with its cost if wrong; so do each task's figures.
- **Commits.** Each task's message is written to `$P/msg-45-<task>.txt`:
  the subject the task names, a blank line, and the session's attribution
  lines. Then `git add -A`; `git status --short` lists only the task's
  files; and `git commit -F "$P/msg-45-<task>.txt"`.
- **The baseline's full output** is `$P/base-45.txt` (Task 0).
- **Runs.** The replay ran every command with these variables set. Each
  shell sets them too, with three shorthands, and runs every command from
  the repository's root:

  ```sh
  export CARGO_INCREMENTAL=0 NO_COLOR=1 RUST_BACKTRACE=0 NOVA_HOME="$P/home-45"
  mkdir -p "$NOVA_HOME"
  W() { NOVA_REQUIRE_NODE=1 cargo test -p nova-cli --test wasm_tests -- "$@"; }
  R() { cargo test -p nova-cli --test run_tests -- "$@"; }
  X() { cargo test -p nova-driver --test index_complete "$@"; }
  ```

  So `W router` runs `wasm_tests`'s tests whose names hold `router`, `R
  ui_programs` `run_tests`'s, and `X` the corpus test. `NOVA_HOME` keeps a
  run by hand off `~/.nova`; every test that needs one sets its own.
- **The patches.** Each task's code is in patch blocks below, each marked
  `<!-- patch NAME -->` and fenced with four backticks. They are the
  replay's commits, so apply them; do not retype them. From the
  repository's root:

  ```sh
  PLAN=docs/superpowers/plans/2026-10-18-phase-4-5-router-and-nova-dev.md   # this file, committed
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
  `.stdout` under `tests/runtime/` and `tests/dom/` is in Task 2's test
  patch; the replay wrote `ui_router.stdout` from the program's native run
  and each `router_*.stdout` from its run under the fake DOM, and each was
  read against the spec.
- **Each task** is test first: its tests' patch, their run and the expected
  failure, then its code's patch, its run and the expected pass. Unit tests
  in the files a task changes ride in its code's patch, and Task 6's
  mutants show they bite. Task 4's patch is tests and CI alone, and Task 5's
  records. Each task then runs its checks:
  - `cargo fmt --all -- --check`, which prints nothing;
  - the workspace's clippy, `cargo clippy --locked --all-targets
    --all-features -- -D warnings`, as CI runs it, which ends with
    `Finished` and no warning;
  - `cargo run --locked -q -p nova-cli -- fmt --check std examples
    tests/wasm tests/dom`, and from Task 4 on with `tests/browser/router-app`
    after `tests/dom`, which prints nothing.

  It ends with one commit. A difference from an `Expected:` is fixed in
  the smallest way and recorded in the ledger as a ruling, with its cost if
  wrong.


## Review Focus

The inputs most likely to bite a user, most likely first. Each has its test
in the task that owns the code:

1. **A location that shows another route than it names:** a literal
   matched without its case, a parameter under the wrong name, pieces
   counted wrong, empty pieces kept, `*` not matching or matching too
   much, the last route winning, a piece not decoded or decoded with a
   wrong hex digit, `+` a space in a path or not in a query, a query split
   wrong or an empty part taken for a key, a `#fragment` kept in the
   location → `tests/runtime/ui_router.nova` and `router_routes_by_the_hash`
   (Task 2), natively and on WASM, as they are and under stress; mutants 7
   to 20, 26 and 27.
2. **A route built twice, or the history wrong:** the location set as
   written where the browser re-encodes it, so its own `hashchange` builds
   the route again; the listener setting an unchanged location; a second
   entry for the location already, compared as written; a push where a
   replace is due, or the reverse; `navigate` with no router →
   `router_routes_by_the_hash` and `router_routes_by_the_history_api`
   (Task 2), mutants 24, 25 and 28 to 34.
3. **A click the router takes from the browser, or one it fails to take:**
   the nearest marked `<a>` above a child, a link without
   `data-nova-link`, each modifier key, another button, a `target`, a
   router's owner disposed, a router not mounted last, which must leave
   the click alone, and a taken click whose default is prevented before
   the router goes → `router_routes_by_the_history_api` and
   `a_router_not_mounted_last_leaves_a_click_to_the_browser` (Task 2),
   mutants 41 to 47, 50, 51 and 53 to 56.
4. **A link that points elsewhere:** its `href` in each mode, a `to`
   without its `/`, a leading `//` or `\`, or one with a tab, LF or CR in
   it, that would name another host, a menu built before its router, a
   reactive `to`, its marker and its other attributes, a link without `to`
   → `ui_router.nova`, `ui_router_no_to.nova` and
   `router_routes_by_the_hash` (Task 2), mutants 21 to 23, 35 to 40 and
   92.
5. **The router's state freed, shared or kept too long:** word 112 left
   unmarked or shared with `std/ui`'s, the native slot not a root, a
   builtin a program can name, a wrong signature or lowering; a later
   router's state cleared by an earlier one's disposal, parameters seen
   outside a render → Task 1's tests, `ui_router.nova` and
   `router_routes_*` under stress (Task 2), mutants 1 to 6, 49 and 52.
6. **A file served from outside the output directory:** `..` plain or
   encoded, a `\` or a NUL, a piece that is not one plain name, a link out,
   the page itself a link out → `a_path_that_leaves_the_directory_or_holds_a_backslash_or_nul_is_403`,
   `a_link_out_of_the_directory_is_403_and_one_inside_is_served` and
   `dev_tests.rs` (Task 3), mutants 57 to 61.
7. **Another site reading the server:** a `Host` that is not
   `127.0.0.1:<port>` or `localhost:<port>`, checked before the method,
   and methods other than `GET` and `HEAD` →
   `the_host_must_name_the_server_and_its_port` and
   `the_answer_checks_the_host_then_the_method_then_the_path` (Task 3),
   mutants 62 and 63.
8. **A page that cannot be reloaded, or the fallback answering what it
   should not:** the generated page loading `./app.js`, a missing script
   answered with HTML, a history path given `404`, no page before the
   first good build → `build_for_wasm_writes_dist` (Task 2), the
   fallback's and the server's own page's unit tests (Task 3) and
   `router.mjs`'s reload (Task 4), mutants 48 and 73 to 75.
9. **The page's script and the stream, and every answer:** the tag in the
   wrong place or `Content-Length` not counting it, `/__nova/` read from
   disk, events framed wrong, an error's lines run together, `hello`,
   `reload` and `error` sent wrong, a build id missing from the tag or the
   events, a failed build not counted, a page that missed a build or
   outlived a restart not reloading; the head's limit, its line ends and
   its version, `no-store`, the types of `.wasm` and `.mjs`, `HEAD`'s body
   and the event stream's `HEAD` → the server's unit tests and
   `dev_tests.rs` (Task 3), mutants 64 to 72 and 76 to 84.
10. **The watcher and the command:** a build started before the sources
    settle, the output directory watched, by any spelling, so a build
    starts another, a single file's watch reading the tree below it, a
    link to nothing left out; the port's message; an output directory that
    holds the project; colour in the overlay → `cmd::dev`'s unit tests and
    `dev_tests.rs` (Task 3), mutants 85 to 91.
11. **A test that passes without its code:** each mutant names one that
    would. The stress counts of `router_routes_*` and `router_two` see a
    listener left behind; `dev_tests.rs` runs `nova dev` without
    `NO_COLOR`, so only its own keeps colour out of the overlay; and the
    browser scripts (Task 4) check each page's build id, so a reload that
    did not happen fails, and `router.mjs` checks the page is the
    generated one, titled `Nova`.

## Decisions

Made while writing the spec (its §10), prototyping, fact-checking and
replaying, each as built. Where one departs from the spec or fills in what
it left open, it says so and why, and is marked **(differs)**; ADR 0044
records them, and the departures from `30-FRONTEND.md`.

**The router**

1. **The router's state is behind three std-only builtins** (spec §10.1,
   §3.5): `router_state_present`, `_load` and `_store`, made as
   `ui_state_*` are. Natively they keep a second thread-local root in
   `nova-runtime`'s `ui.rs`, beside `std/ui`'s, through one `store` both
   share; on WASM, `std/rt`'s `ROUTER_STATE` at word 112, which
   `gc_collect` marks after `UI_STATE`. `STD_ONLY` grows from 84 to 87.
   The record holds the router mounted last (its number, its mode and its
   location, a `Signal<String>`), the parameters of the route being built,
   and the next router's number.
2. **A router is a `memo` of the route that matches its location, feeding
   a `for_each`** of that one match or none. Spec §3.1 asked for "a region"
   and `for_each` is one: it builds the component untracked, and builds it
   again each time the location is set, even to the same route, since a
   `memo` always notifies. So `use_params`, which reads the parameters
   while `rendered` builds the component, and `use_query` need not be
   reactive.
3. **A listener's function runs batched and untracked through `std/ui`'s
   public API, by the router:** `handled(f)` is `batch(|| root(|d| { f();
   d() }))`, since `root` runs its function with no listener and its owner,
   given nothing to own, is disposed at once. **(differs)** Spec §3.6 had
   the backend run each handler so, as `std/ui` runs its own with
   `untracked(|| batch(…))`; but `untracked` is private to `std/ui`, and
   making it public would bind one more name in every module that writes
   `import ui`. So `std/ui` changes not at all.
4. **The `hashchange` or `popstate` listener sets the location only when it
   differs.** **(differs)** Spec §3.3 said it sets the location from the
   URL. `navigate` sets the location at once, and in hash mode the
   `hashchange` its own hash causes comes a turn later; without the check
   the route is built twice (`router_hash`'s "a turn later" line; mutant
   34).
5. **The location is read back after going, and never holds a
   `#fragment`.** **(differs)** Spec §3.6's `backend_go` returns nothing,
   and §3.1 did not speak of fragments.
   - `backend_go` returns the location the page reports afterwards, as
     `backend_location` reads it (natively `to`), and `navigate` sets that.
     A browser writes the address its own way, `#/posts/café` as
     `#/posts/caf%C3%A9` and a space as `%20`, so the location set is the
     one the later `hashchange` finds, and the route is built once (found
     by the fact-check; mutants 24 and 25).
   - A fragment reaches the location in hash mode, as a second `#`
     (through `navigate`, a `link` or a typed hash), and natively through
     `navigate`; in history mode the page reports the path and query
     alone. `located` and `unfragmented` drop what follows the first `#`
     before any matching and before the query is read, so `#/posts/6#sec`
     shows post `6`, and so does `navigate("/posts/6#sec")` natively and
     under `HistoryRouter` (mutants 26 and 27).
6. **Going to the location already replaces the history entry in history
   mode:** `same_location` compares `to` and the location without
   fragments and percent-decoded, so `/posts/café` is the location
   `/posts/caf%C3%A9`, and then `navigate` calls `backend_replace`, a fifth
   backend function, instead of `backend_go`. A second click on the same
   link adds no entry, and a `to` that differs from the location only by
   its fragment replaces it too, since the router ignores fragments. In
   hash mode, setting the hash the page has already adds none in a
   browser. **(differs)** Spec §3.3 pushes always and §3.6 names four
   functions; the fact-check found the duplicate entries (mutants 28 to
   30).
7. **`link(attrs, children)` is a builder of an element's shape, with `to`
   an attribute** (spec §10.2), so a view writes `link(to: "/posts/1") {
   … }`, and a reactive `to` gives a reactive `href`. **(differs)** in what
   spec §3.3 left open:
   - `to` becomes `Dynamic("href", …)`, read when the link's `<a>` is built
     from its view, not when `link` is called, from the router mounted
     then. So a menu written before `Router(…)` in the same view, as
     `router_hash.nova`'s is, gets the router's form (mutant 35);
   - under a router, `to` gets one `/` in front, in an `href` and in
     `navigate` alike: `rooted` drops every tab, LF and CR, which a URL's
     parser deletes before it reads a URL, then makes a leading run of `/`
     and `\` that one `/`. So `//posts/8`, `/\posts/8` and `/\t/posts/8`
     are `/posts/8`, never a URL of another host, whose plain click
     `pushState` would refuse (found by the two checks; mutants 21 to 23
     and 92). `#/posts/1` under `Router`, `/posts/1` under
     `HistoryRouter`; with no router, `to` as written.
8. **The marker renders as `data-nova-link=""`,** the last attribute.
   **(differs)** Spec §3.7 wrote `<a href="#/…" data-nova-link>`;
   `render_to_string` writes every attribute with its value.
9. **Each router listens for `click` on the window** (spec §10.3): the
   nearest `<a>` with `data-nova-link` at or above the target, clicked with
   the main button and no modifier key, with no `target`, is navigated to,
   less a leading `#` in hash mode. **(differs)** Only the router mounted
   last takes the click: `backend_on_link_click(take: fn() -> Bool, go:
   fn(String))` asks `take()`, true for the router mounted last; when it
   is, it prevents the default and then calls `go` with the `href`, so a
   panic in `go` cannot hand a taken click back to the browser; otherwise
   it leaves the click alone. Spec §3.6 has one `fn(String)`, and §3.3
   prevents the default by the click alone, which left a router still
   mounted after a later one's disposal swallowing its links' clicks (the
   first check), and the first fix prevented it after the router's
   function returned (the second). `tests/dom/router_two` mounts two
   routers, disposes of the later, and sees the click left to the browser,
   its default not prevented and no navigation; the fake DOM follows no
   link, so its page stays as it was, where a browser would follow the
   link (mutants 46 and 47).
10. **`Event.modified()` joins `std/dom`** (spec §10.10): its shim function
    is true for Ctrl, Meta, Shift or Alt, or `(e.button ?? 0) !== 0`. The
    fake DOM's `FakeEvent` gains the four keys and `button`, and
    `click(modifiers)` passes them, so `router_history.mjs` clicks with
    each.
11. **Natively the location is `/`, and `navigate` sets the router's
    location alone** (spec §10.4, §3.7): the native backend gives `/`,
    goes nowhere and returns `to`, and listens for nothing, so
    `render_to_string(|| Router(routes))` writes the root route, and
    `ui_router.nova` renders a router after a `navigate` for each path.
12. **The panics** (spec §10.5, §3.3): `navigate: no router is mounted`, and
    `link: no \`to\` attribute`, which `ui_router_no_to.nova` reaches on
    both targets after a link with a `to` has rendered.
13. **Matching fills in spec §3.2.** **(differs)**
    - A pattern whose only piece is `*`, as `*` and `/*` both are, matches
      every location.
    - Patterns are split and percent-decoded as locations are, so
      `/posts/caf%C3%A9` and `/posts/café` are one pattern.
    - An empty part of a query, as in `a=1&&b=2&`, is skipped, not a key
      `""`; `?=v` is the key `""`.
14. **`router_hash.mjs` fires `hashchange` itself after `history.back()`.**
    A browser fires `hashchange` after `popstate` when going back changes
    the hash; the fake DOM's `back()` fires `popstate` alone, and the fake
    DOM does not change for it. Its address is a WHATWG `URL`, which
    percent-encodes a hash and a pushed path as a browser does, and a
    comment now says so.

**`nova dev`**

15. **The server checks `Host`** (spec §10.6), before the method.
    **(differs)** Spec §4.4 named the method's check first and set no
    order; here every request whose `Host` is not the server's is `403`,
    whatever its method, and so is one with no `Host`.
16. **Each build has an id, `<start>.<number>`:** the server's start in
    milliseconds since the Unix epoch, and the build's number from 1. Every
    page gets `<script src="/__nova/reload.js" data-build="<id>"></script>`
    with the id current when it is served, read before the page; `hello`
    and `reload` carry the current id; and `reload.js` reloads on a `hello`
    that is not its own tag's. **(differs)** Spec §4.4's tag has no id, and
    §4.5 and §10.7 sent the number and compared it with the first `hello`
    the script heard. That never reloads a page served before a build whose
    stream first connects after it, and a restarted server counts from 1
    again. A page served after a failed build carries that build's id, so
    its `hello` matches, and the `error` after it shows.
17. **The fallback** (spec §10.8): a `GET` or `HEAD` for a path that names
    no file, whose last piece has no `.`, gets the output directory's
    `index.html`; any other missing file is `404`. **(differs)** in two
    things the spec left open:
    - while the output directory has no `index.html`, the server's own page
      answers `/` and those paths alike, not `/` alone, so a history path
      shows the first build's errors too;
    - a directory without `index.html` falls back by the URL's last piece:
      `/assets` and `/assets/` get the page, `/app.js/` is `404`.
18. **The server's requests and answers fill in spec §4.4.** **(differs)**
    - A piece that is not one plain name to the system is `403`: on
      Windows `C:` would replace the directory it is joined to. On Unix
      only `..` is such a piece, which the check before it refuses
      already, so a mutant of the plain-name check alone cannot fail here:
      a `#[cfg(windows)]` loop of `/C:/x`, `/C:`, `/docs/c:/secret.txt` and
      `/%43:/x` exercises it on CI's Windows leg, and mutant 61 removes
      both checks.
    - A target that does not start with `/`, a `%` not followed by two hex
      digits, a path that does not decode to UTF-8, a head that is not
      UTF-8, two `Host` headers, and a version other than `HTTP/1.` and one
      digit are `400`. `HTTP/1.9` passes on purpose: RFC 9110 §2.5 has a
      higher minor version processed as the highest implemented. A `Host`
      ending in a bare CR is trimmed, which names nothing else.
    - A head's lines may end in CRLF or in a bare LF, which RFC 9112 §2.2
      lets a recipient take as a line end, and the head ends at its first
      empty line; a bare-LF head waited out its timeout before (found by
      the fact-check).
    - `.mjs` is `text/javascript`, as `.js` is.
    - `HEAD /__nova/events` gets the stream's head with `Connection:
      close`, and its connection closes.
19. **Builds** (spec §4.3), as built. **(differs)** in three things:
    - the child also gets `RUST_LIB_BACKTRACE=0`, so an inherited
      `RUST_BACKTRACE` puts no backtrace of an error into the overlay; a
      panic keeps its own;
    - a good build prints the child's notes, then `rebuilt in <ms> ms`, the
      first build included;
    - every build, good or failed, takes the next number, so a page served
      after a failed build carries that build's id (decision 16).

    Killing `nova dev` other than by Ctrl-C at a terminal, which reaches
    the whole process group, leaves a running build to finish on its own;
    ADR 0044 records it among the consequences.
20. **Watching** (spec §10.9, §4.6): every 200 ms, by modification time and
    size, waiting one poll that sees nothing new before a build; the output
    directory is skipped by its resolved path, however `-o` spells it; a
    link to nothing is watched, with no time and size, so its target's
    coming and going is a change. **(differs)** For a single file, the file
    itself and the `.nova` files of its directory, not below it, where spec
    §4.6 applied the project's rules: its program cannot import a module
    from a directory below its own (E0900), and `nova dev ~/x.nova` read a
    whole tree every 200 ms (found by the fact-check).
21. **What `nova build --target wasm` refuses, `nova dev` refuses before it
    binds a port** (spec §4.1, §4.2): a library, and an output directory
    that holds the project or lies inside `public/`, by `bundle.rs`'s
    check, now `check_output_for`, which needs no program yet; the output
    directory is `cmd/run.rs`'s `wasm_output`, shared with the build.
    **(differs)** It also refuses a package in nova's cache, as `nova
    build` does.
22. **The generated page loads `/app.js` by its absolute path,** by the
    user's decision 9 of 2026-10-18, as Vite's base `/` does, so a
    `HistoryRouter` page reloads at any path under `nova dev`, or under any
    server that serves the output directory from `/`; under `/posts/1` a
    relative `./app.js` would be `/posts/app.js`, a path with a `.` that
    gets no page. A project's own `index.html` must do the same for that,
    and an app deployed under a sub-path writes its own page.
    `tests/browser/router-app` has no page of its own, so `router.mjs`'s
    reload at `/posts/2` proves it; `06-counter-spa` keeps its own page.
    **(differs)** The spec did not say, and 4.0's page loaded `./app.js`;
    the first draft kept it, which narrowed the user's decision 3 (mutant
    48).

---

### Task 0: The baseline

The branch is at this plan's commit, whose parent is `3bcd1c6`, the
approved spec. The plan's commit adds this file alone, so the patches
apply to it as to `3bcd1c6`.

- [ ] **Step 1: The tree.**
  - `git status --short` prints nothing, on
    `claude/loving-archimedes-frtf5q-45`.
  - `git diff --stat 3bcd1c6 HEAD` lists only
    `docs/superpowers/plans/2026-10-18-phase-4-5-router-and-nova-dev.md`.
  - `BASE=$(git rev-parse --short HEAD)`: the plan's commit, recorded in
    the ledger. Task 6's review reads from it.
  - `df -h /` shows at least 8G free; if not, remove scratch directories
    of earlier sub-phases first.
- [ ] **Step 2: Node and the browser.** `node --version` prints `v22.` and
  more; `ls -d /opt/node-tools/node_modules/playwright /opt/pw-browsers`
  lists both, for Task 4's local run (skip that run where either is
  missing, and say so in the ledger).
- [ ] **Step 3: The suite.** In a shell set up as the conventions say:

  ```sh
  cargo build --locked --workspace
  NOVA_REQUIRE_NODE=1 cargo test --locked --workspace --no-fail-fast > "$P/base-45.txt" 2>&1
  grep '^test result' "$P/base-45.txt" | awk '{p += $4; f += $6; i += $8} END {print p, f, i}'
  grep ' FAILED$' "$P/base-45.txt"
  ```

  Expected: `2317 1 10`, and one failure, `test
  net::tests::an_ipv6_connect_parks_on_its_first_poll_and_then_establishes
  ... FAILED`. It needs IPv6, which this machine lacks, as in every
  earlier phase. (`3bcd1c6`'s tree is `75aa2d4`'s and the spec; 4.4's last
  run there gave these figures.)
- [ ] **Step 4: The ledger.** Start `$P/ledger-45x.md`, headed with the
  base commit and the baseline's figures.

### Task 1: The router's state builtins

Spec §3.5, §5.2; decision 1.

**Files:**
- Modify:
  - `crates/nova-resolver/src/lib.rs` (`Builtin`'s three variants, their
    names, `STD_ONLY`)
  - `crates/nova-typeck/src/check.rs` (the checker's two lists)
  - `crates/nova-mir/src/lib.rs` (`RtFunc`), `lower.rs`
  - `crates/nova-runtime/src/ui.rs`, `lib.rs` (`symbols()`)
  - `std/rt/lib.nova`
- Test:
  - `crates/nova-resolver/src/lib.rs`, `crates/nova-typeck/src/check.rs`
    and `crates/nova-runtime/src/ui.rs` (tests alone)
  - `crates/nova-mir/tests/lower_tests.rs`

- [ ] **Step 1: The tests.** `patch_block t1-tests`:
  - the resolver: `the_state_builtins_are_std_only`, which pins `std/ui`'s
    three state builtins and the router's three in `STD_ONLY`, finds none
    from a program's module on either target, and each from `$std.ui`
    (Task 2 points it at `$std.ui.router`, which does not exist yet);
  - the checker: the router's three arms in
    `builtin_signatures_are_what_the_std_call_sites_use`'s exhaustive
    `expected`;
  - `lower_tests.rs`: `state_builtins_lower(kind, rts)`, the body of
    `the_ui_state_builtins_lower_to_the_runtime_or_std_rt`, which that test
    now calls with `"ui"`, and the new
    `the_router_state_builtins_lower_to_the_runtime_or_std_rt` with
    `"router"`: the runtime's calls natively, `std/rt`'s
    `rt_router_state_*` on WASM;
  - the runtime: `the_routers_state_is_a_root_of_its_own`, a second root
    apart from `std/ui`'s.
- [ ] **Step 2: Run, and see them fail.**
  - `cargo test -p nova-resolver --lib`. Expected: it does not build:
    ``error[E0599]: no variant, associated function, or constant named
    `RouterStatePresent` found for enum `Builtin` in the current scope``
    (`lib.rs:3862`), and the same for `RouterStateLoad` and
    `RouterStateStore`: ``could not compile `nova-resolver` (lib test) due
    to 3 previous errors``.
  - `cargo test -p nova-typeck --lib`. Expected: it does not build: the
    same three errors for `nova_resolver::Builtin` (`check.rs:16396`,
    `16400`, `16404`), ``could not compile `nova-typeck` (lib test) due to
    3 previous errors``.
  - `cargo test -p nova-mir --test lower_tests`. Expected: it does not
    build: ``error[E0432]: unresolved imports
    `nova_mir::RtFunc::RouterStateLoad`,
    `nova_mir::RtFunc::RouterStatePresent`,
    `nova_mir::RtFunc::RouterStateStore` `` (`lower_tests.rs:2104`), 1
    error.
  - `cargo test -p nova-runtime --lib ui::`. Expected: it does not build:
    ``error[E0425]: cannot find function `nova_rt_router_state_present` in
    this scope`` (`ui.rs:67`), and 7 such errors in all: `_present` twice,
    `_store` twice and `_load` three times.
- [ ] **Step 3: The code.** `patch_block t1-code`:
  - **`nova-resolver`:** `RouterStatePresent`, `RouterStateLoad` and
    `RouterStateStore`, named `router_state_present`, `_load` and
    `_store`; `STD_ONLY` grows from 84 to 87.
  - **`nova-typeck`:** the three in `check_builtin_call`'s list of
    std-only builtins whose wrong argument has no hint, and
    `builtin_signature`'s `() -> Bool`, `() -> T` and `(T) -> ()`, as
    `ui_state_*`'s.
  - **`nova-mir`:** `RtFunc`'s three, `nova_rt_router_state_*` natively and
    `std/rt`'s `rt_router_state_*` on WASM, and their lowering.
  - **`nova-runtime`:** `ROUTER_STATE`, a second thread-local slot, and
    `store`, which both slots share: the record stored is rooted, the one
    it replaces released; `nova_rt_router_state_*`, and `symbols()`'s three
    entries for the JIT.
  - **`std/rt`:** `ROUTER_STATE` at state word 112, free below `DATA_BASE`
    at 128; `rt_router_state_present`, `_load` and `_store`; and
    `gc_collect` marks the word after `UI_STATE`.
- [ ] **Step 4: Run.**
  - `cargo test -p nova-resolver --lib the_state_builtins_are_std_only`.
    Expected: `1 passed`.
  - `cargo test -p nova-typeck --lib builtin_signatures_are_what_the_std_call_sites_use`.
    Expected: `1 passed`.
  - `cargo test -p nova-mir --test lower_tests state_builtins_lower`.
    Expected: `2 passed`.
  - `cargo test -p nova-runtime --lib ui::`. Expected: `2 passed`.
  - The whole binaries:
    - `cargo test -p nova-resolver`. Expected: `53 passed` (52 before).
    - `cargo test -p nova-typeck --lib`. Expected: `410 passed`, as
      before: the signature test changed, and none was added.
    - `cargo test -p nova-mir`. Expected: `36 passed` in the library, and
      `lower_tests` `61 passed` (60 before).
    - `cargo test -p nova-runtime --lib`. Expected: `263 passed; 1
      failed; 8 ignored`, the failure Task 0's IPv6 test ("Address family
      not supported by protocol").
    - `cargo test -p nova-std`. Expected: `2 passed`.
  - `W ui_`, since `std/rt` changed. Expected: `20 passed`.
- [ ] **Step 5: The runtime's library.** `cargo build --locked -p
  nova-runtime`, so that `target/debug/libnova_runtime.a`, which a debug
  `nova` links into every native binary, has the three new symbols before
  Task 2's doc tests build one. Expected: it ends with `Finished`.
- [ ] **Step 6: The checks,** as the conventions list them.
- [ ] **Step 7: Commit** `builtins: router_state_present, _load and _store, the router's state, on both targets`.

**Patch `t1-tests`:**

<!-- patch t1-tests -->

````diff
diff --git a/crates/nova-mir/tests/lower_tests.rs b/crates/nova-mir/tests/lower_tests.rs
index 907c8a7621b0136f41159075b9615b1ce6734b83..2d1b61e871d14672c9a112a37f468057361cbcce 100644
--- a/crates/nova-mir/tests/lower_tests.rs
+++ b/crates/nova-mir/tests/lower_tests.rs
@@ -2091,16 +2091,39 @@ fn a_std_extern_js_block_is_wasms_alone() {
 /// generic wrappers give the builtins their type, as `std/ui`'s do.
 #[test]
 fn the_ui_state_builtins_lower_to_the_runtime_or_std_rt() {
+    use nova_mir::RtFunc::{UiStateLoad, UiStatePresent, UiStateStore};
+    state_builtins_lower("ui", [UiStatePresent, UiStateLoad, UiStateStore]);
+}
+
+/// `std/ui/router`'s state builtins (spec
+/// `docs/superpowers/specs/2026-10-18-phase-4-5-router-and-nova-dev-design.md`
+/// §3.5) lower as `std/ui`'s do: to the runtime natively, to `std/rt` on
+/// WASM.
+#[test]
+fn the_router_state_builtins_lower_to_the_runtime_or_std_rt() {
+    use nova_mir::RtFunc::{RouterStateLoad, RouterStatePresent, RouterStateStore};
+    state_builtins_lower(
+        "router",
+        [RouterStatePresent, RouterStateLoad, RouterStateStore],
+    );
+}
+
+/// The state builtins `<kind>_state_present`, `_load` and `_store`, called
+/// from a stand-in std module, are the runtime calls `rts` natively, and
+/// calls of `std/rt`'s `rt_<kind>_state_*` on WASM.
+fn state_builtins_lower(kind: &str, rts: [nova_mir::RtFunc; 3]) {
     use nova_resolver::{resolve_program_for, CompileTarget, Def, DefKind, ModuleSource};
 
-    let stub = "record LowerTestsState { n: Int }\n\
-                fn lower_tests_load<T>() -> T { ui_state_load() }\n\
-                fn lower_tests_store<T>(s: T) { ui_state_store(s) }\n\
-                pub fn lower_tests_state() -> Int {\n\
-                    if !ui_state_present() { lower_tests_store(LowerTestsState { n: 7 }) }\n\
-                    let s: LowerTestsState = lower_tests_load()\n\
-                    s.n\n\
-                }\n";
+    let stub = format!(
+        "record LowerTestsState {{ n: Int }}\n\
+         fn lower_tests_load<T>() -> T {{ {kind}_state_load() }}\n\
+         fn lower_tests_store<T>(s: T) {{ {kind}_state_store(s) }}\n\
+         pub fn lower_tests_state() -> Int {{\n\
+             if !{kind}_state_present() {{ lower_tests_store(LowerTestsState {{ n: 7 }}) }}\n\
+             let s: LowerTestsState = lower_tests_load()\n\
+             s.n\n\
+         }}\n"
+    );
     let lower = |target: CompileTarget| {
         let (tokens, _) = lex(
             "fn main() { println(\"${lower_tests_state()}\") }",
@@ -2114,7 +2137,7 @@ fn the_ui_state_builtins_lower_to_the_runtime_or_std_rt() {
             target,
             &sources,
             &std_files,
-            Some((("$test.ui_stub", stub), FileId::DUMMY)),
+            Some((("$test.state_stub", stub.as_str()), FileId::DUMMY)),
         );
         assert!(
             resolved.diagnostics.is_empty(),
@@ -2140,33 +2163,27 @@ fn the_ui_state_builtins_lower_to_the_runtime_or_std_rt() {
         nova_mir::lower_module_with(&checked.module, &nova_mir::Options { runtime })
             .expect("lowers")
     };
-    use nova_mir::RtFunc::{UiStateLoad, UiStatePresent, UiStateStore};
+    let [present, load, store] = rts;
     let native = lower(CompileTarget::Native);
     assert_eq!(
-        count_rt(&stmts_of(&native, "lower_tests_state"), UiStatePresent),
-        1
-    );
-    assert_eq!(
-        count_rt(&stmts_of(&native, "lower_tests_load"), UiStateLoad),
-        1
-    );
-    assert_eq!(
-        count_rt(&stmts_of(&native, "lower_tests_store"), UiStateStore),
+        count_rt(&stmts_of(&native, "lower_tests_state"), present),
         1
     );
+    assert_eq!(count_rt(&stmts_of(&native, "lower_tests_load"), load), 1);
+    assert_eq!(count_rt(&stmts_of(&native, "lower_tests_store"), store), 1);
     let wasm = lower(CompileTarget::Wasm);
     for (f, rt) in [
-        ("lower_tests_state", "rt_ui_state_present."),
-        ("lower_tests_load", "rt_ui_state_load."),
-        ("lower_tests_store", "rt_ui_state_store."),
+        ("lower_tests_state", format!("rt_{kind}_state_present.")),
+        ("lower_tests_load", format!("rt_{kind}_state_load.")),
+        ("lower_tests_store", format!("rt_{kind}_state_store.")),
     ] {
         let stmts = stmts_of(&wasm, f);
-        for want in [UiStatePresent, UiStateLoad, UiStateStore] {
+        for want in rts {
             assert_eq!(count_rt(&stmts, want), 0, "{f}");
         }
         assert!(
             stmts.iter().any(
-                |s| matches!(s, nova_mir::Stmt::Call { callee, .. } if callee.starts_with(rt))
+                |s| matches!(s, nova_mir::Stmt::Call { callee, .. } if callee.starts_with(&rt))
             ),
             "{f} calls {rt}"
         );
diff --git a/crates/nova-resolver/src/lib.rs b/crates/nova-resolver/src/lib.rs
index 94adcde23b24a132f4edd725d72ccb0b3fd9b6fd..19fb8ff0f05849f6b74ef02034300877cf878e64 100644
--- a/crates/nova-resolver/src/lib.rs
+++ b/crates/nova-resolver/src/lib.rs
@@ -3842,6 +3842,46 @@ mod tests {
         }
     }
 
+    /// `std/ui`'s and `std/ui/router`'s state builtins are std's alone, each
+    /// pinned by name as the net builtins are (spec
+    /// `docs/superpowers/specs/2026-10-18-phase-4-5-router-and-nova-dev-design.md`
+    /// §3.5, §5.2): on either target, a program's module cannot name them,
+    /// and `std/ui`'s can.
+    #[test]
+    fn the_state_builtins_are_std_only() {
+        for target in [CompileTarget::Native, CompileTarget::Wasm] {
+            let r = resolve_for(target, "fn main() { }\n");
+            assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
+            let ui = (1..=target.std_modules().len())
+                .find(|&m| r.definitions.module_name(ModuleId(m as u32)) == "$std.ui")
+                .expect("std/ui is compiled");
+            for b in [
+                Builtin::UiStatePresent,
+                Builtin::UiStateLoad,
+                Builtin::UiStateStore,
+                Builtin::RouterStatePresent,
+                Builtin::RouterStateLoad,
+                Builtin::RouterStateStore,
+            ] {
+                assert!(
+                    Builtin::STD_ONLY.contains(&b),
+                    "{b:?} must be invisible to user code"
+                );
+                assert!(
+                    r.definitions.resolve_value(ModuleId(0), b.name()).is_none(),
+                    "{b:?} resolves in a program's module"
+                );
+                assert!(
+                    matches!(
+                        r.definitions.resolve_value(ModuleId(ui as u32), b.name()),
+                        Some(Res::Builtin(seen)) if seen == b
+                    ),
+                    "{b:?} must resolve inside std/ui"
+                );
+            }
+        }
+    }
+
     #[test]
     fn user_fn_named_panic_is_a_reserved_word() {
         // The deliberate counterpart of
diff --git a/crates/nova-runtime/src/ui.rs b/crates/nova-runtime/src/ui.rs
index 813e69fc804bbd99fa58d7bee6f7ab6a9b5d7c72..5285614e582aba0ffa33f58035e904a0bddb2567 100644
--- a/crates/nova-runtime/src/ui.rs
+++ b/crates/nova-runtime/src/ui.rs
@@ -58,4 +58,27 @@ mod tests {
         assert_eq!(gc::root_count(b as usize), 1);
         assert_eq!(gc::root_count(a as usize), 0);
     }
+
+    /// The router's slot is a second root, apart from `std/ui`'s: storing
+    /// in it leaves `std/ui`'s slot empty, it reads back what was stored,
+    /// and a record replaced in it is no longer a root.
+    #[test]
+    fn the_routers_state_is_a_root_of_its_own() {
+        assert_eq!(nova_rt_router_state_present(), 0);
+        let a = crate::nova_rt_alloc(16);
+        nova_rt_router_state_store(a);
+        assert_eq!(nova_rt_router_state_present(), 1);
+        assert_eq!(nova_rt_ui_state_present(), 0);
+        assert_eq!(nova_rt_router_state_load(), a);
+        assert_eq!(gc::root_count(a as usize), 1);
+        let b = crate::nova_rt_alloc(16);
+        nova_rt_router_state_store(b);
+        assert_eq!(nova_rt_router_state_load(), b);
+        assert_eq!(gc::root_count(b as usize), 1);
+        assert_eq!(gc::root_count(a as usize), 0);
+        let u = crate::nova_rt_alloc(16);
+        nova_rt_ui_state_store(u);
+        assert_eq!(nova_rt_ui_state_load(), u);
+        assert_eq!(nova_rt_router_state_load(), b);
+    }
 }
diff --git a/crates/nova-typeck/src/check.rs b/crates/nova-typeck/src/check.rs
index d800dbb82bb55b1bcdb696ded3eee547d55c3e76..c757b677cc0bcbbd47fd9d6c4c8968d86d417447 100644
--- a/crates/nova-typeck/src/check.rs
+++ b/crates/nova-typeck/src/check.rs
@@ -16393,6 +16393,18 @@ mod tests {
                     (vec![Ty::Param(0)], Ty::Unit),
                     "`ui_state_store(state)` in `std/ui`'s `store_state`",
                 ),
+                Builtin::RouterStatePresent => (
+                    (vec![], Ty::Bool),
+                    "`router_state_present()` in `std/ui/router`'s `routing`",
+                ),
+                Builtin::RouterStateLoad => (
+                    (vec![], Ty::Param(0)),
+                    "`router_state_load()` in `std/ui/router`'s `load_routing`",
+                ),
+                Builtin::RouterStateStore => (
+                    (vec![Ty::Param(0)], Ty::Unit),
+                    "`router_state_store(state)` in `std/ui/router`'s `store_routing`",
+                ),
                 Builtin::MemLoad8 | Builtin::MemLoad64 => (
                     (vec![Ty::Int], Ty::Int),
                     "`mem_load8(addr)` / `mem_load64(addr)` in `std/rt`",
````

**Patch `t1-code`:**

<!-- patch t1-code -->

````diff
diff --git a/crates/nova-mir/src/lib.rs b/crates/nova-mir/src/lib.rs
index 1d0c2c81057fb8a16b80dff00c0eda90c600b780..b3595164247037b5928b3d79886889b9567e0a94 100644
--- a/crates/nova-mir/src/lib.rs
+++ b/crates/nova-mir/src/lib.rs
@@ -537,6 +537,14 @@ rt_funcs! {
     UiStateLoad,
     /// `(ptr state) -> unit` — keep `state`, a collector root from then on.
     UiStateStore,
+    /// `() -> i8` — whether `std/ui/router` has stored its state for this
+    /// thread (`crates/nova-runtime/src/ui.rs`; spec 4.5 §3.5).
+    RouterStatePresent,
+    /// `() -> ptr` — the router's state stored, a record.
+    RouterStateLoad,
+    /// `(ptr state) -> unit` — keep the router's `state`, a second collector
+    /// root from then on.
+    RouterStateStore,
 }
 
 impl RtFunc {
@@ -640,6 +648,9 @@ impl RtFunc {
             RtFunc::UiStatePresent => "nova_rt_ui_state_present",
             RtFunc::UiStateLoad => "nova_rt_ui_state_load",
             RtFunc::UiStateStore => "nova_rt_ui_state_store",
+            RtFunc::RouterStatePresent => "nova_rt_router_state_present",
+            RtFunc::RouterStateLoad => "nova_rt_router_state_load",
+            RtFunc::RouterStateStore => "nova_rt_router_state_store",
         }
     }
 
@@ -833,6 +844,9 @@ impl RtFunc {
             RtFunc::UiStatePresent => (vec![], MirTy::I8),
             RtFunc::UiStateLoad => (vec![], MirTy::Ptr),
             RtFunc::UiStateStore => (vec![MirTy::Ptr], MirTy::Unit),
+            RtFunc::RouterStatePresent => (vec![], MirTy::I8),
+            RtFunc::RouterStateLoad => (vec![], MirTy::Ptr),
+            RtFunc::RouterStateStore => (vec![MirTy::Ptr], MirTy::Unit),
         }
     }
 }
@@ -950,6 +964,9 @@ impl RtFunc {
             RtFunc::UiStatePresent => WasmRuntime::Nova("rt_ui_state_present"),
             RtFunc::UiStateLoad => WasmRuntime::Nova("rt_ui_state_load"),
             RtFunc::UiStateStore => WasmRuntime::Nova("rt_ui_state_store"),
+            RtFunc::RouterStatePresent => WasmRuntime::Nova("rt_router_state_present"),
+            RtFunc::RouterStateLoad => WasmRuntime::Nova("rt_router_state_load"),
+            RtFunc::RouterStateStore => WasmRuntime::Nova("rt_router_state_store"),
             RtFunc::TestSelector
             | RtFunc::FsReadToString
             | RtFunc::FsWriteString
diff --git a/crates/nova-mir/src/lower.rs b/crates/nova-mir/src/lower.rs
index c30bbf87f1efcd29fc9618af35c079ccea3616bc..240c702704f9cff91b7226bc6cdff8e451759f67 100644
--- a/crates/nova-mir/src/lower.rs
+++ b/crates/nova-mir/src/lower.rs
@@ -790,6 +790,9 @@ impl<'a> Lowerer<'a> {
                     Builtin::UiStatePresent => Lowering::Runtime(RtFunc::UiStatePresent),
                     Builtin::UiStateLoad => Lowering::Runtime(RtFunc::UiStateLoad),
                     Builtin::UiStateStore => Lowering::Runtime(RtFunc::UiStateStore),
+                    Builtin::RouterStatePresent => Lowering::Runtime(RtFunc::RouterStatePresent),
+                    Builtin::RouterStateLoad => Lowering::Runtime(RtFunc::RouterStateLoad),
+                    Builtin::RouterStateStore => Lowering::Runtime(RtFunc::RouterStateStore),
                     Builtin::MemLoad8 => Lowering::Mem(MemOp::Load8),
                     Builtin::MemStore8 => Lowering::Mem(MemOp::Store8),
                     Builtin::MemLoad64 => Lowering::Mem(MemOp::Load64),
diff --git a/crates/nova-resolver/src/lib.rs b/crates/nova-resolver/src/lib.rs
index 19fb8ff0f05849f6b74ef02034300877cf878e64..291eb2d390ad54c22c1710521c11800da5709dbc 100644
--- a/crates/nova-resolver/src/lib.rs
+++ b/crates/nova-resolver/src/lib.rs
@@ -915,6 +915,24 @@ builtins! {
     /// in `std/rt`'s state word 104, which the collector marks. Runtime
     /// symbol `nova_rt_ui_state_store`. Std-only.
     UiStateStore,
+    /// `router_state_present() -> Bool` — whether `std/ui/router` has stored
+    /// its state for this thread, or this page (spec
+    /// `docs/superpowers/specs/2026-10-18-phase-4-5-router-and-nova-dev-design.md`
+    /// §3.5), as [`Builtin::UiStatePresent`] is for `std/ui`. Runtime symbol
+    /// `nova_rt_router_state_present`; on WASM, `std/rt`'s
+    /// `rt_router_state_present`. Std-only.
+    RouterStatePresent,
+    /// `router_state_load<T>() -> T` — the state
+    /// [`Builtin::RouterStateStore`] stored. `T` is the caller's first type
+    /// parameter, as for [`Builtin::UiStateLoad`]; `std/ui/router` calls it
+    /// with its own record. Runtime symbol `nova_rt_router_state_load`.
+    /// Std-only.
+    RouterStateLoad,
+    /// `router_state_store<T>(state: T)` — keep `state`, a record, for the
+    /// thread or the page: natively a second collector root beside
+    /// `std/ui`'s, on WASM in `std/rt`'s state word 112, which the collector
+    /// marks. Runtime symbol `nova_rt_router_state_store`. Std-only.
+    RouterStateStore,
     /// `mem_load8(addr: Int) -> Int` — the byte at `addr`, zero-extended.
     /// `std/rt`, the runtime of a WASM build, reads memory through this and
     /// the eight builtins after it (spec
@@ -1065,6 +1083,9 @@ impl Builtin {
             Builtin::UiStatePresent => "ui_state_present",
             Builtin::UiStateLoad => "ui_state_load",
             Builtin::UiStateStore => "ui_state_store",
+            Builtin::RouterStatePresent => "router_state_present",
+            Builtin::RouterStateLoad => "router_state_load",
+            Builtin::RouterStateStore => "router_state_store",
             Builtin::MemLoad8 => "mem_load8",
             Builtin::MemStore8 => "mem_store8",
             Builtin::MemLoad64 => "mem_load64",
@@ -1105,7 +1126,7 @@ impl Builtin {
     /// consecutive review rounds (see the Phase 2.2b whole-branch review),
     /// because the roster is duplicated information that only this array
     /// needs to stay exact.
-    pub const STD_ONLY: [Builtin; 84] = [
+    pub const STD_ONLY: [Builtin; 87] = [
         Builtin::StrCmp,
         Builtin::StrHash,
         Builtin::CharToInt,
@@ -1190,6 +1211,9 @@ impl Builtin {
         Builtin::UiStatePresent,
         Builtin::UiStateLoad,
         Builtin::UiStateStore,
+        Builtin::RouterStatePresent,
+        Builtin::RouterStateLoad,
+        Builtin::RouterStateStore,
     ];
 
     /// Builtins seeded into an std module's scope only when the target is
diff --git a/crates/nova-runtime/src/lib.rs b/crates/nova-runtime/src/lib.rs
index 1f4cfb5d9d366ac6a8b71a0213f7018c8df1366b..2fea72e1b0578dd67b16dba1fa4bd595e7fb3dae 100644
--- a/crates/nova-runtime/src/lib.rs
+++ b/crates/nova-runtime/src/lib.rs
@@ -99,7 +99,7 @@ pub mod signal;
 pub mod task;
 /// The monotonic clock behind `std/time`.
 mod time;
-/// The state of `std/ui`, kept for the thread.
+/// The state of `std/ui` and of `std/ui/router`, kept for the thread.
 mod ui;
 
 /// A Nova string value: immutable UTF-8, `{ len, ptr }`.
@@ -1255,6 +1255,18 @@ pub fn symbols() -> Vec<(&'static str, *const u8)> {
             "nova_rt_ui_state_store",
             ui::nova_rt_ui_state_store as *const u8,
         ),
+        (
+            "nova_rt_router_state_present",
+            ui::nova_rt_router_state_present as *const u8,
+        ),
+        (
+            "nova_rt_router_state_load",
+            ui::nova_rt_router_state_load as *const u8,
+        ),
+        (
+            "nova_rt_router_state_store",
+            ui::nova_rt_router_state_store as *const u8,
+        ),
     ]
 }
 
diff --git a/crates/nova-runtime/src/ui.rs b/crates/nova-runtime/src/ui.rs
index 5285614e582aba0ffa33f58035e904a0bddb2567..3182f08a82cf6f50ca10514008cd15157b22db59 100644
--- a/crates/nova-runtime/src/ui.rs
+++ b/crates/nova-runtime/src/ui.rs
@@ -4,14 +4,32 @@
 //! §4). The current computation and owner, the effects waiting to run, and
 //! the roots a program keeps all hang from it, so it is a collector root
 //! from its first store for the rest of the thread's life.
+//!
+//! `std/ui/router` keeps a record of its own the same way, a second root
+//! (spec `docs/superpowers/specs/2026-10-18-phase-4-5-router-and-nova-dev-design.md`
+//! §3.5): the router's location, its mode, and the parameters of the route
+//! being rendered.
 
 use std::cell::Cell;
+use std::thread::LocalKey;
 
 use crate::gc;
 
 thread_local! {
-    /// The record's address, or 0 before the first store.
+    /// `std/ui`'s record's address, or 0 before the first store.
     static STATE: Cell<usize> = const { Cell::new(0) };
+    /// `std/ui/router`'s record's address, or 0 before the first store.
+    static ROUTER_STATE: Cell<usize> = const { Cell::new(0) };
+}
+
+/// Keep `state` in `slot`, rooting it and releasing the root of the record
+/// it replaces.
+fn store(slot: &'static LocalKey<Cell<usize>>, state: *mut u8) {
+    gc::add_root(state);
+    let old = slot.with(|s| s.replace(state as usize));
+    if old != 0 {
+        gc::remove_root(old as *mut u8);
+    }
 }
 
 /// `ui_state_present() -> Bool`: whether a state is stored, as `1` or `0`.
@@ -31,11 +49,28 @@ pub extern "C-unwind" fn nova_rt_ui_state_load() -> *mut u8 {
 /// root of the record it replaces.
 #[no_mangle]
 pub extern "C-unwind" fn nova_rt_ui_state_store(state: *mut u8) {
-    gc::add_root(state);
-    let old = STATE.with(|s| s.replace(state as usize));
-    if old != 0 {
-        gc::remove_root(old as *mut u8);
-    }
+    store(&STATE, state);
+}
+
+/// `router_state_present() -> Bool`: whether the router's state is stored,
+/// as `1` or `0`.
+#[no_mangle]
+pub extern "C-unwind" fn nova_rt_router_state_present() -> i8 {
+    i8::from(ROUTER_STATE.with(Cell::get) != 0)
+}
+
+/// `router_state_load<T>() -> T`: the router's record, or null before the
+/// first store, which `std/ui/router` never asks for.
+#[no_mangle]
+pub extern "C-unwind" fn nova_rt_router_state_load() -> *mut u8 {
+    ROUTER_STATE.with(Cell::get) as *mut u8
+}
+
+/// `router_state_store<T>(state: T)`: keep the router's `state`, rooting it
+/// and releasing the root of the record it replaces.
+#[no_mangle]
+pub extern "C-unwind" fn nova_rt_router_state_store(state: *mut u8) {
+    store(&ROUTER_STATE, state);
 }
 
 #[cfg(test)]
diff --git a/crates/nova-typeck/src/check.rs b/crates/nova-typeck/src/check.rs
index c757b677cc0bcbbd47fd9d6c4c8968d86d417447..44dffe9b84fb304f10cd9b1c28c8297e85dcf0ed 100644
--- a/crates/nova-typeck/src/check.rs
+++ b/crates/nova-typeck/src/check.rs
@@ -4275,6 +4275,9 @@ impl<'a> Checker<'a> {
             | Builtin::UiStatePresent
             | Builtin::UiStateLoad
             | Builtin::UiStateStore
+            | Builtin::RouterStatePresent
+            | Builtin::RouterStateLoad
+            | Builtin::RouterStateStore
             | Builtin::MemLoad8
             | Builtin::MemStore8
             | Builtin::MemLoad64
@@ -8153,6 +8156,9 @@ pub(crate) fn builtin_signature(builtin: Builtin) -> (Vec<Ty>, Ty) {
         Builtin::UiStatePresent => (vec![], Ty::Bool),
         Builtin::UiStateLoad => (vec![], Ty::Param(0)),
         Builtin::UiStateStore => (vec![Ty::Param(0)], Ty::Unit),
+        Builtin::RouterStatePresent => (vec![], Ty::Bool),
+        Builtin::RouterStateLoad => (vec![], Ty::Param(0)),
+        Builtin::RouterStateStore => (vec![Ty::Param(0)], Ty::Unit),
         Builtin::MemLoad8 | Builtin::MemLoad64 => (vec![Ty::Int], Ty::Int),
         Builtin::MemStore8 | Builtin::MemStore64 => (vec![Ty::Int, Ty::Int], Ty::Unit),
         Builtin::MemCopy | Builtin::MemFill => (vec![Ty::Int, Ty::Int, Ty::Int], Ty::Unit),
diff --git a/std/rt/lib.nova b/std/rt/lib.nova
index 4fb9fae32bf59bc9d075fe48f72c1ec4b3a614d7..490147486e77f3e238d71489a77a84ea844d4332 100644
--- a/std/rt/lib.nova
+++ b/std/rt/lib.nova
@@ -18,11 +18,11 @@
 // target's std sees.
 
 // The runtime's state words, at fixed addresses below the data segment
-// (spec 4.0 §4.1, 4.1 §6.1, 4.2 §4.2, §5.1, 4.3 §4): Nova has no mutable
-// globals. Each is named where it is used: `HEAP_META`, `STR_SEED`,
+// (spec 4.0 §4.1, 4.1 §6.1, 4.2 §4.2, §5.1, 4.3 §4, 4.5 §3.5): Nova has no
+// mutable globals. Each is named where it is used: `HEAP_META`, `STR_SEED`,
 // `STR_SEED_DRAWN`, `LOG_SET`, `LOG_LEVEL`, `EXEC`, `GC_SINCE`, `GC_NEXT`,
-// `GC_COUNT`, `GC_SWITCHES`, `HANDLES`, `CALLS` and `UI_STATE`, at 8 to
-// 104.
+// `GC_COUNT`, `GC_SWITCHES`, `HANDLES`, `CALLS`, `UI_STATE` and
+// `ROUTER_STATE`, at 8 to 112.
 
 // The allocator (spec 4.1 §6.2), ported from `nova-runtime`'s
 // `gc/pages.rs`. Memory from the first 64 KiB boundary at or above
@@ -1530,6 +1530,21 @@ fn rt_ui_state_load() -> Int { mem_load64(UI_STATE) }
 // `ui_state_store<T>(state: T)`.
 fn rt_ui_state_store(state: Int) { mem_store64(UI_STATE, state) }
 
+// The state of `std/ui/router` (spec
+// docs/superpowers/specs/2026-10-18-phase-4-5-router-and-nova-dev-design.md
+// §3.5): at state word 112, its record's address, or 0 before the first
+// store. The collector marks it beside `UI_STATE`'s.
+const ROUTER_STATE: Int = 112
+
+// `router_state_present() -> Bool`.
+fn rt_router_state_present() -> Bool { mem_load64(ROUTER_STATE) != 0 }
+
+// `router_state_load<T>() -> T`: the record.
+fn rt_router_state_load() -> Int { mem_load64(ROUTER_STATE) }
+
+// `router_state_store<T>(state: T)`.
+fn rt_router_state_store(state: Int) { mem_store64(ROUTER_STATE, state) }
+
 // JS's objects, as `std/dom` holds them (spec
 // docs/superpowers/specs/2026-10-16-phase-4-2-std-dom-and-the-shim-design.md
 // §4): a box is a 16-byte leaf holding a handle of the shim's table, then 0.
@@ -1639,6 +1654,8 @@ fn gc_collect() {
     }
     let u = mem_load64(UI_STATE)
     if u != 0 { gc_mark(m, u) }
+    let router = mem_load64(ROUTER_STATE)
+    if router != 0 { gc_mark(m, router) }
     let c = mem_load64(CALLS)
     if c != 0 {
         gc_mark(m, c)
````


### Task 2: `std/ui/router`, `Event.modified`, the backends, and the generated page

Spec §3, §5.2, and the user's decision 9; decisions 2 to 14 and 22.

**Files:**
- Create: `std/ui/router/lib.nova`
- Modify:
  - `std/dom/lib.nova`, `crates/nova-codegen-wasm/src/shim.rs`
    (`Event.modified`, and `INDEX_HTML`'s doc)
  - `crates/nova-codegen-wasm/src/shim/index.html` (`/app.js`)
  - `std/ui/backend/native.nova`, `wasm.nova`
  - `std/lib.rs`, `crates/nova-resolver/src/lib.rs` (the three lists of
    std modules, and the state builtins' test), `crates/nova-doc/src/model.rs`
    (its test's list of modules)
- Test:
  - `tests/runtime/ui_router.nova`, `ui_router.stdout`, `ui_router_no_to.nova`
  - `tests/dom/router_hash.nova`, `.mjs`, `.stdout`;
    `tests/dom/router_history.nova`, `.mjs`, `.stdout`;
    `tests/dom/router_two.nova`, `.mjs`, `.stdout`; `tests/dom/fake_dom.mjs`
  - `crates/nova-cli/tests/run_tests.rs`, `wasm_tests.rs`, `doc_test.rs`
  - `crates/nova-driver/tests/index_complete.rs`

- [ ] **Step 1: The tests.** `patch_block t2-tests`. Expected: `git apply`
  prints four `trailing whitespace.` notes, at the lines `unmounted:
  /posts/8 `, `after unmount: /posts/8 `, `/posts/7/8: ` and `/exact: `,
  then `warning: 4 lines add whitespace errors.`; and `grep -c ' $'
  tests/dom/router_history.stdout tests/runtime/ui_router.stdout` prints
  `2` for each: the spaces are the expected output (Global Constraints).
  The patch holds:
  - `tests/runtime/ui_router.nova`: natively the location is `/`, so each
    line renders a router after a `navigate`: empty pieces dropped,
    literals matched with their case, parameters percent-decoded (`%C3%A9`,
    `%2F` and `%2f`, `%20`) or kept as written (`100%`, `%zz`, `%FF`), `+`
    kept in a path, a `#fragment` dropped (`/posts/6#sec`,
    `/posts/6#a/b?c=d`), the first route that matches shown, `*` matching
    everything, and fourteen queries split and decoded, two of them cut at
    a `#`; then, with no router, empty maps and a link's `href` as written;
    under a router, a fixed and a reactive `to` written without their `/`;
    `//posts/8`, `/\posts/8` and `\//\posts/8`, each `/posts/8` in a link's
    `href` and in `navigate` under both routers, and so are four with a tab,
    LF or CR in the leading run, as `/\t/posts/8`; and a router that takes
    over from another and outlives its disposal. It joins
    `ui_programs_run_and_survive_gc_stress`, natively, and the gate as
    `gate::ui_router`, on WASM under the fake DOM at `http://localhost/`,
    whose location is `/` too, each as it is and under stress, with one
    `.stdout`.
  - `tests/runtime/ui_router_no_to.nova`: a link with a `to` renders, then
    one with only a class named `to` and an `href` panics, `link: no \`to\`
    attribute`: in `ui_programs_panic_with_their_message`, natively, and
    `gate::ui_router_no_to`. Both programs join the gate's `ALSO`.
  - `tests/dom/router_hash.nova` and `.mjs`, under the fake DOM: what shows
    at `/`; after a link's click, its default prevented, and a turn later
    when its `hashchange` comes, built once; after `navigate` with a query,
    from a button; after the back button, with the `hashchange` the script
    fires itself (decision 14); at a typed hash only `*` matches; after
    `navigate` to `/posts/café` and to `/posts/a b`, each built once though
    the hash holds them percent-encoded; after `navigate` to
    `/posts/6#sec`, whose second `#` the location drops; and at a
    percent-encoded parameter, from a `to` with no `/`. The menu's links,
    built before the router, have its `#`. `router_routes_by_the_hash`
    runs it as it is, then under stress, where every collection keeps six
    calls: the router's two window listeners and the four buttons'
    handlers.
  - `tests/dom/router_history.nova` and `.mjs`: the same steps with
    `HistoryRouter`, through `pushState` and `popstate`; a second click on
    the same link, which adds no entry; a handler that reads no parameters
    after a route's render; `navigate` to `/posts/café` while at
    `/posts/caf%C3%A9`, which adds no entry; links whose path and query
    carry a `#fragment`, which the location drops; clicks left to the
    browser, their default not prevented: with each modifier key and with
    another button, on a link with `target`, and on a plain `<a>`; a click
    on a link's child, which navigates; and, once the router's owner is
    disposed, a link's click that is the browser's, and `navigate`, which
    panics. `router_routes_by_the_history_api` wants status 134 and `nova:
    panic: navigate: no router is mounted`, and under stress seven calls in
    each collection, then five once the router's listeners are gone.
  - `tests/dom/router_two.nova` and `.mjs`: two routers on a page; the one
    mounted later takes a link's click, its default prevented, and both
    show the route; once it is disposed, the earlier one, still listening,
    leaves a link's click to the browser, its default not prevented and no
    navigation; the fake DOM follows no link, so its page stays as it was,
    where a browser would follow the link.
    `a_router_not_mounted_last_leaves_a_click_to_the_browser`
    runs it as it is and under stress, where the collections keep five
    calls, then three once the later router's two listeners are gone.
  - `tests/dom/fake_dom.mjs`: a `FakeEvent`'s `ctrlKey`, `metaKey`,
    `shiftKey`, `altKey` and `button`; `click(modifiers)`; and a comment on
    its address, a WHATWG `URL`, which encodes as a browser does.
  - `wasm_tests.rs`: besides the four tests above,
    `build_for_wasm_writes_dist` now wants `import init from "/app.js";`
    in the generated page (decision 22).
  - `doc_test.rs`: `std::ui::router`'s examples must run, as
    `std::ui::html`'s do.
  - `index_complete.rs`: 153 programs and 178 files.
- [ ] **Step 2: Run, and see them fail.**
  - `R ui_programs`. Expected: `0 passed; 2 failed`:
    `ui_programs_run_and_survive_gc_stress`, "Unexpected failure." with
    ``error[E0001]: cannot find module `ui::router` `` at
    `tests/runtime/ui_router.nova:14:8`; and
    `ui_programs_panic_with_their_message`, ``assertion `left == right`
    failed: ui_router_no_to``, `left: ""`, `right: "<a href=\"/fine\"
    data-nova-link=\"\">fine</a>\n"`.
  - `W router`. Expected: `0 passed; 5 failed`:
    `a_router_not_mounted_last_leaves_a_click_to_the_browser`,
    `gate::ui_router`, `gate::ui_router_no_to`, `router_routes_by_the_hash`
    and `router_routes_by_the_history_api`, each "Unexpected failure." from
    the build's ``error[E0001]: cannot find module `ui::router` `` (at
    `router_two.nova:11:8`, `router_hash.nova:13:8` and
    `router_history.nova:16:8` for the DOM tests).
  - `W build_for_wasm_writes_dist`. Expected: `0 passed; 1 failed`: the
    assertion at `wasm_tests.rs:515`, whose message is the generated page,
    with `import init from "./app.js";`.
  - `W every_runtime_program_is_in_the_gate_or_left_out`. Expected: `1
    passed`: the programs and the gate's lists arrive together.
  - `cargo test -p nova-cli --test doc_test doc_std_runs_stds_doc_tests`.
    Expected: `doc_std_runs_stds_doc_tests_inside_a_project_or_not` fails,
    "no example of std::ui::router ran" (`doc_test.rs:413`).
  - `X`. Expected: `every_name_in_the_corpora_is_indexed` fails
    (`index_complete.rs:257`): "…/tests/runtime/ui_router.nova does not
    check cleanly: [\"E0001 cannot find module `ui::router`\", …]".
- [ ] **Step 3: The code.** `patch_block t2-code`:
  - **`std/dom`:** `dom_event_modified`, an `extern "js"` function, and
    `Event.modified()`; the shim's `dom_event_modified` (decision 10).
  - **The generated page:** `shim/index.html` imports `init` from
    `/app.js`, and `INDEX_HTML`'s doc says why (decision 22).
  - **The backends:** `backend_location`; `backend_go`, which returns the
    location read back, and `backend_replace` (decisions 5 and 6);
    `backend_on_location`; and `backend_on_link_click(take: fn() -> Bool,
    go: fn(String))`, which on WASM walks up to the nearest `<a>` with
    `data-nova-link` (`nova_link`) and, for a plain click when `take()` is
    true, prevents its default, then calls `go` (decision 9). Natively
    `/`, nothing and `to`, and listeners of nothing (decision 11).
  - **`std/ui/router/lib.nova`:** `Route`; `Router` and `HistoryRouter`,
    components over `routed(history, routes)`, which takes over the state,
    listens until its owner is disposed, and returns
    `for_each(memo(…), rendered)` (decisions 2 to 4 and 9); `handled`;
    `navigate`, `located`, `unfragmented` and `same_location` (decisions 5
    and 6); `use_params`, `use_query`; `link`, `href_of` and `rooted`
    (decisions 7, 8 and 12); `matching`, `params_of`, `pieces_of`,
    `query_of`, `query_decoded`, `decoded` and `hex_digit` (decision 13).
    Its doc tests render natively: the module's, `Router`'s with no match,
    `HistoryRouter`'s link, `navigate`'s, `use_params`'s, `use_query`'s and
    `link`'s.
  - **Registration:** `std/lib.rs`'s `UI_ROUTER`, `ALL` (20) and its test's
    24 files; the resolver's `STD_MODULES` (20), `WASM_STD_MODULES` (16)
    and `IMPORTED_STD` (4), with their tests,
    `a_wasm_build_compiles_wasms_std_modules`,
    `a_nested_import_path_names_only_stds_modules` (which also checks that
    `import ui::router::{link}` binds `link` alone and `import ui` no
    `link`), and `the_state_builtins_are_std_only`, now at
    `$std.ui.router`; `nova-doc`'s list of modules gains `ui/router`.
- [ ] **Step 4: Run.**
  - `R ui_programs`. Expected: `2 passed`.
  - `W router`. Expected: `5 passed`.
  - `W build_for_wasm_writes_dist`. Expected: `1 passed`.
  - `cargo test -p nova-cli --test doc_test doc_std_runs_stds_doc_tests`.
    Expected: `1 passed`.
  - `X`. Expected: `1 passed`.
  - The whole binaries:
    - `cargo test -p nova-resolver`. Expected: `53 passed`.
    - `cargo test -p nova-std`. Expected: `2 passed`.
    - `cargo test -p nova-doc --lib`. Expected: `61 passed`.
    - `cargo test -p nova-codegen-wasm --lib`. Expected: `34 passed`.
    - `cargo test -p nova-cli --test doc_test`. Expected: `17 passed`.
    - `W`. Expected: `222 passed` (217 before).
- [ ] **Step 5: The checks,** as the conventions list them, and `cargo run
  --locked -q -p nova-cli -- fmt --check tests/runtime/ui_router.nova
  tests/runtime/ui_router_no_to.nova`, which prints nothing.
- [ ] **Step 6: Commit** `std/ui/router: Router, HistoryRouter, link, navigate, use_params and use_query; Event.modified; the generated page loads /app.js`.

**Patch `t2-tests`:**

<!-- patch t2-tests -->

````diff
diff --git a/crates/nova-cli/tests/doc_test.rs b/crates/nova-cli/tests/doc_test.rs
index 03b3b36be30f68a6f0feaf2992d88a03e8abca5d..b94fbc33ef17bebc0c6db55b154b473d5c24f295 100644
--- a/crates/nova-cli/tests/doc_test.rs
+++ b/crates/nova-cli/tests/doc_test.rs
@@ -409,10 +409,12 @@ fn doc_std_runs_stds_doc_tests_inside_a_project_or_not() {
                 "no example of std::{m} ran:\n{out}"
             );
         }
-        assert!(
-            out.contains("\ntest std::ui::html (std/ui/html/lib.nova:"),
-            "no example of std::ui::html ran:\n{out}"
-        );
+        for m in ["html", "router"] {
+            assert!(
+                out.contains(&format!("\ntest std::ui::{m} (std/ui/{m}/lib.nova:")),
+                "no example of std::ui::{m} ran:\n{out}"
+            );
+        }
         assert!(!out.contains("calc"), "{out}");
         assert_eq!(stderr(&a), "");
     }
diff --git a/crates/nova-cli/tests/run_tests.rs b/crates/nova-cli/tests/run_tests.rs
index 6dd04301dc7241149b0898fb9a9d330ae6057d10..980d43e23060252fe3aa6fd4447371cef1a408c3 100644
--- a/crates/nova-cli/tests/run_tests.rs
+++ b/crates/nova-cli/tests/run_tests.rs
@@ -11307,7 +11307,8 @@ fn example_05_geometry_runs() {
 
 /// `std/ui`'s programs (spec
 /// `docs/superpowers/specs/2026-10-17-phase-4-3-std-ui-reactivity-and-views-design.md`
-/// §9.2): each prints what `30-FRONTEND.md` §13's unit tests check, and runs
+/// §9.2), and `std/ui/router`'s paths and queries (spec 4.5 §3.2 to §3.5):
+/// each prints what `30-FRONTEND.md` §13's unit tests check, and runs
 /// the same with `NOVA_GC_STRESS=1` (collect on every allocation), so a part
 /// of the graph only the state's root keeps, freed, shows as wrong output or
 /// a crash.
@@ -11320,6 +11321,7 @@ fn ui_programs_run_and_survive_gc_stress() {
         "ui_render",
         "ui_collect",
         "ui_view",
+        "ui_router",
     ] {
         let source = repo_root().join(format!("tests/runtime/{name}.nova"));
         let expected = std::fs::read_to_string(source.with_extension("stdout"))
@@ -11338,8 +11340,8 @@ fn ui_programs_run_and_survive_gc_stress() {
 }
 
 /// `std/ui`'s panics (spec 4.3 §5, §6.1): `set` on a memo, an effect that
-/// never settles, and a name that is not a tag's, each with what the
-/// program printed first.
+/// never settles, a name that is not a tag's, and a link with no `to` (spec
+/// 4.5 §3.3), each with what the program printed first.
 #[test]
 fn ui_programs_panic_with_their_message() {
     for (name, out, message) in [
@@ -11358,6 +11360,11 @@ fn ui_programs_panic_with_their_message() {
             "<my-tag>fine</my-tag>\n",
             "nova: panic: std/ui: \"p onclick=x\" is not a tag name\n",
         ),
+        (
+            "ui_router_no_to",
+            "<a href=\"/fine\" data-nova-link=\"\">fine</a>\n",
+            "nova: panic: link: no `to` attribute\n",
+        ),
     ] {
         let a = nova()
             .arg("run")
diff --git a/crates/nova-cli/tests/wasm_tests.rs b/crates/nova-cli/tests/wasm_tests.rs
index 19b64f4c20062057dc01c80b3095cb06e6526d94..edb0b63940992332b2e27ee6122ba1c979adfd90 100644
--- a/crates/nova-cli/tests/wasm_tests.rs
+++ b/crates/nova-cli/tests/wasm_tests.rs
@@ -510,7 +510,9 @@ fn build_for_wasm_writes_dist() {
     assert_eq!(stdout(&a), "built dist\n");
     let dist = dir.join("dist");
     let page = std::fs::read_to_string(dist.join("index.html")).expect("index.html");
-    assert!(page.contains("import init from \"./app.js\";"), "{page}");
+    // By its absolute path, so a `HistoryRouter` page reloads under any path
+    // (the user's decision of 2026-10-18; ADR 0044).
+    assert!(page.contains("import init from \"/app.js\";"), "{page}");
     let js = std::fs::read_to_string(dist.join("app.js")).expect("app.js");
     assert!(js.contains("export default async function init("), "{js}");
     let wasm = std::fs::read(dist.join("app.wasm")).expect("app.wasm");
@@ -1922,6 +1924,8 @@ mod gate {
         "ui_memo_set",
         "ui_loop",
         "ui_render_bad_name",
+        "ui_router",
+        "ui_router_no_to",
     ];
 
     /// `eprint` and `eprintln` write to stderr, as `run_tests.rs` checks
@@ -2073,6 +2077,50 @@ mod gate {
         );
     }
 
+    /// A link with no `to` panics before it needs a page (spec 4.5 §3.3).
+    #[test]
+    fn ui_router_no_to() {
+        super::gate_panics(
+            "ui_router_no_to",
+            "nova: panic: link: no `to` attribute",
+            |out| out == "<a href=\"/fine\" data-nova-link=\"\">fine</a>\n",
+        );
+    }
+
+    /// `std/ui/router`'s paths and queries (spec 4.5 §3.2 to §3.5): a
+    /// router reads the page's location on WASM, so the program runs under
+    /// the fake DOM, at `http://localhost/`, whose location is `/` as the
+    /// native one is, and prints what it prints natively, as it is and under
+    /// stress.
+    #[test]
+    fn ui_router() {
+        let Some(_) = super::node() else { return };
+        let tests = super::repo_root().join("tests");
+        let source = tests.join("runtime/ui_router.nova");
+        let expected = std::fs::read_to_string(source.with_extension("stdout"))
+            .expect("the expected output")
+            .replace("\r\n", "\n");
+        let dir = super::fresh("gate-ui_router");
+        std::fs::copy(tests.join("dom/fake_dom.mjs"), dir.join("fake_dom.mjs"))
+            .expect("copy the fake DOM");
+        std::fs::write(
+            dir.join("run.mjs"),
+            "import { install } from \"./fake_dom.mjs\";\ninstall();\n\
+             const { default: init } = await import(\"./dist/app.js\");\nawait init();\n",
+        )
+        .expect("the harness");
+        super::build_wasm(&dir, &source);
+        for env in [&[][..], &super::STRESS[..]] {
+            let a = super::Command::new("node")
+                .arg(dir.join("run.mjs"))
+                .envs(env.iter().copied())
+                .timeout(std::time::Duration::from_secs(60))
+                .assert()
+                .success();
+            assert_eq!(super::stdout(&a), expected, "{}", super::stderr(&a));
+        }
+    }
+
     #[test]
     fn panic_unwrap() {
         super::gate_panics("panic_unwrap", "nova: panic: called get on None", |out| {
@@ -2347,6 +2395,86 @@ fn ui_mount_never_inserts_html() {
     }
 }
 
+/// The router by the hash (spec
+/// `docs/superpowers/specs/2026-10-18-phase-4-5-router-and-nova-dev-design.md`
+/// §3, §5.2): the root route at `/`; a link's click, its default prevented,
+/// that builds its route once though the `hashchange` it causes comes a turn
+/// later; `navigate` with a query; the back button; a typed hash only `*`
+/// matches; `navigate` to `/posts/café` and to `/posts/a b`, each built
+/// once, though the hash holds them percent-encoded; to `/posts/6#sec`,
+/// whose second `#` starts a fragment the location drops; and a
+/// percent-encoded parameter, from a `to` with no `/`. The menu's links,
+/// made before the router, have its `#`. Under stress the collector runs
+/// after each step and keeps the router's state, at std/rt's word 112, and
+/// its two window listeners with the four buttons' handlers: six calls
+/// throughout.
+#[test]
+fn router_routes_by_the_hash() {
+    dom_program("router_hash", &[]);
+    let err = dom_program("router_hash", &STRESS);
+    if node().is_none() {
+        return;
+    }
+    let calls = calls_after_each(&err);
+    assert!(!calls.is_empty(), "{err}");
+    assert!(calls.iter().all(|&c| c == 6), "{err}");
+}
+
+/// Two routers on a page (spec 4.5 §3.1, §3.3): the router mounted later
+/// takes a link's click, its default prevented, and both show the route;
+/// once the later one is disposed, no router is the one mounted last, and
+/// the earlier one, still listening, leaves a link's click to the browser:
+/// its default is not prevented, and the router does not navigate (the fake
+/// DOM follows no link, so the page stays as it was). Under stress the
+/// collector keeps both routers' window listeners and the button's handler,
+/// five calls, until the later router's two leave std/rt's table: three.
+#[test]
+fn a_router_not_mounted_last_leaves_a_click_to_the_browser() {
+    dom_program("router_two", &[]);
+    let err = dom_program("router_two", &STRESS);
+    if node().is_none() {
+        return;
+    }
+    let calls = calls_after_each(&err);
+    let disposed = calls
+        .iter()
+        .position(|&c| c != 5)
+        .expect("a collection after the disposal");
+    assert!(disposed > 0, "{err}");
+    assert!(calls[disposed..].iter().all(|&c| c == 3), "{err}");
+}
+
+/// The router by the history API (spec 4.5 §3, §5.2): the same steps as by
+/// the hash, through `pushState` and `popstate`; then the clicks left to the
+/// browser, their default not prevented and the page as it was: a link's
+/// click with each modifier key and with another button, a link with a
+/// `target`, and a plain `<a>`. A click on a link's child navigates. A
+/// handler reads no route's parameters, and `navigate` to the current
+/// location written decoded adds no entry. Once the router's owner is
+/// disposed, a link's click is the browser's, and `navigate` panics. Under
+/// stress the router's two listeners leave std/rt's table then: seven calls
+/// become five.
+#[test]
+fn router_routes_by_the_history_api() {
+    for env in [&[][..], &STRESS[..]] {
+        let Some(a) = dom_run("router_history", env) else {
+            return;
+        };
+        let err = stderr(&a.code(134));
+        assert!(
+            err.contains("nova: panic: navigate: no router is mounted\n"),
+            "{err}"
+        );
+        if !env.is_empty() {
+            let calls = calls_after_each(&err);
+            let (last, before) = calls.split_last().expect("collections");
+            assert_eq!(*last, 5, "{err}");
+            assert!(!before.is_empty(), "{err}");
+            assert!(before.iter().all(|&c| c == 7), "{err}");
+        }
+    }
+}
+
 /// `import ui::html::{div}` checks on both targets; a program's own nested
 /// path is still E0900, `ui::nope` is E0001, and `signal` without an import
 /// is E0001 (spec 4.3 §9.5).
diff --git a/crates/nova-driver/tests/index_complete.rs b/crates/nova-driver/tests/index_complete.rs
index 8651c29adfe7f5ba4e355e803ce9698550db1ba3..793a227dea4d015c1d23536648ef0a8fa792eb0d 100644
--- a/crates/nova-driver/tests/index_complete.rs
+++ b/crates/nova-driver/tests/index_complete.rs
@@ -241,9 +241,9 @@ fn every_name_is_covered(a: &Analysis, file: FileId, failures: &mut Vec<String>)
 #[test]
 fn every_name_in_the_corpora_is_indexed() {
     let programs = programs();
-    // 8 examples, 142 top-level runtime programs and the modules program,
-    // on 2026-10-17, after 4.4 added `06-counter-spa` and `ui_view`.
-    assert_eq!(programs.len(), 151, "{programs:#?}");
+    // 8 examples, 144 top-level runtime programs and the modules program,
+    // on 2026-10-18, after 4.5 added `ui_router` and `ui_router_no_to`.
+    assert_eq!(programs.len(), 153, "{programs:#?}");
     let mut files = 0;
     let mut failures: Vec<String> = Vec::new();
     for (k, path) in programs.iter().enumerate() {
@@ -270,9 +270,9 @@ fn every_name_in_the_corpora_is_indexed() {
             every_name_is_covered(&a, file, &mut failures);
         }
     }
-    // 8 + 142 + 3 files of user code, 05-geometry's library's 2, and
-    // native std's 20.
-    assert_eq!(files, 175);
+    // 8 + 144 + 3 files of user code, 05-geometry's library's 2, and
+    // native std's 21.
+    assert_eq!(files, 178);
     failures.sort();
     failures.dedup();
     assert!(
diff --git a/tests/dom/fake_dom.mjs b/tests/dom/fake_dom.mjs
index 34d0254252f26a2cfccaac776be51493a5e49ae6..e469918abca212b5d879130b37f9cb51e2f61b68 100644
--- a/tests/dom/fake_dom.mjs
+++ b/tests/dom/fake_dom.mjs
@@ -18,6 +18,13 @@ class FakeEvent {
     this.bubbles = init.bubbles === true;
     this.cancelable = init.cancelable === true;
     this.key = init.key;
+    // A mouse event's modifier keys and button: none, and the main one,
+    // unless `init` says otherwise.
+    this.ctrlKey = init.ctrlKey === true;
+    this.metaKey = init.metaKey === true;
+    this.shiftKey = init.shiftKey === true;
+    this.altKey = init.altKey === true;
+    this.button = init.button ?? 0;
     this.target = null;
     this.currentTarget = null;
     this.defaultPrevented = false;
@@ -232,8 +239,10 @@ class FakeElement extends FakeNode {
     if (v) this.setAttribute("disabled", "");
     else this.removeAttribute("disabled");
   }
-  click() {
-    this.dispatchEvent(new FakeEvent("click", { bubbles: true, cancelable: true }));
+  // A click, plain or with `modifiers`, such as `{ ctrlKey: true }` or
+  // `{ button: 1 }`. It has no default action: a link is not followed.
+  click(modifiers = {}) {
+    this.dispatchEvent(new FakeEvent("click", { ...modifiers, bubbles: true, cancelable: true }));
   }
   querySelectorAll(selector) {
     const matches = compile(selector);
@@ -342,6 +351,12 @@ function later(target, type) {
   setTimeout(() => target.dispatchEvent(new FakeEvent(type)), 0);
 }
 
+// The address is a WHATWG `URL`, which percent-encodes as a browser does:
+// setting `hash` encodes the fragment percent-encode set (C0 controls, space,
+// `"`, `<`, `>`, `` ` `` and every code point above U+007E, as UTF-8), so
+// `#/posts/café` reads back as `#/posts/caf%C3%A9`; and `pushState`'s URL,
+// parsed against the address, has its path and query encoded by the path and
+// query sets, so `/posts/a b?q=é` reads back as `/posts/a%20b?q=%C3%A9`.
 class FakeLocation {
   constructor(win, url) {
     this.win = win;
diff --git a/tests/dom/router_hash.mjs b/tests/dom/router_hash.mjs
new file mode 100644
index 0000000000000000000000000000000000000000..30708e1c6de614f530030d6f174a423a4c750389
--- /dev/null
+++ b/tests/dom/router_hash.mjs
@@ -0,0 +1,52 @@
+// Drive `router_hash.nova` a turn apart, printing the hash and the routed
+// part of the page after each step: a link's click, `navigate` with a query
+// (the Search button), the back button, a hash typed in the address bar
+// that no route but `*` matches, `navigate` to `/posts/café` and to
+// `/posts/a b`, which the hash holds percent-encoded, and a link to a
+// percent-encoded parameter written without its `/`. A window listener added
+// after the router's says whether each click's default was prevented.
+import { FakeEvent, install } from "./fake_dom.mjs";
+install();
+const app = document.createElement("div");
+app.setAttribute("id", "app");
+document.body.appendChild(app);
+const { default: init } = await import("./dist/app.js");
+await init();
+window.addEventListener("click", (e) => console.log(`click: prevented=${e.defaultPrevented}`));
+const turn = () => new Promise((r) => setTimeout(r, 0));
+const page = document.getElementById("page");
+const show = (what) => console.log(`${what}: ${location.hash || "(no hash)"} ${page.innerHTML}`);
+console.log(`hrefs: ${document.querySelectorAll("a").map((a) => a.getAttribute("href")).join(" ")}`);
+show("at /");
+document.getElementById("post").click();
+show("link clicked");
+await turn();
+show("a turn later");
+document.getElementById("search").click();
+await turn();
+show("navigated");
+// A browser fires `hashchange` after `popstate` when going back changes
+// the hash; the fake DOM's `back` fires `popstate` alone.
+history.back();
+await turn();
+window.dispatchEvent(new FakeEvent("hashchange"));
+show("back");
+location.hash = "#/no/such/page";
+await turn();
+show("typed");
+document.getElementById("accent").click();
+await turn();
+show("café");
+document.getElementById("space").click();
+await turn();
+show("a space");
+document.getElementById("fragment").click();
+await turn();
+show("a second #");
+document.getElementById("cafe").click();
+await turn();
+show("encoded");
+document.getElementById("home").click();
+await turn();
+show("home");
+console.log(`history: ${history.length} entries`);
diff --git a/tests/dom/router_hash.nova b/tests/dom/router_hash.nova
new file mode 100644
index 0000000000000000000000000000000000000000..fdd5a8219cee9e7110e29e0ca54175b3a8b109ad
--- /dev/null
+++ b/tests/dom/router_hash.nova
@@ -0,0 +1,82 @@
+// The router in hash mode under the fake DOM (spec
+// docs/superpowers/specs/2026-10-18-phase-4-5-router-and-nova-dev-design.md
+// §3, §5.2): what shows at `/`, after a link's click, after `navigate` with
+// a query, after the back button, at a path only `*` matches, after
+// `navigate` to a path the browser writes percent-encoded (`café`, a space),
+// and with a percent-encoded parameter. The menu's links are made before the
+// router, and take its `#` all the same. Each route's component says when it
+// is built, so a navigation that built it twice would show: one whose
+// `hashchange` found the encoded hash unlike the location `navigate` set.
+
+import ui
+import ui::html::{button, div, id, nav, p, text}
+import ui::router::{Route, Router, link, navigate, use_params, use_query}
+
+component Home() {
+    println("built Home")
+    view {
+        p { text("home") }
+    }
+}
+
+component Post() {
+    let id = match use_params().get("id") {
+        Some(id) => id
+        None => "?"
+    }
+    println("built Post with id ${id}")
+    view {
+        p { text("post ${id}") }
+    }
+}
+
+component Search() {
+    let query = use_query()
+    let words = query.get("q").unwrap_or("")
+    let page = query.get("page").unwrap_or("1")
+    println("built Search with ${query.len()} keys, params ${use_params().len()}")
+    view {
+        p { text("search for ${words}, page ${page}") }
+    }
+}
+
+component NotFound() {
+    println("built NotFound")
+    view {
+        p { text("not found") }
+    }
+}
+
+component Menu() {
+    view {
+        nav {
+            link(id: "home", to: "/") { text("Home") }
+            link(id: "post", to: "/posts/7") { text("Post 7") }
+            link(id: "cafe", to: "posts/caf%C3%A9") { text("Café") }
+            button(id: "search", on_click: || navigate("/search?q=nova+router&page=2")) {
+                text("Search")
+            }
+            button(id: "accent", on_click: || navigate("/posts/café")) { text("Café") }
+            button(id: "space", on_click: || navigate("/posts/a b")) { text("A b") }
+            button(id: "fragment", on_click: || navigate("/posts/6#sec")) { text("Post 6") }
+        }
+    }
+}
+
+fn main() {
+    let routes = [
+        Route { path: "/", component: Home },
+        Route { path: "/posts/:id", component: Post },
+        Route { path: "/search", component: Search },
+        Route { path: "*", component: NotFound },
+    ]
+    mount(
+        "#app",
+        || view {
+            div {
+                Menu()
+                div(id: "page") { Router(routes) }
+            }
+        },
+    )
+}
diff --git a/tests/dom/router_hash.stdout b/tests/dom/router_hash.stdout
new file mode 100644
index 0000000000000000000000000000000000000000..aa0a372eabd47f21222029b32ce8b0b6bd5f48a1
--- /dev/null
+++ b/tests/dom/router_hash.stdout
@@ -0,0 +1,30 @@
+built Home
+hrefs: #/ #/posts/7 #/posts/caf%C3%A9
+at /: (no hash) <p>home</p>
+built Post with id 7
+click: prevented=true
+link clicked: #/posts/7 <p>post 7</p>
+a turn later: #/posts/7 <p>post 7</p>
+built Search with 2 keys, params 0
+click: prevented=false
+navigated: #/search?q=nova+router&page=2 <p>search for nova router, page 2</p>
+built Post with id 7
+back: #/posts/7 <p>post 7</p>
+built NotFound
+typed: #/no/such/page <p>not found</p>
+built Post with id café
+click: prevented=false
+café: #/posts/caf%C3%A9 <p>post café</p>
+built Post with id a b
+click: prevented=false
+a space: #/posts/a%20b <p>post a b</p>
+built Post with id 6
+click: prevented=false
+a second #: #/posts/6#sec <p>post 6</p>
+built Post with id café
+click: prevented=true
+encoded: #/posts/caf%C3%A9 <p>post café</p>
+built Home
+click: prevented=true
+home: #/ <p>home</p>
+history: 8 entries
diff --git a/tests/dom/router_history.mjs b/tests/dom/router_history.mjs
new file mode 100644
index 0000000000000000000000000000000000000000..78d196722d7fc4cb4c9f932b2fbf01b496614fb3
--- /dev/null
+++ b/tests/dom/router_history.mjs
@@ -0,0 +1,62 @@
+// Drive `router_history.nova` a turn apart, printing the location's path
+// and query and the routed part of the page after each step, and how many
+// history entries a second click on the same link adds. A window listener
+// added after the router's says whether each click's default was prevented.
+import { install } from "./fake_dom.mjs";
+install();
+const app = document.createElement("div");
+app.setAttribute("id", "app");
+document.body.appendChild(app);
+const { default: init } = await import("./dist/app.js");
+await init();
+window.addEventListener("click", (e) => console.log(`click: prevented=${e.defaultPrevented}`));
+const turn = () => new Promise((r) => setTimeout(r, 0));
+const page = document.getElementById("page");
+const show = (what) => console.log(`${what}: ${location.pathname}${location.search} ${page.innerHTML}`);
+const click = async (id, modifiers) => {
+  document.getElementById(id).click(modifiers);
+  await turn();
+};
+console.log(`hrefs: ${document.querySelectorAll("a").map((a) => a.getAttribute("href")).join(" ")}`);
+show("at /");
+await click("post");
+show("link clicked");
+const once = history.length;
+await click("post");
+show("clicked again");
+console.log(`the same link again: ${history.length - once} more entries`);
+await click("search");
+show("navigated");
+history.back();
+await turn();
+show("back");
+await click("elsewhere");
+show("unmatched");
+await click("cafe");
+show("encoded");
+await click("params");
+const encoded = history.length;
+await click("same");
+console.log(`the same place written decoded: ${history.length - encoded} more entries`);
+await click("frag");
+show(`a fragment, ${location.hash}`);
+await click("frag-query");
+show(`a fragment after a query, ${location.hash}`);
+for (const modifiers of [{ ctrlKey: true }, { metaKey: true }, { shiftKey: true }, { altKey: true }, { button: 1 }]) {
+  console.log(`modifiers: ${JSON.stringify(modifiers)}`);
+  await click("post", modifiers);
+}
+console.log("a link with target");
+await click("blank");
+console.log("a plain <a>");
+await click("plain");
+show("left to the browser");
+await click("inner");
+show("inner clicked");
+console.log(`history: ${history.length} entries`);
+await click("unmount");
+show("unmounted");
+await click("post");
+show("after unmount");
+await click("search");
+console.log("after navigate");
diff --git a/tests/dom/router_history.nova b/tests/dom/router_history.nova
new file mode 100644
index 0000000000000000000000000000000000000000..866eace88b3114ff19833bdd7457e02296c35b57
--- /dev/null
+++ b/tests/dom/router_history.nova
@@ -0,0 +1,93 @@
+// The router in history mode under the fake DOM (spec
+// docs/superpowers/specs/2026-10-18-phase-4-5-router-and-nova-dev-design.md
+// §3, §5.2): what shows at `/`, after a link's click, which pushes its path,
+// and after a second click on it, which replaces that entry; after
+// `navigate` with a query, after the back button's `popstate`, at a path only
+// `*` matches, with a percent-encoded parameter, and at a link's path and
+// query that carry a `#fragment`, which the location drops; then the
+// clicks left to the browser: one with a modifier key or another button, on
+// a link with `target`, and on a plain `<a>`. A click inside a link's child
+// navigates. A route's parameters are its render's alone: a handler reads
+// none. Once the router's owner is disposed, a link's click is the
+// browser's again, and `navigate` panics.
+
+import ui
+import ui::html::{a, button, div, id, nav, p, strong, text}
+import ui::router::{HistoryRouter, Route, link, navigate, use_params, use_query}
+
+component Home() {
+    println("built Home")
+    view {
+        p { text("home") }
+    }
+}
+
+component Post() {
+    let id = match use_params().get("id") {
+        Some(id) => id
+        None => "?"
+    }
+    println("built Post with id ${id}")
+    view {
+        p { text("post ${id}") }
+    }
+}
+
+component Search() {
+    let query = use_query()
+    let words = query.get("q").unwrap_or("")
+    let page = query.get("page").unwrap_or("1")
+    println("built Search with ${query.len()} keys, params ${use_params().len()}")
+    view {
+        p { text("search for ${words}, page ${page}") }
+    }
+}
+
+component NotFound() {
+    println("built NotFound")
+    view {
+        p { text("not found") }
+    }
+}
+
+fn main() {
+    let routes = [
+        Route { path: "/", component: Home },
+        Route { path: "/posts/:id", component: Post },
+        Route { path: "/search", component: Search },
+        Route { path: "*", component: NotFound },
+    ]
+    let routed = signal(true)
+    mount(
+        "#app",
+        || view {
+            div {
+                nav {
+                    link(id: "home", to: "/") { text("Home") }
+                    link(id: "post", to: "/posts/7") { text("Post 7") }
+                    link(id: "cafe", to: "/posts/caf%C3%A9") { text("Café") }
+                    link(id: "frag", to: "/posts/6#sec") { text("Post 6") }
+                    link(id: "frag-query", to: "/search?q=frag#x") { text("Search frag") }
+                    link(to: "/posts/8") { strong(id: "inner") { text("Post 8") } }
+                    link(id: "blank", to: "/posts/9", target: "_blank") { text("Post 9") }
+                    a(id: "plain", href: "/posts/5") { text("Post 5") }
+                    button(id: "search", on_click: || navigate("/search?q=nova+router&page=2")) {
+                        text("Search")
+                    }
+                    button(id: "elsewhere", on_click: || navigate("no/such/page")) {
+                        text("Elsewhere")
+                    }
+                    button(
+                        id: "params",
+                        on_click: || println("outside a render: ${use_params().len()} params"),
+                    ) {
+                        text("Params")
+                    }
+                    button(id: "same", on_click: || navigate("/posts/café")) { text("Café again") }
+                    button(id: "unmount", on_click: || routed.set(false)) { text("Unmount") }
+                }
+                div(id: "page") { show(routed, || HistoryRouter(routes)) }
+            }
+        },
+    )
+}
diff --git a/tests/dom/router_history.stdout b/tests/dom/router_history.stdout
new file mode 100644
index 0000000000000000000000000000000000000000..8fa27d228ebbe4646f604a7d362ef58961c0441a
--- /dev/null
+++ b/tests/dom/router_history.stdout
@@ -0,0 +1,57 @@
+built Home
+hrefs: / /posts/7 /posts/caf%C3%A9 /posts/6#sec /search?q=frag#x /posts/8 /posts/9 /posts/5
+at /: / <p>home</p>
+built Post with id 7
+click: prevented=true
+link clicked: /posts/7 <p>post 7</p>
+built Post with id 7
+click: prevented=true
+clicked again: /posts/7 <p>post 7</p>
+the same link again: 0 more entries
+built Search with 2 keys, params 0
+click: prevented=false
+navigated: /search?q=nova+router&page=2 <p>search for nova router, page 2</p>
+built Post with id 7
+back: /posts/7 <p>post 7</p>
+built NotFound
+click: prevented=false
+unmatched: /no/such/page <p>not found</p>
+built Post with id café
+click: prevented=true
+encoded: /posts/caf%C3%A9 <p>post café</p>
+outside a render: 0 params
+click: prevented=false
+built Post with id café
+click: prevented=false
+the same place written decoded: 0 more entries
+built Post with id 6
+click: prevented=true
+a fragment, #sec: /posts/6 <p>post 6</p>
+built Search with 1 keys, params 0
+click: prevented=true
+a fragment after a query, #x: /search?q=frag <p>search for frag, page 1</p>
+modifiers: {"ctrlKey":true}
+click: prevented=false
+modifiers: {"metaKey":true}
+click: prevented=false
+modifiers: {"shiftKey":true}
+click: prevented=false
+modifiers: {"altKey":true}
+click: prevented=false
+modifiers: {"button":1}
+click: prevented=false
+a link with target
+click: prevented=false
+a plain <a>
+click: prevented=false
+left to the browser: /search?q=frag <p>search for frag, page 1</p>
+built Post with id 8
+click: prevented=true
+inner clicked: /posts/8 <p>post 8</p>
+history: 7 entries
+click: prevented=false
+unmounted: /posts/8 
+click: prevented=false
+after unmount: /posts/8 
+click: prevented=false
+after navigate
diff --git a/tests/dom/router_two.mjs b/tests/dom/router_two.mjs
new file mode 100644
index 0000000000000000000000000000000000000000..55e12e27379f106a6e8a67d9373dbacf02330ab7
--- /dev/null
+++ b/tests/dom/router_two.mjs
@@ -0,0 +1,29 @@
+// Drive `router_two.nova` a turn apart, printing the hash and both routers'
+// parts of the page after each step: a link's click with both routers
+// mounted, then the later router disposed, then a click on another link. A
+// window listener added after the routers' says whether each click's default
+// was prevented.
+import { install } from "./fake_dom.mjs";
+install();
+const app = document.createElement("div");
+app.setAttribute("id", "app");
+document.body.appendChild(app);
+const { default: init } = await import("./dist/app.js");
+await init();
+window.addEventListener("click", (e) => console.log(`click: prevented=${e.defaultPrevented}`));
+const turn = () => new Promise((r) => setTimeout(r, 0));
+const part = (id) => document.getElementById(id).innerHTML;
+const show = (what) => console.log(`${what}: ${location.hash || "(no hash)"} first [${part("first")}] second [${part("second")}]`);
+const click = async (id) => {
+  document.getElementById(id).click();
+  await turn();
+};
+console.log(`hrefs: ${document.querySelectorAll("a").map((a) => a.getAttribute("href")).join(" ")}`);
+show("at /");
+await click("post");
+show("both mounted");
+await click("dispose");
+show("the later disposed");
+await click("other");
+show("left to the browser");
+console.log(`history: ${history.length} entries`);
diff --git a/tests/dom/router_two.nova b/tests/dom/router_two.nova
new file mode 100644
index 0000000000000000000000000000000000000000..9400e81fb07c0bb17c6cf8494f8a8243a09bae9e
--- /dev/null
+++ b/tests/dom/router_two.nova
@@ -0,0 +1,48 @@
+// Two routers on one page under the fake DOM (spec
+// docs/superpowers/specs/2026-10-18-phase-4-5-router-and-nova-dev-design.md
+// §3.1, §3.3; ADR 0044): the router mounted later takes a link's click,
+// which the earlier one, also listening, leaves alone. Once the later one is
+// disposed, no router is the one mounted last, so the earlier one, still on
+// the page, leaves a link's click to the browser: its default is not
+// prevented, and the router does not navigate. (The fake DOM performs no
+// default, so the page stays as it was; a browser follows the link.)
+
+import ui
+import ui::html::{button, div, id, p, text}
+import ui::router::{Route, Router, link, use_params}
+
+component Home() {
+    view {
+        p { text("home") }
+    }
+}
+
+component Post() {
+    let id = match use_params().get("id") {
+        Some(id) => id
+        None => "?"
+    }
+    view {
+        p { text("post ${id}") }
+    }
+}
+
+fn main() {
+    let routes = [
+        Route { path: "/", component: Home },
+        Route { path: "/posts/:id", component: Post },
+    ]
+    let later = signal(true)
+    mount(
+        "#app",
+        || view {
+            div {
+                div(id: "first") { Router(routes) }
+                div(id: "second") { show(later, || Router(routes)) }
+                link(id: "post", to: "/posts/7") { text("Post 7") }
+                link(id: "other", to: "/posts/8") { text("Post 8") }
+                button(id: "dispose", on_click: || later.set(false)) { text("Dispose") }
+            }
+        },
+    )
+}
diff --git a/tests/dom/router_two.stdout b/tests/dom/router_two.stdout
new file mode 100644
index 0000000000000000000000000000000000000000..26ea159055241484ae281882e1c8cdf344962fa3
--- /dev/null
+++ b/tests/dom/router_two.stdout
@@ -0,0 +1,9 @@
+hrefs: #/posts/7 #/posts/8
+at /: (no hash) first [<p>home</p>] second [<p>home</p>]
+click: prevented=true
+both mounted: #/posts/7 first [<p>post 7</p>] second [<p>post 7</p>]
+click: prevented=false
+the later disposed: #/posts/7 first [<p>post 7</p>] second []
+click: prevented=false
+left to the browser: #/posts/7 first [<p>post 7</p>] second []
+history: 2 entries
diff --git a/tests/runtime/ui_router.nova b/tests/runtime/ui_router.nova
new file mode 100644
index 0000000000000000000000000000000000000000..35391a49aaaa47ea49f23cd44e9b73bd720ccfe2
--- /dev/null
+++ b/tests/runtime/ui_router.nova
@@ -0,0 +1,245 @@
+// `std/ui/router`'s paths, queries and state (spec
+// docs/superpowers/specs/2026-10-18-phase-4-5-router-and-nova-dev-design.md
+// §3.2 to §3.5, §3.7). Natively the location is `/`, and `navigate` sets the
+// router's location alone, so each line renders a router after a
+// `navigate`: empty pieces dropped, literals matched with their case, `:name`
+// parameters percent-decoded or kept as written, the first route that
+// matches shown, `*` matching everything, queries split and decoded, and a
+// `#fragment` dropped before either. Then a link's `href`, fixed and
+// reactive; a leading run of `/` and `\` that is one `/` under either
+// router, in an `href` and in `navigate`; and a router that takes over from
+// another and outlives its disposal.
+import ui
+import ui::html::{attr, dyn_attr, id, p, text}
+import ui::router::{HistoryRouter, Route, Router, link, navigate, use_params, use_query}
+
+component Root() {
+    view {
+        p { text("root") }
+    }
+}
+
+component Posts() {
+    view {
+        p { text("posts") }
+    }
+}
+
+component Post() {
+    let id = use_params().get("id").unwrap_or("?")
+    view {
+        p { text("post [${id}]") }
+    }
+}
+
+component Comment() {
+    let params = use_params()
+    let id = params.get("id").unwrap_or("?")
+    let c = params.get("c").unwrap_or("?")
+    view {
+        p { text("comment ${c} of ${id}, ${params.len()} params") }
+    }
+}
+
+component Exact() {
+    view {
+        p { text("exact") }
+    }
+}
+
+component Ab() {
+    view {
+        p { text("a then b") }
+    }
+}
+
+component Ax() {
+    let x = use_params().get("x").unwrap_or("?")
+    view {
+        p { text("a then ${x}") }
+    }
+}
+
+component Everything() {
+    view {
+        p { text("everything") }
+    }
+}
+
+// How many keys the query has, and those of them this program asks about,
+// each as `'key'=[value]`.
+component Query() {
+    let q = use_query()
+    let mut shown = ""
+    let keys = ["a", "b", "flag", "a b", ""]
+    let mut i = 0
+    while i < keys.len() {
+        match q.get(keys[i]) {
+            Some(v) => shown = "${shown} '${keys[i]}'=[${v}]"
+            None => {}
+        }
+        i = i + 1
+    }
+    view {
+        p { text("${q.len()} keys:${shown}") }
+    }
+}
+
+// `routes` rendered after `navigate(to)`.
+fn at(routes: [Route], to: String) -> String {
+    render_to_string(
+        || {
+            let router = Router(routes)
+            navigate(to)
+            router
+        },
+    )
+}
+
+// A link to `to` and `routes` rendered after `navigate(to)`, under
+// `Router`, then under `HistoryRouter`.
+fn by_both(routes: [Route], to: String) -> String {
+    let hash = render_to_string(
+        || {
+            let router = Router(routes)
+            navigate(to)
+            p([], [link([attr("to", to)], [text("a")]), router])
+        },
+    )
+    let history = render_to_string(
+        || {
+            let router = HistoryRouter(routes)
+            navigate(to)
+            p([], [link([attr("to", to)], [text("a")]), router])
+        },
+    )
+    "${hash} ${history}"
+}
+
+// Each of `tos`, after `label`, and `routes` rendered there.
+fn each(label: String, routes: [Route], tos: [String]) {
+    let mut i = 0
+    while i < tos.len() {
+        println("${label}${tos[i]}: ${at(routes, tos[i])}")
+        i = i + 1
+    }
+}
+
+fn main() {
+    let routes = [
+        Route { path: "/", component: Root },
+        Route { path: "/posts", component: Posts },
+        Route { path: "/posts/:id", component: Post },
+        Route { path: "posts/:id/comments/:c/", component: Comment },
+        Route { path: "/Exact", component: Exact },
+        Route { path: "/a/b", component: Ab },
+        Route { path: "/a/:x", component: Ax },
+    ]
+    each(
+        "",
+        routes,
+        [
+            "/",
+            "",
+            "//",
+            "/posts/",
+            "posts",
+            "/posts//7",
+            "/posts/7?x=1",
+            "/posts/7/comments/3",
+            "/posts/7/8",
+            "/exact",
+            "/Exact",
+            "/a/b",
+            "/a/c",
+            "/posts/caf%C3%A9",
+            "/posts/a%2Fb",
+            "/posts/a%2fb",
+            "/posts/a%20b%3F",
+            "/posts/100%",
+            "/posts/%zz",
+            "/posts/%FF",
+            "/posts/a+b",
+            "/posts/6#sec",
+            "/posts/6#a/b?c=d",
+        ],
+    )
+    let catch_all = [
+        Route { path: "/posts/:id", component: Post },
+        Route { path: "*", component: Everything },
+        Route { path: "/", component: Root },
+    ]
+    each("* ", catch_all, ["/", "/posts/1", "/posts/1/2"])
+    let queries = [Route { path: "/q", component: Query }]
+    each(
+        "",
+        queries,
+        [
+            "/q",
+            "/q?",
+            "/q?a=1&b=2",
+            "/q?a=1&a=2",
+            "/q?flag",
+            "/q?a=x+y%21",
+            "/q?a+b=c",
+            "/q?=v",
+            "/q?a=1&&b=2&",
+            "/q?a=%zz",
+            "/q?a=b=c",
+            "/q?a=1?b=2",
+            "/q?a=1#x",
+            "/q?a=1#x&b=2",
+        ],
+    )
+    // With no router: nothing to read, and a link's `href` is its `to`.
+    println("no router: ${use_params().len()} params, ${use_query().len()} keys")
+    println(render_to_string(|| link([id("l"), attr("to", "posts/1")], [text("one")])))
+    // Under a router, a fixed and a reactive `to`, written without its `/`.
+    let n = signal(2)
+    println(
+        render_to_string(
+            || {
+                let router = Router(routes)
+                // On WASM, the location is the page's, which the
+                // navigations above changed.
+                navigate("/")
+                let fixed = link([attr("to", "posts/1")], [text("one")])
+                let moving = link([dyn_attr("to", || "posts/${n.value()}")], [text("two")])
+                p([], [fixed, moving, router])
+            },
+        ),
+    )
+    // No `href` and no history entry names another host.
+    let hosts = ["//posts/8", "/\\posts/8", "\\//\\posts/8"]
+    let mut h = 0
+    while h < hosts.len() {
+        println("${hosts[h]}: ${by_both(routes, hosts[h])}")
+        h = h + 1
+    }
+    // Nor with a tab, LF or CR in its leading run, which a URL's parser
+    // deletes: `/\t/posts/8` would be `//posts/8`.
+    let spaced = ["/\t/posts/8", "\t//posts/8", "/\n/posts/8", "/\r\\posts/8"]
+    let mut s = 0
+    while s < spaced.len() {
+        println("spaced ${s}: ${by_both(routes, spaced[s])}")
+        s = s + 1
+    }
+    // A router made later takes over; disposing of the first leaves it.
+    root(
+        |dispose_first: fn()| {
+            let first = Router(routes)
+            root(
+                |dispose_second: fn()| {
+                    let second = HistoryRouter(routes)
+                    dispose_first()
+                    navigate("/q?a=taken")
+                    let href = render_to_string(|| link([attr("to", "x")], []))
+                    println("after the first's disposal: ${use_query().len()} keys, ${href}")
+                    dispose_second()
+                    let none = render_to_string(|| link([attr("to", "x")], []))
+                    println("after the second's: ${use_query().len()} keys, ${none}")
+                },
+            )
+        },
+    )
+}
diff --git a/tests/runtime/ui_router.stdout b/tests/runtime/ui_router.stdout
new file mode 100644
index 0000000000000000000000000000000000000000..b3e92dcbe9a02dddf084c3b4b44dd34d6dc78ef7
--- /dev/null
+++ b/tests/runtime/ui_router.stdout
@@ -0,0 +1,52 @@
+/: <p>root</p>
+: <p>root</p>
+//: <p>root</p>
+/posts/: <p>posts</p>
+posts: <p>posts</p>
+/posts//7: <p>post [7]</p>
+/posts/7?x=1: <p>post [7]</p>
+/posts/7/comments/3: <p>comment 3 of 7, 2 params</p>
+/posts/7/8: 
+/exact: 
+/Exact: <p>exact</p>
+/a/b: <p>a then b</p>
+/a/c: <p>a then c</p>
+/posts/caf%C3%A9: <p>post [café]</p>
+/posts/a%2Fb: <p>post [a/b]</p>
+/posts/a%2fb: <p>post [a/b]</p>
+/posts/a%20b%3F: <p>post [a b?]</p>
+/posts/100%: <p>post [100%]</p>
+/posts/%zz: <p>post [%zz]</p>
+/posts/%FF: <p>post [%FF]</p>
+/posts/a+b: <p>post [a+b]</p>
+/posts/6#sec: <p>post [6]</p>
+/posts/6#a/b?c=d: <p>post [6]</p>
+* /: <p>everything</p>
+* /posts/1: <p>post [1]</p>
+* /posts/1/2: <p>everything</p>
+/q: <p>0 keys:</p>
+/q?: <p>0 keys:</p>
+/q?a=1&b=2: <p>2 keys: 'a'=[1] 'b'=[2]</p>
+/q?a=1&a=2: <p>1 keys: 'a'=[2]</p>
+/q?flag: <p>1 keys: 'flag'=[]</p>
+/q?a=x+y%21: <p>1 keys: 'a'=[x y!]</p>
+/q?a+b=c: <p>1 keys: 'a b'=[c]</p>
+/q?=v: <p>1 keys: ''=[v]</p>
+/q?a=1&&b=2&: <p>2 keys: 'a'=[1] 'b'=[2]</p>
+/q?a=%zz: <p>1 keys: 'a'=[%zz]</p>
+/q?a=b=c: <p>1 keys: 'a'=[b=c]</p>
+/q?a=1?b=2: <p>1 keys: 'a'=[1?b=2]</p>
+/q?a=1#x: <p>1 keys: 'a'=[1]</p>
+/q?a=1#x&b=2: <p>1 keys: 'a'=[1]</p>
+no router: 0 params, 0 keys
+<a id="l" href="posts/1" data-nova-link="">one</a>
+<p><a href="#/posts/1" data-nova-link="">one</a><a href="#/posts/2" data-nova-link="">two</a><p>root</p></p>
+//posts/8: <p><a href="#/posts/8" data-nova-link="">a</a><p>post [8]</p></p> <p><a href="/posts/8" data-nova-link="">a</a><p>post [8]</p></p>
+/\posts/8: <p><a href="#/posts/8" data-nova-link="">a</a><p>post [8]</p></p> <p><a href="/posts/8" data-nova-link="">a</a><p>post [8]</p></p>
+\//\posts/8: <p><a href="#/posts/8" data-nova-link="">a</a><p>post [8]</p></p> <p><a href="/posts/8" data-nova-link="">a</a><p>post [8]</p></p>
+spaced 0: <p><a href="#/posts/8" data-nova-link="">a</a><p>post [8]</p></p> <p><a href="/posts/8" data-nova-link="">a</a><p>post [8]</p></p>
+spaced 1: <p><a href="#/posts/8" data-nova-link="">a</a><p>post [8]</p></p> <p><a href="/posts/8" data-nova-link="">a</a><p>post [8]</p></p>
+spaced 2: <p><a href="#/posts/8" data-nova-link="">a</a><p>post [8]</p></p> <p><a href="/posts/8" data-nova-link="">a</a><p>post [8]</p></p>
+spaced 3: <p><a href="#/posts/8" data-nova-link="">a</a><p>post [8]</p></p> <p><a href="/posts/8" data-nova-link="">a</a><p>post [8]</p></p>
+after the first's disposal: 1 keys, <a href="/x" data-nova-link=""></a>
+after the second's: 0 keys, <a href="x" data-nova-link=""></a>
diff --git a/tests/runtime/ui_router_no_to.nova b/tests/runtime/ui_router_no_to.nova
new file mode 100644
index 0000000000000000000000000000000000000000..8ec1a70e2f7a83f46aae861da9fbffdd053ee32d
--- /dev/null
+++ b/tests/runtime/ui_router_no_to.nova
@@ -0,0 +1,12 @@
+// A link with no `to` attribute panics (spec
+// docs/superpowers/specs/2026-10-18-phase-4-5-router-and-nova-dev-design.md
+// §3.3), after one with a `to` renders: a class named `to`, or an `href`,
+// is not one.
+import ui
+import ui::html::{attr, class, text}
+import ui::router::{link}
+
+fn main() {
+    println(render_to_string(|| link([attr("to", "/fine")], [text("fine")])))
+    println(render_to_string(|| link([class("to"), attr("href", "/x")], [text("no to")])))
+}
````

**Patch `t2-code`:**

<!-- patch t2-code -->

````diff
diff --git a/crates/nova-codegen-wasm/src/shim.rs b/crates/nova-codegen-wasm/src/shim.rs
index bf5107e2107f57707336a1567abb37b59a76c0ed..241594dbdb851e6d510ed5980e3e4dbc85af8356 100644
--- a/crates/nova-codegen-wasm/src/shim.rs
+++ b/crates/nova-codegen-wasm/src/shim.rs
@@ -5,7 +5,9 @@
 
 use anyhow::{bail, Result};
 
-/// The page `dist/` holds: it loads `app.js` as a module and calls `init`.
+/// The page `dist/` holds: it loads `/app.js` as a module, by its absolute
+/// path, and calls `init`, so that a page in history mode served at
+/// `/posts/1` still finds it (the user's decision of 2026-10-18; ADR 0044).
 pub const INDEX_HTML: &str = include_str!("shim/index.html");
 
 /// `dist/package.json`: `app.js` is an ES module, whatever a
@@ -133,6 +135,9 @@ fn js_function(name: &str) -> Option<&'static str> {
             "(ev) => { const t = object(ev).target; return t === window ? 0n : handle(t); }"
         }
         "dom_event_key" => "(ev) => string(object(ev).key ?? \"\")",
+        "dom_event_modified" => {
+            "(ev) => { const e = object(ev); return e.ctrlKey || e.metaKey || e.shiftKey || e.altKey || (e.button ?? 0) !== 0 ? 1 : 0; }"
+        }
         "dom_prevent_default" => "(ev) => { object(ev).preventDefault(); }",
         "dom_stop_propagation" => "(ev) => { object(ev).stopPropagation(); }",
         "dom_click" => "(el) => { object(el).click(); }",
diff --git a/crates/nova-codegen-wasm/src/shim/index.html b/crates/nova-codegen-wasm/src/shim/index.html
index 788ad9ffe6785f316dca6b47a958125acd0fda2a..9243f9e23c00a2f3836a4cfe650f6a0d60cd0907 100644
--- a/crates/nova-codegen-wasm/src/shim/index.html
+++ b/crates/nova-codegen-wasm/src/shim/index.html
@@ -8,7 +8,7 @@
 <body>
 <div id="app"></div>
 <script type="module">
-import init from "./app.js";
+import init from "/app.js";
 init();
 </script>
 </body>
diff --git a/crates/nova-doc/src/model.rs b/crates/nova-doc/src/model.rs
index ee559044860e0e8c2893966aef9df0a1b7cf4e40..154b08c6e195e69b6668102759e4008b281971c8 100644
--- a/crates/nova-doc/src/model.rs
+++ b/crates/nova-doc/src/model.rs
@@ -1404,7 +1404,8 @@ mod tests {
                 "test",
                 "time",
                 "ui",
-                "ui/html"
+                "ui/html",
+                "ui/router"
             ]
         );
         let core = module(&d.site, "std/core/index.html");
diff --git a/crates/nova-resolver/src/lib.rs b/crates/nova-resolver/src/lib.rs
index 291eb2d390ad54c22c1710521c11800da5709dbc..3eb7e1d17e0269ea55edfd4ca83e4e5ba0ba4204 100644
--- a/crates/nova-resolver/src/lib.rs
+++ b/crates/nova-resolver/src/lib.rs
@@ -2104,18 +2104,19 @@ fn missing_names(target: CompileTarget) -> FxHashMap<String, &'static str> {
 }
 
 /// The embedded standard-library modules, compiled as implicit modules and
-/// glob-imported into every user module, but for `std/ui` and `std/ui/html`,
-/// which a program imports, and `std/ui`'s backend, which only std sees
-/// (spec 4.3 §3, §7.3). Order is significant only in that it
-/// fixes module indices; user modules always come first, then these in the
-/// order listed here — `std/core` stays first so its module index is
-/// unchanged from when it was the only embedded module. The sources come
+/// glob-imported into every user module, but for `std/ui`, `std/ui/html`
+/// and `std/ui/router`, which a program imports, and `std/ui`'s backend,
+/// which only std sees (spec 4.3 §3, §7.3; 4.5 §3). Order is significant
+/// only in that it fixes module indices; user modules always come first,
+/// then these in the order listed here — `std/core` stays first so its
+/// module index is unchanged from when it was the only embedded module.
+/// The sources come
 /// from the `nova-std` crate, which embeds `std/` at build time through
 /// paths inside its own package, so the compiler stays a single
 /// self-contained executable and `cargo package` keeps the files. Each name
 /// is `$std.*`, not a valid identifier, so it can never collide with a user
 /// module name or be named in an `import`.
-pub const STD_MODULES: [(&str, &str); 19] = [
+pub const STD_MODULES: [(&str, &str); 20] = [
     ("$std.core", nova_std::CORE),
     ("$std.bytes", nova_std::BYTES),
     ("$std.io", nova_std::IO),
@@ -2135,6 +2136,7 @@ pub const STD_MODULES: [(&str, &str); 19] = [
     ("$std.ui", nova_std::UI),
     ("$std.ui.html", nova_std::UI_HTML),
     ("$std.ui.backend", nova_std::UI_BACKEND_NATIVE),
+    ("$std.ui.router", nova_std::UI_ROUTER),
 ];
 
 /// What a program is compiled for (spec
@@ -2171,13 +2173,13 @@ impl CompileTarget {
 }
 
 /// The std modules a WASM build compiles (spec 4.0 §3.2, 4.1 §3.1, 4.2
-/// §3.2, 4.3 §3): the ten of [`STD_MODULES`] that need no operating
+/// §3.2, 4.3 §3, 4.5 §3): the ten of [`STD_MODULES`] that need no operating
 /// system, in the same order, then `std/dom`, which only an `import`
-/// reaches, then `std/ui`'s three, with the backend's WASM source, then
+/// reaches, then `std/ui`'s four, with the backend's WASM source, then
 /// `std/rt`, the runtime written in Nova. Their names are those of
 /// [`STD_MODULES`], so `std/core` keeps its place first, and the same order
 /// gives a name the same winner in std's glob imports.
-pub const WASM_STD_MODULES: [(&str, &str); 15] = [
+pub const WASM_STD_MODULES: [(&str, &str); 16] = [
     ("$std.core", nova_std::CORE),
     ("$std.bytes", nova_std::BYTES),
     ("$std.collections", nova_std::COLLECTIONS),
@@ -2192,17 +2194,19 @@ pub const WASM_STD_MODULES: [(&str, &str); 15] = [
     ("$std.ui", nova_std::UI),
     ("$std.ui.html", nova_std::UI_HTML),
     ("$std.ui.backend", nova_std::UI_BACKEND_WASM),
+    ("$std.ui.router", nova_std::UI_ROUTER),
     ("$std.rt", nova_std::RT),
 ];
 
 /// The std modules a program imports by name instead of finding in its
 /// prelude, by the name an `import` writes, and the module's own name
 /// (spec `docs/superpowers/specs/2026-10-16-phase-4-2-std-dom-and-the-shim-design.md`
-/// §3.2).
-pub const IMPORTED_STD: [(&str, &str); 3] = [
+/// §3.2; 4.5 §3).
+pub const IMPORTED_STD: [(&str, &str); 4] = [
     ("dom", "$std.dom"),
     ("ui", "$std.ui"),
     ("ui::html", "$std.ui.html"),
+    ("ui::router", "$std.ui.router"),
 ];
 
 /// The names a project's module or a dependency may not take: those of
@@ -3572,9 +3576,9 @@ mod tests {
     }
 
     /// A WASM build compiles its own std (spec 4.0 §3.2, 4.1 §3.1, 4.2
-    /// §3.2, 4.3 §3): ten of the native modules, in native's order, then
-    /// `std/dom`, `std/ui`'s three and `std/rt`, after the user's. A native
-    /// build has neither `std/dom` nor `std/rt`.
+    /// §3.2, 4.3 §3, 4.5 §3): ten of the native modules, in native's order,
+    /// then `std/dom`, `std/ui`'s four and `std/rt`, after the user's. A
+    /// native build has neither `std/dom` nor `std/rt`.
     #[test]
     fn a_wasm_build_compiles_wasms_std_modules() {
         let wasm = resolve_for(CompileTarget::Wasm, "fn main() { }\n");
@@ -3599,6 +3603,7 @@ mod tests {
                 "$std.ui",
                 "$std.ui.html",
                 "$std.ui.backend",
+                "$std.ui.router",
                 "$std.rt"
             ]
         );
@@ -3672,12 +3677,27 @@ mod tests {
         }
     }
 
-    /// A nested import path names a std module, as `ui::html` does; any
-    /// other is E0900 still, and one under std's name that names nothing is
-    /// E0001 (spec 4.3 §3).
+    /// A nested import path names a std module, as `ui::html` and
+    /// `ui::router` do; any other is E0900 still, and one under std's name
+    /// that names nothing is E0001 (spec 4.3 §3, 4.5 §3).
     #[test]
     fn a_nested_import_path_names_only_stds_modules() {
         for target in [CompileTarget::Native, CompileTarget::Wasm] {
+            let routed = resolve_for(target, "import ui::router::{link}\nfn main() { }\n");
+            assert!(routed.diagnostics.is_empty(), "{:?}", routed.diagnostics);
+            assert!(routed
+                .definitions
+                .resolve_value(ModuleId(0), "link")
+                .is_some());
+            assert!(routed
+                .definitions
+                .resolve_value(ModuleId(0), "navigate")
+                .is_none());
+            let bare = resolve_for(target, "import ui\nfn main() { }\n");
+            assert!(bare
+                .definitions
+                .resolve_value(ModuleId(0), "link")
+                .is_none());
             let listed = resolve_for(target, "import ui::html::{div}\nfn main() { }\n");
             assert!(listed.diagnostics.is_empty(), "{:?}", listed.diagnostics);
             assert!(listed
@@ -3870,15 +3890,15 @@ mod tests {
     /// pinned by name as the net builtins are (spec
     /// `docs/superpowers/specs/2026-10-18-phase-4-5-router-and-nova-dev-design.md`
     /// §3.5, §5.2): on either target, a program's module cannot name them,
-    /// and `std/ui`'s can.
+    /// and `std/ui/router`'s can.
     #[test]
     fn the_state_builtins_are_std_only() {
         for target in [CompileTarget::Native, CompileTarget::Wasm] {
             let r = resolve_for(target, "fn main() { }\n");
             assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
-            let ui = (1..=target.std_modules().len())
-                .find(|&m| r.definitions.module_name(ModuleId(m as u32)) == "$std.ui")
-                .expect("std/ui is compiled");
+            let router = (1..=target.std_modules().len())
+                .find(|&m| r.definitions.module_name(ModuleId(m as u32)) == "$std.ui.router")
+                .expect("std/ui/router is compiled");
             for b in [
                 Builtin::UiStatePresent,
                 Builtin::UiStateLoad,
@@ -3897,10 +3917,10 @@ mod tests {
                 );
                 assert!(
                     matches!(
-                        r.definitions.resolve_value(ModuleId(ui as u32), b.name()),
+                        r.definitions.resolve_value(ModuleId(router as u32), b.name()),
                         Some(Res::Builtin(seen)) if seen == b
                     ),
-                    "{b:?} must resolve inside std/ui"
+                    "{b:?} must resolve inside std/ui/router"
                 );
             }
         }
diff --git a/std/dom/lib.nova b/std/dom/lib.nova
index f286dc49132506130bf6b0d84f2a51f49c9b5a12..93b9c52fcd558e564cddb4c0550e14e3ceabaa4e 100644
--- a/std/dom/lib.nova
+++ b/std/dom/lib.nova
@@ -52,6 +52,7 @@ extern "js" {
     fn dom_event_kind(event: Int) -> String
     fn dom_event_target(event: Int) -> Int
     fn dom_event_key(event: Int) -> String
+    fn dom_event_modified(event: Int) -> Bool
     fn dom_prevent_default(event: Int)
     fn dom_stop_propagation(event: Int)
     fn dom_set_timeout(ms: Int, entry: Int) -> Int
@@ -403,6 +404,11 @@ impl Event {
     /// event.
     pub fn key(self) -> String { dom_event_key(rt_handle_id(self.box)) }
 
+    /// Whether Ctrl, Meta, Shift or Alt was down, or a mouse button other
+    /// than the main one was pressed: a click a browser opens in a new tab
+    /// or window, for one. `false` for an event with neither.
+    pub fn modified(self) -> Bool { dom_event_modified(rt_handle_id(self.box)) }
+
     /// Stops the browser's own action for the event, such as following a
     /// link.
     pub fn prevent_default(self) { dom_prevent_default(rt_handle_id(self.box)) }
diff --git a/std/lib.rs b/std/lib.rs
index 76527efb72e0dd9b0245ff1b44148fef059e72ea..0e0913ffbf25cc66599f649bd70662b520b5e91e 100644
--- a/std/lib.rs
+++ b/std/lib.rs
@@ -2,7 +2,7 @@
 //! build time so the compiler stays one self-contained executable.
 //!
 //! Every `include_str!` path stays inside this package, so `cargo package`
-//! carries all 23 files, and its verification step builds the packaged copy
+//! carries all 24 files, and its verification step builds the packaged copy
 //! on its own (spec
 //! `docs/superpowers/specs/2026-10-07-phase-3-0-foundations-design.md` §3).
 //! `nova-resolver` gives each module its `$std.*` name and its place.
@@ -51,6 +51,8 @@ pub const TIME: &str = include_str!("time/lib.nova");
 pub const UI: &str = include_str!("ui/lib.nova");
 /// `std/ui/html/lib.nova`: the element builders, which a program imports.
 pub const UI_HTML: &str = include_str!("ui/html/lib.nova");
+/// `std/ui/router/lib.nova`: routing by the URL, which a program imports.
+pub const UI_ROUTER: &str = include_str!("ui/router/lib.nova");
 /// `std/ui/backend/native.nova`, `std/ui`'s backend natively. Not in
 /// [`ALL`]: only std's modules see it (spec
 /// `docs/superpowers/specs/2026-10-17-phase-4-3-std-ui-reactivity-and-views-design.md`
@@ -68,7 +70,7 @@ pub const RT: &str = include_str!("rt/lib.nova");
 
 /// Every documented module, by its directory under `std/`: all but
 /// [`RT`], [`DOM`] and `std/ui`'s backend.
-pub const ALL: [(&str, &str); 19] = [
+pub const ALL: [(&str, &str); 20] = [
     ("bytes", BYTES),
     ("collections", COLLECTIONS),
     ("core", CORE),
@@ -88,6 +90,7 @@ pub const ALL: [(&str, &str); 19] = [
     ("time", TIME),
     ("ui", UI),
     ("ui/html", UI_HTML),
+    ("ui/router", UI_ROUTER),
 ];
 
 #[cfg(test)]
@@ -137,7 +140,7 @@ mod tests {
             )
             .collect();
         assert_eq!(embedded, on_disk);
-        assert_eq!(on_disk.len(), 23);
+        assert_eq!(on_disk.len(), 24);
     }
 
     /// Each entry carries its own file, not a neighbour's.
diff --git a/std/ui/backend/native.nova b/std/ui/backend/native.nova
index 47c93a2418c17e6538d287c12030f450a7ec3bae..afe5f098c6a8841d8453c89450c00ccb128e544b 100644
--- a/std/ui/backend/native.nova
+++ b/std/ui/backend/native.nova
@@ -46,3 +46,24 @@ pub fn backend_is_same(a: Node, b: Node) -> Bool { no_page() }
 /// Calls `f` with the target's `value` on each event of kind `kind`, until
 /// the function returned is called.
 pub fn backend_listen(node: Node, kind: String, f: fn(String)) -> fn() { no_page() }
+
+/// The router's location: natively always `/` (spec
+/// docs/superpowers/specs/2026-10-18-phase-4-5-router-and-nova-dev-design.md
+/// §3.6, §3.7).
+pub fn backend_location(history: Bool) -> String { "/" }
+
+/// Goes to `to`: natively there is no history, so nothing, and the
+/// location is then `to` as written.
+pub fn backend_go(history: Bool, to: String) -> String { to }
+
+/// Replaces the current history entry with `to`: natively there is none,
+/// so nothing, and the location is then `to` as written.
+pub fn backend_replace(to: String) -> String { to }
+
+/// Calls `f` each time the location changes: natively it never does, so
+/// this listens for nothing, and the function returned does nothing.
+pub fn backend_on_location(history: Bool, f: fn()) -> fn() { || {} }
+
+/// For a plain click on a link, `go` with its `href` when `take()` is true:
+/// natively there are no clicks, so this listens for nothing.
+pub fn backend_on_link_click(take: fn() -> Bool, go: fn(String)) -> fn() { || {} }
diff --git a/std/ui/backend/wasm.nova b/std/ui/backend/wasm.nova
index 68aedd123ae0f749147e50d8f5fd8795ca216eb1..1b2afff796e32959dd67ba1edfa453fdbc8ed55b 100644
--- a/std/ui/backend/wasm.nova
+++ b/std/ui/backend/wasm.nova
@@ -68,3 +68,86 @@ pub fn backend_listen(node: Node, kind: String, f: fn(String)) -> fn() {
     let stop = || listener.remove()
     stop
 }
+
+/// The router's location (spec
+/// docs/superpowers/specs/2026-10-18-phase-4-5-router-and-nova-dev-design.md
+/// §3.1, §3.6): the path and query of the page's address under `history`,
+/// and otherwise its fragment without the `#`, `/` when it has none.
+pub fn backend_location(history: Bool) -> String {
+    if history { return "${location_pathname()}${location_search()}" }
+    let hash = location_hash()
+    if hash == "" || hash == "#" { "/" } else { hash.slice(1, hash.len()) }
+}
+
+/// Goes to `to`: a new history entry for it under `history`, and otherwise
+/// the fragment `#` and `to`. Returns the location the page reports then,
+/// as `backend_location` reads it: the browser writes the address its own
+/// way, as `#/posts/caf%C3%A9` for `#/posts/café`.
+pub fn backend_go(history: Bool, to: String) -> String {
+    if history { history_push(to) } else { set_location_hash("#${to}") }
+    backend_location(history)
+}
+
+/// Replaces the current history entry with `to`, a path and query, adding
+/// none. Returns the location the page reports then, as `backend_go` does.
+pub fn backend_replace(to: String) -> String {
+    history_replace(to)
+    backend_location(true)
+}
+
+/// Calls `f` each time the location changes under the back and forward
+/// buttons, or a fragment typed in: on `popstate` under `history`, and on
+/// `hashchange` otherwise, until the function returned is called.
+pub fn backend_on_location(history: Bool, f: fn()) -> fn() {
+    let kind = if history { "popstate" } else { "hashchange" }
+    let mut listener = window_add_event_listener(kind, |e: Event| f())
+    let stop = || listener.remove()
+    stop
+}
+
+/// For a plain click on a link, until the function returned is called
+/// (spec §3.3): when `take()` is true, prevents the click's default, so the
+/// page is not loaded, then calls `go` with the link's `href`; any other
+/// click is left to the browser. The default is prevented before `go` runs,
+/// so a panic in `go` cannot hand the click back. The link is the nearest
+/// `<a>` with `data-nova-link` at or above the click's target; a plain click
+/// is the main button with no modifier key, on a link with no `target`.
+pub fn backend_on_link_click(take: fn() -> Bool, go: fn(String)) -> fn() {
+    let mut listener = window_add_event_listener(
+        "click",
+        |e: Event| {
+            match nova_link(e.target()) {
+                Some(a) => {
+                    if !e.modified() && a.attribute("target").is_none() && take() {
+                        e.prevent_default()
+                        go(a.attribute("href").unwrap_or(""))
+                    }
+                }
+                None => {}
+            }
+        },
+    )
+    let stop = || listener.remove()
+    stop
+}
+
+// The nearest `<a>` with `data-nova-link` at or above `at`.
+fn nova_link(at: Option<Element>) -> Option<Element> {
+    let mut node = at
+    let mut found = None
+    let mut done = false
+    while !done {
+        match node {
+            Some(n) => {
+                if n.tag() == "a" && n.attribute("data-nova-link").is_some() {
+                    found = Some(n)
+                    done = true
+                } else {
+                    node = n.parent()
+                }
+            }
+            None => done = true
+        }
+    }
+    found
+}
diff --git a/std/ui/router/lib.nova b/std/ui/router/lib.nova
new file mode 100644
index 0000000000000000000000000000000000000000..7daf4de783c530aa9f1d84a8f0e45706cb55ff7c
--- /dev/null
+++ b/std/ui/router/lib.nova
@@ -0,0 +1,586 @@
+//! Routing: the component a page shows for its address, and links that go
+//! from one address to another without loading the page.
+//!
+//! It is not in the prelude: a module imports it, with
+//! `import ui::router::{Route, Router, link}` for some names, or
+//! `import ui::router` for all. `Router` routes by the address's hash, as
+//! `#/posts/1`; `HistoryRouter` routes by its path, as `/posts/1`, through
+//! the history API, and needs a server that answers every path with the
+//! page, as `nova dev` does.
+//!
+//! A router shows the component of the first `Route` whose path matches the
+//! location, and shows it again each time the location changes: a `link`
+//! clicked, `navigate` called, or the back and forward buttons. A route's
+//! path holds `:name` parameters, which `use_params` reads, and the route
+//! `"*"` matches every location, so a last one catches what no other
+//! matches. There is one router per page: the last one made is the one
+//! `navigate`, `use_params` and `use_query` speak to.
+//!
+//! Natively, where there is no address, the location is `/`, so
+//! `render_to_string` writes the root route:
+//!
+//! ```
+//! import ui
+//! import ui::html::{h1, p, text}
+//! import ui::router::{Route, Router, link}
+//!
+//! component Home() {
+//!     view {
+//!         h1 {
+//!             text("Posts")
+//!             link(to: "/posts/1") { text("The first post") }
+//!         }
+//!     }
+//! }
+//!
+//! component NotFound() { view { p { text("Nothing here") } } }
+//!
+//! fn main() {
+//!     let routes = [
+//!         Route { path: "/", component: Home },
+//!         Route { path: "*", component: NotFound },
+//!     ]
+//!     assert_eq(
+//!         render_to_string(|| Router(routes)),
+//!         "<h1>Posts<a href=\"#/posts/1\" data-nova-link=\"\">The first post</a></h1>",
+//!     )
+//! }
+//! ```
+
+// The design (spec
+// docs/superpowers/specs/2026-10-18-phase-4-5-router-and-nova-dev-design.md
+// §3). A router is a memo of the route that matches its location, a signal,
+// and a `for_each` over that memo's one match or none, which builds the
+// match's component again each time the location is set. Its state lives
+// behind three std-only builtins, as `std/ui`'s does, so that `navigate`,
+// `use_params` and `use_query` reach the router mounted last. The page is
+// reached through `std/ui`'s backend: natively the location is `/`, and
+// nothing is listened for.
+
+import ui
+
+/// A route: a path pattern and the component it shows.
+///
+/// The pattern is split at `/` into pieces, as a location is. A piece
+/// `:name` matches any one piece of the location, and makes it the
+/// parameter `name`; any other piece matches the same text, case included.
+/// The pattern `"*"` matches every location.
+pub record Route {
+    /// The pattern, such as `"/"`, `"/posts/:id"` or `"*"`.
+    pub path: String,
+    /// The component shown when the pattern matches.
+    pub component: fn() -> View,
+}
+
+// The router mounted last: its number, its mode, and its location, which
+// never holds a `#fragment`.
+record Active { id: Int, history: Bool, location: Signal<String> }
+
+// The router's state for the thread or the page (spec §3.5): the router
+// mounted last, the parameters of the route being built while it is, and
+// the next router's number.
+record Routing { active: Option<Active>, params: Option<Map<String, String>>, next_id: Int }
+
+// A route that matched: its component and the parameters its pattern took.
+record Matched { component: fn() -> View, params: Map<String, String> }
+
+fn load_routing<T>() -> T { router_state_load() }
+
+fn store_routing<T>(state: T) { router_state_store(state) }
+
+// The state, made the first time it is asked for.
+fn routing() -> Routing {
+    if !router_state_present() { store_routing(Routing { active: None, params: None, next_id: 1 }) }
+    load_routing()
+}
+
+// Whether the router numbered `id` is the one mounted last.
+fn is_active(id: Int) -> Bool {
+    match routing().active {
+        Some(a) => a.id == id
+        None => false
+    }
+}
+
+/// Shows the component of the first route that matches the address's hash,
+/// as `#/posts/1`, and shows it again each time the hash changes. With no
+/// hash, the location is `/`; when no route matches, it shows nothing.
+///
+/// The component is built again on every change of location, even to the
+/// same route, so its own state starts again. A plain click on a `link`
+/// goes to its route without loading the page. The router stops listening
+/// when its owner is disposed.
+///
+/// A router whose routes match nothing shows nothing:
+///
+/// ```
+/// import ui
+/// import ui::html::{p, text}
+/// import ui::router::{Route, Router}
+///
+/// component About() { view { p { text("About") } } }
+///
+/// fn main() {
+///     let html = render_to_string(|| Router([Route { path: "/about", component: About }]))
+///     assert_eq(html, "")
+/// }
+/// ```
+pub component Router(routes: [Route]) { routed(false, routes) }
+
+/// Shows the component of the first route that matches the address's path
+/// and query, as `/posts/1?tab=2`, through the history API, and shows it
+/// again each time the location changes, as `Router` does.
+///
+/// A link's `href` is the path itself, so a server must answer every path
+/// with the page, as `nova dev` does, for the page to be reloaded. The
+/// page `nova build --target wasm` generates loads `/app.js` by its
+/// absolute path, so it can be; a project's own `index.html` must load
+/// `/app.js` by the absolute path too, since under `/posts/1` a relative
+/// `./app.js` would be `/posts/app.js`.
+///
+/// ```
+/// import ui
+/// import ui::html::{p, text}
+/// import ui::router::{HistoryRouter, Route, link}
+///
+/// component Home() { view { p { link(to: "/posts/1") { text("The first post") } } } }
+///
+/// fn main() {
+///     let html = render_to_string(|| HistoryRouter([Route { path: "/", component: Home }]))
+///     assert_eq(html, "<p><a href=\"/posts/1\" data-nova-link=\"\">The first post</a></p>")
+/// }
+/// ```
+pub component HistoryRouter(routes: [Route]) { routed(true, routes) }
+
+// A router of either mode: it takes over the state, listens for the
+// location and for links' clicks until its owner is disposed, and shows the
+// route that matches.
+fn routed(history: Bool, routes: [Route]) -> View {
+    let mut s = routing()
+    let id = s.next_id
+    s.next_id = id + 1
+    let location = signal(located(history))
+    s.active = Some(Active { id: id, history: history, location: location })
+    // `navigate` sets the location itself, to what the page reports, before
+    // the `hashchange` it causes, which then finds it unchanged and builds
+    // nothing again.
+    let stop_location = backend_on_location(
+        history,
+        || handled(
+            || {
+                let at = located(history)
+                if at != location.val { location.set(at) }
+            },
+        ),
+    )
+    // Only the router mounted last takes a link's click; for any other, the
+    // click is the browser's. A click taken stays taken: its default is
+    // prevented before the router goes, so a panic on the way cannot hand it
+    // back to the browser.
+    let stop_clicks = backend_on_link_click(
+        || is_active(id),
+        |href: String| handled(
+            || {
+                if !history && href.starts_with("#") {
+                    navigate(href.slice(1, href.len()))
+                } else {
+                    navigate(href)
+                }
+            },
+        ),
+    )
+    on_cleanup(
+        || {
+            stop_location()
+            stop_clicks()
+            if is_active(id) {
+                let mut s = routing()
+                s.active = None
+                s.params = None
+            }
+        },
+    )
+    let shown = memo(|| matching(routes, location.value()))
+    for_each(shown, |m: Matched| rendered(m))
+}
+
+// Run a listener's `f` as `std/ui` runs a handler: in a batch, and with no
+// listener, so that nothing it reads subscribes the effect running when the
+// event came. `root` is how `std/ui`'s public API runs a function with no
+// listener; its owner, which `f` gives nothing to own, is disposed at once.
+fn handled(f: fn()) {
+    batch(
+        || root(
+            |dispose: fn()| {
+                f()
+                dispose()
+            },
+        ),
+    )
+}
+
+// The component of `m`, built with `m`'s parameters for `use_params`.
+fn rendered(m: Matched) -> View {
+    let mut s = routing()
+    let was = s.params
+    s.params = Some(m.params)
+    let component = m.component
+    let v = component()
+    s.params = was
+    v
+}
+
+/// Goes to `to`, a path within the app such as `/posts/1?tab=2`; a `to`
+/// that does not begin with `/` gets one in front, and a leading run of `/`
+/// and `\` becomes one `/`. Under `Router` it sets the hash to `#` and `to`,
+/// under `HistoryRouter` it adds a history entry, or replaces the current
+/// one when `to` is the location already, and then the router shows the
+/// route at once. The router's location is then the one the page reports,
+/// as the browser writes it (`/posts/café` as `/posts/caf%C3%A9`), less any
+/// `#fragment`. Natively there is no history, and only the router's location
+/// changes.
+///
+/// It panics when no router is mounted.
+///
+/// ```
+/// import ui
+/// import ui::html::{p, text}
+/// import ui::router::{Route, Router, navigate}
+///
+/// component Home() { view { p { text("Home") } } }
+///
+/// component About() { view { p { text("About") } } }
+///
+/// fn main() {
+///     let routes = [
+///         Route { path: "/", component: Home },
+///         Route { path: "/about", component: About },
+///     ]
+///     let html = render_to_string(
+///         || {
+///             let router = Router(routes)
+///             navigate("about")
+///             router
+///         },
+///     )
+///     assert_eq(html, "<p>About</p>")
+/// }
+/// ```
+pub fn navigate(to: String) {
+    let a = match routing().active {
+        Some(a) => a
+        None => panic("navigate: no router is mounted")
+    }
+    let path = rooted(to)
+    let at = if a.history && same_location(path, a.location.val) {
+        backend_replace(path)
+    } else {
+        backend_go(a.history, path)
+    }
+    a.location.set(unfragmented(at))
+}
+
+// The page's location, less any `#fragment`.
+fn located(history: Bool) -> String { unfragmented(backend_location(history)) }
+
+// `location` before its first `#`: a router's location never holds a
+// fragment. One reaches it in hash mode as a second `#`, through
+// `navigate`, a `link` or a typed hash, and natively through `navigate`;
+// in history mode the page reports the path and query alone.
+fn unfragmented(location: String) -> String {
+    match location.index_of("#") {
+        Some(at) => location.slice(0, at)
+        None => location
+    }
+}
+
+// Whether going to `path` stays at `at`, a router's location: the same once
+// `path`'s fragment is dropped and both are percent-decoded, since the page
+// reports `/posts/café` as `/posts/caf%C3%A9`.
+fn same_location(path: String, at: String) -> Bool { decoded(unfragmented(path)) == decoded(at) }
+
+/// The parameters of the route being shown, by name: `{"id": "1"}` for the
+/// route `/posts/:id` at `/posts/1`, each percent-decoded. It reads them
+/// while the router builds that route's component, and is empty at any
+/// other time, or with no router. It is not reactive, and need not be: each
+/// change of location builds the component again.
+///
+/// ```
+/// import ui
+/// import ui::html::{p, text}
+/// import ui::router::{Route, Router, navigate, use_params}
+///
+/// component Post() {
+///     let id = match use_params().get("id") {
+///         Some(id) => id
+///         None => "?"
+///     }
+///     view { p { text("Post ${id}") } }
+/// }
+///
+/// fn main() {
+///     let html = render_to_string(
+///         || {
+///             let router = Router([Route { path: "/posts/:id", component: Post }])
+///             navigate("/posts/a%20b")
+///             router
+///         },
+///     )
+///     assert_eq(html, "<p>Post a b</p>")
+///     assert_eq(use_params().len(), 0)
+/// }
+/// ```
+pub fn use_params() -> Map<String, String> {
+    match routing().params {
+        Some(params) => params
+        None => Map::new()
+    }
+}
+
+/// The query of the current location, by key: `{"tab": "2"}` for
+/// `?tab=2`. A part with no `=` has the value `""`, `+` is a space, both
+/// halves are percent-decoded, and of a key given twice the last value
+/// wins. It is empty with no router, and is not reactive, as `use_params`
+/// is not.
+///
+/// ```
+/// import ui
+/// import ui::html::{p, text}
+/// import ui::router::{Route, Router, navigate, use_query}
+///
+/// component Search() {
+///     let query = use_query()
+///     let words = query.get("q").unwrap_or("")
+///     let page = query.get("page").unwrap_or("1")
+///     view { p { text("${words} on page ${page}") } }
+/// }
+///
+/// fn main() {
+///     let html = render_to_string(
+///         || {
+///             let router = Router([Route { path: "/search", component: Search }])
+///             navigate("/search?q=nova+lang&page=1&page=2")
+///             router
+///         },
+///     )
+///     assert_eq(html, "<p>nova lang on page 2</p>")
+/// }
+/// ```
+pub fn use_query() -> Map<String, String> {
+    match routing().active {
+        Some(a) => query_of(a.location.val)
+        None => Map::new()
+    }
+}
+
+/// An `<a>` that goes to a route without loading the page. Its `to`
+/// attribute, fixed or reactive, is the path, and becomes its `href`:
+/// `#/posts/1` under `Router`, `/posts/1` under `HistoryRouter`, with one
+/// `/` in front as `navigate` puts it, so `//posts/1` is `/posts/1` too,
+/// and `to` as written with no router. It also gets `data-nova-link`, by
+/// which the router finds it; its other attributes are the `<a>`'s.
+///
+/// A plain click navigates; a click with a modifier key or another button,
+/// or on a link with a `target`, is the browser's, so a link still opens in
+/// a new tab. In a view it is `link(to: "/posts/1") { text("…") }`, and
+/// outside one `link([attr("to", "/posts/1")], [text("…")])`. A link with no
+/// `to` panics.
+///
+/// ```
+/// import ui
+/// import ui::html::{attr, class, text}
+/// import ui::router::{link}
+///
+/// let html = render_to_string(|| link([attr("to", "/about"), class("nav")], [text("About")]))
+/// assert_eq(html, "<a href=\"/about\" class=\"nav\" data-nova-link=\"\">About</a>")
+/// ```
+pub fn link(attrs: [Attr], children: [View]) -> View {
+    let mut out = [Static("data-nova-link", ""); attrs.len() + 1]
+    let mut found = false
+    let mut i = 0
+    while i < attrs.len() {
+        let a = attrs[i]
+        out[i] = match a {
+            Static(name, to) => {
+                if name == "to" {
+                    found = true
+                    Dynamic("href", || href_of(to))
+                } else {
+                    a
+                }
+            }
+            Dynamic(name, to) => {
+                if name == "to" {
+                    found = true
+                    Dynamic("href", || href_of(to()))
+                } else {
+                    a
+                }
+            }
+            Property(name, value) => a
+            DynProperty(name, value) => a
+            On(kind, f) => a
+        }
+        i = i + 1
+    }
+    if !found { panic("link: no `to` attribute") }
+    Element("a", out, children)
+}
+
+// A link's `href` for `to`, read when the link is built, so that one made
+// before its router, as a menu before the routes, has the router's.
+fn href_of(to: String) -> String {
+    match routing().active {
+        Some(a) => if a.history { rooted(to) } else { "#${rooted(to)}" }
+        None => to
+    }
+}
+
+// `to` with one `/` in front: a leading run of `/` and `\` becomes one `/`,
+// and a `to` with neither gets one, so that no `href` or history entry is a
+// URL of another host, as `//posts/8` and `/\posts/8` would be. A URL's
+// parser deletes every tab, LF and CR before it reads it, so they go first:
+// `/\t/posts/8` would otherwise be `//posts/8` to a browser.
+fn rooted(to: String) -> String {
+    let tabless = "".join(to.split("\t"))
+    let lineless = "".join(tabless.split("\n"))
+    let plain = "".join(lineless.split("\r"))
+    let cs = plain.chars()
+    let mut i = 0
+    while i < cs.len() && (cs[i] == '/' || cs[i] == '\\') { i = i + 1 }
+    if i == 1 && cs[0] == '/' { return plain }
+    "/${plain.slice(i, cs.len())}"
+}
+
+// The routes that match `location`: the first that does, or none.
+fn matching(routes: [Route], location: String) -> [Matched] {
+    let path = match location.index_of("?") {
+        Some(at) => location.slice(0, at)
+        None => location
+    }
+    let pieces = pieces_of(path)
+    let mut i = 0
+    while i < routes.len() {
+        let r = routes[i]
+        match params_of(pieces_of(r.path), pieces) {
+            Some(params) => return [Matched { component: r.component, params: params }]
+            None => {}
+        }
+        i = i + 1
+    }
+    let none: [Matched] = []
+    none
+}
+
+// The parameters `pattern` takes from `pieces`, or `None` when it does not
+// match them.
+fn params_of(pattern: [String], pieces: [String]) -> Option<Map<String, String>> {
+    let mut params: Map<String, String> = Map::new()
+    if pattern.len() == 1 && pattern[0] == "*" { return Some(params) }
+    if pattern.len() != pieces.len() { return None }
+    let mut i = 0
+    while i < pattern.len() {
+        let p = pattern[i]
+        if p.starts_with(":") {
+            let _ = params.insert(p.slice(1, p.len()), pieces[i])
+        } else if p != pieces[i] {
+            return None
+        }
+        i = i + 1
+    }
+    Some(params)
+}
+
+// The pieces of `path`: split at `/`, with the empty ones dropped, each
+// percent-decoded, or kept as written when it does not decode.
+fn pieces_of(path: String) -> [String] {
+    let parts = path.split("/")
+    let mut n = 0
+    let mut i = 0
+    while i < parts.len() {
+        if parts[i] != "" { n = n + 1 }
+        i = i + 1
+    }
+    let mut out = [""; n]
+    let mut k = 0
+    let mut j = 0
+    while j < parts.len() {
+        if parts[j] != "" {
+            out[k] = decoded(parts[j])
+            k = k + 1
+        }
+        j = j + 1
+    }
+    out
+}
+
+// The query of `location`, after its first `?`: `&`-separated parts, each
+// `key=value` or a `key` whose value is `""`, both halves decoded with `+`
+// as a space. An empty part is skipped, and a key's last value wins.
+fn query_of(location: String) -> Map<String, String> {
+    let mut out: Map<String, String> = Map::new()
+    let at = match location.index_of("?") {
+        Some(at) => at
+        None => return out
+    }
+    let parts = location.slice(at + 1, location.len()).split("&")
+    let mut i = 0
+    while i < parts.len() {
+        let part = parts[i]
+        if part != "" {
+            let mut key = part
+            let mut value = ""
+            match part.index_of("=") {
+                Some(eq) => {
+                    key = part.slice(0, eq)
+                    value = part.slice(eq + 1, part.len())
+                }
+                None => {}
+            }
+            let _ = out.insert(query_decoded(key), query_decoded(value))
+        }
+        i = i + 1
+    }
+    out
+}
+
+// A half of a query's part: `+` is a space, then percent-decoding.
+fn query_decoded(text: String) -> String { decoded(" ".join(text.split("+"))) }
+
+// `text` percent-decoded as UTF-8, or as written when a `%` is not followed
+// by two hex digits or the bytes are not UTF-8.
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


### Task 3: `nova dev`

Spec §4, §5.3; decisions 15 to 21.

**Files:**
- Create:
  - `crates/nova-cli/src/cmd/dev.rs`
  - `crates/nova-cli/src/dev/mod.rs`, `server.rs`, `reload.js`
- Modify:
  - `crates/nova-cli/src/main.rs`, `cmd/mod.rs`
  - `crates/nova-cli/src/bundle.rs` (`check_output_for`; `resolved`
    public), `cmd/run.rs` (`wasm_output`)
- Test: `crates/nova-cli/tests/dev_tests.rs`

- [ ] **Step 1: The tests.** `patch_block t3-tests`: `dev_tests.rs`, which
  starts `nova dev --port 0` in a fresh project under the temporary
  directory, with `NOVA_HOME` beside it, `TERM=xterm-256color` and no
  `NO_COLOR`; reads the URL it prints; speaks HTTP over `TcpStream`; and
  kills the child when the test ends, pass or fail. Each wait polls for 30
  seconds at most and fails with what it read. The project's own page
  loads `/app.js`.
  - `it_serves_the_page_and_files_and_refuses_what_it_must`: `/` is the
    project's page with the tag and build 1's id before `</body>`, its
    `Content-Length` counting it, and `dist/index.html` unchanged;
    `/app.wasm` is `application/wasm` with `dist/`'s bytes; the script;
    `HEAD` has the length and no body; `HEAD /__nova/events` is the
    stream's head with `Connection: close` and no body; a head whose lines
    end in a bare LF is read; `POST` is `405`; every response says
    `no-store`; `/../nova.toml`, `/%2e%2e/nova.toml` and
    `/src/..%2f..%2fnova.toml` are `403`; a foreign `Host`, on port 80 and
    on the server's port, is `403`, and `localhost:<port>` is served;
    `/posts/2` is the page and `/missing.js` `404`; and a head over 16 KiB
    is `400`.
  - `a_link_out_of_dist_is_refused`, on Unix: links to the manifest and to
    the project are `403`.
  - `the_event_stream_reloads_and_reports_errors`: `hello` with the id the
    page was served with; `reload` with the next for an edit; a stream that
    connects after that build hears a `hello` that is not the page's id;
    `error` with the checker's message, `main.nova:` and no escape
    character, for a type error, which a page served after it hears after a
    `hello` with that failed build's id; `reload` for the fix on all three
    streams; and three `rebuilt in` lines on stderr.
  - `a_first_build_that_fails_serves_the_servers_own_page`: with no
    `dist/`, `/` is the server's own page with the tag; a page hears the
    errors at once; the fix reloads it into the project's page.
  - `a_second_server_on_the_same_port_fails`: status 1, and stderr is
    exactly `nova dev: port <port> is in use; choose another with --port`.
  - `it_refuses_what_build_refuses`: `-o .`, "the output directory holds
    the project", and a library, "`dev-app` is a library".
- [ ] **Step 2: Run, and see them fail.** `cargo test -p nova-cli --test
  dev_tests`. Expected: it builds; `0 passed; 6 failed`. Five fail at
  `dev_tests.rs:173` with `no URL on stdout (channel is empty and sending
  half is closed); nova dev's stderr: "error: unrecognized subcommand
  'dev'\n\nUsage: nova <COMMAND>\n\nFor more information, try
  '--help'.\n"`, and `it_refuses_what_build_refuses` at
  `dev_tests.rs:613` with `error: unrecognized subcommand 'dev'`. (On
  Windows `a_link_out_of_dist_is_refused` is not compiled: 5.)
- [ ] **Step 3: The code.** `patch_block t3-code`:
  - **`cmd/dev.rs`:** `DevCmd` (`FILE`, `--port`, 3000 by default,
    `--open`, `-o`); `run`: the refusals of decision 21, the bind of
    `127.0.0.1` and the port's message with status 1, the watcher's first
    read, the first build, the URL line, `--open`, then the watcher's
    thread and the server. `Builder`: one build at a time, the child `nova
    build --target wasm [FILE] -o <out>` with `NO_COLOR=1` and
    `RUST_LIB_BACKTRACE=0`; the notes, then `rebuilt in <ms> ms`, or the
    errors (decision 19). `Watched`, with `deep` true for a project alone,
    `snapshot`, `changed`, `settled`, `sources`, `everything`, `stat` and
    `is_out` (decision 20); `opener` and `open`. Its 6 tests, one
    Unix-only: `a_project_watches_its_sources_manifest_page_and_public`,
    `a_single_file_watches_the_nova_files_of_its_directory`,
    `a_change_an_addition_and_a_removal_are_each_seen`,
    `a_link_to_nothing_is_watched_as_a_file`,
    `a_build_waits_for_a_poll_that_sees_nothing_new` and
    `the_opener_is_the_systems`.
  - **`dev/server.rs`:** `serve`, a thread per connection and the ping;
    `Server::handle` and `answer`, `Host` first, then the method
    (decision 15); `read_head` (16 KiB), `head_end` (CRLF or a bare LF) and
    `parse_head` (`HTTP/1.` and a digit); `host_allowed`; `locate`,
    `fallback`, `plain` and `percent_decode` (decisions 17 and 18);
    `content_type`, `.mjs` with `.js`; `inject` and `script_tag`;
    `write_answer`, with `no-store`, no body for `HEAD`, and the event
    stream's `HEAD` with `Connection: close`; `close`; `event`; and
    `Events`, with the build ids, `built`, `join` and `ping` (decision 16).
    Its 20 tests, one Unix-only and one with a loop that runs on Windows
    alone: the 17 that Task 6's mutants 57 to 84 name, and
    `a_path_that_does_not_parse_is_400`, `a_head_over_16_kib_is_invalid`
    and `a_page_whose_write_fails_later_is_dropped`.
  - **`dev/reload.js`:** the tag's id read once; an `EventSource` on
    `/__nova/events`; reload on `reload` and on a `hello` that is not the
    tag's id; on `error` with data, `#nova-dev-overlay`, titled `nova dev:
    the build failed`, the text in a `<pre>` as `textContent`.
  - **`bundle.rs`, `cmd/run.rs`:** `check_output_for(root, out)`, which
    `Bundle::check_output` now calls, and `resolved`, public; and
    `wasm_output`, which `build_wasm` now calls.
  - **`main.rs`, `cmd/mod.rs`:** `Command::Dev`, and the modules.
- [ ] **Step 4: Run.**
  - `cargo test -p nova-cli --test dev_tests`. Expected: `6 passed`, in
    about 2 seconds.
  - `cargo test -p nova-cli --bin nova dev::`. Expected: `26 passed; 0
    failed; 0 ignored; 0 measured; 4 filtered out`.
  - `cargo test -p nova-cli --bin nova`. Expected: `30 passed` (4 before).
  - The bundle's refactor: `W dist`, `W manifest`, `W a_single_file`, `W
    release`, `W counter_spa`. Expected: `4 passed`, `1 passed`, `1
    passed`, `6 passed` and `2 passed`.
- [ ] **Step 5: The checks,** as the conventions list them.
- [ ] **Step 6: Commit** `cli: nova dev: build, serve, watch, and reload the page or show the overlay`.

**Patch `t3-tests`:**

<!-- patch t3-tests -->

````diff
diff --git a/crates/nova-cli/tests/dev_tests.rs b/crates/nova-cli/tests/dev_tests.rs
new file mode 100644
index 0000000000000000000000000000000000000000..21a7d61deaadb1e82dacd99b0256dfecf8dbac90
--- /dev/null
+++ b/crates/nova-cli/tests/dev_tests.rs
@@ -0,0 +1,630 @@
+//! `nova dev`, end to end (the 4.5 spec
+//! `docs/superpowers/specs/2026-10-18-phase-4-5-router-and-nova-dev-design.md`
+//! §5.3): each test starts `nova dev --port 0` in a fresh copy of a small
+//! project, with a fresh `NOVA_HOME` beside it, reads the URL it prints, and
+//! speaks HTTP to it over `TcpStream`. The child is killed when the test
+//! ends, pass or fail. Every wait polls, for 30 seconds at most, and fails
+//! with what it read.
+
+use std::io::{BufRead, BufReader, Read, Write};
+use std::net::TcpStream;
+use std::path::{Path, PathBuf};
+use std::process::{Child, Command, ExitStatus, Stdio};
+use std::sync::{mpsc, Arc, Mutex};
+use std::time::{Duration, Instant};
+
+/// How long any wait may take.
+const LIMIT: Duration = Duration::from_secs(30);
+
+/// The tag the server adds to every page, with the id of the build current
+/// when it served the page.
+fn tag(id: &str) -> String {
+    format!("<script src=\"/__nova/reload.js\" data-build=\"{id}\"></script>")
+}
+
+/// The build id in `page`'s tag.
+fn tag_id(page: &str) -> String {
+    let at = "<script src=\"/__nova/reload.js\" data-build=\"";
+    let Some(start) = page.find(at).map(|i| i + at.len()) else {
+        panic!("no reload script with a build id in {page:?}");
+    };
+    let end = start + page[start..].find('"').expect("the id ends");
+    page[start..end].to_string()
+}
+
+/// A build id's two halves, `<start>.<number>`: the server's start in
+/// milliseconds since the epoch, and the build's number.
+fn halves(id: &str) -> (u128, u64) {
+    let parsed = id
+        .split_once('.')
+        .and_then(|(start, n)| Some((start.parse().ok()?, n.parse().ok()?)));
+    parsed.unwrap_or_else(|| panic!("not a build id: {id:?}"))
+}
+
+const MANIFEST: &str = "[package]\nname = \"dev-app\"\nversion = \"0.1.0\"\nedition = \"2026\"\n";
+
+const PAGE: &str = "<!doctype html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n\
+<title>Dev</title>\n</head>\n<body>\n<div id=\"app\"></div>\n<script type=\"module\">\n\
+import init from \"/app.js\";\ninit();\n</script>\n</body>\n</html>\n";
+
+const MAIN: &str = "import ui\nimport ui::html::{button, div}\n\n\
+component Counter() {\n    let count = signal(0)\n\n    view {\n        div {\n            \
+text(\"Count: ${count.value()}\")\n            \
+button(on_click: || count.update(|n| n + 1)) { text(\"+\") }\n        }\n    }\n}\n\n\
+fn main() { mount(\"#app\", Counter) }\n";
+
+/// The checker's message for [`broken`]'s error.
+const CHECKER_SAYS: &str = "text must have type `String`, found `Int`";
+
+/// `MAIN` with a type error: `text` given an `Int`.
+fn broken() -> String {
+    MAIN.replace("text(\"Count: ${count.value()}\")", "text(1)")
+}
+
+fn nova() -> Command {
+    Command::new(env!("CARGO_BIN_EXE_nova"))
+}
+
+/// A fresh project with a fixed name, so each run replaces the last, and
+/// the `NOVA_HOME` beside it, emptied.
+fn project(name: &str, main: &str) -> PathBuf {
+    let dir = std::env::temp_dir().join(format!("nova-dev-test-{name}"));
+    let _ = std::fs::remove_dir_all(&dir);
+    let _ = std::fs::remove_dir_all(home(&dir));
+    std::fs::create_dir_all(dir.join("src")).expect("create the test directory");
+    std::fs::write(dir.join("nova.toml"), MANIFEST).expect("write nova.toml");
+    std::fs::write(dir.join("index.html"), PAGE).expect("write index.html");
+    std::fs::write(dir.join("src").join("main.nova"), main).expect("write src/main.nova");
+    dir
+}
+
+fn home(dir: &Path) -> PathBuf {
+    dir.with_file_name(format!(
+        "{}-home",
+        dir.file_name().expect("a name").to_string_lossy()
+    ))
+}
+
+/// `nova dev ARGS` in `dir`, with `NOVA_HOME` beside it and no backtrace;
+/// its stdout's lines arrive on a channel, and its stderr is gathered, each
+/// on a thread of its own, so neither pipe fills. It runs with a terminal
+/// that asks for colour and without `NO_COLOR`, so only `nova dev`'s own
+/// `NO_COLOR` keeps colour out of a build's errors.
+fn spawn(dir: &Path, args: &[&str]) -> (Child, mpsc::Receiver<String>, Arc<Mutex<String>>) {
+    let mut child = nova()
+        .arg("dev")
+        .args(args)
+        .current_dir(dir)
+        .env("NOVA_HOME", home(dir))
+        .env_remove("NO_COLOR")
+        .env("TERM", "xterm-256color")
+        .env("RUST_BACKTRACE", "0")
+        .stdin(Stdio::null())
+        .stdout(Stdio::piped())
+        .stderr(Stdio::piped())
+        .spawn()
+        .expect("spawn nova dev");
+    let stdout = child.stdout.take().expect("stdout is piped");
+    let mut stderr = child.stderr.take().expect("stderr is piped");
+    let (send, lines) = mpsc::channel();
+    std::thread::spawn(move || {
+        for line in BufReader::new(stdout).lines() {
+            let Ok(line) = line else { break };
+            if send.send(line).is_err() {
+                break;
+            }
+        }
+    });
+    let gathered = Arc::new(Mutex::new(String::new()));
+    let into = Arc::clone(&gathered);
+    std::thread::spawn(move || {
+        let mut chunk = [0u8; 4096];
+        while let Ok(n) = stderr.read(&mut chunk) {
+            if n == 0 {
+                break;
+            }
+            into.lock()
+                .unwrap()
+                .push_str(&String::from_utf8_lossy(&chunk[..n]));
+        }
+    });
+    (child, lines, gathered)
+}
+
+/// A running `nova dev`, killed when dropped.
+struct Dev {
+    child: Child,
+    port: u16,
+    stderr: Arc<Mutex<String>>,
+}
+
+impl Dev {
+    /// `nova dev --port 0` in `dir`, once it has printed its URL.
+    fn start(dir: &Path) -> Dev {
+        let (child, lines, stderr) = spawn(dir, &["--port", "0"]);
+        let mut dev = Dev {
+            child,
+            port: 0,
+            stderr,
+        };
+        let line = match lines.recv_timeout(LIMIT) {
+            Ok(line) => line,
+            Err(e) => dev.fail(&format!("no URL on stdout ({e})")),
+        };
+        let port = line
+            .strip_prefix("nova dev: serving http://127.0.0.1:")
+            .and_then(|rest| rest.strip_suffix('/'))
+            .and_then(|port| port.parse::<u16>().ok());
+        match port {
+            Some(port) if port != 0 => dev.port = port,
+            _ => dev.fail(&format!("not the URL line: {line:?}")),
+        }
+        dev
+    }
+
+    fn stderr(&self) -> String {
+        self.stderr.lock().unwrap().replace("\r\n", "\n")
+    }
+
+    fn fail(&mut self, why: &str) -> ! {
+        let _ = self.child.kill();
+        let _ = self.child.wait();
+        std::thread::sleep(Duration::from_millis(200));
+        panic!("{why}; nova dev's stderr: {:?}", self.stderr());
+    }
+
+    /// GET `path` from the server, with its own `Host`.
+    fn get(&self, path: &str) -> Response {
+        self.send(&format!(
+            "GET {path} HTTP/1.1\r\nHost: 127.0.0.1:{}\r\n\r\n",
+            self.port
+        ))
+    }
+
+    fn send(&self, request: &str) -> Response {
+        exchange(self.port, request.as_bytes())
+    }
+}
+
+impl Drop for Dev {
+    fn drop(&mut self) {
+        let _ = self.child.kill();
+        let _ = self.child.wait();
+    }
+}
+
+/// A response: its status, its head and its body.
+struct Response {
+    status: u16,
+    head: String,
+    body: Vec<u8>,
+}
+
+impl Response {
+    /// A header's value, by its name in any case.
+    fn header(&self, name: &str) -> Option<&str> {
+        self.head.split("\r\n").skip(1).find_map(|line| {
+            let (n, v) = line.split_once(':')?;
+            n.eq_ignore_ascii_case(name).then(|| v.trim())
+        })
+    }
+
+    fn text(&self) -> String {
+        String::from_utf8_lossy(&self.body).into_owned()
+    }
+}
+
+/// `request` to the server and its whole response, read until the server
+/// closes the connection, as it does after every response but the event
+/// stream's.
+fn exchange(port: u16, request: &[u8]) -> Response {
+    let mut sock = TcpStream::connect(("127.0.0.1", port)).expect("connect to nova dev");
+    sock.set_read_timeout(Some(LIMIT)).expect("a read timeout");
+    sock.set_write_timeout(Some(LIMIT))
+        .expect("a write timeout");
+    sock.write_all(request).expect("send the request");
+    let mut read = Vec::new();
+    if let Err(e) = sock.read_to_end(&mut read) {
+        panic!(
+            "reading the response failed ({e}) after {:?}",
+            String::from_utf8_lossy(&read)
+        );
+    }
+    let Some(end) = read.windows(4).position(|w| w == b"\r\n\r\n") else {
+        panic!("no head in {:?}", String::from_utf8_lossy(&read));
+    };
+    let head = String::from_utf8_lossy(&read[..end]).into_owned();
+    let status = head
+        .split(' ')
+        .nth(1)
+        .and_then(|s| s.parse().ok())
+        .unwrap_or_else(|| panic!("no status in {head:?}"));
+    Response {
+        status,
+        head,
+        body: read[end + 4..].to_vec(),
+    }
+}
+
+/// A connection to `/__nova/events`, read event by event.
+struct EventStream {
+    sock: TcpStream,
+    read: Vec<u8>,
+    /// How far `read` has been taken as events, the head included.
+    taken: usize,
+}
+
+impl EventStream {
+    fn open(port: u16) -> EventStream {
+        let mut sock = TcpStream::connect(("127.0.0.1", port)).expect("connect to nova dev");
+        sock.set_read_timeout(Some(Duration::from_millis(250)))
+            .expect("a read timeout");
+        sock.write_all(
+            format!("GET /__nova/events HTTP/1.1\r\nHost: localhost:{port}\r\n\r\n").as_bytes(),
+        )
+        .expect("ask for the event stream");
+        let mut stream = EventStream {
+            sock,
+            read: Vec::new(),
+            taken: 0,
+        };
+        let deadline = Instant::now() + LIMIT;
+        loop {
+            if let Some(end) = stream.read.windows(4).position(|w| w == b"\r\n\r\n") {
+                let head = String::from_utf8_lossy(&stream.read[..end]).into_owned();
+                assert!(head.starts_with("HTTP/1.1 200 "), "{head}");
+                assert!(
+                    head.to_ascii_lowercase()
+                        .contains("content-type: text/event-stream"),
+                    "{head}"
+                );
+                stream.taken = end + 4;
+                return stream;
+            }
+            stream.fill(deadline);
+        }
+    }
+
+    /// Read more, or fail with what was read once `deadline` passes or the
+    /// server closes the stream.
+    fn fill(&mut self, deadline: Instant) {
+        let mut chunk = [0u8; 4096];
+        match self.sock.read(&mut chunk) {
+            Ok(0) => panic!(
+                "the event stream closed after {:?}",
+                String::from_utf8_lossy(&self.read)
+            ),
+            Ok(n) => self.read.extend_from_slice(&chunk[..n]),
+            Err(e)
+                if matches!(
+                    e.kind(),
+                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
+                ) => {}
+            Err(e) => panic!(
+                "reading the event stream failed ({e}) after {:?}",
+                String::from_utf8_lossy(&self.read)
+            ),
+        }
+        if Instant::now() >= deadline {
+            panic!(
+                "no event within {LIMIT:?}; the stream read {:?}",
+                String::from_utf8_lossy(&self.read)
+            );
+        }
+    }
+
+    /// The next event, its name and its data, comments skipped.
+    fn next(&mut self) -> (String, String) {
+        let deadline = Instant::now() + LIMIT;
+        loop {
+            let rest = &self.read[self.taken..];
+            if let Some(end) = rest.windows(2).position(|w| w == b"\n\n") {
+                let block = String::from_utf8_lossy(&rest[..end]).into_owned();
+                self.taken += end + 2;
+                let mut name = String::new();
+                let mut data: Vec<&str> = Vec::new();
+                for line in block.lines() {
+                    if let Some(value) = line.strip_prefix("event: ") {
+                        name = value.to_string();
+                    } else if let Some(value) = line.strip_prefix("data: ") {
+                        data.push(value);
+                    }
+                }
+                if !name.is_empty() {
+                    return (name, data.join("\n"));
+                }
+                continue;
+            }
+            self.fill(deadline);
+        }
+    }
+
+    /// The next event, which must be named `name`: its data.
+    fn expect(&mut self, name: &str) -> String {
+        let (got, data) = self.next();
+        assert_eq!(got, name, "the event's data: {data:?}");
+        data
+    }
+}
+
+/// Waits up to [`LIMIT`] for `child` to exit, killing it and failing if it
+/// does not.
+fn exit_of(child: &mut Child) -> ExitStatus {
+    let deadline = Instant::now() + LIMIT;
+    loop {
+        if let Some(status) = child.try_wait().expect("try_wait") {
+            return status;
+        }
+        if Instant::now() >= deadline {
+            let _ = child.kill();
+            let _ = child.wait();
+            panic!("nova dev did not exit within {LIMIT:?}");
+        }
+        std::thread::sleep(Duration::from_millis(50));
+    }
+}
+
+/// Pages and files (spec §5.3): `/` is the project's page with the script,
+/// `/app.wasm` is `application/wasm` with `dist/`'s bytes, `HEAD` has no
+/// body, `HEAD /__nova/events` says its connection closes, a head whose
+/// lines end in a bare LF is read, `POST` is `405`, and every response says
+/// `no-store`; what is
+/// refused, `403`; and the fallback, `/posts/2` the page and `/missing.js`
+/// `404`.
+#[test]
+fn it_serves_the_page_and_files_and_refuses_what_it_must() {
+    let dir = project("pages", MAIN);
+    let dev = Dev::start(&dir);
+    let port = dev.port;
+
+    let page = dev.get("/");
+    let id = tag_id(&page.text());
+    assert_eq!(halves(&id).1, 1, "{id}");
+    let expected = PAGE.replace("</body>", &format!("{}</body>", tag(&id)));
+    assert_eq!(page.status, 200, "{}", page.head);
+    assert_eq!(
+        page.header("content-type"),
+        Some("text/html; charset=utf-8")
+    );
+    assert_eq!(page.header("cache-control"), Some("no-store"));
+    assert_eq!(page.text(), expected);
+    assert_eq!(
+        page.header("content-length"),
+        Some(expected.len().to_string().as_str())
+    );
+    // The file on disk is not changed.
+    assert_eq!(
+        std::fs::read_to_string(dir.join("dist").join("index.html")).expect("dist's page"),
+        PAGE
+    );
+
+    let wasm = dev.get("/app.wasm");
+    assert_eq!(wasm.status, 200, "{}", wasm.head);
+    assert_eq!(wasm.header("content-type"), Some("application/wasm"));
+    assert_eq!(wasm.header("cache-control"), Some("no-store"));
+    assert_eq!(
+        wasm.body,
+        std::fs::read(dir.join("dist").join("app.wasm")).expect("dist's module")
+    );
+
+    let script = dev.get("/__nova/reload.js");
+    assert_eq!(script.status, 200, "{}", script.head);
+    assert_eq!(
+        script.header("content-type"),
+        Some("text/javascript; charset=utf-8")
+    );
+    assert!(script.text().contains("EventSource"), "{}", script.text());
+
+    let head = dev.send(&format!(
+        "HEAD / HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n\r\n"
+    ));
+    assert_eq!(head.status, 200, "{}", head.head);
+    assert_eq!(
+        head.header("content-length"),
+        Some(expected.len().to_string().as_str())
+    );
+    assert!(head.body.is_empty(), "{:?}", head.text());
+
+    // The event stream's `HEAD`: the stream's head, which says the
+    // connection closes, as it does, since `exchange` reads to the end.
+    let events = dev.send(&format!(
+        "HEAD /__nova/events HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n\r\n"
+    ));
+    assert_eq!(events.status, 200, "{}", events.head);
+    assert_eq!(events.header("content-type"), Some("text/event-stream"));
+    assert_eq!(events.header("connection"), Some("close"));
+    assert!(events.body.is_empty(), "{:?}", events.text());
+
+    // A head whose lines end in a bare LF is read as one ending in CRLF.
+    let bare = dev.send(&format!("GET / HTTP/1.1\nHost: 127.0.0.1:{port}\n\n"));
+    assert_eq!(bare.status, 200, "{}", bare.head);
+    assert_eq!(bare.text(), expected);
+
+    let post = dev.send(&format!(
+        "POST / HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nContent-Length: 2\r\n\r\nhi"
+    ));
+    assert_eq!(post.status, 405, "{}", post.head);
+    assert_eq!(post.header("cache-control"), Some("no-store"));
+
+    for path in [
+        "/../nova.toml",
+        "/%2e%2e/nova.toml",
+        "/src/..%2f..%2fnova.toml",
+    ] {
+        let refused = dev.get(path);
+        assert_eq!(refused.status, 403, "{path}: {}", refused.head);
+    }
+    let foreign = dev.send("GET / HTTP/1.1\r\nHost: evil.example:80\r\n\r\n");
+    assert_eq!(foreign.status, 403, "{}", foreign.head);
+    let rebound = dev.send(&format!(
+        "GET / HTTP/1.1\r\nHost: evil.example:{port}\r\n\r\n"
+    ));
+    assert_eq!(rebound.status, 403, "{}", rebound.head);
+    let by_name = dev.send(&format!("GET / HTTP/1.1\r\nHost: localhost:{port}\r\n\r\n"));
+    assert_eq!(by_name.status, 200, "{}", by_name.head);
+
+    let fallback = dev.get("/posts/2");
+    assert_eq!(fallback.status, 200, "{}", fallback.head);
+    assert_eq!(fallback.text(), expected);
+    let missing = dev.get("/missing.js");
+    assert_eq!(missing.status, 404, "{}", missing.head);
+    assert_eq!(missing.header("cache-control"), Some("no-store"));
+
+    let mut long = format!("GET / HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nX-Long: ");
+    long.push_str(&"a".repeat(17 * 1024));
+    long.push_str("\r\n\r\n");
+    let too_long = dev.send(&long);
+    assert_eq!(too_long.status, 400, "{}", too_long.head);
+}
+
+/// A link out of `dist/` is `403` (spec §5.3): a link to the project's
+/// manifest, and a link to the project's directory.
+#[cfg(unix)]
+#[test]
+fn a_link_out_of_dist_is_refused() {
+    let dir = project("link-out", MAIN);
+    let dev = Dev::start(&dir);
+    let dist = dir.join("dist");
+    std::os::unix::fs::symlink(dir.join("nova.toml"), dist.join("leak.toml")).expect("a link");
+    std::os::unix::fs::symlink(&dir, dist.join("up")).expect("a link");
+    for path in ["/leak.toml", "/up/nova.toml", "/up/src/main.nova"] {
+        let refused = dev.get(path);
+        assert_eq!(refused.status, 403, "{path}: {}", refused.head);
+    }
+}
+
+/// The event stream (spec §5.3): `hello` with the build id the page was
+/// served with, then `reload` for an edit; a page served before that build
+/// that connects after it hears a `hello` that is not its id, which reloads
+/// it; `error` with the checker's message for a type error, which a page
+/// served after it hears after a `hello` with that failed build's id, its
+/// own; and `reload` for the fix. Each build is on `nova dev`'s stderr too.
+#[test]
+fn the_event_stream_reloads_and_reports_errors() {
+    let dir = project("events", MAIN);
+    let dev = Dev::start(&dir);
+    let main = dir.join("src").join("main.nova");
+    let served = tag_id(&dev.get("/").text());
+    let mut events = EventStream::open(dev.port);
+    assert_eq!(events.expect("hello"), served);
+    let (start, number) = halves(&served);
+    assert_eq!(number, 1, "{served}");
+    let id = |n: u64| format!("{start}.{n}");
+
+    std::fs::write(&main, MAIN.replace("Count:", "Clicks:")).expect("edit the source");
+    assert_eq!(events.expect("reload"), id(2));
+    // A page served at build 1 that missed build 2 connects now.
+    let mut missed = EventStream::open(dev.port);
+    let hello = missed.expect("hello");
+    assert_eq!(hello, id(2));
+    assert_ne!(hello, served, "the page would not reload");
+    let wasm = dev.get("/app.wasm").body;
+    assert!(
+        wasm.windows(7).any(|w| w == b"Clicks:"),
+        "the module is rebuilt"
+    );
+
+    std::fs::write(&main, broken()).expect("write a type error");
+    let errors = events.expect("error");
+    assert!(errors.contains(CHECKER_SAYS), "{errors}");
+    assert!(errors.contains("main.nova:"), "{errors}");
+    assert!(!errors.contains('\u{1b}'), "no colour: {errors:?}");
+
+    assert_eq!(missed.expect("error"), errors);
+    // A page served after the failed build carries its id, so its `hello`
+    // matches, and it shows the errors.
+    let after_failure = dev.get("/posts/2").text();
+    assert_eq!(tag_id(&after_failure), id(3));
+    let mut late = EventStream::open(dev.port);
+    assert_eq!(late.expect("hello"), id(3));
+    assert_eq!(late.expect("error"), errors);
+
+    std::fs::write(&main, MAIN).expect("fix the source");
+    assert_eq!(events.expect("reload"), id(4));
+    assert_eq!(missed.expect("reload"), id(4));
+    assert_eq!(late.expect("reload"), id(4));
+    assert_eq!(tag_id(&dev.get("/").text()), id(4));
+
+    let stderr = dev.stderr();
+    assert!(stderr.contains(CHECKER_SAYS), "{stderr}");
+    assert_eq!(stderr.matches("rebuilt in ").count(), 3, "{stderr}");
+}
+
+/// Before the first good build (spec §4.4): with no `dist/`, `/` is the
+/// server's own page with the script, a page hears the first build's
+/// errors at once, and the fix reloads it into the project's page.
+#[test]
+fn a_first_build_that_fails_serves_the_servers_own_page() {
+    let dir = project("first-fails", &broken());
+    let dev = Dev::start(&dir);
+    let own = dev.get("/");
+    assert_eq!(own.status, 200, "{}", own.head);
+    let served = tag_id(&own.text());
+    let (start, number) = halves(&served);
+    assert_eq!(number, 1, "{served}");
+    assert!(
+        own.text().contains(&format!("{}</body>", tag(&served))),
+        "{}",
+        own.text()
+    );
+    assert!(!own.text().contains("id=\"app\""), "{}", own.text());
+
+    let mut events = EventStream::open(dev.port);
+    assert_eq!(events.expect("hello"), served);
+    assert!(events.expect("error").contains(CHECKER_SAYS));
+    std::fs::write(dir.join("src").join("main.nova"), MAIN).expect("fix the source");
+    assert_eq!(events.expect("reload"), format!("{start}.2"));
+    let page = dev.get("/").text();
+    assert!(page.contains("id=\"app\""), "{page}");
+    assert_eq!(tag_id(&page), format!("{start}.2"));
+}
+
+/// The port (spec §5.3): a second `nova dev` on the first one's port fails
+/// with the `--port` message, and status 1.
+#[test]
+fn a_second_server_on_the_same_port_fails() {
+    let dir = project("port", MAIN);
+    let first = Dev::start(&dir);
+    let port = first.port.to_string();
+    let (mut second, _lines, stderr) = spawn(&dir, &["--port", &port]);
+    let status = exit_of(&mut second);
+    std::thread::sleep(Duration::from_millis(200));
+    let stderr = stderr.lock().unwrap().replace("\r\n", "\n");
+    assert_eq!(status.code(), Some(1), "{stderr}");
+    assert_eq!(
+        stderr,
+        format!("nova dev: port {port} is in use; choose another with --port\n")
+    );
+    // The first still serves.
+    assert_eq!(first.get("/").status, 200);
+}
+
+/// What `nova build --target wasm` refuses, `nova dev` refuses before it
+/// binds a port (spec §4.1): an output directory that holds the project,
+/// and a library.
+#[test]
+fn it_refuses_what_build_refuses() {
+    let dir = project("refusals", MAIN);
+    let (mut child, _lines, stderr) = spawn(&dir, &["--port", "0", "-o", "."]);
+    let status = exit_of(&mut child);
+    std::thread::sleep(Duration::from_millis(200));
+    let text = stderr.lock().unwrap().clone();
+    assert!(!status.success());
+    assert!(
+        text.contains("the output directory holds the project"),
+        "{text}"
+    );
+
+    let lib = project("refusals-lib", MAIN);
+    std::fs::rename(
+        lib.join("src").join("main.nova"),
+        lib.join("src").join("lib.nova"),
+    )
+    .expect("make it a library");
+    let (mut child, _lines, stderr) = spawn(&lib, &["--port", "0"]);
+    let status = exit_of(&mut child);
+    std::thread::sleep(Duration::from_millis(200));
+    let text = stderr.lock().unwrap().clone();
+    assert!(!status.success());
+    assert!(text.contains("`dev-app` is a library"), "{text}");
+}
````

**Patch `t3-code`:**

<!-- patch t3-code -->

````diff
diff --git a/crates/nova-cli/src/bundle.rs b/crates/nova-cli/src/bundle.rs
index fc05d00ddfec32cd218c9e688992d92543a7d668..4d155078546738cc31d0e0282b63d0bf0bfaa4c2 100644
--- a/crates/nova-cli/src/bundle.rs
+++ b/crates/nova-cli/src/bundle.rs
@@ -55,17 +55,7 @@ impl Bundle {
     /// would overwrite and whose `assets/` a build replaces, or one inside
     /// `public/`, which would copy itself into itself.
     pub fn check_output(&self, out: &Path) -> Result<()> {
-        let Some(root) = &self.root else {
-            return Ok(());
-        };
-        let out = resolved(out);
-        if resolved(root).starts_with(&out) {
-            bail!("the output directory holds the project; name another with `-o`");
-        }
-        if out.starts_with(resolved(&root.join("public"))) {
-            bail!("the output directory is inside `public/`; name another with `-o`");
-        }
-        Ok(())
+        check_output_for(self.root.as_deref(), out)
     }
 
     /// Before the compiler writes `out`: a link at a name the build writes,
@@ -146,6 +136,25 @@ impl Bundle {
     }
 }
 
+/// [`Bundle::check_output`] for a program whose project's root is `root`,
+/// or for a single file with `None`, before there is a program: `nova dev`
+/// refuses an output directory before its first build (the 4.5 spec
+/// `docs/superpowers/specs/2026-10-18-phase-4-5-router-and-nova-dev-design.md`
+/// §4.1).
+pub fn check_output_for(root: Option<&Path>, out: &Path) -> Result<()> {
+    let Some(root) = root else {
+        return Ok(());
+    };
+    let out = resolved(out);
+    if resolved(root).starts_with(&out) {
+        bail!("the output directory holds the project; name another with `-o`");
+    }
+    if out.starts_with(resolved(&root.join("public"))) {
+        bail!("the output directory is inside `public/`; name another with `-o`");
+    }
+    Ok(())
+}
+
 /// `s` as a JSON string.
 fn quote(s: &str) -> String {
     let mut out = String::with_capacity(s.len() + 2);
@@ -257,7 +266,7 @@ fn remove(path: &Path) -> Result<()> {
 /// equal before either is created: `nope/..` is the current directory, as
 /// `create_dir_all` and every write through it find. Windows takes each
 /// `..` away by the text first, before it looks at the disk.
-fn resolved(path: &Path) -> PathBuf {
+pub fn resolved(path: &Path) -> PathBuf {
     let mut out = PathBuf::new();
     let start = std::env::current_dir().unwrap_or_default();
     let whole = if path.is_absolute() {
diff --git a/crates/nova-cli/src/cmd/dev.rs b/crates/nova-cli/src/cmd/dev.rs
new file mode 100644
index 0000000000000000000000000000000000000000..bd5daa000013f459f76321ac6e7e881f74441ce0
--- /dev/null
+++ b/crates/nova-cli/src/cmd/dev.rs
@@ -0,0 +1,554 @@
+//! `nova dev [FILE]`: build a program into its output directory, serve it on
+//! `127.0.0.1`, and build it again when a source changes, then reload the
+//! page, or show the build's errors over it (the 4.5 spec
+//! `docs/superpowers/specs/2026-10-18-phase-4-5-router-and-nova-dev-design.md`
+//! §4). This module is the command, its builds and its watcher; the server
+//! and the event stream are `crate::dev::server`.
+
+use std::collections::BTreeMap;
+use std::io::Write;
+use std::net::TcpListener;
+use std::path::{Path, PathBuf};
+use std::process::{Command, Stdio};
+use std::sync::Arc;
+use std::time::{Duration, Instant, SystemTime};
+
+use anyhow::{Context, Result};
+use clap::Args;
+
+use crate::bundle;
+use crate::cmd::run::wasm_output;
+use crate::dev::server::{self, Events, Server};
+use crate::project::{self, Mode};
+
+#[derive(Args)]
+pub struct DevCmd {
+    /// The Nova source file to serve (default: as for `nova run`).
+    file: Option<PathBuf>,
+
+    /// The port to serve on, on 127.0.0.1; 0 lets the system choose one.
+    #[arg(long, default_value_t = 3000)]
+    port: u16,
+
+    /// Open the page in the system's browser after the first build.
+    #[arg(long)]
+    open: bool,
+
+    /// The directory to build into and serve (default: dist/ at the
+    /// project's root, or in the current directory).
+    #[arg(short, long)]
+    output: Option<PathBuf>,
+}
+
+/// How often the watcher reads the sources' times and sizes (spec §4.6).
+const POLL: Duration = Duration::from_millis(200);
+
+pub fn run(cmd: DevCmd) -> Result<()> {
+    // What `nova build --target wasm` refuses, refused before anything is
+    // bound or built (spec §4.1, §4.2).
+    let mode = project::mode(cmd.file.clone())?;
+    project::refuse_library(&mode)?;
+    if let Some(root) = project::package_root(&mode) {
+        project::refuse_cached(&root)?;
+    }
+    let out = wasm_output(cmd.output, &mode);
+    let root = match &mode {
+        Mode::Project { root, .. } => Some(root.as_path()),
+        Mode::File(_) => None,
+    };
+    bundle::check_output_for(root, &out)?;
+
+    let listener = match TcpListener::bind(("127.0.0.1", cmd.port)) {
+        Ok(listener) => listener,
+        Err(e) if e.kind() == std::io::ErrorKind::AddrInUse => {
+            eprintln!(
+                "nova dev: port {} is in use; choose another with --port",
+                cmd.port
+            );
+            std::process::exit(1);
+        }
+        Err(e) => return Err(e).with_context(|| format!("binding 127.0.0.1:{}", cmd.port)),
+    };
+    let port = listener
+        .local_addr()
+        .context("reading the bound address")?
+        .port();
+
+    let events = Arc::new(Events::started_now());
+    let builder = Builder {
+        file: cmd.file,
+        out: out.clone(),
+        events: Arc::clone(&events),
+    };
+    let watched = Watched::of(&mode, &out);
+    // Read before the first build, so that an edit made during it starts
+    // another.
+    let mut seen = watched.snapshot();
+    builder.build();
+
+    let url = format!("http://127.0.0.1:{port}/");
+    println!("nova dev: serving {url}");
+    let _ = std::io::stdout().flush();
+    if cmd.open {
+        open(&url);
+    }
+
+    std::thread::spawn(move || loop {
+        seen = watched.changed(&seen);
+        builder.build();
+    });
+    server::serve(
+        listener,
+        Server {
+            port,
+            dist: out,
+            events,
+        },
+    );
+    Ok(())
+}
+
+/// Runs the builds, one at a time, each in a child process (spec §4.3).
+struct Builder {
+    file: Option<PathBuf>,
+    out: PathBuf,
+    events: Arc<Events>,
+}
+
+impl Builder {
+    /// One build: `rebuilt in <ms> ms` and `reload` when it succeeds; when
+    /// it fails, its errors on stderr and in `error`. Whatever else the
+    /// child writes on stderr, such as notes, is printed first.
+    fn build(&self) {
+        let started = Instant::now();
+        match self.child() {
+            Ok(notes) => {
+                eprint!("{notes}");
+                eprintln!("rebuilt in {} ms", started.elapsed().as_millis());
+                self.events.built(Ok(()));
+            }
+            Err(errors) => {
+                eprint!("{errors}");
+                if !errors.ends_with('\n') {
+                    eprintln!();
+                }
+                self.events.built(Err(errors));
+            }
+        }
+    }
+
+    /// `nova build --target wasm [FILE] -o <out>`, the running binary, with
+    /// no colour: its stderr, as `Ok` when it succeeds and `Err` when it
+    /// fails. Its errors carry no backtrace, which would only fill the
+    /// overlay; a panic still has one when `RUST_BACKTRACE` asks.
+    fn child(&self) -> Result<String, String> {
+        let exe = std::env::current_exe()
+            .map_err(|e| format!("nova dev: cannot find the nova binary: {e}"))?;
+        let mut command = Command::new(exe);
+        command.args(["build", "--target", "wasm"]);
+        if let Some(file) = &self.file {
+            command.arg(file);
+        }
+        command
+            .arg("-o")
+            .arg(&self.out)
+            .env("NO_COLOR", "1")
+            .env("RUST_LIB_BACKTRACE", "0")
+            .stdin(Stdio::null());
+        let output = command
+            .output()
+            .map_err(|e| format!("nova dev: cannot run the build: {e}"))?;
+        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
+        if output.status.success() {
+            Ok(stderr)
+        } else if stderr.trim().is_empty() {
+            Err(format!("the build failed: {}\n", output.status))
+        } else {
+            Err(stderr)
+        }
+    }
+}
+
+/// Each watched file's modification time and size.
+type Snapshot = BTreeMap<PathBuf, (Option<SystemTime>, u64)>;
+
+/// What the watcher reads (spec §4.6): for a project, every `.nova` file
+/// under its root, `nova.toml`, `index.html` and everything under
+/// `public/`; for a single file, the `.nova` files of its directory, not
+/// below it, and the file. Under a project's root, a directory named
+/// `target`, the output directory and a directory whose name begins with
+/// `.` are skipped.
+struct Watched {
+    /// Where `.nova` files are looked for.
+    sources: PathBuf,
+    /// Whether they are looked for below `sources` too: for a project, but
+    /// not for a single file, whose program cannot import a module from a
+    /// directory below its own (E0900), so that `nova dev ~/x.nova` does
+    /// not read a whole tree every 200 ms.
+    deep: bool,
+    /// Files read whether or not they end in `.nova`.
+    files: Vec<PathBuf>,
+    /// Directories whose every file is read.
+    trees: Vec<PathBuf>,
+    /// The output directory, never read, so a build does not start another.
+    out: PathBuf,
+}
+
+impl Watched {
+    fn of(mode: &Mode, out: &Path) -> Watched {
+        let here = |dir: &Path| {
+            if dir.as_os_str().is_empty() {
+                PathBuf::from(".")
+            } else {
+                dir.to_path_buf()
+            }
+        };
+        match mode {
+            Mode::Project { root, .. } => Watched {
+                sources: here(root),
+                deep: true,
+                files: vec![root.join(nova_pm::MANIFEST), root.join("index.html")],
+                trees: vec![root.join("public")],
+                out: out.to_path_buf(),
+            },
+            Mode::File(file) => Watched {
+                sources: here(file.parent().unwrap_or(Path::new(""))),
+                deep: false,
+                files: vec![file.clone()],
+                trees: Vec::new(),
+                out: out.to_path_buf(),
+            },
+        }
+    }
+
+    fn snapshot(&self) -> Snapshot {
+        let mut seen = Snapshot::new();
+        let out = bundle::resolved(&self.out);
+        sources(&self.sources, &out, self.deep, &mut seen);
+        for file in &self.files {
+            stat(file, &mut seen);
+        }
+        for tree in &self.trees {
+            everything(tree, &out, &mut seen);
+        }
+        seen
+    }
+
+    /// Polls every 200 ms until a file is added, removed or changed since
+    /// `seen`, then until one more poll sees nothing new: what that last
+    /// poll read.
+    fn changed(&self, seen: &Snapshot) -> Snapshot {
+        settled(seen, || {
+            std::thread::sleep(POLL);
+            self.snapshot()
+        })
+    }
+}
+
+/// What `poll` returns once it differs from `seen` and one more poll
+/// returns the same, so that an editor that writes twice starts one build
+/// (spec §4.6).
+fn settled<T: PartialEq>(seen: &T, mut poll: impl FnMut() -> T) -> T {
+    let mut now = poll();
+    while now == *seen {
+        now = poll();
+    }
+    loop {
+        let next = poll();
+        if next == now {
+            return now;
+        }
+        now = next;
+    }
+}
+
+/// Whether the directory `dir` is the output directory `out`, resolved.
+fn is_out(dir: &Path, out: &Path) -> bool {
+    dir.canonicalize().is_ok_and(|dir| dir == out)
+}
+
+/// The `.nova` files in `dir`, and when `deep` those below it, as `nova fmt`
+/// finds them: directories named `target` or beginning with `.` are
+/// skipped, and so is `out`. A link to a directory is not followed.
+fn sources(dir: &Path, out: &Path, deep: bool, seen: &mut Snapshot) {
+    let Ok(entries) = std::fs::read_dir(dir) else {
+        return;
+    };
+    for entry in entries.flatten() {
+        let path = entry.path();
+        let name = entry.file_name().to_string_lossy().into_owned();
+        let Ok(kind) = entry.file_type() else {
+            continue;
+        };
+        if kind.is_dir() {
+            if deep && name != "target" && !name.starts_with('.') && !is_out(&path, out) {
+                sources(&path, out, deep, seen);
+            }
+        } else if name.ends_with(".nova") {
+            stat(&path, seen);
+        }
+    }
+}
+
+/// Every file under `dir`, `out` aside. A link to a directory is not
+/// followed.
+fn everything(dir: &Path, out: &Path, seen: &mut Snapshot) {
+    let Ok(entries) = std::fs::read_dir(dir) else {
+        return;
+    };
+    for entry in entries.flatten() {
+        let path = entry.path();
+        match entry.file_type() {
+            Ok(kind) if kind.is_dir() => {
+                if !is_out(&path, out) {
+                    everything(&path, out, seen);
+                }
+            }
+            Ok(_) => stat(&path, seen),
+            Err(_) => {}
+        }
+    }
+}
+
+/// `path`'s time and size, when it is there, through any link; a missing
+/// file is left out, so that its coming and going is a change.
+fn stat(path: &Path, seen: &mut Snapshot) {
+    match std::fs::metadata(path) {
+        Ok(meta) if meta.is_file() => {
+            seen.insert(path.to_path_buf(), (meta.modified().ok(), meta.len()));
+        }
+        Ok(_) => {}
+        // A link to nothing is still a file of the program's.
+        Err(_) if std::fs::symlink_metadata(path).is_ok() => {
+            seen.insert(path.to_path_buf(), (None, 0));
+        }
+        Err(_) => {}
+    }
+}
+
+/// The command that opens `url` in the system's browser (spec §4.1).
+fn opener(url: &str) -> Command {
+    if cfg!(windows) {
+        let mut command = Command::new("cmd");
+        command.args(["/c", "start", "", url]);
+        command
+    } else if cfg!(target_os = "macos") {
+        let mut command = Command::new("open");
+        command.arg(url);
+        command
+    } else {
+        let mut command = Command::new("xdg-open");
+        command.arg(url);
+        command
+    }
+}
+
+/// Open `url` in the system's browser. A failure is a note on stderr, not
+/// an error: the URL is printed already.
+fn open(url: &str) {
+    let mut command = opener(url);
+    let program = command.get_program().to_string_lossy().into_owned();
+    let child = command
+        .stdin(Stdio::null())
+        .stdout(Stdio::null())
+        .stderr(Stdio::null())
+        .spawn();
+    match child {
+        Ok(mut child) => {
+            std::thread::spawn(move || match child.wait() {
+                Ok(status) if !status.success() => {
+                    eprintln!("note: `{program}` could not open a browser: {status}");
+                }
+                Ok(_) => {}
+                Err(e) => eprintln!("note: `{program}` could not open a browser: {e}"),
+            });
+        }
+        Err(e) => eprintln!("note: could not run `{program}` to open a browser: {e}"),
+    }
+}
+
+#[cfg(test)]
+mod tests {
+    use super::*;
+
+    /// A fresh directory with a fixed name, so each run replaces the last.
+    fn fresh(name: &str) -> PathBuf {
+        let dir = std::env::temp_dir().join(format!("nova-dev-watch-{name}"));
+        let _ = std::fs::remove_dir_all(&dir);
+        std::fs::create_dir_all(&dir).expect("create the test directory");
+        dir
+    }
+
+    fn write(dir: &Path, files: &[&str]) {
+        for path in files {
+            let path = dir.join(path);
+            std::fs::create_dir_all(path.parent().unwrap()).expect("a directory");
+            std::fs::write(path, "x").expect("a file");
+        }
+    }
+
+    fn names(dir: &Path, seen: &Snapshot) -> Vec<String> {
+        seen.keys()
+            .map(|p| {
+                p.strip_prefix(dir)
+                    .unwrap_or(p)
+                    .to_string_lossy()
+                    .replace('\\', "/")
+            })
+            .collect()
+    }
+
+    #[test]
+    fn a_project_watches_its_sources_manifest_page_and_public() {
+        let dir = fresh("project");
+        write(
+            &dir,
+            &[
+                "nova.toml",
+                "index.html",
+                "notes.md",
+                "src/main.nova",
+                "src/ui/view.nova",
+                "src/ui/view.txt",
+                "tests/t.nova",
+                "public/logo.png",
+                "public/css/site.css",
+                "target/debug/gen.nova",
+                "src/target/skip.nova",
+                ".git/hook.nova",
+                "dist/app.nova",
+                "build/out.nova",
+            ],
+        );
+        let mode = Mode::Project {
+            root: dir.clone(),
+            target_dir: dir.join("target"),
+        };
+        let seen = Watched::of(&mode, &dir.join("dist")).snapshot();
+        assert_eq!(
+            names(&dir, &seen),
+            [
+                "build/out.nova",
+                "index.html",
+                "nova.toml",
+                "public/css/site.css",
+                "public/logo.png",
+                "src/main.nova",
+                "src/ui/view.nova",
+                "tests/t.nova",
+            ]
+        );
+        // `-o` elsewhere in the project skips that directory instead, by
+        // any spelling.
+        let seen = Watched::of(&mode, &dir.join("src").join("..").join("build")).snapshot();
+        assert!(names(&dir, &seen).contains(&"dist/app.nova".to_string()));
+        assert!(!names(&dir, &seen).contains(&"build/out.nova".to_string()));
+    }
+
+    /// The `.nova` files of the file's directory, not those below it, which
+    /// its program cannot import (E0900): a change to one starts no build.
+    #[test]
+    fn a_single_file_watches_the_nova_files_of_its_directory() {
+        let dir = fresh("file");
+        write(
+            &dir,
+            &[
+                "app/main.nova",
+                "app/util.nova",
+                "app/page.html",
+                "app/dist/x.nova",
+                "app/sub/deep.nova",
+                "app/sub/deeper/deepest.nova",
+                "other.nova",
+            ],
+        );
+        let mode = Mode::File(dir.join("app").join("main.nova"));
+        let watched = Watched::of(&mode, &dir.join("app").join("dist"));
+        let seen = watched.snapshot();
+        assert_eq!(names(&dir, &seen), ["app/main.nova", "app/util.nova"]);
+        std::fs::write(dir.join("app/sub/deep.nova"), "longer").unwrap();
+        std::fs::write(dir.join("app/sub/new.nova"), "x").unwrap();
+        assert_eq!(watched.snapshot(), seen);
+    }
+
+    #[test]
+    fn a_change_an_addition_and_a_removal_are_each_seen() {
+        let dir = fresh("changes");
+        write(&dir, &["nova.toml", "src/main.nova"]);
+        let mode = Mode::Project {
+            root: dir.clone(),
+            target_dir: dir.join("target"),
+        };
+        let watched = Watched::of(&mode, &dir.join("dist"));
+        let first = watched.snapshot();
+        std::fs::write(dir.join("src/main.nova"), "longer").unwrap();
+        let second = watched.changed(&first);
+        assert_ne!(first, second);
+        std::fs::write(dir.join("src/more.nova"), "x").unwrap();
+        let third = watched.changed(&second);
+        assert_eq!(third.len(), 3);
+        std::fs::remove_file(dir.join("src/more.nova")).unwrap();
+        let fourth = watched.changed(&third);
+        assert_eq!(fourth, second);
+        // The output directory changing is no change.
+        std::fs::create_dir_all(dir.join("dist")).unwrap();
+        std::fs::write(dir.join("dist/app.nova"), "x").unwrap();
+        assert_eq!(watched.snapshot(), fourth);
+    }
+
+    /// A link to nothing is still a file of the program's, with no time and
+    /// no size, so that its target's coming and going is a change.
+    #[cfg(unix)]
+    #[test]
+    fn a_link_to_nothing_is_watched_as_a_file() {
+        let dir = fresh("dangling");
+        write(&dir, &["nova.toml", "src/main.nova"]);
+        let link = dir.join("src").join("linked.nova");
+        std::os::unix::fs::symlink(dir.join("src").join("gone.nova"), &link).expect("a link");
+        let mode = Mode::Project {
+            root: dir.clone(),
+            target_dir: dir.join("target"),
+        };
+        let watched = Watched::of(&mode, &dir.join("dist"));
+        let dangling = watched.snapshot();
+        assert_eq!(dangling.get(&link), Some(&(None, 0)));
+        assert_eq!(dangling.len(), 3);
+        std::fs::write(dir.join("src").join("gone.nova"), "back").unwrap();
+        let back = watched.snapshot();
+        assert_eq!(back.get(&link).map(|(_, size)| *size), Some(4));
+        assert_ne!(back, dangling);
+    }
+
+    #[test]
+    fn a_build_waits_for_a_poll_that_sees_nothing_new() {
+        // Polls of a file written three times, then left alone: the change
+        // is seen at the third poll, and settles at the sixth.
+        let mut polls = [0, 0, 1, 2, 3, 3, 4].into_iter();
+        let mut taken = 0;
+        let settled = settled(&0, || {
+            taken += 1;
+            polls.next().expect("a poll")
+        });
+        assert_eq!((settled, taken), (3, 6));
+    }
+
+    #[test]
+    fn the_opener_is_the_systems() {
+        let command = opener("http://127.0.0.1:1/");
+        let args: Vec<_> = command
+            .get_args()
+            .map(|a| a.to_string_lossy().into_owned())
+            .collect();
+        let program = command.get_program().to_string_lossy().into_owned();
+        if cfg!(windows) {
+            assert_eq!(program, "cmd");
+            assert_eq!(args, ["/c", "start", "", "http://127.0.0.1:1/"]);
+        } else if cfg!(target_os = "macos") {
+            assert_eq!(program, "open");
+            assert_eq!(args, ["http://127.0.0.1:1/"]);
+        } else {
+            assert_eq!(program, "xdg-open");
+            assert_eq!(args, ["http://127.0.0.1:1/"]);
+        }
+    }
+}
diff --git a/crates/nova-cli/src/cmd/mod.rs b/crates/nova-cli/src/cmd/mod.rs
index b3bf274cae2e16ad667cbd45c30ad7a0c5187b8b..2cd96806c601910d379fed1ecd92e51eca4dd0e4 100644
--- a/crates/nova-cli/src/cmd/mod.rs
+++ b/crates/nova-cli/src/cmd/mod.rs
@@ -1,6 +1,7 @@
 //! CLI subcommand implementations.
 
 pub mod deps;
+pub mod dev;
 pub mod doc;
 pub mod fetch;
 pub mod fmt;
diff --git a/crates/nova-cli/src/cmd/run.rs b/crates/nova-cli/src/cmd/run.rs
index 45ec6a2374ed4f01641d0550a3847e1d07903d3f..336e2e0075a1b12f3faed63abc88e0040fed1955 100644
--- a/crates/nova-cli/src/cmd/run.rs
+++ b/crates/nova-cli/src/cmd/run.rs
@@ -139,11 +139,7 @@ fn build_wasm(cmd: BuildCmd) -> Result<()> {
     project::refuse_library(&mode)?;
     project::sync(&mode)?;
     let program = project::program_to_run(&mode)?;
-    let output = match (cmd.output, &mode) {
-        (Some(output), _) => output,
-        (None, Mode::Project { root, .. }) => root.join("dist"),
-        (None, Mode::File(_)) => PathBuf::from("dist"),
-    };
+    let output = wasm_output(cmd.output, &mode);
     let bundle = Bundle::of(&mode, &program, cmd.release);
     bundle.check_output(&output)?;
     bundle.prepare(&output)?;
@@ -160,6 +156,17 @@ fn build_wasm(cmd: BuildCmd) -> Result<()> {
     }
 }
 
+/// The directory `nova build --target wasm` writes, which `nova dev` builds
+/// into and serves: `output` when `-o` names it, or `dist/` at the
+/// project's root, or in the current directory.
+pub fn wasm_output(output: Option<PathBuf>, mode: &Mode) -> PathBuf {
+    match (output, mode) {
+        (Some(output), _) => output,
+        (None, Mode::Project { root, .. }) => root.join("dist"),
+        (None, Mode::File(_)) => PathBuf::from("dist"),
+    }
+}
+
 pub fn build(cmd: BuildCmd) -> Result<()> {
     if cmd.target == TargetArg::Wasm {
         return build_wasm(cmd);
diff --git a/crates/nova-cli/src/dev/mod.rs b/crates/nova-cli/src/dev/mod.rs
new file mode 100644
index 0000000000000000000000000000000000000000..904cba9a2675a902a50493b74cdf75f963f312ec
--- /dev/null
+++ b/crates/nova-cli/src/dev/mod.rs
@@ -0,0 +1,7 @@
+//! What `nova dev` serves (the 4.5 spec
+//! `docs/superpowers/specs/2026-10-18-phase-4-5-router-and-nova-dev-design.md`
+//! §4.4, §4.5): the HTTP server and its event stream, with the reload script
+//! it adds to each page. The command itself, its builds and its watcher are
+//! `crate::cmd::dev`.
+
+pub mod server;
diff --git a/crates/nova-cli/src/dev/reload.js b/crates/nova-cli/src/dev/reload.js
new file mode 100644
index 0000000000000000000000000000000000000000..327f0d5abd81fa5ac511b2ff8c40c283463b7a6b
--- /dev/null
+++ b/crates/nova-cli/src/dev/reload.js
@@ -0,0 +1,50 @@
+// nova dev's reload script (spec
+// docs/superpowers/specs/2026-10-18-phase-4-5-router-and-nova-dev-design.md
+// §4.5), which the server adds to every page it serves and never writes
+// into a file. Its tag carries `data-build`, the id of the build current
+// when the server served the page. It listens to /__nova/events: a `hello`
+// whose id is not the page's own reloads the page, so a page served before
+// a build it missed reloads, and so does every page after a restart;
+// `reload` reloads it; `error` shows the build's errors over it, until a
+// good build reloads it. A page served after a failed build carries that
+// build's id, so its `hello` matches, and the `error` after it shows.
+(() => {
+  // Read once, while the script runs: `currentScript` is null afterwards.
+  const served = document.currentScript?.dataset.build ?? null;
+  const events = new EventSource("/__nova/events");
+
+  // A tag without an id never reloads on `hello`, rather than on every one.
+  events.addEventListener("hello", (e) => {
+    if (served !== null && e.data !== served) location.reload();
+  });
+
+  events.addEventListener("reload", () => location.reload());
+
+  // The stream's own failures arrive as `error` too, as plain events with
+  // no data; EventSource reconnects by itself after them.
+  events.addEventListener("error", (e) => {
+    if (typeof e.data === "string") show(e.data);
+  });
+
+  // The overlay: a fixed layer over the whole page, its text set as
+  // `textContent`, so that nothing in an error is read as HTML.
+  function show(text) {
+    let overlay = document.getElementById("nova-dev-overlay");
+    if (overlay === null) {
+      overlay = document.createElement("div");
+      overlay.id = "nova-dev-overlay";
+      overlay.style.cssText =
+        "position:fixed;inset:0;z-index:2147483647;overflow:auto;box-sizing:border-box;" +
+        "margin:0;padding:24px;background:rgba(24,24,27,0.96);color:#f4f4f5;" +
+        "font:14px/1.5 ui-monospace,SFMono-Regular,Menlo,Consolas,monospace;";
+      const title = document.createElement("h1");
+      title.textContent = "nova dev: the build failed";
+      title.style.cssText = "margin:0 0 16px;font-size:18px;color:#fca5a5;";
+      const pre = document.createElement("pre");
+      pre.style.cssText = "margin:0;white-space:pre-wrap;";
+      overlay.append(title, pre);
+      (document.body ?? document.documentElement).append(overlay);
+    }
+    overlay.querySelector("pre").textContent = text;
+  }
+})();
diff --git a/crates/nova-cli/src/dev/server.rs b/crates/nova-cli/src/dev/server.rs
new file mode 100644
index 0000000000000000000000000000000000000000..f38e1994c53f0092c8a8e5794b529b91d26b00c7
--- /dev/null
+++ b/crates/nova-cli/src/dev/server.rs
@@ -0,0 +1,1217 @@
+//! `nova dev`'s server (the 4.5 spec
+//! `docs/superpowers/specs/2026-10-18-phase-4-5-router-and-nova-dev-design.md`
+//! §4.4, §4.5): HTTP/1.1 on `std::net`, a thread per connection, bound to
+//! `127.0.0.1` only. It serves the output directory's files, adds the reload
+//! script to every page, answers a path with no file with the page, for a
+//! router in history mode, and keeps the event stream that tells each page to
+//! reload, or shows it a build's errors. A page answered at a path such as
+//! `/posts/2` works when it loads `/app.js` by its absolute path, as the
+//! generated page does; a project's own page must do the same.
+//!
+//! Each build has an id, `<start>.<number>`: the server's start in
+//! milliseconds since the Unix epoch, and the build's number, from 1. A page
+//! carries the id current when it was served, in its script tag; `hello`
+//! and `reload` carry the current one. A page reloads on a `hello` whose id
+//! is not its own, so a page served before a build it missed reloads, and
+//! so does every page after a restart, whose ids differ.
+
+use std::io::{Read, Write};
+use std::net::{Shutdown, TcpListener, TcpStream};
+use std::path::{Component, Path, PathBuf};
+use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
+use std::time::{Duration, SystemTime, UNIX_EPOCH};
+
+/// The reload script, served at `/__nova/reload.js` and never read from
+/// disk.
+pub const RELOAD_JS: &str = include_str!("reload.js");
+
+/// What every page gets before its last `</body>`: the reload script, with
+/// `id`, the id of the build current when the page is served.
+pub fn script_tag(id: &str) -> String {
+    format!("<script src=\"/__nova/reload.js\" data-build=\"{id}\"></script>")
+}
+
+/// The longest head a request may have, its last empty line included.
+const HEAD_LIMIT: usize = 16 * 1024;
+
+/// How long a connection may wait for a read or a write.
+const IO_TIMEOUT: Duration = Duration::from_secs(10);
+
+/// How long a page's event stream may take to accept a write before it is
+/// dropped.
+const EVENT_WRITE_TIMEOUT: Duration = Duration::from_secs(5);
+
+/// How often the event stream sends a comment, so that nothing between the
+/// page and the server closes it as idle.
+const PING_EVERY: Duration = Duration::from_secs(15);
+
+const HTML: &str = "text/html; charset=utf-8";
+const JS: &str = "text/javascript; charset=utf-8";
+const TEXT: &str = "text/plain; charset=utf-8";
+
+/// The server's own page, for a path that would be the output directory's
+/// `index.html` before a build has written one, so that the first build's
+/// errors show. It gets the script as any page does.
+const OWN_PAGE: &str = "<!doctype html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n\
+<title>nova dev</title>\n</head>\n<body>\n<p>nova dev: there is no page yet; it shows once a \
+build succeeds.</p>\n</body>\n</html>\n";
+
+/// The head of the event stream's response, which has no length: the stream
+/// lasts until either side closes it.
+const EVENT_STREAM_HEAD: &str = "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\n\
+Cache-Control: no-store\r\nConnection: keep-alive\r\n\r\n";
+
+/// The head of the answer to a `HEAD` of the event stream, whose connection
+/// closes after it.
+const EVENT_STREAM_HEAD_ONLY: &str = "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\n\
+Cache-Control: no-store\r\nConnection: close\r\n\r\n";
+
+/// What the server needs: the port it is bound to, which a request's `Host`
+/// must name, the output directory it serves, and the event stream.
+pub struct Server {
+    pub port: u16,
+    pub dist: PathBuf,
+    pub events: Arc<Events>,
+}
+
+/// Accept connections on `listener` for ever, each on a thread of its own,
+/// and ping the event stream's pages every 15 seconds.
+pub fn serve(listener: TcpListener, server: Server) {
+    let server = Arc::new(server);
+    let events = Arc::clone(&server.events);
+    std::thread::spawn(move || loop {
+        std::thread::sleep(PING_EVERY);
+        events.ping();
+    });
+    for stream in listener.incoming() {
+        match stream {
+            Ok(stream) => {
+                let server = Arc::clone(&server);
+                std::thread::spawn(move || server.handle(stream));
+            }
+            // Out of descriptors, or a connection reset before it was
+            // accepted: the next one may do better.
+            Err(_) => std::thread::sleep(Duration::from_millis(50)),
+        }
+    }
+}
+
+/// A request's head, as far as the server reads it.
+#[derive(Debug, PartialEq, Eq)]
+pub struct Request {
+    pub method: String,
+    pub target: String,
+    /// The `Host` header's value, trimmed; `None` when there is none.
+    pub host: Option<String>,
+}
+
+/// What a request's path names (spec §4.4).
+#[derive(Debug, PartialEq, Eq)]
+pub enum Target {
+    /// `/__nova/reload.js`.
+    Reload,
+    /// `/__nova/events`.
+    Events,
+    /// A file inside the output directory, its links resolved, with its
+    /// content type, from the name the request used.
+    File { path: PathBuf, kind: &'static str },
+    /// The server's own page, when the page is asked for and the output
+    /// directory has no `index.html`.
+    OwnPage,
+    /// A refusal or a miss: `400`, `403` or `404`.
+    Status(u16),
+}
+
+/// What the server answers a request with.
+#[derive(Debug, PartialEq, Eq)]
+pub enum Answer {
+    Status(u16),
+    Body { kind: &'static str, body: Vec<u8> },
+    Events,
+}
+
+impl Server {
+    /// One connection: one request, one answer, then the connection is
+    /// closed, unless it is the event stream's.
+    fn handle(&self, mut stream: TcpStream) {
+        let _ = stream.set_read_timeout(Some(IO_TIMEOUT));
+        let _ = stream.set_write_timeout(Some(IO_TIMEOUT));
+        let request = match read_head(&mut stream) {
+            Head::Complete(head) => parse_head(&head),
+            Head::Invalid => None,
+            Head::Nothing => return,
+        };
+        let Some(request) = request else {
+            let _ = write_answer(&mut stream, &Answer::Status(400), false);
+            return close(stream);
+        };
+        let head_only = request.method == "HEAD";
+        let answer = self.answer(&request);
+        if answer == Answer::Events && !head_only {
+            let _ = stream.set_write_timeout(Some(EVENT_WRITE_TIMEOUT));
+            self.events.join(Box::new(stream));
+            return;
+        }
+        let _ = write_answer(&mut stream, &answer, head_only);
+        close(stream);
+    }
+
+    /// The answer to `request`: the `Host` check first, then the method,
+    /// then what the path names.
+    pub fn answer(&self, request: &Request) -> Answer {
+        if !host_allowed(request.host.as_deref(), self.port) {
+            return Answer::Status(403);
+        }
+        if request.method != "GET" && request.method != "HEAD" {
+            return Answer::Status(405);
+        }
+        // Read before any page, so that a build finishing in between leaves
+        // the page an older id, which reloads it, never a newer one.
+        let id = self.events.id();
+        match locate(&self.dist, &request.target) {
+            Target::Reload => Answer::Body {
+                kind: JS,
+                body: RELOAD_JS.as_bytes().to_vec(),
+            },
+            Target::Events => Answer::Events,
+            Target::OwnPage => Answer::Body {
+                kind: HTML,
+                body: inject(OWN_PAGE.as_bytes(), &id),
+            },
+            Target::File { path, kind } => match std::fs::read(path) {
+                Ok(body) if kind == HTML => Answer::Body {
+                    kind,
+                    body: inject(&body, &id),
+                },
+                Ok(body) => Answer::Body { kind, body },
+                // Removed since it was found, by a build or by hand.
+                Err(_) => Answer::Status(404),
+            },
+            Target::Status(status) => Answer::Status(status),
+        }
+    }
+}
+
+/// How reading a request's head ended.
+enum Head {
+    /// The head, up to and with its first empty line.
+    Complete(Vec<u8>),
+    /// Over 16 KiB, or cut off by the client.
+    Invalid,
+    /// Nothing came, or the read failed or timed out.
+    Nothing,
+}
+
+/// A request's head, which ends at its first empty line. A line ends in
+/// CRLF or in a bare LF, which RFC 9112 §2.2 lets a recipient take as a line
+/// end.
+fn read_head(stream: &mut impl Read) -> Head {
+    let mut buf = Vec::new();
+    let mut chunk = [0u8; 4096];
+    loop {
+        if let Some(end) = head_end(&buf) {
+            if end > HEAD_LIMIT {
+                return Head::Invalid;
+            }
+            buf.truncate(end);
+            return Head::Complete(buf);
+        }
+        if buf.len() >= HEAD_LIMIT {
+            return Head::Invalid;
+        }
+        match stream.read(&mut chunk) {
+            Ok(0) if buf.is_empty() => return Head::Nothing,
+            Ok(0) => return Head::Invalid,
+            Ok(n) => buf.extend_from_slice(&chunk[..n]),
+            Err(_) => return Head::Nothing,
+        }
+    }
+}
+
+/// Where the first empty line of `buf` ends: just after the LF that ends a
+/// line, then an LF or a CRLF.
+fn head_end(buf: &[u8]) -> Option<usize> {
+    (0..buf.len()).find_map(|i| match &buf[i..] {
+        [b'\n', b'\n', ..] => Some(i + 2),
+        [b'\n', b'\r', b'\n', ..] => Some(i + 3),
+        _ => None,
+    })
+}
+
+/// A request's head, `None` when it does not parse: a request line of a
+/// method, a target and `HTTP/1.x`, `x` a digit, then header lines of a
+/// name, `:` and a value, each line ending in CRLF or a bare LF, and then an
+/// empty line. Two `Host` headers do not parse either.
+pub fn parse_head(head: &[u8]) -> Option<Request> {
+    let text = std::str::from_utf8(head).ok()?.strip_suffix('\n')?;
+    let text = text.strip_suffix('\r').unwrap_or(text).strip_suffix('\n')?;
+    let mut lines = text
+        .split('\n')
+        .map(|line| line.strip_suffix('\r').unwrap_or(line));
+    let mut parts = lines.next()?.split(' ');
+    let (method, target, version) = (parts.next()?, parts.next()?, parts.next()?);
+    let token = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_graphic());
+    if parts.next().is_some() || !token(method) || !token(target) {
+        return None;
+    }
+    // `HTTP/1.` and one digit, so that a bare CR left at the request line's
+    // end does not pass (RFC 9112 §2.2, §2.3).
+    let minor = version.strip_prefix("HTTP/1.").unwrap_or_default();
+    if minor.len() != 1 || !minor.bytes().all(|b| b.is_ascii_digit()) {
+        return None;
+    }
+    let mut host = None;
+    for line in lines {
+        let (name, value) = line.split_once(':')?;
+        if !token(name) {
+            return None;
+        }
+        if name.eq_ignore_ascii_case("host") {
+            if host.is_some() {
+                return None;
+            }
+            host = Some(value.trim().to_string());
+        }
+    }
+    Some(Request {
+        method: method.to_string(),
+        target: target.to_string(),
+        host,
+    })
+}
+
+/// Whether `host` names the server: `127.0.0.1:<port>` or
+/// `localhost:<port>`, so that a page of another site cannot read it through
+/// a name of its own that resolves to `127.0.0.1` (DNS rebinding).
+pub fn host_allowed(host: Option<&str>, port: u16) -> bool {
+    let Some(host) = host else {
+        return false;
+    };
+    [format!("127.0.0.1:{port}"), format!("localhost:{port}")]
+        .iter()
+        .any(|allowed| allowed.eq_ignore_ascii_case(host.trim()))
+}
+
+/// What `target`, a request's target, names in `dist` (spec §4.4). A path
+/// that names no file is answered with the page so that a `HistoryRouter`
+/// page can be reloaded there, which works when the page loads `/app.js`
+/// by its absolute path, as the generated page does:
+/// - its path, before any `?`, percent-decoded, is `400` when it does not
+///   decode to UTF-8, and `403` when a piece is `..` or it holds a `\` or a
+///   NUL;
+/// - under `/__nova/`, it is the server's own, never a file;
+/// - a path ending in `/` names that directory's `index.html`;
+/// - a file is `403` when it lies outside `dist` once links are resolved;
+/// - a path that names no file, whose last piece has no `.`, is the page,
+///   for a router in history mode; any other is `404`.
+pub fn locate(dist: &Path, target: &str) -> Target {
+    let path = target.split(['?', '#']).next().unwrap_or_default();
+    if !path.starts_with('/') {
+        return Target::Status(400);
+    }
+    let Some(decoded) = percent_decode(path) else {
+        return Target::Status(400);
+    };
+    if decoded.contains(['\\', '\0']) {
+        return Target::Status(403);
+    }
+    let pieces: Vec<&str> = decoded
+        .split('/')
+        .filter(|piece| !piece.is_empty() && *piece != ".")
+        .collect();
+    if pieces.contains(&"..") {
+        return Target::Status(403);
+    }
+    if pieces.first() == Some(&"__nova") {
+        return match pieces[1..] {
+            ["reload.js"] => Target::Reload,
+            ["events"] => Target::Events,
+            _ => Target::Status(404),
+        };
+    }
+    // A piece that is not one plain name, such as `C:` on Windows, could
+    // replace the directory it is joined to.
+    if !pieces.iter().all(|piece| plain(piece)) {
+        return Target::Status(403);
+    }
+    let mut file = dist.to_path_buf();
+    file.extend(&pieces);
+    if decoded.ends_with('/') {
+        file.push("index.html");
+    }
+    let page = pieces.last().map_or(true, |piece| !piece.contains('.'));
+    let Ok(dist) = dist.canonicalize() else {
+        return fallback(None, page);
+    };
+    match file.canonicalize() {
+        Ok(real) if !real.starts_with(&dist) => Target::Status(403),
+        Ok(real) if real.is_file() => Target::File {
+            path: real,
+            kind: content_type(&file),
+        },
+        // Missing, or a directory, which is never listed.
+        _ => fallback(Some(&dist), page),
+    }
+}
+
+/// A path that names no file: the page when `page`, the output directory's
+/// own `index.html` or the server's; otherwise `404`. `dist` is the output
+/// directory, resolved, or `None` before it exists.
+fn fallback(dist: Option<&Path>, page: bool) -> Target {
+    if !page {
+        return Target::Status(404);
+    }
+    let Some(dist) = dist else {
+        return Target::OwnPage;
+    };
+    match dist.join("index.html").canonicalize() {
+        Ok(real) if !real.starts_with(dist) => Target::Status(403),
+        Ok(real) if real.is_file() => Target::File {
+            path: real,
+            kind: HTML,
+        },
+        _ => Target::OwnPage,
+    }
+}
+
+/// Whether `piece` is one name and nothing else.
+fn plain(piece: &str) -> bool {
+    let mut parts = Path::new(piece).components();
+    matches!(
+        (parts.next(), parts.next()),
+        (Some(Component::Normal(name)), None) if name == std::ffi::OsStr::new(piece)
+    )
+}
+
+/// `path` with each `%XX` replaced by its byte, `None` when a `%` is not
+/// followed by two hexadecimal digits or the bytes are not UTF-8.
+fn percent_decode(path: &str) -> Option<String> {
+    let bytes = path.as_bytes();
+    let mut out = Vec::with_capacity(bytes.len());
+    let mut i = 0;
+    while i < bytes.len() {
+        if bytes[i] == b'%' {
+            let hex = bytes.get(i + 1..i + 3)?;
+            if !hex.iter().all(u8::is_ascii_hexdigit) {
+                return None;
+            }
+            let digit = |b: u8| (b as char).to_digit(16).unwrap_or(0) as u8;
+            out.push(digit(hex[0]) * 16 + digit(hex[1]));
+            i += 3;
+        } else {
+            out.push(bytes[i]);
+            i += 1;
+        }
+    }
+    String::from_utf8(out).ok()
+}
+
+/// A file's content type, by its extension (spec §4.4).
+pub fn content_type(path: &Path) -> &'static str {
+    let extension = path
+        .extension()
+        .and_then(|e| e.to_str())
+        .map(str::to_ascii_lowercase);
+    match extension.as_deref() {
+        Some("html") => HTML,
+        Some("js" | "mjs") => JS,
+        Some("wasm") => "application/wasm",
+        Some("json") => "application/json",
+        Some("css") => "text/css; charset=utf-8",
+        Some("svg") => "image/svg+xml",
+        Some("png") => "image/png",
+        Some("jpg") => "image/jpeg",
+        Some("gif") => "image/gif",
+        Some("ico") => "image/x-icon",
+        Some("txt") => TEXT,
+        Some("woff2") => "font/woff2",
+        _ => "application/octet-stream",
+    }
+}
+
+/// `page` with the reload script, with the build id `id`, before its last
+/// `</body>`, in any case, or at its end when it has none.
+pub fn inject(page: &[u8], id: &str) -> Vec<u8> {
+    let end = b"</body>";
+    let at = (0..=page.len().saturating_sub(end.len()))
+        .rev()
+        .find(|&i| {
+            page.get(i..i + end.len())
+                .is_some_and(|w| w.eq_ignore_ascii_case(end))
+        })
+        .unwrap_or(page.len());
+    let tag = script_tag(id);
+    let mut out = Vec::with_capacity(page.len() + tag.len());
+    out.extend_from_slice(&page[..at]);
+    out.extend_from_slice(tag.as_bytes());
+    out.extend_from_slice(&page[at..]);
+    out
+}
+
+fn reason(status: u16) -> &'static str {
+    match status {
+        200 => "OK",
+        400 => "Bad Request",
+        403 => "Forbidden",
+        404 => "Not Found",
+        405 => "Method Not Allowed",
+        _ => "Error",
+    }
+}
+
+/// `answer` as an HTTP/1.1 response that closes its connection, with no body
+/// for a `HEAD`. A refusal's body is its status in words.
+fn write_answer(stream: &mut impl Write, answer: &Answer, head_only: bool) -> std::io::Result<()> {
+    let refusal;
+    let (status, kind, body) = match answer {
+        Answer::Body { kind, body } => (200, *kind, body.as_slice()),
+        Answer::Status(status) => {
+            refusal = format!("{status} {}\n", reason(*status));
+            (*status, TEXT, refusal.as_bytes())
+        }
+        // A `HEAD` of the event stream: its head, which says the connection
+        // closes, as it then does.
+        Answer::Events => {
+            stream.write_all(EVENT_STREAM_HEAD_ONLY.as_bytes())?;
+            return stream.flush();
+        }
+    };
+    let allow = if status == 405 {
+        "Allow: GET, HEAD\r\n"
+    } else {
+        ""
+    };
+    let head = format!(
+        "HTTP/1.1 {status} {}\r\nContent-Type: {kind}\r\nContent-Length: {}\r\n\
+         Cache-Control: no-store\r\n{allow}Connection: close\r\n\r\n",
+        reason(status),
+        body.len(),
+    );
+    stream.write_all(head.as_bytes())?;
+    if !head_only {
+        stream.write_all(body)?;
+    }
+    stream.flush()
+}
+
+/// Close a connection without losing the response: the write side first,
+/// then whatever the client still sends is read and dropped, briefly, so
+/// that closing with unread bytes does not reset the connection before the
+/// client has read the answer.
+fn close(mut stream: TcpStream) {
+    let _ = stream.shutdown(Shutdown::Write);
+    let _ = stream.set_read_timeout(Some(Duration::from_secs(1)));
+    let mut sink = [0u8; 4096];
+    let mut drained = 0;
+    while drained < (1 << 20) {
+        match stream.read(&mut sink) {
+            Ok(0) | Err(_) => break,
+            Ok(n) => drained += n,
+        }
+    }
+}
+
+/// One Server-Sent Event: its name, then one `data:` line per line of
+/// `data`, then a blank line. Trailing line ends are dropped, and a lone
+/// `\r` ends a line, as the event stream reads it.
+pub fn event(name: &str, data: &str) -> String {
+    let data = data.replace("\r\n", "\n").replace('\r', "\n");
+    let mut out = format!("event: {name}\n");
+    for line in data.trim_end_matches('\n').split('\n') {
+        out.push_str("data: ");
+        out.push_str(line);
+        out.push('\n');
+    }
+    out.push('\n');
+    out
+}
+
+/// The event stream (spec §4.5): the server's start, which each build's id
+/// begins with, the last build's number, its errors when it failed, and the
+/// pages listening.
+pub struct Events {
+    start: u128,
+    state: Mutex<Stream>,
+}
+
+#[derive(Default)]
+struct Stream {
+    /// The number of builds so far: the current build's number.
+    build: u64,
+    /// The last build's errors, when it failed.
+    error: Option<String>,
+    pages: Vec<Box<dyn Write + Send>>,
+}
+
+impl Events {
+    /// The event stream of a server that starts now.
+    pub fn started_now() -> Events {
+        let start = SystemTime::now()
+            .duration_since(UNIX_EPOCH)
+            .map_or(0, |since| since.as_millis());
+        Events::starting_at(start)
+    }
+
+    /// The event stream of a server that started `start` milliseconds after
+    /// the Unix epoch.
+    fn starting_at(start: u128) -> Events {
+        Events {
+            start,
+            state: Mutex::default(),
+        }
+    }
+
+    fn lock(&self) -> MutexGuard<'_, Stream> {
+        self.state.lock().unwrap_or_else(PoisonError::into_inner)
+    }
+
+    /// The current build's id.
+    pub fn id(&self) -> String {
+        let build = self.lock().build;
+        self.id_of(build)
+    }
+
+    /// The id of the build numbered `build`: `<start>.<build>`.
+    fn id_of(&self, build: u64) -> String {
+        format!("{}.{build}", self.start)
+    }
+
+    /// A build has finished: `Ok` sends every page `reload` with the new
+    /// build's id; `Err` sends `error` with the build's errors. Returns the
+    /// new build's id.
+    pub fn built(&self, outcome: Result<(), String>) -> String {
+        let mut stream = self.lock();
+        stream.build += 1;
+        let id = self.id_of(stream.build);
+        let message = match &outcome {
+            Ok(()) => event("reload", &id),
+            Err(errors) => event("error", errors),
+        };
+        stream.error = outcome.err();
+        send(&mut stream.pages, &message);
+        id
+    }
+
+    /// A page joins: the stream's head, `hello` with the current build's
+    /// id, and the last build's errors when it failed. A page whose write
+    /// fails is not kept.
+    fn join(&self, mut page: Box<dyn Write + Send>) {
+        let mut stream = self.lock();
+        let mut message = EVENT_STREAM_HEAD.to_string();
+        message.push_str(&event("hello", &self.id_of(stream.build)));
+        if let Some(errors) = &stream.error {
+            message.push_str(&event("error", errors));
+        }
+        if page
+            .write_all(message.as_bytes())
+            .and_then(|()| page.flush())
+            .is_ok()
+        {
+            stream.pages.push(page);
+        }
+    }
+
+    /// `: ping`, a comment, to every page.
+    pub fn ping(&self) {
+        send(&mut self.lock().pages, ": ping\n\n");
+    }
+}
+
+/// `message` to every page, dropping each whose write fails.
+fn send(pages: &mut Vec<Box<dyn Write + Send>>, message: &str) {
+    pages.retain_mut(|page| {
+        page.write_all(message.as_bytes())
+            .and_then(|()| page.flush())
+            .is_ok()
+    });
+}
+
+#[cfg(test)]
+mod tests {
+    use super::*;
+
+    /// A fresh directory with a fixed name, so each run replaces the last.
+    fn fresh(name: &str) -> PathBuf {
+        let dir = std::env::temp_dir().join(format!("nova-dev-server-{name}"));
+        let _ = std::fs::remove_dir_all(&dir);
+        std::fs::create_dir_all(&dir).expect("create the test directory");
+        dir
+    }
+
+    /// An output directory with a page, a script, a module and a
+    /// subdirectory with a page of its own, and a file beside it.
+    fn dist(name: &str) -> PathBuf {
+        let dir = fresh(name);
+        let dist = dir.join("dist");
+        std::fs::create_dir_all(dist.join("docs")).expect("a directory");
+        std::fs::create_dir_all(dist.join("assets")).expect("a directory");
+        for (path, text) in [
+            ("index.html", "<body>page</body>"),
+            ("app.js", "js"),
+            ("app.wasm", "wasm"),
+            ("docs/index.html", "<p>docs</p>"),
+        ] {
+            std::fs::write(dist.join(path), text).expect("a file");
+        }
+        std::fs::write(dir.join("secret.txt"), "secret").expect("a file");
+        dist
+    }
+
+    fn file(dist: &Path, path: &str) -> Target {
+        let path = dist.join(path).canonicalize().expect("the file exists");
+        Target::File {
+            kind: content_type(&path),
+            path,
+        }
+    }
+
+    #[test]
+    fn a_path_names_its_file_and_a_slash_names_index_html() {
+        let dist = dist("names");
+        assert_eq!(locate(&dist, "/app.js"), file(&dist, "app.js"));
+        assert_eq!(locate(&dist, "/app.wasm?v=2#x"), file(&dist, "app.wasm"));
+        assert_eq!(locate(&dist, "/%61pp.js"), file(&dist, "app.js"));
+        assert_eq!(locate(&dist, "//./app.js"), file(&dist, "app.js"));
+        assert_eq!(locate(&dist, "/"), file(&dist, "index.html"));
+        assert_eq!(locate(&dist, "/docs/"), file(&dist, "docs/index.html"));
+        assert_eq!(locate(&dist, "/index.html"), file(&dist, "index.html"));
+    }
+
+    #[test]
+    fn a_path_that_leaves_the_directory_or_holds_a_backslash_or_nul_is_403() {
+        let dist = dist("refusals");
+        for target in [
+            "/../secret.txt",
+            "/%2e%2e/secret.txt",
+            "/%2E%2E/secret.txt",
+            "/docs/../../secret.txt",
+            "/..",
+            "/../posts",
+            "/%2e%2e/nowhere",
+            "/a\\b",
+            "/%5c..%5csecret.txt",
+            "/app.js%00",
+        ] {
+            assert_eq!(locate(&dist, target), Target::Status(403), "{target}");
+        }
+        // A piece that is not one plain name, which joined to the directory
+        // would replace it.
+        #[cfg(windows)]
+        for target in ["/C:/x", "/C:", "/docs/c:/secret.txt", "/%43:/x"] {
+            assert_eq!(locate(&dist, target), Target::Status(403), "{target}");
+        }
+    }
+
+    #[test]
+    fn a_path_that_does_not_parse_is_400() {
+        let dist = dist("bad-paths");
+        for target in ["app.js", "*", "/%zz", "/%4", "/%+1", "/%ff"] {
+            assert_eq!(locate(&dist, target), Target::Status(400), "{target}");
+        }
+    }
+
+    #[test]
+    fn a_path_with_no_file_and_no_dot_is_the_page_and_any_other_is_404() {
+        let dist = dist("fallback");
+        let page = file(&dist, "index.html");
+        assert_eq!(locate(&dist, "/posts/2"), page);
+        assert_eq!(locate(&dist, "/posts/2/"), page);
+        assert_eq!(locate(&dist, "/posts/2?tab=1"), page);
+        // A directory is never listed: without its `/` it is a path the
+        // router may know; with it, and no `index.html`, too.
+        assert_eq!(locate(&dist, "/assets"), page);
+        assert_eq!(locate(&dist, "/assets/"), page);
+        assert_eq!(locate(&dist, "/missing.js"), Target::Status(404));
+        assert_eq!(locate(&dist, "/posts/2.json"), Target::Status(404));
+        assert_eq!(locate(&dist, "/app.js/"), Target::Status(404));
+    }
+
+    #[test]
+    fn the_servers_own_page_stands_in_for_a_missing_index_html() {
+        let dist = dist("own-page");
+        std::fs::remove_file(dist.join("index.html")).expect("remove the page");
+        assert_eq!(locate(&dist, "/"), Target::OwnPage);
+        assert_eq!(locate(&dist, "/posts/2"), Target::OwnPage);
+        assert_eq!(locate(&dist, "/missing.js"), Target::Status(404));
+        let gone = dist.with_file_name("never-built");
+        assert_eq!(locate(&gone, "/"), Target::OwnPage);
+        assert_eq!(locate(&gone, "/app.js"), Target::Status(404));
+    }
+
+    #[test]
+    fn paths_under_nova_are_the_servers_own() {
+        let dist = dist("own-paths");
+        std::fs::create_dir_all(dist.join("__nova")).expect("a directory");
+        std::fs::write(dist.join("__nova").join("reload.js"), "on disk").expect("a file");
+        assert_eq!(locate(&dist, "/__nova/reload.js"), Target::Reload);
+        assert_eq!(locate(&dist, "/__nova/reload.js?x=1"), Target::Reload);
+        assert_eq!(locate(&dist, "/__nova/events"), Target::Events);
+        assert_eq!(locate(&dist, "/__nova/other"), Target::Status(404));
+        assert_eq!(locate(&dist, "/__nova/"), Target::Status(404));
+    }
+
+    #[cfg(unix)]
+    #[test]
+    fn a_link_out_of_the_directory_is_403_and_one_inside_is_served() {
+        let dist = dist("links");
+        let outside = dist.with_file_name("secret.txt");
+        std::os::unix::fs::symlink(&outside, dist.join("leak.txt")).expect("a link");
+        std::os::unix::fs::symlink(dist.parent().unwrap(), dist.join("up")).expect("a link");
+        std::os::unix::fs::symlink(dist.join("app.js"), dist.join("alias.js")).expect("a link");
+        assert_eq!(locate(&dist, "/leak.txt"), Target::Status(403));
+        assert_eq!(locate(&dist, "/up/secret.txt"), Target::Status(403));
+        assert_eq!(locate(&dist, "/up"), Target::Status(403));
+        assert_eq!(locate(&dist, "/alias.js"), file(&dist, "app.js"));
+        // The page itself, a link out: the fallback refuses it too.
+        std::fs::remove_file(dist.join("index.html")).expect("remove the page");
+        std::os::unix::fs::symlink(&outside, dist.join("index.html")).expect("a link");
+        assert_eq!(locate(&dist, "/"), Target::Status(403));
+        assert_eq!(locate(&dist, "/posts/2"), Target::Status(403));
+    }
+
+    #[test]
+    fn content_types_go_by_extension() {
+        for (name, kind) in [
+            ("index.html", "text/html; charset=utf-8"),
+            ("app.js", "text/javascript; charset=utf-8"),
+            ("worker.mjs", "text/javascript; charset=utf-8"),
+            ("app.wasm", "application/wasm"),
+            ("manifest.json", "application/json"),
+            ("style.css", "text/css; charset=utf-8"),
+            ("logo.svg", "image/svg+xml"),
+            ("logo.png", "image/png"),
+            ("photo.jpg", "image/jpeg"),
+            ("anim.gif", "image/gif"),
+            ("favicon.ico", "image/x-icon"),
+            ("notes.txt", "text/plain; charset=utf-8"),
+            ("font.woff2", "font/woff2"),
+            ("LOGO.PNG", "image/png"),
+            ("archive.tar.gz", "application/octet-stream"),
+            ("photo.jpeg", "application/octet-stream"),
+            ("Makefile", "application/octet-stream"),
+        ] {
+            assert_eq!(content_type(Path::new(name)), kind, "{name}");
+        }
+    }
+
+    #[test]
+    fn the_script_goes_before_the_last_body_end_or_at_the_end() {
+        let tag = script_tag("1729238400123.4");
+        assert_eq!(
+            tag,
+            "<script src=\"/__nova/reload.js\" data-build=\"1729238400123.4\"></script>"
+        );
+        let cases = [
+            (
+                "<body><p>x</p></body></html>",
+                format!("<body><p>x</p>{tag}</body></html>"),
+            ),
+            (
+                "<p>\"</body>\"</p></BODY>\n",
+                format!("<p>\"</body>\"</p>{tag}</BODY>\n"),
+            ),
+            ("<p>no end</p>", format!("<p>no end</p>{tag}")),
+            ("", tag.clone()),
+            ("</body>", format!("{tag}</body>")),
+        ];
+        for (page, expected) in cases {
+            assert_eq!(
+                String::from_utf8(inject(page.as_bytes(), "1729238400123.4")).unwrap(),
+                expected,
+                "{page:?}"
+            );
+        }
+    }
+
+    #[test]
+    fn the_host_must_name_the_server_and_its_port() {
+        assert!(host_allowed(Some("127.0.0.1:4321"), 4321));
+        assert!(host_allowed(Some("localhost:4321"), 4321));
+        assert!(host_allowed(Some("LocalHost:4321"), 4321));
+        assert!(host_allowed(Some(" localhost:4321 "), 4321));
+        for host in [
+            None,
+            Some(""),
+            Some("127.0.0.1"),
+            Some("localhost"),
+            Some("127.0.0.1:4322"),
+            Some("evil.example:4321"),
+            Some("localhost.evil.example:4321"),
+            Some("[::1]:4321"),
+        ] {
+            assert!(!host_allowed(host, 4321), "{host:?}");
+        }
+    }
+
+    #[test]
+    fn a_head_parses_into_method_target_and_host() {
+        let head = b"GET /a?b HTTP/1.1\r\nHost: 127.0.0.1:9\r\nAccept: */*\r\n\r\n";
+        assert_eq!(
+            parse_head(head),
+            Some(Request {
+                method: "GET".to_string(),
+                target: "/a?b".to_string(),
+                host: Some("127.0.0.1:9".to_string()),
+            })
+        );
+        let no_host = parse_head(b"HEAD / HTTP/1.0\r\n\r\n").expect("parses");
+        assert_eq!(no_host.host, None);
+        // Lines that end in a bare LF, or in either (RFC 9112 §2.2).
+        for head in [
+            &b"GET /a?b HTTP/1.1\nHost: 127.0.0.1:9\nAccept: */*\n\n"[..],
+            b"GET /a?b HTTP/1.1\r\nHost: 127.0.0.1:9\nAccept: */*\r\n\n",
+            b"GET /a?b HTTP/1.1\nHost: 127.0.0.1:9\r\n\r\n",
+        ] {
+            assert_eq!(
+                parse_head(head),
+                Some(Request {
+                    method: "GET".to_string(),
+                    target: "/a?b".to_string(),
+                    host: Some("127.0.0.1:9".to_string()),
+                }),
+                "{:?}",
+                String::from_utf8_lossy(head)
+            );
+        }
+        for bad in [
+            &b"GET / HTTP/1.1\r\n"[..],
+            b"GET /\r\n\r\n",
+            b"GET / HTTP/2\r\n\r\n",
+            b"GET / HTTP/1.10\r\n\r\n",
+            b"GET / HTTP/1.\r\n\r\n",
+            b"GET  / HTTP/1.1\r\n\r\n",
+            b"GET / HTTP/1.1 x\r\n\r\n",
+            b"GET / HTTP/1.1\r\nno colon\r\n\r\n",
+            b"GET / HTTP/1.1\r\nHost: a:1\r\nhost: b:1\r\n\r\n",
+            b"GET /\xff HTTP/1.1\r\n\r\n",
+            b"GET / HTTP/1.1\n",
+            b"GET / HTTP/1.1\nno colon\n\n",
+            b"GET / HTTP/1.1\r\r\n\r\n",
+        ] {
+            assert_eq!(parse_head(bad), None, "{:?}", String::from_utf8_lossy(bad));
+        }
+    }
+
+    #[test]
+    fn a_head_over_16_kib_is_invalid() {
+        let mut long = b"GET / HTTP/1.1\r\nX: ".to_vec();
+        long.resize(HEAD_LIMIT - 4, b'a');
+        long.extend_from_slice(b"\r\n\r\n");
+        assert!(matches!(read_head(&mut &long[..]), Head::Complete(h) if h.len() == HEAD_LIMIT));
+        let mut longer = b"GET / HTTP/1.1\r\nX: ".to_vec();
+        longer.resize(HEAD_LIMIT - 3, b'a');
+        longer.extend_from_slice(b"\r\n\r\n");
+        assert!(matches!(read_head(&mut &longer[..]), Head::Invalid));
+        assert!(matches!(read_head(&mut &b"GET / HT"[..]), Head::Invalid));
+        assert!(matches!(read_head(&mut &b""[..]), Head::Nothing));
+    }
+
+    /// A head whose lines end in a bare LF ends at its first empty line, as
+    /// one whose lines end in CRLF does, and what follows is not read as it.
+    #[test]
+    fn a_head_ends_at_its_first_empty_line_whatever_its_line_ends() {
+        for (wire, head) in [
+            (
+                &b"GET / HTTP/1.1\nHost: a:1\n\nbody"[..],
+                &b"GET / HTTP/1.1\nHost: a:1\n\n"[..],
+            ),
+            (
+                b"GET / HTTP/1.1\nHost: a:1\n\r\n\r\n",
+                b"GET / HTTP/1.1\nHost: a:1\n\r\n",
+            ),
+            (
+                b"GET / HTTP/1.1\r\nHost: a:1\r\n\n\n",
+                b"GET / HTTP/1.1\r\nHost: a:1\r\n\n",
+            ),
+            (
+                b"GET / HTTP/1.1\r\n\r\nGET /x HTTP/1.1\n\n",
+                b"GET / HTTP/1.1\r\n\r\n",
+            ),
+        ] {
+            match read_head(&mut &wire[..]) {
+                Head::Complete(read) => {
+                    assert_eq!(read, head, "{:?}", String::from_utf8_lossy(wire))
+                }
+                _ => panic!("no head in {:?}", String::from_utf8_lossy(wire)),
+            }
+        }
+        // Lines that end in a lone CR do not end the head.
+        assert!(matches!(
+            read_head(&mut &b"GET / HTTP/1.1\r\r"[..]),
+            Head::Invalid
+        ));
+        let mut long = b"GET / HTTP/1.1\nX: ".to_vec();
+        long.resize(HEAD_LIMIT - 2, b'a');
+        long.extend_from_slice(b"\n\n");
+        assert!(matches!(read_head(&mut &long[..]), Head::Complete(h) if h.len() == HEAD_LIMIT));
+        long.insert(20, b'a');
+        assert!(matches!(read_head(&mut &long[..]), Head::Invalid));
+    }
+
+    /// A server started at 1000 ms after the epoch, after its first build.
+    fn server(dist: PathBuf) -> Server {
+        let events = Events::starting_at(1000);
+        events.built(Ok(()));
+        Server {
+            port: 4321,
+            dist,
+            events: Arc::new(events),
+        }
+    }
+
+    /// The build id in a page's script tag.
+    fn tag_id(answer: Answer) -> String {
+        let Answer::Body { body, .. } = answer else {
+            panic!("a page, not {answer:?}");
+        };
+        let body = String::from_utf8(body).unwrap();
+        let start = body.find("data-build=\"").expect("a build id") + "data-build=\"".len();
+        body[start..start + body[start..].find('"').unwrap()].to_string()
+    }
+
+    fn get(method: &str, target: &str, host: &str) -> Request {
+        Request {
+            method: method.to_string(),
+            target: target.to_string(),
+            host: Some(host.to_string()),
+        }
+    }
+
+    #[test]
+    fn the_answer_checks_the_host_then_the_method_then_the_path() {
+        let server = server(dist("answers"));
+        let host = "127.0.0.1:4321";
+        assert_eq!(
+            server.answer(&get("GET", "/", "evil.example:4321")),
+            Answer::Status(403)
+        );
+        assert_eq!(
+            server.answer(&get("POST", "/", "evil.example:4321")),
+            Answer::Status(403)
+        );
+        assert_eq!(server.answer(&get("POST", "/", host)), Answer::Status(405));
+        assert_eq!(server.answer(&get("PUT", "/x", host)), Answer::Status(405));
+        assert_eq!(
+            server.answer(&get("GET", "/", host)),
+            Answer::Body {
+                kind: HTML,
+                body: format!("<body>page{}</body>", script_tag("1000.1")).into_bytes(),
+            }
+        );
+        assert_eq!(
+            server.answer(&get("HEAD", "/app.js", host)),
+            Answer::Body {
+                kind: JS,
+                body: b"js".to_vec(),
+            }
+        );
+        assert_eq!(
+            server.answer(&get("GET", "/__nova/reload.js", host)),
+            Answer::Body {
+                kind: JS,
+                body: RELOAD_JS.as_bytes().to_vec(),
+            }
+        );
+        assert_eq!(
+            server.answer(&get("GET", "/__nova/events", host)),
+            Answer::Events
+        );
+        std::fs::remove_file(server.dist.join("index.html")).expect("remove the page");
+        let Answer::Body { body, .. } = server.answer(&get("GET", "/", host)) else {
+            panic!("the server's own page");
+        };
+        let body = String::from_utf8(body).unwrap();
+        assert!(
+            body.contains(&format!("{}</body>", script_tag("1000.1"))),
+            "{body}"
+        );
+    }
+
+    /// The race and the restart (spec §4.5): a page served before a build
+    /// it then misses hears a `hello` that is not its tag's id, which
+    /// reloads it; a page served after a failed build carries that build's
+    /// id, so its `hello` matches and it does not reload, then hears the
+    /// errors; and a server started later gives other ids.
+    #[test]
+    fn a_page_reloads_on_a_hello_that_is_not_its_tags_id() {
+        let server = server(dist("ids"));
+        let host = "127.0.0.1:4321";
+        let served = tag_id(server.answer(&get("GET", "/", host)));
+        assert_eq!(served, "1000.1");
+        assert_eq!(server.events.built(Ok(())), "1000.2");
+        let missed = page(false);
+        server.events.join(Box::new(missed.clone()));
+        let seen = missed.seen.lock().unwrap().clone();
+        assert_eq!(
+            seen,
+            format!("{EVENT_STREAM_HEAD}event: hello\ndata: 1000.2\n\n")
+        );
+        assert!(!seen.contains(&format!("data: {served}\n")), "{seen}");
+
+        assert_eq!(server.events.built(Err("bad".to_string())), "1000.3");
+        let after_failure = tag_id(server.answer(&get("GET", "/posts/2", host)));
+        assert_eq!(after_failure, "1000.3");
+        let failed = page(false);
+        server.events.join(Box::new(failed.clone()));
+        let seen = failed.seen.lock().unwrap().clone();
+        assert_eq!(
+            seen,
+            format!(
+                "{EVENT_STREAM_HEAD}event: hello\ndata: {after_failure}\n\n\
+                 event: error\ndata: bad\n\n"
+            )
+        );
+
+        let restarted = Events::starting_at(2000);
+        restarted.built(Ok(()));
+        assert_eq!(restarted.id(), "2000.1");
+        let now = Events::started_now();
+        assert!(
+            now.id().ends_with(".0") && now.id().len() > 3,
+            "{}",
+            now.id()
+        );
+    }
+
+    #[test]
+    fn a_response_has_its_length_no_store_and_no_body_for_head() {
+        let answer = Answer::Body {
+            kind: HTML,
+            body: b"<p>hi</p>".to_vec(),
+        };
+        let mut wire = Vec::new();
+        write_answer(&mut wire, &answer, false).unwrap();
+        let wire = String::from_utf8(wire).unwrap();
+        assert!(wire.starts_with("HTTP/1.1 200 OK\r\n"), "{wire}");
+        assert!(wire.contains("\r\nContent-Length: 9\r\n"), "{wire}");
+        assert!(wire.contains("\r\nCache-Control: no-store\r\n"), "{wire}");
+        assert!(wire.ends_with("\r\n\r\n<p>hi</p>"), "{wire}");
+        let mut head = Vec::new();
+        write_answer(&mut head, &answer, true).unwrap();
+        let head = String::from_utf8(head).unwrap();
+        assert!(head.contains("\r\nContent-Length: 9\r\n"), "{head}");
+        assert!(head.ends_with("\r\n\r\n"), "{head}");
+        let mut refused = Vec::new();
+        write_answer(&mut refused, &Answer::Status(405), false).unwrap();
+        let refused = String::from_utf8(refused).unwrap();
+        assert!(refused.starts_with("HTTP/1.1 405 Method Not Allowed\r\n"));
+        assert!(refused.contains("\r\nAllow: GET, HEAD\r\n"), "{refused}");
+    }
+
+    /// A `HEAD` of the event stream gets the stream's head, which says the
+    /// connection closes, since it then does, and no event.
+    #[test]
+    fn a_head_of_the_event_stream_says_its_connection_closes() {
+        let mut wire = Vec::new();
+        write_answer(&mut wire, &Answer::Events, true).unwrap();
+        let wire = String::from_utf8(wire).unwrap();
+        assert_eq!(
+            wire,
+            "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\n\
+             Cache-Control: no-store\r\nConnection: close\r\n\r\n"
+        );
+        assert!(!wire.contains("keep-alive"), "{wire}");
+    }
+
+    #[test]
+    fn an_errors_lines_are_one_data_line_each() {
+        assert_eq!(event("reload", "2"), "event: reload\ndata: 2\n\n");
+        assert_eq!(
+            event(
+                "error",
+                "error[E0010]: no\n   ┌─ src/main.nova:1:2\r\n\nlast\n\n"
+            ),
+            "event: error\ndata: error[E0010]: no\ndata:    ┌─ src/main.nova:1:2\n\
+             data: \ndata: last\n\n"
+        );
+        assert_eq!(event("error", "a\rb"), "event: error\ndata: a\ndata: b\n\n");
+    }
+
+    /// A page that records what it is sent, or fails every write.
+    #[derive(Clone)]
+    struct Page {
+        seen: Arc<Mutex<String>>,
+        fails: bool,
+    }
+
+    impl Write for Page {
+        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
+            if self.fails {
+                return Err(std::io::ErrorKind::BrokenPipe.into());
+            }
+            self.seen
+                .lock()
+                .unwrap()
+                .push_str(&String::from_utf8_lossy(buf));
+            Ok(buf.len())
+        }
+        fn flush(&mut self) -> std::io::Result<()> {
+            Ok(())
+        }
+    }
+
+    fn page(fails: bool) -> Page {
+        Page {
+            seen: Arc::new(Mutex::new(String::new())),
+            fails,
+        }
+    }
+
+    #[test]
+    fn a_page_hears_hello_then_reloads_and_errors_and_a_failed_page_is_dropped() {
+        let events = Events::starting_at(7);
+        assert_eq!(events.built(Ok(())), "7.1");
+        let first = page(false);
+        events.join(Box::new(first.clone()));
+        events.join(Box::new(page(true)));
+        assert_eq!(events.lock().pages.len(), 1);
+        assert_eq!(events.built(Err("bad\nworse".to_string())), "7.2");
+        // A page that joins after a failed build hears its errors at once.
+        let late = page(false);
+        events.join(Box::new(late.clone()));
+        events.ping();
+        assert_eq!(events.built(Ok(())), "7.3");
+        let seen = first.seen.lock().unwrap().clone();
+        assert_eq!(
+            seen,
+            format!(
+                "{EVENT_STREAM_HEAD}event: hello\ndata: 7.1\n\n\
+                 event: error\ndata: bad\ndata: worse\n\n: ping\n\n\
+                 event: reload\ndata: 7.3\n\n"
+            )
+        );
+        let seen = late.seen.lock().unwrap().clone();
+        assert_eq!(
+            seen,
+            format!(
+                "{EVENT_STREAM_HEAD}event: hello\ndata: 7.2\n\n\
+                 event: error\ndata: bad\ndata: worse\n\n: ping\n\n\
+                 event: reload\ndata: 7.3\n\n"
+            )
+        );
+        // After a good build, a page that joins hears `hello` alone.
+        let after = page(false);
+        events.join(Box::new(after.clone()));
+        let seen = after.seen.lock().unwrap().clone();
+        assert_eq!(
+            seen,
+            format!("{EVENT_STREAM_HEAD}event: hello\ndata: 7.3\n\n")
+        );
+    }
+
+    #[test]
+    fn a_page_whose_write_fails_later_is_dropped() {
+        let events = Events::starting_at(7);
+        events.built(Ok(()));
+        let shared = Arc::new(Mutex::new(String::new()));
+        events.join(Box::new(Page {
+            seen: Arc::clone(&shared),
+            fails: false,
+        }));
+        assert_eq!(events.lock().pages.len(), 1);
+        // The page goes away: swap it for one that fails.
+        events.lock().pages[0] = Box::new(page(true));
+        events.ping();
+        assert_eq!(events.lock().pages.len(), 0);
+    }
+}
diff --git a/crates/nova-cli/src/main.rs b/crates/nova-cli/src/main.rs
index 270807f5c709fd97846768a67bfb99b3e7777a7a..61c819326fb5651b53554c520190b6ca831456b7 100644
--- a/crates/nova-cli/src/main.rs
+++ b/crates/nova-cli/src/main.rs
@@ -1,7 +1,7 @@
 //! The `nova` command-line tool.
 //!
-//! Dispatches to subcommands: parse, run, build, check, test, fmt, doc, lsp,
-//! new, init, fetch, update, package, publish, login and version. Phase 0
+//! Dispatches to subcommands: parse, run, build, dev, check, test, fmt, doc,
+//! lsp, new, init, fetch, update, package, publish, login and version. Phase 0
 //! implemented `nova
 //! parse`, Phase 1 `nova run` (Cranelift JIT) and `nova check`, Phase 3.0
 //! the project commands (spec
@@ -12,11 +12,14 @@
 //! `docs/superpowers/specs/2026-10-08-phase-3-2-lsp-core-design.md`),
 //! Phase 3.3b the package index (spec
 //! `docs/superpowers/specs/2026-10-09-phase-3-3b-index-and-publishing-design.md`),
-//! and Phase 3.5a `nova doc` (spec
-//! `docs/superpowers/specs/2026-10-11-phase-3-5a-the-documentation-site-design.md`).
+//! Phase 3.5a `nova doc` (spec
+//! `docs/superpowers/specs/2026-10-11-phase-3-5a-the-documentation-site-design.md`),
+//! and Phase 4.5 `nova dev` (spec
+//! `docs/superpowers/specs/2026-10-18-phase-4-5-router-and-nova-dev-design.md`).
 
 mod bundle;
 mod cmd;
+mod dev;
 mod embedded;
 mod project;
 mod template;
@@ -46,6 +49,9 @@ enum Command {
     Build(cmd::run::BuildCmd),
     /// Compile a Nova program to a page in dist/: `build --target wasm`.
     Bundle(cmd::run::BundleCmd),
+    /// Build a page into dist/ and serve it on 127.0.0.1, building it again
+    /// and reloading the page when a source changes.
+    Dev(cmd::dev::DevCmd),
     /// Type-check a Nova program without running it.
     Check(cmd::run::CheckCmd),
     /// Compile and run `@test` functions, one process per test.
@@ -107,6 +113,7 @@ fn main() -> Result<()> {
         Command::Run(cmd) => cmd::run::run(cmd),
         Command::Build(cmd) => cmd::run::build(cmd),
         Command::Bundle(cmd) => cmd::run::bundle(cmd),
+        Command::Dev(cmd) => cmd::dev::run(cmd),
         Command::Check(cmd) => cmd::run::check(cmd),
         Command::Test(cmd) => cmd::test::run(cmd),
         // Its exit code says more than success or failure (spec §7.2).
````


### Task 4: The browser tests, CI, and `06-counter-spa` under `nova dev`

Spec §1 (the gate), §5.4, §7, and the user's decisions 8 and 9; decision 22.

**Files:**
- Create:
  - `tests/browser/dev.mjs`, `router.mjs`
  - `tests/browser/router-app/nova.toml`, `src/main.nova`
- Modify: `.github/workflows/ci.yml` (the `browser` job),
  `examples/06-counter-spa/README.md`

- [ ] **Step 1: The tests and CI.** `patch_block t4-code`:
  - **`tests/browser/dev.mjs`,** run as `node tests/browser/dev.mjs <nova>
    <work dir> <dir holding node_modules/playwright>`: it copies
    `examples/06-counter-spa` into the work directory, emptied first,
    starts `nova dev --port 0` there with `NOVA_HOME` beside it, and reads
    the URL it prints. Through Playwright it:
    1. clicks `+` and `-` and reads the count;
    2. edits `src/main.nova` from `Count:` to `Clicks:`, and waits for the
       page to reload showing `Clicks: 0`;
    3. holds the page's event stream back across one more edit, to
       `Taps:`, and waits for the page, served before that build, to hear a
       `hello` that is not its id and reload showing `Taps: 0`;
    4. writes a type error and waits for `#nova-dev-overlay` with the
       checker's message; reloads the page, which carries the failed
       build's id, and sees the overlay again and no second reload;
    5. restores the file, and waits for the overlay to go and the page to
       count again.

    At each step it reads the build id in the page's tag, one number more
    for each build, and it kills `nova dev` at the end, pass or fail.
  - **`tests/browser/router.mjs`,** with the same arguments, on a copy of
    `tests/browser/router-app`: it checks the page is the generated one,
    titled `Nova`; sets a marker on `window` that a full reload would lose;
    clicks a link, and the URL is `/posts/2`, the post shows and the marker
    stays; goes back, and the home page shows; goes forward and reloads at
    `/posts/2`, and the fallback serves the page, which finds `/app.js`,
    the post shows, the marker is gone, and the page keeps build 1's id, so
    it stays; opens `/nowhere`, and the "not found" route shows; and
    follows a link with a query, which `use_query` reads.
  - **`tests/browser/router-app`:** `nova.toml`, `name = "router-app"`, and
    `src/main.nova`, a `HistoryRouter` over `/`, `/posts/:id`, `/search`
    and `*`, each page with the same three links. It has no `index.html`
    of its own: the generated page, which loads `/app.js` by its absolute
    path, is the one a reload at `/posts/2` must find (decision 22).
  - **CI's `browser` job:** one more step, `nova dev and the router in
    Chromium`, which runs both scripts with `target/debug/nova`, work
    directories in `$RUNNER_TEMP`, and the job's Playwright.
  - **`examples/06-counter-spa/README.md`** runs the example with `nova
    dev` (spec §7).
- [ ] **Step 2: Run the `browser` job, in Chromium,** where Task 0 found
  Playwright: its builds and its first three scripts, since the generated
  page changed in Task 2, then the two new ones.

  ```sh
  cargo build --locked -p nova-cli
  git status --short --ignored --untracked-files=all examples tests > "$P/status-45.txt"
  B="$P/browser-45"
  export PLAYWRIGHT_BROWSERS_PATH=/opt/pw-browsers
  target/debug/nova build --target wasm tests/dom/counter.nova -o "$B/counter"
  target/debug/nova build --target wasm tests/dom/ui_counter.nova -o "$B/ui_counter"
  (cd examples/06-counter-spa && ../../target/debug/nova build --target wasm -o "$B/counter-spa")
  node tests/browser/counter.mjs "$B/counter" /opt/node-tools
  node tests/browser/counter.mjs "$B/ui_counter" /opt/node-tools
  node tests/browser/spa.mjs "$B/counter-spa" /opt/node-tools
  node tests/browser/dev.mjs target/debug/nova "$B/dev" /opt/node-tools
  node tests/browser/router.mjs target/debug/nova "$B/router" /opt/node-tools
  git status --short --ignored --untracked-files=all examples tests | diff "$P/status-45.txt" -
  ```

  Expected:
  - each build prints `built <its directory>`;
  - `counter.mjs`, twice: `counter: three clicks, and the count reads 3`,
    from the generated page, which loads `/app.js` from the server's root;
  - `spa.mjs`: `counter-spa: +, +, +, -, and the count reads 2`;
  - `dev.mjs`, in about 5 seconds: `dev: counts at build <start>.1,
    reloads into .2 on an edit, into .3 after a missed one, shows the
    overlay for the type error of .4 and keeps it over a reload, and counts
    again at .5`, where `<start>` is the server's start in milliseconds,
    such as the replay's `1792331707766`;
  - `router.mjs`, in about 2 seconds: `router: a link to /posts/2 without a
    reload, back home, a reload at /posts/2 that stays at build <start>.1,
    /nowhere not found, and a query read`;
  - `diff` prints nothing: the builds write outside the tree, and the two
    new scripts work in copies.
- [ ] **Step 3: The checks,** as the conventions list them, `nova fmt`'s
  now with `tests/browser/router-app`.
- [ ] **Step 4: Commit** `ci: nova dev and the router in Chromium; 06-counter-spa runs under nova dev`.

**Patch `t4-code`:**

<!-- patch t4-code -->

````diff
diff --git a/.github/workflows/ci.yml b/.github/workflows/ci.yml
index 271fb70d94da814fe99436d407a3064a3be4f960..376a9730b5f233169b8274b9f92f2084ff2a6e7c 100644
--- a/.github/workflows/ci.yml
+++ b/.github/workflows/ci.yml
@@ -97,9 +97,14 @@ jobs:
   # clicked in Chromium through Playwright. Both have the ids `#add` and
   # `#count`, so one script clicks either. And 4.4's gate (§9.4):
   # `examples/06-counter-spa`, built from its project with its own page,
-  # which `spa.mjs` clicks by its buttons' text. Playwright, pinned, is installed
-  # here only, into a scratch directory; nothing in `cargo test` or `nova`
-  # installs a package. Ubuntu only: one browser job suffices.
+  # which `spa.mjs` clicks by its buttons' text. And 4.5's gate (spec
+  # docs/superpowers/specs/2026-10-18-phase-4-5-router-and-nova-dev-design.md
+  # §5.4): `dev.mjs` runs `06-counter-spa` under `nova dev`, edits it, breaks
+  # it and fixes it; `router.mjs` drives `tests/browser/router-app`, in
+  # history mode, under `nova dev`. Each copies its project into
+  # `$RUNNER_TEMP` and kills `nova dev` at the end. Playwright, pinned, is
+  # installed here only, into a scratch directory; nothing in `cargo test` or
+  # `nova` installs a package. Ubuntu only: one browser job suffices.
   browser:
     name: Browser (Chromium)
     runs-on: ubuntu-latest
@@ -126,6 +131,10 @@ jobs:
           node tests/browser/counter.mjs "$RUNNER_TEMP/counter" "$RUNNER_TEMP/pw"
           node tests/browser/counter.mjs "$RUNNER_TEMP/ui_counter" "$RUNNER_TEMP/pw"
           node tests/browser/spa.mjs "$RUNNER_TEMP/counter-spa" "$RUNNER_TEMP/pw"
+      - name: nova dev and the router in Chromium
+        run: |
+          node tests/browser/dev.mjs target/debug/nova "$RUNNER_TEMP/dev" "$RUNNER_TEMP/pw"
+          node tests/browser/router.mjs target/debug/nova "$RUNNER_TEMP/router" "$RUNNER_TEMP/pw"
 
   fmt:
     name: Rustfmt
diff --git a/examples/06-counter-spa/README.md b/examples/06-counter-spa/README.md
index 70875337c3277b9a3a9c51b6ce6321ea368c0dff..7245cf6682c4dd808ba580387313d40809d5ff13 100644
--- a/examples/06-counter-spa/README.md
+++ b/examples/06-counter-spa/README.md
@@ -16,12 +16,19 @@ The smallest WASM frontend: a counter written as a component.
 
 ```bash
 cd examples/06-counter-spa
-nova build --target wasm
-cd dist && python3 -m http.server 8000
+nova dev
 ```
 
-Then open `http://localhost:8000/` in a browser. Any static server works,
-as long as it serves `app.wasm` as `application/wasm`.
+`nova dev` builds the page into `dist/`, prints
+`nova dev: serving http://127.0.0.1:3000/`, and serves it there; open that
+URL in a browser, or start it with `--open`. Each time `src/main.nova`
+changes, it builds again and reloads the page; when the build fails, its
+errors show over the page until a good build replaces them. `--port`
+chooses another port, and Ctrl-C stops it.
+
+To build once instead, `nova build --target wasm` writes the same `dist/`,
+which any static server can serve, as long as it serves `app.wasm` as
+`application/wasm`.
 
 ## Expected output
 
@@ -39,3 +46,7 @@ away: three clicks of `+` and one of `-` leave `Count: 2`.
 - `cargo test` clicks it under a fake DOM, and CI's `browser` job in
   Chromium (`tests/browser/spa.mjs`); the 4.4 spec is
   `docs/superpowers/specs/2026-10-17-phase-4-4-component-syntax-design.md`.
+- The same job runs it under `nova dev` (`tests/browser/dev.mjs`): it
+  counts, an edit reloads it, and a type error shows the overlay until the
+  fix; the 4.5 spec is
+  `docs/superpowers/specs/2026-10-18-phase-4-5-router-and-nova-dev-design.md`.
diff --git a/tests/browser/dev.mjs b/tests/browser/dev.mjs
new file mode 100644
index 0000000000000000000000000000000000000000..e0a790ae678d9ab1b2fd543c803a62d176da5073
--- /dev/null
+++ b/tests/browser/dev.mjs
@@ -0,0 +1,222 @@
+// The gate of 4.5 in Chromium (spec
+// docs/superpowers/specs/2026-10-18-phase-4-5-router-and-nova-dev-design.md
+// §5.4), run by CI's `browser` job:
+//
+//     node tests/browser/dev.mjs <nova> <work dir> <dir holding node_modules/playwright>
+//
+// It copies `examples/06-counter-spa` into the work directory, emptied
+// first, and starts `nova dev --port 0` there, with `NOVA_HOME` beside it,
+// reading the URL it prints. Then, through Playwright:
+// 1. it clicks `+` and `-` and reads the count;
+// 2. it edits `src/main.nova` from `Count:` to `Clicks:`, and waits for the
+//    page to reload showing `Clicks: 0`;
+// 3. it holds the page's event stream back across one more edit, from
+//    `Clicks:` to `Taps:`, and waits for the page, served before that
+//    build, to hear a `hello` that is not its id and reload showing
+//    `Taps: 0`;
+// 4. it writes a type error, and waits for `#nova-dev-overlay` with the
+//    checker's message; then it reloads the page, which carries the failed
+//    build's id, and checks the overlay shows again and the page does not
+//    reload once more;
+// 5. it restores the file, and waits for the overlay to go and the page to
+//    count again.
+// At each step it checks the build id in the page's reload script:
+// `<start>.<number>`, the number one more for each build.
+// `nova dev` is killed at the end, pass or fail. Every wait polls, for 30
+// seconds at most. Playwright is the job's own install, as for `counter.mjs`.
+
+import { spawn } from "node:child_process";
+import { cp, mkdir, readFile, rm, writeFile } from "node:fs/promises";
+import { createRequire } from "node:module";
+import { dirname, join, resolve } from "node:path";
+import { createInterface } from "node:readline";
+import { fileURLToPath } from "node:url";
+
+const [nova, work, modules] = process.argv.slice(2).map((p) => resolve(p));
+const { chromium } = createRequire(join(modules, "noop.js"))("playwright");
+const repo = resolve(dirname(fileURLToPath(import.meta.url)), "..", "..");
+const LIMIT = 30000;
+
+// `nova dev --port 0` in `dir`: its URL, its stderr so far, and a kill.
+async function startDev(dir, home) {
+  const child = spawn(nova, ["dev", "--port", "0"], {
+    cwd: dir,
+    env: { ...process.env, NOVA_HOME: home, NO_COLOR: "1" },
+    stdio: ["ignore", "pipe", "pipe"],
+  });
+  let stderr = "";
+  child.stderr.on("data", (chunk) => {
+    stderr += chunk;
+  });
+  const dev = { url: null, stderr: () => stderr, kill: () => child.kill() };
+  try {
+    dev.url = await new Promise((found, failed) => {
+      const timer = setTimeout(() => failed(new Error("no URL within 30 s")), LIMIT);
+      createInterface({ input: child.stdout }).on("line", (line) => {
+        const url = /^nova dev: serving (http:\/\/127\.0\.0\.1:\d+\/)$/.exec(line)?.[1];
+        if (url) {
+          clearTimeout(timer);
+          found(url);
+        }
+      });
+      child.on("exit", (code) => failed(new Error(`nova dev exited with ${code}`)));
+    });
+  } catch (e) {
+    child.kill();
+    throw new Error(`${e.message}; nova dev's stderr:\n${stderr}`);
+  }
+  return dev;
+}
+
+// Polls `test(arg)` in the page until it holds; a reload between two polls
+// is only a poll that fails.
+async function until(page, test, arg, what) {
+  const deadline = Date.now() + LIMIT;
+  for (;;) {
+    try {
+      if (await page.evaluate(test, arg)) return;
+    } catch {
+      // The page is reloading.
+    }
+    if (Date.now() > deadline) throw new Error(`timed out waiting for ${what}`);
+    await new Promise((later) => setTimeout(later, 100));
+  }
+}
+
+const dir = join(work, "counter-spa");
+await rm(work, { recursive: true, force: true });
+await mkdir(work, { recursive: true });
+await cp(join(repo, "examples", "06-counter-spa"), dir, { recursive: true });
+const main = join(dir, "src", "main.nova");
+const source = await readFile(main, "utf8");
+const checkerSays = "text must have type `String`, found `Int`";
+
+const dev = await startDev(dir, join(work, "nova-home"));
+let browser = null;
+try {
+  browser = await chromium.launch();
+  const page = await browser.newPage();
+  const errors = [];
+  page.on("pageerror", (e) => errors.push(String(e)));
+  const reads = (text) =>
+    until(
+      page,
+      (t) => document.querySelector("#app .counter")?.firstChild?.textContent === t,
+      text,
+      `the count to read ${text}`,
+    );
+  const click = (label) => page.locator("#app button", { hasText: label }).click();
+  // The build id the server gave the page's reload script.
+  const servedId = () =>
+    page.evaluate(() => document.querySelector('script[src="/__nova/reload.js"]')?.dataset.build ?? null);
+  let start = null;
+  const expectBuild = async (number) => {
+    const id = await servedId();
+    const halves = /^(\d+)\.(\d+)$/.exec(id ?? "");
+    if (!halves || (start !== null && halves[1] !== start) || halves[2] !== String(number)) {
+      throw new Error(`the page's build id is ${id}, not build ${number} of ${start ?? "this server"}`);
+    }
+    start = halves[1];
+  };
+  // Waits until the server serves its page with build `number`'s id.
+  const served = async (number) => {
+    const deadline = Date.now() + LIMIT;
+    for (;;) {
+      const page = await (await fetch(dev.url)).text();
+      if (/data-build="\d+\.(\d+)"/.exec(page)?.[1] === String(number)) return;
+      if (Date.now() > deadline) throw new Error(`timed out waiting for build ${number}`);
+      await new Promise((later) => setTimeout(later, 100));
+    }
+  };
+  const overlaid = (says) => {
+    const overlay = document.getElementById("nova-dev-overlay");
+    return (
+      overlay !== null &&
+      overlay.textContent.startsWith("nova dev: the build failed") &&
+      overlay.querySelector("pre")?.textContent.includes(says) === true
+    );
+  };
+
+  // 1. The counter counts.
+  await page.goto(dev.url);
+  await reads("Count: 0");
+  await expectBuild(1);
+  for (const label of ["+", "+", "+", "-"]) await click(label);
+  await reads("Count: 2");
+
+  // 2. An edit reloads the page, which starts again at 0 without the
+  // marker this page had.
+  await page.evaluate(() => {
+    window.novaDevMarker = true;
+  });
+  await writeFile(main, source.replace("Count:", "Clicks:"));
+  await reads("Clicks: 0");
+  if (await page.evaluate(() => window.novaDevMarker === true)) {
+    throw new Error("the page shows `Clicks: 0` without a reload");
+  }
+  await expectBuild(2);
+
+  // 3. A page served at build 2 whose event stream connects only after
+  // build 3 hears a `hello` that is not its id, and reloads.
+  let release = null;
+  const held = new Promise((go) => {
+    release = go;
+  });
+  let holding = true;
+  await page.route("**/__nova/events", async (route) => {
+    if (holding) {
+      holding = false;
+      await held;
+    }
+    await route.continue();
+  });
+  await page.reload();
+  await reads("Clicks: 0");
+  await expectBuild(2);
+  await page.evaluate(() => {
+    window.novaDevMarker = true;
+  });
+  await writeFile(main, source.replace("Count:", "Taps:"));
+  await served(3);
+  release();
+  await reads("Taps: 0");
+  if (await page.evaluate(() => window.novaDevMarker === true)) {
+    throw new Error("the page shows `Taps: 0` without a reload");
+  }
+  await expectBuild(3);
+  await page.unroute("**/__nova/events");
+
+  // 4. A type error shows the overlay, with the checker's message as text.
+  await writeFile(main, source.replace('text("Count: ${count.value()}")', "text(1)"));
+  await until(page, overlaid, checkerSays, "the overlay with the checker's message");
+  // A reload now: the page carries the failed build's id, so its `hello`
+  // matches and it stays, with the overlay from the `error` after it.
+  await page.reload();
+  await until(page, overlaid, checkerSays, "the overlay after a reload");
+  await expectBuild(4);
+  await page.evaluate(() => {
+    window.novaDevMarker = true;
+  });
+  await new Promise((later) => setTimeout(later, 1500));
+  if (!(await page.evaluate(() => window.novaDevMarker === true))) {
+    throw new Error("the page served after the failed build reloaded again");
+  }
+
+  // 5. The fix takes the overlay away, and the page counts again.
+  await writeFile(main, source);
+  await until(page, () => document.getElementById("nova-dev-overlay") === null, null, "the overlay to go");
+  await reads("Count: 0");
+  await expectBuild(5);
+  await click("+");
+  await reads("Count: 1");
+  if (errors.length > 0) throw new Error(`the page threw: ${errors.join("; ")}`);
+  console.log(
+    `dev: counts at build ${start}.1, reloads into .2 on an edit, into .3 after a missed one, shows the overlay for the type error of .4 and keeps it over a reload, and counts again at .5`,
+  );
+} catch (e) {
+  console.error(`nova dev's stderr:\n${dev.stderr()}`);
+  throw e;
+} finally {
+  await browser?.close();
+  dev.kill();
+}
diff --git a/tests/browser/router-app/nova.toml b/tests/browser/router-app/nova.toml
new file mode 100644
index 0000000000000000000000000000000000000000..41458755359a2c492d42f7e179bebec7b30100d3
--- /dev/null
+++ b/tests/browser/router-app/nova.toml
@@ -0,0 +1,5 @@
+[package]
+name = "router-app"
+version = "0.1.0"
+edition = "2026"
+description = "A HistoryRouter app that tests/browser/router.mjs drives under nova dev"
diff --git a/tests/browser/router-app/src/main.nova b/tests/browser/router-app/src/main.nova
new file mode 100644
index 0000000000000000000000000000000000000000..b0e14f862bb69ce00ca9a20e488939d9b6453d0d
--- /dev/null
+++ b/tests/browser/router-app/src/main.nova
@@ -0,0 +1,80 @@
+//! The router's browser test (spec
+//! docs/superpowers/specs/2026-10-18-phase-4-5-router-and-nova-dev-design.md
+//! §5.4): a `HistoryRouter` over a home page, a post by its id, a search
+//! page that reads its query, and a page for any other path, with links
+//! between them. `tests/browser/router.mjs` runs it under `nova dev`, with
+//! no `index.html` of its own: the generated page loads `/app.js` by its
+//! absolute path, so a reload at `/posts/2` finds it.
+
+import ui
+import ui::html::{div, h1, nav, p, text}
+import ui::router::{HistoryRouter, Route, link, use_params, use_query}
+
+/// The links every page shows.
+fn links() -> View {
+    view {
+        nav {
+            link(id: "home", to: "/") { text("Home") }
+            link(id: "post", to: "/posts/2") { text("Post 2") }
+            link(id: "search", to: "/search?q=nova+router&page=3") { text("Search") }
+        }
+    }
+}
+
+component Home() {
+    view {
+        div {
+            links()
+            h1(id: "title") { text("Home") }
+        }
+    }
+}
+
+component Post() {
+    let id = match use_params().get("id") {
+        Some(id) => id
+        None => "?"
+    }
+    view {
+        div {
+            links()
+            h1(id: "title") { text("Post ${id}") }
+        }
+    }
+}
+
+component Search() {
+    let query = use_query()
+    let words = query.get("q").unwrap_or("")
+    let page = query.get("page").unwrap_or("1")
+    view {
+        div {
+            links()
+            h1(id: "title") { text("Search for ${words}, page ${page}") }
+        }
+    }
+}
+
+component NotFound() {
+    view {
+        div {
+            links()
+            h1(id: "title") { text("Not found") }
+            p { text("No route matches this path.") }
+        }
+    }
+}
+
+fn main() {
+    mount(
+        "#app",
+        || HistoryRouter(
+            [
+                Route { path: "/", component: Home },
+                Route { path: "/posts/:id", component: Post },
+                Route { path: "/search", component: Search },
+                Route { path: "*", component: NotFound },
+            ],
+        ),
+    )
+}
diff --git a/tests/browser/router.mjs b/tests/browser/router.mjs
new file mode 100644
index 0000000000000000000000000000000000000000..72a49308f0f366d27e8b6eb574eeb7543031503e
--- /dev/null
+++ b/tests/browser/router.mjs
@@ -0,0 +1,167 @@
+// The router's browser test (spec
+// docs/superpowers/specs/2026-10-18-phase-4-5-router-and-nova-dev-design.md
+// §5.4, the user's decision 8), run by CI's `browser` job:
+//
+//     node tests/browser/router.mjs <nova> <work dir> <dir holding node_modules/playwright>
+//
+// It copies `tests/browser/router-app`, a `HistoryRouter` over `/`,
+// `/posts/:id`, `/search` and `*`, into the work directory, emptied first,
+// and starts `nova dev --port 0` there, with `NOVA_HOME` beside it, reading
+// the URL it prints. The app has no `index.html` of its own, so the page is
+// the one the compiler generates, titled `Nova`, which loads `/app.js` by
+// its absolute path (the user's decision 9 of 2026-10-18): step 3's reload
+// at `/posts/2` proves it. It sets a marker on `window` that a full reload
+// would lose. Then, through Playwright:
+// 1. it clicks a link: the URL is `/posts/2`, the post shows, and the
+//    marker is still there;
+// 2. it goes back: the home page shows, the marker still there;
+// 3. it goes forward and reloads at `/posts/2`: the fallback serves the
+//    page, and the post shows, the marker gone; the page carries the same
+//    build id, `<start>.1`, in its reload script, so it stays;
+// 4. it opens `/nowhere`: the "not found" route shows;
+// 5. it follows a link with a query: `use_query` reads it.
+// `nova dev` is killed at the end, pass or fail. Every wait polls, for 30
+// seconds at most. Playwright is the job's own install, as for `counter.mjs`.
+
+import { spawn } from "node:child_process";
+import { cp, mkdir, rm } from "node:fs/promises";
+import { createRequire } from "node:module";
+import { dirname, join, resolve } from "node:path";
+import { createInterface } from "node:readline";
+import { fileURLToPath } from "node:url";
+
+const [nova, work, modules] = process.argv.slice(2).map((p) => resolve(p));
+const { chromium } = createRequire(join(modules, "noop.js"))("playwright");
+const repo = resolve(dirname(fileURLToPath(import.meta.url)), "..", "..");
+const LIMIT = 30000;
+
+// `nova dev --port 0` in `dir`: its URL, its stderr so far, and a kill.
+async function startDev(dir, home) {
+  const child = spawn(nova, ["dev", "--port", "0"], {
+    cwd: dir,
+    env: { ...process.env, NOVA_HOME: home, NO_COLOR: "1" },
+    stdio: ["ignore", "pipe", "pipe"],
+  });
+  let stderr = "";
+  child.stderr.on("data", (chunk) => {
+    stderr += chunk;
+  });
+  const dev = { url: null, stderr: () => stderr, kill: () => child.kill() };
+  try {
+    dev.url = await new Promise((found, failed) => {
+      const timer = setTimeout(() => failed(new Error("no URL within 30 s")), LIMIT);
+      createInterface({ input: child.stdout }).on("line", (line) => {
+        const url = /^nova dev: serving (http:\/\/127\.0\.0\.1:\d+\/)$/.exec(line)?.[1];
+        if (url) {
+          clearTimeout(timer);
+          found(url);
+        }
+      });
+      child.on("exit", (code) => failed(new Error(`nova dev exited with ${code}`)));
+    });
+  } catch (e) {
+    child.kill();
+    throw new Error(`${e.message}; nova dev's stderr:\n${stderr}`);
+  }
+  return dev;
+}
+
+// Polls `test(arg)` in the page until it holds; a reload between two polls
+// is only a poll that fails.
+async function until(page, test, arg, what) {
+  const deadline = Date.now() + LIMIT;
+  for (;;) {
+    try {
+      if (await page.evaluate(test, arg)) return;
+    } catch {
+      // The page is loading.
+    }
+    if (Date.now() > deadline) {
+      const now = await page.evaluate(() => document.getElementById("title")?.textContent).catch(() => "?");
+      throw new Error(`timed out waiting for ${what}; the title reads ${JSON.stringify(now)} at ${page.url()}`);
+    }
+    await new Promise((later) => setTimeout(later, 100));
+  }
+}
+
+const dir = join(work, "router-app");
+await rm(work, { recursive: true, force: true });
+await mkdir(work, { recursive: true });
+await cp(join(repo, "tests", "browser", "router-app"), dir, { recursive: true });
+
+const dev = await startDev(dir, join(work, "nova-home"));
+let browser = null;
+try {
+  browser = await chromium.launch();
+  const page = await browser.newPage();
+  const errors = [];
+  page.on("pageerror", (e) => errors.push(String(e)));
+  const shows = (title) =>
+    until(page, (t) => document.getElementById("title")?.textContent === t, title, `the title ${title}`);
+  const marked = () => page.evaluate(() => window.novaRouterMarker === true);
+  const path = () => new URL(page.url()).pathname + new URL(page.url()).search;
+  const servedId = () =>
+    page.evaluate(() => document.querySelector('script[src="/__nova/reload.js"]')?.dataset.build ?? null);
+  const expectPath = (expected) => {
+    if (path() !== expected) throw new Error(`the URL is ${page.url()}, not ${expected}`);
+  };
+
+  await page.goto(dev.url);
+  await shows("Home");
+  const title = await page.title();
+  if (title !== "Nova") throw new Error(`the page is titled ${JSON.stringify(title)}, not the generated page's "Nova"`);
+  const id = await servedId();
+  if (!/^\d+\.1$/.test(id ?? "")) throw new Error(`the page's build id is ${id}, not the first build's`);
+  await page.evaluate(() => {
+    window.novaRouterMarker = true;
+  });
+
+  // 1. A link's click navigates without a reload.
+  await page.click("#post");
+  await shows("Post 2");
+  expectPath("/posts/2");
+  if (!(await marked())) throw new Error("the link's click reloaded the page");
+
+  // 2. The back button goes home, still without a reload.
+  await page.goBack();
+  await shows("Home");
+  expectPath("/");
+  if (!(await marked())) throw new Error("the back button reloaded the page");
+
+  // 3. A reload at `/posts/2`: the fallback serves the page, and the
+  // router shows the post.
+  await page.goForward();
+  await shows("Post 2");
+  expectPath("/posts/2");
+  await page.reload();
+  await shows("Post 2");
+  expectPath("/posts/2");
+  if (await marked()) throw new Error("the reload kept the marker: the page did not reload");
+  if ((await servedId()) !== id) throw new Error(`the reloaded page's build id is ${await servedId()}, not ${id}`);
+  await page.evaluate(() => {
+    window.novaRouterMarker = true;
+  });
+  await new Promise((later) => setTimeout(later, 1000));
+  if (!(await marked())) throw new Error("the page reloaded again after its hello");
+
+  // 4. A path no route matches but `*`.
+  await page.goto(new URL("/nowhere", dev.url).href);
+  await shows("Not found");
+  expectPath("/nowhere");
+
+  // 5. A link with a query: `use_query` reads it, `+` as a space.
+  await page.click("#search");
+  await shows("Search for nova router, page 3");
+  expectPath("/search?q=nova+router&page=3");
+
+  if (errors.length > 0) throw new Error(`the page threw: ${errors.join("; ")}`);
+  console.log(
+    `router: a link to /posts/2 without a reload, back home, a reload at /posts/2 that stays at build ${id}, /nowhere not found, and a query read`,
+  );
+} catch (e) {
+  console.error(`nova dev's stderr:\n${dev.stderr()}`);
+  throw e;
+} finally {
+  await browser?.close();
+  dev.kill();
+}
````


### Task 5: The records

Spec §7.

**Files:**
- Create: `docs/adr/0044-the-router-and-nova-dev.md`
- Modify:
  - `nova-spec/00-MASTER-SPEC.md` (§1.8's HMR, and the Phase 4 list's
    `nova dev`), `30-FRONTEND.md` (§6, §8, §13), `40-TOOLING.md` (§1.1),
    `60-EXAMPLES.md` (§6)
  - `docs/phase-4-plan.md` (4.5's entry), `CHANGELOG.md`, `ARCHITECTURE.md`

- [ ] **Step 1: The records.** `patch_block t5-code`.
- [ ] **Step 2: Read them against the code.** Each claim a record makes is
  one this plan's code makes true, and each departure ADR 0044 names is one
  of this plan's decisions:
  - the router, paths, the query, the location read back and without a
    fragment, the replaced entry, links, `rooted` and clicks, against
    `std/ui/router/lib.nova` and `std/ui/backend/wasm.nova`;
  - the state, against Task 1's builtins and `std/rt`'s word 112;
  - the generated page, against `shim/index.html`, and `router-app`, which
    has no page of its own;
  - the server's order of checks, its refusals and its answers, against
    `dev/server.rs`'s `handle`, `answer`, `read_head`, `parse_head`,
    `locate` and `write_answer`;
  - the builds, the watcher and the refusals before the bind, against
    `cmd/dev.rs`;
  - the build ids, the events and the overlay, against `Events` and
    `reload.js`;
  - the figures, against the runs: 21 KB, which Task 6 measures by
    `flate2` at level 9, as the test does and `60-EXAMPLES.md` says; the
    port's message; the URL line.

  A record that says what the code does not is fixed here, and the ledger
  says so.
- [ ] **Step 3: Run.** `cargo test -p nova-std`. Expected: `2 passed`.
- [ ] **Step 4: The checks,** as the conventions list them.
- [ ] **Step 5: Commit** `docs: ADR 0044, and the notes for 4.5`.

**Patch `t5-code`:**

<!-- patch t5-code -->

````diff
diff --git a/ARCHITECTURE.md b/ARCHITECTURE.md
index 52779fbcf4c3ab0b78ccac977452e207db706420..cecd2b2f772df6efed2ab63bb93ff3896e2515cb 100644
--- a/ARCHITECTURE.md
+++ b/ARCHITECTURE.md
@@ -33,7 +33,7 @@ nova-mir            → Mid-level IR
 
 | Crate | Purpose |
 |---|---|
-| `nova-cli` | Binary entry point, CLI argument parsing; a WASM build's bundle: the page, `assets/`, `manifest.json` and `wasm-opt` (ADR 0043) |
+| `nova-cli` | Binary entry point, CLI argument parsing; a WASM build's bundle: the page, `assets/`, `manifest.json` and `wasm-opt` (ADR 0043); `nova dev`: its builds in a child process, its watcher, and its server on `std::net` with the event stream that reloads a page or shows its overlay (ADR 0044) |
 | `nova-driver` | Orchestrates the compilation pipeline |
 | `nova-diagnostics` | Shared error reporting infrastructure, and the fixes a diagnostic suggests |
 | `nova-lexer` | Source → tokens (uses `logos`) |
@@ -47,7 +47,7 @@ nova-mir            → Mid-level IR
 | `nova-codegen-llvm` | Optimized release backend |
 | `nova-codegen-wasm` | WebAssembly backend: MIR to one module with `wasm-encoder`, structured control flow, and the JS shim (ADR 0039) |
 | `nova-runtime` | GC, async runtime, panic handling |
-| `nova-std` (in `std/`) | The standard library's `.nova` sources, embedded for the compiler; `std/rt` is a WASM build's runtime, in Nova: its allocator, collector and executor (ADRs 0039, 0040); `std/dom`, a WASM build's page, which a program imports (ADR 0041); `std/ui` and `std/ui/html`, reactivity and views, which a program imports, and `std/ui`'s backend, one source per target (ADR 0042) |
+| `nova-std` (in `std/`) | The standard library's `.nova` sources, embedded for the compiler; `std/rt` is a WASM build's runtime, in Nova: its allocator, collector and executor (ADRs 0039, 0040); `std/dom`, a WASM build's page, which a program imports (ADR 0041); `std/ui` and `std/ui/html`, reactivity and views, which a program imports, and `std/ui`'s backend, one source per target (ADR 0042); `std/ui/router`, routing by the URL's hash or path, which a program imports (ADR 0044) |
 | `nova-fmt` | The formatter: prints the AST in one fixed layout, keeps every comment, and refuses output that would change the program (ADR 0028); organize imports' block |
 | `nova-lsp` | The language server, `nova lsp`: diagnostics, completion, formatting, hover, definition, references, rename, code actions and semantic tokens over `nova_driver::analyze` (ADR 0029, ADR 0032, ADR 0033) |
 | `nova-test` | A stub, `publish = false`; the test runner is `nova-cli`'s `cmd/test.rs` |
diff --git a/CHANGELOG.md b/CHANGELOG.md
index 8919d98de0fd698291561f97e299ccd9c2fff823..5ce452d88dc13010411903d74c4f516b080008a3 100644
--- a/CHANGELOG.md
+++ b/CHANGELOG.md
@@ -61,9 +61,31 @@ Nova uses [Semantic Versioning](https://semver.org/spec/v2.0.0.html).
 - **`nova build --target wasm` finishes the bundle:** a project's own
   `index.html` is copied into `dist/`, `public/` becomes `assets/`, and
   `manifest.json` lists what was built; `--release` runs `wasm-opt -Oz`
-  when it is on `PATH`. The generated page has `<div id="app"></div>`.
+  when it is on `PATH`. The generated page has `<div id="app"></div>`,
+  and loads `/app.js` by its absolute path.
 - **`examples/06-counter-spa`**, the smallest WASM frontend, 21 KB
   gzipped.
+- **`std/ui/router`:** `Router(routes)` shows the component of the first
+  `Route` whose path matches the URL's hash, `#/posts/1`, and
+  `HistoryRouter(routes)` the one matching its path, `/posts/1`, through
+  the history API. A path holds `:name` parameters, which `use_params()`
+  reads, and the route `*` matches every location; `use_query()` reads the
+  query, and `navigate(to)` goes to a path. `link(to: "/posts/1") { … }` is
+  an `<a>` whose plain click goes to its route without loading the page; a
+  click with a modifier key or another button, or on a link with a
+  `target`, is the browser's. A program imports it with `import
+  ui::router`; natively the location is `/` (ADR 0044).
+- **`nova dev`** builds a program into `dist/`, as `nova build --target
+  wasm` does, and serves it on `127.0.0.1`, on port 3000 or the one
+  `--port` names, printing its URL; `--open` opens a browser. When a source
+  changes, it builds again and reloads the page, or shows the build's
+  errors over it until a good build. A path with no file and no `.` in its
+  last piece gets the page, so a `HistoryRouter` page can be reloaded,
+  with the generated page or a page of the project's own that loads
+  `/app.js` by its absolute path; a path that leaves `dist/`, or a request
+  whose `Host` is not the server's, is refused (ADR 0044).
+- **`Event.modified()`** in `std/dom`: whether Ctrl, Meta, Shift or Alt was
+  down, or a mouse button other than the main one was pressed.
 
 ### Changed
 
@@ -76,6 +98,8 @@ Nova uses [Semantic Versioning](https://semver.org/spec/v2.0.0.html).
 - **`nova build --target wasm --release` builds** where it was refused.
 - **A record named `view` can no longer be written as a literal**, as
   `view {` where an expression starts begins a view (ADR 0043).
+- **`examples/06-counter-spa`'s README runs it with `nova dev`**, where it
+  built it and served `dist/` with another server.
 
 ## [0.3.2] - 2026-10-14
 
diff --git a/docs/adr/0044-the-router-and-nova-dev.md b/docs/adr/0044-the-router-and-nova-dev.md
new file mode 100644
index 0000000000000000000000000000000000000000..21e221824814e5f7079f6993963ee2a853854e8f
--- /dev/null
+++ b/docs/adr/0044-the-router-and-nova-dev.md
@@ -0,0 +1,315 @@
+# ADR 0044 — The router and `nova dev`
+
+## Status
+
+Accepted, 2026-10-18 (Phase 4.5, branch
+`claude/loving-archimedes-frtf5q-45`; spec
+`docs/superpowers/specs/2026-10-18-phase-4-5-router-and-nova-dev-design.md`).
+`30-FRONTEND.md` §6's names, `Router(routes)` and `Route`; `:name`
+parameters and the route `*`; `Router` by the hash and `HistoryRouter` by
+the history API; the URL printed and a browser opened only with `--open`;
+port 3000, changed by `--port`; builds into `dist/`; each rebuild a child
+process; and a browser test of the router are the user's decisions of
+2026-10-18. That the generated page loads `/app.js` by its absolute path is
+the user's decision 9, of the same day, made after the prototype's
+fact-check.
+
+## Context
+
+After 4.4 a program could write a page as a component and build it into
+`dist/`, but had no routing and no dev server. `nova-spec/30-FRONTEND.md`
+§6 sketches a router, `component Router(routes: [Route])` with `navigate`,
+`use_params` and `use_query`, "hash-based routing by default; HTML5 history
+API via flag". §8 sketches a dev server: a watcher, an incremental
+recompile, HMR over a WebSocket, an error overlay and source maps, on
+`axum`. ADR 0038 already scoped Phase 4's `nova dev` to `std::net` and a
+reload over Server-Sent Events. The repository 4.5 starts from has:
+- `std/ui`'s state, a private record behind three std-only builtins, and
+  no re-export, so a router can neither share that state nor move into
+  `std/ui`'s own module;
+- handlers that see only the target's `value`, so no handler can prevent a
+  link's default;
+- `std/dom`'s `location`, `history`, window listeners and `Event`, reached
+  by `std/ui` only through its backend, one source per target, which a
+  program cannot import;
+- a view in which a name with a named argument is an element taking
+  `([Attr], [View])` (E0096), so `Router(routes: […])` cannot be written
+  there and `Router([…])` can;
+- a CLI with no HTTP server and no watcher, whose dependencies are clap,
+  anyhow, tracing, semver and toml_edit;
+- and a driver that writes colour into a pipe unless `NO_COLOR` is set.
+
+## Decision
+
+1. **The router is `std/ui/router`,** which a program imports with `import
+   ui::router`, on both targets: `record Route { path, component }`,
+   `Router(routes)`, which routes by the hash (`#/posts/1`),
+   `HistoryRouter(routes)`, which routes by the path (`/posts/1`) through
+   the history API, `navigate`, `use_params`, `use_query` and `link`. Both
+   routers are written with `component`. In hash mode the location is the
+   hash without its `#`, `/` when there is none; in history mode it is the
+   path and the query. Natively it is `/`, and `navigate` sets the router's
+   location alone. The location never holds a `#fragment`: what follows its
+   first `#`, which reaches it in hash mode as a second `#` (through
+   `navigate`, a `link` or a typed hash) and natively through `navigate`,
+   is dropped before the location is set, so before
+   any matching and before the query is read; `/posts/6#sec` shows post `6`
+   and `/q?a=1#x` has `a = 1`.
+2. **Paths.** A location's path, before any `?`, and a pattern are both
+   split at `/`, empty pieces dropped, and each piece percent-decoded as
+   UTF-8, or kept as written when it does not decode. A pattern's piece is
+   a literal, matched with its case, or `:name`, which matches any one piece
+   and records it for `use_params`. A pattern whose only piece is `*`, as
+   `*` and `/*` are, matches every location. The first route that matches
+   is shown, and none shows nothing. The query, after the first `?`, is
+   split at `&`; an empty part is skipped, a part is `key=value` or `key`
+   with the value `""`, `+` is a space before percent-decoding, and a key's
+   last value wins.
+3. **How a router shows a route.** A router is a `memo` of the route that
+   matches its location, a `Signal<String>`, feeding a `for_each` of that
+   one match or none; `for_each` builds the component untracked, and builds
+   it again each time the location is set, even to the same route. So the
+   component's own state starts again, and `use_params`, which reads the
+   parameters while the router builds the component, and `use_query` need
+   not be reactive. `navigate(to)` puts one `/` before `to` (decision 5),
+   sets the hash or pushes the history, then sets the location to the one
+   the page reports afterwards, which `backend_go` returns (natively `to`),
+   less any fragment; it panics with no router. A browser writes the
+   address its own way, `#/posts/café` as `#/posts/caf%C3%A9` and a space
+   as `%20`, so the location read back is the one the `hashchange` that
+   `navigate`'s own hash causes a turn later finds. The `hashchange` or
+   `popstate` listener sets the location only when it differs, so that
+   `hashchange` does not build the route twice, however the browser
+   re-encodes the hash. In history mode a `to` that is the location
+   already, once its fragment is dropped and both are percent-decoded,
+   replaces the current history entry instead of pushing one, so a second
+   click on the same link adds no entry; in hash mode, setting the hash it
+   already has adds none, as a browser does nothing for the same hash.
+4. **Handlers run through `std/ui`'s public API.** Each listener the router
+   adds runs its function as `batch(|| root(|d| { f(); d() }))`: batched,
+   and with no listener, since `root` runs its function with none and its
+   owner, given nothing to own, is disposed at once. `std/ui` does not
+   change, so `import ui` binds no new name.
+5. **Links.** `link(attrs, children)` is a builder of an element's shape,
+   so a view writes `link(to: "/posts/1") { … }`. Its `to`, fixed or
+   reactive, becomes an `href` computed when the link is built, from the
+   router mounted then: `#/posts/1` under `Router`, `/posts/1` under
+   `HistoryRouter`, with one `/` before `to`, and `to` as written with no
+   router. A leading run of `/` and `\` becomes that one `/`, in an `href`
+   and in `navigate` alike, so `link(to: "//posts/8")` is
+   `href="/posts/8"` or `href="#/posts/8"`, never a protocol-relative URL
+   naming another host, which a plain click would also hand to `pushState`,
+   which throws. A menu before `Router(…)` in the same view thus
+   gets the router's form. The `<a>` also gets `data-nova-link`, which
+   `render_to_string` writes as `data-nova-link=""`. A link with no `to`
+   panics. Each router listens for `click` on the window: the nearest `<a>`
+   with `data-nova-link` at or above the target, clicked with the main
+   button and no Ctrl, Meta, Shift or Alt, and with no `target` attribute,
+   has its default prevented and its `href` navigated to, less a leading
+   `#` in hash mode. Only the router mounted last acts on such a click: the
+   backend prevents the default only when the router's function returns
+   true, which it does, navigating, when its router is the one mounted
+   last; so a router still on the page after a later one was disposed,
+   which leaves none mounted last, leaves the click to the browser. Any
+   other click is the browser's. `std/dom` gains `Event.modified()`, which
+   tells such a click apart.
+6. **The router's state is behind three std-only builtins,**
+   `router_state_present`, `_load` and `_store`, made as `ui_state_*` are:
+   natively a second collector root for the thread, on WASM `std/rt`'s
+   state word 112, which the collector marks. It holds the router mounted
+   last, with its mode and location, the parameters of the route being
+   built, and the next router's number. A page has one router: a router
+   mounted later takes over `navigate`, `use_params` and `use_query`. A
+   router's listeners are removed when its owner is disposed, and the state
+   is cleared then unless a later router has taken over. `std/ui`'s backend
+   gains `backend_location`; `backend_go` and `backend_replace`, which
+   return the location the page reports afterwards; `backend_on_location`;
+   and `backend_on_link_click`, whose function returns whether it took the
+   click. Natively they give `/`, go nowhere and return `to`, and listen
+   for nothing.
+7. **`nova dev [FILE] [--port <PORT>] [--open] [-o <DIR>]`.** `FILE`, `-o`
+   and the output directory are `nova build --target wasm`'s, and so are
+   its refusals, made before a port is bound: a library, an output
+   directory that holds the project or lies inside `public/`, and a package
+   in nova's cache. It binds `127.0.0.1:<port>`, 3000 by default, `0`
+   letting the system choose; a port in use prints `nova dev: port <port>
+   is in use; choose another with --port` and exits with status 1. It
+   builds once, prints `nova dev: serving http://127.0.0.1:<port>/` on
+   stdout, and with `--open` runs `xdg-open`, `open` or `cmd /c start`, a
+   failure of which is a note.
+8. **Builds.** Each build runs the running binary as `build --target wasm
+   [FILE] -o <out>`, with `NO_COLOR=1` and `RUST_LIB_BACKTRACE=0`, so an
+   inherited `RUST_BACKTRACE` puts no backtrace of an error into the
+   overlay; a panic keeps its own. A good build prints the child's notes,
+   then `rebuilt in <ms> ms`, the first build included, and sends `reload`;
+   a failed one prints the child's stderr and sends `error` with it, and
+   leaves the previous build in `dist/`. One build runs at a time, and a
+   change seen during one starts another after it. A panic in the compiler
+   cannot stop the server, and the driver does not change. The watcher
+   polls times and sizes every 200 ms: for a project, every `.nova` file
+   under its root, skipping directories named `target` or beginning with
+   `.`, then `nova.toml`, `index.html` and everything under `public/`; for
+   a single file, the file itself and the `.nova` files of its directory,
+   not below it, since its program cannot import a module from a directory
+   below its own (E0900), so `nova dev ~/x.nova` does not read a whole tree
+   every 200 ms. The output directory is never read. A build starts after
+   one more poll that sees nothing new.
+9. **The server** is HTTP/1.1 on `std::net`, a thread per connection, bound
+   to `127.0.0.1`. Every response has `Cache-Control: no-store`, and every
+   one but the event stream's closes its connection. A request is checked
+   in this order:
+   - a head over 16 KiB, one that does not parse or is not UTF-8, and one
+     with two `Host` headers are `400`; a head's lines may end in CRLF or in
+     a bare LF, which RFC 9112 §2.2 lets a recipient take as a line end,
+     and it ends at its first empty line;
+   - no `Host`, or one that is not `127.0.0.1:<port>` or
+     `localhost:<port>`, is `403`, against DNS rebinding, before the method
+     is looked at;
+   - a method other than `GET` and `HEAD` is `405`;
+   - a target not starting with `/`, bad percent-encoding, and a path that
+     does not decode to UTF-8 are `400`;
+   - a `..` piece, a `\` or a NUL, a piece the system does not read as one
+     plain name (such as `C:` on Windows), and a file outside the output
+     directory once links are resolved are `403`.
+
+   A path ending in `/` names that directory's `index.html`; a file is
+   served with its type by extension, `application/wasm` included, and
+   `.mjs` as `text/javascript` as `.js` is; a directory is never listed. A
+   path that names no file, whose last piece has no `.`, gets the output
+   directory's `index.html`, so a `HistoryRouter` page can be reloaded,
+   with the generated page or one that loads `/app.js` by its absolute path
+   (decision 11); a directory without `index.html` falls back the
+   same way, by the last piece of the URL. While the output directory has
+   no `index.html`, the server's own page answers `/` and those paths
+   alike, so the first build's errors show. Any other missing file is
+   `404`. `/__nova/reload.js` and `/__nova/events` are the server's own.
+10. **Reload and the overlay.** Each build's id is `<start>.<number>`: the
+    server's start in milliseconds since the Unix epoch, and the build's
+    number, which every build, good or failed, takes in turn. Every HTML
+    response gets `<script src="/__nova/reload.js" data-build="<id>"></script>`
+    before its last `</body>`, or at its end, with the id current when the
+    page is served, read before the page; `Content-Length` counts it, and
+    the file on disk does not change. `/__nova/events` sends `hello` with
+    the current id on connecting, followed by the last build's `error` when
+    it failed; `reload` with the new id after a good build; `error` with
+    the child's stderr, one `data:` line per line, after a failed one; and
+    `: ping` every 15 seconds. A page whose write fails is dropped. `HEAD
+    /__nova/events` gets the stream's head with `Connection: close`, and its
+    connection closes. The script reloads the page on `reload`, and
+    on a `hello` whose id is not its own tag's, which covers a page that
+    first connects after a newer build than the one it was served with, and
+    every page after a restart; on `error` it shows `<div
+    id="nova-dev-overlay">`, titled `nova dev: the build failed`, the text
+    in a `<pre>` as `textContent`, until a good build reloads the page. A
+    page served after a failed build carries that build's id, so it shows
+    the error without reloading again.
+11. **The generated page loads `/app.js` by its absolute path** (the
+    user's decision 9), as Vite's base `/` does, so a `HistoryRouter` page
+    reloads at any path under `nova dev`, or under any server that serves
+    the output directory from `/`; a relative `./app.js` under `/posts/1`
+    would be `/posts/app.js`. A project's own `index.html` must load
+    `/app.js` the same way for that, and an app deployed under a sub-path
+    writes its own page. `tests/browser/router-app` has no page of its own,
+    so `router.mjs`'s reload at `/posts/2` proves it; `06-counter-spa`
+    keeps its own page. `HistoryRouter`'s doc comment says so.
+
+## What departs from `30-FRONTEND.md`
+
+- **From §8:**
+  - live reload over Server-Sent Events, not HMR over a WebSocket;
+  - `std::net`, not `axum`;
+  - a full build in a child process, not an incremental one;
+  - no source maps;
+  - a browser opened only with `--open`, where `60-EXAMPLES.md` §6's gate
+    has `nova dev` open one.
+- **From §6:**
+  - `HistoryRouter` as the flag, a second name beside `Router`;
+  - `link`, a builder with a `to` attribute, for the links;
+  - `*` and `:name` in a route's path;
+  - the state behind three std-only builtins;
+  - one router per page.
+
+## What departs from the spec, or fills it in
+
+- **The reload script's tag** is `<script src="/__nova/reload.js"
+  data-build="<id>"></script>`, not the spec's tag without an id, and
+  `hello` and `reload` carry the id `<start>.<number>`, not the number. The
+  script compares a `hello` with its own tag's id, not with the first
+  `hello` it heard, a comparison that never reloads a page whose first
+  `hello` already names a newer build than the one it was served with
+  (decision 10).
+- **The router's listeners** are wrapped in `batch` and `root` by the
+  router, through `std/ui`'s public API, rather than run untracked by the
+  backend; the location listener compares before it sets (decisions 3, 4).
+- **A link's `href`** is computed when the link is built, one `/` is put
+  before `to` under a router, a leading run of `/` and `\` collapsed into
+  it, and `render_to_string` writes `data-nova-link=""` (decision 5).
+- **The location** is read back after going: `backend_go` returns the
+  location the page reports, where spec §3.6 has it return nothing, so a
+  hash the browser re-encodes builds its route once; it never holds a
+  `#fragment`; and in history mode a `to` that is the location already
+  replaces the entry through a fifth backend function, `backend_replace`
+  (decisions 1, 3, 6).
+- **`backend_on_link_click`'s function returns whether it took the
+  click,** where spec §3.6 has `fn(String)`, and the default is prevented
+  only then, so a router that is not the one mounted last leaves the click
+  to the browser (decision 5).
+- **Matching:** `/*` is the wildcard as `*` is, patterns are percent-decoded
+  as locations are, empty query parts are skipped, and only the router
+  mounted last acts on a link's click (decisions 2, 5).
+- **Builds:** the child also gets `RUST_LIB_BACKTRACE=0`; a good build
+  prints the child's notes before `rebuilt in`; a single file is watched
+  itself as well as the `.nova` files of its directory, but not those below
+  it, where spec §4.6 applies the project's rules; and `nova dev` refuses a
+  package in nova's cache, as `nova build` does (decisions 7, 8).
+- **The server:** its own page answers history paths too, a directory
+  without `index.html` falls back by the last piece, `HEAD /__nova/events`
+  gets the stream's head with `Connection: close`, `.mjs` is
+  `text/javascript`, a head's lines may end in a bare LF, and the refusals
+  of decision 9 beyond the spec's, with the `Host` check before the method
+  check (decision 9).
+- **The generated page** loads `/app.js`, where 4.0's loaded `./app.js`,
+  so the user's decision 3, a page in history mode reloaded under
+  `nova dev`, holds for it (decision 11).
+
+## Consequences
+
+- **A component's own state starts again** on every change of location,
+  even to the same route with other parameters.
+- **The generated page assumes the app is served from `/`:** it loads
+  `/app.js` by the absolute path, so a page in either mode reloads at any
+  path, and an app deployed under a sub-path writes its own `index.html`.
+- **Each rebuild is a full build** in a fresh process, so a panic in the
+  compiler shows as a failed build and the server keeps serving; an
+  incremental build could later replace the child without changing what a
+  page hears.
+- **Killing `nova dev`** other than by Ctrl-C at a terminal, which reaches
+  the whole process group, such as `kill` or a test's `child.kill()`,
+  leaves a build that is running to finish on its own: the child is not
+  killed with its parent, and writes the output directory and exits.
+- **Watching by polling** misses nothing a time and a size can show; a file
+  system with coarse times could miss an edit that keeps the size within
+  one tick.
+- **A page of another site** can still send requests to `127.0.0.1` without
+  reading the answers; `nova dev` changes nothing on a `GET`.
+- **No crate is added,** and `Cargo.lock` does not change.
+
+## What is left out
+
+- **For the router:** HMR, and state kept across a reload; nested routers
+  and layouts; guards, lazy routes and a base path for history mode;
+  scroll restoration; an accessor for the current location; and rendering
+  a given path natively, which is 4.6's.
+- **For `nova dev`:** serving a native or server-rendered program, which is
+  4.6's and 4.7's; an overlay for a runtime panic; HTTPS, and access from
+  other machines (`--host`); and source maps.
+
+## References
+
+- `docs/phase-4-plan.md` §3, decisions 6, 9, 10 and 12, §4's 4.5 entry,
+  and §5's security rules.
+- The spec named above.
+- ADR 0038, the recorded scope; ADR 0041, `std/dom`; ADR 0042, `std/ui`;
+  ADR 0043, the component syntax and the bundle.
+- `nova-spec/30-FRONTEND.md` §6, §8 and §13; `nova-spec/40-TOOLING.md`
+  §1.1; `nova-spec/60-EXAMPLES.md` §6.
diff --git a/docs/phase-4-plan.md b/docs/phase-4-plan.md
index 38702d5b49837587bcedf074106e6350a9a3d14b..187c10b919043407789c0ca440361ee95440a41d 100644
--- a/docs/phase-4-plan.md
+++ b/docs/phase-4-plan.md
@@ -545,6 +545,21 @@ RFC 0001; ADR 0043).
   - editing its source reloads the page with the change;
   - a type error shows the overlay.
 
+**Amended 2026-10-18 (branch `claude/loving-archimedes-frtf5q-45`):** by the
+user's decisions of 2026-10-18, the router keeps §6's names: `Router(routes)`
+routes by the hash and `HistoryRouter(routes)` by the history API, a path
+holds `:name` parameters, the route `*` matches what no route before it
+does, and `link` makes the links. `nova dev` prints its URL and opens a
+browser only with `--open`; its port is 3000, or `--port`; it builds into
+`dist/`, each rebuild in a child process whose stderr is the overlay's
+text. The router's state is behind three std-only builtins, and a page has
+one router. The gate is `06-counter-spa` under `nova dev` in Chromium, as
+above, and the type error's fix takes the overlay away; a second browser
+test drives a `HistoryRouter` app under `nova dev` through links, the back
+button and a reload
+(`docs/superpowers/specs/2026-10-18-phase-4-5-router-and-nova-dev-design.md`;
+ADR 0044).
+
 ### 4.6 — SSR, hydration and SSG
 - **SSR:** the string backend renders a whole app, escaping all text.
 - **Hydration**, and `nova build --ssr` and `--ssg` (decision 10).
diff --git a/nova-spec/00-MASTER-SPEC.md b/nova-spec/00-MASTER-SPEC.md
index 0fcc43209b33a7117967f09e7c1b37a94448bbaa..1377fa24f001f288d6e73ffb6886800cca63dd89 100644
--- a/nova-spec/00-MASTER-SPEC.md
+++ b/nova-spec/00-MASTER-SPEC.md
@@ -84,6 +84,9 @@ These are NOT up for debate. If a tradeoff appears mid-implementation, prefer th
 - **SSR/SSG:** built-in flags on `nova build`
 - **Bundler:** built-in (`nova bundle`)
 - **HMR:** built-in (`nova dev`)
+  **Amended 2026-10-18 (branch `claude/loving-archimedes-frtf5q-45`):**
+  `nova dev` reloads the page on a change, live reload with no HMR, by the
+  user's decision of 2026-10-14 (ADRs 0038 and 0044).
 
 ### 1.9 Versioning & Stability
 - **Pre-1.0:** breaking changes allowed in minor versions, RFC required
@@ -674,6 +677,10 @@ Implement std modules in order (each module is a doc in [20-STDLIB.md]):
 5. `std/ui/html/` — element builders
 6. `std/ui/router/` — client router
 7. `nova dev` — dev server with HMR
+   **Amended 2026-10-18 (branch `claude/loving-archimedes-frtf5q-45`):**
+   live reload, no HMR: a change rebuilds the program and reloads the page,
+   whose state starts again, by the user's decision of 2026-10-14 (ADRs
+   0038 and 0044).
 8. SSR: render to string in native runtime, hydrate in WASM
 9. SSG: compile-time route enumeration
 
diff --git a/nova-spec/30-FRONTEND.md b/nova-spec/30-FRONTEND.md
index e2c8d92821b0c2a65d1dcb28c226effc6da6e62d..95682c7478dce8484ebf5dfd6b7baf84323d92b8 100644
--- a/nova-spec/30-FRONTEND.md
+++ b/nova-spec/30-FRONTEND.md
@@ -245,6 +245,33 @@ pub fn use_query() -> Map<String, String>
 
 Hash-based routing by default; HTML5 history API via flag.
 
+**Amended 2026-10-18 (branch `claude/loving-archimedes-frtf5q-45`):** 4.5
+builds `std/ui/router`, which a program imports with `import ui::router`,
+on both targets. The flag is a second name: `Router(routes)` routes by the
+hash, `#/posts/1`, and `HistoryRouter(routes)` by the path, `/posts/1`,
+through the history API; natively the location is `/`. A route's path is
+split at `/` into literals, matched with their case, and `:name`
+parameters, which `use_params()` reads; a path whose only piece is `*`
+matches every location; patterns and locations are percent-decoded alike,
+and the first route that matches is shown. A router is a `memo` of the
+matched route feeding a `for_each`, which builds the component untracked,
+again on every change of location, so `use_params` and `use_query` are not
+reactive. `navigate(to)` sets the hash or pushes the history, then the
+location to the one the page reports, which never holds a `#fragment`, and
+panics with no router; a link to the current location adds no history
+entry. Links are `link(to: "/posts/1") { … }`, a builder whose `to` becomes
+an `href` (`#/posts/1`, `/posts/1`, a leading run of `/` and `\` as one
+`/`, or `to` as written with no router) beside `data-nova-link`; each
+router listens for clicks on the window, and the one mounted last
+navigates on a plain one, which `std/dom`'s new `Event.modified()` tells
+apart, leaving any other to the browser. The router's state is behind
+three std-only builtins, `router_state_present`, `_load` and `_store`, and
+a page has one router, the one mounted last. The router's listeners run
+batched and untracked through `std/ui`'s public API, and `std/ui` does not
+change
+(`docs/superpowers/specs/2026-10-18-phase-4-5-router-and-nova-dev-design.md`
+§3; ADR 0044).
+
 ---
 
 ## 7. Bundler (`nova-bundler`)
@@ -350,6 +377,26 @@ Hydration: client downloads WASM, attaches to existing DOM, replays effects.
 
 Built using `axum` (Rust) for the dev server backend.
 
+**Amended 2026-10-18 (branch `claude/loving-archimedes-frtf5q-45`):** 4.5
+builds `nova dev [FILE] [--port <PORT>] [--open] [-o <DIR>]`, as ADR 0038
+scoped it, and it is the `nova dev` of §7's list. It builds into `dist/` as
+`nova build --target wasm` does, each build a full one in a child process
+with `NO_COLOR=1`, whose stderr is the overlay's text; it polls the
+sources' times and sizes every 200 ms; and it serves on `127.0.0.1`, port
+3000 by default, from an HTTP/1.1 server on `std::net` with a thread per
+connection, not `axum`. A good build reloads the page through Server-Sent
+Events, `/__nova/events`, which a script the server adds to each page
+listens to; a failed one shows `<div id="nova-dev-overlay">` over the page
+until a good build. There is no HMR, so the page's state starts again on
+each reload; there are no source maps; and a browser opens only with
+`--open`. A path with no file whose last piece has no `.` is answered with
+the page, so a `HistoryRouter` page can be reloaded, with the generated
+page, which loads `/app.js` by its absolute path, or a page of the
+project's own that does the same; a path that leaves
+`dist/` is `403`, and so is a request whose `Host` is not the server's own
+(`docs/superpowers/specs/2026-10-18-phase-4-5-router-and-nova-dev-design.md`
+§4; ADR 0044).
+
 ---
 
 ## 9. JS Interop
@@ -450,3 +497,19 @@ those that do not panic under each target's collector stress too;
 it is and under stress, and in Chromium by the `browser` job
 (`tests/browser/spa.mjs`); its `app.wasm` and `app.js` are under 50 KB
 gzipped (`docs/superpowers/specs/2026-10-17-phase-4-4-component-syntax-design.md` §9).
+
+**Amended 2026-10-18 (branch `claude/loving-archimedes-frtf5q-45`):** the
+E2E tests run against `nova dev`. CI's `browser` job drives
+`06-counter-spa` under it in Chromium (`tests/browser/dev.mjs`): it counts,
+an edit reloads it, a page that missed a build reloads when it connects, a
+type error shows the overlay, and the fix takes it away. The same job
+drives `tests/browser/router-app`, a `HistoryRouter` app
+(`tests/browser/router.mjs`), through links, the back button, a reload at
+`/posts/2`, a path only `*` matches, and a query.
+`tests/runtime/ui_router.nova` matches paths and queries on both targets;
+`tests/dom/router_hash.nova` and `router_history.nova` route in each mode
+under the fake DOM, as they are and under stress, and `router_two.nova`
+leaves a click to the browser once its router is not the one mounted last;
+and
+`crates/nova-cli/tests/dev_tests.rs` speaks HTTP to `nova dev`
+(`docs/superpowers/specs/2026-10-18-phase-4-5-router-and-nova-dev-design.md` §5).
diff --git a/nova-spec/40-TOOLING.md b/nova-spec/40-TOOLING.md
index 60360cde1f1e77a65fb608ddcba3fd002af8f522..a53472d1398ed2894e0a2c7c1d030b3df919feed 100644
--- a/nova-spec/40-TOOLING.md
+++ b/nova-spec/40-TOOLING.md
@@ -80,6 +80,19 @@ and refuses `--release`
 (`docs/superpowers/specs/2026-10-14-phase-4-0-foundations-design.md` §3.1; ADR
 0039).
 
+**Amended 2026-10-18 (branch `claude/loving-archimedes-frtf5q-45`):** `nova
+dev [FILE] [--port <PORT>] [--open] [-o <DIR>]` exists, without HMR. It
+takes `FILE` and `-o`, and refuses what `nova build --target wasm` refuses;
+builds into `dist/` as that does; prints `nova dev: serving
+http://127.0.0.1:<port>/`, on port 3000 unless `--port` names another, `0`
+letting the system choose; and opens a browser only with `--open`. When a
+source changes, it builds again in a child process and reloads the page
+over Server-Sent Events, or shows the build's errors over the page until a
+good build. A taken port stops it with `nova dev: port <port> is in use;
+choose another with --port` and status 1
+(`docs/superpowers/specs/2026-10-18-phase-4-5-router-and-nova-dev-design.md`
+§4; ADR 0044).
+
 ### 1.2 Project Template (`nova new`)
 
 ```
diff --git a/nova-spec/60-EXAMPLES.md b/nova-spec/60-EXAMPLES.md
index c304985fb01f79662e2681ce58289549b8e5103a..628a79fd6fff8ccd3e19947ef03294772aae5a75 100644
--- a/nova-spec/60-EXAMPLES.md
+++ b/nova-spec/60-EXAMPLES.md
@@ -657,6 +657,18 @@ a `nova.toml` and a README. The gate is `nova build --target wasm` and
 clicks in Chromium, since `nova dev` comes with 4.5; its `app.wasm` and
 `app.js` are 21 KB gzipped (`docs/superpowers/specs/2026-10-17-phase-4-4-component-syntax-design.md` §8; ADR 0043).
 
+**Amended 2026-10-18 (branch `claude/loving-archimedes-frtf5q-45`):** the
+gate is now `nova dev`, as written above, but `nova dev` opens a browser
+only with `--open`. CI's `browser` job starts `nova dev --port 0` in a copy of
+`examples/06-counter-spa` and, through Playwright, clicks `+` and `-`;
+edits `Count:` to `Clicks:` and sees the page reload with the change;
+writes a type error and sees the overlay with the checker's message; and
+restores the file and sees the overlay go and the page count again
+(`tests/browser/dev.mjs`). The README runs the example with `nova dev`. Its
+`app.wasm` and `app.js` are still 21 KB gzipped, by `flate2` at level 9
+as `wasm_tests.rs` measures them
+(`docs/superpowers/specs/2026-10-18-phase-4-5-router-and-nova-dev-design.md` §5.4; ADR 0044).
+
 ---
 
 ## 7. `07-fullstack-blog` (Phase 4 gate)
````


### Task 6: Verification, the mutants, the review, the PR

- [ ] **Step 1: The full checks.** First, `df -h /` must show at least 8G
  free; if not, remove scratch directories of earlier sub-phases.

  ```sh
  cargo fmt --all -- --check
  cargo clippy --locked --all-targets --all-features -- -D warnings
  cargo run --locked -q -p nova-cli -- fmt --check std examples tests/wasm tests/dom tests/browser/router-app
  cargo run --locked -q -p nova-cli -- fmt --check tests/runtime/ui_router.nova tests/runtime/ui_router_no_to.nova
  cargo build --locked --workspace
  NOVA_REQUIRE_NODE=1 cargo test --locked --workspace --no-fail-fast > "$P/full-45x.txt" 2>&1
  grep '^test result' "$P/full-45x.txt" | awk '{p += $4; f += $6; i += $8} END {print p, f, i}'
  grep ' FAILED$' "$P/full-45x.txt"
  W the_counter_spa_is_under_50_kb_gzipped --nocapture
  ```

  Expected:
  - both formatters print nothing, and clippy ends with `Finished`, with
    no warning or error;
  - the suite, in about 3 minutes: `2357 1 10`, against the baseline's
    `2317 1 10`: 40 new tests: 26 in `nova-cli`'s binary (`cmd::dev`'s 6
    and `dev::server`'s 20), 6 in `dev_tests`, 5 in `wasm_tests`
    (`gate::ui_router`, `gate::ui_router_no_to`,
    `router_routes_by_the_hash`, `router_routes_by_the_history_api` and
    `a_router_not_mounted_last_leaves_a_click_to_the_browser`), and one
    each in `lower_tests`, `nova-resolver` and `nova-runtime`. Three of
    them, `a_link_out_of_the_directory_is_403_and_one_inside_is_served`,
    `a_link_to_nothing_is_watched_as_a_file` and
    `a_link_out_of_dist_is_refused`, are Unix's alone;
  - the one failure is Task 0's IPv6 test;
  - the size: `1 passed`, and it prints `06-counter-spa: 21070 bytes
    gzipped`, by `flate2` at level 9, against 51,200. Record it in the
    ledger.
- [ ] **Step 2: The lockfile.** `git diff --numstat 3bcd1c6 -- Cargo.lock`.
  Expected: nothing.
- [ ] **Step 3: Rust 1.78.**

  ```sh
  RUSTFLAGS="-D warnings" CARGO_TARGET_DIR="$P/msrv-45" cargo +1.78 check --locked --workspace
  ```

  Expected: clean, in a minute or two. Then `rm -rf "$P/msrv-45"`.
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
  the embedded std. The server's plain-name check has no mutant of its
  own: on Unix the `..` check before it refuses the only piece it would,
  so it is mutant 61's, with that check, and CI's Windows leg runs it
  through the `#[cfg(windows)]` loop of
  `a_path_that_leaves_the_directory_or_holds_a_backslash_or_nul_is_403`.

  1. **The collector leaves word 112 unmarked.** `std/rt/lib.nova`:
     `if router != 0 { gc_mark(m, router) }`: the line goes.
     - Run `CARGO_INCREMENTAL=0 NOVA_REQUIRE_NODE=1 cargo test -p nova-cli --test wasm_tests router_routes`.
     - Expected: fails: `router_routes_by_the_hash`.
  2. **The router's state shares `std/ui`'s word.** `std/rt/lib.nova`:
     `const ROUTER_STATE: Int = 112` becomes `const ROUTER_STATE: Int = 104`.
     - Run `CARGO_INCREMENTAL=0 NOVA_REQUIRE_NODE=1 cargo test -p nova-cli --test wasm_tests router_routes`.
     - Expected: fails: `router_routes_by_the_hash`, `router_routes_by_the_history_api`.
  3. **The native slot is no collector root.** `crates/nova-runtime/src/ui.rs`:
     `store(&ROUTER_STATE, state);` becomes `ROUTER_STATE.with(|s| s.set(state as usize));`.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-runtime --lib ui::`.
     - Expected: fails: `ui::tests::the_routers_state_is_a_root_of_its_own`.
  4. **`router_state_store` is not std-only.** `crates/nova-resolver/src/lib.rs`:
     replace

     ```
             Builtin::RouterStateStore,
         ];
     ```

     with

     ```
             Builtin::UiStateStore,
         ];
     ```

     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-resolver --lib the_state_builtins_are_std_only`.
     - Expected: fails: `tests::the_state_builtins_are_std_only`.
  5. **`router_state_present` returns `Int`.** `crates/nova-typeck/src/check.rs`:
     `Builtin::RouterStatePresent => (vec![], Ty::Bool),` becomes `Builtin::RouterStatePresent => (vec![], Ty::Int),`.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-typeck --lib builtin_signatures_are_what_the_std_call_sites_use`.
     - Expected: fails: `check::tests::builtin_signatures_are_what_the_std_call_sites_use`.
  6. **`router_state_load` lowers to `std/ui`'s runtime call.** `crates/nova-mir/src/lower.rs`:
     replace

     ```
     Builtin::RouterStateLoad => Lowering::Runtime(RtFunc::RouterStateLoad),
     ```

     with

     ```
     Builtin::RouterStateLoad => Lowering::Runtime(RtFunc::UiStateLoad),
     ```

     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-mir --test lower_tests state_builtins_lower`.
     - Expected: fails: `the_router_state_builtins_lower_to_the_runtime_or_std_rt`.
  7. **Any literal piece matches.** `std/ui/router/lib.nova`:
     `} else if p != pieces[i] {` becomes `} else if false {`.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests ui_programs_run_and_survive_gc_stress`.
     - Expected: fails: `ui_programs_run_and_survive_gc_stress`.
  8. **A parameter is recorded under `:name`.** `std/ui/router/lib.nova`:
     `let _ = params.insert(p.slice(1, p.len()), pieces[i])` becomes `let _ = params.insert(p, pieces[i])`.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests ui_programs_run_and_survive_gc_stress`.
     - Expected: fails: `ui_programs_run_and_survive_gc_stress`.
  9. **A pattern need not have the path's count of pieces.** `std/ui/router/lib.nova`:
     `if pattern.len() != pieces.len() { return None }`: the line goes.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests ui_programs_run_and_survive_gc_stress`.
     - Expected: fails: `ui_programs_run_and_survive_gc_stress`.
  10. **Empty pieces are kept.** `std/ui/router/lib.nova`:
     replace

     ```
             if parts[i] != "" { n = n + 1 }
             i = i + 1
         }
         let mut out = [""; n]
         let mut k = 0
         let mut j = 0
         while j < parts.len() {
             if parts[j] != "" {
     ```

     with

     ```
             if true { n = n + 1 }
             i = i + 1
         }
         let mut out = [""; n]
         let mut k = 0
         let mut j = 0
         while j < parts.len() {
             if true {
     ```

     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests ui_programs_run_and_survive_gc_stress`.
     - Expected: fails: `ui_programs_run_and_survive_gc_stress`.
  11. **`*` is a literal.** `std/ui/router/lib.nova`:
     `if pattern.len() == 1 && pattern[0] == "*" { return Some(params) }`: the line goes.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests ui_programs_run_and_survive_gc_stress`.
     - Expected: fails: `ui_programs_run_and_survive_gc_stress`.
  12. **The last route that matches is shown.** `std/ui/router/lib.nova`:
     `let r = routes[i]` becomes `let r = routes[routes.len() - 1 - i]`.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests ui_programs_run_and_survive_gc_stress`.
     - Expected: fails: `ui_programs_run_and_survive_gc_stress`.
  13. **A path's pieces are not decoded.** `std/ui/router/lib.nova`:
     `out[k] = decoded(parts[j])` becomes `out[k] = parts[j]`.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests ui_programs_run_and_survive_gc_stress`.
     - Expected: fails: `ui_programs_run_and_survive_gc_stress`.
  14. **An upper-case hex digit is off by one.** `std/ui/router/lib.nova`:
     `b - 55` becomes `b - 54`.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests ui_programs_run_and_survive_gc_stress`.
     - Expected: fails: `ui_programs_run_and_survive_gc_stress`.
  15. **A lower-case hex digit is off by one.** `std/ui/router/lib.nova`:
     `b - 87` becomes `b - 86`.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests ui_programs_run_and_survive_gc_stress`.
     - Expected: fails: `ui_programs_run_and_survive_gc_stress`.
  16. **`+` is a space in a path.** `std/ui/router/lib.nova`:
     `out[k] = decoded(parts[j])` becomes `out[k] = query_decoded(parts[j])`.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests ui_programs_run_and_survive_gc_stress`.
     - Expected: fails: `ui_programs_run_and_survive_gc_stress`.
  17. **`+` is not a space in a query.** `std/ui/router/lib.nova`:
     replace

     ```
     fn query_decoded(text: String) -> String { decoded(" ".join(text.split("+"))) }
     ```

     with

     ```
     fn query_decoded(text: String) -> String { decoded(text) }
     ```

     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests ui_programs_run_and_survive_gc_stress`.
     - Expected: fails: `ui_programs_run_and_survive_gc_stress`.
  18. **A query is split at `;`.** `std/ui/router/lib.nova`:
     `let parts = location.slice(at + 1, location.len()).split("&")` becomes `let parts = location.slice(at + 1, location.len()).split(";")`.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests ui_programs_run_and_survive_gc_stress`.
     - Expected: fails: `ui_programs_run_and_survive_gc_stress`.
  19. **An empty part is a key.** `std/ui/router/lib.nova`:
     `if part != "" {` becomes `if true {`.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests ui_programs_run_and_survive_gc_stress`.
     - Expected: fails: `ui_programs_run_and_survive_gc_stress`.
  20. **A key's first value wins.** `std/ui/router/lib.nova`:
     `let part = parts[i]` becomes `let part = parts[parts.len() - 1 - i]`.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests ui_programs_run_and_survive_gc_stress`.
     - Expected: fails: `ui_programs_run_and_survive_gc_stress`.
  21. **`to` gets no leading `/`.** `std/ui/router/lib.nova`:
     `"/${plain.slice(i, cs.len())}"` becomes `plain.slice(i, cs.len())`.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests ui_programs_run_and_survive_gc_stress`.
     - Expected: fails: `ui_programs_run_and_survive_gc_stress`.
  22. **A leading `\` is kept.** `std/ui/router/lib.nova`:
     `while i < cs.len() && (cs[i] == '/' || cs[i] == '\\') { i = i + 1 }` becomes `while i < cs.len() && cs[i] == '/' { i = i + 1 }`.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests ui_programs_run_and_survive_gc_stress`.
     - Expected: fails: `ui_programs_run_and_survive_gc_stress`.
  23. **A leading run of `/` is kept.** `std/ui/router/lib.nova`:
     `if i == 1 && cs[0] == '/' { return plain }` becomes `if i >= 1 && cs[0] == '/' { return plain }`.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests ui_programs_run_and_survive_gc_stress`.
     - Expected: fails: `ui_programs_run_and_survive_gc_stress`.
  24. **`backend_go` returns `to`, not the location read back.** `std/ui/backend/wasm.nova`:
     replace

     ```
         if history { history_push(to) } else { set_location_hash("#${to}") }
         backend_location(history)
     }
     ```

     with

     ```
         if history { history_push(to) } else { set_location_hash("#${to}") }
         to
     }
     ```

     - Run `CARGO_INCREMENTAL=0 NOVA_REQUIRE_NODE=1 cargo test -p nova-cli --test wasm_tests router_routes_by_the_hash`.
     - Expected: fails: `router_routes_by_the_hash`.
  25. **`navigate` sets the location to `to`, not to what the page reports.** `std/ui/router/lib.nova`:
     `a.location.set(unfragmented(at))` becomes `a.location.set(unfragmented(path))`.
     - Run `CARGO_INCREMENTAL=0 NOVA_REQUIRE_NODE=1 cargo test -p nova-cli --test wasm_tests router_routes_by_the_hash`.
     - Expected: fails: `router_routes_by_the_hash`.
  26. **`navigate` keeps a `#fragment` in the location.** `std/ui/router/lib.nova`:
     replace

     ```
         match location.index_of("#") {
             Some(at) => location.slice(0, at)
     ```

     with

     ```
         match location.index_of("#") {
             Some(at) => location
     ```

     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests ui_programs_run_and_survive_gc_stress`.
     - Expected: fails: `ui_programs_run_and_survive_gc_stress`.
  27. **The location read from the page keeps a `#fragment`.** `std/ui/router/lib.nova`:
     replace

     ```
     fn located(history: Bool) -> String { unfragmented(backend_location(history)) }
     ```

     with

     ```
     fn located(history: Bool) -> String { backend_location(history) }
     ```

     - Run `CARGO_INCREMENTAL=0 NOVA_REQUIRE_NODE=1 cargo test -p nova-cli --test wasm_tests router_routes_by_the_hash`.
     - Expected: fails: `router_routes_by_the_hash`.
  28. **Going to the location already pushes another entry.** `std/ui/router/lib.nova`:
     `let at = if a.history && same_location(path, a.location.val) {` becomes `let at = if false {`.
     - Run `CARGO_INCREMENTAL=0 NOVA_REQUIRE_NODE=1 cargo test -p nova-cli --test wasm_tests router_routes_by_the_history_api`.
     - Expected: fails: `router_routes_by_the_history_api`.
  29. **The same location is compared as written, fragment and encoding included.** `std/ui/router/lib.nova`:
     replace

     ```
     fn same_location(path: String, at: String) -> Bool { decoded(unfragmented(path)) == decoded(at) }
     ```

     with

     ```
     fn same_location(path: String, at: String) -> Bool { path == at }
     ```

     - Run `CARGO_INCREMENTAL=0 NOVA_REQUIRE_NODE=1 cargo test -p nova-cli --test wasm_tests router_routes_by_the_history_api`.
     - Expected: fails: `router_routes_by_the_history_api`.
  30. **`backend_replace` pushes.** `std/ui/backend/wasm.nova`:
     replace

     ```
         history_replace(to)
         backend_location(true)
     ```

     with

     ```
         history_push(to)
         backend_location(true)
     ```

     - Run `CARGO_INCREMENTAL=0 NOVA_REQUIRE_NODE=1 cargo test -p nova-cli --test wasm_tests router_routes_by_the_history_api`.
     - Expected: fails: `router_routes_by_the_history_api`.
  31. **Hash mode pushes a history entry.** `std/ui/backend/wasm.nova`:
     `if history { history_push(to) } else { set_location_hash("#${to}") }` becomes `history_push(to)`.
     - Run `CARGO_INCREMENTAL=0 NOVA_REQUIRE_NODE=1 cargo test -p nova-cli --test wasm_tests router_routes_by_the_hash`.
     - Expected: fails: `router_routes_by_the_hash`.
  32. **History mode replaces the entry.** `std/ui/backend/wasm.nova`:
     replace

     ```
         if history { history_push(to) } else { set_location_hash("#${to}") }
     ```

     with

     ```
         if history { history_replace(to) } else { set_location_hash("#${to}") }
     ```

     - Run `CARGO_INCREMENTAL=0 NOVA_REQUIRE_NODE=1 cargo test -p nova-cli --test wasm_tests router_routes_by_the_history_api`.
     - Expected: fails: `router_routes_by_the_history_api`.
  33. **`navigate` with no router does nothing.** `std/ui/router/lib.nova`:
     `None => panic("navigate: no router is mounted")` becomes `None => return`.
     - Run `CARGO_INCREMENTAL=0 NOVA_REQUIRE_NODE=1 cargo test -p nova-cli --test wasm_tests router_routes_by_the_history_api`.
     - Expected: fails: `router_routes_by_the_history_api`.
  34. **A `hashchange` to the same location sets it again.** `std/ui/router/lib.nova`:
     `if at != location.val { location.set(at) }` becomes `location.set(at)`.
     - Run `CARGO_INCREMENTAL=0 NOVA_REQUIRE_NODE=1 cargo test -p nova-cli --test wasm_tests router_routes_by_the_hash`.
     - Expected: fails: `router_routes_by_the_hash`.
  35. **A link's `href` is fixed when `link` is called.** `std/ui/router/lib.nova`:
     `Dynamic("href", || href_of(to))` becomes `Static("href", href_of(to))`.
     - Run `CARGO_INCREMENTAL=0 NOVA_REQUIRE_NODE=1 cargo test -p nova-cli --test wasm_tests router_routes_by_the_hash`.
     - Expected: fails: `router_routes_by_the_hash`.
  36. **Hash mode's `href` has no `#`.** `std/ui/router/lib.nova`:
     `Some(a) => if a.history { rooted(to) } else { "#${rooted(to)}" }` becomes `Some(a) => rooted(to)`.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests ui_programs_run_and_survive_gc_stress`.
     - Expected: fails: `ui_programs_run_and_survive_gc_stress`.
  37. **History mode's `href` has a `#`.** `std/ui/router/lib.nova`:
     `Some(a) => if a.history { rooted(to) } else { "#${rooted(to)}" }` becomes `Some(a) => "#${rooted(to)}"`.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests ui_programs_run_and_survive_gc_stress`.
     - Expected: fails: `ui_programs_run_and_survive_gc_stress`.
  38. **A link has no `data-nova-link`.** `std/ui/router/lib.nova`:
     `let mut out = [Static("data-nova-link", ""); attrs.len() + 1]` becomes `let mut out = [Static("data-link", ""); attrs.len() + 1]`.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests ui_programs_run_and_survive_gc_stress`.
     - Expected: fails: `ui_programs_run_and_survive_gc_stress`.
  39. **A link drops its other fixed attributes.** `std/ui/router/lib.nova`:
     replace

     ```
                         Dynamic("href", || href_of(to))
                     } else {
                         a
                     }
     ```

     with

     ```
                         Dynamic("href", || href_of(to))
                     } else {
                         Static("data-nova-link", "")
                     }
     ```

     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests ui_programs_run_and_survive_gc_stress`.
     - Expected: fails: `ui_programs_run_and_survive_gc_stress`.
  40. **A link with no `to` renders.** `std/ui/router/lib.nova`:
     `` if !found { panic("link: no `to` attribute") } ``: the line goes.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests ui_programs_panic_with_their_message`.
     - Expected: fails: `ui_programs_panic_with_their_message`.
  41. **The listener looks at the target alone.** `std/ui/backend/wasm.nova`:
     `node = n.parent()` becomes `done = true`.
     - Run `CARGO_INCREMENTAL=0 NOVA_REQUIRE_NODE=1 cargo test -p nova-cli --test wasm_tests router_routes_by_the_history_api`.
     - Expected: fails: `router_routes_by_the_history_api`.
  42. **Any `<a>` is intercepted.** `std/ui/backend/wasm.nova`:
     `if n.tag() == "a" && n.attribute("data-nova-link").is_some() {` becomes `if n.tag() == "a" {`.
     - Run `CARGO_INCREMENTAL=0 NOVA_REQUIRE_NODE=1 cargo test -p nova-cli --test wasm_tests router_routes_by_the_history_api`.
     - Expected: fails: `router_routes_by_the_history_api`.
  43. **A modified click is intercepted.** `std/ui/backend/wasm.nova`:
     `if !e.modified() && a.attribute("target").is_none() && take() {` becomes `if a.attribute("target").is_none() && take() {`.
     - Run `CARGO_INCREMENTAL=0 NOVA_REQUIRE_NODE=1 cargo test -p nova-cli --test wasm_tests router_routes_by_the_history_api`.
     - Expected: fails: `router_routes_by_the_history_api`.
  44. **A link with `target` is intercepted.** `std/ui/backend/wasm.nova`:
     `if !e.modified() && a.attribute("target").is_none() && take() {` becomes `if !e.modified() && take() {`.
     - Run `CARGO_INCREMENTAL=0 NOVA_REQUIRE_NODE=1 cargo test -p nova-cli --test wasm_tests router_routes_by_the_history_api`.
     - Expected: fails: `router_routes_by_the_history_api`.
  45. **Another button is a plain click.** `crates/nova-codegen-wasm/src/shim.rs`:
     `e.altKey || (e.button ?? 0) !== 0 ? 1 : 0;` becomes `e.altKey ? 1 : 0;`.
     - Run `CARGO_INCREMENTAL=0 NOVA_REQUIRE_NODE=1 cargo test -p nova-cli --test wasm_tests router_routes_by_the_history_api`.
     - Expected: fails: `router_routes_by_the_history_api`.
  46. **A router not mounted last takes a link's click.** `std/ui/router/lib.nova`:
     `|| is_active(id),` becomes `|| true,`.
     - Run `CARGO_INCREMENTAL=0 NOVA_REQUIRE_NODE=1 cargo test -p nova-cli --test wasm_tests a_router_not_mounted_last_leaves_a_click_to_the_browser`.
     - Expected: fails: `a_router_not_mounted_last_leaves_a_click_to_the_browser`.
  47. **The listener prevents a click the router does not take.** `std/ui/backend/wasm.nova`:
     `if !e.modified() && a.attribute("target").is_none() && take() {` becomes `if !e.modified() && a.attribute("target").is_none() {`.
     - Run `CARGO_INCREMENTAL=0 NOVA_REQUIRE_NODE=1 cargo test -p nova-cli --test wasm_tests a_router_not_mounted_last_leaves_a_click_to_the_browser`.
     - Expected: fails: `a_router_not_mounted_last_leaves_a_click_to_the_browser`.
  48. **The generated page loads `./app.js`.** `crates/nova-codegen-wasm/src/shim/index.html`:
     `import init from "/app.js";` becomes `import init from "./app.js";`.
     - Run `CARGO_INCREMENTAL=0 NOVA_REQUIRE_NODE=1 cargo test -p nova-cli --test wasm_tests build_for_wasm_writes_dist`.
     - Expected: fails: `build_for_wasm_writes_dist`.
  49. **`use_params` keeps the last route's parameters after its render.** `std/ui/router/lib.nova`:
     `s.params = was`: the line goes.
     - Run `CARGO_INCREMENTAL=0 NOVA_REQUIRE_NODE=1 cargo test -p nova-cli --test wasm_tests router_routes_by_the_history_api`.
     - Expected: fails: `router_routes_by_the_history_api`.
  50. **Disposal leaves the `popstate` listener.** `std/ui/router/lib.nova`:
     `stop_location()`: the line goes.
     - Run `CARGO_INCREMENTAL=0 NOVA_REQUIRE_NODE=1 cargo test -p nova-cli --test wasm_tests router_routes_by_the_history_api`.
     - Expected: fails: `router_routes_by_the_history_api`.
  51. **Disposal leaves the click listener.** `std/ui/router/lib.nova`:
     `stop_clicks()`: the line goes.
     - Run `CARGO_INCREMENTAL=0 NOVA_REQUIRE_NODE=1 cargo test -p nova-cli --test wasm_tests router_routes_by_the_history_api`.
     - Expected: fails: `router_routes_by_the_history_api`.
  52. **Disposing of a router clears a later one's state.** `std/ui/router/lib.nova`:
     replace

     ```
                 if is_active(id) {
                     let mut s = routing()
     ```

     with

     ```
                 if true {
                     let mut s = routing()
     ```

     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests ui_programs_run_and_survive_gc_stress`.
     - Expected: fails: `ui_programs_run_and_survive_gc_stress`.
  53. **`modified` misses Ctrl.** `crates/nova-codegen-wasm/src/shim.rs`:
     `e.ctrlKey || e.metaKey` becomes `e.metaKey`.
     - Run `CARGO_INCREMENTAL=0 NOVA_REQUIRE_NODE=1 cargo test -p nova-cli --test wasm_tests router_routes_by_the_history_api`.
     - Expected: fails: `router_routes_by_the_history_api`.
  54. **`modified` misses Meta.** `crates/nova-codegen-wasm/src/shim.rs`:
     `e.metaKey || e.shiftKey` becomes `e.shiftKey`.
     - Run `CARGO_INCREMENTAL=0 NOVA_REQUIRE_NODE=1 cargo test -p nova-cli --test wasm_tests router_routes_by_the_history_api`.
     - Expected: fails: `router_routes_by_the_history_api`.
  55. **`modified` misses Shift.** `crates/nova-codegen-wasm/src/shim.rs`:
     `e.shiftKey || e.altKey` becomes `e.altKey`.
     - Run `CARGO_INCREMENTAL=0 NOVA_REQUIRE_NODE=1 cargo test -p nova-cli --test wasm_tests router_routes_by_the_history_api`.
     - Expected: fails: `router_routes_by_the_history_api`.
  56. **`modified` misses Alt.** `crates/nova-codegen-wasm/src/shim.rs`:
     `e.altKey || (e.button` becomes `(e.button`.
     - Run `CARGO_INCREMENTAL=0 NOVA_REQUIRE_NODE=1 cargo test -p nova-cli --test wasm_tests router_routes_by_the_history_api`.
     - Expected: fails: `router_routes_by_the_history_api`.
  57. **Pieces are split before decoding.** `crates/nova-cli/src/dev/server.rs`:
     replace

     ```
         let pieces: Vec<&str> = decoded
             .split('/')
     ```

     with

     ```
         let pieces: Vec<&str> = path
             .split('/')
     ```

     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --bin nova dev::server`.
     - Expected: fails: `dev::server::tests::a_path_names_its_file_and_a_slash_names_index_html`, `dev::server::tests::a_path_that_leaves_the_directory_or_holds_a_backslash_or_nul_is_403`.
  58. **A `\` is allowed.** `crates/nova-cli/src/dev/server.rs`:
     `if decoded.contains(['\\', '\0']) {` becomes `if decoded.contains(['\0']) {`.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --bin nova dev::server`.
     - Expected: fails: `dev::server::tests::a_path_that_leaves_the_directory_or_holds_a_backslash_or_nul_is_403`.
  59. **A NUL is allowed.** `crates/nova-cli/src/dev/server.rs`:
     `if decoded.contains(['\\', '\0']) {` becomes `if decoded.contains(['\\']) {`.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --bin nova dev::server`.
     - Expected: fails: `dev::server::tests::a_path_that_leaves_the_directory_or_holds_a_backslash_or_nul_is_403`.
  60. **A link out of `dist/` is served.** `crates/nova-cli/src/dev/server.rs`:
     `Ok(real) if !real.starts_with(&dist) => Target::Status(403),`: the line goes.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --bin nova dev::server`.
     - Expected: fails: `dev::server::tests::a_link_out_of_the_directory_is_403_and_one_inside_is_served`.
  61. **Neither `..` nor a piece that is not a plain name is refused.** `crates/nova-cli/src/dev/server.rs`:
     replace

     ```
         if pieces.contains(&"..") {
             return Target::Status(403);
         }
         if pieces.first() == Some(&"__nova") {
             return match pieces[1..] {
                 ["reload.js"] => Target::Reload,
                 ["events"] => Target::Events,
                 _ => Target::Status(404),
             };
         }
         // A piece that is not one plain name, such as `C:` on Windows, could
         // replace the directory it is joined to.
         if !pieces.iter().all(|piece| plain(piece)) {
     ```

     with

     ```
         if pieces.contains(&"...") {
             return Target::Status(403);
         }
         if pieces.first() == Some(&"__nova") {
             return match pieces[1..] {
                 ["reload.js"] => Target::Reload,
                 ["events"] => Target::Events,
                 _ => Target::Status(404),
             };
         }
         // A piece that is not one plain name, such as `C:` on Windows, could
         // replace the directory it is joined to.
         if !pieces.iter().all(|piece| plain(piece) || !piece.is_empty()) {
     ```

     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --bin nova dev::server`.
     - Expected: fails: `dev::server::tests::a_path_that_leaves_the_directory_or_holds_a_backslash_or_nul_is_403`.
  62. **A `Host` that starts with an allowed name passes, on any port.** `crates/nova-cli/src/dev/server.rs`:
     replace

     ```
             .any(|allowed| allowed.eq_ignore_ascii_case(host.trim()))
     ```

     with

     ```
             .any(|allowed| host.trim().starts_with(allowed.split(':').next().unwrap_or_default()))
     ```

     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --bin nova dev::server`.
     - Expected: fails: `dev::server::tests::the_host_must_name_the_server_and_its_port`.
  63. **`POST` is served.** `crates/nova-cli/src/dev/server.rs`:
     replace

     ```
         if request.method != "GET" && request.method != "HEAD" {
     ```

     with

     ```
         if request.method != "GET" && request.method != "HEAD" && request.method != "POST" {
     ```

     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --bin nova dev::server`.
     - Expected: fails: `dev::server::tests::the_answer_checks_the_host_then_the_method_then_the_path`.
  64. **A head may take 32 KiB.** `crates/nova-cli/src/dev/server.rs`:
     `const HEAD_LIMIT: usize = 16 * 1024;` becomes `const HEAD_LIMIT: usize = 32 * 1024;`.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test dev_tests it_serves_the_page_and_files_and_refuses_what_it_must`.
     - Expected: fails: `it_serves_the_page_and_files_and_refuses_what_it_must`.
  65. **No `Cache-Control: no-store`.** `crates/nova-cli/src/dev/server.rs`:
     `Cache-Control: no-store\r\n{allow}Connection: close` becomes `{allow}Connection: close`.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --bin nova dev::server`.
     - Expected: fails: `dev::server::tests::a_response_has_its_length_no_store_and_no_body_for_head`.
  66. **`.wasm` is `application/octet-stream`.** `crates/nova-cli/src/dev/server.rs`:
     `Some("wasm") => "application/wasm",` becomes `Some("wasm") => "application/octet-stream",`.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --bin nova dev::server`.
     - Expected: fails: `dev::server::tests::content_types_go_by_extension`.
  67. **`.mjs` is `application/octet-stream`.** `crates/nova-cli/src/dev/server.rs`:
     `Some("js" | "mjs") => JS,` becomes `Some("js") => JS,`.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --bin nova dev::server`.
     - Expected: fails: `dev::server::tests::content_types_go_by_extension`.
  68. **A `HEAD` of the event stream says `keep-alive`.** `crates/nova-cli/src/dev/server.rs`:
     `stream.write_all(EVENT_STREAM_HEAD_ONLY.as_bytes())?;` becomes `stream.write_all(EVENT_STREAM_HEAD.as_bytes())?;`.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --bin nova dev::server`.
     - Expected: fails: `dev::server::tests::a_head_of_the_event_stream_says_its_connection_closes`.
  69. **A head whose lines end in a bare LF never ends.** `crates/nova-cli/src/dev/server.rs`:
     `[b'\n', b'\n', ..] => Some(i + 2),`: the line goes.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --bin nova dev::server`.
     - Expected: fails: `dev::server::tests::a_head_ends_at_its_first_empty_line_whatever_its_line_ends`.
  70. **A head's lines are split at CRLF alone.** `crates/nova-cli/src/dev/server.rs`:
     replace

     ```
         let mut lines = text
             .split('\n')
     ```

     with

     ```
         let mut lines = text
             .split("\r\n")
     ```

     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --bin nova dev::server`.
     - Expected: fails: `dev::server::tests::a_head_parses_into_method_target_and_host`.
  71. **Any version that begins `HTTP/1.` passes.** `crates/nova-cli/src/dev/server.rs`:
     `if minor.len() != 1 || !minor.bytes().all(|b| b.is_ascii_digit()) {` becomes `if !version.starts_with("HTTP/1.") {`.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --bin nova dev::server`.
     - Expected: fails: `dev::server::tests::a_head_parses_into_method_target_and_host`.
  72. **`HEAD` has a body.** `crates/nova-cli/src/dev/server.rs`:
     replace

     ```
         if !head_only {
             stream.write_all(body)?;
     ```

     with

     ```
         if true {
             stream.write_all(body)?;
     ```

     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --bin nova dev::server`.
     - Expected: fails: `dev::server::tests::a_response_has_its_length_no_store_and_no_body_for_head`.
  73. **A missing file with a `.` gets the page.** `crates/nova-cli/src/dev/server.rs`:
     `let page = pieces.last().map_or(true, |piece| !piece.contains('.'));` becomes `let page = true;`.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --bin nova dev::server`.
     - Expected: fails: `dev::server::tests::a_path_with_no_file_and_no_dot_is_the_page_and_any_other_is_404`, `dev::server::tests::the_servers_own_page_stands_in_for_a_missing_index_html`.
  74. **No fallback: every missing file is `404`.** `crates/nova-cli/src/dev/server.rs`:
     replace

     ```
         if !page {
             return Target::Status(404);
         }
     ```

     with

     ```
         if true {
             return Target::Status(404);
         }
     ```

     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --bin nova dev::server`.
     - Expected: fails: `dev::server::tests::a_link_out_of_the_directory_is_403_and_one_inside_is_served`, `dev::server::tests::a_page_reloads_on_a_hello_that_is_not_its_tags_id`, `dev::server::tests::a_path_with_no_file_and_no_dot_is_the_page_and_any_other_is_404`, `dev::server::tests::the_answer_checks_the_host_then_the_method_then_the_path`, `dev::server::tests::the_servers_own_page_stands_in_for_a_missing_index_html`.
  75. **No page of the server's own.** `crates/nova-cli/src/dev/server.rs`:
     `_ => Target::OwnPage,` becomes `_ => Target::Status(404),`.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --bin nova dev::server`.
     - Expected: fails: `dev::server::tests::the_answer_checks_the_host_then_the_method_then_the_path`, `dev::server::tests::the_servers_own_page_stands_in_for_a_missing_index_html`.
  76. **The script goes before the first `</body>`.** `crates/nova-cli/src/dev/server.rs`:
     replace

     ```
             .rev()
             .find(|&i| {
     ```

     with

     ```
             .find(|&i| {
     ```

     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --bin nova dev::server`.
     - Expected: fails: `dev::server::tests::the_script_goes_before_the_last_body_end_or_at_the_end`.
  77. **`Content-Length` is one more than the body.** `crates/nova-cli/src/dev/server.rs`:
     replace

     ```
             body.len(),
         );
     ```

     with

     ```
             body.len() + 1,
         );
     ```

     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --bin nova dev::server`.
     - Expected: fails: `dev::server::tests::a_response_has_its_length_no_store_and_no_body_for_head`.
  78. **`/__nova/` is read from disk.** `crates/nova-cli/src/dev/server.rs`:
     `if pieces.first() == Some(&"__nova") {` becomes `if pieces.first() == Some(&"__nova_") {`.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --bin nova dev::server`.
     - Expected: fails: `dev::server::tests::paths_under_nova_are_the_servers_own`, `dev::server::tests::the_answer_checks_the_host_then_the_method_then_the_path`.
  79. **An event has no blank line after it.** `crates/nova-cli/src/dev/server.rs`:
     replace

     ```
         }
         out.push('\n');
         out
     }
     ```

     with

     ```
         }
         out
     }
     ```

     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --bin nova dev::server`.
     - Expected: fails: `dev::server::tests::a_page_hears_hello_then_reloads_and_errors_and_a_failed_page_is_dropped`, `dev::server::tests::a_page_reloads_on_a_hello_that_is_not_its_tags_id`, `dev::server::tests::an_errors_lines_are_one_data_line_each`.
  80. **An error is one `data:` line.** `crates/nova-cli/src/dev/server.rs`:
     `for line in data.trim_end_matches('\n').split('\n') {` becomes `for line in [data.trim_end_matches('\n')] {`.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --bin nova dev::server`.
     - Expected: fails: `dev::server::tests::a_page_hears_hello_then_reloads_and_errors_and_a_failed_page_is_dropped`, `dev::server::tests::an_errors_lines_are_one_data_line_each`.
  81. **A page that joins hears `reload`, not `hello`.** `crates/nova-cli/src/dev/server.rs`:
     `message.push_str(&event("hello", &self.id_of(stream.build)));` becomes `message.push_str(&event("reload", &self.id_of(stream.build)));`.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --bin nova dev::server`.
     - Expected: fails: `dev::server::tests::a_page_hears_hello_then_reloads_and_errors_and_a_failed_page_is_dropped`, `dev::server::tests::a_page_reloads_on_a_hello_that_is_not_its_tags_id`.
  82. **A build's id is its number alone.** `crates/nova-cli/src/dev/server.rs`:
     `format!("{}.{build}", self.start)` becomes `format!("{build}")`.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --bin nova dev::server`.
     - Expected: fails: `dev::server::tests::a_page_hears_hello_then_reloads_and_errors_and_a_failed_page_is_dropped`, `dev::server::tests::a_page_reloads_on_a_hello_that_is_not_its_tags_id`, `dev::server::tests::the_answer_checks_the_host_then_the_method_then_the_path`.
  83. **The page's tag has no build id.** `crates/nova-cli/src/dev/server.rs`:
     replace

     ```
     format!("<script src=\"/__nova/reload.js\" data-build=\"{id}\"></script>")
     ```

     with

     ```
     format!("<script src=\"/__nova/reload.js\"></script>")
     ```

     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --bin nova dev::server`.
     - Expected: fails: `dev::server::tests::a_page_reloads_on_a_hello_that_is_not_its_tags_id`, `dev::server::tests::the_script_goes_before_the_last_body_end_or_at_the_end`.
  84. **A failed build is not counted.** `crates/nova-cli/src/dev/server.rs`:
     replace

     ```
             stream.build += 1;
     ```

     with

     ```
             if outcome.is_ok() {
                 stream.build += 1;
             }
     ```

     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --bin nova dev::server`.
     - Expected: fails: `dev::server::tests::a_page_hears_hello_then_reloads_and_errors_and_a_failed_page_is_dropped`, `dev::server::tests::a_page_reloads_on_a_hello_that_is_not_its_tags_id`.
  85. **A build starts one poll after a change, settled or not.** `crates/nova-cli/src/cmd/dev.rs`:
     replace

     ```
             if next == now {
                 return now;
             }
     ```

     with

     ```
             if true {
                 return next;
             }
     ```

     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --bin nova cmd::dev`.
     - Expected: fails: `cmd::dev::tests::a_build_waits_for_a_poll_that_sees_nothing_new`.
  86. **The output directory is watched.** `crates/nova-cli/src/cmd/dev.rs`:
     replace

     ```
                 if deep && name != "target" && !name.starts_with('.') && !is_out(&path, out) {
     ```

     with

     ```
                 if deep && name != "target" && !name.starts_with('.') {
     ```

     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --bin nova cmd::dev`.
     - Expected: fails: `cmd::dev::tests::a_change_an_addition_and_a_removal_are_each_seen`, `cmd::dev::tests::a_project_watches_its_sources_manifest_page_and_public`.
  87. **A single file's watch reads the directories below its own.** `crates/nova-cli/src/cmd/dev.rs`:
     `deep: false,` becomes `deep: true,`.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --bin nova cmd::dev`.
     - Expected: fails: `cmd::dev::tests::a_single_file_watches_the_nova_files_of_its_directory`.
  88. **A link to nothing is not watched.** `crates/nova-cli/src/cmd/dev.rs`:
     `seen.insert(path.to_path_buf(), (None, 0));`: the line goes.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --bin nova cmd::dev`.
     - Expected: fails: `cmd::dev::tests::a_link_to_nothing_is_watched_as_a_file`.
  89. **The port's message names no `--port`.** `crates/nova-cli/src/cmd/dev.rs`:
     `"nova dev: port {} is in use; choose another with --port",` becomes `"nova dev: port {} is busy",`.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test dev_tests a_second_server_on_the_same_port_fails`.
     - Expected: fails: `a_second_server_on_the_same_port_fails`.
  90. **An output directory holding the project is not refused.** `crates/nova-cli/src/cmd/dev.rs`:
     `bundle::check_output_for(root, &out)?;`: the line goes.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test dev_tests it_refuses_what_build_refuses`.
     - Expected: fails: `it_refuses_what_build_refuses`.
  91. **A build's child runs without `NO_COLOR`.** `crates/nova-cli/src/cmd/dev.rs`:
     `.env("NO_COLOR", "1")`: the line goes.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test dev_tests the_event_stream_reloads_and_reports_errors`.
     - Expected: fails: `the_event_stream_reloads_and_reports_errors`.
  92. **A tab, LF or CR stays in a link's `to`, so `/\t/x` is `//x` to a browser.** `std/ui/router/lib.nova`:
     replace

     ```
         let tabless = "".join(to.split("\t"))
         let lineless = "".join(tabless.split("\n"))
         let plain = "".join(lineless.split("\r"))
     ```

     with

     ```
         let plain = to
     ```

     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests ui_programs_run_and_survive_gc_stress`.
     - Expected: fails: `ui_programs_run_and_survive_gc_stress`.

- [ ] **Step 5: The review's inputs.** `06-counter-spa`'s size and the full
  run's figures, as Step 1 measured them, and the mutants' outcome go in
  the ledger.
- [ ] **Step 6: One fresh read-only review, then one fix pass** (the
  user's workflow).
  - The reviewer reads `git diff $BASE..HEAD` (Task 0) against the spec,
    ADR 0044 and this plan, and is told to hunt for:
    - a location that shows another route than it names: matching,
      decoding, the query, `*`, a `#fragment`;
    - a route built twice, or a history entry pushed or replaced wrongly:
      the location read back, compared decoded and without its fragment;
    - a click taken from the browser or left to it wrongly: the walk to the
      marked `<a>`, the modifier keys and the button, `target`, and a
      router that is not the one mounted last (decision 9);
    - a link's `href` wrong in either mode, before its router, with a
      reactive `to`, or naming another host by a leading `//` or `\`;
    - a generated page that cannot be reloaded at a history path, or a
      project's page the records tell wrongly how to load `/app.js`;
    - the router's state freed, shared or kept: word 112 marked, the
      native root, listeners and state on disposal and on takeover,
      parameters outside a render;
    - a file served from outside the output directory, by any spelling or
      link, on any system, Windows's drive and verbatim paths included; and
      the checks' nits left standing: `/__NOVA/…` read from disk on a file
      system that ignores case, and `/__nova/reload.js/` and
      `/__nova//events` answered as the server's own;
    - the `Host` check passed by a name that is not the server's, or
      checked after anything that answers;
    - the fallback answering a script with HTML, or a history path with
      `404`; the server's own page when there is a page;
    - the reload script's tag in the wrong place, `Content-Length` not
      counting it, an event framed wrong, an id that lets a page keep an
      old build, or a failed build that reloads a page instead of showing
      the overlay;
    - a watcher that misses an edit, sees its own output, or builds twice
      for one save; a build left running, or two at once;
    - a connection, thread or child process that can be held open for
      ever, a build left running when `nova dev` is killed (decision 19),
      or a head read without its limit or its line ends;
    - a test that could pass without the code it names;
    - a record that says what the code does not, MSRV 1.78, and tests that
      touch the internet, use port 3000, or use temp directories without
      fixed names.
  - Each finding is fixed in one commit, `4.5: the review's fixes`, or
    answered in the ledger with the reason it stands.
  - After the fixes, Step 1 runs again.
- [ ] **Step 7: The PR.**
  - Push: `git push -u origin claude/loving-archimedes-frtf5q-45`.
  - Open the PR to `main` with the repository's template headings, if it
    has one. The body names no local path and no person.
  - It holds:
    - what 4.5 adds;
    - the gate's evidence: `06-counter-spa` under `nova dev` in Chromium,
      counting, reloading on an edit and after a missed build, and showing
      and clearing the overlay; the router's browser test; the router's
      programs under the fake DOM, as they are and under stress, and
      natively; `dev_tests.rs` on all three systems; the size;
    - the user's decision 9, and the decisions, numbered as here, with 3
      to 9, 13, and 15 to 22 marked as differing from the spec;
    - that `Cargo.lock` is unchanged;
    - the mutants, and the review's findings and what came of each.
  - Read CI once it ends. Expected: green on all three systems, and the
    `browser` job green; the test job runs the WASM tests under Node 22
    with `NOVA_REQUIRE_NODE=1`.
- [ ] **Step 8: The merge,** only on the user's word.
  - By rebase, pinned to the PR's head commit. Never auto-merge.
  - Then `git fetch origin main` and `git diff <head> origin/main
    --stat`, which prints nothing: the merged tree is the PR's.
