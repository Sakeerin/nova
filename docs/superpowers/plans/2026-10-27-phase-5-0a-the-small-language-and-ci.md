# Phase 5.0a, "The small language and CI", Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
superpowers:subagent-driven-development (recommended) or
superpowers:executing-plans to implement this plan task-by-task. Steps use
checkbox (`- [ ]`) syntax for tracking.

**Goal:** The first of Phase 5.0's three parts: the small language a
compiler written in Nova needs, and the CI that holds the LLVM backend to
the golden programs. The lexer marks each token that follows a line break,
and the parser implements the master spec's "newline terminates
statement": in statement context a line break ends an expression before a
`(`, a `[` or a binary `-`, and a bare `return` or `break` ends at the line
break; `nova fmt` follows, dropping the `;` the rule makes needless and
breaking a chain with a `-` after each operator. `e?` works on std's
`Option` and `Result`, converting nothing, with E0098 and E0099; `+=` and
its kin work on fields and elements, each receiver and index evaluated
once; `String + String` is a new `String`; E0102 warns at a variant that
hides one of std's, and E0103 at a statement whose value is unused. A new
test, `release_golden_suite`, builds every golden program with
`--release` through `clang` and runs it, and CI's Linux leg runs it on
`ubuntu-24.04` with Ubuntu's `clang-18` 18.1.3, its version asserted. ADR
0048 records Phase 5's scope, its backend and the pin.

**Architecture:**
- **`crates/nova-lexer/src/tokens.rs`** (new): `Tokens`, the tokens and a
  "line break before" mark each, which `lex` and `lex_with_comments`
  return. **`crates/nova-parser/src/grammar.rs`:** `stmt_ctx`, saved and
  restored; the postfix loop's `(`/`[` and the additive loop's `-` stop
  after a line break in statement context; `bare_keyword_ends`.
- **`crates/nova-fmt/src/print/{mod,expr}.rs`:** `CONTINUES` without `(`,
  `[` and `-`; `VIEW_CONTINUES` for a view's children; no `;` after a bare
  `return` or `break`; `binary` breaking after each operator in a chain
  with `-`.
- **`crates/nova-typeck/src/check.rs`:** `check_try` and `core_sum`; a
  closure's `return` and `?` against its own return type; `op=` in
  `check_index_set` and `check_field_set` through unscoped locals;
  `String + String` in `binary_result_ty`; E0103 at an expression
  statement. **`crates/nova-typeck/src/check/lang_tests.rs`** (new): their
  tests.
- **`crates/nova-mir/src/lower.rs`:** a `String` `Add` is one call of
  `StrConcat`.
- **`crates/nova-resolver/src/lib.rs`:** E0102, `variant_shadowing`.
- **`crates/nova-cli/tests/run_tests.rs`:** `release_golden_suite` and its
  tables; four golden programs under `run`, `build` and GC stress, and
  E0102's CLI test. **`.github/workflows/ci.yml`:** the `test` job's Linux
  leg on `ubuntu-24.04` with `clang-18`.
- **`tests/runtime/`:** `try_operator`, `try_async`,
  `compound_assign_places`, `string_concat`, each with its `.stdout`, in
  `wasm_tests.rs`'s gate too.
- **Records:** ADR 0048; `CHANGELOG.md`; dated notes in eight files of
  `nova-spec/`; the phase plan's 5.0 amendment.

**Tech Stack:**
- Rust: MSRV 1.78, edition 2021.
- **No new crate; `Cargo.lock` does not change** (spec §7.1).
- `clang`: Ubuntu's `clang-18`, 18.1.3, on CI's `ubuntu-24.04` leg; this
  machine has the same.
- Tests: `nova-lexer`'s `lexer_tests.rs`; `nova-parser`'s new
  `newline_rule.rs`; `nova-fmt`'s `source_rules.rs` and corpus test;
  `nova-typeck`'s `check::lang_tests` and `check::tests`; the resolver's
  tests; `ir_tests.rs`; `run_tests.rs`, `wasm_tests.rs` and
  `index_complete.rs`; the golden suite under `NOVA_REQUIRE_CLANG=1`.

**Spec:**
`docs/superpowers/specs/2026-10-26-phase-5-0-the-language-and-std-a-compiler-needs-design.md`,
approved by the user on 2026-10-26 (`1fc7af6`).
- Read its §3 and §6 to §13 before Task 1: it is the authority this plan
  argues from.
- Its header holds the user's eight decisions, its §12 the decisions made
  while writing it, and its §13 what this plan settles. This plan's
  decisions say where the code departs from the spec, or fills in what it
  left open.

**Checked on 2026-10-27, before execution:**
- **A prototype built 5.0a whole** on a local branch from `1fc7af6`, in
  this tree, in two commits, `aff10f8` (the code, the tests and CI) and
  `848211a` (ADR 0048 and the records), then a third, the gap fix,
  `f00c99b`, a fourth, the fact-check's fixes, `125d8f1`, a fifth, the
  second check's fixes, `794e915`, and a sixth, `77b6b2f`, the test the
  mutants then asked for (below). It settled spec §13's questions:
  the marks inside `Tokens` (decision 12), the chain's operator ending its
  line (decision 15), the golden table (decision 25), CI's YAML (decision
  26), E0103's forms (decision 23) and E0102's list (decision 22). It
  broke each fix and saw its tests fail: the parser's `(`/`[` arm (11
  tests), its `-` arm (2) and the bare `return` (4); the marks forced off
  (4); the old `CONTINUES` (3), and without the chains' layout fmt's corpus
  test; MIR's arm (the IR test, and `string_concat_runs` aborting); the
  closure's return type (3); E0103 off (2); E0102 off (5); the index
  evaluated twice (the HIR test, and "index evaluated 2 time(s)"); a sum of
  the program's own named `Option` (1); a corrupted `.stdout` and a bad
  `NOVA_CLANG` (the golden suite). Its full suite gave `2505 1 10`
  (`2508 1 10` after the gap fix, `2522 1 10` after the fact-check's
  fixes, `2529 1 10` after the second check's), and the golden suite
  `131 passed, 9 excluded` in 18.0 s on 4
  cores.
- **A replay split the prototype into this plan's tasks,** on the local
  branch `split/5-0a` from `1fc7af6`, by a script, and ran each task's
  steps as written:
  - fourteen commits: a tests commit and a code commit for each of Tasks 1
    to 6, then Task 7 and Task 8, each the patch of its task below. The
    last commit's tree is `77b6b2f`'s, the prototype's last (`git diff
    --stat 77b6b2f split/5-0a` prints nothing). The split was made of
    `848211a`, then of `f00c99b`, then of `125d8f1`, then of `794e915` and
    `77b6b2f`,
    whose parser changes ride in Task 6 with the splits, its fmt
    self-check's too, and whose words for fmt name E0103 only from Task 6
    on, which the script checks: the gap fix's tests
    ride in Tasks 1, 4 and 6, its assertion in Task 7, and its records in
    Tasks 3 and 8; the fact-check's splits, in the parser, the AST, the
    resolver and the checker, ride in Task 6 with their warning, so Tasks
    1 to 5 have the parser as it was before them, which the script builds
    from the prototype's history; its fmt change in Task 2, its `?` fixes
    in Task 3, its tests in Tasks 1, 3 and 6, its floor in Task 7 and its
    records in Task 8. Each patch applies with plain `git apply`, in order,
    on `1fc7af6`, with no warning: no added line holds trailing
    whitespace, a CR or a backslash-u escape;
  - the split departs from the natural one in one place: fmt's broken
    chains ride with the rule in Task 1, since without them fmt's corpus
    test fails at Task 1's code (decision 29); and the golden programs
    ride with their features, `compound_assign_places` and `string_concat`
    with Task 5 (decision 30);
  - each tests commit failed as its step says before its code, the
    lexer's tests by not compiling (decision 31), and passed after it;
    every crate's tests, `cargo fmt`, clippy with `-D warnings` and `nova
    fmt --check` passed after every task;
  - every "Expected:" below is the replay's, from a fresh `NOVA_HOME`,
    but Task 0's suite figures, which are 4.8's last full run, on the tree
    `1fc7af6` holds: the replay did not run the suite at the base.
- **The final checks ran on the split's last commit:**
  - the full suite gave `2529 1 10` (234 s): the base's `2420 1 10`
    and 109 tests more, which a comparison by binary finds as
    115 added and 6 gone, the gone ones four renamed tests
    and two doc tests moved to another line (Task 9, Step 1); the one
    failure is the IPv6 test;
  - CI's Linux leg, the whole suite under `NOVA_REQUIRE_CLANG=1
    NOVA_CLANG=clang-18 NOVA_REQUIRE_NODE=1`, run once as CI runs it:
    `2529 1 10`, the golden suite among them (Task 9, Step 2);
  - the golden suite, in 17.4 seconds of test time, 20.1 s wall under `time`:
    `release golden: 131 passed, 9 excluded, Ubuntu clang version 18.1.3 (1ubuntu1)`;
  - the newline rule over the repository: 279 files, the four new programs
    among them, every dump the same, spans included; the base's `nova` and
    the new one both exit non-zero on the same two files, the parser fixture
    `async.nova` and the language server's `complete.nova` fixture, which do
    not parse by design;
  - E0102 and E0103: E0102 twice in a user file, the parser fixture
    `sum_type.nova`, and 51 times in std's sources checked by hand as user
    files, which already fail at the base; E0103 nowhere (Task 9, Step 5);
  - fmt on the second check's probes (Task 9, Step 6):
    `fmtprobe: 60 files, 55 ok, 5 with a parse error, 0 bad`. The five are
    P0001s: `brk_bare.nova`, `brk_val.nova`, `rcomma.nova` and `rp.nova` as
    at the base, `rp.nova` since decision 46; and `arm.nova`, `0 => a` /
    `- 1,`, which parsed at the base and is now decision 47's P0001, with
    its note;
  - Rust 1.78's check was clean, and `Cargo.lock` did not change.
- **The gap fix,** `f00c99b 5.0a prototype: close three test gaps`,
  after the mutants' first run (below) and the replay's reading of the
  code: a `match` inside parentheses keeping the rule in its arms (decision
  33); a field's receiver with a side effect evaluated once (34); the
  golden suite failing rather than skip under `NOVA_REQUIRE_CLANG=1` (35);
  `check_closure`'s doc comment, which `core_sum` had displaced, back above
  its function; `60-EXAMPLES.md`'s three notes keeping the spec's path on
  one line; and E0103's true miss, `foo` then `(bar)()`, pinned by a test,
  the prototype's report having claimed that `foo` then `(bar())` is
  flagged at `foo` alone, which the replay found false: both are flagged
  (36). Each new test failed with its mutant applied and passed without
  it.
- **A fact-check** read this plan against the split at `777ad920` and
  found eight things, each answered in `125d8f1 5.0a prototype: the
  fact-check's fixes` or here, each code fix with a test that fails
  without it, shown by its mutant:
  1. the rule changed a program's meaning in silence, the base's fmt
     having broken `return a - b` into `return a` / `- b` (900 became
     1000): by the user's ruling, the parser records each split and E0103
     warns at each, a written `;` silencing it, and fmt keeps the author's
     `;` (decisions 9 and 37; mutants 40 to 45);
  2. no replay run matched CI's Linux leg: Task 9, Step 2 now runs it;
  3. E0099 contradicted itself for the program's own `Option`: a note now
     says so (decision 39; mutant 47);
  4. `?` in a constant's initializer was accepted, and gave `None`: by the
     user's ruling it is E0099 (decision 38; mutant 46);
  5. test gaps: interpolation holes and nested items under the rule
     (decision 40; mutants 48 and 49); two tests named for what they do
     not test, renamed (decisions 20 and 22); no floor on the golden walk
     (decision 25; mutant 51);
  6. the plan's errors: the counts of tests added and gone, decision 31's
     list, Task 0's figures said to be the replay's, and decision 20 not
     marked as differing: all corrected;
  7. records: the CHANGELOG's claim about parentheses, "every golden
     program" in ADR 0048 and `14-CODEGEN.md`, and E0103's note where no
     line break was involved (decision 42);
  8. nits: `parse_view`'s comment and a lone `\r` fixed (decisions 41 and
     43; mutant 50); the E0010 then E0011 cascade and a comment's place in
     a broken chain recorded (decision 43).
- **A second check** read this plan against the split at `f8f8b61a` and
  found what `794e915 5.0a prototype: the second check's fixes` and this
  draft answer, each code fix with a test that fails without it, shown by
  its mutant:
  1. fmt refused any file whose layout moved a split, its self-check
     comparing the splits' places: it now compares their kinds, in order
     (decision 44; mutants 54 and 55);
  2. after a bare `return` or `break`, a line starting with `(`, `[` or `-`
     recorded two splits at one token, and an arm dropped only one of them:
     the bare keyword is no operand now, and an arm drops every split at
     its end (decision 45; mutants 56 and 57);
  3. `g(return` / `)` was accepted with a false warning: before a token no
     expression begins with, the keyword reads as before the rule (decision
     46; mutant 58);
  4. the mutants it asked for: the `Call` and `Index` records, the
     fingerprint's kinds, the halt, the arm's drop and the exclusion
     (mutants 52 to 58), and the arm's note (59);
  5. the plan: decisions 9 and 37 and Review Focus 1 and 4 said what the
     code no longer does; Review Focus cited the fact-check's probes, which
     no step ran, and now cites their like in `lang_tests.rs`; and Task 9's
     run of CI's leg lacked `--all-features`: all corrected;
  6. nits: `SPLITS`' doc comment, E0103 named before Task 6, two long
     lines, all fixed; a P0001 at an arm the rule began now has a note
     (decision 47); a lone `\r` in a diagnostic recorded, not fixed
     (decision 48); and the replay's disk, which fell to 5.9G once, kept
     above 6G this time, older duplicate test executables going before
     each step;
  7. the mutants' run on `794e915`'s split then left one gap: an arm that
     dropped only the last split at its end token, mutant 57, passed every
     test, a `return`'s value meeting the split before the next arm and the
     `return` around it meeting it again. `77b6b2f 5.0a prototype: an arm
     drops a split it met twice` adds the test, which fails with the
     mutant applied, and the split was made again (decision 45).
- **Amended 2026-10-27 after the review:** the review of the executed
  branch found five things in the code, which `5.0a: the review's fixes`
  answers, each with a test seen failing without its fix:
  1. fmt dropped an author's `;` after a bare keyword nested in a
     statement, `let f = || return;` or `x = break;`, then refused the
     file: its `bare` now asks whether the statement's last code token is
     `return` or `break`, as the base did; and after one, in a block or a
     view, it adds no `;` before a line that starts with `{` or another
     token it would otherwise separate, which removed the split, and so
     refused `return` / `{ p }` and the view of item 4 (decision 9);
  2. `let f = || return` / `(5)`, or `[`, recorded two splits at one
     token, the closure's `Value` and the `let`'s `Call` or `Index`:
     `record_split` keeps one split a token, the first (decision 45);
  3. a P0001 at an arm the rule began had no note where it landed past
     the split's token, `0 => g` / `(5)` / `_ => 1` at `_`, or followed a
     bare keyword, `None => return` / `0,` at `,`: the first error in the
     head of such an arm takes the note, one fitted to the keyword after a
     `Value` split (decisions 37 and 47);
  4. `halt` was never cleared on a view child's path, so `view {` /
     `return` / `(1)` / `}` failed to parse: a view child clears it as
     `parse_expr` does (decision 45);
  5. the golden suite now quotes a failing program's standard error, and
     gives `nova build --release` the run's 60-second limit.
- **The mutants.** 61 breakages of what 5.0a adds, each a text
  replaced in one file, run one at a time at the split's last commit (Task
  9, Step 9): the rule's arms and contexts (11), the marks (3), fmt (7),
  `?` and the closure (9), `op=` (3), the concatenation (2), E0102 (2),
  E0103 and the splits (14), fmt's self-check (2), the golden suite (6) and
  CI (2).
  On the last split, all 60 run failed as written, and 1 was not run (61,
  CI's version step); the eight added for the second check's fixes are 52 to
  59, and the CI mutants are now 60 and 61. The run before it, on
  `794e915`'s split, gave 59 caught, 1 not run and 1 gap: mutant 57, an arm
  dropping only the last split at its end token, passed every test, which
  `77b6b2f` closed (decision 45). On the fact-check's split, all 52 run
  failed as written, and 1 was not run; the twelve added for the
  fact-check's fixes are 40 to 51. The first run, at the first split's last
  commit (the tree of `848211a`), was cut short at mutant 37 by a restart of
  the machine, which left mutant 37 applied; the file was restored with
  `git checkout --` and all 41 of that list run again: 37 failed as written,
  3 passed, each a gap, and 1 was not run. The three gaps were a match arm's
  body inheriting its context (mutant 4), a field's receiver evaluated twice
  by `op=` (27), and the golden suite skipping under `NOVA_REQUIRE_CLANG=1`
  (39); the gap fix closed all three (decisions 33 to 35), and on its split
  the 41 gave 40 caught and 1 not run.
- **What the checks left standing,** each recorded or handed on:
  - CI's version step, which runs on GitHub's runner alone (mutant 61, not
    run), and the suite's run on `ubuntu-24.04`, which Task 9's CI run
    shows; Task 9, Step 2 runs the same command here;
  - the golden suite's assertion against a wrong skip guards the gate's
    condition, not CI's environment: a leg that never sets
    `NOVA_REQUIRE_CLANG=1` still skips, by design (decisions 5 and 35);
  - the splits' warning is syntactic: a split the parser before the rule
    would have refused to parse at all is still recorded where the parser
    cannot tell, but at an arm's end, before a `let` or an item, and before
    a token no expression begins with (decisions 37 and 46);
  - a lone `\r` in a diagnostic, which renders such a file as one line,
    `codespan-reporting`'s `SimpleFiles` taking `\n` alone for a line break:
    recorded, not fixed (decision 48);
  - `|o: Option<Int>| o?` gives one E0010 and then six E0011s, and a comment
    in a broken chain with `-` lands after the operator: recorded, not fixed
    (decision 43);
  - the change of meaning the rule makes for a line that starts with `(`,
    `[` or `-` (spec §10 item 1), which E0103 now reports and the CHANGELOG
    states;
  - the `Tokens` change to `lex`'s and `parse`'s public types, `ast::File`'s
    new field and `ParseError`'s new variant, `Noted` (decisions 12, 37 and
    47), and the closure's `return` refusing code that relied on the old
    checking (decision 19): no code in the repository does;
  - the pin's dependence on Ubuntu's archive keeping `clang-18` at 18.1.3
    (ADR 0048);
  - this machine ran the suite on Linux alone; macOS and Windows run it in
    CI, where the golden suite skips.

## Global Constraints

- **The workspace's minimum Rust stays 1.78,** edition 2021. CI's MSRV job
  runs `cargo check --locked --workspace` with `RUSTFLAGS=-D warnings`. No
  `std::sync::LazyLock` or `LazyCell` (1.80), no `Option::is_none_or`
  (1.82), no `std::path::absolute` (1.79). 5.0a's Rust uses
  `std::thread::scope` (1.63), `let … else` (1.65) and
  `std::thread::available_parallelism` (1.59), nothing newer.
- **No new crate, and `Cargo.lock` does not change** (spec §7.1). Any
  change to the lock stops the task: ask the user.
- **No `unwrap()` outside `#[cfg(test)]` and test files:** `expect("why")`,
  `unwrap_or…` or `?`. `Tokens::marked` uses `unwrap_or(&[])`.
- **Tests never touch the internet.** `clang` is installed by CI's job,
  never by a test or by `nova` (spec §7.1); `release_golden_suite` runs the
  `clang` it is given, and nothing installs one. No test uses a port but
  those the existing tests bind.
- **Temporary names are fixed** (spec §7.1): `release_golden_suite` builds
  into `std::env::temp_dir()/nova-release-golden/`, one `<name>.tmp`
  directory a program for its `TMPDIR`; the E0102 test writes
  `nova-e0102-warning/`. A test that needs `$NOVA_HOME` sets a fresh one.
- **Never clean `/tmp/nova-*`:** `nova test` keeps built binaries in
  `/tmp/nova-test-bin`, a test whose binary goes fails, and the golden
  suite's directory is one of them.
- **A test that needs Node** passes without running when `node` is not on
  `PATH`, and fails instead under `NOVA_REQUIRE_NODE=1`, which CI sets.
  Run the WASM tests here with `NOVA_REQUIRE_NODE=1`: Node 22 is installed.
- **`release_golden_suite` runs only under `NOVA_REQUIRE_CLANG=1`,** which
  CI's Linux leg sets with `NOVA_CLANG=clang-18`; elsewhere it prints
  `release golden: not run (set NOVA_REQUIRE_CLANG=1 to run it)` and
  passes. Run it here with both set: this machine's `clang-18` is Ubuntu's
  18.1.3, CI's pin.
- **`nova-cli`'s tests need two files beside `target/debug/nova`:**
  `libnova_runtime.a` and the `nova-bench-http` binary. A build of the
  workspace writes both; Task 0's build puts them there. Every run step
  starts with `cargo build --locked -p nova-cli -p nova-runtime`
  (Conventions), so the `nova` the steps run by hand is the tree's.
- **Work in one tree.** Two worktrees of this repository sharing one
  `target/` share artifact names, so cargo can take one tree's build as
  fresh for the other (4.0's ledger). Never build in another worktree.
- **Keep `df -h /` above 6G.** `target/debug/incremental` and older
  duplicate test executables in `target/debug/deps` may be removed; Task
  9's Rust 1.78 check uses its own target directory, removed after.
- **Never use the Write or Edit tool for text that holds a backslash-u
  escape,** a backslash and `u` followed by four hex digits or by a code in
  braces: they decode the escape into the character. None of 5.0a's
  patches holds one; apply them with `patch_block`, from the committed
  plan, and write anything that holds one with a script.
- **Nothing writes into the repository but the patches.** The golden suite
  builds under the system's temporary directory; the AST check of Task 9
  writes its dumps under `$P`.
- **Merge by rebase, on the user's word only.** Never enable auto-merge.
  Nothing is published and no tag is made in 5.0a.
- **This machine is Linux.** macOS and Windows results come from CI, where
  `release_golden_suite` skips.

## Conventions

- **Scratch directory:** `$P` is the session's scratch directory, outside
  the repository. Set it in every shell before a command that names `$P`.
- **The ledger:** `$P/ledger-50a.md`. Every ruling made while executing goes
  there, with its cost if wrong; so do each task's figures.
- **Commits.** Each task's message is written to `$P/msg-50a-<task>.txt`:
  the subject the task names, a blank line, and the session's attribution
  lines. Then `git add -A`; `git status --short` lists only the task's
  files; and `git commit -F "$P/msg-50a-<task>.txt"`. A task with a tests
  patch and a code patch makes one commit, after both.
- **Runs.** The replay ran every command with these variables set. Each
  shell sets them too, with five shorthands, and runs every command from
  the repository's root:

  ```sh
  export CARGO_INCREMENTAL=0 NO_COLOR=1 RUST_BACKTRACE=0 NOVA_HOME="$P/home-50a"
  unset NOVA_RUNTIME_LIB NOVA_REQUIRE_NODE NOVA_REQUIRE_CLANG NOVA_CLANG
  mkdir -p "$NOVA_HOME"
  W() { NOVA_REQUIRE_NODE=1 cargo test --locked -p nova-cli --test wasm_tests -- "$@"; }
  R() { cargo test --locked -p nova-cli --test run_tests -- "$@"; }
  X() { cargo test --locked -p nova-driver --test index_complete "$@"; }
  T() { cargo test --locked -p nova-typeck --lib -- "$@"; }
  G() { NOVA_REQUIRE_CLANG=1 NOVA_CLANG=clang-18 cargo test --locked -p nova-cli --test run_tests -- release_golden_suite --nocapture "$@"; }
  ```

  So `W gate::try_operator` runs that WASM gate test, `R string_concat_runs`
  that test of `run_tests`, `X` the corpus test, `T check::lang_tests` the
  checker's tests of 5.0a's language, and `G` the golden suite through
  LLVM as CI's Linux leg runs it. `NOVA_HOME` keeps a run by hand off
  `~/.nova`; every test that needs one sets its own.
- **The build.** Each "Run" step below starts with

  ```sh
  cargo build --locked -p nova-cli -p nova-runtime
  ```

  which ends with `Finished`, in about 10 seconds after a change to the
  front end and a second otherwise, so that the `nova` the steps run by
  hand, and the runtime library beside it, are the tree's (Global
  Constraints). The replay began every step with it.
- **The patches.** Each task's code is in patch blocks below, each marked
  `<!-- patch NAME -->` and fenced with four backticks. They are the
  replay's commits, so apply them; do not retype them. From the
  repository's root:

  ```sh
  PLAN=docs/superpowers/plans/2026-10-27-phase-5-0a-the-small-language-and-ci.md   # this file, committed
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
  nothing: no 5.0a patch adds a line with trailing whitespace, a CR or a
  backslash-u escape, so `git apply` warns for none. The only non-ASCII
  characters the patches add are `§`, `—`, and `é`, Thai and Chinese text
  in `string_concat`'s program and `compound_assign_places`'s. A block that
  does not apply means the tree differs from the plan's: stop, and record
  why in the ledger.
- **Expected outputs are written once, from a run, and read.** Each
  `.stdout` under `tests/runtime/` that 5.0a adds is in its task's tests
  patch: the prototype wrote them from the program's native run, and each
  was read against the spec; the replay ran each under `nova run`, `nova
  build`, `--release` through `clang-18`, and WASM, and two under GC stress.
- **Each task** but Tasks 7 and 8 is test first: its tests' patch, their
  run and the expected failure, then its code's patch, its run and the
  expected pass. Task 7's patch is the golden suite and CI, a test and its
  job; Task 8's the records. Each task then runs its checks:
  - `cargo fmt --all -- --check`, which prints nothing;
  - the workspace's clippy, `cargo clippy --locked --all-targets
    --all-features -- -D warnings`, as CI runs it, which ends with
    `Finished` and no warning;
  - `cargo run --locked -q -p nova-cli -- fmt --check std examples
    tests/wasm tests/dom tests/browser/router-app tests/browser/ssr-app
    tests/ssr`, which prints nothing. 5.0a's four programs are under
    `tests/runtime`, which is not among them, as no program there is.

  It ends with one commit. A difference from an `Expected:` is fixed in
  the smallest way and recorded in the ledger as a ruling, with its cost if
  wrong.

## Review Focus

The inputs most likely to bite a user, most likely first. Each has its
check in the task that owns the code:

1. **The splits and their warning:** every place the rule ends a statement
   the parser before it would have continued must warn, and nowhere else:
   `record_split` at the postfix loop's `(` and `[`, the additive loop's
   `-`, and a bare `return` or `break` before a token an expression can
   begin with but a `let` or an item; `record_split` keeping one split a
   token, the first; `halt`, which keeps the line after a bare keyword
   from being its operand, cleared where the expression that holds it
   ends, in `parse_expr` and a view child; the split at an arm body's end
   token dropped, and the first error in the head of the arm it begins
   noted, after a bare keyword too; none where a `;` is written;
   `warn_newline_splits` merging into a "this value is not used" at the
   same place, else its own E0103; `File`'s `Debug` printing the field
   only when it is not empty; the resolver keeping the user modules'
   splits; fmt keeping the author's `;` before `(`, `[` and `-` and after
   a statement whose last code token is `return` or `break`, and adding
   none, and its self-check comparing the splits' kinds, not their places
   (decisions 9, 37 and 44 to 47; amended 2026-10-27 after the review) →
   the eleven split tests of
   `newline_rule.rs`, the six of `lang_tests.rs`, whose
   `a_split_after_a_return_value_warns` and `a_split_before_a_tail_warns`
   are the fact-check's three probes in small (Task 6), `source_rules.rs`
   (Tasks 2 and 6), the fingerprint's test in `nova-fmt`'s `check.rs`
   (Task 6), Task 9's run of `nova fmt` on the second check's
   probes, every repository `.nova` file warning nowhere (Task 9); mutants
   40 to 45 and 52 to 59. A
   split missed is a silent change of meaning; a split recorded where the
   old parser would have failed is a false warning: the reviewer reads
   each `record_split` and the arm's pop against both.
2. **The newline rule's contexts:** `stmt_ctx`, on in a block (any block,
   inside parentheses too) and in a match arm's body; off inside
   parentheses, brackets, call arguments, an index, record literals,
   interpolation holes, views, nested items, guards and the heads of `if`,
   `while`, `for` and `match`; saved and restored around each; the postfix
   loop's `(`/`[` arm and the additive loop's `-` arm, which consult it;
   `return` and `break`, which end at a line break in any context
   (decisions 3, 13, 14, 33 and 40) → `newline_rule.rs`'s 30 tests (Task
   1); every repository `.nova` file parsing to the same tree, spans
   included (Task 9); mutants 1 to 9, 48 and 49.
3. **The marks:** `Tokens::marked` reads the source between a token and
   the one before it, so a `//` comment, or a `/* */` one that holds a line
   break, marks the token after it, a lone `\r` counts, and a line break
   inside a string marks nothing; `prev_end` takes the furthest end seen,
   for the interpolation tokens whose spans nest; `line_break_before` is
   `false` past the end (decisions 12 and 41) → the lexer's seven tests
   (Task 1); mutants 10, 11 and 50. `Tokens` changes `lex`'s,
   `lex_with_comments`'s, `parse`'s and `parse_recovering`'s public types;
   two callers needed a change, `nova-doc` and `nova-fmt`'s `Source`
   (decision 12).
4. **fmt's layout under the rule:** `CONTINUES` without `(`, `[` and `-`;
   `VIEW_CONTINUES`, the old list, for a view's children, where the rule is
   off; a broken chain with a `-` anywhere in it breaking after every
   operator, in every context; a chain without `-`, and a flat chain, as
   before (decisions 9, 15 and 16) → `source_rules.rs` and the corpus test
   (Tasks 1 and 2); mutants 12 to 17. fmt's self-check compares the
   recorded splits' kinds, in order, though not their places, so a layout
   that adds or drops a split shows as a refusal to format, not as a wrong
   program, while one that only moves a split formats (decision 44; mutants
   54 and 55).
5. **`?`'s desugar and its codes:** `Option` and `Result` found by
   `$std.core`'s `DefId`s; the payload type; the early return re-wrapping
   `Err(x)` into the function's `Result`; the closure's return type, a
   fresh `Option`/`Result` when it is still a variable; E0098 with its
   note, a program's own `Option` named; E0099 with its notes, `.ok_or`
   only for an `Option` in a `Result` function, `main` and `@test` only
   outside closures, the function's own `Option` or `Result` named; E0099
   in a constant's initializer, but not in a closure inside one; E0010
   when the error types differ, with the `map_err` note (decisions 1, 6,
   17, 18, 38 and 39) → `lang_tests.rs`'s 21 `?` tests and two programs
   under four backends and GC stress (Task 3); mutants 18 to 24, 46 and
   47.
6. **A closure's `return`:** `check_closure` now swaps `fcx.ret_ty` for
   the closure's, and `fcx.entry` and `fcx.in_const` for `false`, and
   restores all three; a closure that relied on `return` checking against
   the enclosing function's type is now an error (decision 19) →
   `return_in_a_closure_checks_against_the_closures_type` (Task 3);
   mutants 23 and 24.
7. **`op=`'s single evaluation:** the array, the index and a field's
   receiver bound once to unscoped locals (`__place`, `__index`), the old
   value read through them, `compound_value` reusing
   `binary_result_ty`; the order receiver, index, old value, right-hand
   side, store; E0060 on an immutable root unchanged; `Int += String`
   E0010, not the spec's E0013 (decisions 20 and 34) → `lang_tests.rs`'s
   `+=` tests, the HIR counts of calls for an index and for a field's
   receiver, and `compound_assign_places`'s "evaluated 1 time(s)" lines
   (Tasks 4 and 5); mutants 25 to 27.
8. **`String + String`:** `binary_result_ty`'s `Add` arm, before the
   numeric one; MIR's `StrConcat` arm, before the integer path that would
   add two pointers; `String + Char` and `+ Int` E0010 with a note, not
   E0013 (decisions 7 and 21) → the IR test, `lang_tests.rs`, and
   `string_concat` under GC stress and WASM (Task 5); mutants 28 and 29.
9. **E0102 and E0103's forms:** E0102's list, the public variants of each
   std module globbed into user modules, the first module to export a name
   winning, `IMPORTED_STD` and `STD_INTERNAL` left out; reported at each
   variant of a sum in a module before `std_start`, the package's and its
   path dependencies'; E0103's forms, a parenthesised call, field, index,
   `?`, await or cast told by its span starting before its operand, and
   none on a tail, a call, an assignment, `&&`, `||`, a block, `if` or
   `match` (decisions 22 to 24) → the resolver's six tests, the CLI test,
   `lang_tests.rs`'s E0103 tests (Task 6); mutants 30 to 34.
10. **The golden suite and CI:** the walk (every `tests/runtime/*.nova`
    with a `.stdout`), the exclusion table checked against the files, the
    floor of 131, the stdin table, the empty status table, the 60-second
    limit, at most four threads, a `TMPDIR` a program, `NOVA_GC_STRESS`
    removed; the gate on `NOVA_REQUIRE_CLANG=1`, and the skip branch's
    assertion that the variable is not `1`; CI's matrix `include` on
    `ubuntu-24.04`, the Linux-only install step and its version assertion
    (decisions 4, 5, 25, 26 and 35) → Task 7's three runs, Task 9's run
    of CI's Linux leg, actionlint; mutants 35 to 39, 51 and 60. The
    version step runs on GitHub's runner alone (mutant 61, not run).
11. **The records:** ADR 0048 against the spec's §3.1 and the user's
    decisions; the CHANGELOG's entries; the dated notes of `00`, `11`,
    `12` (and its catalog's E0098, E0099, E0102 and E0103), `14`, `20`,
    `40`, `50` and `60`; the phase plan's amendment → Task 8, Step 2.

## Decisions

Made while writing the spec (its header and §12), prototyping, splitting
and replaying, each as built and each read against the code. Where one
departs from the spec, it says so and why, and is marked **(differs)**;
one that fills in what the spec left to the plan (its §13) says so,
unmarked. ADR 0048 records the scope, the backend and the pin; the
language items implement the master spec and take dated notes (spec
§12.1).

**The user's decisions, as built**

1. **`?` converts nothing; `map_err` does** (the user's decision 2, spec
   §3.3). `check_try` accepts `Option<T>` and `Result<T, E>` alone, and on
   a `Result` unifies the value's `E` with the function's; when they
   differ it reports E0010, "`?` cannot return an error of type `Int` from
   a function whose error type is `String`", with the note "`?` converts
   nothing: convert the error first, with `.map_err(|e| ...)?`" (mutant
   22). No backend changes: the desugar is HIR the backends already lower
   (decision 17).
2. **5.0a is the first of three pull requests** (the user's decision 1):
   ADR 0048, the golden suite through LLVM in CI, `?`, `+=` on fields and
   elements, the newline rule, `String + String`, and the two warnings
   E0102 and E0103 (spec §6), which the user's decision 8 and the spec's
   §3.5 put in 5.0a. Tuples, patterns and std wait for 5.0b and 5.0c.
3. **The newline rule is a rule of the parser** (the user's decision 5,
   spec §3.5). In statement context a line break ends the expression
   before a `(` (the postfix loop's arm), a `[` (the same arm) or a binary
   `-` (the additive loop's arm); a `return` or `break` at a line end takes
   no value, wherever it is (decision 14). `.` and every other operator
   continue the line, as before. The lexer marks the tokens; the parser
   reads the marks (decision 12) (mutants 1 to 3).
4. **`clang`: `ubuntu-24.04` and Ubuntu's `clang-18`, as a `cargo test`**
   (the user's decision 7, spec §3.2). `release_golden_suite` is a test of
   `run_tests.rs`; CI's Linux leg runs on `ubuntu-24.04`, installs
   `clang-18` with `apt-get`, asserts 18.1.3, and sets
   `NOVA_REQUIRE_CLANG=1` and `NOVA_CLANG=clang-18` (decision 26). Here
   `clang-18 --version` prints `Ubuntu clang version 18.1.3 (1ubuntu1)`,
   CI's pin, and the suite ran under it (Task 7).

**The spec's decisions (its §12), as built**

5. **The walk runs only under `NOVA_REQUIRE_CLANG=1`; CI's step asserts
   the version; only the `test` job's Linux leg moves** (spec §12.2 to
   §12.4). Without the variable the test prints `release golden: not run
   (set NOVA_REQUIRE_CLANG=1 to run it)` and passes; with it, a `clang`
   that does not run fails it, "NOVA_REQUIRE_CLANG=1, but `no-such-clang
   --version` does not run" (Task 7). `release_builds_and_runs_when_clang_available`
   is unchanged. The other jobs keep `ubuntu-latest`.
6. **`?` finds `Option` and `Result` by std/core's `DefId`s, and has two
   codes** (spec §12.5, §12.6). `core_sum` looks `Option` and `Result` up
   in `$std.core`'s exports, whatever the names mean in the module; a sum
   of the program's own named `Option` is E0098, with the note "this
   `Option<Int>` is the program's own type, not std's" (mutant 19). E0098
   is the operand's code, E0099 the function's.
7. **`String + String`** (the user's decision 8, spec §3.6).
   `binary_result_ty` gives `String` for `Add` on two `String`s, before
   the numeric arm; MIR lowers it to one `CallRuntime` of `StrConcat`,
   which every backend has, WASM's `rt_str_concat` among them; `s += t`
   follows on a local and, with Task 4, on a field or an element (mutants
   28 and 29). Each `+` copies both strings.
8. **E0102 and E0103 are warnings** (the user's decision 8, spec §3.5,
   §3.7, §12.7, §12.10): a program with either still builds and runs
   (`a_variant_that_hides_one_of_stds_warns_and_still_builds`, Task 6).
9. **fmt drops `(`, `[` and `-` from `CONTINUES`, and keeps the author's
   `;` where the rule would split.** **(differs)** Spec §12.8 has fmt drop
   the `;` before a statement that starts with `(`, `[` or `-`, and after
   a bare `return` or `break`. Since the user's ruling on the fact-check
   (decision 37), the rule warns at each such split unless a `;` is
   written there, so dropping it would make fmt's output warn and leave
   the author no way to silence it. `CONTINUES` is `{`, `*`, `&`, `|` and
   `||`, before which fmt prints `;` as before; before `SPLITS`, `(`, `[`
   and `-`, and after a statement whose last code token is `return` or
   `break` (a bare one, however deep: `let f = || return`, `x = break`),
   it prints `;` only where the source has one
   (`Source::semicolon_before`), and adds none, so the
   program's statements and its warnings are as they were; its
   self-check, which compares the recorded splits' kinds, in order, though
   not their places, holds it to that (decision 44; mutants 12, 13, 14, 45,
   54 and 55). An arm needs no `,` before a `(`, `[`
   or `-`: the split that begins the next arm is not recorded (decision
   37). The spec's "never starts a line with `-` in a broken chain" is
   decision 15. *Amended 2026-10-27 after the review:* fmt asked only
   whether the statement was itself a bare `return` or `break`, so it
   dropped the `;` after `let f = || return;` and then refused the file;
   it now reads the statement's last code token, as the base did. After
   such a statement, or a view child that ends so, it prints `;` only
   where the author wrote one, before `CONTINUES` and `VIEW_CONTINUES`
   too: the line break ends the keyword whatever follows, and a `;` fmt
   added before `return` / `{ p }` removed the split, and fmt refused the
   file (`a_nested_value_less_return_or_break_keeps_the_authors_semicolon`).
10. **The rule is off in parentheses, brackets, arguments, record literals
    and the heads of `if`, `while`, `for` and `match`; a `{` on a new line
    keeps today's meaning** (spec §12.9). `in_head_ctx` turns off record
    literals and the rule together for a head; a block anywhere turns the
    rule back on, inside parentheses too
    (`a_block_inside_parentheses_turns_the_rule_back_on`); a `{` is never
    a postfix, so its meaning is unchanged
    (`a_brace_on_a_new_line_keeps_its_meaning`). Decision 13 lists where
    else it is off (mutants 5, 6, 8 and 9).
11. **No file in the repository changes** (the user's decision 5). The
    replay dumped `nova parse --ast` for every `.nova` file the repository
    tracks with the base's `nova` and the last commit's: 279 files, the four
    new programs among them, every dump the same, spans included; the base's
    `nova` and the new one both exit non-zero on the same two files, the
    parser fixture `async.nova` and the language server's `complete.nova`
    fixture, which do not parse by design (Task 9).
    The prototype found `nova fmt --check`'s list the same before and
    after, and formatted output the same everywhere but one chain in
    `tests/runtime/http_serialise.nova`, which is not fmt-clean and where
    the old fmt printed a line starting with `- 2` (decision 15).

**Decisions the prototype made, read against the code**

12. **The marks live in `Tokens`, which `lex` returns.** **(differs)** Spec
    §13 left the means to the plan: a mark on each token, a table beside
    the tokens, or the source. `nova_lexer::Tokens` holds the tokens and a
    `Vec<bool>`, "a line break before", one per token, built by
    `Tokens::marked` from the source between each token and the furthest
    end before it, so a `//` comment, or a `/* */` comment that holds a
    line break, marks the token after it, and a line break inside a string
    marks nothing. It derefs to `[Spanned<Token>]` and iterates as the
    `Vec` did, so the 14 call sites of `parse` in 12 files compile
    unchanged, and `lex` is the one place that builds it, so no caller can
    leave the marks out. The departure: `lex`, `lex_with_comments`,
    `parse` and `parse_recovering` change their public types, which the
    spec's §8 does not list, and two callers change, `nova-doc`'s
    `FileText` (`tokens.into_vec()`) and `nova-fmt`'s `Source` (a `Tokens`
    field). `line_break_before` is `false` past the end. The 5.1 lexer
    port copies the mark (spec §3.5) (mutants 10 and 11).
13. **Where statement context is off, beyond the spec's list.**
    **(differs)** `stmt_ctx` is saved and restored like
    `no_struct_literal`. It is on for a block's statements and tail and for
    a match arm's body; off inside parentheses and tuples, array literals,
    call arguments, an index, record literals, interpolation holes, the
    heads of `if`, `while`, `for` and `match`, and, beyond spec §3.5's
    list, a match guard, which ends at its `=>` as a head ends at its `{`,
    a view's children (decision 16) and a nested item. The parser starts
    with it off, so an item's signature is never in it (mutants 4 to 9).
14. **`return` and `break` end at a line break in any context,** as spec
    §3.5 says ("wherever they are"), and still at `;`, `}` and the end of
    the file: `bare_keyword_ends` (mutant 3). Inside parentheses
    `(return\n x)` is a parse error, which
    `return_at_a_line_end_takes_no_value_inside_parentheses_too` pins.
15. **A broken chain with a `-` in it breaks after every operator, in
    every context.** **(differs)** Spec §3.5 asked that a broken chain
    never start a line with `-` in statement context, and §13 left the
    choice between the operator ending its line and parentheses. Built:
    when any operator of a chain of one precedence level is `-`, every
    break of that chain comes after its operator (`a +` / `b -` / `c`),
    in statement context and out of it; a chain without `-` breaks before
    each operator, as before; a flat chain is unchanged. One rule needs no
    context in the printer. Parentheses were rejected: fmt keeps an
    author's parentheses, so fmt's own would stick after the first run.
    It rides in Task 1 with the rule (decision 29) (mutants 16 and 17).
16. **A view's children keep their `;` rule.** Spec §3.5 is silent on
    views. The rule is off in `parse_view_children`, as in an element's
    arguments, and fmt prints a view child's `;` by `VIEW_CONTINUES`, the
    old list of eight, so a child that starts with `(` still takes a `;`
    before it (`a_view_child_starting_with_a_parenthesis_keeps_its_semicolon`,
    `a_view_keeps_its_rule`) and no view changes (mutant 15).
17. **`?`'s desugar.** As spec §3.3 says, `check_try` builds the flat
    `match` the `for` desugar builds: `match e { Some(v) => v, None =>
    return None }`, or `Ok(v) => v, Err(x) => return Err(x)` re-wrapped in
    the function's `Result`, through unscoped locals `__ok` and `__err`
    that no source name reaches; its type is the payload's. In a closure
    whose return type is still a variable, `?` unifies it with the same
    sum over fresh variables, so `Some(w.len())` fixes the argument later
    (`try_infers_a_closures_return_type`). An operand whose type is still a
    variable is E0098 with the note "its type is not known here; annotate
    it" (mutant 18).
18. **E0099's notes.** "the function returns `Int`, not an `Option`"; on an
    `Option` in a function that returns a `Result`, "to return an error
    when the value is `None`, write `.ok_or(err)?`"; in `main` or a
    `@test` function, "`main` and `@test` functions return `()`, so `?`
    cannot return from them". `FnCtx.entry` carries the last, set by the
    function's name or its `@test` attribute and turned off inside a
    closure (mutants 20, 21 and 24).
19. **`return` inside a closure checks against the closure's return
    type.** **(differs)** Spec §3.3 asks it of `?` ("the nearest `fn` or
    closure"). Before 5.0a a closure's `return` checked against the
    enclosing function's return type, so `|x: Int| -> Int { if x > 0 {
    return 1 } 2 }` inside `main` was E0010, "`return` value has type
    `Int` but the function returns `()`" (Task 3, Step 2). `check_closure`
    now puts the closure's return type in `fcx.ret_ty` while it checks the
    body, for `return` and `?` alike, and restores it. It is a fix beyond
    the spec, in the CHANGELOG's Fixed: code that relied on the old
    checking would now be refused; none in the repository does (mutant
    23).
20. **`op=` on a field or an element.** **(differs)** As spec §3.4 says: the
    receiver,
    and for an element the index, go into unscoped locals (`__place`,
    `__index`), then `t.f = t.f op e` or `a[k] = a[k] op e` stores, the
    old value read through them; `compound_value` builds `old op value`
    through `binary_result_ty`, so the errors are E0010 and E0013, and
    plain assignment's E0060 for an immutable root. The departure: spec
    §3.4 gives "those of `op` on the two types (E0013)", and `r.n += "x"`,
    an `Int` field and a `String`, is E0010, the operand types differing
    before the operator is looked at, as decision 21 has for `String +
    Int` (`compound_assignment_with_a_bad_operand_type_is_e0013_or_e0010`).
    The block is `let t
    = …; let k = …; set`, unit-typed. The three E0900 tests of
    `compound_assignment_to_*` now check that the form checks (Task 4)
    (mutants 25 to 27). The HIR test of the index is
    `compound_assignment_evaluates_the_index_once`, renamed from
    `…_the_receiver_once` after the fact-check.
21. **`String + Char` and `String + Int` stay E0010, with a note.**
    **(differs)** Spec §3.6 says they "stay E0013, as today". Today they
    are E0010, "mismatched operand types: `String` vs `Char`" (the base's
    `nova check`, read while this plan was written), since the operand
    types differ before the operator is looked at; `String - String` is
    E0013. 5.0a keeps E0010 and adds the note "`+` joins two `String`s; to
    add any other value to a string, interpolate it: `"${a}${b}"`"
    (`string_plus_char_or_int_is_refused_with_a_note`).
22. **E0102's list, and where it fires.** Spec §13 left the list and the
    place. `std_globbed_variants` collects, in the resolver after std's
    imports, the public variants of each std module whose names glob into
    user modules, all but `IMPORTED_STD`'s and `STD_INTERNAL`'s, the first
    module to export a name winning, as the glob does; `variant_shadowing`
    reports each variant of a sum in a module before `std_start`, the
    package's and its path dependencies'. A registry package's warnings
    are dropped by the driver already. The message: "variant `Number`
    hides std's `JsonValue::Number`", its note "in this module the bare
    name `Number` now means this variant; std's is `JsonValue::Number`".
    In the repository it fires twice in a user file, at the parser fixture
    `crates/nova-parser/tests/fixtures/sum_type.nova`'s own `Ok` and `Err`,
    which no test checks; and, 51 times, in six std sources checked by hand
    as user files (`nova check std/http/lib.nova`), which already fail at
    the base with E0001s, since std's builtins are not in scope there, and
    which no build checks so. Nothing is renamed (spec §10 item 8). One
    resolver test changed: `importing_a_std_core_name_from_a_module_is_allowed`
    now expects one E0102 for its `Ok`, and no error (mutants 30 and 31).
    The test of a second module is `another_modules_variant_is_reported_too`,
    renamed after the fact-check from `a_path_dependencys_…`: the resolver's
    tests have no path dependency; a path dependency's modules come before
    `std_start` the same way, and the driver shows their warnings.
23. **E0103's forms.** Spec §13 left the exact list. `unused_value`
    reports a statement, never a block's tail, that is a literal or an
    interpolated string, a name, a unary operator, a binary operator other
    than `&&` and `||`, a non-empty tuple, an array, or a parenthesised
    call, field, index, `?`, await or cast, told by its span starting
    before its operand. Assignments are statements of their own kind and
    never reach it; calls, blocks, `if`, `match` and loops are not
    reported (mutants 32 to 34).
24. **What E0103 reports at a line break, and where it fires.** The
    prototype's report said that `foo` then `(bar())` on the next line is
    flagged at `foo` alone; the replay found both flagged. `unused_value`
    alone misses `foo` then `(bar)()`, a call whose parenthesised callee's
    span starts at the `(` as the call's does; since decision 37 the split
    before `(bar)()` is reported too, so
    `e0103_reports_a_parenthesised_callee_after_a_line_break` counts two
    E0103s. "This value is not used" carries a note about line breaks only
    where a line break ended the statement before it (decision 37). In the
    repository it fires nowhere, the splits' warning included: no repository
    file has a split.
25. **The golden table.** **(differs)** Spec §3.2 listed ten exclusions,
    `io_streams` first, and let the plan give a simple setup instead.
    Built: `io_streams` runs in the walk with standard input `hello-stdin`
    (`RELEASE_GOLDEN_STDIN`), so nine are excluded, each with why:
    `process_args` (its `args()[0]` is the file `nova run` was given),
    `file_open_dir`, `fs_permission_denied`, the five `net_*` programs
    (`net_interleave`, `net_lifetime`, `net_refused`, `net_roundtrip`,
    `net_timeout`: their tests serve or reserve the port) and `nova_test`
    (a `@test` file). The table must name programs that exist, or the
    test fails, and since the fact-check the walk must find at least 131
    programs, today's count (mutant 51). The status table is empty: every
    program exits 0. A
    binary may run 60 seconds; at most four threads, `min(4, cores)`;
    each program gets its own `TMPDIR`, `TMP` and `TEMP`, so none races
    its `run` test's fixtures; `NOVA_GC_STRESS` is removed from its
    environment (spec §3.2). `net_listener_accept` passes alone, so it is
    in the walk (mutants 35 to 38). *Amended 2026-10-27 after the
    review:* `nova build --release` has the same 60 seconds, through
    `run_within_limit`, and a failure quotes the program's standard
    error, its last 2000 characters.
26. **CI's YAML.** Spec §13 left it. The matrix's `os` names
    `ubuntu-24.04` in place of `ubuntu-latest`, and an `include` gives that
    leg `require_clang: "1"` and `clang: clang-18`; the step "Install clang
    18.1.3 (Linux)", `if: runner.os == 'Linux'`, runs `apt-get install -y
    clang-18`, prints `clang-18 --version` and fails unless its first line
    holds `version 18.1.3`; the `cargo test` step sets
    `NOVA_REQUIRE_CLANG: ${{ matrix.require_clang || '0' }}` and
    `NOVA_CLANG: ${{ matrix.clang || 'clang' }}`. actionlint 1.7.7 checks
    the matrix keys the step reads (mutant 60); nothing here runs the step
    (mutant 61, not run).
27. **WASM: four programs join the gate, and no WASM code changes.**
    `try_operator`, `try_async`, `compound_assign_places` and
    `string_concat` are in `wasm_tests.rs`'s `gate!`, and pass under Node
    (Tasks 3 and 5); `try_async` and `string_concat` also run under
    `NOVA_GC_STRESS=1` (spec §7.3).
28. **The records** (spec §9): ADR 0048; the CHANGELOG's Unreleased, Added
    (`?`, `+=`, `String + String`, E0102, E0103), Changed (the newline
    rule, `nova fmt`, the lexer's `Tokens`, CI) and Fixed (decision 19);
    dated notes in `00-MASTER-SPEC.md`, `11-PARSER.md` §2 and §6,
    `12-TYPESYSTEM.md` with its §8 catalog's E0098, E0099, E0102 and E0103,
    `14-CODEGEN.md`, `20-STDLIB.md`, `40-TOOLING.md`, `50-TESTING.md` and
    `60-EXAMPLES.md`; and the phase plan's 5.0 amendment, three parts in
    place of 5.0a and 5.0b.

**The split's decisions**

29. **Task 1 holds fmt's broken chains, with the rule.** The natural split
    gives fmt a task of its own. With the rule and without the chains'
    layout, fmt's corpus test fails at Task 1's code:
    `every_corpus_file_formats_to_the_same_program_and_comments`, "1
    failures: tests/runtime/http_serialise.nova: Internal {
    first_difference: "the AST changed …" }", since the old fmt breaks a
    chain before its `-` and the rule reads that line as a new statement
    (spec §10 item 1: "fmt must change in the same pull request"). So
    `print/expr.rs`'s `binary` and the three chain tests of
    `source_rules.rs` ride in Task 1, and Task 2 is fmt's separators:
    `CONTINUES`, `VIEW_CONTINUES` and the bare `return`, with their tests.
    Every crate's tests pass after each task's code.
30. **The golden programs ride with their features.** `try_operator` and
    `try_async` come with `?` (Task 3). `compound_assign_places` uses `+=`
    on a `String` field, which needs `String + String`, and
    `string_concat` uses `+=` on a record's field, which needs Task 4: so
    both come with Task 5. Each program brings its `run_tests.rs` test,
    its `gate!` entry and `index_complete.rs`'s counts, which pass through
    167 programs and 201 files in Task 3 to 169 and 203 in Task 5; the
    comments of `wasm_tests.rs` and `index_complete.rs` say only what each
    task has added. `index_complete` fails before Task 3's code, since a
    corpus program must check cleanly.
31. **Tests that cannot stand first.** `lang_tests.rs` grows with each
    task, its module doc and its `Severity` import matched to what it
    holds, so no task leaves an unused helper for clippy. The lexer's tests
    name `Tokens` and `line_break_before`, so before Task 1's code they do
    not compile: six errors, E0425 and E0599 (Task 1, Step 2); nor do
    `newline_rule.rs`'s split tests before Task 6's code, `newline_splits`,
    `SplitKind` and `ParseError::notes` not existing (E0609, E0432,
    E0599). The fingerprint's unit test, in `nova-fmt`'s `check.rs`, rides
    in Task 6's code, its module being the code's. These tests pass before
    their code, each as it should, the base already doing what they pin:
    - Task 1: thirteen of `newline_rule.rs`'s,
      `a_brace_on_a_new_line_keeps_its_meaning`,
      `a_minus_at_a_line_end_continues`, `a_semicolon_still_ends_a_statement`,
      `a_view_keeps_its_rule`,
      `other_operators_and_dots_continue_the_line_before`,
      `the_rule_is_off_in_a_guard`,
      `the_rule_is_off_in_a_nested_items_signature_and_value`,
      `the_rule_is_off_in_an_if_head`,
      `the_rule_is_off_inside_a_record_literal`,
      `the_rule_is_off_inside_arguments`,
      `the_rule_is_off_in_an_interpolation_hole`,
      `the_rule_is_off_inside_parentheses` and
      `the_rule_is_off_inside_brackets`; and
      `a_broken_chain_without_a_minus_breaks_before_each_operator` and
      `a_flat_chain_with_a_minus_is_unchanged`;
    - Task 2: `a_view_child_starting_with_a_parenthesis_keeps_its_semicolon`;
    - Task 5: `string_minus_string_is_still_e0013`;
    - Task 6: `std_itself_is_never_reported`,
      `a_name_that_is_not_one_of_stds_variants_is_not_reported`,
      `e0103_is_not_on_calls_assignments_conditions_or_a_tail`,
      `a_written_semicolon_silences_the_split` and
      `an_unformatted_file_with_a_split_formats_keeping_it`, which no split
      can trouble before the code records one; the fingerprint's mutant
      fails it (decision 44).
32. **Tasks 7 and 8 are one patch each.** The golden suite is a test, and
    CI's change is its job; the records are text. `RELEASE_GOLDEN_*` and
    `run_release_binary` sit with `release_golden_suite`, after
    `release_builds_and_runs_when_clang_available`.

**The gap fix's decisions**

The mutants' first run, at the first split's last commit (`848211a`'s
tree), left three gaps, and
the replay found two records out of place and a wrong claim about E0103
(Checked on). The prototype's `f00c99b 5.0a prototype: close three test
gaps` closed them; each new test was shown failing with its mutant applied
and passing without it.

33. **A `match` inside parentheses keeps the rule in its arms.**
    `a_match_inside_parentheses_turns_the_rule_back_on_in_its_arms`
    (`newline_rule.rs`): `let v = (match x { A => g` / `(a)
    => h })` and the same inside `k(…)` with `[a]` parse as two arms. With
    a match arm's body inheriting its context, the `match` inside
    parentheses loses the rule, `g` / `(a)` becomes a call, and the test
    fails at the helper's parse (mutant 4).
34. **A field's receiver with a side effect is evaluated once.**
    `compound_assignment_on_a_field_evaluates_the_receiver_once`
    (`lang_tests.rs`): `rs[f()].n += 1`, whose receiver `rs[f()]` holds a
    call, has one `Call` in its HIR, as the index's test does. With the
    receiver checked again for the old value, it has two (mutant 27). It
    rides in Task 4's tests.
35. **Under `NOVA_REQUIRE_CLANG=1` the golden suite fails rather than
    skip.** `release_golden_suite` reads whether the variable is `1` before
    its gate, and its skip branch asserts that it is not: "NOVA_REQUIRE_CLANG=1,
    but release_golden_suite was about to skip". The assertion cannot fire
    while the gate's condition is right; it is there so that a gate that
    skips wrongly, by an edit to its condition, fails CI's Linux leg rather
    than leave it green (mutant 39). Without the variable the test still
    prints `not run` and passes (decision 5).
36. **Two records put back, and E0103's miss documented.**
    `check_closure`'s doc comment, which the insertion of `core_sum` and
    `check_try` had left above `core_sum`, is above its own function again;
    the three notes of `60-EXAMPLES.md` keep the spec's path on one line.
    The prototype's report said E0103 flags only `foo` in `foo` then
    `(bar())`; no record in the repository repeats it, and decision 24
    gives the true miss, which the gap fix pinned with
    `e0103_misses_a_call_whose_callee_is_parenthesised` (`x` then `(g)()`:
    one E0103); since decision 37 the split before `(g)()` is reported
    too, and the test is
    `e0103_reports_a_parenthesised_callee_after_a_line_break`,
    two E0103s, in Task 6's tests.

**The user's rulings on the fact-check (2026-10-27)**

A fact-check read this plan against the split at `777ad920` and found
eight things; the user ruled on two, and the prototype's `125d8f1 5.0a
prototype: the fact-check's fixes` answers all eight (Checked on).

37. **E0103 warns at every place the rule splits a statement the parser
    before it would have continued** (the user's ruling on finding 1: "warn
    at every point where the rule cuts a line"). The fact-check showed the
    rule changing a program's meaning in silence: the base's fmt broke
    `return gross - withholding` into `return gross` / `- withholding`, so
    such a file printed 1000 where it printed 900, and `nova check` was
    clean; so did a tail `r.a` / `- r.b`, and `make_adder(1)` / `(5)`.
    - **The parser records each split** in `ast::File.newline_splits`, a
      `NewlineSplit` of the token after the break and its `SplitKind`:
      `Call` and `Index` at the postfix loop's `(` and `[`, `Minus` at the
      additive loop's `-`, and `Value` at the line after a bare `return`
      or `break`, unless that line is a `let` or an item, which could not
      have been its value, or begins with a token no expression begins
      with, where the keyword reads as before the rule (decision 46). A
      `return` or `break` that a line break ends is no operand: nothing
      after the break calls, indexes or subtracts from it, so one split is
      recorded there, not two; and `record_split` keeps one split a token,
      the first, so `let f = || return` / `(5)` is one `Value` (decision
      45). The split at a match arm body's end token begins the next arm,
      and is dropped, with no warning. The parser before the rule could
      read that line into the body: `0 => g` / `(5)` / `_ => 1` parses at
      the base, as `g(5)`. But then the next arm cannot parse: it goes on
      only where a pattern and `=>` follow, and no expression the base
      read is followed by `=>`. So that file is now a P0001, at `_`, which
      names the rule (decision 47), and an arm that does parse there is
      one the base refused: no program changes its meaning in silence.
      *Amended 2026-10-27 after the review,* which found that this said
      the base could not read such a line into the body.
      `File`'s `Debug` prints the
      field only when it is not empty, so `nova parse --ast` prints a file
      the rule does not split as before, and Task 9's dump check holds.
      The resolver keeps the user modules' splits in the merged file.
    - **The checker warns at each** with E0103, after checking the
      functions (`warn_newline_splits`): "a line break ends the statement
      before this `-`" (or `(`, `[`), labelled "this `-` begins a new
      statement", or "a line break ends this `return` or `break`", with a
      note on how to continue the line and how to keep the break. Where
      the new statement is also a value nobody uses, the "this value is not
      used" there takes the note instead of a second warning.
    - **A `;` written at the end of the line before silences it:** the
      statement ends at the `;`, so the parser records no split. fmt keeps
      the author's `;` there and adds none (decision 9).
    - **E0103, not a new code.** The spec gives E0103 the job of reporting
      what the rule leaves (§3.5, §12.7), and the user's ruling names it;
      a second code would split one cause across two. The catalog's title
      becomes "This value is not used, or a line break ended a
      statement".
    - The fact-check's three probes each warn now, at their `-` and `(`;
      `lang_tests.rs` holds their like: `a_split_after_a_return_value_warns`
      (its `return a` / `- b`), and `a_split_before_a_tail_warns` (its
      `r.a` / `- r.b` and `k(1)` / `(5)`). The repository's `.nova` files
      warn nowhere: no file has a split, since every one parses to the tree
      the base gives (decision 11).
    - Tests: `newline_rule.rs`'s `each_split_the_rule_makes_is_recorded`,
      `a_written_semicolon_records_no_split` and
      `what_the_old_parser_could_not_have_continued_records_no_split`; and
      `lang_tests.rs`'s `a_split_after_a_return_value_warns`,
      `a_split_before_a_tail_warns`, `a_split_after_a_bare_return_warns`,
      `a_written_semicolon_silences_the_split` and
      `an_unused_value_at_a_split_gets_one_warning_with_the_line_breaks_note`
      (mutants 40 to 45, 52 and 53). The recording is Task 6's, with the
      warning, so
      Tasks 1 to 5 have the parser without it.
38. **`?` in a constant's initializer is an error** (the user's ruling on
    finding 4: "not allowed"). A constant was checked as a zero-argument
    function returning its type, so `const A: Option<Int> = Some(n()? + 1)`
    checked, and `A` was `None` at run time. `FnCtx.in_const`, set by
    `check_const` and turned off inside a closure, makes `check_try` report
    E0099, "`?` cannot return from this function", labelled "this `?` is in
    a constant's initializer", with the note "a constant is not a function:
    `?` works only in a `fn` or a closure; match on the value instead".
    E0099, not a new code: it is the code for a `?` with nowhere to
    return, and spec §3.3 says the enclosing function is the nearest `fn`
    or closure, which a constant's initializer has none of. A closure inside
    a constant may use `?` (`try_in_a_closure_inside_a_constant_checks`)
    (mutant 46).

**The fact-check's other fixes**

39. **E0099 names the program's own `Option` or `Result`.** A function
    returning its own `Option` gave "the function returns `Option<Int>`,
    not an `Option`"; it now adds E0098's note, "this `Option<Int>` is the
    program's own type, not std's"
    (`try_in_a_function_returning_its_own_option_says_so`, mutant 47).
40. **Two contexts tested.** `the_rule_is_off_in_an_interpolation_hole`
    and `the_rule_is_off_in_a_nested_items_signature_and_value`, whose
    contexts' `in_stmt_ctx(false, …)` taken out had passed every test
    (mutants 48 and 49). Both pass before Task 1's code, the base reading
    them the same way (decision 31).
41. **A lone `\r` is a line break.** `Tokens::marked` takes a `\r` as one,
    so `b` / `(d)` across an old Mac line ending is two statements, as
    across `\n` (`a_lone_carriage_return_is_a_line_break`, mutant 50).
42. **The records corrected:** the CHANGELOG no longer says nothing
    changes inside parentheses (`(return` / `x)` is now P0001: a `return`
    at a line end takes no value anywhere) and describes the warnings and
    fmt's `;`; ADR 0048's decision 4 and consequence, and `14-CODEGEN.md`'s
    note, say every golden program but the nine excluded; `11-PARSER.md`,
    `12-TYPESYSTEM.md`, `40-TOOLING.md` and `50-TESTING.md` say what the
    splits, the warning, fmt and the walk's floor do.
43. **Nits.** `parse_view`'s comment, which rustfmt had hung off the
    trailing comment of `self.advance();`, sits above that line. Two are
    recorded, not fixed: `|o:
    Option<Int>| o?` without a declared return type gives one E0010,
    rightly, then six E0011s for the closure's return type left partly
    unresolved, a cascade that needs inference to default after an error;
    and a trailing comment in a broken chain with `-` lands after the
    operator, which changes no program.

**The second check's decisions**

A second check read this plan against the split at `f8f8b61a` and found
three faults in the code, a test gap, four errors in the plan and six
nits; the prototype's `794e915 5.0a prototype: the second check's fixes`
answers them, and `77b6b2f 5.0a prototype: an arm drops a split it met
twice` the gap its mutants then found (Checked on).

44. **fmt's self-check compares the splits' kinds, not their places.** The
    fingerprint was the AST's `Debug` with its `, span: Span { … }` fields
    removed, but a split prints `at: Span { … }`, which stayed: any change
    of layout before a split made the program "differ", and fmt, and the
    language server's formatting, refused the file (exit 2). `fingerprint`
    now takes the splits out of the file and appends their kinds, in
    order. `an_unformatted_file_with_a_split_formats_keeping_it`
    (`source_rules.rs`) and `the_fingerprint_tells_splits_by_kind_not_by_place`
    (`check.rs`'s tests) pin it, in Task 6 with the splits (mutants 54 and
    55).
45. **A `return` or `break` that a line break ends is no operand.** After
    one, a next line that starts with `(`, `[` or `-` recorded a `Value`
    split, and then the postfix or additive loop a second, `Call`, `Index`
    or `Minus`, at the same token, with two contradictory notes; and in an
    arm, `0 => return` / `(1) => 2`, the arm dropped one and warned at the
    other, where nothing could silence it. `bare_keyword_ends` now sets
    `halt` at that token, where `peek` reads nothing, so no operator, call
    or index takes the keyword, until the expression that holds it ends
    (`parse_expr` clears it, and so does a view child, which is read
    without it); `record_split` keeps one split a token, the first
    recorded there: a `return`'s or a `break`'s value, or a closure's
    body, meets the split before the next line, and the expression around
    it meets it again, as `Value` then `Call` after `let f = || return` /
    `(5)`; and an arm drops the split at its end token
    (`a_split_after_a_bare_return_is_recorded_once`,
    `a_bare_return_ending_an_arm_records_no_split`,
    `a_value_ending_an_arm_records_no_split_however_often_it_is_met`,
    `a_closures_bare_return_before_a_parenthesis_or_bracket_is_one_split`,
    `a_bare_return_in_a_view_child_ends_the_child`, and `lang_tests.rs`'s
    `a_closures_bare_return_before_a_parenthesis_or_bracket_warns_once`;
    mutants 56 and 57). The third, `77b6b2f`'s, closed the gap mutant 57
    showed on `794e915`'s split, where an arm dropping only the last split
    passed every test. *Amended 2026-10-27 after the review:* the same
    split was met twice with two kinds, through a closure's body, and
    `parse_file`'s dedup kept both; and the halt outlived a view child,
    `view {` / `return` / `(1)` / `}` failing to parse after the child.
    Now one split stands at a token, and a view child ends the halt, so
    that view reads as two children with the rule's E0103 at `(`.
46. **Before a token no expression begins with, a bare keyword reads as
    before the rule.** `g(return` / `)` was accepted with a warning at `)`,
    where the base gives P0001. `can_start_expr` lists the tokens an
    expression can begin with; before any other, `)`, `]`, `,`, `=>` and
    the like, `bare_keyword_ends` takes the value as before the rule, a
    parse error, and records nothing
    (`a_bare_return_before_a_token_no_expression_starts_with_reads_as_before`;
    mutant 58). `return_at_a_line_end_takes_no_value_inside_parentheses_too`
    now pins `g(return` / `x)`, a P0001, where it pinned `g(return` / `)`.
47. **A P0001 at an arm the rule began names the rule.** `0 => a` / `- 1,`
    parsed at the base, and is now P0001, "expected pattern, found `-`", as
    spec §3.5 has it. The arm that ends at a split leaves it in
    `arm_end_split`; when the arm that begins at that token fails in its
    head (its pattern, its guard or its `=>`), the first error it gave,
    wherever it lands, becomes `ParseError::Noted`. After a split before
    `(`, `[` or `-` the note says "a line that starts with `(`, `[` or `-`
    begins the next match arm (the newline rule); to continue the arm's
    body, end the line before with the operator"; after a bare keyword's
    `Value`, "a `return` or `break` at the end of a line takes no value, so
    the next line begins the next match arm (the newline rule); to return
    that line's value, put it on the same line as the `return` or
    `break`". The four places that turn a parse error into P0001 print it
    (`a_parse_error_at_an_arm_the_rule_began_names_the_rule`,
    `a_parse_error_anywhere_in_an_arms_head_the_rule_began_names_the_rule`;
    mutant 59). An error in the body of an arm whose head parsed takes no
    note: the base would have met that `=>` after an expression, an error
    too. *Amended 2026-10-27 after the review:* the note went only to an
    error at the split's token and never after a `Value`, so `0 => g` /
    `(5)` / `_ => 1` (at `_`) and `None => return` / `0,` (at `,`) had
    none.
48. **The nits.** `SPLITS` has its own doc comment, `VIEW_CONTINUES` its
    own; until Task 6, fmt's words for `SPLITS` and `separator`, and
    `source_rules.rs`'s, do not name E0103, which Task 6 brings (the split
    script checks no file names it before Task 6); the CHANGELOG's line
    about a lone `\r` and one of `11-PARSER.md`'s are wrapped. **Recorded,
    not fixed: a lone `\r` in a diagnostic.** The parser takes a lone `\r`
    as a line break (decision 41), but a diagnostic renders such a file as
    one line: `codespan-reporting`'s `SimpleFiles`, which `render` builds,
    computes its own line starts at `\n` alone, so the fix is a `Files`
    implementation over `FileDb`, beyond 5.0a's change.


---

### Task 0: The baseline

The branch is `claude/loving-archimedes-frtf5q-50`, at this plan's commit,
whose parent is `1fc7af6`, the approved spec. The plan's commit adds this
file alone, so the patches apply to it as to `1fc7af6`.

- [ ] **Step 1: The tree.**
  - `git status --short` prints nothing, on
    `claude/loving-archimedes-frtf5q-50`.
  - `git diff --stat 1fc7af6 HEAD` lists only
    `docs/superpowers/plans/2026-10-27-phase-5-0a-the-small-language-and-ci.md`.
  - `BASE=$(git rev-parse --short HEAD)`: the plan's commit, recorded in
    the ledger. Task 9's review reads from it.
  - `df -h /` shows at least 8G free; if not, remove
    `target/debug/incremental` and older duplicate test executables in
    `target/debug/deps` first.
- [ ] **Step 2: The tools.**
  - `clang-18 --version | head -n 1` prints `Ubuntu clang version 18.1.3
    (1ubuntu1)`: CI's pin, for Task 7's suite.
  - `node --version` prints `v22.22.0`, for the WASM tests.
  - `cargo +1.78 --version` prints `cargo 1.78.0`, for Task 9.
  - `ls "$P/actionlint-dl/actionlint"`: actionlint 1.7.7, for Tasks 7 and
    9. Where it is missing, say so in the ledger: CI's `lint` job runs it.
- [ ] **Step 3: The suite.** In a shell set up as the conventions say:

  ```sh
  cargo build --locked --workspace
  NOVA_REQUIRE_NODE=1 cargo test --locked --workspace --no-fail-fast > "$P/base-50a.txt" 2>&1
  grep '^test result' "$P/base-50a.txt" | awk '{p += $4; f += $6; i += $8} END {print p, f, i}'
  grep ' FAILED$' "$P/base-50a.txt"
  cp target/debug/nova "$P/nova-base-50a"
  cargo test --locked --workspace -- --list > "$P/base-list-50a.txt" 2>&1
  ```

  Expected, from 4.8's last full run and not the replay's (below):
  `2420 1 10`, and one failure, `test
  net::tests::an_ipv6_connect_parks_on_its_first_poll_and_then_establishes
  ... FAILED`. It needs IPv6, which this machine lacks, as in every
  earlier phase. (`1fc7af6` is `main` after `v0.4.0`'s records, the phase
  plan and the spec, none of which changes code; these figures are 4.8's.
  The replay did not run the suite at `1fc7af6` again.) The copy of `nova`
  is the base's, for Task 9's AST check; the list, 2431 tests in 102
  binaries, is the base's names, for Task 9's comparison. Task 9 expects
  `2529 1 10`: 5.0a's tests make 109 more, 115 added and
  6 gone.
- [ ] **Step 4: The ledger.** Start `$P/ledger-50a.md`, headed with the
  base commit and the baseline's figures.

### Task 1: The lexer's marks, the newline rule, and fmt's broken chains

Spec §3.5; the user's decision 5; decisions 3, 10 to 15 and 29.

**Files:**
- Create: `crates/nova-lexer/src/tokens.rs` (`Tokens`)
- Modify:
  - `crates/nova-lexer/src/lib.rs` (`lex` and `lex_with_comments` return
    `Tokens`)
  - `crates/nova-parser/src/lib.rs`, `crates/nova-parser/src/grammar.rs`
    (`stmt_ctx`, the rule's arms, `bare_keyword_ends`)
  - `crates/nova-doc/src/signature.rs`, `crates/nova-fmt/src/source.rs`
    (`Tokens`' two callers)
  - `crates/nova-fmt/src/print/expr.rs` (`binary`: a chain with `-` breaks
    after each operator)
- Test:
  - `crates/nova-lexer/tests/lexer_tests.rs` (seven tests of the mark)
  - `crates/nova-parser/tests/newline_rule.rs` (new, 30 tests; Task 6 adds
    eight)
  - `crates/nova-fmt/tests/source_rules.rs` (three chain tests)

- [ ] **Step 1: The tests.** `patch_block t1-tests`. The patch holds:
  - `lexer_tests.rs`: `marks(src)`, each token's text and mark; a token
    after `\n` and `\r\n` marked; the first token marked only after a line
    break; a `//` comment marking the token after it; a `/* */` comment
    marking it only when it holds a line break; a line break inside a
    string marking nothing; a lone `\r` marking the token after it
    (decision 41); the marks leaving the tokens as they were,
    `lex` and `lex_with_comments` agreeing, and `false` past the end. The
    corpus test's closure takes `nova_lexer::Tokens`.
  - `newline_rule.rs`: a compact rendering of a body's statements and
    `errors(src)`, a body's parse errors, then
    the rule's cases: `(`, `[` and `-` at a line start in a block, a `let`,
    a tail and a match arm; a tuple-pattern arm after an arm's body; `-` at
    a line end and other operators and `.` continuing; the rule off inside
    parentheses, brackets, arguments, a record literal, an `if` head and a
    guard; a block inside parentheses turning it back on, and a `match`
    inside parentheses or arguments keeping it in its arms (decision 33); a
    `{` on a new
    line; the rule off in an interpolation hole and in a nested item
    (decision 40); `(` and `[` after the `}` of `if`, `match` and `while`;
    `return`
    and `break` at a line end, and in a match arm; `g(return` then `x)` an
    error, the `return` ending at the line break inside parentheses too;
    a line comment and a block comment holding a line break; `;`; and a
    view keeping its rule.
  - `source_rules.rs`: a broken chain with a `-` ending each line with its
    operator, as a `let`'s value, a tail and inside a call's parentheses;
    a chain without `-` breaking before each operator; a flat chain with a
    `-` unchanged.
- [ ] **Step 2: Run, and see the tests fail.** First the build
  (Conventions), which ends with `Finished`.
  - `cargo test --locked -p nova-lexer --test lexer_tests`. Expected: it
    does not compile, six errors, `error[E0425]: cannot find type `Tokens`
    in crate `nova_lexer``, at `lexer_tests.rs:626:40`, and five `error[E0599]:
    no method named `line_break_before` found for struct `Vec<Spanned<Token>>``
    (or `into_vec`), then `error: could not compile `nova-lexer` (test
    "lexer_tests") due to 6 previous errors`: the tests name what Step 3
    adds (decision 31).
  - `cargo test --locked -p nova-parser --test newline_rule`. Expected:
    `13 passed; 17 failed; 0 ignored; 0 measured; 0 filtered out`. The 17
    are the rule's own cases:
    `a_parenthesis_at_a_line_start_begins_a_statement`,
    `a_bracket_at_a_line_start_begins_a_statement`,
    `a_minus_at_a_line_start_begins_a_statement`,
    `the_rule_holds_in_a_let_value_and_a_tail`,
    `the_rule_holds_in_a_match_arm`,
    `a_tuple_pattern_arm_after_an_arm_body_begins_an_arm`,
    `a_block_inside_parentheses_turns_the_rule_back_on`,
    `a_match_inside_parentheses_turns_the_rule_back_on_in_its_arms`,
    `a_parenthesis_after_an_if_begins_a_statement`,
    `a_parenthesis_after_a_match_begins_a_statement`,
    `a_bracket_after_a_while_begins_a_statement`,
    `return_at_a_line_end_takes_no_value`,
    `break_at_a_line_end_takes_no_value`,
    `return_at_a_line_end_takes_no_value_inside_parentheses_too`,
    `a_bare_return_in_a_match_arm_ends_the_arm`,
    `a_line_comment_is_a_line_break` and
    `a_block_comment_that_holds_a_line_break_is_one`; some panic at the
    helper's `assert!(errors.is_empty(), …)` (`newline_rule.rs:23:5`), the
    base parsing the input another way or not at all. The 13 that pass are
    the cases the base already parses as the rule does: the rule off
    inside parentheses, brackets, arguments, a record literal, an `if`
    head, a guard, an interpolation hole and a nested item; `-` at a line
    end; other operators; a `{` on a new
    line; `;`; a view.
  - `cargo test --locked -p nova-fmt --test source_rules -- chain`.
    Expected: `2 passed; 1 failed; 0 ignored; 0 measured; 21 filtered
    out`: `a_broken_chain_with_a_minus_ends_each_line_with_its_operator`
    fails, the base breaking before each operator; the other two pass, as
    they should (decision 31).
- [ ] **Step 3: The code.** `patch_block t1-code`:
  - **`tokens.rs`** (new): `Tokens { tokens, line_break_before }`,
    `Tokens::marked(tokens, source)`, `line_break_before(index)`,
    `into_vec()`, `Deref` to the slice and both `IntoIterator`s; a line
    break is a `\n` or a lone `\r` (decisions 12 and 41).
  - **`nova-lexer`'s `lib.rs`:** `mod tokens; pub use tokens::Tokens;`;
    `lex` and `lex_with_comments` return `Tokens::marked(tokens, source)`.
  - **`grammar.rs`:** the `stmt_ctx` field, off at the start;
    `in_stmt_ctx`, `in_head_ctx`, `line_break_here`, `newline_ends_expr`
    and `bare_keyword_ends`; `parse_block` turning the rule on; a nested
    item, call arguments, an index, parentheses (now `parse_paren`), array
    literals (now `parse_array_literal`), an interpolation hole, a guard,
    a view's children (its comment above `self.advance()`) and record
    literals turning it off; the heads of
    `if`, `while`, `for` and `match` through `in_head_ctx`; a match arm's
    body turning it on; the postfix loop's `Token::LParen |
    Token::LBracket if self.newline_ends_expr() => break` and the additive
    loop's `Token::Minus if self.newline_ends_expr() => break`; `return`
    and `break` asking `bare_keyword_ends` (decisions 3, 10, 13 and 14).
  - **`nova-parser`'s `lib.rs`:** `parse` and `parse_recovering` take
    `&Tokens`.
  - **`nova-doc`'s `signature.rs`:** `tokens: tokens.into_vec()`;
    **`nova-fmt`'s `source.rs`:** `Source.tokens: Tokens`.
  - **`print/expr.rs`:** `binary` computes `after`, whether any operator
    of the chain is `-`, and then pushes ` op` and a line break in place of
    a line break and `op ` (decision 15).
- [ ] **Step 4: Run.** First the build, which ends with `Finished`, about
  10 seconds.
  - `cargo test --locked -p nova-lexer --test lexer_tests`. Expected:
    `42 passed; 0 failed`.
  - `cargo test --locked -p nova-parser --test newline_rule`. Expected:
    `30 passed; 0 failed`.
  - `cargo test --locked -p nova-fmt --test source_rules -- chain`.
    Expected: `3 passed; 0 failed; 0 ignored; 0 measured; 21 filtered out`.
  - `cargo test --locked --no-fail-fast -p nova-lexer -p nova-parser -p
    nova-fmt -p nova-doc` (about 13 s). Expected: every binary `ok`, none
    failed: `nova-doc`'s 61, `nova-fmt`'s corpus test `1 passed`, its
    `source_rules` 24, `nova-lexer`'s 42,
    `nova-parser`'s `newline_rule` 30 and parser tests 56. Without Step
    3's `print/expr.rs`, the corpus test fails here on
    `tests/runtime/http_serialise.nova`, "the AST changed" (decision 29).
- [ ] **Step 5: The checks,** as the conventions list them.
- [ ] **Step 6: Commit**
  `lexer, parser: the newline rule, and fmt's chains with a minus`.

**Patch `t1-tests`:**

<!-- patch t1-tests -->

````diff
diff --git a/crates/nova-fmt/tests/source_rules.rs b/crates/nova-fmt/tests/source_rules.rs
index a7a3602ed19105c83f1ba116b236e6f0d358ac88..419d1408987b1ed2a3bb233b0ded24cde1f5c764 100644
--- a/crates/nova-fmt/tests/source_rules.rs
+++ b/crates/nova-fmt/tests/source_rules.rs
@@ -169,6 +169,51 @@ fn a_value_less_return_keeps_its_semicolon() {
     );
 }
 
+/// A broken chain with a `-` in it breaks after each operator, so no line
+/// starts with `-`, which the newline rule would read as a new statement
+/// (5.0 plan's decision); the output parses to the same tree, which
+/// `format`'s self-check asserts.
+#[test]
+fn a_broken_chain_with_a_minus_ends_each_line_with_its_operator() {
+    let a = "first_operand_of_the_chain";
+    let b = "second_operand_of_the_chain";
+    let c = "third_operand_of_the_chain";
+    let d = "fourth_operand_of_the_chain";
+    assert_formats(
+        &format!("fn f() -> Int {{ let x = {a} + {b} - {c} + {d}\n x }}\n"),
+        &format!("fn f() -> Int {{\n    let x = {a} +\n        {b} -\n        {c} +\n        {d}\n    x\n}}\n"),
+    );
+    // As a tail, and as a statement whose value is discarded.
+    assert_formats(
+        &format!("fn f() -> Int {{ {a} - {b} - {c} - {d} }}\n"),
+        &format!("fn f() -> Int {{\n    {a} -\n        {b} -\n        {c} -\n        {d}\n}}\n"),
+    );
+    // Inside parentheses too: the rule is one rule.
+    assert_formats(
+        &format!("fn f() -> Int {{ g({a} - {b} - {c} - {d}) }}\n"),
+        &format!("fn f() -> Int {{\n    g(\n        {a} -\n            {b} -\n            {c} -\n            {d},\n    )\n}}\n"),
+    );
+}
+
+/// A chain with no `-` still breaks before each operator (spec §6).
+#[test]
+fn a_broken_chain_without_a_minus_breaks_before_each_operator() {
+    let a = "first_operand_of_the_chain";
+    let b = "second_operand_of_the_chain";
+    let c = "third_operand_of_the_chain";
+    let d = "fourth_operand_of_the_chain";
+    assert_formats(
+        &format!("fn f() -> Int {{ {a} + {b} + {c} + {d} }}\n"),
+        &format!("fn f() -> Int {{\n    {a}\n        + {b}\n        + {c}\n        + {d}\n}}\n"),
+    );
+}
+
+/// A flat chain with a `-` is laid out as before.
+#[test]
+fn a_flat_chain_with_a_minus_is_unchanged() {
+    assert_stable("fn f(a: Int, b: Int) -> Int { a - b + 1 }\n");
+}
+
 #[test]
 fn width_counts_characters_not_bytes() {
     // 4 + `report("` + 86 Thai letters + `")` is exactly 100 characters, and
diff --git a/crates/nova-lexer/tests/lexer_tests.rs b/crates/nova-lexer/tests/lexer_tests.rs
index 31614044111e0a997efaedd47cd6c313c7aa6145..40344d9bc7f1fd90a33e6562b6c1a9951d6f7727 100644
--- a/crates/nova-lexer/tests/lexer_tests.rs
+++ b/crates/nova-lexer/tests/lexer_tests.rs
@@ -623,7 +623,7 @@ fn lex_and_lex_with_comments_agree_on_every_nova_file() {
         let file = db.add("<corpus>", text.as_str());
         let (plain, _) = lex(&text, file);
         let (with, _, _) = lex_with_comments(&text, file);
-        let pairs = |toks: Vec<nova_diagnostics::Spanned<Token>>| {
+        let pairs = |toks: nova_lexer::Tokens| {
             toks.into_iter()
                 .map(|t| (t.value, t.span))
                 .collect::<Vec<_>>()
@@ -631,3 +631,82 @@ fn lex_and_lex_with_comments_agree_on_every_nova_file() {
         assert_eq!(pairs(plain), pairs(with), "{}", path.display());
     }
 }
+
+// --- The line-break mark (5.0 spec §3.5) ---
+
+/// Each token's mark, by its source text, for `src`, `Eof` left out.
+fn marks(src: &str) -> Vec<(String, bool)> {
+    let mut db = FileDb::new();
+    let file = db.add("<marks>", src);
+    let (tokens, errors) = lex(src, file);
+    assert!(errors.is_empty(), "{errors:?}");
+    tokens
+        .iter()
+        .enumerate()
+        .filter(|(_, t)| t.value != Token::Eof)
+        .map(|(i, t)| {
+            let text = src[t.span.start as usize..t.span.end as usize].to_string();
+            (text, tokens.line_break_before(i))
+        })
+        .collect()
+}
+
+#[test]
+fn a_token_after_a_line_break_is_marked() {
+    let m = marks("a\n(b)\r\n  - c");
+    let want: Vec<(String, bool)> = [
+        ("a", false),
+        ("(", true),
+        ("b", false),
+        (")", false),
+        ("-", true),
+        ("c", false),
+    ]
+    .iter()
+    .map(|(t, b)| (t.to_string(), *b))
+    .collect();
+    assert_eq!(m, want);
+}
+
+#[test]
+fn the_first_token_is_marked_only_after_a_line_break() {
+    assert!(!marks("x")[0].1);
+    assert!(marks("\n\nx")[0].1);
+}
+
+#[test]
+fn a_line_comment_marks_the_token_after_it() {
+    let m = marks("a // note\nb");
+    assert_eq!(m[1], ("b".to_string(), true));
+}
+
+#[test]
+fn a_block_comment_marks_the_token_after_it_only_when_it_holds_a_line_break() {
+    assert_eq!(marks("a /* x\n y */ b")[1], ("b".to_string(), true));
+    assert_eq!(marks("a /* x */ b")[1], ("b".to_string(), false));
+}
+
+#[test]
+fn a_line_break_inside_a_string_marks_nothing() {
+    let m = marks("f(\"a\nb\") (c)");
+    assert!(m.iter().all(|(_, b)| !b), "{m:?}");
+}
+
+#[test]
+fn the_marks_leave_the_tokens_as_they_were() {
+    let src = "fn main() {\n    let x = 1\n    -x\n}\n";
+    let mut db = FileDb::new();
+    let file = db.add("<marks>", src);
+    let (tokens, _) = lex(src, file);
+    let (with, _, _) = lex_with_comments(src, file);
+    assert_eq!(tokens.len(), with.len());
+    let plain: Vec<_> = tokens.clone().into_vec();
+    assert_eq!(plain.len(), tokens.len());
+    assert!((0..tokens.len()).all(|i| tokens.line_break_before(i) == with.line_break_before(i)));
+    assert!(!tokens.line_break_before(tokens.len() + 5));
+}
+
+#[test]
+fn a_lone_carriage_return_is_a_line_break() {
+    assert_eq!(marks("b\r(d)")[1], ("(".to_string(), true));
+}
diff --git a/crates/nova-parser/tests/newline_rule.rs b/crates/nova-parser/tests/newline_rule.rs
new file mode 100644
index 0000000000000000000000000000000000000000..9d691d916862fd15db500743fdb81a9268cb247c
--- /dev/null
+++ b/crates/nova-parser/tests/newline_rule.rs
@@ -0,0 +1,485 @@
+//! The newline rule (spec
+//! `docs/superpowers/specs/2026-10-26-phase-5-0-the-language-and-std-a-compiler-needs-design.md`
+//! §3.5): in statement context, a line break ends the expression before a
+//! `(`, a `[` or a binary `-`; a `return` or `break` at the end of a line
+//! takes no value. Each test renders a function body's statements in a
+//! compact form that leaves the spans out.
+
+use nova_ast::expr::{BinOp, Expr, Literal, StringPart, UnOp};
+use nova_ast::pattern::Pattern;
+use nova_ast::{Block, Item, Stmt};
+use nova_diagnostics::FileDb;
+use nova_lexer::lex;
+use nova_parser::parse;
+
+/// The statements and tail of `f`'s body, in `src`, one string each; a
+/// tail is marked `tail:`. Panics on a lex or parse error.
+fn body(src: &str) -> Vec<String> {
+    let mut db = FileDb::new();
+    let id = db.add("newline.nova", src);
+    let (tokens, lex_errors) = lex(src, id);
+    assert!(lex_errors.is_empty(), "{lex_errors:?}");
+    let (file, errors) = parse(&tokens, id);
+    assert!(errors.is_empty(), "{src}\n{errors:?}");
+    let file = file.expect("a file");
+    let f = file
+        .items
+        .iter()
+        .find_map(|i| match &i.value {
+            Item::Function(f) => Some(f),
+            _ => None,
+        })
+        .expect("a function");
+    block(&f.body.value)
+}
+
+fn block(b: &Block) -> Vec<String> {
+    let mut out: Vec<String> = b
+        .stmts
+        .iter()
+        .map(|s| match &s.value {
+            Stmt::Let { pattern, init, .. } => format!(
+                "let {} = {}",
+                pat(&pattern.value),
+                init.as_ref().map_or("-".to_string(), |e| show(&e.value))
+            ),
+            Stmt::Expr(e) => show(&e.value),
+            Stmt::Item(_) => "item".to_string(),
+        })
+        .collect();
+    if let Some(t) = &b.trailing {
+        out.push(format!("tail:{}", show(&t.value)));
+    }
+    out
+}
+
+fn pat(p: &Pattern) -> String {
+    match p {
+        Pattern::Wildcard => "_".to_string(),
+        Pattern::Ident { name, .. } => name.value.clone(),
+        Pattern::Tuple(ps) => format!(
+            "({})",
+            ps.iter()
+                .map(|p| pat(&p.value))
+                .collect::<Vec<_>>()
+                .join(", ")
+        ),
+        Pattern::Lit(Literal::Int(n)) => n.to_string(),
+        Pattern::Array(ps) => format!(
+            "[{}]",
+            ps.iter()
+                .map(|p| pat(&p.value))
+                .collect::<Vec<_>>()
+                .join(", ")
+        ),
+        Pattern::TupleStruct { path, fields } => format!(
+            "{}({})",
+            path.segments
+                .iter()
+                .map(|s| s.value.as_str())
+                .collect::<Vec<_>>()
+                .join("::"),
+            fields
+                .iter()
+                .map(|p| pat(&p.value))
+                .collect::<Vec<_>>()
+                .join(", ")
+        ),
+        other => format!("{other:?}"),
+    }
+}
+
+fn list(xs: &[nova_diagnostics::Spanned<Expr>]) -> String {
+    xs.iter()
+        .map(|x| show(&x.value))
+        .collect::<Vec<_>>()
+        .join(", ")
+}
+
+fn show(e: &Expr) -> String {
+    match e {
+        Expr::Lit(Literal::Int(n)) => n.to_string(),
+        Expr::Lit(Literal::Bool(b)) => b.to_string(),
+        Expr::Path(p) => p
+            .segments
+            .iter()
+            .map(|s| s.value.as_str())
+            .collect::<Vec<_>>()
+            .join("::"),
+        Expr::Tuple(xs) => format!("({})", list(xs)),
+        Expr::Array(xs) => format!("[{}]", list(xs)),
+        Expr::Block(b) => format!("{{{}}}", block(b).join("; ")),
+        Expr::If { cond, then, else_ } => format!(
+            "if {} {{{}}}{}",
+            show(&cond.value),
+            block(&then.value).join("; "),
+            else_
+                .as_ref()
+                .map_or(String::new(), |e| format!(" else {}", show(&e.value)))
+        ),
+        Expr::Match { scrutinee, arms } => format!(
+            "match {} {{{}}}",
+            show(&scrutinee.value),
+            arms.iter()
+                .map(|a| format!(
+                    "{}{} => {}",
+                    pat(&a.pattern.value),
+                    a.guard
+                        .as_ref()
+                        .map_or(String::new(), |g| format!(" if {}", show(&g.value))),
+                    show(&a.body.value)
+                ))
+                .collect::<Vec<_>>()
+                .join(", ")
+        ),
+        Expr::While { cond, body } => format!(
+            "while {} {{{}}}",
+            show(&cond.value),
+            block(&body.value).join("; ")
+        ),
+        Expr::Return(v) => match v {
+            Some(v) => format!("return {}", show(&v.value)),
+            None => "return".to_string(),
+        },
+        Expr::Break(v) => match v {
+            Some(v) => format!("break {}", show(&v.value)),
+            None => "break".to_string(),
+        },
+        Expr::Record { path, fields, .. } => format!(
+            "{} {{{}}}",
+            path.segments
+                .iter()
+                .map(|s| s.value.as_str())
+                .collect::<Vec<_>>()
+                .join("::"),
+            fields
+                .iter()
+                .map(|f| format!(
+                    "{}: {}",
+                    f.name.value,
+                    f.value.as_ref().map_or("-".to_string(), |v| show(&v.value))
+                ))
+                .collect::<Vec<_>>()
+                .join(", ")
+        ),
+        Expr::Binary { op, lhs, rhs } => {
+            let o = match op {
+                BinOp::Add => "+",
+                BinOp::Sub => "-",
+                BinOp::Mul => "*",
+                BinOp::Or => "||",
+                BinOp::Lt => "<",
+                _ => "op",
+            };
+            format!("({} {o} {})", show(&lhs.value), show(&rhs.value))
+        }
+        Expr::Unary { op, expr } => {
+            let o = match op {
+                UnOp::Neg => "-",
+                UnOp::Not => "!",
+                _ => "u",
+            };
+            format!("{o}{}", show(&expr.value))
+        }
+        Expr::Call { callee, args } => format!("{}({})", show(&callee.value), list(args)),
+        Expr::Index { target, index } => {
+            format!("{}[{}]", show(&target.value), show(&index.value))
+        }
+        Expr::Field { target, field } => format!("{}.{}", show(&target.value), field.value),
+        Expr::Closure { body, .. } => format!("|..| {}", show(&body.value)),
+        Expr::Assign { lhs, rhs, .. } => format!("{} = {}", show(&lhs.value), show(&rhs.value)),
+        Expr::Try(x) => format!("{}?", show(&x.value)),
+        Expr::StringInterp(parts) => format!(
+            "\"{}\"",
+            parts
+                .iter()
+                .map(|p| match p {
+                    StringPart::Lit(s) => s.clone(),
+                    StringPart::Expr(e) => format!("${{{}}}", show(&e.value)),
+                })
+                .collect::<String>()
+        ),
+        other => format!("{other:?}"),
+    }
+}
+
+fn f(stmts: &str) -> String {
+    format!("fn f() {{\n{stmts}\n}}\n")
+}
+
+/// The errors of `f`'s body `stmts`.
+fn errors(stmts: &str) -> Vec<nova_parser::ParseError> {
+    let src = f(stmts);
+    let mut db = FileDb::new();
+    let id = db.add("errors.nova", src.as_str());
+    let (tokens, _) = lex(&src, id);
+    parse(&tokens, id).1
+}
+
+// --- `(`, `[` and `-` at a line start, in statement context ---
+
+#[test]
+fn a_parenthesis_at_a_line_start_begins_a_statement() {
+    assert_eq!(body(&f("g\n(x)")), ["g", "tail:x"]);
+    assert_eq!(body(&f("g\n(x, y)")), ["g", "tail:(x, y)"]);
+    // On the same line, it is a call, as before.
+    assert_eq!(body(&f("g(x)")), ["tail:g(x)"]);
+}
+
+#[test]
+fn a_bracket_at_a_line_start_begins_a_statement() {
+    assert_eq!(body(&f("g\n[1, 2]")), ["g", "tail:[1, 2]"]);
+    assert_eq!(body(&f("a[0]")), ["tail:a[0]"]);
+}
+
+#[test]
+fn a_minus_at_a_line_start_begins_a_statement() {
+    assert_eq!(body(&f("let z = x\n-y")), ["let z = x", "tail:-y"]);
+    assert_eq!(body(&f("let z = x - y")), ["let z = (x - y)"]);
+}
+
+#[test]
+fn other_operators_and_dots_continue_the_line_before() {
+    assert_eq!(body(&f("let z = x\n+ y")), ["let z = (x + y)"]);
+    assert_eq!(body(&f("let z = x\n* y")), ["let z = (x * y)"]);
+    assert_eq!(body(&f("let z = x\n|| y")), ["let z = (x || y)"]);
+    assert_eq!(
+        body(&f("let s = parts\n    .iter()\n    .count()")),
+        ["let s = parts.iter().count()"]
+    );
+    assert_eq!(body(&f("let v = x\n?")), ["let v = x?"]);
+}
+
+#[test]
+fn a_minus_at_a_line_end_continues() {
+    assert_eq!(body(&f("let z = x -\n    y")), ["let z = (x - y)"]);
+    assert_eq!(body(&f("let z = x -\n    -y")), ["let z = (x - -y)"]);
+}
+
+#[test]
+fn the_rule_holds_in_a_let_value_and_a_tail() {
+    assert_eq!(body(&f("let a = g\n(h)")), ["let a = g", "tail:h"]);
+    assert_eq!(body(&f("g\n-1")), ["g", "tail:-1"]);
+}
+
+#[test]
+fn the_rule_holds_in_a_match_arm() {
+    assert_eq!(
+        body(&f("match x {\n    A => g\n    (a) => h\n}")),
+        ["tail:match x {A => g, a => h}"]
+    );
+    assert_eq!(
+        body(&f("match x {\n    A => g\n    [a] => h\n}")),
+        ["tail:match x {A => g, [a] => h}"]
+    );
+}
+
+#[test]
+fn a_tuple_pattern_arm_after_an_arm_body_begins_an_arm() {
+    assert_eq!(
+        body(&f("match p {\n    (0, b) => b\n    (a, b) => a\n}")),
+        ["tail:match p {(0, b) => b, (a, b) => a}"]
+    );
+}
+
+// --- where the rule is off ---
+
+#[test]
+fn the_rule_is_off_inside_parentheses() {
+    assert_eq!(body(&f("let z = (x\n-y)")), ["let z = (x - y)"]);
+    assert_eq!(body(&f("let z = (g\n(1))")), ["let z = g(1)"]);
+    assert_eq!(body(&f("let t = (g\n(1), 2)")), ["let t = (g(1), 2)"]);
+}
+
+#[test]
+fn the_rule_is_off_inside_brackets() {
+    assert_eq!(
+        body(&f("let a = [x\n-y, g\n(1)]")),
+        ["let a = [(x - y), g(1)]"]
+    );
+    assert_eq!(body(&f("let a = b[i\n-1]")), ["let a = b[(i - 1)]"]);
+}
+
+#[test]
+fn the_rule_is_off_inside_arguments() {
+    assert_eq!(body(&f("h(x\n-y, g\n[0])")), ["tail:h((x - y), g[0])"]);
+}
+
+#[test]
+fn the_rule_is_off_inside_a_record_literal() {
+    assert_eq!(
+        body(&f("let r = R { a: x\n-y, b: g\n(1) }")),
+        ["let r = R {a: (x - y), b: g(1)}"]
+    );
+}
+
+#[test]
+fn the_rule_is_off_in_an_if_head() {
+    assert_eq!(
+        body(&f("if x\n-y < 0 { g() }")),
+        ["tail:if ((x - y) < 0) {tail:g()}"]
+    );
+    assert_eq!(
+        body(&f("while g\n(1) { h() }")),
+        ["tail:while g(1) {tail:h()}"]
+    );
+    assert_eq!(
+        body(&f("match g\n(1) { _ => 0 }")),
+        ["tail:match g(1) {_ => 0}"]
+    );
+}
+
+#[test]
+fn the_rule_is_off_in_a_guard() {
+    assert_eq!(
+        body(&f("match x {\n    a if a\n    -1 < 0 => a\n    _ => 0\n}")),
+        ["tail:match x {a if ((a - 1) < 0) => a, _ => 0}"]
+    );
+}
+
+#[test]
+fn a_block_inside_parentheses_turns_the_rule_back_on() {
+    assert_eq!(
+        body(&f("let v = g({\n    h\n    (1)\n})")),
+        ["let v = g({h; tail:1})"]
+    );
+}
+
+#[test]
+fn a_match_inside_parentheses_turns_the_rule_back_on_in_its_arms() {
+    // An arm's body is statement context wherever the `match` is: a line
+    // starting with `(` or `[` after a body begins the next arm.
+    assert_eq!(
+        body(&f("let v = (match x {\n    A => g\n    (a) => h\n})")),
+        ["let v = match x {A => g, a => h}"]
+    );
+    assert_eq!(
+        body(&f("k(match x {\n    A => g\n    [a] => h\n})")),
+        ["tail:k(match x {A => g, [a] => h})"]
+    );
+}
+
+#[test]
+fn a_brace_on_a_new_line_keeps_its_meaning() {
+    assert_eq!(body(&f("let r = R\n{ a: 1 }")), ["let r = R {a: 1}"]);
+}
+
+// --- after the `}` of `if`, `match` and `while` ---
+
+#[test]
+fn a_parenthesis_after_an_if_begins_a_statement() {
+    assert_eq!(
+        body(&f("let w = if c { 1 } else { 2 }\n(a, b)")),
+        ["let w = if c {tail:1} else {tail:2}", "tail:(a, b)"]
+    );
+}
+
+#[test]
+fn a_parenthesis_after_a_match_begins_a_statement() {
+    assert_eq!(
+        body(&f("match x { _ => g }\n(a)")),
+        ["match x {_ => g}", "tail:a"]
+    );
+}
+
+#[test]
+fn a_bracket_after_a_while_begins_a_statement() {
+    assert_eq!(
+        body(&f("while c { g() }\n[1]")),
+        ["while c {tail:g()}", "tail:[1]"]
+    );
+}
+
+// --- `return` and `break` ---
+
+#[test]
+fn return_at_a_line_end_takes_no_value() {
+    assert_eq!(body(&f("return\ncleanup()")), ["return", "tail:cleanup()"]);
+    assert_eq!(body(&f("return x")), ["tail:return x"]);
+    assert_eq!(body(&f("return g(\n    1)")), ["tail:return g(1)"]);
+}
+
+#[test]
+fn break_at_a_line_end_takes_no_value() {
+    assert_eq!(
+        body(&f("while c {\n    break\n    g()\n}")),
+        ["tail:while c {break; tail:g()}"]
+    );
+    assert_eq!(
+        body(&f("while c { break x }")),
+        ["tail:while c {tail:break x}"]
+    );
+}
+
+#[test]
+fn return_at_a_line_end_takes_no_value_inside_parentheses_too() {
+    // `g(return` then `x)`: the `return` ends at the line break, so `x`
+    // cannot follow it in the arguments.
+    assert!(!errors("g(return\n    x)").is_empty());
+}
+
+#[test]
+fn a_bare_return_in_a_match_arm_ends_the_arm() {
+    assert_eq!(
+        body(&f("match x {\n    A => return\n    B => 1\n}")),
+        ["tail:match x {A => return, B => 1}"]
+    );
+}
+
+// --- comments ---
+
+#[test]
+fn a_line_comment_is_a_line_break() {
+    assert_eq!(body(&f("g // note\n(x)")), ["g", "tail:x"]);
+}
+
+#[test]
+fn a_block_comment_that_holds_a_line_break_is_one() {
+    assert_eq!(body(&f("g /* a\n b */ (x)")), ["g", "tail:x"]);
+    // One that holds none is not.
+    assert_eq!(body(&f("g /* a */ (x)")), ["tail:g(x)"]);
+}
+
+#[test]
+fn a_semicolon_still_ends_a_statement() {
+    assert_eq!(body(&f("g; (x)")), ["g", "tail:x"]);
+    assert_eq!(body(&f("let z = x; -y")), ["let z = x", "tail:-y"]);
+}
+
+#[test]
+fn a_view_keeps_its_rule() {
+    // In a view's children, a line break is not significant: a `;` ends a
+    // child before a `(`, as before the rule.
+    let src = "fn f() -> Html { view { g\n(x) } }\n";
+    let mut db = FileDb::new();
+    let id = db.add("v.nova", src);
+    let (tokens, _) = lex(src, id);
+    let (file, errors) = parse(&tokens, id);
+    assert!(errors.is_empty(), "{errors:?}");
+    let file = file.expect("a file");
+    let Item::Function(func) = &file.items[0].value else {
+        panic!("a function")
+    };
+    let Some(tail) = &func.body.value.trailing else {
+        panic!("a tail")
+    };
+    let Expr::View(children) = &tail.value else {
+        panic!("a view")
+    };
+    assert_eq!(children.len(), 1);
+}
+
+#[test]
+fn the_rule_is_off_in_an_interpolation_hole() {
+    assert_eq!(
+        body(&f("let s = \"${a\n(b)} and ${c\n-d}\"")),
+        ["let s = \"${a(b)} and ${(c - d)}\""]
+    );
+}
+
+#[test]
+fn the_rule_is_off_in_a_nested_items_signature_and_value() {
+    // A nested `const`'s value continues across a line break, as a
+    // top-level one does; its body-less item is not statement context.
+    assert_eq!(body(&f("const C: Int = a\n-1\nC")), ["item", "tail:C"]);
+}
````

**Patch `t1-code`:**

<!-- patch t1-code -->

````diff
diff --git a/crates/nova-doc/src/signature.rs b/crates/nova-doc/src/signature.rs
index 68bc93e9ac8f7be1c661c9c79084fc0e3c5ce34b..3286d90d7afb9b6a524b20f81abe9e4f5881cb2b 100644
--- a/crates/nova-doc/src/signature.rs
+++ b/crates/nova-doc/src/signature.rs
@@ -21,7 +21,7 @@ impl<'a> FileText<'a> {
         let (tokens, comments, _) = nova_lexer::lex_with_comments(text, file);
         FileText {
             text,
-            tokens,
+            tokens: tokens.into_vec(),
             comments,
         }
     }
diff --git a/crates/nova-fmt/src/print/expr.rs b/crates/nova-fmt/src/print/expr.rs
index 1c0caef37cb95a052d3c4d2d9e51c7a43a88d7eb..ae39740104e15631f196d718f3d3aa6b15db7e20 100644
--- a/crates/nova-fmt/src/print/expr.rs
+++ b/crates/nova-fmt/src/print/expr.rs
@@ -673,18 +673,32 @@ impl<'s, 't> Printer<'s, 't> {
 
     /// A chain of one precedence level: flat when it fits, otherwise broken
     /// before each operator, indented once (spec §6, "Binary expressions").
+    ///
+    /// A chain with a `-` in it breaks after each operator instead, so no
+    /// line starts with `-`: in statement context the newline rule would
+    /// read that line as a new statement (5.0 spec §3.5). The rule is the
+    /// same in every context, so the printer need not know which one it is
+    /// in, and it adds no token, as parentheses around the chain would.
     fn binary(&mut self, e: &Spanned<Expr>) -> Doc {
         let Expr::Binary { op, .. } = &e.value else {
             unreachable!("binary is only called on a binary expression")
         };
         let mut operands = Vec::new();
         self.flatten(e, precedence(*op), true, &mut operands);
+        let after = operands
+            .iter()
+            .any(|&(op, _)| matches!(op, Some(BinOp::Sub)));
         let first = self.expr(operands[0].1);
         let mut rest = Vec::new();
         for &(op, x) in &operands[1..] {
             let op = op.expect("every operand after the first has an operator");
-            rest.push(Doc::Line);
-            rest.push(text(format!("{} ", bin_op(op))));
+            if after {
+                rest.push(text(format!(" {}", bin_op(op))));
+                rest.push(Doc::Line);
+            } else {
+                rest.push(Doc::Line);
+                rest.push(text(format!("{} ", bin_op(op))));
+            }
             rest.push(self.expr(x));
         }
         group(concat(vec![first, nest(concat(rest))]))
diff --git a/crates/nova-fmt/src/source.rs b/crates/nova-fmt/src/source.rs
index ad94b04bf04df47a30b4a87e29a2fbc4fde96cba..5a7a6ec04185fedef66db0774aa9f6036d18cf99 100644
--- a/crates/nova-fmt/src/source.rs
+++ b/crates/nova-fmt/src/source.rs
@@ -4,8 +4,8 @@
 //! (§5.3).
 
 use nova_ast::File;
-use nova_diagnostics::{render, Diagnostic, FileDb, Span, Spanned};
-use nova_lexer::{Comment, Token};
+use nova_diagnostics::{render, Diagnostic, FileDb, Span};
+use nova_lexer::{Comment, Token, Tokens};
 
 use crate::FormatError;
 
@@ -14,7 +14,7 @@ use crate::FormatError;
 pub(crate) struct Source<'t> {
     pub text: &'t str,
     pub file: File,
-    pub tokens: Vec<Spanned<Token>>,
+    pub tokens: Tokens,
     pub comments: Vec<Comment>,
 }
 
diff --git a/crates/nova-lexer/src/lib.rs b/crates/nova-lexer/src/lib.rs
index 02fd5d45623acacdd3536bfd15a9330a4857c217..fe9a9fb6188e1fcbdc791da5084ed5d47f831349 100644
--- a/crates/nova-lexer/src/lib.rs
+++ b/crates/nova-lexer/src/lib.rs
@@ -1,6 +1,7 @@
 //! Lexer for the Nova programming language.
 //!
-//! Converts Nova source text into a `Vec<Spanned<Token>>`. Uses `logos` for
+//! Converts Nova source text into [`Tokens`]: the tokens, each marked with
+//! whether a line break comes before it. Uses `logos` for
 //! the core token recognition and a hand-written state machine wrapper for
 //! string interpolation (`"Hello, ${name}!"`).
 //!
@@ -24,9 +25,11 @@
 
 mod error;
 mod token;
+mod tokens;
 
 pub use error::LexError;
 pub use token::Token;
+pub use tokens::Tokens;
 
 /// Every keyword, in the order the lexer declares its tokens: the words
 /// completion offers, and the extension's grammar colours (spec
@@ -45,22 +48,20 @@ use nova_diagnostics::{FileId, Span, Spanned};
 ///
 /// Errors are non-fatal: the lexer continues after a bad character and collects
 /// all errors in the returned `Vec`.
-pub fn lex(source: &str, file: FileId) -> (Vec<Spanned<Token>>, Vec<LexError>) {
+pub fn lex(source: &str, file: FileId) -> (Tokens, Vec<LexError>) {
     let mut lexer = Lexer::new(source, file);
-    lexer.tokenize()
+    let (tokens, errors) = lexer.tokenize();
+    (Tokens::marked(tokens, source), errors)
 }
 
 /// [`lex`], plus every comment it skipped, in source order: what the
 /// formatter needs to keep them (spec
 /// `docs/superpowers/specs/2026-10-07-phase-3-1-formatter-design.md` §3.1).
 /// The tokens and errors are exactly `lex`'s.
-pub fn lex_with_comments(
-    source: &str,
-    file: FileId,
-) -> (Vec<Spanned<Token>>, Vec<Comment>, Vec<LexError>) {
+pub fn lex_with_comments(source: &str, file: FileId) -> (Tokens, Vec<Comment>, Vec<LexError>) {
     let mut lexer = Lexer::new(source, file);
     let (tokens, errors) = lexer.tokenize();
-    (tokens, lexer.comments, errors)
+    (Tokens::marked(tokens, source), lexer.comments, errors)
 }
 
 /// A comment the lexer skipped.
diff --git a/crates/nova-lexer/src/tokens.rs b/crates/nova-lexer/src/tokens.rs
new file mode 100644
index 0000000000000000000000000000000000000000..8c55e8201df905882b5585209451fd4cd9c80f4e
--- /dev/null
+++ b/crates/nova-lexer/src/tokens.rs
@@ -0,0 +1,88 @@
+//! [`Tokens`]: the lexer's output, each token marked with whether a line
+//! break comes before it (spec
+//! `docs/superpowers/specs/2026-10-26-phase-5-0-the-language-and-std-a-compiler-needs-design.md`
+//! §3.5).
+
+use std::ops::Deref;
+
+use nova_diagnostics::Spanned;
+
+use crate::Token;
+
+/// The tokens of one source, in order, ending with [`Token::Eof`], and for
+/// each one whether a line break comes before it.
+///
+/// The parser reads the marks for the newline rule: in statement context, a
+/// line break ends an expression before a `(`, a `[` or a binary `-`, and a
+/// `return` or `break` at the end of a line takes no value. A line break
+/// inside a comment between two tokens counts, so a `//` comment, or a `/*
+/// */` comment that holds one, marks the token after it. A line break is a
+/// `\n`, or a lone `\r`, an old Mac line ending.
+///
+/// It reads as a slice of the tokens, so code that only walks the tokens
+/// needs no change; [`lex`](crate::lex) is the one place that builds it, so
+/// no caller of the parser can leave the marks out.
+#[derive(Debug, Clone, Default)]
+pub struct Tokens {
+    tokens: Vec<Spanned<Token>>,
+    line_break_before: Vec<bool>,
+}
+
+impl Tokens {
+    /// `tokens`, lexed from `source`, each marked by the source text between
+    /// it and the token before it (or the start of the source).
+    pub fn marked(tokens: Vec<Spanned<Token>>, source: &str) -> Self {
+        let bytes = source.as_bytes();
+        let mut prev_end = 0usize;
+        let line_break_before = tokens
+            .iter()
+            .map(|t| {
+                let start = (t.span.start as usize).min(bytes.len());
+                let gap = bytes.get(prev_end.min(start)..start).unwrap_or(&[]);
+                prev_end = prev_end.max(t.span.end as usize);
+                gap.iter().any(|&b| b == b'\n' || b == b'\r')
+            })
+            .collect();
+        Self {
+            tokens,
+            line_break_before,
+        }
+    }
+
+    /// Whether a line break comes before the token at `index`. `false` past
+    /// the end.
+    pub fn line_break_before(&self, index: usize) -> bool {
+        self.line_break_before.get(index).copied().unwrap_or(false)
+    }
+
+    /// The tokens alone.
+    pub fn into_vec(self) -> Vec<Spanned<Token>> {
+        self.tokens
+    }
+}
+
+impl Deref for Tokens {
+    type Target = [Spanned<Token>];
+
+    fn deref(&self) -> &[Spanned<Token>] {
+        &self.tokens
+    }
+}
+
+impl<'a> IntoIterator for &'a Tokens {
+    type Item = &'a Spanned<Token>;
+    type IntoIter = std::slice::Iter<'a, Spanned<Token>>;
+
+    fn into_iter(self) -> Self::IntoIter {
+        self.tokens.iter()
+    }
+}
+
+impl IntoIterator for Tokens {
+    type Item = Spanned<Token>;
+    type IntoIter = std::vec::IntoIter<Spanned<Token>>;
+
+    fn into_iter(self) -> Self::IntoIter {
+        self.tokens.into_iter()
+    }
+}
diff --git a/crates/nova-parser/src/grammar.rs b/crates/nova-parser/src/grammar.rs
index f594f919a7f8a62e59bf73542831b9099b9e35f7..7b83d9bf9fe302e1eb8b20800c77ffc51fe8a489 100644
--- a/crates/nova-parser/src/grammar.rs
+++ b/crates/nova-parser/src/grammar.rs
@@ -18,7 +18,7 @@ use nova_ast::{
     Block, File, Item, Path, Stmt,
 };
 use nova_diagnostics::{FileId, Span, Spanned};
-use nova_lexer::Token;
+use nova_lexer::{Token, Tokens};
 
 use crate::ParseError;
 
@@ -32,7 +32,7 @@ const MISPLACED_DOC: &str =
 // ---------------------------------------------------------------------------
 
 struct Parser<'a> {
-    tokens: &'a [Spanned<Token>],
+    tokens: &'a Tokens,
     pos: usize,
     file: FileId,
     errors: Vec<ParseError>,
@@ -40,6 +40,15 @@ struct Parser<'a> {
     /// literal. Set by scrutinee positions (if/while/for/match conditions) to
     /// avoid ambiguity with the following `{ block }`.
     no_struct_literal: bool,
+    /// Whether the parser is in statement context, where the newline rule
+    /// holds (spec
+    /// `docs/superpowers/specs/2026-10-26-phase-5-0-the-language-and-std-a-compiler-needs-design.md`
+    /// §3.5): a line break ends the expression before a `(`, a `[` or a
+    /// binary `-`. On for a block's statements and tail and a match arm's
+    /// body; off inside parentheses, brackets, arguments, record literals,
+    /// interpolation holes, views, items, and the heads of `if`, `while`,
+    /// `for` and `match`. Saved and restored like `no_struct_literal`.
+    stmt_ctx: bool,
     /// Count of closing `>` "borrowed" from a `>>` token while closing nested
     /// generic argument lists (`Option<Option<Int>>`). The lexer glues `>>`
     /// into one token, so closing an inner list splits it: one `>` closes the
@@ -53,13 +62,14 @@ struct Parser<'a> {
 }
 
 impl<'a> Parser<'a> {
-    fn new(tokens: &'a [Spanned<Token>], file: FileId) -> Self {
+    fn new(tokens: &'a Tokens, file: FileId) -> Self {
         Self {
             tokens,
             pos: 0,
             file,
             errors: Vec::new(),
             no_struct_literal: false,
+            stmt_ctx: false,
             pending_gt: 0,
             dropped: Vec::new(),
         }
@@ -77,15 +87,51 @@ impl<'a> Parser<'a> {
     /// Run `f` with `no_struct_literal = false`, then restore the previous
     /// value. Used for a `${…}` interpolation hole: the ambiguity that motivates
     /// the flag cannot arise there, because the hole's own `}` is a distinct
-    /// `InterpClose` token and no `{ block }` can follow it.
+    /// `InterpClose` token and no `{ block }` can follow it. The hole is
+    /// delimited, so it is not statement context either.
     fn in_struct_ok_ctx<R>(&mut self, f: impl FnOnce(&mut Self) -> R) -> R {
         let prev = self.no_struct_literal;
         self.no_struct_literal = false;
-        let result = f(self);
+        let result = self.in_stmt_ctx(false, f);
         self.no_struct_literal = prev;
         result
     }
 
+    /// Run `f` for the head of an `if`, `while`, `for` or `match`, which
+    /// ends at its `{`: no record literal, and not statement context.
+    fn in_head_ctx<R>(&mut self, f: impl FnOnce(&mut Self) -> R) -> R {
+        self.in_no_struct_ctx(|p| p.in_stmt_ctx(false, f))
+    }
+
+    /// Run `f` with `stmt_ctx = on`, then restore the previous value.
+    fn in_stmt_ctx<R>(&mut self, on: bool, f: impl FnOnce(&mut Self) -> R) -> R {
+        let prev = self.stmt_ctx;
+        self.stmt_ctx = on;
+        let result = f(self);
+        self.stmt_ctx = prev;
+        result
+    }
+
+    /// Whether a line break comes before the token at the cursor.
+    fn line_break_here(&self) -> bool {
+        self.tokens.line_break_before(self.pos)
+    }
+
+    /// Whether the newline rule ends the expression before the token at the
+    /// cursor: a line break before it, in statement context. The callers
+    /// ask it of a `(`, a `[` and a binary `-`.
+    fn newline_ends_expr(&self) -> bool {
+        self.stmt_ctx && self.line_break_here()
+    }
+
+    /// Whether a `return` or `break` just read takes no value: the next
+    /// token ends a statement or a block, or a line break comes before it,
+    /// in any context.
+    fn bare_keyword_ends(&self) -> bool {
+        matches!(self.peek(), Token::Semicolon | Token::RBrace | Token::Eof)
+            || self.line_break_here()
+    }
+
     // --- Token access ---
 
     fn peek(&self) -> &Token {
@@ -325,15 +371,12 @@ impl<'a> Parser<'a> {
 // Public entry point
 // ---------------------------------------------------------------------------
 
-pub(crate) fn parse_file(
-    tokens: &[Spanned<Token>],
-    file: FileId,
-) -> (Option<File>, Vec<ParseError>) {
+pub(crate) fn parse_file(tokens: &Tokens, file: FileId) -> (Option<File>, Vec<ParseError>) {
     let parsed = parse_recovering(tokens, file);
     (Some(parsed.file), parsed.errors)
 }
 
-pub(crate) fn parse_recovering(tokens: &[Spanned<Token>], file: FileId) -> crate::Parsed {
+pub(crate) fn parse_recovering(tokens: &Tokens, file: FileId) -> crate::Parsed {
     let mut p = Parser::new(tokens, file);
     let ast_file = p.parse_file();
     crate::Parsed {
@@ -1415,6 +1458,11 @@ impl<'a> Parser<'a> {
 
 impl<'a> Parser<'a> {
     fn parse_block(&mut self, ctx: &str) -> Option<Spanned<Block>> {
+        // A block anywhere turns statement context on (spec §3.5).
+        self.in_stmt_ctx(true, |p| p.parse_block_inner(ctx))
+    }
+
+    fn parse_block_inner(&mut self, ctx: &str) -> Option<Spanned<Block>> {
         let start = self.peek_span();
         self.expect(&Token::LBrace, ctx)?;
         let mut stmts = Vec::new();
@@ -1479,7 +1527,7 @@ impl<'a> Parser<'a> {
 
         // Nested items, documented or not (spec §4)
         if self.is_item_start_at(self.docs_end()) {
-            let item = self.try_parse_item()?;
+            let item = self.in_stmt_ctx(false, |p| p.try_parse_item())?;
             let span = item.span;
             return Some(Spanned::new(Stmt::Item(Box::new(item.value)), span));
         }
@@ -1761,6 +1809,9 @@ impl<'a> Parser<'a> {
         loop {
             let op = match self.peek() {
                 Token::Plus => BinOp::Add,
+                // A `-` after a line break in statement context begins the
+                // next statement (spec §3.5).
+                Token::Minus if self.newline_ends_expr() => break,
                 Token::Minus => BinOp::Sub,
                 _ => break,
             };
@@ -1883,18 +1934,24 @@ impl<'a> Parser<'a> {
         loop {
             let cur_span = expr.span;
             match self.peek().clone() {
+                // A `(` or `[` after a line break in statement context
+                // begins the next statement (spec §3.5).
+                Token::LParen | Token::LBracket if self.newline_ends_expr() => break,
                 Token::LParen => {
                     // Function call
                     self.advance();
-                    let mut args = Vec::new();
-                    while !self.check(&Token::RParen) && !self.is_at_end() {
-                        if let Some(a) = self.parse_expr(ctx) {
-                            args.push(a);
-                        }
-                        if self.eat(&Token::Comma).is_none() {
-                            break;
+                    let args = self.in_stmt_ctx(false, |p| {
+                        let mut args = Vec::new();
+                        while !p.check(&Token::RParen) && !p.is_at_end() {
+                            if let Some(a) = p.parse_expr(ctx) {
+                                args.push(a);
+                            }
+                            if p.eat(&Token::Comma).is_none() {
+                                break;
+                            }
                         }
-                    }
+                        args
+                    });
                     self.expect(&Token::RParen, "call arguments")?;
                     let end = self
                         .tokens
@@ -1912,7 +1969,7 @@ impl<'a> Parser<'a> {
                 Token::LBracket => {
                     // Index
                     self.advance();
-                    let index = self.parse_expr(ctx)?;
+                    let index = self.in_stmt_ctx(false, |p| p.parse_expr(ctx))?;
                     self.expect(&Token::RBracket, "index expression")?;
                     let end = self
                         .tokens
@@ -2022,84 +2079,10 @@ impl<'a> Parser<'a> {
             }
 
             // Parenthesised / tuple
-            Token::LParen => {
-                self.advance();
-                if self.eat(&Token::RParen).is_some() {
-                    // Unit `()`
-                    let end = self
-                        .tokens
-                        .get(self.pos.saturating_sub(1))
-                        .map(|s| s.span)
-                        .unwrap_or(start);
-                    return Some(Spanned::new(Expr::Tuple(vec![]), start.merge(end)));
-                }
-                let first = self.parse_expr(ctx)?;
-                if self.eat(&Token::RParen).is_some() {
-                    // Just a parenthesised expression
-                    let end = self
-                        .tokens
-                        .get(self.pos.saturating_sub(1))
-                        .map(|s| s.span)
-                        .unwrap_or(start);
-                    return Some(Spanned::new(first.value, start.merge(end)));
-                }
-                // Tuple
-                let mut elems = vec![first];
-                while self.eat(&Token::Comma).is_some() && !self.check(&Token::RParen) {
-                    if let Some(e) = self.parse_expr(ctx) {
-                        elems.push(e);
-                    }
-                }
-                self.expect(&Token::RParen, "tuple")?;
-                let end = self
-                    .tokens
-                    .get(self.pos.saturating_sub(1))
-                    .map(|s| s.span)
-                    .unwrap_or(start);
-                Some(Spanned::new(Expr::Tuple(elems), start.merge(end)))
-            }
+            Token::LParen => self.in_stmt_ctx(false, |p| p.parse_paren(start, ctx)),
 
             // Array literal — either `[a, b, c]` or the repeat form `[init; n]`
-            Token::LBracket => {
-                self.advance();
-                let mut elems = Vec::new();
-                let mut first = true;
-                while !self.check(&Token::RBracket) && !self.is_at_end() {
-                    if let Some(e) = self.parse_expr(ctx) {
-                        // `[init; n]`: a `;` after the *first* element switches
-                        // to the repeat form, whose second operand is a length
-                        // rather than another element.
-                        if first && self.eat(&Token::Semicolon).is_some() {
-                            let len = self.parse_expr(ctx)?;
-                            self.expect(&Token::RBracket, "repeat array literal")?;
-                            let end = self
-                                .tokens
-                                .get(self.pos.saturating_sub(1))
-                                .map(|s| s.span)
-                                .unwrap_or(start);
-                            return Some(Spanned::new(
-                                Expr::ArrayRepeat {
-                                    init: Box::new(e),
-                                    len: Box::new(len),
-                                },
-                                start.merge(end),
-                            ));
-                        }
-                        elems.push(e);
-                    }
-                    first = false;
-                    if self.eat(&Token::Comma).is_none() {
-                        break;
-                    }
-                }
-                self.expect(&Token::RBracket, "array literal")?;
-                let end = self
-                    .tokens
-                    .get(self.pos.saturating_sub(1))
-                    .map(|s| s.span)
-                    .unwrap_or(start);
-                Some(Spanned::new(Expr::Array(elems), start.merge(end)))
-            }
+            Token::LBracket => self.in_stmt_ctx(false, |p| p.parse_array_literal(start, ctx)),
 
             // Block expression
             Token::LBrace => {
@@ -2117,7 +2100,7 @@ impl<'a> Parser<'a> {
             // While
             Token::While => {
                 self.advance();
-                let cond = self.in_no_struct_ctx(|p| p.parse_expr(ctx))?;
+                let cond = self.in_head_ctx(|p| p.parse_expr(ctx))?;
                 let body = self.parse_block(ctx)?;
                 let span = start.merge(body.span);
                 Some(Spanned::new(
@@ -2134,7 +2117,7 @@ impl<'a> Parser<'a> {
                 self.advance();
                 let pattern = self.parse_pattern(ctx)?;
                 self.expect(&Token::In, "for loop")?;
-                let iter = self.in_no_struct_ctx(|p| p.parse_expr(ctx))?;
+                let iter = self.in_head_ctx(|p| p.parse_expr(ctx))?;
                 let body = self.parse_block(ctx)?;
                 let span = start.merge(body.span);
                 Some(Spanned::new(
@@ -2150,8 +2133,8 @@ impl<'a> Parser<'a> {
             // Return
             Token::Return => {
                 self.advance();
-                // Optional expression
-                let val = if !matches!(self.peek(), Token::Semicolon | Token::RBrace | Token::Eof) {
+                // Optional expression; none at the end of a line (spec §3.5).
+                let val = if !self.bare_keyword_ends() {
                     Some(Box::new(self.parse_expr(ctx)?))
                 } else {
                     None
@@ -2163,7 +2146,7 @@ impl<'a> Parser<'a> {
             // Break
             Token::Break => {
                 self.advance();
-                let val = if !matches!(self.peek(), Token::Semicolon | Token::RBrace | Token::Eof) {
+                let val = if !self.bare_keyword_ends() {
                     Some(Box::new(self.parse_expr(ctx)?))
                 } else {
                     None
@@ -2222,6 +2205,87 @@ impl<'a> Parser<'a> {
         }
     }
 
+    /// `()`, `(e)` or a tuple, at `(`, which began at `start`.
+    fn parse_paren(&mut self, start: Span, ctx: &str) -> Option<Spanned<Expr>> {
+        self.advance();
+        if self.eat(&Token::RParen).is_some() {
+            // Unit `()`
+            let end = self
+                .tokens
+                .get(self.pos.saturating_sub(1))
+                .map(|s| s.span)
+                .unwrap_or(start);
+            return Some(Spanned::new(Expr::Tuple(vec![]), start.merge(end)));
+        }
+        let first = self.parse_expr(ctx)?;
+        if self.eat(&Token::RParen).is_some() {
+            // Just a parenthesised expression
+            let end = self
+                .tokens
+                .get(self.pos.saturating_sub(1))
+                .map(|s| s.span)
+                .unwrap_or(start);
+            return Some(Spanned::new(first.value, start.merge(end)));
+        }
+        // Tuple
+        let mut elems = vec![first];
+        while self.eat(&Token::Comma).is_some() && !self.check(&Token::RParen) {
+            if let Some(e) = self.parse_expr(ctx) {
+                elems.push(e);
+            }
+        }
+        self.expect(&Token::RParen, "tuple")?;
+        let end = self
+            .tokens
+            .get(self.pos.saturating_sub(1))
+            .map(|s| s.span)
+            .unwrap_or(start);
+        Some(Spanned::new(Expr::Tuple(elems), start.merge(end)))
+    }
+
+    /// An array literal, `[a, b, c]` or `[init; n]`, at `[`, which began at
+    /// `start`.
+    fn parse_array_literal(&mut self, start: Span, ctx: &str) -> Option<Spanned<Expr>> {
+        self.advance();
+        let mut elems = Vec::new();
+        let mut first = true;
+        while !self.check(&Token::RBracket) && !self.is_at_end() {
+            if let Some(e) = self.parse_expr(ctx) {
+                // `[init; n]`: a `;` after the *first* element switches
+                // to the repeat form, whose second operand is a length
+                // rather than another element.
+                if first && self.eat(&Token::Semicolon).is_some() {
+                    let len = self.parse_expr(ctx)?;
+                    self.expect(&Token::RBracket, "repeat array literal")?;
+                    let end = self
+                        .tokens
+                        .get(self.pos.saturating_sub(1))
+                        .map(|s| s.span)
+                        .unwrap_or(start);
+                    return Some(Spanned::new(
+                        Expr::ArrayRepeat {
+                            init: Box::new(e),
+                            len: Box::new(len),
+                        },
+                        start.merge(end),
+                    ));
+                }
+                elems.push(e);
+            }
+            first = false;
+            if self.eat(&Token::Comma).is_none() {
+                break;
+            }
+        }
+        self.expect(&Token::RBracket, "array literal")?;
+        let end = self
+            .tokens
+            .get(self.pos.saturating_sub(1))
+            .map(|s| s.span)
+            .unwrap_or(start);
+        Some(Spanned::new(Expr::Array(elems), start.merge(end)))
+    }
+
     fn parse_string_expr(&mut self) -> Option<Spanned<Expr>> {
         let start = self.peek_span();
         self.expect(&Token::StrStart, "string literal")?;
@@ -2272,7 +2336,7 @@ impl<'a> Parser<'a> {
     fn parse_if_expr(&mut self, ctx: &str) -> Option<Spanned<Expr>> {
         let start = self.peek_span();
         self.expect(&Token::If, "if expression")?;
-        let cond = self.in_no_struct_ctx(|p| p.parse_expr(ctx))?;
+        let cond = self.in_head_ctx(|p| p.parse_expr(ctx))?;
         let then = self.parse_block(ctx)?;
         let else_ = if self.eat(&Token::Else).is_some() {
             if self.check(&Token::If) {
@@ -2300,7 +2364,7 @@ impl<'a> Parser<'a> {
     fn parse_match_expr(&mut self, ctx: &str) -> Option<Spanned<Expr>> {
         let start = self.peek_span();
         self.expect(&Token::Match, "match expression")?;
-        let scrutinee = self.in_no_struct_ctx(|p| p.parse_expr(ctx))?;
+        let scrutinee = self.in_head_ctx(|p| p.parse_expr(ctx))?;
         self.expect(&Token::LBrace, "match body")?;
         let mut arms = Vec::new();
         while !self.check(&Token::RBrace) && !self.is_at_end() {
@@ -2330,13 +2394,16 @@ impl<'a> Parser<'a> {
 
     fn parse_match_arm(&mut self, ctx: &str) -> Option<MatchArm> {
         let pattern = self.parse_pattern(ctx)?;
+        // A guard ends at its `=>`, as a head ends at its `{`.
         let guard = if self.eat(&Token::If).is_some() {
-            Some(self.parse_expr(ctx)?)
+            Some(self.in_stmt_ctx(false, |p| p.parse_expr(ctx))?)
         } else {
             None
         };
         self.expect(&Token::FatArrow, "match arm")?;
-        let body = self.parse_expr(ctx)?;
+        // An arm's body is statement context, so a line that starts with
+        // `(` begins the next arm (spec §3.5).
+        let body = self.in_stmt_ctx(true, |p| p.parse_expr(ctx))?;
         self.eat(&Token::Comma);
         Some(MatchArm {
             pattern,
@@ -2404,8 +2471,10 @@ impl<'a> Parser<'a> {
     /// `view { child* }` (RFC 0001), at `view`.
     fn parse_view(&mut self) -> Option<Spanned<Expr>> {
         let start = self.peek_span();
-        self.advance(); // `view`
-        let children = self.parse_view_children()?;
+        // A view's children keep their `;` rule: the newline rule is off in
+        // them, as in an element's arguments.
+        self.advance();
+        let children = self.in_stmt_ctx(false, |p| p.parse_view_children())?;
         let span = start.merge(self.prev_span_or(start));
         Some(Spanned::new(Expr::View(children), span))
     }
@@ -2550,6 +2619,10 @@ impl<'a> Parser<'a> {
     }
 
     fn parse_record_literal(&mut self, path: Path, start: Span) -> Option<Spanned<Expr>> {
+        self.in_stmt_ctx(false, |p| p.parse_record_literal_inner(path, start))
+    }
+
+    fn parse_record_literal_inner(&mut self, path: Path, start: Span) -> Option<Spanned<Expr>> {
         self.expect(&Token::LBrace, "record literal")?;
         let mut fields = Vec::new();
         let mut base = None;
diff --git a/crates/nova-parser/src/lib.rs b/crates/nova-parser/src/lib.rs
index 5c2f6be1f781cb3f84d5bd0b921cd2559bd35c1d..329d7506a13800a2af9bcad2d8b36c8fcb1ef988 100644
--- a/crates/nova-parser/src/lib.rs
+++ b/crates/nova-parser/src/lib.rs
@@ -1,6 +1,9 @@
 //! Parser for the Nova programming language.
 //!
-//! Converts a `Vec<Spanned<Token>>` (from `nova-lexer`) into an `ast::File`.
+//! Converts the [`Tokens`] `nova-lexer` produces into an `ast::File`. The
+//! tokens' line-break marks drive the newline rule (spec
+//! `docs/superpowers/specs/2026-10-26-phase-5-0-the-language-and-std-a-compiler-needs-design.md`
+//! §3.5).
 //! A hand-written recursive-descent parser: after an error it skips to the next
 //! item or statement boundary and keeps going.
 //!
@@ -25,14 +28,14 @@ pub use error::ParseError;
 
 use nova_ast::File;
 use nova_diagnostics::{FileId, Spanned};
-use nova_lexer::Token;
+use nova_lexer::Tokens;
 
 /// Parse a token stream into a `File` AST.
 ///
 /// Returns `Some(File)` even when errors are present: the parser skips to the
 /// next item or statement boundary after an error and keeps going. It
 /// currently never returns `None`.
-pub fn parse(tokens: &[Spanned<Token>], file: FileId) -> (Option<File>, Vec<ParseError>) {
+pub fn parse(tokens: &Tokens, file: FileId) -> (Option<File>, Vec<ParseError>) {
     grammar::parse_file(tokens, file)
 }
 
@@ -51,6 +54,6 @@ pub struct Parsed {
 /// `docs/superpowers/specs/2026-10-08-phase-3-2-lsp-core-design.md` §3.2).
 /// The language server's analysis calls this; every other caller uses
 /// [`parse`], whose result is the same file and errors.
-pub fn parse_recovering(tokens: &[Spanned<Token>], file: FileId) -> Parsed {
+pub fn parse_recovering(tokens: &Tokens, file: FileId) -> Parsed {
     grammar::parse_recovering(tokens, file)
 }
````


### Task 2: fmt's separators under the rule

Spec §3.5 ("fmt"), §12.8; decisions 9, 16, 29 and 37.

**Files:**
- Modify:
  - `crates/nova-fmt/src/print/mod.rs` (`CONTINUES`, `VIEW_CONTINUES`)
  - `crates/nova-fmt/src/print/expr.rs` (`separator`, a view's children)
  - `crates/nova-fmt/src/source.rs` (`semicolon_before`)
- Test: `crates/nova-fmt/tests/source_rules.rs`

- [ ] **Step 1: The tests.** `patch_block t2-tests`. The patch renames and
  rewrites four tests, and adds one:
  - `a_statement_starting_with_a_parenthesis_keeps_the_authors_semicolon`,
    and
    `a_statement_starting_with_a_bracket_or_a_minus_keeps_the_authors_semicolon`,
    which takes the `[` and `-` cases out of
    `each_token_that_could_continue_a_statement_keeps_its_semicolon`: the
    author's `;` stays, and the same statements without one stay without
    (decisions 9 and 37);
  - `a_match_arm_starting_with_a_parenthesis_needs_no_comma`;
  - `a_value_less_return_keeps_the_authors_semicolon`, for `return` and
    `break`, and a bare `return` with no `;` staying so;
  - `a_view_child_starting_with_a_parenthesis_keeps_its_semicolon`
    (decision 16).
- [ ] **Step 2: Run, and see the tests fail.** First the build.
  - `cargo test --locked -p nova-fmt --test source_rules`. Expected: `22
    passed; 4 failed; 0 ignored; 0 measured; 0 filtered out`: the four
    rewritten tests,
    `a_statement_starting_with_a_parenthesis_keeps_the_authors_semicolon`,
    `a_statement_starting_with_a_bracket_or_a_minus_keeps_the_authors_semicolon`,
    `a_match_arm_starting_with_a_parenthesis_needs_no_comma` and
    `a_value_less_return_keeps_the_authors_semicolon`: the base's fmt adds a
    `;` or `,` where the source has none. The view's test passes, as it
    should: views keep the old rule (decision 31).
- [ ] **Step 3: The code.** `patch_block t2-code`:
  - **`print/mod.rs`:** `CONTINUES: [Token; 5]`, `{`, `*`, `&`, `|` and
    `||`, its doc citing the 5.0 spec §3.5; `SPLITS: [Token; 3]`, `(`,
    `[` and `-`; `VIEW_CONTINUES: [Token; 8]`, the old list (decisions 9
    and 16).
  - **`print/expr.rs`:** `separator` returns `;` before a `CONTINUES`
    token, and before a `SPLITS` token or after a bare `return` or `break`
    only where `semicolon_before` finds the author's; its doc says why; a
    view's children use `VIEW_CONTINUES`.
  - **`source.rs`:** `semicolon_before(offset)`, whether the last token
    before `offset` is a `;`.
- [ ] **Step 4: Run.** First the build.
  - `cargo test --locked -p nova-fmt --test source_rules`. Expected: `26
    passed; 0 failed`.
  - `cargo test --locked --no-fail-fast -p nova-fmt`. Expected: every
    binary `ok`: the corpus test `1 passed`, `source_rules` 26, and the
    rest as at the base.
- [ ] **Step 5: The checks,** as the conventions list them. `nova fmt
  --check` prints nothing.
- [ ] **Step 6: Commit**
  `fmt: a ; before a line the newline rule ends only where the author wrote one`.

**Patch `t2-tests`:**

<!-- patch t2-tests -->

````diff
diff --git a/crates/nova-fmt/tests/source_rules.rs b/crates/nova-fmt/tests/source_rules.rs
index 419d1408987b1ed2a3bb233b0ded24cde1f5c764..5dbd7d30c57d8c9cece72c636716c49f7bbb0bb2 100644
--- a/crates/nova-fmt/tests/source_rules.rs
+++ b/crates/nova-fmt/tests/source_rules.rs
@@ -105,29 +105,42 @@ fn a_record_pattern_ending_in_rest_takes_no_trailing_comma() {
     );
 }
 
+/// The newline rule (5.0 spec
+/// `docs/superpowers/specs/2026-10-26-phase-5-0-the-language-and-std-a-compiler-needs-design.md`
+/// §3.5) ends a statement before a `(`, a `[` or a `-` on a new line, and
+/// a `;` the author wrote there says the break is meant: fmt keeps the
+/// author's `;`, and adds none where there was none, so the program stays
+/// as it was.
 #[test]
-fn a_statement_starting_with_a_parenthesis_keeps_its_semicolon() {
+fn a_statement_starting_with_a_parenthesis_keeps_the_authors_semicolon() {
     assert_formats(
         "fn main() { f(); (a, b).show() }\n",
         "fn main() {\n    f();\n    (a, b).show()\n}\n",
     );
+    assert_stable("fn main() {\n    f()\n    (a, b).show()\n}\n");
+}
+
+#[test]
+fn a_statement_starting_with_a_bracket_or_a_minus_keeps_the_authors_semicolon() {
+    assert_formats(
+        "fn main() { f(); [1, 2].len() }\n",
+        "fn main() {\n    f();\n    [1, 2].len()\n}\n",
+    );
+    assert_formats(
+        "fn main() { let x = y; -z }\n",
+        "fn main() {\n    let x = y;\n    -z\n}\n",
+    );
+    assert_stable("fn main() {\n    f()\n    [1, 2].len()\n}\n");
+    assert_stable("fn main() {\n    let x = y\n    -z\n}\n");
 }
 
 #[test]
 fn each_token_that_could_continue_a_statement_keeps_its_semicolon() {
     for (input, expected) in [
-        (
-            "fn main() { f(); [1, 2].len() }\n",
-            "fn main() {\n    f();\n    [1, 2].len()\n}\n",
-        ),
         (
             "fn main() { let x = y; { z } }\n",
             "fn main() {\n    let x = y;\n    { z }\n}\n",
         ),
-        (
-            "fn main() { let x = y; -z }\n",
-            "fn main() {\n    let x = y;\n    -z\n}\n",
-        ),
         (
             "fn main() { let x = y; *p = 1 }\n",
             "fn main() {\n    let x = y;\n    *p = 1\n}\n",
@@ -150,15 +163,15 @@ fn each_token_that_could_continue_a_statement_keeps_its_semicolon() {
 }
 
 #[test]
-fn a_match_arm_starting_with_a_parenthesis_keeps_its_comma() {
+fn a_match_arm_starting_with_a_parenthesis_needs_no_comma() {
     assert_formats(
         "fn main() { match p { a => 1, (b, c) => 2 } }\n",
-        "fn main() {\n    match p {\n        a => 1,\n        (b, c) => 2\n    }\n}\n",
+        "fn main() {\n    match p {\n        a => 1\n        (b, c) => 2\n    }\n}\n",
     );
 }
 
 #[test]
-fn a_value_less_return_keeps_its_semicolon() {
+fn a_value_less_return_keeps_the_authors_semicolon() {
     assert_formats(
         "fn f() { return; x = 1 }\n",
         "fn f() {\n    return;\n    x = 1\n}\n",
@@ -167,6 +180,15 @@ fn a_value_less_return_keeps_its_semicolon() {
         "fn main() { while true { break; x = 1 } }\n",
         "fn main() {\n    while true {\n        break;\n        x = 1\n    }\n}\n",
     );
+    assert_stable("fn f() {\n    return\n    x = 1\n}\n");
+}
+
+#[test]
+fn a_view_child_starting_with_a_parenthesis_keeps_its_semicolon() {
+    assert_formats(
+        "fn f() -> Html { view { text(\"a\"); (text(\"b\")) } }\n",
+        "fn f() -> Html {\n    view {\n        text(\"a\");\n        (text(\"b\"))\n    }\n}\n",
+    );
 }
 
 /// A broken chain with a `-` in it breaks after each operator, so no line
````

**Patch `t2-code`:**

<!-- patch t2-code -->

````diff
diff --git a/crates/nova-fmt/src/print/expr.rs b/crates/nova-fmt/src/print/expr.rs
index ae39740104e15631f196d718f3d3aa6b15db7e20..8806114bda76349d2b11217c5611ade3b28e5e4c 100644
--- a/crates/nova-fmt/src/print/expr.rs
+++ b/crates/nova-fmt/src/print/expr.rs
@@ -9,7 +9,7 @@ use nova_ast::{Block, Path, Stmt};
 use nova_diagnostics::{Span, Spanned};
 use nova_lexer::Token;
 
-use super::{Blank, LastComma, List, Printer, Vertical, CONTINUES};
+use super::{Blank, LastComma, List, Printer, Vertical, CONTINUES, SPLITS, VIEW_CONTINUES};
 use crate::doc::{concat, group, nest, text, Doc};
 
 /// A statement in a block, or the block's trailing expression.
@@ -345,24 +345,30 @@ impl<'s, 't> Printer<'s, 't> {
 
     /// `;` between two statements where the second would otherwise continue
     /// the first (spec §5.6). A `;` after an item would not parse, and no
-    /// item can be continued.
+    /// item can be continued. Before a statement that starts with `(`, `[`
+    /// or `-`, and after a `return` or `break` with no value, the newline
+    /// rule ends the statement at the line break (5.0 spec §3.5), and a `;`
+    /// written there says the break is meant: fmt keeps the author's `;`,
+    /// and adds none, so the program stays as it was.
     fn separator(&self, cur: Elem, next: Elem) -> Option<&'static str> {
         if let Elem::Stmt(s) = cur {
             if matches!(s.value, Stmt::Item(_)) {
                 return None;
             }
         }
-        // A `return` or `break` with no value would take the next statement
-        // as its value.
-        if matches!(
-            self.src.last_code_token(cur.span().end),
-            Some(Token::Return | Token::Break)
-        ) {
+        let first = self.src.token_from(next.span().start);
+        if CONTINUES.contains(first) {
             return Some(";");
         }
-        CONTINUES
-            .contains(self.src.token_from(next.span().start))
-            .then_some(";")
+        let bare = match cur {
+            Elem::Stmt(s) => matches!(
+                &s.value,
+                Stmt::Expr(e) if matches!(e.value, Expr::Return(None) | Expr::Break(None))
+            ),
+            Elem::Tail(_) => false,
+        };
+        let written = self.src.semicolon_before(next.span().start);
+        ((SPLITS.contains(first) || bare) && written).then_some(";")
     }
 
     fn elem(&mut self, el: Elem) -> Doc {
@@ -547,7 +553,7 @@ impl<'s, 't> Printer<'s, 't> {
         for (i, child) in children.iter().enumerate() {
             let sep = children
                 .get(i + 1)
-                .filter(|n| CONTINUES.contains(self.src.token_from(n.span.start)))
+                .filter(|n| VIEW_CONTINUES.contains(self.src.token_from(n.span.start)))
                 .map(|_| ";");
             let end = self
                 .src
diff --git a/crates/nova-fmt/src/print/mod.rs b/crates/nova-fmt/src/print/mod.rs
index b3b67de5d12386f05bb571a70ffc46c267421916..10ad468d38e8d960742a8feec78f64af05cd4786 100644
--- a/crates/nova-fmt/src/print/mod.rs
+++ b/crates/nova-fmt/src/print/mod.rs
@@ -19,9 +19,28 @@ use crate::comments::Comments;
 use crate::doc::{concat, if_break, nest, text, Doc};
 use crate::source::Source;
 
-/// Tokens that would continue the statement or match arm before them, since
-/// the parser ignores line breaks (spec §5.6).
-const CONTINUES: [Token; 8] = [
+/// Tokens that would continue the statement or match arm before them across
+/// a line break (spec §5.6). The newline rule (5.0 spec
+/// `docs/superpowers/specs/2026-10-26-phase-5-0-the-language-and-std-a-compiler-needs-design.md`
+/// §3.5) ends a statement or an arm before a `(`, a `[` or a `-` on a new
+/// line, so those three are not here.
+const CONTINUES: [Token; 5] = [
+    Token::LBrace,
+    Token::Star,
+    Token::Amp,
+    Token::Pipe,
+    Token::PipePipe,
+];
+
+/// Tokens that begin a new statement after a line break under the newline
+/// rule, where a `;` the author wrote says the break is meant: fmt keeps
+/// the author's `;` before them, and adds none.
+const SPLITS: [Token; 3] = [Token::LParen, Token::LBracket, Token::Minus];
+
+/// Tokens that would continue the view child before them: a view's children
+/// keep the rule they had before the newline rule, so a child that starts
+/// with `(` or `[` still needs a `;` before it.
+const VIEW_CONTINUES: [Token; 8] = [
     Token::LParen,
     Token::LBracket,
     Token::LBrace,
diff --git a/crates/nova-fmt/src/source.rs b/crates/nova-fmt/src/source.rs
index 5a7a6ec04185fedef66db0774aa9f6036d18cf99..90fecd12df6e21573d95bad73e05e8f2bc74bb61 100644
--- a/crates/nova-fmt/src/source.rs
+++ b/crates/nova-fmt/src/source.rs
@@ -75,6 +75,16 @@ impl<'t> Source<'t> {
         (t.value == *kind).then_some(t.span.end)
     }
 
+    /// Whether the last token ending at or before `offset` is a `;`: the
+    /// author ended the statement before `offset` with one.
+    pub fn semicolon_before(&self, offset: u32) -> bool {
+        let i = self.tokens.partition_point(|t| t.span.end <= offset);
+        matches!(
+            self.tokens[..i].last().map(|t| &t.value),
+            Some(Token::Semicolon)
+        )
+    }
+
     /// The last token ending at or before `end` that is not a `;`.
     pub fn last_code_token(&self, end: u32) -> Option<&Token> {
         let i = self.tokens.partition_point(|t| t.span.end <= end);
````


### Task 3: `?`, E0098 and E0099, and a closure's return type

Spec §3.3, §12.5, §12.6; the user's decision 2; decisions 1, 6, 17 to 19,
27, 30, 38 and 39.

**Files:**
- Modify: `crates/nova-typeck/src/check.rs` (`FnCtx.entry` and
  `.in_const`, `core_sum`, `check_try`, `check_closure`, `local_expr`)
- Create: `crates/nova-typeck/src/check/lang_tests.rs` (its `?` section),
  `tests/runtime/try_operator.{nova,stdout}`,
  `tests/runtime/try_async.{nova,stdout}`
- Test:
  - `crates/nova-typeck/src/check.rs` (`mod lang_tests`)
  - `crates/nova-cli/tests/run_tests.rs` (`golden_three_ways`,
    `try_operator_runs`, `try_async_runs`)
  - `crates/nova-cli/tests/wasm_tests.rs` (two `gate!` entries)
  - `crates/nova-driver/tests/index_complete.rs` (167 programs, 201 files)

- [ ] **Step 1: The tests.** `patch_block t3-tests`. The patch holds:
  - `lang_tests.rs`, a child module of `check` under `#[cfg(test)]`:
    `diags`, the resolver's and the checker's diagnostics of a source,
    `only` and `clean`; and 21 tests: `?` on an `Option` in an `Option`
    function, on a `Result` with another `Ok` type, giving the payload's
    type; E0098 on an `Int`, naming it, and on the program's own
    `Option`; E0099 in a function returning `Int`, its `.ok_or` note on an
    `Option` in a `Result` function and not on a `Result` in an `Option`
    one, its note in `main` and in a `@test` function, and none in a
    closure inside `main`, and its note for the function's own `Option`
    (decision 39); E0099 for `?` in a constant's initializer, and a `?` in
    a closure inside a constant checking (decision 38); E0010 with the
    `map_err` note, and `map_err`
    then `?` checking; a closure whose return type `?` infers, `?` in a
    closure returning from the closure, a closure's `return` checking
    against the closure's type (decision 19), `?` in an `async fn`; and
    the HIR, a `Match` of two arms whose second returns, typed `Int`.
  - `try_operator.nova` and its `.stdout`: both sums, `n.next()?.next()?`,
    `?` re-wrapping an `Err`, in a `match` scrutinee, after `map_err`, in
    a closure whose return type it infers, and in a loop inside a closure;
    `try_async.nova`: `.await?` on `Option`s and `Result`s, a record built
    from three, in a loop.
  - `run_tests.rs`: `golden_three_ways(name, gc_stress)`, a program under
    `nova run`, `nova build` and, for `try_async`, `NOVA_GC_STRESS=1`; the
    `--release` run is Task 7's suite.
  - `wasm_tests.rs`: `try_operator` and `try_async` in `gate!`, with a
    comment: a front-end desugar, so WASM needs nothing.
  - `index_complete.rs`: 167 programs and 201 files, the two new programs
    named.
- [ ] **Step 2: Run, and see the tests fail.** First the build.
  - `T check::lang_tests`. Expected: `0 passed; 21 failed; 0 ignored; 0
    measured; 410 filtered out`: `?` is E0900, "the `?` operator are not
    supported yet", and
    `return_in_a_closure_checks_against_the_closures_type` fails on the
    base's E0010, "`return` value has type `Int` but the function returns
    `()`" (decision 19).
  - `R try_operator_runs try_async_runs`. Expected: `0 passed; 2 failed;
    0 ignored; 0 measured; 328 filtered out`, each `nova run` exiting 1
    with `error[E0900]: the `?` operator are not supported yet`.
  -
    `W gate::try_operator gate::try_async every_runtime_program_is_in_the_gate_or_left_out`.
    Expected: `1 passed; 2 failed; 0 ignored; 0 measured; 229 filtered
    out`: the two programs, for the same reason; the coverage test passes,
    both programs being listed.
  - `X`. Expected: `0 passed; 1 failed`: "tests/runtime/try_async.nova does
    not check cleanly: ["E0900 the `?` operator are not supported yet", …]"
    (decision 30).
- [ ] **Step 3: The code.** `patch_block t3-code`, in `check.rs`:
  - **`FnCtx.entry`:** set for `main` and a `@test` function, `false` for
    a constant's context and in the two tests that build an `FnCtx` by
    hand (decision 18); **`FnCtx.in_const`**, set for a constant's
    context alone (decision 38).
  - **`core_sum(name)`:** `$std.core`'s exported type `name` (decision 6).
  - **`check_try`:** the operand's type, E0098 with its note, the sum and
    its good and bad variants, the closure's return type made the same sum
    when it is a variable, E0099 for a constant's initializer, E0099 with
    its notes, the program's own `Option` or `Result` among them, E0010
    for the error types, and the `match` through `__ok` and `__err`
    (decisions 1, 17 and
    18); `ast::Expr::Try` now calls it.
  - **`check_closure`:** `fcx.ret_ty` swapped for the closure's return type,
    and `fcx.entry` and `fcx.in_const` for `false`, while the body is
    checked, all restored
    (decision 19).
  - **`local_expr`,** a read of a local, which Task 4 uses too.
- [ ] **Step 4: Run.** First the build.
  - `T check::lang_tests`. Expected: `21 passed; 0 failed; 0 ignored; 0
    measured; 410 filtered out`.
  - `R try_operator_runs try_async_runs`. Expected: `2 passed; 0 failed`.
  -
    `W gate::try_operator gate::try_async every_runtime_program_is_in_the_gate_or_left_out`.
    Expected: `3 passed; 0 failed`.
  - `X` (about 20 s). Expected: `1 passed; 0 failed`.
  - `cargo test --locked --no-fail-fast -p nova-typeck`. Expected: `431
    passed; 0 failed`, the base's 410 and the 21.
- [ ] **Step 5: The checks,** as the conventions list them.
- [ ] **Step 6: Commit**
  `typeck: the ? operator, E0098 and E0099, and return in a closure`.

**Patch `t3-tests`:**

<!-- patch t3-tests -->

````diff
diff --git a/crates/nova-cli/tests/run_tests.rs b/crates/nova-cli/tests/run_tests.rs
index 0b956797b8b029103ba4450ddc592c5507a27182..d8800535f1b0759aba45e664f1cfbc5cf78799bb 100644
--- a/crates/nova-cli/tests/run_tests.rs
+++ b/crates/nova-cli/tests/run_tests.rs
@@ -12654,3 +12654,45 @@ fn the_blog_example_passes_its_own_tests() {
         "{out}"
     );
 }
+
+/// A golden program under `nova run`, under `nova build`, and, where
+/// `gc_stress`, under `nova run` with `NOVA_GC_STRESS=1` (collect on every
+/// allocation). The `--release` build is `release_golden_suite`'s.
+fn golden_three_ways(name: &str, gc_stress: bool) {
+    let source = format!("tests/runtime/{name}.nova");
+    let expected =
+        std::fs::read_to_string(repo_root().join(format!("tests/runtime/{name}.stdout")))
+            .expect("expected-output fixture exists")
+            .replace("\r\n", "\n");
+    nova()
+        .arg("run")
+        .arg(repo_root().join(&source))
+        .assert()
+        .success()
+        .stdout(expected.clone());
+    let built = build_and_run(&source, name);
+    assert_eq!(built.replace("\r\n", "\n"), expected, "nova build");
+    if gc_stress {
+        nova()
+            .env("NOVA_GC_STRESS", "1")
+            .arg("run")
+            .arg(repo_root().join(&source))
+            .assert()
+            .success()
+            .stdout(expected);
+    }
+}
+
+/// `?` on `Option` and `Result` (5.0 spec §3.3): both sums, chains, a
+/// closure whose return type `?` infers, a loop and a `match` scrutinee.
+#[test]
+fn try_operator_runs() {
+    golden_three_ways("try_operator", false);
+}
+
+/// `?` in `async fn`s, with `.await?`, also under GC stress, since its
+/// values live across `.await`s (5.0 spec §3.3).
+#[test]
+fn try_async_runs() {
+    golden_three_ways("try_async", true);
+}
diff --git a/crates/nova-cli/tests/wasm_tests.rs b/crates/nova-cli/tests/wasm_tests.rs
index d97a72b532fc450b77824d6f3f40ff88333da5cd..f8ab72a87aeb548af730b821c85209de6c6fa43d 100644
--- a/crates/nova-cli/tests/wasm_tests.rs
+++ b/crates/nova-cli/tests/wasm_tests.rs
@@ -1947,6 +1947,9 @@ mod gate {
         ui_view,
         // `render_hydratable` (spec 4.6 §9.2).
         ui_hydratable,
+        // `?`: a front-end desugar, so WASM needs nothing (5.0 spec §3.3).
+        try_operator,
+        try_async,
     );
 
     /// The programs a test of their own in this module checks, by its name.
diff --git a/crates/nova-driver/tests/index_complete.rs b/crates/nova-driver/tests/index_complete.rs
index fd2ba68a56fc63b66ce3bd60e5deeef289a2a9b6..bd645cc6a6ec0c05e00e7bb95370557097750109 100644
--- a/crates/nova-driver/tests/index_complete.rs
+++ b/crates/nova-driver/tests/index_complete.rs
@@ -244,8 +244,10 @@ fn every_name_in_the_corpora_is_indexed() {
     // 9 examples, 155 top-level runtime programs and the modules program,
     // on 2026-10-21, after 4.7 added `http_routes`, `http_request_readers`,
     // `http_responses`, `http_server_wire`, `fs_rename`, `crypto_pbkdf2`,
-    // `crypto_pbkdf2_refused` and `bytes_hex`, and `07-fullstack-blog`.
-    assert_eq!(programs.len(), 165, "{programs:#?}");
+    // `crypto_pbkdf2_refused` and `bytes_hex`, and `07-fullstack-blog`;
+    // 157 runtime programs on 2026-10-26, after 5.0a added `try_operator`
+    // and `try_async`.
+    assert_eq!(programs.len(), 167, "{programs:#?}");
     let mut files = 0;
     let mut failures: Vec<String> = Vec::new();
     for (k, path) in programs.iter().enumerate() {
@@ -272,11 +274,11 @@ fn every_name_in_the_corpora_is_indexed() {
             every_name_is_covered(&a, file, &mut failures);
         }
     }
-    // 9 + 155 + 3 files of user code, 05-geometry's library's 2, the 8
+    // 9 + 157 + 3 files of user code, 05-geometry's library's 2, the 8
     // modules 07-fullstack-blog's client reaches from its `main.nova` (not
     // the server's 4, which only `server.nova` and `lib.nova` reach), and
     // native std's 22, `std/ui/ssr` among them.
-    assert_eq!(files, 199);
+    assert_eq!(files, 201);
     failures.sort();
     failures.dedup();
     assert!(
diff --git a/crates/nova-typeck/src/check.rs b/crates/nova-typeck/src/check.rs
index 39a929de23a18415eedd38e8b3cc86d711753c14..038e6d0189a2c7f913a6c050c77731407724d42b 100644
--- a/crates/nova-typeck/src/check.rs
+++ b/crates/nova-typeck/src/check.rs
@@ -20,6 +20,8 @@ use crate::{
 };
 
 mod fixes;
+#[cfg(test)]
+mod lang_tests;
 mod view;
 
 use fixes::{Binding, Need};
diff --git a/crates/nova-typeck/src/check/lang_tests.rs b/crates/nova-typeck/src/check/lang_tests.rs
new file mode 100644
index 0000000000000000000000000000000000000000..5e24af6afe5d6559720ed8a9efa3a892f9f923a0
--- /dev/null
+++ b/crates/nova-typeck/src/check/lang_tests.rs
@@ -0,0 +1,259 @@
+//! The checker's tests of 5.0a's language (spec
+//! `docs/superpowers/specs/2026-10-26-phase-5-0-the-language-and-std-a-compiler-needs-design.md`
+//! §3.3): `?` and its codes E0098 and E0099.
+
+use super::*;
+use nova_diagnostics::FileId;
+use nova_lexer::lex;
+use nova_parser::parse;
+use nova_resolver::resolve;
+
+/// Every diagnostic of `src`, the resolver's then the checker's.
+fn diags(src: &str) -> Vec<Diagnostic> {
+    let file_id = FileId::DUMMY;
+    let (tokens, lex_errors) = lex(src, file_id);
+    assert!(lex_errors.is_empty(), "lex errors: {lex_errors:?}");
+    let (ast, parse_errors) = parse(&tokens, file_id);
+    assert!(parse_errors.is_empty(), "parse errors: {parse_errors:?}");
+    let resolved = resolve(&ast.expect("an AST"));
+    let checked = check(&resolved.file, &resolved.definitions);
+    let mut all = resolved.diagnostics;
+    all.extend(checked.diagnostics);
+    all
+}
+
+/// The one diagnostic of `src` with `code`.
+fn only(src: &str, code: &str) -> Diagnostic {
+    let all = diags(src);
+    let hits: Vec<&Diagnostic> = all.iter().filter(|d| d.code == code).collect();
+    assert_eq!(hits.len(), 1, "one {code} in {all:#?}");
+    hits[0].clone()
+}
+
+fn clean(src: &str) {
+    let all = diags(src);
+    assert!(all.is_empty(), "{all:#?}");
+}
+
+// --- `?` ---
+
+#[test]
+fn try_on_option_in_an_option_function_checks() {
+    clean(
+        "fn first(s: String) -> Option<String> {\n    let i = s.index_of(\" \")?\n    Some(s.slice(0, i))\n}\nfn main() {}\n",
+    );
+}
+
+#[test]
+fn try_on_result_rewraps_a_different_ok_type() {
+    clean(
+        "fn p(s: String) -> Result<Int, String> { Ok(s.len()) }\n\
+         fn q(s: String) -> Result<Bool, String> {\n    let n = p(s)?\n    Ok(n > 1)\n}\nfn main() {}\n",
+    );
+}
+
+#[test]
+fn try_gives_the_payload_type() {
+    // `?` on `Option<Int>` is an `Int`: adding a `String` to it is E0010.
+    let d = only(
+        "fn f(o: Option<Int>) -> Option<Int> {\n    let s: String = o?\n    Some(1)\n}\nfn main() {}\n",
+        "E0010",
+    );
+    assert!(d.message.contains("String"), "{d:?}");
+}
+
+#[test]
+fn try_on_another_type_is_e0098_naming_it() {
+    let d = only(
+        "fn f(x: Int) -> Option<Int> {\n    let y = x?\n    Some(y)\n}\nfn main() {}\n",
+        "E0098",
+    );
+    assert_eq!(d.message, "`?` needs an `Option` or a `Result`");
+    assert!(d.notes.iter().any(|n| n.contains("`Int`")), "{d:?}");
+}
+
+#[test]
+fn try_on_the_programs_own_option_is_e0098() {
+    let d = only(
+        "type Option<T> =\n    | Some(T)\n    | None\n\
+         fn f(o: Option<Int>) -> Option<Int> {\n    let v = o?\n    Some(v)\n}\nfn main() {}\n",
+        "E0098",
+    );
+    assert!(d.notes.iter().any(|n| n.contains("program's own")), "{d:?}");
+}
+
+#[test]
+fn try_in_a_function_that_returns_neither_is_e0099() {
+    let d = only(
+        "fn f(o: Option<Int>) -> Int {\n    let v = o?\n    v\n}\nfn main() {}\n",
+        "E0099",
+    );
+    assert_eq!(d.message, "`?` cannot return from this function");
+    assert!(d.notes.iter().any(|n| n.contains("returns `Int`")), "{d:?}");
+}
+
+#[test]
+fn try_on_option_in_a_result_function_suggests_ok_or() {
+    let d = only(
+        "fn f(o: Option<Int>) -> Result<Int, String> {\n    let v = o?\n    Ok(v)\n}\nfn main() {}\n",
+        "E0099",
+    );
+    assert!(d.notes.iter().any(|n| n.contains(".ok_or(err)?")), "{d:?}");
+}
+
+#[test]
+fn try_on_result_in_an_option_function_is_e0099_without_ok_or() {
+    let d = only(
+        "fn f(r: Result<Int, String>) -> Option<Int> {\n    let v = r?\n    Some(v)\n}\nfn main() {}\n",
+        "E0099",
+    );
+    assert!(!d.notes.iter().any(|n| n.contains("ok_or")), "{d:?}");
+}
+
+#[test]
+fn try_in_main_notes_that_main_returns_unit() {
+    let d = only(
+        "fn main() {\n    let o: Option<Int> = Some(1)\n    let v = o?\n    println(\"${v}\")\n}\n",
+        "E0099",
+    );
+    assert!(
+        d.notes.iter().any(|n| n.contains("`main` and `@test`")),
+        "{d:?}"
+    );
+}
+
+#[test]
+fn try_in_a_test_function_notes_that_it_returns_unit() {
+    let d = only(
+        "@test\nfn t() {\n    let o: Option<Int> = Some(1)\n    let v = o?\n    assert(v == 1)\n}\nfn main() {}\n",
+        "E0099",
+    );
+    assert!(
+        d.notes.iter().any(|n| n.contains("`main` and `@test`")),
+        "{d:?}"
+    );
+}
+
+#[test]
+fn try_in_a_closure_inside_main_has_no_entry_note() {
+    let d = only(
+        "fn main() {\n    let f = |o: Option<Int>| -> Int {\n        let v = o?\n        v\n    }\n    println(\"${f(Some(1))}\")\n}\n",
+        "E0099",
+    );
+    assert!(!d.notes.iter().any(|n| n.contains("`main`")), "{d:?}");
+}
+
+#[test]
+fn try_with_different_error_types_is_e0010_pointing_at_map_err() {
+    let d = only(
+        "fn p() -> Result<Int, Int> { Ok(1) }\n\
+         fn f() -> Result<Int, String> {\n    let v = p()?\n    Ok(v)\n}\nfn main() {}\n",
+        "E0010",
+    );
+    assert!(d.notes.iter().any(|n| n.contains("map_err")), "{d:?}");
+}
+
+#[test]
+fn map_err_then_try_checks() {
+    clean(
+        "fn p() -> Result<Int, Int> { Ok(1) }\n\
+         fn f() -> Result<Int, String> {\n    let v = p().map_err(|e: Int| \"code ${e}\")?\n    Ok(v)\n}\nfn main() {}\n",
+    );
+}
+
+#[test]
+fn try_infers_a_closures_return_type() {
+    // The closure's return type is unknown until `?` makes it an `Option`;
+    // `Some(w.len())` then fixes its argument.
+    clean(
+        "fn first(s: String) -> Option<String> { Some(s) }\n\
+         fn main() {\n    let f = |s: String| {\n        let w = first(s)?\n        Some(w.len())\n    }\n    \
+         let n: Option<Int> = f(\"a\")\n    println(\"${n.is_some()}\")\n}\n",
+    );
+}
+
+#[test]
+fn try_in_a_closure_returns_from_the_closure_not_the_function() {
+    // The enclosing function returns `()`; the closure returns an `Option`.
+    clean(
+        "fn main() {\n    let f = |o: Option<Int>| -> Option<Int> {\n        let v = o?\n        Some(v + 1)\n    }\n    \
+         println(\"${f(Some(1)).is_some()}\")\n}\n",
+    );
+}
+
+#[test]
+fn return_in_a_closure_checks_against_the_closures_type() {
+    clean(
+        "fn main() {\n    let f = |x: Int| -> Int {\n        if x > 0 {\n            return 1\n        }\n        2\n    }\n    \
+         println(\"${f(1)}\")\n}\n",
+    );
+}
+
+#[test]
+fn try_in_an_async_fn_uses_its_declared_return_type() {
+    clean(
+        "async fn get() -> Option<Int> { Some(1) }\n\
+         async fn f() -> Option<Int> {\n    let v = get().await?\n    Some(v)\n}\nfn main() {}\n",
+    );
+}
+
+#[test]
+fn try_in_a_function_returning_its_own_option_says_so() {
+    let d = only(
+        "type Option<T> =\n    | Some(T)\n    | None\n\
+         fn f(s: String) -> Option<Int> {\n    let i = s.index_of(\"b\")?\n    Some(i)\n}\nfn main() {}\n",
+        "E0099",
+    );
+    assert!(d.notes.iter().any(|n| n.contains("program's own")), "{d:?}");
+}
+
+#[test]
+fn try_in_a_constant_is_e0099() {
+    let d = only(
+        "fn n() -> Option<Int> { None }\nconst A: Option<Int> = Some(n()? + 1)\nfn main() {}\n",
+        "E0099",
+    );
+    assert!(
+        d.notes
+            .iter()
+            .any(|n| n.contains("a constant is not a function")),
+        "{d:?}"
+    );
+}
+
+#[test]
+fn try_in_a_closure_inside_a_constant_checks() {
+    clean(
+        "const F: fn(Option<Int>) -> Option<Int> = |o: Option<Int>| -> Option<Int> { Some(o? + 1) }\nfn main() {}\n",
+    );
+}
+
+#[test]
+fn try_desugars_to_a_match_in_hir() {
+    let file_id = FileId::DUMMY;
+    let src =
+        "fn f(o: Option<Int>) -> Option<Int> {\n    let v = o?\n    Some(v)\n}\nfn main() {}\n";
+    let (tokens, _) = lex(src, file_id);
+    let (ast, _) = parse(&tokens, file_id);
+    let resolved = resolve(&ast.expect("an AST"));
+    let checked = check(&resolved.file, &resolved.definitions);
+    assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
+    let f = checked
+        .module
+        .functions
+        .iter()
+        .find(|f| f.name == "f")
+        .expect("f");
+    let hir::ExprKind::Block { stmts, .. } = &f.body.kind else {
+        panic!("a block")
+    };
+    let hir::ExprKind::Let { init, .. } = &stmts[0].kind else {
+        panic!("a let")
+    };
+    let hir::ExprKind::Match { arms, .. } = &init.kind else {
+        panic!("`?` is a match: {:?}", init.kind)
+    };
+    assert_eq!(arms.len(), 2);
+    assert!(matches!(arms[1].body.kind, hir::ExprKind::Return(Some(_))));
+    assert_eq!(init.ty, Ty::Int);
+}
diff --git a/tests/runtime/try_async.nova b/tests/runtime/try_async.nova
new file mode 100644
index 0000000000000000000000000000000000000000..51f32562fb796ba8022365cf57174b9b14a41c66
--- /dev/null
+++ b/tests/runtime/try_async.nova
@@ -0,0 +1,87 @@
+// `?` inside `async fn`s, with `.await?` (5.0 spec §3.3): the enclosing
+// function's return type is its declared one, not the future. Its values
+// live across `.await`s, so a test runs it under `NOVA_GC_STRESS=1` too.
+
+record Config { name: String, port: Int }
+
+async fn fetch(key: String) -> Option<String> {
+    yield_now().await
+    match key {
+        "name" => Some("server-${key.len()}")
+        "port" => Some("8080")
+        _ => None
+    }
+}
+
+async fn port_number(text: String) -> Result<Int, String> {
+    yield_now().await
+    match text {
+        "8080" => Ok(8080)
+        _ => Err("bad port `${text}`")
+    }
+}
+
+async fn lookup(key: String) -> Result<String, String> {
+    yield_now().await
+    match fetch(key).await {
+        Some(v) => Ok(v)
+        None => Err("missing ${key}")
+    }
+}
+
+// `Option`: `.await?` on each step.
+async fn name_and_port() -> Option<String> {
+    let name = fetch("name").await?
+    let port = fetch("port").await?
+    Some("${name}:${port}")
+}
+
+async fn missing() -> Option<String> {
+    let name = fetch("name").await?
+    let user = fetch("user").await?
+    Some("${name} ${user}")
+}
+
+// `Result`: a record built from three `.await?`s.
+async fn load() -> Result<Config, String> {
+    let name = lookup("name").await?
+    let text = lookup("port").await?
+    let port = port_number(text).await?
+    Ok(Config { name: name, port: port })
+}
+
+async fn load_user() -> Result<Config, String> {
+    let name = lookup("name").await?
+    let user = lookup("user").await?
+    Ok(Config { name: "${name}/${user}", port: 0 })
+}
+
+async fn run() -> Int {
+    match name_and_port().await {
+        Some(s) => println("some ${s}")
+        None => println("none")
+    }
+    match missing().await {
+        Some(s) => println("some ${s}")
+        None => println("none")
+    }
+    let mut ok = 0
+    let mut i = 0
+    while i < 3 {
+        match load().await {
+            Ok(c) => {
+                println("config ${c.name} ${c.port}")
+                ok += 1
+            }
+            Err(e) => println("error ${e}")
+        }
+        match load_user().await {
+            Ok(c) => println("config ${c.name}")
+            Err(e) => println("error ${e}")
+        }
+        i += 1
+    }
+    ok
+}
+
+fn main() { println("loaded ${block_on(run())}") }
diff --git a/tests/runtime/try_async.stdout b/tests/runtime/try_async.stdout
new file mode 100644
index 0000000000000000000000000000000000000000..21fc01cd3144fafbb9fbff7268789bf7c19302cf
--- /dev/null
+++ b/tests/runtime/try_async.stdout
@@ -0,0 +1,9 @@
+some server-4:8080
+none
+config server-4 8080
+error missing user
+config server-4 8080
+error missing user
+config server-4 8080
+error missing user
+loaded 3
diff --git a/tests/runtime/try_operator.nova b/tests/runtime/try_operator.nova
new file mode 100644
index 0000000000000000000000000000000000000000..fe1bba3e3d450349e9c1cbcb9838b094d8b1b65a
--- /dev/null
+++ b/tests/runtime/try_operator.nova
@@ -0,0 +1,152 @@
+// `?` on `Option` and `Result` (5.0 spec §3.3): both sums, chains, in a
+// closure whose return type is inferred, in a loop, and in a `match`
+// scrutinee.
+
+record Node { name: String, next: Option<Node> }
+
+impl Node {
+    fn next(self) -> Option<Node> { self.next }
+
+    fn first_char(self) -> Option<String> {
+        if self.name.len() == 0 { return None }
+        Some(self.name.slice(0, 1))
+    }
+}
+
+fn first_word(s: String) -> Option<String> {
+    let i = s.index_of(" ")?
+    Some(s.slice(0, i))
+}
+
+// `a()?.b()?`: a chain through two `?`s.
+fn third_initial(n: Node) -> Option<String> { n.next()?.next()?.first_char() }
+
+fn parse_digit(s: String) -> Result<Int, String> {
+    match s {
+        "0" => Ok(0)
+        "1" => Ok(1)
+        "2" => Ok(2)
+        "3" => Ok(3)
+        _ => Err("not a digit: ${s}")
+    }
+}
+
+// Each `?` re-wraps the `Err`, whose `Ok` type differs from the function's.
+fn sum_digits(parts: Vec<String>) -> Result<Int, String> {
+    let mut total = 0
+    let mut i = 0
+    while i < parts.len() {
+        total += parse_digit(parts.get(i).unwrap())?
+        i += 1
+    }
+    Ok(total)
+}
+
+// `?` in a `match` scrutinee.
+fn describe(s: String) -> Result<String, String> {
+    let word = match parse_digit(s)? {
+        0 => "zero"
+        1 => "one"
+        _ => "many"
+    }
+    Ok(word)
+}
+
+// `map_err` converts; `?` does not.
+fn digit_or_code(s: String) -> Result<Int, Int> {
+    let d = parse_digit(s).map_err(|e: String| e.len())?
+    Ok(d * 10)
+}
+
+fn show_s(o: Option<String>) -> String {
+    match o {
+        Some(s) => "Some(${s})"
+        None => "None"
+    }
+}
+
+fn show_i(o: Option<Int>) -> String {
+    match o {
+        Some(n) => "Some(${n})"
+        None => "None"
+    }
+}
+
+fn show_r(r: Result<Int, String>) -> String {
+    match r {
+        Ok(n) => "Ok(${n})"
+        Err(e) => "Err(${e})"
+    }
+}
+
+fn show_rs(r: Result<String, String>) -> String {
+    match r {
+        Ok(s) => "Ok(${s})"
+        Err(e) => "Err(${e})"
+    }
+}
+
+fn show_ri(r: Result<Int, Int>) -> String {
+    match r {
+        Ok(n) => "Ok(${n})"
+        Err(e) => "Err(${e})"
+    }
+}
+
+fn main() {
+    println(show_s(first_word("hello world")))
+    println(show_s(first_word("hello")))
+
+    let c = Node { name: "carol", next: None }
+    let b = Node { name: "bob", next: Some(c) }
+    let a = Node { name: "alice", next: Some(b) }
+    println(show_s(third_initial(a)))
+    let short = Node { name: "x", next: Some(Node { name: "y", next: None }) }
+    println(show_s(third_initial(short)))
+    let empty = Node {
+        name: "p",
+        next: Some(Node { name: "q", next: Some(Node { name: "", next: None }) }),
+    }
+    println(show_s(third_initial(empty)))
+
+    let mut good = Vec::new()
+    good.push("1")
+    good.push("2")
+    good.push("3")
+    println(show_r(sum_digits(good)))
+    let mut bad = Vec::new()
+    bad.push("1")
+    bad.push("x")
+    bad.push("3")
+    println(show_r(sum_digits(bad)))
+
+    println(show_rs(describe("0")))
+    println(show_rs(describe("3")))
+    println(show_rs(describe("9")))
+
+    println(show_ri(digit_or_code("2")))
+    println(show_ri(digit_or_code("seven")))
+
+    // A closure whose return type `?` infers.
+    let len_of_first = |s: String| {
+        let w = first_word(s)?
+        Some(w.len())
+    }
+    println(show_i(len_of_first("abc def")))
+    println(show_i(len_of_first("abcdef")))
+
+    // `?` in a loop, in a closure that returns early.
+    let count_until_bad = |xs: Vec<String>| -> Result<Int, String> {
+        let mut n = 0
+        for x in xs.iter() { n += parse_digit(x)? }
+        Ok(n)
+    }
+    let mut ys = Vec::new()
+    ys.push("2")
+    ys.push("2")
+    println(show_r(count_until_bad(ys)))
+    let mut zs = Vec::new()
+    zs.push("2")
+    zs.push("z")
+    println(show_r(count_until_bad(zs)))
+}
diff --git a/tests/runtime/try_operator.stdout b/tests/runtime/try_operator.stdout
new file mode 100644
index 0000000000000000000000000000000000000000..08960f4a8931dcd3defa7bf11986b4f37feb47d2
--- /dev/null
+++ b/tests/runtime/try_operator.stdout
@@ -0,0 +1,16 @@
+Some(hello)
+None
+Some(c)
+None
+None
+Ok(6)
+Err(not a digit: x)
+Ok(zero)
+Ok(many)
+Err(not a digit: 9)
+Ok(20)
+Err(18)
+Some(3)
+None
+Ok(4)
+Err(not a digit: z)
````

**Patch `t3-code`:**

<!-- patch t3-code -->

````diff
diff --git a/crates/nova-typeck/src/check.rs b/crates/nova-typeck/src/check.rs
index 038e6d0189a2c7f913a6c050c77731407724d42b..c77e7fd93a5018ab3a5768257f9e52c29e94f7cd 100644
--- a/crates/nova-typeck/src/check.rs
+++ b/crates/nova-typeck/src/check.rs
@@ -359,6 +359,12 @@ struct FnCtx {
     /// Closure / wrapper functions lifted out while checking this function's
     /// body; finalized with this context's `icx` and then emitted.
     pending_closures: Vec<hir::Function>,
+    /// Whether the body being checked is `main`'s or a `@test` function's,
+    /// which return `()`: E0099's note says so. `false` inside a closure.
+    entry: bool,
+    /// Whether the body being checked is a constant's initializer, which is
+    /// no function: `?` there is E0099. `false` inside a closure.
+    in_const: bool,
 }
 
 impl FnCtx {
@@ -3074,6 +3080,8 @@ impl<'a> Checker<'a> {
             loop_depth: 0,
             in_async: f.is_async,
             pending_closures: Vec::new(),
+            entry: name == "main" || f.attrs.iter().any(|a| a.name.value == "test"),
+            in_const: false,
         };
         for (p, ty) in f.params.iter().zip(sig.params.iter()) {
             self.bind_local(
@@ -3164,6 +3172,8 @@ impl<'a> Checker<'a> {
             // has no `is_async` field), so `.await` is always illegal in one.
             in_async: false,
             pending_closures: Vec::new(),
+            entry: false,
+            in_const: true,
         };
         let value = self.check_expr(&mut fcx, value_ast);
         if !fcx.icx.unify(&value.ty, &sig.ret) {
@@ -3540,7 +3550,7 @@ impl<'a> Checker<'a> {
                 self.check_record_literal(fcx, path, fields, base.as_deref(), span)
             }
             ast::Expr::Field { target, field } => self.check_field(fcx, target, field, span),
-            ast::Expr::Try(_) => self.unsupported_expr(span, "the `?` operator"),
+            ast::Expr::Try(inner) => self.check_try(fcx, inner, span),
             ast::Expr::Await(inner) => {
                 let e = self.check_expr(fcx, inner);
                 if !fcx.in_async {
@@ -4889,6 +4899,220 @@ impl<'a> Checker<'a> {
         })
     }
 
+    /// The `DefId` of std/core's sum named `name` (`Option` or `Result`),
+    /// whatever the name means in the module being checked (5.0 spec §3.3).
+    fn core_sum(&self, name: &str) -> Option<DefId> {
+        let core = (0..self.defs.module_count() as u32)
+            .map(ModuleId)
+            .find(|&m| self.defs.module_name(m) == "$std.core")?;
+        self.defs.exported(core, name).ty
+    }
+
+    /// `e?` (5.0 spec §3.3), desugared to the `match` the `for` desugar
+    /// builds: on `Option<T>`, `Some(v)` gives `v` and `None` returns `None`;
+    /// on `Result<T, E>`, `Ok(v)` gives `v` and `Err(x)` returns `Err(x)`,
+    /// re-wrapped. Nothing is converted: the enclosing function's return
+    /// type must be the same sum, with the same `E`.
+    fn check_try(&mut self, fcx: &mut FnCtx, inner: &Spanned<ast::Expr>, span: Span) -> hir::Expr {
+        let value = self.check_expr(fcx, inner);
+        let value_ty = fcx.icx.apply(&value.ty);
+        if matches!(value_ty, Ty::Error) {
+            return error_expr(span);
+        }
+        let option = self.core_sum("Option");
+        let result = self.core_sum("Result");
+        let (sum, args) = match &value_ty {
+            Ty::Sum { def_id, args } if Some(*def_id) == option || Some(*def_id) == result => {
+                (*def_id, args.clone())
+            }
+            other => {
+                let note = match other {
+                    Ty::Var(_) => "its type is not known here; annotate it".to_string(),
+                    Ty::Sum { def_id, .. }
+                        if matches!(self.defs.def(*def_id).name.as_str(), "Option" | "Result") =>
+                    {
+                        format!(
+                            "this `{}` is the program's own type, not std's",
+                            self.show(other, fcx)
+                        )
+                    }
+                    _ => format!("its type is `{}`", self.show(other, fcx)),
+                };
+                self.diagnostics.push(
+                    Diagnostic::error("E0098", "`?` needs an `Option` or a `Result`")
+                        .with_primary_label(inner.span, "this value")
+                        .with_note(note),
+                );
+                return error_expr(span);
+            }
+        };
+        let is_option = Some(sum) == option;
+        let (good, bad) = if is_option {
+            ("Some", "None")
+        } else {
+            ("Ok", "Err")
+        };
+        let (Some(good_idx), Some(bad_idx)) =
+            (self.variant_index(sum, good), self.variant_index(sum, bad))
+        else {
+            return error_expr(span);
+        };
+        let (good_idx, bad_idx) = (good_idx as u32, bad_idx as u32);
+        let payload = args.first().cloned().unwrap_or(Ty::Error);
+
+        // A constant's initializer is no function, so there is nothing to
+        // return from: E0099, the code for a `?` with nowhere to return.
+        if fcx.in_const {
+            self.diagnostics.push(
+                Diagnostic::error("E0099", "`?` cannot return from this function")
+                    .with_primary_label(span, "this `?` is in a constant's initializer")
+                    .with_note(
+                        "a constant is not a function: `?` works only in a `fn` or a closure; \
+                         match on the value instead"
+                            .to_string(),
+                    ),
+            );
+            return error_expr(span);
+        }
+
+        // The enclosing function's return type. A closure's, still being
+        // inferred, becomes the same sum with fresh arguments.
+        let ret = fcx.icx.apply(&fcx.ret_ty);
+        if matches!(ret, Ty::Var(_)) {
+            let mut fresh_args = vec![fcx.icx.fresh()];
+            if !is_option {
+                fresh_args.push(fcx.icx.fresh());
+            }
+            fcx.icx.unify(
+                &ret,
+                &Ty::Sum {
+                    def_id: sum,
+                    args: fresh_args,
+                },
+            );
+        }
+        let ret = fcx.icx.apply(&fcx.ret_ty);
+        let ret_args = match &ret {
+            Ty::Sum { def_id, args } if *def_id == sum => args.clone(),
+            Ty::Error => return error_expr(span),
+            other => {
+                let mut d = Diagnostic::error("E0099", "`?` cannot return from this function")
+                    .with_primary_label(span, "this `?` would return early")
+                    .with_note(format!(
+                        "the function returns `{}`, not {}",
+                        self.show(other, fcx),
+                        if is_option {
+                            "an `Option`"
+                        } else {
+                            "a `Result`"
+                        }
+                    ));
+                let returns_result =
+                    matches!(other, Ty::Sum { def_id, .. } if Some(*def_id) == result);
+                if is_option && returns_result {
+                    d = d.with_note(
+                        "to return an error when the value is `None`, write `.ok_or(err)?`"
+                            .to_string(),
+                    );
+                }
+                if fcx.entry {
+                    d = d.with_note(
+                        "`main` and `@test` functions return `()`, so `?` cannot return from them"
+                            .to_string(),
+                    );
+                }
+                if let Ty::Sum { def_id, .. } = other {
+                    if matches!(self.defs.def(*def_id).name.as_str(), "Option" | "Result") {
+                        d = d.with_note(format!(
+                            "this `{}` is the program's own type, not std's",
+                            self.show(other, fcx)
+                        ));
+                    }
+                }
+                self.diagnostics.push(d);
+                return error_expr(span);
+            }
+        };
+        // A `Result`'s error types must be the same: `map_err` converts.
+        if !is_option {
+            let value_err = args.get(1).cloned().unwrap_or(Ty::Error);
+            let ret_err = ret_args.get(1).cloned().unwrap_or(Ty::Error);
+            if !fcx.icx.unify(&value_err, &ret_err) {
+                self.diagnostics.push(
+                    Diagnostic::error(
+                        "E0010",
+                        format!(
+                            "`?` cannot return an error of type `{}` from a function whose \
+                             error type is `{}`",
+                            self.show(&value_err, fcx),
+                            self.show(&ret_err, fcx),
+                        ),
+                    )
+                    .with_primary_label(span, "here")
+                    .with_note(
+                        "`?` converts nothing: convert the error first, with \
+                         `.map_err(|e| ...)?`"
+                            .to_string(),
+                    ),
+                );
+                return error_expr(span);
+            }
+        }
+
+        // `match value { good(v) => v, bad(x) => return bad(x) }`, through
+        // unscoped locals no source name can reach.
+        let v = fcx.new_local_unscoped("__ok".to_string(), payload.clone(), false, span);
+        let mut bad_binders = Vec::new();
+        let mut bad_args = Vec::new();
+        if !is_option {
+            let err_ty = args.get(1).cloned().unwrap_or(Ty::Error);
+            let x = fcx.new_local_unscoped("__err".to_string(), err_ty.clone(), false, span);
+            bad_binders.push(Some(x));
+            bad_args.push(local_expr(x, err_ty, span));
+        }
+        let early = hir::Expr {
+            kind: hir::ExprKind::MakeVariant {
+                sum,
+                variant: bad_idx,
+                args: bad_args,
+            },
+            ty: ret.clone(),
+            span,
+        };
+        let arms = vec![
+            hir::Arm {
+                pattern: hir::Pattern::Variant {
+                    sum,
+                    variant: good_idx,
+                    binders: vec![Some(v)],
+                },
+                body: local_expr(v, payload.clone(), span),
+                span,
+            },
+            hir::Arm {
+                pattern: hir::Pattern::Variant {
+                    sum,
+                    variant: bad_idx,
+                    binders: bad_binders,
+                },
+                body: hir::Expr {
+                    kind: hir::ExprKind::Return(Some(Box::new(early))),
+                    ty: Ty::Never,
+                    span,
+                },
+                span,
+            },
+        ];
+        hir::Expr {
+            kind: hir::ExprKind::Match {
+                scrutinee: Box::new(value),
+                arms,
+            },
+            ty: payload,
+            span,
+        }
+    }
+
     /// Check a closure literal `|params| body`, lifting it into its own
     /// function. Free variables referring to enclosing locals are captured
     /// by value; the closure value is a fat pointer `{ code, env }`.
@@ -4938,9 +5162,18 @@ impl<'a> Checker<'a> {
         // `async fn`'s body.
         let saved_in_async = fcx.in_async;
         fcx.in_async = false;
+        // A `return`, and a `?`, in the body return from the closure, so they
+        // check against its return type, which a `?` constrains while it is
+        // still being inferred (5.0 spec §3.3).
+        let saved_ret_ty = std::mem::replace(&mut fcx.ret_ty, ret_ty.clone());
+        let saved_entry = std::mem::replace(&mut fcx.entry, false);
+        let saved_in_const = std::mem::replace(&mut fcx.in_const, false);
         let body_hir = self.check_expr(fcx, body);
         fcx.loop_depth = saved_loop_depth;
         fcx.in_async = saved_in_async;
+        fcx.ret_ty = saved_ret_ty;
+        fcx.entry = saved_entry;
+        fcx.in_const = saved_in_const;
         if !fcx.icx.unify(&body_hir.ty, &ret_ty) {
             self.error(
                 "E0010",
@@ -7979,6 +8212,15 @@ fn lit_expr(lit: &ast::Literal, span: Span) -> hir::Expr {
     hir::Expr { kind, ty, span }
 }
 
+/// A read of `local`, of type `ty`.
+fn local_expr(local: LocalId, ty: Ty, span: Span) -> hir::Expr {
+    hir::Expr {
+        kind: hir::ExprKind::Local(local),
+        ty,
+        span,
+    }
+}
+
 fn error_expr(span: Span) -> hir::Expr {
     hir::Expr {
         kind: hir::ExprKind::Unit,
@@ -13747,6 +13989,8 @@ mod tests {
             loop_depth: 0,
             in_async: false,
             pending_closures: Vec::new(),
+            entry: false,
+            in_const: false,
         };
         assert_eq!(
             checker.qualifier_self_ty(&mut fcx, "Future"),
@@ -13822,6 +14066,8 @@ mod tests {
             loop_depth: 0,
             in_async: false,
             pending_closures: Vec::new(),
+            entry: false,
+            in_const: false,
         };
         for name in nova_resolver::RESERVED_TYPE_NAMES {
             if name == "Future" {
````


### Task 4: `+=` and its kin on fields and elements

Spec §3.4; decisions 20 and 31.

**Files:**
- Modify: `crates/nova-typeck/src/check.rs` (`check_index_set`,
  `check_field_set`, `compound_value`, `let_expr`, `with_binds`)
- Test:
  - `crates/nova-typeck/src/check/lang_tests.rs` (its `+=` section, and
    `codes`)
  - `crates/nova-typeck/src/check.rs`'s tests (the three E0900 tests)

- [ ] **Step 1: The tests.** `patch_block t4-tests`. The patch holds:
  - `lang_tests.rs`: `codes`, and six tests: all ten operators on a
    field and an element; nested places, `w.g.cells[0][0] += 1` and
    `a[1][0] -= 1`; E0060 on an immutable root, naming it, for a field and
    an element; E0013 for `Bool += Bool` and E0010 for `Int += String`;
    `a[f()] += 1` holding one `Call` in its HIR; and `rs[f()].n += 1`, a
    field whose receiver holds a call, holding one too (decision 34). Its
    doc names §3.3
    and §3.4.
  - `check.rs`'s tests: `compound_assignment_to_field_reports_e0900`
    becomes `compound_assignment_to_field_checks`, clean with one
    `FieldSet`; the two `…_still_checks_the_rhs` tests expect no E0900 and
    still `undefined_fn`'s E0001.
- [ ] **Step 2: Run, and see the tests fail.** First the build.
  - `T check::lang_tests compound_assignment`. Expected: `21 passed; 9
    failed; 0 ignored; 0 measured; 407 filtered out`: the six new tests
    (`every_compound_operator_on_a_field_and_an_element_checks`,
    `compound_assignment_on_nested_places_checks`,
    `compound_assignment_on_an_immutable_root_is_e0060`,
    `compound_assignment_with_a_bad_operand_type_is_e0013_or_e0010`,
    `compound_assignment_evaluates_the_index_once`,
    `compound_assignment_on_a_field_evaluates_the_receiver_once`) and the
    three of
    `check.rs` (`compound_assignment_to_field_checks`,
    `compound_assignment_to_field_still_checks_the_rhs`,
    `compound_assignment_to_array_element_still_checks_the_rhs`), each on
    the base's E0900, "compound assignment to an array element are not
    supported yet" or "… to a record field …". The 21 are Task 3's.
- [ ] **Step 3: The code.** `patch_block t4-code`, in `check.rs`
  (decision 20):
  - **`check_index_set`:** the E0900 branch goes; for `op=`, the array and
    the index into `__place` and `__index`, the old value an `Index` of
    the two, `compound_value` the stored value, and the `IndexSet` after
    the two `let`s, through `with_binds`.
  - **`check_field_set`:** the same with the receiver in `__place` and a
    `FieldGet` for the old value.
  - **`compound_value`:** `old op value` through `binary_result_ty`.
  - **`let_expr`, `with_binds`:** a unit `let`, and a unit block of binds
    then the store.
- [ ] **Step 4: Run.** First the build.
  - `T check::lang_tests compound_assignment`. Expected: `30 passed; 0
    failed; 0 ignored; 0 measured; 407 filtered out`.
  - `cargo test --locked --no-fail-fast -p nova-typeck`. Expected: `437
    passed; 0 failed`.
- [ ] **Step 5: The checks,** as the conventions list them.
- [ ] **Step 6: Commit** `typeck: += and its kin on fields and elements`.

**Patch `t4-tests`:**

<!-- patch t4-tests -->

````diff
diff --git a/crates/nova-typeck/src/check.rs b/crates/nova-typeck/src/check.rs
index c77e7fd93a5018ab3a5768257f9e52c29e94f7cd..89242d60d3b76cc1f4870b78a9b740796b125188 100644
--- a/crates/nova-typeck/src/check.rs
+++ b/crates/nova-typeck/src/check.rs
@@ -10401,28 +10401,26 @@ mod tests {
     }
 
     #[test]
-    fn compound_assignment_to_field_reports_e0900() {
-        // `+=` on a field would have to read-modify-write; not supported yet,
-        // mirroring compound assignment to an array element.
+    fn compound_assignment_to_field_checks() {
+        // `+=` on a field reads, applies and writes back (5.0 spec §3.4); it
+        // was E0900 until 5.0a.
         let r = check_src(
             "record P { v: Int }\n\
              fn main() { let mut p = P { v: 1 }\n p.v += 1\n println(\"${p.v}\") }",
         );
-        assert!(error_codes(&r).contains(&"E0900"), "{:?}", r.diagnostics);
+        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
+        assert_eq!(field_sets_in(&r.module, "main").len(), 1);
     }
 
     #[test]
     fn compound_assignment_to_field_still_checks_the_rhs() {
-        // This branch returns before doing anything else at all — no
-        // mutability check, no receiver check — so it used to drop an
-        // independent RHS mistake along with (correctly) rejecting the
-        // compound form itself.
+        // An independent mistake in the right-hand side surfaces on its own.
         let r = check_src(
             "record P { v: Int }\n\
              fn main() { let mut p = P { v: 1 }\n p.v += undefined_fn() }",
         );
         let codes = error_codes(&r);
-        assert!(codes.contains(&"E0900"), "{:?}", r.diagnostics);
+        assert!(!codes.contains(&"E0900"), "{:?}", r.diagnostics);
         assert!(
             codes.contains(&"E0001"),
             "expected `undefined_fn`'s own error to still surface: {:?}",
@@ -10436,7 +10434,7 @@ mod tests {
         // `compound_assignment_to_field_still_checks_the_rhs`.
         let r = check_src("fn main() { let mut xs = [1, 2]\n xs[0] += undefined_fn() }");
         let codes = error_codes(&r);
-        assert!(codes.contains(&"E0900"), "{:?}", r.diagnostics);
+        assert!(!codes.contains(&"E0900"), "{:?}", r.diagnostics);
         assert!(
             codes.contains(&"E0001"),
             "expected `undefined_fn`'s own error to still surface: {:?}",
diff --git a/crates/nova-typeck/src/check/lang_tests.rs b/crates/nova-typeck/src/check/lang_tests.rs
index 5e24af6afe5d6559720ed8a9efa3a892f9f923a0..408b66a5175fc9d8950cfeb419868f48b09a69d9 100644
--- a/crates/nova-typeck/src/check/lang_tests.rs
+++ b/crates/nova-typeck/src/check/lang_tests.rs
@@ -1,6 +1,7 @@
 //! The checker's tests of 5.0a's language (spec
 //! `docs/superpowers/specs/2026-10-26-phase-5-0-the-language-and-std-a-compiler-needs-design.md`
-//! §3.3): `?` and its codes E0098 and E0099.
+//! §3.3 and §3.4): `?` and its codes E0098 and E0099, and `+=` on
+//! fields and elements.
 
 use super::*;
 use nova_diagnostics::FileId;
@@ -22,6 +23,10 @@ fn diags(src: &str) -> Vec<Diagnostic> {
     all
 }
 
+fn codes(src: &str) -> Vec<String> {
+    diags(src).into_iter().map(|d| d.code).collect()
+}
+
 /// The one diagnostic of `src` with `code`.
 fn only(src: &str, code: &str) -> Diagnostic {
     let all = diags(src);
@@ -257,3 +262,91 @@ fn try_desugars_to_a_match_in_hir() {
     assert!(matches!(arms[1].body.kind, hir::ExprKind::Return(Some(_))));
     assert_eq!(init.ty, Ty::Int);
 }
+
+// --- `+=` on fields and elements ---
+
+#[test]
+fn every_compound_operator_on_a_field_and_an_element_checks() {
+    for op in ["+=", "-=", "*=", "/=", "%=", "&=", "|=", "^=", "<<=", ">>="] {
+        clean(&format!(
+            "record R {{ n: Int, a: [Int] }}\n\
+             fn main() {{\n    let mut r = R {{ n: 1, a: [1, 2] }}\n    r.n {op} 3\n    r.a[1] {op} 2\n    println(\"${{r.n}}\")\n}}\n"
+        ));
+    }
+}
+
+#[test]
+fn compound_assignment_on_nested_places_checks() {
+    clean(
+        "record G { cells: [[Int]] }\nrecord W { g: G }\n\
+         fn main() {\n    let mut w = W { g: G { cells: [[1]] } }\n    w.g.cells[0][0] += 1\n    let mut a = [[1, 2], [3, 4]]\n    a[1][0] -= 1\n}\n",
+    );
+}
+
+#[test]
+fn compound_assignment_on_an_immutable_root_is_e0060() {
+    let d = only(
+        "record R { n: Int }\nfn main() {\n    let r = R { n: 1 }\n    r.n += 1\n}\n",
+        "E0060",
+    );
+    assert!(d.message.contains("`r`"), "{d:?}");
+    let d = only(
+        "fn main() {\n    let a = [1, 2]\n    a[0] *= 2\n}\n",
+        "E0060",
+    );
+    assert!(d.message.contains("`a`"), "{d:?}");
+}
+
+#[test]
+fn compound_assignment_with_a_bad_operand_type_is_e0013_or_e0010() {
+    assert!(codes(
+        "record R { b: Bool }\nfn main() {\n    let mut r = R { b: true }\n    r.b += true\n}\n"
+    )
+    .contains(&"E0013".to_string()));
+    assert!(codes(
+        "record R { n: Int }\nfn main() {\n    let mut r = R { n: 1 }\n    r.n += \"x\"\n}\n"
+    )
+    .contains(&"E0010".to_string()));
+}
+
+#[test]
+fn compound_assignment_evaluates_the_index_once() {
+    // `a[f()] += 1` binds the index to a local: one call in the HIR.
+    let file_id = FileId::DUMMY;
+    let src = "fn f() -> Int { 0 }\nfn main() {\n    let mut a = [1]\n    a[f()] += 1\n}\n";
+    let (tokens, _) = lex(src, file_id);
+    let (ast, _) = parse(&tokens, file_id);
+    let resolved = resolve(&ast.expect("an AST"));
+    let checked = check(&resolved.file, &resolved.definitions);
+    assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
+    let main = checked
+        .module
+        .functions
+        .iter()
+        .find(|f| f.name == "main")
+        .expect("main");
+    let text = format!("{:?}", main.body);
+    assert_eq!(text.matches("Call {").count(), 1, "{text}");
+}
+
+#[test]
+fn compound_assignment_on_a_field_evaluates_the_receiver_once() {
+    // `rs[f()].n += 1`: the receiver, `rs[f()]`, is bound to a local, so
+    // its call is in the HIR once.
+    let file_id = FileId::DUMMY;
+    let src = "record R { n: Int }\nfn f() -> Int { 0 }\n\
+               fn main() {\n    let mut rs = [R { n: 1 }]\n    rs[f()].n += 1\n}\n";
+    let (tokens, _) = lex(src, file_id);
+    let (ast, _) = parse(&tokens, file_id);
+    let resolved = resolve(&ast.expect("an AST"));
+    let checked = check(&resolved.file, &resolved.definitions);
+    assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
+    let main = checked
+        .module
+        .functions
+        .iter()
+        .find(|f| f.name == "main")
+        .expect("main");
+    let text = format!("{:?}", main.body);
+    assert_eq!(text.matches("Call {").count(), 1, "{text}");
+}
````

**Patch `t4-code`:**

<!-- patch t4-code -->

````diff
diff --git a/crates/nova-typeck/src/check.rs b/crates/nova-typeck/src/check.rs
index 89242d60d3b76cc1f4870b78a9b740796b125188..5755220a43a542bfa9c4d4d8562aea0180cf4f85 100644
--- a/crates/nova-typeck/src/check.rs
+++ b/crates/nova-typeck/src/check.rs
@@ -6980,11 +6980,6 @@ impl<'a> Checker<'a> {
         rhs: &Spanned<ast::Expr>,
         span: Span,
     ) -> hir::Expr {
-        if !matches!(op, ast::AssignOp::Assign) {
-            self.unsupported(span, "compound assignment to an array element");
-            self.check_expr(fcx, rhs);
-            return error_expr(span);
-        }
         // The array's storage must be reachable through a mutable binding.
         // Walk the whole index/field chain to its root local — `grid[0][1]`,
         // `rec.data[0]`, and `make()[0]` all bypass a single-segment check.
@@ -7008,7 +7003,38 @@ impl<'a> Checker<'a> {
             self.check_expr(fcx, rhs);
             return error_expr(span);
         }
+        // `a[i] op= e` (5.0 spec §3.4): the array and the index go into
+        // unscoped locals, evaluated once, then `a[k] = a[k] op e` stores.
+        // Arrays are references, so the local aliases the place.
+        let compound = assign_binop(op).map(|bin| {
+            let arr_ty = fcx.icx.apply(&arr.ty);
+            let t = fcx.new_local_unscoped("__place".to_string(), arr_ty.clone(), false, span);
+            let k = fcx.new_local_unscoped("__index".to_string(), Ty::Int, false, span);
+            (bin, t, arr_ty, k)
+        });
+        let (arr, idx, binds) = match &compound {
+            None => (arr, idx, Vec::new()),
+            Some((_, t, arr_ty, k)) => (
+                local_expr(*t, arr_ty.clone(), target.span),
+                local_expr(*k, Ty::Int, index.span),
+                vec![let_expr(*t, arr, span), let_expr(*k, idx, span)],
+            ),
+        };
         let value = self.check_expr(fcx, rhs);
+        let value = match &compound {
+            None => value,
+            Some((bin, t, arr_ty, k)) => {
+                let old = hir::Expr {
+                    kind: hir::ExprKind::Index {
+                        target: Box::new(local_expr(*t, arr_ty.clone(), target.span)),
+                        index: Box::new(local_expr(*k, Ty::Int, index.span)),
+                    },
+                    ty: elem_ty.clone(),
+                    span: target.span.merge(index.span),
+                };
+                self.compound_value(fcx, *bin, old, value, span)
+            }
+        };
         if !fcx.icx.unify(&value.ty, &elem_ty) {
             self.error(
                 "E0010",
@@ -7020,7 +7046,7 @@ impl<'a> Checker<'a> {
                 rhs.span,
             );
         }
-        hir::Expr {
+        let set = hir::Expr {
             kind: hir::ExprKind::IndexSet {
                 target: Box::new(arr),
                 index: Box::new(idx),
@@ -7028,6 +7054,29 @@ impl<'a> Checker<'a> {
             },
             ty: Ty::Unit,
             span,
+        };
+        with_binds(binds, set, span)
+    }
+
+    /// `old op value`, the value a compound assignment stores, through the
+    /// same rules as a binary expression (E0010, E0013).
+    fn compound_value(
+        &mut self,
+        fcx: &mut FnCtx,
+        bin: hir::BinOp,
+        old: hir::Expr,
+        value: hir::Expr,
+        span: Span,
+    ) -> hir::Expr {
+        let ty = self.binary_result_ty(fcx, bin, &old, &value, span);
+        hir::Expr {
+            kind: hir::ExprKind::Binary {
+                op: bin,
+                lhs: Box::new(old),
+                rhs: Box::new(value),
+            },
+            ty,
+            span,
         }
     }
 
@@ -7043,11 +7092,6 @@ impl<'a> Checker<'a> {
         rhs: &Spanned<ast::Expr>,
         span: Span,
     ) -> hir::Expr {
-        if !matches!(op, ast::AssignOp::Assign) {
-            self.unsupported(span, "compound assignment to a record field");
-            self.check_expr(fcx, rhs);
-            return error_expr(span);
-        }
         // The record's storage must be reachable through a mutable binding.
         // Walk the whole field/index chain to its root local — `rec.inner.f`
         // and `make().f` both bypass a single-segment check. Records have no
@@ -7080,7 +7124,35 @@ impl<'a> Checker<'a> {
             return error_expr(span);
         };
         self.note_field(fcx, &recv_ty, index, field.span);
+        // `r.f op= e` (5.0 spec §3.4): the receiver goes into an unscoped
+        // local, evaluated once, then `t.f = t.f op e` stores. Records are
+        // references, so the local aliases the place.
+        let (rec, binds, compound) = match assign_binop(op) {
+            None => (rec, Vec::new(), None),
+            Some(bin) => {
+                let t = fcx.new_local_unscoped("__place".to_string(), recv_ty.clone(), false, span);
+                (
+                    local_expr(t, recv_ty.clone(), target.span),
+                    vec![let_expr(t, rec, span)],
+                    Some((bin, t)),
+                )
+            }
+        };
         let value = self.check_expr(fcx, rhs);
+        let value = match compound {
+            None => value,
+            Some((bin, t)) => {
+                let old = hir::Expr {
+                    kind: hir::ExprKind::FieldGet {
+                        target: Box::new(local_expr(t, recv_ty.clone(), target.span)),
+                        index,
+                    },
+                    ty: field_ty.clone(),
+                    span: target.span.merge(field.span),
+                };
+                self.compound_value(fcx, bin, old, value, span)
+            }
+        };
         if !fcx.icx.unify(&value.ty, &field_ty) {
             self.error(
                 "E0010",
@@ -7093,7 +7165,7 @@ impl<'a> Checker<'a> {
                 rhs.span,
             );
         }
-        hir::Expr {
+        let set = hir::Expr {
             kind: hir::ExprKind::FieldSet {
                 target: Box::new(rec),
                 index,
@@ -7101,7 +7173,8 @@ impl<'a> Checker<'a> {
             },
             ty: Ty::Unit,
             span,
-        }
+        };
+        with_binds(binds, set, span)
     }
 
     fn check_match(
@@ -8221,6 +8294,35 @@ fn local_expr(local: LocalId, ty: Ty, span: Span) -> hir::Expr {
     }
 }
 
+/// `let local = init`, unit-typed.
+fn let_expr(local: LocalId, init: hir::Expr, span: Span) -> hir::Expr {
+    hir::Expr {
+        kind: hir::ExprKind::Let {
+            local,
+            init: Box::new(init),
+        },
+        ty: Ty::Unit,
+        span,
+    }
+}
+
+/// `expr` after `binds`, in one unit-typed block; `expr` alone when there
+/// are none.
+fn with_binds(mut binds: Vec<hir::Expr>, expr: hir::Expr, span: Span) -> hir::Expr {
+    if binds.is_empty() {
+        return expr;
+    }
+    binds.push(expr);
+    hir::Expr {
+        kind: hir::ExprKind::Block {
+            stmts: binds,
+            trailing: None,
+        },
+        ty: Ty::Unit,
+        span,
+    }
+}
+
 fn error_expr(span: Span) -> hir::Expr {
     hir::Expr {
         kind: hir::ExprKind::Unit,
````


### Task 5: `String + String`

Spec §3.6, §3.4 (`+=` on a `String` field); the user's decision 8;
decisions 7, 21, 27 and 30.

**Files:**
- Modify:
  - `crates/nova-typeck/src/check.rs` (`binary_result_ty`: the `Add` arm
    and the note)
  - `crates/nova-mir/src/lower.rs` (`StrConcat` for `+`)
- Create: `tests/runtime/compound_assign_places.{nova,stdout}`,
  `tests/runtime/string_concat.{nova,stdout}`
- Test:
  - `crates/nova-typeck/src/check/lang_tests.rs` (its `String + String`
    section, and `+=` on a `String` field)
  - `crates/nova-codegen-llvm/tests/ir_tests.rs`
    (`string_plus_string_lowers_to_one_concat_call`)
  - `crates/nova-cli/tests/run_tests.rs`, `wasm_tests.rs`,
    `crates/nova-driver/tests/index_complete.rs` (the two programs; 169
    programs, 203 files)

- [ ] **Step 1: The tests.** `patch_block t5-tests`. The patch holds:
  - `lang_tests.rs`: `+=` on a `String` field and element; `String +
    String` a `String`, and `+=` on a local; `String + Char` and `+ Int`
    E0010 with the interpolation note (decision 21); `String - String`
    still E0013. Its doc names §3.3, §3.4 and §3.6.
  - `ir_tests.rs`: `function_body`, and `join(a, b) = a + b` holding one
    `call ptr @nova_rt_str_concat(` and no `add i64`.
  - `compound_assign_places.nova`: every operator on a field and an
    element, through a method's `mut self`, nested places, a receiver and
    an index with a side effect each run once ("receiver evaluated 1
    time(s)", "index evaluated 1 time(s)"), and `+=` on a `String` field
    and element; `string_concat.nova`: empty strings, Thai and Chinese
    text, `+=` on a local, in a loop, parts with side effects in order,
    equality with interpolation, and a 400-character string built in a
    loop. Both use `+=` on a field and on a `String`, so they come here
    (decision 30).
  - `run_tests.rs`: `compound_assign_places_runs`, and
    `string_concat_runs` with GC stress; `wasm_tests.rs`: both in `gate!`,
    the comment now naming `+=` and `String + String`; `index_complete.rs`:
    169 programs and 203 files.
- [ ] **Step 2: Run, and see the tests fail.** First the build.
  - `T check::lang_tests`. Expected: `28 passed; 3 failed; 0 ignored; 0
    measured; 410 filtered out`: `string_compound_assignment_on_a_field_checks`
    and `string_plus_string_is_a_string`, on E0013, "arithmetic operators
    are not defined for `String`", and
    `string_plus_char_or_int_is_refused_with_a_note`, whose E0010 has no
    note yet.
  - `cargo test --locked -p nova-codegen-llvm --test ir_tests --
    string_plus_string_lowers_to_one_concat_call`. Expected: `0 passed; 1
    failed; 0 ignored; 0 measured; 16 filtered out`, the checker's E0013
    at `ir_tests.rs:26:5`.
  - `R compound_assign_places_runs string_concat_runs`. Expected: `0
    passed; 2 failed; 0 ignored; 0 measured; 330 filtered out`, on E0013.
  -
    `W gate::compound_assign_places gate::string_concat every_runtime_program_is_in_the_gate_or_left_out`.
    Expected: `1 passed; 2 failed; 0 ignored; 0 measured; 231 filtered
    out`.
  - `X`. Expected: `0 passed; 1 failed`:
    "tests/runtime/compound_assign_places.nova
    does not check cleanly: ["E0013 arithmetic operators are not defined
    for `String` …", …]".
- [ ] **Step 3: The code.** `patch_block t5-code`:
  - **`check.rs`:** in `binary_result_ty`, `Add if matches!(operand,
    Ty::String) => Ty::String`, before the numeric arm; and, where the
    operand types differ, the note "`+` joins two `String`s; to add any
    other value to a string, interpolate it" when `op` is `Add` and either
    side is a `String` (decisions 7 and 21).
  - **`lower.rs`:** a `String` `Add` becomes one `CallRuntime` of
    `RtFunc::StrConcat` into a `Ptr` temporary, before the integer path
    (decision 7).
- [ ] **Step 4: Run.** First the build.
  - `T check::lang_tests`. Expected: `31 passed; 0 failed`.
  - `cargo test --locked -p nova-codegen-llvm --test ir_tests --
    string_plus_string_lowers_to_one_concat_call`. Expected: `1 passed; 0
    failed`.
  - `R compound_assign_places_runs string_concat_runs`. Expected: `2
    passed; 0 failed`.
  -
    `W gate::compound_assign_places gate::string_concat every_runtime_program_is_in_the_gate_or_left_out`.
    Expected: `3 passed; 0 failed`.
  - `X`. Expected: `1 passed; 0 failed`.
  - `cargo test --locked --no-fail-fast -p nova-typeck -p nova-mir -p
    nova-codegen-llvm`. Expected: every binary `ok`: `nova-typeck`'s 441,
    `nova-mir`'s 36 and 61, `ir_tests`'s 17.
- [ ] **Step 5: The checks,** as the conventions list them.
- [ ] **Step 6: Commit** `typeck, mir: String + String`.

**Patch `t5-tests`:**

<!-- patch t5-tests -->

````diff
diff --git a/crates/nova-cli/tests/run_tests.rs b/crates/nova-cli/tests/run_tests.rs
index d8800535f1b0759aba45e664f1cfbc5cf78799bb..f9819139f43c343f1f183e555d25eb8148a28fa7 100644
--- a/crates/nova-cli/tests/run_tests.rs
+++ b/crates/nova-cli/tests/run_tests.rs
@@ -12696,3 +12696,15 @@ fn try_operator_runs() {
 fn try_async_runs() {
     golden_three_ways("try_async", true);
 }
+
+/// `+=` and its kin on fields and elements (5.0 spec §3.4).
+#[test]
+fn compound_assign_places_runs() {
+    golden_three_ways("compound_assign_places", false);
+}
+
+/// `String + String` (5.0 spec §3.6), also under GC stress.
+#[test]
+fn string_concat_runs() {
+    golden_three_ways("string_concat", true);
+}
diff --git a/crates/nova-cli/tests/wasm_tests.rs b/crates/nova-cli/tests/wasm_tests.rs
index f8ab72a87aeb548af730b821c85209de6c6fa43d..2e4958a55f4ee125602f9ed09745ccac26f50d6b 100644
--- a/crates/nova-cli/tests/wasm_tests.rs
+++ b/crates/nova-cli/tests/wasm_tests.rs
@@ -1947,9 +1947,13 @@ mod gate {
         ui_view,
         // `render_hydratable` (spec 4.6 §9.2).
         ui_hydratable,
-        // `?`: a front-end desugar, so WASM needs nothing (5.0 spec §3.3).
+        // `?`, `+=` on places and `String + String`: front-end desugars and
+        // the existing concatenation, so WASM needs nothing (5.0 spec §3.3,
+        // §3.4, §3.6).
         try_operator,
         try_async,
+        compound_assign_places,
+        string_concat,
     );
 
     /// The programs a test of their own in this module checks, by its name.
diff --git a/crates/nova-codegen-llvm/tests/ir_tests.rs b/crates/nova-codegen-llvm/tests/ir_tests.rs
index 571e46f59e8fae8abe0d16ecf8034524e58e603d..2064911490aa76feb237ed59e45d114f782d9f71 100644
--- a/crates/nova-codegen-llvm/tests/ir_tests.rs
+++ b/crates/nova-codegen-llvm/tests/ir_tests.rs
@@ -657,3 +657,33 @@ fn an_async_fn_containing_await_emits_a_resume_dispatch_and_an_indirect_poll() {
         "the suspend must return a status: {suspend:?}"
     );
 }
+
+/// The body of the function whose `define` line names `name`, up to its
+/// closing `}`.
+fn function_body<'a>(ir: &'a str, name: &str) -> &'a str {
+    let start = ir
+        .lines()
+        .find(|l| l.starts_with("define ") && l.contains(name))
+        .and_then(|l| ir.find(l))
+        .unwrap_or_else(|| panic!("no function `{name}`:\n{ir}"));
+    let end = ir[start..].find("\n}").expect("the function's end") + start;
+    &ir[start..end]
+}
+
+/// `String + String` (5.0 spec §3.6) is one call of the runtime's
+/// concatenation, not an integer `add` of the two pointers.
+#[test]
+fn string_plus_string_lowers_to_one_concat_call() {
+    let ir = ir_for(
+        "fn join(a: String, b: String) -> String { a + b }\n\
+         fn main() { println(join(\"x\", \"y\")) }",
+    );
+    let body = function_body(&ir, "join");
+    assert_eq!(
+        body.matches("call ptr @nova_rt_str_concat(").count(),
+        1,
+        "{body}"
+    );
+    assert!(!body.contains("add i64"), "{body}");
+    assert_well_formed(&ir);
+}
diff --git a/crates/nova-driver/tests/index_complete.rs b/crates/nova-driver/tests/index_complete.rs
index bd645cc6a6ec0c05e00e7bb95370557097750109..2239ea5602e76608cd1f5b2f19abbdb74be23bf8 100644
--- a/crates/nova-driver/tests/index_complete.rs
+++ b/crates/nova-driver/tests/index_complete.rs
@@ -245,9 +245,9 @@ fn every_name_in_the_corpora_is_indexed() {
     // on 2026-10-21, after 4.7 added `http_routes`, `http_request_readers`,
     // `http_responses`, `http_server_wire`, `fs_rename`, `crypto_pbkdf2`,
     // `crypto_pbkdf2_refused` and `bytes_hex`, and `07-fullstack-blog`;
-    // 157 runtime programs on 2026-10-26, after 5.0a added `try_operator`
-    // and `try_async`.
-    assert_eq!(programs.len(), 167, "{programs:#?}");
+    // 159 runtime programs on 2026-10-26, after 5.0a added `try_operator`,
+    // `try_async`, `compound_assign_places` and `string_concat`.
+    assert_eq!(programs.len(), 169, "{programs:#?}");
     let mut files = 0;
     let mut failures: Vec<String> = Vec::new();
     for (k, path) in programs.iter().enumerate() {
@@ -274,11 +274,11 @@ fn every_name_in_the_corpora_is_indexed() {
             every_name_is_covered(&a, file, &mut failures);
         }
     }
-    // 9 + 157 + 3 files of user code, 05-geometry's library's 2, the 8
+    // 9 + 159 + 3 files of user code, 05-geometry's library's 2, the 8
     // modules 07-fullstack-blog's client reaches from its `main.nova` (not
     // the server's 4, which only `server.nova` and `lib.nova` reach), and
     // native std's 22, `std/ui/ssr` among them.
-    assert_eq!(files, 201);
+    assert_eq!(files, 203);
     failures.sort();
     failures.dedup();
     assert!(
diff --git a/crates/nova-typeck/src/check/lang_tests.rs b/crates/nova-typeck/src/check/lang_tests.rs
index 408b66a5175fc9d8950cfeb419868f48b09a69d9..86583dfb065ce8818b600730ef0103a6c49928c8 100644
--- a/crates/nova-typeck/src/check/lang_tests.rs
+++ b/crates/nova-typeck/src/check/lang_tests.rs
@@ -1,7 +1,7 @@
 //! The checker's tests of 5.0a's language (spec
 //! `docs/superpowers/specs/2026-10-26-phase-5-0-the-language-and-std-a-compiler-needs-design.md`
-//! §3.3 and §3.4): `?` and its codes E0098 and E0099, and `+=` on
-//! fields and elements.
+//! §3.3, §3.4 and §3.6): `?` and its codes E0098 and E0099, `+=` on
+//! fields and elements, and `String + String`.
 
 use super::*;
 use nova_diagnostics::FileId;
@@ -350,3 +350,33 @@ fn compound_assignment_on_a_field_evaluates_the_receiver_once() {
     let text = format!("{:?}", main.body);
     assert_eq!(text.matches("Call {").count(), 1, "{text}");
 }
+
+#[test]
+fn string_compound_assignment_on_a_field_checks() {
+    clean(
+        "record R { s: String }\nfn main() {\n    let mut r = R { s: \"a\" }\n    r.s += \"b\"\n    let mut v = [\"x\"]\n    v[0] += \"y\"\n}\n",
+    );
+}
+
+// --- `String + String` ---
+
+#[test]
+fn string_plus_string_is_a_string() {
+    clean("fn main() {\n    let s: String = \"a\" + \"b\"\n    let mut t = s\n    t += \"c\"\n    println(t)\n}\n");
+}
+
+#[test]
+fn string_plus_char_or_int_is_refused_with_a_note() {
+    for rhs in ["'c'", "1"] {
+        let d = only(
+            &format!("fn main() {{\n    let s = \"a\" + {rhs}\n    println(s)\n}}\n"),
+            "E0010",
+        );
+        assert!(d.notes.iter().any(|n| n.contains("interpolate")), "{d:?}");
+    }
+}
+
+#[test]
+fn string_minus_string_is_still_e0013() {
+    assert!(codes("fn main() {\n    let s = \"a\" - \"b\"\n}\n").contains(&"E0013".to_string()));
+}
diff --git a/tests/runtime/compound_assign_places.nova b/tests/runtime/compound_assign_places.nova
new file mode 100644
index 0000000000000000000000000000000000000000..0d31a8e73017f1f2152aba74139b10a4ef7369fd
--- /dev/null
+++ b/tests/runtime/compound_assign_places.nova
@@ -0,0 +1,106 @@
+// `+=` and its kin on fields and elements (5.0 spec §3.4): every operator
+// on a field and on an element, nested places, a receiver with a side
+// effect that runs once, and `+=` on a `String` field.
+
+record Lexer { pos: Int, counts: [Int], text: String }
+
+record Grid { cells: [[Int]] }
+
+record Wrapper { inner: Lexer }
+
+record Calls { n: Int }
+
+impl Lexer {
+    fn advance(mut self, by: Int) {
+        self.pos += by
+        self.counts[0] -= 1
+    }
+}
+
+fn index_of(calls: Calls, i: Int) -> Int {
+    let mut c = calls
+    c.n += 1
+    i
+}
+
+fn main() {
+    let mut l = Lexer {
+        pos: 100,
+        counts: [100, 100, 100, 100, 100, 100, 100, 100, 100, 100],
+        text: "a",
+    }
+
+    // Every operator on a field.
+    l.pos += 7
+    println("+= ${l.pos}")
+    l.pos -= 2
+    println("-= ${l.pos}")
+    l.pos *= 3
+    println("*= ${l.pos}")
+    l.pos /= 4
+    println("/= ${l.pos}")
+    l.pos %= 7
+    println("%= ${l.pos}")
+    l.pos |= 8
+    println("|= ${l.pos}")
+    l.pos &= 12
+    println("&= ${l.pos}")
+    l.pos ^= 5
+    println("^= ${l.pos}")
+    l.pos <<= 3
+    println("<<= ${l.pos}")
+    l.pos >>= 2
+    println(">>= ${l.pos}")
+
+    // Every operator on an element.
+    l.counts[0] += 7
+    l.counts[1] -= 2
+    l.counts[2] *= 3
+    l.counts[3] /= 4
+    l.counts[4] %= 7
+    l.counts[5] |= 1
+    l.counts[6] &= 12
+    l.counts[7] ^= 5
+    l.counts[8] <<= 3
+    l.counts[9] >>= 2
+    let mut i = 0
+    while i < 10 {
+        println("counts[${i}] = ${l.counts[i]}")
+        i += 1
+    }
+
+    // Through a method's `mut self`.
+    l.advance(5)
+    println("advance ${l.pos} ${l.counts[0]}")
+
+    // Nested places.
+    let mut g = Grid { cells: [[1, 2], [3, 4]] }
+    g.cells[1][0] *= 10
+    g.cells[0][1] -= 5
+    println("grid ${g.cells[0][0]} ${g.cells[0][1]} ${g.cells[1][0]} ${g.cells[1][1]}")
+    let mut w = Wrapper { inner: l }
+    w.inner.pos += 1000
+    w.inner.counts[9] += 1
+    println("wrapper ${w.inner.pos} ${w.inner.counts[9]}")
+    let mut a = [10, 20, 30]
+    a[a[0] / 10] += 1
+    println("array ${a[0]} ${a[1]} ${a[2]}")
+
+    // A receiver and an index with a side effect each run once: in
+    // `g.cells[f()][0]`, `g.cells[f()]` is the receiver.
+    let calls = Calls { n: 0 }
+    g.cells[index_of(calls, 1)][1] += 1
+    println("receiver evaluated ${calls.n} time(s), cell ${g.cells[1][1]}")
+    let idx = Calls { n: 0 }
+    a[index_of(idx, 2)] += 5
+    println("index evaluated ${idx.n} time(s), a[2] = ${a[2]}")
+
+    // `+=` on a `String` field and element.
+    l.text += "bc"
+    l.text += ""
+    l.text += "é"
+    println("text ${l.text}")
+    let mut words = ["x", "y"]
+    words[1] += "z"
+    println("words ${words[0]} ${words[1]}")
+}
diff --git a/tests/runtime/compound_assign_places.stdout b/tests/runtime/compound_assign_places.stdout
new file mode 100644
index 0000000000000000000000000000000000000000..220e4ad633955d2f6d4a9d3ea5ff411adcdfd2d3
--- /dev/null
+++ b/tests/runtime/compound_assign_places.stdout
@@ -0,0 +1,28 @@
++= 107
+-= 105
+*= 315
+/= 78
+%= 1
+|= 9
+&= 8
+^= 13
+<<= 104
+>>= 26
+counts[0] = 107
+counts[1] = 98
+counts[2] = 300
+counts[3] = 25
+counts[4] = 2
+counts[5] = 101
+counts[6] = 4
+counts[7] = 97
+counts[8] = 800
+counts[9] = 25
+advance 31 106
+grid 1 -3 30 4
+wrapper 1031 26
+array 10 21 30
+receiver evaluated 1 time(s), cell 5
+index evaluated 1 time(s), a[2] = 35
+text abcé
+words x yz
diff --git a/tests/runtime/string_concat.nova b/tests/runtime/string_concat.nova
new file mode 100644
index 0000000000000000000000000000000000000000..ffb333300e590347fa668c49dcfa38c48f7d8c04
--- /dev/null
+++ b/tests/runtime/string_concat.nova
@@ -0,0 +1,58 @@
+// `String + String` (5.0 spec §3.6): empty strings, non-ASCII text, `+=` on
+// a local, in a loop, and parts with side effects, in order. A test runs it
+// under `NOVA_GC_STRESS=1` too.
+
+record Log { n: Int }
+
+fn part(log: Log, s: String) -> String {
+    let mut l = log
+    l.n += 1
+    println("part ${l.n}: ${s}")
+    s
+}
+
+fn main() {
+    let empty = ""
+    println("[" + empty + "]")
+    println("[" + empty + empty + "]")
+    let a = "สวัสดี"
+    let b = "世界"
+    let ab = a + " " + b
+    println(ab)
+    println("${ab.len()} characters")
+
+    let mut s = ""
+    s += "x"
+    s += empty
+    s += "é"
+    println(s)
+
+    let mut acc = ""
+    let mut i = 0
+    while i < 10 {
+        acc += "${i}"
+        if i < 9 { acc = acc + "," }
+        i += 1
+    }
+    println(acc)
+
+    // Left to right, each part once.
+    let log = Log { n: 0 }
+    let joined = part(log, "a") + part(log, "b") + part(log, "c")
+    println(joined)
+
+    // `+` between strings compares equal to interpolation.
+    let x = "left"
+    let y = "right"
+    let same = x + y == "${x}${y}"
+    println("same ${same}")
+
+    // A longer string built in a loop; the GC can run between appends.
+    let mut long = ""
+    let mut k = 0
+    while k < 200 {
+        long = long + "ab"
+        k += 1
+    }
+    println("${long.len()}")
+}
diff --git a/tests/runtime/string_concat.stdout b/tests/runtime/string_concat.stdout
new file mode 100644
index 0000000000000000000000000000000000000000..c84de97d1b1896d675b119d53ed6a7d8ea2eb9e8
--- /dev/null
+++ b/tests/runtime/string_concat.stdout
@@ -0,0 +1,12 @@
+[]
+[]
+สวัสดี 世界
+9 characters
+xé
+0,1,2,3,4,5,6,7,8,9
+part 1: a
+part 2: b
+part 3: c
+abc
+same true
+400
````

**Patch `t5-code`:**

<!-- patch t5-code -->

````diff
diff --git a/crates/nova-mir/src/lower.rs b/crates/nova-mir/src/lower.rs
index 5479f41773a98ed7ec734d8992e88123719b8613..1bc23857bbfa50a0d19591e0fd8e1e784627bdfe 100644
--- a/crates/nova-mir/src/lower.rs
+++ b/crates/nova-mir/src/lower.rs
@@ -1011,6 +1011,18 @@ impl<'a> Lowerer<'a> {
             self.push(Stmt::Not { dst: ne, src: eq });
             return ne;
         }
+        // `String + String` (5.0 spec §3.6) is one call of the runtime's
+        // concatenation, as a two-part interpolation is. Without this arm it
+        // would fall to the integer path below and add the two pointers.
+        if matches!(lhs.ty, Ty::String) && matches!(op, hir::BinOp::Add) {
+            let t = self.new_temp(MirTy::Ptr);
+            self.push(Stmt::CallRuntime {
+                dst: Some(t),
+                func: RtFunc::StrConcat,
+                args: vec![l, r],
+            });
+            return t;
+        }
         // Unit comparison is a constant.
         if matches!(lhs.ty, Ty::Unit) {
             let t = self.new_temp(MirTy::I8);
diff --git a/crates/nova-typeck/src/check.rs b/crates/nova-typeck/src/check.rs
index 5755220a43a542bfa9c4d4d8562aea0180cf4f85..d4b7936f98e6bbc92a5b5b090c7cab39636459dd 100644
--- a/crates/nova-typeck/src/check.rs
+++ b/crates/nova-typeck/src/check.rs
@@ -6665,6 +6665,20 @@ impl<'a> Checker<'a> {
                 ),
                 span,
             );
+            // `+` joins two `String`s only: a `Char` or an `Int` goes in
+            // through interpolation (5.0 spec §3.6).
+            let string = |t: &Ty| matches!(fcx.icx.apply(t), Ty::String);
+            if matches!(op, Add) && (string(&lhs.ty) || string(&rhs.ty)) {
+                self.diagnostics
+                    .last_mut()
+                    .expect("just pushed")
+                    .notes
+                    .push(
+                        "`+` joins two `String`s; to add any other value to a string, \
+                         interpolate it: `\"${a}${b}\"`"
+                            .to_string(),
+                    );
+            }
             return Ty::Error;
         }
         let mut operand = fcx.icx.apply(&lhs.ty);
@@ -6675,6 +6689,9 @@ impl<'a> Checker<'a> {
             operand = Ty::Int;
         }
         match op {
+            // `String + String` is a new `String`, `a` then `b` (5.0 spec
+            // §3.6); MIR lowers it to the runtime's concatenation.
+            Add if matches!(operand, Ty::String) => Ty::String,
             Add | Sub | Mul | Div | Rem => match operand {
                 Ty::Int | Ty::Float | Ty::Never | Ty::Error => operand,
                 other => {
````


### Task 6: E0102 and E0103, and the newline rule's splits

Spec §3.5 (E0103), §3.7, §12.7, §12.10; the user's decision 8 and ruling
on finding 1; decisions 8, 22 to 24, 37 and 44 to 47.

**Files:**
- Modify:
  - `crates/nova-ast/src/lib.rs` (`File.newline_splits`, `NewlineSplit`,
    `SplitKind`, `File`'s `Debug`)
  - `crates/nova-parser/src/grammar.rs` (`record_split`, at the rule's
    three arms and a bare `return` or `break`; `halt`; `can_start_expr`;
    every split at an arm's end dropped, and `note_arm_split`)
  - `crates/nova-parser/src/error.rs` (`ParseError::Noted`, `notes`)
  - `crates/nova-fmt/src/check.rs` (the fingerprint's split kinds, and its
    unit test), `crates/nova-fmt/src/source.rs`,
    `crates/nova-driver/src/program.rs` and
    `crates/nova-cli/src/cmd/parse.rs` (a P0001's notes)
  - `crates/nova-fmt/src/print/mod.rs` and `print/expr.rs` (their words
    name E0103 now)
  - `crates/nova-resolver/src/lib.rs` (`std_globbed_variants`,
    `variant_shadowing`, called in `resolve_program_for`; the user
    modules' splits in the merged file; a P0001's notes)
  - `crates/nova-typeck/src/check.rs` (`unused_value`, and E0103 at an
    expression statement; `warn_newline_splits`; `ast::File::default()` in
    two tests)
- Test:
  - `crates/nova-resolver/src/lib.rs`'s tests (six new, one changed)
  - `crates/nova-parser/tests/newline_rule.rs` (eight split tests)
  - `crates/nova-fmt/tests/source_rules.rs`
    (`an_unformatted_file_with_a_split_formats_keeping_it`, and E0103 in
    the semicolon tests' doc comment)
  - `crates/nova-typeck/src/check/lang_tests.rs` (its E0103 sections)
  - `crates/nova-cli/tests/run_tests.rs`
    (`a_variant_that_hides_one_of_stds_warns_and_still_builds`)

- [ ] **Step 1: The tests.** `patch_block t6-tests`. The patch holds:
  - the resolver's tests: `e0102`, a filter; a hit, `Number`, a warning
    with the message "variant `Number` hides std's `JsonValue::Number`"
    and its note; `Some`, `None` and `NotFound` each reported, in order;
    a qualified use of std's variant adding nothing; `Word`, a record
    named `JsonValue` and a function named `args` not reported; std
    itself never; a second module's `Ok` reported
    (`another_modules_variant_is_reported_too`). And
    `importing_a_std_core_name_from_a_module_is_allowed` now expects one
    E0102, for `lib`'s `Ok`, and no error (decision 22).
  - `newline_rule.rs`: `splits(stmts)`, each recorded split's token and
    kind; each kind recorded, `-` after a `return`'s value among them;
    none after a written `;`; none at the next arm, after a `let` or an
    item following a bare `return`, or where the rule is off (decision
    37); one split, of kind `Value`, after a bare `return` before `(g)(1)`
    or `-1`, the `(g)(1)` a tail of its own, and none when such a `return`
    ends an arm before `(1) => 2`, nor when a `return a`, a `break a`, a
    closure or a `return { a }` does, whose value and the expression
    around it each meet the split (decision 45); `g(return` then `)` and
    `[return` then `]` parse errors, as at the base (decision 46); `0 => a`
    then `- 1,` a P0001 whose note
    names the rule (decision 47).
  - `source_rules.rs`: an unformatted file, `let  c = a` with two spaces,
    then `c` and `- b` on their own lines, formats to `let c = a`, `c` and
    `-b`, the split kept (decision 44).
  - `lang_tests.rs`: `e0103(body)`, the count of E0103s in a `main`, each
    a warning; each form flagged once, `-y`, `!true`, `x + y`, `x == y`,
    `1`, `"text"`, `y`, `(x)`, `(x, y)`, `[x, y]`, `(g())`; `let z = x`
    then `-y` flagged once; none on a call, assignments, `||`, `&&`,
    `if`, `match`, `while`, a block, or a tail; `x` then `(g)()`, two
    (decision 24); and, by message, a split after a `return`'s value and
    before a tail, `k(1)` then `(5)` one warning, a split after a bare
    `return`, a written `;` silencing both, and an unused value at a split
    one warning with the line break's note, none without a break
    (decision 37). The module doc and the `Severity` import take their
    final form.
  - `run_tests.rs`: a program whose `Token` has a `Number(Int)` variant:
    `nova build` succeeds, its stderr holds `warning[E0102]: variant
    `Number` hides std's `JsonValue::Number`` and the note, and the binary
    prints `n4` and `w`.
- [ ] **Step 2: Run, and see the tests fail.** First the build.
  - `cargo test --locked -p nova-resolver --lib -- e0102
    each_hidden_variant_is_reported a_qualified_use_of_stds_variant_is_fine
    a_name_that_is_not_one_of_stds_variants_is_not_reported
    std_itself_is_never_reported another_modules_variant_is_reported_too
    importing_a_std_core_name_from_a_module_is_allowed`. Expected: `2
    passed; 5 failed; 0 ignored; 0 measured; 53 filtered out`: the four
    tests that expect an E0102 and the changed test fail; the two that
    expect none pass, as they should.
  - `cargo test --locked -p nova-parser --test newline_rule -- split
    before_a_token arm_the_rule_began`. Expected: it does not compile,
    `error[E0432]: unresolved import `nova_ast::SplitKind``,
    `error[E0609]: no field `newline_splits` on type `nova_ast::File`` and
    `error[E0599]: no method named `notes` found for enum `ParseError``:
    the tests name what Step 3 adds (decision 31).
  - `cargo test --locked -p nova-fmt --test source_rules -- unformatted`.
    Expected: `1 passed; 0 failed; 0 ignored; 0 measured; 26 filtered
    out`: it passes before the code, since no split is recorded yet; it is
    the fingerprint's mutant that fails it (decisions 31 and 44).
  - `T check::lang_tests::e0103 check::lang_tests::a_split
    check::lang_tests::a_written check::lang_tests::an_unused`. Expected:
    `2 passed; 7 failed; 0 ignored; 0 measured; 441 filtered out`: every
    test that expects an E0103;
    `e0103_is_not_on_calls_assignments_conditions_or_a_tail`
    and `a_written_semicolon_silences_the_split` pass, as they should.
  - `R a_variant_that_hides_one_of_stds_warns_and_still_builds`. Expected:
    `0 passed; 1 failed; 0 ignored; 0 measured; 332 filtered out`: the
    build succeeds with an empty stderr, which holds no `warning[E0102]`.
- [ ] **Step 3: The code.** `patch_block t6-code`:
  - **`nova-ast`:** `File.newline_splits: Vec<NewlineSplit>`; `File`
    derives `Default`, and its `Debug` prints the field only when it is
    not empty, so `nova parse --ast` prints an unsplit file as before;
    `NewlineSplit { at, kind }` and `SplitKind` (`Call`, `Index`, `Minus`,
    `Value`).
  - **`grammar.rs`:** `splits`, and `record_split(kind)`, called where the
    postfix loop stops at a `(` or `[`, where the additive loop stops at a
    `-`, and where `bare_keyword_ends` ends a `return` or `break` at a line
    break before a token `can_start_expr` accepts but a `let` or an item;
    before any other token the keyword reads as before the rule (decision
    46); `halt`, set there, makes the cursor read as the end until
    `parse_expr` returns, so the line after is no operand and one split is
    recorded (decision 45); `parse_match_arm` drops every split at its
    body's end, and `note_arm_split` adds the rule's note to a P0001 at
    the next arm's start (decision 47); `parse_file` sorts and dedups the
    splits into the `File` (decision 37).
  - **`error.rs` and the four P0001 sites:** `ParseError::Noted { error,
    note }` and `notes()`; fmt's `Source`, the driver, the resolver and
    `nova parse` add each note to the P0001 (decision 47).
  - **fmt:** `check.rs`'s fingerprint compares the splits' kinds, in
    order, not their places, with a unit test,
    `the_fingerprint_tells_splits_by_kind_not_by_place`; `SPLITS`' and
    `separator`'s doc comments name E0103 (decision 44).
  - **`nova-resolver`'s `lib.rs`:** `std_globbed_variants`, std's variants
    by name with their sum's name; `variant_shadowing`, an E0102 warning at
    each variant of a sum in a module before `std_start`; called after
    std's imports, its diagnostics added to the resolver's (decision 22);
    and the user modules' splits kept in the merged file.
  - **`check.rs`:** after checking an expression statement, an E0103
    warning, "this value is not used", labelled "this <form> is
    discarded", when `unused_value` names a form (decisions 23 and 24);
    and `warn_newline_splits`, after the functions are checked: at each
    split, the note on the "this value is not used" that starts there, or
    an E0103 of its own (decision 37).
- [ ] **Step 4: Run.** First the build.
  - The resolver's command of Step 2. Expected: `7 passed; 0 failed`.
  - The parser's command of Step 2. Expected: `8 passed; 0 failed; 0
    ignored; 0 measured; 30 filtered out`.
  - The `source_rules` command of Step 2. Expected: `1 passed; 0 failed`.
  - `cargo test --locked -p nova-fmt --lib -- fingerprint`. Expected: `1
    passed; 0 failed; 0 ignored; 0 measured; 27 filtered out`.
  - The `T` command of Step 2. Expected: `9 passed; 0 failed`.
  - `R a_variant_that_hides_one_of_stds_warns_and_still_builds`. Expected:
    `1 passed; 0 failed`.
  - `cargo test --locked --no-fail-fast -p nova-resolver -p nova-typeck
    -p nova-parser -p nova-fmt -p nova-ast`. Expected: none failed;
    `nova-resolver`'s `60 passed`, `nova-typeck`'s `450 passed`,
    `newline_rule`'s `38 passed`, `parser_tests`' `56 passed`, `nova-fmt`'s
    unit tests `28 passed` and `source_rules`' `27 passed`.
- [ ] **Step 5: The checks,** as the conventions list them. Where the two
  warnings fire in the repository is Task 9's count (decisions 22, 24 and
  37).
- [ ] **Step 6: Commit**
  `resolver, typeck: E0102 and E0103, two warnings, and the newline rule's splits`.

**Patch `t6-tests`:**

<!-- patch t6-tests -->

````diff
diff --git a/crates/nova-cli/tests/run_tests.rs b/crates/nova-cli/tests/run_tests.rs
index f9819139f43c343f1f183e555d25eb8148a28fa7..b07c4eadd49d3aefe7cc589a993f4aa3fe662a3c 100644
--- a/crates/nova-cli/tests/run_tests.rs
+++ b/crates/nova-cli/tests/run_tests.rs
@@ -12708,3 +12708,45 @@ fn compound_assign_places_runs() {
 fn string_concat_runs() {
     golden_three_ways("string_concat", true);
 }
+
+/// E0102 (5.0 spec §3.7) is a warning: it prints, and the program still
+/// builds and runs, its own `Number` meaning its own variant.
+#[test]
+fn a_variant_that_hides_one_of_stds_warns_and_still_builds() {
+    let dir = std::env::temp_dir().join("nova-e0102-warning");
+    std::fs::create_dir_all(&dir).expect("temp dir");
+    let src = dir.join("main.nova");
+    std::fs::write(
+        &src,
+        "type Token =\n    | Number(Int)\n    | Word(String)\n\n\
+         fn show(t: Token) -> String {\n    match t {\n        Number(n) => \"n${n}\"\n        Word(w) => w\n    }\n}\n\n\
+         fn main() {\n    println(show(Number(4)))\n    println(show(Word(\"w\")))\n}\n",
+    )
+    .expect("write the program");
+    let exe = dir.join(format!("main{}", std::env::consts::EXE_SUFFIX));
+    let built = nova()
+        .env("NO_COLOR", "1")
+        .arg("build")
+        .arg(&src)
+        .arg("-o")
+        .arg(&exe)
+        .assert()
+        .success();
+    let stderr = String::from_utf8_lossy(&built.get_output().stderr).replace("\r\n", "\n");
+    assert!(
+        stderr.contains("warning[E0102]: variant `Number` hides std's `JsonValue::Number`"),
+        "{stderr}"
+    );
+    assert!(stderr.contains("std's is `JsonValue::Number`"), "{stderr}");
+    let out = Command::new(&exe)
+        .assert()
+        .success()
+        .get_output()
+        .stdout
+        .clone();
+    assert_eq!(
+        String::from_utf8_lossy(&out).replace("\r\n", "\n"),
+        "n4\nw\n"
+    );
+    let _ = std::fs::remove_file(&exe);
+}
diff --git a/crates/nova-fmt/tests/source_rules.rs b/crates/nova-fmt/tests/source_rules.rs
index 5dbd7d30c57d8c9cece72c636716c49f7bbb0bb2..470b88f5e72a81aa01c223293040a981bfc7ae82 100644
--- a/crates/nova-fmt/tests/source_rules.rs
+++ b/crates/nova-fmt/tests/source_rules.rs
@@ -108,9 +108,9 @@ fn a_record_pattern_ending_in_rest_takes_no_trailing_comma() {
 /// The newline rule (5.0 spec
 /// `docs/superpowers/specs/2026-10-26-phase-5-0-the-language-and-std-a-compiler-needs-design.md`
 /// §3.5) ends a statement before a `(`, a `[` or a `-` on a new line, and
-/// a `;` the author wrote there says the break is meant: fmt keeps the
-/// author's `;`, and adds none where there was none, so the program stays
-/// as it was.
+/// warns there (E0103) unless the author wrote `;`: fmt keeps the author's
+/// `;`, and adds none where there was none, so the program and its warnings
+/// stay as they were.
 #[test]
 fn a_statement_starting_with_a_parenthesis_keeps_the_authors_semicolon() {
     assert_formats(
@@ -236,6 +236,17 @@ fn a_flat_chain_with_a_minus_is_unchanged() {
     assert_stable("fn f(a: Int, b: Int) -> Int { a - b + 1 }\n");
 }
 
+/// A file the newline rule splits, laid out otherwise than fmt prints it:
+/// fmt formats it, and keeps its splits (5.0 spec §3.5), whose places move
+/// with the layout.
+#[test]
+fn an_unformatted_file_with_a_split_formats_keeping_it() {
+    assert_formats(
+        "fn f(a: Int, b: Int) -> Int {\n    let  c = a\n    c\n    - b\n}\n",
+        "fn f(a: Int, b: Int) -> Int {\n    let c = a\n    c\n    -b\n}\n",
+    );
+}
+
 #[test]
 fn width_counts_characters_not_bytes() {
     // 4 + `report("` + 86 Thai letters + `")` is exactly 100 characters, and
diff --git a/crates/nova-parser/tests/newline_rule.rs b/crates/nova-parser/tests/newline_rule.rs
index 9d691d916862fd15db500743fdb81a9268cb247c..223ab3fe0298f429afd8686f1e35715448e3e9f9 100644
--- a/crates/nova-parser/tests/newline_rule.rs
+++ b/crates/nova-parser/tests/newline_rule.rs
@@ -7,7 +7,7 @@
 
 use nova_ast::expr::{BinOp, Expr, Literal, StringPart, UnOp};
 use nova_ast::pattern::Pattern;
-use nova_ast::{Block, Item, Stmt};
+use nova_ast::{Block, Item, SplitKind, Stmt};
 use nova_diagnostics::FileDb;
 use nova_lexer::lex;
 use nova_parser::parse;
@@ -483,3 +483,113 @@ fn the_rule_is_off_in_a_nested_items_signature_and_value() {
     // top-level one does; its body-less item is not statement context.
     assert_eq!(body(&f("const C: Int = a\n-1\nC")), ["item", "tail:C"]);
 }
+
+// --- the splits the parser records for E0103 (the user's ruling of 2026-10-27) ---
+
+/// Each split of `f`'s body `stmts`: the text of the token the line break
+/// cut off, and what it would have been.
+fn splits(stmts: &str) -> Vec<(String, SplitKind)> {
+    let src = f(stmts);
+    let mut db = FileDb::new();
+    let id = db.add("splits.nova", src.as_str());
+    let (tokens, _) = lex(&src, id);
+    let (file, errors) = parse(&tokens, id);
+    assert!(errors.is_empty(), "{src}\n{errors:?}");
+    file.expect("a file")
+        .newline_splits
+        .iter()
+        .map(|s| {
+            (
+                src[s.at.start as usize..s.at.end as usize].to_string(),
+                s.kind,
+            )
+        })
+        .collect()
+}
+
+#[test]
+fn each_split_the_rule_makes_is_recorded() {
+    assert_eq!(splits("g\n(1)"), [("(".to_string(), SplitKind::Call)]);
+    assert_eq!(splits("g\n[1]"), [("[".to_string(), SplitKind::Index)]);
+    assert_eq!(
+        splits("let z = x\n-y"),
+        [("-".to_string(), SplitKind::Minus)]
+    );
+    assert_eq!(
+        splits("return a\n- b"),
+        [("-".to_string(), SplitKind::Minus)]
+    );
+    assert_eq!(splits("return\ng()"), [("g".to_string(), SplitKind::Value)]);
+    assert_eq!(
+        splits("while true {\n    break\n    x\n}"),
+        [("x".to_string(), SplitKind::Value)]
+    );
+}
+
+#[test]
+fn a_written_semicolon_records_no_split() {
+    assert!(splits("g;\n(1)").is_empty());
+    assert!(splits("let z = x;\n-y").is_empty());
+    assert!(splits("return;\ng()").is_empty());
+}
+
+#[test]
+fn what_the_old_parser_could_not_have_continued_records_no_split() {
+    // The next arm after an arm's body, and a `let` or an item after a bare
+    // `return`, were never part of the statement before.
+    assert!(splits("match x {\n    A => g\n    (a) => h\n}").is_empty());
+    assert!(splits("match x {\n    A => return\n    B => h\n}").is_empty());
+    assert!(splits("return\nlet y = 1").is_empty());
+    assert!(splits("return\nfn g() {}").is_empty());
+    // A line break where the rule is off is no split either.
+    assert!(splits("let z = (x\n-y)").is_empty());
+}
+
+#[test]
+fn a_split_after_a_bare_return_is_recorded_once() {
+    // The line after a bare `return` is no operand of it: no call, index or
+    // `-` takes the keyword, so one split, of the value.
+    assert_eq!(
+        splits("return\n(g)(1)"),
+        [("(".to_string(), SplitKind::Value)]
+    );
+    assert_eq!(splits("return\n-1"), [("-".to_string(), SplitKind::Value)]);
+    assert_eq!(body(&f("return\n(g)(1)")), ["return", "tail:g(1)"]);
+}
+
+#[test]
+fn a_bare_return_ending_an_arm_records_no_split() {
+    assert!(splits("match x {\n    0 => return\n    (1) => 2\n    _ => 3\n}").is_empty());
+}
+
+#[test]
+fn a_value_ending_an_arm_records_no_split_however_often_it_is_met() {
+    // A `return`'s value or a closure's body ends at the `(`, and so does
+    // the expression that holds it: both meet the split, and the arm drops
+    // each.
+    for body in ["return a", "break a", "|y| y", "return { a }"] {
+        let src = format!("match x {{\n    0 => {body}\n    (1) => 2\n    _ => 3\n}}");
+        assert!(splits(&src).is_empty(), "{body}");
+    }
+}
+
+#[test]
+fn a_bare_return_before_a_token_no_expression_starts_with_reads_as_before() {
+    // `g(return` then `)`: as before the rule, the `)` is read as the
+    // value's start, a parse error, and no split is recorded.
+    assert!(!errors("g(return\n)").is_empty());
+    assert!(!errors("let v = [return\n]").is_empty());
+}
+
+#[test]
+fn a_parse_error_at_an_arm_the_rule_began_names_the_rule() {
+    let errs = errors("match x {\n    0 => a\n    - 1,\n    _ => 2\n}");
+    assert!(!errs.is_empty());
+    assert!(
+        errs[0]
+            .notes()
+            .iter()
+            .any(|n| n.contains("begins the next match arm")),
+        "{errs:?}"
+    );
+}
diff --git a/crates/nova-resolver/src/lib.rs b/crates/nova-resolver/src/lib.rs
index f3df01270d5370c04f28e633ed9b071ccfe2a0f1..b62b0d0a9f3509fe09e4dec0c373a686180e52a5 100644
--- a/crates/nova-resolver/src/lib.rs
+++ b/crates/nova-resolver/src/lib.rs
@@ -4069,7 +4069,14 @@ mod tests {
             "import lib\nfn main() { }\n",
             "pub type Status = | Ok | Fail\n",
         );
-        assert!(p.diagnostics.is_empty(), "{:?}", p.diagnostics);
+        // Only 5.0a's warning that `lib`'s `Ok` hides std's (E0102), at its
+        // definition: no error.
+        assert_eq!(
+            error_codes(&p.diagnostics),
+            ["E0102"],
+            "{:?}",
+            p.diagnostics
+        );
         // `Ok` resolves (to the imported Status variant), not rejected.
         assert!(p.definitions.resolve_value(ModuleId(0), "Ok").is_some());
     }
@@ -4336,4 +4343,88 @@ mod tests {
         ));
         assert!(d.is_std_module(ModuleId(2)) && !d.is_std_module(ModuleId(1)));
     }
+
+    // === 5.0a: E0102, a variant that hides one of std's (spec
+    // docs/superpowers/specs/2026-10-26-phase-5-0-the-language-and-std-a-compiler-needs-design.md
+    // §3.7) ===
+
+    fn e0102(diags: &[Diagnostic]) -> Vec<&Diagnostic> {
+        diags.iter().filter(|d| d.code == "E0102").collect()
+    }
+
+    #[test]
+    fn a_variant_named_as_one_of_stds_is_e0102() {
+        let r = resolve_src("type Token =\n    | Number(Int)\n    | Word(String)\nfn main() {}\n");
+        let hits = e0102(&r.diagnostics);
+        assert_eq!(hits.len(), 1, "{:?}", r.diagnostics);
+        let d = hits[0];
+        assert_eq!(d.severity, nova_diagnostics::Severity::Warning);
+        assert_eq!(
+            d.message,
+            "variant `Number` hides std's `JsonValue::Number`"
+        );
+        assert!(d.notes[0].contains("the bare name `Number` now means this variant"));
+        assert!(d.notes[0].contains("std's is `JsonValue::Number`"));
+    }
+
+    #[test]
+    fn each_hidden_variant_is_reported() {
+        let r =
+            resolve_src("type Mine =\n    | Some(Int)\n    | None\n    | NotFound\nfn main() {}\n");
+        let messages: Vec<&str> = e0102(&r.diagnostics)
+            .iter()
+            .map(|d| d.message.as_str())
+            .collect();
+        assert_eq!(
+            messages,
+            [
+                "variant `Some` hides std's `Option::Some`",
+                "variant `None` hides std's `Option::None`",
+                "variant `NotFound` hides std's `IoErrorKind::NotFound`",
+            ]
+        );
+    }
+
+    #[test]
+    fn a_qualified_use_of_stds_variant_is_fine() {
+        // The warning is at the definition only: a use of std's variant by
+        // its qualified name adds nothing.
+        let r = resolve_src(
+            "type Token =\n    | Number(Int)\n\
+             fn main() {\n    let j = JsonValue::Number(1.0)\n    let t = Number(1)\n}\n",
+        );
+        assert_eq!(error_codes(&r.diagnostics), ["E0102"]);
+    }
+
+    #[test]
+    fn a_name_that_is_not_one_of_stds_variants_is_not_reported() {
+        // `Token` and `Word` are no variant of std's, and a function, a
+        // record or a type that shadows a std name is not reported.
+        let r = resolve_src(
+            "type Token =\n    | Word(String)\nrecord JsonValue { x: Int }\nfn args() -> Int { 1 }\nfn main() {}\n",
+        );
+        assert!(e0102(&r.diagnostics).is_empty(), "{:?}", r.diagnostics);
+    }
+
+    #[test]
+    fn std_itself_is_never_reported() {
+        let r = resolve_src("fn main() {}\n");
+        assert!(e0102(&r.diagnostics).is_empty(), "{:?}", r.diagnostics);
+    }
+
+    #[test]
+    fn another_modules_variant_is_reported_too() {
+        // A second module of the program. A path dependency's modules come
+        // before `std_start` too, and the driver shows their warnings
+        // (`nova-driver`'s `a_path_dependencys_warning_is_still_shown`).
+        let r = resolve_two(
+            "fn main() {}\n",
+            "pub type Answer =\n    | Ok\n    | Maybe\n",
+        );
+        let messages: Vec<&str> = e0102(&r.diagnostics)
+            .iter()
+            .map(|d| d.message.as_str())
+            .collect();
+        assert_eq!(messages, ["variant `Ok` hides std's `Result::Ok`"]);
+    }
 }
diff --git a/crates/nova-typeck/src/check/lang_tests.rs b/crates/nova-typeck/src/check/lang_tests.rs
index 86583dfb065ce8818b600730ef0103a6c49928c8..61ce85f93b605aa6ce299b31714c19952c658318 100644
--- a/crates/nova-typeck/src/check/lang_tests.rs
+++ b/crates/nova-typeck/src/check/lang_tests.rs
@@ -1,10 +1,10 @@
 //! The checker's tests of 5.0a's language (spec
 //! `docs/superpowers/specs/2026-10-26-phase-5-0-the-language-and-std-a-compiler-needs-design.md`
-//! §3.3, §3.4 and §3.6): `?` and its codes E0098 and E0099, `+=` on
-//! fields and elements, and `String + String`.
+//! §3.3 to §3.6): `?` and its codes E0098 and E0099, `+=` on fields and
+//! elements, `String + String`, and E0103.
 
 use super::*;
-use nova_diagnostics::FileId;
+use nova_diagnostics::{FileId, Severity};
 use nova_lexer::lex;
 use nova_parser::parse;
 use nova_resolver::resolve;
@@ -380,3 +380,144 @@ fn string_plus_char_or_int_is_refused_with_a_note() {
 fn string_minus_string_is_still_e0013() {
     assert!(codes("fn main() {\n    let s = \"a\" - \"b\"\n}\n").contains(&"E0013".to_string()));
 }
+
+// --- E0103 ---
+
+fn e0103(body: &str) -> usize {
+    diags(&format!(
+        "fn g() -> Int {{ 1 }}\nfn main() {{\n    let x = 1\n    let y = 2\n{body}\n    println(\"${{x}}\")\n}}\n"
+    ))
+    .iter()
+    .filter(|d| d.code == "E0103")
+    .inspect(|d| assert_eq!(d.severity, Severity::Warning))
+    .count()
+}
+
+#[test]
+fn e0103_flags_each_form_whose_value_is_discarded() {
+    for stmt in [
+        "    -y",
+        "    !true",
+        "    x + y",
+        "    x == y",
+        "    1",
+        "    \"text\"",
+        "    y",
+        "    (x)",
+        "    (x, y)",
+        "    [x, y]",
+        "    (g())",
+    ] {
+        assert_eq!(e0103(stmt), 1, "{stmt}");
+    }
+}
+
+#[test]
+fn e0103_flags_what_the_newline_rule_leaves() {
+    // `let z = x` then `-y`: two statements, the second discarded.
+    assert_eq!(e0103("    let z = x\n    -y\n    println(\"${z}\")"), 1);
+}
+
+#[test]
+fn e0103_is_not_on_calls_assignments_conditions_or_a_tail() {
+    for stmt in [
+        "    g()",
+        "    let mut m = 1\n    m = 2\n    m += 1",
+        "    x == 1 || y == 2",
+        "    x == 1 && y == 2",
+        "    if x > 0 { g() }",
+        "    match x { _ => g() }",
+        "    while false {}",
+        "    { g() }",
+    ] {
+        assert_eq!(e0103(stmt), 0, "{stmt}");
+    }
+    // A block's tail is not a statement.
+    let all = diags("fn f(x: Int) -> Int {\n    x\n}\nfn main() {}\n");
+    assert!(all.is_empty(), "{all:?}");
+}
+
+#[test]
+fn e0103_reports_a_parenthesised_callee_after_a_line_break() {
+    // `x` then `(g)()`: `x` is an unused name, and the line break that ends
+    // it before `(g)()` is reported too, though `unused_value` cannot tell
+    // the call was written in parentheses.
+    assert_eq!(e0103("    x\n    (g)()"), 2);
+}
+
+// --- E0103 at the newline rule's splits (the user's ruling of 2026-10-27) ---
+
+/// Each E0103's message, for `src`.
+fn e0103_messages(src: &str) -> Vec<String> {
+    diags(src)
+        .into_iter()
+        .filter(|d| d.code == "E0103")
+        .map(|d| d.message)
+        .collect()
+}
+
+#[test]
+fn a_split_after_a_return_value_warns() {
+    let all = e0103_messages(
+        "fn net(a: Int, b: Int) -> Int {\n    return a\n        - b\n}\nfn main() {}\n",
+    );
+    assert_eq!(all, ["a line break ends the statement before this `-`"]);
+}
+
+#[test]
+fn a_split_before_a_tail_warns() {
+    let all = e0103_messages(
+        "record R { a: Int, b: Int }\nfn w(r: R) -> Int {\n    r.a\n    - r.b\n}\nfn main() {}\n",
+    );
+    assert_eq!(all, ["a line break ends the statement before this `-`"]);
+    let all = e0103_messages(
+        "fn k(x: Int) -> fn(Int) -> Int { |y: Int| x + y }\nfn main() {\n    let v = k(1)\n    (5)\n    println(\"${v(2)}\")\n}\n",
+    );
+    // `(5)` is also an unused value: one warning, with the line break's note.
+    assert_eq!(all, ["this value is not used"]);
+}
+
+#[test]
+fn a_split_after_a_bare_return_warns() {
+    let all = e0103_messages("fn g() {}\nfn f() {\n    return\n    g()\n}\nfn main() {}\n");
+    assert_eq!(all, ["a line break ends this `return` or `break`"]);
+}
+
+#[test]
+fn a_written_semicolon_silences_the_split() {
+    assert!(
+        e0103_messages("fn net(a: Int, b: Int) -> Int {\n    a;\n    -b\n}\nfn main() {}\n")
+            .iter()
+            .all(|m| m != "a line break ends the statement before this `-`")
+    );
+    assert!(
+        e0103_messages("fn g() {}\nfn f() {\n    return;\n    g()\n}\nfn main() {}\n").is_empty()
+    );
+}
+
+#[test]
+fn an_unused_value_at_a_split_gets_one_warning_with_the_line_breaks_note() {
+    let all: Vec<Diagnostic> = diags(
+        "fn main() {\n    let x = 1\n    let y = 2\n    let z = x\n    -y\n    println(\"${z}\")\n}\n",
+    )
+    .into_iter()
+    .filter(|d| d.code == "E0103")
+    .collect();
+    assert_eq!(all.len(), 1, "{all:?}");
+    assert_eq!(all[0].message, "this value is not used");
+    assert!(
+        all[0]
+            .notes
+            .iter()
+            .any(|n| n.contains("a line break ended the statement")),
+        "{all:?}"
+    );
+    // Without a line break, no such note.
+    let all: Vec<Diagnostic> =
+        diags("fn main() {\n    let y = 2\n    let z = 1; -y\n    println(\"${z}\")\n}\n")
+            .into_iter()
+            .filter(|d| d.code == "E0103")
+            .collect();
+    assert_eq!(all.len(), 1, "{all:?}");
+    assert!(all[0].notes.is_empty(), "{all:?}");
+}
````

**Patch `t6-code`:**

<!-- patch t6-code -->

````diff
diff --git a/crates/nova-ast/src/lib.rs b/crates/nova-ast/src/lib.rs
index a639d13c05676d1baacb6d4086a103f89eaa22d6..053c4dfce096cd2c84741713acf5cd026b9dc07d 100644
--- a/crates/nova-ast/src/lib.rs
+++ b/crates/nova-ast/src/lib.rs
@@ -23,12 +23,54 @@ pub use item::{
 pub use pattern::Pattern;
 pub use ty::{Type, TypeParam};
 
-use nova_diagnostics::Spanned;
+use nova_diagnostics::{Span, Spanned};
 
 /// The top-level compilation unit — one `.nova` source file.
-#[derive(Debug, Clone)]
+#[derive(Clone, Default)]
 pub struct File {
     pub items: Vec<Spanned<Item>>,
+    /// Each place where the newline rule ended a statement that the parser
+    /// before it would have continued (5.0 spec
+    /// `docs/superpowers/specs/2026-10-26-phase-5-0-the-language-and-std-a-compiler-needs-design.md`
+    /// §3.5), in source order: the checker warns at each (E0103).
+    pub newline_splits: Vec<NewlineSplit>,
+}
+
+/// `items`, and `newline_splits` only when there are any, so a file the
+/// rule does not split prints as it did before the rule (`nova parse
+/// --ast`).
+impl std::fmt::Debug for File {
+    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
+        let mut d = f.debug_struct("File");
+        d.field("items", &self.items);
+        if !self.newline_splits.is_empty() {
+            d.field("newline_splits", &self.newline_splits);
+        }
+        d.finish()
+    }
+}
+
+/// A place where a line break ended a statement in statement context: the
+/// token after the break, and what the parser before the rule would have
+/// read there.
+#[derive(Debug, Clone, Copy, PartialEq, Eq)]
+pub struct NewlineSplit {
+    /// The first token of the line the rule made a new statement.
+    pub at: Span,
+    pub kind: SplitKind,
+}
+
+/// What a line break ended, by the token that begins the next line.
+#[derive(Debug, Clone, Copy, PartialEq, Eq)]
+pub enum SplitKind {
+    /// A `(`, which would have called the expression before.
+    Call,
+    /// A `[`, which would have indexed it.
+    Index,
+    /// A binary `-`, which would have subtracted.
+    Minus,
+    /// A line after a `return` or `break`, which would have been its value.
+    Value,
 }
 
 /// Every kind of top-level item that can appear in a Nova file.
diff --git a/crates/nova-cli/src/cmd/parse.rs b/crates/nova-cli/src/cmd/parse.rs
index 1ac38687098502d97d6abc6ba340bac5a9d1daa5..674d601be9e4d24ad93fb5ba609babefc0d9c57a 100644
--- a/crates/nova-cli/src/cmd/parse.rs
+++ b/crates/nova-cli/src/cmd/parse.rs
@@ -49,8 +49,11 @@ pub fn run(cmd: ParseCmd) -> Result<()> {
     let parse_diags: Vec<_> = parse_errors
         .iter()
         .map(|e| {
-            nova_diagnostics::Diagnostic::error("P0001", e.to_string())
-                .with_primary_label(e.span(), "here")
+            e.notes().into_iter().fold(
+                nova_diagnostics::Diagnostic::error("P0001", e.to_string())
+                    .with_primary_label(e.span(), "here"),
+                |d, n| d.with_note(n),
+            )
         })
         .collect();
     if !parse_diags.is_empty() {
diff --git a/crates/nova-driver/src/program.rs b/crates/nova-driver/src/program.rs
index 470137ce0f88d5e5f66002626ea54ac5a7f72fe2..5e52df32ae421a0321df93b13b989772ea2335af 100644
--- a/crates/nova-driver/src/program.rs
+++ b/crates/nova-driver/src/program.rs
@@ -430,7 +430,10 @@ pub(crate) fn load_program(
         }));
         let parsed = nova_parser::parse_recovering(&tokens, file);
         load.diagnostics.extend(parsed.errors.iter().map(|e| {
-            Diagnostic::error("P0001", e.to_string()).with_primary_label(e.span(), "here")
+            e.notes().into_iter().fold(
+                Diagnostic::error("P0001", e.to_string()).with_primary_label(e.span(), "here"),
+                |d, n| d.with_note(n),
+            )
         }));
         load.dropped
             .extend(parsed.dropped.into_iter().map(|n| n.value));
diff --git a/crates/nova-fmt/src/check.rs b/crates/nova-fmt/src/check.rs
index 4d5183d88b1464295f7b3fa0cd2cf04d3982eb20..dc22dfde63f565fdb60549a307565fe884aa738c 100644
--- a/crates/nova-fmt/src/check.rs
+++ b/crates/nova-fmt/src/check.rs
@@ -51,10 +51,16 @@ fn internal(first_difference: String) -> FormatError {
 }
 
 /// The AST without its spans, after the import sorting the printer does
-/// (spec §6). Two inputs have the same fingerprint exactly when they are
-/// the same program.
+/// (spec §6), then the kinds of the newline rule's splits, in order, without
+/// their places, which move with the layout (5.0 spec §3.5). Two inputs
+/// have the same fingerprint exactly when they are the same program with the
+/// same warnings at its line breaks.
 pub(crate) fn fingerprint(file: &File) -> String {
     let mut file = file.clone();
+    let splits: Vec<_> = std::mem::take(&mut file.newline_splits)
+        .iter()
+        .map(|s| s.kind)
+        .collect();
     for run in import_runs(&file) {
         let items = &mut file.items[run];
         items.sort_by_key(|a| import_key(&a.value));
@@ -68,7 +74,7 @@ pub(crate) fn fingerprint(file: &File) -> String {
             }
         }
     }
-    strip_spans(&format!("{file:?}"))
+    format!("{}; splits {splits:?}", strip_spans(&format!("{file:?}")))
 }
 
 /// The index ranges of the runs of consecutive top-level imports.
@@ -208,6 +214,19 @@ fn around(s: &str, at: usize) -> &str {
 mod tests {
     use super::*;
 
+    #[test]
+    fn the_fingerprint_tells_splits_by_kind_not_by_place() {
+        let fp = |text: &str| {
+            let src = Source::parse(text, "<test>").expect("it parses");
+            fingerprint(&src.file)
+        };
+        let split = fp("fn f(a: Int, b: Int) -> Int {\n    a\n    - b\n}\n");
+        let moved = fp("fn f(a: Int,  b: Int) -> Int {\n    a\n        - b\n}\n");
+        let unsplit = fp("fn f(a: Int, b: Int) -> Int {\n    a;\n    - b\n}\n");
+        assert_eq!(split, moved);
+        assert_ne!(split, unsplit);
+    }
+
     #[test]
     fn strip_spans_removes_every_span_field() {
         let debug =
diff --git a/crates/nova-fmt/src/print/expr.rs b/crates/nova-fmt/src/print/expr.rs
index 8806114bda76349d2b11217c5611ade3b28e5e4c..59144b9ef71de508b2f97df3d5c79d19b239dae7 100644
--- a/crates/nova-fmt/src/print/expr.rs
+++ b/crates/nova-fmt/src/print/expr.rs
@@ -347,9 +347,9 @@ impl<'s, 't> Printer<'s, 't> {
     /// the first (spec §5.6). A `;` after an item would not parse, and no
     /// item can be continued. Before a statement that starts with `(`, `[`
     /// or `-`, and after a `return` or `break` with no value, the newline
-    /// rule ends the statement at the line break (5.0 spec §3.5), and a `;`
-    /// written there says the break is meant: fmt keeps the author's `;`,
-    /// and adds none, so the program stays as it was.
+    /// rule ends the statement at the line break (5.0 spec §3.5) and warns
+    /// (E0103) unless a `;` is written there: fmt keeps the author's `;`,
+    /// and adds none, so the program and its warnings stay as they were.
     fn separator(&self, cur: Elem, next: Elem) -> Option<&'static str> {
         if let Elem::Stmt(s) = cur {
             if matches!(s.value, Stmt::Item(_)) {
diff --git a/crates/nova-fmt/src/print/mod.rs b/crates/nova-fmt/src/print/mod.rs
index 10ad468d38e8d960742a8feec78f64af05cd4786..04ee8c91cd8aa0e9ed9066fcad8eae69ae2cd347 100644
--- a/crates/nova-fmt/src/print/mod.rs
+++ b/crates/nova-fmt/src/print/mod.rs
@@ -33,7 +33,7 @@ const CONTINUES: [Token; 5] = [
 ];
 
 /// Tokens that begin a new statement after a line break under the newline
-/// rule, where a `;` the author wrote says the break is meant: fmt keeps
+/// rule, which warns there (E0103) unless the author wrote a `;`: fmt keeps
 /// the author's `;` before them, and adds none.
 const SPLITS: [Token; 3] = [Token::LParen, Token::LBracket, Token::Minus];
 
diff --git a/crates/nova-fmt/src/source.rs b/crates/nova-fmt/src/source.rs
index 90fecd12df6e21573d95bad73e05e8f2bc74bb61..d3879a17a9d4762ad610c931db27e28161a895ae 100644
--- a/crates/nova-fmt/src/source.rs
+++ b/crates/nova-fmt/src/source.rs
@@ -32,7 +32,10 @@ impl<'t> Source<'t> {
             .map(|e| Diagnostic::error("L0001", e.to_string()).with_primary_label(e.span(), "here"))
             .collect();
         diagnostics.extend(parse_errors.iter().map(|e| {
-            Diagnostic::error("P0001", e.to_string()).with_primary_label(e.span(), "here")
+            e.notes().into_iter().fold(
+                Diagnostic::error("P0001", e.to_string()).with_primary_label(e.span(), "here"),
+                |d, n| d.with_note(n),
+            )
         }));
         match file {
             Some(file) if diagnostics.is_empty() => Ok(Self {
diff --git a/crates/nova-parser/src/error.rs b/crates/nova-parser/src/error.rs
index 3ce40ddfc44aabe37e560c6701ef89dc8d84c281..71150de482d350b2c13b8b70c1c591ab641bf697 100644
--- a/crates/nova-parser/src/error.rs
+++ b/crates/nova-parser/src/error.rs
@@ -21,6 +21,13 @@ pub enum ParseError {
 
     #[error("{message}")]
     Custom { message: String, span: Span },
+
+    /// `error`, with a note that says why it may have come about.
+    #[error("{error}")]
+    Noted {
+        error: Box<ParseError>,
+        note: String,
+    },
 }
 
 impl ParseError {
@@ -30,6 +37,19 @@ impl ParseError {
             ParseError::UnexpectedEof { span } => *span,
             ParseError::ChainedComparison { span } => *span,
             ParseError::Custom { span, .. } => *span,
+            ParseError::Noted { error, .. } => error.span(),
+        }
+    }
+
+    /// The notes that go with the error's diagnostic.
+    pub fn notes(&self) -> Vec<String> {
+        match self {
+            ParseError::Noted { error, note } => {
+                let mut notes = error.notes();
+                notes.push(note.clone());
+                notes
+            }
+            _ => Vec::new(),
         }
     }
 }
diff --git a/crates/nova-parser/src/grammar.rs b/crates/nova-parser/src/grammar.rs
index 7b83d9bf9fe302e1eb8b20800c77ffc51fe8a489..13c8e3394a3fb3e79d8f400fe0b0b32c307561a1 100644
--- a/crates/nova-parser/src/grammar.rs
+++ b/crates/nova-parser/src/grammar.rs
@@ -15,7 +15,7 @@ use nova_ast::{
     },
     pattern::{FieldPat, Pattern},
     ty::{Type, TypeParam},
-    Block, File, Item, Path, Stmt,
+    Block, File, Item, NewlineSplit, Path, SplitKind, Stmt,
 };
 use nova_diagnostics::{FileId, Span, Spanned};
 use nova_lexer::{Token, Tokens};
@@ -59,6 +59,16 @@ struct Parser<'a> {
     /// `docs/superpowers/specs/2026-10-08-phase-3-2-lsp-core-design.md`
     /// §3.2). The language server stops reporting their uses as unresolved.
     dropped: Vec<Spanned<String>>,
+    /// Each place the newline rule ended a statement the parser before it
+    /// would have continued (spec §3.5), for the checker's E0103.
+    splits: Vec<NewlineSplit>,
+    /// The position after a `return` or `break` that a line break ended:
+    /// no operator, call or index there takes the bare keyword as its
+    /// operand. Cleared when the expression that holds it ends.
+    halt: Option<usize>,
+    /// The split, before a `(`, `[` or `-`, that ended the last match arm's
+    /// body: a P0001 at the next arm's start gets a note naming the rule.
+    arm_end_split: Option<NewlineSplit>,
 }
 
 impl<'a> Parser<'a> {
@@ -72,6 +82,9 @@ impl<'a> Parser<'a> {
             stmt_ctx: false,
             pending_gt: 0,
             dropped: Vec::new(),
+            splits: Vec::new(),
+            halt: None,
+            arm_end_split: None,
         }
     }
 
@@ -127,14 +140,73 @@ impl<'a> Parser<'a> {
     /// Whether a `return` or `break` just read takes no value: the next
     /// token ends a statement or a block, or a line break comes before it,
     /// in any context.
-    fn bare_keyword_ends(&self) -> bool {
-        matches!(self.peek(), Token::Semicolon | Token::RBrace | Token::Eof)
-            || self.line_break_here()
+    fn bare_keyword_ends(&mut self) -> bool {
+        if matches!(self.peek(), Token::Semicolon | Token::RBrace | Token::Eof) {
+            return true;
+        }
+        if !self.line_break_here() {
+            return false;
+        }
+        // A `let` or an item after the line break is a statement of its own.
+        if matches!(self.peek(), Token::Let) || self.is_item_start_at(self.pos) {
+            self.halt = Some(self.pos);
+            return true;
+        }
+        // A token that cannot begin an expression, such as `)`, `,` or
+        // `=>`, is read as before the rule: as the value's start, an error.
+        if !can_start_expr(self.peek()) {
+            return false;
+        }
+        // The line after would have been the value: the rule split it off.
+        self.record_split(SplitKind::Value);
+        self.halt = Some(self.pos);
+        true
+    }
+
+    /// Whether the parser stands where a line break ended a bare `return` or
+    /// `break`: the token there belongs to whatever follows the keyword's
+    /// expression, not to the expression.
+    fn halted(&self) -> bool {
+        self.halt == Some(self.pos)
+    }
+
+    /// A P0001 at the start of an arm that the newline rule began, by ending
+    /// the arm before at a line break before a `(`, `[` or `-`: a note says
+    /// so, since the parser before the rule read that line as part of the
+    /// body.
+    fn note_arm_split(&mut self, prior: Option<NewlineSplit>) {
+        let Some(split) = prior else {
+            return;
+        };
+        let Some(last) = self.errors.pop() else {
+            return;
+        };
+        if last.span().start != split.at.start {
+            self.errors.push(last);
+            return;
+        }
+        self.errors.push(ParseError::Noted {
+            error: Box::new(last),
+            note: "a line that starts with `(`, `[` or `-` begins the next match arm \
+                   (the newline rule); to continue the arm's body, end the line before \
+                   with the operator"
+                .to_string(),
+        });
+    }
+
+    /// Record that the newline rule ends a statement before the token at
+    /// the cursor, which the parser before the rule would have continued.
+    fn record_split(&mut self, kind: SplitKind) {
+        let at = self.peek_span();
+        self.splits.push(NewlineSplit { at, kind });
     }
 
     // --- Token access ---
 
     fn peek(&self) -> &Token {
+        if self.halted() {
+            return &HALT;
+        }
         self.tokens
             .get(self.pos)
             .map(|s| &s.value)
@@ -431,7 +503,13 @@ impl<'a> Parser<'a> {
                 }
             }
         }
-        File { items }
+        let mut newline_splits = std::mem::take(&mut self.splits);
+        newline_splits.sort_by_key(|s| s.at.start);
+        newline_splits.dedup();
+        File {
+            items,
+            newline_splits,
+        }
     }
 
     /// The item that started at token `start` failed. If it got as far as
@@ -1585,7 +1663,13 @@ impl<'a> Parser<'a> {
 
 impl<'a> Parser<'a> {
     fn parse_expr(&mut self, ctx: &str) -> Option<Spanned<Expr>> {
-        self.parse_assign(ctx)
+        let expr = self.parse_assign(ctx);
+        // The expression that holds a bare `return` or `break` ends here;
+        // what follows reads the token again.
+        if self.halted() {
+            self.halt = None;
+        }
+        expr
     }
 
     fn parse_assign(&mut self, ctx: &str) -> Option<Spanned<Expr>> {
@@ -1811,7 +1895,10 @@ impl<'a> Parser<'a> {
                 Token::Plus => BinOp::Add,
                 // A `-` after a line break in statement context begins the
                 // next statement (spec §3.5).
-                Token::Minus if self.newline_ends_expr() => break,
+                Token::Minus if self.newline_ends_expr() => {
+                    self.record_split(SplitKind::Minus);
+                    break;
+                }
                 Token::Minus => BinOp::Sub,
                 _ => break,
             };
@@ -1936,7 +2023,15 @@ impl<'a> Parser<'a> {
             match self.peek().clone() {
                 // A `(` or `[` after a line break in statement context
                 // begins the next statement (spec §3.5).
-                Token::LParen | Token::LBracket if self.newline_ends_expr() => break,
+                Token::LParen | Token::LBracket if self.newline_ends_expr() => {
+                    let kind = if self.check(&Token::LParen) {
+                        SplitKind::Call
+                    } else {
+                        SplitKind::Index
+                    };
+                    self.record_split(kind);
+                    break;
+                }
                 Token::LParen => {
                     // Function call
                     self.advance();
@@ -2371,9 +2466,11 @@ impl<'a> Parser<'a> {
             if self.reject_docs() {
                 continue;
             }
+            let prior = self.arm_end_split.take();
             if let Some(arm) = self.parse_match_arm(ctx) {
                 arms.push(arm);
             } else {
+                self.note_arm_split(prior);
                 self.sync_to_stmt_boundary();
             }
         }
@@ -2403,7 +2500,21 @@ impl<'a> Parser<'a> {
         self.expect(&Token::FatArrow, "match arm")?;
         // An arm's body is statement context, so a line that starts with
         // `(` begins the next arm (spec §3.5).
+        let splits = self.splits.len();
         let body = self.in_stmt_ctx(true, |p| p.parse_expr(ctx))?;
+        // A split at the body's end begins the next arm, which the parser
+        // before the rule could not have read as part of the body: no
+        // warning for it, however many times it was met (a `return`'s value
+        // and the `return` around it each meet it). A P0001 there gets a
+        // note naming the rule.
+        self.arm_end_split = None;
+        let end = self.peek_span().start;
+        while self.splits.len() > splits && self.splits.last().map(|s| s.at.start) == Some(end) {
+            let split = self.splits.pop();
+            if let Some(s) = split.filter(|s| s.kind != SplitKind::Value) {
+                self.arm_end_split = Some(s);
+            }
+        }
         self.eat(&Token::Comma);
         Some(MatchArm {
             pattern,
@@ -3086,3 +3197,43 @@ fn type_to_path(ty: &Spanned<Type>) -> Option<Path> {
         _ => None,
     }
 }
+
+/// What [`Parser::peek`] reads where a line break ended a bare `return` or
+/// `break` (`Parser::halted`): nothing, so no operator, call or index takes
+/// the keyword as its operand.
+static HALT: Token = Token::Eof;
+
+/// Whether `tok` can begin an expression: a literal, a name, a bracket, a
+/// prefix operator, a closure, or a keyword that begins one.
+fn can_start_expr(tok: &Token) -> bool {
+    matches!(
+        tok,
+        Token::Int(_)
+            | Token::Float(_)
+            | Token::Char(_)
+            | Token::True
+            | Token::False
+            | Token::StrStart
+            | Token::RawStr(_)
+            | Token::LParen
+            | Token::LBracket
+            | Token::LBrace
+            | Token::If
+            | Token::Match
+            | Token::While
+            | Token::For
+            | Token::Return
+            | Token::Break
+            | Token::Continue
+            | Token::Pipe
+            | Token::PipePipe
+            | Token::Ident(_)
+            | Token::SelfLower
+            | Token::SelfUpper
+            | Token::Minus
+            | Token::Bang
+            | Token::Tilde
+            | Token::Star
+            | Token::Amp
+    )
+}
diff --git a/crates/nova-resolver/src/lib.rs b/crates/nova-resolver/src/lib.rs
index b62b0d0a9f3509fe09e4dec0c373a686180e52a5..324d15bbe98e90a378ea14f2582c4cb84f6e6b4a 100644
--- a/crates/nova-resolver/src/lib.rs
+++ b/crates/nova-resolver/src/lib.rs
@@ -2074,6 +2074,12 @@ pub fn resolve_program_for(
         import_std_module(&mut definitions, std_exports, mid, from);
     }
 
+    // E0102 (5.0 spec §3.7): a variant of a program's own sum that hides a
+    // variant std brings into every module. Std's modules are never warned
+    // about; a registry package's warnings are dropped by the driver, so
+    // what is left is the package's own and its path dependencies'.
+    diagnostics.extend(variant_shadowing(&definitions, &exports, &all, std_start));
+
     // Spec 3.4b §4.6: what a fix needs to know.
     definitions.importable = all
         .iter()
@@ -2091,7 +2097,15 @@ pub fn resolve_program_for(
     definitions.missing = missing_names(target);
 
     ProgramResolution {
-        file: File { items: merged },
+        // The newline rule's splits of the user's modules, for E0103; std's
+        // are std's own.
+        file: File {
+            items: merged,
+            newline_splits: all[..std_start.min(all.len())]
+                .iter()
+                .flat_map(|m| m.file.newline_splits.iter().copied())
+                .collect(),
+        },
         definitions,
         diagnostics,
         tests,
@@ -2099,6 +2113,74 @@ pub fn resolve_program_for(
     }
 }
 
+/// The variants std brings into every module, by name: each public variant
+/// of a std module whose names glob into user modules, with its sum's name.
+/// The first module to export a name wins, as std's glob does.
+fn std_globbed_variants(
+    definitions: &Definitions,
+    exports: &[Exports],
+    all: &[ModuleSource],
+    std_start: usize,
+) -> FxHashMap<String, String> {
+    let mut out: FxHashMap<String, String> = FxHashMap::default();
+    for (mid, e) in exports.iter().enumerate().skip(std_start) {
+        let name = all[mid].name.as_str();
+        if IMPORTED_STD.iter().any(|&(_, m)| m == name) || STD_INTERNAL.contains(&name) {
+            continue;
+        }
+        let mut variants: Vec<(&String, &Res)> = e.values.iter().collect();
+        variants.sort_by(|a, b| a.0.cmp(b.0));
+        for (variant, res) in variants {
+            if let Res::Variant(sum, _) = res {
+                out.entry(variant.clone())
+                    .or_insert_with(|| definitions.def(*sum).name.clone());
+            }
+        }
+    }
+    out
+}
+
+/// E0102, a warning at each variant of a non-std module's sum whose name is
+/// also a variant std brings into every module (5.0 spec §3.7): in that
+/// module, the bare name now means the program's own variant.
+fn variant_shadowing(
+    definitions: &Definitions,
+    exports: &[Exports],
+    all: &[ModuleSource],
+    std_start: usize,
+) -> Vec<Diagnostic> {
+    let std_variants = std_globbed_variants(definitions, exports, all, std_start);
+    let mut out = Vec::new();
+    for m in &all[..std_start.min(all.len())] {
+        for item in &m.file.items {
+            let Item::Type(t) = &item.value else {
+                continue;
+            };
+            let TypeDef::Sum(variants) = &t.def else {
+                continue;
+            };
+            for v in variants {
+                let Some(sum) = std_variants.get(&v.name.value) else {
+                    continue;
+                };
+                let name = &v.name.value;
+                out.push(
+                    Diagnostic::warning(
+                        "E0102",
+                        format!("variant `{name}` hides std's `{sum}::{name}`"),
+                    )
+                    .with_primary_label(v.name.span, "this variant")
+                    .with_note(format!(
+                        "in this module the bare name `{name}` now means this variant; \
+                             std's is `{sum}::{name}`"
+                    )),
+                );
+            }
+        }
+    }
+    out
+}
+
 /// Each public name of each std module `target` does not compile, mapped to
 /// that module's name; the first module to export a name keeps it
 /// (spec 4.1 §3.2). The modules are parsed and their items collected as
@@ -2279,11 +2361,12 @@ fn std_module(src: &str, file_id: FileId) -> (Option<File>, Vec<Diagnostic>) {
         .map(|e| Diagnostic::error("L0001", e.to_string()).with_primary_label(e.span(), "here"))
         .collect();
     let (ast, parse_errors) = nova_parser::parse(&tokens, file_id);
-    diags.extend(
-        parse_errors.iter().map(|e| {
-            Diagnostic::error("P0001", e.to_string()).with_primary_label(e.span(), "here")
-        }),
-    );
+    diags.extend(parse_errors.iter().map(|e| {
+        e.notes().into_iter().fold(
+            Diagnostic::error("P0001", e.to_string()).with_primary_label(e.span(), "here"),
+            |d, n| d.with_note(n),
+        )
+    }));
     (ast, diags)
 }
 
diff --git a/crates/nova-typeck/src/check.rs b/crates/nova-typeck/src/check.rs
index d4b7936f98e6bbc92a5b5b090c7cab39636459dd..edd4bdfd0c63c694e2fb81d3ec828348254d5dc4 100644
--- a/crates/nova-typeck/src/check.rs
+++ b/crates/nova-typeck/src/check.rs
@@ -215,6 +215,7 @@ pub fn check_with(file: &ast::File, defs: &Definitions, options: &CheckOptions)
         }
     }
     checker.check_const_cycles(&functions);
+    checker.warn_newline_splits();
     // Closure and bare-fn-wrapper functions synthesized while checking.
     functions.append(&mut checker.extra_functions);
 
@@ -3364,6 +3365,12 @@ impl<'a> Checker<'a> {
                 }
                 ast::Stmt::Expr(e) => {
                     stmts.push(self.check_expr(fcx, e));
+                    if let Some(what) = unused_value(e) {
+                        self.diagnostics.push(
+                            Diagnostic::warning("E0103", "this value is not used")
+                                .with_primary_label(e.span, format!("this {what} is discarded")),
+                        );
+                    }
                 }
                 ast::Stmt::Item(_) => {
                     self.unsupported(stmt.span, "items nested inside function bodies");
@@ -4899,6 +4906,64 @@ impl<'a> Checker<'a> {
         })
     }
 
+    /// E0103 at each place the newline rule ended a statement the parser
+    /// before it would have continued (5.0 spec §3.5; the user's ruling of
+    /// 2026-10-27): the parser records them, and a `;` written there means
+    /// the author ended the statement, so none is recorded. Where the new
+    /// statement is also a value nobody uses, its "this value is not used"
+    /// takes the note instead of a second warning.
+    fn warn_newline_splits(&mut self) {
+        for split in &self.file.newline_splits {
+            let note = match split.kind {
+                ast::SplitKind::Value => {
+                    "a `return` or `break` at the end of a line takes no value: to return \
+                     this line's value, put it on the same line; to keep this line a \
+                     statement of its own, write `;` after the `return` or `break`"
+                }
+                ast::SplitKind::Call | ast::SplitKind::Index | ast::SplitKind::Minus => {
+                    "a line that starts with `(`, `[` or `-` begins a new statement: to \
+                     continue the line before, end that line with the operator, or put the \
+                     `(` or `[` on it; to keep two statements, end that line with `;`"
+                }
+            };
+            let unused = self.diagnostics.iter_mut().find(|d| {
+                d.code == "E0103"
+                    && d.labels.first().is_some_and(|l| {
+                        l.span.file == split.at.file && l.span.start == split.at.start
+                    })
+            });
+            if let Some(d) = unused {
+                d.notes.push(format!(
+                    "a line break ended the statement before this one; {note}"
+                ));
+                continue;
+            }
+            let (message, label) = match split.kind {
+                ast::SplitKind::Value => (
+                    "a line break ends this `return` or `break`",
+                    "this line is not its value",
+                ),
+                ast::SplitKind::Call => (
+                    "a line break ends the statement before this `(`",
+                    "this `(` begins a new statement",
+                ),
+                ast::SplitKind::Index => (
+                    "a line break ends the statement before this `[`",
+                    "this `[` begins a new statement",
+                ),
+                ast::SplitKind::Minus => (
+                    "a line break ends the statement before this `-`",
+                    "this `-` begins a new statement",
+                ),
+            };
+            self.diagnostics.push(
+                Diagnostic::warning("E0103", message)
+                    .with_primary_label(split.at, label)
+                    .with_note(note.to_string()),
+            );
+        }
+    }
+
     /// The `DefId` of std/core's sum named `name` (`Option` or `Result`),
     /// whatever the name means in the module being checked (5.0 spec §3.3).
     fn core_sum(&self, name: &str) -> Option<DefId> {
@@ -8302,6 +8367,39 @@ fn lit_expr(lit: &ast::Literal, span: Span) -> hir::Expr {
     hir::Expr { kind, ty, span }
 }
 
+/// What a statement `e` is, when its value can only be discarded: E0103
+/// (5.0 spec §3.5), which reports what the newline rule leaves behind, such
+/// as a `-y` or a `(w)` on a line of its own. Calls, assignments, `&&`,
+/// `||`, blocks, `if`, `match` and the rest are not reported, nor is a
+/// block's tail, which is not a statement.
+fn unused_value(e: &Spanned<ast::Expr>) -> Option<&'static str> {
+    use ast::Expr as E;
+    // The span of a parenthesised expression starts at its `(`, before the
+    // operand it starts with.
+    let paren = |first: &Spanned<ast::Expr>| e.span.start < first.span.start;
+    match &e.value {
+        E::Lit(_) | E::StringInterp(_) => Some("literal"),
+        E::Path(_) => Some("name"),
+        E::Unary { .. } => Some("operator expression"),
+        E::Binary { op, .. } if !matches!(op, ast::BinOp::And | ast::BinOp::Or) => {
+            Some("operator expression")
+        }
+        E::Tuple(items) if !items.is_empty() => Some("tuple"),
+        E::Array(_) | E::ArrayRepeat { .. } => Some("array"),
+        E::Call { callee: first, .. }
+        | E::Field { target: first, .. }
+        | E::Index { target: first, .. }
+        | E::Try(first)
+        | E::Await(first)
+        | E::Cast { expr: first, .. }
+            if paren(first) =>
+        {
+            Some("parenthesised expression")
+        }
+        _ => None,
+    }
+}
+
 /// A read of `local`, of type `ty`.
 fn local_expr(local: LocalId, ty: Ty, span: Span) -> hir::Expr {
     hir::Expr {
@@ -14065,7 +14163,7 @@ mod tests {
         // `self.impls`, or any other field a full `check()` pass would
         // populate, so this needs only enough of a `Checker` to compile --
         // mirroring `check()`'s own construction above.
-        let file = ast::File { items: Vec::new() };
+        let file = ast::File::default();
         let checker = Checker {
             file: &file,
             defs: &resolved.definitions,
@@ -14142,7 +14240,7 @@ mod tests {
             "resolve errors: {:?}",
             resolved.diagnostics
         );
-        let file = ast::File { items: Vec::new() };
+        let file = ast::File::default();
         let checker = Checker {
             file: &file,
             defs: &resolved.definitions,
````


### Task 7: The golden suite through LLVM, and CI

Spec §3.2, §12.2 to §12.4; the user's decision 7; decisions 4, 5, 25, 26
and 32.

**Files:**
- Test: `crates/nova-cli/tests/run_tests.rs` (`RELEASE_GOLDEN_EXCLUDED`,
  `RELEASE_GOLDEN_STDIN`, `RELEASE_GOLDEN_STATUS`, `RELEASE_GOLDEN_LIMIT`,
  `run_release_binary`, `release_golden_suite`)
- Modify: `.github/workflows/ci.yml` (the `test` job)

- [ ] **Step 1: The suite and CI.** `patch_block t7-code`:
  - **`run_tests.rs`,** after `release_builds_and_runs_when_clang_available`,
    which is unchanged: the exclusion table of nine, each with why; the
    stdin table, `io_streams` and `hello-stdin`; the empty status table;
    the 60-second limit; `run_release_binary`, which runs a binary with
    `TMPDIR`, `TMP` and `TEMP` set to its own directory,
    `NOVA_GC_STRESS` removed, standard input piped or closed, standard
    output read on a thread, stderr dropped, and kills it at the limit;
    and `release_golden_suite`, gated on `NOVA_REQUIRE_CLANG=1`, whose skip
    branch asserts that the variable is not `1` (decision 35), and which
    reads `NOVA_CLANG`'s or `clang`'s `--version` first line and fails
    when it does not run, checks the exclusion table against the files,
    requires a walk of at least 131 programs,
    builds each program with `nova build --release` into
    `nova-release-golden/` on `min(4, cores)` threads, compares its output
    with `\r` removed and its exit status, prints `release golden: <n>
    passed, <m> excluded, <clang>`, and reports every failure at the end
    (decisions 25 and 35).
  - **`ci.yml`:** the `test` job's matrix names `ubuntu-24.04` with an
    `include` of `require_clang: "1"` and `clang: clang-18`; the step
    "Install clang 18.1.3 (Linux)"; the `cargo test` step's
    `NOVA_REQUIRE_CLANG` and `NOVA_CLANG`; each with a comment citing the
    spec and ADR 0048 (decision 26).
- [ ] **Step 2: Run.** First the build.
  - `R release_golden_suite --nocapture`. Expected: `release golden: not
    run (set NOVA_REQUIRE_CLANG=1 to run it)`, `1 passed; 0 failed; 0
    ignored; 0 measured; 333 filtered out` (decision 5).
  - `G` (about 17 s on this machine's 4 cores). Expected: `release golden:
    131 passed, 9 excluded, Ubuntu clang version 18.1.3 (1ubuntu1)`, then
    `1 passed; 0 failed; 0 ignored; 0 measured; 333 filtered out`: of
    `tests/runtime/`'s 160 programs, 140 have a `.stdout`; less the nine
    excluded, 131 are built and run, 5.0a's four among them, and every one
    passes.
  - `NOVA_REQUIRE_CLANG=1 NOVA_CLANG=no-such-clang cargo test --locked -p
    nova-cli --test run_tests -- release_golden_suite`. Expected: `0
    passed; 1 failed`, panicking at `run_tests.rs:1903:9` with
    "NOVA_REQUIRE_CLANG=1, but `no-such-clang --version` does not run".
  - `"$P/actionlint-dl/actionlint"`, from the root. Expected: nothing, exit
    0.
- [ ] **Step 3: The checks,** as the conventions list them.
- [ ] **Step 4: Commit**
  `ci: the golden suite through LLVM on ubuntu-24.04 with clang 18.1.3`.

**Patch `t7-code`:**

<!-- patch t7-code -->

````diff
diff --git a/.github/workflows/ci.yml b/.github/workflows/ci.yml
index 894905e4ba123c72ae5bf5c6dd8f249c10ba8212..6984c22cdd4db6d5ca7467e6476b25f18271a580 100644
--- a/.github/workflows/ci.yml
+++ b/.github/workflows/ci.yml
@@ -27,7 +27,13 @@ jobs:
     strategy:
       fail-fast: false
       matrix:
-        os: [ubuntu-latest, windows-latest, macos-latest]
+        # The Linux leg is pinned to `ubuntu-24.04`, whose `apt` has the
+        # `clang` the golden suite through LLVM pins (ADR 0048).
+        os: [ubuntu-24.04, windows-latest, macos-latest]
+        include:
+          - os: ubuntu-24.04
+            require_clang: "1"
+            clang: clang-18
     steps:
       - uses: actions/checkout@v4
       - uses: dtolnay/rust-toolchain@stable
@@ -41,11 +47,32 @@ jobs:
       - uses: actions/setup-node@v4
         with:
           node-version: 22
+      # The golden suite through LLVM (`release_golden_suite`, spec
+      # docs/superpowers/specs/2026-10-26-phase-5-0-the-language-and-std-a-compiler-needs-design.md
+      # §3.2; ADR 0048) builds every golden program with `--release` through
+      # Ubuntu's `clang-18`, pinned to 18.1.3. This step asserts the version,
+      # not the test, so a local run with another `clang` still works; a
+      # retired runner image moves the pin by a new ADR.
+      - name: Install clang 18.1.3 (Linux)
+        if: runner.os == 'Linux'
+        run: |
+          sudo apt-get update
+          sudo apt-get install -y clang-18
+          clang-18 --version
+          if ! clang-18 --version | head -n 1 | grep -q 'version 18\.1\.3'; then
+            echo "expected Ubuntu's clang 18.1.3; the pin in ADR 0048 needs to move"
+            exit 1
+          fi
       - name: cargo build
         run: cargo build --locked --workspace
+      # On Linux, `NOVA_REQUIRE_CLANG=1` makes `release_golden_suite` run, and
+      # fail if `clang-18` cannot; elsewhere it skips, and `NOVA_CLANG` is
+      # the `clang` nova uses anyway.
       - name: cargo test
         env:
           NOVA_REQUIRE_NODE: "1"
+          NOVA_REQUIRE_CLANG: ${{ matrix.require_clang || '0' }}
+          NOVA_CLANG: ${{ matrix.clang || 'clang' }}
         run: cargo test --locked --workspace --all-features --no-fail-fast
 
       # std/ and examples/ stay in nova fmt's layout (spec
diff --git a/crates/nova-cli/tests/run_tests.rs b/crates/nova-cli/tests/run_tests.rs
index b07c4eadd49d3aefe7cc589a993f4aa3fe662a3c..1f80e602f42b73db3cd8f1f03f4c46be6b12b9b7 100644
--- a/crates/nova-cli/tests/run_tests.rs
+++ b/crates/nova-cli/tests/run_tests.rs
@@ -1779,6 +1779,241 @@ fn release_builds_and_runs_when_clang_available() {
     let _ = std::fs::remove_file(&exe);
 }
 
+/// The programs of `tests/runtime/` that `release_golden_suite` leaves out,
+/// each with why: what their own tests set up, which a built binary run
+/// alone does not have (5.0 spec §3.2).
+const RELEASE_GOLDEN_EXCLUDED: &[(&str, &str)] = &[
+    (
+        "process_args",
+        "prints `args()`, whose index 0 is the file as `nova run` was given it",
+    ),
+    ("file_open_dir", "its test makes the directory it opens"),
+    (
+        "fs_permission_denied",
+        "its test makes a file it cannot read, and skips as root",
+    ),
+    ("net_interleave", "its test serves the port it connects to"),
+    ("net_lifetime", "its test serves the port it connects to"),
+    ("net_refused", "its test reserves the port it connects to"),
+    ("net_roundtrip", "its test serves the port it connects to"),
+    ("net_timeout", "its test serves the port it connects to"),
+    ("nova_test", "a `@test` file, which `nova test` runs"),
+];
+
+/// The standard input `release_golden_suite` gives a program; every other
+/// program's is closed.
+const RELEASE_GOLDEN_STDIN: &[(&str, &str)] = &[("io_streams", "hello-stdin")];
+
+/// The exit status `release_golden_suite` expects of a program, where it is
+/// not 0: a program that panics after printing its `.stdout` aborts.
+const RELEASE_GOLDEN_STATUS: &[(&str, i32)] = &[];
+
+/// How long one release binary may run.
+const RELEASE_GOLDEN_LIMIT: std::time::Duration = std::time::Duration::from_secs(60);
+
+/// What one release binary did: its standard output and its exit status,
+/// or why it has none.
+fn run_release_binary(
+    exe: &std::path::Path,
+    tmp: &std::path::Path,
+    stdin: Option<&str>,
+) -> Result<(String, Option<i32>), String> {
+    use std::io::{Read, Write};
+    use std::process::Stdio;
+    let mut child = std::process::Command::new(exe)
+        // Each program's temporary files go in a directory of its own, so
+        // none races the same fixture's `run` test.
+        .env("TMPDIR", tmp)
+        .env("TMP", tmp)
+        .env("TEMP", tmp)
+        .env_remove("NOVA_GC_STRESS")
+        .stdin(if stdin.is_some() {
+            Stdio::piped()
+        } else {
+            Stdio::null()
+        })
+        .stdout(Stdio::piped())
+        .stderr(Stdio::null())
+        .spawn()
+        .map_err(|e| format!("cannot start: {e}"))?;
+    if let Some(text) = stdin {
+        let mut pipe = child.stdin.take().expect("piped stdin");
+        let _ = pipe.write_all(text.as_bytes());
+    }
+    let mut pipe = child.stdout.take().expect("piped stdout");
+    let reader = std::thread::spawn(move || {
+        let mut out = Vec::new();
+        let _ = pipe.read_to_end(&mut out);
+        out
+    });
+    let start = std::time::Instant::now();
+    let status = loop {
+        match child.try_wait() {
+            Ok(Some(status)) => break status,
+            Ok(None) if start.elapsed() > RELEASE_GOLDEN_LIMIT => {
+                let _ = child.kill();
+                let _ = child.wait();
+                return Err(format!("timed out after {RELEASE_GOLDEN_LIMIT:?}"));
+            }
+            Ok(None) => std::thread::sleep(std::time::Duration::from_millis(20)),
+            Err(e) => return Err(format!("cannot wait: {e}")),
+        }
+    };
+    let out = reader.join().expect("the reader thread");
+    let out = String::from_utf8_lossy(&out).replace('\r', "");
+    Ok((out, status.code()))
+}
+
+/// Every golden program of `tests/runtime/` built with `--release`, through
+/// `clang`, and run (5.0 spec §3.2; ADR 0048): its standard output must be
+/// its `.stdout` file and its exit status the one the table expects.
+///
+/// It runs only under `NOVA_REQUIRE_CLANG=1`, as CI's Ubuntu leg sets it,
+/// and then fails when the `clang` that `nova` would use (`NOVA_CLANG`, or
+/// `clang`) cannot run; elsewhere it says so and passes. CI's step, not
+/// this test, asserts `clang`'s version.
+#[test]
+fn release_golden_suite() {
+    let required = std::env::var("NOVA_REQUIRE_CLANG").as_deref() == Ok("1");
+    if std::env::var("NOVA_REQUIRE_CLANG").as_deref() != Ok("1") {
+        // Under NOVA_REQUIRE_CLANG=1 the suite fails rather than skip, so a
+        // skip that slipped past the condition above cannot pass in CI.
+        assert!(
+            !required,
+            "NOVA_REQUIRE_CLANG=1, but release_golden_suite was about to skip"
+        );
+        eprintln!("release golden: not run (set NOVA_REQUIRE_CLANG=1 to run it)");
+        return;
+    }
+    let clang = std::env::var("NOVA_CLANG").unwrap_or_else(|_| "clang".to_string());
+    let version = std::process::Command::new(&clang)
+        .arg("--version")
+        .output()
+        .ok()
+        .filter(|o| o.status.success())
+        .map(|o| {
+            String::from_utf8_lossy(&o.stdout)
+                .lines()
+                .next()
+                .unwrap_or("")
+                .trim()
+                .to_string()
+        });
+    let Some(version) = version else {
+        panic!("NOVA_REQUIRE_CLANG=1, but `{clang} --version` does not run");
+    };
+
+    let runtime = repo_root().join("tests/runtime");
+    let mut programs: Vec<String> = std::fs::read_dir(&runtime)
+        .expect("tests/runtime")
+        .filter_map(|e| {
+            let path = e.expect("a directory entry").path();
+            let stem = path.file_stem()?.to_str()?.to_string();
+            let is_nova = path.extension().and_then(|x| x.to_str()) == Some("nova");
+            (is_nova && path.with_extension("stdout").exists()).then_some(stem)
+        })
+        .collect();
+    programs.sort();
+    for (name, _) in RELEASE_GOLDEN_EXCLUDED {
+        assert!(
+            programs.iter().any(|p| p == name),
+            "the exclusion table names `{name}`, which has no golden program"
+        );
+    }
+    let walk: Vec<String> = programs
+        .iter()
+        .filter(|p| !RELEASE_GOLDEN_EXCLUDED.iter().any(|(n, _)| n == p))
+        .cloned()
+        .collect();
+    // A floor, 5.0a's count: a walk that found fewer programs would pass
+    // having checked less than it should.
+    assert!(
+        walk.len() >= 131,
+        "the walk found {} programs, fewer than 5.0a's 131",
+        walk.len()
+    );
+
+    let dir = std::env::temp_dir().join("nova-release-golden");
+    std::fs::create_dir_all(&dir).expect("the golden directory");
+    let nova_exe = assert_cmd::cargo::cargo_bin("nova");
+    let threads = std::thread::available_parallelism()
+        .map(|n| n.get())
+        .unwrap_or(1)
+        .min(4);
+    let next = std::sync::atomic::AtomicUsize::new(0);
+    let failures = std::sync::Mutex::new(Vec::<String>::new());
+    std::thread::scope(|s| {
+        for _ in 0..threads {
+            s.spawn(|| loop {
+                let i = next.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
+                let Some(name) = walk.get(i) else { break };
+                let source = runtime.join(format!("{name}.nova"));
+                let exe = dir.join(format!("{name}{}", std::env::consts::EXE_SUFFIX));
+                let tmp = dir.join(format!("{name}.tmp"));
+                std::fs::create_dir_all(&tmp).expect("a temporary directory");
+                let built = std::process::Command::new(&nova_exe)
+                    .arg("build")
+                    .arg("--release")
+                    .arg(&source)
+                    .arg("-o")
+                    .arg(&exe)
+                    .env("NO_COLOR", "1")
+                    .env("RUST_BACKTRACE", "0")
+                    .output()
+                    .expect("nova runs");
+                if !built.status.success() {
+                    failures.lock().unwrap().push(format!(
+                        "{name}: `nova build --release` failed:\n{}",
+                        String::from_utf8_lossy(&built.stderr)
+                    ));
+                    continue;
+                }
+                let stdin = RELEASE_GOLDEN_STDIN
+                    .iter()
+                    .find(|(n, _)| n == name)
+                    .map(|(_, t)| *t);
+                let want_status = RELEASE_GOLDEN_STATUS
+                    .iter()
+                    .find(|(n, _)| n == name)
+                    .map_or(0, |(_, c)| *c);
+                let expected = std::fs::read_to_string(source.with_extension("stdout"))
+                    .expect("the .stdout file")
+                    .replace('\r', "");
+                let ran = run_release_binary(&exe, &tmp, stdin);
+                let _ = std::fs::remove_file(&exe);
+                match ran {
+                    Err(why) => failures.lock().unwrap().push(format!("{name}: {why}")),
+                    Ok((out, status)) => {
+                        if status != Some(want_status) {
+                            failures.lock().unwrap().push(format!(
+                                "{name}: exit status {status:?}, expected {want_status}"
+                            ));
+                        } else if out != expected {
+                            failures.lock().unwrap().push(format!(
+                                "{name}: standard output differs\n--- expected ---\n{expected}\
+                                 --- got ---\n{out}"
+                            ));
+                        }
+                    }
+                }
+            });
+        }
+    });
+    let failures = failures.into_inner().unwrap();
+    println!(
+        "release golden: {} passed, {} excluded, {version}",
+        walk.len() - failures.len(),
+        RELEASE_GOLDEN_EXCLUDED.len()
+    );
+    assert!(
+        failures.is_empty(),
+        "{} of {} release golden programs failed:\n\n{}",
+        failures.len(),
+        walk.len(),
+        failures.join("\n\n")
+    );
+}
+
 #[test]
 fn build_hello_world_standalone() {
     let out = build_and_run("examples/01-hello-world/src/main.nova", "hello");
````


### Task 8: ADR 0048 and the records

Spec §3.1, §9; decisions 2, 19 and 28.

**Files:**
- Create: `docs/adr/0048-phase-5-scope-and-backend.md`
- Modify: `CHANGELOG.md`, `docs/phase-5-plan.md`,
  `nova-spec/00-MASTER-SPEC.md`, `nova-spec/11-PARSER.md`,
  `nova-spec/12-TYPESYSTEM.md`, `nova-spec/14-CODEGEN.md`,
  `nova-spec/20-STDLIB.md`, `nova-spec/40-TOOLING.md`,
  `nova-spec/50-TESTING.md`, `nova-spec/60-EXAMPLES.md`

- [ ] **Step 1: The records.** `patch_block t8-code`:
  - **ADR 0048,** Phase 5's scope and backend: N's pipeline from source to
    `.ll`, the backlog of what Phase 5 leaves out, textual `.ll` through
    `clang` and `cc`, the `clang` pin and how it moves, and 5.0's three
    parts; its consequences and references.
  - **`CHANGELOG.md`,** Unreleased: Added, Changed and Fixed (decision 28),
    with the one way a program changes meaning under the rule.
  - **`docs/phase-5-plan.md`:** the dated 5.0 amendment, three parts in
    place of 5.0a and 5.0b.
  - **Dated notes,** each dated 2026-10-26 and citing the spec:
    `00-MASTER-SPEC.md`, two (the newline rule implements "newline
    terminates statement"; `?` exists and converts nothing);
    `11-PARSER.md` §2 (the rule, statement context, `return` and `break`,
    views) and §6 (the postfix `(`, `[` and `-`, `+=` on any place, `?`);
    `12-TYPESYSTEM.md`, three (`String + String`, `String + Char` and `+
    Int` E0010; `?` and its desugar; the four codes) and the §8 catalog's
    rows for E0098, E0099, E0102 and E0103; `14-CODEGEN.md` (the LLVM
    backend's parity tested by `release_golden_suite`); `20-STDLIB.md`, two
    (`?` and `map_err`; `String + String` and its cost); `40-TOOLING.md`
    (fmt under the rule, the chains with `-`); `50-TESTING.md` (the golden
    suite, its tables and its gate); and `60-EXAMPLES.md`, three (`?`
    where the examples deferred it).
- [ ] **Step 2: Read them against the code.** Each record's claim against
  the tree: the ADR's pin against `ci.yml`; the CHANGELOG's entries
  against Tasks 1 to 7; the catalog's four codes' titles against the spec's
  §6 and the messages in `check.rs` and the resolver; every figure against
  this plan's. Any difference goes in the ledger and to Task 9's review,
  which fixes it in its one commit, so that this task's tree stays the
  replay's. The replay's first split found one, three notes of
  `60-EXAMPLES.md` that broke the spec's path across two lines at a
  hyphen; the gap fix keeps it on one line (decision 36).
- [ ] **Step 3: Run.**
  - `git diff --check HEAD`. Expected: nothing.
  - `git diff --stat 77b6b2f -- . ':!docs/superpowers/plans'`. Expected:
    nothing: the tree is the prototype's last, but this plan, which the
    prototype did not hold. *Amended 2026-10-27 after the review:* the
    step named `f00c99b`, the prototype's third commit.
- [ ] **Step 4: The checks,** as the conventions list them.
- [ ] **Step 5: Commit**
  `docs: ADR 0048, Phase 5's scope and backend, and the notes for 5.0a`.

**Patch `t8-code`:**

<!-- patch t8-code -->

````diff
diff --git a/CHANGELOG.md b/CHANGELOG.md
index 73da4ce7f4faabc838b12a0e64596812f18acff6..6944fa75d78a492044a16381034e57af4adcc9ac 100644
--- a/CHANGELOG.md
+++ b/CHANGELOG.md
@@ -9,6 +9,81 @@ Nova uses [Semantic Versioning](https://semver.org/spec/v2.0.0.html).
 
 ## [Unreleased]
 
+Phase 5.0a, the small language and CI (spec
+`docs/superpowers/specs/2026-10-26-phase-5-0-the-language-and-std-a-compiler-needs-design.md`
+§3; ADR 0048, Phase 5's scope and backend).
+
+### Added
+
+- **`?` on `Option` and `Result`.** `e?` gives the value inside `Some` or
+  `Ok`, and otherwise returns `None`, or the same `Err`, from the
+  enclosing function: the nearest `fn` or closure, whose declared return
+  type it uses in an `async fn`, and which it infers in a closure.
+  Nothing is converted: the error types must be the same, and `map_err`
+  converts. It is E0098 on any other type, a program's own `Option`
+  included, and E0099 in a function that returns neither, or the other of
+  the two, with a note that suggests `.ok_or(err)?` or says that `main`
+  and `@test` functions return `()`. It works on every backend, WASM
+  included.
+- **`+=` and its kin on fields and elements:** `self.pos += 1`,
+  `self.counts[k] -= 1`, `grid[i][j] *= 2`, with all ten operators. The
+  receiver and the index are evaluated once, then the old value, then the
+  right-hand side; the errors are those of plain assignment (E0060) and of
+  the operator (E0010, E0013).
+- **`String + String`,** a new `String`, so `s += t` works on a local, a
+  field or an element. Each `+` copies both strings, so appending in a
+  loop is quadratic, as interpolation is. `String + Char` and `String +
+  Int` are still errors, now with a note that suggests interpolation.
+- **E0102, a warning:** a variant of a program's own sum, or of a path
+  dependency's, whose name is also a variant std brings into every
+  module, as `Number` is `JsonValue::Number`'s. In that module the bare
+  name means the program's own variant.
+- **E0103, a warning, "this value is not used":** a statement, not a
+  block's tail, that is an operator expression other than an assignment,
+  `&&` or `||`, a literal, a name, or a parenthesised, tuple or array
+  expression. **E0103 also warns at each place the newline rule ends a
+  statement the parser before it would have continued:** a line that
+  starts with `(`, `[` or `-`, and a line after a `return` or `break` at
+  the end of a line. A `;` written at the end of the line before says the
+  break is meant, and silences it.
+
+### Changed
+
+- **The newline rule.** In a block, a line break ends an expression
+  before a `(`, a `[` or a binary `-`: the token begins the next
+  statement or match arm. A `return` or `break` at the end of a line
+  takes no value, wherever it is, so `(return` / `x)` no longer parses.
+  Every other operator, and `.`, continue the line before, as before, and
+  inside parentheses, brackets, arguments, record literals and the heads
+  of `if`, `while`, `for` and `match` a `(`, `[` or `-` on a new line
+  continues as before. This implements the master spec's "newline
+  terminates statement". **One way a program changes meaning:** a line
+  that starts with `(`, `[` or `-` and meant to continue the line before
+  now begins a statement; E0103 warns at each such place. No file in the
+  repository changes, and every `.nova` file in it parses to the same
+  tree and warns nowhere.
+- **`nova fmt`** keeps a `;` the author wrote before a statement that
+  starts with `(`, `[` or `-`, and after a bare `return` or `break`, and
+  adds none where there was none, so a program's statements and warnings
+  are as they were; it no longer prints a `,` before an arm that starts
+  with `(`, `[` or `-`. A long chain with a `-` in it now breaks after
+  each operator, so no line starts with `-`; other chains still break
+  before each operator. A `view`'s children keep their `;` rule.
+- **The lexer marks each token that follows a line break,** a comment
+  holding one included, and a lone `\r` counting as one: `lex` returns
+  `Tokens`, which reads as a slice of the tokens, and `parse` takes it.
+- **CI:** the `test` job's Linux leg runs on `ubuntu-24.04`, installs
+  Ubuntu's `clang-18` and asserts that it is 18.1.3. With
+  `NOVA_REQUIRE_CLANG=1` and `NOVA_CLANG=clang-18`, a new test,
+  `release_golden_suite`, builds every golden program of `tests/runtime/`
+  with `--release`, through `clang`, runs it and compares its output;
+  elsewhere it skips (ADR 0048).
+
+### Fixed
+
+- **`return` inside a closure** checks against the closure's return
+  type, not the enclosing function's.
+
 ## [0.4.0] - 2026-10-23
 
 **Phase 4, "Frontend / WASM", is complete**, within the boundary
diff --git a/docs/adr/0048-phase-5-scope-and-backend.md b/docs/adr/0048-phase-5-scope-and-backend.md
new file mode 100644
index 0000000000000000000000000000000000000000..dc84ec395efc2d34a6bc3deecaa21e86ae28cfb6
--- /dev/null
+++ b/docs/adr/0048-phase-5-scope-and-backend.md
@@ -0,0 +1,94 @@
+# ADR 0048 — Phase 5's scope and backend
+
+## Status
+
+Accepted (2026-10-26). Branch `claude/loving-archimedes-frtf5q-50`
+(`docs/superpowers/specs/2026-10-26-phase-5-0-the-language-and-std-a-compiler-needs-design.md`
+§3.1). The scope, the backend, the `clang` pin and the three parts of 5.0
+are the user's decisions of 2026-10-26 (`docs/phase-5-plan.md` §1, and §3
+decisions 1 and 2; the 5.0 spec's decisions 1 and 7).
+
+## Context
+
+`nova-spec/00-MASTER-SPEC.md` §3 makes Phase 5 the self-hosting phase: the
+Nova compiler written in Nova. `docs/phase-5-plan.md` names the stages:
+S0 is the Rust `nova`, N is the Nova compiler's source in `selfhost/`,
+S1 = S0(N), S2 = S1(N) and S3 = S2(N). Its gate (5.6) is S1 == S2 byte
+for byte, and S2 == S3, on Linux, with one pinned `clang`.
+
+The master spec has N "bind LLVM via `unsafe` extern". Today's Nova has
+no pointer FFI and no `unsafe`, and S0's own `--release` already writes
+textual LLVM IR and runs `clang`, as `14-CODEGEN.md`'s note of 2026-10-06
+recorded when it departed from `inkwell`.
+
+Until 5.0, nothing in CI pinned a `clang` or ran the golden suite through
+the LLVM backend: one test built hello world with `--release` where a
+`clang` happened to be installed, and the browser job built the blog's
+server with the runner's.
+
+## Decision
+
+1. **Scope.** N covers the pipeline from source to `.ll`: the lexer, the
+   parser, the resolver, HIR, the type checker, MIR and the textual LLVM
+   IR emitter, with the driver and the commands `build` and `check`.
+   Diagnostics are compared with S0's by code and span, not by rendered
+   text.
+2. **What Phase 5 leaves out,** each to the backlog unless a sub-phase's
+   spec pulls it back with a ruling that names this ADR:
+   - pointer FFI, `unsafe` blocks, linking C libraries, and a binding of
+     LLVM's C API;
+   - porting `run`, `test`, fmt, the language server, doc, the package
+     manager, and the Cranelift and WASM backends;
+   - S1 as the shipped `nova`: `v0.5.0` ships the Rust `nova`;
+   - the byte gate on macOS and Windows;
+   - N built for WASM, which Phase 6's playground needs;
+   - trait objects, mutable globals and nested module directories.
+3. **The backend.** N writes textual `.ll` and runs `clang` and `cc`, as
+   S0's `--release` does, through 5.0c's `std/process`. This departs from
+   the master spec's "bind LLVM via `unsafe` extern", as `14-CODEGEN.md`'s
+   note of 2026-10-06 departed from `inkwell`. Emitting S0's own text is
+   what makes the gate testable.
+4. **The `clang` CI pins.** Ubuntu's `clang-18`, 18.1.3, installed with
+   `apt-get` on the `ubuntu-24.04` runner, which the `test` job's Linux
+   leg now names in place of `ubuntu-latest`. A CI step asserts the
+   version, not the test, so a local run with another `clang` still
+   works. Under `NOVA_REQUIRE_CLANG=1`, which that leg sets with
+   `NOVA_CLANG=clang-18`, `release_golden_suite` builds every golden
+   program of `tests/runtime/` but the nine its exclusion table names,
+   with `--release`, and runs it, and fails if that `clang` cannot run, or
+   if it would skip. Elsewhere it says it did not run and passes.
+   When the runner image is retired, or `apt` ships another build, the
+   version step fails and a new ADR moves the pin.
+5. **5.0's three parts,** which amend the phase plan's 5.0a and 5.0b
+   (the 5.0 spec's decision 1):
+   - **5.0a, the small language and CI:** this ADR, the golden suite
+     through LLVM in CI, `?`, `+=` on fields and elements, the newline
+     rule, `String + String`, and the warnings E0102 and E0103;
+   - **5.0b, tuples and patterns,** with ADR 0049;
+   - **5.0c, std,** with ADR 0050, including `std/process`.
+
+## Consequences
+
+- **The master spec's Phase 5 list is not amended;** this ADR records the
+  difference, as ADR 0038 did for Phase 4.
+- **N needs no new runtime dependency:** `clang` is a CI tool, and
+  `cargo test` never needs it.
+- **Every golden program but the nine excluded runs through the LLVM
+  backend in CI** from 5.0a on, so each program 5.0b and 5.0c add runs
+  there too, unless the table excludes it with its reason. GC stress stays
+  out of that walk: two programs time out under it on both backends.
+- **The pin is a maintenance cost:** a retired image fails the version
+  step loudly, and moving the pin takes an ADR.
+- **The backlog grows** by item 2's list, which Phase 5's closing ADR
+  keeps.
+
+## References
+
+- `nova-spec/00-MASTER-SPEC.md` §3, Phase 5; `nova-spec/14-CODEGEN.md`'s
+  note of 2026-10-06.
+- `docs/phase-5-plan.md` §1, and §3 decisions 1 and 2.
+- `docs/superpowers/specs/2026-10-26-phase-5-0-the-language-and-std-a-compiler-needs-design.md`
+  §3.1, §3.2 and §12.
+- `.github/workflows/ci.yml`, the `test` job;
+  `crates/nova-cli/tests/run_tests.rs`, `release_golden_suite`.
+- ADR 0038, Phase 4's scope; ADR 0047, Phase 4's boundary.
diff --git a/docs/phase-5-plan.md b/docs/phase-5-plan.md
index 6798dedf546fd8ae52e24cce90f8278e3e0b3e89..9081bebd0f6c749c56b3d171e07eed384c6c7b6b 100644
--- a/docs/phase-5-plan.md
+++ b/docs/phase-5-plan.md
@@ -305,6 +305,25 @@ From 5.1 on, each gate has two parts: N's dump equals S0's on the corpus
 ### 5.0 — The language and std that a compiler needs
 May split into 5.0a and 5.0b, as its spec decides.
 
+**Amended 2026-10-26 (branch `claude/loving-archimedes-frtf5q-50`):** 5.0
+is built as **three parts, in order,** by the user's decision of
+2026-10-26
+(`docs/superpowers/specs/2026-10-26-phase-5-0-the-language-and-std-a-compiler-needs-design.md`
+decision 1; ADR 0048), in place of the 5.0a and 5.0b below:
+- **5.0a, the small language and CI:** ADR 0048; the golden suite
+  through the LLVM backend in CI, on a pinned `clang` (from 5.0b's list);
+  `?`; `+=` on fields and elements; the newline rule, a fix in the
+  parser; `String + String` (from the builder's question); and the
+  warnings E0102, for a variant that hides one of std's, and E0103, for
+  a statement whose value is unused;
+- **5.0b, tuples and patterns** (from 5.0a's list): tuples on every
+  backend, WASM included, and nested patterns with guards, or-patterns,
+  and `Char` and range patterns, past the list on the user's word;
+- **5.0c, std** (5.0b's list but CI): the builder, conversions,
+  `parse_int` and `parse_float`, array helpers, `OrderedMap`, and
+  `std/process` with `run`, `status`, `env` and `current_exe`; and
+  `Float.to_bits`.
+
 - **ADR 0048**, the scope and the backend (decisions 1 and 2).
 - **5.0a, the language,** in S0's lexer, parser, type checker, MIR and both
   native backends, with `nova fmt` and the language server following:
diff --git a/nova-spec/00-MASTER-SPEC.md b/nova-spec/00-MASTER-SPEC.md
index 19fdf2d0f6ec200b3155f5faca4606ca9ced65c1..ff068c299b05b376c3e20620003f4b5b95b729ed 100644
--- a/nova-spec/00-MASTER-SPEC.md
+++ b/nova-spec/00-MASTER-SPEC.md
@@ -64,6 +64,14 @@ These are NOT up for debate. If a tradeoff appears mid-implementation, prefer th
 - **TypeScript / Swift inspired** — curly braces, expression-oriented
 - **Significant whitespace: NO** (curly braces win every time)
 - **Semicolons: optional** (newline terminates statement; semicolons allowed for one-liners)
+  **Amended 2026-10-26 (branch `claude/loving-archimedes-frtf5q-50`):** the
+  parser now implements "newline terminates statement" by the newline rule
+  of the 5.0 spec
+  (`docs/superpowers/specs/2026-10-26-phase-5-0-the-language-and-std-a-compiler-needs-design.md`
+  §3.5): in a block, a line break ends an expression before a `(`, a `[`
+  or a binary `-`, and a `return` or `break` at the end of a line takes no
+  value; every other operator, and `.`, continue the line before.
+  `11-PARSER.md` §2 and §6 have the details.
 - **String interpolation:** `"Hello, ${name}"` (TS-style)
 - **Comments:** `//` line, `/* */` block, `///` doc
 
@@ -359,6 +367,12 @@ language feature: the router's `Handler` type alias still does not parse,
 `Map` still has `keys()` and no `values()`. The example routes around each of
 those rather than removing any of them.
 
+**Amended 2026-10-26 (branch `claude/loving-archimedes-frtf5q-50`):** the
+`?` operator exists, on `Option` and `Result`, in 5.0a
+(`docs/superpowers/specs/2026-10-26-phase-5-0-the-language-and-std-a-compiler-needs-design.md`
+§3.3). It converts nothing: the error types must be the same, and `map_err`
+converts. `@derive` and turbofish are still absent.
+
 **AMENDED 2026-09-11: the gate's two criteria, recorded above as able to
 disagree in either direction, now both have a measured figure — and they
 agree.** The absolute figure, above, is 1875.2 to 3108.5 req/sec at ten
diff --git a/nova-spec/11-PARSER.md b/nova-spec/11-PARSER.md
index 1bb95ed76c47fd28712edef2c0de4c7dbe9f063e..f38a2fbf9a9715b91e1b3f9f031a394c321628cb 100644
--- a/nova-spec/11-PARSER.md
+++ b/nova-spec/11-PARSER.md
@@ -244,6 +244,35 @@ view_arg     = ( ident | keyword ) ":" expr | expr ;
 - A `;` may end a view's child, so the next can begin with `(`, `[` or
   `{`.
 
+**Amended 2026-10-26 (branch `claude/loving-archimedes-frtf5q-50`):** **the
+newline rule**
+(`docs/superpowers/specs/2026-10-26-phase-5-0-the-language-and-std-a-compiler-needs-design.md`
+§3.5; ADR 0043, whose lexer skips line breaks, and ADR 0028 §5, whose `;`
+fmt printed around it). The grammar above writes `;` after each statement;
+the parser has never needed one, and now implements the master spec's
+"newline terminates statement" as follows. The lexer marks each token that
+follows a line break, a comment holding one included. **In statement
+context, a line break ends the expression before a `(` (a call), a `[` (an
+index) or a binary `-`;** the token begins the next statement or match arm.
+Every other operator, and `.`, continue the expression, as before.
+
+**Statement context** is the expression of a statement, a block's tail, a
+`let`'s value, a `return`'s or `break`'s value, and a match arm's body. It
+is off inside `(` and `)`, `[` and `]`, argument lists, record literals'
+braces, interpolation holes, a view's children and an element's arguments, a
+guard, and the heads of `if`, `while`, `for` and `match`, which end at their
+`{`; a block anywhere, and a match's arms, turn it back on. A `{` on a new
+line keeps its meaning. **A `return` or `break` at the end of a line takes
+no value,** wherever it is.
+
+So `let z = x`, then `-y` on the next line, is two statements;
+`if c { 1 } else { 2 }`, then `(a, b)`, is not a call of the `if`. The
+parser records each such split, where the parser before the rule would
+have continued, and the checker warns at each (E0103), but at the next arm
+after an arm's body; a `;` written at the end of the line before says the
+break is meant, so there is no split to record. A view's children keep
+their own rule: a `;` ends a child before a `(` or `[`.
+
 ---
 
 ## 3. AST Node Definitions (Rust)
@@ -410,6 +439,16 @@ Every error → `ParseError` with span + expected/found info → diagnostics cra
 
 Comparison operators (==, !=, <, etc.) are non-chainable: `a < b < c` is an error with helpful suggestion.
 
+**Amended 2026-10-26 (branch `claude/loving-archimedes-frtf5q-50`):** a
+postfix `(` or `[`, and a binary `-`, after a line break in statement
+context begin the next statement (§2's newline rule). Compound assignment
+works on any place plain assignment accepts, a field (`self.f += x`) and an
+element (`a[i] -= 1`) included, as this table always allowed: the receiver
+and the index are evaluated once
+(`docs/superpowers/specs/2026-10-26-phase-5-0-the-language-and-std-a-compiler-needs-design.md`
+§3.4). The postfix `?` works on `Option` and `Result` (`12-TYPESYSTEM.md`
+§7).
+
 ---
 
 ## 7. Tests Required
diff --git a/nova-spec/12-TYPESYSTEM.md b/nova-spec/12-TYPESYSTEM.md
index 7b4fb8164d050e98e9a25fc1ff914db270e8a0f1..d4d3f9cdf0f8af9c83347e0cb0464eba4375c41b 100644
--- a/nova-spec/12-TYPESYSTEM.md
+++ b/nova-spec/12-TYPESYSTEM.md
@@ -198,6 +198,28 @@ Future      — async result
 
 `Copy` and `Clone` are auto-derivable: `@derive(Copy, Clone)` attribute.
 
+**Amended 2026-10-26 (branch `claude/loving-archimedes-frtf5q-50`):** the
+operator traits above are not std's yet: `binary_result_ty` decides each
+operator by its operands' types. 5.0a adds `String + String`, a new `String`
+holding `a` then `b`, which MIR lowers to the runtime's concatenation; `+=`
+follows on a local, a field or an element. `String + Char` and `String +
+Int` stay E0010, with a note that suggests interpolation
+(`docs/superpowers/specs/2026-10-26-phase-5-0-the-language-and-std-a-compiler-needs-design.md`
+§3.6).
+
+**Amended 2026-10-26 (branch `claude/loving-archimedes-frtf5q-50`):** `e?`
+works on std/core's `Option` and `Result`, found by their `DefId`s, not by
+the names in scope
+(`docs/superpowers/specs/2026-10-26-phase-5-0-the-language-and-std-a-compiler-needs-design.md`
+§3.3). On `Option<T>`, `Some(v)` gives `v` and `None` returns `None`; on
+`Result<T, E>`, `Ok(v)` gives `v` and `Err(x)` returns `Err(x)`, re-wrapped.
+**Nothing is converted:** the enclosing function, the nearest `fn` or
+closure, must return the same sum with the same `E`, and `map_err` converts.
+There is no `From` trait, since generic traits do not exist. The checker
+desugars `e?` to a flat `match`, so no backend changes. Its errors are
+E0098, E0099, and E0010 with a note pointing at `map_err` when the error
+types differ.
+
 ---
 
 ## 8. Error Catalog (sample — full list grows)
@@ -221,9 +243,32 @@ Future      — async result
 | E0031 | Cycle in trait inheritance |
 | E0040 | Cannot return reference to local |
 | E0050 | Async/await mismatch |
+| E0098 | `?` needs an `Option` or a `Result` (added 2026-10-26, Phase 5.0a) |
+| E0099 | `?` cannot return from this function (added 2026-10-26, Phase 5.0a) |
+| E0102 | A variant hides one of std's; a warning (added 2026-10-26, Phase 5.0a) |
+| E0103 | This value is not used, or a line break ended a statement; a warning (added 2026-10-26, Phase 5.0a) |
 
 Each gets a fully detailed error in `nova-diagnostics::messages` with example, suggestion, and link.
 
+**Amended 2026-10-26 (branch `claude/loving-archimedes-frtf5q-50`):** the
+four codes 5.0a adds
+(`docs/superpowers/specs/2026-10-26-phase-5-0-the-language-and-std-a-compiler-needs-design.md`
+§6). E0098 is `?` on any other type, a program's own sum named `Option`
+included; a note names the type. E0099 is `?` in a function whose return
+type is neither sum, or the other of the two; a note suggests `.ok_or(err)?`
+for an `Option` in a function that returns a `Result`, and another says that
+`main` and `@test` functions return `()`, another that the function's
+`Option` or `Result` is the program's own; `?` in a constant's initializer
+is E0099 too, since a constant is no function. E0102 warns at a variant of the
+package's own sum, or a path dependency's, whose name is also a variant std
+brings into every module; std's own modules are never warned about. E0103
+warns at a statement, not a block's tail, whose value can only be discarded:
+a unary or binary operator other than an assignment, `&&` or `||`, a
+literal, a name, or a parenthesised, tuple or array expression; and at each
+place the newline rule (`11-PARSER.md` §2) ends a statement the parser
+before it would have continued, unless a `;` is written there. E0100 and
+E0101 are 5.0b's.
+
 ---
 
 ## 9. Examples — Test Cases
diff --git a/nova-spec/14-CODEGEN.md b/nova-spec/14-CODEGEN.md
index 4b9e8a4aeb47d70c49f03b641e493c82d405f750..52d4203347fb60730f620eab2fec911745e90162 100644
--- a/nova-spec/14-CODEGEN.md
+++ b/nova-spec/14-CODEGEN.md
@@ -49,6 +49,16 @@ world is built and run in release, by a test that passes without running
 anything when `clang` is not on `PATH`. Parity with the Cranelift backend is
 deferred: `docs/adr/0025-phase-2-boundary.md`.
 
+**Amended 2026-10-26 (branch `claude/loving-archimedes-frtf5q-50`):** the
+LLVM backend's parity is now tested: every golden program of
+`tests/runtime/` but the nine an exclusion table names is built with
+`--release`, through Ubuntu's `clang-18`
+18.1.3, pinned on CI's `ubuntu-24.04` leg, run, and compared with its
+`.stdout`, in `release_golden_suite`
+(`docs/superpowers/specs/2026-10-26-phase-5-0-the-language-and-std-a-compiler-needs-design.md`
+§3.2; ADR 0048, which also makes textual `.ll` through `clang` the backend
+of Phase 5's compiler). The hello-world test stays.
+
 ### 2.3 WASM (browser target)
 - Crates: `wasm-encoder` (write modules), `walrus` (manipulate)
 - Target: `nova build --target wasm` and `nova bundle`
diff --git a/nova-spec/20-STDLIB.md b/nova-spec/20-STDLIB.md
index b43dbb59ce4583e10b0fe94a6e55da27f5a70fa2..79d5902cfb676beaba5e13b1d287cc84064f34a2 100644
--- a/nova-spec/20-STDLIB.md
+++ b/nova-spec/20-STDLIB.md
@@ -1308,6 +1308,14 @@ above. Shipped call style is unqualified — `stringify(v)`,
 `Bool::from_json(v)` — since every `STD_MODULES` entry is glob-imported into
 every module.
 
+**Amended 2026-10-26 (branch `claude/loving-archimedes-frtf5q-50`):** the
+`?` operator exists, in 5.0a
+(`docs/superpowers/specs/2026-10-26-phase-5-0-the-language-and-std-a-compiler-needs-design.md`
+§3.3). It converts nothing, so a `?` on a `Result<T, JsonError>` needs a
+function whose error type is `JsonError`, or a `map_err` first. `@derive`
+and the `module` line still do not parse, so the example block still does
+not compile as written.
+
 **The amendment-marker set in this file, regrepped rather than copied
 forward.** Before this increment there were **16**; this increment adds
 **two** — this one, and §12's, recording `Map::keys` — for **18**. That is
@@ -2780,6 +2788,15 @@ shipped in Phase 2.2b
 section here. Its whole surface is one `impl String` block in
 `std/strings/lib.nova`, glob-imported into every module.
 
+**Amended 2026-10-26 (branch `claude/loving-archimedes-frtf5q-50`):**
+`String + String` is a new `String`, `a` then `b`, and `s += t` follows, on
+a local, a field or an element
+(`docs/superpowers/specs/2026-10-26-phase-5-0-the-language-and-std-a-compiler-needs-design.md`
+§3.6). It is a call of the runtime's concatenation, which copies both, so
+appending in a loop is quadratic, as interpolation is; 5.0c's
+`StringBuilder` is the linear way. `String + Char` and `String + Int` stay
+errors, whose note suggests interpolation.
+
 ```nova
 impl String {
     // Whether this is "". Cheaper than `len() == 0`, which decodes the
diff --git a/nova-spec/40-TOOLING.md b/nova-spec/40-TOOLING.md
index 3cfd641337db1cd65e0a4ba9907e4ee8c73fbb65..9c2e295cb462ce6f6c69f63e9dbecb81e20481b6 100644
--- a/nova-spec/40-TOOLING.md
+++ b/nova-spec/40-TOOLING.md
@@ -60,6 +60,23 @@ file that would change, or 2 on any error. ADR 0028 reads the master spec's
 "only `--check`" as "no style options"
 (`docs/superpowers/specs/2026-10-07-phase-3-1-formatter-design.md` §7).
 
+**Amended 2026-10-26 (branch `claude/loving-archimedes-frtf5q-50`):** with
+the newline rule (`11-PARSER.md` §2;
+`docs/superpowers/specs/2026-10-26-phase-5-0-the-language-and-std-a-compiler-needs-design.md`
+§3.5), `nova fmt` prints the `;` before a statement that starts with `(`,
+`[` or `-`, and after a bare `return` or `break`, only where the author
+wrote one, since there the newline rule ends the statement and E0103 warns
+unless a `;` is written; it no longer prints the `,` before a match arm
+that starts with one of them. ADR 0028 §5 had it print all of them because
+the parser read across line breaks. It still prints the `;` before `{`,
+`*`, `&`, `|` and `||`, and in a view's children, which keep their rule. **A long
+binary chain with a `-` in it breaks after each operator,** so that no line
+starts with `-`, which in statement context would begin a new statement; a
+chain with no `-` still breaks before each operator. The rule is the same in
+every context, so the printer need not track statement context, and it adds
+no token, as parentheses around the chain would. Every file of the corpus
+reformats to the same tree.
+
 **Amended 2026-10-09 (branch `phase-3-3b-index-publishing`):** `nova
 add <pkg>[@<req>]`, `nova update [pkg]`, `nova publish` and `nova login`
 exist, with two commands this list lacks. `nova fetch` downloads what
diff --git a/nova-spec/50-TESTING.md b/nova-spec/50-TESTING.md
index 5aebc812195279ef874d91bf101448723d8d2d20..7971ea4788d0eb51059e8fd923c6212a468377d8 100644
--- a/nova-spec/50-TESTING.md
+++ b/nova-spec/50-TESTING.md
@@ -38,6 +38,22 @@ Location: `tests/runtime/`
 - Adjacent `.stdout` (or `.stdout.json`) is expected output
 - Covers behavior, not just types
 
+**Amended 2026-10-26 (branch `claude/loving-archimedes-frtf5q-50`):** every
+program of `tests/runtime/` with a `.stdout` also runs through the LLVM
+backend, in `release_golden_suite` (`crates/nova-cli/tests/run_tests.rs`;
+`docs/superpowers/specs/2026-10-26-phase-5-0-the-language-and-std-a-compiler-needs-design.md`
+§3.2; ADR 0048). It builds each with `nova build --release` into
+`nova-release-golden/` in the temporary directory, on up to four threads,
+runs each with its standard input closed (or given, by its table) and a time
+limit of 60 s, and compares its standard output, without `\r`, with the
+`.stdout` file and its exit status with its table's. An exclusion table
+names each program left out and why: those whose tests set up arguments, a
+directory, a file or a port. It runs only under `NOVA_REQUIRE_CLANG=1`,
+which CI's Linux leg sets with `NOVA_CLANG=clang-18`, and then fails if that
+`clang` cannot run, if it would skip, or if its walk finds fewer than 131
+programs; a step before it asserts that Ubuntu's `clang-18` is
+18.1.3. It prints `release golden: <n> passed, <m> excluded, <clang>`.
+
 ### 1.4 Stdlib tests
 Location: per-module: `std/<module>/tests/`
 - Written in Nova
diff --git a/nova-spec/60-EXAMPLES.md b/nova-spec/60-EXAMPLES.md
index 98d7fc9ddf4c00161f39c613922cc5b9ee2a62cb..cdb4ce15a3198650b69580d6bdcb0315aab8a73b 100644
--- a/nova-spec/60-EXAMPLES.md
+++ b/nova-spec/60-EXAMPLES.md
@@ -247,6 +247,13 @@ gap — see `docs/benchmarks/README.md` for the procedure used instead and
 produced. The listing below stays as written, as the aspiration it always
 was.
 
+**Amended 2026-10-26 (branch `claude/loving-archimedes-frtf5q-50`):** the `?`
+operator exists, on `Option` and `Result`, in 5.0a
+(`docs/superpowers/specs/2026-10-26-phase-5-0-the-language-and-std-a-compiler-needs-design.md`
+§3.3); it converts nothing, so the error types must be the same, and
+`map_err` converts. Turbofish, `@derive` and the `Handler` alias are still
+absent.
+
 **AMENDED 2026-09-10 (branch `examples-05-json-api`): the example now exists,
 and the gate this section names is still NOT met.** `examples/05-json-api/`
 holds `src/main.nova`, a `README.md` and a `BENCHMARK.md`. It serves the same
@@ -348,6 +355,13 @@ sites. That is the population searched — tracked files, both spellings
 (hyphenated and not), flattened first because this repo's prose wraps — and not
 a claim that no other record says it.
 
+**Amended 2026-10-26 (branch `claude/loving-archimedes-frtf5q-50`):** the `?`
+operator exists, on `Option` and `Result`, in 5.0a
+(`docs/superpowers/specs/2026-10-26-phase-5-0-the-language-and-std-a-compiler-needs-design.md`
+§3.3); it converts nothing, so the error types must be the same, and
+`map_err` converts. Turbofish, `@derive` and the `Handler` alias are still
+absent.
+
 **What the amendment above still has right, and this increment left open.**
 `@derive` is not implemented — an unknown attribute is `E0082`, and the message
 lists what the resolver's `KNOWN_ATTRIBUTES` holds — `test` alone at this
@@ -358,6 +372,13 @@ ids ascending from 1 instead of iterating values. The `Handler` type alias
 still does not parse, so there is nothing for `Server.get`/`.post` to be built
 on. There is no `?` operator. None of those moved here.
 
+**Amended 2026-10-26 (branch `claude/loving-archimedes-frtf5q-50`):** the `?`
+operator exists, on `Option` and `Result`, in 5.0a
+(`docs/superpowers/specs/2026-10-26-phase-5-0-the-language-and-std-a-compiler-needs-design.md`
+§3.3); it converts nothing, so the error types must be the same, and
+`map_err` converts. Turbofish, `@derive` and the `Handler` alias are still
+absent.
+
 **Recorded 2026-10-04 (branch `examples-03-04-inventory`): the `Handler` alias
 is not what stands between `std/http` and a `Server.get`.** It still does not
 parse. But a router over `fn(Request) -> Response` fields, written in today's
````


### Task 9: Verification, the mutants, the review, the PR, and the merge

- [ ] **Step 1: The full checks.** First, `df -h /` must show at least 8G
  free; if not, remove `target/debug/incremental` and older duplicate test
  executables.

  ```sh
  cargo fmt --all -- --check
  cargo clippy --locked --all-targets --all-features -- -D warnings
  cargo run --locked -q -p nova-cli -- fmt --check std examples tests/wasm tests/dom tests/browser/router-app tests/browser/ssr-app tests/ssr
  cargo build --locked --workspace
  NOVA_REQUIRE_NODE=1 cargo test --locked --workspace --no-fail-fast > "$P/full-50a.txt" 2>&1
  grep '^test result' "$P/full-50a.txt" | awk '{p += $4; f += $6; i += $8} END {print p, f, i}'
  grep ' FAILED$' "$P/full-50a.txt"
  cargo test --locked --workspace -- --list > "$P/final-list-50a.txt" 2>&1
  ```

  Expected:
  - both formatters print nothing, and clippy ends with `Finished`, with
    no warning or error;
  - the suite, in 234 s: `2529 1 10`, the baseline's figures and
    109 more tests;
  - the one failure is Task 0's IPv6 test;
  - the two lists, compared binary by binary by name (a binary is its
    `Running` or `Doc-tests` line, hash stripped; a test a line ending
    `: test`), give 2431 tests in 102 binaries before and 2540 in 103
    after, 115 added and 6 gone: `newline_rule` added, with
    its 38 tests, none removed, and in
    the binaries that change these tests added, four renamed and two
    doc tests moved to another line, the old names marked "gone":

    - `tests/run_tests.rs (run_tests)`, 328 to 334:
      `a_variant_that_hides_one_of_stds_warns_and_still_builds`;
      `compound_assign_places_runs`;
      `release_golden_suite`;
      `string_concat_runs`;
      `try_async_runs`;
      `try_operator_runs`;
    - `tests/wasm_tests.rs (wasm_tests)`, 230 to 234:
      `gate::compound_assign_places`;
      `gate::string_concat`;
      `gate::try_async`;
      `gate::try_operator`;
    - `tests/ir_tests.rs (ir_tests)`, 16 to 17:
      `string_plus_string_lowers_to_one_concat_call`;
    - `unittests src/lib.rs (nova_fmt)`, 27 to 28:
      `check::tests::the_fingerprint_tells_splits_by_kind_not_by_place`;
    - `tests/source_rules.rs (source_rules)`, 21 to 27:
      `a_broken_chain_with_a_minus_ends_each_line_with_its_operator`;
      `a_broken_chain_without_a_minus_breaks_before_each_operator`;
      `a_flat_chain_with_a_minus_is_unchanged`;
      `a_match_arm_starting_with_a_parenthesis_needs_no_comma`;
      `a_statement_starting_with_a_bracket_or_a_minus_keeps_the_authors_semicolon`;
      `a_statement_starting_with_a_parenthesis_keeps_the_authors_semicolon`;
      `a_value_less_return_keeps_the_authors_semicolon`;
      `a_view_child_starting_with_a_parenthesis_keeps_its_semicolon`;
      `an_unformatted_file_with_a_split_formats_keeping_it`;
      gone: `a_match_arm_starting_with_a_parenthesis_keeps_its_comma`;
      gone: `a_statement_starting_with_a_parenthesis_keeps_its_semicolon`;
      gone: `a_value_less_return_keeps_its_semicolon`;
    - `tests/lexer_tests.rs (lexer_tests)`, 35 to 42:
      `a_block_comment_marks_the_token_after_it_only_when_it_holds_a_line_break`;
      `a_line_break_inside_a_string_marks_nothing`;
      `a_line_comment_marks_the_token_after_it`;
      `a_lone_carriage_return_is_a_line_break`;
      `a_token_after_a_line_break_is_marked`;
      `the_first_token_is_marked_only_after_a_line_break`;
      `the_marks_leave_the_tokens_as_they_were`;
    - `unittests src/lib.rs (nova_resolver)`, 54 to 60:
      `tests::a_name_that_is_not_one_of_stds_variants_is_not_reported`;
      `tests::a_qualified_use_of_stds_variant_is_fine`;
      `tests::a_variant_named_as_one_of_stds_is_e0102`;
      `tests::another_modules_variant_is_reported_too`;
      `tests::each_hidden_variant_is_reported`;
      `tests::std_itself_is_never_reported`;
    - `unittests src/lib.rs (nova_typeck)`, 410 to 450:
      `check::lang_tests::a_split_after_a_bare_return_warns`;
      `check::lang_tests::a_split_after_a_return_value_warns`;
      `check::lang_tests::a_split_before_a_tail_warns`;
      `check::lang_tests::a_written_semicolon_silences_the_split`;
      `check::lang_tests::an_unused_value_at_a_split_gets_one_warning_with_the_line_breaks_note`;
      `check::lang_tests::compound_assignment_evaluates_the_index_once`;
      `check::lang_tests::compound_assignment_on_a_field_evaluates_the_receiver_once`;
      `check::lang_tests::compound_assignment_on_an_immutable_root_is_e0060`;
      `check::lang_tests::compound_assignment_on_nested_places_checks`;
      `check::lang_tests::compound_assignment_with_a_bad_operand_type_is_e0013_or_e0010`;
      `check::lang_tests::e0103_flags_each_form_whose_value_is_discarded`;
      `check::lang_tests::e0103_flags_what_the_newline_rule_leaves`;
      `check::lang_tests::e0103_is_not_on_calls_assignments_conditions_or_a_tail`;
      `check::lang_tests::e0103_reports_a_parenthesised_callee_after_a_line_break`;
      `check::lang_tests::every_compound_operator_on_a_field_and_an_element_checks`;
      `check::lang_tests::map_err_then_try_checks`;
      `check::lang_tests::return_in_a_closure_checks_against_the_closures_type`;
      `check::lang_tests::string_compound_assignment_on_a_field_checks`;
      `check::lang_tests::string_minus_string_is_still_e0013`;
      `check::lang_tests::string_plus_char_or_int_is_refused_with_a_note`;
      `check::lang_tests::string_plus_string_is_a_string`;
      `check::lang_tests::try_desugars_to_a_match_in_hir`;
      `check::lang_tests::try_gives_the_payload_type`;
      `check::lang_tests::try_in_a_closure_inside_a_constant_checks`;
      `check::lang_tests::try_in_a_closure_inside_main_has_no_entry_note`;
      `check::lang_tests::try_in_a_closure_returns_from_the_closure_not_the_function`;
      `check::lang_tests::try_in_a_constant_is_e0099`;
      `check::lang_tests::try_in_a_function_returning_its_own_option_says_so`;
      `check::lang_tests::try_in_a_function_that_returns_neither_is_e0099`;
      `check::lang_tests::try_in_a_test_function_notes_that_it_returns_unit`;
      `check::lang_tests::try_in_an_async_fn_uses_its_declared_return_type`;
      `check::lang_tests::try_in_main_notes_that_main_returns_unit`;
      `check::lang_tests::try_infers_a_closures_return_type`;
      `check::lang_tests::try_on_another_type_is_e0098_naming_it`;
      `check::lang_tests::try_on_option_in_a_result_function_suggests_ok_or`;
      `check::lang_tests::try_on_option_in_an_option_function_checks`;
      `check::lang_tests::try_on_result_in_an_option_function_is_e0099_without_ok_or`;
      `check::lang_tests::try_on_result_rewraps_a_different_ok_type`;
      `check::lang_tests::try_on_the_programs_own_option_is_e0098`;
      `check::lang_tests::try_with_different_error_types_is_e0010_pointing_at_map_err`;
      `check::tests::compound_assignment_to_field_checks`;
      gone: `check::tests::compound_assignment_to_field_reports_e0900`;
    - `doc-tests nova_lexer`, 1 to 1:
      `crates/nova-lexer/src/lib.rs - (line 16)`;
      gone: `crates/nova-lexer/src/lib.rs - (line 15)`;
    - `doc-tests nova_parser`, 1 to 1:
      `crates/nova-parser/src/lib.rs - (line 12)`;
      gone: `crates/nova-parser/src/lib.rs - (line 9)`.
- [ ] **Step 2: CI's Linux leg, as CI runs it.** The `test` job's
  `ubuntu-24.04` leg runs the whole suite with the golden suite required,
  with CI's own flags, `--locked --workspace --all-features
  --no-fail-fast`:

  ```sh
  NOVA_REQUIRE_CLANG=1 NOVA_CLANG=clang-18 NOVA_REQUIRE_NODE=1 cargo test --locked --workspace --all-features --no-fail-fast > "$P/ci-50a.txt" 2>&1
  grep '^test result' "$P/ci-50a.txt" | awk '{p += $4; f += $6; i += $8} END {print p, f, i}'
  grep ' FAILED$' "$P/ci-50a.txt"
  grep -c 'release_golden_suite ... ok' "$P/ci-50a.txt"
  ```

  Expected, in 226 s: `2529 1 10`, Step 1's figures, the one
  failure the IPv6 test, and `1`: the golden suite ran and passed inside
  the whole run, as on CI, where Step 1's run skips it.
- [ ] **Step 3: The golden suite.** `time G`. Expected, in
  17.4 seconds of test time, 20.1 s wall under `time`:
  `release golden: 131 passed, 9 excluded, Ubuntu clang version 18.1.3 (1ubuntu1)`
  and `1 passed`.
- [ ] **Step 4: The newline rule over the repository.** For every `.nova`
  file `git ls-files` lists, `"$P/nova-base-50a" parse --ast <file>` (Task
  0's copy) and `target/debug/nova parse --ast <file>`, each with its exit
  status appended, into two directories under `$P`; then `cmp` each pair,
  and remove both directories (about 130 MB each). Expected:
  `ast: 279 files, 0 differ`
  (decision 11).
- [ ] **Step 5: Where E0102 and E0103 fire.** `target/debug/nova check` on
  every `.nova` file `git ls-files` lists that is not under a package, and
  `nova check` from each package's root once, counting `warning[E0102]`
  and `warning[E0103]`. Expected: `fires: 245 checks, E0102 53, E0103 0`:
  E0102 twice in `crates/nova-parser/tests/fixtures/sum_type.nova`, and 51
  times in `std/core`, `std/crypto`, `std/http`, `std/io`, `std/json` and
  `std/log`'s `lib.nova` checked as user files, each of which already fails
  at the base with E0001s; E0103 nowhere (decisions 22 and 24).
- [ ] **Step 6: fmt on the second check's probes.** For each `.nova` file
  of `$P/fc50a2/p/` and `$P/fc50a2/f/` that `nova parse` accepts, a copy in
  scratch, formatted with `target/debug/nova fmt`: it must format, `nova
  fmt --check` must then pass, and the kinds of its splits, read from `nova
  parse --ast`, must be the same before and after (decision 44). Expected:
  `fmtprobe: 60 files, 55 ok, 5 with a parse error, 0 bad`. The five are
  P0001s: `brk_bare.nova`, `brk_val.nova`, `rcomma.nova` and `rp.nova` as at
  the base, `rp.nova` since decision 46; and `arm.nova`, `0 => a` / `- 1,`,
  which parsed at the base and is now decision 47's P0001, with its note.
  Where the probes are missing, say so in the ledger.
- [ ] **Step 7: The lockfile.** `git diff --numstat 1fc7af6 -- Cargo.lock`.
  Expected: nothing.
- [ ] **Step 8: Rust 1.78.**

  ```sh
  RUSTFLAGS="-D warnings" CARGO_TARGET_DIR="$P/msrv-50a" cargo +1.78 check --locked --workspace
  rm -rf "$P/msrv-50a"
  ```

  Expected: clean, in 29 seconds. It checks the libraries and binaries, not
  the tests, as 4.6's to 4.8's did: `assert_cmd` 2.2.1's manifest does not
  parse on 1.78.
- [ ] **Step 9: The mutants.** One at a time, each edit is made, its
  command run, and the file restored with `git checkout -- <file>`.
  - Each edit replaces one exact text, which occurs once in its file; "the
    line goes" removes the whole line holding it.
  - A `cargo test` command rebuilds what it needs, `nova` among it for
    `run_tests`; no mutant touches `nova-runtime`.
  - The replay's run took about 4 minutes (249 s) in all; most take
    under a minute, the golden suite's about 25 seconds after its build.
  - Each caught mutant must fail as written: the tests named, or the text
    named in the command's output. A mutant that passes is a gap. The
    replay's last run left none: its first run's three gaps, mutants 4, 27
    and 39, are closed by the gap fix (decisions 33 to 35), and mutant 57's,
    found on the second check's split, by `77b6b2f` (decision 45). Any
    mutant that
    passes is a new gap: record it in the ledger, name it in the PR, and
    give it to the review (Step 11), which says whether a check is added.
  - At the end, `git status --short` prints nothing.

  1. **`(` and `[` after a line break continue the expression.**
     `crates/nova-parser/src/grammar.rs`:
     `Token::LParen | Token::LBracket if self.newline_ends_expr() => {`
     becomes `Token::LParen | Token::LBracket if false => {`.
     - Run `cargo test --locked -p nova-parser --test newline_rule`.
     - Expected: fails: `a_block_comment_that_holds_a_line_break_is_one`,
       `a_block_inside_parentheses_turns_the_rule_back_on`,
       `a_bracket_after_a_while_begins_a_statement`,
       `a_bracket_at_a_line_start_begins_a_statement`,
       `a_line_comment_is_a_line_break`,
       `a_match_inside_parentheses_turns_the_rule_back_on_in_its_arms`,
       `a_parenthesis_after_a_match_begins_a_statement`,
       `a_parenthesis_after_an_if_begins_a_statement`,
       `a_parenthesis_at_a_line_start_begins_a_statement`,
       `a_tuple_pattern_arm_after_an_arm_body_begins_an_arm`,
       `a_value_ending_an_arm_records_no_split_however_often_it_is_met`,
       `each_split_the_rule_makes_is_recorded`,
       `the_rule_holds_in_a_let_value_and_a_tail`,
       `the_rule_holds_in_a_match_arm`,
       `what_the_old_parser_could_not_have_continued_records_no_split`.
  2. **A `-` after a line break continues the expression.**
     `crates/nova-parser/src/grammar.rs`:
     `Token::Minus if self.newline_ends_expr() => {` becomes
     `Token::Minus if false => {`.
     - Run `cargo test --locked -p nova-parser --test newline_rule`.
     - Expected: fails: `a_minus_at_a_line_start_begins_a_statement`,
       `a_parse_error_at_an_arm_the_rule_began_names_the_rule`,
       `each_split_the_rule_makes_is_recorded`,
       `the_rule_holds_in_a_let_value_and_a_tail`.
  3. **A `return` or `break` at a line end takes the next line as its
     value.** `crates/nova-parser/src/grammar.rs`:
     replace

     ```
             if !self.line_break_here() {
                 return false;
             }
     ```

     with

     ```

     ```

     - Run `cargo test --locked -p nova-parser --test newline_rule`.
     - Expected: fails:
       `a_value_ending_an_arm_records_no_split_however_often_it_is_met`,
       `break_at_a_line_end_takes_no_value`,
       `each_split_the_rule_makes_is_recorded`,
       `return_at_a_line_end_takes_no_value`.
  4. **A match arm's body inherits the context instead of turning the rule
     on.** `crates/nova-parser/src/grammar.rs`:
     `let body = self.in_stmt_ctx(true, |p| p.parse_expr(ctx))?;` becomes
     `let body = self.parse_expr(ctx)?;`.
     - Run `cargo test --locked -p nova-parser --test newline_rule`.
     - Expected: fails:
       `a_match_inside_parentheses_turns_the_rule_back_on_in_its_arms`.
  5. **The rule stays on inside call arguments.**
     `crates/nova-parser/src/grammar.rs`:
     `let args = self.in_stmt_ctx(false, |p| {` becomes
     `let args = self.in_stmt_ctx(true, |p| {`.
     - Run `cargo test --locked -p nova-parser --test newline_rule`.
     - Expected: fails: `the_rule_is_off_inside_arguments`.
  6. **The rule stays on in the head of an if, while, for or match.**
     `crates/nova-parser/src/grammar.rs`:
     `self.in_no_struct_ctx(|p| p.in_stmt_ctx(false, f))` becomes
     `self.in_no_struct_ctx(f)`.
     - Run `cargo test --locked -p nova-parser --test newline_rule`.
     - Expected: fails: `the_rule_is_off_in_an_if_head`.
  7. **The rule stays on in a guard.** `crates/nova-parser/src/grammar.rs`:
     `Some(self.in_stmt_ctx(false, |p| p.parse_expr(ctx))?)` becomes
     `Some(self.parse_expr(ctx)?)`.
     - Run `cargo test --locked -p nova-parser --test newline_rule`.
     - Expected: fails: `the_rule_is_off_in_a_guard`.
  8. **The rule stays on inside parentheses.**
     `crates/nova-parser/src/grammar.rs`:
     `Token::LParen => self.in_stmt_ctx(false, |p| p.parse_paren(start, ctx)),`
     becomes `Token::LParen => self.parse_paren(start, ctx),`.
     - Run `cargo test --locked -p nova-parser --test newline_rule`.
     - Expected: fails: `the_rule_is_off_inside_parentheses`,
       `what_the_old_parser_could_not_have_continued_records_no_split`.
  9. **The rule stays on inside a record literal.**
     `crates/nova-parser/src/grammar.rs`:
     `self.in_stmt_ctx(false, |p| p.parse_record_literal_inner(path, start))`
     becomes `self.parse_record_literal_inner(path, start)`.
     - Run `cargo test --locked -p nova-parser --test newline_rule`.
     - Expected: fails: `the_rule_is_off_inside_a_record_literal`.
  10. **No token is marked.** `crates/nova-lexer/src/tokens.rs`:
      `gap.iter().any(|&b| b == b'\n' || b == b'\r')` becomes `false`.
      - Run `cargo test --locked -p nova-lexer --test lexer_tests`.
      - Expected: fails:
        `a_block_comment_marks_the_token_after_it_only_when_it_holds_a_line_break`,
        `a_line_comment_marks_the_token_after_it`,
        `a_lone_carriage_return_is_a_line_break`,
        `a_token_after_a_line_break_is_marked`,
        `the_first_token_is_marked_only_after_a_line_break`.
  11. **A token is marked by the one byte before it, not the whole gap.**
      `crates/nova-lexer/src/tokens.rs`:
      `let gap = bytes.get(prev_end.min(start)..start).unwrap_or(&[]);`
      becomes
      `let gap = bytes.get(start.saturating_sub(1)..start).unwrap_or(&[]);`.
      - Run `cargo test --locked -p nova-lexer --test lexer_tests`.
      - Expected: fails:
        `a_block_comment_marks_the_token_after_it_only_when_it_holds_a_line_break`,
        `a_token_after_a_line_break_is_marked`.
  12. **fmt keeps `(` in CONTINUES, printing `;` before a line that starts
      with it.** `crates/nova-fmt/src/print/mod.rs`:
      replace

      ```
      const CONTINUES: [Token; 5] = [
          Token::LBrace,
      ```

      with

      ```
      const CONTINUES: [Token; 6] = [
          Token::LParen,
          Token::LBrace,
      ```

      - Run `cargo test --locked -p nova-fmt --test source_rules`.
      - Expected: fails:
        `a_match_arm_starting_with_a_parenthesis_needs_no_comma`,
        `a_statement_starting_with_a_parenthesis_keeps_the_authors_semicolon`.
  13. **fmt keeps `-` in CONTINUES.** `crates/nova-fmt/src/print/mod.rs`:
      replace

      ```
      const CONTINUES: [Token; 5] = [
          Token::LBrace,
      ```

      with

      ```
      const CONTINUES: [Token; 6] = [
          Token::Minus,
          Token::LBrace,
      ```

      - Run `cargo test --locked -p nova-fmt --test source_rules`.
      - Expected: fails:
        `a_statement_starting_with_a_bracket_or_a_minus_keeps_the_authors_semicolon`,
        `an_unformatted_file_with_a_split_formats_keeping_it`.
  14. **fmt prints `;` before every `(`, `[` and `-` statement and after
      every bare `return`, written or not.**
      `crates/nova-fmt/src/print/expr.rs`:
      `((SPLITS.contains(first) || bare) && written).then_some(";")` becomes
      `(SPLITS.contains(first) || bare).then_some(";")`.
      - Run `cargo test --locked -p nova-fmt --test source_rules`.
      - Expected: fails:
        `a_statement_starting_with_a_bracket_or_a_minus_keeps_the_authors_semicolon`,
        `a_statement_starting_with_a_parenthesis_keeps_the_authors_semicolon`,
        `a_value_less_return_keeps_the_authors_semicolon`,
        `an_unformatted_file_with_a_split_formats_keeping_it`.
  15. **A view's children take the new CONTINUES.**
      `crates/nova-fmt/src/print/expr.rs`:
      `.filter(|n| VIEW_CONTINUES.contains(self.src.token_from(n.span.start)))`
      becomes
      `.filter(|n| CONTINUES.contains(self.src.token_from(n.span.start)))`.
      - Run `cargo test --locked -p nova-fmt --test source_rules`.
      - Expected: fails:
        `a_view_child_starting_with_a_parenthesis_keeps_its_semicolon`.
  16. **A chain with `-` breaks before each operator, as before.**
      `crates/nova-fmt/src/print/expr.rs`:
      `.any(|&(op, _)| matches!(op, Some(BinOp::Sub)));` becomes
      `.any(|_| false);`.
      - Run `cargo test --locked --no-fail-fast -p nova-fmt`.
      - Expected: fails:
        `a_broken_chain_with_a_minus_ends_each_line_with_its_operator`,
        `every_corpus_file_formats_to_the_same_program_and_comments`.
  17. **Every broken chain breaks after each operator.**
      `crates/nova-fmt/src/print/expr.rs`:
      `.any(|&(op, _)| matches!(op, Some(BinOp::Sub)));` becomes
      `.any(|_| true);`.
      - Run `cargo test --locked --no-fail-fast -p nova-fmt`.
      - Expected: fails:
        `a_broken_chain_without_a_minus_breaks_before_each_operator`,
        `a_long_condition_breaks_before_each_operator`,
        `an_end_of_line_comment_inside_an_expression_stays_where_it_was`.
  18. **`?`'s early return re-wraps the error in the good variant.**
      `crates/nova-typeck/src/check.rs`:
      replace

      ```
      variant: bad_idx,
                      args: bad_args,
      ```

      with

      ```
      variant: good_idx,
                      args: bad_args,
      ```

      - Run `R try_operator_runs try_async_runs`.
      - Expected: fails: `try_async_runs`, `try_operator_runs`.

        ```text
        thread 'main' panicked at crates/nova-runtime/src/lib.rs:258:13:
        null pointer dereference occurred
        ```
  19. **`?` takes any sum named Option or Result (E0098).**
      `crates/nova-typeck/src/check.rs`:
      `Ty::Sum { def_id, args } if Some(*def_id) == option || Some(*def_id) == result => {`
      becomes
      `Ty::Sum { def_id, args } if matches!(self.defs.def(*def_id).name.as_str(), "Option" | "Result") => {`.
      - Run `T check::lang_tests`.
      - Expected: fails:
        `check::lang_tests::try_on_the_programs_own_option_is_e0098`.
  20. **E0099's note leaves out `@test` functions.**
      `crates/nova-typeck/src/check.rs`:
      `entry: name == "main" || f.attrs.iter().any(|a| a.name.value == "test"),`
      becomes `entry: name == "main",`.
      - Run `T check::lang_tests`.
      - Expected: fails:
        `check::lang_tests::try_in_a_test_function_notes_that_it_returns_unit`.
  21. **E0099 never suggests `.ok_or(err)?`.**
      `crates/nova-typeck/src/check.rs`:
      `if is_option && returns_result {` becomes `if false {`.
      - Run `T check::lang_tests`.
      - Expected: fails:
        `check::lang_tests::try_on_option_in_a_result_function_suggests_ok_or`.
  22. **`?` lets the error types differ (no E0010).**
      `crates/nova-typeck/src/check.rs`:
      `if !fcx.icx.unify(&value_err, &ret_err) {` becomes `if false {`.
      - Run `T check::lang_tests`.
      - Expected: fails:
        `check::lang_tests::try_with_different_error_types_is_e0010_pointing_at_map_err`.
  23. **A closure's `return` and `?` check against the enclosing function's
      type.** `crates/nova-typeck/src/check.rs`:
      `let saved_ret_ty = std::mem::replace(&mut fcx.ret_ty, ret_ty.clone());`
      becomes `let saved_ret_ty = fcx.ret_ty.clone();`.
      - Run `T check::lang_tests`.
      - Expected: fails:
        `check::lang_tests::return_in_a_closure_checks_against_the_closures_type`,
        `check::lang_tests::try_in_a_closure_inside_a_constant_checks`,
        `check::lang_tests::try_in_a_closure_returns_from_the_closure_not_the_function`,
        `check::lang_tests::try_infers_a_closures_return_type`.
  24. **A closure inside `main` keeps `main`'s E0099 note.**
      `crates/nova-typeck/src/check.rs`:
      `let saved_entry = std::mem::replace(&mut fcx.entry, false);` becomes
      `let saved_entry = fcx.entry;`.
      - Run `T check::lang_tests`.
      - Expected: fails:
        `check::lang_tests::try_in_a_closure_inside_main_has_no_entry_note`.
  25. **`a[i] op= e` evaluates the index again for the old value.**
      `crates/nova-typeck/src/check.rs`:
      `index: Box::new(local_expr(*k, Ty::Int, index.span)),` becomes
      `index: Box::new(self.check_expr(fcx, index)),`.
      - Run `T check::lang_tests && R compound_assign_places_runs`.
      - Expected: fails:
        `check::lang_tests::compound_assignment_evaluates_the_index_once`.
  26. **`a[i] op= e` evaluates the array's receiver again for the old
      value.** `crates/nova-typeck/src/check.rs`:
      `target: Box::new(local_expr(*t, arr_ty.clone(), target.span)),`
      becomes `target: Box::new(self.check_expr(fcx, target)),`.
      - Run `R compound_assign_places_runs`.
      - Expected: fails: `compound_assign_places_runs`.

        ```text
        -receiver evaluated 1 time(s), cell 5
        +receiver evaluated 2 time(s), cell 5
        ```
  27. **`r.f op= e` evaluates the receiver again for the old value.**
      `crates/nova-typeck/src/check.rs`:
      `target: Box::new(local_expr(t, recv_ty.clone(), target.span)),`
      becomes `target: Box::new(self.check_expr(fcx, target)),`.
      - Run `T check::lang_tests && R compound_assign_places_runs`.
      - Expected: fails:
        `check::lang_tests::compound_assignment_on_a_field_evaluates_the_receiver_once`.
  28. **MIR lowers `String + String` as an integer add.**
      `crates/nova-mir/src/lower.rs`:
      `if matches!(lhs.ty, Ty::String) && matches!(op, hir::BinOp::Add) {`
      becomes `if false {`.
      - Run
        `cargo test --locked -p nova-codegen-llvm --test ir_tests -- string_plus && R string_concat_runs`.
      - Expected: fails: `string_plus_string_lowers_to_one_concat_call`.
  29. **The checker refuses `String + String` (E0013).**
      `crates/nova-typeck/src/check.rs`:
      `Add if matches!(operand, Ty::String) => Ty::String,`: the line goes.
      - Run `T check::lang_tests`.
      - Expected: fails:
        `check::lang_tests::string_compound_assignment_on_a_field_checks`,
        `check::lang_tests::string_plus_string_is_a_string`.
  30. **E0102 is never reported.** `crates/nova-resolver/src/lib.rs`:
      `diagnostics.extend(variant_shadowing(&definitions, &exports, &all, std_start));`:
      the line goes.
      - Run `cargo test --locked -p nova-resolver --lib`.
      - Expected: fails: `tests::a_qualified_use_of_stds_variant_is_fine`,
        `tests::a_variant_named_as_one_of_stds_is_e0102`,
        `tests::another_modules_variant_is_reported_too`,
        `tests::each_hidden_variant_is_reported`,
        `tests::importing_a_std_core_name_from_a_module_is_allowed`.
  31. **E0102 looks at the first module alone, not path dependencies.**
      `crates/nova-resolver/src/lib.rs`:
      `for m in &all[..std_start.min(all.len())] {` becomes
      `for m in &all[..1] {`.
      - Run `cargo test --locked -p nova-resolver --lib`.
      - Expected: fails: `tests::another_modules_variant_is_reported_too`,
        `tests::importing_a_std_core_name_from_a_module_is_allowed`.
  32. **E0103 is never reported.** `crates/nova-typeck/src/check.rs`:
      `if let Some(what) = unused_value(e) {` becomes
      `if let Some(what) = unused_value(e).filter(|_| false) {`.
      - Run `T check::lang_tests::e0103`.
      - Expected: fails:
        `check::lang_tests::e0103_flags_each_form_whose_value_is_discarded`,
        `check::lang_tests::e0103_reports_a_parenthesised_callee_after_a_line_break`.
  33. **E0103 flags `&&` and `||`.** `crates/nova-typeck/src/check.rs`:
      `E::Binary { op, .. } if !matches!(op, ast::BinOp::And | ast::BinOp::Or) => {`
      becomes `E::Binary { .. } => {`.
      - Run `T check::lang_tests::e0103`.
      - Expected: fails:
        `check::lang_tests::e0103_is_not_on_calls_assignments_conditions_or_a_tail`.
  34. **E0103 flags a call, field, index, `?`, await or cast whether or not
      it is parenthesised.** `crates/nova-typeck/src/check.rs`:
      `if paren(first) =>` becomes `if true =>`.
      - Run `T check::lang_tests::e0103`.
      - Expected: fails:
        `check::lang_tests::e0103_flags_each_form_whose_value_is_discarded`,
        `check::lang_tests::e0103_flags_what_the_newline_rule_leaves`,
        `check::lang_tests::e0103_is_not_on_calls_assignments_conditions_or_a_tail`,
        `check::lang_tests::e0103_reports_a_parenthesised_callee_after_a_line_break`.
  35. **`nova_test` left out of the exclusion table.**
      `crates/nova-cli/tests/run_tests.rs`:
      `` ("nova_test", "a `@test` file, which `nova test` runs"), ``: the
      line goes.
      - Run `G`.
      - Expected: fails: `release_golden_suite`.

        ```text
        release golden: 131 passed, 8 excluded, Ubuntu clang version 18.1.3 (1ubuntu1)
        1 of 132 release golden programs failed:
        nova_test: `nova build --release` failed:
        ```
  36. **The exclusion table names a program that does not exist.**
      `crates/nova-cli/tests/run_tests.rs`:
      `` ("nova_test", "a `@test` file, which `nova test` runs"), `` becomes
      `` ("nova_tests", "a `@test` file, which `nova test` runs"), ``.
      - Run `G`.
      - Expected: fails: `release_golden_suite`.

        ```text
        the exclusion table names `nova_tests`, which has no golden program
        ```
  37. **`io_streams` gets no standard input.**
      `crates/nova-cli/tests/run_tests.rs`:
      `const RELEASE_GOLDEN_STDIN: &[(&str, &str)] = &[("io_streams", "hello-stdin")];`
      becomes `const RELEASE_GOLDEN_STDIN: &[(&str, &str)] = &[];`.
      - Run `G`.
      - Expected: fails: `release_golden_suite`.

        ```text
        release golden: 130 passed, 9 excluded, Ubuntu clang version 18.1.3 (1ubuntu1)
        1 of 131 release golden programs failed:
        io_streams: standard output differs
        ```
  38. **A golden `.stdout` corrupted.** `tests/runtime/strings.stdout`:
      `is_empty empty=true x=false` becomes `is_empty empty=true x=true`.
      - Run `G`.
      - Expected: fails: `release_golden_suite`.

        ```text
        release golden: 130 passed, 9 excluded, Ubuntu clang version 18.1.3 (1ubuntu1)
        1 of 131 release golden programs failed:
        strings: standard output differs
        ```
  39. **The suite skips even under NOVA_REQUIRE_CLANG=1.**
      `crates/nova-cli/tests/run_tests.rs`:
      `if std::env::var("NOVA_REQUIRE_CLANG").as_deref() != Ok("1") {`
      becomes `if true {`.
      - Run `G`.
      - Expected: fails: `release_golden_suite`.

        ```text
        NOVA_REQUIRE_CLANG=1, but release_golden_suite was about to skip
        ```
  40. **The splits are recorded and never warned about.**
      `crates/nova-typeck/src/check.rs`:
      `checker.warn_newline_splits();`: the line goes.
      - Run `T check::lang_tests`.
      - Expected: fails:
        `check::lang_tests::a_split_after_a_bare_return_warns`,
        `check::lang_tests::a_split_after_a_return_value_warns`,
        `check::lang_tests::a_split_before_a_tail_warns`,
        `check::lang_tests::an_unused_value_at_a_split_gets_one_warning_with_the_line_breaks_note`,
        `check::lang_tests::e0103_reports_a_parenthesised_callee_after_a_line_break`.
  41. **A split before a `-` is not recorded.**
      `crates/nova-parser/src/grammar.rs`:
      `self.record_split(SplitKind::Minus);`: the line goes.
      - Run
        `cargo test --locked -p nova-parser --test newline_rule && T check::lang_tests`.
      - Expected: fails:
        `a_parse_error_at_an_arm_the_rule_began_names_the_rule`,
        `each_split_the_rule_makes_is_recorded`.
  42. **A split after a bare `return` or `break` is not recorded.**
      `crates/nova-parser/src/grammar.rs`:
      `self.record_split(SplitKind::Value);`: the line goes.
      - Run
        `cargo test --locked -p nova-parser --test newline_rule && T check::lang_tests`.
      - Expected: fails: `a_split_after_a_bare_return_is_recorded_once`,
        `each_split_the_rule_makes_is_recorded`.
  43. **The split that begins the next arm is kept.**
      `crates/nova-parser/src/grammar.rs`:
      `while self.splits.len() > splits && self.splits.last().map(|s| s.at.start) == Some(end) {`
      becomes `while false {`.
      - Run `cargo test --locked -p nova-parser --test newline_rule`.
      - Expected: fails: `a_bare_return_ending_an_arm_records_no_split`,
        `a_parse_error_at_an_arm_the_rule_began_names_the_rule`,
        `a_value_ending_an_arm_records_no_split_however_often_it_is_met`,
        `what_the_old_parser_could_not_have_continued_records_no_split`.
  44. **An unused value at a split gets a second warning.**
      `crates/nova-typeck/src/check.rs`:
      replace

      ```
                      ));
                      continue;
                  }
                  let (message, label) = match split.kind {
      ```

      with

      ```
                      ));
                  }
                  let (message, label) = match split.kind {
      ```

      - Run `T check::lang_tests`.
      - Expected: fails: `check::lang_tests::a_split_before_a_tail_warns`,
        `check::lang_tests::an_unused_value_at_a_split_gets_one_warning_with_the_line_breaks_note`,
        `check::lang_tests::e0103_flags_each_form_whose_value_is_discarded`,
        `check::lang_tests::e0103_flags_what_the_newline_rule_leaves`.
  45. **fmt drops the author's `;` before a split.**
      `crates/nova-fmt/src/print/expr.rs`:
      `((SPLITS.contains(first) || bare) && written).then_some(";")` becomes
      `None`.
      - Run `cargo test --locked -p nova-fmt --test source_rules`.
      - Expected: fails:
        `a_statement_starting_with_a_bracket_or_a_minus_keeps_the_authors_semicolon`,
        `a_statement_starting_with_a_parenthesis_keeps_the_authors_semicolon`,
        `a_value_less_return_keeps_the_authors_semicolon`.
  46. **`?` in a constant's initializer is accepted.**
      `crates/nova-typeck/src/check.rs`:
      `if fcx.in_const {` becomes `if false {`.
      - Run `T check::lang_tests`.
      - Expected: fails: `check::lang_tests::try_in_a_constant_is_e0099`.
  47. **E0099 says nothing of the function's own `Option`.**
      `crates/nova-typeck/src/check.rs`:
      `if matches!(self.defs.def(*def_id).name.as_str(), "Option" | "Result") {`
      becomes `if false {`.
      - Run `T check::lang_tests`.
      - Expected: fails:
        `check::lang_tests::try_in_a_function_returning_its_own_option_says_so`.
  48. **An interpolation hole inherits the rule.**
      `crates/nova-parser/src/grammar.rs`:
      `let result = self.in_stmt_ctx(false, f);` becomes
      `let result = f(self);`.
      - Run `cargo test --locked -p nova-parser --test newline_rule`.
      - Expected: fails: `the_rule_is_off_in_an_interpolation_hole`.
  49. **A nested item inherits the rule.** `crates/nova-parser/src/grammar.rs`:
      `let item = self.in_stmt_ctx(false, |p| p.try_parse_item())?;` becomes
      `let item = self.try_parse_item()?;`.
      - Run `cargo test --locked -p nova-parser --test newline_rule`.
      - Expected: fails:
        `the_rule_is_off_in_a_nested_items_signature_and_value`.
  50. **A lone `\r` is no line break.** `crates/nova-lexer/src/tokens.rs`:
      `gap.iter().any(|&b| b == b'\n' || b == b'\r')` becomes
      `gap.contains(&b'\n')`.
      - Run `cargo test --locked -p nova-lexer --test lexer_tests`.
      - Expected: fails: `a_lone_carriage_return_is_a_line_break`.
  51. **The walk finds two programs.** `crates/nova-cli/tests/run_tests.rs`:
      `.filter(|p| !RELEASE_GOLDEN_EXCLUDED.iter().any(|(n, _)| n == p))`
      becomes `.filter(|p| p.starts_with("try_"))`.
      - Run `G`.
      - Expected: fails: `release_golden_suite`.
  52. **A split before `(` is recorded as an index, and before `[` as a
      call.** `crates/nova-parser/src/grammar.rs`:
      replace

      ```
                              SplitKind::Call
                          } else {
                              SplitKind::Index
      ```

      with

      ```
                              SplitKind::Index
                          } else {
                              SplitKind::Call
      ```

      - Run `cargo test --locked -p nova-parser --test newline_rule`.
      - Expected: fails: `each_split_the_rule_makes_is_recorded`.
  53. **The postfix loop records no split.**
      `crates/nova-parser/src/grammar.rs`:
      `self.record_split(kind);`: the line goes.
      - Run
        `cargo test --locked -p nova-parser --test newline_rule && T check::lang_tests`.
      - Expected: fails: `each_split_the_rule_makes_is_recorded`.
  54. **Fmt's self-check compares the splits' places.**
      `crates/nova-fmt/src/check.rs`:
      `let splits: Vec<_> = std::mem::take(&mut file.newline_splits)`
      becomes `let splits: Vec<_> = file.newline_splits.clone()`.
      - Run
        `cargo test --locked -p nova-fmt --lib -- fingerprint && cargo test --locked -p nova-fmt --test source_rules`.
      - Expected: fails:
        `check::tests::the_fingerprint_tells_splits_by_kind_not_by_place`.
  55. **Fmt's self-check ignores the splits.** `crates/nova-fmt/src/check.rs`:
      replace

      ```
          format!("{}; splits {splits:?}", strip_spans(&format!("{file:?}")))
      ```

      with

      ```
          let _ = splits;
          strip_spans(&format!("{file:?}"))
      ```

      - Run
        `cargo test --locked -p nova-fmt --lib -- fingerprint && cargo test --locked -p nova-fmt --test source_rules`.
      - Expected: fails:
        `check::tests::the_fingerprint_tells_splits_by_kind_not_by_place`.
  56. **A bare `return` that a line break ends is an operand of what
      follows.** `crates/nova-parser/src/grammar.rs`:
      replace

      ```
              self.record_split(SplitKind::Value);
              self.halt = Some(self.pos);
      ```

      with

      ```
              self.record_split(SplitKind::Value);
      ```

      - Run `cargo test --locked -p nova-parser --test newline_rule`.
      - Expected: fails: `a_split_after_a_bare_return_is_recorded_once`.
  57. **An arm drops one split at its end token, not every one.**
      `crates/nova-parser/src/grammar.rs`:
      `while self.splits.len() > splits && self.splits.last().map(|s| s.at.start) == Some(end) {`
      becomes
      `if self.splits.len() > splits && self.splits.last().map(|s| s.at.start) == Some(end) {`.
      - Run `cargo test --locked -p nova-parser --test newline_rule`.
      - Expected: fails:
        `a_value_ending_an_arm_records_no_split_however_often_it_is_met`.
  58. **A bare `return` ends at a line break before any token.**
      `crates/nova-parser/src/grammar.rs`:
      replace

      ```
              if !can_start_expr(self.peek()) {
                  return false;
              }
      ```

      with

      ```

      ```

      - Run `cargo test --locked -p nova-parser --test newline_rule`.
      - Expected: fails:
        `a_bare_return_before_a_token_no_expression_starts_with_reads_as_before`.
  59. **A P0001 at an arm the rule began has no note.**
      `crates/nova-parser/src/grammar.rs`:
      `self.note_arm_split(prior);`: the line goes.
      - Run `cargo test --locked -p nova-parser --test newline_rule`.
      - Expected: fails:
        `a_parse_error_at_an_arm_the_rule_began_names_the_rule`.
  60. **The matrix's `require_clang` key renamed, which the test step
      reads.** `.github/workflows/ci.yml`:
      `require_clang: "1"` becomes `require-clang: "1"`.
      - Run `"$P/actionlint-dl/actionlint"`.
      - Expected: exits 1, its output holding
        `property "require_clang" is not defined`.
  61. **CI's step no longer asserts clang 18.1.3.** `.github/workflows/ci.yml`:
      `if ! clang-18 --version | head -n 1 | grep -q 'version 18\.1\.3'; then`
      becomes `if false; then`.
      - **Not run here:** the step runs on GitHub's ubuntu-24.04 runner
        alone; nothing here runs a workflow's steps, and actionlint accepts
        `if false`.
- [ ] **Step 10: The review's inputs.** The full run's figures, the new
  tests by binary, the golden suite's line and time, the AST and E0102/E0103
  counts, and the mutants' outcome, with each gap, go in the ledger.
- [ ] **Step 11: One fresh read-only review, then one fix pass** (the
  user's workflow).
  - The reviewer reads `git diff $BASE..HEAD` (Task 0) against the spec,
    ADR 0048 and this plan, and is told to hunt for:
    - a place where the rule splits a statement the old parser would have
      continued and no E0103 says so, or a warning where the old parser
      could not have continued; two splits at one token; a split surviving
      a written `;`; fmt adding or dropping a `;` that changes the recorded
      splits, or refusing a file whose layout only moves one; a "this value
      is not used" and a split's warning both at one place; a bare `return`
      or `break` before a token no expression begins with read otherwise
      than at the base; a P0001 at an arm the rule began without its note
      (decisions 9, 37 and 44 to 47);
    - `?` accepted in a constant's initializer, or refused in a closure
      inside one (decision 38);
    - a context where the newline rule is on and the spec turns it off, or
      off where it should be on; an expression the rule splits that the
      author wrote across lines; a `return` or `break` taking a value
      across a line break; a mark wrong after a comment, a string, an
      interpolation or `\r\n` (decisions 3 and 10 to 14);
    - fmt printing a layout that parses to another tree, or dropping a `;`
      a view still needs; a chain with `-` breaking before an operator
      (decisions 9, 15 and 16);
    - `?` on a type it should refuse, or refusing one it should take; the
      early return built with the wrong variant or type; a closure's
      return type left unconstrained; a note on the wrong case (decisions
      1, 6 and 17 to 19);
    - a closure's `return` now refused where it was meant to work, or
      `fcx.ret_ty` or `fcx.entry` not restored on every path (decision
      19);
    - `op=` evaluating a receiver or an index twice, or in the wrong
      order; E0060 missed for an immutable root (decision 20);
    - `String + String` reaching the integer add in any backend; WASM's
      concatenation differing from native's (decision 7);
    - E0102 on std's own variants, on a registry package, or missing a
      path dependency's; E0103 on a tail, a call, an assignment, `&&` or
      `||` (decisions 22 to 24);
    - the golden suite: a program the walk should include and leaves out,
      an exclusion without its reason, a race on a fixture, a binary left
      running, a failure that does not fail the test; CI: the step's
      version check, the matrix keys, the variables on other legs, and
      the skip branch's assertion, the walk's floor (decisions 25, 26 and
      35; mutant 61);
    - the gap fix's three tests and the golden suite's assertion, each
      against the mutant it answers (decisions 33 to 36);
    - a record that says what the code does not: ADR 0048, the CHANGELOG,
      the dated notes, the catalog's titles;
    - each mutant of Step 9 that passes, and whether a check is added;
    - MSRV 1.78, a new crate, a change to `Cargo.lock`, a test that
      touches the internet.
  - Each finding is fixed in one commit, `5.0a: the review's fixes`, or
    answered in the ledger with the reason it stands.
  - After the fixes, Steps 1 and 2 run again.
- [ ] **Step 12: The PR.**
  - Push: `git push -u origin claude/loving-archimedes-frtf5q-50`.
  - Open the PR to `main` with the repository's template headings, if it
    has one. The body names no local path and no person.
  - It holds:
    - what 5.0a adds: `?`, `+=` on fields and elements, the newline rule
      with fmt following it, `String + String`, E0102 and E0103, the
      golden suite through LLVM on `ubuntu-24.04` with `clang-18` 18.1.3,
      and ADR 0048;
    - the one way a program changes meaning: a line that starts with `(`,
      `[` or `-` and meant to continue the line before now begins a
      statement, which E0103 reports; every repository `.nova` file parses
      to the same tree;
    - the figures: the suite's `2529 1 10`, the golden suite's `131 passed,
      9 excluded`, and that `Cargo.lock` is unchanged;
    - the user's decisions, and this plan's decisions, numbered as here,
      with 9, 12, 13, 15, 19, 20, 21 and 25 marked as differing from the spec,
      and the user's two rulings on the fact-check (decisions 37 and 38);
    - the mutants, the first run's three gaps and the tests that closed
      them, the fact-check's eight findings and the second check's, and
      what came of each, and the review's findings and what came of each.
  - Read CI once it ends. Expected: green on all three systems. On the
    `test` job's `ubuntu-24.04` leg, the step "Install clang 18.1.3
    (Linux)" prints `Ubuntu clang version 18.1.3`, and `cargo test`'s log
    holds `test release_golden_suite ... ok`, taking about 20 seconds; the
    test's own `release golden:` line is captured, as cargo captures a
    passing test's output, and a skip there would fail it (decision 35).
    On macOS and Windows the step is skipped and the test passes at once.
    Record the run's address in the ledger.
  - **If the version step fails,** Ubuntu's archive has moved `clang-18`
    off 18.1.3: stop and ask the user; the pin moves by a new ADR (ADR
    0048), never by editing the step. **If the suite fails on CI and not
    here,** record its failures and stop and ask the user.
- [ ] **Step 13: The merge,** only on the user's word.
  - By rebase, pinned to the PR's head commit. Never auto-merge.
  - Then `git fetch origin main` and `git diff <head> origin/main
    --stat`, which prints nothing: the merged tree is the PR's.
  - Then read CI's run on `main` once it ends: green, as Step 12's, its
    Linux leg's log holding `test release_golden_suite ... ok`.
